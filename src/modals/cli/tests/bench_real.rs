//! C안 2단계: 실환경 벤치마크 — claude -p 단일 명령형 호출
//!
//! - LLM: 실제 claude -p "프롬프트" --session-id pipeline-bench --output-format text
//! - Embedding: HashEmbedder (API 키 불필요)
//! - VectorDB: LocalVectorStore (in-memory)
//! - 문서: 5개 (비용 최소화, 실측 목적)
//!
//! 실행: PIPELINE_REAL_BENCH=1 cargo test -p file-pipeline --test bench_real -- --nocapture

use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use file_pipeline_core::domain::models::*;
use file_pipeline_core::ports::output::*;
use file_pipeline_shared::test_helpers::ServiceBuilder;

// ── claude -p 직접 호출하는 실 LLM ─────────────────────────

struct RealClaudeLlm {
    #[allow(dead_code)]
    session_id: String,
}

impl RealClaudeLlm {
    fn new() -> Self {
        // UUID v4 형식 생성 (간이)
        let ts = chrono::Local::now().timestamp_nanos_opt().unwrap_or(0) as u64;
        let uuid = format!(
            "{:08x}-{:04x}-4{:03x}-{:04x}-{:012x}",
            (ts >> 32) as u32, ((ts >> 16) as u16),
            ts as u16 & 0x0fff, 0x8000 | (ts as u16 & 0x3fff),
            ts & 0xffffffffffff
        );
        Self { session_id: uuid }
    }

    fn call(&self, prompt: &str) -> anyhow::Result<String> {
        let output = Command::new("claude")
            .arg("-p")
            .arg(prompt)
            .args(["--output-format", "text"])
            .output()?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("claude 오류: {}", stderr);
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn parse_json(raw: &str) -> Option<serde_json::Value> {
        // JSON 블록 추출
        if let Some(start) = raw.find('{') {
            let mut depth = 0i32;
            let mut end = start;
            for (i, ch) in raw[start..].char_indices() {
                match ch {
                    '{' => depth += 1,
                    '}' => { depth -= 1; if depth == 0 { end = start + i + 1; break; } }
                    _ => {}
                }
            }
            serde_json::from_str(&raw[start..end]).ok()
        } else {
            None
        }
    }
}

#[async_trait]
impl LLMPort for RealClaudeLlm {
    async fn classify_and_process(
        &self,
        file_path: &Path,
        registry: &DocTypeRegistry,
    ) -> anyhow::Result<ClassifyAndProcessResult> {
        let content = std::fs::read_to_string(file_path)?;
        let filename = file_path.file_name().unwrap_or_default().to_string_lossy();

        // 유형 힌트
        let mut hints = String::from("알려진 유형:\n");
        for def in registry.all() {
            hints.push_str(&format!("- {}: {} (섹션: {})\n", def.id, def.label_ko, def.sections.join(", ")));
        }

        let prompt = format!(
            r#"문서를 분석하고 JSON만 출력하세요.

{hints}

출력 형식:
```json
{{"doc_types":["meeting"],"rationale":"이유","date":"2026-04-06","summary":"요약","keywords":["k1","k2"],"content":"=== 섹션 ===\n가공 내용"}}
```

파일: {filename}
내용:
{content}"#,
            hints = hints,
            filename = filename,
            content = if content.len() > 20000 { &content[..20000] } else { &content },
        );

        let start = Instant::now();
        let raw = self.call(&prompt)?;
        let llm_ms = start.elapsed().as_millis();
        eprintln!("  claude -p 호출: {}ms ({}자 입력 → {}자 출력)", llm_ms, prompt.len(), raw.len());

        let json = Self::parse_json(&raw)
            .ok_or_else(|| anyhow::anyhow!("JSON 파싱 실패: {}", &raw[..raw.len().min(200)]))?;

        let doc_types: Vec<String> = json["doc_types"].as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_else(|| vec!["etc".into()]);
        let rationale = json["rationale"].as_str().unwrap_or("").to_string();
        let date = json["date"].as_str().unwrap_or("2026-04-06").to_string();
        let summary = json["summary"].as_str().unwrap_or("").to_string();
        let keywords: Vec<String> = json["keywords"].as_array()
            .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
            .unwrap_or_default();
        let processed_content = json["content"].as_str().unwrap_or(&raw).to_string();

        let metadata = Metadata {
            doc_types: doc_types.clone(), rationale: rationale.clone(),
            date, summary, keywords, sensitive: false, doi: None, related_docs: vec![], source_doc_ids: vec![], search_hints: vec![],
            entities: vec![],
            ..Default::default()        };

        Ok(ClassifyAndProcessResult { doc_types, rationale, content: processed_content, metadata, sections: None })
    }

