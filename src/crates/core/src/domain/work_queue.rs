//! 작업 큐 매니저 — 파일 목록 캐싱 + 배치 분류 + 상태 추적
//!
//! 기능:
//! - inbox 스캔 → 파일 목록 캐싱 (해시 포함)
//! - 상태 추적: Pending → Processing → Done / Modified / Deleted / Failed
//! - 배치 분류: 소형(≤40KB) vs 대형(>40KB) 분리
//! - 스마트 재처리: 해시 변경 → Modified, 파일 삭제 → Deleted
//! - 영속화: JSON 파일로 저장/복원

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::info;

/// 파일 처리 상태
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkStatus {
    /// 대기 중 (아직 처리 안 됨)
    Pending,
    /// 처리 중
    Processing,
    /// 처리 완료
    Done,
    /// 원본이 변경됨 (해시 불일치 → 재처리 필요)
    Modified,
    /// 원본이 삭제됨
    Deleted,
    /// 처리 실패 (재시도 대기)
    Failed { reason: String, retries: u32 },
}

/// 개별 작업 항목
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkItem {
    pub path: PathBuf,
    pub hash: String,
    pub size_bytes: u64,
    pub status: WorkStatus,
    pub created_at: String,
    pub updated_at: String,
    /// 대용량 파일 여부 (>40KB)
    pub is_large: bool,
}

/// 배치 분류 결과
#[derive(Debug, Default)]
pub struct BatchPlan {
    /// 소형 파일 (≤40KB) — 빠른 처리
    pub small_files: Vec<PathBuf>,
    /// 대형 파일 (>40KB) — 에이전트 위임
    pub large_files: Vec<PathBuf>,
    /// 변경된 파일 — 재처리
    pub modified_files: Vec<PathBuf>,
    /// 삭제된 파일 — DB에서 제거
    pub deleted_ids: Vec<String>,
    /// 이미 처리 완료 — 스킵
    pub skipped: usize,
}

const LARGE_FILE_THRESHOLD: u64 = 40_000;

/// 작업 큐 매니저
#[derive(Debug, Serialize, Deserialize)]
pub struct WorkQueue {
    items: HashMap<String, WorkItem>,
}

impl Default for WorkQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkQueue {
    pub fn new() -> Self {
        Self { items: HashMap::new() }
    }

