# Lesson Learned: Phase 98 위생 묶음 (C-5 + C-6 + C-8)

## 상황

2026-05-26 Phase 98 진행. 사용자 명시 "Q1 진행" → next-session-c-items.md 3차 묶음 (C-5 + C-6 + C-8) 선택. C 항목 9건 모두 종결 단계. C-8(webapp-design)은 사용자 결정 "현행화 재작성" 명시 합의 후 진행.

본 phase 변경 분류 결과 release_rebuild_required.sh가 "재빌드 불필요" 자동 판정 — **메타 룰 17 자동화 첫 자기 적용**.

## 문제

### 문제 1 — benchmarks/ JSON 124개 단일 디렉토리 누적 (C-5)

Phase 47~64 시대(2026-04 측정)의 110개 JSON과 Phase 80+ 시대(2026-05 측정)의 14개가 같은 디렉토리에 평면 누적. 신규 측정 시 baseline 비교 위치 식별 비용 + 자동화 스크립트가 모든 파일 스캔 가능성. 메타 룰 12(잔존 N건 종결 의무) 변형.

### 문제 2 — dead_selector_scan CSS rule scanner 부재 (C-6)

v2(JS↔HTML id 매칭) 검출 후 lesson 47 패턴은 부분 차단되었으나, CSS rule이 dead 잔존하는 경우 검출 못 함. 예: `pb-subtab*` 제거 시 CSS 5 rule 잔존 (lesson 47 §1차 발견 사례).

### 문제 3 — webapp-design.md 변동 이력 누적 stale (C-8)

Phase 56 자문용 컨텍스트 + Phase 65~90 변동 이력 누적으로 **337줄**. 현행 구조 파악에 변동 이력 우회 의존. 자문 재요청 시점도 불명.

### 문제 4 — false positive 처리 패턴 (C-6 진행 중 발견)

dead_selector_scan_v3 1차 작동 결과 63건 DEAD CSS rule 검출. 분석 결과:
- 진짜 dead: `.search-box` 등 (lesson 47 회귀 차단 가치 입증)
- false positive: `.pb-midtab` 등 — JS template literal `\`<div class="pb-midtab${active}"\`` 패턴에서 quasi 단위 처리 시 `class=...` 경계 매칭 실패

TemplateLiteral 전체를 `${EXPR}`로 결합하는 패턴으로 수정 → 63→57건. 잔여 57건 중 진짜 dead 분류는 별도 phase 작업 (false positive 추가 처리 + 부모 셀렉터 분리 대응 필요).

## 원인

### 직접 원인
1. (문제 1) 측정 시점마다 baseline 비교 의무가 없어 archive 트리거 미발동. Phase 89에서 micro_profile 등 6건 측정 시 archive 결정 없이 평면 누적
2. (문제 2) v2 작성 시 ID 매칭만 우선순위 + "CSS rule scanner는 후속"으로 분리. lesson 47 §개선에 "CSS rule scanner는 v3 후보"로 명시되었으나 다음 phase 트리거 부재
3. (문제 3) Phase 56 자문 컨텍스트 보존 정책이 변동 누적과 양립 어려움. 매 phase 변동 이력 추가 vs 재작성 결정 보류
4. (문제 4) AST walk에서 TemplateLiteral을 TemplateElement 개별 처리 — `class="..."` 패턴의 경계가 ${} 보간으로 잘림. v2 walker 패턴 그대로 차용 시 동일 한계

### 구조적 원인
- spec 문서의 "변동 이력 누적" vs "현행 재작성" 결정 기준 부재 — 사용자 결정 영역에 매번 의존 (메타 룰 22 후보 변형)
- AST 기반 도구는 정확도 vs false positive 트레이드오프 존재. CSS rule scanner는 **부모 셀렉터 / 속성 셀렉터 / 외부 CDN** 처리 한계로 100% 정밀도 불가
- 회귀 게이트와 점검 도구 분리 정책 부재 — v3는 게이트로 사용 시 false positive로 빌드 차단 위험 → "점검 도구"로 분류 필요

