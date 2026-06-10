---
updated: 2026-06-04
type: research
status: 7건 외부 솔루션 누적 비교 (Phase 91~107 + Phase A/B/E 흡수 결과 반영)
purpose: file-pipeline의 시장 위치 + 강점/약점 + 흡수 효과 정리
related:
  - prd/research/external-analysis-2026-05-15.md (wikidocs 353407)
  - prd/research/external-analysis-2026-05-22.md (Mirage)
  - prd/research/external-analysis-2026-05-27-graphrag.md (GraphRAG)
  - prd/research/external-analysis-2026-06-04-adaptive-chunking.md
  - prd/research/external-analysis-2026-06-04-grimoire.md
  - prd/research/tfm-tabpfn-analysis.md
  - spec/lesson-learned/META.md (메타 룰 20/21 누적 표)
---

# 경쟁 분석: file-pipeline vs 외부 솔루션 7건 누적 비교

## 0. 본 문서 위상

본 문서는 메타 룰 9 "외부 출처 단일 진실원" 자기 적용. 개별 솔루션 상세는 `prd/research/external-analysis-{date}-{name}.md` 단일 진실원. 본 문서는 **7건 누적 비교 + 본 솔루션 시장 위치 정리** 전담.

2026-04-08 작성 1차 버전(10 카테고리 × 10 요소 매트릭스, Phase 65 이전 상태)은 Phase 91~107 흡수로 60%+ 항목이 완료/변경되어 폐기. 본 2차 버전은 **흡수 결과 기반 비교**.

## 1. 비교 대상 (메타 룰 20/21 누적)

| # | 솔루션 | 본질 도메인 | 메타 룰 분류 | 분석 일시 |
|---|--------|------------|------------|----------|
| 1 | **JAMES v0.3.0** | RAG + 다중 사용자 협업 | 메타 룰 20 본질 일치 | 2026-05-21 (Phase 91/92) |
| 2 | **TabPFN / TFM** | 숫자 테이블 예측 (ML) | 메타 룰 21 도메인 불일치 | 2026-05-21 |
| 3 | **Mirage v0.0.1** | AI 에이전트 VFS | 메타 룰 21 도메인 불일치 | 2026-05-22 (Phase 92) |
| 4 | **AWS GraphRAG Toolkit** | 엔터프라이즈 RAG (AWS 클라우드) | 메타 룰 21 도메인 불일치 | 2026-05-27 (Phase 103) |
| 5 | **wikidocs 353407** | RAG 품질 검증 권고 | 메타 룰 20 본질 일치 | 2026-05-15 (Phase 87/88) |
| 6 | **Adaptive Chunking (arxiv 2603.25333)** | RAG 청킹 품질 | 메타 룰 20 본질 일치 | 2026-06-04 (Phase A/B) |
| 7 | **Grimoire** | 마크다운 KB no-embedding | 메타 룰 21 부분 일치 | 2026-06-04 (Phase E) |

## 2. 차원별 비교

### 2-1. JAMES v0.3.0 — 가장 가까운 동종

| 차원 | JAMES | file-pipeline |
|------|-------|---------------|
| 대상 사용자 | 다중 사용자 + 5 역할 RBAC | **단일 사용자 데스크톱** |
| 인증 | JWT + PolicyEngine + Change Request | OS 키링 (`module-secrets`) |
| 검증 | cognitive middleware | `reasoning/verifier.rs` (Phase 91 B1, JAMES 흡수) |
| 출력 | PII mask 3-stage | PII mask + live reload (Phase 91 A2, JAMES 흡수) |
| 감사 | trace_id + audit | `audit_trace` 단일 키 (Phase 91 A3, JAMES 흡수) |
| 스택 | ChromaDB + Ollama + JWT | **Rust 단일 바이너리** (인프로세스 HNSW) |
| 배포 | 서버 구성 필요 | `pipeline.exe` 한 파일 (19.03 MB) |

**판정**: 다중 사용자 영역(RBAC/Change Request) 명시 보류 (메타 룰 16 차원 B 🔴). 검증/PII/감사 영역만 흡수 — **단일 사용자 데스크톱 RAG의 검증 강도는 JAMES와 동급**.

