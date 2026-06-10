---
created: 2026-06-05
phase: Phase 200 placeholder + Phase 201 PluginRegistry + Phase 202 wire 타입 placeholder + spec 본문 즉시 갱신 (Q2 + Q3 후속, 단일 세션)
prd_truth: prd/research/plugin-architecture-2026-06-04.md (2026-06-05 §2-A 재정의)
related_lessons:
  - 16 (module 워크스페이스 분리 — 골격 시 placeholder 필수, 단계 0)
  - 17 (module-api/impl 분리 시 의존 누수 점검 6단계)
  - 29 (PIPELINE_BASE 통합 — base 결정 함수 다중 정의 회피)
  - 71 (cargo-xwin Linux→Windows cross-build)
  - 72 (본질 재정의 2차 — tasty 패턴 흡수 결정)
  - 74 (본 세션 6 묶음 + 메타 룰 17 강화 정식 승격)
meta_rules:
  - 메타 룰 22 (사용자 정책 경계 합의) — **13건째 (P-4/P-5 원격 빌드 분류) + 14건째 (개발/빌드 모두 원격) + 본 4축 합의 = 누적 17건 도달**
  - 메타 룰 16 차원 B (외부 솔루션 추상화 매칭) — tasty 패턴 직접 흡수 4축 결정
  - 메타 룰 19 (단일 진실원 위임) — plugin-architecture-2026-06-04.md §2-A 단일 진실원
  - 메타 룰 30 (spec 본문 phase별 즉시 갱신) — §2-A + lesson 75 동시 갱신
  - 메타 룰 25 (자기 적용 의무) — 본질 재정의 2차 확정 직후 Phase 200 진입까지 한 세션 진행
external_source: "tasty v0.6.0 (workspace + 별도 프로세스 plugin + tasty-plugin-sdk + tasty-plugin-protocol + tasty-plugin.toml 매니페스트)"
---

# Lesson 75 — Phase 200 진입: binary plugin 모델 확정 + placeholder 통과

## 상황

본 세션 후속 (lesson 73 + 74 + M-3 + G-1f 완료 후) 사용자 신규 트리거:

> **"형제 모듈과 현재 프로젝트는 별도 빌드 하고 현재 프로젝트 바이너리 실행 → plugin 폴더 초기화 → plugin 폴더에 형제 모듈 빌드된 바이너리 추가. 순으로 고도화 하고 싶어."**

이는 plugin-architecture-2026-06-04.md §2-A의 **번들 plugin 모델 (default-members로 cargo build 1회)** 과 충돌. 더 깨끗한 **순수 외부 plugin 모델 (tasty 패턴 정공법)** 채택 결정.

## 문제

§2-A 원안의 한계:
1. **빌드 단위 비분리** — host와 plugin이 같은 cargo 호출. host 빌드 시간이 plugin 수에 비례 증가 (Phase 207 24 어댑터 plugin 진입 시 폭발)
2. **plugin 단위 배포 불가** — 사용자가 필요 기능만 binary 추가 불가
3. **형제 모듈(_rust_module/) 24 멤버와 plugin 24 + 11 = 35 멤버 중복** — 같은 도메인이 두 워크스페이스에 분산
4. **path 의존 → IPC 의존 전환의 단계가 불명확** — Phase 207에서 일괄 전환은 위험

## 원인

원안은 tasty 패턴을 흡수했지만 **번들 빌드** 부분만 채택. tasty의 진짜 가치는 **독립 plugin binary + 런타임 발견**이다. 사용자가 이 점을 정확히 짚어내며 결정 트리거.

## 개선

### 4축 사용자 합의 (메타 룰 22 누적 3건 동시)

