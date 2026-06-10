---
created: 2026-05-22
updated: 2026-06-05 (본 세션 종결 — Phase 200/201/202 placeholder 진입 + lesson 73~75 + 메타 룰 17 강화/27 정식 + 자동화 9종 + 후보 잔여 2건 (24/31) + sub-rule 후보 1건 (G-1f) — 현행화)
phase_context: Phase 91~107 + Phase 200/201/202 placeholder 완료
purpose: 외부 테스트 / 사용자 직접 테스트 / 데이터 누적 트리거 의존 항목 분리 정리. 자체 진행 가능 항목과 명확 구분
related: spec/lesson-learned/META.md (메타 룰 16 차원 A/B 라벨), prd/roadmap/next-session-c-items.md (C 9/9 종결 단일 진실원), spec/lesson-learned/74_session-2026-06-05-6axis-batch-and-baseline-side.md, spec/lesson-learned/75_phase200-binary-plugin-decision-and-placeholder.md
---

# 외부 테스트 / 누적 트리거 의존 항목 체크리스트

Phase 91~95 누적 후속 작업을 다음 3 카테고리로 분리:

1. **A. 사용자 직접 테스트 필요** — claude가 진행 불가
2. **B. 데이터 누적 트리거 의존** — N건 도달 후 자동 측정 가능
3. **C. 자체 진행 가능** — claude가 즉시 진행 (별도 정리)

메타 룰 16 차원 A 라벨 적용: 🟢 자동 측정 / 🟡 사용자 코퍼스 의존 / 🔴 사용자 만족도 의존.

---

## A. 사용자 직접 테스트 필요 (🔴 — claude 진행 불가)

### A-1. GUI 시각 검증 (Phase 93 GUI 가시화 4건 + Phase 106 온보딩)

| 항목 | 검증 방법 | 기대 동작 |
|------|----------|---------|
| Verification 탭 anomaly-report-card | Tauri 실행 → Verification 탭 진입 | "🩺 자동 이상 감지" 카드 표시. audit_trace 비어있으면 "이상 신호 없음" / 누적 시 신호 카드 |
| Settings 탭 MCP 도구 분류 | Tauri 실행 → Settings 탭 진입 | "🧰 MCP 도구 분류" 카드. 24+ 도구 카테고리/mutates/cost 컬럼 |
| Settings 탭 PII mask 토글 | Tauri 실행 → Settings → "🛡 출력 PII 마스킹" 체크박스 | 디폴트 ON. 토글 시 save_config 호출 + pipeline.toml.bak 백업 |
| Pipeline 인스펙터 Notion capability | Pipeline 탭 → remote_upload 노드 클릭 | 활성 백엔드 capability 표시. mode=attach 선택 시 경고 |
| **헤더 🧭 온보드 4-step** (Phase 106) | 헤더 우측 🧭 클릭 → 1/4 → 2/4 (크레덴셜+기본 설정) → 3/4 → 4/4 | 무상태 흐름 (DB 미저장). 취소 시 중단. 다시 클릭 시 1부터 |
| **온보딩 기본 크레덴셜 설정** (Phase 106) | step 2 크레덴셜 저장 후 자동 표시 | 신규/기존 다른/이미 본인 3 분기. "기본으로 설정" 시 `default_credential` 갱신 |

**테스트 환경**: Tauri release 22.09 MB (2026-05-27 19:29 빌드, Phase 106 incremental). 본 파일과 함께 배포 (build artifacts).

**메타 룰 16 차원 A**: 🟡 사용자 코퍼스 의존 (시각 검증 자동 측정 불가)

### A-2. PII mask 마스킹 확인 (Phase 91 A2)

| 항목 | 검증 방법 |
|------|---------|
| 검색 결과 PII 마스킹 | 이메일/주민번호/카드번호 포함 문서 가공 후 search MCP 호출 → `[REDACTED:email]` 등 표시 확인 |
| MCP 응답 PII 마스킹 | Claude Code에서 `mcp__file-pipeline__search` 호출 → 응답에 PII 미노출 확인 |
| 사용자 정의 패턴 마스킹 | Settings → PII 패턴 추가 → 검색 결과에 적용 확인 |

