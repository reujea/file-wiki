//! 설정 SQLite DB — `module-storage-db` crate로 본체 이관됨 (cycle 7 module-storage-db-1 step-d5).
//!
//! 호환성 thin re-export shim. 실제 정의(`SettingsDb` struct + 순수 DB 메서드 + 6 sub-trait impl +
//! 도메인 Row struct)는 `module_storage_db::settings_repo`. DB 본체는 fp-domain-types만 의존하는
//! 독립 crate로 분리됐고(file_pipeline_core 무의존, 순환 0), adapters는 본 re-export로
//! 기존 `file_pipeline_adapters::driven::settings::sqlite::*` 경로를 유지한다.
//!
//! 부팅 toml 마이그레이션(`open_or_migrate`)은 `PipelineConfigExt`(shared) 의존이라
//! `file_pipeline_shared::settings_db`에 자유함수로 잔류한다 (변경 없음).

pub use module_storage_db::settings_repo::*;
