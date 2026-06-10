---
phase: 89
date: 2026-05-18
topics: lint 다층 주기 / Metadata 보조 필드 UI / 포트 메서드 디폴트 노출
related_lessons: 14, 30, 39, 42, 40
related_meta_rules: 1, 5강화, 9
---

# Phase 89 — lint 다층 주기 활성 + 보조 필드 UI 노출

> Phase 88 완성에서 미연결로 표시되었던 N-3 / N-4를 한 phase에 묶어 완성. Phase 87 인프라(detect_strong_claims + lint_weekly_hours + lint_monthly_hours)의 schedule task 호출, Metadata 보조 필드 2종의 frontend 노출. lesson 42의 "lesson 14 dead 자산 3단계(인프라/활성화/측정)" 후속 phase. N-2가 활성화 단계라면 본 phase는 **사용자 노출 단계**.

## 상황

Phase 87이 인프라(필드/함수/configField)만 추가하고, Phase 88이 LLM 활성화 + 측정까지 완료한 직후 상태. 잔여:
- N-3: lint_weekly_hours / lint_monthly_hours 필드만 있고 service.rs의 schedule task 분기에 호출 없음 — lesson 14 패턴(미연결 포트와 동형)
- N-4: Metadata.needs_verification / open_questions가 LLM에 채워지고 영속화는 되지만 UI에 노출 없음. lint_strong_claims 결과도 Tauri command 없음

권장 우선순위 1단계로 Phase 89 묶음 진행.

## 문제

1. **포트 메서드 추가 위치**: get_document Tauri command에서 needs_verification / open_questions 접근하려면 vector_db에서 풀 메타 조회가 필요한데 VectorDBPort에 노출된 게 없음. StoredDocSummary는 요약본만 보유. 어댑터 내부 잠금(documents Mutex)을 깨지 않고 외부에 풀 메타를 노출하는 패턴 선택 필요
2. **wikidocs 353407 매핑과 함수 매핑 불일치**: 외부 문서는 weekly = "중복·미연결", monthly = "오래된·상충"으로 정의하지만 본 프로젝트엔 그 정확한 함수가 없음. lint_strong_claims(품질 검토)와 lint_topics(모순 검사) 중 어디에 매핑할지
3. **dead 함수와 일관성**: modals/app/src/service.rs의 `start_background_tasks` 함수가 `#[allow(dead_code)]` 상태로 standalone과 분기. 일관성을 위해 같이 갱신할지, 그대로 둘지

## 원인

1. lesson 1 메타 룰 1(다중 위치 동기화)을 회피하려고 포트에 풀 객체 노출을 피해온 결과 — list_all=Summary, find_related=관계, 메타 직접 접근 부재. N-4 시점에서 결국 노출 필요
2. wikidocs 353407 매핑이 본 프로젝트 함수 셋과 정확히 1:1이 아님. 기능 카테고리(품질 의심 / 토픽 모순)로 재해석 필요
3. start_background_tasks 본문이 #[allow(dead_code)]인 이유는 standalone에서 같은 로직을 별도 BackgroundRef 패턴으로 운영하기 때문. 두 함수 모두 lint 분기를 갖는다는 일관성 자체는 유지 가치 있음

## 개선

### N-3: lint 다층 주기 schedule task 연결 (3곳 동기화)

3진입점 모두 weekly + monthly 분기 추가:
1. `modals/app/src/service.rs::start_background_tasks_standalone` — Tauri standalone (활성)
2. `modals/app/src/service.rs::start_background_tasks` — dead_code 일관성 유지
3. `modals/cli/src/main.rs::pipeline start` — CLI 모드

매핑 결정:
- `lint_interval_hours` (6h 디폴트) → `Linter::lint` (orphan/missing backlink/유형 없음)
- `lint_weekly_hours` (168h 디폴트) → `Linter::lint_strong_claims(vector_db, storage, 5)` — wikidocs 353407 "주 1회 중복·미연결"의 변형. 본 프로젝트엔 정확한 중복 검사 함수가 없으므로 품질 의심 카테고리로 매핑
- `lint_monthly_hours` (720h 디폴트) → `Linter::lint_topics(&topics_dir)` — "월 1회 오래된·상충"에 해당하는 토픽 모순 마크 검사