| 축 | 결정 | 의미 |
|----|------|------|
| Q1 Plugin 빌드 위치 | **`_rust_module/` 별도 워크스페이스** | 형제 모듈 워크스페이스가 plugin 워크스페이스 = 같은 워크스페이스. file-pipeline path 의존 제거 |
| Q2 형제 모듈 ↔ plugin 관계 | **형제 모듈 = plugin** | `module-llm = fp-plugin-llm`. lib 유지 + bin target 추가. 24 멤버 자연 plugin 화 |
| Q3 런타임 plugin 폴더 | **`PIPELINE_BASE/plugins/` 첫 실행 자동 생성** | lesson 29 PIPELINE_BASE 패턴 재사용. 비어있으면 plugin 0개로 정상 부팅 |
| Q4 Phase 200 진입 | **본 세션에서 §2-A 갱신 + placeholder 진입** | 결정 직후 즉시 자기 적용 (메타 룰 25) |

### Phase 200 placeholder 진입 (lesson 16 단계 0)

`_rust_module/` 에 **2 크레이트 placeholder** 생성 + workspace 등록:

#### `_rust_module/fp-plugin-protocol/`
```rust
// src/lib.rs (placeholder)
pub const API_VERSION: u32 = 1;
pub type ApiVersion = u32;
pub enum EntryKind { Process }
pub enum ProtocolError { ApiVersionMismatch / ManifestParse / Serde }
```

#### `_rust_module/fp-plugin-sdk/`
```rust
// src/lib.rs (placeholder)
pub use fp_plugin_protocol::{API_VERSION, ApiVersion, EntryKind, ProtocolError};
pub const SDK_API_VERSION: u32 = fp_plugin_protocol::API_VERSION;
pub trait Plugin: Send + Sync { fn id(&self) -> &str; }
pub enum SdkError { Protocol / Ipc }
```

#### `_rust_module/Cargo.toml` workspace.members
```toml
members = [
    # ... 기존 24 멤버
    "fp-plugin-protocol",   # 신규
    "fp-plugin-sdk",        # 신규
]
```

#### 검증 (본 세션 사용자 합의 전 단일 cargo check)
- `cargo check -p fp-plugin-protocol -p fp-plugin-sdk` → 42초 PASS
- 단위 테스트 4건 PASS (protocol 2 + SDK 2)
- 본 세션 직후 사용자 결정으로 추가 cargo 명령은 원격 위임 (feedback_remote_build_only 강화)

### `PIPELINE_BASE/plugins/` 런타임 폴더 (Task #20)

`crates/shared/src/config.rs` 변경:

```rust
// ResolvedPaths 구조체 +1 필드
pub struct ResolvedPaths {
    // ... 기존
    /// plugin binary 배치 디렉토리 (Phase 200, plugin-architecture-2026-06-04.md §2-A)
    pub plugins: PathBuf,
}

// resolve_paths 빌더
plugins: std::env::var("PIPELINE_PLUGINS")
    .ok()
    .filter(|s| !s.trim().is_empty())
    .map(PathBuf::from)
    .unwrap_or_else(|| base.join("plugins")),

// create_all에 plugins 포함
for dir in [&self.base, ..., &self.plugins] { fs::create_dir_all(dir)? }
```

ResolvedPaths 직접 생성처 grep — `config.rs` 1곳뿐 → 통합 테스트 영향 0건 (lesson 21/27 회피).

### feedback_remote_build_only 강화 (메타 룰 22 14건째)

본 세션 진행 중 사용자 명시: "개발/빌드는 원격서버에서만 실행". `cargo check` 까지 확장:

- **로컬 금지**: `cargo check / build / test / clippy / run` 모두
- **로컬 가능**: 파일 편집, grep/glob, spec/lesson 갱신, 회귀 자동화 스크립트
- 코드 변경 직후 원격 진입 트리거 요청 워크플로 의무화

본 결정으로 lesson 16 단계 0 검증조차 향후 원격 위임. 본 세션 placeholder cargo check는 본 규칙 확정 전 1회 실행으로 분류 (예외).

## 개선의 메타 가치

### 메타 룰 22 누적 가속 (단일 세션 +3건)