**메타 룰 16 차원 A**: 🔴 사용자 만족도 의존 (마스킹 적정성 = 사용자 판단)

### A-3. MCP 도구 카탈로그 외부 노출 (Phase 92 H3)

| 항목 | 검증 |
|------|------|
| Claude Code MCP 등록 | `file-pipeline-tauri.exe serve` → Claude Code mcpServers 등록 → 24+ 도구 사용 |
| mutates_state 표시 동작 | Settings 카드에서 6개 mutating 도구가 ⚠로 표시되는지 |
| 카테고리별 그룹화 | Search/Kg/Settings/Todo/Signal/Snapshot/Lint 7 그룹 정렬 확인 |

**메타 룰 16 차원 A**: 🟡 (외부 도구 카탈로그 시각 검증)

### A-4. Notion 원격 저장소 실제 연동 (Phase 90 + 92 H5 capability)

| 항목 | 사전 조건 | 검증 |
|------|---------|------|
| Notion API 토큰 + parent_page_id | notion.so/my-integrations → Internal integration 생성 | pipeline.toml [remote_storage]에 token/parent 설정 |
| mode=page 업로드 | 위 설정 + Notion 페이지 Connect to integration | 가공본이 자식 페이지로 생성 |
| mode=attach 명시적 차단 | mode=attach 설정 | upload 시 bail! 명시적 에러 ("S3/WebDAV 권장" 메시지) |
| capability 표시 | GUI Pipeline 인스펙터 remote_upload 클릭 | `backend=notion / mode=page / can_upload=true` 표시 |
| Notion attach 모드 경고 | mode=attach 시 | GUI에 경고 메시지 표시 |

**메타 룰 16 차원 A**: 🔴 사용자 만족도 의존 (외부 서비스 인증 + 연동 결과)

---

## B. 데이터 누적 트리거 의존 (🟢 — N건 도달 후 자동 측정 가능)

### B-1. H1 audit_anomaly 실측 — lesson 46 G-1 root cause 확정

| 트리거 | 임계값 | 측정 방법 |
|--------|--------|---------|
| audit_trace 누적 | **50건+** (recent_window=50, AnomalyThresholds 디폴트) | 가공 50회+ 발생 시 H1 주기 호출 (auto_suggest_interval_hours)이 의미 있는 분석 시작 |
| stage_failure 트리거 | **동일 stage 5건+ 실패** | `bash spec/benchmarks/scripts/replay_trace.sh <trace_id>` 로 root cause 확인 |
| lesson 46 G-1 재현 | "Claude CLI 산발 실패" 패턴 | audit_trace에 `applied_rule=error` + stage=`llm.classify` 5건+ 누적 시 root cause 자동 확정 |

**현재 누적**: 0건 (Phase 91 인프라 1단계만, 본격 가공 미진행)
**예상 도달**: 사용자 본격 가공 50파일+ 처리 후
**자동 측정**: 🟢 트리거 도달 시 GUI Verification 탭 anomaly-report-card 자동 표시

### B-2. C1 자동 추천 — lesson 33 누적

| 트리거 | 임계값 | 효과 |
|--------|--------|------|
| processing_metrics 누적 | **50건+** (mcp_server.rs 안내) | 자동 추천 가능 임계값 도달 안내 |
| search_mode_counters | 다수 검색 후 분포 형성 | mode 사용 패턴 추출 (default/exact/related/recent/fusion) |
| crag_counters | CRAG 응답 누적 | correct/ambiguous/incorrect 비율 → 정밀 검색 권장 |

**현재**: settings.db `processing_metrics` 0건
**예상 도달**: 가공 50파일+ + 검색 30회+ 후
**자동 측정**: 🟢 setup_apply_modules MCP 도구 호출 시 자동

