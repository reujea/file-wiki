//! 실제 문서 벤치마크 — K8s docs 493 + OpenStack docs 505 = ~1000문서
//!
//! 실제 기술 문서로 교차참조 성능/품질 측정

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use file_pipeline_core::domain::models::*;
use file_pipeline_core::ports::output::*;
use file_pipeline_shared::test_helpers::ServiceBuilder;

// ── 테스트 LLM (텍스트를 그대로 반환 + 키워드 추출) ──

struct RealCorpusLlm;
#[async_trait]
impl LLMPort for RealCorpusLlm {
    async fn classify_and_process(&self, path: &Path, _reg: &DocTypeRegistry) -> anyhow::Result<ClassifyAndProcessResult> {
        let content = std::fs::read_to_string(path).unwrap_or_default();
        let filename = path.file_name().unwrap_or_default().to_string_lossy();

        // 파일명에서 도메인 추출
        let doc_type = if filename.contains("k8s") { "k8s" }
            else if filename.contains("openstack") || filename.contains("os_") { "openstack" }
            else { "other" };

        // 키워드: 영문 단어 중 3글자 이상 상위 15개
        let mut word_freq: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for word in content.split(|c: char| !c.is_alphanumeric()).filter(|w| w.len() >= 3) {
            *word_freq.entry(word.to_lowercase()).or_default() += 1;
        }
        let mut sorted: Vec<_> = word_freq.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        let keywords: Vec<String> = sorted.into_iter().take(15).map(|(w, _)| w).collect();

        let metadata = Metadata {
            doc_types: vec![doc_type.into()],
            rationale: "real_corpus_bench".into(),
            date: "2026-04-17".into(),
            summary: format!("{} ({}자)", filename, content.len()),
            keywords,
            sensitive: false, doi: None, related_docs: vec![], source_doc_ids: vec![],
            search_hints: vec![], entities: vec![],
            ..Default::default()
        };

        let truncated = if content.len() > 10000 { &content[..10000] } else { &content };
        Ok(ClassifyAndProcessResult {
            doc_types: vec![doc_type.into()],
            rationale: "real_corpus_bench".into(),
            content: truncated.to_string(),
            metadata,
            sections: None,
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
    async fn classify_and_process_text(&self, file_name: &str, text: &str, _reg: &DocTypeRegistry) -> anyhow::Result<ClassifyAndProcessResult> {
        let doc_type = if file_name.contains("k8s") { "k8s" } else { "openstack" };
        let keywords: Vec<String> = text.split_whitespace().filter(|w| w.len() >= 3).take(15).map(|s| s.to_lowercase()).collect();
        let metadata = Metadata {
            doc_types: vec![doc_type.into()], rationale: "text".into(), date: "2026-04-17".into(),
            summary: format!("{} ({}자)", file_name, text.len()), keywords,
            sensitive: false, doi: None, related_docs: vec![], source_doc_ids: vec![],
            search_hints: vec![], entities: vec![],
            ..Default::default()
        };
        Ok(ClassifyAndProcessResult {
            doc_types: vec![doc_type.into()], rationale: "text".into(),
            content: text.chars().take(10000).collect(), metadata, sections: None,
        })
    }
}

// ── HashEmbedder (128dim) ──

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
        let norm: f32 = vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        if norm > 0.0 { vec.iter_mut().for_each(|x| *x /= norm); }
        Ok(vec)
    }
    async fn embed_batch(&self, texts: &[String]) -> anyhow::Result<Vec<Vec<f32>>> {
        let mut r = Vec::new();
        for t in texts { r.push(self.embed(t).await?); }
        Ok(r)
    }
}

#[tokio::test]
async fn bench_real_corpus_1000() {
    // BENCH_CORPUS_DIR 환경변수 우선, 없으면 workspace root tests/real_corpus
    let corpus_dir = std::env::var("BENCH_CORPUS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/real_corpus"));
    if !corpus_dir.exists() {
        println!("코퍼스 디렉토리 없음 ({}) → 스킵", corpus_dir.display());
        return;
    }

    // 처리 가능 확장자만 필터 (D:\file-test\samples 같은 혼합 코퍼스 대응)
    fn allow_ext(p: &Path) -> bool {
        match p.extension().and_then(|s| s.to_str()).map(|s| s.to_ascii_lowercase()) {
            Some(ext) => matches!(ext.as_str(),
                "txt" | "md" | "markdown" | "html" | "htm" |
                "docx" | "pptx" | "xlsx" | "pdf" |
                "json" | "yaml" | "yml" | "csv" | "log"),
            None => false,
        }
    }

    fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) {
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() { collect_files(&p, out); }
                else if p.is_file() && allow_ext(&p) { out.push(p); }
            }
        }
    }
    let mut files = Vec::new();
    collect_files(&corpus_dir, &mut files);

    if files.len() < 100 {
        println!("파일 {}개 → 부족, 스킵", files.len());
        return;
    }

    println!("\n=== 실제 문서 벤치마크 ===");
    println!("K8s + OpenStack: {} 파일", files.len());

    // 서비스 설정
    let base = tempfile::TempDir::new().unwrap();
    let registry = DocTypeRegistry::new(vec![
        DocTypeDef { id: "k8s".into(), label_ko: "쿠버네티스".into(), patterns: vec![],
            sections: vec![], prompt: String::new(), dedup_key: None, sensitive: false, thresholds: None },
        DocTypeDef { id: "openstack".into(), label_ko: "오픈스택".into(), patterns: vec![],
            sections: vec![], prompt: String::new(), dedup_key: None, sensitive: false, thresholds: None },
    ]);
    let service = ServiceBuilder::new(base.path())
        .with_llm(Arc::new(RealCorpusLlm))
        .with_embedding(Arc::new(HashEmbedder::new(128)))
        .with_registry(Arc::new(registry))
        .build();
    let inbox = service.inbox_dir.clone();

    // inbox에 복사 (같은 파일명 충돌 시 prefix 부여)
    let mut used: std::collections::HashSet<String> = std::collections::HashSet::new();
    for (i, f) in files.iter().enumerate() {
        let base_name = f.file_name().unwrap().to_string_lossy().to_string();
        let mut dest_name = base_name.clone();
        if used.contains(&dest_name) {
            dest_name = format!("{:04}_{}", i, base_name);
        }
        used.insert(dest_name.clone());
        let dest = inbox.join(&dest_name);
        let _ = std::fs::copy(f, &dest);
    }

    // 순차 가공 + 구간별 프로파일링
    let start = Instant::now();
    let inbox_files: Vec<PathBuf> = std::fs::read_dir(&inbox).unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .collect();
    let total_files = inbox_files.len();

    let mut ok_count = 0usize;
    let mut err_count = 0usize;
    let mut per_doc_times: Vec<f64> = Vec::new();

    // batch mode로 persist + compile_state.save() 지연
    service.vector_db.batch_begin();
    service.compile_state_batch_begin();

    // 파일 크기 vs 처리 시간 수집
    let mut size_time_pairs: Vec<(usize, f64)> = Vec::new();

    for (i, f) in inbox_files.iter().enumerate() {
        let file_size = std::fs::metadata(f).map(|m| m.len() as usize).unwrap_or(0);
        let t = Instant::now();
        match service.process_file(f).await {
            Ok(_) => {
                let elapsed = t.elapsed().as_secs_f64();
                per_doc_times.push(elapsed);
                size_time_pairs.push((file_size, elapsed));
                ok_count += 1;
            }
            Err(_) => err_count += 1,
        }
        if (i + 1) % 100 == 0 {
            let avg = per_doc_times.iter().sum::<f64>() / per_doc_times.len().max(1) as f64;
            println!("  진행: {}/{} (성공 {}, 실패 {}, avg {:.3}s/doc)", i + 1, total_files, ok_count, err_count, avg);
        }
    }

    // 파일 크기 vs 처리 시간 상관관계 출력
    println!("\n  [size_time] pairs={} ok={} err={}", size_time_pairs.len(), ok_count, err_count);
    size_time_pairs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    if !size_time_pairs.is_empty() {
        println!("  [느린 TOP 15]");
        for (size, time) in size_time_pairs.iter().take(15) {
            println!("    {}KB -> {:.3}s", size / 1024, time);
        }
        println!("  [빠른 BOTTOM 5]");
        for (size, time) in size_time_pairs.iter().rev().take(5) {
            println!("    {}KB -> {:.3}s", size / 1024, time);
        }
    }

    let t_process = start.elapsed();
    let service = Arc::new(service);
    service.vector_db.batch_end();
    service.compile_state_batch_end();

    // 교차참조 flush
    let t_flush_start = Instant::now();
    let _ = service.flush_crossref();
    let t_flush = t_flush_start.elapsed();
    let total_secs = start.elapsed().as_secs_f64();

    // 문서당 처리 시간 분포
    per_doc_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let p50 = per_doc_times.get(per_doc_times.len() / 2).copied().unwrap_or(0.0);
    let p95 = per_doc_times.get((per_doc_times.len() as f64 * 0.95) as usize).copied().unwrap_or(0.0);
    let p99 = per_doc_times.get((per_doc_times.len() as f64 * 0.99) as usize).copied().unwrap_or(0.0);
    let max_doc = per_doc_times.last().copied().unwrap_or(0.0);
    println!("\n  [pipeline] process={:.1}s flush={:.1}s total={:.1}s", t_process.as_secs_f64(), t_flush.as_secs_f64(), total_secs);
    println!("  [per-doc] avg={:.3}s p50={:.3}s p95={:.3}s p99={:.3}s max={:.3}s",
        per_doc_times.iter().sum::<f64>() / per_doc_times.len().max(1) as f64, p50, p95, p99, max_doc);

    // 진단
    use file_pipeline_core::domain::diagnostics;
    let stats = diagnostics::analyze_corpus(service.vector_db.as_ref()).unwrap();
    let issues = diagnostics::health_check(&stats);

    // 검색 성능
    let embedder = HashEmbedder::new(128);
    let queries = ["kubernetes pod deployment", "openstack nova compute", "container networking", "API authentication token", "volume storage cinder"];
    let mut search_times = Vec::new();
    for q in &queries {
        let emb = embedder.embed(q).await.unwrap();
        for _ in 0..20 {
            let s = Instant::now();
            let _ = service.vector_db.search_similar(&emb, 10);
            search_times.push(s.elapsed().as_secs_f64() * 1000.0);
        }
    }
    search_times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let search_avg = search_times.iter().sum::<f64>() / search_times.len() as f64;
    let search_p95 = search_times[(search_times.len() as f64 * 0.95) as usize];

    println!("\n=== 결과 ===");
    println!("  파일: {} (성공 {}, 실패 {})", total_files, ok_count, err_count);
    println!("  소요: {:.1}초 | {:.1} docs/s", total_secs, ok_count as f64 / total_secs);
    println!("  검색: avg {:.2}ms | p95 {:.2}ms", search_avg, search_p95);
    println!("{}", diagnostics::format_report(&stats, &issues));

    // 기본 검증
    assert!(ok_count >= 100, "최소 100문서 성공");
    println!("\n[PASS] 실제 문서 벤치마크 완료");
}

