//! 토픽 분할 전략 고도화
//!
//! Phase 1: 유형별 디렉토리 + 시간 분할 + 크기 트리거 + 2단계 요약
//! Phase 2: 임베딩 클러스터링 + LLM 토픽 라벨 + 커버리지 태그
//! Phase 3: 구조화 프롬프트 (모순 해결, 타임라인, 출처 표시)
//! Phase 4: 버전 관리 (.bak)

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;

use crate::domain::deduplicator::cosine_similarity;
use crate::ports::output::{EmbeddingPort, LLMPort, StoragePort, VectorDBPort};

pub const AUTO_MERGE_THRESHOLD: usize = 5;
const MAX_CLUSTER_SIZE: usize = 20;

// ── 결과 구조체 ─────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct TopicMergeReport {
    pub topics_created: u64,
    pub topics_revised: u64,
    pub documents_merged: u64,
    pub clusters_found: u64,
    pub errors: Vec<String>,
}

#[derive(Debug, Clone)]
struct DocSummary {
    id: String,
    date: String,
    content: String,
    embedding: Vec<f32>,
}

// ── TopicMerger ─────────────────────────────────────────────

pub struct TopicMerger;

impl TopicMerger {
    /// 전체 문서를 유형별 → 클러스터별 → 시간별로 분할하여 토픽 페이지 생성
    pub async fn merge_all(
        vector_db: &dyn VectorDBPort,
        storage: &dyn StoragePort,
        llm: &dyn LLMPort,
        embedding: &dyn EmbeddingPort,
        topics_dir: &Path,
    ) -> Result<TopicMergeReport> {
        let mut report = TopicMergeReport::default();
        std::fs::create_dir_all(topics_dir)?;

        let all_docs = vector_db.list_all()?;

        // 유형별 그룹화
        let mut by_type: HashMap<String, Vec<_>> = HashMap::new();
        for doc in &all_docs {
            let primary = doc.doc_types.first().cloned().unwrap_or_else(|| "etc".into());
            by_type.entry(primary).or_default().push(doc);
        }

        for (doc_type, docs) in &by_type {
            if docs.len() < 2 {
                continue;
            }

            let type_dir = topics_dir.join(doc_type);
            std::fs::create_dir_all(&type_dir)?;

            // 1단계: 각 문서 요약 + 임베딩 수집
            let mut summaries = Vec::new();
            for doc in docs {
                match collect_doc_summary(doc, storage, llm, embedding).await {
                    Ok(s) => summaries.push(s),
                    Err(e) => report.errors.push(format!("{}: {}", doc.id, e)),
                }
            }

            if summaries.len() < 2 {
                continue;
            }

            // 2단계: 임베딩 클러스터링
            let clusters = cluster_by_embedding(&summaries, MAX_CLUSTER_SIZE);
            report.clusters_found += clusters.len() as u64;

            // 3단계: 클러스터별 토픽 페이지 생성
            let mut topic_links = Vec::new();

            for (ci, cluster_indices) in clusters.iter().enumerate() {
                let cluster_docs: Vec<&DocSummary> = cluster_indices
                    .iter()
                    .map(|&i| &summaries[i])
                    .collect();

                // LLM 토픽 라벨링
                let label = generate_topic_label(llm, &cluster_docs).await
                    .unwrap_or_else(|_| format!("토픽_{}", ci + 1));

                // 시간 분할
                let by_quarter = group_by_quarter(&cluster_docs);

                if by_quarter.len() <= 1 {
                    // 단일 기간: 하나의 파일로
                    let content = generate_topic_content(llm, doc_type, &label, &cluster_docs, vector_db).await?;
                    let filename = sanitize_filename(&label);
                    let file_path = type_dir.join(format!("{}.md", filename));
                    std::fs::write(&file_path, &content)?;
                    topic_links.push((label.clone(), format!("{}.md", filename), cluster_docs.len()));
                    report.topics_created += 1;
                    report.documents_merged += cluster_docs.len() as u64;
                } else {
                    // 여러 기간: 분기별 파일 + 종합
                    let cluster_dir = type_dir.join(sanitize_filename(&label));
                    std::fs::create_dir_all(&cluster_dir)?;

                    let mut quarter_links = Vec::new();
                    for (quarter, q_docs) in &by_quarter {
                        let content = generate_topic_content(llm, doc_type, &format!("{} ({})", label, quarter), q_docs, vector_db).await?;
                        let file_path = cluster_dir.join(format!("{}.md", quarter));
                        std::fs::write(&file_path, &content)?;
                        quarter_links.push((quarter.clone(), q_docs.len()));
                        report.documents_merged += q_docs.len() as u64;
                    }

                    // 클러스터 종합 인덱스
                    let index = generate_cluster_index(&label, &quarter_links);
                    std::fs::write(cluster_dir.join("종합.md"), &index)?;
                    topic_links.push((label.clone(), format!("{}/종합.md", sanitize_filename(&label)), cluster_docs.len()));
                    report.topics_created += 1;
                }
            }

            // 유형 종합 인덱스
            let type_index = generate_type_index(doc_type, &topic_links);
            std::fs::write(type_dir.join("종합.md"), &type_index)?;
        }

        Ok(report)
    }

