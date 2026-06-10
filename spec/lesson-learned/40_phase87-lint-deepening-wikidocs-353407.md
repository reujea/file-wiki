# Lesson 40 — Phase 87 lint 고도화: wikidocs 353407 권고 부분 적용

## 상황

wikidocs 353407(정리와 감사 흐름) 분석 결과, 본 프로젝트는 권고의 ~90%를 이미 구현. 부분 미구현 4건 중 측정 무관 3건을 일괄 적용:

- A-1: `Metadata.needs_verification` + `Metadata.open_questions` 필드 추가 (형식 점검의 미구현 항목)
- A-2: `detect_strong_claims()` 함수 추가 (근거 점검의 "강한 주장 약화" 권고)
- A-3: `ScheduleConfig.lint_weekly_hours` + `lint_monthly_hours` 필드 추가 (다층 lint 주기)

## 문제 / 발견

### 1. 외부 문서 권고 vs 내부 구현의 정합성 점검 방법

wikidocs 353407이 권고하는 5필드(요약/근거/확인됨/확인 필요/다시 물어볼 질문) 중 본 프로젝트는 **확인 필요·다시 물어볼 질문 2건이 미구현**. 단, 구현 시 즉시 LLM 가공 결과로 채워지는 게 아니라 **빈 Vec로 두고 검증·lint 단계에서 채움** — LLM 프롬프트 변경은 별도 후속(A-2 lint 룰이 채우는 게 자연스러움).

**메타 룰**: 외부 권고 도입 시 "필드만 추가" → "값 생성 로직 추가" → "UI 노출"의 3단계 분리. 본 phase는 1단계만 진행 — 인프라 우선. 사용 시점은 트리거 도달 또는 다음 phase.

### 2. 단정 표현 검출 — Vec<String> vs 점수화

wikidocs 353407은 "가능성이 있다"로 약화 표현 권고. 본 구현 `detect_strong_claims`는 검출된 문장을 **Vec<String>으로 반환** — 점수가 아니라 사용자 검토 후보 목록. 이유:
- 검증 점수(0~1)는 자동 처리에 적합하지만, "약화 권고"는 사용자 판단 필요
- 본 함수는 verify_with_thresholds에 통합하지 않고, lint 단계에서 별도 호출 권장
- 빈 Vec = 강한 주장 없음 (또는 모두 안전 맥락)

**메타 룰**: "검증 = 거부가 아니라 피드백"(메타 룰 2) 재적용. 점수화하면 임계값으로 거부하기 쉽지만, 약화 권고는 본질이 다름.

### 3. 다층 lint 주기 — 단일 필드 확장 vs 신규 필드 추가

`lint_interval_hours` 단일 필드를 enum/struct로 바꾸는 대신 **3개 필드 병렬** (hourly/weekly/monthly):
- 기존 코드 변경 0건 (lint_interval_hours 의미 보존)
- serde(default)로 신규 인스턴스 호환
- TOML 사용자가 한 필드만 조정 가능

**메타 룰**: 단일 필드를 복잡한 구조로 바꾸는 것보다 **병렬 필드 추가**가 호환성 측면 안전. lesson 21/27 (구조체 필드 추가 = 동기화 누락) 위험도 낮음 (default 적용 시).

### 4. 구조체 필드 추가 — lesson 21/27 재발 확인

`Metadata`에 2 필드 추가 → `cargo build --tests --all`에서 1 위치(verification 라운드트립 테스트) 누락 발견. ServiceBuilder 도입(lesson 38) 이후 통합 테스트는 영향 없음(서비스 단위에서만 사용). 본 phase에서 lesson 21/27 위험 영역이 **테스트 코드의 모델 라운드트립**임을 재확인 — `..Default::default()` 패턴 적용 권장.

### 5. wikidocs 권고 적용 시 "외부 문서 출처" 명시

코드 주석에 `Phase 87 wikidocs 353407` 명시. 외부 권고 출처를 코드 상단에 인용하면:
- 향후 권고 갱신 시 추적 가능
- 권고와 구현의 차이(우리는 Vec 반환 vs 권고 표현 약화 자동 수정)를 명확히 함

**메타 룰 (신규 후보)**: 외부 문서 권고 도입 시 코드 주석 + lesson 양쪽에 출처(URL/문서 ID) 명시. lesson 14 dead 자산 누적 감시와 같은 추적 가치.

## 개선 / 적용

### 코드 변경 요약

| 파일 | 변경 |
|------|------|
| `crates/core/src/domain/models.rs` | `Metadata.needs_verification` + `Metadata.open_questions` 필드 + 라운드트립 테스트 갱신 |
| `crates/core/src/domain/verification.rs` | `detect_strong_claims()` 함수 + 단위 테스트 4건 |
| `crates/shared/src/config.rs` | `ScheduleConfig.lint_weekly_hours` + `lint_monthly_hours` 필드 + config_metadata 노출 |

### 회귀 기준선

- workspace lib **340** 통과 (Phase 86 336 + 4 신규: detect_strong_claims 4)
- workspace clippy `--all --tests` **0건** 유지
- workspace + Tauri `cargo check` ✅
- 신규 config: `schedule.lint_weekly_hours` (168) / `schedule.lint_monthly_hours` (720) — 둘 다 사용자 토글
- 신규 Metadata 필드 2종 — 디폴트 빈 Vec, 기존 인덱스 호환 (`#[serde(default)]`)

### 후속

- **A-1 후속**: needs_verification / open_questions를 LLM 가공 시 채우려면 prompts.toml `classify` 프롬프트에 두 필드 안내 추가. 또는 별도 lint 룰이 이미 가공된 결과를 분석해 채우는 형태
- **A-2 후속**: `detect_strong_claims`를 lint 흐름에 통합. 현재는 단위 테스트로만 검증, 호출처 0건 (lesson 14 패턴 — 본 phase에선 인프라만)
- **A-3 후속**: lint 다층 주기를 실제 실행 흐름에 연결 (현재 ScheduleConfig 필드만, schedule task가 이 값을 읽어 다층 분기 필요)
- **A-4 보류**: "수집일 vs 점검일 분리" — 본 프로젝트는 가공+검증 통합 사이클이 적절하다는 판단 (수집·점검 분리는 위키 운영에 적합, 자동 파이프라인엔 과한 분리)
- **외부 문서 분석 결과** (`prd/research/external-analysis-supertonic-wikidocs.md`로 별도 기록 권장):
  - supertonic: TTS, 본 프로젝트 직접 연관 없음 (ONNX 패턴은 fastembed에 이미 차용)
  - wikidocs 352523: 자기 진화 에이전트 패턴. 본 프로젝트 C1/decision_log/config_snapshot이 이미 부분 적용
  - wikidocs 353407: 본 phase에서 부분 적용 완료
