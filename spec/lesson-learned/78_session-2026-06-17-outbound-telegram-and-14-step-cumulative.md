# Lesson 78 — 2026-06-17 outbound 우산 본 진입 (step-o2 partial 해소 + step-o3 telegram + step-o6 spec 갱신, 14 step 누적)

## 상황

본 세션 = lesson 77 (11 step) 후속 진입 + 3 step 추가 (step-o2 partial 해소 + step-o3 + step-o4 + step-o6) = **단일 cycle 14 step succeeded**. lesson 77 시점 = outbound 우산 본질 재정의 + 24 어댑터 manifest impl 박힘 (6 port super-trait 부재 + zstd 영역 host 결정 영역 누적). lesson 78 = host 결정 영역 (super-trait + zstd) 의 worker 자율 진입 + telegram outbound 본 진입 + spec 갱신 정합.

### 본 세션 누적 11 → 14 step

| step | 영역 |
|------|------|
| step-o2 partial 해소 | 6 port super-trait OutboundManifest 박힘 + 19 mock manifest impl (stub 2 + composite 4 + cached_llm 2 + service.rs 4 + local_embed 1 + integration test 9 파일 mock 16) + capabilities ambiguity 정정 2 점 + zstd 분류 spec 정정 |
| step-o3 telegram outbound | 5 파일 박힘 (settings_db.rs telegram_message_map + 4 CRUD / telegram_storage.rs ~310줄 신규 / storage/mod.rs / telegram_notify.rs mode_options / adapters/Cargo.toml rusqlite + reqwest multipart) |
| step-o4 디렉토리 정합 | notification → notify / reranking → rerank / verification → verify 3건 mv + 8 호출처 sed 일괄 치환 |
| step-o6 spec 갱신 | architecture.md outbound 표 갱신 + plugin-architecture L226 zstd 정정 + lesson 78 (본 entry) 신규 박힘 |

## 문제

### sub-pattern 1: super-trait 강제 의무 영역 광역화 (lesson 14 R1 family)

step-o1 시점 = `OutboundManifest` trait 정의 박힘 + 24 어댑터 manifest impl 박힘 (super-trait 부재) → step-o2 명세 정합 부재.
step-o2 partial 해소 시 = 6 port super-trait `OutboundManifest + Send + Sync` 박힘 → **30+ impl 점 강제 의무 발견** = stub.rs (2) + composite.rs (4) + cached_llm.rs (2) + service.rs (4) + local_embed.rs SensitivityAware (1, **명세 부재 누락 발견**) + integration test 9 파일 mock 16건.

cargo check error 반복 사이클 = **5회**:
1. service.rs 4 mock + composite.rs 2 mock + cached_llm.rs 1 mock + capabilities ambiguity 2 점 (use_cases/process_file.rs + modals/app/commands.rs)
2. local_embed.rs SensitivityAwareEmbeddingAdapter (명세 누락)
3. integration test mock 10건 (BenchLlm/FailingLlm/HashEmbedder/RealClaudeLlm/RealCorpusLlm/SlowLlm/SmartTestLlm/StubLlm/TestLlm/TwoPassLlm)
4. integration test mock 3건 추가 (FastLlm/ScenarioLlm/HashEmbedder 다른 module)
5. integration test mock 2건 잔여 (HashEmbedder bench_real_corpus + search_accuracy)

= **lesson #14 R1 family 사이클 5회 반복 정형 발견** (host 예상 5회 vs 실측 5회 정합).

### sub-pattern 2: workspace 의존 cycle 함정 (lesson 14 R1 family)

step-o3 시 = telegram_storage.rs 안 `file_pipeline_shared::settings_db::SettingsDb::open` 호출 박힘 → **adapters → shared 의존 추가 시 workspace cycle** (shared → adapters 박힘 영역 + 본 추가 = 순환).
정정 = `rusqlite::Connection::open` 직접 호출 + adapters 자체 `TELEGRAM_MAP_SCHEMA` 박힘 (settings_db.rs schema 와 정합).

