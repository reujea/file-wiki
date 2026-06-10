# META — 메타 룰 인덱스

> 개별 lesson에서 반복 식별된 메타 패턴 모음. 신규 작업 진입 시 사전 체크리스트로 사용.

작성 트리거: lesson 28에서 "메타 룰화 시점 도달 (8건 누적)" 표시 (2026-05-14).

## 메타 룰 1: 같은 사실의 다중 위치 동기화 누락

**핵심**: 같은 정의·검증·자산이 여러 곳에 표현되면, 한 곳을 바꿀 때 다른 곳을 반드시 함께 갱신해야 한다. 컴파일러가 잡지 못하는 형태(설정/테스트/JS/SQL DDL 등)는 grep 기반 체크리스트로 보완. **spec 문서 자신도 위반자가 될 수 있다** (lesson 49).

### 누적 사례 (19건 → Phase 94 sub-rule 분리)

19건 도달로 단일 표 가독성 임계. 7 sub-rule로 카테고리화하되 본문 표는 시계열 보존.

#### Sub-rule 카테고리 (lesson 53 도입, Phase 96 상세화)

| Sub-rule | 영역 | 누적 사례 lesson | 해소 패턴 | 자동화 도구 |
|----------|------|----------------|---------|-----------|
| **1a UI 제거 패턴** | UI/HTML/CSS/JS 정리 시 다중 위치 동기화 | 13, 19, 19+, 47 | 10단계 체크리스트 + JS↔HTML id 매칭 grep | `dead_selector_scan.sh` (lesson 47) |
| **1b 구조체 필드 추가** | core/adapters 구조체 필드 → 테스트·생성처 일괄 갱신 | 21, 27, 35 | ServiceBuilder + Default impl | `cargo build --tests --workspace` (lib만으론 부족) |
| **1c DB 스키마** | DDL 추가/수정 시 open/open_in_memory + 인덱스 + 쿼리 | 10, 26 | SETTINGS_DB_SCHEMA 단일 상수 (Phase 82-prep) | `grep -rn "CREATE TABLE" crates/shared/src/` |
| **1d 미연결 포트/함수** | port trait·함수 정의만 있고 호출처 0건 | 14, 31, 35 | 명시적 보류 마커 `#[allow(dead_code)] // [Phase X] 연결 예정` | `grep -rn "impl .*Port for"` + 호출처 0건 검증 |
| **1e 직렬화 4계층** | 도메인 모델 → 어댑터 응답 → 영속 모델 → 파일 | 32, 42 | 도메인→어댑터→스토어→파일 일괄 추적 | (수동 grep, 자동화 후보) |
| **1f 함수/검사 분산** | 같은 의미의 함수·진입점이 여러 곳에 정의 | 29, 38, 50-A, 50-B, 51, 52 | 단일 진입점 + 도메인 타입 (`SensitivityDecision` / `Verifier` wrapper) | (메타 룰 19와 동일 패턴) |
| **1g spec 자기 위반** | spec 문서·메타 룰·인덱스의 다중 위치 (메타 룰 19로 분기) | 49, 28, 73 | 시간축(Why) vs 상태축(What) 역할 분리 + 단방향 링크 위임 | `single_source_check.sh` (2026-06-05 신규, 점검 분류 — 메타 룰 27) |

**신규 작업 진입 시 sub-rule별 분기**:

| 작업 유형 | 매칭 sub-rule | 적용 체크리스트 |
|----------|--------------|-----------|
| UI HTML/CSS/JS 변경 | 1a | "UI/기능 제거 시" 10단계 |
| Rust 구조체 필드 변경 | 1b | "코드/구조체 변경 시" + `cargo build --tests` 필수 |
| DB 스키마 변경 | 1c | "DB/스키마 변경 시" |
| 신규 trait/함수 추가 | 1d | 호출처 0건 검증 → 명시적 보류 마커 |
| 도메인 모델 필드 추가 | 1e | 어댑터 응답 + 영속 모델 + 파일 직렬화 4계층 점검 |
| 같은 의미 함수 다중 정의 발견 | 1f | 단일 진입점 도메인 타입 추출 (메타 룰 19) |
| spec 문서 같은 사실 중복 | 1g | 진실원 선언 + 단방향 링크 위임 (메타 룰 19) |

#### 누적 사례 (시계열, 19건)

| lesson | 패턴 | 단일화 가능 여부 |
|--------|------|------------------|
| 10 | 컬럼 rename → 인덱스가 옛 컬럼 참조 | DDL 검색 자동화 |
| 13 | UI 기능 제거 → JS dead code 잔존 | UI 8단계 체크리스트 |
| 14 | 포트 추가 → 호출처 0건 (GraphDBPort) | 포트 추가 시 service 주입 검증 |
| 19 | UI 제거 → Tauri commands dead code (Feedback 탭) | UI 10단계 체크리스트 (lesson 13 확장) |
| 19+ | **새 MCP 도구 추가 → Tauri commands 누락 (Phase 80 6건)** | **신규 도구는 MCP + Tauri + invoke_handler + JS 4곳 동시 등록** (2026-05-14 해소) |
| 21 | 구조체 필드 추가 → 테스트 5곳 누락 (summary) | builder 패턴 도입 |
| 26 | DDL 추가 → open_in_memory() 누락 | SETTINGS_DB_SCHEMA 상수 단일화 (Phase 82-prep 해소) |
| 27 | 구조체 필드 추가 → 통합 테스트 13곳 누락 (metrics_recorder, lesson 21 재발) | ServiceBuilder 또는 Default impl (Phase 27 해소) |
| 28 | 기능 제거 → 통합 테스트 단언 잔존 (stale, lesson 13/19 변형) | UI 10단계 체크리스트에 통합 테스트 grep 추가 |
| 29 | base 디렉토리 결정 함수 3건 누락 (lesson 26 변형) | `find_data_dir(None)`로 4곳 통합 |
| 31 | A1 캐시 wrapper — core→shared 의존 회피 | 신규 도메인 영역은 어댑터 단에서 검토 (헥사고날 유지) |
| 32 | API 정의만 노출 vs Dashboard 카드 통합 (Phase 80 카운터) | 신규 API 추가 시 frontend 호출처 grep 의무 |
| 35 | C2 scan 함수 미연결 (lesson 14 재발) + 신규 도메인 필드 4곳 동기화 (lesson 21/27 재확인) | 신규 함수는 호출처 grep, 신규 필드는 service+test_helpers+build+default 4곳 |
| **47** | **JS `getElementById('id')` ↔ HTML `id="..."` 불일치 — IA 전환 시 HTML만 정리되고 JS render 함수 잔존 (pb-subtabs 6 함수)** | **`grep -oE "getElementById\('[^']+'\)" ui/dashboard.js \| HTML id 매칭 검사 자동화** (lesson 47 §개선) |
| **49** | **spec 문서 자체 — architecture-archive.md ↔ deprecated.md 같은 삭제 사실 6건 중복 (feedback_* / credential_store_* / cli.rs / onnxruntime / Phase 64 dead 7건 / dead config 5건 단방향)** | **시간축(Why) vs 상태축(What) 역할 분리 + 단방향 링크 위임. archive 인벤토리 → deprecated.md 단일 진실원** (옵션 A 적용 2026-05-20) |
| **50-A** | **`service.rs` 3분기 (process_file_with_pipeline + process_file_legacy(dead) + simulate_pipeline) → 검사 분산 — `is_sensitive` + `scan_pii_in_text_with` 5+ 호출처 + 미연결 `is_sensitive_with_content`** | **classifier.rs `check_sensitive_and_pii` 단일 진입점 + `SensitivityDecision` 도메인 타입** (Phase 91 2026-05-21) |
| **50-B** | **검증 함수 3개 분산 — `verify_with_thresholds` + `detect_strong_claims` + `Linter::lint_strong_claims` (각 다른 모듈)** | **`reasoning/verifier.rs` Verifier wrapper 단일 진입점** (Phase 91 2026-05-21) |
| **51** | **MCP 도구 분류가 단일 차원(mutates_state)만 — `mcp_tool_catalog` 26 항목 단일 표 / 카테고리·비용 미노출** | **`McpToolMetadata` 다차원 + `mcp_tool_catalog_full` + 단일/다차원 카탈로그 일치성 테스트** (Phase 92 H3, Mirage Command 3차원 등록 패턴) |
| **52** | **백엔드 `mcp_tool_catalog_full` ↔ frontend `renderMcpCatalog` 일치 — 신규 MCP 도구 추가 시 양쪽 동기화 의무** | **단일 진입점 백엔드 + Tauri command + frontend API 단방향 흐름** (Phase 93 H3, lesson 32 패턴 적용) |

### 신규 작업 시 사전 체크리스트

#### 코드/구조체 변경 시
- [ ] 구조체 필드 추가/제거: `grep -rln "{StructName} {" --include="*.rs"` → 모든 초기화 파일 동시 갱신 (lesson 21/27)
- [ ] `cargo check --workspace`만으로 부족. `cargo build --tests --workspace` 통과 확인
- [ ] 포트 trait 추가: `grep -rn "impl .*Port for" crates/adapters/` + 호출처 0건 검증 (lesson 14)
- [ ] 호출처 0건이면 PR 보류 또는 명시적 보류 마커 (`#[allow(dead_code)] // [Phase X] 연결 예정`)

#### UI/기능 제거 시 (lesson 19 10단계 + lesson 28 + lesson 47 추가)
1. UI HTML/CSS/JS 제거
2. Tauri commands 함수 삭제
3. invoke_handler 등록 삭제
4. dependent struct/static 삭제
5. 보조 함수 삭제 (`cargo check` 경고로 식별)
6. core/adapters 호출처 grep 후 정리
7. settings.db / config 필드 정리
8. spec/architecture.md 수치·매핑 갱신
9. JS API.* 객체에서 호출 함수 삭제
10. JS render 함수 + DOM 셀렉터 정합성 검증
11. **통합 테스트 단언 grep**: `grep -rln "{기능명}_docs\|{함수명}\|{필드명}" modals/*/tests/` (lesson 28)
12. **JS `getElementById` ↔ HTML id 매칭 grep** (lesson 47): `grep -oE "getElementById\('[^']+'\)" ui/dashboard.js \| sort -u` → HTML id 존재 확인. **IA 재설계 phase 종결 시 의무**

#### DB/스키마 변경 시
- [ ] `open()` + `open_in_memory()` 양쪽 동시 반영 (lesson 26) — 또는 단일 상수 도입
- [ ] DDL 변경 시 인덱스 + 쿼리도 동시 grep (lesson 10)

#### Phase 종결 시 — GUI 회귀 자동 검증 (G-5, 2026-05-20 / Phase 97 +2 / 2026-06-05 +2 확장)

코드 변경 phase에 한정. `spec/benchmarks/scripts/` **9종 스크립트**:

```bash
# 정적 회귀 게이트 (필수)
bash spec/benchmarks/scripts/dead_selector_scan.sh        # exit 0 의무 (lesson 47)
bash spec/benchmarks/scripts/gui_http_smoke.sh             # 5/5 통과 의무
bash spec/benchmarks/scripts/audit_stage_check.sh          # exit 0 의무 (메타 룰 24, Phase 97 신규)

# 메타 룰 17 강화 자동화 (release 재빌드 + D:\file-test 배포)
bash spec/benchmarks/scripts/release_rebuild_required.sh   # exit 1 시 재빌드 의무 (메타 룰 17, Phase 97 신규)
bash spec/benchmarks/scripts/release_redeploy.sh           # 빌드 후 sha256 검증 (메타 룰 17 강화 정식, 2026-06-05 신규)

# 카탈로그 변동 (architecture.md 수치 동기화)
bash spec/benchmarks/scripts/action_catalog.sh --count     # baseline=68 (2026-05-20 G-6 후) → 72 (2026-06-05, Phase 107+A/B/E 누적)

# 후보 점검 (게이트 아님)
bash spec/benchmarks/scripts/empty_state_audit.sh
bash spec/benchmarks/scripts/single_source_check.sh        # spec 단일 진실원 위임 누락 (메타 룰 19/30/sub-rule 1g, 2026-06-05 신규)

# action 단위 흐름 추적 (디버깅 도구)
bash spec/benchmarks/scripts/data_flow_trace.sh <action>
```

자세한 사용법: `spec/benchmarks/scripts/README.md`

## 메타 룰 2: 검증 = 거부가 아니라 피드백

(lesson archive 핵심 교훈 1) 2-Pass 가공이 증명. 적용 영역: LLM 가공 / 사용자 입력 검증 / config 적용.

## 메타 룰 3: 외부 크레이트 소스를 코드 작성 전에 읽는다

(lesson archive 핵심 교훈 3) 외부 API와 자체 core trait 양쪽 적용. lesson 4/24/51/72에서 반복 확인.

## 메타 룰 4: 벤치마크는 3회 중앙값

(lesson 04) 단일 실행은 캐시 편향 위험. 본 프로젝트 회귀 게이트는 3회 중앙값 강제.

## 메타 룰 5: 트리거 대기 항목은 "코드 변경 없이 켤 수 있는 형태"

(lesson 14/15) 포트 trait + impl만 만들고 호출처를 비워두면 dead 자산. 트리거 도달 시 설정 토글로 활성화되는 구조로 완성해야 함.

## 메타 룰 6: 계측 → 실측 → 구현

(lesson 05) 가설은 실측으로 기각될 수 있다. 병목은 이동한다. 매회 재계산 → 배치 1회 패턴이 반복 효과.

## 메타 룰 7: 사용자에게 묻는 질문은 답할 수 있는 것만

(lesson 25) 도메인 자기 분류 같은 답할 수 없는 질문은 금지. 코퍼스 신호(stats/CRAG/lint) → 동작 모듈 추천 흐름.

## 메타 룰 9: 빌드 진단은 자원(메모리·디스크) 먼저 확인

(lesson 37) `error[E0786] invalid metadata files` / `E0463 can't find crate` / `E0460 newer version` 대량 발생 시 코드 문제로 단정하지 않는다. `mmap` 실패(os error 1455 = 페이징 파일 부족)가 동일하게 metadata 손상으로 보고된다.

체크 순서:
1. `head` 응답에 `os error 1455` / `failed to mmap` 포함 여부
2. `cargo build -j 2` (또는 더 작게)로 재시도
3. Cargo.lock 변경 직후라면 `cargo clean -p <crate>` 또는 `cargo clean` 전체
4. modals/app은 별도 `target/`을 가지므로 함께 clean

## 메타 룰 8: 신규 작업 항목은 기존 코드 grep 먼저

(lesson 34) "Ruflo B2 = 새로 구현해야 한다" 같은 항목명만 보고 신규 작업으로 분류하면 중복 구현 또는 무의미한 노력. 기능명·패턴명을 codebase에 grep해서 기존 구현 여부를 먼저 확인. 있으면 **가시화/노출 강화**로 작업 재정의, 없으면 신규 구현.

체크: `grep -rn "{기능명}\|{관련 패턴}" crates/ modals/` → 결과 0건일 때만 신규 작업으로 분류.

## 메타 룰 5 강화: 트리거 인프라 = 3요소 모두

(lesson 39 / 42) "트리거 대기 = 코드 변경 없이 켤 수 있는 형태"의 3요소:
1. **토글 가능 config 필드** (디폴트 비활성)
2. **호출처 분기 완성** (조건문이 코드에 존재)
3. **디폴트 동작 보장** (no-op 또는 안전한 fallback)

3요소 모두 충족해야 lesson 14(dead 자산) 회귀 방지. 또한 인프라 활성화는 **3단계**: 인프라 추가 → LLM/로직 활성화 → 실 코퍼스 측정. Phase 86 인프라 / Phase 87 lint 통합 / Phase 88 측정으로 첫 완성 사례 확보 (lesson 42).

## 메타 룰 1 추가 사례: 4계층 직렬화 동기화

(lesson 42) 도메인 모델 필드 추가 시 다음 4계층 모두 점검 필수:
1. `core/domain/models.rs::*` (도메인 모델)
2. `adapters/.../response.rs` (외부 응답 → 도메인 변환)
3. `adapters/.../{storage_model}` (영속 모델)
4. 영속 파일/DB 직렬화 결과

도메인 모델만 추가하면 영속 시점에 누락되어 측정 검증 실패. Phase 88에서 `Metadata.needs_verification` 추가 후 `StoredDoc` 미반영으로 .local-store.json에 미저장되는 회귀 발견·수정.

## 메타 룰 1 추가 사례: 같은 의미 함수 다중 정의

(lesson 38) `find_data_dir` ↔ `resolve_paths` ↔ `LocalVectorStore::new`의 base 결정이 3곳에 분산. 같은 의미("데이터 디렉토리 결정")의 함수를 여러 곳에 정의하면 한쪽 갱신 시 다른 쪽 누락. Phase 85 B-4(resolve_paths→find_data_dir 위임) + Phase 88(LocalVectorStore도 동일 분기 트리)로 통일.

## 메타 룰 29: 외부 문서 권고 도입 3단계 분리 (Phase 104 번호 충돌 해소 — 기존 "메타 룰 9" 중복 발견)

(lesson 40 / 41) 외부 best practice를 본 프로젝트에 도입할 때 다음 3단계로 분리:
1. **필드/구조 추가** — 위험 낮음, 호환성 우선
2. **로직 채우기** — LLM 프롬프트 또는 lint 룰이 필드 활용
3. **UI 노출** — 사용자 검토 가능

한 phase에 모두 진행하면 회귀 추적 어려움. 단계별 진행이 안전. 또한:
- 외부 출처를 **코드 주석**(예: `Phase 87 wikidocs 353407`)과 **lesson 양쪽**에 명시 — 권고 갱신 시 추적 가능
- 같은 외부 문서 분석 결과는 `prd/research/external-analysis-{date}.md` **단일 진실원**으로 정형화 → 다음 분석 시 본 문서 인용, 결정 반복 차단

## 메타 룰 10: 검사 함수 호출 비용으로 메서드 분리

(lesson 41) 검사 함수 추가 시 호출 비용 기준 분리:
- 가벼운 검사 (O(N) 메타데이터) → 단일 메서드 `Linter::lint`
- 무거운 검사 (O(N × content_size), 압축 해제·LLM 호출 등) → 별도 메서드 `lint_strong_claims` + 다층 주기(weekly/monthly)에서 호출

`max_per_doc` 같은 상한 파라미터로 긴 문서 폭발 방지.

## 메타 룰 11: fastembed 측정은 cold/warm 분리

(lesson 42) fastembed feature는 첫 모델 로드 ~80초. 단일 측정은 cold start로 시간 왜곡:
- v1 (비활성): 49.1s/doc
- v2 (활성 cold): 68.7s/doc — **첫 모델 로드 + 2-Pass 재가공**
- v3 (활성 warm): 44.9s/doc — 실측치

`lesson 04` "3회 중앙값"의 fastembed 변형. 2회 이상 실행 후 warm 값 채택.

## 메타 룰 12: lesson 본문 "잔존 N건" 표기는 다음 phase에서 종결 명시 의무

(lesson 39) lesson 본문의 "잔존 N건" / "트리거 대기" / "후속" 표기는 stale 위험. 다음 phase 종결 시 해당 lesson 본문에 **(✅ Phase X에서 종결)** 표시 의무. INDEX.md만 갱신하면 본문 stale 잔존. lesson 36 → Phase 85 종결 표시 누락이 Phase 86 A-3에서 보강 사례.

## 메타 룰 13: 인프라 활성화 4단계 (lesson 14 진화형)

(lesson 43) 메타 룰 5강화의 3단계(인프라 추가 / LLM·로직 활성화 / 실 코퍼스 측정)는 **사용자 가시화 4단계**로 확장. 4단계 모두 도달해야 dead 자산 회귀 0:

1. **인프라 추가** — config 필드 + 포트 trait + 디폴트 no-op (위험 0)
2. **로직 활성화** — LLM 프롬프트 / 어댑터 override / service 분기 연결
3. **실 코퍼스 측정** — fastembed cold/warm 분리 (메타 룰 11)
4. **UI 노출** — Tauri command / frontend 렌더 (사용자가 결과 확인 가능)

Phase 87 (1단계) → Phase 88 (2~3단계) → Phase 89 (4단계 + 측정 완성) 이 첫 사례. 4단계 누락 시 측정만 끝나고 사용자가 "동작 여부 모름" 상태.

체크: 각 단계에서 호출처 0건이면 다음 단계 진입 금지.

## 메타 룰 14: 다중 진입점 분기 트리 통일 (메타 룰 1 변형)

(lesson 38 + Phase 89 C-1) 같은 의미의 함수가 여러 인스턴스 생성 경로로 호출되면, 한쪽만 갱신 시 다른 쪽 누락. 본 프로젝트에서 누적된 분기 트리 불일치 사례:

| 함수/메서드 | 진입점 다중성 | 통일 시점 |
|------------|-------------|----------|
| `find_data_dir` ↔ `resolve_paths` ↔ `LocalVectorStore::resolve_data_base` | 3곳 | Phase 85 B-4 / Phase 88 사이드 / Phase 89 C-1 |
| `CompositePreprocessor::new` ↔ `with_tools` | 2곳, preprocess_with_config가 매번 `::new` 호출 | Phase 89 C-2 (clone) |
| `doc_types`: file → settings.db | CLI는 toml 파일, build_service는 settings.db | Phase 89 C-3 (toml 미존재 시 settings.db 폴백) |

