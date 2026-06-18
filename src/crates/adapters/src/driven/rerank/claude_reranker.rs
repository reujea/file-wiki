use std::process::Command;
use anyhow::{Context, Result};
use async_trait::async_trait;
use file_pipeline_core::domain::models::SimilarDoc;
use file_pipeline_core::ports::output::RerankerPort;

/// Claude CLI 기반 리랭커 — 검색 결과를 Claude에게 관련도 평가 요청
pub struct ClaudeReranker {
    claude_bin: String,
    top_n: usize,
}

impl ClaudeReranker {
    pub fn new(top_n: usize) -> Self {
        Self {
            claude_bin: std::env::var("CLAUDE_BIN").unwrap_or_else(|_| "claude".into()),
            top_n,
        }
    }

    fn call_claude(&self, prompt: &str) -> Result<String> {
        let mut cmd = Command::new(&self.claude_bin);
        cmd.args(["--print", "--output-format", "text", "--max-tokens", "2000"])
            .arg("--prompt")
            .arg(prompt);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        let output = cmd.output()
            .context("Claude CLI 실행 실패")?;
        if !output.status.success() {
            anyhow::bail!("Claude CLI 오류: {}", String::from_utf8_lossy(&output.stderr));
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn parse_scores(text: &str, count: usize) -> Vec<f32> {
        // Expect lines like "1: 8.5" or "0: 7.2" or just numbers per line
        let mut scores = Vec::new();
        for line in text.lines() {
            let trimmed = line.trim();
            // Try "N: score" format
            let score_str = if let Some(idx) = trimmed.find(':') {
                trimmed[idx + 1..].trim()
            } else {
                trimmed
            };
            if let Ok(s) = score_str.parse::<f32>() {
                scores.push(s);
            }
        }
        // Pad with 0.0 if not enough scores
        while scores.len() < count {
            scores.push(0.0);
        }
        scores.truncate(count);
        scores
    }
}

#[async_trait]
impl RerankerPort for ClaudeReranker {
    async fn rerank(&self, query: &str, mut candidates: Vec<SimilarDoc>) -> Result<Vec<SimilarDoc>> {
        if candidates.is_empty() {
            return Ok(candidates);
        }

        let take = candidates.len().min(self.top_n);
        candidates.truncate(take);

        // Build prompt with candidate summaries
        let mut prompt = format!(
            "검색 쿼리: \"{}\"\n\n아래 {} 개 문서의 관련도를 0~10 점수로 평가하세요.\n\
             각 줄에 \"번호: 점수\" 형식으로만 출력하세요.\n\n",
            query, candidates.len()
        );

        for (i, doc) in candidates.iter().enumerate() {
            prompt.push_str(&format!(
                "문서 {}: [{}] {}\n",
                i,
                doc.doc_types.join(", "),
                doc.id,
            ));
        }

        match self.call_claude(&prompt) {
            Ok(response) => {
                let scores = Self::parse_scores(&response, candidates.len());
                // Attach scores and sort descending
                let mut scored: Vec<(f32, SimilarDoc)> = scores.into_iter()
                    .zip(candidates)
                    .collect();
                scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
                Ok(scored.into_iter().map(|(score, mut doc)| {
                    doc.score = score / 10.0; // Normalize to 0-1
                    doc
                }).collect())
            }
            Err(e) => {
                tracing::warn!("리랭킹 실패: {} — 원본 순서 유지", e);
                Ok(candidates)
            }
        }
    }

    fn is_enabled(&self) -> bool { true }
}

#[cfg(test)]
mod tests {
    use super::*;
    use file_pipeline_core::domain::models::SimilarDoc;
    use file_pipeline_core::ports::output::RerankerPort;
    use std::path::PathBuf;

    #[test]
    fn test_parse_scores_standard() {
        let text = "0: 8.5\n1: 7.2\n2: 3.0";
        let scores = ClaudeReranker::parse_scores(text, 3);
        assert_eq!(scores.len(), 3);
        assert!((scores[0] - 8.5).abs() < 0.01);
        assert!((scores[1] - 7.2).abs() < 0.01);
    }

    #[test]
    fn test_parse_scores_plain_numbers() {
        let text = "8.5\n7.2\n3.0";
        let scores = ClaudeReranker::parse_scores(text, 3);
        assert_eq!(scores.len(), 3);
    }

    #[test]
    fn test_parse_scores_padding() {
        let text = "8.5";
        let scores = ClaudeReranker::parse_scores(text, 3);
        assert_eq!(scores.len(), 3);
        assert!((scores[0] - 8.5).abs() < 0.01);
        assert_eq!(scores[1], 0.0);
    }

    #[tokio::test]
    async fn test_null_reranker_passthrough() {
        use crate::driven::rerank::null_reranker::NullReranker;
        let reranker = NullReranker;
        assert!(!reranker.is_enabled());
        let docs = vec![
            SimilarDoc { id: "a".into(), path: PathBuf::from("a.zst"), score: 0.9, doc_types: vec!["meeting".into()], date: "2026-04-14".into(), hierarchy: vec![] },
            SimilarDoc { id: "b".into(), path: PathBuf::from("b.zst"), score: 0.5, doc_types: vec!["study".into()], date: "2026-04-13".into(), hierarchy: vec![] },
        ];
        let result = reranker.rerank("test query", docs.clone()).await.expect("rerank 실패");
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, "a");
    }
}