### B-3. fastembed warm 측정 (Phase 62 / 88 메타 룰 11)

| 트리거 | 임계값 | 효과 |
|--------|--------|------|
| 가공 횟수 | **2회+ (cold + warm)** | 첫 cold 80초 + 이후 warm 측정. 3회 중앙값 (메타 룰 4) |
| 동일 코퍼스 재가공 | `.local-store.json` + `.compile-state.json` 삭제 후 | 격리 측정 |

**현재**: Phase 88에서 fastembed warm 44.9s/doc 측정 완료
**재측정 트리거**: 코퍼스 변경 또는 fastembed/모델 변경 시
**자동 측정**: 🟢 `spec/benchmarks/scripts/gen_synthetic_corpus.ps1` 결합 가능

### B-4. MinHash / 메타 블로킹 (트리거 #2/#4) 재측정

| 트리거 | 임계값 | 효과 |
|--------|--------|------|
| 코퍼스 크기 | **500+ 파일** | Phase 86 + 89에서 485 파일로 5변형 측정. 다른 코퍼스 도달 시 재측정 |
| MinHash force vs 디폴트 | 동일 코퍼스 5변형 | baseline / threshold 0.8 / MinHash / Metadata blocking / all |

**현재**: 디폴트 변경 보류 결정 유지 (Phase 86 + 89 동일 결과)
**재측정 트리거**: 1000+ 파일 코퍼스 또는 사용자 명시 요청
**자동 측정**: 🟢 `bench_real_docs.rs` 등 통합 테스트로 자동

### B-5. KG 관계 풍부도 측정

| 트리거 | 임계값 | 효과 |
|--------|--------|------|
| KG 관계 평균 | **<2 관계/문서** | mcp_server.rs 권장 — rich_relations 모듈 추천 |
| 문서 수 | **300+** | 의미 있는 관계 분포 |

**현재**: 0문서 (가공 미진행)
**자동 측정**: 🟢 `kg_stats` MCP 도구 호출

### B-6. 메타 룰 22 후보 정식 승격 (사용자 정책 경계 명시 합의)

| 트리거 | 임계값 |
|--------|--------|
| 누적 사례 | **현재 2건** (Phase 92 외부 협업 경계 + Phase 94 헥사고날 정공법). **1건 추가 시** META 정식 승격 검토 |

**자동 측정**: 🟡 사용자 명시 합의가 추가 발생할 때 (claude가 트리거 시점 추정 불가)

### B-7. 메타 룰 21 후보 정식 승격 (본질/부수 도메인 분리)

| 트리거 | 임계값 |
|--------|--------|
| 누적 사례 | **현재 2건** (TFM + Mirage 본질 도메인 불일치). **1건 추가 시** 검토 |

**자동 측정**: 🟡 외부 프로젝트 분석이 1건 더 발생할 때

### B-8. 메타 룰 후보 정식 승격 누적

