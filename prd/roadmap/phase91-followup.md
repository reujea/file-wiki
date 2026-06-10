---
created: 2026-05-22
updated: 2026-05-22 (Phase 92~95 완료 반영 — 모든 항목 완료/진행/이관됨)
phase_context: Phase 91 (JAMES v0.3.0 패턴 흡수 5건 묶음) 완료 직후 → Phase 95까지 누적 완료
purpose: Phase 91 후속 작업 우선순위 정리 (역사적 기록 + 완료 표시)
status: ✅ **모두 종결됨** — Phase 92~95에서 진행 완료 또는 별도 후보로 이관
successor: prd/roadmap/external-trigger-checklist.md (외부 의존) + prd/roadmap/next-session-c-items.md (다음 세션 C 항목)
---

# Phase 91 후속 작업 로드맵 (✅ 종결)

Phase 91 (2026-05-21) 완료 시점 기준 후속 작업. Phase 92~95에서 모두 완료/진행/이관됨.

## 종결 요약 (2026-05-22)

| 직전 항목 | 결정 | 진행 phase |
|---------|------|----------|
| P0 1번 Tauri release 재빌드 | ✅ 완료 | Phase 93 (21m 41s) + 95 (5m 47s incremental) |
| P0 2번 A3 trace_id 호출처 부착 | ✅ 완료 | Phase 94 (5 호출처) + 95 (총 9 호출처) |
| P0 3번 B2 GUI Settings 카드 | ✅ 완료 | Phase 93 (MCP 다차원 카탈로그) |
| P0 4번 A2 PII mask GUI 토글 | ✅ 완료 | Phase 93 (Settings 카드) |
| P1 5번 메타 룰 1 sub-rule 분리 | ✅ 완료 | Phase 94 (7 카테고리 1a~1g) |
| P1 6번 메타 룰 19 정식 승격 | ✅ 완료 | Phase 94 (누적 5건) |
| P1 7번 메타 룰 20 후보 등록 | ✅ 완료 | Phase 92 META 정식 승격 (누적 4건 도달) |
| P1 8번 메타 룰 17 자동화 | ⏳ 이관 | next-session-c-items.md C-7 |
| P2 9번 F1 추정 키워드 grep | ⏳ 이관 | next-session-c-items.md C-3 |
| P2 10번 F2 메타 룰 19 잠재 점검 | ⏳ 이관 | next-session-c-items.md C-4 |
| P2 11번 F3 single_source_check.sh | ⏳ 보류 (메타 룰 19 정식 승격으로 우선순위 낮음) | — |
| P2 12번 F5 lesson 47 v3 (CSS scanner) | ⏳ 이관 | next-session-c-items.md C-6 |
| P2 13번 F6 benchmarks 아카이빙 | ⏳ 이관 | next-session-c-items.md C-5 |
| P2 14번 F7 lesson 33 메타 룰 후보 | ⏳ 보류 (메타 룰 후보 23/24 등 신규 등록 우선) | — |
| P3 15번 #10 BGE-M3 Sparse | 트리거 대기 | external-trigger-checklist.md B-4 변형 |
| P3 16번 webapp-design.md 분리 | ⏳ 이관 | next-session-c-items.md C-8 |

## 후속 phase 매핑

- **Phase 92** (2026-05-22): JAMES 재검증 + Mirage 흡수 (H3/H5/H1 + 메타 룰 20 META) — lesson 51
- **Phase 93** (2026-05-22): GUI 가시화 4건 묶음 (P0 3·4) — lesson 52
- **Phase 94** (2026-05-22): AuditPort 헥사고날 + 메타 정형화 (P0 2 + P1 5·6) — lesson 53
- **Phase 95** (2026-05-22): trace_id 영역 확장 (P0 2 후속) — lesson 54

## 이관 문서

- `prd/roadmap/external-trigger-checklist.md` — 외부 테스트 / 데이터 누적 트리거 의존 항목 (A 사용자 + B 데이터 누적)
- `prd/roadmap/next-session-c-items.md` — 다음 세션 C 항목 9건 구현 계획

---

## (이하 원본 본문 보존 — 역사적 기록)

Phase 91 (2026-05-21) 완료 시점 기준 다음 작업 항목 정리. RBAC/외부 협업/외부 연계는 보류 정책 유지.

## P0 — 즉시 진행 가능 🟢

### 1. Tauri release 재빌드 (메타 룰 17 의무)

- **트리거**: Phase 91에서 `mcp_server.rs` / `commands.rs` / `cli.rs` / `main.rs` 변경 → release 재빌드 의무
- **작업**: `cd src/modals/app && cargo build --release` (소요 7~30분)
- **가치**: 메타 룰 17 위반 차단 (Phase 90 G-3 사례 재발 방지)
- **측정**: 빌드 통과 + GUI 시각 확인

### 2. A3 trace_id 호출처 부착 (메타 룰 13 2단계)

- **현재**: Phase 91에서 인프라(테이블 + TraceId + replay_trace.sh)만 추가 — 4단계 중 1단계 완료
- **작업**: LLM 호출 / 검색 / MCP 핸들러 / Notion 어댑터에 `trace_id` 주입 + `record_audit_event` 호출
- **가치**: 메타 룰 13 4단계 중 2단계 도달. lesson 7 분기 누락 검출 인프라 활성화
- **비용**: 중간 (핫패스 5~10곳)
- **측정**: trace 미부착 핫패스 grep 0건

### 3. B2 GUI Settings 카드 `mutates_state` 분류 표시

