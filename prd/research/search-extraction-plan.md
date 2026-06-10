---
created: 2026-06-01
status: ⚠️ **무효화 (2026-06-04)** — tasty 패턴 흡수 결정이 상위. 본문은 fp-plugin-search 진입 (Phase 203) 시 참고 자료로 보존
superseded_by: prd/research/plugin-architecture-2026-06-04.md
deprecated_at: spec/deprecated.md (§무효화됨 (계획 폐기))
purpose: (무효화) file-pipeline 본질 재정의 — 검색 도메인을 외부 라이브러리로 분리하여 가공+추천+검증 전담 프로젝트로 책임 축소. → 2026-06-04 tasty 패턴 흡수로 자연 흡수
related: spec/lesson-learned/META.md (메타 룰 16/20/21/22), spec/lesson-learned/16_module-workspace-extraction.md (Phase 60 분리 패턴), spec/lesson-learned/17_module-api-impl-dependency-leak-check.md (의존 누수 점검), spec/mydocsearch_decision.md (2026-04-08 결정 재고 대상)
---

> ⚠️ **본 문서는 2026-06-04에 무효화되었습니다.**
> 상위 결정: `prd/research/plugin-architecture-2026-06-04.md` (host = 파일 가공만 / 모든 외부 기능 plugin / tasty 패턴 직접 흡수).
> 검색 분리는 fp-plugin-search (Phase 203) 하나로 자연 흡수. 본문의 §1.3 MCP 36 정밀 분류 / §3 8 논점 분석 / §5 단계 분할 / §6 위험 매트릭스는 Phase 203 진입 시 참고 자료로 보존.

# 검색 도메인 분리 Plan (검색·저장·알림 → `_rust_module/`) — 무효화

## 0. 사용자 합의 사항 (메타 룰 22 7건째 누적)

