//! Ruflo C1 1단계 — Phase 80 카운터 기반 자동 추천 (Self-Learning Stub)
//!
//! 누적된 검색 mode 카운터·CRAG 신뢰도 카운터를 분석해, 사용자 confirm 없이도
//! `decision_log` 테이블에 `source="auto_suggestion"`으로 추천 entry를 INSERT한다.
//! 사용자는 Dashboard나 MCP `setup_decision_log_list`로 검토 후 수동 적용한다.
//!
//! 본 모듈은 **제안만** 한다 — 실제 config 변경은 하지 않는다 (lesson 30 패턴).

use std::path::Path;

use anyhow::{Context, Result};
use chrono::Utc;

use crate::config::PipelineConfigExt;

use crate::settings_db::{DecisionLogEntry, SettingsDb};

/// 누적 카운터 기반 자동 추천 생성
///
/// 반환값: INSERT된 entry 개수.
///
/// 임계값:
/// - 검색 mode 100회 이상 누적 시 dominant mode 분석
/// - CRAG incorrect 비율 25% 초과 시 similarity_threshold 상향 제안
pub fn suggest_from_counters(db: &SettingsDb) -> Result<usize> {
    let mut inserted = 0;
    let now = Utc::now().to_rfc3339();

    // 임계값 — DB 룰 우선, 없으면 코드 디폴트
    let mode_min_total = db.get_c1_threshold("mode_min_total", 100.0).unwrap_or(100.0) as u64;
    let mode_dominant_ratio = db.get_c1_threshold("mode_dominant_ratio", 0.6).unwrap_or(0.6) as f32;
    let crag_min_total = db.get_c1_threshold("crag_min_total", 50.0).unwrap_or(50.0) as u64;
    let crag_incorrect_ratio = db.get_c1_threshold("crag_incorrect_ratio", 0.25).unwrap_or(0.25) as f32;
    let processed_min = db.get_c1_threshold("processed_min", 30.0).unwrap_or(30.0) as u64;
    let quarantine_ratio = db.get_c1_threshold("quarantine_ratio", 0.25).unwrap_or(0.25) as f32;
    let verify_pass_min = db.get_c1_threshold("verify_pass_min", 0.6).unwrap_or(0.6) as f32;

    // 1) 검색 mode 분석
    let mode_rows = db.get_search_mode_counters().unwrap_or_default();
    let total_searches: u64 = mode_rows.iter().map(|(_, c, _)| *c).sum();

    if total_searches >= mode_min_total {
        // dominant mode 60% 초과 시 해당 mode 우선 추천
        if let Some((mode, count, _)) = mode_rows.iter().max_by_key(|(_, c, _)| *c) {
            let ratio = *count as f32 / total_searches as f32;
            if ratio > mode_dominant_ratio {
                let entry = make_entry(DecisionDraft {
                    now: &now,
                    path: "search.preferred_mode",
                    after_value: &format!("\"{}\"", mode),
                    priority: "medium",
                    risk: "low",
                    evidence: &format!("{} 모드 {}회 / 전체 {}회 ({:.0}%)", mode, count, total_searches, ratio * 100.0),
                    confidence: "medium",
                    reason: &format!("사용자가 주로 {} 모드를 사용 — 디폴트 mode 변경 검토", mode),
                });
                db.insert_decision(&entry)?;
                inserted += 1;
            }
        }
    }

    // 2) CRAG 신뢰도 분석
    let crag_rows = db.get_crag_counters().unwrap_or_default();
    let total_crag: u64 = crag_rows.iter().map(|(_, c, _)| *c).sum();

    if total_crag >= crag_min_total {
        let incorrect = crag_rows.iter()
            .find(|(b, _, _)| b == "incorrect")
            .map(|(_, c, _)| *c).unwrap_or(0);
        let ratio = incorrect as f32 / total_crag as f32;
        if ratio > crag_incorrect_ratio {
            let entry = make_entry(DecisionDraft {
                now: &now,
                path: "vector_db.similarity_threshold",
                after_value: "0.85",
                priority: "high",
                risk: "medium",
                evidence: &format!("CRAG incorrect {}건 / 전체 {}건 ({:.0}%) — 임계값 상향으로 부적합 결과 감소 기대",
                    incorrect, total_crag, ratio * 100.0),
                confidence: "medium",
                reason: "검색 신뢰도 낮음 — similarity_threshold 0.85 시도 권장",
            });
            db.insert_decision(&entry)?;
            inserted += 1;
        }
    }

    // 3) 처리 메트릭 — quarantine_rate 25% 초과 시 max_retry 상향 제안
    if let Ok(summary) = db.get_processing_metric_summary() {
        let processed_total = summary.success + summary.errors;
        if processed_total >= processed_min {
            if let Some(qrate) = summary.quarantine_rate {
                if qrate > quarantine_ratio {
                    let entry = make_entry(DecisionDraft {
                        now: &now,
                        path: "verification.max_retry",
                        after_value: "3",
                        priority: "medium",
                        risk: "low",
                        evidence: &format!("quarantine {}건 / 처리 {}건 ({:.0}%) — 2-Pass 재시도 늘려 격리 감소",
                            summary.quarantined, processed_total, qrate * 100.0),
                        confidence: "medium",
                        reason: "격리율 높음 — max_retry 상향 검토",
                    });
                    db.insert_decision(&entry)?;
                    inserted += 1;
                }
            }
            // verify_pass_rate < 60% → structure_min 완화 제안
            if let Some(pass) = summary.verify_pass_rate {
                if pass < verify_pass_min {
                    let entry = make_entry(DecisionDraft {
                        now: &now,
                        path: "verification.thresholds.structure_min",
                        after_value: "0.3",
                        priority: "medium",
                        risk: "medium",
                        evidence: &format!("verify pass {}건 / 검증 {}건 ({:.0}%) — 구조 임계값 완화로 통과율 회복",
                            summary.verified_pass, summary.verified_pass + summary.verified_fail, pass * 100.0),
                        confidence: "low",
                        reason: "검증 통과율 낮음 — structure_min 완화 검토 (정확도 손실 가능성 동반)",
                    });
                    db.insert_decision(&entry)?;
                    inserted += 1;
                }
            }
        }
    }

    Ok(inserted)
}