- **현재**: `mcp_tool_catalog` 백엔드만. GUI 표시 없음
- **작업**: `ui/dashboard.js` Settings 탭에 MCP 도구 위험 분류 카드 추가 (24 도구, 6 mutating에 ⚠ 표시)
- **가치**: B2 가시화 완성 (메타 룰 13 4단계 중 4단계)
- **비용**: 작음

### 4. A2 PII mask 사용자 토글 GUI

- **현재**: `SearchConfig.output_pii_mask` 디폴트 true. GUI 토글 없음
- **작업**: Settings 카드에 체크박스 + save_config 연동 (lesson 12 secret 복원 패턴)
- **가치**: 사용자 가시화
- **비용**: 작음

## P1 — 메타 정형화 🟡

### 5. META.md 메타 룰 1의 17건 sub-rule 분리

- **트리거**: Phase 91에서 15→17건 도달. 가독성 임계
- **분리 후보**:
  - 1a: UI 제거 패턴 (lesson 13/19/19+/47)
  - 1b: 구조체 필드 추가 (lesson 21/27/35)
  - 1c: DB 스키마 (lesson 10/26)
  - 1d: 미연결 포트 (lesson 14/15)
  - 1e: 직렬화 4계층 (lesson 32/42)
  - 1f: 함수/검사 분산 (lesson 38/50-A/50-B)
  - 1g: spec 자기 위반 (lesson 49)
- **가치**: 신규 작업 사전 체크리스트 가독성
- **비용**: META.md 핵심 구조 변경. INDEX.md / 개별 lesson 본문 일괄 재매핑 의무

### 6. 메타 룰 19 (단일 진실원 위임) 정식 승격

- **현재**: 후보. 누적 사례 lesson 49 + Phase 91 classifier.rs/Verifier + 메타 룰 16 차원 B
- **작업**: META.md "메타 룰 19 후보" → "메타 룰 19" 승격. 3축 분리 + 3요소 동반 명문화
- **가치**: 메타 룰 17/18처럼 정식 룰로 사전 체크 적용

### 7. 메타 룰 20 후보 등록 — "외부 프로젝트 패턴 흡수 시 도메인 가정 정렬"

- **트리거**: lesson 50에 등록만 됨. META.md 정식 후보 등록 미진행
- **작업**: META.md 끝에 메타 룰 20 후보 섹션 추가
- **누적 사례**: JAMES (lesson 50)

### 8. 메타 룰 17 자동화 (release 재빌드 git diff 감지)

- **현재**: 메타 룰 17 정식 승격. 자동화는 후보로만 등록
- **작업**: `git diff --name-only HEAD | grep -E '\.rs$|ui/.*\.(js|css|html)$'` → phase 종결 hook 통합
- **가치**: 본 P0 1번 자기 적용 — release 재빌드 누락 자동 차단
- **제약**: git 미저장 환경에서 적용 제한

## P2 — 직전 누적 미진행 🟡

### 9. F1 추정 키워드 grep 재검증 (메타 룰 18 자기 적용)

- 직전 P0. lesson 49 본문에 등록만 됨. 실 grep 미진행
- 작업: `grep -nE '추정|것으로 보임|불명|likely|suspect' spec/lesson-learned/*.md` → 1건 이상 재검증
- Phase 91에서 추정 빗나감 3/3 추가됨 → 더 강한 자기 적용 필요

### 10. F2 메타 룰 19 잠재 적용 후보 점검

- 2a (좁은): domain-map.md ↔ architecture.md 포트 매핑 중복 — 30분
- 2b (중간): architecture.md ↔ webapp-design.md 7탭/노드 수치
- 2c (넓은): scenarios.md 포함 3 파일 grep

### 11. F3 single_source_check.sh 자동화 스크립트

- 메타 룰 19 검증 grep 자동화 (G-5 **7번째** 스크립트, replay_trace.sh가 6번째 선점)

### 12. F5 lesson 47 v3 (AST + CSS rule scanner)

- 현재 v2 (acorn AST). CSS rule scan 미적용

### 13. F6 benchmarks/ JSON 125개 Phase별 아카이빙

- `benchmarks/archive/phase-{NN}/` 분리

### 14. F7 lesson 33 → 메타 룰 후보 ("configFields toml 섹션 1:1")

- 누적 3건 (lesson 22/23/33)

## P3 — 인프라 (변경 없음) 🟡

### 15. #10 BGE-M3 Sparse LocalVectorStore 완전 통합

- 트리거 대기 (측정 코퍼스 도달 시)

### 16. webapp-design.md 분리

- 사용자 결정 영역

## 추천 진행 순서

**즉시 (이번 세션)**: 1번 Tauri release 재빌드 — 메타 룰 17 의무

**단기 (1~2세션)**:
- 2번 A3 trace_id 호출처 부착 (메타 룰 13 진척)
- 3+4번 GUI 표시 묶음 (둘 다 ui/dashboard.js Settings 카드)

**중기 (메타 정형화 묶음)**:
- 5~8번: 메타 룰 1 sub-rule 분리 / 메타 룰 19 승격 / 메타 룰 20 등록 / 메타 룰 17 자동화

**후속**: 9~14번 (위생 + 자동화)

**별도 phase**: 15번 #10 Sparse 완전 통합

**사용자 결정**: 16번 webapp-design.md 분리

## 메타 룰 16 자기 적용 (🟢🟡🔴)

🔴 항목 0건. 모든 항목 자체 진행 가능 영역 (외부 협업/연계 보류 정책 준수).

| 항목 | 라벨 |
|------|------|
| 1~4 P0 | 🟢 |
| 5~8 메타 정형화 | 🟢 |
| 9~14 위생 | 🟢 |
| 15 #10 Sparse | 🟡 (측정 코퍼스 대기) |
| 16 webapp 분리 | 🟢 (사용자 결정 후) |
