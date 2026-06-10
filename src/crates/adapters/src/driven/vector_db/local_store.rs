use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use file_pipeline_core::domain::models::{
    DbStats, DocRelation, Document, RelationType, SimilarDoc, StoredDocSummary,
};
use file_pipeline_core::ports::output::VectorDBPort;

// ── Blue-Green Search Slot ──────────────────────────────────

/// 검색에 사용되는 읽기 전용 스냅샷. 구축 후 불변, atomic swap으로 교체.
struct SearchSlot {
    mmap: Option<memmap2::Mmap>,
    hnsw: Option<HnswCache>,
    #[allow(dead_code)]
    doc_count: usize,
    #[allow(dead_code)]
    dim: usize,
}

impl SearchSlot {
    fn empty() -> Self {
        Self { mmap: None, hnsw: None, doc_count: 0, dim: 0 }
    }

    fn load_embeddings(&self, docs: &[StoredDoc]) -> Vec<Vec<f32>> {
        let mmap = match self.mmap.as_ref() {
            Some(m) => m,
            None => return docs.iter().map(|_| vec![]).collect(),
        };
        docs.iter().map(|d| {
            let byte_len = d.vec_dim * 4;
            if d.vec_dim == 0 || d.vec_offset + byte_len > mmap.len() {
                return vec![];
            }
            let slice = &mmap[d.vec_offset..d.vec_offset + byte_len];
            bytemuck::cast_slice::<u8, f32>(slice).to_vec()
        }).collect()
    }
}

/// Refresh 상태 추적
struct RefreshState {
    docs_since_last_refresh: usize,
    last_refresh_time: std::time::Instant,
}

/// Refresh 임계치 설정
struct RefreshConfig {
    /// 이 수를 초과하면 슬롯 교체 트리거
    doc_count_threshold: usize,
    /// 마지막 refresh 이후 이 시간 초과 시 트리거 (1건 이상)
    time_threshold: std::time::Duration,
}

impl Default for RefreshConfig {
    fn default() -> Self {
        Self {
            doc_count_threshold: 50,
            time_threshold: std::time::Duration::from_secs(300),
        }
    }
}

impl RefreshState {
    fn should_refresh(&self, config: &RefreshConfig) -> bool {
        if self.docs_since_last_refresh == 0 { return false; }
        self.docs_since_last_refresh >= config.doc_count_threshold
            || self.last_refresh_time.elapsed() >= config.time_threshold
    }
}

// ── 증분 flush 설정 ──────────────────────────────────────────

/// 증분 flush: 교차참조 + DB refresh 분리 설정
pub struct IncrementalFlushConfig {
    /// 시간 임계치 (기본: 300초)
    pub time_threshold: std::time::Duration,
    /// flushed_embeddings 강제 refresh 임계치 (기본: 10,000)
    pub max_flushed: usize,
}

impl Default for IncrementalFlushConfig {
    fn default() -> Self {
        Self {
            time_threshold: std::time::Duration::from_secs(300),
            max_flushed: 10_000,
        }
    }
}

impl IncrementalFlushConfig {
    /// 총 문서 수 기반 동적 임계치
    pub fn effective_threshold(&self, total_docs: usize) -> usize {
        match total_docs {
            0..=500 => 50,
            501..=5_000 => 200,
            5_001..=20_000 => 500,
            _ => 1_000,
        }
    }
}

/// 증분 flush 런타임 상태 — Atomic 카운터 + Mutex(flush 시에만)
struct IncrementalFlushState {
    /// flush 완료 but refresh 전 임베딩 (flush 시에만 lock)
    flushed_embeddings: std::sync::Mutex<Vec<(String, Vec<f32>)>>,
    /// 마지막 flush 시점 (flush 시에만 lock)
    last_flush_time: std::sync::Mutex<std::time::Instant>,
    /// 마지막 flush 이후 upsert 수 — Atomic (lock-free)
    docs_since_last_flush: std::sync::atomic::AtomicUsize,
}

impl Default for IncrementalFlushState {
    fn default() -> Self {
        Self {
            flushed_embeddings: std::sync::Mutex::new(Vec::new()),
            last_flush_time: std::sync::Mutex::new(std::time::Instant::now()),
            docs_since_last_flush: std::sync::atomic::AtomicUsize::new(0),
        }
    }
}

/// 시간 가중 부스트: 최신 문서일수록 1.0에 가까움, 365일 이상 오래되면 0.0
fn time_decay_boost(doc_date: &str, today: &str) -> f32 {
    if doc_date.len() < 10 || today.len() < 10 { return 0.0; }
    let parse = |s: &str| -> Option<i64> {
        let y: i64 = s[..4].parse().ok()?;
        let m: i64 = s[5..7].parse().ok()?;
        let d: i64 = s[8..10].parse().ok()?;
        Some(y * 365 + m * 30 + d)
    };
    let doc_days = match parse(doc_date) { Some(d) => d, None => return 0.0 };
    let today_days = match parse(today) { Some(d) => d, None => return 0.0 };
    let age_days = (today_days - doc_days).max(0) as f32;
    (1.0 - age_days / 365.0).max(0.0)
}

// ── LocalVectorStore ────────────────────────────────────────

