pub mod composite;
pub mod format;
pub mod slack_notify;
pub mod telegram_notify;

/// module-notify `NotifyError`를 file-pipeline `anyhow::Error`로 변환
pub(crate) fn map_err(e: module_notify_api::NotifyError) -> anyhow::Error {
    anyhow::Error::msg(e.to_string())
}
