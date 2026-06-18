---
updated: 2026-06-17 (cycle 5: mcp-removal-1 완결 — MCP 전체 폐기. mcp_server.rs 2139줄 삭제 + 37 도구 + CLI Serve 삭제 + Tauri get_mcp_tool_catalog_full 삭제 + webapp-design.md MCP 카드 폐기 + architecture.md current MCP 수치 0 정정. 코드 내 MCP 실체 0. 본질 재정의 3차 — host=파일 가공만, 외부 연계는 plugin(헥사고날 adaptor)으로 통일. 과거 Phase 로그는 히스토리 보존. 빌드 3종 PASS(workspace check 0 / Tauri check 0 / nextest 회귀 0). step-m5 완결. cycle 4: prep-unlock(SettingsDb 본체) done — settings-db-split-1 plan 완결. SettingsDb 86 메서드 → adapters/driven/settings/sqlite.rs (shared/settings_db.rs 2230→138줄 re-export + open_or_migrate 자유함수). 누적 19 step succeeded)
---

## 누적 변경 요약 (2026-06-17) — 3 plan 병행 14 step succeeded + outbound 우산 telegram 본 진입 (lesson 78 후보)

### 본 세션 진행 요약

단일 세션 안 3 plan 병행 = hex-arch-d (헥사고날 정공법 D) + settings-db-split-1 (sqlite 분리 prep) + outbound-umbrella-1 (외부 연계 우산 추상화). 단일 세션 11 step succeeded = 역대 최대 묶음. 사용자 발화 2건 트리거:

1. `"원격 저장소를 텔레그램 추가 구성 추가하고 tasty loop 진행해"` — outbound 우산 본질 재정의 발생
2. `"storageAdaptor는 외부 연계 플러그인으로 공통화 + 외부로 나가는 구현체는 헥사고날 adaptor 패턴으로 통일"` — 옵션 C (outbound 공통 우산 추상화) 확정

### plan 별 산출

| plan | 진척 | 산출 |
|------|------|------|
| **hex-arch-d** (헥사고날 정공법 D) | S-1 + S-4 + guard 1234 + **s5-tray done (cycle 3)** / S-2/S-3 + s5-cli = `SettingsDb` 잔류 cycle 장벽 → prep-unlock 선행 의존 | SearchPort+KgPort + `core/domain/search_engine.rs` (handle_search 410줄 위임) + `core/use_cases/{process_file,crossref}.rs` 분해 + guard-1 host 정정 (agent-task-spec.md while true polling → barrier depends_on Mode 1/2) |
| **settings-db-split-1** (sqlite 분리 — **완결 cycle 4**) | prep-1~3b + **prep-unlock done**: SettingsDb 86 메서드 → `adapters/driven/settings/sqlite.rs`. `open_or_migrate` = shared 자유함수 추출(PipelineConfigExt::load + load_doc_type_registry 의존 잔류, shared→adapters 정방향). shared/settings_db.rs 2230→138줄(re-export). ConfigSnapshot도 core 이전(prep-unlock 선행). cycle 0 + rusqlite 시그니처 누출 0. SQLite 어댑터 헥사고날 정상화 완결 | `core/domain/settings_models.rs` (107줄, 6 도메인 struct) + `core/ports/settings_repo.rs` (125줄, 6 sub-trait skeleton) + **prep-3: `shared/settings_db.rs::SettingsDb` 6 sub-trait + SettingsRepoPort in-place impl (+~150줄)**. baseline 경로 `adapters/driven/settings/sqlite.rs` 는 **cycle 위반** (shared→adapters 의존 기존 박힘) → settings_db.rs 내부 impl 정정. 6 sub-trait = config-free 영역만. HostToolRepo::refresh = adapters HostToolDetector::detect_full 직접 호출. LlmCacheRepo::get_llm_cache = file_hash 조회 후 content_hash 필터. **prep-3b: 28 순수 config 타입(PipelineConfig + 25 하위 struct + LlmCredential + ResolvedPaths + FieldMeta) → `core/domain/config_models.rs` (1140줄) 이전 + `shared/config.rs` re-export (타입 호출처 0건 변경)**. orphan rule = **extension trait 2종** (PipelineConfigExt: load/load_from_str/to_toml_string/resolve_paths + ResolvedPathsExt: create_all) — core 타입에 shared inherent impl 불가 회피. 순수 메서드 3개(default_config/validate/needs_restart) = core inherent. toml/dirs/DB 의존 free fn(find_data_dir/load_from_db/find_config_path/load_doc_type_registry/resolve_doc_types_path/find_and_load_config) + exe_dir + config_snapshot = shared 잔류. **core Cargo.toml toml/dirs 미추가 (cycle 0 유지)** |
| **outbound-umbrella-1** (외부 연계 우산) | o1 + o2 + o2-partial-resolved + o3 + o4 + o6 done / o5 pending | `core/ports/outbound/mod.rs` (119줄, OutboundManifest + OutboundCategory + ConfigKey + 6 port skeleton) + **6 port super-trait 박힘 (RemoteStoragePort/LLMPort/EmbeddingPort/NotificationPort/RerankerPort/VerificationPort = OutboundManifest super-trait)** + **24 어댑터 + 19 mock manifest impl 박힘** (stub 2 + composite 4 + cached_llm 2 + service.rs 4 + local_embed 1 + integration test 9 파일 16) + **zstd 분류 정정** (StoragePort 내부 인프라, OutboundManifest 부재 정합 = storage 5, 합계 24) + 3 디렉토리 정정 (notification/reranking/verification → notify/rerank/verify) + **telegram outbound 양쪽 어댑터 신설** (storage `~310줄` 신규 = TelegramStorageAdapter + 3 mode + 50MB pre-check + 48h delete 검증 + Mirage capabilities + rusqlite 직접 호출 + 자체 TELEGRAM_MAP_SCHEMA + telegram_message_map sqlite table + 4 CRUD 메서드, notify mode_options [alert, event] 박힘) |

### service.rs 분해 (역대 최대)

**1681 → 615줄 (-1066줄, -63%)** — 본 cycle 가장 큰 단일 변경. ProcessFileUseCase + CrossRefUseCase + MaintenanceUseCase 3 use case + 헬퍼 잔류 (compute_hash + read_text + metrics_* + emit_progress + cosine_sim_inline + meta_block_pass) 패턴. 호출자 시그니처 불변 = 통합 테스트 영향 0건.

### outbound 우산 본질 재정의 (plugin-architecture §3-C 본문)

| 변경 영역 | Before (2026-06-04) | After (2026-06-16) |
|----------|---------------------|------------------- |
| plugin id prefix | `fp-plugin-{category}-{name}` | `fp-outbound-{category}-{name}` (storage/embedding/llm/notify/rerank/verify) |
| storage 카테고리 어댑터 | 5 (s3/webdav/network/notion/zstd) | **5 (s3/webdav/network/notion/telegram, zstd 부재 = StoragePort 내부 인프라 정합)** |
| 공통 우산 trait | RemoteStoragePort.capabilities() (Phase 92 H5) | OutboundManifest super-trait (id/category/capabilities/modes/config_keys) |
| 카테고리 분류 | 6 + 합계 24 plugin | **6 port + 24 어댑터 (storage 5 + notify 2 + embedding 6 + llm 7 + rerank 3 + verify 1, telegram = storage + notify 양쪽 = id 별도)** |

### 회귀 검증

- ssh 원격 cargo check --tests PASS 다수 회 (7.92s ~ 18.79s)
- nextest 500 passed + 26 skipped 다수 회 (45.58s ~ 69.80s 범위, 후속 step 포함)
- 회귀 0건 (분해 단계마다 검증)
- 사이드 발견: ssh 원격 timeout 1회 발생 (step-o3 scp 시점) → 재시도 후 정합 (lesson 78 후보 영역)

### 후속 진행 (본 세션 11 → 14 step 누적)

| 추가 step | 결과 |
|----------|------|
| step-o2 partial 해소 | 6 port super-trait OutboundManifest 박힘 + 19 mock manifest impl (stub + composite + cached_llm + service.rs + local_embed SensitivityAware + integration test 9 파일 mock 16) + capabilities ambiguity 정정 2 점 (use_cases/process_file.rs + modals/app/commands.rs `RemoteStoragePort::capabilities` trait method 명시) + zstd 분류 spec 정정 (plugin-architecture L226 → storage 5) |
| step-o3 telegram outbound | 5 파일 박힘 (settings_db.rs telegram_message_map + 4 CRUD / telegram_storage.rs 신규 / storage/mod.rs / telegram_notify.rs mode_options / adapters/Cargo.toml rusqlite + reqwest multipart). lesson #14 R1 사이클 2회 발견 (workspace cycle 회피 → rusqlite 직접 / reqwest multipart feature) |
| step-o6 spec 갱신 | 본 §누적 변경 요약 갱신 + plugin-architecture L226 zstd 정정 (step-o2 partial 안 이미 박힘) + lesson 78 신규 entry 박힘 의무 |

### 메타 룰 누적 갱신

| 메타 룰 | Before | After | 본 세션 누적 |
|---------|--------|-------|------------|
| 22 (사용자 정책 경계 합의) | 17건 | **20건** | +3 (lesson 77 (a) telegram 분류 + 77 (b) outbound 우산 본질 재정의 + 78 (a) telegram 본 진입) |
| 25 (자기 적용 의무) | 8건 | **13건** | +5 (lesson 76 + 77 + 78 자기 적용 + 본 cycle 2 host 자율: 메타 룰 18 정식 승격 자기 적용 + INDEX L5 메타 룰 요약 stale 정정) |
| 30 (spec 즉시 갱신) | 12건 | **15건** | +3 (plugin-architecture §3-C 재정의 직후 lesson 77 + 본 §누적 변경 요약 직후 lesson 78 + 본 cycle 2 META 정식 28건 직후 INDEX L5 stale 정정) |
| **18 (lesson 본문 추정 사항 재검증)** | 3건 | **4건 = 정식 승격 (2026-06-17)** | +1 (lesson 78 step-o2/o3 host 추정 5회 사이클 vs 실측 2회 빗나감 = 4/4 100% 패턴 확정 + 메타 룰 23 §3요소 충족 정식 승격 + lesson 78 강화 체크리스트 3건 박힘) |
| **14 R1 (단언 vs 실측 부재 측정)** | (별도 family) | **본 세션 +2 사이클** | step-o3 의 workspace cycle 회피 + reqwest multipart feature 발견 (host 예상 5회 vs 실측 2회 = lesson 78 sub-pattern 후보) |

### 잔여 host 결정 영역 (4건 — 본 세션 7→4 감소)

다음 cycle 우선 영역 (settings-db-split-1 완결 cycle 4):

1. **hex-arch-d s3 + s5-cli (unlock 완료)**: SettingsDb 가 adapters 로 이전돼 cached_llm/settings_audit_adapter(s3) + cli.rs(s5) = adapters 행 시 같은 crate 의존. **단 s2(setup_review/setup_modules/setup_dryrun)는 SettingsDb 의존으로 core 진입 불가** (core→adapters 금지 = 헥사고날 한계). s2 는 adapter 의존 도메인이라 core 부적격 — plan 재평가 필요.
2. step-o5 TELEGRAM_BOT_TOKEN 통합 test (host 환경 명시 의무 — env export + cargo test --ignored)
3. 호출처 의존 주입 전환 (현 SettingsDb 직접 호출 → `Arc<dyn SettingsRepoPort>` / 카테고리별 `Arc<dyn AuditRepo>`, 점진). SettingsDb adapters 이전으로 이제 진짜 DI 가능.
4. SettingsDbMetricsAdapter(shared/lib.rs) 등 잔존 wrapper 정합 검토

### 본 세션 종결 step (lesson 78 후보 영역 정합)

- step-o2 super-trait + zstd 분류 = step-o2 partial 해소 박힘 (완료)
- step-o3 telegram outbound = 완료 (검증 부재 영역 ssh 재시도 후 정합)
- step-o4 디렉토리 정합 = 완료
- step-o6 outbound 우산 spec 갱신 = 본 entry (완료)

---

## 누적 변경 요약 (2026-06-10) — Phase 202 본진입 (실 IPC) 완료 (lesson 76)

### 본 세션 진행 요약

bundle-cycle 스킬로 B1→B2→B3→B4 4묶음 직렬 진행. 권장 순서 (lesson 75 후속 트리거 4건) 모두 해소.

| 묶음 | 영역 | 산출 | 단위/통합 테스트 |
|------|------|------|----|
| **B1** | Task #22 원격 빌드 검증 | placeholder 26건 검증 (protocol 11 + sdk 2 + core::plugin 13) | 26 PASS |
| **B2** | Phase 202 본진입 (실 IPC) | fp-plugin-sdk::connection + ConnectionPool + PluginRegistry::call 실 호출 + broadcast_event + audit 통합 | +12 = 40 PASS |
| **B3** | 통합 테스트 + 회귀 자동화 | plugin_e2e.rs 3 시나리오 + audit_stage_check.sh `plugin.*` + 변수 stage 확장 | +3 = 43 PASS |
| **B4** | Phase 203 fp-plugin-search placeholder | _rust_module/fp-plugin-search/ 신규 (lib.rs + Cargo.toml + fp-plugin.toml 매니페스트) | +4 = 47 PASS |

### Phase 202 본진입 핵심 산출

| 영역 | 변경 |
|------|------|
| 신규 파일 | `_rust_module/fp-plugin-sdk/src/connection.rs` (220줄, cross-platform IPC) + `core/plugin/connection_pool.rs` (90줄, lazy connect + 캐시) |
| `core::plugin::PluginRegistry::call` | `Err(IpcNotYetImplemented)` 완전 제거 → 실 IPC + audit prepend + 결과 분기 |
| `core::plugin::PluginRegistry::broadcast_event` | 신규 — event_kind serde tag 추출 + enabled + event_subscribe 매칭 plugin만 send_event (실패 silent) |
| `PluginError` enum | `NotRunning {plugin_id, cause}` + `IpcTransport(String)` + `IpcProtocol(String)` 추가 / `IpcNotYetImplemented` 삭제 |
| `FileProcessingService` | `plugin_registry: Arc<PluginRegistry>` 필드 추가 |
| `McpState` | `plugin_registry: Arc<PluginRegistry>` 필드 추가 (라우팅은 B4 미진입, 필드만 노출) |
| `ServiceBuilder` | `with_plugin_registry()` + build() 디폴트 `Arc::new(PluginRegistry::new())` (lesson 21/27 회피 핵심) |
| `build_service` | `PluginRegistry::new().with_audit(Arc::clone(&audit)).discover(&paths.plugins)` 통합 |
| audit stage 신규 | `plugin.{plugin_id}.{method}` (변수 기반) — 메타 룰 24 정합 |

### IPC wire 포맷

- newline-delimited JSON (`BufReader::read_line` + `write_all`/`flush`)
- 단일 connection 1쌍 양방향 (request/response + event)
- cross-platform: `#[cfg(windows)] tokio::net::windows::named_pipe::*` / `#[cfg(unix)] tokio::net::UnixStream`
- endpoint: `\\.\pipe\fp-plugin-{id}` (Windows) / `/tmp/fp-plugin-{id}.sock` (Unix)

### 회귀 자동화 확장 (B3)

`spec/benchmarks/scripts/audit_stage_check.sh`:
- ALLOWED prefix에 `plugin` 추가
- 신규 검사 패턴 — `let stage = format!("...")` 변수 기반 stage (Phase 202 본진입 시 등장한 새 패턴)
- 자동화 9종 → **자동화 10종** (audit_stage_check v2 분기)

### file-pipeline 단독 git 저장소 첫 설정

본 세션 진입 직전 git 미초기화 상태 발견 → 사용자 합의로 `C:\dev\claude_workspaces\file-pipeline` git init + `http://gitlab.bi.co.kr/reujea/file.git` origin. 단독 저장소 (메타 룰 22 16건째 사용자 정책 경계 합의).

- src/.git 잔존 (master, 4/14 생성) — 사용자 합의로 완전 삭제
- _rust_module은 별도 워크스페이스 — file-pipeline git에 미포함 (현재 사이클 _rust_module 변경 추적 부재 = lesson 76 후속 트리거 후보)
- 4 commit (baseline + B2 + B3 머지) — B4는 file-pipeline 측 변경 0이라 git 영향 없음

### 본 세션 메타 룰 누적

| 메타 룰 | 변경 |
|---------|------|
| **22 사용자 정책 경계 합의** | 15 → **17건** (본 세션 +2: git 저장소 단독 설정 / src/.git 완전 삭제) |
| **17 release 재빌드 + 배포 의무** | 강화 정식 후속 — 본 세션은 Linux 원격 빌드만, Windows cross 빌드는 후속 트리거 |
| **24 stage 명명 규칙** | `plugin.*` 영역 신규 + 변수 기반 stage 검사 자동화 — 메타 룰 24 정식 승격 후보 강화 |
| **30 spec 본문 phase별 즉시 갱신** | 자기 적용 12건째 (본 architecture + domain-map + roadmap + deprecated 동시 갱신) |
| **25 자기 적용 의무** | 9건째 — bundle-cycle 사이클 종결 직후 즉시 현행화 |

### 후속 트리거

- **Windows cfg 분기 검증** — 본 세션 Linux 원격 빌드만. `cargo-xwin` (lesson 71)으로 named_pipe cfg 분기 별도 검증
- **Phase 203 본진입** — fp-plugin-search placeholder의 LocalVectorStore + MMR + vec_io 본체 이관
- **_rust_module git 추적** — 본 사이클 _rust_module/fp-plugin-search/ 등 변경이 file-pipeline git 밖. 단일 진실원 위반 가능성 (메타 룰 19 후보)
- **Phase 207 어댑터 plugin 변환** — 24 어댑터 (embedding 6 / llm 7 / storage 5 / notify 2 / rerank 3 / verify 1)에 bin target 추가

---

## 누적 변경 요약 (2026-06-05 후속) — Phase 200~202 placeholder 진입 (lesson 75 + Q2/Q3 후속)

### 본 세션 후속 결정 시계열

1. **"외부 형제 모듈은 어떻게 추가해?"** → lesson 16/17 패턴 정형화 보고 (6단계 절차)
2. **"형제 모듈과 현재 프로젝트는 별도 빌드 하고 ... plugin 폴더 초기화 → plugin 폴더에 형제 모듈 빌드된 바이너리 추가"** → binary plugin 4축 사용자 합의 (메타 룰 22 13~15건째)
3. **"개발/빌드는 원격서버에서만 실행"** → feedback_remote_build_only 강화 (cargo check까지 확장)
4. **"Q2 진행해"** → Phase 201 placeholder 진입 (PluginRegistry + PermissionGate + PluginManifest)
5. **"Q2 → Q3"** → Phase 202 wire 프로토콜 placeholder (IpcMessage / IpcResponse / HostEvent) + 본 §누적 변경 요약 추가

### binary plugin 4축 사용자 합의 (메타 룰 22 단일 세션 +3 첫 사례)

| 축 | 결정 |
|----|------|
| Q1 빌드 위치 | `_rust_module/` 별도 워크스페이스 (형제 모듈 워크스페이스 = plugin 워크스페이스) |
| Q2 형제 모듈 ↔ plugin 관계 | **형제 모듈 = plugin** (lib + bin 듀얼, Phase 207 시점 bin target 추가) |
| Q3 런타임 plugin 폴더 | `PIPELINE_BASE/plugins/` 첫 실행 자동 생성 (lesson 29 패턴) |
| Q4 Phase 200 진입 | 본 세션 즉시 (§2-A 갱신 + placeholder 동시) |

상위 단일 진실원: `prd/research/plugin-architecture-2026-06-04.md` §2-A (2026-06-05 재정의)

### Phase 200 placeholder 산출 (lesson 16 단계 0)

- `_rust_module/fp-plugin-protocol/` 신규 — `API_VERSION=1` + `ApiVersion` 타입 별칭 + `EntryKind::Process { command }` + `ProtocolError` 5 variants
- `_rust_module/fp-plugin-sdk/` 신규 — `Plugin` trait (placeholder) + `SDK_API_VERSION` + `SdkError` + protocol 재노출
- `_rust_module/Cargo.toml` workspace.members 24 → **26**
- `file-pipeline/src/crates/shared/src/config.rs` — `ResolvedPaths.plugins` 필드 + `PIPELINE_PLUGINS` env 분기 + `create_all` 자동 생성

### Phase 201 placeholder 산출 (lesson 16 단계 1, Q2)

- `_rust_module/fp-plugin-protocol/Cargo.toml` +1 의존 (`toml`)
- `_rust_module/fp-plugin-protocol/src/lib.rs` — `PluginManifest` + `Contributes` + `ContributedMcpTool` + `parse_manifest_toml`
- `_rust_module/fp-plugin-sdk/src/lib.rs` — re-export 확장
- `file-pipeline/src/crates/core/Cargo.toml` +2 의존 (`thiserror` + `fp-plugin-protocol` path)
- `file-pipeline/src/crates/core/src/plugin/` 신규 모듈 4 파일:
  - `mod.rs` — 재노출 + Phase 진행 docstring
  - `permission_gate.rs` — `KnownPermission` 12종(VectorDbRead / AuditWrite / ConfigRead / ConfigWrite / SnapshotWrite / SignalRead / TodoRead / TodoWrite / LlmCall / CacheRead / CacheWrite / FsWrite) + `PermissionGate`
  - `handle.rs` — `PluginHandle` + `PluginState::{Discovered, Enabled, Disabled}`
  - `registry.rs` — `PluginRegistry::{discover, enable, disable, call, count, list, get}` + `PluginError` 7 variants

discover 동작:
- `PIPELINE_BASE/plugins/{plugin_id}/fp-plugin.toml` 발견 + 파싱
- api_version 불일치 / 알 수 없는 권한 / 중복 id → **전체 discover 중단** (부분 등록 회피)
- 디렉토리 부재 / 빈 디렉토리 / 매니페스트 부재 → 정상 (0개 부팅)

### Phase 202 placeholder 산출 (wire 타입만, Q3 직전)

- `_rust_module/fp-plugin-protocol/src/lib.rs` 끝부분에 wire 타입 추가:
  - `IpcMessage { trace_id, method, api_version, params }` — host → plugin 요청 envelope
  - `IpcResponse::{Ok, Err}` (status tag) + `trace_id()` 단축 접근
  - `HostEvent::{ProcessingStarted, ProcessingCompleted, QuarantineAdded, VerifyFailed, ShutdownRequested}` (kind tag, snake_case)
- `fp-plugin-sdk` 재노출 확장
- `core::plugin::registry::PluginError::IpcNotYetImplemented` 메시지 명료화 ("wire 타입은 정의, 전송은 다음 진입 시점")

실제 IPC 전송 (named pipe / Unix domain socket)은 다음 세션 본진입.

### method 명명 규칙 (메타 룰 24 정합)

`{영역}.{도구명}[.{sub}]` — Phase 95 audit stage 명명 규칙과 동일 패턴 직접 흡수:
- `search.execute` / `search.cached`
- `kg.neighbors` / `kg.paths`
- `lint.strong_claims`
- `remote.{backend}.upload.{sub}` (이미 메타 룰 24 후보 표에 등재)

### 본 세션 누적 단위 테스트 (예정, 원격 검증 위임)

| 모듈 | 테스트 수 |
|------|---------|
| fp-plugin-protocol (Phase 200/201/202) | **11** (2 placeholder + 4 manifest + 5 wire) |
| fp-plugin-sdk | 2 |
| core::plugin::permission_gate | 4 |
| core::plugin::registry | 9 |
| **합계** | **26** |

### 메타 룰 17 강화 정식 승격 (M-1, 본 세션 초반)

| 위치 | 변경 |
|------|------|
| `META.md` §메타 룰 17 | 후보 3 섹션 분산 → 1 §정식 + 2 위임 표시 (메타 룰 19 자기 적용 8건째) |
| 누적 사례 | 6건 (정식 3 + 강화 3: Phase 106 / Phase 107 / lesson 71 cross-build) |
| 자동화 도구 | `release_rebuild_required.sh` (게이트, Phase 97) + `release_redeploy.sh` (게이트, 본 세션 신규) |
| 적용 영역 | release 재빌드 + D:\file-test 잔류 binary 감지 + sha256 검증 |

### 메타 룰 27 정식 승격 (M-3, 본 세션 중반)

- 누적 3건 도달 (Phase 98 dead_selector_scan_v3 + 본 세션 release_redeploy 게이트 + single_source_check 점검)
- 5축 분류 매트릭스 명문화 (정밀도/결정성/외부 의존/exit code/CI 통합)
- 자기 적용 4건 (lesson 71 / B-8 표 / release_redeploy.sh docstring / single_source_check.sh docstring)
- `external-trigger-checklist.md` B-8 행 ✅ 정식 승격 표시

### 메타 룰 30 sub-rule "도구 stale" 후보 등재 (G-1f)

- 1건 누적 (lesson 74 G1 — gui_http_smoke 7탭 stale 즉시 해소)
- 회귀 게이트의 검출 범위 확장: **코드 회귀 → 코드 회귀 + 도구 stale**
- 누적 가능 영역: dead_selector / action_catalog / audit_stage_check / release_rebuild_required

### 회귀 자동화 9종 (G-5 5종 + Phase 97 +2 + 2026-06-05 +2)

본 세션 신규 2종:
- `release_redeploy.sh` (게이트, 메타 룰 17 강화 §자동화)
- `single_source_check.sh` (점검, 메타 룰 19/30 §자동화)

### 본 세션 종결 시 메타 룰 상태 전이

| 메타 룰 | 본 세션 전 | 본 세션 후 |
|---------|----------|----------|
| 정식 누적 | 13건 | **15건** (17 강화 + 27 추가) |
| 후보 잔여 | 4건 | **2건** (24/31) + sub-rule 후보 1건 (G-1f) |
| 17 강화 사례 | 3건 | 6건 (정식 3 + 강화 3, 단일 정식 본문 통합) |
| 19 자기 적용 | 7건 | **9건** (74 (M-1) + 74 (S-5)) |
| 22 누적 사례 | 10건 | **15건** (73 mydocsearch + 74 6 묶음 + 75 (a)(b)(c) = +5건 단일 세션) |
| 25 자기 적용 | 3건 | **8건** (74 (a~e) + 75 Phase 201 진입) |
| 27 정식 누적 | — | **3건** (Phase 98 + 본 세션 +2) |
| 30 정식 누적 | 6건 | **10건** (74 (a)(b)(c)(d)) + sub-rule 후보 1건 |

### Phase 진행 표 (plugin-architecture-2026-06-04.md §6 동기)

| Phase | 상태 |
|-------|------|
| **200** | ✅ 2026-06-05 (placeholder 통과) |
| **201** | ✅ 2026-06-05 (PluginRegistry placeholder, Q2) |
| **202** | 🟡 wire 타입 placeholder 완료 (Q3 직전), 실 전송은 다음 세션 |
| 203 | ⏳ fp-plugin-search 진입 대기 |
| 204~207 | ⏳ |
| 208 | ⏳ GUI Plugins 탭 |
| 209 | ⏳ 회귀 게이트 + 측정 |

---

## 누적 변경 요약 (2026-06-05) — 본 세션 6 묶음 처리 + Phase 200 진입 전 정비 (lesson 74)

### 사용자 트리거 시계열

1. **"spec 폴더 분석해"** → 8 본문 + 72 lesson + META 862줄 분석 → Q1 mydocsearch 즉시 이관 결정
2. **lesson 73 등재** — "Phase N 진입 시 처리" 표기 spec 즉시 처리 정형화 (메타 룰 22+19 결합)
3. **다음 고도화 항목 5축 22건 정리** → M-1 (메타 룰 17 강화 정식 승격) 우선 진행
4. **"1~6 진행해"** 단일 트리거로 **6 묶음 처리** → 사이드 G1/G2 발견
5. **"Q1, Q2 포함해서 프로젝트 현행화 해"** → 본 §누적 변경 요약 신규 + spec/prd 일괄 갱신

### 처리 결과 (6 묶음 + lesson 74 + 현행화)

| 작업 | 핵심 산출 |
|------|---------|
| M-1 메타 룰 17 강화 정식 승격 | META.md 3 섹션 분산 → 1 §정식 + 2 위임. 누적 6건 (정식 3 + 강화 3) |
| S-4 spec "Phase N 시" 표기 grep | architecture.md:145 Pipeline 이관 → 옵션 A 결정 완료 표시 추가 |
| S-1 webapp-design.md 헤더 갱신 | updated 2026-06-01 → 2026-06-05 + status_note 추가 |
| S-6 release_redeploy.sh 신규 | D:\file-test 잔류 binary 감지 + sha256 + --check/--apply 분리 (메타 룰 17 강화 §자동화) |
| P-2 회귀 게이트 baseline 측정 | `gate_baseline_phase200pre_20260605.json` 보존 + 사이드 G1 즉시 해소 |
| S-3 archive ↔ deprecated 추가 발굴 | lesson 49 옵션 A 완전성 재검증 — 추가 누락 0건. 사이드 G2 발견 |
| S-5 single_source_check.sh 신규 | spec 5종 위임 누락 후보 출력 (메타 룰 19/30 §자동화) |
| lesson 74 등재 | 6 묶음 + G1/G2 + 회귀 게이트 자체 stale 자기 검출 메커니즘 첫 사례 |
| G2 architecture.md 수치 stale 동기화 | 본 §누적 변경 요약에 최신 측정값 흡수 (시간축 보존 영역은 유지) |

