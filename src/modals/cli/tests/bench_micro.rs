//! 마이크로 벤치마크 — 100문서 빠른 피드백 루프 (~8초)
//!
//! 코드 변경 후 즉시 실행하여 성능 회귀 확인용.
//! cargo test bench_micro -- --nocapture

use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use file_pipeline_core::domain::diagnostics::{
    BenchmarkSnapshot, CrossrefMetrics, PerDocMetrics, ThroughputMetrics,
    snapshot_filename,
};
use file_pipeline_core::domain::models::*;
use file_pipeline_core::ports::output::*;
use file_pipeline_core::service::FileProcessingService;
use file_pipeline_shared::test_helpers::ServiceBuilder;

// ── 어댑터 ──

struct HashEmbedder { dim: usize }
impl HashEmbedder {
    fn new(dim: usize) -> Self { Self { dim } }
}
#[async_trait]
impl EmbeddingPort for HashEmbedder {
    fn dim(&self) -> usize { self.dim }
    async fn embed(&self, text: &str) -> anyhow::Result<Vec<f32>> {
        let mut vec = vec![0.0f32; self.dim];
        for word in text.split_whitespace() {
            let h = word.bytes().fold(0u64, |a, b| a.wrapping_mul(31).wrapping_add(b as u64));
            vec[(h as usize) % self.dim] += 1.0;
        }
        let n: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        if n > 0.0 { vec.iter_mut().for_each(|x| *x /= n); }
        Ok(vec)
    }
    async fn embed_batch(&self, texts: &[String]) -> anyhow::Result<Vec<Vec<f32>>> {
        let mut r = Vec::new();
        for t in texts { r.push(self.embed(t).await?); }
        Ok(r)
    }
}

/// LLM 지연 시뮬레이션 (실 LLM 병렬화 효과 검증용)
struct SlowLlm {
    delay: std::time::Duration,
    inner: StubLlm,
}
#[async_trait]
impl LLMPort for SlowLlm {
    async fn classify_and_process(&self, file_path: &Path, reg: &DocTypeRegistry) -> anyhow::Result<ClassifyAndProcessResult> {
        tokio::time::sleep(self.delay).await;
        self.inner.classify_and_process(file_path, reg).await
    }
    async fn summarize_text(&self, new: &str, existing: &str) -> anyhow::Result<String> { self.inner.summarize_text(new, existing).await }
    async fn enrich_existing(&self, existing: &str, a: &str, b: &[String]) -> anyhow::Result<EnrichResult> {
        self.inner.enrich_existing(existing, a, b).await
    }
}

struct StubLlm;
#[async_trait]
impl LLMPort for StubLlm {
    async fn classify_and_process(&self, file_path: &Path, _reg: &DocTypeRegistry) -> anyhow::Result<ClassifyAndProcessResult> {
        let content = std::fs::read_to_string(file_path)?;
        let filename = file_path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
        let doc_type = if filename.contains("회의") { "meeting" }
            else if filename.contains("학습") { "study" }
            else { "log" };
        let keywords: Vec<String> = content.split_whitespace().filter(|w| w.chars().count() >= 2).take(12).map(String::from).collect();
        let metadata = Metadata {
            doc_types: vec![doc_type.into()], rationale: "micro".into(),
            date: "2026-04-20".into(), summary: format!("micro: {}", filename),
            keywords, sensitive: false, doi: None, related_docs: vec![],
            source_doc_ids: vec![], search_hints: vec![], entities: vec![],
            ..Default::default()
        };
        Ok(ClassifyAndProcessResult {
            doc_types: vec![doc_type.into()], rationale: "micro".into(),
            content: content.chars().take(5000).collect(), metadata, sections: None,
        })
    }
    async fn summarize_text(&self, new: &str, existing: &str) -> anyhow::Result<String> { Ok(format!("{}\n{}", existing, new)) }
    async fn enrich_existing(&self, existing: &str, _: &str, _: &[String]) -> anyhow::Result<EnrichResult> {
        Ok(EnrichResult { updated_content: existing.into(), change_summary: String::new(), should_update: false })
    }
}

