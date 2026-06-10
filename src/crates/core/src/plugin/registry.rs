//! PluginRegistry — `PIPELINE_BASE/plugins/` discover + 핸들 보관.
//!
//! Phase 201 placeholder — discover + enable/disable 까지만. **실제 IPC 호출은
//! Phase 202** (`call` 은 `PluginError::IpcNotYetImplemented` 반환).
//!
//! 단일 진실원: `prd/research/plugin-architecture-2026-06-04.md` §5-C

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use fp_plugin_protocol::{parse_manifest_toml, ProtocolError, API_VERSION};
use thiserror::Error;

use crate::plugin::handle::{PluginHandle, PluginState};
use crate::plugin::permission_gate::PermissionGate;

/// PluginRegistry / discover / IPC 호출 에러.
#[derive(Debug, Error)]
pub enum PluginError {
    #[error("plugin 디렉토리 접근: {0}")]
    Io(String),
    #[error("매니페스트 파싱 ({plugin_dir:?}): {source}")]
    ManifestParse {
        plugin_dir: PathBuf,
        #[source]
        source: ProtocolError,
    },
    #[error(
        "api_version 불일치: host={host}, plugin={plugin} (plugin_id={plugin_id})"
    )]
    ApiVersionMismatch {
        plugin_id: String,
        host: u32,
        plugin: u32,
    },
    #[error("알 수 없는 권한 '{permission}' (plugin_id={plugin_id})")]
    UnknownPermission {
        plugin_id: String,
        permission: String,
    },
    #[error("동일 plugin id 중복: '{0}'")]
    DuplicatePluginId(String),
    #[error("plugin id '{0}' 미존재")]
    PluginNotFound(String),
    #[error("Phase 202 IPC bus 미구현 — wire 타입(IpcMessage/IpcResponse/HostEvent)은 정의되었으나 named pipe / Unix domain socket 전송은 다음 진입 시점에 구현")]
    IpcNotYetImplemented,
}

/// plugin 디렉토리 discover + 핸들 보관 registry.
///
/// Phase 201은 in-memory only. Phase 208(GUI Plugins 탭) 시점에 enable/disable
/// 상태 settings.db 영속 추가.
#[derive(Debug, Default)]
pub struct PluginRegistry {
    plugins: HashMap<String, PluginHandle>,
}

impl PluginRegistry {
    pub fn new() -> PluginRegistry {
        PluginRegistry::default()
    }

    /// `PIPELINE_BASE/plugins/` 디렉토리 스캔 → 매니페스트 1건당 PluginHandle 등록.
    ///
    /// 디렉토리 구조:
    /// ```text
    /// plugins/
    ///   io.file-pipeline.search/
    ///     fp-plugin.toml          ← 매니페스트
    ///     fp-plugin-search.exe    ← Phase 202 IPC 실행 binary
    ///   io.file-pipeline.kg/
    ///     fp-plugin.toml
    ///     ...
    /// ```
    ///
    /// 디렉토리 자체가 부재해도 OK — plugin 0개로 정상 부팅 (lesson 29 PIPELINE_BASE
    /// 패턴). plugin 디렉토리 내 `fp-plugin.toml` 부재면 해당 디렉토리 스킵.
    ///
    /// 매니페스트 파싱 실패 / api_version 불일치 / 알 수 없는 권한 1건이라도 발견되면
    /// **전체 discover 중단**. 부분 등록 회피로 부팅 시 부분 실패 가시화.
    pub fn discover(&mut self, plugins_root: &Path) -> Result<usize, PluginError> {
        if !plugins_root.exists() {
            // 첫 부팅 시 디렉토리 부재 — config.rs::create_all 이 곧 생성. 본 시점은 0건 정상.
            return Ok(0);
        }
        let entries = std::fs::read_dir(plugins_root)
            .map_err(|e| PluginError::Io(format!("read_dir({:?}): {}", plugins_root, e)))?;

        let mut discovered = 0;
        for entry in entries {
            let entry = entry.map_err(|e| PluginError::Io(format!("entry: {}", e)))?;
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let manifest_path = path.join("fp-plugin.toml");
            if !manifest_path.exists() {
                continue; // 빈 디렉토리 / non-plugin 디렉토리 — 스킵
            }
            let toml_str = std::fs::read_to_string(&manifest_path)
                .map_err(|e| PluginError::Io(format!("read {:?}: {}", manifest_path, e)))?;
            let manifest = parse_manifest_toml(&toml_str).map_err(|e| {
                PluginError::ManifestParse {
                    plugin_dir: path.clone(),
                    source: e,
                }
            })?;
            if manifest.api_version != API_VERSION {
                return Err(PluginError::ApiVersionMismatch {
                    plugin_id: manifest.id.clone(),
                    host: API_VERSION,
                    plugin: manifest.api_version,
                });
            }
            let permission =
                PermissionGate::from_manifest_permissions(&manifest.permissions, &manifest.id)?;
            if self.plugins.contains_key(&manifest.id) {
                return Err(PluginError::DuplicatePluginId(manifest.id.clone()));
            }
            let id = manifest.id.clone();
            let handle = PluginHandle::new(manifest, manifest_path, permission);
            self.plugins.insert(id, handle);
            discovered += 1;
        }
        Ok(discovered)
    }

