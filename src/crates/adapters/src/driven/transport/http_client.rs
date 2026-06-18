//! `HttpTransport` 구현체 — transport-flatten-1 step-t2 (2026-06-18).
//!
//! `reqwest::Client` (async) wrap. method/url/headers/body 를 불투명 byte 로 전달하고
//! status+headers+body 를 raw 수집한다. **도메인 로직 0**: multipart 구성, JSON 직렬화/파싱,
//! telegram 48h/50MB, Notion page mode 등은 전부 plugin/caller 책임.

use anyhow::{Context, Result};
use async_trait::async_trait;
use file_pipeline_core::ports::raw_transport::{HttpResponse, HttpTransport, TransportMeta};
use reqwest::{Client, Method};

/// reqwest 기반 HTTP raw transport. 상태는 재사용 가능한 `Client` 1개만.
pub struct ReqwestHttpTransport {
    client: Client,
}

impl ReqwestHttpTransport {
    /// 디폴트 `Client` 로 생성.
    pub fn new() -> Self {
        Self { client: Client::new() }
    }

    /// 외부에서 구성한 `Client` 주입 (proxy/timeout 등 caller 결정).
    pub fn with_client(client: Client) -> Self {
        Self { client }
    }
}

impl Default for ReqwestHttpTransport {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl HttpTransport for ReqwestHttpTransport {
    async fn send(
        &self,
        method: &str,
        url: &str,
        headers: &[(String, String)],
        body: &[u8],
        _meta: &TransportMeta,
    ) -> Result<HttpResponse> {
        let method = Method::from_bytes(method.as_bytes())
            .with_context(|| format!("잘못된 HTTP method: {method}"))?;

        let mut req = self.client.request(method, url);
        for (name, value) in headers {
            req = req.header(name.as_str(), value.as_str());
        }
        // body 는 불투명 byte 전달 — 빈 body 도 그대로 둔다 (caller 가 Content-Length 제어).
        req = req.body(body.to_vec());

        let resp = req.send().await.with_context(|| format!("HTTP 요청 실패: {url}"))?;

        let status = resp.status().as_u16();
        let resp_headers: Vec<(String, String)> = resp
            .headers()
            .iter()
            .map(|(k, v)| (k.as_str().to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let resp_body = resp.bytes().await.context("HTTP 응답 body 수집 실패")?.to_vec();

        Ok(HttpResponse { status, headers: resp_headers, body: resp_body })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_and_default_construct() {
        let _ = ReqwestHttpTransport::new();
        let _ = ReqwestHttpTransport::default();
    }

    #[tokio::test]
    async fn test_invalid_method_errors() {
        let t = ReqwestHttpTransport::new();
        let meta = TransportMeta::default();
        // 공백 포함 method = invalid → 네트워크 도달 전 Err.
        let r = t.send("BAD METHOD", "http://127.0.0.1:1/x", &[], b"", &meta).await;
        assert!(r.is_err());
    }
}
