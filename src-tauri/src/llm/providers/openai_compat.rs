//! OpenAI-Compatible adapter.
//!
//! Covers OpenAI, DeepSeek, Mistral, Qwen DashScope, Ollama's /v1,
//! LM Studio, vLLM, llama.cpp server, OpenRouter, Zhipu, Moonshot,
//! and most LLM servers that expose the chat-completions schema.
//!
//! SSE protocol: lines starting with `data: ` (with a single space), each
//! a JSON object with `choices[0].delta.content`. The stream ends with a
//! `data: [DONE]` sentinel.

use async_trait::async_trait;
use futures::StreamExt;
use serde::Deserialize;

use crate::llm::{
    ChatMessage, ChatRequest, ChatResponse, LlmError, LlmProvider, ProviderContext,
    ProviderModelInfo, StreamChunk,
};

pub struct OpenAiCompat;

#[async_trait]
impl LlmProvider for OpenAiCompat {
    fn kind(&self) -> &'static str { "openai_compat" }

    fn supports_list_models(&self) -> bool { true }

    /// Hits `{base}/models` (OpenAI / Ollama / vLLM / LM Studio / DeepSeek /
    /// Moonshot / OpenRouter / Zhipu / DashScope OpenAI mode / ...).
    async fn list_models(
        &self,
        ctx: &ProviderContext,
    ) -> Result<Vec<ProviderModelInfo>, LlmError> {
        let url = format!("{}/models", ctx.base_url.trim_end_matches('/'));
        let mut req = ctx.http.get(&url);
        if let Some(hv) = auth_header(ctx) {
            let header = ctx.auth_header.as_deref().unwrap_or("authorization");
            req = req.header(header, hv);
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
            #[serde(rename = "owned_by")]
            owned_by: Option<String>,
            #[serde(default)]
            object: Option<String>,
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
                display_name: Some(e.id.clone()),
                context_window: None,
                kind: match e.object.as_deref() {
                    Some("embedding") => Some("embedding".into()),
                    _ => Some("chat".into()),
                },
            })
            .collect())
    }

    async fn chat(
        &self,
        ctx: &ProviderContext,
        req: ChatRequest,
    ) -> Result<ChatResponse, LlmError> {
        let url = format!("{}/chat/completions", ctx.base_url.trim_end_matches('/'));
        let body = build_body(&req, false);
        let resp = send(ctx, &url, &body).await?;
        let parsed: ChatCompletion = resp
            .json()
            .await
            .map_err(|e| LlmError::Decode(e.to_string()))?;
        let choice = parsed.choices.first();
        let content = choice
            .and_then(|c| c.message.as_ref())
            .map(|m| m.content.clone())
            .unwrap_or_default();
        Ok(ChatResponse {
            content,
            prompt_tokens: parsed.usage.as_ref().map(|u| u.prompt_tokens).unwrap_or(0),
            completion_tokens: parsed.usage.as_ref().map(|u| u.completion_tokens).unwrap_or(0),
            total_tokens: parsed.usage.as_ref().map(|u| u.total_tokens).unwrap_or(0),
            model: parsed.model,
            finish_reason: choice.and_then(|c| c.finish_reason.clone()),
        })
    }

    async fn chat_stream(
        &self,
        ctx: &ProviderContext,
        req: ChatRequest,
        on_chunk: Box<dyn Fn(StreamChunk) + Send + Sync>,
    ) -> Result<ChatResponse, LlmError> {
        let url = format!("{}/chat/completions", ctx.base_url.trim_end_matches('/'));
        let body = build_body(&req, true);
        let resp = send(ctx, &url, &body).await?;
        let mut stream = resp.bytes_stream();
        let mut full = String::new();
        let mut prompt_tokens = 0u32;
        let mut completion_tokens = 0u32;
        let mut total_tokens = 0u32;
        let mut model: Option<String> = None;
        let mut finish_reason: Option<String> = None;
        let mut leftover = String::new();

        while let Some(chunk) = stream.next().await {
            let bytes = chunk.map_err(|e| LlmError::Network(e.to_string()))?;
            leftover.push_str(&String::from_utf8_lossy(&bytes));
            // SSE frames are separated by \n\n
            while let Some(idx) = leftover.find("\n\n") {
                let frame: String = leftover.drain(..idx + 2).collect();
                for line in frame.lines() {
                    let line = line.trim();
                    if !line.starts_with("data:") {
                        continue;
                    }
                    let payload = line[5..].trim();
                    if payload.is_empty() || payload == "[DONE]" {
                        continue;
                    }
                    let parsed: ChatCompletionChunk = match serde_json::from_str(payload) {
                        Ok(v) => v,
                        Err(_) => continue, // partial chunk; skip
                    };
                    if model.is_none() {
                        model = parsed.model;
                    }
                    if let Some(choice) = parsed.choices.first() {
                        if let Some(delta) = choice.delta.as_ref().and_then(|d| d.content.as_ref()) {
                            full.push_str(delta);
                            on_chunk(StreamChunk {
                                delta: delta.clone(),
                                finish_reason: choice.finish_reason.clone(),
                            });
                        }
                        if choice.finish_reason.is_some() {
                            finish_reason = choice.finish_reason.clone();
                        }
                    }
                    if let Some(u) = parsed.usage {
                        prompt_tokens = u.prompt_tokens;
                        completion_tokens = u.completion_tokens;
                        total_tokens = u.total_tokens;
                    }
                }
            }
        }

        Ok(ChatResponse {
            content: full,
            prompt_tokens,
            completion_tokens,
            total_tokens,
            model,
            finish_reason,
        })
    }
}

