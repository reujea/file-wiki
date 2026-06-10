use std::path::{Path, PathBuf};

use anyhow::Result;
use async_trait::async_trait;
use file_pipeline_core::ports::output::{RemoteStoragePort, ResourceCapabilities};
use module_storage::NetworkRemoteStorage;
use module_storage::RemoteStoragePort as RawRemote;

use super::map_err;

pub struct NetworkStorageAdapter {
    inner: NetworkRemoteStorage,
}

impl NetworkStorageAdapter {
    pub fn new(base_path: PathBuf) -> Self {
        Self {
            inner: NetworkRemoteStorage::new(base_path),
        }
    }
}

#[async_trait]
impl RemoteStoragePort for NetworkStorageAdapter {
    async fn upload(&self, local_path: &Path, remote_key: &str) -> Result<()> {
        self.inner.upload(local_path, remote_key).await.map_err(map_err)
    }
    async fn download(&self, remote_key: &str, local_path: &Path) -> Result<()> {
        self.inner.download(remote_key, local_path).await.map_err(map_err)
    }
    async fn list(&self, prefix: &str) -> Result<Vec<String>> {
        self.inner.list(prefix).await.map_err(map_err)
    }
    async fn delete(&self, remote_key: &str) -> Result<()> {
        self.inner.delete(remote_key).await.map_err(map_err)
    }
    fn is_configured(&self) -> bool {
        self.inner.is_configured()
    }

    fn capabilities(&self) -> ResourceCapabilities {
        ResourceCapabilities::standard("network")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::remote_null::NullRemoteStorage;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_network_storage_roundtrip() {
        let remote_dir = TempDir::new().unwrap();
        let local_dir = TempDir::new().unwrap();
        let adapter = NetworkStorageAdapter::new(remote_dir.path().to_path_buf());

        let src = local_dir.path().join("test.txt");
        std::fs::write(&src, "hello network storage").unwrap();
        adapter.upload(&src, "processed/test.txt").await.unwrap();
        assert!(remote_dir.path().join("processed/test.txt").exists());

        let dest = local_dir.path().join("downloaded.txt");
        adapter.download("processed/test.txt", &dest).await.unwrap();
        assert_eq!(std::fs::read_to_string(&dest).unwrap(), "hello network storage");

        let items = adapter.list("processed").await.unwrap();
        assert_eq!(items.len(), 1);
        assert!(items[0].contains("test.txt"));

        adapter.delete("processed/test.txt").await.unwrap();
        assert!(!remote_dir.path().join("processed/test.txt").exists());
    }

    #[tokio::test]
    async fn test_null_remote_storage() {
        let null = NullRemoteStorage;
        assert!(!null.is_configured());
        assert!(null.upload(Path::new("x"), "y").await.is_ok());
        assert!(null.list("").await.unwrap().is_empty());
    }
}
