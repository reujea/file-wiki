---
phase: 91
date: 2026-05-21
topics: JAMES v0.3.0 cognitive middleware 패턴 흡수 / RBAC 도메인 가정 정렬 / 추정 빗나감 3/3 / 메타 룰 자기 적용 5건 일괄
related_lessons: 14, 18, 19, 26, 28, 46, 48, 49
related_meta_rules: 1, 9, 13, 14, 18, 19
---

# 50. Phase 91 — JAMES 패턴 흡수 (RBAC/외부 협업 보류) + 추정 빗나감 3/3 메타 룰 18 적용

## 상황

외부 프로젝트 JAMES (Hashevolution/James-RAG-Evol, v0.3.0 Platform Skeleton, 2026-05-17) 분석 후 cognitive middleware §5.7 + 3-stage 보안 파이프라인 + PolicyEngine + Change Request 패턴 검토. file-pipeline 도메인 가정과 일치 영역만 흡수, 나머지(RBAC / 다중 사용자 / Change Request 인간 게이트 / 외부 협업·솔루션·연계)는 보류 결정.

## 문제

### 문제 1: 도메인 가정 불일치 영역의 흡수 결정

JAMES는 다중 사용자 + 자가진화 + 인간 승인 게이트 전제. file-pipeline은 단일 사용자 desktop + 자동 가공이 본질. 외부 좋은 패턴을 일괄 흡수하면 over-engineering. 패턴별 도메인 정렬 매트릭스가 필요.

### 문제 2: 추정 빗나감 누적 (메타 룰 18 강화)

본 phase 진입 직후 service.rs:235 ↔ service.rs:644 "활성 분기 2곳 중복"이라 추정했으나 실제는:
- service.rs:235 = `process_file_legacy` #[allow(dead_code)] (호출처 0건 deprecated)
- service.rs:644 = `process_file_with_pipeline` 활성 진입점 1곳

추정 빗나감 누적:
- lesson 46 G-1: "동시 호출 / stdin / 짧은 파일" → 외부 일시 요인
- lesson 46 G-4: "5건 invoke-no-fallback" → 1건만 진짜 + dead-code
- **Phase 91**: "service.rs 2곳 중복" → 1곳만 활성 + 다른 곳은 dead

**3/3 = 100% 추정 빗나감 유지**. 메타 룰 18 강화 필요.

### 문제 3: 광범위 변경 + git 미저장 + lesson 48 dirty 사고 위험

본 phase 변경 범위:
- Rust: 8 파일 (classifier.rs / service.rs / config.rs / settings_db.rs / mcp_server.rs / cli.rs / main.rs / commands.rs)
- 신규 모듈: 3 (audit.rs / reasoning/mod.rs / reasoning/verifier.rs)
- 신규 스크립트: 1 (replay_trace.sh)
- spec/lesson: 4 (architecture.md / deprecated.md / domain-map.md / lesson 50)

git 미저장 환경에서 lesson 48 git stash 사고 + 추정 빗나감 위험 양면. 사용자 명시 합의("무조건 5건 강행") 후 진행.

## 원인

### 직접 원인 (추정 빗나감 누적)

- 직전 분석에서 grep 결과(`service.rs:235:235 / service.rs:644 같은 분기`)만 보고 활성성 검증 누락
- `process_file_legacy`의 `#[allow(dead_code)]` 마커를 사전 확인하지 않음
- 메타 룰 18 "추정 재검증 의무"가 lesson 46 G-1/G-4 이후 정식 승격(2026-05-20)되었음에도 본 분석에 즉시 자기 적용 안 함

### 구조적 원인 (외부 프로젝트 흡수 패턴)

- 외부 프로젝트는 자체 도메인 가정 위에 설계됨 — 흡수 시 가정 정렬 의무가 명문화 안 됨
- JAMES의 "3-stage 보안 파이프라인"은 다중 사용자 RBAC 전제. file-pipeline 적용 시 1단계(출력 마스킹)만 의미 있음
- "좋은 패턴"이라는 인상이 도메인 불일치 영역까지 흡수 욕구를 유발

## 개선

### 개선 1 — JAMES 패턴 흡수 5건 (RBAC/외부 협업 제외)

