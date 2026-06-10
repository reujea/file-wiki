# [답변] file-pipeline 시나리오 기반 초기 설정 추천 시스템 고도화 방안

> 수신: 2026-05-07
> 원 질의: `prd/queries/initial-setup-advisor-experts.md`
> 후속 질문 3건 미해결 (Q1~Q3, 본 문서 말미)

---

## 요약

| 영역 | 현재 | 개선 방향 |
|------|------|-----------|
| 시나리오 분류 | 키워드 매칭 5종 (단일축) | 다축 프로파일 (5축, 복수 선택) |
| 추천 알고리즘 | if/else 하드코딩 | 선언적 룰 테이블 (TOML/JSON) |
| 적용 범위 | 9개 path만 `apply` 가능 | P0 항목 전체 → generic TOML writer |
| 검증 | `.bak` 롤백만 존재 | Snapshot 비교 + 자동 롤백 트리거 + dry-run |

**핵심 판단**: LLM 분류는 비용 대비 이점이 낮고, **구조화된 다축 폴링 + 가중 룰 테이블**이 로컬 데스크톱 도구의 특성에 가장 적합하다.

---

## 1. 시나리오 분류 고도화

### 1.1 문제 진단

- 키워드 매칭이라 "회의 코드 리뷰 결과" 같은 모호 입력에 약함
- `mixed`가 가장 흔한 케이스인데 추천이 비어있음
- `user_role` 필드를 받지만 반영하지 않음
- 보안 민감도/팀 규모/데이터 양 등 직교 차원 미고려

### 1.2 권장안: 다축 분류

단일축 시나리오 분류를 버리고, 직교하는 5개 축을 독립적으로 수집한다.

| 축 | 이름 | 값 | 비고 |
|----|------|----|------|
| 1 | `content_type` | meeting · research · code · legal · general | **복수 선택 + 비율** 허용 |
| 2 | `sensitivity` | low · medium · high · regulated | 민감 키워드/확장자 분포 기반 |
| 3 | `volume` | light (<50/week) · moderate (50~500) · heavy (500+) | 유입 빈도 |
| 4 | `search_intent` | precision · exploration · temporal | 주 검색 패턴 |
| 5 | `collaboration` | solo · small_team (2~5) · team (5+) | 협업 수준 |

### 1.3 입력 구조

```rust
pub struct SetupProfile {
    /// 자유 텍스트 (호환용, 축 값 추론 시도)
    pub description: Option<String>,
    /// 명시적 다축 입력
    pub content_mix: Vec<(ContentType, f32)>,  // [(meeting, 0.6), (code, 0.4)]
    pub sensitivity: Sensitivity,
    pub volume: Volume,
    pub search_intent: SearchIntent,
    pub collaboration: Collaboration,
    pub user_role: Option<String>,
}
```

- 자유 텍스트가 들어오면 기존 키워드 매칭을 **축별로 분리 적용**하여 `SetupProfile`로 변환
- "회의 코드 리뷰 결과" → `content_mix: [(meeting, 0.5), (code, 0.5)]`
- `mixed`라는 카테고리 자체가 사라지고, 비율 조합으로 표현됨

### 1.4 단기 vs 장기

| 단계 | 내용 |
|------|------|
| **단기** | 구조화된 다축 폴링 — MCP 호출 시 AI 에이전트가 대화로 축 값 수집 |
| **장기** | 사용 패턴 자동 프로파일링 — 실제 처리 파일의 `doc_type` 분포, 검색 mode 분포, sensitive 격리 비율 등을 50파일 처리 후 자동 산출하여 설정 불일치 알림 |

### 1.5 LLM 기반 분류는 비권장

| 이유 | 설명 |
|------|------|
| 빈도 대비 과도 | 초기 설정은 1회성~수회, LLM 호출의 지능이 과도 |
| 비결정적 | 동일 입력에 다른 결과 → 재현성 문제 |
| 오프라인 불가 | 로컬 데스크톱 앱의 특성과 충돌 |

단, 자유 텍스트 → 축 값 변환에 한해 **옵션으로** LLM 분류 제공 가능 (기본값은 키워드 룰).

---

## 2. 추천 알고리즘 고도화

### 2.1 문제 진단

- 추천 근거 부족 ("왜 2000 bytes?" → 정량적 근거 없음)
- `mixed = 변경 없음`으로 가장 흔한 케이스가 비어있음
- 설정 9개만 적용 가능, 나머지 ~60개는 추천해도 적용 불가

