---
created: 2026-06-16
updated: 2026-06-16 (트리거 H 본질 명칭 재정의: mydocsearch → file-search / file-pipeline → file-wiki / GitHub 신규 origin)
status: draft (프로젝트 미신설 — file-wiki 측 보존)
owner: reujea
project_state: "C:\\dev\\claude_workspaces\\file-search\\ 부재 (2026-06-16 사용자 확인). GitHub https://github.com/reujea/file-search.git 빈 저장소 신설 확인 (2026-06-16)"
boundary_decision: "file-wiki 측 prd/research/ 잔류 — file-search.git 신설 후 Phase 211 C1 시점에 file-search.git/spec/architecture.md로 본문 이관 예정 (lesson 16 단계 0 패턴 정합)"
trigger: "file-wiki 트리거 D (해석 B 확정, 2026-06-16 사용자 명시) — 검색 영역 별도 git 저장소 분리. 트리거 H (2026-06-16) 본질 명칭 재정의로 file-search → file-search 갱신"
parent_decision:
  - prd/roadmap/pipeline-redesign-and-wiki-2026-06-16.md §1.1.ter (단일 진실원) + §1.1.octa (트리거 H 본질 명칭 재정의)
  - prd/research/plugin-architecture-2026-06-04.md §3-C (Phase 207 어댑터 plugin 변환)
external_ref:
  - https://gist.github.com/karpathy/442a6bf555914893e9891c11519de94f (Karpathy LLM Wiki — 본질 영감)
  - https://github.com/reujea/file-search.git (Phase 211 진입 시 신규 저장소 신설 대상, 빈 저장소 신설 완료 2026-06-16)
deprecated_predecessor:
  - mydocsearch_decision.md (2026-04-08 결정, 2026-06-05 즉시 삭제 — lesson 73). 본 결정과 완전 다른 결정 (구 결정 = LocalVectorStore 단일 / 본 결정 = 별도 git + 단독 실행). 시계열 영역 원본 명칭 보존
  - mydocsearch-spec-2026-06-16.md (2026-06-16 초안, 트리거 H로 rename — 본 파일이 후속). 시계열 영역 원본 명칭 보존
rename_history:
  - 2026-06-16: mydocsearch-spec-2026-06-16.md → file-search-spec-2026-06-16.md (트리거 H, 본질 명칭 재정의). 시계열 영역 원본 명칭 보존
---

# file-search 신규 프로젝트 spec 초안

## 0. 본 문서 위상

- 본 문서는 **file-search 프로젝트 미신설 시점의 spec 단일 진실원**
- 위치: file-wiki 측 `prd/research/` (lesson 16 단계 0 패턴 — 신규 프로젝트 placeholder 보존)
- 이관 트리거: file-search.git 신설 시 (Phase 211 C1) → file-search.git/spec/architecture.md 본문 이관 + 본 파일은 file-wiki 측 deprecated.md 위임
- 본 문서의 결정은 file-wiki `prd/roadmap/pipeline-redesign-and-wiki-2026-06-16.md` §1.1.ter (트리거 D) + §1.1.octa (트리거 H) + §11 Phase 211과 동기 갱신 의무
- ⚠ 본 문서 시계열 영역 (lesson 73 인용 + 2026-06-04 결정 인용 + mydocsearch_decision.md 표기) 의 file-pipeline / mydocsearch 명칭은 결정 시점의 명칭이라 보존

## 1. 본질 정의

### 1.1 한 문장

> **file-search = 개인 사용자의 가공된 문서 코퍼스를 단독 / plugin 양 모드로 색인하고 검색하는 Rust 단일 바이너리 솔루션.**

### 1.2 본질 도메인 (file-wiki 본질과의 직교)

