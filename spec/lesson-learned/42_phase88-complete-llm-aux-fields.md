# Lesson 42 — Phase 88 완성: LLM 보조 필드 채움 + 직렬화 + fastembed 검증

## 상황

Phase 88 부분(W-1 + N-1)에 이어 N-2(prompts.toml `classify` 갱신) + fastembed 활성 측정 진행. Phase 87 인프라(Metadata.needs_verification / open_questions)가 LLM 가공 시 실제로 채워지고 영속화되는지 검증.

## 문제 / 발견

### 1. 인프라만 추가하면 끝이 아님 — 직렬화 계층 누락 발견

Phase 87에서 `Metadata.needs_verification` / `open_questions` 필드 추가 + `#[serde(default)]`로 호환. 그러나 실 측정에서 발견:
- LLM은 신규 프롬프트로 두 필드를 정상 생성
- 어댑터 `LlmResponse` → `Metadata` 변환은 N-2에서 추가
- **`.local-store.json` 직렬화 시점에 두 필드가 빠짐** — `StoredDoc` 구조에 미반영

`crates/adapters/src/driven/vector_db/local_store.rs::StoredDoc`가 별도 직렬화 구조라 `Metadata`의 모든 필드가 자동 전달되지 않음. 본 phase에서 발견·수정.

**메타 룰 (신규 후보)**: "인프라 추가 후 활성화 측정" 시 데이터 흐름의 모든 계층을 추적해야 함. 도메인 모델(`Metadata`) → 어댑터 응답(`LlmResponse`) → 저장 모델(`StoredDoc`) → 영속(`.json`/`.db`) → UI 노출. 각 경계마다 필드 매핑 확인.

### 2. lesson 1 메타 룰 1 "다중 위치 동기화 누락" 실증

`Metadata.needs_verification` 추가가 호환되려면 다음 4곳 모두 갱신 필요:
1. `core/domain/models.rs::Metadata` (Phase 87 완료)
2. `adapters/llm/response.rs::LlmResponse` + `build_classify_result` (Phase 88 N-2)
3. `adapters/vector_db/local_store.rs::StoredDoc` + upsert (Phase 88 본 phase)
4. `prompts.toml::classify` + 어댑터 fallback prompts.rs (Phase 88 N-2)

CLAUDE.md "구조체 필드 추가 = lib + 통합 테스트 동시 갱신" + lesson 21/27 패턴이 도메인 모델뿐 아니라 **저장 모델**에도 적용됨.

### 3. fastembed feature 빌드 시간 + 측정 차이

| 측정 | 환경 | 시간 | per-doc |
|------|------|------|---------|
| v1 (Phase 88 부분, 1차) | fastembed 비활성 | 392.8s | 49.1s |
| v2 (Phase 88 완성 중간, 2차) | fastembed 활성 (cold start) | 549.5s | 68.7s |
| v3 (Phase 88 완성, 3차) | fastembed 활성 (warm) | 358.8s | 44.9s |

**관찰**:
- v2가 가장 느림 — fastembed 첫 모델 로드(~80초) + 2-Pass 재가공 1건
- v3는 v1보다 -8.6% (모델 캐시 + 최적화)
- BGE-M3 1024차원이 polluted 메타데이터 빠르게 색인 (검색 정확도 측정은 별도 필요)

**메타 룰**: fastembed 측정은 **2회 이상** 실행해 cold start 영향 제거. lesson 04 "3회 중앙값"의 fastembed 변형.

### 4. LLM 보조 필드 품질 — 1-Pass 가공으로도 매우 구체적

10건 가공 결과 (8건 LLM 처리, 2건 PII 격리):
- needs_verification: **19건** (평균 1.9/doc)
- open_questions: **22건** (평균 2.2/doc)

예시:
- ✅ "tomcat10 'cannot be cast to class jakarta' 에러의 정확한 원인 — Servlet API 패키지(javax→jakarta) 호환성 이슈로 추정되나 원문에 상세 명시 없음"
- ✅ "log4j 2.15.0 적용으로 충분한지 — 이후 발견된 추가 취약점(2.16.0, 2.17.0 등) 대응 여부 확인 필요"
- ✅ "JDK 버전별로 nanoTime의 정밀도/구현이 어떻게 달라지는가?"

