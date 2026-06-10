# Lesson Learned: 교차참조 전체 문서 스캔 전환

## 상황
top_k=3(상위 3건만 관계 생성) → threshold 기반 전체 문서 스캔으로 전환. 100문서에서 관계 394 → 9,900(25배). 하지만 1,000문서에서 30분+ 시간초과.

## 문제
1. threshold=0.5에서 HashEmbedder는 80%+ 문서가 매칭 → 관계 폭발
2. link()의 Vec.contains가 O(k)로 관계 누적 시 수억 스캔
3. get_keywords()의 Mutex가 Rayon 병렬성을 파괴
4. 벤치마크에서 flush_crossref 호출 누락으로 관계 0건 출력

## 원인
- top_k=3이 "비교 비용은 동일하면서 결과만 제한"하는 구조임을 인식하지 못함
- Vec.contains의 초선형 스케일링 (100문서 OK → 1,000문서 폭발) 미예측
- 외부 전문가 상담으로 진짜 병목(데이터 구조)을 확정

## 개선
- HashSet O(1) 중복 체크 → link 비용 일정
- keywords 스냅샷 1회 로드 → Mutex 없이 Rayon 병렬 유지
- 유형별 cap (2/5/20/10) + 조기 종료 → 관계 폭발 방지
- threshold 0.7 (0.80으로 추가 상향 검토 중)
- 결과: 1,000문서 30분+ → 105초 (17배 개선), 관계 59K
- **다음**: 진단 스크립트로 유형별 분포 측정 → threshold 0.80 실험 → Phase 2 mutual top-K
