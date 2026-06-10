use std::path::Path;
use std::sync::OnceLock;

use regex::Regex;

/// 민감 파일 판별기
pub struct SensitivityDetector {
    keywords: Vec<String>,
    sensitive_extensions: Vec<String>,
}

/// Ruflo C2: PII 정규식 패턴 (정적 컴파일 1회).
///
/// 정밀도 우선: false positive 줄이려고 컨텍스트 키워드 동반 필요한 패턴은 별도 함수에서 처리.
fn pii_patterns() -> &'static [(&'static str, &'static Regex)] {
    static PATTERNS: OnceLock<Vec<(&'static str, Regex)>> = OnceLock::new();
    static REFS: OnceLock<Vec<(&'static str, &'static Regex)>> = OnceLock::new();
    REFS.get_or_init(|| {
        let owned = PATTERNS.get_or_init(|| vec![
            // 한국 주민등록번호: 6자리-7자리 (체크섬 검증 안 함 — 형식만)
            ("ssn_kr", Regex::new(r"\b\d{6}[-\s]?[1-4]\d{6}\b").expect("ssn_kr")),
            // 신용카드: 16자리 (4-4-4-4 형식 또는 연속)
            ("credit_card", Regex::new(r"\b(?:\d{4}[-\s]?){3}\d{4}\b").expect("credit_card")),
            // 이메일
            ("email", Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b").expect("email")),
            // 한국 휴대전화: 010-xxxx-xxxx 또는 +82-10-xxxx-xxxx
            ("phone_kr", Regex::new(r"\b(?:\+82[-\s]?)?0?1[016789][-\s]?\d{3,4}[-\s]?\d{4}\b").expect("phone_kr")),
            // 한국 사업자등록번호: xxx-xx-xxxxx
            ("biz_reg_kr", Regex::new(r"\b\d{3}[-\s]?\d{2}[-\s]?\d{5}\b").expect("biz_reg_kr")),
        ]);
        owned.iter().map(|(k, r)| (*k, r)).collect()
    });
    REFS.get().expect("pii REFS init")
}

/// PII 검출 결과
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PiiHit {
    pub kind: String,
    pub count: usize,
}

/// Phase 91 A1': 민감/PII 검사 단일 결정 결과.
///
/// `check_sensitive_and_pii`의 반환형. 호출처는 `is_sensitive`/`scan_pii_in_text_with` 직접 호출
/// 대신 본 구조체로 결정을 받는다. 메타 룰 1/14/19 자기 적용.
#[derive(Debug, Clone)]
pub struct SensitivityDecision {
    pub is_sensitive: bool,
    pub reason: Option<String>,
    pub pii_hits: Vec<PiiHit>,
}

impl SensitivityDecision {
    pub fn safe() -> Self {
        Self { is_sensitive: false, reason: None, pii_hits: Vec::new() }
    }
}

impl SensitivityDetector {
    pub fn new(extra_keywords: Vec<String>, extra_extensions: Vec<String>) -> Self {
        let mut keywords: Vec<String> = vec![
            "계약", "contract", "법률", "legal", "소송",
            "개인정보", "주민", "여권", "passport",
            "진단", "처방", "의료", "병원",
            "급여", "연봉", "세금", "통장", "계좌",
            "비밀", "confidential", "secret", "private",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        keywords.extend(extra_keywords);

        let mut sensitive_extensions: Vec<String> = vec![
            ".pdf", ".docx", ".hwp", ".xlsx",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        sensitive_extensions.extend(extra_extensions);

        Self {
            keywords,
            sensitive_extensions,
        }
    }

    /// 민감 파일 여부 판별 (3단계)
    /// 반환: (민감 여부, 이유)
    pub fn is_sensitive(&self, path: &Path) -> (bool, Option<String>) {
        let path_str = path.to_string_lossy().to_lowercase();

        // 1단계: 경로에 /sensitive/ 포함
        if path_str.contains("sensitive") || path_str.contains("민감") {
            return (true, Some("경로에 민감 디렉토리 포함".into()));
        }

        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_lowercase();

        // 2단계: 파일명에 키워드 포함
        for kw in &self.keywords {
            if filename.contains(&kw.to_lowercase()) {
                return (true, Some(format!("파일명에 민감 키워드 '{}' 포함", kw)));
            }
        }

        // 3단계: 민감 확장자 + 키워드 조합
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| format!(".{}", e.to_lowercase()))
            .unwrap_or_default();

        if self.sensitive_extensions.contains(&ext) {
            for kw in &self.keywords {
                if path_str.contains(&kw.to_lowercase()) {
                    return (
                        true,
                        Some(format!("민감 확장자 '{}' + 경로에 키워드 '{}'", ext, kw)),
                    );
                }
            }
        }

        (false, None)
    }

    /// Ruflo C2: 본문 텍스트에서 PII 패턴 검출 (디폴트 5종만).
    pub fn scan_pii_in_text(text: &str) -> Vec<PiiHit> {
        Self::scan_pii_in_text_with(text, &[])
    }

    /// Ruflo C2: 디폴트 + 사용자 정의 추가 패턴 (이름, regex). regex 컴파일 실패는 silent skip.
    pub fn scan_pii_in_text_with(text: &str, extra: &[(String, String)]) -> Vec<PiiHit> {
        let mut hits = Vec::new();
        for (kind, re) in pii_patterns() {
            let count = re.find_iter(text).count();
            if count > 0 {
                hits.push(PiiHit { kind: kind.to_string(), count });
            }
        }
        for (name, pat) in extra {
            if let Ok(re) = Regex::new(pat) {
                let count = re.find_iter(text).count();
                if count > 0 {
                    hits.push(PiiHit { kind: name.clone(), count });
                }
            }
        }
        hits
    }

    /// 경로 + 본문 결합 판별. 본문에 PII가 있으면 민감으로 분류.
    /// 본문 검사는 path 기반 검사가 negative일 때만 수행 (이미 sensitive면 skip).
    pub fn is_sensitive_with_content(&self, path: &Path, content: &str) -> (bool, Option<String>) {
        let (path_sensitive, reason) = self.is_sensitive(path);
        if path_sensitive {
            return (true, reason);
        }
        let hits = Self::scan_pii_in_text(content);
        if hits.is_empty() {
            return (false, None);
        }
        let summary: Vec<String> = hits.iter()
            .map(|h| format!("{}×{}", h.kind, h.count))
            .collect();
        (true, Some(format!("본문 PII 감지: {}", summary.join(", "))))
    }

    /// Phase 91 A2: 출력 텍스트의 PII를 마스킹하여 반환. 입력 차단 못한 PII가
    /// 검색 결과/MCP 응답으로 유출되는 갭 차단.
    ///
    /// 마스킹 형식: `[REDACTED:kind]` (예: `[REDACTED:email]`).
    /// 검색 결과 header 같은 짧은 텍스트에 적용 권장. 긴 본문은 비용 고려.
    pub fn mask_pii_in_text(text: &str, user_patterns: &[(String, String)]) -> String {
        let mut result = text.to_string();
        for (kind, re) in pii_patterns() {
            result = re.replace_all(&result, format!("[REDACTED:{}]", kind)).into_owned();
        }
        for (name, pat) in user_patterns {
            if let Ok(re) = Regex::new(pat) {
                result = re.replace_all(&result, format!("[REDACTED:{}]", name)).into_owned();
            }
        }
        result
    }

    /// Phase 91 A1': 경로 + 본문 + 사용자 정의 PII 패턴 통합 단일 진입점.
    ///
    /// 호출 순서: 경로/파일명 sensitive → 본문 PII (디폴트 + 사용자 정의) → SensitivityDecision.
    /// content가 None이면 경로 기반 검사만 수행. content가 Some이면 PII까지 검사.
    /// 호출처 통일 대상: `process_file_with_pipeline` / `simulate_pipeline` /
    /// `process_file_legacy`(삭제 예정). 메타 룰 1/14/19 자기 적용.
    pub fn check_sensitive_and_pii(
        &self,
        path: &Path,
        content: Option<&str>,
        user_patterns: &[(String, String)],
    ) -> SensitivityDecision {
        // 1단계: 경로/파일명/확장자 기반 판별 (LLM 호출 전 필수)
        let (path_sensitive, reason) = self.is_sensitive(path);
        if path_sensitive {
            return SensitivityDecision { is_sensitive: true, reason, pii_hits: Vec::new() };
        }

        // 2단계: content 제공 시 본문 PII 검사
        if let Some(text) = content {
            let hits = Self::scan_pii_in_text_with(text, user_patterns);
            if !hits.is_empty() {
                let summary: Vec<String> = hits.iter().map(|h| format!("{}×{}", h.kind, h.count)).collect();
                return SensitivityDecision {
                    is_sensitive: true,
                    reason: Some(format!("본문 PII 감지: {}", summary.join(", "))),
                    pii_hits: hits,
                };
            }
        }

        SensitivityDecision::safe()
    }
}

impl Default for SensitivityDetector {
    fn default() -> Self {
        Self::new(vec![], vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sensitive_keyword() {
        let d = SensitivityDetector::default();
        let (sensitive, reason) = d.is_sensitive(Path::new("계약서_2026.pdf"));
        assert!(sensitive);
        assert!(reason.unwrap().contains("계약"));
    }

    #[test]
    fn test_not_sensitive() {
        let d = SensitivityDetector::default();
        let (sensitive, _) = d.is_sensitive(Path::new("meeting_notes.txt"));
        assert!(!sensitive);
    }

    #[test]
    fn test_pii_ssn_kr() {
        let hits = SensitivityDetector::scan_pii_in_text("주민번호 950101-1234567 확인 부탁");
        assert!(hits.iter().any(|h| h.kind == "ssn_kr"), "ssn_kr 매치: {:?}", hits);
    }

    #[test]
    fn test_pii_credit_card_and_email() {
        let text = "카드: 4111-1111-1111-1111, 이메일: john.doe@example.com";
        let hits = SensitivityDetector::scan_pii_in_text(text);
        assert!(hits.iter().any(|h| h.kind == "credit_card"));
        assert!(hits.iter().any(|h| h.kind == "email"));
    }

    #[test]
    fn test_pii_phone_kr() {
        let hits = SensitivityDetector::scan_pii_in_text("연락처 010-1234-5678");
        assert!(hits.iter().any(|h| h.kind == "phone_kr"));
    }

    #[test]
    fn test_no_pii_in_clean_text() {
        let hits = SensitivityDetector::scan_pii_in_text("이번 주 회의 주제는 분기 OKR 정리입니다.");
        assert!(hits.is_empty(), "false positive: {:?}", hits);
    }

    #[test]
    fn test_sensitive_with_content_triggers_on_pii() {
        let d = SensitivityDetector::default();
        let (sensitive, reason) = d.is_sensitive_with_content(
            Path::new("memo.txt"),
            "이메일: john@example.com",
        );
        assert!(sensitive);
        assert!(reason.unwrap().contains("PII"));
    }

    #[test]
    fn test_sensitive_with_content_path_takes_precedence() {
        let d = SensitivityDetector::default();
        let (sensitive, reason) = d.is_sensitive_with_content(
            Path::new("계약서.pdf"),
            "일반 내용",
        );
        assert!(sensitive);
        assert!(reason.unwrap().contains("계약"));
    }

    // Phase 91 A1': 단일 진입점 검사 테스트
    #[test]
    fn test_check_sensitive_and_pii_path_only_safe() {
        let d = SensitivityDetector::default();
        let dec = d.check_sensitive_and_pii(Path::new("meeting_notes.txt"), None, &[]);
        assert!(!dec.is_sensitive);
        assert!(dec.pii_hits.is_empty());
    }

    #[test]
    fn test_check_sensitive_and_pii_path_sensitive() {
        let d = SensitivityDetector::default();
        let dec = d.check_sensitive_and_pii(Path::new("계약서_2026.pdf"), None, &[]);
        assert!(dec.is_sensitive);
        assert!(dec.reason.unwrap().contains("계약"));
        assert!(dec.pii_hits.is_empty());
    }

    #[test]
    fn test_check_sensitive_and_pii_content_pii() {
        let d = SensitivityDetector::default();
        let dec = d.check_sensitive_and_pii(
            Path::new("memo.txt"),
            Some("이메일: john@example.com"),
            &[],
        );
        assert!(dec.is_sensitive);
        assert!(dec.reason.unwrap().contains("PII"));
        assert!(dec.pii_hits.iter().any(|h| h.kind == "email"));
    }

    #[test]
    fn test_check_sensitive_and_pii_user_pattern() {
        let d = SensitivityDetector::default();
        let extra = vec![("custom_id".to_string(), r"\bACC-\d{6}\b".to_string())];
        let dec = d.check_sensitive_and_pii(
            Path::new("memo.txt"),
            Some("계좌 ACC-123456 참조"),
            &extra,
        );
        assert!(dec.is_sensitive);
        assert!(dec.pii_hits.iter().any(|h| h.kind == "custom_id"));
    }

    #[test]
    fn test_check_sensitive_and_pii_path_takes_precedence() {
        let d = SensitivityDetector::default();
        let dec = d.check_sensitive_and_pii(
            Path::new("계약서.pdf"),
            Some("이메일: a@b.com"),
            &[],
        );
        // 경로가 먼저 잡혀서 PII 검사로 안 들어감
        assert!(dec.is_sensitive);
        assert!(dec.reason.unwrap().contains("계약"));
        assert!(dec.pii_hits.is_empty()); // path가 먼저 잡혀 PII는 검사 안 함
    }

    // Phase 91 A2: 출력 PII mask 테스트
    #[test]
    fn test_mask_pii_email() {
        let s = "연락처: john@example.com 으로 문의";
        let m = SensitivityDetector::mask_pii_in_text(s, &[]);
        assert!(m.contains("[REDACTED:email]"));
        assert!(!m.contains("john@example.com"));
    }

    #[test]
    fn test_mask_pii_multiple() {
        let s = "주민 950101-1234567 / 카드 4111-1111-1111-1111";
        let m = SensitivityDetector::mask_pii_in_text(s, &[]);
        assert!(m.contains("[REDACTED:ssn_kr]"));
        assert!(m.contains("[REDACTED:credit_card]"));
    }

    #[test]
    fn test_mask_pii_user_pattern() {
        let extra = vec![("token".to_string(), r"\bsk-[a-z0-9]{8}\b".to_string())];
        let s = "API key: sk-abcd1234 노출";
        let m = SensitivityDetector::mask_pii_in_text(s, &extra);
        assert!(m.contains("[REDACTED:token]"));
        assert!(!m.contains("sk-abcd1234"));
    }

    #[test]
    fn test_mask_pii_no_op_on_clean() {
        let s = "이번 주 회의 주제";
        let m = SensitivityDetector::mask_pii_in_text(s, &[]);
        assert_eq!(m, s);
    }
}