### 최신 실측 수치 (2026-06-05 baseline)

| 지표 | 본 세션 측정 | 이전 기록 | 변동 |
|------|------------|---------|------|
| `action_catalog.sh --count` | **72** | 68 (G-6 후 2026-05-20) | +4 (Phase 107 +1 / Phase A/B/E / 본질 재정의 누적) |
| `dead_selector_scan.sh` ID 수 | **94 ID PASS** | 92 (Phase 93) → 88 (Phase 92) | +2 (Phase 93~107 누적) |
| `gui_http_smoke.sh` | **5/5 PASS (6탭)** | 7탭 grep stale | 사이드 G1 — Phase 107 통합 미반영 즉시 해소 |
| `audit_stage_check.sh` | **PASS** (10 정적 + remote.* 동적) | — | 변동 없음 |
| `release_rebuild_required.sh` | **FAIL (13 변경 파일)** | — | Phase 107 + Phase A/B/E + lesson 71 cross-build 누적 — 메타 룰 17 의무 미준수 가시화 (사용자 결정 트리거 대기) |
| `release_redeploy.sh` | exit 1 (source binary 부재) | — | 본 세션 신규 게이트 (위 release_rebuild_required와 의존 체인) |
| `single_source_check.sh` | PASS (spec 5종 위임 누락 0건) | — | 본 세션 신규 점검 도구 |
| `empty_state_audit.sh` | PASS (빈 객체 0건) | — | 변동 없음 |
| Tauri commands | **66 (변동 없음)** | — | 본 세션 코드 변경 없음 (자동화/문서만) |

### 회귀 자동화 9종 (G-5 5종 + Phase 97 +2 + 2026-06-05 +2)

| 분류 | 스크립트 | 적용 메타 룰 |
|------|---------|-----------|
| 게이트 | `dead_selector_scan.sh` | sub-rule 1a + lesson 47 |
| 게이트 | `gui_http_smoke.sh` | sub-rule 1a + Phase 107 6탭 반영 |
| 게이트 | `audit_stage_check.sh` | 메타 룰 24 후보 |
| 게이트 | `release_rebuild_required.sh` | 메타 룰 17 §1단계 |
| **게이트 (신규)** | `release_redeploy.sh` | **메타 룰 17 강화 §2단계 (2026-06-05 신규)** |
| **점검 (신규)** | `single_source_check.sh` | **메타 룰 19/30 + sub-rule 1g (2026-06-05 신규)** |
| 카탈로그 | `action_catalog.sh --count` | 메타 룰 1 sub-rule 1a |
| 점검 | `empty_state_audit.sh` | sub-rule 1a (G-4) |
| 디버깅 | `data_flow_trace.sh <action>` | lesson 46 |

### 메타 룰 상태 전이 (본 세션 누적)

- **메타 룰 17 강화**: 후보 3 섹션 → **정식 승격** (강화 영역 누적 6건 = 정식 3 + 강화 3)
- **메타 룰 19 누적 사례**: 7건 → **9건** (74 (M-1) + 74 (S-5))
- **메타 룰 22 누적 사례**: 10건 → **12건** (lesson 73 + lesson 74)
- **메타 룰 25 누적 사례**: 3건 → **8건** (lesson 74 (a~e) 5건 자기 적용)
- **메타 룰 27 누적 사례**: 1건 → **3건** (release_redeploy 게이트 + single_source_check 점검)
- **메타 룰 30 누적 사례**: 6건 → **10건** (74 (a) S-1 + (b) S-4 + (c) S-5 + (d) G2 본 흡수)
- **lesson**: 73 → **74** 등재
- **회귀 자동화**: 7종 → **9종**
- **메타 룰 자동화 도구**: 2건 → **4건**

### Phase 200 시리즈 진입 전 baseline 보존

`spec/benchmarks/gate_baseline_phase200pre_20260605.json` — Phase 209 종결 시 비교 기준 (메타 룰 4 3회 중앙값). 본 baseline은 코드 변경 없는 자동화 정비 직후 측정값이라 Phase 200~209 회귀 측정의 신뢰 기준.

### 사이드 발견 G1 / G2 메타 가치

- **G1 (gui_http_smoke 7탭 stale)**: P-2 baseline 측정 자체가 도구의 stale을 자기 검출 — 회귀 게이트가 코드 회귀뿐 아니라 **도구 자신의 stale도 자기 검출** 새 패턴 첫 사례. 메타 룰 30 sub-rule "도구 stale" 신규 후보 (3건 누적 도달 시 정식)
- **G2 (architecture.md 수치 stale)**: 본 §누적 변경 요약 신규 등재로 흡수. 시간축 보존 영역(326/399/485/568 등)은 그대로 유지 (Phase 92/93/91 결정 맥락)

---

## 누적 변경 요약 (2026-06-04) — 본질 재정의 2차 (tasty 패턴 흡수)

### 사용자 본질 재정의 결정 (메타 룰 22 10건째 누적, Phase 200 시리즈 진입 전)

| 항목 | 결정 |
|------|------|
| 본질 도메인 | **file-pipeline = 파일 가공만 (host 최소)** |
| host 잔류 | watcher + WorkQueue + Preprocess (PDF/Excel/한글) + Chunk + Metadata 구조 + DB 영속 + audit 코어 + Plugin Registry + Tauri 진입점 (~~MCP server~~ — MCP 전체 폐기 2026-06-17, host 잔류 아님) |
| plugin 이관 | LLM / 임베딩 / 검증 / 분류 / 검색 / KG / lint / CrossRef / Topic / Wiki Export / 추천 / 알림 / 첨부 / 링크 / Storage 백엔드 |
| 추상화 패턴 | tasty 패턴 직접 흡수 — workspace + 별도 프로세스 plugin + IPC + 매니페스트 + permission gate |
| 진행 단위 | Phase 200~209 (lesson 16 패턴 단계 0~9) |
| 이전 결정 처리 | search-extraction-plan.md (Phase 108 검색 분리, 2026-06-01) 무효화 → deprecated.md 단방향 위임 |

**단일 진실원**: `prd/research/plugin-architecture-2026-06-04.md` (2026-06-04). §3 host 잔류 경계 / §4 plugin 28 MCP 매핑 / §5 SDK + Protocol 정의 / §6 Phase 200~209 단계 / §7 위험 매트릭스 / §9 기존 흡수 영역 재배치. (※ "MCP 매핑"은 MCP 전체 폐기 2026-06-17로 무효 — plugin 경계는 유효하되 MCP 프로토콜이 아닌 일반 IPC plugin)

### 결정 사실

- **search-extraction-plan.md 무효화** — 2026-06-01 검색 분리 결정이 본 plugin 결정의 부분집합 (fp-plugin-search 하나로 자연 흡수). spec/deprecated.md 등재 의무
- **mydocsearch_decision.md 삭제 완료** (2026-06-05) — 2026-04-08 결정 폐기 사실은 `spec/deprecated.md` 단일 진실원 위임. 사용자 합의로 Phase 203 대기 없이 즉시 삭제
- **6 도메인 → 1 host + 11 plugin + 24 어댑터 plugin** — ~~28 MCP 도구~~ / 23 어댑터 → 11 + 24 = 35 plugin 멤버 (※ MCP 전체 폐기 2026-06-17 — plugin은 일반 IPC, MCP 도구 수치 무효)
- **외부 흡수 정책 진화** — lesson 30 "인프라 선구현" 자연 진화: plugin 단위 발행으로 host 영향 0

### 사이드 발견 (본 결정으로 자연 해소)

- ~~MCP 카탈로그 단일 진실원 분산 우려 → `PluginRegistry::mcp_router`가 매니페스트 수집 → 동적 카탈로그~~ (Phase 92 H3 일치성 테스트 패턴 자연 흡수) — ※ MCP 전체 폐기 2026-06-17로 무효 (mcp_router 미구현, plugin 카탈로그는 일반 IPC 매니페스트로 대체)
- 메타 룰 1 sub-rule 1f (단일 진입점) 비대화 → plugin 단위 자연 해소
- Tauri commands 67 → host 직접 등록 + plugin contributes 자연 분산 → 단일 진실원 회복

---

## 누적 변경 요약 (2026-06-01, 무효화) — 검색 분리 Plan (본 결정으로 흡수)

> ⚠️ 본 §는 2026-06-04 결정으로 **무효화**. fp-plugin-search 하나로 자연 흡수. 시간축 보존을 위해 본문은 유지하되 결정은 위 §2026-06-04 참조.

### 사용자 본질 재정의 결정 (메타 룰 22 7건째 누적, Phase 108 진입 전 — 무효화)

| 항목 | 결정 |
|------|------|
| 본질 도메인 | **file-pipeline = 가공 + 추천 + 검증 전담** (3 도메인) |
| 분리 대상 | 검색 + KG + 임베딩 + 리랭커 + Topic Merger (검색 측) |
| 분리 위치 | `C:\dev\claude_workspaces\_rust_module\` (기존 `module/`과 별도 신규 워크스페이스) |
| 진행 단위 | lesson 16 패턴 단계 0~8 (Phase 108~115 추정) |
| 선행 의무 | Phase 107 release 재빌드 (메타 룰 17) |

**단일 진실원**: `prd/research/search-extraction-plan.md` (2026-06-01, status: draft). §3 8개 논점 + §1.3 MCP 36 도구 정밀 분류 + §5 단계 분할 + §6 위험 매트릭스 + §8 결정 매트릭스 + §9 진입 흐름. 사용자 합의 후 Phase 108 진입.

### 결정 사실

- **mydocsearch_decision.md 무효화 예정** — 2026-04-08 "LocalVectorStore 단일" 결정이 본 plan으로 폐기. Phase 108 단계 7에서 deprecated.md 이관 (⚠️ 2026-06-04 본질 재정의 2차로 본 plan 무효화 + 2026-06-05 사용자 합의로 spec 파일 즉시 삭제 완료 — 위 §2026-06-04 참조)
- **6 도메인 → 3 도메인 축소** — 도메인 1(가공) / 5(추천) / 6(검증·lint) 잔류 / 도메인 2(검색) 외부 이관 / 도메인 3(저장) module/ 분리 완료 + Notion 잔여분 / 도메인 4(알림) module/ 분리 완료 + AuditPort 본질 영역
- **분리 후보**: 약 4,200줄 (검색 어댑터 2,221 + 검색 의존 core 1,987)
- **MCP 분할**: 외부 8 / 잔류 28 (실측 36 정밀 분류, §1.3)

### 사이드 발견 (메타 룰 1 sub-rule 1f 자기 적용 트리거)

- MCP 도구 수치 불일치: `architecture.md` "11" / `webapp-design.md` "25" / 실측 36. Phase 102 이후 누적. 본 plan 단계 7에서 일괄 동기화 (메타 룰 30 4번째 자기 적용 사례 → META 정식 승격 가능 시점)

### 다음 세션 진입 흐름

1. release 재빌드 (선행 의무, Phase 107 누적분)
2. plan §3 8개 논점 사용자 합의 (claude 추정 일괄 채택 권장)
3. 회귀 게이트 7종 baseline 측정
4. Phase 108-0 placeholder workspace 생성
5. Phase 108-1 vectordb-search-api 분리 진입

---

## 누적 변경 요약 (2026-05-29) — Phase 107 dev seed + watch() WorkQueue 통합 + GUI 검증 묶음

### Phase 107 — 8건 묶음 (lesson 66)

사용자 트리거 "dev 모드 자동 credential 세팅" → cargo run 검증 흐름에서 회귀 5건 연쇄 발견 후 일괄 해소. 본 phase 8 영역:

1. **dev seed credential** — `crates/shared/src/lib.rs::dev_seed_credential()` (cfg debug_assertions). CLAUDE_PROFILE_PATH env → `C:\dev\ide\claude\profiles\reujea` 폴백 → `%USERPROFILE%\.claude`. settings.db 미저장 (in-memory) — release 환경 오염 방지. `modals/app/src/service.rs::init_app_state`에서 credentials 0건 시 주입.
2. **release 온보딩 자동 시작** — `dashboard.js::init()` credential 0건 시 `startOnboarding()` 자동 호출 (300ms 지연 후). dev 환경은 백엔드 seed로 자동 발동 안 됨.
3. **qdrant dead path 6곳 제거** — `PathsConfig.qdrant` (Option<String>) + `ResolvedPaths.qdrant` (PathBuf) + `create_all()` 자동 생성 + cli.rs/main.rs print 2곳. Phase 65 Qdrant 어댑터 제거 후 path 영역 잔존 (메타 룰 1 sub-rule 1f path 영역 확장).
4. **watch() WorkQueue 통합** — `crates/adapters/src/driving/watcher.rs::watch()`에 queue_mutex + queue_path Arc 추가. spawn 안 ensure_item + mark_processing + mark_done/failed + 즉시 save. batch_process는 종료 1회 save 패턴이었음 → batch도 spawn 안 즉시 save로 변경 (5분 batch 도중 진행률 가시화).
5. **WorkQueue::ensure_item helper** — `core/domain/work_queue.rs`. items 부재 시 file metadata(hash/size/is_large) 자동 채워 Pending 등록. mark_processing의 silent no-op 회피. idempotent.
6. **ensure_item을 semaphore.acquire 이전으로 이동** — max_workers=4 환경에서 5+번째 파일은 acquire 대기 중에 work-queue 미등록 → 대시보드 카운트 누락 발생 → acquire 전 등록으로 해소 (추정 빗나감 10번째).
7. **R-1/R-2c dashboard 버그 + 즉시 갱신** — `refreshDashboard` qData.processing → qData.stats.processing (응답 구조 정합). progress 채널 활용 — start 이벤트 도착 시 200ms 후 `_refreshQueueOnly()` 추가 호출.
8. **row 정렬 단일 키화** — `_renderProcTable` created_at asc 단일 정렬. status 1차 정렬 제거 → 행 점프 차단.
9. **get_file_log Tauri command + 풍부화 로그 표시** — `commands.rs` pipeline.log + pipeline.log.{date} 파일별 라인 추출. main.rs invoke_handler 등록. `dashboard.js::showProcessingLog` 3 섹션 (한글 라벨 처리 이벤트 + 큐 상태 + Pipeline Log raw 라인). claude_cli 호출/응답 trace 가시화.
10. **Processing + Verification 탭 통합** — index.html 7→6탭. processing 탭 라벨 "처리 현황" + verification 콘텐츠(검증 결과 / 강한 주장 / anomaly) 흡수. dashboard.js switchTab 분기 통합.

#### 호환성 / 회귀

- workspace cargo check: 경고 0 / Tauri app cargo check: 경고 0
- get_queue 응답 schema +2 필드 (created_at, size_bytes) — frontend backward compat 유지
- 7탭 의존 외부 도구 없음 (verification 흡수)
- dev seed는 cfg(debug_assertions) gated — release 빌드에 영향 0

#### 측정

| 지표 | 값 |
|------|-----|
| Tauri commands | 65 → **66** (+get_file_log) |
| Dashboard 탭 | 7 → **6** |
| ResolvedPaths 필드 | 11 → **10** |
| WorkQueue 메서드 | 5 → **6** (+ensure_item) |
| WorkQueue 갱신 흐름 | batch만 → **batch + watch 양쪽** |
| 추정 빗나감 누적 | 9 → **10** (semaphore acquire 위치) |
| 메타 룰 1 sub-rule 1f | 6 → **8건** |
| 메타 룰 22 누적 | 5 → **6건** (dev seed in-memory) |
| 메타 룰 후보 | 신규 **31** (도메인 분류 vs 작업 흐름 IA) |
| lesson | 65 → **66** |

#### 사이드 발견

- `ensure_item`에서 `compute_file_hash` 호출 시 race로 hash="" 사례 (api-integration.md, 19:25:00 시각) — 다음 세션 후보
- `pipeline.toml` vs `pipeline.toml.bak` 사용자 혼란 (settings_db.open_or_migrate가 rename하는 설계 동작) — 메모리 보존
- frontend 5초 폴링 → Tauri event emit 전환 가능성 (이벤트 기반 push)

#### 메타 룰 적용

| 룰 | Phase 107 적용 |
|----|--------------|
| 1 sub-rule 1f | ResolvedPaths.qdrant + batch_process/watch 같은 의미 분산 (2건 추가) |
| 18 | semaphore.acquire 위치 추정 빗나감 10번째 (사용자 시점 질문 추가 메타 강화 후보) |
| 22 | dev seed in-memory 결정 (사용자 명시 합의) 6건째 |
| 23 | 메타 룰 31 후보 등록 (도메인 vs 작업 흐름 IA 트레이드오프) |
| 25 | 메타 룰 1 sub-rule 1f 자기 적용 — ensure_item 단일 진입점 추출 |
| 30 | 본 phase 종결 즉시 spec 본문 갱신 (자기 적용 3건째) |

---

## 누적 변경 요약 (2026-05-28) — Phase 105~106 프로젝트 현행화 + GUI 온보딩

### Phase 106 — GUI 온보딩 4-step (lesson 65)
사용자 트리거 "초기 실행 시 온보딩 기능 추가" — claude 초안(settings.db 영속화 + 자동 트리거)을 사용자 결정으로 "별도 DB 없이 상단 명시 버튼 + step별 입력"의 무상태 흐름으로 단순화 (메타 룰 22 5건째). 헤더 우측 🧭 온보드 버튼 + 4-step 모달 (환영/크레덴셜 등록+기본 설정/inbox 안내/optimize 가이드). 기존 Modal 유틸 + `showCredentialForm` 재사용. 추가 요청 "기본으로 설정 문의" 흐름 — 3 분기 (신규 / 기존 다른 → 덮어쓰기 / 이미 본인). 백엔드 변경 **0건** (Tauri commands 0 신규 / settings.db 미사용). Tauri release 22.09 MB. D:\file-test 재배포 19:29 (SHA-256 일치). **사이드 발견**: 1차 빌드 후 D:\file-test 재배포 누락 (이전 14:36 binary 잔류) — 메타 룰 17 강화 후보 (release 재빌드 → 자동 배포 연결).

### Phase 105 — 프로젝트 현행화 통합 (lesson 64)
사용자 명시 "프로젝트 현행화 해" 트리거 — comm-spec + comm-prd + comm-log 3 스킬 연계. Phase 100~104 종결 시 spec 본문 누적 stale 5건 발견 (architecture.md / domain-map.md / webapp-design.md). 본 phase 통합 갱신 + B-9 GraphRAG 트리거 4건(G1~G4) external-trigger-checklist 신규 등재. 메타 룰 19 위임 표시 9건째 자기 적용 + 메타 룰 30 후보 등록 ("phase 종결 시 spec 본문 즉시 갱신 의무"). 코드 변경 0건.

---

## 누적 변경 요약 (2026-05-27) — Phase 100~104 사용자 UX + 메타 정형화 + GraphRAG 흡수

### Phase 100 — Settings IA 5 운영 카드 좌측 그룹 이관 (lesson 59)
사용자 첫 GUI 사용 피드백 즉시 처리. `#settings-ops-cards` 단일 진실원 컨테이너 + DOM appendChild mount 패턴 (메타 룰 19 7건째 자기 적용). settings-nav 4→5 그룹 (운영 신규). Tauri release 21.05MB (+512 bytes).

### Phase 101 — 내부 코드명 UI 노출 제거 + Pipeline 이관 검토(✅ 옵션 A 이관 안 함 결정) (lesson 60)
사용자 질문 "C1은 무슨뜻?" 트리거. `(C1)/(C2)/(H1)/(Phase 92 H1)` 8건 일괄 제거. JS 함수명/HTML id는 추적성 보존. 메타 룰 28 후보 등재 (메모리 feedback_no_phase_in_ui 메타 승격). Pipeline 이관 3 옵션 보고 (옵션 A 이관 안 함 권장 — 메타 룰 1 위반 위험).

### Phase 102 — 비전문가용 통합 메타 MCP 도구 `optimize` (lesson 61)
사용자 시나리오 5단계 검증 후 자동화 갭 해소. `mcp__file-pipeline__optimize` 호출 1회로 C1 분석 + 누적 진행률 + 검토 대기 + 시나리오 권고 + next_actions 통합. 제안만 반환 (lesson 30 Ruflo 완전 준수). MCP 도구 24→**25**. handle_optimize +140줄. 추정 빗나감 9번째 (SetupAdvice 구조체 추정).

### Phase 103 — GraphRAG 흡수 4건 묶음 (lesson 62)
AWS GraphRAG Toolkit (Apache-2.0 엔터프라이즈 RAG) 부수 영역만 흡수. 본 프로젝트 단일 사용자 데스크톱 도메인 보존:
- **G1 Statement 노드**: `Metadata.statements: Vec<String>` 신규 (디폴트 빈 Vec)
- **G2 의미 관계**: `RelationType::Semantic(String)` variant 신규 (디폴트 미사용)
- **G3 Multi-hop 빔 검색**: `SearchConfig.kg_beam_search: bool` (디폴트 false)
- **G4 TF-IDF 다양성 재순위**: `SearchConfig.tfidf_rerank_enabled: bool` (디폴트 false)
- 모두 lesson 30 패턴 (인프라 선구현 + 디폴트 비활성)
- McpState 인스턴스 생성처 3곳 동기 갱신 (메타 룰 1 1f 회피)
- **영구 보류**: Neptune / Neo4j / OpenSearch / Bedrock / LlamaIndex / AWS boto3 (단일 바이너리 정책)
- 메타 룰 21 정식 승격 (TFM + Mirage + GraphRAG 3건 누적 도달)
- Tauri release 21.09MB (+9 KB)
- `prd/research/external-analysis-2026-05-27-graphrag.md` 단일 진실원 신규

### Phase 104 — 메타 룰 22/28 정식 + 메타 룰 9 중복 해소 (lesson 63)
사후 검증에서 Phase 103 메타 룰 21 정식 승격 시 후보 섹션 미삭제 발견 (메타 룰 19 자기 위반 회귀). 일괄 해소:
- 메타 룰 22 정식 승격 (Phase 92/94/100/103 누적 4건)
- 메타 룰 28 정식 승격 (Phase 76/92/101 누적 3건)
- 메타 룰 9 번호 중복 해소 → 메타 룰 29 (외부 문서 권고 3단계 분리)
- 메모리 `feedback_no_phase_in_ui.md` → spec 메타 룰 28 정식 승격 (첫 메모리→spec 메타 승격 사례)
- 코드 변경 0건

---

## 누적 변경 요약 (2026-05-26) — Phase 96~99 메타 작업 + C 항목 9/9 완료

### Phase 96 — 메타 룰 자기 적용 묶음 (lesson 55)
C-3 추정 키워드 grep 재검증 (lesson 42 ✅ Phase 89 36건 0% 해소 / lesson 46 G-1 ❓ audit_trace 누적 미도달) + C-2 메타 룰 1 sub-rule 7 카테고리 상세화 + C-4 메타 룰 19 spec/domain-map.md 단일 진실원 선언. 메타 룰 25 후보 등록.

### Phase 97 — A3 영역 완성 + 자동화 2건 (lesson 56)
C-1 trace_id 잔여 3 호출처 부착 (handle_get_document / handle_list_documents / verify_reprocess, 총 9→12) + C-9 `audit_stage_check.sh` 자동화 + C-7 `release_rebuild_required.sh` 자동화. 회귀 게이트 5→7종. 추정 빗나감 8번째 (match 케이스 스코프 — 메타 룰 26 후보). CLI 17.99MB / GUI 21.08MB.

### Phase 98 — 위생 묶음 + C 항목 9/9 완료 (lesson 57)
C-5 benchmarks/ 124→12 JSON (archive/phase47-64-2026-04/ 112건 분리) + C-6 dead_selector_scan_v3 (CSS rule scanner, 점검 도구로 분류 — 메타 룰 27 후보) + C-8 webapp-design.md 전면 재작성 (337→187줄, Phase 56 자문 컨텍스트 폐기). release_rebuild_required.sh 자동 판정 첫 자기 적용 (PASS).

### Phase 99 — 메타 룰 23/25/26 정식 승격 (lesson 58)
메타 룰 23 정식 (승격 3요소 AND 조건 — 누적 ≥3건 + 체크리스트 + META 등재) + 메타 룰 25 정식 (자기 적용 의무) + 메타 룰 26 정식 (match 케이스 스코프). 후보 22/24/27 본문 META 등재 (메타 룰 19 자기 위반 해소). 정식 20→23, 후보 7→4.

---

## 누적 변경 요약 (2026-05-22) — Phase 95 trace_id 영역 확장 + release 재빌드

> Phase 94 직후 trace_id 부착 영역 확장 4건. 메타 룰 13 2단계 완성도 향상. Phase 94 service.rs/mcp_server.rs/modals/app/service.rs 변경 반영 release 재빌드.

### A3 trace_id 부착 영역 확장 (4 신규 호출처)

| 영역 | 위치 | stage 이름 |
|------|------|-----------|
| Tauri search | `commands.rs::search` | `tauri.search` |
| MCP kg_neighbors | `mcp_server.rs::handle_kg_neighbors` | `mcp.kg_neighbors` |
| MCP kg_paths | `mcp_server.rs::handle_kg_paths` | `mcp.kg_paths` |
| 원격 저장소 업로드 | `service.rs::process_file_with_pipeline` (4 sub-branch: processed/origin × 성공/실패) | `remote.{backend}.upload.{processed\|origin}` |

stage 명명 규칙: `{영역}.{도구명}` 또는 `{영역}.{도구명}.{sub}`. backend 동적 (capability.backend) — Notion/S3/WebDAV/Network 구분 가능.

### Phase 91~94 누적 trace_id 부착 영역 (총 9 호출처)

| Phase | 영역 | stage |
|-------|------|-------|
| 91 | (인프라만) | — |
| 94 | service.rs LLM classify | `llm.classify` (3 sub: text/file/verify, ok/err 모두 기록) |
| 94 | mcp_server.rs search | `mcp.search` / `mcp.search.cached` |
| **95** | **Tauri search** | **`tauri.search`** |
| **95** | **MCP kg** | **`mcp.kg_neighbors` / `mcp.kg_paths`** |
| **95** | **원격 업로드 4분기** | **`remote.{backend}.upload.{processed\|origin}` ok/err** |

총 9 호출처 부착 → 메타 룰 13 2단계 완성도: Phase 94 부분 → **Phase 95 완전**.

### Release 재빌드 (메타 룰 17 의무 이행, ✅ 완료)

| 바이너리 | 크기 | 시각 | 반영 Phase |
|---------|------|------|-----------|
| `pipeline.exe` (CLI) | **17.88 MB** | 2026-05-22 11:17 | Phase 91~94 (Phase 95 영향 없음 — Tauri commands.rs만 변경) |
| `file-pipeline-tauri.exe` (GUI) | **21.02 MB** | 2026-05-22 11:33 | Phase 91~95 (Tauri release 1차 9m 20s + Phase 95 incremental 5m 47s) |

Phase 93 (17.85MB / 20.98MB) 대비:
- pipeline.exe **+32 KB** (A3 호출처 부착 9곳)
- file-pipeline-tauri.exe **+44 KB** (AuditPort trait + audit_anomaly + Tauri commands +4 + trace_id 부착)

### 메타 룰 적용

| 룰 | Phase 95 적용 |
|----|--------------|
| 메타 룰 1 sub-rule 1f (함수 분산) | trace_id 부착 = 단일 trait `AuditPort` 호출로 9 호출처 통일 |
| 메타 룰 13 (4단계) | A3 2단계 완성도 향상 — 9 핫패스 부착 |
| 메타 룰 17 (release 재빌드) | Phase 94 변경 반영 빌드 (메타 룰 17 자기 적용) |
| 메타 룰 18 (추정 재검증) | KgQueryResult 반환 타입 추정 → grep으로 `nodes/edges/paths` 필드 확인 (추정 빗나감 7번째 차단) |
| 메타 룰 19 (단일 진실원 위임) | stage 명명 규칙 단일 진입점 (`{영역}.{도구명}`) |

### 회귀 기준선

- workspace cargo check: 경고 0건
- workspace lib 테스트: **383 통과 / 0 실패** (Phase 94 변동 없음)
- workspace clippy: 0건
- Tauri cargo check: 통과
- workspace release 빌드: ✅ 2m 13s
- Tauri release 빌드: 진행 중

### 후속 트리거

