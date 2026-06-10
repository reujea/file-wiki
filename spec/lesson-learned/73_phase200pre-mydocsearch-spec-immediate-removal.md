---
created: 2026-06-05
phase: Phase 200 시리즈 진입 전 (spec 분석 트리거)
prd_truth: prd/research/plugin-architecture-2026-06-04.md
meta_rules:
  - 메타 룰 30 (spec 본문 phase별 즉시 갱신) — 자기 적용 6건째
  - 메타 룰 22 (사용자 정책 경계 합의) — 11건째
  - 메타 룰 19 (단일 진실원 위임) — 8건째 (lesson 49 / 50-A / 50-B / 51 / 52 / 55 / Phase 100 / 본 lesson)
  - 메타 룰 12 (잔존 종결 의무) — stale 잔존 차단 결합
related:
  - 49_spec-self-duplication-single-source.md (옵션 A — 단일 진실원 위임 원형)
  - 72_essence-redefinition-2-tasty-plugin-pattern.md (본질 재정의 2차로 mydocsearch 결정 무효화 직접 트리거)
  - 64_phase105-project-sync-batch.md (메타 룰 30 발견 시점)
---

# Lesson 73 — "Phase N 진입 시 처리" 표시 spec의 즉시 처리 패턴 (메타 룰 22+19 결합)

## 상황

사용자 트리거 "spec 폴더 분석해" 수행 중 발견 — `spec/mydocsearch_decision.md`가 `status: superseded` + 본문 헤더 "Phase 115 종결 시 deprecated.md 이관" 상태로 잔존. 시계열:

- 2026-04-08: "MyDocSearch 통합 불필요 / LocalVectorStore 단일 구조" 결정
- 2026-06-01: 본질 재정의 1차 → "Phase 115 종결 시 deprecated.md 이관" 표시 (search-extraction-plan에 위임)
- 2026-06-04: 본질 재정의 2차 → search-extraction-plan 자체 무효화 + "Phase 203 진입 시 본 spec 삭제 또는 무효 명시" 표시 (deprecated.md 엔트리)
- **2026-06-05 (본 lesson)**: 사용자 합의로 Phase 203 대기 없이 **즉시 삭제 + 결정 사실 deprecated.md 흡수**

분석 보고 Q1로 "지금 이관할까 / Phase 200 진입까지 보존"을 제시 → 사용자 Q1 적용 → 즉시 처리 진행.

## 문제

"Phase N 진입 시 처리"라고 표시한 spec 잔존물의 누적 stale 위험:

1. **이중 무효화 누적** — mydocsearch_decision은 1차(2026-06-01) + 2차(2026-06-04) 두 번의 무효화를 겪었으나 spec 파일 자체는 잔존. 다음 무효화가 또 발생할 수 있음
2. **다음 phase 진입 의존** — Phase 203은 plugin-architecture 진입의 중간 단계. 한 phase가 지연되면 spec 정리도 동반 지연
3. **단일 진실원 위반** — 결정 사실이 mydocsearch_decision.md 본문 + deprecated.md 엔트리 + architecture.md 본문 인용 = 3곳 분산. 메타 룰 19 자기 위반 잔존
4. **메타 룰 12 잔존 표기** — "Phase X 종결 시 ..." 표기 자체가 stale 위험 (lesson 36→85 패턴 반복)

## 원인

phase 진입 의존 처리는 **사용자 도메인 흐름 (코드 변경 phase)** 기준의 처리 트리거. 그러나 spec 정리는 다음 둘 중 하나로도 충분:

- **사용자 트리거 의존** — "spec 분석해" / "현행화" 같은 명시 신호
- **자체 phase 독립** — 메타 룰 22(사용자 합의) + 메타 룰 19(단일 진실원) 결합으로 코드 phase와 분리 처리 가능

mydocsearch_decision은 본문 결론·근거 4건·흡수 완료 항목이 모두 다른 곳(CLAUDE.md / prd/features/ / deprecated.md)에 보존되어 있어 **원본 파일 보존 가치 0** 상태였음. 그럼에도 "Phase 203 대기"라 표시한 것은 lesson 30 패턴("실 측정 도달 후 활성화")을 무의식 차용한 결과.

## 개선

### 4건 동시 처리 (2026-06-05)

