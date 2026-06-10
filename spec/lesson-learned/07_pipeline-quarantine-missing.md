# 07: 파이프라인 검증 quarantine 누락

## 상황
Phase 49 E2E 테스트에서 검증 실패 시 quarantine 이동을 테스트하던 중 발견.

## 문제
`process_file_with_pipeline()` (파이프라인 기반 처리)에서 검증 2-Pass 실패 시 quarantine으로 이동하지 않고, 단순히 재가공 결과를 덮어쓰며 가공을 계속 진행함. `process_file_legacy()` (기존 레거시)에는 quarantine 로직이 완전히 구현되어 있었으나 파이프라인 리팩터링 시 누락.

## 원인
파이프라인 기반 처리(`process_file_with_pipeline`)로 리팩터링 시 Verify 스텝의 FAIL 핸들러가 1차 재가공만 구현하고 2차 검증+quarantine 분기를 구현하지 않음.

## 개선
Verify 스텝에 2차 검증 + quarantine 이동 로직을 추가 (service.rs 719~751행).
- 1차 FAIL → 피드백 재가공
- 2차 검증: 여전히 FAIL → quarantine 이동 + 알림 + summary 기록 + return Ok(())
- 2차 통과 → result 덮어쓰기

## 교훈
기능 리팩터링(legacy → pipeline) 시 모든 분기 경로를 검증하는 E2E 테스트가 필수. 특히 에러/실패 경로는 happy path보다 누락되기 쉽다.
