---
created: 2026-06-16
updated: 2026-06-16 (사용자 결정 +7: 가공본 .md 평문 + frontmatter / zstd 압축 단계 완전 제거 / 검색 영역 완전 별도 git 저장소 분리 (해석 B) / CrossRef·Entity → frontmatter 흡수 / 토픽 자동 병합 제거 / LLM Wiki 흡수 Merge / 본질 명칭 재정의 file-pipeline → file-wiki + mydocsearch → file-search + GitHub 신규 origin)
status: draft (사용자 결정 24건 대기, 7건 확정)
owner: reujea
trigger: 본 세션 사용자 트리거 — (a) "가공 파이프라인 spec 변경" + (b) "karpathy LLM Wiki 패턴으로 고도화" + (c) "가공본 .md 평문 + frontmatter" + (d) "zstd 압축 단계 제거" + (e) "검색 기능 별도 프로젝트 분리 (해석 B)" + (f) "CrossRef 큐 + 엔티티는 frontmatter로 기능 이관" + (g) "토픽 자동 병합 제거" + (h) "llm 위키처럼 문서 가공 결과가 기존 문서와 관련 있는 경우 기존 문서로 흡수 (해석 G1 Merge)" + (i) "file-wiki로 본질 명칭 재정의 + git origin GitHub 변경 + 커밋"
related:
  - spec/architecture.md (가공 파이프라인 본체 — service.rs:241)
  - spec/domain-map.md §본질 재정의 (2026-06-04 tasty 패턴)
  - spec/scenarios.md §시나리오 2 (파일 투입 → 가공 → 검색)
  - prd/research/plugin-architecture-2026-06-04.md (Phase 200~209)
  - prd/research/external-analysis-2026-05-15.md (외부 분석 단일 진실원 패턴)
  - prd/research/mydocsearch-spec-2026-06-16.md (트리거 D 신규 프로젝트 spec 초안 — 본 문서와 시계열 동기, mydocsearch.git 신설 시 그쪽으로 이관 예정)
  - lesson 30 / 45 / 67 (인프라 선구현 + 도메인 특수성)
external_ref:
  - https://gist.github.com/karpathy/442a6bf555914893e9891c11519de94f
---

# 가공 파이프라인 재설계 + LLM Wiki 고도화 통합 구현 방안

## 0. 본 문서 위치

- **단일 진실원**: 본 문서가 두 트리거(파이프라인 재설계 + Wiki 고도화)의 통합 결정 단일 진실원
- **결정 대기 영역**: §10 의사결정 매트릭스의 Q1~Q7. 사용자 결정 후 §11 Phase 분할안 확정 → Phase 진입 시점에 spec/architecture.md §누적 변경 요약 + spec/domain-map.md 동기 갱신
- **외부 인용**: Karpathy gist는 §1.1 인용 후 인용 출처 의무 (메타 룰 외부 분석 패턴)

## 1. 사용자 트리거 요약

### 1.1 트리거 A — 가공 파이프라인 spec 변경

사용자 명시 변경 (본 세션 직접 입력):

#### 사전 처리 (블록 A)
- 증분 해시 (CompileState) **제거** — "처리 결과는 별도 폴더로 이관되므로 해당 단계 필요 없음"
- 민감 경로/파일명/키워드 판별 단계 **merge** ([확인 필요] mere → merge 해석. §10 Q1)

#### 파이프라인 스텝 (블록 B)
- Preprocess **제거** — "LLM에 전적으로 추출 맡김"
- Chunking **제거** — "LLM에 전적으로 청킹 전략 맡김"
- LLM 분류/가공 — 유지
- Verify — 유지
- Embedding — **후처리로 이관**
- Storage — **후처리로 이동**

#### 후처리 (블록 C)
- 의미 중복 체크 **제거**

### 1.1.bis 트리거 C — 가공본 저장 형식 변경 (2026-06-16 사용자 결정 ✅ 확정)

사용자 명시 (시계열):
1. *"가공 본문을 압축 형태에서 md 파일로 저장하고 메타데이터로 frontmatter 형태로 본문에 추가하자"*
2. *"zst 압축 단계 제거하자"* — 가공본 한정이 아닌 **압축 단계 전체 폐기** (원본 포함)

| 영역 | 변경 전 | 변경 후 |
|------|---------|---------|
| 가공본 파일 | `processed/{doc_type}_{stem}.txt.zst` (zstd 압축) | `processed/{slug}.md` (평문 markdown) |
| 원본 파일 | `originals/{stem}.{ext}.zst` (zstd 압축) | `originals/{stem}.{ext}` (평문 원본 그대로) |
| 메타데이터 영속화 | VectorDB `StoredDoc` 단일 진실원 | **본문 frontmatter** (YAML) + VectorDB `StoredDoc` 양쪽 (frontmatter 우선 — 본 결정으로 단일 진실원 본문 이관) |
| 압축 단계 | zstd 적용 (가공본 + 원본) | **완전 미적용** (사용자 2차 결정) |
| 가독성 | 압축 해제 필요 | **직접 cat/text editor 가독** |
| 트리거 B (W1)와 정합 | (별도 트리거였음) | **동일 변경 = 자연 통합** |
| 디스크 사용량 | zstd 압축률 (텍스트 ~30%) | **원본 크기 (~3.3배 증가)** — 사용자 결정 영역 |

⚠ 본 결정은 트리거 B의 W1 (Markdown Wiki 출력)과 완전 동일 산출물 → §3.1과 §11 B4가 트리거 C의 시행 영역.

영향 범위 (사전 확인 — Grep 결과 11 파일 + 압축 단계 전체):
- `service.rs` 가공본 저장 흐름 (610-625, 812-814) + 원본 압축 (624)
- `StoragePort::compress_and_store / decompress_temp / read_header / delete_expired / compress_with_level` trait — **5 메서드 모두 의미 변경 또는 제거**
- `zstd_storage.rs` 어댑터 — **완전 폐기 또는 평문 복사로 변환** (§10 Q8 결정 영역)
- `wiki_export.rs / topic_merger.rs / lint.rs / cross_reference.rs / auto_reindexer.rs` — `processed_path` 읽기 호출처 (decompress_temp → 직접 read)
- `mcp_server.rs::get_document` 응답 schema (압축 해제 → 직접 read)
- `local_store.rs` mmap 영속 (`StoredDoc` 영역은 보존, frontmatter 우선 정합 검증만)
- `CompressionConfig` (config.rs) — `zstd_level` / `original_ttl_days` 의미 재정의
- Pipeline 탭 노드 그래프 — Storage 노드 자체 제거 (B6 노드 그래프 14 → 13)
- `compression.zstd_level` UI 설정 (domain-map.md:355) — 제거 또는 dead config

영향 받지 않는 영역 (보존):
- **`.vec` 영속화** — embedding 바이너리 (기존 그대로, zstd 미사용)
- **원격 저장소 (notion/s3/webdav)** — 업로드 대상이 평문으로 통일 (오히려 자연 정합)
- **`Metadata` 구조체** — frontmatter는 직렬화 산출물, 구조체 변경 0

### 1.1.ter 트리거 D — 검색 영역 완전 별도 git 저장소 분리 (2026-06-16 사용자 결정 ✅ 확정, 해석 B)

사용자 명시: *"본 솔루션에 검색 기능은 별도 프로젝트로 분리하면 어때?"* → 해석 B (file-pipeline.git ↔ 신규 검색 저장소.git 완전 분리)

