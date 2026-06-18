use std::path::Path;

use anyhow::Result;
use async_trait::async_trait;
use file_pipeline_core::ports::output::{RemoteStoragePort, ResourceCapabilities};
use module_storage::NullRemoteStorage as RawNull;
use module_storage::RemoteStoragePort as RawRemote;

use super::map_err;

/// 비활성 원격 저장소 (file-pipeline 도메인 RemoteStoragePort 구현)
pub struct NullRemoteStorage;

#[async_trait]
impl RemoteStoragePort for NullRemoteStorage {
    async fn upload(&self, local_path: &Path, remote_key: &str) -> Result<()> {
        RawNull.upload(local_path, remote_key).await.map_err(map_err)
    }
    async fn download(&self, remote_key: &str, local_path: &Path) -> Result<()> {
        RawNull.download(remote_key, local_path).await.map_err(map_err)
    }
    async fn list(&self, prefix: &str) -> Result<Vec<String>> {
        RawNull.list(prefix).await.map_err(map_err)
    }
    async fn delete(&self, remote_key: &str) -> Result<()> {
        RawNull.delete(remote_key).await.map_err(map_err)
    }
    fn is_configured(&self) -> bool {
        RawNull.is_configured()
    }

    fn capabilities(&self) -> ResourceCapabilities {
        ResourceCapabilities::null()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_null_remote_is_not_configured() {
        let null = NullRemoteStorage;
        assert!(!null.is_configured());
    }

    #[tokio::test]
    async fn test_null_remote_upload_noop() {
        let null = NullRemoteStorage;
        // 존재하지 않는 경로를 줘도 NullRemoteStorage는 noop이라 성공해야 함
        let result = null.upload(&PathBuf::from("nonexistent.txt"), "remote/key").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_null_remote_list_returns_empty() {
        let null = NullRemoteStorage;
        let result = null.list("prefix/").await.expect("list");
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_null_remote_delete_noop() {
        let null = NullRemoteStorage;
        let result = null.delete("any/key").await;
        assert!(result.is_ok());
    }
}
