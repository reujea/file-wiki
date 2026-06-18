//! `SettingsRepoPort` super-port + 6 sub-trait skeleton — settings-db-split-1 prep-2 (2026-06-16) 정합.
//!
//! 본 mod = trait 정의만 박힘 (impl 부재). `prep-3` 시점에 `adapters/driven/settings/sqlite.rs`
//! 에서 `SettingsDb` 가 본 trait 들을 impl. 기존 `shared/settings_db.rs::SettingsDb` 의 116
//! 메서드 분류 (메서드 그룹별 책임 단일) 정합:
//!
//! | sub-trait | 책임 영역 | settings_db 측 대응 메서드 |
//! |-----------|----------|--------------------------|
//! | `AuditRepo` | audit_trace 1행 기록 + 조회 | `record_audit_event` / `list_audit_by_trace` |
//! | `TodoRepo` | todo CRUD | `add_todo` / `list_todos` / `complete_todo` |
//! | `DecisionRepo` | Decision Log apply 이력 영속화 + 조회 | `insert_decision` / `list_decisions` / `list_decisions_by_snapshot` |
//! | `MetricRepo` | processing_metrics + search_mode + CRAG + chunk_stats 카운터 | `add_processing_metric` / `get_processing_metric_*` / `get_search_mode_counters` / `get_crag_counters` / `get_chunk_stats` |
//! | `HostToolRepo` | host_tools 감지 캐시 (Phase 81) | `host_tools_cache` 모듈 + `ensure_cached` 정합 |
//! | `LlmCacheRepo` | LLM 결과 캐시 (A1) + GC | `gc_llm_cache_to` / `record_llm_cache_gc` |
//!
//! 본 prep-2 skeleton 의 의도 = `SettingsDb` impl 변경 부재 + 신규 trait 추가만 박힘 →
//! 기존 호출처 영향 0 (lesson #14 R1 + #25 정합 점진적 진화).
//!
//! 후속 prep-3 시점 = `shared/settings_db.rs::SettingsDb` 를 `adapters/driven/settings/sqlite.rs::SqliteSettingsRepo`
//! 로 이전 + 본 trait 들 impl 박힘 + 호출처 `Arc<dyn SettingsRepoPort>` 또는 카테고리별 `Arc<dyn AuditRepo>`
//! 의존 주입 (host 결정 영역).
//!
//! 본 prep-2 영역 부재 = `ConfigRepo` / `SnapshotRepo` / `CredentialRepo` / `PromptRepo` 카테고리 (메서드 다수
//! 박힘 영역) = host 결정 의무, prep-3 명세에 박힘 가능성.

use anyhow::Result;

use crate::domain::settings_models::{
    AuditEventRow, DecisionLogEntry, HostToolCacheRow, LlmCacheEntry, NewTodo, ProcessingMetricSummary,
};

/// audit_trace 1행 기록 + 조회 (Phase 91 A3 + 94 A3 정합).
pub trait AuditRepo: Send + Sync {
    /// 1줄 결정 기록.
    fn record_audit_event(
        &self,
        trace_id: &str,
        stage: &str,
        inputs_hash: Option<&str>,
        output_summary: Option<&str>,
        applied_rule: Option<&str>,
    ) -> Result<()>;

    /// trace_id 별 audit_trace 행 조회.
    fn list_audit_by_trace(&self, trace_id: &str) -> Result<Vec<AuditEventRow>>;
}

/// todo CRUD — `setup_review` 의 todo 진입점 정합.
pub trait TodoRepo: Send + Sync {
    /// todo 1건 추가. 중복 (fingerprint) 시 None.
    fn add_todo(&self, todo: NewTodo<'_>) -> Result<Option<String>>;

    /// status / category 필터 조회.
    fn list_todos(&self, status: Option<&str>, category: Option<&str>) -> Result<Vec<serde_json::Value>>;

    /// todo 완료 처리 — 미존재 시 false.
    fn complete_todo(&self, id: &str) -> Result<bool>;
}

/// Decision Log — `setup_apply` / `setup_apply_modules` 의 결정 이력 영속화 (Phase 82).
pub trait DecisionRepo: Send + Sync {
    /// Decision 1건 INSERT — 신규 row id.
    fn insert_decision(&self, entry: &DecisionLogEntry) -> Result<i64>;

    /// 최근 limit 건 조회 (decided_at 내림차순).
    fn list_decisions(&self, limit: usize) -> Result<Vec<DecisionLogEntry>>;

    /// snapshot_id 별 Decision 조회.
    fn list_decisions_by_snapshot(&self, snapshot_id: &str) -> Result<Vec<DecisionLogEntry>>;
}

/// processing_metrics + search_mode + CRAG + chunk_stats 카운터 (Phase 80~82-prep).
pub trait MetricRepo: Send + Sync {
    /// 단일 카운터 누적 (key 별 delta).
    fn add_processing_metric(&self, key: &str, delta: i64) -> Result<()>;

    /// 누적 카운터 raw 조회.
    fn get_processing_metric_raw(&self) -> Result<std::collections::HashMap<String, i64>>;

    /// 누적 카운터 → 요약 (verify_pass_rate / quarantine_rate / avg_process_time_ms).
    fn get_processing_metric_summary(&self) -> Result<ProcessingMetricSummary>;

    /// 검색 mode 카운터 — (mode, count, last_at).
    fn get_search_mode_counters(&self) -> Result<Vec<(String, u64, Option<String>)>>;

    /// CRAG 신뢰도 카운터 — (bucket, count, last_at).
    fn get_crag_counters(&self) -> Result<Vec<(String, u64, Option<String>)>>;

    /// 청킹 통계 — (key, value, last_at).
    fn get_chunk_stats(&self) -> Result<Vec<(String, f64, Option<String>)>>;
}

/// host_tools 감지 캐시 (Phase 81) — pandoc / python_docx / libreoffice 등.
pub trait HostToolRepo: Send + Sync {
    /// 캐시 보유 시 반환, 부재 시 즉시 감지 + 저장 + 반환.
    fn ensure_cached(&self) -> Result<Vec<(String, String)>>;

    /// 강제 재감지 + 저장.
    fn refresh(&self) -> Result<Vec<(String, String)>>;

    /// 캐시 raw 조회.
    fn list_host_tools(&self) -> Result<Vec<HostToolCacheRow>>;
}

/// A1 LLM 캐시 GC + 통계 — file_hash / content_hash 기반.
pub trait LlmCacheRepo: Send + Sync {
    /// 캐시 entry 1건 조회 — (file_hash, content_hash) 키.
    fn get_llm_cache(&self, file_hash: &str, content_hash: &str) -> Result<Option<LlmCacheEntry>>;

    /// 캐시 entry 1건 저장 + hits 누적.
    fn save_llm_cache(&self, entry: &LlmCacheEntry) -> Result<()>;

    /// LRU GC — max_entries 초과분 삭제, 삭제 개수 반환.
    fn gc_llm_cache_to(&self, max_entries: u64) -> Result<usize>;

    /// 마지막 GC 결과 기록 (id=1 단일 row).
    fn record_llm_cache_gc(&self, at: &str, deleted: i64) -> Result<()>;
}

/// super-port — 6 sub-trait 합집합. prep-3 시점 `SqliteSettingsRepo` 의 단일 `dyn SettingsRepoPort`
/// 주입 영역. 카테고리별 분리 주입 (`Arc<dyn AuditRepo>` 등) 도 가능 — 의존 영역 따라 host 결정.
pub trait SettingsRepoPort:
    AuditRepo + TodoRepo + DecisionRepo + MetricRepo + HostToolRepo + LlmCacheRepo
{
}