fn generate_doc(idx: usize) -> (String, String) {
    let doc_type = match idx % 3 { 0 => "회의록", 1 => "학습", _ => "일지" };
    let name = format!("{}_{:04}.txt", doc_type, idx);
    let content = format!(
        "{} 문서 #{} id={}\n2026년 4월 {}일\n\
         참석자: 김철수, 이영희, 박지민\n\
         주제: 프로젝트 #{} 진행률 {}% 보고\n\
         DOC-{:04} 벤치마크 {} ops/s 달성\n\
         결정: 4월 {}일 완료 목표\n",
        doc_type, idx, idx * 7919 + 1234567,
        (idx % 28) + 1, idx, (idx * 7) % 100,
        idx, idx * 13 + 42, (idx % 25) + 3,
    );
    (name, content)
}

fn setup_service_with_llm(base: &Path, llm: Arc<dyn LLMPort>) -> FileProcessingService {
    let registry = DocTypeRegistry::new(vec![
        DocTypeDef { id: "meeting".into(), label_ko: "회의록".into(), patterns: vec![],
            sections: vec![], prompt: String::new(), dedup_key: None, sensitive: false, thresholds: None },
        DocTypeDef { id: "study".into(), label_ko: "학습".into(), patterns: vec![],
            sections: vec![], prompt: String::new(), dedup_key: None, sensitive: false, thresholds: None },
        DocTypeDef { id: "log".into(), label_ko: "일지".into(), patterns: vec![],
            sections: vec![], prompt: String::new(), dedup_key: None, sensitive: false, thresholds: None },
    ]);

    ServiceBuilder::new(base)
        .with_llm(llm)
        .with_embedding(Arc::new(HashEmbedder::new(128)))
        .with_registry(Arc::new(registry))
        .with_semantic_dup_threshold(0.03)
        .with_fragment_threshold(0)
        .build()
}

fn setup_service(base: &Path) -> FileProcessingService {
    setup_service_with_llm(base, Arc::new(StubLlm))
}

/// 100문서 마이크로 벤치 — 순차, compile_state 배치화 효과 비교
#[tokio::test]
async fn bench_micro_100() {
    let doc_count = 100;
    println!("\n=== 마이크로 벤치마크 (100문서) ===");

    // ── A: 배치 모드 OFF (기존 방식) ──
    let base_a = tempfile::TempDir::new().expect("tempdir failed");
    let svc_a = setup_service(base_a.path());
    let inbox_a = base_a.path().join("inbox");
    for i in 0..doc_count {
        let (name, content) = generate_doc(i);
        std::fs::write(inbox_a.join(&name), &content).expect("write failed");
    }
    let files_a: Vec<_> = std::fs::read_dir(&inbox_a).unwrap()
        .filter_map(|e| e.ok().map(|e| e.path())).collect();

    let start_a = Instant::now();
    for f in &files_a {
        let _ = svc_a.process_file(f).await;
    }
    let _ = svc_a.flush_crossref();
    let secs_a = start_a.elapsed().as_secs_f64();

    // ── B: 배치 모드 ON (vector_db + compile_state) ──
    let base_b = tempfile::TempDir::new().expect("tempdir failed");
    let svc_b = setup_service(base_b.path());
    let inbox_b = base_b.path().join("inbox");
    for i in 0..doc_count {
        let (name, content) = generate_doc(i);
        std::fs::write(inbox_b.join(&name), &content).expect("write failed");
    }
    let files_b: Vec<_> = std::fs::read_dir(&inbox_b).unwrap()
        .filter_map(|e| e.ok().map(|e| e.path())).collect();

    let start_b = Instant::now();
    svc_b.vector_db.batch_begin();
    svc_b.compile_state_batch_begin();
    for f in &files_b {
        let _ = svc_b.process_file(f).await;
    }
    svc_b.vector_db.batch_end();
    svc_b.compile_state_batch_end();
    let _ = svc_b.flush_crossref();
    let secs_b = start_b.elapsed().as_secs_f64();

    // ── 결과 비교 ──
    let speedup = secs_a / secs_b;
    let rels_a = svc_a.vector_db.list_all().unwrap().iter()
        .map(|d| svc_a.vector_db.find_related(&d.id).unwrap().len()).sum::<usize>();
    let rels_b = svc_b.vector_db.list_all().unwrap().iter()
        .map(|d| svc_b.vector_db.find_related(&d.id).unwrap().len()).sum::<usize>();

    println!("  [A: 배치OFF] {:.2}s | {:.1} docs/s | 관계 {}", secs_a, doc_count as f64 / secs_a, rels_a);
    println!("  [B: 배치ON ] {:.2}s | {:.1} docs/s | 관계 {}", secs_b, doc_count as f64 / secs_b, rels_b);
    println!("  [개선] {:.2}x ({:.1}초 절감)", speedup, secs_a - secs_b);
    println!("  [관계 차이] {} (0이면 정상)", (rels_a as i64 - rels_b as i64).abs());

    // 기본 검증
    assert!(secs_b < 30.0, "100문서 배치 모드: 30초 이내");
    assert!(speedup > 1.0, "배치 모드가 비배치보다 빨라야 함");
}

