---
created: 2026-06-17
phase: 본질 재정의 3차 (cycle 4 진행 중 사용자 합의) — MCP 완전 폐기 + Plugin Step Hook 인터페이스 신설 + Output adapter = raw I/O 만
prd_truth: prd/research/plugin-architecture-2026-06-04.md §0-A + §3-A #8 + §3-C + §3-D (본 cycle 4 host 자율 영역 박힘)
related_lessons:
  - 72 (본질 재정의 1차 = host 가공만 + plugin 분리, 메타 룰 20 8건째 tasty 패턴 흡수)
  - 77 (본질 재정의 2차 = outbound 우산 추상화 OutboundManifest super-trait + 25 어댑터 통일)
  - 78 (cycle 2 outbound 본 진입 + lesson #14 R1 5회 사이클 정형)
  - 45 (Notion 도메인 특수성 어댑터 — 본 3차로 plugin 본문 완전 이관)
  - 51 (Mirage Resource capabilities 표준화 — 본 3차로 plugin manifest 박힘 이관)
meta_rules:
  - 메타 룰 22 (사용자 정책 경계 명시 합의) — 본 세션 +1건 (3 영역 옵션 1 × 3 = 본질 재정의 3차 단일 발화) = 누적 21건째
  - 메타 룰 30 (spec 본문 phase별 즉시 갱신) — 본 cycle host 자율 영역 plugin-architecture-2026-06-04.md §0-A + §3-A + §3-C + §3-D 광범위 재정의 직후 lesson 79 즉시 등재 = 17건째 자기 적용
  - 메타 룰 25 (메타 룰 자기 적용 의무) — lesson 79 등재 = 15건째 자기 적용
  - 메타 룰 20 (외부 프로젝트 패턴 흡수) — 본 3차는 외부 흡수 외 사용자 발화 자체에서 본질 재정의 발생 = 자기 발현 패턴 첫 사례 (lesson 72/77 의 tasty/JAMES/Mirage 외부 흡수 → lesson 79 자기 발현)
user_decisions_3_questions: "본 세션 단일 발화로 3 영역 (MCP 제거 / Plugin step hook / Output adapter 재설계) 동시 본질 재정의 결정. AskUserQuestion 3 묶음 발송 + 옵션 1 × 3 채택"
---

# Lesson 79 — 본질 재정의 3차 (MCP 폐기 + Plugin Step Hook + raw I/O Output)

## 상황

cycle 4 진행 중 (prep-unlock 잔여 = SettingsDb 본체 adapters 이전 검증 완료 + 보고 작성 단계) 사용자 발화 3 영역 동시 본질 재정의:

```
"mcp 기능 제거해.
- 외부 plugin이 가공 파이프라인의 스텝에 연결 될 수 있도록 인터페이스 spec 추가하자.
- Output adaptor들은 외부 데이터를 전달만 하고, plugin이 받아서 처리하게 하자"
```

= **단일 발화 3 결정** = 본질 재정의 1차 (2026-06-04 host 가공만 + plugin 분리) → 2차 (2026-06-16 outbound 우산 추상화) → **3차 (2026-06-17 어댑터 단순화 + plugin 단위 책임 확장 + MCP 표면 차단)**.

## 문제

### 이슈 1: lesson 77 outbound 우산 추상화의 한계 발견

본 cycle 1~4 의 outbound-umbrella-1 plan 진행 결과 (lesson 77 + 78 누적):
- 24 어댑터 manifest impl 박힘 완료 (cycle 1 step-o2 partial + cycle 2 step-o2 partial 해소 19 mock manifest 추가 박힘)
- 6 port super-trait OutboundManifest 박힘 완료
- telegram outbound 양쪽 어댑터 신설 = `telegram_storage.rs` 291줄 + `telegram_notify.rs` (cycle 2 step-o3 종결)

**결과 = 어댑터 비대화**: 단일 telegram_storage.rs 안 mode 분기 (document/text/channel) + sqlite mapping + 48h delete 검증 + 50MB pre-check + Telegram Bot API 호출 + capabilities/manifest 정합 = **291줄 단일 파일** = 도메인 로직 + transport 혼재.

사용자 직관 = "어댑터가 너무 많은 책임을 가짐" → 본질 재정의 3차 트리거.

### 이슈 2: MCP 표면 28 도구의 비대화

Phase 102 시점 = MCP 도구 11→25→28 누적 = `mcp_server.rs` 안 28 도구 라우팅 = host 표면 비대화. 외부 (Claude Code MCP 클라이언트) 직접 접근 = 외부 의존 + 정책 경계 불분명. plugin contribute = MCP 도구 매핑 (lesson 72 §4 표) 단방향 의존.

사용자 직관 = "MCP 표면 자체 제거 + plugin contribute 일원화" → 본질 재정의 3차 트리거.

### 이슈 3: 가공 파이프라인 plugin 연결 인터페이스 부재

본 host 본질 = 파일 가공 (8 step + Quarantine 분기). 기존 plugin 연결 = `core/domain/hooks.rs` 5 이벤트 (file_detected / process_start / process_complete / verify_fail / search_query) + HostEvent 5종 (Phase 202 IPC 정합). 다만:
- **시점 부족** = 5 이벤트만 = step 별 pre/post 시점 부재
- **단방향 알림** = HostEvent broadcast only = plugin 응답 흡수 부재 (plugin 결정으로 step skip / quarantine / payload 교체 부재)
- **payload 정형 부재** = plugin 이 step 별 데이터 (전처리 텍스트 / 청크 / 벡터 / 메타 등) 접근 불가

사용자 직관 = "step 전후 hook 인터페이스 spec 신설" → 본질 재정의 3차 트리거.

## 원인

### 원인 1: outbound 우산 추상화의 본질적 한계

lesson 77 = outbound 우산 추상화 (`OutboundManifest` super-trait + 6 port + 24 어댑터 manifest impl) = **추상화 진영의 정점**. 다만 추상화 = 도메인 로직을 어댑터 단에 박는 패턴 강화 (lesson 45 Notion 도메인 특수성 직접 구현의 진화). 본 패턴 = 새 외부 연계 추가 시 어댑터 본문 비대화 가속.

본질적 한계 = **추상화로는 어댑터 비대화 차단 불가**. 어댑터 = transport (외부 SDK 호출) + 도메인 (mode/검증/매핑) 동시 책임 = 패턴 한계.

해결 방향 = **어댑터 책임 단순화 + 도메인 로직 plugin 이관** = lesson 79 본 3차 재정의.

### 원인 2: MCP 도구 카탈로그의 표면 비대화

lesson 51 (Mirage MCP 다차원) + lesson 72 §4 (28 MCP 도구 매핑) = MCP 카탈로그 단일 진실원 시도. 다만 host 가 28 도구 라우팅 = 28 변경점 = 메타 룰 1 sub-rule 1f (함수 분산) 누적 source.

본질적 한계 = **host 표면 = 외부 의존 표면**. plugin contribute 의 외부 노출 = MCP 도구 = host 표면 비대화. plugin = 자율 호출 가능 + host 외부 표면 = Tauri commands 만 정합.

해결 방향 = **host MCP 표면 완전 제거 + plugin step hook 으로 일원화** = lesson 79 본 3차 재정의.

### 원인 3: HostEvent 5종의 표현력 부족

Phase 202 본진입 (lesson 76) = HostEvent 5종 (ProcessingStarted/Completed/QuarantineAdded/VerifyFailed/ShutdownRequested) + PluginRegistry::broadcast_event = 이벤트 단방향 알림. 다만:
- step 별 pre/post 시점 부재 = 8 step × 2 시점 = **16 시점 필요** (현재 5 시점 = 32% 표현력)
- plugin 응답 흡수 부재 = plugin 이 step 결정 (skip/quarantine/replace/augment) 불가
- payload 정형 부재 = plugin 이 ctx (file_id/trace_id/step/position/payload/metadata) 접근 불가

본질적 한계 = **broadcast = 알림 only, 통합 부재**.

해결 방향 = **HookPoint trait + 동기 응답 + payload 교체 + 16 hook 시점** = lesson 79 본 3차 §3-D 신설.

## 개선

### 개선 1: MCP 완전 폐기 (옵션 1, host 표면 차단)

- `shared/mcp_server.rs` 본체 + 25 도구 라우팅 = 삭제
- Tauri commands 안 MCP 호출 영역 = 삭제 (현재 65 commands 중 MCP 의존 비율 정밀 sniff 의무)
- plugin contribute = `[[contributes.mcp_tool]]` manifest 영역 = 폐기 (§3-D step hook 으로 일원화)
- 외부 접근 부재 (Claude Code MCP 클라이언트 차단)

### 개선 2: §3-D Plugin Step Hook 인터페이스 spec 신설

**8 step + Quarantine 분기 + 각 step pre/post = 16 시점**:

| step id | 단계 | host 본체 위치 |
|---------|------|------------|
| watch | 파일 감지 | adapters/driving/watcher.rs |
| preprocess | 전처리 (PDF/Excel/한글 추출 + 인코딩 감지) | adapters/driven/preprocessing/preprocessor.rs |
| classify | 도메인 분류 + 민감도 검사 | fp-plugin-classify 호출 |
| chunk | 청킹 (Fixed/Semantic/Recursive/Adaptive) + 4지표 | core/domain/chunking.rs |
| embed | 임베딩 (dense + sparse) | fp-plugin-embedding-* 호출 |
| verify | 검증 (강한 주장 + needs_verification) | fp-plugin-verify 호출 |
| index | 벡터 DB 색인 | fp-plugin-search 또는 LocalVectorStore |
| store | DB 영속 + raw 파일 보관 | host 본체 |
| quarantine (분기) | 격리 분기 | host 본체 |

**HookPoint trait + HookResponse 5종**:
- `Continue` (다음 step 진행)
- `Skip` (본 step 건너뜀)
- `Quarantine { reason }` (격리 분기 진입)
- `Replace { payload }` (payload 교체)
- `Augment { metadata }` (메타 추가)

**manifest contribute**:
```toml
[[contributes.step_hook]]
step = "embed"
position = "pre"
priority = 0
timeout_ms = 5000
on_timeout = "continue"
```

**IPC wire** = `step_hook.{step}.{position}` (메타 룰 24 정합) + audit stage = `plugin.{id}.step_hook.{step}.{position}`.

기존 `core/domain/hooks.rs` (5 이벤트) = §3-D 흡수 + deprecated.md 위임.

### 개선 3: Output adapter = raw I/O 만 (transport 단순화)

**4 transport 채널**:

| 채널 | 어댑터 위치 | 사용 plugin |
|------|----------|----------|
| HTTP (reqwest) | adapters/driven/transport/http_client.rs | fp-plugin-storage-{s3,webdav,notion,telegram} / fp-plugin-llm-* / fp-plugin-embedding-{claude,openai,fastembed} / fp-plugin-rerank-* / fp-plugin-verify-claude / fp-plugin-notify-{telegram,slack} |
| filesystem | adapters/driven/transport/fs.rs | fp-plugin-storage-network / fp-plugin-embedding-{local,python-onnx} / fp-plugin-storage-zstd |
| stdio (subprocess) | adapters/driven/transport/stdio.rs | fp-plugin-llm-claude (CLI) / fp-plugin-embedding-python-onnx / fp-plugin-llm-ollama |
| sqlite | adapters/driven/settings/sqlite.rs (cycle 4 prep-unlock 종결, host 잔류) | host 본체 |

**도메인 로직 9 영역 plugin 이관**:

| 영역 | 기존 어댑터 | 이관 plugin |
|------|--------|---------|
| telegram mode 분기 (document/text/channel) | telegram_storage.rs | fp-plugin-storage-telegram::process_request |
| telegram_message_map sqlite 매핑 | telegram_storage.rs + settings_db.rs | fp-plugin-storage-telegram 자체 sqlite |
| telegram 48h delete 검증 | telegram_storage.rs::delete | fp-plugin-storage-telegram 정책 |
| telegram 50MB upload pre-check | telegram_storage.rs::upload | fp-plugin-storage-telegram 검증 |
| Notion page/attach mode | notion_storage.rs | fp-plugin-storage-notion |
| Notion rate limit (3 req/s) | notion_storage.rs | fp-plugin-storage-notion (token bucket) |
| LLM 요청 형식 (prompt+temperature+max_tokens) | 각 *_adapter.rs | 각 plugin |
| LLM 응답 파싱 + 에러 처리 | 각 어댑터 | 각 plugin |
| fastembed BGE-M3 batch / pooling | fastembed_adapter.rs | fp-plugin-embedding-fastembed |

**OutboundManifest super-trait 폐기** + `capabilities` = plugin manifest `[capabilities]` 박힘 (`fp-plugin.toml`).

어댑터 잔류 책임 = SDK 호출 wrap + 에러 forward only. 어댑터 LOC 100줄 이하 권장 (telegram_storage.rs 291줄 → ~50줄 예상).

## 본 세션 수치 요약

| 항목 | 수치 |
|------|------|
| 사용자 발화 결정 | **단일 발화 3 결정** (MCP 폐기 / Step hook / raw I/O) |
| AskUserQuestion 묶음 | 3 질문 (옵션 3 × 3 = 9 옵션) → 사용자 = **옵션 1 × 3 채택** |
| spec 갱신 영역 | plugin-architecture-2026-06-04.md §0-A 신설 + §3-A #8 폐기 표시 + §3-C 본문 재정의 + §3-D 신규 본문 (160줄 추정) + §4 폐기 표시 |
| 폐기 대상 plan | outbound-umbrella-1 (의미 변경, history 보존) |
| 신규 plan 후보 | transport-flatten-1 + plugin-step-hook-1 + mcp-removal-1 |
| 메타 룰 누적 | 22 = 20→21 / 25 = 14→15 / 30 = 16→17 |
| host 자기 적용 | spec 광범위 재정의 직후 lesson 79 즉시 등재 (메타 룰 30 17건째) |
| worker 22 상태 | cycle 4 진행 중 (SettingsDb 본체 adapters 이전 검증 완료 + 보고 작성), 본질 재정의 3차 인지 = cycle 4 종결 후 정합 |

## 후속 트리거

- `mcp-removal-1` plan 진입 = mcp_server.rs + 25 도구 + Tauri MCP 의존 commands 삭제 + 회귀 검증
- `transport-flatten-1` plan 진입 = 24 어댑터 본문 단순화 (도메인 로직 plugin 이관) + telegram 어댑터 291→~50줄
- `plugin-step-hook-1` plan 진입 = `core/ports/plugin_hook.rs` trait + PluginRegistry::invoke_step_hook + 8 step 본체 hook 호출 박힘 + manifest 확장 + integration test (mock plugin step hook)
- `outbound-umbrella-1` plan 종결 처리 (의미 폐기 + spec/deprecated.md 위임)
- spec/architecture.md MCP 폐기 본문 갱신 + 헤더 수치 동기화 (28 MCP 도구 → 0)
- spec/webapp-design.md MCP 카탈로그 카드 (Settings 운영 그룹 5 카드 중 🧰 MCP 도구 분류) 폐기 + Settings 4 카드 재구성
- §4 MCP 도구 매핑 표 → step hook 매핑 표 신설 (11 plugin × 16 hook 시점 = sparse matrix)

## step-m1 sniff 실측 (cycle 5, 2026-06-17, 코드 변경 0)

메타 룰 18 6번째 적용 — baseline 추정 사전 grep 검증:

- **실제 MCP 도구 = 37개** (`match arm => self.handle` 37건. baseline "25/28 도구" 빗나감 = grimoire/optimize 등 후속 추가분 누적). mcp_server.rs 2139줄.
- **MCP 외부 코드 의존 = 단 3곳** (광역 grep 21파일 대부분 주석/노이즈, 실 의존만):
  1. `shared/cli.rs:366` — `McpState {...}` MCP 서버 실행 진입점 (가장 큰 블록)
  2. `modals/cli/src/main.rs:955` — `McpState {...}`
  3. `modals/app/src/commands.rs:1775` — `get_mcp_tool_catalog_full` (`mcp_tool_catalog_full()` 호출, Tauri 65 commands 중 MCP 의존 유일 1개)
- **cli/tests MCP 의존 0 / ui frontend invoke MCP 0** (매칭 없음 = 회귀 위험 낮음).
- **webapp-design.md MCP 산재 = 8곳** (L17 도구 수 / L40 AI 도우미 / L68·102 Settings 5카드 / L117 PII / **L118 🧰 MCP 도구 분류 카드** / L159 HTTP fetch 다이어그램 / L170 MCP 카탈로그 4단계).
- **삭제 대상 카탈로그**(step-m2~m5 명세): mcp_server.rs 본체 + cli.rs/cli-main McpState 블록 2 + commands.rs get_mcp_tool_catalog_full + lib.rs `pub mod mcp_server` + webapp-design 8곳 + architecture 본문.
- **권장**: step-m2(본체 삭제)/m3(Tauri)는 광범위 + 컨텍스트 보호 = 에이전트 위임 + 직접 재검증 (cycle 4 정형 정합).

## step-m2 본체 삭제 (cycle 5, 2026-06-17, 에이전트 위임 + 직접 재검증)

- **삭제**: mcp_server.rs(2139줄) 파일 + lib.rs `pub mod mcp_server` + cli.rs/cli-main `Commands::Serve` enum variant + 핸들러 전체(McpState 20+ 필드 구성 + rmcp stdio serve) + auto_init serve 안내 + cli.rs unused `use tracing::info`.
- **검증**: workspace cargo check 0/0 + nextest **502/502** (520→502 = MCP 내부 단위 테스트 18건 삭제분, 회귀 0). **테스트 수 감소 = 삭제 의도 정합, 회귀 아님** (lesson #14 R1 — 감소 원인 명시 의무).
- **app(Tauri) = 의도된 1 에러**: commands.rs:1776 `get_mcp_tool_catalog_full` → 삭제된 `mcp_tool_catalog_full()` 참조 = E0433. step-m3 의존(이번 미수정). step-m1 sniff 가 예측한 유일 Tauri 의존점과 정확히 일치 = 사전 grep 정확도 입증.
- **TC.m2-mcp-server-deleted** = `! test -f crates/shared/src/mcp_server.rs && grep -c "pub mod mcp_server" crates/shared/src/lib.rs` = 0 의무.
- **TC.m2-no-mcpstate** = `grep -c "McpState\|mcp_server" crates/shared/src/cli.rs modals/cli/src/main.rs` = 0 의무.

## step-m3 Tauri MCP 제거 (cycle 5, 2026-06-17, 직접 편집 — 범위 작아 위임 불필요)

- **제거**: commands.rs `get_mcp_tool_catalog_full`(doc 주석 포함 함수) + app/main.rs:142 command 등록. ui/ MCP invoke 0 확인(step-m1 정합 — frontend 변경 불필요).
- **app 1 에러 해소**: step-m2 잔존 E0433(commands.rs:1776) 제거 → **Tauri cargo check Finished 0**. step-m1 sniff 가 예측한 유일 Tauri 의존점 정합.
- **검증**: Tauri check 0 + workspace check 0 + nextest **502/502**(회귀 0 — Tauri 함수 제거는 nextest 무영향).
- **MCP 코드 완전 제거 완료**: mcp_server.rs 본체(m2) + CLI Serve(m2) + Tauri command(m3). 남은 = webapp-design 문서(m4) + architecture 수치 28→0(m5) = **문서 영역만**.
- **위임 판단 정형**: 변경 2곳(함수 1 + 등록 1) = 소규모 = 직접 편집(에이전트 위임 불필요). cycle 4/5 위임은 광범위(SettingsDb 86메서드 / mcp_server 2139줄) 영역만. **규모 기반 위임 결정** = 컨텍스트 효율 정합.
- **TC.m3-no-tauri-mcp** = `grep -c "get_mcp_tool_catalog_full\|mcp_tool_catalog" modals/app/src/commands.rs modals/app/src/main.rs` = 0 의무.

## step-m4 webapp-design MCP 카드 폐기 (cycle 5, 2026-06-17, 직접 편집 — 문맥별 정밀 정정)

- **정정 8곳**: L12 소개문 / L17 도구 수(36→폐기) / L40 AI 도우미 MCP 안내 / L68 Settings 카드 7→6 / L102+108 운영 5→4 카드 / L117 PII 'MCP 응답' 제거 / **L118 🧰 MCP 도구 분류 카드 행 삭제** / L159-160 다이어그램 MCP fetch 폐기 / L169 메타룰13 표 MCP 카탈로그 행 삭제.
- **위임 판단 정형 보강**: 명세는 "에이전트 위임"이었으나, 8곳이 한 파일(216줄) 내 산재 + 각 문맥(소개/수치/카드표/네비/다이어그램) 상이 = **문맥 일관성 = 정확성 우선 직접 처리**. 규모만이 아니라 **문맥 일관성도 위임 vs 직접 판단 축** (cycle 5 정형 확장).
- **폐기 표시 vs 실 참조 구분 정형**: grep MCP 잔존 = 7곳이나 전부 "MCP 폐기/이관" 표시 문구(히스토리 보존). **실체(카드/도구/다이어그램) 0**. 회귀 TC 는 실 참조만 0 검증, 폐기 표시는 허용 (lesson 38 단방향 누락 회피 정합).
- 코드 변경 0(문서만) = 빌드 검증 불필요. 남은 step-m5 = architecture 수치(28 MCP 도구 → 0) + 전체 빌드(workspace + Tauri + nextest) 최종 검증.

## step-m5 architecture 정정 + 전체 빌드 (cycle 5, 2026-06-17, 에이전트 위임 + 직접 재검증) — mcp-removal-1 완결

- **architecture.md MCP 85건 매칭 → current vs history 분리 정정**: current 수치/구조(host 잔류 경계 MCP server / 28→0 / get_mcp_tool_catalog_full / 헥사고날 다이어그램 McpState / CLI serve / 디렉토리 mcp_server.rs) = strikethrough + 폐기 표시. 과거 Phase 로그(Phase 92 H3 / 94 / 95 trace_id / 102 optimize / 80 도구 추가) = **히스토리 보존**. 헤더 갱신.
- **current vs history 구분 정형 확립**: "지금 시스템이 이렇다" = 정정 / "Phase N에서 이렇게 했다" = 보존. 광범위 문서(85 매칭)에서 일괄 삭제 = 히스토리 손실 위험 → 판단 기준 명시 후 에이전트 위임. **lesson 38(단방향 누락) + 히스토리 보존 동시 정합**.
- **빌드 3종 직접 재검증**: workspace check 0 + Tauri check 0 + nextest **502/502**(명세 필터 not bench_scale, 회귀 0). 에이전트가 `not test(/bench/)` 다른 필터로 1 FAIL(bench_crossref_variants 7s) 보고 → flaky 판정 → **직접 명세 필터 재검증으로 502 전부 PASS 확인**(비재현 = feedback_bench_3runs 단일 실행 캐시 편향 정합). 타 PASS/FAIL 단언 비신뢰 = 직접 재검증 정형(lesson #14 R1).
- **mcp-removal-1 plan 완결**(5 step): m1 sniff(영향 3곳 정밀) → m2 본체(2139줄) → m3 Tauri → m4 webapp → m5 architecture+빌드. **MCP 코드 실체 0 + 문서 current 0**(폐기 표시/히스토리 보존). 본질 재정의 3차 1/3 plan 완결.
- **TC.m5-mcp-current-zero** = architecture.md MCP current 참조(strikethrough/폐기 표시 제외)가 0 — host 잔류 경계 "MCP server"는 `~~...~~` 폐기 표시만 허용.

## transport-flatten-1 진입 + step-t1 raw_transport skeleton (cycle 5, 2026-06-17)

본질 재정의 3차 2/3 plan. "Output adapter = raw I/O 만" 구현 — OutboundManifest(도메인 메타 우산) 폐기 + 4 transport 채널 + 도메인 로직 plugin 이관.

- **step-t1 (직접 처리, skeleton 단계 영향 0)**: `core/ports/raw_transport/mod.rs` 신설(104줄) — 4 trait: `HttpTransport`(send→HttpResponse) / `FilesystemTransport`(read·write·delete) / `StdioTransport`(invoke) / `SqliteTransport`(get·put·delete) + `TransportMeta`(source_id+mode 불투명 전달) + `HttpResponse`. ports/mod.rs 등록. outbound/mod.rs OutboundManifest 폐기 마킹(DEPRECATED, 완전 폐기는 t6).
- **transport 채널 = 도메인 무관 raw I/O 정형**: HttpTransport 는 method/url/headers/body 전부 plugin 구성 + raw byte 반환(multipart/JSON 파싱 = plugin). SqliteTransport 는 raw key-value(스키마/48h/50MB 의미 = plugin). **transport 는 불투명 전달, 도메인 의미 0** = 헥사고날 raw I/O 경계 정형.
- **검증**: cargo check 0/0 + nextest 502/502(회귀 0). impl 부재 = 기존 코드 영향 0 (점진 lesson #25 — outbound 폐기 마킹도 기존 6 port impl 무영향).
- **위임 판단**: step-t1 = skeleton(신규 trait 정의 104줄) = 직접 처리. t3(24 어댑터)/t4(9영역 plugin) = 광범위 = 에이전트 위임 예정(lesson 79 정형).
- **TC.t1-raw-transport-skeleton** = `grep -c "trait HttpTransport\|trait FilesystemTransport\|trait StdioTransport\|trait SqliteTransport" core/src/ports/raw_transport/mod.rs` = 4 의무.

## step-t2 transport 구현체 신설 (cycle 5, 2026-06-18, 에이전트 위임 + 직접 재검증)

- **선행 async 전환 (사용자 결정)**: raw_transport 4 trait sync → `#[async_trait]` (기존 core port output.rs 패턴 정합). **이유**: 기존 어댑터(notion/telegram/openai)가 전부 async reqwest → sync trait면 blocking client가 tokio 런타임과 충돌. **신규 trait 시그니처는 실 사용처(기존 async 어댑터)에 맞춰 결정** = skeleton 단계에 추정 말고 t2 진입 시 실측 결정 정형(메타 룰 18 정합 — sync 추정 빗나감 사전 차단).
- **신설 4 구현체**: ReqwestHttpTransport(87) / TokioFsTransport(72) / TokioStdioTransport(90) / RusqliteTransport(130, spawn_blocking + table 식별자 영숫자 검증 = SQL injection 방지). 전부 ~100줄 이하(sqlite만 130 = 검증 로직). `new()` 생성자.
- **도메인 로직 0 코드 실증**: multipart/48h/50MB/page mode/telegram_message_map 매칭 = **전부 주석(금지 명시)**, 실 코드 0. unwrap 0. transport = 불투명 byte 전달만 = raw I/O 경계 코드 레벨 보존.
- **SqliteTransport 실구현 채택**: 에이전트가 조건부 stub 명세를 실구현으로 판단(rusqlite 경량 KV = step-t4 이관 부담 감소). raw KV 스키마라 telegram_message_map 다중 컬럼과 형태 상이 → step-t4 plugin 재사용 vs 자체 스키마 = 결정점 표시.
- **검증**: cargo check 0/0 + nextest **509/509**(502+7 신규 transport 테스트, 회귀 0).
- **TC.t2-transport-impls** = `ls adapters/src/driven/transport/*.rs | wc -l` ≥ 5 (mod + 4 구현체).
- **TC.t2-no-domain-logic** = transport/*.rs 의 multipart/JSON parse 실 호출(주석 제외) = 0 의무.

## step-t3 sniff BLOCKED — plan 전제 미충족 발견 (cycle 5, 2026-06-18, 코드 변경 0)

**메타 룰 18 최강 사례** — 사전 grep 1회가 27 어댑터 깨짐 + 헛수고를 차단. plan 전제 3건 미충족 실측:

1. **수치 빗나감**: OutboundManifest impl = **27건**(baseline "24"). 어댑터 storage 7/llm 9/notify 4 (baseline 5/7/2).
2. **plugin = 외부 바이너리 IPC (도메인 로직 이관처 미실재)**: `core/plugin/registry.rs::call` → `pool.get_or_connect` (connection IPC, NotRunning 분기). plugin = `PIPELINE_BASE/plugins/` 외부 바이너리(manifest 스캔). **`fp-plugin-protocol`/`fp-plugin-sdk` crate 미존재**. → 도메인 로직(telegram 50MB/48h, multipart)을 "plugin 본문 이관"하려면 별도 외부 바이너리 작성(Phase 200~209 미완) 선행 필요. **t3 도메인 로직 todo!() 마킹 시 = 기능 소실 + nextest 깨짐 = 회귀 0 불가** (step 의존성 모순).
3. **module 위임 thin wrapper**: network_storage(93줄) 등 다수 = `module_storage`(외부 _rust_module crate) 위임 → reqwest/std::fs 직호출 부재 → **RawTransport 교체 실익 0(중복 추상화)**. telegram/notion 직호출은 multipart 직렬화 신작성 = t4 이관 전 **헛수고**.

**판단**: t3 "27 어댑터 RawTransport 배선 교체"는 순효익 불명확(module 위임=실익0, 직호출=헛수고) + plugin 이관처 미실재. **host plan 재설계 위임** (사용자 결정). 코드 변경 0 = sniff 보고만.

**정형 — plan 전제 사전 검증 의무 (메타 룰 18 확장)**: 광범위 plan step 진입 전 = (a) baseline 수치 grep 실측 + (b) **이관 대상처(plugin/module) 실재 여부 grep** + (c) step 의존성 모순(빼면 갈 곳 있나) 검증. 셋 중 하나라도 미충족 = 코드 변경 전 host 보고. cycle 5 t3 = "단순 이전 추정 → 인프라 미완 실측"의 8번째 빗나감 = plan 자체가 미완 인프라를 전제한 첫 사례.

## transport-flatten-1 종결(t3~t6 skipped) + plugin-sdk-1 step-p1 baseline 검증 (cycle 5, 2026-06-18, 코드 변경 0)

- **host 재설계(옵션 B)**: transport-flatten-1 t3~t6 = skipped(history 보존). t1(raw_transport skeleton 104줄)+t2(4 구현체)는 실 산출 자산 유지. 신규 plugin-sdk-1(8 step) 위임.
- **step-p1 baseline 검증 = 메타 룰 18 plan 첫 step 정식화 (정형의 plan 구조 흡수)**: "plan 전제 사전 검증 의무" 정형이 신규 plan 의 step-p1 으로 정식 편입 = 정형이 절차로 진화한 첫 사례.
- **결정적 발견 (t3 차단 근거 정정)**: fp-plugin-protocol(405줄: IpcMessage/PluginManifest/HostEvent/IpcResponse) + fp-plugin-sdk(connection.rs 365줄 cross-platform IPC + Plugin trait id/handle_request/shutdown) + fp-plugin-search(69줄 선례) = **전부 `_rust_module/`에 이미 존재 + workspace 등록 + core/cli 의존 연결**(Phase 200~203 본진입, lesson 76). → **step-p2/p3/p4(신설/등록) = 이미 완료 = 검증만**. 실 잔여 = p5(telegram plugin 본문)/p6(단순화)/p7(폐기)/p8(spec).
- **메타 교훈 — grep 범위 누락도 사전 검증이 차단**: cycle 5 step-t3 차단 시 "fp-plugin-protocol/sdk 미존재" 근거 = **`src/crates/`만 grep + `_rust_module/` 누락**한 오판. t3 차단 결론(코드 변경 회피) 자체는 옳았으나 근거가 부정확. **step-p1 baseline 검증이 정정** = 메타 룰 18 의 진가 = "추정 빗나감"뿐 아니라 **"이전 검증의 범위 누락"도 다음 검증이 잡는다**. grep 대상 디렉토리 범위(src vs _rust_module 외부 workspace) 명시 의무 = 정형 보강.
- **TC.p1-fp-plugin-crates-exist** = `ls _rust_module/ | grep -c "fp-plugin-protocol\|fp-plugin-sdk"` = 2 (이미 존재 확인 — 신설 아님).

## step-p5 telegram plugin 본문 신설 (cycle 5, 2026-06-18, 에이전트 위임 + 직접 재검증)

- **신설**: `_rust_module/fp-plugin-storage-telegram` crate (lib.rs 354줄 + Cargo.toml + fp-plugin.toml + workspace.members 등록). 첫 실 도메인 로직 plugin(fp-plugin-search placeholder 다음).
- **도메인 로직 이관**: 50MB pre-check + 48h delete window(chrono) + mode 분기(document/channel multipart / text sendMessage) + telegram_message_map sqlite(rusqlite 직접) + parse_send_response. telegram_storage.rs 원본 로직 self-contained 이관.
- **Plugin trait 미완 반영 정형**: SDK `Plugin` trait = 현재 `id()`만 정의(handle_request 미정의). plugin 도 id()만 trait 구현 + 도메인 로직 = **pub async fn 본문**(SDK 완성 시 trait 승격 예정 주석). **미완 인프라 위에 본문 선준비** = lesson 76 placeholder 정형 계승 (골격 + 본문, trait 연결은 SDK 진화 후).
- **plugin 독립성 정형**: file-pipeline core/adapters 의존 0(fp-plugin-sdk + workspace crate만) = IPC 경계. reqwest multipart feature 추가 + rusqlite 0.31 bundled 명시(workspace dep 부재 시 crate 명시). `#![forbid(unsafe_code)]` + unwrap 0.
- **검증**: cargo check 0/0 + `cargo test --lib` **9 passed**(1 ignored live_upload) + file-pipeline 본체 영향 0.
- **직접 재검증 정형 보강 — 의심도 실측으로 해소**: 초기 `cargo test`(--lib 없이) "0 tests" + `sed` 범위 밖 의존 누락으로 reqwest/rusqlite 부재 의심 → `--lib` 명시 재실행 9 passed + Cargo.toml 전체 grep 으로 의존 실재 확인. **에이전트 보고 정확 입증**. lesson #14 R1 = 타 보고 비신뢰뿐 아니라 **자기 의심도 실측으로 해소**(추측 단언 금지 양방향).
- **step-p6 결정점(에이전트 Q1~Q3)**: download/list plugin 이관 여부 / mcp_tool 명명 메타 룰 24(`storage.upload`) 정합 / SDK handle_request 정의 시 IpcMessage.method 디스패처. = host 명시 대기.
- **TC.p5-plugin-independent** = `grep -c "file-pipeline\|file_pipeline" fp-plugin-storage-telegram/Cargo.toml` 의 실 dependency 항목(description/주석 제외) = 0.

## step-p6 telegram_storage 단순화 (cycle 5, 2026-06-18, 에이전트 위임 + 직접 재검증)

- **단순화**: telegram_storage.rs **331→105줄(-68%)**. 제거: 50MB/48h 상수 + TELEGRAM_MAP_SCHEMA + parse_send_response/open_db/api_url + upload/download/list/delete 본문(multipart/sendMessage/getFile/sqlite/48h) + reqwest/rusqlite use + client/db_path 필드. 도메인 메서드 4 = `delegated!` macro = plugin io.file-pipeline.storage-telegram 위임 bail stub. 잔류: struct 최소 + new + is_configured + capabilities + OutboundManifest(step-p7 영역).
- **사전 확인 = 미활성 어댑터 안전 정형**: telegram_storage = build_service/shared 미활성(실 사용처 0, 단위 테스트 0) → 단순화 런타임 손실 0. **step-t3(활성 어댑터 광역)과 결정적 차이** = 미연결 어댑터는 단순화 안전. 사전 grep(build_service 활성화 여부)으로 안전 경계 확정.
- **아키텍처 제약 반영 (Q3 default 정정)**: host default Q3 = "adapter → PluginRegistry::call IPC 디스패처"였으나, 사전 grep 으로 **어댑터=PluginRegistry(core) 접근 불가**(adapters→core registry 경로 부재) 확인 → IPC 디스패처 대신 **bail stub + 위임 주석**(SDK handle_request 미완 + registry 경로 부재 = 과도기). **host default 도 사전 grep 으로 검증 = 메타 룰 18 양방향**(baseline뿐 아니라 host 가이드도 실측 검증).
- **검증**: workspace check 0 + nextest **509/509**(flaky bench 명세 필터 비재현, 직접 실측) + Tauri 0.
- **step-p7/p8 결정점(에이전트 Q1~Q3)**: OutboundManifest 폐기 시 어댑터 manifest → plugin manifest 단일화 흡수 / RemoteStoragePort impl 자체 향후 제거 + PluginRegistry 라우팅 대체(stub=과도기) / slack·webdav 등 미활성 어댑터 동일 단순화 포함 여부. = host 명시 대기.
- **TC.p6-telegram-no-domain** = telegram_storage.rs 의 multipart/rusqlite/reqwest::Client/50MB 상수 실 코드(주석 제외) = 0.

## step-p7 OutboundManifest 완전 폐기 (cycle 6, 2026-06-18, 에이전트 위임 + 직접 재검증) — transport-flatten-1 t6 흡수

본질 재정의 3차 2/3 plan(transport-flatten-1)이 t3 차단으로 skipped 되며 떠넘긴 **OutboundManifest super-trait 완전 폐기**를 plugin-sdk-1 step-p7 이 종결. cycle 5 worker 가 in_progress 마킹 후 코드 변경 0 으로 종료 → cycle 6 재개.

- **사전 baseline 검증 (메타 룰 18 plan step-p1 정형 계승)**: 폐기 전 grep 으로 **OutboundManifest = 호출처 0건의 死 super-trait** 실측 — `OutboundCategory::` 사용은 전부 impl 본문 내부(category() 메서드), `dyn/as OutboundManifest`·외부 `.config_keys()`/`.modes()` 호출 0건. → 순수 삭제(타입/trait 깨짐 위험 0) 확정 후 진입. **"광범위 삭제 전 = 진짜 死코드인지 호출처 grep" = step-t3 차단 정형의 양성 버전**(차단이 아니라 안전 진행 근거).
- **폐기 범위 4단계**: (1) output.rs 6 port super-trait bound 제거(LLMPort/RemoteStoragePort/EmbeddingPort/NotificationPort/VerificationPort/RerankerPort 의 `: OutboundManifest +`) (2) 어댑터/service/cached_llm impl **32건** 제거 (3) 통합·벤치 테스트 impl **21건** 제거 (4) `core/ports/outbound/` 디렉토리 + `ports/mod.rs::pub mod outbound` 등록 해제. **총 53 impl 블록**.
- **위임 판단 정형 (cycle 5 정형 적용)**: 53 블록이 41 파일 산재 + 패턴 일정(id/category/capabilities 3 메서드) = **광범위 + 기계적** = 에이전트 5분할 위임(adapters 3 그룹 + 테스트 2 그룹). service.rs/cached_llm.rs(테스트 모듈 내부 = 신중) + output.rs(6 bound = 핵심) = 직접 처리. **규모 기반 위임 + 핵심/신중 영역 직접 = cycle 5 정형 계승**.
- **검증 (remote-build-only 정합, 원격 Linux 172.16.13.45)**: `cargo check --all` 0/0 + `cargo build --tests --all` 통과(lesson #21/#27 — check 는 lib만, 통합 테스트 빌드 별도 의무) + Tauri app `cargo check` 통과 + `cargo nextest run --all` (bench 제외) **489/489**. 
- **사이드 발견 — Tauri icon.png 누락(무관 확정)**: app `generate_context!`(main.rs:218) 가 `icons/icon.png` 요구하나 로컬·원격 모두 부재(tauri.conf.json 은 icon.ico 참조) = **코드/config 불일치 기존 환경 이슈**. 더미 png 생성 후 app check Finished 0 → **OutboundManifest 폐기 회귀 0 확정**. lesson #14 R1 = 자기 의심(이 에러가 내 작업 탓인가)도 실측으로 해소.
- **사이드 — bench_scale_5000 nextest timeout(무관)**: 120s 초과 TIMEOUT = 5000문서 성능 벤치(원격 Linux 느림), 폐기와 무관. bench 제외 필터로 489/489 PASS = 실 회귀 0 명확화. (feedback_bench_3runs + lesson 79 step-m5 flaky 정형 정합).
- **lesson 77 1-cycle 폐기 정형**: 2026-06-16 도입(outbound-umbrella-1 OutboundManifest)→2026-06-18 폐기 = **1 cycle 수명**. 추상화 진영의 정점(lesson 77)이 본질 재정의 3차(raw I/O)로 즉시 폐기 = "추상화는 본질 재정의의 가속 비용" 실증. 메타 룰 22(사용자 정책 경계) 후속 = 도입/폐기 모두 사용자 발화 단일 트리거.
- **TC.p7-no-outbound-manifest** = `grep -rc "OutboundManifest\|OutboundCategory\|ports::outbound" crates/ modals/` 의 실 코드(doc 주석 제외) = 0.
- **TC.p7-outbound-dir-gone** = `! test -d crates/core/src/ports/outbound && grep -c "pub mod outbound" crates/core/src/ports/mod.rs` = 0.

## step-p8 spec 정합 (cycle 6, 2026-06-18, 직접 편집 — 단일 진실원 위임) — plugin-sdk-1 완결

- **단일 진실원 위임 (메타 룰 19 정합)**: OutboundManifest 폐기의 단일 진실원 = `spec/deprecated.md` §삭제됨(엔트리 신설 — 53 impl + 디렉토리 + 검증 수치 + 재도입 트리거). 나머지 문서는 폐기 반영 + deprecated.md 링크.
- **정정 4 문서**: (1) deprecated.md 엔트리 + 헤더 updated 날짜 (2) domain-map.md 공통 우산 trait L200 + RemoteStoragePort 표 L344 = 폐기 반영 (3) architecture.md outbound 우산 § = 역사 기록 보존 + 머리에 ⚠️ 폐기 안내(2026-06-18 step-p7) (4) plugin-architecture Phase 208 = OutboundManifest::config_keys/modes 활용 전제 → plugin manifest(`fp-plugin.toml`) 기반 재설계.
- **history 보존 vs current 정합 정형 (step-m5 계승)**: architecture.md outbound-umbrella-1 § = 2026-06-16 도입 시점 작업 기록 = **삭제 아니라 머리에 폐기 안내 추가**(히스토리 보존). "Phase N 에 이렇게 했다"=보존 / "지금 시스템 이렇다"=정정 기준 적용. plugin-architecture Phase 208 = **미래 계획 = 정정**(폐기된 API 전제 금지).
- **plugin-sdk-1 plan 완결**(p1 baseline + p5 telegram plugin + p6 단순화 + p7 폐기 + p8 spec). p2/p3/p4 skipped(fp-plugin crate 기존재). 본질 재정의 3차 raw I/O 영역 = OutboundManifest 폐기로 핵심 종결(도메인 로직 plugin 본문 이관은 SDK handle_request 완성 후 후속).
- **코드 변경 0(문서만) = 빌드 검증 불필요**(step-m4 정형 정합).
