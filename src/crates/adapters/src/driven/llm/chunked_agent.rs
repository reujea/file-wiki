//! 대용량 파일 에이전트 — 청크 분할 → 청크별 LLM 호출 → 병합
//!
//! 헥사고날 경계: Driven adapter (Decorator 패턴).
//! 내부 `LLMPort`에 위임하되, 대용량 파일은 청크로 나눠 처리.
//! 청크 분할은 `module_llm_chunked::ByteSplitter`/`FnSplitter` 또는
//! `core::domain::chunking::split_semantic`을 사용.

use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use file_pipeline_core::domain::chunking::{self, SemanticChunkConfig};
use file_pipeline_core::domain::models::*;
use file_pipeline_core::ports::output::LLMPort;
use module_llm_chunked::{ByteSplitter, Splitter};
use tracing::info;

use super::prompts;

/// 청크 크기 (40KB) — 바이트 분할 fallback 용
const CHUNK_SIZE: usize = prompts::CHUNK_SIZE;

pub struct ChunkedAgentAdapter {
    inner: Arc<dyn LLMPort>,
    /// 의미 단위 청킹 설정 (None이면 module byte splitter)
    semantic_config: Option<SemanticChunkConfig>,
}

impl ChunkedAgentAdapter {
    pub fn new(inner: Arc<dyn LLMPort>) -> Self {
        Self { inner, semantic_config: None }
    }

    pub fn with_semantic_chunking(mut self, config: SemanticChunkConfig) -> Self {
        self.semantic_config = Some(config);
        self
    }

    fn is_large_file(path: &Path) -> bool {
        std::fs::metadata(path).map(|m| m.len() as usize > CHUNK_SIZE).unwrap_or(false)
    }

    fn build_chunk_prompt(filename: &str, idx: usize, total: usize, body: &str, type_hints: &str) -> String {
        format!(
            r#"당신은 문서 분석 에이전트입니다.
아래는 큰 문서의 일부분입니다 (청크 {n}/{total}). 이 부분만 분석하세요.

## 작업
1. 이 부분의 문서 유형을 판단하세요
2. 핵심 내용을 요약하세요 (3~5문장)
3. 중요 키워드를 추출하세요 (최대 10개)
4. 날짜, 금액, 고유명사를 모두 보존하세요

{type_hints}

## 출력 형식 (JSON만 출력)
```json
{{
  "doc_types": ["meeting"],
  "summary": "이 부분의 요약...",
  "keywords": ["키워드1", "키워드2"],
  "key_entities": ["날짜/금액/이름 등"],
  "content": "핵심 내용 정리..."
}}
```

## 문서 (파일명: {filename}, 청크 {n}/{total})

{body}"#,
            n = idx + 1,
            total = total,
            type_hints = type_hints,
            filename = filename,
            body = body,
        )
    }

    fn build_merge_prompt(_filename: &str, summaries: &[ChunkSummary], type_hints: &str) -> String {
        let mut sections = String::new();
        for (i, r) in summaries.iter().enumerate() {
            sections.push_str(&format!(
                "### 청크 {} 결과\n유형: {:?}\n요약: {}\n키워드: {:?}\n핵심: {}\n\n",
                i + 1, r.doc_types, r.summary, r.keywords, r.content,
            ));
        }
        format!(
            r#"당신은 문서 통합 에이전트입니다.
큰 문서를 {n} 개 청크로 나누어 분석한 결과를 하나로 통합하세요.

{type_hints}

## 각 청크 분석 결과
{sections}

## 출력 형식 (JSON만 출력)
```json
{{
  "doc_types": ["최종 유형"],
  "rationale": "유형 판단 근거",
  "date": "문서 날짜",
  "summary": "전체 문서 2~3문장 요약",
  "keywords": ["통합 키워드 최대 15개"],
  "sections": {{ "섹션명": ["항목"] }},
  "content": "=== 유형 ===\n통합된 가공 내용"
}}
```"#,
            n = summaries.len(),
            type_hints = type_hints,
            sections = sections,
        )
    }

    /// 청크 텍스트 분할 — semantic_config 있으면 의미 단위, 없으면 module byte splitter
    fn split_chunks(&self, content: &str) -> Vec<(usize, String)> {
        if let Some(ref sc) = self.semantic_config {
            chunking::split_semantic(content, sc)
                .into_iter()
                .map(|c| (c.index, c.text))
                .collect()
        } else {
            ByteSplitter::new(CHUNK_SIZE)
                .split(content)
                .into_iter()
                .enumerate()
                .collect()
        }
    }
}

#[derive(Debug)]
struct ChunkSummary {
    doc_types: Vec<String>,
    summary: String,
    keywords: Vec<String>,
    content: String,
}

