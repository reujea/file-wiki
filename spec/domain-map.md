---
updated: 2026-06-18 (cycle 7 module-storage-db-1 d2~d4: fp-domain-types(도메인 타입+포트 trait 단일 진실원) + module-storage-db(DB 본체 impl) crate 추출 반영. core/adapters/module-storage-db 모두 fp-domain-types 의존, 순환 0. 이전: Phase 202 본진입 실 IPC + Phase 203 fp-plugin-search placeholder. host = 파일 가공만. 외부 기능 모두 plugin. 단일 진실원: prd/research/plugin-architecture-2026-06-04.md)
---

# 도메인별 구성도

> **단일 진실원** (메타 룰 19, Phase 96 자기 적용): 포트 trait + 어댑터 + 메서드 매핑은 본 문서가 단일 진실원. `architecture.md`는 결정 맥락(Why)과 수치(How many)만 보유. 신규 포트/어댑터 추가 시 본 문서를 먼저 갱신, architecture.md는 결정 맥락만 추가. 메타 룰 19 누적 **9건** 도달 (lesson 49 / 50-A / 50-B / 51 / 52 / 55 / Phase 100 settings-ops-cards / **lesson 74 M-1 META.md 자체 자기 적용** / **lesson 74 S-5 `single_source_check.sh` §검증 grep 자동화**).

## 본질 재정의 (2026-06-04, 메타 룰 22 10건째 누적) — tasty 패턴 흡수

| 영역 | host (파일 가공) 잔류 | plugin 이관 |
|------|---------------------|-------------|
| **입력** | watcher + WorkQueue | - |
| **전처리** | Preprocess (PDF/Excel/한글 추출) | - |
| **구조화** | Chunk + Metadata 구조 + DB 영속 | - |
| **audit** | audit 코어 (AuditPort impl) | audit 분석 (fp-plugin-audit-analyzer) |
| **분류** | - | fp-plugin-classify |
| **검증** | - | fp-plugin-verify / fp-plugin-lint |
| **검색** | - | fp-plugin-search (LocalVectorStore 본체) |
| **KG** | - | fp-plugin-kg |
| **임베딩 어댑터** | - | fp-plugin-embedding-{6} |
| **LLM 어댑터** | - | fp-plugin-llm-{7} |
| **Storage 어댑터** | - | fp-plugin-storage-{5} |
| **Notification 어댑터** | - | fp-plugin-notify-{2} |
| **Reranker 어댑터** | - | fp-plugin-rerank-{3} |
| **추천 / 운영** | Plugin Registry + 매니페스트 + IPC bus | fp-plugin-setup / optimize / signal / todo / pii / c1-thresholds / llm-cache / grimoire |
| **CrossRef / Topic / Wiki / Wikilink / Purge / Reindex** | - | 각 plugin |

→ **host = 1 도메인 (파일 가공)**, 그 외 11 MCP plugin + 24 어댑터 plugin = **35 plugin 멤버**

**메타 룰 16 차원 B 라벨**:
- 🟢 추상화 매칭 완전 (tasty 같은 Rust + workspace + IPC plugin 선례)
- 🟢 본질 도메인 일치 (host + plugin은 본 솔루션 표면적 과다 문제 직접 해결)

상위 단일 진실원: `prd/research/plugin-architecture-2026-06-04.md`. 본 문서 §도메인 1 ~ 6 시계열은 Phase 200~209 진입 직전까지 자료로 보존, Phase 209 종결 시 host-plugin 매핑 본문으로 재작성 예정.

## 도메인 타입 + DB 본체 분리 (cycle 7 module-storage-db-1, 2026-06-18)

> **단일 진실원** (메타 룰 19): 도메인 타입 + 출력 포트 trait 정의는 `_rust_module/fp-domain-types`가 단일 진실원. `file_pipeline_core::domain::*` / `::ports::*`는 re-export shim(호출처 경로 호환). DB 영속 구현은 `_rust_module/module-storage-db`.

### 배경 (d1 BLOCKED → 재설계)

DB 본체(`SqliteSettingsRepo`/`LocalVectorStore`)를 외부 crate로 분리하려면 그 impl이 의존하는 도메인 타입·포트 trait를 core 밖에서도 참조할 수 있어야 한다. core에 두면 `module-storage-db → file_pipeline_core` 의존이 생겨 form-agnostic 위배 + 도메인 단일 진실원이 core에 박힌다. → 순수 타입 + 포트를 별도 crate로 추출.

### fp-domain-types (도메인 타입 + 포트 단일 진실원)

| 구성 | 내용 |
|------|------|
| 순수 타입 통째 이관 | `models` / `config_models` / `settings_models` / `vec_io` / `crossref_optimizer`(MinHashIndex 등) |
| 로직 분리 후 순수 타입만 | `verification_thresholds`(←verification.rs) / `kg_types`(←wiki_export.rs) / `chunk_quality`(←chunking_quality.rs) / `hooks`=HookEvent+HookDefinition(←hooks.rs) |
| 출력 포트 trait | `ports/output`(12 trait: LLM/Embedding/VectorDB/Storage/RemoteStorage/Notification/Verification/Reranker/Preprocess/ProcessingMetrics/Kg/Audit) + `ports/settings_repo`(SettingsRepoPort + 6 sub-trait) |

core 측 = re-export shim. 로직 잔류: verification ROUGE-L / wiki_export `KgQueryEngine` / chunking_quality `compute_*` / hooks `HookRegistry`.

### module-storage-db (DB 본체 impl)

