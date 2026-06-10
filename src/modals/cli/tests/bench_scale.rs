//! C안 1단계: stub 대규모 벤치마크 (100/500/1000/5000 문서)
//!
//! 측정: 처리량(docs/sec), 압축률, 교차참조, 검색 속도, 토큰 추정

use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use file_pipeline_core::domain::diagnostics::{
    BenchmarkSnapshot, CrossrefMetrics, SearchMetrics, StorageMetrics,
    ThroughputMetrics, snapshot_filename,
};
use file_pipeline_core::domain::incremental::BenchmarkReport;
use file_pipeline_core::domain::models::*;
use file_pipeline_core::ports::output::*;
use file_pipeline_core::service::FileProcessingService;
use file_pipeline_shared::test_helpers::ServiceBuilder;

// ── 어댑터 (빠른 처리용) ────────────────────────────────────

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

struct FastLlm;
#[async_trait]
impl LLMPort for FastLlm {
    async fn classify_and_process(&self, file_path: &Path, registry: &DocTypeRegistry) -> anyhow::Result<ClassifyAndProcessResult> {
        let content = std::fs::read_to_string(file_path)?;
        let filename = file_path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
        let mut doc_types: Vec<String> = Vec::new();
        if filename.contains("회의") { doc_types.push("meeting".into()); }
        if filename.contains("학습") { doc_types.push("study".into()); }
        if filename.contains("일지") { doc_types.push("log".into()); }
        if doc_types.is_empty() { doc_types.push("etc".into()); }

        let mut processed = String::new();
        let content_lines: Vec<&str> = content.lines().collect();
        let lines_per_section = (content_lines.len() / 3).max(3);
        let mut line_idx = 0;
        let mod_len = content_lines.len().max(1);
        for dt in &doc_types {
            let sections = registry.sections_for(dt);
            if sections.is_empty() {
                processed.push_str(&format!("=== {} ===\n", dt));
                for line in &content_lines[..content_lines.len().min(lines_per_section)] {
                    processed.push_str(line); processed.push('\n');
                }
            }
            for sec in &sections {
                processed.push_str(&format!("=== {} ===\n", sec));
                let end = (line_idx + lines_per_section).min(content_lines.len());
                for line in &content_lines[line_idx..end] {
                    processed.push_str(line); processed.push('\n');
                }
                line_idx = end % mod_len;
            }
        }

        let keywords: Vec<String> = content.split_whitespace().filter(|w| w.chars().count() >= 2).take(12).map(String::from).collect();
        let metadata = Metadata {
            doc_types: doc_types.clone(), rationale: "bench".into(),
            date: "2026-04-06".into(), summary: format!("bench: {}", filename),
            keywords, sensitive: false, doi: None, related_docs: vec![], source_doc_ids: vec![], search_hints: vec![],
            entities: vec![],
            ..Default::default()        };
        Ok(ClassifyAndProcessResult { doc_types, rationale: "bench".into(), content: processed, metadata, sections: None })
    }
    async fn summarize_text(&self, new: &str, existing: &str) -> anyhow::Result<String> { Ok(format!("{}\n{}", existing, new)) }
    async fn enrich_existing(&self, existing: &str, _: &str, _: &[String]) -> anyhow::Result<EnrichResult> {
        Ok(EnrichResult { updated_content: existing.into(), change_summary: String::new(), should_update: false })
    }
}

// ── 문서 생성기 ─────────────────────────────────────────────