#[async_trait]
impl LLMPort for ChunkedAgentAdapter {
    async fn classify_and_process(
        &self,
        file_path: &Path,
        registry: &DocTypeRegistry,
    ) -> Result<ClassifyAndProcessResult> {
        if !Self::is_large_file(file_path) {
            return self.inner.classify_and_process(file_path, registry).await;
        }

        // 바이너리 파일(PDF 등)은 read 실패 → 내부 LLM에 직접 위임
        let content = match std::fs::read_to_string(file_path) {
            Ok(c) => c,
            Err(_) => {
                tracing::info!("대용량 바이너리 파일 → 내부 LLM에 직접 위임: {:?}", file_path);
                return self.inner.classify_and_process(file_path, registry).await;
            }
        };
        let fname = file_path.file_name().unwrap_or_default().to_string_lossy().to_string();
        let type_hints = prompts::build_type_hints(registry);

        let chunk_texts = self.split_chunks(&content);
        let total = chunk_texts.len();
        info!("대용량 파일 에이전트: {} → {} 청크로 분할", fname, total);

        // 1. 각 청크를 inner LLM에 위임 (임시 파일에 청크 프롬프트 기록 → classify_and_process)
        let mut summaries = Vec::with_capacity(total);
        for (idx, chunk_content) in &chunk_texts {
            let prompt = Self::build_chunk_prompt(&fname, *idx, total, chunk_content, &type_hints);
            let tmp = tempfile::NamedTempFile::new()?;
            std::fs::write(tmp.path(), &prompt)?;

            match self.inner.classify_and_process(tmp.path(), registry).await {
                Ok(result) => {
                    summaries.push(ChunkSummary {
                        doc_types: result.doc_types,
                        summary: result.metadata.summary,
                        keywords: result.metadata.keywords,
                        content: result.content,
                    });
                }
                Err(e) => {
                    info!("청크 {}/{} 가공 실패: {} — 직접 요약으로 대체", idx + 1, total, e);
                    summaries.push(ChunkSummary {
                        doc_types: vec![],
                        summary: chunk_content.chars().take(200).collect(),
                        keywords: vec![],
                        content: chunk_content.chars().take(500).collect(),
                    });
                }
            }
        }

        // 2. 병합 LLM 호출
        let merge_prompt = Self::build_merge_prompt(&fname, &summaries, &type_hints);
        let tmp_merge = tempfile::NamedTempFile::new()?;
        std::fs::write(tmp_merge.path(), &merge_prompt)?;
        info!("대용량 파일 에이전트: {} 청크 결과 병합 중", total);
        let merged = self.inner.classify_and_process(tmp_merge.path(), registry).await?;

        info!("대용량 파일 에이전트: {} 완료 ({} 청크 → 유형: {:?})", fname, total, merged.doc_types);
        Ok(merged)
    }

    async fn summarize_text(&self, new: &str, existing: &str) -> Result<String> {
        self.inner.summarize_text(new, existing).await
    }

    async fn generate_hypothetical(&self, query: &str) -> Result<String> {
        self.inner.generate_hypothetical(query).await
    }

    async fn reprocess_with_feedback(
        &self,
        file_path: &Path,
        registry: &DocTypeRegistry,
        feedback: &str,
    ) -> Result<ClassifyAndProcessResult> {
        if !Self::is_large_file(file_path) {
            return self.inner.reprocess_with_feedback(file_path, registry, feedback).await;
        }
        self.classify_and_process(file_path, registry).await
    }

    async fn enrich_existing(
        &self,
        existing: &str,
        new_info: &str,
        doc_types: &[String],
    ) -> Result<EnrichResult> {
        self.inner.enrich_existing(existing, new_info, doc_types).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_large_file_nonexistent_is_false() {
        assert!(!ChunkedAgentAdapter::is_large_file(std::path::Path::new("/nonexistent")));
    }

    #[test]
    fn build_chunk_prompt_contains_metadata() {
        let prompt = ChunkedAgentAdapter::build_chunk_prompt("test.txt", 0, 3, "내용", "힌트");
        assert!(prompt.contains("test.txt"));
        assert!(prompt.contains("1/3"));
        assert!(prompt.contains("내용"));
    }

    #[test]
    fn build_merge_prompt_contains_chunk_summaries() {
        let summaries = vec![
            ChunkSummary { doc_types: vec!["meeting".into()], summary: "요약1".into(), keywords: vec![], content: "내용1".into() },
            ChunkSummary { doc_types: vec!["study".into()], summary: "요약2".into(), keywords: vec![], content: "내용2".into() },
        ];
        let prompt = ChunkedAgentAdapter::build_merge_prompt("test.txt", &summaries, "");
        assert!(prompt.contains("요약1"));
        assert!(prompt.contains("요약2"));
        assert!(prompt.contains("2 개 청크"));
    }
}

// step-o2 (2026-06-16, outbound-umbrella-1): OutboundManifest 박힘
impl file_pipeline_core::ports::outbound::OutboundManifest for ChunkedAgentAdapter {
    fn id(&self) -> &str { "fp-outbound-llm-chunked" }
    fn category(&self) -> file_pipeline_core::ports::outbound::OutboundCategory {
        file_pipeline_core::ports::outbound::OutboundCategory::Llm
    }
    fn capabilities(&self) -> file_pipeline_core::ports::output::ResourceCapabilities {
        file_pipeline_core::ports::output::ResourceCapabilities::standard("chunked")
    }
}
