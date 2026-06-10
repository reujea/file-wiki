# Lesson Learned: Phase 97 A3 영역 완성 + 자동화 2건 (C-1 + C-9 + C-7)

## 상황

2026-05-26 Phase 97 진행. 사용자 명시 "Q1 진행" → next-session-c-items.md 2차 묶음 (C-1 + C-9 + C-7) 선택. Phase 96 메타 자기 적용 직후 코드 변경 phase 진입. 메타 룰 17 재빌드 의무 발동.

직전 Phase 95에서 trace_id 부착 7 호출처 완성 (Tauri search + MCP kg + 원격 업로드). 잔여 영역 3건 발견 (handle_get_document / handle_list_documents / verify_reprocess).

## 문제

3개 영역에서 메타 룰 13 2단계 완성도 향상 + 자동화 누락:

### 문제 1 — A3 trace_id 부착 영역 3건 잔여 (C-1)

Phase 95에서 7 호출처 부착 완료 후 메타 룰 13 2단계 완성도 향상이라 표기했으나 실제로는:
- mcp_server.rs `handle_get_document` / `handle_list_documents` — MCP 검색 흐름의 진입점이지만 trace_id 미부착
- service.rs `reprocess_with_feedback` — 2-Pass 검증 재가공 LLM 호출 시점이지만 미부착

→ 메타 룰 13 2단계 완성도 100% 미달 상태였음.

### 문제 2 — stage 명명 규칙 검증 자동화 부재 (C-9)

Phase 95에서 메타 룰 24 후보(`{영역}.{도구명}.{sub?}`) 등록 했지만 신규 stage 추가 시 명명 규칙 위반 검출 수단 없음. 메타 룰 1 (다중 위치 동기화 누락) 변형 위험 — 새 stage가 임의 명명으로 진입하면 audit_trace 분석 시 영역별 그룹화 깨짐.

### 문제 3 — release 재빌드 의무 자동 감지 부재 (C-7)

메타 룰 17 정식 승격(Phase 95) 했지만 phase 종결 시 재빌드 필요 여부를 사람이 매번 판단. lesson 46 G-3(Phase 90 Notion 재빌드 누락) 같은 실수 재발 위험.

### 문제 4 — verify reprocess 스코프 추정 빗나감 (8번째 누적)

C-1 진행 중 `service.rs:475` reprocess_with_feedback 위치의 `trace`/`inputs_hash` 변수가 직전 LLM classify(line 387)의 같은 스코프에 있다고 추정. 빌드 결과 E0425 4건 — 실제로는 별도 `PipelineStep::Verify` 블록(line 426)이라 변수 비공유.

## 원인

### 직접 원인
1. (문제 1) Phase 95 작업 시 "잔여 영역 있음"으로 표기했으나 후속 phase에서 잊혀짐. 메타 룰 13 4단계 진행 시 2단계 완성도 100% 도달 검증 없이 진행
2. (문제 2) 메타 룰 24 후보 등록 시 "수동 grep"으로 충분하다고 판단 → C-9 트리거 도달까지 자동화 미실시
3. (문제 3) 메타 룰 17 정식 승격 시 "재빌드 의무" 명시만 했지 자동 감지 도구 작성 안 됨
4. (문제 4) 메타 룰 8 (신규 작업 grep 먼저) 자기 적용 시 grep 범위가 좁음 — `PipelineStep` 매치 케이스 구조 사전 grep 누락. **추정 빗나감 8번째 누적**

### 구조적 원인
- "트리거 대기 인프라 = 코드 변경 없이 켤 수 있는 형태" (메타 룰 5 강화)가 phase 진행 흐름에 자동 적용 안 됨 — phase 작업 직후 즉시 다음 단계 진척도 갱신 의무 부재
- 메타 룰 자동화 도구는 메타 룰 정식 승격 시 동반되어야 한다는 별도 메타 룰 부재 (Phase 96 메타 룰 25 후보가 이를 일부 해소하지만 "자동화 동반" 측면은 미명시)
- service.rs는 `PipelineStep` enum의 5+ 케이스가 큰 match 블록으로 분기 — 각 케이스가 별도 스코프임이 코드 가독성에서 명확하지 않음. **trait 분리 또는 핸들러 함수 추출 후보** (lesson 14 변형 — 분기 트리 가독성)

## 개선

### 즉시 적용 (본 Phase 97 완료)