- audit_trace 실측 (lesson 46 G-1 root cause 확정 시도) — H1 주기 호출이 누적 데이터 50건+ 도달 시 의미 있는 분석
- A3 부착 후속 영역: Notion 어댑터 자체에 audit (현재는 service.rs upload 호출 시점만 기록)
- 메타 룰 22 후보(사용자 정책 경계) 1건 추가 누적
- 메타 룰 23 후보(승격 기준) 정형화

---

## 누적 변경 요약 (2026-05-22) — Phase 94 인프라 활성화 + 메타 정형화

## 누적 변경 요약 (2026-05-22) — Phase 94 인프라 활성화 + 메타 정형화

> Phase 91~93 누적 후속 4건 묶음. A3 trace_id 호출처 부착 (메타 룰 13 2단계 완전 활성화) + H1 audit_anomaly 주기 호출 (3단계) + 메타 룰 1 sub-rule 분리 (19건 임계 해소) + 메타 룰 19 META 정식 승격.

### A3 trace_id 호출처 부착 — AuditPort 헥사고날 (사용자 명시 합의 A 옵션)

- `core/ports/output.rs::AuditPort` trait 신규 + `NullAuditAdapter` 디폴트 (lesson 14 회피)
- `shared/src/settings_audit_adapter.rs::SettingsAuditAdapter` 신규 — settings.db `audit_trace` 테이블 기록
- `FileProcessingService` 신규 필드 `audit: Arc<dyn AuditPort>` (lesson 21/27 회피: ServiceBuilder 자동 디폴트, 5 통합 테스트 파일 변경 0건)
- service.rs LLM 호출 시점 3 지점 부착: classify_and_process_text / classify_and_process / verify reprocess
  - 성공: `applied_rule="success"` + 결과 요약 (types + keywords count)
  - 실패: `applied_rule="error"` + 에러 메시지
- mcp_server.rs `handle_search` 진입에 trace_id 생성 + 응답 시 audit 기록 (캐시 hit 분기 포함)
- McpState 신규 필드 `audit` (3 생성처 갱신: cli/main.rs, shared/cli.rs, make_mcp_state)
- 메타 룰 13 2단계 (로직 활성화) 완전 달성

### H1 audit_anomaly 주기 호출 (메타 룰 13 3단계 진척)

- `modals/app/src/service.rs::c1-periodic` 주기 task에 `analyze_recent_audit` 호출 추가
- 이상 신호 발견 시 `warn!` 로깅 + GUI는 Phase 93에서 이미 노출됨 (Verification 탭)
- **자동 롤백 아닌 사용자 검토 권고만** (lesson 50 메타 룰 20 자기 적용)
- Phase 93 anomaly-report-card와 결합 시 메타 룰 13 4단계 도달 완성

### 메타 룰 1 sub-rule 분리 (19건 임계 해소)

7 카테고리로 분류:
- 1a UI 제거 패턴 (lesson 13, 19, 19+, 47)
- 1b 구조체 필드 추가 (21, 27, 35)
- 1c DB 스키마 (10, 26)
- 1d 미연결 포트/함수 (14, 31, 35)
- 1e 직렬화 4계층 (32)
- 1f 함수/검사 분산 (29, 38, 50-A, 50-B, 51, 52)
- 1g spec 자기 위반 (49, 28) — 메타 룰 19로 분기 후보

본문 시계열 표는 보존 (메타 룰 12 "잔존 종결 의무" 변형) + 신규 카테고리 표 헤더 추가.

### 메타 룰 19 META 정식 승격 (단일 진실원 위임 패턴)

후보 → 정식 룰 승격 (누적 5건):
- lesson 49 (spec 문서) / 50-A (classifier) / 50-B (Verifier) / 51 (MCP 카탈로그) / 52 (백엔드→frontend)

3축 분리 (What/Why/Link) + 3요소 동반 필수 (선언/grep/규칙) 명문화.

### 메타 룰 적용

| 룰 | Phase 94 적용 |
|----|--------------|
| 메타 룰 1 (다중 위치 동기화) | **sub-rule 1a~1g 분리** + AuditPort 추가 시 ServiceBuilder 디폴트 자동 (lesson 21/27 회피) |
| 메타 룰 9 (빌드 진단 자원 먼저) | `-j 2` 누적 적용 |
| 메타 룰 13 (인프라 활성화 4단계) | A3 호출처 부착 → 2단계 완전 / H1 주기 호출 → 3단계 |
| 메타 룰 18 (추정 재검증) | 사전 검증 grep — `paths.base` 없음 확인 → state.settings_db_path 사용 (Phase 93 패턴 재적용) |
| 메타 룰 19 (단일 진실원 위임) | **META 정식 승격** + AuditPort 단일 trait → 호출처 분산 통일 |
| 메타 룰 20 (도메인 정렬) | RBAC 보류 정책 자기 적용 — `audit.record` 실패는 silent (본 흐름 막지 않음) |

### 헥사고날 보존 결정 (사용자 명시 합의 A 옵션)

ClientB 안전 옵션(shared만)이 아닌 **A 옵션 (AuditPort trait + 6+ 호출처)** 선택. 이유:
- 메타 룰 13 2단계 완전 활성화 필요 (B 옵션은 service.rs 부착 보류)
- ServiceBuilder 패턴 덕분에 lesson 21/27 회귀 0건 (통합 테스트 파일 변경 0)
- 헥사고날 정공법 — core의 trait + shared의 adapter

### 회귀 기준선

- workspace cargo check: 경고 0건
- workspace lib 테스트: **383 통과 / 0 실패** (Phase 93 381 → 383, +2 신규 `settings_audit_adapter` 테스트)
- workspace clippy `--all --tests`: 0건
- Tauri cargo check: 통과
- dead_selector_scan: 92 ID 통과 (GUI 변경 없으므로 유지)

### 신규 파일

- `crates/core/src/ports/output.rs::AuditPort` trait + `NullAuditAdapter` (1단계 인프라 lesson 14 회피)
- `crates/shared/src/settings_audit_adapter.rs` (어댑터)

### 후속 트리거

- audit_trace 누적 후 H1 anomaly 실측 (lesson 46 G-1 같은 산발 실패 root cause 확정)
- A3 trace_id 부착 영역 확장 (Notion adapter / 검색 commands.rs / kg_paths 등)
- 메타 룰 22 후보(사용자 정책 경계) 1건 추가 누적 시 META 등록
- 메타 룰 21 후보(본질/부수 도메인) 1건 추가 누적 시 META 정식 승격

---

## 누적 변경 요약 (2026-05-22) — Phase 93 GUI 가시화 4건 묶음

## 누적 변경 요약 (2026-05-22) — Phase 93 GUI 가시화 4건 묶음

> Phase 91 후속 P0 3·4번 + Phase 92 H1/H3/H5 백엔드를 단일 phase로 GUI 가시화. 메타 룰 13 "인프라 활성화 4단계" 중 4단계(UI 노출) 도달. Tauri commands +4, dashboard.js +1 핸들러 그룹.

### 신규 Tauri commands 4건

| Command | 목적 | 출처 |
|---------|------|------|
| `get_anomaly_report` | audit_trace 최근 N건 분석 → AnomalySignal 반환 | Phase 92 H1 |
| ~~`get_mcp_tool_catalog_full`~~ | ~~26 MCP 도구 다차원 분류 (mutates/category/cost)~~ | Phase 92 H3 (※ Tauri command 삭제 + MCP 전체 폐기 2026-06-17) |
| `get_remote_storage_capabilities` | 활성 원격 저장소 capability + Notion mode | Phase 92 H5 |
| `get_pii_mask_config` | output_pii_mask 토글 상태 조회 | Phase 91 A2 |

Tauri commands 합계: G-7 후 61 → Phase 91 변동 없음 → Phase 93 **65건** (+4).

### Settings 탭 신규 카드 2건

- **🛡 출력 PII 마스킹**: `pii-mask-toggle` 체크박스. config.search.output_pii_mask ↔ save_config 연동 (lesson 12 secret 복원 패턴)
- ~~**🧰 MCP 도구 분류**: 26 도구 카테고리별 그룹화 + mutates/cost 컬럼. `refresh-mcp-catalog` 액션~~ (※ 카드 폐기 + MCP 전체 폐기 2026-06-17)

### Verification 탭 신규 카드 1건

- **🩺 자동 이상 감지** (`anomaly-report-card`): AnomalyReport.signals 렌더. **자동 롤백 아닌 사용자 검토 권고 명시** (lesson 50 메타 룰 20 자기 적용)
- 동적 생성 (renderAnomalyReport에서 verification-results 다음에 insert)

### Pipeline `remote_upload` 인스펙터 확장

- 노드 선택 시 활성 어댑터 capability 비동기 로드 (`_loadRemoteStorageCapInline`)
- 표시: backend / is_configured / can_upload·download·list·delete / supports_hard_delete / mode_options + active_mode
- **Notion attach 모드 선택 시 경고 표시**: "upload 명시적 미지원, S3/WebDAV 권장 또는 mode=page로 변경"

### dashboard.js 변경 통계

- 5 신규 함수: `loadAnomalyReport` / `renderAnomalyReport` / `loadMcpCatalog` / `renderMcpCatalog` / `loadPiiMaskToggle` / `togglePiiMask` / `loadRemoteStorageCapabilities` / `_loadRemoteStorageCapInline`
- 3 신규 action 핸들러: `pii-mask-toggle` / `refresh-mcp-catalog` / `refresh-anomaly-report`
- 4 신규 API 메서드: `anomalyReport` / `mcpToolCatalogFull` / `remoteStorageCapabilities` / `piiMaskConfig`
- 탭 진입 시 자동 로드: verification → anomaly, settings → pii mask + mcp catalog

### 메타 룰 적용

| 룰 | Phase 93 적용 |
|----|--------------|
| 메타 룰 1 (다중 위치) | MCP 카탈로그 백엔드 ↔ frontend 표시 일치 — 단일 진실원 백엔드 `mcp_tool_catalog_full()` |
| 메타 룰 13 (인프라 활성화 4단계) | **4단계 (UI 노출) 도달** — H1/H3/H5/A2 모두 1단계 인프라부터 4단계 GUI까지 완성 |
| 메타 룰 17 (release 재빌드) | Tauri commands.rs / main.rs / dashboard.js 변경 → release 재빌드 의무 (다음 task) |
| 메타 룰 18 (추정 재검증) | 사전 검증 grep 수행 — `paths.base` → `settings_db_path` 정정 (AppState 필드 확인) |
| 메타 룰 19 (단일 진실원 위임) | API 객체 → 백엔드 commands → service 단방향 흐름 유지 |
| 메타 룰 20 (도메인 정렬) | 자동 롤백 미도입 — 사용자 검토 권고만 (RBAC 보류 정책 자기 적용) |

### 회귀 기준선

- workspace cargo check: 경고 0건
- workspace lib 테스트: **381 통과 / 0 실패** (Phase 92 변동 없음 — GUI 변경은 lib 영향 없음)
- workspace clippy `--all --tests`: 0건
- Tauri cargo check: 통과
- **dead_selector_scan.sh: 92 ID 통과** (Phase 92 88 → 92, +4 신규 ID)
- Tauri commands: **65** (G-7 후 61 → +4)

### Release 빌드 (2026-05-22, ✅ 메타 룰 17 의무 이행)

| 바이너리 | 크기 | 시각 | 반영 Phase |
|---------|------|------|-----------|
| `pipeline.exe` (CLI) | **17.85 MB** | 2026-05-22 09:53 | Phase 91+92 (workspace release 1m 58s) |
| `file-pipeline-tauri.exe` (GUI) | **20.98 MB** | 2026-05-22 10:15 | Phase 91+92+93 (Tauri release 21m 41s) |

Phase 90+ 시점(18.69MB / 22.14MB) 대비:
- pipeline.exe **-840KB** (process_file_legacy 389줄 + classify_and_process_with_retry 20줄 삭제 효과, Phase 91)
- file-pipeline-tauri.exe **-1.16MB** (workspace 정리 + Tauri commands 71→65 효과)

### 후속 트리거

- H1 audit_anomaly 호출처 부착 (현재 audit_trace 비어있어 GUI에서 "이상 신호 없음" 표시) — 메타 룰 13 3단계 (호출처 + 실 코퍼스 측정)
- A3 trace_id 호출처 부착 (Phase 91 후속) — 같은 메타 룰 13 진척
- 메타 룰 1 sub-rule 분리 (19건 임계 도달)
- 메타 룰 19 (단일 진실원 위임) 정식 승격 검토

---

## 누적 변경 요약 (2026-05-22) — Phase 92 JAMES 재검증 + Mirage 흡수 (RBAC/외부 협업 제외)

## 누적 변경 요약 (2026-05-22) — Phase 92 JAMES 재검증 + Mirage 흡수 (RBAC/외부 협업 제외)

> Phase 91 직후 사용자 요청으로 JAMES v0.3.0 (변동 없음 재검증) + Mirage v0.0.1 분석. RBAC/외부 협업/외부 연계 보류 정책 유지. 메타 룰 20 정식 승격 (외부 프로젝트 도메인 가정 정렬, 누적 4건 도달) + 메타 룰 21 후보 등록 (본질/부수 도메인 분리).

### H3 MCP 카탈로그 다차원 분류 (Mirage Command 3차원 등록 패턴 흡수)

- `mcp_server.rs::McpToolMetadata` 신규 구조체 — `name / mutates / category / cost`
- `McpToolCategory` enum 7종 (Search / Kg / Settings / Todo / Signal / Snapshot / Lint)
- `McpToolCost` enum 3종 (Free / LlmCall / HeavyCompute)
- `mcp_tool_catalog_full()` 다차원 카탈로그 — 26 도구 분류
- `mcp_tool_mutates_state` + `mcp_tool_catalog`는 호환성 wrapper로 유지
- 추가 테스트 4건 (일치성 검증 / 카테고리 분포 / 비용 / 카테고리 문자열)
- 메타 룰 1 자기 적용 (단일 차원 ↔ 다차원 일치성 자체 테스트)

### H5 원격 저장소 표준화 (Mirage Resource 패턴 흡수)

- `RemoteStoragePort::capabilities() -> ResourceCapabilities` 디폴트 메서드 추가 (호환성 유지)
- `ResourceCapabilities` 구조체 — `backend / can_upload / can_download / can_list / can_delete / mode_options / active_mode / supports_hard_delete`
- 5 어댑터 모두 `capabilities()` 구현: S3 / WebDAV / Network / Null / Notion
- **Notion mode 분기 표준화**: page는 upload 가능, attach는 명시적 불가 (capability에 노출)
- Notion `supports_hard_delete: false` (archived=true PATCH)
- 추가 테스트 2건 (Notion page/attach 모드 capability)
- Mirage VFS / bash 인터페이스는 도메인 불일치로 보류 (메타 룰 20 🔴)

### H1 audit_anomaly 자동 이상 감지 + 사용자 권고 (JAMES 자체 진화 게이트 흡수, RBAC 게이트 보류)

- `crates/shared/src/audit_anomaly.rs` 신규 모듈
- `AnomalyThresholds` (stage_failure_count=5 / recent_window=50)
- `AnomalySignal` (kind / stage / summary / recommendation)
- `AnomalyReport::has_anomaly()` + `analyze_recent_audit` + `analyze_events`
- `SettingsDb::list_recent_audit_events(limit)` 신규 메서드
- **단일 사용자 도메인 정렬**: 자동 롤백이 아닌 사용자 검토 권고만 (JAMES Change Request 인간 게이트 흡수, RBAC 보류)
- 추가 테스트 5건 (clean / 임계 초과 / 미달 / quarantine / 권고 메시지 자기 적용)
- Phase 91 A3 인프라(audit_trace) 자연 확장 — 메타 룰 13 4단계 중 2단계 진척

### 메타 룰 20 META 정식 승격

- 외부 프로젝트 패턴 흡수 시 도메인 가정 정렬 — 누적 4건 (JAMES + TFM + JAMES 재검증 + Mirage)
- 사전 분류 체크리스트 (🟢/🟡/🔴 라벨 + 메타 룰 16 차원 B 결합)

### 메타 룰 21 후보 등록

- 외부 도메인 도구 흡수 시 본질/부수 도메인 분리 — 누적 2건 (TFM + Mirage)
- 메타 룰 20과 차이: 본질 도메인 자체가 다른 경우 (XGBoost / VFS)

### 메타 룰 자기 적용

| 룰 | Phase 92 적용 |
|----|--------------|
| 메타 룰 1 (다중 위치 동기화) | H3 카탈로그 단일↔다차원 일치성 자체 테스트 (15→18건 누적) |
| 메타 룰 13 (4단계) | H1 audit_anomaly = 2단계 (로직 활성화). 호출처 부착 + UI 노출은 후속 |
| 메타 룰 16 차원 B | Mirage 4 영역 라벨 부착 (🟢/🟡/🟡/🔴) |
| 메타 룰 18 (추정 재검증) | "JAMES v0.3.0 큰 변동 있을 것" 추정 → 5일간 변동 없음 (가벼운 사례) |
| 메타 룰 20 (도메인 정렬) | 정식 승격 + 본 Phase 자기 적용 (Mirage VFS/bash 🔴 보류) |

### 회귀 기준선

- workspace cargo check: 경고 0건
- workspace lib 테스트 통과: 104 (adapters) + 169 (core) + 108 (shared) = **381 통과 0 실패** (Phase 91 370 → 381, +11 신규)
- workspace clippy `--all --tests`: **0건** 유지
- Tauri cargo check: 통과
- `dead_selector_scan.sh`: 88 ID 통과

### 신규 파일

- `crates/shared/src/audit_anomaly.rs` (H1)
- `prd/research/external-analysis-2026-05-22.md` (JAMES 재검증 + Mirage 분석 단일 진실원)
- `prd/research/tfm-tabpfn-analysis.md` (TFM 분석 단일 진실원, 메타 룰 19 적용)

### 후속 트리거

- H1 audit_anomaly 호출처 부착 — service.rs 가공 종료 후 주기 호출 (메타 룰 13 3단계)
- H1 GUI Verification 탭에 이상 신호 카드 (메타 룰 13 4단계)
- H3 GUI Settings 카드에 MCP 도구 다차원 분류 표시 (Phase 91 후속 P0 3번 확장)
- H5 GUI Pipeline 외부 저장소 인스펙터에 capability 노출
- 메타 룰 21 후보 누적 1건 추가 시 META 정식 승격

---

## 누적 변경 요약 (2026-05-21) — Phase 91 JAMES v0.3.0 패턴 흡수 (RBAC 제외)

## 누적 변경 요약 (2026-05-21) — Phase 91 JAMES v0.3.0 패턴 흡수 (RBAC 제외)

> JAMES (Hashevolution/James-RAG-Evol, v0.3.0 Platform Skeleton) 분석 후 cognitive middleware §5.7 + 3-stage 보안 파이프라인 패턴 흡수. RBAC / Change Request 인간 게이트 / 5 역할 상한제 / 외부 협업·솔루션·연계는 도메인 가정 불일치로 보류. 검사 통합·출력 마스킹·감사 추적·검증 통합·MCP 메타 5건만 흡수.

### A1' 검사 분산 통일 — `SensitivityDecision` + `check_sensitive_and_pii` 단일 진입점

- `crates/core/src/domain/classifier.rs`: `SensitivityDecision { is_sensitive, reason, pii_hits }` 신규 + `check_sensitive_and_pii(path, content_opt, user_patterns)` 진입점
- `service.rs` `process_file_with_pipeline`의 5분기(`is_sensitive` + `scan_pii_in_text_with` + 본문 PII) → 단일 호출 표면
- `commands.rs::simulate_pipeline`의 별도 OR 분기도 동일 진입점 사용
- `process_file_legacy`(389줄, 호출처 0건 deprecated) + `classify_and_process_with_retry`(20줄, legacy 호출만) 삭제
- 메타 룰 1/14/19 자기 적용 + lesson 14 미연결 함수(`is_sensitive_with_content`) 보존 (호환성)
- 추가 단위 테스트 5건 (path-only / content PII / user pattern / path-precedence / safe)

### A2 출력 단계 PII mask — `mask_pii_in_text` 신규 + 검색 응답 적용

- `classifier.rs::mask_pii_in_text(text, user_patterns) -> String`: `[REDACTED:kind]` 형식
- `config.rs::SearchConfig.output_pii_mask: bool` 신규 (디폴트 true)
- `commands.rs::search` 응답 header에 마스킹 적용
- `mcp_server.rs::handle_search`의 캐시 hit + Sentence Window 응답 2곳에 마스킹 적용
- `McpState.output_pii_mask + pii_user_patterns` 신규 필드 (3곳 생성처 모두 갱신 — `modals/cli/src/main.rs` / `crates/shared/src/cli.rs` / 테스트 helper)
- JAMES 3-stage 보안 파이프라인 중 **output post_filter** 1단계만 file-pipeline에 자연 적용
- 추가 단위 테스트 4건 (email mask / multiple PII / user pattern / no-op clean)

### A3 trace_id 단일 키 + `audit_trace` 테이블

- `crates/shared/src/settings_db.rs::SETTINGS_DB_SCHEMA`에 `audit_trace` 테이블 신규 (id / trace_id / stage / inputs_hash / output_summary / applied_rule / created_at + 2 인덱스)
- 단일 상수 패턴 유지로 lesson 26 회피 (`open()` + `open_in_memory()` 양쪽 자동 적용)
- `crates/core/src/audit.rs` 신규 모듈: `TraceId` + `input_hash_prefix` + `truncate_output_summary`
- `SettingsDb::record_audit_event` + `list_audit_by_trace` + `AuditEventRow` 구조체
- `spec/benchmarks/scripts/replay_trace.sh` 신규 (G-5 6번째 스크립트)
- 메타 룰 18 "추정 재검증 의무" 인프라 — lesson 46 G-1 같은 추정 root cause 확정 가능
- 신규 호출처 부착은 별도 phase로 분리 — 본 phase는 **인프라 추가** 1단계만 (메타 룰 13 4단계 중)

### B1 Verifier 통합 진입점

- `crates/core/src/reasoning/mod.rs` + `reasoning/verifier.rs` 신규
- 흩어진 검증 함수(`verify_with_thresholds` / `detect_strong_claims` / `Linter::lint_strong_claims`)를 `Verifier::verify_processed` + `detect_strong_claims` wrapper로 묶음
- 기존 함수 보존(호환성). 신규 호출처는 본 진입점 권장
- 메타 룰 14 "다중 진입점 분기 트리 통일" 자기 적용

### B2 MCP `mutates_state` 메타데이터 (RBAC 없이 표시만)

- `crates/shared/src/mcp_server.rs::mcp_tool_mutates_state(name)` + `mcp_tool_catalog()` 신규
- 24 MCP 도구 분류: 18 read-only + 6 mutating (`complete_todo` / `revise_topic` / `setup_apply` / `setup_apply_modules` / `setup_snapshot_rollback` / `setup_snapshot_measure`)
- 호출 게이트 도입 없음 (단일 사용자 — RBAC 가정 안 함). 외부 도구 카탈로그 노출용
- 추가 테스트 4건 (read-only / writers / catalog 일치성 / 4+건 등재 검증)

### 메타 룰 자기 적용 누적

| 룰 | Phase 91 적용 |
|----|--------------|
| 메타 룰 1 (다중 위치 동기화) | A1'/B1/B2 — 15건 → 17건 누적 |
| 메타 룰 14 (다중 진입점 통일) | A1' service.rs + simulate_pipeline 통일 + Verifier wrapper |
| 메타 룰 18 (추정 재검증) | A3 trace_id 인프라 (재검증 도구) + Phase 91 진입 직후 service.rs:235 추정 빗나감 3/3 → lesson 50 |
| 메타 룰 19 (단일 진실원 위임) | classifier.rs 단일 진입점 + Verifier wrapper |
| 메타 룰 9 (빌드 진단 자원 먼저) | 본 phase에서 `cargo build --tests -j 2` 적용 (os error 1455 회피) |

### 회귀 기준선

- workspace cargo check: 경고 0건 (Phase 89 → 0건 유지)
- workspace lib 테스트 통과: 102 (adapters) + 169 (core) + 99 (shared) = **370 통과 0 실패** (Phase 89 349 → 370, +21건)
- workspace clippy `--all --tests`: **0건** 유지
- Tauri cargo check: 통과 (기존 dirty 경고 2건 유지, Phase 91 신규 0건)
- `dead_selector_scan.sh`: 88 ID 검증 통과
- `process_file_legacy` 삭제로 service.rs **2034 → 약 1620줄** (-414줄, -20.4%) — **2026-06-16 lesson 77 후속**: hex-arch-d step-s4 use case 분해로 추가 **1681 → 615줄** (-1066줄, -63%). 3 use case (ProcessFileUseCase + CrossRefUseCase + MaintenanceUseCase) + 헬퍼 잔류. 본 stale 수치 (1620줄) 의 후속 진척은 §누적 변경 요약 (2026-06-16) 참조

### 신규 파일

- `crates/core/src/audit.rs` (A3)
- `crates/core/src/reasoning/mod.rs` + `reasoning/verifier.rs` (B1)
- `spec/benchmarks/scripts/replay_trace.sh` (A3)

### 후속 트리거

- A3 trace_id 호출처 신규 부착 (메타 룰 13 4단계 중 2단계 "로직 활성화") — LLM/검색/MCP 호출 부착은 다음 phase
- B2 GUI Settings 카드 표시 (`mutates_state` 분류 UI 노출) — 다음 phase
- A2 PII mask 사용자 토글 GUI — 다음 phase
- 메타 룰 1의 17건 누적 → sub-rule 분리 검토 (E4 후보 임계 도달)

---

## 누적 변경 요약 (2026-05-19) — Phase 90+ GUI 전수 검증

> Phase 90 Notion 추가 직후 사용자 요청으로 GUI 전수 검증 진행. 80개 액션 카탈로그 + Playwright HTTP 자동 검증 + Tauri 실행. 결과는 lesson 46에 정리, 사이드 발견 5건 활성 트리거에 추가.

### GUI 액션 카탈로그 (80개)

- 헤더 5 / 탭 전환 7 / Documents 5 / Processing 4 / Todos 3 / Verification 1 (Phase 89 N-4) / Topics 2 / Pipeline 15 / Settings 28 / 모달 10
- 전체 식별 후 `spec/lesson-learned/INDEX.md` + `spec/architecture.md` 수치 갱신 대상

### Playwright HTTP 모드 자동 검증 (20/20 통과)

- 7탭 전환 정상, 동적 액션 23 → 86 증가 (탭 진입에 따라)
- 헤더 토글 / AI 설정 도우미 모달 / Documents 검색 input / Processing 카드 / Todos 모달 / Verification 강한 주장 카드 (Phase 89 N-4) / Topics / Pipeline 3컬럼 + 미드탭 3 / Settings 카드 4종 모두 DOM 렌더 통과
- **invoke 미동작 영역** (lesson 55/59 재확인): Settings nav items / Pipeline batch sections / Pipeline 서브탭 (외부 저장소 Notion 옵션) / 검색 결과 / 모듈 12 체크박스
- Notion 옵션은 dashboard.js:3782에 코드 존재 검증 (grep), Tauri WebView에서만 렌더

### 데이터 입력 6종 흐름 grep 검증

frontend → invoke → commands.rs → service → DB 연쇄 모두 통과:
- 검색 (Documents) / Todo 추가 / PII 패턴 추가 + live reload (Phase 84) / C1 임계값 / Hook CRUD (재시작 필요) / 크레덴셜 추가 + restore_masked_secrets (lesson 12)

### 사이드 발견 (활성 트리거 추가)

1. **Pipeline 가공 노드 21 → 23** / **검색 노드 18 → 20** (spec 갱신 완료)
2. **Claude CLI exit code 1 산발 실패**: 9건 가공 시도 중 5건 실패 (단일 파일 719 chars). ✅ **2026-05-20 G-1 진단 종결** — 격리 환경 재현 (max_workers=1 단독 1/1, max_workers=4 9건 동시 9/9 모두 성공) → 외부 일시적 요인으로 결론. 구조적 약점 5건은 F-1~F-5로 해소 (module-llm/claude_cli.rs: timeout 300s / stderr 200자 + elapsed 포함 / 빈 stderr+exit1 시 어댑터 내부 1회 자동 재시도 / stdin flush+drop / service.rs LLM 호출 실패 시 quarantine 라우팅). lesson 46 본문 갱신
3. **fastembed feature OFF release 빌드** WARN: `default_model='fastembed' 폴백`. Claude CLI 임베딩(128축)으로 폴백 동작
4. **Tauri 빌드 시점 ≠ Phase 변경 시점**: Tauri release 17:29 빌드 → Phase 90(Notion build_service 분기) 미반영. UI dashboard.js는 정적이라 ui/만 보면 표시되지만 invoke 호출은 Phase 89까지 한정
5. **Pipeline 서브탭 미렌더**: 2026-05-20 G-4 재진단 결과 — `pb-subtabs`는 invoke 의존이 아니라 **HTML 엘리먼트 자체 부재(dead-code, 옛 IA 잔재)**. 진짜 invoke-no-fallback 트리거는 (b) 단계 재진단으로 **Verification 카드 한 곳**으로 좁혀짐 (Settings/Documents/Processing/Todos/Topics는 모두 정상 placeholder 작동 중. 원래 검증의 셀렉터 오류로 오인). browser-automation MCP v0.2 (10종 도구) 전환 후 검증. ✅ **G-4 (a) dead-code 정리 완료** (2026-05-20): dashboard.js 6 함수 271줄 + dashboard.css 5 rule 삭제 + lesson 47 + 메타 룰 1 14번째 사례. ✅ **G-4 (b) invoke-no-fallback 종결** (2026-05-20): `renderVerificationMetrics` 빈 객체 가드 강화 (`!m || typeof m.total !== 'number'`). "TOTAL undefined" → "검증 메트릭이 아직 없습니다..." 표시