/// LocalVectorStore — Blue-Green 슬롯 + mmap + Rayon + HNSW + 키워드 역색인
pub struct LocalVectorStore {
    // 쓰기 경로
    documents: std::sync::Mutex<Vec<StoredDoc>>,
    relations: std::sync::Mutex<Vec<StoredRelation>>,
    entities: std::sync::Mutex<Vec<file_pipeline_core::domain::models::Entity>>,
    keyword_index: std::sync::Mutex<HashMap<String, Vec<String>>>,
    sim_cache: moka::sync::Cache<(String, String), f32>,
    db_path: PathBuf,
    vec_path: PathBuf,
    dim: std::sync::atomic::AtomicUsize,
    batch_mode: std::sync::atomic::AtomicBool,
    relation_set: std::sync::Mutex<HashSet<(String, String, String)>>,

    // 읽기 경로 — Blue-Green
    active_slot: std::sync::RwLock<Arc<SearchSlot>>,
    refresh_state: std::sync::Mutex<RefreshState>,
    refresh_config: RefreshConfig,

    // 증분 flush (lock-free 카운터 + flush 시에만 Mutex)
    incr_state: IncrementalFlushState,
    incr_config: IncrementalFlushConfig,

    // MinHash LSH (3K+ 문서에서 활성화)
    minhash: std::sync::Mutex<file_pipeline_core::domain::crossref_optimizer::MinHashIndex>,
}

#[derive(Serialize, Deserialize, Clone)]
struct StoredRelation {
    source_id: String,
    target_id: String,
    relation_type: String,
    /// Phase 83: 관계 origin (없으면 auto_similarity 가정)
    #[serde(default)]
    origin: String,
}

