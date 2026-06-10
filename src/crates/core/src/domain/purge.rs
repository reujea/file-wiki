use std::path::{Path, PathBuf};
use std::time::SystemTime;
use anyhow::Result;

/// Purge 대상 파일 정보
#[derive(Debug, Clone, serde::Serialize)]
pub struct PurgeCandidate {
    pub path: PathBuf,
    pub filename: String,
    pub size_bytes: u64,
    pub age_days: u32,
}

/// Purge 실행 결과
#[derive(Debug, Clone, serde::Serialize)]
pub struct PurgeResult {
    pub deleted: usize,
    pub freed_bytes: u64,
    pub errors: Vec<(String, String)>,
}

/// 대상 디렉토리를 스캔하여 보존 기간 초과 파일 목록 반환
pub fn purge_dry_run(
    directory: &Path,
    retention_days: Option<u32>,
) -> Result<Vec<PurgeCandidate>> {
    let mut candidates = Vec::new();
    if !directory.exists() {
        return Ok(candidates);
    }

    let now = SystemTime::now();

    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let metadata = entry.metadata()?;
        if !metadata.is_file() { continue; }

        let age_days = if let Ok(modified) = metadata.modified() {
            if let Ok(age) = now.duration_since(modified) {
                (age.as_secs() / 86400) as u32
            } else { 0 }
        } else { 0 };

        // retention_days가 Some이면 초과 파일만, None이면 전체
        let include = match retention_days {
            Some(days) => age_days > days,
            None => true,
        };

        if include {
            let path = entry.path();
            candidates.push(PurgeCandidate {
                filename: path.file_name().unwrap_or_default().to_string_lossy().to_string(),
                path,
                size_bytes: metadata.len(),
                age_days,
            });
        }
    }

    // 오래된 순 정렬
    candidates.sort_by(|a, b| b.age_days.cmp(&a.age_days));
    Ok(candidates)
}

/// 실제 파일 삭제 실행
pub fn purge_execute(
    directory: &Path,
    retention_days: Option<u32>,
) -> Result<PurgeResult> {
    let candidates = purge_dry_run(directory, retention_days)?;
    let mut result = PurgeResult {
        deleted: 0,
        freed_bytes: 0,
        errors: Vec::new(),
    };

    for c in &candidates {
        match std::fs::remove_file(&c.path) {
            Ok(_) => {
                result.deleted += 1;
                result.freed_bytes += c.size_bytes;
                // .vec 동반 삭제
                let vec_path = c.path.with_extension("vec");
                if vec_path.exists() {
                    let _ = std::fs::remove_file(&vec_path);
                }
                tracing::info!("purge: 삭제 {:?} ({}일 경과, {}KB)", c.path, c.age_days, c.size_bytes / 1024);
            }
            Err(e) => {
                result.errors.push((c.filename.clone(), e.to_string()));
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_purge_dry_run_empty() {
        let dir = tempfile::tempdir().expect("tempdir");
        let result = purge_dry_run(dir.path(), Some(30)).expect("dry_run");
        assert!(result.is_empty());
    }

    #[test]
    fn test_purge_dry_run_all() {
        let dir = tempfile::tempdir().expect("tempdir");
        fs::write(dir.path().join("test.zst"), b"data").expect("write");
        // retention_days=None → 전체 대상
        let result = purge_dry_run(dir.path(), None).expect("dry_run");
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].filename, "test.zst");
    }

    #[test]
    fn test_purge_execute() {
        let dir = tempfile::tempdir().expect("tempdir");
        fs::write(dir.path().join("old.zst"), b"data").expect("write");
        let result = purge_execute(dir.path(), None).expect("execute");
        assert_eq!(result.deleted, 1);
        assert!(!dir.path().join("old.zst").exists());
    }
}