fn generate_doc(idx: usize) -> (String, String) {
    let doc_type = match idx % 3 {
        0 => "회의록",
        1 => "학습",
        _ => "일지",
    };
    let name = format!("{}_{:05}.txt", doc_type, idx);

    // 각 문서가 고유하도록 idx를 직접 포함
    let content = format!(
        "{} 문서 #{} 고유식별자={}\n\n\
         2026년 4월 {}일 작성\n\
         참석자: 김철수, 이영희, 박지민, 최동훈, 정수연\n\n\
         첫 번째 주제: 프로젝트 알파의 진행 상황을 보고하고 다음 단계를 논의합니다.\n\
         백엔드 API 개발이 {}% 완료되었으며 프론트엔드 작업이 진행 중입니다.\n\
         QA 테스트 계획을 수립하며 버전 {}.{}.{} 릴리스를 준비합니다.\n\n\
         두 번째 주제: 기술 스택 검토 및 성능 최적화 방안을 논의합니다.\n\
         문서번호 DOC-{:05} 기준으로 작업 진행 중입니다.\n\
         벤치마크 결과 처리량 {} ops/s로 목표치의 {}%를 달성했습니다.\n\n\
         결정사항: 4월 {}일까지 MVP 완료, 테스트 {} 건 통과 목표.\n\
         액션아이템: 담당자{} CI/CD, 담당자{} 문서, 담당자{} 프론트.\n\
         다음 회의: 4월 {}일 오후 {}시.\n",
        doc_type, idx, idx * 7919 + 1234567, // 고유 해시 시드
        (idx % 28) + 1,
        (idx * 7) % 100,
        idx / 1000, (idx / 100) % 10, idx % 100,
        idx,
        idx * 13 + 42,
        (idx * 3 + 17) % 100,
        (idx % 25) + 3,
        idx * 5 + 10,
        (idx % 5) + 1, (idx % 5) + 2, (idx % 5) + 3,
        (idx % 28) + 2,
        (idx % 8) + 1,
    );
    (name, content)
}

// ── 벤치마크 엔진 ───────────────────────────────────────────

#[allow(dead_code)]
fn setup_service(base: &Path) -> FileProcessingService {
    setup_service_with_threshold(base, 0.7)
}

fn setup_service_with_threshold(base: &Path, threshold: f32) -> FileProcessingService {
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

    ServiceBuilder::new(base)
        .with_llm(Arc::new(FastLlm))
        .with_embedding(Arc::new(HashEmbedder::new(128)))
        .with_registry(Arc::new(registry))
        .with_semantic_dup_threshold(0.03)
        .with_verification_enabled(true)
        .with_fragment_threshold(0)
        .with_crossref_threshold(threshold)
        .build()
}

struct ScaleResult {
    doc_count: usize,
    total_secs: f64,
    process_secs: f64,
    batch_end_secs: f64,
    flush_secs: f64,
    docs_per_sec: f64,
    input_bytes: u64,
    processed_bytes: u64,
    originals_bytes: u64,
    compression_pct: f64,
    relation_count: usize,
    unique_pairs: usize,
    double_ratio: f64,
    isolated_docs: usize,
    search_avg_ms: f64,
    search_p95_ms: f64,
    search_queries: usize,
    per_doc_avg_ms: f64,
    per_doc_p50_ms: f64,
    per_doc_p95_ms: f64,
    per_doc_max_ms: f64,
    estimated_tokens: u64,
    estimated_cost_usd: f64,
}

impl ScaleResult {
    fn print(&self) {
        println!(
            "  {:>5} 문서 | {:.2}초 | {:.1} docs/s | 입력 {}KB → 가공 {}KB + 원본 {}KB ({:.0}% 절감) | \
             관계 {} | 검색 {:.2}ms (p95 {:.2}ms) | ~{}K tokens ~${:.4}",
            self.doc_count,
            self.total_secs,
            self.docs_per_sec,
            self.input_bytes / 1024,
            self.processed_bytes / 1024,
            self.originals_bytes / 1024,
            self.compression_pct,
            self.relation_count,
            self.search_avg_ms,
            self.search_p95_ms,
            self.estimated_tokens / 1000,
            self.estimated_cost_usd,
        );
    }

    fn to_snapshot(&self, label: &str) -> BenchmarkSnapshot {
        use file_pipeline_core::domain::diagnostics::PerDocMetrics;
        BenchmarkSnapshot {
            version: BenchmarkSnapshot::CURRENT_VERSION,
            timestamp: chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
            label: label.to_string(),
            git_hash: BenchmarkSnapshot::git_short_hash(),
            doc_count: self.doc_count,
            throughput: ThroughputMetrics {
                total_secs: self.total_secs,
                process_secs: self.process_secs,
                batch_end_secs: self.batch_end_secs,
                flush_secs: self.flush_secs,
                docs_per_sec: self.docs_per_sec,
            },
            per_doc: Some(PerDocMetrics {
                avg_ms: self.per_doc_avg_ms,
                p50_ms: self.per_doc_p50_ms,
                p95_ms: self.per_doc_p95_ms,
                max_ms: self.per_doc_max_ms,
                variance_ratio: if self.per_doc_p50_ms > 0.0 { self.per_doc_p95_ms / self.per_doc_p50_ms } else { 0.0 },
            }),
            search: Some(SearchMetrics {
                avg_ms: self.search_avg_ms,
                p95_ms: self.search_p95_ms,
                queries: self.search_queries,
            }),
            crossref: CrossrefMetrics {
                relation_count: self.relation_count,
                unique_pairs: self.unique_pairs,
                double_count_ratio: self.double_ratio,
                isolated_docs: self.isolated_docs,
            },
            storage: Some(StorageMetrics {
                input_bytes: self.input_bytes,
                processed_bytes: self.processed_bytes,
                originals_bytes: self.originals_bytes,
                compression_pct: self.compression_pct,
            }),
            corpus: None,
        }
    }
}

