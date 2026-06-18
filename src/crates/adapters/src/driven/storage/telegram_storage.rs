//! Telegram Bot API 기반 storage 어댑터 — step-o3 (2026-06-17, outbound-umbrella-1) 정합.
//!
//! 3 mode: `document` (sendDocument, 디폴트) / `text` (sendMessage) / `channel` (channel chat_id 발송).
//!
//! ## 제약 (Telegram Bot API 한계)
//!
//! - **자기 발송 history 자동 조회 부재** → `list` / `download` = `telegram_message_map` sqlite 외부 매핑 의무
//! - **document API 50MB 한계** — 본 어댑터 upload 시 사전 size check + bail
//! - **48시간 후 delete 불가** — `delete` 시점 본 row 의 `ts` 비교 + 48h 초과 시 bail (soft)
//! - **`hard_delete` 부재** = `supports_hard_delete=false`
//!
//! ## 인증
//!
//! - `bot_token` — 환경변수 `TELEGRAM_BOT_TOKEN` 또는 config 주입
//! - `chat_id` — group / channel / user chat_id (storage 와 notify 별도 권장)
//!
//! ## 의존 영역
//!
//! - `reqwest::Client` — telegram bot API REST 호출 (sendDocument multipart / sendMessage / deleteMessage / getFile)
//! - `telegram_message_map` sqlite table — `settings_db.rs` 안 박힘 (step-o3 prep)
//! - `file_pipeline_core::ports::output::{RemoteStoragePort, ResourceCapabilities}` — 기존 storage 패턴 정합

use std::path::Path;

use anyhow::{anyhow, bail, Result};
use async_trait::async_trait;
use file_pipeline_core::ports::output::{RemoteStoragePort, ResourceCapabilities};
use reqwest::Client;
use rusqlite::Connection;
use serde_json::Value;

/// telegram_message_map schema — settings_db.rs 의 정의와 정합 (step-o3, 2026-06-17).
/// shared 의존 부재 = adapters 자체 schema 박힘 (workspace cycle 회피, lesson #14 R1 정합).
const TELEGRAM_MAP_SCHEMA: &str = "
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

/// 50MB 한계 (Telegram document API).
const TELEGRAM_DOCUMENT_LIMIT_BYTES: u64 = 50 * 1024 * 1024;

/// 48시간 (telegram delete 제약).
const TELEGRAM_DELETE_WINDOW_SECS: i64 = 48 * 60 * 60;

/// Telegram storage 어댑터 — sendDocument / sendMessage / deleteMessage / getFile 단일 진입점.
pub struct TelegramStorageAdapter {
    bot_token: String,
    chat_id: String,
    mode: String,
    client: Client,
    /// settings.db 경로 — telegram_message_map CRUD.
    db_path: std::path::PathBuf,
}

impl TelegramStorageAdapter {
    /// `mode` = "document" / "text" / "channel". 디폴트 = "document".
    pub fn new(bot_token: String, chat_id: String, mode: String, db_path: std::path::PathBuf) -> Self {
        Self {
            bot_token,
            chat_id,
            mode: if mode.is_empty() { "document".into() } else { mode },
            client: Client::new(),
            db_path,
        }
    }

    fn api_url(&self, method: &str) -> String {
        format!("https://api.telegram.org/bot{}/{}", self.bot_token, method)
    }

    /// settings.db open + telegram_message_map schema 보장.
    /// settings_db.rs 의 SettingsDb 와 동일 path 공유 (race condition 회피 = 트랜잭션 단위 호출 + SQLite 자체 file lock).
    fn open_db(&self) -> Result<Connection> {
        let conn = Connection::open(&self.db_path)?;
        conn.execute_batch(TELEGRAM_MAP_SCHEMA)?;
        Ok(conn)
    }

