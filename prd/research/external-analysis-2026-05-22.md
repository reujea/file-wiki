---
created: 2026-05-22
purpose: 외부 프로젝트 분석 단일 진실원 — JAMES v0.3.0 재검증 + Mirage v0.0.1
predecessor: prd/research/external-analysis-2026-05-15.md (wikidocs 352523/353407)
related: spec/lesson-learned/50_phase91-james-pattern-absorption.md
---

# 외부 프로젝트 분석 — JAMES v0.3.0 재검증 + Mirage v0.0.1

## 1. JAMES 변동 (2026-05-17 → 2026-05-22)

**핵심**: v0.3.0 (2026-05-17) 이후 메이저 변동 없음. Phase 91 lesson 50 흡수 결정 유효.

### 추가 명확화 사항
- 백엔드: FastAPI + Uvicorn (Python)
- 벡터DB: ChromaDB
- LLM: Ollama (Gemma / DeepSeek-Coder / LLaVA)
- 검색: BM25 + 벡터 하이브리드
- 인증: JWT
- 라이선스: MIT (CLA 필수)
- 로드맵: v0.3 (현재) → v0.4 도메인 파일럿 → v1.0 (HTTPS/SSO/멀티테넌시)

### Phase 91 흡수 결정 재검증
- 🟢 흡수 5건 (A1'/A2/A3/B1/B2): 유효
- 🔴 보류 (RBAC/Change Request/5 역할/메모리 3계층): v1.0 트리거 대기

### 신규 발견 (Phase 91 미흡수)
- **자체 진화 게이트** (피드백→후보→벤치→인간→배포→자동 롤백): 🟡 부분 일치 — Phase 77 ConfigSnapshot + Phase 91 audit_trace 결합 시 흡수 가능 (H1 후보)
- ChromaDB/Ollama/JWT 스택: 🔴 Rust 단일 바이너리 불일치 — 보류

## 2. Mirage v0.0.1 (2026-05-06)

**저장소**: github.com/strukto-ai/mirage
**목적**: "Unified Virtual File System for AI Agents" — S3/GDrive/Slack 등을 단일 VFS로 통합
**라이선스**: Apache-2.0 (상용 자유)

### 기술 스택
- 언어: TypeScript 56.7% + Python 43.2% (Rust 없음)
- 런타임: Python ≥ 3.12 (`mirage-ai`), Node.js ≥ 20 (`@struktoai/mirage-node`)
- 플랫폼: macOS, Linux (FUSE 기반)

### 핵심 아키텍처

**Workspace API**:
```python
ws = Workspace({"/data": RAMResource(), "/s3": S3Resource(S3Config(...))})
await ws.execute("cp /s3/report.csv /data/report.csv")
ws.snapshot("demo.tar")
ws.command('summarize', ...)  # 전역 커스텀 명령
```

**Resource 타입** (18종+):
- 로컬: RAMResource, 디스크
- 클라우드: S3, R2, OCI, Supabase, GCS
- 협업: Gmail, GDrive, GDocs, GSheets, GitHub, Linear, Notion, Slack, Discord
- DB: MongoDB, Redis
- 기타: SSH

**Cache 2계층**:
| 캐시 | 역할 | 저장소 | TTL |
|------|------|--------|-----|
| Index Cache | 디렉토리·메타데이터 | RAM 또는 Redis | 10분 |
| File Cache | 객체 바이트 | RAM 512MB 또는 Redis 8GB | - |

**Command 등록 3차원**:
- 전역 / 리소스별 / 파일타입별
- 예: `ws.command('cat', { resource: 's3', filetype: 'parquet' }, ...)` → Parquet을 JSON으로 렌더

**에이전트 통합**: OpenAI Agents SDK / Vercel AI SDK / LangChain / Pydantic AI / CAMEL / OpenHands / Mastra

### file-pipeline 관점 흡수 후보

| 후보 | 라벨 | 결정 |
|------|------|------|
| H3 MCP 카탈로그 다차원 분류 (Phase 91 B2 확장) | 🟢/🟢 | Phase 92 진행 |
| H4 search 8단계 인스펙터 확장 | 🟢/🟢 | Phase 92~93 진행 |
| H1 자동 롤백 트리거 (Phase 77 + 91 결합) | 🟢/🟢 | Phase 92~93 진행 |
| H2 Index/File 캐시 분리 | 🟡/🟡 | ROI 측정 후 |
| **H5 원격 저장소 표준화 (Mirage Resource 패턴)** | 🟢/🟡 | **사용자 명시 합의 후 Phase 92 진행** |
| H6 VFS bash 인터페이스 | 🔴/🔴 | 본질 도메인 불일치 보류 |

### 도메인 정렬 평가 (메타 룰 16)

| Mirage 가정 | file-pipeline 도메인 | 정렬 |
|------------|---------------------|------|
| AI 에이전트 사용자 | MCP 통해 Claude Code 보조 | 🟡 |
| 여러 원격 백엔드 통합 | 로컬 + Notion 1개 (Phase 90) | 🟡 |
| bash 명령 인터페이스 | 도메인 특화 MCP 도구 | 🔴 |
| TypeScript + Python | Rust 단일 바이너리 | 🔴 |
| 분산 캐시 | 인프로세스 단일 사용자 | 🔴 |
| VFS 추상화 | 헥사고날 도메인 특화 | 🟡 |

## 3. 메타 룰 적용

### 메타 룰 16 차원 B 신규 누적 사례

| 솔루션 | 차원 B 라벨 | 결정 |
|--------|-----------|------|
| JAMES v0.3.0 RBAC PolicyEngine | 🔴 | 보류 |
| JAMES Change Request 인간 게이트 | 🔴 | 보류 |
| JAMES 3-stage output 1단계 | 🟢 | 흡수 (Phase 91 A2) |
| JAMES verifier 함수 통합 | 🟢 | 흡수 (Phase 91 B1) |
| JAMES trace_id + audit | 🟢 | 흡수 (Phase 91 A3) |
| **Mirage VFS / bash 인터페이스** | **🔴** | **보류 (본 분석)** |
| **Mirage Resource 추상화 (S3/GDrive/...)** | **🟡** | **사용자 명시 합의 — Phase 92 진행** |
| **Mirage Index/File 캐시 2계층** | **🟡** | **ROI 측정 후** |
| **Mirage Command 3차원 등록** | **🟢** | **흡수 (Phase 92 H3)** |

### 메타 룰 20 누적 (외부 프로젝트 도메인 정렬, lesson 50 시작)

| 프로젝트 | 본질 일치 흡수 | 부수 일치 흡수 | 불일치 보류 |
|---------|-------------|-------------|------------|
| JAMES (lesson 50) | Verifier 통합 | audit_trace / MCP mutates | RBAC / Change Request / 5 역할 |
| TabPFN/TFM | 없음 | 이상 탐지 / ETA 예측 | doc_type 분류 대체 / 검색 리랭킹 대체 |
| JAMES v0.3.0 재검증 | (변동 없음) | 자동 롤백 트리거 | ChromaDB / Ollama / JWT 스택 |
| Mirage | 없음 (도메인 불일치) | MCP 카탈로그 다차원 / Resource 추상화 부분 | VFS / bash / TypeScript-Python 스택 |

**누적 4건 → 메타 룰 20 META 정식 승격 임계 도달**

### 메타 룰 21 후보 강화

"외부 도메인 도구 흡수 시 본질/부수 도메인 분리":
- TabPFN + Mirage 모두 본질 도메인 불일치 / 부수 도메인 부분 일치
- 누적 2건 → 1건 추가 시 META 정식 승격 검토

## 4. 진행 결정 (사용자 합의 2026-05-22)

**사용자 명시**: H5 원격 저장소 표준화 **포함**해서 Phase 92 진행

### Phase 92 작업 범위
1. **H3** MCP 카탈로그 다차원 분류 (Phase 91 B2 확장)
2. **H4** search 8단계 인스펙터 확장 (또는 Phase 93로 분리)
3. **H1** 자동 롤백 트리거 (Phase 77 + 91 결합)
4. **H5** 원격 저장소 표준화 (Mirage Resource 패턴 흡수)
5. 메타 룰 20 META 정식 승격 + 메타 룰 21 후보 정식 등록

### 보류 항목 (Phase 92 제외)
- H2 Index/File 캐시 분리 (ROI 측정 후)
- H6 VFS bash 인터페이스 (본질 도메인 불일치)
- JAMES ChromaDB/Ollama/JWT (Rust 단일 바이너리 불일치)
