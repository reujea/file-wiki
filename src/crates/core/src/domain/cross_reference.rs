//! 교차참조 체계적 업데이트 — llm-wiki Ingest 워크플로우 벤치마크
//!
//! llm-wiki 패턴:
//!   새 소스 추가 시 → LLM이 읽고 → 요약 페이지 생성 → 10~15개 기존 페이지 업데이트
//!   각 섹션에 커버리지 태그: [coverage: high -- 15 sources]
//!
//! 우리의 구현:
//!   새 파일 가공 완료 → 유사 문서 top-K 검색 → 각각에 대해 enrich 판단
//!   → 업데이트된 문서에 update_history 기록 + 커버리지 재계산

use anyhow::Result;

use super::models::{Entity, EntityType, RelationType};
use crate::ports::output::{LLMPort, StoragePort, VectorDBPort};
use regex::Regex;

/// 교차참조 업데이트 보고서
#[derive(Debug, Clone, Default)]
pub struct CrossRefReport {
    /// 검토한 관련 문서 수
    pub candidates_checked: u64,
    /// 실제 업데이트된 문서 수
    pub documents_updated: u64,
    /// 새로 생성된 관계 링크 수
    pub links_created: u64,
    /// 업데이트 상세 내역
    pub updates: Vec<CrossRefUpdate>,
    /// 추출된 엔티티 목록
    pub entities: Vec<Entity>,
}

#[derive(Debug, Clone)]
pub struct CrossRefUpdate {
    pub doc_id: String,
    pub change_summary: String,
    pub relation_type: RelationType,
}

impl CrossRefReport {
    pub fn summary(&self) -> String {
        format!(
            "=== 교차참조 업데이트 ===\n\
             후보 검토: {} 문서\n\
             실제 업데이트: {} 문서\n\
             링크 생성: {} 개\n\
             {}",
            self.candidates_checked,
            self.documents_updated,
            self.links_created,
            self.updates
                .iter()
                .map(|u| format!("  - {} ({}: {})", u.doc_id, u.relation_type, u.change_summary))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
}

/// 커버리지 계산기
pub struct CoverageCalculator;

impl CoverageCalculator {
    /// 문서의 섹션별 커버리지 태그 생성
    /// 관련 소스 파일 수에 따라 high/medium/low 분류
    pub fn compute_coverage(
        doc_id: &str,
        vector_db: &dyn VectorDBPort,
    ) -> Result<Vec<CoverageTag>> {
        let relations = vector_db.find_related(doc_id)?;
        let source_count = relations.len();

        let level = if source_count >= 10 {
            "high"
        } else if source_count >= 3 {
            "medium"
        } else {
            "low"
        };

        Ok(vec![CoverageTag {
            section: "전체".to_string(),
            level: level.to_string(),
            source_count,
        }])
    }

    /// 커버리지 태그를 텍스트로 포맷
    pub fn format_tag(tag: &CoverageTag) -> String {
        format!(
            "[coverage: {} -- {} sources]",
            tag.level, tag.source_count
        )
    }
}

#[derive(Debug, Clone)]
pub struct CoverageTag {
    pub section: String,
    pub level: String,
    pub source_count: usize,
}

/// 체계적 교차참조 업데이트 실행기
pub struct CrossRefUpdater;

/// `update_cross_references` 입력 컨텍스트
pub struct CrossRefUpdateContext<'a> {
    pub new_doc_id: &'a str,
    pub new_content: &'a str,
    pub new_doc_types: &'a [String],
    pub embedding: &'a [f32],
    pub top_k: usize,
    pub vector_db: &'a dyn VectorDBPort,
    pub llm: &'a dyn LLMPort,
    pub storage: &'a dyn StoragePort,
}

impl CrossRefUpdater {
    /// 새 문서 가공 완료 후 관련 기존 문서들을 체계적으로 업데이트
    ///
    /// llm-wiki Ingest 패턴:
    /// 1. 유사 문서 top-K 검색
    /// 2. 각 후보에 대해 LLM으로 보강 필요 여부 판단
    /// 3. 보강이 필요하면 기존 가공본 직접 수정
    /// 4. 양방향 링크 생성
    /// 5. 커버리지 태그 갱신
    pub async fn update_cross_references(ctx: CrossRefUpdateContext<'_>) -> Result<CrossRefReport> {
        let CrossRefUpdateContext {
            new_doc_id,
            new_content,
            new_doc_types,
            embedding,
            top_k,
            vector_db,
            llm,
            storage,
        } = ctx;
        let mut report = CrossRefReport::default();

        // 1. 유사 문서 검색
        let similar = vector_db.search_similar(embedding, top_k)?;

        for candidate in &similar {
            if candidate.id == new_doc_id {
                continue;
            }
            if candidate.score < 0.5 {
                continue;
            }

            report.candidates_checked += 1;

            // 2. 기존 문서 내용 읽기
            let existing_content = match storage.decompress_temp(&candidate.path) {
                Ok(temp) => {
                    let content = std::fs::read_to_string(&temp).unwrap_or_default();
                    let _ = std::fs::remove_file(&temp);
                    content
                }
                Err(_) => continue,
            };

            if existing_content.is_empty() {
                continue;
            }

            // 3. LLM으로 보강 판단
            let enrich_result = llm
                .enrich_existing(&existing_content, new_content, new_doc_types)
                .await;

            let relation_type = if candidate.score > 0.9 {
                RelationType::Updates
            } else if candidate.score > 0.7 {
                RelationType::References
            } else {
                RelationType::RelatedTopic
            };

            // 4. 양방향 링크 생성
            let _ = vector_db.link(new_doc_id, &candidate.id, relation_type.clone());
            let _ = vector_db.link(&candidate.id, new_doc_id, relation_type.clone());
            report.links_created += 2;

            // 5. 보강 필요 시 업데이트
            if let Ok(enriched) = enrich_result {
                if enriched.should_update {
                    let _ = vector_db.update_content(
                        &candidate.id,
                        &enriched.updated_content,
                        &enriched.change_summary,
                    );
                    report.documents_updated += 1;
                    report.updates.push(CrossRefUpdate {
                        doc_id: candidate.id.clone(),
                        change_summary: enriched.change_summary,
                        relation_type,
                    });
                }
            }
        }

        Ok(report)
    }