| 후보 | 누적 | 의미 |
|------|------|------|
| 22 (사용자 정책 경계) | ✅ Phase 104 정식 승격 (Phase 92/94/100/103 4건) | Phase 107 dev seed in-memory 결정 추가 — 누적 +1 (총 6건) |
| 23 (메타 룰 승격 기준 명문화) | ✅ Phase 99 정식 승격 | — |
| 25 (자기 적용 의무) | ✅ Phase 99 정식 승격 | — |
| 26 (match 케이스 스코프) | ✅ Phase 99 정식 승격 | — |
| 28 (UI 텍스트 코드명 노출 금지) | ✅ Phase 104 정식 승격 (Phase 76/92/101 3건) | — |
| 24 (stage 명명 규칙 정형화) | 2건 (1건 부족) | Phase 95 + Phase 97 audit_stage_check 자동화 |
| 27 (회귀 게이트 vs 점검 도구) | ✅ **2026-06-05 정식 승격** (Phase 98 + lesson 74 S-6 + lesson 74 S-5 = 3건 누적) | 5축 분류 기준 매트릭스 (정밀도/결정성/외부 의존/exit code/CI 통합) |
| 16 (자동 측정 가능성 사전 분류) | 누적 중 | Phase 90 + 메타 룰 16 A/B 라벨 적용 |
| 17 강화 (release 재빌드 + 배포 의무) | ✅ **2026-06-05 정식 승격** (Phase 106 + Phase 107 + lesson 71 Linux cross-build 3건 누적, 메타 룰 23 §승격 3요소 충족) | 자동화: `release_rebuild_required.sh` + `release_redeploy.sh` (2026-06-05 신규) 모두 게이트 분류 |
| 19 (단일 진실원 위임) | ✅ Phase 94 정식 → 본 세션 누적 7→9건 (lesson 74 M-1 + S-5) | 자동화: `single_source_check.sh` (2026-06-05 신규, §검증 grep 요소 충족) |
| 25 (메타 룰 자기 적용) | ✅ Phase 99 정식 → 본 세션 누적 3→8건 (lesson 74 a~e 5건) | 신규 작업: 정식 승격 직후 다른 영역 grep 의무 (메타 룰 17 강화 → 5건 자기 적용 사례 형성) |
| 30 (spec 본문 phase별 즉시 갱신) | ✅ 2026-06-04 정식 → 본 세션 누적 5→10건 (lesson 73 + lesson 74 a/b/c/d) | 자동화: `single_source_check.sh` 공유 + S-1/S-4 자기 적용 패턴 형성 |
| 30 (phase 종결 시 spec 본문 즉시 갱신) | 3건 | Phase 105 자기 위반 + Phase 106 자기 적용 + **Phase 107 자기 적용** — **승격 3요소 도달 (메타 룰 23) — 다음 phase에서 META 정식 승격 검토** |
| **31 (도메인 분류 vs 작업 흐름 IA 트레이드오프)** | **1건** (Phase 107 Processing+Verification 통합) | 신규. 메뉴 IA 결정 시 두 기준 트레이드오프 명시 |
| **30 sub-rule "도구 stale" (G-1f)** | **1건** (lesson 74 G1 — gui_http_smoke 7탭 stale) | 신규. 회귀 게이트 자체 stale 자기 검출 메커니즘. 누적 가능 영역: dead_selector / action_catalog / audit_stage_check / release_rebuild_required |

**자동 측정**: 🟡 후속 phase에서 1건+ 추가 누적 시 검토 (24/27/16/17강화/31)
**즉시 검토**: 🟢 메타 룰 30 (3건 누적, 정식 승격 기준 도달)

---

## B-9. GraphRAG 흡수 트리거 (Phase 103 신규)

`prd/research/external-analysis-2026-05-27-graphrag.md` 단일 진실원 참조. 모두 lesson 30 패턴 (인프라 선구현 + 디폴트 비활성).

| ID | 항목 | 인프라 위치 | 활성화 트리거 | 차원 A 라벨 |
|----|------|-----------|------------|-----------|
| **G1** | Metadata.statements 활성화 | `core::domain::models::Metadata.statements` | 가공 50파일+ + needs_verification 누적 5건+ + LLM prompts.toml statements 필드 추가 | 🟡 사용자 코퍼스 의존 |
| **G2** | RelationType::Semantic 활성화 | `core::domain::models::RelationType::Semantic(String)` | KG 관계 평균 <2 + LLM prompts.toml semantic_relations 활성 | 🟡 도메인 다양성 의존 |
| **G3** | KG Multi-hop 빔 검색 활성화 | `shared::config::SearchConfig.kg_beam_search` + McpState | A2 활성(expand_kg_hops>0) + 사용자 만족도 신호 | 🔴 사용자 만족도 의존 |
| **G4** | TF-IDF 다양성 재순위 활성화 | `shared::config::SearchConfig.tfidf_rerank_enabled` + McpState | 사용자 검색 30회+ + MRR before/after 측정 | 🟢 자동 측정 가능 |

