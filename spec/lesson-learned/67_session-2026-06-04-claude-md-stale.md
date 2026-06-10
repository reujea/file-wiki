---
created: 2026-06-04
phase: session-2026-06-04 (CLAUDE.md stale 정리)
meta_rules:
  - 메타 룰 1 sub-rule 1g (spec 자기 위반 누적)
  - 메타 룰 19 (단일 진실원 위임)
related:
  - src/CLAUDE.md (정리 대상)
  - spec/lesson-learned/49_spec-self-duplication-single-source.md (전례)
---

# Lesson 67 — CLAUDE.md도 spec 단일 진실원 위반자

## 상황

사용자 "spec 폴더 분석" 요청으로 spec/ 전수 점검 중 `src/CLAUDE.md` 4건 stale 발견:

| 위치 | stale 사실 |
|------|-----------|
| L86 헥사고날 원칙 | `core → adapters/infrastructure 참조 금지` — `infrastructure` 크레이트 비실재 |
| L106 shared 설명 | `config, qdrant_manager, build_service` — qdrant_manager는 Phase 65 제거됨 |
| L109 modals/mcp | 별도 모달로 표기 — shared/mcp_server.rs로 통합되어 모달 없음 |
| L111 vendor | `qdrant.exe` 명시 — Phase 64 트리거 #11/#12 onnxruntime 폐기 후 빈 디렉토리 |

## 문제

lesson 49 메타 룰 1 sub-rule 1g (spec 문서 자기 위반)이 spec/ 내부에 한정된 것으로 인식. **CLAUDE.md도 spec 진실원 영역**임을 미인식. 결과로 신규 세션 시작 시 사용자가 잘못된 구조 인식 → 분석 정확도 ↓.

## 원인

1. **메타 룰 30 후보 (phase 종결 시 spec 본문 즉시 갱신 의무) 자기 적용 영역에 CLAUDE.md 미포함**
2. **comm-spec 영역 확장 인식 부족** — `spec/` 디렉토리만 spec 영역으로 인식했으나 `src/CLAUDE.md`도 사실상 spec (Claude 전용 컨텍스트)
3. Phase 65 (Qdrant 제거) / Phase 102 (MCP 통합) 등 변경 시점에 CLAUDE.md 미갱신 stale 누적

## 개선

### 즉시 적용 (본 세션)
- CLAUDE.md L86/L106/L109/L111 4건 수정 — 실제 구조 반영
- `_rust_module/` workspace 16종 + `adapters` 8 모듈 참조 / `shared` 2 모듈 참조 명시 추가

### 메타 룰 확장
- **메타 룰 30 후보 자기 적용 영역에 `src/CLAUDE.md` 추가** — phase 종결 시 architecture.md / domain-map.md / webapp-design.md / **CLAUDE.md** 4건 grep 의무
- **메타 룰 1 sub-rule 1g 확장** — spec 자기 위반 영역에 `src/CLAUDE.md` 포함

### 메타 룰 30 정식 승격 조건 갱신
| 조건 | 본 lesson 후 |
|------|------------|
| 누적 ≥ 3건 | **4건 도달** (lesson 64 / 65 / 66 / **67**) ✅ |
| 체크리스트 | "phase 종결 직전 영향 영역 grep + 즉시 갱신 — `src/CLAUDE.md` 포함" 강화 |
| META.md 본문 등재 | ✅ (Phase 106 이미 등재) |

→ **다음 phase에서 메타 룰 30 정식 승격 의무** (조건 모두 충족).