**체크리스트** (신규 함수 추가 시):
- [ ] grep `{함수명}\\(` — 호출처 모두 동일 인자/경로를 받는가?
- [ ] CLI / Tauri standalone / Tauri command / 테스트 4곳 모두 같은 분기 트리?
- [ ] CLI `--base` 옵션이 마지막까지 전파되는가? (Phase 89 C-1 재발 차단)

## 메타 룰 15: 측정 환경 격리 + 증분 상태 파일 일괄 삭제

(lesson 43 사이드 발견 / Phase 89 A1-hit 측정) 측정 시 SHA-256 + `.compile-state.json` + `.work-queue.json`이 LLM 호출 이전에 동일 파일을 스킵 → 캐시 hit 측정 도달 불가. 측정 위생 체크리스트:

- [ ] PIPELINE_BASE 격리 디렉토리 사용 (cwd 오염 방지)
- [ ] 동일 파일 재가공 측정 시 `.local-store.json` + `.compile-state.json` + `.work-queue.json` 일괄 삭제 (settings.db는 유지 — LLM 캐시 영속)
- [ ] cold/warm 분리 (메타 룰 11) + 3회 중앙값 (메타 룰 4)
- [ ] 격리는 LLM 호출 이전 단계라 격리율 측정에는 짧은 시간 가공 표본도 충분 (Phase 89 C2-fp 30 docs로 결정)

## 메타 룰 16: 작업·외부 솔루션 사전 분류 라벨 (🟢/🟡/🔴)

(lesson 44 + lesson 45 누적 / Phase 89~90) 신규 phase 진입 전 또는 신규 외부 솔루션 통합 전 **사전 분류 의무**. 라벨 누락 시 phase 중간에 "자동 측정 불가" 또는 "추상화 불일치"를 발견하여 회귀.

### 차원 A: 자동 측정 가능성 (lesson 44)

phase의 모든 트리거 대기 항목에 라벨 부착:

- 🟢 **자동 측정 가능**: 코퍼스 + 벤치 함수만 있으면 즉시 진행
  - 예: A1-hit / C2-fp / B-1 #2/#4 5변형 / N-3 / N-4
- 🟡 **사용자 코퍼스 의존**: 특정 도메인 코퍼스가 있어야 신호 발생
  - 예: 다른 도메인 PII / #4 doc_type 다양성 / #8 표 비중 / C-d1~3
- 🔴 **사용자 만족도 의존**: 자동 측정 불가, 실 사용자 피드백 필수
  - 예: #6 HyDE / A2-def / B1-def / #7 MRR 회귀 체감

🔴 라벨은 phase 시작 전 "외부 신호 대기" 단계로 분리. phase 본 작업 진입 금지.

### 차원 B: 외부 솔루션 통합 추상화 매칭 (lesson 45)

신규 외부 솔루션(저장소·알림·LLM·임베딩) 통합 전 라벨 부착:

- 🟢 **추상화 매칭 + module 위임 가능**: form-agnostic 모듈에 raw 어댑터 추가 후 thin wrap
  - 예: S3 / WebDAV / Network 저장소 — `module-storage::S3RemoteStorage`
  - 예: Telegram / Slack 알림 — `module-notify::TelegramNotify`
- 🟡 **추상화 부분 매칭 + 직접 구현**: 기존 포트 trait에 mode 분기로 도메인 특수성 흡수
  - 예: Notion 원격 저장소 — `mode=page` 의미 있음, `mode=attach` 명시적 미지원 (bail!)
  - 예: Confluence / Slack 메시지 (예측) — mode 분기 가능
- 🔴 **추상화 불일치 → 다른 포트로 매핑**: 기존 포트 강제 매핑 시 도메인 누수, 별도 포트 trait 또는 모듈 필요
  - 예: Discord webhook 알림 → `NotificationPort` 매핑 (저장소 아님)
  - 예: Notion database 쿼리를 검색 결과로 → `VectorDBPort.search_hybrid`와 불일치, 별도 `RemoteSearchPort` 검토

### 사전 분류 체크리스트 (메타 룰 14와 결합)

새 외부 솔루션 통합 / phase 진입 전:

- [ ] **차원 A 라벨**: 🟢/🟡/🔴 부착. 🔴은 phase 진입 자제
- [ ] **차원 B 라벨** (외부 솔루션만): 🟢/🟡/🔴 부착. 🟡은 mode 분기 설계, 🔴은 포트 분리 검토
- [ ] **공식 API 버전 + 분기 사전 조사**: 예 — Notion v1 (2022-06-28) vs file_upload v2024 (메타 룰 3 사례)
- [ ] **제약 조사**: rate limit / 크기 제한 / 인증 흐름 / hard delete vs archive
- [ ] **module 위임 가능 여부 결정**: form-agnostic 모듈에 도메인 누수 없는가? (메타 룰 14 결합)

### 사례 (Phase 89~90 누적)

| 항목 | 차원 A | 차원 B | 결과 |
|------|--------|--------|------|
| A1 hit | 🟢 | — | Phase 89 측정 (1.93x) |
| C2 fp | 🟢 | — | Phase 89 측정 (FP 0%) |
| #2 / #4 5변형 | 🟢 | — | Phase 89 재측정 (Phase 86 재현) |
| A2-def / B1-def | 🔴 | — | 보류 (검색 만족도 = 실 사용자) |
| #6 HyDE 디폴트 | 🔴 | — | 보류 (검색 안됨 피드백 대기) |
| #4 다른 도메인 | 🟡 | — | 보류 (다른 도메인 코퍼스 도달 시) |
| S3/WebDAV/Network | — | 🟢 | module-storage thin wrap |
| **Notion** | — | 🟡 | **Phase 90 직접 구현 + mode 분기** |
| Telegram/Slack 알림 | — | 🟢 | module-notify thin wrap |
| (예측) Discord webhook | — | 🔴 | 알림 포트로 매핑 |
| **JAMES v0.3.0 RBAC PolicyEngine** | — | 🔴 | **단일 사용자 도메인 불일치 — 보류** (lesson 50) |
| **JAMES Change Request 인간 게이트** | — | 🔴 | **proposer=approver=single 도메인 불일치 — 보류** (lesson 50) |
| **JAMES 3-stage 보안 파이프라인 (output 1단계)** | — | 🟢 | **mask_pii_in_text 흡수 — Phase 91 A2** (lesson 50) |
| **JAMES cognitive middleware verifier 함수 통합** | — | 🟢 | **reasoning/verifier.rs wrapper 흡수 — Phase 91 B1** (lesson 50) |
| **JAMES trace_id 단일 키 + audit 테이블** | — | 🟢 | **audit_trace + TraceId 흡수 — Phase 91 A3** (lesson 50) |

## 메타 룰 17: 코드 변경 phase의 release 빌드 + 배포 시점 의무화 (2026-06-05 강화 정식 승격)

(lesson 46 / G-3 사례 / Phase 90 Notion 빌드 시점 누락 → 정식 / **Phase 106 D:\file-test 재배포 누락 + Phase 107 release 재빌드 + lesson 71 Linux cross-build = 강화 누적 3건 → 2026-06-05 강화 정식 승격, 메타 룰 23 §승격 3요소 모두 충족**)

**코드 변경 phase 종결 시 release 재빌드 + D:\file-test 배포까지 의무**. 자원만 변경(UI/prompts.toml/config 디폴트)된 phase는 재빌드 예외. 빌드 완료 ≠ 배포 완료 — 사용자 시점 신규 기능 반영을 위해 D:\file-test 잔류 binary 감지 + 재배포까지 단일 의무 단위.

### 1단계: 변경 영역 분류 (재빌드 필요 여부)

phase 종결 시 변경 분류:

| 변경 유형 | 재빌드 필요 | 예 |
|----------|------------|-----|
| Rust 코드 (core/adapters/shared) | ✅ workspace | 도메인 모델 변경 / 포트 추가 |
| Tauri commands.rs / main.rs | ✅ workspace + Tauri | invoke_handler 등록 변경 |
| `ui/dashboard.js` / `ui/dashboard.css` / `ui/index.html` | ✅ Tauri (정적 자원 임베드) | dead-code 정리 / 새 핸들러 |
| `icon.png` (Tauri bundle 자산) | ✅ Tauri | RGBA 채널 보장 (lesson 71) |
| `pipeline.toml` / `prompts.toml` / `setup_rules.toml` | ❌ 재빌드 불필요 | 디폴트 값 / 프롬프트 수정 |
| `spec/**/*.md` | ❌ 재빌드 불필요 | 문서 갱신 |
| `spec/benchmarks/scripts/*.sh` | ❌ 재빌드 불필요 | 도구 스크립트 |

### 2단계: 재빌드 후 배포 확인 (강화 영역)

빌드 완료 ≠ 배포 완료. 다음 절차로 D:\file-test 잔류 binary 감지 + 재배포:

```bash
# (1) 실행 중 pipeline.exe 감지
tasklist | grep -i pipeline.exe

# (2) 감지 시 사용자 확인 → 종료 + 5초 대기
taskkill /F /IM pipeline.exe

# (3) cp + sha256 검증
cp target/release/pipeline.exe D:/file-test/
sha256sum target/release/pipeline.exe D:/file-test/pipeline.exe   # 일치 확인

# (4) Tauri binary 동일 절차
```

cross-build (Linux→Windows MSVC, lesson 71) 시 추가 사전 의무:
- `cargo install cargo-xwin` + LLVM 사전 설치 확인
- `icon.png` RGBA 채널 (RGB는 ico 변환 실패)
- 빌드 시간 비교 (cross 대 native — release 동등 시간 범위 확인)

### 누적 사례 (정식 3건 + 강화 3건 = 총 6건)

| 시점 | 영역 | 사례 |
|------|------|------|
| Phase 90 | release 재빌드 | Notion build_service 분기 후 Tauri 재빌드 누락 (17:29 → 18:35 pipeline.exe만) → G-3 해소 (2026-05-20 11:24 재빌드) |
| G-1 (Phase 90+) | release 재빌드 | claude_cli.rs F-1~F-5 코드 변경 후 즉시 재빌드 ✅ 의무 준수 (2026-05-20) |
| G-7 (Phase 90+) | release 재빌드 | commands.rs 함수 삭제 + main.rs invoke_handler 정리 후 재빌드 ✅ 의무 준수 (2026-05-20) |
| **Phase 106** | **D:\file-test 배포** | 1차 빌드(18:48) 후 D:\file-test 재배포 누락 → 사용자 신고 → 2차 종료+재배포 (19:29) — **강화 1건** |
| **Phase 107** | **release 재빌드 보류** | Rust + UI 변경 누적분 release 재빌드 의무, 다음 세션 보류 결정 (메타 룰 22 사용자 합의) — **강화 2건** |
| **lesson 71 (2026-06-04)** | **Linux cross-build + D:\file-test 배포** | cargo-xwin MSVC 빌드 + D:\file-test\pipeline.exe 19.03MB 배포 + sha256 일치 검증 ✅ — **강화 3건** |