    /// 텍스트에서 엔티티 자동 추출 (규칙 기반, LLM 불필요)
    pub fn extract_entities(text: &str, doc_id: &str, date: &str) -> Vec<Entity> {
        let mut entities = vec![];
        let mut seen = std::collections::HashSet::new();

        // 1. 사람 이름 패턴 (한국어: 2~4글자 성+이름, 괄호 앞)
        let person_re = Regex::new(r"([가-힣]{2,4})\s*\(").unwrap_or_else(|_| Regex::new(r"$^").expect("regex"));
        for cap in person_re.captures_iter(text) {
            let name = cap[1].to_string();
            if !seen.contains(&name) {
                seen.insert(name.clone());
                entities.push(Entity {
                    id: format!("person_{}", hash_name(&name)),
                    name,
                    entity_type: EntityType::Person,
                    doc_ids: vec![doc_id.into()],
                    mention_count: 1,
                    first_seen: date.into(),
                });
            }
        }

        // 2. 금액 패턴 (숫자+만원/억원/달러)
        let amount_re = Regex::new(r"(\d[\d,.]*)\s*(만원|억원|원|달러|\$|USD)").unwrap_or_else(|_| Regex::new(r"$^").expect("regex"));
        for cap in amount_re.captures_iter(text) {
            let amount = format!("{}{}", &cap[1], &cap[2]);
            if !seen.contains(&amount) {
                seen.insert(amount.clone());
                entities.push(Entity {
                    id: format!("amount_{}", hash_name(&amount)),
                    name: amount,
                    entity_type: EntityType::Amount,
                    doc_ids: vec![doc_id.into()],
                    mention_count: 1,
                    first_seen: date.into(),
                });
            }
        }

        // 3. 기술/도구 패턴 (영문 대소문자 혼합 2글자+ 또는 점 포함)
        let tech_re = Regex::new(r"\b([A-Z][a-zA-Z]+(?:\.[a-zA-Z]+)*(?:\s+\d+(?:\.\d+)*)?)\b").unwrap_or_else(|_| Regex::new(r"$^").expect("regex"));
        let stopwords = ["The","This","That","These","Those","When","Where","What","How","From","With","About","After","Before","During","Into","Through","Between"];
        for cap in tech_re.captures_iter(text) {
            let name = cap[1].to_string();
            if name.len() >= 3 && !stopwords.contains(&name.as_str()) && !seen.contains(&name) {
                seen.insert(name.clone());
                entities.push(Entity {
                    id: format!("tech_{}", hash_name(&name)),
                    name,
                    entity_type: EntityType::Technology,
                    doc_ids: vec![doc_id.into()],
                    mention_count: 1,
                    first_seen: date.into(),
                });
            }
        }

        // 4. 프로젝트 패턴 ("프로젝트 X", "Project X")
        let proj_re = Regex::new(r"(?:프로젝트|Project)\s+([A-Za-z가-힣]\S*)").unwrap_or_else(|_| Regex::new(r"$^").expect("regex"));
        for cap in proj_re.captures_iter(text) {
            let name = format!("프로젝트 {}", &cap[1]);
            if !seen.contains(&name) {
                seen.insert(name.clone());
                entities.push(Entity {
                    id: format!("project_{}", hash_name(&name)),
                    name,
                    entity_type: EntityType::Project,
                    doc_ids: vec![doc_id.into()],
                    mention_count: 1,
                    first_seen: date.into(),
                });
            }
        }

        entities
    }

