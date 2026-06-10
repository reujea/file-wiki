---
updated: 2026-04-17T2
---

# 교훈 및 이슈 기록

Lint 결과 이력은 00_lint-archive.md 참조.

## 오류·반성

1. **DocType enum 하드코딩** → doc_types.toml 런타임으로 해결
2. **검증 기준 하드코딩** → VerificationThresholds 설정 가능으로 해결 (DB 등록률 20%→100%)
3. **stub 테스트만으로 완료 판단** → 실환경 claude -p 벤치마크 의무화
4. **rmcp API 사전 미확인** → 외부 크레이트 소스를 코드 작성 전에 읽기
5. **content: String → HashMap 시도** → 13곳 수정 필요. "추가한다, 바꾸지 않는다"로 해결
6. **sed 일괄 치환** → Metadata에 잘못된 필드 삽입. 자동 치환 후 수동 검토 필수
7. **Mutex.lock().unwrap()** → expect("mutex poisoned")로 전수 교체

8. **MyDocSearch 통합 검토** → "나사 하나 박으려고 공장을 사오는 격". 필요한 기능만 Qdrant 설정+~200줄로 해결.
9. **qdrant-client API 버전 차이** → Cargo.toml에 1.13이지만 실제 설치는 1.17. `Vector`, `Query` 구조가 deprecated → 새 API(`vector::Vector`, `query::Variant`, `VectorInput`) 확인 필수.
10. **pipeline.toml [verification.thresholds] 미반영** → `service.rs`에 `global_thresholds` 필드 추가로 해결. 설정이 코드에 전달되는 경로를 항상 검증.
11. **qdrant_local file:// URI** → Windows 백슬래시가 URI 규격에 위배. 바이너리 별도 실행 시 gRPC URL 사용으로 수정.
12. **CLAUDE_BIN 디렉토리 설정** → 환경변수가 디렉토리를 가리키면 `Command::new`가 실패. PATH에서 찾도록 `"claude"` 기본값 사용.
13. **구조 완전성 0% 문제** → LLM 프롬프트에 sections JSON 필드 누락 + content 파싱이 `===` 패턴만 인식. `##`/`###` 패턴 추가 + 프롬프트에 sections 필드 명시로 해결.

14. **cargo test --all hang (1시간+)** → bench_scale.rs의 bench_scale_5000(5000문서)과 bench_search_curve(1600문서)에 환경변수 가드 없이 매번 실행. cargo-nextest 도입으로 테스트별 프로세스 격리 + SLOW 경고 + 진행률 표시. 원인: `cargo test`는 바이너리 단위 실행이라 어느 테스트에서 멈췄는지 알 수 없음.
15. **외부 연동 설정이 환경변수 전용** → Dashboard 인증(DASHBOARD_USER/PASS), CORS(DASHBOARD_CORS_ORIGIN)가 환경변수만 지원해서 pipeline.toml과 불일치. DashboardConfig 구조체 추가 + env→config fallback 패턴으로 통일.
16. **spec/prd 수치 낡음** → architecture.md의 파일 수(58→62), 코드량(~9.8K→~11K), CLI(12→15), MCP(6→9), 설정(9→10) 등이 갱신 안 됨. 기능 완료 시 즉시 동기화 규칙(CLAUDE.md)이 있었지만 23건 일괄 구현 시 중간 동기화를 건너뜀.
17. **23건 일괄 구현의 완료 판단 부재** → 23건 중 17건만 완료되었는데 "완료"로 간주. 항목별 체크리스트 없이 진행하여 6건이 누락된 상태로 방치됨. 대규모 배치 구현 시 항목별 완료 체크 의무화.

18. **알림이 건별 전송만 지원** → watch 중 파일 10개 투입 시 알림 10개 폭탄. ProcessingSummary 모델 + 배치 요약 알림(30초 유휴 시 flush)으로 해결. 알림은 "현황 대시보드"처럼 요약해서 보여줘야 유용.
19. **config 파일 inbox 투입 시 보안 위험** → .env, .toml, .json 등 config 파일에 API키가 LLM에 전달됨. watcher.rs에 5단계 스킵 기준 추가(config/소스코드/바이너리/특정파일명). 소스코드(.rs/.py/.js 등 24종)도 전처리기 미지원이라 스킵 대상.
20. **Settings UI 없이 pipeline.toml 수동 편집** → 설정 항목 증가(10섹션 40+필드)로 사용자 편의성 저하. config_metadata() + GET/PUT /api/config + Dashboard Settings 탭으로 해결. 필드별 설명/타입/기본값/재시작필요 표시.
21. **service.rs에 summary 필드 추가 시 테스트 5곳 누락** → FileProcessingService 구조체에 필드 추가할 때마다 테스트의 모든 초기화 코드에도 반영 필요. Default derive 검토하거나, builder 패턴 도입 검토 필요.
22. **다이어그램 부재** → 아키텍처/플로우/시퀀스/경계/알림 다이어그램이 없어 구조 파악에 코드 읽기 필수. doc/architecture-diagrams.md에 5개 다이어그램 + 고도화 10개 정리. 다이어그램은 코드와 동기화 필요.

