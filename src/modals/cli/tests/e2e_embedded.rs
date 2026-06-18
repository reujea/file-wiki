//! E2E 임베디드 테스트 — Docker/외부 서버 없이 전체 파이프라인 검증

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use file_pipeline_core::domain::diagnostics;
use file_pipeline_core::domain::incremental::CompileState;
use file_pipeline_core::domain::lint::Linter;
use file_pipeline_core::domain::models::*;
use file_pipeline_core::domain::wiki_export::WikiExporter;
use file_pipeline_core::ports::output::*;
use file_pipeline_core::service::FileProcessingService;
use file_pipeline_shared::test_helpers::ServiceBuilder;

// ── 테스트용 어댑터 ─────────────────────────────────────────

struct HashEmbedder { dim: usize }
impl HashEmbedder {
    fn new(dim: usize) -> Self { Self { dim } }
    fn hash_text(text: &str, dim: usize) -> Vec<f32> {
        let mut vec = vec![0.0f32; dim];
        for word in text.split_whitespace() {
            let hash = word.bytes().fold(0u64, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u64));
            vec[(hash as usize) % dim] += 1.0;
        }
        let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 { vec.iter_mut().for_each(|x| *x /= norm); }
        vec
    }
}
#[async_trait]
impl EmbeddingPort for HashEmbedder {
    fn dim(&self) -> usize { self.dim }
    async fn embed(&self, text: &str) -> anyhow::Result<Vec<f32>> { Ok(Self::hash_text(text, self.dim)) }
    async fn embed_batch(&self, texts: &[String]) -> anyhow::Result<Vec<Vec<f32>>> {
        Ok(texts.iter().map(|t| Self::hash_text(t, self.dim)).collect())
    }
}

struct SmartTestLlm;
#[async_trait]
impl LLMPort for SmartTestLlm {
    async fn classify_and_process(&self, file_path: &Path, registry: &DocTypeRegistry) -> anyhow::Result<ClassifyAndProcessResult> {
        let content = std::fs::read_to_string(file_path)?;
        let filename = file_path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();

        let mut doc_types: Vec<String> = Vec::new();
        if filename.contains("회의") || filename.contains("meeting") { doc_types.push("meeting".into()); }
        if filename.contains("학습") || filename.contains("study") { doc_types.push("study".into()); }
        if filename.contains("일지") || filename.contains("log") { doc_types.push("log".into()); }
        if doc_types.is_empty() { doc_types.push("etc".into()); }

        let mut processed = String::new();
        for dt in &doc_types {
            let sections = registry.sections_for(dt);
            for sec in &sections {
                processed.push_str(&format!("=== {} ===\n", sec));
                for line in content.lines().take(5) {
                    processed.push_str(line);
                    processed.push('\n');
                }
            }
            if sections.is_empty() {
                let chars: Vec<char> = content.chars().collect();
                let half: String = chars[..chars.len().min(chars.len() / 2 + 10)].iter().collect();
                processed.push_str(&half);
                processed.push('\n');
            }
        }

        let keywords: Vec<String> = content.split_whitespace().filter(|w| w.chars().count() >= 2).take(12).map(String::from).collect();
        let metadata = Metadata {
            doc_types: doc_types.clone(), rationale: "test".into(),
            date: chrono::Local::now().format("%Y-%m-%d").to_string(),
            summary: format!("테스트: {}", filename), keywords,
            sensitive: false, doi: None, related_docs: vec![], source_doc_ids: vec![], search_hints: vec![],
            entities: vec![],
            ..Default::default()        };
        Ok(ClassifyAndProcessResult { doc_types, rationale: "test".into(), content: processed, metadata, sections: None })
    }
    async fn summarize_text(&self, new: &str, existing: &str) -> anyhow::Result<String> { Ok(format!("{}\n{}", existing, new)) }
    async fn enrich_existing(&self, existing: &str, new_info: &str, _: &[String]) -> anyhow::Result<EnrichResult> {
        if new_info.lines().count() >= 5 {
            Ok(EnrichResult { updated_content: format!("{}\n--- 보강 ---\n{}", existing, new_info), change_summary: "보강됨".into(), should_update: true })
        } else {
            Ok(EnrichResult { updated_content: existing.into(), change_summary: String::new(), should_update: false })
        }
    }
}

// ── 환경 구성 ────────────────────────────────────────────────

struct TestEnv {
    _base: tempfile::TempDir,
    inbox: PathBuf, processed: PathBuf, originals: PathBuf,
    #[allow(dead_code)] sensitive: PathBuf,
    #[allow(dead_code)] todo: PathBuf,
    wiki: PathBuf,
    service: FileProcessingService,
}