/// 100문서 병렬 벤치 — 3회 반복 중앙값으로 비교 (캐시 편향 제거)
#[tokio::test]
async fn bench_micro_parallel() {
    let doc_count = 100;
    let repeats = 3;
    println!("\n=== 병렬 벤치마크 (100문서, {}회 반복 중앙값) ===", repeats);

    for workers in [1, 2, 4] {
        let mut totals = Vec::new();
        let mut rel_count = 0usize;

        for _round in 0..repeats {
            let base = tempfile::TempDir::new().expect("tempdir failed");
            let service = Arc::new(setup_service(base.path()));
            let inbox = base.path().join("inbox");

            for i in 0..doc_count {
                let (name, content) = generate_doc(i);
                std::fs::write(inbox.join(&name), &content).expect("write failed");
            }
            let files: Vec<_> = std::fs::read_dir(&inbox).unwrap()
                .filter_map(|e| e.ok().map(|e| e.path())).collect();

            service.vector_db.batch_begin();
            service.compile_state_batch_begin();

            let semaphore = Arc::new(tokio::sync::Semaphore::new(workers));
            let start = Instant::now();
            let mut handles = Vec::new();

            for f in files {
                let svc = Arc::clone(&service);
                let sem = Arc::clone(&semaphore);
                handles.push(tokio::spawn(async move {
                    let _permit = sem.acquire().await.expect("semaphore");
                    let _ = svc.process_file(&f).await;
                }));
            }
            for h in handles {
                let _ = h.await;
            }

            service.vector_db.batch_end();
            service.compile_state_batch_end();
            let _ = service.flush_crossref();
            let total = start.elapsed().as_secs_f64();
            totals.push(total);

            rel_count = service.vector_db.list_all().unwrap().iter()
                .map(|d| service.vector_db.find_related(&d.id).unwrap().len()).sum::<usize>();
        }

        totals.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let median = totals[repeats / 2];
        let min = totals[0];
        let max = totals[repeats - 1];

        println!(
            "  [workers={}] median={:.2}s (min={:.2} max={:.2}) | {:.1} docs/s | 관계 {}",
            workers, median, min, max, doc_count as f64 / median, rel_count
        );
    }
}

/// 스냅샷 저장 경로 (프로젝트 루트/spec/benchmarks/)
fn snapshot_dir() -> std::path::PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    let project_root = std::path::Path::new(&manifest)
        .ancestors()
        .find(|p| p.join("spec").exists())
        .unwrap_or(std::path::Path::new("."))
        .to_path_buf();
    project_root.join("spec").join("benchmarks")
}

