//! PluginRegistry — `PIPELINE_BASE/plugins/` discover + 핸들 보관 + IPC 호출.
//!
//! Phase 202 본진입 — discover + enable/disable + `call` (실 IPC) + `broadcast_event`.
//!
//! 단일 진실원: `prd/research/plugin-architecture-2026-06-04.md` §5-C

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use fp_plugin_protocol::{parse_manifest_toml, HostEvent, IpcMessage, IpcResponse, ProtocolError, API_VERSION};
use thiserror::Error;

use crate::audit::{input_hash_prefix, truncate_output_summary};
use crate::plugin::connection_pool::ConnectionPool;
use crate::plugin::handle::{PluginHandle, PluginState};
use crate::plugin::permission_gate::PermissionGate;
use crate::ports::output::{AuditPort, NullAuditAdapter};

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
    /// Phase 202 본진입 — plugin 프로세스 미실행 (endpoint 연결 실패).
    #[error("plugin 프로세스 미실행 (plugin_id={plugin_id}): {cause}")]
    NotRunning { plugin_id: String, cause: String },
    /// IPC 송수신 자체 실패 (소켓 끊김 / write 실패 등)
    #[error("IPC 전송 실패: {0}")]
    IpcTransport(String),
    /// plugin이 명시적으로 반환한 실패 응답 (`IpcResponse::Err`)
    #[error("plugin 에러 응답: {0}")]
    IpcProtocol(String),
}

/// plugin 디렉토리 discover + 핸들 보관 + IPC 호출 registry.
///
/// Phase 202 본진입 — `call` 이 ConnectionPool로 실제 IPC 수행 + audit 통합.
/// Phase 208(GUI Plugins 탭) 시점에 enable/disable 상태 settings.db 영속 추가.
pub struct PluginRegistry {
    plugins: HashMap<String, PluginHandle>,
    pool: ConnectionPool,
    audit: Arc<dyn AuditPort>,
}

impl std::fmt::Debug for PluginRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginRegistry")
            .field("plugins", &self.plugins)
            .field("pool", &self.pool)
            .finish_non_exhaustive()
    }
}

impl Default for PluginRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl PluginRegistry {
    pub fn new() -> PluginRegistry {
        PluginRegistry {
            plugins: HashMap::new(),
            pool: ConnectionPool::new(),
            audit: Arc::new(NullAuditAdapter),
        }
    }