품질이 매우 높음 — wikidocs 353407 권고가 본 코퍼스에서 즉시 가치 발생.

**메타 룰**: 외부 권고를 LLM 프롬프트에 도입하면 1-Pass에서 즉시 가치 — 별도 학습/튜닝 없이 위키 운영 규약 자동 적용 가능.

### 5. doc_type 분류 변경 (구 → 신)

신규 프롬프트로 1차 가공된 doc_type이 변경됨:
- 1차: `technical_note + decision` (postgresql)
- 3차: `study + reference` (동일 파일)

doc_types.toml 등록 유형과 LLM 자율 판단의 차이. 본 phase 신규 프롬프트가 더 단순한 유형(study/guide/reference/log) 선택 — 추후 doc_types.toml 등록 유형 명시 강화 검토 가치.

## 개선 / 적용

### 코드 변경 요약

| 파일 | 변경 |
|------|------|
| `src/prompts.toml` | `[classify]` JSON 스키마 + 규칙에 `needs_verification` / `open_questions` 추가 |
| `crates/adapters/src/driven/llm/prompts.rs` | fallback 프롬프트 동일 동기화 |
| `crates/adapters/src/driven/llm/response.rs` | `LlmResponse` 두 필드 + `build_classify_result` 주입 |
| `crates/adapters/src/driven/vector_db/local_store.rs` | `StoredDoc` 두 필드 + upsert 매핑 (신규/업데이트 양쪽) |

### 회귀 기준선

- workspace lib **343** (Phase 87 340 + Phase 88 부분 +3 = 343 유지, 본 phase 구조체 확장만)
- workspace clippy `--all --tests` **0건** 유지
- workspace + Tauri `cargo check` ✅
- fastembed feature 빌드 시간: **11m 26s** (첫 빌드) / **2m 04s** (incremental)

### 측정 결과

`spec/benchmarks/llm_smoke_10_v3_20260518.json` 참조:
- 10건 처리 358.8초 (per-doc 44.9초, fastembed warm)
- verify pass 100% (8/8)
- needs_verification 19건 / open_questions 22건 채움
- PII 격리 2건 (FP 추정 보류 — 100건+ 표본 측정 후 결정) — **✅ Phase 89에서 36건 측정으로 FP 0% 해소** (`spec/benchmarks/c2_fp_phase89_20260518.json`). Phase 88 추정 ~20%는 DB 도구 문서 예제 코드 우연 매칭으로 확정. 메타 룰 18 6번째 재검증 사례 (✅ 추정 검증 성공)

### Phase 87 인프라 활성화 진행도 갱신

| 인프라 | 상태 |
|--------|------|
| Metadata.needs_verification/open_questions | ✅ **Phase 88 완성** — LLM 가공 + 직렬화 + 영속화 완료 |
| detect_strong_claims | ✅ Phase 88 부분 — lint 통합 완료 (단, schedule task 호출 미연결) |
| lint_weekly_hours/lint_monthly_hours | 필드만 (schedule task 분기 미연결 — N-3 후속) |

### Phase 88 잔여 → Phase 89

- **N-3**: lint 다층 주기를 service.rs schedule task에 분기 연결
- **N-4**: needs_verification/open_questions + lint_strong_claims 결과 UI 노출 (Settings/Documents 탭)
- **C2-fp 측정**: 100건+ 표본으로 PII FP 비율 정확 측정 (Q1 보류분)
- **A1-hit 측정**: 동일 파일 재가공으로 hit 측정

### lesson 14 완전 해소

`detect_strong_claims` 호출처:
- Phase 87: 0건
- Phase 88 부분: 단위 테스트 3건
- 본 phase: **실 코퍼스 8건 가공본에 적용 검증** (Python 사본) → 9건 검출

`needs_verification` / `open_questions` 호출처:
- Phase 87: 0건 (필드만)
- 본 phase: **LLM 가공 8건에서 41건 채움** + .local-store.json 영속화 검증

**lesson 14 dead 자산 패턴**: 인프라 추가 → 활성화 → 측정의 3단계가 본 phase에서 한 번에 완성된 첫 사례.
