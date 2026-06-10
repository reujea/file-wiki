---
updated: 2026-05-20 (Phase 80 진입점 3분기 + Phase 84 PII live reload + Phase 90 Notion 옵션 반영)
---

# 사용자 시나리오 구성도

## 시나리오 1: 첫 실행

```
사용자: pipeline.exe 실행
  |
  v
auto_init() 감지: pipeline.toml 없음
  |
  +-> pipeline.toml 생성 (주석 포함 템플릿)
  +-> inbox/ processed/ originals/ logs/ 생성
  +-> doc_types.toml 생성 (17개 유형 스키마)
  +-> 콘솔 안내 메시지 표시
  |
  v
build_service()
  +-> backend=sqlite (기본, LocalVectorStore 인프로세스)
  +-> Claude CLI 감지 → LLM/임베딩 활성화
  +-> 알림: NullNotificationAdapter (미설정)
  |
  v
Tauri GUI 시작 → Dashboard 표시
  +-> 문서 0건, KG 0건
  +-> Settings 탭에서 크레덴셜/알림 설정 가능
  +-> Pipeline 탭에서 Default 파이프라인 확인
```

**관련 설정:** pipeline.toml 전체 (auto_init 생성)
**관련 UI:** Settings 탭 (크레덴셜, 시스템), Pipeline 탭 (Default)

---

## 시나리오 2: 파일 투입 → 가공 → 검색

```
사용자: inbox/ 에 파일 드롭 (예: 회의록.txt)
  |
  v
FileWatcher (notify::Watcher)
  +-> should_skip() 5단계 검사 → 통과
  +-> match_pipeline() → glob 패턴 매칭 (priority 순)
  |     +-> *.txt → "텍스트 파이프라인" 매칭
  |     +-> 미매칭 시 → Default 파이프라인 (*.*) 사용
  |
  v
process_file_with_pipeline() — 14단계
  |
  [사전검사]
  | 1. 민감 판별 (SensitivityDetector)
  | 2. Fragment 감지 (< fragment_threshold → LLM 스킵)
  | 3. SHA-256 중복 체크
  | 4. 증분 해시 (CompileState)
  |
  [파이프라인 스텝]
  | 5. Preprocess (PDF/OCR → 텍스트)
  | 5.5 Chunking (>40KB → 의미 단위 분할)
  | 6. LLM 분류+가공 (classify_and_process)
  |     +-> doc_types, keywords, search_hints, sections, code_blocks
  | 7. Verify 6가지 검증 → 2-Pass 피드백 → quarantine
  | 8. Embedding (Claude CLI 128축 의미 벡터)
  | 9. Storage (zstd 압축)
  |
  [후처리]
  | 10. 의미 중복 체크
  | 11. .vec 파일 영속화
  | 12. Todo 병합
  | 13. VectorDB 색인 (dense + sparse RRF)
  | 14. CrossRef 양방향 링크
  | 15. 증분 기록 → 자동 토픽 병합
  |
  v
Dashboard: Processing 탭에 실시간 반영
  +-> 완료=녹색, 실패=빨강
  +-> 알림 전송 (Telegram/Slack, 설정 시)

--- 검색 ---

Claude Code (MCP): search("회의 결정사항")
  |
  v
McpState::handle_search()
  +-> embedding.embed("회의 결정사항") → Vec<f32>
  +-> vector_db.search_hybrid(embedding, keyword, top_k*3)
  |     +-> LocalVectorStore: dense + sparse(BM25) RRF + HNSW 검색
  +-> [리랭킹] reranker.rerank(query, candidates)  ← Phase 4에서 추가
  +-> doc_type/date 필터
  +-> storage.read_header(path, 15) → 미리보기
  +-> 결과 반환: [{id, score, doc_types, date, header}]
```

**관련 설정:**
- Pipeline 탭: 스텝별 오버라이드 (LLM credential, 검증 임계값, 압축 레벨)
- Settings: 크레덴셜, 벡터DB, 알림

---

## 시나리오 3: 파이프라인 설정 (Phase 56 2컬럼)

```
사용자: Pipeline 탭 열기
  |
  v
2컬럼 레이아웃 (단일 파이프라인, 18 노드 고정)
  |
  좌측 사이드바(320px): 시뮬레이션 + 결과 + 로그
       텍스트 입력 → 실제 dry-run (LLM 호출 + 검증, DB 저장 없음)
  |
  우측 메인:
    [데이터 가공] [외부 저장소] [청킹] [보존 & Purge]
    │
    └─ 활성 서브탭 콘텐츠 (Preprocess/LLM/Verify/Embedding/Storage 등)
    │  + 호스트 도구 현황, 전처리 테스트, doc_types 검증 스키마
    │
    └─ 하단 축소 플로우 (사전검사 → 스텝 → 후처리, 읽기전용)
  |
  v
필드 변경 → 1초 debounce auto-save → settings.db
```

