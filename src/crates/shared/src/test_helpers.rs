//! 통합 테스트용 FileProcessingService 빌더
//!
//! lesson 21 + 27 재발 차단. 핵심 도메인 구조체 필드 추가 시 통합 테스트 12+ 파일을
//! 일일이 수정해야 했던 문제를 해소.
//!
//! ## 사용
//!
//! ```rust,ignore
//! use file_pipeline_shared::test_helpers::ServiceBuilder;
//!
//! let base = tempfile::TempDir::new().unwrap();
//! let service = ServiceBuilder::new(base.path()).build();
//! ```
//!
//! 커스텀 어댑터가 필요한 테스트:
//!
//! ```rust,ignore
//! let service = ServiceBuilder::new(base.path())
//!     .with_llm(Arc::new(MyCustomLlm))
//!     .with_embedding(Arc::new(MyEmbedder))
//!     .with_crossref_threshold(0.8)
//!     .build();
//! ```
//!
//! 신규 필드 추가 시 본 빌더에 with_* 메서드만 추가하면 기존 테스트는 변경 0건.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use file_pipeline_adapters::driven::notification::composite::NullNotificationAdapter;
use file_pipeline_adapters::driven::storage::remote_null::NullRemoteStorage;
use file_pipeline_adapters::driven::storage::zstd_storage::ZstdStorageAdapter;
use file_pipeline_adapters::driven::vector_db::local_store::LocalVectorStore;
use file_pipeline_adapters::stub::{
    StubDuplicateResolution, StubEmbedder, StubLlm, StubSensitiveNotification, PlainTextPreprocessor,
};
use file_pipeline_core::domain::classifier::SensitivityDetector;
use file_pipeline_core::domain::error_log::ErrorLog;
use file_pipeline_core::domain::incremental::CompileState;
use file_pipeline_core::domain::models::DocTypeRegistry;
use file_pipeline_core::domain::verification::VerificationThresholds;
use file_pipeline_core::ports::input::{DuplicateResolutionPort, SensitiveNotificationPort};
use file_pipeline_core::ports::output::{
    EmbeddingPort, LLMPort, NotificationPort, PreprocessPort,
    RemoteStoragePort, StoragePort, VectorDBPort, VerificationPort, ProcessingMetricsPort,
};
use file_pipeline_core::service::FileProcessingService;

/// 통합 테스트용 FileProcessingService 빌더
///
/// 모든 필드를 안전한 기본값으로 초기화. 필요한 어댑터만 with_*로 교체.
/// 새 도메인 필드가 추가되면 본 빌더의 build()만 수정하면 모든 테스트가 자동 호환.
pub struct ServiceBuilder {
    base: PathBuf,
    llm: Option<Arc<dyn LLMPort>>,
    storage: Option<Arc<dyn StoragePort>>,
    vector_db: Option<Arc<dyn VectorDBPort>>,
    embedding: Option<Arc<dyn EmbeddingPort>>,
    notification: Option<Arc<dyn NotificationPort>>,
    verification: Option<Arc<dyn VerificationPort>>,
    preprocessing: Option<Arc<dyn PreprocessPort>>,
    remote_storage: Option<Arc<dyn RemoteStoragePort>>,
    duplicate_resolution: Option<Arc<dyn DuplicateResolutionPort>>,
    sensitive_notification: Option<Arc<dyn SensitiveNotificationPort>>,
    registry: Option<Arc<DocTypeRegistry>>,
    // 도메인 스칼라 — 빈번한 커스터마이즈 대상
    semantic_dup_threshold: f32,
    max_retry: u32,
    verification_enabled: bool,
    fragment_threshold: usize,
    crossref_threshold: f32,
    crossref_supersedes_threshold: f32,
    crossref_keyword_overlap_min: usize,
    crossref_top_k: usize,
    crossref_cap_supersedes: usize,
    crossref_cap_updates: usize,
    crossref_cap_related: usize,
    crossref_cap_references: usize,
    crossref_cap_incoming: usize,
    crossref_minhash_force: bool,
    crossref_minhash_min_docs: usize,
    crossref_metadata_blocking: bool,
    crossref_mode: String,
    crossref_interval_secs: u64,
    embed_instruction_prefix: Option<String>,
    global_thresholds: Option<VerificationThresholds>,
    metrics_recorder: Option<Arc<dyn ProcessingMetricsPort>>,
    /// Phase 202 B2: plugin IPC registry (디폴트 빈 레지스트리, lesson 14 회피).
    plugin_registry: Option<Arc<file_pipeline_core::plugin::PluginRegistry>>,
}

