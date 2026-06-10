---
number: 11
topic: stub-skip-db-registration
date: 2026-04-24
---

# StubDuplicateResolution Skip → DB 등록률 60% (Phase 54)

## 상황
실제 문서 20개 벤치마크에서 18개 성공(OK)이지만 12개만 DB 등록. 8개가 "어딘가에서" 스킵됨.

## 문제
`StubDuplicateResolution`이 `DuplicateAction::Skip`을 무조건 반환.
- GUI/watcher(비대화형)에서 `build_service`가 이 Stub을 사용
- 의미 중복이 감지되면(cosine > 0.97) 즉시 `return Ok(())` → DB 미등록
- 성공이지만 등록 안 됨 → 벤치에서 "OK"지만 DB에 없는 현상

## 원인
- `StubDuplicateResolution`은 본래 "항상 스킵"이 아니라 "사용자에게 물어볼 수 없는 환경의 기본 동작"이어야 했음
- 비대화형에서 Skip을 기본으로 설정한 것은 보수적이지만, 실사용에서 정상 문서가 누락되는 치명적 문제
- CLI 터미널 모드에서만 `TerminalDuplicateResolution`(사용자 선택)이 사용됨

## 추가 발견
- DOCX/catalina.out 실패는 별도 원인: Windows 명령줄 32KB 제한 → `cmd.arg(prompt)` 초과
- `ClaudeCliAdapter`를 stdin 파이프(`claude -p -`)로 전환하여 해결

## 개선
1. `StubDuplicateResolution`: `Skip` → `Keep` (둘 다 유지)
2. `ClaudeCliAdapter`: `cmd.arg(prompt)` → stdin 파이프 (`Stdio::piped()`)
3. 전처리기: 인코딩 자동 감지 (`chardetng` + `encoding_rs`)
4. `classify_and_process_text`: 임시 파일 확장자 `.txt` 강제

## 결과
- DB 등록률: 60% → **100%** (12/20 → 20/20)
- DOCX: 0% → **100%** (0/2 → 2/2)
- 교차참조: 22 → **60** 관계
- stub 처리량: 23.6 → **16.6 docs/s** (Keep으로 전체 파이프라인 완주, 정상 하락)

## 교훈
- Stub/기본값은 "안전한 기본"이 아니라 "비용 없는 기본"으로 설계해야 한다
- Skip이 안전해 보이지만 실사용에서 데이터 손실과 동일한 효과
- 벤치마크에서 "성공 건수"와 "DB 등록 건수"를 별도로 추적해야 숨은 스킵을 발견할 수 있다