### G-5 GUI 회귀 자동화 (2026-05-20, ✅ 종결)

`spec/benchmarks/scripts/` 5종 정형화 + META.md "Phase 종결 시 GUI 회귀 자동 검증" 체크리스트 추가:

| 스크립트 | baseline | 게이트 |
|----------|---------|--------|
| `action_catalog.sh --count` | **68** (G-6 후, 이전 75) | `--diff 68`로 회귀 감지 |
| `dead_selector_scan.sh` | **0건** PASS (G-6 정리 + whitelist 강화 후) | DEAD 0건이면 PASS |
| `empty_state_audit.sh` | 0건 | 후보 출력만 (게이트 아님) |
| `data_flow_trace.sh <action>` | 6단계 추적 도구 | 디버깅용 (게이트 아님) |
| `gui_http_smoke.sh` | **5/5 PASS** (dashboard.js **4234** lines) | 1+ fail이면 exit 1 |

### G-7 Tauri commands 9건 백엔드 정리 (2026-05-20, ✅ 종결)

G-6 frontend 정리 후속. backend Tauri commands frontend 호출처 0건 → 함수 본체 + invoke_handler 등록 일괄 삭제:
- **삭제 함수 10개**: `search_with_trace` / `purge_dry_run` / `purge_execute` / `list_doc_types` / `save_doc_type` / `delete_doc_type` / `refresh_host_tools` / `test_preprocess` / `mcp_tools_list` / `mcp_tool_set_enabled`
- `modals/app/src/commands.rs`: 1956 → **1590 라인** (-366줄, -18.7%)
- `modals/app/src/main.rs` invoke_handler 10건 제거
- workspace + Tauri `cargo check` ✅ 통과
- 사이드 발견: `ListParams` / `mask_secrets` / `restore_masked_secrets` 정의가 dirty working tree에 누락 → 본 작업에서 정의 추가 (lesson 12 패턴 응용, save_config secret 복원)

### 메타 룰 17/18 정식 승격 (2026-05-20, ✅)

- **메타 룰 17**: "코드 변경 phase의 release 빌드 시점 의무화" (Phase 90 + G-3 사례). 변경 분류 표 + 자동화 후보 (`git diff` 기반)
- **메타 룰 18**: "lesson 본문의 추정 사항은 다음 phase에서 재검증 의무" (G-1/G-4 추정 빗나감 2/2 = 100% 후 정식화). 메타 룰 12 확장

### Git pre-push hook + #8 AST 정밀화 (2026-05-20, ✅)

- `.git/hooks/pre-push` 등록: dead_selector_scan + gui_http_smoke 자동 실행. 우회 `PIPELINE_SKIP_GUI_GATE=1 git push`
- `dead_selector_scan_v2.js` (Node + acorn AST 기반): 템플릿 보간 동적 ID 정확 처리 / innerHTML 안의 정적 id="..." 정확 추출 / createElement + .id 할당 인식. 87개 정적 ID 모두 PASS

### scenarios.md / webapp-design.md 현행화 (2026-05-20, ✅)

- `scenarios.md`: 시나리오 5 Notion 옵션 + 시나리오 6 설정 도우미 3분기 (Phase 80) + 시나리오 7 PII live reload (Phase 84) 신규
- `webapp-design.md`: Phase 86~90 + G-1~G-7 변동 이력 추가. Phase 56 자문 컨텍스트 보존, 현행 IA 표 갱신

### G-6 dead 13건 일괄 정리 (2026-05-20, ✅ 종결)

G-5 첫 실행 사이드 발견의 후속. dashboard.js dead 영역 일괄 정리:
- **10 함수 삭제**: `_renderSearchSimulation` / `_renderMcpTools` / `_renderSystemCredentials` / `_runSearchSim` / `_renderMigration` / `_renderHostToolsStatus` / `_refreshHostTools` / `_loadDocTypes` / `_renderDocTypesTable` / `_openDocTypeModal`
- **handlePBAction switch 7 case 삭제**: pb-purge-dry-run / pb-purge-execute / pb-preprocess-test / pb-add-doctype / pb-edit-doctype / pb-delete-doctype / pb-doctype-page
- **별도 5 if action 삭제**: refresh-host-tools / mcp-disable / mcp-enable / run-search-sim / test-preprocess
- **11 API 정의 삭제**: listDocTypes / saveDocType / deleteDocType / testPreprocess / refreshHostTools / purgeDryRun / purgeExecute / mcpToolsList / mcpToolSetEnabled / searchWithTrace + 기타
- **dead_selector_scan.sh whitelist 강화**: createElement + .id 패턴 (`settings-no-results` 같은 동적 fallback) 자동 제외
- **dashboard.js**: 4645 → **4234** (-411줄, -8.8%). 누적 G-4 (a) + G-6 = **4915 → 4234 (-681, -13.9%)**
- **백엔드 후속 트리거**: Tauri commands 9건 (refresh_host_tools / mcp_tools_list / purge_dry_run / test_preprocess / list_doc_types 등) frontend 호출처 0건. 별도 phase에서 백엔드 정리 검토

lesson 46의 "잘한 것 3건"(80 액션 카탈로그 grep + Playwright HTTP 모드 + 데이터 흐름 6종 grep) + G-4 (a)/(b)의 lesson 47/G-4 (b) 메타 패턴이 통합 정형화됨.

### CLI 가공 검증 (GUI 흐름 사전 조건)

- `D:/file-test/bench_gui_phase90` 격리 환경, 9건 가공 시도
- 4건 성공 (reference 3 / guide 1 / study 3), 5건 실패 (Claude CLI exit 1)
- per-doc 120.2s (Phase 89 측정 48.1s 대비 +2.5x, fastembed OFF + 실패 재시도 영향)

### Release 빌드 시각

| 바이너리 | 크기 | 시각 | 반영 Phase |
|---------|------|------|-----------|
| `pipeline.exe` (CLI) | 18.69 MB | 2026-05-20 11:16 | Phase 90 + F-1~F-5 (G-1 진단 후속) |
| `file-pipeline-tauri.exe` (GUI) | 22.14 MB | 2026-05-20 11:24 | Phase 90 + F-1~F-5 (Notion + G-1/G-3 해소) |

→ Tauri 재빌드 7m 19s (이전 26m+ 추정 대비 작음 — 증분 빌드). Notion build_service 분기 + F-5 quarantine 라우팅 모두 GUI에 반영. G-3 트리거 해소.

### 회귀 기준선 (변동 없음)

- workspace lib **349** 유지
- workspace clippy `--all --tests` **0건**
- workspace + Tauri `cargo check` ✅

### 주요 lesson (예정)

- lesson 46 — GUI 검증 한계 + Claude CLI 산발 실패 + Tauri 빌드 시점 메타 룰 후보

---

## 누적 변경 요약 (2026-05-19) — Phase 90 Notion 원격 저장소

> Phase 89 외부 신호 대기 단계의 첫 사용자 요청 — Notion을 원격 저장소 백엔드로 추가. 일반 파일 시스템과 다른 페이지/블록 기반 플랫폼이라 mode별 분기로 통합.

### 핵심 변경

| 영역 | 변경 |
|------|------|
| 어댑터 신규 | `crates/adapters/.../storage/notion_storage.rs` (380줄). `RemoteStoragePort` 4 메서드 구현 + reqwest 직접 호출 (module-storage 외부 — Notion 도메인 특수성) |
| Config 4 필드 | `RemoteStorageConfig.notion_token / notion_parent_page_id / notion_mode (page|attach) / notion_database_id` |
| build_service 분기 | `provider="notion"` 분기 추가. token+parent 모두 있어야 활성, 누락 시 Null로 폴백 |
| UI 인스펙터 | Pipeline 외부 저장소 서브탭에 notion 옵션 + 4필드 + mode select + 안내 |
| 단위 테스트 | 6건 (key_to_title / text_to_blocks 단락 분할 / 2000자 분할 / mode 파싱 / attach upload 에러 / is_configured) |

### Notion API 통합 결정

- **page 모드 (디폴트)**: 가공본 텍스트 → 자식 페이지 (paragraph 블록 배열). 100블록 / 2000자 제한 자동 분할
- **attach 모드**: `anyhow::bail!`로 명시적 미지원 — Notion 공식 API가 zst 직접 업로드 불가 (file_upload v2024 별도 + 복잡도 큼). UI 안내에 S3/WebDAV 권장 메시지
- **인증**: Internal Integration token + 대상 페이지 Connect to integration 필요
- **download**: 자식 페이지의 paragraph 블록을 합쳐 텍스트 복원
- **list**: 부모 페이지의 child_page 제목 목록
- **delete**: archived=true PATCH (Notion은 hard delete 미지원)

### 헥사고날 경계 결정

기존 어댑터(S3/WebDAV/Network)는 `module-storage`의 raw 어댑터를 thin wrap. Notion은 도메인 특수적(페이지/블록/database)이라 form-agnostic한 `module-storage`에 포함 부적합. **file-pipeline-adapters에 직접 reqwest 구현**. 형제 프로젝트에서 Notion 필요 시 별도 모듈(`module-notion-api`) 추출 검토.

### 회귀 기준선

- workspace lib **349** 통과 (Phase 89 343 + Notion 단위 테스트 6건)
- workspace clippy `--all --tests` **0건** 유지 (redundant_closure / consecutive_str_replace 2건 수정 후)
- workspace + Tauri `cargo check` ✅
- Tauri commands 71 유지 (변동 없음) → **G-7 후 61** (10건 삭제, 2026-05-20)
- 신규 외부 의존: reqwest (기존 의존, 변경 없음). Notion API 직접 호출
- **Release 빌드 검증 (2026-05-19)**: `pipeline.exe` **18.67 MB** (Phase 89 18.5MB + 175KB, Notion 어댑터). workspace release 2m 08s. Tauri 미재빌드 (UI dashboard.js만 변경)

### 사용 절차

1. notion.so/my-integrations → New integration (Internal) → token 복사
2. 대상 부모 페이지 우측 ⋯ → Connect to integration → 본 integration 선택
3. Pipeline 외부 저장소 서브탭에서 provider=notion 선택, token + parent_page_id 입력
4. mode=page 권장 (가공본 텍스트가 자식 페이지로 자동 생성)

### 후속

- `attach` 모드 진짜 구현 (Notion file_upload v2024 API): 트리거 대기
- rate limit (3 req/s) 위반 시 자동 backoff: 트리거 대기
- 형제 프로젝트 활용 시 `module-notion-api` 분리

---


## 누적 변경 요약 (2026-05-18) — Phase 89 C+D+B 영역 (위생 + 메타 룰 + #10 인프라)

> 권장 우선순위 1~3단계 완료 후 위생 작업 일괄. 측정 중 발견된 사이드 이슈 3건 해소 + 메타 룰 13~15 승격 + #10 Sparse 인프라 토대.

### C 영역 — 위생 (측정 사이드 발견 해소)

**C-1: `--base` CLI 옵션이 LocalVectorStore까지 전파**
- `crates/shared/src/lib.rs::build_service` — `LocalVectorStore::new()` → `LocalVectorStore::with_path(paths.base.join(".local-store.json"))`
- `crates/shared/src/cli.rs::Stats` 분기도 동일 적용
- 환경변수 PIPELINE_BASE만 보던 `resolve_data_base()` 분기를 우회 — A1-hit 측정에서 발견된 회귀 차단

**C-2: host_tools_cache fallback 매번 발생 해소**
- `crates/adapters/.../preprocessor.rs::preprocess_with_config` — `CompositePreprocessor::new(pdf_tool, ocr_tool)` (재감지 spawn) → `with_tools(pdf_tool, ocr_tool, self.host_tools.clone())` (캐시 재사용)
- "호스트 전처리 도구 감지(fallback, 비캐시)" 매 가공마다 출력되던 문제 해소

**C-3: doc_types.toml 없음 WARN → settings.db 폴백**
- `crates/shared/src/config.rs::load_doc_type_registry` — 파일 미존재 시 `find_data_dir(None)` → `settings.db.to_doc_type_registry()` 폴백
- WARN → INFO/DEBUG로 격하. 17 기본 유형이 settings.db에 자동 마이그레이션되므로 파일은 옵션 (외부 편집 진입점)

### D 영역 — META.md 메타 룰 13~15 승격

| 메타 룰 | 출처 | 핵심 |
|---------|------|------|
| 13: 인프라 활성화 4단계 | lesson 43 | 인프라 / 로직 / 측정 / **UI 노출** 4단계. 4단계 누락 시 "동작 여부 모름" |
| 14: 다중 진입점 분기 트리 통일 | lesson 38 + Phase 89 C-1/C-2/C-3 | `find_data_dir` / `CompositePreprocessor` / `doc_types` 3건 추가 사례 |
| 15: 측정 환경 격리 + 증분 상태 일괄 삭제 | lesson 43 사이드 발견 / Phase 89 A1-hit | SHA-256 + `.compile-state.json` + `.work-queue.json` 차단 |

### B-1 영역 — 트리거 #2/#4 5변형 재측정 (485 파일)

`spec/benchmarks/b1_variants_phase89_20260518.json` (cargo test --release, 306s):