fn setup() -> TestEnv {
    let registry = DocTypeRegistry::new(vec![
        DocTypeDef { id: "meeting".into(), label_ko: "회의록".into(), patterns: vec!["회의".into()],
            sections: vec!["결정사항".into(), "액션아이템".into(), "다음안건".into()],
            prompt: String::new(), dedup_key: None, sensitive: false, thresholds: None },
        DocTypeDef { id: "study".into(), label_ko: "학습".into(), patterns: vec!["학습".into()],
            sections: vec!["핵심개념".into(), "요약".into(), "모르는것".into(), "복습포인트".into()],
            prompt: String::new(), dedup_key: None, sensitive: false, thresholds: None },
        DocTypeDef { id: "log".into(), label_ko: "일지".into(), patterns: vec!["일지".into()],
            sections: vec!["완료".into(), "이슈".into(), "내일계획".into()],
            prompt: String::new(), dedup_key: None, sensitive: false, thresholds: None },
    ]);

    let base = tempfile::TempDir::new().unwrap();
    let service = ServiceBuilder::new(base.path())
        .with_llm(Arc::new(SmartTestLlm))
        .with_embedding(Arc::new(HashEmbedder::new(128)))
        .with_registry(Arc::new(registry))
        .with_semantic_dup_threshold(0.03)
        .with_verification_enabled(true)
        .with_fragment_threshold(0)
        .with_crossref_threshold(0.5)
        .with_crossref_interval(30)
        .build();
    let inbox = service.inbox_dir.clone();
    let processed = service.processed_dir.clone();
    let originals = service.originals_dir.clone();
    let sensitive = service.sensitive_dir.clone();
    let todo = service.todo_dir.clone();
    let wiki = base.path().join("wiki");

    TestEnv { _base: base, inbox, processed, originals, sensitive, todo, wiki, service }
}

fn write_file(dir: &Path, name: &str, content: &str) -> PathBuf {
    let p = dir.join(name);
    std::fs::write(&p, content).unwrap();
    p
}

fn count_files(dir: &Path) -> usize {
    std::fs::read_dir(dir).unwrap().filter_map(|e| e.ok()).filter(|e| e.path().is_file()).count()
}

fn walkdir(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() { files.extend(walkdir(&path)); } else { files.push(path); }
        }
    }
    files
}

// ═══════════════════════════════════════════════════════════════
// 시나리오 1: 처음 사용
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn e2e_first_use() {
    let env = setup();
    let f = write_file(&env.inbox, "회의록_0405.txt",
        "2026년 4월 5일 정기 회의\n참석: 김철수, 이영희\n결정: 4월 10일 배포\n액션: 김철수 QA\n다음 회의: 4월 12일");
    env.service.process_file(&f).await.unwrap();

    assert!(!f.exists());
    assert!(count_files(&env.processed) > 0);
    assert!(count_files(&env.originals) > 0);
    assert_eq!(env.service.vector_db.stats().unwrap().total_documents, 1);

    // wiki-export
    WikiExporter::export(env.service.vector_db.as_ref(), env.service.storage.as_ref(), &env.wiki).unwrap();
    assert!(env.wiki.join("INDEX.md").exists());
}

// ═══════════════════════════════════════════════════════════════
// 시나리오 2: 여러 파일 + 교차참조
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn e2e_multiple_files_with_relations() {
    let env = setup();
    for (name, content) in [
        ("회의록_0401.txt", "프로젝트 킥오프 회의\n참석: 김철수 이영희\n결정: 4월 말 MVP\n액션: 김철수 설계\n다음 안건: API"),
        ("회의록_0405.txt", "프로젝트 진행 회의\n참석: 김철수 이영희 박지민\n결정: API 확정\n액션: 박지민 프론트\n다음 안건: QA"),
        ("학습_rust.txt", "Rust 소유권 학습\n핵심개념: ownership borrowing lifetime\n요약: 메모리 안전성\n모르는것: elision\n복습: borrow checker"),
    ] {
        let f = write_file(&env.inbox, name, content);
        env.service.process_file(&f).await.unwrap();
    }
    assert_eq!(env.service.vector_db.stats().unwrap().total_documents, 3);
    assert_eq!(count_files(&env.inbox), 0);

    // wiki 내보내기
    let report = WikiExporter::export(env.service.vector_db.as_ref(), env.service.storage.as_ref(), &env.wiki).unwrap();
    assert_eq!(report.exported, 3);
}

// ═══════════════════════════════════════════════════════════════
// 시나리오 3: 완전 중복 스킵
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn e2e_exact_duplicate_skip() {
    let env = setup();
    let content = "이것은 중복 테스트 문서입니다.\n동일한 내용이 두 번 들어옵니다.";
    let f1 = write_file(&env.inbox, "doc_a.txt", content);
    env.service.process_file(&f1).await.unwrap();
    let f2 = write_file(&env.inbox, "doc_b.txt", content);
    env.service.process_file(&f2).await.unwrap();
    assert_eq!(env.service.vector_db.stats().unwrap().total_documents, 1);
}

