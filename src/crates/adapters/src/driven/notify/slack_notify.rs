use anyhow::Result;
use async_trait::async_trait;
use file_pipeline_core::domain::models::{DbStats, ProcessingSummary};
use file_pipeline_core::ports::output::NotificationPort;
use module_notify::{NotifyPort as RawNotify, SlackRawClient};

use super::map_err;
use super::format::{format_completion, format_duplicate_slack, format_send_slack, format_sensitive, format_summary_slack};

pub struct SlackNotificationAdapter {
    inner: SlackRawClient,
}

impl SlackNotificationAdapter {
    pub fn new(bot_token: String, channel: String) -> Self {
        Self {
            inner: SlackRawClient::new(bot_token, channel),
        }
    }
}

#[async_trait]
impl NotificationPort for SlackNotificationAdapter {
    async fn send(&self, title: &str, body: &str, level: &str) -> Result<()> {
        self.inner.send_text(&format_send_slack(title, body, level)).await.map_err(map_err)
    }

    async fn send_duplicate_alert(
        &self,
        filename: &str,
        reason: &str,
        diff_summary: &str,
    ) -> Result<()> {
        self.inner
            .send_text(&format_duplicate_slack(filename, reason, diff_summary))
            .await
            .map_err(map_err)
    }

    async fn send_sensitive_alert(&self, filename: &str, reason: &str) -> Result<()> {
        self.inner
            .send_text(&format_sensitive("slack", filename, reason))
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
            .send_text(&format_completion("slack", filename, doc_type, stats))
            .await
            .map_err(map_err)
    }

    async fn send_summary(&self, s: &ProcessingSummary) -> Result<()> {
        if s.is_empty() {
            return Ok(());
        }
        self.inner.send_text(&format_summary_slack(s)).await.map_err(map_err)
    }
}

// step-o2 (2026-06-16, outbound-umbrella-1): OutboundManifest 박힘
impl file_pipeline_core::ports::outbound::OutboundManifest for SlackNotificationAdapter {
    fn id(&self) -> &str { "fp-outbound-notify-slack" }
    fn category(&self) -> file_pipeline_core::ports::outbound::OutboundCategory {
        file_pipeline_core::ports::outbound::OutboundCategory::Notify
    }
    fn capabilities(&self) -> file_pipeline_core::ports::output::ResourceCapabilities {
        file_pipeline_core::ports::output::ResourceCapabilities::standard("slack")
    }
}
