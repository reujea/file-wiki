use std::path::Path;

use anyhow::{Context, Result};
use serde::Serialize;

use crate::domain::models::StoredDocSummary;
use crate::ports::output::{StoragePort, VectorDBPort};

/// Obsidian 호환 위키 내보내기 (Phase D)
pub struct WikiExporter;

impl WikiExporter {
    /// 전체 가공본을 마크다운 위키로 내보내기
    pub fn export(
        vector_db: &dyn VectorDBPort,
        storage: &dyn StoragePort,
        output_dir: &Path,
    ) -> Result<ExportReport> {
        std::fs::create_dir_all(output_dir)?;

        let all_docs = vector_db.list_all()?;
        let mut report = ExportReport::default();
        let mut index_entries = Vec::new();

        for doc in &all_docs {
            // 가공본 해제
            let content = match storage.read_header(&doc.path, 1000) {
                Ok(c) => c,
                Err(_) => {
                    // read_header 실패 시 전체 해제 시도
                    match storage.decompress_temp(&doc.path) {
                        Ok(temp) => {
                            let c = std::fs::read_to_string(&temp).unwrap_or_default();
                            let _ = std::fs::remove_file(&temp);
                            c
                        }
                        Err(e) => {
                            report.errors.push(format!("{:?}: {}", doc.path, e));
                            continue;
                        }
                    }
                }
            };

            // 메타데이터 + 본문 분리
            let (meta_section, body) = split_meta_content(&content);

            // 유형별 디렉토리
            let type_dir = doc
                .doc_types
                .first()
                .map(|t| t.as_str())
                .unwrap_or("etc");
            let dir = output_dir.join(type_dir);
            std::fs::create_dir_all(&dir)?;

            // 파일명 결정
            let filename = doc
                .path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(&doc.id);
            // .zst 확장자 제거
            let filename = filename.strip_suffix(".txt").unwrap_or(filename);
            let md_path = dir.join(format!("{}.md", filename));

            // 백링크 삽입
            let relations = vector_db.find_related(&doc.id).unwrap_or_default();
            let backlinks = if relations.is_empty() {
                String::new()
            } else {
                let links: Vec<String> = relations
                    .iter()
                    .map(|r| format!("- [[{}]] ({})", r.target_id, r.relation_type))
                    .collect();
                format!("\n\n## 관련 문서\n\n{}\n", links.join("\n"))
            };

            // 마크다운 생성
            let md_content = format!(
                "---\n{}\n---\n\n{}{}\n",
                meta_section.trim(),
                body.trim(),
                backlinks
            );

            // 증분: 기존 파일과 동일하면 스킵
            if md_path.exists() {
                if let Ok(existing) = std::fs::read_to_string(&md_path) {
                    if existing == md_content {
                        report.exported += 1; // 변경 없지만 카운트에 포함
                        index_entries.push(IndexEntry {
                            title: filename.to_string(),
                            path: format!("{}/{}.md", type_dir, filename),
                            doc_types: doc.doc_types.clone(),
                            summary: extract_summary(&meta_section),
                        });
                        continue;
                    }
                }
            }

            std::fs::write(&md_path, &md_content)
                .context(format!("위키 파일 쓰기 실패: {:?}", md_path))?;

            index_entries.push(IndexEntry {
                title: filename.to_string(),
                path: format!("{}/{}.md", type_dir, filename),
                doc_types: doc.doc_types.clone(),
                summary: extract_summary(&meta_section),
            });

            report.exported += 1;
        }

        // INDEX.md 생성
        let index_content = generate_index(&index_entries);
        std::fs::write(output_dir.join("INDEX.md"), &index_content)?;

        // _graph.json 생성 (노드 + 엣지 + 2D 좌표)
        let graph = generate_graph(vector_db, &all_docs)?;
        std::fs::write(
            output_dir.join("_graph.json"),
            serde_json::to_string_pretty(&graph).unwrap_or_default(),
        )?;

        report.total = all_docs.len() as u64;
        Ok(report)
    }
}

