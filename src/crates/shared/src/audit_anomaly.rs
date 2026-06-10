//! Phase 92 H1: audit_trace 누적 데이터에서 이상 패턴 자동 감지 + 사용자 알림 권고.
//!
//! JAMES 자체 진화 게이트의 "자동 롤백 트리거" 패턴 흡수 (RBAC/Change Request 게이트는 보류):
//! - 피드백 → 후보 → 벤치 → **사용자 알림 권고** → 사용자 수동 승인 (자동 롤백 아닌 알림)
//! - 단일 사용자 도메인 정렬 (lesson 50 메타 룰 20)
//!
//! Phase 77 `evaluate_rollback`(before/after 비교)과 다른 영역 — 본 모듈은 audit_trace
//! 누적 자체에서 패턴 추출.
//!
//! ## 메타 룰 적용
//! - 메타 룰 1: 이상 감지를 단일 진입점 `analyze_recent_audit` 으로 통일
//! - 메타 룰 13: 인프라 추가 1단계 (호출처 부착 + 측정 + UI 노출은 후속 phase)
//! - 메타 룰 18: lesson 46 G-1 "추정" 같은 빗나감을 trace 누적으로 root cause 확정

use crate::settings_db::{AuditEventRow, SettingsDb};
use anyhow::Result;
use std::collections::HashMap;

/// 이상 감지 임계값. 디폴트는 보수적 — 명백한 패턴만 트리거.
#[derive(Debug, Clone)]
pub struct AnomalyThresholds {
    /// 동일 stage에서 N건 이상 실패가 최근 사이즈 안에 발생하면 트리거 (G-1 패턴)
    pub stage_failure_count: usize,
    /// 분석 윈도우 (최근 N건)
    pub recent_window: usize,
    /// 동일 stage에서 평균 대비 N배 이상 지연 시 트리거 (지연 패턴)
    /// 단, audit_trace는 latency를 직접 보유하지 않음 — 본 임계값은 후속 확장용 placeholder.
    pub latency_factor: f32,
}

impl Default for AnomalyThresholds {
    fn default() -> Self {
        Self {
            stage_failure_count: 5,
            recent_window: 50,
            latency_factor: 3.0,
        }
    }
}

/// 단일 이상 신호.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnomalySignal {
    /// 이상 분류 (stage_failure / latency_spike / unknown)
    pub kind: String,
    /// 영향받은 stage
    pub stage: String,
    /// 사용자 표시용 설명
    pub summary: String,
    /// 권고 액션 (보수적 — "검토 권장" 수준, 자동 롤백 아님)
    pub recommendation: String,
}

/// 이상 감지 결과.
#[derive(Debug, Clone)]
pub struct AnomalyReport {
    pub signals: Vec<AnomalySignal>,
    pub examined_events: usize,
}

impl AnomalyReport {
    pub fn has_anomaly(&self) -> bool {
        !self.signals.is_empty()
    }
}

/// audit_trace 최근 N건을 분석하여 이상 패턴 추출.
///
/// 본 함수는 settings.db 직접 접근. 호출처는 service.rs 또는 별도 진단 도구.
///
/// JAMES "자체 진화 게이트" 흡수: 사용자에게 알림만, 자동 롤백 아님.
pub fn analyze_recent_audit(
    db: &SettingsDb,
    thresholds: &AnomalyThresholds,
) -> Result<AnomalyReport> {
    // settings.db에서 최근 N건 조회 — 신규 메서드 list_recent_audit 필요.
    // 현재는 트레이스 ID 기반 조회만 있어 본 함수에서 SQL 직접 작성.
    let events = db.list_recent_audit_events(thresholds.recent_window)?;
    Ok(analyze_events(&events, thresholds))
}