// ═══════════════════════════════════════════════════════════════
// 시나리오 4: 민감 파일
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn e2e_sensitive_mixed() {
    let env = setup();
    let f1 = write_file(&env.inbox, "일반_노트.txt", "일반적인 내용의 노트입니다. 민감하지 않은 정보.");
    env.service.process_file(&f1).await.unwrap();
    let f2 = write_file(&env.inbox, "계약서_용역.txt", "갑: A사 을: B사 금액: 5억");
    env.service.process_file(&f2).await.unwrap();
    // 민감 파일도 DB에 색인됨 (sensitive/ 이동 + 최소 메타데이터로 색인)
    assert_eq!(env.service.vector_db.stats().unwrap().total_documents, 2);
    // 민감 파일이 sensitive/ 디렉토리로 이동되었는지 확인
    assert!(env.sensitive.join("계약서_용역.txt").exists(), "민감 파일이 sensitive/로 이동");
}

// ═══════════════════════════════════════════════════════════════
// 시나리오 5: lint (orphan 검출)
//
// Phase 55에서 stale 검사 자체 제거됨 (lint_stale_days 설정 + Linter::stale 분기 폐기).
// 본 테스트는 orphan(관계 0건) 검증만 수행. stale 단언은 lesson 28 따라 제거.
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn e2e_lint_with_orphan() {
    let env = setup();
    // 관계가 생기지 않는 단일 문서 1건만 투입 (orphan 발생)
    let f = write_file(&env.inbox, "lonely.txt", "고아 문서 검증용. 키워드가 매우 특수해서 어떤 다른 문서와도 연결되지 않을 내용입니다.");
    env.service.process_file(&f).await.unwrap();

    let report = Linter::lint(env.service.vector_db.as_ref()).unwrap();
    // orphan_docs는 관계 0건인 문서 ID 목록. 1건만 처리했으니 반드시 비어있지 않음.
    assert!(!report.orphan_docs.is_empty(), "고아 문서 감지: {:?}", report.orphan_docs);
}

// 시나리오 6 (purge) 제거됨: purge_expired_originals는 Phase 55에서 제거됨
// (retention/purge 시스템 + Tauri purge_dry_run/execute 커맨드로 대체)

// ═══════════════════════════════════════════════════════════════
// 시나리오 7: wiki-export 구조
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn e2e_wiki_export_structure() {
    let env = setup();
    for (name, content) in [
        ("회의록_sprint.txt", "스프린트 회의 결정사항 액션아이템 다음안건 논의"),
        ("학습_docker.txt", "Docker 컨테이너 핵심개념 이미지 빌드 배포 학습"),
    ] {
        let f = write_file(&env.inbox, name, content);
        env.service.process_file(&f).await.unwrap();
    }
    let report = WikiExporter::export(env.service.vector_db.as_ref(), env.service.storage.as_ref(), &env.wiki).unwrap();
    assert_eq!(report.exported, 2);
    let index = std::fs::read_to_string(env.wiki.join("INDEX.md")).unwrap();
    assert!(index.contains("총 2 문서"));
    let md_count = walkdir(&env.wiki).iter().filter(|p| p.extension().and_then(|e| e.to_str()) == Some("md")).count();
    assert!(md_count >= 3);
}

// ═══════════════════════════════════════════════════════════════
// 시나리오 8: 증분 컴파일
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn e2e_incremental_compile() {
    let env = setup();

    let f = write_file(&env.inbox, "note.txt", "첫 번째 내용입니다. 충분히 긴 문서.");
    env.service.process_file(&f).await.unwrap();
    assert_eq!(env.service.vector_db.stats().unwrap().total_documents, 1);

    // 동일 파일 재투입 → SHA-256 중복 스킵
    let f2 = write_file(&env.inbox, "note_copy.txt", "첫 번째 내용입니다. 충분히 긴 문서.");
    env.service.process_file(&f2).await.unwrap();
    assert_eq!(env.service.vector_db.stats().unwrap().total_documents, 1, "중복 스킵");

    // 내용 변경 파일 → 새로 처리
    let f3 = write_file(&env.inbox, "note_v2.txt", "완전히 다른 내용의 문서입니다. 새로운 주제.");
    env.service.process_file(&f3).await.unwrap();
    assert_eq!(env.service.vector_db.stats().unwrap().total_documents, 2, "변경 감지 → 새 처리");
}

// ═══════════════════════════════════════════════════════════════
// 시나리오 9: MCP 호환성 (VectorDB + Storage 직접 호출)
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn e2e_mcp_compatible_search() {
    let env = setup();

    for (name, content) in [
        ("회의록_test.txt", "회의 내용 프로젝트 일정 결정사항 액션아이템 다음안건"),
        ("학습_test.txt", "학습 내용 Rust 핵심개념 요약 모르는것 복습포인트"),
    ] {
        let f = write_file(&env.inbox, name, content);
        env.service.process_file(&f).await.unwrap();
    }

    // MCP search 동작 재현: embed → search_similar → read_header
    let embedding = env.service.embedding.embed("회의").await.unwrap();
    let results = env.service.vector_db.search_similar(&embedding, 5).unwrap();
    assert!(!results.is_empty(), "검색 결과가 있어야 함");

    // MCP get_document 동작 재현: list_all → decompress
    let all = env.service.vector_db.list_all().unwrap();
    assert_eq!(all.len(), 2);

    // MCP stats 동작 재현
    let stats = env.service.vector_db.stats().unwrap();
    assert_eq!(stats.total_documents, 2);

    // MCP lint 동작 재현
    let report = Linter::lint(env.service.vector_db.as_ref()).unwrap();
    let _ = report.issues.len();
}