/// per-doc 프로파일링 — 병목 구간 식별
#[tokio::test]
async fn bench_micro_profile() {
    let doc_count = 50;
    println!("\n=== per-doc 프로파일링 (50문서) ===");

    let base = tempfile::TempDir::new().expect("tempdir failed");
    let service = setup_service(base.path());
    let inbox = base.path().join("inbox");

    for i in 0..doc_count {
        let (name, content) = generate_doc(i);
        std::fs::write(inbox.join(&name), &content).expect("write failed");
    }

    let files: Vec<_> = std::fs::read_dir(&inbox).unwrap()
        .filter_map(|e| e.ok().map(|e| e.path())).collect();

    service.vector_db.batch_begin();
    service.compile_state_batch_begin();

    let mut times = Vec::new();
    let start = Instant::now();
    for f in &files {
        let t = Instant::now();
        let _ = service.process_file(f).await;
        times.push(t.elapsed().as_secs_f64());
    }
    let process_secs = start.elapsed().as_secs_f64();

    let t_end = Instant::now();
    service.vector_db.batch_end();
    service.compile_state_batch_end();
    let end_secs = t_end.elapsed().as_secs_f64();

    let t_flush = Instant::now();
    let _ = service.flush_crossref();
    let flush_secs = t_flush.elapsed().as_secs_f64();

    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let avg = times.iter().sum::<f64>() / times.len() as f64;
    let p50 = times[times.len() / 2];
    let p95 = times[(times.len() as f64 * 0.95) as usize];
    let max = *times.last().unwrap();
    let total_secs = process_secs + end_secs + flush_secs;

    println!("  [process] {:.2}s ({:.1} docs/s)", process_secs, doc_count as f64 / process_secs);
    println!("  [batch_end] {:.3}s", end_secs);
    println!("  [flush] {:.2}s", flush_secs);
    println!("  [total] {:.2}s", total_secs);
    println!("  [per-doc] avg={:.4}s p50={:.4}s p95={:.4}s max={:.4}s", avg, p50, p95, max);

    // 스냅샷 저장
    let snapshot = BenchmarkSnapshot {
        version: BenchmarkSnapshot::CURRENT_VERSION,
        timestamp: chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
        label: "micro_profile_50".to_string(),
        git_hash: BenchmarkSnapshot::git_short_hash(),
        doc_count,
        throughput: ThroughputMetrics {
            total_secs,
            process_secs,
            batch_end_secs: end_secs,
            flush_secs,
            docs_per_sec: doc_count as f64 / total_secs,
        },
        per_doc: Some(PerDocMetrics {
            avg_ms: avg * 1000.0,
            p50_ms: p50 * 1000.0,
            p95_ms: p95 * 1000.0,
            max_ms: max * 1000.0,
            variance_ratio: if p50 > 0.0 { p95 / p50 } else { 0.0 },
        }),
        search: None,
        crossref: CrossrefMetrics {
            relation_count: 0,
            unique_pairs: 0,
            double_count_ratio: 0.0,
            isolated_docs: 0,
        },
        storage: None,
        corpus: None,
    };
    let dir = snapshot_dir();
    let filename = snapshot_filename("micro_profile_50");
    if let Err(e) = snapshot.save_to(&dir.join(&filename)) {
        eprintln!("  [스냅샷 저장 실패] {}", e);
    } else {
        println!("  [스냅샷] {}/{}", dir.display(), filename);
    }
}

/// SlowLlm 병렬 벤치 — sleep 500ms로 실 LLM 지연 시뮬레이션 (20문서)
#[tokio::test]
async fn bench_micro_slow_llm() {
    let doc_count = 20;
    let delay = std::time::Duration::from_millis(500);
    println!("\n=== SlowLlm 병렬 벤치 ({}문서, delay={}ms) ===", doc_count, delay.as_millis());

    for workers in [1, 2, 4] {
        let base = tempfile::TempDir::new().expect("tempdir failed");
        let slow_llm = Arc::new(SlowLlm { delay, inner: StubLlm });
        let service = Arc::new(setup_service_with_llm(base.path(), slow_llm));
        let inbox = base.path().join("inbox");

        for i in 0..doc_count {
            let (name, content) = generate_doc(i);
            std::fs::write(inbox.join(&name), &content).expect("write failed");
        }
        let files: Vec<_> = std::fs::read_dir(&inbox).unwrap()
            .filter_map(|e| e.ok().map(|e| e.path())).collect();

        service.vector_db.batch_begin();
        service.compile_state_batch_begin();

        let semaphore = Arc::new(tokio::sync::Semaphore::new(workers));
        let start = Instant::now();
        let mut handles = Vec::new();

        for f in files {
            let svc = Arc::clone(&service);
            let sem = Arc::clone(&semaphore);
            handles.push(tokio::spawn(async move {
                let _permit = sem.acquire().await.expect("semaphore");
                let _ = svc.process_file(&f).await;
            }));
        }
        for h in handles {
            let _ = h.await;
        }
        let process_secs = start.elapsed().as_secs_f64();

        service.vector_db.batch_end();
        service.compile_state_batch_end();

        let t_flush = Instant::now();
        let _ = service.flush_crossref();
        let flush_secs = t_flush.elapsed().as_secs_f64();
        let total = process_secs + flush_secs;

        // 이론적 최적: doc_count * delay / workers
        let theory = doc_count as f64 * delay.as_secs_f64() / workers as f64;

        println!(
            "  [workers={}] process={:.2}s flush={:.2}s total={:.2}s | {:.1} docs/s | 이론={:.1}s 효율={:.0}%",
            workers, process_secs, flush_secs, total,
            doc_count as f64 / total, theory,
            theory / process_secs * 100.0,
        );
    }
}

