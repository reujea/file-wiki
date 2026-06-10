//! Phase 78: setup_dryrun + 사용 패턴 자동 프로파일링
//!
//! - dryrun: 두 PipelineConfig 차이를 단계별로 분석 (실제 실행 없이 영향 미리보기)
//! - profile_inference: stats + 처리된 doc_type 분포 + 검색 모드 분포에서 SetupProfile 추정
//!
//! 실제 verify+embed dry-run (전문가 답변 §4.4)은 비용이 커서 단기안에서 제외.
//! 핵심 가치: "이 추천을 적용하면 어떤 단계의 어떤 파라미터가 바뀌는가?"를 시각화.

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::config::PipelineConfig;
use crate::setup_review::{ContentType, SetupProfile, Sensitivity, Volume, SearchIntent, Collaboration};

/// 두 config 간 차이 항목
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigDiff {
    pub path: String,
    pub before: serde_json::Value,
    pub after: serde_json::Value,
    /// 영향 받는 파이프라인 단계 (가공/검색/저장 등)
    pub stage: String,
    /// 추정 영향도 설명
    pub impact: String,
}

/// dryrun 결과
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DryRunReport {
    pub diffs: Vec<ConfigDiff>,
    /// 단계별 영향 노드 수 요약
    pub stage_summary: std::collections::BTreeMap<String, usize>,
    /// 예상 위험 (예: 임베딩 차원 변경 → 전체 재색인)
    pub warnings: Vec<String>,
}

/// stage 분류 — path prefix → 단계
fn classify_stage(path: &str) -> (&'static str, &'static str) {
    if path.starts_with("chunking.") { return ("Chunking", "LLM 입력 분할 영향"); }
    if path.starts_with("embedding.") || path.starts_with("vector_db.dim") { return ("Embedding", "벡터 차원/모델 변경 — 전체 재색인 필요할 수 있음"); }
    if path.starts_with("verification.") { return ("Verify", "가공 검증 임계 변경 — quarantine 비율 변동 가능"); }
    if path.starts_with("preprocessing.") { return ("Preprocess", "비텍스트 변환 도구 변경"); }
    if path.starts_with("vector_db.") { return ("VectorDB", "검색 후보 수/RRF 영향"); }
    if path.starts_with("rerank.") { return ("Rerank", "Cross-Encoder 재정렬 영향"); }
    if path.starts_with("crossref.") { return ("CrossRef", "관계 그래프 생성 영향"); }
    if path.starts_with("compression.") { return ("Storage", "압축/보관 영향"); }
    if path.starts_with("remote_storage.") { return ("RemoteStorage", "원격 백업 영향"); }
    if path.starts_with("retention.") { return ("Retention", "자동 purge — 데이터 손실 위험"); }
    if path.starts_with("schedule.") { return ("Schedule", "주기 작업 영향"); }
    if path.starts_with("search.") { return ("Search", "검색 후처리 (Sentence Window/MMR) 영향"); }
    if path.starts_with("memory_tier.") { return ("MemoryTier", "tier 분류 임계 변경 — 다음 갱신 시 반영"); }
    if path.starts_with("notification") { return ("Notify", "알림 영향"); }
    if path.starts_with("sensitive.") { return ("Sensitive", "민감 격리 — 새 파일에만 적용"); }
    if path.starts_with("logging.") { return ("Logging", "로그 레벨 변경"); }
    if path.starts_with("max_workers") { return ("Workers", "병렬도 — CPU/메모리 영향"); }
    ("Misc", "기타")
}

pub fn diff_configs(before: &PipelineConfig, after: &PipelineConfig) -> Result<DryRunReport> {
    let bv = serde_json::to_value(before)?;
    let av = serde_json::to_value(after)?;
    let mut diffs = Vec::new();
    diff_recursive(&bv, &av, "", &mut diffs);

    let mut stage_summary: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let mut warnings = Vec::new();
    for d in &mut diffs {
        let (stage, impact) = classify_stage(&d.path);
        d.stage = stage.into();
        d.impact = impact.into();
        *stage_summary.entry(stage.into()).or_insert(0) += 1;
        if d.path == "vector_db.dim" || d.path == "embedding.default_model" {
            warnings.push("임베딩 차원/모델 변경 — 전체 코퍼스 재색인 필요".into());
        }
        if d.path == "retention.enabled" {
            warnings.push("retention 활성화 — 보관 기간 초과 파일 자동 삭제 시작".into());
        }
    }

    Ok(DryRunReport { diffs, stage_summary, warnings })
}

