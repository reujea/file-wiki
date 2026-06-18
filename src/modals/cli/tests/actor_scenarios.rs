//! Actor 시나리오 테스트
//!
//! 3가지 사용자 유형별 실제 워크플로우 검증:
//! - 일반 사용자: inbox 투입 → 가공 → 검색 → 문서 조회
//! - 관리자: stats → lint → purge → backfill
//! - 개발자: 설정 변경 → Todo 관리 → 메모리 계층

use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use file_pipeline_core::domain::models::*;
use file_pipeline_core::ports::output::*;
use file_pipeline_core::service::FileProcessingService;
use file_pipeline_shared::test_helpers::ServiceBuilder;

// ── 테스트용 LLM ─────────────────────────────────

struct ScenarioLlm;

#[async_trait]
impl LLMPort for ScenarioLlm {
    async fn classify_and_process(&self, file_path: &Path, _registry: &DocTypeRegistry) -> anyhow::Result<ClassifyAndProcessResult> {
        let content = std::fs::read_to_string(file_path).unwrap_or_default();
        let filename = file_path.file_name().unwrap_or_default().to_string_lossy();

        let doc_types = if filename.contains("todo") {
            vec!["todo".into()]
        } else if filename.contains("meeting") {
            vec!["meeting".into()]
        } else {
            vec!["memo".into()]
        };

        let keywords: Vec<String> = content.split_whitespace().take(10).map(String::from).collect();
        let metadata = Metadata {
            doc_types: doc_types.clone(), rationale: "scenario_test".into(),
            date: chrono::Local::now().format("%Y-%m-%d").to_string(),
            summary: format!("시나리오 테스트: {}", filename), keywords,
            sensitive: false, doi: None, related_docs: vec![], source_doc_ids: vec![], search_hints: vec![],
            entities: vec![],
            ..Default::default()        };
        let processed = format!("=== {} ===\n{}", doc_types[0], content);
        Ok(ClassifyAndProcessResult {
            doc_types, rationale: "scenario_test".into(), content: processed, metadata, sections: None,
        })
    }
    async fn summarize_text(&self, new: &str, existing: &str) -> anyhow::Result<String> {
        Ok(format!("{}\n{}", existing, new))
    }
    async fn reprocess_with_feedback(&self, path: &Path, reg: &DocTypeRegistry, _fb: &str) -> anyhow::Result<ClassifyAndProcessResult> {
        self.classify_and_process(path, reg).await
    }
    async fn enrich_existing(&self, existing: &str, _: &str, _: &[String]) -> anyhow::Result<EnrichResult> {
        Ok(EnrichResult { updated_content: existing.into(), change_summary: String::new(), should_update: false })
    }
}

fn setup_service(base: &Path) -> FileProcessingService {
    let registry = DocTypeRegistry::new(vec![
        DocTypeDef { id: "meeting".into(), label_ko: "회의록".into(), patterns: vec!["회의".into()],
            sections: vec!["결정사항".into()], prompt: String::new(), dedup_key: None, sensitive: false, thresholds: None },
        DocTypeDef { id: "todo".into(), label_ko: "할일".into(), patterns: vec!["todo".into()],
            sections: vec!["긴급+중요".into()], prompt: String::new(), dedup_key: Some("date".into()), sensitive: false, thresholds: None },
        DocTypeDef { id: "memo".into(), label_ko: "메모".into(), patterns: vec!["메모".into()],
            sections: vec!["핵심내용".into()], prompt: String::new(), dedup_key: None, sensitive: false, thresholds: None },
    ]);

    ServiceBuilder::new(base)
        .with_llm(Arc::new(ScenarioLlm))
        .with_embedding(Arc::new(file_pipeline_adapters::stub::StubEmbedder::new(128)))
        .with_registry(Arc::new(registry))
        .with_semantic_dup_threshold(0.03)
        .with_fragment_threshold(100)
        .with_crossref_threshold(0.5)
        .with_crossref_interval(30)
        .build()
}

// ═══════════════════════════════════════════════════════════════
// 시나리오 A: 일반 사용자 — 파일 투입 → 가공 → 검색 → 조회
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn scenario_user_basic_workflow() {
    let base = tempfile::TempDir::new().unwrap();
    let service = setup_service(base.path());

    // 1. 파일 투입
    let meeting = base.path().join("inbox/meeting_2026.txt");
    std::fs::write(&meeting, "프로젝트 API 회의 결정사항 Qdrant 유지").unwrap();

    let memo = base.path().join("inbox/memo_note.txt");
    std::fs::write(&memo, "메모: 내일 배포 확인 필요").unwrap();

    // 2. 가공
    service.process_file(&meeting).await.unwrap();
    service.process_file(&memo).await.unwrap();

    // 3. DB에 2건 저장 확인
    let stats = service.vector_db.stats().unwrap();
    assert_eq!(stats.total_documents, 2, "2건 저장되어야 함");

    // 4. 검색
    let dummy_embedding = vec![0.1f32; 128];
    let results = service.vector_db.search_similar(&dummy_embedding, 10).unwrap();
    assert!(!results.is_empty(), "검색 결과가 있어야 함");

    // 5. 문서 목록 조회
    let all = service.vector_db.list_all().unwrap();
    assert_eq!(all.len(), 2);

    // 6. 중복 파일 → 스킵 확인
    let dup = base.path().join("inbox/meeting_dup.txt");
    std::fs::write(&dup, "프로젝트 API 회의 결정사항 Qdrant 유지").unwrap();
    service.process_file(&dup).await.unwrap();
    let stats2 = service.vector_db.stats().unwrap();
    assert_eq!(stats2.total_documents, 2, "중복은 스킵 — 여전히 2건");
}