/// 2000문서 스케일 벤치 — 행렬곱 flush O(N²) 스케일 검증
#[tokio::test]
async fn bench_scale_2000() {
    let doc_count = 2000;
    println!("\n=== 2000문서 스케일 벤치 (행렬곱 flush) ===");

    let base = tempfile::TempDir::new().expect("tempdir failed");
    let service = setup_service(base.path());
    let inbox = base.path().join("inbox");

    for i in 0..doc_count {
        let (name, content) = generate_doc(i);
        std::fs::write(inbox.join(&name), &content).expect("write failed");
    }
    let files: Vec<_> = std::fs::read_dir(&inbox).unwrap()
        .filter_map(|e| e.ok().map(|e| e.path())).collect();
    let total_files = files.len();

    service.vector_db.batch_begin();
    service.compile_state_batch_begin();

    let start = Instant::now();
    let mut per_doc_times: Vec<f64> = Vec::new();
    for (i, f) in files.iter().enumerate() {
        let t = Instant::now();
        let _ = service.process_file(f).await;
        per_doc_times.push(t.elapsed().as_secs_f64());
        if (i + 1) % 500 == 0 {
            let avg = per_doc_times.iter().sum::<f64>() / per_doc_times.len() as f64;
            println!("  진행: {}/{} avg={:.3}s/doc", i + 1, total_files, avg);
        }
    }
    let process_secs = start.elapsed().as_secs_f64();

    let t_end = Instant::now();
    service.vector_db.batch_end();
    service.compile_state_batch_end();
    let end_secs = t_end.elapsed().as_secs_f64();

    let t_flush = Instant::now();
    let _ = service.flush_crossref();
    let flush_secs = t_flush.elapsed().as_secs_f64();

    let total_secs = process_secs + end_secs + flush_secs;

    // per-doc 분포
    per_doc_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let avg = per_doc_times.iter().sum::<f64>() / per_doc_times.len() as f64;
    let p50 = per_doc_times[per_doc_times.len() / 2];
    let p95 = per_doc_times[(per_doc_times.len() as f64 * 0.95) as usize];
    let max = *per_doc_times.last().unwrap();

    // 관계 수
    let all = service.vector_db.list_all().unwrap();
    let rel_count: usize = all.iter()
        .map(|d| service.vector_db.find_related(&d.id).unwrap().len())
        .sum();

    println!("\n  [결과]");
    println!("  문서: {}", total_files);
    println!("  process: {:.1}s ({:.1} docs/s)", process_secs, doc_count as f64 / process_secs);
    println!("  batch_end: {:.3}s", end_secs);
    println!("  flush: {:.1}s", flush_secs);
    println!("  total: {:.1}s ({:.1} docs/s)", total_secs, doc_count as f64 / total_secs);
    println!("  per-doc: avg={:.3}s p50={:.3}s p95={:.3}s max={:.3}s", avg, p50, p95, max);
    println!("  관계: {}", rel_count);

    // 스냅샷 저장
    let snapshot = BenchmarkSnapshot {
        version: BenchmarkSnapshot::CURRENT_VERSION,
        timestamp: chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
        label: "scale_2000".to_string(),
        git_hash: BenchmarkSnapshot::git_short_hash(),
        doc_count,
        throughput: ThroughputMetrics {
            total_secs,
            process_secs,
            batch_end_secs: end_secs,
            flush_secs,
            docs_per_sec: doc_count as f64 / total_secs,
        },
        per_doc: Some(PerDocMetrics {
            avg_ms: avg * 1000.0,
            p50_ms: p50 * 1000.0,
            p95_ms: p95 * 1000.0,
            max_ms: max * 1000.0,
            variance_ratio: if p50 > 0.0 { p95 / p50 } else { 0.0 },
        }),
        search: None,
        crossref: CrossrefMetrics {
            relation_count: rel_count,
            unique_pairs: 0,
            double_count_ratio: 0.0,
            isolated_docs: 0,
        },
        storage: None,
        corpus: None,
    };
    let dir = snapshot_dir();
    let filename = snapshot_filename("scale_2000");
    if let Err(e) = snapshot.save_to(&dir.join(&filename)) {
        eprintln!("  [스냅샷 저장 실패] {}", e);
    } else {
        println!("  [스냅샷] {}/{}", dir.display(), filename);
    }

    assert!(total_secs < 600.0, "2000문서: 10분 이내");
}

