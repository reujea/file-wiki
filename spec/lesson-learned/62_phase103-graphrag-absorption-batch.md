# Lesson Learned: Phase 103 GraphRAG 흡수 4건 묶음 (G1/G2/G3/G4) + 메타 룰 21 정식 승격

## 상황

2026-05-27 Phase 103. 사용자 명시 트리거: "Q2 진행해. 흡수 가치 종합 목록 전체 구현해" (직전 GraphRAG 분석 결과). AWS GraphRAG Toolkit (Apache-2.0, 엔터프라이즈 RAG)에서 4건 흡수 후보 식별 + 메타 룰 21 후보 3건 누적 도달 (TFM + Mirage + 본 분석).

본 phase는:
- 단일 진실원 외부 분석 작성 (메타 룰 9)
- 메타 룰 21 정식 승격 (메타 룰 23 §승격 3요소 충족)
- G4 즉시 구현 + G1/G2/G3 인프라 선구현 (lesson 30 패턴)

## 문제

### 문제 1 — GraphRAG 흡수 가치 평가 + 도메인 정렬

AWS GraphRAG Toolkit은 엔터프라이즈 RAG (Neptune + OpenSearch + LlamaIndex + Bedrock) — 본 프로젝트 단일 사용자 데스크톱과 본질 불일치. 메타 룰 20 §🔴 보류 정책 충돌 회피 필요. 그러나 부수 알고리즘(Statement / 의미 관계 / Multi-hop / TF-IDF) 흡수 가치 ↑.

### 문제 2 — 메타 룰 21 후보 3건 누적 도달 미적용

메타 룰 21 후보(본질/부수 도메인 분리)는 lesson 50/51에서 2건 누적 + Phase 99 META.md 본문 등재 완료. **3건째 누적 도달 시 메타 룰 23 §승격 3요소 충족** — 본 phase가 3건째 사례.

### 문제 3 — 구조체 필드 추가 시 단위 테스트 누락 (메타 룰 1 sub-rule 1b 적중)

G1 Metadata.statements 필드 추가 시 lib 단위 테스트(`test_metadata_serde_roundtrip`, line 737) 명시 초기화 1건 누락. E0063 빌드 에러 1건. **메타 룰 1 sub-rule 1b 적중** — `cargo check --workspace`만으로 부족, `cargo test --lib`이 잡아냄.

### 문제 4 — McpState 다중 인스턴스 생성처 갱신 의무 (lesson 15/26 패턴)

McpState 인스턴스 생성처 3곳 (cli/main.rs + shared/cli.rs + make_mcp_state 테스트) — Phase 103 신규 필드 tfidf_rerank_enabled + kg_beam_search 추가 시 3곳 모두 갱신 필수. **메타 룰 1 sub-rule 1f (함수/생성처 다중 정의) 적중**.

## 원인

### 직접 원인
1. (문제 1) 외부 도메인 도구는 부수 영역 흡수 후 본질은 보류해야 함 (메타 룰 20). GraphRAG는 본질 도메인 다르지만 알고리즘은 도메인 무관
2. (문제 2) 메타 룰 23 정식 승격(Phase 99) 후 다른 후보(21 등) 자기 적용 자동 트리거 부재 — 사용자 명시 합의 후 진행
3. (문제 3) 메타 룰 1 sub-rule 1b 체크리스트(`cargo build --tests --workspace`) 사전 실행 누락. `cargo check`만 실행 → lib 테스트 명시 초기화 누락 미발견
4. (문제 4) 메타 룰 1 sub-rule 1f McpState 생성처 grep 사전 실행 ✅ (이번에는 회피, 3곳 모두 동기 갱신)

### 구조적 원인
- 외부 프로젝트 흡수 패턴 표준화: 메타 룰 21 정식 승격으로 다음 흡수 시 의사결정 비용 ↓
- 구조체 필드 추가 시 lib test 검증 자동화 부재 (메타 룰 1 sub-rule 1b 자동화 도구 후보)
- McpState 생성처 다중성은 메타 룰 19 (단일 진실원) 위반 후보 — `make_mcp_state_from_config(cfg)` 같은 팩토리 함수 도입 검토 (lesson 30 후속)

## 개선

### 즉시 적용 (본 Phase 103 완료)

#### 메타 작업 2건
- ✅ `prd/research/external-analysis-2026-05-27-graphrag.md` 신규 단일 진실원 (메타 룰 9 자기 적용)
  - 메타데이터 / 기술 스택 / 본 프로젝트 비교 / 흡수 후보 평가 / 메타 룰 적용 / 본 프로젝트 우위 / 트리거 매핑
