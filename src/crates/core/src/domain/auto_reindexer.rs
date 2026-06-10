use anyhow::Result;
use tracing::{info, warn};

use super::models::{DocTypeRegistry, ReindexReport};
use crate::ports::output::{LLMPort, StoragePort, VectorDBPort};

/// doc_types.toml 변경 시 전체 문서를 재분류하는 모듈
pub struct AutoReindexer;

impl AutoReindexer {
    /// 전체 문서를 순차 재분류하고 유형 변경된 문서만 업데이트
    pub async fn reindex_all(
        vector_db: &dyn VectorDBPort,
        llm: &dyn LLMPort,
        storage: &dyn StoragePort,
        registry: &DocTypeRegistry,
    ) -> Result<ReindexReport> {
        let mut report = ReindexReport::default();

        let all_docs = vector_db.list_all()?;
        report.total_scanned = all_docs.len() as u64;
        info!("재색인 시작: {} 문서 스캔", all_docs.len());

        for doc_summary in &all_docs {
            // 가공본 임시 해제
            let temp_path = match storage.decompress_temp(&doc_summary.path) {
                Ok(p) => p,
                Err(e) => {
                    warn!("해제 실패 {:?}: {}", doc_summary.path, e);
                    report.errors.push(format!("{:?}: {}", doc_summary.path, e));
                    continue;
                }
            };

            // LLM 재분류
            let result = match llm.classify_and_process(&temp_path, registry).await {
                Ok(r) => r,
                Err(e) => {
                    warn!("재분류 실패 {:?}: {}", doc_summary.path, e);
                    report.errors.push(format!("{:?}: {}", doc_summary.path, e));
                    // 임시 파일 정리
                    let _ = std::fs::remove_file(&temp_path);
                    continue;
                }
            };

            // 임시 파일 정리
            let _ = std::fs::remove_file(&temp_path);

            // 유형이 변경되었는지 확인
            let mut old_sorted = doc_summary.doc_types.clone();
            old_sorted.sort();
            let mut new_sorted = result.doc_types.clone();
            new_sorted.sort();

            if old_sorted != new_sorted {
                info!(
                    "유형 변경: {:?} → {:?} ({})",
                    old_sorted, new_sorted, doc_summary.id
                );
                vector_db.update_types(&doc_summary.id, result.doc_types)?;
                report.types_changed += 1;
            }
        }

        info!(
            "재색인 완료: {} 스캔, {} 변경, {} 오류",
            report.total_scanned, report.types_changed, report.errors.len()
        );

        Ok(report)
    }
}
