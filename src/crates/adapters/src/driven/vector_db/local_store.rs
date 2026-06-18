//! 인프로세스 벡터 DB(LocalVectorStore) — `module-storage-db` crate로 본체 이관됨 (cycle 7 step-d5).
//!
//! 호환성 thin re-export shim. 실제 정의는 `module_storage_db::local_store`
//! (`LocalVectorStore` = VectorDBPort impl + `IncrementalFlushConfig`). fp-domain-types만 의존하는
//! 독립 crate로 분리(file_pipeline_core 무의존, 순환 0). adapters는 본 re-export로
//! 기존 `file_pipeline_adapters::driven::vector_db::local_store::*` 경로를 유지한다.

pub use module_storage_db::local_store::*;
