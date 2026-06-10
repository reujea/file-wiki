use std::collections::HashMap;

use regex::Regex;
use serde::{Deserialize, Serialize};

use super::models::{VerificationLevel, VerificationResult};

/// 검증 기준 임계값
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct VerificationThresholds {
    pub structure_min: f64,
    pub compression_min: f64,
    pub compression_max: f64,
    pub keyword_coverage_min: f64,
    /// 원본 핵심 키워드가 가공본에 보존된 비율 (Phase 2: 양방향)
    pub keyword_completeness_min: f64,
    pub rouge_l_min: f64,
    pub entity_preservation_min: f64,
}

impl Default for VerificationThresholds {
    fn default() -> Self {
        Self {
            structure_min: 0.5,
            compression_min: 0.05,
            compression_max: 1.5,
            keyword_coverage_min: 0.5,
            keyword_completeness_min: 0.3,
            rouge_l_min: 0.10,
            entity_preservation_min: 0.5,
        }
    }
}

impl VerificationThresholds {
    pub fn strict() -> Self {
        Self {
            structure_min: 0.9,
            compression_min: 0.10,
            compression_max: 0.70,
            keyword_coverage_min: 0.85,
            keyword_completeness_min: 0.5,
            rouge_l_min: 0.25,
            entity_preservation_min: 0.80,
        }
    }
}

// ── ROUGE-L ─────────────────────────────────────────────────

pub fn rouge_l_recall(reference: &str, hypothesis: &str) -> f64 {
    let ref_tokens: Vec<&str> = reference.split_whitespace().collect();
    let hyp_tokens: Vec<&str> = hypothesis.split_whitespace().collect();
    if ref_tokens.is_empty() {
        return 0.0;
    }
    let m = ref_tokens.len();
    let n = hyp_tokens.len();
    let mut prev = vec![0usize; n + 1];
    let mut curr = vec![0usize; n + 1];
    for i in 1..=m {
        for j in 1..=n {
            if ref_tokens[i - 1] == hyp_tokens[j - 1] {
                curr[j] = prev[j - 1] + 1;
            } else {
                curr[j] = curr[j - 1].max(prev[j]);
            }
        }
        std::mem::swap(&mut prev, &mut curr);
        curr.fill(0);
    }
    prev[n] as f64 / m as f64
}

// ── 구조 완전성 ─────────────────────────────────────────────

/// 텍스트 기반 구조 검사 (기존 — fallback)
pub fn check_structure(processed: &str, required_sections: &[String]) -> f64 {
    if required_sections.is_empty() {
        return 1.0;
    }
    let found = required_sections
        .iter()
        .filter(|section| processed.contains(section.as_str()))
        .count();
    found as f64 / required_sections.len() as f64
}

/// sections HashMap 기반 구조 검사 (Phase 1: A-1 해결)
pub fn check_structure_from_sections(
    sections: &HashMap<String, Vec<String>>,
    required_sections: &[String],
) -> f64 {
    if required_sections.is_empty() {
        return 1.0;
    }
    let found = required_sections
        .iter()
        .filter(|req| sections.contains_key(req.as_str()))
        .count();
    found as f64 / required_sections.len() as f64
}

// ── 압축률 ──────────────────────────────────────────────────

pub fn check_compression_ratio(original: &str, processed: &str) -> f64 {
    if original.is_empty() {
        return 0.0;
    }
    processed.len() as f64 / original.len() as f64
}

// ── 키워드 커버리지 (환각 탐지: LLM keywords → 원본) ────────

pub fn check_keyword_coverage(original: &str, keywords: &[String]) -> f64 {
    if keywords.is_empty() {
        return 1.0;
    }
    let original_lower = original.to_lowercase();
    let found = keywords
        .iter()
        .filter(|kw| original_lower.contains(&kw.to_lowercase()))
        .count();
    found as f64 / keywords.len() as f64
}

// ── 키워드 완전성 (누락 탐지: 원본 핵심 → 가공본) ──────────

/// 한국어 불용어
const STOPWORDS: &[&str] = &[
    "은", "는", "이", "가", "의", "를", "에", "서", "도", "로", "와", "한", "다",
    "것", "수", "등", "및", "더", "또", "그", "이런", "저런", "하는", "있는",
    "the", "a", "an", "is", "are", "was", "were", "in", "on", "at", "to", "for",
    "of", "and", "or", "but", "with", "from", "by", "as", "this", "that",
];

