---
phase: 90+
date: 2026-05-19
topics: GUI 전수 검증 / Playwright HTTP 한계 / Claude CLI 산발 실패 / Tauri 빌드 시점 / 서브탭 invoke 의존
related_lessons: 32, 55(archive), 59(archive), 12, 18
related_meta_rules: 14, 16
---

# 46. Phase 90+ GUI 전수 검증 세션 — GUI 검증 한계 + Claude CLI 산발 실패 + Tauri 빌드 시점

## 상황 (공통)

Phase 90 Notion 어댑터 추가 직후 사용자 요청: "GUI의 모든 기능을 나열하고 클릭 기능 테스트 + 데이터 입력 시 연관된 기능들 + 정상 동작 검토". 본 세션은 4영역 검증:
1. 80개 액션 카탈로그 작성 (코드 grep)
2. Playwright HTTP 모드 자동 검증 (DOM 정합성)
3. 데이터 입력 6종 흐름 grep 추적 (frontend → invoke → commands.rs → service → DB)
4. CLI 가공 9건 + Tauri 실행 → 본인 클릭 검증 시나리오 안내

본인 GUI 클릭 결과는 미보고 상태에서 사용자 종료. 검증 환경(`D:/file-test/bench_gui_phase90`, Tauri PID 33600)은 보존.

## 이슈 1: Playwright HTTP 모드는 invoke 의존 영역 미검증 (lesson 32 재확인)

### 상황

`python -m http.server`로 `ui/` 정적 서빙 → Playwright MCP `browser_navigate` → 80개 액션 중 23개 정적 + 동적 진입 시 86개까지 등장 확인.

### 문제

invoke 의존 영역 5건 미렌더:
- Settings nav items (`get_config` 응답 의존)
- Pipeline batch sections (invoke)
- Pipeline 서브탭 (`pb-subtabs` DOM은 `_renderSubtabBatch` 결과에 동적 생성, invoke 응답 의존)
- 검색 결과 영역
- 모듈 12 체크박스 (`setup_modules_list` invoke)

특히 Phase 90 신규 Notion 옵션이 HTTP 모드에서 미노출됨 (dashboard.js:3782 코드 존재는 grep으로 검증).

### 원인

직접 원인:
- HTTP 서버는 `__TAURI_INTERNALS__` 미존재 → `window.__TAURI_INTERNALS__.invoke` 호출이 fallback `Promise.resolve({})` 반환
- DOM 동적 생성이 invoke 응답 데이터에 의존하면 HTTP 모드에서 미렌더

구조적 원인:
- frontend 렌더 흐름이 "invoke 응답 → DOM 생성" 단일 경로. invoke 실패 시 fallback UI 없음
- lesson 32(`5K 합성 코퍼스 측정 인프라`)에서 이미 식별된 한계가 메타 룰화 안 됨

### 개선

- [ ] **invoke 미동작 영역 사전 분류 의무**: 정적 자원만으로 검증 가능 vs invoke 필수 영역을 코드 주석에 명시
- [ ] **Pipeline 서브탭 fallback**: invoke 응답 없을 때 빈 상태 UI 또는 placeholder 자동 표시 (G-4 트리거)
- [ ] **Playwright 검증 한계 문서화**: `doc/gui-test-scenarios.md` 미생성 상태 — 옵션 D 정형화 트리거 도달 시 작성
- [ ] **Tauri 환경 통합 테스트**: `modals/app/tests/` 신규 (옵션 C, 1일+) — Tauri commands 직접 호출로 invoke 우회 검증

### G-4 재진단 (2026-05-20, browser-automation MCP v0.2 전환 후)

**MCP 전환**: `@playwright/mcp@latest` (도구명 `browser_navigate` 등) → `_mcp/browser-automation` (도구명 `navigate`/`click`/`extract_structured` 등). v0.2 노출 도구 10종. 부트스트랩 시 Chromium 캐시 v1217→v1223 mismatch로 `install-browsers` 1회 재실행 필요 (옛 v1217 캐시 674 MB 회수).

**HTTP 모드 재측정 결과**:

