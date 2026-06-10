//! Phase 91 A3: 감사 추적 단일 키 (trace_id) + 입력 해시 헬퍼.
//!
//! settings.db `audit_trace` 테이블에 기록되는 결정의 도메인 타입.
//! 헥사고날 유지를 위해 본 모듈은 SQL/DB에 의존하지 않는다 — `SettingsDb::record_audit_event`가
//! 본 타입을 받아 영속화한다.
//!
//! ## 메타 룰 적용
//! - 메타 룰 1: 모든 결정 1줄 기록 — 다중 위치 분산 차단
//! - 메타 룰 18: "추정 재검증 의무" — trace로 root cause 확인
//! - lesson 7: quarantine 분기 누락 검출

use std::fmt;

/// 결정 단위 식별자. UUID v4 형식 (32자 hex + 4 hyphen).
///
/// 단일 사용자 요청 / 단일 파일 가공 / 단일 검색 호출 등 "맥락 단위"를 하나의 trace_id로 묶는다.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TraceId(String);

impl TraceId {
    /// 신규 UUID v4 trace_id 생성. uuid 크레이트 의존을 피하기 위해 간단한 생성기 사용.
    pub fn new() -> Self {
        // 단순 timestamp + 카운터 기반. 강한 유일성 필요시 외부 uuid 크레이트로 교체.
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        Self(format!("{:016x}-{:08x}", ts, n))
    }

    /// 외부 trace_id 수용 (예: MCP client가 전달한 trace_id).
    /// 메서드명은 `from_string`을 사용 — `FromStr` trait의 from_str와 의미가 다름 (실패 없음).
    pub fn from_string(s: &str) -> Self {
        Self(s.to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TraceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for TraceId {
    fn default() -> Self {
        Self::new()
    }
}

/// 입력 데이터의 SHA-256 16자 prefix (디버깅용 짧은 해시).
///
/// 전체 입력 보존은 settings.db에 부담이라 짧은 해시로 식별.
pub fn input_hash_prefix(input: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(input);
    let hash = hasher.finalize();
    format!("{:x}", hash).chars().take(16).collect()
}

/// 출력 요약 (최대 200자, 줄바꿈 제거).
pub fn truncate_output_summary(output: &str) -> String {
    let cleaned: String = output.chars().filter(|c| *c != '\n' && *c != '\r').collect();
    if cleaned.chars().count() <= 200 {
        cleaned
    } else {
        cleaned.chars().take(200).collect::<String>() + "..."
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trace_id_unique() {
        let a = TraceId::new();
        let b = TraceId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn test_input_hash_deterministic() {
        let h1 = input_hash_prefix(b"hello");
        let h2 = input_hash_prefix(b"hello");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 16);
    }

    #[test]
    fn test_truncate_output_short() {
        let s = "short text";
        assert_eq!(truncate_output_summary(s), s);
    }

    #[test]
    fn test_truncate_output_long() {
        let s = "a".repeat(300);
        let t = truncate_output_summary(&s);
        assert!(t.chars().count() <= 203); // 200 + "..."
        assert!(t.ends_with("..."));
    }

    #[test]
    fn test_truncate_strips_newlines() {
        let s = "line1\nline2\rline3";
        let t = truncate_output_summary(s);
        assert!(!t.contains('\n'));
        assert!(!t.contains('\r'));
    }
}
