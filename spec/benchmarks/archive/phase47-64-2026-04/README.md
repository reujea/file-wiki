# Benchmark Archive — Phase 47~64 (2026-04 측정)

## 아카이빙 (Phase 98, 2026-05-26)

Phase 98 C-5에서 112건 일괄 이관. 신규 측정 baseline 비교 명확화 목적.

### 내용
- micro_profile_50_*: 50문서 마이크로 벤치 (per-doc p95 측정)
- scale_100/500/1000/2000_*: 스케일 벤치
- threshold_070/080_*: similarity_threshold 0.7 vs 0.8 비교 (Phase 59 트리거 #1)
- phase52_final_100/2000: Phase 52 최적화 마무리 측정

### 측정 기간
- 2026-04-21 ~ 2026-04-30 (Phase 47~64)

### 주요 결과 (spec/architecture.md 본문 + spec/lesson-learned/05_perf-optimization-summary.md 참조)
- Phase 42~48 누적 289s → 125s (2.3x, 2000문서)
- per-doc 60ms 도달
- p95 2,075ms → 69ms (30x)
- 분산 (p95/p50) 45x → 1.6x

### 관련 lesson
- 01_crossref-full-scan / 03_refresh-mmap-batch / 04_parallel-cache-bias / 05_perf-optimization-summary / 06_hnsw-rebuild-batch / 08_flushed-embeddings-dup

### 단일 진실원 (메타 룰 19)
"왜 이 측정이 진행됐는가" + "결과 종합"은 spec/architecture-archive.md 단일 진실원. 본 디렉토리는 raw JSON 보존만.