### 2-2. TabPFN / TFM — 도메인 불일치

| 차원 | TFM | file-pipeline |
|------|-----|---------------|
| 본질 | 숫자 테이블 예측 (XGBoost 대체) | 임의 파일 RAG |
| 입력 | 정형 테이블 | 비정형 문서 |
| 흡수 | 이상 탐지 / ETA 예측 (부수만) | 메타 룰 21 |

**판정**: 본질 도메인 일치 없음. Phase 92 H1 audit_anomaly가 TFM 이상 탐지 패턴 변형 흡수.

### 2-3. Mirage v0.0.1 — 다른 패러다임

| 차원 | Mirage | file-pipeline |
|------|--------|---------------|
| 본질 | AI 에이전트용 가상 파일 시스템 | RAG 파이프라인 |
| 인터페이스 | bash + VFS path | MCP 28 도구 |
| 스택 | TypeScript + Python | Rust 단일 바이너리 |
| 흡수 | MCP 다차원 카탈로그 / Resource capabilities | 메타 룰 21 |

**판정**: Phase 92 H3/H5 흡수. 본질(bash/VFS)은 다름.

### 2-4. AWS GraphRAG — 클라우드 vs 데스크톱

| 차원 | GraphRAG | file-pipeline |
|------|---------|---------------|
| 본질 | 엔터프라이즈 RAG (Neptune/OpenSearch/Bedrock) | 단일 사용자 데스크톱 |
| 인프라 | AWS 클라우드 의존 | **인프로세스, 외부 의존 0** |
| 흡수 | G1 Statement / G2 Semantic / G3 Multi-hop / G4 TF-IDF (Phase 103 인프라 4건) | 메타 룰 21 |

**판정**: 알고리즘 아이디어만 흡수. 인프라 의존은 본 솔루션 철학과 정반대 — 명시 보류.

### 2-5. wikidocs 353407 — 외부 권고 (논문 아님)

| 차원 | wikidocs 권고 | file-pipeline 흡수 |
|------|--------------|---------------------|
| Strong claims 검출 | 권고 | `detect_strong_claims()` + Linter (Phase 87/88) |
| 다층 lint 주기 | 권고 | weekly/monthly 분기 (Phase 87/89) |
| needs_verification | 권고 | Metadata 필드 추가 (Phase 88) |

**판정**: 본질 100% 흡수. 본 솔루션의 lint 품질이 wikidocs 권고 수준.

### 2-6. Adaptive Chunking (arxiv 2603.25333) — 본질 일치

| 차원 | Adaptive Chunking | file-pipeline |
|------|--------------------|---------------|
| 본질 | RAG 청킹 품질 평가 + 전략 선택 | RAG 청킹 |
| 4 지표 | SC/BI/ICC/DCC (RC는 영어) | Phase A 흡수 (RC 보류) |
| 전략 추상화 | Adaptive | Phase B ChunkingStrategy enum (Adaptive 본체는 Phase C) |
| 종단 성능 | +8pp 답변 정확도 | baseline 측정 후 검증 |

**판정**: Phase A/B로 측정+추상화 인프라 도달. C/D가 남음.

### 2-7. Grimoire — 정반대 정책

| 차원 | Grimoire | file-pipeline |
|------|---------|---------------|
| 본질 | 마크다운 KB no-embedding | hybrid 검색 (dense+sparse) |
| 입력 | 마크다운만 | PDF/Excel/한글/마크다운 등 임의 |
| 인덱스 | SQLite FTS5 | LocalVectorStore HNSW |
| 가공 | **없음** | **LLM 2-Pass** (classify + verify) |
| 흡수 | E1 get_index / E2 write_note / E3 get_context (Phase E 3건) | 메타 룰 21 부분 일치 |

**판정**: 검색 라우팅 도구 패턴만 흡수. 임베딩 제거 정책은 본 솔루션 핵심 가치(품질 우선)와 정반대 — 영구 보류.

## 3. file-pipeline 고유 강점 4 영역

### A. 단일 사용자 데스크톱 + Rust 단일 바이너리

- 외부 인프라 의존 0 (JAMES JWT/ChromaDB, GraphRAG AWS, Mirage Python 스택과 정반대)
- 배포: `pipeline.exe` 한 파일 (19.03 MB, 2026-06-04 cross-build 검증)
- Linux → Windows cross-compile (cargo-xwin MSVC) 가능

