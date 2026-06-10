---
created: 2026-06-05
phase: Phase 200 시리즈 진입 전 (본 세션 종결)
related_lessons:
  - 71 (Linux cross-build — 메타 룰 17 강화 정식 승격 3건째 사례)
  - 72 (본질 재정의 2차 — Phase 200~209 단계 합의)
  - 73 (mydocsearch_decision.md 즉시 삭제 — 메타 룰 22+19 결합 패턴 첫 사례)
meta_rules:
  - 메타 룰 17 강화 (release 재빌드 + 배포 의무) — **2026-06-05 강화 정식 승격**
  - 메타 룰 19 (단일 진실원 위임) — 자기 적용 9건째 (S-5 도구로 게이트 도달)
  - 메타 룰 22 (사용자 정책 경계 합의) — 12건째 (사용자 "1~6 진행" 단일 트리거)
  - 메타 룰 25 (자기 적용 의무) — 강화 정식 승격 직후 자기 적용 5건 (lesson 71 + B-8 + release_redeploy.sh + single_source_check.sh + 본 lesson)
  - 메타 룰 27 (게이트 vs 점검) — 누적 +2 (release_redeploy 게이트 + single_source_check 점검) → **2026-06-05 정식 승격** (M-3, 누적 3건 도달)
  - 메타 룰 30 sub-rule "도구 stale" — **2026-06-05 후보 신규 등재** (G-1f, lesson 74 G1 1건째)
  - 메타 룰 30 (spec 본문 phase별 즉시 갱신) — 누적 9건째 (S-1 + S-4 + S-5)
side_findings:
  - G1 (gui_http_smoke 7탭 stale, Phase 107 미반영) — 즉시 해소
  - G2 (architecture.md 수치 stale, dead_selector_scan 88/92 → 94 / action_catalog 68 → 72) — 본 현행화 묶음에 흡수
---

# Lesson 74 — 2026-06-05 6 묶음 처리 세션: 사용자 1줄 트리거 + 자동화 도구 stale 자기 발굴

## 상황

본 세션은 사용자 "spec 폴더 분석해" 시작 → 누적 4 트리거 + 6 묶음 처리 종결.

세션 시계열:
1. **spec 분석 보고** (8 본문 + 72 lesson + META 862줄) — Q1 mydocsearch 즉시 이관 결정
2. **lesson 73 등재** — "Phase N 진입 시 처리" 표기 spec 즉시 처리 패턴 정형화 (메타 룰 22+19 결합)
3. **다음 고도화 항목 5축 22건 정리** — M-1 메타 룰 17 강화 정식 승격 우선 진행 결정
4. **M-1 종결** — META.md 3 섹션 분산 → 단일 §정식 + 2 위임 표시. 강화 누적 3건 도달
5. **6 묶음 처리** — S-4 / S-1 / S-6 / P-2 / S-3 / S-5 사용자 "1~6 진행" 단일 트리거로 한 번에 진행

## 문제

본 세션 진입 시 누적 6 영역 동시 영향:
1. **spec 본문 stale** 다발 — webapp-design 헤더 + architecture 수치 + "Phase N 시" 표기
2. **자동화 도구 부재** 영역 2건 — 메타 룰 17 강화 §자동화(release_redeploy) + 메타 룰 19/30 자동화(single_source_check)
3. **Phase 200 baseline 부재** — Phase 209 회귀 게이트 비교 기준 미수립
4. **lesson 49 옵션 A 완전성 미검증** — archive ↔ deprecated 추가 발굴 미실시
5. **메타 룰 17 강화 후보** 3 섹션 분산 (정식 + 후보 2) — 메타 룰 19 자기 위반 잔존
6. **baseline 측정 자체가 도구 stale 발굴 메커니즘** 가시화 부재

## 원인

1·4·5는 누적 stale의 자연 결과 (메타 룰 30 자기 위반 패턴 반복).
2·3은 인프라 미작성 (메타 룰 23 §승격 3요소 중 "체크리스트/자동화 도구" 미충족).
6은 본 세션 P-2 baseline 측정 직후 가시화 — **회귀 게이트가 도구 자체의 stale을 자기 검출하는 메타 가치** 첫 사례.

## 개선

### 6 묶음 처리 결과

| ID | 작업 | 산출 | 메타 룰 |
|----|------|-----|--------|
| **S-4** | spec "Phase N 시" 표기 grep + 분류 | 즉시 처리 1건 (architecture.md:145 "Pipeline 이관 검토 → 옵션 A 결정 완료" 추가) + 정당한 트리거 대기 3건 + 자기 해소 2건 (M-1 + S-5) | 30 + 25 |
| **S-1** | webapp-design.md 헤더 갱신 | updated 2026-06-01 → 2026-06-05 + status_note 신규 (본문 6탭 ↔ 본질 1도메인 host 불일치는 Phase 208 미루기 명시) | 30 자기 적용 7건째 |
| **S-6** | `release_redeploy.sh` 신규 작성 | D:\file-test 잔류 binary 감지 + sha256 검증 + Windows(tasklist/taskkill)/Linux(ps/kill) 분기 + --check/--apply 안전 분리 + README §메타 룰 자동화 표 추가 | 17 강화 정식 §자동화 + 27 게이트 |
| **P-2** | 회귀 게이트 7종(+1) baseline 측정 | `gate_baseline_phase200pre_20260605.json` 보존 + **사이드 G1 발견·즉시 해소** | 4 (3회 중앙값 baseline) + 30 |
| **S-3** | archive ↔ deprecated 중복 추가 발굴 | lesson 49 옵션 A 적용 완전성 재검증 결과 추가 누락 0건 + **사이드 G2 발견** | 19 자기 적용 |
| **S-5** | `single_source_check.sh` 신규 작성 | spec 본문 5종 "삭제/폐기/제거" + 위임 표시 누락 후보 출력 + META.md sub-rule 1g 행 자동화 도구 등재 + 메타 룰 19/30 §자동화 동시 등재 | 19 + 30 + 27 점검 |

