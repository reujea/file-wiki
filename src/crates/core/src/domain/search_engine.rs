//! step-s1 (2026-06-16, hex-arch-d): `handle_search` 410줄 안 순수 검색 도메인 로직을 추출.
//!
//! - mode 분기 (exact / related / recent / fusion / default)
//! - keyword + 그래프 확장
//! - reranker 호출
//! - CRAG 신뢰도 (correct / ambiguous / incorrect) + 보완 (그래프 확장 / keyword 전용)
//! - HyDE 폴백 (트리거 #6 인프라)
//! - KG 1-hop 확장 (Ruflo A2 + Phase 103 G3 빔)
//! - diversity 강화 (Ruflo B1)
//! - TF-IDF 재순위 (Phase 103 G4 GraphRAG 흡수)
//!
//! 잔류 (mcp_server.rs 측):
//! - search_cache (Mutex<HashMap>)
//! - search_log (Mutex<Vec>)
//! - audit_trace (AuditPort)
//! - record_search_mode + record_crag (settings.db 카운터 영속화)
//! - PII mask + Sentence Window snippet (출력 포맷)
//!
//! 분리 의도 = SearchEngine 의 의존성 = port trait 5종 (VectorDBPort / EmbeddingPort /
//! RerankerPort / LLMPort / StoragePort) + 순수 config struct. settings_db / 캐시 / 로그 부재 →
//! lesson #14 R1 family 정합.

use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use tracing::info;

use crate::domain::models::SimilarDoc;
use crate::ports::output::{EmbeddingPort, LLMPort, RerankerPort, StoragePort, VectorDBPort};

/// 검색 엔진 설정 — `McpState` 와 `SearchConfig` 사이 브릿지.
#[derive(Debug, Clone)]
pub struct SearchEngineConfig {
    /// Ruflo A2: KG 1-hop 확장 개수 (0=비활성).
    pub expand_kg_hops: usize,
    /// Ruflo B1: 동일 doc_type 결과 임계값 (0=비활성).
    pub diversity_threshold: usize,
    /// 트리거 #6: HyDE 폴백 활성 (디폴트 false).
    pub hyde_enabled: bool,
    /// HyDE 폴백 발동 임계 (디폴트 3).
    pub hyde_min_results: usize,
    /// Phase 103 G3: KG Multi-hop 빔 검색 활성 (디폴트 false).
    pub kg_beam_search: bool,
    /// Phase 103 G4: TF-IDF 다양성 재순위 활성 (디폴트 false).
    pub tfidf_rerank_enabled: bool,
}

impl Default for SearchEngineConfig {
    fn default() -> Self {
        Self {
            expand_kg_hops: 0,
            diversity_threshold: 0,
            hyde_enabled: false,
            hyde_min_results: 3,
            kg_beam_search: false,
            tfidf_rerank_enabled: false,
        }
    }
}

/// 검색 질의 입력 — MCP 도구 args (`query` / `keyword` / `doc_type` / ...) 정합.
#[derive(Debug, Clone)]
pub struct SearchQuery {
    pub query: String,
    pub keyword: Option<String>,
    pub doc_type: Option<String>,
    pub mode: String,
    pub top_k: usize,
    pub date_from: String,
    pub date_to: String,
}

/// 검색 결과 출력 — `results` + CRAG `confidence` ("correct" / "ambiguous" / "incorrect").
#[derive(Debug, Clone)]
pub struct SearchOutcome {
    pub results: Vec<SimilarDoc>,
    pub confidence: String,
}

/// 검색 엔진 — port 5종 + 순수 config struct. settings.db / 캐시 / 로그 부재.
pub struct SearchEngine {
    pub vector_db: Arc<dyn VectorDBPort>,
    pub embedding: Arc<dyn EmbeddingPort>,
    pub reranker: Arc<dyn RerankerPort>,
    pub llm: Arc<dyn LLMPort>,
    pub storage: Arc<dyn StoragePort>,
    pub cfg: SearchEngineConfig,
}