| 영역 | 상태 | 진짜 원인 | 분류 |
|------|------|----------|------|
| `pb-subtabs` (서브탭) | 미렌더 | **HTML 엘리먼트 자체 없음** (index.html에 #pb-subtabs 미존재). `_renderPBSubtabs` + `PB_SUBTABS` 배열은 dashboard.js에 남아 있지만 `getElementById('pb-subtabs')`가 null이라 즉시 return | **dead-code (lesson 14/19)** |
| `pb-midtabs` + 인스펙터 47 노드 | 정적 렌더 | dashboard.js가 invoke 없이 정적 데이터 렌더 | 정적 — Playwright HTTP 검증 가능 |
| `pb-batch-config` | "설정을 불러오는 중..." | invoke 응답 대기 placeholder 존재 | invoke-fallback (좋은 패턴) |
| Settings 그룹/필드 | **재진단 (2026-05-20)**: 사실 placeholder 작동 중 ("설정 편집은 Tauri 앱에서만 가능합니다...") | 원래 lesson 검증 시 `#settings-content` 셀렉터로 검사했으나 실제 ID는 `#settings-form` — 셀렉터 오류로 빈 상태로 오인 | 정상 (lesson 추정 오류) |
| Documents 검색 결과 | "아직 가공된 문서가 없습니다" + 시작 가이드 | renderDocList line 481~490의 친절 안내 | 정상 |
| Processing | "작업 내역이 없습니다" + 카드 0 | 빈 객체 키 `\|\| 0` 가드 | 정상 |
| Todos | "할일 없음" | renderTodos placeholder | 정상 |
| Topics | "토픽 없음" | renderTopics placeholder | 정상 |
| `setup-modules` 모달 | "로딩 중..." + 에러 처리 | openSetupModules 패턴 | 정상 (모달 안 — 페이지 진입 시 미노출이 정상) |
| **Verification 카드** | "TOTAL undefined / PASS undefined / WARNING undefined / FAIL undefined" | **`renderVerificationMetrics` line 1775의 `if (!m)` 분기가 빈 객체 `{}`를 truthy로 판정 → `m.total` (undefined) 그대로 렌더** | **invoke-no-fallback (진짜 G-4 본질)** |

**원래 G-4 추정 수정**: 본 lesson 처음 작성 시 "pb-subtabs DOM이 동적 생성 → invoke 응답 의존" + "Settings/검색/모듈 체크박스 invoke-no-fallback"이라고 추정. 실제는:
1. `pb-subtabs`는 **HTML 엘리먼트 자체가 없는 dead-code 패턴** (옛 IA Phase 56/67 잔재)
2. Settings/Documents/Processing/Todos/Topics/모듈 모달은 **모두 정상 placeholder 작동 중**. 원래 검증 시 셀렉터 오류 (`#settings-content` vs `#settings-form`)로 빈 상태로 오인
3. **진짜 invoke-no-fallback는 Verification 카드 한 곳**: 빈 객체 `{}`가 truthy라 placeholder 분기 통과 → undefined 노출

**G-4 진짜 트리거 재정의** (두 갈래):
1. **dead-code 정리**: ✅ 종결 (2026-05-20). dashboard.js -271줄 / dashboard.css -5 rule / lesson 47 작성 (lesson 19 변형, JS↔HTML 셀렉터 불일치 메타 패턴, 메타 룰 1 14번째 사례)
2. **invoke-no-fallback UX 개선**: ✅ 종결 (2026-05-20). `renderVerificationMetrics`의 빈 상태 가드를 `!m || typeof m.total !== 'number'`로 강화. "TOTAL undefined" → "검증 메트릭이 아직 없습니다. 가공이 완료되면 표시됩니다." 표시. 다른 영역은 이미 잘 처리됨

위 두 작업은 별개 trigger로 분리 필요. 메모리 `project_g4_diagnosed` 신규 작성.

## 이슈 2: Claude CLI exit code 1 산발 실패 (9건 중 5건)

### 상황

CLI 가공 `pipeline.exe process` 9건 시도 → 4건 성공 (reference 3 / guide 1 / study 3), **5건 실패** (LLM 분류+가공 실패, Claude CLI exit code 1). 단일 파일 719 chars (`jdk keystore 목록_추가_삭제_jdk 인증서 추가.txt`)도 실패. 재시도 2회 모두 실패.

per-doc 120.2s (Phase 89 측정 48.1s 대비 +2.5x — 실패 재시도 + fastembed OFF 영향).

### 문제

- 동일 파일이 단일 실행 내에서 여러 번 재시도 후 모두 exit 1 — 일시적 실패가 아닌 패턴 가능성
- quarantine으로 라우팅되지 않고 inbox 잔존 (실패 후 work-queue가 inbox에서 제거 안 함)
- GUI에서 Processing 탭 [실패 항목 재처리]로 재시도 시 동일 오류 재현될지 불명 (검증 미수행)

### 원인

직접 원인 (추정):
- claude CLI 동시 호출 rate (multiple processes 진행 가능)
- stdin 파이프 (Phase 54 도입 — 32KB 명령줄 회피)이 짧은 입력에서 EOF/SIGPIPE 발생
- fastembed OFF로 Claude CLI 임베딩(128축) 추가 호출 발생 → 동시 claude CLI 인스턴스 충돌

구조적 원인:
- LLM 어댑터에 retry 로직은 있지만 (`max_retry=1`) exit code별 차등 처리 없음
- 동시성 제어 (max_workers=4)가 LLM 호출까지 전파되는지 불명
- 실패 후 quarantine vs inbox 잔존 정책이 모호 — `verification.on_fail = "quarantine_with_notify"`이지만 LLM 호출 자체 실패는 검증 단계 이전이라 분기 미적용

### 재현 결과 (2026-05-20)

격리 환경 `D:/file-test/bench_g1_repro`에서 1차 실패 환경 재현 시도:

| 시각 | 환경 | 파일 | 결과 |
|------|------|------|------|
| 19:07:30 (1차, process.log) | max_workers=4, 9건 동시, 실제 코퍼스 | 9건 | **4 성공 / 5 실패** |
| 19:18:00 (2차, GUI pipeline.log) | max_workers=4, 5건 동시, 동일 실패 파일 | 5건 | **5 성공 / 0 실패** |
| 10:23 (3차, 단독) | max_workers=1, 632 bytes 단일 파일 | 1건 | **1 성공 (48.2s)** |
| 10:46 (4차, 동시) | max_workers=4, 9건 동시, 합성 파일 (426~6764 bytes) | 9건 | **9 성공 (per-doc 24.8s)** |

**결론**: G-1은 **외부 일시적 요인**으로 결론. Claude API 측 일시 장애 / 네트워크 흔들림 / claude CLI 인증 토큰 갱신 등 추정. 동시성 4 + 다양한 파일 크기 재현 시도 전부 성공.

**메타 룰 18 재검증 상태 (2026-05-26 Phase 96 자기 적용)**:
- Phase 91 audit_trace 인프라 도입 + Phase 94 H1 audit_anomaly 주기 호출 완료
- audit_trace 누적 50건+ 도달 시 root cause 자동 확정 가능 (현재 누적 미도달 — 사용자 본격 가공 50파일+ 신호 대기)
- ❓ 추정 검증 보류 — audit_trace 누적 데이터 미도달로 검증 불가. 사용자 코퍼스 신호 도달 시 재검증

### 구조적 약점 (재현 실패와 무관)

`module-llm/src/claude_cli.rs:57-102` 검토에서 식별:

1. **stderr 빈 문자열에 진단 정보 0** — 1차 실패 5건 모두 `Claude CLI 오류 (exit code: 1):` 뒤가 비어 있어 분류 불가
2. **exit code별 차등 처리 없음** — 1/130/SIGPIPE 모두 `LlmError::Backend` 단일 매핑
3. **timeout 없음** — `wait_with_output()` 무한 대기. 1차 실패의 첫 호출이 89초 stall
4. **stdin half-close 미명시** — drop은 블록 끝 자동이지만 명시 처리가 안전
5. **재시도 정책 부재** — 어댑터 내부 retry 없음

### 적용된 조치 (2026-05-20, F-1~F-5)

- ✅ **F-1+F-2+F-3** `_rust_module/module-llm/src/claude_cli.rs` 재작성:
  - exit code + elapsed + stderr 앞 200자를 항상 에러 메시지에 포함
  - 300초 timeout 폴링 (`try_wait` + `kill`)
  - stdin 명시 `flush + drop`
- ✅ **F-4** 빈 stderr + exit 1 패턴 감지 시 500ms 대기 후 어댑터 내부 1회 자동 재시도 (호출처 `max_retry`와 독립)
- ✅ **F-5** `file-pipeline-core/src/service.rs`: LLM 호출 실패 시 quarantine 라우팅 (`process_file` line 285 + `process_file_with_pipeline` line 731 두 호출처)
  - 검증 실패의 quarantine 분기와 동일 패턴 — notification + record_error + metrics_quarantine + 파일 이동
  - 현재 work-queue.failed로만 분류되던 LLM 호출 실패가 사용자 가시화됨

### 후속 (트리거 대기)

- [ ] **동시 호출 제한**: 어댑터 내부 Semaphore (max_workers와 별개 LLM 상한). 본 재현에서 max_workers=4 동시 9/9 성공이라 즉시 필요성은 낮음. 다음 산발 실패 재발 시 검토
- [ ] **G-2 fastembed ON release 빌드 후 재측정**: 본 재현은 fastembed OFF에서도 9/9 성공. G-2는 속도/품질 트리거로 분리 (실패율 관련 없음)

## 이슈 3: Tauri 빌드 시점 ≠ Phase 변경 시점

### 상황

Phase 90 Notion 어댑터 추가 후 pipeline.exe만 재빌드 (Q1 답변에 따라). Tauri는 dashboard.js만 변경이라 재빌드 보류. 검증 시 Tauri exe 시각 17:29 (Phase 89까지만), pipeline.exe 시각 18:35 (Phase 90 반영).

### 문제

- Notion 옵션은 dashboard.js에 있어 UI에 표시되지만 invoke 호출은 Phase 89까지의 build_service만 사용 → "provider=notion 선택해도 실 라우팅 안 됨" 상태
- 사용자가 UI를 보고 "Notion 통합 완료"로 오인할 가능성
- Phase 변경분 → 어떤 바이너리 재빌드 필요 + 어떤 변경은 자원만 (UI/prompts.toml)인지 명시 부재

### 원인

직접 원인:
- Tauri 재빌드 비용 큼 (26m+) → 자원만 변경된 phase는 재빌드 회피 자연스러움
- 그러나 build_service 분기 변경은 Tauri 바이너리에 컴파일 포함됨 → 자원 변경과 코드 변경 구분 필요

구조적 원인:
- Phase 변경 분류 표 부재 (코드 변경 / UI 자원 변경 / prompts.toml / config 디폴트 변경)
- release 빌드 시점 추적이 코드 변경과 별도 흐름 — 메타 룰 후보

### 개선

- [ ] **Phase 변경 분류 의무**: 매 phase 헤더에 "재빌드 필요 영역" 명시 (workspace / Tauri / 자원만)
- [ ] **build script**: `build-all.ps1` 같은 일괄 스크립트로 phase 종료 시 자동 재빌드
- [ ] **메타 룰 후보 (메타 룰 17)**: "코드 변경 phase의 release 재빌드 의무" — Tauri 비용 큼이라 사용자 결정 영역이지만 명시 의무
- [ ] **G-3 트리거**: Notion 실 통합 검증 시점에 Tauri 재빌드 (현재 26m 대기 + 사용자 결정)

## 공통 교훈

1. **invoke 의존 vs 정적 자원의 경계가 검증 한계를 결정**: HTTP 모드는 정적 자원만 검증 가능. Tauri 환경 실 검증은 본인 클릭 또는 통합 테스트(modals/app/tests/) 필수
2. **LLM 어댑터 안정성은 단일 측정으로 결정 불가**: 9건 중 5건 실패라는 산발 패턴은 단일 진단 어렵고 재현 환경 분리(fastembed ON/OFF, 동시성 1) 필요
3. **release 빌드 시점은 phase 변경 시점과 분리 추적**: workspace + Tauri 두 바이너리 비대칭 비용 (2분 vs 26분). 자원만 변경된 phase는 재빌드 보류가 자연스러우나 코드 변경 phase는 의무
4. **본 세션은 lesson 14 (인프라 토대 누적) + lesson 30 (측정 후 활성화)의 검증 단계** — 메타 룰 13(인프라 활성화 4단계) 중 "측정 → UI 노출" 사이의 GUI 검증 단계가 신규 패턴으로 식별됨

## 잘한 것 (재사용 가능)

1. **80개 액션 카탈로그 grep 자동 추출**: `grep -ohE 'data-action="[^"]+"' ui/*.html ui/*.js | sort -u | sed ...` 패턴. 다음 GUI 검증 시 즉시 재사용 가능
2. **Playwright HTTP 모드 한계 명시 + grep 흐름 추적 병행**: HTTP 모드에서 검증 가능한 정적 영역(20/20 통과)과 grep으로 검증 가능한 invoke 의존 영역을 분리하여 보완. lesson 32의 "HTTP 모드 invoke 미동작" 한계 회피 패턴
3. **데이터 입력 흐름 6종 grep 추적**: frontend `data-action` → `API.X` → `invoke` → `commands.rs` → `service` → `DB`를 grep 6단계로 검증. 통합 테스트 없이도 연쇄 검증 가능

## 다음 세션 플래그

- **G-1**: ✅ 종결 (2026-05-20) — 외부 일시 요인 + F-1~F-5 코드 강화
- **G-2**: 보류 — fastembed ON 재측정 (속도/품질 트리거, G-1과 별개)
- **G-3**: ✅ 종결 (2026-05-20) — Tauri 재빌드 7m 19s로 Phase 90 Notion + F-1~F-5 모두 반영
- **G-4**: ✅ 종결 (2026-05-20) — (a) dead-code 271줄 삭제 + lesson 47 / (b) Verification 카드 빈 객체 가드 강화
- **G-5**: ✅ 종결 (2026-05-20) — `spec/benchmarks/scripts/` 5종 자동화: action_catalog.sh / dead_selector_scan.sh / empty_state_audit.sh / data_flow_trace.sh / gui_http_smoke.sh + README. META.md "Phase 종결 시 GUI 회귀 자동 검증" 체크리스트 추가. **#7 git pre-push hook 등록 (2026-05-20)**. **#8 AST 정밀화 dead_selector_scan_v2.js (acorn 기반) 추가**
- **본인 GUI 클릭 결과 보고 대기**: 시나리오 1, 3, 11 (Phase 89 N-4 / Phase 84 LLM 캐시) 검증 시 추가 사이드 발견 가능
- **메타 룰 17**: ✅ META.md 정식 승격 (2026-05-20) — "코드 변경 phase의 release 빌드 시점 의무화"
- **메타 룰 18**: ✅ META.md 정식 승격 (2026-05-20) — "lesson 본문의 추정 사항은 다음 phase에서 재검증 의무" (G-1/G-4 추정 빗나감 2회 누적 후)
- **dead_selector_scan baseline 14건**: ✅ G-6 종결 (2026-05-20). 13건 진짜 dead 일괄 삭제 + 1건(settings-no-results) whitelist
- **G-7 Tauri commands 9건 백엔드 정리**: ✅ 종결 (2026-05-20) — main.rs invoke_handler 10건 + commands.rs 함수 본체 (search_with_trace / purge_dry_run / purge_execute / list_doc_types / save_doc_type / delete_doc_type / refresh_host_tools / test_preprocess / mcp_tools_list / mcp_tool_set_enabled) 일괄 삭제. commands.rs **-366줄**. 빌드 통과 (workspace + Tauri cargo check)