/// 사용자가 confirm한 suggested decision_log entry를 pipeline.toml에 적용.
///
/// 동작:
/// 1. db에서 entry 조회 (decision="suggested"이어야 함)
/// 2. after_value를 path 위치에 toml_edit으로 쓰기 (주석 보존)
/// 3. .toml.bak 백업 생성
/// 4. entry의 decision을 "accepted"로 갱신 (재호출 방지)
///
/// 반환: 적용된 (path, after_value) 튜플.
pub fn apply_suggested(
    db: &SettingsDb,
    config_path: &Path,
    decision_id: i64,
) -> Result<(String, String)> {
    let entry = db.get_decision(decision_id)
        .context("decision_log 조회 실패")?
        .ok_or_else(|| anyhow::anyhow!("decision_log entry 없음: id={}", decision_id))?;

    if entry.decision != "suggested" {
        anyhow::bail!("이미 처리된 entry (decision={}): id={}", entry.decision, decision_id);
    }
    let after_value = entry.after_value.clone()
        .ok_or_else(|| anyhow::anyhow!("after_value 없음: id={}", decision_id))?;

    // toml_edit으로 적용 — 주석 보존
    if config_path.exists() {
        let bak = config_path.with_extension("toml.bak");
        std::fs::copy(config_path, &bak).context("pipeline.toml.bak 백업 실패")?;
    }
    let raw = if config_path.exists() {
        std::fs::read_to_string(config_path).context("pipeline.toml 읽기 실패")?
    } else {
        crate::config::PipelineConfig::default_config().to_toml_string()?
    };
    let mut doc = raw.parse::<toml_edit::DocumentMut>()
        .context("pipeline.toml toml_edit 파싱 실패")?;
    let value: serde_json::Value = serde_json::from_str(&after_value)
        .with_context(|| format!("after_value JSON 파싱 실패: {}", after_value))?;
    crate::setup_review::write_toml_path(&mut doc, &entry.path, &value)
        .with_context(|| format!("toml path 쓰기 실패: {}", entry.path))?;

    std::fs::write(config_path, doc.to_string()).context("pipeline.toml 쓰기 실패")?;

    // decision 상태 갱신 (suggested → accepted)
    db.update_decision_status(decision_id, "accepted")?;

    Ok((entry.path, after_value))
}

