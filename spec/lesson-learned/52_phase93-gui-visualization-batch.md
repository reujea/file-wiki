---
phase: 93
date: 2026-05-22
topics: GUI 가시화 4건 묶음 / 메타 룰 13 4단계 도달 / 사전 검증 grep으로 추정 빗나감 회피 (state.paths → settings_db_path 정정)
related_lessons: 12, 14, 19, 50, 51
related_meta_rules: 1, 13, 17, 18, 19, 20
---

# 52. Phase 93 — GUI 가시화 4건 묶음 (H1/H3/H5/A2) + 메타 룰 13 4단계 도달

## 상황

Phase 91 후속 (P0 3·4번 — B2 GUI 표시 + A2 PII 토글) + Phase 92 H1/H3/H5 백엔드 4 항목을 단일 phase로 GUI 가시화. 메타 룰 13 "인프라 활성화 4단계" 중 4단계(UI 노출) 도달.

## 문제

### 문제 1: 4 백엔드 항목을 단일 phase로 묶음 적정성

각 항목 GUI 변경은 작지만 (각 1 카드 또는 1 인스펙터 확장), 4건 분리 시 4 release 재빌드 + 4 spec 갱신 부담.
**결정**: 같은 dashboard.js / index.html 영역 (Settings + Verification + Pipeline 인스펙터) 한 묶음으로 진행.

### 문제 2: 추정 빗나감 회피 — `state.paths` 추정

본 Phase 진입 직후 commands.rs에서 `state.paths.base.join("settings.db")` 추정. **사전 검증 grep으로 AppState 실제 필드 확인 → `state.settings_db_path` 사용**으로 정정.
lesson 50 추정 빗나감 4/4 누적 후 메타 룰 18 강화 체크리스트 자기 적용 성공 (추정 빗나감 5번째 차단).

### 문제 3: anomaly-report-card 동적 생성 패턴

기존 Verification 탭 (`tab-verification`)에 `verification-results` + 강한 주장 카드 정적 HTML 존재. 신규 anomaly 카드를 HTML에 추가하지 않고 **JavaScript에서 동적 생성** (`document.createElement('div')`).
- 장점: index.html 변경 최소화 + 카드 부재 시 자동 생성
- 단점: lesson 47 dead_selector_scan 패턴 위반 위험 — 본 phase에서 정적 ID 추가하지 않고 `createElement` 후 ID 부여라 whitelist 안전

## 원인

### 직접 원인

- Phase 91 후속 P0 3·4번이 등록만 되고 미진행 — Phase 92 진행 후 묶음으로 진행하는 것이 효율적
- 메타 룰 13 4단계 진척 — Phase 91 A2/B2 + Phase 92 H1/H3/H5 모두 1~2단계만. 4단계 도달이 lesson 32 "API 정의만 노출 vs Dashboard 카드 통합" 패턴 적용 시점

### 구조적 원인

- `메타 룰 13 4단계 (인프라 → 로직 → 측정 → UI 노출)` 진행 시 4단계가 UI 변경이라 자연스럽게 GUI 묶음 phase 발생
- 백엔드 변경 phase (91/92) ↔ GUI 가시화 phase (93) 분리 패턴 — Tauri release 재빌드 부담 최소화 + 회귀 영향 명확화

## 개선

### 개선 1 — 4건 묶음 진행

| 항목 | 작업 |
|------|------|
| H1 anomaly 카드 | Verification 탭 동적 생성 — 자동 롤백 아닌 사용자 검토 권고 명시 |
| H3 MCP 카탈로그 | Settings 탭 정적 카드 — 26 도구 카테고리/mutates/cost 표시 |
| H5 Notion capability | Pipeline 인스펙터 동적 확장 — remote_upload 노드 선택 시 비동기 로드 |
| A2 PII mask 토글 | Settings 탭 정적 카드 — config.search.output_pii_mask 토글 |

### 개선 2 — Tauri commands 4건 추가

- `get_anomaly_report` / `get_mcp_tool_catalog_full` / `get_remote_storage_capabilities` / `get_pii_mask_config`
- commands.rs +4 함수 / main.rs invoke_handler +4 등록 (lesson 19 10단계 적용)
- 백엔드 호출처 단일 (lesson 32 "API 정의만 노출 vs Dashboard 카드 통합" 자기 적용)

### 개선 3 — dashboard.js +5 함수 묶음

- API 4 메서드 + ViewModel 5+ 함수 (load*/render*/toggle*)
- 탭 진입 트리거 자동 로드 (verification → anomaly, settings → pii + mcp)
- 액션 핸들러 3건 (`pii-mask-toggle` / `refresh-mcp-catalog` / `refresh-anomaly-report`)

### 개선 4 — 사전 검증 grep 자기 적용 (메타 룰 18 강화)

