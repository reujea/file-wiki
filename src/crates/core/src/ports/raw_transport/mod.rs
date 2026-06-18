//! `RawTransport` 채널 — transport-flatten-1 step-t1 (2026-06-17) 정합.
//!
//! 본질 재정의 3차 (plugin-architecture-2026-06-04.md §3-C): **Output adapter = raw I/O 만**.
//! 기존 `OutboundManifest` super-trait (outbound-umbrella-1, 도메인 메타데이터 우산) = **폐기 완료**
//! (plugin-sdk-1 step-p7, 2026-06-18 — outbound/mod.rs 본체 + 6 port super-trait bound + 어댑터/테스트
//! impl 전량 제거). 도메인 로직 (telegram mode/sqlite mapping/48h/50MB, Notion page-attach,
//! LLM 형식+파싱, fastembed batch 등 9 영역) = plugin 본문 이관 (step-t4).
//!
//! adapter 잔류 책임 = **순수 전송 채널 호출만**. 4 transport 채널:
//!
//! | 채널 | 책임 | 기존 어댑터 매핑 (step-t3 단순화 대상) |
//! |------|------|--------------------------------------|
//! | `HttpTransport`     | HTTP req/resp raw byte | s3/webdav/notion/telegram/slack/anthropic/openai/gemini/ollama |
//! | `FilesystemTransport`| 로컬 파일 read/write   | network storage / local embed / fastembed (모델 파일) |
//! | `StdioTransport`    | 자식 프로세스 stdin/stdout | claude CLI / python-onnx |
//! | `SqliteTransport`   | settings.db 등 sqlite I/O | telegram_message_map / llm_cache 등 |
//!
//! ## 본 step-t1 영역 (skeleton)
//!
//! 본 mod = 4 trait 정의만 박힘 (impl 부재). adapters/driven/transport/{http_client,fs,stdio}.rs =
//! step-t2 신설 (SDK 호출 wrap). 24 어댑터 → transport 호출 잔류 단순화 = step-t3.
//!
//! ## step-t1 부재 영역 (후속 step)
//!
//! - adapters/driven/transport/* 구현체 신설 — step-t2
//! - 24 어댑터 transport 호출 단순화 + 도메인 로직 마킹 — step-t3
//! - 도메인 로직 9 영역 plugin 본문 이관 — step-t4
//! - telegram_storage.rs 291→~50줄 검증 + plugin manifest — step-t5
//! - OutboundManifest 폐기 완료 (plugin-sdk-1 step-p7) + spec 정합 — step-p8

use anyhow::Result;
use async_trait::async_trait;

/// raw transport 호출 1건의 메타 — 채널 무관 공통 (타임아웃/재시도 등은 caller plugin 책임).
#[derive(Debug, Clone, Default)]
pub struct TransportMeta {
    /// 호출 식별 (audit trace_id 부착용, 예: "fp-outbound-storage-telegram").
    pub source_id: String,
    /// 모드 (telegram document/text/channel 등 — plugin 결정, transport 는 불투명 전달).
    pub mode: Option<String>,
}

/// HTTP 전송 채널 — req/resp raw byte. 도메인 형식(multipart, JSON 파싱)은 plugin 책임.
#[async_trait]
pub trait HttpTransport: Send + Sync {
    /// raw 요청 전송 → raw 응답 byte. method/url/headers/body 전부 plugin 이 구성.
    async fn send(
        &self,
        method: &str,
        url: &str,
        headers: &[(String, String)],
        body: &[u8],
        meta: &TransportMeta,
    ) -> Result<HttpResponse>;
}

/// HTTP raw 응답 — status + headers + body byte. 파싱은 plugin 책임.
#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

/// 로컬 파일 전송 채널 — read/write raw byte.
#[async_trait]
pub trait FilesystemTransport: Send + Sync {
    /// 경로 read → raw byte.
    async fn read(&self, path: &str, meta: &TransportMeta) -> Result<Vec<u8>>;

    /// 경로 write (raw byte). 디렉토리 자동 생성은 구현체 결정.
    async fn write(&self, path: &str, bytes: &[u8], meta: &TransportMeta) -> Result<()>;

    /// 경로 삭제 — 미존재 시 false.
    async fn delete(&self, path: &str, meta: &TransportMeta) -> Result<bool>;
}

/// 자식 프로세스 stdio 전송 채널 — stdin write → stdout raw 수집.
#[async_trait]
pub trait StdioTransport: Send + Sync {
    /// 프로세스 spawn + stdin 주입 → stdout raw byte (exit code 비0 시 Err).
    async fn invoke(
        &self,
        program: &str,
        args: &[&str],
        stdin: &[u8],
        meta: &TransportMeta,
    ) -> Result<Vec<u8>>;
}

/// sqlite I/O 전송 채널 — settings.db 등 직접 접근 (telegram_message_map/llm_cache).
///
/// 본 채널은 raw key-value 영역만 노출 — 스키마/도메인 의미는 plugin 책임.
#[async_trait]
pub trait SqliteTransport: Send + Sync {
    /// 단일 row 조회 (key → JSON value).
    async fn get(&self, table: &str, key: &str, meta: &TransportMeta) -> Result<Option<String>>;

    /// 단일 row upsert (key → JSON value).
    async fn put(&self, table: &str, key: &str, value: &str, meta: &TransportMeta) -> Result<()>;

    /// 단일 row 삭제 — 미존재 시 false.
    async fn delete(&self, table: &str, key: &str, meta: &TransportMeta) -> Result<bool>;
}
