# 교훈 #19 — Tauri commands dead code + CLI enum 이중 정의

## 상황

Phase 55에서 Feedback 탭 UI를 제거했으나 Rust 백엔드의 `feedback_*` 7개 Tauri commands가 잔존(lesson #13 패턴 재발). 추가로 CLI Commands enum이 `modals/cli/src/main.rs`(활성)와 `modals/cli/src/cli.rs`(dead)에 이중 정의돼 있음. Phase 64 매핑 점검(2026-04-30)에서 발견.

## 문제

1. **dead Tauri commands 11개**:
   - `feedback_check_mode / feedback_submit / feedback_list / feedback_diff / feedback_undo / feedback_recommendations / feedback_learn` (7개)
   - `credential_store_available / credential_store_save / credential_store_get / credential_store_delete` (4개) — Phase 60 secrets 분리 후 shim 잔존, UI 호출처 0건

2. **dead 파일**:
   - `modals/cli/src/cli.rs` — `pub mod cli` 선언 어디에도 없음. 완전한 dead code

3. **수치 모순**:
   - `spec/architecture.md`: "Tauri commands 61개" → 실측 50개 (정리 후)
   - `spec/webapp-design.md`: "(58개)" → 실측 50개
   - "CLI commands 18개" — 어느 모달인지 명시 없음 (modals/cli=18 vs shared=12)

## 원인

- **lesson #13 재발**: UI 기능 제거 시 백엔드 commands 정리가 누락됐고 invoke_handler 등록 7건도 그대로
- Phase 60 secrets 분리 시 호환성 shim(credential_store)을 만들었으나 신 API(`save_credential` 등)로 모든 호출처가 이전된 후 shim Tauri commands를 정리하지 않음
- enum Commands가 3곳에 정의되어 (`shared/cli.rs` / `modals/cli/main.rs` / `modals/cli/cli.rs`) 어느 것이 활성인지 불명확. 후자 둘은 거의 동일했고 한 쪽은 mod 선언 없이 dead

## 개선

### 즉시 처리 (2026-04-30 적용)

1. `modals/app/src/commands.rs` line 1162~1487 (feedback 영역 326줄) 일괄 삭제
2. `modals/app/src/commands.rs` line 1332~1359 (credential_store_* 28줄) 삭제
3. `modals/app/src/main.rs` `invoke_handler!` 등록 11건 삭제
4. `modals/cli/src/cli.rs` 파일 자체 삭제 (mod 선언 없음 = dead)
5. spec/architecture.md 수치 정정 + 모달 진입점 매핑 표 추가

### 재발 방지 체크리스트

UI 기능 제거 시 다음 8단계 (lesson #13 확장):

1. UI HTML/CSS/JS 제거
2. **Tauri commands 함수 삭제** (commands.rs)
3. **invoke_handler 등록 삭제** (main.rs)
4. **dependent struct/static 삭제** (예: FeedbackEntry, FEEDBACK_LOCK)
5. **보조 함수 삭제** (find_source_root, git_cmd 등 — `cargo check` 경고로 식별)
6. core/adapters 호출처 grep 후 정리
7. settings.db / config 필드 정리
8. spec/architecture.md 수치·매핑 갱신
9. **JS API.* 객체에서 호출 함수 삭제** (`ui/dashboard.js`) — 백엔드 commands가 사라지면 frontend invoke 호출도 dead. 호출처 grep 후 정리. 삭제 사유를 주석으로 명시 (예: `// Phase 56: 단일 파이프라인 구조로 전환 — list/delete/reorder 백엔드 제거됨`)
10. **JS render 함수 + DOM 셀렉터 정합성 검증** — `getElementById('xxx')`로 참조하는 ID가 `index.html`에 실제 존재하는지 grep. 누락 시 함수가 silent fail (Phase 61 `#search-results` 누락이 이 패턴)

### enum 중복 방지

- `enum Commands`처럼 모달별 노출 명령이 다른 경우, **각 모달이 자체 enum을 main.rs에 직접 정의** (현재 modals/cli 패턴)
- 별도 `cli.rs` 파일에 enum을 두면 mod 선언 누락으로 dead 위험. main.rs와 cli.rs 분리 시 반드시 `mod cli;` 선언 + `use cli::Commands` 검증
- spec 표기는 모달별 분리 (예: "modals/cli 18개 / shared 12개")

## 재발 방지

- 매 Phase 종료 시 `cargo check --all 2>&1 | grep "never used"` 실행 → 경고 0건 확인
- UI 기능 제거 시 위 10단계 체크리스트를 commit message에 명시
- 모듈 정의 후 `mod` 선언 검증: `grep -r "mod {filename}" src/` 실행
- frontend 정합성 검증:
  - `grep -oE "call\(['\"]([a-z_]+)" ui/dashboard.js`로 invoke 호출 추출 → 백엔드 `#[tauri::command]` 목록과 대조
  - `grep -oE "getElementById\(['\"]([a-z-]+)" ui/dashboard.js`로 DOM 셀렉터 추출 → `index.html`의 id 목록과 대조

## 추가 발견 (Phase 80 시점, 2026-05-14 검증)

MCP 도구 추가 시 Tauri commands 누락 패턴 6건 식별 (frontend dead):

| 명령 | Phase | MCP 등록 | Tauri 등록 (이전) | JS 호출 | 증상 |
|------|-------|---------|------------------|---------|------|
| setup_modules_list / setup_apply_modules | 80 | ✅ | ❌ | ✅ | "직접 동작 모듈 선택" 모달 silent fail |
| get_search_mode_stats | 80 | ✅ | ❌ | ✅ | 검색 mode 분포 카드 빈 데이터 |
| get_crag_stats | 80 | ✅ | ❌ | ✅ | CRAG 신뢰도 카드 빈 데이터 |
| get_chunk_stats | 80 | ✅ | ❌ | ✅ | 청크 통계 카드 빈 데이터 |
| get_processing_metrics | 80/82-prep | ✅ | ❌ | ✅ | 처리 메트릭 카드 빈 데이터 |

**해소 (2026-05-14)**: `commands.rs`에 6개 Tauri command 추가 + `main.rs` invoke_handler 등록. Tauri commands 56 → 62. JS-Tauri frontend dead 6 → 0건. settings.db 직접 접근으로 MCP McpState 메모리 카운터와 동등 동작.

## 메타 룰 (lesson 19 확장)

**신규 규칙**: 새 MCP 도구 추가 시 다음 4가지 등록 위치를 모두 점검한다.

1. `crates/shared/src/mcp_server.rs` (MCP 측 — make_tool + handler)
2. `modals/app/src/commands.rs` (Tauri 측 — `#[tauri::command]`)
3. `modals/app/src/main.rs` (invoke_handler 등록)
4. `ui/dashboard.js` (JS API.* 객체에 call() 헬퍼)

JS만 등록되고 Tauri 누락 = frontend dead. Tauri만 등록되고 JS 누락 = backend dead. 양쪽 grep으로 차이 검출.

본 메타 룰은 META.md 메타 룰 1 "다중 위치 동기화 누락"의 구체화 사례.

## 추가 발견 (Phase 64, 2026-04-30)

MCP Playwright 단위 테스트 중 frontend 측 dead code 3건 추가 발견:
- `dashboard.js renderSearchResults` 함수 — 호출처 0건 + `#search-results` ID는 index.html에 없음. Phase 61 hierarchy breadcrumb이 미작동했던 원인
- `API.listPipelines / deletePipeline / reorderPipelines` — Phase 56에서 백엔드 삭제됐지만 frontend 잔존
- 처리: `renderDocList`로 hierarchy 통합(doc-table 컬럼) + dead API 함수 정리