    /// Phase 83: 위키링크 명시 관계 등록.
    /// 가공본 본문에서 `[[xxx]]` 추출 → 코퍼스 문서 ID 매칭 → References + UserWikilink origin으로 link.
    pub fn link_wikilinks(
        new_doc_id: &str,
        text: &str,
        vector_db: &dyn VectorDBPort,
    ) -> Result<usize> {
        use crate::domain::wikilink::{extract_wikilinks, resolve_wikilink_target};
        use crate::domain::models::RelationOrigin;
        let targets = extract_wikilinks(text);
        if targets.is_empty() { return Ok(0); }
        let all = vector_db.list_all().unwrap_or_default();
        let mut linked = 0;
        for t in targets {
            if let Some(target_id) = resolve_wikilink_target(&t, &all) {
                if target_id == new_doc_id { continue; }
                let _ = vector_db.link_with_origin(
                    new_doc_id, &target_id,
                    RelationType::References,
                    RelationOrigin::UserWikilink,
                );
                let _ = vector_db.link_with_origin(
                    &target_id, new_doc_id,
                    RelationType::ReferencedBy,
                    RelationOrigin::UserWikilink,
                );
                linked += 2;
            }
        }
        Ok(linked)
    }

    // [보류] auto_link(SQL 스타일 자동 교차참조) — 2026-05-15 삭제 (lesson 14 형태).
    //
    // 원본: pgvector 패턴 차용. cosine similarity + 키워드 겹침 + 같은 유형/날짜로
    // Supersedes/Updates/RelatedTopic/References 4종 관계를 LLM 호출 없이 자동 부여.
    // 시그니처: (new_doc_id, new_doc_types, new_date, new_keywords, embedding, top_k,
    //          similarity_threshold, supersedes_threshold, keyword_overlap_min,
    //          vector_db, cap_supersedes, cap_updates, cap_related, cap_references)
    //
    // 사유: 7+ Phase 호출처 0건. update_cross_references(LLM 기반)가 동일 영역을 커버.
    // 재도입 트리거: 사용자가 LLM 호출 없는 자동 관계 추출을 명시적으로 요구할 때.
    // 복구 위치: git history — `git log --all -S "pub fn auto_link"` 로 추적.
}

fn hash_name(name: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    name.to_lowercase().hash(&mut h);
    format!("{:016x}", h.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coverage_tag_format() {
        let tag = CoverageTag {
            section: "요약".to_string(),
            level: "high".to_string(),
            source_count: 15,
        };
        assert_eq!(
            CoverageCalculator::format_tag(&tag),
            "[coverage: high -- 15 sources]"
        );
    }

    #[test]
    fn test_crossref_report_summary() {
        let report = CrossRefReport {
            candidates_checked: 5,
            documents_updated: 2,
            links_created: 4,
            updates: vec![CrossRefUpdate {
                doc_id: "abc".into(),
                change_summary: "새 정보 추가".into(),
                relation_type: RelationType::References,
            }],
            entities: vec![],
        };
        let summary = report.summary();
        assert!(summary.contains("후보 검토: 5"));
        assert!(summary.contains("실제 업데이트: 2"));
    }
}
