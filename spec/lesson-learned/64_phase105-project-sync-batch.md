# Lesson Learned: Phase 105 프로젝트 현행화 — spec 본문 누적 stale 발견

## 상황

2026-05-27 Phase 105. 사용자 명시 "프로젝트 현행화 해" 트리거. comm-spec + comm-prd + comm-log 3 스킬 연계 워크플로 진행.

Phase 100~104 각 phase 종결 시 lesson 본문 + roadmap.md 갱신은 진행했으나, `spec/architecture.md` / `spec/domain-map.md` / `spec/webapp-design.md` 본문 갱신은 누락. 본 phase 통합 갱신 시점에 누적 stale 5건 발견.

## 문제

### 문제 1 — spec/architecture.md 수치 Phase 80~89 시점

본문 line 1721/1722:
- "Tauri commands **71개**" (Phase 89 시점) — 실제 Phase 95 시점 65개
- "MCP **32개 도구**" (Phase 80~84 시점) — 실제 Phase 102 시점 25개

본 phase에서 메타 룰 19 자기 적용으로 위임 표시 (`prd/roadmap.md` 단일 진실원). 향후 phase별 즉시 갱신 의무.

### 문제 2 — spec/webapp-design.md Phase 100/101 미반영

Phase 98 C-8 전면 재작성 시점이 Phase 97 IA 기준. Phase 100 (settings-nav 4→5, 운영 그룹 신규) + Phase 101 (UI 코드명 (C1)(C2) 8건 제거) 미반영 잔존.

### 문제 3 — spec/domain-map.md Phase 103 미반영

Phase 96 메타 룰 19 자기 적용 시점 갱신. Phase 103 G1~G4 흡수 (Metadata.statements / RelationType::Semantic / SearchConfig +2 / McpState +2) 미반영.

### 문제 4 — 매 phase 본문 즉시 갱신 의무 부재 (구조적)

Phase 종결 시점 작업:
- ✅ lesson 본문 작성
- ✅ INDEX.md 추가
- ✅ prd/roadmap.md Phase 항목 추가
- ❌ spec 본문 갱신 (architecture / domain-map / webapp-design 영향 시점)

매 phase 종결 시 spec 본문 영향 여부 확인 + 즉시 갱신 의무 부재. CLAUDE.md "기능 완료 시 문서 동기화 필수" 항목과 충돌. 사용자 명시 "프로젝트 현행화" 트리거에만 의존 → 5 phase 누적 stale.

## 원인

### 직접 원인
1. (문제 1~3) 매 phase 종결 시 spec 본문 영향 여부 사전 검증 부재. lesson 본문 / roadmap 갱신만 진행
2. (문제 4) 메타 룰 25 (자기 적용 의무) §체크리스트는 "메타 룰 정식 승격 시 자기 적용"만 명시 — "phase 종결 시 spec 본문 갱신" 별도 항목 부재

### 구조적 원인
- 메타 룰 19 (단일 진실원 위임)는 phase 종결 시점이 아닌 _문제 발견 시점_에 자기 적용 — Phase 96 domain-map / Phase 105 architecture 사례 모두 사후 발견
- spec 본문은 "현행 상태 기록" 역할 / lesson은 "변경 이력" 역할 — 양쪽 동시 갱신 의무 명문화 부재
- CLAUDE.md "기능 완료 시 문서 동기화 필수" 항목은 cargo nextest 통과 시점 트리거 — 본 프로젝트는 git 미저장 + nextest 명시 호출 부재로 트리거 안 됨

## 개선

### 즉시 적용 (본 Phase 105 완료)

#### spec 본문 통합 갱신
- ✅ `spec/architecture.md`: Phase 100~104 통합 섹션 추가 + 수치 행 메타 룰 19 위임 표시
- ✅ `spec/domain-map.md`: Phase 103 GraphRAG 흡수 표 신규 (G1~G4 매핑)
- ✅ `spec/webapp-design.md`: Settings 5그룹 (운영 신규) + UI 라벨 정책 (메타 룰 28) + Phase 102 optimize 반영

#### 외부 트리거 체크리스트 갱신
- ✅ `prd/roadmap/external-trigger-checklist.md`: B-9 GraphRAG 흡수 트리거 4건 (G1~G4) 신규 등재
- ✅ 메타 룰 후보 표 갱신 (23 정식 / 24 2건 / 27 1건)

### 메타 룰 강화 (다음 phase 자기 적용)

- [ ] **메타 룰 25 §체크리스트 추가**: "phase 종결 시 spec 본문 영향 여부 grep 의무 — architecture.md / domain-map.md / webapp-design.md 각각 영향 영역 점검 후 즉시 갱신"
- [ ] **메타 룰 30 후보 등록 검토**: "spec 본문 매 phase 즉시 갱신 의무 (사용자 명시 현행화 트리거 의존 회피)"
  - 누적 사례: Phase 105 본 lesson 5건 일괄 발견
  - 트리거: 다음 phase 종결 시점부터 적용
- [ ] **CLAUDE.md 갱신 검토**: "기능 완료 시 문서 동기화" 항목에 "spec 본문 phase별 즉시 갱신" 명시 (cargo nextest 트리거 대신 phase 종결 트리거)

## 다음 세션 플래그

- 메타 룰 30 후보 등록 후 누적 모니터링 (Phase 106 이상에서 비슷한 사례 재발 시 정식 승격 검토)
- 사용자 GUI 재실행 + G4 TF-IDF 활성화 측정 (트리거 #G4 도달 후)
- 메모리 `feedback_no_phase_in_ui.md`는 메타 룰 28 정식 승격 (Phase 104)으로 spec 등재 완료 — memory archive 후속 작업
- 잔여 메타 룰 후보 2건 (24 stage 명명 / 27 회귀 게이트 vs 점검) 추가 누적 대기

## 회귀 기준선

| 지표 | Phase 104 | Phase 105 | 차이 |
|------|---------|---------|------|
| spec 본문 stale 영역 | 3건 (architecture / domain-map / webapp-design) | **0건** | -3 |
| spec/architecture.md updated 날짜 | 2026-05-22 (Phase 95) | **2026-05-27** | +10일 (5 phase 갱신) |
| spec/domain-map.md updated | 2026-05-26 (Phase 96) | **2026-05-27** | Phase 103 반영 |
| spec/webapp-design.md updated | 2026-05-26 (Phase 98) | **2026-05-27** | Phase 100~103 반영 |
| external-trigger-checklist B 카테고리 | 8건 (B-1~B-8) | **9건** (+ B-9 GraphRAG) | +1 |
| 코드 변경 | — | **0건** | release 재빌드 불필요 |

## 사이드 발견

- Phase 100~104 각 phase 종결 시 즉시 spec 갱신 의무 부재가 lesson 64 본 사례의 원인
- spec 본문 갱신을 "사용자 명시 현행화 트리거"에만 의존 → 5 phase 누적 stale 회귀
- 메타 룰 19 위임 표시 (architecture.md → roadmap.md) 패턴이 큰 본문 차원 갱신 비용 ↓ — 메타 룰 19 9건째 자기 적용
- comm-spec `format-delta-only` 정책 정합 — 변경분만 추가, 기존 본문 보존
- 메타 룰 12 (잔존 N건 종결 의무)의 본문 stale 변형 — phase별 즉시 갱신으로 회피
