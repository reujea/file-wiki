---
created: 2026-06-04
purpose: tasty 패턴 흡수 — host-plugin 분리 단일 진실원
external_sources:
  - tasty: "C:\\dev\\claude_workspaces\\tasty (workspace, host + plugin 별도 프로세스 + tasty-plugin-sdk + tasty-plugin-protocol + tasty-plugin.toml 매니페스트)"
  - tasty_version: "tasty 0.6.0 / api_version 1"
  - precedent_plugins: "tasty-plugin-{claude,codex,image,explorer,clipboard-history,html,git-viewer} 7종"
related:
  - prd/research/competitive-analysis.md (§7 기능 과다 진단 → 본 결정의 직접 트리거)
  - spec/lesson-learned/META.md (메타 룰 20 본 누적 8번째)
  - spec/architecture.md (본 결정 직후 본질 재정의 본문 갱신 의무)
  - spec/deprecated.md (search-extraction-plan.md 무효화 등재)
meta_rule_label:
  dimension_a: "🟢 본질 도메인 일치 (host + plugin 아키텍처는 본 솔루션의 표면적 과다를 직접 해결)"
  dimension_b: "🟢 추상화 매칭 완전 — tasty가 같은 Rust 단일 바이너리 + workspace + IPC plugin"
  classification: "메타 룰 20 8건째 누적"
status: "결정 확정 (2026-06-04 사용자 합의). 2026-06-05 §2-A 재합의 (별도 빌드 + PIPELINE_BASE/plugins/ 런타임 배치) + Phase 200 placeholder 진입"
updated: 2026-06-05 (§2-A 사용자 합의 4축 재정의 + lesson 75 등재)
---

# Plugin 아키텍처 재설계 (tasty 패턴 흡수)

## 0. 본 문서 위상

본 문서는 **file-pipeline 본질 재정의의 단일 진실원**. 2026-06-04 사용자 합의로 다음 결정 확정:

1. **host = 파일 가공만 (최소)** — watcher + Preprocess + Chunk + Metadata 구조화 + DB 영속 + audit 코어
2. **그 외 모두 plugin** — LLM / 임베딩 / 검증 / 분류 / 검색 / KG / 추천 / 알림 / 첨부 / 링크
3. **tasty 패턴 직접 흡수** — workspace + 별도 프로세스 plugin + IPC + 매니페스트 + permission gate
4. **search-extraction-plan.md (Phase 108 검색 분리) 폐기** — 본 결정이 상위, deprecated.md 단방향 위임
5. **본 세션 = 문서만**. Phase 200 시리즈 진입은 별도 세션

본 결정으로 무효화·재정의되는 사항은 `spec/deprecated.md` 단방향 위임. 본 문서가 진실원.

## 1. 결정의 직접 트리거

`prd/research/competitive-analysis.md` 갱신 (2026-06-04) §7 기능 과다 진단:

| 영역 | 현재 | 문제 |
|------|------|------|
| MCP 도구 | **28** (Grimoire 9 대비 3.1배) | 사용자 인지 부담 |
| Tauri commands | **67** | 다중 위치 동기화 비용 (메타 룰 1 sub-rule 1f 누적) |
| UI 6탭 + 서브카드 다수 | - | 첫 진입 학습 곡선 |
| 외부 흡수 영역 | **20** (Phase 87~103 + A/B/E) | 흡수 정책의 무한 누적 한계 |

**옵션 D (검색 분리) → 옵션 E (tasty 패턴) 진화**:
- 옵션 D는 단일 이슈(검색) 정적 분리. 4,200줄 이관 + 8논점 합의 + Phase 7~8건
- 옵션 E는 본질 아키텍처 변경. **사용자 표면적 직접 제어 + 흡수 비용 0** (신규 plugin 등록만)
- 옵션 D는 옵션 E의 부분집합 (fp-plugin-search 하나로 자연 흡수)

## 2. tasty 패턴 5건 (직접 흡수)

### 2-A. workspace 구성 (2026-06-05 갱신 — 사용자 합의 4축)

**원래 안 (번들 plugin)**: `default-members`로 host + 모든 plugin을 한 번에 빌드.

**현행 안 (별도 빌드 + 런타임 배치)**: 사용자 합의 (메타 룰 22 13건째)로 다음 모델 채택:

```
_rust_module/                   ← plugin 워크스페이스 (현재 form-agnostic 모듈 24 멤버)
  └─ cargo build --release      ← module-* binary 산출 (별도)

file-pipeline/src/              ← host 워크스페이스 (member 4)
  └─ cargo build --release      ← pipeline.exe + file-pipeline-tauri.exe (host만)

런타임:
  pipeline.exe 실행 시
   → PIPELINE_BASE/plugins/ 자동 생성 (lesson 29 패턴)
   → plugin 0개로 정상 부팅
   → 사용자가 module-* binary를 plugins/ 폴더에 배치
   → 다음 실행 시 매니페스트 + binary 자동 발견
```

#### 4축 사용자 합의 (2026-06-05)

| 축 | 결정 |
|----|------|
| Plugin 빌드 위치 | `_rust_module/` 별도 워크스페이스 (현재 path 의존 제거 + lesson 71 cargo-xwin 환경 그대로) |
| 형제 모듈 ↔ plugin 관계 | **형제 모듈이 곧 plugin** — `module-llm` = `fp-plugin-llm`. 기존 lib + main.rs(bin) 동시 보유. 24 멤버 → 자연 plugin |
| 런타임 plugin 폴더 | `PIPELINE_BASE/plugins/` — 첫 실행 시 자동 생성 (lesson 29 PIPELINE_BASE 패턴). 비어있으면 plugin 0개로 정상 부팅 |
| Phase 200 진입 | 본 결정과 동시 (§2-A 갱신 + Phase 200 placeholder 진입) |

#### 결정 사실의 의미

- file-pipeline의 path 의존(`{ path = "../../../../_rust_module/module-X" }`) → **plugin 매니페스트 + IPC**로 전환 예정 (Phase 207)
- 단계적 전환: 기존 path 의존 어댑터는 Phase 207까지 유지, 그 시점에 일괄 plugin 전환
- 형제 모듈은 **lib (기존 외부 의존용) + bin (plugin 진입점)** 듀얼 target 보유
- host 빌드 시간 감소 (form-agnostic 모듈 미컴파일)
- plugin 단위 배포 → 사용자가 필요한 기능만 binary 추가 가능 (tasty 패턴)

#### Cargo.toml 형태 예시 (`_rust_module/module-llm/Cargo.toml`)

```toml
[package]
name = "module-llm"
version.workspace = true
edition.workspace = true
license.workspace = true

[lib]
name = "module_llm"
path = "src/lib.rs"
# 외부 lib 의존 유지 (현재 file-pipeline path 의존 호환)

[[bin]]
name = "fp-plugin-llm"     # plugin binary 명명 규칙
path = "src/bin/plugin.rs"
# main() → fp_plugin_sdk::run::<LlmPlugin>()

[dependencies]
module-llm-api  = { path = "../module-llm-api" }
fp-plugin-sdk   = { path = "../fp-plugin-sdk" }   # ← Phase 200 진입 후
# 기존 도메인 의존
```

#### Phase 200 진입 후 변경 트리거

본 §2-A는 Phase 200 placeholder 진입 직후 정합성 검증. 위반 시 메타 룰 30 자기 위반(spec 본문 phase별 즉시 갱신). 검증 항목:
- `_rust_module/Cargo.toml` workspace.members에 `fp-plugin-protocol` / `fp-plugin-sdk` 등록
- 각 module-*/Cargo.toml에 `[[bin]] name = "fp-plugin-*"` 추가 (Phase 207 시점)
- file-pipeline 측 path 의존 → 매니페스트 의존 전환 (Phase 207)

### 2-B. 별도 프로세스 + IPC

- host(`pipeline.exe`) ↔ plugin(`fp-plugin-*.exe`) JSON-RPC over named pipe (Windows) / Unix domain socket (Linux)
- plugin이 죽어도 host 영향 없음
- host가 plugin permissions 검사 후 IPC 메서드 노출

### 2-C. SDK 공유 (`fp-plugin-sdk`)

plugin 작성자가 의존하는 단일 크레이트:
- `connection.rs` — host IPC 연결
- `plugin.rs` — Plugin trait + handle_request
- `host.rs` — host로의 콜백 (vector_db.read 등)
- `runtime.rs` — 이벤트 loop

### 2-D. Protocol 별도 크레이트 (`fp-plugin-protocol`)

- wire 메시지 정의 (events / ipc_method / capability)
- host와 plugin이 양쪽에서 import
- `api_version`으로 호환성 관리