## 개선

### 즉시 적용 (본 Phase 98 완료)

- ✅ `spec/benchmarks/archive/phase47-64-2026-04/` 신규 — 112개 JSON 이관 (10건 5월 측정만 루트 잔존)
- ✅ archive/phase47-64-2026-04/README.md 신규 — 측정 기간 / 주요 결과 / 관련 lesson + 단일 진실원 위임 (메타 룰 19 자기 적용)
- ✅ `spec/benchmarks/scripts/dead_selector_scan_v3.js` 신규 — v2 ID 매칭 + CSS rule scanner. TemplateLiteral 결합 패턴으로 false positive 일부 차단 (63→57)
- ✅ `spec/webapp-design.md` 전면 재작성 (337→187줄, -44%). 변동 이력 누적 제거 + Phase 97 현행 IA 기준
- ✅ G-5 회귀 게이트 영향 0 (3 게이트 모두 PASS)
- ✅ release_rebuild_required.sh "재빌드 불필요" 자동 판정 — **메타 룰 17 자동화 첫 자기 적용 통과** (수동 판정 → 자동화 도구가 결정)

### 메타 룰 강화 (다음 phase 자기 적용)

- [ ] **회귀 게이트 vs 점검 도구 분리 명시** — META.md G-5 7종 목록에 "회귀 게이트 (exit 0 의무)" vs "점검 도구 (참고용)" 분류 추가. v3는 후자로 분류
- [ ] **benchmarks 아카이빙 트리거** — 측정 후 90일 경과 시 archive 분리 자동화 후보 (메타 룰 24 변형)
- [ ] **dead_selector_scan v3 false positive 분석 + 정리 후속 phase** — 57건 중 진짜 dead 식별 후 dashboard.css 정리. 메타 룰 13 4단계 (UI 노출 = dead 정리 완료) 패턴 자기 적용

### 메타 룰 후보 추가 검토

- **메타 룰 27 후보 (Phase 98 신규)**: **"회귀 게이트(exit 0 의무) vs 점검 도구(참고) 분류 의무"** — false positive 가능 도구는 게이트화 금지. 신규 도구 작성 시 정밀도 임계 검증 후 게이트 승격. 누적 1건만 (v3).

## 다음 세션 플래그

- C 항목 9건 모두 완료 (3/3/3 = 9/9). 외부 신호 의존 항목으로 진입 단계
- 외부 트리거 체크리스트(prd/roadmap/external-trigger-checklist.md) A/B 카테고리 도달 대기
- dead_selector_scan_v3 57건 false positive 분리 + 진짜 dead 정리 (별도 phase)
- 메타 룰 후보 누적: 21/22/23/24/25/26/27 = **7건** → 메타 룰 23 후보(승격 기준) 정형화 우선순위 최상
- audit_trace 누적 50건+ 도달 시 lesson 46 G-1 root cause 검증 (Phase 91~95 인프라 활용)
- webapp-design.md 자문 재요청 시점 도달 시 webapp-design-phase56.md 백업본 부재로 git history 또는 git log 의존 필요

## 회귀 기준선

| 지표 | Phase 97 | Phase 98 | 차이 |
|------|---------|---------|------|
| benchmarks/*.json 루트 | 124 | **12** (-112 archive) | -90% |
| benchmarks/archive/ | 0 | **1 폴더 112 파일** | 신규 분리 |
| 회귀 게이트 | 7 | **8** (+ dead_selector_scan_v3 점검 도구) | +1 |
| webapp-design.md | 337줄 | **187줄** | -44% |
| C 항목 진행도 | 6/9 | **9/9** ✅ | 완료 |
| audit.record 호출처 | 12 | 12 | 동일 (코드 변경 0건) |
| workspace cargo check | 통과 | (검증 생략 — Rust 변경 0건) | — |
| release 재빌드 의무 | — | **불필요 (자동 판정 PASS)** | 메타 룰 17 자동화 자기 적용 |
| 메타 룰 후보 누적 | 6 | **7** (+ 메타 룰 27 후보) | +1 |
