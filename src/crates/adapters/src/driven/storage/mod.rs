pub mod zstd_storage;
pub mod remote_null;
pub mod network_storage;
pub mod webdav_storage;
pub mod s3_storage;
pub mod notion_storage;
pub mod telegram_storage;

/// module-storage `StorageError`를 file-pipeline `anyhow::Error`로 변환
pub(crate) fn map_err(e: module_storage_api::StorageError) -> anyhow::Error {
    anyhow::Error::msg(e.to_string())
}