fn diff_recursive(a: &serde_json::Value, b: &serde_json::Value, prefix: &str, out: &mut Vec<ConfigDiff>) {
    use serde_json::Value;
    match (a, b) {
        (Value::Object(am), Value::Object(bm)) => {
            let mut keys: std::collections::BTreeSet<&str> = am.keys().map(|s| s.as_str()).collect();
            keys.extend(bm.keys().map(|s| s.as_str()));
            for k in keys {
                let p = if prefix.is_empty() { k.to_string() } else { format!("{}.{}", prefix, k) };
                let av = am.get(k).cloned().unwrap_or(Value::Null);
                let bv = bm.get(k).cloned().unwrap_or(Value::Null);
                if av != bv {
                    if matches!(av, Value::Object(_)) || matches!(bv, Value::Object(_)) {
                        diff_recursive(&av, &bv, &p, out);
                    } else {
                        out.push(ConfigDiff {
                            path: p,
                            before: av,
                            after: bv,
                            stage: String::new(),
                            impact: String::new(),
                        });
                    }
                }
            }
        }
        _ => {
            if a != b && !prefix.is_empty() {
                out.push(ConfigDiff {
                    path: prefix.into(),
                    before: a.clone(),
                    after: b.clone(),
                    stage: String::new(),
                    impact: String::new(),
                });
            }
        }
    }
}

// ── 사용 패턴 자동 프로파일링 ─────────────────────────────────

/// 코퍼스 통계 입력 (vector_db에서 fetch)
#[derive(Debug, Clone, Default)]
pub struct CorpusUsageStats {
    pub total_documents: usize,
    /// (doc_type, count) — by_type from DbStats
    pub by_doc_type: Vec<(String, u64)>,
    /// 최근 4주 추가 문서 수 (volume 추정용)
    pub weekly_recent_avg: f32,
    /// 민감 격리 비율 (sensitive_count / total)
    pub sensitive_ratio: f32,
    /// 검색 mode 분포: (mode, count)
    pub search_mode_distribution: Vec<(String, usize)>,
}

/// 코퍼스 통계로부터 SetupProfile 추정 (자동 프로파일링)
pub fn infer_profile_from_usage(stats: &CorpusUsageStats) -> SetupProfile {
    // content_mix — doc_type을 ContentType으로 매핑
    let total = stats.by_doc_type.iter().map(|(_, c)| *c).sum::<u64>().max(1) as f32;
    let mut counts: std::collections::HashMap<ContentType, f32> = std::collections::HashMap::new();
    for (dt, c) in &stats.by_doc_type {
        let kind = doc_type_to_content(dt);
        *counts.entry(kind).or_insert(0.0) += *c as f32;
    }
    let mut content_mix: Vec<(ContentType, f32)> = counts.into_iter()
        .map(|(k, n)| (k, n / total))
        .filter(|(_, r)| *r > 0.05) // 5% 미만은 무시
        .collect();
    content_mix.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    if content_mix.is_empty() { content_mix.push((ContentType::General, 1.0)); }

    // sensitivity — sensitive_ratio 기반
    let sensitivity = if stats.sensitive_ratio >= 0.20 { Sensitivity::High }
        else if stats.sensitive_ratio >= 0.05 { Sensitivity::Medium }
        else { Sensitivity::Low };

    // volume — weekly_recent_avg
    let volume = if stats.weekly_recent_avg >= 500.0 { Volume::Heavy }
        else if stats.weekly_recent_avg >= 50.0 { Volume::Moderate }
        else { Volume::Light };

    // search_intent — mode 분포
    let intent = {
        let total_searches: usize = stats.search_mode_distribution.iter().map(|(_, c)| *c).sum();
        if total_searches == 0 { SearchIntent::Precision }
        else {
            let recent = stats.search_mode_distribution.iter()
                .find(|(m, _)| m == "recent").map(|(_, c)| *c).unwrap_or(0);
            let related = stats.search_mode_distribution.iter()
                .find(|(m, _)| m == "related").map(|(_, c)| *c).unwrap_or(0);
            if recent as f32 / total_searches as f32 > 0.4 { SearchIntent::Temporal }
            else if related as f32 / total_searches as f32 > 0.4 { SearchIntent::Exploration }
            else { SearchIntent::Precision }
        }
    };

    SetupProfile {
        description: Some("자동 프로파일링 결과".into()),
        content_mix,
        sensitivity,
        volume,
        search_intent: intent,
        collaboration: Collaboration::Solo, // 추정 불가, 기본 solo
        user_role: None,
    }
}

fn doc_type_to_content(dt: &str) -> ContentType {
    let t = dt.to_lowercase();
    if t.contains("meeting") || t.contains("회의") { ContentType::Meeting }
    else if t.contains("research") || t.contains("paper") || t.contains("연구") || t.contains("논문") || t.contains("study") { ContentType::Research }
    else if t.contains("code") || t.contains("snippet") || t.contains("repo") || t.contains("스니펫") { ContentType::Code }
    else if t.contains("legal") || t.contains("contract") || t.contains("계약") { ContentType::Legal }
    else { ContentType::General }
}