본 세션에서 다음 메타 룰 22 사례가 동시 도달:
- 13건째 (P-4/P-5 원격 빌드 분류)
- 14건째 (개발/빌드 모두 원격)
- 15건째 (binary plugin 4축 합의)

→ 메타 룰 22 누적 17건 도달. 단일 세션 +3건은 본 세션이 첫 사례. 메타 룰 22 자체가 사용자 정책 경계가 빠르게 형성되는 phase 전환점의 시그니처.

### 메타 룰 16 차원 B 명료화

§2-A 원안 vs 본 결정의 차이:
- **원안**: tasty 패턴 흡수 = 워크스페이스 + 별도 프로세스 IPC + 매니페스트 (3축)
- **본 결정**: + **독립 빌드 + 런타임 발견 + plugin 단위 배포** (3축 추가)

→ tasty 패턴 흡수 = 6축 완전 흡수. 메타 룰 16 차원 B "🟢 추상화 매칭 완전" 정확 사례.

### lesson 29 PIPELINE_BASE 패턴 재사용

`PIPELINE_BASE/plugins/`는 lesson 29 (PIPELINE_BASE 통합) 패턴 그대로 재적용. 신규 폴더 도입 시 PIPELINE_BASE 환경 변수 분기 + base.join() fallback의 패턴 자연 확장.

### Phase 207 path 의존 → IPC 의존 전환 단계 명료화

원안에서 Phase 207 일괄 전환은 위험했지만 본 결정으로:
- **Phase 200~206**: file-pipeline path 의존 유지 (host + 어댑터 정적 링크)
- **Phase 207**: 형제 모듈에 bin target 추가 + IPC plugin 변환. file-pipeline 측 path 의존 → 매니페스트 의존 단계 전환

→ Phase 207의 변환 단위가 명확 (한 모듈 = 한 plugin).

## 측정

| 지표 | 변경 |
|------|------|
| `_rust_module/` workspace 멤버 | 24 → **26** (fp-plugin-protocol + fp-plugin-sdk) |
| placeholder 단위 테스트 | 0 → **4 PASS** (protocol 2 + SDK 2) |
| file-pipeline `ResolvedPaths` 필드 | 10 → **11** (plugins 추가) |
| plugin-architecture §2-A 본문 | 번들 모델 → **별도 빌드 + 런타임 배치 모델** |
| 메타 룰 22 누적 | 12 → **15건** (본 세션 +3) |
| lesson | 74 → **75** |

## 본 lesson의 메타 가치

1. **단일 세션 메타 룰 22 +3건** — phase 전환점에서 사용자 정책 경계가 빠르게 형성되는 시그니처 첫 사례
2. **외부 패턴 흡수의 부분 흡수 → 완전 흡수 진화** — 본질 재정의 2차(lesson 72)에서 일부 흡수, 본 lesson 75에서 완전 흡수
3. **사용자 1줄 의도 → 4축 옵션 → 7 영역 동시 처리** (메타 룰 22 비대칭 비용 패턴 강화)
4. **개발/빌드 환경 경계 명시화** — feedback_remote_build_only가 release만 → 모든 cargo로 확장. 메타 룰 22 + 메타 룰 9 결합 사례

## Phase 201 placeholder 진입 (본 세션 후속, Q2 진행)

본 lesson 등재 직후 사용자 "Q2 진행해" 트리거로 Phase 201 placeholder 동시 진입.

### Phase 201-A: PluginManifest (fp-plugin-protocol)

`_rust_module/fp-plugin-protocol/src/lib.rs` 확장:

```rust
pub struct PluginManifest {
    pub manifest_version: u32,
    pub id: String,             // 역도메인 (io.file-pipeline.search)
    pub name: String,
    pub version: String,
    pub api_version: ApiVersion,
    pub permissions: Vec<String>,
    pub event_subscribe: Vec<String>,
    pub entry: EntryKind,       // Process { command }
    pub contributes: Contributes,
}

pub struct ContributedMcpTool {
    pub name: String,
    pub mutates: bool,
    pub category: Option<String>,
    pub cost: Option<String>,
}

pub fn parse_manifest_toml(s: &str) -> Result<PluginManifest, ProtocolError>;
```

