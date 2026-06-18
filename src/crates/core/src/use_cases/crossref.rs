//! `CrossRefUseCase` 영역 — step-s4 (2026-06-16, hex-arch-d) 정합.
//!
//! `FileProcessingService` 의 교차참조 배치 함수 3건을 별도 impl block 으로 분리.
//! split impl 패턴 (`ProcessFileUseCase` 와 동일) — 호출자/필드/lifetime 변경 부재.
//!
//! 책임 영역:
//! - `flush_crossref` — 행렬 곱 기반 배치 (snapshot + flushed + MinHash + 메타 블로킹 정합)
//! - `flush_crossref_legacy` — N×search_similar 레거시 (검증용 유지, `#[allow(dead_code)]`)
//! - `crossref_queue_len` — 큐 대기 항목 수
//!
//! 자유 함수 `cosine_sim_inline` + `meta_block_pass` = service.rs 잔류 (`pub(crate)`),
//! 본 모듈은 `crate::service::{cosine_sim_inline, meta_block_pass}` 절대경로 호출.

use anyhow::Result;
use tracing::info;

use crate::service::{cosine_sim_inline, meta_block_pass, CrossRefQueueItem, FileProcessingService};

impl FileProcessingService {
    /// 교차참조 배치 실행 — EmbeddingSnapshot 행렬 곱 기반
    /// 큐에 쌓인 항목을 일괄 처리. N×search_similar → 1회 스냅샷 cosine.
    pub fn flush_crossref(&self) -> Result<usize> {
        // 간격 체크
        {
            let mut last_run = self.crossref_last_run.lock().expect("mutex poisoned");
            if let Some(last) = *last_run {
                if last.elapsed().as_secs() < self.crossref_interval_secs {
                    return Ok(0);
                }
            }
            *last_run = Some(std::time::Instant::now());
        }

        let mut items: Vec<CrossRefQueueItem> = {
            let mut queue = self.crossref_queue.lock().expect("mutex poisoned");
            std::mem::take(&mut *queue)
        };
        items.sort_by_key(|i| i.priority);

        if items.is_empty() {
            return Ok(0);
        }

        let n = items.len();
        info!("교차참조 배치 시작 (행렬곱): {} 건", n);
        let t_total = std::time::Instant::now();

        // === 1. 스냅샷 구축 ===
        let t_snap = std::time::Instant::now();
        let emb_snapshot = self.vector_db.embedding_snapshot()?;
        if emb_snapshot.is_empty() { return Ok(0); }
        let m = emb_snapshot.len();
        let dim = emb_snapshot.dim;

        // 전체 문서 메타 (doc_types, date, keywords) 수집
        let all_docs = self.vector_db.list_all()?;
        let doc_meta: std::collections::HashMap<&str, &crate::domain::models::StoredDocSummary> =
            all_docs.iter().map(|d| (d.id.as_str(), d)).collect();

        let kw_snapshot: std::collections::HashMap<String, Vec<String>> = all_docs.iter()
            .map(|d| (d.id.clone(), self.vector_db.get_keywords(&d.id).unwrap_or_default()))
            .collect();

        let mut existing_map: std::collections::HashMap<String, std::collections::HashSet<String>> =
            std::collections::HashMap::new();
        for item in &items {
            let rels = self.vector_db.find_related(&item.doc_id).unwrap_or_default();
            existing_map.insert(item.doc_id.clone(), rels.into_iter().map(|r| r.target_id).collect());
        }

        // incoming count (mutual top-K용) — cap_incoming > 0일 때만 수집
        let incoming_cap = self.crossref_cap_incoming;
        let mut incoming_count: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        if incoming_cap > 0 {
            for doc in &all_docs {
                let rels = self.vector_db.find_related(&doc.id).unwrap_or_default();
                let incoming = rels.iter().filter(|r| r.target_id == doc.id).count();
                incoming_count.insert(doc.id.clone(), incoming);
            }
        }
        let d_snap = t_snap.elapsed();

        // === 2. 행렬 곱: 각 새 문서 × (snapshot + flushed) cosine ===
        let t_search = std::time::Instant::now();
        let threshold = self.crossref_similarity_threshold;
        let flushed = self.vector_db.get_flushed_embeddings();
        let snapshot_ids: std::collections::HashSet<&str> = emb_snapshot.ids.iter().map(|s| s.as_str()).collect();

        // MinHash 활성 판단 (force OR 자동 임계치 도달)
        let minhash_active = self.vector_db
            .minhash_enabled_with(self.crossref_minhash_force, self.crossref_minhash_min_docs);
        let metadata_blocking = self.crossref_metadata_blocking;

        let sim_results: Vec<Vec<(String, f32)>> = items.iter().map(|item| {
            if item.embedding.len() != dim { return vec![]; }
            let q = &item.embedding;
            let mut candidates: Vec<(String, f32)> = Vec::new();

            // MinHash 후보 set (활성 시): 자카드 유사 키워드를 가진 문서만 비교
            let mh_set: Option<std::collections::HashSet<String>> = if minhash_active {
                Some(self.vector_db.minhash_candidates(&item.keywords).into_iter().collect())
            } else {
                None
            };
            let item_kw_set: std::collections::HashSet<&String> = item.keywords.iter().collect();

            // vs snapshot (기존 문서)
            for j in 0..m {
                let cand_id = &emb_snapshot.ids[j];
                if cand_id == &item.doc_id { continue; }
                if let Some(ref s) = mh_set {
                    if !s.contains(cand_id.as_str()) { continue; }
                }
                if metadata_blocking
                    && !meta_block_pass(item, cand_id.as_str(), &doc_meta, &kw_snapshot, &item_kw_set) {
                    continue;
                }
                let d = emb_snapshot.get(j);
                let score = cosine_sim_inline(q, d);
                if score >= threshold {
                    candidates.push((cand_id.clone(), score));
                }
            }

            // vs flushed (이전 flush 문서, refresh 전 — snapshot에 없는 것만)
            for (fid, femb) in &flushed {
                if *fid == item.doc_id || femb.len() != dim { continue; }
                if snapshot_ids.contains(fid.as_str()) { continue; }
                if let Some(ref s) = mh_set {
                    if !s.contains(fid.as_str()) { continue; }
                }
                if metadata_blocking
                    && !meta_block_pass(item, fid.as_str(), &doc_meta, &kw_snapshot, &item_kw_set) {
                    continue;
                }
                let score = cosine_sim_inline(q, femb);
                if score >= threshold {
                    candidates.push((fid.clone(), score));
                }
            }

            candidates
        }).collect();

        let d_search = t_search.elapsed();

        // === 3. link 판정 (기존 로직 재사용) ===
        let t_link = std::time::Instant::now();
        self.vector_db.batch_begin();
        let mut total_links = 0u64;
        let sup_threshold = self.crossref_supersedes_threshold;
        let kw_min = self.crossref_keyword_overlap_min;

        for (i, candidates) in sim_results.iter().enumerate() {
            let item = &items[i];
            let existing = existing_map.get(&item.doc_id);
            let new_kw: std::collections::HashSet<&String> = item.keywords.iter().collect();

            let mut cnt_sup = 0usize;
            let mut cnt_upd = 0usize;
            let mut cnt_rel = 0usize;
            let mut cnt_ref = 0usize;

            // score 내림차순 정렬
            let mut sorted: Vec<(String, f32)> = candidates.clone();
            sorted.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

            for (cand_id, score) in &sorted {
                let score = *score;
                if let Some(ex) = existing {
                    if ex.contains(cand_id.as_str()) { continue; }
                }

                // mutual top-K: 대상의 incoming이 cap을 넘으면 skip
                if incoming_cap > 0 {
                    let cand_incoming = incoming_count.get(cand_id.as_str()).copied().unwrap_or(0);
                    if cand_incoming >= incoming_cap { continue; }
                }

                let cand_meta = doc_meta.get(cand_id.as_str());
                let cand_types: Vec<String> = cand_meta.map(|d| d.doc_types.clone()).unwrap_or_default();
                let cand_date_str = cand_meta.map(|d| d.date.as_str()).unwrap_or("");

                let same_type = cand_types.iter().any(|t| item.doc_types.contains(t));
                let new_date = crate::domain::models::DocDate::from_string(&item.date);
                let cand_date = crate::domain::models::DocDate::from_string(cand_date_str);
                let dates_reliable = crate::domain::models::DocDate::both_reliable(&new_date, &cand_date);
                let same_date = dates_reliable && cand_date_str == item.date;

                // Supersedes
                if score > sup_threshold && same_type && dates_reliable && cnt_sup < self.crossref_cap_supersedes {
                    let _ = self.vector_db.link(&item.doc_id, cand_id, crate::domain::models::RelationType::Supersedes);
                    let _ = self.vector_db.link(cand_id, &item.doc_id, crate::domain::models::RelationType::References);
                    total_links += 2;
                    cnt_sup += 1;
                    continue;
                }

                // Updates
                if same_type && same_date && cnt_upd < self.crossref_cap_updates {
                    let _ = self.vector_db.link(&item.doc_id, cand_id, crate::domain::models::RelationType::Updates);
                    let _ = self.vector_db.link(cand_id, &item.doc_id, crate::domain::models::RelationType::Updates);
                    total_links += 2;
                    cnt_upd += 1;
                    continue;
                }

                // RelatedTopic
                let cand_kw = kw_snapshot.get(cand_id.as_str());
                let overlap = cand_kw
                    .map(|kws| kws.iter().filter(|k| new_kw.contains(k)).count())
                    .unwrap_or(0);
                if overlap >= kw_min && cnt_rel < self.crossref_cap_related {
                    let _ = self.vector_db.link(&item.doc_id, cand_id, crate::domain::models::RelationType::RelatedTopic);
                    let _ = self.vector_db.link(cand_id, &item.doc_id, crate::domain::models::RelationType::RelatedTopic);
                    total_links += 2;
                    cnt_rel += 1;
                    continue;
                }

                // References + ReferencedBy
                if score > threshold && cnt_ref < self.crossref_cap_references {
                    let _ = self.vector_db.link(&item.doc_id, cand_id, crate::domain::models::RelationType::References);
                    let _ = self.vector_db.link(cand_id, &item.doc_id, crate::domain::models::RelationType::ReferencedBy);
                    total_links += 2;
                    cnt_ref += 1;
                }

                if cnt_sup >= self.crossref_cap_supersedes && cnt_upd >= self.crossref_cap_updates
                    && cnt_rel >= self.crossref_cap_related && cnt_ref >= self.crossref_cap_references { break; }
            }
        }

        let d_link = t_link.elapsed();
        let t_persist = std::time::Instant::now();
        self.vector_db.batch_end();
        let d_persist = t_persist.elapsed();

        // batch_mode 중이면 flushed_embeddings에 추가 (mmap 미갱신 상태에서 다음 flush 검색용)
        if self.vector_db.flushed_count() > 0 || !items.is_empty() {
            // batch_end에서 mmap이 갱신되므로, flushed는 batch_end 전 상태에서만 의미 있음
            // 여기서는 batch_end가 호출되었으므로 flushed 불필요 (snapshot에 이미 포함)
        }

        // 강제 refresh 체크 (메모리 보호: flushed 10K 초과)
        if self.vector_db.flushed_count() >= 10_000 {
            info!("flushed 임계치 도달 → 강제 db_refresh");
            self.vector_db.db_refresh();
        }

        let d_total = t_total.elapsed();
        let mh_tag = if minhash_active { " minhash=on" } else { "" };
        let mb_tag = if metadata_blocking { " block=on" } else { "" };
        println!(
            "  [flush-matrix] snap={:.1}s search={:.1}s link={:.1}s persist={:.1}s total={:.1}s | {}docs {}links (flushed: {}){}{}",
            d_snap.as_secs_f64(), d_search.as_secs_f64(), d_link.as_secs_f64(),
            d_persist.as_secs_f64(), d_total.as_secs_f64(), n, total_links, self.vector_db.flushed_count(),
            mh_tag, mb_tag
        );
        info!(
            "flush-matrix: snap={:.1}s search={:.1}s link={:.1}s persist={:.1}s total={:.1}s | {}docs {}links",
            d_snap.as_secs_f64(), d_search.as_secs_f64(), d_link.as_secs_f64(),
            d_persist.as_secs_f64(), d_total.as_secs_f64(), n, total_links
        );
        Ok(n)
    }