    /// 발견된 plugin 활성화 — host MCP/Tauri 라우터 등록 (Phase 208 자기 적용 예정).
    /// Phase 201은 상태만 전이.
    pub fn enable(&mut self, plugin_id: &str) -> Result<(), PluginError> {
        let handle = self
            .plugins
            .get_mut(plugin_id)
            .ok_or_else(|| PluginError::PluginNotFound(plugin_id.to_string()))?;
        handle.state = PluginState::Enabled;
        Ok(())
    }

    /// plugin 비활성화.
    pub fn disable(&mut self, plugin_id: &str) -> Result<(), PluginError> {
        let handle = self
            .plugins
            .get_mut(plugin_id)
            .ok_or_else(|| PluginError::PluginNotFound(plugin_id.to_string()))?;
        handle.state = PluginState::Disabled;
        Ok(())
    }

    /// plugin 호출 — **Phase 202 진입 전이라 미구현**. 호출자는 `IpcNotYetImplemented`
    /// 처리 의무.
    pub async fn call(
        &self,
        _plugin_id: &str,
        _method: &str,
        _params: serde_json::Value,
        _trace_id: &str,
    ) -> Result<serde_json::Value, PluginError> {
        Err(PluginError::IpcNotYetImplemented)
    }

    pub fn count(&self) -> usize {
        self.plugins.len()
    }

    pub fn list(&self) -> Vec<&PluginHandle> {
        let mut v: Vec<_> = self.plugins.values().collect();
        v.sort_by(|a, b| a.id().cmp(b.id()));
        v
    }