**프로젝트 현 상태 (2026-06-16 사용자 확인)**:
- `C:\dev\claude_workspaces\mydocsearch\` **실제 존재 부재** (사용자 명시 + Bash ls 확인)
- 본 결정은 **신규 프로젝트 신설 영역** — Phase 211 C1에서 신설
- mydocsearch spec 초안: `prd/research/mydocsearch-spec-2026-06-16.md` 신규 작성 (본 문서와 시계열 동기)
- 본 spec 초안은 file-pipeline 측 `prd/research/` 잔류 — mydocsearch.git 신설 시 `mydocsearch/spec/architecture.md`로 이관 (lesson 16 단계 0 placeholder 패턴 정합)

**위상**:
- 2026-06-04 본질 재정의 2차 (plugin-architecture)의 **확장** — plugin 멤버 형태에 git 저장소 경계 추가
- 2026-06-10 Phase 203 placeholder (`_rust_module/fp-plugin-search/`) **위치 재정의** — `_rust_module/` 잔류 vs 신규 저장소 이관 결정
- 기존 lesson 75/76 (binary plugin 4축 합의) **위배 아님** — IPC + 매니페스트 + permission gate 그대로 유지

**핵심 결정 5축**:

| 축 | 결정 | 단일 진실원 |
|----|------|------------|
| **D-1 저장소 위치** | `C:\dev\claude_workspaces\mydocsearch\` (또는 동급 — §10 Q9) | `mydocsearch/.git` (file-pipeline.git과 완전 독립) |
| **D-2 워크스페이스 위치** | `_rust_module/` 분리 → 신규 워크스페이스 단독 | `mydocsearch/Cargo.toml` (workspace 단독 또는 멤버 다수) |
| **D-3 통신 경계** | plugin-architecture-2026-06-04 IPC 그대로 (named pipe / Unix domain socket) | host = file-pipeline / client = mydocsearch binary가 plugin 진입점 |
| **D-4 단독 실행 가능성** | mydocsearch는 **file-pipeline 없이도 단독 사용 가능** (해석 B 핵심) | 단독 CLI + 단독 MCP server 진입점 보유. file-pipeline plugin 모드는 진입점 1개 추가 |
| **D-5 LocalVectorStore 본체 + 의존 영역 이관 범위** | (§10 Q10 결정) | 보수: HNSW + mmap + vec_io / 광범위: + reranker + KG + embedding 어댑터 일체 |

**이관 범위 사전 분류** (§10 Q10 옵션):

| 영역 | 보수 (D-5a) | 중간 (D-5b) | 광범위 (D-5c) |
|------|------------|------------|--------------|
| LocalVectorStore + mmap + HNSW + vec_io | ✅ | ✅ | ✅ |
| MMR + sparse + RRF 검색 알고리즘 | ✅ | ✅ | ✅ |
| Reranker (FastEmbed Cross-Encoder + Claude + Null) | host 잔류 | ✅ | ✅ |
| Embedding 어댑터 6종 | host 잔류 | host 잔류 | ✅ |
| KG (kg_neighbors / kg_paths) | host 잔류 | ✅ | ✅ |
| MinHash + 메타블로킹 (CrossRef 영역) | host 잔류 | host 잔류 | ✅ |
| MCP 도구 search / search_with_filter / search_similar / search_kg | mydocsearch | mydocsearch | mydocsearch |
| **mydocsearch가 file-pipeline 없이도 가공 가능성** | ❌ (host = 가공) | ❌ | ✅ (가공도 자체 보유) |

claude 추정: **D-5b 중간** — Reranker + KG는 검색 의존이라 자연 이관, Embedding은 가공도 사용해서 host 잔류 + plugin 인터페이스만 노출. 본 추정 영역은 lesson 30 패턴 (인프라 선구현 후 사용자 트리거 활성화) 의존도 없음 — 명확한 도메인 경계.

**해석 A (단순 plugin 본진입)과의 차이**:

| 영역 | 해석 A (단순 plugin) | 해석 B (저장소 분리) ← **본 채택** |
|------|---------------------|--------------------------------------|
| git 저장소 | file-pipeline.git 1개 | file-pipeline.git + mydocsearch.git (2개) |
| 워크스페이스 | `_rust_module/` 멤버 | `_rust_module/`에서 빠짐 + mydocsearch.git workspace |
| 단독 실행 | ❌ (plugin은 file-pipeline 의존) | ✅ (단독 CLI + 단독 MCP server) |
| 릴리즈 사이클 | file-pipeline 종속 | 독립 |
| Phase 200~209 정합 | ✅ | ✅ (저장소 경계는 IPC 결정과 직교) |
| 사용자 다른 검색 도구로 대체 가능 | △ (plugin 인터페이스 의존) | ✅ |
| 변경 시 회귀 추적 | 단일 PR | 2 저장소 PR 동기 필요 (CI 부담) |
| lesson 76 단독 git 첫 설정 시점 정합 | △ | ✅ (분리 비용 낮음) |

**Phase 분할 결정 (§10 Q9b)**:
- claude 추정: **분리 — Phase 211 = 트리거 D 본진입** (lesson 76 B1~B4 bundle-cycle 패턴 정합. 트리거 A+B+C는 가공 본체 영역, 트리거 D는 plugin 본진입 + 저장소 경계 영역으로 직교)
- Phase 210 = 트리거 A+B+C 단일 / Phase 211 = 트리거 D (저장소 신설 + 이관 + IPC 검증)

**영향 받지 않는 영역 (보존)**:
- plugin-architecture-2026-06-04 §2-A 4축 사용자 합의 — 그대로
- 본 통합 문서의 §11 Phase 210 B1~B6 — 그대로 (트리거 D는 Phase 211로 별도)
- 트리거 B의 W2 cascade — `vector_db.search_similar` 호출이 IPC plugin 경계 통과 (디폴트 false 유지 시 영향 0, 활성 시 +10~30ms/호출 측정)

**의존 영역 (D 진입 시 영향)**:
- `spec/architecture.md` §누적 변경 요약 — Phase 211 항목 + 본 문서 인용
- `spec/domain-map.md` §도메인 2 (검색) — mydocsearch 이관 매핑 표 신규
- `spec/deprecated.md` — file-pipeline 측의 검색 어댑터 위임 표기
- `prd/research/plugin-architecture-2026-06-04.md` §3-C — fp-plugin-search 위치를 _rust_module/ → mydocsearch.git으로 갱신
- 기존 path 의존 어댑터 (LocalVectorStore 등) → mydocsearch crate 의존 + IPC 인터페이스
- Phase 207 어댑터 → plugin 변환 (24 어댑터) 영역 중 검색 측 6종 (embedding 6) 의 처리 분기 (§10 Q10b)

### 1.1.quater 트리거 E — CrossRef + Entity → frontmatter 흡수 (2026-06-16 사용자 결정 ✅ 확정)

사용자 명시: *"CrossRef 큐 + 엔티티는 frontmatter로 기능 이관"*

**현 상태** (service.rs:680-728):
- CrossRef 큐 — `CrossRefQueueItem` 비동기 큐 + 배치 실행 (메타블로킹 + MinHash + similarity_threshold)
- Entity 추출 — LLM 응답 우선 (`llm_entities`) + regex 폴백 (`CrossRefUpdater::extract_entities`)
- 영속화 — `DocRelation` 도메인 객체 + VectorDB → KG 노드 신호

**변경 후**:
- 가공 단계 4 (LLM 분류+가공) 시점에 LLM이 frontmatter 안에 다음 필드 직접 작성:
  - `entities: [{name, type}]` (현 LLM entities 정확도 우위 영역 — Phase 83 측정 후 LLM 우선 채택)
  - `relations: [{target, type, origin}]` (LLM이 본문 wikilink 분석 + 신규 doc 영향 인지로 직접 작성)
- 후처리 별도 큐 / 별도 엔티티 추출 단계 **폐기**
- DocRelation 도메인 객체 → frontmatter `relations` 필드의 영속화 결과 read (단일 진실원 = frontmatter)
- VectorDB `StoredDoc.relations` → frontmatter read 동기화 (단일 진실원 = frontmatter)

**근거 정합**:
- 트리거 B Karpathy 패턴 §3.1 W1 — 본 결정과 **완전 동일 산출물** (frontmatter `relations` + `entities`)
- 트리거 B의 W2 cascade와 자연 정합 — cascade가 frontmatter `relations` 직접 갱신
- Karpathy 인용 정합: *"the LLM incrementally builds and maintains a persistent wiki"* — 별도 큐 없이 LLM이 직접 wiki bookkeeping

**영향 범위** (Grep 사전 확인 — 9 파일):
- `service.rs` 680-728 (CrossRef 큐 + 엔티티 추출 흐름) — 완전 제거
- `core/domain/cross_reference.rs` — `CrossRefQueueItem` / `CrossRefUpdater::extract_entities` 폐기, KG 관련 영역만 잔류 (mydocsearch 이관 대상)
- `core/domain/topic_merger.rs` — 트리거 F 함께 폐기
- `shared/test_helpers.rs` / `shared/cli.rs` / `shared/lib.rs` / `shared/mcp_server.rs` — CrossRef 진입점 제거
- `adapters/driving/watcher.rs` — CrossRef 큐 호출 제거
- `core/domain/mod.rs` — 모듈 노출 정리

**위험**:
- LLM이 entities/relations를 잘못 작성할 경우 → Verify 6지표에 entity_preservation 이미 존재 (Phase 88 정합) — 검증 흐름 그대로
- regex 폴백 부재 → LLM 응답 entities=[] 시 frontmatter `entities` 빈 배열. Verify entity_preservation 임계값 조정 영역 (Phase 210 B5 후 측정 의무)
- MinHash + 메타블로킹 (Phase 52/59 트리거 #2/#4 인프라) → mydocsearch 측 잔류 (검색 가속), CrossRef 큐 영역만 제거

**보존 영역**:
- `RelationType` enum (Similar / Reference / Contradict / Semantic 등) — frontmatter `relations[].type` 값 직렬화
- `RelationOrigin` 5종 (auto_similarity / user_wikilink / llm_extracted / user_manual / lint_auto_fix) — frontmatter `relations[].origin` 직렬화
  - origin = `llm_extracted` 신규 사용 (LLM frontmatter 작성 시) — auto_similarity는 W2 cascade 활성 시점에서만 사용

### 1.1.quinque 트리거 F — 토픽 자동 병합 완전 제거 (2026-06-16 사용자 결정 ✅ 확정)

사용자 명시: *"토픽 자동 병합 제거"*

**현 상태** (service.rs 블록 C 11단계 + topic_merger.rs):
- 가공 종료 시점에 증분 기록 + 인접 doc 토픽 자동 클러스터링
- Phase 65~83 영역 (코퍼스 신호 카운터 + 추천 시스템 5축 → 동작 모듈 12종)

**변경 후**:
- 블록 C 11단계 (토픽 자동 병합) **완전 폐기**
- `core/domain/topic_merger.rs` 삭제
- 토픽 식별은 frontmatter `doc_types` + `hierarchy` (Phase 61 청킹 메타데이터)로 대체 — LLM이 가공 시점에 직접 분류
- 코퍼스 신호 카운터 (search_mode_counters / crag_counters / chunk_stats) — Phase 80 추천 시스템 의존 영역 → §10 Q19 결정

**근거 정합**:
- Karpathy 패턴은 토픽 자동 병합 영역 부재 — wikilink + relations로 자연 그룹화
- 본 솔루션 본질 (file-pipeline = 파일 가공만) 정합 — 토픽 병합은 추천 시스템(Phase 65~83) 영역으로 host 본질에서 분리 영역
- frontmatter `hierarchy` (Phase 61) 이미 존재 — 토픽 분류 영역 LLM 단일 진실원

**영향 범위**:
- `core/domain/topic_merger.rs` 삭제
- `service.rs` 블록 C 11단계 호출 제거
- `shared/cli.rs` / `shared/test_helpers.rs` 호출처 정리
- 코퍼스 신호 카운터 (3 settings.db 테이블) — Phase 80 추천 시스템 의존 영역의 처리 분기 (§10 Q19)

**보존 영역** (트리거 F가 영향 주지 않는 영역):
- frontmatter `hierarchy` (Phase 61 청킹 메타데이터) — LLM 분류
- frontmatter `doc_types` (17 doc_types 스키마)
- 추천 시스템 5축 → 동작 모듈 12종 — Phase 65~83 잔류 (토픽 병합과 직교)

### 1.1.sex 트리거 G — LLM Wiki 흡수 Merge (2026-06-16 사용자 결정 ✅ 확정)

사용자 명시: *"llm 위키처럼 문서 가공 결과가 기존 문서와 관련 있는 경우 기존 문서로 흡수하게 하고 싶어."*

**해석 G1 Merge 확정** — 사용자 단어 *"llm 위키처럼"* + *"흡수"* 의 결합:
- Karpathy 인용 직접 정합: *"the LLM incrementally builds and maintains a persistent wiki — a structured, interlinked collection of markdown files."* — 신규 source가 기존 wiki page로 흡수되어 사라지는 흐름이 Karpathy Wiki의 핵심 동작
- 해석 G2 (Append, wikilink 추가만)는 트리거 E (frontmatter relations) 의 자연 결과로 이미 통합 — 본 트리거 G는 G1 단독 영역
- 해석 G3 (Replace 의미 중복 덮어쓰기)는 §10 Q5 결정과 충돌 — 본 트리거에서 제외
- 해석 G4 (Fragment만 흡수)는 G1의 부분집합 — 본 트리거에 자연 흡수

**본 결정의 본질**:
- **신규 doc이 기존 doc 안으로 사라지는 결정** — Wiki 본질 영역
- *"파일 가공 = .md 1건 생성"* 기존 모델 → *"파일 가공 = 기존 wiki에 흡수 또는 신규 .md 생성"* 양방향 모델
- W2 cascade (인접 doc 본문 갱신, 신규 doc 잔류) 와 동일 LLM 분류 흐름이지만 **분기 결과가 정반대** — 흡수 vs 갱신

**작동 흐름 (W2 cascade 통합 처리, claude 추정 Q3 통합 채택)**:

```
신규 doc 가공 단계 5 (Verify) 통과 후, 단계 7 저장 직전
   │
   ▼