모두 0=비활성으로 토글 가능. 메타 룰 5강화의 3요소(config + 분기 + no-op) 충족.

### N-4: Metadata 보조 필드 UI 노출 (5계층 동기화)

신규 메서드 + 5계층 갱신:

| 계층 | 파일 | 변경 |
|------|------|------|
| 포트 trait | `core/ports/output.rs::VectorDBPort` | `fn get_metadata(doc_id) -> Result<Option<Metadata>>` 디폴트 None |
| 어댑터 override | `adapters/.../local_store.rs` | StoredDoc의 doc_types/date/summary/keywords/needs_verification/open_questions를 Metadata로 매핑 |
| Tauri command | `modals/app/src/commands.rs::get_document` | 응답에 needs_verification / open_questions / summary / keywords 추가 |
| Tauri command 신규 | `modals/app/src/commands.rs::get_lint_strong_claims` | 즉시 실행 + 결과 반환. max_per_doc 5 고정 |
| invoke_handler | `modals/app/src/main.rs` | get_lint_strong_claims 등록 |
| Frontend | `ui/index.html` + `ui/dashboard.js` | doc-detail에 detail-aux div + Verification 탭에 강한 주장 카드. dashboard.js에 renderDocDetail 확장 + runLintStrongClaims 메서드 + _escape 헬퍼 + click 위임 분기 |

기본 구현 None인 포트 메서드 패턴 — lesson 14 회피의 정석. 어댑터에서 override해도 디폴트 의존 어댑터(stub 등)는 영향 0.

### 메타 룰 1 추가 사례: 5계층 직렬화 (lesson 42 4계층 + 포트 메서드)

lesson 42에서 4계층(도메인/어댑터 응답/저장 모델/직렬화)을 정리했는데, UI 노출 phase에서는 **포트 메서드 + Tauri command + frontend**가 추가되어 사실상 5~7계층 동기화. lesson 42 메타 룰 1의 확장 적용:

```
도메인 모델 → LLM 응답 변환 → 저장 모델 → 영속 파일 → 포트 노출 메서드 → Tauri command → frontend 렌더
```

각 계층이 빠지면 다음 계층에서 데이터 부재. Phase 88에서는 1~4계층(LLM 채움), 본 phase에서는 5~7계층(UI 노출)을 묶어 완료.

### 회귀 기준선

- workspace lib **343 유지** (96 core + 152 adapters + 95 shared) — get_metadata는 lib 테스트 미신규 (필드 매핑만)
- workspace clippy `--all --tests` **0건**
- workspace + Tauri `cargo check` ✅
- 통합 테스트 빌드 ✅ (lesson 27 회귀 차단 확인)
- Tauri commands 70 → **71** (get_lint_strong_claims +1)

### 다음 단계

권장 우선순위 2단계 — ✅ **본 phase에서 완료**:
- A1-hit: 9건 코퍼스 1.93x 가속 측정. SHA 중복 체크 사이드 발견. `spec/benchmarks/a1_hit_phase89_20260518.json`
- C2-fp: 36 docs FP 0% 측정. `spec/benchmarks/c2_fp_phase89_20260518.json`

3단계 — ✅ **#6 HyDE 어댑터 활성 본 phase에서 완료**:
- LLM 어댑터 5종 (claude_cli / anthropic / openai / ollama / gemini) + wrapper 3종 (chunked / fallback / cached) 모두 `generate_hypothetical` override 추가
- `prompts.rs`: NAME_HYDE / DEFAULT_HYDE + build_hyde_prompt + SECTIONS 등록
- `prompts.toml`: `[hyde]` 섹션 추가
- 디폴트 비활성 유지 (`search.hyde_enabled = false`). 트리거 #6 도달 시 디폴트 1줄 변경 활성