- ✅ mcp_server.rs `handle_get_document` — `mcp.get_document` stage 부착 (성공·실패 양쪽)
- ✅ mcp_server.rs `handle_list_documents` — `mcp.list_documents` stage 부착
- ✅ service.rs `reprocess_with_feedback` — `llm.verify_reprocess` stage 부착 (별도 `verify_trace` 생성, PipelineStep::Verify 스코프 독립)
- ✅ `spec/benchmarks/scripts/audit_stage_check.sh` 신규 — 메타 룰 24 후보 자동화. ALLOWED 영역 정규식 + format!() 동적 stage prefix 추출
- ✅ `spec/benchmarks/scripts/release_rebuild_required.sh` 신규 — 메타 룰 17 자동화. git 모드 + 마커 모드(find -newer) 양쪽 지원
- ✅ workspace cargo check 통과 (27.40s, 0 경고)
- ✅ workspace lib tests 통과 — core 169 + adapters 104 + shared 110 = **383 통과 / 0 실패** (Phase 95 동일, 회귀 0)
- ✅ Tauri cargo check 통과 (58.29s)
- ✅ audit_stage_check.sh 재실행 → 10 정적 stage + 1 동적 prefix 모두 PASS (Phase 95 7건 → Phase 97 10건)

### 메타 룰 강화 (다음 phase 자기 적용)

- [ ] **메타 룰 8 강화 (사전 grep 범위)** — `PipelineStep` 같은 큰 enum의 match 케이스 구조는 각 케이스가 별도 스코프임을 사전 grep 시 명시. grep 패턴: `match .* PipelineStep:: \{ ... \}` 케이스 내부 변수는 외부 비공유
- [ ] **메타 룰 17 + 메타 룰 24 자동화 도구 G-5 5종 → 7종 확장 표시** — META.md 회귀 게이트 목록에 audit_stage_check + release_rebuild_required 추가
- [ ] **메타 룰 25 후보 (메타 룰 자기 적용 의무) 강화 — 자동화 도구 동반 의무 추가**: 메타 룰 정식 승격 시 즉시 본 룰의 자기 적용 + 검증 자동화 도구 작성 의무

### 메타 룰 후보 추가

- **메타 룰 26 후보 (Phase 97 신규)**: **"추정 빗나감 8번째 누적 — match 케이스 스코프 사전 명시 의무"** — match block 안의 변수는 case 외부에서 비공유. lesson 50 (service.rs 3분기 추정) / lesson 52 (state.paths 추정) / 본 lesson 56 (match 케이스 스코프 추정)의 공통 패턴 — 큰 match block 안의 코드는 명시적 스코프 표시 의무
- 누적 3건 도달 → 메타 룰 23 후보(승격 기준) 미정 상태로 등록만

## 다음 세션 플래그

- C-5 + C-6 + C-8 (Phase 98 3차 묶음): benchmarks 아카이빙 / dead_selector_scan v3 / webapp-design 사용자 결정
- **메타 룰 18 8번째 누적 → 메타 룰 26 후보 등록 → 메타 룰 23 후보 정형화 우선순위 상승** (3개 메타 룰 후보 누적 = 21/22/23/24/25/26 = 6 후보)
- audit_trace 누적 데이터 50건+ 도달 시 lesson 46 G-1 root cause 자동 검증 (메타 룰 13 4단계 도달 시점)
- Phase 97 신규 stage 3건의 실측 (사용자 본격 가공 시점에 audit_trace 누적 확인)

## 회귀 기준선

| 지표 | Phase 95 | Phase 97 | 차이 |
|------|---------|---------|------|
| workspace cargo check | 통과 | 통과 (27.40s) | — |
| workspace lib 테스트 | 383 | **383** | 0 |
| workspace clippy | 0 | (재실행 후보) | — |
| Tauri cargo check | 통과 | 통과 (58.29s) | — |
| audit.record 호출처 | 9 | **12** (+3 신규) | +33% |
| stage 종류 (정적) | 7 | **10** (+3) | +43% |
| audit_stage_check 통과 | — | **PASS** (신규 도구) | 메타 룰 24 자동화 |
| release 재빌드 자동 감지 | — | **PASS** (신규 도구) | 메타 룰 17 자동화 |
| 추정 빗나감 누적 | 7 (Phase 95 KgQueryResult) | **8** (verify reprocess 스코프) | +1 |
| pipeline.exe (CLI release) | 17.88 MB | **17.92 MB** | +40 KB |
| file-pipeline-tauri.exe (GUI release) | 21.02 MB | **21.03 MB** | +14 KB |
| workspace release 빌드 시간 | 2m 13s | **1m 58s** | -11% |
| Tauri release 빌드 시간 | 9m 20s (incremental) | **80m 57s** (clean) | (cache miss) |
| .last-release 마커 생성 | — | **✅ 2026-05-26 13:55** | release_rebuild_required.sh 마커 모드 기준점 |
