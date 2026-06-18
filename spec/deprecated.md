---
updated: 2026-06-18 (OutboundManifest super-trait + core/ports/outbound/ 본체 폐기 등재 — plugin-sdk-1 step-p7, 본질 재정의 3차 raw I/O 정합)
purpose: 삭제/폐기/보류 항목의 **단일 진실원**. lesson 14 "미연결 포트는 코드 부담" 누적 가시화
---

> **단일 진실원 원칙** (2026-05-20 확립): "무엇이 지금 없는가"는 본 문서. "왜 그 Phase에 그 결정을 했는가"는 `architecture.md` / `architecture-archive.md`. 신규 삭제 시 본 문서에 즉시 추가, 다른 문서는 본 문서로 링크. 메타 룰 1 "다중 위치 동기화 누락" 자기 적용.

# Deprecated / 보류 / 폐기 인벤토리

본 문서는 코드베이스에서 **삭제·폐기·장기 보류**된 항목을 한 곳에 모아 추적한다. 목적:
- lesson 14 "미연결 포트는 코드 부담" 패턴의 누적 감시 (월 1회 점검 후보)
- 재도입 시 git history 추적 비용 최소화
- "왜 없는가"를 물을 때 단일 답변 위치

각 항목은 **삭제일 / 사유 / 재도입 트리거 / 복구 방법**을 명시한다.

---

## 무효화됨 (계획 폐기)

### `search-extraction-plan.md` — 2026-06-04 (Phase 200 결정 트리거)

| 항목 | 값 |
|------|-----|
| 위치 | `prd/research/search-extraction-plan.md` |
| 폐기 사유 | 2026-06-01 작성 "검색 도메인 분리 (Phase 108~115, `_rust_module/` 이관)" 결정이 2026-06-04 tasty 패턴 흡수 결정의 **부분집합**으로 자연 흡수. fp-plugin-search 하나로 동일 가치 도달 + 사용자 표면적 직접 제어 추가 가치 |
| 상위 결정 (단일 진실원) | `prd/research/plugin-architecture-2026-06-04.md` |
| 자료 보존 | 본문은 보존 (분리 후보 4,200줄 / MCP 36 정밀 분류 / 위험 매트릭스 / 8 논점 분석은 Phase 203 fp-plugin-search 진입 시 참고 자료) |
| 무효화 트리거 메타 룰 | 메타 룰 22 10건째 (사용자 정책 경계 합의) + 메타 룰 19 단일 진실원 위임 |
| Phase 108~115 → Phase 200~209 | fp-plugin-search (Phase 203) + 14 plugin 추가 (Phase 204~207) |

### `mydocsearch_decision.md` — 2026-04-08 결정 → 2026-06-05 spec 파일 삭제 완료

| 항목 | 값 |
|------|-----|
| 원본 위치 | `spec/mydocsearch_decision.md` (2026-06-05 삭제) |
| 원본 결정 (2026-04-08) | "MyDocSearch 통합 불필요, LocalVectorStore 단일 구조 유지". 근거: file-pipeline은 검색 엔진이 아닌 검색 기능 몇 가지만 필요 / BM25 sparse vector ~200줄 / 3K~10K 규모에서 Qdrant 0.57ms 충분 / 두 프로젝트 동시 디버깅 부담 회피 |
| 폐기 사유 | 2026-04-08 "LocalVectorStore 단일 구조" 결정이 search-extraction-plan (2026-06-01, 무효화) + plugin-architecture (2026-06-04) 두 결정으로 무효 — LocalVectorStore 자체가 fp-plugin-search로 이관되어 "단일 구조" 전제 자체가 폐기 |
| 흡수 완료 항목 (보존 위치) | 문서 동기화 규칙·성능 회귀 기준선·세션 시작 절차 → CLAUDE.md / BM25 sparse 아이디어 → prd/features/bm25-sparse-search.md / .vec 영속화 → prd/features/vec-file-persistence.md |
| 상위 결정 (단일 진실원) | `prd/research/plugin-architecture-2026-06-04.md` |
| 삭제 트리거 | 사용자 합의 (2026-06-05) — Phase 203 대기 대신 즉시 삭제 (메타 룰 19 단일 진실원 위임 / 메타 룰 12 stale 잔존 차단) |
| 복구 방법 | 본 deprecated.md 엔트리가 결정 사실 보존. 원본 본문 필요 시 git history(`git log --all -- spec/mydocsearch_decision.md`) — 단 현재 git 미저장 가능성 |
| 관련 lesson | 49 (단일 진실원 위임) / 72 (본질 재정의 2차) |