남은 트리거 대기:
- A2-def / B1-def: 5K+ 코퍼스 측정 후 디폴트 1줄 변경 — 본 phase에서 **자동 측정 불가** 결정 (검색 만족도는 사용자 신호 필요)
- #2 / #4: doc_type 다양성 확보된 5K+ 코퍼스 재측정 — **bench_real_corpus_variants 사용 가능** (HashEmbedder + LLM stub로 LLM 비호출, 1211 파일 측정 가능. 본 phase 진행 중)
- #7 Parent-Child 청크: 1K+ 코퍼스 MRR 회귀 발견 시
- #10 BGE-M3 Sparse LocalVectorStore 통합: **본 phase B-2에서 도메인/포트 토대 완성**. FastEmbedSparseAdapter EmbeddingPort impl + LocalVectorStore sparse_index는 별도 phase

## C/D/B-2 영역 확장 (Phase 89 동일 phase)

### C 영역 — 측정 사이드 발견 해소 (메타 룰 14 사례 3건 누적)

**C-1**: `--base` CLI 옵션이 `paths`까지는 도달했지만 `LocalVectorStore::new()`는 환경변수만 봄. A1 측정 중 발견. `build_service`에서 `LocalVectorStore::with_path(paths.base.join(".local-store.json"))` 명시 호출로 해소. `cli.rs::Stats` 분기도 동일 적용.

**C-2**: `CompositePreprocessor::preprocess_with_config`가 매번 `::new(pdf_tool, ocr_tool)` 호출 → 도구 재감지 spawn 발생. "fallback, 비캐시" 로그 매 가공 출력. `with_tools(pdf_tool, ocr_tool, self.host_tools.clone())`로 캐시 재사용.

**C-3**: `load_doc_type_registry`가 파일 미존재 시 빈 레지스트리 + WARN. settings.db에 17 기본 유형이 자동 마이그레이션됨에도 CLI 분기가 toml만 보던 lesson 38 변형. `find_data_dir(None)` → `settings.db.to_doc_type_registry()` 폴백 추가.

3건 모두 **lesson 38(같은 의미 함수 다중 정의)의 변형**. 메타 룰 14로 승격.

### D 영역 — 메타 룰 13~15 승격

- **13: 인프라 활성화 4단계** — 메타 룰 5강화 3단계(인프라/로직/측정) → 4단계(UI 노출) 확장. Phase 87→88→89가 첫 사례
- **14: 다중 진입점 분기 트리 통일** — lesson 38 + Phase 89 C-1/C-2/C-3 누적. 신규 함수 추가 시 grep + 진입점 4곳 확인 의무
- **15: 측정 환경 격리 + 증분 상태 일괄 삭제** — A1 hit 측정 사이드 발견. PIPELINE_BASE 격리 + .compile-state.json + .work-queue.json 일괄 삭제

### B-2 — #10 Sparse 인프라 (트리거 대기 패턴, lesson 30)

도메인/포트 토대만 추가. 완전 통합은 별도 phase 분리:

- `core/domain/models.rs::SparseEmbedding` 신규 — Serialize/Deserialize/dot
- `EmbeddingPort::embed_sparse` 디폴트 bail!, `supports_sparse` 디폴트 false
- `VectorDBPort::upsert_sparse_embedding` 디폴트 no-op, `search_sparse` 디폴트 빈 결과, `sparse_enabled` 디폴트 false

트리거 #10 도달 시 어댑터 override + LocalVectorStore sparse_index 추가만 하면 즉시 활성. lesson 14 패턴 회귀 0 — 디폴트 동작이 안전한 no-op.

## 교훈

1. **인프라 활성화 3단계가 4단계로 확장**: lesson 42에서 "인프라 추가 → LLM 활성화 → 실 코퍼스 측정" 3단계로 정의했지만, 실 사용자가 결과를 보려면 4단계 = **UI 노출**이 필수. Phase 87→88→89 = "인프라 → 데이터 채움 → 사용자 노출" 패턴
2. **포트 메서드는 신규할 때 디폴트 None/empty로 추가**: get_metadata 같이 모든 어댑터가 override할 필요 없는 메서드는 `Result<Option<T>>` 디폴트 None으로 추가. stub 어댑터 영향 0, 통합 테스트 회귀 0. lesson 14 패턴의 안전한 적용
3. **wikidocs 353407 매핑 자유도**: 외부 문서 권고가 본 프로젝트 함수 셋과 1:1이 아닐 때 카테고리 단위로 매핑. weekly = 품질 의심(strong_claims), monthly = 정합성 검사(topics). 메타 룰 9 "외부 문서 권고 도입 3단계"의 1단계 자유도
4. **dead_code 일관성**: `#[allow(dead_code)]`라도 같은 영역 코드는 같이 갱신하면 dead가 활성화되는 시점(예: 별도 트리거 도달)에 회귀 줄어듦. lesson 14의 트리거 대기 패턴과 동일 원리