impl ServiceBuilder {
    pub fn new(base: &Path) -> Self {
        Self {
            base: base.to_path_buf(),
            llm: None,
            storage: None,
            vector_db: None,
            embedding: None,
            notification: None,
            verification: None,
            preprocessing: None,
            remote_storage: None,
            duplicate_resolution: None,
            sensitive_notification: None,
            registry: None,
            semantic_dup_threshold: 0.0001,
            max_retry: 1,
            verification_enabled: false,
            fragment_threshold: 50,
            crossref_threshold: 0.7,
            crossref_supersedes_threshold: 0.95,
            crossref_keyword_overlap_min: 3,
            crossref_top_k: 3,
            crossref_cap_supersedes: 2,
            crossref_cap_updates: 5,
            crossref_cap_related: 20,
            crossref_cap_references: 10,
            crossref_cap_incoming: 0,
            crossref_minhash_force: false,
            crossref_minhash_min_docs: 3_000,
            crossref_metadata_blocking: false,
            crossref_mode: "auto".to_string(),
            crossref_interval_secs: 0,
            embed_instruction_prefix: None,
            global_thresholds: None,
            metrics_recorder: None,
            plugin_registry: None,
        }
    }

    pub fn with_llm(mut self, llm: Arc<dyn LLMPort>) -> Self { self.llm = Some(llm); self }
    pub fn with_storage(mut self, s: Arc<dyn StoragePort>) -> Self { self.storage = Some(s); self }
    pub fn with_vector_db(mut self, v: Arc<dyn VectorDBPort>) -> Self { self.vector_db = Some(v); self }
    pub fn with_embedding(mut self, e: Arc<dyn EmbeddingPort>) -> Self { self.embedding = Some(e); self }
    pub fn with_notification(mut self, n: Arc<dyn NotificationPort>) -> Self { self.notification = Some(n); self }
    pub fn with_verification(mut self, v: Arc<dyn VerificationPort>) -> Self { self.verification = Some(v); self }
    pub fn with_preprocessing(mut self, p: Arc<dyn PreprocessPort>) -> Self { self.preprocessing = Some(p); self }
    pub fn with_remote_storage(mut self, r: Arc<dyn RemoteStoragePort>) -> Self { self.remote_storage = Some(r); self }
    pub fn with_duplicate_resolution(mut self, d: Arc<dyn DuplicateResolutionPort>) -> Self { self.duplicate_resolution = Some(d); self }
    pub fn with_sensitive_notification(mut self, s: Arc<dyn SensitiveNotificationPort>) -> Self { self.sensitive_notification = Some(s); self }
    pub fn with_registry(mut self, r: Arc<DocTypeRegistry>) -> Self { self.registry = Some(r); self }
    pub fn with_semantic_dup_threshold(mut self, t: f32) -> Self { self.semantic_dup_threshold = t; self }
    pub fn with_max_retry(mut self, n: u32) -> Self { self.max_retry = n; self }
    pub fn with_verification_enabled(mut self, e: bool) -> Self { self.verification_enabled = e; self }
    pub fn with_fragment_threshold(mut self, n: usize) -> Self { self.fragment_threshold = n; self }
    pub fn with_crossref_threshold(mut self, t: f32) -> Self { self.crossref_threshold = t; self }
    pub fn with_crossref_supersedes_threshold(mut self, t: f32) -> Self { self.crossref_supersedes_threshold = t; self }
    pub fn with_crossref_keyword_overlap_min(mut self, n: usize) -> Self { self.crossref_keyword_overlap_min = n; self }
    pub fn with_crossref_top_k(mut self, n: usize) -> Self { self.crossref_top_k = n; self }
    pub fn with_crossref_caps(mut self, supersedes: usize, updates: usize, related: usize, references: usize, incoming: usize) -> Self {
        self.crossref_cap_supersedes = supersedes;
        self.crossref_cap_updates = updates;
        self.crossref_cap_related = related;
        self.crossref_cap_references = references;
        self.crossref_cap_incoming = incoming;
        self
    }
    pub fn with_minhash(mut self, force: bool, min_docs: usize) -> Self {
        self.crossref_minhash_force = force;
        self.crossref_minhash_min_docs = min_docs;
        self
    }
    pub fn with_metadata_blocking(mut self, on: bool) -> Self { self.crossref_metadata_blocking = on; self }
    pub fn with_crossref_mode(mut self, m: &str) -> Self { self.crossref_mode = m.to_string(); self }
    pub fn with_crossref_interval(mut self, secs: u64) -> Self { self.crossref_interval_secs = secs; self }
    pub fn with_embed_instruction_prefix(mut self, p: String) -> Self { self.embed_instruction_prefix = Some(p); self }
    pub fn with_global_thresholds(mut self, t: VerificationThresholds) -> Self { self.global_thresholds = Some(t); self }
    pub fn with_metrics_recorder(mut self, m: Arc<dyn ProcessingMetricsPort>) -> Self {
        self.metrics_recorder = Some(m);
        self
    }

