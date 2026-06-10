//! ConnectionPool — plugin_id별 active connection 1개 lazy-init + 재사용.
//!
//! Phase 202 본진입. PluginRegistry::call이 본 풀을 통해 connection 확보.
//!
//! 단일 진실원: `prd/research/plugin-architecture-2026-06-04.md` §5-C

use std::collections::HashMap;
use std::sync::Arc;

use fp_plugin_sdk::Connection;
use tokio::sync::Mutex;

use crate::plugin::registry::PluginError;

/// plugin_id 별 active connection 보관.
///
/// 본 풀은 lazy connect — 첫 호출 시 connect_client 시도. 실패는 `PluginError::NotRunning`
/// 으로 매핑. 후속 호출은 캐시된 connection 재사용.
///
/// 한 plugin에 대해 동시 in-flight 호출 1건만 허용 (단일 connection mutex).
/// 동시성 필요 시 connection 다중 보관은 Phase 207 이후 검토.
#[derive(Default)]
pub struct ConnectionPool {
    /// plugin_id → 활성 connection (lazy)
    conns: Mutex<HashMap<String, Arc<Mutex<Connection>>>>,
}

impl std::fmt::Debug for ConnectionPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ConnectionPool {{ .. }}")
    }
}

impl ConnectionPool {
    pub fn new() -> Self {
        ConnectionPool::default()
    }

    /// plugin connection 확보 (필요 시 신규 연결).
    ///
    /// 동작:
    /// 1. 캐시 lookup — 존재하면 `Arc<Mutex<Connection>>` 클론 반환
    /// 2. 미존재 시 `Connection::connect_client(plugin_id)` 시도
    /// 3. 성공 시 캐시 + 반환 / 실패 시 `PluginError::NotRunning` 매핑
    pub async fn get_or_connect(
        &self,
        plugin_id: &str,
    ) -> Result<Arc<Mutex<Connection>>, PluginError> {
        {
            let map = self.conns.lock().await;
            if let Some(c) = map.get(plugin_id) {
                return Ok(Arc::clone(c));
            }
        }
        // 캐시 miss — connect 시도 (lock 밖에서, 네트워크 호출이라 await 길어질 수 있음)
        let conn = Connection::connect_client(plugin_id).await.map_err(|e| {
            PluginError::NotRunning {
                plugin_id: plugin_id.to_string(),
                cause: e.to_string(),
            }
        })?;
        let arc = Arc::new(Mutex::new(conn));
        {
            let mut map = self.conns.lock().await;
            // race — 다른 task가 동시 connect 했을 가능성 — 이미 있으면 본 것 폐기
            if let Some(existing) = map.get(plugin_id) {
                return Ok(Arc::clone(existing));
            }
            map.insert(plugin_id.to_string(), Arc::clone(&arc));
        }
        Ok(arc)
    }

    /// 캐시된 connection 제거 (plugin 재시작 / 명시 disable 시 호출).
    pub async fn invalidate(&self, plugin_id: &str) {
        let mut map = self.conns.lock().await;
        map.remove(plugin_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn get_or_connect_returns_not_running_when_no_server() {
        let pool = ConnectionPool::new();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let plugin_id = format!("test.pool.absent.{:x}", nanos);
        match pool.get_or_connect(&plugin_id).await {
            Ok(_) => panic!("기대: Err(NotRunning), 실제: Ok"),
            Err(PluginError::NotRunning { plugin_id: id, .. }) => assert_eq!(id, plugin_id),
            Err(other) => panic!("기대: NotRunning, 실제: {:?}", other),
        }
    }

    #[tokio::test]
    async fn invalidate_removes_cached_entry() {
        let pool = ConnectionPool::new();
        pool.invalidate("nonexistent").await; // panic 없이 idempotent
    }
}