async fn run_scale(doc_count: usize) -> ScaleResult {
    run_scale_with_threshold(doc_count, 0.7).await
}

async fn run_scale_with_threshold(doc_count: usize, threshold: f32) -> ScaleResult {
    let base = tempfile::TempDir::new().unwrap();
    let service = setup_service_with_threshold(base.path(), threshold);
    let inbox = base.path().join("inbox");

    // 문서 생성
    let mut files = Vec::new();
    let mut input_bytes = 0u64;
    for i in 0..doc_count {
        let (name, content) = generate_doc(i);
        input_bytes += content.len() as u64;
        let p = inbox.join(&name);
        std::fs::write(&p, &content).unwrap();
        files.push(p);
    }

    // 전체 처리 (배치 모드: persist + compile_state.save() 지연)
    service.vector_db.batch_begin();
    service.compile_state_batch_begin();
    let start = Instant::now();
    let mut per_doc_times: Vec<f64> = Vec::with_capacity(doc_count);
    for f in &files {
        let t = Instant::now();
        let _ = service.process_file(f).await;
        per_doc_times.push(t.elapsed().as_secs_f64());
    }
    let process_secs = start.elapsed().as_secs_f64();

    let t_end = Instant::now();
    service.vector_db.batch_end();
    service.compile_state_batch_end();
    let batch_end_secs = t_end.elapsed().as_secs_f64();

    // 비동기 교차참조 큐 flush (배치 완료 후 일괄 처리)
    let t_flush = Instant::now();
    let _ = service.flush_crossref();
    let flush_secs = t_flush.elapsed().as_secs_f64();
    let total_secs = process_secs + batch_end_secs + flush_secs;

    // per-doc 분포 계산
    per_doc_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let pd_avg = per_doc_times.iter().sum::<f64>() / per_doc_times.len().max(1) as f64;
    let pd_p50 = per_doc_times[per_doc_times.len() / 2];
    let pd_p95 = per_doc_times[(per_doc_times.len() as f64 * 0.95) as usize];
    let pd_max = *per_doc_times.last().unwrap_or(&0.0);

    // 스토리지 측정
    let processed_bytes = dir_size(&base.path().join("processed"));
    let originals_bytes = dir_size(&base.path().join("originals"));
    let storage_total = processed_bytes + originals_bytes;
    let compression_pct = if input_bytes > 0 {
        (1.0 - storage_total as f64 / input_bytes as f64) * 100.0
    } else { 0.0 };

    // 교차참조 수 + 유형별 분포 진단
    let all = service.vector_db.list_all().unwrap();
    let mut relation_count = 0;
    let mut type_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut per_doc_counts: Vec<usize> = Vec::new();
    let mut unique_pairs: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();
    for doc in &all {
        let rels = service.vector_db.find_related(&doc.id).unwrap();
        relation_count += rels.len();
        per_doc_counts.push(rels.len());
        for r in &rels {
            *type_counts.entry(format!("{}", r.relation_type)).or_default() += 1;
            let pair = if r.source_id < r.target_id {
                (r.source_id.clone(), r.target_id.clone())
            } else {
                (r.target_id.clone(), r.source_id.clone())
            };
            unique_pairs.insert(pair);
        }
    }
    let isolated = per_doc_counts.iter().filter(|&&c| c == 0).count();
    let double_ratio = if !unique_pairs.is_empty() {
        relation_count as f64 / unique_pairs.len() as f64
    } else { 0.0 };
    // 문서당 관계 수 히스토그램
    let h0_10 = per_doc_counts.iter().filter(|&&c| c < 10).count();
    let h10_20 = per_doc_counts.iter().filter(|&&c| (10..20).contains(&c)).count();
    let h20_30 = per_doc_counts.iter().filter(|&&c| (20..30).contains(&c)).count();
    let h30p = per_doc_counts.iter().filter(|&&c| c >= 30).count();

    if doc_count >= 100 {
        println!("\n  [진단] 관계 총수: {}, 고유 쌍: {}, double_ratio: {:.2}", relation_count, unique_pairs.len(), double_ratio);
        println!("  [진단] 유형별: {:?}", type_counts);
        println!("  [진단] 고립 문서: {}, 히스토그램: 0-10={} 10-20={} 20-30={} 30+={}", isolated, h0_10, h10_20, h20_30, h30p);
        if let Some(max_doc) = per_doc_counts.iter().max() {
            println!("  [진단] 허브 문서 최대 관계: {}", max_doc);
        }
    }

    // 검색 성능 측정 (100회 반복)
    let embedder = HashEmbedder::new(128);
    let queries = ["프로젝트 회의", "Rust 학습", "일지 이슈", "배포 일정", "테스트 자동화"];
    let mut search_times = Vec::new();
    for q in &queries {
        let emb = embedder.embed(q).await.unwrap();
        for _ in 0..20 {
            let s = Instant::now();
            let _ = service.vector_db.search_similar(&emb, 5);
            search_times.push(s.elapsed().as_secs_f64() * 1000.0);
        }
    }
    search_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let search_avg = search_times.iter().sum::<f64>() / search_times.len() as f64;
    let search_p95 = search_times[(search_times.len() as f64 * 0.95) as usize];

    // 토큰 추정
    let state = service.compile_state.lock().unwrap();
    let est_input_tokens = BenchmarkReport::estimate_tokens(state.stats.total_input_chars);
    let est_output_tokens = BenchmarkReport::estimate_tokens(state.stats.total_output_chars);
    let est_cost = BenchmarkReport::estimate_cost(est_input_tokens, est_output_tokens);

    ScaleResult {
        doc_count,
        total_secs,
        process_secs,
        batch_end_secs,
        flush_secs,
        docs_per_sec: doc_count as f64 / total_secs,
        input_bytes,
        processed_bytes,
        originals_bytes,
        compression_pct,
        relation_count,
        unique_pairs: unique_pairs.len(),
        double_ratio,
        isolated_docs: isolated,
        search_avg_ms: search_avg,
        search_p95_ms: search_p95,
        search_queries: search_times.len(),
        per_doc_avg_ms: pd_avg * 1000.0,
        per_doc_p50_ms: pd_p50 * 1000.0,
        per_doc_p95_ms: pd_p95 * 1000.0,
        per_doc_max_ms: pd_max * 1000.0,
        estimated_tokens: est_input_tokens + est_output_tokens,
        estimated_cost_usd: est_cost,
    }
}

