---
created: 2026-06-18
phase: 본질 재정의 4차 (cycle 5 진행 중 사용자 합의) — host = 파일 가공만 (검색 분리) + GUI = Processing 탭만 + 6 탭 외부 검색 프로젝트 이관
prd_truth: prd/research/plugin-architecture-2026-06-04.md §0-A (본 cycle 5 host 자율 영역 박힘 의무)
related_lessons:
  - 72 (본질 재정의 1차 = host 가공만 + plugin 분리, tasty 패턴 흡수 메타 룰 20 8건째)
  - 77 (본질 재정의 2차 = outbound 우산 추상화)
  - 79 (본질 재정의 3차 = MCP 폐기 + Plugin Step Hook + raw I/O Output)
  - 65 (Phase 106 GUI 온보딩 4-step, 본 4차로 헤더 카드 + 6 탭 함께 폐기 = 운영 의존 제거)
  - 51/52/72 (메타 룰 22 사용자 정책 경계 합의 패턴)
meta_rules:
  - 메타 룰 22 (사용자 정책 경계 명시 합의) — 본 세션 +1건 (검색 분리 결정 + Processing 유지 범위) = 누적 22건째
  - 메타 룰 30 (spec 본문 phase별 즉시 갱신) — 본 host 자율 영역 plugin-architecture §0-A + webapp-design.md 7→1 탭 광역 재정의 직후 lesson 80 즉시 등재 = 18건째 자기 적용
  - 메타 룰 25 (메타 룰 자기 적용 의무) — lesson 80 등재 = 16건째 자기 적용
  - 메타 룰 20 (외부 흡수 vs 자기 발현) — 본 4차도 자기 발현 패턴 (사용자 발화 = 외부 흡수 무관, 본질 재정의 누적 4차)
user_decisions:
  - "현재 솔루션은 파일을 가공하는 기능만 담당하자."
  - "GUI에는 처리 현황만 유지하고 나머지 기능들은 외부로 이관."
  - AskUserQuestion Q1 → 옵션 4 (검색 프로젝트로 이관) — 별도 외부 검색 솔루션 신설 의미
  - AskUserQuestion Q2 → 옵션 1 (Processing 현 명세 동일 = 4 카드 + 작업 테이블 + 실패 재처리 유지)
---

# Lesson 80 — 본질 재정의 4차 (GUI Headless + 검색 분리)

## 상황

cycle 5 plugin-sdk-1 step-p7 (OutboundManifest 완전 폐기) 진행 중 사용자 발화 2 영역 동시 본질 재정의:

```
"현재 솔루션은 파일을 가공하는 기능만 담당하자.
- GUI에는 처리 현황만 유지하고 나머지 기능들은 외부로 이관."
```

= **본질 재정의 4차** = 1차 (host 가공만 + plugin 분리) → 2차 (outbound 우산) → 3차 (MCP 폐기 + Step Hook + raw I/O) → **4차 (GUI Headless + 검색 분리)**.

## 문제

### 이슈 1: lesson 72/79 본질 재정의 1/3차 후에도 host = "파일 가공 + 검색 + 운영" 혼합

lesson 72 본질 재정의 1차 = host = 파일 가공만. 다만 실제 host 본문 = 파일 가공 + 검색 (vec_io/mmr/search_engine) + 운영 (Settings 5 그룹 + Decision Log + C1 자기학습 + C2 PII 등) + Documents/Topics 검색 등 영역 모두 포함.

lesson 79 본질 재정의 3차 = MCP 폐기 + Step Hook + raw I/O = 어댑터 단순화는 도달했으나 **GUI 본체 변경 부재** = GUI 7 탭 + Tauri 65 commands 그대로 유지.

본 4차 = **GUI 본체 재정의** = Processing 탭만 host 잔류 + 나머지 6 탭 (Documents / Todos / Verification / Topics / Pipeline / Settings) 외부 검색 프로젝트로 이관.

### 이슈 2: 검색 기능 = file-pipeline 본체에 잔류 = 본질 재정의 1차와 정합 부재

lesson 72 §3-B 매핑:
- `core/domain/mmr.rs + vec_io.rs` → `fp-plugin-search` (Phase 207 예정)
- `core/domain/search_engine.rs` (cycle 1 S-1 신규) → 검색 본체

다만 Phase 207 미진입 + 본 host 본문에 검색 잔류 = 본질 재정의 1차 정합 부재 누적.