/// 원본의 핵심 키워드가 가공본에 보존되었는지 (Phase 2: A-2 해결)
pub fn check_original_keyword_presence(original: &str, processed: &str) -> f64 {
    let processed_lower = processed.to_lowercase();

    // 원본에서 빈도 상위 키워드 추출
    let mut freq: HashMap<String, usize> = HashMap::new();
    for word in original.split_whitespace() {
        let w = word
            .trim_matches(|c: char| !c.is_alphanumeric())
            .to_lowercase();
        if w.chars().count() < 2 {
            continue;
        }
        if STOPWORDS.contains(&w.as_str()) {
            continue;
        }
        *freq.entry(w).or_default() += 1;
    }

    if freq.is_empty() {
        return 1.0;
    }

    // 빈도 상위 15개
    let mut words: Vec<(String, usize)> = freq.into_iter().collect();
    words.sort_by(|a, b| b.1.cmp(&a.1));
    let top_keywords: Vec<&str> = words.iter().take(15).map(|(w, _)| w.as_str()).collect();

    let found = top_keywords
        .iter()
        .filter(|kw| processed_lower.contains(**kw))
        .count();

    found as f64 / top_keywords.len() as f64
}

// ── 개체 보존 (Phase 3: B-3 확장) ──────────────────────────

pub fn check_entity_preservation(original: &str, processed: &str) -> f64 {
    // 여러 패턴으로 개체 추출
    let patterns = [
        r"\d{4}[-./]\d{1,2}[-./]\d{1,2}",          // 날짜: 2026-04-05
        r"\d{1,3}(,\d{3})+(만원|억|원|달러)?",       // 금액: 1,234,567원
        r"\d{4,}",                                   // 4자리+ 숫자
        r"[a-zA-Z0-9._%+-]+@[a-zA-Z0-9.-]+\.[a-zA-Z]{2,}", // 이메일
        r"https?://[^\s]+",                          // URL
    ];

    let mut all_entities: Vec<String> = Vec::new();
    for pattern in &patterns {
        if let Ok(re) = Regex::new(pattern) {
            for m in re.find_iter(processed) {
                let entity = m.as_str().to_string();
                if !all_entities.contains(&entity) {
                    all_entities.push(entity);
                }
            }
        }
    }

    if all_entities.is_empty() {
        return 1.0;
    }

    let found = all_entities
        .iter()
        .filter(|e| original.contains(e.as_str()))
        .count();

    found as f64 / all_entities.len() as f64
}

// ── 강한 주장 약화 검증 (Phase 87, wikidocs 353407) ───────────────────────────

/// 가공본에서 "확실히/반드시/항상/모든/100%" 같은 단정 표현을 추출.
///
/// 원천 자료에 같은 강도의 근거가 있는지는 본 함수가 판단할 수 없으므로,
/// **약화가 필요할 수 있는 후보**를 반환하여 lint/Verify 단계에서 사용자 검토에 노출.
///
/// 빈 Vec 반환 = 강한 주장 없음 (또는 모두 안전한 맥락에 있음).
pub fn detect_strong_claims(processed: &str) -> Vec<String> {
    // 단정 표현 패턴 — 문장 단위로 추출
    let strong_markers = [
        "확실히", "반드시", "항상", "모든", "전부", "절대", "결코",
        "100%", "완벽히", "완전히", "무조건", "유일",
        // 영문도 흔하므로 일부 포함 (혼합 문서 대응)
        "always", "never", "definitely", "absolutely", "certainly",
    ];
    let mut hits: Vec<String> = Vec::new();
    // 문장 분리: 마침표/물음표/느낌표 + 공백 또는 줄바꿈
    for raw_sentence in processed.split(['.', '?', '!', '。', '？', '！', '\n']) {
        let s = raw_sentence.trim();
        if s.is_empty() { continue; }
        let s_lower = s.to_lowercase();
        let has_marker = strong_markers.iter().any(|m| {
            // 한글 마커는 그대로, 영문은 word boundary 단순 체크
            s_lower.contains(*m)
        });
        if has_marker {
            // 200자 이상 문장은 truncate (긴 단락 줄임)
            let snippet = if s.chars().count() > 200 {
                let trimmed: String = s.chars().take(200).collect();
                format!("{}…", trimmed)
            } else {
                s.to_string()
            };
            if !hits.contains(&snippet) {
                hits.push(snippet);
            }
        }
    }
    hits
}