| 항목 | 결정 |
|------|------|
| 분리 동기 | (1) 본 프로젝트 책임 축소 (2) 아키텍처 위생 |
| 본질 재정의 | file-pipeline = **가공 + 추천 + 검증** 전담. 검색은 외부 라이브러리 |
| 분리 위치 | `C:\dev\claude_workspaces\_rust_module\` (기존 `module/`과 별도 신규 워크스페이스) |
| 진행 단위 | Plan 문서 작성 후 결정 (lesson 16 단계 분할 후보) |
| 검색 결정 | 본질에서 제외, 외부 위임 (이전 mydocsearch_decision.md "LocalVectorStore 단일" 결정 무효화) |

본 plan은 사용자 합의 후 Phase 108~ 실행 단위 결정의 입력.

---

## 1. 현황 측정 (사실)

### 1.1 검색 측 코드량

| 영역 | 파일 | 줄수 |
|------|------|------|
| LocalVectorStore | `crates/adapters/driven/vector_db/local_store.rs` | **1,079** |
| Embedding 6 어댑터 | `crates/adapters/driven/embedding/*.rs` | **813** (Claude 243 + FastEmbed 83 + Sparse 129 + Local 143 + OpenAI 84 + PythonOnnx 131) |
| Reranker 3 어댑터 | `crates/adapters/driven/reranking/*.rs` | **329** (FastEmbed 125 + Claude 157 + Null 47) |
| **검색 어댑터 합계** | | **2,221** |
| CrossRef (core 도메인) | `crates/core/domain/cross_reference.rs` + `crossref_optimizer.rs` | **626** |
| Wikilink + Wiki Export | `crates/core/domain/wikilink.rs` + `wiki_export.rs` | **657** |
| Topic Merger | `crates/core/domain/topic_merger.rs` | **490** |
| MMR + vec_io | `crates/core/domain/mmr.rs` + `vec_io.rs` | **214** |
| **검색 의존 core 도메인 합계** | | **1,987** |

**총 분리 후보**: **약 4,200줄** (CrossRef/Wikilink/Topic은 분류 논점, 아래 §3.1 참조)

### 1.2 결합 지점 (core/src에서 검색 포트 참조)

```
service.rs           7건  ← 가공 14단계 step 8/13 + 검색 진입점
core/domain/auto_reindexer.rs    2건
core/reasoning/verifier.rs        1건
core/domain/cross_reference.rs    4건  ← CrossRef
core/ports/output.rs              3건  ← VectorDBPort/EmbeddingPort/RerankerPort trait 정의
core/domain/diagnostics.rs        2건
core/domain/lint.rs               5건  ← lint_strong_claims(vector_db, ...)
core/domain/topic_merger.rs       7건
core/domain/wiki_export.rs        6건

총 37건의 core/src 참조
```

### 1.3 MCP 도구 카탈로그 (실측 36개 — 정밀 분류 완료)

> 사이드 발견: spec/architecture.md "11 도구" / spec/webapp-design.md "25 도구" / mcp_server.rs 실측 **36 도구**. 차이 25/11건. Phase 102 이후 카탈로그 변동 분이 spec 본문에 누적 미반영. 메타 룰 1 sub-rule 1f 자기 적용 트리거 (본 plan 단계 7에서 일괄 정리).

| # | 도구명 | 분류 | 분리 후 위치 |
|---|--------|------|------------|
| 1 | search | 검색 | 외부 |
| 2 | get_document | 검색 | 외부 |
| 3 | list_documents | 검색 | 외부 |
| 4 | stats | 검색 | 외부 (vector_db 통계) |
| 5 | lint | 검증 | **잔류 (§3.7)** |
| 6 | revise_topic | 가공 (topic 편집) | 잔류 |
| 7 | kg_neighbors | KG | 외부 |
| 8 | kg_paths | KG | 외부 |
| 9 | kg_stats | KG | 외부 |
| 10 | list_todos | Todo | 잔류 |
| 11 | complete_todo | Todo | 잔류 |
| 12 | optimize | 통합 메타 (Phase 102) | 잔류 |
| 13 | setup_review | 추천 | 잔류 |
| 14 | setup_apply | 추천 | 잔류 |
| 15 | setup_snapshot_list | 추천 | 잔류 |
| 16 | setup_snapshot_rollback | 추천 | 잔류 |
| 17 | setup_snapshot_measure | 추천 | 잔류 |
| 18 | setup_decision_log_list | 추천 | 잔류 |
| 19 | setup_modules_list | 추천 | 잔류 |
| 20 | setup_apply_modules | 추천 | 잔류 |
| 21 | get_search_mode_stats | 카운터 | 잔류 (§3.2) |
| 22 | get_crag_stats | 카운터 | 잔류 (§3.2) |
| 23 | get_chunk_stats | 카운터 | 잔류 |
| 24 | get_processing_metrics | 카운터 | 잔류 |
| 25 | get_llm_cache_stats | A1 캐시 | 잔류 |
| 26 | clear_llm_cache | A1 캐시 | 잔류 |
| 27 | c1_thresholds_list | C1 임계값 | 잔류 |
| 28 | c1_threshold_set | C1 임계값 | 잔류 |
| 29 | pii_patterns_list | C2 PII | 잔류 |
| 30 | pii_pattern_add | C2 PII | 잔류 |
| 31 | pii_pattern_remove | C2 PII | 잔류 |
| 32 | auto_suggest_from_counters | C1 자동 추천 | 잔류 |
| 33 | accept_suggested_decision | C1 | 잔류 |
| 34 | reject_suggested_decision | C1 | 잔류 |
| 35 | setup_dryrun | 추천 | 잔류 |
| 36 | setup_profile_infer | 추천 | 잔류 |

**최종 분류**:
- **외부 이관 8 도구**: search / get_document / list_documents / stats / kg_neighbors / kg_paths / kg_stats / (revise_topic은 가공 측 → 잔류 재분류)
- **file-pipeline 잔류 28 도구**: 추천 13 + 카운터 4 + A1 캐시 2 + C1 임계값 2 + C2 PII 3 + Todo 2 + optimize 1 + lint 1

### 1.4 진입점 영향

| 진입점 | 영향 |
|--------|------|
| **modals/cli** | `search` 서브커맨드 + `kg` 서브커맨드(Neighbors/Paths/Stats) + `golden` (검색 골든셋) — 외부 라이브러리 호출 형태로 위임 |
| **modals/app (Tauri)** | Tauri commands 65개 중 검색·문서·KG·MCP 카탈로그 관련 ~15개 — 외부 라이브러리 호출로 변경 |
| **mcp_server (Claude Code)** | 위 §1.3 분할 |
| **Dashboard 7탭** | Documents 탭(검색·KG 시각화) 전면 외부 의존. Topics 탭(topic_merger 분리 시) 일부 영향 |

---

## 2. 메타 룰 16 차원 B 사전 라벨

각 영역별 분리 가능성 라벨:

| 영역 | 라벨 | 사유 |
|------|------|------|
| LocalVectorStore | 🟢 | mmap+HNSW+sparse 자체 완결. file-pipeline 도메인 결합 0 (이미 trait impl) |
| EmbeddingPort 어댑터 6종 | 🟢 | 단순 텍스트→벡터 변환. 도메인 의존 없음 |
| RerankerPort 어댑터 3종 | 🟢 | 단순 점수 재산정. 도메인 의존 없음 |
| VectorDBPort / EmbeddingPort / RerankerPort trait 자체 | 🟡 | core/ports/output.rs에 정의됨. trait는 외부 API 크레이트로 이관 후 core가 의존 |
| CrossRef (Phase 52) | 🟡 | 도메인 모델(DocRelation/RelationOrigin 5종)에 의존. **소속 결정 논점 (§3.1)** |
| Wikilink 추출 | 🟡 | Metadata에 wikilinks 필드 저장 (가공의 출력). 위치 논점 |
| Wiki Export | 🟡 | obsidian markdown export — 검색 측 부수 기능 |
| Topic Merger | 🟡 | 토픽 자동 병합 — 검색 후 분석. 위치 논점 |
| MMR / vec_io | 🟢 | 순수 알고리즘. 검색 모듈로 이관 |
| **KG (kg_neighbors/paths/stats)** | 🟡 | LocalVectorStore find_related 기반. 검색에 종속 |

🔴 없음 — 분리 시 추상화 불일치는 발견되지 않음. 그러나 🟡 6건은 소속 결정 합의 필요.

---

## 3. 핵심 논점 (사용자 결정 필요)

### 3.1 CrossRef 소속 결정 (가장 어려운 논점)

**현황**: `crates/core/domain/cross_reference.rs` (369줄) + `crossref_optimizer.rs` (257줄). MinHash + 메타 블로킹 (Phase 52~59).

**옵션**:
- **A. 검색 측**: KG 생성이 검색 입력이므로 외부. service.rs flush_crossref → 외부 라이브러리 호출
- **B. 가공 측**: CrossRef는 가공 14단계 step 14(후처리)에 위치하므로 file-pipeline 잔류. 결과만 외부 검색 모듈에 upsert
- **C. 별도 모듈**: cross-reference 자체를 또 다른 외부 모듈로 분리 (검색·가공 양쪽이 의존)

**claude 추정**: **B**. CrossRef는 가공의 후처리이고 RelationOrigin 5종이 도메인 타입. 외부로 빼면 도메인 누수.

### 3.2 추천 카운터 소속 (search_mode_counters / crag_counters)

**현황**: settings.db에 카운터. 검색 호출 시 카운터 증가, C1(추천)에서 분포 분석.

**옵션**:
- **A. file-pipeline 잔류**: 추천 입력이라 본질. 외부 검색 라이브러리가 카운터 트리거를 file-pipeline 콜백으로 호출
- **B. 외부 잔류**: 카운터 영속화도 검색 영역. file-pipeline은 카운터 조회만

**claude 추정**: **A**. C1 자기학습이 file-pipeline 본질. 외부는 콜백 호출만.

### 3.3 EmbeddingPort 호출 시점

**현황**: 가공 14단계 step 8에서 service.rs가 직접 호출. 결과를 step 13에서 VectorDB 색인.

**옵션**:
- **A. 가공이 호출**: service.rs가 EmbeddingPort 직접 호출 → 결과를 외부 라이브러리에 upsert (포트는 외부 API 크레이트에서 import)
- **B. 외부가 호출**: service.rs는 텍스트만 외부로 전달, 외부가 임베딩+색인 일괄 처리

**claude 추정**: **B**. 임베딩+색인 일괄 처리가 결합도 낮음. 가공은 텍스트만 제공.

### 3.4 KG 모듈 위치

**현황**: kg_neighbors/paths/stats는 LocalVectorStore의 find_related 기반.

**옵션**:
- **A. 검색 모듈에 포함**: VectorDB가 KG 노드 저장 → 외부 일괄
- **B. 별도 KG 모듈**: vectordb-search ↔ kg-graph 분리. 가공이 두 모듈 호출

**claude 추정**: **A**. KG는 검색 부산물. 별도 분리는 over-engineering.

### 3.5 Topic Merger / Wiki Export 위치

**현황**: Topic Merger는 가공 후 자동 토픽 병합 (490줄). Wiki Export는 obsidian 형식 출력 (547줄).

**옵션**:
- **A. 검색 측**: 둘 다 검색 결과 후속 분석 → 외부
- **B. file-pipeline 잔류**: 둘 다 가공의 부수 출력 → 본질
- **C. 분리**: Topic은 검색, Wiki는 가공

**claude 추정**: **C**. Topic 자동 병합은 KG 분석 → 검색 측. Wiki Export는 obsidian 마크다운 출력 → file-pipeline (사용자 가공 결과 책임).

### 3.6 GUI Dashboard 7탭 처리

**현황**: Tauri Dashboard의 Documents 탭(검색·KG)이 외부 라이브러리에 의존하게 됨.

**옵션**:
- **A. GUI 분리 안 함**: file-pipeline이 Tauri GUI 보유. 외부 검색 라이브러리 결과를 GUI에서 렌더 (현재 구조 유지)
- **B. GUI도 분리**: file-pipeline은 비-GUI core 라이브러리, GUI는 별도 app 프로젝트
- **C. 검색 GUI만 분리**: Documents 탭은 외부 검색 라이브러리 own. file-pipeline은 가공/추천/검증 GUI만

**claude 추정**: **A**. GUI는 사용자 접점이라 본 프로젝트 유지. 외부 라이브러리는 비-GUI 코어만.

### 3.7 lint 도구 (`Linter::lint_strong_claims`)

**현황**: `lint_strong_claims(vector_db, storage, ...)` 가 vector_db 직접 받음.

**옵션**:
- **A. 잔류**: 검증·lint 본질. vector_db trait 호출은 외부에서 impl 받음
- **B. 외부**: lint는 검색·KG 분석의 부산물

**claude 추정**: **A**. 검증·lint는 file-pipeline 본질. trait 의존으로 충분.

### 3.8 mydocsearch_decision.md 재고

**현황**: 2026-04-08에 "MyDocSearch 통합 불필요, LocalVectorStore 단일" 결정. 본 plan으로 무효화됨.

**처리**:
- spec/mydocsearch_decision.md를 **deprecated.md로 이관 + Phase 108 결정으로 무효화 명시**
- 또는 본 plan을 **mydocsearch_decision 재정의 후속 결정**으로 본문 갱신

**claude 추정**: 이관 + 무효화 명시. spec/deprecated.md 신규 항목.

---

## 4. 분리 위치 구조 (`_rust_module/`)

```
_rust_module/
  Cargo.toml                      ← workspace, 6 멤버
  vectordb-search-api/            ← VectorDBPort + EmbeddingPort + RerankerPort trait + 도메인 타입(SearchResult, etc.)
  vectordb-search/                ← LocalVectorStore (mmap+HNSW+sparse+MinHash+CrossRef-검색 영역만)
  embedding-fastembed/            ← FastEmbed BGE-M3 + Sparse
  embedding-providers/            ← Claude/OpenAI/Local/PythonOnnx 4종
  reranker-fastembed/             ← Cross-Encoder + Claude + Null 3종
  kg-graph/                       ← KG 노드/엣지 + neighbors/paths/stats (검색 모듈에 포함 vs 별도, §3.4 결정 후 확정)
```

기존 `C:\dev\claude_workspaces\module\` (9 크레이트, form-agnostic 인프라)와 별도. `_rust_module/`은 file-pipeline 도메인 결합도가 높은 핵심 모듈 분리용.

**참고**: NotionStorageAdapter (Phase 90 ~300줄) 분리는 본 plan과 별개로 작은 trigger. 본 plan 종료 후 별도 단위로 처리 검토.

---

## 5. 분리 단위 (lesson 16 패턴 5~6 단계)

### 단계 0: Workspace placeholder (lesson 16 필수)

- `_rust_module/Cargo.toml` workspace 생성
- 모든 멤버 디렉토리에 placeholder `Cargo.toml` + `src/lib.rs` 동시 생성
- 0건 빌드 통과 검증

### 단계 1: vectordb-search-api 분리

- core/ports/output.rs의 VectorDBPort / EmbeddingPort / RerankerPort + 의존 도메인 타입(SearchResult/SimilarDoc 등) 이관
- core가 vectordb-search-api에 의존 (단방향)
- lesson 17 6단계 누수 점검

### 단계 2: LocalVectorStore + MMR + vec_io 이관

- adapters/driven/vector_db/local_store.rs (1,079줄) + mmr.rs + vec_io.rs (214줄) → vectordb-search
- core 도메인 의존(특히 CrossRef 일부) 분리 검토 — §3.1 결정에 따라
- 형제 시뮬레이션 통과 검증

### 단계 3: Embedding 어댑터 6종 이관

- FastEmbed + Sparse → embedding-fastembed
- Claude/OpenAI/Local/PythonOnnx → embedding-providers
- 가공 14단계 step 8 호출 변경 (§3.3 결정에 따라)

### 단계 4: Reranker 3종 이관

- adapters/driven/reranking/*.rs (329줄) → reranker-fastembed

### 단계 5: KG + (Topic Merger) 이관

- 검색 측 KG 로직 → kg-graph
- §3.4 결정에 따라 vectordb-search 통합 또는 별도

### 단계 6: MCP 도구 분할

- mcp_server.rs 36 도구 중 9 도구(검색·문서·KG·revise_topic 일부) 외부 이관
- file-pipeline 측은 잔류 27 도구 + 외부 라이브러리 검색 호출 래핑

### 단계 7: 진입점 + GUI + spec 정리

- modals/cli `search`/`kg` 서브커맨드 외부 호출 위임
- Tauri commands 검색·KG 관련 ~15개 외부 호출 위임
- spec/architecture.md / domain-map.md / webapp-design.md 갱신
- spec/mydocsearch_decision.md → deprecated 이관 (§3.8)
- 형제 시뮬레이션 통과 (lesson 17 6단계 누수 점검 최종 1회)

### 단계 8: 회귀 게이트 + 측정

- 통합 테스트 통과
- bench 3회 중앙값 (메타 룰 4) — 가공 처리량 회귀 0 확인
- 검색 응답 시간 측정 (외부 호출 오버헤드 영향)
- release 빌드 + D:\file-test 재배포 (메타 룰 17)

---

## 6. 위험 매트릭스

| 위험 | 영향 | 완화 |
|------|------|------|
| **core 도메인 누수**: CrossRef/Wikilink/Topic이 검색 측에 도메인 타입 누수 시 lesson 17 6단계 점검 실패 | 분리 무효화, 본 plan 후퇴 가능 | §3.1/3.5 결정 후 grep 사전 검증. anyhow/도메인 타입 0건 확인 |
| **빌드 회귀**: 1~2만 줄 이관으로 통합 테스트 다수 깨짐 | Phase 108~ 다단계 분할로 위험 분산 | lesson 16 단계 0 placeholder 우선. 각 단계 끝마다 빌드 통과 의무 |
| **추정 빗나감**: 본 plan의 §3 7개 추정이 실제와 다를 수 있음 (메타 룰 18 누적 빗나감 10건) | 분리 후 재작업 비용 | 단계 1(vectordb-search-api) 진입 직후 격리 검증 1회 (메타 룰 18 Phase 91 강화 체크리스트) |
| **mydocsearch 결정 무효화**: 2026-04-08 결정 폐기로 LocalVectorStore 본질 변동 | spec 진실원 충돌 | §3.8 처리. 무효화 사실 deprecated.md에 명시 |
| **B-1/B-2 데이터 누적 미도달**: audit_trace 0건 / processing_metrics 0건 상태에서 분리 → 분리 후 측정 차이 검증 불가 | 분리 효과 측정 불가 | 사용자 본격 가공 50파일+ 도달 후 단계 8 측정 진행. 단계 0~7은 코드 분리만 |
| **GUI Documents 탭 영향**: 외부 라이브러리 호출로 응답 시간 +α | 사용자 가시 응답 지연 | §3.6 옵션 A 유지 (in-process Rust 라이브러리 호출, IPC 아님). 오버헤드 무시 가능 추정 |
| **추천 카운터 콜백 결합**: §3.2 옵션 A 시 외부→file-pipeline 콜백 의존 | 외부 라이브러리에 도메인 콜백 trait 정의 필요 | CounterSinkPort trait를 vectordb-search-api에 정의, file-pipeline impl 주입 |

---

## 7. 측정 (메타 룰 4 / 11 / 15 결합)

### 분리 전 baseline (Phase 107 시점)

- bench 3회 중앙값: 23.62 docs/s (트리거 #11/#12 처리 시점)
- per-doc warm: 44.9s (fastembed 활성)
- workspace lib 테스트: 383~
- 통합 테스트: 69 통과

### 분리 후 측정

- 동일 bench 3회 중앙값 → 회귀 0 확인 (5% 이내)
- 검색 응답 시간 P50/P95 (외부 호출 오버헤드 영향)
- workspace lib 테스트: file-pipeline 잔류분 + _rust_module 분리분 합계가 baseline 이상

### 분리 효과 검증 (B-1/B-2 누적 후)

- 외부 모듈 단독 cargo build 시간 측정 (file-pipeline 영향 없이)
- 형제 프로젝트 1개 추가 시 _rust_module 재사용 가능 검증
- file-pipeline 책임 축소 정량화 (.rs 파일 148 → 분리 후 N, 줄수 17.2만 → M)

---

## 8. 결정 의존 매트릭스 (사용자 합의 후 확정)

| 논점 | claude 권장 | 근거 | 사용자 결정 |
|------|----------|------|------------|
| 3.1 CrossRef 소속 | **B (가공 측 잔류)** | RelationOrigin 5종(auto_similarity/user_wikilink/llm_extracted/user_manual/lint_auto_fix)이 file-pipeline 도메인 타입. 가공 14단계 step 14 후처리. 외부 이관 시 도메인 누수(lesson 17 6단계 실패) | ❓ |
| 3.2 추천 카운터 | **A (file-pipeline 잔류)** | search_mode_counters/crag_counters는 C1 자기학습 입력. settings.db에 영속. 외부는 CounterSinkPort trait로 콜백 | ❓ |
| 3.3 EmbeddingPort 호출 | **B (외부가 임베딩+색인 일괄)** | 가공 14단계 step 8(임베딩)+13(색인)을 외부 라이브러리가 묶어서 처리. service.rs는 텍스트만 전달. 결합도 최소 | ❓ |
| 3.4 KG 모듈 위치 | **A (vectordb-search 통합)** | KG는 LocalVectorStore find_related 부산물. 별도 모듈은 over-engineering. kg_3종 MCP 도구도 검색 측 | ❓ |
| 3.5 Topic / Wiki Export | **C (Topic 외부 / Wiki 잔류)** | Topic Merger(490줄)는 KG 분석 → 검색 측. Wiki Export(547줄)는 obsidian 마크다운 출력 → file-pipeline 가공 결과 책임 | ❓ |
| 3.6 GUI Dashboard | **A (file-pipeline 유지)** | GUI는 사용자 접점. 외부 라이브러리 in-process Rust 호출(IPC 아님)로 오버헤드 무시 가능. Documents 탭은 외부 호출 wrapper만 | ❓ |
| 3.7 Linter::lint_strong_claims | **A (file-pipeline 잔류)** | 검증·lint는 본질. vector_db trait 호출(외부 impl)로 충분 | ❓ |
| 3.8 mydocsearch_decision | **deprecated 이관 + 무효화 명시** | 2026-04-08 "LocalVectorStore 단일" 결정이 본 plan으로 무효화. 진실원 충돌 회피 | ❓ |
| **진행 단위** | **lesson 16 단계 0~8 (Phase 108~115)** | 1~2만 줄 이관 1 Phase 일괄은 빌드 회귀 위험. lesson 16 Phase 60 패턴 재적용 | ❓ |
| **선행 작업** | Phase 107 release 재빌드 (메타 룰 17 의무) | 코드 변경 phase 종결 의무 | ✅ 의무 |
| **MCP 도구 분할** | 외부 8 / 잔류 28 (§1.3 정밀 카운트) | mcp_server.rs `make_tool` 36회 실측 + 분류 완료 | ❓ |
| **데이터 누적 의존** | 단계 0~7은 즉시 / 단계 8(측정)은 B-1/B-2 누적 후 | 코드 분리는 누적 무관, 측정은 누적 필요 (§6 위험) | ❓ |

### 권장 채택 시 즉시 가능한 진입 흐름

1. release 재빌드 (선행, 의무)
2. plan §3 추정 채택 합의 (사용자 단일 응답)
3. Phase 108-A 진입 — 단계 0 + 1 (placeholder + vectordb-search-api)
4. 각 단계 종결마다 형제 시뮬레이션 통과 + lesson 17 6단계 점검
5. 단계 7 종결 시 spec/architecture.md / domain-map.md / webapp-design.md / mydocsearch_decision.md 일괄 갱신 (메타 룰 30 자기 적용 4번째 사례 → META 정식 승격 가능)
6. 단계 8(측정)은 B-1 audit_trace 50건+ + B-2 processing_metrics 50건+ 누적 후로 분리

---

## 9. 다음 단계 (다음 세션 진입 흐름)

### 9.1 본 plan 합의 절차

1. 사용자가 plan §3 8개 논점 + §1.3 MCP 분할 + §8 진행 단위 검토
2. claude 권장 일괄 채택 (단일 응답) 또는 항목별 다른 결정
3. plan §0 사용자 합의 사항에 결정 사실 추가 (메타 룰 22 자기 적용)

### 9.2 Phase 108 진입 전 선행 작업

- **A. release 재빌드** (Phase 107 누적분, 메타 룰 17 의무)
  - watcher.rs / work_queue.rs / config.rs / commands.rs / main.rs / lib.rs / service.rs Rust 변경
  - index.html / dashboard.js UI 변경
  - `release_rebuild_required.sh` 실행 후 PASS 의무
  - D:\file-test 재배포 (메타 룰 17 강화 후보)
- **B. 회귀 게이트 7종 baseline 측정**
  - dead_selector_scan / gui_http_smoke / audit_stage_check / release_rebuild_required / action_catalog / dead_selector_scan_v3 / empty_state_audit
- **C. bench 3회 중앙값 baseline 재측정** (분리 후 회귀 비교 기준)

### 9.3 Phase 108 단계 진입 (lesson 16 패턴)

| Phase | 단계 | 산출 |
|-------|------|------|
| 108-0 | placeholder workspace 생성 | `_rust_module/Cargo.toml` + 6 멤버 placeholder. 0건 빌드 통과 |
| 108-1 | vectordb-search-api 분리 | trait + 도메인 타입 이관. core가 외부에 의존 (단방향) |
| 109 | vectordb-search 본체 이관 | LocalVectorStore + MMR + vec_io |
| 110 | Embedding 6 어댑터 이관 | FastEmbed + Providers 2 크레이트 |
| 111 | Reranker 3 어댑터 이관 | reranker-fastembed |
| 112 | KG + Topic Merger 이관 | (§3.4/3.5 결정에 따라) |
| 113 | MCP 도구 분할 | 외부 8 / 잔류 28 |
| 114 | 진입점 + GUI 위임 + spec 정리 | CLI/Tauri/Dashboard. spec/mydocsearch_decision → deprecated |
| 115 | 회귀 게이트 + 측정 | bench 회귀 0 + B-1/B-2 누적 후 분리 효과 측정 |

각 phase 종결마다:
- lesson 17 6단계 누수 점검 (`grep -rn file_pipeline_ _rust_module/`)
- 형제 시뮬레이션 통과 (단계 7 마지막 1회)
- `cargo build --tests --workspace` 통과 (메타 룰 1 sub-rule 1b)
- spec 본문 즉시 갱신 (메타 룰 30 정식 승격 4번째 사례)

### 9.4 본 plan 합의 후 즉시 실행 가능한 작업

claude 권장 일괄 채택 시 다음 세션 진입 1시간 내 가능:
1. plan §0 합의 갱신 (5분)
2. release 재빌드 (15~20분)
3. 회귀 게이트 baseline (5분)
4. Phase 108-0 placeholder 생성 (10분)
5. Phase 108-1 vectordb-search-api 분리 진입 (나머지 시간)