### 2-E. 매니페스트 (`fp-plugin.toml`)

```toml
manifest_version = 1
id = "io.file-pipeline.search"
name = "Search"
version = "0.1.0"
api_version = "1"
permissions = ["vector_db.read", "audit.write"]
event_subscribe = ["processing_completed"]
lang_dir = "lang"

[entry]
type = "process"
command = "fp-plugin-search"

[[contributes.mcp_tool]]
name = "search"
mutates = false
category = "search"
cost = "heavy_compute"

[[contributes.tauri_command]]
name = "search"

[[contributes.ui_card]]
location = "documents_tab"
component = "search-box"
```

## 3. host 잔류 = 파일 가공 (최소 host)

### 3-A. host 책임 (사용자 합의)

| 영역 | 위치 | 사유 |
|------|------|------|
| **파일 입력** | `adapters/driving/watcher.rs` + `core/domain/work_queue.rs` | 모든 plugin이 의존하는 입력 채널 |
| **전처리** | `adapters/driven/preprocessing/preprocessor.rs` (PDF/Excel/한글 추출, 인코딩 감지) | 본 솔루션 본질 가치 (다양한 파일 입력) |
| **청킹** | `core/domain/chunking.rs` (semantic + hierarchy) | 가공의 본질 구조화 |
| **메타데이터 구조** | `core/domain/models.rs::Metadata` | 데이터 스키마 — plugin이 읽기만 |
| **DB 영속** | LocalVectorStore 본체는 plugin이지만 **DocStore + settings.db는 host** | plugin은 읽기, 쓰기는 host 주도 |
| **audit 코어** | `core/audit.rs` + `core::AuditPort` | host가 모든 plugin IPC 호출 기록 |
| **Plugin discovery + registry + permission gate** | 신규 `core/plugin/` | tasty 패턴 직접 흡수 |
| **MCP server + Tauri 진입점** | `shared/mcp_server.rs` + `modals/app/` | 외부 인터페이스 — plugin은 contribute |
| **`pipeline.toml` + `prompts.toml`** | `shared/config.rs` | 본문은 host, plugin은 자기 섹션 contribute |

### 3-B. host에서 plugin으로 이관

| 현재 위치 | plugin |
|----------|--------|
| `core/domain/classifier.rs` | **fp-plugin-classify** (검사 결과는 host에 반환) |
| `core/reasoning/verifier.rs` | **fp-plugin-verify** |
| `core/domain/lint.rs` + Linter | **fp-plugin-lint** |
| `core/domain/cross_reference.rs` + crossref_optimizer | **fp-plugin-crossref** |
| `core/domain/deduplicator.rs` | **fp-plugin-dedup** |
| `core/domain/topic_merger.rs` | **fp-plugin-topic** |
| `core/domain/wiki_export.rs` | **fp-plugin-wiki-export** |
| `core/domain/wikilink.rs` | **fp-plugin-wikilink** |
| `core/domain/mmr.rs` + `vec_io.rs` | **fp-plugin-search** (vector store 본체) |
| `core/domain/hooks.rs` | host에 인터페이스 잔류 + 각 hook은 plugin (host가 호출) |
| `core/domain/purge.rs` | **fp-plugin-purge** |
| `core/domain/incremental.rs` | host 잔류 (가공 핵심) |
| `core/domain/auto_reindexer.rs` | **fp-plugin-reindex** |
| `core/audit.rs` (AuditPort impl) | core impl은 host, 분석/anomaly는 **fp-plugin-audit-analyzer** |

### 3-C. 어댑터 → plugin 변환 (현재 23종)

| 카테고리 | 어댑터 수 | plugin |
|----------|----------|--------|
| **embedding** | 6 | `fp-plugin-embedding-{claude,openai,local,fastembed,fastembed-sparse,python-onnx}` |
| **llm** | 7 | `fp-plugin-llm-{claude,anthropic,openai,gemini,ollama,fallback,chunked-agent}` |
| **storage** | 5 | `fp-plugin-storage-{s3,webdav,network,notion,zstd}` |
| **notification** | 2 | `fp-plugin-notify-{telegram,slack}` |
| **reranking** | 3 | `fp-plugin-rerank-{claude,fastembed,null}` |
| **verification** | 1 | `fp-plugin-verify-claude` |
| **합계 plugin (어댑터만)** | **24** | (현재 어댑터 23 + verify-claude 분리) |

