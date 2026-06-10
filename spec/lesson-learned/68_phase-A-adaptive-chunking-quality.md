---
created: 2026-06-04
phase: A (Adaptive Chunking 4지표 측정 인프라)
external_source: arxiv 2603.25333 (Adaptive Chunking, LREC 2026)
prd_truth: prd/research/external-analysis-2026-06-04-adaptive-chunking.md
meta_rules:
  - 메타 룰 20 (외부 본질 도메인 일치 흡수) — 6건째
  - 메타 룰 13 (인프라 4단계) — 1단계
  - 메타 룰 9 (외부 출처 단일 진실원) — 자기 적용
---

# Lesson 68 — Adaptive Chunking 4지표 측정 인프라 흡수 (Phase A)

## 상황

사용자 요청 "https://discuss.pytorch.kr/t/adaptive-chunking-rag/10478 문서 분석하고 고도화 방안 정리". Adaptive Chunking 논문 (arxiv 2603.25333, LREC 2026 채택, ekimetrics/adaptive-chunking MIT 구현) 분석.

4 내재 지표 (SC/BI/ICC/DCC) + 1 영어 전용 (RC) 흡수 결정. 메타 룰 20 본질 도메인 일치 (file-pipeline 청킹 = Adaptive Chunking 청킹).

## 문제

본 솔루션 현재 청킹은 단일 전략 (`semantic` 고정). 문서별 최적 전략 선택 메커니즘 없음. SC(크기 준수) / BI(블록 무결성) 같은 품질 지표 미측정.

## 원인

- 청킹 품질을 측정할 수 있는 지표 자체가 없었음
- Phase 86 #8 트리거 `preserve_tables` 인프라가 있었으나 측정 기반이 부족
- 외부 흡수 시 본 솔루션 코드 매핑 없이 진입하면 도메인 누수 위험 (메타 룰 17)

## 개선

### Phase A 측정 인프라 (lesson 30 패턴 — 디폴트 비활성)

| 신규 | 위치 | 내용 |
|------|------|------|
| `ChunkQualityMetrics` | `core/domain/chunking_quality.rs` | sc/bi/icc/dcc (Option) + n_chunks |
| `compute_sc` | 동 | 100~1100 토큰 비율 |
| `extract_blocks` + `compute_bi` | 동 | 표/코드 펜스/헤딩 보존, τ=5자 |
| `compute_icc` | 동 | 청크 내 코사인 (임베딩 의존) |
| `compute_dcc` | 동 | 3000토큰 윈도우 코사인 |
| `Metadata.chunk_quality: Option<ChunkQualityMetrics>` | `core/domain/models.rs` | 4계층 자동 호환 (serde(default)) |
| `ChunkingConfig.compute_quality` | `shared/config.rs` | 디폴트 false |

### 4계층 동기화 (메타 룰 1 sub-rule 1e)
- 도메인 (Metadata) → 어댑터 응답 (자동, serde(default)) → StoredDoc (자동, Option) → 영속 (자동)
- test_helpers metadata_serde 라운드트립 갱신 (`chunk_quality: None`)

### 보류 (메타 룰 20 🔴 / 메타 룰 21 분류)
- **RC (Reference Completeness)** — Maverick coref 모델 영어 전용. 한국어 coref 도구 도달 시 재평가

### 검증
- 13 chunking_quality 단위 테스트 + 2 metadata_serde = 15/15 통과
- 원격 Linux 컴파일 incremental 1m 12s

## 메타 룰 적용

| 메타 룰 | 적용 |
|---------|------|
| 9 (외부 출처 단일 진실원) | `prd/research/external-analysis-2026-06-04-adaptive-chunking.md` 신규 |
| 13 (인프라 4단계) | 1단계(인프라) 완료. 2~4 (로직/측정/UI) 후속 |
| 20 (본질 일치 외부) | 6번째 누적 (JAMES + TFM + Mirage + GraphRAG + wikidocs + 본 건) |
| 30 후보 (spec 본문 즉시 갱신) | 본 세션 자기 적용 누적 +1 (정식 승격 조건 도달) |
