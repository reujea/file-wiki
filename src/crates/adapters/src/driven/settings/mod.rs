//! settings — SQLite 기반 설정 DB 어댑터 (settings-db-split-1 prep-3 baseline).
//!
//! `SettingsDb` 본체 (struct + 순수 DB 메서드 + 6 sub-trait impl) 이전 영역.
//! 부팅 시 toml 마이그레이션 (`open_or_migrate`) 은 shared 잔류 (cycle 회피 —
//! `PipelineConfigExt::load` / `load_doc_type_registry` 가 shared 소속).

pub mod sqlite;