    /// Phase 202 B2: plugin IPC registry 명시 주입 (테스트용).
    /// 미주입 시 build()에서 빈 `PluginRegistry::new()` 디폴트 (lesson 14 회피).
    pub fn with_plugin_registry(
        mut self,
        r: Arc<file_pipeline_core::plugin::PluginRegistry>,
    ) -> Self {
        self.plugin_registry = Some(r);
        self
    }

    /// 디렉토리를 생성하고 모든 미주입 필드를 stub으로 채워 FileProcessingService 반환.
    /// 호출자가 with_* 를 부르지 않은 어댑터는 다음 기본값 사용:
    /// - LLM: StubLlm
    /// - Storage: ZstdStorageAdapter(level=3)
    /// - VectorDB: LocalVectorStore (with_path)
    /// - Embedding: StubEmbedder
    /// - Preprocess: PlainTextPreprocessor
    /// - Notification: NullNotificationAdapter
    /// - RemoteStorage: NullRemoteStorage
    /// - Verification: None
    /// - DuplicateResolution: StubDuplicateResolution
    /// - SensitiveNotification: StubSensitiveNotification
    /// - DocTypeRegistry: 빈 레지스트리
    pub fn build(self) -> FileProcessingService {
        let inbox = self.base.join("inbox");
        let processed = self.base.join("processed");
        let originals = self.base.join("originals");
        let sensitive = self.base.join("sensitive");
        let todo = self.base.join("todo");
        let quarantine = self.base.join("quarantine");
        let temp = self.base.join(".tmp");
        for d in [&inbox, &processed, &originals, &sensitive, &todo, &quarantine, &temp] {
            let _ = std::fs::create_dir_all(d);
        }

        let storage: Arc<dyn StoragePort> = self.storage
            .unwrap_or_else(|| Arc::new(ZstdStorageAdapter::new(3, temp.clone())));
        let vector_db: Arc<dyn VectorDBPort> = self.vector_db.unwrap_or_else(|| {
            let v = Arc::new(LocalVectorStore::with_path(self.base.join(".local-store.json")));
            v.init().expect("vector_db init");
            v
        });
        let llm: Arc<dyn LLMPort> = self.llm.unwrap_or_else(|| Arc::new(StubLlm));
        let embedding: Arc<dyn EmbeddingPort> = self.embedding.unwrap_or_else(|| Arc::new(StubEmbedder::new(128)));
        let notification: Arc<dyn NotificationPort> = self.notification.unwrap_or_else(|| Arc::new(NullNotificationAdapter));
        let preprocessing: Arc<dyn PreprocessPort> = self.preprocessing.unwrap_or_else(|| Arc::new(PlainTextPreprocessor));
        let remote_storage: Arc<dyn RemoteStoragePort> = self.remote_storage.unwrap_or_else(|| Arc::new(NullRemoteStorage));
        let duplicate_resolution: Arc<dyn DuplicateResolutionPort> = self.duplicate_resolution.unwrap_or_else(|| Arc::new(StubDuplicateResolution));
        let sensitive_notification: Arc<dyn SensitiveNotificationPort> = self.sensitive_notification.unwrap_or_else(|| Arc::new(StubSensitiveNotification));
        let registry: Arc<DocTypeRegistry> = self.registry.unwrap_or_else(|| Arc::new(DocTypeRegistry::new(vec![])));

        // Phase 94 A3: 디폴트 NullAuditAdapter (lesson 14 회피)
        let audit: std::sync::Arc<dyn file_pipeline_core::ports::output::AuditPort> =
            std::sync::Arc::new(file_pipeline_core::ports::output::NullAuditAdapter);

        FileProcessingService {
            llm, storage, vector_db, embedding, notification,
            verification: self.verification,
            preprocessing, remote_storage, audit, duplicate_resolution, sensitive_notification,
            registry,
            sensitivity_detector: SensitivityDetector::default(),
            pii_user_patterns: std::sync::RwLock::new(Vec::new()),
            inbox_dir: inbox,
            processed_dir: processed,
            originals_dir: originals,
            sensitive_dir: sensitive,
            todo_dir: todo,
            semantic_dup_threshold: self.semantic_dup_threshold,
            max_retry: self.max_retry,
            quarantine_dir: quarantine,
            compile_state: std::sync::Mutex::new(CompileState::new()),
            compile_state_path: temp.join(".compile-state.json"),
            compile_state_batch: std::sync::atomic::AtomicBool::new(false),
            global_thresholds: self.global_thresholds,
            verification_enabled: self.verification_enabled,
            summary: std::sync::Mutex::new(Default::default()),
            progress_callback: None,
            error_log: std::sync::Mutex::new(ErrorLog::new()),
            fragment_threshold: self.fragment_threshold,
            crossref_mode: self.crossref_mode,
            crossref_similarity_threshold: self.crossref_threshold,
            crossref_supersedes_threshold: self.crossref_supersedes_threshold,
            crossref_keyword_overlap_min: self.crossref_keyword_overlap_min,
            crossref_top_k: self.crossref_top_k,
            crossref_cap_supersedes: self.crossref_cap_supersedes,
            crossref_cap_updates: self.crossref_cap_updates,
            crossref_cap_related: self.crossref_cap_related,
            crossref_cap_references: self.crossref_cap_references,
            crossref_cap_incoming: self.crossref_cap_incoming,
            crossref_minhash_force: self.crossref_minhash_force,
            crossref_minhash_min_docs: self.crossref_minhash_min_docs,
            crossref_metadata_blocking: self.crossref_metadata_blocking,
            token_usage: std::sync::Mutex::new(Default::default()),
            embed_instruction_prefix: self.embed_instruction_prefix,
            crossref_queue: std::sync::Mutex::new(Vec::new()),
            crossref_last_run: std::sync::Mutex::new(None),
            crossref_interval_secs: self.crossref_interval_secs,
            metrics_recorder: self.metrics_recorder,
            plugin_registry: self.plugin_registry.unwrap_or_else(|| {
                Arc::new(file_pipeline_core::plugin::PluginRegistry::new())
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn builder_creates_service_with_defaults() {
        let base = tempfile::TempDir::new().unwrap();
        let svc = ServiceBuilder::new(base.path()).build();
        assert_eq!(svc.crossref_similarity_threshold, 0.7);
        assert_eq!(svc.crossref_interval_secs, 0);
        assert!(svc.metrics_recorder.is_none());
    }

    #[tokio::test]
    async fn builder_default_plugin_registry_is_empty() {
        let base = tempfile::TempDir::new().unwrap();
        let svc = ServiceBuilder::new(base.path()).build();
        assert_eq!(svc.plugin_registry.count(), 0);
    }

    #[tokio::test]
    async fn builder_overrides_take_effect() {
        let base = tempfile::TempDir::new().unwrap();
        let svc = ServiceBuilder::new(base.path())
            .with_crossref_threshold(0.85)
            .with_crossref_interval(30)
            .build();
        assert!((svc.crossref_similarity_threshold - 0.85).abs() < 1e-6);
        assert_eq!(svc.crossref_interval_secs, 30);
    }
}
