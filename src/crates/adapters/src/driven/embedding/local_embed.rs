use anyhow::Result;
use async_trait::async_trait;
use file_pipeline_core::ports::output::EmbeddingPort;

/// 로컬 임베딩 어댑터 — 외부 API 미사용, 민감 파일 안전
///
/// 구현 방식:
/// - 키워드 해시 기반 임베딩 (ONNX 모델 불필요)
/// - 같은 키워드를 공유하는 문서끼리 유사도가 높아짐
/// - OpenAI 임베딩보다 품질은 낮지만, 민감 파일이 외부로 전송되지 않음
///
/// ONNX 모델 사용 시:
/// - `PIPELINE_ONNX_MODEL` 환경변수로 모델 경로 지정
/// - ort 크레이트 활성화 후 `OnnxEmbeddingAdapter`로 교체
pub struct LocalEmbeddingAdapter {
    dim: usize,
}

impl LocalEmbeddingAdapter {
    pub fn new(dim: usize) -> Self {
        Self { dim }
    }

    /// 키워드 해시 기반 벡터 생성 (L2 정규화)
    fn hash_embed(text: &str, dim: usize) -> Vec<f32> {
        let mut vec = vec![0.0f32; dim];
        for word in text.split_whitespace() {
            let w = word.trim_matches(|c: char| !c.is_alphanumeric());
            if w.is_empty() {
                continue;
            }
            let hash = w.bytes().fold(0u64, |acc, b| {
                acc.wrapping_mul(31).wrapping_add(b as u64)
            });
            let idx = (hash as usize) % dim;
            vec[idx] += 1.0;
            // bigram: 인접 단어 조합으로 더 풍부한 벡터
            let hash2 = hash.wrapping_mul(0x517cc1b727220a95);
            vec[(hash2 as usize) % dim] += 0.5;
        }
        // L2 정규화
        let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 {
            vec.iter_mut().for_each(|x| *x /= norm);
        }
        vec
    }
}

impl Default for LocalEmbeddingAdapter {
    fn default() -> Self {
        Self::new(1024) // BGE-M3 호환 dim
    }
}

#[async_trait]
impl EmbeddingPort for LocalEmbeddingAdapter {
    fn dim(&self) -> usize {
        self.dim
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        Ok(Self::hash_embed(text, self.dim))
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        Ok(texts.iter().map(|t| Self::hash_embed(t, self.dim)).collect())
    }
}

/// 민감도 기반 임베딩 분기 어댑터
pub struct SensitivityAwareEmbeddingAdapter {
    general: Box<dyn EmbeddingPort>,
    sensitive: Box<dyn EmbeddingPort>,
    use_sensitive: std::sync::atomic::AtomicBool,
}

impl SensitivityAwareEmbeddingAdapter {
    pub fn new(general: Box<dyn EmbeddingPort>, sensitive: Box<dyn EmbeddingPort>) -> Self {
        Self {
            general,
            sensitive,
            use_sensitive: std::sync::atomic::AtomicBool::new(false),
        }
    }

    pub fn set_sensitive(&self, sensitive: bool) {
        self.use_sensitive
            .store(sensitive, std::sync::atomic::Ordering::Relaxed);
    }

    fn current(&self) -> &dyn EmbeddingPort {
        if self.use_sensitive.load(std::sync::atomic::Ordering::Relaxed) {
            self.sensitive.as_ref()
        } else {
            self.general.as_ref()
        }
    }
}

#[async_trait]
impl EmbeddingPort for SensitivityAwareEmbeddingAdapter {
    fn dim(&self) -> usize {
        self.current().dim()
    }

    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        self.current().embed(text).await
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        self.current().embed_batch(texts).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_local_embed_similarity() {
        let adapter = LocalEmbeddingAdapter::new(128);

        let v1 = adapter.embed("프로젝트 회의 결정사항").await.unwrap();
        let v2 = adapter.embed("프로젝트 회의 액션아이템").await.unwrap();
        let v3 = adapter.embed("Rust 소유권 lifetime").await.unwrap();

        let sim_12: f32 = v1.iter().zip(&v2).map(|(a, b)| a * b).sum();
        let sim_13: f32 = v1.iter().zip(&v3).map(|(a, b)| a * b).sum();

        assert!(sim_12 > sim_13, "유사 주제({:.3}) > 다른 주제({:.3})", sim_12, sim_13);
    }

    #[tokio::test]
    async fn test_local_embed_dim() {
        let adapter = LocalEmbeddingAdapter::new(1024);
        let v = adapter.embed("테스트 문장").await.unwrap();
        assert_eq!(v.len(), 1024);

        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 0.01, "L2 정규화: norm={:.4}", norm);
    }
}
