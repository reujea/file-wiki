use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};

use crate::domain::classifier::SensitivityDetector;
use crate::domain::incremental::CompileState;
use crate::domain::models::{DocTypeRegistry, ProcessingSummary};
// verify_with_thresholds를 직접 호출 (유형별 임계값)
use crate::ports::input::{DuplicateResolutionPort, SensitiveNotificationPort};
use crate::ports::output::{
    AuditPort, EmbeddingPort, LLMPort, NotificationPort, PreprocessPort,
    ProcessingMetricsPort, RemoteStoragePort, StoragePort, VectorDBPort, VerificationPort,
};

/// 진행률 이벤트 콜백 타입. JSON 문자열 한 줄을 전달받아 로그/UI로 라우팅한다.
pub type ProgressCallback = Arc<dyn Fn(&str) + Send + Sync>;

/// cosine similarity (인라인, flush_crossref_matrix 용)
#[inline]
pub(crate) fn cosine_sim_inline(a: &[f32], b: &[f32]) -> f32 {
    let mut dot = 0.0f32;
    let mut na = 0.0f32;
    let mut nb = 0.0f32;
    for i in 0..a.len() {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    dot / (na.sqrt() * nb.sqrt() + 1e-10)
}

/// 메타데이터 블로킹: doc_type 또는 키워드 1개 이상 겹쳐야 통과
#[inline]
pub(crate) fn meta_block_pass(
    item: &CrossRefQueueItem,
    cand_id: &str,
    doc_meta: &std::collections::HashMap<&str, &crate::domain::models::StoredDocSummary>,
    kw_snapshot: &std::collections::HashMap<String, Vec<String>>,
    item_kw_set: &std::collections::HashSet<&String>,
) -> bool {
    let cand = match doc_meta.get(cand_id) { Some(c) => c, None => return true };
    if cand.doc_types.iter().any(|t| item.doc_types.contains(t)) {
        return true;
    }
    if let Some(cand_kw) = kw_snapshot.get(cand_id) {
        if cand_kw.iter().any(|k| item_kw_set.contains(k)) {
            return true;
        }
    }
    false
}

/// 파일 처리 파이프라인 서비스 (헥사고날 코어)
pub struct FileProcessingService {
    // Driven 포트
    pub llm: Arc<dyn LLMPort>,
    pub storage: Arc<dyn StoragePort>,
    pub vector_db: Arc<dyn VectorDBPort>,
    pub embedding: Arc<dyn EmbeddingPort>,
    pub notification: Arc<dyn NotificationPort>,
    pub verification: Option<Arc<dyn VerificationPort>>,
    pub preprocessing: Arc<dyn PreprocessPort>,
    pub remote_storage: Arc<dyn RemoteStoragePort>,
    /// Phase 94 A3: audit_trace 기록. 디폴트 NullAuditAdapter (no-op, lesson 14 회피).
    pub audit: Arc<dyn AuditPort>,

    // Driving 포트
    pub duplicate_resolution: Arc<dyn DuplicateResolutionPort>,
    pub sensitive_notification: Arc<dyn SensitiveNotificationPort>,

    // 도메인
    pub registry: Arc<DocTypeRegistry>,
    pub sensitivity_detector: SensitivityDetector,
    /// Ruflo C2: 사용자 정의 PII 패턴 (name, regex). build_service에서 settings.db로부터 주입.
    /// RwLock으로 live reload 지원 — reload_pii_patterns()로 재주입 가능 (재시작 불필요).
    pub pii_user_patterns: std::sync::RwLock<Vec<(String, String)>>,

    // 경로 설정
    pub inbox_dir: PathBuf,
    pub processed_dir: PathBuf,
    pub originals_dir: PathBuf,
    pub sensitive_dir: PathBuf,
    pub todo_dir: PathBuf,

    // 설정
    pub semantic_dup_threshold: f32,
    pub max_retry: u32,
    pub quarantine_dir: PathBuf,
    /// pipeline.toml [verification.thresholds] 글로벌 오버라이드
    pub global_thresholds: Option<crate::domain::verification::VerificationThresholds>,
    /// 검증 활성화 여부
    pub verification_enabled: bool,
    /// fragment 임계값 (이하 글자수는 LLM 스킵)
    pub fragment_threshold: usize,

    // 교차참조 설정
    /// "auto" (키워드/임베딩 기반) | "llm" (LLM 보강 판단) | "off" (비활성)
    pub crossref_mode: String,
    pub crossref_similarity_threshold: f32,
    pub crossref_supersedes_threshold: f32,
    pub crossref_keyword_overlap_min: usize,
    pub crossref_top_k: usize,
    // TypedSlots: 유형별 outgoing cap
    pub crossref_cap_supersedes: usize,
    pub crossref_cap_updates: usize,
    pub crossref_cap_related: usize,
    pub crossref_cap_references: usize,
    /// mutual top-K: incoming cap (0=무제한)
    pub crossref_cap_incoming: usize,
    /// MinHash LSH 강제 활성
    pub crossref_minhash_force: bool,
    /// MinHash LSH 자동 활성 최소 문서 수
    pub crossref_minhash_min_docs: usize,
    /// 메타데이터 블로킹 (doc_type 또는 키워드 겹침 필요)
    pub crossref_metadata_blocking: bool,

    // 증분 컴파일 상태
    pub compile_state: std::sync::Mutex<CompileState>,
    pub compile_state_path: PathBuf,
    /// 배치 모드 시 compile_state.save() 스킵 (batch_end에서 1회만 저장)
    pub compile_state_batch: std::sync::atomic::AtomicBool,

    // 처리 요약 (배치 알림용)
    pub summary: std::sync::Mutex<ProcessingSummary>,

    // 진행률 이벤트 콜백 (None이면 비활성)
    pub progress_callback: Option<ProgressCallback>,

    // 구조화된 에러 로그
    pub error_log: std::sync::Mutex<crate::domain::error_log::ErrorLog>,

    // 토큰 사용 추적
    pub token_usage: std::sync::Mutex<crate::domain::models::TokenUsage>,

    /// 임베딩 instruction prefix (설정 시 임베딩 입력 앞에 추가)
    pub embed_instruction_prefix: Option<String>,

    /// 교차참조 비동기 대기 큐: (doc_id, doc_types, date, keywords, embedding)
    pub crossref_queue: std::sync::Mutex<Vec<CrossRefQueueItem>>,
    /// 마지막 교차참조 배치 실행 시각
    pub crossref_last_run: std::sync::Mutex<Option<std::time::Instant>>,
    /// 교차참조 배치 간격 (초)
    pub crossref_interval_secs: u64,

    /// Phase 82-prep: 처리 메트릭 영속화 (None=비활성, summary는 메모리만)
    pub metrics_recorder: Option<Arc<dyn ProcessingMetricsPort>>,

    /// Phase 202 B2: plugin IPC 호출 게이트.
    /// `build_service`에서 `PluginRegistry::new().with_audit(audit).discover(paths.plugins)` 결과 주입.
    /// 테스트 디폴트는 빈 PluginRegistry (lesson 14 + lesson 21/27 회피).
    pub plugin_registry: Arc<crate::plugin::PluginRegistry>,
}

/// 교차참조 비동기 큐 항목
#[derive(Clone)]
pub struct CrossRefQueueItem {
    pub doc_id: String,
    pub doc_types: Vec<String>,
    pub date: String,
    pub keywords: Vec<String>,
    pub embedding: Vec<f32>,
    /// 우선순위: 0=최고(사용자 직접 투입), 1=보통(watcher), 2=낮음(배치)
    pub priority: u8,
}

impl FileProcessingService {
    // step-s4 (2026-06-16): 헬퍼는 `pub(crate)` 박힘 — `use_cases::process_file` / `use_cases::crossref` 의
    // split impl 안에서 `Self::*` 호출 가능 (Rust 의 private fn = module-scoped 정합).
    pub(crate) fn emit_progress(&self, event: &str) {
        if let Some(cb) = &self.progress_callback {
            cb(event);
        }
    }

    // ── Phase 82-prep: 메트릭 영속화 헬퍼 ─────────────────────
    pub(crate) fn metrics_success(&self) {
        if let Some(m) = &self.metrics_recorder { m.record_success(); }
    }
    pub(crate) fn metrics_error(&self) {
        if let Some(m) = &self.metrics_recorder { m.record_error(); }
    }
    pub(crate) fn metrics_quarantine(&self) {
        if let Some(m) = &self.metrics_recorder { m.record_quarantine(); }
    }
    pub(crate) fn metrics_verify(&self, passed: bool) {
        if let Some(m) = &self.metrics_recorder { m.record_verify(passed); }
    }
    pub(crate) fn metrics_time(&self, started: std::time::Instant) {
        if let Some(m) = &self.metrics_recorder {
            let ms = started.elapsed().as_millis().min(u64::MAX as u128) as u64;
            m.record_process_time(ms);
        }
    }

    pub(crate) fn compute_hash(path: &Path) -> Result<String> {
        let bytes = std::fs::read(path).context("파일 읽기 실패")?;
        let hash = Sha256::digest(&bytes);
        Ok(hex::encode(hash))
    }

    pub(crate) fn read_text(path: &Path) -> Result<String> {
        std::fs::read_to_string(path).context("텍스트 파일 읽기 실패")
    }


    /// 누적된 처리 요약을 알림으로 전송하고 초기화
    /// step-s4 (2026-06-16): `MaintenanceUseCase` 위임 (파사드 패턴).
    pub async fn flush_summary(&self) -> Result<()> {
        self.maintenance().flush_summary().await
    }

    /// step-s4 (2026-06-16): `MaintenanceUseCase` instance 생성 — 운영 함수 단일 진입점.
    fn maintenance(&self) -> crate::use_cases::maintenance::MaintenanceUseCase<'_> {
        crate::use_cases::maintenance::MaintenanceUseCase {
            notification: &self.notification,
            summary: &self.summary,
            compile_state: &self.compile_state,
            compile_state_path: &self.compile_state_path,
            compile_state_batch: &self.compile_state_batch,
            pii_user_patterns: &self.pii_user_patterns,
        }
    }


    /// compile_state 배치 모드 시작 — `MaintenanceUseCase` 위임 (step-s4 파사드).
    pub fn compile_state_batch_begin(&self) {
        self.maintenance().compile_state_batch_begin();
    }

    /// compile_state 배치 모드 종료 — `MaintenanceUseCase` 위임 (step-s4 파사드).
    pub fn compile_state_batch_end(&self) {
        self.maintenance().compile_state_batch_end();
    }

    /// PII 패턴 핫 리로드 — `MaintenanceUseCase` 위임 (step-s4 파사드).
    pub fn reload_pii_patterns(&self, patterns: Vec<(String, String)>) -> Result<usize> {
        self.maintenance().reload_pii_patterns(patterns)
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::models::*;
    use crate::domain::incremental::CompileState;
    use crate::domain::verification::VerificationThresholds;
    use crate::ports::input::{DuplicateResolutionPort, SensitiveNotificationPort};
    use crate::ports::output::*;
    use async_trait::async_trait;
    use std::sync::atomic::{AtomicU32, Ordering};
    use tempfile::TempDir;

    // ── Stub 포트 구현 ──

    struct TestLlm {
        fail_first: bool,
        call_count: AtomicU32,
    }

    impl TestLlm {
        fn new() -> Self { Self { fail_first: false, call_count: AtomicU32::new(0) } }
        fn failing_first() -> Self { Self { fail_first: true, call_count: AtomicU32::new(0) } }
    }

    #[async_trait]
    impl LLMPort for TestLlm {
        async fn classify_and_process(&self, file_path: &Path, _registry: &DocTypeRegistry) -> Result<ClassifyAndProcessResult> {
            let count = self.call_count.fetch_add(1, Ordering::SeqCst);
            if self.fail_first && count == 0 {
                anyhow::bail!("LLM 1차 실패");
            }
            let fname = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("test");
            let content = std::fs::read_to_string(file_path).unwrap_or_default();
            Ok(ClassifyAndProcessResult {
                doc_types: vec!["meeting".into()],
                rationale: "test".into(),
                content: format!("[가공] {}", content),
                metadata: Metadata {
                    doc_types: vec!["meeting".into()],
                    rationale: "test".into(),
                    date: "2026-04-14".into(),
                    summary: format!("summary of {}", fname),
                    keywords: vec!["keyword1".into()],
                    sensitive: false, doi: None,
                    related_docs: vec![], source_doc_ids: vec![], search_hints: vec![],
                    entities: vec![],
                    ..Default::default()
                },
                sections: Some(std::collections::HashMap::from([
                    ("결정사항".into(), vec!["item1".into()]),
                ])),
            })
        }
        async fn summarize_text(&self, new: &str, existing: &str) -> Result<String> {
            Ok(format!("{}\n{}", existing, new))
        }
        async fn enrich_existing(&self, existing: &str, _new: &str, _types: &[String]) -> Result<EnrichResult> {
            Ok(EnrichResult { updated_content: existing.into(), change_summary: "no change".into(), should_update: false })
        }
    }

    struct TestStorage;
    impl StoragePort for TestStorage {
        fn compress_and_store(&self, source: &Path, dest_dir: &Path) -> Result<PathBuf> {
            let dest = dest_dir.join(format!("{}.zst",
                source.file_name().unwrap_or_default().to_string_lossy()));
            std::fs::copy(source, &dest)?;
            Ok(dest)
        }
        fn decompress_temp(&self, compressed: &Path) -> Result<PathBuf> {
            Ok(compressed.to_path_buf())
        }
        fn read_header(&self, compressed: &Path, lines: usize) -> Result<String> {
            let content = std::fs::read_to_string(compressed).unwrap_or_default();
            Ok(content.lines().take(lines).collect::<Vec<_>>().join("\n"))
        }
    }

    struct TestVectorDb {
        hashes: std::sync::Mutex<Vec<String>>,
    }
    impl TestVectorDb {
        fn new() -> Self { Self { hashes: std::sync::Mutex::new(vec![]) } }
    }
    impl VectorDBPort for TestVectorDb {
        fn init(&self) -> Result<()> { Ok(()) }
        fn upsert(&self, doc: &Document) -> Result<()> {
            self.hashes.lock().expect("mutex").push(doc.file_hash.clone());
            Ok(())
        }
        fn search_similar(&self, _embedding: &[f32], _top_k: usize) -> Result<Vec<SimilarDoc>> { Ok(vec![]) }
        fn find_by_hash(&self, hash: &str) -> Result<Option<String>> {
            let hashes = self.hashes.lock().expect("mutex");
            if hashes.contains(&hash.to_string()) { Ok(Some("existing".into())) } else { Ok(None) }
        }
        fn find_by_type(&self, _doc_type: &str, _date: &str) -> Result<Option<String>> { Ok(None) }
        fn stats(&self) -> Result<DbStats> { Ok(DbStats::default()) }
        fn list_all(&self) -> Result<Vec<StoredDocSummary>> { Ok(vec![]) }
        fn get_types(&self, _doc_id: &str) -> Result<Vec<String>> { Ok(vec![]) }
        fn update_types(&self, _doc_id: &str, _types: Vec<String>) -> Result<()> { Ok(()) }
        fn link(&self, _source_id: &str, _target_id: &str, _relation: RelationType) -> Result<()> { Ok(()) }
        fn find_related(&self, _doc_id: &str) -> Result<Vec<DocRelation>> { Ok(vec![]) }
        fn update_content(&self, _doc_id: &str, _new_content: &str, _change_summary: &str) -> Result<()> { Ok(()) }
    }

    struct TestEmbedder;
    #[async_trait]
    impl EmbeddingPort for TestEmbedder {
        fn dim(&self) -> usize { 4 }
        async fn embed(&self, _text: &str) -> Result<Vec<f32>> { Ok(vec![0.1, 0.2, 0.3, 0.4]) }
        async fn embed_batch(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
            Ok(texts.iter().map(|_| vec![0.1, 0.2, 0.3, 0.4]).collect())
        }
    }

    struct TestNotification;
    #[async_trait]
    impl NotificationPort for TestNotification {
        async fn send(&self, _title: &str, _body: &str, _level: &str) -> Result<()> { Ok(()) }
        async fn send_duplicate_alert(&self, _f: &str, _r: &str, _d: &str) -> Result<()> { Ok(()) }
        async fn send_sensitive_alert(&self, _f: &str, _r: &str) -> Result<()> { Ok(()) }
        async fn send_completion(&self, _f: &str, _t: &str, _s: &DbStats) -> Result<()> { Ok(()) }
        async fn send_summary(&self, _s: &ProcessingSummary) -> Result<()> { Ok(()) }
    }

    struct TestDupResolution;
    #[async_trait]
    impl DuplicateResolutionPort for TestDupResolution {
        async fn resolve(&self, _new: &Path, _existing: &Path, _diff: &str, _reason: &str) -> Result<DuplicateAction> {
            Ok(DuplicateAction::Skip)
        }
        async fn collect_manual_merge(&self, _a: &Path, _b: &Path) -> Result<String> { Ok(String::new()) }
    }

    struct TestSensitiveNotif;
    #[async_trait]
    impl SensitiveNotificationPort for TestSensitiveNotif {
        async fn notify_and_collect(&self, _path: &Path, _reason: &str) -> Result<Option<Metadata>> {
            Ok(None)
        }
    }

    struct TestPreprocess;
    impl PreprocessPort for TestPreprocess {
        fn preprocess(&self, file_path: &Path) -> Result<PreprocessResult> {
            let text = std::fs::read_to_string(file_path).unwrap_or_default();
            Ok(PreprocessResult { text, images: vec![], tables: vec![] })
        }
        fn supports(&self, _ext: &str) -> bool { true }
    }

    struct TestNullRemoteStorage;
    #[async_trait]
    impl crate::ports::output::RemoteStoragePort for TestNullRemoteStorage {
        async fn upload(&self, _: &Path, _: &str) -> Result<()> { Ok(()) }
        async fn download(&self, _: &str, _: &Path) -> Result<()> { Ok(()) }
        async fn list(&self, _: &str) -> Result<Vec<String>> { Ok(vec![]) }
        async fn delete(&self, _: &str) -> Result<()> { Ok(()) }
        fn is_configured(&self) -> bool { false }
    }

    // step-o2 partial 해소 (2026-06-17): test mock OutboundManifest 박힘 (super-trait 의무)
    impl crate::ports::outbound::OutboundManifest for TestLlm {
        fn id(&self) -> &str { "fp-outbound-llm-test" }
        fn category(&self) -> crate::ports::outbound::OutboundCategory { crate::ports::outbound::OutboundCategory::Llm }
        fn capabilities(&self) -> crate::ports::output::ResourceCapabilities {
            crate::ports::output::ResourceCapabilities::standard("test")
        }
    }
    impl crate::ports::outbound::OutboundManifest for TestEmbedder {
        fn id(&self) -> &str { "fp-outbound-embedding-test" }
        fn category(&self) -> crate::ports::outbound::OutboundCategory { crate::ports::outbound::OutboundCategory::Embedding }
        fn capabilities(&self) -> crate::ports::output::ResourceCapabilities {
            crate::ports::output::ResourceCapabilities::standard("test")
        }
    }
    impl crate::ports::outbound::OutboundManifest for TestNotification {
        fn id(&self) -> &str { "fp-outbound-notify-test" }
        fn category(&self) -> crate::ports::outbound::OutboundCategory { crate::ports::outbound::OutboundCategory::Notify }
        fn capabilities(&self) -> crate::ports::output::ResourceCapabilities {
            crate::ports::output::ResourceCapabilities::standard("test")
        }
    }
    impl crate::ports::outbound::OutboundManifest for TestNullRemoteStorage {
        fn id(&self) -> &str { "fp-outbound-storage-test-null" }
        fn category(&self) -> crate::ports::outbound::OutboundCategory { crate::ports::outbound::OutboundCategory::Storage }
        fn capabilities(&self) -> crate::ports::output::ResourceCapabilities {
            crate::ports::output::ResourceCapabilities::standard("test-null")
        }
    }

    fn build_service(tmp: &TempDir) -> FileProcessingService {
        let inbox = tmp.path().join("inbox");
        let processed = tmp.path().join("processed");
        let originals = tmp.path().join("originals");
        let sensitive = tmp.path().join("sensitive");
        let todo = tmp.path().join("todo");
        let quarantine = tmp.path().join("quarantine");
        for d in [&inbox, &processed, &originals, &sensitive, &todo, &quarantine] {
            std::fs::create_dir_all(d).expect("create dir");
        }

        FileProcessingService {
            llm: Arc::new(TestLlm::new()),
            storage: Arc::new(TestStorage),
            vector_db: Arc::new(TestVectorDb::new()),
            embedding: Arc::new(TestEmbedder),
            notification: Arc::new(TestNotification),
            verification: None,
            preprocessing: Arc::new(TestPreprocess),
            remote_storage: Arc::new(TestNullRemoteStorage),
            audit: Arc::new(crate::ports::output::NullAuditAdapter),
            duplicate_resolution: Arc::new(TestDupResolution),
            sensitive_notification: Arc::new(TestSensitiveNotif),
            registry: Arc::new(DocTypeRegistry::empty()),
            sensitivity_detector: SensitivityDetector::default(),
            pii_user_patterns: std::sync::RwLock::new(Vec::new()),
            inbox_dir: inbox,
            processed_dir: processed,
            originals_dir: originals,
            sensitive_dir: sensitive,
            todo_dir: todo,
            semantic_dup_threshold: 0.0001,
            max_retry: 1,
            quarantine_dir: quarantine,
            global_thresholds: None,
            verification_enabled: false,
            fragment_threshold: 0,
            crossref_mode: "auto".into(),
            crossref_similarity_threshold: 0.5,
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
            compile_state: std::sync::Mutex::new(CompileState::new()),
            compile_state_path: tmp.path().join(".compile-state.json"),
            compile_state_batch: std::sync::atomic::AtomicBool::new(false),
            summary: std::sync::Mutex::new(ProcessingSummary::default()),
            progress_callback: None,
            error_log: std::sync::Mutex::new(crate::domain::error_log::ErrorLog::new()),
            token_usage: std::sync::Mutex::new(crate::domain::models::TokenUsage::default()),
            embed_instruction_prefix: None,
            crossref_queue: std::sync::Mutex::new(Vec::new()),
            crossref_last_run: std::sync::Mutex::new(None),
            crossref_interval_secs: 30,
            metrics_recorder: None,
            plugin_registry: Arc::new(crate::plugin::PluginRegistry::new()),
        }
    }

    // ── 테스트 ──

    #[tokio::test]
    async fn test_process_file_normal_flow() {
        let tmp = TempDir::new().expect("tempdir");
        let svc = build_service(&tmp);
        let file = svc.inbox_dir.join("normal.txt");
        std::fs::write(&file, "This is a test document with enough content for processing.").expect("write");

        svc.process_file(&file).await.expect("process_file");

        // 원본 삭제됨
        assert!(!file.exists());
        // processed에 zst 파일 생성됨
        let processed_files: Vec<_> = std::fs::read_dir(&svc.processed_dir)
            .expect("read_dir").flatten()
            .filter(|e| e.path().extension().and_then(|e| e.to_str()) == Some("zst"))
            .collect();
        assert!(!processed_files.is_empty(), "processed .zst file should exist");
        // summary 업데이트됨
        let summary = svc.summary.lock().expect("mutex");
        assert_eq!(summary.success, 1);
    }

    #[tokio::test]
    async fn test_process_file_sha256_duplicate() {
        let tmp = TempDir::new().expect("tempdir");
        let svc = build_service(&tmp);

        // 첫 파일 처리
        let file1 = svc.inbox_dir.join("first.txt");
        std::fs::write(&file1, "duplicate content").expect("write");
        svc.process_file(&file1).await.expect("first");

        // 같은 내용의 두 번째 파일
        let file2 = svc.inbox_dir.join("second.txt");
        std::fs::write(&file2, "duplicate content").expect("write");
        svc.process_file(&file2).await.expect("second");

        let summary = svc.summary.lock().expect("mutex");
        assert_eq!(summary.duplicates, 1, "second file should be detected as duplicate");
    }

    #[tokio::test]
    async fn test_process_file_incremental_skip() {
        let tmp = TempDir::new().expect("tempdir");
        let svc = build_service(&tmp);

        let file = svc.inbox_dir.join("incremental.txt");
        std::fs::write(&file, "incremental test content").expect("write");
        svc.process_file(&file).await.expect("first");

        // 같은 파일 재생성 (같은 내용)
        std::fs::write(&file, "incremental test content").expect("write");
        svc.process_file(&file).await.expect("second");

        let summary = svc.summary.lock().expect("mutex");
        // 첫 처리는 success, 두 번째는 SHA256 중복 (vector_db에 hash 등록됨)
        // 또는 증분 스킵
        assert!(summary.success + summary.duplicates + summary.skipped >= 2);
    }

    #[tokio::test]
    async fn test_process_file_fragment() {
        let tmp = TempDir::new().expect("tempdir");
        let mut svc = build_service(&tmp);
        svc.fragment_threshold = 100; // 100자 이하 = fragment

        let file = svc.inbox_dir.join("short.txt");
        std::fs::write(&file, "짧은 메모").expect("write");
        svc.process_file(&file).await.expect("fragment");

        // fragment로 처리됨
        let summary = svc.summary.lock().expect("mutex");
        assert_eq!(summary.success, 1);
        assert!(!file.exists(), "inbox file should be removed");
    }

    #[tokio::test]
    async fn test_process_file_sensitive() {
        let tmp = TempDir::new().expect("tempdir");
        let svc = build_service(&tmp);

        // 민감 파일 (확장자 기반)
        let file = svc.inbox_dir.join("secret.env");
        std::fs::write(&file, "API_KEY=secret123").expect("write");
        svc.process_file(&file).await.expect("sensitive");

        let summary = svc.summary.lock().expect("mutex");
        assert_eq!(summary.sensitive, 1);
    }

    #[tokio::test]
    async fn test_process_file_verification_pass() {
        let tmp = TempDir::new().expect("tempdir");
        let mut svc = build_service(&tmp);
        svc.verification_enabled = true;
        // 매우 느슨한 임계값으로 통과 보장
        svc.global_thresholds = Some(VerificationThresholds {
            structure_min: 0.0, compression_min: 0.0, compression_max: 100.0,
            keyword_coverage_min: 0.0, keyword_completeness_min: 0.0,
            rouge_l_min: 0.0, entity_preservation_min: 0.0,
        });

        let file = svc.inbox_dir.join("verified.txt");
        std::fs::write(&file, "This document contains keyword1 and some content for verification testing.").expect("write");
        svc.process_file(&file).await.expect("verified");

        let summary = svc.summary.lock().expect("mutex");
        assert_eq!(summary.success, 1);
        assert!(!summary.verification_metrics.is_empty());
    }

    #[tokio::test]
    async fn test_classify_with_retry() {
        let tmp = TempDir::new().expect("tempdir");
        let mut svc = build_service(&tmp);
        svc.llm = Arc::new(TestLlm::failing_first());
        svc.max_retry = 2;

        let file = svc.inbox_dir.join("retry.txt");
        std::fs::write(&file, "retry test content").expect("write");

        // 1차 실패 → 2차 성공
        svc.process_file(&file).await.expect("should succeed after retry");
        let summary = svc.summary.lock().expect("mutex");
        assert_eq!(summary.success, 1);
    }

    #[test]
    fn test_compute_hash_deterministic() {
        let tmp = TempDir::new().expect("tempdir");
        let file = tmp.path().join("hash_test.txt");
        std::fs::write(&file, "deterministic content").expect("write");

        let h1 = FileProcessingService::compute_hash(&file).expect("hash1");
        let h2 = FileProcessingService::compute_hash(&file).expect("hash2");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64); // SHA-256 hex
    }

    // test_purge_expired 제거됨: purge_expired_originals 메서드는 Phase 55에서 제거됨
    // (retention/purge 시스템으로 대체)
}
