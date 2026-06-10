# Lesson 37 — Phase 84 일괄: 백엔드 정리 + 신규 UI + live reload

## 상황

Phase 84 작업으로 P1·P2·P3를 일괄 진행:
- P1: 후속 기능 6건 (Hook UI / Quarantine 분기 / search_with_trace / MCP 토글 / processing_metrics / config 이전 조사)
- P2: 백엔드 dead 정리 + clippy 0건화
- P3: C1/C2 live reload + A1 GC 가시화

## 문제 / 발견

### 1. "후속 작업 11건" 표현의 실제 분포

architecture.md에 "Backend dead 11건" 명시되어 있었으나 실 grep 결과:
- 7건은 명시적 정리 대상 (Phase 64 주석 참조)
- 2건(`setup_snapshot_list/rollback`)은 미연결 안전 기능 — 삭제 아닌 **UI 연결**이 적절
- 2건은 차이(`kg_paths` 등)는 안내 텍스트에만 등장 = 호출은 없지만 도메인 핵심 API라 보존

**메타 룰**: "dead 코드"라는 표기가 있어도 일괄 삭제가 정답 아님. 호출 graph 외에 **도메인 중요도/안전 가치**도 함께 판단.

### 2. lesson 36 "잔존 8건" vs --all-targets 22건

clippy 잔존 카운트가 lib(8) vs --all-targets(22)로 큰 차이. 본 작업은 lib 8건만 0으로. tests/bench 14건은 `useless_vec` 등 자동 fixable이라 회귀 시 cargo clippy --fix로 일괄 처리 가능.

### 3. clippy --fix 후 빌드 캐시 충돌

`cargo clippy --workspace --fix --allow-dirty` 실행 후:
- Cargo.lock이 일부 패키지 갱신 발생
- 이후 `cargo build --tests --workspace`에서 E0786 "invalid metadata files" + E0460 "newer version" 대량 발생
- `cargo clean` 1회로는 부족 — `modals/app/target/`도 별도 clean 필요
- 추가로 **페이징 파일 부족(os error 1455)**: 기본 jobs 13개로 동시 빌드 시 메모리 부족. `-j 2`로 회피

**메타 룰**: clippy --fix는 caching 안정성에 영향. 직후 빌드 검증할 때는 `cargo clean -p <crate>` 또는 `-j` 제한 고려.

### 4. async + RwLockReadGuard Send 불가

`pii_user_patterns: Vec → RwLock<Vec>` 전환 시 `tokio::spawn` 이후 사용처에서:
```
error: RwLockReadGuard cannot be sent between threads safely
```
→ `read().expect(...).clone()`로 owned Vec 추출하는 패턴이 필요. 가드를 들고 가지 않음.

### 5. metadata 충돌의 false-positive 진단

빌드 메모리 부족(페이징)으로 `mmap` 실패 → rustc가 "invalid metadata files" / "can't find crate" 에러를 다수 표출. 코드 자체엔 문제 없으나 mmap I/O 실패가 metadata 손상처럼 보고됨. `-j 2`로 메모리 압박 줄이면 해소.

## 개선

### 즉시 적용

- **clippy --fix → 빌드 검증 시퀀스**: `cargo clean` + `cargo build -j 2` (메모리 제약 환경)
- **mmap os error 1455 진단**: "invalid metadata"가 동시에 발생하면 메모리/페이징 의심
- **async에서 lock guard 보관 금지**: 항상 `read().clone()` 또는 `let v = ... .clone(); drop(guard);`

### 메타 룰 갱신 후보 (META.md)

- **메타 룰 9 (신규 후보)**: "빌드 진단은 자원(메모리·디스크) 먼저 확인" — code 변경 직후 metadata 에러 대량 발생 시 코드 retry보다 자원 확인.
- 메타 룰 5 보강: "트리거 대기 항목 = 코드 변경 없이 켤 수 있는 형태" → snapshot list/rollback이 정의만 되고 UI 미연결이었던 패턴 재확인. 본 phase에서 connection 완성.

## 코드 변경 요약

### 추가
- `crates/shared/src/settings_db.rs`: `mcp_disabled_tools` / `llm_cache_gc_log` 테이블 + API 6개
- `crates/shared/src/mcp_server.rs`: list_tools 필터링 + call_tool 차단
- `crates/core/src/service.rs`: `ProgressCallback` type alias, `reload_pii_patterns()`, `pii_user_patterns: RwLock<Vec>`
- `modals/app/src/commands.rs`: `search_with_trace` / `mcp_tools_list` / `mcp_tool_set_enabled` / `gc_llm_cache_now` (+ `reload_service_pii` private helper)
- `modals/app/src/main.rs`: invoke_handler 갱신 (delete 7 + add 5)
- `ui/dashboard.js`: Hook CRUD 모달 + MCP 토글 패널 + search trace 단계 렌더 + GC 버튼 + last_gc 카드 + Decision Log rollback 버튼
- `ui/index.html`: LLM 캐시 헤더에 GC 버튼 + 카드 2개
- `ui/dashboard.css`: `.pb-node-branch` + `.pb-node-branch-badge`

### 삭제
- `modals/app/src/commands.rs`: 7개 fn 본문 (~80 LOC) + main.rs 등록 3 라인

### 결과
- clippy lib **0건** (8 → 0)
- workspace lib **332/332** 통과
- 통합 테스트 빌드 통과 (12 파일)
- settings.db 테이블 +2 (총 6 신규)

## 후속 트리거 (Phase 84 작업 중 발견)

- **ServiceBuilder에 `with_pii_user_patterns` 메서드 없음**: `pii_user_patterns: RwLock<Vec>` 전환 후에도 test_helpers.rs는 빈 Vec로 초기화. 통합 테스트에서 PII 패턴 주입이 필요해질 때 with_* 메서드 추가 (lesson 27 빌더 확장 패턴).
- **`-j 2` 빌드 제한 일상화 가능성**: 페이징 부족이 재발하면 `.cargo/config.toml`의 `[build] jobs = 2` 영구 설정 고려.
- **Tauri Cargo.lock 갱신 영향**: clippy --fix가 Cargo.lock을 건드린 후 modals/app 빌드가 별도 long rebuild를 요구. cargo update를 의도적으로 분리하는 워크플로 검토.

## 참고

- 본 phase는 architecture.md "후속 phase (Phase 72+)" + "후속 검토 (Phase 70+)" 항목을 묶음 처리.
- 5K 코퍼스 트리거 대기 항목(MinHash / 메타블로킹 / A1 hit률 / A2/B1 디폴트 / C2 FP)은 본 phase 범위 외.
