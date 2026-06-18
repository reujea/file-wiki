//! 실제 문서 벤치마크 — D:\file-test 의 실제 파일 (PDF 제외)
//!
//! - LLM: ClaudeCliAdapter (실제 Claude CLI 호출)
//! - Embedding: HashEmbedder (API 키 불필요)
//! - Preprocessing: CompositePreprocessor (DOCX/XLSX 네이티브 폴백)
//! - VectorDB: LocalVectorStore (in-memory)
//!
//! 실행: PIPELINE_REAL_BENCH=1 cargo test -p file-pipeline-cli --test bench_real_docs -- --nocapture
//!
//! 비용 주의: 실제 Claude CLI 호출 발생

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use file_pipeline_adapters::driven::llm::claude_adapter::ClaudeCliAdapter;
use file_pipeline_adapters::driven::preprocessing::preprocessor::CompositePreprocessor;
use file_pipeline_core::domain::models::*;
use file_pipeline_core::ports::output::*;
use file_pipeline_core::service::FileProcessingService;
use file_pipeline_shared::test_helpers::ServiceBuilder;

// ── HashEmbedder (API 키 불필요) ────────────────────────────

struct HashEmbedder {
    dim: usize,
}
impl HashEmbedder {
    fn new(dim: usize) -> Self {
        Self { dim }
    }
    fn hash_text(text: &str, dim: usize) -> Vec<f32> {
        let mut vec = vec![0.0f32; dim];
        for word in text.split_whitespace() {
            let h = word
                .bytes()
                .fold(0u64, |a, b| a.wrapping_mul(31).wrapping_add(b as u64));
            vec[(h as usize) % dim] += 1.0;
        }
        let n: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        if n > 0.0 {
            vec.iter_mut().for_each(|x| *x /= n);
        }
        vec
    }
}
#[async_trait]
impl EmbeddingPort for HashEmbedder {
    fn dim(&self) -> usize {
        self.dim
    }
    async fn embed(&self, text: &str) -> anyhow::Result<Vec<f32>> {
        Ok(Self::hash_text(text, self.dim))
    }
    async fn embed_batch(&self, texts: &[String]) -> anyhow::Result<Vec<Vec<f32>>> {
        Ok(texts.iter().map(|t| Self::hash_text(t, self.dim)).collect())
    }
}

// ── 대상 파일 수집 (PDF 제외) ───────────────────────────────

fn collect_test_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    collect_recursive(root, &mut files);
    files.sort();
    files
}

fn collect_recursive(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_recursive(&path, out);
        } else if path.is_file() {
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_lowercase();
            // PDF 제외, 대용량 PDF 2개도 제외 (이미 D:\file-test에 없을 수 있지만 안전장치)
            match ext.as_str() {
                "pdf" => {} // PDF 제외
                "txt" | "docx" | "xlsx" | "xls" | "png" | "jpg" | "jpeg" | "csv" | "log"
                | "md" | "json" => {
                    out.push(path);
                }
                _ => {
                    // 확장자 없는 파일 (catalina.out 등) — 파일명에 "catalina" 또는 로그 패턴
                    let name = path.file_name().unwrap_or_default().to_string_lossy();
                    if name.contains("catalina") || name.contains(".out.") || name.contains(".log") {
                        out.push(path);
                    }
                }
            }
        }
    }
}

// ── 서비스 구성 ─────────────────────────────────────────────

