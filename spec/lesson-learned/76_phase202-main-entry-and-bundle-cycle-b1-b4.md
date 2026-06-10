---
created: 2026-06-10
phase: Phase 202 본진입 (실 IPC) + Phase 203 fp-plugin-search placeholder + bundle-cycle B1~B4 단일 세션
prd_truth: prd/research/plugin-architecture-2026-06-04.md (§6 Phase 202 ✅ / 203 partial ✅)
related_lessons:
  - 14 (미연결 포트 — IpcNotYetImplemented variant 본진입 시 자연 삭제 의무)
  - 16 (module 워크스페이스 분리 — Phase 203 단계 0 placeholder 자기 적용)
  - 17 (의존 누수 점검 — core가 fp-plugin-sdk 의존 추가 시점 검증)
  - 18 (lesson 추정 빗나감 — 본 세션 +2건 누적 12번째)
  - 21/27 (구조체 필드 추가 통합 테스트 회피 — ServiceBuilder 디폴트 주입 3중 방어)
  - 29 (PIPELINE_BASE 패턴 — endpoint_path 분기 패턴 일관)
  - 71 (Linux Windows cross-build — 본 세션 Windows cfg 검증 후속 트리거)
  - 75 (Phase 200/201/202 placeholder + binary plugin 4축 — 본 lesson의 직접 선행)
meta_rules:
  - 메타 룰 5 강화 (구현 보류 마커는 본진입 시 자연 삭제 의무) — IpcNotYetImplemented 사례 첫 명시
  - 메타 룰 18 (lesson 추정 빗나감) — 본 세션 +2건 (thiserror `source` / Connection Debug)
  - 메타 룰 19 (단일 진실원) — _rust_module git 추적 부재 후속 트리거
  - 메타 룰 22 (사용자 정책 경계 합의) — 본 세션 +2건 (git 저장소 단독 / src/.git 완전 삭제) 누적 17건
  - 메타 룰 24 (stage 명명 규칙) — `plugin.{id}.{method}` 영역 추가 + 변수 기반 stage 검사 자동화
  - 메타 룰 25 (자기 적용 의무) — bundle-cycle 사이클 종결 직후 즉시 현행화 진입 = 9건째
  - 메타 룰 30 (spec 본문 phase별 즉시 갱신) — architecture + domain-map + deprecated + roadmap + plugin-architecture 동시 갱신 = 12건째
external_source: "bundle-cycle 스킬 (사용자 직접 작성, scripts/cycle.sh 결정론 드라이버)"
---

# Lesson 76 — Phase 202 본진입 (실 IPC) + bundle-cycle B1~B4 단일 세션

## 상황

lesson 75 종결 직후 다음 세션. 사용자 트리거 "spec 폴더 분석하고 다음 구현항목 진행해. 구현/빌드는 tasty 스킬로 원격 서버에서 수행해". 본 세션은 tasty 환경 외부 (`TASTY_SOCKET` 비어있음) → tasty 스킬 사용 불가 사실 보고 + 원격 빌드는 ssh ubuntu@172.16.13.45 직접 호출로 진행.

lesson 75의 후속 트리거 4건 — Task #22 원격 빌드 검증 / Phase 202 본진입 (실 IPC) / Phase 203 fp-plugin-search / 메타 룰 22 16건째 도달 시점 추적 — 모두 단일 세션 내 해소 시도.

권장 순서: B1 (검증) → B2 (실 IPC) → B3 (통합 테스트) → B4 (Phase 203 placeholder). bundle-cycle 스킬로 직렬 사이클 진입.

## 문제

### 이슈 1: bundle-cycle cycle.sh가 git 저장소를 가정 (file-pipeline 단독 git 미초기화)

본 세션 진입 시점 `C:\dev\claude_workspaces\file-pipeline`은 git 저장소 아님. `cycle.sh branch <bundle> main`이 즉시 실패. _rust_module도 동일.

사이드: src/ 내부에 별도 git 저장소(`master` 브랜치, 4/14 생성) 존재 — 무관한 잔존 .git. 첫 add 시 `mode 160000` (submodule) 처리.

