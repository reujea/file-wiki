# Lesson Learned: Phase 107 GUI 검증 묶음 + watch() WorkQueue 누락 발견

## 상황

2026-05-29 본 세션. 사용자가 dev seed credential 자동화 요청 → cargo run으로 GUI 띄워 검증 시도 → 검증 과정에서 추가 회귀 5건 연쇄 발견 → 모두 해소까지 진행한 묶음 phase. Phase 90+ GUI 검증 세션(lesson 46) 패턴 재현 — 사용자 실 사용 환경에서만 발견 가능한 회귀들.

## 문제

### 이슈 1: dev 모드 credential 0건 → 가공 즉시 실패

`cargo run`으로 dev 빌드 GUI를 띄우면 settings.db credential 0건 상태. service.rs build_service는 LLM credential 없으면 claude_cli 어댑터 만들 수 없음 → inbox에 파일 넣어도 가공 전체 차단. 매 dev 실행마다 사용자가 GUI에서 직접 credential 등록해야 테스트 가능.

### 이슈 2: qdrant/ 폴더 dead path 자동 생성

Phase 65에서 Qdrant 어댑터 제거 완료했지만 `ResolvedPaths.qdrant` 필드 + `create_all()` 자동 생성 + cli 인쇄 2곳이 잔존. `D:\file-test/qdrant/` 빈 폴더가 매 실행마다 생성되어 사용자 혼란. 메타 룰 1 sub-rule 1c 변형 (DB 스키마가 아닌 path 영역).

### 이슈 3: 실시간 watch()가 WorkQueue 미사용 → 대시보드 통계 0

`batch_process()`는 `WorkQueue::load + scan_and_plan + mark_processing/done/failed`로 큐 갱신하지만 실시간 `watch()` 함수는 **WorkQueue 미사용**. notify 이벤트 → spawn → process_file_with_pipeline 호출만 함. 결과적으로:
- inbox 파일 실시간 투입 시 `.work-queue.json`은 영원히 stale
- 상단 대시보드 "처리중/대기/실패" 카드 항상 0
- 메타 룰 1 sub-rule 1f 변형 (batch_process와 watch가 같은 의미인데 큐 갱신 정책 분산)

### 이슈 4: mark_processing의 silent no-op

`WorkQueue::mark_processing(path)`는 `items.get_mut(key)` 후 Some일 때만 갱신. 신규 파일은 items에 없으므로 **silent return**. watch() 흐름은 `scan_and_plan`을 거치지 않으므로 mark_processing 호출만으로는 큐가 빈 상태 유지. ensure_item 없이 mark_processing만 추가한 1차 수정은 효과 0.

### 이슈 5: ensure_item을 semaphore.acquire 후에 호출 → 대기 중 파일 미가시화

watch()에 ensure_item을 추가했지만 위치를 `let _permit = sem.acquire().await` **다음**에 둠. max_workers=4 환경에서 5+번째 이상 파일은 acquire 대기 중에 work-queue 미등록 → 사용자 보고 "8개 넣었는데 2개만 표시". semaphore 대기 시간 동안에는 대시보드에 가시화되어야 한다는 UX 요구사항을 추정으로 만족시키지 못함. 추정 빗나감 10번째 누적.

### 이슈 6: row 정렬에 status 1차 기준 사용 → 행 점프

dashboard.js `_renderProcTable` 정렬 1차 키가 status(처리중>대기>실패>완료), 2차가 created_at. status가 Pending → Processing → Done으로 빠르게 변하므로 5초마다 row 위치가 점프. 사용자가 진행 상황을 시각적으로 따라가기 어려움. 사용자는 "input 순서로 고정"을 명시 요청.

### 이슈 7: 로그 표시가 progress 이벤트 일부 필드만 → 빈약

`showProcessingLog`가 progress 이벤트의 `stage / types / pipeline / timestamp` 필드를 표시하려 시도했지만 실제 emit 형식은 `{event, file, stage?, types?, reason?}`. 결과적으로 거의 모든 라인이 빈 문자열 출력. claude_cli 호출/응답 등 상세 trace는 progress 채널에 없고 pipeline.log.{date} 파일에만 존재. 사용자 보고 "로그가 이상".

### 이슈 8: Processing + Verification 탭 분리로 비검증 영역 산재

Verification 탭에는 검증 메트릭 + 강한 주장 lint + audit anomaly 카드만 있었음. 사용자 관점에선 "파일 가공 흐름"과 "그 결과 검증"이 한 곳에서 보여야 자연스러운데 두 탭으로 분리되어 클릭 비용 발생. 사용자 명시 요청 "두 메뉴 통합".

## 원인

### 직접 원인