| 영역 | file-wiki (host) | file-search |
|------|------------------|-------------|
| 본질 | **파일 가공 → Wiki 구축** (Preprocess 위임 → LLM 분류·가공 → Verify → frontmatter .md 저장 → 흡수 Merge) | **검색** (색인 + 질의 + 리랭킹 + KG 탐색) |
| 입력 | 원본 파일 (inbox/) | 가공본 markdown + frontmatter + embedding vec |
| 출력 | `processed/{slug}.md` (frontmatter + 본문 + wikilink) + `originals/{stem}.{ext}` | search hit 리스트 + KG 인접 노드 + 통계 |
| 사용자 진입점 | watcher + Tauri GUI + CLI | 단독 CLI + 단독 MCP server + plugin (file-wiki IPC) |
| 단독 실행 가능성 | (검색 측 plugin 부재 시도) | ✅ **file-wiki 없이도 색인·검색 가능** (해석 B 핵심) |

### 1.3 부정 정의 (file-search가 *아닌* 것)

- ❌ 파일 가공 (Preprocess / LLM 분류 / Verify 책임 없음 — file-wiki 영역)
- ❌ frontmatter 생성 (file-wiki가 LLM 위임으로 생성한 결과 read-only 색인)
- ❌ Storage 압축 (file-wiki의 zstd 폐기 결정에 정합, file-search도 평문 가정)
- ❌ PII 격리 (file-wiki 사전검사 영역)
- ❌ 알림 (file-wiki notification 도메인)
- ❌ **흡수 결정** (트리거 G — LLM 관련도 분류 + 사용자 확인 모달 + 본문 통합은 file-wiki 측 가공 본질 영역. file-search는 `search.similar` IPC로 후보만 제공)
- ❌ **CrossRef 큐 / 토픽 병합** (트리거 E·F — frontmatter 단일 진실원, file-search는 색인 시점에 frontmatter `relations` read만)

### 1.4 본질 영감 (Karpathy LLM Wiki 4축 정합)

| Karpathy 영역 | file-search 매핑 |
|--------------|------------------|
| Wiki 페이지 (markdown + wikilink) | file-wiki 측 산출, file-search는 색인 |
| Cross-reference | DocRelation (RelationOrigin 5종) — file-search KG 영역 |
| Index refresh | file-search 본질 책임 (vector_db upsert + HNSW + sparse + RRF) |
| Query → 영구 페이지 승격 | W3 (Phase 213) — file-wiki 측 처리 (가공본 부분집합), file-search는 색인 대상 |
| Lint (모순/orphan/stale) | W4 — file-wiki 측 통합 뷰, file-search는 raw 신호 제공 |

→ file-search는 Karpathy 패턴의 **검색·인덱스 측 책임**만 분담. Wiki 본체는 file-wiki.

## 2. 도메인 경계

### 2.1 file-search 잔류 영역 (file-wiki → 이관 대상, D-5b 중간 채택 가정)

| 영역 | 출처 (현 file-wiki) | 비고 |
|------|------------------------|------|
| **벡터 저장소** | `crates/adapters/src/driven/vector_db/local_store.rs` (LocalVectorStore + mmap + HNSW + Blue-Green slot + batch) | 본체 — 단일 진실원 |
| **vec_io** | `crates/core/src/domain/vec_io.rs` | `.vec` 파일 직렬화 |
| **MinHash 인덱스** | `crates/adapters/src/driven/vector_db/local_store.rs` 내 (Phase 52/59 옵션) | 검색 측 가속 인프라 |
| **검색 알고리즘** | dense / sparse(BM25) / RRF / MMR / HNSW | 검색 본질 |
| **리랭커** | `crates/adapters/src/driven/reranking/` (FastEmbedReranker + ClaudeReranker + NullReranker) | 검색 의존 |
| **KG (지식 그래프)** | `crates/core/src/domain/cross_reference.rs` 일부 (kg_neighbors / kg_paths / DocRelation) | 검색 의존 |
| **MCP 검색 도구** | `crates/shared/src/mcp_server.rs::handle_search` / `handle_kg_neighbors` / `handle_kg_paths` (4종) | 단독 MCP server 대상 |
| **SearchConfig** | `crates/shared/src/config.rs::SearchConfig` (window_lines, mmr_lambda, sparse_weight, expand_kg_hops, diversity_threshold, hyde_enabled, tfidf_rerank_enabled, kg_beam_search) | 본 영역 설정 |

