use crate::domain::models::SimilarDoc;

/// Maximal Marginal Relevance -- 검색 결과 다양성 보장
pub fn mmr_rerank(results: Vec<SimilarDoc>, lambda: f32, top_k: usize) -> Vec<SimilarDoc> {
    if results.len() <= 1 || top_k == 0 {
        return results;
    }

    let mut selected: Vec<SimilarDoc> = vec![];
    let mut remaining: Vec<SimilarDoc> = results;

    // 첫 번째: 가장 높은 점수
    remaining.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    selected.push(remaining.remove(0));

    while selected.len() < top_k && !remaining.is_empty() {
        let mut best_idx = 0;
        let mut best_mmr = f32::MIN;

        for (i, candidate) in remaining.iter().enumerate() {
            let relevance = candidate.score;
            // 다양성: 이미 선택된 문서들과의 최대 유사도
            let max_sim = selected
                .iter()
                .map(|s| doc_type_similarity(candidate, s))
                .fold(0.0f32, f32::max);
            let mmr_score = lambda * relevance - (1.0 - lambda) * max_sim;
            if mmr_score > best_mmr {
                best_mmr = mmr_score;
                best_idx = i;
            }
        }
        selected.push(remaining.remove(best_idx));
    }
    selected
}

/// 문서 유형 유사도 (같은 유형이면 1.0, 다르면 0.0)
fn doc_type_similarity(a: &SimilarDoc, b: &SimilarDoc) -> f32 {
    let overlap = a
        .doc_types
        .iter()
        .filter(|t| b.doc_types.contains(t))
        .count();
    if a.doc_types.is_empty() && b.doc_types.is_empty() {
        return 0.0;
    }
    overlap as f32 / a.doc_types.len().max(b.doc_types.len()).max(1) as f32
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_doc(id: &str, score: f32, doc_types: Vec<&str>) -> SimilarDoc {
        SimilarDoc {
            id: id.to_string(),
            path: PathBuf::from(format!("/tmp/{}.txt", id)),
            score,
            doc_types: doc_types.into_iter().map(String::from).collect(),
            hierarchy: vec![],
            date: "2026-01-01".to_string(),
        }
    }

    #[test]
    fn test_mmr_empty() {
        let result = mmr_rerank(vec![], 0.7, 5);
        assert!(result.is_empty());
    }

    #[test]
    fn test_mmr_single() {
        let docs = vec![make_doc("a", 0.9, vec!["meeting"])];
        let result = mmr_rerank(docs.clone(), 0.7, 5);
        assert_eq!(result.len(), 1);
    }

    #[test]
    fn test_mmr_diversity() {
        let docs = vec![
            make_doc("a", 0.95, vec!["meeting"]),
            make_doc("b", 0.90, vec!["meeting"]),
            make_doc("c", 0.85, vec!["report"]),
        ];
        let result = mmr_rerank(docs, 0.5, 3);
        assert_eq!(result.len(), 3);
        // 첫 번째는 가장 높은 점수
        assert_eq!(result[0].id, "a");
        // 두 번째는 다른 유형이 우선될 수 있음 (lambda=0.5로 다양성 중시)
        assert_eq!(result[1].id, "c");
    }

    #[test]
    fn test_mmr_top_k_limit() {
        let docs = vec![
            make_doc("a", 0.9, vec!["meeting"]),
            make_doc("b", 0.8, vec!["report"]),
            make_doc("c", 0.7, vec!["log"]),
        ];
        let result = mmr_rerank(docs, 0.7, 2);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_doc_type_similarity() {
        let a = make_doc("a", 0.9, vec!["meeting", "report"]);
        let b = make_doc("b", 0.8, vec!["meeting"]);
        let sim = doc_type_similarity(&a, &b);
        assert!(sim > 0.0);

        let c = make_doc("c", 0.7, vec!["log"]);
        let sim2 = doc_type_similarity(&a, &c);
        assert!((sim2 - 0.0).abs() < 0.001);
    }
}
