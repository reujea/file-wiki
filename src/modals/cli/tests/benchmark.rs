//! 벤치마크 테스트 — 30개 문서로 토큰/압축률/교차참조 실측
//!
//! 문서셋: 회의록 10 + 학습노트 10 + 일지 10 = 30개
//! 측정: 입력 chars, 출력 chars, 추정 토큰, 압축률, zstd 절감률, 교차참조 수

use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use file_pipeline_core::domain::incremental::BenchmarkReport;
use file_pipeline_core::domain::models::*;
use file_pipeline_core::ports::output::*;
use file_pipeline_core::service::FileProcessingService;
use file_pipeline_shared::test_helpers::ServiceBuilder;

// ── 테스트용 어댑터 (e2e_embedded.rs와 동일) ────────────────

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

struct BenchLlm;
#[async_trait]
impl LLMPort for BenchLlm {
    async fn classify_and_process(&self, file_path: &Path, registry: &DocTypeRegistry) -> anyhow::Result<ClassifyAndProcessResult> {
        let content = std::fs::read_to_string(file_path)?;
        let filename = file_path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();

        let mut doc_types: Vec<String> = Vec::new();
        if filename.contains("회의") { doc_types.push("meeting".to_string()); }
        if filename.contains("학습") { doc_types.push("study".to_string()); }
        if filename.contains("일지") { doc_types.push("log".to_string()); }
        if doc_types.is_empty() { doc_types.push("etc".to_string()); }

        // 유형별 섹션으로 가공 — 원본 키워드 최대 보존 (검증 통과 목적)
        let mut processed = String::new();
        for dt in &doc_types {
            let sections = registry.sections_for(dt);
            if sections.is_empty() {
                processed.push_str(&format!("=== {} ===\n", dt));
            }
            for sec in &sections {
                processed.push_str(&format!("=== {} ===\n", sec));
                // 각 섹션에 원본 줄을 분배 (키워드 커버리지 + ROUGE-L 확보)
                for line in content.lines().take(5) {
                    processed.push_str(line);
                    processed.push('\n');
                }
            }
        }

        let keywords: Vec<String> = content.split_whitespace()
            .filter(|w| w.chars().count() >= 2)
            .take(12)
            .map(String::from)
            .collect();

        let metadata = Metadata {
            doc_types: doc_types.clone(),
            rationale: "bench".into(),
            date: chrono::Local::now().format("%Y-%m-%d").to_string(),
            summary: format!("벤치: {}", filename),
            keywords,
            sensitive: false,
            doi: None,
            related_docs: vec![], source_doc_ids: vec![], search_hints: vec![],
            entities: vec![],
            ..Default::default()        };

        Ok(ClassifyAndProcessResult { doc_types, rationale: "bench".into(), content: processed, metadata, sections: None })
    }

    async fn summarize_text(&self, new: &str, existing: &str) -> anyhow::Result<String> {
        Ok(format!("{}\n{}", existing, new))
    }

    async fn enrich_existing(&self, existing: &str, new_info: &str, _: &[String]) -> anyhow::Result<EnrichResult> {
        // 5줄 이상이면 보강
        if new_info.lines().count() >= 5 {
            Ok(EnrichResult {
                updated_content: format!("{}\n--- 보강 ---\n{}", existing, new_info.lines().take(3).collect::<Vec<_>>().join("\n")),
                change_summary: "벤치 보강".into(),
                should_update: true,
            })
        } else {
            Ok(EnrichResult { updated_content: existing.into(), change_summary: String::new(), should_update: false })
        }
    }
}

// ── 문서 생성기 ─────────────────────────────────────────────

