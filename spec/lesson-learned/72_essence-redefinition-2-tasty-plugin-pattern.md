---
created: 2026-06-04
phase: 본질 재정의 2차 (Phase 200 시리즈 진입 전)
prd_truth: prd/research/plugin-architecture-2026-06-04.md
external_source: C:\dev\claude_workspaces\tasty (v0.6.0, api_version 1)
meta_rules:
  - 메타 룰 20 (본질 도메인 일치 외부) — 8건째
  - 메타 룰 22 (사용자 정책 경계 합의) — 10건째
  - 메타 룰 30 후보 → 정식 승격 의무
  - 메타 룰 19 (단일 진실원 위임) — search-extraction-plan / mydocsearch_decision 무효화
related:
  - 48_session-2026-05-20-meta-patterns.md (1차 본질 재정의 시점의 메타 패턴)
  - 49_spec-self-duplication-single-source.md (단일 진실원 패턴)
---

# Lesson 72 — 본질 재정의 2차: tasty 패턴 흡수 (host = 파일 가공만 / 외부 모두 plugin)

## 상황

본 세션에서 누적된 외부 솔루션 흡수 (Phase 87~107 + Phase A/B/E) 7건이 사용자 표면적 비대를 초래. competitive-analysis.md 갱신 시점에 **MCP 28 / Tauri commands 67 / UI 6탭 + 서브카드 다수** 등 정량 확인.

사용자 결정: **D안 (검색 분리) 진입** → 코드 진입 직전 사용자 추가 결정 **"tasty처럼 원본 기능 + plugin 형태로 구현"** → 검색 분리는 부분집합으로 자연 흡수.

추가 4축 사용자 합의 (메타 룰 22 10건째):
1. host 경계 = 파일 가공 최소 (입력 + 구조화 + 저장 + audit 코어)
2. search-extraction-plan.md 처리 = 폐기 + deprecated.md
3. 진입 범위 = spec/prd 재설계 문서만 (코드 0)
4. plugin 분류 = 28 MCP + 23 어댑터 → 11 + 24 = 35 plugin

## 문제

**기능 과다 = 흡수 정책의 한계 신호**:
- lesson 30 "인프라 선구현 + 디폴트 비활성"이 무한 누적 차단 메커니즘 부재
- 메타 룰 1 sub-rule 1f (단일 진입점)는 진입점 통일을 잘 했지만 **진입점 수 자체는 줄지 않음**
- 메타 룰 22 (사용자 정책 합의)는 흡수 합의만 받고 **정리 합의는 시작 안 됨**
- 사용자 인지 부담 + 다중 위치 동기화 비용 + 첫 진입 학습 곡선 누적

## 원인

3 영역 누적:
1. **외부 흡수 7건 (Phase 87~103)** — wikidocs + JAMES + Mirage + GraphRAG + Adaptive + Grimoire + tasty 8건째
2. **lesson 30 자연 누적** — 모든 흡수가 인프라로 잔류, 활성화 없이 표면적 ↑
3. **메타 룰 13 4단계** — UI 노출 도달하려면 카드/탭 증가 필수

## 개선

### 본질 재정의 2차 결정 사실

| 영역 | 1차 (2026-06-01) | **2차 (2026-06-04)** |
|------|-------------------|----------------------|
| host 잔류 | 가공 + 추천 + 검증 (3 도메인) | **파일 가공만 (1 도메인)** |
| 외부 분리 | 검색 + KG + 임베딩 + 리랭커 + Topic | **모든 비-host 기능 plugin화** |
| 위치 | `_rust_module/` (정적 모듈) | **별도 프로세스 plugin + IPC** (tasty 패턴) |
| 단위 | Phase 108~115 | **Phase 200~209** |

### 단일 진실원 갱신

| 위치 | 역할 |
|------|------|
| `prd/research/plugin-architecture-2026-06-04.md` | **본 결정 단일 진실원 신규** |
| `spec/architecture.md` | 최상단 §2026-06-04 + §1차 무효화 표시 |
| `spec/domain-map.md` | host-plugin 매핑 + §1차 무효화 |
| `spec/deprecated.md` | search-extraction-plan + mydocsearch_decision 무효화 등재 |
| `prd/research/search-extraction-plan.md` | 무효화 헤더 + 자료 보존 |

### 신규 plugin 분류 (11 MCP + 24 어댑터)

| 카테고리 | plugin 수 |
|----------|----------|
| MCP 도구 | 11 plugin (search/kg/lint/setup/optimize/signal/todo/llm-cache/pii/c1-thresholds/grimoire) |
| 어댑터 | 24 plugin (embedding 6 / llm 7 / storage 5 / notify 2 / rerank 3 / verify 1) |
| **합계 workspace 멤버** | **35 + host** (tasty 20+ 선례) |

### Phase 200~209 단계

- 200: protocol + sdk placeholder (lesson 16 단계 0)
- 201: PluginRegistry + 매니페스트 + permission gate
- 202: IPC bus + wire 프로토콜 + audit 통합
- 203: fp-plugin-search (검색 자연 흡수 = 옵션 D 대체)
- 204~207: 나머지 plugin
- 208: GUI Plugins 탭 (사용자 표면적 직접 제어)
- 209: 회귀 게이트 + bench + release 재빌드 + D:\file-test 재배포

## 메타 룰 적용

| 메타 룰 | 적용 |
|---------|------|
| 20 (본질 도메인 일치 외부) | **8건째** (tasty 직접 흡수, 본질 100% 일치) |
| 22 (사용자 정책 합의) | **10건째** (4축 합의: host 경계 / plan 처리 / 진입 범위 / 폐기) |
| 19 (단일 진실원 위임) | search-extraction-plan + mydocsearch_decision 양쪽 위임 |
| 30 후보 → 정식 승격 의무 | 본 세션 자기 적용 4건째 (CLAUDE.md + architecture + domain-map + deprecated 동시 갱신) |
| 21 → 약화 | 1차 본질 재정의 시 메타 룰 21 적용된 검색 분리가 본 결정으로 흡수 = 메타 룰 20이 21을 흡수 |

## 메타 가치

본 결정의 본질 메타 패턴:

1. **흡수 정책의 진화** — "코드 흡수 + 인프라 잔류" → "plugin 발행"으로 단위 변경
2. **외부 흡수 무한 누적 차단 메커니즘 도입** — host 영향 0 + 사용자 on/off
3. **메타 룰 1 sub-rule 1f 자연 해소** — plugin 단위로 진입점 자연 분산
4. **사용자 표면적 직접 제어** — tasty 패턴 핵심 가치 흡수
5. **lesson 30 자연 진화** — 인프라 비활성 → plugin 비활성 = 더 명확한 사용자 선택

## 다음 세션 진입 의무

1. Phase 200 placeholder 진입 (코드 0건 빌드)
2. 메타 룰 30 정식 승격 (4건 누적 도달)
3. 메타 룰 17 강화 정식 승격 검토 (3건 누적 도달)
4. release 재빌드 의무는 본 세션 코드 변경 없으므로 미발생 — 다음 코드 phase 시 발생