### 이슈 2: plan 추정 빗나감 2건 (lesson 18 누적 12번째 + 13번째)

**1번째 빗나감**: `PluginError::NotRunning { plugin_id: String, source: String }`. 빌드 시 `thiserror`가 `source` 필드명을 자동으로 `#[source]` 매핑 → `String: StdError` 트레이트 미만족으로 E0599 컴파일 실패. 필드명 회피 (`source` → `cause`) 후 통과.

**2번째 빗나감**: `connection_pool.rs` 단위 테스트 `pool.get_or_connect(...).await.unwrap_err()`. `Result<Arc<Mutex<Connection>>, PluginError>`의 Ok variant T가 Debug 요구. `Connection`은 Debug 미구현 → E0277. `match` 분기로 변경.

### 이슈 3: 회귀 자동화 `audit_stage_check.sh` 자체 stale (변수 기반 stage 미검출)

기존 v1은 `audit.record(_, "literal", ...)` 직접 리터럴 + `audit.record(_, &format!("{영역}.{}", _), ...)` 동적 format! 두 패턴만 검사. Phase 202 본진입 코드의 `let stage = format!("plugin.{}.{}", ...)` + `audit.record(_, &stage, ...)` 변수 기반 stage 미검출.

검사 PASS 출력에 `plugin.*` 없음 → 회귀 자동화가 신규 영역 누락 silent. 자기 stale.

### 이슈 4: _rust_module 변경이 file-pipeline 단독 git 밖

본 사이클의 _rust_module 측 변경 4건 (fp-plugin-sdk/src/connection.rs 신규 / Cargo.toml 갱신 / fp-plugin-search 디렉토리 신규 / Cargo.toml workspace 멤버 추가)이 file-pipeline git 추적 밖. 단일 진실원(메타 룰 19) 잠재 위반.

## 원인

### 이슈 1 원인

bundle-cycle은 git workflow 가정 도구. file-pipeline은 본 세션까지 git 미초기화 상태로 작업되어왔음. lesson 75 시점에도 git 저장소 부재 — 별도 git push도 없었음. 본 세션이 bundle-cycle 첫 실 사용이라 미초기화 노출 첫 사례.

src/.git 잔존은 4/14 시점 별도 작업 흔적. 사용자 합의로 완전 삭제 결정 (이력 손실 < 통합 단순화 우선).

### 이슈 2 원인

