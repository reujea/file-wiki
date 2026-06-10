---
number: 5
topic: perf-optimization-summary
date: 2026-04-21
---

# 성능 최적화 종합 (Phase 42~48)

## 프로세스: 계측 → 실측 → 구현

### 1. 계측 없는 추정은 위험
- "flush가 85% 병목" 가설 → 실측 7%. **가설 기각**
- "289-47=242초가 flush" → 실측 16초. **차감 추정 불가**
- "양방향 link 미구현" 가설 → 코드 확인 결과 **이미 구현됨**
- 프로파일링 코드를 먼저 삽입하고, 실측 후 판단

### 2. 병목은 이동한다
```
flush → process → refresh_mmap → compile_state.save() → HNSW 재빌드 → Mutex → link
```
한 병목을 해결하면 다음이 드러남. 각 단계에서 계측을 반복.

### 3. "매회 재계산 → 배치 1회" 패턴
- refresh_mmap: 1줄 변경으로 **2.3x** (238s→105s)
- compile_state.save(): 1줄 변경으로 **1.26x** (105s→83s)
- HNSW 재빌드: batch_mode 체크 → **17x** per-doc (0.92s→0.053s @1500문서)
- 디스크 I/O뿐 아니라 **CPU 집약 재계산(HNSW)**에도 동일 패턴 적용

### 4. 벤치마크 3회 중앙값 필수
- 병렬화 벤치에서 단일 실행 1.31x → 3회 중앙값 -3.5%
- 디스크 캐시 cold/warm, 실행 순서가 결과를 왜곡

### 5. 구조 변경은 성능 동등 확인 후
- Blue-Green 1차: batch_end HNSW 빌드로 13.9초 → 빈 슬롯 + HNSW 스킵으로 해결
- Blue-Green 2차: 배치 중 brute-force 비용 증가(165초) → batch_begin에서 빈 슬롯으로 해결
- 구조 개선 시 반드시 **기존 대비 벤치마크 비교** 수행

## 전체 여정

```
Phase 0:   289초 (2.4 docs/s)    — 초기
Phase 3:   105초 (6.7 docs/s)    — refresh_mmap 배치화
Phase 4:    83초 (8.4 docs/s)    — compile_state.save() 배치화
Phase 47:  125초 (16.0 docs/s)   — 행렬곱 + HNSW 스킵 (2000문서)
Phase 48:  125초 (16.0 docs/s)   — Blue-Green (구조 개선, 성능 동등)
```

## 최종 수치 (2000문서 기준)

| 지표 | 시작 | 최종 | 개선 |
|------|------|------|------|
| 총합 | 289초 | 125초 | 2.3x |
| per-doc avg | 318ms | 53ms | 6x |
| per-doc p95 | 2,075ms | 62ms | 33x |
| p95/p50 분산 | 45x | 1.2x | 평탄화 |

## 확정된 최적화 (8개)

| # | 기법 | 효과 | Phase |
|---|------|------|-------|
| 1 | refresh_mmap 배치 스킵 | 2.3x | 45 |
| 2 | compile_state.save() 배치 스킵 | 1.26x | 46 |
| 3 | HNSW 재빌드 배치 스킵 | 17x per-doc | 47 |
| 4 | ReferencedBy 양방향 link | 관계 23% 증가 | 47 |
| 5 | EmbeddingSnapshot (zero-copy) | flush 인프라 | 47 |
| 6 | 행렬 곱 flush_crossref_matrix | flush search 최적화 | 47 |
| 7 | Blue-Green SearchSlot + atomic swap | 검색 무중단 | 48 |
| 8 | 임계치 기반 refresh (50건/5분) | p95 104ms→62ms | 48 |

## 보류 기법의 재검토 트리거

| 기법 | 재검토 조건 |
|------|------------|
| MinHash LSH | 5,000+ 문서에서 flush > 60초 |
| producer-consumer | 실 LLM 효율 60% 이하 |
| cap 축소 | link 비용이 전체의 50%+ |
| 증분 flush | 소량 추가 패턴에서 flush 비효율 |
