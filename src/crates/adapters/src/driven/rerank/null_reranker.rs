use anyhow::Result;
use async_trait::async_trait;
use file_pipeline_core::domain::models::SimilarDoc;
use file_pipeline_core::ports::output::RerankerPort;

/// 리랭킹을 수행하지 않는 패스스루 어댑터
pub struct NullReranker;

#[async_trait]
impl RerankerPort for NullReranker {
    async fn rerank(&self, _query: &str, candidates: Vec<SimilarDoc>) -> Result<Vec<SimilarDoc>> {
        Ok(candidates)
    }
    fn is_enabled(&self) -> bool { false }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_null_reranker_passthrough() {
        let reranker = NullReranker;
        let candidates = vec![
            SimilarDoc { id: "a".into(), score: 0.9, path: PathBuf::from("a.zst"), doc_types: vec![], date: "".into(), hierarchy: vec![] },
            SimilarDoc { id: "b".into(), score: 0.5, path: PathBuf::from("b.zst"), doc_types: vec![], date: "".into(), hierarchy: vec![] },
        ];
        let result = reranker.rerank("query", candidates.clone()).await.expect("rerank");
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id, "a");
        assert_eq!(result[1].id, "b");
    }

    #[tokio::test]
    async fn test_null_reranker_empty() {
        let reranker = NullReranker;
        let result = reranker.rerank("query", vec![]).await.expect("rerank");
        assert!(result.is_empty());
    }

    #[test]
    fn test_null_reranker_disabled() {
        let reranker = NullReranker;
        assert!(!reranker.is_enabled());
    }
}

// step-o2 (2026-06-16, outbound-umbrella-1): OutboundManifest 박힘
impl file_pipeline_core::ports::outbound::OutboundManifest for NullReranker {
    fn id(&self) -> &str { "fp-outbound-rerank-null" }
    fn category(&self) -> file_pipeline_core::ports::outbound::OutboundCategory {
        file_pipeline_core::ports::outbound::OutboundCategory::Rerank
    }
    fn capabilities(&self) -> file_pipeline_core::ports::output::ResourceCapabilities {
        file_pipeline_core::ports::output::ResourceCapabilities::standard("null")
    }
}