| 항목 | 작업 | 도메인 정렬 |
|------|------|------------|
| A1' SensitivityDecision | 검사 분산 통일 (RBAC 아닌 검사 통합) | ✅ 적용 — 메타 룰 1/14/19 자기 적용 |
| A2 출력 PII mask | `mask_pii_in_text` + 검색 응답 적용 | ✅ 적용 — 3-stage 중 output 1단계만 |
| A3 trace_id | `audit_trace` 테이블 + `TraceId` + replay_trace.sh | ✅ 적용 — RBAC 무관 감사 추적 |
| B1 Verifier 통합 | `reasoning/verifier.rs` 단일 진입점 wrapper | ✅ 적용 — 5 역할 상한제 보류, 함수 통합만 |
| B2 MCP mutates_state | `mcp_tool_mutates_state` + 카탈로그 24 도구 분류 | ✅ 적용 — Change Request 게이트 보류, 메타 표시만 |

### 보류 (도메인 불일치)

| 항목 | 보류 사유 |
|------|----------|
| PolicyEngine 4 메서드 (can_retrieve/walk/call_tool/emit) | 단일 사용자 — `can_walk`/`can_call_tool` 적용 영역 없음 |
| Change Request 인간 게이트 | approver ≠ proposer 불변식 (현재 proposer=approver=single user) |
| 5 역할 상한제 (Orchestrator/Specialist/...) | 역할 개념 자체 없음 |
| 메모리 3계층 (system/workspace/session) | system/workspace 구분 가치 미증명 |
| OpenSSF Best Practices 신청 | 외부 인증 (외부 협업/연계 보류 정책) |
| injection regression fixtures 외부 협업 | Ali Afana 같은 외부 협업 패턴 (외부 협업 보류) |

### 개선 2 — 메타 룰 18 강화 (추정 빗나감 3/3 누적)

본 lesson을 메타 룰 18의 3번째 누적 사례로 META.md 갱신:

| Lesson | 원래 추정 | 실제 검증 결과 |
|--------|----------|----------------|
| lesson 46 G-1 | "동시 호출 / stdin / 짧은 파일 추정" | 외부 일시 요인 |
| lesson 46 G-4 | "5건 invoke-no-fallback" | 1건만 진짜 + dead-code |
| **lesson 50 Phase 91** | **"service.rs 235↔644 활성 중복"** | **235 = dead deprecated, 644만 활성** |

빗나감 비율 = 3/3 = 100% (메타 룰 18 메인 신호 강화).

**재검증 체크리스트 보강**:
- [ ] grep 결과를 활성/dead 추가 검증: `grep -B5 "fn {함수명}" {파일}.rs | grep -E "allow\(dead_code\)|deprecated"`
- [ ] 추정한 "중복 분기" 모두 호출처 grep으로 활성성 확인 의무
- [ ] phase 시작 시 본인이 만든 추정 1개 이상 격리 검증

### 개선 3 — 메타 룰 9 자기 적용 (빌드 진단)

`cargo build --tests --all`에서 `E0463 can't find crate` + `os error 1455` (페이징 파일 부족) 발생. 메타 룰 9 "빌드 진단은 자원 먼저" 적용 — `cargo build -j 2`로 즉시 해소 (lesson 37 / Phase 84 사례 재발 차단).

### 개선 4 — 외부 프로젝트 흡수 결정 매트릭스 정형화 (메타 룰 신규 후보)

본 phase에서 사용한 결정 매트릭스를 메타 룰 후보로 등록:

> **메타 룰 N 후보 — 외부 프로젝트 패턴 흡수 시 도메인 가정 정렬**
>
> 외부 프로젝트의 좋은 패턴을 흡수할 때 다음 3축 분리 의무:
> 1. **패턴 추출** — "무엇이 좋은가" (디자인 패턴)
> 2. **도메인 가정 검증** — "그 패턴이 전제하는 도메인이 우리 도메인과 일치하는가"
> 3. **부분 흡수 결정** — 일치 영역만 흡수, 불일치 영역 보류 표시
>
> 흡수 항목 라벨: 🟢 적용 / 🟡 부분 적용 (mode 분기) / 🔴 보류 (다른 도메인 트리거 대기)
>
> 메타 룰 16(차원 A/B) 사전 분류 라벨과 결합. 누적 사례: lesson 45 Notion mode 분기 + lesson 50 JAMES RBAC 보류.

### 개선 5 — 메타 룰 자기 적용 5건 일괄 (Phase 91 코드 변경 결과물)