본 4차 = **검색 자체를 별도 외부 프로젝트로 분리** = file-wiki = 가공 데몬만 + 별도 file-search 프로젝트 = 검색·조회·운영 책임.

### 이슈 3: GUI 65 Tauri commands = host 표면 비대화 누적

cycle 5 mcp-removal-1 plan 완결 = MCP 28 도구 폐기 + Tauri commands 1건만 제거 (commands.rs:1776 get_mcp_tool_catalog_full) = **여전히 64 commands 잔존**. 본 host 표면 비대화 누적 = lesson 79 §개선 1 (MCP 폐기) 와 같은 본질 단순화 vs Tauri commands = 부재.

본 4차 = **Tauri commands 대거 축소** = Processing 진척 표시 영역 (큐 카운트 / 작업 테이블 / 실패 재처리) 만 host 측 잔류 = **5~10 commands 추정**.

## 원인

### 원인 1: lesson 72 본질 재정의 1차의 단계적 미진입

lesson 72 (2026-06-04) = host = 파일 가공만 본질 재정의. 다만 §3-B 매핑 표 = **plugin 이관 대상 도메인 다수 박힘** + Phase 200~209 단계 본 진입 = Phase 202 (cycle 1~5 = Phase 207 미진입).

본질적 한계 = **본질 재정의 1차는 결정 + 매핑 단계, 본 진입 미충족**. cycle 1~5 진행 중 외부 plugin 이관 = **0건** (fp-plugin-storage-telegram = cycle 5 step-p5 = 첫 사례).

해결 방향 = **GUI 본체부터 단순화** = lesson 80 본 4차 = 본질 재정의 1차의 GUI 영역 본 진입.

### 원인 2: 검색 기능의 본체 잔류 누적

검색 = file-pipeline의 가장 큰 단일 도메인 영역 (Phase 1~107 누적):
- core/domain/{vec_io,mmr,search_engine,cross_reference,deduplicator,classifier,verification}.rs
- adapters/driven/{embedding,llm,rerank,verify}/ (4 카테고리 17 어댑터)
- 검색 결과 GUI = Documents 탭 + KG 시각화 + Topics + Verification

본질적 한계 = **검색 = 가공보다 큰 도메인** = file-pipeline 본체 의미가 "가공 + 검색" 혼합 = 본질 재정의 1차 정합 부재 누적.

해결 방향 = **검색 = 별도 외부 프로젝트** (file-search 가칭) + file-wiki = 가공 데몬만 = 본질 재정의 1차 완전 정합.

### 원인 3: GUI 다탭 = 사용자 표면 비대화 vs Phase 106 온보딩 정합 부재

Phase 106 GUI 온보딩 4-step = 사용자 진입 가이드. 다만 7 탭 다수 = 첫 진입 학습 곡선 누적 = competitive-analysis.md §7 기능 과다 진단 (lesson 72 본 결정 직접 트리거).

본질적 한계 = **GUI 표면 = 모든 plugin/도메인 의존 표면**. plugin 단위 contribute (Phase 208 GUI Plugins 탭) 미진입 = host = 모든 GUI 책임 = 표면 비대화.

해결 방향 = **GUI Processing 만 잔류 + 6 탭 검색 프로젝트 이관** = 사용자 표면 최소 + 검색 솔루션 = 별도 표면 (검색 프로젝트 GUI는 별도 결정 영역).

## 개선

### 개선 1: file-wiki (file-pipeline) host = 파일 가공 데몬만 (옵션 4 = 검색 프로젝트 이관)

- **잔류 영역**: watcher + work_queue + preprocess + chunk + classify + verify + index + store + Quarantine 분기 + audit + Plugin Registry (Phase 200~202 본진입 정합)
- **분리 영역**: 검색 본체 (vec_io/mmr/search_engine/cross_reference) + Documents/Topics 검색 + KG 시각화 = **별도 file-search 프로젝트** 신설 의무 (또는 기존 검색 솔루션 흡수)
- **vector_db 영역 잔류 결정**: 가공 결과 = 벡터 색인 의무 (가공 = 색인 단위) → vector_db host 잔류 + 외부 file-search 가 vector_db 조회 (REST/IPC) = read-only 분리

### 개선 2: GUI 본체 재정의 = Processing 탭만 + 6 탭 폐기

