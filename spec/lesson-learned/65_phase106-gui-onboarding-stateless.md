# Lesson Learned: Phase 106 — GUI 온보딩 무상태 4-step + 빌드/배포 분리 자각

## 상황

2026-05-27~28 Phase 106. 사용자 트리거: "초기 실행 시 온보딩 기능을 추가하자. ① GUI 첫 실행 ② 크레덴셜 등록 팝업 ③ inbox 가이드 ④ 100개 업로드 후 optimize 가이드".

claude 초안: settings.db `onboarding_state` 테이블 신규 + 자동 트리거(첫 실행 감지) + 단계별 진행 상태 영속화. 사용자 수정: "별도 DB 없이 상단에 온보드 버튼 추가하고 온보드 클릭시 step별 입력 하게 하자". 무상태 모달 4-step + 사용자가 명시적으로 클릭할 때만 진행하는 방향으로 단순화. 추가 요청: "크레덴셜 등록 시 기본으로 설정하시겠습니까? 문의 추가".

## 문제

### 문제 1 — claude 초안 vs 사용자 결정 영역 분기

claude는 메타 룰 13(인프라 4단계) + 메타 룰 15(측정 위생)로 자동 감지·영속화·100/500/1000 마일스톤 단계별 트리거를 제안. 사용자는 단순한 명시 버튼 + 무상태 흐름을 선호. 메타 룰 22(사용자 정책 경계 명시 합의)가 작동 — claude 추천 옵션이 첫 위치였으나 사용자가 더 단순한 방향으로 결정.

### 문제 2 — 빌드 완료 ≠ 배포 완료 사이드 발견

1차 빌드(18:48) 후 "회귀 게이트 PASS" 보고로 종결 표시. 사용자 시점에서 헤더에 신규 버튼이 보이지 않음. 진단 결과: `D:\file-test\pipeline.exe`가 이전 14:36 binary (다른 세션 잔류). 빌드된 binary는 `src/modals/app/target/release/file-pipeline-tauri.exe`에만 존재. D:\file-test 재배포 누락.

이전 lesson 59/62에서 `taskkill /F /PID + cp + sha256sum` 패턴이 명시됐으나 본 phase에서 자동 재배포 단계 부재.

### 문제 3 — Modal 유틸 onCancel 옵션 부재

기존 `Modal.open(title, body, opts)`는 onSave 콜백만 지원. 취소·ESC·overlay 클릭 시 콜백 미호출. 멀티스텝 온보딩에서 "다음 진행"과 "중단" 의미 분리 어려움. 본 phase는 우회 — 취소를 "온보딩 중단"으로 해석하여 진행. 다른 모달 사용처에 영향 없는 안전 우회.

## 원인

### 직접 원인
1. (문제 1) sub-step 자동 측정·트리거 영속화는 메타 룰 13 4단계 완성도를 추구하는 claude 본능 — 사용자는 단순/직관 우선. 본질적으로 사용자 정책 경계 결정 영역(메타 룰 22)
2. (문제 2) `release_rebuild_required.sh`는 "재빌드 필요 여부"만 판정 — 빌드 후 D:\file-test 배포까지 자동 연결 없음. 사용자 자동 종료 + 재배포는 destructive 작업(GUI 강제 종료)이라 사전 합의 의무
3. (문제 3) Modal 유틸 단일 책임 설계 — 멀티스텝 흐름은 호출처에서 조립

### 구조적 원인
- comm-spec 영역과 사용자 GUI 영역의 책임 경계 — 사용자가 "단순"이라 표현하는 것은 종종 메타 룰 13 4단계의 "측정·자동화" 우선순위와 직교
- 빌드/배포/실행이 3 단계로 분리 — claude는 빌드 1단계만 자동, 배포 2단계는 사용자 합의, 실행 3단계는 사용자 영역. 1→2 자동 연결 안 됨

## 개선

### 즉시 적용 (본 Phase 106 완료)

#### 코드 변경 (UI 전용)
- ✅ `ui/index.html` 라인 16~18 — `data-action="open-onboarding"` 🧭 온보드 버튼 신규
- ✅ `ui/dashboard.js` — `startOnboarding` / `_showOnboardingStep` / `_advanceOnboarding` / `_onboardOpenCredForm` / `_onboardingAskSetDefault` 5 메서드 + dispatcher 2건 (`open-onboarding`, `onboard-open-cred-form`) + `showCredentialForm` onSave hook (`_onboardingResumeAfterCred` 플래그)
- ✅ 백엔드 변경 0 — Rust / Tauri commands / settings.db 영향 없음. cargo check 통과
- ✅ Tauri release 재빌드 2회 (18:48 1차 + 19:29 2차 = 기본 크레덴셜 설정 모달 추가분) — 마지막 22.09 MB
- ✅ D:\file-test\pipeline.exe 재배포 19:29 + SHA-256 검증 일치 (`0a968551...88d54b6`)
- ✅ 회귀 게이트: dead_selector_scan 94 ID PASS / release_rebuild PASS

