---
created: 2026-06-16
updated: 2026-06-16
status: decision-pending
related:
  - spec/classification_and_verification.md
  - src/prompts.toml
  - src/crates/core/src/domain/verification.rs
  - src/doc_types.toml
---

# Classify 프롬프트 v2.1 확정안 — 본문·Fragment·메타데이터 spec

## 배경

`src/prompts.toml [classify].template`의 기존 프롬프트에 대한 2차 리뷰(2026-06-16)를 거쳐 v2.1로 확정. 핵심 변경 6건:

1. 유형→필수섹션 매핑을 LLM에 명시 (쟁점 1, `{type_hints}` 기존 인프라 활용)
2. 압축률 기준 이중 게이트 (권장 30%~150% / 강제 5%~150%)
3. ROUGE-L 검증 #5를 "보조 지표(게이트 아님)"로 강등
4. `date` null 허용 (LLM 환각 차단, 호출자 보정)
5. `keywords` 하한 5개로 완화 (짧은 문서 대응)
6. 호출자 컨텍스트·검증 결과 컬럼 슬림화 (토큰 절감 ~60%)

## Fact-check 결과 (코드 정합성)

### `{type_hints}` 구조 — `build_type_hints` (`prompts.rs:196-213`)

```
## 알려진 문서 유형 (검증 스키마)

- **meeting** (회의록): 섹션=[결정사항, 액션아이템, 다음안건]
- **legal** (법률문서): 섹션=[당사자, 핵심조항, 기한, 서명일]
... (17유형)

위 유형에 해당하지 않으면 적합한 id를 새로 만드세요.
```

→ v2.1의 §2 "필수 섹션 50% 충족" 전제 충족. 단 신규 유형 추가 시 sections 누락하면 `check_structure` (`verification.rs:80`)가 분모 0 → 항상 통과 처리 (검증 무력화).

### 검증 #번호 ↔ 코드 매핑 — `verify_with_thresholds` (`verification.rs:273-369`)

코드는 #번호를 모름. `VerificationThresholds`의 6개 필드(`structure_min`, `compression_min/max`, `keyword_coverage_min`, `keyword_completeness_min`, `rouge_l_min`, `entity_preservation_min`)로 이름 기반 매칭.

`VerificationThresholds::default()`:
- `structure_min: 0.5`
- `compression_min: 0.05` / `compression_max: 1.5`
- `keyword_coverage_min: 0.5`
- `keyword_completeness_min: 0.3`
- `rouge_l_min: 0.10` ⚠️ **여전히 FAIL로 처리** (`verification.rs:334-341`의 `has_fail = true`)
- `entity_preservation_min: 0.5`

→ v2.1 §6 "ROUGE-L 보조 지표" 표기는 정책 변경이며 코드 변경 동반 필요.

### doc_types.toml 유형별 임계값 오버라이드

- `meeting`: `structure_min=0.3, compression_max=2.0`
- `todo`: `structure_min=0.0, compression_max=3.0`

→ v2.1 §6 #1 "50% 이상"은 글로벌 디폴트. 유형별 오버라이드가 있으면 그쪽 우선.

### `date` 보정 — **호출자에 없음**

`service.rs`에서 `result.metadata.date`를 그대로 사용. `topic_merger.rs:394`, `mcp_server.rs:1985`는 빈 문자열일 때 표시만 생략. LLM이 임의 날짜 넣으면 그대로 영속화.

→ v2.1 §5 `date: string | null` 적용 시 `service.rs`에 mtime 보정 추가 필요.

### 재시도 상한 — **존재**

- `FileProcessingService.max_retry: u32` (`service.rs:94`)
- LLM 호출 실패 retry: `for attempt in 0..=self.max_retry` (`service.rs:362, 375`)
- 검증 FAIL → 2-Pass 재가공: `verify_reprocess_with_feedback` (`service.rs:483`)

→ 프롬프트에 명시 불필요 (호출자 책임).

---

## v2.1 최종 프롬프트

