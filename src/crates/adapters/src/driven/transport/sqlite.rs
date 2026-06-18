//! `SqliteTransport` 구현체 — transport-flatten-1 step-t2 (2026-06-18).
//!
//! rusqlite 기반 raw key-value table get/put/delete. **도메인 로직 0**: 스키마 의미,
//! telegram_message_map 컬럼 매핑, llm_cache TTL 등은 plugin/caller 책임. 본 transport 는
//! `(key TEXT PRIMARY KEY, value TEXT)` 형태의 단순 KV 테이블만 노출한다.
//!
//! rusqlite 는 sync 라 `spawn_blocking` 으로 async trait 에 적응시킨다. 매 호출 시 파일을
//! 새로 연다 (경량 wrapper — 커넥션 풀/캐시는 step-t4 plugin 이관 시 결정).

use anyhow::{Context, Result};
use async_trait::async_trait;
use file_pipeline_core::ports::raw_transport::{SqliteTransport, TransportMeta};
use rusqlite::{Connection, OptionalExtension};

/// rusqlite 기반 sqlite raw transport. 상태 = db 파일 경로만.
pub struct RusqliteTransport {
    db_path: String,
}

impl RusqliteTransport {
    /// db 파일 경로로 생성 (`":memory:"` 는 호출마다 새 DB 라 영속 불가 — 파일 경로 권장).
    pub fn new(db_path: impl Into<String>) -> Self {
        Self { db_path: db_path.into() }
    }

    /// 테이블이 없으면 raw KV 스키마로 생성. table 명은 caller 가 식별자만 전달 (SQL injection
    /// 방지를 위해 영숫자+`_` 만 허용).
    fn ensure_table(conn: &Connection, table: &str) -> Result<()> {
        validate_ident(table)?;
        conn.execute(
            &format!("CREATE TABLE IF NOT EXISTS {table} (key TEXT PRIMARY KEY, value TEXT NOT NULL)"),
            [],
        )
        .with_context(|| format!("테이블 생성 실패: {table}"))?;
        Ok(())
    }
}

/// 테이블/식별자명 검증 — 영숫자 + `_` 만. SQL 인젝션 방지 (값은 bind 라 안전, 식별자만 검증).
fn validate_ident(ident: &str) -> Result<()> {
    if ident.is_empty() || !ident.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        anyhow::bail!("잘못된 테이블명 (영숫자/_ 만 허용): {ident}");
    }
    Ok(())
}

#[async_trait]
impl SqliteTransport for RusqliteTransport {
    async fn get(&self, table: &str, key: &str, _meta: &TransportMeta) -> Result<Option<String>> {
        let (db, table, key) = (self.db_path.clone(), table.to_string(), key.to_string());
        tokio::task::spawn_blocking(move || -> Result<Option<String>> {
            let conn = Connection::open(&db).with_context(|| format!("sqlite open 실패: {db}"))?;
            Self::ensure_table(&conn, &table)?;
            let val = conn
                .query_row(
                    &format!("SELECT value FROM {table} WHERE key = ?1"),
                    rusqlite::params![key],
                    |row| row.get::<_, String>(0),
                )
                .optional()
                .context("sqlite get 실패")?;
            Ok(val)
        })
        .await
        .context("sqlite get 태스크 join 실패")?
    }

    async fn put(&self, table: &str, key: &str, value: &str, _meta: &TransportMeta) -> Result<()> {
        let (db, table, key, value) =
            (self.db_path.clone(), table.to_string(), key.to_string(), value.to_string());
        tokio::task::spawn_blocking(move || -> Result<()> {
            let conn = Connection::open(&db).with_context(|| format!("sqlite open 실패: {db}"))?;
            Self::ensure_table(&conn, &table)?;
            conn.execute(
                &format!("INSERT INTO {table} (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value"),
                rusqlite::params![key, value],
            )
            .context("sqlite put 실패")?;
            Ok(())
        })
        .await
        .context("sqlite put 태스크 join 실패")?
    }

    async fn delete(&self, table: &str, key: &str, _meta: &TransportMeta) -> Result<bool> {
        let (db, table, key) = (self.db_path.clone(), table.to_string(), key.to_string());
        tokio::task::spawn_blocking(move || -> Result<bool> {
            let conn = Connection::open(&db).with_context(|| format!("sqlite open 실패: {db}"))?;
            Self::ensure_table(&conn, &table)?;
            let n = conn
                .execute(&format!("DELETE FROM {table} WHERE key = ?1"), rusqlite::params![key])
                .context("sqlite delete 실패")?;
            Ok(n > 0)
        })
        .await
        .context("sqlite delete 태스크 join 실패")?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_put_get_delete_roundtrip() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = dir.path().join("raw.db");
        let t = RusqliteTransport::new(db.to_str().expect("utf8"));
        let meta = TransportMeta::default();

        assert_eq!(t.get("kv", "k1", &meta).await.expect("get"), None);
        t.put("kv", "k1", "{\"a\":1}", &meta).await.expect("put");
        assert_eq!(t.get("kv", "k1", &meta).await.expect("get").as_deref(), Some("{\"a\":1}"));
        // upsert.
        t.put("kv", "k1", "v2", &meta).await.expect("put2");
        assert_eq!(t.get("kv", "k1", &meta).await.expect("get").as_deref(), Some("v2"));

        assert!(t.delete("kv", "k1", &meta).await.expect("delete"));
        assert!(!t.delete("kv", "k1", &meta).await.expect("delete2"));
    }

    #[tokio::test]
    async fn test_invalid_table_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = dir.path().join("raw.db");
        let t = RusqliteTransport::new(db.to_str().expect("utf8"));
        let meta = TransportMeta::default();
        assert!(t.get("bad table; DROP", "k", &meta).await.is_err());
    }
}