    /// 큐 상태 파일에서 복원
    pub fn load(path: &Path) -> Result<Self> {
        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            Ok(serde_json::from_str(&content)?)
        } else {
            Ok(Self::new())
        }
    }

    /// 큐 상태 저장
    pub fn save(&self, path: &Path) -> Result<()> {
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }

    /// inbox 디렉토리 스캔 → 캐시 갱신 + 배치 계획 생성
    pub fn scan_and_plan(&mut self, inbox_dir: &Path) -> Result<BatchPlan> {
        let mut plan = BatchPlan::default();
        let now = chrono::Local::now().to_rfc3339();

        // 현재 inbox 파일 목록
        let mut current_files: HashMap<String, (PathBuf, u64, String)> = HashMap::new();
        if let Ok(entries) = std::fs::read_dir(inbox_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() { continue; }

                let key = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
                let hash = compute_file_hash(&path).unwrap_or_default();

                current_files.insert(key, (path, size, hash));
            }
        }

        // 1. 기존 캐시와 비교 — 삭제/변경 감지
        let existing_keys: Vec<String> = self.items.keys().cloned().collect();
        for key in &existing_keys {
            if let Some(item) = self.items.get_mut(key) {
                if !current_files.contains_key(key) {
                    // 파일 삭제됨
                    if item.status == WorkStatus::Done {
                        item.status = WorkStatus::Deleted;
                        item.updated_at = now.clone();
                        plan.deleted_ids.push(item.hash.clone());
                    }
                } else if let Some((_, _, ref new_hash)) = current_files.get(key) {
                    if item.status == WorkStatus::Done && *new_hash != item.hash {
                        // 파일 변경됨
                        item.status = WorkStatus::Modified;
                        item.hash = new_hash.clone();
                        item.updated_at = now.clone();
                        plan.modified_files.push(item.path.clone());
                    }
                }
            }
        }

        // 2. 새 파일 + 미처리 파일 → 배치 분류
        for (key, (path, size, hash)) in &current_files {
            let item = self.items.entry(key.clone()).or_insert_with(|| {
                WorkItem {
                    path: path.clone(),
                    hash: hash.clone(),
                    size_bytes: *size,
                    status: WorkStatus::Pending,
                    created_at: now.clone(),
                    updated_at: now.clone(),
                    is_large: *size > LARGE_FILE_THRESHOLD,
                }
            });

            match &item.status {
                WorkStatus::Pending | WorkStatus::Failed { .. } => {
                    if *size > LARGE_FILE_THRESHOLD {
                        plan.large_files.push(path.clone());
                    } else {
                        plan.small_files.push(path.clone());
                    }
                }
                // Modified는 1단계에서 이미 plan에 추가됨 → 스킵
                WorkStatus::Modified => {}
                WorkStatus::Done => {
                    plan.skipped += 1;
                }
                _ => {}
            }
        }

        // 우선순위 정렬: 소형 파일 크기 오름차순 (작은 것 먼저)
        plan.small_files.sort_by_key(|p| std::fs::metadata(p).map(|m| m.len()).unwrap_or(0));
        // 대형 파일 크기 오름차순
        plan.large_files.sort_by_key(|p| std::fs::metadata(p).map(|m| m.len()).unwrap_or(0));

        info!(
            "배치 계획: 소형 {} + 대형 {} + 변경 {} + 삭제 {} + 스킵 {}",
            plan.small_files.len(), plan.large_files.len(),
            plan.modified_files.len(), plan.deleted_ids.len(), plan.skipped,
        );

        Ok(plan)
    }

    /// items에 항목이 없으면 Pending 상태로 등록 (file metadata 자동 채움).
    ///
    /// 실시간 watch 흐름은 scan_and_plan을 거치지 않으므로, mark_processing/mark_done이
    /// 기존 items에만 작동하는 제약을 해소하기 위해 mark_processing 직전 호출. 이미 있으면 no-op.
    pub fn ensure_item(&mut self, path: &Path) {
        let key = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
        if key.is_empty() || self.items.contains_key(&key) {
            return;
        }
        let size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        let hash = compute_file_hash(path).unwrap_or_default();
        let now = chrono::Local::now().to_rfc3339();
        self.items.insert(key, WorkItem {
            path: path.to_path_buf(),
            hash,
            size_bytes: size,
            status: WorkStatus::Pending,
            created_at: now.clone(),
            updated_at: now,
            is_large: size > LARGE_FILE_THRESHOLD,
        });
    }

    /// 처리 시작 표시
    pub fn mark_processing(&mut self, path: &Path) {
        let key = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
        if let Some(item) = self.items.get_mut(&key) {
            item.status = WorkStatus::Processing;
            item.updated_at = chrono::Local::now().to_rfc3339();
        }
    }

    /// 처리 완료 표시
    pub fn mark_done(&mut self, path: &Path) {
        let key = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
        if let Some(item) = self.items.get_mut(&key) {
            item.status = WorkStatus::Done;
            item.updated_at = chrono::Local::now().to_rfc3339();
        }
    }

    /// 처리 실패 표시
    pub fn mark_failed(&mut self, path: &Path, reason: &str) {
        let key = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
        if let Some(item) = self.items.get_mut(&key) {
            let retries = match &item.status {
                WorkStatus::Failed { retries, .. } => retries + 1,
                _ => 1,
            };
            item.status = WorkStatus::Failed { reason: reason.to_string(), retries };
            item.updated_at = chrono::Local::now().to_rfc3339();
        }
    }

    /// 실패 항목을 Pending으로 리셋하여 재처리 대기. 리셋 건수 반환.
    pub fn retry_all_failed(&mut self) -> usize {
        let mut count = 0;
        for item in self.items.values_mut() {
            if matches!(item.status, WorkStatus::Failed { .. }) {
                item.status = WorkStatus::Pending;
                item.updated_at = chrono::Local::now().to_rfc3339();
                count += 1;
            }
        }
        count
    }

    /// 삭제된 항목 정리 (완전 제거)
    pub fn purge_deleted(&mut self) {
        self.items.retain(|_, item| item.status != WorkStatus::Deleted);
    }

    /// 통계
    pub fn stats(&self) -> QueueStats {
        let mut s = QueueStats::default();
        for item in self.items.values() {
            match &item.status {
                WorkStatus::Pending => s.pending += 1,
                WorkStatus::Processing => s.processing += 1,
                WorkStatus::Done => s.done += 1,
                WorkStatus::Modified => s.modified += 1,
                WorkStatus::Deleted => s.deleted += 1,
                WorkStatus::Failed { .. } => s.failed += 1,
            }
            s.total_bytes += item.size_bytes;
        }
        s.total = self.items.len() as u64;
        s
    }

    pub fn items(&self) -> &HashMap<String, WorkItem> {
        &self.items
    }
}

#[derive(Debug, Default, Serialize)]
pub struct QueueStats {
    pub total: u64,
    pub pending: u64,
    pub processing: u64,
    pub done: u64,
    pub modified: u64,
    pub deleted: u64,
    pub failed: u64,
    pub total_bytes: u64,
}

fn compute_file_hash(path: &Path) -> Result<String> {
    let bytes = std::fs::read(path).context("파일 읽기 실패")?;
    let hash = Sha256::digest(&bytes);
    Ok(hex::encode(hash))
}

impl BatchPlan {
    pub fn total_work(&self) -> usize {
        self.small_files.len() + self.large_files.len() + self.modified_files.len()
    }

    pub fn is_empty(&self) -> bool {
        self.total_work() == 0 && self.deleted_ids.is_empty()
    }