/// 사용자가 reject한 suggested entry를 rejected로 마킹 (toml 변경 없음).
pub fn reject_suggested(db: &SettingsDb, decision_id: i64) -> Result<()> {
    let entry = db.get_decision(decision_id)?
        .ok_or_else(|| anyhow::anyhow!("decision_log entry 없음: id={}", decision_id))?;
    if entry.decision != "suggested" {
        anyhow::bail!("이미 처리된 entry (decision={}): id={}", entry.decision, decision_id);
    }
    db.update_decision_status(decision_id, "rejected")
}

struct DecisionDraft<'a> {
    now: &'a str,
    path: &'a str,
    after_value: &'a str,
    priority: &'a str,
    risk: &'a str,
    evidence: &'a str,
    confidence: &'a str,
    reason: &'a str,
}

fn make_entry(d: DecisionDraft<'_>) -> DecisionLogEntry {
    DecisionLogEntry {
        id: None,
        decided_at: d.now.to_string(),
        source: "auto_suggestion".to_string(),
        snapshot_id: None,
        path: d.path.to_string(),
        decision: "suggested".to_string(),
        before_value: None,
        after_value: Some(d.after_value.to_string()),
        priority: Some(d.priority.to_string()),
        risk: Some(d.risk.to_string()),
        evidence: Some(d.evidence.to_string()),
        confidence: Some(d.confidence.to_string()),
        reason: Some(d.reason.to_string()),
        context: Some("c1_auto_suggester".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_no_suggestion_below_threshold() {
        let tmp = tempdir().expect("tempdir");
        let db = SettingsDb::open(&tmp.path().join("settings.db")).expect("open");
        // 검색 카운터 50회만 (임계값 100 미달)
        for _ in 0..50 { db.increment_search_mode("default").expect("inc"); }
        let n = suggest_from_counters(&db).expect("suggest");
        assert_eq!(n, 0, "임계값 미달 시 제안 없음");
    }

    #[test]
    fn test_suggest_on_dominant_mode() {
        let tmp = tempdir().expect("tempdir");
        let db = SettingsDb::open(&tmp.path().join("settings.db")).expect("open");
        for _ in 0..80 { db.increment_search_mode("exact").expect("inc"); }
        for _ in 0..20 { db.increment_search_mode("default").expect("inc"); }
        let n = suggest_from_counters(&db).expect("suggest");
        assert!(n >= 1, "dominant mode 80% → 제안 발생");
        let decisions = db.list_decisions(10).expect("list");
        assert!(decisions.iter().any(|d| d.path == "search.preferred_mode"));
    }

    #[test]
    fn test_apply_suggested_writes_toml_and_marks_accepted() {
        let tmp = tempdir().expect("tempdir");
        let db = SettingsDb::open(&tmp.path().join("settings.db")).expect("open");
        // 1) suggested entry 1건 INSERT
        for _ in 0..80 { db.increment_search_mode("exact").expect("inc"); }
        for _ in 0..20 { db.increment_search_mode("default").expect("inc"); }
        suggest_from_counters(&db).expect("suggest");
        let decisions = db.list_decisions(10).expect("list");
        let entry = decisions.iter().find(|d| d.path == "search.preferred_mode")
            .expect("suggested entry 있어야 함");
        let id = entry.id.expect("id");

        // 2) pipeline.toml 빈 파일 준비
        let toml_path = tmp.path().join("pipeline.toml");
        std::fs::write(&toml_path, "version = \"1\"\n[search]\npreferred_mode = \"default\"\n").expect("write toml");

        // 3) apply
        let (applied_path, _) = apply_suggested(&db, &toml_path, id).expect("apply");
        assert_eq!(applied_path, "search.preferred_mode");

        // 4) toml에 값 반영됐는지
        let raw = std::fs::read_to_string(&toml_path).expect("read");
        assert!(raw.contains("\"exact\""), "toml에 exact 반영: {}", raw);

        // 5) decision 상태가 accepted로 갱신
        let after = db.get_decision(id).expect("get").expect("entry");
        assert_eq!(after.decision, "accepted");

        // 6) 재호출 시 에러 (이미 처리됨)
        let result = apply_suggested(&db, &toml_path, id);
        assert!(result.is_err(), "이미 처리된 entry는 재적용 거부");
    }

    #[test]
    fn test_suggest_on_high_quarantine_rate() {
        let tmp = tempdir().expect("tempdir");
        let db = SettingsDb::open(&tmp.path().join("settings.db")).expect("open");
        // 처리 메트릭 — processed_total 35 (≥30) + quarantine 30%
        db.add_processing_metric("success", 30).expect("add");
        db.add_processing_metric("errors", 5).expect("add");
        db.add_processing_metric("quarantined", 12).expect("add");
        let n = suggest_from_counters(&db).expect("suggest");
        assert!(n >= 1, "quarantine 30% → 제안");
        let decisions = db.list_decisions(10).expect("list");
        assert!(decisions.iter().any(|d| d.path == "verification.max_retry"));
    }

    #[test]
    fn test_suggest_on_low_verify_pass_rate() {
        let tmp = tempdir().expect("tempdir");
        let db = SettingsDb::open(&tmp.path().join("settings.db")).expect("open");
        // verify pass 10 / total 30 (33%)
        db.add_processing_metric("success", 30).expect("add");
        db.add_processing_metric("verified_pass", 10).expect("add");
        db.add_processing_metric("verified_fail", 20).expect("add");
        let n = suggest_from_counters(&db).expect("suggest");
        assert!(n >= 1, "verify 33% → 제안");
        let decisions = db.list_decisions(10).expect("list");
        assert!(decisions.iter().any(|d| d.path == "verification.thresholds.structure_min"));
    }

    #[test]
    fn test_reject_suggested_marks_rejected() {
        let tmp = tempdir().expect("tempdir");
        let db = SettingsDb::open(&tmp.path().join("settings.db")).expect("open");
        for _ in 0..30 { db.increment_crag("incorrect").expect("inc"); }
        for _ in 0..30 { db.increment_crag("correct").expect("inc"); }
        suggest_from_counters(&db).expect("suggest");
        let id = db.list_decisions(10).expect("list")
            .iter().find(|d| d.path == "vector_db.similarity_threshold")
            .and_then(|d| d.id).expect("id");

        reject_suggested(&db, id).expect("reject");
        let after = db.get_decision(id).expect("get").expect("entry");
        assert_eq!(after.decision, "rejected");
    }

    #[test]
    fn test_suggest_on_high_incorrect() {
        let tmp = tempdir().expect("tempdir");
        let db = SettingsDb::open(&tmp.path().join("settings.db")).expect("open");
        for _ in 0..30 { db.increment_crag("incorrect").expect("inc"); }
        for _ in 0..30 { db.increment_crag("correct").expect("inc"); }
        let n = suggest_from_counters(&db).expect("suggest");
        assert!(n >= 1, "incorrect 50% → 제안 발생");
        let decisions = db.list_decisions(10).expect("list");
        assert!(decisions.iter().any(|d| d.path == "vector_db.similarity_threshold"));
    }
}