    /// 교차참조 배치 (레거시: N×search_similar) — 검증용 유지
    #[allow(dead_code)]
    pub fn flush_crossref_legacy(&self) -> Result<usize> {
        // 간격 체크
        {
            let mut last_run = self.crossref_last_run.lock().expect("mutex poisoned");
            if let Some(last) = *last_run {
                if last.elapsed().as_secs() < self.crossref_interval_secs {
                    return Ok(0);
                }
            }
            *last_run = Some(std::time::Instant::now());
        }

        let mut items: Vec<CrossRefQueueItem> = {
            let mut queue = self.crossref_queue.lock().expect("mutex poisoned");
            std::mem::take(&mut *queue)
        };
        items.sort_by_key(|i| i.priority);

        if items.is_empty() {
            return Ok(0);
        }

        let n = items.len();
        info!("교차참조 배치 시작: {} 건", n);
        let t_total = std::time::Instant::now();

        // === 1. 스냅샷 구축 (keywords + 기존 관계) ===
        let t_snap = std::time::Instant::now();
        let all_docs = self.vector_db.list_all()?;
        let m = all_docs.len();
        if m == 0 { return Ok(0); }

        let kw_snapshot: std::collections::HashMap<String, Vec<String>> = all_docs.iter()
            .map(|d| (d.id.clone(), self.vector_db.get_keywords(&d.id).unwrap_or_default()))
            .collect();

        let mut existing_map: std::collections::HashMap<String, std::collections::HashSet<String>> =
            std::collections::HashMap::new();
        for item in &items {
            let rels = self.vector_db.find_related(&item.doc_id).unwrap_or_default();
            existing_map.insert(item.doc_id.clone(), rels.into_iter().map(|r| r.target_id).collect());
        }
        let d_snap = t_snap.elapsed();

        // === 2. search + link ===
        self.vector_db.batch_begin();
        let mut total_links = 0u64;
        let threshold = self.crossref_similarity_threshold;
        let sup_threshold = self.crossref_supersedes_threshold;
        let kw_min = self.crossref_keyword_overlap_min;
        let mut d_search = std::time::Duration::ZERO;
        let mut d_link = std::time::Duration::ZERO;

        for item in &items {
            let ts = std::time::Instant::now();
            let scores = self.vector_db.search_similar(&item.embedding, m)?;
            d_search += ts.elapsed();

            let tl = std::time::Instant::now();
            let existing = existing_map.get(&item.doc_id);
            let new_kw: std::collections::HashSet<&String> = item.keywords.iter().collect();

            let mut cnt_sup = 0usize;
            let mut cnt_upd = 0usize;
            let mut cnt_rel = 0usize;
            let mut cnt_ref = 0usize;

            for candidate in &scores {
                if candidate.id == item.doc_id { continue; }
                if candidate.score < threshold { continue; }
                if let Some(ex) = existing {
                    if ex.contains(&candidate.id) { continue; }
                }

                let same_type = candidate.doc_types.iter()
                    .any(|t| item.doc_types.contains(t));
                let new_date = crate::domain::models::DocDate::from_string(&item.date);
                let cand_date = crate::domain::models::DocDate::from_string(&candidate.date);
                let dates_reliable = crate::domain::models::DocDate::both_reliable(&new_date, &cand_date);
                let same_date = dates_reliable && candidate.date == item.date;

                // Supersedes
                if candidate.score > sup_threshold && same_type && dates_reliable && cnt_sup < 2 {
                    let _ = self.vector_db.link(&item.doc_id, &candidate.id, crate::domain::models::RelationType::Supersedes);
                    let _ = self.vector_db.link(&candidate.id, &item.doc_id, crate::domain::models::RelationType::References);
                    total_links += 2;
                    cnt_sup += 1;
                    continue;
                }

                // Updates
                if same_type && same_date && cnt_upd < self.crossref_cap_updates {
                    let _ = self.vector_db.link(&item.doc_id, &candidate.id, crate::domain::models::RelationType::Updates);
                    let _ = self.vector_db.link(&candidate.id, &item.doc_id, crate::domain::models::RelationType::Updates);
                    total_links += 2;
                    cnt_upd += 1;
                    continue;
                }

                // RelatedTopic (스냅샷에서 조회 — Mutex 없음)
                let cand_kw = kw_snapshot.get(&candidate.id);
                let overlap = cand_kw
                    .map(|kws| kws.iter().filter(|k| new_kw.contains(k)).count())
                    .unwrap_or(0);
                if overlap >= kw_min && cnt_rel < self.crossref_cap_related {
                    let _ = self.vector_db.link(&item.doc_id, &candidate.id, crate::domain::models::RelationType::RelatedTopic);
                    let _ = self.vector_db.link(&candidate.id, &item.doc_id, crate::domain::models::RelationType::RelatedTopic);
                    total_links += 2;
                    cnt_rel += 1;
                    continue;
                }

                // References (양방향: References + ReferencedBy)
                if candidate.score > 0.7 && cnt_ref < 10 {
                    let _ = self.vector_db.link(&item.doc_id, &candidate.id, crate::domain::models::RelationType::References);
                    let _ = self.vector_db.link(&candidate.id, &item.doc_id, crate::domain::models::RelationType::ReferencedBy);
                    total_links += 2;
                    cnt_ref += 1;
                }

                if cnt_sup >= self.crossref_cap_supersedes && cnt_upd >= self.crossref_cap_updates
                    && cnt_rel >= self.crossref_cap_related && cnt_ref >= self.crossref_cap_references { break; }
            }
            d_link += tl.elapsed();
        }

        let t_persist = std::time::Instant::now();
        self.vector_db.batch_end();
        let d_persist = t_persist.elapsed();

        let d_total = t_total.elapsed();
        // プロファイリング出力 (println で確実に表示)
        println!(
            "  [flush] snap={:.1}s search={:.1}s link={:.1}s persist={:.1}s total={:.1}s | {}docs {}links",
            d_snap.as_secs_f64(), d_search.as_secs_f64(), d_link.as_secs_f64(),
            d_persist.as_secs_f64(), d_total.as_secs_f64(), n, total_links
        );
        info!(
            "flush breakdown: snap={:.1}s search={:.1}s link={:.1}s persist={:.1}s total={:.1}s | {}docs {}links",
            d_snap.as_secs_f64(), d_search.as_secs_f64(), d_link.as_secs_f64(),
            d_persist.as_secs_f64(), d_total.as_secs_f64(), n, total_links
        );
        Ok(n)
    }

    /// 교차참조 큐에 대기 중인 항목 수
    pub fn crossref_queue_len(&self) -> usize {
        self.crossref_queue.lock().expect("mutex poisoned").len()
    }
}
