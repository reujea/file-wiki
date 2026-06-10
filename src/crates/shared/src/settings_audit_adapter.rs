//! Phase 94 A3: AuditPort 어댑터 — settings.db `audit_trace` 테이블 기록.
//!
//! 헥사고날 의존 방향:
//! - core/ports/output.rs: AuditPort trait + NullAuditAdapter (디폴트 no-op, lesson 14 회피)
//! - shared (본 모듈): SettingsAuditAdapter — settings.db 기반 실 구현
//! - shared의 build_service에서 service에 주입
//!
//! 실패는 silent — LLM/검색 본 흐름을 절대 막지 않음.

use crate::settings_db::SettingsDb;
use file_pipeline_core::ports::output::AuditPort;
use std::path::PathBuf;
use std::sync::Arc;

/// settings.db 기반 AuditPort 어댑터.
///
/// 매 record 호출마다 SettingsDb를 새로 열고 닫음 — audit_trace INSERT는 가벼움.
/// 빈번한 호출 시 connection pool 검토 후속.
pub struct SettingsAuditAdapter {
    db_path: PathBuf,
}

impl SettingsAuditAdapter {
    pub fn new(db_path: PathBuf) -> Self {
        Self { db_path }
    }

    /// Arc로 감싸 어댑터 인스턴스 공유.
    pub fn shared(db_path: PathBuf) -> Arc<dyn AuditPort> {
        Arc::new(Self::new(db_path))
    }
}

impl AuditPort for SettingsAuditAdapter {
    fn record(
        &self,
        trace_id: &str,
        stage: &str,
        inputs_hash: Option<&str>,
        output_summary: Option<&str>,
        applied_rule: Option<&str>,
    ) {
        // 실패는 silent — 본 흐름을 절대 막지 않음.
        if let Ok(db) = SettingsDb::open(&self.db_path) {
            let _ = db.record_audit_event(trace_id, stage, inputs_hash, output_summary, applied_rule);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_settings_audit_adapter_records() {
        let tmp = TempDir::new().expect("tmp");
        let db_path = tmp.path().join("settings.db");
        let _ = SettingsDb::open(&db_path).expect("open db");

        let adapter = SettingsAuditAdapter::new(db_path.clone());
        adapter.record("trace1", "llm.classify", Some("abcd1234"), Some("success"), Some("success"));
        adapter.record("trace1", "search.hybrid", None, Some("3 results"), None);

        let db = SettingsDb::open(&db_path).expect("reopen");
        let rows = db.list_audit_by_trace("trace1").expect("list");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].stage, "llm.classify");
        assert_eq!(rows[1].stage, "search.hybrid");
    }

    #[test]
    fn test_settings_audit_adapter_silent_failure() {
        // 존재하지 않는 경로 → silent failure (panic 없어야 함)
        let adapter = SettingsAuditAdapter::new(PathBuf::from("/nonexistent/settings.db"));
        adapter.record("trace_x", "stage_x", None, None, None);
        // panic 없으면 통과
    }
}
