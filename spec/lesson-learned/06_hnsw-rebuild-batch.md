---
number: 6
topic: hnsw-rebuild-batch
date: 2026-04-20
---

# HNSW 재빌드 batch 스킵

## 상황
2000문서 벤치에서 per-doc avg가 500문서 0.076s → 1000문서 0.844s → 1500문서 0.922s로 급증. 타임아웃 발생.

## 문제
process_file 내 의미 중복 체크(search_similar) → 500문서 이상에서 HNSW 캐시 사용 → upsert 후 hnsw_dirty=true → **매 문서마다 HNSW 전체 재빌드**. O(N log N) × N = O(N² log N).

## 원인
"매회 I/O → 배치 1회" 패턴의 변종. persist/refresh_mmap은 batch_mode 체크를 했지만, search_similar 내 HNSW 재빌드에는 적용하지 않았음.

## 개선
search_similar()에서 `if batch_mode { return brute_force(); }` 추가. batch 중에는 HNSW 대신 Rayon brute-force 사용.
- 결과: per-doc avg 0.922s → **0.053s** @1500문서 (17x)
- 2000문서 전체: 타임아웃 → **125초** (16 docs/s)

## 교훈
"매회 I/O" 패턴은 디스크 I/O(persist, save)뿐 아니라 **CPU 집약 재계산(HNSW 빌드, 인덱스 재구축)**에도 동일하게 적용된다. batch_mode 체크 대상을 "디스크 쓰기"에서 **"비용이 큰 모든 재계산"**으로 확장해야 한다.
