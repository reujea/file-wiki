use std::io::Write;
use std::path::Path;

use anyhow::Result;
use async_trait::async_trait;
use file_pipeline_core::domain::models::DuplicateAction;
use file_pipeline_core::ports::input::DuplicateResolutionPort;

/// 터미널 기반 중복 해결 — 사용자에게 diff를 보여주고 선택을 받음
pub struct TerminalDuplicateResolution;

#[async_trait]
impl DuplicateResolutionPort for TerminalDuplicateResolution {
    async fn resolve(
        &self,
        new_path: &Path,
        existing_path: &Path,
        diff_rendered: &str,
        reason: &str,
    ) -> Result<DuplicateAction> {
        eprintln!("\n🔄 중복 탐지: {}", reason);
        eprintln!("  신규: {:?}", new_path.file_name().unwrap_or_default());
        eprintln!("  기존: {:?}", existing_path.file_name().unwrap_or_default());
        eprintln!("{}", diff_rendered);
        eprint!("[1] 신규로 교체  [2] 기존 유지  [3] 수동 병합  [4] 둘 다 유지: ");
        std::io::stderr().flush()?;

        let choice = tokio::task::spawn_blocking(|| {
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).ok();
            input.trim().to_string()
        })
        .await
        .unwrap_or_default();

        match choice.as_str() {
            "1" => Ok(DuplicateAction::Replace),
            "2" => Ok(DuplicateAction::Skip),
            "3" => Ok(DuplicateAction::Merge),
            "4" => Ok(DuplicateAction::Keep),
            _ => {
                eprintln!("  → 기본: 기존 유지");
                Ok(DuplicateAction::Skip)
            }
        }
    }

    async fn collect_manual_merge(&self, path_a: &Path, path_b: &Path) -> Result<String> {
        let editor = std::env::var("EDITOR").unwrap_or_else(|_| {
            if cfg!(windows) { "notepad.exe".into() } else { "vim".into() }
        });

        // 두 파일 내용을 임시 파일에 합침
        let a = std::fs::read_to_string(path_a).unwrap_or_default();
        let b = std::fs::read_to_string(path_b).unwrap_or_default();
        let merged = format!(
            "<<<< 기존 ({})\n{}\n==== 신규 ({})\n{}\n>>>>",
            path_a.file_name().unwrap_or_default().to_string_lossy(),
            a,
            path_b.file_name().unwrap_or_default().to_string_lossy(),
            b
        );

        let temp = std::env::temp_dir().join("pipeline_merge.txt");
        std::fs::write(&temp, &merged)?;

        eprintln!("에디터로 병합 파일을 엽니다: {:?}", temp);
        let status = std::process::Command::new(&editor)
            .arg(&temp)
            .status();

        if let Ok(s) = status {
            if s.success() {
                return std::fs::read_to_string(&temp).map_err(Into::into);
            }
        }

        Ok(merged)
    }
}