/// 이벤트 리스트에서 직접 분석 — 테스트 + 진단 도구 재사용 가능.
pub fn analyze_events(
    events: &[AuditEventRow],
    thresholds: &AnomalyThresholds,
) -> AnomalyReport {
    let mut signals = Vec::new();

    // stage별 카운트 + 실패 분류 (applied_rule이 "error" 또는 "failure"로 시작)
    let mut stage_failures: HashMap<String, usize> = HashMap::new();
    for event in events {
        let is_failure = event.applied_rule.as_deref()
            .map(|r| r.starts_with("error") || r.starts_with("failure") || r.contains("quarantine"))
            .unwrap_or(false);
        if is_failure {
            *stage_failures.entry(event.stage.clone()).or_insert(0) += 1;
        }
    }

    for (stage, count) in stage_failures {
        if count >= thresholds.stage_failure_count {
            signals.push(AnomalySignal {
                kind: "stage_failure".to_string(),
                stage: stage.clone(),
                summary: format!(
                    "Stage '{}' 에서 최근 {}건 중 {}건 실패 (임계값 {}건)",
                    stage, events.len(), count, thresholds.stage_failure_count,
                ),
                recommendation:
                    "최근 trace 검토 권장. replay_trace.sh로 trace_id별 분석. \
                     lesson 46 G-1 같은 외부 일시 요인 또는 어댑터 회귀 가능성. 자동 롤백 아닌 사용자 검토 필요."
                    .to_string(),
            });
        }
    }

    AnomalyReport {
        signals,
        examined_events: events.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mk_event(stage: &str, applied_rule: Option<&str>) -> AuditEventRow {
        AuditEventRow {
            stage: stage.to_string(),
            inputs_hash: None,
            output_summary: None,
            applied_rule: applied_rule.map(String::from),
            created_at: "2026-05-22T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn test_no_anomaly_on_clean_events() {
        let events: Vec<AuditEventRow> = (0..20)
            .map(|_| mk_event("llm.classify", Some("success")))
            .collect();
        let report = analyze_events(&events, &AnomalyThresholds::default());
        assert!(!report.has_anomaly());
        assert_eq!(report.examined_events, 20);
    }

    #[test]
    fn test_stage_failure_above_threshold_triggers() {
        let mut events: Vec<AuditEventRow> = (0..10)
            .map(|_| mk_event("llm.classify", Some("error: claude_cli exit 1")))
            .collect();
        events.extend((0..10).map(|_| mk_event("llm.classify", Some("success"))));

        let report = analyze_events(&events, &AnomalyThresholds::default());
        assert!(report.has_anomaly());
        assert_eq!(report.signals.len(), 1);
        assert_eq!(report.signals[0].kind, "stage_failure");
        assert_eq!(report.signals[0].stage, "llm.classify");
    }

    #[test]
    fn test_failure_below_threshold_no_trigger() {
        // 5건 임계값 → 4건만 실패하면 트리거 안 함
        let mut events: Vec<AuditEventRow> = (0..4)
            .map(|_| mk_event("llm.classify", Some("error")))
            .collect();
        events.extend((0..10).map(|_| mk_event("llm.classify", Some("success"))));

        let report = analyze_events(&events, &AnomalyThresholds::default());
        assert!(!report.has_anomaly());
    }

    #[test]
    fn test_quarantine_keyword_counts_as_failure() {
        let events: Vec<AuditEventRow> = (0..6)
            .map(|_| mk_event("verify.run", Some("quarantine_routed")))
            .collect();
        let report = analyze_events(&events, &AnomalyThresholds::default());
        assert!(report.has_anomaly());
        assert!(report.signals[0].recommendation.contains("롤백"));
    }

    #[test]
    fn test_recommendation_says_user_review_not_auto_rollback() {
        // 메타 룰 20 자기 적용 — JAMES RBAC 보류 정책. 자동 롤백 아닌 사용자 검토만 권고.
        let events: Vec<AuditEventRow> = (0..6)
            .map(|_| mk_event("mcp.search", Some("error")))
            .collect();
        let report = analyze_events(&events, &AnomalyThresholds::default());
        assert!(report.signals[0].recommendation.contains("사용자 검토"));
        assert!(report.signals[0].recommendation.contains("자동 롤백 아닌"));
    }
}
