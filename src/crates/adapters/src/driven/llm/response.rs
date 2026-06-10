//! LLM 응답 파싱 — file-pipeline 도메인 (`Metadata` / `ClassifyAndProcessResult` 변환).
//!
//! module-llm의 raw 텍스트 출력에서 JSON을 추출하여 file-pipeline 도메인 모델로 변환.
//! 모든 LLM 어댑터가 공유.

use std::collections::HashMap;

use anyhow::{Context, Result};
use file_pipeline_core::domain::models::{ClassifyAndProcessResult, Metadata};
use serde::Deserialize;
use tracing::debug;

#[derive(Deserialize)]
pub struct LlmResponse {
    pub doc_types: Vec<String>,
    pub rationale: String,
    pub date: String,
    pub summary: String,
    pub keywords: Vec<String>,
    pub content: String,
    #[serde(default)]
    pub sections: Option<HashMap<String, Vec<String>>>,
    /// 검색 힌트 — 사용자가 이 문서를 찾을 때 입력할 만한 질문/키워드
    #[serde(default)]
    pub search_hints: Vec<String>,
    /// 코드블록 구조화
    #[serde(default)]
    pub code_blocks: Vec<CodeBlock>,
    /// LLM이 추출한 엔티티 (사람/조직/기술/금액/프로젝트)
    #[serde(default)]
    pub entities: Vec<LlmEntity>,
    /// Phase 88 (wikidocs 353407): 원문 미확인/추가 검증 필요 항목
    #[serde(default)]
    pub needs_verification: Vec<String>,
    /// Phase 88 (wikidocs 353407): 원문으로 답할 수 없는 후속 질문
    #[serde(default)]
    pub open_questions: Vec<String>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct LlmEntity {
    #[serde(default)]
    pub name: String,
    #[serde(default, rename = "type")]
    pub entity_type: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct CodeBlock {
    #[serde(default)]
    pub language: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub code: String,
}

/// LLM 출력에서 균형 잡힌 JSON 객체 추출 + 파싱.
pub fn parse_llm_response(raw: &str) -> Result<LlmResponse> {
    let json_str = if let Some(start) = raw.find('{') {
        let mut depth = 0i32;
        let mut end = start;
        for (i, ch) in raw[start..].char_indices() {
            match ch {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = start + i + 1;
                        break;
                    }
                }
                _ => {}
            }
        }
        &raw[start..end]
    } else {
        raw
    };

    debug!("JSON 파싱 시도: {}...", &json_str[..json_str.len().min(200)]);
    serde_json::from_str(json_str).context("LLM JSON 응답 파싱 실패")
}

/// content 텍스트에서 `=== 섹션명 ===` / `## 섹션명` / `### 섹션명` 패턴 파싱.
pub fn parse_sections_from_content(content: &str) -> HashMap<String, Vec<String>> {
    let mut sections = HashMap::new();
    let mut current_section: Option<String> = None;
    let mut current_lines: Vec<String> = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        let is_section = (trimmed.starts_with("===") && trimmed.ends_with("===") && trimmed.len() > 6)
            || (trimmed.starts_with("## ") && !trimmed.starts_with("## #"))
            || trimmed.starts_with("### ");
        if is_section {
            if let Some(ref sec) = current_section {
                let lines: Vec<String> = current_lines.iter().filter(|l| !l.is_empty()).cloned().collect();
                if !lines.is_empty() {
                    sections.insert(sec.clone(), lines);
                }
            }
            let name = trimmed
                .trim_start_matches('=')
                .trim_end_matches('=')
                .trim_start_matches('#')
                .trim_start_matches(' ')
                .trim()
                .to_string();
            current_section = Some(name);
            current_lines.clear();
        } else if current_section.is_some() {
            current_lines.push(trimmed.to_string());
        }
    }

    if let Some(ref sec) = current_section {
        let lines: Vec<String> = current_lines.iter().filter(|l| !l.is_empty()).cloned().collect();
        if !lines.is_empty() {
            sections.insert(sec.clone(), lines);
        }
    }

    sections
}

/// `LlmResponse` → `ClassifyAndProcessResult` 변환 (어댑터 공통).
pub fn build_classify_result(resp: LlmResponse) -> ClassifyAndProcessResult {
    let metadata = Metadata {
        doc_types: resp.doc_types.clone(),
        rationale: resp.rationale.clone(),
        date: resp.date,
        summary: resp.summary,
        keywords: resp.keywords,
        sensitive: false,
        doi: None,
        related_docs: vec![],
        source_doc_ids: vec![],
        search_hints: resp.search_hints,
        entities: resp.entities.iter().map(|e| (e.name.clone(), e.entity_type.clone())).collect(),
        needs_verification: resp.needs_verification,
        open_questions: resp.open_questions,
        ..Default::default()
    };

    let sections = resp.sections.unwrap_or_else(|| parse_sections_from_content(&resp.content));

    ClassifyAndProcessResult {
        doc_types: resp.doc_types,
        rationale: resp.rationale,
        content: resp.content,
        metadata,
        sections: if sections.is_empty() { None } else { Some(sections) },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_clean_json() {
        let raw = r#"{"doc_types":["meeting"],"rationale":"회의록","date":"2026-04-05","summary":"테스트","keywords":["test"],"content":"=== meeting ===\n내용"}"#;
        let resp = parse_llm_response(raw).unwrap();
        assert_eq!(resp.doc_types, vec!["meeting"]);
    }

    #[test]
    fn parse_with_markdown_codeblock() {
        let raw = r#"```json
{"doc_types":["study"],"rationale":"학습 노트","date":"2026-04-05","summary":"요약","keywords":["rust"],"content":"내용"}
```"#;
        let resp = parse_llm_response(raw).unwrap();
        assert_eq!(resp.doc_types, vec!["study"]);
    }

    #[test]
    fn parse_with_trailing_text() {
        let raw = r#"{"doc_types":["meeting"],"rationale":"회의록","date":"2026-04-14","summary":"테스트","keywords":["test"],"content":"내용"}
Some trailing commentary here"#;
        let resp = parse_llm_response(raw).unwrap();
        assert_eq!(resp.doc_types, vec!["meeting"]);
        assert_eq!(resp.rationale, "회의록");
    }

    #[test]
    fn parse_missing_required_field_fails() {
        let raw = r#"{"doc_types":["meeting"],"date":"2026-04-14","summary":"테스트","keywords":["test"],"content":"내용"}"#;
        assert!(parse_llm_response(raw).is_err());
    }
}
