# 08: flushed_embeddings 중복 검색 성능 저하

## 상황
Phase 50 증분 flush 구현 후 100문서 벤치에서 10 docs/s → 4.6 docs/s로 성능 저하 발생.

## 문제
flush_crossref에서 flushed_embeddings + 배치 내 상호 참조(items 간 O(K²))를 추가한 결과:
- 100×100 = 10K 추가 cosine 계산 (배치 내 상호)
- flushed에 이미 snapshot에 포함된 문서가 중복 추가되어 관계 수 폭증 (5,650 → 62,130)

## 원인
1. batch_end() 호출 시 mmap이 갱신되어 snapshot에 문서가 포함됨
2. flush 후 add_flushed_embedding으로 같은 문서를 flushed에도 추가 → 중복 검색
3. 배치 내 상호 참조(items 간)는 O(K²)로 K가 커질수록 비용 급증

## 개선
1. 배치 내 상호 참조 제거 — snapshot에서 이미 검색됨
2. flushed에서 snapshot에 이미 있는 문서 스킵 (HashSet 체크)
3. batch_end 후 flushed 추가 제거 — snapshot에 이미 포함

## 교훈
검색 대상 확장 시 기존 소스와의 중복을 반드시 확인. O(K²) 추가 비용은 K=100에서도 체감됨.
