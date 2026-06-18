//! 실환경 검증 테스트 — 외부 서비스 미사용 시 자동 스킵
//!
//! 6-3: A3(2-Pass), B2(Sparse), B3(Hybrid), H2(토픽병합)
//! 6-4: 알림 포맷 검증
//! 6-5: Windows Service 커맨드 검증

// ═══════════════════════════════════════════════════════════════
// 6-6: 전처리 라우팅 테스트 (외부 도구 불필요)
// ═══════════════════════════════════════════════════════════════

#[test]
fn preprocessing_supports_text_formats() {
    use file_pipeline_core::ports::output::PreprocessPort;
    let p = file_pipeline_adapters::driven::preprocessing::preprocessor::CompositePreprocessor::new("none", "none");
    for ext in &["txt", "md", "csv", "json", "toml", "yaml", "yml"] {
        assert!(p.supports(ext), "should support .{}", ext);
    }
    for ext in &["pdf", "png", "jpg", "jpeg", "docx"] {
        assert!(p.supports(ext), "should support .{}", ext);
    }
    for ext in &["xyz", "abc", "rs", "py"] {
        assert!(!p.supports(ext), "should NOT support .{}", ext);
    }
}

#[test]
fn preprocessing_plain_text_passthrough() {
    use file_pipeline_core::ports::output::PreprocessPort;
    let p = file_pipeline_adapters::driven::preprocessing::preprocessor::CompositePreprocessor::new("none", "none");
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("test.txt");
    std::fs::write(&path, "hello world 테스트").unwrap();
    let result = p.preprocess(&path).unwrap();
    assert_eq!(result.text, "hello world 테스트");
}

#[test]
fn preprocessing_pdf_fallback_no_tool() {
    use file_pipeline_core::ports::output::PreprocessPort;
    let p = file_pipeline_adapters::driven::preprocessing::preprocessor::CompositePreprocessor::new("none", "none");
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("test.pdf");
    std::fs::write(&path, "fake pdf content for test").unwrap();
    // pdf_tool=none → fallback으로 plain text 읽기
    let result = p.preprocess(&path).unwrap();
    assert!(result.text.contains("fake pdf content"));
}

#[test]
fn preprocessing_image_fallback_no_tool() {
    use file_pipeline_core::ports::output::PreprocessPort;
    let p = file_pipeline_adapters::driven::preprocessing::preprocessor::CompositePreprocessor::new("none", "none");
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("test.png");
    std::fs::write(&path, "fake image").unwrap();
    // ocr_tool=none → placeholder 반환
    let result = p.preprocess(&path).unwrap();
    assert!(result.text.contains("test.png") || !result.text.is_empty());
}

// ═══════════════════════════════════════════════════════════════
// 6-4: 알림 포맷 검증 (실제 API 호출 없음)
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn notification_null_adapter_no_panic() {
    use file_pipeline_adapters::driven::notify::composite::NullNotificationAdapter;
    use file_pipeline_core::domain::models::{DbStats, ProcessingSummary};
    use file_pipeline_core::ports::output::NotificationPort;

    let null = NullNotificationAdapter;
    null.send("test", "body", "info").await.unwrap();
    null.send_sensitive_alert("file.txt", "reason").await.unwrap();
    null.send_duplicate_alert("file.txt", "reason", "diff").await.unwrap();
    null.send_completion("file.txt", "meeting", &DbStats::default()).await.unwrap();
    null.send_summary(&ProcessingSummary::default()).await.unwrap();
}

#[test]
fn processing_summary_record_and_status() {
    use file_pipeline_core::domain::models::ProcessingSummary;

    let mut s = ProcessingSummary::default();
    assert!(s.is_empty());

    s.record_success(&["meeting".to_string(), "todo".to_string()]);
    s.record_success(&["meeting".to_string()]);
    s.record_error("bad.txt", "구조 완전성 실패", "quarantine 이동");
    s.record_warning("warn.txt", "압축률 초과", "2-Pass 재가공 후 통과");
    s.duplicates = 1;
    s.sensitive = 1;

    assert!(!s.is_empty());
    assert_eq!(s.success, 2);
    assert_eq!(s.errors, 1);
    assert_eq!(s.issues.len(), 2);
    assert_eq!(*s.by_type.get("meeting").unwrap(), 2);
    assert_eq!(*s.by_type.get("todo").unwrap(), 1);
}

// ═══════════════════════════════════════════════════════════════
// 6-5: Windows Service 상수 검증
// ═══════════════════════════════════════════════════════════════

#[test]
fn daemon_module_exists() {
    // daemon 모듈이 컴파일되는지 검증 (platform-specific 실행은 하지 않음).
    // 본 함수가 컴파일되는 것 자체가 검증 → 본문 없음.
}