#[derive(Debug, Clone, Default)]
pub struct ExportReport {
    pub total: u64,
    pub exported: u64,
    pub errors: Vec<String>,
}

struct IndexEntry {
    title: String,
    path: String,
    doc_types: Vec<String>,
    summary: String,
}

/// === META === ... === CONTENT === 분리
fn split_meta_content(content: &str) -> (String, String) {
    if let Some(idx) = content.find("=== CONTENT ===") {
        let meta = &content[..idx];
        let body = &content[idx + "=== CONTENT ===".len()..];
        (meta.to_string(), body.to_string())
    } else {
        (String::new(), content.to_string())
    }
}

/// 메타데이터에서 summary 추출
fn extract_summary(meta: &str) -> String {
    for line in meta.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("summary") {
            return trimmed.split_once(':').map(|x| x.1)
                .unwrap_or("")
                .trim()
                .to_string();
        }
    }
    String::new()
}

/// INDEX.md 생성
fn generate_index(entries: &[IndexEntry]) -> String {
    let mut index = String::from("# 문서 인덱스\n\n");

    // 유형별 그룹화
    let mut by_type: std::collections::HashMap<String, Vec<&IndexEntry>> =
        std::collections::HashMap::new();

    for entry in entries {
        let primary = entry
            .doc_types
            .first()
            .map(|s| s.as_str())
            .unwrap_or("etc");
        by_type
            .entry(primary.to_string())
            .or_default()
            .push(entry);
    }

    let mut types: Vec<_> = by_type.keys().cloned().collect();
    types.sort();

    index.push_str(&format!("총 {} 문서\n\n", entries.len()));

    for t in &types {
        let docs = &by_type[t];
        index.push_str(&format!("## {} ({} 문서)\n\n", t, docs.len()));
        for doc in docs {
            index.push_str(&format!(
                "- [{}]({}) — {}\n",
                doc.title, doc.path, doc.summary
            ));
        }
        index.push('\n');
    }

    index
}

// ── 그래프 JSON 생성 ────────────────────────────────────────

#[derive(Serialize)]
struct GraphData {
    nodes: Vec<GraphNode>,
    edges: Vec<GraphEdge>,
    stats: GraphStats,
}

#[derive(Serialize)]
struct GraphNode {
    id: String,
    label: String,
    doc_types: Vec<String>,
    date: String,
    x: f64,
    y: f64,
}

#[derive(Serialize)]
struct GraphEdge {
    source: String,
    target: String,
    relation: String,
    /// Phase 83: 관계 origin (auto_similarity / user_wikilink / llm_extracted ...)
    origin: String,
    origin_label_ko: String,
}

#[derive(Serialize)]
struct GraphStats {
    total_nodes: usize,
    total_edges: usize,
}

fn generate_graph(
    vector_db: &dyn VectorDBPort,
    all_docs: &[StoredDocSummary],
) -> Result<GraphData> {
    let mut nodes = Vec::new();
    let mut edges = Vec::new();

    // 노드 생성 — 간이 2D 좌표 (doc_type 해시 + index로 분산)
    for (i, doc) in all_docs.iter().enumerate() {
        let primary_type = doc.doc_types.first().map(|s| s.as_str()).unwrap_or("etc");

        // 유형별 기본 x 좌표 (해시 기반)
        let type_hash = primary_type.bytes().fold(0u64, |a, b| a.wrapping_mul(31).wrapping_add(b as u64));
        let base_x = (type_hash % 800) as f64;
        let base_y = (type_hash.wrapping_mul(7919) % 600) as f64;

        // 같은 유형 내 분산
        let angle = (i as f64) * 2.4; // golden angle
        let radius = 30.0 + (i as f64 * 5.0) % 100.0;

        nodes.push(GraphNode {
            id: doc.id.clone(),
            label: doc.path.file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or(&doc.id)
                .to_string(),
            doc_types: doc.doc_types.clone(),
            date: doc.date.clone(),
            x: base_x + radius * angle.cos(),
            y: base_y + radius * angle.sin(),
        });

        // 엣지 생성
        if let Ok(relations) = vector_db.find_related(&doc.id) {
            for rel in &relations {
                let origin_key = serde_json::to_value(&rel.origin).ok()
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
                    .unwrap_or_else(|| "auto_similarity".into());
                edges.push(GraphEdge {
                    source: doc.id.clone(),
                    target: rel.target_id.clone(),
                    relation: rel.relation_type.to_string(),
                    origin: origin_key,
                    origin_label_ko: rel.origin.label_ko().to_string(),
                });
            }
        }
    }

    Ok(GraphData {
        stats: GraphStats {
            total_nodes: nodes.len(),
            total_edges: edges.len(),
        },
        nodes,
        edges,
    })
}