- ✅ `spec/lesson-learned/META.md` 메타 룰 21 **정식 승격** (3건 누적: TFM + Mirage + GraphRAG)
  - 메타 룰 20과 차이 (본질 vs 부수만)
  - 누적 사례 표
  - 신규 작업 사전 체크리스트 5건

#### G1 Statement 노드 인프라 (lesson 30 패턴)
- ✅ `crates/core/src/domain/models.rs::Metadata.statements: Vec<String>` 신규 (디폴트 빈 Vec)
- ✅ 트리거: 가공 50파일+ + needs_verification 누적 5건+ 도달 시 LLM 프롬프트 활성화
- ✅ 단위 테스트 명시 초기화 1건 갱신 (E0063 즉시 해소)

#### G2 의미 관계 LLM 추출 인프라 (lesson 30 패턴)
- ✅ `crates/core/src/domain/models.rs::RelationType::Semantic(String)` variant 신규
- ✅ Display impl 추가 (`semantic:{verb}` 형식)
- ✅ 트리거: KG 관계 평균 <2 + 도메인 다양성 확보 + LLM 프롬프트 semantic_relations 활성화 시
- ✅ 디폴트 미사용 (LLM 프롬프트 미활성으로 자동 회피)

#### G3 Multi-hop 빔 검색 (A2 KG hop 확장)
- ✅ `crates/shared/src/config.rs::SearchConfig.kg_beam_search: bool` 신규 (디폴트 false)
- ✅ `crates/shared/src/mcp_server.rs::McpState.kg_beam_search: bool` 신규
- ✅ handle_search A2 KG 1-hop 확장 위치에 빔 검색 분기 — kg_beam_search=true + expand_kg_hops>0 시 시드 점수 상위 beam_width만 확장
- ✅ McpState 인스턴스 생성처 3곳 동기 갱신 (cli/main.rs / shared/cli.rs / make_mcp_state)

#### G4 TF-IDF 다양성 재순위
- ✅ `crates/shared/src/config.rs::SearchConfig.tfidf_rerank_enabled: bool` 신규 (디폴트 false)
- ✅ `crates/shared/src/mcp_server.rs::McpState.tfidf_rerank_enabled: bool` 신규
- ✅ handle_search 후처리 단계 (다양성 강화 직후, 캐시 저장 직전) TF-IDF 분기
- ✅ 알고리즘: 본문 100줄 read_header → 토큰 추출 → 상위 head_len 토큰 합집합 → top_k 범위 밖 결과 중 신규 토큰 비율 ≥50% 결과 promote
- ✅ McpState 인스턴스 생성처 3곳 동기 갱신

### 빌드 + 회귀 검증

- ✅ workspace cargo check 통과 (1m 20s, 0 경고)
- ✅ workspace lib 테스트 **383 통과 / 0 실패** (core 169 + adapters 104 + shared 110, 회귀 0)
- ✅ Tauri release 통과 (14m 56s incremental)
- ✅ audit_stage_check PASS (G3/G4는 audit.record 미사용 — stage 변동 없음)
- ✅ release_rebuild_required.sh 자동 판정 의무 적용
- ✅ D:\file-test\pipeline.exe 재배포 SHA-256 일치

### 메타 룰 적용 결과

| 룰 | 적용 |
|----|------|
| 메타 룰 8 (사전 grep) | ✅ RelationType + Metadata + SearchConfig + McpState 사전 grep |
| 메타 룰 9 (외부 문서 권고 3단계) | ✅ 단일 진실원 작성 완료 |
| 메타 룰 1 sub-rule 1b | ⚠️ E0063 1건 (Metadata 단위 테스트 초기화 누락) — 즉시 해소 |
| 메타 룰 1 sub-rule 1f | ✅ McpState 생성처 3곳 동기 갱신 (사전 grep 회피 성공) |
| 메타 룰 5 강화 (트리거 인프라 3요소) | ✅ G1~G4 모두 디폴트 비활성 + 분기 완성 + no-op 안전 |
| 메타 룰 13 4단계 | G4 단계 1-2 도달 / G1/G2/G3 단계 1 (인프라만) |
| 메타 룰 16 차원 B | ✅ GraphRAG 라벨 🟢/🟡/🔴 부착 |
| 메타 룰 17 (release 재빌드) | ✅ 자동 판정 + 의무 이행 |
| 메타 룰 19 (단일 진실원) | external-analysis-2026-05-27-graphrag.md 단일 위치 |
| 메타 룰 20 (도메인 정렬) | ✅ 본질 불일치 식별 + 부수만 흡수 |
| **메타 룰 21 (본질/부수 분리)** | ✅ **Phase 103 정식 승격** (누적 3건 도달) |
| 메타 룰 22 (사용자 정책 합의) | ✅ 사용자 명시 "전체 구현" 합의 (4건째 누적, 1건 추가로 정식 승격 가능) |
| 메타 룰 23 (승격 기준) | ✅ 메타 룰 21에 자기 적용 |
| 메타 룰 25 (자기 적용 의무) | ✅ 메타 룰 21 정식 직후 외부 분석 + 본 lesson에 즉시 자기 적용 |
| 메타 룰 26 (match 스코프) | ✅ handle_search match 분기 사전 검증 |

