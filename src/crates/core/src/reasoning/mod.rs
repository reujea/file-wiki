//! Phase 91 B1: 추론·검증 미들웨어 모듈.
//!
//! JAMES (v0.3.0) cognitive middleware §5.7의 verification agent 패턴을 file-pipeline
//! 도메인에 맞춰 흡수 — RBAC/5 역할 상한제는 보류, 검증 함수 단일 진입점만 적용.
//!
//! 메타 룰 1/14 자기 적용: 흩어진 검증 함수(`verify_with_thresholds` / `detect_strong_claims`
//! / `Linter::lint_strong_claims`)를 단일 호출 표면으로 묶어 다중 위치 동기화 누락 차단.

pub mod verifier;