---

## 삭제됨 (코드 제거)

### `OutboundManifest` super-trait + `core/ports/outbound/` 본체 — 2026-06-18 (plugin-sdk-1 step-p7, lesson 79)

| 항목 | 값 |
|------|-----|
| 위치 | 구 `crates/core/src/ports/outbound/mod.rs` (디렉토리째 삭제) + `ports/mod.rs::pub mod outbound` 등록 해제 |
| 삭제 대상 | `OutboundManifest` trait (id/category/capabilities/modes/config_keys) + `OutboundCategory` enum + `ConfigKey` struct + `OutboundStoragePort`/`OutboundEmbeddingPort`/`OutboundLlmPort`/`OutboundNotifyPort`/`OutboundRerankPort`/`OutboundVerifyPort` 6 alias trait |
| 동반 제거 | output.rs 6 port super-trait bound (`LLMPort`/`RemoteStoragePort`/`EmbeddingPort`/`NotificationPort`/`VerificationPort`/`RerankerPort` 의 `: OutboundManifest +` 제거) + 어댑터/service/cached_llm impl 32건 + 통합/벤치 테스트 impl 21건 = 총 **53 impl 블록 제거** |
| 삭제 사유 | 본질 재정의 3차 (2026-06-17, plugin-architecture §3-C "Output adapter = raw I/O 만") → outbound 도메인 메타데이터 우산은 `raw_transport` 4 채널(Http/Filesystem/Stdio/Sqlite)로 대체. 도메인 로직(capabilities/modes/config_keys)은 plugin manifest(`fp-plugin.toml`) 이관. **사전 baseline 검증(메타 룰 18): OutboundManifest는 호출처 0건의 死 super-trait — `OutboundCategory::` 사용은 전부 impl 본문 내부, `dyn/as OutboundManifest`·외부 `.config_keys()` 호출 0건 확인 후 안전 폐기** |
| 검증 (원격 Linux) | `cargo check --all` 경고 0 / `cargo build --tests --all` 통과 / Tauri app `cargo check` 통과(icon.png 누락은 기존 환경 이슈, 무관) / `cargo nextest run --all` (bench 제외) **489/489 통과** |
| 재도입 트리거 | outbound 어댑터 공통 메타데이터 우산이 다시 필요할 때 — 단 plugin manifest 기반(`fp-plugin.toml`)이 동일 가치를 제공하므로 가능성 낮음 |
| 복구 방법 | `git log --all -S "trait OutboundManifest"` (cycle 5 worker baseline 86c4f4f 이후) |
| 관련 lesson | 79 (본질 재정의 3차 raw I/O) / 77 (outbound 우산 도입 — 도입 후 1 cycle 만에 재정의로 폐기, 메타 룰 22 후속) / 14 (미연결 포트 = 코드 부담) / 18 (광범위 step 진입 전 baseline 검증) |

### `TerminalDuplicateResolution` + `TerminalSensitiveNotification` (driving/terminal_*.rs) — 2026-06-18 (cli-prompt-remove-1, sub-decision A)

