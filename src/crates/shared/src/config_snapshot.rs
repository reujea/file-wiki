//! Phase 77: 설정 스냅샷 + 효과 측정 + 자동 롤백
//!
//! 흐름:
//! 1. setup_apply 직전 — 현재 pipeline.toml을 ConfigSnapshot으로 백업
//! 2. apply 후 50파일(또는 사용자 지정) 처리 동안 metrics 누적
//! 3. measure_and_check_rollback() — 임계 위반 시 rollback 권고
//! 4. rollback() — toml 원본 복원 + DB 마킹
//!
//! metrics는 stats/lint MCP 도구가 이미 노출하는 지표를 그대로 사용한다
//! (외부 전문가 답변 §4.1). 별도 노드를 추가하지 않고 측정 시점에 fetch.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::config::PipelineConfig;
use crate::setup_review::SetupProfile;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigSnapshot {
    /// 16자 hex (millis 기반)
    pub id: String,
    /// RFC3339
    pub created_at: String,
    /// pipeline.toml SHA256 (16 hex)
    pub config_hash: String,
    /// pipeline.toml 원본 (적용 전)
    pub config_backup: String,
    /// 적용 시 사용한 SetupProfile JSON (없으면 None)
    pub profile_json: Option<String>,
    /// 적용된 path 목록
    pub applied_paths: Vec<String>,
    /// 측정된 metrics JSON (NULL이면 미측정)
    pub metrics_json: Option<String>,
    pub rolled_back: bool,
    pub rollback_reason: Option<String>,
}

impl ConfigSnapshot {
    pub fn applied_paths_json(&self) -> String {
        serde_json::to_string(&self.applied_paths).unwrap_or_else(|_| "[]".into())
    }

    pub fn metrics(&self) -> Option<SnapshotMetrics> {
        self.metrics_json.as_ref().and_then(|s| serde_json::from_str(s).ok())
    }
}

/// 측정 지표 — stats + lint + verify에서 fetch
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SnapshotMetrics {
    /// 측정 시점 RFC3339
    pub measured_at: String,
    /// 측정 시점까지 처리된 문서 수
    pub files_processed: usize,
    /// verify 1-Pass 성공률 (0.0~1.0)
    pub verify_pass_rate: f32,
    /// quarantine 비율 (0.0~1.0)
    pub quarantine_rate: f32,
    /// 평균 처리 시간 (ms, 토큰 사용량 대신 stats.avg_process_ms)
    pub avg_process_time_ms: u64,
    /// lint 경고 수
    pub lint_warnings: usize,
    /// 평균 crossref 링크 수 (문서당)
    pub avg_crossref_per_doc: f32,
}

/// snapshot 생성
pub fn create_snapshot(
    pipeline_toml: &Path,
    profile: Option<&SetupProfile>,
    applied_paths: &[String],
) -> Result<ConfigSnapshot> {
    let raw = if pipeline_toml.exists() {
        std::fs::read_to_string(pipeline_toml).context("pipeline.toml 읽기 실패")?
    } else {
        String::new()
    };
    let hash = sha256_short(&raw);
    let id = generate_id();
    Ok(ConfigSnapshot {
        id,
        created_at: chrono::Utc::now().to_rfc3339(),
        config_hash: hash,
        config_backup: raw,
        profile_json: profile.and_then(|p| serde_json::to_string(p).ok()),
        applied_paths: applied_paths.to_vec(),
        metrics_json: None,
        rolled_back: false,
        rollback_reason: None,
    })
}

fn sha256_short(s: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    hex::encode(&hasher.finalize()[..8])
}

fn generate_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let t = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    format!("{:016x}", t.as_nanos() as u64)
}

/// snapshot의 config_backup을 pipeline.toml로 복원 + DB 마킹
pub fn rollback_snapshot(
    pipeline_toml: &Path,
    snapshot: &ConfigSnapshot,
    reason: &str,
) -> Result<()> {
    // 현재 파일을 .pre-rollback.bak으로 보존
    if pipeline_toml.exists() {
        let bak = pipeline_toml.with_extension("toml.pre-rollback.bak");
        std::fs::copy(pipeline_toml, &bak).context("pre-rollback 백업 실패")?;
    }
    // 검증 후 복원
    PipelineConfig::load_from_str(&snapshot.config_backup)
        .context("snapshot의 backup이 유효한 TOML이 아님 — 롤백 거부")?;
    std::fs::write(pipeline_toml, &snapshot.config_backup).context("롤백 쓰기 실패")?;
    tracing::warn!("설정 롤백 실행 (snapshot={}, reason={})", snapshot.id, reason);
    Ok(())
}

/// 자동 롤백 조건 (외부 전문가 답변 §4.3)
#[derive(Debug, Clone, Copy)]
pub struct RollbackThresholds {
    /// verify_pass_rate가 이전 대비 N%p 이상 떨어지면 트리거
    pub verify_pass_drop_pp: f32,
    /// quarantine_rate가 이 값 초과면 트리거
    pub quarantine_rate_max: f32,
    /// avg_process_time_ms가 이전 대비 N배 이상이면 트리거
    pub process_time_factor_max: f32,
}

