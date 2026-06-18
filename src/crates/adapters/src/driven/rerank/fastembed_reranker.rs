//! fastembed BGE-Reranker-v2-M3 Cross-Encoder 리랭커 — `RerankerPort` 구현.
//!
//! Q3 결정 (2026-04-29): ClaudeReranker(LLM API 호출) 대체 후보. 로컬 ms 단위 처리.
//!
//! # 후보 텍스트 추출
//!
//! `SimilarDoc`은 `path`만 보유하므로 path에서 zstd 해제 후 헤더 N줄을 읽어
//! 리랭커에 전달한다. 전체 본문 대신 헤더 사용은 비용 절감 + 충분한 신호.

use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use async_trait::async_trait;
use fastembed::{RerankInitOptions, RerankerModel, TextRerank};
use file_pipeline_core::domain::models::SimilarDoc;
use file_pipeline_core::ports::output::RerankerPort;
use tokio::sync::Mutex;

/// 리랭커가 평가할 후보 텍스트 길이 (헤더 줄 수).
const HEADER_LINES: usize = 30;

pub struct FastEmbedReranker {
    model: Arc<Mutex<TextRerank>>,
    top_n: usize,
    enabled: bool,
}

impl FastEmbedReranker {
    pub fn new(top_n: usize) -> Result<Self> {
        let model = TextRerank::try_new(RerankInitOptions::new(RerankerModel::BGERerankerV2M3))
            .context("fastembed BGE-Reranker-v2-M3 로드 실패")?;
        tracing::info!("fastembed BGE-Reranker-v2-M3 로드 완료 (top_n={})", top_n);
        Ok(Self { model: Arc::new(Mutex::new(model)), top_n, enabled: true })
    }

    /// zstd 압축 파일 헤더 N줄 추출 (== CONTENT == 마커 또는 제한까지)
    fn extract_text(path: &Path) -> Result<String> {
        use std::io::{BufRead, BufReader};

        let file = std::fs::File::open(path).context("후보 파일 열기 실패")?;
        let decoder = zstd::Decoder::new(file).context("zstd 해제 실패")?;
        let reader = BufReader::new(decoder);

        let mut text = String::new();
        for (i, line) in reader.lines().enumerate() {
            if i >= HEADER_LINES {
                break;
            }
            let line = line.context("zstd 라인 읽기 실패")?;
            if line.contains("=== CONTENT ===") {
                break;
            }
            text.push_str(&line);
            text.push('\n');
        }
        Ok(text)
    }
}

#[async_trait]
impl RerankerPort for FastEmbedReranker {
    async fn rerank(&self, query: &str, mut candidates: Vec<SimilarDoc>) -> Result<Vec<SimilarDoc>> {
        if candidates.is_empty() {
            return Ok(candidates);
        }
        let take = candidates.len().min(self.top_n);
        candidates.truncate(take);

        // 후보별 텍스트 추출 (실패 시 doc_types + id로 대체)
        let documents: Vec<String> = candidates
            .iter()
            .map(|c| {
                Self::extract_text(&c.path).unwrap_or_else(|e| {
                    tracing::warn!("후보 텍스트 추출 실패 ({}): {}", c.id, e);
                    format!("[{}] {}", c.doc_types.join(", "), c.id)
                })
            })
            .collect();

        let model = self.model.clone();
        let query_owned = query.to_string();
        let documents_clone = documents.clone();

        let scored = tokio::task::spawn_blocking(move || -> Result<Vec<(usize, f32)>> {
            let mut guard = model.blocking_lock();
            let results = guard
                .rerank::<String>(query_owned, documents_clone.as_slice(), true, None)
                .context("fastembed rerank 실패")?;
            // RerankResult { index, score, document } — fastembed가 score 내림차순 반환 추정
            Ok(results.into_iter().map(|r| (r.index, r.score)).collect())
        })
        .await
        .context("spawn_blocking 합류 실패")??;

        // 원본 candidates를 score 순으로 재배열
        let mut reordered: Vec<SimilarDoc> = scored
            .into_iter()
            .filter_map(|(idx, score)| {
                candidates.get(idx).cloned().map(|mut doc| {
                    doc.score = score;
                    doc
                })
            })
            .collect();

        // 누락된 후보가 있으면 끝에 추가 (안전망)
        if reordered.len() < candidates.len() {
            tracing::warn!("리랭커 결과 누락: {} → {}", candidates.len(), reordered.len());
            let kept_ids: std::collections::HashSet<String> =
                reordered.iter().map(|d| d.id.clone()).collect();
            for c in candidates {
                if !kept_ids.contains(&c.id) {
                    reordered.push(c);
                }
            }
        }

        Ok(reordered)
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }
}

// step-o2 (2026-06-16, outbound-umbrella-1): OutboundManifest 박힘
impl file_pipeline_core::ports::outbound::OutboundManifest for FastEmbedReranker {
    fn id(&self) -> &str { "fp-outbound-rerank-fastembed" }
    fn category(&self) -> file_pipeline_core::ports::outbound::OutboundCategory {
        file_pipeline_core::ports::outbound::OutboundCategory::Rerank
    }
    fn capabilities(&self) -> file_pipeline_core::ports::output::ResourceCapabilities {
        file_pipeline_core::ports::output::ResourceCapabilities::standard("fastembed")
    }
}
