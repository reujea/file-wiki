//! fastembed BGE-M3 Sparse 임베더 — Phase 63 (하이브리드 검색 통합용).
//!
//! BGE-M3 모델의 sparse(lexical) 출력을 `SparseVector {indices, values}`로 반환.
//! 현재 file-pipeline은 `keyword_index`(HashMap)로 sparse를 운영하므로 본 어댑터는
//! **옵션 단계**: keyword_index 보강 또는 대체용. 통합 결정은 트리거 대기.
//!
//! # 빌드 요구사항
//!
//! Phase 62와 동일 — `fastembed` feature + MSVC v14.38+ 필요.

use std::sync::Arc;

use anyhow::{Context, Result};
use fastembed::{SparseInitOptions, SparseModel, SparseTextEmbedding};
use tokio::sync::Mutex;

/// BGE-M3 sparse 임베딩 결과 (file-pipeline 도메인 호환 형식).
#[derive(Debug, Clone)]
pub struct SparseVector {
    /// 활성 토큰 인덱스 (vocab 위치).
    pub indices: Vec<u32>,
    /// 각 인덱스의 가중치.
    pub values: Vec<f32>,
}

impl SparseVector {
    /// 비어있는지 (활성 토큰 0개).
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    /// 활성 토큰 수.
    pub fn len(&self) -> usize {
        self.indices.len()
    }

    /// 두 sparse 벡터의 dot product (검색 유사도 점수).
    pub fn dot(&self, other: &SparseVector) -> f32 {
        let mut score = 0.0;
        let mut i = 0;
        let mut j = 0;
        while i < self.indices.len() && j < other.indices.len() {
            match self.indices[i].cmp(&other.indices[j]) {
                std::cmp::Ordering::Equal => {
                    score += self.values[i] * other.values[j];
                    i += 1;
                    j += 1;
                }
                std::cmp::Ordering::Less => i += 1,
                std::cmp::Ordering::Greater => j += 1,
            }
        }
        score
    }
}

pub struct FastEmbedSparseAdapter {
    model: Arc<Mutex<SparseTextEmbedding>>,
}

impl FastEmbedSparseAdapter {
    pub fn new() -> Result<Self> {
        let model = SparseTextEmbedding::try_new(SparseInitOptions::new(SparseModel::BGEM3))
            .context("fastembed BGE-M3 sparse 로드 실패")?;
        tracing::info!("fastembed BGE-M3 sparse 로드 완료");
        Ok(Self { model: Arc::new(Mutex::new(model)) })
    }

    /// 단건 sparse 임베딩.
    pub async fn embed_sparse(&self, text: &str) -> Result<SparseVector> {
        let model = self.model.clone();
        let text = text.to_string();
        tokio::task::spawn_blocking(move || -> Result<SparseVector> {
            let mut guard = model.blocking_lock();
            let mut embeddings = guard.embed(vec![text], None).context("sparse embed 실패")?;
            let raw = embeddings.pop().context("빈 sparse 응답")?;
            Ok(SparseVector {
                indices: raw.indices.into_iter().map(|i| i as u32).collect(),
                values: raw.values,
            })
        })
        .await
        .context("spawn_blocking 합류 실패")?
    }

    /// 배치 sparse 임베딩.
    pub async fn embed_sparse_batch(&self, texts: &[String]) -> Result<Vec<SparseVector>> {
        let model = self.model.clone();
        let texts = texts.to_vec();
        tokio::task::spawn_blocking(move || -> Result<Vec<SparseVector>> {
            let mut guard = model.blocking_lock();
            let raws = guard.embed(texts, None).context("sparse embed_batch 실패")?;
            Ok(raws.into_iter().map(|r| SparseVector {
                indices: r.indices.into_iter().map(|i| i as u32).collect(),
                values: r.values,
            }).collect())
        })
        .await
        .context("spawn_blocking 합류 실패")?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sparse_vector_dot_product() {
        let a = SparseVector { indices: vec![1, 3, 5], values: vec![0.5, 0.3, 0.2] };
        let b = SparseVector { indices: vec![1, 5], values: vec![0.4, 0.6] };
        // 인덱스 1: 0.5*0.4 = 0.2, 인덱스 5: 0.2*0.6 = 0.12 → 합 0.32
        let score = a.dot(&b);
        assert!((score - 0.32).abs() < 1e-5);
    }

    #[test]
    fn sparse_vector_dot_no_overlap() {
        let a = SparseVector { indices: vec![1, 2], values: vec![1.0, 1.0] };
        let b = SparseVector { indices: vec![3, 4], values: vec![1.0, 1.0] };
        assert_eq!(a.dot(&b), 0.0);
    }

    #[test]
    fn sparse_vector_empty() {
        let v = SparseVector { indices: vec![], values: vec![] };
        assert!(v.is_empty());
        assert_eq!(v.len(), 0);
    }
}
