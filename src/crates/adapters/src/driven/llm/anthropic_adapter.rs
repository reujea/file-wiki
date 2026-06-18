//! Anthropic LLMPort 어댑터 — module-llm `AnthropicRawClient` thin wrapper.

use std::path::Path;

use anyhow::{Context, Result};
use async_trait::async_trait;
use file_pipeline_core::domain::models::{
    ClassifyAndProcessResult, DocTypeRegistry, EnrichResult,
};
use file_pipeline_core::ports::output::LLMPort;
use module_llm::{AnthropicRawClient, LlmRawPort};
use tracing::info;

use super::map_err;
use super::prompts;
use super::response::{build_classify_result, parse_llm_response};

pub struct AnthropicApiAdapter {
    raw: AnthropicRawClient,
}

impl AnthropicApiAdapter {
    pub fn new(api_key: String, model: String) -> Self {
        Self { raw: AnthropicRawClient::new(api_key, model) }
    }
}

#[async_trait]
impl LLMPort for AnthropicApiAdapter {
    async fn classify_and_process(
        &self,
        file_path: &Path,
        registry: &DocTypeRegistry,
    ) -> Result<ClassifyAndProcessResult> {
        let full = std::fs::read_to_string(file_path).context("파일 읽기 실패")?;
        let content = prompts::truncate_content(&full);
        let filename = file_path.file_name().unwrap_or_default().to_string_lossy();
        let hints = prompts::build_type_hints(registry);
        let prompt = prompts::build_classify_prompt(&filename, content, &hints);

        info!("Anthropic API 호출: {} ({} chars)", filename, content.len());
        let raw = self.raw.call_text("", &prompt, 4096).await.map_err(map_err)?;
        let resp = parse_llm_response(&raw)?;
        Ok(build_classify_result(resp))
    }

    async fn summarize_text(&self, new_content: &str, existing: &str) -> Result<String> {
        let prompt = prompts::build_summarize_text_prompt(new_content, existing);
        self.raw.call_text("", &prompt, 2048).await.map_err(map_err)
    }

    async fn generate_hypothetical(&self, query: &str) -> Result<String> {
        let prompt = prompts::build_hyde_prompt(query);
        self.raw.call_text("", &prompt, 512).await.map_err(map_err)
    }

    async fn reprocess_with_feedback(
        &self,
        file_path: &Path,
        registry: &DocTypeRegistry,
        feedback: &str,
    ) -> Result<ClassifyAndProcessResult> {
        let full = std::fs::read_to_string(file_path).context("파일 읽기 실패")?;
        let content = prompts::truncate_content(&full);
        let filename = file_path.file_name().unwrap_or_default().to_string_lossy();
        let hints = prompts::build_type_hints(registry);
        let prompt = prompts::build_reprocess_prompt(&filename, content, &hints, feedback);

        info!("Anthropic API 2-Pass 재가공: {}", filename);
        let raw = self.raw.call_text("", &prompt, 4096).await.map_err(map_err)?;
        let resp = parse_llm_response(&raw)?;
        Ok(build_classify_result(resp))
    }

    async fn enrich_existing(
        &self,
        _existing_content: &str,
        _new_content: &str,
        _doc_types: &[String],
    ) -> Result<EnrichResult> {
        Ok(EnrichResult {
            updated_content: String::new(),
            change_summary: String::new(),
            should_update: false,
        })
    }
}

// step-o2 (2026-06-16, outbound-umbrella-1): OutboundManifest 박힘
impl file_pipeline_core::ports::outbound::OutboundManifest for AnthropicApiAdapter {
    fn id(&self) -> &str { "fp-outbound-llm-anthropic" }
    fn category(&self) -> file_pipeline_core::ports::outbound::OutboundCategory {
        file_pipeline_core::ports::outbound::OutboundCategory::Llm
    }
    fn capabilities(&self) -> file_pipeline_core::ports::output::ResourceCapabilities {
        file_pipeline_core::ports::output::ResourceCapabilities::standard("anthropic")
    }
}
