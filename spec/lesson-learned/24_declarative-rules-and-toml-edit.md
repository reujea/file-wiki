# Lesson 24: 선언적 룰 테이블 + toml_edit 주석 보존 6패턴

> Phase 76 — 시나리오 기반 추천 엔진의 if/else 하드코딩을 데이터 주도 룰 테이블로 전환.

## 상황

- Phase 73의 `setup_review` 모듈은 if/else 하드코딩 9개 룰 + 단일축 시나리오 5종.
- 외부 전문가 답변(`prd/queries/initial-setup-advisor-experts.answer.md`)에 따라 다축 프로파일 + 선언적 룰 테이블 + toml_edit 기반 적용으로 전면 재설계.
- 동시에 ConfigChange 확장(priority/risk/evidence/confidence/conflict_note)과 Critical 차단 정책 도입.

## 문제

1. **if/else는 추천 근거 부재**: "왜 2000?"에 대해 코드가 답하지 않음. evidence 등급도 없음.
2. **mixed 카테고리가 빈 추천**: 가장 흔한 케이스인데 변경 0건.
3. **toml::to_string_pretty 재작성 시 주석 손실**: 사용자 편집 주석이 매번 사라짐.
4. **충돌 해소 정책 미정의**: 여러 룰이 같은 path에 다른 값을 추천할 때 무엇을 선택?
5. **Critical 항목 자동 적용 위험**: retention 활성화 같은 데이터 손실 트리거가 일반 적용 경로에 노출.

## 원인

- 추천 시스템을 코드로 인코딩하면 룰 추가가 코드 변경 → PR 리뷰 → 빌드. 룰은 데이터로 분리해야 빈번한 갱신 가능.
- TOML 직렬화는 값만 보존, 주석은 데이터가 아니라 decor(scaffold). `toml_edit`만이 decor를 보존.
- 다축 분류는 비율이 핵심. "meeting 70% + code 30%" 같은 표현이 있어야 실제 사용 패턴을 매핑 가능.

## 개선 (6패턴)

### 1. 룰 테이블은 별도 TOML + include_str! 임베드 + 빌드 타임 검증

```rust
pub const DEFAULT_RULES_TOML: &str = include_str!("setup_rules.toml");

// 빌드 타임 보증: from_toml(DEFAULT_RULES_TOML)이 panic하지 않아야 함
pub fn default_engine() -> Self {
    Self::from_toml(DEFAULT_RULES_TOML)
        .expect("DEFAULT_RULES_TOML 파싱 실패 — 빌드 타임에 검증되어야 함")
}
```

- TOML 파일이 잘못되면 단위 테스트(`test_default_rules_parse`)에서 즉시 실패.
- 사용자도 자신의 `rules.toml`을 추가해 `from_toml`로 로드 가능 (확장성).

### 2. 다축 프로파일 + content_mix 비율 — `mixed` 카테고리 폐기

```rust
pub struct SetupProfile {
    pub content_mix: Vec<(ContentType, f32)>,  // [(meeting, 0.7), (code, 0.3)]
    pub sensitivity: Sensitivity,
    pub volume: Volume,
    pub search_intent: SearchIntent,
    pub collaboration: Collaboration,
    // ...
}
```

- 단일축 5종(meeting/research/code/mixed/general) → 다축 비율로 표현.
- "혼합" 케이스는 자연스럽게 비율 조합으로 표현. 별도 카테고리 불필요.
- `infer_profile_from_text`로 호환 (자유 텍스트 → 다축 추론).

### 3. 충돌 해소: 비율 가중 → 보수 → conflict_note

```rust
fn resolve_conflicts(...) -> Vec<ResolvedChange> {
    // 1. 같은 recommend 값이면 P0 우선 1건
    // 2. 다른 값이면 매칭 룰의 content 비율로 가중 합산
    //    → 우세 그룹의 P0 룰 선택
    //    → 손실된 그룹은 conflict_note로 표시
}
```

- 사용자에게 "다른 추천이 있었지만 비율상 적합도가 낮아 제외"를 알려줌으로써 신뢰 확보.
- 보수적 선택(현재 값에 가까운 쪽)은 후속 phase에서 옵션화 예정.

### 4. toml_edit으로 주석 보존 — 단순 insert가 아니라 decor 복원 필요

```rust
// ❌ Naive: 기존 키의 prefix/suffix decor가 사라질 수 있음
tbl.insert(last, tv(item));

// ✅ 안전: 기존 Item::Value의 decor를 보존하고 값만 교체
if let Some(existing) = tbl.get_mut(last) {
    if let Item::Value(v) = existing {
        let prefix = v.decor().prefix().cloned();
        let suffix = v.decor().suffix().cloned();
        let mut new_v = item;
        if let Some(p) = prefix { new_v.decor_mut().set_prefix(p); }
        if let Some(s) = suffix { new_v.decor_mut().set_suffix(s); }
        *existing = Item::Value(new_v);
    }
}
```

- 테스트 `test_toml_edit_preserves_comments`로 회귀 방지.
- `# 주석`이 line prefix decor로 들어가는 경우가 많으므로 prefix 보존이 핵심.

### 5. Critical 등급 + 명시적 동의 토글

```rust
pub fn apply_advice_with_options(
    cfg_path: &Path,
    advice: &SetupAdvice,
    accepted_paths: &[String],
    apply_critical: bool,  // 명시적 false 기본값
) -> Result<Vec<String>>;
```

- `accepted_paths`에 Critical path가 있어도 `apply_critical=false`면 무시.
- UI에서는 별도 체크박스("Critical 적용 동의")로 두 단계 확인.
- 단위 테스트 `test_apply_blocks_critical_by_default`로 보호.

### 6. 적용 후 PipelineConfig 재파싱 검증 — 깨진 TOML 거부

```rust
let result = doc.to_string();
PipelineConfig::load_from_str(&result)
    .context("적용 후 설정 재파싱 실패 — 적용 거부됨")?;
std::fs::write(config_path, result)?;
```

- toml_edit으로 직접 수정하면 타입 검증 없이 통과 가능.
- 적용 결과를 `PipelineConfig::load_from_str`로 다시 deserialize하여 형 검증.
- 실패 시 파일 쓰기 거부 (.bak은 이미 생성됐으므로 안전).

## 재발 방지

- 룰 테이블 변경 시 단위 테스트 16개(default_rules_parse + 시나리오별 evaluate + apply_*) 자동 회귀.
- toml_edit 사용 시 항상 "주석 보존" 단위 테스트 동봉.
- Critical 등급 도입 시 "기본 차단 + 명시 동의" 패턴 고정.
- 추천 알고리즘 변경 시 `evidence` 필드를 강제. 근거 없는 추천은 룰에 들어갈 수 없음.

## 트리거

- 룰 테이블 항목이 100건 이상 누적되면: TOML 분리(예: `rules/content.toml`, `rules/sensitivity.toml`).
- benchmark 데이터(트리거 #3, 5K 코퍼스)가 확보되면: heuristic 룰을 benchmark로 evidence 승격.
- toml_edit으로 대량 변경 시: `tbl.entry()` API로 전환 검토.