`EntryKind::Process { command }` 로 확장 (Phase 200 unit variant → struct variant). `toml` 의존 추가.

### Phase 201-B: PluginRegistry (host `core/plugin/`)

신규 모듈 4 파일:

- `mod.rs` — 재노출 + Phase 진행 docstring
- `permission_gate.rs` — `KnownPermission` 12종 + `PermissionGate` (알 수 없는 권한 reject)
- `handle.rs` — `PluginHandle { manifest, manifest_path, permission, state }` + `PluginState::{Discovered, Enabled, Disabled}`
- `registry.rs` — `PluginRegistry::{discover, enable, disable, call, count, list, get}` + `PluginError` 7 variants

discover 동작:
- `PIPELINE_BASE/plugins/{plugin_id}/fp-plugin.toml` 발견 + 파싱
- api_version 불일치 / 알 수 없는 권한 / 중복 id → 전체 discover 중단 (부분 등록 회피)
- 디렉토리 부재 OK (0개 정상 부팅)

call은 Phase 202 IPC 미진입이라 `Err(PluginError::IpcNotYetImplemented)` 반환.

### 의존 추가

- `file-pipeline/src/crates/core/Cargo.toml` +1 path dep (`fp-plugin-protocol`) + `thiserror`
- core가 처음으로 외부 형제 모듈에 직접 의존하는 사례 — fp-plugin-protocol이 thiserror + async-trait + serde + toml만 의존하는 경량이라 lesson 17 (의존 누수 점검) 통과

### 산출 단위 테스트 (placeholder 단계)

| 모듈 | 테스트 |
|------|------|
| fp-plugin-protocol | api_version_is_1, entry_kind_process_round_trip, manifest_minimal_parses, manifest_full_parses, manifest_invalid_returns_protocol_error (5건) |
| fp-plugin-sdk | sdk_api_version_matches_protocol, plugin_trait_compiles (2건) |
| core::plugin::permission_gate | known_permission_round_trip, gate_grants_known_permissions, gate_rejects_unknown_permission, gate_list_is_sorted (4건) |
| core::plugin::registry | discover_empty_dir_returns_zero, discover_missing_dir_returns_zero, discover_single_plugin_registers, discover_rejects_api_version_mismatch, discover_rejects_unknown_permission, discover_rejects_duplicate_id, enable_disable_transitions, enable_missing_plugin_errors, call_not_yet_implemented (9건) |
| **합계** | **20 단위 테스트 (예정)** — 원격 빌드 검증 위임 |

### Phase 201 진입의 메타 가치

- **메타 룰 25 자기 적용 6건째** (lesson 75 등재 직후 즉시 Phase 201 진입)
- **lesson 17 의존 누수 점검 자기 적용** — core → fp-plugin-protocol 의존 추가 시점에 의존 트리 사전 검증
- **lesson 16 단계 1 진입** — Phase 200(단계 0 placeholder) → Phase 201(단계 1 실 구현 시작)
- **메타 룰 1 sub-rule 1c 자기 적용** — KnownPermission::ALL 단일 진실원 + 신규 권한 추가 시 enum + ALL 동시 갱신 의무 명시

## Phase 202 placeholder 진입 (Q2 → Q3 직전, wire 타입만)

본 lesson §Phase 201 진입 직후 Q2→Q3 트리거로 Phase 202 wire 타입 placeholder 동시 진입. 실 IPC 전송(named pipe / Unix domain socket)은 다음 세션 본진입.

### wire 타입 3종 (fp-plugin-protocol)

