//! Plugin discovery + registry + IPC + permission gate (host 측).
//!
//! Phase 201/202 본진입 — plugin-architecture-2026-06-04.md §5-C 매핑.
//!
//! 본 모듈은 `PIPELINE_BASE/plugins/` 의 plugin discovery + 매니페스트 파싱 +
//! permission 검사 + 핸들 보관 + **실제 IPC 호출** (Phase 202)을 담당.
//! `PluginRegistry::call`은 ConnectionPool로 plugin 프로세스와 통신.
//!
//! 단일 진실원: `prd/research/plugin-architecture-2026-06-04.md` §5-C
//! 의존: `fp-plugin-protocol` (wire types) + `fp-plugin-sdk` (Connection)
//!
//! ## 책임 분리
//!
//! - [`PermissionGate`] — plugin이 요구한 권한 집합 검증 (이름 매칭)
//! - [`PluginHandle`] — 발견된 plugin 1건의 매니페스트 + 권한 + 활성 상태
//! - [`PluginRegistry`] — plugins/ 디렉토리 discover + id 인덱스 + enable/disable + call + broadcast_event
//! - [`ConnectionPool`] — plugin_id별 active connection 1개 lazy-init + 재사용
//!
//! ## Phase 진행
//!
//! - Phase 200: fp-plugin-protocol + fp-plugin-sdk placeholder (완료)
//! - Phase 201: PluginRegistry::discover + permission gate (완료)
//! - **Phase 202 (본진입)**: ConnectionPool + PluginRegistry::{call, broadcast_event} 실 구현
//! - Phase 208: GUI Plugins 탭 — registry.list + enable/disable Tauri command

pub mod connection_pool;
pub mod handle;
pub mod permission_gate;
pub mod registry;

pub use connection_pool::ConnectionPool;
pub use handle::{PluginHandle, PluginState};
pub use permission_gate::{KnownPermission, PermissionGate};
pub use registry::{PluginError, PluginRegistry};

// fp-plugin-protocol 재노출 (host 호출자 편의)
pub use fp_plugin_protocol::{parse_manifest_toml, HostEvent, IpcMessage, IpcResponse, PluginManifest, API_VERSION};
