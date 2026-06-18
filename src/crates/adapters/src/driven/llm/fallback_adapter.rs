//! Fallback LLM 어댑터 — 복수 프로바이더 순차 시도
//!
//! 헥사고날 경계: Driven adapter (Composite 패턴).
//! 첫 번째 성공하는 프로바이더 결과를 반환.

use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use tracing::{info, warn};

use file_pipeline_core::domain::models::*;
use file_pipeline_core::ports::output::LLMPort;

pub struct FallbackLlmAdapter {
    providers: Vec<(String, Arc<dyn LLMPort>)>,
}

impl FallbackLlmAdapter {
    pub fn new(providers: Vec<(String, Arc<dyn LLMPort>)>) -> Self {
        Self { providers }
    }
}

macro_rules! fallback_call {
    ($self:ident, $method:ident, $($arg:expr),*) => {{
        let mut last_err = None;
        for (name, provider) in &$self.providers {
            match provider.$method($($arg),*).await {
                Ok(result) => {
                    info!("LLM fallback: {} 성공", name);
                    return Ok(result);
                }
                Err(e) => {
                    warn!("LLM fallback: {} 실패 — {}", name, e);
                    last_err = Some(e);
                }
            }
        }
        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("프로바이더 없음")))
    }};
}

#[async_trait]
impl LLMPort for FallbackLlmAdapter {
    async fn classify_and_process(
        &self, file_path: &Path, registry: &DocTypeRegistry,
    ) -> Result<ClassifyAndProcessResult> {
        fallback_call!(self, classify_and_process, file_path, registry)
    }

    async fn summarize_text(&self, new_content: &str, existing: &str) -> Result<String> {
        fallback_call!(self, summarize_text, new_content, existing)
    }

    async fn generate_hypothetical(&self, query: &str) -> Result<String> {
        fallback_call!(self, generate_hypothetical, query)
    }

    async fn reprocess_with_feedback(
        &self, file_path: &Path, registry: &DocTypeRegistry, feedback: &str,
    ) -> Result<ClassifyAndProcessResult> {
        fallback_call!(self, reprocess_with_feedback, file_path, registry, feedback)
    }

    async fn enrich_existing(
        &self, existing: &str, new_info: &str, doc_types: &[String],
    ) -> Result<EnrichResult> {
        fallback_call!(self, enrich_existing, existing, new_info, doc_types)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stub::StubLlm;

    #[tokio::test]
    async fn test_fallback_first_succeeds() {
        let adapter = FallbackLlmAdapter::new(vec![
            ("stub".into(), Arc::new(StubLlm)),
        ]);
        let tmp = tempfile::NamedTempFile::new().expect("tmp");
        std::fs::write(tmp.path(), "테스트 내용").expect("write");
        let registry = DocTypeRegistry::new(vec![]);
        let result = adapter.classify_and_process(tmp.path(), &registry).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_fallback_empty_providers() {
        let adapter = FallbackLlmAdapter::new(vec![]);
        let tmp = tempfile::NamedTempFile::new().expect("tmp");
        std::fs::write(tmp.path(), "테스트").expect("write");
        let registry = DocTypeRegistry::new(vec![]);
        let result = adapter.classify_and_process(tmp.path(), &registry).await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("프로바이더 없음"));
    }

    #[tokio::test]
    async fn test_fallback_summarize_text() {
        let adapter = FallbackLlmAdapter::new(vec![
            ("stub".into(), Arc::new(StubLlm)),
        ]);
        let result = adapter.summarize_text("new", "old").await;
        assert!(result.is_ok());
    }
}

// step-o2 (2026-06-16, outbound-umbrella-1): OutboundManifest 박힘
impl file_pipeline_core::ports::outbound::OutboundManifest for FallbackLlmAdapter {
    fn id(&self) -> &str { "fp-outbound-llm-fallback" }
    fn category(&self) -> file_pipeline_core::ports::outbound::OutboundCategory {
        file_pipeline_core::ports::outbound::OutboundCategory::Llm
    }
    fn capabilities(&self) -> file_pipeline_core::ports::output::ResourceCapabilities {
        file_pipeline_core::ports::output::ResourceCapabilities::standard("fallback")
    }
}
