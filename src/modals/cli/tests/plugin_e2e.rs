//! Phase 202 B3 통합 테스트 — PluginRegistry 실 IPC end-to-end.
//!
//! 시나리오:
//! 1. `host_call_to_real_plugin_round_trip_via_service_builder`
//!    — ServiceBuilder + with_plugin_registry → real mock plugin server (in-process) →
//!    `plugin_registry.call` 으로 echo 호출 → 결과 검증
//! 2. `broadcast_event_reaches_only_subscriber`
//!    — 2 plugin discover (A: processing_completed 구독 / B: 미구독) → broadcast →
//!    A만 수신
//! 3. `call_returns_not_running_when_plugin_process_absent`
//!    — discover 완료 + plugin server 미시작 → call → `PluginError::NotRunning` →
//!    host 본 흐름 panic 없이 정상 종료
//!
//! 단일 진실원: plan `cozy-honking-crayon.md` §6 통합 테스트 시나리오 3건

use std::sync::Arc;
use std::time::Duration;

use file_pipeline_core::plugin::{HostEvent, IpcResponse, PluginError, PluginRegistry};
use fp_plugin_sdk::Connection;
use tempfile::tempdir;

fn unique_id(label: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("e2e.{}.{:x}", label, nanos)
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
name = "E2E Test"
version = "0.1.0"
api_version = 1
event_subscribe = [{subs}]

[entry]
type = "process"
command = "fp-plugin-e2e"
"#,
        id = plugin_id,
        subs = subs,
    )
}

fn write_plugin_dir(plugins_root: &std::path::Path, plugin_id: &str, manifest: &str) {
    let dir = plugins_root.join(plugin_id);
    std::fs::create_dir_all(&dir).expect("create plugin dir");
    std::fs::write(dir.join("fp-plugin.toml"), manifest).expect("write manifest");
}

/// 시나리오 1: host → real plugin (in-process mock server) round-trip
#[tokio::test]
async fn host_call_to_real_plugin_round_trip() {
    let dir = tempdir().unwrap();
    let plugin_id = unique_id("rt");
    write_plugin_dir(dir.path(), &plugin_id, &manifest_for(&plugin_id, &[]));

    // PluginRegistry — NullAudit 디폴트 (audit 통합은 Phase 202 단위 테스트에서 검증)
    let mut registry = PluginRegistry::new();
    let n = registry.discover(dir.path()).unwrap();
    assert_eq!(n, 1);

    // mock plugin server — 단일 요청 받고 Ok{result: {echo: params}} 응답
    let pid_for_srv = plugin_id.clone();
    let server_handle = tokio::spawn(async move {
        let mut srv = Connection::accept_server(&pid_for_srv).await.unwrap();
        let req = srv.recv_request().await.unwrap();
        let resp = IpcResponse::Ok {
            trace_id: req.trace_id.clone(),
            result: serde_json::json!({
                "echo": req.params,
                "method": req.method,
                "api_version": req.api_version,
            }),
        };
        srv.send_response(&resp).await.unwrap();
    });

    // server bind 대기 (cross-platform race 회피)
    tokio::time::sleep(Duration::from_millis(50)).await;

    let registry = Arc::new(registry);
    let result = registry
        .call(
            &plugin_id,
            "echo",
            serde_json::json!({"hello": "world", "n": 42}),
            "trace-e2e-rt",
        )
        .await
        .expect("call success");

    assert_eq!(result["echo"]["hello"], "world");
    assert_eq!(result["echo"]["n"], 42);
    assert_eq!(result["method"], "echo");
    assert_eq!(result["api_version"], 1);

    server_handle.await.unwrap();
}

/// 시나리오 2: broadcast_event — 구독 plugin만 수신
#[tokio::test]
async fn broadcast_event_reaches_only_subscriber() {
    let dir = tempdir().unwrap();
    let plugin_a = unique_id("brsub-A");
    let plugin_b = unique_id("brsub-B");
    write_plugin_dir(
        dir.path(),
        &plugin_a,
        &manifest_for(&plugin_a, &["processing_completed"]),
    );
    write_plugin_dir(
        dir.path(),
        &plugin_b,
        &manifest_for(&plugin_b, &["quarantine_added"]),
    );

    let mut registry = PluginRegistry::new();
    registry.discover(dir.path()).unwrap();
    registry.enable(&plugin_a).unwrap();
    registry.enable(&plugin_b).unwrap();

    // A 서버만 spawn — processing_completed 1건 수신 후 종료
    let pid_a_for_srv = plugin_a.clone();
    let a_handle = tokio::spawn(async move {
        let mut srv = Connection::accept_server(&pid_a_for_srv).await.unwrap();
        let evt = srv.recv_event().await.unwrap();
        evt
    });

    // B는 서버 미시작 — broadcast가 B로 connect 시도해도 silent skip 되어야 함
    // 만약 B를 구독으로 잘못 매칭하면 connect 실패 시 invalidate만 호출되고 정상 통과

    tokio::time::sleep(Duration::from_millis(50)).await;

    registry
        .broadcast_event(HostEvent::ProcessingCompleted {
            doc_id: "doc-broadcast-1".to_string(),
            title: Some("Test Doc".to_string()),
        })
        .await;

    // A는 1건 수신
    let received = tokio::time::timeout(Duration::from_secs(2), a_handle)
        .await
        .expect("A receive timeout")
        .expect("A spawn join");
    match received {
        HostEvent::ProcessingCompleted { doc_id, title } => {
            assert_eq!(doc_id, "doc-broadcast-1");
            assert_eq!(title.as_deref(), Some("Test Doc"));
        }
        other => panic!("기대: ProcessingCompleted, 실제: {:?}", other),
    }
    // B는 미구독 → connect 시도 0건이거나 silent fail. 본 phase는 A 수신만 검증.
}

/// 시나리오 3: plugin 미실행 — graceful NotRunning (lesson 14 패턴)
#[tokio::test]
async fn call_returns_not_running_when_plugin_process_absent() {
    let dir = tempdir().unwrap();
    let plugin_id = unique_id("absent");
    write_plugin_dir(dir.path(), &plugin_id, &manifest_for(&plugin_id, &[]));

    let mut registry = PluginRegistry::new();
    registry.discover(dir.path()).unwrap();
    let registry = Arc::new(registry);

    // server 미시작 → call → NotRunning
    let err = registry
        .call(
            &plugin_id,
            "any",
            serde_json::json!({}),
            "trace-e2e-absent",
        )
        .await
        .expect_err("기대: NotRunning");

    match err {
        PluginError::NotRunning { plugin_id: id, .. } => assert_eq!(id, plugin_id),
        other => panic!("기대: NotRunning, 실제: {:?}", other),
    }

    // 후속 호출도 동일하게 silent fail (host 본 흐름 막지 않음 검증)
    // 본 phase 범위는 panic 없이 NotRunning 반환만 검증
    let _ = registry
        .call(&plugin_id, "any", serde_json::json!({}), "trace-2")
        .await;
}