| 타입 | 책임 | 구현 포트 |
|------|------|----------|
| `SettingsDb` (settings_repo.rs, ←adapters sqlite.rs 2292) | settings.db 영속 | `SettingsRepoPort` + 6 sub-trait (Audit/Todo/Decision/Metric/HostTool/LlmCache) |
| `LocalVectorStore` (local_store.rs, ←adapters local_store.rs 1079) | 인프로세스 벡터 DB | `VectorDBPort` |

의존: `module-storage-db → fp-domain-types` 단방향 (file_pipeline_core 무의존, 순환 0). adapters/driven/{settings,vector_db}는 **thin re-export shim (d5 완료)** — `sqlite.rs`(2292→11줄) = `pub use module_storage_db::settings_repo::*`, `local_store.rs`(1079→8줄) = `pub use module_storage_db::local_store::*`. 기존 `file_pipeline_adapters::driven::{settings::sqlite,vector_db::local_store}::*` 경로 + `shared/settings_db.rs` re-export 체인 유지(호출처 변경 0). 코드 중복 해소.

### HostToolRepo 책임 분리 (d3 역방향 결합 제거)

`HostToolRepo::refresh`가 adapters `HostToolDetector::detect_full`을 직접 호출하던 결합(module-storage-db → adapters 역방향) 제거. trait는 **저장 전용**(`list_host_tools`/`host_tools_count`/`replace_host_tools`)으로 재설계. 감지(std::process 실행)는 adapters 책임, 감지+저장 조합은 `shared/host_tools_cache.rs` 자유함수가 담당.

### 의존 방향 (단방향 불변식)

```
file_pipeline_core ──┐
                     ├──> fp-domain-types  (도메인 타입 + 포트 trait)
module-storage-db ───┘
file_pipeline_adapters ──> file_pipeline_core (+ fp-domain-types re-export 경유)
```

## Plugin 도메인 (Phase 200~203 본진입 완료, 2026-06-10)

> **단일 진실원** (메타 룰 19, lesson 75/76 자기 적용): plugin 인터페이스 + 매니페스트 + 권한 매핑 + IPC wire는 본 §가 단일 진실원. `architecture.md`는 결정 맥락(Why)과 상태 전이(How many)만 보유. 신규 plugin 추가 시 본 §를 먼저 갱신, architecture.md는 결정 맥락만 추가.

### Phase 200~203 진입 상태

| Phase | 산출 | 위치 | 상태 |
|-------|------|------|------|
| **200** | fp-plugin-protocol + fp-plugin-sdk placeholder + ResolvedPaths.plugins | `_rust_module/fp-plugin-{protocol,sdk}/` + `crates/shared/src/config.rs` | ✅ |
| **201** | PluginManifest + parse_manifest_toml + PermissionGate + PluginRegistry | `_rust_module/fp-plugin-protocol/` (PluginManifest) + `crates/core/src/plugin/` (host측) | ✅ |
| **202 wire** | IpcMessage + IpcResponse + HostEvent wire 타입 정의 | `_rust_module/fp-plugin-protocol/src/lib.rs` | ✅ |
| **202 본진입** | cross-platform Connection (named pipe / Unix domain socket) + ConnectionPool + PluginRegistry::{call, broadcast_event} 실 구현 + audit 통합 | `_rust_module/fp-plugin-sdk/src/connection.rs` (신규) + `crates/core/src/plugin/{connection_pool.rs, registry.rs}` | ✅ (lesson 76) |
| **203 placeholder** | fp-plugin-search 신규 크레이트 + 매니페스트 placeholder | `_rust_module/fp-plugin-search/` | ✅ (lesson 76 B4) |

### Plugin Protocol 타입 (fp-plugin-protocol 단일 진실원)

| 타입 | Phase | 위치 (src/lib.rs) |
|------|------|------|
| `API_VERSION` | 200 | 상수 (u32 = 1) |
| `ApiVersion` | 200 | type alias |
| `EntryKind::Process { command }` | 200 → 201 | enum (struct variant) |
| `ProtocolError` 5 variants | 200 → 201 | enum |
| `PluginManifest` | 201 | struct (manifest_version / id / name / version / api_version / permissions / event_subscribe / entry / contributes) |
| `Contributes { mcp_tool }` | 201 | struct |
| `ContributedMcpTool` | 201 | struct (name / mutates / category / cost) |
| `parse_manifest_toml(s) -> Result<PluginManifest, ProtocolError>` | 201 | 함수 |
| `IpcMessage { trace_id, method, api_version, params }` | 202 | struct |
| `IpcResponse::{Ok, Err}` (+ `trace_id()`) | 202 | enum (status tag) |
| `HostEvent::{ProcessingStarted, ProcessingCompleted, QuarantineAdded, VerifyFailed, ShutdownRequested}` | 202 | enum (kind tag, snake_case) |

### Plugin SDK (fp-plugin-sdk)

| 타입 / trait | 위치 | 비고 |
|-------------|------|------|
| `Plugin` trait | `_rust_module/fp-plugin-sdk/src/lib.rs` | placeholder — `fn id(&self) -> &str` 만. Phase 207 시점 handle_request / on_event 추가 |
| `SdkError::{Protocol, Ipc}` | 동 | thiserror enum |
| `SDK_API_VERSION` | 동 | protocol과 일치 (compile-time 검증) |
| protocol 재노출 | 동 | parse_manifest_toml / PluginManifest / IpcMessage / IpcResponse / HostEvent / Contributes / ContributedMcpTool / EntryKind / API_VERSION / ApiVersion / ProtocolError |
| **`Connection`** (Phase 202 본진입) | `_rust_module/fp-plugin-sdk/src/connection.rs` | cross-platform IPC wrapper. `#[cfg(windows)]` `WinStream` enum (Client / Server) + `tokio::io::BufStream` wrap |
| **`endpoint_path(plugin_id)`** | 동 | `\\.\pipe\fp-plugin-{id}` (Windows) / `/tmp/fp-plugin-{id}.sock` (Unix) |
| **Connection 공개 API 7건** | 동 | `connect_client`, `accept_server`, `send_request`, `recv_response`, `send_event`, `recv_event`, `recv_request`, `send_response` (실제 8건 — host/plugin 양쪽 모두 노출) |