// ── 통합 검증 ───────────────────────────────────────────────

/// 기본 임계값으로 검증 (후방 호환)
pub fn verify_all(
    original: &str,
    processed: &str,
    required_sections: &[String],
    keywords: &[String],
    sections: Option<&HashMap<String, Vec<String>>>,
) -> VerificationResult {
    verify_with_thresholds(
        original,
        processed,
        required_sections,
        keywords,
        sections,
        &VerificationThresholds::default(),
    )
}

/// 커스텀 임계값 + sections 기반 검증
pub fn verify_with_thresholds(
    original: &str,
    processed: &str,
    required_sections: &[String],
    keywords: &[String],
    sections: Option<&HashMap<String, Vec<String>>>,
    thresholds: &VerificationThresholds,
) -> VerificationResult {
    // 구조 검증: sections가 있으면 JSON 키 기반, 없으면 contains 기반
    let structure = match sections {
        Some(s) => check_structure_from_sections(s, required_sections),
        None => check_structure(processed, required_sections),
    };

    let compression = check_compression_ratio(original, processed);
    let keyword_cov = check_keyword_coverage(original, keywords);
    let keyword_comp = check_original_keyword_presence(original, processed);
    let rouge_l = rouge_l_recall(original, processed);
    let entity = check_entity_preservation(original, processed);

    let mut details = Vec::new();
    let mut has_fail = false;
    let mut has_warning = false;

    if structure < thresholds.structure_min {
        has_fail = true;
        details.push(format!(
            "FAIL: 구조 완전성 {:.0}% (기준 {:.0}%)",
            structure * 100.0,
            thresholds.structure_min * 100.0
        ));
    }

    if compression < thresholds.compression_min || compression > thresholds.compression_max {
        has_warning = true;
        details.push(format!(
            "WARNING: 압축률 {:.0}% (적정 {:.0}~{:.0}%)",
            compression * 100.0,
            thresholds.compression_min * 100.0,
            thresholds.compression_max * 100.0
        ));
    }

    if keyword_cov < thresholds.keyword_coverage_min {
        has_fail = true;
        details.push(format!(
            "FAIL: 키워드 커버리지 {:.0}% (기준 {:.0}%)",
            keyword_cov * 100.0,
            thresholds.keyword_coverage_min * 100.0
        ));
    }

    if keyword_comp < thresholds.keyword_completeness_min {
        has_warning = true;
        details.push(format!(
            "WARNING: 키워드 완전성 {:.0}% (기준 {:.0}%)",
            keyword_comp * 100.0,
            thresholds.keyword_completeness_min * 100.0
        ));
    }

    if rouge_l < thresholds.rouge_l_min {
        has_fail = true;
        details.push(format!(
            "FAIL: ROUGE-L recall {:.0}% (기준 {:.0}%)",
            rouge_l * 100.0,
            thresholds.rouge_l_min * 100.0
        ));
    }

    if entity < thresholds.entity_preservation_min {
        has_warning = true;
        details.push(format!(
            "WARNING: 개체 보존 {:.0}% (기준 {:.0}%)",
            entity * 100.0,
            thresholds.entity_preservation_min * 100.0
        ));
    }

    let overall = if has_fail {
        VerificationLevel::Fail(details.join("; "))
    } else if has_warning {
        VerificationLevel::Warning(details.join("; "))
    } else {
        VerificationLevel::Pass
    };

    VerificationResult {
        structure_completeness: structure,
        compression_ratio: compression,
        keyword_coverage: keyword_cov,
        keyword_completeness: keyword_comp,
        rouge_l_recall: rouge_l,
        entity_preservation: entity,
        overall,
        details,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rouge_l() {
        let reference = "오늘 회의에서 프로젝트 일정을 논의했다";
        let hypothesis = "회의에서 프로젝트 일정을 다뤘다";
        let score = rouge_l_recall(reference, hypothesis);
        assert!(score > 0.0);
    }

    #[test]
    fn test_structure_contains() {
        let processed = "=== 결정사항 ===\n내용\n=== 액션아이템 ===\n항목\n=== 다음안건 ===\n안건";
        let sections: Vec<String> = vec!["결정사항", "액션아이템", "다음안건"]
            .into_iter()
            .map(String::from)
            .collect();
        assert!((check_structure(processed, &sections) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_structure_from_sections_json() {
        let mut sections = HashMap::new();
        sections.insert("결정사항".to_string(), vec!["항목1".to_string()]);
        sections.insert("액션아이템".to_string(), vec!["항목2".to_string()]);
        // "다음안건" 없음

        let required: Vec<String> = vec!["결정사항", "액션아이템", "다음안건"]
            .into_iter()
            .map(String::from)
            .collect();

        let score = check_structure_from_sections(&sections, &required);
        assert!((score - 2.0 / 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_structure_from_sections_full() {
        let mut sections = HashMap::new();
        sections.insert("결정사항".to_string(), vec!["항목1".to_string()]);
        sections.insert("액션아이템".to_string(), vec!["항목2".to_string()]);
        sections.insert("다음안건".to_string(), vec!["항목3".to_string()]);

        let required: Vec<String> = vec!["결정사항", "액션아이템", "다음안건"]
            .into_iter()
            .map(String::from)
            .collect();

        assert!((check_structure_from_sections(&sections, &required) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_original_keyword_presence() {
        let original = "NVIDIA H100 GPU 클러스터 3200대 배포 완료 보고서";
        let processed = "NVIDIA H100 클러스터 배포 완료";
        let score = check_original_keyword_presence(original, processed);
        assert!(score > 0.5, "핵심 키워드 보존: {:.2}", score);
    }

    #[test]
    fn test_original_keyword_missing() {
        let original = "NVIDIA H100 GPU 클러스터 3200대 배포 완료 보고서";
        let processed = "일반적인 내용만 있음";
        let score = check_original_keyword_presence(original, processed);
        assert!(score < 0.3, "키워드 누락 감지: {:.2}", score);
    }

    #[test]
    fn test_entity_preservation_date() {
        let original = "2026-04-05에 매출 1,234,567원 달성";
        let processed = "2026-04-05 매출 1,234,567원 기록";
        let score = check_entity_preservation(original, processed);
        assert!((score - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_entity_preservation_missing_date() {
        let original = "2026-04-05에 매출 1,234,567원 달성";
        // processed에도 개체가 있지만 원본과 다름 → 보존율 낮음
        let processed = "2025-01-01에 매출 999원 달성";
        let score = check_entity_preservation(original, processed);
        assert!(score < 0.5, "다른 개체는 보존 실패: {:.2}", score);
    }

    #[test]
    fn test_default_thresholds() {
        let t = VerificationThresholds::default();
        assert!(t.structure_min < 0.9);
        assert!(t.keyword_completeness_min > 0.0);
    }

    #[test]
    fn test_strict_thresholds() {
        let t = VerificationThresholds::strict();
        assert!((t.structure_min - 0.9).abs() < 1e-6);
        assert!((t.keyword_completeness_min - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_detect_strong_claims_korean() {
        let text = "이 방법은 확실히 빠릅니다. 다른 방법보다 항상 우월합니다. 일부 경우엔 다를 수 있습니다.";
        let hits = detect_strong_claims(text);
        assert_eq!(hits.len(), 2, "확실히/항상 두 문장 검출, 일부 경우는 제외: {:?}", hits);
    }

    #[test]
    fn test_detect_strong_claims_english() {
        let text = "This is always fast. Sometimes it could be slower. Never use this in production.";
        let hits = detect_strong_claims(text);
        assert_eq!(hits.len(), 2, "always / never 검출: {:?}", hits);
    }

    #[test]
    fn test_detect_strong_claims_none() {
        let text = "이 방법은 빠를 수 있습니다. 다른 방법이 더 나을 가능성이 있습니다.";
        let hits = detect_strong_claims(text);
        assert!(hits.is_empty(), "약한 표현만 있으면 검출 없음: {:?}", hits);
    }

    #[test]
    fn test_detect_strong_claims_dedup() {
        // 같은 문장이 본문에 여러 번 등장해도 dedup
        let text = "반드시 확인하세요. 반드시 확인하세요.";
        let hits = detect_strong_claims(text);
        assert_eq!(hits.len(), 1, "동일 문장은 dedup");
    }
}
