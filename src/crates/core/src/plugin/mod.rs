//! Plugin discovery + registry + permission gate (host 측).
//!
//! Phase 201 placeholder — plugin-architecture-2026-06-04.md §5-C 매핑.
//!
//! 본 모듈은 `PIPELINE_BASE/plugins/` 의 plugin discovery + 매니페스트 파싱 +
//! permission 검사 + 핸들 보관까지만 담당. **실제 IPC 호출은 Phase 202**에서
//! `PluginHandle::call` 이 `Err(PluginError::IpcNotYetImplemented)` 반환.
//!
//! 단일 진실원: `prd/research/plugin-architecture-2026-06-04.md` §5-C
//! 의존: `fp-plugin-protocol`(`PluginManifest`, `parse_manifest_toml`)
//!
//! ## 책임 분리
//!
//! - [`PermissionGate`] — plugin이 요구한 권한 집합 검증 (이름 매칭)
//! - [`PluginHandle`] — 발견된 plugin 1건의 매니페스트 + 권한 + 활성 상태
//! - [`PluginRegistry`] — plugins/ 디렉토리 discover + id 인덱스 + enable/disable
//!
//! ## Phase 진행
//!
//! - Phase 200: fp-plugin-protocol + fp-plugin-sdk placeholder (완료)
//! - **Phase 201 (본 모듈)**: PluginRegistry::discover + permission gate (placeholder)
//! - Phase 202: IPC bus 진입 — PluginHandle::call 실제 구현
//! - Phase 208: GUI Plugins 탭 — registry.list + enable/disable Tauri command

pub mod handle;
pub mod permission_gate;
pub mod registry;

pub use handle::{PluginHandle, PluginState};
pub use permission_gate::{KnownPermission, PermissionGate};
pub use registry::{PluginError, PluginRegistry};

// fp-plugin-protocol 재노출 (host 호출자 편의)
pub use fp_plugin_protocol::{parse_manifest_toml, PluginManifest, API_VERSION};
