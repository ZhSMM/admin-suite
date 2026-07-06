//! Anthropic Messages API adapter.
//!
//! Wire format differs from OpenAI: `x-api-key` header, `anthropic-version`
//! header, body uses `max_tokens` (required), `messages` (no system role
//! inline — system goes in a top-level `system` field), and SSE events
//! use `message_start` / `content_block_delta` / `message_stop`.

use async_trait::async_trait;
use futures::StreamExt;
use serde::Deserialize;
use serde_json::json;

use crate::llm::{
    ChatRequest, ChatResponse, LlmError, LlmProvider, ProviderContext, ProviderModelInfo,
    StreamChunk,
};

pub struct Anthropic;

const API_VERSION: &str = "2023-06-01";

#[async_trait]
impl LlmProvider for Anthropic {
    fn kind(&self) -> &'static str { "anthropic" }

    fn supports_list_models(&self) -> bool { true }

    /// Hits Anthropic's `{base}/v1/models` with `anthropic-version` header.
    /// Default page limit (`?limit=100`) is plenty for the niche list.
    async fn list_models(
        &self,
        ctx: &ProviderContext,
    ) -> Result<Vec<ProviderModelInfo>, LlmError> {
        let url = format!(
            "{}/v1/models?limit=100",
            ctx.base_url.trim_end_matches('/')
        );
        // Mirror the same auth convention as `send()` below: x-api-key +
        // anthropic-version. A bearer header works too but x-api-key is
        // what Anthropic documents; we keep behaviour consistent.
        let mut req = ctx
            .http
            .get(&url)
            .header("anthropic-version", API_VERSION);
        if let Some(k) = ctx.api_key.as_deref() {
            if !k.is_empty() {
                // Honour custom header name from provider settings, but
                // default to x-api-key — bearer-style for Anthropic still
                // needs the x-api-key header, per their docs.
                let h = ctx.auth_header.as_deref().unwrap_or("x-api-key");
                if h.eq_ignore_ascii_case("authorization") {
                    req = req.header(h, format!("Bearer {k}"));
                } else {
                    req = req.header(h, k);
                }
            }
        }
        let resp = req
            .send()
            .await
            .map_err(|e| LlmError::Network(e.to_string()))?;
        let status = resp.status();
        if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
            return Err(LlmError::Unauthorized);
        }
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(LlmError::BadStatus {
                status: status.as_u16(),
                body,
            });
        }
        #[derive(Deserialize)]
        struct Resp { data: Vec<Entry> }
        #[derive(Deserialize)]
        struct Entry {
            id: String,
            #[serde(default)]
            display_name: Option<String>,
            #[serde(default)]
            r#type: Option<String>,
        }
        let parsed: Resp = resp
            .json()
            .await
            .map_err(|e| LlmError::Decode(e.to_string()))?;
        Ok(parsed
            .data
            .into_iter()
            .map(|e| ProviderModelInfo {
                id: e.id.clone(),
                display_name: e.display_name.or(Some(e.id.clone())),
                context_window: None,
                kind: Some(e.r#type.unwrap_or_else(|| "chat".into())),
            })
            .collect())
    }

    async fn chat(
        &self,
        ctx: &ProviderContext,
        req: ChatRequest,
    ) -> Result<ChatResponse, LlmError> {
        let url = format!("{}/v1/messages", ctx.base_url.trim_end_matches('/'));
        let body = build_body(&req, false);
        let resp = send(ctx, &url, &body).await?;
        let parsed: MessageResponse = resp.json().await.map_err(|e| LlmError::Decode(e.to_string()))?;
        let content = parsed
            .content
            .into_iter()
            .filter_map(|b| match b.r#type.as_deref() {
                Some("text") => Some(b.text.unwrap_or_default()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("");
        Ok(ChatResponse {
            content,
            prompt_tokens: parsed.usage.as_ref().map(|u| u.input_tokens).unwrap_or(0),
            completion_tokens: parsed.usage.as_ref().map(|u| u.output_tokens).unwrap_or(0),
            total_tokens: parsed
                .usage
                .as_ref()
                .map(|u| u.input_tokens + u.output_tokens)
                .unwrap_or(0),
            model: parsed.model,
            finish_reason: parsed.stop_reason,
        })
    }

    async fn chat_stream(
        &self,
        ctx: &ProviderContext,
        req: ChatRequest,
        on_chunk: Box<dyn Fn(StreamChunk) + Send + Sync>,
    ) -> Result<ChatResponse, LlmError> {
        let url = format!("{}/v1/messages", ctx.base_url.trim_end_matches('/'));
        let body = build_body(&req, true);
        let resp = send(ctx, &url, &body).await?;
        let mut stream = resp.bytes_stream();
        let mut full = String::new();
        let mut prompt_tokens = 0u32;
        let mut completion_tokens = 0u32;
        let mut model: Option<String> = None;
        let mut finish_reason: Option<String> = None;
        let mut leftover = String::new();

        while let Some(chunk) = stream.next().await {
            let bytes = chunk.map_err(|e| LlmError::Network(e.to_string()))?;
            leftover.push_str(&String::from_utf8_lossy(&bytes));
            while let Some(idx) = leftover.find("\n\n") {
                let frame: String = leftover.drain(..idx + 2).collect();
                let mut event = "";
                let mut data = "";
                for line in frame.lines() {
                    if let Some(rest) = line.strip_prefix("event:") {
                        event = rest.trim();
                    } else if let Some(rest) = line.strip_prefix("data:") {
                        data = rest.trim();
                    }
                }
                if data.is_empty() || data == "[DONE]" {
                    continue;
                }
                if event == "content_block_delta" {
                    if let Ok(v) = serde_json::from_str::<ContentBlockDelta>(data) {
                        if let Some(delta) = v.delta.and_then(|d| d.text) {
                            full.push_str(&delta);
                            on_chunk(StreamChunk { delta, finish_reason: None });
                        }
                    }
                } else if event == "message_start" {
                    if let Ok(v) = serde_json::from_str::<MessageStart>(data) {
                        model = v.message.and_then(|m| m.model);
                    }
                } else if event == "message_delta" {
                    if let Ok(v) = serde_json::from_str::<MessageDelta>(data) {
                        if let Some(u) = v.usage {
                            prompt_tokens = u.input_tokens;
                            completion_tokens = u.output_tokens;
                        }
                        finish_reason = v.delta.and_then(|d| d.stop_reason);
                    }
                }
            }
        }

        Ok(ChatResponse {
            content: full,
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens + completion_tokens,
            model,
            finish_reason,
        })
    }
}

fn build_body(req: &ChatRequest, stream: bool) -> serde_json::Value {
    let messages: Vec<serde_json::Value> = req
        .messages
        .iter()
        .filter(|m| m.role != "system")
        .map(|m| json!({"role": m.role, "content": m.content}))
        .collect();
    let system = req
        .system
        .clone()
        .or_else(|| {
            req.messages
                .iter()
                .find(|m| m.role == "system")
                .map(|m| m.content.clone())
        });
    let mut body = json!({
        "model": req.model,
        "messages": messages,
        "max_tokens": req.max_tokens.unwrap_or(4096),
        "stream": stream,
    });
    if let Some(sys) = system {
        if !sys.is_empty() {
            body["system"] = json!(sys);
        }
    }
    if let Some(t) = req.temperature { body["temperature"] = json!(t); }
    if let Some(p) = req.top_p       { body["top_p"]       = json!(p); }
    if let Some(s) = &req.stop {
        if !s.is_empty() { body["stop_sequences"] = json!(s); }
    }
    body
}

fn auth_header(ctx: &ProviderContext) -> Option<String> {
    match ctx.auth_type.as_str() {
        "none" => None,
        "bearer" => ctx.api_key.as_ref().map(|k| format!("Bearer {k}")),
        // anthropic uses 'x-api-key' — auth_header is the raw key
        "header" => ctx.api_key.clone(),
        _ => ctx.api_key.clone(),
    }
}

async fn send(
    ctx: &ProviderContext,
    url: &str,
    body: &serde_json::Value,
) -> Result<reqwest::Response, LlmError> {
    let key = ctx.api_key.as_deref().unwrap_or("");
    let resp = ctx
        .http
        .post(url)
        .header("x-api-key", key)
        .header("anthropic-version", API_VERSION)
        .json(body)
        .send()
        .await
        .map_err(|e| LlmError::Network(e.to_string()))?;
    let status = resp.status();
    if status == reqwest::StatusCode::UNAUTHORIZED {
        return Err(LlmError::Unauthorized);
    }
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        let retry = resp
            .headers()
            .get("retry-after")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(1000)
            * 1000;
        return Err(LlmError::RateLimited { retry_after_ms: retry });
    }
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(LlmError::BadStatus { status: status.as_u16(), body });
    }
    Ok(resp)
}

