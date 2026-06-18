use anyhow::Result;
use async_trait::async_trait;
use file_pipeline_core::domain::models::{DbStats, ProcessingSummary};
use file_pipeline_core::ports::output::NotificationPort;

/// 아무 동작도 하지 않는 알림 어댑터
pub struct NullNotificationAdapter;

#[async_trait]
impl NotificationPort for NullNotificationAdapter {
    async fn send(&self, _title: &str, _body: &str, _level: &str) -> Result<()> {
        Ok(())
    }

    async fn send_duplicate_alert(
        &self,
        _filename: &str,
        _reason: &str,
        _diff_summary: &str,
    ) -> Result<()> {
        Ok(())
    }

    async fn send_sensitive_alert(&self, _filename: &str, _reason: &str) -> Result<()> {
        Ok(())
    }

    async fn send_completion(
        &self,
        _filename: &str,
        _doc_type: &str,
        _stats: &DbStats,
    ) -> Result<()> {
        Ok(())
    }

    async fn send_summary(&self, _summary: &ProcessingSummary) -> Result<()> {
        Ok(())
    }
}

/// 여러 알림 어댑터를 결합하는 Composite
pub struct CompositeNotificationAdapter {
    adapters: Vec<Box<dyn NotificationPort>>,
}

impl CompositeNotificationAdapter {
    pub fn new(adapters: Vec<Box<dyn NotificationPort>>) -> Self {
        Self { adapters }
    }
}

#[async_trait]
impl NotificationPort for CompositeNotificationAdapter {
    async fn send(&self, title: &str, body: &str, level: &str) -> Result<()> {
        for adapter in &self.adapters {
            if let Err(e) = adapter.send(title, body, level).await {
                tracing::warn!("알림 전송 실패: {}", e);
            }
        }
        Ok(())
    }

    async fn send_duplicate_alert(
        &self,
        filename: &str,
        reason: &str,
        diff_summary: &str,
    ) -> Result<()> {
        for adapter in &self.adapters {
            if let Err(e) = adapter.send_duplicate_alert(filename, reason, diff_summary).await {
                tracing::warn!("중복 알림 전송 실패: {}", e);
            }
        }
        Ok(())
    }

    async fn send_sensitive_alert(&self, filename: &str, reason: &str) -> Result<()> {
        for adapter in &self.adapters {
            if let Err(e) = adapter.send_sensitive_alert(filename, reason).await {
                tracing::warn!("민감 알림 전송 실패: {}", e);
            }
        }
        Ok(())
    }

    async fn send_completion(
        &self,
        filename: &str,
        doc_type: &str,
        stats: &DbStats,
    ) -> Result<()> {
        for adapter in &self.adapters {
            if let Err(e) = adapter.send_completion(filename, doc_type, stats).await {
                tracing::warn!("완료 알림 전송 실패: {}", e);
            }
        }
        Ok(())
    }