1. IPC: mydocsearch.search.similar(embedding, K=3~5)
   └─ 유사도 상위 K건 후보 페이지 반환
   │
   ▼
2. LLM 관련도 분류 (1회 호출, A1 LLM 캐시 결합)
   "신규 doc이 기존 doc X와 어떤 관계?"
   → {흡수 (G1), 인용 (G2), 갱신 (W2 cascade), 무관} 4분류
   │
   ▼
3. 분기 처리:
   ├─ 흡수 (G1):
   │  └─ 기존 doc 본문에 신규 doc 본문 LLM 통합 (1회 추가 호출)
   │  └─ 기존 doc frontmatter.sources 에 신규 doc 출처 추가
   │  └─ 기존 doc frontmatter.date 갱신
   │  └─ 신규 doc .md 파일 **생성 안 함**
   │  └─ 원본 originals/{stem}.{ext} 잔류 (trace 보존, Q22 결정)
   │  └─ audit_trace + decision_log 기록 ("absorbed_into: {existing_doc_id}")
   │  └─ 사용자 확인 모달 (Q20 claude 추정 (c))
   │
   ├─ 인용 (G2):
   │  └─ 신규 doc 별도 저장 (단계 7 정상 진입)
   │  └─ 기존 doc frontmatter.relations에 양방향 링크 추가
   │  └─ 트리거 E (frontmatter relations) 의 자연 결과
   │
   ├─ 갱신 (W2 cascade):
   │  └─ 신규 doc 별도 저장 + 기존 doc 본문 LLM 부분 갱신
   │  └─ 트리거 B §3.2 W2 영역
   │
   └─ 무관:
      └─ 신규 doc 별도 저장 (단계 7 정상 진입)
```

**Karpathy 정합 정확화**:
- *"the bookkeeping"* 영역의 본질 — 신규 source 도착 시 자동으로 기존 wiki 적절한 위치에 흡수 또는 추가, LLM이 결정
- *"compounds rather than decays"* 영역 — 흡수 시 기존 wiki page가 점진 강화

**영향 범위**:
- `service.rs` 블록 C 단계 7 직전 신규 단계 6.5 (흡수 분기) 추가
- 트리거 E (`RelationOrigin::llm_extracted` 활성) + 트리거 G (`absorbed_into` 신규 audit_trace field) 결합
- `pipeline.toml` `[wiki_absorption]` 섹션 신규 (디폴트 = §10 Q23 결정)
  - `enabled: bool`
  - `k_candidates: u8 = 3` (유사도 상위 K)
  - `require_user_confirm: bool = true` (Q20 claude 추정 (c))
  - `max_absorb_per_doc: u8 = 1` (1건만 흡수, K건 중 최상위)
- 신규 MCP 도구 `wiki_absorb_decision(new_doc_id, target_doc_id, user_choice)` — Q20 (c) 사용자 확인 흐름
- 신규 settings.db 테이블 `wiki_absorption_log` — 흡수 이력 영속화 (decision_log 패턴 흡수)
- GUI 신규 모달 — "🔀 기존 문서로 흡수 제안" (Phase 4 `duplicate_resolution` 모달 패턴 흡수, lesson 65 온보딩 패턴 정합)
- `originals/{stem}.{ext}` 잔류 (Q22 (a) claude 추정) — 흡수된 원본 trace 보존

**위험**:
- 사용자 추적성 영역 — 흡수된 신규 doc 파일 부재 → 사용자가 "이 doc은 어디 갔는가" 추적 어려움 → audit_trace + decision_log + frontmatter `sources` 3중 trace로 완화
- 흡수 오판정 영역 — LLM이 무관한 doc을 흡수 라벨 → 사용자 확인 모달 (Q20 (c)) 로 차단. 디폴트 활성 시 사용자 피로도 위험 (메타 룰 22 후보 — 사용자 정책 경계)
- 기존 doc 본문 LLM 통합 시 회귀 — Verify 6지표 통과한 신규 본문이 통합 후 임계값 위반 가능 → 통합 후 Verify 재실행 의무 (Q24 결정 영역)
- A1 LLM 캐시 결합 효과 미측정 — 동일 (신규 doc 유사도 후보) 쌍 재호출 영역, Phase 212 측정 의무

**보존 영역** (트리거 G가 영향 주지 않는 영역):
- W2 cascade (트리거 B §3.2) — 본 트리거와 동일 LLM 분류 흐름, 다른 분기로 통합 처리
- 트리거 E (frontmatter relations) — G2 (인용) 분기의 자연 결과
- 의미 중복 체크 제거 (§10 Q5) — G3 (Replace) 영역은 본 트리거 미포함
- Fragment 감지 (블록 A 단계 2) — fragment_threshold 미만은 기존 흐름 (handle_fragment), 본 트리거 진입 전 차단

### 1.1.octa 트리거 H — 본질 명칭 재정의 + GitHub origin 신설 (2026-06-16 사용자 결정 ✅ 확정)

사용자 명시: *"file-wiki로 본질 명칭 재정의 진행해. git 주소 변경하고 commit 하자 — file-wiki: https://github.com/reujea/file-wiki.git / file-search: https://github.com/reujea/file-search.git"* + *"생성 했어"* (GitHub 측 빈 저장소 신설 확인, Q25=a 확정)

**본질 명칭 재정의 매핑**:

| 변경 전 | 변경 후 | 근거 |
|---------|---------|------|
| **file-pipeline** | **file-wiki** | Wiki 본질 (트리거 B Karpathy + G 흡수 Merge) 직접 반영. 가공 파이프라인은 Wiki 구축 수단으로 재정의 |
| **mydocsearch** (트리거 D 결정 시 임시 명칭) | **file-search** | file-wiki 측 prefix 일치 + 명료성 우위. lesson 60 내부 코드명 UI 노출 제거 정합 |

**git origin 매핑**:

| 저장소 | 변경 전 | 변경 후 | 빈 저장소 신설 확인 |
|--------|---------|---------|---------------------|
| file-wiki (구 file-pipeline) | `http://gitlab.bi.co.kr/reujea/file.git` | `https://github.com/reujea/file-wiki.git` | ✅ 2026-06-16 사용자 확인 |
| file-search (Phase 211 신설 대상) | (부재 — Phase 211 진입 시 신설) | `https://github.com/reujea/file-search.git` | ✅ 2026-06-16 사용자 확인, Phase 211 진입 시 코드 이관 |

**본 turn 명칭 변경 범위 (Q27 (a) 채택)**:
- ✅ **spec/prd 본문 명칭 갱신** — 본 turn 진행 (시계열 영역은 보존 — lesson 시계열 보존 의무)
- ⏳ **코드 식별자** (`file-pipeline-shared` 등) → Phase 210 진입 시 일괄
- ⏳ **Cargo.toml 패키지명** → Phase 210 진입 시 일괄
- ⏳ **bin name** (`pipeline.exe`, `file-pipeline-tauri.exe`) → Phase 210 진입 시 일괄
- ⏳ **mydocsearch-spec-2026-06-16.md → file-search-spec-2026-06-16.md** rename — 본 turn 진행

**시계열 보존 영역 (명칭 변경 안 함)**:
- `spec/lesson-learned/*.md` 본문의 file-pipeline 표기 — 결정 시점의 명칭 보존 의무
- `spec/architecture-archive.md` — 아카이브 영역
- `spec/architecture.md` §누적 변경 요약의 과거 시점 항목 — 시계열 영역
- 본 통합 문서 §1.1~§1.1.sex (트리거 A~G) + §10 Q9~Q10 + §11 Phase 211 — 결정 시점의 명칭 (file-pipeline / mydocsearch) 보존
- `prd/research/plugin-architecture-2026-06-04.md` + 본 트리거 H 이전 작성 문서 — 결정 시점 보존
- 본 트리거 H 시점 이후의 신규 영역만 file-wiki / file-search 명칭 사용

**시계열 보존 사실 자체의 의미**:
- 메타 룰 19 (단일 진실원 위임) + lesson 시계열 보존 원칙 정합
- 본 통합 문서가 본 세션 결정 과정의 시계열 기록이라 결정 시점 명칭 보존 의무. 명칭 일괄 치환 시 결정 맥락 손실 위험
- file-wiki / file-search 명칭은 트리거 H 시점 이후의 신규 영역 (spec 본문 갱신 + 코드 영역 Phase 210 진입 시) 에 적용

**Phase 210 진입 시 코드 영역 명칭 변경 전수 영역 (claude 추정 채택 후)**:
- Cargo.toml 패키지명: `file-pipeline-core` → `file-wiki-core` 등 (workspace 4 멤버 + Tauri exclude 1건)
- bin name: `pipeline` → `file-wiki` / `file-pipeline-tauri` → `file-wiki-tauri`
- 환경 변수: `PIPELINE_BASE` → `FILE_WIKI_BASE` (lesson 29 패턴)
- 디렉토리 식별자 (`pipeline.log`, `pipeline.toml`): 변경 검토 영역 (Phase 210 진입 시 결정)

**git origin 변경 흐름 (Q26 (a) 채택)**:
1. `git remote set-url origin https://github.com/reujea/file-wiki.git`
2. gitlab 원격 완전 삭제 (mirror 부담 회피, lesson 76 단독 git 첫 설정 시점)
3. 첫 push는 사용자 명시 후 (Q28 (a))

**커밋 전략 (Q3 본 답변 추정 = 단일 커밋)**:
- 본 세션 트리거 A~H 8건 통합 단일 커밋
- 메시지: 본질 명칭 재정의 + 신규 영역 + 결정 누적 + 잔여 결정 대기 표기
- lesson 65 bundle-cycle 패턴 정합

**위험**:
- 명칭 grep 누락 회귀 위험 → 본 turn은 점진 안전 (Q27 (a)), Phase 210 진입 시 일괄 grep + sed 후 수동 검토 (lesson 6 정합)
- 시계열 영역 명칭 변경 시 lesson 시계열 보존 위반 → 본 turn 갱신 영역에서 자동 회피 (lesson-learned/ 디렉토리 + architecture-archive.md + 누적 변경 요약 과거 시점은 grep 제외)
- git origin 변경 후 첫 push 실패 영역 → 빈 저장소 사전 확인 완료 (ls-remote 0줄 통과)
- gitlab 원격 삭제 후 추적성 손실 → file-pipeline.git lesson 76 첫 설정 시점이라 history 가치 낮음, GitHub 측 single source

신규 위험 (디스크 사용량 증가):
- 텍스트 평균 압축률 zstd ~30% → **평문 ~3.3배 증가**
- 5K 코퍼스 (Phase 89 baseline 485 파일 + Phase 86 가정) 환산:
  - 현재: 평균 doc 50KB × 0.3 = 15KB × 5,000 = **75MB**
  - 변경 후: 50KB × 5,000 = **250MB**
  - 증가량 +175MB (1 사용자 데스크톱 도메인 = 무시 가능)
