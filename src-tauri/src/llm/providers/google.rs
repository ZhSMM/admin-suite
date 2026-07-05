//! Google Gemini adapter (v1beta API).
//!
//! Wire format:
//!   POST `{base_url}/v1beta/models/{model}:generateContent`
//!   GET  `{base_url}/v1beta/models/{model}:streamGenerateContent?alt=sse`
//!   Header: `x-goog-api-key: <key>` (or `?key=` query param — we use header).
//!
//! Body:
//!   { "contents": [{ "role": "user"|"model", "parts": [{"text": "..."}] }],
//!     "systemInstruction": {"parts": [{"text": "..."}]},
//!     "generationConfig": { "temperature": ..., "maxOutputTokens": ..., "topP": ..., "stopSequences": [...] } }
//!
//! Response (non-stream):
//!   { "candidates": [{ "content": {"parts": [{"text": "..."}] } }],
//!     "usageMetadata": { "promptTokenCount": ..., "candidatesTokenCount": ..., "totalTokenCount": ... } }
//!
//! Stream chunks have the same shape, each with one or more candidates.

use async_trait::async_trait;
use futures::StreamExt;
use serde::Deserialize;

use crate::llm::{
    ChatRequest, ChatResponse, LlmError, LlmProvider, ProviderContext, StreamChunk,
};

pub struct Google;

#[async_trait]
impl LlmProvider for Google {
    fn kind(&self) -> &'static str { "google" }

    async fn chat(
        &self,
        ctx: &ProviderContext,
        req: ChatRequest,
    ) -> Result<ChatResponse, LlmError> {
        let url = format!(
            "{}/v1beta/models/{}:generateContent",
            ctx.base_url.trim_end_matches('/'),
            req.model,
        );
        let body = build_body(&req);
        let resp = send(ctx, &url, &body).await?;
        let parsed: GenerateResponse = resp
            .json()
            .await
            .map_err(|e| LlmError::Decode(e.to_string()))?;
        let text = collect_text(&parsed);
        Ok(ChatResponse {
            content: text,
            prompt_tokens: parsed
                .usage_metadata
                .as_ref()
                .map(|u| u.prompt_token_count.unwrap_or(0))
                .unwrap_or(0),
            completion_tokens: parsed
                .usage_metadata
                .as_ref()
                .map(|u| u.candidates_token_count.unwrap_or(0))
                .unwrap_or(0),
            total_tokens: parsed
                .usage_metadata
                .as_ref()
                .map(|u| u.total_token_count.unwrap_or(0))
                .unwrap_or(0),
            model: Some(req.model.clone()),
            finish_reason: None,
        })
    }

    async fn chat_stream(
        &self,
        ctx: &ProviderContext,
        req: ChatRequest,
        on_chunk: Box<dyn Fn(StreamChunk) + Send + Sync>,
    ) -> Result<ChatResponse, LlmError> {
        let url = format!(
            "{}/v1beta/models/{}:streamGenerateContent?alt=sse",
            ctx.base_url.trim_end_matches('/'),
            req.model,
        );
        let body = build_body(&req);
        let resp = send(ctx, &url, &body).await?;
        let mut stream = resp.bytes_stream();
        let mut full = String::new();
        let mut prompt_tokens = 0u32;
        let mut completion_tokens = 0u32;
        let mut total_tokens = 0u32;
        let mut leftover = String::new();
        while let Some(chunk) = stream.next().await {
            let bytes = chunk.map_err(|e| LlmError::Network(e.to_string()))?;
            leftover.push_str(&String::from_utf8_lossy(&bytes));
            while let Some(idx) = leftover.find("\n\n") {
                let frame: String = leftover.drain(..idx + 2).collect();
                for line in frame.lines() {
                    let line = line.trim();
                    if !line.starts_with("data:") {
                        continue;
                    }
                    let payload = &line[5..].trim();
                    if payload.is_empty() {
                        continue;
                    }
                    if let Ok(parsed) = serde_json::from_str::<GenerateResponse>(payload) {
                        let text = collect_text(&parsed);
                        if !text.is_empty() {
                            full.push_str(&text);
                            on_chunk(StreamChunk { delta: text, finish_reason: None });
                        }
                        if let Some(u) = &parsed.usage_metadata {
                            prompt_tokens = u.prompt_token_count.unwrap_or(prompt_tokens);
                            completion_tokens =
                                u.candidates_token_count.unwrap_or(completion_tokens);
                            total_tokens = u.total_token_count.unwrap_or(total_tokens);
                        }
                    }
                }
            }
        }
        Ok(ChatResponse {
            content: full,
            prompt_tokens,
            completion_tokens,
            total_tokens,
            model: Some(req.model.clone()),
            finish_reason: None,
        })
    }
}