fn generate_meeting(idx: usize) -> (String, String) {
    let name = format!("회의록_{:04}.txt", idx);
    let content = format!(
        "2026년 4월 {}일 정기 프로젝트 회의\n\
         참석자: 김철수, 이영희, 박지민, 최동훈\n\
         \n\
         안건 1: 프로젝트 알파 진행 상황 보고\n\
         - 백엔드 API 개발 {}% 완료\n\
         - 프론트엔드 UI 작업 진행 중\n\
         - QA 테스트 계획 수립 필요\n\
         \n\
         결정사항:\n\
         - 4월 {}일까지 API v2 배포 확정\n\
         - 김철수가 테스트 자동화 담당\n\
         - 이영희가 문서 갱신 담당\n\
         \n\
         액션아이템:\n\
         - [김철수] CI/CD 파이프라인 구축 | 4월 {}일\n\
         - [이영희] API 문서 업데이트 | 4월 {}일\n\
         - [박지민] 프론트 컴포넌트 리팩터링 | 4월 {}일\n\
         \n\
         다음 회의: 4월 {}일 오후 2시",
        idx, idx * 10, idx + 5, idx + 3, idx + 4, idx + 6, idx + 7
    );
    (name, content)
}

fn generate_study(idx: usize) -> (String, String) {
    let topics = ["Rust 소유권", "async/await", "트레이트 시스템", "매크로", "에러 처리",
                   "스마트 포인터", "동시성", "웹 프레임워크", "테스트 전략", "성능 최적화"];
    let topic = topics[idx % topics.len()];
    let name = format!("학습_{:02}.txt", idx);
    let content = format!(
        "{} 학습 노트 #{}\n\
         \n\
         핵심개념:\n\
         - {} 의 기본 원리와 동작 방식\n\
         - 컴파일러가 이를 어떻게 처리하는지\n\
         - 다른 언어와의 비교 (C++, Go, Python)\n\
         \n\
         요약:\n\
         {} 은 Rust의 핵심 기능 중 하나로, 메모리 안전성과 성능을\n\
         동시에 보장하는 데 중요한 역할을 한다. 특히 zero-cost abstraction\n\
         원칙에 따라 런타임 오버헤드 없이 안전성을 제공한다.\n\
         \n\
         모르는것:\n\
         - 복잡한 라이프타임 시나리오에서의 적용\n\
         - 실제 프로덕션 코드에서의 패턴\n\
         - 성능 임팩트 측정 방법\n\
         \n\
         복습포인트:\n\
         - 공식 문서 Chapter {} 재독\n\
         - 연습 문제 풀기\n\
         - 실제 프로젝트에 적용해보기",
        topic, idx, topic, topic, idx + 1
    );
    (name, content)
}

fn generate_log(idx: usize) -> (String, String) {
    let name = format!("일지_0{:02}.txt", idx);
    let content = format!(
        "4월 {}일 개발 일지\n\
         \n\
         완료:\n\
         - file-pipeline 코어 모듈 구현 ({}시간)\n\
         - 단위 테스트 {} 개 작성 및 통과\n\
         - 코드 리뷰 반영 완료\n\
         - 문서 업데이트\n\
         \n\
         이슈:\n\
         - Windows 경로 처리에서 한글 인코딩 문제 발견\n\
         - qdrant-client API 변경으로 인한 호환성 이슈\n\
         - zstd 압축 시 대용량 파일에서 메모리 사용량 높음\n\
         \n\
         내일계획:\n\
         - 벤치마크 테스트 작성\n\
         - MCP 서버 연동\n\
         - 교차참조 시스템 검증",
        idx, idx, idx * 3
    );
    (name, content)
}

// ── 벤치마크 헬퍼 ───────────────────────────────────────────