// ── KG 쿼리 엔진 ──────────────────────────────────────────────

// KG 쿼리 결과 타입(KgQueryResult/KgNode/KgEdge/KgStats)은 `fp-domain-types`로 추출됨
// (cycle 7 step-d2 — GraphDBPort 반환형이므로 포트와 동일 crate 필요). 쿼리 *엔진*
// (KgQueryEngine)은 VectorDBPort에 의존하는 도메인 로직이므로 core 잔류.
// 기존 `file_pipeline_core::domain::wiki_export::{KgQueryResult, ...}` 경로는 re-export로 유지.
pub use fp_domain_types::kg_types::{KgEdge, KgNode, KgQueryResult, KgStats};

/// KG 쿼리 엔진 — _graph.json 데이터를 쿼리 가능하게 확장
pub struct KgQueryEngine;

impl KgQueryEngine {
    /// 특정 문서의 1-hop 이웃 조회
    pub fn neighbors(
        vector_db: &dyn VectorDBPort,
        doc_id: &str,
    ) -> Result<KgQueryResult> {
        let relations = vector_db.find_related(doc_id)?;

        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut seen = std::collections::HashSet::new();

        // 루트 노드
        let root_types = vector_db.get_types(doc_id).unwrap_or_default();
        nodes.push(KgNode {
            id: doc_id.to_string(),
            doc_types: root_types,
            date: String::new(),
            relation_count: relations.len(),
        });
        seen.insert(doc_id.to_string());

        for rel in &relations {
            let origin_key = serde_json::to_value(&rel.origin).ok()
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| "auto_similarity".into());
            edges.push(KgEdge {
                source: rel.source_id.clone(),
                target: rel.target_id.clone(),
                relation: rel.relation_type.to_string(),
                origin: origin_key,
                origin_label_ko: rel.origin.label_ko().to_string(),
            });
            if seen.insert(rel.target_id.clone()) {
                let target_types = vector_db.get_types(&rel.target_id).unwrap_or_default();
                let target_rels = vector_db.find_related(&rel.target_id).unwrap_or_default();
                nodes.push(KgNode {
                    id: rel.target_id.clone(),
                    doc_types: target_types,
                    date: String::new(),
                    relation_count: target_rels.len(),
                });
            }
        }

