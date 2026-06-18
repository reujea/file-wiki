use std::path::Path;

use anyhow::Result;
use async_trait::async_trait;
use file_pipeline_core::domain::models::{
    ClassifyAndProcessResult, DocTypeRegistry, DuplicateAction, EnrichResult, Metadata,
};
use file_pipeline_core::ports::input::{DuplicateResolutionPort, SensitiveNotificationPort};
use file_pipeline_core::ports::output::{EmbeddingPort, LLMPort, PreprocessPort};

/// Stub LLM 어댑터 (Claude CLI로 교체 예정)
pub struct StubLlm;

#[async_trait]
impl LLMPort for StubLlm {
    async fn classify_and_process(
        &self,
        file_path: &Path,
        _registry: &DocTypeRegistry,
    ) -> Result<ClassifyAndProcessResult> {
        let content = std::fs::read_to_string(file_path).unwrap_or_default();
        let filename = file_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let metadata = Metadata {
            doc_types: vec!["etc".into()],
            rationale: "stub: 자동 분류 미구현".into(),
            date: chrono::Local::now().format("%Y-%m-%d").to_string(),
            summary: format!("stub 메타데이터: {}", filename),
            keywords: vec![filename],
            sensitive: false,
            doi: None,
            related_docs: vec![], source_doc_ids: vec![], search_hints: vec![],
            entities: vec![],
            ..Default::default()
        };

        Ok(ClassifyAndProcessResult {
            doc_types: vec!["etc".into()],
            rationale: "stub: 자동 분류 미구현".into(),
            content: format!("[stub 가공] length={}\n{}", content.len(), content),
            metadata,
            sections: None,
        })
    }

    async fn summarize_text(&self, new_content: &str, existing: &str) -> Result<String> {
        Ok(format!("{}\n\n--- 추가 ---\n{}", existing, new_content))
    }

    async fn enrich_existing(
        &self,
        existing_content: &str,
        _new_info: &str,
        _doc_types: &[String],
    ) -> Result<EnrichResult> {
        Ok(EnrichResult {
            updated_content: existing_content.to_string(),
            change_summary: "stub: ��강 미수행".into(),
            should_update: false,
        })
    }
}

/// Stub 임베딩 어댑터
pub struct StubEmbedder {
    dim: usize,
}

impl StubEmbedder {
    pub fn new(dim: usize) -> Self {
        Self { dim }
    }
}

#[async_trait]
impl EmbeddingPort for StubEmbedder {
    fn dim(&self) -> usize {
        self.dim
    }

    async fn embed(&self, _text: &str) -> Result<Vec<f32>> {
        Ok(vec![0.0; self.dim])
    }

    async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        Ok(texts.iter().map(|_| vec![0.0; self.dim]).collect())
    }
}

/// 플레인텍스트 전처리 (기존 동작 호환)
pub struct PlainTextPreprocessor;

impl PreprocessPort for PlainTextPreprocessor {
    fn preprocess(&self, file_path: &std::path::Path) -> anyhow::Result<file_pipeline_core::domain::models::PreprocessResult> {
        let text = std::fs::read_to_string(file_path).unwrap_or_default();
        Ok(file_pipeline_core::domain::models::PreprocessResult { text, images: vec![], tables: vec![] })
    }

    fn supports(&self, _extension: &str) -> bool {
        true
    }
}

/// 비대화형 중복 해결 — 둘 다 유지 (Keep)
///
/// GUI/watcher 등 사용자에게 물어볼 수 없는 환경에서 사용.
/// 의미 중복이 감지되어도 신규 문서를 DB에 등록한다.
pub struct StubDuplicateResolution;

#[async_trait]
impl DuplicateResolutionPort for StubDuplicateResolution {
    async fn resolve(
        &self,
        _new_path: &Path,
        _existing_path: &Path,
        diff_rendered: &str,
        reason: &str,
    ) -> Result<DuplicateAction> {
        tracing::info!("[stub] 중복 감지 — 둘 다 유지. 이유: {}", reason);
        tracing::debug!("[stub] diff: {}", diff_rendered);
        Ok(DuplicateAction::Keep)
    }

    async fn collect_manual_merge(&self, _path_a: &Path, _path_b: &Path) -> Result<String> {
        Ok(String::new())
    }
}

/// 터미널 기반 민감 파일 알림 (항상 None 반환하는 stub)
pub struct StubSensitiveNotification;