| 메타 룰 | Phase 91 자기 적용 |
|--------|------------------|
| 메타 룰 1 (다중 위치 동기화) | classifier.rs 단일 진입점 (3분기 → 1진입점) + B1 Verifier wrapper + B2 카탈로그 일치성 검증 |
| 메타 룰 9 (빌드 진단 자원 먼저) | `-j 2` 적용으로 E0463 회피 |
| 메타 룰 13 (인프라 활성화 4단계) | A3 trace_id는 **1단계(인프라 추가)** 만 완료. 호출처 부착(2단계) + 측정(3단계) + UI 노출(4단계) 별도 phase |
| 메타 룰 14 (다중 진입점 통일) | service.rs + simulate_pipeline + classify_and_process_with_retry 통일 |
| 메타 룰 18 (추정 재검증) | 추정 빗나감 3/3 누적 명시 + 재검증 체크리스트 보강 |
| 메타 룰 19 (단일 진실원 위임) | classifier.rs 단일 진입점 + Verifier wrapper + 카탈로그 일치성 |

## 공통 교훈

1. **외부 프로젝트 흡수는 도메인 가정 정렬 의무** — 좋은 패턴이라도 자기 도메인 불일치면 over-engineering. JAMES RBAC/Change Request는 다중 사용자 전제라 단일 사용자 desktop에 부적합
2. **추정 빗나감 3/3 → 메타 룰 18 핵심 신호로 자리 잡음** — 본인 추정은 신뢰할 수 없다는 패턴 확정. 재검증 의무는 grep + `#[allow(dead_code)]` / deprecated 마커 확인 + 호출처 활성성 검증 필수
3. **메타 룰 자기 적용 5건 동시** — 단일 phase에서 코드 통합으로 룰 1/14/19 + 빌드로 룰 9 + 검증으로 룰 18 모두 자기 적용. 메타 룰이 phase 단위로 실증되는 첫 사례
4. **인프라 활성화 4단계의 1단계만 진행 의도적 분리** — A3 trace_id는 테이블 + 도메인 타입 + replay 도구만. 호출처 부착은 별도 phase. 메타 룰 13 적용으로 "동작 여부 모름" 회귀 차단

## 잘한 것 (재사용 가능)

1. **5건 묶음 단일 phase 진행** — 사용자 명시 합의("무조건 강행") + 진입 전 추정 빗나감 보고로 책임 분리 명확화 (lesson 48 dirty 사고 회피)
2. **메타 룰 9 적시 적용** — `os error 1455` 발견 즉시 `-j 2` 전환. lesson 37 패턴 재발 차단
3. **lesson 28 패턴 적용 — process_file_legacy 삭제 시 호출처 0건 검증 + deprecated.md 즉시 등재** — lesson 14 회귀 차단
4. **MCP 카탈로그 일치성 자체 테스트** — `mcp_tool_mutates_state` ↔ `mcp_tool_catalog` 매핑 일치성을 단위 테스트로 검증. 신규 도구 추가 시 자동 회귀 검출
5. **분석-합의-적용 3단계 분리 유지** — 본 작업 진입 전 추정 빗나감 발견 → 사용자 보고 → 명시 합의 → 진행. lesson 49 패턴 재사용

## 메타 룰 1 추가 사례 누적 (META.md 갱신 의무)

본 lesson을 메타 룰 1의 16~17번째 누적 사례로 META.md에 추가:

| lesson | 패턴 | 단일화 가능 여부 |
|--------|------|------------------|
| **50-A** | **`service.rs` 3분기 (process_file_with_pipeline + process_file_legacy + simulate_pipeline) → SensitivityDecision 단일 진입점** | **classifier.rs `check_sensitive_and_pii` 통일 완료** |
| **50-B** | **검증 함수 3개 (verify_with_thresholds + detect_strong_claims + lint_strong_claims) → Verifier wrapper** | **reasoning/verifier.rs 통일 완료** |

## 다음 세션 플래그

- [ ] META.md 메타 룰 1: 15건 → 17건 누적 추가 (50-A + 50-B)
- [ ] META.md 메타 룰 18: 3/3 빗나감 누적 갱신
- [ ] META.md 메타 룰 20 후보 신규 등록: "외부 프로젝트 패턴 흡수 시 도메인 가정 정렬" (메타 룰 16과 결합)
- [ ] A3 trace_id 호출처 신규 부착 (2단계 진입) — LLM/검색/MCP 호출에 trace_id 주입
- [ ] B2 GUI Settings 카드 `mutates_state` 분류 표시 (다음 phase)
- [ ] A2 PII mask 사용자 토글 GUI (다음 phase)
- [ ] Tauri release 재빌드 의무 (메타 룰 17): A2 mcp_server.rs 변경 + Tauri commands.rs 변경 포함 → 재빌드 필요