### 정식 승격 (메타 룰 23 §승격 3요소 모두 충족)

- [x] 누적 ≥ 3건 — **강화 영역 3건 도달** ✅ (Phase 106 / Phase 107 / lesson 71)
- [x] 체크리스트 + 자동화 도구 — `spec/benchmarks/scripts/release_rebuild_required.sh` (Phase 97, 메타 룰 24와 결합) + cross-build 사전 확인 의무
- [x] META.md 본문 등재 ✅ (2026-06-05 강화 정식 승격으로 후보 2 섹션 단일 위임)

### 신규 작업 시 사전 체크리스트

- [ ] Phase 종결 시 `bash spec/benchmarks/scripts/release_rebuild_required.sh` 실행 — exit 0 시 재빌드 불필요, exit 1 시 재빌드 의무
- [ ] 재빌드 의무 발생 시: workspace cargo build --release + Tauri release 빌드 모두
- [ ] 재빌드 완료 시: D:\file-test 잔류 binary 감지 + 종료 + 재배포 + sha256 일치 검증
- [ ] cross-build 시: cargo-xwin + LLVM + icon.png RGBA 사전 확인 (lesson 71)
- [ ] 메타 룰 22 사용자 합의 결합: "다음 세션 보류" 결정 가능 (Phase 107 사례) — 단 lesson 본문에 명시 기록 + 다음 세션 진입 시 우선 처리 의무

### 자동화 도구

- `spec/benchmarks/scripts/release_rebuild_required.sh` — git diff / find -newer 기반 재빌드 필요 여부 자동 판정 (Phase 97 신규, 메타 룰 27 분류 = 게이트)
- `spec/benchmarks/scripts/release_redeploy.sh` — D:\file-test 잔류 binary 감지 + 종료(--apply) + cp + sha256 일치 검증 (**2026-06-05 신규**, 정식 승격 직후 자기 적용. 메타 룰 27 분류 = 게이트, sha256 결정적). lesson 65 + lesson 71 패턴 자동화

## 메타 룰 18: lesson 본문의 추정 사항은 다음 phase에서 재검증 의무

(lesson 46 G-1/G-4 추정 오류 / 메타 룰 12 확장) lesson 본문의 **"~로 추정", "~인 듯", "원인 불명" 표현**은 다음 phase 종결 시 재검증 의무. 단순히 lesson에 기록만 하면 stale 위험.

### 사례 (3회 누적)

| Lesson | 원래 추정 | 실제 재검증 결과 |
|--------|----------|----------------|
| lesson 46 G-1 | "동시 호출 / stdin 파이프 / 짧은 파일 추정" | **외부 일시 요인** (격리 재현 9/9 성공) |
| lesson 46 G-4 | "Settings/검색결과/모듈체크박스 invoke-no-fallback 5건" | **5건 모두 정상 작동 + 빠진 1건(Verification)이 진짜 문제** + dead-code 패턴 발견 |
| **lesson 50 Phase 91** | **"service.rs:235 ↔ service.rs:644 활성 중복 2곳"** | **service.rs:235 = process_file_legacy #[allow(dead_code)] deprecated (호출처 0건). 644만 활성** |

추정 빗나간 비율 = **3/3 = 100%**. 추정은 신뢰할 수 없다는 패턴 확정 강화.

### 재검증 체크리스트

- [ ] **lesson 본문에서 "추정" 키워드 grep**: `grep -nE '추정|것으로 보임|불명|likely|suspect' spec/lesson-learned/*.md`
- [ ] **다음 phase 종결 시 추정 사항 1건 이상 검증**: 재현 환경 분리, browser-automation 정밀 검증, grep 추적 등
- [ ] **검증 결과를 lesson 본문에 ✅ 또는 ❌로 명시 갱신** (메타 룰 12 "잔존 종결 의무" 변형)

**Phase 91 강화 (lesson 50)**:
- [ ] **"중복 분기"라 추정한 함수/호출처는 활성성 추가 grep 의무**: `grep -B5 "fn {함수명}" {파일}.rs | grep -E "allow\(dead_code\)|deprecated"` — `#[allow(dead_code)]` 또는 deprecated 마커 사전 확인
- [ ] **phase 시작 시 본인의 추정 1개 이상 격리 검증**: 본 phase 진입 직후가 검증의 최적 시점 — 코드 광범위 변경 전에 추정 정확도 확보

### 메타 룰 12와의 차이

| 측면 | 메타 룰 12 | 메타 룰 18 (본 룰) |
|------|----------|------------------|
| 대상 | "잔존 N건" 수치 stale | "~로 추정" 사실 stale |
| 처리 | 종결 표시 | 재검증 + 갱신 또는 ✅/❌ |
| 누적 | lesson 36→85 / 39→86 | 본 lesson 47 + 후속 |

## 메타 룰 20: 외부 프로젝트 패턴 흡수 시 도메인 가정 정렬

(lesson 50 Phase 91 JAMES + 2026-05-22 TFM + JAMES 재검증 + Mirage 누적 4건 → Phase 92 META 정식 승격)

외부 프로젝트의 좋은 패턴을 흡수할 때 다음 3축 분리 의무:

1. **패턴 추출** — "무엇이 좋은가" (디자인 패턴 자체)
2. **도메인 가정 검증** — "그 패턴이 전제하는 도메인이 우리 도메인과 일치하는가"
3. **부분 흡수 결정** — 일치 영역만 흡수, 불일치 영역 명시 보류

### 흡수 결정 라벨 (메타 룰 16 차원 B 결합)

- 🟢 **본질 도메인 일치** — 즉시 흡수 검토
- 🟡 **부분 일치 + mode 분기 가능** — 흡수 + mode 분기 표준화
- 🔴 **불일치** — 명시 보류 + META 누적 사례 등재 (재검토 트리거 명시)

### 누적 사례 (8건 도달, 2026-06-04 본 결정 +4건 추가)

| 프로젝트 | 본질 일치 흡수 (🟢) | 부수 일치 흡수 (🟡) | 불일치 보류 (🔴) |
|---------|---------|---------|---------|
| **JAMES v0.3.0** (Phase 91 lesson 50) | Verifier 통합 (B1) | audit_trace (A3) / MCP mutates (B2) / 출력 PII mask (A2) | RBAC / Change Request / 5 역할 / 메모리 3계층 |
| **TabPFN / TFM** (prd/research/tfm-tabpfn-analysis.md) | 없음 (본질 도메인 불일치) | 이상 탐지 (G1) / ETA 예측 (G2) | doc_type 분류 LLM 대체 / 검색 리랭킹 대체 / TabTune Python 통합 |
| **JAMES v0.3.0 재검증** (2026-05-22, 변동 없음) | (변동 없음) | 자동 롤백 트리거 (H1, Phase 92) | ChromaDB / Ollama / JWT 스택 (Rust 단일 바이너리 불일치) |
| **Mirage v0.0.1** (prd/research/external-analysis-2026-05-22.md) | 없음 (본질 도메인 불일치) | MCP 카탈로그 다차원 (H3, Phase 92) / Resource capabilities 표준화 (H5, Phase 92) | VFS / bash 인터페이스 / TypeScript-Python 스택 / 전체 원격 백엔드 통합 |
| **wikidocs 353407** (Phase 87/88, external-analysis-2026-05-15.md) | needs_verification + open_questions + lint 다층 주기 + detect_strong_claims | (해당 없음 — 본질 100% 일치) | (해당 없음) |
| **Adaptive Chunking (arxiv 2603.25333)** (Phase A/B 2026-06-04, external-analysis-2026-06-04-adaptive-chunking.md) | 4지표 측정 (SC/BI/ICC/DCC) + ChunkingStrategy enum | (Phase C Adaptive 본체 대기) | RC (영어 전용 Maverick coref) |
| **Grimoire** (Phase E1/E2/E3 2026-06-04, external-analysis-2026-06-04-grimoire.md) | (해당 없음 — 정반대 정책) | get_index / write_note / get_context 3 MCP 도구 | 임베딩 제거 / 마크다운만 / Go 바이너리 / Ollama frontmatter |
| **tasty v0.6.0** (본질 재정의 2차 2026-06-04, plugin-architecture-2026-06-04.md) | **workspace + 별도 프로세스 plugin + IPC + 매니페스트 + permission gate (전체 패턴 직접 흡수)** | (해당 없음 — 본질 100% 일치) | (해당 없음) |

### 사전 분류 체크리스트

외부 프로젝트 흡수 검토 시 다음 의무:

- [ ] **라벨 부착**: 패턴별 🟢/🟡/🔴 분류
- [ ] **본질 도메인 일치 영역만 즉시 흡수** — 부수 영역은 인프라 선구현 후 측정 도달 시 활성화 (lesson 30 패턴)
- [ ] **🔴 항목은 명시 보류 + 재검토 트리거 명시** (예: "JAMES Change Request는 v1.0 다중 사용자 도달 시")
- [ ] **메타 룰 16 차원 B와 결합** — 외부 솔루션 추상화 매칭 라벨도 함께 부착
- [ ] **lesson 본문 + META.md 양쪽에 누적 사례 등재** (메타 룰 1 자기 적용)

### 본 룰의 메타 가치

- 외부 좋은 패턴이라도 도메인 불일치 시 over-engineering — lesson 50 핵심 발견
- "흡수할 수 있는가"와 "흡수해야 하는가"가 다른 질문
- 사전 분류 체크 없이 진행하면 추정 빗나감(메타 룰 18) + 다중 위치 동기화 누락(메타 룰 1) 동시 위반 위험

## 메타 룰 21 후보 → 정식 승격됨 (Phase 103, 위 §메타 룰 21 정식 섹션 참조)

본 섹션은 **메타 룰 19 자기 적용** — Phase 103 정식 승격 후 후보 본문은 본 위임 표시로 대체. 단일 진실원은 위 §메타 룰 21 정식 섹션. Phase 104 메타 룰 25 자기 적용 회귀 차단 (정식 승격 직후 후보 섹션 제거 의무).

## 메타 룰 19: 단일 진실원 위임 패턴 (메타 룰 1 변형, Phase 94 META 정식 승격)

(lesson 49 + Phase 91 classifier/Verifier + Phase 92 MCP 카탈로그 일치성 + Phase 93 백엔드→frontend = 5건 누적 → Phase 94 정식 승격)

같은 사실이 둘 이상 문서/위치에 등장할 때, 한 곳을 **진실원(인벤토리/상태)** 으로 지정하고 나머지는 **링크 참조(맥락/이유)** 로 위임한다.

### 3축 분리