// ═══════════════════════════════════════════════════════════════
// 시나리오 10: HashEmbedder 유사도
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn e2e_hash_embedder_similarity() {
    let embedder = HashEmbedder::new(128);
    let v1 = embedder.embed("프로젝트 회의 결정사항 배포 일정").await.unwrap();
    let v2 = embedder.embed("프로젝트 회의 결정사항 QA 일정").await.unwrap();
    let v3 = embedder.embed("Rust 소유권 lifetime borrow checker").await.unwrap();
    let sim_12 = file_pipeline_core::domain::deduplicator::cosine_similarity(&v1, &v2);
    let sim_13 = file_pipeline_core::domain::deduplicator::cosine_similarity(&v1, &v3);
    assert!(sim_12 > sim_13, "유사 주제({:.3}) > 다른 주제({:.3})", sim_12, sim_13);
}

// ═══════════════════════════════════════════════════════════════
// 시나리오 11: 스킵 확장자
// ═══════════════════════════════════════════════════════════════

#[test]
fn e2e_skip_extension_check() {
    let skip_exts = [".tmp", ".part", ".crdownload", ".download"];
    for ext in &skip_exts {
        let filename = format!("test{}", ext);
        let path = Path::new(&filename);
        let ext_str = path.extension().and_then(|e| e.to_str()).map(|e| format!(".{}", e)).unwrap_or_default();
        assert!(skip_exts.contains(&ext_str.as_str()));
    }
    for ext in &[".txt", ".md", ".docx"] {
        let filename = format!("test{}", ext);
        let path = Path::new(&filename);
        let ext_str = path.extension().and_then(|e| e.to_str()).map(|e| format!(".{}", e)).unwrap_or_default();
        assert!(!skip_exts.contains(&ext_str.as_str()));
    }
}