| 변형 | 시간 vs baseline | 관계 vs baseline | 결정 |
|------|----------------|-----------------|------|
| baseline (0.7) | — | 11166 | 기준 |
| threshold 0.8 (적용됨) | -19.3% | -74.5% | ✅ 이미 디폴트 |
| MinHash force (#2) | -3.3% | -19.9% | ❌ 보류 — threshold 0.8 대비 부차적 |
| Metadata blocking (#4) | +3.2% | -0.6% | ❌ 보류 — 무효과 |
| all | +2.2% | -79.9% | 과적용 |

Phase 86 동일 결과 재현. 측정 재현성 검증. 디폴트 변경 보류 재확인.

A2-def / B1-def는 자동 측정 불가 (검색 만족도 = 실 사용자 신호 필요) — 보류 유지.

### B-2 영역 — #10 BGE-M3 Sparse LocalVectorStore 인프라 (트리거 대기)

> 완전 통합은 별도 phase 분리 (fastembed feature 게이트 진입 비용). 본 phase는 **도메인/포트 토대**만 추가.

**core 도메인**:
- `SparseEmbedding { indices: Vec<u32>, values: Vec<f32> }` 신규 (Serialize/Deserialize, dot product 메서드)
- fastembed::SparseEmbedding과 호환 형식. 어댑터에서 변환 (헥사고날 유지)

**core 포트**:
- `EmbeddingPort::embed_sparse(text) -> Result<SparseEmbedding>` 디폴트 미지원 (bail!)
- `EmbeddingPort::supports_sparse() -> bool` 디폴트 false
- `VectorDBPort::upsert_sparse_embedding(doc_id, sparse) -> Result<()>` 디폴트 no-op
- `VectorDBPort::search_sparse(sparse, top_k) -> Result<Vec<SimilarDoc>>` 디폴트 빈 결과
- `VectorDBPort::sparse_enabled() -> bool` 디폴트 false

**다음 단계** (트리거 #10 도달 시):
- `FastEmbedSparseAdapter`에 `EmbeddingPort` impl 추가 (현재는 자체 메서드만)
- `LocalVectorStore`에 `sparse_index: Mutex<HashMap<String, SparseEmbedding>>` 필드 + override
- service.rs의 embedding 단계에서 sparse도 계산 + upsert
- search_hybrid에 dense + sparse RRF 결합

### 회귀 기준선

- workspace lib **343 유지** (96 + 152 + 95)
- workspace clippy `--all --tests` **0건** 유지
- workspace + Tauri `cargo check` ✅
- Tauri commands **71 유지** (변동 없음)
- 신규 도메인 타입: `SparseEmbedding`
- 신규 포트 디폴트 메서드: 5건 (EmbeddingPort 2 + VectorDBPort 3) — 모두 no-op/bail
- **Release 빌드 검증 (2026-05-19)**: `pipeline.exe` 18.5MB / `file-pipeline-tauri.exe` 22.0MB. Tauri 빌드 26m 23s (fastembed feature 비활성)

### 주요 lesson

- META.md 메타 룰 13~15 승격 (이전 11~12 + 신규 3)
- lesson 43 본문에 A1 hit 측정 사이드 발견 추가
- C-1/C-2/C-3은 측정에서 발견된 사이드 이슈 — 측정이 위생 작업의 발견 메커니즘 역할 (lesson 후보)



## 누적 변경 요약 (2026-05-18) — Phase 89 N-3+N-4 (lint 다층 주기 + Metadata UI 노출)

> Phase 87 인프라(detect_strong_claims + lint_weekly/monthly) 호출처 0건 해소 + Metadata.needs_verification/open_questions의 사용자 가시화. 권장 우선순위 1단계 묶음(N-3+N-4 동시 진행).

### N-3 lint 다층 주기 schedule task 연결

3진입점 모두 weekly + monthly 분기 추가:

| 파일 | 함수 | 변경 |
|------|------|------|
| `modals/app/src/service.rs` | `start_background_tasks_standalone` | weekly + monthly 신규 (활성 진입점) |
| `modals/app/src/service.rs` | `start_background_tasks` (dead_code) | weekly + monthly 신규 (일관성) |
| `modals/cli/src/main.rs` | `pipeline start` 핸들러 | weekly + monthly 신규 |

매핑 (wikidocs 353407 카테고리 단위):
- `lint_interval_hours` 6h → `Linter::lint` (orphan/missing backlink)
- `lint_weekly_hours` 168h → `Linter::lint_strong_claims(vector_db, storage, 5)` (품질 의심)
- `lint_monthly_hours` 720h → `Linter::lint_topics(&topics_dir)` (정합성)

모두 0=비활성 토글 가능. 메타 룰 5강화 3요소(config + 분기 + no-op) 충족.

### N-4 Metadata 보조 필드 UI 노출 (5~7계층 동기화)

신규 메서드 + 7계층 갱신:

| 계층 | 파일 | 변경 |
|------|------|------|
| 포트 trait | `core/ports/output.rs::VectorDBPort` | `fn get_metadata(doc_id) -> Result<Option<Metadata>>` 디폴트 None |
| 어댑터 override | `adapters/.../local_store.rs` | StoredDoc → Metadata 매핑 (doc_types/date/summary/keywords/needs_verification/open_questions) |
| Tauri command 확장 | `modals/app/src/commands.rs::get_document` | needs_verification / open_questions / summary / keywords 응답 추가 |
| Tauri command 신규 | `modals/app/src/commands.rs::get_lint_strong_claims` | 즉시 lint_strong_claims 실행 + 결과 반환 (max_per_doc 5) |
| invoke_handler | `modals/app/src/main.rs` | get_lint_strong_claims 등록 |
| Frontend HTML | `ui/index.html` | detail-aux div + Verification 탭 강한 주장 카드 |
| Frontend JS | `ui/dashboard.js` | renderDocDetail 확장 + runLintStrongClaims + _escape 헬퍼 + click 위임 분기 + API.lintStrongClaims |

### 회귀 기준선

- workspace lib **343 유지** (96 core + 152 adapters + 95 shared)
- workspace clippy `--all --tests` **0건** 유지
- workspace + Tauri `cargo check` ✅
- 통합 테스트 빌드 ✅ (lesson 27 회귀 차단)
- Tauri commands **70 → 71** (get_lint_strong_claims +1)
- MCP tools 32 변동 없음 (※ 당시 수치 — MCP 전체 폐기 2026-06-17로 현 시점 0)
- settings.db 테이블 변동 없음

### 주요 lesson

- lesson 43 신규 — 인프라 활성화 3단계 → **4단계 (UI 노출)** / 메타 룰 1 사례 확장 (4계층 직렬화 → 7계층 동기화 체크리스트) / 포트 메서드는 `Option<T>` 디폴트 None으로 추가 / wikidocs 353407 매핑 카테고리 자유도

### Phase 88 잔여 해소 진행도

| 항목 | 상태 |
|------|------|
| N-3 lint 다층 주기 → service.rs 분기 | ✅ **Phase 89 완성** (standalone + dead + CLI 3곳) |
| N-4 보조 필드 + lint_strong_claims UI 노출 | ✅ **Phase 89 완성** |
| C2-fp 100건+ 측정 | ✅ **Phase 89에서 측정** (36 docs / 0 격리 / FP 0%) |
| A1-hit 측정 (동일 파일 재가공) | ✅ **Phase 89에서 측정** (9 entries / 9 hits / per-doc 48.1→24.9s = 1.93x) |

### 권장 우선순위 2단계 측정 결과 (2026-05-18)

**A1 LLM 캐시 hit률** (`spec/benchmarks/a1_hit_phase89_20260518.json`):
- 1차 가공 (miss): 9건 / 433.2s / per-doc **48.1s** / cache `9 entries / 0 hits`
- 2차 가공 (hit): 9건 / 199.2s / per-doc **24.9s** / cache `9 entries / 9 hits, avg 1.00`
- **1.93x 가속**. LLM 호출이 전체 시간 약 50% 차지
- **사이드 발견**: SHA-256 + .compile-state.json + .work-queue.json이 LLM 호출 이전에 동일 파일을 스킵 → 단순 inbox 재투입으로는 A1 캐시까지 도달 못함. 측정에는 증분 상태 파일 일괄 삭제가 필요. 운영 hit 시나리오는 extra_inboxes / 색인 rebuild / >40KB 청크 재등장 등

**C2 PII false positive** (`spec/benchmarks/c2_fp_phase89_20260518.json`):
- 코퍼스: Java/JSP/CSS/DB/Web 기술문서 36 docs 가공
- 격리 (sensitive): **0건** / quarantine: 0건 / **FP 0%**
- Phase 88 측정 (10 docs 2 격리, FP ~20%) 대비 큰 격차 — DB 도구 문서의 예제 코드 우연 매칭으로 추정
- 결정: PII regex 5종 (ssn_kr / credit_card / email / phone_kr / biz_reg_kr) 디폴트 유지. 도메인 확장 시 settings.db `pii_patterns_user` 등록

### 3단계 트리거 #6 HyDE — LLM 어댑터 활성 (2026-05-18)

Phase 86 인프라(`search.hyde_enabled` configField + `LLMPort.generate_hypothetical` 디폴트 no-op + handle_search 분기) 활성을 위한 어댑터 작업:

- LLM 어댑터 5종 (claude_cli / anthropic / openai / ollama / gemini) 모두 `generate_hypothetical(query) -> 가상 답변` override 추가. `raw.call_text("", &prompt, 512)` 패턴
- LLM wrapper 3종 (chunked_agent / fallback / cached_llm) 모두 inner 위임 override 추가
- `crates/adapters/.../llm/prompts.rs`: `NAME_HYDE` / `DEFAULT_HYDE` 상수 + `SECTIONS` 등록 + `build_hyde_prompt(query)` 빌더
- `src/prompts.toml`: `[hyde]` 섹션 + template 추가 (한국어 프롬프트, 2~4문장 가상 답변 지시)
- **디폴트 비활성 유지** (`search.hyde_enabled = false`). 트리거 #6 도달 시 디폴트 1줄 변경으로 즉시 활성

lesson 30 패턴 — 인프라 추가 + 디폴트 비활성. 실 사용자 "검색 안 됨" 피드백 도달 시 활성화.

---

## 누적 변경 요약 (2026-05-18) — Phase 88 완성 (LLM 보조 필드 + fastembed 측정)

> Phase 87 인프라(Metadata.needs_verification/open_questions)의 활성화 완성. 신규 프롬프트 + 어댑터 파싱 + StoredDoc 직렬화 + fastembed 활성 환경에서 실 코퍼스 측정 검증.

### N-2 prompts.toml classify 갱신

- `src/prompts.toml::[classify]`의 JSON 스키마 + 규칙에 `needs_verification` / `open_questions` 추가
- `crates/adapters/src/driven/llm/prompts.rs` fallback 프롬프트 동기화

### 어댑터 파싱 + 영속화 4계층 동기화

| 계층 | 파일 | 변경 |
|------|------|------|
| 도메인 모델 | `core/domain/models.rs::Metadata` | Phase 87 |
| LLM 응답 | `adapters/llm/response.rs::LlmResponse` | 두 필드 + `build_classify_result` 주입 |
| 저장 모델 | `adapters/vector_db/local_store.rs::StoredDoc` | 두 필드 + upsert 매핑 (신규/업데이트) |
| 영속 | `.local-store.json` | 자동 직렬화 |

### fastembed feature 활성 측정 (D:/file-test/samples 10건)

`spec/benchmarks/llm_smoke_10_v3_20260518.json` 참조:

| 측정 | 환경 | per-doc | needs_verification | open_questions |
|------|------|---------|-------------------|----------------|
| v1 (Phase 88 부분) | fastembed 비활성 | 49.1s | 0 (LLM 미작성) | 0 |
| v2 (중간, cold) | fastembed 활성 cold | 68.7s | 측정 외 | 측정 외 |
| **v3 (Phase 88 완성, warm)** | **fastembed 활성 warm** | **44.9s** | **19건 (1.9/doc)** | **22건 (2.2/doc)** |

- 검증 통과율: **100%** (8/8)
- PII 격리: 2건
- LLM 보조 필드 품질 매우 높음 (오타 검증 / 보안 정책 / 호환성 / 모호 수치)

### Phase 87 인프라 활성화 진행도 (Phase 88 완성)

| 인프라 | 상태 |
|--------|------|
| Metadata.needs_verification/open_questions | ✅ **Phase 88 완성** — LLM 가공 + 직렬화 + 영속화 완료 |
| detect_strong_claims | ✅ Phase 88 부분 — lint 통합 완료 (schedule task 호출은 N-3 후속) |
| lint_weekly_hours/lint_monthly_hours | 필드만 (schedule task 분기 미연결 — N-3 후속) |

### 회귀 기준선

- workspace lib **343** 유지 (StoredDoc 필드 추가만, 신규 테스트 없음)
- workspace clippy `--all --tests` **0건**
- workspace + Tauri `cargo check` ✅
- fastembed feature 빌드: 11m 26s (첫) / 2m 04s (incremental)

### 주요 lesson

- lesson 42 신규 — 4계층 직렬화 동기화 (lesson 1 메타 룰 1 실증). fastembed cold/warm 2회 측정 필요. lesson 14 인프라/활성화/측정 3단계 첫 완성

### Phase 88 잔여 → Phase 89

- N-3: lint 다층 주기 → service.rs schedule task 분기 연결 — **✅ Phase 89에서 종결**
- N-4: needs_verification/open_questions + lint_strong_claims UI 노출 — **✅ Phase 89에서 종결**
- C2-fp 100건+ 측정 (Q1 보류분) — 권장 우선순위 2단계 대기
- A1-hit 측정 (동일 파일 재가공) — 권장 우선순위 2단계 대기

---

## 누적 변경 요약 (2026-05-15) — Phase 88 부분 (lint 통합)

> Phase 87 인프라 호출처 0건 해소의 1단계. W-1(외부 분석 정형화) + N-1(lint 통합)만 진행. N-2/N-3은 후속.

### W-1 외부 문서 분석 단일 진실원

- 신규 `prd/research/external-analysis-2026-05-15.md` — supertonic / wikidocs 352523 / 353407 분석 결과 + 결정 매핑표
- 다음 외부 분석 시 본 문서 인용 → 결정 반복 차단

### N-1 detect_strong_claims lint 흐름 통합

- 신규 `Linter::lint_strong_claims(vector_db, storage, max_per_doc)` — 가공본을 storage에서 복원 → `detect_strong_claims` 호출 → `LintIssueType::StrongClaim` 생성
- `LintIssueType::StrongClaim` enum 변형 신규
- 단위 테스트 3건: 감지 / 빈 본문 스킵 / max_per_doc 상한
- **호출처**: 단위 테스트만. 실 서비스 schedule task에서 호출은 N-3 후속

### 회귀 기준선

- workspace lib **343** 통과 (Phase 87 340 + 3 신규)
- workspace clippy `--all --tests` **0건** 유지
- workspace + Tauri `cargo check` ✅

### 주요 lesson

- lesson 41 신규 — 외부 문서 분석 단일 진실원 의무 (메타 룰 후보) / 검사 함수 호출 비용으로 메서드 분리 / max_per_doc 상한 / 호출처 부분 해소

### Phase 87 인프라 활성화 진행도

| 인프라 | 상태 |
|--------|------|
| Metadata.needs_verification/open_questions | 필드만 (LLM 프롬프트 미연결 — N-2 후속) |
| detect_strong_claims | **lint 통합 완료 (본 phase)** — 단, schedule task 호출 미연결 (N-3 후속) |
| lint_weekly_hours/lint_monthly_hours | 필드만 (schedule task 분기 미연결 — N-3 후속) |

### 실 LLM 스모크 측정 (2026-05-18, D:/file-test/samples 10건)

`spec/benchmarks/llm_smoke_10_20260518.json` 참조. 핵심:

- **처리**: 10건 중 8건 가공 성공, 2건 PII 격리(sensitive/). 평균 49.1초/건 (claude_cli, fastembed 비활성)
- **doc_type 다양성**: 13종 (Java/DB/Web 코퍼스 풍부) — Phase 86 측정의 -0.6% Metadata blocking 무효과 원인이 코퍼스 의존임을 시사
- **검증 통과율**: 8/8 = **100%** (디폴트 임계값)
- **A1 캐시**: 8 entries / 0 hits (첫 가공이라 미스만 — 재가공 시 hit 측정 가능)
- **C2 PII FP 추정**: 2/10 격리 모두 FP 가능성 높음 (DB 도구 문서의 예제 코드 + 우연 매칭). **100건+ 표본 측정 필요**
- **lint_strong_claims 검증**: Python 사본으로 가공본 8건 검사 → 8건 검출, TP/FP 약 50/50 추정. wikidocs 353407 권고대로 사용자 검토 후보 목록 설계가 적절
- **사이드 수정**: `LocalVectorStore::new()`가 PIPELINE_BASE 미통합 → 본 측정 중 발견·수정 (`adapters/local_store.rs::resolve_data_base()`). lesson 29 / Phase 85 B-4 보강

### Phase 88 부분 — lesson 14 부분 해소

`detect_strong_claims` 호출처:
- Phase 87: 0건 (인프라만)
- 본 phase: **단위 테스트 3건 + Python 사본 실 코퍼스 검증** → 실 효과 확인
- 다음 phase: `Linter::lint_strong_claims`의 schedule task 호출 연결 시 완전 해소

---

---

## 누적 변경 요약 (2026-05-15) — Phase 87 일괄 (lint 고도화 wikidocs 353407)

> 외부 문서 wikidocs 353407(정리와 감사 흐름) 권고 중 부분 미구현 항목 적용. 본 프로젝트는 권고 ~90% 이미 구현, 본 phase는 나머지 인프라 추가.

### A-1 Metadata 확장 (확인 필요 / 다시 물어볼 질문)

- `Metadata.needs_verification: Vec<String>` — 원천 자료 미확인 또는 추가 검증 필요 주장. 빈 Vec = 전부 확인됨
- `Metadata.open_questions: Vec<String>` — 원천 자료로 답할 수 없는 후속 질문. 빈 Vec = 후속 질문 없음
- `#[serde(default)]` 적용 — 기존 인덱스 호환

### A-2 detect_strong_claims() 함수

- `crates/core/src/domain/verification.rs::detect_strong_claims(processed)` — 단정 표현 12종 마커(확실히/반드시/항상/모든/100%/always/never 등)로 문장 추출
- 반환: `Vec<String>` (점수화가 아닌 사용자 검토 후보 목록 — 메타 룰 2 "검증 = 거부가 아니라 피드백")
- 단위 테스트 4건: 한국어 / 영어 / 검출 없음(약한 표현) / dedup
- **현 상태**: 단위 테스트로만 검증, 호출처 0건 (인프라만, lesson 14 패턴 — 다음 phase에서 lint 흐름 통합)

### A-3 lint 다층 주기

- `ScheduleConfig.lint_weekly_hours: u64` (기본 168=7일) — 중복·미연결 검사
- `ScheduleConfig.lint_monthly_hours: u64` (기본 720=30일) — 오래된·상충 검사
- 기존 `lint_interval_hours` 의미 보존(색인 정합성 단주기)
- config_metadata + Settings UI 노출

### 회귀 기준선

- workspace lib **340** 통과 (96 core + 149 adapters + 95 shared, Phase 86 336 + 4 신규)
- workspace clippy `--all --tests` **0건** 유지
- workspace + Tauri `cargo check` ✅
- 신규 config: `schedule.lint_weekly_hours` / `schedule.lint_monthly_hours`
- 신규 Metadata 필드 2종 — 디폴트 빈 Vec

### 주요 lesson

- lesson 40 신규 — 외부 문서 권고 도입 3단계 분리 / 단정 표현은 점수화 아닌 후보 목록 / 병렬 필드 확장의 호환성 / 외부 출처 코드 주석 명시

### 외부 문서 분석 결론 (Phase 87 진입 직전)

| 문서 | 본 프로젝트 적용 결과 |
|------|---------------------|
| supertone-inc/supertonic | TTS 시스템, 본 프로젝트 직접 연관 없음. ONNX 정적 링크 패턴은 fastembed에 이미 차용 (Phase 62) |
| wikidocs 352523 (자기 진화 에이전트) | Ruflo C1/decision_log/config_snapshot이 이미 부분 적용. 추가 도입 가치 낮음 (단일 사용자 데스크톱 도구) |
| wikidocs 353407 (정리와 감사 흐름) | **본 phase에서 부분 적용 완료**. ~90% 기 구현, 4건 미구현 중 3건 적용, 1건(수집·점검 분리)은 본 프로젝트에 부적합 |

---

## 누적 변경 요약 (2026-05-15) — Phase 86 일괄 (위생 후속 + 트리거 인프라)

> Phase 85 위생 마무리 + 측정 무관 트리거 인프라 선구현. 디폴트 비활성으로 lesson 30 패턴 유지.

### A-3 lesson 36 종결 표시
- lesson 36 "잔존 8건"의 too_many_arguments / very_complex_type / loop_var_index 항목에 Phase 84/85 종결 표시 추가
- 다음 phase 진입 시 stale 재발 방지

### A-4 spec/deprecated.md 신규 — dead 자산 단일 진실원
- 삭제/보류/폐기 항목 인벤토리 단일 위치
- `CrossRefUpdater::auto_link` / feedback_* / credential_store_* / cli.rs / Phase 84 dead 7건 / vendor/onnxruntime 등 누적
- 월 1회 점검 규칙 명시 (lesson 14 "미연결 포트는 코드 부담" 누적 가시화)

### A-5 architecture.md Phase 65~78 추가 아카이빙
- 1758 → **1368줄** (-390줄, 누적 1876→1368 = -27%)
- `architecture-archive.md` 153 → 527줄
- 분리 범위: Phase 65~78 (2026-05-04 ~ 2026-05-07) — 추천 시스템 + IA 재설계 + UI 정합성
- 본문엔 한 줄 포인터 + 시기 요약만 유지

### A-2 표 마크다운 보존 청킹 (트리거 #8 인프라)
- `SemanticChunkConfig.preserve_tables: bool` 신규 (디폴트 false, lesson 30 패턴)
- `ChunkingConfig.preserve_tables` 신규 + config_metadata "표 마크다운 보존" 노출
- `split_by_headings_with_path` / `split_paragraphs`에 `is_table_line` 헬퍼 기반 `in_table` 추적 — true 시 헤딩/HR 분할 + 빈 줄 단락 분할 금지
- 단위 테스트 2건 추가 (default off / when enabled)
- 트리거: 표 비중 높은 도메인(법률/회계/통계) 진입 시 활성화

### A-1 HyDE 폴백 검색 (트리거 #6 인프라)
- `LLMPort.generate_hypothetical(query)` 디폴트 메서드 추가 (디폴트 = query 반환 = no-op)
- `McpState.hyde_enabled: bool` + `hyde_min_results: usize` 필드
- `SearchConfig.hyde_enabled` (default false) + `hyde_min_results` (default 3) configField 노출
- `handle_search` 본문에 HyDE 분기 — CRAG 처리 후, 결과 < min_results + 활성화 시 LLM 가상 답변 임베딩으로 재검색. 추가 결과는 score × 0.6 감산
- 단위 테스트 2건 (default 비활성 / 비활성 시 폴백 없음)
- 트리거: 실사용 "검색 안 됨" 피드백 도달 시 활성화. 어댑터별 prompts.toml에 hyde 프롬프트 추가하면 즉시 동작

### 회귀 기준선

- workspace lib **336** 통과 (96 core + 145 adapters + 95 shared, Phase 85 332 → +4 신규)
- workspace clippy `--all --tests` **0건** 유지
- workspace + Tauri `cargo check` ✅
- Tauri commands 70 / MCP tools 32 변동 없음
- settings.db 테이블 변동 없음
- 신규 config: `chunking.preserve_tables` / `search.hyde_enabled` / `search.hyde_min_results` (모두 디폴트 비활성)

### 주요 lesson

- lesson 30 패턴 재적용: 측정 무관 트리거 인프라는 디폴트 비활성으로 선구현 → 측정 후 1줄 디폴트 변경으로 활성
- A-4(deprecated.md) — lesson 14 누적 가시화 위치 단일화

### 실 코퍼스 재측정 (2026-05-15, D:/file-test/samples 485 가공 파일)

3회 중앙값 (`spec/benchmarks/crossref_variants_real_485_20260515.json`):

| 변형 | 시간 중앙값 | vs baseline | 관계 | vs baseline | 디폴트 변경 |
|------|-------------|------------|------|------------|------------|
| baseline (0.7) | 25.92s | — | 11166 | — | 기준 |
| threshold 0.8 (적용됨) | 27.89s | +7.6% | 2848 | **-74.5%** | ✅ Phase 85 적용 |
| **MinHash force** (#2) | 26.34s | +1.6% | 8940 | -19.9% | ❌ 보류 재확인 |
| **Metadata blocking** (#4) | 27.36s | +5.6% | 11094 | **-0.6%** | ❌ 보류 재확인 |
| all (0.8+mh+block) | 30.82s | +18.9% | 2242 | -79.9% | 과적용 |

**의사결정**:
- 트리거 #2(MinHash 디폴트 활성): 관계 -19.9% (threshold 0.8의 -74.5%에 비해 부차적). 디폴트 변경 보류 재확인 (325파일 측정 결정과 일치)
- 트리거 #4(Metadata blocking 디폴트): Java/Spring 코퍼스 doc_type 다양성 부족(other 압도적) → 관계 -0.6%. **doc_type 다양성 확보된 5K+ 코퍼스에서 재측정 필요**
- 본 측정으로 1번 더 보류 누적 — 인프라(`crossref.minhash_force`, `crossref.metadata_blocking` configField)는 즉시 토글 가능

---

## 누적 변경 요약 (2026-05-15) — Phase 85 일괄 (위생)

> 트리거 무관 후속. 측정 의존 항목(5K 코퍼스)과 분리하여 코드 위생만 일괄 처리.

### B-1 clippy `too_many_arguments` 4건 → 입력 구조체

- `crates/core/src/domain/cross_reference.rs`: `CrossRefUpdateContext<'a>` 신규 — `update_cross_references` 8 인자 캡슐화
- `crates/shared/src/auto_suggester.rs`: `DecisionDraft<'a>` 신규 — `make_entry` 8 인자 캡슐화 (4 호출처)
- `crates/shared/src/settings_db.rs`: `NewTodo<'a>` 신규 — `add_todo` 8 인자 캡슐화 (5 호출처, public API 변경)
- `#[allow(clippy::too_many_arguments)]` 4건 제거. `cargo clippy --all --tests` 경고 0건 유지

### B-2 `auto_link` 삭제 + 보류 마커 (lesson 14 형태)

- 호출처 0건 7+ Phase 미사용. `update_cross_references`(LLM 기반)가 동일 영역 커버
- 본 줄 위치(`crates/core/src/domain/cross_reference.rs`)에 보류 마커 주석 — 원본 시그니처·사유·재도입 트리거·git 복구 명령 명시
- 약 164줄 감소 (함수 138 + struct 17 + docstring 8 + unused import 1)

### B-3 architecture.md Phase 64 이하 아카이빙

- 1876 → **1758줄** (-118줄). 분리 범위: 트리거 #1/#11/#12 + release 빌드 + Phase 61~64
- 신규 `spec/architecture-archive.md` (153줄, 읽기 전용). 본문엔 한 줄 포인터만 유지
- Phase 65 이후 + 영구 섹션(한 줄 요약·모달 매핑·도메인별 수치 등)은 본문 유지

### B-4 `find_data_dir` ↔ `resolve_paths` base 결정 통일

- 사이드 발견 6 ("auto_init이 PIPELINE_BASE와 exe_dir 양쪽에서 inbox 생성") 진단 결과 — 실제 원인은 base 결정 함수가 두 개로 분리되어 다른 분기 트리를 따른 것
- `resolve_paths`가 base 결정을 `find_data_dir`에 위임. `config.paths.base`는 PIPELINE_BASE 미설정 + cli_base 미지정일 때만 적용
- CLI/Tauri가 같은 분기 트리(PIPELINE_BASE → cwd settings.db/toml → exe_dir → APPDATA)를 사용하도록 통일

### B-1+ tests clippy 위생 (4건)

- `field_reassign_with_default` 3건: `ProcessingSummary::default()` 후 필드 대입 → struct 리터럴 + `..Default::default()`
- `assertions_on_constants` 1건: `assert!(true, ...)` → 주석 + 본문 없음
- 발견 위치: `modals/cli/tests/notification_integration.rs` / `modals/cli/tests/real_env_tests.rs` / `crates/adapters/src/driven/notify/format.rs` (2026-06-16 notification → notify 디렉토리 정정, lesson 77)

### 회귀 기준선

- workspace lib **332** 통과 (96 core + 143 adapters + 93 shared)
- workspace clippy `--all --tests` 경고 **0건** (lib 0 + tests 0)
- workspace + Tauri `cargo check` ✅
- Tauri commands **70** / MCP tools **32** 변동 없음
- settings.db 테이블 변동 없음

### 주요 lesson

- lesson 38 신규 — 빌더 vs 입력 구조체 선택 기준, lesson 36 "잔존 8건" stale 검증, 같은 의미 함수 다중 정의 = lesson 1 변형, 아카이빙은 사용자 결정 영역

---

## 누적 변경 요약 (2026-05-15) — Phase 84 일괄 (P1·P2·P3)

### P2 백엔드 정리
- **Dead Tauri commands 7건 삭제**: `get_health` / `get_lint` / `delete_document` / `fix_backlinks` / `get_retention_config` / `get_pipeline` / `save_pipeline` (Phase 64 주석에 명시된 frontend 정리 대상). main.rs invoke_handler에서도 제거.
- **setup_snapshot_list / setup_snapshot_rollback UI 연결**: Decision Log 카드 행에 `↶ Rollback` 버튼 추가 (accepted + snapshot_id + !rolled_back 조건). `setup_decision_log_list` 응답에 snapshot의 `rolled_back` 플래그 합성하여 노출.
- **clippy lib warning 8 → 0**:
  - `chunking.rs:197` `for lower in (level+1)..3` → `headings.iter_mut().skip().take()`
  - `crossref_optimizer.rs:88` `for i in 0..num_perm` → `for (i, slot) in sig.iter_mut().enumerate()`
  - `service.rs:129` `Option<Arc<dyn Fn>>` → `ProgressCallback` type alias
  - `cross_reference.rs:115/310`, `auto_suggester.rs:191`, `settings_db.rs:533` → `#[allow(clippy::too_many_arguments)]` (도메인 시그니처 트레이드오프)
  - `mcp_server.rs:433` → `#[allow(clippy::manual_async_fn)]` (rmcp trait 시그니처)

### P1-1 코드 상수 → config 이전 (조사 결과: Phase 71에 이미 완료)
- Memory Tier(`memory_tier.*`) / flush_crossref(`crossref.flush_interval_secs`) / Sparse·RRF·시간 가중(`search.sparse_weight`, `vector_db.rrf_multiplier`, `search.time_weight`) / Lint(`schedule.lint_interval_hours=0`) 모두 이미 config 노출. 추가 작업 없음.

### P1-2 HookDefinition UI CRUD
- Settings 인프라 그룹의 readonly 표 → 추가/편집/삭제 모달. `_openHookModal(state)` 모달: event 5종 select + webhook/command 라디오 + enabled 체크박스. `save_config`로 영속.

### P1-3 Quarantine 분기 노드 시각화
- `PB_NODES.quarantine`에 `branch_from:'verify'` + `branch_condition` 메타 추가.
- `_renderPBNodeCard`가 `branch_from` 있는 노드에 점선 보더 + 주황 배경 + `↘ FAIL 분기` 배지 표시.
- CSS `.pb-node.pb-node-branch` / `.pb-node-branch-badge` 신규 (dashboard.css).

### P1-4 search_with_trace 구현
- 신규 Tauri command `search_with_trace`: Dense / Hybrid(RRF) / Filtered 3단계 결과를 각각 top-K로 반환.
- 검색 시뮬레이션 UI: 단계별 표 + Dense 대비 순위 변화 (↑/↓/신규/=).
- Phase 69의 placeholder 문구 제거.

### P1-5 MCP enable/disable 토글
- `settings.db.mcp_disabled_tools` 테이블 신규: 존재하는 행 = 비활성.
- `mcp_server.rs::call_tool` 진입에 차단 + `list_tools` 응답에서 필터링.
- 신규 Tauri commands: `mcp_tools_list` (32종 카탈로그 + enabled 상태) / `mcp_tool_set_enabled` (활성/비활성 + reason).
- Settings 탭 `_renderMcpTools` placeholder → 토글 가능한 표 (재시작 불필요).

### P1-6 get_processing_metrics 통합
- `service.summary` 런타임 ProcessingSummary를 `runtime_summary`로 추가 노출 (영속 카운터와 분리).
- "현 세션 vs 전체 누적" 비교 가능. Phase 80 placeholder 해소.

### P3-1 C1/C2 GUI live reload
- `service.pii_user_patterns: Vec → RwLock<Vec>` 변경. `service.reload_pii_patterns()` 메서드 신규.
- `pii_pattern_add/remove` 응답에 `live_reloaded: true` + `active_count`. UI 안내 메시지 "재시작 후 반영" → "즉시 반영, 활성 N건".
- `c1_threshold_set` 응답에도 `live_reloaded: true` (auto_suggester가 매번 settings.db read하므로 별도 reload 불필요).
- RwLockReadGuard는 async에서 Send 불가 → 사용처 2곳에서 `read().clone()`으로 owned Vec 추출.

### P3-2/P3-3 A1 LRU 즉시 GC + last_gc 카드
- `settings.db.llm_cache_gc_log` 테이블 신규 (id=1 단일 행 upsert).
- 신규 Tauri command `gc_llm_cache_now`: max_entries 미지정 시 `cfg.llm.llm_cache_max_entries` 사용. 결과를 `record_llm_cache_gc`로 기록.
- 헤더 LLM 캐시 그룹에 `[GC 실행]` 버튼 + `최근 GC` (timestamp) + `GC 삭제` (건수) 카드 2개 추가.
- 4h 주기 GC (service.rs)에서도 `record_llm_cache_gc` 호출하여 카드 갱신.

### P3-4 C1 주기 트리거 (기 통합 확인 + 보강)
- `modals/app/src/service.rs`에 `auto_suggest_interval_hours` 기반 주기 spawn 이미 통합됨 (4h 디폴트).
- GC 결과 `record_llm_cache_gc` 호출 추가로 GUI 가시화 완성.

### settings.db 신규 테이블 (2건)
- `mcp_disabled_tools(tool_name PK, disabled_at, reason)` — MCP 도구 토글
- `llm_cache_gc_log(id=1, last_at, last_deleted)` — A1 GC 이력 (단일 행 upsert)

### 회귀 기준선
- workspace lib **332** 통과 (core 143 + adapters 96 + shared 93)
- workspace clippy lib warning **0건** (8 → 0)
- 통합 테스트 빌드 통과 (12파일 모두)
- Tauri commands: 72 → **70** (delete 7 + add 5 = -2 net)
- 빌드 메모리 이슈: `-j 2`로 회피 (페이징 파일 제약, lesson 후보)

---

## 누적 변경 요약 (2026-05-15) — 이전 (Ruflo)

### Ruflo 영감 6건 (A1/A2/B1/B2/C1/C2)
- **A1** LLM 결과 캐시: `cached_llm.rs` LLMPort wrapper + `settings.db.llm_cache` + Dashboard 카드 + 비우기 버튼
- **A2** KG 1-hop 확장: `McpState.expand_kg_hops` + handle_search 후처리 + configField `search.expand_kg_hops`
- **B1** 다양성 swap: `McpState.diversity_threshold` + 동일 doc_type dominant swap + configField `search.diversity_threshold`
- **B2** 백그라운드 워커: watcher Semaphore + tokio::spawn 패턴 기 적용 (`max_workers` configField 노출, 신규 작업 없음 — lesson 34)
- **C1** 자기학습: `auto_suggester.rs` 누적 카운터 분석 → decision_log INSERT, 사용자 confirm 후 toml 자동 적용, 4시간 주기 trigger, 임계값 7종 DB 가변화 (`c1_rule_thresholds`)
- **C2** PII 검출: `SensitivityDetector::scan_pii_in_text_with` regex 5종 + 사용자 추가 (`pii_patterns_user`), service.rs 두 진입점 통합

### 회귀 기준선
- workspace lib **332** 통과 (core 96+143 + adapters 0 + shared 93)
- Tauri commands **72** / MCP tools **32**
- settings.db 신규 테이블: `decision_log` / `llm_cache` / `c1_rule_thresholds` / `pii_patterns_user`
- compile warnings: workspace 0 / clippy **8** (71 → 18 자동 fix + 5 수동 = 8 잔존)
- Settings 탭 카드: **4** (Decision Log + C1 임계값 + PII 패턴 + config 폼)

### 주요 lesson (29~36)
| # | 제목 | 핵심 메타 룰 |
|---|------|--------------|
| 29 | CLI ↔ Tauri 데이터 격리 | PIPELINE_BASE 환경변수 통합 |
| 30 | Ruflo 인프라만 선구현 | lesson 15 패턴 재적용 (인프라 + 디폴트 비활성) |
| 31 | A1 통합 + PIPELINE_BASE 잔존 | wrapper 어댑터로 헥사고날 유지 + 4곳 find_data_dir 통합 |
| 32 | A1 가시화 + C1 1단계 + 위생 | API 정의만 vs Dashboard 카드 통합 차이 |
| 33 | C1 2단계 + A1 비우기 + startup | write_toml_path 헬퍼 노출 + Tauri .setup() 별도 스레드 |
| 34 | Decision Log 카드 + C2 5종 + B2 평가 | 신규 작업 사전 grep 평가 (B2 기 완료 발견) |
| 35 | C2 service 통합 + 사용자 룰 | 신규 함수 미연결 (lesson 14 재발 방지) + 4곳 동기화 |
| 36 | C1/C2 GUI + A1 LRU GC + clippy 잔존 | 운영 데이터 자동 정리는 주기 task 통합 패턴, Settings 4카드 |



## ServiceBuilder 도입 + 통합 테스트 점진 마이그레이션 (2026-05-14)

lesson 21/27 재발 차단 작업. 도메인 구조체 `FileProcessingService` 필드 추가 시 통합 테스트 12파일을 일일이 수정해야 했던 문제를 빌더 패턴으로 해소.

### 핵심 추가
- `crates/shared/src/test_helpers.rs` 신규
- `ServiceBuilder::new(base).with_*(...).build()` — 모든 도메인 필드를 안전한 stub 기본값으로 초기화
- with_* 메서드 18종: llm/storage/vector_db/embedding/notification/verification/preprocessing/remote_storage/duplicate_resolution/sensitive_notification/registry/semantic_dup_threshold/max_retry/verification_enabled/fragment_threshold/crossref_*/embed_instruction_prefix/global_thresholds/metrics_recorder
- 자체 단위 테스트 2건 (shared 77 → 79)

### 🎉 통합 테스트 마이그레이션 12/12 완료
| 파일 | 변경 | 결과 |
|------|------|------|
| scenarios.rs | 50줄 → 10줄 | 10/10 통과 |
| actor_scenarios.rs | 51줄 → 11줄 | 4/4 통과 |
| search_accuracy.rs | 50줄 → 11줄 | 12/12 통과 |
| bench_prompt_compare.rs | 50줄 → 12줄 | 빌드 통과 (claude CLI 없으면 자동 스킵) |
| real_env_tests.rs | 58줄 → 18줄 | 13/13 통과 |
| bench_real.rs | 41줄 → 10줄 | 빌드 통과 (PIPELINE_REAL_BENCH 환경변수 의존) |
| benchmark.rs | 44줄 → 10줄 | 3/3 통과 |
| bench_real_docs.rs | 64줄 → 9줄 | 빌드 통과 |
| bench_real_corpus.rs | 88+82줄 → 13+13줄 | 빌드 + 실 코퍼스 24.4 docs/s 통과 |
| bench_micro.rs | 71줄 → 14줄 | 6/6 통과 (343s) |
| bench_scale.rs | 53줄 → 14줄 | scale_100 12.1 docs/s + scale_1000 19.6 docs/s 회귀 기준선 통과 |
| **e2e_embedded.rs** | **85+85줄 → 19+19줄** | **21/21 통과 (4.61s)** |

**총 코드 감소**: 12파일 평균 -77% (init 본문 줄수). lesson 21/27 근본 차단 완료.
**효과**: 향후 `FileProcessingService` 도메인 필드 추가 시 통합 테스트 12파일 모두 변경 0건. `ServiceBuilder::build()` 한 곳만 수정.

### 효과
- 5파일 평균 50줄 → 11줄 (-78%)
- 향후 `FileProcessingService` 필드 추가 시 마이그레이션된 5파일은 변경 0건
- `ServiceBuilder::build()` 본문만 수정하면 신규 도메인 필드 자동 적용

### 새 규칙 승격
- src/CLAUDE.md "아키텍처 규칙"에 "신규 통합 테스트는 ServiceBuilder 사용 의무" 추가
- META.md 메타 룰 1 "다중 위치 동기화 누락"의 구조적 대응책



## Tauri GUI 실 파일 가공 검증 (2026-05-14)

D:\file-test\samples 10건(txt 6 + docx 2 + html 2)을 Tauri GUI inbox에 투입하여 실제 파이프라인 작동 확인:

### 통과
- **DOCX 네이티브 가공**: `예외(Exception).docx` → study_note 분류 + Java 예외 키워드 (Phase 39 calamine/zip)
- **다중 doc_type 분류**: LLM이 "technical_note configuration" / "troubleshooting note" 같은 multi-type 정확히 부여
- **한국어 자연어 요약**: 2~3문장으로 핵심 추출 ("스프링부트(Spring Boot)에서 프로파일(profile) 기반...")
- **한국어 + 영어 혼합 키워드**: 14개 키워드 추출 (스프링부트/springboot/profile/...)
- **fastembed 임베딩 영속화**: vec_offset/vec_dim 0/1024 — feature 비활성 폴백 시에도 dim 보존
- **2종 산출물**: processed/에 `.zst`(가공본 압축) + `.vec`(임베딩) 1:1
- **doc_type prefix 파일명**: `technical_note_*.txt.zst` — 라우팅 + 검색 hint

### 사이드 발견 (lesson 후보) + 해소 표시 (2026-05-14)

1. ✅ **GUI 백그라운드 가공 로그가 파일 미기록** → **해소**: `modals/app/src/service.rs` progress_callback에 write_log 추가. start/done/error/fragment 이벤트가 `logs/pipeline.log`에 INFO/WARN 기록. fastembed 재검증 시 `{"event":"start","file":"..."}` + `{"event":"done","file":"...","types":"..."}` 정상 기록 확인
2. ✅ **CLI ↔ Tauri 데이터 격리** → **lesson 29 + 코드 fix**: `find_data_dir`에 PIPELINE_BASE 환경변수 지원 추가. settings.db/inbox/processed 등 모두 PIPELINE_BASE 적용 작동 확인. 잔존 이슈: `logs/` writer 초기화 경로가 별개라 exe_dir에 leak (write_log writer 초기화 우선순위 다음 lesson 후보)
3. ✅ **`quarantine/` 디렉토리 auto_init 미생성** → **해소**: `ResolvedPaths::create_all`에 quarantine 디렉토리 추가
4. **LLM 호출 시간** → fastembed 재검증으로 **부분 가속 확인** (검증 결과 섹션 참조)
5. **relations 0건** — Phase 47 30초 유휴 정책 (수정 안 함)
6. ⚠️ **신규 발견**: auto_init이 PIPELINE_BASE와 exe_dir 양쪽에서 실행 — `inbox/` 등이 두 곳에 생성. find_data_dir 분기 우선순위 검토 필요

### 검증 도구

- `.local-store.json` 직접 파싱: 가공된 문서의 metadata 확인
- `processed/` + `originals/` 파일 카운트: 1:1 비율 검증
- `logs/pipeline.log`: 초기화 단계 + 파일별 progress event 검증 (사이드 발견 1 해소 후)
- Monitor 도구: inbox→processed 카운트 변화 실시간 추적

### fastembed feature 재검증 (2026-05-14)

같은 10건 샘플로 fastembed 활성 빌드 재검증:

**산출물**:
- pipeline.exe **17.2 → 40.6 MB** / file-pipeline-tauri.exe **20.4 → 42.8 MB** (ort 정적 링크)
- 빌드 시간: workspace 1m 42s + Tauri 5m 43s
- MSVC v14.44 (lesson 18 요구사항 v14.38+ 충족)

**런타임**:
- fastembed BGE-M3 모델 첫 로드: **80초** (Phase 62 lesson 66초와 유사)
- 가공 속도: 시작 t=10s에서 fastembed init 감지 → 첫 파일 done t=58s (1건 26초). **이전 검증의 t=4분에 4건 완료 vs fastembed의 t=34s에 4건 완료 = 약 6배 빠름**
- 백그라운드 progress 로그 정상 기록 (사이드 발견 1 해소 검증)

**관찰**:
- LLM 호출은 여전히 claude_cli (LLM 분류·가공). 임베딩 단계만 fastembed로 가속
- 실제 가공 시간 단축은 임베딩이 차지하는 비중에 따라 다름 (lesson 70 — LLM이 큰 비중)
- `default_model='fastembed'` WARN 사라짐 → "fastembed 폴백" 메시지 없음

---

## MCP Playwright UI 정합성 자동 검증 (2026-05-14)

본 세션 Phase 80 6개 Tauri commands 추가 + ServiceBuilder 마이그레이션 후 UI 정합성을 자동으로 재검증. HTTP 서버(`python -m http.server 8765` on `ui/`) + MCP Playwright `browser_evaluate`로 DOM 정합성 측정. invoke 의존 동작(데이터 흐름)은 lesson 55/59 한계로 검증 불가 — 별도 `doc/gui-test-scenarios.md` 수동 가이드 작성.

### 자동 검증 통과 항목 (8 시나리오)

| # | 시나리오 | 결과 |
|---|---------|------|
| 1 | 페이지 로드 + title | ✅ "file-pipeline Dashboard" |
| 2 | 콘솔 JS 에러 0건 | ✅ (favicon 404만, 무시) |
| 3 | 7탭 전환 (documents/pipeline/processing/todos/settings/topics/verification) | ✅ 모두 tabActive + contentActive |
| 4 | 헤더 토글 (collapsed false → true → false 복원) | ✅ |
| 5 | 검색 입력 (`#search-query` + Enter 시뮬레이션) | ✅ value 유지 |
| 6 | AI 설정 도우미 모달 open ([data-action="open-setup-assistant"]) | ✅ Phase 75 모달 텍스트 표시 |
| 7 | **Phase 80 진입점 3분기** (⚡ 일반 / 🤖 AI / 🧩 동작 모듈 + (고급) 5축) | ✅ 4개 카드 모두 존재 |
| 8 | 🧩 동작 모듈 모달 open ([data-action="open-setup-modules"]) | ✅ 모달 + Critical 동의 + dryrun/적용 버튼 |
| 9 | Pipeline 노드 카운트 (가공 21 + 검색 18 + 배치 = 43) | ✅ 43 노드 DOM 존재 |

### 자동 검증 한계 (lesson 55/59)

HTTP 서버 모드는 `__TAURI_INTERNALS__` 미존재 → invoke 호출이 fallback `{}` 반환:
- 🧩 모듈 모달 안 **12 체크박스 미렌더** (setup_modules_list invoke 결과 없음)
- 크레덴셜 "추가" 버튼 등 데이터 의존 UI 미렌더
- 검색 결과 / 문서 목록 / KG ego graph / 토픽 카드 / 통계 카드 값 등 invoke 의존 데이터 흐름 검증 불가

→ 본 세션 추가한 Tauri commands 6개의 실제 작동은 Tauri WebView 환경에서만 검증 가능. `doc/gui-test-scenarios.md` E4/E5 시나리오를 사용자가 수동 검증해야 함.

### 메타 발견

JS의 `call` 헬퍼(`dashboard.js:6`)는 invoke 미존재 시 빈 객체 반환:
```js
const call = (cmd, args) => invoke ? invoke(cmd, args).catch(e => { console.error(`[${cmd}]`, e); return {}; }) : Promise.resolve({});
```
→ **HTTP 서버 모드에서 silent fail** — 에러 콘솔도 안 남음. Tauri commands가 등록되지 않은 채로 배포되어도 빈 UI만 표시될 위험. lesson 19 11항(JS-Tauri grep) 정기 점검 필수.

---

## Tauri commands 매핑 정합성 보정 (2026-05-14)

웹앱 동작 검증 중 lesson 19 11항(통합 테스트 단언 grep)을 frontend-backend invoke 매핑에도 확장 적용. JS `call('cmd')` ↔ Tauri `commands::cmd` 교차 grep 결과 frontend dead 6건 발견:

- `setup_modules_list / setup_apply_modules` — Phase 80 동작 모듈, MCP만 등록되고 Tauri 미등록
- `get_search_mode_stats / get_crag_stats / get_chunk_stats / get_processing_metrics` — Phase 80 코퍼스 신호 카운터, MCP만 등록

**증상**: JS가 invoke 호출 시 Tauri 측에 핸들러 없어 silent fail → 헤더 진입점 3분기 중 "🧩 직접 동작 모듈 선택" UI / 통계 카드가 빈 데이터로 표시됨. console.error로만 잡힘.

**해결**: `commands.rs`에 6개 함수 추가 + `main.rs` invoke_handler 등록. settings.db 직접 접근(`get_search_mode_counters` / `get_crag_counters` / `get_processing_metric_summary`)으로 MCP McpState 메모리 카운터와 동등 동작.

**결과**: Tauri commands 56 → **62개**. Frontend dead **6 → 0건**. Backend dead 11건(`get_health / kg_paths / setup_snapshot_*` 등)은 별도 작업 대기.

**메타 룰**: lesson 19 10단계에 11항 "JS call() ↔ Tauri commands grep 정합성" 추가 후보. 새 MCP 도구 추가 시 Tauri 등록도 함께 검토하는 규칙으로 승격 검토.

---

## 실 코퍼스 측정 (2026-05-14): D:\file-test\samples 325파일

3회 release 측정, 중앙값 채택:

| 회차 | total | docs/s | per-doc avg | p95 | 검색 avg/p95 |
|------|-------|--------|-------------|-----|--------------|
| 1 | 15.5s | 21.0 | 47ms | 90ms | 0.23 / 0.48 ms |
| **2 (중앙값)** | **12.5s** | **26.0** | **38ms** | **50ms** | **0.13 / 0.27 ms** |
| 3 | 25.2s | 12.9 | 76ms | 166ms | 0.30 / 0.56 ms |

- 코퍼스: 626파일 / 155MB / .html/.htm/.txt/.docx/.pptx/.pdf/.java 혼합. 처리 가능 325파일 → 310 hash-dedup
- 회귀 기준선 (per-doc p95 ≤ 100ms, 13 docs/s 이상) **통과**
- 단일 측정 편차 2배 (12.9~26 docs/s) — lesson 04 캐시 편향 재확인
- 코퍼스 건강도: 33 고립(10%) ⚠️ HashEmbedder 한계. fastembed BGE-M3 도입 시 해소 예상

### 트리거 #2/#4 실측 의사결정

같은 코퍼스로 5변형 비교 (`bench_real_corpus_variants`):

| 변형 | 시간 vs baseline | 관계 vs baseline | 디폴트 변경 |
|------|------------------|------------------|------------|
| baseline (0.7, mh=off, block=off) | — | 10034 | 기준 |
| threshold 0.8 (이미 적용) | -4.5% | -73.7% | ✅ 적용됨 |
| **MinHash force** (트리거 #2) | **+9.3%** | -21.0% | ❌ 시간 증가 → 보류 |
| **Metadata blocking** (트리거 #4) | -8.3% | **0%** | ❌ 효과 없음 → 보류 |
| all (0.8+mh+block) | +7.4% | -79.5% | 과적용 |

5K+ 코퍼스 도달 전엔 트리거 #2/#4 디폴트 변경 보류 (메모리 `project_phase59_done.md` 결정 실측 재확인). lesson 15 인프라(force/min_docs/blocking 옵션화)는 정상 작동, 즉시 토글 가능.

### 빌드 보정

Phase 82-prep `metrics_recorder` 필드 추가 시 통합 테스트 12파일 13곳 누락 → `metrics_recorder: None` 추가로 보정. lesson 21 재발(필드 추가 시 테스트 초기화 누락) — **lesson 27** 신규 작성. builder 패턴 또는 Default impl 도입 검토 트리거.

### 테스트 통계 (재측정 2026-05-14)

- workspace lib: **310** (adapters 96 + core 137 + shared 77, Phase 82와 동일)
- 통합 테스트 (벤치 제외): 58 통과 / 1 실패
- 실패 1건: `e2e_embedded::e2e_lint_with_stale` — Linter::lint stale_docs 빈 배열. 사전 결함 추적 → **lesson 28**.

---

## Phase 82 처리 (2026-05-14): Decision Log

`setup_apply` / `setup_apply_modules` 호출 시 각 ConfigChange 후보의 결정(accepted / rejected / critical_skipped)을 settings.db에 영속화. ConfigSnapshot(Phase 77)과 snapshot_id로 연결되어 "어떤 추천이 왜 적용/거부되었는가" 추적 가능. Proposal Diff 측면은 기존 `setup_dryrun`이 이미 제공하므로 제외.

### 신규 테이블 `decision_log`
- 컬럼: id(AUTOINCREMENT) / decided_at / source / snapshot_id / path / decision /
  before_value / after_value / priority / risk / evidence / confidence / reason / context
- 인덱스 3종: decided_at DESC / snapshot_id / path
- 한 번의 apply = N개 ConfigChange 후보 → N row (거부 항목 포함)

### 신규 API
- `apply_advice_full_with_log(config_path, advice, accepted, apply_critical, db, source, context)` — source/context 받아 decision_log에 기록. 기존 `apply_advice_full`은 source="setup_review", context=None으로 위임.
- `SettingsDb::insert_decision(&DecisionLogEntry)`
- `SettingsDb::list_decisions(limit)` — 최근 N건 (limit=0 전체). DESC + id DESC tiebreaker (동일 timestamp 안정성).
- `SettingsDb::list_decisions_by_snapshot(snapshot_id)` — 입력 순서 ASC

### 신규 MCP 도구
- `setup_decision_log_list { limit?, snapshot_id? }` — limit 기본 50, snapshot_id 지정 시 그것만

### 신규 Tauri 명령
- `setup_decision_log_list(limit, snapshot_id)`

### `setup_apply_modules` 변경
- `apply_advice_full_with_log` 사용. source="setup_modules", context={"module_ids":[...]}.

### 결정 분류 규칙
- `accepted`: accepted_paths 포함 + Critical 아니거나 apply_critical=true + TOML write 성공
- `critical_skipped`: accepted_paths 포함 + RiskLevel::Critical + apply_critical=false
- `rejected`: accepted_paths 미포함 OR TOML write 실패

### 신규 테스트 +5
- settings_db: `test_decision_log_insert_and_list` / `_filter_by_snapshot` / `_limit`
- setup_review: `test_apply_writes_decision_log` (accepted+rejected 혼합 + snapshot_id 연결 검증) / `test_apply_with_log_records_critical_skipped` (source=setup_modules + context 전달 검증)

### 의도된 비범위
- Proposal/Decide 2단계 분리 (현재 1단계 apply 유지)
- 거부 reason 자유입력 UI (rejected 자동 마킹만)
- 자동 결정 정책 / 다중 사용자 결정 권한자

### 테스트 통계
workspace lib 310건 통과 (adapters 96 + core 137 + shared 77, +5 신규).

## Phase 82-prep 처리 (2026-05-14)

Phase 77 자동 롤백·setup_snapshot_measure가 의존하던 `verify_pass_rate` / `quarantine_rate` / `avg_process_time_ms` placeholder 0을 실측치로 교체.

### 신규 포트 (core)
`ProcessingMetricsPort` (sync, default no-op):
- `record_success` / `record_error` / `record_quarantine`
- `record_verify(passed: bool)`
- `record_process_time(elapsed_ms: u64)`

`FileProcessingService.metrics_recorder: Option<Arc<dyn ProcessingMetricsPort>>` 필드 추가.
호출 헬퍼 `metrics_success/error/quarantine/verify/time` 5종. 호출 지점:
- legacy 경로 + pipeline 경로 양쪽의 verify 분기(Pass/Warning/Fail2차) + quarantine 이동 + record_success/error
- pipeline 경로 시작 시점에 `metrics_t_start` 인스턴스 측정, 성공 종료 시 `metrics_time` 호출

### 신규 어댑터 (shared)
`SettingsDbMetricsAdapter` — settings.db `processing_metrics` 테이블에 UPSERT 누적. DB 락/오류는 silent (`let _ = ...`). `build_service`에서 `paths.base.join("settings.db")`로 자동 주입.

### 신규 테이블 `processing_metrics`
key-value 누적 카운터 (7키):
- `success` / `errors` / `verified_pass` / `verified_fail` / `quarantined`
- `total_time_ms` / `counted_for_time` (avg 산출)

### settings.db schema 단일 상수화 (lesson 26 해소)
`open()` + `open_in_memory()` DDL 이중 정의를 `SETTINGS_DB_SCHEMA: &str` 상수 단일 진실 소스로 통합. 신규 테이블/인덱스 추가 시 상수 한 곳만 수정. lesson 10 재발 패턴 차단.

### MCP 도구 변경
- `get_processing_metrics` 응답: placeholder null → settings.db 산출치 (`verify_pass_rate` / `quarantine_rate` / `avg_process_time_ms` + `counters: {success/errors/quarantined}`)
- `collect_current_metrics` (setup_snapshot_measure 내부): placeholder 0 → settings.db `ProcessingMetricSummary` 산출치

### 데이터 부족 처리
- `verified_pass + verified_fail == 0` → `verify_pass_rate = None` (JSON null)
- `success + errors == 0` → `quarantine_rate = None`
- `counted_for_time == 0` → `avg_process_time_ms = None`
- `collect_current_metrics`는 None을 0.0/0으로 변환 (RollbackThresholds 기존 동작 보존)

### 신규 테스트 +3
`test_processing_metric_increment` / `test_processing_metric_summary_empty` / `test_processing_metric_summary_rates` (8:2 verify, 0.1 quarantine, 1000ms avg 검증).

### 테스트 통계
workspace lib 305건 통과 (adapters 96 + core 137 + shared 72, +3 신규).

## Phase 83 처리 (2026-05-14)

### 관계 origin 속성
사용자 요청 "관계도에 속성값 추가 — 사용자_참조 / 자동_생성_참조" 반영.

`DocRelation`에 `origin: RelationOrigin` 필드 신설 (5종):
- `auto_similarity` — 임베딩 유사도 자동 (기본, 가중치 0.5)
- `user_wikilink` — `[[xxx]]` 위키링크 추출 (0.95)
- `llm_extracted` — LLM 가공 시 references 명시 (0.85)
- `user_manual` — UI 수동 추가 (1.0)
- `lint_auto_fix` — Lint 자동 수정 적용 (0.7)

`RelationOrigin::label_ko()` — UI 표시용 한글 라벨 (`자동_유사도` / `사용자_위키링크` / ...).

`VectorDBPort::link_with_origin()` 기본 메서드 추가. 기존 `link()`는 default `AutoSimilarity`로 위임. LocalVectorStore에서 `StoredRelation.origin` 컬럼으로 영속화 (serde default로 후방호환).

`KgEdge` / `GraphEdge` 응답에 `origin` + `origin_label_ko` 필드 노출 (kg_neighbors / kg_paths / kg 전체 그래프).

### 위키링크 추출
`core/domain/wikilink.rs` 신설:
- `extract_wikilinks(text)` — `[[xxx]]`, `[[xxx#section]]`, `[[xxx|alias]]` 패턴 → 소문자화·dedup
- `resolve_wikilink_target(target, docs)` — 파일명(확장자 제거) case-insensitive 매칭
- 한국어/영문 모두 지원, 미닫힘 패턴 무시
- 단위 테스트 7건

`CrossRefUpdater::link_wikilinks()` — upsert 직후 service.rs에서 호출. 자동 crossref(`auto_link`) 전 실행하여 명시 관계를 먼저 등록. `References + ReferencedBy` 양방향 + `UserWikilink` origin.

## Phase 81 처리 (2026-05-13)

### 호스트 도구 감지 settings.db 캐시
사용자 요청 "호스트 전처리 도구 감지 등의 고정 설정은 매번 불러오는건 낭비" 반영.

문제: `HostToolDetector::detect()`이 매번 외부 프로세스 4종 spawn (200~1000ms). `CompositePreprocessor::new()` + `get_host_tools` Tauri command 매 호출마다 발생.

신규 settings.db 테이블 `host_tools_cache`:
- `tool` PK (pandoc/python_docx/python_openpyxl/libreoffice)
- `version`, `detected_at`, `not_found` (음성 캐시), `install_hint`

`shared/host_tools_cache.rs` 모듈:
- `load_from_db(db)` — 캐시에서 설치 도구만 반환
- `ensure_cached(db)` — 비었으면 1회 감지 + 저장
- `refresh(db)` — 강제 재감지 + 캐시 교체

`HostTool` enum 확장: `as_key()` / `from_key()` / `install_hint()` / `all()` 메서드 + `HostToolDetector::detect_full()` (음성 캐시 포함).

`CompositePreprocessor::with_tools()` 신설 — 외부 spawn 없이 host_tools 주입. `build_service`(shared)에서 DB에서 로드해 주입.

신규 Tauri command `refresh_host_tools`. Pipeline 탭의 preprocess 노드 인스펙터에 "🔄 새로고침" 버튼.

### 빌드 환경 변경
외부 디렉토리 `C:\dev\claude_workspaces\module` → `_rust_module` rename에 따라:
- `crates/adapters/Cargo.toml` 8개 path 의존성 갱신
- `crates/shared/Cargo.toml` 1개 path 갱신

### 테스트 결과
- workspace lib 테스트 302/302 통과 (core 137 + adapters 96 + shared 69)
- lesson 26 작성: settings.db schema 이중 정의(open vs open_in_memory) 동기화 누락 (lesson 10 재발 패턴)

## Phase 80 처리 (2026-05-07)

5축 사용자 입력의 한계(사용자가 자기 문서 비율을 모름)를 인식하고, 코퍼스 신호 + 동작 모듈로 추천 모델 재설계.

### 80-A/B/C/D: 코퍼스 신호 수집

settings.db 신규 테이블 3개:
- `search_mode_counters` (mode PK, count, last_at) — search 호출 시 자동 ++
- `crag_counters` (bucket PK: correct/ambiguous/incorrect) — CRAG 신뢰도 누적
- `chunk_stats` (key PK, value, last_at) — 향후 청크 측정 적재 (현재는 샘플링 산출)

McpState에 메모리 카운터 추가 + 서버 시작 시 `restore_counters()` 호출. 매 요청마다 메모리 ++ + DB INSERT/UPDATE 동시.

신규 MCP 도구 4개 (분리):
- `get_search_mode_stats` — mode 분포
- `get_crag_stats` — correct/ambiguous/incorrect 비율
- `get_chunk_stats` — vector_db.list_all() 50건 샘플링으로 평균 청크 크기·코드펜스·헤딩 비율 추정
- `get_processing_metrics` — vector_db.stats 기반 (verify/quarantine은 placeholder, service.summary 통합 후속)

### 80-E: 동작 모듈 12개

`setup_modules.toml` (가공 5 / 검색 4 / 운영 3):
- 가공: secure_strict / preprocess_rich / chunk_large / chunk_small / verify_strict
- 검색: search_precision / search_exploration / search_recent / rich_relations
- 운영: high_throughput / long_retention / auto_lint

각 모듈은 `[[module.changes]]` 배열로 path+value+reason+risk를 묶음. `exclusive_group`으로 배타 표현 (chunk_size / search_intent).

`setup_modules.rs` `ModuleRegistry::build_changes(ids, current)`:
- 배타 그룹 검증
- path별 그룹핑 → 충돌 시 보수적 해소 (boolean true 우선 / 숫자 max / array 합집합 / string 강한 도구)
- 동일 값이면 P0 우선
- 현재 값과 같으면 변경 미생성

신규 MCP 도구 2개:
- `setup_modules_list` — 12개 모듈 메타 반환
- `setup_apply_modules` — module_ids 배열 + apply_critical/dryrun. SetupAdvice 어댑팅 후 `apply_advice_full`로 위임

단위 테스트 6건 (parse / get / single / exclusive_violation / unknown / array_merge / combined).

### 80-F: MCP instructions 재작성

5축 가이드 폐기 → 패턴 분석 흐름:
1. 50파일+ 처리 후 신호 수집 (분리 도구 호출)
2. 신호 → 휴리스틱으로 모듈 추천
3. 사용자에게 패턴 + 추천 모듈 제시
4. setup_apply_modules (dryrun → apply)
5. 50파일 더 처리 → setup_snapshot_measure로 효과 검증

5축 도구는 legacy로 표시하되 호환 유지.

### 80-G: 5축 폼 보존

코드는 보존(setup_review/SetupProfile). UI 진입점에서만 "고급" 메뉴로 숨김.

### 80-1: 진입점 UI 3분기

`openSetupAssistant` 헤더에 3카드:
- ⚡ 일반 설정으로 시작 (변경 없음, 안내만 표시)
- 🤖 AI에게 분석 요청 (기존 MCP 안내 진입)
- 🧩 직접 동작 모듈 선택 (12개 체크박스, 그룹별)

신규 화면 `openSetupModules`:
- 가공/검색/운영 그룹별 체크박스
- 배타 그룹 자동 토글
- Critical 적용 동의 체크박스
- "미리보기 (dryrun)" + "적용" 두 버튼
- 변경 결과를 path/현재/추천 표로 표시

5축 폼은 우하단 "(고급) 5축 프로파일 폼 ▸"로 축소.



## Phase 65~78 처리 이력

본 영역(Phase 65~78, 2026-05-04 ~ 2026-05-07)은 2026-05-15(Phase 85)에 **[architecture-archive.md](architecture-archive.md)** 로 이관. 핵심 결과만 본 줄로 요약:

- Phase 78: `setup_dryrun` + `setup_profile_infer` MCP 도구 (사용 패턴 자동 프로파일링)
- Phase 77: `ConfigSnapshot` + 자동 롤백 4트리거 + `config_snapshots` 테이블
- Phase 76: 5축 SetupProfile + 선언적 룰 46건 (`setup_rules.toml`) + Critical 차단 + toml_edit 주석 보존
- Phase 75: AI 설정 도우미 MCP 안내 모달 (Claude Code 통합)
- Phase 73+74: `setup_review` 백엔드 + Dashboard 시나리오 모달
- Phase 72: 26 어댑터 단위 테스트 +25 (notify/format, stub, storage) — 2026-06-16 notification → notify 디렉토리 정정, lesson 77
- Phase 70+71: config 신규 4섹션 (`memory_tier`/`search`/`notification_batch`/`crossref.flush_interval_secs`) + 가공 노드 3건 (Quarantine/Memory Tier/Lint) + Settings 4그룹
- Phase 69: `configFields` 메타 + 검색 18→20노드 (kg_attach/entity_hl)
- Phase 68: 노드 시각 마커 (⚙) + 인스펙터 3영역 분리 (헤더/info/settings/auto)
- Phase 67: 가운데 4서브탭 폐기 + 인스펙터 480px + 가공 17→21노드 (Chunking/Todo/Topic merge)
- Phase 66: Phase 65 3계층 IA 부분 원복 + Pipeline 3컬럼 + 검색 17노드 신규
- Phase 65: 3계층 IA + fastembed 고정 + dead config 5건 제거

이 시기는 **추천 시스템(Phase 73~78)** + **IA 재설계(Phase 65~67)** + **UI 정합성(Phase 68~69)** 이 핵심 흐름. Phase 80 이후의 동작 모듈/코퍼스 카운터/Decision Log 작업이 이 기반 위에서 진행됨.

---

## Phase 64 이하 처리 이력

본 영역(트리거 #1/#11/#12 + release 빌드 + Phase 61~64)은 2026-05-15에 **[architecture-archive.md](architecture-archive.md)** 로 이관. 핵심 결과만 본 줄로 요약:

- 트리거 #11: onnx feature 완전 폐기 + vendor/onnxruntime 394MB 삭제
- 트리거 #12: bench p95 회귀 재측정 결과 회귀 없음 (23.62 docs/s, p95 48.3ms, +22%)
- 트리거 #1: similarity_threshold 디폴트 0.7→0.8 상향
- release 빌드: pipeline.exe 15.5MB / file-pipeline-tauri.exe 19.4MB
- Phase 64: dead Tauri commands 11 + dashboard.js API 9 삭제 + Phase 61 hierarchy UI 정합
- Phase 63: FastEmbedSparseAdapter + LocalVectorStore 통합은 트리거 #10 대기
- Phase 61: SemanticChunk.title_path + Metadata.hierarchy + dashboard breadcrumb
- Phase 62: fastembed BGE-M3 + Cross-Encoder, MRR 0.65→0.975, 64ms/건
---

# File Processing Pipeline — Architecture Spec

## 한 줄 요약

로컬 파일을 자동 분류·가공·압축·색인하여 검색 가능하게 만드는 Windows 데스크톱 파이프라인. (※ MCP 진입점은 2026-06-17 전체 폐기 — 외부 연계는 plugin(헥사고날 adaptor)으로 통일) LocalVectorStore(mmap+HNSW) + 교차참조 + 누적 업데이트 + 2-Pass 피드백 + 토픽 분할 + KG ego graph.

## 모달 진입점 매핑 (CLI / Tauri) — ~~MCP~~ 폐기 2026-06-17

두 진입점이 동일 core 서비스를 공유하지만 노출 명령 셋과 책임이 다름. (MCP 진입점은 전체 폐기 — mcp_server.rs 2139줄 + CLI Serve 삭제)

| 진입점 | 정의 위치 | 명령 수 | 책임 |
|--------|----------|---------|------|
| **modals/cli 바이너리** | `modals/cli/src/main.rs:34` enum Commands | 18개 | 풀 CLI: process/search/config/golden/bench/doctor/service 등 자동화·벤치마크 |
| **shared/cli (Tauri 진입)** | `crates/shared/src/cli.rs:30` enum Commands | 12개 | Tauri app(`pipeline.exe`) 비-GUI 명령 위임 (init/start/stats/batch 등) |
| **Tauri commands** | `modals/app/src/commands.rs` `#[tauri::command]` | 50개 | Dashboard 7탭 + 시스템 트레이 백엔드 |
| ~~**MCP 도구**~~ | ~~`crates/shared/src/mcp_server.rs` make_tool~~ | ~~0개~~ | ※ MCP 전체 폐기 2026-06-17 — mcp_server.rs 삭제 |

**진입점 경계**:
- modals/cli는 **별도 바이너리** (자동화·CI·벤치마크 전용)
- modals/app은 **Tauri GUI** + 비-GUI 명령은 shared/cli로 위임
- ~~MCP는 `pipeline serve` 또는 modals/cli `Serve`로 stdio 진입~~ (폐기 2026-06-17)

### CLI 서브커맨드 (modals/cli)

| 명령 | 서브커맨드 | 어댑터 의존 |
|------|----------|-----------|
| `Todo` | `List` / `Done {text}` | settings_db |
| `Kg` | `Neighbors {doc_id}` / `Paths {source, target}` / `Stats` | VectorDBPort |
| `Config` | (ConfigAction 서브커맨드) | settings_db + config |
| `Golden` | (GoldenAction — 검색 품질 골든셋) | LocalVectorStore |
| `Doctor {--save}` | JSON/콘솔 출력 | diagnostics + health_check |
| `Service` | (윈도우 서비스 install/start/stop) | daemon |

### Daemon 모드 (백그라운드 서비스)

| 모듈 | 역할 |
|------|------|
| `modals/cli/src/daemon/windows.rs` | Windows Service 래퍼 (Start/Stop/Install) |
| `modals/cli/src/daemon/unix.rs` | Unix daemon (호환용) |

`pipeline service install/start/stop` 진입 — 부팅 시 자동 실행 (옵션). `pipeline start --daemon`과 별개.

### Tauri commands 50개 카테고리별 분류

| 카테고리 | 개수 | 명령 |
|---------|------|------|
| 검색·문서 | 4 | search / get_document / list_documents / delete_document |
| 통계·진단 | 6 | get_stats / get_health / get_progress / get_errors / get_lint / get_token_usage |
| 큐·재처리 | 3 | get_queue / retry_failed / get_crossref_stats |
| 검증 | 1 | get_verification_metrics |
| 설정 | 4 | get_config / save_config / export_config_toml / import_config_toml |
| 크레덴셜 | 4 | list_credentials / save_credential / delete_credential / set_default_credential |
| DocType | 3 | list_doc_types / save_doc_type / delete_doc_type |
| 프롬프트 | 2 | get_prompts / save_prompts |
| 파이프라인 | 4 | get_pipeline / save_pipeline / simulate_pipeline / test_preprocess |
| 호스트 도구 | 2 | get_host_tools / test_host_tool |
| 워처 | 2 | get_watcher_status / set_watcher_active |
| 토픽 | 3 | list_topics / get_topic / update_topic |
| Todo | 3 | get_todos / add_todo / complete_todo |
| KG | 3 | kg_neighbors / kg_paths / kg_stats |
| 마이그레이션 | 3 | rebuild_all / rebuild_embeddings / rebuild_vectordb |
| Purge·Retention | 3 | purge_dry_run / purge_execute / get_retention_config |
| Lint·Backlinks | 1 | fix_backlinks |

### 모달 시스템 (UI 4종, G-6 후 — DocType 모달 dead-code 제거)

`Modal.open(title, bodyHtml, { onSave, wide })` 공통 팝업 패턴:
- **Credentials**: 크레덴셜 추가/수정 (프로바이더 + API key + 모델 + 프로필 경로)
- **Topics**: 토픽 페이지 편집 (마크다운 편집기 + LLM 수정 요청)
- **Todos**: 할일 추가/수정 (텍스트 + 기한 + 태그)
- **Prompts**: 프롬프트 외부화 편집 (TOML 핫 리로드)
- **Hooks**: 이벤트 훅 추가/수정 (event 5종 + webhook/command + enabled, Phase 84)
- ~~**DocType**: 문서 유형 추가/수정~~ → G-6에서 dead-code 제거 (2026-05-20). 모달은 dashboard.js에 있었으나 `pb-doctypes-table` 컨테이너가 HTML에 없어 도달 불가

### Pipeline 탭 인터랙티브 기능 (G-6/G-7 후 dead-code 제거)

- **시뮬레이션 사이드바** (`simulate_pipeline`): 텍스트 입력 → 실제 dry-run (LLM 호출 + 검증, DB 저장 없음). 노드별 pass/fail/skip 표시.
- **호스트 도구 테스트** (`test_host_tool`): pandoc/python-docx/openpyxl/libreoffice 개별 실행 가능 여부 확인.
- ~~**전처리 테스트** (`test_preprocess`)~~ → G-6/G-7 dead-code 제거 (frontend + backend 모두). 컨테이너 미정의로 도달 불가
- ~~**Purge dry-run/execute**~~ → G-6/G-7 dead-code 제거. backend `purge_dry_run`/`purge_execute` 함수까지 삭제
- ~~**문서 유형 관리 (DocType CRUD)**~~ → G-6/G-7 dead-code 제거. backend `list/save/delete_doc_type` 삭제

## 외부 워크스페이스: `module/` (재사용 라이브러리, Phase 60)

`C:\dev\claude_workspaces\module\`에 도메인 무관 재사용 모듈을 분리. 형제 프로젝트(약 20개)가 path dep로 가져갈 수 있는 인터페이스+구현체 9 크레이트 구조 (`*-api` + 구현체).

**확정 결정 (2026-04-28)**:
- Q1 LLM 어댑터: 통합 단일 크레이트 (Anthropic/OpenAI/Gemini/Ollama/Claude CLI/Fallback "무조건 같이 제공")
- Q2 Storage: 통합 단일 크레이트 (zstd 압축 + S3/WebDAV/Network "내부/외부 같이 제공")
- Q3 prompts/ChunkedAgent: 안 C 채택 — generic 템플릿 엔진 + 청크 오케스트레이터 둘 다 분리

| 크레이트 | 상태 | 역할 |
|---------|------|------|
| module-secrets-api / module-secrets | ✅ | `SecretStorage` trait + `KeyringSecretStore` (Windows Credential Manager) |
| module-storage-api / module-storage | ✅ | `LocalStoragePort` + `RemoteStoragePort` + `StorageError`(thiserror), zstd/S3/WebDAV/Network/Null 5종 raw |
| module-notify-api / module-notify | ✅ | `NotifyPort{send_text}` + `NotifyError`, Telegram/Slack/Composite/Null raw |
| module-llm-prompts | ✅ | `TemplateEngine` + `SectionSpec` generic — TOML 핫 리로드 + 변수 치환 + 섹션 인자화 |
| module-llm-api / module-llm | ✅ | `LlmRawPort{call_text(system, user, max_tokens)}` + `LlmError`, Anthropic/OpenAI/Gemini/Ollama/ClaudeCli/Fallback 6종 raw |
| module-llm-chunked | ✅ | `ChunkOrchestrator` + `Splitter` trait + `ByteSplitter`/`FnSplitter` — closure 주입으로 청크/병합 프롬프트 인자화. file-pipeline `chunked_agent.rs`는 LLMPort 위에서 inner 위임 + module의 ByteSplitter 재사용 |

**진행 상황**:
- ✅ 단계 0: workspace placeholder 일괄 생성 — `module/Cargo.toml` 9 멤버 등록 (lesson 16)
- ✅ 단계 1: module-storage 분리 — file-pipeline 5개 어댑터 thin wrapper. **lesson 17 작성** (6단계 의존 누수 점검)
- ✅ 단계 2: module-notify 분리 — raw send_text만, ProcessingSummary→text 포매팅은 file-pipeline `notify/format.rs` 분리 (2026-06-16 notification → notify 디렉토리 정정, lesson 77)
- ✅ 단계 3-1: module-llm-prompts 분리 — file-pipeline `prompts.rs` 한국어 콘텐츠 + 도메인 빌더만 잔류 (~210줄)
- ✅ 단계 3-3: module-llm 분리 — 5개 LLM 어댑터 thin wrapper, JSON 파싱+도메인 변환은 file-pipeline `llm/response.rs` 분리. (Q1 결정으로 3-2 앞에 진행)
- ✅ 단계 3-2: module-llm-chunked 분리 — generic Splitter + ChunkOrchestrator. file-pipeline `chunked_agent.rs`는 LLMPort 위에서 동작(인터페이스 유지) + module의 ByteSplitter 재사용으로 도메인 결합 회피
- ✅ 단계 4: 문서 갱신 + qdrant.exe(83MB) 삭제 + 형제 시뮬레이션 통과 (10 module + tokio 직접 의존, 의존 트리 깨끗)

**dead dep 정리** (Q3 정책): adapters Cargo.toml에서 zstd/sha2/hex/toml direct dep 제거 — module로 이관됨. bench_scale.rs:79 빈 fixture 0 나누기 사전 결함 동시 수정.

**누수 점검** (lesson 17): 단계마다 `cargo tree -p module-{name} | grep file_pipeline_` 0건 + `grep -rn file_pipeline module-*/src` 0건 확인 완료. 형제 시뮬레이션은 단계 4에서 1회 실행.

file-pipeline 어댑터는 thin Anti-Corruption Layer로 module 인터페이스를 호출. 도메인 결합 0 유지. file-pipeline 한국어 프롬프트 콘텐츠 + ProcessingSummary 포매팅 + 5종 LLMPort 도메인 메서드는 file-pipeline 잔류.

**Plan 파일**: `C:\dev\ide\claude\profiles\reujea\plans\q1-ethereal-cocke.md` (안 C 반영, 구 `immutable-humming-deer.md` 폐기)

## 수치

| 지표 | 값 |
|------|-----|
| .rs 파일 | **148개** (target 제외, 2026-05-14 실측. Phase 60~83 누적: module 분리 + fastembed + sparse + sparse + setup_review/modules/dryrun/snapshot/decision_log/wikilink + processing_metrics) |
| 코드 | **~17.2만 줄** (테스트 포함, target 제외 wc -l) |
| 테스트 (workspace lib) | **349개** (core 152 + adapters 102 + shared 95, Phase 90 Notion 단위 테스트 +6) |
| 통합 테스트 | 69 통과 (scenarios 10 + actor 4 + e2e 21 + notification 9 + search 12 + real_env 13) — 벤치 제외 |
| 통합 테스트 사전 결함 | scale_validation::scale_work_queue_10k/100k (FAILED, lesson 28 추적 대상, FileProcessingService 미관련) |
| CLI | 18개 커맨드 (+process/search/config/golden/bench) |
| Dashboard 탭 | 7개 (Documents, Processing, Todos, Verification, Topics, Pipeline, Settings) |
| 테스트 러너 | cargo test (nextest 미설치 시 폴백) |
| CLI commands (modals/cli) | 18개 (start/init/show-config/stats/doctor/process/search/config/golden/bench/memo/export/todo/kg/backfill-vec/topic-revise/serve/service) |
| CLI commands (shared, Tauri 진입) | 12개 (start/init/show-config/stats/export/memo/topic-revise/todo/kg/backfill-vec/batch/serve) — Tauri app이 비-GUI 명령 위임 시 사용. Doctor/Process/Search/Config/Golden/Bench/Service는 modals/cli 전용 |
| 앱 | 단일 바이너리: CLI(pipeline) + GUI(Tauri). ~~MCP는 `serve` 커맨드로 통합~~ (MCP 전체 폐기 2026-06-17) |
| 포트 | 없음 (Tauri WebView, 포트 바인딩 불필요) |
| Tauri commands | **65개** (Phase 89~95 누적. Phase 97 audit_stage 자동화 무관, Phase 102 optimize 추가 0건). **현행 수치는 `prd/roadmap.md` 단일 진실원 위임 (메타 룰 19)** |
| MCP | **0개 — 전체 폐기 2026-06-17** (mcp_server.rs 2139줄 + 37 도구 + CLI Serve + Tauri command + webapp 카드 모두 삭제. 본질 재정의 3차로 외부 연계는 plugin(헥사고날 adaptor)으로 통일) |
| REST | 제거됨 (rest_server.rs 삭제) |
| 벡터 DB | **LocalVectorStore** (인프로세스, Blue-Green 슬롯 + mmap + Rayon + HNSW). Qdrant 제거됨 |
| 기본 dim | 128 (Claude CLI 의미축 최적) |
| 표준 파이프라인 | 1개 (단일 구조, 배열 제거됨) |
| SqliteVec | `with_path()` 격리 + HNSW(instant-distance) + 키워드 역색인 + batch_begin/end |
| 임베딩 | **fastembed BGE-M3(1024차원, 순수 Rust, MRR 0.975, 64ms/건)** / OpenAI / Claude CLI(128축) / Local / Python ONNX (legacy, 트리거 #11에서 Rust ort 폐기) |
| 문서 유형 | 17개 검증 스키마 (doc_types.toml — 섹션+임계값만, LLM 자율 판단) |
| 포트 | 11개 (+RerankerPort, +RemoteStoragePort, EmbeddingPort+ColBERT) — Qdrant+GraphDB 제거됨, LocalVectorStore 단일 |
| 설정 섹션 | 19개 (config struct: compression, vector_db, embedding, notification, verification, models, llm, llm_credential, preprocessing, sensitive, logging, schedule, paths, remote_storage, rerank, chunking, crossref, resolved_paths, pipeline) — DashboardConfig 제거됨 |
| Settings 그룹 | 5개 (크레덴셜 관리, 일반, 스케줄·경로, 인프라, 마이그레이션) — 처리설정 Pipeline 이관 |
| Pipeline 레이아웃 | 2컬럼 (사이드바: 시뮬레이션+로그 / 메인: 서브탭 4개 + 축소 플로우) |
| Pipeline 서브탭 | 4개 (데이터 가공, 외부 저장소, 청킹, 보존 & Purge) |
| Pipeline 노드 (가공) | **23개** (Phase 90 GUI 검증 실측, Playwright). 사전검사 + 스텝 + 후처리 + Quarantine 분기 + Memory Tier + Lint (Phase 70~71/84 누적). 이전 표기 18개 → 21개 → 23개 |
| Pipeline 노드 (검색) | **20개** (Phase 90 GUI 검증 실측). Phase 66 신규 17노드 → 18 → 20 (Phase 84 Dense/Hybrid/Filtered trace + 시뮬레이션 단계 추가) |
| 검색 MRR@5 | 0.525 (HashEmbedder), ~0.65 (Claude CLI), **0.975** (fastembed BGE-M3, 실측) |
| 검색 P@3 | 0.67 (유형별 평균, HashEmbedder 단위테스트) |
| 검색 테스트 | 13 시나리오 (랭킹/필터/하이브리드/~~MCP~~/MRR/엣지케이스/P@k/교차참조품질) — ※ MCP 시나리오는 폐기 2026-06-17 |
| dashboard.js | ~2,805줄 (Phase 64 dead 정리 -130줄) |
| 외부 연동 | Claude CLI, OpenAI, Telegram, Slack |
| DB 등록률 | 100% |
| HNSW 검색 | 0.57ms (3000문서) |
| 양자화 | Int8 Scalar (RAM 75% 절감) |
| 검색 모드 | Dense + Sparse(BM25 키워드+search_hints+summary) RRF + 시간 가중(10% boost) + Re-ranking(선택). SearchMode: default/exact/related/recent/fusion |
| 리랭킹 | **FastEmbedReranker (BGE-Reranker-v2-M3 Cross-Encoder 로컬, 권장)** / ClaudeReranker (Claude CLI LLM 호출, fallback) / NullReranker (비활성) |
| 외부 저장소 | Network(SMB/NFS) / WebDAV / S3 / **Notion (Phase 90, page 모드)** — NullRemoteStorage(비활성 기본) |
| 엔티티 추출 | LLM 프롬프트 우선 (person/org/tech/amount/project) + regex 폴백 |
| 가공본 저장 | 순수 본문만 (META 헤더 제거, 메타데이터는 벡터DB 전용) |
| 전처리기 | 호스트 도구 자동 감지 (pandoc/python-docx/openpyxl/libreoffice) + Rust 네이티브 DOCX(zip)/XLSX(calamine) 폴백 + **인코딩 자동 감지(chardetng+encoding_rs)** |
| Claude CLI 임베딩 | 128축 의미 벡터 (키워드 해시 fallback) |
| Claude CLI 호출 | **stdin 파이프** (Windows 32KB 명령줄 제한 회피, Phase 54) |
| 의미 중복 정책 | **Keep** (비대화형: 둘 다 유지, CLI 터미널: 사용자 선택) |
| ~~MCP 로깅~~ | ~~`[mcp-usage]` search/get_document 호출 로깅~~ (MCP 전체 폐기 2026-06-17) |
| auto-init | pipeline.toml + inbox/processed/originals/logs 등 **자동 생성** (파일/디렉토리 없으면 첫 실행 시) |
| 모달 시스템 | `Modal.open(title, bodyHtml, { onSave })` 공통 팝업. **전체 전환 완료**: Credentials, DocType, Topics, Todos, Prompts |
| 교차참조 모드 | auto (키워드/임베딩 기반, LLM 0건) / llm (기존) / off |
| 교차참조 방식 | threshold 기반 전체 문서 스캔 (top_k 제거). 유형별 cap: Supersedes 2 / Updates 5 / RelatedTopic 20 / References 10. **양방향**: References↔ReferencedBy 추가 |
| 교차참조 최적화 | HashSet O(1) 중복체크 + keywords 스냅샷(Mutex 제거) + mmap+Rayon + moka + 비동기배치 + 증분스킵 + 조기종료 + refresh_mmap 배치 스킵 + compile_state 배치 스킵 + **EmbeddingSnapshot 행렬곱 flush** |
| 교차참조 비동기 | crossref_queue → flush_crossref (EmbeddingSnapshot 1회 로드 + 인라인 cosine). 30초 유휴 시 자동 실행 |
| 교차참조 관계 유형 | 5종: Supersedes, Updates, RelatedTopic, References, **ReferencedBy** (Phase 47 추가) |
| 마이그레이션 | 임베딩 재생성, 벡터DB 재구축, 전체 재가공 (Settings UI) |
| 피드백 | 소스 모드 전용 (우클릭 → claude -p → UI 수정) |
| CLI batch | pipeline.exe batch (GUI 없이 inbox 배치 가공) |
| Todo 시스템 | settings.db todos 테이블 (14컬럼+5인덱스). doc_ids JSON 배열 (중복 업무 합산). 키워드 7종+체크박스 자동 추출. CLI: list/done/skip/reopen/add |
| LLMPort | summarize_text (merge_todo에서 rename). classify_and_process + reprocess_with_feedback + enrich_existing — **포트 전체 매핑은 `spec/domain-map.md` 단일 진실원 (메타 룰 19)** |
| CLI doctor | `pipeline doctor [--json]` — CorpusStats + health_check + incoming degree top-10. JSON/터미널 듀얼 출력 |
| 진단 | CorpusStats(doc_count, relations, histogram, hub_top10, **incoming_top10**, isolated) + HealthIssue(고립/허브편중/비대칭/incoming폭증) |
| 설정 DB | **SettingsDb** (SQLite WAL). pipeline.toml + doc_types.toml + prompts.toml → settings.db 단일 파일. TOML 자동 마이그레이션 + .bak 백업 |
| 교차참조 cap | **TypedSlots**: cap_supersedes/updates/related/references 설정 기반. **mutual top-K**: cap_incoming으로 incoming 폭증 방지 |
| 교차참조 사전필터 | **MinHash LSH** (minhash_force_enable / minhash_min_docs=3K) + **메타데이터 블로킹** (metadata_blocking, doc_type 또는 키워드 1개 이상 겹쳐야 비교). 둘 다 기본 비활성, flush_crossref에서 후보 축소용 |
| 증분 flush | **incremental_flush**: 동적 임계치(50/200/500/1K) + flushed_embeddings 3소스 검색. **db_refresh**: pending 0 시 Blue-Green swap. Atomic 카운터(lock-free upsert). flush 77% 개선(19.6→4.5초) |

---

## 헥사고날 아키텍처 (위반 0건 검증됨)

```
외부(Driving)                코어                           외부(Driven)
──────────────────    ────────────────────────────    ──────────────────────────
notify::Watcher   →   FileProcessingService       →   LLMPort (Claude CLI)
[MCP stdio 폐기]  →   (McpState 삭제 2026-06-17)   →   StoragePort (zstd)
터미널 stdin      →   TerminalResolution/Sensitive →   VectorDBPort (LocalVectorStore)
                      CrossRefUpdater              →   EmbeddingPort (OpenAI/Local)
                      TopicMerger (클러스터링)      →   NotificationPort (Telegram/Slack)
                      Linter (orphan/stale/모순)   →   VerificationPort (Claude CLI)
                      WikiExporter (_graph.json)   →   PreprocessPort (PDF/OCR/Plain)
                      AutoReindexer / CompileState  →   GraphDBPort (JSON / Neo4j)
```

core → adapters/shared/modals 참조 **0건**. 포트 trait만 참조.

## 처리 플로우 (14단계)

```
inbox 투입 (notify)
 0. 스킵 확장자 → 파이프라인 매칭 (glob 패턴, priority 순)
 1. 민감 판별 → 2. Fragment 감지 → 3. SHA-256 중복 → 4. 증분 해시
 ── 파이프라인 스텝 순회 ──
 5. [Preprocess] PDF/OCR 전처리 → 텍스트 변환
 5.5 [Chunking] 대용량 → 의미 단위 분할 (헤딩/코드펜스/오버랩) 또는 40KB 바이트
 6. [LLM] classify_and_process (노이즈 제거+search_hints+code_blocks+standalone_context)
 7. [Verify] 6가지 검증 → 2-Pass → quarantine
 8. [Embedding] 임베딩 (모델 오버라이드)
 9. [Storage] zstd 압축 (레벨 오버라이드)
 ── 공통 후처리 ──
 10. 의미 중복 → 11. .vec 파일 영속화 → 12. Todo 병합
 13. LocalVectorStore 색인 (HNSW 캐시 + 키워드 역색인)
 14. CrossRef (양방향 링크 + LLM 보강) → 15. 증분 기록 → 자동 토픽 병합
```

## 검증 시스템

| 검사 | 방법 | 기준 |
|------|------|------|
| 구조 완전성 | sections JSON 키 | 50% |
| 압축률 | 가공/원본 비율 | 5~150% |
| 키워드 커버리지 | LLM→원본 | 50% |
| 키워드 완전성 | 원본→가공본 | 30% |
| ROUGE-L | LCS | 10% |
| 개체 보존 | 날짜/금액/숫자/이메일/URL | 50% |

적응적: doc_types.toml 유형별 오버라이드 (검증 스키마). 2-Pass: FAIL→피드백 재가공→quarantine.
실측 (신규 프롬프트, 3문서): 구조 100%, 키워드 97.7%, ROUGE-L 65.7%, 개체 83.3% → 전체 1-Pass 통과.

## 토픽 분할

유형별 디렉토리 → 임베딩 agglomerative 클러스터링(max 20) → LLM 라벨링 → 시간 분할(분기별) → 2단계 요약 → 커버리지 태그 → 모순 해결 프롬프트. 자동 병합(5개+ 트리거). 수정(이력 누적 + .bak 3단계).

## CLI 11개 (~~/ MCP 11도구~~ 폐기 2026-06-17)

```
start / init / show-config / stats / doctor / memo / export / todo / kg / backfill-vec / topic-revise
```
(※ `serve`(MCP stdio) 커맨드 삭제 2026-06-17)

```
search / get_document / list_documents / stats / lint / revise_topic
kg_neighbors / kg_paths / kg_stats / list_todos / complete_todo
```

## 외부 연동 설정

```
pipeline.toml:
  [vector_db]      — LocalVectorStore (인프로세스)
  [embedding]      — OpenAI(env: OPENAI_API_KEY) / Claude CLI(자동감지) / Local
  [notification]   — Telegram(bot_token+chat_id) / Slack(bot_token+channel)
  [preprocessing]  — 호스트 도구 자동 감지 (pandoc/python-docx/openpyxl/libreoffice), 기본값 "none"
  [verification]   — Claude CLI (enabled/llm_hallucination_check)
  [dashboard]      — port / user / pass / cors_origin
  [models]         — classify/process/verify_model
  [llm]            — provider(claude_cli/anthropic_api) / anthropic_api_key
```

모든 설정: 환경변수 → pipeline.toml fallback 순서.
Settings UI → "Settings UI" 섹션 참조.

## Pipeline Builder (Ansible Tower 스타일)

파일 유형별 커스텀 처리 경로를 시각적으로 구성. Dashboard 독립 "Pipeline" 탭.

**데이터 모델**: `PipelineDefinition` (steps: Vec<PipelineStep>). 5종 스텝: Preprocess, Llm, Verify, Embedding, Storage. `pipeline.toml` [pipelines] 단일 구조로 영속화. name/pattern/priority/enabled 필드 제거됨.

**전처리기**: HostToolDetector가 호스트 도구를 자동 감지 (pandoc/python-docx/openpyxl/libreoffice). 미설치 시 기본값 "none". 전처리 실패 → 텍스트 직접 읽기 시도 → 바이너리 파일이면 가공 중단.

**파이프라인**: 단일 파이프라인으로 모든 파일 처리. match_pipeline/matches_glob 삭제. list/delete/reorder 커맨드 삭제 → get/save 2개로 교체.

**GUI (2컬럼 레이아웃)**:
- **좌측 사이드바 (320px)**: 시뮬레이션 (텍스트/파일 입력 → 노드별 pass/fail/skip) + 로그 출력
- **우측 메인**: 서브탭 4개 (데이터 가공 / 외부 저장소 / 청킹 / 보존 & Purge) + 하단 축소 파이프라인 플로우

**노드 편집**: 버튼 기반 ([+추가], [×삭제], [▲▼순서]). 노드 체크박스로 활성/비활성 토글.

기존 Pipelines 탭(수평 카드 플로우) + Settings > 파이프라인 흐름 → 독립 Pipeline 탭으로 통합. ~795줄 제거, ~450줄 추가.

**Tauri commands**: get_pipeline, save_pipeline (2개).

**서비스**: `process_file_with_pipeline()` — 파이프라인 스텝을 순회하며 5종 오버라이드 실행:
- **Preprocess**: `preprocess_with_config(pdf_tool, ocr_tool)` — 파이프라인별 전처리기 설정 적용
- **LLM**: `credential_llms: HashMap<String, Arc<dyn LLMPort>>` — watcher에서 미리 빌드, 파이프라인 매칭 시 주입
- **Verify**: 임계값 오버라이드 (`VerificationThresholds`)
- **Embedding**: `embed_with_model(text, model)` — 모델 오버라이드 (기본 구현: model 무시)
- **Storage**: `compress_with_level(source, dest, level)` — zstd 레벨 오버라이드

공통 전처리(민감/중복/증분)와 후처리(임베딩/색인/CrossRef)는 항상 실행.

**LLMPort 확장**: `classify_and_process_text(file_name, text, registry)` — 전처리된 텍스트를 직접 LLM에 전달. 기본 구현은 임시 파일 작성 후 기존 메서드 위임.

## 크레덴셜 시스템

`LlmCredential`(pipeline.toml [[llm.credentials]])로 프로바이더별 API 키/모델을 관리.
- 필드: **id** (UUID), provider, label, api_key, base_url, model, `profile_path` (Claude CLI CLAUDE_CONFIG_DIR)
- `ClaudeCliAdapter`에 `config_dir` 옵션 + `with_config_dir()` 빌더
- `build_llm_from_credential()` 유틸 함수 (shared/lib.rs) — 크레덴셜 → LLM 어댑터 인스턴스
- `LlmConfig`에 `default_credential` 필드 — 역할 미지정 시 사용할 기본 크레덴셜
- Settings UI: 크레덴셜 행에 "수정" 버튼 (기존 값 채워진 폼) + "기본으로 설정" 버튼 + DEFAULT 배지
- save_credential: id 기반 upsert (신규=생성, 기존=수정)
- 폼 초기화: 탭/네비 전환 시 hideCredentialForm 호출
- 역할별 연동: 분류/가공/검증/기본임베딩/민감임베딩 각각 [크레덴셜|모델] 쌍 선택
- **파이프라인 스텝별 크레덴셜**: PipelineStep(Llm/Verify/Embedding) 각각 `credential: Option<String>` 필드
- **후처리 크레덴셜**: PipelineDefinition.postprocess_credential — Todo 병합, 교차참조, 토픽 병합용
- **fallback**: 스텝별 credential 미지정 → default_credential → 글로벌 provider
- **resolve_pipeline_llms()**: 파이프라인에서 역할별(classify/verify/embed/postprocess) LLM 맵 생성

## Processing 탭

Dashboard "Processing" 탭. 파일 가공 현황 실시간 모니터링.
- Queue 카드: 처리중/완료/실패/대기 카운트 표시
- Row 클릭 시 progress 이벤트 + error 로그 패널 표시
- 자동 갱신 (5초 주기 polling)
- 상태별 색상: 완료=녹색, 실패=빨강, 처리중=파랑, 대기=회색
- 상태별 정렬: 처리중 > 대기 > 실패 > 완료
- "실패 항목 재처리" 버튼: `retry_failed` 커맨드 → WorkQueue.retry_all_failed() → Failed→Pending 리셋

## Settings UI

VSCode 스타일 (좌측 네비 한국어 + 검색 + 섹션별 표시). Tauri commands get_config/save_config/export_config_toml/import_config_toml.
- **5그룹**: 크레덴셜 관리 / 일반(로깅) / 스케줄·경로(스케줄·경로·동시성) / 인프라(리랭킹) / 마이그레이션 — 처리설정(청킹·교차참조)은 Pipeline 서브탭으로 이관
- **멀티컬럼**: 모든 그룹이 2~3col 그리드 (`system-3col`). 섹션 접기/펼치기 (section-toggle)
- **크레덴셜 카드 UI**: 프로바이더 아이콘 + 이름 + 모델 + API 키 + 기본/수정/삭제 버튼. `cred-card-grid`
- **경로 설정**: 기본 inbox readonly 표시 + extra_inboxes 테이블 UI (추가/삭제)
- **활성화 체크박스**: 각 섹션 내 enabled/활성 필드를 맨 위로 자동 정렬
- 검색: 매칭 필드 하이라이트 (`search-highlight`), 결과 없음 메시지
- 비밀번호 필드: eye toggle (👁 show/hide)
- 초기화: `confirm()` 확인 모달
- **import/export**: footer에 "pipeline.toml 가져오기/내보내기" 버튼
- **save_config 시크릿 보존**: `restore_masked_secrets()` — 마스킹된 "****" 값을 기존 값으로 복원. credentials는 항상 기존 목록 보존
- **제거됨**: DashboardConfig, notification UI, retention_days UI, graph_db UI
- **외부저장소/Fragment/청킹/교차참조**: Pipeline 서브탭으로 이관
- UI 단일 진입점: `ui/index.html` (dashboard.html 삭제됨)

## 알림 시스템

건별 알림(민감/quarantine/완료/중복) + **배치 요약 알림**(30초 유휴 시 flush).
ProcessingSummary: 성공/처리중/에러/스킵/민감/중복/격리 카운트 + 이슈 상세(파일명+사유+대안).
Telegram(HTML) / Slack(mrkdwn) / Composite(멀티채널) / Null(비활성).

## 비가공 파일 스킵

watcher.rs 5단계: 임시파일(.tmp) → config(.env/.ini) → 소스코드(.rs/.py/.js 등 24종) → 바이너리(.exe/.zip/.mp3 등) → 특정파일명(Cargo.toml, pipeline.toml 등).

## Todo 생명주기

7단계 Todo 병합 + 7b 이월: 체크박스 파싱(`[ ]`/`[x]`) → 어제 미완료 항목 자동 carry-forward.
todo_lifecycle.rs: parse_todos, carry_forward, mark_completed, status_summary.

## CLI 구조 (17개 커맨드, ~~serve(MCP)~~ 삭제 2026-06-17)

```
pipeline start [--port]  — 메인 실행 (watch+batch+dashboard+트레이+주기lint/purge+topic-merge)
pipeline init / show-config / stats / memo / export / todo / topic-revise / backfill-vec
pipeline doctor [--json]  — 코퍼스 진단 (health check + incoming degree)
pipeline kg neighbors/paths/stats  — 지식 그래프 CLI
[pipeline serve — MCP 서버 모드 삭제 2026-06-17]
```

제거: watch, dashboard, purge, lint, topic-merge, backfill-sparse, service, batch (모두 start로 통합 또는 미구현)

## 시스템 트레이

Tauri TrayIconBuilder. 앱 아이콘 통일 (윈도우/트레이/작업표시줄).
메뉴 5개: Dashboard 열기 / 통계 보기 / 감지 ON·OFF / 구분선 / 종료.
좌클릭: 앱 창 토글. 창 닫기(X): 트레이로 최소화.
**CREATE_NO_WINDOW**: 전체 Command::new 호출에 적용 (preprocessor 10곳, hooks, diagnostics, python_onnx_embed).

## 모달 아키텍처 (CLI / APP 분리, ~~MCP~~ 폐기 2026-06-17)

modals/ 디렉토리에 사용자 인터페이스별 독립 크레이트:
- **modals/cli/** — CLI 바이너리 (pipeline.exe). clap 서브커맨드, daemon 관리.
- **modals/app/** — Tauri 2.0 GUI. Dashboard WebView + 시스템 트레이 + 백그라운드 서비스.
- ~~MCP 서버는 CLI `pipeline serve`로 통합~~ (MCP 전체 폐기 2026-06-17 — mcp_server.rs 2139줄 + CLI Serve 삭제)

공유 로직은 crates/shared/ (config, build_service, cli). (~~mcp_server~~ 삭제 2026-06-17)
REST 서버 완전 제거 (rest_server.rs 삭제) → APP은 50개 Tauri commands로 직접 Rust 호출.
UI 단일 진입점: `ui/index.html` (dashboard.html 삭제됨).
단일 바이너리: APP(Tauri)에 CLI가 인자 분기로 통합 (~~+MCP~~ 폐기 2026-06-17. Tauri 19.4MB / CLI 15.5MB, 2026-04-30 release 실측).
첫 실행: auto_init() → pipeline.toml + inbox/processed/originals/logs 자동 생성 + 안내.
GUI 시작: `#![windows_subsystem = "windows"]`로 cmd 창 미표시 → Dashboard 표시.
벡터DB: LocalVectorStore 인프로세스 (mmap+Rayon+HNSW 캐시). 외부 서버 불필요.
트레이 종료: app_handle.exit(0) → 프로세스 완전 종료. 창 닫기 = 숨기기.
빌드 최적화 (APP): strip + LTO + codegen-units=1 + opt-level=s.

## 프로젝트 디렉토리 구조

```
file-pipeline/
├── doc/           — 사용자 가이드 (deployment-guide, architecture-diagrams)
├── handsoff/      — 요구사항 원문 (읽기 전용)
├── prd/           — 기획 문서 (roadmap, features, research)
├── spec/          — 세션 컨텍스트 (architecture, lesson-learned)
└── src/           — 모든 구현 산출물
    ├── Cargo.toml / Cargo.lock
    ├── crates/    — 공유 라이브러리
    │   ├── core/      도메인 모델, 서비스, 포트 trait (output 9 + input 2)
    │   ├── adapters/  어댑터 구현체 (driven/driving)
    │   └── shared/    cli, config, credential_store, platform, settings_db, tray (~~mcp_server~~ 삭제 2026-06-17)
    ├── modals/    — 사용자 인터페이스 (모달별 독립)
    │   ├── cli/       CLI 바이너리 + daemon + 테스트
    │   └── app/       Tauri GUI (Dashboard + 트레이) (~~MCP는 cli `serve` 통합~~ 폐기 2026-06-17)
    ├── ui/        — 정적 프론트엔드 (index.html/css/js, 단일 진입점)
    ├── vendor/    — 외부 바이너리 디렉토리 (Phase 64 트리거 #11에서 onnxruntime 394MB 삭제, 현재 비어있음)
    ├── pipeline.toml / doc_types.toml
    └── .config/nextest.toml
```

## LLM 프로바이더 (5종 + Fallback)

claude_cli / anthropic_api / openai_api / ollama / gemini 선택.
prompts.rs 공유 모듈, FallbackLlmAdapter로 복수 프로바이더 순차 시도.
pipeline.toml `[llm]` provider + fallback_providers 설정.
프롬프트 외부화: prompts.toml 외부 파일 로드 + RwLock 핫 리로드. Settings UI에서 편집 → 즉시 반영 (재시작 불필요).

## 파일 병렬 처리

watcher에 Semaphore 기반 동시성 제어. max_workers(기본 4) 설정.
tokio::spawn은 유지, semaphore.acquire()로 동시 처리 수 제한.

## 헬스체크

/api/health: LocalVectorStore 상태 + 디스크 여유 + LLM 프로바이더 이름.

## 검증 메트릭

VerificationMetricEntry: 6가지 메트릭 + overall + timestamp.
/api/verification/metrics: 최근 50건 + pass/fail/warning 카운트.
Dashboard Verification 탭.

## 메모리 계층화 (Hot/Warm/Cold)

MemoryTierConfig: hot(7일)/warm(30일)/cold(90일) 기준.
Metadata에 tier/last_accessed/access_count 필드.
memory_tier.rs: compute_tier(), days_since().

## 대용량 파일 에이전트

ChunkedAgentAdapter: LLMPort 래핑 (Decorator 패턴).
>40KB 파일 → chunking.rs로 40KB 단위 분할 → 각 청크 에이전트 가공 → 병합 에이전트 통합.
≤40KB 파일은 기존 LLM에 직접 위임 (변경 없음).
PDF 처리: read_to_string 실패 시 내부 LLM에 직접 위임. ClaudeCliAdapter: PDF(바이너리) 감지 → 파일 경로를 프롬프트에 포함.

## 작업 큐 매니저

WorkQueue (.work-queue.json 영속화):
- scan_and_plan(): inbox 스캔 → 캐시 비교 → BatchPlan 생성
- 상태: Pending → Processing → Done / Modified / Deleted / Failed
- 배치 분류: 소형(≤40KB) vs 대형(>40KB)
- 변경 감지: 해시 비교 → 자동 재처리
- CLI: `pipeline batch` — 큐 기반 배치 처리

## 하드코딩 제거

벡터 dim(1536→config), lint stale(90→config), zstd level(3→config), 헤더 읽기 통일.
McpState에 lint_stale_days 필드 추가. 모든 매직 넘버를 pipeline.toml로 이관.

## 문서

doc/architecture-diagrams.md: 5개 다이어그램 + 영역별 고도화 10개.

## 벤치마크

| 규모 | stub 처리량 (교차참조 off) | stub 처리량 (교차참조 auto+batch) | 검색 avg | 검색 p95 |
|------|--------------------------|--------------------------------|---------|---------|
| 100 | 133 docs/s | **17.2 docs/s** (전체배치, 3회 중앙값) | **0.15ms** | **0.17ms** |
| 1,000 | 92 docs/s | **9.5 docs/s** (105초, 59K 관계) | **1.38ms** | **1.71ms** |
| 5,000 | 68 docs/s | 미측정 | — | — |
| **실 코퍼스 325** (D:\file-test\samples, .html/.htm/.txt/.docx/.pptx/.pdf) | — | **26.0 docs/s** (3회 중앙값, p95 50ms) | **0.13ms** | **0.27ms** |

> 실 코퍼스 측정 (2026-05-14): HashEmbedder + RealCorpusLlm(키워드 추출). bench_real_corpus_1000 환경변수 BENCH_CORPUS_DIR 지원. 트리거 #2/#4 실측 의사결정은 본문 상단 "실 코퍼스 측정" 섹션 참조.

> 순차 최적화 완료 (Phase 46): 289초→83초 (3.5x), per-doc 318ms→60ms (5.3x). 병렬화는 stub LLM에서 효과 없음 (Mutex > 가공).

### per-doc 오버헤드 (최적화 전후)

| 지표 | 최적화 전 | 최적화 후 | 개선 |
|------|----------|----------|------|
| avg | 318ms | **47ms** | **6.8x** |
| p50 | 46ms | 43ms | — |
| p95 | 2,075ms | **69ms** | **30x** |
| max | 2,472ms | 180ms | **14x** |
| 분산 (p95/p50) | 45x | **1.6x** | 평탄화 |

### 실제 문서 벤치마크 (2026-04-18~20)

| 코퍼스 | 문서 수 | 처리량 | 소요 | 검색 avg | 관계 | 비고 |
|--------|--------|--------|------|---------|------|------|
| K8s+OpenStack | 1,312 | 28.0 | 47초 | 1.12ms | 40K | flush 미포함 |
| 7유형 (초기) | 601 | 2.4 | 289초 | 4.02ms | 22K | Phase 0 baseline |
| 7유형 (mmap배치) | 601 | 9.5 | 105초 | 1.38ms | 25K | Phase 45: refresh_mmap 스킵 |
| 7유형 (전체배치, 추정) | 601 | ~8.4 | ~83초 | — | 25K | Phase 46: +compile_state 스킵 |
| stub 2000 (Blue-Green) | 2,000 | **16.0** | **124.8초** | — | 108K | Phase 48: Blue-Green + 행렬곱 flush |
| stub 100 (Phase 52) | 100 | **23.6** | **4.2초** | 0.06ms | 5,650 | Phase 52 final: p95=54ms |
| stub 2000 (Phase 52) | 2,000 | **20.5** | **97.7초** | 0.21ms | 108,240 | Phase 52 final: flush 3.0초, p95=69ms |
| stub 100 (Phase 54) | 100 | **16.6** | **6.0초** | 0.08ms | 5,650 | Phase 54: Stub Keep, 3회 중앙값. 등록률 100% |
| 실문서 20개 (Phase 54) | 20 | 0.4 | **992초** | 0.37ms | 60 | Claude CLI, 전처리+stdin. 성공 20/20, DB 20/20 |

### 프롬프트 비교 벤치마크 (2026-04-14)

| 지표 | 기존 (RealClaudeLlm) | 신규 (ClaudeCliAdapter) |
|------|---------------------|----------------------|
| 속도 | 41.2초/파일 | **38.6초/파일 (-6%)** |
| 구조 완전성 | ~60% | **100%** |
| ROUGE-L | ~20% | **65.7%** |
| 키워드 | 5개 | **10~15개** |
| 1-Pass 통과율 | ~80% | **100%** |
| search_hints | 없음 | **3~5개/문서** |
| code_blocks | 없음 | **구조화 (language+description)** |

### 벤치마크 스냅샷 (Phase 49)

벤치마크 결과를 `BenchmarkSnapshot` JSON으로 `spec/benchmarks/`에 자동 저장.

**구조**: `BenchmarkSnapshot` (diagnostics.rs)
- `throughput`: total/process/batch_end/flush 분리 타이밍
- `per_doc`: avg/p50/p95/max/variance_ratio
- `search`: avg/p95/queries
- `crossref`: relation_count/unique_pairs/double_ratio/isolated
- `storage`: input/processed/originals/compression
- `corpus`: CorpusStats (선택)

**CI 회귀 감지**: `bench_regression_check` (bench_scale.rs)
- `scale_100` 이전 스냅샷 자동 로드 → 비교
- per-doc p95 ≤ 100ms (절대)
- flush ≤ 30초 (절대)
- throughput 20% 이상 하락 금지 (상대)

**스냅샷 파일명**: `{label}_{YYYYMMDD_HHMMSS}.json` (시간순 정렬)

## MyDocSearch 비교 결론 (2026-04-08 확정 → 2026-06-04 무효화 → 2026-06-05 spec 삭제)

**원래 결정**: 통합 불필요, LocalVectorStore 단일 구조 유지 (Qdrant Phase 44 제거 이전).

**2026-06-04 본질 재정의 2차로 무효** — LocalVectorStore 자체가 fp-plugin-search로 이관되어 "단일 구조" 전제 폐기. 결정 사실은 `spec/deprecated.md` → §`mydocsearch_decision.md` 단일 진실원 위임 (2026-06-05 spec 파일 삭제, 메타 룰 19 자기 적용).

흡수 완료 항목(CLAUDE.md / prd/features/)은 그대로 잔류.

## 파일 트리 (105 .rs)

```
crates/core/     domain/(models, classifier, verification, incremental, cross_reference,
                         topic_merger+tests, wiki_export+tests, lint, auto_reindexer,
                         deduplicator, vec_io, todo_lifecycle+tests, memory_tier+tests,
                         chunking+tests, work_queue+tests, crossref_optimizer,
                         diagnostics, tests, audit_log, error_log, hooks, mmr)
                 ports/(output: 9 traits, input: 2 traits)
                 service.rs (14단계 + 2-Pass + quarantine — legacy+pipeline 양쪽 구현)

crates/adapters/ driven/(llm/claude+anthropic+openai+ollama+gemini+fallback+chunked_agent+prompts,
                         storage/zstd+remote_null+network+webdav+s3,
                         embedding/openai+local+claude+python_onnx(legacy)+fastembed(feature)+fastembed_sparse(feature),
                         vector_db/local_store(mmap+Rayon+HNSW+moka+역색인+batch),
                         notify/telegram+slack+composite,    (2026-06-16 notification → notify)
                         verify/claude, preprocessing/composite+tests,    (2026-06-16 verification → verify)
                         rerank/claude+null)    (2026-06-16 reranking → rerank)
                 driving/(watcher+retry_queue, terminal_resolution, terminal_sensitive)
                 stub.rs (Stub×5 + PlainTextPreprocessor)

crates/shared/   cli.rs, config.rs(19섹션+FieldMeta+validate+search 그룹), credential_store.rs,
                 platform.rs, settings_db.rs (단일 SCHEMA + llm_cache + decision_log + c1_rule_thresholds + pii_patterns_user), [mcp_server.rs 삭제 2026-06-17]
                 cached_llm.rs (A1 LLMPort wrapper), auto_suggester.rs (C1 카운터→decision_log), tray.rs

modals/cli/      main.rs(CLI 진입점), cli.rs(커맨드 핸들러), daemon/(windows,unix)
                 tests/(e2e_embedded, scenarios, benchmark, bench_scale, bench_real,
                        bench_real_corpus, bench_micro, bench_prompt_compare,
                        real_env_tests, actor_scenarios, llm_quality_bench,
                        scale_validation, search_accuracy, notification_integration)

modals/app/      main.rs(Tauri GUI + C1 startup trigger), commands.rs(72개), service.rs (C1 4h 주기 task), state.rs
                 tauri.conf.json, icons/icon.ico

ui/              index.html (Decision Log 필터 3개 select + LLM 캐시 카드 그룹),
                 dashboard.css, dashboard.js (loadDecisionLog + accept/reject + clear-llm-cache)
vendor/          (비어있음 — Phase 64 트리거 #11에서 onnxruntime 394MB 삭제. fastembed가 ort-sys 정적 링크로 대체)
prompts.toml     프롬프트 외부화 (핫 리로드)
```

## Ruflo 영감 개선 (2026-05-15)

ruvnet/ruflo 분석에서 도입한 즉시 적용 가능 3건. 모두 인프라만 추가하고 디폴트 비활성 (lesson 15 패턴).

### A1 — LLM 결과 캐시 (file_hash 기반) ★ 통합 완료 2026-05-15

- 위치: `settings.db` 신규 테이블 `llm_cache`
- 스키마: `file_hash PRIMARY KEY, content_hash, result_json, doc_types, hits, created_at, last_hit_at`
- API: `SettingsDb::{lookup_llm_cache, upsert_llm_cache, llm_cache_stats}`
- **통합 어댑터**: `crates/shared/src/cached_llm.rs::CachedLLM` (LLMPort wrapper)
  - 의존 방향: shared → adapters → core (헥사고날 위반 없음)
  - `build_service`에서 ChunkedAgentAdapter 직후 wrapping
- **동작**:
  - `classify_and_process`: 파일 SHA-256 hash로 lookup → hit 시 `result_json` 역직렬화 반환, miss 시 inner 호출 후 upsert
  - `classify_and_process_text`: file_name + content_hash 조합으로 cache key 생성
  - `reprocess_with_feedback`: 캐시 우회 (이전 결과가 부정확해 재시도하는 경로)
  - `enrich_existing`/`summarize_text`: inner 위임 (요약/병합은 누적 컨텍스트라 캐시 부적합)
- **config**: `LlmConfig.llm_cache_enabled` (기본 true)
- **단위 테스트**: 2건 — hit 시 inner 미호출, content 변경 시 재호출 (`cached_llm::tests`)
- **모델 변경**: `ClassifyAndProcessResult`에 `Serialize/Deserialize` 추가 (Metadata는 이미 보유)
- **효과**: 동일 파일 재가공 시 claude_cli 호출 (10~20s/파일, lesson 70) 회피

### A2 — KG 1-hop 확장 검색

- 위치: `McpState.handle_search` truncate 이전
- config: `SearchConfig.expand_kg_hops` (기본 0 = 비활성)
- 동작: seed 결과 N건의 `find_related` 호출 → 신규 target_id를 score=0.0으로 results 끝에 append → 캐시 저장 후 top_k truncate
- 비활성 시 비용 0. 활성 시 추가 비용은 seed × find_related (graph_db 호출 N건)
- 사용 시점: 사용자가 "관련 문서까지 함께"를 명시할 때 또는 검색 confidence가 ambiguous일 때 (트리거 도달 후 디폴트 변경 검토)

### B1 — 다양성 강화 (동일 doc_type 임계값)

- 위치: `McpState.handle_search`, A2 직후
- config: `SearchConfig.diversity_threshold` (기본 0 = 비활성)
- 동작: top_k 범위 내 doc_type 카운트 → threshold 초과 type을 dominant로 마킹 → 범위 밖 첫 non-dominant 결과를 dominant 마지막 항목과 swap
- 단일 swap 보수 동작 (full MMR 아님). RM 5K 코퍼스에서 dominant 편향 측정 후 결정
- 트리거 도달 후 본격 MMR (점수 가중 + 다중 swap) 확장 검토

### 시그널 흐름

```
PipelineConfig.search ──┐
                       ├── McpState { expand_kg_hops, diversity_threshold } (cli.rs + cli/main.rs 양쪽 주입)
SearchConfig (config.rs)┘

handle_search:
  vector_db.search_hybrid → CRAG re-rank → A2 KG hop append → B1 dominant swap → cache save → truncate(top_k) → snippet/log
```

### 테스트

- shared lib 88건 (LLM 캐시 3 + Ruflo defaults 1 + CachedLLM 2 + auto_suggester 3 신규)
- 통합 검증: 빈 stub DB로 컴파일·기본 동작만 검증. 실 효과는 5K 코퍼스 측정 시 검증 예정

### C1 1단계 — Phase 80 카운터 → Decision Log 자동 제안

- 위치: `crates/shared/src/auto_suggester.rs`
- API: `suggest_from_counters(&SettingsDb) -> Result<usize>` (INSERT된 entry 개수)
- 노출: Tauri command `auto_suggest_from_counters` (~~MCP tool 동명~~ — MCP 전체 폐기 2026-06-17)
- 임계값:
  - 검색 mode 100회 이상 + dominant 60% 초과 → `search.preferred_mode` 제안
  - CRAG 50회 이상 + incorrect 25% 초과 → `vector_db.similarity_threshold = 0.85` 제안
- 동작: `decision_log`에 `source="auto_suggestion"`로 INSERT, 사용자는 `setup_decision_log_list`로 검토 후 수동 적용
- **제안만, config 변경 없음** — lesson 30 패턴

### C1 2단계 — Suggested → Accepted (toml 적용)

- API:
  - `auto_suggester::apply_suggested(db, config_path, decision_id) -> (path, after_value)`
  - `auto_suggester::reject_suggested(db, decision_id)`
- 노출: Tauri command `accept_suggested_decision` / `reject_suggested_decision` (~~MCP tool 동명~~ — MCP 전체 폐기 2026-06-17)
- 동작:
  - `accept`: suggested entry의 after_value를 toml_edit으로 path 위치에 쓰기 (주석 보존) + .toml.bak 백업 + decision="accepted"로 갱신
  - `reject`: config 변경 없이 decision="rejected"로 마킹
- 이중 처리 방지: 이미 처리된 entry (suggested 아님) 재호출 시 에러
- DB API: `SettingsDb::get_decision(id)` + `update_decision_status(id, status)` 신규
- 헬퍼 공개: `setup_review::write_toml_path` private → pub (C1 재사용)

### C1 startup 자동 트리거 (2026-05-15)

- 위치: `modals/app/src/main.rs` `.setup()` 직후 별도 thread
- 동작: 앱 시작 시 1회 `suggest_from_counters` 호출. 임계값 미달 시 no-op (debug log only)
- 부수: 사용자가 GUI 켤 때마다 카운터 기준 재평가 — 누적 사용에 따라 자연스럽게 제안 노출

### A1 캐시 무효화

- API: `SettingsDb::clear_llm_cache() -> usize` (삭제된 행 수)
- 노출: Tauri command `clear_llm_cache` (~~MCP tool 동명~~ — MCP 전체 폐기 2026-06-17)
- UI: 헤더 "LLM 캐시" 그룹 라벨에 [비우기] 버튼. confirm + 카드 즉시 갱신
- 사용 시점: 모델/프롬프트 변경 후 (캐시된 결과가 더 이상 유효하지 않을 때)

### A2/B1 inspector 노출

- configFields에 `("search", ...)` 그룹 신규
  - `expand_kg_hops` (integer, default 0)
  - `diversity_threshold` (integer, default 0)
- 기본 0 = 비활성 유지. 사용자가 GUI inspector에서 토글 가능

### Decision Log Dashboard 카드 (Settings 탭)

- 위치: `index.html#decision-log-section` (settings-toolbar 직후)
- `dashboard.js::loadDecisionLog` → `API.setupDecisionLogList(50)`
- suggested entry: Accept/Reject 버튼 (`accept-suggested`/`reject-suggested` action)
- 누른 직후 `loadDecisionLog` 재호출로 즉시 갱신
- 새로고침 + "분석 실행" 버튼 (`run-auto-suggest` action)

### C1 자동 추천 주기 트리거

- `ScheduleConfig.auto_suggest_interval_hours` (기본 4, 0=비활성)
- configField `schedule.auto_suggest_interval_hours` 신규
- `service::start_background_tasks_standalone`에 tokio::spawn loop 추가 (lint 직후)

### C1 임계값 확장 (lesson 33 후속)

추천 룰 3건 추가:
- `verification.max_retry = 3` — processed_total ≥ 30 + quarantine_rate > 25%
- `verification.thresholds.structure_min = 0.3` — verify_pass_rate < 60%
- (기존) `search.preferred_mode`, `vector_db.similarity_threshold`

### B2 — 워커 풀 (사실상 기 완료)

watcher (`adapters/driving/watcher.rs`) 가 `tokio::sync::Semaphore::new(max_workers)` + `tokio::spawn` 패턴으로 가공 큐를 워커 풀로 처리. `max_workers` configField 노출됨 (기본 4). 별도 작업 불필요.

### C2 — PII 검출 강화

- `core/domain/classifier.rs::SensitivityDetector`
  - `scan_pii_in_text(text) -> Vec<PiiHit>` (정규식 5종: ssn_kr / credit_card / email / phone_kr / biz_reg_kr)
  - `is_sensitive_with_content(path, content)` — path 검사 우선, 미일치 시 본문 PII 검사
- `OnceLock` 으로 regex 1회 컴파일 (정적 패턴)
- 외부 의존 없음 (presidio 부재)
- 단위 테스트 6건 신규 (core lib 137 → 143)

### clippy 정리

- error 2건 → 0 (PI 근사값 회피 + never_loop 단일 재시도 명시)
- warning 71 → 18 → **8** (cargo clippy --fix + 안전한 수동 5건: doc 들여쓰기 / wildcard / clamp / &Path / sort_by_key)
- 잔존 8건: too_many_arguments / very_complex_type / loop_var_index — 도메인 설계 트레이드오프, CI 통합 시 `#[allow]` 명시 또는 리팩터링 결정

### A1 LRU GC + Dashboard 사용자 정의 UI (2026-05-15)

- `SettingsDb::gc_llm_cache_to(max_entries)` — last_hit_at NULL 우선 → ASC → hits ASC 순 LRU 삭제
- `LlmConfig.llm_cache_max_entries` (default 10000, 0=무제한). configField 노출
- 주기 task: C1 4시간 트리거 옆에 GC 함께 호출 (`a1-gc` 로그)
- Settings 탭에 3개 카드 신규:
  - **자동 추천 (C1)** — Decision Log + 분석 실행 / status·source·sort 필터
  - **C1 자동 추천 임계값** — 7개 키 디폴트 표시 + input 저장
  - **PII 검출 패턴 (C2)** — 디폴트 5종 readonly + 사용자 정의 추가/제거 + regex 사전 검증 결과 표시

### C2 PII service.rs 통합 (2026-05-15)

`service.rs::process_file_legacy` + `process_file_with_pipeline` 두 진입점에서 본문 PII 검사 통합:

```
1. 경로/파일명 sensitive 판별 (기존)
1.3. 본문 PII 검출 (scan_pii_in_text_with) — 발견 시 handle_sensitive (NEW)
1.5. Fragment 감지 (기존, 동일 read_to_string 재활용)
2. SHA-256 + 증분
```

content를 1.3에서 한 번만 읽어 1.5에서 재사용 — I/O 1회.

### C1/C2 사용자 정의 (settings.db)

신규 테이블 2종:
- `c1_rule_thresholds(key, value)` — 7개 임계값 키 오버라이드. 디폴트는 코드 (auto_suggester.rs)
- `pii_patterns_user(name, pattern, enabled, created_at)` — 디폴트 5종 외 추가. 가입 시 regex 사전 검증

API:
- `SettingsDb::{get/set/list_c1_threshold(s)}`
- `SettingsDb::{list/add/remove_user_pii_pattern(s)}`

build_service에서 PII 패턴을 settings.db에서 로드 → `FileProcessingService.pii_user_patterns` 필드 (lesson 21/27 패턴 — 도메인 구조체 필드 신규).

Tauri commands 추가 (~~+ MCP tools~~ — MCP 전체 폐기 2026-06-17):
- `c1_thresholds_list` / `c1_threshold_set`
- `pii_patterns_list` / `pii_pattern_add` / `pii_pattern_remove`

### Decision Log 필터/정렬 (Settings 탭)

3개 select: status (suggested/accepted/rejected/critical_skipped/all) · source (auto_suggestion/setup_review/setup_modules/all) · sort (decided_at desc/asc).

전역 change 위임 (`document.addEventListener('change', ...)`) 에 `dl-filter-change` 분기. 필터당 즉시 loadDecisionLog 재호출.

200건까지 fetch 후 클라이언트 필터링. 향후 5K+ 항목 시 DB 쿼리 필터링 필요.

### 측정 인프라

- `spec/benchmarks/scripts/gen_synthetic_corpus.ps1` — 5K 합성 코퍼스 생성 PowerShell 스크립트
  - 분포: meeting 30 / research 20 / code 15 / legal 10 / general 25 (%)
  - doc_type별 키워드 풀 30개로 유사도 클러스터링 효과
  - 사용: `pwsh -File spec/benchmarks/scripts/gen_synthetic_corpus.ps1 -OutDir D:\file-test\synthetic_5k -Count 5000`

### Dashboard 가시화 (2026-05-15)

- A1 LLM 캐시: header 카드 그룹 추가 (entries / total_hits / avg_hits)
  - `index.html` stat 카드 + `dashboard.js` `renderStats` + 5초 refresh
- Tauri commands: `get_llm_cache_stats` + `auto_suggest_from_counters` 신규 (62 → 64건)