    /// telegram API JSON 응답 → result.message_id + (file_id, file_unique_id) 추출.
    fn parse_send_response(value: &Value) -> Result<(i64, Option<String>)> {
        let ok = value.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
        if !ok {
            let desc = value
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error");
            bail!("telegram API failed: {}", desc);
        }
        let result = value
            .get("result")
            .ok_or_else(|| anyhow!("telegram response missing `result`"))?;
        let message_id = result
            .get("message_id")
            .and_then(|v| v.as_i64())
            .ok_or_else(|| anyhow!("telegram response missing `result.message_id`"))?;
        let file_id = result
            .get("document")
            .and_then(|d| d.get("file_id"))
            .and_then(|v| v.as_str())
            .map(String::from);
        Ok((message_id, file_id))
    }
}

#[async_trait]
impl RemoteStoragePort for TelegramStorageAdapter {
    async fn upload(&self, local_path: &Path, remote_key: &str) -> Result<()> {
        let metadata = std::fs::metadata(local_path)?;
        let size = metadata.len();

        // step-o3: 50MB pre-check (telegram document API 한계 — 초과 시 bot upload bail)
        if self.mode == "document" && size > TELEGRAM_DOCUMENT_LIMIT_BYTES {
            bail!(
                "telegram document API 50MB 초과: {} bytes (key={})",
                size,
                remote_key
            );
        }

        let (message_id, file_id) = match self.mode.as_str() {
            "document" | "channel" => {
                // sendDocument multipart
                let bytes = std::fs::read(local_path)?;
                let filename = local_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("upload");
                let part = reqwest::multipart::Part::bytes(bytes)
                    .file_name(filename.to_string());
                let form = reqwest::multipart::Form::new()
                    .text("chat_id", self.chat_id.clone())
                    .text("caption", remote_key.to_string())
                    .part("document", part);
                let resp = self
                    .client
                    .post(self.api_url("sendDocument"))
                    .multipart(form)
                    .send()
                    .await?;
                let json: Value = resp.json().await?;
                Self::parse_send_response(&json)?
            }
            "text" => {
                // sendMessage — local_path 본문을 text 로 발송 (size 작은 메모 영역)
                let text = std::fs::read_to_string(local_path)?;
                let resp = self
                    .client
                    .post(self.api_url("sendMessage"))
                    .json(&serde_json::json!({
                        "chat_id": self.chat_id,
                        "text": format!("{}\n\n{}", remote_key, text),
                    }))
                    .send()
                    .await?;
                let json: Value = resp.json().await?;
                Self::parse_send_response(&json)?
            }
            other => bail!("telegram storage 지원 부재 mode: {}", other),
        };

        // telegram_message_map 박힘 — rusqlite 직접 호출 (shared 의존 부재, lesson #14 R1 정합)
        let conn = self.open_db()?;
        conn.execute(
            "INSERT OR REPLACE INTO telegram_message_map
             (remote_key, message_id, file_id, chat_id, mode, size_bytes)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![remote_key, message_id, file_id, self.chat_id, self.mode, size as i64],
        )?;

        Ok(())
    }

