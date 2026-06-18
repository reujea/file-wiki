//! step-s4 (2026-06-16, hex-arch-d): `FileProcessingService` 1681줄 → use case 분해.
//!
//! 본 mod = `ProcessFileUseCase` (537) + `CrossRefUseCase` (394) + `MaintenanceUseCase` (26) 분해.
//! `FileProcessingService` 는 파사드 패턴으로 backward compat 유지 (lesson 21/27 정합 — 12+ 통합 테스트 영향 0건 목표).
//!
//! 본 cycle 진행 = `MaintenanceUseCase` 부분 추출 (분해 패턴 확립, 가장 작은 use case 우선).
//! 후속 cycle = `ProcessFileUseCase` + `CrossRefUseCase` 추출 + 파사드 정합.

pub mod crossref;
pub mod maintenance;
pub mod process_file;
