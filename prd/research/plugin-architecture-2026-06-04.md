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
status: "결정 확정 (2026-06-04 사용자 합의). 2026-06-05 §2-A 재합의 (별도 빌드 + PIPELINE_BASE/plugins/ 런타임 배치) + Phase 200~202 placeholder. 2026-06-10 Phase 202 본진입 완료 + Phase 203 placeholder (lesson 76, bundle-cycle B1~B4)"
updated: 2026-06-05 (§2-A 사용자 합의 4축 재정의 + lesson 75 등재)
---

# Plugin 아키텍처 재설계 (tasty 패턴 흡수)

## 0. 본 문서 위상

본 문서는 **file-pipeline 본질 재정의의 단일 진실원**. 2026-06-04 사용자 합의로 다음 결정 확정 (2026-06-16 outbound 우산 추상화 2차 / 2026-06-17 본질 재정의 3차 누적):

1. **host = 파일 가공만 (최소)** — watcher + Preprocess + Chunk + Metadata 구조화 + DB 영속 + audit 코어
2. **그 외 모두 plugin** — LLM / 임베딩 / 검증 / 분류 / 검색 / KG / 추천 / 알림 / 첨부 / 링크
3. **tasty 패턴 직접 흡수** — workspace + 별도 프로세스 plugin + IPC + 매니페스트 + permission gate
4. **search-extraction-plan.md (Phase 108 검색 분리) 폐기** — 본 결정이 상위, deprecated.md 단방향 위임
5. **본 세션 = 문서만**. Phase 200 시리즈 진입은 별도 세션

### 0-A. 본질 재정의 3차 (2026-06-17 사용자 합의, lesson 79 후보)

직전 cycle 4 진행 중 사용자 발화 = 본질 재정의 3차:

1. **MCP 완전 폐기** — `shared/mcp_server.rs` (25 도구) + Tauri commands MCP 연계 전부 삭제. 외부 접근 부재. 28 MCP 도구 표면 차단. 메타 룰 22 21건째
2. **외부 plugin이 가공 파이프라인 스텝에 연결** — 신규 §3-D 인터페이스 spec (전/후 hook + HookPoint trait + IPC 입찰). step 전후에 plugin broadcast_event + 동기 수신 대기
3. **Output adapter = raw I/O 만** — outbound 어댑터 = raw 전송/수신 transport only (telegram bot.sendMessage 호출 only). 변환/검증/capabilities/mode = plugin 책임. OutboundManifest super-trait 폐기. plugin 측에서 어댑터 호출 후 모든 도메인 로직 처리

본 재정의 = lesson 77 outbound 우산 추상화 (메타 룰 22 19/20건째) 의 후속 결정 — 어댑터 단순화 + plugin 단위 책임 확장 + 본 host 표면 차단 강화.

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
| **~~MCP server~~ + Tauri 진입점** | ~~`shared/mcp_server.rs`~~ (2026-06-17 본질 재정의 3차 = 완전 폐기) + `modals/app/` | **MCP 완전 폐기** (lesson 79 후보, 메타 룰 22 21건째). Tauri commands 만 host 외부 인터페이스. plugin contribute = §3-D step hook 으로 일원화 |
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

### 3-C. 어댑터 → raw I/O transport (2026-06-17 본질 재정의 3차, lesson 79 후보)

#### 본질 재정의 3차 — Output adapter = raw I/O 만 (사용자 합의 2026-06-17)

사용자 발화 = `"Output adaptor들은 외부 데이터를 전달만 하고, plugin이 받아서 처리하게 하자"`.

직전 2026-06-16 outbound 우산 추상화 (옵션 C, OutboundManifest super-trait + capabilities/modes 명세) = **폐기**. 본 3차 재정의:

