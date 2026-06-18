//! `ProcessFileUseCase` 영역 — step-s4 (2026-06-16, hex-arch-d) 정합.
//!
//! `FileProcessingService` 의 단일 파일 가공 경로 4 함수를 별도 impl block 으로 분리.
//! Rust 의 split impl 패턴 — 호출자/필드/lifetime 변경 부재, 본문 그대로 mv.
//!
//! 책임 영역:
//! - `process_file` — 기본 파이프라인 위임
//! - `process_file_with_pipeline` — 핵심 가공 흐름 (전처리 → LLM → verify → 임베딩 → 색인 → 알림)
//! - `handle_fragment` — 짧은 메모 LLM 스킵 직접 색인
//! - `handle_sensitive` — 민감 파일 격리 + 옵션 메타데이터 색인
//!
//! `FileProcessingService` 의 헬퍼 (`emit_progress` / `metrics_*` / `compute_hash` / `read_text`) 는
//! service.rs 잔류 — split impl 안에서 직접 호출 가능 (Rust 의 self/Self 가시성).
//!
//! 본 split impl 적용 사유: process_file_with_pipeline = 520줄 본문 + `self.*` 호출 광범위
//! → MaintenanceUseCase 패턴 (lifetime borrow struct) 적용 시 본문 광역 변환 + 회귀 위험 ↑.
//! 본 cycle 시간 균형 정합 split impl 채택 (lesson #25 큰 변경 지양 + lesson #30 default).
//! 차후 cycle = use case struct 형태로 점진 진화 가능.

use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use tracing::{info, warn};

use crate::domain::deduplicator::render_diff;
use crate::domain::models::{
    ClassifyAndProcessResult, Document, DuplicateAction, Metadata,
    VerificationLevel, VerificationMetricEntry,
};
use crate::ports::output::LLMPort;
use crate::service::{CrossRefQueueItem, FileProcessingService};

impl FileProcessingService {
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
            let backend = crate::ports::output::RemoteStoragePort::capabilities(self.remote_storage.as_ref()).backend;
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

    /// Fragment 처리: 짧은 메모를 LLM 스킵하고 직접 색인
    pub(crate) async fn handle_fragment(&self, file_path: &Path, content: &str) -> Result<()> {
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

    /// 민감 파일 처리 플로우
    pub(crate) async fn handle_sensitive(&self, file_path: &Path, reason: &str) -> Result<()> {
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
}