## 4. plugin 분류 — 28 MCP 도구 매핑

| Plugin | MCP 도구 | permissions |
|--------|----------|-------------|
| **fp-plugin-search** | search / get_document / list_documents / get_index | `vector_db.read` |
| **fp-plugin-kg** | kg_neighbors / kg_paths / kg_stats | `vector_db.read` |
| **fp-plugin-lint** | lint / lint_strong_claims | `vector_db.read`, `audit.write` |
| **fp-plugin-setup** | setup_review / setup_apply / setup_dryrun / setup_apply_modules / setup_modules_list / setup_snapshot_{list,rollback,measure} / setup_decision_log_list / setup_profile_infer | `config.read`, `config.write`, `snapshot.write` |
| **fp-plugin-optimize** | optimize / auto_suggest_from_counters / accept_suggested_decision / reject_suggested_decision | `signal.read`, `config.read` |
| **fp-plugin-signal** | stats / get_processing_metrics / get_search_mode_stats / get_crag_stats / get_chunk_stats | `signal.read` |
| **fp-plugin-todo** | list_todos / complete_todo / revise_topic | `todo.read`, `todo.write`, `llm.call` |
| **fp-plugin-llm-cache** | clear_llm_cache / get_llm_cache_stats | `cache.read`, `cache.write` |
| **fp-plugin-pii** | pii_patterns_list / pii_pattern_add / pii_pattern_remove | `config.read`, `config.write` |
| **fp-plugin-c1-thresholds** | c1_thresholds_list / c1_threshold_set | `config.read`, `config.write` |
| **fp-plugin-grimoire** | get_context / write_note | `vector_db.read`, `fs.write` |
| **합계** | **28 도구 / 11 plugin** | 사용자 on/off로 표면적 제어 |

## 5. SDK + Protocol 정의 (Phase 200/201)

### 5-A. `fp-plugin-protocol` 와이어

```rust
// 메시지 envelope
#[derive(Serialize, Deserialize)]
pub struct IpcMessage {
    pub trace_id: String,       // Phase 95 audit 통합
    pub method: String,         // "search.execute" 같은
    pub params: serde_json::Value,
    pub api_version: String,    // 호환성 게이트
}

// 이벤트 (host → plugin)
pub enum HostEvent {
    ProcessingStarted { file_id: String },
    ProcessingCompleted { doc_id: String, metadata: Metadata },
    QuarantineAdded { file_id: String, reason: String },
    VerifyFailed { doc_id: String, claims: Vec<String> },
    ShutdownRequested,
}

// Capability contribute (매니페스트에서 추출)
pub struct ContributedMcpTool {
    pub name: String,
    pub mutates: bool,
    pub category: McpToolCategory,
    pub cost: McpToolCost,
    pub plugin_id: String,
}
```

### 5-B. `fp-plugin-sdk` Plugin trait

```rust
#[async_trait]
pub trait Plugin: Send + Sync {
    fn manifest(&self) -> &PluginManifest;

    async fn handle_request(
        &self,
        method: &str,
        params: serde_json::Value,
        ctx: &PluginContext,
    ) -> Result<serde_json::Value>;

    async fn on_event(&self, event: HostEvent, ctx: &PluginContext) {
        let _ = (event, ctx);
    }
}

pub struct PluginContext {
    pub host: HostHandle,           // permission 게이트가 적용된 host 호출
    pub config_read: ConfigReader,  // 자기 섹션만
    pub config_write: Option<ConfigWriter>,  // permission 있을 때만
    pub audit: AuditWriter,         // trace_id 자동 prepend
}
```

### 5-C. host 측 PluginRegistry

```rust
pub struct PluginRegistry {
    plugins: HashMap<String, PluginHandle>,  // id → 핸들
    permission_gate: PermissionGate,
    mcp_router: McpRouter,                   // contributes.mcp_tool → plugin_id
    tauri_router: TauriRouter,
}

impl PluginRegistry {
    pub async fn discover(&mut self, plugin_dir: &Path) -> Result<()> { ... }
    pub async fn enable(&mut self, plugin_id: &str) -> Result<()> { ... }
    pub async fn disable(&mut self, plugin_id: &str) -> Result<()> { ... }
    pub async fn call(&self, method: &str, params: Value, trace_id: &str) -> Result<Value> { ... }
}
```

