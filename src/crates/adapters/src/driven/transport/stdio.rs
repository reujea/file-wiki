//! `StdioTransport` 구현체 — transport-flatten-1 step-t2 (2026-06-18).
//!
//! `tokio::process::Command` 로 자식 프로세스 spawn + stdin 주입 → stdout raw 수집.
//! exit code 비0 시 Err. **도메인 로직 0**: 프롬프트 구성, stdout JSON 파싱 등은
//! plugin/caller 책임 (claude CLI / python-onnx wrapper 가 호출처).

use std::process::Stdio;

use anyhow::{bail, Context, Result};
use async_trait::async_trait;
use file_pipeline_core::ports::raw_transport::{StdioTransport, TransportMeta};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

/// tokio::process 기반 stdio raw transport. 상태 없음.
pub struct TokioStdioTransport;

impl TokioStdioTransport {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TokioStdioTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl StdioTransport for TokioStdioTransport {
    async fn invoke(
        &self,
        program: &str,
        args: &[&str],
        stdin: &[u8],
        _meta: &TransportMeta,
    ) -> Result<Vec<u8>> {
        let mut child = Command::new(program)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("프로세스 spawn 실패: {program}"))?;

        // stdin 주입 (없어도 take 후 drop = EOF).
        if let Some(mut sink) = child.stdin.take() {
            sink.write_all(stdin).await.context("자식 프로세스 stdin write 실패")?;
            sink.flush().await.context("자식 프로세스 stdin flush 실패")?;
            drop(sink);
        }

        let output = child
            .wait_with_output()
            .await
            .with_context(|| format!("프로세스 종료 대기 실패: {program}"))?;

        if !output.status.success() {
            let code = output.status.code().map(|c| c.to_string()).unwrap_or_else(|| "signal".into());
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("프로세스 비정상 종료 ({program}, exit={code}): {stderr}");
        }

        Ok(output.stdout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[tokio::test]
    async fn test_cat_echoes_stdin() {
        let t = TokioStdioTransport::new();
        let meta = TransportMeta::default();
        let out = t.invoke("cat", &[], b"raw-bytes", &meta).await.expect("cat");
        assert_eq!(out, b"raw-bytes");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn test_nonzero_exit_errors() {
        let t = TokioStdioTransport::new();
        let meta = TransportMeta::default();
        let r = t.invoke("false", &[], b"", &meta).await;
        assert!(r.is_err());
    }
}
