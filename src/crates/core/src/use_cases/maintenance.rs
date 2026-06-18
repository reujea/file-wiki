//! `MaintenanceUseCase` — `FileProcessingService` 의 운영성 유틸 함수 분리 (step-s4, 2026-06-16).
//!
//! 책임 영역:
//! - `flush_summary` — 누적 `ProcessingSummary` 를 알림 어댑터로 전달 후 비움
//! - `compile_state_batch_begin/end` — 증분 컴파일 상태 배치 모드 (save 1회)
//! - `reload_pii_patterns` — PII 사용자 패턴 핫 리로드 (RwLock 재주입)
//!
//! 의존 영역 = `NotificationPort` + 내부 가변 상태 (`Mutex<ProcessingSummary>` + `Mutex<CompileState>` + `RwLock<Vec<...>>`).
//! `FileProcessingService` 의 본 영역 메서드는 본 use case 로 위임 (파사드 패턴).

use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};
use std::sync::atomic::{AtomicBool, Ordering};

use anyhow::Result;

use crate::domain::incremental::CompileState;
use crate::domain::models::ProcessingSummary;
use crate::ports::output::NotificationPort;

/// 운영 유스케이스 — 알림 + 컴파일 상태 + PII 패턴 핫 리로드 단일 진입점.
///
/// 본 struct = `FileProcessingService` 의 필드 일부를 빌려 사용 (Arc clone). 본 use case 직접
/// 생성 부재 — `FileProcessingService::maintenance()` 같은 헬퍼로 컨텍스트별 instance 생성.
pub struct MaintenanceUseCase<'a> {
    pub notification: &'a Arc<dyn NotificationPort>,
    pub summary: &'a Mutex<ProcessingSummary>,
    pub compile_state: &'a Mutex<CompileState>,
    pub compile_state_path: &'a PathBuf,
    pub compile_state_batch: &'a AtomicBool,
    pub pii_user_patterns: &'a RwLock<Vec<(String, String)>>,
}

impl<'a> MaintenanceUseCase<'a> {
    /// 배치 완료 시 알림 — 비어 있으면 송신 부재.
    pub async fn flush_summary(&self) -> Result<()> {
        let summary: ProcessingSummary = {
            let mut s = self.summary.lock().expect("mutex poisoned");
            std::mem::take(&mut *s)
        };
        if !summary.is_empty() {
            self.notification.send_summary(&summary).await?;
        }
        Ok(())
    }

    /// 증분 컴파일 상태 배치 모드 시작 — `save()` 호출을 skip.
    pub fn compile_state_batch_begin(&self) {
        self.compile_state_batch.store(true, Ordering::Relaxed);
    }

    /// 증분 컴파일 상태 배치 모드 종료 — 1회 저장.
    pub fn compile_state_batch_end(&self) {
        self.compile_state_batch.store(false, Ordering::Relaxed);
        let state = self.compile_state.lock().expect("mutex poisoned");
        let _ = state.save(self.compile_state_path);
    }

    /// PII 패턴 핫 리로드 — settings.db 변경 후 호출 시 다음 가공부터 새 패턴 적용.
    pub fn reload_pii_patterns(&self, patterns: Vec<(String, String)>) -> Result<usize> {
        let mut guard = self.pii_user_patterns.write()
            .map_err(|e| anyhow::anyhow!("pii_user_patterns lock poisoned: {}", e))?;
        *guard = patterns;
        Ok(guard.len())
    }
}
