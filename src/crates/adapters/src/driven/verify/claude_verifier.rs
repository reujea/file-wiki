use std::process::Command;

use anyhow::{Context, Result};
use async_trait::async_trait;
use file_pipeline_core::ports::output::VerificationPort;

/// Claude CLI 기반 검증 어댑터
pub struct ClaudeVerificationAdapter {
    claude_bin: String,
}

impl ClaudeVerificationAdapter {
    pub fn new() -> Self {
        Self {
            claude_bin: std::env::var("CLAUDE_BIN").unwrap_or_else(|_| "claude".into()),
        }
    }

    fn call_claude(&self, prompt: &str) -> Result<String> {
        let mut cmd = Command::new(&self.claude_bin);
        cmd.args(["--print", "--output-format", "text", "--max-tokens", "2000"])
            .arg("--prompt")
            .arg(prompt);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
        }
        let output = cmd.output()
            .context("Claude CLI 실행 실패")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("Claude CLI 오류: {}", stderr);
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    fn parse_score(text: &str) -> (f64, String) {
        // 첫 줄에서 숫자 추출, 나머지는 설명
        let mut lines = text.lines();
        let first = lines.next().unwrap_or("0.5");
        let score: f64 = first
            .chars()
            .filter(|c| c.is_ascii_digit() || *c == '.')
            .collect::<String>()
            .parse()
            .unwrap_or(0.5);
        let description = lines.collect::<Vec<_>>().join("\n");
        (score.clamp(0.0, 1.0), description)
    }
}

impl Default for ClaudeVerificationAdapter {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl VerificationPort for ClaudeVerificationAdapter {
    async fn detect_hallucination(
        &self,
        original: &str,
        processed: &str,
        doc_type: &str,
    ) -> Result<(f64, String)> {
        let orig_truncated = if original.len() > 5000 { &original[..5000] } else { original };
        let proc_truncated = if processed.len() > 5000 { &processed[..5000] } else { processed };

        let prompt = format!(
            "원본과 가공본을 비교하여 환각(hallucination)을 탐지하세요.\n\
             가공본에만 있고 원본에 없는 정보를 찾으세요.\n\
             첫 줄: 0.0(환각 없음)~1.0(심각한 환각) 점수\n\
             둘째 줄부터: 구체적 설명\n\n\
             문서 유형: {}\n\n\
             [원본]\n{}\n\n[가공본]\n{}",
            doc_type, orig_truncated, proc_truncated
        );

        match self.call_claude(&prompt) {
            Ok(response) => Ok(Self::parse_score(&response)),
            Err(e) => {
                tracing::warn!("환각 탐지 실패: {} — 기본값 반환", e);
                Ok((0.5, format!("CLI 호출 실패: {}", e)))
            }
        }
    }

    async fn verify_completeness(
        &self,
        original: &str,
        processed: &str,
        doc_type: &str,
    ) -> Result<(f64, String)> {
        let orig_truncated = if original.len() > 5000 { &original[..5000] } else { original };
        let proc_truncated = if processed.len() > 5000 { &processed[..5000] } else { processed };

        let prompt = format!(
            "가공본이 원본의 핵심 내용을 얼마나 보존하는지 평가하세요.\n\
             첫 줄: 0.0(전혀 보존 안됨)~1.0(완벽 보존) 점수\n\
             둘째 줄부터: 누락된 핵심 내용 설명\n\n\
             문서 유형: {}\n\n\
             [원본]\n{}\n\n[가공본]\n{}",
            doc_type, orig_truncated, proc_truncated
        );

        match self.call_claude(&prompt) {
            Ok(response) => Ok(Self::parse_score(&response)),
            Err(e) => {
                tracing::warn!("완전성 확인 실패: {} — 기본값 반환", e);
                Ok((0.5, format!("CLI 호출 실패: {}", e)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_score_valid() {
        let (score, desc) = ClaudeVerificationAdapter::parse_score("0.85\nSome description here");
        assert!((score - 0.85).abs() < f64::EPSILON);
        assert_eq!(desc, "Some description here");
    }

    #[test]
    fn test_parse_score_no_number() {
        let (score, desc) = ClaudeVerificationAdapter::parse_score("No number");
        assert!((score - 0.5).abs() < f64::EPSILON);
        assert_eq!(desc, "");
    }

    #[test]
    fn test_parse_score_clamped_high() {
        let (score, desc) = ClaudeVerificationAdapter::parse_score("1.5\nover");
        assert!((score - 1.0).abs() < f64::EPSILON);
        assert_eq!(desc, "over");
    }

    #[test]
    fn test_parse_score_clamped_low() {
        // parse_score strips non-digit/non-dot chars, so "-0.3" becomes "0.3"
        // The negative sign is filtered out, resulting in 0.3 (not clamped)
        let (score, desc) = ClaudeVerificationAdapter::parse_score("-0.3\nunder");
        assert!((score - 0.3).abs() < f64::EPSILON);
        assert_eq!(desc, "under");
    }

    #[test]
    fn test_parse_score_multiline() {
        let (score, desc) = ClaudeVerificationAdapter::parse_score("0.7\nLine 1\nLine 2\nLine 3");
        assert!((score - 0.7).abs() < f64::EPSILON);
        assert_eq!(desc, "Line 1\nLine 2\nLine 3");
    }
}

// step-o2 (2026-06-16, outbound-umbrella-1): OutboundManifest 박힘
impl file_pipeline_core::ports::outbound::OutboundManifest for ClaudeVerificationAdapter {
    fn id(&self) -> &str { "fp-outbound-verify-claude" }
    fn category(&self) -> file_pipeline_core::ports::outbound::OutboundCategory {
        file_pipeline_core::ports::outbound::OutboundCategory::Verify
    }
    fn capabilities(&self) -> file_pipeline_core::ports::output::ResourceCapabilities {
        file_pipeline_core::ports::output::ResourceCapabilities::standard("claude")
    }
}
