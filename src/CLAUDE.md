# file-pipeline — Claude Code 지침

## 세션 시작 절차 (필수)

> 이 절차를 완료하기 전에 코드를 작성하지 마라. 2-Pass 가공·검증 파이프라인은 맥락 없이 수정하면 품질 회귀가 발생한다.

1. `spec/lesson-learned/INDEX.md` → 나열된 파일 전체 읽기 — 과거 실수 확인
2. `spec/architecture.md` 읽기 — 현재 아키텍처·수치 파악
3. `spec/classification_and_verification.md` 읽기 — 검증 시스템 이해
4. 작업 관련 소스 코드 읽기
5. 구현 시작

## 기능 완료 시 문서 동기화 (필수)

기능 구현 완료 → `cargo nextest run --all` 통과 후 **즉시** 아래 파일을 갱신한다:

1. `spec/architecture.md` — 수치표·플로우·파일 트리 업데이트
2. `spec/classification_and_verification.md` — 검증 관련 변경 시
3. `spec/lesson-learned/` — 오류·교훈 발생 시 comm-log 규약에 따라 분리 파일 추가

2개 이상 기능을 연속 구현할 때는 **중간 동기화 필수**. 마지막에 몰아서 갱신하지 않는다.

교훈이 반복적으로 적용되는 규칙이면 이 CLAUDE.md에 승격시킨다.

## 빌드

빌드 시 **workspace + Tauri GUI 모두** 확인해야 한다. GUI(app)는 workspace에서 exclude되어 있으므로 별도 빌드 필수.

```bash
# 1. workspace (core, adapters, shared, cli, mcp)
cd src && cargo check --all

# 2. Tauri GUI (별도)
cd src/modals/app && cargo check

# release 빌드
cd src && cargo build --release --all          # workspace
cd src/modals/app && cargo build --release     # Tauri GUI

# fastembed feature 활성 빌드 (BGE-M3 임베더 + Cross-Encoder 리랭커, Phase 62)
cd src && cargo build --release --all --features file-pipeline-shared/fastembed
cd src/modals/app && cargo build --release --features fastembed
```

### fastembed feature 빌드 요구사항 (Phase 62)

`fastembed` feature 활성 시 ort-sys (ONNX Runtime C++ 바인딩) 정적 라이브러리 링크가 필요:

- **MSVC C++ 워크로드 필수**: VS 2022 Build Tools에 "Desktop development with C++" 워크로드 설치
- **MSVC v14.38+** (VS 2022 17.8+): `ls "/c/Program Files (x86)/Microsoft Visual Studio/2022/BuildTools/VC/Tools/MSVC/"` 결과 v14.38 이상 디렉토리 존재 확인
- **Windows SDK 10.0.19041.0+**