### 2.2 file-wiki 잔류 영역 (file-search 이관 안 함)

| 영역 | 이유 |
|------|------|
| **Embedding 어댑터 6종** | 가공 + 검색 양쪽 사용 → 양쪽 보유 (Phase 207 fp-plugin-embedding-* plugin이 양 워크스페이스 공유) |
| **CrossRef queue + 메타블로킹** | CrossRef는 가공 후처리 본질 영역 (file-wiki service.rs 후처리 단계 17) |
| **MinHash 트리거 옵션** (`minhash_force`) | CrossRef 측 결정이라 file-wiki 측 잔류, file-search는 결과 read만 |
| **Entity 추출 (LLM 우선 + regex 폴백)** | 가공 후처리 영역 |
| **promote_to_wiki (W3, Phase 213)** | 답변 페이지는 가공본 부분집합 (자연어), 색인 대상 |
| **Verify 6지표 + 2-Pass** | 가공 본질 |
| **PII / 민감 격리** | 가공 사전검사 본질 |

### 2.3 양쪽 공유 영역 (회색 지대)

| 영역 | 처리 방식 |
|------|----------|
| **Embedding 어댑터 6종** | Phase 207 `fp-plugin-embedding-*` plugin이 `_rust_module/`에 잔류, 양 워크스페이스 공유 |
| **`Document` / `Metadata` / `DocRelation` 도메인 구조체** | `_rust_module/` 신규 멤버 `fp-domain-types` (선택, §10 Q11) 또는 양쪽 중복 정의 |
| **`SearchHit` / `SearchConfig` 응답·요청 구조체** | file-search 본진실원 + file-wiki은 client 측 re-export |
| **frontmatter 파싱** | 양쪽 read 능력 보유 (file-wiki = write, file-search = read 위주) |

## 3. 단독 모드 vs plugin 모드 (해석 B 핵심)

### 3.1 단독 모드 (file-search standalone)

```
사용자: file-search index --path D:/docs   # 색인
사용자: file-search search "회의 결정사항"  # 검색
사용자: file-search serve --mcp             # MCP server 단독 진입점
```

특성:
- file-wiki 없이 단독 사용
- 입력: 외부에서 가공된 `.md` 파일 (frontmatter 포함) — file-wiki 산출 호환
- 호환 frontmatter 스펙: `doc_types` / `keywords` / `entities` / `summary` / `hierarchy` / `needs_verification` / `open_questions` / `relations` / `sources` / `date` / `id`
- 사용자가 다른 도구로 frontmatter `.md`를 생성해도 색인 가능 (file-wiki 의존 없음)

### 3.2 Plugin 모드 (file-wiki에서 IPC 호출)

```
file-wiki 가공 종료 시점:
  └→ IPC: fp-plugin-search 호출 (upsert)
       └→ file-search::index_one(doc) 호출
            └→ LocalVectorStore upsert + HNSW + vec_io.save_vec
```

특성:
- file-wiki plugin-architecture-2026-06-04 §2-A 4축 정합
- thin wrapper: `_rust_module/fp-plugin-search/src/main.rs` ~50줄 (file-search crate 의존)
- IPC 경계: named pipe / Unix domain socket (lesson 76)
- 매니페스트: `fp-plugin.toml` (`vector_db.read` + `vector_db.write` + `kg.read` 권한)

### 3.3 양 모드 진입점 분리 표

| 진입점 | 단독 | plugin |
|-------|------|--------|
| `file-search` CLI 바이너리 | ✅ | ✅ (사용자 직접 색인 + plugin은 자동) |
| `file-search-mcp` MCP server 바이너리 (선택, §10 Q12) | ✅ | △ (file-wiki 측 MCP도 search 노출하면 중복) |
| `fp-plugin-search` thin wrapper 바이너리 | ❌ | ✅ (file-wiki 측 진입점) |

claude 추정: **`file-search-mcp` 별도 진입점 잔류** — Claude Code가 file-search만 등록해도 search 가능 (file-wiki 미설치 환경 정합).

