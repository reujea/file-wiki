---
created: 2026-06-04
purpose: Grimoire (alanhakhyeonsong/grimoire) 외부 분석 단일 진실원
external_sources:
  - repo: "github.com/alanhakhyeonsong/grimoire"
  - language: "Go (단일 바이너리, cgo 불필요)"
  - mcp_sdk: "modelcontextprotocol/go-sdk v1.6.1"
  - index: "modernc.org/sqlite FTS5"
  - license: "미정 (개인 프로젝트)"
related:
  - spec/lesson-learned/META.md (메타 룰 20 + 메타 룰 21)
  - prd/research/external-analysis-2026-06-04-adaptive-chunking.md (직전 외부 분석)
  - prd/research/external-analysis-2026-05-27-graphrag.md (직전 메타 룰 21 사례)
meta_rule_label:
  dimension_a: "🟡 부분 도메인 일치 (RAG-lite, 임베딩 없음)"
  dimension_b: "🟡 부분 일치 + 도구 추가 패턴 (mode 분기 없음)"
  classification: "메타 룰 21 누적 4건째 (TFM + Mirage + GraphRAG + Grimoire)"
status: "Phase E1+E2+E3 흡수 진입 (2026-06-04 본 세션)"
---

# Grimoire (Local-first MCP KB) 흡수 분석

## 0. 본 문서 위상

본 문서는 메타 룰 9 "외부 출처 단일 진실원" 자기 적용. Grimoire 흡수 결정·기준·진척의 단일 진실원이며, 본 외부 자료를 인용하는 lesson/spec/코드 주석은 본 파일 링크 (`prd/research/external-analysis-2026-06-04-grimoire.md`)만 참조한다.

메타 룰 21 누적 4건째 — TFM/TabPFN + Mirage v0.0.1 + AWS GraphRAG에 이어 다른 도메인 도구 부수 흡수 사례. 본질 도메인(RAG)은 같으나 핵심 정책(임베딩 사용 vs 제외)이 정반대라 메타 룰 20 본질 일치보다 메타 룰 21 부수 흡수가 적합.

## 1. 본질 정렬 — 정반대 정책

| 측면 | Grimoire | file-pipeline |
|------|---------|---------------|
| 본질 | 마크다운 KB 직접 라우팅 (no-embedding) | 임의 파일 가공 → hybrid 검색 |
| 입력 | 사용자 마크다운 (Obsidian/git 호환) | PDF/Excel/한글/마크다운 등 임의 |
| 인덱스 | SQLite FTS5 (키워드만) | LocalVectorStore HNSW (dense + sparse) |
| 가공 | 없음 — 원본 그대로 | LLM 2-Pass (classify + verify) |
| 검색 라우팅 | Claude 발화 의존 | hybrid (vector + keyword + reranker) |
| MCP 도구 수 | 9 | 25 |
| 라이선스 | 미정 | (본 프로젝트 별도) |

**메타 룰 16 차원 A/B**: 🟡 / 🟡 — 같은 도메인이지만 정책 정반대. 본질이 아닌 부수 흡수 후보만.

## 2. Grimoire 9 도구 ↔ file-pipeline 25 도구 매핑

| Grimoire | file-pipeline | 격차 / 흡수 결정 |
|----------|---------------|---------------|
| `search` (FTS5) | `search` (hybrid) | ✅ 본 솔루션 우월 — 흡수 불필요 |
| `get_index` (목차) | **없음** | 🟢 **E1 흡수** — Claude 사전 라우팅 가치 |
| `read_note` | `get_document` | ✅ 동일 |
| `links` (위키링크 그래프) | `kg_neighbors` / `kg_paths` | ✅ 본 솔루션 우월 — KG가 상위 추상 |
| `write_note` (분류규약 저장) | **없음** | 🟢 **E2 흡수** — setup_rules.toml 활용 |
| `get_context` (cwd→project) | **없음** | 🟢 **E3 흡수** — Phase 106 온보드 결합 |
| `get_runbook` | `list_topics` / `get_topic` | 🟡 부분 일치 — 보류 |
| `lint` | `lint` MCP + `lint_strong_claims` | ✅ 동일 영역 |
| `suggest_frontmatter` (Ollama) | `optimize` (C1) | 🟡 다른 결 — 보류 |

## 3. Grimoire 5 원칙 ↔ 본 솔루션 정책

| Grimoire 원칙 | file-pipeline 현재 | 변경 필요? |
|--------------|--------------------|----------|
| 1. 원본 보존 (복제 금지) | 🟡 `processed/` 가공본 사본 생성 | 본질 보존 — 변경 안 함 |
| 2. 임베딩 제외 | 🔴 정반대 | 본질 보존 — N1 보류 |
| 3. 접근 경계 명확화 | 🟡 `sensitive/` 격리. frontmatter 토글 없음 | E4 후보 (mode 분기) |
| 4. Config 외재화 | ✅ pipeline.toml + setup_rules + doc_types + prompts | 이미 충족 |
| 5. 점진적 강화 (frontmatter 활용) | 🟡 LLM 추론으로 대체. frontmatter 미파싱 | E4 후보 |