fn parse_origin(s: &str) -> file_pipeline_core::domain::models::RelationOrigin {
    use file_pipeline_core::domain::models::RelationOrigin;
    match s {
        "user_wikilink" => RelationOrigin::UserWikilink,
        "llm_extracted" => RelationOrigin::LlmExtracted,
        "user_manual"   => RelationOrigin::UserManual,
        "lint_auto_fix" => RelationOrigin::LintAutoFix,
        _ => RelationOrigin::AutoSimilarity,
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct StoredDoc {
    id: String,
    path: String,
    hash: String,
    doc_types: Vec<String>,
    date: String,
    #[serde(default)]
    keywords: Vec<String>,
    #[serde(default)]
    summary: String,
    #[serde(default)]
    vec_offset: usize,
    #[serde(default)]
    vec_dim: usize,
    /// Phase 88 (wikidocs 353407): 원문 미확인/추가 검증 필요 항목
    #[serde(default)]
    needs_verification: Vec<String>,
    /// Phase 88 (wikidocs 353407): 원문으로 답할 수 없는 후속 질문
    #[serde(default)]
    open_questions: Vec<String>,
}

#[derive(Serialize, Deserialize, Default)]
struct DbSnapshot {
    documents: Vec<StoredDoc>,
    relations: Vec<StoredRelation>,
    #[serde(default)]
    entities: Vec<file_pipeline_core::domain::models::Entity>,
}

#[derive(Deserialize)]
struct LegacyDoc {
    id: String,
    path: String,
    hash: String,
    doc_types: Vec<String>,
    date: String,
    embedding: Vec<f32>,
    #[serde(default)]
    keywords: Vec<String>,
}

#[derive(Deserialize)]
struct LegacySnapshot {
    documents: Vec<LegacyDoc>,
    relations: Vec<StoredRelation>,
    #[serde(default)]
    entities: Vec<file_pipeline_core::domain::models::Entity>,
}

impl LocalVectorStore {
    pub fn new() -> Self {
        // PIPELINE_BASE 우선 → cwd settings.db/pipeline.toml → exe_dir 분기
        // (lesson 29 / Phase 85 B-4와 동일 트리. shared::find_data_dir 사본 — 헥사고날상
        // adapters는 shared 의존 금지이므로 코드 사본 유지)
        let base = resolve_data_base();
        Self::with_path(base.join(".local-store.json"))
    }

    pub fn with_path(db_path: PathBuf) -> Self {
        let vec_path = db_path.with_extension("vectors");
        let (snapshot, embeddings) = Self::load_snapshot(&db_path, &vec_path);

        let mut kw_index: HashMap<String, Vec<String>> = HashMap::new();
        for doc in &snapshot.documents {
            for kw in &doc.keywords {
                kw_index.entry(kw.to_lowercase()).or_default().push(doc.id.clone());
            }
        }

        if !embeddings.is_empty() {
            let flat: Vec<u8> = embeddings.iter()
                .flat_map(|v| bytemuck::cast_slice::<f32, u8>(v).to_vec())
                .collect();
            let _ = std::fs::write(&vec_path, &flat);
        }

        let dim = snapshot.documents.first().map(|d| d.vec_dim).unwrap_or(0);
        let rel_set: HashSet<(String, String, String)> = snapshot.relations.iter()
            .map(|r| (r.source_id.clone(), r.target_id.clone(), r.relation_type.clone()))
            .collect();

        tracing::info!("[LocalStore] 로드: {} 문서, {} 관계, {} 엔티티, {} 키워드, dim={} ({})",
            snapshot.documents.len(), snapshot.relations.len(), snapshot.entities.len(),
            kw_index.len(), dim, db_path.display());

        Self {
            documents: std::sync::Mutex::new(snapshot.documents),
            relations: std::sync::Mutex::new(snapshot.relations),
            entities: std::sync::Mutex::new(snapshot.entities),
            keyword_index: std::sync::Mutex::new(kw_index),
            sim_cache: moka::sync::Cache::builder().max_capacity(100_000).build(),
            db_path,
            vec_path,
            dim: std::sync::atomic::AtomicUsize::new(dim),
            batch_mode: std::sync::atomic::AtomicBool::new(false),
            relation_set: std::sync::Mutex::new(rel_set),
            active_slot: std::sync::RwLock::new(Arc::new(SearchSlot::empty())),
            refresh_state: std::sync::Mutex::new(RefreshState {
                docs_since_last_refresh: 0,
                last_refresh_time: std::time::Instant::now(),
            }),
            refresh_config: RefreshConfig::default(),
            incr_state: IncrementalFlushState::default(),
            incr_config: IncrementalFlushConfig::default(),
            minhash: std::sync::Mutex::new(file_pipeline_core::domain::crossref_optimizer::MinHashIndex::new(128, 16)),
        }
    }

    fn load_snapshot(db_path: &std::path::Path, vec_path: &std::path::Path) -> (DbSnapshot, Vec<Vec<f32>>) {
        if !db_path.exists() {
            return (DbSnapshot::default(), vec![]);
        }
        let json = match std::fs::read_to_string(db_path) {
            Ok(s) => s,
            Err(_) => return (DbSnapshot::default(), vec![]),
        };

        if let Ok(snap) = serde_json::from_str::<DbSnapshot>(&json) {
            if snap.documents.iter().all(|d| d.vec_dim > 0) && vec_path.exists() {
                return (snap, vec![]);
            }
        }

        if let Ok(legacy) = serde_json::from_str::<LegacySnapshot>(&json) {
            let mut offset = 0usize;
            let mut docs = Vec::new();
            let mut embeddings = Vec::new();
            for ld in &legacy.documents {
                let edim = ld.embedding.len();
                docs.push(StoredDoc {
                    id: ld.id.clone(), path: ld.path.clone(), hash: ld.hash.clone(),
                    doc_types: ld.doc_types.clone(), date: ld.date.clone(),
                    keywords: ld.keywords.clone(), summary: String::new(),
                    vec_offset: offset, vec_dim: edim,
                    needs_verification: vec![], open_questions: vec![],
                });
                embeddings.push(ld.embedding.clone());
                offset += edim * 4;
            }
            return (DbSnapshot {
                documents: docs, relations: legacy.relations, entities: legacy.entities,
            }, embeddings);
        }

        (DbSnapshot::default(), vec![])
    }

    // ── Blue-Green: 슬롯 구축 + 교체 ──

    /// 새 SearchSlot을 구축하고 atomic swap. 검색 중단 없음.
    /// skip_hnsw=true면 mmap만 교체 (배치 완료 시 HNSW 빌드 비용 회피)
    fn build_and_swap_slot_inner(&self, skip_hnsw: bool) {
        let docs = self.documents.lock().expect("mutex poisoned");
        let doc_count = docs.len();
        let dim = self.dim.load(std::sync::atomic::Ordering::Relaxed);

        // 1. 새 mmap
        let mmap = std::fs::File::open(&self.vec_path).ok()
            .and_then(|f| unsafe { memmap2::Mmap::map(&f).ok() });

        // 2. HNSW (500+ 문서, skip_hnsw가 아닌 경우만)
        let hnsw = if !skip_hnsw && doc_count >= 500 {
            if let Some(ref m) = mmap {
                let embeddings: Vec<Vec<f32>> = docs.iter().map(|d| {
                    let byte_len = d.vec_dim * 4;
                    if d.vec_dim == 0 || d.vec_offset + byte_len > m.len() { return vec![]; }
                    bytemuck::cast_slice::<u8, f32>(&m[d.vec_offset..d.vec_offset + byte_len]).to_vec()
                }).collect();
                let points: Vec<HnswPoint> = embeddings.into_iter().map(HnswPoint).collect();
                let indices: Vec<usize> = (0..points.len()).collect();
                let doc_ids: Vec<String> = docs.iter().map(|d| d.id.clone()).collect();
                if !points.is_empty() {
                    let map = instant_distance::Builder::default().build(points, indices);
                    Some(HnswCache { map, doc_ids })
                } else { None }
            } else { None }
        } else { None };

        drop(docs);

        // 3. Atomic swap
        let new_slot = Arc::new(SearchSlot { mmap, hnsw, doc_count, dim });
        {
            let mut slot = self.active_slot.write().expect("rwlock poisoned");
            *slot = new_slot;
        }

        // 4. Refresh state 리셋
        {
            let mut rs = self.refresh_state.lock().expect("mutex poisoned");
            rs.docs_since_last_refresh = 0;
            rs.last_refresh_time = std::time::Instant::now();
        }
    }

    /// HNSW 포함 전체 슬롯 교체
    fn build_and_swap_slot(&self) {
        self.build_and_swap_slot_inner(false);
    }

    /// mmap만 교체 (HNSW 빌드 스킵 — 배치 완료 시 사용)
    fn swap_slot_mmap_only(&self) {
        self.build_and_swap_slot_inner(true);
    }

    // ── 임베딩 로드 (active_slot 기반) ──

    #[allow(dead_code)]
    fn load_all_embeddings(&self, docs: &[StoredDoc]) -> Vec<Vec<f32>> {
        if docs.is_empty() { return vec![]; }
        let slot = self.active_slot.read().expect("rwlock poisoned").clone();
        slot.load_embeddings(docs)
    }

    // ── 벡터 파일 I/O ──

    fn append_embedding(&self, embedding: &[f32]) -> usize {
        use std::io::Write;
        let offset = std::fs::metadata(&self.vec_path)
            .map(|m| m.len() as usize)
            .unwrap_or(0);
        let bytes = bytemuck::cast_slice::<f32, u8>(embedding);
        let mut file = std::fs::OpenOptions::new()
            .create(true).append(true)
            .open(&self.vec_path)
            .expect("벡터 파일 열기 실패");
        file.write_all(bytes).expect("벡터 쓰기 실패");
        offset
    }

    // ── persist ──

    fn persist(&self) {
        if self.batch_mode.load(std::sync::atomic::Ordering::Relaxed) { return; }
        self.persist_now();
    }

    fn persist_now(&self) {
        let docs = self.documents.lock().expect("mutex poisoned");
        let rels = self.relations.lock().expect("mutex poisoned");
        let ents = self.entities.lock().expect("mutex poisoned");
        let snapshot = DbSnapshot {
            documents: docs.clone(), relations: rels.clone(), entities: ents.clone(),
        };
        if let Ok(json) = serde_json::to_string(&snapshot) {
            let _ = std::fs::write(&self.db_path, json);
        }
    }

    fn update_keyword_index(&self, doc_id: &str, keywords: &[String]) {
        let mut idx = self.keyword_index.lock().expect("mutex poisoned");
        for doc_ids in idx.values_mut() { doc_ids.retain(|id| id != doc_id); }
        for kw in keywords { idx.entry(kw.to_lowercase()).or_default().push(doc_id.to_string()); }
        idx.retain(|_, v| !v.is_empty());
    }

    /// Rayon 병렬 cosine brute-force
    fn search_parallel(embeddings: &[Vec<f32>], docs: &[StoredDoc], query: &[f32], top_k: usize) -> Vec<SimilarDoc> {
        if docs.is_empty() || query.is_empty() { return vec![]; }
        let mut scored: Vec<(f32, usize)> = embeddings.par_iter()
            .enumerate()
            .map(|(i, emb)| (cosine_sim(query, emb), i))
            .collect();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.into_iter().take(top_k).filter_map(|(score, i)| {
            docs.get(i).map(|d| SimilarDoc {
                id: d.id.clone(), path: PathBuf::from(&d.path), score,
                doc_types: d.doc_types.clone(), date: d.date.clone(),
                ..Default::default()
            })
        }).collect()
    }
}

// ── HNSW ────────────────────────────────────────────────────

struct HnswCache {
    map: instant_distance::HnswMap<HnswPoint, usize>,
    #[allow(dead_code)]
    doc_ids: Vec<String>,
}

#[derive(Clone)]
struct HnswPoint(Vec<f32>);

impl instant_distance::Point for HnswPoint {
    fn distance(&self, other: &Self) -> f32 {
        1.0 - cosine_sim(&self.0, &other.0)
    }
}

#[inline]
fn cosine_sim(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() { return 0.0; }
    let mut dot = 0.0f32;
    let mut na = 0.0f32;
    let mut nb = 0.0f32;
    for i in 0..a.len() {
        dot += a[i] * b[i];
        na += a[i] * a[i];
        nb += b[i] * b[i];
    }
    let denom = na.sqrt() * nb.sqrt();
    if denom < 1e-8 { 0.0 } else { dot / denom }
}

impl Default for LocalVectorStore {
    fn default() -> Self { Self::new() }
}

/// PIPELINE_BASE → cwd 분기 → exe_dir 순으로 데이터 디렉토리 결정.
/// `file-pipeline-shared::config::find_data_dir`의 사본 — adapters는 shared 의존 금지(헥사고날).
/// shared 함수의 4번 분기(APPDATA) 생략 — adapters는 dirs 크레이트 의존 회피.
fn resolve_data_base() -> PathBuf {
    // 1. PIPELINE_BASE 환경변수
    if let Ok(base) = std::env::var("PIPELINE_BASE") {
        if !base.trim().is_empty() {
            return PathBuf::from(base);
        }
    }
    // 2. cwd에 settings.db 또는 pipeline.toml 존재
    let cwd = std::path::Path::new(".");
    if cwd.join("settings.db").exists() || cwd.join("pipeline.toml").exists() {
        return PathBuf::from(".");
    }
    // 3. exe_dir fallback
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}

// ── VectorDBPort 구현 ───────────────────────────────────────

impl VectorDBPort for LocalVectorStore {
    fn init(&self) -> Result<()> {
        self.build_and_swap_slot();
        tracing::info!("[LocalStore] DB 초기화 완료 (Blue-Green)");
        Ok(())
    }

    fn batch_begin(&self) {
        self.batch_mode.store(true, std::sync::atomic::Ordering::Relaxed);
        // 배치 중 검색은 의미 없으므로 빈 슬롯으로 교체 → search 즉시 반환
        {
            let mut slot = self.active_slot.write().expect("rwlock poisoned");
            *slot = Arc::new(SearchSlot::empty());
        }
    }

    fn batch_end(&self) {
        self.batch_mode.store(false, std::sync::atomic::Ordering::Relaxed);
        // mmap만 교체, HNSW는 지연 빌드 (검색 시 brute-force → 첫 search에서 빌드)
        self.swap_slot_mmap_only();
        self.persist_now();
    }

    fn should_incremental_flush(&self) -> bool {
        let count = self.incr_state.docs_since_last_flush.load(std::sync::atomic::Ordering::Relaxed);
        if count == 0 { return false; }
        let total = self.documents.lock().expect("docs poisoned").len();
        let threshold = self.incr_config.effective_threshold(total);
        if count >= threshold { return true; }
        let last = self.incr_state.last_flush_time.lock().expect("flush_time poisoned");
        last.elapsed() >= self.incr_config.time_threshold
    }

    fn get_flushed_embeddings(&self) -> Vec<(String, Vec<f32>)> {
        self.incr_state.flushed_embeddings.lock().expect("flushed poisoned").clone()
    }

    fn add_flushed_embedding(&self, doc_id: &str, embedding: &[f32]) {
        self.incr_state.flushed_embeddings.lock().expect("flushed poisoned")
            .push((doc_id.to_string(), embedding.to_vec()));
    }

    fn db_refresh(&self) {
        self.build_and_swap_slot();
        self.persist_now();
        self.incr_state.flushed_embeddings.lock().expect("flushed poisoned").clear();
        self.incr_state.docs_since_last_flush.store(0, std::sync::atomic::Ordering::Relaxed);
        *self.incr_state.last_flush_time.lock().expect("flush_time poisoned") = std::time::Instant::now();
    }

    fn flushed_count(&self) -> usize {
        self.incr_state.flushed_embeddings.lock().expect("flushed poisoned").len()
    }

    fn has_pending_work(&self) -> bool {
        self.incr_state.docs_since_last_flush.load(std::sync::atomic::Ordering::Relaxed) > 0
            || !self.incr_state.flushed_embeddings.lock().expect("flushed poisoned").is_empty()
    }

    fn minhash_candidates(&self, keywords: &[String]) -> Vec<String> {
        // 임시 doc_id로 후보 조회
        let temp_id = "__query__";
        let mut mh = self.minhash.lock().expect("minhash poisoned");
        mh.insert(temp_id, keywords);
        let candidates: Vec<String> = mh.query_candidates(temp_id).into_iter().collect();
        mh.remove(temp_id);
        candidates
    }

    fn minhash_enabled_with(&self, force: bool, min_docs: usize) -> bool {
        if force { return true; }
        self.documents.lock().expect("docs poisoned").len() >= min_docs
    }

    fn doc_count(&self) -> usize {
        self.documents.lock().expect("docs poisoned").len()
    }

    fn upsert(&self, doc: &Document) -> Result<()> {
        let mut docs = self.documents.lock().expect("mutex poisoned");
        let meta = doc.metadata.as_ref();
        // Phase 61 G1: hierarchy(상위 제목 계층)를 키워드 인덱스에 합침 → 검색 시 제목 매칭 향상
        let mut keywords = meta.map(|m| m.keywords.clone()).unwrap_or_default();
        if let Some(m) = meta {
            for title in &m.hierarchy {
                if !title.is_empty() && !keywords.contains(title) {
                    keywords.push(title.clone());
                }
            }
        }
        let dim = doc.embedding.len();

        let offset = self.append_embedding(&doc.embedding);
        self.dim.store(dim, std::sync::atomic::Ordering::Relaxed);

        let summary = meta.map(|m| m.summary.clone()).unwrap_or_default();

        if let Some(existing) = docs.iter_mut().find(|d| d.hash == doc.file_hash) {
            existing.vec_offset = offset;
            existing.vec_dim = dim;
            if let Some(ref meta) = doc.metadata {
                existing.doc_types = meta.doc_types.clone();
                existing.date = meta.date.clone();
                existing.keywords = meta.keywords.clone();
                existing.summary = meta.summary.clone();
                existing.needs_verification = meta.needs_verification.clone();
                existing.open_questions = meta.open_questions.clone();
            }
        } else {
            docs.push(StoredDoc {
                id: doc.file_hash.clone(),
                path: doc.processed_path.clone()
                    .unwrap_or_else(|| doc.origin_path.clone())
                    .to_string_lossy().to_string(),
                hash: doc.file_hash.clone(),
                doc_types: meta.map(|m| m.doc_types.clone()).unwrap_or_default(),
                date: meta.map(|m| m.date.clone()).unwrap_or_default(),
                keywords: keywords.clone(),
                summary: summary.clone(),
                vec_offset: offset,
                vec_dim: dim,
                needs_verification: meta.map(|m| m.needs_verification.clone()).unwrap_or_default(),
                open_questions: meta.map(|m| m.open_questions.clone()).unwrap_or_default(),
            });
        }
        drop(docs);

        self.update_keyword_index(&doc.file_hash, &keywords);
        // MinHash LSH에 등록
        if let Ok(mut mh) = self.minhash.lock() {
            mh.insert(&doc.file_hash, &keywords);
        }
        self.sim_cache.invalidate_all();

        // 비배치 모드: 임계치 기반 슬롯 교체
        if !self.batch_mode.load(std::sync::atomic::Ordering::Relaxed) {
            let should_refresh = {
                let mut rs = self.refresh_state.lock().expect("mutex poisoned");
                rs.docs_since_last_refresh += 1;
                rs.should_refresh(&self.refresh_config)
            };
            if should_refresh {
                self.build_and_swap_slot();
            }
            self.persist();
        } else {
            let mut rs = self.refresh_state.lock().expect("mutex poisoned");
            rs.docs_since_last_refresh += 1;
        }

        // 증분 flush 카운터 (lock-free)
        self.incr_state.docs_since_last_flush.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

        Ok(())
    }

    fn search_similar(&self, embedding: &[f32], top_k: usize) -> Result<Vec<SimilarDoc>> {
        let docs = self.documents.lock().expect("mutex poisoned");
        // Active 슬롯의 Arc를 복제 (lock-free 읽기)
        let slot = self.active_slot.read().expect("rwlock poisoned").clone();

        // brute-force (500 미만 또는 HNSW 없음)
        if docs.len() < 500 || slot.hnsw.is_none() {
            let embeddings = slot.load_embeddings(&docs);
            return Ok(Self::search_parallel(&embeddings, &docs, embedding, top_k));
        }

        // HNSW 검색 (slot이 Arc이므로 swap 중에도 안전)
        if let Some(ref hnsw) = slot.hnsw {
            let query = HnswPoint(embedding.to_vec());
            let mut search = instant_distance::Search::default();
            let results = hnsw.map.search(&query, &mut search);

            let found: Vec<SimilarDoc> = results.take(top_k).filter_map(|item| {
                let idx = *item.value;
                docs.get(idx).map(|d| {
                    // slot mmap에서 직접 cosine 계산
                    let score = slot.mmap.as_ref().map(|m| {
                        let byte_len = d.vec_dim * 4;
                        if d.vec_offset + byte_len <= m.len() {
                            let floats: &[f32] = bytemuck::cast_slice(
                                &m[d.vec_offset..d.vec_offset + byte_len]);
                            cosine_sim(embedding, floats)
                        } else { 0.0 }
                    }).unwrap_or(0.0);
                    SimilarDoc {
                        id: d.id.clone(), path: PathBuf::from(&d.path), score,
                        doc_types: d.doc_types.clone(), date: d.date.clone(),
                        ..Default::default()
                    }
                })
            }).collect();
            Ok(found)
        } else {
            let embeddings = slot.load_embeddings(&docs);
            Ok(Self::search_parallel(&embeddings, &docs, embedding, top_k))
        }
    }

    fn embedding_snapshot(&self) -> Result<file_pipeline_core::domain::models::EmbeddingSnapshot> {
        let docs = self.documents.lock().expect("mutex poisoned");
        let dim = self.dim.load(std::sync::atomic::Ordering::Relaxed);
        if docs.is_empty() || dim == 0 {
            return Ok(file_pipeline_core::domain::models::EmbeddingSnapshot {
                data: vec![], ids: vec![], dim,
            });
        }

        let slot = self.active_slot.read().expect("rwlock poisoned").clone();
        let mmap = match slot.mmap.as_ref() {
            Some(m) => m,
            None => return Ok(file_pipeline_core::domain::models::EmbeddingSnapshot {
                data: vec![], ids: vec![], dim,
            }),
        };

        let mut data = Vec::with_capacity(docs.len() * dim);
        let mut ids = Vec::with_capacity(docs.len());
        for d in docs.iter() {
            let byte_len = d.vec_dim * 4;
            if d.vec_dim == dim && d.vec_offset + byte_len <= mmap.len() {
                let slice = &mmap[d.vec_offset..d.vec_offset + byte_len];
                data.extend_from_slice(bytemuck::cast_slice(slice));
                ids.push(d.id.clone());
            }
        }

        Ok(file_pipeline_core::domain::models::EmbeddingSnapshot { data, ids, dim })
    }

    fn search_hybrid(&self, embedding: &[f32], keyword: &str, top_k: usize) -> Result<Vec<SimilarDoc>> {
        let docs = self.documents.lock().expect("mutex poisoned");
        let keywords: Vec<&str> = keyword.split_whitespace().collect();

        let slot = self.active_slot.read().expect("rwlock poisoned").clone();

        if keywords.is_empty() {
            let embeddings = slot.load_embeddings(&docs);
            return Ok(Self::search_parallel(&embeddings, &docs, embedding, top_k));
        }

        let kw_idx = self.keyword_index.lock().expect("mutex poisoned");
        let mut candidate_ids: HashSet<String> = HashSet::new();
        for kw in &keywords {
            if let Some(ids) = kw_idx.get(&kw.to_lowercase()) {
                for id in ids { candidate_ids.insert(id.clone()); }
            }
        }
        drop(kw_idx);

        // Document Summary 매칭: summary에 키워드가 포함된 문서도 후보에 추가
        for d in docs.iter() {
            if !candidate_ids.contains(&d.id) && !d.summary.is_empty() {
                let summary_lower = d.summary.to_lowercase();
                if keywords.iter().any(|kw| summary_lower.contains(&kw.to_lowercase())) {
                    candidate_ids.insert(d.id.clone());
                }
            }
        }

        if candidate_ids.is_empty() {
            let embeddings = slot.load_embeddings(&docs);
            return Ok(Self::search_parallel(&embeddings, &docs, embedding, top_k));
        }

        let embeddings = slot.load_embeddings(&docs);
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let mut scored: Vec<SimilarDoc> = docs.iter().enumerate()
            .filter(|(_, d)| candidate_ids.contains(&d.id))
            .map(|(i, d)| {
                let cosine = if i < embeddings.len() { cosine_sim(embedding, &embeddings[i]) } else { 0.0 };
                // 시간 가중: 최신 문서에 최대 10% 보너스
                let time_boost = time_decay_boost(&d.date, &today);
                let score = cosine * (1.0 + 0.1 * time_boost);
                SimilarDoc {
                    id: d.id.clone(), path: PathBuf::from(&d.path), score,
                    doc_types: d.doc_types.clone(), date: d.date.clone(),
                    ..Default::default()
                }
            })
            .collect();
        scored.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        Ok(scored.into_iter().take(top_k).collect())
    }

    fn find_by_hash(&self, hash: &str) -> Result<Option<String>> {
        let docs = self.documents.lock().expect("mutex poisoned");
        Ok(docs.iter().find(|d| d.hash == hash).map(|d| d.path.clone()))
    }

    fn find_by_type(&self, doc_type: &str, date: &str) -> Result<Option<String>> {
        let docs = self.documents.lock().expect("mutex poisoned");
        Ok(docs.iter()
            .find(|d| d.doc_types.iter().any(|t| t == doc_type) && d.date == date)
            .map(|d| d.path.clone()))
    }

    fn stats(&self) -> Result<DbStats> {
        let docs = self.documents.lock().expect("mutex poisoned");
        let mut by_type: HashMap<String, u64> = HashMap::new();
        for d in docs.iter() {
            for t in &d.doc_types { *by_type.entry(t.clone()).or_default() += 1; }
        }
        Ok(DbStats {
            total_documents: docs.len() as u64,
            by_type: by_type.into_iter().collect(),
            total_size_bytes: 0, sensitive_count: 0,
        })
    }

    fn list_all(&self) -> Result<Vec<StoredDocSummary>> {
        let docs = self.documents.lock().expect("mutex poisoned");
        Ok(docs.iter().map(|d| StoredDocSummary {
            id: d.id.clone(), path: PathBuf::from(&d.path),
            doc_types: d.doc_types.clone(), date: d.date.clone(),
        }).collect())
    }

    fn get_types(&self, doc_id: &str) -> Result<Vec<String>> {
        let docs = self.documents.lock().expect("mutex poisoned");
        Ok(docs.iter().find(|d| d.id == doc_id).map(|d| d.doc_types.clone()).unwrap_or_default())
    }

    fn update_types(&self, doc_id: &str, types: Vec<String>) -> Result<()> {
        let mut docs = self.documents.lock().expect("mutex poisoned");
        if let Some(d) = docs.iter_mut().find(|d| d.id == doc_id) { d.doc_types = types; }
        drop(docs);
        self.persist();
        Ok(())
    }

    fn link(&self, source_id: &str, target_id: &str, relation: RelationType) -> Result<()> {
        self.link_with_origin(source_id, target_id, relation, file_pipeline_core::domain::models::RelationOrigin::AutoSimilarity)
    }

    fn link_with_origin(
        &self,
        source_id: &str,
        target_id: &str,
        relation: RelationType,
        origin: file_pipeline_core::domain::models::RelationOrigin,
    ) -> Result<()> {
        let rel_str = relation.to_string();
        let key = (source_id.to_string(), target_id.to_string(), rel_str.clone());
        let mut set = self.relation_set.lock().expect("mutex poisoned");
        if !set.insert(key) { return Ok(()); }
        drop(set);
        let mut rels = self.relations.lock().expect("mutex poisoned");
        rels.push(StoredRelation {
            source_id: source_id.to_string(),
            target_id: target_id.to_string(),
            relation_type: rel_str,
            origin: serde_json::to_value(&origin).ok()
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| "auto_similarity".into()),
        });
        drop(rels);
        self.persist();
        Ok(())
    }

    fn find_related(&self, doc_id: &str) -> Result<Vec<DocRelation>> {
        let rels = self.relations.lock().expect("mutex poisoned");
        Ok(rels.iter()
            .filter(|r| r.source_id == doc_id)
            .map(|r| DocRelation {
                source_id: r.source_id.clone(),
                target_id: r.target_id.clone(),
                relation_type: match r.relation_type.as_str() {
                    "references" => RelationType::References,
                    "referenced_by" => RelationType::ReferencedBy,
                    "updates" => RelationType::Updates,
                    "supersedes" => RelationType::Supersedes,
                    _ => RelationType::RelatedTopic,
                },
                confidence: 0.0, context: String::new(), created_at: String::new(),
                origin: parse_origin(&r.origin),
            })
            .collect())
    }

    fn delete(&self, doc_id: &str) -> Result<()> {
        let mut docs = self.documents.lock().expect("mutex poisoned");
        docs.retain(|d| d.id != doc_id);
        let mut rels = self.relations.lock().expect("mutex poisoned");
        rels.retain(|r| r.source_id != doc_id && r.target_id != doc_id);
        drop(docs);
        drop(rels);
        {
            let mut set = self.relation_set.lock().expect("mutex poisoned");
            set.retain(|(s, t, _)| s != doc_id && t != doc_id);
        }
        self.update_keyword_index(doc_id, &[]);
        self.sim_cache.invalidate_all();
        self.build_and_swap_slot();
        self.persist();
        Ok(())
    }

    fn update_content(&self, doc_id: &str, _new_content: &str, change_summary: &str) -> Result<()> {
        tracing::debug!("update_content: {} — {}", doc_id, change_summary);
        Ok(())
    }

    fn get_keywords(&self, doc_id: &str) -> Result<Vec<String>> {
        let docs = self.documents.lock().expect("mutex poisoned");
        Ok(docs.iter().find(|d| d.id == doc_id).map(|d| d.keywords.clone()).unwrap_or_default())
    }

    fn get_metadata(&self, doc_id: &str) -> Result<Option<file_pipeline_core::domain::models::Metadata>> {
        let docs = self.documents.lock().expect("mutex poisoned");
        Ok(docs.iter().find(|d| d.id == doc_id).map(|d| {
            file_pipeline_core::domain::models::Metadata {
                doc_types: d.doc_types.clone(),
                date: d.date.clone(),
                summary: d.summary.clone(),
                keywords: d.keywords.clone(),
                needs_verification: d.needs_verification.clone(),
                open_questions: d.open_questions.clone(),
                ..Default::default()
            }
        }))
    }

    fn upsert_entity(&self, entity: &file_pipeline_core::domain::models::Entity) -> Result<()> {
        let mut ents = self.entities.lock().expect("mutex poisoned");
        if let Some(existing) = ents.iter_mut().find(|e| e.id == entity.id) {
            for doc_id in &entity.doc_ids {
                if !existing.doc_ids.contains(doc_id) { existing.doc_ids.push(doc_id.clone()); }
            }
            existing.mention_count += entity.mention_count;
        } else {
            ents.push(entity.clone());
        }
        drop(ents);
        self.persist();
        Ok(())
    }

    fn list_entities(&self) -> Result<Vec<file_pipeline_core::domain::models::Entity>> {
        Ok(self.entities.lock().expect("mutex poisoned").clone())
    }

    fn entities_for_doc(&self, doc_id: &str) -> Result<Vec<file_pipeline_core::domain::models::Entity>> {
        let ents = self.entities.lock().expect("mutex poisoned");
        Ok(ents.iter().filter(|e| e.doc_ids.contains(&doc_id.to_string())).cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use file_pipeline_core::domain::models::{Document, Metadata};

    fn make_doc(id: &str, embedding: Vec<f32>, keywords: Vec<String>) -> Document {
        Document {
            file_hash: id.to_string(),
            origin_path: PathBuf::from(format!("inbox/{}.txt", id)),
            processed_path: Some(PathBuf::from(format!("processed/{}.zst", id))),
            compressed_origin: None,
            embedding,
            metadata: Some(Metadata {
                doc_types: vec!["test".into()], rationale: String::new(),
                date: "2026-04-17".into(), summary: String::new(),
                keywords, sensitive: false, doi: None, related_docs: vec![],
                source_doc_ids: vec![], search_hints: vec![], entities: vec![],
                ..Default::default()
            }),
        }
    }

    #[test]
    fn test_upsert_and_search() {
        let tmp = tempfile::TempDir::new().expect("tmp");
        let adapter = LocalVectorStore::with_path(tmp.path().join("test.json"));
        adapter.init().expect("init");

        adapter.batch_begin();
        for i in 0..10 {
            let mut emb = vec![0.0f32; 8];
            emb[i % 8] = 1.0;
            adapter.upsert(&make_doc(&format!("d{}", i), emb, vec![])).expect("upsert");
        }
        adapter.batch_end(); // mmap 생성

        let query = vec![1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let results = adapter.search_similar(&query, 3).expect("search");
        assert!(!results.is_empty());
        assert!(results[0].score > 0.5, "top-1 score: {}", results[0].score);
    }

    #[test]
    fn test_keyword_index() {
        let tmp = tempfile::TempDir::new().expect("tmp");
        let adapter = LocalVectorStore::with_path(tmp.path().join("test.json"));
        adapter.init().expect("init");

        adapter.upsert(&make_doc("d1", vec![0.1; 4], vec!["rust".into(), "async".into()])).expect("upsert");
        let kw = adapter.get_keywords("d1").expect("get");
        assert!(kw.contains(&"rust".to_string()));
    }

    #[test]
    fn test_mmap_persistence() {
        let tmp = tempfile::TempDir::new().expect("tmp");
        let path = tmp.path().join("test.json");

        {
            let adapter = LocalVectorStore::with_path(path.clone());
            adapter.init().expect("init");
            adapter.upsert(&make_doc("d1", vec![0.5; 4], vec!["hello".into()])).expect("upsert");
        }

        {
            let adapter = LocalVectorStore::with_path(path);
            let stats = adapter.stats().expect("stats");
            assert_eq!(stats.total_documents, 1);
            let kw = adapter.get_keywords("d1").expect("get");
            assert!(kw.contains(&"hello".to_string()));
        }
    }

    #[test]
    fn test_batch_mode() {
        let tmp = tempfile::TempDir::new().expect("tmp");
        let adapter = LocalVectorStore::with_path(tmp.path().join("test.json"));
        adapter.init().expect("init");

        adapter.batch_begin();
        for i in 0..5 {
            adapter.upsert(&make_doc(&format!("b{}", i), vec![0.1; 4], vec![])).expect("upsert");
        }
        adapter.batch_end();

        assert_eq!(adapter.stats().expect("stats").total_documents, 5);
    }

    #[test]
    fn test_delete_cleanup() {
        let tmp = tempfile::TempDir::new().expect("tmp");
        let adapter = LocalVectorStore::with_path(tmp.path().join("test.json"));
        adapter.init().expect("init");

        adapter.upsert(&make_doc("d1", vec![0.1; 4], vec!["rust".into()])).expect("upsert");
        adapter.delete("d1").expect("delete");

        assert_eq!(adapter.stats().expect("stats").total_documents, 0);
        let idx = adapter.keyword_index.lock().expect("lock");
        assert!(idx.get("rust").is_none() || idx.get("rust").expect("").is_empty());
    }
}
