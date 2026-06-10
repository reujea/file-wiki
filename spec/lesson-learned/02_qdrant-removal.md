# Lesson Learned: Qdrant 제거 — 외부 의존성 vs 인프로세스

## 상황
80MB qdrant.exe를 vendor/에 동봉하고, gRPC로 연결하는 구조. 로컬 데스크톱 앱에서 별도 프로세스 관리.

## 문제
1. 배포 크기 +80MB, 메모리 +30~40MB (프로세스 기본)
2. 시작 시 3~5초 대기 (Qdrant 프로세스 시작)
3. gRPC 직렬화 오버헤드 ~0.1~0.2ms/호출
4. SqliteVecAdapter라는 이름이지만 SQLite를 사용하지 않음 (혼란)
5. 20K 문서에서 Qdrant의 분산/멀티테넌트 기능 불필요

## 원인
- 초기 설계 시 "대규모 확장을 위해 Qdrant 도입"했으나, 실제 사용 규모(20K)에서 인프로세스 mmap+Rayon이 충분
- 외부 전문가 상담으로 "로컬 앱에서 별도 프로세스는 과잉"임을 확인

## 개선
- LocalVectorStore로 리네임 + Qdrant 완전 제거
- 배포 크기 80MB 절감, 시작 즉시, gRPC 오버헤드 0
- HNSW 캐시 (dirty flag + lazy rebuild)로 500+ 문서 검색 O(log N)
- **교훈**: 외부 서비스 의존은 "정말 필요한 규모"에서만. 인프로세스가 가능하면 인프로세스가 항상 우위 (로컬 앱 기준)
