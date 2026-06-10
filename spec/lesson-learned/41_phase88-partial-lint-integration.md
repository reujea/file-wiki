# Lesson 41 — Phase 88(부분) lint 통합: Phase 87 인프라 호출처 연결

## 상황

Phase 87이 인프라 3건(Metadata 확장 / detect_strong_claims / lint 다층 주기)을 추가했으나 호출처 0건 (lesson 14 패턴). 외부 문서 분석 정형화(W-1) + N-1(lint 통합) 두 항목만 진행한 **부분 phase**.

남은 항목:
- N-2: prompts.toml `classify` 갱신 (Metadata 보조 필드를 LLM이 채우게)
- N-3: lint 다층 주기를 schedule task에 분기 연결

## 문제 / 발견

### 1. 외부 문서 분석 단일 진실원 — prd/research/external-analysis-2026-05-15.md

본 phase에서 외부 3 문서(supertonic / wikidocs 352523 / 353407) 분석 결과를 별도 결정 문서로 정형화. 다음 외부 분석 시점에 본 문서 인용 → 기존 결정 반복 금지.

**메타 룰 (신규 후보)**: 외부 문서 분석 → 결정 시 `prd/research/`에 단일 진실원 작성 의무. 같은 분석 반복 비용 차단.

### 2. lint 통합 — 새 메서드 추가 vs 기존 메서드 확장

`detect_strong_claims`를 기존 `Linter::lint`에 통합 vs 별도 `lint_strong_claims` 메서드 분리. **분리 채택** 이유:
- 본문 복원(storage) 비용이 큼 — 매 lint 호출에 모든 문서 압축 해제는 부담
- Phase 87 A-3 다층 주기와 맞물려야 함 — `lint_strong_claims`는 `lint_weekly_hours` 또는 산출물 전 검사에 적합
- 기존 `lint`(관계/유형/백링크 검사)는 가벼움 — 분리 유지가 호출 비용 자유

**메타 룰**: 검사 함수 추가 시 **호출 비용**으로 분리 결정. 가벼운 검사(O(N) 메타데이터) vs 무거운 검사(O(N × content_size))는 별도 메서드.

### 3. 테스트용 Storage stub — file-path 기반 매핑

`lint_strong_claims`는 `storage.decompress_temp(&doc.path)`로 본문 복원. 테스트에선 in-memory `StrongClaimStorage`가 `doc.path`(예: `doc_strong.zst`)의 stem(`doc_strong`)을 키로 사용해 미리 등록한 본문을 임시 파일로 쓴 후 경로 반환. 실 어댑터(ZstdStorageAdapter)와 동일한 인터페이스로 동작.

**메타 룰**: StoragePort stub은 file-stem 키 매핑이 가장 단순. 실 어댑터와 동일 path-style 입력 유지.

### 4. max_per_doc 상한 — 긴 문서 폭발 방지

긴 문서에 단정 표현이 50개 등장하면 LintIssue 50건이 보고됨 → 검토 비용 폭발. `max_per_doc` 파라미터로 문서당 최대 보고 수 제한. 디폴트 권장 5건 (호출자가 조정).

### 5. LintIssueType 확장 — Phase 88 새 변형

`LintIssueType::StrongClaim` 신규 enum 변형. 기존 4 변형(Orphan/Stale/MissingBacklink/DuplicateTopic/Contradiction) 호환 — Rust enum 추가는 non-exhaustive match가 아니므로 모든 match 처리 검토 필요. 본 phase는 enum 정의만 갱신 — 외부 match는 grep 검증.

## 개선 / 적용

### 코드 변경 요약

| 파일 | 변경 |
|------|------|
| `crates/core/src/domain/models.rs` | `LintIssueType::StrongClaim` 신규 변형 |
| `crates/core/src/domain/lint.rs` | `Linter::lint_strong_claims(vector_db, storage, max_per_doc)` 신규 + 단위 테스트 3건 (감지 / 빈 본문 / max 상한) |
| `prd/research/external-analysis-2026-05-15.md` | **신규 결정 문서** — 외부 3 문서 분석 단일 진실원 |

### 회귀 기준선

- workspace lib **343** 통과 (Phase 87 340 + 3 신규)
- workspace clippy `--all --tests` **0건** 유지
- workspace + Tauri `cargo check` ✅
- `LintIssueType` 새 변형 추가 — 외부 match 호환성 확인 완료

### 후속 (Phase 88 잔여)

- **N-2**: prompts.toml `classify` 프롬프트 갱신 → LLM이 `needs_verification` / `open_questions` 채움
- **N-3**: lint 다층 주기를 service.rs schedule task에 분기 연결 (`lint_weekly_hours` 도달 시 `lint_strong_claims` 호출 등)
- **N-4**: lint_strong_claims 결과 UI 노출 (Settings 또는 Documents 탭)

본 phase에서 호출처 0건 → 1건(테스트만) 변화. 실 서비스 호출은 N-3에서 연결. **lesson 14 부분 해소** (full 해소는 N-3 완료 시점).

### 외부 출처 인용 (lesson 40 메타 룰)

`Linter::lint_strong_claims` 주석에 `Phase 88 (wikidocs 353407 근거 점검)` 명시. 향후 권고 갱신 추적 가능.