### 2.2 권장안: 선언적 룰 테이블

if/else 하드코딩을 **데이터 주도 룰 테이블**로 전환한다.

#### 룰 정의 예시

```toml
# rules/chunking_target_bytes.toml

[[rule]]
setting = "chunking.target_bytes"
axis = "content_type"
condition = "meeting >= 0.5"
recommend = 2000
reason = "회의록은 발화 단위가 길어 1500B 청크에서 문맥 절단 빈도가 높음. 2000B에서 ROUGE-L 보존율 약 12% 개선 [가정: 내부 벤치마크 필요]"
priority = "P0"
risk = "low"

[[rule]]
setting = "chunking.target_bytes"
axis = "content_type"
condition = "code >= 0.5"
recommend = 2500
reason = "코드 블록이 포함된 문서는 함수/클래스 단위 보존을 위해 큰 청크가 유리"
priority = "P0"
risk = "low"
```

#### 엔진 동작 로직

```rust
pub struct RecommendationEngine {
    rules: Vec<Rule>,  // TOML/JSON에서 로드
}

impl RecommendationEngine {
    pub fn evaluate(
        &self,
        profile: &SetupProfile,
        config: &PipelineConfig,
    ) -> Vec<ConfigChange> {
        self.rules.iter()
            .filter(|r| r.matches_profile(profile))  // 축 조건 매칭
            .filter(|r| r.needs_change(config))       // 현재 값과 비교
            .map(|r| ConfigChange {
                path: r.setting.clone(),
                current: config.get(&r.setting),
                recommended: r.recommend.clone(),
                reason: r.reason.clone(),
                priority: r.priority,
                risk: r.risk,
                axis_source: r.axis.clone(),
            })
            .collect()
    }
}
```

### 2.3 mixed 시나리오 처리

`mixed`라는 카테고리가 사라지고, `content_mix` 비율 조합으로 처리된다.

```
예: content_mix = [(meeting, 0.4), (code, 0.3), (research, 0.3)]

→ 각 축에서 독립적으로 룰 평가
→ 충돌 시 우선순위 해소:
  1. 비율 가중 — meeting이 60%면 meeting 룰 우선
  2. 보수적 선택 — 두 룰이 다른 값을 추천하면 기본값에 가까운 쪽
  3. 사용자 표시 — 충돌 시 양쪽 근거를 보여주고 선택 요청
```

### 2.4 추천 결과 표시 구조 (근거 포함)

```json
{
  "path": "chunking.target_bytes",
  "current": 1500,
  "recommended": 2000,
  "reason": "회의록 비중 60%: 발화 단위 문맥 보존을 위해 청크 확대 권장",
  "evidence": "heuristic",
  "confidence": "medium",
  "reversible": true,
  "restart_required": false
}
```

| `evidence` 값 | 의미 |
|----------------|------|
| `heuristic` | 경험 규칙 |
| `benchmark` | 내부 측정 |
| `literature` | 외부 근거 |
| `user_feedback` | 이전 사용자 피드백 |

### 2.5 단기 vs 장기

| 단계 | 내용 |
|------|------|
| **단기** | 현재 9개 하드코딩 룰을 TOML 테이블로 이관 + 근거 메시지 추가 |
| **장기** | 효과 피드백 루프 — 적용 → 50파일 측정 → 룰 신뢰도 갱신 (`evidence` 승격) |

---

## 3. 설정 항목 우선순위 매트릭스

### 3.1 분류 기준