fn build_body(req: &ChatRequest, stream: bool) -> serde_json::Value {
    // Merge system prompt with the messages array (some providers want it
    // as a separate field, some only via the messages list — sending it as
    // both is the safest).
    let mut messages: Vec<serde_json::Value> = Vec::new();
    if let Some(sys) = &req.system {
        if !sys.is_empty() {
            messages.push(serde_json::json!({"role":"system","content":sys}));
        }
    }
    for m in &req.messages {
        messages.push(serde_json::json!({"role":m.role,"content":m.content}));
    }
    let mut body = serde_json::json!({
        "model": req.model,
        "messages": messages,
        "stream": stream,
    });
    if let Some(t) = req.temperature { body["temperature"] = serde_json::json!(t); }
    if let Some(m) = req.max_tokens  { body["max_tokens"]  = serde_json::json!(m); }
    if let Some(p) = req.top_p       { body["top_p"]       = serde_json::json!(p); }
    if let Some(s) = &req.stop {
        if !s.is_empty() {
            body["stop"] = serde_json::json!(s);
        }
    }
    body
}

fn auth_header(ctx: &ProviderContext) -> Option<String> {
    match ctx.auth_type.as_str() {
        "none" => None,
        "bearer" => ctx.api_key.as_ref().map(|k| format!("Bearer {k}")),
        "header" => ctx.api_key.clone(),
        _ => ctx.api_key.as_ref().map(|k| format!("Bearer {k}")),
    }
}

async fn send(
    ctx: &ProviderContext,
    url: &str,
    body: &serde_json::Value,
) -> Result<reqwest::Response, LlmError> {
    let mut req = ctx.http.post(url).json(body);
    if let Some(hv) = auth_header(ctx) {
        let header = ctx.auth_header.as_deref().unwrap_or("authorization");
        req = req.header(header, hv);
    }
    let resp = req.send().await.map_err(|e| LlmError::Network(e.to_string()))?;
    let status = resp.status();
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
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
        if body.contains("context_length_exceeded") || body.contains("maximum context length") {
            return Err(LlmError::ContextOverflow { prompt: 0, max: 0 });
        }
        return Err(LlmError::BadStatus {
            status: status.as_u16(),
            body,
        });
    }
    Ok(resp)
}

#[derive(Debug, Deserialize)]
struct ChatCompletion {
    choices: Vec<ChatCompletionChoice>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionChoice {
    #[serde(default)]
    message: Option<ChatMessageRemote>,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ChatMessageRemote {
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionChunk {
    choices: Vec<ChatCompletionChunkChoice>,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct ChatCompletionChunkChoice {
    #[serde(default)]
    delta: Option<Delta>,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Delta {
    #[serde(default)]
    content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Usage {
    prompt_tokens: u32,
    completion_tokens: u32,
    total_tokens: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_body_merges_system_prompt() {
        let req = ChatRequest {
            model: "x".into(),
            messages: vec![ChatMessage { role: "user".into(), content: "hi".into() }],
            temperature: Some(0.5),
            max_tokens: Some(100),
            top_p: None,
            stop: Some(vec!["END".into()]),
            system: Some("be helpful".into()),
        };
        let body = build_body(&req, false);
        assert_eq!(body["model"], "x");
        assert_eq!(body["messages"].as_array().unwrap().len(), 2);
        assert_eq!(body["messages"][0]["role"], "system");
        assert_eq!(body["messages"][0]["content"], "be helpful");
        assert_eq!(body["temperature"], 0.5);
        assert_eq!(body["max_tokens"], 100);
        assert_eq!(body["stop"][0], "END");
    }

    #[test]
    fn build_body_omits_optional_fields() {
        let req = ChatRequest {
            model: "x".into(),
            messages: vec![],
            temperature: None,
            max_tokens: None,
            top_p: None,
            stop: None,
            system: None,
        };
        let body = build_body(&req, true);
        assert_eq!(body["stream"], true);
        assert!(body.get("temperature").is_none());
        assert!(body.get("stop").is_none());
    }

    #[test]
    fn auth_header_bearer_default() {
        let ctx = ProviderContext {
            provider_id: "p".into(),
            base_url: "https://x".into(),
            auth_type: "bearer".into(),
            auth_header: None,
            api_key: Some("sk-abc".into()),
            timeout_ms: 30_000,
            http: reqwest::Client::new(),
        };
        assert_eq!(auth_header(&ctx), Some("Bearer sk-abc".into()));
    }

    #[test]
    fn auth_header_custom() {
        let ctx = ProviderContext {
            provider_id: "p".into(),
            base_url: "https://x".into(),
            auth_type: "header".into(),
            auth_header: Some("x-api-key".into()),
            api_key: Some("xyz".into()),
            timeout_ms: 30_000,
            http: reqwest::Client::new(),
        };
        // 'header' auth_type passes the raw key through (custom name picks up)
        assert_eq!(auth_header(&ctx), Some("xyz".into()));
    }

    #[test]
    fn auth_header_none() {
        let ctx = ProviderContext {
            provider_id: "p".into(),
            base_url: "https://x".into(),
            auth_type: "none".into(),
            auth_header: None,
            api_key: Some("ignored".into()),
            timeout_ms: 30_000,
            http: reqwest::Client::new(),
        };
        assert_eq!(auth_header(&ctx), None);
    }
}