**관련 설정:**
- 데이터 가공 서브탭: preprocessing/llm/verify/embedding/compression
- 외부 저장소 서브탭: remote_storage (network/webdav/s3)
- 청킹 서브탭: chunking + crossref
- 보존 & Purge 서브탭: retention + 수동 dry-run/execute

---

## 시나리오 4: 실패 대응

```
Processing 탭: 파일 상태 = "Failed" (빨간색)
  |
  v
사용자: row 클릭 → 상세 패널
  +-> progress 이벤트 타임라인
  +-> error 로그 (실패 원인)
  |
  v
판단:
  +-> 검증 실패 → quarantine/ 이동됨
  |     → Verification 탭에서 상세 메트릭 확인
  |     → Pipeline 탭에서 검증 임계값 조정 후 재시도
  |
  +-> LLM 오류 (타임아웃/API 에러)
  |     → Settings에서 크레덴셜 확인
  |     → "실패 항목 재처리" 버튼 클릭
  |     → retry_failed() → Failed→Pending 리셋
  |
  +-> Lint 문제
        → Documents 탭 하단 Lint 보고서
        → orphan: 삭제 버튼
        → stale: 재가공 또는 삭제
        → 백링크 누락: 보강 버튼
```

**관련 설정:** Verification 탭, Processing 탭, Pipeline verify 노드 (임계값)

---

## 시나리오 5: 외부 저장소 백업

```
사용자: Pipeline 탭 > 외부 저장소 서브탭 (Phase 67 인스펙터)
  |
  v
provider 선택 (Phase 90: Notion 추가):
  +-> "network" → network_path 입력 (\\NAS\share\backup)
  +-> "webdav" → URL + 인증 정보
  +-> "s3" → endpoint + bucket + access_key + secret_key
  +-> "notion" → integration token + parent_page_id + mode(page/attach)
       └ mode=page: 가공본 → Notion 자식 페이지 (paragraph 블록 자동 분할)
       └ mode=attach: 명시적 미지원 (Notion API 제약, S3/WebDAV 권장)
  |
  v
인스펙터 480px → enabled = true → 저장 (1초 debounce auto-save)
  |
  v
파일 처리 완료 시 자동 업로드:
  +-> processed/파일명.zst → remote_key: "processed/파일명.zst"
  +-> originals/파일명.zst → remote_key: "originals/파일명.zst"
  +-> Notion: 가공본 텍스트 → 부모 페이지 아래 자식 페이지 생성
  +-> 실패 시 warn 로그 (처리 자체는 성공)
```

**관련 설정:** Pipeline 외부 저장소 서브탭 (network/webdav/s3/notion 4종)

---

## 시나리오 6: 설정 도우미 진입 (Phase 80 3분기)

```
사용자: 헤더 "🤖 AI 설정 도우미" 클릭
  |
  v
3분기 선택지 (Phase 80):
  +-> 일반 → 시나리오 기반 추천 (5축 SetupProfile)
  +-> AI 분석 → 코퍼스 신호 자동 분석 (search_mode_counters + CRAG + chunk_stats)
  +-> 직접 모듈 선택 → 12개 동작 모듈 체크박스 (가공 5 + 검색 4 + 운영 3)
  |
  v
모듈 추천 (충돌 시 보수적 해소 — 큰 청크/활성화/합집합)
  |
  v
Critical 항목 적용 동의 체크박스 → setup_apply_modules
  |
  v
decision_log에 적용 이력 영속화 (Phase 82)
  +-> apply 결정 (accepted/rejected/critical_skipped)
  +-> snapshot 자동 생성 (Phase 77, 자동 롤백 4트리거)
```

**관련 설정:** 헤더 도우미 진입점 / Settings > 자동 추천 (C1) / setup_modules 모달

---

## 시나리오 7: PII 사용자 정의 패턴 (Phase 84 live reload)

```
사용자: Settings > 사용자 정의 PII 패턴
  |
  v
"패턴 추가" 클릭 → 모달:
  +-> 이름 (예: my_company_id)
  +-> 정규식 (예: ^EMP-[0-9]{6}$)
  +-> enabled = true
  |
  v
저장 → pii_pattern_add (Regex::new 사전 검증)
  +-> 잘못된 정규식이면 모달에 에러 표시 + DB 미저장
  +-> 정상이면 settings.db.pii_patterns_user 저장
  |
  v
service.reload_pii_patterns() 자동 호출 (Phase 84)
  +-> 재시작 불필요, 다음 가공부터 즉시 반영
  |
  v
이후 파일 가공 시 본문 PII 스캔 — 신규 패턴 매칭 시 → sensitive/ 격리 + 알림
```

**관련 설정:** Settings PII 카드 (refresh-pii-patterns + pii-add)