## 4. 모듈 구성 (워크스페이스 초안)

```
file-search/
├── .git/                            ← 신규 git 저장소 (file-wiki.git과 완전 독립)
├── Cargo.toml                       ← workspace
├── Cargo.lock
├── README.md                        ← 단독 + plugin 양 모드 명시
├── LICENSE
├── .gitignore
├── CLAUDE.md                        ← file-search 측 세션 시작 절차 (file-wiki 패턴 흡수)
├── spec/                            ← Claude용 기술 명세
│   ├── architecture.md              ← 본 파일이 이관될 자리
│   ├── domain-map.md
│   ├── scenarios.md
│   ├── deprecated.md
│   └── lesson-learned/
│       ├── INDEX.md
│       └── META.md
├── prd/                             ← roadmap / research
├── docs/                            ← 사용자 가이드 (선택)
├── crates/
│   ├── core/                        ← 도메인 모델 + 포트 trait + service
│   │   ├── domain/
│   │   │   ├── document.rs          ← Document / Metadata (frontmatter 직렬화 호환)
│   │   │   ├── search_hit.rs
│   │   │   ├── doc_relation.rs      ← DocRelation + RelationOrigin 5종
│   │   │   └── vec_io.rs
│   │   ├── ports/
│   │   │   ├── input.rs             ← IndexPort / SearchPort / KgPort
│   │   │   └── output.rs            ← VectorDBPort / RerankerPort / EmbeddingPort (client interface)
│   │   └── service.rs               ← IndexService + SearchService
│   ├── adapters/
│   │   ├── driven/
│   │   │   ├── vector_db/
│   │   │   │   └── local_store.rs   ← LocalVectorStore + mmap + HNSW + MinHash
│   │   │   ├── reranking/
│   │   │   │   ├── fastembed_reranker.rs
│   │   │   │   ├── claude_reranker.rs
│   │   │   │   └── null_reranker.rs
│   │   │   ├── kg/
│   │   │   │   └── local_kg.rs      ← kg_neighbors / kg_paths / DocRelation 영역
│   │   │   └── embedding_client/    ← Embedding 어댑터 client (실 어댑터는 file-wiki 또는 plugin 측)
│   │   │       └── plugin_client.rs
│   │   └── driving/
│   │       ├── frontmatter_parser.rs ← .md frontmatter read
│   │       └── ingest_watcher.rs    ← 단독 모드 시 폴더 watch (선택)
│   └── shared/
│       ├── config.rs                ← SearchConfig + IndexConfig
│       ├── settings_db.rs           ← 단독 모드 시 자체 DB (선택)
│       └── mcp_server.rs            ← 단독 MCP server (handle_search / handle_kg_*)
├── modals/
│   ├── cli/                         ← file-search CLI 바이너리
│   │   └── main.rs                  ← index / search / serve / health subcmd
│   └── mcp/                         ← file-search-mcp 바이너리 (선택, §10 Q12)
│       └── main.rs
└── tests/                           ← 통합 테스트 (ServiceBuilder 패턴)
```

추정 라인 수 (file-wiki 현 코드 기준 — Phase 211 진입 전 사전 측정 의무):
- LocalVectorStore + mmap + HNSW + MinHash + Blue-Green slot + batch: ~1,500줄
- Reranking 3 어댑터: ~600줄
- KG (cross_reference.rs 일부 추출): ~400줄
- vec_io + Document/Metadata domain: ~300줄
- MCP search handlers: ~250줄
- 통합 테스트: ~600줄
- **소계: ~3,650줄** (2026-06-01 plan 4,200줄 추정과 거의 정합, Phase 65~107 누적 영향 미반영 — Phase 211 진입 시 재측정 의무)

## 5. 인터페이스 (단일 진실원)

### 5.1 frontmatter 호환 스펙 (단독 모드 진입 의무)

file-search는 다음 frontmatter 필드를 read한다:

```yaml
---
id: <doc id, 필수>
doc_types: [<문서 유형>, ...]
keywords: [<키워드>, ...]
date: <ISO 8601, 선택>
summary: <요약, 선택>
hierarchy: [<계층 경로>, ...]
entities:
  - {name: <이름>, type: <person|organization|place|technology|amount|project|concept>}
needs_verification: [<검증 필요 표현>, ...]
open_questions: [<미해결 질문>, ...]
relations:
  - {target: <doc id>, type: <similar|reference|contradict|...>, origin: <auto_similarity|user_wikilink|llm_extracted|user_manual|lint_auto_fix>}
sources: [<인용 출처>, ...]
---
```

스펙 우선순위:
1. **frontmatter** (단일 진실원)
2. **본문 안 wikilink** (`[[xxx]]`) — 자동 추출 후 relations 보완
3. **embedding `.vec`** — 별도 바이너리, 같은 디렉토리

### 5.2 IPC 인터페이스 (plugin 모드)

`fp-plugin-protocol::IpcMessage::method` 표:

| method | 책임 | 권한 |
|--------|------|------|
| `search.execute` | dense + sparse + RRF + 리랭킹 hit 리스트 | `vector_db.read` |
| `search.similar` | embedding 기반 인접 doc 검색 | `vector_db.read` |
| `search.with_filter` | doc_types/date 필터 + 검색 | `vector_db.read` |
| `search.cached` | A1 캐시 hit 확인 (선택) | `cache.read` |
| `kg.neighbors` | DocRelation 인접 노드 (origin 라벨 포함) | `vector_db.read` |
| `kg.paths` | DocRelation 다중 hop 경로 | `vector_db.read` |
| `index.upsert` | 가공 종료 시 file-wiki → file-search 업서트 | `vector_db.write` |
| `index.delete` | doc 삭제 (TTL 만료 / 수동) | `vector_db.write` |
| `index.health` | 색인 상태 (총 doc 수 / HNSW depth / mmap size) | `vector_db.read` |

### 5.3 단독 CLI 인터페이스

```
file-search index --path <DIR>          # 폴더 색인
file-search index --file <PATH>         # 단일 파일 색인
file-search search <QUERY> [--top-k N] [--rerank fastembed]
file-search kg neighbors <DOC_ID>
file-search kg paths <FROM> <TO>
file-search serve --mcp [--stdio | --port N]
file-search health
```

## 6. 의존 영역

### 6.1 외부 크레이트 (file-wiki와 공유)

| 영역 | 출처 |
|------|------|
| HNSW | `instant-distance` 또는 자체 구현 (file-wiki 현 사용 검증) |
| mmap | `memmap2` |
| Embedding (호출만) | IPC → `fp-plugin-embedding-*` (Phase 207) |
| Reranking 본체 | `fastembed-rs` (BGE-Reranker-v2-M3) |
| MCP protocol | `_rust_module/module-mcp` (lesson 60 정합) 또는 단독 구현 |
| MinHash | `probabilistic-collections` 또는 자체 |
| zstd | ❌ (file-wiki 트리거 C 정합 — file-search도 zstd 폐기) |

### 6.2 `_rust_module/` 공유 plugin 의존

| plugin | 호출 책임 | 시점 |
|--------|----------|------|
| `fp-plugin-embedding-*` (6종, Phase 207) | embedding 생성 | 색인 시 (단독 모드는 자체 호출 또는 외부에서 vec 입력) |
| `fp-plugin-llm-*` (7종, Phase 207) | reranker가 Claude 사용 시만 | search 시 |

## 7. Phase 진행

### 7.1 본 spec의 Phase 매핑 (file-wiki 측 Phase와 동기)

| file-wiki Phase | file-search 영향 |
|--------------------|------------------|
| Phase 210 (트리거 A+B+C) | file-search 미존재 — frontmatter 호환 스펙만 영향 |
| **Phase 211 (트리거 D 본진입)** | **file-search.git 신설 + 코드 이관 + IPC 검증** ⭐ 본 spec이 이관되는 시점 |
| Phase 212 (W2 활성화) | IPC 경유 `search.similar` 호출 비용 실측 |
| Phase 213 (W3 활성화) | `promote_to_wiki` 결과 색인 대상 추가 |