- **어댑터 = raw transport only** — telegram bot.sendMessage / s3 PutObject / openai chat.completions HTTP 호출 only. 도메인 로직 0
- **변환 / 검증 / capabilities / mode / 제약 = plugin 책임** — fp-plugin-storage-telegram 이 mode 분기 + sqlite mapping + 48h 검증 + 50MB pre-check 등 모든 도메인 로직 보유
- **OutboundManifest super-trait 폐기** — capabilities = plugin manifest (`fp-plugin.toml`) 박힘. 어댑터는 `RawTransport` trait 만 impl
- **헥사고날 정합** = `core/ports/raw_transport/{http,websocket,sqlite,fs,...}.rs` (전송 채널별 분류) + `adapters/driven/transport/{telegram_http,s3_http,...}.rs` (외부 SDK 호출 wrap)
- **신규 외부 연계 추가** = 외부 SDK transport wrap (수십 줄) + plugin 본문 신설 (도메인 로직 plugin 책임). 어댑터 자체는 SDK 호출 forward only

본 재정의의 가치 = **어댑터 비대화 차단** + **plugin 단위 책임 확장** + **회귀 영역 plugin 단위로 분산** (어댑터 변경 = SDK API 변경만, 도메인 변경 = plugin 본문만). lesson 45 (Notion 특수성 직접 구현) 의 다음 진화 = 특수성을 어댑터에서 plugin으로 완전 이관.

#### raw transport 표 (2026-06-17 본질 재정의 3차)

기존 outbound 우산 6 port (RemoteStoragePort + EmbedderPort + LlmPort + NotifyPort + RerankerPort + VerifierPort) **모두 폐기**. plugin 단위 매핑:

| transport 채널 | 어댑터 위치 | 호출하는 plugin |
|---------------|----------|--------------|
| **HTTP (reqwest)** | `adapters/driven/transport/http_client.rs` (raw GET/POST/PUT/DELETE) | fp-plugin-storage-{s3,webdav,notion,telegram} / fp-plugin-llm-{claude,openai,anthropic,gemini,ollama} / fp-plugin-embedding-{claude,openai,fastembed} / fp-plugin-rerank-{claude,fastembed} / fp-plugin-verify-claude / fp-plugin-notify-{telegram,slack} |
| **filesystem** | `adapters/driven/transport/fs.rs` (raw read/write/exists) | fp-plugin-storage-network / fp-plugin-embedding-{local,python-onnx} / fp-plugin-storage-zstd |
| **stdio (subprocess)** | `adapters/driven/transport/stdio.rs` (raw stdin/stdout/stderr) | fp-plugin-llm-claude (claude CLI) / fp-plugin-embedding-python-onnx / fp-plugin-llm-ollama |
| **sqlite** | `adapters/driven/settings/sqlite.rs` (cycle 4 prep-unlock 종결, host 잔류) | host 본체 (config/audit/todo/decision_log/c1/c2/llm_cache) |

기존 outbound id (`fp-outbound-storage-telegram` 등) 는 **deprecated.md 단방향 위임**. plugin id 는 `fp-plugin-{category}-{name}` 로 재정의 (2026-06-04 §3-B 매핑 정합 복원).

#### plugin 책임 (도메인 로직)

기존 어댑터에 박힌 모든 도메인 로직 = plugin 본문으로 이관:

| 영역 | 기존 어댑터 위치 | 이관 후 plugin 위치 |
|------|--------------|------------------|
| telegram mode 분기 (document/text/channel) | `telegram_storage.rs::TelegramStorageAdapter` | `fp-plugin-storage-telegram::process_request` |
| telegram_message_map sqlite 매핑 | `telegram_storage.rs` + `settings_db.rs::telegram_message_map` | `fp-plugin-storage-telegram` 자체 sqlite (plugin 단위) |
| telegram 48h delete 검증 | `telegram_storage.rs::delete` | `fp-plugin-storage-telegram` 정책 본문 |
| telegram 50MB upload pre-check | `telegram_storage.rs::upload` | `fp-plugin-storage-telegram` 검증 본문 |
| Notion page/attach mode 분기 | `notion_storage.rs::NotionStorageAdapter` | `fp-plugin-storage-notion::process_request` |
| Notion rate limit (3 req/s) | `notion_storage.rs` 호출 spacing | `fp-plugin-storage-notion` 정책 본문 (token bucket) |
| openai/claude/anthropic LLM 요청 형식 | 각 `*_adapter.rs` | 각 plugin 본문 (prompt + temperature + max_tokens 등) |
| LLM 응답 파싱 + 에러 처리 | 각 어댑터 | 각 plugin (LLM 응답 도메인 = plugin 책임) |
| fastembed BGE-M3 batch / pooling | `fastembed_adapter.rs` | `fp-plugin-embedding-fastembed` 본문 |