| 축 | 위치 | 내용 |
|-----|------|------|
| **상태축 (What)** | 진실원 (예: `deprecated.md` / classifier.rs `check_sensitive_and_pii` / `mcp_tool_catalog_full`) | 현재 무엇이 그러한가 |
| **시간축 (Why)** | 참조원 (예: `architecture.md` / `architecture-archive.md`) | 왜 그 Phase에 그 결정 |
| **링크** | 참조원 → 진실원 | `→ 참조` 1줄 |

### 3요소 동반 필수 (정식 승격 기준)

1. **진실원 선언** — 양쪽 문서 머리말 / 코드 docstring에 역할 명시
2. **검증 grep** — phase 종결 시 자동 grep으로 중복 검출 (자동화 도구: `spec/benchmarks/scripts/single_source_check.sh`, 2026-06-05 신규)
3. **갱신 규칙** — 신규 변경 시 진실원만 갱신, 다른 곳은 결정 맥락 + 링크만

### 누적 사례 (9건 도달, lesson 73/74 2026-06-05 자기 적용 추가)

| lesson | 영역 | 진실원 | 참조원 |
|--------|------|--------|--------|
| 49 | spec 문서 | `deprecated.md` | `architecture-archive.md` |
| 50-A (Phase 91) | 코드 (검사 함수) | classifier.rs `check_sensitive_and_pii` | service.rs / commands.rs |
| 50-B (Phase 91) | 코드 (검증 함수) | `reasoning/verifier.rs` Verifier | 분산 호출처 |
| 51 (Phase 92) | 데이터 (MCP 카탈로그) | `mcp_tool_catalog_full()` | `mcp_tool_catalog` (wrapper) + 일치성 테스트 |
| 52 (Phase 93) | 데이터 흐름 (백엔드→frontend) | 백엔드 단일 진입점 | API 4 메서드 → ViewModel 렌더 |
| **55 (Phase 96)** | **spec 문서 (포트 매핑)** | **`spec/domain-map.md`** | **`spec/architecture.md` (수치+결정 맥락만)** |
| **73 (2026-06-05)** | **spec 문서 (mydocsearch 결정 사실)** | **`spec/deprecated.md` §mydocsearch_decision.md** | **`spec/architecture.md` 본문 인용 1건 + 변경 요약 2건 — 모두 단방향 위임. 원본 spec/mydocsearch_decision.md 삭제** |
| **74 (M-1, 2026-06-05)** | **META.md §메타 룰 17 강화** | **단일 §정식 본문 (강화 영역 통합)** | **§메타 룰 17 강화 후보 (Phase 106) + §3건 누적 도달 = 2 위임 표시. 메타 룰 19 자기 적용 — META 본문 자체 자기 위반 해소** |
| **74 (S-5, 2026-06-05)** | **메타 룰 19 §검증 grep 요소** | **`spec/benchmarks/scripts/single_source_check.sh`** | **spec 본문 5종 grep + 위임 누락 후보 출력 — 메타 룰 19 §3요소 동반 의무 중 §검증 grep 자동화 첫 도구** |

### 잠재 적용 후보

- `architecture.md` 탭/노드 카탈로그 ↔ `webapp-design.md` UI 표 ↔ `scenarios.md` 사용자 흐름 (7탭/노드 수치)
- `domain-map.md` ↔ `architecture.md` 도메인 구성도 (포트 목록 중복 가능성)
- 본 META.md ↔ 개별 lesson "공통 교훈" 섹션

## 메타 룰 23: 메타 룰 후보 → 정식 승격 객관 기준 (Phase 99 META 정식)

(lesson 50/53/56/57 후보 등록 + Phase 99 정식 승격 본인 자기 적용)

신규 메타 패턴 발견 시 "후보" → "정식" 전환 시점의 객관 기준 부재로 후보 누적 지연. 본 룰은 승격 기준을 명문화하여 후보 인플레이션 차단 + 정식 승격 일관성 확보.

### 승격 3 요소 (AND 조건)

정식 승격을 위해 다음 3 요소 **모두** 충족 필요:

1. **누적 사례 ≥ 3건** (서로 다른 lesson 또는 phase)
   - 단, **1건이라도 패턴이 명확히 일반화 가능**하면 즉시 정식 가능 (예: 메타 룰 13 인프라 4단계는 1건만으로 명확)
   - 동일 패턴 반복 발생 = 메타 가치 증명
2. **신규 작업 체크리스트** 또는 **자동화 도구** 보유
   - "발견했다"만으로 부족 — 다음 phase가 자기 적용 가능한 행동 지시 필수
   - 예: 메타 룰 17 = release_rebuild_required.sh, 메타 룰 24 = audit_stage_check.sh
3. **단일 진실원 위치** META.md에 본문 등재
   - 후보 본문이 lesson에만 분산되면 메타 룰 19 자기 위반 (Phase 99 본 작업이 사례)

### 후보 본문 등재 의무

신규 후보 발견 시 즉시 META.md에 "후보" 섹션으로 본문 등재. lesson 본문에만 분산 등록 금지 (메타 룰 19 자기 적용).

```markdown
## 메타 룰 N 후보: {제목}

(lesson X / Y / Z 누적 M건)

### 핵심
{1-2 문장 요약}

### 누적 사례
| lesson | 영역 | 패턴 |
...

### 정식 승격 조건 (메타 룰 23)
- [ ] 누적 ≥ 3건 (현재 M)
- [ ] 체크리스트/자동화 도구
- [ ] META.md 본문 등재 ✅
```

### 누적 사례 (Phase 99 본 룰 자기 적용 1건)

| lesson | 영역 | 패턴 |
|--------|------|------|
| **본 룰 자체 (Phase 99)** | 메타 룰 승격 기준 부재로 후보 7건 누적 인플레이션 위험 | 3 요소 AND 조건 명문화 |

### 본 룰 자기 적용 (Phase 99 메타 룰 25 자기 적용)

본 룰은 1건 누적이지만 **즉시 정식 가능** (조건 1의 예외 조항 적용 — 1건만으로 패턴 명확). 본 룰의 정식 승격은 후보 22/24/25/26/27의 즉시 평가 가능하게 함.

**Phase 99 동시 정식 승격 후보들 평가**:

| 후보 | 누적 | 체크리스트 | META 등재 | 평가 결과 |
|------|------|----------|----------|---------|
| 21 (본질/부수 도메인) | 2건 | ✅ | ✅ | ❌ 1건 부족 (외부 분석 1건 추가 시) |
| 22 (사용자 정책 합의) | 2건 | ✅ | ✅ | ❌ 1건 부족 |
| **23 (본 룰)** | 1건 (예외) | ✅ | ✅ | ✅ **Phase 99 정식** |
| 24 (stage 명명) | 2건 | ✅ audit_stage_check.sh | ✅ | ❌ 1건 부족 |
| **25 (자기 적용 의무)** | 3건 ✅ | ✅ (보강 완료) | ✅ | ✅ **Phase 99 정식** |
| **26 (match 스코프)** | 3건 ✅ | ✅ | ✅ | ✅ **Phase 99 정식** |
| 27 (게이트 vs 점검) | 1건 | ✅ | ✅ | ❌ 2건 부족 |

→ Phase 99 정식 승격 **3건** (23 + 25 + 26). 잔여 후보 4건 (21/22/24/27)은 추가 누적 대기.

---

## 메타 룰 22: 사용자 정책 경계 명시 합의 의무 (Phase 104 META 정식 승격)

(lesson 51 / 53 / 59 / 62 누적 4건 → Phase 104 정식 승격, 메타 룰 23 §승격 3요소 모두 충족)

### 핵심
A vs B vs C 옵션 같이 사용자 도메인 정책 경계 결정이 필요한 시점에 claude가 임의 진행하지 않고 사용자 명시 합의 후 진행. 결정 사실은 lesson + spec/architecture.md 양쪽 기록.

### 누적 사례 (15건 도달, 2026-06-05 본 세션 +4건: mydocsearch 즉시 삭제 + 6 묶음 + 75 (a)/(b)/(c))
| lesson | phase | 영역 |
|--------|------|------|
| 51 | Phase 92 | 외부 협업 보류 정책 (RBAC / Change Request) |
| 53 | Phase 94 | AuditPort 헥사고날 정공법 vs 어댑터 직접 (A vs B 옵션) |
| 59 | Phase 100 | Settings IA — 5 운영 카드 좌측 그룹 옵션 (A 단일 / B 개별 5 / C 2그룹) |
| 62 (Phase 103) | Phase 103 | GraphRAG 흡수 옵션 (제안만 / low 자동 / dry-run + apply 등 4 옵션) + 본 GraphRAG 흡수 전체 수행 합의 |
| 65 (Phase 106) | Phase 106 | 온보딩 저장 위치 / 100개 마일스톤 / 크레덴셜 흐름 / optimize 표시 → 사용자 결정: 별도 DB 없음 + 명시 버튼 + 단순 흐름 |
| 66 (Phase 107) | Phase 107 | dev seed credential in-memory 결정 (release 환경 오염 방지) + Processing+Verification 탭 통합 (작업 흐름 IA) |
| 2026-06-01 본질 재정의 1차 (무효화) | Phase 108 진입 전 | 검색 도메인 분리 옵션 4축 → 무효화됨 (2026-06-04 결정으로 흡수) |
| 70 (Phase E1+E2+E3) | 본 세션 | Grimoire 흡수 범위 4 옵션 (E1만 / E1+E2+E3 / prd만 / 보류) → E1+E2+E3 결정 |
| 71 (Linux cross-build) | 본 세션 | cargo-xwin 방식 4 옵션 + 배포 대상 (Linux/Windows) 4 옵션 + 정리 범위 3 옵션 + 동기화 범위 3 옵션 (총 4축) |
| **72 (본질 재정의 2차)** | **본 세션** | **tasty 패턴 흡수 4축**: (1) host 경계 (최소/표준/극소) (2) search-extraction-plan 처리 (폐기/잔존/병행) (3) 진입 범위 (문서만/Phase 200/Phase 203) (4) 폐기 처리 (mydocsearch 동시) → **단일 진실원: `prd/research/plugin-architecture-2026-06-04.md`** |
| **73 (mydocsearch 즉시 삭제)** | **Phase 200 시리즈 진입 전 2026-06-05** | **spec 분석 Q1 형식 — "Phase 203 대기 / 지금 이관" 옵션 → 사용자 Q1 적용 → 4건 동시 처리 (deprecated.md 흡수 + 본문 인용 위임 + 변경 요약 stale 해소 + 원본 삭제). 메타 룰 22+19 결합 패턴 첫 사례** |
| **74 (6 묶음 처리 세션)** | **Phase 200 시리즈 진입 전 2026-06-05** | **사용자 "1~6 진행해" 단일 트리거 → 6 묶음 처리 (M-1 + S-4 + S-1 + S-6 + P-2 + S-3 + S-5) + 사이드 G1/G2 발견 + 자동화 2종 신규 + Phase 200 baseline 보존 + 메타 룰 25 5건 자기 적용. 1줄 트리거 비대칭 비용 패턴 강화 사례** |
| **75 (a) P-4/P-5 원격 빌드 분류** | **본 세션 2026-06-05** | **"빌드는 원격 서버에서 할꺼야" — release 빌드 전체 원격 위임. feedback_remote_build_only 1차 메모리 등재** |
| **75 (b) 개발/빌드 모두 원격** | **본 세션 2026-06-05** | **"개발/빌드는 원격서버에서만 실행" — cargo check까지 확장. feedback_remote_build_only 강화. 로컬 cargo 명령 전면 금지** |
| **75 (c) binary plugin 4축 합의** | **Phase 200 진입 시점 2026-06-05** | **Q1 빌드 위치 (_rust_module/) + Q2 형제 모듈 = plugin + Q3 PIPELINE_BASE/plugins/ 자동 + Q4 본 세션 진입. plugin-architecture-2026-06-04.md §2-A 재정의 (번들 → 별도 빌드). 단일 세션 +3건 첫 사례** |