| 탭 | 처리 | 잔류/폐기 |
|----|-----|---------|
| **Documents** | 검색 결과 + KG 시각화 + 검색 + 상세 + Metadata 보조 필드 | 🔴 **폐기 → file-search GUI** |
| **Processing** | 큐 현황(4카드) + 교차참조 + 작업 테이블 + 실패 재처리 + verification 흡수 (Phase 107) | ✅ **잔류** (사용자 결정 Q2 옵션 1 = 현 명세 동일) |
| **Todos** | Pending/Completed + 할일 CRUD (settings.db) | 🔴 **폐기 → file-search GUI 또는 별도** |
| **Verification** | pass/fail/warning + 메트릭 + 강한 주장 + 자동 이상 감지 (H1) | 🔴 **폐기 → file-search GUI** (단 자동 이상 감지 audit_anomaly = host 잔류 candidate) |
| **Topics** | 토픽 카드 + 검색/정렬 + 모달 편집 | 🔴 **폐기 → file-search GUI** |
| **Pipeline** | 2컬럼 (사이드바 시뮬레이션 + 메인 4서브탭 + 인스펙터) | 🔴 **폐기 → file-search GUI** (가공 설정 시뮬레이션 = file-wiki Processing 안 흡수 검토) |
| **Settings** | 5그룹 네비 + 카드 6+종 | 🔴 **폐기 + 분기**: 가공 관련 (preprocess/chunk/verify) = file-wiki 최소 settings.json 잔류 / 검색·운영 (C1/C2/Decision Log/PII/MCP는 폐기 완료) = file-search 또는 별도 |

**잔류 GUI = Processing 탭 단일**:
- 헤더 = 감지 ON/OFF + 처리 진척
- 카드 = 큐 카운트 + 가공 완료 / 실패 / Quarantine 4 카드
- 표 = 최근 가공 작업 (file_id + 진행 단계 + 시작/종료 ts)
- 액션 = 실패 재처리 + Quarantine 항목 정정

### 개선 3: Tauri commands 65 → ~10 축소

축소 후보 (Processing 영역):
1. `get_processing_summary` (4 카운트)
2. `get_work_queue` (작업 테이블)
3. `get_recent_audit` (감지/가공 audit_trace 최근)
4. `retry_failed_file` (실패 재처리)
5. `get_quarantine_list` + `release_from_quarantine`
6. `toggle_watcher` (감지 ON/OFF)
7. `get_pipeline_config_summary` (가공 단계 표시만)
8. `get_audit_anomaly` (Phase 92 H1 자동 이상 감지, candidate)

폐기 후보 (65 - 잔류 8 = **57 commands 폐기**):
- 검색 관련 = search / get_document / list_documents / get_index / kg_* (file-search 이관)
- Todo CRUD = list_todos / complete_todo (폐기)
- C1 자기학습 = auto_suggest_from_counters / accept/reject (폐기)
- C2 PII = pii_patterns_* (폐기)
- Settings UI = setup_review / setup_apply / setup_snapshot_* / setup_decision_log_list (폐기)
- Decision Log = get_decision_log (폐기)
- 그 외 카탈로그 / signal / lint = 검색 프로젝트 이관

## 본 세션 수치 요약

| 항목 | 수치 |
|------|------|
| 사용자 발화 결정 | **단일 발화 2 결정** (host 가공만 + GUI Processing만) |
| AskUserQuestion 묶음 | 2 질문 → 옵션 (검색 프로젝트 이관 + Processing 현 명세 동일) |
| spec 갱신 영역 | plugin-architecture-2026-06-04.md §0-A 본질 재정의 4차 (예정) + webapp-design.md 7→1 탭 재정의 (예정) + lesson 80 본 등재 |
| GUI 탭 변경 | 7 → 1 (Processing 단독, -6 탭) |
| Tauri commands | 65 → ~8 (-57 commands, 87% 축소) |
| Settings 그룹 | 5 → 0~1 (가공 최소 settings.json 잔류) |
| 헤더 카드 | 15 → ~4 (Processing 카드만) |
| 분리 대상 신규 프로젝트 | **file-search** (가칭) — 검색 본체 + Documents/KG/Topics/Verification/Settings 검색 운영 GUI |
| 메타 룰 누적 | 22 = 21→22 / 25 = 15→16 / 30 = 17→18 |
| 본질 재정의 누적 | 1차 (host/plugin) → 2차 (outbound 우산) → 3차 (MCP/Step Hook/raw I/O) → **4차 (GUI Headless + 검색 분리)** |

## §확장 — sub-decision 2건 (2026-06-18 후속 사용자 발화)