23. **Write 도구 한글 경로 문제** → `Write`로 `llm/mod.rs`를 재작성했으나 실제 디스크에 반영 안 됨. 한글 경로 인코딩 문제 추정. `Edit` 도구로 기존 파일 수정하는 방식이 안전. 새 파일은 Write, 기존 파일은 반드시 Edit.
24. **LLM 어댑터 API 불일치** → AnthropicApiAdapter 작성 시 `EnrichResult` 필드명, `reprocess_with_feedback` 파라미터 순서를 코드 읽기 없이 추측. 외부 크레이트뿐 아니라 **자체 core trait도 반드시 읽고 나서** 구현체를 작성해야 함 (교훈 #3의 확장).
25. **bench_scale_5000 타임아웃** → nextest terminate-after=3(360초)으로 설정했으나 환경 변동으로 120초*3=360초 내 완료 못함. period*terminate-after 계산을 정확히 이해하고, 5000문서는 여유 있게 terminate-after=5(600초)로 조정.
26. **prompts.rs에서 registry.types() 호출** → private 필드. 실제 public API는 `registry.all()`. adapters 계층에서 core의 public API만 사용해야 하는 헥사고날 원칙 재확인. core를 수정하지 않고 기존 public 메서드로 해결.

27. **Metadata 필드 추가 시 20+곳 수동 수정** → tier/last_accessed/access_count 3개 필드 추가 시 13파일 20곳의 Metadata 초기화 코드를 일일이 수정. `sed` 일괄 치환으로 해결했지만, Metadata에 Default derive 또는 builder 패턴이 있었다면 불필요한 작업. 교훈 #11 재확인.
28. **core 크레이트에서 tokio 사용 불가** → service.rs에 `broadcast::Sender<String>` 추가 시 core가 tokio에 의존하게 됨. 헥사고날 원칙 위반. `Option<Arc<dyn Fn(&str)>>` 콜백 패턴으로 해결 — core는 std만 의존.
29. **경쟁 분석 결과의 과다 항목** → 100개 요소 중 5개만 즉시 구현. 분석은 넓게, 구현은 좁게. "즉시/단기/중장기" 분류가 핵심.
30. **SKILL.md 도입 불필요 판정** → doc_types.toml이 이미 5차원 커스터마이징 제공. 새 프레임워크 도입보다 기존 추상화 확장이 ROI 높음.

31. **대용량 파일 truncate → 정보 99% 손실** → 1M 문서 실사용 시뮬레이션에서 발견. CHUNK_SIZE(40KB)로 잘라서 1GB 파일의 0.004%만 가공. ChunkedAgentAdapter(Decorator) + chunking.rs로 해결 — 분할→에이전트 위임→병합.
32. **작업 큐 없이 watch만** → 1M 문서 배치 시 프로세스 중단 시 처음부터 재시작. WorkQueue(.work-queue.json)로 상태 영속화 — 중단 후 이어서 처리, 변경/삭제 감지, 배치 분류(소형/대형).
33. **WorkQueue test_detect_modified 중복 push** → scan_and_plan에서 Modified를 1단계+2단계 두 번 plan에 추가. 2단계에서 Modified를 스킵하도록 수정. 상태 전이 로직은 한 곳에서만 plan에 추가해야 함.
34. **bench_scale_5000 타임아웃 반복** → nextest terminate-after 설정이 반영 안 되는 현상. `period × terminate-after`로 총 허용 시간이 계산되는데, 동시 테스트 부하로 환경 변동 큼. 5000문서 벤치는 별도 프로필로 분리 권장.
35. **190+ 기능 중 40개만 테스트** → 기능 인벤토리를 만들어 보니 테스트 커버리지 ~20%. 핵심 도메인(19건) + Actor 시나리오(6건) + WorkQueue(10건) 추가로 ~35%까지 향상. 포트 trait 구현체(어댑터)는 외부 서비스 의존으로 단위 테스트 어려움 — 통합 테스트로 커버.

36. **KG 기능이 MCP에만 있고 CLI/REST에 없음** → 기능 구현 후 모든 인터페이스에 노출했는지 체크리스트 필요. 이번에 CLI kg 커맨드 3개 + REST /api/kg/paths 추가. "구현 = core + 모든 인터페이스 노출"로 정의해야 함.
37. **CLI 16개 → 10개 통합** → watch/batch/serve/dashboard/purge/lint/topic-merge가 별도 커맨드였으나, 사용자는 `pipeline start` 하나만 실행. 솔루션은 사용자 행동 기준으로 커맨드를 설계해야 함. 개발자 편의가 아닌 사용자 편의.
38. **하드코딩 1536이 4곳에 있었음** → 임베딩 모델 교체 시 4곳 모두 수정해야 하는 위험. config에서 한 곳만 변경하면 전파되도록 수정. "값은 한 곳에서만 정의한다" 원칙.
39. **backfill-sparse 불필요 판단** → upsert()가 이미 sparse vector를 생성하는지 확인하지 않고 별도 커맨드를 만들었음. 기능 추가 전 기존 코드가 이미 처리하는지 확인 필수. 교훈 #3(외부 크레이트 읽기)의 자체 코드 버전.
40. **Tauri는 설정만, 빌드는 별도** → Tauri 앱은 `cargo tauri build`로 빌드하므로 기존 workspace `cargo build --all`에 포함하지 않음. 멀티 프레임워크 프로젝트에서 빌드 경계를 명확히.

41. **infrastructure lib.rs 추출** → main.rs에서 build_service/which_claude/config를 공유하려면 lib.rs가 필요. bin-only 크레이트는 외부에서 참조 불가. lib+bin 패턴으로 해결.
42. **Tauri Cargo.toml 패키지명 불일치** → `package = "file-pipeline"` + `lib = "file_pipeline_infra"` 조합에서, 의존하는 크레이트는 `package = "file-pipeline"`으로 명시해야 함. Cargo가 lib name이 아닌 package name으로 resolve.
43. **CLI 전용 터미널 interaction** → lib.rs의 build_service는 항상 Stub(비대화형). CLI에서만 TerminalResolution/TerminalSensitive를 사용해야 하므로, `build_service_cli` 래퍼로 포트 교체. 헥사고날 패턴의 장점 — 포트만 바꾸면 됨.

44. **cargo clean 후 빌드 불가** → 빌드 캐시가 있을 때만 성공, clean 후 Git Bash `/usr/bin/link`가 MSVC `link.exe`를 가로채는 문제 노출. 이전 빌드는 캐시 덕분에 링크 단계를 건너뛰었을 뿐. cargo clean은 빌드 환경 문제를 드러내는 파괴적 작업.
45. **Windows SDK kernel32.lib 미설치** → VS 2017 BuildTools에 C++ 도구는 있으나 Windows SDK lib가 없었음. vcvarsall.bat으로 link.exe는 찾지만 kernel32.lib를 못 찾음. Windows Kits 10의 Lib 디렉토리를 LIB 환경변수로 직접 지정하여 해결.
46. **winget install VS BuildTools의 함정** → winget은 인스톨러를 다운로드/실행하지만, 실제 워크로드(VCTools+SDK) 설치는 보장하지 않음. `--override` 플래그로 컴포넌트를 명시해도 VS Installer가 별도 실행될 수 있음.
47. **GUI+CLI 단일 바이너리 통합** → Tauri main.rs에서 clap 파싱 후 인자 분기. CLI 커맨드는 tokio 런타임으로 직접 실행, GUI는 Tauri 앱으로 실행. clap 의존성을 Tauri Cargo.toml에도 추가해야 함 (workspace 밖이라 자동 상속 안 됨).
48. **Playwright MCP로 UI 버그 발견** → `stat-hub` 요소가 HTML에 없는데 JS에서 참조하여 콘솔 에러 발생. 수동 테스트로는 발견하기 어려운 종류의 버그. 브라우저 자동화 테스트의 가치 증명.

49. **모달 분리 시 Cargo 의존성 누락** → infrastructure를 shared + cli + mcp로 분리했을 때, cli에 `dirs`/`rmcp`, app에 `clap` 의존성이 누락. 기존 크레이트가 workspace 의존성을 암묵적으로 상속받고 있었으나, 분리 후에는 각 Cargo.toml에 명시적으로 추가해야 함.
50. **모달 분리 = 책임 순수화** → APP(GUI)에서 CLI 분기 코드를 제거하자 main.rs가 100줄→35줄로 간결해짐. "하나의 모달은 하나의 인터페이스"가 유지보수에 유리.
51. **rmcp API 탐색** → `serve` 메서드가 trait 메서드인 줄 알았으나, 실제로는 `rmcp::service::serve_server()` 자유 함수. 외부 크레이트는 소스를 먼저 읽는다는 교훈(#3) 재확인.
52. **디렉토리 재구조화는 한 번에** → 파일 이동 → import 수정 → Cargo.toml 경로 → tauri.conf.json 경로 → 빌드 순서로. 중간에 빌드하면 캐시 오염으로 혼란 가중.

53. **windows_subsystem="windows" vs 콘솔 출력** → GUI 바이너리에서 초기화 상태를 보여줘야 하는데 콘솔이 없음. 해결: `windows_subsystem` 제거 + `AllocConsole` → 초기화 후 `FreeConsole`로 숨기기. 사용자가 시작 상태를 확인하고 GUI가 뜨면 콘솔 사라짐.
54. **auto-init이 cwd에 생성** → `std::fs::write("pipeline.toml")`은 cwd 기준. 바이너리가 있는 디렉토리에 생성하려면 `std::env::current_exe().parent()` 기반 `exe_dir()` 함수 필요.
55. **find_and_load_config vs auto_init 타이밍** → execute() 안에서 auto-init을 하면, main.rs에서 이미 config를 기본값으로 로드한 뒤라 파일이 안 생김. auto_init()은 main() 최초에 실행해야 함.
56. **Tauri CloseRequested 가로채기** → `api.prevent_close()` + `window.hide()`로 창 닫기를 숨기기로 처리. 이 때문에 `taskkill`(graceful, WM_CLOSE)도 창을 숨기기만 함. 트레이 "종료"가 유일한 정상 종료 경로. 의도된 동작이지만 사용자에게 안내 필요.
57. **backend 기본값 qdrant → sqlite** → Qdrant 없는 환경에서 첫 실행 시 7초 타임아웃 + 에러. 기본값을 sqlite로 바꾸고 주석으로 Qdrant 활성화 방법 안내. 첫 사용자 경험이 에러 없이 깔끔해짐.
58. **pipeline.toml 템플릿에 주석** → Serde toml 직렬화는 주석 미지원. const 문자열 템플릿으로 교체하여 사용자가 읽을 수 있는 주석 포함.

59. **Tauri 2.0 invoke API 변경** → `window.__TAURI__`가 아니라 `window.__TAURI_INTERNALS__`로 변경됨. 모든 Tauri command 호출이 실패하고 빈 값만 반환되고 있었는데, 에러가 조용히 무시되어 발견이 늦었음. 외부 프레임워크 API는 버전별로 반드시 확인.
60. **GUI 중복 실행** → 트레이 앱이 여러 개 뜨는 문제. Named Mutex(`Global\\FilePipelineSingleInstance`)로 OS 레벨 싱글 인스턴스 보장. CLI 모드는 중복 허용.
61. **Lint 관리 기능 부재** → 탐지만 하고 조치가 없으면 사용자에게 무의미. Dashboard에 삭제/백링크 보강 버튼 추가. 자동 정책(auto-delete 등)은 위험하므로 수동만.

62. **Settings UI 전면 재구성** → 설정 항목이 많아지면서 스크롤 피로. VSCode 스타일(좌측 네비 + 검색 + 섹션별 표시)으로 전환. 좌측 클릭 시 해당 섹션만 표시, 검색 시 전체 필터링.
63. **select:value=label 패턴** → config_metadata의 field_type에 `select:0.01=설명|0.03=설명` 형식으로 옵션 정의. JS에서 파싱하여 `<select>` 렌더링. 범용적으로 재사용 가능.
64. **inbox 다중 경로** → PathsConfig에 `extra_inboxes: Vec<String>` 추가. FileWatcher에 `with_extra_inboxes()` 체인. 모든 inbox를 동시 감시.

65. **Settings 11메뉴→5그룹 통합** → config_metadata(백엔드)는 그대로 두고 JS에서 그룹 매핑. 백엔드 변경 없이 프론트엔드만으로 메뉴 구조 변경 가능.
66. **Credentials 독립 탭→Settings 내 통합** → AI엔진 그룹 상단에 크레덴셜 인라인 표시 + 모달 폼. 독립 탭이 없어져 탭 수 감소.
67. **GUI에서 AllocConsole 제거** → CMD 창이 번쩍 떴다 사라지는 UX 문제. 로그 파일에만 기록하고 콘솔은 CLI 모드에서만 사용.
68. **5초 자동 갱신** → setInterval로 stats+queue+문서목록 주기적 polling. inbox에 파일 투입 시 대시보드 실시간 반영.

69. **SqliteVec 인메모리→영속화** → JSON 파일(.sqlite-vec.json)로 persist. upsert/link/delete 시마다 전체 스냅샷 저장. 재시작 후 stats가 유지됨. 대규모에서는 성능 문제 가능하지만 소규모에서 충분.
70. **가공 45초 대기** → Claude CLI 호출이 느려서 15초 안에 가공 미완료. LLM 가공은 파일당 10~15초 소요. 테스트 시 충분한 대기 필요.
71. **Playwright 세션 만료** → 장시간 사용 시 브라우저가 종료됨. MCP 재시작 필요. E2E 테스트는 세션 초반에 집중 실행.

72. **index.html과 dashboard.html 이중 관리** → dashboard.html만 수정하고 index.html(Tauri 진입점)을 동기화하지 않아 앱이 동작하지 않음. 원인: 두 파일이 동일 내용이지만 별도 관리. 해결: cp로 동기화, 향후 단일 파일로 통일 필요.
73. **SensitiveConfig alias 충돌** → 기존 TOML에 extra_keywords/custom_keywords/extra_extensions가 동시 존재하는데 serde alias로 같은 필드에 매핑 시도 → 파싱 실패. 해결: 하위 호환 필드를 skip_serializing으로 유지 + merged_*() 메서드로 신구 필드 병합.
74. **Tauri frontendDist 경로** → /static/ 절대경로가 Tauri 번들에서 작동하지 않음. 상대경로로 수정 필요. 빌드 환경과 개발 환경에서 경로 해석이 다를 수 있으므로 상대경로가 안전.

75. **JS 문법 에러가 조용히 앱 전체를 죽임** → `.map(c => { ... })` 뒤에 `)` 중복으로 JS 파싱 에러 발생. `vm` 객체가 정의되지 않아 모든 탭 클릭이 불가. 브라우저 콘솔 에러가 보이지 않아 발견이 늦었음. 해결: `node -c dashboard.js`로 문법 검증 루틴 추가. JS 수정 후 반드시 `node -c` 확인.
76. **FreeConsole()이 Tauri WebView2를 죽임** → GUI 모드에서 콘솔을 숨기려고 `FreeConsole()` 호출 시 Tauri 빌더가 크래시. 원인: WebView2가 콘솔 핸들에 의존. 해결: `#![windows_subsystem = "windows"]` 크레이트 속성으로 처음부터 콘솔 없이 시작. CLI 모드만 `AttachConsole`으로 콘솔 생성.
77. **adapters → shared 순환 의존** → watcher(adapters)에서 `build_llm_from_credential`(shared) 호출 시도 → 컴파일 실패. shared가 이미 adapters에 의존하므로 역참조 불가. 해결: watcher에 `credential_llms: HashMap<String, Arc<dyn LLMPort>>`를 미리 빌드해서 주입. 호출 측(app/cli)에서 빌드, watcher는 조회만.
78. **dashboard.html 삭제 후 단일 파일 통일** → index.html과 dashboard.html 이중 관리(교훈 #72) 근본 해결. dashboard.html 삭제, index.html만 유지. CLAUDE.md에 규칙 명시.
79. **포트 trait에 기본 구현으로 하위 호환** → `PreprocessPort::preprocess_with_config`, `EmbeddingPort::embed_with_model`, `StoragePort::compress_with_level` 모두 기본 구현(오버라이드 무시)을 제공하여, 기존 어댑터가 깨지지 않으면서 새 기능을 점진적으로 적용. 포트 확장 시 "기본 구현 + 필요한 어댑터만 오버라이드" 패턴이 유효.

80. **.compile-state.json 잔존 → 재처리 스킵** → DB 초기화 후에도 .compile-state.json 파일이 남아있어 파일이 이미 처리된 것으로 판정, 재처리가 스킵됨. DB 초기화 시 compile-state 파일도 함께 삭제해야 함. 상태 파일은 DB와 동기화되어야 한다.
81. **dashboard.port==0 검증이 config 저장 실패 유발** → Tauri에서는 포트 바인딩이 불필요한데, port==0 검증이 config 저장을 거부. Tauri 환경에서 불필요한 검증을 제거. 플랫폼 컨텍스트에 맞게 검증 규칙을 분기해야 함.
82. **배경 서비스 로그가 GUI에서 확인 불가** → tracing만 사용하여 로그 파일에만 기록. GUI 대시보드에서 확인할 수 없음. write_log 함수를 병행하여 GUI에서도 서비스 상태 확인 가능하도록 수정.

83. **SqliteVecAdapter::new()가 테스트 바이너리 디렉토리에 DB 공유** → 여러 테스트 파일이 동시 실행되면 이전 테스트 데이터가 누적됨. `total_documents` assertion이 예측 불가능하게 실패. 해결: `with_path(temp.join(".sqlite-vec.json"))`로 테스트별 격리. **테스트 격리는 DB 경로까지 포함해야 한다.**
84. **semantic_dup_threshold 0.03이 테스트에서 대부분 문서를 중복 처리** → HashEmbedder는 키워드 해시 기반이라 모든 문서 간 cosine similarity > 0.05. distance < 0.03이면 거의 모든 문서가 중복으로 스킵됨. 테스트에서는 0.0001로 설정해야 12건 모두 색인. **테스트용 threshold는 운영 값과 다르게 설정해야 한다. 단위 테스트가 운영 설정에 암묵적으로 의존하면 안 된다.**
85. **한글 파일명에서 바이트 인덱스 슬라이싱 panic** → `&filename[i..i+4]`에서 한글 바이트 경계를 넘김. `chars()` 벡터로 변환 후 인덱싱해야 한다. **Rust에서 한글 문자열은 반드시 char 단위로 접근.**
86. **pipeline.toml dim=1536 vs Claude CLI 128축 불일치** → ClaudeEmbeddingAdapter가 128축 점수를 생성하고 나머지 1408차원을 0으로 패딩. 검색 정확도에 직접 영향. dim=128로 설정해야 벡터가 의미축과 1:1 대응. **설정값이 어댑터 내부 고정값과 맞는지 확인. 패딩은 정확도를 희석한다.**
87. **검색 테스트가 `!results.is_empty()` 수준이었음** → "결과가 있는지" 만 확인하면 검색 품질 회귀를 감지할 수 없다. MRR, P@K, 랭킹 순서, 점수 분포, 필터 정합성까지 검증해야 실제 사용자 경험을 보장. **검색 테스트는 존재 확인이 아니라 품질 측정이어야 한다.**
88. **DEFAULT_CONFIG_TEMPLATE에 파이프라인 정의가 없었음** → 첫 실행 시 `pipeline.toml`이 생성되지만 [[pipelines]] 없이 Default만 코드에서 자동생성. 사용자가 파이프라인 구조를 이해할 수 없음. 템플릿에 표준 파이프라인 4개를 포함해야 "inbox에 넣으면 바로 동작"이 보장됨. **기본 설정은 사용자가 아무것도 안 건드려도 즉시 동작해야 한다.**
89. **prd/features에 완료된 항목 3건이 정리 안 됨** → vec-file-persistence, bm25-sparse-search, todo-lifecycle이 Phase 완료 후에도 pending 상태로 남아있었음. 로드맵과 features 사이 정합성 체크를 Phase 완료 시 반드시 수행. **Phase 완료 = features 상태 갱신 + roadmap 체크 + spec 동기화.**

90. **core 크레이트에서 #[tokio::test] 사용 불가** → 교훈 #28(core는 std만 의존)에 의해 tokio가 없어 async 테스트 불가. 해결: `[dev-dependencies]`에 tokio 추가. dev-dependency는 테스트 전용이므로 라이브러리 의존성에 영향 없음. **core의 "std만 의존" 원칙은 `[dependencies]`에만 적용. `[dev-dependencies]`는 테스트 전용이므로 예외.**
91. **service.rs (871 LOC) 테스트 전무** → 파이프라인 엔진 전체가 무검증 상태. Stub 포트 9개를 테스트 모듈 내에 구현하여 9개 테스트 추가(정상 플로우, 중복, 증분, Fragment, 민감, 검증, 재시도, 해시, purge). **핵심 서비스는 Stub 포트로 외부 의존 없이 테스트 가능. 테스트 작성을 미루면 부채가 기하급수적으로 증가.**
92. **spec 수치와 코드 불일치 4건 발견** → 테스트 152→110, Tauri commands 28→30, CLI 9→11, MCP 9→11. 기능 추가 후 spec 갱신 누락이 원인. **기능 추가 시 CLI/MCP/Tauri 커맨드 수를 spec에 즉시 반영. 주기적 검증(grep으로 실측)으로 drift 방지.**
93. **prd/features에 완료 항목 11건 방치** → 8건의 features + phase6 + 보고서 2건이 완료 후에도 삭제 안 됨. roadmap.md에 완료 기록이 있으므로 정보 손실 없이 삭제 가능. **Phase 완료 시 features 파일 삭제 + roadmap 반영을 하나의 원자적 작업으로 수행.**

94. **프롬프트 확장이 속도를 늦추지 않는다** → 독립 claude -p 테스트에서는 신규 프롬프트가 48% 느렸으나, 실제 파이프라인에서는 오히려 6% 빠름(41.2→38.6초/파일). 원인: 신규 프롬프트의 가공 품질이 높아 검증 1-Pass 통과(구조 100%, ROUGE 65.7%), 기존은 2-Pass 재가공 발생. **프롬프트 비용은 LLM 호출 1회가 아니라 전체 파이프라인(재시도 포함)으로 측정해야 한다.**
95. **doc_types.toml의 patterns/prompt는 LLM 힌트일 뿐** → 제거 후에도 유형 판단 정확도 100%. LLM은 파일명+내용만으로 자율 판단 가능. sections만 남기면 검증 스키마로서 충분. **설정은 "LLM에 뭘 시킬지"가 아니라 "결과를 어떻게 검증할지"에 집중.**
96. **DocTypeDef.sensitive 필드가 죽은 코드** → doc_types.toml에 `sensitive=true`가 있었지만 SensitivityDetector는 독립적으로 파일명+확장자+경로 기반 판별. 코드에서 한 번도 참조되지 않는 필드 발견. **신규 필드 추가 시 실제 사용 경로를 grep으로 확인.**
97. **프롬프트 중복은 품질 drift를 유발** → prompts.rs와 claude_adapter.rs에 동일 프롬프트가 중복 존재. 한쪽만 수정 시 불일치 발생. 위임 패턴(`prompts::build_classify_prompt` 호출)으로 단일 소스화. **프롬프트도 코드처럼 DRY 원칙 적용.**

55. **extui 커스텀 프로토콜 IPC 실패** → Tauri WebView에서 `file://`이나 커스텀 프로토콜(`extui://`)로 navigate하면 `__TAURI_INTERNALS__`가 주입되지 않아 invoke 전부 실패. 해결: UI는 빌드 시 임베드하고 외부 UI 모드 제거.

56. **StubSensitiveNotification이 None 반환** → 민감 파일 감지되어도 `notify_and_collect()`가 None을 반환하여 파일 이동 안 됨. 해결: Some(기본 Metadata) 반환으로 변경. Stub은 "아무것도 안 함"이 아니라 "기본 동작"을 해야 할 때가 있음.

57. **batch_process에 should_skip 누락** → scan_and_plan은 inbox 전체 파일을 큐에 등록하는데, 처리 루프에서 should_skip 필터가 없어서 .env 파일도 가공됨. watch 모드에는 있었지만 batch에는 없었음. 동일 필터를 양쪽에 적용해야 함.

58. **parse_response 폴백 논쟁** → JSON 파싱 실패 시 폴백 응답을 생성하면 잘못된 데이터가 색인됨. 사용자 판단: 가공 실패로 처리하는 것이 낫다. max_retry 재시도 후 quarantine.

59. **교차참조 LLM 호출이 전체 시간의 50%** → 문서 8건 배치에서 교차참조가 30~60초/건. pgvector SQL 패턴 차용하여 키워드/임베딩 기반 자동 링크로 전환. LLM 호출 0건, <1ms.

60. **pipeline.toml에 정의된 파이프라인이 코드 default보다 우선** → 코드에서 Default 파이프라인에 Preprocess 스텝을 추가했지만, pipeline.toml의 파이프라인에는 Preprocess가 없어서 xlsx가 전처리되지 않았음. 설정 파일과 코드 기본값의 우선순위를 항상 확인.

61. **전처리 실패 시 "직접 읽기 시도" 폴백이 바이너리 파일에서 무의미** → 전처리 실패 → read_to_string 시도 → 바이너리라 실패 → LLM에 파일 경로만 전달 → 의미 없는 가공. 텍스트 직접 읽기 성공하면 폴백, 실패하면 에러로 처리.

62. **파이프라인 배열 불필요** → 고정 17단계 플로우에서 패턴별 분기가 무의미. [[pipelines]] 4개 → [pipelines] 1개로 단순화. match_pipeline 로직 삭제. 설계 초기부터 "정말 분기가 필요한가?" 질문했어야 함.

63. **호스트 도구 감지 비용** → HostToolDetector::detect()가 subprocess 4개를 spawn하여 ~1초 소요. CompositePreprocessor::new()에서 매번 호출하면 preprocess_with_config()마다 반복됨. 생성자에서 1회만 감지하여 캐시.

64. **프롬프트 외부화는 OnceLock으로 충분** → RwLock 없이 OnceLock(프로세스 수명 캐시)으로 구현. 프롬프트 변경 시 재시작 필요하지만, prompts.toml이 없으면 내장 기본값이 작동하므로 안전. 향후 핫 리로드가 필요하면 RwLock으로 교체.

65. **DOCX는 ZIP + XML** → docx 파일은 ZIP 아카이브이므로 `zip` 크레이트만으로 텍스트 추출 가능. word/document.xml에서 `<w:t>` 태그만 파싱하면 충분. 전용 docx 크레이트 불필요.

66. **StubSensitiveNotification Some 반환 변경 후 테스트 3건 실패** → 교훈 #56에서 None→Some으로 변경했는데, 기존 테스트(scenarios, e2e, actor)가 "민감 파일은 DB에 없음"을 기대. stub 동작 변경 시 관련 테스트를 전수 검색해야 함.

67. **embed_batch 병렬화에 Semaphore 필수** → Claude CLI는 subprocess를 spawn하므로 무제한 병렬 시 프로세스 폭탄. Semaphore(4)로 동시 실행 수 제한. 순차→병렬 전환 시 리소스 제한을 항상 동반.

68. **ONNX 토크나이저는 tokenizers 크레이트** → whitespace 분할은 서브워드 모델(BPE/WordPiece)과 완전히 다른 입력을 생성. BGE-M3는 XLMRoberta 토크나이저를 사용하므로 반드시 tokenizer.json과 함께 사용. 간이 토크나이저는 테스트용에만.

69. **ONNX token_type_ids 필요** → BERT 계열 모델(BGE-M3 포함)은 input_ids, attention_mask 외에 token_type_ids(전부 0)도 입력으로 받음. 누락 시 추론 실패.

70. **프롬프트 핫 리로드는 RwLock이 적절** → OnceLock은 한 번 초기화 후 변경 불가. 런타임 갱신이 필요하면 RwLock<Option<T>>으로. 읽기 빈도가 높고 쓰기가 드물면 경합 최소.

71. **ort load-dynamic + Windows = DLL 초기화 크래시** → ort 2.0-rc의 `load-dynamic` feature는 ORT_DYLIB_PATH 환경변수로 DLL을 로드하는데, Python onnxruntime 패키지의 DLL과 호환성 문제 발생(STATUS_ACCESS_VIOLATION). 해결: Microsoft 공식 ONNX Runtime 바이너리 사용 또는 `load-dynamic` 대신 정적 링크.

72. **ort 2.0 API 변경** → Session::builder()가 ort::Session이 아니라 ort::session::Session, GraphOptimizationLevel이 ort::session::builder::GraphOptimizationLevel, try_extract_tensor가 (&Shape, &[T]) 반환(ndarray 아님), Session::run이 &mut self. 교훈 #4(외부 크레이트 소스 먼저 읽기) 재확인.

73. **ort load-dynamic은 Windows에서 근본적으로 불안정** → set_var, init_from, PATH 추가 등 4가지 접근 모두 STATUS_ACCESS_VIOLATION. ort 2.0-rc.12의 load-dynamic feature가 Windows DLL 초기화에서 크래시. **rc 버전은 런타임 안정성을 보장하지 않는다**. download-binaries는 MSVC STL 링크 에러.

74. **블로커 우회: Python subprocess** → Rust ort가 동작하지 않자 Python onnxruntime으로 MRR 벤치마크 실행. MRR 0.975 달성. **목표 달성이 특정 도구에 의존해서는 안 된다**. 대안 경로를 항상 준비.

75. **인터페이스만 준비하는 것과 실구현의 차이** → ColBERT, Neo4j, 모바일 빌드는 포트 trait + 설정만 추가. 실제 어댑터는 없음. **사용자에게 "인터페이스 준비"임을 명시**해야 혼란 방지.

76. **keyring 크레이트는 크로스 플랫폼** → Windows Credential Manager, macOS Keychain, Linux Secret Service를 단일 API로 추상화. 별도 OS별 구현 불필요. 3개 함수(store/get/delete)로 충분.

77. **persist()가 진짜 병목** → SqliteVecAdapter의 upsert/link마다 전체 JSON 직렬화+디스크 쓰기. 100문서×5회=500회 persist. batch_begin/end로 1회로 줄이자 6.3→13.3 docs/s (+111%). **매 변경마다 persist하지 말고, 배치 경계에서 flush**.

78. **instant-distance HNSW는 build()마다 새 인덱스** → 500문서 이상에서만 HNSW 활성화하는 분기가 있지만, 현재 매 search_similar마다 HNSW를 새로 빌드. 문서 추가 시 한 번만 빌드하고 캐시해야 함. 현재는 brute-force 분기(<500)로 우회.

79. **교차참조 O(N²) 오해** → 실제로는 O(N×k) (k=top_k=3). 병목은 search_similar의 O(N) 선형 스캔 + persist I/O. HNSW로 O(log N)으로 줄이고, batch persist로 I/O 제거하면 100문서에서 기준선 달성.

80. **벤치마크 프로세스 lock** → cargo test로 벤치마크를 여러 번 실행하면 이전 프로세스가 exe 파일을 잠가서 LNK1104 에러. taskkill + rm으로 해결. 벤치마크는 반드시 **단일 프로세스로 순차 실행**.

81. **교차참조를 동기→비동기 큐로 분리** → 파이프라인 처리량이 교차참조에 종속되지 않음. 가공 후 큐에 넣고, 정해진 간격(30초)마다 배치로 flush. 처리량과 교차참조 품질을 독립적으로 조절 가능.

82. **중복 큐 방지는 doc_id 비교로** → 같은 파일이 여러 번 투입되면 큐에 중복 항목이 쌓임. `queue.iter().any(|q| q.doc_id == item.doc_id)`로 간단히 방지. SHA-256 해시가 doc_id이므로 정확한 중복 판별.

83. **65분 오기재** → handsoff 보고서의 추정치(65분)를 실측치(422초=7분)로 착각하고 여러 문서에 반영. **외부 보고서 수치는 실측으로 검증 후 인용**해야 함. 정정 노트를 원문에 추가하는 것이 최선.

84. **Neo4j HTTP API = Cypher + /tx/commit** → Bolt 프로토콜 대신 HTTP Transactional API를 사용하면 별도 드라이버 없이 reqwest로 구현 가능. 데스크톱 앱에서 Neo4j를 선택적으로 사용할 때 적합.

---

## 2026-04-16T2 세션 반성문

### 잘한 것

1. **ort 크래시 원인 체계적 추적** — ORT_DYLIB_PATH 설정, init_from 호출, download-binaries 전환, DLL PATH 배치까지 4가지 접근을 시도하고 각각의 실패 원인을 기록. ort 2.0-rc.12의 load-dynamic이 Windows에서 근본적으로 불안정하다는 결론에 도달.
2. **Python 폴백으로 MRR 실측** — Rust ort가 동작하지 않자 Python onnxruntime으로 대안 실행. MRR@5 = 0.975로 BGE-M3의 실제 가치를 수치로 입증. "블로커가 있으면 우회"가 아니라 "블로커를 기록하고 목표는 달성".
3. **10항목 전체 커버** — High부터 Low까지 모든 항목을 인터페이스+설정+테스트 수준으로 구현. 완전 구현이 불가능한 항목(ort DLL, Neo4j 어댑터, 모바일 실빌드)은 인터페이스와 설정만 준비하여 향후 연결 가능하게.
4. **credential_store 크로스 플랫폼** — keyring 크레이트로 Windows/macOS/Linux 모두 지원하는 시크릿 저장소를 한 모듈로 해결. Tauri 커맨드까지 연결.

### 반성할 것

1. **ort load-dynamic 삽질 40분** — set_var → init_from → download-binaries → PATH 추가 순서로 4번 시도하며 매번 빌드+테스트. 첫 번째 실패 시 ort GitHub issues를 먼저 검색했으면 "rc 버전 Windows 크래시"가 알려진 문제임을 확인할 수 있었을 것. **외부 크레이트 런타임 문제 시 issues 먼저 검색**.
2. **download-binaries MSVC 링크 에러 미예상** — C++ STL 심볼 불일치. VS BuildTools 설정이 불완전한 환경에서 정적 링크는 실패할 수 있다는 교훈(#44~#46)을 다시 경험. **정적 링크는 빌드 환경 의존성이 높다**.
3. **search_accuracy 간헐적 실패 미해결** — HashEmbedder의 비결정성으로 10회 중 1~2회 실패. 근본적으로 HashEmbedder의 해시 충돌이 원인. BGE-M3 전환 시 해소될 문제지만, 현재 테스트의 안정성을 보장하지 못함. **간헐적 실패 테스트에 retry 또는 허용 오차 추가 필요**.
4. **#7~#10 인터페이스만 구현** — ColBERT, Qdrant auto_start, 모바일, Neo4j 모두 "포트 trait + 설정"만 추가하고 실제 어댑터 구현은 없음. 정직하게 "인터페이스 준비"로 기록했지만, 실구현과 인터페이스 준비의 차이를 사용자에게 먼저 설명했어야 함.
5. **Playwright 세션 만료 대응 부재** — 이전 세션에서 검증 완료했지만, 새 세션에서 Settings 청킹을 검증하지 못함. Playwright MCP 세션 수명 관리가 없음.
6. **spec 수치 갱신 반복 지연** — 교훈 #16의 4번째 위반. 10항목 구현 후 한꺼번에 갱신.

### 다음 세션에 적용할 규칙

1. 외부 크레이트 런타임 문제 시 **GitHub issues 먼저 검색**
2. 인터페이스만 준비하는 항목은 사용자에게 **사전 고지** ("포트만 추가, 어댑터는 미구현")
3. 간헐적 실패 테스트에 **retry 매크로 또는 허용 오차** 적용
4. 3건 이상 연속 구현 시 **중간 spec 갱신** (기존 5건 → 3건으로 강화)

## 2026-04-16 세션 반성문

### 잘한 것

1. **병렬 작업 진행** — features 삭제, config 추가, 프롬프트 외부화, DOCX/XLSX, 배치 병렬화를 순차적이지만 빠르게 처리. 각 단계마다 빌드+테스트로 회귀 즉시 감지.
2. **기존 누락 발견 후 즉시 수정** — domain-map의 "누락" 항목 중 5/6이 이미 해결되어 있음을 확인하고, 유일한 실제 누락(chunking)만 처리. 불필요한 작업 회피.
3. **테스트 수정 시 원인 파악** — FileProcessingService 필드 누락(9파일), 민감 파일 테스트 assertion 실패(3건), MCP 검색 정확도 실패(1건) 각각 근본 원인을 파악하고 수정. 맹목적 수정 안 함.
4. **Playwright 즉시 검증** — UI 변경 후 HTTP 서버를 띄워 Playwright MCP로 실제 동작 확인. 프롬프트 모달, ONNX 경로 showIf 조건부 필드 모두 검증.
5. **ONNX 실사용 준비 완료** — 스켈레톤이었던 onnx_embed.rs를 tokenizers 연동, from_dir/auto_detect, attention mask mean pooling으로 실사용 가능하게 재작성. 모델 파일(2.2GB)도 준비.

### 반성할 것

1. **OnceLock → RwLock 2단계 구현** — 프롬프트 외부화를 OnceLock으로 먼저 구현한 뒤, 핫 리로드 요청을 받고 RwLock으로 재작성. 처음부터 요구사항을 확인했으면 1회 작업으로 충분했을 것. "향후 교체 가능" 코멘트를 남기는 대신 사용자에게 물어봐야 했다.
2. **ort load-dynamic DLL 문제 미예상** — ONNX feature 빌드는 성공했지만 런타임 DLL 초기화에서 STATUS_ACCESS_VIOLATION 크래시. Python onnxruntime의 DLL이 ort 2.0-rc와 호환되지 않는 문제를 사전에 조사하지 않음. 외부 크레이트 + 런타임 의존성은 **빌드 성공 ≠ 런타임 성공**임을 재확인.
3. **BGE-M3 MRR 벤치마크 미완** — 모델 다운로드와 코드 준비는 완료했지만, DLL 호환성 문제로 실제 MRR 수치를 측정하지 못함. ONNX Runtime 공식 바이너리를 vendor/에 미리 다운로드했으면 해결 가능했을 것.
4. **calamine 0.26 vs 최신 0.34** — workspace에 calamine 0.26을 추가했지만 최신은 0.34. API 호환성은 확인했으나, 최신 버전을 사용하지 않은 이유를 기록하지 않음.
5. **tokenizers 0.21 vs 최신 0.22** — 같은 이유. workspace에 0.21을 추가했는데 최신은 0.22. 의존성 버전 선택 근거를 남겨야 함.
6. **spec 수치 갱신을 마지막에 몰아서** — 교훈 #16(중간 동기화 필수)의 반복. 이번에도 Phase 39~40 전체를 구현한 뒤 spec을 한꺼번에 갱신.

### 다음 세션에 적용할 규칙

1. 요구사항 불확실 시 사용자에게 먼저 확인 (핫 리로드 필요 여부 등)
2. 외부 크레이트 + 런타임 바이너리 의존 시 빌드 + **런타임 테스트**까지 검증
3. 의존성 버전 선택 시 근거를 Cargo.toml 주석에 기록
4. 5건 이상 연속 구현 시 중간 동기화 (반복 위반)

---

## 2026-04-14~15 세션 반성문

### 잘한 것

1. **헥사고날 원칙 유지** — 22건 기능 추가에도 core→adapters 참조 0건. 새 포트(RerankerPort, RemoteStoragePort) 추가 시 trait 먼저, 어댑터 나중 순서를 지켰다.
2. **테스트 먼저** — Phase 1~2에서 테스트 36건을 먼저 추가한 뒤 기능 구현. 안전망이 후속 변경(process_file 통합, 교차참조 전환)에서 회귀를 방지했다.
3. **E2E 실사용 테스트** — 실제 문서 투입으로 민감 파일 미감지, 배치 스킵 누락, StubSensitiveNotification None 반환 3건의 실제 버그를 발견+수정. 단위 테스트만으로는 잡지 못했을 것.
4. **pgvector SQL 패턴 차용** — 교차참조를 LLM 호출에서 키워드/임베딩 기반으로 전환하여 30~60초 → <1ms. 기존 솔루션의 패턴을 자체 구현에 적용하는 판단이 적절했다.
5. **피드백 SDK 아이디어를 실증** — 아이디어만 있던 피드백 시스템을 실제 동작하는 코드로 검증. ideabank에 실험 결과를 피드백하여 순환 완성.

### 반성할 것

1. **에이전트 위임 시 정합성 검증 부재** — 에이전트에게 UI 작업을 위임하면서 기존 변수명(vals.embedding → embed_gen, vals.storage → save_compress) 변경 사항을 전달하지 않아 **Pipeline 탭이 안 보이는** 런타임 에러 발생. 에이전트 결과를 받은 뒤 **변수명/참조 일관성을 수동 검증**해야 했다.
2. **번호 체계 관리 실패** — lesson-learned의 번호가 54에서 98로 뛰었다. 에이전트에게 "마지막 번호 확인 후 이어서"를 명시하지 않았다. 에이전트에 위임할 때 **기존 데이터의 컨텍스트를 정확히 전달**해야 한다.
3. **spec 갱신을 마지막에 몰아서** — CLAUDE.md에 "중간 동기화 필수"라고 적어놓고 22건 구현 후에야 spec을 갱신했다. 교훈 #47의 반복. 5건 이상 연속 구현 시 **반드시 중간 동기화**.
4. **extui 프로토콜 삽질** — Tauri IPC가 커스텀 프로토콜에서 동작하지 않는다는 것을 **사전 조사 없이 구현**했다. 30분 낭비 후 제거. 외부 프레임워크 기능은 **구현 전에 제약사항을 확인**해야 한다 (교훈 #4의 변형).
5. **릴리스 빌드 3분 × 10회 = 30분 낭비** — UI 변경마다 릴리스 빌드를 했다. `cargo tauri dev`로 개발 서버를 띄웠으면 즉시 반영되었을 것. 개발 중에는 **dev 모드를 사용하고 릴리스는 최종 확인 시만**.
6. **동시에 너무 많은 기능 구현** — 한 세션에 22건은 과도했다. 각 기능의 검증이 얕아져서 Pipeline 탭 깨짐, 민감 파일 미처리 등의 버그가 E2E에서야 발견됨. **한 세션에 5~7건이 적정**, 나머지는 다음 세션으로.
7. **process_file 제거 시 영향 분석 부족** — process_file을 파이프라인 위임으로 변경했을 때, 테스트 40곳이 process_file을 직접 호출하고 있었다. 다행히 시그니처는 유지했지만, 내부 동작 변경(검증 메트릭 기록 누락)으로 테스트 1건이 실패. **공개 API 변경 시 호출자 전수 조사 필수**.

### 다음 세션에 적용할 규칙

1. 에이전트 위임 후 `cargo check` + 변수명 grep으로 정합성 검증
2. 5건 구현마다 spec 중간 동기화
3. UI 개발 시 `cargo tauri dev` 사용, 릴리스 빌드는 최종 1회만
4. 한 세션 목표는 5~7건으로 제한

## 핵심 교훈

1. 검증은 "거부"가 아니라 "피드백". 2-Pass가 증명.
2. 타입을 바꾸지 않고 필드를 추가한다.
3. 외부 크레이트 소스를 코드 작성 전에 읽는다.
4. 반성문이 부채를 가시화하고, 해결을 이끈다.
5. 벤치마킹은 수치로. "더 빠르다"가 아니라 "0.57ms@3000문서".
6. 대규모 테스트는 관측 가능해야 한다. 묵음 실행은 hang과 구분 불가.
7. 설정은 한 곳(config 파일)에서 관리하고, 환경변수는 오버라이드용으로만.
8. 일괄 구현 시 중간 동기화와 항목별 완료 체크를 건너뛰지 않는다.
9. 알림은 건별이 아니라 요약. "현황 대시보드"처럼 보여줘야 유용.
10. inbox는 "문서"만 받는다. config/소스코드/바이너리는 스킵 기준을 명시.
11. 구조체 필드 추가 시 테스트 초기화 코드도 반드시 일괄 수정. builder 패턴 검토.
12. 다이어그램은 코드와 동기화해야 의미가 있다. 기능 변경 시 docs도 갱신.
13. 자체 core trait도 반드시 읽고 구현체를 작성한다. 외부 크레이트 교훈(#3)의 확장.
14. Write 도구는 한글 경로에서 불안정. 기존 파일은 Edit 도구만 사용.
15. 프롬프트 등 어댑터 간 공유 로직은 별도 모듈(prompts.rs)로 분리. core에는 영향 없이.
16. core 크레이트는 std만 의존. tokio 등 런타임 특정 타입은 콜백/trait으로 추상화.
17. 구조체 필드 추가 시 Default derive 또는 builder 패턴 도입 검토. 20곳 수동 수정은 비효율.
18. 경쟁 분석은 넓게, 구현은 즉시/단기/중장기로 분류해서 좁게.
19. 대용량 파일은 truncate가 아니라 에이전트에게 청크 위임. Decorator 패턴으로 기존 LLM 래핑.
20. 배치 처리는 작업 큐 영속화 필수. 중단→재개, 변경→재처리, 삭제→정리를 캐시로 판단.
21. 상태 전이(Pending→Processing→Done) 로직은 한 곳에서만 plan에 추가. 중복 방지.
22. 기능 구현 = core + 모든 인터페이스(CLI/REST/MCP/Dashboard) 노출. 하나라도 빠지면 미완성.
23. CLI는 사용자 행동 기준으로 설계. 개발자 편의(watch/batch 분리)가 아닌 사용자 편의(start 하나).
24. 값은 한 곳(config)에서만 정의. 하드코딩된 매직 넘버는 기술 부채.
25. 기능 추가 전 기존 코드가 이미 처리하는지 확인. 불필요한 커맨드(backfill-sparse) 방지.
26. bin-only 크레이트는 외부 참조 불가. 공유 로직은 lib.rs로 추출하고 lib+bin 패턴 사용.
27. Tauri와 CLI가 같은 서비스를 공유할 때 헥사고날 장점 — 포트 교체로 대화형/비대화형 전환.
28. cargo clean은 빌드 환경 문제를 드러내는 파괴적 작업. 캐시가 링커/SDK 문제를 숨기고 있을 수 있다.
29. Git Bash의 /usr/bin/link가 MSVC link.exe를 가로챈다. Developer Command Prompt 또는 LIB/PATH 직접 설정 필요.
30. Playwright MCP로 브라우저 자동화 테스트 시 콘솔 에러까지 잡을 수 있다. 수동 테스트로 놓치는 종류의 버그.
31. 모달 분리 시 각 Cargo.toml에 의존성을 명시적으로 추가해야 한다. workspace 멤버일 때 암묵적 상속에 의존하면 분리 후 빌드 실패.
32. "하나의 모달 = 하나의 인터페이스". GUI에서 CLI 분기 코드를 제거하면 코드가 3배 간결해진다.
33. GUI 바이너리의 첫 사용자 경험: 콘솔로 초기화 진행을 보여주고, 완료 후 콘솔 숨기기. 묵음 시작은 "멈춘 건지 실행 중인지" 구분 불가.
34. 기본값은 외부 의존성이 없는 쪽으로. backend=qdrant_local보다 backend=sqlite가 첫 실행 에러 0건.
35. auto_init()은 main() 최초에 실행. config 로드보다 먼저 파일을 생성해야 기본값 대신 실제 파일을 읽음.
36. Tauri 2.0은 `window.__TAURI_INTERNALS__.invoke`이다. `window.__TAURI__.core.invoke`는 v1 API.
37. GUI 앱은 OS 레벨 Named Mutex로 싱글 인스턴스를 보장. CLI는 여러 인스턴스 허용.
38. Settings UI는 항목이 많아지면 VSCode 스타일(좌측 네비 + 검색 + 섹션별 표시)이 UX 최적.
39. config_metadata의 field_type에 `select:val=label` 패턴으로 셀렉트 박스를 범용 지원.
40. Settings 메뉴 구조 변경은 백엔드(config_metadata) 변경 없이 JS 그룹 매핑만으로 가능.
41. GUI 앱에서 println!은 CMD 창 깜빡임을 유발. write_log()로 파일에만 기록.
42. SqliteVec 영속화는 JSON 스냅샷이 가장 간단. upsert마다 전체 저장은 소규모에서 충분.
43. 테스트 격리는 DB 경로까지. SqliteVecAdapter::new()가 공유 경로를 쓰면 테스트 간 데이터 누적. with_path(temp)로 격리.
44. 검색 테스트는 존재 확인이 아니라 품질 측정. MRR, P@K, 랭킹 순서, 점수 갭까지 검증해야 회귀 감지.
45. 기본 설정은 아무것도 안 건드려도 즉시 동작해야 한다. dim, backend, pipelines 모두 포함.
46. 설정값과 어댑터 내부 고정값의 정합성 확인. dim=1536 + 128축 = 1408차원 패딩 → 정확도 희석.
47. Phase 완료 시 features 상태 + roadmap + spec 세 곳 동시 갱신. 하나라도 빠지면 정합성 깨짐.
48. core의 "std만 의존" 원칙은 `[dependencies]`에만 적용. `[dev-dependencies]`에 tokio 추가는 테스트 전용이므로 허용.
49. 핵심 서비스(service.rs)는 Stub 포트로 외부 의존 없이 테스트 가능. 작성을 미루면 부채가 기하급수적으로 증가.
50. 기능 추가 시 CLI/MCP/Tauri 커맨드 수를 spec에 즉시 반영. 주기적으로 grep 실측하여 drift 방지.
51. Phase 완료 시 features 파일 삭제 + roadmap 반영을 원자적으로 수행. 방치하면 정합성 깨짐.
52. 프롬프트 비용은 단일 LLM 호출이 아니라 전체 파이프라인(재시도 포함)으로 측정. 품질 높은 프롬프트가 2-Pass 재가공을 줄여 오히려 빠를 수 있다.
53. doc_types.toml은 "유형 정의"가 아니라 "검증 스키마". LLM이 자율 판단한 결과를 검증하는 기준만 제공하면 충분.
54. 프롬프트도 DRY 원칙 적용. 중복 프롬프트는 위임 패턴으로 단일 소스화.