= **이전 lesson 77 sub-pattern (step-s3 발견) 와 동일 영역**. settings_db 가 shared 잔류 + adapters → shared 추가 시 cycle 발생 = **본 lesson 78 시점 정형 확립**.

### sub-pattern 3: workspace dep feature flag 누락 (lesson 14 R1 family)

step-o3 시 = telegram_storage.rs 의 `reqwest::multipart::Form` 호출 박힘 → workspace `reqwest` dep = `features = ["json"]` 만 박힘 → cargo check error `no method named multipart`.
정정 = adapters/Cargo.toml `reqwest = { workspace = true, features = ["json", "multipart"] }` 박힘 (workspace dep 영역 침범 부재 + crate-level features 박힘).

= host 예상 5회 사이클 vs 실측 2회 = sub-pattern 3 도 lesson #14 R1 family 정합.

### sub-pattern 4: ssh 원격 timeout 발생 (lesson 14 R1 family 부재 영역)

step-o3 scp 시점 = ssh 원격 timeout 발생 (2회 연속). 호스트 측 서버 일시 부재. 본 worker = bb partial coded 박힘 + idle 진입 default + 사용자 명시 후 재시도 정합.
재시도 = ssh 복구 후 scp + cargo check + nextest 정합. **본 sub-pattern = lesson 78 후보 (외부 인프라 의존 영역 임시 부재 처리 패턴)**.

## 원인

### 직접 원인 — sub-pattern 1 (super-trait 강제 광역화)

- step-o1 명세 부재 = super-trait 박힘 시 impl 강제 점 측정 의무 (manifest impl 후보 사니프)
- mock + stub + composite + cached + integration test 박힘 점 광역 발견 = manifest impl 광역 박힘 의무
- worker 측 default = manifest impl 일괄 박힘 + cargo check error 반복 사이클 5회

### 직접 원인 — sub-pattern 2 (workspace cycle)

- adapters → shared 의존 부재 = lesson 77 시점 step-s3 발견 영역
- 본 sub-pattern 정형 부재 → step-o3 시점 재발 + rusqlite 직접 호출 정정
- 정형 확립 정합 = adapters 자체 sqlite schema 박힘 (shared 의존 부재) + race condition = SQLite file lock 자체 보장

### 직접 원인 — sub-pattern 3 (feature flag 누락)

- workspace `reqwest` features = `["json"]` 만 박힘 (notion_storage 정합)
- telegram_storage = multipart 필요 → crate-level features 박힘 의무 발견
- 정정 = adapters/Cargo.toml 의 `features = ["json", "multipart"]` 박힘 (workspace 영역 침범 부재)

### 구조적 원인 — sub-pattern 4 (외부 인프라 의존)

- ssh 원격 서버 = host 측 외부 인프라 = worker 측 통제 부재
- timeout 발생 시 = worker idle + 사용자 명시 대기 default
- 본 default = lesson #30 family 정합 (외부 인프라 임시 부재 = host 결정 영역)

## 개선

### immediate (본 lesson 박힘)

- **super-trait 박힘 사니프 정형**: trait 정의에 super-trait 박힘 시 `grep -rln "impl <Trait>"` 으로 영향 점 사니프 + 모든 점 manifest impl 박힘 의무 (stub + composite + mock 포함)
- **workspace cycle 사니프 정형**: adapters → shared 의존 추가 시 = cycle 의무 → adapters 자체 영역 박힘 default (rusqlite 직접 호출 + 자체 schema 박힘)
- **workspace dep feature 사니프 정형**: 신규 어댑터 작성 시 = 사용 feature 영역 측정 + crate-level features 박힘 (workspace 영역 침범 부재)

### structural (lesson 78 정형 확립)

