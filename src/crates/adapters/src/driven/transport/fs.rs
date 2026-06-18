//! `FilesystemTransport` 구현체 — transport-flatten-1 step-t2 (2026-06-18).
//!
//! `tokio::fs` (async) 기반 로컬 파일 read/write/delete. raw byte 만 전달한다.
//! **도메인 로직 0**: 압축/인코딩/경로 해석 규칙 등은 plugin/caller 책임.

use anyhow::{Context, Result};
use async_trait::async_trait;
use file_pipeline_core::ports::raw_transport::{FilesystemTransport, TransportMeta};

/// tokio::fs 기반 파일 raw transport. 상태 없음.
pub struct TokioFsTransport;

impl TokioFsTransport {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TokioFsTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl FilesystemTransport for TokioFsTransport {
    async fn read(&self, path: &str, _meta: &TransportMeta) -> Result<Vec<u8>> {
        tokio::fs::read(path).await.with_context(|| format!("파일 read 실패: {path}"))
    }

    async fn write(&self, path: &str, bytes: &[u8], _meta: &TransportMeta) -> Result<()> {
        // 부모 디렉토리 자동 생성 (raw write 편의 — 도메인 의미 없음).
        if let Some(parent) = std::path::Path::new(path).parent() {
            if !parent.as_os_str().is_empty() {
                tokio::fs::create_dir_all(parent)
                    .await
                    .with_context(|| format!("디렉토리 생성 실패: {}", parent.display()))?;
            }
        }
        tokio::fs::write(path, bytes).await.with_context(|| format!("파일 write 실패: {path}"))
    }

    async fn delete(&self, path: &str, _meta: &TransportMeta) -> Result<bool> {
        match tokio::fs::remove_file(path).await {
            Ok(()) => Ok(true),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(e) => Err(e).with_context(|| format!("파일 삭제 실패: {path}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_write_read_delete_roundtrip() {
        let t = TokioFsTransport::new();
        let meta = TransportMeta::default();
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("sub").join("raw.bin");
        let path_str = path.to_str().expect("utf8 path");

        t.write(path_str, b"hello-raw", &meta).await.expect("write");
        let got = t.read(path_str, &meta).await.expect("read");
        assert_eq!(got, b"hello-raw");

        assert!(t.delete(path_str, &meta).await.expect("delete"));
        // 두 번째 삭제 = 미존재 → false.
        assert!(!t.delete(path_str, &meta).await.expect("delete2"));
    }
}
