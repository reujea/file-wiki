---
number: 3
topic: refresh-mmap-batch
date: 2026-04-20
---

# refresh_mmap 배치 스킵 최적화

## 상황
7유형 601문서 벤치마크에서 per-doc avg 0.39s, 소형 파일(0KB)도 2.9s 소요. 파일 크기와 처리 시간 간 상관관계 없음 확인.

## 문제
upsert()에서 **매 문서마다** refresh_mmap() 호출 — mmap 파일 재로드 오버헤드가 컨텐츠 처리보다 지배적.

## 원인
배치 처리(batch_begin/end) 패턴이 persist()에만 적용되고, refresh_mmap()에는 적용되지 않았음. batch_mode 체크 누락.

## 개선
- upsert()에서 `if !batch_mode { self.refresh_mmap(); }` 1줄 추가
- batch_end()에서 refresh_mmap() 1회만 실행
- 결과: **238s→105s (2.3x)**, per-doc p99 0.87s→안정화
- stale mmap 리스크 없음: find_by_hash는 documents Vec 사용, flush는 batch_end 이후

## 교훈
배치 모드 패턴은 **모든 I/O 작업**에 일관 적용해야 한다. persist만 스킵하고 mmap은 빠뜨린 것처럼, 한 곳만 최적화하면 다른 I/O가 병목이 된다. "파일 크기 vs 처리 시간 무상관" 같은 반직관적 데이터가 I/O 오버헤드 힌트.