**현재 누적**: 0건 (가공 미진행). 사용자 본격 가공 시작 시 G4 우선 측정 가능.

---

## C. 자체 진행 가능 (🟢 — claude 즉시 진행) — ✅ **9/9 전체 종결 (Phase 96~98)**

본 카테고리는 외부 입력 없이 claude가 진행 가능. C-1~C-9 모두 종결. 단일 진실원: `prd/roadmap/next-session-c-items.md` (status: 전체 종결).

| ID | 항목 | 종결 Phase | 산출 lesson |
|----|------|-----------|------------|
| C-3 | spec 추정 키워드 재검증 (메타 룰 18 자기 적용) | Phase 96 | 55 |
| C-2 | 메타 룰 1 sub-rule별 누적 사례 표 상세화 | Phase 96 | 55 |
| C-4 | domain-map.md ↔ architecture.md 포트 매핑 단일 진실원 | Phase 96 | 55 |
| C-1 | A3 trace_id 부착 영역 추가 (handle_get/list/verify_reprocess) | Phase 97 | 56 |
| C-9 | 메타 룰 24 자동화 — `audit_stage_check.sh` 신규 | Phase 97 | 56 |
| C-7 | 메타 룰 17 자동화 — `release_rebuild_required.sh` 신규 | Phase 97 | 56 |
| C-5 | benchmarks/ JSON 124→12 (archive/phase47-64-2026-04/) | Phase 98 | 57 |
| C-6 | dead_selector_scan_v3 (CSS rule scanner, 점검 도구로 분류) | Phase 98 | 57 |
| C-8 | webapp-design.md 전면 재작성 (337→187줄) | Phase 98 | 57 |

**잔여 후속**: C-1 Notion 어댑터 자체 audit 부착 / dead_selector_scan_v3 false positive 57건 정리 — 별도 phase 트리거 대기.

---

## D. 우선순위 매트릭스 (다음 진행 추천)

### 사용자 결정 영역 (현재 우선)

1. **A-1** GUI 시각 검증 (Verification anomaly / MCP 카드 / PII 토글 / Notion capability / 🧭 온보드 4-step) — 사용자 Tauri 실행 필요
2. **A-2** PII 마스킹 검색 결과 확인
3. **A-3** Claude Code MCP 등록 + 카탈로그 검증
4. **A-4** Notion 토큰 + parent_page_id 실제 연동

### 데이터 누적 대기 (외부 신호)

5. **B-1** audit_trace 50건+ 누적 후 H1 실측 (lesson 46 G-1 root cause 확정)
6. **B-2** processing_metrics 50건+ 후 C1 자동 추천 누적
7. **B-9** GraphRAG G1~G4 활성화 (가공 50파일+ + 검색 30회+)
8. **B-6/B-7/B-8** 메타 룰 후보 정식 승격 (24/27/16/17강화/30 추가 누적 시)

### 잔여 후속 (별도 phase 트리거 대기)