        Ok(KgQueryResult { nodes, edges, paths: vec![] })
    }

    /// 2-hop 경로 탐색: source → ? → target
    pub fn find_paths(
        vector_db: &dyn VectorDBPort,
        source_id: &str,
        target_id: &str,
    ) -> Result<KgQueryResult> {
        let source_rels = vector_db.find_related(source_id)?;
        let mut paths = Vec::new();
        let mut all_edges = Vec::new();
        let mut all_node_ids = std::collections::HashSet::new();

        all_node_ids.insert(source_id.to_string());
        all_node_ids.insert(target_id.to_string());

        // 직접 연결
        for rel in &source_rels {
            let origin_key = serde_json::to_value(&rel.origin).ok()
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .unwrap_or_else(|| "auto_similarity".into());
            all_edges.push(KgEdge {
                source: source_id.to_string(),
                target: rel.target_id.clone(),
                relation: rel.relation_type.to_string(),
                origin: origin_key,
                origin_label_ko: rel.origin.label_ko().to_string(),
            });
            if rel.target_id == target_id {
                paths.push(vec![source_id.to_string(), target_id.to_string()]);
            }
        }

        // 2-hop
        if paths.is_empty() {
            for rel in &source_rels {
                all_node_ids.insert(rel.target_id.clone());
                let mid_rels = vector_db.find_related(&rel.target_id).unwrap_or_default();
                for mid_rel in &mid_rels {
                    let origin_key = serde_json::to_value(&mid_rel.origin).ok()
                        .and_then(|v| v.as_str().map(|s| s.to_string()))
                        .unwrap_or_else(|| "auto_similarity".into());
                    all_edges.push(KgEdge {
                        source: rel.target_id.clone(),
                        target: mid_rel.target_id.clone(),
                        relation: mid_rel.relation_type.to_string(),
                        origin: origin_key,
                        origin_label_ko: mid_rel.origin.label_ko().to_string(),
                    });
                    if mid_rel.target_id == target_id {
                        paths.push(vec![
                            source_id.to_string(),
                            rel.target_id.clone(),
                            target_id.to_string(),
                        ]);
                        all_node_ids.insert(rel.target_id.clone());
                    }
                }
            }
        }

        let nodes = all_node_ids.into_iter().map(|id| {
            let types = vector_db.get_types(&id).unwrap_or_default();
            let rels = vector_db.find_related(&id).unwrap_or_default();
            KgNode { id, doc_types: types, date: String::new(), relation_count: rels.len() }
        }).collect();

        Ok(KgQueryResult { nodes, edges: all_edges, paths })
    }

    /// 전체 그래프 통계
    pub fn stats(vector_db: &dyn VectorDBPort) -> Result<KgStats> {
        let all = vector_db.list_all()?;
        let mut total_edges = 0usize;
        let mut isolated = 0usize;
        let mut hub_id = String::new();
        let mut max_rels = 0usize;

        for doc in &all {
            let rels = vector_db.find_related(&doc.id).unwrap_or_default();
            total_edges += rels.len();
            if rels.is_empty() {
                isolated += 1;
            }
            if rels.len() > max_rels {
                max_rels = rels.len();
                hub_id = doc.id.clone();
            }
        }

        Ok(KgStats {
            total_nodes: all.len(),
            total_edges: total_edges / 2, // 양방향이므로 2로 나눔
            isolated_nodes: isolated,
            hub_node: hub_id,
            hub_degree: max_rels,
        })
    }
}

// KgStats 정의는 fp-domain-types::kg_types 로 이관됨 (상단 re-export 참조).

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_meta_content() {
        let content = "=== META ===\nsource: test\n=== CONTENT ===\n\nbody text";
        let (meta, body) = split_meta_content(content);
        assert!(meta.contains("source: test"));
        assert!(body.contains("body text"));
    }

    #[test]
    fn test_split_meta_content_no_marker() {
        let content = "plain text without marker";
        let (meta, body) = split_meta_content(content);
        assert!(meta.is_empty());
        assert_eq!(body, content);
    }

    #[test]
    fn test_extract_summary() {
        let meta = "source: test.txt\ntype: meeting\nsummary: 핵심 요약 내용";
        assert_eq!(extract_summary(meta), "핵심 요약 내용");
    }

    #[test]
    fn test_extract_summary_missing() {
        let meta = "source: test.txt\ntype: meeting";
        assert_eq!(extract_summary(meta), "");
    }

    #[test]
    fn test_generate_index() {
        let entries = vec![
            IndexEntry {
                title: "회의록_0401".into(),
                path: "meeting/회의록_0401.md".into(),
                doc_types: vec!["meeting".into()],
                summary: "4월 1일 회의".into(),
            },
            IndexEntry {
                title: "학습_rust".into(),
                path: "study/학습_rust.md".into(),
                doc_types: vec!["study".into()],
                summary: "Rust 학습".into(),
            },
        ];
        let index = generate_index(&entries);
        assert!(index.contains("총 2 문서"));
        assert!(index.contains("meeting"));
        assert!(index.contains("study"));
        assert!(index.contains("[회의록_0401]"));
    }
}