본질 재정의 4차 직후 사용자 추가 발화로 2 sub-decision 확정:

### Sub-decision A: CLI prompt 논리 제거 + 설정 기반 자동 결정

사용자 발화 = `"TerminalDuplicateResolution과 TerminalSensitiveNotification은 사용자 입력 받는 논리 제거해. 해당 시나리오는 설정으로 적용 할꺼야."`

**변경 영역**:
- `driving/terminal_resolution.rs` + `driving/terminal_sensitive.rs` = **삭제**
- `driving/auto_resolution.rs` + `driving/auto_sensitive.rs` 신규 (config 기반 자동 결정)
- `core/domain/config_models.rs` 신규 struct = `DuplicateResolutionConfig` + `SensitiveResolutionConfig` (28→30 struct)
- `core/domain/models.rs` 신규 enum = `SensitiveAction { Skip / MoveOnly / IndexWithStub }`
- `core/ports/input.rs` 2 trait 시그니처 = **보존** (계약 불변, 어댑터 본문만 변경)
- `shared/lib.rs::build_service` = ConfigDuplicateResolution / ConfigSensitiveNotification 주입
- `shared/test_helpers.rs::ServiceBuilder` 갱신

**자동 결정 규칙**:
```toml
# pipeline.toml 신규 영역
[duplicate_resolution]
sha256_match = "skip"        # SHA-256 동일 = 기존 유지 (default)
semantic_match = "skip"      # 의미 유사 임계 = 기존 유지 (default)

[sensitive_resolution]
default_action = "move_only" # 민감 감지 = 격리만 (가공 부재, 안전 default)
stub_summary_template = "민감 파일: {reason}"
stub_keywords = ["sensitive"]
move_to_sensitive_folder = true
```

**lesson #25 정합** (사용자 입력 vs 코퍼스 신호) = 사용자 입력 제거 + 자동 결정 = lesson #25 본질 정합.

#### Sub-decision A 구현 완료 (cli-prompt-remove-1 plan, cycle 6, 2026-06-18, 직접 처리)

호출처 1곳(build_service_cli) + GUI/테스트 0건 = 회귀 위험 낮음 = 에이전트 위임 부재(직접 처리, cycle 5 규모 기반 위임 정형 정합).