fn setup_service(base: &Path) -> FileProcessingService {
    let registry = DocTypeRegistry::new(vec![
        DocTypeDef {
            id: "guide".into(),
            label_ko: "가이드".into(),
            patterns: vec!["가이드".into(), "매뉴얼".into(), "설치".into()],
            sections: vec!["개요".into(), "절차".into(), "주의사항".into()],
            prompt: String::new(),
            dedup_key: None,
            sensitive: false,
            thresholds: None,
        },
        DocTypeDef {
            id: "reference".into(),
            label_ko: "참고자료".into(),
            patterns: vec!["참고".into(), "비교".into()],
            sections: vec!["요약".into(), "핵심내용".into(), "결론".into()],
            prompt: String::new(),
            dedup_key: None,
            sensitive: false,
            thresholds: None,
        },
        DocTypeDef {
            id: "log".into(),
            label_ko: "로그/이슈".into(),
            patterns: vec!["에러".into(), "이슈".into(), "오류".into()],
            sections: vec!["증상".into(), "원인".into(), "해결".into()],
            prompt: String::new(),
            dedup_key: None,
            sensitive: false,
            thresholds: None,
        },
        DocTypeDef {
            id: "report".into(),
            label_ko: "보고서".into(),
            patterns: vec!["보고".into(), "분석".into(), "결과".into()],
            sections: vec!["요약".into(), "분석".into(), "결론".into()],
            prompt: String::new(),
            dedup_key: None,
            sensitive: false,
            thresholds: None,
        },
    ]);

    ServiceBuilder::new(base)
        .with_llm(Arc::new(ClaudeCliAdapter::new()))
        .with_embedding(Arc::new(HashEmbedder::new(128)))
        .with_preprocessing(Arc::new(CompositePreprocessor::new("none", "none")))
        .with_registry(Arc::new(registry))
        .with_semantic_dup_threshold(0.03)
        .with_fragment_threshold(0)
        .with_crossref_interval(0)
        .build()
}

// ── 개별 파일 결과 ──────────────────────────────────────────

struct FileResult {
    name: String,
    #[allow(dead_code)]
    size_bytes: u64,
    elapsed_ms: u64,
    success: bool,
    error: Option<String>,
    ext: String,
}

// ── 벤치마크 실행 ───────────────────────────────────────────