1. (이슈 1) dev/release 분기가 코드에 부재 — `cfg!(debug_assertions)` 분기 안 둠. test 환경 자동 설정이 영속 환경 오염을 피하려면 in-memory만 사용해야 한다는 정책 미결정
2. (이슈 2) Phase 65 Qdrant 제거 시 path 영역 정리 누락 — `ResolvedPaths.qdrant` 구조체 필드를 컴파일러가 dead로 잡지 못함 (Default impl 자동 초기화). lesson 14의 dead 자산 패턴 변형
3. (이슈 3) `batch_process`는 lesson 21/27 후 정밀화됐지만 `watch()`는 미수정. 두 함수가 같은 의미("inbox 파일을 큐 등록하고 처리")인데 큐 갱신 정책이 분산됨. 메타 룰 1 sub-rule 1f
4. (이슈 4) `mark_processing` 시그니처가 idempotent하지 않음 — items에 없으면 silent return. API 사용자가 "있으면 갱신 / 없으면 등록 후 갱신" 의도였을 가능성 있는데 코드는 강한 가정. 회피하려면 ensure_item 명시 호출 필요
5. (이슈 5) "spawn 직후 ensure_item이면 충분"으로 추정 — 실제론 acquire가 spawn 시점 이후이며 acquire 통과 전엔 가시화 안 됨. 사용자 실 사용에서만 발견. 추정 빗나감
6. (이슈 6) 정렬 키 우선순위 결정을 "처리 우선 순서"로 했음 — 사용자 멘탈 모델은 "투입 순서로 추적"이라 정렬 의도 불일치
7. (이슈 7) showProcessingLog 작성 시 progress 이벤트 형식 추정 — 실 emit 코드 grep 하지 않고 작성. pipeline.log를 별도 source로 활용할 생각 미진
8. (이슈 8) 탭 분리는 Phase 89에서 Verification 탭에 anomaly 카드 추가하며 굳어진 구조 — 사용자 멘탈 모델 점검 누락

### 구조적 원인

- **dev 환경 자동화 부재**: cargo run으로 GUI 실행 시 "테스트 환경 자동 설정"이 코드에 없음. 매 phase 빌드/실행 검증 시 사용자가 매번 같은 수동 단계 반복
- **메타 룰 1 sub-rule 1f가 path 영역까지 확장 필요**: `ResolvedPaths.qdrant` 같은 path 정의 vs 사용 분산도 sub-rule 1f에 포함. 자동화 도구 후보 (`grep "pub [a-z_]+: PathBuf"` + 사용처 0건 검증)
- **WorkQueue가 batch_process 종료 시점에만 save한다는 정책**: 진행률 실시간 가시화 요구를 만족 못함. "상태 전환 즉시 save" 정책으로 변경하면 5분 batch 도중 사용자가 진행 상황을 볼 수 있음
- **추정 빗나감 누적 10건**: 메타 룰 18 (사전 grep 의무)가 코드 호출 위치 검증에는 잘 적용되지만 **타이밍 위치(spawn 후/await 후)** 같은 흐름 추정에는 약함. 흐름 추정 시점에 "사용자가 어느 시점에 무엇을 보고 싶어하는가" 질문을 더해야 함
- **메뉴 IA가 도메인 분류 기준 — 사용자 작업 흐름 기준 아님**: Verification 탭은 "검증 도메인"이라 분리됐지만 사용자는 "한 파일의 처리 진행 상황"을 따라가고 그 결과 검증도 같은 화면에서 보고 싶어함. 도메인 분류 vs 작업 흐름 분류 트레이드오프 명시 메타 룰 후보

## 개선

### 즉시 적용 (본 Phase 107 완료)

- ✅ `crates/shared/src/lib.rs::dev_seed_credential()` 신규 (`cfg(debug_assertions)`) — in-memory만, settings.db INSERT 없음
- ✅ `modals/app/src/service.rs::init_app_state`에서 credentials 0건 시 seed 주입
- ✅ Claude 프로필 경로 결정: CLAUDE_PROFILE_PATH env → `C:\dev\ide\claude\profiles\reujea` 하드코딩 폴백 → `%USERPROFILE%\.claude`
- ✅ `dashboard.js::init()` credential 0건 시 `startOnboarding()` 자동 호출 (release 환경 첫 실행 케어)
- ✅ qdrant dead path 6곳 일괄 제거: PathsConfig.qdrant / ResolvedPaths.qdrant 정의 + 할당 + create_all + cli/main.rs print 2곳
- ✅ `watch()` 함수에 WorkQueue 통합 — queue_mutex + queue_path Arc + spawn 안 ensure_item + mark_processing + mark_done/failed + 즉시 save
- ✅ `WorkQueue::ensure_item(path)` helper 신규 — items 없으면 file metadata 자동 채워 Pending 등록 (idempotent)
- ✅ batch_process spawn도 ensure_item + 즉시 save 패턴으로 통일 (5분 batch 도중 진행률 가시화)
- ✅ ensure_item 호출을 `semaphore.acquire().await` **이전**으로 이동 — 대기 중 파일도 Pending으로 즉시 가시화
- ✅ frontend R-1 버그 수정: `qData.processing` → `qData.stats.processing` (get_queue 응답 구조 정합)
- ✅ R-2c progress 채널 활용: refreshDashboard에서 progress 이벤트 도착 시 200ms 후 `_refreshQueueOnly()` 추가 호출 (폴링 주기와 별개 즉시 갱신)
- ✅ row 정렬 단일 키화: created_at asc만 사용 (status 1차 정렬 제거 → 행 점프 차단)
- ✅ `commands.rs::get_file_log(file_name)` Tauri command 신규 — pipeline.log + pipeline.log.{date} 파일별 라인 추출. main.rs invoke_handler 등록 + frontend API.fileLog 매핑
- ✅ `showProcessingLog` 풍부화: 처리 이벤트(한글 라벨) + 큐 상태 + Pipeline Log raw 라인 3 섹션
- ✅ get_queue 응답에 created_at + size_bytes 필드 추가 (frontend 정렬용)
- ✅ index.html 탭 메뉴 7→6: verification 탭 제거 + processing 탭 라벨 "처리 현황" + verification 콘텐츠 흡수
- ✅ dashboard.js switchTab: processing 분기에 loadVerificationMetrics + loadAnomalyReport 흡수

