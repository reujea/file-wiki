//! LLM 결과 캐시 wrapper (Ruflo A1)
//!
//! `LLMPort` 구현을 감싸 settings.db `llm_cache` 테이블로 결과를 캐싱한다.
//! file_hash + content_hash 매칭 시 LLM 호출 스킵.
//! claude_cli 호출 (10~20초/파일, lesson 70) 회피가 목표.

use std::path::Path;
use std::sync::Arc;

use anyhow::Result;
use sha2::{Digest, Sha256};

use file_pipeline_core::domain::models::{
    ClassifyAndProcessResult, DocTypeRegistry, EnrichResult,
};
use file_pipeline_core::ports::output::LLMPort;

use crate::settings_db::{LlmCacheEntry, SettingsDb};

pub struct CachedLLM {
    inner: Arc<dyn LLMPort>,
    settings_db_path: std::path::PathBuf,
}

impl CachedLLM {
    pub fn new(inner: Arc<dyn LLMPort>, settings_db_path: std::path::PathBuf) -> Self {
        Self { inner, settings_db_path }
    }

    fn compute_file_hash(path: &Path) -> Result<String> {
        let bytes = std::fs::read(path)?;
        let mut hasher = Sha256::new();
        hasher.update(&bytes);
        Ok(format!("{:x}", hasher.finalize()))
    }

