# Lesson 28: 기능 제거 시 검증 테스트 잔존 (lesson 13/19 변형)

## 상황

`e2e_embedded::e2e_lint_with_stale` (modals/cli/tests/e2e_embedded.rs:282)가 stale 문서 감지를 검증. 2026-05-14 실 코퍼스 측정 직전 통합 테스트 실행에서 단독 실패:

```
thread 'e2e_lint_with_stale' panicked at modals\cli\tests\e2e_embedded.rs:306:5:
stale 문서 감지: []
```

추적 결과 `Linter::lint` (crates/core/src/domain/lint.rs)에 **stale 검사 코드 자체가 없음**. 코멘트 라인 162에 "test_lint_stale_detection 제거됨: lint stale 검사는 Phase 55에서 삭제됨 (lint_stale_days 설정 + Linter::stale 분기 제거)"가 명시. 그런데 테스트는 stale_docs가 비어있지 않음을 단언.

## 문제

- `Linter::lint`는 orphan + missing-backlink + duplicate-topic만 검사. stale은 Phase 55에서 의도적 제거.
- `e2e_lint_with_stale` 테스트는 stale_docs 비-empty 단언 유지 → **반드시 실패하는 테스트**
- 테스트 단위 모듈(`test_lint_stale_detection`)은 제거됐지만 통합 테스트는 누락
- `cargo test --workspace --lib`만 돌리면 통과 → 통합 테스트 실행 시점에야 발견

## 원인

1. **lesson 13/19 변형**: UI/기능 제거 시 코드/JS/Tauri commands는 8단계 체크리스트로 정리하지만, **통합 테스트는 체크리스트에 빠짐**.
2. **Phase 55 시점 누락**: stale 검사 제거 시 `e2e_lint_with_stale` 테스트가 `e2e_lint_with_orphan`로 rename되거나 stale 단언 제거되어야 했음. 둘 다 누락.
3. **테스트 격리 부재**: 이 테스트가 lib 단위 테스트였으면 함께 제거됐을 것. 통합 테스트 디렉토리는 별도 라이프사이클로 관리되어 깜빡함.
4. **CI 부재**: `cargo build --tests --workspace` 또는 nextest 정기 실행 부재로 한참 동안 잠복.

## 개선

### 즉시 처리 (2026-05-14 본 세션 트리거)
- **단기**: 테스트 `e2e_lint_with_stale` → `e2e_lint_with_orphan`로 rename + stale_docs 단언 제거. 또는 `#[ignore]` 처리하며 "Phase 55 stale 검사 제거됨" 주석 추가.
- **선택지**: 사용자가 stale 검사 복원 결정 시 `Linter::lint`에 stale 분기 재추가 (현재는 미복원 결정 — Phase 55 로드맵 기록 참조).

### 재발 방지 (lesson 13/19 확장)
UI/기능 제거 시 10단계 체크리스트에 추가:
- ~~기존: UI/JS/Tauri commands/struct/handler/help/CLI/MCP/spec/JS API 객체~~
- **신규 11단계**: **통합 테스트(`modals/cli/tests/`)에서 해당 기능 단언 grep 후 제거 또는 ignore**
- 검색 패턴: `grep -rln "{기능명}_docs\|{함수명}\|{필드명}" modals/*/tests/`

## 메타 패턴

본 lesson은 lesson 13/19와 같은 "기능 제거 시 잔존 자산" 패턴. lesson 14(미연결 포트)/26(이중 정의 동기화)와 함께 **"코드/사실의 다중 위치 동기화 누락" 메타 룰**의 일부:
- lesson 10: 컬럼 rename → 인덱스 미반영
- lesson 13: UI 제거 → JS dead code
- lesson 14: 포트 추가 → 호출처 누락
- lesson 19: UI 제거 → Tauri commands dead
- lesson 21: 필드 추가 → 테스트 초기화 누락
- lesson 26: DDL 추가 → open_in_memory() 누락
- lesson 27: 필드 추가 → 통합 테스트 초기화 누락 (lesson 21 재발)
- **lesson 28: 기능 제거 → 통합 테스트 잔존** (lesson 13/19 변형)

**META.md 인덱스 작성 시점 도달**: 8건 누적은 메타 룰화하기 충분. 다음 세션 트리거.

## 후속

- [ ] Phase 55 시점에서 stale 검사 제거 결정의 명시적 spec 추적 — `prd/roadmap.md` Phase 55 섹션 확인 후 보강
- [x] `e2e_lint_with_stale` 테스트 수정 (2026-05-14 완료) — `e2e_lint_with_orphan`으로 rename, stale 단언 제거, orphan 단건 검증으로 단순화. e2e_embedded 21/21 통과
- [ ] 통합 테스트 보강 (CI에서 `cargo build --tests --workspace` 정기 실행)

## 해소 표시

- 2026-05-14: 단언 제거 + 함수 rename 완료. 본문 주석에 "Phase 55 stale 검사 제거됨, 본 테스트는 orphan 검증만 수행" 명시. e2e_embedded.rs:277~294.

## 추적 (사전 결함)

본 lesson과 무관하지만 함께 발견된 통합 테스트 사전 결함:

- **`scale_validation::scale_work_queue_10k`** (2026-05-14 발견 + 진단)
  - `WorkQueue::scan_and_plan` + `mark_done` 10K 파일 시뮬레이션
  - FileProcessingService 미사용 (ServiceBuilder 마이그레이션과 무관)
  - 진단 결과 (2026-05-14 자율 세션, release 단독 실행):
    - 첫 스캔: 22.4s ✅ (한계 30s 통과)
    - 재스캔 (전체 mark_done 후): **39.9s** ❌ (한계 30s 초과 → assertion 실패)
    - 파일 생성: 25.7s, 큐 저장: 측정 미완 (failed before)
  - **원인**: NTFS 10K 파일 mark_done 후 재스캔이 30s 초과. assertion 기준이 환경(SSD/HDD/AV scanner)에 암묵적 의존. **lesson 84 패턴 변형**
  - **선택지** (사용자 결정 필요):
    1. assertion 한계 완화 (30s → 60s) — 회귀 검출 능력 절반 감소
    2. assertion 제거 + 측정만 eprintln — 회귀 추적을 시계열로 옮김 (lesson 04 "3회 중앙값" 정책과 일치)
    3. `#[ignore]` + 환경변수로 켜기 — 평소 실행 안 함
  - **자율 진행 보류**: 성능 게이트 완화 결정은 사용자 의도 필요

- **`scale_validation::scale_work_queue_100k`** — 100K 파일 첫 스캔 ≤ 120s 한계. 미측정 (100K 파일 생성에 4분+ 소요 예상)
