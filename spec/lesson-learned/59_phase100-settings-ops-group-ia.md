# Lesson Learned: Phase 100 Settings IA 변경 — 5 운영 카드를 좌측 "운영" 그룹으로 이관

## 상황

2026-05-26 Phase 100. 사용자 GUI 사용 직후 즉시 피드백: **"설정 메뉴 컨텐츠 상단의 자동 추천/임계값 등 5 카드가 보기 불편 — 왼쪽 네비로 이관"**. 직전 Phase 99 종결 후 D:\file-test\에 GUI 배포 + 사용자 실행 → 첫 UX 피드백 도달.

5 운영 카드 (Decision Log / C1 임계값 / PII 패턴 / PII mask / MCP 카탈로그)는 Phase 82~93에서 누적 등록되며 settings-toolbar 직후 위치에 평면 배치. 좌측 settings-nav 4그룹(크레덴셜/일반/이벤트 훅/마이그레이션)과 분리되어 있었음.

## 문제

### 문제 1 — 5 운영 카드의 IA 위치 부적절 (사용자 직접 피드백)

settings-section 5개가 settings-layout 위에 배치 → Settings 탭 진입 시 5 카드가 항상 가시. 좌측 네비 그룹과 분리된 평면 누적. 메타 룰 7 (사용자에게 묻는 질문은 답할 수 있는 것만) — 사용자 피드백이 명확한 영역.

### 문제 2 — 메타 룰 22 후보 (사용자 정책 합의) 3번째 누적 도달

사용자 피드백에 대한 응답 방식:
- 옵션 A: 단일 "운영" 그룹으로 묶기
- 옵션 B: 5개 개별 그룹 분리
- 옵션 C: 2그룹 (자동화 + 보안)

→ claude가 임의 결정하지 않고 AskUserQuestion으로 합의 후 진행. **메타 룰 22 후보 3번째 누적 사례** (Phase 92 외부 협업 / Phase 94 헥사고날 / 본 Phase 100 IA).

### 문제 3 — 단일 진실원(settings-ops-cards) 이동 패턴 신규

settings-section 5개를 settings-content 안으로 직접 이동하면 settingsScrollTo의 hidden 토글이 5 카드를 다른 그룹에서도 노출시킬 가능성 (메타 룰 1 다중 위치 동기화 위험). 해소:
- index.html에 `#settings-ops-cards` 단일 진실원 컨테이너 신설
- "운영" 그룹 콘텐츠에 `#settings-ops-cards-mount` 빈 div
- dashboard.js `_mountOpsCards()` 메서드가 단일 진실원을 mount 위치로 이동 (DOM appendChild)

→ 메타 룰 19 (단일 진실원 위임) 자기 적용. 5 카드는 한 군데에만 존재.

## 원인

### 직접 원인
1. (문제 1) 5 카드 등록 시점(Phase 82~93)에 좌측 네비 그룹 합류 결정 미수행. 누적 배치만 진행
2. (문제 2) 옵션 결정이 사용자 워크플로 영향 직접 — claude 임의 결정 회피 패턴 (lesson 50 RBAC 보류 결정 패턴 응용)
3. (문제 3) settings-section 클래스가 "그룹과 별개의 토글 단위"로도 작동 → 직접 이동 시 hidden 정책 충돌. 컨테이너 분리로 충돌 회피

### 구조적 원인
- 카드 단위 누적 배치가 IA 그룹 구조와 동기화 안 됨 — 메타 룰 1 1f sub-rule (같은 의미 함수/카드 다중 정의) 변형
- "운영" 같은 의미 카테고리는 코드 등록 시 명확하지 않아 사후 분류 필요 (lesson 25 사용자 입력 vs 코퍼스 신호 패턴 변형)
- DOM 이동 (appendChild) 패턴은 단일 진실원 + 다중 mount 위치 시 표준 — 본 phase 첫 사용

## 개선

### 즉시 적용 (본 Phase 100 완료)