    async fn download(&self, remote_key: &str, local_path: &Path) -> Result<()> {
        // telegram_message_map 매핑 조회 (rusqlite 직접)
        let conn = self.open_db()?;
        let entry: Option<(i64, Option<String>, String, String)> = conn
            .query_row(
                "SELECT message_id, file_id, chat_id, mode
                 FROM telegram_message_map WHERE remote_key = ?1",
                rusqlite::params![remote_key],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .ok();
        let entry = entry.ok_or_else(|| anyhow!("telegram_message_map 부재 key={}", remote_key))?;
        let (_message_id, file_id, _chat_id, mode) = entry;

        if mode != "document" {
            bail!("telegram download = document mode 만 지원 (key={}, mode={})", remote_key, mode);
        }
        let file_id = file_id
            .ok_or_else(|| anyhow!("telegram_message_map 의 file_id 부재 key={}", remote_key))?;

        // getFile → file_path 추출
        let get_file_resp: Value = self
            .client
            .post(self.api_url("getFile"))
            .json(&serde_json::json!({ "file_id": file_id }))
            .send()
            .await?
            .json()
            .await?;
        let ok = get_file_resp.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
        if !ok {
            bail!(
                "telegram getFile 실패: {:?}",
                get_file_resp.get("description")
            );
        }
        let file_path = get_file_resp
            .get("result")
            .and_then(|r| r.get("file_path"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("getFile 응답 file_path 부재"))?;

        // https://api.telegram.org/file/bot<token>/<file_path>
        let download_url = format!(
            "https://api.telegram.org/file/bot{}/{}",
            self.bot_token, file_path
        );
        let bytes = self.client.get(&download_url).send().await?.bytes().await?;
        std::fs::write(local_path, &bytes)?;
        Ok(())
    }

    async fn list(&self, prefix: &str) -> Result<Vec<String>> {
        // bot API list 부재 → telegram_message_map 외부 매핑 활용 (rusqlite 직접)
        let conn = self.open_db()?;
        let mut stmt = conn.prepare(
            "SELECT remote_key FROM telegram_message_map
             WHERE chat_id = ?1 AND remote_key LIKE ?2
             ORDER BY ts DESC LIMIT 1000",
        )?;
        let pattern = format!("{}%", prefix);
        let rows = stmt
            .query_map(rusqlite::params![self.chat_id, pattern], |row| row.get(0))?
            .collect::<Result<Vec<String>, _>>()?;
        Ok(rows)
    }

    async fn delete(&self, remote_key: &str) -> Result<()> {
        let conn = self.open_db()?;
        let entry: Option<(i64, String)> = conn
            .query_row(
                "SELECT message_id, ts FROM telegram_message_map WHERE remote_key = ?1",
                rusqlite::params![remote_key],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .ok();
        let (message_id, ts) =
            entry.ok_or_else(|| anyhow!("telegram_message_map 부재 key={}", remote_key))?;

        // 48h 검증 — telegram bot API deleteMessage 제약
        // ts = SQLite CURRENT_TIMESTAMP "YYYY-MM-DD HH:MM:SS" UTC
        if let Ok(parsed) = chrono::NaiveDateTime::parse_from_str(&ts, "%Y-%m-%d %H:%M:%S") {
            let now = chrono::Utc::now().naive_utc();
            let elapsed = now.signed_duration_since(parsed).num_seconds();
            if elapsed > TELEGRAM_DELETE_WINDOW_SECS {
                bail!(
                    "telegram delete 48h 초과 (key={}, elapsed={}s, limit={}s) — bot API 제약",
                    remote_key,
                    elapsed,
                    TELEGRAM_DELETE_WINDOW_SECS
                );
            }
        }

        let resp: Value = self
            .client
            .post(self.api_url("deleteMessage"))
            .json(&serde_json::json!({
                "chat_id": self.chat_id,
                "message_id": message_id,
            }))
            .send()
            .await?
            .json()
            .await?;
        let ok = resp.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
        if !ok {
            let desc = resp.get("description").and_then(|v| v.as_str()).unwrap_or("?");
            bail!("telegram deleteMessage 실패: {}", desc);
        }
        // 본 row 삭제 (외부 매핑도 정합 의무)
        conn.execute(
            "DELETE FROM telegram_message_map WHERE remote_key = ?1",
            rusqlite::params![remote_key],
        )?;
        Ok(())
    }

    fn is_configured(&self) -> bool {
        !self.bot_token.is_empty() && !self.chat_id.is_empty()
    }

    fn capabilities(&self) -> ResourceCapabilities {
        ResourceCapabilities {
            backend: "telegram",
            can_upload: true,
            can_download: true,
            can_list: false,
            can_delete: true,
            mode_options: &["document", "text", "channel"],
            active_mode: self.mode.clone(),
            supports_hard_delete: false,
        }
    }
}

impl file_pipeline_core::ports::outbound::OutboundManifest for TelegramStorageAdapter {
    fn id(&self) -> &str {
        "fp-outbound-storage-telegram"
    }
    fn category(&self) -> file_pipeline_core::ports::outbound::OutboundCategory {
        file_pipeline_core::ports::outbound::OutboundCategory::Storage
    }
    fn capabilities(&self) -> ResourceCapabilities {
        <Self as RemoteStoragePort>::capabilities(self)
    }
    fn modes(&self) -> &[&str] {
        &["document", "text", "channel"]
    }
}
