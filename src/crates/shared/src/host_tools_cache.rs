//! Phase 81: 호스트 전처리 도구 감지 결과 settings.db 캐시
//!
//! 매번 외부 프로세스를 spawn하던 `HostToolDetector::detect()`를 settings.db에 캐싱.
//! 음성 캐시 포함 — 미설치 도구도 not_found=true로 기록해 PATH 탐색 재시도 비용 제거.
//!
//! 흐름:
//! - 서비스 시작 시 `ensure_cached()` 호출 → DB가 비었으면 1회 감지 + 저장
//! - 이후 모든 호출은 `load_from_db()` 또는 `current()` → DB 조회만 (5~10ms)
//! - 사용자가 도구 설치/제거 후 `refresh()` 호출 → 강제 재감지 + DB 교체

use anyhow::Result;
use file_pipeline_adapters::driven::preprocessing::preprocessor::{HostTool, HostToolDetector};

use crate::settings_db::{SettingsDb, HostToolCacheRow};

/// settings.db에서 캐시를 불러와 (HostTool, version) 형태로 반환 (설치된 도구만).
pub fn load_from_db(db: &SettingsDb) -> Result<Vec<(HostTool, String)>> {
    let rows = db.get_host_tools_cache()?;
    Ok(rows.into_iter()
        .filter(|r| !r.not_found)
        .filter_map(|r| HostTool::from_key(&r.tool).map(|t| (t, r.version)))
        .collect())
}

/// 캐시 비었으면 1회 감지 + 저장. 채워져 있으면 그대로 반환.
pub fn ensure_cached(db: &SettingsDb) -> Result<Vec<(HostTool, String)>> {
    if db.host_tools_cache_count()? == 0 {
        refresh(db)?;
    }
    load_from_db(db)
}

/// 강제 재감지 + DB 교체. 사용자가 "도구 새로고침" 클릭 시 호출.
pub fn refresh(db: &SettingsDb) -> Result<Vec<(HostTool, String)>> {
    let now = chrono::Local::now().to_rfc3339();
    let full = HostToolDetector::detect_full();
    let rows: Vec<HostToolCacheRow> = full.iter().map(|(t, ver)| HostToolCacheRow {
        tool: t.as_key().to_string(),
        version: ver.clone().unwrap_or_default(),
        detected_at: now.clone(),
        not_found: ver.is_none(),
        install_hint: Some(t.install_hint().to_string()),
    }).collect();
    db.replace_host_tools_cache(&rows)?;
    tracing::info!(
        "호스트 도구 감지 + 캐시: 설치 {} / 미설치 {} (settings.db)",
        rows.iter().filter(|r| !r.not_found).count(),
        rows.iter().filter(|r| r.not_found).count(),
    );
    load_from_db(db)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_empty_cache() {
        let db = SettingsDb::open_in_memory().expect("in-mem db");
        let rows = load_from_db(&db).expect("load");
        assert!(rows.is_empty(), "빈 캐시는 empty 반환");
    }

    #[test]
    fn test_ensure_cached_populates_db() {
        let db = SettingsDb::open_in_memory().expect("in-mem db");
        let _ = ensure_cached(&db).expect("ensure");
        let count = db.host_tools_cache_count().expect("count");
        // detect_full은 항상 HostTool::all() 4개 모두 기록 (설치/미설치 무관)
        assert_eq!(count, 4, "ensure 호출 후 4개 도구 모두 기록");
    }

    #[test]
    fn test_refresh_replaces_all() {
        let db = SettingsDb::open_in_memory().expect("in-mem db");
        // 가짜 데이터 삽입
        let fake = vec![HostToolCacheRow {
            tool: "pandoc".into(),
            version: "fake 0.0".into(),
            detected_at: "2026-01-01".into(),
            not_found: false,
            install_hint: None,
        }];
        db.replace_host_tools_cache(&fake).expect("seed");
        assert_eq!(db.host_tools_cache_count().expect("c"), 1);
        // refresh 후 4개 모두 채워짐
        let _ = refresh(&db).expect("refresh");
        assert_eq!(db.host_tools_cache_count().expect("c"), 4);
    }
}
