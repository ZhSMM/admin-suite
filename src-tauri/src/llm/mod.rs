//! LLM provider abstraction.
//!
//! Each provider implements `LlmProvider` for `chat()` and `chat_stream()`.
//! The OpenAI-Compatible adapter covers ~80% of providers (OpenAI, DeepSeek,
//! Ollama /v1, LM Studio, vLLM, llama.cpp server, OpenRouter, Zhipu, ...).
//! Anthropic and Google get their own adapter because their wire formats
//! diverge enough that sharing code hurts.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::crypto;

pub mod fallback;
pub mod providers;

/// Vendor-agnostic chat request passed to every adapter.
#[derive(Debug, Clone)]
pub struct ChatRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: Option<f32>,
    pub max_tokens: Option<u32>,
    pub top_p: Option<f32>,
    pub stop: Option<Vec<String>>,
    pub system: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: String, // "system" | "user" | "assistant"
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct ChatResponse {
    pub content: String,
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub model: Option<String>,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct StreamChunk {
    pub delta: String,
    /// Provider-specific finish reason; surfaced verbatim to the UI.
    pub finish_reason: Option<String>,
}

/// What the Rust side needs to make an HTTP call. Decrypted API key lives
/// here (never crosses the IPC boundary).
#[derive(Debug, Clone)]
pub struct ProviderContext {
    pub provider_id: String,
    pub base_url: String,
    pub auth_type: String,
    pub auth_header: Option<String>,
    pub api_key: Option<String>,
    pub timeout_ms: u64,
    /// reqwest client built per-request. Tauri command layer holds the
    /// shared client; we just borrow it.
    pub http: reqwest::Client,
}

#[derive(Debug, thiserror::Error, Serialize)]
pub enum LlmError {
    #[error("network error: {0}")]
    Network(String),
    #[error("auth failed: check API key in provider settings")]
    Unauthorized,
    #[error("rate limited (retry after {retry_after_ms}ms)")]
    RateLimited { retry_after_ms: u64 },
    #[error("context length exceeded: prompt={prompt} max={max}")]
    ContextOverflow { prompt: usize, max: usize },
    #[error("provider returned {status}: {body}")]
    BadStatus { status: u16, body: String },
    #[error("decoding response: {0}")]
    Decode(String),
    #[error("operation not supported: {0}")]
    Unsupported(String),
    #[error("internal: {0}")]
    Internal(String),
}

/// One remote model entry returned by `LlmProvider::list_models`.
///
/// `display_name` and `context_window` are best-effort — some providers
/// don't expose them, in which case the field is `None`.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProviderModelInfo {
    pub id: String,
    pub display_name: Option<String>,
    pub context_window: Option<u32>,
    /// Family hint ("chat", "embeddings", "image"...) when provider exposes it.
    pub kind: Option<String>,
}

#[async_trait]
pub trait LlmProvider: Send + Sync {
    fn kind(&self) -> &'static str;

    /// Whether this provider exposes a "list available models" endpoint
    /// we know how to call. Default `false` for adapters we don't have a
    /// probe for (custom, fallback, etc).
    fn supports_list_models(&self) -> bool { false }

    /// Optional: fetch the provider's published model catalog.
    /// Default: `Err(Unsupported)` — caller falls back to manual entry.
    async fn list_models(
        &self,
        _ctx: &ProviderContext,
    ) -> Result<Vec<ProviderModelInfo>, LlmError> {
        Err(LlmError::Unsupported(
            "this provider does not expose model discovery".into(),
        ))
    }

    async fn chat(
        &self,
        ctx: &ProviderContext,
        req: ChatRequest,
    ) -> Result<ChatResponse, LlmError>;
    async fn chat_stream(
        &self,
        ctx: &ProviderContext,
        req: ChatRequest,
        on_chunk: Box<dyn Fn(StreamChunk) + Send + Sync>,
    ) -> Result<ChatResponse, LlmError>;
}

/// Decrypt `api_key_enc` if present.  Returns None when the column is NULL
/// or when auth_type is 'none' (Ollama local, etc).
pub fn decrypt_api_key(
    blob: Option<&[u8]>,
    master: &crypto::MasterKey,
) -> Result<Option<String>, LlmError> {
    match blob {
        None => Ok(None),
        Some(b) => master
            .decrypt(b)
            .map(Some)
            .map_err(|e| LlmError::Internal(format!("api key decrypt: {e}"))),
    }
}

/// Build a reqwest Client with the provider's configured timeout.
pub fn http_client(timeout_ms: u64) -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(timeout_ms))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}

/// Lookup a provider's `kind` from the DB by id. Used by discovery
/// commands so the right adapter gets selected.
pub fn provider_kind_for_id_static(
    state: &crate::AppState,
    provider_id: &str,
) -> Result<String, crate::error::AppError> {
    state
        .db
        .with_conn(|c| {
            c.query_row(
                "SELECT kind FROM llm_providers WHERE id = ?1",
                [provider_id],
                |r| r.get::<_, String>(0),
            )
            .map_err(crate::error::AppError::from)
        })
}