/// threshold/minhash/blocking 4축 비교 — 100문서 동일 코퍼스에서 관계 수 + 처리 시간 비교
/// 트리거 대기 항목(#1, #2, #4)의 ROI를 한눈에 확인
#[tokio::test]
async fn bench_crossref_variants() {
    let doc_count = 100;
    println!("\n=== 교차참조 변형 비교 (100문서, HashEmbedder) ===");

    async fn run(label: &str, threshold: f32, mh_force: bool, meta_block: bool, doc_count: usize) -> (f64, usize) {
        let base = tempfile::TempDir::new().expect("tempdir failed");
        let mut svc = setup_service(base.path());
        svc.crossref_similarity_threshold = threshold;
        svc.crossref_minhash_force = mh_force;
        svc.crossref_metadata_blocking = meta_block;
        let inbox = base.path().join("inbox");
        for i in 0..doc_count {
            let (name, content) = generate_doc(i);
            std::fs::write(inbox.join(&name), &content).expect("write failed");
        }
        let files: Vec<_> = std::fs::read_dir(&inbox).unwrap()
            .filter_map(|e| e.ok().map(|e| e.path())).collect();

        let start = Instant::now();
        svc.vector_db.batch_begin();
        svc.compile_state_batch_begin();
        for f in &files {
            let _ = svc.process_file(f).await;
        }
        svc.vector_db.batch_end();
        svc.compile_state_batch_end();
        let _ = svc.flush_crossref();
        let secs = start.elapsed().as_secs_f64();

        let rels = svc.vector_db.list_all().unwrap().iter()
            .map(|d| svc.vector_db.find_related(&d.id).unwrap().len()).sum::<usize>();
        println!("  [{:<28}] {:.2}s | 관계 {}", label, secs, rels);
        (secs, rels)
    }

    let mut res: Vec<(&str, (f64, usize))> = Vec::new();
    res.push(("baseline (0.7, mh=off, block=off)", run("baseline", 0.7, false, false, doc_count).await));
    res.push(("threshold 0.8", run("threshold 0.8", 0.8, false, false, doc_count).await));
    res.push(("minhash force", run("minhash force", 0.7, true, false, doc_count).await));
    res.push(("metadata blocking", run("metadata blocking", 0.7, false, true, doc_count).await));
    res.push(("all (0.8 + mh + block)", run("all", 0.8, true, true, doc_count).await));

    let (_, (base_secs, base_rels)) = res[0];
    println!("\n  변형 vs baseline:");
    for (label, (s, r)) in res.iter().skip(1) {
        let dt = (s - base_secs) / base_secs * 100.0;
        let dr = if base_rels > 0 {
            (*r as f64 - base_rels as f64) / base_rels as f64 * 100.0
        } else { 0.0 };
        println!("  {:<28} 시간 {:+.1}% | 관계 {:+.1}%", label, dt, dr);
    }

    // 회귀 가드: 변형은 baseline보다 많은 관계를 만들 수 없음
    for (_, (_, r)) in res.iter().skip(1) {
        assert!(*r <= base_rels, "변형은 baseline보다 많은 관계를 만들 수 없음");
    }
}