// ═══════════════════════════════════════════════════════════════
// 시나리오 B: 관리자 — stats → lint → 처리 요약
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn scenario_admin_monitoring() {
    let base = tempfile::TempDir::new().unwrap();
    let service = setup_service(base.path());

    // 3개 파일 처리
    for i in 0..3 {
        let path = base.path().join(format!("inbox/doc_{}.txt", i));
        std::fs::write(&path, format!("문서 {} 내용 프로젝트 API 리뷰", i)).unwrap();
        service.process_file(&path).await.unwrap();
    }

    // 1. stats
    let stats = service.vector_db.stats().unwrap();
    assert_eq!(stats.total_documents, 3);

    // 2. lint
    let report = file_pipeline_core::domain::lint::Linter::lint(service.vector_db.as_ref()).unwrap();
    // 고아 문서가 있을 수 있음 (compressed_origin 없으므로)
    let _ = report.issues.len(); // lint 실행 성공만 확인

    // 3. 처리 요약
    let summary = service.summary.lock().unwrap();
    assert_eq!(summary.success, 3);
    assert!(!summary.is_empty());
}

// ═══════════════════════════════════════════════════════════════
// 시나리오 C: 개발자 — Todo 생명주기
// ═══════════════════════════════════════════════════════════════

#[test]
#[ignore = "todo_lifecycle 모듈 제거됨"]
fn scenario_developer_todo_lifecycle() {
    // todo_lifecycle 모듈이 제거되어 비활성화
}

// ═══════════════════════════════════════════════════════════════
// 시나리오 E: 일반 사용자 — 민감 파일 처리
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn scenario_user_sensitive_file() {
    let base = tempfile::TempDir::new().unwrap();
    let service = setup_service(base.path());

    // 민감 파일 (파일명에 "계약" 포함)
    let sensitive_file = base.path().join("inbox/계약서_2026.txt");
    std::fs::write(&sensitive_file, "비밀 계약 내용").unwrap();
    service.process_file(&sensitive_file).await.unwrap();

    // 민감 파일은 sensitive 디렉토리로 이동됨
    let sensitive_copy = base.path().join("sensitive/계약서_2026.txt");
    assert!(sensitive_copy.exists(), "민감 파일이 sensitive/로 이동되어야 함");

    // summary에 민감 카운트
    let summary = service.summary.lock().unwrap();
    assert_eq!(summary.sensitive, 1);
}

// ═══════════════════════════════════════════════════════════════
// 시나리오 F: 일반 사용자 — 증분 컴파일 (변경 없는 파일 스킵)
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn scenario_incremental_skip() {
    let base = tempfile::TempDir::new().unwrap();
    let service = setup_service(base.path());

    let file = base.path().join("inbox/report.txt");
    std::fs::write(&file, "분기 보고서 내용 프로젝트 현황").unwrap();

    // 1차 처리
    service.process_file(&file).await.unwrap();
    let stats1 = service.vector_db.stats().unwrap();
    assert_eq!(stats1.total_documents, 1);

    // inbox에서 삭제됨 → 다시 생성 (같은 내용)
    std::fs::write(&file, "분기 보고서 내용 프로젝트 현황").unwrap();

    // 2차 처리 → SHA-256 중복으로 스킵
    service.process_file(&file).await.unwrap();
    let stats2 = service.vector_db.stats().unwrap();
    assert_eq!(stats2.total_documents, 1, "같은 내용은 중복 스킵");
}

// step-o2 partial 해소 추가 (2026-06-17)
impl file_pipeline_core::ports::outbound::OutboundManifest for ScenarioLlm {
    fn id(&self) -> &str { "fp-outbound-llm-scenario" }
    fn category(&self) -> file_pipeline_core::ports::outbound::OutboundCategory {
        file_pipeline_core::ports::outbound::OutboundCategory::Llm
    }
    fn capabilities(&self) -> file_pipeline_core::ports::output::ResourceCapabilities {
        file_pipeline_core::ports::output::ResourceCapabilities::standard("scenario")
    }
}