fn dir_size(path: &Path) -> u64 {
    std::fs::read_dir(path)
        .unwrap_or_else(|_| std::fs::read_dir(".").unwrap()) // fallback
        .filter_map(|e| e.ok())
        .filter_map(|e| e.metadata().ok())
        .map(|m| m.len())
        .sum()
}

// ═══════════════════════════════════════════════════════════════
// 1단계: stub 대규모 벤치마크
// ═══════════════════════════════════════════════════════════════

/// 스냅샷 저장 경로 (프로젝트 루트/spec/benchmarks/)
fn snapshot_dir() -> std::path::PathBuf {
    // tests/ 기준으로 프로젝트 루트를 역추적
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    let project_root = Path::new(&manifest)
        .ancestors()
        .find(|p| p.join("spec").exists())
        .unwrap_or(Path::new("."))
        .to_path_buf();
    project_root.join("spec").join("benchmarks")
}

fn save_snapshot(result: &ScaleResult, label: &str) {
    let snapshot = result.to_snapshot(label);
    let dir = snapshot_dir();
    let filename = snapshot_filename(label);
    if let Err(e) = snapshot.save_to(&dir.join(&filename)) {
        eprintln!("  [스냅샷 저장 실패] {}: {}", filename, e);
    } else {
        println!("  [스냅샷] {}/{}", dir.display(), filename);
    }
}

#[tokio::test]
async fn bench_scale_100() {
    println!("\n=== Stub 대규모 벤치마크 ===");
    let r = run_scale(100).await;
    r.print();
    save_snapshot(&r, "scale_100");
    assert!(r.docs_per_sec > 5.0, "100문서: 최소 5 docs/s (debug 빌드)");
}