### B. 광범위 입력 + 강한 가공

- 비교 솔루션 중 PDF/Excel/한글 모두 가공 가능한 건 본 솔루션만
- 2-Pass LLM (classify + verify) → JAMES cognitive middleware 수준 흡수
- Adaptive Chunking 4지표 측정 인프라 (Phase A)

### C. 헥사고날 + 모듈 분리

- `core` (도메인 + 포트) ↔ `adapters` (driven/driving) ↔ `shared` ↔ `modals`
- `_rust_module/` 16개 form-agnostic 외부 모듈
- Mirage/GraphRAG 추상화 한계를 헥사고날로 우월하게 해소

### D. 사용자 친화 외피 + 운영 도구

- MCP **28 도구** (Search/KG/Settings/Todo/Signal/Snapshot/Lint 7 카테고리)
- 6 UI 탭 + 인스펙터 패턴 + 🧭 4-step 온보드 (Phase 106)
- C1 자동 추천 + setup_modules 12종 + Decision Log + audit_anomaly
- Grimoire 9 도구 대비 **3배 표면적**이면서 단일 바이너리 유지

## 4. 본 솔루션의 시장 위치 (Elevator pitch)

> **"단일 사용자 데스크톱에서 JAMES 수준의 검증 + GraphRAG 수준의 추상화 + Grimoire 수준의 단순 배포를 동시에 제공하는 Rust 단일 바이너리 RAG 파이프라인"**

| 요소 | 출처 | 본 솔루션 |
|------|------|----------|
| 검증 강도 | JAMES Verifier + audit_trace + PII mask | Phase 91 흡수 |
| 추상화 깊이 | GraphRAG G1~G4 인프라 + Mirage MCP 다차원 | Phase 92~103 흡수 |
| 배포 단순성 | Grimoire 영감 단일 바이너리 | `pipeline.exe` 1 파일 |
| 가공 품질 | wikidocs 353407 + Adaptive Chunking | Phase 87/88/A/B 흡수 |

## 5. 누적 흡수 결과 정량

| Phase | 외부 출처 | 흡수 영역 |
|-------|---------|----------|
| 87/88/89 | wikidocs 353407 | needs_verification + open_questions + lint 다층 주기 + UI 가시화 |
| 91 | JAMES v0.3.0 | A1' 검사 통일 + A2 PII mask + A3 trace_id + B1 Verifier + B2 MCP mutates (5건) |
| 92 | JAMES 재검증 + Mirage | H1 audit_anomaly + H3 MCP 다차원 + H5 ResourceCapabilities (3건) |
| 93 | Phase 91/92 GUI 가시화 | 4건 UI 카드 |
| 94 | 메타 룰 20 META 정식 승격 | AuditPort 헥사고날 정공법 |
| 103 | AWS GraphRAG | G1 Statement / G2 Semantic / G3 Multi-hop / G4 TF-IDF (4건 인프라) |
| **A/B** | **Adaptive Chunking** | **4지표 측정 + ChunkingStrategy enum** |
| **E1/E2/E3** | **Grimoire** | **3 신규 MCP 도구** |

**누적**: 7건 외부 분석 → 약 **20 영역 흡수** (인프라 선구현 패턴, 디폴트 비활성). 회귀 0건.

## 6. 약점 + 위협 (SWOT의 W/T)

| 영역 | 약점 | 위협 |
|------|------|------|
| 디스크 | target ~18.8GB 측정 (cold full 빌드 후) | 빌드 환경 메모리 부담 |
| 성능 | fastembed cold start ~80s | 빠른 검색 솔루션 (Grimoire FTS5) 대비 초기 지연 |
| 언어 지원 | 한국어 coref 미보유 (Adaptive Chunking RC 보류) | 다국어 코퍼스 확장 시 제약 |
| Tauri Linux cross-build | llvm-rc 의존 발견 (2026-06-04) | 빌드 자동화 추가 의존 |
| **기능 과다** | **MCP 28 + Tauri command 67 + UI 탭 6** | **사용자 학습 곡선, 메타 룰 1 sub-rule 1f 누적 동기화 비용** |
| 다중 사용자 미지원 | JAMES RBAC 보류 | 다중 사용자 요구 시 흡수 비용 큼 |