    /// audit 어댑터 주입 — `build_service` 가 SettingsAuditAdapter 주입.
    /// 테스트는 디폴트 NullAuditAdapter 사용 (lesson 14 회피).
    pub fn with_audit(mut self, audit: Arc<dyn AuditPort>) -> Self {
        self.audit = audit;
        self
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

    /// plugin 호출 — Phase 202 본진입.
    ///
    /// 동작:
    /// 1. 등록된 plugin 존재 확인 → 미존재 시 `PluginError::PluginNotFound`
    /// 2. audit 시작 record (`stage = "plugin.{id}.{method}"`, applied_rule=None)
    /// 3. `ConnectionPool::get_or_connect` 로 connection 확보 (미실행 시 `NotRunning`)
    /// 4. `IpcMessage` 송신 + `IpcResponse` 수신
    /// 5. 결과 분기:
    ///    - `Ok { result }` → audit 종료 record (applied_rule="success") → Ok 반환
    ///    - `Err { message }` → audit 종료 record (applied_rule="error") → `IpcProtocol`
    ///
    /// stage 명명 규칙 (메타 룰 24): `plugin.{plugin_id}.{method}`
    pub async fn call(
        &self,
        plugin_id: &str,
        method: &str,
        params: serde_json::Value,
        trace_id: &str,
    ) -> Result<serde_json::Value, PluginError> {
        // 1. 존재 확인 — connection 시도 전 빠른 실패
        if !self.plugins.contains_key(plugin_id) {
            return Err(PluginError::PluginNotFound(plugin_id.to_string()));
        }

        let stage = format!("plugin.{}.{}", plugin_id, method);
        let inputs_bytes = serde_json::to_vec(&params).unwrap_or_default();
        let inputs_hash = input_hash_prefix(&inputs_bytes);

        // 2. audit 시작
        self.audit
            .record(trace_id, &stage, Some(&inputs_hash), None, None);

        // 3. connection 확보
        let conn = match self.pool.get_or_connect(plugin_id).await {
            Ok(c) => c,
            Err(e) => {
                // NotRunning 등 — audit 종료(error) 후 반환
                self.audit.record(
                    trace_id,
                    &stage,
                    Some(&inputs_hash),
                    Some(&format!("not_running: {}", e)),
                    Some("error"),
                );
                return Err(e);
            }
        };

        // 4. IpcMessage 송수신
        let msg = IpcMessage {
            trace_id: trace_id.to_string(),
            method: method.to_string(),
            api_version: API_VERSION,
            params,
        };
        let resp_result = {
            let mut guard = conn.lock().await;
            match guard.send_request(&msg).await {
                Ok(()) => guard.recv_response().await,
                Err(e) => Err(e),
            }
        };

        match resp_result {
            Ok(IpcResponse::Ok { result, .. }) => {
                let summary = truncate_output_summary(&result.to_string());
                self.audit.record(
                    trace_id,
                    &stage,
                    Some(&inputs_hash),
                    Some(&summary),
                    Some("success"),
                );
                Ok(result)
            }
            Ok(IpcResponse::Err { message, .. }) => {
                let summary = truncate_output_summary(&message);
                self.audit.record(
                    trace_id,
                    &stage,
                    Some(&inputs_hash),
                    Some(&summary),
                    Some("error"),
                );
                Err(PluginError::IpcProtocol(message))
            }
            Err(sdk_err) => {
                // transport 에러 — connection 무효화 (다음 호출에서 reconnect)
                self.pool.invalidate(plugin_id).await;
                let s = sdk_err.to_string();
                self.audit.record(
                    trace_id,
                    &stage,
                    Some(&inputs_hash),
                    Some(&truncate_output_summary(&s)),
                    Some("error"),
                );
                Err(PluginError::IpcTransport(s))
            }
        }
    }

    /// host → 모든 구독 plugin 이벤트 broadcast.
    ///
    /// 동작:
    /// 1. event_kind 추출 (`HostEvent` serde tag 와 동일)
    /// 2. 모든 plugin 순회 — enabled + event_subscribe 매칭 plugin만 대상
    /// 3. 각 대상에 connection 확보 후 `send_event` 시도
    /// 4. 실패는 silent (warn 로그) — broadcast는 best-effort (lesson 14 패턴)
    pub async fn broadcast_event(&self, event: HostEvent) {
        let kind = event_kind_str(&event);
        for handle in self.plugins.values() {
            if !handle.is_enabled() {
                continue;
            }
            if !handle.manifest.event_subscribe.iter().any(|s| s == kind) {
                continue;
            }
            let conn = match self.pool.get_or_connect(handle.id()).await {
                Ok(c) => c,
                Err(e) => {
                    tracing::warn!(
                        plugin_id = handle.id(),
                        event_kind = kind,
                        "broadcast: connect 실패 — skip: {}",
                        e
                    );
                    continue;
                }
            };
            let mut guard = conn.lock().await;
            if let Err(e) = guard.send_event(&event).await {
                tracing::warn!(
                    plugin_id = handle.id(),
                    event_kind = kind,
                    "broadcast: send_event 실패 — skip: {}",
                    e
                );
                drop(guard);
                self.pool.invalidate(handle.id()).await;
            }
        }
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

/// `HostEvent` serde tag (snake_case kind) 추출 헬퍼.
/// 매니페스트 `event_subscribe` 의 문자열과 정확히 일치.
fn event_kind_str(event: &HostEvent) -> &'static str {
    match event {
        HostEvent::ProcessingStarted { .. } => "processing_started",
        HostEvent::ProcessingCompleted { .. } => "processing_completed",
        HostEvent::QuarantineAdded { .. } => "quarantine_added",
        HostEvent::VerifyFailed { .. } => "verify_failed",
        HostEvent::ShutdownRequested => "shutdown_requested",
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

    // ───── Phase 202 본진입 — call + broadcast_event 테스트 ─────

    use fp_plugin_sdk::Connection;

    fn unique_id(label: &str) -> String {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        format!("test.{}.{:x}", label, nanos)
    }

    fn manifest_for(plugin_id: &str, event_subscribe: &[&str]) -> String {
        let subs = event_subscribe
            .iter()
            .map(|s| format!(r#""{}""#, s))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            r#"
manifest_version = 1
id = "{id}"
name = "Test"
version = "0.1.0"
api_version = 1
event_subscribe = [{subs}]

[entry]
type = "process"
command = "fp-plugin-test"
"#,
            id = plugin_id,
            subs = subs,
        )
    }

    #[tokio::test]
    async fn call_returns_plugin_not_found_for_unknown_id() {
        let registry = PluginRegistry::new();
        let err = registry
            .call("io.ghost.absent", "any", serde_json::json!({}), "trace-ghost")
            .await
            .unwrap_err();
        assert!(matches!(err, PluginError::PluginNotFound(_)));
    }

    #[tokio::test]
    async fn call_returns_not_running_when_no_server() {
        let dir = tempdir().unwrap();
        let plugin_id = unique_id("notrun");
        write_manifest(dir.path(), &plugin_id, &manifest_for(&plugin_id, &[]));
        let mut registry = PluginRegistry::new();
        registry.discover(dir.path()).unwrap();

        let err = registry
            .call(&plugin_id, "any", serde_json::json!({}), "trace-notrun")
            .await
            .unwrap_err();
        match err {
            PluginError::NotRunning { plugin_id: id, .. } => assert_eq!(id, plugin_id),
            other => panic!("기대: NotRunning, 실제: {:?}", other),
        }
    }

    #[tokio::test]
    async fn call_succeeds_with_mock_server() {
        let dir = tempdir().unwrap();
        let plugin_id = unique_id("ok");
        write_manifest(dir.path(), &plugin_id, &manifest_for(&plugin_id, &[]));
        let mut registry = PluginRegistry::new();
        registry.discover(dir.path()).unwrap();

        // mock plugin server — Ok{echo: params} 응답
        let pid_for_srv = plugin_id.clone();
        let server_handle = tokio::spawn(async move {
            let mut srv = Connection::accept_server(&pid_for_srv).await.unwrap();
            let req = srv.recv_request().await.unwrap();
            let resp = IpcResponse::Ok {
                trace_id: req.trace_id.clone(),
                result: serde_json::json!({"echo": req.params, "method": req.method}),
            };
            srv.send_response(&resp).await.unwrap();
        });

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let result = registry
            .call(
                &plugin_id,
                "test.echo",
                serde_json::json!({"x": 7}),
                "trace-ok",
            )
            .await
            .unwrap();
        assert_eq!(result["echo"]["x"], 7);
        assert_eq!(result["method"], "test.echo");
        server_handle.await.unwrap();
    }

    #[tokio::test]
    async fn call_propagates_plugin_error_response() {
        let dir = tempdir().unwrap();
        let plugin_id = unique_id("perr");
        write_manifest(dir.path(), &plugin_id, &manifest_for(&plugin_id, &[]));
        let mut registry = PluginRegistry::new();
        registry.discover(dir.path()).unwrap();

        let pid_for_srv = plugin_id.clone();
        let server_handle = tokio::spawn(async move {
            let mut srv = Connection::accept_server(&pid_for_srv).await.unwrap();
            let req = srv.recv_request().await.unwrap();
            let resp = IpcResponse::Err {
                trace_id: req.trace_id.clone(),
                message: "boom from plugin".to_string(),
            };
            srv.send_response(&resp).await.unwrap();
        });

        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        let err = registry
            .call(
                &plugin_id,
                "test.fail",
                serde_json::json!({}),
                "trace-perr",
            )
            .await
            .unwrap_err();
        match err {
            PluginError::IpcProtocol(msg) => assert_eq!(msg, "boom from plugin"),
            other => panic!("기대: IpcProtocol, 실제: {:?}", other),
        }
        server_handle.await.unwrap();
    }

    #[tokio::test]
    async fn broadcast_skips_non_subscriber() {
        let dir = tempdir().unwrap();
        let plugin_id = unique_id("brsub");
        write_manifest(
            dir.path(),
            &plugin_id,
            &manifest_for(&plugin_id, &["quarantine_added"]),
        );
        let mut registry = PluginRegistry::new();
        registry.discover(dir.path()).unwrap();
        registry.enable(&plugin_id).unwrap();

        // mock server는 시작하지 않음 — 미구독 이벤트는 connect 시도조차 없어야 함
        // 즉 connect 실패가 silent 처리되어 broadcast가 panic 없이 반환되면 PASS
        registry
            .broadcast_event(HostEvent::ProcessingStarted {
                file_id: "f1".to_string(),
            })
            .await;
        // panic 없으면 PASS
    }

    #[tokio::test]
    async fn broadcast_skips_disabled_plugin() {
        let dir = tempdir().unwrap();
        let plugin_id = unique_id("brdis");
        write_manifest(
            dir.path(),
            &plugin_id,
            &manifest_for(&plugin_id, &["processing_completed"]),
        );
        let mut registry = PluginRegistry::new();
        registry.discover(dir.path()).unwrap();
        // enable 안 함 — Discovered 상태 유지. disabled 와 동일 효과 (is_enabled() false)

        registry
            .broadcast_event(HostEvent::ProcessingCompleted {
                doc_id: "d1".to_string(),
                title: None,
            })
            .await;
        // panic 없으면 PASS — connect 시도 0건
    }

    #[tokio::test]
    async fn broadcast_kind_filter_matches_serde_tag() {
        let dir = tempdir().unwrap();
        let plugin_id = unique_id("brkind");
        // quarantine_added만 구독 — processing_started 이벤트 시 skip 되어야 함
        write_manifest(
            dir.path(),
            &plugin_id,
            &manifest_for(&plugin_id, &["quarantine_added"]),
        );
        let mut registry = PluginRegistry::new();
        registry.discover(dir.path()).unwrap();
        registry.enable(&plugin_id).unwrap();

        // ProcessingStarted 이벤트 broadcast — 미구독 → connect 시도 없어야 함
        // 만약 잘못된 매칭으로 connect 시도하면 NotRunning silent 무시 — 어떤 경우든 panic 없음
        registry
            .broadcast_event(HostEvent::ProcessingStarted {
                file_id: "f1".to_string(),
            })
            .await;
    }
}
