# 09: summary 토큰 keyword_index 오염

## 상황
Phase 51 Document Summary 인덱스 구현 시 summary 토큰을 keyword_index에 추가.

## 문제
summary의 일반 단어(문서, 테스트, 내용 등)가 keyword_index에 들어가면서 모든 검색에서 특정 문서(meeting)가 상위에 나옴. search_accuracy 12건 중 6건 실패.

## 원인
summary를 단순 split_whitespace로 토큰화하여 keyword_index에 추가. 불용어 필터 없음. 일반적 단어가 모든 쿼리와 매칭됨.

## 개선
summary 토큰을 keyword_index에서 제거. 대신 search_hybrid에서 별도로 summary 필드를 매칭하여 후보에 추가. keyword_index는 LLM이 추출한 키워드만 유지.

## 교훈
인덱스에 새 데이터 소스를 추가할 때 기존 검색 품질 테스트를 반드시 실행. summary처럼 일반적 텍스트를 키워드 인덱스에 직접 넣으면 precision이 급락.