fn setup_bench() -> (tempfile::TempDir, FileProcessingService) {
    let base = tempfile::TempDir::new().unwrap();

    let registry = DocTypeRegistry::new(vec![
        DocTypeDef { id: "meeting".into(), label_ko: "회의록".into(),
            patterns: vec!["회의".into()],
            sections: vec!["결정사항".into(), "액션아이템".into(), "다음안건".into()],
            prompt: String::new(), dedup_key: None, sensitive: false, thresholds: None },
        DocTypeDef { id: "study".into(), label_ko: "학습".into(),
            patterns: vec!["학습".into()],
            sections: vec!["핵심개념".into(), "요약".into(), "모르는것".into(), "복습포인트".into()],
            prompt: String::new(), dedup_key: None, sensitive: false, thresholds: None },
        DocTypeDef { id: "log".into(), label_ko: "일지".into(),
            patterns: vec!["일지".into()],
            sections: vec!["완료".into(), "이슈".into(), "내일계획".into()],
            prompt: String::new(), dedup_key: None, sensitive: false, thresholds: None },
    ]);

    let service = ServiceBuilder::new(base.path())
        .with_llm(Arc::new(BenchLlm))
        .with_embedding(Arc::new(HashEmbedder::new(128)))
        .with_registry(Arc::new(registry))
        .with_semantic_dup_threshold(0.03)
        .with_verification_enabled(true)
        .with_fragment_threshold(0)
        .with_crossref_threshold(0.5)
        .with_crossref_interval(30)
        .build();

    (base, service)
}

// ═══════════════════════════════════════════════════════════════
// 벤치마크: 30개 문서 전체 컴파일
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn benchmark_full_compile_30_docs() {
    let (base, service) = setup_bench();
    let inbox = base.path().join("inbox");

    // 30개 문서 생성
    let mut total_input_chars = 0u64;
    let mut files = Vec::new();

    for i in 0..10 {
        let (name, content) = generate_meeting(i + 1);
        total_input_chars += content.len() as u64;
        let p = inbox.join(&name);
        std::fs::write(&p, &content).unwrap();
        files.push(p);
    }
    for i in 0..10 {
        let (name, content) = generate_study(i);
        total_input_chars += content.len() as u64;
        let p = inbox.join(&name);
        std::fs::write(&p, &content).unwrap();
        files.push(p);
    }
    for i in 0..10 {
        let (name, content) = generate_log(i + 1);
        total_input_chars += content.len() as u64;
        let p = inbox.join(&name);
        std::fs::write(&p, &content).unwrap();
        files.push(p);
    }

    // 전체 컴파일
    let start = Instant::now();
    let mut processed_count = 0u64;
    
    let mut error_count = 0u64;

    for f in &files {
        if f.exists() {
            match service.process_file(f).await {
                Ok(()) => processed_count += 1,
                Err(e) => {
                    eprintln!("  처리 실패: {:?} → {}", f.file_name(), e);
                    error_count += 1;
                }
            }
        }
    }

    let elapsed = start.elapsed();

    // 컴파일 상태에서 통계 수집
    let state = service.compile_state.lock().unwrap();
    let total_output_chars = state.stats.total_output_chars;

    // 벤치마크 보고서
    let input_tokens = BenchmarkReport::estimate_tokens(total_input_chars);
    let output_tokens = BenchmarkReport::estimate_tokens(total_output_chars);
    let report = BenchmarkReport {
        files_processed: processed_count,
        files_skipped: 0,
        input_chars: total_input_chars,
        output_chars: total_output_chars,
        estimated_input_tokens: input_tokens,
        estimated_output_tokens: output_tokens,
        compression_ratio: if total_output_chars > 0 { total_input_chars as f64 / total_output_chars as f64 } else { 0.0 },
        estimated_cost_usd: BenchmarkReport::estimate_cost(input_tokens, output_tokens),
        is_incremental: false,
    };

    println!("\n{}", report.summary());
    println!("소요 시간: {:.2}초", elapsed.as_secs_f64());

    // DB 통계
    let stats = service.vector_db.stats().unwrap();
    println!("DB 문서 수: {}", stats.total_documents);

    // 교차참조 통계
    let all = service.vector_db.list_all().unwrap();
    let mut total_relations = 0;
    for doc in &all {
        let rels = service.vector_db.find_related(&doc.id).unwrap();
        total_relations += rels.len();
    }
    println!("교차참조 링크: {} 개", total_relations);

    // zstd 압축 통계
    let processed_dir = base.path().join("processed");
    let originals_dir = base.path().join("originals");
    let processed_size: u64 = std::fs::read_dir(&processed_dir).unwrap()
        .filter_map(|e| e.ok())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum();
    let originals_size: u64 = std::fs::read_dir(&originals_dir).unwrap()
        .filter_map(|e| e.ok())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum();
    println!(
        "zstd: 가공본 {}B, 원본 {}B (입력 {}B 대비 {:.1}% 절감)",
        processed_size, originals_size, total_input_chars,
        (1.0 - (processed_size + originals_size) as f64 / total_input_chars as f64) * 100.0
    );

    // 검증
    println!("오류: {} 건", error_count);
    assert!(stats.total_documents >= 20, "최소 20개 이상 등록 (실제: {})", stats.total_documents);
    assert!(total_output_chars > 0, "출력이 있어야 함");
}

