---
created: 2026-06-04
phase: B (ChunkingStrategy enum 추상화)
related: 68_phase-A-adaptive-chunking-quality.md (Phase A 직후)
meta_rules:
  - 메타 룰 1 sub-rule 1f (단일 진입점) — chunk_by_strategy
  - 메타 룰 13 (인프라 4단계) — 2단계 (로직 위임)
  - 메타 룰 30 (인프라 선구현)
---

# Lesson 69 — ChunkingStrategy enum 인프라 (Phase B)

## 상황

Phase A 4지표 측정 인프라 완료 후 Phase B 전략 추상화 진입. Adaptive Chunking 본체(Phase C) 진입 전 4 전략(Fixed/Semantic/Recursive/Adaptive)의 추상화 계층 필요.

## 문제

현재 `split_semantic` + `split_into_chunks` 두 함수가 분산. 호출처는 `chunked_agent.rs:117` + 테스트 11곳. 새로운 전략 추가 시 호출처 매번 분기 필요 → 메타 룰 1 sub-rule 1f (분산 함수 다중 정의) 누적 위험.

## 원인

- 청킹 전략이 enum 추상화 없이 함수 별 호출
- Phase A 4지표를 측정하더라도 "어느 전략을 선택할 것인가"는 별도 분기 필요
- Adaptive 본체 진입 전 enum 인프라가 없으면 호출처 회귀 위험

## 개선

### 신규 추상화

```rust
pub enum ChunkingStrategy { Fixed, Semantic, Recursive, Adaptive }

pub fn chunk_by_strategy(
    content: &str,
    strategy: ChunkingStrategy,
    config: &SemanticChunkConfig,
) -> Vec<SemanticChunk> { ... }
```

- `Fixed` → `chunk_fixed` (split_into_chunks 래핑)
- `Semantic` → `split_semantic` 위임 (호환)
- `Recursive` → `split_semantic` 위임 (Phase C에서 분리 검토)
- `Adaptive` → `split_semantic` 위임 (Phase C 본체 진입 전까지 fallback)

### 설정

```toml
[chunking]
strategy = "semantic"  # 디폴트 (호환)
```

### 6 단위 테스트
- strategy_from_str_known / unknown_falls_back / as_str_roundtrip / default / chunk_by_strategy_fixed / semantic 일치 / adaptive fallback

### 메타 룰 1 sub-rule 1f 자기 적용
- `chunk_by_strategy` 단일 진입점 추가로 향후 모든 청킹 호출 통일 가능
- 기존 `split_semantic` 직접 호출은 호환 유지 (점진 마이그레이션)

## 검증
- 컴파일 원격 incremental 2m 59s
- 누적 회귀 검증 37/37 통과 (Phase A 15 + Phase B 6 + 기존 chunking 16)

## 메타 룰 적용

| 메타 룰 | 적용 |
|---------|------|
| 1 sub-rule 1f | `chunk_by_strategy` 단일 진입점 신규 (분산 통일 시작) |
| 13 (4단계) | 2단계 (로직) 도달 — 3단계 측정 / 4단계 UI 후속 |
| 30 후보 | 본 세션 자기 적용 누적 |
