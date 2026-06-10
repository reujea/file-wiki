pub mod anthropic_adapter;
pub mod chunked_agent;
pub mod claude_adapter;
pub mod fallback_adapter;
pub mod gemini_adapter;
pub mod ollama_adapter;
pub mod openai_llm_adapter;
pub mod prompts;
pub mod response;

/// module-llm `LlmError`를 file-pipeline `anyhow::Error`로 변환
pub(crate) fn map_err(e: module_llm_api::LlmError) -> anyhow::Error {
    anyhow::Error::msg(e.to_string())
}
