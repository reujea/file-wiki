pub mod audit_log;
pub mod auto_reindexer;
pub mod chunking;
pub mod chunking_quality;
pub mod classifier;
pub mod config_models;
pub mod cross_reference;
pub mod crossref_optimizer;
pub mod deduplicator;
pub mod diagnostics;
pub mod error_log;
pub mod hooks;
pub mod incremental;
pub mod lint;
pub mod mmr;
pub mod models;
pub mod purge;
pub mod search_engine;
pub mod settings_models;
#[cfg(test)]
mod tests;
// todo_lifecycle 제거됨 — 신규 todo 시스템으로 대체 (Phase 53)
pub mod topic_merger;
pub mod vec_io;
pub mod verification;
pub mod wiki_export;
pub mod wikilink;
pub mod work_queue;
