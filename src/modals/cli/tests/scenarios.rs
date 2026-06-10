use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use file_pipeline_adapters::driven::storage::zstd_storage::ZstdStorageAdapter;
use file_pipeline_adapters::driven::vector_db::local_store::LocalVectorStore;
use file_pipeline_core::domain::lint::Linter;
use file_pipeline_core::domain::models::{
    ClassifyAndProcessResult, DocTypeRegistry, Document, EnrichResult, Metadata, RelationType,
};
use file_pipeline_core::domain::wiki_export::WikiExporter;
use file_pipeline_core::ports::output::{LLMPort, StoragePort, VectorDBPort};
use file_pipeline_core::service::FileProcessingService;
use file_pipeline_shared::test_helpers::ServiceBuilder;

fn create_test_dirs() -> (tempfile::TempDir, PathBuf, PathBuf, PathBuf, PathBuf, PathBuf, PathBuf) {
    let base = tempfile::TempDir::new().unwrap();
    let inbox = base.path().join("inbox");
    let processed = base.path().join("processed");
    let originals = base.path().join("originals");
    let sensitive = base.path().join("sensitive");
    let todo = base.path().join("todo");
    let temp = base.path().join(".tmp");
    for d in [&inbox, &processed, &originals, &sensitive, &todo, &temp] {
        std::fs::create_dir_all(d).unwrap();
    }
    (base, inbox, processed, originals, sensitive, todo, temp)
}

/// 검증을 통과하도록 충분히 짧은 가공본을 반환하는 테스트용 LLM
struct TestLlm;

#[async_trait]
impl LLMPort for TestLlm {
    async fn classify_and_process(
        &self,
        file_path: &std::path::Path,
        _registry: &DocTypeRegistry,
    ) -> anyhow::Result<ClassifyAndProcessResult> {
        let content = std::fs::read_to_string(file_path).unwrap_or_default();
        // 압축률 검증 통과: 원본의 30~60% 수준으로 요약
        let summary_content = {
            let chars: Vec<char> = content.chars().collect();
            if chars.len() > 10 {
                chars[..chars.len() / 2].iter().collect::<String>()
            } else {
                content.clone()
            }
        };

        let filename = file_path.file_name().unwrap_or_default().to_string_lossy();
        let metadata = Metadata {
            doc_types: vec!["etc".into()],
            rationale: "test".into(),
            date: chrono::Local::now().format("%Y-%m-%d").to_string(),
            summary: format!("테스트: {}", filename),
            keywords: content
                .split_whitespace()
                .take(10)
                .map(String::from)
                .collect(),
            sensitive: false,
            doi: None,
            related_docs: vec![], source_doc_ids: vec![], search_hints: vec![],
            entities: vec![],
            ..Default::default()        };

        Ok(ClassifyAndProcessResult {
            doc_types: vec!["etc".into()],
            rationale: "test".into(),
            content: summary_content,
            metadata,
            sections: None,
        })
    }

    async fn summarize_text(&self, new_content: &str, existing: &str) -> anyhow::Result<String> {
        Ok(format!("{}\n{}", existing, new_content))
    }

    async fn enrich_existing(
        &self,
        existing_content: &str,
        _new_info: &str,
        _doc_types: &[String],
    ) -> anyhow::Result<EnrichResult> {
        Ok(EnrichResult {
            updated_content: existing_content.to_string(),
            change_summary: String::new(),
            should_update: false,
        })
    }
}

fn build_test_service(base: &Path) -> FileProcessingService {
    ServiceBuilder::new(base)
        .with_llm(Arc::new(TestLlm))
        .with_embedding(Arc::new(file_pipeline_adapters::stub::StubEmbedder::new(1536)))
        .with_semantic_dup_threshold(0.03)
        .with_verification_enabled(true)
        .with_fragment_threshold(0)
        .with_crossref_threshold(0.5)
        .with_crossref_interval(30)
        .build()
}

// ═══════════════════════════════════════════════════════════════
// 시나리오 1: 처음 사용 — 단일 파일 투입
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn scenario_first_use_single_file() {
    let (_base, inbox, processed, originals, _sensitive, _todo, _temp) = create_test_dirs();
    let service = build_test_service(_base.path());

    let test_file = inbox.join("test_note.txt");
    std::fs::write(
        &test_file,
        "오늘 회의에서 프로젝트 일정을 논의했다. 결정사항: 4월 10일까지 완료. 담당: 김철수.",
    )
    .unwrap();

    service.process_file(&test_file).await.unwrap();

    // inbox에서 파일 삭제됨
    assert!(!test_file.exists(), "inbox 파일이 삭제되어야 함");

    // processed에 .zst 파일 생성
    let processed_count = std::fs::read_dir(&processed)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .to_string_lossy()
                .ends_with(".zst")
        })
        .count();
    assert!(processed_count > 0, "가공본 .zst 파일이 있어야 함");

    // originals에 원본 .zst 파일 생성
    let originals_count = std::fs::read_dir(&originals)
        .unwrap()
        .filter_map(|e| e.ok())
        .count();
    assert!(originals_count > 0, "원본 .zst 파일이 있어야 함");

    // 벡터 DB에 1개 문서 등록
    let stats = service.vector_db.stats().unwrap();
    assert_eq!(stats.total_documents, 1);
}