### 7.2 Phase 211 본진입 6 묶음 (file-wiki 측 §11 Phase 211과 동기)

C1: 저장소 + 워크스페이스 신설
C2: 검색 영역 코드 이관 (D-5b 중간)
C3: IPC 인터페이스 정의 + thin wrapper plugin
C4: file-wiki 측 정리 + 어댑터 client 전환
C5: 단독 실행 검증
C6: 회귀 자동화 + 릴리즈

각 묶음의 file-search 측 산출:
- C1: `file-search/Cargo.toml` + `spec/architecture.md` (본 spec 이관) + `CLAUDE.md`
- C2: `crates/core/` + `crates/adapters/` + 통합 테스트 신규
- C3: `fp-plugin-search/src/main.rs` (~50줄) + `_rust_module/fp-plugin-search/Cargo.toml`에 `file-search = { git = "..." }` 의존
- C4: file-wiki 측 `VectorDBPort` 어댑터 → IPC client 어댑터로 전환
- C5: `file-search index --path test_corpus` + `file-search search query` 통합 검증
- C6: gitlab.bi.co.kr CI baseline + D:\file-test 양 저장소 배포

## 8. 위험 매트릭스

| 위험 | 가능성 | 영향 | 완화 |
|------|--------|------|------|
| **2026-04-08 동명 결정과 혼동** | 🟡 | 사용자/Claude 둘 다 혼동 가능 | 본 spec 첫 줄에 "구 결정과 완전 다름" 명시 + lesson 73 (즉시 삭제) 인용 |
| **file-wiki ↔ file-search 동기 갱신 부담** | 🟡 | 양 저장소 PR 동기 의무 (메타 룰 1 sub-rule 1f 확장) | 본 spec이 frontmatter 호환 스펙 단일 진실원 — file-wiki 측은 본 spec read-only 인용 |
| **Embedding 어댑터 양쪽 보유 → 단일 진실원 위반** | 🟡 | 메타 룰 19 위반 위험 | Phase 207 `fp-plugin-embedding-*` plugin이 `_rust_module/` 잔류, 양 저장소가 plugin read만 |
| **단독 모드의 frontmatter 스펙 ↔ file-wiki 측 spec 표류** | 🟡 | 호환성 깨짐 | 본 spec §5.1 단일 진실원 + file-wiki `prompts.toml` classify 갱신 시 본 spec 인용 의무 |
| **file-search가 가공 본질 흡수 시도 (D-5c 광범위 채택)** | 🟢 (낮음) | 도메인 경계 흐림 | claude 추정 = D-5b 중간 — 본 spec §2.2 명시 |
| **IPC 비용 (search.similar +10~30ms)** | 🟡 | W2 활성 시 cascade 비용 누적 | Phase 212 실측 후 결정. W2 디폴트 false 유지로 영향 0 |
| **CI 분리로 인한 회귀 자동화 양쪽 부담** | 🟡 | release 빌드 + 배포 2회 | release_redeploy.sh 양 저장소 동기 (메타 룰 17 강화) |
| **history 폐기 (단순 복사) → 추적성 손실** | 🟢 | git blame 끊김 | 본 결정 사용자 합의 (file-wiki.git이 2026-06-10 신설이라 history 가치 낮음) |

## 9. 호환성 / 회귀

### 9.1 file-wiki 측 영향

- `VectorDBPort` / `RerankerPort` trait는 그대로 (interface 유지)
- 어댑터 본체 → IPC client 어댑터 1건 (file-search 호출)
- `SearchConfig` → `file-search::SearchConfig` re-export (또는 IPC params 직렬화)
- 통합 테스트: ServiceBuilder 패턴 — 변경 0 (file-search mock으로 대체)

### 9.2 file-search 단독 시 회귀 기준선

- 색인 처리량: file-wiki Phase 64 트리거 #11/#12 bench 23.62 docs/s 기준 — Phase 211 종결 후 동등 이상 유지
- 검색 정확도: Phase 62 fastembed MRR 0.975 동등 이상
- per-search 시간: Phase 64 검색 64ms/건 동등 이상
- HNSW depth / mmap size: file-wiki 현 baseline과 동일

