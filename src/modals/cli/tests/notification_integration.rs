//! 알림 통합 테스트 — 실제 Telegram/Slack 메시지 전송
//!
//! 환경변수 미설정 시 자동 스킵.
//! TELEGRAM_BOT_TOKEN + TELEGRAM_CHAT_ID 설정 시 Telegram 테스트 실행.
//! SLACK_BOT_TOKEN + SLACK_CHANNEL 설정 시 Slack 테스트 실행.

use file_pipeline_adapters::driven::notification::slack_notify::SlackNotificationAdapter;
use file_pipeline_adapters::driven::notification::telegram_notify::TelegramNotificationAdapter;
use file_pipeline_core::domain::models::{DbStats, ProcessingIssue, ProcessingSummary};
use file_pipeline_core::ports::output::NotificationPort;

fn telegram_configured() -> bool {
    std::env::var("TELEGRAM_BOT_TOKEN").is_ok() && std::env::var("TELEGRAM_CHAT_ID").is_ok()
}

fn slack_configured() -> bool {
    std::env::var("SLACK_BOT_TOKEN").is_ok() && std::env::var("SLACK_CHANNEL").is_ok()
}

fn telegram_adapter() -> TelegramNotificationAdapter {
    TelegramNotificationAdapter::new(
        std::env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN"),
        std::env::var("TELEGRAM_CHAT_ID").expect("TELEGRAM_CHAT_ID"),
    )
}

fn telegram_adapter_group() -> TelegramNotificationAdapter {
    TelegramNotificationAdapter::new(
        std::env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN"),
        "-1003990184767".to_string(),
    )
}

fn telegram_adapter_channel() -> TelegramNotificationAdapter {
    TelegramNotificationAdapter::new(
        std::env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN"),
        "-1003976785396".to_string(),
    )
}

fn slack_adapter() -> SlackNotificationAdapter {
    SlackNotificationAdapter::new(
        std::env::var("SLACK_BOT_TOKEN").expect("SLACK_BOT_TOKEN"),
        std::env::var("SLACK_CHANNEL").expect("SLACK_CHANNEL"),
    )
}

fn test_summary() -> ProcessingSummary {
    let mut s = ProcessingSummary {
        success: 5,
        errors: 1,
        skipped: 2,
        duplicates: 1,
        ..Default::default()
    };
    s.by_type.insert("meeting".into(), 3);
    s.by_type.insert("study".into(), 2);
    s.issues.push(ProcessingIssue {
        filename: "test.txt".into(),
        reason: "검증 실패: ROUGE-L 5%".into(),
        level: "error".into(),
        action_taken: "quarantine 이동".into(),
    });
    s
}

fn test_stats() -> DbStats {
    DbStats {
        total_documents: 42,
        by_type: vec![("meeting".into(), 15), ("study".into(), 10)],
        total_size_bytes: 1024 * 1024 * 5,
        sensitive_count: 2,
    }
}

// ── Telegram ────────────────────────────────────────

#[tokio::test]
async fn test_telegram_send_info() {
    if !telegram_configured() {
        return;
    }
    let adapter = telegram_adapter();
    adapter
        .send("통합 테스트", "info 레벨 알림 테스트 메시지", "info")
        .await
        .expect("Telegram info 전송 실패");
}

#[tokio::test]
async fn test_telegram_send_error() {
    if !telegram_configured() {
        return;
    }
    let adapter = telegram_adapter();
    adapter
        .send("에러 테스트", "error 레벨 알림 테스트 메시지", "error")
        .await
        .expect("Telegram error 전송 실패");
}

#[tokio::test]
async fn test_telegram_send_summary() {
    if !telegram_configured() {
        return;
    }
    let adapter = telegram_adapter();
    adapter
        .send_summary(&test_summary())
        .await
        .expect("Telegram summary 전송 실패");
}

#[tokio::test]
async fn test_telegram_send_duplicate_alert() {
    if !telegram_configured() {
        return;
    }
    let adapter = telegram_adapter();
    adapter
        .send_duplicate_alert(
            "회의록_2026-04-14.txt",
            "의미 중복 (유사도 0.95)",
            "- 결정사항: A → B\n+ 결정사항: A → C",
        )
        .await
        .expect("Telegram duplicate alert 전송 실패");
}

#[tokio::test]
async fn test_telegram_send_completion() {
    if !telegram_configured() {
        return;
    }
    let adapter = telegram_adapter();
    adapter
        .send_completion("보고서.pdf", "report", &test_stats())
        .await
        .expect("Telegram completion 전송 실패");
}

#[tokio::test]
async fn test_telegram_group_chat() {
    if std::env::var("TELEGRAM_BOT_TOKEN").is_err() {
        return;
    }
    let adapter = telegram_adapter_group();
    adapter
        .send("그룹 테스트", "group_bot 채팅 테스트", "info")
        .await
        .expect("Telegram group 전송 실패");
}

#[tokio::test]
async fn test_telegram_channel() {
    if std::env::var("TELEGRAM_BOT_TOKEN").is_err() {
        return;
    }
    let adapter = telegram_adapter_channel();
    adapter
        .send("채널 테스트", "channel_bot 테스트", "info")
        .await
        .expect("Telegram channel 전송 실패");
}

// ── Slack ────────────────────────────────────────

#[tokio::test]
async fn test_slack_send_info() {
    if !slack_configured() {
        return;
    }
    let adapter = slack_adapter();
    adapter
        .send("통합 테스트", "info 레벨 알림 테스트", "info")
        .await
        .expect("Slack info 전송 실패");
}

#[tokio::test]
async fn test_slack_send_summary() {
    if !slack_configured() {
        return;
    }
    let adapter = slack_adapter();
    adapter
        .send_summary(&test_summary())
        .await
        .expect("Slack summary 전송 실패");
}