/// 실 코퍼스 5변형 비교 — 트리거 #2(MinHash) / #4(메타블로킹) 효과 측정
/// BENCH_CORPUS_DIR 환경변수 필수
#[tokio::test]
async fn bench_real_corpus_variants() {
    let corpus_dir = match std::env::var("BENCH_CORPUS_DIR") {
        Ok(d) => PathBuf::from(d),
        Err(_) => { println!("BENCH_CORPUS_DIR 미설정 → 스킵"); return; }
    };
    if !corpus_dir.exists() { println!("코퍼스 없음 → 스킵"); return; }

    fn allow_ext(p: &Path) -> bool {
        match p.extension().and_then(|s| s.to_str()).map(|s| s.to_ascii_lowercase()) {
            Some(ext) => matches!(ext.as_str(),
                "txt" | "md" | "markdown" | "html" | "htm" |
                "docx" | "pptx" | "xlsx" | "pdf" |
                "json" | "yaml" | "yml" | "csv" | "log"),
            None => false,
        }
    }
    fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) {
        if let Ok(rd) = std::fs::read_dir(dir) {
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() { collect_files(&p, out); }
                else if p.is_file() && allow_ext(&p) { out.push(p); }
            }
        }
    }
    let mut all_files = Vec::new();
    collect_files(&corpus_dir, &mut all_files);
    if all_files.len() < 50 { println!("파일 부족 {}건 → 스킵", all_files.len()); return; }
    println!("\n=== 실 코퍼스 5변형 비교 ({} 파일) ===", all_files.len());

    async fn run(label: &str, files: &[PathBuf], threshold: f32, mh_force: bool, meta_block: bool)
        -> (f64, usize, usize)
    {
        let base = tempfile::TempDir::new().unwrap();
        let registry = DocTypeRegistry::new(vec![
            DocTypeDef { id: "k8s".into(), label_ko: "쿠버네티스".into(), patterns: vec![], sections: vec![], prompt: String::new(), dedup_key: None, sensitive: false, thresholds: None },
            DocTypeDef { id: "openstack".into(), label_ko: "오픈스택".into(), patterns: vec![], sections: vec![], prompt: String::new(), dedup_key: None, sensitive: false, thresholds: None },
            DocTypeDef { id: "other".into(), label_ko: "기타".into(), patterns: vec![], sections: vec![], prompt: String::new(), dedup_key: None, sensitive: false, thresholds: None },
        ]);
        let svc = ServiceBuilder::new(base.path())
            .with_llm(Arc::new(RealCorpusLlm))
            .with_embedding(Arc::new(HashEmbedder::new(128)))
            .with_registry(Arc::new(registry))
            .with_crossref_threshold(threshold)
            .with_minhash(mh_force, 3_000)
            .with_metadata_blocking(meta_block)
            .build();
        let inbox = svc.inbox_dir.clone();
        let mut used: std::collections::HashSet<String> = std::collections::HashSet::new();
        for (i, f) in files.iter().enumerate() {
            let bn = f.file_name().unwrap().to_string_lossy().to_string();
            let mut dn = bn.clone();
            if used.contains(&dn) { dn = format!("{:04}_{}", i, bn); }
            used.insert(dn.clone());
            let _ = std::fs::copy(f, inbox.join(&dn));
        }

        let inbox_files: Vec<PathBuf> = std::fs::read_dir(&inbox).unwrap()
            .filter_map(|e| e.ok().map(|e| e.path())).collect();

        let start = Instant::now();
        svc.vector_db.batch_begin();
        svc.compile_state_batch_begin();
        let mut ok = 0usize;
        for f in &inbox_files {
            if svc.process_file(f).await.is_ok() { ok += 1; }
        }
        svc.vector_db.batch_end();
        svc.compile_state_batch_end();
        let _ = svc.flush_crossref();
        let secs = start.elapsed().as_secs_f64();

        let rels: usize = svc.vector_db.list_all().unwrap().iter()
            .map(|d| svc.vector_db.find_related(&d.id).unwrap().len()).sum();
        println!("  [{:<28}] {:.2}s | ok={} | 관계 {}", label, secs, ok, rels);
        (secs, ok, rels)
    }

    let res = [("baseline (0.7, mh=off, block=off)", run("baseline", &all_files, 0.7, false, false).await),
        ("threshold 0.8", run("threshold 0.8", &all_files, 0.8, false, false).await),
        ("minhash force", run("minhash force", &all_files, 0.7, true, false).await),
        ("metadata blocking", run("metadata blocking", &all_files, 0.7, false, true).await),
        ("all (0.8 + mh + block)", run("all", &all_files, 0.8, true, true).await)];

    let (_, (base_secs, _, base_rels)) = (res[0].0, res[0].1);
    println!("\n  변형 vs baseline:");
    for (label, (s, _, r)) in res.iter().skip(1) {
        let dt = (s - base_secs) / base_secs * 100.0;
        let dr = if base_rels > 0 {
            (*r as f64 - base_rels as f64) / base_rels as f64 * 100.0
        } else { 0.0 };
        println!("  {:<32} 시간 {:+.1}% | 관계 {:+.1}%", label, dt, dr);
    }
    for (_, (_, _, r)) in res.iter().skip(1) {
        assert!(*r <= base_rels, "변형은 baseline보다 많은 관계를 만들 수 없음");
    }
}

// step-o2 partial 해소 (2026-06-17): integration test mock OutboundManifest 박힘
impl file_pipeline_core::ports::outbound::OutboundManifest for RealCorpusLlm {
    fn id(&self) -> &str { "fp-outbound-llm-real-corpus" }
    fn category(&self) -> file_pipeline_core::ports::outbound::OutboundCategory {
        file_pipeline_core::ports::outbound::OutboundCategory::Llm
    }
    fn capabilities(&self) -> file_pipeline_core::ports::output::ResourceCapabilities {
        file_pipeline_core::ports::output::ResourceCapabilities::standard("real-corpus")
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