9. C-1 Notion 어댑터 자체 audit 부착 (service.rs upload 외 attach/download/list/delete) → **Phase 207 fp-plugin-storage-notion에 자연 흡수 예정**
10. dead_selector_scan_v3 false positive 57건 정리 (메타 룰 13 4단계 자기 적용)
11. ~~release 재빌드 (Phase 107 누적분)~~ → **본 세션 (2026-06-04) 완료** — Linux cross-build (cargo-xwin MSVC) + D:\file-test\pipeline.exe 19.03MB 배포 + sha256 일치
12. **ensure_item hash="" race 사례** → Phase 200 진입 전 host 잔류 영역으로 처리 검토
13. **frontend 5초 폴링 → Tauri event emit 전환** → Phase 208 GUI Plugins 탭과 묶음 검토
14. ~~검색 도메인 분리 (Phase 108~115)~~ → **🔥 2026-06-04 본질 재정의 2차로 무효화. Phase 203 fp-plugin-search 자연 흡수**. 단일 진실원 변경: `prd/research/search-extraction-plan.md` → `prd/research/plugin-architecture-2026-06-04.md` (spec/deprecated.md 등재)
15. **🔥 본질 재정의 2차 (Phase 200~209) — tasty 패턴 흡수** (2026-06-04 사용자 결정 4축 합의):
    - host = 파일 가공만 / 외부 기능 모두 plugin
    - 11 MCP plugin + 24 어댑터 plugin = 35 workspace 멤버
    - **단일 진실원: `prd/research/plugin-architecture-2026-06-04.md`** (Phase 200 진입 본문)
    - Phase 200 (protocol+sdk placeholder) → 201 (Registry+permission) → 202 (IPC bus) → 203 (fp-plugin-search) → 204~207 (plugin 38종) → 208 (GUI Plugins 탭) → 209 (회귀+배포)

### 본 세션 (2026-06-04) 신규 누적 항목

| ID | 영역 | 상태 |
|----|------|------|
| Phase A 4지표 측정 인프라 | core/domain/chunking_quality.rs + Metadata.chunk_quality | ✅ 1단계 (인프라) — 2~4단계 (로직/측정/UI) 후속 |
| Phase B ChunkingStrategy enum | chunk_by_strategy 단일 진입점 | ✅ 2단계 (로직 위임) — Phase C Adaptive 본체 후속 |
| Phase E1 get_index MCP | shared/mcp_server.rs | ✅ 1+2단계 |
| Phase E2 write_note MCP (dry-run) | 동 | ⏸ 실제 저장은 stage2 (setup_rules 통합) |
| Phase E3 get_context MCP | 동 | ✅ 1+2단계 |
| Linux cross-build 사이드 5건 | icon.png / llvm-rc / cargo-xwin / unused_mut / 빌드 시간 | lesson 71 |
| 메타 룰 17 강화 누적 +1 | Linux cross-build → D:\file-test 배포 = 3건 도달 | ✅ **2026-06-05 정식 승격** (lesson 74 M-1) |
| 메타 룰 30 후보 자기 적용 +1 | CLAUDE.md + architecture + domain-map + deprecated 4건 동시 갱신 = 4건 누적 | ✅ **2026-06-04 정식 승격** (lesson 72) — 본 세션 누적 +5 (10건 도달) |
| 외부 분석 신규 3건 | adaptive-chunking + grimoire + plugin-architecture(tasty) | prd/research/ |

### 본 세션 (2026-06-05) 신규 누적 항목 — lesson 73 + 74 + 75