| 작업 | 파일 | 변경 |
|------|------|------|
| ① 결정 사실 흡수 | `spec/deprecated.md` | mydocsearch 엔트리 확장 — 원래 결정·근거 4건·흡수 완료 항목·복구 방법·관련 lesson 명시 (단일 진실원 도달) |
| ② 본문 인용 위임 | `spec/architecture.md` §MyDocSearch 비교 결론 (2265줄) | "통합 불필요…상세: mydocsearch_decision.md" → deprecated.md 단일 진실원 위임 + 무효화 사유 명시 |
| ③ 변경 요약 stale 해소 | `spec/architecture.md` 23·53줄 | "무효화 유지/예정" → "삭제 완료 (2026-06-05)" 갱신 |
| ④ 원본 삭제 | `spec/mydocsearch_decision.md` | 파일 제거 — spec 본문 8→7개 |

### 패턴 일반화 — "Phase N 진입 시 처리" 표시 spec의 즉시 처리 조건

다음 3 요소 **모두** 충족 시 다음 phase 대기 없이 즉시 처리 가능 (메타 룰 22 + 19 결합):

1. **결정 사실 보존 위치 명확** — 원본 본문이 다른 진실원(deprecated.md / lesson / prd/research/)에 흡수 가능
2. **사용자 합의 가능** — 코드 변경 없는 spec 정리는 사용자 트리거 (분석 보고 Q 형식 등)로 합의 도달 가능
3. **메타 룰 19 단일 진실원 위임 가능** — 결정 사실 = 진실원 1곳, 결정 맥락 = 참조원 N곳, 단방향 링크 위임 가능

체크리스트:
- [ ] `spec/` 본문 grep으로 "Phase N 종결 시" / "Phase N 진입 시" / "무효화 예정" / "이관 예정" 표기 정기 점검
- [ ] 발견 시 위 3 요소 충족 여부 평가 → 충족 시 즉시 처리 Q 형식 제시
- [ ] 처리 결정 시 4건 동시 처리 패턴 (deprecated.md 흡수 + 본문 인용 위임 + 변경 요약 stale 해소 + 원본 삭제) 적용
- [ ] 본 작업 자체를 lesson 73 패턴 사례로 등재 (메타 룰 25 자기 적용 의무)

### 적용된 메타 룰 (자기 적용 사례)

| 메타 룰 | 사례 | 위치 |
|--------|------|------|
| 30 (spec 즉시 갱신) | 6건째 자기 적용 — mydocsearch 삭제 + architecture.md 3건 동시 갱신 + deprecated.md 흡수 | META 누적 사례 +1 |
| 22 (사용자 정책 합의) | 11건째 — spec 분석 Q1 형식으로 사용자 결정 도출 | META 누적 사례 +1 |
| 19 (단일 진실원 위임) | 8건째 — mydocsearch 결정 사실의 deprecated.md 단일 진실원 도달 | META 누적 사례 +1 |
| 12 (잔존 종결 의무) | "Phase 203 대기" 표기 종결 (deprecated.md 엔트리 + architecture.md 53줄) | 본 lesson 본문 |
| 25 (메타 룰 자기 적용) | 본 lesson 작성 자체가 메타 룰 30 자기 적용 사례 등재 | 본 lesson 본문 |

## 측정

| 지표 | 변경 |
|------|------|
| spec 본문 파일 수 | 8 → **7** |
| 진실원 분산 | 3곳 (mydocsearch + deprecated + architecture) → **1곳** (deprecated.md) |
| `spec/architecture.md` 인용 위치 | 3건 (23·53·2265줄) → 모두 deprecated.md 단일 위임 |
| 사용자 합의 ↔ 처리 종결 간 시간 | 사용자 결정 "Q1 적용" 즉시 → 4건 동시 처리 1 메시지 |

## 후속 점검 후보

- [ ] `webapp-design.md` 헤더(2026-06-01 stale) — 본질 재정의 2차 미반영
- [ ] `architecture-archive.md` ↔ `deprecated.md` 중복 추가 발굴 (lesson 49 옵션 A 재검토)
- [ ] 메타 룰 30 정식 승격 후 후보 본문 잔존 검증 (Phase 본질 재정의 2차 직후)

## 본 lesson의 메타 가치

- **"Phase N 대기" ≠ "처리 불가"** — 코드 변경 phase 대기와 spec 정리는 분리 가능 결정
- **사용자 합의 트리거의 비대칭 비용** — Q1 응답 1줄로 4건 처리 완료. 처리 비용 < 대기 누적 비용
- **메타 룰 22 + 19 결합 패턴 정형화** — 본 lesson을 양 메타 룰의 결합 사용 첫 사례로 등재
