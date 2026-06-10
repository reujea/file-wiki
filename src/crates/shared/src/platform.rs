use std::path::PathBuf;

/// 기본 데이터 디렉토리 — 바이너리가 있는 디렉토리
pub fn default_base_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .unwrap_or_else(|| PathBuf::from("."))
}