| 항목 | 값 |
|------|-----|
| 위치 | 구 `crates/adapters/src/driving/terminal_resolution.rs` + `terminal_sensitive.rs` (각 1 struct, 파일째 삭제) |
| 삭제 사유 | 사용자 발화("사용자 입력 받는 논리 제거해. 해당 시나리오는 설정으로 적용 할꺼야") → stdin 대화형 프롬프트 폐기. config 기반 자동 결정으로 대체 (lesson #25 정합 — 사용자 입력 제거 + 자동 결정) |
| 대체 | `driving/auto_resolution.rs::AutoDuplicateResolution` + `auto_sensitive.rs::AutoSensitiveNotification` (config 주입). `DuplicateResolutionConfig`(sha256_match/semantic_match → `DuplicateAction`) + `SensitiveResolutionConfig`(default_action → `SensitiveAction {Skip/MoveOnly/IndexWithStub}`) |
| 포트 계약 | `DuplicateResolutionPort` / `SensitiveNotificationPort` (input.rs) = **불변** — 어댑터 본문만 교체 |
| DI 변경 | `shared/lib.rs::build_service` = Auto* 주입(GUI/CLI/watcher 공통). `cli/main.rs::build_service_cli` 래퍼 + stdin IsTerminal 분기 삭제 → 11 호출처 build_service 직접. ServiceBuilder/stub.rs Stub* = 테스트용 유지 |
| 검증 (원격 Linux) | cargo check --all 경고 0 / build --tests --all 통과 / Tauri check 통과 / nextest 489/489 (bench 제외, 회귀 0) |
| 재도입 트리거 | 대화형 CLI 모드 재요구 시 — 단 자동 결정 config가 표준이므로 가능성 낮음 |
| 복구 방법 | `git log --all -S "TerminalDuplicateResolution"` |
| 관련 lesson | 80 (본질 재정의 4차 sub-decision A) / 11 (StubDuplicateResolution Skip→Keep, 비대화형 기본 동작) / 56 (StubSensitiveNotification 기본 Metadata) / 25 (사용자 입력 vs 코퍼스 신호) |

### `PluginError::IpcNotYetImplemented` variant (core/plugin/registry.rs) — 2026-06-10 (Phase 202 본진입, lesson 76)

| 항목 | 값 |
|------|-----|
| 위치 | `crates/core/src/plugin/registry.rs::PluginError` enum |
| 삭제 사유 | Phase 201 placeholder 진입 시 등재된 일시 variant. Phase 202 본진입 (B2 묶음)에서 `PluginRegistry::call`이 실제 IPC 수행하면서 자연 삭제 — 메타 룰 5 강화 ("구현 보류 마커는 본진입 시 자연 삭제 의무") |
| 대체 variants | `NotRunning {plugin_id, cause}` + `IpcTransport(String)` + `IpcProtocol(String)` — Phase 202 본진입 시점 추가 |
| 복구 방법 | git log feature/B2 SHA `0e3fe58` 직전 |
| 관련 | lesson 76 (Phase 202 본진입 + bundle-cycle B1~B4) / B2 머지 SHA `e598807` / domain-map.md §PluginError 9 variants |

### `process_file_legacy` + `classify_and_process_with_retry` (service.rs) — 2026-05-21 (Phase 91)

| 항목 | 값 |
|------|-----|
| 위치 | `crates/core/src/service.rs` |
| 삭제 함수 | `process_file_legacy` (#[allow(dead_code)] 7+ Phase) + `classify_and_process_with_retry` (legacy 안에서만 호출) |
| 삭제 사유 | Phase 91 A1' 검사 분산 통일 작업 중 발견. `process_file_legacy`는 `process_file` → `process_file_with_pipeline` 위임 전환 이후 호출처 0건. classify_and_process_with_retry는 legacy 본문에서만 사용. lesson 14 "미연결 포트" 변형 사례 |
| 재도입 트리거 | 비파이프라인 직접 가공 모드 재도입 시 |
| 복구 방법 | `git log --all -S "fn process_file_legacy\|fn classify_and_process_with_retry"` (현재 git 미저장 → 백업 파일 service.rs.phase91-bak는 본 phase 종료 시 삭제됨) |
| 영향 | service.rs **2034 → 약 1620줄** (-414줄, -20.4%) |
| 관련 lesson | 14 (미연결 포트) / 28 (기능 제거 시 통합 테스트 잔존) |

### Tauri commands 10건 백엔드 정리 (G-7) — 2026-05-20

| 항목 | 값 |
|------|-----|
| 위치 | `modals/app/src/commands.rs` + `modals/app/src/main.rs` invoke_handler |
| 삭제 함수 10건 | `search_with_trace` / `purge_dry_run` / `purge_execute` / `list_doc_types` / `save_doc_type` / `delete_doc_type` / `refresh_host_tools` / `test_preprocess` / `mcp_tools_list` / `mcp_tool_set_enabled` |
| 삭제 사유 | G-6 frontend API 11건 정리 후 backend 호출처 0건 확인. lesson 19 10단계 단계 2~3 (Tauri commands + invoke_handler 등록) 적용 |
| 재도입 트리거 | 해당 frontend 기능 재요구 시 — frontend dead-code(G-6) 복구 필요 |
| 복구 방법 | `git log --all -S "fn search_with_trace\|fn refresh_host_tools\|fn mcp_tools_list"` |
| 영향 | `commands.rs` **1956 → 1590 (-366줄, -18.7%)**. main.rs invoke_handler 10건 제거. Tauri commands **71 → 61** |
| 사이드 발견 | dirty working tree에 `ListParams` struct / `mask_secrets` / `mask_secret_at` / `restore_masked_secrets` 정의 누락 → 본 작업에서 정의 추가 (lesson 12 secret 복원 패턴 응용) |
| 관련 lesson | 19 (UI 제거 10단계) / 14 (dead 자산) / 12 (save_config secret) |

### dashboard.js dead 6 함수 + 7 action case + 5 if action + 11 API 정의 (G-6) — 2026-05-20

| 항목 | 값 |
|------|-----|
| 위치 | `ui/dashboard.js` 전반 |
| 삭제 대상 | 6 함수 (`_renderSearchSimulation` / `_renderMcpTools` / `_renderSystemCredentials` / `_renderMigration` / `_renderHostToolsStatus` / `_refreshHostTools` / `_runSearchSim` / `_loadDocTypes` / `_renderDocTypesTable` / `_openDocTypeModal`) + handlePBAction 안 7 case (`pb-purge-dry-run` / `pb-purge-execute` / `pb-preprocess-test` / `pb-add-doctype` / `pb-edit-doctype` / `pb-delete-doctype` / `pb-doctype-page`) + 별도 5 if action (`refresh-host-tools` / `mcp-disable` / `mcp-enable` / `run-search-sim` / `test-preprocess`) + 11 API 정의 (`listDocTypes` / `saveDocType` / `deleteDocType` / `testPreprocess` / `refreshHostTools` / `purgeDryRun` / `purgeExecute` / `mcpToolsList` / `mcpToolSetEnabled` / `searchWithTrace` + 기타) |
| 삭제 사유 | G-5 dead_selector_scan baseline 실행 시 14건 후보 발견 → 분류 결과 13건 진짜 dead 확정. HTML 엘리먼트 미정의로 모든 호출처가 NoOp이거나 동적 생성 트리거 도달 불가. lesson 47 패턴 (JS↔HTML 셀렉터 불일치) 반복 사례 |
| 재도입 트리거 | 해당 기능 재요구 시 — 단 IA 재설계가 동반되므로 별도 phase 권장 |
| 복구 방법 | `git log --all -S "_renderMcpTools\|_renderHostToolsStatus\|_renderSearchSimulation"` |
| 영향 | dashboard.js **4645 → 4234 (-411줄, -8.8%)**. Tauri commands 백엔드 (refresh_host_tools / mcp_tools_list / purge_dry_run 등 9건) 영구 dead 가능성 — 다음 phase에서 분리 트리거로 정리 검토 |
| 관련 lesson | 47 (JS↔HTML 셀렉터 메타 패턴) + 19 (UI 제거 10단계) |
| 자동화 | `spec/benchmarks/scripts/dead_selector_scan.sh` baseline 0건. createElement 동적 fallback whitelist 추가됨 (`settings-no-results` false positive 해소) |

### `pb-subtabs` 4서브탭 dead-code (dashboard.js / dashboard.css) — 2026-05-20 (G-4 진단 후속)

| 항목 | 값 |
|------|-----|
| 위치 | `ui/dashboard.js` 6 함수 + 6 호출처 + `ui/dashboard.css` 5 rule |
| 함수 | `_renderPBSubtabs` / `_renderPBSubtabContent` / `_renderSubtabProcessing` / `_renderSubtabRemote` / `_renderSubtabChunking` / `_renderSubtabRetention` |
| CSS | `.pb-subtabs` / `.pb-subtab` / `.pb-subtab:hover` / `.pb-subtab.active` / `.pb-subtab-content` |
| 호출처 6건 | `case 'pb-subtab'` 핸들러 1건 + 단독 호출 5건 (모두 NoOp 상태였음) |
| 역할 | 옛 IA(Phase 56/67) "데이터 가공 / 외부 저장소 / 청킹 / 보존 & Purge" 4서브탭 UI |
| 삭제 사유 | HTML index.html에 `#pb-subtabs` / `#pb-subtab-content` 엘리먼트 부재. `getElementById` null로 즉시 return — 호출 자체가 NoOp이었음. Phase 67(인스펙터 480px) IA 전환 시 HTML만 정리되고 JS/CSS는 잔존. lesson 19 10단계 체크리스트의 "JS render 함수 + DOM 셀렉터 정합성 검증" 누락 사례 |
| 발견 트리거 | G-4 진단 (2026-05-20). browser-automation MCP `extract_structured`로 HTTP 모드 검증 시 invoke 의존이 아닌 dead 패턴 식별 |
| 재도입 트리거 | 4서브탭 IA 복귀가 결정될 때. 현행 인스펙터 480px IA가 충분히 검증되었으므로 가능성 낮음 |
| 복구 방법 | `git log --all -S "_renderSubtabProcessing"` |
| 영향 | dashboard.js 4915→4644 (-271줄, -5.5%). dashboard.css -5 rule + 주석. 호출처 6건 NoOp 제거 (동작 변경 없음) |
| 관련 lesson | 13/19/28 (UI 기능 제거 시 다중 위치 동기화) / **47 (신규, JS↔HTML 셀렉터 불일치 메타 룰)** |

### `CrossRefUpdater::auto_link` + `AutoLinkContext` — 2026-05-15 (Phase 85)

| 항목 | 값 |
|------|-----|
| 위치 | `crates/core/src/domain/cross_reference.rs` (현재 보류 마커 주석만 유지) |
| 시그니처 | `pub fn auto_link(ctx: AutoLinkContext) -> Result<CrossRefReport>` (14 인자: 도메인 5 + 임계값 3 + cap 4 + 포트 1) |
| 역할 | SQL 스타일 자동 교차참조 — cosine similarity + 키워드 겹침 + 같은 유형/날짜로 Supersedes/Updates/RelatedTopic/References 4종 관계를 LLM 호출 없이 자동 부여 (pgvector 패턴 차용) |
| 삭제 사유 | 7+ Phase 호출처 0건. `update_cross_references`(LLM 기반)가 동일 영역을 더 정교하게 커버. lesson 14 패턴 (inherent 메서드형 dead 자산) |
| 재도입 트리거 | 사용자가 "LLM 호출 없는 자동 관계 추출"을 명시적으로 요구할 때. 예: LLM 비용 절감 시나리오, 오프라인 환경 운영 |
| 복구 방법 | `git log --all -S "pub fn auto_link"` → 직전 커밋 본문 참조 |
| 영향 | 약 164줄 감소 (함수 138 + struct 17 + docstring 8 + unused `DocDate` import 1) |
| 관련 lesson | 14 (미연결 포트) / 38 (Phase 85 위생 일괄) |

### dead config 5건 (`vector_db.qdrant_url` / `collection` / `auto_start` / `embedding.sensitive_model` / `embedding.onnx_model_dir`) — 2026-05-04 (Phase 65-2)

| 항목 | 값 |
|------|-----|
| 위치 | 구 `crates/shared/src/config.rs::VectorDbConfig` + `EmbeddingConfig` |
| 삭제 사유 | Phase 44 Qdrant 완전 제거 + Phase 62 fastembed 채택 후 사용처 0건. lesson 20 §44 grep 검증 |
| 재도입 트리거 | (1) Qdrant 외부 모드 재도입 시 `qdrant_url` / `collection` / `auto_start` (2) ONNX feature 재도입 시 `onnx_model_dir` (3) sensitive 전용 임베딩 분리 시 `sensitive_model` |
| 복구 방법 | `git log --all -S "qdrant_url\|sensitive_model\|onnx_model_dir"` |
| 영향 | Pipeline Embedding 노드 단순화 ("fastembed BGE-M3 1024차원 고정" 표시) |
| 관련 lesson | 20 (IA 재설계 + dead config 정리) |

### `feedback_*` Tauri commands 7건 — 2026-04-30 (Phase 64)

| 항목 | 값 |
|------|-----|
| 위치 | `modals/app/src/commands.rs` (현재 제거됨) |
| 역할 | Feedback 탭 백엔드 API |
| 삭제 사유 | Feedback 탭 비활성화 (Phase 55) 후 JS dead code만 정리되고 Rust 백엔드 잔존. lesson 13/19 8단계 → 10단계 체크리스트로 회귀 방지 |
| 재도입 트리거 | Feedback 탭 재도입 시 |
| 복구 방법 | `git log --all -S "feedback_apply"` |

### `credential_store_*` Tauri commands 4건 — 2026-04-30 (Phase 64)

| 항목 | 값 |
|------|-----|
| 역할 | UI 호출처 0건 (`keyring` 직접 호출로 대체됨) |
| 삭제 사유 | UI 측 호출 dead, Settings 크레덴셜 카드가 `cred_*` 시리즈만 사용 |
| 재도입 트리거 | 외부 시크릿 저장소 추가 시 검토 |

### `get_health` / `get_lint` / `delete_document` / `fix_backlinks` / `get_retention_config` / `get_pipeline` / `save_pipeline` Tauri commands — 2026-05-15 (Phase 84)

| 항목 | 값 |
|------|-----|
| 삭제 사유 | Phase 64 시점에 "frontend 정리 대상" 주석으로 표기되어 있던 dead 7건 일괄 삭제. JS API 측에서 호출 0건 검증 후 |
| 재도입 트리거 | 해당 기능 재요구 시. 단, `health`·`lint`·`retention`·`pipeline`은 영구 도메인 개념이라 재도입 시 시그니처 재설계 권장 |

### `cli.rs` 파일 — 2026-04-30 (Phase 64)

| 항목 | 값 |
|------|-----|
| 위치 | 구 `modals/cli/src/cli.rs` |
| 삭제 사유 | mod 선언 없는 dead 파일 (lib.rs/main.rs에서 참조 없음) |

---

## 외부 의존 제거

### `vendor/onnxruntime/` 디렉토리 — 2026-04-30 (트리거 #11)

| 항목 | 값 |
|------|-----|
| 위치 | 구 `src/vendor/onnxruntime/` (현재 디렉토리 비어 있음) |
| 크기 | 394MB (onnxruntime.dll + 1.24.4 archive) |
| 삭제 사유 | `fastembed` feature가 ort-sys 정적 링크로 동일 기능 + 더 안정적으로 제공 |
| 재도입 트리거 | fastembed가 작동 불가한 환경(MSVC v14.38- 등)에서 ONNX 외부 DLL 폴백 필요 시 |

### `ort` + `tokenizers` optional dep + `[features] onnx` — 2026-04-30 (트리거 #11)

| 항목 | 값 |
|------|-----|
| 위치 | 구 `crates/adapters/Cargo.toml` |
| 삭제 사유 | fastembed 기반 BGE-M3로 일원화 |

---

## 기능 비활성 (코드 보존)

### Feedback 탭 — 2026-04-30 (Phase 55)

| 항목 | 값 |
|------|-----|
| 위치 | UI는 제거, Rust 백엔드는 보존 |
| 비활성 사유 | 실사용 가치 미검증, 우선순위 낮음 |
| 재도입 트리거 | 사용자 피드백 수집 요구 시 |

### GraphDB (JSON/Neo4j 어댑터) — 2026-04-28 (Phase 58)

| 항목 | 값 |
|------|-----|
| 위치 | 코드 삭제 (Phase 58) |
| 삭제 사유 | LocalVectorStore `find_related`로 KG 기능 충분. 7 Phase 미연결 (lesson 14의 원형 사례) |

### Notification UI — Settings 탭에서 제거 (코드 보존)

| 비활성 사유 | NullNotification 기본. Telegram/Slack 통합은 Phase 29에서 확인되었으나 UI는 제거 |

---

## 폐기 항목

| 항목 | 폐기 시점 | 사유 |
|------|----------|------|
| PDF/OCR 전처리 (구 6-6) | 2026-04-14 | 파일 가공은 claude_cli 전용. 별도 전처리기 불필요 |
| Qdrant embedded 모드 | 2026-04-21 (Phase 44) | Qdrant 완전 제거. 복귀 가능성 없음 |
| Qdrant named vector | 2026-04-21 (Phase 44) | LocalVectorStore 단일 |
| 모바일 빌드 (iOS/Android) | 2026-04-22 | Desktop 전용 결정 |
| 민감 config 내용 기반 탐지 | 2026-04-14 | watcher 스킵으로 충분 |
| BGE-M3 ONNX Python production (트리거 #3a) | Phase 62 | fastembed 채택으로 폐기 |
| BGE-M3 Rust 네이티브 ort load-dynamic (트리거 #3b) | Phase 62 | fastembed가 ort-sys 정적 링크로 즉시 가능 |
| ColBERT late interaction (트리거 #5) | Phase 62 | BGE-M3 Reranker로 대체 |
| `todo_merge` 노드 | Phase 53 | `entity_extract`로 교체 |

---

## 점검 규칙

- **월 1회**: 본 문서의 "삭제됨" 섹션을 grep으로 검증 — 삭제 항목이 코드에 잔존하지 않는지 확인
- **신규 삭제 시**: 본 문서에 즉시 추가. 코드 위치 주석(보류 마커)만으론 추적 누락 가능
- **재도입 시**: 본 문서에서 항목 제거 + git history 인용으로 복구 컨텍스트 확보
