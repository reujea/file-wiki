use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use file_pipeline_core::service::FileProcessingService;

/// 스킵할 확장자 목록 (임시/다운로드 중 파일)
const SKIP_EXTENSIONS: &[&str] = &[".tmp", ".part", ".crdownload", ".download"];

/// 비가공 대상: config 파일 (구조 자체가 정보, 보안 위험)
const CONFIG_EXTENSIONS: &[&str] = &[
    ".env", ".ini", ".cfg", ".conf",
    ".properties", ".plist", ".reg",
];

/// 비가공 대상: 소스 코드 (문서가 아님, 전처리기 미지원)
const SOURCE_CODE_EXTENSIONS: &[&str] = &[
    ".rs", ".py", ".js", ".ts", ".tsx", ".jsx",
    ".go", ".java", ".kt", ".c", ".cpp", ".h", ".hpp",
    ".cs", ".rb", ".php", ".swift", ".scala", ".zig",
    ".sh", ".bash", ".ps1", ".bat", ".cmd",
    ".sql", ".graphql", ".proto",
    ".lock", ".sum",
];

/// 비가공 대상: 바이너리/미디어 (텍스트 추출 불가)
const BINARY_EXTENSIONS: &[&str] = &[
    ".exe", ".dll", ".so", ".dylib", ".o", ".a",
    ".zip", ".gz", ".tar", ".7z", ".rar", ".zst",
    ".mp3", ".mp4", ".avi", ".mov", ".wav", ".flac",
    ".woff", ".woff2", ".ttf", ".otf",
    ".sqlite", ".db",
];

/// 비가공 대상 파일명 패턴 (확장자 무관)
const SKIP_FILENAMES: &[&str] = &[
    "Cargo.toml", "Cargo.lock",
    "package.json", "package-lock.json",
    "tsconfig.json", "eslint",
    "Makefile", "Dockerfile", "docker-compose",
    ".gitignore", ".gitattributes",
    "pipeline.toml",
];

/// 스킵 대상인지 확인
fn should_skip(path: &Path) -> bool {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| format!(".{}", e.to_lowercase()))
        .unwrap_or_default();

    // 1. 임시/다운로드 중
    if SKIP_EXTENSIONS.contains(&ext.as_str()) {
        debug!("스킵 (임시파일): {:?}", path);
        return true;
    }

    // 2. config 파일
    if CONFIG_EXTENSIONS.contains(&ext.as_str()) {
        debug!("스킵 (config): {:?}", path);
        return true;
    }

    // 3. 소스 코드
    if SOURCE_CODE_EXTENSIONS.contains(&ext.as_str()) {
        debug!("스킵 (소스코드): {:?}", path);
        return true;
    }

    // 4. 바이너리/미디어
    if BINARY_EXTENSIONS.contains(&ext.as_str()) {
        debug!("스킵 (바이너리): {:?}", path);
        return true;
    }

    // 5. 특정 파일명
    let filename = path.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    for pattern in SKIP_FILENAMES {
        if filename == *pattern || filename.starts_with(pattern) {
            debug!("스킵 (파일명): {:?}", path);
            return true;
        }
    }

    false
}

/// 파이프라인 스텝별 크레덴셜을 역할별 LLM 맵으로 해석
fn resolve_pipeline_llms(
    pipeline: &file_pipeline_core::domain::models::PipelineDefinition,
    credential_llms: &std::collections::HashMap<String, Arc<dyn file_pipeline_core::ports::output::LLMPort>>,
) -> std::collections::HashMap<String, Arc<dyn file_pipeline_core::ports::output::LLMPort>> {
    use file_pipeline_core::domain::models::PipelineStep;
    let mut overrides = std::collections::HashMap::new();

    for step in &pipeline.steps {
        match step {
            PipelineStep::Llm { credential: Some(cred_name) } => {
                if let Some(llm) = credential_llms.get(cred_name) {
                    overrides.insert("classify".to_string(), Arc::clone(llm));
                }
            }
            PipelineStep::Verify { credential: Some(cred_name), .. } => {
                if let Some(llm) = credential_llms.get(cred_name) {
                    overrides.insert("verify".to_string(), Arc::clone(llm));
                }
            }
            PipelineStep::Embedding { credential: Some(cred_name), .. } => {
                if let Some(llm) = credential_llms.get(cred_name) {
                    overrides.insert("embed".to_string(), Arc::clone(llm));
                }
            }
            _ => {}
        }
    }

    // 후처리 크레덴셜 (PipelineDefinition 레벨)
    if let Some(ref cred_name) = pipeline.postprocess_credential {
        if let Some(llm) = credential_llms.get(cred_name) {
            overrides.insert("postprocess".to_string(), Arc::clone(llm));
        }
    }

    overrides
}