impl Default for RollbackThresholds {
    fn default() -> Self {
        Self {
            verify_pass_drop_pp: 0.15,
            quarantine_rate_max: 0.10,
            process_time_factor_max: 2.0,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct RollbackEvaluation {
    pub should_rollback: bool,
    pub triggers: Vec<String>,
}

/// before와 after 메트릭을 비교해 롤백 권고 여부 산출
pub fn evaluate_rollback(
    before: &SnapshotMetrics,
    after: &SnapshotMetrics,
    thresholds: &RollbackThresholds,
) -> RollbackEvaluation {
    let mut triggers = Vec::new();

    let verify_drop = before.verify_pass_rate - after.verify_pass_rate;
    if verify_drop >= thresholds.verify_pass_drop_pp {
        triggers.push(format!(
            "verify_pass_rate {:.1}%p 하락 (임계 {:.1}%p)",
            verify_drop * 100.0,
            thresholds.verify_pass_drop_pp * 100.0,
        ));
    }
    if after.quarantine_rate > thresholds.quarantine_rate_max {
        triggers.push(format!(
            "quarantine_rate {:.1}% (임계 {:.1}%)",
            after.quarantine_rate * 100.0,
            thresholds.quarantine_rate_max * 100.0,
        ));
    }
    if before.avg_process_time_ms > 0 {
        let factor = after.avg_process_time_ms as f32 / before.avg_process_time_ms as f32;
        if factor >= thresholds.process_time_factor_max {
            triggers.push(format!(
                "avg_process_time_ms {:.1}배 증가 (임계 {:.1}배)",
                factor, thresholds.process_time_factor_max,
            ));
        }
    }

    RollbackEvaluation {
        should_rollback: !triggers.is_empty(),
        triggers,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_snapshot_with_existing_file() {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().expect("temp");
        writeln!(tmp, "version = \"1\"").expect("write");
        let snap = create_snapshot(tmp.path(), None, &["a.b".into()]).expect("create");
        assert!(!snap.id.is_empty());
        assert_eq!(snap.applied_paths, vec!["a.b"]);
        assert!(snap.config_backup.contains("version"));
        assert!(!snap.rolled_back);
    }

    #[test]
    fn test_rollback_restores_backup() {
        use std::io::Write;
        let mut tmp = tempfile::NamedTempFile::new().expect("temp");
        let original = PipelineConfig::default_config().to_toml_string().unwrap();
        write!(tmp, "{}", original).expect("write");
        let snap = create_snapshot(tmp.path(), None, &[]).expect("create");

        // 파일을 유효한 다른 TOML로 수정 (PipelineConfig가 파싱 가능해야 함)
        let modified = original.replace("version = \"1\"", "version = \"999_modified\"");
        std::fs::write(tmp.path(), &modified).expect("modify");
        let after_modify = std::fs::read_to_string(tmp.path()).expect("read mod");
        assert!(after_modify.contains("999_modified"), "수정이 반영되어야");

        rollback_snapshot(tmp.path(), &snap, "테스트").expect("rollback");
        let restored = std::fs::read_to_string(tmp.path()).expect("read");
        assert!(!restored.contains("999_modified"), "원본으로 복원");
        assert_eq!(restored, original, "snapshot의 backup과 정확히 일치");
    }

    #[test]
    fn test_evaluate_rollback_triggers_on_quarantine() {
        let before = SnapshotMetrics { verify_pass_rate: 0.95, quarantine_rate: 0.02, avg_process_time_ms: 100, ..Default::default() };
        let after = SnapshotMetrics { verify_pass_rate: 0.94, quarantine_rate: 0.15, avg_process_time_ms: 110, ..Default::default() };
        let ev = evaluate_rollback(&before, &after, &RollbackThresholds::default());
        assert!(ev.should_rollback);
        assert!(ev.triggers.iter().any(|t| t.contains("quarantine_rate")));
    }

    #[test]
    fn test_evaluate_rollback_triggers_on_verify_drop() {
        let before = SnapshotMetrics { verify_pass_rate: 0.95, quarantine_rate: 0.02, avg_process_time_ms: 100, ..Default::default() };
        let after = SnapshotMetrics { verify_pass_rate: 0.70, quarantine_rate: 0.05, avg_process_time_ms: 110, ..Default::default() };
        let ev = evaluate_rollback(&before, &after, &RollbackThresholds::default());
        assert!(ev.should_rollback);
        assert!(ev.triggers.iter().any(|t| t.contains("verify_pass_rate")));
    }

    #[test]
    fn test_evaluate_rollback_no_trigger() {
        let before = SnapshotMetrics { verify_pass_rate: 0.95, quarantine_rate: 0.02, avg_process_time_ms: 100, ..Default::default() };
        let after = SnapshotMetrics { verify_pass_rate: 0.94, quarantine_rate: 0.03, avg_process_time_ms: 110, ..Default::default() };
        let ev = evaluate_rollback(&before, &after, &RollbackThresholds::default());
        assert!(!ev.should_rollback);
        assert!(ev.triggers.is_empty());
    }

    #[test]
    fn test_metrics_serde_roundtrip() {
        let m = SnapshotMetrics {
            measured_at: "2026-05-07T10:00:00Z".into(),
            files_processed: 50,
            verify_pass_rate: 0.92,
            quarantine_rate: 0.03,
            avg_process_time_ms: 250,
            lint_warnings: 4,
            avg_crossref_per_doc: 2.5,
        };
        let s = serde_json::to_string(&m).unwrap();
        let r: SnapshotMetrics = serde_json::from_str(&s).unwrap();
        assert_eq!(r.files_processed, 50);
        assert_eq!(r.lint_warnings, 4);
    }
}