- 사용자 결정 영역 (성능보다 가독성 우위 우선) — 메타 룰 22 (사용자 정책 경계) 누적 사례

### 1.2 트리거 B — Karpathy LLM Wiki 패턴 흡수

원문 (gist):
- **Raw Sources → The Wiki → The Schema** 3-layer
- **Ingest / Query / Lint** 3-operation
- 핵심 통찰: *"지식 베이스 유지의 지루한 부분은 읽기·사고가 아니라 bookkeeping. LLM은 cross-reference 동시 갱신을 잘하므로 decay 대신 compound하는 KB 가능."*

본 프로젝트 정합 진단 (메타 룰 16 차원 B):

| Karpathy 영역 | 본 자산 | 라벨 |
|---|---|---|
| Raw Sources | `originals/` zstd + SHA-256 + TTL | 🟢 완전 |
| Wiki 페이지 | `processed/{type}_{stem}.txt` | 🟡 markdown + wikilink 부재 |
| LLM 메타 | `Metadata` 8필드 | 🟢 완전 |
| Cross-reference | `DocRelation` + `[[wikilink]]` 추출 (Phase 83) | 🟢 인프라, **UI 약함** |
| Index refresh | `crossref_queue` + KG | 🟢 완전 |
| Contradiction flag | `needs_verification` + `lint_strong_claims` | 🟢 완전, **통합 뷰 부재** |
| Stale/Orphan lint | `Linter` 다층 주기 | 🟢 완전 |
| Schema | `pipeline.toml` + `prompts.toml` + `doc_types.toml` + `CLAUDE.md` | 🟢 완전 |
| Query → 영구 페이지 | 부재 | 🔴 신규 |
| Ingest N 페이지 동시 갱신 | N=1 만 | 🟡 부분 (Karpathy N=10~15) |

→ **본 프로젝트는 Wiki 인프라의 70% 이미 보유.** 빠진 핵심은 markdown 출력 / 인접 페이지 cascade / answer 승격 / 통합 health 뷰.

## 2. 변경 후 파이프라인 (트리거 A 적용)

### 2.1 단계 비교