## 6. Phase 200~209 단계 (lesson 16 패턴)

| Phase | 영역 | 산출 |
|-------|------|------|
| **200 ✅ (2026-06-05)** | `fp-plugin-protocol` + `fp-plugin-sdk` placeholder + workspace 등록 + `ResolvedPaths.plugins` | lesson 16 단계 0 통과 (placeholder cargo check 42s + 4 단위 테스트 PASS) |
| **201 ✅ (2026-06-05 Q2)** | PluginManifest + parse_manifest_toml + core/plugin/{permission_gate, handle, registry} + 20 단위 테스트 (예정 — 원격 검증) | tasty 직접 흡수, lesson 16 단계 1. `PluginRegistry::call` 은 `IpcNotYetImplemented` (Phase 202 대기) |
| **202** | IPC bus (named pipe / domain socket) + wire 프로토콜 + audit 통합 | trace_id 자동 prepend |
| **203** | **첫 plugin: fp-plugin-search** (4 MCP 도구 외부 이관) | LocalVectorStore + MMR + vec_io 본체 이관 |
| **204** | fp-plugin-kg + fp-plugin-lint + fp-plugin-crossref + fp-plugin-dedup | 검색·검증 영역 |
| **205** | fp-plugin-{setup,optimize,signal,todo} | 운영 plugin 4종 |
| **206** | fp-plugin-{llm-cache,pii,c1-thresholds,grimoire,topic,wiki-export,wikilink,purge,reindex,audit-analyzer} | 영역 plugin 10종 |
| **207** | 어댑터 plugin화 — embedding 6 / llm 7 / storage 5 / notify 2 / rerank 3 / verify 1 | 24 백엔드 plugin |
| **208** | GUI Plugins 탭 (tasty 패턴) — on/off + 매니페스트 표시 + permission 디스플레이 | 사용자 표면적 직접 제어 도달 |
| **209** | 회귀 게이트 + bench 측정 + release 재빌드 + D:\file-test 재배포 | 메타 룰 17/4 의무 |

## 7. 위험 + 완화 (정량)

| 위험 | 영향 | 완화 |
|------|------|------|
| **IPC 오버헤드** | 같은 프로세스 호출 대비 ms 단위 추가 | tasty도 같은 트레이드오프로 운영 중 (선례). Phase 209 bench로 5% 회귀 임계 |
| **타입 안전성 ↓** | wire JSON ↔ Rust 변환 누락 가능 | `fp-plugin-protocol` 크레이트 + 통합 테스트 의무 + serde 강제 |
| **개발 복잡도 ↑** | 28 → 11 + 24 = **35 plugin 멤버** | tasty 20+ 멤버 운영 사례. workspace 패턴 동일 |
| **전환 비용 매우 큼** | 약 1.5~2만 줄 재구조화 | lesson 16 단계 분할 의무 (Phase 200~209 10 단계) |
| **본질 재정의** | 외부 사용자 영향 | 사용자 단일이라 영향 0. spec/architecture.md 재정의로 진실원 갱신 |
| **lesson 30 (인프라 선구현)** | 신규 외부 흡수는 plugin 단위로 분리 → 디폴트 비활성은 자동 (plugin 비활성) | 자연 충족, lesson 30 진화 |
| **MCP 카탈로그 단일 진실원 분산** | host가 mcp_tool_catalog_full() 보유 → plugin 매니페스트로 분산 | `PluginRegistry::mcp_router`가 매니페스트 수집 → 카탈로그 동적 생성. 일치성 테스트 (Phase 92 H3 패턴) |

## 8. 외부 흡수 정책 변경 (메타 룰 20 진화)

본 결정으로 외부 흡수의 단위가 **코드 흡수 → plugin 등재**로 변경:

| 영역 | 변경 전 | 변경 후 |
|------|---------|---------|
| 본질 일치 외부 (메타 룰 20) | 코드 흡수 + 디폴트 비활성 인프라 | **신규 plugin 발행** (host 영향 0) |
| 부수 일치 (메타 룰 21) | 인프라 선구현 | **plugin 발행 + 디폴트 비활성** |
| 도메인 불일치 (메타 룰 21 🔴) | 보류 + META 등재 | 동일 (변경 없음) |

