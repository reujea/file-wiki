use std::path::Path;

use anyhow::Result;
use async_trait::async_trait;

use crate::domain::models::{DuplicateAction, Metadata};

/// 중복 발견 시 사용자에게 해결 방안을 묻는 포트
#[async_trait]
pub trait DuplicateResolutionPort: Send + Sync {
    /// 중복 해결: diff를 보여주고 사용자 선택을 반환
    async fn resolve(
        &self,
        new_path: &Path,
        existing_path: &Path,
        diff_rendered: &str,
        reason: &str,
    ) -> Result<DuplicateAction>;

    /// 수동 병합: 에디터를 열어 사용자가 직접 병합
    async fn collect_manual_merge(&self, path_a: &Path, path_b: &Path) -> Result<String>;
}

/// 민감 파일 감지 시 사용자에게 알리고 메타데이터를 수집하는 포트
#[async_trait]
pub trait SensitiveNotificationPort: Send + Sync {
    /// 민감 파일 알림 + 메타데이터 수집 (None이면 건너뜀)
    async fn notify_and_collect(
        &self,
        file_path: &Path,
        reason: &str,
    ) -> Result<Option<Metadata>>;
}
