//! Claude CLI LLMPort 어댑터 — module-llm `ClaudeCliRawClient` thin wrapper.

use std::path::Path;

use anyhow::{Context, Result};
use async_trait::async_trait;
use file_pipeline_core::domain::models::{
    ClassifyAndProcessResult, DocTypeRegistry, EnrichResult,
};
use file_pipeline_core::ports::output::LLMPort;
use module_llm::{ClaudeCliRawClient, LlmRawPort};
use tracing::info;

use super::map_err;
use super::prompts;
use super::response::{build_classify_result, parse_llm_response};

/// 청크 분할 기준 (글자 수)
const CHUNK_SIZE: usize = 40_000;

pub struct ClaudeCliAdapter {
    raw: ClaudeCliRawClient,
}

impl ClaudeCliAdapter {
    pub fn new() -> Self {
        Self { raw: ClaudeCliRawClient::new() }
    }

    pub fn with_bin(claude_bin: String) -> Self {
        Self { raw: ClaudeCliRawClient::with_bin(claude_bin) }
    }

    /// 프로필 경로(CLAUDE_CONFIG_DIR)를 지정하여 생성
    pub fn with_config_dir(self, config_dir: Option<String>) -> Self {
        Self { raw: self.raw.with_config_dir(config_dir) }
    }

    fn read_file(path: &Path) -> Result<String> {
        std::fs::read_to_string(path).context(format!("파일 읽기 실패: {:?}", path))
    }

    /// 대용량 파일이면 앞부분만 추출 (UTF-8 안전)
    fn truncate_content(content: &str) -> &str {
        if content.len() <= CHUNK_SIZE {
            content
        } else {
            let mut end = CHUNK_SIZE;
            while !content.is_char_boundary(end) && end > 0 {
                end -= 1;
            }
            &content[..end]
        }
    }
}

impl Default for ClaudeCliAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LLMPort for ClaudeCliAdapter {
    async fn classify_and_process(
        &self,
        file_path: &Path,
        registry: &DocTypeRegistry,
    ) -> Result<ClassifyAndProcessResult> {
        let filename = file_path.file_name().unwrap_or_default().to_string_lossy();
        let hints = prompts::build_type_hints(registry);

        let full = Self::read_file(file_path).map_err(|e| {
            let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("?");
            anyhow::anyhow!(
                "파일을 텍스트로 읽을 수 없습니다 (.{}). 전처리기(Preprocess 스텝)를 파이프라인에 추가하세요.\n\
                 지원 도구: pandoc, python-docx, openpyxl, libreoffice\n\
                 원본 에러: {}",
                ext,
                e
            )
        })?;
        let content = Self::truncate_content(&full);
        let prompt = prompts::build_classify_prompt(&filename, content, &hints);

        info!("Claude CLI 호출: {} ({} chars)", filename, content.len());
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
        let full = Self::read_file(file_path)?;
        let content = Self::truncate_content(&full);
        let filename = file_path.file_name().unwrap_or_default().to_string_lossy();
        let hints = prompts::build_type_hints(registry);
        let prompt = prompts::build_reprocess_prompt(&filename, content, &hints, feedback);

        info!("Claude CLI 피드백 재가공: {} ({} chars)", filename, content.len());
        let raw = self.raw.call_text("", &prompt, 4096).await.map_err(map_err)?;
        let resp = parse_llm_response(&raw)?;
        Ok(build_classify_result(resp))
    }

    async fn enrich_existing(
        &self,
        existing_content: &str,
        new_info: &str,
        doc_types: &[String],
    ) -> Result<EnrichResult> {
        let prompt = format!(
            r#"기존 문서에 새로운 정보를 통합하세요.

## 규칙
- 기존 문서의 구조와 섹션을 유지하세요
- 새 정보 중 기존에 없는 내용만 추가하세요
- 모순되는 정보가 있으면 새 정보를 우선하세요
- 변경 사항을 한 줄로 요약하세요
- 변경할 내용이 없으면 "NO_CHANGE"를 출력하세요

## 문서 유형: {types}

## 기존 문서
{existing}

## 새 정보
{new_info}

## 출력 형식
첫 줄: CHANGE_SUMMARY: (변경 요약) 또는 NO_CHANGE
나머지: 보강된 전체 문서"#,
            types = doc_types.join(", "),
            existing = Self::truncate_content(existing_content),
            new_info = Self::truncate_content(new_info),
        );

        let raw = self.raw.call_text("", &prompt, 4096).await.map_err(map_err)?;

        if raw.contains("NO_CHANGE") {
            return Ok(EnrichResult {
                updated_content: existing_content.to_string(),
                change_summary: String::new(),
                should_update: false,
            });
        }

        let (summary, content) = if let Some(idx) = raw.find('\n') {
            let summary_line = &raw[..idx];
            let summary = summary_line
                .strip_prefix("CHANGE_SUMMARY:")
                .unwrap_or(summary_line)
                .trim()
                .to_string();
            (summary, raw[idx + 1..].to_string())
        } else {
            ("보강됨".to_string(), raw)
        };

        Ok(EnrichResult {
            updated_content: content,
            change_summary: summary,
            should_update: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_content_below_chunk_size() {
        let short = "짧은 내용";
        assert_eq!(ClaudeCliAdapter::truncate_content(short), short);
    }

    #[test]
    fn truncate_content_above_chunk_size() {
        let long = "a".repeat(50_000);
        let truncated = ClaudeCliAdapter::truncate_content(&long);
        assert_eq!(truncated.len(), CHUNK_SIZE);
    }
}

// step-o2 (2026-06-16, outbound-umbrella-1): OutboundManifest 박힘
impl file_pipeline_core::ports::outbound::OutboundManifest for ClaudeCliAdapter {
    fn id(&self) -> &str { "fp-outbound-llm-claude" }
    fn category(&self) -> file_pipeline_core::ports::outbound::OutboundCategory {
        file_pipeline_core::ports::outbound::OutboundCategory::Llm
    }
    fn capabilities(&self) -> file_pipeline_core::ports::output::ResourceCapabilities {
        file_pipeline_core::ports::output::ResourceCapabilities::standard("claude")
    }
}
