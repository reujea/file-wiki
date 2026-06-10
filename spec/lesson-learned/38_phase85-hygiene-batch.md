# Lesson 38 — Phase 85 일괄: 파라미터 구조체 + dead 삭제 + 분기 통일 + 아카이빙

## 상황

Phase 84 후속으로 측정 무관 위생 작업 4건을 일괄 처리:
- B-1: clippy `too_many_arguments` 4건 → 파라미터 구조체 도입
- B-1+: workspace `--all --tests` clippy 잔존 4건 정리 (`field_reassign_with_default` 3 + `assertions_on_constants` 1)
- B-2: `auto_link` 함수 + `AutoLinkContext` 삭제 (호출처 0건, 7+ Phase 미사용)
- B-3: architecture.md 1876 → 1758줄 아카이빙 (Phase 64 이하 → `architecture-archive.md`)
- B-4: `find_data_dir` ↔ `resolve_paths` base 결정 통일 (사이드 발견 6 해소)

## 문제 / 발견

### 1. `too_many_arguments` 4건 — 빌더 vs 입력 구조체 선택 기준

lesson 36 "트리거 대기"였던 4건을 처리하며 함수별로 다른 패턴이 적합했다:

| 함수 | 인자 | 패턴 | 사유 |
|------|------|------|------|
| `update_cross_references` | 8개 (도메인 5 + 포트 3) | 입력 구조체 `CrossRefUpdateContext<'a>` | 단일 호출처, 빌더가 과도 |
| `auto_link` | 14개 (도메인 5 + 임계값 3 + cap 4 + 포트 1) | 입력 구조체 `AutoLinkContext<'a>` | (→ 추가 발견으로 삭제 결정) |
| `make_entry` (모듈 내부) | 8개 string slice | 입력 구조체 `DecisionDraft<'a>` | 4 호출처 모두 동일 모듈 |
| `add_todo` (public API) | 8개 (필수 2 + Optional 6) | 입력 구조체 `NewTodo<'a>` | Optional 6개 → 호출자가 일부만 채움. 빌더 검토했으나 호출 5건 모두 한 번에 채우는 패턴이라 구조체로 충분 |

**메타 룰**: 빌더 패턴은 "**선택적 필드가 많고 호출자가 점진적으로 구성**"하는 경우만. 한 번에 모든 필드를 채우는 호출이라면 입력 구조체로도 가독성·유지보수성을 충분히 얻는다. 빌더 의무화는 과적용 위험.

### 2. lesson 36 "잔존 8건"의 실제 상태 — 트리거 대기 표기 신뢰성

lesson 36 작성 시점(Phase 84 직전)에는 `too_many_arguments` 4건 + `very_complex_type` + `loop_var_index`가 "잔존 8건"으로 표기되어 있었다. 그러나 Phase 84에서 8→0으로 만들면서:
- `very_complex_type` → `ProgressCallback` type alias로 해소
- `loop_var_index` → `iter_mut().skip().take()` / `iter_mut().enumerate()` 로 해소
- `too_many_arguments` 4건 → `#[allow]` 회피로 잔존

본 phase에서 4건의 `#[allow]`를 모두 입력 구조체로 교체 → **lesson 36 잔존 항목 전부 0**.

**메타 룰**: "트리거 대기" 표기는 다음 phase에서 명시적으로 재확인하지 않으면 stale 가능. **lesson 작성 시점의 코드 상태**와 **현재 코드 상태**가 다를 수 있으므로, "잔존 X건" 같은 수치는 다음 phase 진입 시 grep으로 재검증해야 한다.

### 3. `auto_link` — 호출처 0건 7+ Phase 미사용 = lesson 14 패턴 재발

`auto_link`(14 인자 SQL 스타일 자동 교차참조)는 Phase 60 이전부터 존재했으나 service.rs/MCP/Tauri 어디에서도 호출되지 않음. lesson 14 "미연결 포트는 코드 부담만"의 변형 — 포트가 아닌 **inherent 메서드** 형태로 동일한 dead 자산.

처리 결정 (사용자 확인 후):
- **삭제** + **lesson 14 형태 보류 마커**(코드 위치 주석으로 시그니처·사유·재도입 트리거·git 복구 명령 명시)
- 함수 138줄 + struct 17줄 + docstring 8줄 + unused import 1줄 = 약 164줄 감소
- `update_cross_references`(LLM 기반)가 동일 영역을 더 정교하게 커버하므로 기능 손실 없음

**메타 룰**: dead 자산을 "보류 마커만" vs "삭제"로 결정할 때 — **사용자 결정 영역**. 자율 진행 보류. lesson 28의 "성능 게이트 완화 결정은 사용자 의도 필요"와 같은 범주.

### 4. architecture.md 아카이빙 분기점 결정

