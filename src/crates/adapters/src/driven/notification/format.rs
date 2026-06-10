//! 알림 메시지 포매팅 (도메인 결합 — file-pipeline 잔류).
//!
//! `ProcessingSummary` / `DbStats` 등 file-pipeline 도메인 타입을 텍스트로 변환.
//! Telegram(HTML) / Slack(mrkdwn) 두 백엔드 양식 분리.

use file_pipeline_core::domain::models::{DbStats, ProcessingSummary};

pub fn format_send_telegram(title: &str, body: &str, level: &str) -> String {
    let emoji = match level {
        "error" => "🔴",
        "warning" => "🟡",
        _ => "🟢",
    };
    format!("{} <b>{}</b>\n{}", emoji, title, body)
}

pub fn format_send_slack(title: &str, body: &str, level: &str) -> String {
    let emoji = match level {
        "error" => ":red_circle:",
        "warning" => ":warning:",
        _ => ":white_check_mark:",
    };
    format!("{} *{}*\n{}", emoji, title, body)
}

pub fn format_duplicate_telegram(filename: &str, reason: &str, diff_summary: &str) -> String {
    let truncated = if diff_summary.len() > 3000 { &diff_summary[..3000] } else { diff_summary };
    format!(
        "🔄 <b>중복 탐지</b>\n파일: {}\n이유: {}\n\n<pre>{}</pre>",
        filename, reason, truncated
    )
}

pub fn format_duplicate_slack(filename: &str, reason: &str, diff_summary: &str) -> String {
    format!(
        ":arrows_counterclockwise: *중복 탐지*\n파일: {}\n이유: {}\n```{}```",
        filename, reason, diff_summary
    )
}

pub fn format_sensitive(channel: &str, filename: &str, reason: &str) -> String {
    if channel == "telegram" {
        format!("⚠️ <b>민감 파일 감지</b>\n파일: {}\n이유: {}", filename, reason)
    } else {
        format!(":warning: *민감 파일 감지*\n파일: {}\n이유: {}", filename, reason)
    }
}

pub fn format_completion(channel: &str, filename: &str, doc_type: &str, stats: &DbStats) -> String {
    if channel == "telegram" {
        format!(
            "✅ <b>처리 완료</b>\n파일: {}\n유형: {}\n총 문서: {}",
            filename, doc_type, stats.total_documents
        )
    } else {
        format!(
            ":white_check_mark: *처리 완료*\n파일: {}\n유형: {}\n총 문서: {}",
            filename, doc_type, stats.total_documents
        )
    }
}

pub fn format_summary_telegram(s: &ProcessingSummary) -> String {
    let mut msg = format!(
        "📊 <b>파이프라인 처리 요약</b>\n\n\
         ✅ 성공: {}  ⏳ 처리중: {}  ❌ 에러: {}\n\
         ⏭ 스킵: {}  🔒 민감: {}  🔄 중복: {}  🚫 격리: {}",
        s.success, s.processing, s.errors, s.skipped, s.sensitive, s.duplicates, s.quarantined,
    );

    if !s.by_type.is_empty() {
        msg.push_str("\n\n<b>유형별:</b>");
        for (t, c) in &s.by_type {
            msg.push_str(&format!("\n  {} — {}", t, c));
        }
    }

    if !s.issues.is_empty() {
        msg.push_str("\n\n<b>이슈:</b>");
        for issue in s.issues.iter().take(10) {
            let icon = if issue.level == "error" { "❌" } else { "⚠️" };
            msg.push_str(&format!(
                "\n{} {} — {}\n   → {}",
                icon, issue.filename, issue.reason, issue.action_taken
            ));
        }
        if s.issues.len() > 10 {
            msg.push_str(&format!("\n  ... 외 {}건", s.issues.len() - 10));
        }
    }

    msg
}

pub fn format_summary_slack(s: &ProcessingSummary) -> String {
    let mut msg = format!(
        ":bar_chart: *파이프라인 처리 요약*\n\n\
         :white_check_mark: 성공: {}  :hourglass: 처리중: {}  :x: 에러: {}\n\
         :fast_forward: 스킵: {}  :lock: 민감: {}  :arrows_counterclockwise: 중복: {}  :no_entry: 격리: {}",
        s.success, s.processing, s.errors, s.skipped, s.sensitive, s.duplicates, s.quarantined,
    );

    if !s.by_type.is_empty() {
        msg.push_str("\n\n*유형별:*");
        for (t, c) in &s.by_type {
            msg.push_str(&format!("\n  {} — {}", t, c));
        }
    }

    if !s.issues.is_empty() {
        msg.push_str("\n\n*이슈:*");
        for issue in s.issues.iter().take(10) {
            let icon = if issue.level == "error" { ":x:" } else { ":warning:" };
            msg.push_str(&format!(
                "\n{} {} — {}\n   → {}",
                icon, issue.filename, issue.reason, issue.action_taken
            ));
        }
    }

    msg
}

