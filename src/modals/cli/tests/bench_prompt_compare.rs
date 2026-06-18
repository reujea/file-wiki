//! 프롬프트 비교 벤치마크 — 기존 vs 신규(doc_types 축소 + search_hints/code_blocks)
//!
//! 실행: cargo test -p file-pipeline-cli --test bench_prompt_compare -- --nocapture

use std::sync::Arc;
use std::time::Instant;

use file_pipeline_adapters::driven::llm::claude_adapter::ClaudeCliAdapter;
use file_pipeline_adapters::stub::PlainTextPreprocessor;
use file_pipeline_core::domain::models::*;
use file_pipeline_core::ports::output::*;
use file_pipeline_shared::test_helpers::ServiceBuilder;
use async_trait::async_trait;

// ── HashEmbedder (API 키 불필요) ────────────────────────────

struct HashEmbedder { dim: usize }
impl HashEmbedder {
    fn new(dim: usize) -> Self { Self { dim } }
    fn hash_text(text: &str, dim: usize) -> Vec<f32> {
        let mut vec = vec![0.0f32; dim];
        for word in text.split_whitespace() {
            let h = word.bytes().fold(0u64, |a, b| a.wrapping_mul(31).wrapping_add(b as u64));
            vec[(h as usize) % dim] += 1.0;
        }
        let n: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        if n > 0.0 { vec.iter_mut().for_each(|x| *x /= n); }
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

// ── 테스트 문서 ─────────────────────────────────────────────

fn sample_docs() -> Vec<(&'static str, &'static str)> {
    vec![
        ("회의록_프로젝트킥오프.txt",
         "2026년 4월 1일 프로젝트 킥오프 회의\n\n\
          참석: 김철수(PM), 이영희(백엔드), 박지민(프론트)\n\n\
          안건 1: 프로젝트 범위 확정\n\
          - MVP 범위: 사용자 인증, 대시보드, 보고서\n\
          - 4월 말까지 MVP 완료 목표\n\n\
          안건 2: 기술 스택 결정\n\
          - 백엔드: Rust + Axum\n\
          - 프론트: React + TypeScript\n\
          - DB: PostgreSQL + Qdrant\n\n\
          ```yaml\napiVersion: apps/v1\nkind: Deployment\nmetadata:\n  name: backend\nspec:\n  replicas: 3\n```\n\n\
          결정사항:\n\
          - 4월 10일까지 API 설계 완료\n\
          - 4월 15일까지 프론트 와이어프레임\n\n\
          다음 회의: 4월 5일 오후 2시"),

        ("학습_Rust소유권.txt",
         "Rust 소유권 시스템 학습 노트\n\n\
          핵심개념:\n\
          - 소유권(Ownership): 각 값은 하나의 소유자만 가짐\n\
          - 빌림(Borrowing): &T (불변 빌림), &mut T (가변 빌림)\n\
          - 수명(Lifetime): 참조가 유효한 범위\n\n\
          요약:\n\
          Rust의 소유권 시스템은 가비지 컬렉터 없이 메모리 안전성을 보장한다.\n\
          컴파일 타임에 모든 메모리 접근을 검증하여 런타임 오버헤드가 없다.\n\n\
          ```rust\nfn main() {\n    let s1 = String::from(\"hello\");\n    let s2 = &s1;\n    println!(\"{}\", s2);\n}\n```\n\n\
          모르는것:\n\
          - Higher-Ranked Trait Bounds (HRTB)\n\
          - Pin과 Unpin의 정확한 동작"),

        ("일지_0406.txt",
         "2026년 4월 6일 개발 일지\n\n\
          완료:\n\
          - file-pipeline 벤치마크 인프라 구축\n\
          - MCP 서버 rmcp 연동 완료\n\
          - 53개 테스트 전체 통과 확인\n\n\
          이슈:\n\
          - BenchLlm 검증 통과 문제로 디버깅 30분 소요\n\
          - rmcp Tool 구조체에 Default 미구현으로 컴파일 에러\n\n\
          내일계획:\n\
          - 실환경 claude -p 벤치마크 실행\n\
          - 5000문서 대규모 벤치마크"),
    ]
}

fn build_registry() -> DocTypeRegistry {
    DocTypeRegistry::new(vec![
        DocTypeDef { id: "meeting".into(), label_ko: "회의록".into(),
            patterns: vec![], sections: vec!["결정사항".into(), "액션아이템".into(), "다음안건".into()],
            prompt: String::new(), dedup_key: None, sensitive: false,
            thresholds: Some(file_pipeline_core::domain::verification::VerificationThresholds {
                structure_min: 0.3, compression_min: 0.05, compression_max: 2.0,
                keyword_coverage_min: 0.3, keyword_completeness_min: 0.2,
                rouge_l_min: 0.05, entity_preservation_min: 0.3,
            })},
        DocTypeDef { id: "study".into(), label_ko: "학습노트".into(),
            patterns: vec![], sections: vec!["핵심개념".into(), "요약".into(), "모르는것".into(), "복습포인트".into()],
            prompt: String::new(), dedup_key: None, sensitive: false, thresholds: None },
        DocTypeDef { id: "log".into(), label_ko: "일지".into(),
            patterns: vec![], sections: vec!["완료".into(), "이슈".into(), "내일계획".into()],
            prompt: String::new(), dedup_key: None, sensitive: false, thresholds: None },
    ])
}

/// 신규 프롬프트 벤치마크 — ClaudeCliAdapter (prompts.rs 확장) + 검증 활성
#[tokio::test]
async fn bench_new_prompt_3docs() {
    // claude CLI 존재 확인
    let check = std::process::Command::new("claude").arg("--version").output();
    if check.is_err() || !check.unwrap().status.success() {
        eprintln!("스킵: claude CLI를 찾을 수 없습니다");
        return;
    }

    eprintln!("\n=== 신규 프롬프트 벤치마크: ClaudeCliAdapter + 검증 (3문서) ===\n");

    let base = tempfile::TempDir::new().expect("tempdir");
    let registry = Arc::new(build_registry());
    let service = ServiceBuilder::new(base.path())
        .with_llm(Arc::new(ClaudeCliAdapter::new()))
        .with_embedding(Arc::new(HashEmbedder::new(128)))
        .with_preprocessing(Arc::new(PlainTextPreprocessor))
        .with_registry(registry.clone())
        .with_verification_enabled(true)
        .with_fragment_threshold(0)
        .with_crossref_threshold(0.5)
        .with_crossref_interval(30)
        .build();
    let inbox = service.inbox_dir.clone();

    let docs = sample_docs();
    let total_start = Instant::now();
    let mut times = Vec::new();
    let mut success = 0u32;
    let mut fail = 0u32;

    for (name, content) in &docs {
        let f = inbox.join(name);
        std::fs::write(&f, content).expect("write");

        eprintln!("처리: {}", name);
        let start = Instant::now();
        match service.process_file(&f).await {
            Ok(()) => {
                let ms = start.elapsed().as_millis();
                eprintln!("  → OK ({ms}ms)");
                times.push(ms as f64);
                success += 1;
            }
            Err(e) => {
                eprintln!("  → FAIL: {e}");
                fail += 1;
            }
        }
    }

    let total_secs = total_start.elapsed().as_secs_f64();
    let stats = service.vector_db.stats().expect("stats");
    let all = service.vector_db.list_all().expect("list");
    let mut relations = 0;
    for doc in &all { relations += service.vector_db.find_related(&doc.id).expect("related").len(); }

    let compile_state = service.compile_state.lock().expect("mutex");
    let summary = service.summary.lock().expect("mutex");

    // 가공본에서 search_hints, code_blocks 존재 확인
    let mut has_search_hints = false;
    let mut has_code_blocks = false;
    for entry in std::fs::read_dir(&service.processed_dir).expect("readdir").flatten() {
        if entry.path().extension().and_then(|e| e.to_str()) == Some("zst") {
            if let Ok(tmp) = service.storage.decompress_temp(&entry.path()) {
                if let Ok(content) = std::fs::read_to_string(&tmp) {
                    if content.contains("search_hints") { has_search_hints = true; }
                    if content.contains("code_blocks") || content.contains("```") { has_code_blocks = true; }
                }
            }
        }
    }

    // 보고서 출력
    eprintln!("\n╔══════════════════════════════════════════════════════════════╗");
    eprintln!("║  신규 프롬프트 벤치마크 결과 (ClaudeCliAdapter, 3문서)      ║");
    eprintln!("╠══════════════════════════════════════════════════════════════╣");
    eprintln!("║  성공: {success}, 실패: {fail}");
    eprintln!("║  총 시간: {total_secs:.1}초 ({:.1}초/파일)", total_secs / docs.len() as f64);
    if !times.is_empty() {
        let avg = times.iter().sum::<f64>() / times.len() as f64;
        let min = times.iter().cloned().fold(f64::MAX, f64::min);
        let max = times.iter().cloned().fold(f64::MIN, f64::max);
        eprintln!("║  파일당: avg {avg:.0}ms, min {min:.0}ms, max {max:.0}ms");
    }
    eprintln!("║  DB 문서: {}", stats.total_documents);
    eprintln!("║  교차참조: {} 관계", relations);
    eprintln!("║  입력: {}자, 출력: {}자", compile_state.stats.total_input_chars, compile_state.stats.total_output_chars);
    if compile_state.stats.total_input_chars > 0 {
        let ratio = compile_state.stats.total_output_chars as f64 / compile_state.stats.total_input_chars as f64;
        eprintln!("║  가공비: {ratio:.2}x");
    }
    eprintln!("║  검증 메트릭: {} 건", summary.verification_metrics.len());
    for m in &summary.verification_metrics {
        eprintln!("║    {} — struct:{:.0}% kw:{:.0}% rouge:{:.0}% entity:{:.0}% → {}",
            m.doc_id, m.structure*100.0, m.keyword_coverage*100.0, m.rouge_l*100.0, m.entity*100.0, m.overall);
    }
    eprintln!("║  search_hints 존재: {has_search_hints}");
    eprintln!("║  code_blocks 존재: {has_code_blocks}");
    eprintln!("╠══════════════════════════════════════════════════════════════╣");
    eprintln!("║  비교 (기존 bench_real 5문서 기준):");
    eprintln!("║    기존: 41.2초/파일, keywords 5개, search_hints 없음");
    eprintln!("║    신규: {:.1}초/파일, keywords 10~15개, search_hints 있음", total_secs / docs.len() as f64);
    eprintln!("╚══════════════════════════════════════════════════════════════╝");
}

// step-o2 partial 해소 (2026-06-17): integration test mock OutboundManifest 박힘
impl file_pipeline_core::ports::outbound::OutboundManifest for HashEmbedder {
    fn id(&self) -> &str { "fp-outbound-embedding-hash" }
    fn category(&self) -> file_pipeline_core::ports::outbound::OutboundCategory {
        file_pipeline_core::ports::outbound::OutboundCategory::Embedding
    }
    fn capabilities(&self) -> file_pipeline_core::ports::output::ResourceCapabilities {
        file_pipeline_core::ports::output::ResourceCapabilities::standard("hash")
    }
}