미충족 시 LNK2019 에러 50건 발생 (lesson-learned #18 참조).

기본 빌드(`fastembed` feature 비활성)는 영향 받지 않음 — 새 환경에서 실패하면 일반 빌드로 회귀하여 진단.

- GUI 바이너리: `modals/app/target/release/file-pipeline-tauri.exe`
- CLI 바이너리: `target/release/pipeline.exe`
- 배포 시 GUI 바이너리를 `pipeline.exe`로 rename하여 배포
- **UI 진입점**: `ui/index.html` 단일 파일. `dashboard.html`은 존재하지 않음 (삭제됨).
  CSS/JS 경로는 상대경로(`dashboard.css`, `dashboard.js`).

## 테스트 실행

- **기본 명령**: `cargo nextest run --all`
- nextest 설정: `.config/nextest.toml`
- 느린 테스트(60초+)는 SLOW 경고 표시됨
- 벤치마크 필터: `cargo nextest run --all -E 'test(/bench/)'`
- 외부 서비스 의존 테스트(bench_real, bench_qdrant)는 서비스 미실행 시 자동 스킵

## 성능 회귀 기준선

아래 수치 이하로 떨어지는 변경은 **성능 회귀**이므로 원인을 조사하고 해결한 뒤 커밋한다.

| 지표 | 기준선 | 측정 조건 |
|------|--------|-----------|
| per-doc 오버헤드 | p95 ≤ 100ms | 100문서 마이크로 벤치 (bench_micro, 3회 중앙값) |
| stub 가공 처리량 | 13 docs/sec 이상 | 100문서 배치 모드 (교차참조 auto) |
| stub 가공 처리량 | 60 docs/sec 이상 | 5,000문서 (교차참조 off) |
| 컴파일 경고 | 0건 | `cargo check --all` |
| 테스트 통과 | 전체 통과 | `cargo test --all` |

## 아키텍처 규칙

- **배치 재계산 원칙**: upsert/process 경로에서 비용이 큰 연산(persist, mmap refresh, state save, **HNSW 재빌드**)은 반드시 batch_mode 체크를 포함. 개별 처리 시 매회 실행, 배치 시 batch_end()에서 1회만 실행. 위반 시 per-doc 오버헤드가 O(N)으로 증가 (Phase 45~47에서 3.5x+17x 개선의 핵심).
- 헥사고날: core → adapters 참조 금지. 포트 trait만 참조.
- 타입을 바꾸지 않고 필드를 추가한다 (lesson-learned #5).
- 외부 크레이트 소스를 코드 작성 전에 읽는다 (lesson-learned #4).
- `unwrap()` 금지 → `expect("설명")` 사용 (lesson-learned #7).
- 자동 치환(sed 등) 후 수동 검토 필수 (lesson-learned #6).
- **구조체 필드 추가 = lib + 통합 테스트 동시 갱신** (lesson-learned #21/#27). `cargo check --workspace`는 lib만 검사하므로, 핵심 도메인 구조체(`FileProcessingService` 등) 필드 추가 시 `cargo build --tests --workspace`로 통합 테스트 빌드 확인 필수. **신규 통합 테스트는 `file_pipeline_shared::test_helpers::ServiceBuilder` 사용 의무** — `ServiceBuilder::new(base).with_*(...).build()` 형태로 작성하면 향후 도메인 필드 추가 시 테스트 변경 0건. 기존 12파일은 점진 마이그레이션 예정.
- **기능 제거 = 통합 테스트 단언 grep** (lesson-learned #13/#19/#28). UI/기능 제거 시 lesson 19의 10단계 + 통합 테스트 grep을 함께 수행: `grep -rln "{기능명}\|{함수명}\|{필드명}" modals/*/tests/`.
- **메타 룰 인덱스**: `spec/lesson-learned/META.md` 참조 — "다중 위치 동기화 누락"(메타 룰 1, 7 sub-rule 1a~1g) 등 **14건 정식 메타 룰 + 3건 후보**를 신규 작업 사전 체크리스트로 사용. **회귀 자동화 9종** 동시 호출 의무 (`spec/benchmarks/scripts/README.md` §Phase 종결 체크리스트).

## UI 규칙

- **MVVM 패턴 필수**: View(HTML/CSS) ↔ ViewModel(JS 상태·바인딩) ↔ Model(REST API)을 분리한다. View에 비즈니스 로직을 넣지 않는다.
- **디자인 토큰화 필수**: 색상·간격·폰트·보더 등 모든 시각 속성은 CSS 변수(토큰)로 정의하고, 컴포넌트에서 토큰만 참조한다. 하드코딩된 `rgb()`/`#hex` 값을 직접 사용하지 않는다.
- 신규 UI 작업 시 기존 토큰 목록을 먼저 확인하고, 없으면 토큰을 추가한 뒤 사용한다.

## 프로젝트 구조

```
crates/core/         — 도메인 모델, 포트 trait, service, reasoning/verifier
crates/adapters/     — driven (embedding/llm/vector_db/reranking/storage/notification/preprocessing/verification) + driving (watcher/terminal_*)
crates/shared/       — config, settings_db, mcp_server, setup_modules/rules, cached_llm, audit_anomaly, tray, test_helpers/ServiceBuilder
modals/cli/          — CLI 바이너리 (pipeline.exe) + daemon
modals/app/          — Tauri GUI (file-pipeline-tauri.exe, Dashboard + 트레이, workspace exclude)
ui/                  — 정적 프론트엔드 (index.html, dashboard.css/js)
```

외부 재사용 모듈은 `C:\dev\claude_workspaces\_rust_module\`의 별도 workspace (form-agnostic 16종 — secrets/storage/notify/llm/signing/trivy/pdf-korean/checksum/messaging/kernel-ids, 각 api/impl 분리). `adapters`는 8개 모듈을, `shared`는 secrets 2개를 직접 참조.

MCP 서버는 별도 모달이 아니라 `shared/mcp_server.rs`에 통합되어 cli/app 양쪽에서 호출 (Phase 102 도구 25). `src/vendor/`는 Phase 64 트리거 #11/#12 onnxruntime 폐기 후 빈 디렉토리.
