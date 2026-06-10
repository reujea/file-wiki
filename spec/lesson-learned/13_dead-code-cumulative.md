# 13. UI 기능 제거 시 dead code 누적

## 상황

Phase 55에서 Feedback 탭과 Lint 탭을 HTML에서 제거했음. 사용자에게는 기능이 사라진 것처럼 보였으나, JS 측에는 다음이 그대로 남아 있었음:

- `loadFeedbackPanel`, `_renderFeedbackPanel`, `_renderFeedbackDetail` (~250줄)
- `MOCK_FEEDBACKS` 참조 (정의는 이미 없어서 `undefined` 참조)
- `loadLint`, `renderLint`, `lintDelete`, `lintFixBacklink` (~50줄)
- `state.fbFilter`, `state.fbSearch`, `state.fbSelectedId`, `state.lintReport` 필드
- `switchTab`의 `tab === 'lint'` 분기
- 이벤트 위임의 `lint-delete`, `lint-fix-backlink` 액션

## 문제

- 코드 베이스가 ~300줄 부풀려짐 (3,245→2,938로 줄어든 것이 그 증거)
- `MOCK_FEEDBACKS` 참조처럼 **이미 깨진 상태로 살아있는 호출 경로**가 존재 (내부 호출이 없을 뿐 진입점 추가하면 즉시 런타임 에러)
- 신규 작업자가 코드를 읽을 때 "이 기능은 살아있나? 없어졌나?" 판단 부담

## 원인

UI 기능을 제거할 때 **HTML만 삭제**하고 그쳤다. JS의 호출 그래프는 진입점(탭/버튼)이 없어지면 외부에서 호출되지 않을 뿐, 코드 자체는 컴파일러가 dead로 인지하지 못함 (JS는 정적 분석 도구가 약함).

## 개선

기능 제거 시 다음을 **동시에** 수행:

1. HTML 마크업 (탭/버튼/컨테이너)
2. CSS 클래스 (`.fb-*`, `.lint-*` 등)
3. JS 진입 함수 (load*, render*, switchTab 분기)
4. JS 헬퍼/렌더러 (모든 보조 함수)
5. JS state 필드
6. JS 이벤트 핸들러 (data-action 분기)
7. API 정의 (call('feedback_*') 등)
8. (선택) Rust 백엔드 Tauri command 또는 명시적 "보류 중" 주석

체크리스트가 없으면 6개월 후 누적된다 (실측: Feedback은 Phase 55 → Phase 58까지 3 Phase 잔존).

## 재발 방지 (재사용 가능한 검증)

```bash
# JS dead code 후보 (정의되지 않은 식별자 참조)
grep -E "this\.(MOCK_|_fb|_lint|loadFeedback|loadLint)" src/ui/dashboard.js

# Rust dead code (cargo가 자동 감지)
cargo check --all  # warning 0건 기준선

# 미연결 포트 (다음 교훈 #14 참조)
```