```rust
// host → plugin 요청 envelope
pub struct IpcMessage {
    pub trace_id: String,        // Phase 95 audit 통합 자동 prepend
    pub method: String,          // {영역}.{도구명}[.{sub}] (메타 룰 24 정합)
    pub api_version: ApiVersion, // 호환성 게이트
    pub params: serde_json::Value,
}

// plugin → host 응답 envelope (status tag)
pub enum IpcResponse {
    Ok { trace_id: String, result: serde_json::Value },
    Err { trace_id: String, message: String },
}

impl IpcResponse {
    pub fn trace_id(&self) -> &str { ... }  // 양쪽 variant 단축
}

// host → plugin 이벤트 (kind tag, snake_case)
pub enum HostEvent {
    ProcessingStarted { file_id: String },
    ProcessingCompleted { doc_id: String, title: Option<String> },
    QuarantineAdded { file_id: String, reason: String },
    VerifyFailed { doc_id: String, claims: Vec<String> },
    ShutdownRequested,
}
```

method 명명 규칙은 **메타 룰 24 stage 명명 규칙 (`{영역}.{도구명}[.{sub}]`)** 과 동일 패턴 직접 흡수.

### 단위 테스트 추가 +6 (총 26건)

- `ipc_message_round_trip` — IpcMessage JSON round-trip
- `ipc_response_ok_serializes_with_status_tag` + `ipc_response_err_serializes_with_status_tag` — status tag 검증 + trace_id() 단축 접근
- `host_event_processing_completed_round_trip` — kind tag 검증
- `host_event_shutdown_serializes` — unit variant 직렬화 (`{"kind":"shutdown_requested"}`)
- `host_event_all_variants_deserialize` — 5 variant 일괄 역직렬화

### Phase 202 본진입 시 미진입 영역

- `fp-plugin-sdk::connection` — named pipe (Windows) / Unix domain socket 실 연결
- `core::plugin::registry::PluginRegistry::call` — 본 시점 `Err(IpcNotYetImplemented)`, 메시지를 "wire 타입은 정의, 전송은 다음 진입 시점" 으로 명료화
- audit 통합 (`AuditPort::record` + trace_id 자동 prepend)
- broadcast → 매니페스트 `event_subscribe` 필터 → 구독 plugin 전달

## spec 본문 즉시 갱신 (Q3, 메타 룰 30 자기 적용 11건째)

본 lesson 등재 + Phase 202 placeholder 직후 Q3 트리거로 spec 본문 즉시 갱신:

| 위치 | 갱신 |
|------|------|
| `spec/architecture.md` | 헤더 갱신 + §누적 변경 요약(2026-06-05 후속) 신규 — 본 세션 후속 결정 시계열 5 + 4축 + Phase 200~202 산출 + 메타 룰 17 강화/27 정식/30 sub-rule + 상태 전이 표 + Phase 진행 표 |
| `spec/domain-map.md` | 헤더 갱신 + §Plugin 도메인 신규 — Protocol 타입 표 + SDK 표 + Host plugin 모듈 표 + KnownPermission 12종 + PluginError 7 + 런타임 폴더 + Phase 207 어댑터 매핑 |

domain-map.md는 메타 룰 19 단일 진실원 원칙으로 **plugin 인터페이스 + 매니페스트 + 권한 매핑의 단일 진실원**. architecture.md는 결정 맥락(Why)과 상태 전이만 보유.

## 후속 트리거

- **Task #22 원격 빌드 검증** — ResolvedPaths.plugins + Phase 200/201/202 모듈 (총 26 단위 테스트) 사용자 원격 진입 트리거 대기
- **Phase 202 본진입** — `fp-plugin-sdk::connection` 실 IPC + `PluginRegistry::call` 실제 호출 + audit 통합. wire 타입은 본 세션에 완료
- **Phase 207 형제 모듈 plugin 변환** — 각 module-* 에 bin target 추가 + main.rs (`fp_plugin_sdk::run::<P>()`)
- **메타 룰 22 16건째 도달 시점 추적** — 현재 15건, lesson 22의 "단일 세션 +3" 패턴 재발 시 메타 룰 자체 진화 검토