## 4. 흡수 후보 — 5건 분류

### 🟢 본 세션 즉시 흡수 (Phase E)

| ID | 항목 | 위치 | 위험 |
|----|------|------|------|
| **E1** | `get_index` MCP | `shared/mcp_server.rs::handle_get_index` + Tauri | 0 (read-only) |
| **E2** | `write_note` MCP | `shared/mcp_server.rs::handle_write_note` + setup_rules 역매핑 | 낮음 (mutates, 디폴트 비활성) |
| **E3** | `get_context` MCP | `shared/mcp_server.rs::handle_get_context` + cwd 추론 | 0 (read-only) |

### 🟡 보류 후보 (다음 phase)

| ID | 항목 | 결정 트리거 |
|----|------|----------|
| **E4** | frontmatter 사전 파싱 + `ai_access:shared` 토글 | Markdown 비중 높은 코퍼스 도달 시 |
| **E5** | mtime 60초 주기 인덱싱 | watcher 기반 즉시 인덱싱이 이미 충족 — 보류 영구 후보 |

### 🔴 도메인 불일치 (영구 보류 — 메타 룰 21 §🔴)

| ID | 항목 | 사유 |
|----|------|------|
| **N1** | 임베딩 제거 | hybrid 검색이 본 솔루션 핵심 가치 — 정반대 정책 |
| **N2** | "마크다운만" 입력 정책 | PDF/Excel/한글 가공 본질 영역 |
| **N3** | Go 단일 바이너리 변환 | Rust 단일 바이너리로 이미 충족 |
| **N4** | Ollama frontmatter 백필 전용 모드 | LLM 2-Pass 가공이 상위 — 통합 가치 낮음 |

## 5. Phase E 진행 매트릭스 (lesson 30 패턴)

| Phase | 영역 | 디폴트 | 트리거 |
|-------|------|--------|--------|
| **E1** | `get_index` MCP | 항상 가용 | 본 세션 흡수 (위험 0) |
| **E2** | `write_note` MCP | 비활성 (mutates) | 본 세션 인프라 + 사용자 명시 활성화 |
| **E3** | `get_context` MCP | 활성 | 본 세션 흡수 (read-only) |
| **E4** | frontmatter 사전 파싱 | 비활성 | Markdown 비중 도달 시 |
| **E5** | (영구 보류) | - | - |

## 6. 메타 룰 21 누적 사례 표 갱신

`spec/lesson-learned/META.md` §메타 룰 21 누적 표에 4건째 추가:

| 외부 도구 | 본질 도메인 | file-pipeline 일치 | 부수 일치 흡수 | Phase |
|---------|----------|------------------|-------------|------|
| TabPFN / TFM | 숫자 테이블 예측 | 없음 | 이상 탐지 / ETA 예측 | (분석만) |
| Mirage v0.0.1 | AI 에이전트 VFS | 없음 | MCP 카탈로그 다차원 / Resource capabilities | Phase 92 H3/H5 |
| AWS GraphRAG Toolkit | 엔터프라이즈 RAG (AWS) | 없음 (단일 사용자 데스크톱) | G4 TF-IDF / G1 Statement / G2 Semantic / G3 Multi-hop | Phase 103 |
| **Grimoire** | **마크다운 KB no-embedding** | **부분 (RAG-lite)** | **E1 get_index / E2 write_note / E3 get_context** | **Phase E (본 세션)** |

## 7. 본 분석의 메타 가치

- **메타 룰 21 4건 누적 도달** — 본질 일치/부수 일치/명시 보류 3 영역 분류가 안정 패턴화
- **lesson 30 인프라 선구현 자기 적용** — Phase E1/E2/E3 모두 디폴트 비활성 또는 read-only
- **메타 룰 13 4단계 자기 적용** — E1/E3는 1+2단계 즉시 도달 (인프라 + 로직), E2는 1단계만 (사용자 활성화 대기)
- **메타 룰 1 sub-rule 1f (단일 진입점) 위험** — MCP 도구 3건 추가 시 mcp_tool_catalog + Tauri commands + frontend 4계층 동기화 의무 (lesson 32 패턴 재적용)

## 8. 측정 트리거

- E1 `get_index` 활용도: 사용자 신호 (검색 precision 향상 체감)
- E2 `write_note` 활용도: Claude 발화로 노트 작성 빈도
- E3 `get_context` 적중률: cwd 매칭 정확도 측정 (다음 phase)

다음 phase 트리거: `setup_modules` 12종에 "grimoire-style routing" 신규 추가 검토.