어댑터 잔류 책임 = **외부 SDK 호출 wrap** + **에러 forward**. 어댑터 자체에 LOC 100줄 이하 권장 (현재 telegram_storage.rs 291줄 → 이관 후 ~50줄 예상).

#### capabilities 이관 — plugin manifest 박힘

기존 `ResourceCapabilities` (Phase 92 H5) + `OutboundManifest::modes() / config_keys()` **모두 폐기**. plugin manifest (`fp-plugin.toml`) 에 박힘:

```toml
# fp-plugin-storage-telegram/fp-plugin.toml
manifest_version = 1
id = "fp-plugin-storage-telegram"
name = "Telegram Storage"
version = "0.1.0"
api_version = "1"

[capabilities]
can_upload = true
can_download = true
can_list = false  # bot API 한계
can_delete = true  # 48h 제약
supports_hard_delete = false

[modes]
default = "document"
options = ["document", "text", "channel"]

[config_keys]
required = ["bot_token", "chat_id"]
optional = ["mode"]

[constraints]
upload_max_bytes = 52428800  # 50MB
delete_window_hours = 48

[contributes.step_hook]
phase = "store"  # §3-D step hook 정합
position = "post"
```

host = 매니페스트 파싱 후 `Plugin discovery` 단계 (§3-A #7) 에서 capabilities 카탈로그 생성. plugin 책임 = 매니페스트 선언 + 본문에서 capabilities 정합 강제.

#### telegram 어댑터 + plugin 재배치 (2026-06-17 본질 재정의 3차 정합)

기존 telegram 양쪽 어댑터 (`telegram_storage.rs` 291줄 + `telegram_notify.rs`) = **raw I/O transport 만 잔류**. 도메인 로직 = `fp-plugin-storage-telegram` + `fp-plugin-notify-telegram` 본문 이관:

| 영역 | 어댑터 잔류 (raw I/O) | plugin 이관 (도메인 로직) |
|------|-------------------|--------------------|
| 어댑터 위치 | `adapters/driven/transport/telegram_http.rs` (Bot API HTTP wrap, ~50줄 예상) | `fp-plugin-storage-telegram` + `fp-plugin-notify-telegram` (별도 plugin) |
| 책임 (어댑터) | `send_document(chat_id, file_path) -> Result<MessageId>` / `send_message(chat_id, text) -> Result<MessageId>` / `delete_message(chat_id, message_id) -> Result<()>` / `get_file(file_id) -> Result<Bytes>` | (도메인 로직 부재) |
| 책임 (plugin) | (raw 호출만) | mode 분기 (document/text/channel for storage, alert/event for notify) + sqlite mapping (plugin 단위 sqlite) + 48h delete 검증 + 50MB pre-check + chat_id 정책 + 인증 토큰 검증 |
| 인증 | `bot_token` env 또는 plugin config | plugin manifest `config_keys` |
| capabilities | (어댑터 부재) | plugin manifest `[capabilities]` 박힘 |
| 기존 인프라 활용 | `CLAUDE.local.md` telegram bot (`@reujea_test_bot`) + bridges.json (group_bot=-1003990184767, channel_bot=-1003976785396) — plugin 측 config 박힘 | 동 |
| plugin id | `fp-plugin-storage-telegram` + `fp-plugin-notify-telegram` | (양쪽 plugin 별도 본체, 같은 어댑터 공유) |
| 본 plan trigger | 사용자 발화 2026-06-16 "원격 저장소를 텔레그램 추가 구성 추가" → 2026-06-17 본질 재정의 3차로 어댑터 단순화 |

#### 본 재정의의 plan 위임

- 본 §3-C 본문 = **raw I/O transport 단일 진실원** (2026-06-17 본질 재정의 3차). 기존 §3-C outbound 우산 본문 (2026-06-16) 흡수 + 어댑터 도메인 로직 plugin 이관 의무
- 진입 plan = `transport-flatten-1` (별도 plan, hex-arch-d / settings-db-split-1 와 직교). 본 plan 본진입 = settings-db-split-1 prep-unlock 완료 후 cycle 5+
- 신규 외부 연계 추가 = transport 호출 wrap (수십 줄) + plugin 본문 신설 (도메인 로직)
- outbound-umbrella-1 plan = 본 재정의로 의미 변경 (24 어댑터 manifest impl + 6 port super-trait 박힘 = 모두 폐기 대상). plan 자체는 종결 처리 (history 보존) + transport-flatten-1 이 후속

### 3-D. Plugin Step Hook 인터페이스 (2026-06-17 신규, 본질 재정의 3차)

사용자 발화 = `"외부 plugin이 가공 파이프라인의 스텝에 연결 될 수 있도록 인터페이스 spec 추가"`.

해석 합의 (옵션 1, **전/후 hook**): host 의 가공 파이프라인 각 step 전후에 plugin 이 IPC hook 으로 연결. step 실행 전후 = host broadcast_event → plugin 동기 응답 대기 (timeout) → host 결과 흡수.

#### 3-D-1. 파이프라인 step 정의 (7 단계 + Quarantine 분기)

| step id | 단계 | host 본체 위치 | hook 가능 시점 |
|---------|------|------------|----------|
| `watch` | 파일 감지 | `adapters/driving/watcher.rs` | `pre` (감지 직후, plugin이 ignore 결정 가능) / `post` (큐 적재 후) |
| `preprocess` | 전처리 (PDF/Excel/한글 추출 + 인코딩 감지) | `adapters/driven/preprocessing/preprocessor.rs` | `pre` (입력 검증) / `post` (추출 텍스트 전달) |
| `classify` | 도메인 분류 + 민감도 검사 | host 잔류 (`fp-plugin-classify` 호출) | `pre` (분류 전 전처리 텍스트) / `post` (분류 결과) |
| `chunk` | 청킹 (Fixed/Semantic/Recursive/Adaptive) + 4지표 (SC/BI/ICC/DCC) | `core/domain/chunking.rs` | `pre` (전체 텍스트) / `post` (청크 배열) |
| `embed` | 임베딩 (dense + sparse) | host = `fp-plugin-embedding-*` plugin 호출 | `pre` (청크) / `post` (벡터) |
| `verify` | 검증 (강한 주장 + needs_verification) | `fp-plugin-verify` 호출 | `pre` (가공본) / `post` (검증 결과) |
| `index` | 벡터 DB 색인 (LocalVectorStore 또는 fp-plugin-search) | host = `fp-plugin-search` plugin 호출 | `pre` (벡터+메타) / `post` (색인 완료) |
| `store` | DB 영속 (DocStore + settings.db) + raw 파일 보관 | host 본체 | `pre` (저장 직전) / `post` (저장 완료) |
| `quarantine` (분기) | 격리 (verify 실패 / preprocess 실패 / sensitive 분류) | host 본체 | `pre` (격리 직전, plugin이 alert 발송 가능) / `post` (격리 완료) |

각 step 의 `pre` / `post` 시점에 plugin 의 hook 호출 가능 = **총 16 hook 시점** (8 step × 2 시점 + quarantine 2).

#### 3-D-2. HookPoint trait + plugin SDK

```rust
// core/ports/plugin_hook.rs (신규)
#[async_trait]
pub trait PluginHook: Send + Sync {
    fn step(&self) -> StepId;             // watch / preprocess / classify / chunk / embed / verify / index / store / quarantine
    fn position(&self) -> HookPosition;   // Pre / Post
    fn priority(&self) -> i32 { 0 }       // 같은 step+position 안 정렬 (오름차순)

    async fn invoke(
        &self,
        ctx: &HookContext,
    ) -> Result<HookResponse>;
}

pub enum StepId { Watch, Preprocess, Classify, Chunk, Embed, Verify, Index, Store, Quarantine }
pub enum HookPosition { Pre, Post }

pub struct HookContext {
    pub trace_id: String,
    pub file_id: String,
    pub step: StepId,
    pub position: HookPosition,
    pub payload: serde_json::Value,   // step 별 데이터 (전처리 텍스트 / 청크 / 벡터 / 메타 등)
    pub metadata: HashMap<String, String>,
}

pub enum HookResponse {
    Continue,                                  // 다음 step 진행
    Skip,                                       // 본 step 건너뜀 (예: pre hook 에서 ignore 결정)
    Quarantine { reason: String },             // 격리 분기 진입
    Replace { payload: serde_json::Value },    // payload 교체 (예: pre chunk hook 에서 청킹 전 텍스트 정제)
    Augment { metadata: HashMap<String, String> }, // 메타 추가 (예: post classify 에서 분류 태그 추가)
}
```

#### 3-D-3. plugin manifest contribute (`fp-plugin.toml`)

plugin 이 hook 등록 = manifest 의 `[[contributes.step_hook]]` 박힘:

```toml
[[contributes.step_hook]]
step = "embed"
position = "pre"
priority = 0
timeout_ms = 5000      # host 가 응답 대기 timeout
on_timeout = "continue" # continue / fail / quarantine
```

manifest 박힘 = host 의 `PluginRegistry::discover` 가 자동 등록 + step 실행 시 자동 호출. plugin 코드 변경 부재 = manifest 만으로 hook 등재 가능.

#### 3-D-4. IPC wire (Phase 202 정합 확장)

기존 `IpcMessage::method` 확장 — `"step_hook.{step}.{position}"` 명명 규칙 (메타 룰 24 정합):

```
method = "step_hook.embed.pre"   →   plugin = handle_step_hook(ctx) → HookResponse JSON 반환
method = "step_hook.chunk.post"  →   동일
```

host audit stage = `plugin.{id}.step_hook.{step}.{position}` (메타 룰 24 정합).

#### 3-D-5. hook 실행 순서 + 분기

```
host step 실행 흐름 (예: embed step):

1. host: emit step_hook.embed.pre broadcast
2. plugin (priority asc): handle_step_hook(ctx) 동기 응답
   - Continue: 다음 plugin
   - Skip: embed step 자체 skip
   - Quarantine: 격리 분기 진입
   - Replace: ctx.payload 교체 후 다음 plugin
   - Augment: ctx.metadata 추가 후 다음 plugin
3. host: embed step 본체 실행 (vec_io::embed_batch)
4. host: emit step_hook.embed.post broadcast
5. plugin (priority asc): handle_step_hook(ctx_with_result) 동기 응답
6. host: 다음 step 진행 (verify)
```

#### 3-D-6. 기존 hook (`core/domain/hooks.rs`) 와의 관계

기존 hook = 본 §3-D 의 부분집합 (5 이벤트: file_detected / process_start / process_complete / verify_fail / search_query). 본 §3-D = **16 시점 확장 + 동기 응답 + payload 교체 기능 추가**. 기존 hook = deprecated.md 단방향 위임 + §3-D 로 흡수.

#### 3-D-7. 진입 plan 위임

- 진입 plan = `plugin-step-hook-1` (cycle 5+ 후보, settings-db-split-1 prep-unlock 완료 후)
- 의존 = Phase 202 본진입 완료 ✅ (PluginRegistry::call + broadcast_event 실 구현, lesson 76)
- step-by-step 작업 = (a) `core/ports/plugin_hook.rs` trait 정의 (b) plugin manifest 확장 (c) `PluginRegistry::invoke_step_hook(ctx)` 헬퍼 신설 (d) 가공 파이프라인 8 step 본체에 hook 호출 박힘 (e) integration test (mock plugin step hook)


## 4. ~~plugin 분류 — 28 MCP 도구 매핑~~ (2026-06-17 MCP 완전 폐기, deprecated.md 위임)

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
| **202 ✅ (2026-06-10 B2/B3)** | IPC bus (named pipe / domain socket) + wire 프로토콜 + audit 통합 | **본진입 완료** — fp-plugin-sdk::connection (cross-platform Connection + endpoint_path) + core/plugin/connection_pool.rs + PluginRegistry::{call, broadcast_event} 실 구현. `IpcNotYetImplemented` 삭제 + `NotRunning/IpcTransport/IpcProtocol` 추가. audit stage `plugin.{id}.{method}`. trace_id 자동 prepend. 47 단위/통합 테스트 PASS. lesson 76 |
| **203 partial ✅ (2026-06-10 B4)** | **첫 plugin: fp-plugin-search** placeholder (lesson 16 단계 0) | _rust_module/fp-plugin-search/ 신규 (Cargo.toml + lib.rs + fp-plugin.toml 매니페스트). PLUGIN_ID + CONTRIBUTED_TOOLS 4건 + Plugin trait 골격. 4 단위 테스트 PASS. **본진입(LocalVectorStore + MMR + vec_io 본체 이관)은 다음 세션 대기** |
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

## 14. Phase 207~209 outbound 우산 본진입 단계 (2026-06-17, lesson 78 정합)

본 §3-C outbound 우산 재정의 + lesson 78 step-o1~o6 진행 결과 정합:

### Phase 207 — outbound 어댑터 → plugin 변환

| 단계 | 영역 |
|------|------|
| 207-A | `fp-outbound-storage-*` 5 plugin 변환 (s3 / webdav / network / notion / telegram) — 각 path 의존 어댑터 → IPC plugin binary |
| 207-B | `fp-outbound-llm-*` 7 plugin 변환 (claude / anthropic / openai / gemini / ollama / fallback / chunked-agent) |
| 207-C | `fp-outbound-embedding-*` 6 plugin 변환 (claude / openai / fastembed / fastembed-sparse / local / python-onnx) |
| 207-D | `fp-outbound-notify-*` 2 + `fp-outbound-rerank-*` 3 + `fp-outbound-verify-1` 변환 |

각 plugin = `[[bin]] name = "fp-outbound-*"` 박힘 + `main.rs (fp_plugin_sdk::run::<P>())` 패턴. host 측 = path 의존 → 매니페스트 의존 전환.

### Phase 208 — outbound 우산 UI 자동 폼 + 회귀 가드

- `OutboundManifest::config_keys()` 활용 = settings 표면 자동 폼 생성 (Pipeline 노드별)
- `OutboundManifest::modes()` = mode 분기 UI 자동 노출 (telegram document/text/channel 등)
- 회귀 가드: 신규 outbound 추가 시 = manifest impl 박힘 의무 (super-trait 강제 = lesson 78 sub-pattern 1 정합)

### Phase 209 — IPC bench + 5% 임계

- 같은 프로세스 호출 대비 IPC 오버헤드 측정 (tasty 트레이드오프 선례)
- **5% 회귀 임계** = bench 결과 outbound IPC 호출 대비 in-process 호출 의 평균 latency 차이 5% 초과 시 = 본 plugin 영역 영구 in-process 잔류 결정
- bench 대상 = `fp-outbound-storage-telegram` (sendDocument 50MB 영역) + `fp-outbound-embedding-fastembed` (BGE-M3 64ms/건 영역)

각 단계 종결 시 메타 룰 17 release 재빌드 + 원격 검증 의무 + lesson 신규 entry 박힘 의무.