**thiserror `source` 필드명**: thiserror의 [magic field name](https://docs.rs/thiserror/latest/thiserror/derive.Error.html#display) 규칙 — `source` / `backtrace` 필드명은 자동으로 `#[source]` / `#[backtrace]` 매핑 시도. 일반 String 필드는 StdError 미만족이라 컴파일 실패. plan 작성 시 본 규칙 미고려 (filename 자유 가정).

**Connection Debug 부재**: Connection은 BufStream을 wrap한 IPC handle. `tokio::io::BufStream`은 Debug 미구현. `unwrap_err()`는 Result<T, E>의 Ok variant T가 Debug 요구 — `match Err(e) => panic!("{:?}", e)`로 회피 가능. plan 작성 시 unwrap_err의 trait 요구 미고려.

### 이슈 3 원인

audit_stage_check는 lesson 54 Phase 95 시점에 작성. 당시 stage 패턴은 `audit.record(_, "literal", _)` + `audit.record(_, &format!("{}.{}", ...), _)` 2종. Phase 202 본진입의 변수 기반 stage는 (a) stage 길어서 (b) 시작 + 종료 2번 호출 + (c) 분기 마다 동일 stage 사용 등의 이유로 `let stage = format!(...)` 패턴 사용 — 자연스러운 코드 패턴이나 기존 스캐너가 잡지 못함.

### 이슈 4 원인

본 세션 git init 시점에 _rust_module을 file-pipeline 단독 저장소 안에 포함할지 사용자 의사 확인 — "file-pipeline만 단독 + http 원격" 결정. 형제 모듈 워크스페이스는 별도 관리 영역 (사용자 합의). 단, 본 세션 _rust_module 변경 4건 추적 부재는 별도 트리거 시점에 해소 필요.

## 개선

### 메타 룰 5 강화 — 구현 보류 marker variant는 본진입 시 자연 삭제 의무

**`IpcNotYetImplemented` variant 사례**:
- Phase 201 placeholder 진입 시 등재
- Phase 202 본진입 (`PluginRegistry::call` 실 IPC)에서 자연 삭제
- `spec/deprecated.md` 위임 갱신 동시

**규칙**: `#[error("Phase X에서 구현")]` 같이 phase 번호가 본문에 박힌 variant는 본진입 시:
1. variant 삭제
2. 대체 variant (NotRunning / IpcTransport / IpcProtocol 등) 추가
3. `spec/deprecated.md` 위임 갱신
4. 호출처 grep 검증 (테스트의 `assert!(matches!(err, IpcNotYetImplemented))` 패턴 삭제 의무)

### 메타 룰 18 누적 12/13번째 — plan 추정 빗나감 자동 차단 강화

**본 세션 누적 2건**:
- 12번째: thiserror `source` 필드명 자동 매핑 — plan에 thiserror magic field name 의식 누락
- 13번째: Connection Debug 부재 — `unwrap_err()` trait 요구 의식 누락

**자기 적용**: plan 작성 시 신규 enum variant 추가 + Result type 사용 시점에 다음 사전 grep:
```
grep "#[source]\|#\[from\]\|#\[backtrace\]" {새 enum 파일}
grep "impl.*Debug.*for {새 struct}" {새 struct 파일}
```

### bundle-cycle 첫 실 사용 메타 가치

**성공 패턴** (재사용 가능):
- evidence-mandatory: 모든 단계 결과를 `.bundle-cycle-runs/<bundle>/<phase>.{exit,log,sec}` 파일 인용. 환각 0건
- fail-stop: B2 build 첫 시도 2회 실패 (thiserror + Debug) → 즉시 중단 + 원인 명시 + 재시도. silent retry 0건
- 직렬 의존 명시: B1 통과 ↔ B2 진입 차단 / B2 통과 ↔ B3 진입 차단 — task 의존 그래프(`addBlockedBy`)와 정확히 일치

**자기 적용 학습**: cycle.sh가 git workflow 가정 — git 미초기화 환경에서 진입 전 사전 check 단계 필요 (bundle-cycle 스킬 본문에 추가 후보).

### 회귀 자동화 자체 stale 자기 검출 패턴 (lesson 74 M-1 동일 패턴 2건째)

**감지**: 신규 코드 영역(`plugin.*` stage) 추가 후 회귀 자동화 PASS 출력에 신규 영역 누락 → silent stale 의심.

**해소**: 자동화 스크립트에 신규 코드 패턴 분기 추가 (`audit_stage_check.sh` v2 — 변수 기반 stage 검사).

**메타 룰화 후보** (메타 룰 24 + 메타 룰 5 결합): "회귀 자동화는 본진입 시 출력 검증 의무" — 본진입 영역이 자동화 출력에 반드시 등장해야 함. 누락이면 자동화 자체 stale 진단.

### lesson 21/27 회피 3중 방어 검증 (Service.plugin_registry 필드 추가)

본 phase의 신규 필드 `plugin_registry: Arc<PluginRegistry>` 추가 시 다음 사전 grep 의무:
1. `grep -rln "FileProcessingService\s*{" src/crates/*/tests/ src/modals/*/tests/` — 직접 리터럴 검출
2. `grep -rln "McpState\s*{" src/crates/*/tests/ src/modals/*/tests/` — 동일
3. ServiceBuilder 사용 비율 확인 — 본 세션은 12 tests 모두 ServiceBuilder 사용 → 변경 0건 (lesson 21/27 회피 자연 충족)

**3중 방어**:
1. ServiceBuilder 디폴트 주입 (`build()`에서 `unwrap_or_else(|| Arc::new(PluginRegistry::new()))`)
2. 직접 리터럴 grep + 명시 갱신 (본 세션 service.rs 내장 테스트 + mcp_server.rs::make_mcp_state + cli.rs + main.rs 4건)
3. `cargo check --workspace --tests` (lesson 21/27)

### file-pipeline 단독 git 첫 설정 (메타 룰 22 누적 17건째)

**사용자 정책 경계 합의 2건 (본 세션 +2)**:
- 15건째 → 16건째: file-pipeline 단독 git + http://gitlab.bi.co.kr/reujea/file.git origin
- 16건째 → 17건째: src/.git 완전 삭제 (이력 손실 < 통합 단순화)

**push 정책**: `no-push-default` 적용 — 사용자 명시 push 지시 시까지 origin push 보류. 본 세션 4 commit이 로컬만 잔류.

## 측정

| 지표 | 변경 |
|------|------|
| 단위/통합 테스트 | 0 (lesson 75 시점, 본 git 미초기화) → **47 PASS** (B1 26 + B2 +12 + B3 +3 + B4 +4) |
| 신규 크레이트 | `fp-plugin-search` (Phase 203 placeholder) |
| 신규 모듈 | `connection_pool.rs` (core/plugin/) |
| 신규 파일 | `connection.rs` (fp-plugin-sdk/src/) + `plugin_e2e.rs` (modals/cli/tests/) |
| PluginError variants | 7 → **9** (IpcNotYetImplemented 삭제 + NotRunning/IpcTransport/IpcProtocol 추가) |
| audit stage 영역 | 6 → **7** (`plugin` 추가) |
| 회귀 자동화 | 9 → **10종** (audit_stage_check v2 + 변수 stage 분기) |
| spec 본문 갱신 | 4 (architecture + domain-map + deprecated + roadmap + plugin-architecture, 메타 룰 30 자기 적용 12건째) |
| 메타 룰 22 누적 | 15 → **17건** (본 세션 +2) |
| 메타 룰 18 누적 | 11 → **13건** (본 세션 +2: thiserror `source` / Connection Debug) |
| git commit | 0 → **4** (baseline + B2 + B3 + Merge B3) |
| lesson | 75 → **76** |

## 본 lesson의 메타 가치

1. **bundle-cycle 스킬 첫 실 사용 + 단일 세션 4묶음 직렬 완료** — Phase 202 본진입 같은 코드 변경 큰 작업도 단일 사이클로 완료 가능 입증
2. **lesson 75 후속 트리거 4건 단일 세션 모두 해소** — 메타 룰 25 자기 적용 9건째 + 단일 세션 4 트리거 동시 첫 사례
3. **메타 룰 5 강화 첫 명시 사례** — phase 번호 박힌 variant의 본진입 시 자연 삭제 의무
4. **회귀 자동화 자체 stale 검출 2건째** (lesson 74 M-1과 같은 패턴) — 본진입 시 출력 검증 의무 메타 룰화 후보
5. **plan 추정 빗나감 동일 세션 2건** — thiserror magic field name + Debug trait 요구 — plan 작성 단계 사전 grep 의무화 후보

## 후속 트리거

- **Windows cfg 분기 검증** — 본 세션은 Linux 원격 빌드만. `cargo-xwin` (lesson 71)으로 named_pipe cfg 분기 별도 검증
- **Phase 203 본진입** — fp-plugin-search placeholder의 LocalVectorStore + MMR + vec_io 본체 이관
- **_rust_module git 추적** — 본 사이클의 _rust_module 변경 4건이 file-pipeline git 밖. 단일 진실원 위반 가능성 (메타 룰 19 후보 누적)
- **git push 사용자 결정** — 4 commit 로컬만 잔류. origin push 사용자 명시 시점 대기 (`no-push-default`)
- **메타 룰 5 강화 정식 승격 도달 시점 추적** — phase 번호 박힌 variant 자연 삭제 패턴 누적 2~3건 시 정식 승격 검토
- **bundle-cycle git 미초기화 사전 check 추가** — 본 사이클 cycle.sh가 git 가정 노출. 스킬 본문에 진입 전 git 저장소 확인 단계 추가 후보