| ID | 영역 | 상태 |
|----|------|------|
| lesson 73 | mydocsearch_decision.md 즉시 삭제 (메타 룰 22+19 결합 첫 사례) | ✅ 종결 |
| lesson 74 (M-1) | 메타 룰 17 강화 정식 승격 — META.md 3 섹션 → 1 정식 + 2 위임 | ✅ 종결 |
| lesson 74 (S-4) | spec "Phase N 시" 표기 grep + 분류 | ✅ 종결 — 즉시 처리 1건 + 정당한 트리거 대기 3건 + 자기 해소 2건 |
| lesson 74 (S-1) | webapp-design.md 헤더 + status_note 갱신 | ✅ 종결 |
| lesson 74 (S-6) | `release_redeploy.sh` 신규 (메타 룰 17 강화 §자동화) | ✅ 종결 |
| lesson 74 (P-2) | 회귀 게이트 9종 baseline 측정 + json 보존 | ✅ 종결 — Phase 209 비교 기준 |
| lesson 74 (S-3) | archive ↔ deprecated 추가 발굴 (lesson 49 옵션 A 완전성 재검증) | ✅ 종결 — 추가 누락 0건 |
| lesson 74 (S-5) | `single_source_check.sh` 신규 (메타 룰 19/30 §자동화) | ✅ 종결 |
| lesson 74 G2 | architecture.md 수치 stale 동기화 (action_catalog 72 / dead_selector 94) | ✅ 종결 — 본 현행화 묶음에 흡수 |
| 사이드 G1 | gui_http_smoke 7탭 → 6탭 (Phase 107 미반영) | ✅ P-2 baseline 측정 중 즉시 해소 — 회귀 게이트 자체 stale 자기 검출 메커니즘 첫 사례 |
| M-3 | 메타 룰 27 정식 승격 (3건 누적: Phase 98 + release_redeploy 게이트 + single_source_check 점검) | ✅ 종결 — 5축 분류 매트릭스 + 자기 적용 4건 |
| G-1f | 메타 룰 30 sub-rule "도구 stale" 후보 등재 (1건 누적, lesson 74 G1) | ✅ 후보 등재 — 누적 가능 영역 명시 |
| P-4 / P-5 | release 재빌드 + D:\file-test 배포 | 🔄 원격 서버 분류 전환 (feedback_remote_build_only) |
| lesson 75 4축 합의 | binary plugin 4축 (빌드 위치 / 형제 모듈=plugin / PIPELINE_BASE/plugins/ / Phase 200 즉시) | ✅ 사용자 합의 — 메타 룰 22 단일 세션 +3건 첫 사례 |
| Phase 200 placeholder | fp-plugin-protocol + fp-plugin-sdk + ResolvedPaths.plugins (lesson 16 단계 0) | ✅ 종결 — 7 단위 테스트 (원격 검증 위임) |
| Phase 201 placeholder | PluginManifest + PermissionGate + PluginHandle + PluginRegistry (lesson 16 단계 1) | ✅ 종결 — 13 단위 테스트 추가 (누적 20건) |
| Phase 202 wire 타입 placeholder | IpcMessage + IpcResponse + HostEvent (wire 타입만, 실 IPC는 다음 세션) | ✅ wire 정의 완료 — 6 단위 테스트 추가 (누적 26건) |
| Q3 spec 즉시 갱신 | architecture.md §누적 변경 요약 신규 + domain-map.md §Plugin 도메인 신규 | ✅ 종결 — 메타 룰 30 11건째 자기 적용 |
| 본 prd 현행화 | roadmap.md + external-trigger-checklist.md 본 § 갱신 | ✅ 본 작업 |

### Phase 200/201/202 placeholder 후속 트리거 (다음 세션)

| Task | 영역 | 차단 사유 |
|------|------|---------|
| #22 원격 빌드 검증 | 26 단위 테스트 + workspace cargo build --tests + release_rebuild_required.sh 갱신 | feedback_remote_build_only — 원격 진입 트리거 대기 |
| Phase 202 본진입 | `fp-plugin-sdk::connection` 실 IPC (named pipe / Unix domain socket) + `PluginRegistry::call` 실제 호출 + audit 통합 | 원격 빌드 검증 완료 후 |
| Phase 203 진입 | fp-plugin-search (LocalVectorStore + MMR + vec_io 본체 plugin 이관) | Phase 202 본진입 완료 후 |
| Phase 207 형제 모듈 plugin 변환 | 각 module-* 에 `[[bin]] name = "fp-plugin-*"` + main.rs (`fp_plugin_sdk::run::<P>()`) | Phase 203~206 완료 후 |
| Phase 208 GUI Plugins 탭 | enable/disable Tauri command + settings.db 영속 + Plugin 카탈로그 UI | Phase 207 완료 후 |

---

## E. 본 문서 갱신 트리거

- 신규 phase 추가 시 본 문서 매트릭스 갱신
- 메타 룰 후보 정식 승격 시 B-6/B-7/B-8 갱신
- 데이터 누적 도달 시 B-1~B-5/B-9 결과 기록 (별도 lesson 또는 prd/research)
- 잔여 후속 1건 완료 시 본 문서에서 제거