## 7. 기능 과다 진단 (2026-06-04 본 세션 신규 인식)

본 솔루션은 7건 외부 흡수 + Phase 65~107 + Phase A/B/E를 거치며 표면적이 비대화. 다음 항목은 정리 후보 — 별도 작업 (본 문서 §7.x 또는 신규 phase 트리거):

| 영역 | 현재 | 정리 후보 |
|------|------|----------|
| MCP 도구 | 28개 | 사용 빈도 측정 → 미사용 도구 deprecate |
| Tauri commands | 67개 | dead_selector_scan_v3 false positive 정리와 묶음 |
| UI 6탭 + 서브카드 | 다수 | 사용자 첫 진입 흐름 (🧭 온보드) 외 카드 우선순위 재평가 |
| setup_modules | 12종 | 사용 통계 기반 통합 검토 |
| 외부 솔루션 흡수 영역 | 20개 인프라 (디폴트 비활성) | 측정 트리거 도달 영역 제외하고는 인프라만 유지 |

**메타 진단**: 기능 과다는 흡수 가속의 자연스러운 결과지만, **사용자 인지 부담** + **다중 위치 동기화 비용** + **첫 진입 학습 곡선** 3가지가 한계 도달. 정리 방안은 §8.

## 8. 정리 방안 (사용자 합의 필요)

다음 4 옵션을 §9 Q에서 합의:

### 옵션 A — 기능 사용 통계 기반 deprecate (보수적)
- 모든 MCP/Tauri command 호출 카운터 (audit_trace 누적)
- 30일+ 미사용 → deprecate.md 단방향 위임 (lesson 49 패턴)
- 사용자 알림 후 60일 유예 → 삭제
- **위험 0** (사용 흔적 기반)

### 옵션 B — 카테고리 통합 (적극적)
- MCP 28 → 12로 통합 (`setup_*` 6개 → 1 메타 도구 / `kg_*` 3개 → 1 등)
- `optimize` 같은 메타 도구 패턴 확장 (Phase 102 패턴)
- Tauri commands 67 → 30 (UI 표면 동시 단순화)
- **위험 중** (API 호환성 깨짐)

### 옵션 C — 진입 흐름 재설계 (UX 중심)
- 🧭 4-step 온보드 (Phase 106)을 메인 진입으로 강화
- 6탭 → 3탭 (기본/고급/설정) IA 재설계
- 고급 기능은 "고급" 탭에 숨기기 (인지 부담 ↓)
- **위험 중** (lesson 20/21 IA 재설계 경험 활용)

### 옵션 D — 외부 분리 (구조적)
- 검색 분리 (Phase 108~115, `_rust_module/`로 약 4,200줄 이관)
- file-pipeline = 가공+추천+검증 전담
- MCP 도구 자연 분리 → 본 솔루션 도구 수 감소
- **위험 큼** (대형 phase, search-extraction-plan.md §3 8논점 합의 필요)

## 9. Q1~Q3

**Q1** (정리 우선순위) — §8 옵션 A/B/C/D 중 (a) **A 단독 (사용 통계 누적 측정 인프라 추가)**, (b) **A + C 결합 (사용 통계 + UX 재설계)**, (c) **D 단독 (검색 분리로 자연 분리)**, (d) **위 셋 보류, 본 분석만 활용** — 어느 게 맞습니까?

**Q2** (기능 과다 임계) — 28 MCP + 67 Tauri + 6 탭이 "과다"의 임계입니까, 아니면 **이미 임계를 넘은 상태**입니까? 후자라면 더 적극적 정리(옵션 B/D)가 필요합니다.

**Q3** (외부 흡수 정책 조정) — 본 비교에서 7건 누적 흡수로 표면적 비대를 확인. 다음 외부 분석부터는 (a) **본질 일치만 흡수, 부수 영역 보류 강화**, (b) **현 정책 유지 (lesson 30 인프라 선구현)**, (c) **신규 흡수 일시 동결, 정리 완료 후 재개** — 정책 변경이 필요합니까?
