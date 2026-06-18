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
//!
//! 헥사고날 분리 (prep-3c): 순수 데이터 타입 + 순수 함수는
//! `file_pipeline_core::domain::config_models`로 이전됨. 본 모듈은 아래 re-export로
//! 기존 `crate::config_snapshot::X` 호출처를 그대로 흡수하며, 인프라 의존 로직
//! (create_snapshot/rollback_snapshot — SetupProfile + fs + PipelineConfigExt)만 잔류한다.

use anyhow::{Context, Result};
use std::path::Path;

use crate::config::PipelineConfig;
use crate::config::PipelineConfigExt;
use crate::setup_review::SetupProfile;

// ── core로 이전된 순수 타입 re-export ────────────────────────────────
// 기존 `crate::config_snapshot::ConfigSnapshot` 등 호출처가 변경 없이 작동하도록.
pub use file_pipeline_core::domain::config_models::{
    evaluate_rollback, ConfigSnapshot, RollbackEvaluation, RollbackThresholds, SnapshotMetrics,
};

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
}
