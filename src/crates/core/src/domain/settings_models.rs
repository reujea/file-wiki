//! `SettingsDb` 도메인 모델 — settings-db-split-1 prep-1 (2026-06-16) 정합.
//!
//! `shared/settings_db.rs` 안 박힌 6 도메인 struct (Row 타입 + 입력 struct) 를 core/domain 으로 이전.
//! 본 struct들은 pure data (anyhow / serde / std 만 의존) = IO 부재 = 헥사고날 도메인 영역 정합.
//!
//! 분리 의도 = prep-2 (`SettingsRepoPort` trait 정의) + prep-3 (`SettingsDb` impl 어댑터 분리) 의 prep.
//! 본 prep 만 진입 시 = shared 안 re-export 박힘 (`pub use file_pipeline_core::domain::settings_models::*`)
//! → 외부 호출처 (`crate::settings_db::DecisionLogEntry` 등) backward compat 정합.

/// Phase 91 A3: audit_trace 1행. `SettingsDb::list_audit_by_trace` 반환형.
#[derive(Debug, Clone)]
pub struct AuditEventRow {
    pub stage: String,
    pub inputs_hash: Option<String>,
    pub output_summary: Option<String>,
    pub applied_rule: Option<String>,
    pub created_at: String,
}

/// `add_todo` 입력 — Optional 필드 다수 (lesson 36 too_many_arguments 해소)
pub struct NewTodo<'a> {
    pub title: &'a str,
    pub category: &'a str,
    pub doc_id: Option<&'a str>,
    pub doc_description: Option<&'a str>,
    pub fingerprint: &'a str,
    pub source_line: Option<i64>,
    pub source_text: Option<&'a str>,
    pub due_date: Option<&'a str>,
}

/// A1: LLM 결과 캐시 항목 (Ruflo ReasoningBank 차용)
#[derive(Debug, Clone)]
pub struct LlmCacheEntry {
    pub file_hash: String,
    pub content_hash: String,
    pub result_json: String,
    pub doc_types: String,
    pub hits: u64,
    pub created_at: String,
    pub last_hit_at: Option<String>,
}

/// Phase 81: 호스트 도구 캐시 행
#[derive(Debug, Clone)]
pub struct HostToolCacheRow {
    pub tool: String,
    pub version: String,
    pub detected_at: String,
    pub not_found: bool,
    pub install_hint: Option<String>,
}

/// Phase 82: Decision Log 1건. setup_apply / setup_apply_modules의 ConfigChange 결정.
///
/// - `decision`: "accepted" / "rejected" / "critical_skipped"
/// - `source`: "setup_review" (path 기반) / "setup_modules" (모듈 ID 기반)
/// - `snapshot_id`: 적용 성공 시 ConfigSnapshot.id 링크 (rejected는 보통 None)
/// - `before_value` / `after_value`: serde_json 직렬화 문자열 (NULL 허용)
/// - `context`: 적용 호출의 모듈 ID 배열·시나리오 등 JSON 메타
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct DecisionLogEntry {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<i64>,
    pub decided_at: String,
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot_id: Option<String>,
    pub path: String,
    pub decision: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub risk: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
}

/// Phase 82-prep: 누적 카운터에서 산출된 처리 메트릭 요약.
/// 데이터 부족(분모 0) 시 비율 필드는 None — caller가 placeholder 처리 결정.
#[derive(Debug, Clone, serde::Serialize, Default)]
pub struct ProcessingMetricSummary {
    pub verify_pass_rate: Option<f32>,
    pub quarantine_rate: Option<f32>,
    pub avg_process_time_ms: Option<u64>,
    pub success: u64,
    pub errors: u64,
    pub quarantined: u64,
    pub verified_pass: u64,
    pub verified_fail: u64,
    pub counted_for_time: u64,
}
