//! Telegram storage 어댑터 — step-p6 단순화 (2026-06-18, plugin-sdk-1).
//!
//! ## 본질
//!
//! telegram 도메인 로직(50MB/multipart/48h delete/mode/sqlite 매핑)은 step-p5 에서
//! `_rust_module/fp-plugin-storage-telegram` plugin crate 로 이관 완료. 본 어댑터는
//! 도메인 로직을 보유하지 않고 **어댑터 식별 + capabilities 표면**만 담당한다.
//!
//! ## RemoteStoragePort impl
//!
//! upload/download/list/delete 는 모두 plugin 위임 stub — 직접 IO 폐기.
//! 실제 IO 는 plugin 바이너리 IPC 경유 (SDK `handle_request` + `PluginRegistry` 라우팅
//! 완성 후 연결). 어댑터→core registry.call 경로가 헥사고날상 부재하므로 현 단계는
//! 명시적 미연결 bail.
//!
//! ## 잔류 이유
//!
//! - `is_configured` / `capabilities` (RemoteStoragePort) = plugin manifest 와 별개로
//!   어댑터 식별 + UI capabilities 표면. (OutboundManifest super-trait 우산은 step-p7
//!   에서 완전 폐기 — raw I/O 재정의 정합. raw_transport 4 채널로 대체.)

use std::path::Path;

use anyhow::{bail, Result};
use async_trait::async_trait;
use file_pipeline_core::ports::output::{RemoteStoragePort, ResourceCapabilities};

/// plugin 위임 안내 — 도메인 메서드 본문 공통 stub.
macro_rules! delegated {
    () => {
        bail!(
            "telegram storage = plugin io.file-pipeline.storage-telegram 위임 (step-p5 이관). \
             어댑터 직접 호출 폐기 — plugin 바이너리 IPC 경유. \
             SDK handle_request + PluginRegistry 라우팅 완성 후 연결."
        )
    };
}

/// Telegram storage 어댑터 — 식별 + capabilities 표면만 (도메인 로직 plugin 이관, step-p6).
pub struct TelegramStorageAdapter {
    bot_token: String,
    chat_id: String,
    mode: String,
}

impl TelegramStorageAdapter {
    /// `mode` = "document" / "text" / "channel". 디폴트 = "document".
    pub fn new(bot_token: String, chat_id: String, mode: String) -> Self {
        Self {
            bot_token,
            chat_id,
            mode: if mode.is_empty() { "document".into() } else { mode },
        }
    }
}

#[async_trait]
impl RemoteStoragePort for TelegramStorageAdapter {
    async fn upload(&self, _local_path: &Path, _remote_key: &str) -> Result<()> {
        delegated!()
    }

    async fn download(&self, _remote_key: &str, _local_path: &Path) -> Result<()> {
        delegated!()
    }

    async fn list(&self, _prefix: &str) -> Result<Vec<String>> {
        delegated!()
    }

    async fn delete(&self, _remote_key: &str) -> Result<()> {
        delegated!()
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