    /// watch 중 자동 병합
    pub async fn auto_merge_if_needed(
        vector_db: &dyn VectorDBPort,
        storage: &dyn StoragePort,
        llm: &dyn LLMPort,
        embedding: &dyn EmbeddingPort,
        topics_dir: &Path,
        threshold: usize,
    ) -> Result<TopicMergeReport> {
        let all_docs = vector_db.list_all()?;
        let mut by_type: HashMap<String, usize> = HashMap::new();
        for doc in &all_docs {
            let primary = doc.doc_types.first().cloned().unwrap_or_else(|| "etc".into());
            *by_type.entry(primary).or_default() += 1;
        }

        // threshold 이상인 유형이 있는지 확인
        let needs_merge = by_type.values().any(|&count| count >= threshold);
        if !needs_merge {
            return Ok(TopicMergeReport::default());
        }

        Self::merge_all(vector_db, storage, llm, embedding, topics_dir).await
    }

    /// 사용자 피드백으로 토픽 페이지 수정 (버전 관리 포함)
    /// 사용자 피드백으로 토픽 수정 (이력 누적, 반복 가능)
    pub async fn revise_topic(
        topic_file: &Path,
        feedback: &str,
        llm: &dyn LLMPort,
    ) -> Result<String> {
        let current = std::fs::read_to_string(topic_file)?;

        // .bak 버전 관리 (최대 3개)
        rotate_backups(topic_file);
        let bak_path = topic_file.with_extension("md.bak");
        std::fs::write(&bak_path, &current)?;

        // 수정 이력 추출 (기존 문서에서 <!-- revision-history --> 파싱)
        let mut revision_history = Vec::new();
        for line in current.lines() {
            if line.starts_with("<!-- revision:") && line.ends_with("-->") {
                let entry = line
                    .trim_start_matches("<!-- revision:")
                    .trim_end_matches("-->")
                    .trim();
                revision_history.push(entry.to_string());
            }
        }

        // 이전 피드백을 프롬프트에 포함
        let history_section = if revision_history.is_empty() {
            String::new()
        } else {
            format!(
                "\n## 이전 수정 이력\n{}\n",
                revision_history
                    .iter()
                    .enumerate()
                    .map(|(i, h)| format!("{}. {}", i + 1, h))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        };

        let prompt = format!(
            "아래 토픽 문서를 사용자 피드백에 따라 수정하세요.\n\
             기존 구조(타임라인/결정사항/미해결/모순)는 유지하세요.\n\
             수정된 전체 문서만 출력하세요.\n\n\
             ## 이번 피드백\n{}\n{}\n## 현재 문서\n{}",
            feedback, history_section, current
        );

        let mut revised = llm.summarize_text(&prompt, "").await?;

        // 수정 이력 메타데이터 추가
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();
        let revision_meta = format!("<!-- revision: {} | {} -->\n", timestamp, feedback.chars().take(100).collect::<String>());
        revised = format!("{}{}", revision_meta, revised);

        std::fs::write(topic_file, &revised)?;
        tracing::info!("토픽 수정: {:?} (.bak 보존, 이력 #{} 추가)", topic_file, revision_history.len() + 1);
        Ok(revised)
    }
}

// ── 내부 함수: 문서 수집 + 2단계 요약 ──────────────────────

async fn collect_doc_summary(
    doc: &crate::domain::models::StoredDocSummary,
    storage: &dyn StoragePort,
    llm: &dyn LLMPort,
    embedding: &dyn EmbeddingPort,
) -> Result<DocSummary> {
    let temp = storage.decompress_temp(&doc.path)?;
    let full_content = std::fs::read_to_string(&temp)?;
    let _ = std::fs::remove_file(&temp);

    // 2단계 요약: 긴 문서는 LLM으로 200자 요약
    let summary = if full_content.chars().count() > 500 {
        let truncated: String = full_content.chars().take(2000).collect();
        llm.summarize_text(
            &format!("이 문서를 200자 이내로 핵심만 요약하세요:\n\n{}", truncated),
            "",
        ).await.unwrap_or_else(|_| full_content.chars().take(500).collect())
    } else {
        full_content.clone()
    };

    let emb = embedding.embed(&summary).await.unwrap_or_else(|_| vec![0.0; embedding.dim()]);

    Ok(DocSummary {
        id: doc.id.clone(),
        date: doc.date.clone(),
        content: summary,
        embedding: emb,
    })
}

// ── 내부 함수: 임베딩 클러스터링 ────────────────────────────

fn cluster_by_embedding(docs: &[DocSummary], max_size: usize) -> Vec<Vec<usize>> {
    if docs.len() <= max_size {
        return vec![(0..docs.len()).collect()];
    }

    // 간이 agglomerative clustering
    let n = docs.len();
    let mut assignments: Vec<usize> = (0..n).collect(); // 각 문서의 클러스터 ID
    let mut cluster_count = n;

    // 유사도 행렬에서 가장 가까운 쌍을 병합
    loop {
        if cluster_count <= (n / max_size).max(2) {
            break;
        }

        let mut best_sim = -1.0f32;
        let mut best_pair = (0, 0);

        for i in 0..n {
            for j in (i + 1)..n {
                if assignments[i] == assignments[j] {
                    continue;
                }
                let sim = cosine_similarity(&docs[i].embedding, &docs[j].embedding);
                if sim > best_sim {
                    best_sim = sim;
                    best_pair = (i, j);
                }
            }
        }

        if best_sim < 0.3 {
            break; // 너무 다른 문서는 병합 안 함
        }

        // 병합: j의 클러스터를 i의 클러스터로
        let target = assignments[best_pair.0];
        let source = assignments[best_pair.1];

        // 병합 후 크기 체크
        let merged_size = assignments.iter().filter(|&&a| a == target || a == source).count();
        if merged_size > max_size {
            break;
        }

        for a in assignments.iter_mut() {
            if *a == source {
                *a = target;
            }
        }
        cluster_count -= 1;
    }

    // 클러스터별 인덱스 그룹화
    let mut clusters: HashMap<usize, Vec<usize>> = HashMap::new();
    for (i, &cluster_id) in assignments.iter().enumerate() {
        clusters.entry(cluster_id).or_default().push(i);
    }

    clusters.into_values().collect()
}

// ── 내부 함수: 시간 분할 ────────────────────────────────────

fn group_by_quarter<'a>(docs: &[&'a DocSummary]) -> Vec<(String, Vec<&'a DocSummary>)> {
    let mut by_q: HashMap<String, Vec<&'a DocSummary>> = HashMap::new();

    for doc in docs {
        let quarter = date_to_quarter(&doc.date);
        by_q.entry(quarter).or_default().push(doc);
    }

    let mut sorted: Vec<_> = by_q.into_iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    sorted
}

