//! Phase 80-E: 동작 모듈 — 사용자가 직접 선택하는 의미 단위 설정 묶음
//!
//! 5축 룰 추론(Phase 76)을 폐기하고 모듈 합집합으로 대체.
//! 사용자가 "민감 강화 + 정밀 검색 + 자동 lint" 같이 직접 선택 → 변경 합집합 산출 → setup_apply.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::config::PipelineConfig;
use crate::setup_review::{ConfigChange, RiskLevel, Priority, Evidence, Confidence};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModuleChange {
    pub path: String,
    pub value: serde_json::Value,
    pub reason: String,
    #[serde(default = "default_risk")]
    pub risk: RiskLevel,
}

fn default_risk() -> RiskLevel { RiskLevel::Low }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Module {
    pub id: String,
    pub group: String,           // process | search | ops
    pub icon: String,
    pub label: String,
    pub hint: String,
    #[serde(default = "default_priority")]
    pub priority: Priority,
    #[serde(default)]
    pub exclusive_group: Option<String>,
    pub changes: Vec<ModuleChange>,
}

fn default_priority() -> Priority { Priority::P0 }

#[derive(Debug, Deserialize)]
struct ModuleFile {
    module: Vec<Module>,
}

pub const DEFAULT_MODULES_TOML: &str = include_str!("setup_modules.toml");

#[derive(Debug, Clone)]
pub struct ModuleRegistry {
    modules: Vec<Module>,
}

impl ModuleRegistry {
    pub fn from_toml(s: &str) -> Result<Self> {
        let parsed: ModuleFile = toml::from_str(s).context("setup_modules.toml 파싱 실패")?;
        Ok(Self { modules: parsed.module })
    }

    pub fn default_registry() -> Self {
        Self::from_toml(DEFAULT_MODULES_TOML)
            .expect("DEFAULT_MODULES_TOML 파싱 실패 — 빌드 타임에 검증되어야 함")
    }

    pub fn all(&self) -> &[Module] { &self.modules }

    pub fn get(&self, id: &str) -> Option<&Module> {
        self.modules.iter().find(|m| m.id == id)
    }

    /// 선택된 모듈 ID 목록 → ConfigChange 합집합
    /// 충돌 해소: 같은 path에 다른 값이면 보수적 선택 (큰 청크 / true / 더 강한 도구).
    pub fn build_changes(&self, selected_ids: &[String], current: &PipelineConfig) -> Result<Vec<ConfigChange>> {
        // 배타 그룹 검증
        let mut by_excl: std::collections::HashMap<String, Vec<&Module>> = std::collections::HashMap::new();
        for id in selected_ids {
            let m = self.get(id).ok_or_else(|| anyhow::anyhow!("알 수 없는 모듈: {}", id))?;
            if let Some(eg) = &m.exclusive_group {
                by_excl.entry(eg.clone()).or_default().push(m);
            }
        }
        for (eg, mods) in &by_excl {
            if mods.len() > 1 {
                let names: Vec<&str> = mods.iter().map(|m| m.label.as_str()).collect();
                anyhow::bail!("배타 그룹 '{}'에서 다중 선택 — 1개만 선택해야 함: {:?}", eg, names);
            }
        }

        // path별 변경 수집 (한 path에 여러 모듈이 다른 값을 추천하면 충돌)
        let mut by_path: std::collections::BTreeMap<String, Vec<(&Module, &ModuleChange)>> = std::collections::BTreeMap::new();
        for id in selected_ids {
            let m = self.get(id).expect("validated");
            for ch in &m.changes {
                by_path.entry(ch.path.clone()).or_default().push((m, ch));
            }
        }

        let mut out = Vec::new();
        for (path, candidates) in by_path {
            let cur = current_value(current, &path).unwrap_or(serde_json::Value::Null);
            // 값이 모두 동일하면 그대로
            let first_value = candidates[0].1.value.clone();
            let all_same = candidates.iter().all(|(_, c)| c.value == first_value);

            let (chosen, conflict_note) = if all_same {
                (candidates[0].1.value.clone(), None)
            } else {
                // 충돌 해소: 보수적 선택
                let resolved = resolve_conservative(&candidates);
                let labels: Vec<&str> = candidates.iter().map(|(m, _)| m.label.as_str()).collect();
                (resolved, Some(format!("모듈 {} 간 다른 값 — 보수적 선택", labels.join(" / "))))
            };

            // 현재 값과 같으면 변경 안 만듦
            if values_equal(&cur, &chosen) { continue; }

            // 가장 강한 risk + P0 priority 선택
            let (max_risk, max_priority, reason) = aggregate_meta(&candidates);

            out.push(ConfigChange {
                path: path.clone(),
                current: cur,
                recommended: chosen,
                reason,
                priority: max_priority,
                risk: max_risk,
                evidence: Evidence::Heuristic,
                confidence: Confidence::Medium,
                reversible: true,
                needs_restart: false,
                conflict_note,
            });
        }
        Ok(out)
    }
}

fn aggregate_meta(candidates: &[(&Module, &ModuleChange)]) -> (RiskLevel, Priority, String) {
    use RiskLevel::*;
    let risk_order = |r: &RiskLevel| match r { Low => 0, Medium => 1, High => 2, Critical => 3 };
    let max_risk = candidates.iter().map(|(_, c)| c.risk).max_by_key(risk_order).unwrap_or(Low);
    let pri_order = |p: &Priority| match p { Priority::P0 => 0, Priority::P1 => 1, Priority::P2 => 2 };
    let max_priority = candidates.iter().map(|(m, _)| m.priority).min_by_key(pri_order).unwrap_or(Priority::P1);
    let reason = candidates.iter()
        .map(|(m, c)| format!("[{}] {}", m.label, c.reason))
        .collect::<Vec<_>>()
        .join(" + ");
    (max_risk, max_priority, reason)
}