/// 실제 문서 벤치마크 — PIPELINE_REAL_BENCH=1 환경변수로 활성화
#[tokio::test]
async fn bench_real_docs_from_file_test() {
    if std::env::var("PIPELINE_REAL_BENCH").is_err() {
        eprintln!("스킵: PIPELINE_REAL_BENCH=1 환경변수 필요");
        return;
    }

    // Claude CLI 존재 확인
    let claude_check = std::process::Command::new("claude")
        .arg("--version")
        .output();
    if claude_check.is_err() || !claude_check.unwrap().status.success() {
        eprintln!("스킵: Claude CLI를 찾을 수 없습니다");
        return;
    }

    let source_dir = PathBuf::from("D:/file-test");
    if !source_dir.exists() {
        eprintln!("스킵: D:\\file-test 디렉토리 없음");
        return;
    }

    let files = collect_test_files(&source_dir);
    if files.is_empty() {
        eprintln!("스킵: 대상 파일 없음");
        return;
    }

    eprintln!("\n╔══════════════════════════════════════════════════════════════╗");
    eprintln!("║  실제 문서 벤치마크: ClaudeCliAdapter + CompositePreprocessor  ║");
    eprintln!("║  대상: D:\\file-test (PDF 제외)                               ║");
    eprintln!("║  파일: {} 개                                                  ║", files.len());
    eprintln!("╠══════════════════════════════════════════════════════════════╣");

    let base = tempfile::TempDir::new().expect("임시 디렉토리 생성 실패");
    let service = setup_service(base.path());
    let inbox = base.path().join("inbox");

    // 파일을 inbox에 복사
    let mut total_input_bytes: u64 = 0;
    let mut inbox_files: Vec<(PathBuf, PathBuf, u64)> = Vec::new(); // (원본, inbox경로, 크기)
    for src in &files {
        let size = std::fs::metadata(src).map(|m| m.len()).unwrap_or(0);
        total_input_bytes += size;
        let _name = src.file_name().unwrap_or_default();
        // 파일명 중복 방지: 상대경로를 _ 로 연결
        let rel = src
            .strip_prefix(&source_dir)
            .unwrap_or(src)
            .to_string_lossy()
            .replace(['/', '\\'], "_");
        let dest = inbox.join(&rel);
        std::fs::copy(src, &dest).expect("파일 복사 실패");
        inbox_files.push((src.clone(), dest, size));
    }

    eprintln!("║  총 입력: {} KB ({} 파일)                          ║", total_input_bytes / 1024, files.len());
    eprintln!("╠══════════════════════════════════════════════════════════════╣\n");

    // 배치 모드 시작
    service.vector_db.batch_begin();
    service.compile_state_batch_begin();

    let total_start = Instant::now();
    let mut results: Vec<FileResult> = Vec::new();

    for (src, inbox_path, size) in &inbox_files {
        let ext = src
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("none")
            .to_lowercase();
        let name = inbox_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let display_name: String = name.chars().take(50).collect();
        eprint!("  [{:>2}/{:>2}] {} ({} KB, .{}) ... ",
            results.len() + 1,
            inbox_files.len(),
            display_name,
            size / 1024,
            ext,
        );

        let start = Instant::now();
        match service.process_file(inbox_path).await {
            Ok(()) => {
                let ms = start.elapsed().as_millis() as u64;
                eprintln!("OK ({:.1}s)", ms as f64 / 1000.0);
                results.push(FileResult {
                    name,
                    size_bytes: *size,
                    elapsed_ms: ms,
                    success: true,
                    error: None,
                    ext,
                });
            }
            Err(e) => {
                let ms = start.elapsed().as_millis() as u64;
                let err_msg = format!("{}", e);
                eprintln!("FAIL ({:.1}s): {}", ms as f64 / 1000.0, &err_msg[..err_msg.len().min(100)]);
                results.push(FileResult {
                    name,
                    size_bytes: *size,
                    elapsed_ms: ms,
                    success: false,
                    error: Some(err_msg),
                    ext,
                });
            }
        }
    }

    let process_secs = total_start.elapsed().as_secs_f64();

    // 배치 종료
    let t_batch = Instant::now();
    service.vector_db.batch_end();
    service.compile_state_batch_end();
    let batch_end_secs = t_batch.elapsed().as_secs_f64();

    // 교차참조 flush
    let t_flush = Instant::now();
    let _ = service.flush_crossref();
    let flush_secs = t_flush.elapsed().as_secs_f64();

    let total_secs = process_secs + batch_end_secs + flush_secs;

    // ── 결과 수집 ──
    let success_count = results.iter().filter(|r| r.success).count();
    let fail_count = results.iter().filter(|r| !r.success).count();
    let success_times: Vec<f64> = results
        .iter()
        .filter(|r| r.success)
        .map(|r| r.elapsed_ms as f64)
        .collect();

    let stats = service.vector_db.stats().unwrap_or_default();
    let all_docs = service.vector_db.list_all().unwrap_or_default();
    let mut relation_count = 0;
    for doc in &all_docs {
        relation_count += service.vector_db.find_related(&doc.id).unwrap_or_default().len();
    }

    let processed_bytes: u64 = std::fs::read_dir(base.path().join("processed"))
        .into_iter()
        .flat_map(|d| d.flatten())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum();
    let originals_bytes: u64 = std::fs::read_dir(base.path().join("originals"))
        .into_iter()
        .flat_map(|d| d.flatten())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum();

    // 검색 성능 측정
    let search_queries = ["MariaDB 설치", "deadlock 해결", "failover 설정", "방화벽 정책", "트랜잭션 분석"];
    let embedding = HashEmbedder::new(128);
    let mut search_times: Vec<f64> = Vec::new();
    for query in &search_queries {
        let emb = embedding.embed(query).await.unwrap_or_default();
        let t = Instant::now();
        let _ = service.vector_db.search_hybrid(&emb, query, 5);
        search_times.push(t.elapsed().as_secs_f64() * 1000.0);
    }
    let search_avg = search_times.iter().sum::<f64>() / search_times.len().max(1) as f64;
    search_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let search_p95 = search_times.get((search_times.len() as f64 * 0.95) as usize).copied().unwrap_or(0.0);

    // ── 보고서 출력 ──
    eprintln!("\n╔══════════════════════════════════════════════════════════════╗");
    eprintln!("║                    벤치마크 결과 보고서                       ║");
    eprintln!("╠══════════════════════════════════════════════════════════════╣");
    eprintln!("║  파일: {} 개 (성공 {}, 실패 {})", inbox_files.len(), success_count, fail_count);
    eprintln!("║");
    eprintln!("║  [시간]");
    eprintln!("║    처리:       {:.1}s", process_secs);
    eprintln!("║    batch_end:  {:.2}s", batch_end_secs);
    eprintln!("║    flush:      {:.2}s", flush_secs);
    eprintln!("║    총:         {:.1}s ({:.1}s/파일)", total_secs, total_secs / success_count.max(1) as f64);

    if !success_times.is_empty() {
        let avg = success_times.iter().sum::<f64>() / success_times.len() as f64;
        let mut sorted = success_times.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let p50 = sorted[sorted.len() / 2];
        let p95 = sorted[(sorted.len() as f64 * 0.95) as usize];
        let max = sorted.last().copied().unwrap_or(0.0);
        let min = sorted.first().copied().unwrap_or(0.0);
        eprintln!("║");
        eprintln!("║  [파일당 소요시간 (성공 건)]");
        eprintln!("║    avg:  {:.1}s", avg / 1000.0);
        eprintln!("║    p50:  {:.1}s", p50 / 1000.0);
        eprintln!("║    p95:  {:.1}s", p95 / 1000.0);
        eprintln!("║    min:  {:.1}s", min / 1000.0);
        eprintln!("║    max:  {:.1}s", max / 1000.0);
    }

    eprintln!("║");
    eprintln!("║  [저장]");
    eprintln!("║    입력:     {} KB", total_input_bytes / 1024);
    eprintln!("║    가공본:   {} KB", processed_bytes / 1024);
    eprintln!("║    원본.zst: {} KB", originals_bytes / 1024);
    if total_input_bytes > 0 {
        let ratio = (processed_bytes + originals_bytes) as f64 / total_input_bytes as f64 * 100.0;
        eprintln!("║    압축률:   {:.1}%", ratio);
    }

    eprintln!("║");
    eprintln!("║  [벡터DB]");
    eprintln!("║    등록 문서: {}", stats.total_documents);
    eprintln!("║    교차참조:  {} 관계", relation_count);
    eprintln!("║    검색 avg:  {:.2}ms", search_avg);
    eprintln!("║    검색 p95:  {:.2}ms", search_p95);

    // 확장자별 통계
    eprintln!("║");
    eprintln!("║  [확장자별]");
    let mut ext_stats: std::collections::HashMap<String, (usize, usize, f64)> = std::collections::HashMap::new();
    for r in &results {
        let entry = ext_stats.entry(r.ext.clone()).or_default();
        entry.0 += 1; // 전체
        if r.success {
            entry.1 += 1; // 성공
            entry.2 += r.elapsed_ms as f64;
        }
    }
    for (ext, (total, success, time_ms)) in &ext_stats {
        let avg_s = if *success > 0 { *time_ms / *success as f64 / 1000.0 } else { 0.0 };
        eprintln!("║    .{:<6} {:>2}/{:>2} 성공  avg {:.1}s", ext, success, total, avg_s);
    }

    // 실패 상세
    if fail_count > 0 {
        eprintln!("║");
        eprintln!("║  [실패 상세]");
        for r in results.iter().filter(|r| !r.success) {
            let err = r.error.as_deref().unwrap_or("unknown");
            eprintln!("║    {} → {}", r.name, &err[..err.len().min(80)]);
        }
    }

    eprintln!("╚══════════════════════════════════════════════════════════════╝");

    // 성공률 확인 (최소 50% 이상 성공해야 유의미)
    assert!(
        success_count as f64 / inbox_files.len() as f64 >= 0.5,
        "성공률 50% 미달: {}/{}",
        success_count,
        inbox_files.len()
    );
}