| # | 변경 전 (현 service.rs:241) | 변경 후 | 비고 |
|---|---|---|---|
| **블록 A** | | | |
| 1 | 민감 경로/파일명 판별 | **민감 통합 (경로+본문+키워드+PII 1회)** | merge — `check_sensitive_and_pii` 단일 진입점 강화 (Phase 91 A1' 정합) |
| 2 | 본문 PII + 키워드 | (1과 통합) | |
| 3 | Fragment 감지 | Fragment 감지 | |
| 4 | SHA-256 완전 중복 | SHA-256 완전 중복 | |
| 5 | 증분 해시 (CompileState) | **제거** | |
| **블록 B** | | | |
| 6 | Preprocess | **제거** | LLM 위임 |
| 7 | Chunking (>40KB) | **제거** | LLM 위임 |
| 8 | LLM 분류+가공 | LLM 분류+가공 (markdown + wikilink 출력) | W1 결합 |
| 9 | Verify 6지표 + 2-Pass | Verify 6지표 + 2-Pass | |
| 10 | Embedding (스텝) | **블록 C로 이관** | |
| 11 | Storage (스텝) | **블록 C로 이관** | |
| **블록 C** | | | |
| 12 | Embedding 실행 | Embedding 실행 (스텝 흡수) | |
| 13 | 의미 중복 체크 | **제거** | |
| 14 | 가공본 + 원본 압축 저장 | **가공본 평문 .md 저장 + 원본 평문 그대로 저장** (zstd 제거 — 트리거 C 2차) | 압축 단계 완전 폐기 |
| 15 | .vec + VectorDB upsert | .vec + VectorDB upsert | |
| (신규) 6.5 | — | **흡수 분기 (G1) + cascade 분기 (W2) 통합 LLM 분류** | 트리거 G + 트리거 B §3.2 통합 |
| 16 | 원격 저장소 업로드 | 원격 저장소 업로드 (흡수 doc은 미진입) | |
| 17 | CrossRef 큐 + 엔티티 추출 | **제거** (트리거 E — LLM이 가공 단계에서 frontmatter `relations` + `entities` 직접 작성) | 트리거 E |
| 18 | 증분 기록 → 토픽 병합 | **완전 제거** (트리거 F + 증분 해시 제거) | 트리거 F |

→ **18단계 → 9단계 + 흡수 분기 1** (50% 감소, 트리거 E·F·G 적용)

### 2.2 변경 후 흐름도

```
[블록 A — 사전검사 3단계]
 1. 민감/PII 단일 호출 (경로 + 본문 + 키워드 + 사용자 PII regex)
 2. Fragment 감지 (< fragment_threshold)
 3. SHA-256 완전 중복 (vector_db.find_by_hash)

[블록 B — 파이프라인 스텝 2단계]
 4. LLM 분류+가공 (markdown + frontmatter + 인라인 [[wikilink]])
 5. Verify 6지표 + 2-Pass (실패 시 quarantine)

[블록 C — 후처리 4단계 + 흡수 분기 — 트리거 E·F·G 적용 후]
 6. Embedding 실행 (구조화 입력 + instruction prefix)
    └─ 입력 = frontmatter.summary + keywords + entities (LLM 작성) + 본문

 6.5 ⭐ 흡수 분기 (트리거 G + W2 cascade 통합)  [Phase 211 활성, 디폴트 §10 Q23]
    └─ IPC: mydocsearch.search.similar(embedding, K=3~5)
    └─ LLM 관련도 분류 1회: {흡수 G1, 인용 G2, 갱신 W2, 무관}
    │
    ├─ G1 흡수: 사용자 확인 모달 → 동의 시 기존 doc 본문 LLM 통합
    │           신규 doc .md 미생성, 원본 originals/ 잔류, audit absorb 기록
    │           → 종결 (단계 7~9 미진입)
    │
    ├─ G2 인용: 트리거 E 자연 결과 (frontmatter relations 양방향 링크)
    │           → 단계 7 진입
    │
    ├─ W2 갱신: 인접 페이지 frontmatter 재작성 + embedding 재생성 + IPC upsert
    │           → 단계 7 진입 (신규 doc도 별도 저장)
    │
    └─ 무관: → 단계 7 진입 (기존 흐름)

 7. 가공본 (.md 평문 + frontmatter) + 원본 (평문) 저장 — zstd 미적용
    └─ frontmatter 안에 relations + entities + needs_verification + open_questions
       이미 LLM이 단계 4에서 작성 완료 → 별도 후처리 없음
 8. .vec 영속화 + VectorDB upsert (mydocsearch IPC index.upsert)
 9. 원격 저장소 업로드 (configured 시, .md 직접 업로드)

[⛔ 10. CrossRef 큐 + 엔티티 추출 → 단계 4 LLM 위임으로 흡수 (트리거 E)]
[⛔ 11. 토픽 자동 병합 → 완전 제거 (트리거 F)]
```

## 3. Karpathy 패턴 흡수 4축 (트리거 B)

### 3.1 W1 — Markdown Wiki 출력 형식 전환 (트리거 C와 완전 통합)

**왜**:
- 현 가공본 = `processed/{doc_type}_{stem}.txt.zst` 압축 본문 (service.rs:608-625)
- Karpathy 패턴 = markdown + frontmatter + 본문 안 `[[wikilink]]` → 페이지 간 탐색
- **트리거 C와 동일 산출물** → W1 = 트리거 C 시행 = 단일 변경

**무엇**:
- `processed/{slug}.md` 형식 (zstd 압축 미적용) + YAML frontmatter (`doc_types`, `keywords`, `entities`, `summary`, `hierarchy`, `needs_verification`, `open_questions`, `relations`, `sources`)
- 본문에 LLM이 자연어 안에서 `[[entity_slug]]` 또는 `[[doc_slug]]` 직접 삽입 (Phase 83 `wikilink.rs` 인프라 역방향 활용)
- `prompts.toml` `classify` 갱신 — markdown + 인용 + 인라인 wikilink 형식 지시
- 영향: **Storage 압축 폐기** (.md 직접 저장), MCP `get_document` 응답 markdown 직접 read (decompress_temp 호출 제거)

**frontmatter 직렬화 구조 (예시)**:
```markdown
---
id: doc_2026-06-16_abc123
doc_types: [회의록, 기술문서]
keywords: [파이프라인, Karpathy, Wiki]
summary: 가공 파이프라인 재설계 + Wiki 고도화 통합 결정
date: 2026-06-16
hierarchy: [프로젝트, file-pipeline, Phase 210]
entities:
  - {name: Karpathy, type: person}
  - {name: file-pipeline, type: project}
needs_verification: []
open_questions:
  - W2 cascade의 LLM 비용 실측 필요
relations:
  - {target: doc_xxx, type: similar, origin: auto_similarity}
sources:
  - 트리거 A (사용자 입력 2026-06-16)
  - https://gist.github.com/karpathy/442a6bf555914893e9891c11519de94f
---

# 본문 제목

본문에서 [[Karpathy]]가 제안한 패턴은 [[file-pipeline]]의 ...
```

**호환성**:
- `Metadata` 구조체 변경 0 (frontmatter는 직렬화 산출물)
- 기존 `.txt.zst` 가공본 → Phase 210 진입 시 일괄 markdown 변환 + 압축 해제 마이그레이션 도구 1회
- ServiceBuilder 패턴 덕분에 통합 테스트 변경 0 (lesson 21/27 회피)
- `StoredDoc` 잔류 — VectorDB 측 메타는 frontmatter와 동기 유지 (단일 진실원 = frontmatter, StoredDoc은 캐시·인덱스 영역)

### 3.2 W2 — Ingest Cascade + 흡수 통합 분기 (트리거 G 통합 처리)

⚠ **본 영역은 트리거 G (LLM Wiki 흡수 Merge) 와 동일 LLM 분류 흐름으로 통합 처리됨.** 신규 doc 도착 시 LLM이 인접 K개 페이지와의 관계를 1회 분류 → {흡수 G1 / 인용 G2 / 갱신 W2 / 무관} 4분기. §1.1.sex 참조.

**왜**:
- 현재 신규 doc → 본인 1페이지만 작성, CrossRef 큐는 doc_id push만
- Karpathy 핵심: "인접 페이지 본문이 신규 doc 영향으로 다시 작성"

**무엇**:
- 신규 doc 후처리 신규 단계 (블록 C 10단계 직후) — `vector_db.search_similar(embedding, K=10)` 상위 K 페이지 pull
- LLM 보조 호출 1회 — "신규 doc이 페이지 X에 영향? (보완/모순/중복/연결/무관)" 5분류
- 영향 있는 페이지만 본문 부분 갱신 LLM 위임 (`reprocess_with_feedback` 재사용)
- `RelationType::Semantic(String)` 활성 (Phase 103 G2 인프라, 디폴트 미사용) — "보완/모순/예시" 의미 관계
- `Linter::lint_strong_claims` 결합 — 신규 doc이 기존 strong_claim 약화/강화 시 `needs_verification` 자동 업데이트

**비용 통제**:
- K 디폴트 5~10 (`pipeline.toml.ingest_cascade.k = 5`)
- `ingest_cascade.enabled` 토글 (디폴트 false, lesson 30 인프라 선구현 패턴)
- `ingest_cascade.max_llm_calls_per_doc` 상한 (디폴트 3)
- A1 LLM 캐시 (Phase 31) 자연 결합 — 같은 (신규 doc, 인접 doc) 쌍 재호출 회피

### 3.3 W3 — Query 답변 영구 페이지 승격

**왜**:
- 현 MCP `search` → 결과 일회성 (A1 캐시는 LLM 호출만, answer 페이지화 부재)
- Karpathy 핵심: "가치 있는 답변은 wiki 페이지로 승격되어 다음 query 재사용"

**무엇**:
- 신규 MCP 도구 `promote_to_wiki(query, answer, sources)` — Claude Code 측에서 "이 답변 wiki에 저장" 호출
- `processed/answers/{slug}.md` 신규 카테고리 + frontmatter `kind: answer` + `sources: [doc_id]` 역참조 wikilink
- 다음 search 시 `vector_db.search_hybrid`가 answer 페이지도 후보 — Reranker가 doc/answer 가중치 조정 (`SearchConfig.answer_weight`)
- Phase 86 #6 HyDE 인프라 (hyde_enabled 디폴트 false)와 결합 — answer 페이지가 HyDE 출처 1순위

**비용**:
- 사용자 명시 promote 시만 (자동 없음)
- LLM 호출 0 (answer 텍스트는 사용자/Claude Code가 이미 보유)

### 3.4 W4 — Wiki Health 단일 뷰

**왜**:
- 현재 `LintIssue` + `needs_verification` + `open_questions` + `AnomalyReport` (Phase 92 H1) 각각 별 카드 + 별 탭 분산
- Karpathy 핵심: Wiki Health 한 뷰에서 contradictions/stale/orphans/missing_xref 통합

**무엇**:
- 신규 GUI 탭 (또는 Verification 탭 단일 카드 통합) `🩺 Wiki Health` — 4 sub:
  - **모순**: `needs_verification` ∪ `lint_strong_claims` ∪ `audit_anomaly` 통합 리스트
  - **Orphan**: `DocRelation` in/out=0인 doc
  - **Stale**: `Metadata.date` 기준 6개월+ 미갱신 + 이후 변경된 인접 doc 존재
  - **누락 cross-ref**: entity 동일하지만 wikilink 미연결 후보 쌍 (LLM 보조 1회 판정)
- 신규 MCP 도구 `wiki_health()` — Claude Code에서 직접 호출 가능
- 백엔드는 기존 trait 조합만 (Linter + AnomalyAnalyzer + VectorDBPort + DocRelation)
- Tauri commands 4 신규 + dashboard.js 1 신규 함수 그룹

**비용**:
- 정적 분석 99% (LLM 호출은 누락 cross-ref 후보 LLM 판정 시만, 사용자 명시 시점)

## 4. 본 프로젝트 고유 영역 (보존 의무)

Karpathy gist 부재 영역으로, 본 통합 적용 시 보존:

- **PII / 민감 격리** (블록 A 1단계) — Karpathy는 단일 사용자 unencrypted 가정. 본 프로젝트 차별점
- **Verify 6지표 + 2-Pass** — Karpathy gist는 verification 정량화 미언급
- **Plugin 분리 (Phase 200~209)** — Karpathy는 단일 LLM 가정. 본은 LLM 7 / 임베딩 6 / Storage 5
- **doc_types 17 스키마 + sections 강제** — Karpathy는 free-form wiki, 본은 구조화 검증 우위

→ lesson 45 "도메인 특수성" 패턴 (Notion 직접 구현 사례) 정합

## 5. 통합 영향 매트릭스

| 영역 | 트리거 A 영향 | 트리거 B 영향 | 결합 시너지/충돌 |
|---|---|---|---|
| `service.rs` 본체 | -200줄 (Preprocess/Chunking/CompileState/SemanticDup 제거) | +150줄 (W2 cascade) | A 제거 → B 추가 흡수 여지 |
| `prompts.toml` classify | (변경 없음) | markdown + wikilink 형식 추가 | W1 시너지 — A의 LLM 위임 강화와 정합 |
| `processed/` 형식 | `.txt.zst` → 압축 폐기 (트리거 C) | `.md` 평문 + frontmatter | **트리거 A 압축 해제 + 트리거 B markdown 출력 = 단일 변경** |
| `originals/` 형식 | `.{ext}.zst` → 평문 `.{ext}` (트리거 C) | (변경 없음) | 트리거 C 단독 — 원본 평문 보존 |
| `StoragePort` trait | `compress_and_store` / `decompress_temp` / `read_header` / `delete_expired` / `compress_with_level` 5 메서드 의미 변경 또는 제거 | (변경 없음) | 트리거 C — §10 Q8 결정 영역 |
| `zstd_storage.rs` 어댑터 | 완전 폐기 또는 평문 복사 어댑터로 변환 | (변경 없음) | 트리거 C — §10 Q8 |
| `CompressionConfig` | `zstd_level` dead, `original_ttl_days` 의미 재정의 | (변경 없음) | 트리거 C — TTL 만료 삭제는 평문 파일 그대로 적용 가능 |
| `cross_reference.rs` CrossRef 큐 | 비동기 큐 + 메타블로킹 + MinHash 영역 | KG 영역만 잔류 (mydocsearch 이관), 큐 + 엔티티 추출 폐기 | **트리거 E** — frontmatter `relations` + `entities` 단일 진실원 |
| `topic_merger.rs` | 토픽 자동 클러스터링 + 인접 doc 병합 | 완전 삭제 | **트리거 F** — frontmatter `hierarchy` + `doc_types` 단일 진실원 |
| Entity 추출 (LLM + regex 폴백) | service.rs 697-728 | LLM 단일 (regex 폴백 폐기) | 트리거 E — frontmatter `entities` 직접 작성 |
| `RelationOrigin::llm_extracted` | 인프라 존재, 미사용 | 활성 (LLM frontmatter 작성 시 origin) | 트리거 E 자연 활성 |
| 코퍼스 신호 카운터 (search_mode/crag/chunk_stats) | Phase 80 추천 시스템 의존 | (§10 Q19 결정) | 트리거 F — 토픽 병합 제거에 따른 의존 분기 |
| `PipelineStep` enum | Preprocess/Chunking/Embedding/Storage 4 variant 제거 | (변경 없음) | A 단독 처리 |
| `ChunkedAgentAdapter` | 제거 (또는 LLM 어댑터로 흡수) | (변경 없음) | A 단독 — 단, LLM 분할 위임 시 어댑터 책임 이동 결정 필요 (§10 Q3) |
| `CompileState` 구조체 | 제거 (또는 코드만 보존) | (변경 없음) | §10 Q4 |
| `duplicate_resolution` 포트·어댑터 | 제거 (또는 보존) | (변경 없음) | §10 Q5 |
| `Metadata` 구조체 | (변경 없음) | (변경 없음) | 영향 0 |
| Pipeline 탭 노드 그래프 (Phase 56 18 노드) | 4 노드 제거 → 14 노드 | (변경 없음) | UI 단순화 |
| MCP 도구 수 | (변경 없음) | +2 (`promote_to_wiki`, `wiki_health`) | 25 → 27 |
| Tauri commands | (변경 없음) | +4 (Wiki Health 카드) | 66 → 70 |
| `pipeline.toml` 섹션 | 4 섹션 제거 (preprocessing/chunking/embedding/compression 스텝) | +2 섹션 (ingest_cascade / wiki) | 단순화 + 신규 |
| 통합 테스트 | ServiceBuilder 패턴으로 변경 0 | ServiceBuilder 패턴으로 변경 0 | 메타 룰 1 sub-rule 1b 회피 |
| release 빌드 | 메타 룰 17 의무 (1회) | 메타 룰 17 의무 (1회) | 묶음 시 1회로 통합 |

## 6. 위험 매트릭스

| 위험 | 트리거 | 가능성 | 영향 | 완화 |
|---|---|---|---|---|
| LLM Preprocess 위임 — PDF/한글/Excel 바이너리 LLM 직접 입력 불가 | A | 🔴 확실 | 가공 실패 100% | **§10 Q2** 결정 의무 — (a) 텍스트류 형식 제한 / (b) LLM 어댑터에 추출 책임 이동 / (c) Preprocess는 LLM 호출 직전 흡수 |
| LLM Chunking 위임 — 40KB+ context overflow | A | 🟡 가능 | 큰 문서 실패 | LLM 어댑터 내부 분할 (현 `ChunkedAgentAdapter` 책임 이동) |
| 의미 중복 체크 제거 — 같은 의미 doc 중복 색인 | A | 🟡 가능 | 검색 품질 저하 | SHA 중복만으로 차단, 사용자 결정 (§10 Q5) |
| W1 markdown 출력 — Verify keyword_coverage/rouge_l 측정 변동 | B+A | 🟡 가능 | 검증 임계값 재튜닝 필요 | wikilink/frontmatter는 측정 시 제외 (Verify 입력 전처리 추가) |
| W2 cascade — LLM 호출 폭증 | B | 🟡 가능 | 비용/시간 +30~50% | `enabled` 디폴트 false + `max_llm_calls_per_doc` 상한 + A1 캐시 결합 |
| Pipeline 탭 노드 그래프 4 노드 제거 — UI 회귀 | A | 🟢 낮음 | 사용자 혼란 | dead_selector_scan + action_catalog 자동화 검증 (회귀 자동화 10종) |
| spec 단일 진실원 → 본 통합 변경의 spec 산재 | A+B | 🟡 가능 | 메타 룰 19 위반 | 본 문서가 단일 진실원, spec 갱신 시 본 문서 인용 의무 |
| Plugin 분리 (Phase 200~209) 진행 중 영향 | A+B | 🟡 가능 | 어댑터 plugin 변환 시 충돌 | Phase 200~209 host 잔류 영역 (가공) 정합 — fp-plugin-search/storage/llm 영역과 직교 |

## 7. 기존 자산 활용 매핑

| W축 | 활용 자산 | 출처 |
|---|---|---|
| W1 | `wikilink.rs` `[[xxx]]` 추출 + 한국어/영문 지원 | Phase 83 |
| W1 | `Metadata.hierarchy` + breadcrumb | Phase 61 |
| W1 | `Metadata.needs_verification` + `open_questions` | Phase 87/88 |
| W2 | `RelationType::Semantic(String)` 인프라 (디폴트 미사용) | Phase 103 G2 |
| W2 | `search_similar` + LocalVectorStore HNSW | Phase 45~47 |
| W2 | `reprocess_with_feedback` | service.rs:483 (Verify 재가공 패턴 재사용) |
| W2 | A1 LLM 캐시 | Phase 31 |
| W3 | A1 LLM 캐시 (extension) | Phase 31 |
| W3 | HyDE 인프라 (디폴트 false) | Phase 86 #6 |
| W3 | Reranker (FastEmbed BGE-Reranker-v2-M3) | Phase 62 |
| W4 | `Linter::lint_strong_claims` | Phase 88 |
| W4 | `detect_strong_claims` 12종 마커 | Phase 87 |
| W4 | `audit_anomaly` | Phase 92 H1 |
| W4 | `DocRelation` + `RelationOrigin` 5종 | Phase 83 |
| W4 | 다층 lint 주기 (interval/weekly/monthly) | Phase 89 N-3 |

→ **신규 코드 비중 30% / 기존 자산 결합 70%** — lesson 30/45 "인프라 선구현 + 활성화" 패턴 정합

## 8. 보존 vs 제거 영역 정합 검증

| 영역 | 제거 시 회귀 위험 | 결정 방향 |
|---|---|---|
| Preprocess (PDF/한글/Excel) | 🔴 바이너리 처리 불가 | LLM 어댑터로 책임 이동 (제거 ≠ 폐기) |
| Chunking (>40KB) | 🟡 LLM context overflow | LLM 어댑터 내부 분할 |
| CompileState (.compile_state.json) | 🟢 SHA 중복으로 대체 가능 | 완전 제거 또는 인프라 보존 (§10 Q4) |
| SemanticDup (의미 중복 체크) | 🟡 같은 의미 doc 색인 | 사용자 결정 (§10 Q5) — keep 시 일관성 / 제거 시 단순성 |
| `PipelineStep::Storage`/`Embedding` (스텝) | 🟢 후처리 흡수 시 동등 | 단순 흡수 |

## 9. 비용 추정

| 변경 | per-doc LLM 호출 변동 | per-doc 시간 변동 | 디스크 변동 |
|---|---|---|---|
| 블록 A merge (민감 단계) | 0 | -10ms | 0 |
| 증분 해시 제거 | 0 | -5ms | -compile_state.json |
| Preprocess 제거 → LLM 흡수 | +0~1 (LLM 어댑터 내부) | ±0 (LLM 호출 시간으로 통합) | 0 |
| Chunking 제거 → LLM 흡수 | +0~1 | ±0 | 0 |
| 의미 중복 체크 제거 | 0 | -50ms (search_similar 호출 1회) | 0 |
| W1 markdown 출력 | 0 (prompt 변경만) | ±0 | +frontmatter ~200byte/doc |
| **트리거 C — zstd 폐기** | **0** | **-20ms/doc (압축 시간 제거)** | **+~3.3배 (텍스트 압축률 30%→100%) — 5K 코퍼스 75MB → 250MB** |
| **트리거 E — CrossRef/Entity frontmatter 흡수** | **±0** (단계 4 LLM 출력에 흡수, 별도 LLM 호출 추가 없음) | **-100~300ms/doc** (CrossRef 큐 처리 시간 제거) | -DocRelation 별도 영속 (frontmatter 통합) |
| **트리거 F — 토픽 자동 병합 제거** | **0** | **-50~150ms/doc** (토픽 클러스터링 시간 제거) | -코퍼스 신호 카운터 3 테이블 (Q19 결정 영역) |
| **트리거 G — 흡수 분기 (디폴트 활성, Q23 (b) 채택 가정)** | **+1 분류 + 0~1 통합** (흡수 시점만 +1, 평균 +1.2 LLM/doc) | **+500~1500ms/doc** (LLM 분류 + 사용자 확인 모달 + IPC search.similar) | **-흡수된 신규 doc .md 미생성** (평균 10~20% doc) |
| **합계 (디폴트 모든 트리거 활성, K=3, 모든 결정 claude 추정 채택)** | **+1.2 LLM/doc** | **+200~1000ms/doc** | **-원본 압축 + +frontmatter + -흡수 doc** (5K 코퍼스 250MB → 200~220MB 추정) |
| W2 cascade (디폴트 false) | 0 | 0 | 0 |
| W2 cascade (활성, K=5) | +3~5 (영향 페이지만) | +1.5~2.5s/doc (LLM 호출 시간) | 인접 페이지 본문 재기록 |
| W3 promote_to_wiki | 0 (사용자 명시 시만) | 0 | +answer page |
| W4 wiki_health 통합 뷰 | 0 (정적 분석 99%) | 0 | 0 |
| **합계 (디폴트, W2 비활성)** | **±0** | **-65ms** | **거의 0** |
| **합계 (W2 활성, K=5)** | **+3~5** | **+1.5~2.5s** | **인접 페이지 갱신** |

→ **디폴트 비활성 시 회귀 없음, 활성 시 사용자 명시 결정** (lesson 30 패턴)

## 10. 의사결정 매트릭스 (사용자 결정 트리거)

### Q1. 블록 A 1·2단계 "mere" 해석
- (a) **merge** — 1+2 단계 통합 (1회 호출로 경로+파일명+본문+키워드+PII)
- (b) **remove** — 1·2단계 전체 제거 (위험: PII 격리 흐름 무력화)
- claude 추정: **(a) merge** (§1.1 본문에서 추정 표기)

### Q2. Preprocess 제거 — 바이너리 파일 처리
- (a) **LLM 어댑터로 추출 책임 이동** — `ClaudeCliAdapter` 등이 파일 직접 읽기 + 추출 (현 `classify_and_process(file_path)` 흐름 강화)
- (b) **파일 형식 텍스트류 제한** — PDF/한글/Excel 가공 자체 폐기
- (c) **Preprocess는 LLM 호출 직전으로 흡수** — 단계 표기만 제거, 실 호출은 보존
- claude 추정: **(a)** + plugin 분리 시 fp-plugin-llm-* 어댑터 단위로 자연 흡수

### Q3. Chunking 제거 — context overflow 대응
- (a) **LLM 어댑터 내부 분할** — `ChunkedAgentAdapter` 책임을 각 LLM 어댑터로 이동
- (b) **단순 텍스트 1회 전달** — overflow 시 LLM 측 에러 → quarantine 라우팅
- claude 추정: **(a)** — lesson 9 키워드 오염 회피 + Verify 2-Pass 정합

### Q4. CompileState 제거 범위
- (a) **완전 제거** — 구조체 + `.compile_state.json` + 관련 메서드
- (b) **인프라 보존, 호출만 제거** — 향후 재활용 여지
- claude 추정: **(a)** — 본 프로젝트 dead code 누적 회피 정책 (lesson 13 19+ 47)

### Q5. 의미 중복 체크 제거 범위
- (a) **완전 제거** — `duplicate_resolution` 포트·어댑터·`semantic_dup_threshold` 설정·UI 일괄
- (b) **포트·인프라 보존, 호출만 제거**
- claude 추정: **(b)** — `DuplicateAction` 사용자 의사결정 흐름은 향후 W2 cascade에서 "중복" 분류 시 재활용 여지

### Q6. Karpathy Wiki 4축 진행 옵션
- **옵션 A 점진 안전**: W1+W4만, LLM 비용 0 증가, 가치 60% 도달
- **옵션 B Wiki 본진입**: W1+W2+W3+W4, LLM 비용 +30~50%, 가치 100%
- **옵션 C 인프라 선구현** (lesson 30 패턴): W1+W4 즉시 + W2/W3 placeholder + 코퍼스 신호 후 활성화
- claude 추정: **옵션 C** — 본 프로젝트 lesson 30 / 45 / 86 / 103 4회 검증된 안전 경로

### Q7. 트리거 A + B 단일 Phase 묶음 vs 분리
- (a) **단일 Phase (예: Phase 210)** — release 빌드 1회, 회귀 자동화 1회, spec 갱신 1회
- (b) **2 Phase 분리** — Phase 210 (트리거 A) → Phase 211 (트리거 B)
- claude 추정: **(a) 단일** — 시너지 영역 (Preprocess 제거 + W1 markdown 출력 + 트리거 C zstd 폐기) 자연 정합, lesson 65 bundle-cycle 패턴 정합

### Q8. zstd 압축 단계 폐기 — `zstd_storage.rs` 어댑터 처리 (트리거 C 후속)
- (a) **어댑터 완전 폐기** — `StoragePort` trait 5 메서드 모두 제거 + `zstd_storage.rs` 삭제 + `CompressionConfig` 폐기 + Pipeline 탭 Storage 노드 제거
- (b) **평문 복사 어댑터로 변환** — `StoragePort` trait 유지하되 `compress_and_store` = 단순 파일 복사 / `decompress_temp` = 동일 경로 반환 (인프라 보존, 향후 압축 재도입 여지)
- (c) **trait 유지, zstd 어댑터 dead** — 어댑터는 보존하되 호출 흐름만 제거 (dead_code 회귀 위험)
- claude 추정: **(a) 완전 폐기** — lesson 13/19+/47 dead code 누적 회피 정책 + 사용자 명시 "zst 압축 단계 제거" 의도 정합. `originals/` TTL 만료 삭제는 평문 파일 직접 `std::fs::remove_file`로 대체 (메서드 1개로 충분)

### Q9. 트리거 D — 신규 저장소 위치 + Phase 분할 (해석 B 후속)

#### Q9a. 신규 저장소 위치·이름
- (a) `C:\dev\claude_workspaces\mydocsearch\` (사용자 도메인 이름 — 2026-04-08 무효화된 mydocsearch_decision.md와 동일 이름)
- (b) `C:\dev\claude_workspaces\file-search\` (file-pipeline과 prefix 일치)
- (c) `C:\dev\claude_workspaces\fp-search\` (fp-plugin-search 약어)
- (d) 사용자 명시 결정 대기 (다른 이름)
- claude 추정: **(a) mydocsearch** — 사용자가 한 번 채택한 이름 + 본질 도메인 일치 (개인 문서 검색). 단, 2026-04-08 결정 무효화 사실 (단일 저장 결정의 폐기)과 본 결정 (단독 저장소 신설)은 완전 다른 결정 — 이름 재사용 가능

#### Q9b. Phase 분할
- (a) **단일 Phase 210** — 트리거 A+B+C+D 모두 단일 묶음
- (b) **2 Phase 분리** — Phase 210 (A+B+C) → Phase 211 (D)
- claude 추정: **(b) 분리** — 위 §1.1.ter 영역 결정 정합. 트리거 A+B+C는 가공 본체 영역, 트리거 D는 plugin 본진입 + 저장소 경계 영역으로 직교. 회귀 추적 용이

### Q10. 트리거 D — 검색 이관 범위 (D-5 후속)

#### Q10a. 영역 이관 범위
- (a) **보수 (D-5a)**: LocalVectorStore + mmap + HNSW + vec_io + MMR/sparse/RRF만 이관. Reranker / Embedding / KG는 host 잔류
- (b) **중간 (D-5b)**: + Reranker + KG 추가 이관. Embedding은 가공도 사용해서 host 잔류 + IPC 노출
- (c) **광범위 (D-5c)**: + Embedding 어댑터 6종 + MinHash 모두 이관. mydocsearch는 가공도 자체 보유 (file-pipeline 없이 단독 가공·검색 가능)
- claude 추정: **(b) 중간** — 검색 본질 의존 (Reranker, KG)은 자연 이관, Embedding은 가공도 사용해서 분리 시 양쪽 의존 → 단일 진실원 위반. 광범위는 mydocsearch가 file-pipeline 본질 흡수 → 도메인 경계 흐림

#### Q10b. Embedding 어댑터 처리 (Phase 207 정합)
- (a) **host 잔류 + IPC 인터페이스 노출** — file-pipeline이 가공 시 embedding 호출, mydocsearch가 색인·검색 시 IPC로 file-pipeline 측 embedding 호출
- (b) **양쪽 보유 (중복)** — file-pipeline + mydocsearch 각각 embedding 어댑터 보유. fp-plugin-embedding-* (Phase 207)이 두 워크스페이스에서 공유 plugin으로 동작
- (c) **mydocsearch 단독 보유** — file-pipeline은 IPC로 mydocsearch 측 embedding 호출 (가공 의존 IPC 1회 추가, 가공 속도 영향 +5~10ms/doc)
- claude 추정: **(b) 양쪽 보유** — Phase 207 fp-plugin-embedding-* (24 어댑터 → plugin) 결정 정합. plugin은 양 워크스페이스에서 공유. 단일 진실원 위반은 plugin 영역 자체가 두 저장소 외부의 `_rust_module/`에 잔류하므로 회피

#### Q10c. Phase 203 placeholder (`_rust_module/fp-plugin-search/`) 처리
- (a) **삭제 + mydocsearch.git로 재구성** — `_rust_module/`에서 빠짐, 신규 저장소 단독
- (b) **잔류 + thin wrapper** — `_rust_module/fp-plugin-search/`는 mydocsearch crate 의존하는 plugin 진입점 binary만 보유
- claude 추정: **(b) 잔류 + thin wrapper** — Phase 200~209 plugin 패턴 정합 (모든 plugin이 `_rust_module/` 잔류). mydocsearch는 본체, `fp-plugin-search`는 plugin 어댑터 (~50줄 main.rs)

### Q19. 트리거 F — 코퍼스 신호 카운터 (Phase 80 추천 의존) 처리
- (a) **카운터 보존, 토픽 병합 호출만 제거** — search_mode_counters / crag_counters / chunk_stats 잔류. setup_modules 추천 시스템 (Phase 65~83) 의존 영역 보호
- (b) **카운터 완전 제거** — 토픽 병합과 함께 추천 시스템 5축 영역도 dead code (lesson 13 회피)
- (c) **추천 시스템 자체를 plugin 이관 (fp-plugin-recommend)** — Phase 207 영역 확장 (host 본질 = 파일 가공만 정합)
- claude 추정: **(a)** — 추천 시스템은 사용자 GUI 진입점 보유 (Phase 100 운영 카드 + Phase 102 optimize MCP 도구), host 본질 잔류 결정 영역. 카운터 보존이 회귀 위험 낮음

### Q20. 트리거 G — 흡수 분류 임계값
- (a) **LLM 분류 결과 단독** — LLM이 "흡수" 라벨 반환 시 즉시 흡수
- (b) **유사도 점수 + LLM 양쪽 동의** — embedding 유사도 > 0.9 AND LLM "흡수" 라벨
- (c) **사용자 확인** — LLM "흡수" 라벨 시 GUI 모달로 사용자 결정 (Phase 4 `duplicate_resolution` 패턴 재사용)
- claude 추정: **(c)** — 신규 doc 사라지는 결정은 사용자 추적성 위험 영역 + 본 솔루션의 `duplicate_resolution` 사용자 의사결정 흐름 정합 (lesson 30 Ruflo "제안만" 패턴 정합)

### Q21. 트리거 G — 흡수 시점
- (a) **단계 5 Verify 직후, 단계 7 저장 직전** — 신규 doc 본문이 검증 완료된 상태에서 흡수 판정
- (b) **단계 7 저장 직후, 단계 8 IPC upsert 직전** — 신규 doc 저장 후 흡수 시 기존 doc 덮어쓰기
- (c) **W2 cascade 영역 안 분기** — Phase 212 활성 시점에서만 작동
- claude 추정: **(a)** — 저장 전 결정이 회귀 위험 낮음 (저장 후 삭제 흐름은 race condition 영역). 단계 6.5 신규 위치

### Q22. 트리거 G — 흡수 시 원본 처리
- (a) **`originals/{stem}.{ext}` 잔류** — trace 보존
- (b) **`absorbed/{stem}.{ext}` 별도 폴더 이동** — 흡수 영역 명시 분리
- (c) **삭제** — 디스크 절약 (사용자 추적성 위험)
- claude 추정: **(a)** — 추적성 보존 의무. (b)는 별도 폴더 신설 영역이라 (a) 권장. 흡수 trace는 audit_trace + decision_log + frontmatter `sources` 3중

### Q23. 트리거 G — 디폴트 활성 여부
- (a) **디폴트 false** — lesson 30 패턴, 코퍼스 50건+ 후 활성
- (b) **디폴트 true + 사용자 확인 모달** — 사용자 명시 *"하고 싶어"* = 디폴트 활성 의도. Q20 (c) 사용자 결정 흐름 결합
- (c) **디폴트 false + GUI 첫 진입 시 활성 가이드** — Phase 106 온보딩 패턴
- claude 추정: **(b)** — 사용자 톤 시그널 우선 + 사용자 확인 모달로 추적성 보존 양립

### Q24. 트리거 G — 흡수 후 Verify 재실행 여부
- (a) **흡수 후 기존 doc 전체 Verify 6지표 재실행** — 임계값 위반 시 흡수 롤백
- (b) **신규 영역만 Verify** — 흡수된 본문 영역 단독 검증
- (c) **재검증 안 함** — LLM 통합이 신규 본문 의미 보존 가정
- claude 추정: **(a)** — Verify 6지표 회귀 차단 의무. 흡수 후 임계값 위반 시 G2 인용 분기로 자동 fallback

## 11. Phase 분할안 (Q6 옵션 C + Q7 (a) 채택 가정)

### Phase 210 — 가공 파이프라인 재설계 + Wiki W1+W4 즉시 (단일 묶음)

#### B1 — 블록 A 정리 (단순 제거)
- 민감 1·2단계 merge — `check_sensitive_and_pii` 단일 호출 정형화
- CompileState 완전 제거 — 구조체 + 메서드 + `.compile_state.json` 일괄
- 단위 테스트: 민감 통합 호출 + SHA 중복 단독 차단 검증

#### B2 — 블록 B 단순화 (Preprocess/Chunking LLM 위임)
- `PipelineStep::Preprocess` / `PipelineStep::Chunking` enum variant 제거
- `ChunkedAgentAdapter` 책임 → `ClaudeCliAdapter` 등 각 LLM 어댑터 내부 이동
- `preprocessing` 어댑터 → LLM 어댑터 호출 시점 흡수 (어댑터 자체는 fp-plugin-llm-* 변환 대기 영역)
- Pipeline 탭 노드 그래프 18 → 14 노드 (UI 갱신)

#### B3 — 블록 C 정리 (스텝 흡수 + 의미 중복 제거 + zstd 폐기 — 트리거 C)
- `PipelineStep::Embedding` / `PipelineStep::Storage` 후처리 흡수 → 동시에 Storage 노드 자체 제거 (트리거 C Q8 (a) 채택 가정)
- 의미 중복 체크 호출만 제거 (포트·인프라 보존, Q5 (b) 채택 가정)
- **zstd 압축 단계 완전 폐기** (트리거 C, Q8 (a)):
  - `StoragePort` trait 5 메서드 제거 → 신규 `delete_original_after_ttl(path) -> Result<()>` 1 메서드만 (TTL 영역 보존)
  - `zstd_storage.rs` 어댑터 삭제
  - `CompressionConfig` → `RetentionConfig` 명칭 변경 + `zstd_level` 필드 제거 (`original_ttl_days`만 잔류)
  - `service.rs:608-625`: `temp_processed` 생성 → `compress_and_store` → 임시 삭제 흐름 폐기, 단일 `std::fs::write(processed_dir/{slug}.md, frontmatter + content)`
  - `service.rs:624`: 원본 압축 → 원본 평문 이동 (`std::fs::rename(file_path, originals_dir/{stem}.{ext})`)
  - `wiki_export.rs / topic_merger.rs / lint.rs / cross_reference.rs / auto_reindexer.rs` — `decompress_temp` 호출 → 직접 `std::fs::read_to_string` 변환
- Pipeline 탭 노드 그래프: 18 → 14 (B2) → **13 (Storage 노드 제거)**
- 통합 테스트: ServiceBuilder 패턴 — 변경 0 + Storage 트레이트 mock 5 메서드 → 1 메서드로 단순화

#### B4 — W1 markdown 출력 형식 (트리거 C와 통합 시행)
- `prompts.toml` `classify` 갱신 — markdown + frontmatter + 인라인 wikilink 지시
- `service.rs` 가공본 저장 시 `.md` 확장자 + frontmatter YAML 직렬화
  - frontmatter 직렬화: `serde_yaml::to_string(&FrontMatter)` + `---\n{yaml}---\n\n{content}` 결합
  - `FrontMatter` 구조체 신규 — `Metadata` from 변환 (직렬화 산출물, 영속 별도 X)
- 기존 `.txt.zst` 가공본 일괄 변환 마이그레이션 도구 (1회 실행, 결과 보고):
  - 입력: `processed/*.txt.zst` (전체)
  - 동작: zstd 해제 → frontmatter 직렬화 → `.md` 저장 → 원본 `.txt.zst` 삭제
  - 결과 보고: 변환 N건 / 실패 M건 / 디스크 변동 +K MB
  - 동시에 `originals/*.{ext}.zst` → 원본 평문 복원 (TTL 미만료 시만)
- MCP `get_document` 응답 markdown 직접 read (decompress 호출 제거)
- 회귀 자동화: `dead_selector_scan` + `action_catalog` 갱신 + 신규 `frontmatter_lint.sh` (YAML 파싱 + 필수 필드 검사)

#### B5 — W4 Wiki Health 통합 뷰 + 트리거 G 흡수 분기 인프라
- 신규 MCP 도구 `wiki_health()` — `Linter` + `AnomalyAnalyzer` + `VectorDBPort` 조합
- Tauri commands 4 신규 (모순/orphan/stale/누락 cross-ref)
- dashboard.js Wiki Health 카드 (Verification 탭 통합 또는 신규 탭)
- 회귀 자동화: dead_selector_scan + action_catalog 갱신
- **트리거 G 흡수 분기 인프라** (단계 6.5):
  - `service.rs` 단계 6.5 분기 추가 — IPC search.similar + LLM 4분류
  - `pipeline.toml` `[wiki_absorption]` 섹션 신규 (디폴트 §10 Q23 (b) true)
  - 신규 MCP 도구 `wiki_absorb_decision(new_doc_id, target_doc_id, user_choice)`
  - 신규 settings.db 테이블 `wiki_absorption_log` (decision_log 패턴)
  - 신규 GUI 모달 "🔀 기존 문서로 흡수 제안" (Phase 4 `duplicate_resolution` 모달 패턴 흡수)
  - Tauri commands +2 (`propose_absorption` + `confirm_absorption`)
  - 흡수 후 Verify 재실행 분기 (Q24 (a)) — 임계값 위반 시 G2 fallback

#### B6 — W2/W3 placeholder + 회귀 자동화 + release
- `pipeline.toml` `[ingest_cascade]` + `[wiki]` 섹션 신규 (디폴트 비활성)
- `ingest_cascade.enabled` 토글 인프라 (호출 흐름은 placeholder)
- `promote_to_wiki` MCP 도구 placeholder (200 OK, 로직 비활성)
- 회귀 자동화 10종 baseline 측정 (Phase 210 진입 전 vs 종결 후)
- release 재빌드 + D:\file-test 배포 (메타 룰 17 의무)

### Phase 211 — 트리거 D 본진입 (검색 영역 mydocsearch 분리) ⭐ 신규

조건: Phase 210 종결 + 트리거 D 5축 결정 확정 (§10 Q9·Q10)

#### C1 — 신규 저장소 + 워크스페이스 신설
- `mydocsearch/` 신규 git init + workspace Cargo.toml + lib + bin 듀얼 target
- README.md (단독 사용 + plugin 통합 양 모드 명시)
- LICENSE / .gitignore / Cargo.lock 신규
- CI 신설 (gitlab.bi.co.kr 또는 GitHub) — 추정 빗나감 회피 위해 사용자 결정 (§13 Q5)

#### C2 — 검색 영역 코드 이관 (D-5b 중간 채택 가정)
- `file-pipeline/src/crates/adapters/src/driven/vector_db/` → `mydocsearch/src/vector_db/`
  - LocalVectorStore + mmap + HNSW + vec_io + MinHash (메타블로킹은 CrossRef 잔류 결정 영역)
- `file-pipeline/src/crates/adapters/src/driven/reranking/` → `mydocsearch/src/reranking/`
  - FastEmbedReranker + ClaudeReranker + NullReranker
- `file-pipeline/src/crates/core/src/domain/cross_reference.rs` 중 KG 영역 → `mydocsearch/src/kg/`
  - kg_neighbors / kg_paths / DocRelation 영역
- 통합 테스트도 이관 (ServiceBuilder 패턴 변경 0)
- git history 보존: `git filter-repo` 또는 `git subtree split` (§13 Q6)

#### C3 — IPC 인터페이스 정의 + thin wrapper plugin
- `_rust_module/fp-plugin-search/` 갱신 — Cargo.toml에 `mydocsearch` git 의존 추가 (또는 path 의존 단계적)
- `fp-plugin-search/src/main.rs` ~50줄 thin wrapper — `mydocsearch::search()` 등 호출 + `fp-plugin-sdk::run::<P>()`
- `mydocsearch/src/plugin_entry.rs` (선택) — plugin 모드 진입점, 단독 모드와 직교

#### C4 — file-pipeline 측 정리
- `VectorDBPort` / `RerankerPort` 호출 흐름은 그대로 (인터페이스 유지)
- 어댑터는 IPC 경유 (plugin call) 또는 단계적 (path 의존 → IPC 전환)
- `spec/architecture.md` §누적 변경 요약 Phase 211 항목 신규
- `spec/domain-map.md` §도메인 2 (검색) — "mydocsearch 이관 완료" 표 갱신
- `spec/deprecated.md` — file-pipeline 측 검색 어댑터 위임 표기

#### C5 — 단독 실행 검증
- `mydocsearch` 단독 CLI 진입점 검증 — file-pipeline 없이도 색인·검색 가능
- 단독 MCP server 진입점 (선택) — Claude Code가 mydocsearch만 등록해도 search 가능
- 통합 테스트: file-pipeline 측 색인 → mydocsearch 측 검색 양방향 (IPC 통과)

#### C6 — 회귀 자동화 + 릴리즈
- 회귀 자동화 10종 baseline 측정 (Phase 211 진입 전 vs 종결 후)
- mydocsearch.git CI baseline 측정 (별도)
- release 재빌드 + D:\file-test 배포 (메타 룰 17 의무) — 양 저장소

### Phase 212 — W2 활성화 (트리거 대기)

조건:
- Phase 211 종결 + 코퍼스 50건+ 누적
- 사용자 W2 활성 명시 결정

내용:
- `ingest_cascade.enabled = true` 디폴트 변경
- `RelationType::Semantic(String)` 활성 — "보완/모순/예시" 의미 관계 부착
- A1 LLM 캐시 결합 효과 측정 (3회 중앙값, 메타 룰 4)
- IPC 경유 `search_similar` 호출 비용 실측 (+10~30ms/호출 추정 검증)

### Phase 213 — W3 활성화 (사용자 신호 후)

조건:
- 사용자 search 활용 신호 누적 (예: 같은 query 3회+ 반복)
- 또는 사용자 promote_to_wiki 명시 호출

내용:
- `promote_to_wiki` 로직 활성 — `processed/answers/{slug}.md` 생성 (file-pipeline 측 또는 mydocsearch 측 — §13 Q7)
- `SearchConfig.answer_weight` 노출 (Pipeline 검색 노드)
- Reranker가 answer 페이지 가중치 조정

## 12. 본 문서 → spec 갱신 흐름 (Phase 210 진입 시점)

본 통합 방안 사용자 결정 후:

1. **spec/architecture.md §누적 변경 요약** — Phase 210 항목 신규 (트리거 A+B 요약 + 본 문서 인용)
2. **spec/domain-map.md** — §도메인 1 (문서 처리) "파이프라인 스텝" 표 갱신 (18 → 11단계)
3. **spec/scenarios.md** — 시나리오 2 (파일 투입 → 가공 → 검색) 14단계 → 11단계 갱신
4. **spec/classification_and_verification.md** — Verify 입력 전처리 (frontmatter/wikilink 제외) 추가
5. **spec/deprecated.md** — Preprocess 스텝 / CompileState / SemanticDup (Q5 결정에 따라) 등재
6. **spec/lesson-learned/N_phase210-*.md** — Phase 210 종결 lesson 등재 (메타 룰 30 자기 적용)
7. **spec/lesson-learned/META.md** — 본 통합 사례를 메타 룰 16 차원 B 활용 누적 사례로 등재
8. **prd/research/karpathy-llm-wiki-2026-06-16.md** — Karpathy gist 원문 분석 (외부 분석 단일 진실원 패턴, §10 Q 외 추가 결정)

## 13. 사용자 다음 결정 요청 (요약)

§10 Q1~Q7 결정 → §11 Phase 분할안 확정 → §12 spec 갱신 흐름 진입.

claude 추정 일괄 채택 시:
- Q1: merge / Q2: LLM 어댑터 추출 / Q3: LLM 어댑터 분할 / Q4: CompileState 완전 제거 / Q5: SemanticDup 포트 보존 / Q6: 옵션 C / Q7: Phase 210 단일 (A+B+C+E+F+G) / **Q8: zstd 어댑터 완전 폐기 (트리거 C 확정)** / **Q9a: mydocsearch / Q9b: Phase 211 분리 (트리거 D)** / **Q10a: D-5b 중간 / Q10b: Embedding 양쪽 보유 / Q10c: thin wrapper 잔류** / **Q19: 카운터 보존 (트리거 F)** / **Q20: 사용자 확인 / Q21: 단계 6.5 / Q22: originals 잔류 / Q23: 디폴트 true + 사용자 모달 / Q24: 흡수 후 Verify 재실행 (트리거 G)**

확정 결정 (2026-06-16, 본 세션):
- ✅ **트리거 C-1**: 가공본 `.md` 평문 + frontmatter (사용자 명시)
- ✅ **트리거 C-2**: zstd 압축 단계 완전 제거 (사용자 명시 — 원본 포함)
- ✅ **트리거 D**: 검색 영역 완전 별도 git 저장소 분리 (해석 B, 사용자 명시) — 5축 결정은 §10 Q9·Q10 사용자 결정 대기
- ✅ **트리거 E**: CrossRef + Entity → frontmatter 흡수 (사용자 명시)
- ✅ **트리거 F**: 토픽 자동 병합 완전 제거 (사용자 명시) — Q19 처리 분기 대기
- ✅ **트리거 G**: LLM Wiki 흡수 Merge — 신규 doc이 기존 doc로 사라지는 흐름 (사용자 명시 "llm 위키처럼... 흡수") — Q20~Q24 결정 대기

→ Phase 210 진입 가능 (B1~B6 6 묶음 직렬 사이클, bundle-cycle 스킬 정합) + Phase 211 진입 가능 (C1~C6 6 묶음 직렬 사이클).

### 13.1 트리거 D 추가 결정 대기 (해석 B 5축 외)

#### Q5. mydocsearch.git CI 위치
- (a) gitlab.bi.co.kr (lesson 76 file-pipeline.git 정합)
- (b) GitHub (별도)
- (c) 미설정 (로컬 git만)
- claude 추정: **(a)** — 동일 환경 정합

#### Q6. file-pipeline.git history 보존
- (a) git filter-repo로 검색 영역만 추출 후 mydocsearch.git에 import (history 보존)
- (b) git subtree split (history 부분 보존)
- (c) history 없이 단순 복사 + 신규 commit (history 폐기)
- claude 추정: **(c)** — file-pipeline.git이 2026-06-10 신설 (lesson 76)이라 history 가치 낮음. 단순 복사가 비용 최저

#### Q7. promote_to_wiki 위치 (W3 영역)
- (a) file-pipeline 측 (`processed/answers/{slug}.md` — 가공본 영역 정합)
- (b) mydocsearch 측 (검색 영역 정합)
- claude 추정: **(a)** — answer 페이지는 가공본의 부분집합 (자연어 답변), 검색은 단순 색인 대상으로 잔류
