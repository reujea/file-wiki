//! Permission gate — plugin 매니페스트의 권한 집합 검증.
//!
//! Phase 201 placeholder — 알려진 권한 목록 (`KnownPermission`) 매칭만 수행. Phase
//! 202(IPC bus) 시점에 실제 권한별 호출 차단 로직 추가.
//!
//! 단일 진실원: `prd/research/plugin-architecture-2026-06-04.md` §4 plugin 분류
//!
//! ## 권한 매칭
//!
//! - 알려진 권한(`KnownPermission`) → 정상 등록
//! - 알 수 없는 권한 → `PluginError::UnknownPermission`
//!
//! 알려진 권한 목록은 본 룰 자기 적용으로 plugin 추가 시 동시 갱신 의무
//! (메타 룰 1 sub-rule 1c — 다중 위치 동기화 누락).

use std::collections::HashSet;

use crate::plugin::registry::PluginError;

/// 알려진 권한 — Phase 201 placeholder.
///
/// `prd/research/plugin-architecture-2026-06-04.md` §4 plugin 분류 표에서 등재된
/// 권한 집합. 신규 plugin 추가 시 본 enum + 본 enum의 `ALL` 상수 동시 갱신 의무.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KnownPermission {
    VectorDbRead,
    AuditWrite,
    ConfigRead,
    ConfigWrite,
    SnapshotWrite,
    SignalRead,
    TodoRead,
    TodoWrite,
    LlmCall,
    CacheRead,
    CacheWrite,
    FsWrite,
}

impl KnownPermission {
    /// 알려진 권한 전체 목록 — discover 시 매니페스트 검증에 사용.
    pub const ALL: &'static [(&'static str, KnownPermission)] = &[
        ("vector_db.read", KnownPermission::VectorDbRead),
        ("audit.write", KnownPermission::AuditWrite),
        ("config.read", KnownPermission::ConfigRead),
        ("config.write", KnownPermission::ConfigWrite),
        ("snapshot.write", KnownPermission::SnapshotWrite),
        ("signal.read", KnownPermission::SignalRead),
        ("todo.read", KnownPermission::TodoRead),
        ("todo.write", KnownPermission::TodoWrite),
        ("llm.call", KnownPermission::LlmCall),
        ("cache.read", KnownPermission::CacheRead),
        ("cache.write", KnownPermission::CacheWrite),
        ("fs.write", KnownPermission::FsWrite),
    ];

    pub fn parse(name: &str) -> Option<KnownPermission> {
        KnownPermission::ALL
            .iter()
            .find(|(s, _)| *s == name)
            .map(|(_, p)| *p)
    }

    pub fn as_str(self) -> &'static str {
        KnownPermission::ALL
            .iter()
            .find(|(_, p)| *p == self)
            .map(|(s, _)| *s)
            .unwrap_or("(unknown)")
    }
}

/// 검증 후 grant 된 권한 집합. plugin 1건당 1개 보유.
#[derive(Debug, Clone, Default)]
pub struct PermissionGate {
    granted: HashSet<KnownPermission>,
}

impl PermissionGate {
    /// 매니페스트의 `permissions` 문자열 목록을 검증해서 PermissionGate 생성.
    /// 알 수 없는 권한 1건이라도 발견되면 `Err`.
    pub fn from_manifest_permissions(
        permissions: &[String],
        plugin_id: &str,
    ) -> Result<PermissionGate, PluginError> {
        let mut granted = HashSet::new();
        for raw in permissions {
            match KnownPermission::parse(raw) {
                Some(p) => {
                    granted.insert(p);
                }
                None => {
                    return Err(PluginError::UnknownPermission {
                        plugin_id: plugin_id.to_string(),
                        permission: raw.clone(),
                    })
                }
            }
        }
        Ok(PermissionGate { granted })
    }

    /// 본 plugin이 권한을 보유하는지 확인. Phase 202 IPC 호출 진입점에서 적용.
    pub fn has(&self, permission: KnownPermission) -> bool {
        self.granted.contains(&permission)
    }

    /// grant 된 권한 수 — Tauri command "plugin info" 에 노출 (Phase 208).
    pub fn count(&self) -> usize {
        self.granted.len()
    }

    /// 모든 권한 목록 (sorted by as_str).
    pub fn list(&self) -> Vec<&'static str> {
        let mut v: Vec<_> = self.granted.iter().map(|p| p.as_str()).collect();
        v.sort();
        v
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_permission_round_trip() {
        for (s, p) in KnownPermission::ALL {
            assert_eq!(KnownPermission::parse(s), Some(*p));
            assert_eq!(p.as_str(), *s);
        }
    }

    #[test]
    fn gate_grants_known_permissions() {
        let perms = vec!["vector_db.read".to_string(), "audit.write".to_string()];
        let gate = PermissionGate::from_manifest_permissions(&perms, "io.test").unwrap();
        assert!(gate.has(KnownPermission::VectorDbRead));
        assert!(gate.has(KnownPermission::AuditWrite));
        assert!(!gate.has(KnownPermission::FsWrite));
        assert_eq!(gate.count(), 2);
    }

    #[test]
    fn gate_rejects_unknown_permission() {
        let perms = vec!["vector_db.read".to_string(), "nonsense.thing".to_string()];
        let err = PermissionGate::from_manifest_permissions(&perms, "io.test").unwrap_err();
        match err {
            PluginError::UnknownPermission { permission, .. } => {
                assert_eq!(permission, "nonsense.thing");
            }
            other => panic!("기대: UnknownPermission, 실제: {:?}", other),
        }
    }

    #[test]
    fn gate_list_is_sorted() {
        let perms = vec![
            "vector_db.read".to_string(),
            "audit.write".to_string(),
            "config.read".to_string(),
        ];
        let gate = PermissionGate::from_manifest_permissions(&perms, "io.test").unwrap();
        let listed = gate.list();
        assert_eq!(listed, vec!["audit.write", "config.read", "vector_db.read"]);
    }
}
