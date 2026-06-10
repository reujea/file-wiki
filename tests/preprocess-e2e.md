# 전처리기 실사용 테스트 시나리오

> 2026-04-15

## 호스트 환경

| 도구 | 상태 |
|------|------|
| pandoc | ❌ 미설치 |
| python-docx | ❌ 미설치 |
| openpyxl | ✅ 3.1.5 |
| libreoffice | ❌ 미설치 |
| tesseract | ❌ 미설치 |

## 시나리오

### S1: XLSX 가공 (openpyxl 있음 → 성공 예상)
- inbox에 .xlsx 투입
- openpyxl로 시트별 텍스트 추출
- LLM 가공 → 색인

### S2: DOCX 가공 (도구 없음 → 에러 예상)
- inbox에 .docx 투입
- pandoc/python-docx/libreoffice 모두 없음
- 에러 메시지 + 설치 가이드 표시

### S3: TXT/JSON/YAML 가공 (직접 읽기 → 성공 예상)
- 이미 동작 확인됨 (기존 테스트)
- 추가 확인만

### S4: pandoc 설치 후 DOCX 재테스트
- pip install python-docx 후 재시도

### S5: 호스트 도구 API 테스트
- pipeline.exe stats로 호스트 도구 감지 로그 확인
