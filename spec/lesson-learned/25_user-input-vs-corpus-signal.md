# Lesson 25: 사용자 입력보다 코퍼스 신호 — 추천 시스템 설계 4패턴

> Phase 80 — 5축 사용자 입력의 한계를 인식하고 코퍼스 신호 + 동작 모듈로 재설계.

## 상황

- Phase 76에서 다축 SetupProfile(content_mix 비율 등)을 사용자가 입력하는 룰 기반 추천 도입.
- 실 사용 후 발견: 사용자가 자신의 doc_type 비율을 정확히 모름. "회의록 25%, 코드 25%, 연구 25%, 일반 25%" 같은 균등 입력은 모든 룰의 50% 임계 미달 → 추천 0건.
- 더 본질적으로 같은 사용자가 코드도 회의록도 다 다루므로 "비율"이 의미 없음.
- 룰 추론 단계 자체가 잘못된 추상화 — 사용자가 원하는 건 도메인 자기 분류가 아니라 "어떤 동작을 우선?"이라는 의도 표현.

## 문제

1. **사용자 입력 정확도 부재**: 비율은 LLM이 처리 후 산출하는 것이지 사용자가 입력하는 것이 아님.
2. **임계 기반 룰의 멀티 도메인 약점**: min_ratio=0.5는 단일 도메인 우세를 가정. 멀티 도메인이면 침묵.
3. **추천 근거 불명확**: 사용자에게 "왜 이게 추천되지?"가 항상 따라옴. 룰 매칭 로직이 사용자에게 보이지 않으면 신뢰 약함.
4. **검증 불가**: 추천이 좋았는지 평가하려면 효과 측정이 필요 — 코퍼스 신호 없이는 사후 검증 불가.

## 원인

- 추천 시스템 설계 시 "사용자 자기 분류"를 입력 신호로 가정. 하지만 사용자는 자기 분류 능력이 떨어짐.
- 룰 엔진의 추론 단계가 사용자에게 블랙박스. 신뢰가 깨짐.
- 코퍼스 신호(stats/CRAG/lint/검색 mode 분포)가 이미 있는데 활용 안 됨.

## 개선 (4패턴)

### 1. 사용자에게 "도메인"이 아니라 "원하는 동작"을 묻는다

5축(content_mix/sensitivity/volume/search_intent/collaboration) → 12개 동작 모듈로 교체:
- 가공: 민감 강화 / PDF·OCR / 큰 청크 / 작은 청크 / 엄격 검증
- 검색: 정밀 검색 / 탐색 검색 / 최근 우선 / 풍부한 관계
- 운영: 고성능 / 장기 보존 / 자동 정합성

사용자는 자기 의도를 직접 선택. 시스템이 추론하지 않음.

```rust
pub struct Module {
    pub id: String,
    pub group: String,           // process | search | ops
    pub label: String,
    pub hint: String,
    pub exclusive_group: Option<String>,  // chunk_large / chunk_small 같은 배타
    pub changes: Vec<ModuleChange>,
}
```

### 2. 룰 엔진을 폐기하고 합집합 + 보수적 충돌 해소로 단순화

선택된 모듈의 모든 ConfigChange를 합집합. 같은 path 충돌 시:
- boolean: true 우선 (활성화 우선)
- 숫자: max (큰 청크/긴 보존/큰 cap)
- array: 합집합 (sensitive.extensions)
- string: 사전 정의 우선순위 (marker > pymupdf4llm > pandoc > none)

룰 가중치/우선순위/evidence 같은 메타 추론은 모듈 자체에 1:1로 박힘. 추천 결과가 결정적.

### 3. 코퍼스 신호 카운터를 영속화

settings.db에 신규 테이블 3개:
- `search_mode_counters` — search 호출 시 mode++
- `crag_counters` — top_score 산출 시 correct/ambiguous/incorrect++
- `chunk_stats` — 향후 청크 측정용

McpState에 메모리 카운터 + 서버 시작 시 `restore_counters()`. 매 요청마다 메모리 즉시 + DB 동시 영속화.

```rust
fn record_search_mode(&self, mode: &str) {
    if let Ok(mut m) = self.search_mode_counts.lock() {
        *m.entry(mode.to_string()).or_insert(0) += 1;
    }
    if let Ok(db) = SettingsDb::open(&self.settings_db_path) {
        let _ = db.increment_search_mode(mode);
    }
}
```

→ AI가 패턴 분석 시 이 신호로 추천 모듈 산출. 사용자에게는 "당신이 'recent' mode를 30% 쓰니 search_recent 모듈을 추천드려요"처럼 보여줌.

### 4. 진입점을 "사용자가 답할 수 있는 것"으로 재설계

3분기 진입점:
- ⚡ 일반 설정으로 시작 — 첫 사용자는 default. 변경 없음.
- 🤖 AI에게 분석 요청 — 50파일+ 처리 후 코퍼스 신호 기반 추천.
- 🧩 직접 동작 모듈 선택 — 12개 체크박스로 직접 의도 표현.

5축 폼은 (고급) 메뉴로 축소. 코드는 호환용 보존하되 진입 동선에서 제거.

## 재발 방지

- 추천 시스템 설계 시: "사용자가 정확히 답할 수 있는가?"를 먼저 묻는다. 도메인 자기 분류 같은 것은 보통 답할 수 없음.
- 임계 기반 룰을 도입할 때: 멀티 입력 시 어떻게 동작하는지 시뮬레이션. 0건이 나오는 경우의 fallback 정의.
- 추천 근거: 코드/룰이 아니라 사용자가 보낸 "코퍼스 신호"를 인용. "당신 데이터 보면 X% 패턴이라 Y 추천"이 이상적.
- legacy 보존: 신구 추천 시스템 전환 시 backend 코드를 즉시 폐기하지 않고 진입점만 숨김. 사용자 마이그레이션 시간 확보.
