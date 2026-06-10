//! Plugin handle — discover 된 plugin 1건의 매니페스트 + 권한 + 활성 상태.
//!
//! Phase 201 placeholder — IPC 연결 처리는 Phase 202.

use std::path::PathBuf;

use fp_plugin_protocol::PluginManifest;

use crate::plugin::permission_gate::PermissionGate;

/// plugin 활성 상태.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginState {
    /// 디스크에 발견되어 매니페스트 로드 완료. host에 contribute 미적용.
    Discovered,
    /// `enable` 호출됨. host MCP/Tauri 라우터에 contribute 등록.
    Enabled,
    /// 명시 비활성 (사용자 결정 또는 enable 실패).
    Disabled,
}

/// discover 된 plugin 1건의 host 측 핸들.
///
/// Phase 201은 정적 메타데이터만 보유. Phase 202(IPC bus) 시점에:
/// - 프로세스 spawn / 연결 / 재시작 정책
/// - `call(method, params, trace_id)` 실제 IPC
#[derive(Debug, Clone)]
pub struct PluginHandle {
    /// 매니페스트 (fp-plugin.toml 파싱 결과)
    pub manifest: PluginManifest,
    /// 매니페스트 파일 위치 (`PIPELINE_BASE/plugins/{id}/fp-plugin.toml`)
    pub manifest_path: PathBuf,
    /// permission gate (검증 완료)
    pub permission: PermissionGate,
    /// 현재 활성 상태
    pub state: PluginState,
}

impl PluginHandle {
    /// 매니페스트 + permission gate 으로 핸들 생성. 초기 상태는 `Discovered`.
    pub fn new(
        manifest: PluginManifest,
        manifest_path: PathBuf,
        permission: PermissionGate,
    ) -> PluginHandle {
        PluginHandle {
            manifest,
            manifest_path,
            permission,
            state: PluginState::Discovered,
        }
    }

    /// plugin id (매니페스트 필드 단축 접근).
    pub fn id(&self) -> &str {
        &self.manifest.id
    }

    /// 사용자 가시 이름.
    pub fn name(&self) -> &str {
        &self.manifest.name
    }

    /// 활성 여부.
    pub fn is_enabled(&self) -> bool {
        self.state == PluginState::Enabled
    }
}