### 정식 승격 (Phase 104 메타 룰 23 자기 적용 — 3 요소 모두 충족)
- [x] 누적 ≥ 3건 — **7건 도달** ✅ (2026-06-01 본질 재정의로 +1)
- [x] 체크리스트: "사용자 결정 영역 식별 시 즉시 AskUserQuestion 호출"
- [x] META.md 본문 등재 ✅ (Phase 99 + Phase 104 정식 승격 갱신)

### 신규 작업 시 사전 체크리스트
- [ ] 사용자 도메인 영향 결정(IA / 정책 / 외부 연동 / 옵션 분기) 식별 시 즉시 AskUserQuestion 호출
- [ ] 옵션 2~4개 제시 의무 (claude 추천 옵션 첫 위치 + "추천" 표기)
- [ ] 사용자 합의 후 lesson 본문에 결정 사실 기록
- [ ] claude 임의 진행 금지 — 메타 룰 7 (답할 수 있는 질문) 결합 적용

---

## 메타 룰 24 후보: stage 명명 규칙 정형화

(lesson 54 Phase 95 자기 적용 + Phase 97 audit_stage_check.sh 자동화)

### 핵심
audit.record stage는 `{영역}.{도구명}[.{sub}]` 규칙. 영역 prefix는 허용 목록 (llm/mcp/tauri/remote/verify/service)에서만 선택. 신규 영역 추가 시 audit_stage_check.sh ALLOWED 업데이트 의무.

### 누적 사례 + 자동화 도구
| 시점 | 사례 |
|------|------|
| Phase 95 (lesson 54) | `tauri.search` / `mcp.kg_*` / `remote.{backend}.upload.*` 정형화 |
| Phase 97 (lesson 56) | `audit_stage_check.sh` 자동화 PASS (10 정적 + 1 동적 prefix) |

### 정식 승격 조건 (메타 룰 23)
- [x] 누적 ≥ 3건 — 현재 **2건** (1건 추가 필요)
- [x] 자동화 도구: `spec/benchmarks/scripts/audit_stage_check.sh`
- [x] META.md 본문 등재 ✅ (Phase 99)

→ 1건 추가 누적 시 정식 승격.

---

## 메타 룰 25: 메타 룰 자기 적용 의무 (Phase 99 META 정식 승격)

(lesson 49 / 53 / 55 누적 3건 → Phase 99 정식 승격)

### 핵심
메타 룰 정식 승격 시 즉시 본 룰의 다른 영역 자기 적용 의무. 발견 lesson 외 다른 코드/문서 영역에서도 같은 패턴 검출 + 적용. 후속 phase 자기 적용 누락 방지.

### 누적 사례 (8건 도달, lesson 74 본 세션 +5)
| lesson | 영역 | 자기 적용 |
|--------|------|---------|
| 49 | spec 문서 (메타 룰 19 → deprecated.md 단일 진실원) | archive ↔ deprecated 위임 |
| 53 | 메타 룰 1 sub-rule 분리 → 19건 카테고리화 | META.md 자체 자기 적용 |
| 55 | 메타 룰 19 spec/domain-map.md 단일 진실원 선언 | 포트 매핑 위임 |
| **74 (a)** | **메타 룰 17 강화 정식 직후 lesson 71 meta_rules 갱신** | **lesson 71 본 사례 정식 승격 사실 명시** |
| **74 (b)** | **메타 룰 17 강화 정식 직후 external-trigger-checklist B-8** | **"17 강화" 행 "✅ 정식 승격" 갱신** |
| **74 (c)** | **메타 룰 17 강화 정식 직후 release_redeploy.sh 신규** | **§자동화 도구 후보 → 게이트 도구 등재 (S-6)** |
| **74 (d)** | **메타 룰 19/30 자기 적용 직후 single_source_check.sh 신규** | **§검증 grep 자동화 도구 (S-5)** |
| **74 (e)** | **본 세션 6 묶음 처리 직후 lesson 74 등재** | **메타 룰 정식 승격 → 자동화 → 자기 적용 → lesson 등재 전체 사이클 한 세션 종결 첫 사례** |

### 정식 승격 (Phase 99 메타 룰 23 자기 적용 — 3 요소 모두 충족, 체크리스트 보강)
- [x] 누적 ≥ 3건 — **3건 도달** ✅
- [x] 체크리스트: 아래 §신규 작업 시 사전 체크리스트
- [x] META.md 본문 등재 ✅ (Phase 99)

### 신규 작업 시 사전 체크리스트
- [ ] 메타 룰 정식 승격 lesson 작성 시: 즉시 본 룰의 다른 영역 grep 의무 (`grep -rn "{관련 키워드}" spec/ src/ prd/`)
- [ ] 발견된 추가 적용 영역 lesson 또는 spec 내 즉시 반영 (다음 phase 미루지 않음)
- [ ] 메타 룰 자체에 "자기 적용 사례" 표 갱신 (메타 룰 19의 누적 사례 표 패턴)

---

## 메타 룰 26: match 케이스 스코프 사전 명시 의무 (Phase 99 META 정식 승격)

(lesson 50 / 52 / 56 누적 3건 → Phase 99 정식 승격)

### 핵심
Rust `match { Variant1 => { ... }, Variant2 => { ... } }` 같은 큰 match block 안의 변수는 case 외부에서 비공유. 다른 case에서 같은 변수명 사용 시 추정 빗나감. 사전 grep으로 case 경계 + 스코프 사전 확인 의무.

### 누적 사례
| lesson | 추정 빗나감 |
|--------|----------|
| 50 (Phase 91) | service.rs `is_sensitive_with_content` 활성 분기 추정 → 실제는 deprecated |
| 52 (Phase 93) | `state.paths` 추정 → 실제 `state.settings_db_path` |
| 56 (Phase 97) | `trace`/`inputs_hash` PipelineStep::Llm 스코프 추정 → 실제 PipelineStep::Verify 별도 스코프 |

### 정식 승격 (Phase 99 메타 룰 23 자기 적용 — 3 요소 모두 충족)
- [x] 누적 ≥ 3건 — **3건 도달** ✅
- [x] 체크리스트: "match 케이스 내부 변수 사용 시 grep으로 정의 case 사전 확인"
- [x] META.md 본문 등재 ✅ (Phase 99)

### 신규 작업 시 사전 체크리스트
- [ ] 큰 match block 안의 변수 사용 시 `grep -B 30 "{변수명}" {파일}.rs | grep "match\|=>"` → 어느 case에서 정의되는지 확인
- [ ] 다른 case에서 같은 변수명 사용 시 별도 변수 생성 (`verify_trace`처럼 prefix 부여)
- [ ] 본질적 공유가 필요하면 match 외부로 변수 추출 (PipelineStep 케이스 전체에서 공유 가능 변수)

---

## 메타 룰 28: 내부 코드명·Phase 번호 UI 노출 금지 (Phase 104 META 정식 승격)

(메모리 feedback_no_phase_in_ui (2026-05-08) + Phase 92 H1 + Phase 101 일괄 누적 3건 → Phase 104 정식 승격, 메타 룰 23 §승격 3요소 모두 충족)

### 핵심
사용자 가시 영역(라벨, 모달 제목, 알림 메시지, placeholder, description)에 내부 추적 코드명(C1/C2/A1/B1/H1/G1 등 외부 프로젝트 분류 + Phase 76 등 마일스톤) 노출 금지. 코드 주석 / spec / lesson-learned / git 메시지 / HTML 주석 / JS 함수명 / HTML id는 그대로 보존 (추적성 유지).

### 누적 사례 (3건 도달 → 정식 승격)
| 시점 | 사례 |
|------|------|
| Phase 76 (memory 2026-05-08) | "다축 프로파일 기반 추천 (Phase 76)" 모달 제목 — 사용자 직접 제거 요청 |
| Phase 92 (lesson 51) | Verification 카드 "자동 롤백이 아닌 사용자 검토 권고 시스템 (Phase 92 H1)" |
| Phase 101 (lesson 60) | (C1) 5건 + (C2) 1건 + (Phase 92 H1) 1건 + 그룹 설명 1건 = 8건 일괄 |

### 정식 승격 (Phase 104 메타 룰 23 자기 적용 — 3 요소 모두 충족)
- [x] 누적 ≥ 3건 — **3건 도달** ✅
- [x] 체크리스트: 신규 UI 라벨 작성 시 grep 의무 (아래 §체크리스트)
- [x] META.md 본문 등재 ✅ (Phase 101 + Phase 104 정식 승격 갱신)

### 신규 작업 시 사전 체크리스트
- [ ] 신규 UI 라벨/메시지 추가 시: `grep -E 'Phase \d+|\([A-Z]\d?\)' <변경 파일>` 의무
- [ ] 외부 프로젝트 명명(Ruflo C1/A1 / GraphRAG G1~G4 등) 차용 시: 내부 도메인 이름으로 번역 우선
- [ ] 사용자 메시지 / 에러 메시지에 내부 식별자 포함 금지 (예: `"C1 ${key} 저장"` → `"${key} 저장"`)
- [ ] JS 함수명 / HTML id / 코드 주석은 보존 (추적성 우선 — UI 라벨만 변경)
- [ ] **메모리 `feedback_no_phase_in_ui.md`는 본 룰 정식 승격 후 archive 검토** (메타 룰 25 자기 적용 — 메모리→spec 메타 승격 패턴 첫 사례)
- [ ] **Phase 103 G1/G2/G3/G4 식별자 사전 grep** — 본 룰 정식 직후 GraphRAG 흡수 영역에 (G1) 같은 라벨 노출 0건 확인 (메타 룰 25 자기 적용)

