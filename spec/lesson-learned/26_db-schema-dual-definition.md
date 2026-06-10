# Lesson Learned: DB schema 이중 정의 동기화 누락

## 상황

Phase 81에서 settings.db에 `host_tools_cache` 테이블 추가. `SettingsDb::open()`의 `execute_batch("CREATE TABLE IF NOT EXISTS ...")` 블록에 정의 추가했고 빌드 통과, run-time 확인 완료. 단위 테스트 실행하니 `host_tools_cache::tests::*` 3건 동시 실패. 메시지는 `no such table: host_tools_cache`.

원인 추적 중 더 큰 문제 발견: `SettingsDb::open_in_memory()`(테스트 전용)에는 `host_tools_cache`뿐 아니라 **Phase 77/80에서 추가한 4개 테이블** (`config_snapshots`, `search_mode_counters`, `crag_counters`, `chunk_stats`)도 모두 누락. 이전 phase들이 in-memory 테스트를 거치지 않거나 우회했기 때문에 잠재 버그로 남아 있었음.

## 문제

1. `open_in_memory()`의 schema가 `open()`과 분기 — 실제 의도는 "동일한 DB의 메모리 버전"이지만 코드상으로는 **별도 DDL 블록 2개**.
2. Phase 77/80/81 모든 추가 테이블이 한쪽에만 반영됨. 신규 작업자/단위 테스트 시점에야 발견.
3. lesson 10(컬럼 rename 인덱스 불일치)의 재발 — **DDL 정의가 두 곳에 있을 때 동기화 누락** 동일 패턴.

## 원인

- **직접 원인**: `open()` 수정만으로 충분하다고 가정. in-memory schema 존재 자체를 의식 못함.
- **구조적 원인**: SQLite schema가 단일 진실 소스(single source of truth)로 정의되지 않음. `open()`과 `open_in_memory()`가 각각 `execute_batch` 인자에 raw SQL을 직접 적음 → 동기화 책임을 사람에게 떠넘김.
- **테스트 관성**: in-memory test가 신규 테이블을 사용 안 하면 (해당 phase의 단위 테스트가 in-memory 활용 안 함) 결함이 잠복.

## 개선

- DDL 통합 체크리스트:
  - [ ] settings.db에 새 테이블 추가 시 `open()` + `open_in_memory()` **양쪽에 동시 반영**.
  - [ ] 신규 테이블 사용 단위 테스트는 가능하면 `open_in_memory()`로 작성 — 양쪽 동기화 자동 검증.
  - [ ] 인덱스(`CREATE INDEX`)도 같은 규칙. `idx_xxx`만 한쪽에 있으면 쿼리 성능이 환경에 따라 달라짐.

- 구조 개선 후보 (트리거 대기):
  - DDL을 `const SCHEMA: &str = "..."`로 단일화 → `open()` / `open_in_memory()` 모두 같은 상수 참조
  - 또는 schema migration 시스템 도입 (sqlx/rusqlite migration crate). 현재 규모로는 과도하므로 단일 상수만으로 충분.

- 재발 추적:
  - lesson 10(컬럼 rename 인덱스 불일치)과 본질적으로 같은 "정의의 다중 위치 동기화 누락" 패턴.
  - 코드/UI/config의 동시 동기화 누락은 lesson 13(UI dead code) / 19(Tauri commands dead code)에서도 나타남 — **"같은 사실을 표현하는 코드가 여러 곳에 있으면 한 곳의 변경은 다른 곳에도 반영해야 한다"** 가 본 프로젝트 누적 교훈.

- 다음 세션 플래그: schema 단일화 작업(`SETTINGS_DB_SCHEMA` 상수 추출)이 가벼우면 다음 settings.db 변경 시 함께 도입 검토.

## 해소 (2026-05-14, Phase 82-prep)

`processing_metrics` 테이블 추가 작업과 함께 단일 상수화 완료. `open()` + `open_in_memory()` 양쪽이 `SETTINGS_DB_SCHEMA: &str` 동일 상수를 `execute_batch`에 전달. 이중 정의 자체가 사라져 lesson 10/26 재발 가능성 차단.