### 트리거 등록 (Phase 103 신규)

| ID | 트리거 | 조건 |
|----|--------|------|
| #G1 | Statement 노드 활성화 | 가공 50파일+ + needs_verification 누적 5건+ |
| #G2 | 의미 관계 활성화 | KG 관계 평균 <2 + LLM 프롬프트 semantic_relations 활성 |
| #G3 | Multi-hop 빔 검색 | A2 활성화(expand_kg_hops>0) + 실 사용자 만족도 신호 |
| #G4 | TF-IDF 디폴트 활성화 | 사용자 검색 30회+ + MRR before/after 측정 |

## 다음 세션 플래그

- 사용자 GUI 재실행 + Claude Code MCP 등록 후 `optimize` (Phase 102) + 검색 30회+ 누적
- G4 측정 후 디폴트 활성화 검토 (트리거 #G4)
- G1 statements 활성화 — LLM prompts.toml에 `statements` 필드 추가 시점 (트리거 #G1)
- 메타 룰 22 후보 4건째 누적 도달 (Phase 100 IA + Phase 99 헥사고날 + Phase 92 외부 협업 + 본 phase) → 정식 승격 임계
- 메타 룰 1 sub-rule 1b 자동화 후보: `cargo build --tests --workspace` 회귀 게이트 추가

## 회귀 기준선

| 지표 | Phase 102 | Phase 103 | 차이 |
|------|---------|---------|------|
| MCP 도구 | 25 | 25 (변동 없음) | 0 |
| handle_search 코드 | (기존) | **+44줄** (G3 빔 4줄 + G4 TF-IDF 40줄) | +44 |
| Metadata 필드 | 14 | **15** (+ statements) | +1 |
| RelationType variant | 5 | **6** (+ Semantic) | +1 |
| SearchConfig 필드 | 9 | **11** (+ tfidf_rerank_enabled, kg_beam_search) | +2 |
| McpState 필드 | (기존) | **+2** | +2 |
| McpState 생성처 갱신 | (필요 시) | **3곳 동기 갱신** ✅ | 메타 룰 1 1f |
| workspace lib 테스트 | 383 | **383** | 회귀 0 |
| pipeline.exe (CLI release) | 17.99 MB | (재빌드 미수행 — UI 미변경) | — |
| file-pipeline-tauri.exe (GUI release) | 21.08 MB | **21.09 MB** (+9 KB) | +9 KB |
| Tauri release 빌드 시간 | 15m | 14m 56s | -0.4% |
| 정식 메타 룰 | 23 (1~20+23/25/26) | **24** (+ 21 정식 승격) | +1 |
| 후보 메타 룰 | 5 (21/22/24/27/28) | **4** (22/24/27/28, -21) | -1 |
| 외부 프로젝트 분석 단일 진실원 | 4건 (Ruflo + JAMES x2 + Mirage) | **5건** (+ GraphRAG) | +1 |
| 추정 빗나감 누적 | 9 (Phase 102) | 9 (변동 없음) | 0 |

## 사이드 발견

- `SetupAdvice` 구조체는 `Serialize` 파생 — JSON 변환 불필요 (Phase 102 추정 빗나감 해소 후 본 phase 동일 패턴 재발 회피)
- `RelationType::Semantic(String)`은 String 페이로드라 기존 `#[derive(Default)]` 호환 (#[default] = RelatedTopic 유지)
- McpState 생성처 3곳 grep 사전 실행 + 동기 갱신으로 메타 룰 1 sub-rule 1f 적중 회피 (Phase 103 첫 사례)
- TF-IDF 알고리즘은 본문 read_header 의존 — storage I/O 비용 ↑. 트리거 #G4 도달 시 캐시 도입 검토
- G3 빔 검색은 expand_kg_hops를 빔 폭으로 재사용 — 별도 config 필드 불필요 (lesson 30 인프라 단순화)