### Host 측 plugin 모듈 (core/plugin/)

| 모듈 | 책임 | 단일 진입점 |
|------|------|----------|
| `permission_gate.rs` | 권한 검증 | `PermissionGate::from_manifest_permissions(perms, plugin_id) -> Result<_, PluginError>` |
| `handle.rs` | discover 1건의 메타데이터 | `PluginHandle { manifest, manifest_path, permission, state }` + `PluginState::{Discovered, Enabled, Disabled}` |
| `registry.rs` | discover + enable/disable + **실 IPC call + broadcast_event** (Phase 202 본진입) | `PluginRegistry::{new, with_audit, discover, enable, disable, call, broadcast_event, count, list, get}` |
| **`connection_pool.rs`** (Phase 202 본진입) | plugin_id별 active connection lazy + 재사용 | `ConnectionPool::{new, get_or_connect, invalidate}` |
| `mod.rs` | 재노출 + Phase docstring | 외부에서 `core::plugin::*` 로 접근. `HostEvent/IpcMessage/IpcResponse` 재노출 |

### PluginRegistry::call 동작 (Phase 202 본진입)

```
call(plugin_id, method, params, trace_id) -> Result<serde_json::Value, PluginError>
  1. plugin 존재 확인 → 없으면 PluginNotFound
  2. inputs_hash = input_hash_prefix(serde_json::to_vec(&params))
  3. audit.record(trace_id, "plugin.{plugin_id}.{method}", Some(hash), None, None)  ← 시작
  4. conn = pool.get_or_connect(plugin_id) → 실패 시 NotRunning + audit error
  5. IpcMessage 송신 + IpcResponse 수신
  6. Ok{result} → audit.record(..., Some(summary), Some("success")) → Ok(result)
     Err{message} → audit.record(..., Some(summary), Some("error")) → IpcProtocol(message)
     transport_err → pool.invalidate + audit error → IpcTransport
```

### PluginRegistry::broadcast_event 동작 (Phase 202 본진입)

```
broadcast_event(event: HostEvent) -> ()  (silent, best-effort)
  1. event_kind_str(event) → serde tag (예: "processing_completed")
  2. 모든 plugin 순회:
     - !is_enabled() → skip
     - manifest.event_subscribe 미매칭 → skip
     - connect 실패 → warn 로그 + skip
     - send_event 실패 → invalidate + warn 로그 + 계속
```

### 알려진 권한 (KnownPermission 12종, lesson 75)

> **메타 룰 1 sub-rule 1c 자기 적용**: 신규 권한 추가 시 `KnownPermission` enum + `KnownPermission::ALL` 상수 동시 갱신 의무.

| 권한 키 | 의미 (Phase 202 본진입 시 적용 범위) |
|--------|----------|
| `vector_db.read` | 도큐먼트/벡터 조회 (fp-plugin-search / kg) |
| `audit.write` | trace_id 기록 (fp-plugin-lint / audit-analyzer) |
| `config.read` / `config.write` | pipeline.toml 자기 섹션 읽기/쓰기 |
| `snapshot.write` | ConfigSnapshot 생성 (fp-plugin-setup) |
| `signal.read` | processing/search/crag counters (fp-plugin-signal / optimize) |
| `todo.read` / `todo.write` | settings.db.todo (fp-plugin-todo) |
| `llm.call` | LLM 어댑터 호출 (fp-plugin-todo revise) |
| `cache.read` / `cache.write` | A1 LLM 캐시 (fp-plugin-llm-cache) |
| `fs.write` | 외부 파일 쓰기 (fp-plugin-grimoire write_note) |

### PluginError 9 variants (registry.rs, Phase 202 본진입 시점)

`Io / ManifestParse / ApiVersionMismatch / UnknownPermission / DuplicatePluginId / PluginNotFound / NotRunning {plugin_id, cause} / IpcTransport(String) / IpcProtocol(String)` — `thiserror` 파생.

⚠ `IpcNotYetImplemented` variant는 **Phase 202 본진입 시 완전 삭제** — `spec/deprecated.md` 위임.

### Service / McpState 주입 (Phase 202 본진입)

`build_service` (`shared/lib.rs`) — 다음 순서로 PluginRegistry 생성/주입:

```rust
let audit: Arc<dyn AuditPort> = SettingsAuditAdapter::shared(paths.base.join("settings.db"));
let mut plugin_registry = PluginRegistry::new().with_audit(Arc::clone(&audit));
match plugin_registry.discover(&paths.plugins) {
    Ok(n) => if n > 0 { info!("plugin: {} 건 discover", n); }
    Err(e) => warn!("plugin discover 실패 (계속 진행): {}", e),
}
let plugin_registry = Arc::new(plugin_registry);
// FileProcessingService { ..., plugin_registry }
```

`McpState` (`shared/mcp_server.rs`) — `Arc::clone(&service.plugin_registry)` 주입. 본 phase 라우팅 미진입, B4/Phase 203 시점에 contributes.mcp_tool 라우터 활성.