````
당신은 file-pipeline의 문서 가공 LLM입니다. 입력 파일을 분석해 **JSON 한 덩어리**로만 응답하세요. JSON 외 텍스트 금지.

────────────────────────────────────────
## 1. 작업 순서

1단계: **노이즈 제거** — 네비게이션, 광고, 중복 헤더/푸터, UI 잔여물("Edit this page", "Copy link" 등) 제거.
2단계: **유형 판단** — 아래 §2 알려진 유형에서 선택 (복수 가능). 적합한 유형이 없으면 새 id 생성.
3단계: **섹션 가공** — 선택한 유형의 **필수 섹션**을 우선 채움. 각 섹션이 독립적으로 이해 가능하도록 맥락 포함.
4단계: **메타데이터 추출** — §4 필드 채움.

────────────────────────────────────────
## 2. 알려진 문서 유형 (검증 스키마)

{type_hints}

**복합 유형 규칙** (doc_types에 2개 이상 나열한 경우):
- 첫 번째 유형을 **주 유형**으로 간주, 필수 섹션 모두 채움.
- 두 번째 이후는 **보조 유형**, 해당 유형의 필수 섹션 중 원문에 단서가 있는 것만 채움.
- 총 섹션 수가 8개를 넘으면 보조 유형의 섹션을 통합·생략 (압축률 상한 위반 방지).
- 보조 유형의 섹션을 하나도 채우지 못하면 `doc_types`에서 해당 보조 유형 제거 (메타데이터 정합).

────────────────────────────────────────
## 3. 출력 본문(content) 규약

### 형식
- **순수 본문만**: frontmatter / YAML / 메타 헤더 금지. 메타데이터는 JSON 필드에만.
- **섹션 구분자**: `=== {섹션명} ===` (등호 3개 + 공백 + 섹션명 + 공백 + 등호 3개).
- **섹션 순서**: 유형별 필수 섹션 → 보조 유형 섹션 → 원문에 있으나 스키마 외인 핵심 섹션.

### 필수 섹션 충족
- 주 유형의 필수 섹션 중 **50% 이상**을 `=== ... ===` 형태로 출력해야 검증 통과.
- 원문에 단서가 없는 섹션은 **생략** (빈 섹션 채우기 금지 — 환각 유발).
- `sections` JSON 필드(§4)에는 출력한 섹션만 키로 포함.

### 충실도
- **숫자, 날짜, 고유명사, 코드블록은 변형·삭제 금지** (원문 표기 그대로).
- **개체 보존 50% 이상**: 원문의 날짜·금액·숫자·이메일·URL 패턴 중 절반 이상이 본문에 잔존해야 함.
- **약어 표기**: 원문 표기를 우선. 첫 등장 시 풀이 병기 (예: 원문이 "Kubernetes"면 그대로, 원문이 "K8s"면 첫 등장 시 `K8s(Kubernetes)`).

### 압축률
- 가공/원본 길이 비율 **30%~150%** 권장 (RAG 충실도 게이트).
- 30% 미만은 정보 손실, 150% 초과는 환각·중복 위험.
- 원문이 매우 짧아 30% 미만이 불가피하면 본문에 핵심 정보를 보존하고 그대로 출력.
- **단, 가공/원본 5% 미만은 강제 실패** (정보 손실 과다). 30% 미만은 짧은 원문에 한해 허용.

