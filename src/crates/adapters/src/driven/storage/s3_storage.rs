use std::path::Path;

use anyhow::Result;
use async_trait::async_trait;
use file_pipeline_core::ports::output::{RemoteStoragePort, ResourceCapabilities};
use module_storage::RemoteStoragePort as RawRemote;
use module_storage::S3RemoteStorage;

use super::map_err;

pub struct S3StorageAdapter {
    inner: S3RemoteStorage,
}

impl S3StorageAdapter {
    pub fn new(
        endpoint: String,
        bucket: String,
        region: String,
        access_key: String,
        secret_key: String,
        prefix: String,
    ) -> Self {
        Self {
            inner: S3RemoteStorage::new(endpoint, bucket, region, access_key, secret_key, prefix),
        }
    }
}

#[async_trait]
impl RemoteStoragePort for S3StorageAdapter {
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
        ResourceCapabilities::standard("s3")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_s3_is_configured_always_true() {
        // module-storage S3RemoteStorage::is_configured는 어댑터 활성화 = 설정됨으로 간주.
        // 실제 자격증명 검증은 first request 시 발견.
        let adapter = S3StorageAdapter::new(
            "https://s3.amazonaws.com".into(),
            "my-bucket".into(), "us-east-1".into(),
            "AKIA...".into(), "secret...".into(), "prefix/".into(),
        );
        assert!(adapter.is_configured());
    }

    #[tokio::test]
    async fn test_s3_construct_does_not_panic_with_empty() {
        let _adapter = S3StorageAdapter::new(
            String::new(), String::new(), String::new(),
            String::new(), String::new(), String::new(),
        );
    }
}

// step-o2 (2026-06-16, outbound-umbrella-1): OutboundManifest 박힘
impl file_pipeline_core::ports::outbound::OutboundManifest for S3StorageAdapter {
    fn id(&self) -> &str { "fp-outbound-storage-s3" }
    fn category(&self) -> file_pipeline_core::ports::outbound::OutboundCategory {
        file_pipeline_core::ports::outbound::OutboundCategory::Storage
    }
    fn capabilities(&self) -> file_pipeline_core::ports::output::ResourceCapabilities {
        file_pipeline_core::ports::output::ResourceCapabilities::standard("s3")
    }
}
