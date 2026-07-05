//! Fallback adapter — wires the OpenAI-Compatible adapter to a
//! locally-running llama-server. The server URL is owned by `FallbackManager`;
//! every chat call resolves it at runtime so port changes are picked up
//! without restarting the app.

use async_trait::async_trait;

use crate::llm::providers::openai_compat::OpenAiCompat;
use crate::llm::{
    ChatRequest, ChatResponse, LlmError, LlmProvider, ProviderContext, StreamChunk,
};

pub struct FallbackAdapter;

#[async_trait]
impl LlmProvider for FallbackAdapter {
    fn kind(&self) -> &'static str { "fallback" }
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
    fn fallback_kind_label() {
        assert_eq!(FallbackAdapter.kind(), "fallback");
    }
}