//! 교차참조 최적화 자료구조 — `fp-domain-types` crate로 추출됨 (cycle 7 step-d2/d3 게이트).
//!
//! 호환성 re-export shim. 실제 정의는 `fp_domain_types::crossref_optimizer`
//! (MinHashIndex / TaskQueue / TaskPriority / PrioritizedTask — 전부 순수 자료구조, std만 의존).
//! LocalVectorStore(module-storage-db 이관 대상)가 `MinHashIndex`를 참조하므로, core 의존 없이
//! 외부 crate에서 쓸 수 있도록 분리. 기존
//! `file_pipeline_core::domain::crossref_optimizer::*` 경로는 본 re-export 로 유지된다.

pub use fp_domain_types::crossref_optimizer::*;
