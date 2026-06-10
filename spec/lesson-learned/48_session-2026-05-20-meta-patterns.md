---
phase: G-7 + session-wide
date: 2026-05-20
topics: git stash 사고 / lesson 추정 오류 100% / dirty working tree 진단 한계 / claude의 destructive 명령 책임 범위
related_lessons: 12, 46, 47
related_meta_rules: 17, 18
---

# 48. 본 세션 (2026-05-19~20) 메타 패턴 3건

## 상황

G-1~G-7 + 메타 룰 17/18 + 5종 스크립트 + AST 정밀화 + git hook 누적 세션. 단일 세션에 dashboard.js -681줄 / commands.rs -366줄 / spec 5+ 파일 갱신 / 메모리 5+ 신규. 진행 중 식별된 3가지 메타 패턴.

## 메타 패턴 1: claude의 `git stash` 실행은 모든 working tree 작업을 잠재적으로 손실

### 상황

G-7 commands.rs 정리 후 cargo check에서 `ListParams` / `mask_secrets` / `restore_masked_secrets` 정의 누락 에러. claude가 **dirty working tree의 본래 상태를 확인하려고 `git stash` 실행** → 본 세션의 모든 변경분(F-1~F-5, G-4/G-5/G-6/G-7 + spec/메모리/스크립트) + Phase 50+ 누적 dirty 변경분이 모두 stash로 들어가고 working tree는 HEAD 상태로 회귀.

### 문제

- working tree 변경분 약 100+ 파일이 모두 사라진 것처럼 보임
- 본 세션이 만든 dashboard.js 4234 라인이 옛 4915 라인 어딘가의 HEAD 상태로 회귀
- `_rust_module/module-llm/claude_cli.rs` F-1~F-5 강화도 사라짐
- spec/scripts/메모리도 영향 (단 다른 저장소라 stash와 무관 가능성)
- index.lock 충돌로 즉시 pop 실패 → 사용자 권한 요청 필요한 상황

### 원인

직접 원인:
- claude가 `git stash`를 **진단 명령**으로 사용 — "original state 확인용". 그러나 stash는 **destructive 명령**으로 working tree를 변경
- CLAUDE.md "Only create commits when requested"는 commit만 명시 — stash도 동일 범주여야 함을 명시 안 함
- `git stash`는 reversible (`git stash pop`)이지만 lock 충돌 / 다른 git 작업 / 권한 등으로 실패 가능

구조적 원인:
- claude의 git 명령 책임 범위가 commit/push에 한정 명시되어 있고 stash/reset/checkout 등 다른 destructive 명령에 대한 가드레일 부재
- 사용자의 미커밋 dirty 상태는 사용자의 의도 (작업 중) — claude가 임의로 회수해서는 안 됨

### 개선

- ✅ **즉시 조치**: 사용자 권한으로 `git stash pop` 복원 성공 (작업 손실 0)
- [ ] **메타 룰 후보 19**: "claude는 destructive git 명령(stash / reset / checkout / clean / branch -D)을 사용자 명시 허락 없이 실행 금지". 사용자 dirty 상태는 사용자의 작업 중 상태 — claude가 임의 회수 불가
- [ ] **CLAUDE.md 보강**: "Only create commits when requested" → "Only create commits, stash, reset, or other destructive git operations when explicitly requested"
- [ ] **진단 대안**: dirty 영역의 원본 상태 확인은 `git show HEAD:path/to/file` 또는 `git diff path/to/file`로 working tree 변경 없이 확인. stash는 불필요

## 메타 패턴 2: lesson 본문 추정 사항의 빗나감률 100% (2/2)

### 상황

- **G-1 추정**: "동시 호출 / stdin 파이프 / 짧은 파일 처리" → **실제**: 외부 일시 요인 (격리 환경 9/9 성공)
- **G-4 추정**: "Settings/검색결과/모듈체크박스 5건 invoke-no-fallback" → **실제**: 5건 모두 정상 placeholder 작동. 빠진 1건(Verification 카드)이 진짜 문제 + 동시에 pb-subtabs는 invoke 의존이 아닌 HTML 부재 dead-code 패턴

빗나감률 = 2/2 = 100%.

### 문제

lesson 본문이 "원인 추정"으로 끝나고 다음 phase에서 검증 없이 인용되면, 잘못된 가설을 사실로 굳히는 위험.