## A1 hit 측정 사이드 발견 (Phase 89, 2026-05-18 후속)

A1 LLM 캐시 hit률 측정 중 발견: **SHA-256 중복 체크 + .compile-state.json + .work-queue.json이 LLM 호출 이전에 동일 파일을 스킵**하므로 단순 inbox 재투입으로는 A1 캐시가 트리거되지 않음. 

```
inbox → SHA-256 중복 체크 (스킵 9건) → ... → LLM 호출 → A1 캐시 lookup
                                              ↑
                                       단순 재투입은 여기까지 도달 못함
```

측정 흐름:
1. 1차 가공: LLM 호출 → A1 캐시 entries 9, total_hits 0, per-doc 48.1s
2. inbox에 같은 파일 재투입 → **스킵 9건** (SHA 중복) → LLM 미호출
3. `.local-store.json` 삭제 → 다시 시도 → 여전히 스킵 (.compile-state.json/.work-queue.json이 SHA 보존)
4. 모든 증분 상태 파일 일괄 삭제 → 재투입 → **LLM 호출 경로 활성화** → `[llm-cache] hit text file_name=... hash=...` 로그 → per-doc 24.9s

결과: per-doc 48.1 → 24.9 = **1.93x 가속**. LLM 호출이 전체 시간의 약 50% 차지.

운영 시 A1 hit 발생 시나리오:
- 동일 파일 hash이지만 다른 inbox(extra_inboxes)에서 발견되어 새 가공 진입
- settings.db 보존 + 색인 DB만 rebuild (마이그레이션 시나리오)
- 동일 content_hash 청크 가공 (>40KB 분할 시 같은 청크 콘텐츠 재등장)
- 운영 데스크톱에서 .local-store.json 손상으로 재생성

본 측정으로 A1 캐시 hit 동작은 검증됨. 다만 **실 사용자의 일반적 사용 패턴**에서 hit률은 낮을 가능성 (SHA 중복 체크가 먼저 걸러냄). lesson 30 "Ruflo 인프라 + 측정 후 활성화" 패턴 — 측정으로 동작은 확인했으나 디폴트 변경 가치는 시나리오 의존.

코드 위치:
- SHA-256 중복 체크: `core/service.rs::process_file_*` (단계 3)
- A1 캐시 lookup: `shared/cached_llm.rs::CachedLLM::classify_and_process_text` (LLM 호출 전 prefix)

벤치 결과: `spec/benchmarks/a1_hit_phase89_20260518.json`

## 메타 룰 1 추가 사례 (본 phase에서 새로 식별)

UI 노출 phase의 **5계층 → 7계층 동기화 점검 체크리스트**:
- [ ] 도메인 모델 필드 추가 (#[serde(default)])
- [ ] LLM 응답 → 도메인 변환 (response.rs)
- [ ] 저장 모델 직렬화 (StoredDoc 등)
- [ ] 영속 파일/DB 결과 직접 확인 (.local-store.json grep)
- [ ] **포트 trait 메서드 (디폴트 None/empty)**  ← lesson 43 추가
- [ ] **어댑터 override**  ← lesson 43 추가
- [ ] **Tauri command 응답 매핑**  ← lesson 43 추가
- [ ] **frontend 렌더링 (escape 필수)**  ← lesson 43 추가
- [ ] invoke_handler 등록 (lesson 19 변형 — neuro MCP/Tauri 6건 동기화 누락 패턴)

특히 frontend `_escape` 같은 XSS 안전 헬퍼는 신규 위치에서 매번 재검토.
