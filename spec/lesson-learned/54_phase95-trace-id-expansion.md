---
phase: 95
date: 2026-05-22
topics: trace_id 부착 영역 확장 4건 / 메타 룰 13 2단계 완성도 / stage 명명 규칙 / KgQueryResult 추정 빗나감 차단
related_lessons: 50, 51, 52, 53
related_meta_rules: 1, 13, 17, 18, 19
---

# 54. Phase 95 — trace_id 부착 영역 확장 + release 재빌드

## 상황

Phase 94 AuditPort 인프라 + 5 호출처 부착 후 메타 룰 13 2단계 부분 완성. 후속 영역 확장:
1. Tauri search (commands.rs::search) — GUI 호출 audit
2. MCP kg_neighbors / kg_paths — 지식 그래프 쿼리 audit
3. 원격 저장소 upload — service.rs 4 sub-branch (processed/origin × 성공/실패)

Phase 94 누적 변경 반영 release 재빌드 (메타 룰 17 의무).

## 문제

### 문제 1: stage 명명 규칙 부재

Phase 94에서 임의로 `llm.classify` / `mcp.search` 사용. Phase 95에서 영역 4건 추가 시 일관성 필요.

**해소**: `{영역}.{도구명}.{sub}` 규칙 정형화.
- 영역: `llm` / `search` / `mcp` / `tauri` / `remote`
- 도구명: 단일 행위 (classify / search / kg_neighbors / kg_paths / upload)
- sub (옵션): 분기 (text/file, processed/origin, cached, ok/err)

### 문제 2: KgQueryResult 반환 타입 추정 빗나감 (메타 룰 18)

`find_paths` 결과를 `result.len()` 추정. 실제 `KgQueryResult { nodes, edges, paths }` 구조체. **사전 검증 grep으로 7번째 빗나감 차단**:
- Phase 91 lesson 50: service.rs:235 (active 추정)
- Phase 91~92 추가 사례 4건
- Phase 93: `state.paths` (filed 추정)
- Phase 94: `state.paths.base.join` 추정
- **Phase 95: `KgQueryResult` Vec 추정** ← 새 차단

### 문제 3: 원격 저장소 4 sub-branch audit

`remote_storage.upload` 호출이 service.rs:632 (processed) + 638 (origin) — 2 위치. 각각 성공·실패 분기 → 4 audit 호출.

분기마다 backend 동적 (capability.backend) 필요 — Notion/S3/WebDAV/Network 구분 가능.

## 원인

### 직접 원인

- Phase 94에서 stage 명명을 사전 정형화하지 않고 임의 사용 — 영역 확장 시 일관성 부담
- `find_paths` 반환 타입을 사전 grep 없이 추정 (메타 룰 18 강화 직후에도 발생 가능성 인식)

### 구조적 원인

- 영역 확장은 phase별 incremental — 매 phase 사전 검증 의무 명문화 필요
- 추정 빗나감 7건 누적은 메타 룰 18의 핵심 신호 — "추정은 100% 빗나간다" 정립

## 개선

### 개선 1 — 4 신규 호출처 부착

```rust
// commands.rs::search (Tauri)
let trace = file_pipeline_core::audit::TraceId::new();
let inputs_hash = file_pipeline_core::audit::input_hash_prefix(params.query.as_bytes());
// ... 검색 로직 ...
let summary = file_pipeline_core::audit::truncate_output_summary(...);
state.service.audit.record(trace.as_str(), "tauri.search", Some(&inputs_hash), Some(&summary), Some("success"));

// mcp_server.rs::handle_kg_neighbors / handle_kg_paths
// 동일 패턴, stage "mcp.kg_neighbors" / "mcp.kg_paths"

// service.rs::process_file_with_pipeline (원격 업로드)
let backend = self.remote_storage.capabilities().backend;
// match upload result ... stage "remote.{backend}.upload.processed" / origin
```

### 개선 2 — stage 명명 규칙 정형화

`{영역}.{도구명}.{sub?}` 패턴:

| 영역 | 의미 | 예 |
|------|------|---|
| `llm` | LLM 호출 | `llm.classify` |
| `mcp` | MCP 도구 | `mcp.search` / `mcp.search.cached` / `mcp.kg_neighbors` |
| `tauri` | Tauri command | `tauri.search` |
| `remote` | 원격 저장소 (backend 동적) | `remote.s3.upload.processed` / `remote.notion.upload.origin` |

향후 신규 호출처 추가 시 본 규칙 준수 의무 — META.md 추가 후보.

### 개선 3 — KgQueryResult 추정 빗나감 차단 (메타 룰 18 자기 적용)

빌드 에러 즉시 grep으로 실제 타입 확인 → `nodes/edges/paths` 3 필드 발견 → summary 형식 정정. 메타 룰 18의 "추정 사전 grep 의무" 7번째 적용.

### 개선 4 — Release 재빌드 메타 룰 17 자기 적용

Phase 94 service.rs + mcp_server.rs + modals/app/service.rs 변경 반영. workspace release 2m 13s. Tauri release는 백그라운드 진행 (이전 phase 21m 41s 선례).

## 공통 교훈

1. **추정 빗나감 7건 누적 — "추정 사전 grep 의무"의 의미** — 메타 룰 18 강화 직후에도 빗나감 발생. **본인 추정은 100% 빗나간다**는 패턴 확정
2. **stage 명명 규칙 사전 정형화 의무** — phase별 영역 확장 시 일관성 부담 회피. META 추가 후보
3. **backend 동적 stage 명** — 원격 저장소 패턴은 capability.backend 사용. 신규 어댑터 추가 시 자동 분류
4. **incremental release 빌드** — Phase 94 빌드 후 Phase 95 변경분만 추가 시간

## 메타 룰 자기 적용

| 룰 | Phase 95 적용 |
|----|--------------|
| 메타 룰 1 sub-rule 1f | trace_id 부착 = 단일 `AuditPort` trait → 9 호출처 통일 |
| 메타 룰 13 | 2단계 부분 완성도 향상 — 9 핫패스 부착 |
| 메타 룰 17 | Phase 94 변경 반영 재빌드 (자기 적용) |
| 메타 룰 18 | KgQueryResult 추정 7번째 차단 (빌드 에러로 즉시 발견) |
| 메타 룰 19 | stage 명명 단일 규칙 (단일 진실원 위임) |

## 메타 룰 24 후보 등록 (Phase 95 신규)

"audit/log/trace stage 명명 규칙 정형화":
- 패턴: `{영역}.{도구명}.{sub?}`
- 영역 enum: llm / mcp / tauri / remote / service / verify
- sub: 분기 표시 (text/file, cached, ok/err)
- 신규 호출처 추가 시 본 규칙 grep 검증

1건 누적 (본 phase 자기 적용).

## 다음 세션 플래그

- [ ] Tauri release 빌드 결과 확인 (백그라운드 완료 알림 대기)
- [ ] audit_trace 실측 (H1 주기 호출 누적 50건+ 도달 후 lesson 46 G-1 root cause 시도)
- [ ] Notion 어댑터 자체 audit (현재 service.rs 호출 시점만, attach/download 등 추가 영역)
- [ ] 메타 룰 24 후보 1건 추가 누적 시 META 등록 (현재 1건)
- [ ] 메타 룰 22 후보(사용자 정책 경계) 1건 추가 누적
- [ ] 메타 룰 1 sub-rule 1f에 누적 사례 표 명시 추가