// ═══════════════════════════════════════════════════════════════
// 벤치마크: 증분 컴파일 — 동일 30개 재투입 시 스킵
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn benchmark_incremental_skip() {
    let (base, service) = setup_bench();
    let inbox = base.path().join("inbox");

    // 5개만 먼저 처리
    let mut files = Vec::new();
    for i in 0..5 {
        let (name, content) = generate_meeting(i + 1);
        let p = inbox.join(&name);
        std::fs::write(&p, &content).unwrap();
        files.push((p, content));
    }

    for (f, content) in &files {
        // 파일이 삭제됐을 수 있으므로 다시 쓰기
        if !f.exists() {
            std::fs::write(f, content).unwrap();
        }
        match service.process_file(f).await {
            Ok(()) => {
                let db = service.vector_db.stats().unwrap();
                println!("  OK: {:?} (DB: {})", f.file_name(), db.total_documents);
            }
            Err(e) => println!("  ERR: {:?} → {}", f.file_name(), e),
        }
    }

    let stats1 = service.vector_db.stats().unwrap();
    println!("처리 후 DB 문서 수: {}", stats1.total_documents);
    assert_eq!(stats1.total_documents, 5);

    // 동일 파일 재투입 — 증분 컴파일에서 스킵되어야 함
    for (f, content) in &files {
        std::fs::write(f, content).unwrap();
        service.process_file(f).await.unwrap();
    }

    // 완전 중복 (SHA-256)으로 스킵되므로 여전히 5개
    let stats2 = service.vector_db.stats().unwrap();
    assert_eq!(stats2.total_documents, 5, "증분 스킵: 여전히 5개");

    println!("\n=== 증분 컴파일 벤치마크 ===");
    println!("첫 컴파일: 5 파일 처리");
    println!("재투입: 5 파일 스킵 (SHA-256 중복)");
}

// ═══════════════════════════════════════════════════════════════
// 벤치마크: 교차참조 업데이트 수 측정
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn benchmark_cross_references() {
    let (base, service) = setup_bench();
    let inbox = base.path().join("inbox");

    // 같은 주제(프로젝트 회의) 문서 5개
    for i in 0..5 {
        let (name, content) = generate_meeting(i + 1);
        let p = inbox.join(&name);
        std::fs::write(&p, &content).unwrap();
        let _ = service.process_file(&p).await;
    }

    // 다른 주제(학습) 문서 5개
    for i in 0..5 {
        let (name, content) = generate_study(i);
        let p = inbox.join(&name);
        std::fs::write(&p, &content).unwrap();
        let _ = service.process_file(&p).await;
    }

    let all = service.vector_db.list_all().unwrap();
    let mut meeting_relations = 0;
    let mut study_relations = 0;

    for doc in &all {
        let rels = service.vector_db.find_related(&doc.id).unwrap();
        if doc.doc_types.contains(&"meeting".to_string()) {
            meeting_relations += rels.len();
        } else {
            study_relations += rels.len();
        }
    }

    println!("\n=== 교차참조 벤치마크 ===");
    println!("회의록 5개 → 관계 {} 개", meeting_relations);
    println!("학습노트 5개 → 관계 {} 개", study_relations);
    println!("총 문서: {}", all.len());

    assert!(all.len() >= 5, "최소 5개 등록 (실제: {})", all.len());
}
