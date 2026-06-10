use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use tracing::{info, warn};

use crate::domain::classifier::SensitivityDetector;
use crate::domain::deduplicator::render_diff;
use crate::domain::incremental::CompileState;
use crate::domain::models::{
    ClassifyAndProcessResult, DocTypeRegistry, Document, DuplicateAction, Metadata,
    ProcessingSummary, VerificationLevel, VerificationMetricEntry,
};
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
fn cosine_sim_inline(a: &[f32], b: &[f32]) -> f32 {
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
fn meta_block_pass(
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
    fn emit_progress(&self, event: &str) {
        if let Some(cb) = &self.progress_callback {
            cb(event);
        }
    }

    // ── Phase 82-prep: 메트릭 영속화 헬퍼 ─────────────────────
    fn metrics_success(&self) {
        if let Some(m) = &self.metrics_recorder { m.record_success(); }
    }
    fn metrics_error(&self) {
        if let Some(m) = &self.metrics_recorder { m.record_error(); }
    }
    fn metrics_quarantine(&self) {
        if let Some(m) = &self.metrics_recorder { m.record_quarantine(); }
    }
    fn metrics_verify(&self, passed: bool) {
        if let Some(m) = &self.metrics_recorder { m.record_verify(passed); }
    }
    fn metrics_time(&self, started: std::time::Instant) {
        if let Some(m) = &self.metrics_recorder {
            let ms = started.elapsed().as_millis().min(u64::MAX as u128) as u64;
            m.record_process_time(ms);
        }
    }

    fn compute_hash(path: &Path) -> Result<String> {
        let bytes = std::fs::read(path).context("파일 읽기 실패")?;
        let hash = Sha256::digest(&bytes);
        Ok(hex::encode(hash))
    }

    fn read_text(path: &Path) -> Result<String> {
        std::fs::read_to_string(path).context("텍스트 파일 읽기 실패")
    }

    /// 메인 처리 플로우: Default 파이프라인으로 위임
    ///
    /// 모든 파일 처리는 파이프라인 경유. 이 메서드는 기본 크레덴셜(self.llm)을 사용하는 편의 메서드.
    pub async fn process_file(&self, file_path: &Path) -> Result<()> {
        let default_pipeline = crate::domain::models::PipelineDefinition {
            steps: vec![
                crate::domain::models::PipelineStep::Preprocess {
                    pdf_tool: "none".into(),
                    ocr_tool: "none".into(),
                },
                crate::domain::models::PipelineStep::Llm { credential: None },
                crate::domain::models::PipelineStep::Verify {
                    enabled: self.verification_enabled,
                    thresholds: self.global_thresholds.clone(),
                    credential: None,
                },
            ],
            postprocess_credential: None,
        };
        let empty_overrides = std::collections::HashMap::new();
        self.process_file_with_pipeline(file_path, &default_pipeline, &empty_overrides).await
    }


    /// 파이프라인 정의에 따라 파일 처리 (커스텀 파이프라인 모드)
    ///
    /// `llm_overrides` — 역할별 LLM 오버라이드 맵:
    /// - "classify": 분류/가공 LLM
    /// - "verify": 검증/2-Pass 재가공 LLM
    /// - "embed": 임베딩용 (EmbeddingPort로 래핑 필요 — 현재는 모델 오버라이드만)
    /// - "postprocess": Todo 병합, 교차참조, 토픽 병합 LLM
    pub async fn process_file_with_pipeline(
        &self,
        file_path: &Path,
        pipeline: &crate::domain::models::PipelineDefinition,
        llm_overrides: &std::collections::HashMap<String, Arc<dyn LLMPort>>,
    ) -> Result<()> {
        use crate::domain::models::PipelineStep;

        let fname = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown");
        info!("파이프라인 처리 시작: {:?}", file_path);
        self.emit_progress(&format!("{{\"event\":\"start\",\"file\":\"{}\"}}", fname));

        // [프로파일링] 구간별 타이머
        let _t_pipeline = std::time::Instant::now();
        // Phase 82-prep: 처리 시간 측정 시작
        let metrics_t_start = std::time::Instant::now();

        // === 공통 전처리 (항상 실행) ===

        // Phase 91 A1': 민감/PII 단일 진입점 (check_sensitive_and_pii).
        // 1단계: 경로/파일명 기반 검사 (content=None) — 본문 읽기 전에 빠른 차단.
        let path_decision = self.sensitivity_detector.check_sensitive_and_pii(file_path, None, &[]);
        if path_decision.is_sensitive {
            return self.handle_sensitive(file_path, &path_decision.reason.unwrap_or_default()).await;
        }

        // 1.3. 본문 PII 검출 (Ruflo C2) + Fragment 감지
        if let Ok(content) = std::fs::read_to_string(file_path) {
            // RwLockReadGuard는 async fn에서 Send 불가. 스코프 안에서 clone 후 drop.
            let patterns: Vec<(String, String)> = self.pii_user_patterns.read()
                .expect("pii_user_patterns lock").clone();
            let content_decision = self.sensitivity_detector.check_sensitive_and_pii(
                file_path, Some(&content), &patterns,
            );
            if content_decision.is_sensitive {
                return self
                    .handle_sensitive(file_path, &content_decision.reason.unwrap_or_default())
                    .await;
            }
            if self.fragment_threshold > 0
                && content.trim().len() <= self.fragment_threshold
                && !content.trim().is_empty()
            {
                return self.handle_fragment(file_path, content.trim()).await;
            }
        }

        // 3. SHA-256 + 증분 중복
        let hash = Self::compute_hash(file_path)?;
        if let Some(existing) = self.vector_db.find_by_hash(&hash)? {
            info!("완전 중복 탐지 (SHA-256), 스킵: {}", existing);
            self.summary.lock().expect("mutex poisoned").duplicates += 1;
            return Ok(());
        }
        {
            let state = self.compile_state.lock().expect("mutex poisoned");
            let file_key = file_path.to_string_lossy().to_string();
            if !state.is_changed(&file_key, &hash) {
                info!("증분 컴파일: 변경 없음, 스킵: {:?}", file_path);
                drop(state);
                self.summary.lock().expect("mutex poisoned").skipped += 1;
                return Ok(());
            }
        }

        // === 파이프라인 스텝 순회 ===
        let classify_llm: &dyn LLMPort = llm_overrides.get("classify")
            .map(|a| a.as_ref())
            .unwrap_or(self.llm.as_ref());
        let verify_llm: &dyn LLMPort = llm_overrides.get("verify")
            .map(|a| a.as_ref())
            .unwrap_or(self.llm.as_ref());
        let _postprocess_llm: &dyn LLMPort = llm_overrides.get("postprocess")
            .map(|a| a.as_ref())
            .unwrap_or(self.llm.as_ref());
        let mut preprocessed_text: Option<String> = None;
        let mut llm_result: Option<ClassifyAndProcessResult> = None;
        let mut zstd_level_override: Option<i32> = None;
        let mut embedding_model_override: Option<String> = None;

        for step in &pipeline.steps {
            match step {
                PipelineStep::Preprocess { pdf_tool, ocr_tool } => {
                    self.emit_progress(&format!("{{\"event\":\"step\",\"file\":\"{}\",\"stage\":\"preprocess\"}}", fname));
                    match self.preprocessing.preprocess_with_config(file_path, pdf_tool, ocr_tool) {
                        Ok(result) => {
                            info!("전처리 완료: {} chars", result.text.len());
                            preprocessed_text = Some(result.text);
                        }
                        Err(e) => {
                            // 텍스트 파일이면 직접 읽기 시도, 바이너리 파일이면 에러
                            match Self::read_text(file_path) {
                                Ok(text) => {
                                    info!("전처리 스킵 (텍스트 직접 읽기): {} chars", text.len());
                                    preprocessed_text = Some(text);
                                }
                                Err(_) => {
                                    let ext = file_path.extension().and_then(|e| e.to_str()).unwrap_or("?");
                                    let fname_str = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown").to_string();
                                    self.summary.lock().expect("mutex poisoned").record_error(
                                        &fname_str, &format!("전처리 실패 (.{}): {}", ext, e), "inbox에 잔류",
                                    );
                                    self.metrics_error();
                                    warn!("전처리 실패 (.{}): {} — 가공 중단", ext, e);
                                    return Ok(());
                                }
                            }
                        }
                    }
                }
                PipelineStep::Llm { .. } => {
                    self.emit_progress(&format!("{{\"event\":\"step\",\"file\":\"{}\",\"stage\":\"classify\"}}", fname));
                    // Phase 94 A3: 본 파일 가공의 trace_id (메타 룰 13 2단계 활성화).
                    let trace = crate::audit::TraceId::new();
                    let inputs_hash = crate::audit::input_hash_prefix(file_path.to_string_lossy().as_bytes());
                    // F-5 (lesson 46 G-1): LLM 호출 실패 시 quarantine 라우팅
                    let llm_call: std::result::Result<ClassifyAndProcessResult, anyhow::Error> =
                        if let Some(text) = &preprocessed_text {
                            let file_name = file_path.file_name().unwrap_or_default().to_string_lossy();
                            let mut last_err = None;
                            let mut ok = None;
                            for attempt in 0..=self.max_retry {
                                match classify_llm.classify_and_process_text(&file_name, text, &self.registry).await {
                                    Ok(r) => { ok = Some(r); break; }
                                    Err(e) => {
                                        warn!("LLM 분류+가공 실패 (시도 {}): {}", attempt + 1, e);
                                        last_err = Some(e);
                                    }
                                }
                            }
                            ok.ok_or_else(|| last_err.unwrap())
                        } else {
                            let mut last_err = None;
                            let mut ok = None;
                            for attempt in 0..=self.max_retry {
                                match classify_llm.classify_and_process(file_path, &self.registry).await {
                                    Ok(r) => { ok = Some(r); break; }
                                    Err(e) => {
                                        warn!("LLM 분류+가공 실패 (시도 {}): {}", attempt + 1, e);
                                        last_err = Some(e);
                                    }
                                }
                            }
                            ok.ok_or_else(|| last_err.unwrap())
                        };
                    // Phase 94 A3: LLM 호출 결과를 audit_trace에 기록 (성공·실패 양쪽).
                    match &llm_call {
                        Ok(r) => {
                            let summary = crate::audit::truncate_output_summary(
                                &format!("types={:?} keywords={}", r.doc_types, r.metadata.keywords.len())
                            );
                            self.audit.record(trace.as_str(), "llm.classify", Some(&inputs_hash), Some(&summary), Some("success"));
                        }
                        Err(e) => {
                            let summary = crate::audit::truncate_output_summary(&format!("{}", e));
                            self.audit.record(trace.as_str(), "llm.classify", Some(&inputs_hash), Some(&summary), Some("error"));
                        }
                    }
                    let result = match llm_call {
                        Ok(r) => r,
                        Err(e) => {
                            let reason = format!("LLM 호출 실패: {}", e);
                            warn!("LLM 분류+가공 실패 → quarantine: {}", reason);
                            self.notification.send("LLM 호출 실패", &reason, "error").await?;
                            let fname_str = file_path.file_name()
                                .and_then(|n| n.to_str()).unwrap_or("unknown").to_string();
                            self.summary.lock().expect("mutex poisoned").record_error(
                                &fname_str, &reason, "quarantine 이동, LLM 백엔드 확인 필요",
                            );
                            self.metrics_error();
                            self.metrics_quarantine();
                            let _ = std::fs::create_dir_all(&self.quarantine_dir);
                            if let Some(f) = file_path.file_name() {
                                let dest = self.quarantine_dir.join(f);
                                let _ = std::fs::copy(file_path, &dest);
                                let _ = std::fs::remove_file(file_path);
                                info!("quarantine 이동: {:?}", dest);
                            }
                            return Ok(());
                        }
                    };
                    info!("문서 유형: [{}] ({})", result.doc_types.join(", "), result.rationale);
                    // 토큰 사용 추정 기록 (text.len() / 4)
                    {
                        let input_tokens = (preprocessed_text.as_ref().map(|t| t.len()).unwrap_or(0) / 4) as u64;
                        let output_tokens = (result.content.len() / 4) as u64;
                        self.token_usage.lock().expect("token_usage mutex").record("classify", "", input_tokens, output_tokens);
                    }
                    llm_result = Some(result);
                }
                PipelineStep::Verify { enabled, thresholds, .. } => {
                    if !enabled {
                        info!("검증 비활성화 (파이프라인 설정)");
                        continue;
                    }
                    if let Some(ref mut result) = llm_result {
                        self.emit_progress(&format!("{{\"event\":\"step\",\"file\":\"{}\",\"stage\":\"verify\"}}", fname));
                        let original_text = preprocessed_text.as_deref()
                            .unwrap_or(&Self::read_text(file_path).unwrap_or_default())
                            .to_string();
                        let required_sections = self.registry.sections_for_types(&result.doc_types);
                        let step_thresholds = thresholds.clone()
                            .or_else(|| self.registry.thresholds_for_types(&result.doc_types))
                            .or_else(|| self.global_thresholds.clone())
                            .unwrap_or_default();
                        let verification = crate::domain::verification::verify_with_thresholds(
                            &original_text, &result.content, &required_sections,
                            &result.metadata.keywords, result.sections.as_ref(), &step_thresholds,
                        );

                        // 검증 메트릭 기록
                        {
                            let doc_id = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("unknown").to_string();
                            let overall_str = match &verification.overall {
                                VerificationLevel::Pass => "pass",
                                VerificationLevel::Warning(_) => "warning",
                                VerificationLevel::Fail(_) => "fail",
                            };
                            self.summary.lock().expect("mutex poisoned").verification_metrics.push(
                                VerificationMetricEntry {
                                    doc_id, timestamp: chrono::Local::now().to_rfc3339(),
                                    structure: verification.structure_completeness,
                                    compression: verification.compression_ratio,
                                    keyword_coverage: verification.keyword_coverage,
                                    keyword_completeness: verification.keyword_completeness,
                                    rouge_l: verification.rouge_l_recall,
                                    entity: verification.entity_preservation,
                                    overall: overall_str.to_string(),
                                }
                            );
                        }

                        match &verification.overall {
                            VerificationLevel::Fail(reason) => {
                                warn!("파이프라인 검증 실패 (1차): {} → 피드백 재가공", reason);
                                let feedback = format!(
                                    "이전 가공 결과에서 문제 발견:\n{}\n위 문제를 보완하여 다시 가공하세요.",
                                    verification.details.join("\n")
                                );
                                // Phase 97 A3 확장 — verify reprocess stage (별도 trace_id)
                                let verify_trace = crate::audit::TraceId::new();
                                let verify_inputs_hash = crate::audit::input_hash_prefix(file_path.to_string_lossy().as_bytes());
                                let verify_reprocess_call = verify_llm.reprocess_with_feedback(file_path, &self.registry, &feedback).await;
                                match &verify_reprocess_call {
                                    Ok(r) => {
                                        let summary = crate::audit::truncate_output_summary(
                                            &format!("types={:?} keywords={}", r.doc_types, r.metadata.keywords.len())
                                        );
                                        self.audit.record(verify_trace.as_str(), "llm.verify_reprocess", Some(&verify_inputs_hash), Some(&summary), Some("success"));
                                    }
                                    Err(e) => {
                                        let summary = crate::audit::truncate_output_summary(&format!("{}", e));
                                        self.audit.record(verify_trace.as_str(), "llm.verify_reprocess", Some(&verify_inputs_hash), Some(&summary), Some("error"));
                                    }
                                }
                                match verify_reprocess_call {
                                    Ok(retry_result) => {
                                        // 2차 검증
                                        let retry_sections = self.registry.sections_for_types(&retry_result.doc_types);
                                        let retry_thresholds = self.registry.thresholds_for_types(&retry_result.doc_types)
                                            .or_else(|| self.global_thresholds.clone())
                                            .unwrap_or_default();
                                        let retry_verification = crate::domain::verification::verify_with_thresholds(
                                            &original_text, &retry_result.content, &retry_sections,
                                            &retry_result.metadata.keywords, retry_result.sections.as_ref(), &retry_thresholds,
                                        );
                                        match &retry_verification.overall {
                                            VerificationLevel::Fail(reason2) => {
                                                warn!("파이프라인 검증 실패 (2차): {} → quarantine", reason2);
                                                let _ = self.notification.send("검증 2차 실패", reason2, "error").await;
                                                let fname_str = fname.to_string();
                                                self.summary.lock().expect("mutex poisoned").record_error(
                                                    &fname_str, reason2, "quarantine 이동, 수동 확인 필요",
                                                );
                                                self.metrics_verify(false);
                                                self.metrics_error();
                                                self.metrics_quarantine();
                                                let _ = std::fs::create_dir_all(&self.quarantine_dir);
                                                if let Some(f) = file_path.file_name() {
                                                    let dest = self.quarantine_dir.join(f);
                                                    let _ = std::fs::copy(file_path, &dest);
                                                    info!("quarantine 이동: {:?}", dest);
                                                }
                                                return Ok(());
                                            }
                                            _ => {
                                                info!("2차 가공으로 검증 통과");
                                                self.metrics_verify(true);
                                                *result = retry_result;
                                            }
                                        }
                                    }
                                    Err(e) => warn!("피드백 재가공 실패: {}", e),
                                }
                            }
                            VerificationLevel::Warning(reason) => {
                                warn!("검증 경고: {}", reason);
                                self.metrics_verify(true);
                            }
                            VerificationLevel::Pass => {
                                info!("검증 통과");
                                self.metrics_verify(true);
                            }
                        }
                    }
                }
                PipelineStep::Embedding { model, .. } => {
                    if let Some(ref m) = model {
                        embedding_model_override = Some(m.clone());
                        info!("Embedding 스텝: 모델 오버라이드 → {}", m);
                    } else {
                        info!("Embedding 스텝 (글로벌 설정 사용)");
                    }
                }
                PipelineStep::Storage { zstd_level } => {
                    zstd_level_override = Some(*zstd_level);
                    info!("Storage 스텝: zstd_level={}", zstd_level);
                }
            }
        }

        // === LLM 결과가 없으면 실패 ===
        let result = llm_result.context("파이프라인에 LLM 스텝 결과가 없습니다")?;
        let original_text = preprocessed_text.unwrap_or_else(|| Self::read_text(file_path).unwrap_or_default());
        let doc_types_str = result.doc_types.join(", ");

        // === 공통 후처리 (항상 실행) ===

        // 구조화된 임베딩 입력 (메타데이터 컨텍스트 포함)
        let embed_input = format!(
            "유형: {}\n키워드: {}\n요약: {}\n\n{}",
            result.doc_types.join(", "),
            result.metadata.keywords.join(", "),
            result.metadata.summary,
            result.content
        );
        // instruction prefix 적용
        let embed_text = if let Some(ref prefix) = self.embed_instruction_prefix {
            format!("{} {}", prefix, embed_input)
        } else {
            embed_input
        };
        // 임베딩 (모델 오버라이드 적용)
        let embedding = if let Some(ref model) = embedding_model_override {
            self.embedding.embed_with_model(&embed_text, model).await?
        } else {
            self.embedding.embed(&embed_text).await?
        };

        // 의미 중복 체크
        let similar_docs = self.vector_db.search_similar(&embedding, 1)?;
        if let Some(top) = similar_docs.first() {
            let distance = 1.0 - top.score;
            if distance < self.semantic_dup_threshold {
                info!("의미 중복 탐지 (거리 {:.4}): {:?}", distance, top.path);
                let existing_text = self.storage.decompress_temp(&top.path)
                    .and_then(|p| std::fs::read_to_string(&p).map_err(Into::into))
                    .unwrap_or_default();
                let diff = render_diff(&top.path.to_string_lossy(), &file_path.to_string_lossy(), &existing_text, &result.content);
                let action = self.duplicate_resolution.resolve(file_path, &top.path, &diff, "의미 중복").await?;
                if action == DuplicateAction::Skip { return Ok(()); }
                self.notification.send_duplicate_alert(&file_path.to_string_lossy(), "의미 중복", &diff).await?;
            }
        }

        // [제거됨] Todo 병합/이월 — 신규 todo 시스템으로 대체 (Phase 53)

        // 가공본 저장 — 순수 본문만 (메타데이터는 벡터DB에만 저장)
        let full_content = result.content.clone();
        let temp_processed = self.processed_dir.join(format!(
            "{}_{}.txt",
            result.doc_types.first().map(|s| s.as_str()).unwrap_or("etc"),
            file_path.file_stem().unwrap_or_default().to_string_lossy()
        ));
        std::fs::write(&temp_processed, &full_content)?;

        let compressed_processed = if let Some(level) = zstd_level_override {
            self.storage.compress_with_level(&temp_processed, &self.processed_dir, level)?
        } else {
            self.storage.compress_and_store(&temp_processed, &self.processed_dir)?
        };
        let _ = std::fs::remove_file(&temp_processed);

        let compressed_origin = self.storage.compress_and_store(file_path, &self.originals_dir)?;
        let _ = std::fs::remove_file(file_path);

        // .vec 파일
        let vec_path = compressed_processed.with_extension("vec");
        crate::domain::vec_io::save_vec(&vec_path, &embedding)?;

        // 교차참조용 메타데이터 복사 (upsert에서 metadata가 이동되므로)
        let crossref_date = result.metadata.date.clone();
        let crossref_keywords = result.metadata.keywords.clone();
        let crossref_doc_types = result.doc_types.clone();
        let crossref_content = result.content.clone();
        let llm_entities = result.metadata.entities.clone();

        // 벡터 DB 색인
        let doc = Document {
            origin_path: file_path.to_path_buf(),
            compressed_origin: Some(compressed_origin),
            processed_path: Some(compressed_processed),
            metadata: Some(result.metadata),
            file_hash: hash.clone(),
            embedding: embedding.clone(),
        };
        self.vector_db.upsert(&doc)?;

        // 원격 저장소 업로드 — Phase 95 A3 확장: 업로드 결과 audit_trace 기록
        if self.remote_storage.is_configured() {
            let backend = self.remote_storage.capabilities().backend;
            if let Some(ref processed) = doc.processed_path {
                let key = format!("processed/{}", processed.file_name().unwrap_or_default().to_string_lossy());
                let trace = crate::audit::TraceId::new();
                let inputs_hash = crate::audit::input_hash_prefix(key.as_bytes());
                match self.remote_storage.upload(processed, &key).await {
                    Ok(()) => self.audit.record(trace.as_str(), &format!("remote.{}.upload.processed", backend), Some(&inputs_hash), Some(&key), Some("success")),
                    Err(e) => {
                        warn!("원격 저장소 업로드 실패 (가공본): {}", e);
                        let summary = crate::audit::truncate_output_summary(&format!("{}", e));
                        self.audit.record(trace.as_str(), &format!("remote.{}.upload.processed", backend), Some(&inputs_hash), Some(&summary), Some("error"));
                    }
                }
            }
            if let Some(ref origin) = doc.compressed_origin {
                let key = format!("originals/{}", origin.file_name().unwrap_or_default().to_string_lossy());
                let trace = crate::audit::TraceId::new();
                let inputs_hash = crate::audit::input_hash_prefix(key.as_bytes());
                match self.remote_storage.upload(origin, &key).await {
                    Ok(()) => self.audit.record(trace.as_str(), &format!("remote.{}.upload.origin", backend), Some(&inputs_hash), Some(&key), Some("success")),
                    Err(e) => {
                        warn!("원격 저장소 업로드 실패 (원본): {}", e);
                        let summary = crate::audit::truncate_output_summary(&format!("{}", e));
                        self.audit.record(trace.as_str(), &format!("remote.{}.upload.origin", backend), Some(&inputs_hash), Some(&summary), Some("error"));
                    }
                }
            }
        }

        // 교차참조 → 비동기 큐에 추가 (정해진 간격마다 배치 실행)
        if self.crossref_mode != "off" {
            let item = CrossRefQueueItem {
                doc_id: hash.clone(),
                doc_types: crossref_doc_types.clone(),
                date: crossref_date.clone(),
                keywords: crossref_keywords.clone(),
                embedding: embedding.clone(),
                priority: 1, // 기본: 보통 (watcher)
            };
            let mut queue = self.crossref_queue.lock().expect("crossref queue poisoned");
            // 이전 항목과 같으면 skip (동일 문서 중복 방지)
            if !queue.iter().any(|q| q.doc_id == item.doc_id) {
                queue.push(item);
            }
        }

        // 엔티티 추출: LLM 응답 우선, 없으면 regex 폴백
        let extracted_entities = if !llm_entities.is_empty() {
            // LLM이 추출한 엔티티 사용 (정확도 높음)
            llm_entities.iter().map(|(name, etype)| {
                use crate::domain::models::{Entity, EntityType};
                let et = match etype.as_str() {
                    "person" => EntityType::Person,
                    "organization" => EntityType::Organization,
                    "place" => EntityType::Place,
                    "technology" => EntityType::Technology,
                    "amount" => EntityType::Amount,
                    "project" => EntityType::Project,
                    "concept" => EntityType::Concept,
                    _ => EntityType::Concept,
                };
                Entity {
                    id: format!("{}_{:x}", etype, {
                        use std::hash::{Hash, Hasher};
                        let mut h = std::collections::hash_map::DefaultHasher::new();
                        name.to_lowercase().hash(&mut h); h.finish()
                    }),
                    name: name.clone(),
                    entity_type: et,
                    doc_ids: vec![hash.clone()],
                    mention_count: 1,
                    first_seen: crossref_date.clone(),
                }
            }).collect::<Vec<_>>()
        } else {
            // regex 폴백 (LLM이 entities를 반환하지 않은 경우)
            crate::domain::cross_reference::CrossRefUpdater::extract_entities(
                &crossref_content, &hash, &crossref_date,
            )
        };
        if !extracted_entities.is_empty() {
            info!("엔티티: {} 건 [{}] ({})", extracted_entities.len(),
                if llm_entities.is_empty() { "regex" } else { "LLM" },
                extracted_entities.iter().map(|e| format!("{}:{}", e.entity_type, e.name)).take(5).collect::<Vec<_>>().join(", "));
            for entity in &extracted_entities {
                let _ = self.vector_db.upsert_entity(entity);
            }
        }

        let stats = self.vector_db.stats()?;
        self.notification.send_completion(&file_path.to_string_lossy(), &doc_types_str, &stats).await?;

        // 증분 컴파일 상태
        {
            let mut state = self.compile_state.lock().expect("mutex poisoned");
            let file_key = file_path.to_string_lossy().to_string();
            state.record_compile(&file_key, &hash, original_text.len() as u64, result.content.len() as u64);
            if !self.compile_state_batch.load(std::sync::atomic::Ordering::Relaxed) {
                let _ = state.save(&self.compile_state_path);
            }
        }

        self.summary.lock().expect("mutex poisoned").record_success(&result.doc_types);
        self.metrics_success();
        self.metrics_time(metrics_t_start);
        self.emit_progress(&format!("{{\"event\":\"done\",\"file\":\"{}\",\"types\":\"{}\"}}", fname, doc_types_str));
        info!("파이프라인 처리 완료: {:?}", file_path);
        Ok(())
    }

    /// 누적된 처리 요약을 알림으로 전송하고 초기화
    pub async fn flush_summary(&self) -> Result<()> {
        let summary: ProcessingSummary = {
            let mut s = self.summary.lock().expect("mutex poisoned");
            std::mem::take(&mut *s)
        };
        if !summary.is_empty() {
            self.notification.send_summary(&summary).await?;
        }
        Ok(())
    }

    /// 민감 파일 처리 플로우
    /// Fragment 처리: 짧은 메모를 LLM 스킵하고 직접 색인
    async fn handle_fragment(&self, file_path: &Path, content: &str) -> Result<()> {
        let hash = Self::compute_hash(file_path)?;
        if self.vector_db.find_by_hash(&hash)?.is_some() {
            self.summary.lock().expect("mutex poisoned").duplicates += 1;
            return Ok(());
        }

        let fname = file_path.file_name().and_then(|n| n.to_str()).unwrap_or("fragment");
        info!("Fragment 색인 (LLM 스킵): {} ({} 자)", fname, content.len());
        self.emit_progress(&format!("{{\"event\":\"fragment\",\"file\":\"{}\"}}", fname));

        // 키워드: 원문 공백 분리
        let keywords: Vec<String> = content.split_whitespace()
            .filter(|w| w.chars().count() >= 2)
            .take(10)
            .map(String::from)
            .collect();

        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let metadata = Metadata {
            doc_types: vec!["fragment".into()],
            rationale: "짧은 메모 (LLM 스킵)".into(),
            date: today,
            summary: content.chars().take(50).collect(),
            keywords: keywords.clone(),
            sensitive: false, doi: None,
            related_docs: vec![], source_doc_ids: vec![], search_hints: vec![],
            entities: vec![],
            ..Default::default()
        };

        // 임베딩 생성
        let embedding = self.embedding.embed(content).await?;

        // 압축 저장
        let header = format!("=== META ===\nsource: {}\ndoc_types: fragment\ndate: {}\n=== CONTENT ===\n{}",
            fname, metadata.date, content);
        let processed_path = self.processed_dir.join(format!("{}.txt", fname));
        std::fs::write(&processed_path, &header)?;
        let compressed = self.storage.compress_and_store(&processed_path, &self.processed_dir)?;
        let _ = std::fs::remove_file(&processed_path);

        // .vec 저장
        let vec_path = compressed.with_extension("vec");
        crate::domain::vec_io::save_vec(&vec_path, &embedding)?;

        // 벡터 DB 색인
        let doc = Document {
            origin_path: file_path.to_path_buf(),
            compressed_origin: None,
            processed_path: Some(compressed),
            metadata: Some(metadata),
            file_hash: hash,
            embedding,
        };
        self.vector_db.upsert(&doc)?;

        // inbox 원본 삭제
        let _ = std::fs::remove_file(file_path);

        self.summary.lock().expect("mutex poisoned").record_success(&["fragment".into()]);
        self.metrics_success();
        info!("Fragment 색인 완료: {}", fname);
        Ok(())
    }

    async fn handle_sensitive(&self, file_path: &Path, reason: &str) -> Result<()> {
        warn!("민감 파일 감지: {:?} ({})", file_path, reason);
        self.summary.lock().expect("mutex poisoned").sensitive += 1;

        self.notification
            .send_sensitive_alert(&file_path.to_string_lossy(), reason)
            .await?;

        let metadata = self
            .sensitive_notification
            .notify_and_collect(file_path, reason)
            .await?;

        match metadata {
            Some(meta) => {
                let text = Self::read_text(file_path).unwrap_or_default();
                let embedding = self.embedding.embed(&text).await?;

                let dest = self.sensitive_dir.join(
                    file_path.file_name().unwrap_or_default(),
                );
                std::fs::copy(file_path, &dest)?;
                let _ = std::fs::remove_file(file_path);

                // 임베딩 벡터 파일 저장 (.vec)
                let vec_path = dest.with_extension("vec");
                crate::domain::vec_io::save_vec(&vec_path, &embedding)?;

                let doc = Document {
                    origin_path: dest.clone(),
                    compressed_origin: None,
                    processed_path: None,
                    metadata: Some(meta),
                    file_hash: Self::compute_hash(&dest)?,
                    embedding,
                };
                self.vector_db.upsert(&doc)?;
                info!("민감 파일 색인 완료: {:?}", dest);
            }
            None => {
                info!("민감 파일 건너뜀: {:?}", file_path);
            }
        }

        Ok(())
    }


    /// compile_state 배치 모드 시작 — save() 호출을 스킵
    pub fn compile_state_batch_begin(&self) {
        self.compile_state_batch.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    /// compile_state 배치 모드 종료 — 1회 저장
    pub fn compile_state_batch_end(&self) {
        self.compile_state_batch.store(false, std::sync::atomic::Ordering::Relaxed);
        let state = self.compile_state.lock().expect("mutex poisoned");
        let _ = state.save(&self.compile_state_path);
    }

    /// PII 패턴 핫 리로드 — settings.db 변경 후 호출하면 다음 가공부터 새 패턴이 적용됨.
    /// 재시작 불필요. 무효 regex는 호출 측에서 사전 검증해야 함 (add_user_pii_pattern은 이미 검증).
    pub fn reload_pii_patterns(&self, patterns: Vec<(String, String)>) -> Result<usize> {
        let mut guard = self.pii_user_patterns.write()
            .map_err(|e| anyhow::anyhow!("pii_user_patterns lock poisoned: {}", e))?;
        *guard = patterns;
        Ok(guard.len())
    }

    /// 교차참조 배치 실행 — EmbeddingSnapshot 행렬 곱 기반
    /// 큐에 쌓인 항목을 일괄 처리. N×search_similar → 1회 스냅샷 cosine.
    pub fn flush_crossref(&self) -> Result<usize> {
        // 간격 체크
        {
            let mut last_run = self.crossref_last_run.lock().expect("mutex poisoned");
            if let Some(last) = *last_run {
                if last.elapsed().as_secs() < self.crossref_interval_secs {
                    return Ok(0);
                }
            }
            *last_run = Some(std::time::Instant::now());
        }

        let mut items: Vec<CrossRefQueueItem> = {
            let mut queue = self.crossref_queue.lock().expect("mutex poisoned");
            std::mem::take(&mut *queue)
        };
        items.sort_by_key(|i| i.priority);

        if items.is_empty() {
            return Ok(0);
        }

        let n = items.len();
        info!("교차참조 배치 시작 (행렬곱): {} 건", n);
        let t_total = std::time::Instant::now();

        // === 1. 스냅샷 구축 ===
        let t_snap = std::time::Instant::now();
        let emb_snapshot = self.vector_db.embedding_snapshot()?;
        if emb_snapshot.is_empty() { return Ok(0); }
        let m = emb_snapshot.len();
        let dim = emb_snapshot.dim;

        // 전체 문서 메타 (doc_types, date, keywords) 수집
        let all_docs = self.vector_db.list_all()?;
        let doc_meta: std::collections::HashMap<&str, &crate::domain::models::StoredDocSummary> =
            all_docs.iter().map(|d| (d.id.as_str(), d)).collect();

        let kw_snapshot: std::collections::HashMap<String, Vec<String>> = all_docs.iter()
            .map(|d| (d.id.clone(), self.vector_db.get_keywords(&d.id).unwrap_or_default()))
            .collect();

        let mut existing_map: std::collections::HashMap<String, std::collections::HashSet<String>> =
            std::collections::HashMap::new();
        for item in &items {
            let rels = self.vector_db.find_related(&item.doc_id).unwrap_or_default();
            existing_map.insert(item.doc_id.clone(), rels.into_iter().map(|r| r.target_id).collect());
        }

        // incoming count (mutual top-K용) — cap_incoming > 0일 때만 수집
        let incoming_cap = self.crossref_cap_incoming;
        let mut incoming_count: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        if incoming_cap > 0 {
            for doc in &all_docs {
                let rels = self.vector_db.find_related(&doc.id).unwrap_or_default();
                let incoming = rels.iter().filter(|r| r.target_id == doc.id).count();
                incoming_count.insert(doc.id.clone(), incoming);
            }
        }
        let d_snap = t_snap.elapsed();

        // === 2. 행렬 곱: 각 새 문서 × (snapshot + flushed) cosine ===
        let t_search = std::time::Instant::now();
        let threshold = self.crossref_similarity_threshold;
        let flushed = self.vector_db.get_flushed_embeddings();
        let snapshot_ids: std::collections::HashSet<&str> = emb_snapshot.ids.iter().map(|s| s.as_str()).collect();

        // MinHash 활성 판단 (force OR 자동 임계치 도달)
        let minhash_active = self.vector_db
            .minhash_enabled_with(self.crossref_minhash_force, self.crossref_minhash_min_docs);
        let metadata_blocking = self.crossref_metadata_blocking;

        let sim_results: Vec<Vec<(String, f32)>> = items.iter().map(|item| {
            if item.embedding.len() != dim { return vec![]; }
            let q = &item.embedding;
            let mut candidates: Vec<(String, f32)> = Vec::new();

            // MinHash 후보 set (활성 시): 자카드 유사 키워드를 가진 문서만 비교
            let mh_set: Option<std::collections::HashSet<String>> = if minhash_active {
                Some(self.vector_db.minhash_candidates(&item.keywords).into_iter().collect())
            } else {
                None
            };
            let item_kw_set: std::collections::HashSet<&String> = item.keywords.iter().collect();

            // vs snapshot (기존 문서)
            for j in 0..m {
                let cand_id = &emb_snapshot.ids[j];
                if cand_id == &item.doc_id { continue; }
                if let Some(ref s) = mh_set {
                    if !s.contains(cand_id.as_str()) { continue; }
                }
                if metadata_blocking
                    && !meta_block_pass(item, cand_id.as_str(), &doc_meta, &kw_snapshot, &item_kw_set) {
                    continue;
                }
                let d = emb_snapshot.get(j);
                let score = cosine_sim_inline(q, d);
                if score >= threshold {
                    candidates.push((cand_id.clone(), score));
                }
            }

            // vs flushed (이전 flush 문서, refresh 전 — snapshot에 없는 것만)
            for (fid, femb) in &flushed {
                if *fid == item.doc_id || femb.len() != dim { continue; }
                if snapshot_ids.contains(fid.as_str()) { continue; }
                if let Some(ref s) = mh_set {
                    if !s.contains(fid.as_str()) { continue; }
                }
                if metadata_blocking
                    && !meta_block_pass(item, fid.as_str(), &doc_meta, &kw_snapshot, &item_kw_set) {
                    continue;
                }
                let score = cosine_sim_inline(q, femb);
                if score >= threshold {
                    candidates.push((fid.clone(), score));
                }
            }

            candidates
        }).collect();

        let d_search = t_search.elapsed();

        // === 3. link 판정 (기존 로직 재사용) ===
        let t_link = std::time::Instant::now();
        self.vector_db.batch_begin();
        let mut total_links = 0u64;
        let sup_threshold = self.crossref_supersedes_threshold;
        let kw_min = self.crossref_keyword_overlap_min;

        for (i, candidates) in sim_results.iter().enumerate() {
            let item = &items[i];
            let existing = existing_map.get(&item.doc_id);
            let new_kw: std::collections::HashSet<&String> = item.keywords.iter().collect();

            let mut cnt_sup = 0usize;
            let mut cnt_upd = 0usize;
            let mut cnt_rel = 0usize;
            let mut cnt_ref = 0usize;

            // score 내림차순 정렬
            let mut sorted: Vec<(String, f32)> = candidates.clone();
            sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

            for (cand_id, score) in &sorted {
                let score = *score;
                if let Some(ex) = existing {
                    if ex.contains(cand_id.as_str()) { continue; }
                }

                // mutual top-K: 대상의 incoming이 cap을 넘으면 skip
                if incoming_cap > 0 {
                    let cand_incoming = incoming_count.get(cand_id.as_str()).copied().unwrap_or(0);
                    if cand_incoming >= incoming_cap { continue; }
                }

                let cand_meta = doc_meta.get(cand_id.as_str());
                let cand_types: Vec<String> = cand_meta.map(|d| d.doc_types.clone()).unwrap_or_default();
                let cand_date_str = cand_meta.map(|d| d.date.as_str()).unwrap_or("");

                let same_type = cand_types.iter().any(|t| item.doc_types.contains(t));
                let new_date = crate::domain::models::DocDate::from_string(&item.date);
                let cand_date = crate::domain::models::DocDate::from_string(cand_date_str);
                let dates_reliable = crate::domain::models::DocDate::both_reliable(&new_date, &cand_date);
                let same_date = dates_reliable && cand_date_str == item.date;

                // Supersedes
                if score > sup_threshold && same_type && dates_reliable && cnt_sup < self.crossref_cap_supersedes {
                    let _ = self.vector_db.link(&item.doc_id, cand_id, crate::domain::models::RelationType::Supersedes);
                    let _ = self.vector_db.link(cand_id, &item.doc_id, crate::domain::models::RelationType::References);
                    total_links += 2;
                    cnt_sup += 1;
                    continue;
                }

                // Updates
                if same_type && same_date && cnt_upd < self.crossref_cap_updates {
                    let _ = self.vector_db.link(&item.doc_id, cand_id, crate::domain::models::RelationType::Updates);
                    let _ = self.vector_db.link(cand_id, &item.doc_id, crate::domain::models::RelationType::Updates);
                    total_links += 2;
                    cnt_upd += 1;
                    continue;
                }

                // RelatedTopic
                let cand_kw = kw_snapshot.get(cand_id.as_str());
                let overlap = cand_kw
                    .map(|kws| kws.iter().filter(|k| new_kw.contains(k)).count())
                    .unwrap_or(0);
                if overlap >= kw_min && cnt_rel < self.crossref_cap_related {
                    let _ = self.vector_db.link(&item.doc_id, cand_id, crate::domain::models::RelationType::RelatedTopic);
                    let _ = self.vector_db.link(cand_id, &item.doc_id, crate::domain::models::RelationType::RelatedTopic);
                    total_links += 2;
                    cnt_rel += 1;
                    continue;
                }

                // References + ReferencedBy
                if score > threshold && cnt_ref < self.crossref_cap_references {
                    let _ = self.vector_db.link(&item.doc_id, cand_id, crate::domain::models::RelationType::References);
                    let _ = self.vector_db.link(cand_id, &item.doc_id, crate::domain::models::RelationType::ReferencedBy);
                    total_links += 2;
                    cnt_ref += 1;
                }

                if cnt_sup >= self.crossref_cap_supersedes && cnt_upd >= self.crossref_cap_updates
                    && cnt_rel >= self.crossref_cap_related && cnt_ref >= self.crossref_cap_references { break; }
            }
        }

        let d_link = t_link.elapsed();
        let t_persist = std::time::Instant::now();
        self.vector_db.batch_end();
        let d_persist = t_persist.elapsed();

        // batch_mode 중이면 flushed_embeddings에 추가 (mmap 미갱신 상태에서 다음 flush 검색용)
        if self.vector_db.flushed_count() > 0 || !items.is_empty() {
            // batch_end에서 mmap이 갱신되므로, flushed는 batch_end 전 상태에서만 의미 있음
            // 여기서는 batch_end가 호출되었으므로 flushed 불필요 (snapshot에 이미 포함)
        }

        // 강제 refresh 체크 (메모리 보호: flushed 10K 초과)
        if self.vector_db.flushed_count() >= 10_000 {
            info!("flushed 임계치 도달 → 강제 db_refresh");
            self.vector_db.db_refresh();
        }

        let d_total = t_total.elapsed();
        let mh_tag = if minhash_active { " minhash=on" } else { "" };
        let mb_tag = if metadata_blocking { " block=on" } else { "" };
        println!(
            "  [flush-matrix] snap={:.1}s search={:.1}s link={:.1}s persist={:.1}s total={:.1}s | {}docs {}links (flushed: {}){}{}",
            d_snap.as_secs_f64(), d_search.as_secs_f64(), d_link.as_secs_f64(),
            d_persist.as_secs_f64(), d_total.as_secs_f64(), n, total_links, self.vector_db.flushed_count(),
            mh_tag, mb_tag
        );
        info!(
            "flush-matrix: snap={:.1}s search={:.1}s link={:.1}s persist={:.1}s total={:.1}s | {}docs {}links",
            d_snap.as_secs_f64(), d_search.as_secs_f64(), d_link.as_secs_f64(),
            d_persist.as_secs_f64(), d_total.as_secs_f64(), n, total_links
        );
        Ok(n)
    }

    /// 교차참조 배치 (레거시: N×search_similar) — 검증용 유지
    #[allow(dead_code)]
    pub fn flush_crossref_legacy(&self) -> Result<usize> {
        // 간격 체크
        {
            let mut last_run = self.crossref_last_run.lock().expect("mutex poisoned");
            if let Some(last) = *last_run {
                if last.elapsed().as_secs() < self.crossref_interval_secs {
                    return Ok(0);
                }
            }
            *last_run = Some(std::time::Instant::now());
        }

        let mut items: Vec<CrossRefQueueItem> = {
            let mut queue = self.crossref_queue.lock().expect("mutex poisoned");
            std::mem::take(&mut *queue)
        };
        items.sort_by_key(|i| i.priority);

        if items.is_empty() {
            return Ok(0);
        }

        let n = items.len();
        info!("교차참조 배치 시작: {} 건", n);
        let t_total = std::time::Instant::now();

        // === 1. 스냅샷 구축 (keywords + 기존 관계) ===
        let t_snap = std::time::Instant::now();
        let all_docs = self.vector_db.list_all()?;
        let m = all_docs.len();
        if m == 0 { return Ok(0); }

        let kw_snapshot: std::collections::HashMap<String, Vec<String>> = all_docs.iter()
            .map(|d| (d.id.clone(), self.vector_db.get_keywords(&d.id).unwrap_or_default()))
            .collect();

        let mut existing_map: std::collections::HashMap<String, std::collections::HashSet<String>> =
            std::collections::HashMap::new();
        for item in &items {
            let rels = self.vector_db.find_related(&item.doc_id).unwrap_or_default();
            existing_map.insert(item.doc_id.clone(), rels.into_iter().map(|r| r.target_id).collect());
        }
        let d_snap = t_snap.elapsed();

        // === 2. search + link ===
        self.vector_db.batch_begin();
        let mut total_links = 0u64;
        let threshold = self.crossref_similarity_threshold;
        let sup_threshold = self.crossref_supersedes_threshold;
        let kw_min = self.crossref_keyword_overlap_min;
        let mut d_search = std::time::Duration::ZERO;
        let mut d_link = std::time::Duration::ZERO;

        for item in &items {
            let ts = std::time::Instant::now();
            let scores = self.vector_db.search_similar(&item.embedding, m)?;
            d_search += ts.elapsed();

            let tl = std::time::Instant::now();
            let existing = existing_map.get(&item.doc_id);
            let new_kw: std::collections::HashSet<&String> = item.keywords.iter().collect();

            let mut cnt_sup = 0usize;
            let mut cnt_upd = 0usize;
            let mut cnt_rel = 0usize;
            let mut cnt_ref = 0usize;

            for candidate in &scores {
                if candidate.id == item.doc_id { continue; }
                if candidate.score < threshold { continue; }
                if let Some(ex) = existing {
                    if ex.contains(&candidate.id) { continue; }
                }

                let same_type = candidate.doc_types.iter()
                    .any(|t| item.doc_types.contains(t));
                let new_date = crate::domain::models::DocDate::from_string(&item.date);
                let cand_date = crate::domain::models::DocDate::from_string(&candidate.date);
                let dates_reliable = crate::domain::models::DocDate::both_reliable(&new_date, &cand_date);
                let same_date = dates_reliable && candidate.date == item.date;

                // Supersedes
                if candidate.score > sup_threshold && same_type && dates_reliable && cnt_sup < 2 {
                    let _ = self.vector_db.link(&item.doc_id, &candidate.id, crate::domain::models::RelationType::Supersedes);
                    let _ = self.vector_db.link(&candidate.id, &item.doc_id, crate::domain::models::RelationType::References);
                    total_links += 2;
                    cnt_sup += 1;
                    continue;
                }

                // Updates
                if same_type && same_date && cnt_upd < self.crossref_cap_updates {
                    let _ = self.vector_db.link(&item.doc_id, &candidate.id, crate::domain::models::RelationType::Updates);
                    let _ = self.vector_db.link(&candidate.id, &item.doc_id, crate::domain::models::RelationType::Updates);
                    total_links += 2;
                    cnt_upd += 1;
                    continue;
                }

                // RelatedTopic (스냅샷에서 조회 — Mutex 없음)
                let cand_kw = kw_snapshot.get(&candidate.id);
                let overlap = cand_kw
                    .map(|kws| kws.iter().filter(|k| new_kw.contains(k)).count())
                    .unwrap_or(0);
                if overlap >= kw_min && cnt_rel < self.crossref_cap_related {
                    let _ = self.vector_db.link(&item.doc_id, &candidate.id, crate::domain::models::RelationType::RelatedTopic);
                    let _ = self.vector_db.link(&candidate.id, &item.doc_id, crate::domain::models::RelationType::RelatedTopic);
                    total_links += 2;
                    cnt_rel += 1;
                    continue;
                }

                // References (양방향: References + ReferencedBy)
                if candidate.score > 0.7 && cnt_ref < 10 {
                    let _ = self.vector_db.link(&item.doc_id, &candidate.id, crate::domain::models::RelationType::References);
                    let _ = self.vector_db.link(&candidate.id, &item.doc_id, crate::domain::models::RelationType::ReferencedBy);
                    total_links += 2;
                    cnt_ref += 1;
                }

                if cnt_sup >= self.crossref_cap_supersedes && cnt_upd >= self.crossref_cap_updates
                    && cnt_rel >= self.crossref_cap_related && cnt_ref >= self.crossref_cap_references { break; }
            }
            d_link += tl.elapsed();
        }

        let t_persist = std::time::Instant::now();
        self.vector_db.batch_end();
        let d_persist = t_persist.elapsed();

        let d_total = t_total.elapsed();
        // プロファイリング出力 (println で確実に表示)
        println!(
            "  [flush] snap={:.1}s search={:.1}s link={:.1}s persist={:.1}s total={:.1}s | {}docs {}links",
            d_snap.as_secs_f64(), d_search.as_secs_f64(), d_link.as_secs_f64(),
            d_persist.as_secs_f64(), d_total.as_secs_f64(), n, total_links
        );
        info!(
            "flush breakdown: snap={:.1}s search={:.1}s link={:.1}s persist={:.1}s total={:.1}s | {}docs {}links",
            d_snap.as_secs_f64(), d_search.as_secs_f64(), d_link.as_secs_f64(),
            d_persist.as_secs_f64(), d_total.as_secs_f64(), n, total_links
        );
        Ok(n)
    }

    /// 교차참조 큐에 대기 중인 항목 수
    pub fn crossref_queue_len(&self) -> usize {
        self.crossref_queue.lock().expect("mutex poisoned").len()
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