/// inbox 디렉토리 감시 → 파일 생성 이벤트를 FileProcessingService로 전달
pub struct FileWatcher {
    inbox_dir: PathBuf,
    /// 추가 inbox 경로 목록
    extra_inboxes: Vec<PathBuf>,
    service: Arc<FileProcessingService>,
    /// 토픽 자동 병합 디렉토리 (None이면 자동 병합 비활성화)
    topics_dir: Option<PathBuf>,
    /// 자동 병합 트리거 문서 수
    auto_merge_threshold: usize,
    /// 동시 처리 파일 수 제한
    max_workers: usize,
    /// 고정 파이프라인 정의
    pipeline: file_pipeline_core::domain::models::PipelineDefinition,
    /// 크레덴셜 이름 → LLM 어댑터 매핑 (파이프라인 credential override용)
    credential_llms: std::collections::HashMap<String, Arc<dyn file_pipeline_core::ports::output::LLMPort>>,
}

impl FileWatcher {
    pub fn new(inbox_dir: PathBuf, service: Arc<FileProcessingService>) -> Self {
        Self {
            inbox_dir,
            extra_inboxes: vec![],
            service,
            topics_dir: None,
            auto_merge_threshold: file_pipeline_core::domain::topic_merger::AUTO_MERGE_THRESHOLD,
            max_workers: 4,
            pipeline: Default::default(),
            credential_llms: std::collections::HashMap::new(),
        }
    }

    /// 추가 inbox 경로 설정
    pub fn with_extra_inboxes(mut self, extra: Vec<PathBuf>) -> Self {
        self.extra_inboxes = extra;
        self
    }

    /// 고정 파이프라인 설정
    pub fn with_pipeline(mut self, pipeline: file_pipeline_core::domain::models::PipelineDefinition) -> Self {
        self.pipeline = pipeline;
        self
    }

    /// 크레덴셜 → LLM 어댑터 매핑 설정
    pub fn with_credential_llms(mut self, map: std::collections::HashMap<String, Arc<dyn file_pipeline_core::ports::output::LLMPort>>) -> Self {
        self.credential_llms = map;
        self
    }

    pub fn with_max_workers(mut self, max_workers: usize) -> Self {
        self.max_workers = max_workers.max(1);
        self
    }

    /// 자동 토픽 병합 활성화
    pub fn with_auto_merge(mut self, topics_dir: PathBuf, threshold: usize) -> Self {
        self.topics_dir = Some(topics_dir);
        self.auto_merge_threshold = threshold;
        self
    }

    /// 교차참조 배치 처리 위임
    pub fn flush_crossref(&self) -> Result<usize> {
        self.service.flush_crossref()
    }