impl SearchEngine {
    pub async fn run_search(&self, q: &SearchQuery) -> Result<SearchOutcome> {
        let query = q.query.as_str();
        let keyword = q.keyword.as_deref();
        let doc_type = q.doc_type.as_deref();
        let mode = q.mode.as_str();
        let top_k = q.top_k;

        // 질의 확장 (mode=default 또는 related에서, 짧은 쿼리에 자동 적용)
        let expanded_query = if query.split_whitespace().count() <= 3 && (mode == "default" || mode == "related") {
            let words: Vec<&str> = query.split_whitespace().collect();
            let extra = words.join(" ");
            format!("{} {}", query, extra)
        } else {
            query.to_string()
        };
        let embedding = self.embedding.embed(&expanded_query).await?;

        let mut results = match mode {
            "exact" => {
                let kw = keyword.unwrap_or(query);
                self.vector_db.search_hybrid(&embedding, kw, top_k * 3)?
            }
            "related" => {
                let mut dense = self.vector_db.search_similar(&embedding, top_k * 3)?;
                let top_ids: Vec<String> = dense.iter().take(3).map(|r| r.id.clone()).collect();
                for id in &top_ids {
                    if let Ok(rels) = self.vector_db.find_related(id) {
                        for rel in rels.iter().take(2) {
                            if !dense.iter().any(|r| r.id == rel.target_id) {
                                if let Ok(all) = self.vector_db.list_all() {
                                    if let Some(doc) = all.iter().find(|d| d.id == rel.target_id) {
                                        dense.push(SimilarDoc {
                                            id: doc.id.clone(), path: doc.path.clone(),
                                            score: 0.5,
                                            doc_types: doc.doc_types.clone(), date: doc.date.clone(),
                                            ..Default::default()
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
                dense
            }
            "recent" => {
                let mut r = if let Some(kw) = keyword {
                    self.vector_db.search_hybrid(&embedding, kw, top_k * 5)?
                } else {
                    self.vector_db.search_similar(&embedding, top_k * 5)?
                };
                r.sort_by(|a, b| b.date.cmp(&a.date).then(b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal)));
                r
            }
            "fusion" => {
                let mut all_results = self.vector_db.search_similar(&embedding, top_k * 3)?;
                for word in query.split_whitespace().take(3) {
                    let kw_results = self.vector_db.search_hybrid(&embedding, word, top_k * 2)?;
                    for r in kw_results {
                        if !all_results.iter().any(|a| a.id == r.id) {
                            all_results.push(r);
                        }
                    }
                }
                let mut rrf_scores: std::collections::HashMap<String, f64> = std::collections::HashMap::new();
                for (rank, r) in all_results.iter().enumerate() {
                    *rrf_scores.entry(r.id.clone()).or_default() += 1.0 / (60.0 + rank as f64);
                }
                all_results.sort_by(|a, b| {
                    let sa = rrf_scores.get(&a.id).unwrap_or(&0.0);
                    let sb = rrf_scores.get(&b.id).unwrap_or(&0.0);
                    sb.partial_cmp(sa).unwrap_or(std::cmp::Ordering::Equal)
                });
                all_results.dedup_by(|a, b| a.id == b.id);
                all_results
            }
            _ => {
                if let Some(kw) = keyword {
                    self.vector_db.search_hybrid(&embedding, kw, top_k * 3)?
                } else {
                    self.vector_db.search_similar(&embedding, top_k * 3)?
                }
            }
        };

        // doc_type 필터
        if let Some(dt) = doc_type {
            results.retain(|r| r.doc_types.iter().any(|t| t == dt));
        }

        // 날짜 필터
        if !q.date_from.is_empty() || !q.date_to.is_empty() {
            results.retain(|r| {
                let date = &r.date;
                let after = q.date_from.is_empty() || date.as_str() >= q.date_from.as_str();
                let before = q.date_to.is_empty() || date.as_str() <= q.date_to.as_str();
                after && before
            });
        }

        // 리랭킹 (활성화 시)
        if self.reranker.is_enabled() && !results.is_empty() {
            let fallback = results.clone();
            results = self.reranker.rerank(query, results).await.unwrap_or(fallback);
        }

        // CRAG: 검색 신뢰도 판정 + 보완 검색
        let top_score = results.first().map(|r| r.score).unwrap_or(0.0);
        let confidence = if top_score >= 0.8 { "correct" }
            else if top_score >= 0.5 { "ambiguous" }
            else { "incorrect" };

        if confidence == "ambiguous" {
            let top_ids: Vec<String> = results.iter().take(3).map(|r| r.id.clone()).collect();
            for id in &top_ids {
                if let Ok(rels) = self.vector_db.find_related(id) {
                    for rel in rels.iter().take(2) {
                        if !results.iter().any(|r| r.id == rel.target_id) {
                            if let Ok(all) = self.vector_db.list_all() {
                                if let Some(doc) = all.iter().find(|d| d.id == rel.target_id) {
                                    results.push(SimilarDoc {
                                        id: doc.id.clone(), path: doc.path.clone(),
                                        score: top_score * 0.7,
                                        doc_types: doc.doc_types.clone(), date: doc.date.clone(),
                                        ..Default::default()
                                    });
                                }
                            }
                        }
                    }
                }
            }
        } else if confidence == "incorrect" && !results.is_empty() {
            let kw_results = self.vector_db.search_hybrid(&embedding, query, top_k * 2)?;
            for kr in kw_results {
                if !results.iter().any(|r| r.id == kr.id) {
                    results.push(kr);
                }
            }
        }

        // 트리거 #6 HyDE 폴백
        if self.cfg.hyde_enabled && results.len() < self.cfg.hyde_min_results {
            if let Ok(hyde_text) = self.llm.generate_hypothetical(query).await {
                if hyde_text != query && !hyde_text.trim().is_empty() {
                    if let Ok(hyde_emb) = self.embedding.embed(&hyde_text).await {
                        let hyde_results = self.vector_db
                            .search_similar(&hyde_emb, top_k * 2)
                            .unwrap_or_default();
                        for hr in hyde_results {
                            if !results.iter().any(|r| r.id == hr.id) {
                                let mut adj = hr.clone();
                                adj.score *= 0.6;
                                results.push(adj);
                            }
                        }
                        info!("[hyde] fallback triggered: results {} (min {})", results.len(), self.cfg.hyde_min_results);
                    }
                }
            }
        }

        // Ruflo A2: KG 1-hop 확장 + Phase 103 G3 빔
        if self.cfg.expand_kg_hops > 0 && !results.is_empty() {
            let mut added = 0usize;
            let max_add = self.cfg.expand_kg_hops;
            let seed_ids: Vec<String> = results.iter().take(top_k.min(results.len()))
                .map(|r| r.id.clone()).collect();
            let seed_count = if self.cfg.kg_beam_search { max_add.min(seed_ids.len()) } else { seed_ids.len() };
            for id in seed_ids.into_iter().take(seed_count) {
                if added >= max_add { break; }
                if let Ok(rels) = self.vector_db.find_related(&id) {
                    for rel in rels {
                        if added >= max_add { break; }
                        if results.iter().any(|r| r.id == rel.target_id) { continue; }
                        if let Ok(all) = self.vector_db.list_all() {
                            if let Some(target) = all.into_iter().find(|d| d.id == rel.target_id) {
                                results.push(SimilarDoc {
                                    id: target.id,
                                    path: target.path,
                                    score: 0.0,
                                    doc_types: target.doc_types,
                                    date: target.date,
                                    hierarchy: vec![],
                                });
                                added += 1;
                            }
                        }
                    }
                }
            }
        }

        // Ruflo B1: 다양성 강화
        if self.cfg.diversity_threshold > 0 && results.len() > top_k {
            let threshold = self.cfg.diversity_threshold;
            let head_len = top_k.min(results.len());
            let mut type_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
            for r in results.iter().take(head_len) {
                for t in &r.doc_types {
                    *type_counts.entry(t.clone()).or_insert(0) += 1;
                }
            }
            let dominant: Vec<String> = type_counts.iter()
                .filter(|(_, c)| **c > threshold)
                .map(|(t, _)| t.clone()).collect();
            if !dominant.is_empty() {
                let mut promote_idx: Option<usize> = None;
                for (idx, r) in results.iter().enumerate().skip(head_len) {
                    let is_dominant = r.doc_types.iter().any(|t| dominant.contains(t));
                    if !is_dominant {
                        promote_idx = Some(idx);
                        break;
                    }
                }
                if let Some(p) = promote_idx {
                    let mut demote_idx: Option<usize> = None;
                    for (idx, r) in results.iter().enumerate().take(head_len).rev() {
                        if r.doc_types.iter().any(|t| dominant.contains(t)) {
                            demote_idx = Some(idx);
                            break;
                        }
                    }
                    if let Some(d) = demote_idx {
                        results.swap(d, p);
                    }
                }
            }
        }

        // Phase 103 G4: TF-IDF 다양성 재순위
        if self.cfg.tfidf_rerank_enabled && results.len() > top_k {
            let head_len = top_k.min(results.len());
            let mut tokens_per_doc: Vec<std::collections::HashSet<String>> = Vec::with_capacity(results.len());
            for r in results.iter() {
                let text = self.storage.read_header(&r.path, 100).unwrap_or_default();
                let set: std::collections::HashSet<String> = text.to_lowercase()
                    .split(|c: char| !c.is_alphanumeric())
                    .filter(|t| t.len() >= 3)
                    .map(String::from)
                    .collect();
                tokens_per_doc.push(set);
            }
            let mut seen_tokens: std::collections::HashSet<String> = std::collections::HashSet::new();
            for s in tokens_per_doc.iter().take(head_len) {
                for t in s.iter() { seen_tokens.insert(t.clone()); }
            }
            let mut best_idx: Option<usize> = None;
            let mut best_novelty: f32 = 0.0;
            for (idx, s) in tokens_per_doc.iter().enumerate().skip(head_len) {
                if s.is_empty() { continue; }
                let novel: usize = s.iter().filter(|t| !seen_tokens.contains(*t)).count();
                let ratio = novel as f32 / s.len() as f32;
                if ratio > best_novelty {
                    best_novelty = ratio;
                    best_idx = Some(idx);
                }
            }
            if best_novelty >= 0.5 {
                if let (Some(promote), demote) = (best_idx, head_len.saturating_sub(1)) {
                    if demote < results.len() && promote < results.len() {
                        results.swap(demote, promote);
                    }
                }
            }
        }

        Ok(SearchOutcome {
            results,
            confidence: confidence.to_string(),
        })
    }
}

/// SearchPort — `SearchEngine` 의 trait 인터페이스. fp-plugin-search 이관 시점 대체.
#[async_trait]
pub trait SearchPort: Send + Sync {
    async fn search(&self, q: &SearchQuery) -> Result<SearchOutcome>;
}

#[async_trait]
impl SearchPort for SearchEngine {
    async fn search(&self, q: &SearchQuery) -> Result<SearchOutcome> {
        self.run_search(q).await
    }
}
