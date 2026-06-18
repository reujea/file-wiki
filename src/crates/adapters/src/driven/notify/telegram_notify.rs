use anyhow::Result;
use async_trait::async_trait;
use file_pipeline_core::domain::models::{DbStats, ProcessingSummary};
use file_pipeline_core::ports::output::NotificationPort;
use module_notify::{NotifyPort as RawNotify, TelegramRawClient};

use super::map_err;
use super::format::{format_completion, format_duplicate_telegram, format_send_telegram, format_sensitive, format_summary_telegram};

pub struct TelegramNotificationAdapter {
    inner: TelegramRawClient,
}

impl TelegramNotificationAdapter {
    pub fn new(bot_token: String, chat_id: String) -> Self {
        Self {
            inner: TelegramRawClient::new(bot_token, chat_id),
        }
    }
}

#[async_trait]
impl NotificationPort for TelegramNotificationAdapter {
    async fn send(&self, title: &str, body: &str, level: &str) -> Result<()> {
        self.inner.send_text(&format_send_telegram(title, body, level)).await.map_err(map_err)
    }

    async fn send_duplicate_alert(
        &self,
        filename: &str,
        reason: &str,
        diff_summary: &str,
    ) -> Result<()> {
        self.inner
            .send_text(&format_duplicate_telegram(filename, reason, diff_summary))
            .await
            .map_err(map_err)
    }

    async fn send_sensitive_alert(&self, filename: &str, reason: &str) -> Result<()> {
        self.inner
            .send_text(&format_sensitive("telegram", filename, reason))
            .await
            .map_err(map_err)
    }

    async fn send_completion(
        &self,
        filename: &str,
        doc_type: &str,
        stats: &DbStats,
    ) -> Result<()> {
        self.inner
            .send_text(&format_completion("telegram", filename, doc_type, stats))
            .await
            .map_err(map_err)
    }

    async fn send_summary(&self, s: &ProcessingSummary) -> Result<()> {
        if s.is_empty() {
            return Ok(());
        }
        self.inner.send_text(&format_summary_telegram(s)).await.map_err(map_err)
    }
}

// step-o3 (2026-06-17, outbound-umbrella-1): OutboundManifest + mode_options (alert/event) 박힘
impl file_pipeline_core::ports::outbound::OutboundManifest for TelegramNotificationAdapter {
    fn id(&self) -> &str { "fp-outbound-notify-telegram" }
    fn category(&self) -> file_pipeline_core::ports::outbound::OutboundCategory {
        file_pipeline_core::ports::outbound::OutboundCategory::Notify
    }
    fn capabilities(&self) -> file_pipeline_core::ports::output::ResourceCapabilities {
        let mut caps = file_pipeline_core::ports::output::ResourceCapabilities::standard("telegram");
        caps.mode_options = &["alert", "event"];
        caps.active_mode = "alert".into();
        caps
    }
    fn modes(&self) -> &[&str] {
        &["alert", "event"]
    }
}