---

## 메타 룰 17 강화 후보 → 정식 승격됨 (2026-06-05, 위 §메타 룰 17 정식 섹션 참조)

본 섹션은 **메타 룰 19 자기 적용** + **메타 룰 25 자기 적용** — 2026-06-05 강화 정식 승격 후 후보 본문은 본 위임 표시로 대체. 단일 진실원은 위 §메타 룰 17 정식 섹션 (`## 메타 룰 17: 코드 변경 phase의 release 빌드 + 배포 시점 의무화`). lesson 71 누적 3건 도달로 메타 룰 23 §승격 3요소 모두 충족 확인.

---

## 메타 룰 30: spec 본문 phase별 즉시 갱신 의무 (Phase 본질 재정의 2차 META 정식 승격)

(lesson 64 / 65 / 66 / **67 / 72 / 73** 누적 6건 → 2026-06-04 정식 승격, 메타 룰 23 §승격 3요소 모두 충족)

### 핵심
Phase 종결 시 spec 본문 영향 여부를 grep으로 점검 + 즉시 갱신. lesson 본문 / roadmap 갱신만 진행 + spec 본문 사용자 명시 "현행화" 트리거 의존 → 누적 stale 회귀. **"Phase N 진입 시 처리" 표시 spec 잔존물도 같은 누적 stale 위험** (lesson 73).

**대상 spec 영역 (lesson 67로 확장)**:
- `spec/architecture.md`
- `spec/domain-map.md`
- `spec/webapp-design.md`
- `spec/deprecated.md`
- **`src/CLAUDE.md`** (lesson 67 확장 — Claude 전용 컨텍스트도 spec 영역)

### 누적 사례 (12건 도달, 본 세션 +6)
| lesson | phase | 패턴 |
|--------|------|------|
| 64 | Phase 105 | 본 룰 발견 — 100~104 종결 시 spec 본문 5건 누적 stale 일괄 갱신 |
| 65 | Phase 106 | phase 종결 시점 architecture/webapp-design 즉시 갱신 |
| 66 | Phase 107 | 8건 묶음 phase 종결 직후 architecture / roadmap / external-trigger / META 일괄 갱신 |
| **67** | **세션 2026-06-04** | **`src/CLAUDE.md` 4건 stale 발견 — spec 자기 적용 영역 확장 (sub-rule 1g)** |
| **72** | **본질 재정의 2차 2026-06-04** | **4건 동시 갱신 (CLAUDE.md + architecture + domain-map + deprecated) — search-extraction-plan 무효화 + plugin-architecture 신규 진실원** |
| **73** | **Phase 200 시리즈 진입 전 2026-06-05** | **"Phase 203 진입 시 처리" 표시 spec(`mydocsearch_decision.md`)의 즉시 처리 — 사용자 합의 트리거로 다음 phase 대기 없이 4건 동시 (deprecated.md 흡수 + 본문 인용 위임 + 변경 요약 stale 해소 + 원본 삭제). 메타 룰 22+19 결합 패턴 첫 사례** |
| **74 (a)** | **본 세션 S-1 2026-06-05** | **`webapp-design.md` 헤더 stale 갱신 + status_note 추가 (본문 6탭 ↔ 본질 1도메인 host 불일치는 Phase 208 미루기 명시)** |
| **74 (b)** | **본 세션 S-4 2026-06-05** | **spec "Phase N 진입/종결 시" 표기 전수 grep — `architecture.md:145` Pipeline 이관 검토 → 결정 완료 표시 추가. lesson 73 패턴 자기 적용** |
| **74 (c)** | **본 세션 S-5 2026-06-05** | **`single_source_check.sh` 자동화 신규 — 메타 룰 30 §자동화 도구로 등재. lesson 49 sub-rule 1g 자동화** |
| **74 (d)** | **본 세션 G2 2026-06-05** | **architecture.md 본문 수치 stale 일괄 동기화 (dead_selector_scan 88/92 → 94 / action_catalog 68 → 72) — P-2 baseline 측정 중 자기 발굴 + 본 현행화 묶음 흡수** |
| **75 (Q3 spec 즉시 갱신)** | **본 세션 종결 시점 2026-06-05** | **Phase 200/201/202 placeholder 진입 직후 spec/architecture.md §누적 변경 요약 신규 + spec/domain-map.md §Plugin 도메인 신규 동시 추가. lesson 75 본문 + INDEX + META 표 +1 동시. 메타 룰 30 자기 적용 11건째 — placeholder 진입과 spec 갱신이 한 세션 종결 첫 사례** |
| **75 (prd 현행화)** | **본 세션 종결 시점 2026-06-05** | **사용자 "프로젝트 현행화 해" 2차 트리거 → roadmap.md §본 세션 종결 신규 + 완료 수치 표 갱신 + external-trigger-checklist B-8 표 +3행 + §본 세션 신규 항목 표 +9행 + 위생 게이트 4 PASS 검증. 메타 룰 30 자기 적용 12건째** |

### 정식 승격 (2026-06-04 메타 룰 23 자기 적용 — 3 요소 모두 충족)
- [x] 누적 ≥ 3건 — **5건 도달** ✅
- [x] 체크리스트: phase 종결 직전 영향 영역 grep + 즉시 갱신 (4 spec + CLAUDE.md)
- [x] META.md 본문 등재 ✅

### 신규 작업 시 사전 체크리스트
- [ ] Phase 종결 시 다음 5 영역 grep:
  - `grep -l "{변경 키워드}" spec/architecture.md spec/domain-map.md spec/webapp-design.md spec/deprecated.md src/CLAUDE.md`
- [ ] 변경 검출 시 즉시 갱신 (사용자 "현행화" 트리거 대기 금지)
- [ ] 무효화 결정 시 deprecated.md 단방향 위임 + 본문 헤더 무효화 표시 (lesson 49 패턴)

### 자동화 도구

- `spec/benchmarks/scripts/single_source_check.sh` — spec 본문 5종에서 "삭제/폐기/제거" 키워드 grep + 단일 진실원 위임 표시 누락 후보 출력 (**2026-06-05 신규**, lesson 49 sub-rule 1g 자동화). 메타 룰 19 §검증 grep 요소 동시 충족. 점검 분류(메타 룰 27, false positive 가능).

### Sub-rule 후보: 회귀 게이트/도구 자체의 stale 자기 검출 (lesson 74 G1, 2026-06-05 신규 후보)

#### 핵심
회귀 게이트는 일반적으로 **코드 회귀**만 검출한다고 인식되지만, **baseline 측정 자체가 게이트 도구 자신의 stale을 자기 검출**한다는 새 패턴. Phase 종결 시 코드/UI 변경뿐 아니라 회귀 자동화 스크립트도 변경 영향 점검 영역에 포함해야 함.

#### 발견 메커니즘
- Phase N에서 코드/UI 변경 발생 (예: Phase 107 verification → processing 통합)
- 회귀 자동화 스크립트 갱신 누락 (메타 룰 30 자기 위반)
- Phase N+M 시점 baseline 측정 시 스크립트 FAIL → 자기 발견

#### 누적 사례 (1건)

| 시점 | 도구 | 변경 영역 | 발견 |
|------|------|---------|------|
| lesson 74 G1 (본 세션 P-2) | `gui_http_smoke.sh` Test 5 (7탭 검사) | Phase 107 verification → processing 흡수 (7→6탭) | P-2 baseline 측정 시 FAIL → 즉시 해소 (6탭 grep으로 갱신) |

#### 정식 승격 조건 (메타 룰 23)

- [ ] 누적 ≥ 3건 — 현재 **1건** (2건 부족)
- [x] 체크리스트: phase 종결 시 코드/UI 변경 영역과 회귀 자동화 스크립트 영역 grep 매칭
- [x] META.md 본문 등재 ✅ (2026-06-05 후보 등재)

→ 2건 추가 누적 시 정식 승격 검토. 누적 가능 영역: dead_selector_scan 화이트리스트 / action_catalog 카운트 / audit_stage_check ALLOWED 목록 / gui_http_smoke 5종 검증 / release_rebuild_required 파일 확장자 패턴.

#### 신규 작업 시 사전 체크리스트 (후보 상태)

- [ ] Phase 종결 시 코드/UI 변경 영역과 매칭되는 회귀 자동화 스크립트 grep 의무 (예: 탭 변경 → gui_http_smoke 7탭 검사 / 메뉴 변경 → action_catalog 화이트리스트)
- [ ] 스크립트 grep 패턴이 변경 영역을 명시적으로 참조하면 영향 검증 필수
- [ ] baseline 측정 결과를 phase 종결 직후 1회는 수행 (도구 stale 자기 검출 기회)

#### 본 sub-rule의 메타 가치

- 회귀 게이트의 검출 범위가 **코드 회귀 → 코드 회귀 + 도구 stale** 양면으로 확장
- baseline 측정의 빈도 가치 강화 (단순 비교 기준이 아닌 자기 진단 메커니즘)
- 메타 룰 30 sub-rule 확장 후보 (코드/UI 영역 외 자동화 영역 신설)

---

## 메타 룰 17 강화 후보 (3건 누적) → 정식 승격됨 (2026-06-05, 위 §메타 룰 17 정식 섹션 참조)

본 섹션은 **메타 룰 19 자기 적용** — 2026-06-05 강화 정식 승격으로 본문 위임. 단일 진실원은 위 §메타 룰 17 정식 섹션. 누적 3건 (Phase 106 / Phase 107 / lesson 71) 도달 + 자동화 도구(`release_rebuild_required.sh`) 보유 + META 본문 등재 = 메타 룰 23 §승격 3요소 모두 충족.

자기 적용 (정식 승격 후 즉시, 메타 룰 25):
- ✅ lesson 71 본문에 메타 룰 17 강화 정식 승격 사실 반영
- ✅ external-trigger-checklist B-8 표 "17 강화" 행 "✅ 정식 승격" 갱신
- ✅ `release_redeploy.sh` 자동화 도구 후보 등재 (3건 누적 도달 시 신규 작성)

---

## 메타 룰 31 후보 (Phase 107 등록): 메뉴 IA — 도메인 분류 vs 작업 흐름 분류 트레이드오프

(lesson 66 누적 1건)

### 핵심
탭/섹션 분리 결정 시 두 기준 트레이드오프 명시:
- **도메인 분류** — 같은 도메인의 기능을 한 곳에 모음 (예: Verification 탭 = 검증 메트릭 + lint + anomaly)
- **작업 흐름 분류** — 사용자의 단일 작업 흐름이 끝나는 곳까지 한 화면에서 가시화 (예: 파일 가공 후 검증까지 한 탭)

작업 흐름 분류가 사용자 click 비용을 줄이지만 메뉴가 비대해질 위험. 도메인 분류는 메뉴는 깔끔하지만 사용자가 한 작업의 진행 상황을 따라가기 위해 여러 탭을 오가야 함.