fn date_to_quarter(date: &str) -> String {
    // "2026-04-05" → "2026-Q2"
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() >= 2 {
        let year = parts[0];
        let month: u32 = parts[1].parse().unwrap_or(1);
        let q = match month {
            1..=3 => "Q1",
            4..=6 => "Q2",
            7..=9 => "Q3",
            _ => "Q4",
        };
        format!("{}-{}", year, q)
    } else {
        "unknown".to_string()
    }
}

// ── 내부 함수: LLM 토픽 라벨링 ─────────────────────────────

async fn generate_topic_label(llm: &dyn LLMPort, docs: &[&DocSummary]) -> Result<String> {
    let previews: String = docs
        .iter()
        .take(5)
        .map(|d| d.content.chars().take(100).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n");

    let prompt = format!(
        "아래 문서들의 공통 주제를 한국어 10자 이내로 요약하세요. 주제명만 출력.\n\n{}",
        previews
    );

    let label = llm.summarize_text(&prompt, "").await?;
    let cleaned = label.trim().lines().next().unwrap_or("기타").trim().to_string();
    Ok(if cleaned.is_empty() { "기타".into() } else { cleaned })
}

// ── 내부 함수: 토픽 페이지 생성 (구조화 프롬프트) ───────────

async fn generate_topic_content(
    llm: &dyn LLMPort,
    doc_type: &str,
    title: &str,
    docs: &[&DocSummary],
    _vector_db: &dyn VectorDBPort,
) -> Result<String> {
    let merged_input = docs
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let date_info = if d.date.is_empty() { String::new() } else { format!(" ({})", d.date) };
            format!("--- 문서 {}{} ---\n{}", i + 1, date_info, d.content)
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    // 구조화 프롬프트 (Phase 3)
    let prompt = format!(
        "{} 유형 '{}' 토픽, 문서 {}개를 종합하세요.\n\n\
         ## 지시사항\n\
         1. 시간순 정렬 (날짜 명시)\n\
         2. 중복 제거\n\
         3. 모순 발견 시: \"⚠️ 모순: 문서N에서는 X, 문서M에서는 Y\"\n\
         4. 각 항목 뒤 출처: (문서N)\n\
         5. 섹션 분리: 타임라인 / 핵심 결정사항 / 미해결 사항 / 모순\n\n\
         ## 출력 형식\n\
         ### 타임라인\n- YYYY-MM-DD: 내용 (문서N)\n\n\
         ### 핵심 결정사항\n- ...\n\n\
         ### 미해결 사항\n- ...\n\n\
         ### 모순/불일치\n- ⚠️ ...\n\n\
         ## 문서\n\n{}",
        doc_type, title, docs.len(), merged_input
    );

    let content = llm.summarize_text(&prompt, "").await?;

    // 커버리지 태그
    let coverage_tag = format!("[coverage: {} -- {} sources]",
        if docs.len() >= 10 { "high" } else if docs.len() >= 3 { "medium" } else { "low" },
        docs.len()
    );

    Ok(format!(
        "# {}\n\n{}\n\n> {} 문서 종합\n\n{}\n\n## 출처\n\n{}\n",
        title,
        coverage_tag,
        docs.len(),
        content,
        docs.iter()
            .enumerate()
            .map(|(i, d)| format!("{}. {} ({})", i + 1, d.id, d.date))
            .collect::<Vec<_>>()
            .join("\n"),
    ))
}

// ── 내부 함수: 인덱스 생성 ──────────────────────────────────

fn generate_type_index(doc_type: &str, topic_links: &[(String, String, usize)]) -> String {
    let total_docs: usize = topic_links.iter().map(|(_, _, n)| n).sum();
    let mut md = format!(
        "# {} 종합\n\n> {} 토픽, {} 문서\n\n## 토픽 목록\n\n",
        doc_type,
        topic_links.len(),
        total_docs
    );
    for (label, path, count) in topic_links {
        md.push_str(&format!("- [[{}]] — {} ({} 문서)\n", path, label, count));
    }
    md
}

fn generate_cluster_index(label: &str, quarter_links: &[(String, usize)]) -> String {
    let total: usize = quarter_links.iter().map(|(_, n)| n).sum();
    let mut md = format!(
        "# {}\n\n> {} 문서, {} 기간\n\n## 기간별\n\n",
        label, total, quarter_links.len()
    );
    for (quarter, count) in quarter_links {
        md.push_str(&format!("- [[{}.md]] — {} ({} 문서)\n", quarter, quarter, count));
    }
    md
}

// ── 내부 함수: 유틸리티 ─────────────────────────────────────

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

fn rotate_backups(path: &Path) {
    let bak2 = path.with_extension("md.bak.2");
    let bak1 = path.with_extension("md.bak.1");
    let bak = path.with_extension("md.bak");

    let _ = std::fs::remove_file(&bak2);
    if bak1.exists() { let _ = std::fs::rename(&bak1, &bak2); }
    if bak.exists() { let _ = std::fs::rename(&bak, &bak1); }
}

#[cfg(test)]
#[path = "topic_merger_tests.rs"]
mod topic_merger_tests;
