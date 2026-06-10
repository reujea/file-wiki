# 실사용 E2E 테스트 시나리오

> 2026-04-14 작성

## 환경

- 위치: `D:\file-test\`
- 바이너리: `pipeline.exe` (Tauri GUI + CLI + MCP 통합)
- 설정: `pipeline.toml` (auto-init 생성, sqlite 기본)
- Claude CLI: 설치됨

## 시나리오 1: 문서 3종 투입 → 자동 가공

### 입력 문서

| 파일명 | 유형 | 내용 |
|--------|------|------|
| 회의록_2026-04-14.txt | meeting | 마케팅 전략 회의 |
| 일지_2026-04-14.txt | log | 일일 업무 기록 |
| 메모_아이디어.txt | memo | 제품 개선 아이디어 |

### 검증 항목

- [ ] 3개 파일 모두 가공 완료 (Processing 탭에서 확인)
- [ ] doc_types 자동 분류 (meeting, log, memo)
- [ ] processed/ 폴더에 .zst 파일 생성
- [ ] originals/ 폴더에 원본 .zst 파일 생성
- [ ] .vec 파일 생성 (임베딩)
- [ ] stats 커맨드: 총 문서 수 = 3

## 시나리오 2: CLI 검색

```bash
pipeline.exe serve  # MCP 서버 시작
# 별도 터미널에서 Claude Code로 검색
```

### 검증 항목

- [ ] "마케팅 전략" 검색 → 회의록 반환
- [ ] "업무 기록" 검색 → 일지 반환
- [ ] "제품 개선" 검색 → 메모 반환
- [ ] doc_type 필터: meeting → 회의록만
- [ ] 날짜 필터: 2026-04-14 → 3건

## 시나리오 3: Dashboard GUI 확인

- [ ] Documents 탭: 3건 표시
- [ ] Processing 탭: 3건 완료 상태
- [ ] Verification 탭: 메트릭 기록됨
- [ ] Pipeline 탭: 17단계 노드 표시
- [ ] Settings 탭: 설정 로드 성공
- [ ] 상단 헤더: 전체 문서 = 3

## 시나리오 4: 중복 투입

- [ ] 동일 파일 재투입 → SHA-256 중복 → 스킵
- [ ] stats: 여전히 3건

## 시나리오 5: 민감 파일 스킵

- [ ] .env 파일 투입 → watcher 스킵 (config 확장자)
- [ ] password.txt (키워드 "비밀번호" 포함) → 민감 판별 → sensitive/ 이동
