use std::path::Path;

use anyhow::Result;
use async_trait::async_trait;
use file_pipeline_core::domain::config_models::SensitiveResolutionConfig;
use file_pipeline_core::domain::models::{Metadata, SensitiveAction};
use file_pipeline_core::ports::input::SensitiveNotificationPort;

/// config 기반 자동 민감 파일 처리 (cli-prompt-remove-1 sub-decision A).
///
/// 기존 `TerminalSensitiveNotification`(stdin 대화형) 폐기 대체. 사용자 입력 없이
/// `SensitiveResolutionConfig`의 `default_action` 정책으로 자동 결정.
///
/// - `Skip` → None (색인·이동 부재)
/// - `MoveOnly` → 최소 Metadata (격리만, 가공/요약 부재 = 안전 default)
/// - `IndexWithStub` → 스텁 요약(템플릿 `{reason}` 치환) + 키워드 Metadata
pub struct AutoSensitiveNotification {
    config: SensitiveResolutionConfig,
}

impl AutoSensitiveNotification {
    pub fn new(config: SensitiveResolutionConfig) -> Self {
        Self { config }
    }
}

#[async_trait]
impl SensitiveNotificationPort for AutoSensitiveNotification {
    async fn notify_and_collect(
        &self,
        _file_path: &Path,
        reason: &str,
    ) -> Result<Option<Metadata>> {
        let action = SensitiveAction::from_config_str(&self.config.default_action);
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();

        let metadata = match action {
            SensitiveAction::Skip => return Ok(None),
            SensitiveAction::MoveOnly => Metadata {
                doc_types: vec!["sensitive".into()],
                rationale: format!("자동 격리(move_only): {}", reason),
                date: today,
                summary: String::new(),
                keywords: vec![],
                sensitive: true,
                doi: None,
                related_docs: vec![],
                source_doc_ids: vec![],
                search_hints: vec![],
                entities: vec![],
                ..Default::default()
            },
            SensitiveAction::IndexWithStub => Metadata {
                doc_types: vec!["sensitive".into()],
                rationale: format!("자동 스텁 색인(index_with_stub): {}", reason),
                date: today,
                summary: self.config.stub_summary_template.replace("{reason}", reason),
                keywords: self.config.stub_keywords.clone(),
                sensitive: true,
                doi: None,
                related_docs: vec![],
                source_doc_ids: vec![],
                search_hints: vec![],
                entities: vec![],
                ..Default::default()
            },
        };
        Ok(Some(metadata))
    }
}