    async fn summarize_text(&self, new: &str, existing: &str) -> anyhow::Result<String> {
        let prompt = format!(
            "기존 할일과 새 할일을 병합하세요. 아이젠하워 매트릭스로 정리.\n\n기존:\n{}\n\n새:\n{}",
            existing, new
        );
        self.call(&prompt)
    }

    async fn enrich_existing(&self, existing: &str, new_info: &str, doc_types: &[String]) -> anyhow::Result<EnrichResult> {
        let prompt = format!(
            "기존 문서에 새 정보를 통합하세요. 변경 없으면 NO_CHANGE.\n유형: {}\n\n기존:\n{}\n\n새:\n{}",
            doc_types.join(", "),
            if existing.len() > 5000 { &existing[..5000] } else { existing },
            if new_info.len() > 3000 { &new_info[..3000] } else { new_info },
        );
        let raw = self.call(&prompt)?;
        if raw.contains("NO_CHANGE") {
            Ok(EnrichResult { updated_content: existing.into(), change_summary: String::new(), should_update: false })
        } else {
            Ok(EnrichResult { updated_content: raw.clone(), change_summary: "보강됨".into(), should_update: true })
        }
    }
}

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
          결정사항:\n\
          - 4월 10일까지 API 설계 완료\n\
          - 4월 15일까지 프론트 와이어프레임\n\n\
          다음 회의: 4월 5일 오후 2시"),

        ("회의록_스프린트리뷰.txt",
         "2026년 4월 5일 스프린트 1 리뷰\n\n\
          참석: 김철수, 이영희, 박지민, 최동훈(QA)\n\n\
          완료 항목:\n\
          - API 설계 문서 v1 완료\n\
          - 사용자 인증 모듈 구현 (JWT 기반)\n\
          - 프론트 라우팅 구조 확정\n\n\
          이슈:\n\
          - Qdrant 연동에서 dim 불일치 문제 발견\n\
          - CORS 설정 누락으로 프론트 테스트 지연\n\n\
          다음 스프린트 계획:\n\
          - 대시보드 API 구현\n\
          - 벡터 검색 통합\n\
          - E2E 테스트 작성"),

        ("학습_Rust소유권.txt",
         "Rust 소유권 시스템 학습 노트\n\n\
          핵심개념:\n\
          - 소유권(Ownership): 각 값은 하나의 소유자만 가짐\n\
          - 빌림(Borrowing): &T (불변 빌림), &mut T (가변 빌림)\n\
          - 수명(Lifetime): 참조가 유효한 범위\n\n\
          요약:\n\
          Rust의 소유권 시스템은 가비지 컬렉터 없이 메모리 안전성을 보장한다.\n\
          컴파일 타임에 모든 메모리 접근을 검증하여 런타임 오버헤드가 없다.\n\n\
          모르는것:\n\
          - Higher-Ranked Trait Bounds (HRTB)\n\
          - Pin과 Unpin의 정확한 동작\n\n\
          복습포인트:\n\
          - The Book Chapter 4, 10, 19 재독\n\
          - lifetime elision 규칙 암기"),

        ("일지_0406.txt",
         "2026년 4월 6일 개발 일지\n\n\
          완료:\n\
          - file-pipeline 벤치마크 인프라 구축\n\
          - MCP 서버 rmcp 연동 완료\n\
          - 53개 테스트 전체 통과 확인\n\
          - stub 대규모 벤치마크 100/500/1000 문서 실측\n\n\
          이슈:\n\
          - BenchLlm 검증 통과 문제로 디버깅 30분 소요\n\
          - rmcp Tool 구조체에 Default 미구현으로 컴파일 에러\n\n\
          내일계획:\n\
          - 실환경 claude -p 벤치마크 실행\n\
          - 5000문서 대규모 벤치마크\n\
          - 벤치마크 보고서 최종 정리"),

        ("학습_비동기프로그래밍.txt",
         "Rust 비동기 프로그래밍 학습\n\n\
          핵심개념:\n\
          - async/await: 비동기 함수 정의와 실행\n\
          - Future trait: poll 기반 게으른 실행\n\
          - tokio 런타임: 멀티스레드 비동기 실행기\n\n\
          요약:\n\
          Rust의 async/await는 제로코스트 추상화를 제공한다.\n\
          Future는 poll될 때까지 실행되지 않는 게으른(lazy) 값이다.\n\
          tokio는 work-stealing 스케줄러로 효율적인 태스크 분배를 한다.\n\n\
          모르는것:\n\
          - async trait의 제한사항과 해결법\n\
          - Pin의 필요성과 self-referential struct\n\n\
          복습포인트:\n\
          - tokio tutorial 전체\n\
          - async-book 읽기"),
    ]
}

