//! Tauri 앱 공유 상태
//!
//! FileProcessingService + 설정 + 메트릭을 Tauri State로 관리.

use std::path::PathBuf;
use std::sync::Arc;

use file_pipeline_core::domain::models::VerificationMetricEntry;
use file_pipeline_core::service::FileProcessingService;
use file_pipeline_shared::config::PipelineConfig;
use tokio::sync::RwLock;

/// Tauri 앱 공유 상태 (manage()로 등록)
pub struct AppState {
    pub service: Arc<FileProcessingService>,
    pub config: Arc<RwLock<PipelineConfig>>,
    pub settings_db_path: PathBuf,
    pub topics_dir: PathBuf,
    pub verification_metrics: Arc<RwLock<Vec<VerificationMetricEntry>>>,
    pub progress_tx: Option<tokio::sync::broadcast::Sender<String>>,
    /// inbox 감지 활성화 여부 (true=감시 중, false=일시 정지)
    pub watcher_active: Arc<std::sync::atomic::AtomicBool>,
}

/// 백그라운드 스레드에서 사용할 경량 참조
pub struct BackgroundRef {
    pub service: Arc<FileProcessingService>,
    pub config: Arc<RwLock<PipelineConfig>>,
}

impl AppState {
    /// 백그라운드 스레드용 참조 생성
    pub fn clone_for_background(&self) -> BackgroundRef {
        BackgroundRef {
            service: Arc::clone(&self.service),
            config: Arc::clone(&self.config),
        }
    }
}