#[tokio::test]
async fn bench_scale_500() {
    let r = run_scale(500).await;
    r.print();
    save_snapshot(&r, "scale_500");
    assert!(r.docs_per_sec > 5.0, "500문서: 최소 5 docs/s");
}

#[tokio::test]
async fn bench_scale_1000() {
    let r = run_scale(1000).await;
    r.print();
    save_snapshot(&r, "scale_1000");
    assert!(r.docs_per_sec > 1.0, "1000문서: 최소 1 docs/s");
}

#[tokio::test]
async fn bench_scale_5000() {
    let r = run_scale(5000).await;
    r.print();
    save_snapshot(&r, "scale_5000");
    assert!(r.docs_per_sec > 0.5, "5000문서: 최소 0.5 docs/s");
}

// ═══════════════════════════════════════════════════════════════
// CI 회귀 감지 (100문서 기준, 이전 스냅샷 비교)
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn bench_regression_check() {
    println!("\n=== CI 회귀 감지 (100문서) ===");
    let r = run_scale(100).await;
    r.print();

    let current = r.to_snapshot("scale_100");
    let dir = snapshot_dir();

    // 이전 스냅샷 로드 시도
    match BenchmarkSnapshot::load_latest(&dir, "scale_100") {
        Ok(Some(baseline)) => {
            println!("  [baseline] {} ({})", baseline.timestamp, baseline.git_hash.as_deref().unwrap_or("?"));
            let result = file_pipeline_core::domain::diagnostics::check_regression(&baseline, &current);
            for check in &result.checks {
                println!("  {}", check);
            }
            if !result.passed {
                println!("  ⚠ 성능 회귀 감지! baseline과 비교하여 기준 미달 항목 있음");
            } else {
                println!("  ✓ 회귀 없음");
            }
            // CI에서는 assert로 변환 가능 — 현재는 경고만
        }
        Ok(None) => {
            println!("  [baseline 없음] 첫 실행 — 현재 결과를 baseline으로 저장");
        }
        Err(e) => {
            println!("  [baseline 로드 실패] {}", e);
        }
    }

    // 현재 결과를 스냅샷으로 저장 (다음 실행의 baseline)
    save_snapshot(&r, "scale_100");
}

// ═══════════════════════════════════════════════════════════════
// 검색 성능 곡선 (규모별)
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn bench_search_curve() {
    println!("\n=== 검색 성능 곡선 ===");
    for count in [100, 500, 1000] {
        let r = run_scale(count).await;
        println!(
            "  {:>5} 문서: 검색 {:.3}ms avg, {:.3}ms p95",
            count, r.search_avg_ms, r.search_p95_ms
        );
    }
}

// ═══════════════════════════════════════════════════════════════
// threshold 0.70 vs 0.80 비교 (100문서)
// ═══════════════════════════════════════════════════════════════

#[tokio::test]
async fn bench_threshold_comparison() {
    println!("\n=== threshold 비교 (100문서) ===");

    let r70 = run_scale_with_threshold(100, 0.70).await;
    let r80 = run_scale_with_threshold(100, 0.80).await;

    println!("  [0.70] 관계 {:>6} | {:.1} docs/s | flush 구간 포함 {:.2}s", r70.relation_count, r70.docs_per_sec, r70.total_secs);
    println!("  [0.80] 관계 {:>6} | {:.1} docs/s | flush 구간 포함 {:.2}s", r80.relation_count, r80.docs_per_sec, r80.total_secs);

    let reduction = if r70.relation_count > 0 {
        (1.0 - r80.relation_count as f64 / r70.relation_count as f64) * 100.0
    } else { 0.0 };

    println!("  [비교] 관계 감소: {:.0}% ({} → {})", reduction, r70.relation_count, r80.relation_count);
    println!("  [비교] 속도 변화: {:.1} → {:.1} docs/s", r70.docs_per_sec, r80.docs_per_sec);

    // threshold 0.80이면 관계가 줄어야 함
    assert!(r80.relation_count <= r70.relation_count,
        "threshold 상향 시 관계 수 감소 기대: 0.70={} vs 0.80={}", r70.relation_count, r80.relation_count);

    // 스냅샷 저장
    save_snapshot(&r70, "threshold_070");
    save_snapshot(&r80, "threshold_080");
}
