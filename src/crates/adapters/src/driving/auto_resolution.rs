use std::path::Path;

use anyhow::Result;
use async_trait::async_trait;
use file_pipeline_core::domain::config_models::DuplicateResolutionConfig;
use file_pipeline_core::domain::models::DuplicateAction;
use file_pipeline_core::ports::input::DuplicateResolutionPort;

/// config 기반 자동 중복 해결 (cli-prompt-remove-1 sub-decision A).
///
/// 기존 `TerminalDuplicateResolution`(stdin 대화형) 폐기 대체. 사용자 입력 없이
/// `DuplicateResolutionConfig`의 `sha256_match`/`semantic_match` 정책으로 자동 결정.
/// lesson #25 정합 — 사용자 입력 제거 + 자동 결정.
///
/// 완전 중복(SHA-256)과 의미 유사를 구분하기 위해 `reason` 문자열을 검사한다
/// (deduplicator가 생성하는 reason에 "SHA"/"해시"가 포함되면 완전 중복으로 간주).
pub struct AutoDuplicateResolution {
    config: DuplicateResolutionConfig,
}

impl AutoDuplicateResolution {
    pub fn new(config: DuplicateResolutionConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl DuplicateResolutionPort for AutoDuplicateResolution {
    async fn resolve(
        &self,
        _new_path: &Path,
        _existing_path: &Path,
        _diff_rendered: &str,
        reason: &str,
    ) -> Result<DuplicateAction> {
        // reason 에 SHA/해시 단서가 있으면 완전 중복 정책, 아니면 의미 유사 정책.
        let lower = reason.to_ascii_lowercase();
        let is_exact = lower.contains("sha") || reason.contains("해시") || reason.contains("동일");
        let action_str = if is_exact {
            &self.config.sha256_match
        } else {
            &self.config.semantic_match
        };
        Ok(DuplicateAction::from_config_str(action_str))
    }

    async fn collect_manual_merge(&self, path_a: &Path, _path_b: &Path) -> Result<String> {
        // 자동 모드 = 수동 병합 부재. 기존 내용 그대로 반환 (Merge 동작이 호출해도 안전).
        std::fs::read_to_string(path_a).map_err(Into::into)
    }
}
