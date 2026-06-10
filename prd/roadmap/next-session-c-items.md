---
created: 2026-05-22
updated: 2026-05-26 (Phase 98 3차 묶음 C-5+C-6+C-8 완료 — C 항목 9/9 ✅ 전체 종결)
purpose: 다음 세션 C 항목 9건 구현 계획. 사용자 명시 "C 항목 모두 구현".
phase_target: Phase 96 (1차 ✅) / Phase 97 (2차 ✅) / Phase 98 (3차 ✅)
source: prd/roadmap/external-trigger-checklist.md (C 카테고리)
status: ✅ **전체 종결 (9/9)** — 외부 신호 의존 항목으로 진입 단계
successor: prd/roadmap/external-trigger-checklist.md (A/B 카테고리 외부 트리거 대기)
---

# 다음 세션 — C 항목 9건 구현 계획

사용자 명시 결정 (2026-05-22): **"다음 세션에서는 C 항목 모두 구현"**.

C 항목은 claude 단독 진행 가능 (외부 입력 무관 + 데이터 누적 무관). 9건 모두 단일 phase 또는 다중 phase로 진행.

## 전체 항목 + 추천 진행 순서

### 1차 묶음 (Phase 96 — 메타 룰 자기 적용 + 위생) ✅ **2026-05-26 완료**

> 종결 표시: C-3 + C-2 + C-4 모두 Phase 96에서 처리. lesson 55 작성. roadmap.md Phase 96 항목 추가. 코드 변경 0건.

**C-3 추정 키워드 grep 재검증** (메타 룰 18 자기 적용)
- 비용: 30분
- 작업:
  ```bash
  grep -nE '추정|것으로 보임|불명|likely|suspect' spec/lesson-learned/*.md
  ```
  결과 lesson 1+건 재검증 후 ✅/❌ 명시
- 사전 검증: 메타 룰 18 강화 체크리스트 자기 적용 확인
- 가치: lesson 49 본문 후속 플래그 종결 (메타 룰 12 자기 적용)

**C-2 메타 룰 1 sub-rule 누적 사례 표 상세화**
- 비용: 1세션
- 작업: 7 카테고리 (1a~1g)에:
  - 누적 lesson 번호 + 패턴 + 해소 방법 + 자동화 도구
  - 신규 작업 사전 체크리스트 sub-rule별 분기
- 사전 검증: 19건 시계열 표 보존 (메타 룰 12) + 카테고리 표 보강
- 가치: 메타 룰 1 가독성 + 신규 작업 lookup 비용 감소

**C-4 domain-map.md ↔ architecture.md 포트 매핑 중복 점검**
- 비용: 30분
- 작업: 메타 룰 19 정식 승격 후 자기 적용. 포트 목록 + 어댑터 매핑 grep 중복 검출 → 단방향 링크 위임
- 사전 검증: 직전 lesson 49 옵션 A 패턴 재적용
- 가치: 메타 룰 19 자기 적용 누적 사례 6건째

### 2차 묶음 (Phase 97 — 코드 영역 확장 + 자동화)

**C-1 A3 trace_id 부착 영역 추가**
- 비용: 1세션
- 작업: 잔여 영역
  - Notion 어댑터 자체 (attach/download/list/delete) — 헥사고날 위반 회피 위해 service.rs upload 호출 시점 추가 또는 NotionStorageAdapter에 옵셔널 audit 필드
  - MCP handle_get_document / handle_list_documents
  - service.rs Verify 호출 / 교차참조
- 사전 검증: 메타 룰 18 grep — 각 위치 정확 확인
- 가치: 메타 룰 13 2단계 완성도 100% 도달

**C-9 메타 룰 24 자동화 — stage 명명 grep**
- 비용: 30분 (스크립트 1개)
- 작업: `spec/benchmarks/scripts/audit_stage_check.sh` 신규
  ```bash
  grep -rnE '\.audit\.record\(' src/ | awk -F'"' '{print $2}' \
    | grep -vE '^(llm|mcp|tauri|remote|verify|service)\.' \
    && echo "FAIL: stage 명명 규칙 위반"
  ```
  pre-push hook 등록
- 가치: 메타 룰 24 후보 → 자동화 + 누적 사례 1건 추가 (META 정식 승격 임계)

**C-7 메타 룰 17 자동화 — release 재빌드 git diff 감지**
- 비용: 1세션
- 작업: `spec/benchmarks/scripts/release_rebuild_required.sh`
  ```bash
  git diff --name-only HEAD | grep -E '\.rs$|ui/.*\.(js|css|html)$' | head
  # 결과 1+건 → workspace + Tauri release 재빌드 의무
  ```
- 제약: git 미저장 환경 우회 옵션 필요 (예: `find -newer .last-release` 대안)
- 가치: 메타 룰 17 자기 적용

### 3차 묶음 (Phase 98 — 후속 위생) ✅ **2026-05-26 완료**

> 종결 표시: C-5 + C-6 + C-8 모두 Phase 98에서 처리. lesson 57 작성. C 항목 9건 전체 종결. release 재빌드 자동 판정 PASS (메타 룰 17 자동화 자기 적용).

