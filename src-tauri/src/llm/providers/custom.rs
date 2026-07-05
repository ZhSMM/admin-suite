//! Custom adapter — for any provider that speaks OpenAI-Compatible with a
//! different base_url, auth scheme, or minor quirks we don't bake into the
//! stock `OpenAiCompat` adapter. Currently delegates to `OpenAiCompat`; the
//! difference is in how the caller populates `ProviderContext` (auth_header
//! etc.) — the wire format is the same.

use async_trait::async_trait;

use crate::llm::providers::openai_compat::OpenAiCompat;
use crate::llm::{ChatRequest, ChatResponse, LlmError, LlmProvider, ProviderContext, StreamChunk};

pub struct CustomAdapter;

#[async_trait]
impl LlmProvider for CustomAdapter {
    fn kind(&self) -> &'static str { "custom" }
    async fn chat(&self, ctx: &ProviderContext, req: ChatRequest) -> Result<ChatResponse, LlmError> {
        OpenAiCompat.chat(ctx, req).await
    }
    async fn chat_stream(
        &self,
        ctx: &ProviderContext,
        req: ChatRequest,
        on_chunk: Box<dyn Fn(StreamChunk) + Send + Sync>,
    ) -> Result<ChatResponse, LlmError> {
        OpenAiCompat.chat_stream(ctx, req, on_chunk).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn custom_kind_label() {
        assert_eq!(CustomAdapter.kind(), "custom");
    }
}