// ── 벤치마크 실행 ───────────────────────────────────────────

/// 실환경 벤치마크 — PIPELINE_REAL_BENCH=1 환경변수로 활성화
#[tokio::test]
async fn bench_real_claude_5docs() {
    // claude CLI 존재 확인 — 없으면 스킵
    let claude_check = Command::new("claude").arg("--version").output();
    if claude_check.is_err() || !claude_check.unwrap().status.success() {
        eprintln!("스킵: claude CLI를 찾을 수 없습니다");
        return;
    }

    eprintln!("\n=== 실환경 벤치마크: claude -p + HashEmbedder (5문서) ===\n");

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

    let registry = DocTypeRegistry::new(vec![
        DocTypeDef { id: "meeting".into(), label_ko: "회의록".into(), patterns: vec!["회의".into()],
            sections: vec!["결정사항".into(), "액션아이템".into(), "다음안건".into()],
            prompt: String::new(), dedup_key: None, sensitive: false, thresholds: None },
        DocTypeDef { id: "study".into(), label_ko: "학습노트".into(), patterns: vec!["학습".into()],
            sections: vec!["핵심개념".into(), "요약".into(), "모르는것".into(), "복습포인트".into()],
            prompt: String::new(), dedup_key: None, sensitive: false, thresholds: None },
        DocTypeDef { id: "log".into(), label_ko: "일지".into(), patterns: vec!["일지".into()],
            sections: vec!["완료".into(), "이슈".into(), "내일계획".into()],
            prompt: String::new(), dedup_key: None, sensitive: false, thresholds: None },
    ]);

    let service = ServiceBuilder::new(base.path())
        .with_llm(Arc::new(RealClaudeLlm::new()))
        .with_embedding(Arc::new(HashEmbedder::new(128)))
        .with_registry(Arc::new(registry))
        .with_semantic_dup_threshold(0.03)
        .with_verification_enabled(true)
        .with_fragment_threshold(0)
        .with_crossref_threshold(0.5)
        .with_crossref_interval(30)
        .build();

    // 문서 생성 + 처리
    let docs = sample_docs();
    let total_start = Instant::now();
    let mut times = Vec::new();
    let mut total_input_chars = 0u64;
    let mut success = 0u32;
    let mut fail = 0u32;

    for (name, content) in &docs {
        let f = inbox.join(name);
        std::fs::write(&f, content).unwrap();
        total_input_chars += content.len() as u64;

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

    // 결과 수집
    let stats = service.vector_db.stats().unwrap();
    let all = service.vector_db.list_all().unwrap();
    let mut relations = 0;
    for doc in &all { relations += service.vector_db.find_related(&doc.id).unwrap().len(); }

    let processed_size: u64 = std::fs::read_dir(&processed).unwrap()
        .filter_map(|e| e.ok()).filter_map(|e| e.metadata().ok()).map(|m| m.len()).sum();
    let originals_size: u64 = std::fs::read_dir(&originals).unwrap()
        .filter_map(|e| e.ok()).filter_map(|e| e.metadata().ok()).map(|m| m.len()).sum();

    let compile_state = service.compile_state.lock().unwrap();

    // 보고서 출력
    eprintln!("\n╔══════════════════════════════════════════════════════╗");
    eprintln!("║  실환경 벤치마크 결과 (claude -p, 5문서)             ║");
    eprintln!("╠══════════════════════════════════════════════════════╣");
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
    eprintln!("║  입력: {}KB", total_input_chars / 1024);
    eprintln!("║  가공본.zst: {}B, 원본.zst: {}B", processed_size, originals_size);
    eprintln!("║  입력 chars: {}, 출력 chars: {}", compile_state.stats.total_input_chars, compile_state.stats.total_output_chars);
    if compile_state.stats.total_input_chars > 0 {
        let ratio = compile_state.stats.total_input_chars as f64 / compile_state.stats.total_output_chars.max(1) as f64;
        eprintln!("║  압축률: {ratio:.1}x");
    }
    eprintln!("╠══════════════════════════════════════════════════════╣");
    eprintln!("║  1000문서 추정: {:.0}분, ${:.2}",
        total_secs / docs.len() as f64 * 1000.0 / 60.0,
        compile_state.stats.estimated_input_tokens as f64 * 3.0 / 1_000_000.0 * 200.0  // 5문서 → 1000문서 비례
    );
    eprintln!("╚══════════════════════════════════════════════════════╝");
}
