//! Phase 91 B1: 검증 단일 진입점 (Verification Agent).
//!
//! 흩어진 검증 함수를 본 모듈의 `Verifier`로 묶는다:
//! - `verify_with_thresholds` (6 검증) → `Verifier::verify_processed`
//! - `detect_strong_claims` (Phase 87 단정 표현) → `Verifier::detect_strong_claims`
//! - `Linter::lint_strong_claims` (Phase 88 lint) → `Verifier::lint_strong_claims`
//!
//! 기존 함수는 그대로 두어 호환성 유지. 본 wrapper는 신규 호출처 / 단일 진입점 패턴이 필요한
//! 경우 사용. 메타 룰 14 "다중 진입점 분기 트리 통일" 자기 적용.
//!
//! ## 사용 예
//! ```ignore
//! let verifier = Verifier::new();
//! let result = verifier.verify_processed(original, processed, &sections, &keywords, None, &thresholds);
//! let claims = verifier.detect_strong_claims(processed);
//! ```

use std::collections::HashMap;

use crate::domain::models::VerificationResult;
use crate::domain::verification::{
    detect_strong_claims as detect_strong_claims_impl,
    verify_with_thresholds,
    VerificationThresholds,
};

/// 검증 단일 진입점. 상태 없음 — `Default::default()` 또는 `new()`로 자유 생성.
#[derive(Debug, Default, Clone)]
pub struct Verifier;

impl Verifier {
    pub fn new() -> Self {
        Self
    }

    /// 6 검증 (구조/압축/키워드 커버리지/키워드 완전성/ROUGE-L/개체 보존) 실행.
    /// `verify_with_thresholds`의 wrapper.
    #[allow(clippy::too_many_arguments)]
    pub fn verify_processed(
        &self,
        original: &str,
        processed: &str,
        required_sections: &[String],
        keywords: &[String],
        sections: Option<&HashMap<String, Vec<String>>>,
        thresholds: &VerificationThresholds,
    ) -> VerificationResult {
        verify_with_thresholds(original, processed, required_sections, keywords, sections, thresholds)
    }

    /// Phase 87 wikidocs 353407: 단정 표현 검출 (확실히/반드시/항상/100% 등).
    /// 점수화 아닌 후보 목록 반환 — 사용자 검토용.
    pub fn detect_strong_claims(&self, processed: &str) -> Vec<String> {
        detect_strong_claims_impl(processed)
    }

    // lint_strong_claims는 VectorDBPort + StoragePort 의존이라 본 wrapper에 직접 노출하지
    // 않는다 (헥사고날 경계 유지). 호출처는 기존 `Linter::lint_strong_claims` 사용.
    // 본 진입점은 "단일 문서 진단" 용도 — service.rs 가공 직후 호출용.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verifier_default() {
        let v = Verifier::new();
        let v2 = Verifier;
        // both should be equivalent (no state)
        assert_eq!(format!("{:?}", v), format!("{:?}", v2));
    }

    #[test]
    fn test_verifier_detect_strong_claims_passthrough() {
        let v = Verifier::new();
        let claims = v.detect_strong_claims("이 방법은 확실히 모든 경우에 작동한다.");
        assert!(!claims.is_empty());
    }

    #[test]
    fn test_verifier_no_claims() {
        let v = Verifier::new();
        let claims = v.detect_strong_claims("일반적으로 권장됩니다.");
        assert!(claims.is_empty());
    }
}