/// SetupProfile과 현재 추정된 프로파일 간 불일치 항목 검출
#[derive(Debug, Clone, Serialize)]
pub struct ProfileMismatch {
    pub axis: &'static str,
    pub current: String,
    pub inferred: String,
    pub note: String,
}

pub fn detect_mismatch(saved: &SetupProfile, inferred: &SetupProfile) -> Vec<ProfileMismatch> {
    let mut out = Vec::new();
    // content_mix 우세 비교
    let dom = |p: &SetupProfile| p.content_mix.iter()
        .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(c, _)| c.as_key().to_string()).unwrap_or_default();
    if dom(saved) != dom(inferred) {
        out.push(ProfileMismatch {
            axis: "content_type",
            current: dom(saved),
            inferred: dom(inferred),
            note: "저장된 프로파일과 실제 처리된 doc_type 분포가 다릅니다".into(),
        });
    }
    if saved.sensitivity != inferred.sensitivity {
        out.push(ProfileMismatch {
            axis: "sensitivity",
            current: format!("{:?}", saved.sensitivity),
            inferred: format!("{:?}", inferred.sensitivity),
            note: "민감 격리 비율이 저장된 등급과 다릅니다".into(),
        });
    }
    if saved.volume != inferred.volume {
        out.push(ProfileMismatch {
            axis: "volume",
            current: format!("{:?}", saved.volume),
            inferred: format!("{:?}", inferred.volume),
            note: "최근 4주 유입 추세가 저장된 볼륨과 다릅니다".into(),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diff_configs_chunking() {
        let a = PipelineConfig::default_config();
        let mut b = PipelineConfig::default_config();
        b.chunking.target_bytes = 2500;
        let report = diff_configs(&a, &b).expect("diff");
        assert!(report.diffs.iter().any(|d| d.path == "chunking.target_bytes"));
        let chunking = report.diffs.iter().find(|d| d.path == "chunking.target_bytes").unwrap();
        assert_eq!(chunking.stage, "Chunking");
        let _ = a; // silence unused mut on a if no other modifications
    }

    #[test]
    fn test_diff_configs_warning_on_dim() {
        let a = PipelineConfig::default_config();
        let mut b = PipelineConfig::default_config();
        b.vector_db.dim = 768;
        let report = diff_configs(&a, &b).expect("diff");
        assert!(report.warnings.iter().any(|w| w.contains("재색인")));
    }

    #[test]
    fn test_diff_configs_retention_warning() {
        let a = PipelineConfig::default_config();
        let mut b = PipelineConfig::default_config();
        b.retention.enabled = true;
        let report = diff_configs(&a, &b).expect("diff");
        assert!(report.warnings.iter().any(|w| w.contains("자동 삭제")));
    }

    #[test]
    fn test_infer_profile_from_meeting_corpus() {
        let stats = CorpusUsageStats {
            total_documents: 100,
            by_doc_type: vec![("meeting".into(), 80), ("note".into(), 20)],
            weekly_recent_avg: 30.0,
            sensitive_ratio: 0.02,
            search_mode_distribution: vec![],
        };
        let p = infer_profile_from_usage(&stats);
        assert_eq!(p.content_mix[0].0, ContentType::Meeting);
        assert_eq!(p.volume, Volume::Light);
        assert_eq!(p.sensitivity, Sensitivity::Low);
    }

    #[test]
    fn test_infer_high_sensitivity() {
        let stats = CorpusUsageStats {
            total_documents: 100,
            by_doc_type: vec![("code".into(), 100)],
            weekly_recent_avg: 100.0,
            sensitive_ratio: 0.30,
            search_mode_distribution: vec![],
        };
        let p = infer_profile_from_usage(&stats);
        assert_eq!(p.sensitivity, Sensitivity::High);
        assert_eq!(p.volume, Volume::Moderate);
    }

    #[test]
    fn test_infer_temporal_intent() {
        let stats = CorpusUsageStats {
            total_documents: 50,
            by_doc_type: vec![("note".into(), 50)],
            weekly_recent_avg: 10.0,
            sensitive_ratio: 0.0,
            search_mode_distribution: vec![("recent".into(), 7), ("default".into(), 3)],
        };
        let p = infer_profile_from_usage(&stats);
        assert_eq!(p.search_intent, SearchIntent::Temporal);
    }

    #[test]
    fn test_detect_mismatch_volume() {
        let saved = SetupProfile { volume: Volume::Light, ..Default::default() };
        let inferred = SetupProfile { volume: Volume::Heavy, ..Default::default() };
        let mismatches = detect_mismatch(&saved, &inferred);
        assert!(mismatches.iter().any(|m| m.axis == "volume"));
    }
}