| 등급 | 의미 |
|------|------|
| **P0** (Must-touch) | 시나리오에 따라 기본값이 부적합할 가능성이 높은 항목 |
| **P1** (Should-touch) | 튜닝하면 의미 있는 개선이 기대되는 항목 |
| **P2** (Don't-touch) | 변경 시 부작용 위험이 크거나 시나리오 무관한 항목 |

### 3.2 축: content_type별 매트릭스

| 설정 항목 | meeting | research | code | general | 위험도 |
|-----------|---------|----------|------|---------|--------|
| `chunking.target_bytes` | P0→2000 | P1→1500 | P0→2500 | — | low |
| `chunking.preserve_code_blocks` | P2 | P1→true | P0→true | — | low |
| `chunking.overlap_sentences` | P1→3 | P1→3 | P0→1 | — | low |
| `crossref.cap_related` | P0→30 | P0→30 | P1→15 | — | low |
| `crossref.similarity_threshold` | P1→0.75 | P0→0.85 | P1→0.80 | — | low |
| `crossref.supersedes_threshold` | — | P1→0.93 | — | — | low |
| `rerank.enabled` | P1→true | P0→true | P1→true | — | low |
| `rerank.top_n` | — | P1→30 | — | — | low |
| `verification.enabled` | P0→true | P0→true | P1 | — | low |
| `verification.thresholds.rouge_l_min` | P1→0.3 | P0→0.5 | — | — | med |
| `schedule.lint_interval_hours` | P0→6 | P1→12 | P1→12 | P1→24 | low |
| `search.mmr_lambda` | P1→0.6 | P1→0.7 | — | — | low |
| `search.window_lines` | P0→8 | P1→5 | P0→10 | — | low |
| `compression.original_ttl_days` | P1→90 | P0→0 | P1→180 | — | low |
| `preprocessing.pdf_tool` | — | P0→marker | P1→marker | — | med |
| `preprocessing.ocr_tool` | — | P1 | — | — | med |

### 3.3 축: sensitivity별 매트릭스

| 설정 항목 | high / regulated | medium | low | 위험도 |
|-----------|-----------------|--------|-----|--------|
| `sensitive.keywords` | P0→확장 | P1 | — | low |
| `sensitive.extensions` | P0→확장 | P1 | — | low |
| `compression.encrypt_sensitive` | P0→true | P1 | — | low |
| `remote_storage.enabled` | P2 (비활성 권장) | — | — | **high** |
| `logging.level` | P1→warn | — | — | low |

### 3.4 축: volume별 매트릭스

| 설정 항목 | heavy (500+) | moderate | light | 위험도 |
|-----------|-------------|----------|-------|--------|
| `max_workers` | P0→8 | P1→4 | P1→2 | med |
| `crossref.minhash_force_enable` | P0→true | — | — | low |
| `notification_batch.summary_interval_secs` | P1→60 | — | — | low |
| `memory_tier.hot_days` | P1→3 | — | P1→14 | low |
| `retention.enabled` | P0→true | P1 | — | **high** |
| `vector_db.rrf_multiplier` | P1→2 | — | — | low |

### 3.5 P2 고정 목록 (시나리오 무관, 추천 제외)

```
paths.*                    — 사용자 환경 고유
embedding.default_model    — fastembed 고정
vector_db.dim              — 1024 고정 (모델 종속)
vector_db.backend          — sqlite 고정
llm.provider               — 사용자 인증 정보
llm.*_api_key              — 보안 정보
credentials.*              — 보안 정보
hooks.*                    — 사용자 정의 이벤트
pipelines.steps            — 파이프라인 구조 변경은 고위험
```

### 3.6 위험도 태깅

```rust
pub enum RiskLevel {
    Low,       // 즉시 적용·롤백, 재시작 불필요
    Medium,    // 적용 후 재인덱싱/재처리 필요할 수 있음
    High,      // 데이터 삭제·이동 가능성, 되돌리기 어려움
    Critical,  // 잘못 적용 시 데이터 손실 (예: retention 활성화)
}
```

> `retention.enabled = true`는 **Critical**로 태깅하고, 추천 시 별도 경고 메시지를 포함해야 한다.

---

## 4. 검증 가능성 + 안전장치

### 4.1 효과 측정 지표

| 카테고리 | 지표 | 수집 위치 |
|----------|------|-----------|
| 가공 품질 | verify 1-Pass 성공률 | verify 노드 |
| 가공 품질 | quarantine 비율 | quarantine 노드 |
| 가공 품질 | ROUGE-L 평균 | verify 노드 |
| 가공 품질 | entity_preservation 평균 | verify 노드 |
| 검색 품질 | CRAG correct 비율 (top_score ≥ 0.8) | 검색 파이프라인 |
| 검색 품질 | CRAG ambiguous 비율 | 검색 파이프라인 |
| 검색 품질 | 평균 리랭크 스코어 | rerank 단계 |
| 처리 효율 | 파일당 평균 처리 시간 (ms) | compile_state |
| 처리 효율 | LLM 1-Pass vs 2-Pass 비율 | verify 노드 |
| 처리 효율 | semantic_dup 탐지율 | semantic_dup 노드 |
| 그래프 품질 | lint 경고 수 | lint 노드 |
| 그래프 품질 | 문서당 평균 crossref 링크 수 | crossref 노드 |

### 4.2 Snapshot 메커니즘

```rust
pub struct ConfigSnapshot {
    pub id: String,                        // uuid
    pub timestamp: DateTime,
    pub config_hash: String,               // pipeline.toml SHA256
    pub config_backup: Vec<u8>,            // .toml 원본
    pub scenario_profile: SetupProfile,
    pub metrics: Option<SnapshotMetrics>,   // 측정 후 채워짐
}

pub struct SnapshotMetrics {
    pub files_processed: usize,
    pub verify_pass_rate: f32,
    pub quarantine_rate: f32,
    pub rouge_l_avg: f32,
    pub crag_correct_rate: f32,
    pub avg_process_time_ms: u64,
    pub lint_warnings: usize,
}
```

- `compile_state` 노드(23번)에 metrics 수집 로직 추가
- 추천 적용 시 `snapshot_id` 기록 → 50파일 처리 후 자동 측정

### 4.3 자동 롤백 트리거

| 조건 | 동작 |
|------|------|
| `verify_pass_rate` 이전 대비 **15%p↓** | 알림 + 롤백 제안 |
| `quarantine_rate` **10% 초과** | 알림 + 롤백 제안 |
| `crag_correct_rate` 이전 대비 **20%p↓** | 알림 + 롤백 제안 |
| `avg_process_time_ms` **2배↑** | 알림 + 롤백 제안 |
| 파이프라인 크래시 (panic/OOM) | **자동 롤백** |

### 4.4 A/B 비교: dry-run 모드

로컬 단일 인스턴스에서 엄밀한 A/B 테스트는 불가능하므로, **시간 분할 비교 + dry-run**으로 근사한다.

```rust
pub struct DryRunResult {
    pub snapshot_id: String,
    pub config_used: String,      // "current" 또는 "recommended"
    pub files: Vec<DryRunFileResult>,
}

pub struct DryRunFileResult {
    pub file_hash: String,
    pub verify_passed: bool,
    pub rouge_l: f32,
    pub entity_preservation: f32,
    pub chunks_count: usize,
    pub process_time_ms: u64,
}
```

**MCP 도구 추가 제안**:

```
setup_dryrun {
  config_a: "current",
  config_b: "recommended",
  sample_count: 20
}
→ 실제 저장/색인 없이 verify + embed 단계까지만 실행
→ 처리 시간 + ROUGE-L + entity_preservation 비교
```

### 4.5 단기 vs 장기

| 단계 | 내용 |
|------|------|
| **단기** | Snapshot 저장 + Before/After 비교 (50파일 기준) + 자동 롤백 트리거 |
| **장기** | dry-run 모드 MCP 도구 + 룰 신뢰도 자동 갱신 (evidence 승격) |

---

## 5. 알려진 한계 #7 대응: apply 범위 확장

현재 `setup_apply`가 9개 path만 지원하는 문제의 단계적 해소:

| 단계 | 범위 | 방법 |
|------|------|------|
| 1단계 | 9 → 20개 | P0 항목 전체 커버 |
| 2단계 | 20 → ~45개 | generic TOML writer (`toml_edit` 크레이트, 주석 보존) |
| 3단계 | ~45개 | P2 제외 전체 지원 |

---

## 6. 구현 로드맵

| 시점 | 작업 | 산출물 |
|------|------|--------|
| **즉시** | 룰 테이블 TOML 스키마 확정 + 기존 9개 룰 이관 | `rules/*.toml` |
| **1주** | `SetupProfile` 다축 입력 구조 + 자유텍스트 → 프로파일 변환 | `setup_review` 개선 |
| **2주** | `setup_apply` path 지원 범위 P0 전체 확장 | generic TOML writer |
| **4주** | Snapshot 비교 + metrics 수집 (`compile_state` 통합) | `ConfigSnapshot` |
| **장기** | dry-run 모드 + 사용 패턴 자동 프로파일링 | `setup_dryrun` MCP 도구 |

---

## 후속 결정 필요 사항

| # | 질문 | 영향 범위 |
|---|------|-----------|
| Q1 | 다축 프로파일을 AI 에이전트가 대화로 수집 vs `setup_review`에 구조화 필드 직접 전달 — 주 사용 패턴에 맞는 쪽은? | MCP 도구 인터페이스 설계 |
| Q2 | 룰 테이블 추천 값에 대해 실제 벤치마크 수행 계획이 있는가? 있다면 벤치마크 데이터셋(회의록/논문/코드 각 50건) 확보가 선행 필요 | 룰 `evidence` 필드, 신뢰도 |
| Q3 | `retention.enabled` 같은 Critical 항목을 추천 범위에 포함할 것인가, 수동 설정만 허용할 것인가? | 룰 테이블 스코프, 위험도 정책 |