- ✅ 사용자 명시 합의: "단일 그룹 운영" 옵션 (AskUserQuestion)
- ✅ index.html — 5 settings-section을 `#settings-ops-cards` 단일 컨테이너로 묶음. 기본 `display:none` (mount 전 비가시)
- ✅ index.html — settings-layout 안 settings-content는 그대로 유지
- ✅ dashboard.js — groups 배열에 `['sys-ops', '운영', '자동 추천 · C1 임계값 · PII · MCP 카탈로그', []]` 추가
- ✅ dashboard.js — 운영 그룹 콘텐츠로 `<div id="settings-ops-cards-mount"></div>` 신설
- ✅ dashboard.js — `_mountOpsCards()` 메서드 신설 — 단일 진실원 DOM 이동
- ✅ form.innerHTML 직후 _mountOpsCards() 호출 (초기 렌더 시점 1회)
- ✅ dead_selector_scan v1/v2 통과 (ID 93 → 94, +1 신규 mount)
- ✅ Tauri release 빌드 통과 (10m 45s incremental, 21.05MB +512 bytes)
- ✅ D:\file-test\pipeline.exe 재배포 + SHA-256 일치 검증 + `.last-release` 마커 갱신
- ✅ release_rebuild_required.sh PASS (마커 갱신 후)

### 사용자 워크플로 (재배포 후)

```
[1] D:\file-test\pipeline.exe 더블클릭 (이전 PID 25732 종료 후)
[2] Settings 탭 진입 → 좌측 네비 5그룹 표시
    크레덴셜 관리 / 일반 / 운영 / 이벤트 훅 / 마이그레이션
[3] "운영" 클릭 → 5 카드(자동 추천 / C1 임계값 / PII 패턴 / PII mask / MCP 카탈로그) 표시
[4] 다른 그룹 클릭 → 운영 카드 자동 숨김 (부모 settings-group hidden 토글)
```

### 메타 룰 강화 (다음 phase 자기 적용)

- [ ] **메타 룰 22 후보 정식 승격 검토** — Phase 100 3번째 누적 도달. 메타 룰 23 §승격 3요소 충족 가능 (3건 + 체크리스트 + META 등재)
- [ ] **신규 카드 등록 시 IA 그룹 합류 의무** — Phase 82~93 같은 누적 배치 회귀 방지. 카드 PR 작성 시 좌측 네비 그룹 결정 동반 의무
- [ ] **DOM 이동 패턴 docs화** — `_mountOpsCards()` 같은 단일 진실원 + 다중 mount 위치 패턴은 webapp-design.md에 공통 UI 패턴으로 추가 후보

## 다음 세션 플래그

- 사용자 GUI 첫 실행 후속 피드백 도달 시 즉시 처리 (사용자 UX 신호 우선)
- 메타 룰 22 후보 3건 누적 → 메타 룰 23 §승격 3요소 평가
- C1 자동 추천이 발화하는 시점(검색 100회+) 도달 시 실 사용자 검증
- audit_trace 50건+ 도달 시 lesson 46 G-1 재검증

## 회귀 기준선

| 지표 | Phase 99 | Phase 100 | 차이 |
|------|---------|---------|------|
| Settings 그룹 수 | 4 (크레덴셜 / 일반 / 이벤트 훅 / 마이그레이션) | **5** (+ 운영) | +1 |
| 운영 카드 위치 | settings-toolbar 직후 (평면) | **settings-content 안 운영 그룹** | IA 정리 |
| dead_selector_scan ID | 92 | **94** (+ settings-ops-cards / settings-ops-cards-mount) | +2 |
| Tauri release | 21.03 MB | **21.05 MB** (+512 bytes) | UI 텍스트 변동 |
| D:\file-test\pipeline.exe | Phase 97 시점 | **Phase 100 시점** (재배포) | 갱신 |
| 사용자 명시 합의 누적 | 2건 | **3건** (메타 룰 22 후보) | +1 |
| 메타 룰 19 자기 적용 | 6건 (Phase 96 domain-map) | **7건** (Phase 100 settings-ops-cards 단일 진실원) | +1 |

## 사이드 발견

- D:\file-test\pipeline.exe가 실행 중일 때 재배포 — `Device or resource busy` 에러. taskkill로 종료 후 5초 대기 필요 (Windows 파일 핸들 락 지연)
- 사용자 GUI 종료 후 즉시 재실행 안내 의무 (taskkill로 강제 종료된 상태)
