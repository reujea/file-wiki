# Lesson 35 — C2 service.rs 통합 + C1/C2 사용자 정의 + Decision Log 필터 + clippy --fix

## 상황 (2026-05-15)

즉시 가능 5건 일괄 처리:
1. C2 PII service.rs 통합 (lesson 14 미연결 포트 패턴 재발 방지)
2. C1 임계값 사용자 설정 (settings.db c1_rule_thresholds)
3. C2 PII 패턴 사용자 설정 (settings.db pii_patterns_user + GUI CRUD)
4. Decision Log 필터/정렬 (status + source + sort)
5. clippy warning 점진 정리 (cargo clippy --fix)

## 문제 / 결정 사항

### C2 통합 위치 — fragment 직전 vs 직후 vs 통합

`process_file_legacy`/`process_file_with_pipeline` 양쪽 모두 1) 경로 sensitive → 2) fragment → 3) hash 순서.

PII 본문 검사는 fragment에서 이미 `read_to_string`한다 — 같은 I/O 재활용이 효율적. 결정:
- 1.3 단계 신설: read_to_string 직후 PII 검사 → 발견 시 handle_sensitive
- 1.5 단계: 동일 content로 fragment 체크 (I/O 1회만)

### C1 임계값 — DB vs ENV vs config TOML

3가지 옵션:
1. settings.db 신규 테이블 (선택)
2. 환경 변수
3. pipeline.toml 신규 섹션

선택 이유: settings.db에는 이미 카운터/decision_log/llm_cache 등 자동 관리되는 운영 데이터가 있음. C1 임계값도 사용자가 GUI에서 조정하고 즉시 반영되는 운영 데이터에 가깝다. TOML 섹션은 사용자가 직접 편집 가능한 의도된 설정 — 임계값은 점진 미세조정 대상이라 GUI 우선.

### C2 PII 패턴 — core vs shared 의존 방향

`SensitivityDetector`는 core 도메인. `SettingsDb`는 shared. core → shared 금지.

해결: `scan_pii_in_text_with(text, &extra)` 시그니처로 추가 패턴을 인자로 받음. `FileProcessingService.pii_user_patterns: Vec<(String, String)>` 필드에 보관 (lesson 21/27 — 신규 도메인 필드 추가 시 ServiceBuilder + test_helpers 동시 갱신 의무).

`build_service`가 settings.db → 패턴 로드 → 필드 주입.

### clippy fix — auto vs manual

`cargo clippy --fix --allow-dirty` 로 71 → 18 (74%). 자동 수정 안전 패턴:
- field_reassign_with_default → struct literal
- unused imports
- needless_borrow
- redundant_closure

수동 처리 필요 잔존 18건 — wildcard_in_or_patterns / format_in_format_args 등 의도성 있는 것들.

## 원인

1. **C2 함수만 두고 미연결** (lesson 14 재발): `scan_pii_in_text`을 만들고 service.rs 호출 누락 가능성 — 이번에 발견 즉시 통합. 신규 함수 추가 시 호출처 grep 의무.
2. **C1 임계값 하드코딩**: 측정 표본/사용자 환경마다 적정값 다름. DB 룰로 가변화.
3. **PII 패턴 코드 고정**: 도메인별로 추가 패턴 필요 (예: 금융=계좌번호, 의료=환자번호). DB로 가변화.
4. **Decision Log 200건 fetch 후 클라이언트 필터**: 5K+ 항목 시 비효율. 현재 단계는 충분, 향후 DB 쿼리 필터링으로 확장.
5. **clippy CI 부재**: warning 71건은 한 번에 누적된 게 아니라 점진 — CI에 통합되어 있었으면 PR 단위로 막혔을 것.

## 개선

### 도메인 필드 추가 시 ServiceBuilder 갱신 (lesson 21/27 재확인)

```rust
// service.rs
pub struct FileProcessingService {
    ...
    pub pii_user_patterns: Vec<(String, String)>,  // NEW
}

// test_helpers.rs::ServiceBuilder::build()
FileProcessingService {
    ...
    pii_user_patterns: Vec::new(),  // 동시 갱신
}

// service.rs 도메인 생성 (test 폴백)
sensitivity_detector: SensitivityDetector::default(),
pii_user_patterns: Vec::new(),  // 동시 갱신

// lib.rs build_service
let pii_user_patterns = settings_db::SettingsDb::open(&db_path)
    .and_then(|db| db.list_user_pii_patterns())
    .ok().unwrap_or_default()
    .into_iter()
    .filter(|(_, _, enabled)| *enabled)
    .map(|(n, p, _)| (n, p))
    .collect();
```

4곳 동기화 — `cargo check --workspace`로 lib만 검사, `cargo build --tests --workspace`로 통합 테스트까지 검증 필수.

### DB 사용자 정의 패턴

```sql
CREATE TABLE c1_rule_thresholds (key TEXT PRIMARY KEY, value REAL NOT NULL);
CREATE TABLE pii_patterns_user (
    name TEXT PRIMARY KEY,
    pattern TEXT NOT NULL,
    enabled INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL
);
```

regex 검증은 `add_user_pii_pattern`에서 사전 `Regex::new(pattern)?` — 잘못된 정규식 저장 차단.

### Decision Log 필터 패턴

```js
const statusFilter = document.getElementById('dl-filter-status').value;
const sourceFilter = document.getElementById('dl-filter-source').value;
const sortOrder = document.getElementById('dl-filter-sort').value;
// 200건 fetch → 클라이언트 필터
```

5K+ 항목 시: `setup_decision_log_list` API에 status/source 파라미터 추가 + SQL WHERE 절.

### clippy fix 절차

1. `cargo clippy --workspace --lib --fix --allow-dirty --allow-staged`
2. `cargo test --lib --workspace` → 회귀 확인
3. 잔존 warning grep으로 수동 처리 후보 식별
4. CI에 `cargo clippy --workspace --lib -- -D warnings` 추가 (단계 별로 임계값 상향)

## 결과

- workspace lib 331건 유지 (회귀 없음, 신규 테스트 0건)
- clippy warning 71 → 18 (74% 감소)
- Tauri commands 67 → 72 (c1_thresholds_list/c1_threshold_set + pii_patterns_list/add/remove)
- MCP tools 27 → 32 (동일)
- settings.db 신규 테이블 2종 (c1_rule_thresholds + pii_patterns_user)
- Tauri GUI check + workspace check + lib regression 통과, 경고 0

## 후속

- C2 PII 패턴 GUI add/remove UI (Settings 탭에 PII 섹션) — API만 노출 상태
- C1 임계값 GUI 슬라이더 (Settings 탭) — API만 노출 상태
- clippy warning 18건 수동 처리
- CI 통합 (clippy + test build + workspace lib)