### 누적 사례 (1건)
| lesson | phase | 결정 | 트레이드오프 |
|--------|------|------|------------|
| 66 | Phase 107 | Processing + Verification → "처리 현황" 단일 탭 | 사용자 명시 요청 "두 메뉴 통합" — 작업 흐름 분류 우선 |

### 정식 승격 조건 (메타 룰 23)
- [ ] 누적 ≥ 3건 — 현재 **1건** (2건 추가 필요)
- [ ] 체크리스트: "신규 탭/섹션 결정 시 도메인 vs 작업 흐름 트레이드오프 명시" (후속 작성)
- [x] META.md 본문 등재 ✅ (Phase 107)

→ 2건 추가 누적 시 정식 승격.

---

## 메타 룰 27: 회귀 게이트 vs 점검 도구 분리 (2026-06-05 META 정식 승격)

(lesson 57 Phase 98 + 본 세션 lesson 74 누적 3건 → 2026-06-05 정식 승격, 메타 룰 23 §승격 3요소 모두 충족)

### 핵심
회귀 게이트는 **결정적 grep / 명확한 ALLOWED 목록**으로 false positive 0에 가까운 경우만 승격. 점검 도구는 **휴리스틱 / 부분 매칭 / 외부 의존**으로 false positive 가능한 경우 사용. 게이트화하면 빌드 부당 차단 위험. 정밀도 임계 검증 후 분류 결정 의무.

### 분류 기준 매트릭스

| 도구 특성 | 게이트 승격 가능 | 점검 분류 |
|----------|---------------|---------|
| 정밀도 | ≥99% (false positive 0~1%) | <99% (false positive 가능) |
| 결정성 | 결정적 (grep / sha256 / git diff / ALLOWED 목록) | 휴리스틱 (heuristic / 외부 CDN / CSS 부모 셀렉터 / 정규식 폭) |
| 외부 의존 | 없음 (또는 Linux/Windows 사전 확인 가능) | 외부 환경(Node + npm install / Tauri 런타임 / 네트워크) |
| exit code | 0 의무 (1+건 발견 시 차단) | 출력만 (exit 0 유지) |
| CI 통합 | 가능 | 권장 안 함 |

### 누적 사례 (5건 도달)

| 도구 | 분류 | 사유 | Phase |
|------|------|------|------|
| `dead_selector_scan.sh` / v2.js | **게이트** | ID 매칭 정밀도 100% | Phase 47/G-5 |
| `dead_selector_scan_v3.js` | **점검** | CSS rule false positive 가능 (부모 셀렉터 / 외부 CDN 한계) | **Phase 98 (lesson 57)** ← 1건째 |
| `audit_stage_check.sh` | **게이트** | ALLOWED 허용 목록 명확 | Phase 97 |
| `release_rebuild_required.sh` | **게이트** | git diff / find -newer 결정적 | Phase 97 |
| **`release_redeploy.sh`** | **게이트** | **sha256 결정적 + tasklist 명확 + D:\file-test 미존재 시 환경 의존 WARN(false positive 회피)** | **2026-06-05 lesson 74 S-6** ← 2건째 |
| **`single_source_check.sh`** | **점검** | **grep 휴리스틱 (DELEGATION_PATTERNS + SKIP_PATTERNS 매칭). 시간축 보존 vs 상태축 위임 판단은 휴리스틱이라 false positive 가능** | **2026-06-05 lesson 74 S-5** ← 3건째 |

### 정식 승격 (2026-06-05 메타 룰 23 자기 적용 — 3 요소 모두 충족)

- [x] 누적 ≥ 3건 — **3건 도달** ✅ (Phase 98 + lesson 74 S-6 + lesson 74 S-5)
- [x] 체크리스트: 위 §분류 기준 매트릭스 (5축: 정밀도/결정성/외부 의존/exit code/CI 통합)
- [x] META.md 본문 등재 ✅ (Phase 99 후보 등재 → 2026-06-05 정식 승격)

### 신규 작업 시 사전 체크리스트

- [ ] 신규 자동화 도구 작성 시: 위 §분류 기준 매트릭스 5축 평가 의무
- [ ] 점검 분류면 본문 docstring + README에 명시 ("점검 분류, 메타 룰 27 — false positive 가능, exit 0 의무 아님")
- [ ] 게이트 분류면 exit 1 발생 시 사용자 안내 메시지 필수 (액션 가능한 명령 명시)
- [ ] Phase 종결 체크리스트(`spec/benchmarks/scripts/README.md` §Phase 종결)에서 게이트는 의무 / 점검은 권장 분리

### 본 룰 자기 적용 (정식 승격 직후, 메타 룰 25)

- ✅ `release_redeploy.sh` docstring에 "메타 룰 27 분류 = 게이트 (false positive 없음 — 결정적 sha256/tasklist)" 명시
- ✅ `single_source_check.sh` docstring에 "분류 (메타 룰 27): 게이트 — 명시적 위임 표기는 결정적 grep 가능. 단 시간축(Why) 기록은 위임 불필요하므로 단순 grep으로는 게이트 승격 어려움. 본 도구는 **점검** 분류." 명시
- ✅ `spec/benchmarks/scripts/README.md` §메타 룰 자동화 4종 표에 "분류 (메타 룰 27)" 컬럼 존재
- ✅ lesson 74 본문 메타 룰 표에 "27 — 누적 +2 (release_redeploy 게이트 + single_source_check 점검)" 명시

### 본 룰의 메타 가치

- 자동화 도구의 분류 결정이 도구 신규 작성 시점에 강제됨 (사후 분류 회피)
- false positive 가능 도구의 게이트화 차단 → 빌드 부당 차단 위험 0
- 메타 룰 17 강화 §자동화 + 메타 룰 19/30 §자동화 모두 본 룰 적용으로 분류 일관성 확보

---

## 메타 룰 21: 외부 도메인 도구 흡수 시 본질/부수 도메인 분리 (Phase 103 META 정식 승격)

(TFM + Mirage + GraphRAG 누적 3건 도달 → Phase 103 정식 승격, 메타 룰 23 §승격 3요소 모두 충족)

### 핵심
외부 도구의 핵심 도메인이 본 프로젝트와 완전히 다를 때:
1. **본질 도메인 일치 영역**: 즉시 흡수 검토 (메타 룰 20 🟢)
2. **부수 도메인 일치 영역**: (운영 지표 / 메타데이터 / 사용 패턴 / 알고리즘) 인프라 선구현 + 측정 도달 시 활성화 (lesson 30 패턴)
3. **불일치 영역**: 명시 보류 + 본 룰 누적 사례 등재

### 메타 룰 20과의 차이

| 측면 | 메타 룰 20 | 메타 룰 21 (본 룰) |
|------|----------|--------------|
| 대상 | 같은 도메인 외부 프로젝트 (예: JAMES = RAG/지식) | 다른 도메인 도구 (예: TabPFN = ML / Mirage = VFS / GraphRAG = 엔터프라이즈) |
| 흡수 비중 | 본질 + 부수 모두 후보 | 부수만 후보 |
| 보류 빈도 | 부분 (RBAC 등 일부) | 본질 영역 전부 |

### 누적 사례 (3건 도달 → 정식 승격)

| 외부 도구 | 본질 도메인 | file-pipeline 일치 | 부수 일치 흡수 | Phase |
|---------|----------|------------------|-------------|------|
| **TabPFN / TFM** | 숫자 테이블 예측 (XGBoost 대체) | 없음 | 이상 탐지 / ETA 예측 | (분석만, 흡수 미진행) |
| **Mirage v0.0.1** | AI 에이전트 VFS | 없음 | MCP 카탈로그 다차원 / Resource capabilities | Phase 92 H3 / H5 |
| **AWS GraphRAG Toolkit** | 엔터프라이즈 RAG (AWS 클라우드) | 없음 (단일 사용자 데스크톱) | G4 TF-IDF 재순위 / G1 Statement 노드 / G2 의미 관계 / G3 Multi-hop 빔 | **Phase 103** (G4 즉시 + G1/G2/G3 인프라 선구현) |

### 신규 작업 시 사전 체크리스트
- [ ] 외부 도구 본질 도메인이 본 프로젝트(단일 사용자 데스크톱 + Rust 단일 바이너리)와 일치 여부 사전 판정
- [ ] 불일치 시 본 룰 적용 → 부수 영역만 흡수 후보화
- [ ] 메타 룰 16 차원 B (외부 솔루션 추상화 매칭) 라벨 부착 의무
- [ ] 보류 영역은 명시 트리거 등재 (예: "v1.0 다중 사용자 도달 시")
- [ ] 누적 사례 본 표 갱신 의무 (메타 룰 25 자기 적용)

### Phase 103 자기 적용 (정식 승격 직후)
- ✅ GraphRAG 분석을 본 룰 3번째 누적 사례로 즉시 등재
- ✅ `prd/research/external-analysis-2026-05-27-graphrag.md` 단일 진실원 작성 (메타 룰 9 자기 적용)
- ✅ G4 즉시 흡수 + G1/G2/G3 인프라 선구현 (lesson 30 패턴)

---

## 메타 룰 21 후보 → 정식 승격 완료 (Phase 103)

본 섹션은 메타 룰 23 §승격 3요소 충족 + 본 phase 자기 적용 완료로 정식 승격. 본 룰 사용은 위 §메타 룰 21 정식 섹션 참조.

### 정식 승격 조건 (메타 룰 23) 충족 확인
- [x] 누적 ≥ 3건 — **3건 도달** ✅ (TFM + Mirage + GraphRAG)
- [x] 체크리스트: 메타 룰 16 차원 B 결합 + 본 룰 사전 체크리스트 5건
- [x] META.md 본문 등재 ✅ (위 정식 섹션)

---

## 사용 방법

새 lesson 작성 시 본 META의 어느 룰에 해당하는지 명시. 신규 메타 패턴 발견 시 본 문서에 추가.

본 문서는 **인덱스**이지 본문이 아님 — 개별 lesson 파일이 원자료. 단, **메타 룰 후보 등록은 META.md 본문 필수** (메타 룰 23 자기 적용, Phase 99).

### 후보 정식 승격 의사결정 흐름

```
신규 메타 패턴 발견
   ↓
META.md 본문 "메타 룰 N 후보" 섹션 즉시 등재 (메타 룰 23 §2)
   ↓
누적 ≥ 3건 + 체크리스트/자동화 도구 + META 본문 등재 (메타 룰 23 §승격 3요소)
   ↓
정식 승격 (섹션 번호 유지, "후보" → 정식 변경, "(Phase X META 정식 승격)" 표기)
   ↓
정식 승격 lesson 작성 시 즉시 다른 영역 자기 적용 (메타 룰 25 — 정식 승격 후 적용 예정)
```
