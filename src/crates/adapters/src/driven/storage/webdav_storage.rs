use std::path::Path;

use anyhow::Result;
use async_trait::async_trait;
use file_pipeline_core::ports::output::{RemoteStoragePort, ResourceCapabilities};
use module_storage::RemoteStoragePort as RawRemote;
use module_storage::WebDavRemoteStorage;

use super::map_err;

pub struct WebDavStorageAdapter {
    inner: WebDavRemoteStorage,
}

impl WebDavStorageAdapter {
    pub fn new(base_url: String, username: String, password: String, prefix: String) -> Self {
        Self {
            inner: WebDavRemoteStorage::new(base_url, username, password, prefix),
        }
    }
}

#[async_trait]
impl RemoteStoragePort for WebDavStorageAdapter {
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
        ResourceCapabilities::standard("webdav")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_webdav_is_configured_always_true() {
        // module-storage WebDavRemoteStorage::is_configured는 어댑터 활성화 = 설정됨으로 간주.
        // NullRemoteStorage만 false. 실제 설정 검증은 first request 시 네트워크 에러로 발견.
        let adapter = WebDavStorageAdapter::new(
            "https://webdav.example.com/".into(),
            "user".into(), "pass".into(), "/backup".into(),
        );
        assert!(adapter.is_configured());
    }

    #[tokio::test]
    async fn test_webdav_construct_does_not_panic_with_empty() {
        // 빈 인자로도 생성은 성공 (lazy 검증)
        let _adapter = WebDavStorageAdapter::new(
            String::new(), String::new(), String::new(), String::new(),
        );
    }
}