### 9.3 통합 모드 회귀

- file-wiki 단독 처리 시간 변동: +1~5ms/doc (IPC overhead 추정)
- 본 추정은 Phase 211 C5 시점 실측 의무

## 10. 결정 대기 영역 (15건)

### Q11. 공유 도메인 타입 처리

- (a) `_rust_module/fp-domain-types` 신규 멤버 — `Document` / `Metadata` / `DocRelation` / `SearchHit` 양 저장소 공유
- (b) 양쪽 중복 정의 + frontmatter YAML로 wire 직렬화 통신
- (c) file-search 단일 진실원 + file-wiki은 client re-export
- claude 추정: **(c)** — Document/Metadata는 가공 영역이라 file-wiki 단일 진실원, SearchHit/SearchConfig는 file-search 단일 진실원. RelationOrigin 5종은 가공 (file-wiki) 측 단일 진실원이라 file-search는 read enum 복사 또는 string 직렬화

### Q12. `file-search-mcp` 단독 진입점 별도 바이너리

- (a) 별도 바이너리 (`file-search-mcp.exe`) — 단독 사용성 우위
- (b) `file-search serve --mcp` subcmd 단일 바이너리 — 배포 단순
- claude 추정: **(b)** — tasty 패턴 (단일 바이너리) 정합 + lesson 60 `_rust_module/module-mcp` 패턴 정합

### Q13. 단독 모드 시 settings.db / config 위치

- (a) `file-search/.config/` (XDG 패턴)
- (b) `~/.file-search/`
- (c) file-wiki의 `PIPELINE_BASE` 패턴 흡수 — `MYDOCSEARCH_BASE` env + 자동 생성
- claude 추정: **(c)** — file-wiki lesson 29 PIPELINE_BASE 패턴 흡수, 일관성 우위

### Q14. file-wiki ↔ file-search frontmatter 스펙 단일 진실원

- (a) **본 spec §5.1** (file-search 측) — file-wiki `prompts.toml` classify는 read-only 인용
- (b) file-wiki 측 spec (가공 산출이라 가공 측 단일 진실원)
- (c) `_rust_module/fp-domain-types` 분리 시 거기 단일 진실원
- claude 추정: **(b)** — frontmatter는 file-wiki LLM 생성 산출물이라 가공 측 단일 진실원. file-search는 read 호환만 보장. 본 spec §5.1은 file-wiki 측 spec 인용으로 변경

### Q15. CI 위치

- (a) gitlab.bi.co.kr (file-wiki.git 정합, lesson 76)
- (b) GitHub (오픈소스 가능성 영역)
- (c) 미설정 (로컬만)
- claude 추정: **(a)** — file-wiki.git 환경 정합

## 11. 진행 상태

- 본 spec 초안: ✅ 작성 (2026-06-16)
- file-search.git 신설: ⏳ (Phase 211 C1)
- 본 spec → file-search/spec/architecture.md 이관: ⏳ (Phase 211 C1)
- file-wiki 측 `prd/roadmap/pipeline-redesign-and-wiki-2026-06-16.md` §1.1.ter + §11 Phase 211과 본 spec의 동기 갱신: ⏳ (다음 turn)

## 12. 본 spec 위치 정합

본 spec은 file-wiki 측 `prd/research/`에 잔류:

- 이유: file-search.git 미신설 → 보존 위치 부재. lesson 16 단계 0 패턴 (신규 프로젝트 placeholder)
- file-search.git 신설 (Phase 211 C1) 시점에:
  1. `file-search/spec/architecture.md` 본문 이관
  2. `file-search/spec/domain-map.md` §검색 도메인 갱신
  3. file-wiki 측 본 파일 → `spec/deprecated.md` 위임 (lesson 49 단일 진실원 위임 패턴)
  4. file-wiki 측 `prd/roadmap/pipeline-redesign-and-wiki-2026-06-16.md`에 본 spec 새 위치 인용