architecture.md가 1876줄 누적. 최초 제안은 "Phase 60 이하" 이었으나 실제 분기 후보는 3가지:
- Phase 65 이전 (트리거 #1/#11/#12 + release + Phase 61~64): 130줄 감소
- Phase 80 이전: 약 500줄 감소
- 전체 Phase 처리 이력 분리: 약 900줄 감소

사용자 선택은 **가장 보수적인 Phase 65 이전**. 영구 섹션(한 줄 요약·모달 매핑·도메인별 수치 등)과 최근 Phase(65~84)는 본문 유지.

**메타 룰**: 아카이빙은 매 세션 컨텍스트로 로드되는 핵심 문서이므로 분기점 결정은 **사용자 확인 필요**. 자율로 한 번에 큰 분리를 시도하면 "방금 한 작업의 맥락이 사라지는" 부작용 가능. 단계적·보수적 분리가 안전.

### 5. `find_data_dir` ↔ `resolve_paths` 분기 차이 — 사이드 발견 6 진단

architecture.md 사이드 발견 6: "auto_init이 PIPELINE_BASE와 exe_dir 양쪽에서 실행 — inbox/가 두 곳에 생성".

코드 분석 결과 **`auto_init`은 한 번만 호출**되며, 진짜 원인은 **base 결정 함수가 두 개로 분리**되어 다른 분기 트리를 따른 것:

| 함수 | 사용처 | 분기 트리 |
|------|--------|----------|
| `find_data_dir(None)` | Tauri auto_init / write_log | PIPELINE_BASE → cwd settings.db/toml → exe_dir(파일존재시) → APPDATA → exe_dir |
| `resolve_paths(cli_base)` | CLI build_service | cli_base → PIPELINE_BASE → config.paths.base → platform::default_base_dir |

같은 환경에서 CLI는 cwd 파일 검사를 안 하고, Tauri는 검사함 → 다른 base 반환 가능.

처리: `resolve_paths`가 base 결정을 `find_data_dir`에 위임. `config.paths.base`는 PIPELINE_BASE 미설정 + cli_base 미지정일 때만 적용.

**메타 룰**: 같은 의미("데이터 디렉토리 결정")의 함수를 두 개로 분리하면 lesson 1 "다중 위치 동기화 누락"의 신종 변형. **의미가 같으면 함수도 하나**, 차이가 있다면 차이를 명시적 인자로 표현.

## 개선 / 적용

### 신규 메타 룰

- **메타 룰 10 (후보)**: 함수 인자 7개 이상일 때 빌더 vs 입력 구조체 선택은 "호출자가 점진적으로 채우는가" 기준. 호출자가 한 번에 모든 필드를 채우면 입력 구조체가 더 가볍다.
- **메타 룰 1 보강**: "같은 의미의 함수 다중 정의"도 다중 위치 동기화 누락의 변형. base/경로/상수 결정 함수는 단일 함수로 통일.

### 코드 변경 요약

| 파일 | 변경 |
|------|------|
| `crates/core/src/domain/cross_reference.rs` | `CrossRefUpdateContext` 추가 / `AutoLinkContext` 추가→삭제 / `auto_link` 함수 삭제 + 보류 마커 / unused `DocDate` import 제거 |
| `crates/core/src/service.rs` | `update_cross_references` 호출처 갱신 |
| `crates/shared/src/auto_suggester.rs` | `DecisionDraft` 추가 / 4 호출처 갱신 |
| `crates/shared/src/settings_db.rs` | `NewTodo` 추가 / `add_todo` 시그니처 변경 / 3 내부 호출처 갱신 |
| `modals/app/src/commands.rs` + `modals/cli/src/main.rs` | `add_todo` 외부 호출처 갱신 |
| `modals/cli/tests/notification_integration.rs` + `tests/real_env_tests.rs` | clippy 테스트 위생 |
| `crates/adapters/src/driven/notification/format.rs` | clippy 테스트 위생 |
| `crates/shared/src/config.rs` | `resolve_paths` base 결정을 `find_data_dir`에 위임 |
| `spec/architecture.md` + 신규 `spec/architecture-archive.md` | Phase 64 이하 아카이빙 (1876→1758줄) |

### 회귀 기준선 (Phase 84 유지)

- workspace lib 테스트: **332/332** (96 core + 143 adapters + 93 shared)
- workspace clippy `--all --tests`: **경고 0건** (lib 0 + tests 0)
- workspace + Tauri `cargo check`: ✅
- Tauri commands 70 / MCP tools 32 변동 없음
- settings.db 테이블 변동 없음

### 후속 관찰

- lesson 36 "잔존 clippy" 항목 완료 → lesson 36 갱신 필요 (또는 본 lesson에서 종결 명시)
- architecture-archive.md는 첫 분리 사례. 향후 phase 누적 시 동일 패턴 적용 가능 (예: Phase 80 이전 추가 분리)
- 5K 합성 코퍼스 측정(트리거 #2/#4/A1-hit/A2-def/B1-def/C2-fp)은 본 phase 범위 외 — 다음 작업 후보