fn collect_text(parsed: &GenerateResponse) -> String {
    let mut out = String::new();
    if let Some(cands) = &parsed.candidates {
        for c in cands {
            if let Some(content) = &c.content {
                for part in &content.parts {
                    if let Some(text) = &part.text {
                        out.push_str(text);
                    }
                }
            }
        }
    }
    out
}

fn build_body(req: &ChatRequest) -> serde_json::Value {
    let mut contents: Vec<serde_json::Value> = Vec::new();
    for m in &req.messages {
        let role = match m.role.as_str() {
            "system" => continue, // Gemini uses systemInstruction, handled below
            "assistant" => "model",
            _ => "user",
        };
        contents.push(serde_json::json!({
            "role": role,
            "parts": [{"text": m.content}],
        }));
    }
    let mut body = serde_json::json!({ "contents": contents });
    // Prefer the explicit `system` field; fall back to a system-role message.
    let system_text = req
        .system
        .clone()
        .filter(|s| !s.is_empty())
        .or_else(|| {
            req.messages
                .iter()
                .find(|m| m.role == "system")
                .map(|m| m.content.clone())
        });
    if let Some(sys) = system_text {
        body["systemInstruction"] = serde_json::json!({
            "parts": [{"text": sys}],
        });
    }
    let mut gc = serde_json::json!({});
    if let Some(t) = req.temperature { gc["temperature"] = serde_json::json!(t); }
    if let Some(m) = req.max_tokens  { gc["maxOutputTokens"] = serde_json::json!(m); }
    if let Some(p) = req.top_p       { gc["topP"] = serde_json::json!(p); }
    if let Some(s) = &req.stop {
        if !s.is_empty() { gc["stopSequences"] = serde_json::json!(s); }
    }
    if gc.as_object().map(|o| !o.is_empty()).unwrap_or(false) {
        body["generationConfig"] = gc;
    }
    body
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
        .header("x-goog-api-key", key)
        .json(body)
        .send()
        .await
        .map_err(|e| LlmError::Network(e.to_string()))?;
    let status = resp.status();
    if status == reqwest::StatusCode::UNAUTHORIZED {
        return Err(LlmError::Unauthorized);
    }
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err(LlmError::RateLimited { retry_after_ms: 60_000 });
    }
    if !status.is_success() {
        let body = resp.text().await.unwrap_or_default();
        return Err(LlmError::BadStatus { status: status.as_u16(), body });
    }
    Ok(resp)
}

#[derive(Debug, Deserialize)]
struct GenerateResponse {
    #[serde(default)]
    candidates: Option<Vec<Candidate>>,
    #[serde(default)]
    usage_metadata: Option<UsageMetadata>,
}

#[derive(Debug, Deserialize)]
struct Candidate {
    #[serde(default)]
    content: Option<Content>,
}

#[derive(Debug, Deserialize)]
struct Content {
    #[serde(default)]
    parts: Vec<Part>,
}

#[derive(Debug, Deserialize)]
struct Part {
    #[serde(default)]
    text: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UsageMetadata {
    prompt_token_count: Option<u32>,
    candidates_token_count: Option<u32>,
    total_token_count: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm::ChatMessage;

    #[test]
    fn build_body_extracts_system_prompt_and_config() {
        let req = ChatRequest {
            model: "gemini-1.5-pro".into(),
            messages: vec![
                ChatMessage { role: "system".into(), content: "be brief".into() },
                ChatMessage { role: "user".into(), content: "hi".into() },
            ],
            temperature: Some(0.5),
            max_tokens: Some(512),
            top_p: Some(0.9),
            stop: Some(vec!["END".into()]),
            system: None,
        };
        let body = build_body(&req);
        assert_eq!(body["systemInstruction"]["parts"][0]["text"], "be brief");
        assert_eq!(body["contents"].as_array().unwrap().len(), 1);
        assert_eq!(body["contents"][0]["role"], "user");
        assert_eq!(body["generationConfig"]["temperature"], 0.5);
        assert_eq!(body["generationConfig"]["maxOutputTokens"], 512);
        assert_eq!(body["generationConfig"]["stopSequences"][0], "END");
    }

    #[test]
    fn assistant_role_maps_to_model() {
        let req = ChatRequest {
            model: "x".into(),
            messages: vec![ChatMessage { role: "assistant".into(), content: "hi".into() }],
            temperature: None,
            max_tokens: None,
            top_p: None,
            stop: None,
            system: None,
        };
        let body = build_body(&req);
        assert_eq!(body["contents"][0]["role"], "model");
    }

    #[test]
    fn collects_text_from_multiple_parts() {
        let r: GenerateResponse = serde_json::from_value(serde_json::json!({
            "candidates": [{
                "content": {
                    "parts": [
                        {"text": "Hello "},
                        {"text": "world"}
                    ]
                }
            }]
        })).unwrap();
        assert_eq!(collect_text(&r), "Hello world");
    }
}