// ═══════════════════════════════════════════════════════════════
// 시나리오 2: 여러 파일 추가 + 완전 중복 스킵
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn scenario_multiple_files() {
    let (_base, inbox, _processed, _originals, _sensitive, _todo, _temp) = create_test_dirs();
    let service = build_test_service(_base.path());

    // 3개 서로 다른 파일
    for (name, content) in [
        ("note1.txt", "첫 번째 문서 내용입니다. 프로젝트 A에 대한 논의."),
        ("note2.txt", "두 번째 문서 내용입니다. Rust 학습 노트 정리."),
        ("note3.txt", "세 번째 문서 내용입니다. 4월 5일 일지 작성."),
    ] {
        let f = inbox.join(name);
        std::fs::write(&f, content).unwrap();
        service.process_file(&f).await.unwrap();
    }

    let stats = service.vector_db.stats().unwrap();
    assert_eq!(stats.total_documents, 3, "3개 문서 등록");

    // inbox 비어있음
    let inbox_count = std::fs::read_dir(&inbox)
        .unwrap()
        .filter_map(|e| e.ok())
        .count();
    assert_eq!(inbox_count, 0, "inbox가 비어있어야 함");

    // 완전 중복: note1과 동일 내용
    let dup = inbox.join("note1_copy.txt");
    std::fs::write(&dup, "첫 번째 문서 내용입니다. 프로젝트 A에 대한 논의.").unwrap();
    service.process_file(&dup).await.unwrap();

    let stats = service.vector_db.stats().unwrap();
    assert_eq!(stats.total_documents, 3, "완전 중복은 스킵되어 여전히 3개");
}

// ═══════════════════════════════════════════════════════════════
// 시나리오 3: 민감 파일 감지
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn scenario_sensitive_file() {
    let (_base, inbox, _processed, _originals, sensitive, _todo, _temp) = create_test_dirs();
    let service = build_test_service(_base.path());

    let file = inbox.join("계약서_2026.txt");
    std::fs::write(&file, "갑: A사\n을: B사\n계약 금액: 1억원").unwrap();

    // StubSensitiveNotification은 Some(기본 Metadata) 반환 → sensitive/ 이동 + 색인
    service.process_file(&file).await.unwrap();

    // 민감 파일이 sensitive 디렉토리로 이동되었는지 확인
    assert!(sensitive.join("계약서_2026.txt").exists(), "민감 파일이 sensitive/로 이동");
    let stats = service.vector_db.stats().unwrap();
    assert_eq!(stats.total_documents, 1, "민감 파일도 최소 메타데이터로 색인됨");
}

// ═══════════════════════════════════════════════════════════════
// 시나리오 4: lint
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn scenario_lint() {
    let (_base, inbox, _processed, _originals, _sensitive, _todo, _temp) = create_test_dirs();
    let service = build_test_service(_base.path());

    for (name, content) in [
        ("a.txt", "문서 A 내용입니다. 충분히 길게 작성합니다."),
        ("b.txt", "문서 B 내용입니다. 다른 주제의 내용입니다."),
    ] {
        let f = inbox.join(name);
        std::fs::write(&f, content).unwrap();
        service.process_file(&f).await.unwrap();
    }

    let report = Linter::lint(service.vector_db.as_ref()).unwrap();
    // stub 임베딩은 모두 제로벡터 → 관계 미생성 → 고아
    assert_eq!(report.orphan_docs.len(), 2, "모든 문서가 고아");
}

// ═══════════════════════════════════════════════════════════════
// 시나리오 5: wiki-export
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn scenario_wiki_export() {
    let (_base, inbox, _processed, _originals, _sensitive, _todo, _temp) = create_test_dirs();
    let service = build_test_service(_base.path());

    let file = inbox.join("export_test.txt");
    std::fs::write(&file, "위키 내보내기 테스트 문서입니다. 충분한 내용.").unwrap();
    service.process_file(&file).await.unwrap();

    let wiki_dir = _base.path().join("wiki");
    let report = WikiExporter::export(
        service.vector_db.as_ref(),
        service.storage.as_ref(),
        &wiki_dir,
    )
    .unwrap();

    assert_eq!(report.total, 1);
    assert!(wiki_dir.join("INDEX.md").exists(), "INDEX.md 생성됨");
}