**C-5 benchmarks/ JSON 125개 Phase별 아카이빙**
- 비용: 1세션
- 작업:
  - `spec/benchmarks/archive/phase-{NN}/` 폴더 분리
  - scripts/ 경로 갱신 (`replay_trace.sh` / `gui_http_smoke.sh` 등)
  - 신규 측정만 루트 유지
- 사전 검증: G-5 스크립트 baseline 영향 확인
- 가치: 신규 측정 시 baseline 비교 명확화

**C-6 lesson 47 v3 — dead_selector_scan CSS rule scanner**
- 비용: 1세션
- 작업: `dead_selector_scan_v3.js` 신규
  - acorn AST (v2 유지) + CSS rule scanning
  - pb-subtabs 제거 시 `.pb-subtab` CSS 5 rule 잔존 같은 패턴 검출
- 가치: lesson 47 패턴 완전 회귀 차단

**C-8 webapp-design.md 분리** (사용자 결정 영역 — 보류)
- 사용자 결정 필요: 분리 vs 보존 (Phase 56 자문 컨텍스트)
- claude 단독 진행 가능하지만 가치 판단은 사용자

## 사전 검증 의무 (메타 룰 18 + 17 강화)

각 C 항목 진입 전 사전 검증 grep 의무 (Phase 93~95 패턴):

| 항목 | 사전 grep |
|------|---------|
| C-1 | LLM/MCP/Notion 호출처 실제 위치 확인 (path 추정 회피) |
| C-2 | 7 sub-rule별 lesson 누적 정확 확인 (개별 lesson 본문 grep) |
| C-3 | "추정" 키워드 grep 결과 분석 후 재검증 lesson 선정 |
| C-4 | domain-map.md / architecture.md 포트 grep |
| C-5 | benchmarks/ scripts/ 경로 의존성 grep |
| C-6 | 기존 dead_selector_scan v1/v2 CSS 처리 영역 grep |
| C-7 | git diff 미저장 환경 대안 명확화 |
| C-9 | 기존 audit.record stage 명명 누락 영역 grep |

## 메타 룰 자기 적용 예상 (Phase 96~98)

| 룰 | 예상 적용 |
|----|---------|
| 메타 룰 1 sub-rule | C-2에서 7 카테고리 상세화 자기 적용 |
| 메타 룰 12 (잔존 종결) | C-3에서 추정 키워드 lesson 본문 ✅/❌ 갱신 |
| 메타 룰 13 (4단계) | C-1에서 2단계 완성도 100% |
| 메타 룰 17 (release 재빌드) | C-7 자동화 + Phase 96 종결 시 자기 적용 |
| 메타 룰 18 (추정 재검증) | 각 항목 사전 grep 의무 적용 |
| 메타 룰 19 (단일 진실원 위임) | C-4 포트 매핑 자기 적용 |
| 메타 룰 22 후보 | C-8 사용자 명시 합의 필요 → 누적 1건 추가 가능 |
| 메타 룰 24 후보 | C-9 자동화 → 누적 1건 추가 (META 승격 임계 도달 가능) |

## 누적 메타 룰 후보 정식 승격 가능성

Phase 96~98 진행 후 META 정식 승격 가능 후보:

| 후보 | 현재 누적 | C 항목 진행 후 예상 | META 승격 |
|------|--------|------------------|---------|
| 메타 룰 22 (사용자 정책 경계) | 2건 | +1 (C-8 사용자 결정) | 가능성 있음 |
| 메타 룰 23 (승격 기준) | 1건 | +1 (본 plan 자기 적용) | 가능성 있음 |
| 메타 룰 24 (stage 명명) | 1건 | +1 (C-9 자동화) | 가능성 있음 |

## 예상 비용 + 일정

| Phase | 항목 | 비용 |
|-------|------|------|
| **Phase 96** | C-3 + C-2 + C-4 (메타 룰 자기 적용 + 위생) | 1세션 |
| **Phase 97** | C-1 + C-9 + C-7 (코드 + 자동화) | 1세션 |
| **Phase 98** | C-5 + C-6 + (C-8 사용자 결정) | 1세션 |

총 3 phase 예상. 각 phase 끝에 메타 룰 17 release 재빌드 의무 (단, C-7 자동화 도입 후 자동 검출 가능).

## 신규 lesson 예상

- **lesson 55** (Phase 96): 메타 룰 자기 적용 묶음 (메타 룰 1 sub-rule 상세화 + 메타 룰 18 추정 grep + 메타 룰 19 포트 매핑)
- **lesson 56** (Phase 97): A3 영역 완성 + 자동화 2건 (메타 룰 17/24)
- **lesson 57** (Phase 98): 위생 묶음 (benchmarks 아카이빙 + lesson 47 v3)

## 후속 (Phase 99+)

C 항목 종결 후:
- 외부 신호 의존 (A/B 카테고리) 도달 대기
- 신규 외부 프로젝트 분석 (메타 룰 20/21 누적 사례 추가)
- 메타 룰 후보 정식 승격 검토 (22/23/24)
- 사용자 본격 가공 50파일+ 도달 시 H1 audit_anomaly 실측
