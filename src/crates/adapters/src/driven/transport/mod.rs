//! raw transport 구현체 — transport-flatten-1 step-t2 (2026-06-18).
//!
//! 본질 재정의 3차 (plugin-architecture-2026-06-04.md §3-C): **Output adapter = raw I/O 만**.
//! core `ports::raw_transport` 의 4 trait (Http/Filesystem/Stdio/Sqlite) 구현체. 도메인 로직 0 —
//! 불투명 byte 전달만. 도메인 형식(multipart/JSON/telegram mode/Notion page 등) = plugin 책임.

pub mod fs;
pub mod http_client;
pub mod sqlite;
pub mod stdio;