#[derive(Debug, Deserialize)]
struct MessageResponse {
    #[serde(default)]
    content: Vec<ContentBlock>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    stop_reason: Option<String>,
    #[serde(default)]
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    r#type: Option<String>,
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Usage {
    input_tokens: u32,
    output_tokens: u32,
}

#[derive(Debug, Deserialize)]
struct ContentBlockDelta {
    delta: Option<TextDelta>,
}

#[derive(Debug, Deserialize)]
struct TextDelta {
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MessageStart {
    message: Option<MessageStartBody>,
}

#[derive(Debug, Deserialize)]
struct MessageStartBody {
    model: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MessageDelta {
    delta: Option<DeltaBody>,
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct DeltaBody {
    stop_reason: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::ChatMessage;

    #[test]
    fn build_body_extracts_system_prompt() {
        let req = ChatRequest {
            model: "claude-3-5-sonnet".into(),
            messages: vec![
                ChatMessage { role: "system".into(), content: "be brief".into() },
                ChatMessage { role: "user".into(), content: "hi".into() },
            ],
            temperature: None,
            max_tokens: Some(1024),
            top_p: None,
            stop: None,
            system: None,
        };
        let body = build_body(&req, false);
        assert_eq!(body["system"], "be brief");
        assert_eq!(body["messages"].as_array().unwrap().len(), 1);
        assert_eq!(body["messages"][0]["role"], "user");
        assert_eq!(body["max_tokens"], 1024);
    }

    #[test]
    fn build_body_explicit_system_wins() {
        let req = ChatRequest {
            model: "x".into(),
            messages: vec![ChatMessage { role: "system".into(), content: "in-message".into() }],
            temperature: None,
            max_tokens: None,
            top_p: None,
            stop: None,
            system: Some("explicit".into()),
        };
        let body = build_body(&req, false);
        assert_eq!(body["system"], "explicit");
    }
}