---
updated: 2026-05-15 (Phase 84: PII 사용자 패턴 live reload — pii_user_patterns: RwLock<Vec> + reload_pii_patterns(), 재시작 불필요)
---

# 문서 유형 판단 + 구조 검증 Spec

## 0. 분류 이전 격리 (PII / 민감)

분류·가공은 LLM 호출(claude_cli)을 동반하므로 민감 데이터를 LLM에 전송하기 전 격리해야 한다. 흐름은:

```
read_to_string
  ├ 1. 경로/파일명/확장자 sensitive (SensitivityDetector::is_sensitive)
  ├ 1.3. 본문 PII regex (SensitivityDetector::scan_pii_in_text_with) — Ruflo C2
  └ 1.5. Fragment (짧은 메모) → LLM 스킵, 직접 색인
```

PII 패턴 (디폴트 5종, OnceLock 1회 컴파일):
- `ssn_kr` 주민번호 6-7자리
- `credit_card` 16자리 (4-4-4-4 또는 연속)
- `email` RFC 5322 단순 형식
- `phone_kr` 010 + +82
- `biz_reg_kr` 사업자번호 xxx-xx-xxxxx

사용자 정의 패턴은 `settings.db.pii_patterns_user(name, pattern, enabled, created_at)` 추가 가능. `add_user_pii_pattern`에서 `Regex::new` 사전 검증. 추가/제거 시 `service.reload_pii_patterns()`가 호출되어 **재시작 불필요, 다음 가공부터 즉시 반영** (Phase 84).

본문 PII 검출 시 `handle_sensitive`로 분기 → `sensitive/` 폴더 격리 + 알림. LLM 미호출.

## 유형 판단

- **17개 유형 검증 스키마** doc_types.toml 런타임 로드 (meeting, legal, guide, study, paper, log, todo, email, report, proposal, resume, receipt, memo, brainstorm, retrospective, specification, reference)
- **LLM 자율 판단** — 복수 유형 가능, 새 유형 생성 가능 (patterns/prompt 삭제됨, 섹션 힌트만 제공)
- **sections JSON** — LLM이 `{ "sections": { "결정사항": [...] } }` 형태로 반환
- **프롬프트에 검증 스키마 힌트** — DocTypeRegistry에서 id+label_ko+sections만 제공, LLM 자율 판단 우선
- **search_hints** — LLM이 "이 문서를 찾을 때 입력할 만한 질문/키워드 3~5개" 생성 → Metadata + Qdrant sparse vector 반영
- **code_blocks** — LLM이 코드블록을 `{language, description, code}` 구조로 추출

## 6가지 검증 + 보조 검증

| # | 검사 | 방법 | 기준 (default) | FAIL/WARN |
|---|------|------|---------------|-----------|
| 1 | 구조 완전성 | **sections JSON 키** 확인 (fallback: contains) | 50% | FAIL |
| 2 | 압축률 | 가공/원본 길이 비율 | 5~150% | WARNING |
| 3 | 키워드 커버리지 | LLM keywords → 원본 (환각 탐지) | 50% | FAIL |
| 4 | **키워드 완전성** | 원본 빈도 상위 15개 → 가공본 (누락 탐지) | 30% | WARNING |
| 5 | ROUGE-L | LCS 기반 | 10% | FAIL |
| 6 | 개체 보존 | **5패턴**: 날짜/금액/숫자/이메일/URL | 50% | WARNING |

### 보조 검증 (Phase 87, wikidocs 353407)

| 함수 | 역할 | 반환 |
|------|------|------|
| `detect_strong_claims(processed)` | 단정 표현(확실히/반드시/항상/100%/always/never 등) 검출 — 약화 권고 후보 | `Vec<String>` (사용자 검토용, 점수화 아님) |

Metadata 보조 필드 (LLM 가공 또는 lint가 채움):
- `needs_verification: Vec<String>` — 원천 자료 미확인 주장
- `open_questions: Vec<String>` — 원천 자료로 답할 수 없는 후속 질문

## 적응적 임계값

- doc_types.toml `[types.thresholds]`로 유형별 오버라이드
- 복수 유형 시 가장 엄격한 값 merge
- `VerificationThresholds::strict()` = 이전 엄격 기준

## 2-Pass 피드백 재가공

```
1차 가공 → 검증 FAIL
  → 실패 상세를 피드백으로 LLM에 전달
  → 2차 가공 → 재검증
  → 여전히 FAIL → quarantine/ 이동 + 알림
```

## 파이프라인 스텝별 오버라이드

파이프라인 에디터의 Verify 스텝에서 `VerificationThresholds`를 파이프라인별로 오버라이드 가능. 미지정 시 doc_types.toml 유형별 기준 → 전역 기준 순으로 fallback.

## 실환경 실측

| 기준 | DB 등록률 |
|------|----------|
| strict (90/85/25%) | 20% |
| default (50/50/10%) | 80% |
| **고도화 후 (sections + 양방향 + 2-Pass)** | **100%** |
| **신규 프롬프트 (노이즈제거+search_hints+code_blocks)** | **100% (1-Pass 통과, 구조100%, ROUGE 65.7%)** |