- **step-c1 baseline sniff (코드 변경 0, 메타 룰 18)**: 폐기 대상 terminal_*.rs 호출처 = `cli/main.rs::build_service_cli`(use 2 + 주입 2줄) + `driving/mod.rs` 2줄이 **전부**. GUI app + 통합 테스트 호출처 0건 실측 → 안전 진행.
- **step-c2 config struct**: `config_models.rs`(기존 파일 — "신규" 명세 빗나감, 메타 룰 18 9번째)에 `DuplicateResolutionConfig`(sha256_match/semantic_match) + `SensitiveResolutionConfig`(default_action/stub_summary_template/stub_keywords/move_to_sensitive_folder) + PipelineConfig 2 필드 + default_config + config_metadata UI 2 그룹. `models.rs` `SensitiveAction {Skip/MoveOnly/IndexWithStub}` + `from_config_str` + `DuplicateAction::from_config_str`(미인식 = 안전 default 폴백).
- **step-c3 어댑터 교체**: `auto_resolution.rs`(reason 검사로 exact/semantic 분기) + `auto_sensitive.rs`(SensitiveAction 3분기 Metadata) 신설, terminal_*.rs 삭제. **포트 계약(input.rs 2 trait) 불변** = 어댑터 본문만 교체 (lesson #25 점진).
- **step-c4 DI**: `lib.rs::build_service` = Auto* 주입(cfg.*.clone, GUI/CLI/watcher 공통). `build_service_cli` 래퍼 + stdin IsTerminal 분기 삭제 → 11 호출처 build_service 직접. pipeline.toml 2 영역. **ServiceBuilder + stub.rs Stub* = 테스트용 유지** — 명세는 "ServiceBuilder 갱신"이나 Stub 폴백 변경 시 테스트 회귀(lesson 11/56 — Stub=비대화형 기본 동작) → **명세보다 실측 회귀 위험 우선**(lesson #14 R1 양방향).
- **step-c5 검증 (원격 Linux)**: cargo check --all 경고 0(DocTypeRegistry unused 1건 발견→래퍼 삭제 부수효과, import 제거 후 0) + build --tests --all 통과(PipelineConfig 필드 추가 — 구조체 리터럴 생성처 0건 grep 확인 후 안전) + Tauri check 통과 + nextest **489/489**(bench 제외, 회귀 0). spec: architecture.md flow+트리 2곳 + deprecated.md terminal_*.rs 단일 진실원 엔트리.
- **정형 — "신규 파일" 명세도 사전 grep**: config_models.rs는 명세상 "신규"였으나 이미 존재(domain/mod.rs 등록 완료). step-c2 진입 시 grep 으로 실재 확인 후 "신규 생성" 대신 "기존 파일 추가"로 정정 = 메타 룰 18 의 "신규/기존 추정"도 사전 검증 대상(p7 死코드 검증과 동류).

### Sub-decision B: DB 관련 소스 외부 lib 분리 (B-1 옵션)

사용자 발화 = `"DB 관련 소스도 외부 프로젝트로 분리하자."` → AskUserQuestion 4 옵션 → **B-1 = module-storage-db lib 분리**

**변경 영역**:
- `_rust_module/module-storage-db-api/` 신설 = SettingsRepoPort + 6 sub-trait + VectorDBPort 본체 본문 (cycle 3 prep-2 신규 영역 외부 이관)
- `_rust_module/module-storage-db/` 신설 = SqliteSettingsRepo + LocalVectorStore + DocStore + work_queue 영속 본문 이관
- `adapters/driven/settings/sqlite.rs` = thin wrapper (module-storage-db 위임)
- `adapters/driven/vector_db/local_store.rs` = thin wrapper (module-storage-db 위임)
- `core/domain/vec_io.rs` = trait 의존만 (구현 외부)
- `core/domain/work_queue.rs` = trait 의존만 (영속 외부)
- workspace `_rust_module/Cargo.toml` member 추가 + `adapters/Cargo.toml` path 의존 추가

**옵션 사유**:
- B-3 (file-search 통합) / B-2 (file-store 별도) = 외부 프로젝트 신설 = 광범위 + 운영 복잡도 증가 = 보류
- B-1 = 컴파일 단위 분리만 + host = trait 의존만 = cycle 4 prep-unlock 정합 강화 + 점진 진화 (lesson #25)
- B-4 = settings.db host 잔류 = 정합 분기 = 보류

**현재 정합률** (등급 A = lib 위임 thin wrapper) = 14% → B-1 진입 시 = **70%+ 도달 가능** (DB 본체 추가).

#### Sub-decision B 구현 — step-d1 baseline 차단 발견 (module-storage-db-1, cycle 6, 2026-06-18, 코드 변경 0)

**lesson 79 step-t3 차단과 동형 — plan 전제(깔끔한 lib 분리) 미충족 실측** (메타 룰 18):

- **이관 대상 규모**: sqlite.rs **2292줄**(SqliteSettingsRepo) + local_store.rs **1079줄**(LocalVectorStore) + work_queue.rs 474 + vec_io.rs 97(trait). `DocStore` = **코드 부재**(명세 빗나감, 메타 룰 18 — 명세상 이관 4종 중 1종 비실재).
- **결정적 제약 — core 도메인 타입 깊은 결합**: LocalVectorStore가 `file_pipeline_core::domain::models::{Metadata, Entity, Document, EmbeddingSnapshot, RelationOrigin, SimilarDoc}` + `domain::crossref_optimizer::MinHashIndex`(알고리즘 본체) + `ports::output::VectorDBPort` 의존(21 매칭). sqlite.rs는 `PipelineConfig/ConfigSnapshot/LlmCredential/settings_models/settings_repo`(12 매칭).
- **form-agnostic 원칙 위배**: 기존 16 module(`module-storage` 등)은 `file_pipeline_core` 의존 **0건** = 완전 독립(다른 프로젝트 재사용 가능). DB 본체를 `module-storage-db`로 빼면 두 갈래뿐 — (1) 도메인 타입을 api crate로 이동/복제 = **도메인 단일 진실원(core) 붕괴** + form-agnostic 위배, (2) module-storage-db가 file_pipeline_core 의존 = **form-agnostic 아님** + 별도 workspace 결합/순환 위험. 둘 다 B-1 전제(깔끔한 lib 위임)와 모순.
- **판단**: step-d2~d6 진입 전 **plan 재설계 = host/사용자 결정 필요**(lesson 79 t3 정형 — plan이 미충족 전제 가정 시 코드 변경 전 보고). 후보: (a) B-4(settings.db host 잔류) 회귀, (b) 도메인 타입 일부를 별도 shared-types crate로 추출 후 분리(대공사), (c) B-1 보류 + 다른 P(gui-headless/search-extract) 우선.
- **정형 재확인 — "이관처/결합도 사전 grep"**: step-t3(이관처 plugin 미실재) → step-d1(이관 대상의 core 결합도). 광범위 lib 분리 plan = baseline 에서 **이관 대상의 역의존 grep 의무** = 메타 룰 18 확장(lesson 79 step-t3 정형 8번째 → step-d1 9번째, "분리 가능성 = 결합도 실측").

**사용자 결정 (2026-06-18): shared-types crate 추출 후 분리 (옵션 B)** — 근본 해결. plan 재설계:

- **추출 대상 타입 집합 (정밀 grep 확정)**: `domain::models`(Metadata/Entity/EmbeddingSnapshot/RelationOrigin/DocTypeDef/DocTypeRegistry/Document/SimilarDoc + 연관 enum) + `domain::config_models`(PipelineConfig/CompressionConfig/ConfigSnapshot/LlmCredential + settings 영역) + `domain::settings_models`(전체) + `domain::crossref_optimizer::MinHashIndex`(알고리즘) + `ports::output::VectorDBPort` + `ports::settings_repo`.
- **규모 = core 재편 수준**: models.rs + config_models.rs + settings_models.rs + crossref_optimizer 일부 = core 도메인의 절반 이상. 단순 타입 이동이 아니라 **신규 crate(`module-storage-db-types` 또는 `fp-domain-types`) 추출 → core 가 re-export 의존 → module-storage-db 가 동일 crate 의존(core 비의존)** 구조 재편.
- **재설계 plan 구조 (d1 갱신본)**: d2 = types crate 추출(+ core re-export) → d3 = module-storage-db-api(types 의존) → d4 = module-storage-db 본체 이관 → d5 = adapters thin wrapper → d6 = workspace 등록 + 검증. **순환 검증 의무**(core ↔ types ↔ module-storage-db 단방향).
- **신규 세션 이관 권장**: 타입 추출 = core 전반 영향(모든 도메인 사용처 import 경로 변경) = 광범위 + 깨끗한 컨텍스트 유리. cycle 6 = P0+P1+P2 완결 + P3 baseline+재설계 방향까지 = 종결, 본 구현은 신규 세션.

### 신규 plan 후보 (sub-decision 2건 반영)

| plan id | 영역 | step 수 (추정) |
|---------|------|---------|
| `cli-prompt-remove-1` | terminal_*.rs 폐기 + auto_*.rs + config 영역 + service.rs DI 변경 + 회귀 검증 | 5 step |
| `module-storage-db-1` | module-storage-db-api/impl 신설 + adapters thin wrapper + workspace 등록 + 회귀 검증 | 6 step |
| `gui-headless-1` (직전 lesson 80 §개선 2) | 6 탭 폐기 + Processing 유지 + Tauri commands 65→8 축소 + 헤더 카드 축소 | 8 step |
| `search-extract-1` (직전 lesson 80 §개선 1) | file-search 별도 프로젝트 baseline + REST/IPC 인터페이스 + core 검색 영역 이관 | 10+ step |

= **4 plan 후보 누적** (cycle 5 step-p7 종결 후 진입 결정 영역).

## 후속 트리거

- `gui-headless-1` plan 신설 (6 탭 폐기 + Processing 유지 + 64→8 commands 축소 + 헤더 카드 축소 + Settings 단순화)
- `search-extract-1` plan 신설 (검색 본체 + Documents/KG/Topics/Verification = file-search 별도 프로젝트 baseline + REST/IPC 인터페이스 설계)
- plugin-architecture-2026-06-04.md §0-A 본질 재정의 4차 본문 박힘
- spec/webapp-design.md 본문 7→1 탭 재정의
- spec/architecture.md MCP 폐기 본문 외 추가 host 영역 축소 박힘
- vector_db 영역 결정 = file-wiki 잔류 (가공 = 색인 단위) + file-search read-only 조회 (REST/IPC) — sub-decision 후속 트리거
- audit_anomaly (Phase 92 H1) 결정 = file-wiki 잔류 vs file-search 이관 (candidate sub-decision)
- 검색 분리 시점 결정 = (a) gui-headless-1 + search-extract-1 병행 / (b) gui-headless-1 먼저 + search-extract-1 후속 (별도 cycle)