`ServiceBuilder` (`shared/test_helpers.rs`) — `with_plugin_registry()` + 디폴트 `Arc::new(PluginRegistry::new())` (lesson 21/27 회피).

### audit stage 명명 (메타 룰 24 정합)

`plugin.{plugin_id}.{method}` — 변수 기반 stage (`let stage = format!("plugin.{}.{}", ...)`).
audit_stage_check.sh가 본 패턴 검사 (Phase 202 B3 자동화 확장).

### 런타임 plugin 폴더 (lesson 29 PIPELINE_BASE 패턴)

```
PIPELINE_BASE/
└── plugins/                       ← 첫 실행 자동 생성 (config.rs::create_all)
    └── {plugin_id}/               ← 예: io.file-pipeline.search/
        ├── fp-plugin.toml         ← 매니페스트 (parse_manifest_toml 대상)
        └── fp-plugin-search       ← Phase 203 본진입 시 binary 배포 (현재는 _rust_module placeholder)
```

`PIPELINE_PLUGINS` 환경 변수로 override 가능 (lesson 29 패턴).

### Phase 203 fp-plugin-search placeholder (B4, lesson 76)

| 영역 | 위치 / 내용 |
|------|----|
| 크레이트 | `_rust_module/fp-plugin-search/` (lib.rs + Cargo.toml + fp-plugin.toml) |
| `PLUGIN_ID` | `"io.file-pipeline.search"` |
| `CONTRIBUTED_TOOLS` | `&["search", "search_with_filter", "search_similar", "search_kg"]` |
| 매니페스트 권한 | `vector_db.read`, `storage.read`, `audit.write` |
| 매니페스트 구독 | `processing_completed` |
| Plugin trait 구현 | `SearchPlugin { id() = PLUGIN_ID }` placeholder |
| 본진입 시점 이관 대상 | LocalVectorStore + MMR + vec_io 본체 + 4 MCP 도구 handle_* |

### Phase 207 어댑터 → plugin 매핑 (예정, 2026-06-16 outbound 우산 재정의)

상위 단일 진실원: `prd/research/plugin-architecture-2026-06-04.md` §3-C (2026-06-16 outbound 우산 본문 재정의). 본 §는 현재 path 의존 어댑터 24종 + verify 분리 1종 = **25 outbound 변환 대상**. plugin id prefix = `fp-plugin-` → `fp-outbound-` 정합 (lesson 77).

| 카테고리 | 현재 path 의존 (file-pipeline/src/crates/adapters/driven/) | 어댑터 수 | Phase 207 outbound id |
|---------|---------------------------------------------------|-----------|----------------------|
| embedding | embedding/ (claude / openai / local / fastembed / fastembed-sparse / python-onnx) | 6 | `fp-outbound-embedding-*` |
| llm | llm/ | 7 | `fp-outbound-llm-*` |
| storage | storage/ (s3 / webdav / network / notion / telegram / zstd) | 6 | `fp-outbound-storage-*` |
| notify | notify/ (telegram / slack) — **2026-06-16 notification → notify 디렉토리 정정** | 2 | `fp-outbound-notify-*` |
| rerank | rerank/ (claude / fastembed / null) — **2026-06-16 reranking → rerank 디렉토리 정정** | 3 | `fp-outbound-rerank-*` |
| verify | verify/ (claude) — **2026-06-16 verification → verify 디렉토리 정정** | 1 | `fp-outbound-verify-claude` |

**telegram = storage + notify 양쪽 어댑터** (`fp-outbound-storage-telegram` + `fp-outbound-notify-telegram`, CLAUDE.local.md bot 인프라 재사용). lesson 77 §개선 3 정합.

**공통 우산 trait** (2026-06-16 도입 → **2026-06-18 폐기**): 구 `core/ports/outbound/mod.rs::OutboundManifest` super-trait는 본질 재정의 3차(raw I/O) 정합으로 plugin-sdk-1 step-p7에서 **완전 폐기** (디렉토리 + 6 port super-trait bound + 53 impl 제거). capabilities/modes/config_keys 도메인 메타데이터는 plugin manifest(`fp-plugin.toml`) 이관, raw 전송은 `core/ports/raw_transport/` 4 채널로 대체. 단일 진실원 = `spec/deprecated.md` §삭제됨.

전환 단위: **한 모듈 = 한 plugin** (binary plugin 4축 합의). 형제 모듈에 `[[bin]] name = "fp-outbound-*"` 추가 + main.rs (`fp_plugin_sdk::run::<P>()`) 작성.

## 본질 재정의 1차 (2026-06-01, 무효화)

> ⚠️ 본 §는 2026-06-04 결정으로 **무효화**. 시간축 보존을 위해 본문은 유지. 단일 진실원은 위 §2026-06-04.

| 도메인 | 상태 | 진실원 |
|-------|------|--------|
| **1. 문서 처리 (가공)** | ✅ 본질 잔류 (2026-06-04에도 유지) | 본 문서 §도메인 1 |
| **2. 검색** | 🔥 (무효) 외부 분리 → 2026-06-04 fp-plugin-search로 자연 흡수 | `prd/research/search-extraction-plan.md` (무효화) |
| **3. 저장** | (module/ 분리 완료) + Notion 잔여분 → 2026-06-04 fp-plugin-storage-* | §도메인 3 (잔류분) |
| **4. 알림/모니터링** | (module/ 분리 완료) + AuditPort → audit 코어는 host, 분석은 plugin | §도메인 4 (잔류분) |
| **5. 추천 시스템** | (1차 본질 잔류) → 2026-06-04 fp-plugin-{setup,optimize,signal,c1-thresholds} | §도메인 5 |
| **6. 검증·lint 보조** | (1차 본질 잔류) → 2026-06-04 fp-plugin-{verify,lint,classify} | §도메인 6 |