    fn compute_text_hash(text: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(text.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    fn lookup(&self, file_hash: &str, content_hash: &str) -> Option<ClassifyAndProcessResult> {
        let db = SettingsDb::open(&self.settings_db_path).ok()?;
        let entry = db.lookup_llm_cache(file_hash).ok().flatten()?;
        if entry.content_hash != content_hash {
            return None;
        }
        serde_json::from_str(&entry.result_json).ok()
    }

    fn store(&self, file_hash: &str, content_hash: &str, result: &ClassifyAndProcessResult) {
        let Ok(db) = SettingsDb::open(&self.settings_db_path) else { return; };
        let result_json = match serde_json::to_string(result) {
            Ok(s) => s,
            Err(_) => return,
        };
        let entry = LlmCacheEntry {
            file_hash: file_hash.to_string(),
            content_hash: content_hash.to_string(),
            result_json,
            doc_types: result.doc_types.join(","),
            hits: 0,
            created_at: chrono::Utc::now().to_rfc3339(),
            last_hit_at: None,
        };
        let _ = db.upsert_llm_cache(&entry);
    }
}

#[async_trait::async_trait]
impl LLMPort for CachedLLM {
    async fn classify_and_process(
        &self,
        file_path: &Path,
        registry: &DocTypeRegistry,
    ) -> Result<ClassifyAndProcessResult> {
        if let Ok(file_hash) = Self::compute_file_hash(file_path) {
            let content_hash = file_hash.clone();
            if let Some(cached) = self.lookup(&file_hash, &content_hash) {
                tracing::info!("[llm-cache] hit file={} hash={}",
                    file_path.display(), &file_hash[..16.min(file_hash.len())]);
                return Ok(cached);
            }
            let result = self.inner.classify_and_process(file_path, registry).await?;
            self.store(&file_hash, &content_hash, &result);
            Ok(result)
        } else {
            self.inner.classify_and_process(file_path, registry).await
        }
    }

    async fn classify_and_process_text(
        &self,
        file_name: &str,
        text: &str,
        registry: &DocTypeRegistry,
    ) -> Result<ClassifyAndProcessResult> {
        let content_hash = Self::compute_text_hash(text);
        let file_hash = Self::compute_text_hash(&format!("{}::{}", file_name, content_hash));
        if let Some(cached) = self.lookup(&file_hash, &content_hash) {
            tracing::info!("[llm-cache] hit text file_name={} hash={}",
                file_name, &file_hash[..16.min(file_hash.len())]);
            return Ok(cached);
        }
        let result = self.inner.classify_and_process_text(file_name, text, registry).await?;
        self.store(&file_hash, &content_hash, &result);
        Ok(result)
    }

    async fn summarize_text(&self, new_content: &str, existing: &str) -> Result<String> {
        self.inner.summarize_text(new_content, existing).await
    }

    async fn generate_hypothetical(&self, query: &str) -> Result<String> {
        // HyDE는 query에 의존, 캐시 가치 낮음 (사용자마다 다른 검색어). inner 위임
        self.inner.generate_hypothetical(query).await
    }

    async fn reprocess_with_feedback(
        &self,
        file_path: &Path,
        registry: &DocTypeRegistry,
        feedback: &str,
    ) -> Result<ClassifyAndProcessResult> {
        // 피드백 재가공은 캐시 우회 (이전 결과가 부정확해 재시도하는 경로)
        self.inner.reprocess_with_feedback(file_path, registry, feedback).await
    }

    async fn enrich_existing(
        &self,
        existing_content: &str,
        new_info: &str,
        doc_types: &[String],
    ) -> Result<EnrichResult> {
        self.inner.enrich_existing(existing_content, new_info, doc_types).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use file_pipeline_core::domain::models::Metadata;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingLLM {
        calls: AtomicUsize,
    }

    #[async_trait::async_trait]
    impl LLMPort for CountingLLM {
        async fn classify_and_process(
            &self,
            _file_path: &Path,
            _registry: &DocTypeRegistry,
        ) -> Result<ClassifyAndProcessResult> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            Ok(ClassifyAndProcessResult {
                doc_types: vec!["test".into()],
                rationale: "stub".into(),
                content: "content".into(),
                metadata: Metadata::default(),
                sections: None,
            })
        }

        async fn enrich_existing(
            &self,
            _e: &str, _n: &str, _d: &[String],
        ) -> Result<EnrichResult> {
            Ok(EnrichResult::default())
        }
    }

    impl file_pipeline_core::ports::outbound::OutboundManifest for CountingLLM {
        fn id(&self) -> &str { "fp-outbound-llm-counting-test" }
        fn category(&self) -> file_pipeline_core::ports::outbound::OutboundCategory {
            file_pipeline_core::ports::outbound::OutboundCategory::Llm
        }
        fn capabilities(&self) -> file_pipeline_core::ports::output::ResourceCapabilities {
            file_pipeline_core::ports::output::ResourceCapabilities::standard("counting-test")
        }
    }

    #[tokio::test]
    async fn test_cache_hit_skips_inner_call() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let db_path = tmp.path().join("settings.db");
        SettingsDb::open(&db_path).expect("open db");

        let test_file = tmp.path().join("sample.txt");
        std::fs::write(&test_file, b"hello world").expect("write");

        let inner = Arc::new(CountingLLM { calls: AtomicUsize::new(0) });
        let cached = CachedLLM::new(inner.clone(), db_path);
        let registry = DocTypeRegistry::empty();

        // 1차: miss → inner 호출
        let _ = cached.classify_and_process(&test_file, &registry).await.expect("1st");
        assert_eq!(inner.calls.load(Ordering::SeqCst), 1);

        // 2차: hit → inner 호출 없음
        let _ = cached.classify_and_process(&test_file, &registry).await.expect("2nd");
        assert_eq!(inner.calls.load(Ordering::SeqCst), 1, "캐시 hit 시 inner 미호출");
    }

    #[tokio::test]
    async fn test_cache_miss_when_content_changes() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let db_path = tmp.path().join("settings.db");
        SettingsDb::open(&db_path).expect("open db");

        let test_file = tmp.path().join("sample.txt");
        std::fs::write(&test_file, b"v1").expect("write v1");

        let inner = Arc::new(CountingLLM { calls: AtomicUsize::new(0) });
        let cached = CachedLLM::new(inner.clone(), db_path);
        let registry = DocTypeRegistry::empty();

        let _ = cached.classify_and_process(&test_file, &registry).await.expect("v1");
        assert_eq!(inner.calls.load(Ordering::SeqCst), 1);

        // 파일 내용 변경 → 다른 file_hash → miss
        std::fs::write(&test_file, b"v2 different").expect("write v2");
        let _ = cached.classify_and_process(&test_file, &registry).await.expect("v2");
        assert_eq!(inner.calls.load(Ordering::SeqCst), 2, "내용 변경 시 재호출");
    }
}

// step-o2 partial 해소 (2026-06-17, outbound-umbrella-1): cached_llm OutboundManifest 박힘
impl file_pipeline_core::ports::outbound::OutboundManifest for CachedLLM {
    fn id(&self) -> &str { "fp-outbound-llm-cached" }
    fn category(&self) -> file_pipeline_core::ports::outbound::OutboundCategory {
        file_pipeline_core::ports::outbound::OutboundCategory::Llm
    }
    fn capabilities(&self) -> file_pipeline_core::ports::output::ResourceCapabilities {
        file_pipeline_core::ports::output::ResourceCapabilities::standard("cached")
    }
}
