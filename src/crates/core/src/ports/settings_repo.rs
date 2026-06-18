//! `SettingsRepoPort` super-port + 6 sub-trait — `fp-domain-types` crate로 추출됨
//! (cycle 7 module-storage-db-1 step-d2).
//!
//! 호환성 re-export shim. 실제 정의(super-port + sub-trait + 책임 표)는
//! `fp_domain_types::ports::settings_repo`. `module-storage-db`(SqliteSettingsRepo)가
//! core 를 의존하지 않고도 본 trait 들을 impl 할 수 있도록 분리했다. 기존
//! `file_pipeline_core::ports::settings_repo::*` 경로는 본 re-export 로 유지된다.

pub use fp_domain_types::ports::settings_repo::*;