    pub fn get(&self, plugin_id: &str) -> Option<&PluginHandle> {
        self.plugins.get(plugin_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn write_manifest(plugins_root: &Path, plugin_dir_name: &str, manifest_toml: &str) -> PathBuf {
        let plugin_dir = plugins_root.join(plugin_dir_name);
        fs::create_dir_all(&plugin_dir).unwrap();
        let manifest_path = plugin_dir.join("fp-plugin.toml");
        fs::write(&manifest_path, manifest_toml).unwrap();
        manifest_path
    }

    #[test]
    fn discover_empty_dir_returns_zero() {
        let dir = tempdir().unwrap();
        let mut registry = PluginRegistry::new();
        let n = registry.discover(dir.path()).unwrap();
        assert_eq!(n, 0);
        assert_eq!(registry.count(), 0);
    }

    #[test]
    fn discover_missing_dir_returns_zero() {
        let mut registry = PluginRegistry::new();
        let n = registry
            .discover(Path::new("/nonexistent/plugins/path"))
            .unwrap();
        assert_eq!(n, 0);
    }

    #[test]
    fn discover_single_plugin_registers() {
        let dir = tempdir().unwrap();
        write_manifest(
            dir.path(),
            "io.file-pipeline.search",
            r#"
manifest_version = 1
id = "io.file-pipeline.search"
name = "Search"
version = "0.1.0"
api_version = 1
permissions = ["vector_db.read", "audit.write"]

[entry]
type = "process"
command = "fp-plugin-search"
"#,
        );
        let mut registry = PluginRegistry::new();
        let n = registry.discover(dir.path()).unwrap();
        assert_eq!(n, 1);
        let handle = registry.get("io.file-pipeline.search").unwrap();
        assert_eq!(handle.name(), "Search");
        assert_eq!(handle.state, PluginState::Discovered);
        assert_eq!(handle.permission.count(), 2);
    }

    #[test]
    fn discover_rejects_api_version_mismatch() {
        let dir = tempdir().unwrap();
        write_manifest(
            dir.path(),
            "io.file-pipeline.future",
            r#"
manifest_version = 1
id = "io.file-pipeline.future"
name = "Future"
version = "0.1.0"
api_version = 99

[entry]
type = "process"
command = "fp-plugin-future"
"#,
        );
        let mut registry = PluginRegistry::new();
        let err = registry.discover(dir.path()).unwrap_err();
        match err {
            PluginError::ApiVersionMismatch { plugin, host, .. } => {
                assert_eq!(plugin, 99);
                assert_eq!(host, API_VERSION);
            }
            other => panic!("기대: ApiVersionMismatch, 실제: {:?}", other),
        }
    }

    #[test]
    fn discover_rejects_unknown_permission() {
        let dir = tempdir().unwrap();
        write_manifest(
            dir.path(),
            "io.file-pipeline.bad",
            r#"
manifest_version = 1
id = "io.file-pipeline.bad"
name = "Bad"
version = "0.1.0"
api_version = 1
permissions = ["totally.unknown"]

[entry]
type = "process"
command = "fp-plugin-bad"
"#,
        );
        let mut registry = PluginRegistry::new();
        let err = registry.discover(dir.path()).unwrap_err();
        assert!(matches!(err, PluginError::UnknownPermission { .. }));
    }

    #[test]
    fn discover_rejects_duplicate_id() {
        let dir = tempdir().unwrap();
        let manifest_a = r#"
manifest_version = 1
id = "io.file-pipeline.dup"
name = "A"
version = "0.1.0"
api_version = 1

[entry]
type = "process"
command = "fp-plugin-a"
"#;
        let manifest_b = r#"
manifest_version = 1
id = "io.file-pipeline.dup"
name = "B"
version = "0.2.0"
api_version = 1

[entry]
type = "process"
command = "fp-plugin-b"
"#;
        write_manifest(dir.path(), "dir-a", manifest_a);
        write_manifest(dir.path(), "dir-b", manifest_b);
        let mut registry = PluginRegistry::new();
        let err = registry.discover(dir.path()).unwrap_err();
        assert!(matches!(err, PluginError::DuplicatePluginId(_)));
    }

    #[test]
    fn enable_disable_transitions() {
        let dir = tempdir().unwrap();
        write_manifest(
            dir.path(),
            "io.file-pipeline.toggle",
            r#"
manifest_version = 1
id = "io.file-pipeline.toggle"
name = "Toggle"
version = "0.1.0"
api_version = 1

[entry]
type = "process"
command = "fp-plugin-toggle"
"#,
        );
        let mut registry = PluginRegistry::new();
        registry.discover(dir.path()).unwrap();
        assert_eq!(
            registry.get("io.file-pipeline.toggle").unwrap().state,
            PluginState::Discovered
        );
        registry.enable("io.file-pipeline.toggle").unwrap();
        assert!(registry.get("io.file-pipeline.toggle").unwrap().is_enabled());
        registry.disable("io.file-pipeline.toggle").unwrap();
        assert_eq!(
            registry.get("io.file-pipeline.toggle").unwrap().state,
            PluginState::Disabled
        );
    }

    #[test]
    fn enable_missing_plugin_errors() {
        let mut registry = PluginRegistry::new();
        let err = registry.enable("io.file-pipeline.ghost").unwrap_err();
        assert!(matches!(err, PluginError::PluginNotFound(_)));
    }

    #[tokio::test]
    async fn call_not_yet_implemented() {
        let registry = PluginRegistry::new();
        let err = registry
            .call("io.x", "m", serde_json::json!({}), "trace-1")
            .await
            .unwrap_err();
        assert!(matches!(err, PluginError::IpcNotYetImplemented));
    }
}
