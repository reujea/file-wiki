# Lesson 34 — Decision Log 카드 + 주기 트리거 + C1 확장 + B2 평가 + C2 PII + clippy

## 상황 (2026-05-15)

즉시 가능 + Ruflo 미구현 7건 일괄 처리:
1. Decision Log Dashboard 카드 (Settings 탭)
2. 자동 추천 주기 트리거 (4시간 디폴트)
3. C1 임계값 확장 (quarantine_rate + verify_pass_rate)
4. B2 — 백그라운드 워커 (기존 인프라 평가 결과 사실상 완료)
5. C2 — PII 검출 강화 (정규식 5종)
6. clippy 정리 (error 2 → 0)

## 문제 / 결정 사항

### B2 — 새 구현 vs 기존 평가

"Ruflo B2: 백그라운드 워커"라는 항목명만 보면 신규 구현 필요해 보였지만, 코드 점검 결과 `watcher.rs::process_batch`에 이미 `Semaphore::new(max_workers)` + `tokio::spawn` 패턴이 적용. `max_workers` configField도 노출됨. 즉 **이미 워커 풀 패턴 적용**.

추가 구현 없이 lesson에 "기 완료"로 기록 + spec에 명시. 신규 작업 항목은 코드 점검을 먼저.

### C2 — Rust regex vs presidio 의존

presidio (Microsoft) 같은 ML PII 도구는 외부 의존 무거움 (Python + 모델). 본 프로젝트는 Rust 단일 바이너리 원칙. regex 기반 정밀도 우선 패턴 (5종) 으로 시작:
- ssn_kr: `\b\d{6}[-\s]?[1-4]\d{6}\b`
- credit_card: `\b(?:\d{4}[-\s]?){3}\d{4}\b`
- email: 표준 RFC 5322 단순 형식
- phone_kr: 010-xxxx-xxxx + +82 형식
- biz_reg_kr: xxx-xx-xxxxx

false positive 우려는 컨텍스트 키워드 결합 (`is_sensitive_with_content`) 으로 path negative + 본문 PII 발견 시만 sensitive 마킹.

`OnceLock`으로 regex 1회 컴파일 — 매 호출 비용 0.

### clippy — error 우선 vs warning 일괄

71건 warning + 2건 error. 일괄 정리는 회귀 위험 + 시간 비용 큼. error만 즉시 (deny lint이므로 빌드 실패 위험). warning은 lesson 28 CI 자동화 후속에서 점진 정리.

### Decision Log — Setup 탭 vs Settings 탭

Setup은 모달(`openSetupAssistant`)로 띄우는 진입점. 항상 보이는 카드를 두려면 Settings 탭 상단이 적합. 사용자 동선: 설정 변경 의도 → Settings 탭 진입 → 자동 추천 검토 + 적용 → 일반 설정 편집.

## 원인

1. **B2 사전 평가 누락**: 코드 점검 없이 "Ruflo B2 = 신규 작업"으로 분류. → 신규 작업 항목 분류 시 기존 코드 grep 먼저.
2. **clippy 부재**: `cargo check`만 돌리는 관행 → `#[deny(...)]` 경고를 빌드 시점에 놓침. CI에 clippy 포함 필요.
3. **Settings 탭 진입 hook**: 기존 `switchTab` 분기에 `loadSettings`만 있어 신규 섹션 추가 시 자동 로드 누락 위험. `loadDecisionLog` 호출 명시 필수.

## 개선

### 신규 작업 사전 평가 체크리스트

새 기능 항목 받으면:
1. 기능명 grep으로 기존 코드 검색 (예: B2 → `Semaphore`, `tokio::spawn`)
2. 기존 패턴이 있으면 **신규 구현 vs 가시화·노출 강화**로 분기
3. 신규 구현은 마지막 옵션

### regex PII 패턴 추가 시 패턴

```rust
static PATTERNS: OnceLock<Vec<(&'static str, Regex)>> = OnceLock::new();
// 정밀도 우선: 컨텍스트 동반 필요한 패턴은 별도 함수
// false positive 줄이려면 단위 테스트로 일반 텍스트 음성 검증 필수
```

테스트 패턴:
- positive: 패턴 매치 (각 종류)
- negative (clean text): false positive 0

### clippy CI 통합 후보

```bash
cargo clippy --workspace --lib --tests -- -D warnings
```

`-D warnings`로 모든 경고를 error로 — CI 기준선. 현재 71건 정리 필요.

## 결과

- workspace lib 323 → 331 (core 137 → 143: PII 6건, shared 90 → 92: auto_suggester 2건)
- Tauri commands 67 → 67 (Decision Log API 기존 노출, 신규 추가 없음)
- MCP tools 27 → 27 (동일)
- configFields: search.auto_suggest_interval_hours / schedule 그룹 확장
- clippy error 2 → 0, warning 71 잔존 (후속 트리거)
- 컴파일 경고 0, Tauri GUI check 통과

## 후속

- clippy warning 71건 점진 정리 (lesson 28 CI 자동화와 함께)
- Decision Log 카드에 "이력 다시 보기" 필터 (rejected/critical_skipped 별도 노출)
- C1 임계값을 사용자 설정 가능하게 (auto_suggester rules table → settings.db)
- PII 패턴을 사용자 설정 가능하게 (정규식 추가/제거)
