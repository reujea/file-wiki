//! 출력 포트 trait — `fp-domain-types` crate로 추출됨 (cycle 7 module-storage-db-1 step-d2).
//!
//! 호환성 re-export shim. 실제 정의는 `fp_domain_types::ports::output`.
//! `module-storage-db`(LocalVectorStore 등)가 core 를 의존하지 않고도 VectorDBPort 등
//! 포트 trait 를 구현할 수 있도록 분리했다. 기존
//! `file_pipeline_core::ports::output::*` 경로는 본 re-export 로 유지된다.

pub use fp_domain_types::ports::output::*;
