# Quick Start Guide

## 1분 설치

1. `pipeline.exe`를 원하는 디렉토리에 복사
2. 더블클릭 → 자동 초기화 (pipeline.toml + inbox/processed/originals/logs 생성)

```
D:\file-test\
├── pipeline.exe      ← 복사한 바이너리
├── pipeline.toml     ← 자동 생성 (설정)
├── doc_types.toml    ← 자동 생성 (문서 유형 스키마)
├── inbox/            ← 파일 투입 폴더
├── processed/        ← 가공 결과 (.zst + .vec)
├── originals/        ← 원본 백업
└── logs/             ← 실행 로그
```

> 첫 실행 시 Qdrant 없이 SQLite 벡터DB로 즉시 동작합니다. Qdrant 사용은 pipeline.toml에서 `backend = "qdrant"` 설정.

---

## 사용 흐름

```
                        ┌──────────────────────────────────────────┐
                        │           pipeline.exe (16MB)            │
                        │                                          │
  inbox/ 에 파일 투입 ──►  자동 감시 → 분류 → 가공 → 검증 → 색인  │
                        │         ↓                                │
                        │   processed/ 에 .zst + .vec 저장         │
                        │         ↓                                │
  Claude Code (MCP) ◄───  search / list / stats 도구로 검색       │
                        │                                          │
  Dashboard (GUI) ◄─────  실시간 모니터링 + 파이프라인 설정        │
                        └──────────────────────────────────────────┘
```

1. `inbox/` 폴더에 파일을 복사하면 자동 감시(watcher)가 감지
2. 파이프라인 17단계를 거쳐 가공 (전처리 → LLM 분류/가공 → 검증 → 임베딩 → 압축 → 색인)
3. Claude Code에서 MCP 도구로 검색, 또는 Dashboard에서 모니터링

---

## MCP 연결 (Claude Code)

```bash
claude mcp add file-pipeline -- D:\file-test\pipeline.exe serve
```

연결 후 Claude Code에서 사용 가능한 도구 (11개):

| 도구 | 설명 |
|------|------|
| `search` | 키워드/의미 기반 문서 검색 |
| `get_document` | 특정 문서 상세 조회 |
| `list_documents` | 전체 문서 목록 |
| `stats` | 시스템 통계 |
| `lint` | 문서 품질 검사 (orphan/stale/모순) |
| `revise_topic` | 토픽 수정 |
| `kg_neighbors` | 지식 그래프 이웃 노드 |
| `kg_paths` | 지식 그래프 경로 탐색 |
| `kg_stats` | 지식 그래프 통계 |
| `list_todos` | Todo 목록 |
| `complete_todo` | Todo 완료 처리 |

---

## 배치 가공

GUI 없이 inbox의 모든 파일을 일괄 처리:

```bash
pipeline.exe batch
```

- 작업 큐 기반 (`.work-queue.json`으로 상태 영속화)
- 중단 후 이어서 처리 가능
- 변경/삭제된 파일 자동 감지

---

## Dashboard (GUI)

`pipeline.exe`를 인자 없이 실행하면 Dashboard GUI가 표시됩니다.

**9개 탭:**

| 탭 | 기능 |
|----|------|
| Documents | 가공된 문서 목록, 검색, 상세 보기 |
| Processing | 처리 현황 (대기/처리중/완료/실패) + 재처리 |
| Todos | Todo 목록 + 이월/완료 관리 |
| Verification | 검증 메트릭 (구조/키워드/ROUGE-L/개체) |
| Topics | 토픽 클러스터링 + 요약 |
| Credentials | LLM 프로바이더 API 키 관리 |
| Feedback | 소스 모드 피드백 (우클릭 → claude -p → 수정) |
| Pipeline | 17단계 파이프라인 시각화 + 시뮬레이션 |
| Settings | 시스템 설정 (크레덴셜/로깅/알림) |

시스템 트레이에 상주하며, 창 닫기 = 숨기기. 트레이 메뉴에서 종료.

---

## 지원 포맷

| 분류 | 확장자 | 전처리 |
|------|--------|--------|
| 텍스트 | `.txt`, `.md`, `.csv`, `.log` | 직접 읽기 |
| 문서 | `.pdf` | pandoc / libreoffice (자동 감지) |
| 오피스 | `.docx` | python-docx (자동 감지) |
| 스프레드시트 | `.xlsx` | openpyxl (자동 감지) |
| 기타 텍스트 | `.json`, `.xml`, `.yaml`, `.html` | 직접 읽기 |

**자동 스킵 대상:** 임시파일(.tmp), 설정파일(.env/.ini), 소스코드(.rs/.py/.js 등 24종), 바이너리(.exe/.zip/.mp3), 특정파일(Cargo.toml, pipeline.toml)

> 전처리 도구는 호스트 자동 감지 (HostToolDetector). 미설치 시 기본값 "none"으로 텍스트 직접 읽기 시도.

---

## CLI 명령어 요약

| 명령어 | 설명 |
|--------|------|
| `pipeline.exe` | Dashboard GUI 실행 |
| `pipeline.exe start` | 전체 서비스 시작 (watch + batch + 트레이 + lint/purge + topic-merge) |
| `pipeline.exe batch` | GUI 없이 inbox 배치 가공 |
| `pipeline.exe serve` | MCP 서버 모드 (Claude Code stdio) |
| `pipeline.exe init` | 수동 초기화 |
| `pipeline.exe show-config` | 현재 설정 표시 |
| `pipeline.exe stats` | 시스템 통계 |
| `pipeline.exe memo <text>` | 메모 등록 (inbox 경유) |
| `pipeline.exe export` | 문서 내보내기 |
| `pipeline.exe todo` | Todo 관리 |
| `pipeline.exe kg <sub>` | 지식 그래프 (neighbors/paths/stats) |
| `pipeline.exe topic-revise` | 토픽 수정 |
| `pipeline.exe backfill-vec` | .vec 파일 재생성 |