    /// 배치 처리: inbox 스캔 → 계획 → 순차/병렬 처리 → 상태 저장
    pub async fn batch_process(&self) -> Result<file_pipeline_core::domain::work_queue::QueueStats> {
        use file_pipeline_core::domain::work_queue::WorkQueue;

        let queue_path = self.inbox_dir.parent().unwrap_or(&self.inbox_dir).join(".work-queue.json");
        let mut queue = WorkQueue::load(&queue_path).unwrap_or_else(|_| WorkQueue::new());

        let plan = queue.scan_and_plan(&self.inbox_dir)?;
        if plan.is_empty() {
            info!("배치 처리: 작업 없음");
            let stats = queue.stats();
            queue.save(&queue_path)?;
            return Ok(stats);
        }

        // ── before ���냅샷 ──
        let batch_start = std::time::Instant::now();
        let before_docs = self.service.vector_db.stats().map(|s| s.total_documents).unwrap_or(0);
        let before_rels: usize = self.service.vector_db.list_all()
            .map(|all| all.iter().map(|d| self.service.vector_db.find_related(&d.id).map(|r| r.len()).unwrap_or(0)).sum())
            .unwrap_or(0);
        let file_count = plan.total_work();
        info!(
            "[bench-before] docs={}, relations={}, pending={}",
            before_docs, before_rels, file_count
        );

        let eta = plan.estimated_time_secs(12.6, self.max_workers);
        info!(
            "배치 처리 시작: {} 건 (소형 {}, 대형 {}, 변경 {}), 예상 {:.0}초",
            plan.total_work(), plan.small_files.len(), plan.large_files.len(),
            plan.modified_files.len(), eta,
        );

        // 소형 + 변경 파일 처리 (병렬)
        let all_files: Vec<_> = plan.small_files.iter()
            .chain(plan.modified_files.iter())
            .chain(plan.large_files.iter())
            .cloned()
            .collect();

        // 배치 모드: vector_db + compile_state 디스크 I/O 지연
        self.service.vector_db.batch_begin();
        self.service.compile_state_batch_begin();

        let semaphore = Arc::new(tokio::sync::Semaphore::new(self.max_workers));
        let queue_mutex = Arc::new(tokio::sync::Mutex::new(queue));

        let pipeline = Arc::new(self.pipeline.clone());
        let cred_llms = Arc::new(self.credential_llms.clone());
        let mut handles = vec![];
        let queue_path_arc = Arc::new(queue_path.clone());
        for path in all_files {
            // 스킵 대상 필터링 (config, 소스코드, 바이너리 등)
            if should_skip(&path) {
                tracing::debug!("배치 스킵: {:?}", path);
                continue;
            }

            let service = Arc::clone(&self.service);
            let sem = Arc::clone(&semaphore);
            let qm = Arc::clone(&queue_mutex);
            let qp = Arc::clone(&queue_path_arc);
            let path_clone = path.clone();
            let pl = Arc::clone(&pipeline);
            let cred_llms = Arc::clone(&cred_llms);

            handles.push(tokio::spawn(async move {
                // semaphore 대기 중인 파일도 대시보드에 Pending으로 보이도록 사전 등록.
                // (scan_and_plan에서 이미 items에 들어있지만 status가 Pending인지 확인 + save)
                {
                    let mut q = qm.lock().await;
                    q.ensure_item(&path_clone);
                    let _ = q.save(&qp);
                }

                let _permit = sem.acquire().await.expect("semaphore closed");

                // 상태 전환 즉시 save하여 대시보드 통계 폴링 다음 tick에 반영.
                {
                    let mut q = qm.lock().await;
                    q.mark_processing(&path_clone);
                    let _ = q.save(&qp);
                }

                info!("파이프라인 처리 시작: {:?}", path_clone);
                let llm_overrides = resolve_pipeline_llms(&pl, &cred_llms);
                let result = service.process_file_with_pipeline(&path_clone, &pl, &llm_overrides).await;

                {
                    let mut q = qm.lock().await;
                    match result {
                        Ok(()) => q.mark_done(&path_clone),
                        Err(e) => q.mark_failed(&path_clone, &e.to_string()),
                    }
                    let _ = q.save(&qp);
                }
            }));
        }

        // 모든 작업 대기
        for h in handles {
            let _ = h.await;
        }

        // 배치 모드 종료: 1회만 디스크에 기록
        self.service.vector_db.batch_end();
        self.service.compile_state_batch_end();

        // 교차참조 flush + DB refresh (전체 가공 완료 시점)
        let _ = self.service.flush_crossref();
        if self.service.vector_db.has_pending_work() {
            self.service.vector_db.db_refresh();
        }

        // 삭제된 파일 정리
        {
            let mut q = queue_mutex.lock().await;
            q.purge_deleted();
            q.save(&queue_path)?;
            let stats = q.stats();

            // 요약 알림
            if let Err(e) = self.service.flush_summary().await {
                error!("요약 알림 전송 실패: {}", e);
            }

            // ── after 스냅샷 + 비교 로그 ──
            let batch_secs = batch_start.elapsed().as_secs_f64();
            let after_docs = self.service.vector_db.stats().map(|s| s.total_documents).unwrap_or(0);
            let after_rels: usize = self.service.vector_db.list_all()
                .map(|all| all.iter().map(|d| self.service.vector_db.find_related(&d.id).map(|r| r.len()).unwrap_or(0)).sum())
                .unwrap_or(0);
            let new_docs = after_docs.saturating_sub(before_docs);
            let new_rels = after_rels.saturating_sub(before_rels);
            let per_doc_avg = if new_docs > 0 { batch_secs / new_docs as f64 } else { 0.0 };

            info!(
                "[bench-after] docs={} (+{}), relations={} (+{}), time={:.1}s, per-doc={:.1}s, throughput={:.1} docs/s",
                after_docs, new_docs, after_rels, new_rels, batch_secs, per_doc_avg,
                if batch_secs > 0.0 { new_docs as f64 / batch_secs } else { 0.0 }
            );

            // JSON 로그 파일 저장 (logs/bench_{timestamp}.json)
            let log_level = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
            let bench_log = serde_json::json!({
                "timestamp": chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
                "log_level": log_level,
                "config": {
                    "max_workers": self.max_workers,
                    "crossref_mode": self.service.crossref_mode,
                    "crossref_threshold": self.service.crossref_similarity_threshold,
                    "llm_provider": "runtime",
                },
                "before": { "docs": before_docs, "relations": before_rels },
                "after": { "docs": after_docs, "relations": after_rels },
                "delta": { "new_docs": new_docs, "new_relations": new_rels },
                "performance": {
                    "total_secs": batch_secs,
                    "per_doc_secs": per_doc_avg,
                    "docs_per_sec": if batch_secs > 0.0 { new_docs as f64 / batch_secs } else { 0.0 },
                    "file_count": file_count,
                },
                "queue": { "done": stats.done, "failed": stats.failed, "pending": stats.pending },
            });
            let logs_dir = self.inbox_dir.parent().unwrap_or(&self.inbox_dir).join("logs");
            let _ = std::fs::create_dir_all(&logs_dir);
            let bench_file = logs_dir.join(format!("bench_{}.json", chrono::Local::now().format("%Y%m%d_%H%M%S")));
            if let Ok(json) = serde_json::to_string_pretty(&bench_log) {
                let _ = std::fs::write(&bench_file, json);
                info!("[bench-saved] {}", bench_file.display());
            }

            info!("배치 처리 완료: {:?}", stats);
            Ok(stats)
        }
    }