// 시나리오 6 (purge) 제거됨: purge_expired_originals는 Phase 55에서 제거됨
// (Tauri purge_dry_run/purge_execute로 대체, Settings UI에서 호출)

// ═══════════════════════════════════════════════════════════════
// 단위 테스트: LocalVectorStore 관계 기능
// ═══════════════════════════════════════════════════════════════

#[test]
fn test_sqlite_link_and_find_related() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db = LocalVectorStore::with_path(tmp.path().join(".local-store.json"));
    db.init().unwrap();

    db.link("doc_a", "doc_b", RelationType::References).unwrap();
    db.link("doc_a", "doc_c", RelationType::RelatedTopic).unwrap();
    db.link("doc_b", "doc_a", RelationType::References).unwrap();

    let related = db.find_related("doc_a").unwrap();
    assert_eq!(related.len(), 2);

    let related_b = db.find_related("doc_b").unwrap();
    assert_eq!(related_b.len(), 1);
    assert_eq!(related_b[0].target_id, "doc_a");
}

#[test]
fn test_sqlite_no_duplicate_links() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db = LocalVectorStore::with_path(tmp.path().join(".local-store.json"));
    db.link("a", "b", RelationType::References).unwrap();
    db.link("a", "b", RelationType::References).unwrap();

    let related = db.find_related("a").unwrap();
    assert_eq!(related.len(), 1);
}

#[test]
fn test_sqlite_update_types() {
    let tmp = tempfile::TempDir::new().unwrap();
    let db = LocalVectorStore::with_path(tmp.path().join(".local-store.json"));
    db.init().unwrap();

    let doc = Document {
        origin_path: "test.txt".into(),
        compressed_origin: None,
        processed_path: None,
        metadata: Some(Metadata {
            doc_types: vec!["meeting".into()],
            rationale: "test".into(),
            date: "2026-04-05".into(),
            summary: "test".into(),
            keywords: vec![],
            sensitive: false,
            doi: None,
            related_docs: vec![], source_doc_ids: vec![], search_hints: vec![],
            entities: vec![],
            ..Default::default()        }),
        file_hash: "abc123".into(),
        embedding: vec![0.0; 10],
    };
    db.upsert(&doc).unwrap();

    db.update_types("abc123", vec!["study".into(), "log".into()]).unwrap();
    let types = db.get_types("abc123").unwrap();
    assert_eq!(types, vec!["study", "log"]);
}

// ═══════════════════════════════════════════════════════════════
// 단위 테스트: zstd 라운드트립 + 헤더
// ═══════════════════════════════════════════════════════════════

#[test]
fn test_zstd_roundtrip_with_header() {
    let temp = tempfile::TempDir::new().unwrap();
    let storage = ZstdStorageAdapter::new(3, temp.path().join(".tmp"));

    let src_dir = temp.path().join("src");
    let dest_dir = temp.path().join("dest");
    std::fs::create_dir_all(&src_dir).unwrap();

    let content = "=== META ===\nsource: test.txt\ntype: meeting\n=== CONTENT ===\n\n본문 내용";
    let src_file = src_dir.join("test.txt");
    std::fs::write(&src_file, content).unwrap();

    let compressed = storage.compress_and_store(&src_file, &dest_dir).unwrap();
    let header = storage.read_header(&compressed, 10).unwrap();
    assert!(header.contains("=== META ==="));
    assert!(header.contains("source: test.txt"));

    let decompressed = storage.decompress_temp(&compressed).unwrap();
    let roundtrip = std::fs::read_to_string(&decompressed).unwrap();
    assert_eq!(roundtrip, content);
}

// ═══════════════════════════════════════════════════════════════
// 단위 테스트: DocTypeRegistry
// ═══════════════════════════════════════════════════════════════

#[test]
fn test_registry_sections_for_types_dedup() {
    use file_pipeline_core::domain::models::{DocTypeDef, DocTypeRegistry};

    let reg = DocTypeRegistry::new(vec![
        DocTypeDef {
            id: "a".into(),
            label_ko: "A".into(),
            patterns: vec![],
            sections: vec!["공통".into(), "A전용".into()],
            prompt: String::new(),
            dedup_key: None,
            sensitive: false,
            thresholds: None,
        },
        DocTypeDef {
            id: "b".into(),
            label_ko: "B".into(),
            patterns: vec![],
            sections: vec!["공통".into(), "B전용".into()],
            prompt: String::new(),
            dedup_key: None,
            sensitive: false,
            thresholds: None,
        },
    ]);

    let sections = reg.sections_for_types(&["a".into(), "b".into()]);
    // "공통"은 중복 제거되어 1번만
    assert_eq!(sections.len(), 3); // 공통, A전용, B전용
}