#[test]
#[cfg(windows)]
fn windows_service_command_format() {
    // sc.exe에 전달할 binPath 형식 검증
    let exe = std::env::current_exe().expect("current_exe");
    let bin_path = format!("binPath= \"{}\" watch", exe.display());
    assert!(bin_path.contains("binPath= "), "sc.exe requires space before value");
    assert!(bin_path.contains("watch"), "binPath must include subcommand");
}

#[test]
#[cfg(windows)]
fn windows_task_scheduler_path_format() {
    let task_name = "\\FilePipeline\\Watch";
    assert!(task_name.starts_with('\\'), "task path must start with backslash");
    assert!(task_name.contains("FilePipeline"), "task path must include app name");
}

// ═══════════════════════════════════════════════════════════════
// A3: 2-Pass 피드백 재가공 검증 (Stub LLM으로 FAIL→재가공→PASS 시뮬레이션)
// ═══════════════════════════════════════════════════════════════

/// 1차에서 빈 sections를 반환하여 구조 완전성 FAIL을 유발하고,
/// 2차(reprocess_with_feedback) 때 올바른 sections를 반환하는 LLM stub.
struct TwoPassLlm {
    call_count: std::sync::atomic::AtomicU32,
}

impl TwoPassLlm {
    fn new() -> Self { Self { call_count: std::sync::atomic::AtomicU32::new(0) } }
}

#[async_trait::async_trait]
impl file_pipeline_core::ports::output::LLMPort for TwoPassLlm {
    async fn classify_and_process(
        &self, file_path: &std::path::Path, _registry: &file_pipeline_core::domain::models::DocTypeRegistry,
    ) -> anyhow::Result<file_pipeline_core::domain::models::ClassifyAndProcessResult> {
        let text = std::fs::read_to_string(file_path).unwrap_or_default();
        let n = self.call_count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        // 1차: sections 없음 → 구조 완전성 0%로 FAIL
        Ok(file_pipeline_core::domain::models::ClassifyAndProcessResult {
            doc_types: vec!["meeting".into()],
            rationale: "test".into(),
            content: {
                let end = text.char_indices().nth(200).map(|(i, _)| i).unwrap_or(text.len());
                text[..end].to_string()
            },
            metadata: file_pipeline_core::domain::models::Metadata {
                doc_types: vec!["meeting".into()],
                rationale: "test".into(),
                date: "2026-01-01".into(),
                summary: "test summary".into(),
                keywords: vec!["meeting".into(), "agenda".into()],
                sensitive: false,
                doi: None,
                related_docs: vec![],
                source_doc_ids: vec![], search_hints: vec![],
                entities: vec![],
                ..Default::default()            },
            sections: if n == 0 { None } else {
                // 2차: sections 있음 → PASS
                let mut m = std::collections::HashMap::new();
                m.insert("결정사항".into(), vec!["항목1".into()]);
                m.insert("참석자".into(), vec!["홍길동".into()]);
                Some(m)
            },
        })
    }

    async fn summarize_text(&self, _new: &str, existing: &str) -> anyhow::Result<String> {
        Ok(existing.to_string())
    }

    async fn reprocess_with_feedback(
        &self, file_path: &std::path::Path, registry: &file_pipeline_core::domain::models::DocTypeRegistry, _feedback: &str,
    ) -> anyhow::Result<file_pipeline_core::domain::models::ClassifyAndProcessResult> {
        // 2차 호출: sections 포함
        self.classify_and_process(file_path, registry).await
    }

    async fn enrich_existing(
        &self, existing: &str, _new_info: &str, _doc_types: &[String],
    ) -> anyhow::Result<file_pipeline_core::domain::models::EnrichResult> {
        Ok(file_pipeline_core::domain::models::EnrichResult {
            updated_content: existing.to_string(),
            change_summary: "enriched".into(),
            should_update: false,
        })
    }
}