## Phase 103 GraphRAG 흡수 (인프라 선구현, lesson 30 패턴)

| 영역 | 신규 필드/타입 | 디폴트 | 트리거 |
|------|--------------|--------|--------|
| **G1 Statement 노드** | `core::domain::models::Metadata.statements: Vec<String>` | 빈 Vec | 가공 50파일+ + needs_verification 누적 5건+ |
| **G2 의미 관계** | `core::domain::models::RelationType::Semantic(String)` variant | 미사용 | KG 관계 평균 <2 + LLM 프롬프트 semantic_relations 활성 |
| **G3 Multi-hop 빔 검색** | `shared::config::SearchConfig.kg_beam_search: bool` + `shared::mcp_server::McpState.kg_beam_search: bool` | false | A2 활성(expand_kg_hops>0) + 사용자 만족도 신호 |
| **G4 TF-IDF 다양성 재순위** | `shared::config::SearchConfig.tfidf_rerank_enabled: bool` + `shared::mcp_server::McpState.tfidf_rerank_enabled: bool` | false | 사용자 검색 30회+ + MRR before/after 측정 |

## 4개 도메인 경계

```
+====================+     +====================+     +====================+
|  문서 처리 도메인    |     |  검색 도메인         |     |  저장 도메인         |
|                    |     |                    |     |                    |
|  LLM 분류/가공      | --> |  임베딩 생성         | --> |  zstd 압축          |
|  검증 (6가지)       |     |  벡터DB 색인        |     |  원격 저장소 (신규)   |
|  청킹 (의미 단위)   |     |  하이브리드 검색      |     |  .vec 영속화        |
|  전처리 (PDF/OCR)   |     |  리랭킹 (신규)       |     |  TTL 만료 삭제      |
|  민감 판별          |     |  KG 그래프          |     |                    |
+====================+     +====================+     +====================+
         |                          |                          |
         v                          v                          v
+========================================================================+
|                      알림/모니터링 도메인                                  |
|                                                                        |
|  Telegram/Slack 알림 | 스케줄 (purge/lint) | 로깅 | 대시보드             |
+========================================================================+
```

---

## 도메인 1: 문서 처리

### 포트
| 포트 | 파일 | 메서드 |
|------|------|--------|
| LLMPort | ports/output.rs:14 | classify_and_process, reprocess_with_feedback, merge_todo |
| PreprocessPort | ports/output.rs:160 | preprocess, preprocess_with_config |
| VerificationPort | ports/output.rs:222 | detect_hallucination, verify_completeness |
| DuplicateResolutionPort | ports/input.rs | resolve |
| SensitiveNotificationPort | ports/input.rs | notify_and_collect |

### 어댑터
| 어댑터 | 파일 | 설명 |
|--------|------|------|
| ClaudeCliAdapter | llm/claude_adapter.rs | Claude CLI 호출 |
| AnthropicApiAdapter | llm/anthropic_adapter.rs | Anthropic API |
| OpenAiLlmAdapter | llm/openai_llm_adapter.rs | OpenAI API |
| OllamaAdapter | llm/ollama_adapter.rs | Ollama 로컬 |
| GeminiAdapter | llm/gemini_adapter.rs | Google Gemini |
| FallbackLlmAdapter | llm/fallback_adapter.rs | 순차 시도 |
| ChunkedAgentAdapter | llm/chunked_agent.rs | >40KB 분할 위임 |
| ClaudeVerificationAdapter | verify/claude_verifier.rs | 환각 탐지 (2026-06-16 verification → verify 디렉토리 정정) |
| CompositePreprocessor | preprocessing/preprocessor.rs | PDF/OCR |

