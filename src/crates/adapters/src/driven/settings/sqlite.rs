//! 설정 SQLite DB — pipeline.toml + doc_types.toml + prompts.toml 통합 관리
//!
//! 테이블: config (섹션별 key-value), doc_types, prompts, credentials
//!
//! ── settings-db-split-1 prep-3 (2026-06-17): `SettingsDb` 본체 이전 ──────────
//! 기존 `shared/settings_db.rs::SettingsDb` (struct + 순수 DB 메서드 + 6 sub-trait impl)
//! 를 adapters 로 이전. shared 측은 `pub use` re-export + `open_or_migrate` 자유함수만 잔류.
//!
//! config 타입은 `file_pipeline_core::domain::config_models` 직접 경로 사용 (shared re-export
//! 우회 — adapters→shared 역참조 cycle 회피). `PipelineConfigExt`(shared) 의존 메서드는
//! 본 struct 에 부재 — 부팅 toml 마이그레이션(`open_or_migrate`)이 shared 자유함수로 잔류.

use anyhow::{Context, Result};
use rusqlite::{params, Connection, OptionalExtension};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use file_pipeline_core::domain::config_models::{ConfigSnapshot, LlmCredential, PipelineConfig};
use file_pipeline_core::domain::models::DocTypeDef;

/// settings.db 전체 DDL — `open()` / `open_in_memory()` 단일 진실 소스 (lesson 26).
///
/// 새 테이블/인덱스 추가 시 이 상수만 수정. CREATE TABLE/INDEX는 모두 `IF NOT EXISTS`라
/// 기존 DB에도 멱등 적용된다 (마이그레이션 부담 없음).
const SETTINGS_DB_SCHEMA: &str = "
    CREATE TABLE IF NOT EXISTS config (
        section TEXT NOT NULL,
        key     TEXT NOT NULL,
        value   TEXT NOT NULL,
        PRIMARY KEY (section, key)
    );

    CREATE TABLE IF NOT EXISTS doc_types (
        id         TEXT PRIMARY KEY,
        label_ko   TEXT NOT NULL,
        sections   TEXT NOT NULL DEFAULT '[]',
        thresholds TEXT
    );

    CREATE TABLE IF NOT EXISTS prompts (
        id       TEXT PRIMARY KEY,
        template TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS credentials (
        id           TEXT PRIMARY KEY,
        name         TEXT NOT NULL,
        provider     TEXT NOT NULL,
        api_key      TEXT,
        url          TEXT,
        model        TEXT,
        profile_path TEXT
    );

    CREATE TABLE IF NOT EXISTS embedding_meta (
        key   TEXT PRIMARY KEY,
        value TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS todos (
        id              TEXT PRIMARY KEY,
        title           TEXT NOT NULL,
        doc_description TEXT,
        description     TEXT,
        category        TEXT NOT NULL DEFAULT 'uncategorized',
        doc_ids         TEXT NOT NULL DEFAULT '[]',
        doc_type        TEXT,
        status          TEXT NOT NULL DEFAULT 'open',
        fingerprint     TEXT NOT NULL,
        source_line     INTEGER,
        source_text     TEXT,
        created_at      TEXT NOT NULL,
        due_date        TEXT,
        completed_at    TEXT
    );

    CREATE INDEX IF NOT EXISTS idx_todos_status ON todos(status);
    CREATE INDEX IF NOT EXISTS idx_todos_category ON todos(category);
    CREATE UNIQUE INDEX IF NOT EXISTS idx_todos_fingerprint ON todos(fingerprint);
    CREATE INDEX IF NOT EXISTS idx_todos_doc_ids ON todos(doc_ids);
    CREATE INDEX IF NOT EXISTS idx_todos_due_date ON todos(due_date);

    CREATE TABLE IF NOT EXISTS golden_set (
        query      TEXT NOT NULL,
        doc_id     TEXT NOT NULL,
        source     TEXT NOT NULL DEFAULT 'manual',
        created_at TEXT NOT NULL,
        PRIMARY KEY (query, doc_id)
    );

    -- Phase 77: 설정 스냅샷 (apply 전 백업 + apply 후 metrics 추적)
    CREATE TABLE IF NOT EXISTS config_snapshots (
        id              TEXT PRIMARY KEY,
        created_at      TEXT NOT NULL,
        config_hash     TEXT NOT NULL,
        config_backup   TEXT NOT NULL,
        profile_json    TEXT,
        applied_paths   TEXT NOT NULL DEFAULT '[]',
        metrics_json    TEXT,
        rolled_back     INTEGER NOT NULL DEFAULT 0,
        rollback_reason TEXT
    );
    CREATE INDEX IF NOT EXISTS idx_snapshots_created ON config_snapshots(created_at DESC);

    -- Phase 80-A: 검색 mode 누적 카운터 (메모리 카운터 영속화)
    CREATE TABLE IF NOT EXISTS search_mode_counters (
        mode  TEXT PRIMARY KEY,
        count INTEGER NOT NULL DEFAULT 0,
        last_at TEXT
    );

    -- Phase 80-B: CRAG 신뢰도 누적 카운터
    CREATE TABLE IF NOT EXISTS crag_counters (
        bucket TEXT PRIMARY KEY,  -- correct | ambiguous | incorrect
        count  INTEGER NOT NULL DEFAULT 0,
        last_at TEXT
    );

    -- Phase 80-C: 청크 통계 (embed_gen 시점 누적)
    CREATE TABLE IF NOT EXISTS chunk_stats (
        key   TEXT PRIMARY KEY,  -- total_chunks | total_bytes | code_fenced | heading_recognized
        value REAL NOT NULL DEFAULT 0,
        last_at TEXT
    );

    -- Phase 81: 호스트 전처리 도구 감지 결과 캐시 (음성 캐시 포함)
    CREATE TABLE IF NOT EXISTS host_tools_cache (
        tool        TEXT PRIMARY KEY,
        version     TEXT NOT NULL DEFAULT '',
        detected_at TEXT NOT NULL,
        not_found   INTEGER NOT NULL DEFAULT 0,
        install_hint TEXT
    );

    -- Phase 82-prep: 처리 메트릭 누적 (verify_pass_rate / quarantine_rate / avg_process_time_ms 산출용).
    -- record_success/record_error/quarantine 이벤트에 따라 증분. EMA 등 시계열 산출은 조회 시점에 계산.
    CREATE TABLE IF NOT EXISTS processing_metrics (
        key      TEXT PRIMARY KEY,  -- success | errors | verified_pass | verified_fail | quarantined | total_time_ms | counted_for_time
        value    INTEGER NOT NULL DEFAULT 0,
        last_at  TEXT
    );

    -- Phase 82: Decision Log — setup_apply / setup_apply_modules 결정 이력.
    -- 한 번의 apply 호출 = 여러 ConfigChange 후보 → 각 항목별로 1 row (accepted/rejected/critical_skipped 마킹).
    -- snapshot_id로 Phase 77 ConfigSnapshot과 연결. rejected 항목도 기록해 향후 거부 패턴 분석 가능.
    CREATE TABLE IF NOT EXISTS decision_log (
        id            INTEGER PRIMARY KEY AUTOINCREMENT,
        decided_at    TEXT NOT NULL,
        source        TEXT NOT NULL,  -- setup_review | setup_modules
        snapshot_id   TEXT,            -- 적용 성공 시 ConfigSnapshot.id (NULL=거부/전체실패)
        path          TEXT NOT NULL,
        decision      TEXT NOT NULL,  -- accepted | rejected | critical_skipped
        before_value  TEXT,            -- JSON 직렬화 (NULL=값 없음)
        after_value   TEXT,            -- JSON 직렬화 (NULL=값 없음)
        priority      TEXT,            -- P0 | P1 | P2
        risk          TEXT,            -- low | medium | high | critical
        evidence      TEXT,            -- heuristic | benchmark | literature | user_feedback
        confidence    TEXT,            -- low | medium | high
        reason        TEXT,            -- ConfigChange.reason (룰 출처)
        context       TEXT             -- JSON 임의 메타 (module_ids/scenario 등)
    );
    CREATE INDEX IF NOT EXISTS idx_decision_log_at ON decision_log(decided_at DESC);
    CREATE INDEX IF NOT EXISTS idx_decision_log_snapshot ON decision_log(snapshot_id);
    CREATE INDEX IF NOT EXISTS idx_decision_log_path ON decision_log(path);

    -- A1 (Ruflo ReasoningBank 차용): LLM 가공 결과 캐시.
    -- 같은 파일을 두 번 가공하면 LLM 호출 스킵 (lesson 70 — 파일당 10~20초 병목 해소).
    -- file_hash = SHA-256(원본 내용). content_hash = SHA-256(가공 결과 JSON). hits = 누적 사용 횟수.
    CREATE TABLE IF NOT EXISTS llm_cache (
        file_hash    TEXT PRIMARY KEY,
        content_hash TEXT NOT NULL,
        result_json  TEXT NOT NULL,
        doc_types    TEXT NOT NULL DEFAULT '',
        hits         INTEGER NOT NULL DEFAULT 0,
        created_at   TEXT NOT NULL,
        last_hit_at  TEXT
    );
    CREATE INDEX IF NOT EXISTS idx_llm_cache_doc_types ON llm_cache(doc_types);

    -- C1 룰 임계값: auto_suggester가 사용하는 4개 임계값 (없으면 코드 디폴트).
    -- 사용자가 GUI에서 조정 가능. 디폴트 값은 코드(auto_suggester.rs)에 정의.
    CREATE TABLE IF NOT EXISTS c1_rule_thresholds (
        key   TEXT PRIMARY KEY,
        value REAL NOT NULL
    );

    -- C2 PII 사용자 정의 패턴: 코드의 5종 디폴트 외 추가 패턴.
    -- enabled=0이면 비활성. pattern은 valid regex여야 함 (가입 시 검증).
    CREATE TABLE IF NOT EXISTS pii_patterns_user (
        name      TEXT PRIMARY KEY,
        pattern   TEXT NOT NULL,
        enabled   INTEGER NOT NULL DEFAULT 1,
        created_at TEXT NOT NULL
    );

    -- MCP 도구 비활성화 목록. 존재하는 행 = 비활성. call_tool/list_tools에서 차단.
    -- 도구 이름은 mcp_server.rs의 match 분기 이름과 동일.
    CREATE TABLE IF NOT EXISTS mcp_disabled_tools (
        tool_name TEXT PRIMARY KEY,
        disabled_at TEXT NOT NULL,
        reason TEXT
    );

    -- A1 LLM 캐시 GC 이력. UI stat 카드에 마지막 GC 시각/삭제 건수 노출.
    -- 누적 행을 두지 않고 단일 행(id=1) upsert로 유지하여 무한 누적 차단.
    CREATE TABLE IF NOT EXISTS llm_cache_gc_log (
        id INTEGER PRIMARY KEY CHECK (id = 1),
        last_at TEXT NOT NULL,
        last_deleted INTEGER NOT NULL
    );

    -- Phase 91 A3: trace_id 단일 키 감사 추적 테이블.
    -- 모든 LLM/검색/MCP/검증 결정 1줄 기록. scripts/replay_trace.sh로 trace_id 단위 재구성.
    -- inputs_hash: 입력의 SHA-256 (16자 prefix, 디버깅용). output_summary: 결과 요약 200자.
    CREATE TABLE IF NOT EXISTS audit_trace (
        id              INTEGER PRIMARY KEY AUTOINCREMENT,
        trace_id        TEXT NOT NULL,
        stage           TEXT NOT NULL,
        inputs_hash     TEXT,
        output_summary  TEXT,
        applied_rule    TEXT,
        created_at      TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
    );
    CREATE INDEX IF NOT EXISTS idx_audit_trace_id ON audit_trace(trace_id);
    CREATE INDEX IF NOT EXISTS idx_audit_trace_created ON audit_trace(created_at DESC);

    -- step-o3 (2026-06-17, outbound-umbrella-1): telegram_storage 어댑터 message → file/document 매핑.
    -- bot API 가 자기 발송 history 자동 조회 부재 → list/download 시점 본 table 외부 매핑 의무.
    -- 48시간 후 delete 불가 (bot API 제약) → ts 보존, delete 시점 본 row 조회 + 48h 검증.
    CREATE TABLE IF NOT EXISTS telegram_message_map (
        remote_key      TEXT PRIMARY KEY,
        message_id      INTEGER NOT NULL,
        file_id         TEXT,
        chat_id         TEXT NOT NULL,
        mode            TEXT NOT NULL DEFAULT 'document',
        size_bytes      INTEGER,
        ts              TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
    );
    CREATE INDEX IF NOT EXISTS idx_tg_msg_chat ON telegram_message_map(chat_id);
    CREATE INDEX IF NOT EXISTS idx_tg_msg_ts ON telegram_message_map(ts DESC);
";

// prep-1 (2026-06-16, settings-db-split-1): 6 도메인 struct → file_pipeline_core::domain::settings_models 이전.
// 외부 호출처 backward compat 정합 re-export.
pub use file_pipeline_core::domain::settings_models::{
    AuditEventRow, DecisionLogEntry, HostToolCacheRow, LlmCacheEntry, NewTodo, ProcessingMetricSummary,
};

/// SQLite 기반 설정 DB
pub struct SettingsDb {
    conn: Mutex<Connection>,
    path: PathBuf,
}

impl SettingsDb {
    /// DB 열기 (없으면 생성 + 스키마 초기화)
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)
            .with_context(|| format!("settings.db 열기 실패: {}", path.display()))?;

        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        conn.execute_batch(SETTINGS_DB_SCHEMA)?;

        Ok(Self {
            conn: Mutex::new(conn),
            path: path.to_path_buf(),
        })
    }

    /// in-memory DB (테스트용)
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()
            .with_context(|| "in-memory DB 열기 실패")?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;
        conn.execute_batch(SETTINGS_DB_SCHEMA)?;
        Ok(Self {
            conn: Mutex::new(conn),
            path: PathBuf::from(":memory:"),
        })
    }

    // `open_or_migrate` (부팅 toml 마이그레이션) 은 shared 자유함수로 잔류:
    // `file_pipeline_shared::settings_db::open_or_migrate`. cycle 회피 — `PipelineConfigExt::load`
    // / `load_doc_type_registry` 가 shared 소속이므로 adapters 측에서 호출 불가.

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// 테이블에 데이터가 있는지 확인
    ///
    /// shared `open_or_migrate` 자유함수가 마이그레이션 분기에서 사용하므로 `pub`.
    pub fn has_data_in(&self, table: &str) -> Result<bool> {
        let conn = self.conn.lock().expect("mutex poisoned");
        // 테이블명은 내부 상수만 허용 (SQL injection 방지)
        let query = match table {
            "config" => "SELECT COUNT(*) FROM config",
            "doc_types" => "SELECT COUNT(*) FROM doc_types",
            "prompts" => "SELECT COUNT(*) FROM prompts",
            "credentials" => "SELECT COUNT(*) FROM credentials",
            _ => return Ok(false),
        };
        let count: i64 = conn.query_row(query, [], |row| row.get(0))?;
        Ok(count > 0)
    }

    // ═══════════════════════════════════════════════════════════
    // config 테이블 (key-value) + 타입 안전 래퍼
    // ═══════════════════════════════════════════════════════════

    /// 섹션 전체를 타입 안전하게 역직렬화
    pub fn get_section_as<T: serde::de::DeserializeOwned>(&self, section: &str) -> Result<T> {
        let pairs = self.get_section(section)?;
        let mut map = serde_json::Map::new();
        for (key, value_str) in pairs {
            let value: serde_json::Value =
                serde_json::from_str(&value_str).unwrap_or(serde_json::Value::String(value_str));
            map.insert(key, value);
        }
        serde_json::from_value(serde_json::Value::Object(map))
            .with_context(|| format!("섹션 '{}' 역직렬화 실패", section))
    }

    /// 개별 값을 타입 안전하게 역직렬화
    pub fn get_config_as<T: serde::de::DeserializeOwned>(&self, section: &str, key: &str) -> Result<Option<T>> {
        match self.get_config(section, key)? {
            Some(value_str) => {
                let value: T = serde_json::from_str(&value_str)
                    .with_context(|| format!("설정 '{}.{}' 역직렬화 실패", section, key))?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    /// 설정 값 조회
    pub fn get_config(&self, section: &str, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().expect("mutex poisoned");
        let mut stmt = conn.prepare("SELECT value FROM config WHERE section = ?1 AND key = ?2")?;
        let result = stmt
            .query_row(params![section, key], |row| row.get::<_, String>(0))
            .ok();
        Ok(result)
    }

    /// 설정 값 저장 (upsert)
    pub fn set_config(&self, section: &str, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock().expect("mutex poisoned");
        conn.execute(
            "INSERT OR REPLACE INTO config (section, key, value) VALUES (?1, ?2, ?3)",
            params![section, key, value],
        )?;
        Ok(())
    }

    /// 섹션 전체 조회
    pub fn get_section(&self, section: &str) -> Result<Vec<(String, String)>> {
        let conn = self.conn.lock().expect("mutex poisoned");
        let mut stmt = conn.prepare("SELECT key, value FROM config WHERE section = ?1")?;
        let rows = stmt
            .query_map(params![section], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    /// 모든 설정 조회 (section → key → value)
    pub fn get_all_config(&self) -> Result<std::collections::HashMap<String, std::collections::HashMap<String, serde_json::Value>>> {
        let conn = self.conn.lock().expect("mutex poisoned");
        let mut stmt = conn.prepare("SELECT section, key, value FROM config ORDER BY section, key")?;
        let mut result: std::collections::HashMap<String, std::collections::HashMap<String, serde_json::Value>> =
            std::collections::HashMap::new();
        let rows = stmt.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?;
        for row in rows {
            let (section, key, value_str) = row?;
            let value: serde_json::Value =
                serde_json::from_str(&value_str).unwrap_or(serde_json::Value::String(value_str));
            result
                .entry(section)
                .or_default()
                .insert(key, value);
        }
        Ok(result)
    }

    /// 섹션 전체 삭제
    pub fn delete_section(&self, section: &str) -> Result<usize> {
        let conn = self.conn.lock().expect("mutex poisoned");
        let count = conn.execute("DELETE FROM config WHERE section = ?1", params![section])?;
        Ok(count)
    }

    // ═══════════════════════════════════════════════════════════
    // doc_types 테이블
    // ═══════════════════════════════════════════════════════════

    /// 문서 유형 전체 조회
    pub fn list_doc_types(&self) -> Result<Vec<DocTypeDef>> {
        let conn = self.conn.lock().expect("mutex poisoned");
        let mut stmt = conn.prepare("SELECT id, label_ko, sections, thresholds FROM doc_types ORDER BY id")?;
        let rows = stmt
            .query_map([], |row| {
                let id: String = row.get(0)?;
                let label_ko: String = row.get(1)?;
                let sections_json: String = row.get(2)?;
                let thresholds_json: Option<String> = row.get(3)?;
                Ok((id, label_ko, sections_json, thresholds_json))
            })?
            .filter_map(|r| r.ok())
            .map(|(id, label_ko, sections_json, thresholds_json)| {
                let sections: Vec<String> =
                    serde_json::from_str(&sections_json).unwrap_or_default();
                let thresholds = thresholds_json
                    .and_then(|s| serde_json::from_str(&s).ok());
                DocTypeDef {
                    id,
                    label_ko,
                    patterns: vec![],
                    sections,
                    prompt: String::new(),
                    dedup_key: None,
                    sensitive: false,
                    thresholds,
                }
            })
            .collect();
        Ok(rows)
    }

    /// 문서 유형 저장 (upsert)
    pub fn save_doc_type(&self, dt: &DocTypeDef) -> Result<()> {
        let conn = self.conn.lock().expect("mutex poisoned");
        let sections_json = serde_json::to_string(&dt.sections)?;
        let thresholds_json = dt.thresholds.as_ref().map(serde_json::to_string).transpose()?;
        conn.execute(
            "INSERT OR REPLACE INTO doc_types (id, label_ko, sections, thresholds) VALUES (?1, ?2, ?3, ?4)",
            params![dt.id, dt.label_ko, sections_json, thresholds_json],
        )?;
        Ok(())
    }

    /// 문서 유형 삭제
    pub fn delete_doc_type(&self, id: &str) -> Result<bool> {
        let conn = self.conn.lock().expect("mutex poisoned");
        let count = conn.execute("DELETE FROM doc_types WHERE id = ?1", params![id])?;
        Ok(count > 0)
    }

    // ═══════════════════════════════════════════════════════════
    // prompts 테이블
    // ═══════════════════════════════════════════════════════════

    /// 프롬프트 조회
    pub fn get_prompt(&self, id: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().expect("mutex poisoned");
        let result = conn
            .query_row(
                "SELECT template FROM prompts WHERE id = ?1",
                params![id],
                |row| row.get::<_, String>(0),
            )
            .ok();
        Ok(result)
    }

    /// 프롬프트 저장 (upsert)
    pub fn set_prompt(&self, id: &str, template: &str) -> Result<()> {
        let conn = self.conn.lock().expect("mutex poisoned");
        conn.execute(
            "INSERT OR REPLACE INTO prompts (id, template) VALUES (?1, ?2)",
            params![id, template],
        )?;
        Ok(())
    }

    /// 모든 프롬프트 조회
    pub fn list_prompts(&self) -> Result<Vec<(String, String)>> {
        let conn = self.conn.lock().expect("mutex poisoned");
        let mut stmt = conn.prepare("SELECT id, template FROM prompts ORDER BY id")?;
        let rows = stmt
            .query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    // ═══════════════════════════════════════════════════════════
    // credentials 테이블
    // ═══════════════════════════════════════════════════════════

    /// 크레덴셜 전체 조회
    pub fn list_credentials(&self) -> Result<Vec<LlmCredential>> {
        let conn = self.conn.lock().expect("mutex poisoned");
        let mut stmt = conn.prepare(
            "SELECT id, name, provider, api_key, url, model, profile_path FROM credentials ORDER BY name",
        )?;
        let rows = stmt
            .query_map([], |row| {
                Ok(LlmCredential {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    provider: row.get(2)?,
                    api_key: row.get(3)?,
                    url: row.get(4)?,
                    model: row.get(5)?,
                    profile_path: row.get(6)?,
                })
            })?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    /// 크레덴셜 저장 (upsert)
    pub fn save_credential(&self, cred: &LlmCredential) -> Result<()> {
        let conn = self.conn.lock().expect("mutex poisoned");
        conn.execute(
            "INSERT OR REPLACE INTO credentials (id, name, provider, api_key, url, model, profile_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![cred.id, cred.name, cred.provider, cred.api_key, cred.url, cred.model, cred.profile_path],
        )?;
        Ok(())
    }

    /// 크레덴셜 삭제
    pub fn delete_credential(&self, id: &str) -> Result<bool> {
        let conn = self.conn.lock().expect("mutex poisoned");
        let count = conn.execute("DELETE FROM credentials WHERE id = ?1", params![id])?;
        Ok(count > 0)
    }

    // ═══════════════════════════════════════════════════════════
    // todos 테이블
    // ═══════════════════════════════════════════════════════════

    /// todo 추가 (fingerprint 중복 시 doc_ids에 doc_id 추가)
    ///
    /// 입력 필드는 [`NewTodo`] 참조.
    pub fn add_todo(&self, todo: NewTodo<'_>) -> Result<Option<String>> {
        let NewTodo {
            title, category, doc_id, doc_description,
            fingerprint, source_line, source_text, due_date,
        } = todo;
        let conn = self.conn.lock().expect("mutex poisoned");
        let id = format!("todo_{}", &fingerprint[..12.min(fingerprint.len())]);
        let now = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
        let doc_ids_json = doc_id.map(|d| format!("[\"{}\"]", d)).unwrap_or_else(|| "[]".to_string());

        // fingerprint 중복 시: 기존 row의 doc_ids에 doc_id 추가
        let existing: Option<String> = conn.query_row(
            "SELECT doc_ids FROM todos WHERE fingerprint = ?1", params![fingerprint],
            |row| row.get(0),
        ).ok();

        if let Some(existing_json) = existing {
            // 기존 todo에 새 doc_id 추가
            if let Some(did) = doc_id {
                let mut ids: Vec<String> = serde_json::from_str(&existing_json).unwrap_or_default();
                if !ids.contains(&did.to_string()) {
                    ids.push(did.to_string());
                    let updated = serde_json::to_string(&ids)?;
                    conn.execute(
                        "UPDATE todos SET doc_ids = ?1 WHERE fingerprint = ?2",
                        params![updated, fingerprint],
                    )?;
                }
            }
            Ok(None) // 기존 항목에 추가됨
        } else {
            // 신규 todo 삽입
            conn.execute(
                "INSERT INTO todos (id, title, doc_description, category, doc_ids, status, fingerprint, source_line, source_text, created_at, due_date)
                 VALUES (?1, ?2, ?3, ?4, ?5, 'open', ?6, ?7, ?8, ?9, ?10)",
                params![id, title, doc_description, category, doc_ids_json, fingerprint, source_line, source_text, now, due_date],
            )?;
            Ok(Some(id))
        }
    }

    /// todo 목록 조회 (status 필터)
    pub fn list_todos(&self, status: Option<&str>, category: Option<&str>) -> Result<Vec<serde_json::Value>> {
        let conn = self.conn.lock().expect("mutex poisoned");
        let mut query = "SELECT id, title, doc_description, description, category, doc_ids, doc_type, status, source_text, created_at, due_date, completed_at FROM todos".to_string();
        let mut conditions = Vec::new();
        if let Some(s) = status { if s != "all" { conditions.push(format!("status = '{}'", s)); } }
        if let Some(c) = category { conditions.push(format!("category = '{}'", c)); }
        if !conditions.is_empty() { query.push_str(&format!(" WHERE {}", conditions.join(" AND "))); }
        query.push_str(" ORDER BY created_at DESC");

        let mut stmt = conn.prepare(&query)?;
        let rows = stmt.query_map([], |row| {
            Ok(serde_json::json!({
                "id": row.get::<_, String>(0)?,
                "title": row.get::<_, String>(1)?,
                "doc_description": row.get::<_, Option<String>>(2)?,
                "description": row.get::<_, Option<String>>(3)?,
                "category": row.get::<_, String>(4)?,
                "doc_ids": row.get::<_, String>(5)?,
                "doc_type": row.get::<_, Option<String>>(6)?,
                "status": row.get::<_, String>(7)?,
                "source_text": row.get::<_, Option<String>>(8)?,
                "created_at": row.get::<_, String>(9)?,
                "due_date": row.get::<_, Option<String>>(10)?,
                "completed_at": row.get::<_, Option<String>>(11)?,
            }))
        })?.filter_map(|r| r.ok()).collect();
        Ok(rows)
    }

    /// todo 완료 처리
    pub fn complete_todo(&self, id: &str) -> Result<bool> {
        let conn = self.conn.lock().expect("mutex poisoned");
        let now = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
        let count = conn.execute(
            "UPDATE todos SET status = 'done', completed_at = ?1 WHERE id = ?2 AND status = 'open'",
            params![now, id],
        )?;
        Ok(count > 0)
    }

    /// todo 스킵 처리
    pub fn skip_todo(&self, id: &str, reason: Option<&str>) -> Result<bool> {
        let conn = self.conn.lock().expect("mutex poisoned");
        let now = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
        let desc = reason.map(|r| format!("[skipped] {}", r));
        let count = conn.execute(
            "UPDATE todos SET status = 'skip', completed_at = ?1, description = COALESCE(?2, description) WHERE id = ?3 AND status = 'open'",
            params![now, desc, id],
        )?;
        Ok(count > 0)
    }

    /// todo 재오픈
    pub fn reopen_todo(&self, id: &str) -> Result<bool> {
        let conn = self.conn.lock().expect("mutex poisoned");
        let count = conn.execute(
            "UPDATE todos SET status = 'open', completed_at = NULL WHERE id = ?1 AND status IN ('done', 'skip')",
            params![id],
        )?;
        Ok(count > 0)
    }

    /// todo 수정
    pub fn update_todo(&self, id: &str, description: Option<&str>, due_date: Option<&str>, category: Option<&str>) -> Result<bool> {
        let conn = self.conn.lock().expect("mutex poisoned");
        let mut updates = Vec::new();
        let mut values: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        if let Some(d) = description { updates.push("description = ?"); values.push(Box::new(d.to_string())); }
        if let Some(d) = due_date { updates.push("due_date = ?"); values.push(Box::new(d.to_string())); }
        if let Some(c) = category { updates.push("category = ?"); values.push(Box::new(c.to_string())); }
        if updates.is_empty() { return Ok(false); }
        values.push(Box::new(id.to_string()));
        let sql = format!("UPDATE todos SET {} WHERE id = ?", updates.join(", "));
        let params: Vec<&dyn rusqlite::ToSql> = values.iter().map(|v| v.as_ref()).collect();
        let count = conn.execute(&sql, params.as_slice())?;
        Ok(count > 0)
    }

    /// todo 수
    pub fn todo_count(&self, status: Option<&str>) -> Result<usize> {
        let conn = self.conn.lock().expect("mutex poisoned");
        let query = match status {
            Some("all") | None => "SELECT COUNT(*) FROM todos".to_string(),
            Some(s) => format!("SELECT COUNT(*) FROM todos WHERE status = '{}'", s),
        };
        let count: i64 = conn.query_row(&query, [], |row| row.get(0))?;
        Ok(count as usize)
    }

    /// 문서에서 todo 자동 추출 (키워드 패턴 매칭)
    pub fn extract_todos_from_text(&self, doc_id: &str, text: &str, file_path: &str, doc_description: Option<&str>) -> Result<usize> {
        use sha2::{Digest, Sha256};
        let patterns: &[(&str, &str)] = &[
            ("TODO", "TODO"), ("FIXME", "FIXME"), ("HACK", "HACK"), ("XXX", "XXX"),
        ];
        let ko_patterns: &[(&str, &str)] = &[
            ("할 일", "할일"), ("할 것", "할것"), ("검토 필요", "검토필요"),
            ("확인 바람", "확인바람"), ("액션 아이템", "액션아이템"),
        ];

        // category 추출: 파일 경로의 2번째 레벨
        let category = std::path::Path::new(file_path)
            .components()
            .filter(|c| matches!(c, std::path::Component::Normal(_)))
            .nth(1)
            .map(|c| c.as_os_str().to_string_lossy().to_string())
            .unwrap_or_else(|| "uncategorized".to_string());

        let mut count = 0;
        for (line_num, line) in text.lines().enumerate() {
            let trimmed = line.trim();

            // 마크다운 체크박스 (미완료)
            if trimmed.starts_with("- [ ] ") {
                let title = trimmed.trim_start_matches("- [ ] ").trim();
                if !title.is_empty() {
                    let mut hasher = Sha256::new();
                    hasher.update(format!("{}:{}", doc_id, title.to_lowercase()).as_bytes());
                    let fp = hex::encode(hasher.finalize());
                    if self.add_todo(NewTodo {
                        title, category: &category, doc_id: Some(doc_id),
                        doc_description, fingerprint: &fp,
                        source_line: Some(line_num as i64), source_text: Some(trimmed),
                        due_date: None,
                    })?.is_some() {
                        count += 1;
                    }
                }
                continue;
            }

            // 영문 패턴
            for (pattern, _label) in patterns {
                if let Some(pos) = trimmed.to_uppercase().find(pattern) {
                    let after = &trimmed[pos + pattern.len()..];
                    let title = after.trim_start_matches([':', ' ']).trim();
                    if title.len() >= 3 {
                        let mut hasher = Sha256::new();
                        hasher.update(format!("{}:{}", doc_id, title.to_lowercase()).as_bytes());
                        let fp = hex::encode(hasher.finalize());
                        if self.add_todo(NewTodo {
                            title, category: &category, doc_id: Some(doc_id),
                            doc_description, fingerprint: &fp,
                            source_line: Some(line_num as i64), source_text: Some(trimmed),
                            due_date: None,
                        })?.is_some() {
                            count += 1;
                        }
                    }
                    break;
                }
            }

            // 한글 패턴
            for (pattern, _label) in ko_patterns {
                if trimmed.contains(pattern) {
                    let title = trimmed.trim();
                    if title.len() >= 5 {
                        let mut hasher = Sha256::new();
                        hasher.update(format!("{}:{}", doc_id, title.to_lowercase()).as_bytes());
                        let fp = hex::encode(hasher.finalize());
                        if self.add_todo(NewTodo {
                            title, category: &category, doc_id: Some(doc_id),
                            doc_description, fingerprint: &fp,
                            source_line: Some(line_num as i64), source_text: Some(trimmed),
                            due_date: None,
                        })?.is_some() {
                            count += 1;
                        }
                    }
                    break;
                }
            }
        }
        Ok(count)
    }

    // ═══════════════════════════════════════════════════════════
    // embedding_meta 테이블
    // ═══════════════════════════════════════════════════════════

    /// 임베딩 메타데이터 설정/조회
    pub fn set_embedding_meta(&self, key: &str, value: &str) -> Result<()> {
        let conn = self.conn.lock().expect("mutex poisoned");
        conn.execute(
            "INSERT OR REPLACE INTO embedding_meta (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn get_embedding_meta(&self, key: &str) -> Result<Option<String>> {
        let conn = self.conn.lock().expect("mutex poisoned");
        let result = conn
            .query_row("SELECT value FROM embedding_meta WHERE key = ?1", params![key], |row| row.get::<_, String>(0))
            .ok();
        Ok(result)
    }

    /// 현재 임베딩 설정을 기록 (모델 변경 감지용)
    pub fn record_embedding_config(&self, model: &str, dim: usize) -> Result<()> {
        self.set_embedding_meta("model", model)?;
        self.set_embedding_meta("dim", &dim.to_string())?;
        self.set_embedding_meta("recorded_at", &chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string())?;
        Ok(())
    }

    /// 임베딩 모델 변경 여부 확인. None이면 첫 기록, Some(true)이면 변경됨
    pub fn check_embedding_mismatch(&self, model: &str, dim: usize) -> Result<Option<bool>> {
        let stored_model = self.get_embedding_meta("model")?;
        let stored_dim = self.get_embedding_meta("dim")?;
        match (stored_model, stored_dim) {
            (Some(m), Some(d)) => {
                let mismatch = m != model || d != dim.to_string();
                Ok(Some(mismatch))
            }
            _ => Ok(None), // 첫 기록
        }
    }

    // ═══════════════════════════════════════════════════════════
    // golden_set 테이블 (검색 품질 모니터링)
    // ═══════════════════════════════════════════════════════════

    /// 골든셋 쌍 추가
    pub fn add_golden_pair(&self, query: &str, doc_id: &str, source: &str) -> Result<()> {
        let conn = self.conn.lock().expect("mutex poisoned");
        conn.execute(
            "INSERT OR REPLACE INTO golden_set (query, doc_id, source, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![query, doc_id, source, chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string()],
        )?;
        Ok(())
    }

    /// 골든셋 전체 조회
    pub fn list_golden_set(&self) -> Result<Vec<(String, String)>> {
        let conn = self.conn.lock().expect("mutex poisoned");
        let mut stmt = conn.prepare("SELECT query, doc_id FROM golden_set ORDER BY query")?;
        let rows = stmt
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(rows)
    }

    /// 골든셋 수
    pub fn golden_set_count(&self) -> Result<usize> {
        let conn = self.conn.lock().expect("mutex poisoned");
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM golden_set", [], |row| row.get(0))?;
        Ok(count as usize)
    }

    /// 검색 로그에서 자동 골든셋 구축 (query → 사용된 doc_id 쌍)
    pub fn auto_populate_golden_set(&self, search_logs: &[(String, Vec<String>)]) -> Result<usize> {
        let mut added = 0;
        for (query, doc_ids) in search_logs {
            if let Some(top_id) = doc_ids.first() {
                // 같은 쿼리가 여러 번 사용되면 가장 많이 선택된 doc만
                self.add_golden_pair(query, top_id, "auto_log")?;
                added += 1;
            }
        }
        Ok(added)
    }

    // ═══════════════════════════════════════════════════════════
    // TOML → DB 마이그레이션
    // ═══════════════════════════════════════════════════════════

    /// PipelineConfig를 DB에 마이그레이션
    pub fn migrate_from_config(&self, config: &PipelineConfig) -> Result<()> {
        let json = serde_json::to_value(config)?;
        if let serde_json::Value::Object(map) = json {
            for (section, value) in map {
                match value {
                    serde_json::Value::Object(fields) => {
                        for (key, val) in fields {
                            self.set_config(&section, &key, &serde_json::to_string(&val)?)?;
                        }
                    }
                    // 최상위 스칼라 (version, max_workers 등)
                    _ => {
                        self.set_config("_root", &section, &serde_json::to_string(&value)?)?;
                    }
                }
            }
        }

        // credentials는 별도 테이블로
        for cred in &config.credentials {
            self.save_credential(cred)?;
        }

        Ok(())
    }

    /// DocTypeDef 목록을 DB에 마이그레이션
    pub fn migrate_from_doc_types(&self, types: &[DocTypeDef]) -> Result<()> {
        for dt in types {
            self.save_doc_type(dt)?;
        }
        Ok(())
    }

    /// 프롬프트를 DB에 마이그레이션 (TOML 문자열에서)
    pub fn migrate_from_prompts_toml(&self, toml_content: &str) -> Result<()> {
        let table: toml::Value = toml_content.parse()
            .with_context(|| "프롬프트 TOML 파싱 실패")?;
        if let Some(classify) = table.get("classify").and_then(|v| v.get("template")).and_then(|v| v.as_str()) {
            self.set_prompt("classify", classify)?;
        }
        if let Some(reprocess) = table.get("reprocess").and_then(|v| v.get("suffix")).and_then(|v| v.as_str()) {
            self.set_prompt("reprocess_suffix", reprocess)?;
        }
        if let Some(merge) = table.get("summarize_text").and_then(|v| v.get("template")).and_then(|v| v.as_str()) {
            self.set_prompt("summarize_text", merge)?;
        }
        Ok(())
    }

    /// DB에서 PipelineConfig 복원
    pub fn to_pipeline_config(&self) -> Result<PipelineConfig> {
        let all = self.get_all_config()?;
        // 전체를 하나의 JSON object로 재구성
        let mut root = serde_json::Map::new();
        for (section, fields) in &all {
            if section == "_root" {
                for (k, v) in fields {
                    root.insert(k.clone(), v.clone());
                }
            } else {
                root.insert(section.clone(), serde_json::Value::Object(
                    fields.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
                ));
            }
        }
        // credentials 복원
        let creds = self.list_credentials()?;
        root.insert("credentials".into(), serde_json::to_value(&creds)?);

        let config: PipelineConfig = serde_json::from_value(serde_json::Value::Object(root))
            .unwrap_or_else(|e| {
                tracing::warn!("DB→PipelineConfig 복원 실패, 기본값 사용: {}", e);
                PipelineConfig::default_config()
            });
        Ok(config)
    }

    /// DB에서 DocTypeRegistry 복원
    pub fn to_doc_type_registry(&self) -> Result<file_pipeline_core::domain::models::DocTypeRegistry> {
        let types = self.list_doc_types()?;
        Ok(file_pipeline_core::domain::models::DocTypeRegistry::new(types))
    }

    // ── Phase 77: ConfigSnapshot CRUD ─────────────────────────

    pub fn save_snapshot(&self, snap: &ConfigSnapshot) -> Result<()> {
        let conn = self.conn.lock().expect("settings.db lock");
        conn.execute(
            "INSERT OR REPLACE INTO config_snapshots
             (id, created_at, config_hash, config_backup, profile_json, applied_paths, metrics_json, rolled_back, rollback_reason)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                &snap.id,
                &snap.created_at,
                &snap.config_hash,
                &snap.config_backup,
                snap.profile_json.as_deref(),
                &snap.applied_paths_json(),
                snap.metrics_json.as_deref(),
                snap.rolled_back as i32,
                snap.rollback_reason.as_deref(),
            ],
        ).context("snapshot 저장 실패")?;
        Ok(())
    }

    pub fn list_snapshots(&self, limit: usize) -> Result<Vec<ConfigSnapshot>> {
        let conn = self.conn.lock().expect("settings.db lock");
        let mut stmt = conn.prepare(
            "SELECT id, created_at, config_hash, config_backup, profile_json, applied_paths, metrics_json, rolled_back, rollback_reason
             FROM config_snapshots ORDER BY created_at DESC LIMIT ?1"
        )?;
        let rows = stmt.query_map([limit as i64], |row| {
            let applied_paths: String = row.get(5)?;
            Ok(ConfigSnapshot {
                id: row.get(0)?,
                created_at: row.get(1)?,
                config_hash: row.get(2)?,
                config_backup: row.get(3)?,
                profile_json: row.get(4)?,
                applied_paths: serde_json::from_str(&applied_paths).unwrap_or_default(),
                metrics_json: row.get(6)?,
                rolled_back: { let v: i32 = row.get(7)?; v != 0 },
                rollback_reason: row.get(8)?,
            })
        })?;
        let mut out = Vec::new();
        for r in rows { out.push(r?); }
        Ok(out)
    }

    pub fn get_snapshot(&self, id: &str) -> Result<Option<ConfigSnapshot>> {
        let conn = self.conn.lock().expect("settings.db lock");
        let mut stmt = conn.prepare(
            "SELECT id, created_at, config_hash, config_backup, profile_json, applied_paths, metrics_json, rolled_back, rollback_reason
             FROM config_snapshots WHERE id = ?1"
        )?;
        let mut rows = stmt.query([id])?;
        if let Some(row) = rows.next()? {
            let applied_paths: String = row.get(5)?;
            Ok(Some(ConfigSnapshot {
                id: row.get(0)?,
                created_at: row.get(1)?,
                config_hash: row.get(2)?,
                config_backup: row.get(3)?,
                profile_json: row.get(4)?,
                applied_paths: serde_json::from_str(&applied_paths).unwrap_or_default(),
                metrics_json: row.get(6)?,
                rolled_back: { let v: i32 = row.get(7)?; v != 0 },
                rollback_reason: row.get(8)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn update_snapshot_metrics(&self, id: &str, metrics_json: &str) -> Result<()> {
        let conn = self.conn.lock().expect("settings.db lock");
        conn.execute(
            "UPDATE config_snapshots SET metrics_json = ?1 WHERE id = ?2",
            rusqlite::params![metrics_json, id],
        )?;
        Ok(())
    }

    pub fn mark_snapshot_rolled_back(&self, id: &str, reason: &str) -> Result<()> {
        let conn = self.conn.lock().expect("settings.db lock");
        conn.execute(
            "UPDATE config_snapshots SET rolled_back = 1, rollback_reason = ?1 WHERE id = ?2",
            rusqlite::params![reason, id],
        )?;
        Ok(())
    }

    // ── Phase 80-A: 검색 mode 카운터 ─────────────────────────

    pub fn increment_search_mode(&self, mode: &str) -> Result<()> {
        let conn = self.conn.lock().expect("settings.db lock");
        let now = chrono::Local::now().to_rfc3339();
        conn.execute(
            "INSERT INTO search_mode_counters (mode, count, last_at) VALUES (?1, 1, ?2)
             ON CONFLICT(mode) DO UPDATE SET count = count + 1, last_at = ?2",
            rusqlite::params![mode, &now],
        )?;
        Ok(())
    }

    pub fn get_search_mode_counters(&self) -> Result<Vec<(String, u64, Option<String>)>> {
        let conn = self.conn.lock().expect("settings.db lock");
        let mut stmt = conn.prepare("SELECT mode, count, last_at FROM search_mode_counters ORDER BY count DESC")?;
        let rows = stmt.query_map([], |row| Ok((
            row.get::<_, String>(0)?,
            row.get::<_, i64>(1)? as u64,
            row.get::<_, Option<String>>(2)?,
        )))?;
        let mut out = Vec::new();
        for r in rows { out.push(r?); }
        Ok(out)
    }

    // ── Phase 80-B: CRAG 카운터 ─────────────────────────────

    pub fn increment_crag(&self, bucket: &str) -> Result<()> {
        let conn = self.conn.lock().expect("settings.db lock");
        let now = chrono::Local::now().to_rfc3339();
        conn.execute(
            "INSERT INTO crag_counters (bucket, count, last_at) VALUES (?1, 1, ?2)
             ON CONFLICT(bucket) DO UPDATE SET count = count + 1, last_at = ?2",
            rusqlite::params![bucket, &now],
        )?;
        Ok(())
    }

    pub fn get_crag_counters(&self) -> Result<Vec<(String, u64, Option<String>)>> {
        let conn = self.conn.lock().expect("settings.db lock");
        let mut stmt = conn.prepare("SELECT bucket, count, last_at FROM crag_counters ORDER BY bucket")?;
        let rows = stmt.query_map([], |row| Ok((
            row.get::<_, String>(0)?,
            row.get::<_, i64>(1)? as u64,
            row.get::<_, Option<String>>(2)?,
        )))?;
        let mut out = Vec::new();
        for r in rows { out.push(r?); }
        Ok(out)
    }

    // ── Phase 80-C: 청크 통계 ───────────────────────────────

    pub fn add_chunk_stat(&self, key: &str, delta: f64) -> Result<()> {
        let conn = self.conn.lock().expect("settings.db lock");
        let now = chrono::Local::now().to_rfc3339();
        conn.execute(
            "INSERT INTO chunk_stats (key, value, last_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET value = value + ?2, last_at = ?3",
            rusqlite::params![key, delta, &now],
        )?;
        Ok(())
    }

    pub fn get_chunk_stats(&self) -> Result<Vec<(String, f64, Option<String>)>> {
        let conn = self.conn.lock().expect("settings.db lock");
        let mut stmt = conn.prepare("SELECT key, value, last_at FROM chunk_stats ORDER BY key")?;
        let rows = stmt.query_map([], |row| Ok((
            row.get::<_, String>(0)?,
            row.get::<_, f64>(1)?,
            row.get::<_, Option<String>>(2)?,
        )))?;
        let mut out = Vec::new();
        for r in rows { out.push(r?); }
        Ok(out)
    }

    // ── Phase 82-prep: 처리 메트릭 카운터 ────────────────────

    /// 메트릭 키를 delta 만큼 증분. key는 SETTINGS_DB_SCHEMA 주석의 7종 중 하나.
    pub fn add_processing_metric(&self, key: &str, delta: i64) -> Result<()> {
        let conn = self.conn.lock().expect("settings.db lock");
        let now = chrono::Local::now().to_rfc3339();
        conn.execute(
            "INSERT INTO processing_metrics (key, value, last_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(key) DO UPDATE SET value = value + ?2, last_at = ?3",
            rusqlite::params![key, delta, &now],
        )?;
        Ok(())
    }

    /// 모든 메트릭 카운터 raw 조회 (key -> value)
    pub fn get_processing_metric_raw(&self) -> Result<std::collections::HashMap<String, i64>> {
        let conn = self.conn.lock().expect("settings.db lock");
        let mut stmt = conn.prepare("SELECT key, value FROM processing_metrics")?;
        let rows = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?)))?;
        let mut out = std::collections::HashMap::new();
        for r in rows { let (k, v) = r?; out.insert(k, v); }
        Ok(out)
    }

    /// 누적 메트릭에서 산출 비율을 조회. 데이터 부족 시 None.
    pub fn get_processing_metric_summary(&self) -> Result<ProcessingMetricSummary> {
        let raw = self.get_processing_metric_raw()?;
        let g = |k: &str| raw.get(k).copied().unwrap_or(0);
        let verified_pass = g("verified_pass");
        let verified_fail = g("verified_fail");
        let verified_total = verified_pass + verified_fail;
        let success = g("success");
        let errors = g("errors");
        let quarantined = g("quarantined");
        let processed_total = success + errors;
        let total_time_ms = g("total_time_ms");
        let counted_for_time = g("counted_for_time");
        Ok(ProcessingMetricSummary {
            verify_pass_rate: if verified_total > 0 {
                Some(verified_pass as f32 / verified_total as f32)
            } else { None },
            quarantine_rate: if processed_total > 0 {
                Some(quarantined as f32 / processed_total as f32)
            } else { None },
            avg_process_time_ms: if counted_for_time > 0 {
                Some((total_time_ms / counted_for_time) as u64)
            } else { None },
            success: success as u64,
            errors: errors as u64,
            quarantined: quarantined as u64,
            verified_pass: verified_pass as u64,
            verified_fail: verified_fail as u64,
            counted_for_time: counted_for_time as u64,
        })
    }

    // ── Phase 82: Decision Log ──────────────────────────────

    /// 결정 1건 기록. setup_apply / setup_apply_modules가 각 ConfigChange에 대해 호출.
    pub fn insert_decision(&self, entry: &DecisionLogEntry) -> Result<i64> {
        let conn = self.conn.lock().expect("settings.db lock");
        conn.execute(
            "INSERT INTO decision_log (
                decided_at, source, snapshot_id, path, decision,
                before_value, after_value, priority, risk, evidence, confidence,
                reason, context
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            rusqlite::params![
                &entry.decided_at,
                &entry.source,
                entry.snapshot_id.as_deref(),
                &entry.path,
                &entry.decision,
                entry.before_value.as_deref(),
                entry.after_value.as_deref(),
                entry.priority.as_deref(),
                entry.risk.as_deref(),
                entry.evidence.as_deref(),
                entry.confidence.as_deref(),
                entry.reason.as_deref(),
                entry.context.as_deref(),
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    /// 최근 결정 조회. limit=0이면 전체.
    pub fn list_decisions(&self, limit: usize) -> Result<Vec<DecisionLogEntry>> {
        let conn = self.conn.lock().expect("settings.db lock");
        let sql = if limit == 0 {
            "SELECT id, decided_at, source, snapshot_id, path, decision,
                    before_value, after_value, priority, risk, evidence, confidence,
                    reason, context
             FROM decision_log ORDER BY decided_at DESC, id DESC".to_string()
        } else {
            format!(
                "SELECT id, decided_at, source, snapshot_id, path, decision,
                        before_value, after_value, priority, risk, evidence, confidence,
                        reason, context
                 FROM decision_log ORDER BY decided_at DESC, id DESC LIMIT {}",
                limit as i64
            )
        };
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map([], |row| Ok(DecisionLogEntry {
            id: Some(row.get(0)?),
            decided_at: row.get(1)?,
            source: row.get(2)?,
            snapshot_id: row.get(3)?,
            path: row.get(4)?,
            decision: row.get(5)?,
            before_value: row.get(6)?,
            after_value: row.get(7)?,
            priority: row.get(8)?,
            risk: row.get(9)?,
            evidence: row.get(10)?,
            confidence: row.get(11)?,
            reason: row.get(12)?,
            context: row.get(13)?,
        }))?;
        let mut out = Vec::new();
        for r in rows { out.push(r?); }
        Ok(out)
    }

    /// snapshot_id로 해당 적용의 모든 결정 조회 (롤백 후 재현 등)
    pub fn list_decisions_by_snapshot(&self, snapshot_id: &str) -> Result<Vec<DecisionLogEntry>> {
        let conn = self.conn.lock().expect("settings.db lock");
        let mut stmt = conn.prepare(
            "SELECT id, decided_at, source, snapshot_id, path, decision,
                    before_value, after_value, priority, risk, evidence, confidence,
                    reason, context
             FROM decision_log WHERE snapshot_id = ?1 ORDER BY id ASC"
        )?;
        let rows = stmt.query_map(rusqlite::params![snapshot_id], |row| Ok(DecisionLogEntry {
            id: Some(row.get(0)?),
            decided_at: row.get(1)?,
            source: row.get(2)?,
            snapshot_id: row.get(3)?,
            path: row.get(4)?,
            decision: row.get(5)?,
            before_value: row.get(6)?,
            after_value: row.get(7)?,
            priority: row.get(8)?,
            risk: row.get(9)?,
            evidence: row.get(10)?,
            confidence: row.get(11)?,
            reason: row.get(12)?,
            context: row.get(13)?,
        }))?;
        let mut out = Vec::new();
        for r in rows { out.push(r?); }
        Ok(out)
    }

    /// 단일 decision_log entry 조회 (id 기반)
    pub fn get_decision(&self, id: i64) -> Result<Option<DecisionLogEntry>> {
        let conn = self.conn.lock().expect("settings.db lock");
        let mut stmt = conn.prepare(
            "SELECT id, decided_at, source, snapshot_id, path, decision,
                    before_value, after_value, priority, risk, evidence, confidence,
                    reason, context
             FROM decision_log WHERE id = ?1"
        )?;
        let row = stmt.query_row([id], |row| Ok(DecisionLogEntry {
            id: Some(row.get(0)?),
            decided_at: row.get(1)?,
            source: row.get(2)?,
            snapshot_id: row.get(3)?,
            path: row.get(4)?,
            decision: row.get(5)?,
            before_value: row.get(6)?,
            after_value: row.get(7)?,
            priority: row.get(8)?,
            risk: row.get(9)?,
            evidence: row.get(10)?,
            confidence: row.get(11)?,
            reason: row.get(12)?,
            context: row.get(13)?,
        })).optional()?;
        Ok(row)
    }

    /// decision_log entry의 decision 컬럼 갱신 (C1 2단계: suggested → accepted/rejected)
    pub fn update_decision_status(&self, id: i64, new_status: &str) -> Result<()> {
        let conn = self.conn.lock().expect("settings.db lock");
        conn.execute(
            "UPDATE decision_log SET decision = ?1 WHERE id = ?2",
            rusqlite::params![new_status, id],
        )?;
        Ok(())
    }

    // ── Phase 81: 호스트 도구 캐시 ──────────────────────────

    /// 캐시 결과 전체 조회 (found + not_found 모두)
    pub fn get_host_tools_cache(&self) -> Result<Vec<HostToolCacheRow>> {
        let conn = self.conn.lock().expect("settings.db lock");
        let mut stmt = conn.prepare(
            "SELECT tool, version, detected_at, not_found, install_hint FROM host_tools_cache ORDER BY tool"
        )?;
        let rows = stmt.query_map([], |row| Ok(HostToolCacheRow {
            tool: row.get(0)?,
            version: row.get(1)?,
            detected_at: row.get(2)?,
            not_found: { let v: i32 = row.get(3)?; v != 0 },
            install_hint: row.get(4)?,
        }))?;
        let mut out = Vec::new();
        for r in rows { out.push(r?); }
        Ok(out)
    }

    pub fn host_tools_cache_count(&self) -> Result<usize> {
        let conn = self.conn.lock().expect("settings.db lock");
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM host_tools_cache", [], |r| r.get(0))?;
        Ok(n as usize)
    }

    /// 캐시 전체 교체 (음성 캐시 포함). refresh 시 호출.
    pub fn replace_host_tools_cache(&self, rows: &[HostToolCacheRow]) -> Result<()> {
        let mut conn = self.conn.lock().expect("settings.db lock");
        let tx = conn.transaction()?;
        tx.execute("DELETE FROM host_tools_cache", [])?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO host_tools_cache (tool, version, detected_at, not_found, install_hint)
                 VALUES (?1, ?2, ?3, ?4, ?5)"
            )?;
            for r in rows {
                stmt.execute(rusqlite::params![
                    &r.tool,
                    &r.version,
                    &r.detected_at,
                    r.not_found as i32,
                    r.install_hint.as_deref(),
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    // ── A1 (Ruflo ReasoningBank 차용): LLM 결과 캐시 ─────────────

    /// file_hash로 캐시 조회. 히트 시 hits++, last_hit_at 갱신.
    pub fn lookup_llm_cache(&self, file_hash: &str) -> Result<Option<LlmCacheEntry>> {
        let conn = self.conn.lock().expect("settings.db lock");
        let now = chrono::Utc::now().to_rfc3339();
        // 1) 조회
        let row: Option<(String, String, String, u64, String)> = conn.query_row(
            "SELECT content_hash, result_json, doc_types, hits, created_at FROM llm_cache WHERE file_hash = ?1",
            [file_hash],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
        ).optional()?;
        // 2) 히트면 카운터 증분
        if row.is_some() {
            conn.execute(
                "UPDATE llm_cache SET hits = hits + 1, last_hit_at = ?2 WHERE file_hash = ?1",
                rusqlite::params![file_hash, &now],
            )?;
        }
        Ok(row.map(|(content_hash, result_json, doc_types, hits, created_at)| LlmCacheEntry {
            file_hash: file_hash.to_string(),
            content_hash,
            result_json,
            doc_types,
            hits: hits + 1,  // 갱신된 값 반영
            created_at,
            last_hit_at: Some(now),
        }))
    }

    /// 신규 가공 결과 저장. file_hash 중복 시 REPLACE.
    pub fn upsert_llm_cache(&self, entry: &LlmCacheEntry) -> Result<()> {
        let conn = self.conn.lock().expect("settings.db lock");
        conn.execute(
            "INSERT OR REPLACE INTO llm_cache (file_hash, content_hash, result_json, doc_types, hits, created_at, last_hit_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                &entry.file_hash,
                &entry.content_hash,
                &entry.result_json,
                &entry.doc_types,
                entry.hits,
                &entry.created_at,
                entry.last_hit_at.as_deref(),
            ],
        )?;
        Ok(())
    }

    /// 캐시 통계 — Dashboard 표시용. (총 항목 수, 총 히트, 평균 히트)
    /// C1: 룰 임계값 조회 (없으면 default)
    pub fn get_c1_threshold(&self, key: &str, default: f64) -> Result<f64> {
        let conn = self.conn.lock().expect("settings.db lock");
        let v: Option<f64> = conn.query_row(
            "SELECT value FROM c1_rule_thresholds WHERE key = ?1",
            [key],
            |row| row.get(0),
        ).optional()?;
        Ok(v.unwrap_or(default))
    }

    /// C1: 룰 임계값 설정 (upsert)
    pub fn set_c1_threshold(&self, key: &str, value: f64) -> Result<()> {
        let conn = self.conn.lock().expect("settings.db lock");
        conn.execute(
            "INSERT OR REPLACE INTO c1_rule_thresholds (key, value) VALUES (?1, ?2)",
            rusqlite::params![key, value],
        )?;
        Ok(())
    }

    /// C1: 모든 룰 임계값 조회 (UI 렌더용)
    pub fn list_c1_thresholds(&self) -> Result<Vec<(String, f64)>> {
        let conn = self.conn.lock().expect("settings.db lock");
        let mut stmt = conn.prepare("SELECT key, value FROM c1_rule_thresholds ORDER BY key")?;
        let rows = stmt.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?)))?;
        let mut out = Vec::new();
        for r in rows { out.push(r?); }
        Ok(out)
    }

    /// C2: 사용자 정의 PII 패턴 목록 (enabled만)
    pub fn list_user_pii_patterns(&self) -> Result<Vec<(String, String, bool)>> {
        let conn = self.conn.lock().expect("settings.db lock");
        let mut stmt = conn.prepare(
            "SELECT name, pattern, enabled FROM pii_patterns_user ORDER BY name"
        )?;
        let rows = stmt.query_map([], |row| Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)? != 0,
        )))?;
        let mut out = Vec::new();
        for r in rows { out.push(r?); }
        Ok(out)
    }

    /// C2: PII 패턴 추가 (regex 사전 검증 권장)
    pub fn add_user_pii_pattern(&self, name: &str, pattern: &str, enabled: bool) -> Result<()> {
        // regex 유효성 검사 (실패 시 에러)
        let _ = regex::Regex::new(pattern).context("regex 컴파일 실패")?;
        let now = chrono::Utc::now().to_rfc3339();
        let conn = self.conn.lock().expect("settings.db lock");
        conn.execute(
            "INSERT OR REPLACE INTO pii_patterns_user (name, pattern, enabled, created_at) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![name, pattern, if enabled { 1 } else { 0 }, now],
        )?;
        Ok(())
    }

    /// C2: PII 패턴 제거
    pub fn remove_user_pii_pattern(&self, name: &str) -> Result<bool> {
        let conn = self.conn.lock().expect("settings.db lock");
        let n = conn.execute("DELETE FROM pii_patterns_user WHERE name = ?1", [name])?;
        Ok(n > 0)
    }

    /// MCP: 현재 비활성화된 도구 이름 목록
    pub fn list_disabled_mcp_tools(&self) -> Result<Vec<String>> {
        let conn = self.conn.lock().expect("settings.db lock");
        let mut stmt = conn.prepare("SELECT tool_name FROM mcp_disabled_tools ORDER BY tool_name")?;
        let names: Vec<String> = stmt
            .query_map([], |r| r.get::<_, String>(0))?
            .filter_map(|r| r.ok())
            .collect();
        Ok(names)
    }

    /// MCP: 도구 비활성화 (이미 있으면 reason만 갱신)
    pub fn disable_mcp_tool(&self, tool_name: &str, reason: Option<&str>) -> Result<()> {
        let conn = self.conn.lock().expect("settings.db lock");
        let now = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();
        conn.execute(
            "INSERT OR REPLACE INTO mcp_disabled_tools (tool_name, disabled_at, reason) VALUES (?1, ?2, ?3)",
            rusqlite::params![tool_name, now, reason],
        )?;
        Ok(())
    }

    /// MCP: 도구 활성화 (행 삭제)
    pub fn enable_mcp_tool(&self, tool_name: &str) -> Result<bool> {
        let conn = self.conn.lock().expect("settings.db lock");
        let n = conn.execute("DELETE FROM mcp_disabled_tools WHERE tool_name = ?1", [tool_name])?;
        Ok(n > 0)
    }

    /// A1: 마지막 LLM 캐시 GC 결과 조회. (at, deleted) Some / None.
    pub fn get_last_llm_cache_gc(&self) -> Result<Option<(String, i64)>> {
        let conn = self.conn.lock().expect("settings.db lock");
        let row: Option<(String, i64)> = conn.query_row(
            "SELECT last_at, last_deleted FROM llm_cache_gc_log WHERE id = 1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        ).ok();
        Ok(row)
    }

    /// Phase 91 A3: 감사 추적 1줄 기록. trace_id로 단위 재구성 가능.
    ///
    /// stage 예: "llm.classify" / "search.hybrid" / "mcp.search" / "verify.run".
    /// inputs_hash는 입력 SHA-256 16자 prefix 권장 (Self::trace_input_hash 헬퍼 사용).
    pub fn record_audit_event(
        &self,
        trace_id: &str,
        stage: &str,
        inputs_hash: Option<&str>,
        output_summary: Option<&str>,
        applied_rule: Option<&str>,
    ) -> Result<()> {
        let conn = self.conn.lock().expect("settings.db lock");
        conn.execute(
            "INSERT INTO audit_trace (trace_id, stage, inputs_hash, output_summary, applied_rule)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![trace_id, stage, inputs_hash, output_summary, applied_rule],
        )?;
        Ok(())
    }

    /// Phase 92 H1: 최근 N건 audit_trace 조회 (이상 패턴 분석용).
    /// 호출처: `audit_anomaly::analyze_recent_audit`.
    pub fn list_recent_audit_events(&self, limit: usize) -> Result<Vec<AuditEventRow>> {
        let conn = self.conn.lock().expect("settings.db lock");
        let mut stmt = conn.prepare(
            "SELECT stage, inputs_hash, output_summary, applied_rule, created_at
             FROM audit_trace ORDER BY id DESC LIMIT ?1",
        )?;
        let rows = stmt.query_map([limit as i64], |r| {
            Ok(AuditEventRow {
                stage: r.get::<_, String>(0)?,
                inputs_hash: r.get::<_, Option<String>>(1)?,
                output_summary: r.get::<_, Option<String>>(2)?,
                applied_rule: r.get::<_, Option<String>>(3)?,
                created_at: r.get::<_, String>(4)?,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// Phase 91 A3: trace_id로 audit_trace 행 조회 (replay_trace.sh 등 진단 도구용).
    pub fn list_audit_by_trace(&self, trace_id: &str) -> Result<Vec<AuditEventRow>> {
        let conn = self.conn.lock().expect("settings.db lock");
        let mut stmt = conn.prepare(
            "SELECT stage, inputs_hash, output_summary, applied_rule, created_at
             FROM audit_trace WHERE trace_id = ?1 ORDER BY id ASC",
        )?;
        let rows = stmt.query_map([trace_id], |r| {
            Ok(AuditEventRow {
                stage: r.get::<_, String>(0)?,
                inputs_hash: r.get::<_, Option<String>>(1)?,
                output_summary: r.get::<_, Option<String>>(2)?,
                applied_rule: r.get::<_, Option<String>>(3)?,
                created_at: r.get::<_, String>(4)?,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    /// A1: 마지막 LLM 캐시 GC 결과 기록 (id=1 단일 행 upsert).
    pub fn record_llm_cache_gc(&self, at: &str, deleted: i64) -> Result<()> {
        let conn = self.conn.lock().expect("settings.db lock");
        conn.execute(
            "INSERT OR REPLACE INTO llm_cache_gc_log (id, last_at, last_deleted) VALUES (1, ?1, ?2)",
            rusqlite::params![at, deleted],
        )?;
        Ok(())
    }

    /// A1: 전체 LLM 캐시 비우기. 반환: 삭제된 행 수.
    pub fn clear_llm_cache(&self) -> Result<usize> {
        let conn = self.conn.lock().expect("settings.db lock");
        let n = conn.execute("DELETE FROM llm_cache", [])?;
        Ok(n)
    }

    /// A1: LRU 가비지 컬렉션 — max_entries 초과 시 last_hit_at NULL → ASC, hits ASC 순으로 삭제.
    /// max_entries=0이면 no-op. 반환: 삭제된 행 수.
    pub fn gc_llm_cache_to(&self, max_entries: u64) -> Result<usize> {
        if max_entries == 0 { return Ok(0); }
        let conn = self.conn.lock().expect("settings.db lock");
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM llm_cache", [], |r| r.get(0))?;
        if (count as u64) <= max_entries { return Ok(0); }
        let to_remove = count as u64 - max_entries;
        let n = conn.execute(
            "DELETE FROM llm_cache WHERE file_hash IN (
                SELECT file_hash FROM llm_cache
                ORDER BY (last_hit_at IS NULL) DESC, last_hit_at ASC, hits ASC
                LIMIT ?1
            )",
            [to_remove as i64],
        )?;
        Ok(n)
    }

    pub fn llm_cache_stats(&self) -> Result<(usize, u64, f32)> {
        let conn = self.conn.lock().expect("settings.db lock");
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM llm_cache", [], |r| r.get(0))?;
        let total_hits: i64 = conn.query_row(
            "SELECT COALESCE(SUM(hits), 0) FROM llm_cache",
            [], |r| r.get(0)
        )?;
        let avg = if count > 0 { total_hits as f32 / count as f32 } else { 0.0 };
        Ok((count as usize, total_hits as u64, avg))
    }

    // step-o3 (2026-06-17, outbound-umbrella-1): telegram_message_map CRUD.
    // bot API 자기 발송 history 자동 조회 부재 → list/download 매핑 + 48h 제약.

    /// telegram 발송 1건 매핑 박힘.
    pub fn add_telegram_message(
        &self,
        remote_key: &str,
        message_id: i64,
        file_id: Option<&str>,
        chat_id: &str,
        mode: &str,
        size_bytes: Option<i64>,
    ) -> Result<()> {
        let conn = self.conn.lock().expect("conn poisoned");
        conn.execute(
            "INSERT OR REPLACE INTO telegram_message_map
             (remote_key, message_id, file_id, chat_id, mode, size_bytes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![remote_key, message_id, file_id, chat_id, mode, size_bytes],
        )?;
        Ok(())
    }

    /// remote_key 기반 매핑 1건 조회 — (message_id, file_id, chat_id, mode, size_bytes, ts).
    pub fn get_telegram_message_by_key(
        &self,
        remote_key: &str,
    ) -> Result<Option<(i64, Option<String>, String, String, Option<i64>, String)>> {
        let conn = self.conn.lock().expect("conn poisoned");
        let mut stmt = conn.prepare(
            "SELECT message_id, file_id, chat_id, mode, size_bytes, ts
             FROM telegram_message_map WHERE remote_key = ?1",
        )?;
        let mut rows = stmt.query(rusqlite::params![remote_key])?;
        if let Some(row) = rows.next()? {
            Ok(Some((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
            )))
        } else {
            Ok(None)
        }
    }

    /// remote_key 기반 매핑 1건 삭제 — 본 메서드는 telegram bot API delete 호출 직후 호출.
    /// 48h 검증 = 호출자 측 ts 비교 후 본 메서드 또는 bail.
    pub fn delete_telegram_message_by_key(&self, remote_key: &str) -> Result<usize> {
        let conn = self.conn.lock().expect("conn poisoned");
        let affected = conn.execute(
            "DELETE FROM telegram_message_map WHERE remote_key = ?1",
            rusqlite::params![remote_key],
        )?;
        Ok(affected)
    }

    /// chat_id + mode 필터 list — list (bot API 한계 회피).
    pub fn list_telegram_messages(
        &self,
        chat_id: &str,
        mode: Option<&str>,
        limit: usize,
    ) -> Result<Vec<(String, i64, Option<String>, String, String)>> {
        let conn = self.conn.lock().expect("conn poisoned");
        let mut out = Vec::new();
        if let Some(m) = mode {
            let mut stmt = conn.prepare(
                "SELECT remote_key, message_id, file_id, mode, ts FROM telegram_message_map
                 WHERE chat_id = ?1 AND mode = ?2 ORDER BY ts DESC LIMIT ?3",
            )?;
            let mut rows = stmt.query(rusqlite::params![chat_id, m, limit as i64])?;
            while let Some(row) = rows.next()? {
                out.push((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?));
            }
        } else {
            let mut stmt = conn.prepare(
                "SELECT remote_key, message_id, file_id, mode, ts FROM telegram_message_map
                 WHERE chat_id = ?1 ORDER BY ts DESC LIMIT ?2",
            )?;
            let mut rows = stmt.query(rusqlite::params![chat_id, limit as i64])?;
            while let Some(row) = rows.next()? {
                out.push((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?));
            }
        }
        Ok(out)
    }
}

// 6 도메인 struct (AuditEventRow / NewTodo / LlmCacheEntry / HostToolCacheRow / DecisionLogEntry /
// ProcessingMetricSummary) = prep-1 (settings-db-split-1, 2026-06-16) 정합
// file_pipeline_core::domain::settings_models 이전 + 본 모듈 상단 re-export 박힘.

// ── prep-3 (settings-db-split-1, 2026-06-17): SettingsDb → 6 sub-trait impl ──────────
//
// SettingsDb 본체가 adapters 로 이전되어 본 impl 도 동반 이전 (cycle 해소 —
// `HostToolRepo::refresh` 가 같은 crate `crate::driven::preprocessing` 직접 참조 가능).
//
// 6 sub-trait (Audit/Todo/Decision/Metric/HostTool/LlmCache) = config-free 영역만.
// Config/Snapshot/Credential 그룹은 config 의존 → port 미정의 (prep-3b 후속).
// lesson #14 R1 (cycle 회피) + #25 (점진 진화) 정합.
use file_pipeline_core::ports::settings_repo::{
    AuditRepo, DecisionRepo, HostToolRepo, LlmCacheRepo, MetricRepo, SettingsRepoPort, TodoRepo,
};

impl AuditRepo for SettingsDb {
    fn record_audit_event(
        &self,
        trace_id: &str,
        stage: &str,
        inputs_hash: Option<&str>,
        output_summary: Option<&str>,
        applied_rule: Option<&str>,
    ) -> Result<()> {
        SettingsDb::record_audit_event(self, trace_id, stage, inputs_hash, output_summary, applied_rule)
    }

    fn list_audit_by_trace(&self, trace_id: &str) -> Result<Vec<AuditEventRow>> {
        SettingsDb::list_audit_by_trace(self, trace_id)
    }
}

impl TodoRepo for SettingsDb {
    fn add_todo(&self, todo: NewTodo<'_>) -> Result<Option<String>> {
        SettingsDb::add_todo(self, todo)
    }

    fn list_todos(&self, status: Option<&str>, category: Option<&str>) -> Result<Vec<serde_json::Value>> {
        SettingsDb::list_todos(self, status, category)
    }

    fn complete_todo(&self, id: &str) -> Result<bool> {
        SettingsDb::complete_todo(self, id)
    }
}

impl DecisionRepo for SettingsDb {
    fn insert_decision(&self, entry: &DecisionLogEntry) -> Result<i64> {
        SettingsDb::insert_decision(self, entry)
    }

    fn list_decisions(&self, limit: usize) -> Result<Vec<DecisionLogEntry>> {
        SettingsDb::list_decisions(self, limit)
    }

    fn list_decisions_by_snapshot(&self, snapshot_id: &str) -> Result<Vec<DecisionLogEntry>> {
        SettingsDb::list_decisions_by_snapshot(self, snapshot_id)
    }
}

impl MetricRepo for SettingsDb {
    fn add_processing_metric(&self, key: &str, delta: i64) -> Result<()> {
        SettingsDb::add_processing_metric(self, key, delta)
    }

    fn get_processing_metric_raw(&self) -> Result<std::collections::HashMap<String, i64>> {
        SettingsDb::get_processing_metric_raw(self)
    }

    fn get_processing_metric_summary(&self) -> Result<ProcessingMetricSummary> {
        SettingsDb::get_processing_metric_summary(self)
    }

    fn get_search_mode_counters(&self) -> Result<Vec<(String, u64, Option<String>)>> {
        SettingsDb::get_search_mode_counters(self)
    }

    fn get_crag_counters(&self) -> Result<Vec<(String, u64, Option<String>)>> {
        SettingsDb::get_crag_counters(self)
    }

    fn get_chunk_stats(&self) -> Result<Vec<(String, f64, Option<String>)>> {
        SettingsDb::get_chunk_stats(self)
    }
}

impl HostToolRepo for SettingsDb {
    /// 캐시 보유 시 반환, 부재 시 즉시 감지 + 저장 + 반환.
    ///
    /// `host_tools_cache::ensure_cached` (shared) 가 adapters HostToolDetector 를 호출하나,
    /// port impl 은 (tool_key, version) 문자열 튜플 계약 → 본 메서드는 cache row 직접 매핑.
    fn ensure_cached(&self) -> Result<Vec<(String, String)>> {
        if SettingsDb::host_tools_cache_count(self)? == 0 {
            return self.refresh();
        }
        Ok(SettingsDb::get_host_tools_cache(self)?
            .into_iter()
            .map(|r| (r.tool, r.version))
            .collect())
    }

    /// 강제 재감지 + 저장 — adapters HostToolDetector::detect_full 직접 호출 (같은 crate).
    fn refresh(&self) -> Result<Vec<(String, String)>> {
        use crate::driven::preprocessing::preprocessor::HostToolDetector;
        let now = chrono::Utc::now().to_rfc3339();
        let full = HostToolDetector::detect_full();
        let rows: Vec<HostToolCacheRow> = full
            .iter()
            .map(|(tool, ver)| HostToolCacheRow {
                tool: tool.as_key().to_string(),
                version: ver.clone().unwrap_or_default(),
                detected_at: now.clone(),
                not_found: ver.is_none(),
                install_hint: None,
            })
            .collect();
        SettingsDb::replace_host_tools_cache(self, &rows)?;
        Ok(rows.into_iter().map(|r| (r.tool, r.version)).collect())
    }

    fn list_host_tools(&self) -> Result<Vec<HostToolCacheRow>> {
        SettingsDb::get_host_tools_cache(self)
    }
}

impl LlmCacheRepo for SettingsDb {
    /// (file_hash, content_hash) 키 조회. settings_db 의 `lookup_llm_cache` 는 file_hash 단독
    /// 키이므로, content_hash 불일치 시 None (가공 결과 변경 = 캐시 무효).
    fn get_llm_cache(&self, file_hash: &str, content_hash: &str) -> Result<Option<LlmCacheEntry>> {
        Ok(SettingsDb::lookup_llm_cache(self, file_hash)?
            .filter(|e| e.content_hash == content_hash))
    }

    fn save_llm_cache(&self, entry: &LlmCacheEntry) -> Result<()> {
        SettingsDb::upsert_llm_cache(self, entry)
    }

    fn gc_llm_cache_to(&self, max_entries: u64) -> Result<usize> {
        SettingsDb::gc_llm_cache_to(self, max_entries)
    }

    fn record_llm_cache_gc(&self, at: &str, deleted: i64) -> Result<()> {
        SettingsDb::record_llm_cache_gc(self, at, deleted)
    }
}

impl SettingsRepoPort for SettingsDb {}


#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> SettingsDb {
        let tmp = tempfile::NamedTempFile::new().expect("tmp");
        SettingsDb::open(tmp.path()).expect("open")
    }

    #[test]
    fn test_config_set_get() {
        let db = setup();
        db.set_config("compression", "zstd_level", "3").expect("set");
        let val = db.get_config("compression", "zstd_level").expect("get");
        assert_eq!(val, Some("3".into()));
    }

    #[test]
    fn test_config_upsert() {
        let db = setup();
        db.set_config("compression", "zstd_level", "3").expect("set");
        db.set_config("compression", "zstd_level", "5").expect("upsert");
        let val = db.get_config("compression", "zstd_level").expect("get");
        assert_eq!(val, Some("5".into()));
    }

    #[test]
    fn test_config_missing_key() {
        let db = setup();
        let val = db.get_config("nonexistent", "key").expect("get");
        assert_eq!(val, None);
    }

    #[test]
    fn test_get_section() {
        let db = setup();
        db.set_config("logging", "level", "\"info\"").expect("set");
        db.set_config("logging", "file", "true").expect("set");
        let section = db.get_section("logging").expect("get_section");
        assert_eq!(section.len(), 2);
    }

    #[test]
    fn test_get_all_config() {
        let db = setup();
        db.set_config("a", "x", "1").expect("set");
        db.set_config("b", "y", "2").expect("set");
        let all = db.get_all_config().expect("get_all");
        assert!(all.contains_key("a"));
        assert!(all.contains_key("b"));
    }

    #[test]
    fn test_doc_type_crud() {
        let db = setup();
        let dt = DocTypeDef {
            id: "meeting".into(),
            label_ko: "회의록".into(),
            patterns: vec![],
            sections: vec!["결정사항".into(), "액션아이템".into()],
            prompt: String::new(),
            dedup_key: None,
            sensitive: false,
            thresholds: None,
        };
        db.save_doc_type(&dt).expect("save");

        let types = db.list_doc_types().expect("list");
        assert_eq!(types.len(), 1);
        assert_eq!(types[0].id, "meeting");
        assert_eq!(types[0].sections.len(), 2);

        // update
        let mut updated = dt.clone();
        updated.label_ko = "회의록(수정)".into();
        db.save_doc_type(&updated).expect("upsert");
        let types = db.list_doc_types().expect("list");
        assert_eq!(types.len(), 1);
        assert_eq!(types[0].label_ko, "회의록(수정)");

        // delete
        assert!(db.delete_doc_type("meeting").expect("delete"));
        assert_eq!(db.list_doc_types().expect("list").len(), 0);
    }

    #[test]
    fn test_prompt_crud() {
        let db = setup();
        db.set_prompt("classify", "분류 프롬프트").expect("set");
        assert_eq!(db.get_prompt("classify").expect("get"), Some("분류 프롬프트".into()));

        db.set_prompt("classify", "수정됨").expect("upsert");
        assert_eq!(db.get_prompt("classify").expect("get"), Some("수정됨".into()));

        let all = db.list_prompts().expect("list");
        assert_eq!(all.len(), 1);
    }

    #[test]
    fn test_credential_crud() {
        let db = setup();
        let cred = LlmCredential {
            id: "c1".into(),
            name: "Claude CLI".into(),
            provider: "claude_cli".into(),
            api_key: None,
            url: None,
            model: Some("sonnet".into()),
            profile_path: None,
        };
        db.save_credential(&cred).expect("save");

        let creds = db.list_credentials().expect("list");
        assert_eq!(creds.len(), 1);
        assert_eq!(creds[0].name, "Claude CLI");

        assert!(db.delete_credential("c1").expect("delete"));
        assert_eq!(db.list_credentials().expect("list").len(), 0);
    }

    #[test]
    fn test_migrate_from_config() {
        let db = setup();
        let config = PipelineConfig::default_config();
        db.migrate_from_config(&config).expect("migrate");

        // compression.zstd_level이 존재하는지 확인
        let val = db.get_config("compression", "zstd_level").expect("get");
        assert!(val.is_some(), "마이그레이션 후 compression.zstd_level 존재");
    }

    #[test]
    fn test_migrate_doc_types() {
        let db = setup();
        let types = vec![
            DocTypeDef {
                id: "meeting".into(), label_ko: "회의록".into(),
                patterns: vec![], sections: vec!["결정사항".into()],
                prompt: String::new(), dedup_key: None, sensitive: false, thresholds: None,
            },
            DocTypeDef {
                id: "study".into(), label_ko: "학습".into(),
                patterns: vec![], sections: vec!["핵심개념".into()],
                prompt: String::new(), dedup_key: None, sensitive: false, thresholds: None,
            },
        ];
        db.migrate_from_doc_types(&types).expect("migrate");
        assert_eq!(db.list_doc_types().expect("list").len(), 2);
    }

    #[test]
    fn test_migrate_prompts_toml() {
        let db = setup();
        let toml = r#"
[classify]
template = "분류 프롬프트"

[reprocess]
suffix = "재가공 접미사"

[summarize_text]
template = "투두 병합"
"#;
        db.migrate_from_prompts_toml(toml).expect("migrate");
        assert_eq!(db.get_prompt("classify").expect("get"), Some("분류 프롬프트".into()));
        assert_eq!(db.get_prompt("reprocess_suffix").expect("get"), Some("재가공 접미사".into()));
        assert_eq!(db.get_prompt("summarize_text").expect("get"), Some("투두 병합".into()));
    }

    #[test]
    fn test_roundtrip_config() {
        let db = setup();
        let original = PipelineConfig::default_config();
        db.migrate_from_config(&original).expect("migrate");
        let restored = db.to_pipeline_config().expect("restore");
        assert_eq!(restored.compression.zstd_level, original.compression.zstd_level);
        assert_eq!(restored.vector_db.dim, original.vector_db.dim);
    }

    #[test]
    fn test_persistence() {
        let tmp = tempfile::NamedTempFile::new().expect("tmp");
        let path = tmp.path().to_path_buf();

        // 1차: 데이터 쓰기
        {
            let db = SettingsDb::open(&path).expect("open");
            db.set_config("test", "key", "value").expect("set");
        }
        // 2차: 재로드
        {
            let db = SettingsDb::open(&path).expect("reopen");
            assert_eq!(db.get_config("test", "key").expect("get"), Some("value".into()));
        }
    }

    #[test]
    fn test_open_in_memory() {
        let db = SettingsDb::open_in_memory().expect("in-memory");
        db.set_config("test", "k", "\"v\"").expect("set");
        assert_eq!(db.get_config("test", "k").expect("get"), Some("\"v\"".into()));
        assert_eq!(db.path(), Path::new(":memory:"));
    }

    #[test]
    fn test_get_section_as_typed() {
        let db = SettingsDb::open_in_memory().expect("in-memory");
        db.set_config("compression", "zstd_level", "3").expect("set");
        db.set_config("compression", "original_ttl_days", "30").expect("set");
        db.set_config("compression", "compress_processed", "true").expect("set");
        db.set_config("compression", "encrypt_sensitive", "false").expect("set");

        let comp: file_pipeline_core::domain::config_models::CompressionConfig =
            db.get_section_as("compression").expect("typed");
        assert_eq!(comp.zstd_level, 3);
        assert_eq!(comp.original_ttl_days, 30);
    }

    #[test]
    fn test_get_config_as_typed() {
        let db = SettingsDb::open_in_memory().expect("in-memory");
        db.set_config("vector_db", "dim", "128").expect("set");
        let dim: Option<u64> = db.get_config_as("vector_db", "dim").expect("typed");
        assert_eq!(dim, Some(128));

        let missing: Option<u64> = db.get_config_as("vector_db", "nonexistent").expect("typed");
        assert_eq!(missing, None);
    }

    #[test]
    fn test_has_data_in() {
        let db = SettingsDb::open_in_memory().expect("in-memory");
        assert!(!db.has_data_in("config").expect("check"));
        db.set_config("a", "b", "c").expect("set");
        assert!(db.has_data_in("config").expect("check"));
    }

    // open_or_migrate 통합 테스트는 shared/settings_db.rs 잔류 (자유함수 + PipelineConfigExt 의존).

    // ── Phase 82-prep: processing_metrics 테스트 ───────────────

    #[test]
    fn test_processing_metric_increment() {
        let db = SettingsDb::open_in_memory().expect("in-memory");
        db.add_processing_metric("success", 1).expect("inc");
        db.add_processing_metric("success", 1).expect("inc");
        db.add_processing_metric("errors", 1).expect("inc");
        let raw = db.get_processing_metric_raw().expect("raw");
        assert_eq!(raw.get("success").copied(), Some(2));
        assert_eq!(raw.get("errors").copied(), Some(1));
    }

    #[test]
    fn test_processing_metric_summary_empty() {
        let db = SettingsDb::open_in_memory().expect("in-memory");
        let s = db.get_processing_metric_summary().expect("summary");
        // 분모 0 → 모두 None
        assert!(s.verify_pass_rate.is_none());
        assert!(s.quarantine_rate.is_none());
        assert!(s.avg_process_time_ms.is_none());
        assert_eq!(s.success, 0);
    }

    #[test]
    fn test_processing_metric_summary_rates() {
        let db = SettingsDb::open_in_memory().expect("in-memory");
        // 8 pass / 2 fail → verify_pass_rate=0.8
        for _ in 0..8 { db.add_processing_metric("verified_pass", 1).expect("inc"); }
        for _ in 0..2 { db.add_processing_metric("verified_fail", 1).expect("inc"); }
        // 9 success / 1 error → 1 quarantined → quarantine_rate=0.1
        for _ in 0..9 { db.add_processing_metric("success", 1).expect("inc"); }
        db.add_processing_metric("errors", 1).expect("inc");
        db.add_processing_metric("quarantined", 1).expect("inc");
        // 5 측정, 총 5000ms → 평균 1000ms
        for _ in 0..5 { db.add_processing_metric("counted_for_time", 1).expect("inc"); }
        db.add_processing_metric("total_time_ms", 5000).expect("inc");

        let s = db.get_processing_metric_summary().expect("summary");
        assert!((s.verify_pass_rate.expect("rate") - 0.8).abs() < 1e-5);
        assert!((s.quarantine_rate.expect("rate") - 0.1).abs() < 1e-5);
        assert_eq!(s.avg_process_time_ms, Some(1000));
        assert_eq!(s.success, 9);
        assert_eq!(s.errors, 1);
        assert_eq!(s.quarantined, 1);
    }

    // ── Phase 82: Decision Log 테스트 ───────────────────────

    fn sample_entry(path: &str, decision: &str, snap: Option<&str>) -> DecisionLogEntry {
        DecisionLogEntry {
            id: None,
            decided_at: "2026-05-14T00:00:00Z".into(),
            source: "setup_review".into(),
            snapshot_id: snap.map(String::from),
            path: path.into(),
            decision: decision.into(),
            before_value: Some("\"old\"".into()),
            after_value: Some("\"new\"".into()),
            priority: Some("P0".into()),
            risk: Some("low".into()),
            evidence: Some("heuristic".into()),
            confidence: Some("medium".into()),
            reason: Some("test rule".into()),
            context: None,
        }
    }

    #[test]
    fn test_decision_log_insert_and_list() {
        let db = SettingsDb::open_in_memory().expect("in-memory");
        let id1 = db.insert_decision(&sample_entry("a.x", "accepted", Some("snap-1"))).expect("ins1");
        let id2 = db.insert_decision(&sample_entry("a.y", "rejected", None)).expect("ins2");
        assert!(id1 > 0 && id2 > id1);
        let all = db.list_decisions(0).expect("list");
        assert_eq!(all.len(), 2);
        // 최근순 DESC
        assert_eq!(all[0].decision, "rejected");
        assert_eq!(all[1].decision, "accepted");
    }

    #[test]
    fn test_decision_log_filter_by_snapshot() {
        let db = SettingsDb::open_in_memory().expect("in-memory");
        db.insert_decision(&sample_entry("a", "accepted", Some("snap-A"))).expect("ins");
        db.insert_decision(&sample_entry("b", "rejected", None)).expect("ins");
        db.insert_decision(&sample_entry("c", "accepted", Some("snap-B"))).expect("ins");
        db.insert_decision(&sample_entry("d", "accepted", Some("snap-A"))).expect("ins");
        let a = db.list_decisions_by_snapshot("snap-A").expect("list");
        assert_eq!(a.len(), 2);
        // snap-A 결정만 + 입력 순서 ASC
        assert_eq!(a[0].path, "a");
        assert_eq!(a[1].path, "d");
    }

    #[test]
    fn test_decision_log_limit() {
        let db = SettingsDb::open_in_memory().expect("in-memory");
        for i in 0..5 {
            db.insert_decision(&sample_entry(&format!("p{}", i), "accepted", None)).expect("ins");
        }
        let two = db.list_decisions(2).expect("list");
        assert_eq!(two.len(), 2);
    }

    // ── A1: LLM 결과 캐시 테스트 ───────────────────────

    fn sample_cache_entry(file_hash: &str) -> LlmCacheEntry {
        LlmCacheEntry {
            file_hash: file_hash.to_string(),
            content_hash: "c1".to_string(),
            result_json: r#"{"summary":"테스트"}"#.to_string(),
            doc_types: "meeting".to_string(),
            hits: 0,
            created_at: "2026-05-14T00:00:00Z".to_string(),
            last_hit_at: None,
        }
    }

    #[test]
    fn test_llm_cache_miss_then_hit() {
        let db = SettingsDb::open_in_memory().expect("in-memory");
        // 1. miss
        let miss = db.lookup_llm_cache("nonexistent").expect("lookup");
        assert!(miss.is_none());
        // 2. upsert + hit
        db.upsert_llm_cache(&sample_cache_entry("h1")).expect("upsert");
        let hit1 = db.lookup_llm_cache("h1").expect("lookup");
        assert!(hit1.is_some());
        assert_eq!(hit1.unwrap().hits, 1); // 첫 조회로 hits 0 → 1
        // 3. 두 번째 조회
        let hit2 = db.lookup_llm_cache("h1").expect("lookup");
        assert_eq!(hit2.unwrap().hits, 2);
    }

    #[test]
    fn test_llm_cache_replace_on_upsert() {
        let db = SettingsDb::open_in_memory().expect("in-memory");
        db.upsert_llm_cache(&sample_cache_entry("h1")).expect("upsert1");
        let mut e = sample_cache_entry("h1");
        e.content_hash = "c2".into();
        e.doc_types = "study_note".into();
        db.upsert_llm_cache(&e).expect("upsert2");
        let got = db.lookup_llm_cache("h1").expect("lookup").expect("some");
        assert_eq!(got.content_hash, "c2");
        assert_eq!(got.doc_types, "study_note");
    }

    #[test]
    fn test_llm_cache_stats() {
        let db = SettingsDb::open_in_memory().expect("in-memory");
        for i in 0..3 {
            db.upsert_llm_cache(&sample_cache_entry(&format!("h{}", i))).expect("upsert");
        }
        // h0를 2번 조회 → hits=2
        let _ = db.lookup_llm_cache("h0").expect("look");
        let _ = db.lookup_llm_cache("h0").expect("look");
        let _ = db.lookup_llm_cache("h1").expect("look");
        let (count, total, avg) = db.llm_cache_stats().expect("stats");
        assert_eq!(count, 3);
        assert_eq!(total, 3);  // h0 +2, h1 +1, h2 +0
        assert!((avg - 1.0).abs() < 1e-5);
    }

    #[test]
    fn test_llm_cache_gc_to_keeps_most_recent() {
        let db = SettingsDb::open_in_memory().expect("in-memory");
        // 5건 INSERT
        for i in 0..5 {
            db.upsert_llm_cache(&sample_cache_entry(&format!("h{}", i))).expect("upsert");
        }
        // h2, h3, h4를 hit하여 last_hit_at 갱신. h0, h1은 NULL 상태 → GC 대상 1순위
        let _ = db.lookup_llm_cache("h2").expect("look");
        let _ = db.lookup_llm_cache("h3").expect("look");
        let _ = db.lookup_llm_cache("h4").expect("look");

        // max 3 → 2건 삭제 (h0, h1)
        let removed = db.gc_llm_cache_to(3).expect("gc");
        assert_eq!(removed, 2);
        assert!(db.lookup_llm_cache("h0").expect("look").is_none());
        assert!(db.lookup_llm_cache("h1").expect("look").is_none());
        assert!(db.lookup_llm_cache("h2").expect("look").is_some());

        // 이미 size <= max인 경우 no-op
        let again = db.gc_llm_cache_to(10).expect("gc");
        assert_eq!(again, 0);

        // max=0이면 무제한 → no-op
        let zero = db.gc_llm_cache_to(0).expect("gc");
        assert_eq!(zero, 0);
    }
}