Task 15에서:
1. `dashboard.js` 의 Settings/Verification 탭 구조 grep
2. `index.html` 탭 위치 + 기존 카드 확인
3. `commands.rs` AppState 필드 grep (`state.paths` 추정 → `state.settings_db_path` 실제)
4. `config.rs` `output_pii_mask` 위치 확인

**추정 빗나감 5번째 사례 차단** — Phase 91 직후 lesson 50/51에서 4/4 누적된 패턴을 사전 grep으로 차단.

### 개선 5 — 메타 룰 13 4단계 도달 첫 완성 사례

| 항목 | 1단계 (인프라) | 2단계 (로직) | 3단계 (측정) | 4단계 (UI 노출) |
|------|------------|------------|------------|---------------|
| A2 PII mask | Phase 91 | Phase 91 | (사용자 측정 대기) | **Phase 93 ✅** |
| B2 mutates_state | Phase 91 | Phase 91 | — | **Phase 93 ✅** |
| H1 audit_anomaly | Phase 92 | Phase 92 | (호출처 부착 대기) | **Phase 93 ✅** |
| H3 MCP 다차원 | Phase 92 | Phase 92 | — | **Phase 93 ✅** |
| H5 ResourceCap | Phase 92 | Phase 92 | — | **Phase 93 ✅** |

5 항목 4단계 도달. Phase 88 lesson 42 "인프라 활성화 4단계" 첫 완성 사례(Metadata 보조 필드 → Phase 89 UI 노출)에 이어 본 phase가 5건 동시 4단계 첫 사례.

## 공통 교훈

1. **백엔드 phase (91/92) → GUI phase (93) 분리는 메타 룰 13 진척 자연 패턴** — release 재빌드 부담 최소화 + 회귀 영향 명확화
2. **사전 검증 grep으로 추정 빗나감 차단** — 4건 누적된 패턴을 본 phase에서 처음 차단 성공
3. **동적 생성 패턴은 dead_selector_scan whitelist 안전** — `document.createElement` 후 ID 부여는 정적 ID 검증 대상 외
4. **H1 audit_anomaly는 audit_trace 비어있어도 정상 동작** — "이상 신호 없음" 표시. 메타 룰 13 3단계 (호출처 부착) 미완성 상태에서도 4단계 도달 가능 (역설적이지만 합리적)
5. **자동 롤백 미도입 명시는 사용자 검토 권고만** — JAMES Change Request 인간 게이트 흡수 시 RBAC 보류 정책 자기 적용 (lesson 50 메타 룰 20)

## 잘한 것 (재사용 가능)

1. **묶음 progress 효율적 진행** — 4 백엔드 항목 단일 phase 묶음. Tauri commands 4건 + dashboard.js 4 영역 변경 한 번에
2. **사전 검증 grep 의무 실제 적용 + 1건 정정** — `state.paths` → `state.settings_db_path`. 메타 룰 18 자기 적용 성공
3. **동적 생성 + whitelist 안전** — anomaly-report-card 동적 생성으로 lesson 47 dead_selector_scan 회귀 차단
4. **탭 진입 자동 로드** — verification/settings 탭 진입 시 신규 카드 자동 로드. 사용자 명시 액션 없이도 정보 가시화

## 메타 룰 1 추가 사례 (META.md 갱신)

본 lesson을 메타 룰 1의 19번째 누적 사례로 등재 (잠재 영역, sub-rule 분리 검토 필요):

| lesson | 패턴 | 단일화 |
|--------|------|--------|
| **52** | **백엔드 카탈로그(`mcp_tool_catalog_full`) ↔ frontend 렌더 일치 — 신규 도구 추가 시 양쪽 동기화 의무** | **단일 진입점 백엔드 + Tauri command → frontend 단방향 흐름** (lesson 32 패턴 적용) |

15→17→18→**19건** 누적. 메타 룰 1 sub-rule 분리 임계 명확 도달 (직전 lesson 50/51 후속 플래그).

## 다음 세션 플래그

- [ ] **Tauri release 재빌드 의무** (메타 룰 17, 본 phase Task 22) — pipeline.exe + file-pipeline-tauri.exe
- [ ] H1 audit_anomaly 호출처 부착 (메타 룰 13 3단계 — service.rs 가공 종료 후 주기 호출 / mcp 핸들러 종료 후 비동기)
- [ ] A3 trace_id 호출처 부착 (Phase 91 A3 인프라 활성화 — LLM/검색/MCP 핫패스)
- [ ] 메타 룰 1 sub-rule 분리 (19건 도달, 임계 명확)
- [ ] 메타 룰 19 (단일 진실원 위임) 정식 승격 검토
- [ ] action_catalog.sh --diff baseline 갱신 (Phase 92 68 → 본 phase +3 action)