### 사이드 발견 G1 — 회귀 게이트 자체의 stale 자기 검출 메커니즘 (메타 가치)

P-2 baseline 측정 중 `gui_http_smoke.sh` Test 5 FAIL (7탭 검사에 verification 포함). 원인: Phase 107(verification → processing 흡수, 7→6탭)이 dashboard.js / index.html에는 반영됐지만 회귀 게이트 스크립트는 stale.

**메타 가치**: 회귀 게이트는 일반적으로 코드 회귀만 잡는다고 인식되지만, **baseline 측정 자체가 도구의 stale을 자기 검출**한다는 새 패턴. 본 세션이 첫 사례.

| Phase | 코드/UI 변경 | 도구 갱신 | 발견 시점 |
|-------|------------|---------|---------|
| Phase 107 | 7→6탭 통합 | 누락 | Phase 107 종결 시점 (메타 룰 30 자기 위반) |
| 본 세션 P-2 | (변경 없음) | (변경 없음) | baseline 측정 시 FAIL → 즉시 해소 |

신규 체크리스트 추가 후보: **phase 종결 시 회귀 게이트 + 도구도 변경 영향 점검 의무** (메타 룰 30에 sub-rule "도구 stale" 추가 검토).

### 사이드 발견 G2 — architecture.md 본문 수치 stale 다발

S-3 archive ↔ deprecated 추가 발굴 중 발견. architecture.md 본문에 분산된 수치들이 시간축 결정 맥락에는 정확하지만 최신 수치는 stale:
- `dead_selector_scan: 88 ID 통과` (Phase 92 결정 맥락) / `92 ID` (Phase 93) → 실측 **94 ID** (본 세션 P-2)
- `action_catalog 카운트` → 본 세션 실측 **72** (기존 baseline 68)
- Tauri commands 수치 — Phase 107 +1 / 본질 재정의 누적 영향 점검 필요

G2는 본 세션 lesson 74 등재 후 "프로젝트 현행화" 묶음에 흡수 처리.

### 메타 룰 25 자기 적용 5건 (강화 정식 승격 직후)

M-1 메타 룰 17 강화 정식 승격 후 즉시 자기 적용 사례 누적:
1. ✅ lesson 71 meta_rules 갱신 (강화 정식 사례 명시)
2. ✅ external-trigger-checklist B-8 표 "✅ 정식 승격" 갱신
3. ✅ `release_redeploy.sh` 신규 작성 (S-6, 후보 → 도구)
4. ✅ `single_source_check.sh` 신규 작성 (S-5, 메타 룰 19/30 §검증 grep 요소 충족)
5. ✅ 본 lesson 등재

### 메타 룰 22 12건째 — 사용자 1줄 트리거 묶음 처리

사용자 "1~6 진행해" 단일 메시지로 6 묶음 처리. 비대칭 비용 패턴:
- 사용자 입력 비용: 1 메시지
- claude 처리: 6 작업 + 사이드 2건 발견 + 자동화 2종 신규 + baseline 보존 + 메타 룰 4건 동시 자기 적용

## 측정

| 지표 | 변경 |
|------|------|
| spec 본문 파일 | 7 유지 (M-1 종결 후 변동 없음) |
| 회귀 자동화 스크립트 | 7 → **9종** (release_redeploy + single_source_check 신규) |
| 메타 룰 자동화 도구 | 2 → **4건** |
| 메타 룰 17 강화 분산 | 3 섹션 → **1 정식 + 2 위임** |
| 메타 룰 25 자기 적용 누적 | 3 → **5건** (강화 정식 직후 5 사례) |
| 메타 룰 30 자기 적용 누적 | 6 → **9건** |
| baseline json 보존 | 0 → **1건** (`gate_baseline_phase200pre_20260605.json`) |
| 사이드 발견 | 0 → 2건 (G1 즉시 / G2 본 현행화 묶음 흡수) |
| 본 세션 lesson 등재 | 73 → **74** |

## 본 lesson의 메타 가치

1. **회귀 게이트 자체의 stale 자기 검출 메커니즘** (G1) — phase 종결 시 도구도 점검 영역 확장 (메타 룰 30 신규 sub-rule 후보)
2. **사용자 1줄 트리거로 6 묶음 + 사이드 2건 동시 처리** — 메타 룰 22 비대칭 비용 패턴 확장
3. **자동화 도구 신규 작성 직후 자기 적용 의무** — 메타 룰 25 5건 동시 사례
4. **메타 룰 정식 승격 → 자동화 → 자기 적용 → lesson 등재** 전체 사이클 한 세션 종결 (본 세션이 첫 사례)

## 후속 트리거

- **본 현행화 묶음에 흡수 진행 중** (사용자 "Q1, Q2 포함해서 프로젝트 현행화 해" 트리거):
  - architecture.md 수치 stale 일괄 동기화 (G2)
  - spec 본문 5종 + roadmap + external-trigger 본 세션 누적분 반영
- **별도 트리거 대기**:
  - release 재빌드 + `release_redeploy.sh --apply` (사용자 환경 영향)
  - M-2/M-3/M-4 메타 룰 후보(24/27/31) Phase 200 진입 시 자연 누적