#[async_trait]
impl SensitiveNotificationPort for StubSensitiveNotification {
    async fn notify_and_collect(
        &self,
        file_path: &Path,
        reason: &str,
    ) -> Result<Option<Metadata>> {
        tracing::warn!(
            "[stub] 민감 파일 → sensitive/ 이동: {:?} ({})",
            file_path,
            reason
        );
        // 기본 메타데이터를 반환하여 sensitive/ 폴더로 이동 + 색인 처리
        let filename = file_path.file_name().unwrap_or_default().to_string_lossy().to_string();
        Ok(Some(Metadata {
            doc_types: vec!["sensitive".into()],
            rationale: format!("민감 파일: {}", reason),
            date: chrono::Local::now().format("%Y-%m-%d").to_string(),
            summary: format!("민감 파일 ({})", filename),
            keywords: vec![filename],
            sensitive: true,
            doi: None,
            related_docs: vec![], source_doc_ids: vec![], search_hints: vec![],
            entities: vec![],
            ..Default::default()
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn write_temp(content: &str) -> NamedTempFile {
        let mut f = NamedTempFile::new().expect("temp file");
        f.write_all(content.as_bytes()).expect("write");
        f
    }

    #[tokio::test]
    async fn test_stub_llm_classify() {
        let llm = StubLlm;
        let f = write_temp("hello world");
        let registry = DocTypeRegistry::empty();
        let r = llm.classify_and_process(f.path(), &registry).await.expect("classify");
        assert_eq!(r.doc_types, vec!["etc".to_string()]);
        assert!(r.content.contains("[stub 가공]"));
        assert!(r.content.contains("hello world"));
        assert_eq!(r.metadata.doc_types, vec!["etc".to_string()]);
        assert!(!r.metadata.sensitive);
    }

    #[tokio::test]
    async fn test_stub_llm_summarize_concatenates() {
        let llm = StubLlm;
        let r = llm.summarize_text("새 내용", "기존 내용").await.expect("summarize");
        assert!(r.contains("기존 내용"));
        assert!(r.contains("새 내용"));
    }

    #[tokio::test]
    async fn test_stub_llm_enrich_does_not_update() {
        let llm = StubLlm;
        let r = llm.enrich_existing("원문", "새 정보", &["report".into()]).await.expect("enrich");
        assert!(!r.should_update);
        assert_eq!(r.updated_content, "원문");
    }

    #[tokio::test]
    async fn test_stub_embedder_dim_zero_vector() {
        let e = StubEmbedder::new(128);
        assert_eq!(e.dim(), 128);
        let v = e.embed("hello").await.expect("embed");
        assert_eq!(v.len(), 128);
        assert!(v.iter().all(|&x| x == 0.0));
    }

    #[tokio::test]
    async fn test_stub_embedder_batch_size() {
        let e = StubEmbedder::new(64);
        let texts = vec!["a".into(), "b".into(), "c".into()];
        let vs = e.embed_batch(&texts).await.expect("batch");
        assert_eq!(vs.len(), 3);
        assert!(vs.iter().all(|v| v.len() == 64));
    }

    #[test]
    fn test_plaintext_preprocessor_supports_anything() {
        let p = PlainTextPreprocessor;
        assert!(p.supports("txt"));
        assert!(p.supports("md"));
        assert!(p.supports("anything"));
    }

    #[test]
    fn test_plaintext_preprocessor_reads_file() {
        let p = PlainTextPreprocessor;
        let f = write_temp("plain text content");
        let r = p.preprocess(f.path()).expect("preprocess");
        assert_eq!(r.text, "plain text content");
        assert!(r.images.is_empty());
        assert!(r.tables.is_empty());
    }

    #[tokio::test]
    async fn test_stub_duplicate_resolution_keeps() {
        let r = StubDuplicateResolution;
        let action = r.resolve(
            &PathBuf::from("a.md"), &PathBuf::from("b.md"),
            "diff", "동일 의미"
        ).await.expect("resolve");
        assert!(matches!(action, DuplicateAction::Keep));
    }

    #[tokio::test]
    async fn test_stub_sensitive_notification_returns_metadata() {
        let n = StubSensitiveNotification;
        let f = write_temp("password=secret");
        let result = n.notify_and_collect(f.path(), "API 키 노출").await.expect("notify");
        let metadata = result.expect("metadata");
        assert!(metadata.sensitive);
        assert_eq!(metadata.doc_types, vec!["sensitive".to_string()]);
        assert!(metadata.rationale.contains("API 키 노출"));
    }

    use std::path::PathBuf;
}

// step-o2 partial 해소 (2026-06-17, outbound-umbrella-1): stub OutboundManifest 박힘
impl file_pipeline_core::ports::outbound::OutboundManifest for StubLlm {
    fn id(&self) -> &str { "fp-outbound-llm-stub" }
    fn category(&self) -> file_pipeline_core::ports::outbound::OutboundCategory {
        file_pipeline_core::ports::outbound::OutboundCategory::Llm
    }
    fn capabilities(&self) -> file_pipeline_core::ports::output::ResourceCapabilities {
        file_pipeline_core::ports::output::ResourceCapabilities::standard("stub")
    }
}

impl file_pipeline_core::ports::outbound::OutboundManifest for StubEmbedder {
    fn id(&self) -> &str { "fp-outbound-embedding-stub" }
    fn category(&self) -> file_pipeline_core::ports::outbound::OutboundCategory {
        file_pipeline_core::ports::outbound::OutboundCategory::Embedding
    }
    fn capabilities(&self) -> file_pipeline_core::ports::output::ResourceCapabilities {
        file_pipeline_core::ports::output::ResourceCapabilities::standard("stub")
    }
}