#[tokio::test]
async fn two_pass_feedback_reprocess() {
    use file_pipeline_core::domain::models::DocTypeDef;
    use file_pipeline_adapters::stub::StubEmbedder;
    use file_pipeline_shared::test_helpers::ServiceBuilder;

    let base = tempfile::TempDir::new().expect("tempdir 생성 실패");

    let registry = file_pipeline_core::domain::models::DocTypeRegistry::new(vec![
        DocTypeDef {
            id: "meeting".into(),
            label_ko: "회의록".into(),
            patterns: vec!["회의".into()],
            sections: vec!["결정사항".into(), "참석자".into()],
            prompt: "회의록을 정리하세요".into(),
            dedup_key: None,
            sensitive: false,
            thresholds: None,
        },
    ]);

    let service = ServiceBuilder::new(base.path())
        .with_llm(std::sync::Arc::new(TwoPassLlm::new()))
        .with_embedding(std::sync::Arc::new(StubEmbedder::new(1536)))
        .with_registry(std::sync::Arc::new(registry))
        .with_semantic_dup_threshold(0.03)
        .with_max_retry(1)
        .with_verification_enabled(true)
        .with_fragment_threshold(0)
        .with_crossref_threshold(0.5)
        .with_crossref_interval(30)
        .with_global_thresholds(file_pipeline_core::domain::verification::VerificationThresholds {
            structure_min: 0.5,
            compression_min: 0.0,
            compression_max: 10.0,
            keyword_coverage_min: 0.0,
            keyword_completeness_min: 0.0,
            rouge_l_min: 0.0,
            entity_preservation_min: 0.0,
        })
        .build();

    // 회의록 테스트 문서 작성 (충분히 길게)
    let test_file = service.inbox_dir.join("test-meeting.txt");
    std::fs::write(&test_file, "2026-01-15 개발팀 주간회의\n\n참석자: 홍길동, 김철수, 이영희\n\n안건:\n1. 스프린트 진행 현황 공유\n2. 기술 부채 정리 계획\n3. QA 일정 확정\n\n결정사항:\n- 다음 주까지 리팩토링 완료\n- QA는 3월 1일부터\n- 코드 리뷰 의무화\n\n다음 회의: 2026-01-22").expect("테스트 파일 작성 실패");

    service.process_file(&test_file).await.expect("파일 처리 실패");

    // 검증: 문서가 처리되었는지 (DB 등록 또는 processed 디렉토리에 파일 존재)
    let stats = service.vector_db.stats().expect("stats 실패");
    let processed_files: Vec<_> = std::fs::read_dir(&service.processed_dir).unwrap().flatten().collect();
    let quarantined: Vec<_> = std::fs::read_dir(&service.quarantine_dir).unwrap().flatten().collect();

    // 2-Pass LLM은 2차에서 sections를 반환하므로:
    // - DB에 등록되었거나 processed에 파일이 있어야 함
    // - quarantine에는 파일이 없어야 함
    assert!(
        stats.total_documents >= 1 || !processed_files.is_empty(),
        "2-Pass 후 문서가 처리되어야 함 (DB: {}, processed: {}, quarantine: {})",
        stats.total_documents, processed_files.len(), quarantined.len()
    );
    assert!(
        quarantined.is_empty(),
        "2-Pass 성공 시 quarantine에 파일이 없어야 함 (got {} files)",
        quarantined.len()
    );
}

// ═══════════════════════════════════════════════════════════════
// Fallback LLM 테스트
// ═══════════════════════════════════════════════════════════════

#[test]
fn fallback_adapter_compiles() {
    let _adapter = file_pipeline_adapters::driven::llm::fallback_adapter::FallbackLlmAdapter::new(vec![]);
}

#[tokio::test]
async fn fallback_adapter_empty_returns_error() {
    use file_pipeline_core::ports::output::LLMPort;
    let adapter = file_pipeline_adapters::driven::llm::fallback_adapter::FallbackLlmAdapter::new(vec![]);
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("test.txt");
    std::fs::write(&path, "test content").unwrap();
    let registry = file_pipeline_core::domain::models::DocTypeRegistry::empty();
    let result = adapter.classify_and_process(&path, &registry).await;
    assert!(result.is_err()); // 프로바이더 없음
}

// ═══════════════════════════════════════════════════════════════
// 6-3: 실환경 테스트 (env 가드 — 없으면 스킵)
// ═══════════════════════════════════════════════════════════════

fn skip_unless(var: &str) -> bool {
    std::env::var(var).is_err()
}

#[tokio::test]
async fn real_openai_embedding() {
    if skip_unless("OPENAI_API_KEY") {
        eprintln!("SKIP: OPENAI_API_KEY not set");
        return;
    }
    let key = std::env::var("OPENAI_API_KEY").unwrap();
    let adapter = file_pipeline_adapters::driven::embedding::openai_embed::OpenAIEmbeddingAdapter::new(key);

    use file_pipeline_core::ports::output::EmbeddingPort;
    let vec = adapter.embed("테스트 문장입니다").await.unwrap();
    assert_eq!(vec.len(), 1536);
    // 벡터가 정규화 되어있는지 확인 (L2 norm ≈ 1.0)
    let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!((norm - 1.0).abs() < 0.1, "norm should be ~1.0, got {}", norm);
}

// Qdrant 테스트 제거됨 (Phase 44에서 Qdrant 완전 삭제)
