use std::io::Write;
use std::path::Path;

use anyhow::Result;
use async_trait::async_trait;
use file_pipeline_core::domain::models::Metadata;
use file_pipeline_core::ports::input::SensitiveNotificationPort;

/// 터미널 기반 민감 파일 알림 — 사용자에게 경고하고 메타데이터를 수집
pub struct TerminalSensitiveNotification;

#[async_trait]
impl SensitiveNotificationPort for TerminalSensitiveNotification {
    async fn notify_and_collect(
        &self,
        file_path: &Path,
        reason: &str,
    ) -> Result<Option<Metadata>> {
        eprintln!("\n⚠️  민감 파일 감지");
        eprintln!("  파일: {:?}", file_path.file_name().unwrap_or_default());
        eprintln!("  이유: {}", reason);
        eprint!("[1] 메타데이터 입력 후 색인  [2] sensitive 이동만  [3] 건너뜀: ");
        std::io::stderr().flush()?;

        let choice = tokio::task::spawn_blocking(|| {
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).ok();
            input.trim().to_string()
        })
        .await
        .unwrap_or_default();

        match choice.as_str() {
            "1" => {
                // 메타데이터 직접 입력
                let summary = prompt_line("  요약: ").await;
                let keywords_raw = prompt_line("  키워드 (쉼표 구분): ").await;
                let keywords: Vec<String> = keywords_raw
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();

                Ok(Some(Metadata {
                    doc_types: vec!["sensitive".into()],
                    rationale: format!("수동 입력: {}", reason),
                    date: chrono::Local::now().format("%Y-%m-%d").to_string(),
                    summary,
                    keywords,
                    sensitive: true,
                    doi: None,
                    related_docs: vec![], source_doc_ids: vec![], search_hints: vec![],
                    entities: vec![],
                    ..Default::default()
                }))
            }
            "2" => {
                // 최소 메타데이터로 이동
                Ok(Some(Metadata {
                    doc_types: vec!["sensitive".into()],
                    rationale: format!("이동만: {}", reason),
                    date: chrono::Local::now().format("%Y-%m-%d").to_string(),
                    summary: String::new(),
                    keywords: vec![],
                    sensitive: true,
                    doi: None,
                    related_docs: vec![], source_doc_ids: vec![], search_hints: vec![],
                    entities: vec![],
                    ..Default::default()
                }))
            }
            _ => {
                eprintln!("  → 건너뜀");
                Ok(None)
            }
        }
    }
}

async fn prompt_line(prompt: &str) -> String {
    eprint!("{}", prompt);
    let _ = std::io::stderr().flush();
    tokio::task::spawn_blocking(|| {
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).ok();
        input.trim().to_string()
    })
    .await
    .unwrap_or_default()
}