- **lesson #14 R1 family 사이클 N회 정형**: super-trait 강제 박힘 시 = mock + stub + integration test mock 측정 사이클 의무 (5회 정합 ~ host 예상 영역)
- **lesson #14 R1 sub-pattern 정형 박힘**: (a) super-trait 광역 사니프 + (b) workspace cycle 회피 + (c) feature flag 박힘 = 3 sub-pattern 정합
- **외부 인프라 임시 부재 처리 패턴**: ssh timeout 시 = bb partial coded 박힘 + idle 진입 + 사용자 명시 후 재시도 (lesson #30 family 정합)
- **cycle 3 prep-3 = 메타 룰 18 + #14 R1 6번째 사이클 (baseline 경로/규모 2축 동시 빗나감)**: baseline 핸드오프가 prep-3 을 "`adapters/driven/settings/sqlite.rs` 116 메서드 impl" 로 추정. 실측 2축 빗나감:
  - **(축1) 경로 cycle 위반**: `shared → adapters` dep 이 Cargo.toml 에 이미 박힘 → wrapper 가 `adapters → shared` 역참조 불가 = cycle. baseline 경로 불가 → `settings_db.rs` 내부 6 sub-trait + SettingsRepoPort 직접 impl 정정 (SettingsDb 자체가 trait impl, wrapper struct 부재). **사전 grep (`grep file-pipeline-adapters crates/shared/Cargo.toml`) 1회로 차단** = lesson #14 R1 정합.
  - **(축2) 규모 빗나감**: 86 pub 메서드 (baseline "116") 중 config-free 영역만 분리 가능 = 6 sub-trait 의 20 메서드. Config/Snapshot/Credential 60여 메서드는 `crate::config` 의존 → adapters 이전 시 cycle (= prep-3b 후속, config core 이전 선행 의무). **메서드 라인 범위 `grep crate::config` 0건 사전 검증으로 분리 안전 경계 확정**.
  - host 추정 빗나감 누적 = 5회 (lesson 78 본문) + prep-3 2축 = 사실상 **6/6 패턴 확정**. 메타 룰 18 (lesson 본문 추정 사항 재검증) 정식 승격 사후 첫 적용 사례.
- **cycle 3 prep-3b = 메타 룰 18 2번째 적용 + orphan rule 정형 (config core 이전)**: 사용자 명세 baseline "config 60여 메서드 영역" + "config 호출처 광역 변경 수십 곳" 2축 추정. 실측 빗나감:
  - **(축1) 규모**: config.rs 1621줄 중 순수 타입 = **28 struct** (PipelineConfig + 25 하위 + LlmCredential + ResolvedPaths + FieldMeta). "60여 메서드" 아님 = 타입 중심. 사전 `grep ^pub struct config.rs` 1회로 실측.
  - **(축2) 호출처**: `grep -rln PipelineConfig` = **14 파일** (shared 10 + app 3 + cli 1), "수십 곳" 아님. 게다가 **re-export(`pub use`)로 타입 사용 호출처 0건 변경** — use import만 10파일 (extension trait import). 사전 호출처 grep 1회로 변경 규모 정확 산정.
  - **orphan rule 정형 (재사용 패턴)**: shared 타입을 core 로 옮기면 shared 의 기존 inherent impl(메서드)이 **orphan rule 위반** (타입이 다른 crate). 회피 = **extension trait** (`PipelineConfigExt` / `ResolvedPathsExt`) 를 shared 에 정의 + 호출처 `use ...Ext;` 추가. 호출 형태(`PipelineConfig::load(p)` / `cfg.resolve_paths()`) 불변. 순수 메서드(인프라 비의존)는 core inherent impl 가능 (default_config/validate/needs_restart). **향후 shared→core 타입 이전 시 동일 패턴 재사용 의무.**
  - cycle 0 보존 핵심 = **core Cargo.toml 에 toml/dirs 미추가** (인프라 의존 코드 = shared 잔류, 타입만 core). 사전 `grep core Cargo.toml` 로 core 의존 제약 확인 후 분리 경계 확정.
- **cycle 3 hex-arch-d s5 = 메타 룰 18 3번째 적용 + `SettingsDb` 잔류 광역 차단 발견**: 사용자 명세 s2/s3/s5 = "shared 도메인/어댑터 파일 → core/adapters 이전 (단순)". 사전 grep 으로 **3 step 모두 동일 cycle 장벽** 실측:
  - **근본 원인**: prep-3 에서 `SettingsDb` impl 을 shared in-place 잔류시킨 결과, 이를 의존하는 모든 파일이 core/adapters 로 이전 불가 (core→shared 금지 / adapters→shared cycle). setup_review/config_snapshot(s2) + cached_llm/settings_audit_adapter(s3) + cli.rs(s5) = 전부 `crate::settings_db::SettingsDb` 의존 → **prep-unlock(SettingsDb 자체 adapters 이전) 선행 의존**.
  - **깨끗한 부분 진입만 실행**: `tray.rs`(s5, shared/core 의존 0 = std + tray_icon only) → `adapters/driving/tray.rs` 이전 완료. tray-icon dep = shared→adapters `[target.'cfg(windows)'.dependencies]` 이동. cli/main.rs import 1줄 갱신. nextest 520/520 + Tauri PASS. **나머지 5파일 = SettingsDb 잔류 차단으로 보류** (lesson #25 점진 정합 — 일괄 회피).
  - **교훈**: in-place impl(prep-3 같은 cycle 회피책)은 **하위 호환은 0이나 후속 이전을 차단**한다. SettingsDb 를 adapters 로 완전 이전(prep-unlock)하기 전엔 의존 파일들의 hex 정상화 불가. 의존 사슬 사전 grep 없이 "단순 이전" 추정 시 cycle 벽 다중 충돌. 메타 룰 18 = baseline "단순" 류 추정 = 사전 grep 의무 재확인 (3/3 자기적용 빗나감).
- **cycle 3 prep-unlock(ConfigSnapshot 선행) = 메타 룰 18 4번째 적용 + 다단계 cycle 점진 해소**: baseline prep-unlock 명세 "cycle 0 예상 (config core 이전 완료)". 사전 grep 실측 = **2 장벽** (config 단독 아님):
  - **(장벽1) `crate::config_snapshot::ConfigSnapshot`** (settings_db save/list/get_snapshot 5회) — config_snapshot.rs 는 shared 잔류(setup_review 사슬). SettingsDb adapters 이전 시 adapters→shared cycle.
  - **(장벽2) `crate::config::PipelineConfigExt`** (to_pipeline_config/migrate 의 load/default_config/load_from_str) — ext trait shared 잔류.
  - **이번 해소 = 장벽1만 (점진, 컨텍스트 23% 압박)**: ConfigSnapshot struct = `profile_json: Option<String>` (SetupProfile 타입 미보유) → prep-3b 정형(순수 struct core 이전)으로 cycle-free 이전 가능. ConfigSnapshot/SnapshotMetrics/RollbackThresholds/RollbackEvaluation/evaluate_rollback → core/domain/config_models.rs. create_snapshot/rollback_snapshot(SetupProfile+ext 의존) = shared 잔류. config_snapshot.rs 273→123줄. **호출처 0건 변경(re-export)**. **orphan rule 미발생** (ConfigSnapshot inherent impl=순수 serde, 타입과 함께 core행 → ext trait 불필요. prep-3b 와 차이: prep-3b 는 인프라 의존 메서드가 있어 ext trait 필요했으나 ConfigSnapshot 은 전부 순수).
  - **결과**: settings_db.rs cycle 장벽 2→1 감소. 남은 PipelineConfigExt 해소 + SettingsDb 본체 이전 = 다음 단계. 다단계 cycle = **장벽별 점진 해소** 정형 (lesson #25 + 메타 룰 18 정합). 4/4 자기적용 빗나감.
- **cycle 4 prep-unlock(SettingsDb 본체 이전) = 메타 룰 18 5번째 적용 + settings-db-split-1 완결**: prep-3 원래 baseline 목표(`adapters/driven/settings/sqlite.rs`) 가 cycle 3 에서 cycle 회피로 보류됐다가 cycle 4 에서 달성. baseline "PipelineConfigExt 마지막 단일 장벽" 추정 → 사전 grep 실측:
  - **실측 장벽 = ext + load_doc_type_registry 2건** (둘 다 `open_or_migrate` 내부. baseline "1건" 빗나감). **core 의 SettingsDb 참조 3파일 = 전부 주석** (실 타입 의존 0 = core→adapters cycle 위험 없음 — 사전 grep 으로 확인, 추정 위험 차단).
  - **해소 정형**: `open_or_migrate`(toml 마이그레이션 부팅 로직) = **shared 자유함수 추출** (PipelineConfigExt::load + load_doc_type_registry 의존 유지, shared 잔류). adapters 의 SqliteSettingsRepo(SettingsDb 이름 유지) = 순수 DB 메서드만. shared 자유함수가 adapters 타입 호출 = shared→adapters 정방향 (cycle 0). **associated fn → free fn 전환** = `SettingsDb::open_or_migrate` → `file_pipeline_shared::settings_db::open_or_migrate` (호출처 3파일 수정).
  - **결과**: SettingsDb 본체(86 메서드) adapters 이전. shared/settings_db.rs 2230→138줄(re-export + open_or_migrate). cycle 0 + rusqlite 시그니처 누출 0. **settings-db-split-1 plan 완결** (prep-1/2/3/3b/unlock). 5/5 자기적용 빗나감 패턴 확장.
  - **부분 unlock 정합**: SettingsDb 가 adapters 로 가면 → s3(cached_llm/settings_audit_adapter) + s5-cli = adapters 행이라 같은 crate 의존 = unlock. **단 s2(setup_review) 는 여전히 SettingsDb 의존 → core 불가** (core→adapters 금지). SettingsDb 의존 도메인은 core 진입 불가 = 헥사고날 한계 (adapter 의존은 core 금지).

### regression_tc

- TC.O2-super-trait-impl-coverage = `grep -rln "impl LLMPort\|impl RemoteStoragePort\|..."` 출력 vs `grep -rln "impl OutboundManifest"` 출력 = manifest impl 누락 0건 의무
- TC.O3-workspace-cycle-avoidance = adapters → shared dep 박힘 사니프 의무
- TC.O3-reqwest-multipart-feature = telegram_storage 박힘 사니프 의무
- TC.prep3-settingsrepo-impl-coverage = `grep -c "impl .*Repo for SettingsDb\|impl SettingsRepoPort for SettingsDb" crates/adapters/src/driven/settings/sqlite.rs` = 7 (6 sub-trait + super-port) 의무 (**cycle 4: settings_db.rs → sqlite.rs 경로 이전**)
- TC.prep3-no-config-in-subtrait = 6 sub-trait impl 블록 라인 범위 `grep crate::config` 0건 의무 (config-free 경계 보존, 현 sqlite.rs)
- TC.prep3b-core-no-infra-dep = `grep -E "^toml|^dirs" crates/core/Cargo.toml` = 0건 의무 (cycle 0 보존 — config 타입만 core, 인프라 의존 shared 잔류)
- TC.prep3b-config-reexport = `grep -c "pub use file_pipeline_core::domain::config_models" crates/shared/src/config.rs` ≥ 1 의무 (타입 호출처 0건 변경 보존)
- TC.prep3b-ext-trait = `grep -c "trait PipelineConfigExt\|trait ResolvedPathsExt" crates/shared/src/config.rs` = 2 의무 (orphan rule 회피 패턴 보존)
- TC.s5-tray-moved = `test -f crates/adapters/src/driving/tray.rs && ! test -f crates/shared/src/tray.rs` (tray 이전 완료 보존)
- TC.s5-no-shared-tray-dep = `grep -c "tray-icon" crates/shared/Cargo.toml` = 0 의무 (tray-icon dep adapters 이동 보존)
- TC.unlock-configsnapshot-in-core = `grep -c "pub struct ConfigSnapshot" crates/core/src/domain/config_models.rs` = 1 의무 (ConfigSnapshot core 이전 보존)
- TC.unlock-configsnapshot-reexport = `grep -c "pub use file_pipeline_core::domain::config_models" crates/shared/src/config_snapshot.rs` ≥ 1 의무 (호출처 0건 변경 보존)
- TC.unlock-core-no-setupprofile = config_models.rs 의 SetupProfile 출현 = 주석만 (실 타입 참조 0, cycle 보존)
- TC.unlock-settingsdb-in-adapters = `grep -c "pub struct SettingsDb" crates/adapters/src/driven/settings/sqlite.rs` = 1 (cycle 4: 본체 adapters 이전 보존)
- TC.unlock-settingsdb-reexport = `grep -c "pub use file_pipeline_adapters::driven::settings" crates/shared/src/settings_db.rs` ≥ 1 (호출처 호환 보존)
- TC.unlock-settingsdb-no-cycle = `grep -c "file-pipeline-shared" crates/adapters/Cargo.toml` = 0 (adapters→shared cycle 부재 보존)

## last_seen

2026-06-17 (cycle 4: prep-unlock(SettingsDb 본체) done — settings-db-split-1 plan 완결. SettingsDb 86 메서드 → adapters/driven/settings/sqlite.rs. open_or_migrate shared 자유함수 추출. shared/settings_db.rs 2230→138줄. cycle 0. nextest 520/520 + Tauri PASS. 누적 19 step succeeded. 메타 룰 18 5/5 빗나감).

## source_files

- src/crates/core/src/ports/output.rs (6 port super-trait 박힘)
- src/crates/adapters/src/stub.rs (2 mock manifest)
- src/crates/adapters/src/driven/notify/composite.rs (4 mock manifest)
- src/crates/shared/src/cached_llm.rs (2 mock manifest)
- src/crates/core/src/service.rs (4 mock manifest)
- src/crates/adapters/src/driven/embedding/local_embed.rs (SensitivityAware manifest)
- src/modals/cli/tests/*.rs (integration test mock 16건)
- src/crates/adapters/src/driven/storage/telegram_storage.rs (~310줄 신규)
- src/crates/shared/src/settings_db.rs (telegram_message_map + 4 CRUD)
- src/crates/adapters/src/driven/notify/telegram_notify.rs (mode_options 정정)
- src/crates/adapters/Cargo.toml (rusqlite + reqwest multipart)
- prd/research/plugin-architecture-2026-06-04.md (L226 zstd 정정)
- spec/architecture.md (본 §누적 변경 요약 갱신 + cycle 3 prep-3 행 갱신)
- spec/lesson-learned/78_*.md (본 lesson 신규 + cycle 3 prep-3 누적)
- src/crates/shared/src/settings_db.rs (cycle 3 prep-3: 6 sub-trait + SettingsRepoPort in-place impl, +~150줄)
- src/crates/core/src/ports/settings_repo.rs (prep-2 skeleton, prep-3 impl 대응)
- src/crates/core/src/domain/config_models.rs (cycle 3 prep-3b 신규, 1140줄, 28 순수 config 타입)
- src/crates/core/src/domain/mod.rs (prep-3b: pub mod config_models 등록)
- src/crates/shared/src/config.rs (prep-3b: 타입 제거 + re-export + PipelineConfigExt/ResolvedPathsExt 2종)
- prep-3b use import 추가 10파일 (shared: auto_suggester/cli/config_snapshot/mcp_server/setup_review/settings_db, cli/main, app: commands/service)
- src/crates/adapters/src/driving/tray.rs (cycle 3 s5 신규, 101줄, shared/tray.rs 이전)
- src/crates/adapters/src/driving/mod.rs (s5: pub mod tray 등록)
- src/crates/adapters/Cargo.toml (s5: [target.cfg(windows)] tray-icon 추가)
- src/crates/shared/Cargo.toml (s5: tray-icon 제거)
- src/crates/shared/src/lib.rs (s5: pub mod tray 제거)
- src/modals/cli/src/main.rs (s5: tray import 경로 shared→adapters)
- src/crates/core/src/domain/config_models.rs (cycle 3 prep-unlock: ConfigSnapshot 등 5 타입/함수 append)
- src/crates/shared/src/config_snapshot.rs (prep-unlock: 순수 타입 제거 + re-export, 273→123줄)
- src/crates/adapters/src/driven/settings/sqlite.rs (cycle 4 신규, SettingsDb 본체 86 메서드 + 6 sub-trait impl)
- src/crates/adapters/src/driven/settings/mod.rs (cycle 4 신규)
- src/crates/adapters/src/driven/mod.rs (cycle 4: pub mod settings 등록)
- src/crates/adapters/Cargo.toml (cycle 4: sha2/hex/toml workspace dep 추가)
- src/crates/shared/src/settings_db.rs (cycle 4: 2230→138줄 re-export + open_or_migrate 자유함수)
- src/modals/app/src/{main.rs,commands.rs} (cycle 4: open_or_migrate associated fn → free fn 호출처 22곳)

## 메타 룰 누적

- 메타 룰 22 (사용자 정책 경계 합의) = **20건** (+3 = lesson 77 (a)+(b) + 78 (a) telegram 본 진입)
- 메타 룰 25 (자기 적용 의무) = **11건** (+3 = lesson 76 + 77 + 78 자기 적용)
- 메타 룰 30 (spec 즉시 갱신) = **14건** (+2 = plugin-architecture §3-C 재정의 + 본 §누적 변경 요약 직후)
- **메타 룰 14 R1 family** = 본 세션 +5 사이클 (super-trait 광역 + workspace cycle + reqwest multipart + 2 외부 영역) + **cycle 3 prep-3 6번째 사이클** (baseline 경로 cycle 위반 + 규모 빗나감 2축) = lesson 78 sub-pattern 정형 확립
- **메타 룰 18 (lesson 본문 추정 사항 재검증)** = 정식 승격 (2026-06-17) 사후 **1~5번째 적용 사례 = prep-3 + prep-3b + s5 + prep-unlock(ConfigSnapshot) + prep-unlock(SettingsDb 본체)** (각 baseline 추정 → 사전 grep 재검증. prep-3=경로cycle+규모 / prep-3b=28struct+14파일0변경 / s5=SettingsDb 3step 광역차단 / unlock-cs=cycle0예상→2장벽 / unlock-body=ext1건예상→2건+core주석확인. **5/5 빗나감 패턴 확정** — baseline "단순/마지막/N건" 류 추정 = 사전 grep 절대 의무)
- **settings-db-split-1 plan 완결** (cycle 4): prep-1(도메인struct core)→prep-2(port skeleton)→prep-3(in-place impl, cycle 회피)→prep-3b(config core)→prep-unlock(ConfigSnapshot core + SettingsDb 본체 adapters). 5 step = SQLite 어댑터 헥사고날 정상화 완결. **다단계 cycle 점진 해소 정형의 완주 사례** (lesson #25 + 메타 룰 18).
- **orphan rule 회피 정형 (extension trait)** = cycle 3 prep-3b 신규 패턴. shared→core 타입 이전 시 기존 inherent impl = orphan rule 위반 → shared 에 `XxxExt` trait 정의 + 호출처 `use ...Ext;`. 향후 동일 이전 작업 재사용 의무 (메타 룰 14 R1 family sub-pattern 후보)