→ lesson 30 "인프라 선구현 + 디폴트 비활성"의 자연 진화. plugin 자체가 비활성 단위.

## 9. 기존 흡수 영역 처리 (Phase 87~107 + A/B/E)

본 결정 후 기존 흡수 20영역은 다음 매핑:

| 흡수 영역 | 현재 위치 | Phase 200 후 |
|-----------|----------|---------------|
| Phase 87 strong_claims | `core/domain/lint.rs` | **fp-plugin-lint** 이관 |
| Phase 88 Metadata 보조 필드 | `core/domain/models.rs` | host 잔류 (스키마) |
| Phase 91 A1 검사 단일 진입점 | `core/domain/classifier.rs` | **fp-plugin-classify** 이관 |
| Phase 91 A2 PII mask | `shared/setup_review.rs` | **fp-plugin-pii** 이관 |
| Phase 91 A3 trace_id | `core/audit.rs` | host 잔류 (audit 코어) |
| Phase 91 B1 Verifier | `core/reasoning/verifier.rs` | **fp-plugin-verify** 이관 |
| Phase 92 H1 anomaly | `shared/audit_anomaly.rs` | **fp-plugin-audit-analyzer** |
| Phase 92 H3 MCP 다차원 카탈로그 | `shared/mcp_server.rs` | host 잔류 (PluginRegistry::mcp_router로 진화) |
| Phase 103 G1~G4 GraphRAG | 분산 | **fp-plugin-search** + **fp-plugin-kg** 분산 |
| Phase A 4지표 측정 | `core/domain/chunking_quality.rs` | host 잔류 (가공 본질) |
| Phase B ChunkingStrategy | `core/domain/chunking.rs` | host 잔류 (가공 본질) |
| Phase E1/E2/E3 Grimoire | `shared/mcp_server.rs` | **fp-plugin-grimoire** 이관 |

## 10. 본 결정으로 무효화되는 사항

| 항목 | 위치 | 처리 |
|------|------|------|
| **search-extraction-plan.md** | `prd/research/` | 자료 보존 + `spec/deprecated.md` 단방향 위임 (lesson 49 패턴) |
| Phase 108~115 (검색 분리) | external-trigger-checklist B-9 | Phase 200~209로 흡수 (fp-plugin-search) |
| mydocsearch_decision.md | `spec/` | 본 결정으로도 무효 (search-extraction-plan과 동일 trail) |
| 메타 룰 1 sub-rule 1f (단일 진입점) 비대화 | META.md | plugin 단위로 자연 해소 |

## 11. 메타 룰 누적

| 메타 룰 | 변경 |
|---------|------|
| **20 본질 도메인 일치 외부 분석** | **8건째** (JAMES + TFM + Mirage + GraphRAG + wikidocs + Adaptive + Grimoire + **tasty**) |
| **22 사용자 정책 경계 합의** | 10건째 (본 결정 4축 합의: host 경계 / plan 처리 / 진입 범위 / 폐기 처리) |
| **30 후보 (spec 본문 즉시 갱신 의무)** | 정식 승격 도달 — Phase 200 진입 시 자기 적용 |

## 12. 본 결정의 메타 가치

- **사용자 표면적 직접 제어** — 28 MCP / 67 Tauri commands가 한꺼번에 노출되는 문제 본질 해결
- **외부 흡수 무한 누적 차단** — plugin 단위 발행으로 host 영향 0
- **개발 복잡도 자연 분산** — workspace 35 멤버지만 각 plugin은 독립 (tasty 선례)
- **lesson 30 / 메타 룰 19 / 메타 룰 16 자연 진화** — 디폴트 비활성 / 단일 진실원 / 차원 A/B가 plugin 단위로 자동 충족
- **검색 분리 plan (옵션 D)의 상위 결정** — 더 큰 가치를 더 작은 단위로 분할 가능 (Phase 200~209 vs Phase 108~115)

## 13. 다음 세션 진입 흐름

본 세션 후 다음 작업 (사용자 합의 후):

1. **Phase 200** — `fp-plugin-protocol` + `fp-plugin-sdk` placeholder + workspace 0건 빌드 (lesson 16 단계 0)
2. **Phase 201** — PluginRegistry + permission gate + 매니페스트 파서
3. **Phase 202** — IPC bus + wire 프로토콜 + audit 통합

각 단계 종결 시 메타 룰 17 release 재빌드 + 원격 검증 의무.