#### 흐름
```
[헤더 🧭 온보드 클릭]
  ↓
1/4 환영 (4단계 안내) → "다음 →"
  ↓
2/4 크레덴셜 등록 → "크레덴셜 등록 폼 열기" (showCredentialForm 재사용)
  ↓ 저장 성공
[NEW] 기본 크레덴셜 설정 확인 모달 (3 분기: 신규 / 기존 다른 / 이미 본인)
  ↓ "기본으로 설정 + 다음 →"
3/4 inbox 안내 → "다음 →"
  ↓
4/4 100개 도달 후 optimize 가이드 (Settings 자동 추천 카드 안내) → "완료"
```

취소·ESC·X = 온보딩 중단. 다시 🧭 클릭 시 1부터 (무상태).

### 메타 룰 강화 후보

#### 메타 룰 17 강화 (release 재빌드 → D:\file-test 배포 자동 연결)
- 현재: `release_rebuild_required.sh`가 PASS/FAIL 판정만
- 강화 후보: 빌드 후 단계
  1. `tasklist`로 실행 중 `pipeline.exe` 감지
  2. 감지 시 사용자 확인 → `taskkill /F` + 5초 대기 + cp + sha256 검증
  3. 자동 연결 스크립트 `release_redeploy.sh` 신규 검토
- 누적: 본 phase 1건. 다음 release 빌드 phase에서 동일 패턴 발생 시 정식 후보 등록

#### 메타 룰 30 후보(Phase 105 등록) 자기 적용
- "phase 종결 시 spec 본문 즉시 갱신 의무"
- 본 phase는 트리거 받자마자 적용 — architecture.md / webapp-design.md / roadmap.md / external-trigger-checklist.md 4건 모두 본 세션 내 갱신
- 누적 사례 2건 도달 (Phase 105 발견 + 본 Phase 106 자기 적용). 정식 승격 1건 부족

#### Modal.open 시그니처 확장 검토 (보류)
- onCancel 옵션 추가는 다른 모달 사용처 모두 회귀 검사 필요 (lesson 19/47 패턴) → 본 phase 보류
- 멀티스텝 흐름 필요 시 호출처에서 자체 조립 (본 phase가 첫 사례)

## 다음 세션 플래그

- 메타 룰 30 후보 정식 승격 검토 (누적 2건 도달, 1건 추가 시)
- 메타 룰 17 강화 (release 재빌드 → D:\file-test 자동 배포 연결) — 누적 1건
- 본 phase 사이드 발견 — Modal.onCancel 추가가 의미 있어지는 시점(멀티스텝 모달 2건 이상 누적)에 검토
- 온보딩 실 사용 피드백 (E-1 본인 실사용 시작 시 자연스러운 진입점 검증)
- Step 4 "100개 도달 후 optimize" — 현재 안내 텍스트만. handle_optimize 결과 인라인 표시는 trigger 대기 (사용자 결정 영역)

## 회귀 기준선

| 지표 | Phase 105 | Phase 106 | 차이 |
|------|---------|---------|------|
| Tauri commands | 65 | 65 | 0 (백엔드 무변경) |
| settings.db 테이블 | 7 | 7 | 0 (별도 DB 미사용) |
| dead_selector_scan ID | 94 | 94 | 0 (신규 ID 없음, data-action 기반) |
| header 우측 버튼 | 1 (🤖 AI 설정 도우미) | **2** (+ 🧭 온보드) | +1 |
| Tauri release | 21.05 MB (Phase 100) | **22.09 MB** | +1.04 MB (incremental 빌드, UI 자원 임베드 증분 포함) |
| 코드 변경 | 0건 | UI 전용 (index.html +6줄 / dashboard.js +~135줄) | — |
| 회귀 게이트 | PASS | PASS | 동일 |

## 사이드 발견

- **빌드 완료 ≠ 배포 완료** 자각 (메타 룰 17 강화 후보 1건)
- **Modal 유틸 onCancel 부재** — 멀티스텝 흐름에서 우회 처리. 다음 멀티스텝 모달 작성 시 재발 가능 (lesson 47 패턴)
- **사용자 정책 경계 결정** 4번째 정도 누적 (Phase 92 외부 협업 / Phase 94 헥사고날 / Phase 100 Settings IA / Phase 103 GraphRAG 흡수 / 본 Phase 106 온보딩 무상태 = 5건째). 메타 룰 22 누적 사례 표 +1
- **메타 룰 30 후보 자기 적용 첫 사례** (Phase 105 발견 → Phase 106 즉시 적용). 누적 2건
- **추정 빗나감 0건** — 본 phase는 코드 변경 단순(메서드 추가만)이라 추정 영역 없음