### 설정 ↔ UI 매핑
| 설정 | config.rs | UI 위치 | 상태 |
|------|-----------|---------|------|
| llm.provider | LlmConfig | Settings 크레덴셜 그룹 | O |
| llm.default_credential | LlmConfig | Settings 크레덴셜 그룹 | O |
| llm.fallback_providers | LlmConfig | (없음) | **누락** — 크레덴셜 우선순위로 대체 가능 |
| models.classify_model | ModelsConfig | Pipeline LLM 노드 | O |
| models.process_model | ModelsConfig | Pipeline LLM 노드 | O |
| models.verify_model | ModelsConfig | Pipeline Verify 노드 | O |
| verification.enabled | VerificationConfig | Pipeline Verify 노드 | O |
| verification.max_retry | VerificationConfig | Pipeline Verify 노드 | O |
| verification.thresholds | VerificationConfig | Pipeline Verify 노드 | O |
| preprocessing.pdf_tool | PreprocessingConfig | Pipeline Preprocess 노드 | O |
| preprocessing.ocr_tool | PreprocessingConfig | Pipeline Preprocess 노드 | O |
| sensitive.keywords | SensitiveConfig | Pipeline Sensitive 노드 | O |
| sensitive.extensions | SensitiveConfig | Pipeline Sensitive 노드 | O |
| chunking.semantic_enabled | ChunkingConfig | Pipeline 청킹 서브탭 | O |
| chunking.target_bytes | ChunkingConfig | Pipeline 청킹 서브탭 | O |
| chunking.overlap_sentences | ChunkingConfig | Pipeline 청킹 서브탭 | O |
| chunking.preserve_code_blocks / preserve_tables | ChunkingConfig | Pipeline 청킹 서브탭 (preserve_tables는 Phase 86 트리거 #8 인프라, default false) | O |

---

## 도메인 2: 검색

### 포트
| 포트 | 파일 | 메서드 |
|------|------|--------|
| VectorDBPort | ports/output.rs:82 | search_similar, search_hybrid, upsert, link |
| EmbeddingPort | ports/output.rs:176 | embed, embed_batch, embed_with_model |
| RerankerPort | ports/output.rs | rerank, is_enabled |

### 어댑터
| 어댑터 | 파일 | 설명 |
|--------|------|------|
| LocalVectorStore | vector_db/local_store.rs | 인프로세스 (mmap+HNSW+키워드 역색인+Blue-Green slot+batch). MinHashIndex + 메타데이터 블로킹 옵션 (Phase 52/59) |
| FastEmbedAdapter | embedding/fastembed_adapter.rs | **BGE-M3 1024차원, MRR 0.975, 64ms/건** (Phase 62, feature=fastembed) |
| FastEmbedSparseAdapter | embedding/fastembed_sparse.rs | BGE-M3 sparse(lexical) + dot 유사도 (Phase 63). **Phase 89 B-2**: core 도메인 `SparseEmbedding` + 포트 디폴트(`embed_sparse`/`upsert_sparse_embedding`/`search_sparse`) no-op 추가. 어댑터 EmbeddingPort impl + LocalVectorStore sparse_index 통합은 트리거 #10 대기 |
| ClaudeEmbeddingAdapter | embedding/claude_embed.rs | 128축 의미 벡터 |
| OpenAIEmbeddingAdapter | embedding/openai_embed.rs | text-embedding-3-small |
| LocalEmbeddingAdapter | embedding/local_embed.rs | 키워드 해시 |
| PythonOnnxAdapter | embedding/python_onnx_embed.rs | Python subprocess 폴백 (legacy. Rust ort는 트리거 #11에서 폐기) |
| FastEmbedReranker | rerank/fastembed_reranker.rs | **BGE-Reranker-v2-M3 Cross-Encoder** (Phase 62, default) (2026-06-16 reranking → rerank 디렉토리 정정) |
| ClaudeReranker | rerank/claude_reranker.rs | Claude CLI 관련도 점수 (fallback) |
| NullReranker | rerank/null_reranker.rs | 패스스루 |

### 설정 ↔ UI 매핑
| 설정 | config.rs | UI 위치 | 상태 |
|------|-----------|---------|------|
| vector_db.backend | VectorDbConfig | Settings (get_config) | O |
| vector_db.dim | VectorDbConfig | Settings (get_config) | O |
| vector_db.semantic_dup_threshold | VectorDbConfig | Settings (select 박스) | O |
| embedding.default_model | EmbeddingConfig | Pipeline Embedding 노드 ("fastembed BGE-M3 1024차원 고정" 표시, Phase 65) | O |
| crossref.similarity_threshold | CrossrefConfig | Pipeline 청킹 서브탭 (기본 0.8, Phase 59 트리거 #1 적용) | O |
| crossref.minhash_force / minhash_min_docs | CrossrefConfig | Pipeline 청킹 서브탭 (Phase 59 트리거 대기 옵션) | O |
| crossref.metadata_blocking | CrossrefConfig | Pipeline 청킹 서브탭 (Phase 59 트리거 대기 옵션) | O |
| rerank.enabled | RerankConfig | Settings 인프라 (default true, Phase 65 fastembed 고정) | O |
| rerank.provider | RerankConfig | Settings 인프라 ("fastembed" default) | O |
| rerank.top_n | RerankConfig | Settings 인프라 | O |
| search.window_lines / mmr_lambda / sparse_weight / time_weight | SearchConfig | Pipeline 검색 노드 (Phase 71 신규 섹션) | O |
| search.expand_kg_hops / diversity_threshold | SearchConfig | Pipeline 검색 노드 (Ruflo A2/B1, default 0=비활성) | O |
| search.hyde_enabled / hyde_min_results | SearchConfig | Pipeline 검색 노드 (Phase 86 트리거 #6 인프라, default false=비활성, 어댑터 generate_hypothetical 오버라이드 후 활성) | O |

---

## 도메인 3: 저장

### 포트
| 포트 | 파일 | 메서드 |
|------|------|--------|
| StoragePort | ports/output.rs:62 | compress_and_store, decompress_temp, delete_expired, read_header |
| RemoteStoragePort | ports/output.rs | upload, download, list, delete, is_configured, **capabilities (Phase 92 H5)**. (구 `OutboundManifest` super-trait bound는 2026-06-18 step-p7 폐기 — raw I/O 재정의. 단일 진실원 = deprecated.md) |

### 어댑터
| 어댑터 | 파일 | 설명 |
|--------|------|------|
| ZstdStorageAdapter | storage/zstd_storage.rs | zstd 압축 |
| NullRemoteStorage | storage/remote_null.rs | 비활성 (module-storage `NullRemoteStorage` thin wrapper) |
| NetworkStorageAdapter | storage/network_storage.rs | 로컬 네트워크 경로 (module-storage `NetworkRemoteStorage` thin wrapper) |
| WebDavStorageAdapter | storage/webdav_storage.rs | WebDAV (module-storage `WebDavRemoteStorage` thin wrapper) |
| S3StorageAdapter | storage/s3_storage.rs | S3 호환 (module-storage `S3RemoteStorage` thin wrapper) |
| NotionStorageAdapter | storage/notion_storage.rs | **Phase 90 신규** — Notion API v1 (Notion-Version: 2022-06-28). `mode="page"` (가공본 → 자식 페이지 + paragraph 블록, 2000자/100블록 분할 처리) / `mode="attach"` (명시적 미지원, Notion API 제약). reqwest 직접 호출 (module-storage 외부, 도메인 특수성) |

### 설정 ↔ UI 매핑
| 설정 | config.rs | UI 위치 | 상태 |
|------|-----------|---------|------|
| compression.zstd_level | CompressionConfig | Pipeline Storage 노드 | O |
| compression.original_ttl_days | CompressionConfig | Settings (get_config) | O |
| remote_storage.enabled | RemoteStorageConfig | Pipeline 외부 저장소 서브탭 | O |
| remote_storage.provider | RemoteStorageConfig | Pipeline 외부 저장소 서브탭 (network/webdav/s3/**notion** Phase 90) | O |
| remote_storage.notion_token / notion_parent_page_id / notion_mode / notion_database_id | RemoteStorageConfig | Pipeline 외부 저장소 서브탭 (provider=notion 선택 시 노출) | O (Phase 90) |

---

## 도메인 4: 알림/모니터링

### 포트
| 포트 | 파일 | 메서드 |
|------|------|--------|
| NotificationPort | ports/output.rs:194 | send, send_duplicate_alert, send_sensitive_alert, send_completion, send_summary |
| **AuditPort (Phase 94 신규)** | **ports/output.rs:12** | **record(trace_id, stage, inputs_hash, output_summary, applied_rule) — settings.db audit_trace 1줄 기록** |

### 어댑터
| 어댑터 | 파일 | 설명 |
|--------|------|------|
| TelegramNotificationAdapter | notify/telegram_notify.rs | Telegram Bot API (2026-06-16 notification → notify 디렉토리 정정. **outbound 우산 재정의 직후 storage 양쪽 어댑터 신설 후보** = `notify/telegram_notify.rs` + `storage/telegram_storage.rs`, lesson 77) |
| SlackNotificationAdapter | notify/slack_notify.rs | Slack Web API |
| CompositeNotificationAdapter | notify/composite.rs | 멀티채널 |
| NullNotificationAdapter | notify/composite.rs | 비활성 |
| **NullAuditAdapter (Phase 94 신규)** | **core ports/output.rs** | **디폴트 no-op (lesson 14 회피)** |
| **SettingsAuditAdapter (Phase 94 신규)** | **shared/settings_audit_adapter.rs** | **settings.db audit_trace 기록. 실패 silent** |

### 설정 ↔ UI 매핑
| 설정 | config.rs | UI 위치 | 상태 |
|------|-----------|---------|------|
| notification.telegram.bot_token | TelegramConfig | Settings 시스템 > 알림 | O |
| notification.telegram.chat_id | TelegramConfig | Settings 시스템 > 알림 | O |
| notification.slack.bot_token | SlackConfig | Settings 시스템 > 알림 | O |
| notification.slack.channel | SlackConfig | Settings 시스템 > 알림 | O |
| logging.level | LoggingConfig | Settings 시스템 > 로깅 | O |
| logging.file/console | LoggingConfig | Settings 시스템 > 로깅 | O |
| dashboard | [제거됨] | Tauri WebView — 포트/인증 불필요 | — |
| schedule.* | ScheduleConfig | Settings 스케줄·경로 | O |
| schedule.lint_interval_hours / lint_weekly_hours / lint_monthly_hours | ScheduleConfig | Settings 스케줄·경로 (Phase 87 다층 lint, wikidocs 353407 매일/주1회/월1회) | ✅ **Phase 89 N-3** — service.rs 3진입점 분기 활성 (lint/lint_strong_claims/lint_topics) |
| paths.extra_inboxes | PathsConfig | Settings 스케줄·경로 | O |
| max_workers | PipelineConfig | Settings 스케줄·경로 | O |
| hooks (event/webhook_url/command/enabled) | Vec<HookDefinition> | Settings 이벤트 훅 — **CRUD 모달** (Phase 84) | O |

---

## 누락 요약

### 해결 완료 (2026-04-16)

모든 누락 설정이 config_metadata + Settings UI system 그룹에 추가됨:
- ✅ schedule 전체 — Phase 29에서 추가
- ✅ chunking 전체 — Phase 39에서 추가 (semantic_enabled/target_bytes/max_bytes/overlap_sentences/preserve_code_blocks)
- ✅ paths.extra_inboxes — Phase 29에서 추가
- ✅ max_workers — Phase 29에서 추가
- ✅ remote_storage — Phase 29에서 추가
- ✅ rerank — Phase 29에서 추가

---

## 도메인 5: 추천 시스템 (Phase 73~83 신규)

사용자 설정 추천 + 자동 롤백 + 의사결정 영속화. 5축 SetupProfile (Phase 76) → 동작 모듈 12종 (Phase 80) 전환 완료.

### 핵심 모듈

| 모듈 | 위치 | 역할 |
|------|------|------|
| setup_review | shared/setup_review.rs | 룰 테이블 + RecommendationEngine + apply_advice_full (toml_edit 주석 보존) |
| setup_rules.toml | shared/setup_rules.toml | 46개 룰 (content 27 + sensitivity 7 + volume 6 + intent 4 + collaboration 2). include_str! 임베드 |
| setup_modules | shared/setup_modules.rs | 12개 동작 모듈 (가공 5 + 검색 4 + 운영 3). exclusive_group + 충돌 해소 |
| setup_modules.toml | shared/setup_modules.toml | 모듈 정의 |
| setup_dryrun | shared/setup_dryrun.rs | diff_configs + infer_profile_from_usage + detect_mismatch (Phase 78) |
| config_snapshot | shared/config_snapshot.rs | ConfigSnapshot + 자동 롤백 4트리거 (Phase 77) |
| decision_log | settings.db decision_log 테이블 | apply 이력 영속화 (Phase 82) |
| host_tools_cache | shared/host_tools_cache.rs | 호스트 도구 감지 settings.db 캐시 (Phase 81) |

### MCP 도구 (Claude Code 통합)

| 도구 | Phase | 역할 |
|------|-------|------|
| setup_review / setup_apply | 73 | 시나리오 → 추천 → 적용 |
| setup_dryrun / setup_profile_infer | 78 | 미리보기 + 패턴 자동 추정 |
| setup_snapshot_list / rollback / measure | 77 | 스냅샷 + 효과 측정 + 자동 롤백 |
| setup_modules_list / setup_apply_modules | 80 | 동작 모듈 추천 |
| setup_decision_log_list | 82 | 적용 이력 조회 |
| get_search_mode_stats / get_crag_stats / get_chunk_stats / get_processing_metrics | 80/82-prep | 코퍼스 신호 카운터 |
| refresh_host_tools | 81 | 호스트 도구 캐시 재감지 |

### settings.db 신규 테이블 (Phase 77~84)

- `config_snapshots` (Phase 77) — apply 직전 pipeline.toml + metrics_json 보존
- `search_mode_counters` / `crag_counters` / `chunk_stats` (Phase 80) — 코퍼스 신호 카운터
- `processing_metrics` (Phase 82-prep) — verify/quarantine/process_time 누적
- `decision_log` (Phase 82) — apply 결정 이력 (accepted/rejected/critical_skipped)
- `host_tools_cache` (Phase 81) — pandoc/python_docx/python_openpyxl/libreoffice 감지 결과
- `mcp_disabled_tools` (Phase 84) — MCP 도구 비활성화 목록 (존재 = OFF, call_tool/list_tools 차단)
- `llm_cache_gc_log` (Phase 84) — A1 LRU GC 마지막 결과 (id=1 단일 행, GUI stat 카드 노출)
- DDL 단일 상수화 (Phase 82-prep, `SETTINGS_DB_SCHEMA`) — lesson 10/26 재발 차단

### 신규 도메인 객체 (Phase 83)

- `DocRelation.origin: RelationOrigin` 5종: auto_similarity / user_wikilink / llm_extracted / user_manual / lint_auto_fix
- `core/domain/wikilink.rs` — `[[xxx]]` 위키링크 추출 + 한국어/영문 지원
- KG API(kg_neighbors/kg_paths) origin + origin_label_ko 노출

---

## 도메인 6: 검증·lint 보조 (Phase 87~88, wikidocs 353407 정리·감사 흐름)

본 프로젝트는 wikidocs 353407 권고의 ~90%를 이미 구현. Phase 87~88에서 부분 미구현 항목 추가:

### Metadata 보조 필드 (Phase 87 인프라, Phase 88 LLM 채움 완성)

| 필드 | 위치 | 채우는 주체 |
|------|------|-----------|
| `Metadata.needs_verification: Vec<String>` | core/domain/models.rs | LLM 가공 (prompts.toml classify) — Phase 88 완성. 평균 1.9건/doc |
| `Metadata.open_questions: Vec<String>` | core/domain/models.rs | LLM 가공 — Phase 88 완성. 평균 2.2건/doc |

영속화 4계층 (lesson 1 메타 룰 1 / lesson 42):
- `core/domain/models.rs::Metadata` (Phase 87)
- `adapters/llm/response.rs::LlmResponse` (Phase 88)
- `adapters/vector_db/local_store.rs::StoredDoc` (Phase 88)
- `.local-store.json` 자동 직렬화

### Lint 보조 검증 (Phase 87/88)

| 함수 | 위치 | 역할 |
|------|------|------|
| `verification::detect_strong_claims(text)` | core/domain/verification.rs (Phase 87) | 단정 표현 12종 마커 검출 → 약화 권고 후보 Vec<String> (점수화 아님, 사용자 검토용) |
| `Linter::lint_strong_claims(vector_db, storage, max_per_doc)` | core/domain/lint.rs (Phase 88) | 가공본 storage 복원 → detect_strong_claims → LintIssue 생성 |
| `LintIssueType::StrongClaim` | core/domain/models.rs (Phase 88) | enum 신규 변형 |

호출처: 단위 테스트 + 측정 검증 + **Phase 89 N-3에서 service.rs schedule task `lint_weekly_hours` 분기 활성 (standalone + dead + CLI 3곳)**. UI는 Phase 89 N-4 `get_lint_strong_claims` Tauri command + Verification 탭 "주간 검토 — 강한 주장" 카드.

### lint 다층 주기 (Phase 87 인프라, N-3 후속 활성)

| 필드 | 디폴트 | wikidocs 353407 매핑 |
|------|--------|---------------------|
| `schedule.lint_interval_hours` | 6h | 매일 — 색인 정합성/상한 검사 |
| `schedule.lint_weekly_hours` | 168h (7일) | 주 1회 — 중복·미연결 검사 |
| `schedule.lint_monthly_hours` | 720h (30일) | 월 1회 — 오래된·상충 검사 |

### 외부 분석 단일 진실원

- `prd/research/external-analysis-2026-05-15.md` (Phase 88 W-1) — supertonic / wikidocs 352523 / 353407 분석 결정. 향후 외부 분석 시 본 문서 인용 의무
