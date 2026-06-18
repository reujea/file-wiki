//! 도메인 모델 — `fp-domain-types` crate로 추출됨 (cycle 7 module-storage-db-1 step-d2).
//!
//! 본 모듈은 호환성 re-export shim 이다. 실제 정의는 `fp_domain_types::models` 에 있으며,
//! `module-storage-db` 가 core 를 의존하지 않고도 같은 타입을 참조할 수 있도록 분리했다.
//! 기존 `file_pipeline_core::domain::models::*` 경로는 본 re-export 로 그대로 유지된다.

pub use fp_domain_types::models::*;
