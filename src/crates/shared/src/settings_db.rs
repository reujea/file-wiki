//! 설정 SQLite DB — adapters 이전 후 re-export + 부팅 toml 마이그레이션 자유함수.
//!
//! ── settings-db-split-1 prep-3 (2026-06-17): `SettingsDb` 본체 adapters 이전 ──────────
//! `SettingsDb` struct + 순수 DB 메서드 + 6 sub-trait impl 은
//! `file_pipeline_adapters::driven::settings::sqlite` 로 이전되었다. 본 모듈은:
//!   1. `pub use` re-export — 기존 호출처 `crate::settings_db::SettingsDb` /
//!      `file_pipeline_shared::settings_db::SettingsDb` 가 변경 없이 작동.
//!   2. `open_or_migrate` 자유함수 — 부팅 시 toml 마이그레이션. `PipelineConfigExt::load`
//!      (shared extension trait) + `load_doc_type_registry`(shared 자유함수) 의존이라
//!      adapters 로 못 옮긴다 (cycle: adapters→shared 역참조). shared 잔류가 정방향
//!      (shared→adapters: adapters 의 `SettingsDb::open` + `migrate_*` 메서드 호출).
//!
//! 기존 `SettingsDb::open_or_migrate(dir)` associated-fn 호출처는 자유함수
//! `file_pipeline_shared::settings_db::open_or_migrate(dir)` 로 변경됨 (호출처 24곳).

use std::path::Path;

use anyhow::Result;

// ── adapters 이전 본체 re-export (호출처 변경 0건) ───────────────────────────
pub use file_pipeline_adapters::driven::settings::sqlite::{
    AuditEventRow, DecisionLogEntry, HostToolCacheRow, LlmCacheEntry, NewTodo,
    ProcessingMetricSummary, SettingsDb,
};

use crate::config::{load_doc_type_registry, PipelineConfig, PipelineConfigExt};

/// DB 열기 + TOML 자동 마이그레이션 (멱등)
///
/// 흐름:
/// 1. settings.db 열기 (없으면 생성)
/// 2. TOML 파일 존재 + DB에 해당 데이터 없음 → 마이그레이션
/// 3. 마이그레이션 성공 시 TOML → *.toml.bak 백업
///
/// ── prep-3 (2026-06-17): adapters 이전 후 자유함수로 잔류 ──────────
/// `PipelineConfigExt::load`(shared) + `load_doc_type_registry`(shared) 의존이라
/// adapters 측 `SettingsDb` impl 로 못 옮긴다 (cycle). adapters 의 순수 DB 메서드
/// (`open` / `has_data_in` / `migrate_from_*`)를 호출하는 정방향 자유함수.
pub fn open_or_migrate(data_dir: &Path) -> Result<SettingsDb> {
    let db_path = data_dir.join("settings.db");
    let db = SettingsDb::open(&db_path)?;

    // pipeline.toml 마이그레이션
    let pipeline_toml = data_dir.join("pipeline.toml");
    if pipeline_toml.exists() && !db.has_data_in("config")? {
        tracing::info!("pipeline.toml → settings.db 마이그레이션");
        let config = PipelineConfig::load(&pipeline_toml)?;
        db.migrate_from_config(&config)?;
        let bak = pipeline_toml.with_extension("toml.bak");
        std::fs::rename(&pipeline_toml, &bak)
            .unwrap_or_else(|e| tracing::warn!("TOML 백업 실패: {}", e));
    }

    // doc_types.toml 마이그레이션
    let doc_types_toml = data_dir.join("doc_types.toml");
    if doc_types_toml.exists() && !db.has_data_in("doc_types")? {
        tracing::info!("doc_types.toml → settings.db 마이그레이션");
        let registry = load_doc_type_registry(&doc_types_toml)?;
        db.migrate_from_doc_types(registry.all())?;
        let bak = doc_types_toml.with_extension("toml.bak");
        std::fs::rename(&doc_types_toml, &bak)
            .unwrap_or_else(|e| tracing::warn!("TOML 백업 실패: {}", e));
    }

    // prompts.toml 마이그레이션
    let prompts_toml = data_dir.join("prompts.toml");
    if prompts_toml.exists() && !db.has_data_in("prompts")? {
        tracing::info!("prompts.toml → settings.db 마이그레이션");
        let content = std::fs::read_to_string(&prompts_toml)?;
        db.migrate_from_prompts_toml(&content)?;
        let bak = prompts_toml.with_extension("toml.bak");
        std::fs::rename(&prompts_toml, &bak)
            .unwrap_or_else(|e| tracing::warn!("TOML 백업 실패: {}", e));
    }

    // DB에 아무 데이터도 없으면 기본값 생성
    if !db.has_data_in("config")? {
        tracing::info!("settings.db 기본 설정 생성");
        let default_config = PipelineConfig::default_config();
        db.migrate_from_config(&default_config)?;
    }

    Ok(db)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_open_or_migrate_from_toml() {
        let dir = tempfile::TempDir::new().expect("tmpdir");

        // pipeline.toml 생성
        let config = PipelineConfig::default_config();
        let toml_str = config.to_toml_string().expect("to_toml");
        std::fs::write(dir.path().join("pipeline.toml"), &toml_str).expect("write");

        // open_or_migrate → TOML에서 마이그레이션
        let db = open_or_migrate(dir.path()).expect("migrate");
        assert!(db.has_data_in("config").expect("check"));

        // TOML 파일이 .bak으로 이동됨
        assert!(!dir.path().join("pipeline.toml").exists());
        assert!(dir.path().join("pipeline.toml.bak").exists());

        // DB에서 설정 읽기
        let restored = db.to_pipeline_config().expect("restore");
        assert_eq!(restored.compression.zstd_level, config.compression.zstd_level);
    }

    #[test]
    fn test_open_or_migrate_idempotent() {
        let dir = tempfile::TempDir::new().expect("tmpdir");

        // 1차: 기본값으로 DB 생성
        {
            let db = open_or_migrate(dir.path()).expect("first");
            assert!(db.has_data_in("config").expect("check"));
        }

        // 2차: 이미 DB 존재 → 마이그레이션 스킵
        {
            let db = open_or_migrate(dir.path()).expect("second");
            assert!(db.has_data_in("config").expect("check"));
        }
    }

    #[test]
    fn test_open_or_migrate_no_toml_creates_defaults() {
        let dir = tempfile::TempDir::new().expect("tmpdir");
        // TOML 없음 → 기본값으로 DB 생성
        let db = open_or_migrate(dir.path()).expect("migrate");
        assert!(db.has_data_in("config").expect("check"));
        let config = db.to_pipeline_config().expect("restore");
        assert_eq!(config.compression.zstd_level, 3); // 기본값
    }
}