    async fn send_summary(&self, summary: &ProcessingSummary) -> Result<()> {
        for adapter in &self.adapters {
            if let Err(e) = adapter.send_summary(summary).await {
                tracing::warn!("요약 알림 전송 실패: {}", e);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    struct RecordingNotificationAdapter {
        calls: Arc<Mutex<Vec<String>>>,
    }

    impl RecordingNotificationAdapter {
        fn new() -> (Self, Arc<Mutex<Vec<String>>>) {
            let calls = Arc::new(Mutex::new(vec![]));
            (Self { calls: calls.clone() }, calls)
        }
    }

    #[async_trait]
    impl NotificationPort for RecordingNotificationAdapter {
        async fn send(&self, title: &str, body: &str, level: &str) -> Result<()> {
            self.calls.lock().unwrap().push(format!("send:{}:{}:{}", title, body, level));
            Ok(())
        }

        async fn send_duplicate_alert(
            &self,
            filename: &str,
            reason: &str,
            diff_summary: &str,
        ) -> Result<()> {
            self.calls.lock().unwrap().push(format!("duplicate:{}:{}:{}", filename, reason, diff_summary));
            Ok(())
        }

        async fn send_sensitive_alert(&self, filename: &str, reason: &str) -> Result<()> {
            self.calls.lock().unwrap().push(format!("sensitive:{}:{}", filename, reason));
            Ok(())
        }

        async fn send_completion(
            &self,
            filename: &str,
            doc_type: &str,
            _stats: &DbStats,
        ) -> Result<()> {
            self.calls.lock().unwrap().push(format!("completion:{}:{}", filename, doc_type));
            Ok(())
        }

        async fn send_summary(&self, _summary: &ProcessingSummary) -> Result<()> {
            self.calls.lock().unwrap().push("summary".to_string());
            Ok(())
        }
    }

    impl file_pipeline_core::ports::outbound::OutboundManifest for RecordingNotificationAdapter {
        fn id(&self) -> &str { "fp-outbound-notify-recording-test" }
        fn category(&self) -> file_pipeline_core::ports::outbound::OutboundCategory {
            file_pipeline_core::ports::outbound::OutboundCategory::Notify
        }
        fn capabilities(&self) -> file_pipeline_core::ports::output::ResourceCapabilities {
            file_pipeline_core::ports::output::ResourceCapabilities::standard("recording-test")
        }
    }

    struct FailingNotificationAdapter;

    #[async_trait]
    impl NotificationPort for FailingNotificationAdapter {
        async fn send(&self, _title: &str, _body: &str, _level: &str) -> Result<()> {
            anyhow::bail!("send failed")
        }

        async fn send_duplicate_alert(
            &self,
            _filename: &str,
            _reason: &str,
            _diff_summary: &str,
        ) -> Result<()> {
            anyhow::bail!("duplicate failed")
        }

        async fn send_sensitive_alert(&self, _filename: &str, _reason: &str) -> Result<()> {
            anyhow::bail!("sensitive failed")
        }

        async fn send_completion(
            &self,
            _filename: &str,
            _doc_type: &str,
            _stats: &DbStats,
        ) -> Result<()> {
            anyhow::bail!("completion failed")
        }

        async fn send_summary(&self, _summary: &ProcessingSummary) -> Result<()> {
            anyhow::bail!("summary failed")
        }
    }

    impl file_pipeline_core::ports::outbound::OutboundManifest for FailingNotificationAdapter {
        fn id(&self) -> &str { "fp-outbound-notify-failing-test" }
        fn category(&self) -> file_pipeline_core::ports::outbound::OutboundCategory {
            file_pipeline_core::ports::outbound::OutboundCategory::Notify
        }
        fn capabilities(&self) -> file_pipeline_core::ports::output::ResourceCapabilities {
            file_pipeline_core::ports::output::ResourceCapabilities::standard("failing-test")
        }
    }

    #[tokio::test]
    async fn test_composite_fans_out() {
        let (adapter1, calls1) = RecordingNotificationAdapter::new();
        let (adapter2, calls2) = RecordingNotificationAdapter::new();
        let composite = CompositeNotificationAdapter::new(vec![
            Box::new(adapter1),
            Box::new(adapter2),
        ]);

        composite.send("title", "body", "info").await.unwrap();

        assert_eq!(calls1.lock().unwrap().len(), 1);
        assert_eq!(calls1.lock().unwrap()[0], "send:title:body:info");
        assert_eq!(calls2.lock().unwrap().len(), 1);
        assert_eq!(calls2.lock().unwrap()[0], "send:title:body:info");
    }

    #[tokio::test]
    async fn test_composite_tolerates_failure() {
        let (adapter, calls) = RecordingNotificationAdapter::new();
        let composite = CompositeNotificationAdapter::new(vec![
            Box::new(FailingNotificationAdapter),
            Box::new(adapter),
        ]);

        let result = composite.send("title", "body", "warn").await;
        assert!(result.is_ok());
        assert_eq!(calls.lock().unwrap().len(), 1);
        assert_eq!(calls.lock().unwrap()[0], "send:title:body:warn");
    }

    #[tokio::test]
    async fn test_null_adapter_noop() {
        let null = NullNotificationAdapter;
        assert!(null.send("t", "b", "l").await.is_ok());
        assert!(null.send_duplicate_alert("f", "r", "d").await.is_ok());
        assert!(null.send_sensitive_alert("f", "r").await.is_ok());
        let stats = DbStats {
            total_documents: 0,
            by_type: vec![],
            total_size_bytes: 0,
            sensitive_count: 0,
        };
        assert!(null.send_completion("f", "t", &stats).await.is_ok());
        let summary = ProcessingSummary::default();
        assert!(null.send_summary(&summary).await.is_ok());
    }
}

// step-o2 partial 해소 (2026-06-17, outbound-umbrella-1): notify mock OutboundManifest 박힘
impl file_pipeline_core::ports::outbound::OutboundManifest for NullNotificationAdapter {
    fn id(&self) -> &str { "fp-outbound-notify-null" }
    fn category(&self) -> file_pipeline_core::ports::outbound::OutboundCategory {
        file_pipeline_core::ports::outbound::OutboundCategory::Notify
    }
    fn capabilities(&self) -> file_pipeline_core::ports::output::ResourceCapabilities {
        file_pipeline_core::ports::output::ResourceCapabilities::standard("null")
    }
}

impl file_pipeline_core::ports::outbound::OutboundManifest for CompositeNotificationAdapter {
    fn id(&self) -> &str { "fp-outbound-notify-composite" }
    fn category(&self) -> file_pipeline_core::ports::outbound::OutboundCategory {
        file_pipeline_core::ports::outbound::OutboundCategory::Notify
    }
    fn capabilities(&self) -> file_pipeline_core::ports::output::ResourceCapabilities {
        file_pipeline_core::ports::output::ResourceCapabilities::standard("composite")
    }
}