fn resolve_conservative(candidates: &[(&Module, &ModuleChange)]) -> serde_json::Value {
    use serde_json::Value as V;

    let values: Vec<&V> = candidates.iter().map(|(_, c)| &c.value).collect();

    // 모두 boolean — true 우선 (보수적: 활성화 우선)
    if values.iter().all(|v| matches!(v, V::Bool(_))) {
        if values.iter().any(|v| v.as_bool() == Some(true)) {
            return V::Bool(true);
        }
        return V::Bool(false);
    }
    // 모두 숫자 — 큰 값 (대부분 보수적: 큰 청크/오래 보존/큰 cap)
    if values.iter().all(|v| matches!(v, V::Number(_))) {
        let max_f = values.iter()
            .filter_map(|v| v.as_f64())
            .fold(f64::MIN, f64::max);
        return serde_json::json!(max_f);
    }
    // 모두 array — 합집합
    if values.iter().all(|v| matches!(v, V::Array(_))) {
        let mut seen = std::collections::HashSet::<String>::new();
        let mut out = Vec::new();
        for v in values {
            if let Some(arr) = v.as_array() {
                for item in arr {
                    let key = item.to_string();
                    if seen.insert(key) { out.push(item.clone()); }
                }
            }
        }
        return V::Array(out);
    }
    // 모두 string — 더 "강한" 도구 우선 (preprocessing 등): 사전 정의 우선순위
    if values.iter().all(|v| matches!(v, V::String(_))) {
        let strength_order = |s: &str| -> i32 {
            match s {
                "marker" => 3,
                "pymupdf4llm" => 2,
                "pandoc" => 2,
                "tesseract" => 2,
                "claude_vision" => 3,
                "auto" => 1,
                "none" => 0,
                _ => 1,
            }
        };
        let best = values.iter().max_by_key(|v| strength_order(v.as_str().unwrap_or(""))).unwrap();
        return (*best).clone();
    }
    // object 또는 혼합 타입 — 첫 값 (이런 충돌은 거의 안 일어남, threshold object 등은 단일 모듈만 추천)
    values[0].clone()
}

fn values_equal(a: &serde_json::Value, b: &serde_json::Value) -> bool {
    use serde_json::Value::*;
    match (a, b) {
        (Number(x), Number(y)) => match (x.as_f64(), y.as_f64()) {
            (Some(xa), Some(yb)) => (xa - yb).abs() < 1e-6,
            _ => x == y,
        },
        _ => a == b,
    }
}

fn current_value(c: &PipelineConfig, path: &str) -> Option<serde_json::Value> {
    let parts: Vec<&str> = path.split('.').collect();
    let v = serde_json::to_value(c).ok()?;
    let mut cur = &v;
    for p in &parts {
        cur = cur.get(*p)?;
    }
    Some(cur.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_modules_parse() {
        let r = ModuleRegistry::default_registry();
        assert!(!r.all().is_empty());
        // 12개 정의 (가공 5 + 검색 4 + 운영 3)
        assert_eq!(r.all().len(), 12);
    }

    #[test]
    fn test_get_module() {
        let r = ModuleRegistry::default_registry();
        let m = r.get("secure_strict").expect("module exists");
        assert_eq!(m.group, "process");
        assert!(!m.changes.is_empty());
    }

    #[test]
    fn test_build_changes_single_module() {
        let r = ModuleRegistry::default_registry();
        let mut cfg = PipelineConfig::default_config();
        // 현재 값이 모듈 추천 값과 다르도록 설정
        cfg.schedule.lint_interval_hours = 0;
        let changes = r.build_changes(&["auto_lint".into()], &cfg).expect("build");
        assert!(changes.iter().any(|c| c.path == "schedule.lint_interval_hours"));
    }

    #[test]
    fn test_exclusive_group_violation() {
        let r = ModuleRegistry::default_registry();
        let cfg = PipelineConfig::default_config();
        let result = r.build_changes(&["chunk_large".into(), "chunk_small".into()], &cfg);
        assert!(result.is_err(), "배타 그룹 위반 — 에러");
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("배타"));
    }

    #[test]
    fn test_unknown_module_error() {
        let r = ModuleRegistry::default_registry();
        let cfg = PipelineConfig::default_config();
        let result = r.build_changes(&["unknown_module".into()], &cfg);
        assert!(result.is_err());
    }

    #[test]
    fn test_conservative_array_merge() {
        let r = ModuleRegistry::default_registry();
        let cfg = PipelineConfig::default_config();
        // secure_strict는 sensitive.extensions를 길게 늘리지만, 사용자 cfg가 빈 배열이면 변경 발생
        let changes = r.build_changes(&["secure_strict".into()], &cfg).expect("build");
        let ext = changes.iter().find(|c| c.path == "sensitive.extensions");
        assert!(ext.is_some());
    }

    #[test]
    fn test_combined_modules_no_conflict() {
        let r = ModuleRegistry::default_registry();
        let mut cfg = PipelineConfig::default_config();
        // 두 path 모두 default와 추천이 다르도록
        cfg.schedule.lint_interval_hours = 0;
        cfg.compression.original_ttl_days = 30;
        let changes = r.build_changes(&["auto_lint".into(), "long_retention".into()], &cfg).expect("build");
        assert!(changes.iter().any(|c| c.path == "schedule.lint_interval_hours"));
        assert!(changes.iter().any(|c| c.path == "compression.original_ttl_days"));
    }
}