    /// 감시 시작 (블로킹, ctrl+c로 종료)
    pub async fn watch(&self) -> Result<()> {
        use file_pipeline_core::domain::work_queue::WorkQueue;

        // WorkQueue: 실시간 watch도 대시보드 통계에 반영되도록 갱신 (batch_process와 동일 패턴).
        // queue_path는 inbox의 부모(=base) 아래 .work-queue.json.
        let queue_path = self.inbox_dir.parent().unwrap_or(&self.inbox_dir).join(".work-queue.json");
        let queue = WorkQueue::load(&queue_path).unwrap_or_else(|_| WorkQueue::new());
        let queue_mutex = Arc::new(tokio::sync::Mutex::new(queue));
        let queue_path = Arc::new(queue_path);

        let (tx, mut rx) = mpsc::channel::<PathBuf>(100);

        let tx_clone = tx.clone();
        let mut watcher = RecommendedWatcher::new(
            move |res: Result<Event, notify::Error>| match res {
                Ok(event) => {
                    if matches!(
                        event.kind,
                        EventKind::Create(_) | EventKind::Modify(_)
                    ) {
                        for path in event.paths {
                            if path.is_file() && !should_skip(&path) {
                                let _ = tx_clone.blocking_send(path);
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("파일 감시 오류: {}", e);
                }
            },
            notify::Config::default(),
        )?;

        watcher.watch(&self.inbox_dir, RecursiveMode::NonRecursive)?;
        info!("inbox 감시 시작: {:?} (max_workers={})", self.inbox_dir, self.max_workers);

        // extra inboxes도 감시
        for extra in &self.extra_inboxes {
            if extra.exists() {
                watcher.watch(extra, RecursiveMode::NonRecursive)?;
                info!("extra inbox 감시: {:?}", extra);
            } else {
                warn!("extra inbox 경로 없음 (무시): {:?}", extra);
            }
        }

        let semaphore = Arc::new(tokio::sync::Semaphore::new(self.max_workers));

        // 기존 파일도 처리 (메인 inbox + extra)
        let all_inboxes: Vec<&PathBuf> = std::iter::once(&self.inbox_dir)
            .chain(self.extra_inboxes.iter())
            .collect();
        for inbox in &all_inboxes {
            if let Ok(entries) = std::fs::read_dir(inbox) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() && !should_skip(&path) {
                        info!("기존 파일 처리: {:?}", path);
                        tx.send(path).await?;
                    }
                }
            }
        }

        loop {
            // 30초 유휴 시 요약 알림 전송
            let path = match tokio::time::timeout(
                tokio::time::Duration::from_secs(30),
                rx.recv(),
            ).await {
                Ok(Some(path)) => path,
                Ok(None) => break, // 채널 닫힘
                Err(_) => {
                    // 30초간 새 파일 없음 → 누적 요약 전송 + 교차참조 배치
                    if let Err(e) = self.service.flush_summary().await {
                        error!("요약 알림 전송 실패: {}", e);
                    }
                    match self.service.flush_crossref() {
                        Ok(n) if n > 0 => info!("교차참조 배치 처리: {} 건", n),
                        Err(e) => error!("교차참조 배치 실패: {}", e),
                        _ => {}
                    }
                    // 유휴 시 db_refresh (mmap + HNSW 재빌드)
                    if self.service.vector_db.has_pending_work() {
                        self.service.vector_db.db_refresh();
                        info!("유휴 시점 db_refresh 완료");
                    }
                    continue;
                }
            };

            // 짧은 딜레이: 파일 쓰기 완료 대기
            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

            if !path.exists() {
                continue;
            }

            let service = Arc::clone(&self.service);
            let path_clone = path.clone();
            let topics_dir = self.topics_dir.clone();
            let threshold = self.auto_merge_threshold;
            let sem = Arc::clone(&semaphore);
            let pl = self.pipeline.clone();
            let cred_llms_watch = self.credential_llms.clone();
            let qm = Arc::clone(&queue_mutex);
            let qp = Arc::clone(&queue_path);

            let retry_tx = tx.clone();
            tokio::spawn(async move {
                // 1) semaphore.acquire 이전에 ensure_item으로 Pending 등록 + save.
                //    동시 처리 한계(max_workers)에 막혀 대기 중인 파일도 즉시 대시보드 통계에 반영.
                {
                    let mut q = qm.lock().await;
                    q.ensure_item(&path_clone);
                    let _ = q.save(&qp);
                }

                let _permit = sem.acquire().await.expect("semaphore closed");

                // 2) acquire 통과 후 Pending → Processing 전환 + save.
                {
                    let mut q = qm.lock().await;
                    q.mark_processing(&path_clone);
                    let _ = q.save(&qp);
                }

                info!("파이프라인 처리 시작: {:?}", path_clone);
                let llm_overrides = resolve_pipeline_llms(&pl, &cred_llms_watch);
                let process_result = service.process_file_with_pipeline(&path_clone, &pl, &llm_overrides).await;

                if let Err(e) = process_result {
                    // WorkQueue: 실패 마킹 + save (재시도 로직과 별개로 현재 상태 반영).
                    {
                        let mut q = qm.lock().await;
                        q.mark_failed(&path_clone, &e.to_string());
                        let _ = q.save(&qp);
                    }
                    error!("파일 처리 실패 {:?}: {} — 30초 후 재시도", path_clone, e);
                    let path_retry = path_clone.clone();
                    tokio::spawn(async move {
                        // 단일 재시도 (현 구조). 재실패는 retry_tx 채널 측에서 다음 라운드 처리.
                        // 다중 백오프 (30s/60s/120s)는 후속 — 채널 구조 변경 필요.
                        tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
                        if !path_retry.exists() {
                            tracing::debug!("재시도 대상 파일 없음: {:?}", path_retry);
                            return;
                        }
                        tracing::info!("재시도 1/1: {:?}", path_retry);
                        let _ = retry_tx.send(path_retry.clone()).await;
                    });
                    return;
                }

                // WorkQueue: 완료 마킹 + save.
                {
                    let mut q = qm.lock().await;
                    q.mark_done(&path_clone);
                    let _ = q.save(&qp);
                }

                // 자동 토픽 병합
                if let Some(ref dir) = topics_dir {
                    use file_pipeline_core::domain::topic_merger::TopicMerger;
                    match TopicMerger::auto_merge_if_needed(
                        service.vector_db.as_ref(),
                        service.storage.as_ref(),
                        service.llm.as_ref(),
                        service.embedding.as_ref(),
                        dir,
                        threshold,
                    ).await {
                        Ok(report) if report.topics_created > 0 => {
                            info!("자동 병합: {} 토픽 ({} 문서)", report.topics_created, report.documents_merged);
                        }
                        Err(e) => {
                            debug!("자동 병합 실패: {}", e);
                        }
                        _ => {}
                    }
                }
            });
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn test_skip_temp_files() {
        assert!(should_skip(Path::new("download.tmp")));
        assert!(should_skip(Path::new("file.part")));
        assert!(should_skip(Path::new("chrome.crdownload")));
    }

    #[test]
    fn test_skip_config_files() {
        // ".env" 는 확장자가 없는 dotfile이므로, 확장자 매칭 대상이 아님
        // 확장자가 .env, .ini, .cfg 인 파일을 테스트
        assert!(should_skip(Path::new("production.env")));
        assert!(should_skip(Path::new("settings.ini")));
        assert!(should_skip(Path::new("app.cfg")));
    }

    #[test]
    fn test_skip_source_code() {
        assert!(should_skip(Path::new("main.rs")));
        assert!(should_skip(Path::new("app.py")));
        assert!(should_skip(Path::new("index.js")));
        assert!(should_skip(Path::new("component.ts")));
    }

    #[test]
    fn test_skip_binary_files() {
        assert!(should_skip(Path::new("program.exe")));
        assert!(should_skip(Path::new("archive.zip")));
        assert!(should_skip(Path::new("video.mp4")));
    }

    #[test]
    fn test_skip_special_filenames() {
        assert!(should_skip(Path::new("Cargo.toml")));
        assert!(should_skip(Path::new("Dockerfile")));
        assert!(should_skip(Path::new("pipeline.toml")));
    }

    #[test]
    fn test_allow_documents() {
        assert!(!should_skip(Path::new("readme.txt")));
        assert!(!should_skip(Path::new("notes.md")));
        assert!(!should_skip(Path::new("report.pdf")));
    }

    #[test]
    fn test_skip_case_insensitive() {
        assert!(should_skip(Path::new("FILE.TMP")));
        assert!(should_skip(Path::new("Main.RS")));
        assert!(should_skip(Path::new("ARCHIVE.ZIP")));
    }

    #[test]
    fn test_allow_docx_xlsx() {
        assert!(!should_skip(Path::new("회의록.docx")));
        assert!(!should_skip(Path::new("데이터.xlsx")));
        assert!(!should_skip(Path::new("프레젠테이션.pptx")));
    }

    #[test]
    fn test_resolve_pipeline_llms_empty() {
        let pipeline = file_pipeline_core::domain::models::PipelineDefinition::default();
        let cred_llms = std::collections::HashMap::new();
        let overrides = resolve_pipeline_llms(&pipeline, &cred_llms);
        assert!(overrides.is_empty());
    }

    #[test]
    fn test_resolve_pipeline_llms_with_credential() {
        use file_pipeline_core::domain::models::{PipelineDefinition, PipelineStep};
        let pipeline = PipelineDefinition {
            steps: vec![
                PipelineStep::Llm { credential: Some("my_claude".into()) },
            ],
            postprocess_credential: None,
        };
        let mut cred_llms: std::collections::HashMap<String, Arc<dyn file_pipeline_core::ports::output::LLMPort>> = std::collections::HashMap::new();
        cred_llms.insert("my_claude".into(), Arc::new(crate::stub::StubLlm));

        let overrides = resolve_pipeline_llms(&pipeline, &cred_llms);
        assert!(overrides.contains_key("classify"), "LLM 스텝 credential → classify override");
    }

    #[test]
    fn test_resolve_pipeline_llms_postprocess() {
        use file_pipeline_core::domain::models::{PipelineDefinition, PipelineStep};
        let pipeline = PipelineDefinition {
            steps: vec![
                PipelineStep::Llm { credential: None },
            ],
            postprocess_credential: Some("post_llm".into()),
        };
        let mut cred_llms: std::collections::HashMap<String, Arc<dyn file_pipeline_core::ports::output::LLMPort>> = std::collections::HashMap::new();
        cred_llms.insert("post_llm".into(), Arc::new(crate::stub::StubLlm));

        let overrides = resolve_pipeline_llms(&pipeline, &cred_llms);
        assert!(overrides.contains_key("postprocess"));
    }

    #[test]
    fn test_resolve_pipeline_llms_missing_credential() {
        use file_pipeline_core::domain::models::{PipelineDefinition, PipelineStep};
        let pipeline = PipelineDefinition {
            steps: vec![
                PipelineStep::Llm { credential: Some("nonexistent".into()) },
            ],
            postprocess_credential: None,
        };
        let cred_llms = std::collections::HashMap::new();
        let overrides = resolve_pipeline_llms(&pipeline, &cred_llms);
        assert!(overrides.is_empty(), "존재하지 않는 credential → 무시");
    }
}