    /// 예상 처리 시간 (초) — Claude CLI 기준 12.6s/파일
    pub fn estimated_time_secs(&self, secs_per_file: f64, workers: usize) -> f64 {
        let total = self.total_work() as f64;
        (total * secs_per_file) / workers as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_queue_empty() {
        let q = WorkQueue::new();
        let stats = q.stats();
        assert_eq!(stats.total, 0);
    }

    #[test]
    fn test_scan_new_files() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("a.txt"), "small file").unwrap();
        std::fs::write(dir.path().join("b.txt"), "another small file").unwrap();

        let mut q = WorkQueue::new();
        let plan = q.scan_and_plan(dir.path()).unwrap();

        assert_eq!(plan.small_files.len(), 2);
        assert_eq!(plan.large_files.len(), 0);
        assert_eq!(q.stats().pending, 2);
    }

    #[test]
    fn test_mark_done_and_rescan_skips() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("a.txt"), "content").unwrap();

        let mut q = WorkQueue::new();
        let plan1 = q.scan_and_plan(dir.path()).unwrap();
        assert_eq!(plan1.small_files.len(), 1);

        // 완료 처리
        q.mark_done(&dir.path().join("a.txt"));

        // 재스캔 → 스킵
        let plan2 = q.scan_and_plan(dir.path()).unwrap();
        assert_eq!(plan2.small_files.len(), 0);
        assert_eq!(plan2.skipped, 1);
    }

    #[test]
    fn test_detect_modified() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("a.txt"), "original").unwrap();

        let mut q = WorkQueue::new();
        q.scan_and_plan(dir.path()).unwrap();
        q.mark_done(&dir.path().join("a.txt"));

        // 파일 내용 변경
        std::fs::write(dir.path().join("a.txt"), "modified content").unwrap();

        let plan = q.scan_and_plan(dir.path()).unwrap();
        assert_eq!(plan.modified_files.len(), 1);
    }

    #[test]
    fn test_detect_deleted() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("a.txt"), "content").unwrap();

        let mut q = WorkQueue::new();
        q.scan_and_plan(dir.path()).unwrap();
        q.mark_done(&dir.path().join("a.txt"));

        // 파일 삭제
        std::fs::remove_file(dir.path().join("a.txt")).unwrap();

        let plan = q.scan_and_plan(dir.path()).unwrap();
        assert_eq!(plan.deleted_ids.len(), 1);
    }

    #[test]
    fn test_large_file_classification() {
        let dir = tempfile::TempDir::new().unwrap();
        // 50KB 파일 생성 (>40KB → large)
        std::fs::write(dir.path().join("big.txt"), "x".repeat(50_000)).unwrap();
        std::fs::write(dir.path().join("small.txt"), "tiny").unwrap();

        let mut q = WorkQueue::new();
        let plan = q.scan_and_plan(dir.path()).unwrap();

        assert_eq!(plan.large_files.len(), 1);
        assert_eq!(plan.small_files.len(), 1);
    }

    #[test]
    fn test_mark_failed_increments_retries() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("fail.txt"), "bad file").unwrap();

        let mut q = WorkQueue::new();
        q.scan_and_plan(dir.path()).unwrap();

        q.mark_failed(&dir.path().join("fail.txt"), "LLM 오류");
        q.mark_failed(&dir.path().join("fail.txt"), "LLM 오류 2");

        if let Some(item) = q.items().get("fail.txt") {
            match &item.status {
                WorkStatus::Failed { retries, .. } => assert_eq!(*retries, 2),
                _ => panic!("should be Failed"),
            }
        }
    }

    #[test]
    fn test_save_and_load() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("a.txt"), "content").unwrap();

        let mut q = WorkQueue::new();
        q.scan_and_plan(dir.path()).unwrap();
        q.mark_done(&dir.path().join("a.txt"));

        let queue_path = dir.path().join("work-queue.json");
        q.save(&queue_path).unwrap();

        let loaded = WorkQueue::load(&queue_path).unwrap();
        assert_eq!(loaded.stats().done, 1);
    }

    #[test]
    fn test_estimated_time() {
        let plan = BatchPlan {
            small_files: vec![PathBuf::from("a"), PathBuf::from("b")],
            large_files: vec![PathBuf::from("c")],
            modified_files: vec![],
            deleted_ids: vec![],
            skipped: 0,
        };
        // 3파일 × 12.6초 / 4 workers = ~9.45초
        let time = plan.estimated_time_secs(12.6, 4);
        assert!((time - 9.45).abs() < 0.01);
    }

    #[test]
    fn test_purge_deleted() {
        let dir = tempfile::TempDir::new().unwrap();
        std::fs::write(dir.path().join("a.txt"), "content").unwrap();

        let mut q = WorkQueue::new();
        q.scan_and_plan(dir.path()).unwrap();
        q.mark_done(&dir.path().join("a.txt"));

        std::fs::remove_file(dir.path().join("a.txt")).unwrap();
        q.scan_and_plan(dir.path()).unwrap();

        assert_eq!(q.stats().deleted, 1);
        q.purge_deleted();
        assert_eq!(q.stats().total, 0);
    }
}