#[cfg(test)]
mod tests {
    use super::*;
    use file_pipeline_core::domain::models::ProcessingIssue;

    #[test]
    fn test_format_send_telegram_levels() {
        assert!(format_send_telegram("제목", "본문", "info").starts_with("🟢"));
        assert!(format_send_telegram("제목", "본문", "warning").starts_with("🟡"));
        assert!(format_send_telegram("제목", "본문", "error").starts_with("🔴"));
        // HTML 태그 포함
        assert!(format_send_telegram("t", "b", "info").contains("<b>t</b>"));
    }

    #[test]
    fn test_format_send_slack_levels() {
        assert!(format_send_slack("제목", "본문", "info").starts_with(":white_check_mark:"));
        assert!(format_send_slack("제목", "본문", "warning").starts_with(":warning:"));
        assert!(format_send_slack("제목", "본문", "error").starts_with(":red_circle:"));
        // mrkdwn 형식 (별표)
        assert!(format_send_slack("t", "b", "info").contains("*t*"));
    }

    #[test]
    fn test_format_duplicate_telegram_truncates_long_diff() {
        let long_diff = "a".repeat(5000);
        let out = format_duplicate_telegram("file.txt", "동일", &long_diff);
        // 3000자 컷 + 헤더는 별도
        assert!(out.contains("file.txt"));
        assert!(out.contains("동일"));
        // diff 영역이 정확히 3000자 이하인지
        assert!(out.matches('a').count() <= 3000);
    }

    #[test]
    fn test_format_duplicate_slack_uses_code_block() {
        let out = format_duplicate_slack("file.txt", "동일", "diff");
        assert!(out.contains("```diff```"));
    }

    #[test]
    fn test_format_sensitive_channel_branch() {
        let tg = format_sensitive("telegram", "secret.env", "API key");
        let sl = format_sensitive("slack", "secret.env", "API key");
        assert!(tg.contains("<b>"));
        assert!(sl.contains("*"));
        assert!(tg.contains("secret.env"));
        assert!(sl.contains("API key"));
    }

    #[test]
    fn test_format_completion_includes_total() {
        let stats = DbStats { total_documents: 42, ..Default::default() };
        let tg = format_completion("telegram", "f.md", "report", &stats);
        let sl = format_completion("slack", "f.md", "report", &stats);
        assert!(tg.contains("42"));
        assert!(sl.contains("42"));
        assert!(tg.contains("report"));
    }

    #[test]
    fn test_format_summary_telegram_basic() {
        let mut s = ProcessingSummary {
            success: 10,
            errors: 2,
            duplicates: 1,
            ..Default::default()
        };
        s.by_type.insert("meeting".to_string(), 5);
        let out = format_summary_telegram(&s);
        assert!(out.contains("성공: 10"));
        assert!(out.contains("에러: 2"));
        assert!(out.contains("중복: 1"));
        assert!(out.contains("meeting"));
    }

    #[test]
    fn test_format_summary_truncates_issues_to_10() {
        let mut s = ProcessingSummary::default();
        for i in 0..15 {
            s.issues.push(ProcessingIssue {
                filename: format!("f{}.md", i),
                level: "error".into(),
                reason: "오류".into(),
                action_taken: "스킵".into(),
            });
        }
        let out = format_summary_telegram(&s);
        // 10건만 표시 + "외 5건" 메시지
        assert!(out.contains("외 5건"));
        // 11번째는 truncated
        assert!(!out.contains("f10.md"));
    }

    #[test]
    fn test_format_summary_slack_no_html_tags() {
        let s = ProcessingSummary {
            success: 1,
            ..Default::default()
        };
        let out = format_summary_slack(&s);
        // Slack에는 HTML <b> 사용 안 함
        assert!(!out.contains("<b>"));
        assert!(out.contains("*"));
    }
}
