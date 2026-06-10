---
created: 2026-06-04
phase: E1+E2+E3 (Grimoire 3 MCP 도구 흡수)
external_source: github.com/alanhakhyeonsong/grimoire
prd_truth: prd/research/external-analysis-2026-06-04-grimoire.md
meta_rules:
  - 메타 룰 21 (외부 도메인 도구 본질/부수) — 4건째
  - 메타 룰 18 (추정 빗나감) — 11번째 누적
  - 메타 룰 16 차원 A 🟡 + 차원 B 🟡 (정반대 정책 — no-embedding)
---

# Lesson 70 — Grimoire 3 MCP 도구 흡수 (Phase E)

## 상황

사용자 요청 "alanhakhyeonsong/grimoire 분석 + 고도화 방안". Go 단일 바이너리 + SQLite FTS5 + no-embedding RAG-lite. 9 MCP 도구 중 3건 (`get_index` / `write_note` / `get_context`) 본 솔루션 부재 영역으로 즉시 흡수 결정.

## 문제

본 솔루션 현재 28 MCP 도구는 검색·KG·설정·signal·todo·snapshot·lint 7 카테고리 보유하나, **"코퍼스 전체 라우팅"** 영역 도구가 없음. Claude가 `search` 호출 전 사전 라우팅 불가 → search noise.

## 원인

- `list_documents`는 단순 리스트, hierarchy/카테고리 그룹화 없음
- `stats`는 카운터만, 구조 없음
- `optimize`는 추천만, 라우팅 아님
- Grimoire `get_index` 패턴 = 본 솔루션 빈 영역과 정확히 매칭

## 개선

### Phase E1: get_index
- `handle_get_index` — doc_type / date 그룹화 + top_per_group + count
- 카탈로그 Search 카테고리, Free cost, mutates=false

### Phase E2: write_note (dry-run 인프라)
- `handle_write_note` — title/content/type/domain → 분류규약 역매핑 (휴리스틱 slug 생성)
- 실제 저장은 Phase E2-stage2 (setup_rules.toml 통합 후) — 현재는 suggested_path 반환만
- 카탈로그 Search 카테고리, Free cost, mutates=true

### Phase E3: get_context
- `handle_get_context` — cwd 마지막 컴포넌트 키워드 매칭 (path*3 / type*2)
- 상위 10건 + 점수 반환
- 카탈로그 Search 카테고리, Free cost, mutates=false

### 카탈로그 25→28
- `mcp_tool_catalog_full()` 3 항목 추가
- 5 단위 테스트 추가 (E1+E2+E3 등록 / count / read-only / mutates)
- 단일 카탈로그 자동 파생 (Phase 92 H3 단일 진실원 유지)

## 사이드 발견 — 메타 룰 18 11번째 누적

**StoredDocSummary에 `hierarchy` 필드 부재** (Metadata에만 존재. list_all은 summary만 반환). 추정으로 `doc.hierarchy.is_empty()` 호출 → 컴파일 에러 E0609 3건. 즉시 해소: hierarchy → date 그룹화로 대체.

### 다음 phase 회피 의무
- 신규 MCP 핸들러 작성 전 `StoredDocSummary` 필드 grep 의무
- 추정 빗나감 11번째 누적 → 메타 룰 18 본문 "추정 빗나감 비율 100%" 강화 표기

## 메타 룰 적용

| 메타 룰 | 적용 |
|---------|------|
| 21 (본질/부수 도메인) | **4건째 누적** (TFM + Mirage + GraphRAG + Grimoire) — 정식 승격은 이미 Phase 103 |
| 18 (추정 빗나감) | 11번째 누적 (StoredDocSummary 필드) |
| 9 (외부 출처 단일 진실원) | `external-analysis-2026-06-04-grimoire.md` 신규 |
| 13 (4단계) | E1/E3 = 1+2단계, E2 = 1단계만 (실제 저장 stage2 후속) |
| 22 (사용자 정책 합의) | 9번째 (흡수 범위 결정 4 옵션) |

## 보류 (메타 룰 21 🔴 영구)

- N1: 임베딩 제거 (본 솔루션 핵심 가치와 정반대)
- N2: 마크다운만 입력 (PDF/Excel/한글 가공 본질 위반)
- N3: Go 단일 바이너리 (Rust로 이미 충족)
- N4: Ollama frontmatter 백필 (LLM 2-Pass가 상위)