### 키워드 정합
- `keywords` 필드의 각 단어는 **본문(content)에 실제 등장**해야 함 (검증 #3).
- 원본 빈도 상위 단어가 본문에 30% 이상 잔존해야 함 (검증 #4).

────────────────────────────────────────
## 4. 출력 JSON 스키마

```json
{
  "doc_types": ["meeting", "todo"],
  "rationale": "회의록이면서 할일 목록 포함",
  "date": "2026-04-05",
  "summary": "2~3문장 핵심 요약",
  "keywords": ["키워드1", "키워드2"],
  "search_hints": ["사용자가 이 문서를 찾을 때 입력할 만한 질문/키워드"],
  "sections": {
    "결정사항": ["MyDocSearch 통합 불필요", "Qdrant 유지 결정"],
    "액션아이템": ["이개발: .vec 구현", "박기획: 와이어프레임"]
  },
  "entities": [
    {"name": "김철수", "type": "person"},
    {"name": "마케팅팀", "type": "organization"},
    {"name": "Next.js", "type": "technology"},
    {"name": "500만원", "type": "amount"},
    {"name": "프로젝트 A", "type": "project"},
    {"name": "이벤트 소싱", "type": "concept"}
  ],
  "code_blocks": [
    {"language": "yaml", "description": "배포 설정 예시", "code": "apiVersion: v1\nkind: Pod"}
  ],
  "needs_verification": [
    "Redis 클러스터 자동 페일오버 설정 — 원문에 명시 없음, 운영 정책 확인 필요"
  ],
  "open_questions": [
    "롤백 시 트랜잭션 로그 보존 기간은?"
  ],
  "content": "=== meeting ===\n결정사항...\n=== todo ===\n[ ] 항목..."
}
```

────────────────────────────────────────
## 5. 필드 규칙

| 필드 | 타입 | 필수 | 규칙 |
|------|------|------|------|
| `doc_types` | string[] | ✅ | 1개 이상, **주 유형부터** 나열. snake_case. §2의 id 우선, 새 id는 snake_case 영문 |
| `rationale` | string | ✅ | 유형 판단 근거 1~2문장 |
| `date` | string \| null | ✅ | YYYY-MM-DD. **원문에서 추출 불가능하면 `null`** (오늘 날짜 임의 사용 금지 — 환각). 호출자가 보정 |
| `summary` | string | ✅ | 2~3문장 핵심 요약 |
| `keywords` | string[] | ✅ | **5~15개**. 원문에 실제 등장하는 단어. 고유 단어 부족 시 5개 미만도 허용하되 가능한 만큼 채움 |
| `search_hints` | string[] | ◯ | 3~5개. 사용자가 이 문서를 검색할 만한 질문/키워드 |
| `sections` | object<string, string[]> | ✅ | §3에 따라 **출력한 섹션만** 키로. 주 유형 필수 섹션 50% 이상 |
| `entities` | {name, type}[] | ◯ | 최대 20개. type ∈ {person, organization, place, technology, amount, project, concept} |
| `code_blocks` | {language, description, code}[] | ◯ | 원본 코드블록 구조화. 없으면 빈 배열 |
| `needs_verification` | string[] | ◯ | 0~3개. 원문 미명시/추가 확인 필요 사항 |
| `open_questions` | string[] | ◯ | 0~3개. 원문으로 답할 수 없는 후속 질문 |
| `content` | string | ✅ | §3 규약 본문 |

**금지**:
- JSON 외 텍스트 (설명, 코멘트, 코드펜스 ```json 펜스 등).
- `content`에 frontmatter / YAML / 메타 헤더.
- `keywords`에 원문 미등장 단어.
- `date`에 임의 날짜 (원문 미추출 시 `null`).
- 원문 단서 없는 섹션 채우기.

────────────────────────────────────────
## 6. 검증 통과 기준 (LLM이 충족해야 통과)

| # | 검사 | 통과 기준 |
|---|------|----------|
| 1 | 구조 완전성 | 주 유형 필수 섹션 50% 이상이 `=== ... ===`로 출력 |
| 2 | 압축률 | 가공/원본 30%~150% (권장), 5%~150% (강제) |
| 3 | 키워드 커버리지 | `keywords` 단어 50% 이상이 본문에 등장 |
| 4 | 키워드 완전성 | 원본 빈도 상위 15개 중 30% 이상이 본문에 등장 |
| 5 | ROUGE-L | 보조 지표 (게이트 아님). 적극적 재구성을 권장하므로 점수 낮음 정상 |
| 6 | 개체 보존 | 원문 날짜/금액/숫자/이메일/URL 패턴 중 50% 이상 잔존 |

> **게이트(FAIL)**: #1, #3. **경고(WARNING)**: #2, #4, #6. **보조(점수만 기록)**: #5.
> 코드 정합: `verification.rs::VerificationThresholds`의 6개 필드와 1:1 매핑.
> 유형별 임계값 오버라이드(`doc_types.toml [types.thresholds]`)가 있으면 글로벌 디폴트보다 우선. 예: `meeting.structure_min=0.3`, `todo.structure_min=0.0`.

────────────────────────────────────────
## 7. 입력

파일명: {filename}

{content}
````

---

## Fragment·Sensitive 분기 (LLM 미도달)

### Fragment (`service.rs:782-816`)

본문이 매우 짧고 LLM 가공 가치가 낮은 경우. LLM 호출 없이 헤더만 생성하여 색인.

```
=== META ===
source: {filename}
doc_types: fragment
date: {yyyy-mm-dd}
=== CONTENT ===
{원문}
```

- 키워드: 원문 공백 분리 후 2글자 이상 상위 10개
- date: 호출 시각 (`chrono::Local::now()`)
- 임베딩: 원문 그대로

### Sensitive (`service.rs::handle_sensitive`)

PII 검출 시 LLM 미호출, `sensitive/` 폴더 격리 + 알림. 사용자가 직접 메타데이터 채움 (`sensitive_notification.notify_and_collect`).

PII 패턴 디폴트 5종: `ssn_kr` / `credit_card` / `email` / `phone_kr` / `biz_reg_kr`. 사용자 정의 패턴은 `settings.db.pii_patterns_user`에서 live reload (Phase 84).

---

## 가공 산출물 spec

### 디스크 (1 가공 = 3+1 파일)

| 산출물 | 경로 | 포맷 | 생성 위치 |
|--------|------|------|-----------|
| 가공 본문 (압축) | `processed/{doc_type}_{stem}.txt.zst` | zstd 압축, 원문은 UTF-8 plain text | `service.rs:610-622` |
| 임베딩 벡터 | `processed/{doc_type}_{stem}.txt.vec` | `f32` little-endian raw bytes | `service.rs:628-629` |
| 원본 (압축) | `originals/{원본명}.zst` | zstd 압축 (원본 그대로) | `service.rs:624` |
| 메타데이터 | `.local-store.json` | JSON serde (`StoredDoc` + `StoredRelation` + `Entity`) | `local_store.rs:193` |

> **가공 본문 파일에는 메타데이터가 없음**. `service.rs:608` 주석: "가공본 저장 — 순수 본문만 (메타데이터는 벡터DB에만 저장)".

### 벡터 DB `StoredDoc` (`local_store.rs:193-214`)

```rust
struct StoredDoc {
    id: String,
    path: String,
    hash: String,
    doc_types: Vec<String>,
    date: String,
    keywords: Vec<String>,
    summary: String,
    vec_offset: usize,
    vec_dim: usize,
    needs_verification: Vec<String>,   // Phase 88
    open_questions: Vec<String>,        // Phase 88
}
```

> `Metadata` 도메인 모델에는 `rationale / sensitive / doi / related_docs / source_doc_ids / search_hints / entities / hierarchy / content_type / statements / chunk_quality`도 있으나 `StoredDoc` 영속 필드는 위 11개.

---

## 후속 코드 작업 5건 (프롬프트만 바꾸면 안 되는 항목)

| # | 작업 | 영향 파일 | 우선순위 |
|---|------|----------|--------|
| 1 | `LlmResponse.date: String` → `Option<String>` | `adapters/src/driven/llm/response.rs:17` | 높음 |
| 2 | 빈 date 처리 검증 (검색·정렬 영향 확인) | `mcp_server.rs:1985`, `topic_merger.rs:394` | 중간 |
| 3 | `service.rs`에서 빈 date 시 mtime 보정 | `core/src/service.rs:632` 이전 | 높음 |
| 4 | null/빈문자열/Unknown 3중 표현 매핑 규칙 단일 지점 고정 | `response.rs::build_classify_result` | 높음 |
| 5 | ROUGE-L FAIL → 경고/보조 강등 | `verification.rs:334-341` (`has_fail = true` 제거 또는 `has_info` 플래그 신설) | 중간 |

### 매핑 흐름 (작업 4)

```
null (LLM JSON)
  → None (LlmResponse.date: Option<String>)
  → "" (Metadata.date: String, build_classify_result에서 None → "")
  → DocDate::Unknown (DocDate::from_string)
  → mtime 보정 후 "YYYY-MM-DD" (service.rs)
```

### 작업 5 상세

현재 `verification.rs:334-341`:
```rust
if rouge_l < thresholds.rouge_l_min {
    has_fail = true;  // ← 제거 필요
    details.push(format!("FAIL: ROUGE-L recall {:.0}% (기준 {:.0}%)", ...));
}
```

→ v2.1 §6 "보조 지표" 정합을 위해 `has_fail` 제거 + 메시지 prefix "FAIL" → "INFO". 또는 별도 `has_info` 플래그 신설하여 메트릭만 기록.

---

## 메타 룰 후보 (META.md 승격 검토)

### 후보 1: doc_types.toml 신규 유형 추가 시 sections 정의 의무화

**Why**: `def.sections.is_empty()`인 유형은 `build_type_hints`에서 섹션 표시 없이 유형 이름만 노출 (`prompts.rs:202`). 검증 시에도 `check_structure`가 분모 0 → 항상 통과 (`verification.rs:80`). 즉 신규 유형 추가 시 sections 누락하면 LLM은 섹션 정보 없이 가공하고 검증은 무조건 통과 — 검증 게이트 자기 무력화.

**How to apply**: 신규 유형 추가 시 sections 1개 이상 정의. 코드 강제는 `doc_types.toml` 파싱 시 `sections.is_empty()` 경고 추가.

**관련 메타 룰**: 메타 룰 1 sub-rule (다중 위치 동기화) 확장.

---

## 미결 결정 (사용자 확인 필요)

### Q1: 적용 범위

- **(α)** v2.1 본문 채택 + 후속 코드 작업 5건을 한 phase로 묶어 실제 적용. 메타 룰 1 sub-rule 충족.
- **(β)** v2.1을 `src/prompts.toml [classify].template`에 핫 리로드 교체 (코드 변경 없이) + 2-Pass 통과율 회귀 측정. ⚠️ ROUGE-L FAIL 위험 (작업 5 미적용 시) → 2-Pass 호출 증가 가능.
- **(γ)** 본 결정 문서만 보존, 적용은 별도 세션.

### Q2: doc_types.toml sections 의무화 메타 룰 승격 여부

후보 1을 META.md에 정식 메타 룰로 등록할지.

### Q3: 글로벌 `structure_min` 디폴트 vs 유형별 오버라이드 정책

현재:
- 글로벌 `structure_min = 0.5` (v2.1 §6 "50% 이상" 권고와 일치)
- `meeting.structure_min = 0.3` / `todo.structure_min = 0.0` 오버라이드

선택지:
- (가) 글로벌을 0.3으로 낮춰 유형별 오버라이드 단순화
- (나) v2.1 §6 각주에 명시하고 현재 구조 유지 (코드 변경 0건)

---

## 단일 진실원

- 본 문서: `prd/research/classify-prompt-v2.1-decision.md` (v2.1 프롬프트 + Fragment/Sensitive 분기 + 산출물 spec + 후속 작업 5건)
- 검증 코드 진실원: `src/crates/core/src/domain/verification.rs::VerificationThresholds`
- 유형 스키마 진실원: `src/doc_types.toml`
- 분류 프롬프트 진실원: `src/prompts.toml [classify]` (v2.1 채택 시 본 문서로 교체 예정)
- 가공 흐름 진실원: `src/crates/core/src/service.rs` (`process_file_with_pipeline`)