### 메타 룰 강화

- **메타 룰 1 sub-rule 1f 누적 사례 추가**: ResolvedPaths.qdrant path 영역 + batch_process/watch 같은 의미 분산. 누적 8건 도달 (29/38/50-A/50-B/51/52/107a/107b)
- **메타 룰 18 강화 — 흐름 추정 시 사용자 시점 추가**: 코드 위치 grep만으로 부족. "이 코드가 실행되기 전/도중에 사용자가 무엇을 보고 싶어하는가" 질문을 사전 체크리스트에 추가
- **메타 룰 22 6건째 누적 (사용자 정책 경계)**: dev seed in-memory 결정 — settings.db 영속화 vs in-memory 트레이드오프를 사용자 명시 합의로 결정
- **메타 룰 17 강화 후보 진척**: lesson 65 사이드 발견(빌드 ≠ 배포)에 이어 lesson 66 사이드 (release 재빌드 보류 결정도 메타 룰 17 자기 적용 — 사용자 명시 합의로 다음 세션 보류)
- **메타 룰 31 후보 (도메인 분류 vs 작업 흐름 분류)**: 메뉴 IA 결정 시 두 기준 트레이드오프 명시. Verification 탭 흡수가 첫 사례

### 신규 작업 사전 체크리스트 추가

- [ ] **상태 전환 시각 즉시 가시화 요구 검증**: 새 상태 추적 함수 추가 시 "사용자가 이 상태를 언제 봐야 하는가" 질문. 5초+ 폴링 대기 무방 vs 즉시 가시화 명확히 분기
- [ ] **idempotent helper 패턴**: get_or_insert / ensure_X 형태로 silent no-op 방지
- [ ] **path 영역도 sub-rule 1f 적용**: `pub [a-z_]+: PathBuf` 필드 추가 시 사용처 0건 grep 의무
- [ ] **흐름 타이밍 추정 시 사용자 시점 질문**: spawn 후/await 후 위치 결정 시 "사용자가 이 시점에 어떤 가시화를 기대하는가"

## 다음 세션 플래그

- release 재빌드 의무 이행 (메타 룰 17) — 본 phase 종결 후 다음 세션 첫 작업
- 메타 룰 31 후보 정형화 (도메인 분류 vs 작업 흐름 IA 기준)
- dev seed hash 계산 시점 race (api-integration.md hash="" 사례) — 짧은 디바운스 후 재계산 또는 mark_done 시점에 재계산 옵션
- frontend 5초 폴링 → 이벤트 기반 push 전환 검토 (Tauri event emit 활용)

## 회귀 기준선

| 지표 | Phase 106 | Phase 107 |
|------|-----------|-----------|
| Tauri commands | 65 | **66** (+1 get_file_log) |
| Dashboard 탭 | 7 | **6** (verification 흡수) |
| ResolvedPaths 필드 | 11 | **10** (qdrant 제거) |
| WorkQueue 갱신 흐름 | batch만 | **batch + watch 양쪽** |
| WorkQueue 메서드 | 5 | **6** (ensure_item +1) |
| audit stage 종류 | 10+1 | **10+1** (변동 없음) |
| compile warnings | 0 | **0** (workspace + Tauri app) |
| lesson | 65 | **66** |
| 추정 빗나감 누적 | 9 | **10** (semaphore acquire 위치) |
| 메타 룰 1 sub-rule 1f 사례 | 6 | **8** (+ResolvedPaths.qdrant + batch/watch 분산) |
| 메타 룰 22 누적 | 5 | **6** (+dev seed in-memory) |