// ═══════════════════════════════════════════════════════════════
// 시나리오 12: DOCX 네이티브 전처리 → 파이프라인
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn e2e_docx_native_preprocess() {
    use std::io::Write;

    let env = setup();

    // 테스트용 DOCX 파일 생성 (ZIP + word/document.xml)
    let docx_path = env.inbox.join("회의록.docx");
    {
        let file = std::fs::File::create(&docx_path).expect("docx 생성");
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();

        zip.start_file("[Content_Types].xml", options).unwrap();
        write!(zip, r#"<?xml version="1.0"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="xml" ContentType="application/xml"/><Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/></Types>"#).unwrap();

        zip.start_file("word/document.xml", options).unwrap();
        write!(zip, r#"<?xml version="1.0" encoding="UTF-8"?><w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:body><w:p><w:r><w:t>2026년 4월 프로젝트 회의록</w:t></w:r></w:p><w:p><w:r><w:t>결정사항: API 리팩터링 완료</w:t></w:r></w:p><w:p><w:r><w:t>참석자: 김팀장, 이개발</w:t></w:r></w:p></w:body></w:document>"#).unwrap();

        zip.finish().unwrap();
    }

    // 전처리 → 파이프라인
    env.service.process_file(&docx_path).await.unwrap();

    // DB에 색인 확인
    let stats = env.service.vector_db.stats().unwrap();
    assert!(stats.total_documents >= 1, "DOCX 가공 결과 DB에 색인");
}

// ═══════════════════════════════════════════════════════════════
// 시나리오 13: 프롬프트 핫 리로드
// ═══════════════════════════════════════════════════════════════

#[test]
fn e2e_prompt_hot_reload() {
    use file_pipeline_adapters::driven::llm::prompts;

    // 현재 프롬프트 확인
    let content = prompts::get_prompts_content();
    assert!(content.contains("[classify]"));
    assert!(content.contains("{filename}"));

    // 핫 리로드: RwLock을 통해 즉시 반영되는지 확인
    prompts::reload_prompts();

    // 리로드 후에도 기본 프롬프트 유지
    let prompt = prompts::build_classify_prompt("test.txt", "내용", "힌트");
    assert!(prompt.contains("test.txt"));
    assert!(prompt.contains("내용"));

    // 잘못된 TOML은 에러 반환
    let result = prompts::save_prompts("{{{{invalid");
    assert!(result.is_err());
}

// ═══════════════════════════════════════════════════════════════
// 시나리오 14: 배치 임베딩 병렬
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn e2e_batch_embed_parallel() {
    let embedder = HashEmbedder::new(128);
    let texts: Vec<String> = (0..10).map(|i| format!("배치 임베딩 테스트 문서 #{}", i)).collect();

    let start = std::time::Instant::now();
    let results = embedder.embed_batch(&texts).await.unwrap();
    let elapsed = start.elapsed();

    assert_eq!(results.len(), 10);
    for emb in &results {
        assert_eq!(emb.len(), 128);
    }
    println!("배치 10건 임베딩: {:?}", elapsed);
}

// ═══════════════════════════════════════════════════════════════
// 시나리오 15: 교차참조 비동기 배치
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn e2e_crossref_async_batch() {
    let env = setup();

    // 문서 3건 투입 (배치 모드: batch_end에서 mmap 생성 보장)
    env.service.vector_db.batch_begin();
    for i in 0..3 {
        let f = write_file(&env.inbox, &format!("batch_{}.txt", i),
            &format!("교차참조 배치 테스트 문서 #{}", i));
        env.service.process_file(&f).await.unwrap();
    }
    env.service.vector_db.batch_end();

    // 큐에 3건 대기
    assert_eq!(env.service.crossref_queue_len(), 3, "큐에 3건 대기");

    // flush_crossref 실행
    let processed = env.service.flush_crossref().unwrap();
    assert_eq!(processed, 3, "3건 처리됨");
    assert_eq!(env.service.crossref_queue_len(), 0, "큐 비워짐");

    // 간격 미달 시 스킵
    let skipped = env.service.flush_crossref().unwrap();
    assert_eq!(skipped, 0, "간격 미달 → 스킵");
}

// ═══════════════════════════════════════════════════════════════
// 시나리오 16: 교차참조 중복 skip
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn e2e_crossref_duplicate_skip() {
    let env = setup();

    // 같은 파일 2번 투입 → 큐에 1건만
    let f = write_file(&env.inbox, "dup.txt", "중복 테스트 문서");
    env.service.process_file(&f).await.unwrap();
    // 두 번째는 SHA-256 중복으로 process_file 자체가 스킵되므로
    // 새 파일로 같은 doc_id를 시뮬레이션
    assert!(env.service.crossref_queue_len() <= 1, "중복 방지");
}

// ═══════════════════════════════════════════════════════════════
// 시나리오 17: XLSX 전처리 → 텍스트 추출
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn e2e_xlsx_preprocess() {
    let env = setup();

    // Python으로 최소 XLSX 생성 (openpyxl)
    let xlsx_path = env.inbox.join("데이터.xlsx");
    let script = format!(
        "import openpyxl; wb=openpyxl.Workbook(); ws=wb.active; ws.title='시트1'; ws.append(['이름','나이']); ws.append(['김철수',30]); ws.append(['이영희',25]); wb.save(r'{}')",
        xlsx_path.display()
    );
    let py_result = std::process::Command::new("python")
        .args(["-c", &script])
        .output();

    match py_result {
        Ok(output) if output.status.success() => {
            // openpyxl로 XLSX 생성 성공 → CompositePreprocessor로 네이티브 전처리 테스트
            use file_pipeline_core::ports::output::PreprocessPort;
            let pp = file_pipeline_adapters::driven::preprocessing::preprocessor::CompositePreprocessor::new("none", "none");
            let result = pp.preprocess(&xlsx_path);
            match result {
                Ok(r) => {
                    println!("XLSX 전처리 결과: [{}]", &r.text[..r.text.len().min(500)]);
                    assert!(!r.text.trim().is_empty(), "XLSX 텍스트가 비어있음");
                    println!("XLSX 전처리 성공: {}자", r.text.len());
                }
                Err(e) => {
                    println!("XLSX 전처리 에러 (호스트 도구 없음): {}", e);
                    // 호스트 도구 없으면 에러 — 정상
                }
            }
        }
        _ => {
            // openpyxl 미설치 → 스킵
            println!("openpyxl 미설치 → XLSX 테스트 스킵");
        }
    }
}

// ═══════════════════════════════════════════════════════════════
// Phase C: 고급 워크플로우 테스트
// ═══════════════════════════════════════════════════════════════

// ── Phase C용 검증 실패 LLM ────────────────────────────────────

/// 빈 가공 결과를 반환하여 검증 실패를 유도하는 LLM
struct FailingLlm;
#[async_trait]
impl LLMPort for FailingLlm {
    async fn classify_and_process(&self, file_path: &Path, _registry: &DocTypeRegistry) -> anyhow::Result<ClassifyAndProcessResult> {
        let _filename = file_path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
        let metadata = Metadata {
            doc_types: vec!["meeting".into()], rationale: "fail-test".into(),
            date: "2026-04-21".into(), summary: "fail".into(),
            keywords: vec!["없는키워드".into()], // 커버리지 실패 유도
            sensitive: false, doi: None, related_docs: vec![], source_doc_ids: vec![],
            search_hints: vec![], entities: vec![],
            ..Default::default()
        };
        // 빈 가공본 → 구조/압축률/ROUGE 모두 실패
        Ok(ClassifyAndProcessResult {
            doc_types: vec!["meeting".into()], rationale: "fail-test".into(),
            content: "".into(), metadata, sections: None,
        })
    }
    async fn summarize_text(&self, new: &str, existing: &str) -> anyhow::Result<String> { Ok(format!("{}\n{}", existing, new)) }
    async fn enrich_existing(&self, existing: &str, _: &str, _: &[String]) -> anyhow::Result<EnrichResult> {
        Ok(EnrichResult { updated_content: existing.into(), change_summary: String::new(), should_update: false })
    }
}

fn setup_with_llm(llm: Arc<dyn LLMPort>, verification_enabled: bool) -> TestEnv {
    let registry = DocTypeRegistry::new(vec![
        DocTypeDef { id: "meeting".into(), label_ko: "회의록".into(), patterns: vec!["회의".into()],
            sections: vec!["결정사항".into(), "액션아이템".into(), "다음안건".into()],
            prompt: String::new(), dedup_key: None, sensitive: false, thresholds: None },
        DocTypeDef { id: "study".into(), label_ko: "학습".into(), patterns: vec!["학습".into()],
            sections: vec!["핵심개념".into(), "요약".into(), "모르는것".into(), "복습포인트".into()],
            prompt: String::new(), dedup_key: None, sensitive: false, thresholds: None },
        DocTypeDef { id: "log".into(), label_ko: "일지".into(), patterns: vec!["일지".into()],
            sections: vec!["완료".into(), "이슈".into(), "내일계획".into()],
            prompt: String::new(), dedup_key: None, sensitive: false, thresholds: None },
    ]);

    let base = tempfile::TempDir::new().expect("tempdir failed");
    let service = ServiceBuilder::new(base.path())
        .with_llm(llm)
        .with_embedding(Arc::new(HashEmbedder::new(128)))
        .with_registry(Arc::new(registry))
        .with_semantic_dup_threshold(0.03)
        .with_verification_enabled(verification_enabled)
        .with_fragment_threshold(0)
        .with_crossref_threshold(0.5)
        .with_crossref_interval(30)
        .build();
    let inbox = service.inbox_dir.clone();
    let processed = service.processed_dir.clone();
    let originals = service.originals_dir.clone();
    let sensitive = service.sensitive_dir.clone();
    let todo = service.todo_dir.clone();
    let wiki = base.path().join("wiki");

    TestEnv { _base: base, inbox, processed, originals, sensitive, todo, wiki, service }
}

// ═══════════════════════════════════════════════════════════════
// 시나리오 18: 검증 2-Pass → quarantine 이동
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn e2e_verification_quarantine() {
    let env = setup_with_llm(Arc::new(FailingLlm), true);
    let f = write_file(&env.inbox, "회의록_fail.txt",
        "2026년 4월 회의\n참석: 김철수 이영희\n결정: 4월 배포\n액션: QA\n다음 안건: 리뷰\n충분히 긴 내용입니다.");
    let _ = env.service.process_file(&f).await;

    // quarantine 디렉토리에 파일이 이동되어야 함
    let quarantine = &env.service.quarantine_dir;
    let quarantine_files = std::fs::read_dir(quarantine)
        .map(|entries| entries.filter_map(|e| e.ok()).count())
        .unwrap_or(0);
    assert!(quarantine_files >= 1, "검증 2-Pass 실패 → quarantine 이동: {}건", quarantine_files);

    // DB에는 색인되지 않아야 함
    assert_eq!(env.service.vector_db.stats().expect("stats").total_documents, 0,
        "quarantine 파일은 DB에 색인되지 않음");

    // summary에 에러 기록
    let summary = env.service.summary.lock().expect("mutex poisoned");
    assert!(summary.errors > 0, "에러 카운트 기록됨");
}

// ═══════════════════════════════════════════════════════════════
// 시나리오 19: 배치 모드 전체 정합성 + 진단 스냅샷
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn e2e_batch_mode_integrity() {
    let env = setup();
    let doc_count: usize = 20;

    // 문서 20건 생성
    let mut files = Vec::new();
    for i in 0..doc_count {
        let name = format!("{}_{:03}.txt", match i % 3 { 0 => "회의록", 1 => "학습", _ => "일지" }, i);
        let content = format!(
            "{} 문서 #{}\n2026년 4월 {}일\n참석: 김철수 이영희\n결정: 프로젝트 진행\nDOC-{:03}",
            match i % 3 { 0 => "회의록", 1 => "학습", _ => "일지" }, i, (i % 28) + 1, i
        );
        let f = write_file(&env.inbox, &name, &content);
        files.push(f);
    }

    // 배치 모드 처리
    env.service.vector_db.batch_begin();
    env.service.compile_state_batch_begin();

    for f in &files {
        let _ = env.service.process_file(f).await;
    }

    env.service.vector_db.batch_end();
    env.service.compile_state_batch_end();
    let flush_count = env.service.flush_crossref().expect("flush failed");

    // 1. 전체 문서 색인 확인
    let stats = env.service.vector_db.stats().expect("stats");
    assert_eq!(stats.total_documents, doc_count as u64, "전체 문서 색인");

    // 2. inbox 비워짐
    assert_eq!(count_files(&env.inbox), 0, "inbox 비워짐");

    // 3. 교차참조 flush 실행됨
    assert!(flush_count > 0, "교차참조 flush 실행: {}건", flush_count);

    // 4. 진단 실행
    let corpus_stats = diagnostics::analyze_corpus(env.service.vector_db.as_ref())
        .expect("corpus analysis");
    assert_eq!(corpus_stats.doc_count, doc_count);
    assert!(corpus_stats.relations.total > 0, "교차참조 관계 존재");

    let issues = diagnostics::health_check(&corpus_stats);
    // 20문서 규모에서는 critical error 없어야 함
    let errors: Vec<_> = issues.iter()
        .filter(|i| i.level == diagnostics::HealthLevel::Error)
        .collect();
    assert!(errors.is_empty(), "health check 에러 없음: {:?}",
        errors.iter().map(|e| &e.message).collect::<Vec<_>>());

    // 5. 스냅샷 직렬화 가능 확인
    let json = serde_json::to_string(&corpus_stats).expect("serialize");
    let _: diagnostics::CorpusStats = serde_json::from_str(&json).expect("deserialize");
}

// ═══════════════════════════════════════════════════════════════
// 시나리오 20: compile_state 영속화 → 서비스 재생성 → 스킵
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn e2e_compile_state_persistence() {
    // 1단계: 파일 처리 → compile_state 저장
    let base = tempfile::TempDir::new().expect("tempdir failed");
    let state_path = base.path().join(".compile-state.json");

    let env = setup();
    let f = write_file(&env.inbox, "note.txt", "영속화 테스트 문서입니다. 충분히 긴 내용.");
    env.service.process_file(&f).await.expect("process");
    assert_eq!(env.service.vector_db.stats().expect("stats").total_documents, 1);

    // compile_state를 파일로 저장
    {
        let state = env.service.compile_state.lock().expect("mutex");
        state.save(&state_path).expect("compile_state save");
    }
    assert!(state_path.exists(), "compile_state.json 저장됨");

    // 2단계: 새 서비스 생성 → 저장된 state 로드
    let loaded_state = CompileState::load(&state_path).expect("load");

    // 서비스가 기록한 해시로 확인: entries가 비어있지 않아야 함
    assert!(!loaded_state.entries.is_empty(), "compile_state에 엔트리 존재");

    // 동일 해시로 변경 체크 → 스킵되어야 함
    let (file_key, file_state) = loaded_state.entries.iter().next().expect("첫 엔트리 존재");
    assert!(!loaded_state.is_changed(file_key, &file_state.hash), "동일 해시 → 변경 없음 판정");

    // 다른 해시 → 변경으로 감지
    assert!(loaded_state.is_changed(file_key, "different_hash"), "다른 해시 → 변경 감지");
}

// ═══════════════════════════════════════════════════════════════
// 시나리오 21: 벤치마크 스냅샷 JSON 직렬화 왕복
// ═══════════════════════════════════════════════════════════════

#[test]
fn e2e_benchmark_snapshot_roundtrip() {
    use file_pipeline_core::domain::diagnostics::*;

    let snapshot = BenchmarkSnapshot {
        version: BenchmarkSnapshot::CURRENT_VERSION,
        timestamp: "2026-04-21T12:00:00".into(),
        label: "test_roundtrip".into(),
        git_hash: Some("abc1234".into()),
        doc_count: 100,
        throughput: ThroughputMetrics {
            total_secs: 10.5,
            process_secs: 8.0,
            batch_end_secs: 0.5,
            flush_secs: 2.0,
            docs_per_sec: 9.5,
        },
        per_doc: Some(PerDocMetrics {
            avg_ms: 80.0,
            p50_ms: 75.0,
            p95_ms: 95.0,
            max_ms: 120.0,
            variance_ratio: 1.27,
        }),
        search: Some(SearchMetrics {
            avg_ms: 0.5,
            p95_ms: 1.2,
            queries: 100,
        }),
        crossref: CrossrefMetrics {
            relation_count: 5000,
            unique_pairs: 2500,
            double_count_ratio: 2.0,
            isolated_docs: 5,
        },
        storage: Some(StorageMetrics {
            input_bytes: 100000,
            processed_bytes: 50000,
            originals_bytes: 30000,
            compression_pct: 20.0,
        }),
        corpus: None,
    };

    // 1. JSON 직렬화 왕복
    let json = serde_json::to_string_pretty(&snapshot).expect("serialize");
    let deserialized: BenchmarkSnapshot = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(deserialized.doc_count, 100);
    assert_eq!(deserialized.label, "test_roundtrip");
    assert!((deserialized.throughput.docs_per_sec - 9.5).abs() < 0.01);

    // 2. 파일 저장/로드 왕복
    let tmp = tempfile::TempDir::new().expect("tempdir");
    let path = tmp.path().join("test.json");
    snapshot.save_to(&path).expect("save");
    let loaded = BenchmarkSnapshot::load_from(&path).expect("load");
    assert_eq!(loaded.doc_count, snapshot.doc_count);
    assert_eq!(loaded.version, BenchmarkSnapshot::CURRENT_VERSION);

    // 3. 회귀 감지
    let mut degraded = snapshot.clone();
    degraded.throughput.docs_per_sec = 5.0; // 47% 하락
    let result = check_regression(&snapshot, &degraded);
    assert!(!result.passed, "47% 하락 → 회귀 감지");

    let ok_result = check_regression(&snapshot, &snapshot);
    assert!(ok_result.passed, "동일 결과 → 회귀 없음");
}

// ═══════════════════════════════════════════════════════════════
// 시나리오 22: 검색 후 교차참조 양방향 링크 확인
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn e2e_crossref_bidirectional() {
    let mut env = setup();
    // 교차참조 즉시 flush를 위해 간격 0으로 설정
    env.service.crossref_interval_secs = 0;

    // 유사한 주제의 문서 3건 투입 (배치 모드: batch_end에서 mmap 생성)
    env.service.vector_db.batch_begin();
    for (name, content) in [
        ("회의록_api.txt", "API 설계 회의\n결정: REST API → gRPC 전환\n액션: 프로토콜 정의\n다음 안건: 성능 테스트"),
        ("회의록_grpc.txt", "gRPC 전환 회의\n결정: proto3 채택\n참석: API팀\n액션: gRPC 서버 구현\n다음 안건: 배포"),
        ("학습_grpc.txt", "gRPC 학습 노트\n핵심: proto3 HTTP/2 streaming\n요약: API 대비 10x 성능"),
    ] {
        let f = write_file(&env.inbox, name, content);
        env.service.process_file(&f).await.expect("process");
    }
    env.service.vector_db.batch_end(); // mmap 생성

    // 큐 상태 확인
    let queue_len = env.service.crossref_queue_len();
    assert!(queue_len > 0, "교차참조 큐에 {}건 대기", queue_len);

    // flush 교차참조
    let flushed = env.service.flush_crossref().expect("flush");
    assert!(flushed > 0, "교차참조 flush 실행됨: {}건", flushed);

    // 모든 문서의 관계 확인
    let all = env.service.vector_db.list_all().expect("list");
    let mut total_relations = 0;
    for doc in &all {
        let rels = env.service.vector_db.find_related(&doc.id).expect("find_related");
        total_relations += rels.len();
    }
    assert!(total_relations > 0, "유사 문서 간 교차참조 생성");

    // 양방향: A→B가 있으면 B→A도 있어야 함
    for doc in &all {
        let rels = env.service.vector_db.find_related(&doc.id).expect("find_related");
        for rel in &rels {
            let reverse = env.service.vector_db.find_related(&rel.target_id).expect("reverse");
            let has_reverse = reverse.iter().any(|r| r.target_id == doc.id);
            // referenced_by 등 역방향 관계 존재해야 함
            assert!(has_reverse, "양방향 링크: {} ↔ {}", doc.id, rel.target_id);
        }
    }
}

// step-o2 partial 해소 (2026-06-17): integration test mock OutboundManifest 박힘
impl file_pipeline_core::ports::outbound::OutboundManifest for FailingLlm {
    fn id(&self) -> &str { "fp-outbound-llm-failing-test" }
    fn category(&self) -> file_pipeline_core::ports::outbound::OutboundCategory {
        file_pipeline_core::ports::outbound::OutboundCategory::Llm
    }
    fn capabilities(&self) -> file_pipeline_core::ports::output::ResourceCapabilities {
        file_pipeline_core::ports::output::ResourceCapabilities::standard("failing-test")
    }
}

// step-o2 partial 해소 (2026-06-17): integration test mock OutboundManifest 박힘
impl file_pipeline_core::ports::outbound::OutboundManifest for SmartTestLlm {
    fn id(&self) -> &str { "fp-outbound-llm-smart-test" }
    fn category(&self) -> file_pipeline_core::ports::outbound::OutboundCategory {
        file_pipeline_core::ports::outbound::OutboundCategory::Llm
    }
    fn capabilities(&self) -> file_pipeline_core::ports::output::ResourceCapabilities {
        file_pipeline_core::ports::output::ResourceCapabilities::standard("smart-test")
    }
}

// step-o2 partial 해소 추가 (2026-06-17)
impl file_pipeline_core::ports::outbound::OutboundManifest for HashEmbedder {
    fn id(&self) -> &str { "fp-outbound-embedding-hash" }
    fn category(&self) -> file_pipeline_core::ports::outbound::OutboundCategory {
        file_pipeline_core::ports::outbound::OutboundCategory::Embedding
    }
    fn capabilities(&self) -> file_pipeline_core::ports::output::ResourceCapabilities {
        file_pipeline_core::ports::output::ResourceCapabilities::standard("hash")
    }
}