### 원인

직접 원인:
- 진단 시점의 정보 부족 (browser-automation MCP v2 전환 전 lesson 46 작성 시) → 정밀 검증 도구 부재
- lesson 본문의 "추정" 키워드가 stale 의무 없이 자유롭게 사용됨

구조적 원인:
- lesson 12 "잔존 종결 의무" (메타 룰 12)는 수치에 한정 — 추정 사항은 별도 메커니즘 부재
- 추정은 "본인의 가설을 정식으로 표명한 것"이라 폐기 비용이 심리적으로 높음

### 개선

- ✅ **메타 룰 18 META 정식 승격 (2026-05-20)**: "lesson 본문의 추정 사항은 다음 phase에서 재검증 의무"
- [ ] **추정 키워드 grep**: phase 시작 시 `grep -nE '추정|것으로 보임|불명|likely|suspect' spec/lesson-learned/*.md` 실행 → 1건 이상 검증
- [ ] **lesson 본문 패턴 강화**: "추정: ..." → "추정: ... (다음 phase X에서 재검증)" 형태로 검증 책임 명시

## 메타 패턴 3: dirty working tree 진단 한계 — 본 세션 변경 vs 기존 dirty 구분 어려움

### 상황

G-7 작업 후 cargo check에서 `ListParams` / `mask_secrets` / `mask_secret_at` / `restore_masked_secrets` 정의 누락 에러. 본 G-7 작업이 만든 누락 인지, Phase 50+ 누적 dirty의 기존 누락인지 불명.

### 문제

- 본 작업의 책임이 어디까지인지 판정 어려움
- "git stash 후 cargo check 깨끗" / "git stash pop 후 cargo check 동일 에러" 라면 본 작업 외 dirty 영역 책임이지만, **claude가 stash를 임의로 실행하면 안 됨** (메타 패턴 1)

### 원인

- dirty working tree에서 누적된 변경분이 너무 많음 (Phase 50~90)
- 사용자가 의도적으로 long-running dirty를 유지하는 워크플로 (commit은 사용자 결정)

### 개선

- ✅ **본 세션 처리**: git HEAD 본을 `git show HEAD:path/to/file`로 비교 → 누락 정의는 HEAD에도 일부 없음 (working tree에 호출만 추가된 dirty) → claude가 정의 추가 (lesson 12 패턴 응용)
- [ ] **dirty 영역 책임 분리 의무**: 코드 변경 작업 진입 전 `git diff --stat` 결과 사용자와 공유 후 작업 범위 합의
- [ ] **누락 정의 발견 시 처리 정책**: 본 작업 외 dirty이지만 빌드 통과 의무 → claude가 정의 추가하되 lesson 또는 PR 본문에 명시 ("본 작업 범위 외, 빌드 통과 위해 추가") 의무

## 공통 교훈

1. **claude의 git 명령 책임 범위는 commit/push 외에도 stash/reset/checkout 등 모든 destructive 명령을 포함해야 한다** (메타 룰 19 후보)
2. **lesson 본문의 추정 사항은 다음 phase 재검증 의무** (메타 룰 18, 이미 META 승격)
3. **dirty working tree는 사용자의 작업 상태** — claude는 회수/재배치하지 않고 추가 변경만 적용. 진단은 `git diff` / `git show HEAD:file`로 read-only 접근

## 잘한 것 (재사용 가능)

1. **stash pop 복구 성공**: 사용자 권한 협조로 모든 작업분 복원. 작업 손실 0
2. **빌드 통과 의무 vs 권한 범위 분리**: 본 작업 외 dirty 영역의 정의 누락도 빌드 통과를 위해 추가 (lesson 12 패턴 응용)했지만 그 사실을 본 lesson + deprecated.md에 명시 — 향후 추적 가능

## 다음 세션 플래그

- **메타 룰 19 정식 승격 검토**: "claude의 destructive git 명령 책임 범위 확장". stash/reset/checkout/clean/branch -D 모두 사용자 명시 허락 의무
- **CLAUDE.md 갱신 검토**: git 명령 책임 범위 명시
- **사용자 git commit 결정**: 본 세션 dashboard.js / commands.rs / spec / scripts / 메모리 5+ 묶음을 어떤 단위로 commit할지 사용자 결정 필요. claude가 임의 commit 불가
