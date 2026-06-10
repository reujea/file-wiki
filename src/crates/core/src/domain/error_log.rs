//! 구조화된 에러 로그 — 파이프라인 전 단계의 에러를 추적
//!
//! JSON 구조화 로그로 영속화. Dashboard /api/errors에서 조회 가능.

use std::path::Path;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorEntry {
    pub timestamp: String,
    pub stage: String,
    pub file: String,
    pub error: String,
    pub context: String,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct ErrorLog {
    pub entries: Vec<ErrorEntry>,
}

impl ErrorLog {
    pub fn new() -> Self { Self { entries: vec![] } }

    pub fn record(&mut self, stage: &str, file: &str, error: &str, context: &str) {
        self.entries.push(ErrorEntry {
            timestamp: chrono::Local::now().to_rfc3339(),
            stage: stage.to_string(),
            file: file.to_string(),
            error: error.to_string(),
            context: context.to_string(),
        });
        // 최대 1000건 유지
        if self.entries.len() > 1000 {
            self.entries.drain(0..self.entries.len() - 1000);
        }
    }

    pub fn recent(&self, n: usize) -> &[ErrorEntry] {
        let start = self.entries.len().saturating_sub(n);
        &self.entries[start..]
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn load(path: &Path) -> Self {
        if path.exists() {
            std::fs::read_to_string(path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            Self::new()
        }
    }

    pub fn count_by_stage(&self) -> Vec<(String, usize)> {
        let mut counts = std::collections::HashMap::new();
        for e in &self.entries {
            *counts.entry(e.stage.clone()).or_insert(0) += 1;
        }
        let mut result: Vec<_> = counts.into_iter().collect();
        result.sort_by(|a, b| b.1.cmp(&a.1));
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_and_recent() {
        let mut log = ErrorLog::new();
        log.record("classify", "a.txt", "LLM timeout", "Claude CLI");
        log.record("verify", "b.txt", "구조 완전성 0%", "2-Pass 실패");
        assert_eq!(log.entries.len(), 2);
        assert_eq!(log.recent(1).len(), 1);
        assert_eq!(log.recent(1)[0].stage, "verify");
    }

    #[test]
    fn test_max_entries() {
        let mut log = ErrorLog::new();
        for i in 0..1100 {
            log.record("test", &format!("{}.txt", i), "err", "ctx");
        }
        assert!(log.entries.len() <= 1000);
    }

    #[test]
    fn test_count_by_stage() {
        let mut log = ErrorLog::new();
        log.record("classify", "a.txt", "err", "");
        log.record("classify", "b.txt", "err", "");
        log.record("verify", "c.txt", "err", "");
        let counts = log.count_by_stage();
        assert_eq!(counts[0], ("classify".to_string(), 2));
    }

    #[test]
    fn test_save_load() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("errors.json");
        let mut log = ErrorLog::new();
        log.record("test", "f.txt", "err", "ctx");
        log.save(&path).unwrap();
        let loaded = ErrorLog::load(&path);
        assert_eq!(loaded.entries.len(), 1);
    }
}
