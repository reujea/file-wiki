---
updated: 2026-06-05 (본 세션 종결 — Phase 200/201/202 placeholder 진입 + lesson 73~75 + 메타 룰 17 강화/27 정식 + 자동화 9종 + 4축 합의 + spec 본문 즉시 갱신. 본질 재정의 2차 §유지)
status: active
---

# file-pipeline 로드맵

## 2026-06-05 본 세션 종결 (Phase 200/201/202 placeholder 진입 + 정비, lesson 73~75)

### 사용자 트리거 시계열 (메타 룰 22 단일 세션 +4건 첫 사례 — 누적 12→15+α)

| # | 트리거 | 결과 |
|---|------|------|
| 1 | "spec 폴더 분석해" | 8 본문 + 72 lesson 분석 + Q1 mydocsearch 즉시 이관 결정 |
| 2 | "Q1 적용, mydocsearch_decision.md 이관해" | lesson 73 등재 + 4건 동시 처리 (deprecated.md 흡수 + 본문 위임 + 원본 삭제). spec 본문 8→7 |
| 3 | "다음 고도화 항목" + "Q3 적용해" | lesson 73 패턴 정형화 (메타 룰 22+19 결합 첫 사례) |
| 4 | "1~6 진행해" | 6 묶음 처리 (M-1 + S-4 + S-1 + S-6 + P-2 + S-3 + S-5) + 사이드 G1/G2 + 메타 룰 17 강화 정식 승격 |
| 5 | "프로젝트 현행화 해" | lesson 74 등재 + G2 사이드 흡수 + spec/prd 일괄 갱신 |
| 6 | "Q1, Q2 순으로 진행" | M-3 메타 룰 27 정식 승격 + G-1f sub-rule 후보 등재 + P-4/P-5 원격 분류 전환 |
| 7 | "외부 형제 모듈 추가" + "별도 빌드 + plugin 폴더 + binary 추가" | binary plugin 4축 합의 → Phase 200 진입 |
| 8 | "Q2 진행해" | Phase 201 placeholder (PluginRegistry + PermissionGate + PluginManifest) |
| 9 | "Q2 → Q3 진행해" | Phase 202 wire 타입 placeholder + spec/architecture + domain-map 즉시 갱신 |
| 10 | "프로젝트 현행화 해" (본) | prd 일괄 갱신 + 위생 게이트 + (옵션) lesson 76 |

### 핵심 산출 종합

| 영역 | 결과 |
|------|------|
| lesson | 72 → **75** (3건 등재: 73 mydocsearch 즉시 / 74 6 묶음 + 사이드 / 75 Phase 200/201/202 + 4축) |
| 메타 룰 정식 | 13 → **15건** (17 강화 + 27 정식) |
| 메타 룰 후보 잔여 | 4 → **2건** (24/31) + sub-rule 후보 1건 (G-1f 도구 stale) |
| 메타 룰 22 누적 | 10 → **15+α건** (단일 세션 +5건 첫 사례) |
| 메타 룰 25 누적 | 3 → **8건** (강화 정식 직후 + Phase 200 진입 + lesson 75 자기 적용 등) |
| 메타 룰 30 누적 | 5 → **11건** (S-1/S-4/S-5/G2 본 흡수 + Q3 spec 즉시 갱신) |
| 회귀 자동화 | 7 → **9종** (release_redeploy + single_source_check 신규) |
| 메타 룰 자동화 도구 | 2 → **4건** |
| Phase 진입 | — → **200/201/202 placeholder 완료** |
| spec 본문 파일 | 8 → **7** (mydocsearch 삭제) |
| _rust_module 워크스페이스 멤버 | 24 → **26** (fp-plugin-protocol + fp-plugin-sdk) |
| file-pipeline core 모듈 | +1 (`core::plugin::*` 4 파일) |
| 단위 테스트 누적 (예정) | 0 → **26건** (Phase 200~202 placeholder, 원격 검증 위임) |
| Phase 200 baseline json | 보존 (`spec/benchmarks/gate_baseline_phase200pre_20260605.json`, Phase 209 비교 기준) |
| 메모리 신규 | 4건 + feedback_remote_build_only 강화 |

### 진입 차단 잔여 (모두 원격 트리거 대기)

| Task | 영역 |
|------|------|
| #22 | Phase 200/201/202 원격 빌드 검증 (26 단위 테스트) |
| #16 P-4 | release 재빌드 (workspace + Tauri) |
| #17 P-5 | release_redeploy.sh --apply (D:\file-test 배포) |
| Phase 202 본진입 | 실 IPC 전송 (named pipe / Unix domain socket) + PluginRegistry::call 실제 구현 + audit 통합 |
| Phase 203 진입 | fp-plugin-search (첫 plugin 이관 — LocalVectorStore + MMR + vec_io) |

### 핵심 결정 사실 (단일 진실원 위치)

- **binary plugin 4축 합의** → `prd/research/plugin-architecture-2026-06-04.md` §2-A
- **개발/빌드 원격 전용** → `memory/feedback_remote_build_only.md`
- **메타 룰 17 강화 정식 본문** → `spec/lesson-learned/META.md` §메타 룰 17
- **메타 룰 27 정식 본문** → `spec/lesson-learned/META.md` §메타 룰 27
- **Phase 200~202 산출** → `spec/domain-map.md` §Plugin 도메인 (단일 진실원) + `spec/architecture.md` §누적 변경 요약 (결정 맥락만)

## 본질 재정의 2차 (2026-06-04) — Phase 200~209 plugin 아키텍처 진입 예정

### 사용자 합의 (메타 룰 22 10건째 누적, 4축)

| 항목 | 결정 |
|------|------|
| host 경계 | **파일 가공만 (최소 host)** — watcher + WorkQueue + Preprocess + Chunk + Metadata + DB + audit 코어 + Plugin Registry + MCP server + Tauri |
| 외부 기능 | **모두 plugin** — LLM / 임베딩 / 검증 / 분류 / 검색 / KG / lint / CrossRef / Topic / Wiki / 추천 / 알림 / 첨부 / 링크 |
| 추상화 패턴 | **tasty 직접 흡수** — workspace + 별도 프로세스 + IPC + 매니페스트 + permission gate |
| 폐기 | search-extraction-plan.md (Phase 108~115) + mydocsearch_decision.md 무효화 → deprecated.md |

**단일 진실원**: `prd/research/plugin-architecture-2026-06-04.md` (Phase 200 진입 결정 본문, 13 섹션)

### Phase 200~209 로드맵

| Phase | 단계 | 산출 |
|-------|------|------|
| 200 | placeholder workspace | `fp-plugin-protocol` + `fp-plugin-sdk` placeholder, 0건 빌드 (lesson 16 단계 0) |
| 201 | PluginRegistry | 매니페스트 파서 + permission gate + discovery |
| 202 | IPC bus | named pipe (Windows) / domain socket (Linux) + wire 프로토콜 + audit 통합 |
| 203 | fp-plugin-search | LocalVectorStore + MMR + vec_io 본체 이관 (검색 분리 자연 흡수) |
| 204 | 검색·검증 plugin 4종 | fp-plugin-{kg, lint, crossref, dedup} |
| 205 | 운영 plugin 4종 | fp-plugin-{setup, optimize, signal, todo} |
| 206 | 영역 plugin 10종 | fp-plugin-{llm-cache, pii, c1-thresholds, grimoire, topic, wiki-export, wikilink, purge, reindex, audit-analyzer} |
| 207 | 어댑터 plugin 24종 | embedding 6 / llm 7 / storage 5 / notify 2 / rerank 3 / verify 1 |
| 208 | GUI Plugins 탭 | tasty 패턴 — 매니페스트 표시 + on/off + permission 디스플레이 |
| 209 | 회귀 게이트 + 측정 | bench 3회 중앙값 + release 재빌드 + D:\file-test 재배포 (메타 룰 17/4) |

### 무효화 사항 (메타 룰 19 단방향 위임)

- **Phase 108~115 검색 분리** (2026-06-01 결정) → Phase 203 fp-plugin-search 하나로 자연 흡수
- **search-extraction-plan.md** → `spec/deprecated.md` 등재 + 본문 헤더 무효화 표시 (자료 보존)
- **mydocsearch_decision.md** → 동일 처리 (Phase 203 진입 시 spec 삭제 검토)

### 사이드 발견 (본 결정으로 자연 해소)

- **MCP 카탈로그 단일 진실원 분산 우려** → `PluginRegistry::mcp_router`가 매니페스트 수집 → 동적 카탈로그
- **메타 룰 1 sub-rule 1f (단일 진입점) 비대화** → plugin 단위 자연 해소
- **Tauri commands 67 비대** → host 직접 등록 + plugin contributes 자연 분산

## 본질 재정의 1차 (2026-06-01) — 무효화

> ⚠️ 2026-06-04 결정으로 무효화. Phase 108~115는 Phase 200~209에 흡수.

| 항목 | 결정 (1차, 무효) |
|------|------|
| 본질 도메인 (1차) | file-pipeline = 가공 + 추천 + 검증 (3 도메인) → **2차: 가공 1 도메인만** |
| 분리 대상 (1차) | 검색 + KG + 임베딩 + 리랭커 + Topic Merger → **2차: 모든 비-host 기능 plugin** |
| 분리 위치 (1차) | `_rust_module/` (정적 모듈) → **2차: 별도 프로세스 plugin + IPC** |
| 진행 단위 (1차) | Phase 108~115 → **2차: Phase 200~209** |

### 진입 선행 의무

1. release 재빌드 (Phase 107 누적분, 메타 룰 17)
2. plan §3 사용자 합의
3. 회귀 게이트 7종 baseline 측정

---

## 완료 수치 (Phase 107 시점 + 본 세션 자동화 정비 2026-06-05)

| 지표 | 값 |
|------|-----|
| .rs 파일 | 105+ (Phase 91 audit.rs / Phase 91 verifier.rs / Phase 92 audit_anomaly.rs / Phase 94 settings_audit_adapter.rs +4) |
| Phase | **107 + Phase 200/201/202 placeholder 완료** (91 JAMES → ... → 107 dev seed → **본 세션 lesson 73~75 + Phase 200/201/202 placeholder**) |
| 테스트 | **383개** (lib 기존) + **26 단위 테스트 (Phase 200~202 placeholder, 원격 검증 대기)** |
| 회귀 자동화 | **9종** (G-5 5종 + Phase 97 +2 + 2026-06-05 +2) / 메타 룰 자동화 **4건** |
| Phase 200 baseline | `spec/benchmarks/gate_baseline_phase200pre_20260605.json` 보존 |
| spec 본문 파일 | **7개** (lesson 73 mydocsearch_decision.md 삭제 후) |
| lesson | **75** (lesson 73 + 74 + 75 본 세션 등재) |
| `action_catalog` 실측 | **72** (G-6 후 68 → +4) |
| `dead_selector_scan` ID | **94 PASS** (Phase 93 92 → +2) |
| _rust_module 멤버 | **26** (기존 24 + fp-plugin-protocol + fp-plugin-sdk) |
| file-pipeline core 모듈 | **6개** (audit / domain / **plugin (신규 4 파일)** / ports / reasoning / service) |
| Phase 91 service.rs 정리 | **-414줄** (process_file_legacy 389 + classify_and_process_with_retry 20 삭제) |
| 구조 | crates(core/adapters/shared) + modals(cli/app) + ui |
| APP | Tauri 2.0 GUI — release **21.03 MB** (Phase 97, 2026-05-26 13:55, **메타 룰 17 의무 다음 세션 보류** — Phase 107 변경 누적분 release 재빌드 미진행) / CLI **17.92 MB** |
| MCP | **36 도구 실측** (2026-06-01 정밀 카운트, search-extraction-plan §1.3) — 검색 8 + 추천 13 + 카운터 4 + C1 임계값 2 + C2 PII 3 + A1 캐시 2 + Todo 2 + optimize 1 + lint 1. Phase 108~115 분리 시 외부 8 / 잔류 28 |
| Dashboard 탭 | **6개 (Phase 107 Processing+Verification 통합)** — "처리 현황" 단일 메뉴 안에 가공 흐름 + 검증 결과 + 강한 주장 lint + audit anomaly 통합 |
| Tauri commands | **66개** (Phase 107 +1: `get_file_log` — pipeline.log 파일별 라인 추출) |
| 임베딩 | Claude CLI 128축 / OpenAI 1536축 / fastembed BGE-M3 1024차원(MRR 0.975) |
| LLM | 5종+Fallback + CachedLLM wrapper (A1) |
| 포트 | **13개** (Phase 91 audit + Verifier wrapper + Phase 92 ResourceCapabilities) |
| 외부 연동 | 6종 (Notion Phase 90 + page/attach mode 분기 capability) |
| 전처리 | 호스트 도구 + Rust 네이티브 DOCX/XLSX |
| 프롬프트 | prompts.toml 외부화 + RwLock 핫 리로드 |
| config 섹션 | 19개 + **output_pii_mask (Phase 91)** |
| 시크릿 저장소 | keyring 크레이트 |
| settings.db 테이블 | **신규 7종** — decision_log / llm_cache / c1_rule_thresholds / pii_patterns_user / mcp_disabled_tools / llm_cache_gc_log + **audit_trace (Phase 91 A3, trace_id 단일 키)** |
| compile warnings | workspace 0 / **clippy `--all --tests` 0** 유지 |
| 헤더 카드 | 15 (문서4 + KG3 + 토큰3 + LLM 캐시 5) |
| 라이브 리로드 | PII 패턴 + C1 임계값 (Phase 84) |
| DB 등록률 | 100% |
| **dead_selector_scan baseline** | **92 ID** (Phase 92 88 → 92, +4 Phase 93 GUI) |
| **메타 룰 인덱스** | **30개 (정식 27 + 후보 3)** — 후보: 24 / 27 / **31 신규 (도메인 분류 vs 작업 흐름 IA 트레이드오프, Phase 107)** |
| **lesson** | **66개** (Phase 91 50 / ... / 105 64 / 106 65 / **107 66**) |
| **WorkQueue 갱신 흐름** | **batch + watch 양쪽** (Phase 107, 이전 batch만) — ensure_item helper + 즉시 save |
| **ResolvedPaths 필드** | **10개** (Phase 107 qdrant 제거, 11→10) |
| **추정 빗나감 누적** | **10건** (Phase 107: semaphore.acquire 위치 추정 빗나감) |
| **헤더 우측 버튼** | **2개** (Phase 106 +1: 🧭 온보드 + 🤖 AI 설정 도우미) |
| **Tauri release** | **22.09 MB** (Phase 106 incremental) |
| **외부 분석 단일 진실원** | **5건** (Ruflo + JAMES x2 + Mirage + GraphRAG) |
| **검색 인프라** | TF-IDF 재순위 + KG 빔 검색 추가 (디폴트 비활성, lesson 30 패턴) |
| **Metadata 필드** | **15개** (+ statements G1) |
| **RelationType variant** | **6종** (+ Semantic G2) |
| **Settings 그룹** | **5개** (Phase 100 +1: 운영 — Phase 82~93 누적 5 카드 통합) |
| **회귀 게이트** | **7종 (exit 0 의무) + 1 점검 도구** (Phase 98 dead_selector_scan_v3 점검용 추가) |
| **A3 호출처** | **12** (Phase 95 9 → Phase 97 12, Phase 98 변동 없음) |
| **audit stage 종류** | **10 정적 + 1 동적 prefix** (메타 룰 24 자동화 PASS) |
| **benchmarks/ JSON** | **루트 12 + archive/phase47-64-2026-04/ 112** (Phase 98 분리) |
| **webapp-design.md** | **187줄** (Phase 98 -150줄, Phase 56 컨텍스트 폐기) |
| **C 항목 진행** | **9/9 완료** (Phase 96 3/9 → 97 6/9 → 98 9/9) |

## Phase 91~95 핵심 작업 (2026-05-21~22)

### Phase 91 — JAMES v0.3.0 패턴 흡수 5건 (RBAC/외부 협업 보류)

| 작업 | 결과 |
|------|------|
| A1' 검사 분산 통일 | classifier.rs `check_sensitive_and_pii` + `SensitivityDecision`. process_file_legacy 삭제 (-414줄) |
| A2 출력 PII mask | `mask_pii_in_text` + 검색 응답 마스킹. SearchConfig.output_pii_mask |
| A3 trace_id 단일 키 | `core/audit.rs::TraceId` + settings.db `audit_trace` 테이블 + replay_trace.sh (G-5 6번째) |
| B1 Verifier 통합 | `core/reasoning/verifier.rs` wrapper (verify_with_thresholds + detect_strong_claims) |
| B2 MCP mutates_state | `mcp_tool_catalog()` 26 도구 read-only/mutating 분류 |

### Phase 92 — Mirage v0.0.1 흡수 + JAMES 재검증

| 작업 | 결과 |
|------|------|
| H3 MCP 다차원 카탈로그 | `McpToolMetadata { mutates, category, cost }` 7 카테고리 + 3 비용 |
| H5 ResourceCapabilities | `RemoteStoragePort::capabilities()` + 5 어댑터 구현. Notion mode 분기 |
| H1 audit_anomaly | `shared/audit_anomaly.rs` 신규 — 자동 이상 감지 + 사용자 검토 권고 (자동 롤백 아님) |
| 메타 룰 20 META 승격 | "외부 프로젝트 패턴 흡수 시 도메인 가정 정렬" 누적 4건 → 정식 |

### Phase 93 — GUI 가시화 4건 묶음

| 작업 | 결과 |
|------|------|
| H1 anomaly 카드 | Verification 탭 동적 생성 — 자동 롤백 아닌 검토 권고 명시 |
| H3 MCP 카탈로그 | Settings 탭 — 26 도구 다차원 표 |
| H5 Notion capability | Pipeline `remote_upload` 인스펙터 확장 — attach 모드 경고 |
| A2 PII mask 토글 | Settings 카드 — 디폴트 ON 체크박스 |
| Tauri commands +4 | 61 → 65 |

### Phase 94 — AuditPort 헥사고날 + 메타 정형화

| 작업 | 결과 |
|------|------|
| AuditPort trait | core/ports/output.rs + NullAuditAdapter 디폴트 |
| SettingsAuditAdapter | shared 측 settings.db audit_trace 기록 어댑터 |
| service.rs LLM 3 부착 | classify_text / classify / verify reprocess (성공·실패 양쪽 기록) |
| mcp_server.rs handle_search 2 부착 | 일반 + 캐시 hit |
| H1 주기 호출 | modals/app/service.rs c1-periodic에 analyze_recent_audit 통합 |
| 메타 룰 1 sub-rule 분리 | 19건 → 7 카테고리 (1a UI 제거 / 1b 구조체 필드 / 1c DB 스키마 / 1d 미연결 / 1e 직렬화 / 1f 함수 분산 / 1g spec 자기 위반) |
| 메타 룰 19 META 승격 | 단일 진실원 위임 패턴 — 누적 5건 → 정식 + 후보 섹션 자기 적용 삭제 |

### Phase 106 — GUI 온보딩 4-step (무상태 명시 버튼)

| 작업 | 결과 |
|------|------|
| 사용자 트리거 | "초기 실행 시 온보딩 기능 추가" → 사용자 결정으로 "별도 DB 없이 상단에 온보드 버튼 + step별 입력" 무상태 흐름 합의 (메타 룰 22 5건째) |
| 헤더 🧭 온보드 버튼 | `ui/index.html` 라인 16~18 신규. `data-action="open-onboarding"` |
| 4-step 모달 | 환영(1/4) → 크레덴셜 등록(2/4) → inbox 안내(3/4) → optimize 가이드(4/4). 기존 Modal 유틸 + showCredentialForm 재사용 |
| 기본 크레덴셜 설정 모달 | step 2 저장 성공 후 3 분기 (신규 / 기존 다른 → 덮어쓰기 / 이미 본인). 사용자 추가 요청 |
| 백엔드 변경 | **0건** (settings.db 미사용 / Tauri commands 0 신규 / Rust 영향 0) |
| UI 변경 | index.html +6줄 / dashboard.js +~135줄 (5 메서드 + dispatcher 2건 + onSave hook) |
| Tauri release | 21.05 → **22.09 MB** (+1.04 MB, incremental 빌드 + UI 자원 임베드) |
| D:\file-test 재배포 | ✅ SHA-256 일치 (`0a968551...88d54b6`). taskkill PID 56008 후 19:29 배포 |
| 사이드 발견 | 1차 빌드 후 D:\file-test 재배포 누락 (이전 14:36 binary 실행 중) → 사용자 신고 → 종료+재배포. **메타 룰 17 강화 후보** (release 재빌드 → D:\file-test 자동 배포 연결) |
| 회귀 게이트 | dead_selector_scan 94 ID PASS (신규 ID 0건, data-action 기반) / release_rebuild PASS (마커 갱신) |

### Phase 105 — 프로젝트 현행화 통합 (spec 본문 누적 stale 해소)

| 작업 | 결과 |
|------|------|
| 사용자 트리거 | "프로젝트 현행화 해" — comm-spec + comm-prd + comm-log 3 스킬 연계 |
| spec/architecture.md | Phase 100~104 통합 섹션 추가 + 수치 행 메타 룰 19 위임 표시 |
| spec/domain-map.md | Phase 103 GraphRAG 흡수 표 (G1~G4 매핑) |
| spec/webapp-design.md | Settings 5그룹 (운영 신규) + UI 라벨 정책 + Phase 102 optimize 반영 |
| prd/roadmap/external-trigger-checklist.md | B-9 GraphRAG 트리거 4건 (G1~G4) 신규 등재 |
| 누적 stale 발견 | 5건 (Phase 100~104 종결 시 spec 본문 미갱신) |
| 메타 룰 30 후보 검토 | "spec 본문 phase별 즉시 갱신 의무" — 다음 phase에서 정식 등록 검토 |
| 코드 변경 | **0건** — release 재빌드 불필요 (자동 판정 PASS) |

### Phase 104 — 메타 룰 22/28 정식 승격 + 메타 룰 19 자기 위반 회귀 해소

| 작업 | 결과 |
|------|------|
| 메타 룰 21 후보 섹션 중복 제거 | Phase 103 정식 승격 시 후보 섹션 미삭제 (메타 룰 19 회귀) → 위임 표시 |
| **메타 룰 22 정식 승격** | 누적 4건 (Phase 92/94/100/103). 표 갱신 + 체크리스트 4건 |
| **메타 룰 28 정식 승격** | 누적 3건 (Phase 76/92/101). 메모리 feedback_no_phase_in_ui spec 승격 첫 사례 |
| 메타 룰 25 자기 적용 | G1/G2/G3/G4 UI 노출 0건 grep 확인 (디폴트 비활성 + 메타 룰 5 강화 부산물) |
| 정식 메타 룰 | 24 → **26** (+22, +28) |
| 후보 메타 룰 | 5 → **2** (-21 정식 / -22 정식 / -28 정식) |
| 잔여 후보 | 24 (stage 명명, 1건 부족) / 27 (게이트 vs 점검, 2건 부족) |
| 코드 변경 | **0건** — release 재빌드 불필요 |

### Phase 103 — GraphRAG 흡수 4건 묶음 + 메타 룰 21 정식 승격

| 작업 | 결과 |
|------|------|
| 외부 분석 단일 진실원 | `prd/research/external-analysis-2026-05-27-graphrag.md` 신규 (AWS GraphRAG Apache-2.0 엔터프라이즈 RAG) |
| 메타 룰 21 정식 승격 | 누적 3건 (TFM + Mirage + GraphRAG). 메타 룰 23 §승격 3요소 모두 충족 |
| **G4 TF-IDF 다양성 재순위** | handle_search 후처리 단계. 본문 토큰 빈도 → 신규 토큰 ≥50% 결과 promote. 디폴트 비활성 |
| **G3 KG Multi-hop 빔 검색** | A2 KG hop 확장 위치. expand_kg_hops 빔 폭 재사용. 디폴트 비활성 |
| **G1 Statement 노드 인프라** | Metadata.statements Vec<String> 추가. needs_verification 결합 트리거 #G1 대기 |
| **G2 의미 관계 LLM 추출 인프라** | RelationType::Semantic(String) variant 추가. Display impl. 디폴트 미사용 |
| McpState 생성처 3곳 동기 갱신 | cli/main.rs + shared/cli.rs + make_mcp_state. 메타 룰 1 sub-rule 1f 회피 |
| 추정 빗나감 메타 룰 1 1b | E0063 1건 (Metadata 단위 테스트 초기화) — 즉시 해소 |
| 회귀 | workspace lib 383 동일 / cargo check 0 경고 / audit_stage_check PASS |
| Release | CLI 17.99MB (재빌드 미수행, UI 미변경) / GUI 21.08→**21.09 MB** (+9KB) |
| D:\file-test 재배포 | ✅ SHA-256 일치. GUI 미실행 상태 |
| 영구 보류 | Neptune/Neo4j/OpenSearch/Bedrock/LlamaIndex/AWS boto3 — 단일 바이너리 정책 |

### Phase 102 — 비전문가용 통합 메타 MCP 도구 `optimize` 신규

| 작업 | 결과 |
|------|------|
| 사용자 트리거 | 5단계 시나리오 검증 → "설정 최적화 해줘" 자연어 통합 부재. "비전문가 사용자에게 꼭 필요" |
| handle_optimize 신규 | +140줄. 1 호출로 5단계 통합 (C1 분석 + 진행률 + 검토 대기 + 시나리오 권고 + next_actions) |
| 적용 정책 | **제안만 반환** (자동 적용 0건, lesson 30 Ruflo 완전 준수) |
| MCP 도구 카탈로그 | 24 → **25** (optimize: Settings/non-mutating/Free) |
| make_tool spec | 자연어 trigger 매칭 위해 description에 "비전문가용 통합" 키워드 |
| 추정 빗나감 9번째 | SetupAdvice 구조체를 JSON으로 추정 → grep으로 구조체 필드 접근 수정 |
| 회귀 | workspace lib 383 동일 / cargo check 0 경고 / audit_stage_check PASS |
| Release | CLI 17.92→**17.99 MB** (+60KB) / GUI 21.05→**21.08 MB** (+24KB) |
| D:\file-test 재배포 | ✅ SHA-256 일치. taskkill PID 44608 |

### Phase 101 — 내부 코드명 UI 노출 제거 + Pipeline 이관 검토

| 작업 | 결과 |
|------|------|
| 사용자 질문 트리거 | "C1은 무슨뜻?" → 외부 프로젝트 Ruflo 코드명이 UI 라벨 노출 발견 |
| UI 라벨 (C1)/(C2)/(H1) 8건 제거 | index.html 4건 + dashboard.js 4건. JS 함수명/HTML id 보존 (추적성) |
| 메타 룰 28 후보 신규 등재 | "내부 코드명·Phase 번호 UI 노출 금지" — 메모리 feedback_no_phase_in_ui 메타 승격 (3건 누적) |
| Pipeline 이관 검토 (구현 0건) | 6 카드 × 노드 매핑 평가. 3 옵션 보고 (A 이관 안 함 / B PII 마스킹만 / C 확대) |
| 사용자 결정 필요 | Pipeline 이관 옵션 (메타 룰 22 후보 4번째 누적 가능) |
| Tauri release | 21.05 MB (변동 0, UI 텍스트만) — 11m 53s incremental |
| D:\file-test 재배포 | ✅ SHA-256 일치. taskkill PID 47976 후 재배포 |

### Phase 100 — Settings IA 변경 (사용자 첫 UX 피드백 즉시 처리)

| 작업 | 결과 |
|------|------|
| 사용자 피드백 | "5 운영 카드가 컨텐츠 상단에 평면 — 좌측 네비로 이관" |
| AskUserQuestion | 단일 그룹 "운영" 옵션 합의 (메타 룰 22 후보 3번째 누적) |
| 단일 진실원 컨테이너 | `#settings-ops-cards` 신설 (5 카드 묶음). 기본 display:none |
| Mount 위치 | 운영 그룹 콘텐츠 `#settings-ops-cards-mount`. `_mountOpsCards()` DOM 이동 (메타 룰 19 자기 적용 7건째) |
| settings-nav 그룹 | 4 → **5** (+ 운영) |
| dead_selector_scan ID | 92 → **94** (+2 신규 mount) |
| Tauri release | 21.03 → **21.05 MB** (+512 bytes, UI 텍스트 변동) |
| D:\file-test 재배포 | ✅ SHA-256 일치. taskkill /F /PID 25732 후 5초 대기 (Windows 핸들 락) |

### Phase 99 — 메타 룰 23/25/26 정식 승격 + 메타 룰 19 자기 위반 해소

| 작업 | 결과 |
|------|------|
| 메타 룰 23 정식 승격 (승격 기준) | 본 룰 자체 1건만으로 즉시 정식 (예외 적용). 승격 3요소 AND 조건: 누적 ≥3건 + 체크리스트 + META 등재 |
| 메타 룰 25 정식 승격 (자기 적용 의무) | 누적 3건 (lesson 49/53/55) + 체크리스트 보강 |
| 메타 룰 26 정식 승격 (match 스코프) | 누적 3건 (lesson 50/52/56) + grep 사전 확인 체크리스트 |
| 메타 룰 19 자기 위반 해소 | 후보 22/24/25/26/27 본문 META 등재 (lesson 본문 분산 5건 해소) |
| 후보 평가 표 추가 | META 사용 방법 섹션에 의사결정 흐름 + Phase 99 동시 평가 표 |
| 잔여 후보 | 4건 (21/22/24/27) — 추가 누적 대기 |
| 코드 변경 | **0건** — release 재빌드 불필요 자동 판정 PASS |

### Phase 98 — 위생 묶음 (C-5+C-6+C-8) — **C 항목 9/9 완료**

| 작업 | 결과 |
|------|------|
| C-5 benchmarks 아카이빙 | 124→12 JSON (archive/phase47-64-2026-04/ 112 분리). 신규 측정 baseline 명확화 |
| C-6 dead_selector_scan v3 | CSS rule scanner 추가. TemplateLiteral 결합 패턴으로 false positive 일부 차단 (63→57). 점검 도구로 분류 |
| C-8 webapp-design.md 재작성 | 337→187줄 (-44%). Phase 56 자문 컨텍스트 + 변동 이력 누적 제거. Phase 97 현행 IA 기준 |
| 회귀 게이트 | 7 → **8** (+ v3 점검 도구) |
| release 재빌드 | **불필요 (자동 판정 PASS)** — 메타 룰 17 자동화 첫 자기 적용 |
| 메타 룰 27 후보 | 회귀 게이트 vs 점검 도구 분리 의무 |

### Phase 97 — A3 영역 완성 + 자동화 2건 (C-1+C-9+C-7)

| 작업 | 결과 |
|------|------|
| C-1 A3 trace_id 잔여 영역 부착 | mcp.get_document / mcp.list_documents (mcp_server.rs) + llm.verify_reprocess (service.rs PipelineStep::Verify 스코프 독립) |
| C-9 audit_stage_check.sh 자동화 | 메타 룰 24 후보 자동화. ALLOWED 영역 정규식 + format!() 동적 stage prefix. 10 정적 + 1 동적 PASS |
| C-7 release_rebuild_required.sh 자동화 | 메타 룰 17 자동화. git 모드 + 마커 모드 (find -newer) 양쪽 지원 |
| audit.record 호출처 | 9 → **12** (+3) |
| stage 종류 | 7 → **10** (정적) |
| 회귀 | workspace lib 383 동일 / cargo check 0 경고 / Tauri check 통과 |
| 추정 빗나감 8번째 | service.rs verify_reprocess 스코프 추정 → match 케이스 스코프 분리 (E0425 4건) → 별도 verify_trace 생성으로 해소 |
| 메타 룰 26 후보 | "match 케이스 스코프 사전 명시 의무" — lesson 50/52/56 누적 3건 |

### Phase 96 — 메타 룰 자기 적용 묶음 (C-3+C-2+C-4)

| 작업 | 결과 |
|------|------|
| C-3 추정 키워드 재검증 | lesson 42 PII FP ✅ Phase 89 36건 0% 해소 / lesson 46 G-1 ❓ audit_trace 누적 미도달 후속 트리거 명시 |
| C-2 메타 룰 1 sub-rule 보강 | 7 카테고리(1a~1g)에 해소 패턴 + 자동화 도구 + 신규 작업 분기 추가 |
| C-4 메타 룰 19 spec 자기 적용 | domain-map.md 단일 진실원 선언 + architecture.md 위임. 누적 5→6건 |
| 메타 룰 25 후보 등록 | "메타 룰 자기 적용 의무" — 3건 누적 (lesson 49 / 53 / 55) |
| 코드 변경 | **0건** — 메타 자기 적용만, release 재빌드 의무 없음 |

### Phase 95 — trace_id 영역 확장

| 작업 | 결과 |
|------|------|
| Tauri search audit | `tauri.search` stage |
| MCP kg_neighbors / kg_paths | `mcp.kg_*` stage |
| 원격 업로드 4 sub-branch | `remote.{backend}.upload.{processed\|origin}` ok/err 모두 기록 |
| stage 명명 규칙 정형화 | `{영역}.{도구명}.{sub?}` → 메타 룰 24 후보 등록 |
| 총 trace_id 호출처 | **9건** (메타 룰 13 2단계 완성도 향상) |

## 외부 흡수 (메타 룰 20 누적 4건)

| 외부 | 흡수 영역 | 보류 영역 |
|------|---------|--------|
| JAMES v0.3.0 (lesson 50) | Verifier 통합 / audit_trace / MCP mutates | RBAC / Change Request / 5 역할 / 메모리 3계층 |
| JAMES 재검증 (lesson 51) | 자동 롤백 트리거 (H1) | ChromaDB / Ollama / JWT 스택 |
| TabPFN / TFM | 없음 (본질 도메인 불일치) — G1 이상 탐지 / G2 ETA 예측 후보 | doc_type 분류 대체 / 리랭킹 대체 / TabTune Python |
| Mirage v0.0.1 (lesson 51) | MCP 카탈로그 다차원 / Resource capabilities | VFS / bash 인터페이스 / TypeScript-Python 스택 |

## 신규 메타 룰 (Phase 91~95)

| 룰 | 상태 | 누적 |
|----|------|------|
| 19 단일 진실원 위임 | **정식 (Phase 94 승격)** | 5건 |
| 20 외부 프로젝트 도메인 정렬 | **정식 (Phase 92 승격)** | 4건 |
| 21 후보 본질/부수 도메인 분리 | 후보 | 2건 (TFM + Mirage) |
| 22 후보 사용자 정책 경계 명시 합의 | 후보 | 2건 (Phase 92 외부 협업 + Phase 94 헥사고날) |
| 23 후보 메타 룰 승격 기준 | 후보 | 1건 (본 룰 자기) |
| 24 후보 stage 명명 규칙 | 후보 | 1건 (Phase 95 자기) |

## Ruflo 영감 통합 (2026-05-15)

| 항목 | 인프라 | 활성화 | 트리거 |
|------|--------|--------|--------|
| **A1** LLM 결과 캐시 | ✅ CachedLLM wrapper + settings.db.llm_cache + Dashboard 카드 + 비우기 | ✅ 디폴트 활성 (`llm.llm_cache_enabled=true`) | ✅ **Phase 89 측정**: 1.93x 가속 (per-doc 48.1→24.9s) |
| **A2** KG 1-hop 확장 | ✅ McpState.expand_kg_hops + handle_search 후처리 | ⏳ 디폴트 비활성 (0) | 실 사용자 검색 만족도 신호 대기 |
| **B1** 다양성 swap | ✅ McpState.diversity_threshold + dominant swap | ⏳ 디폴트 비활성 (0) | 실 사용자 dominant 편향 신호 대기 |
| **B2** 백그라운드 워커 | ✅ watcher Semaphore + tokio::spawn (기존) | ✅ 디폴트 활성 (`max_workers=4`) | 신규 작업 없음 |
| **C1** 자기학습 | ✅ auto_suggester + 4h 주기 + Settings Decision Log 카드 + accept/reject | ✅ 디폴트 활성 (`schedule.auto_suggest_interval_hours=4`) | 임계값 7종 DB 가변 |
| **C2** PII 검출 | ✅ regex 5종 + 사용자 추가 + service.rs 2 진입점 통합 | ✅ 디폴트 활성 (모든 read_to_string 경로) | ✅ **Phase 89 측정**: 36 docs / 0 격리 / FP 0% |
| **A3** trace_id audit (Phase 91~95 신규) | ✅ AuditPort trait + SettingsAuditAdapter + 9 호출처 부착 | ✅ 디폴트 활성 (service.rs / mcp_server.rs) | ✅ **메타 룰 13 2단계 완성도** — H1 주기 호출 (Phase 94) |
| **H1** audit_anomaly (Phase 92~94 신규) | ✅ shared/audit_anomaly.rs + GUI Verification 카드 (Phase 93) | ✅ 주기 호출 (Phase 94 c1-periodic) | 누적 50건+ + stage_failure 5건+ 시 자동 신호 (외부 신호) |

## Ruflo 영감 통합 (2026-05-15)

| 항목 | 인프라 | 활성화 | 트리거 |
|------|--------|--------|--------|
| **A1** LLM 결과 캐시 | ✅ CachedLLM wrapper + settings.db.llm_cache + Dashboard 카드 + 비우기 | ✅ 디폴트 활성 (`llm.llm_cache_enabled=true`) | ✅ **Phase 89 측정**: 1.93x 가속 (per-doc 48.1→24.9s). 사이드: SHA 중복 체크가 일반 hit 도달 차단 (메타 룰 15) |
| **A2** KG 1-hop 확장 | ✅ McpState.expand_kg_hops + handle_search 후처리 | ⏳ 디폴트 비활성 (0) | 실 사용자 검색 만족도 신호 대기 (자동 측정 불가) |
| **B1** 다양성 swap | ✅ McpState.diversity_threshold + dominant swap | ⏳ 디폴트 비활성 (0) | 실 사용자 dominant 편향 신호 대기 |
| **B2** 백그라운드 워커 | ✅ watcher Semaphore + tokio::spawn (기존) | ✅ 디폴트 활성 (`max_workers=4`) | 신규 작업 없음 |
| **C1** 자기학습 | ✅ auto_suggester + 4h 주기 + Settings Decision Log 카드 + accept/reject | ✅ 디폴트 활성 (`schedule.auto_suggest_interval_hours=4`) | 임계값 7종 DB 가변 (c1_rule_thresholds) |
| **C2** PII 검출 | ✅ regex 5종 + 사용자 추가 (pii_patterns_user) + service.rs 2 진입점 통합 | ✅ 디폴트 활성 (모든 read_to_string 경로) | ✅ **Phase 89 측정**: 36 docs / 0 격리 / FP 0%. 도메인 확장 시 재측정 |

## 핵심 결정

- **MyDocSearch 통합 불필요** — LocalVectorStore 단일 구조 (Qdrant Phase 44에서 제거됨)

## 로드맵 — Qdrant 점진적 개선

### ✅ Phase 1: .vec 파일 영속화 — 완료
### ✅ Phase 2: Qdrant 메모리 최적화 — 완료 (Int8 양자화 + mmap)
### ✅ Phase 3: BM25 Sparse 검색 — 완료 (dense+sparse RRF, 바이그램)
### ✅ Phase 4: KG 쿼리 확장 — 완료 (kg_neighbors, kg_paths, kg_stats)
### ✅ Phase 5: REST API + Web UI — 완료 (MVVM 대시보드 + Settings UI + Basic Auth + CORS)

### ✅ Phase 6: 잔여 미완료 항목 — 완료
- 6-1. 토픽 병합 UI (REST /api/topics + Dashboard Topics 탭)
- 6-2. 문서 목록 페이지네이션 (page/per_page)
- 6-3~6-6. 검증 테스트 9건 (전처리/알림/서비스/실환경)

### ✅ Phase 7: 고도화 — 완료
- 7-1. LLM 프로바이더 5종+Fallback (claude_cli/anthropic_api/openai_api/ollama/gemini)
- 7-2. Todo 생명주기 (체크박스 파싱 + 이월 + 완료처리)
- 7-3. 벤치마크 타임아웃 (nextest overrides + terminate-after)

### ✅ Phase 8: 경쟁 분석 → 즉시 적용 — 완료
> PRD: prd/research/competitive-analysis.md
- 8-1. /api/health 헬스체크 (Qdrant+디스크+LLM 상태)
- 8-2. 파일 병렬 처리 (Semaphore, max_workers 설정)
- 8-3. LLM Fallback 체인 (FallbackLlmAdapter)
- 8-4. 검증 메트릭 Dashboard (VerificationMetricEntry + Verification 탭)
- 8-5. 처리 진행률 API (/api/progress)
- 8-6. 메모리 계층화 Hot/Warm/Cold (MemoryTierConfig + memory_tier.rs)

### ✅ Phase 9: 실사용 준비 — 완료
- 9-1. 대용량 파일 에이전트 위임 (ChunkedAgentAdapter, chunking.rs)
- 9-2. 작업 큐 매니저 (WorkQueue, BatchPlan, 상태 추적)
- 9-3. `pipeline batch` CLI + /api/queue REST
- 9-4. 단위 테스트 19건 + Actor 시나리오 6건 + WorkQueue 테스트 10건
- 9-5. 경쟁 분석 (10카테고리 × 10요소 = 100항목, competitive-analysis.md)

### ✅ Phase 10: 아키텍처 정리 — 완료
- 10-1. backfill-sparse 제거 (upsert 자동 생성으로 불필요)
- 10-2. KG CLI 추가 (pipeline kg neighbors/paths/stats)
- 10-3. REST /api/kg/paths 추가
- 10-4. 시스템 트레이 start 통합 (Windows)
- 10-5. 하드코딩 전체 수정 (1536→dim, 90→lint_stale_days, 3→zstd_level)
- 10-6. Tauri 2.0 프로젝트 설정 (tauri/ 디렉토리, prd/features/tauri-migration.md)
- 10-7. CLI 통합 (16→9 커맨드, watch/batch/serve/dashboard→start)

### ✅ Phase 11: 단일 바이너리 + 구조 정리 — 완료
- 11-1. REST 서버 제거 (axum/tower-http 제거, 포트 바인딩 없음)
- 11-2. Tauri 단일 바이너리 (서비스+UI+트레이 = 11.3MB)
- 11-3. Todo CLI/REST/Dashboard/MCP 전체 인터페이스 노출
- 11-4. Fragment 색인 (짧은 메모 LLM 스킵)
- 11-5. 로그/CSV/이미지 전처리 강화
- 11-6. 에러 로깅 + 스케일 검증 (100K)
- 11-7. 디렉토리 구조 정리 (doc/handsoff/prd/spec/src)

### ✅ Phase 12: Tauri 완성 + 실환경 검증 — 완료
- 12-1. infrastructure lib.rs 추출 (config/qdrant_manager/build_service 공유)
- 12-2. Tauri commands 20개 완성 (REST 18 route 완전 대체)
- 12-3. Tauri service.rs 실제 FileProcessingService 연동 (placeholder 제거)
- 12-4. Dashboard JS fetch() → invoke() 전환 (REST 의존성 완전 제거)
- 12-5. Tauri 네이티브 트레이 (TrayIconBuilder) — 기존 tray-icon과 분리
- 12-6. cargo tauri build 성공 (MSI + NSIS 인스톨러)
- 12-7. 2-Pass 피드백 재가공 단위 테스트 (TwoPassLlm stub)
- 12-8. Windows Service 명령어 구조 검증 테스트
- 12-9. PDF/OCR 전처리 제거 결정 반영 (claude_cli 전용)
- 12-10. H1(교차참조), 6-4(알림) 보류 결정 반영

### ✅ Phase 13: 단일 바이너리 통합 + Qdrant 동봉 — 완료
- 13-1. GUI+CLI 단일 바이너리 (인자 분기: 없음/start→GUI, 그 외→CLI)
- 13-2. cli.rs 모듈 추출 (lib.rs에서 CLI 커맨드 공유)
- 13-3. rest_server.rs 완전 삭제
- 13-4. Qdrant v1.17.1 바이너리 다운로드 + .gitignore
- 13-5. Qdrant 미발견 시 경고 메시지 + SqliteVec fallback 자동 전환
- 13-6. Qdrant 연결 실패 시에도 SqliteVec fallback (build_service 방어)
- 13-7. 바이너리 크기 최적화 (strip+LTO+codegen-units=1: 22MB→15MB)
- 13-8. docs 설치 가이드 갱신 (Qdrant 설치, 빌드 방법, 단일 바이너리)
- 13-9. Playwright MCP로 Dashboard UI 테스트 + stat-hub 버그 수정
- 13-10. Windows SDK 빌드 환경 문제 해결 (LIB 경로 설정)

### ✅ Phase 14: 디렉토리 재구조화 (모달 분리) — 완료
- 14-1. infrastructure → crates/shared + modals/cli 분리
- 14-2. tauri/ → modals/app/ 이동
- 14-3. MCP 서버를 modals/mcp/로 독립 (별도 바이너리 pipeline-mcp)
- 14-4. static/ → ui/ 분리 (정적 프론트엔드)
- 14-5. qdrant.exe → vendor/ 이동 (외부 바이너리 관리)
- 14-6. targets/ 디렉토리 생성 (빌드 산출물)
- 14-7. workspace Cargo.toml 재구성 (members: crates/* + modals/cli + modals/mcp)
- 14-8. 모든 import 경로 갱신 (file_pipeline_infra → file_pipeline_shared)
- 14-9. APP에서 CLI 분기 제거 (GUI 전용 모달로 순수화)
- 14-10. 전체 빌드 + 테스트 125개 통과, 경고 0건

### ✅ Phase 15: 첫 실행 UX + 단일 바이너리 완성 — 완료
- 15-1. CMD 창 표시 + 초기화 진행 [1/4]~[4/4] + GUI 시작 후 콘솔 숨김
- 15-2. auto_init() — pipeline.toml + inbox/processed/originals/logs 자동 생성 (exe 디렉토리)
- 15-3. 첫 실행 안내 메시지 (생성된 파일/폴더, 사용법, 명령어)
- 15-4. backend 기본값 sqlite (Qdrant 없이 에러 0건으로 즉시 동작)
- 15-5. pipeline.toml 주석 포함 템플릿 (backend/provider 옵션 설명)
- 15-6. Qdrant 미발견 시 sqlite 설정 가이드 콘솔 출력
- 15-7. Qdrant init 실패 시에도 SqliteVec fallback (build_service 방어)
- 15-8. find_config_path/resolve_doc_types_path에 exe_dir() 탐색 추가
- 15-9. CLI+MCP를 APP에 통합 — 단일 바이너리 (16MB, 3개→1개)
- 15-10. 에러 시 "아무 키나 누르면 종료" (사용자 읽을 시간)

### ✅ Phase 16: UX 안정화 — 완료
- 16-1. Tauri 2.0 invoke API 수정 (`__TAURI_INTERNALS__` → 모든 커맨드 정상 동작)
- 16-2. GUI 중복 실행 방지 (Named Mutex 싱글 인스턴스)
- 16-3. Dashboard Lint 탭 — 삭제/백링크 보강 버튼
- 16-4. VectorDBPort에 delete() 메서드 추가 + Qdrant 구현
- 16-5. verification 기본값 변경 (max_retry=3, on_fail=quarantine_with_notify)
- 16-6. schedule에 cron 표현식 + retention_days 추가
- 16-7. memory_tier 설정/UI에서 제거 (성능 영향 없는 라벨)
- 16-8. pipeline.toml 전면 재작성 (필드별 설명, backend=qdrant 기본, sqlite fallback 가이드)
- 16-9. doc/verification-guide.md 첫 실행 자동 생성

### ✅ Phase 17: Settings UI + inbox 다중 경로 — 완료
- 17-1. Settings VSCode 스타일 (좌측 네비 + 검색바 + 섹션별 표시)
- 17-2. 좌측 네비 한국어 라벨 (압축/벡터DB/검증/LLM 등)
- 17-3. 검색 실시간 필터링 (이름+설명), 결과 없음 표시
- 17-4. semantic_dup_threshold 셀렉트 박스 (5개 옵션 + 설명)
- 17-5. select:value=label 범용 패턴 (config_metadata → JS 자동 렌더링)
- 17-6. verification.thresholds 설명 명확화 ("zstd와 무관")
- 17-7. inbox 다중 경로 (extra_inboxes 설정 + watcher 동시 감시)
- 17-8. pipeline.toml 템플릿에 extra_inboxes 설명 추가

### ✅ Phase 18: Settings 재설계 + 실시간 갱신 — 완료
- 18-1. Settings 11메뉴 → 5그룹 통합 (AI엔진/문서처리/벡터DB/품질검증/시스템)
- 18-2. Credentials 독립 탭 → AI엔진 그룹 내 인라인 표시 + 모달 폼
- 18-3. LLM 섹션에서 API 키/모델 필드 숨김 (크레덴셜로 이관)
- 18-4. GUI 모드 CMD 창 깜빡임 제거 (AllocConsole 제거, 로그 파일만)
- 18-5. 대시보드 5초 자동 갱신 (stats + queue + 문서 목록)
- 18-6. handsoff/고도화.txt → prd/features/settings-redesign.md 이관

### ✅ Phase 19: SqliteVec 영속화 + 테스트 검증 — 완료
- 19-1. SqliteVec JSON 파일 영속화 (.sqlite-vec.json)
- 19-2. upsert/link/delete 시 자동 persist
- 19-3. 재시작 후 stats 유지 확인 (총 문서 수: 1)
- 19-4. 전체 기능 테스트 (CLI 5건 + 파이프라인 6건 + APP 4건)
- 19-5. 가공 E2E 확인 (메일.txt → email_*.zst + .vec, memo → fragment)

### ✅ Phase 20: 가공 내역 관리 — 완료
- 20-1. Dashboard "Processing" 탭 추가
- 20-2. get_queue 커맨드 확장 (stats + items 상세 목록)
- 20-3. 가공 내역 테이블 (파일명/상태/크기/갱신일시)
- 20-4. 상태별 색상 (완료=녹색, 실패=빨강, 처리중=파랑, 대기=회색)
- 20-5. 상태별 정렬 (처리중 > 대기 > 실패 > 완료)

### ✅ Phase 21: 파이프라인 에디터 + 크레덴셜 확장 + Settings/Processing 개선 — 완료
- 21-1. PipelineDefinition/PipelineStep 도메인 모델 (5종 스텝: Preprocess/Llm/Verify/Embedding/Storage)
- 21-2. Tauri commands 4개 추가 (list/save/delete/reorder_pipelines) + retry_failed → 총 28개
- 21-3. process_file_with_pipeline 서비스 메서드 + LLMPort classify_and_process_text
- 21-4. watcher.rs glob 패턴 매칭 + 파이프라인 디스패치 + Default 파이프라인 자동생성
- 21-5. Dashboard Pipelines 탭 (수평 카드 플로우 에디터 + credential 미리보기)
- 21-6. LlmCredential profile_path 필드 + ClaudeCliAdapter config_dir 옵션
- 21-7. build_llm_from_credential() 유틸 + LlmConfig default_credential 필드
- 21-8. Settings 크레덴셜 연동 (분류/가공/검증/기본임베딩/민감임베딩 역할별 선택)
- 21-9. Settings 전처리/민감문서/벡터DB 가이드 문구 + 스크롤 제거 + restart 배지 개선
- 21-10. Processing 탭 (Queue 카드 + row 클릭 상세 + 5초 자동갱신 + 실패 재처리 버튼)
- 21-11. 버그수정: FreeConsole→windows_subsystem, SensitiveConfig alias 하위호환, frontendDist 상대경로, JS 문법 에러
- 21-12. 5종 스텝 오버라이드 실제 구현: Preprocess(preprocess_with_config) + LLM(credential_llms HashMap) + Embedding(embed_with_model) + Storage(compress_with_level) + Verify(thresholds)
- 21-13. credential_llms를 watcher에 미리 빌드해서 주입 (adapters→shared 순환 의존 회피)
- 21-14. WorkQueue.retry_all_failed() + UI "실패 항목 재처리" 버튼
- 21-15. dashboard.html 삭제 → index.html 단일 파일 통일
- 21-16. Tauri 컴파일 경고 5건→0건 수정 (unreachable_code, forgetting_copy_types, dead_code, 미사용 import)

### ✅ Phase 22: Pipeline Builder 통합 + 의미 임베딩 + PDF 처리 — 완료
- 22-1. Pipeline Builder 통합: Pipelines 탭 + Settings 파이프라인 흐름 → 독립 Pipeline 탭 (3패널: 사이드바+캔버스+인스펙터)
- 22-2. 노드 편집 UI: 버튼 기반 추가/삭제/순서 + 그룹별 collapse (사전검사/스텝/후처리) + 체크박스 활성/비활성
- 22-3. 시뮬레이션: 텍스트 입력 → 노드별 pass/fail/skip 표시
- 22-4. Settings 탭 축소: 6그룹→2그룹 (크레덴셜 관리, 시스템). AI엔진/문서처리/벡터DB/품질검증 → Pipeline 노드 설정으로 흡수
- 22-5. 시스템 그룹: 대시보드/로깅/알림 3col 그리드 합침, 로깅 level 선택박스
- 22-6. 크레덴셜 시스템 개선: LlmCredential UUID id + 수정 버튼 + id 기반 upsert + 폼 초기화
- 22-7. Claude CLI 의미 임베딩: 128축 고정 의미 벡터 (키워드 해시 fallback), 검색 MRR ~0.3→~0.65
- 22-8. PDF 처리 수정: ChunkedAgentAdapter read_to_string 실패→내부 LLM 위임, ClaudeCliAdapter PDF 경로 포함
- 22-9. pipeline.toml: 4개 파이프라인 활성 (PDF/텍스트/마크다운/Default), 샘플 주석
- 22-10. 버그수정: dashboard.port==0 검증 제거, .compile-state.json 잔존, 배경 서비스 로그 write_log 병행
- 22-11. dashboard.js 리팩토링: ~2087줄→~1580줄 (~795줄 제거, ~450줄 추가)
- 22-12. 새 PRD: embedding-enhancement-benchmark.md, onnx-embedding.md, external-storage.md

### ✅ Phase 23: 검색 정확도 테스트 + 표준 설정 + 테스트 격리 — 완료
- 23-1. search_accuracy.rs 신규 (11 시나리오: 랭킹/도메인/자연어/필터/하이브리드/MCP/MRR/점수분포/엣지/MCP플로우/P@K)
- 23-2. MRR@5 벤치마크 = 0.525 (20 쿼리, HashEmbedder 기준선 0.40 이상)
- 23-3. Precision@3 유형별 평균 = 0.67 (meeting 1.0, study 1.0, log 0.33, report 0.33)
- 23-4. pipeline.toml 표준화: backend=sqlite, dim=128, 검증 활성, 표준 파이프라인 4개
- 23-5. DEFAULT_CONFIG_TEMPLATE 갱신: 첫 실행 시 표준 파이프라인 포함, dim=128, sqlite 기본
- 23-6. SqliteVecAdapter.with_path() 추가 — 테스트 간 DB 격리
- 23-7. 전체 테스트 파일 7개에서 격리 경로 적용 (기존 테스트 데이터 누적 버그 해결)
- 23-8. e2e_embedded.rs Metadata 필드 수정 (삭제된 tier/last_accessed/access_count 제거)

### ✅ Phase 24: 가공 품질 고도화 + 테스트 보강 + doc_types 축소 — 완료
- 24-1. 의미 단위 청킹 구현 (chunking.rs: split_semantic, SemanticChunkConfig) — 헤딩/코드펜스/오버랩
- 24-2. ChunkedAgentAdapter에 의미 단위 청킹 연결 (with_semantic_chunking 빌더)
- 24-3. pipeline.toml [chunking] 설정 추가 + DEFAULT_CONFIG_TEMPLATE 갱신
- 24-4. 가공 프롬프트 확장: 노이즈 제거 + search_hints + code_blocks + standalone_context + 약어 풀어쓰기
- 24-5. 프롬프트 중복 제거: claude_adapter → prompts.rs 위임 (단일 소스화)
- 24-6. Metadata에 search_hints 필드 추가 + 18파일 초기화 코드 갱신
- 24-7. LlmResponse에 search_hints/code_blocks/CodeBlock 구조체 추가
- 24-8. search_hints를 Qdrant sparse vector(BM25)에 반영 (keywords + search_hints 병합)
- 24-9. doc_types.toml 축소: patterns/prompt/dedup_key/sensitive 제거 → 검증 스키마화 (섹션+임계값만)
- 24-10. DocTypeDef.sensitive 죽은 필드 확인 (SensitivityDetector와 무관)
- 24-11. Tier 1 테스트 보강: service.rs(9) + models.rs(10) + lint.rs(5) + chunking(7) = +31개
- 24-12. Tier 2 어댑터 테스트: sqlite_adapter(9) = +9개
- 24-13. bench_prompt_compare.rs 신규 (실환경 프롬프트 비교 벤치마크)
- 24-14. 벤치마크 결과: 신규 프롬프트 38.6초/파일(-6%), 구조100%, ROUGE 65.7%, 1-Pass 통과 100%
- 24-15. prd/features 정리: 완료 11건 삭제, Q&A Phase C 제거, 고도화 후보 등록
- 24-16. lesson-learned #90~#97 + 핵심 교훈 #48~#54 추가
- 24-17. lint-history.md 분리 (session-log.md에서 Lint 결과 19회분 분리)

### ✅ Phase 25: 역할별 크레덴셜 + 파이프라인 통합 — 완료
- 25-1. PipelineStep(Verify/Embedding)에 credential 필드 추가
- 25-2. PipelineDefinition에 postprocess_credential 필드 추가
- 25-3. process_file_with_pipeline에서 classify_llm/verify_llm/postprocess_llm 분리
- 25-4. resolve_pipeline_llms() — 역할별 LLM 맵 생성
- 25-5. process_file → Default 파이프라인 위임 (모든 파일 파이프라인 경유)
- 25-6. build_embedding_from_credential() — credential → EmbeddingPort 빌드

### ✅ Phase 26: Tier 2+3 테스트 보강 — 완료
- 26-1. zstd_storage 테스트 3건 (레벨 오버라이드, TTL 삭제, 헤더)
- 26-2. claude_adapter 테스트 4건 (파싱 엣지케이스)
- 26-3. notification 테스트 3건 (composite fan-out, 실패 내성, null)
- 26-4. claude_verifier 테스트 5건 (점수 파싱, 클램핑)
- 26-5. qdrant_adapter 단위 테스트 4건 (sparse 벡터, 해시)
- 26-6. config.rs 테스트 5건 (TOML 라운드트립, 부분 파싱)
- 26-7. watcher.rs 테스트 7건 (5단계 스킵, glob 매칭)
- 26-8. mcp_server.rs 테스트 3건 (stub 기반)

### ✅ Phase 27: Cross-Encoder 리랭킹 — 완료
- 27-1. RerankerPort 포트 (core/ports/output.rs)
- 27-2. ClaudeReranker 어댑터 (Claude CLI 관련도 0~10 점수)
- 27-3. NullReranker (패스스루, 기본값)
- 27-4. RerankConfig 설정 (enabled/provider/top_n)
- 27-5. McpState에 reranker 필드 + 검색 플로우 삽입
- 27-6. 리랭커 테스트 4건 (점수 파싱, null 패스스루)

### ✅ Phase 28: 외부 저장소 — 완료
- 28-1. RemoteStoragePort 포트 (upload/download/list/delete/is_configured)
- 28-2. NullRemoteStorage (비활성 기본값)
- 28-3. NetworkStorageAdapter (fs::copy, SMB/NFS 마운트)
- 28-4. WebDavStorageAdapter (reqwest, Nextcloud/Synology)
- 28-5. RemoteStorageConfig 설정 (network/webdav/s3)
- 28-6. FileProcessingService.remote_storage 필드 + process_file_with_pipeline 업로드
- 28-7. build_service 와이어링 + 테스트 2건

### ✅ Phase 29: Telegram/Slack 통합 테스트 + UI 보완 — 완료
- 29-1. notification_integration.rs — Telegram 7건 + Slack 2건 (env guard)
- 29-2. Settings UI: 스케줄/경로/동시성/외부저장소/리랭킹 섹션 추가
- 29-3. config_metadata() 확장 (6개 섹션 메타데이터)
- 29-4. PRD 정리: sensitive-config-detection.md 삭제

### ✅ Phase 30: Pipeline UI 재설계 + Credentials 탭 + Feedback 탭 — 완료
- 30-1. 고정 17단계 플로우 (편집 기능 제거)
- 30-2. 3컬럼 레이아웃 (시뮬레이션/캔버스/인스펙터)
- 30-3. 노드 간 화살표 (→, ↓)
- 30-4. 태그 UI (민감 키워드/확장자)
- 30-5. doc_types 웹 관리 (CRUD + 검색 + 페이징)
- 30-6. Credentials 독립 탭
- 30-7. Feedback 탭 (Mock + 소스 모드 실제 동작)
- 30-8. 상단 헤더 그룹화 (문서/지식그래프/토큰)
- 30-9. Storage 레벨 설명, Embedding/Storage 스텝 통합

### ✅ Phase 31: 피드백 실제 구현 + E2E 테스트 — 완료
- 31-1. 우클릭 컨텍스트 메뉴 (소스 모드 전용)
- 31-2. feedback_submit: claude -p → ui/ 수정 → git commit
- 31-3. feedback_list/diff/undo/recommendations/learn 실제 구현
- 31-4. 동시성 제어 (Mutex 순차 실행)
- 31-5. .feedback-history.json 영속화
- 31-6. E2E 테스트: 문서 3종 투입, 민감 파일, 중복 검증
- 31-7. CLI batch 커맨드 추가
- 31-8. 배치 should_skip 필터 수정
- 31-9. StubSensitiveNotification → 기본 Metadata 반환

### ✅ Phase 32: 성능 개선 + SQL 스타일 KG — 완료
- 32-1. CLI stats 경량화 (2.4초→0.7초, 벡터DB만 로드)
- 32-2. 교차참조 auto 모드 (LLM 0건, 키워드/임베딩 기반)
- 32-3. CrossRefConfig 설정 (mode/similarity_threshold/supersedes_threshold)
- 32-4. 교차참조 후보 수 5→3 축소
- 32-5. VerificationThresholds #[serde(default)] 수정
- 32-6. 기본 민감 키워드 7개 추가

### ✅ Phase 33: 마이그레이션 패널 — 완료
- 33-1. Settings > 마이그레이션 패널 UI
- 33-2. rebuild_embeddings Tauri 커맨드
- 33-3. rebuild_vectordb Tauri 커맨드
- 33-4. rebuild_all Tauri 커맨드 (originals → inbox 복사)
- 33-5. 변경 시 재수행 대상 안내 문구
- 33-6. TTL 자동 제거 원본 경고

### ✅ Phase 34: 토큰 추적 + S3 저장소 + 시뮬레이션 dry-run + CMD 창 수정 — 완료
- 34-1. TokenUsage/TokenRoleUsage 도메인 모델 + 토큰 추정 기록 (text.len()/4)
- 34-2. get_token_usage Tauri 커맨드 + 헤더 UI 실제 데이터 연동
- 34-3. S3StorageAdapter (reqwest 직접, aws-sdk 미사용, Signature V4)
- 34-4. simulate_pipeline Tauri 커맨드 (민감/Fragment/LLM/검증/임베딩 실제 실행, DB 스킵)
- 34-5. Pipeline 시뮬레이션 JS → API.simulatePipeline() 호출로 전환
- 34-6. CREATE_NO_WINDOW 플래그 (claude CLI 호출 5곳 + feedback)
- 34-7. Preprocess 테스트 버튼 + Topics 탭 검색/정렬/편집기
- 34-8. Feedback 실제 데이터 연동 + 프롬프트 JSON 강조
- 34-9. spec/lesson-learned 반성문 (7항목)

### ✅ Phase 35: 교차참조/KG 고도화 + 엔티티 영속화 — 완료
- 35-1. Entity/EntityType 도메인 모델 (8종: Person, Organization, Place, Date, Amount, Concept, Technology, Project)
- 35-2. DocRelation 메타데이터 확장 (confidence, context, created_at)
- 35-3. extract_entities() 규칙 기반 자동 추출 (인명/금액/기술/프로젝트)
- 35-4. VectorDBPort에 upsert_entity/list_entities/entities_for_doc 추가
- 35-5. SqliteVecAdapter 엔티티 영속화 (JSON 스냅샷)
- 35-6. get_crossref_stats Tauri 커맨드 + Processing 탭 교차참조 현황
- 35-7. KG 그래프 force-directed 시각화 (degree 크기, 유형별 색상, 관계별 엣지색)
- 35-8. 50건 실사용 배치 테스트 (430초, 34문서+46엔티티)
- 35-9. MCP 서버 Claude Code 연동 설정 (mcp-config.json)

### ✅ Phase 36: 검색/보안 고도화 15항목 — 완료
- 36-1~5. RRF 동적k, Instruction prefix, MMR 다양성, 검색 캐싱(5분 TTL), 검색 로깅
- 36-6~10. 구조화 임베딩 입력, 검색 스니펫, 이벤트 훅 5종(webhook+명령어), 감사 로그
- 36-11~15. at-rest 암호화 stub, RBAC stub, 토큰 추적, S3 어댑터, 시뮬레이션 dry-run

### ✅ Phase 37: 파이프라인 단일화 + 전처리기 고도화 + Processing 탭 — 완료
- 37-1. [[pipelines]] 배열 → [pipelines] 단일 구조
- 37-2. PipelineDefinition에서 name/pattern/priority/enabled 제거
- 37-3. match_pipeline/matches_glob 삭제
- 37-4. list/delete/reorder 커맨드 삭제 → get/save 2개로 교체
- 37-5. 호스트 도구 자동 감지 (HostToolDetector: pandoc/python-docx/openpyxl/libreoffice)
- 37-6. 확장자별 도구 선택 UI + 연동 테스트 버튼
- 37-7. 전처리 도구 기본값 모두 "none"
- 37-8. Preprocess 스텝 파싱 에러 → 텍스트 직접 읽기 시도 → 실패 시 가공 중단
- 37-9. Processing 탭 재구성 (문서 처리 현황 + 교차 참조 현황 + 통합 테이블 + 검색 + 페이징)
- 37-10. Feedback 탭 바이너리 모드 숨김
- 37-11. 전처리 테스트 → 시뮬레이션 영역 이관
- 37-12. 성능 벤치마크 (10카테고리 200항목)

### ✅ Phase 38: 약점 보완 3건 — 완료
- 38-1. LLM 프롬프트에 entities 필드 추가 (person/org/tech/amount/project)
- 38-2. LlmResponse + Metadata에 entities 필드 추가 (29곳 수정)
- 38-3. 엔티티 추출: LLM 응답 우선 → regex 폴백
- 38-4. BGE-M3 ONNX 활성화 (ort 2.0.0-rc.12, optional feature flag)
- 38-5. 가공본 META 헤더 제거 → 순수 본문만 저장 (메타데이터는 벡터DB 전용)
- 38-6. Quick Start 가이드 (docs/quick-start.md)
- 38-7. 성능 벤치마크 보고서 (docs/benchmark-results.md, 10카테고리 200항목)

### ✅ Phase 39: 프롬프트 외부화 + DOCX/XLSX 네이티브 + 배치 병렬화 + 설정 UI 보완 — 완료
- 39-1. 프롬프트 외부화 — prompts.toml 외부 파일 로드 (OnceLock 캐시, 없으면 내장 기본값)
- 39-2. DOCX 네이티브 텍스트 추출 — zip 크레이트로 word/document.xml 파싱 (외부 도구 불필요)
- 39-3. XLSX 네이티브 텍스트 추출 — calamine 크레이트로 시트별 데이터 추출 (외부 도구 불필요)
- 39-4. 배치 임베딩 병렬화 — ClaudeEmbeddingAdapter.embed_batch()를 Semaphore(4) 병렬 실행
- 39-5. chunking 설정 Settings UI 추가 — config_metadata에 5필드 + system 그룹에 "청킹" 섹션
- 39-6. 완료된 prd/features 4건 삭제 (external-storage, test-reinforcement, chunking-and-search, pipeline-ui-redesign)
- 39-7. 테스트 수정 — FileProcessingService 누락 필드 9파일 보강 + 민감 파일 테스트 3건 수정
- 39-8. domain-map 누락 요약 갱신 (모든 누락 해결)
- 39-9. spec 동기화 (architecture, scenarios, domain-map)

### ✅ Phase 40: BGE-M3 ONNX 실사용 + 프롬프트 핫 리로드 + 테스트 PRD — 완료
- 40-1. OnnxEmbeddingAdapter 실사용 — tokenizers 크레이트 연동, from_dir/auto_detect, attention mask mean pooling
- 40-2. build_service에서 embedding.default_model=onnx/bge_m3 시 자동 로드 + Claude CLI 폴백
- 40-3. config_metadata embedding.default_model을 select 박스로 변경 (OpenAI/Claude/ONNX)
- 40-4. 프롬프트 핫 리로드 — OnceLock → RwLock, reload_prompts() 즉시 반영
- 40-5. get_prompts/save_prompts Tauri 커맨드 (UI에서 편집 + 저장 + 핫 리로드)
- 40-6. E2E 테스트 시나리오 PRD (prd/features/e2e-test-scenarios.md, 8카테고리 ~60개)
- 40-7. 전처리 단위 테스트 14건 (DOCX 4 + XLSX 2 + CSV 1 + 로그 1 + 라우팅 3 + HostTool 3)
- 40-8. 프롬프트 핫 리로드 테스트 3건 (RwLock 교체 + get/save + invalid TOML)
- 40-9. Pipeline 탭 LLM 노드에 프롬프트 편집 모달 (textarea + 저장 + 핫 리로드)
- 40-10. BGE-M3 ONNX 모델 다운로드 (2.2GB) + 토크나이저
- 40-11. Pipeline Embedding 노드에 ONNX 모델 경로 필드 + showIf 조건부 표시
- 40-12. config validate()에 model.onnx + tokenizer.json 존재 검증
- 40-13. Playwright 검증 — LLM 프롬프트 모달 + Embedding ONNX 경로 동작 확인

### ✅ Phase 41: 잔여 10항목 전체 구현 — 완료
- 41-1. ONNX Runtime DLL — vendor/에 ORT 1.24.4 공식 바이너리 배치, ort load-dynamic 크래시는 rc 버전 문제
- 41-2. BGE-M3 MRR 벤치마크 — Python onnxruntime으로 실측 MRR@5 = **0.975** (+86% vs Hash)
- 41-3. E2E 테스트 Phase B — DOCX 파이프라인 + 프롬프트 핫 리로드 + 배치 병렬 3건
- 41-4. Playwright Phase C — 이전 세션 5시나리오 검증 완료, 세션 만료로 추가 스킵
- 41-5. Windows Credential Manager — keyring 크레이트, credential_store 모듈, Tauri 커맨드 4개
- 41-6. 의존성 최신화 — calamine 0.26 / tokenizers 0.21 유지, API 변경으로 업그레이드 보류
- 41-7. ColBERT — EmbeddingPort에 embed_colbert/supports_colbert 메서드 추가 (기본: 미지원)
- 41-8. Qdrant embedded — VectorDbConfig.auto_start + vendor/ 자동 탐색 + config_metadata
- 41-9. 모바일 빌드 — Cargo.toml mobile feature + tauri.conf.json apk/app 번들 타겟
- 41-10. 그래프DB — GraphDBPort 포트 trait + config graph_db 섹션 (json/neo4j) + Settings UI

### ✅ Phase 42: 교차참조 성능 최적화 — 완료
- 42-1. SqliteVecAdapter에 키워드 역색인 (HashMap<keyword, Vec<doc_id>>) + get_keywords 실구현
- 42-2. StoredDoc에 keywords 필드 추가 + upsert 시 저장
- 42-3. 인메모리 HNSW (instant-distance 크레이트) — 500문서 이상에서 O(log N) 검색
- 42-4. VectorDBPort에 batch_begin/batch_end 추가 — persist 지연으로 배치 I/O 최적화
- 42-5. search_hybrid 개선 — 키워드 역색인 + doc_type 매칭으로 후보 축소
- 42-6. 유사도 캐시 구조 (HashMap<(doc_a, doc_b), f32>) 추가
- 42-7. 벤치마크 결과: 100문서 **13.3 docs/s** (+111% vs 최적화 전), 검색 **0.44ms** (-20%)
- 42-8. 20개 솔루션 10카테고리 50항목 분석 PRD (crossref-optimization.md)

### ✅ Phase 43: 교차참조 비동기 배치 + Neo4j + 경고 제거 — 완료
- 43-1. mmap 캐시 필드 보유 (매번 File::open 제거 → refresh_mmap + init 로드)
- 43-2. 컴파일 경고 0건 달성 (6건 수정: unused import/variable/dead_code/cfg)
- 43-3. 65분→7분(422초) 오기재 정정 (crossref-deep-analysis.md)
- 43-4. **교차참조 비동기 배치** — crossref_queue + flush_crossref (간격 30초 + 중복 skip + priority 정렬)
- 43-5. CrossRefQueueItem.priority 필드 (MLFQ: 0=최고, 1=보통, 2=낮음)
- 43-6. E2E 교차참조 배치 테스트 2건 (e2e_crossref_async_batch, e2e_crossref_duplicate_skip)
- 43-7. **Neo4j 어댑터 실구현** — json_graph.rs (JSON 기반) + neo4j_graph.rs (HTTP/Cypher API)
- 43-8. Python ONNX subprocess 어댑터 공식화 (python_onnx_embed.rs)
- 43-9. 벤치마크 실측: 1,000문서 **14.3 docs/s (70초)** — 7분(422초) → 70초 = **6배 개선**
- 43-10. 교차참조 전체 문서 스캔 전환 (top_k 제거 → threshold 기반)
- 43-11. 유형별 cap: Supersedes 2 / Updates 5 / RelatedTopic 20 / References 10 + 조기 종료
- 43-12. HashSet O(1) link 중복 체크 + keywords 스냅샷 (Mutex 제거)
- 43-13. threshold 0.5→0.7 기본값 변경
- 43-14. search_accuracy 간헐적 실패 근본 해결 (5회 연속 통과)
- 43-15. XLSX E2E 테스트 (openpyxl + CompositePreprocessor 네이티브)
- 43-16. flush_crossref watcher 연결 (30초 유휴 + 초기 배치 후)
- 43-17. lesson-learned 분리형 전환 (INDEX.md + 아카이브)
- 43-18. 외부 전문가 상담 → Phase 2 설계 확정
- 43-19. 벤치마크 재측정: 1,000문서 **9.5 docs/s (105초), 59K 관계** — 전체 스캔+cap

### ✅ Phase 44: LocalVectorStore 통합 + Qdrant 제거 — 완료
- 44-1. SqliteVecAdapter → LocalVectorStore 리네임 (16파일)
- 44-2. Qdrant 완전 제거 (qdrant-client, qdrant_adapter, qdrant_manager, bench_qdrant 삭제)
- 44-3. HNSW 캐시 영속화 (dirty flag + lazy rebuild, 500+ 문서 자동 활성)
- 44-4. doc_type 사전 필터 제거 (search_hybrid 키워드 역색인만)
- 44-5. crossref 설정 Settings UI 추가 (10필드)
- 44-6. outgoing/incoming cap config 이관 (하드코딩 → CrossRefConfig)
- 44-7. 실제 문서 벤치마크: K8s+OpenStack 1,312문서 **28.0 docs/s (47초)**
- 44-8. 외부 전문가 상담 2회: 그래프DB 과잉 판정, 현 구조(Vec+HashSet) 유지 확정
- 44-9. 배포 크기 80MB 절감 (qdrant.exe 제거)

### ✅ Phase 45: refresh_mmap 배치 최적화 + 전문가 검증 — 완료
- 45-1. **refresh_mmap 배치 스킵** — upsert() 시 batch_mode이면 refresh_mmap 호출 생략, batch_end()에서 1회만 실행
- 45-2. 성능 결과: 7유형 601문서 **238s→105s (2.3x 개선)**, per-doc avg 0.39s→0.17s
- 45-3. persist()도 batch_mode 시 스킵 (batch_end에서 persist_now 1회)
- 45-4. 외부 전문가 검증: stale mmap 리스크 없음 확인 (find_by_hash는 documents Vec 사용, flush는 batch_end 이후)
- 45-5. 전체 230 테스트 통과, search_accuracy 12/12, 관계 수 불변 (25,087→25,086)
- 45-6. flush_crossref 프로파일링 완료: snap/search/link/persist 구간별 시간 측정 구현

### ✅ Phase 46: 순차 최적화 완료 + 병렬화 검증 + 벤치 인프라 — 완료
- 46-1. **compile_state.save() 배치화** — 매 문서 JSON 저장 → batch_mode 스킵. 실측 **1.26x** (8.04s→6.45s/100문서, 601문서 추정 105s→83s)
- 46-2. **LLM 프로파일링** — 5개 어댑터(Claude CLI/Anthropic/OpenAI/Ollama/Gemini)에 `[llm-profile]` 로깅 추가
- 46-3. **watcher batch_process 배치 모드 통합** — vector_db.batch_begin/end + compile_state_batch_begin/end
- 46-4. **100문서 마이크로 벤치** (bench_micro.rs) — A/B 비교 + per-doc 프로파일링 + 병렬 벤치 (3회 중앙값)
- 46-5. **병렬화 재시도** — 3회 중앙값 측정で効果なし확인. workers=1(5.83s) ≥ workers=4(6.04s). stub LLM에서는 Mutex 경합 > 가공 시간
- 46-6. Qdrant 잔여 참조 정리 (real_env_tests.rs)
- 46-7. **순차 최적화 종결 선언**: 289s→83s (3.5x), per-doc 318ms→60ms (5.3x), p95/p50 분산 45x→1.1x

### ✅ Phase 47: 고도화 3기법 — ReferencedBy + EmbeddingSnapshot + 행렬곱 flush — 완료
- 47-1. **ReferencedBy 관계 유형 추가** — RelationType 5종 (References↔ReferencedBy 양방향). flush_crossref + cross_reference.rs 양쪽 적용
- 47-2. **EmbeddingSnapshot** — zero-copy 인프라. VectorDBPort::embedding_snapshot() 포트 + LocalVectorStore 구현 (mmap → contiguous float32)
- 47-3. **행렬곱 flush_crossref** — N×search_similar → EmbeddingSnapshot 1회 로드 + 인라인 cosine. 기존 link 로직(양방향 cap) 재사용
- 47-4. **cosine_sim_inline** — flush 전용 인라인 cosine similarity 함수
- 47-5. **flush_crossref_legacy** — 기존 경로를 검증용으로 유지 (dead_code 허용)
- 47-6. **SlowLlm 병렬 벤치** — sleep 500ms 시뮬레이션. workers=4에서 **3.5x** (12.4s→3.5s, 효율 72%)
- 47-7. 100문서 관계 수: 4,398→**5,398** (+23%, ReferencedBy 추가분)
- 47-8. producer-consumer 분리 **보류** — 현재 tokio::spawn+Semaphore로 75% 효율 충분
- 47-9. **HNSW 재빌드 batch 스킵** — search_similar()에서 batch_mode 시 brute-force 폴백. per-doc 0.922s→0.053s @1500문서
- 47-10. **2000문서 스케일 벤치**: 125.2s (16.0 docs/s), flush 19.4s (search 3.8s + link 15.0s), 관계 108,240

### ✅ Phase 48: Blue-Green DB Refresh — 완료
- 48-1. **SearchSlot + RwLock<Arc>** — 읽기 전용 슬롯, atomic swap으로 무중단 검색
- 48-2. **build_and_swap_slot** — mmap + HNSW 통합 구축 + atomic 교체
- 48-3. **배치 시 빈 슬롯** — batch_begin()에서 empty 슬롯 설정, search 즉시 반환 (brute-force 비용 제거)
- 48-4. **HNSW 빌드 지연** — batch_end()에서 swap_slot_mmap_only (HNSW는 첫 search 시 구축)
- 48-5. **임계치 기반 refresh** — RefreshConfig (doc_count_threshold=50, time_threshold=5분). 비배치 모드에서 50건마다 1회 슬롯 교체
- 48-6. **2000문서 벤치**: 124.8s (16.0 docs/s), flush 19.6s, 관계 108,240 — 이전 대비 동등
- 48-7. **50문서 per-doc**: avg=57ms, p95=68ms — 이전(매회 refresh) 대비 p95 **104ms→68ms (1.53x)**
- 48-8. mmap_cache + hnsw_cache + hnsw_dirty 3필드 → active_slot + refresh_state 2필드로 단순화
- 48-9. batch_mode 체크 4곳→1곳 (persist만)

### ✅ Phase 49: 품질 보증 체계 구축 — 완료
- 49-1. **BenchmarkSnapshot JSON 스냅샷** — diagnostics.rs에 ThroughputMetrics/PerDocMetrics/SearchMetrics/CrossrefMetrics/StorageMetrics 구조체 + save/load/load_latest
- 49-2. **CI 회귀 감지** — check_regression() (per-doc p95 ≤100ms, flush ≤30s, throughput -20%) + bench_regression_check 테스트
- 49-3. **스냅샷 자동 저장** — bench_scale(100/500/1000/5000) + bench_micro(profile/2000) 실행 시 spec/benchmarks/{label}_{timestamp}.json 자동 저장
- 49-4. **E2E Phase C 테스트 5건** — quarantine 2-Pass, 배치 정합성+진단, compile_state 영속화, 스냅샷 왕복, 교차참조 양방향
- 49-5. **파이프라인 quarantine 버그 수정** — process_file_with_pipeline Verify 스텝에 2-Pass quarantine 누락 수정 (legacy에만 있었음)
- 49-6. **기존 테스트 수정** — crossref_async_batch + test_upsert_and_search에 batch 모드 추가 (mmap refresh threshold 미달 이슈)
- 49-7. **pipeline doctor CLI** — `pipeline doctor [--json]` 서브커맨드 + incoming degree top-10 + health_check 경고 (incoming 허브 폭증)

### ✅ Phase 50: SettingsDb + TypedSlots + 고도화 일괄 — 완료
- 50-1. **SettingsDb SQLite 구현** — settings.db 단일 파일 (config/doc_types/prompts/credentials 4테이블, 20 테스트)
- 50-2. **전문가 피드백 반영** — 타입 안전 래퍼(get_section_as<T>/get_config_as<T>) + open_or_migrate 멱등 + in-memory 테스트 (+7 테스트)
- 50-3. **Driving 어댑터 TOML→SettingsDb 전환** — CLI/MCP/APP 진입점 전환. state.config_path/doc_types_path 제거 → settings_db_path
- 50-4. **prompts.rs DB 전환** — inject_prompts() 주입 방식. Tauri save_prompts → DB 저장 + RwLock 주입
- 50-5. **TypedSlots** — flush_crossref cap 하드코딩 제거 → CrossRefConfig 필드 참조 (cap_supersedes/updates/related/references)
- 50-6. **mutual top-K** — cap_incoming 필드 추가. incoming > cap인 문서에 새 link 거부
- 50-7. **threshold 0.80 실험** — 100문서 벤치: HashEmbedder에서 0.70 vs 0.80 관계 수 동일 (similarity 분포 특성). 실 임베딩에서 재실험 필요
- 50-8. **pipeline doctor CLI** — `pipeline doctor [--json]` + incoming degree top-10
- 50-9. **dead code 제거** — save_doc_types_toml 삭제, 모바일 빌드 roadmap에서 제거
- 50-10. **단위 테스트 +65건** — adapters 6종 + diagnostics + watcher + config + SettingsDb
- 50-11. **증분 flush + DB Refresh 분리** — IncrementalFlushConfig(동적 임계치 50/200/500/1K) + flushed_embeddings 3소스 검색. db_refresh: Blue-Green swap. watcher 자동 호출. 70K OOM 방지
- 50-12. **Atomic 최적화** — IncrementalFlushState: Mutex → AtomicUsize 카운터 + 개별 Mutex(flush 시에만). upsert lock 0회. **결과**: process 131→117초(-11%), flush 7.4→4.5초(-39%), 총합 138→121초, p95 100→71ms. Phase 48(125초) 대비 **3% 개선**

### ✅ Phase 51: RAG 고도화 + MCP 모달 제거 — 완료
- 51-1. **Document Summary 인덱스** — StoredDoc에 summary 필드 추가. summary 토큰을 keyword_index에 자동 추가 → search_hybrid에서 주제 수준 매칭
- 51-2. **Sentence Window** — read_header(15줄) → read_header(100줄) + sentence_window(query 매칭 위치 ±5줄). 맥락 보존
- 51-3. **시간 가중 검색** — time_decay_boost() (365일 기준 감쇠, 최대 10% 부스트). search_hybrid에 자동 적용
- 51-4. **임베딩 메타데이터 기록** — settings.db embedding_meta 테이블. record_embedding_config() / check_embedding_mismatch()
- 51-5. **SearchMode** — MCP search에 mode 파라미터: default/exact/related/recent/fusion. related는 그래프 확장, fusion은 RAG-Fusion(다중 쿼리 RRF)
- 51-6. **MCP 모달 제거** — modals/mcp/ 삭제, Cargo.toml members에서 제거. pipeline serve로 통합
- 51-7. **실사용 벤치마크 자동 수집** — watcher batch_process에 before/after 스냅샷 + JSON 로그(logs/bench_{timestamp}.json)
- 51-8. **검색 로그 인프라** — SearchLogEntry에 result_ids + mode 추가. 골든셋 자동 구축 데이터 소스
- 51-9. **credentials 테이블 UI** — 수정/기본 설정 버튼 추가
- 51-10. **질의 확장 (경량)** — 짧은 쿼리(3단어 이하) 자동 확장 (LLM 호출 없음)

### ✅ Phase 52: RAG Tier 2-3 전체 구현 — 완료
- 52-1. **골든셋 MRR 모니터링** — settings.db golden_set 테이블 + add/list/auto_populate. 검색 로그에서 자동 구축
- 52-2. **MinHash LSH 통합** — MinHashIndex를 LocalVectorStore에 통합. upsert 시 자동 등록. 3K+ 문서에서 minhash_candidates 활성화
- 52-3. **CRAG (Corrective RAG)** — 검색 신뢰도 3단계 판정 (correct/ambiguous/incorrect). ambiguous→그래프 확장, incorrect→keyword 전용 보완. 결과에 confidence 필드
- 52-4. **RAG-Fusion** — SearchMode::Fusion에서 다중 키워드 검색 + RRF 점수 결합
- 52-5. **검색 로그 강화** — SearchLogEntry에 result_ids + mode + confidence
- 52-6. **BGE-M3** — PythonOnnxEmbeddingAdapter 이미 구현 (Phase 43). 모델 설치 시 즉시 사용 가능
- 52-7. **Parent-Child (경량)** — Sentence Window(100줄 + 매칭 위치 ±5줄)가 사실상 Parent-Child 역할
- 52-8. **CLI 5개 커맨드 추가** — process(inbox 배치)/search(검색+모드+필터)/config(get/set)/golden(add/list/eval)/bench(stub 벤치마크). 총 18개
- 52-9. **Playwright GUI 테스트** — mock-invoke + test.html + 12건 전체 PASS (탭 렌더링, 콘솔 에러 0건)

### ✅ Phase 53: TODO 시스템 리팩터링 — 완료
- 53-1. **기존 todo_lifecycle 제거** — 모듈 삭제, merge_todo→summarize_text rename (일괄 sed), service.rs todo 병합/이월 분기 2곳 제거
- 53-2. **todos 테이블** — settings.db에 14컬럼+5인덱스 (fingerprint UNIQUE). **doc_ids JSON 배열** — 같은 업무가 여러 문서에서 발견 시 doc_id를 list로 관리 (중복 제거 안 함)
- 53-3. **자동 추출** — 키워드 7종(TODO/FIXME/HACK/XXX/할일/검토필요/확인바람) + 마크다운 체크박스. fingerprint SHA-256 중복 방지. category 파일 경로 자동 추출
- 53-4. **CLI todo** — list/done/skip/reopen/add (5개 서브커맨드). settings.db 직접 조회
- 53-5. **MCP todo** — handle_list_todos/handle_complete_todo settings.db 연동. McpState에 settings_db_path 추가
- 53-6. **handsoff 이관** — rag_retrieval_matrix.md + rag_vector_rules.md → prd/research/rag-enhancement-plan.md 이관 후 삭제

### ✅ Phase 54: 실사용 준비 — 완료
- 54-1. **StubDuplicateResolution Skip→Keep** — 비대화형(GUI/watcher)에서 의미 중복 시 "둘 다 유지". DB 등록률 60%→100%
- 54-2. **ClaudeCliAdapter stdin 파이프** — `cmd.arg(prompt)` → `stdin Stdio::piped()`. Windows 32KB 명령줄 제한 해소. DOCX/대용량 문서 처리 복구
- 54-3. **인코딩 자동 감지** — chardetng + encoding_rs. UTF-8 실패 시 EUC-KR/Shift-JIS 등 자동 변환. 전처리 read_plain_text/process_csv/process_log에 적용
- 54-4. **pymupdf4llm 연결 개선** — PYMUPDF_PYTHON 환경변수 지원, 에러 폴백, 빈 결과 경고, 콘솔 창 억제
- 54-5. **classify_and_process_text 임시 파일 확장자 .txt 강제** — `.with_extension("txt")` 추가
- 54-6. **MCP 실사용 측정 로깅** — search/get_document에 `[mcp-usage]` 로그 (검색 성공률, 일일 검색 횟수, 검색-사용 지연 측정 기반)
- 54-7. **CI 기준선 갱신** — stub 처리량 15→13 docs/s (Keep 전환으로 전체 파이프라인 완주)
- 54-8. **벤치마크 검증** — 실문서 20개: 성공 20/20, DB 20/20. stub 100: 16.6 docs/s (3회 중앙값). 교차참조 동일(5,650)
- 54-9. **actor_scenarios.rs 수정** — 삭제된 todo_lifecycle 모듈 참조 → `#[ignore]`
- 54-10. **전문가 리뷰** — 성능/아키텍처/로드맵 2차 리뷰 수행. prd/expert-review-prompt.md + expert-review-response-v2.md
- 54-11. **auto-init** — CLI/GUI 실행 시 pipeline.toml + inbox/processed/originals 등 자동 생성. `ResolvedPaths::create_all()` 매 실행 호출
- 54-12. **GUI 바이너리 배포** — Tauri release 빌드 (20MB). D:\file-test에 단일 바이너리 배포
- 54-13. **공통 모달 시스템** — `Modal.open/close` CSS+JS 유틸. Credentials 탭 모달 전환 완료. Pipeline/Topics/Todos 진행 중

### ✅ Phase 55: UI 고도화 + 통합 QA — 완료
- 55-1. **모달 시스템 전체 전환** — DocType/Topics/Todos/Prompts 모달 전환 완료. 인라인 에디터(topic-editor) 제거
- 55-2. **Credentials 탭 제거** — Settings > 크레덴셜 관리로 통합. Dashboard 탭 9→8개
- 55-3. **Todo CRUD 실구현** — Tauri get_todos/complete_todo를 settings.db 연동으로 교체. add_todo 커맨드 신규
- 55-4. **통합 QA 결함 수정 4건** — Pipeline→Settings config 경쟁(flush), 크레덴셜 삭제 후 stale 참조, configMeta 누락, refreshDashboard Promise.all
- 55-5. **save_config 시크릿 보존** — restore_masked_secrets() 추가. 마스킹된 "****" 복원 + credentials 항상 보존
- 55-6. **DashboardConfig 완전 제거** — 구조체+메타데이터+mask/restore+UI. 설정 섹션 20→19개
- 55-7. **Settings 서브메뉴 분리** — 시스템 1그룹 → 일반/스케줄·경로/처리설정/인프라 4그룹. 총 5그룹
- 55-8. **Settings 멀티컬럼** — 모든 그룹에 system-3col 그리드 적용 (2~3col)
- 55-9. **크레덴셜 카드 UI** — cred-card-grid 카드형 표시 (프로바이더 아이콘+이름+모델+API키+관리 버튼)
- 55-10. **관리 편의성** — 섹션 접기/펼치기(section-toggle), 검색 하이라이트, eye toggle, 초기화 confirm
- 55-11. **트레이 메뉴 확장** — 5개 메뉴 (열기/통계/감지 ON·OFF/구분선/종료) + 좌클릭 토글 + 앱 아이콘 통일
- 55-12. **cmd 창 완전 제거** — CREATE_NO_WINDOW 전체 적용 (preprocessor HostToolDetector 4곳 + marker/tesseract/vision 6곳 + hooks/diagnostics/python_onnx)
- 55-13. **Pipeline credential "기본" 매핑** — 드롭다운에 "기본 — {name} ({provider})" 표시
- 55-14. **Settings export** — export_config_toml Tauri 커맨드 + "pipeline.toml 내보내기" 버튼
- 55-15. **Todos/Topics refresh 버튼** — 수동 새로고침 버튼 추가 (자동 갱신 없음)
- 55-16. **Playwright CRUD 실기 검증** — mock invoke 주입 + Todo/Credentials/Settings 등 10항목 전체 PASS
- 55-17. **GraphDB 비활성화** — config_metadata/Settings UI에서 graph_db 제거. KG를 ego graph(1회 API)로 전환. 코드 비활성 유지
- 55-18. **Pipeline 노드 불일치 수정** — todo_merge→entity_extract 교체, crossref/vectordb how 텍스트 수정. 18노드
- 55-19. **외부저장소+Fragment → Pipeline 사이드바 이관** — Settings에서 제거, Pipeline 좌측에 동적 폼+auto-save
- 55-20. **마이그레이션 서브메뉴** — Settings 인프라 하단 → 독립 서브메뉴로 분리. Qdrant/TTL 참조 정리
- 55-21. **pipeline.toml import** — import_config_toml Tauri 커맨드 + "가져오기" 버튼 + credentials 보존
- 55-22. **lint_stale_days/purge/notification 제거** — config_metadata에서 제거. purge 백그라운드 태스크 삭제. Linter stale 검사 삭제. notification Settings UI 제거
- 55-23. **경로 테이블 UI** — string_array를 array-table(추가/삭제) 전환. 기본 inbox readonly 표시
- 55-24. **활성화 체크박스 정렬** — enabled/활성 boolean 필드를 각 섹션 맨 위로 자동 정렬
- 55-25. **Feedback 탭 제거** — 탭+콘텐츠+CSS+이벤트+컨텍스트메뉴 삭제. Dashboard 7탭. Rust 코드는 유지
- 55-26. **RetentionConfig** — config.rs에 retention 섹션 추가 (enabled/days/targets/interval_hours). schedule.retention_days는 하위호환 유지
- 55-27. **purge.rs 도메인 로직** — purge_dry_run (대상 파일 목록) + purge_execute (실제 삭제) + 단위 테스트 3건
- 55-28. **Purge Tauri commands** — purge_dry_run/purge_execute/get_retention_config 3개 커맨드

### ✅ Phase 56: Pipeline 탭 2컬럼 + 서브탭 — 완료
- 56-1. Pipeline 탭 3컬럼 → 2컬럼 (인스펙터 제거, 캔버스→하단 축소 플로우)
- 56-2. 서브탭 4개 (데이터 가공 / 외부 저장소 / 청킹 / 보존 & Purge)
- 56-3. 시뮬레이션 로그 출력 영역 (사이드바 하단)
- 56-4. 데이터 가공 서브탭 (Preprocess/LLM/Verify/Embedding/Storage 설정 통합)
- 56-5. 보존 & Purge 서브탭 (RetentionConfig + Dry Run/Execute UI)
- 56-6. 외부 저장소 서브탭 (사이드바에서 콘텐츠 영역으로 이관)
- 56-7. 청킹+교차참조 서브탭 (Settings 처리설정에서 이관)
- 56-8. Settings 그룹 축소 (6→5, 처리설정 제거)
- 56-9. JS dead code 정리 (lint 탭 분기 제거)
- 56-10. spec 수치 현행화 (테스트 252개, .rs 105개, ~31,300줄, dashboard.js ~3,229줄)

### ✅ Phase 57: UX 고도화 + Playwright 검증 — 완료
- 57-1. 탭 순서 재배치 (사용 빈도 순: Documents > Pipeline > Processing > Todos > Settings > Topics > Verification)
- 57-2. 헤더 접기/펼치기 (통계 카드 토글, collapsed 클래스 전환)
- 57-3. 온보딩 안내 (문서 0건 시 시작하기 가이드: Settings → Pipeline → inbox)
- 57-4. KG 빈 상태 메시지 개선 ("문서 목록에서 항목을 선택하면 관계 그래프가 표시됩니다")
- 57-5. 전처리 테스트를 데이터 가공 서브탭 > Preprocess 섹션으로 이관 (사이드바에서 제거)
- 57-6. Playwright UI 검증 45건 전체 PASS (탭 순서/헤더 토글/온보딩/서브탭 4개/Purge Dry Run/Settings 5그룹/KG 안내)

### ✅ Phase 58: 코드 정리 + 트리거 대기 전환 — 완료
- 58-1. JS dead code 완전 정리 — Feedback(250줄) + Lint(30줄) + state 필드 삭제. 3,245→**2,938줄** (-307줄)
- 58-2. GraphDB 코드 제거 — graph_db/ 디렉토리(3파일) + GraphDBPort trait 삭제. 포트 12→**11개**, .rs 105→**102개**
- 58-3. 컴파일 경고 0건 달성 — unused_mut(2), unused_import(3), unused_variable(1) 수정. workspace + Tauri 모두 0건
- 58-4. 완료된 prd/features 5건 삭제 (qdrant-embedded-mode, improvement-30-plan, e2e-test-scenarios, web-integration-test-plan, pipeline-tab-v2-purge)
- 58-5. webapp-design.md 7개 질문 전체 결론 기록
- 58-6. 트리거 대기 항목 5건 정리 (threshold/MinHash/ONNX/메타데이터 블로킹/ColBERT)
- 58-7. 고도화 후보 테이블 → 트리거 대기 테이블로 전환

### ✅ Phase 90: Notion 원격 저장소 추가 — 완료 (2026-05-19)

> Phase 89 외부 신호 대기 단계의 첫 사용자 요청. spec/lesson-learned/44 후 첫 phase.

- **신규 어댑터**: `crates/adapters/.../storage/notion_storage.rs` — `NotionStorageAdapter` (RemoteStoragePort 4 메서드 + reqwest 직접 호출)
- **Config 4 필드**: `notion_token / notion_parent_page_id / notion_mode (page|attach) / notion_database_id`
- **mode 분기**: `page` (가공본 → 자식 페이지 paragraph 블록 / 100블록 / 2000자 자동 분할) / `attach` (명시적 미지원 — Notion API zst 직접 업로드 불가)
- **build_service**: `provider="notion"` 분기 + token/parent 누락 시 Null 폴백
- **UI**: Pipeline 외부 저장소 서브탭에 notion 옵션 + 4필드 + mode select + 안내 메시지
- **테스트**: 6건 (key_to_title / text_to_blocks 분할 / 2000자 분할 / mode 파싱 / attach upload 에러 / is_configured)
- **헥사고날 결정**: Notion 도메인 특수성으로 module-storage 외부에 직접 구현 (S3/WebDAV는 module-storage thin wrap)

**회귀**: workspace lib **349** (Phase 89 343 + Notion +6) / clippy `--all --tests` **0** / Tauri ✅

**후속**: file_upload v2024 API로 attach 모드 진짜 구현 (트리거 대기) / 형제 프로젝트 활용 시 module-notion-api 분리

### ✅ Phase 89: 권장 우선순위 1~3단계 + 위생 + #10 Sparse 인프라 — 완료 (2026-05-18)

> 단일 phase로 4영역 묶음. N-3/N-4 (1단계) → A1/C2 측정 (2단계) → #6 HyDE 어댑터 (3단계) → C/D 위생+메타 룰 → B-2 #10 Sparse 인프라. lesson 43 본문 확장.

**C 영역 (위생, 측정 중 발견)**:
- C-1: `--base` CLI 옵션이 LocalVectorStore까지 전파 (`paths.base.join(".local-store.json")` 명시)
- C-2: host_tools_cache fallback 매번 발생 해소 (`preprocess_with_config`에서 `with_tools(clone)` 사용)
- C-3: doc_types.toml 없음 WARN → settings.db 폴백 + DEBUG 격하

**D 영역 (메타 룰 승격)**:
- 메타 룰 13: 인프라 활성화 4단계 (인프라/로직/측정/UI)
- 메타 룰 14: 다중 진입점 분기 트리 통일
- 메타 룰 15: 측정 환경 격리 + 증분 상태 일괄 삭제

**B-2 #10 Sparse 인프라 (트리거 대기)**:
- `SparseEmbedding` 도메인 모델 추가 (core/domain/models.rs)
- `EmbeddingPort::embed_sparse / supports_sparse` 디폴트 (bail!/false)
- `VectorDBPort::upsert_sparse_embedding / search_sparse / sparse_enabled` 디폴트 no-op
- 완전 통합은 별도 phase (FastEmbedSparseAdapter에 EmbeddingPort impl + LocalVectorStore sparse_index)



**N-3 — lint 다층 주기 schedule task 연결 (3진입점)**:
- `modals/app/src/service.rs::start_background_tasks_standalone` (활성) — weekly + monthly 분기 추가
- `modals/app/src/service.rs::start_background_tasks` (dead_code) — 일관성 유지
- `modals/cli/src/main.rs::pipeline start` — CLI 모드
- 매핑: `lint_interval_hours`→Linter::lint / `lint_weekly_hours`→`lint_strong_claims(_,_,5)` / `lint_monthly_hours`→`lint_topics`

**N-4 — Metadata 보조 필드 UI 노출 (7계층 동기화)**:
- `VectorDBPort::get_metadata(doc_id) -> Result<Option<Metadata>>` 신규 (디폴트 None)
- `LocalVectorStore`에서 override (StoredDoc → Metadata 매핑)
- `get_document` Tauri command 확장 — needs_verification / open_questions / summary / keywords 응답
- `get_lint_strong_claims` Tauri command 신규 — 즉시 실행, max_per_doc 5
- `ui/index.html`: doc-detail에 detail-aux div + Verification 탭 "주간 검토 — 강한 주장" 카드
- `ui/dashboard.js`: renderDocDetail 확장 + runLintStrongClaims + _escape + click 위임

**3단계 #6 HyDE — LLM 어댑터 활성 (Phase 89 후속)**:
- LLM 어댑터 5종 (claude_cli / anthropic / openai / ollama / gemini) + wrapper 3종 (chunked / fallback / cached) 모두 `generate_hypothetical` override 추가
- `prompts.rs`: NAME_HYDE / DEFAULT_HYDE / build_hyde_prompt + SECTIONS 등록
- `src/prompts.toml`: `[hyde]` 섹션 + template 추가
- 디폴트 비활성 유지 (`search.hyde_enabled = false`). 트리거 #6 도달 시 디폴트 1줄 변경으로 즉시 활성

**측정 (권장 우선순위 2단계)**:
- A1 hit률: 9건 코퍼스 1.93x 가속 (per-doc 48.1→24.9s). 사이드 발견 — SHA 중복 체크가 A1 hit 도달 차단 (lesson 43 본문). `spec/benchmarks/a1_hit_phase89_20260518.json`
- C2-fp: 36 docs FP 0%. PII regex 5종 디폴트 유지. `spec/benchmarks/c2_fp_phase89_20260518.json`

**회귀 기준선**:
- workspace lib **343 유지** (96 + 152 + 95)
- clippy `--all --tests` **0건** 유지
- workspace + Tauri `cargo check` ✅
- 통합 테스트 빌드 ✅
- Tauri commands **70 → 71** (get_lint_strong_claims +1)

**주요 lesson**:
- lesson 43 신규 — 인프라 활성화 3단계 → 4단계 (UI 노출) / 메타 룰 1 사례 확장 (4계층 → 7계층 체크리스트) / 포트 메서드 디폴트 None 패턴 / A1 hit 측정 시 SHA 중복 체크 차단 사이드 발견

### ✅ Phase 88: lint 통합 + LLM 보조 필드 활성화 — 완료 (2026-05-18)

> Phase 87 인프라 호출처 0건 해소. N-1 + W-1 (5/15) → N-2 + fastembed 측정 (5/18) 완성. spec/lesson-learned/41~42_phase88*.md.

**Phase 88 부분 (2026-05-15)**:
- W-1: `prd/research/external-analysis-2026-05-15.md` 외부 분석 단일 진실원
- N-1: `Linter::lint_strong_claims` + `LintIssueType::StrongClaim` + 단위 테스트 3건

**Phase 88 완성 (2026-05-18)**:
- N-2: prompts.toml `[classify]` JSON 스키마에 `needs_verification` + `open_questions` 추가
- 어댑터 파싱: `LlmResponse` + `build_classify_result` 매핑
- 저장 모델: `StoredDoc` 두 필드 + upsert 매핑 (신규/업데이트 양쪽)
- fastembed feature 활성 release 빌드 (11m 26s 첫, 2m 04s incremental)
- 실 코퍼스 10건 재가공 검증: **needs_verification 19건 (1.9/doc) + open_questions 22건 (2.2/doc) 채움**. per-doc 44.9초 (v1 49.1초 대비 -8.6%)
- LocalVectorStore PIPELINE_BASE 통합 (사이드 수정, lesson 29 / Phase 85 B-4 보강)

**Phase 88 잔여 → Phase 89**:
- N-3: lint 다층 주기 → service.rs schedule task 분기 연결
- N-4: 보조 필드 + lint_strong_claims UI 노출
- C2-fp 100건+ 측정 (Q1 보류분)

빌드: workspace lib **343** 유지, clippy 0건, Tauri ✅

- **W-1**: `prd/research/external-analysis-2026-05-15.md` 신규 — supertonic/wikidocs 352523/353407 분석 결정 단일 진실원
- **N-1**: `Linter::lint_strong_claims(vector_db, storage, max_per_doc)` 추가. `LintIssueType::StrongClaim` enum 변형. 단위 테스트 3건
- **N-2 잔여**: prompts.toml `classify` 갱신 (Metadata 보조 필드를 LLM이 채움)
- **N-3 잔여**: lint 다층 주기를 service.rs schedule task에 분기 연결
- **N-4 잔여**: lint_strong_claims 결과 UI 노출
- 빌드: workspace lib **343** 통과 (+3 lint_strong_claims 테스트), clippy 0건, Tauri ✅

### ✅ Phase 87: lint 고도화 (wikidocs 353407 부분 적용) — 완료 (2026-05-15)

> 외부 문서 wikidocs 353407(정리와 감사 흐름) 권고 4건 중 측정 무관 3건 적용. spec/lesson-learned/40_phase87-lint-deepening-wikidocs-353407.md.

- **A-1**: `Metadata.needs_verification` + `Metadata.open_questions` 필드 추가 (형식 점검 5필드 중 미구현 2건)
- **A-2**: `detect_strong_claims()` 함수 추가 — 단정 표현 12종 마커 검출, Vec<String> 반환 (점수화 아님, 사용자 검토 후보)
- **A-3**: `ScheduleConfig.lint_weekly_hours` (168) + `lint_monthly_hours` (720) 다층 주기 필드
- **A-4 보류**: "수집일 vs 점검일 분리" — 본 프로젝트 가공+검증 통합 사이클이 적절
- 빌드: workspace lib **340** 통과 (+4 detect_strong_claims), clippy `--all --tests` 0건, Tauri ✅

### 외부 문서 분석 결론 (Phase 87)

| 문서 | 결과 |
|------|------|
| supertone-inc/supertonic | TTS 시스템, 직접 연관 없음. ONNX 패턴은 fastembed에 이미 차용 |
| wikidocs 352523 (자기 진화 에이전트) | Ruflo C1/decision_log이 이미 부분 적용. 추가 도입 가치 낮음 |
| wikidocs 353407 (정리와 감사 흐름) | **Phase 87에서 부분 적용 완료** (3/4) |

### ✅ Phase 86: 위생 후속 + 트리거 인프라 — 완료 (2026-05-15)

> Phase 85 위생 후속 + 측정 무관 트리거 인프라(#6/#8) 선구현. spec/lesson-learned/39_phase86-hygiene-followup-and-trigger-infra.md.

- **A-3**: lesson 36 "잔존 8건" 항목에 Phase 84/85 종결 표시 (stale 재발 방지)
- **A-4**: `spec/deprecated.md` 신규 — 삭제/보류/폐기 항목 단일 인벤토리. auto_link / Phase 64/84 dead / vendor/onnxruntime 등 누적. 월 1회 점검 규칙
- **A-5**: architecture.md Phase 65~78 추가 아카이빙 (1758 → **1368줄**, 누적 1876→1368 = -27%). archive 153→527
- **A-2**: 표 마크다운 보존 청킹 (트리거 #8 인프라). `chunking.preserve_tables` configField + `is_table_line` + `SemanticChunkConfig.preserve_tables` (디폴트 false)
- **A-1**: HyDE 폴백 검색 (트리거 #6 인프라). `search.hyde_enabled` + `search.hyde_min_results` configField + `LLMPort.generate_hypothetical` 디폴트 no-op + `handle_search` 분기 (디폴트 false)
- 빌드: workspace lib **332→336** (+4), clippy `--all --tests` 0건, Tauri ✅

### ✅ Phase 85: 위생 일괄 — 완료 (2026-05-15)

> Phase 84 후속 위생 작업. 측정 무관 항목만. spec/lesson-learned/38_phase85-hygiene-batch.md.

- **B-1**: clippy `too_many_arguments` 4건 → 입력 구조체. `CrossRefUpdateContext` / `DecisionDraft` / `NewTodo`. `#[allow]` 4건 제거
- **B-1+**: clippy `--all --tests` 잔존 4건 정리 (`field_reassign_with_default` 3 + `assertions_on_constants` 1)
- **B-2**: `auto_link` 함수 + `AutoLinkContext` 삭제 (호출처 0건 7+ Phase 미사용, lesson 14 형태 보류 마커). 약 164줄 감소
- **B-3**: architecture.md 1876 → 1758줄 아카이빙 (Phase 64 이하 → `spec/architecture-archive.md` 153줄)
- **B-4**: `find_data_dir` ↔ `resolve_paths` base 결정 통일 (사이드 발견 6 해소) — CLI/Tauri가 같은 분기 트리 사용
- 빌드: workspace lib **332** 통과, clippy `--all --tests` **0건**, Tauri `cargo check` ✅

### ✅ Phase 84: 후속 작업 일괄 (P1·P2·P3) — 완료 (2026-05-15)

> 트리거 무관 후속 12건 일괄 처리. spec/lesson-learned/37_phase84-batch-p1-p2-p3.md.

- **P2-a**: 백엔드 dead 7건 삭제 (`get_health` 외) + `setup_snapshot_list/rollback` UI 연결 (Decision Log에 Rollback 버튼)
- **P2-b**: clippy lib warning 8→0 (chunking iter_mut / cross_reference allow / type alias `ProgressCallback` / settings_db allow / rmcp trait allow)
- **P1-1**: 코드 상수 4건 → Phase 71에 이미 이전 완료 확인
- **P1-2**: HookDefinition UI CRUD 모달 (event 5종 + webhook/command + enabled)
- **P1-3**: Quarantine 분기 노드 시각화 (`branch_from` 메타 + 점선 보더 + FAIL 분기 배지)
- **P1-4**: `search_with_trace` 신규 — Dense → Hybrid → Filtered 3단계 + 순위 변화
- **P1-5**: MCP 도구 enable/disable 토글 (`mcp_disabled_tools` 테이블 + Settings 카드)
- **P1-6**: `get_processing_metrics` runtime_summary 추가
- **P3-1**: C1/C2 live reload (`pii_user_patterns: RwLock<Vec>` + `reload_pii_patterns()` + `read().clone()` 패턴)
- **P3-2**: A1 LRU 즉시 GC 버튼 (헤더 [GC 실행])
- **P3-3**: last_gc stat 카드 2개 (시각 + 삭제 건수)
- **P3-4**: 4h 주기 GC 결과를 `record_llm_cache_gc`로 settings.db 기록 (가시화)
- settings.db 신규 2테이블: `mcp_disabled_tools`, `llm_cache_gc_log`
- 빌드: workspace lib **332** 통과, clippy lib **0**, 통합 테스트 빌드 통과

### ✅ Phase 82: Decision Log (setup_apply 이력 영속화) — 완료 (2026-05-14)

**우선순위**: 🟡 Medium
**선행**: Phase 76 (RecommendationEngine), Phase 77 (ConfigSnapshot), Phase 82-prep
**후행**: 없음 (UI 노출은 후속 작업으로 분리 가능)

**목적**: `setup_apply` / `setup_apply_modules` 호출 시 각 ConfigChange 후보의 결정(accepted/rejected/critical_skipped)을 settings.db에 영속화. ConfigSnapshot과 snapshot_id로 연결해 "어떤 추천이 왜 적용/거부되었는가" 추적. 사용자 라벨 "Kuku 차용 / Proposal Diff" 중 Proposal Diff는 기존 `setup_dryrun`이 이미 제공하므로 Decision Log만 채택.

**구현 결과**:
- ✅ 신규 테이블 `decision_log` (settings.db, SETTINGS_DB_SCHEMA 단일 상수에 등록)
  - 컬럼: id / decided_at / source / snapshot_id / path / decision / before_value / after_value / priority / risk / evidence / confidence / reason / context
  - 인덱스 3종: decided_at DESC / snapshot_id / path
- ✅ `apply_advice_full_with_log` API 신규 — source/context 받아 항목별 결정 기록. 기존 `apply_advice_full`은 source="setup_review", context=None으로 위임
- ✅ `setup_apply_modules` 자동 source="setup_modules" + context={module_ids}
- ✅ 결정 분류: accepted / rejected / critical_skipped (Critical 후보가 apply_critical=false라 스킵된 경우 별도 마킹)
- ✅ MCP 도구 `setup_decision_log_list { limit?, snapshot_id? }`
- ✅ Tauri 명령 `setup_decision_log_list(limit, snapshot_id)`
- ✅ 단위 테스트 5건: insert/list/limit/filter_by_snapshot + apply 통합 2건 (accepted+rejected 혼합 + critical_skipped + context 전달)

**의도된 비범위** (후속 가능):
- Proposal/Decide 2단계 분리 (현재 1단계 apply 유지)
- 거부 reason 자유입력 UI
- 자동 결정 정책 / 다중 사용자 결정 권한자
- Decision Log UI 노출 (Settings 또는 Pipeline 모듈 모달)

**검증**:
- workspace cargo check ✅ / Tauri cargo check ✅
- lib 테스트 310건 통과 (96 + 137 + 77, +5 신규)

### ✅ Phase 82-prep: service.summary MCP 노출 + settings.db schema 단일 상수화 — 완료 (2026-05-14)

**우선순위**: 🔴 High (Phase 77 자동 롤백이 placeholder 0에 의존해 실질 동작 불능 상태였음)
**선행**: Phase 77 (ConfigSnapshot + RollbackThresholds)
**후행**: Phase 82 (Decision Log)

**목적**: Phase 77 자동 롤백·setup_snapshot_measure가 의존하던 `verify_pass_rate` / `quarantine_rate` / `avg_process_time_ms` placeholder 0을 실측치로 교체. 동시에 lesson 26 "settings.db schema 이중 정의" 해소.

**구현 결과**:
- ✅ `ProcessingMetricsPort` (core 신규 포트, sync default no-op)
  - `record_success` / `record_error` / `record_quarantine` / `record_verify(passed)` / `record_process_time(ms)`
- ✅ `FileProcessingService.metrics_recorder: Option<Arc<dyn ProcessingMetricsPort>>` 필드 추가
- ✅ service.rs 호출 지점 6곳: legacy/pipeline × verify 분기(Pass/Warning/Fail2차) + quarantine 이동 + record_success/error + pipeline 시작·성공 시점 시간 측정
- ✅ `SettingsDbMetricsAdapter` (shared) — settings.db `processing_metrics` 7키 UPSERT 누적. DB 실패는 silent
- ✅ 신규 테이블 `processing_metrics` (success / errors / verified_pass / verified_fail / quarantined / total_time_ms / counted_for_time)
- ✅ `build_service`에서 `paths.base.join("settings.db")`로 자동 주입
- ✅ MCP `get_processing_metrics` + `collect_current_metrics` 응답 placeholder → settings.db 산출치
- ✅ `SETTINGS_DB_SCHEMA: &str` 단일 상수화 — `open()` / `open_in_memory()` 이중 DDL 정의 해소 (lesson 26 완료)
- ✅ 단위 테스트 3건 (increment / summary_empty / summary_rates 0.8 verify, 0.1 quarantine, 1000ms avg 검증)

**데이터 부족 처리**:
- 분모 0인 경우 `ProcessingMetricSummary` 비율 필드 None → JSON null
- `collect_current_metrics`는 None을 0.0/0으로 변환 (기존 RollbackThresholds 동작 보존)

**검증**:
- workspace cargo check ✅ / Tauri cargo check ✅
- lib 테스트 305건 통과 (+3 신규)

**lesson 갱신**:
- lesson 26: "해소 (2026-05-14, Phase 82-prep)" 섹션 추가 — 단일 상수화 완료, lesson 10/26 재발 가능성 차단

### ✅ Phase 64: CLI/UI dead code 정리 + 매핑 정합성 회복 — 완료 (2026-04-30)

**우선순위**: 🔴 High (lesson 13/19 재발 패턴)
**선행**: Phase 60~63 완료
**후행**: 트리거 대기 (실 사용 피드백)

**목적**: 어댑터별 기능 매핑 분석 + MCP Playwright 단위 테스트 중 발견된 9가지 부족 영역 처리. dead code 정리 + 매핑 정합성 회복.

**구현 결과**:
- ✅ 64-1. spec 수치 정정 (CLI 18 vs 12 모달별 분리, Tauri 61→50→**49**)
- ✅ 64-2. CLI Commands enum 이중 정의 제거 — `modals/cli/src/cli.rs`(완전 dead, mod 선언 없음) 삭제
- ✅ 64-3. **백엔드 Tauri commands 11개 dead 정리** — feedback_*(7) + credential_store_*(4) + 보조 함수/struct (FeedbackEntry/find_source_root 등). 326줄 + 28줄 삭제
- ✅ 64-4. `credential_store` shim 보존 결정 (Phase 60 호환성)
- ✅ 64-5. 매핑 보강 — architecture.md에 모달 진입점 표 + Daemon 모드 + 모달 5종 + Pipeline 인터랙티브 + Tauri 카테고리 표 + CLI 서브커맨드 추가
- ✅ 64-6. lesson 19 작성 — UI 기능 제거 시 8단계 → 10단계 체크리스트 (frontend dead code 검출 추가)
- ✅ 64-7. **Phase 61 hierarchy 버그 수정** — `dashboard.js renderSearchResults`가 #search-results 누락으로 silent fail. doc-table 컬럼(계층/접근수)으로 통합 + breadcrumb 렌더 + serde 호환
- ✅ 64-8. **MCP Playwright 17 시나리오 단위 테스트** — 7탭/16 data-action/모달/검색/Pipeline 서브탭/hierarchy 모두 검증
- ✅ 64-9. **frontend dead 함수 8개 정리** (Q1) — renderSearchResults / _renderCredBindings / health / lint / deleteDocument / fixBacklinks / kgPaths / savePipeline / getRetentionConfig + 연쇄 dead (_onRoleCredChange / _updateRoleModelOptions 117줄)

**검증**:
- `cargo check --all`: ✅
- Tauri commands: 60 → **50** (백엔드 11개 정리)
- dashboard.js: 2935줄 → 2805줄 (frontend ~130줄 정리)
- API 함수: 52 → **45** (7개 정리)
- MCP Playwright 17/17 시나리오 통과
- doc-table breadcrumb 렌더 시각 검증 완료

**lesson 갱신**:
- lesson 19: 10단계 체크리스트 + frontend 정합성 grep 패턴
- 신규 발견: renderSearchResults / API.listPipelines/deletePipeline/reorderPipelines (Phase 56 백엔드 삭제 후 잔존)

### ✅ Phase 60: 재사용 모듈 분리 — 완료 (2026-04-29)

**목적**: 도메인 무관 raw 수준 모듈을 형제 프로젝트(`C:\dev\claude_workspaces\` 약 20개)가 path dep로 가져갈 수 있도록 외부 워크스페이스 `C:\dev\claude_workspaces\module\`에 분리.

**확정 결정 (2026-04-28)**:
- **Q1 LLM 어댑터**: 통합 단일 크레이트 (Anthropic/OpenAI/Gemini/Ollama/Claude CLI/Fallback "무조건 같이 제공")
- **Q2 Storage**: 통합 단일 크레이트 (zstd 압축 + S3/WebDAV/Network "내부/외부 같이 제공")
- **Q3 prompts/ChunkedAgent**: **안 C 채택** — `module-llm-prompts` + `module-llm-chunked` 둘 다 신규 분리. 청크 분할+병합 패턴은 형제 프로젝트(로그 분석/대용량 문서 요약 등)에서도 재사용 가치 있음

**구조**: 인터페이스 레이어(`*-api`)와 구현체를 분리한 9 크레이트.
- 외부는 `*-api`만 의존 시 reqwest/keyring 등 무거운 의존 없이 mock/테스트 작성 가능
- 자체 에러 타입(thiserror): `SecretError`, `StorageError`, `NotifyError`, `LlmError`

**진행 상황** (2026-04-29 갱신):
- ✅ 60-1. 워크스페이스 골격 (`module/Cargo.toml`, README)
- ✅ 60-2. **module-secrets-api / module-secrets** — `SecretStorage` trait + `KeyringSecretStore` 구현체
- ✅ 60-3. **단계 0: 9 멤버 placeholder 일괄 생성** (lesson 16 준수)
- ✅ 60-4. **단계 1: module-storage-api / module-storage** — `LocalStoragePort` + `RemoteStoragePort` + `StorageError`(thiserror), zstd/S3/WebDAV/Network/Null 5종 raw. file-pipeline 5개 어댑터 thin wrapper. **lesson 17 작성 완료** (6단계 의존 누수 점검). 검증: module 11/11, file-pipeline 73/73 lib + bench_scale 7/7 (release 단독), Tauri ✅, 누수 0건. dead dep 정리 (zstd/sha2/hex 제거).
- ✅ 60-5. **단계 2: module-notify-api / module-notify** — `NotifyPort{send_text}` + `NotifyError`, Telegram/Slack/Composite/Null raw. file-pipeline 어댑터 thin wrapper, ProcessingSummary→text 포매팅은 `format.rs`로 분리하여 file-pipeline 잔류. 검증: module 2/2, adapters lib 통과, Tauri ✅.
- ✅ 60-6. **단계 3-1: module-llm-prompts** — `TemplateEngine` generic + `SectionSpec`(name/toml_key/default 인자화), TOML 핫 리로드 + 변수 치환. file-pipeline `prompts.rs` 한국어 콘텐츠 + `build_type_hints`/`build_classify_prompt` 등 도메인 빌더만 잔류 (~210줄). 검증: module 8/8 + 1 doctest, file-pipeline 5/5 prompts. dead dep 정리 (toml direct 제거).
- ✅ 60-7. **단계 3-3: module-llm-api / module-llm** (Q1 결정으로 3-2보다 먼저) — `LlmRawPort{call_text(system, user, max_tokens)}` + `LlmError`(Backend/Auth/Parse/Empty/Io/Config/NoProvider/Other). Anthropic/OpenAI/Gemini/Ollama/ClaudeCli + Fallback raw 6종. file-pipeline 5개 LLM 어댑터 thin wrapper, JSON 파싱+도메인 변환은 `response.rs`로 분리(`parse_llm_response`/`parse_sections_from_content`/`build_classify_result`). 검증: module 4/4, adapters 73/73 lib, Tauri ✅, 누수 0건.
- ✅ 60-8. **단계 3-2: module-llm-chunked** — `Splitter` trait + `ByteSplitter`/`FnSplitter` + `ChunkOrchestrator` generic (closure 주입). file-pipeline `chunked_agent.rs`는 LLMPort 위에서 inner 위임 + module ByteSplitter 재사용으로 도메인 결합 회피. 검증: module 6/6, adapters 3/3, Tauri ✅, 누수 0건
- ✅ 60-9. **단계 4: 문서 일괄 갱신 + 정리** — architecture.md/domain-map.md 표기 모순 일괄 수정 (Qdrant 잔존/REST queue/Tauri commands 28→61/chunking·rerank·remote_storage 누락→O/(신규) 표기 제거), `vendor/qdrant.exe` 삭제(83MB), 메모리 `inprogress→done` rename. **형제 시뮬레이션 1회 통과** (빈 임시 크레이트에서 10 module + tokio import → cargo check 통과, 의존 트리 깨끗, anyhow 강제 끌기 없음)

**Q1·Q2·Q3 결정 사항**:
- Q1 단계 순서: chunked는 raw provider 분리(3-3) 뒤로 이동 (chunked가 raw 위에서 동작하므로 순서 의존)
- Q2 형제 시뮬레이션: 매 단계 X, **Phase 60 마지막 1회**만 수행
- Q3 dead code: 분리 작업 중 발견 즉시 정리 (toml/zstd/sha2/hex direct dep 제거 + bench_scale.rs:79 0 나누기 사전 결함 수정)

**Plan 파일**: `C:\dev\ide\claude\profiles\reujea\plans\q1-ethereal-cocke.md` (안 C 반영, 구 `immutable-humming-deer.md` 폐기)

### ✅ Phase 59: 안전한 트리거 대기 항목 선구현 — 완료
- 59-1. **MinHash LSH 강제 활성화 옵션** — `crossref.minhash_force_enable` + `minhash_min_docs`. 자동 임계치(3K) 무시 옵션. VectorDBPort.minhash_enabled_with(force, min_docs)으로 변경
- 59-2. **메타데이터 블로킹** — `crossref.metadata_blocking`. 활성 시 doc_type 또는 키워드 1개 이상 겹치는 후보만 cosine 비교. 기본 비활성
- 59-3. **flush_crossref 통합** — minhash_active + metadata_blocking 양쪽 후보 필터를 snapshot/flushed 양 경로에 적용. 로그에 `minhash=on` / `block=on` 태그
- 59-4. **bench_crossref_variants** — 4축 비교 테스트 (baseline/threshold 0.8/minhash force/metadata blocking/all). HashEmbedder 100문서 실측: 0.8=관계 -57.9%, minhash=관계 -85.4%, all=관계 -94.8%, metadata blocking=관계 동률(키워드 다양성 부족)
- 59-5. **사전 결함 정리** — 제거된 메서드 잔존 테스트 4건 (test_delete_expired, test_purge_expired, e2e_purge_preserves_processed, scenario_purge, test_lint_stale_detection) + StoragePort 미존재 메서드 stub 2곳 + keyring 환경의존 #[ignore]
- 59-6. **빌드 검증** — workspace + tests + Tauri 모두 0 경고/0 에러. lib 테스트 247건 전체 통과. bench_micro_100 회귀 없음 (60.7 docs/s 유지)
- 59-7. **남겨둔 항목** — threshold 디폴트는 0.7 유지 (실 임베딩 검증 필요), BGE-M3 Rust ONNX는 외부 의존(ort 정식 릴리스 대기), ColBERT는 BGE-M3 이후



### 🟡 Phase 63: fastembed Sparse 어댑터 — 완료 (2026-04-29)

**우선순위**: 🟡 Medium
**선행**: Phase 62 (완료)
**후행**: Sparse LocalVectorStore 통합 트리거 대기 (실 코퍼스 측정 후)

**구현**:
- `crates/adapters/src/driven/embedding/fastembed_sparse.rs` 신규 — `FastEmbedSparseAdapter` + `SparseVector{indices, values}` + `dot()` 유사도 계산
- BGE-M3 모델의 sparse(lexical) 출력 활용
- `fastembed` feature flag 격리 (Phase 62와 동일 빌드 요구사항)
- 단위 테스트 3건 통과

**통합 결정**:
- 현재 `keyword_index`(HashMap<String, Vec<doc_id>>)는 문자열 매칭, BGE-M3 sparse는 vocab 인덱스(u32) 기반 — 비호환
- **어댑터만 제공**, LocalVectorStore 통합은 트리거 대기 #10으로 이관
- 통합 시 작업: `sparse_index: HashMap<u32, Vec<(doc_id, weight)>>` 신규 + upsert/search 양쪽 운영

### ✅ Phase 62: fastembed 기반 BGE-M3 임베더 + Cross-Encoder 리랭커 — 완료 (2026-04-29)

**우선순위**: 🔴 High
**선행**: Phase 60 (완료)
**후행**: Phase 61 (청킹 메타데이터) 또는 트리거 대기

**목적**: 전문가 자문(2026-04-29) 결과 채택. fastembed v5.12 크레이트로 BGE-M3 Dense 임베더 + BGE-Reranker-v2-M3 Cross-Encoder 통합 도입. Claude CLI 의존도 감소 + MRR 50% 향상.

**검증 결과 (사전, 통과)**:
- DLL 크래시 없음
- 단건 64ms/건, 배치 100건 평균 61ms/건
- RSS 메모리 1.5~1.7GB
- 자료 기대값(0.05~0.3초/건) 적중

**핵심 결정**:
- Q1: 옵션 A/B/C(Python subprocess) 모두 폐기. fastembed 단일 안 채택
- Q3: ClaudeReranker 교체 (LLM API → fastembed BGE-Reranker-v2-M3 로컬). ClaudeReranker는 fallback으로 유지

**작업 분해**:
- ✅ 62-1. 사전 검증 (DLL/속도/메모리 4항목 통과 — 단건 64ms, 배치 100건 61ms/건, RSS 1.7GB)
- ✅ 62-2. `FastEmbedAdapter` 구현 (`Arc<Mutex<TextEmbedding>>` + `tokio::task::spawn_blocking`)
- ✅ 62-3. `FastEmbedReranker` 구현 (`BGERerankerV2M3`, zstd 헤더 30줄 추출)
- ✅ 62-4. `build_service` 통합 + Settings UI selector 옵션 추가 (default→fastembed)
- ✅ 62-5. Fallback 체인 (fastembed 실패 시 Claude CLI / Stub)
- ✅ 62-6. spec/architecture.md Phase 62 섹션 추가, 임베더/리랭커 표 갱신
- ✅ 62-7. 메모리 `project_phase62_done.md` 작성

**기대 효과**:
- MRR@5: 0.65 → 0.975 (+50%)
- 임베딩 속도: 15초/건 → 64ms/건 (234배)
- 1K문서 초기 투입: 4.2시간 → ~1분 (252배)
- Cross-Encoder 리랭커: ClaudeReranker LLM 호출 → 로컬 ms 단위

**빌드 환경 요구사항**:
- Visual Studio Build Tools 2022 v17.8+ (MSVC v14.38+)
- Windows SDK 10.0.19041.0+
- file-pipeline의 향후 모든 ML 크레이트 도입에도 필요한 보편적 기반

**근거**: `prd/research/bge-m3-fastembed-decision.md`, 실측 검증

### ✅ Phase 61: 청킹 메타데이터 고도화 (G1 + G7) — 완료 (2026-04-29)

**우선순위**: 🟡 Medium
**선행**: Phase 60 (완료)
**후행**: Phase 62 BGE-M3 Phase A 또는 트리거 대기

**목적**: 원문 스마트 청킹 자료(`prd/research/rag-roadmap.md`)의 ① 계층적 청킹 + ⑦ 인덱싱 메타데이터 표준화 통합 적용. 비용 낮음(2일), 외부 의존 0, 호환성 높음.

**구현 결과**:
- ✅ 61-1. `SemanticChunk`에 `title_path: Vec<String>` 추가 (chunking.rs)
- ✅ 61-2. `split_by_headings_with_path` 신규 — H1/H2/H3 path 추적 (기존 `split_by_headings`는 호환용 유지)
- ✅ 61-3. `Metadata.hierarchy: Vec<String>` + `content_type: String` 추가 (Default 구현 + serde default로 lesson 5 준수). 기존 인덱스 backward compat 테스트 추가
- ✅ 61-4. `LocalVectorStore::upsert`에서 `metadata.hierarchy`를 keyword_index에 합침
- ✅ 61-5. `SimilarDoc.hierarchy` 추가 + MCP/Tauri search 응답 JSON에 hierarchy 포함 + dashboard.js 검색 결과에 breadcrumb (`크럼브 › 크럼브`) 표시 + dashboard.css 스타일
- ✅ 61-6. cargo test --all --lib 31/31 + adapters 73/73 통과 (회귀 0건)
- ✅ 61-7. serde default로 기존 인덱스 자동 호환 (legacy_json 테스트 통과)

**기대 효과**: 검색 시 문맥 파악 향상, 추론 정확도 +5~10% (이미 강한 청킹 위에 추가). 실측 후 본 평가.

**근거**: `prd/research/rag-roadmap.md` §3 "Phase 61"

### 🔄 전환 결정 (2026-04-22 전문가 자문)

**"만들기에서 쓰기로 전환"** — 추가 최적화보다 실사용 피드백의 ROI가 높다고 판단.

- 남은 고도화 항목은 전부 **트리거 대기** 또는 **피드백 기반 착수**로 전환
- threshold/TypedSlots/BGE-M3 → 실사용에서 "검색 안 됨" 또는 "관계 노이즈" 피드백 시 착수
- MinHash LSH → 5K+ 문서 도달 시 착수 (코드 준비 완료, 1일 내 도입 가능)
- 교차참조 "양보다 질" 관점: threshold 상향 + cap 축소가 LSH보다 선행

## 보류 → 해소

- BGE-M3 ONNX: 코드+모델 완료 (Phase 40), DLL만 대기 → 트리거 대기 #3
- 외부 저장소: Phase 28 완료
- 교차참조 실환경 검증: 실사용 시 자동 검증됨 (별도 작업 불필요)
- Telegram/Slack 알림: Phase 29 통합 테스트 완료

## 트리거 대기 항목 (실사용 피드백 기반 착수)

> 통합 출처: `prd/research/rag-roadmap.md` §2

### 활성 트리거 (Phase 89 갱신 — 외부 사용자 신호 대기)

**메타**: 자동 진행 가능 항목은 모두 소진 (Phase 89). 남은 항목 전부 외부 사용자 신호 또는 도메인 확장 의존.



| # | 항목 | 트리거 조건 | 준비 상태 | 비용 |
|---|------|------------|----------|------|
| 2 | MinHash 자동 활성 임계치 조정 | 5K+ 다양 도메인 코퍼스 | Phase 59 force/min_docs 노출. **Phase 89 측정 재확인 (485파일 -3.3% 시간 / -19.9% 관계, threshold 0.8의 -74.5% 대비 부차적)** → 디폴트 변경 보류 (`spec/benchmarks/b1_variants_phase89_20260518.json`) | 30분 |
| 4 | 메타데이터 블로킹 디폴트 활성 | 검색 정확도 불만 + 코퍼스 다양성 검증 | Phase 59 옵션화. **Phase 89 측정 재확인 (485파일 +3.2% 시간 / -0.6% 관계, 무효과)** → 디폴트 변경 보류. 다른 도메인 코퍼스 진입 시 재측정 | 30분 |
| 6 | HyDE 폴백 검색 | 실사용 "검색 안 됨" 피드백 | **Phase 86 인프라 + Phase 89 어댑터 활성 완료** — `search.hyde_enabled` configField + `LLMPort.generate_hypothetical` 디폴트 no-op + handle_search 분기 + **Phase 89: claude/anthropic/openai/ollama/gemini 5종 + chunked/fallback/cached wrapper 8종 모두 override + prompts.toml `[hyde]` 추가**. 디폴트 비활성 유지 — 트리거 도달 시 디폴트 1줄 변경으로 즉시 활성 | 디폴트 1줄 |
| 7 | Parent-Child 청크 구조 | 1K+ 코퍼스 MRR 회귀 발견 | 스키마 변경 + mmap 영향 큼. 본 코퍼스 측정에서 회귀 신호 없음 → 착수 보류 | 3일 |
| 8 | 표 마크다운 보존 청킹 | 표 비중 높은 도메인 진입 | **Phase 86 인프라 완료** — `chunking.preserve_tables` configField + `is_table_line` 추적. Java/DB 코퍼스는 표 비중 낮음 → 디폴트 변경 보류 | 디폴트 1줄 |
| 10 | BGE-M3 Sparse LocalVectorStore 통합 | 1K+ 코퍼스 검색 정확도 측정 후 부족 시 | Phase 63 어댑터 제공됨, sparse_index 신규. 본 코퍼스 정확도 측정 미진행 → 착수 보류 | 2일 |
| ~~**A1-hit**~~ ✅ | ~~A1 LLM 캐시 hit률 실측~~ | ~~동일 파일 재가공 hit/miss 데이터~~ | **Phase 89에서 측정 완료** — 9 entries / 9 hits / per-doc 48.1→24.9s (1.93x 가속). `spec/benchmarks/a1_hit_phase89_20260518.json` | 완료 |
| **A2-def** | A2 expand_kg_hops 디폴트 변경 0→1 | 실 LLM 사용자 검색 만족도 측정 | configField + handle_search 후처리 (lesson 30). 측정 미진행 → 보류 | 디폴트 1줄 |
| **B1-def** | B1 diversity_threshold 디폴트 변경 0→3 | 실 LLM dominant doc_type 편향 측정 | configField + dominant swap (lesson 30). 측정 미진행 → 보류 | 디폴트 1줄 |
| ~~**C2-fp**~~ ✅ | ~~C2 PII false positive 측정~~ | ~~100건+ 표본에서 의도치 않은 격리 비율~~ | **Phase 89에서 측정** — 36 docs / 0 격리 / FP 0%. Phase 88 (10건 2 FP) 대비 큰 격차 — DB 도구 문서 특수성으로 추정. PII regex 5종 디폴트 유지. `spec/benchmarks/c2_fp_phase89_20260518.json` | 완료 (도메인 확장 시 재측정) |

### 활성 트리거 — Phase 89 신규 추가 (외부 사용자 신호 / 도메인 확장 / 코퍼스 의존)

| # | 항목 | 트리거 조건 | 준비 상태 | 비용 |
|---|------|------------|----------|------|
| **E-1** | 본인(reujea) 실사용 시작 | 업무 문서를 inbox에 투입 시 | 인프라 모두 완료. PIPELINE_BASE 격리 또는 기본 cwd 환경 어느 쪽도 가능 | 0 (즉시 가능) |
| **E-2** | 동료/외부 사용자 데모·공유 | 사용자 결정 | 단일 바이너리 배포 가능 (pipeline.exe rename + ui/) | 배포 0.5일 |
| **C-d1** | 다른 도메인 코퍼스 측정 | 의료/금융/공공 문서 코퍼스 보유 시 | PII regex 5종 + 가공 파이프라인 동작 검증됨 | 측정만 |
| **C-d2** | 5K 합성 코퍼스 생성·측정 | 사용자 결정 | `spec/benchmarks/scripts/gen_synthetic_corpus.ps1` 사용 가능 | 생성 + 측정 1일 |
| **C-d3** | D:/file-test/samples 도메인 확장 | 사용자 결정 | 현재 1211파일(485 가공 가능)은 Java/JSP/CSS/DB/Web 위주 | 0 (코퍼스 추가만) |
| **D-1** | webapp-design v2 자문 | UI/UX 외부 자문 필요 시 | Phase 56 자문 이후 Phase 65~88 IA 변동 누적 (`spec/webapp-design.md` 변동 이력 섹션) | 외부 자문 |
| **D-2** | Decision Log 후속 UI | 사용자 요구 시 | Phase 82 의도된 비범위 — Proposal/Decide 2단계 분리 / 거부 reason 자유입력 / 자동 결정 정책 | 1~2일 |
| **F-1** | architecture.md 재아카이빙 | 본문 1900줄+ 도달 시 | Phase 85의 65~78 분리 패턴 반복. 현재 Phase 80~88 아카이빙 대상 후보 | 0.5일 |
| ~~**G-1**~~ ✅ | ~~Claude CLI exit code 1 산발 실패 진단~~ | ~~실 사용 시 재발 시~~ | **2026-05-20 종결** — 격리 환경 재현 9/9 성공 → 외부 일시 요인 결론. F-1~F-5 코드 강화 (claude_cli.rs: 300s timeout / stderr 200자+elapsed / 빈 stderr+exit1 1회 자동 재시도 / stdin flush+drop / service.rs LLM 호출 실패 quarantine 라우팅). lesson 46 본문 갱신 | 완료 |
| **G-2** | fastembed feature ON release 빌드 | MRR 측정 / 정확도 시나리오 진입 시 | 현재 release 빌드 fastembed OFF → Claude CLI 임베딩(128축) 폴백. fastembed ON 빌드 26m+ (lesson 18 MSVC v14.38+ 필요) | release 빌드 30분 |
| ~~**G-3**~~ ✅ | ~~Tauri 재빌드 (Phase 90 Notion 반영)~~ | — | **2026-05-20 종결** — Tauri 재빌드 7m 19s로 Phase 90 Notion + F-1~F-5 모두 반영. file-pipeline-tauri.exe 22.14 MB | 완료 |
| ~~**G-4 (a)**~~ ✅ | ~~Pipeline 서브탭 dead-code 정리~~ | — | **2026-05-20 종결** — pb-subtabs는 invoke 의존이 아닌 **HTML 엘리먼트 부재 dead-code**로 재진단. dashboard.js 6 함수 271줄 + CSS 5 rule 삭제 + lesson 47 신규 + 메타 룰 1 14번째 사례 | 완료 |
| ~~**G-4 (b)**~~ ✅ | ~~Verification 카드 invoke-no-fallback~~ | — | **2026-05-20 종결** — 빈 객체 `{}` truthy 가드 강화 (`!m \|\| typeof m.total !== 'number'`). "TOTAL undefined" → 친절 메시지 | 완료 |
| ~~**G-5**~~ ✅ | ~~GUI 회귀 테스트 자동화~~ | — | **2026-05-20 종결** — `spec/benchmarks/scripts/` 5종 (action_catalog / dead_selector_scan v1/v2 / empty_state_audit / data_flow_trace / gui_http_smoke) + README + META.md Phase 종결 체크리스트 + git pre-push hook | 완료 |
| ~~**G-6**~~ ✅ | ~~dead_selector 14건 분류~~ | — | **2026-05-20 종결** — 13건 진짜 dead 일괄 정리 (dashboard.js -411줄: 10 함수 + 7 case + 5 if + 11 API). 1건 false positive(settings-no-results) whitelist | 완료 |
| ~~**G-7**~~ ✅ | ~~Tauri commands 9건 백엔드 정리~~ | — | **2026-05-20 종결** — commands.rs -366줄 (10 함수 본체 + invoke_handler 10건). 빌드 통과. 사이드: `ListParams` / `mask_secrets` / `restore_masked_secrets` 정의 추가 (lesson 12 패턴 응용) | 완료 |

5K 합성 코퍼스 생성 스크립트: `spec/benchmarks/scripts/gen_synthetic_corpus.ps1` (lesson 32) — Phase 86 측정에서 485파일 대체 사용. 5K+ 다양 도메인 코퍼스가 #2/#4 디폴트 변경 결정에 여전히 필요.

### 트리거 #2/#4 실측 의사결정 (2026-05-14)

D:\file-test\samples 325파일 (HashEmbedder + bench_real_corpus_variants):

| 변형 | 시간 vs baseline | 관계 vs baseline | 결정 |
|------|------------------|------------------|------|
| baseline (0.7) | — | 10034 | 기준 |
| threshold 0.8 (#1 적용됨) | -4.5% | -73.7% | ✅ 이미 적용 |
| MinHash force (#2) | +9.3% | -21.0% | ❌ 디폴트 변경 보류 |
| Metadata blocking (#4) | -8.3% | 0% | ❌ 디폴트 변경 보류 |
| all (0.8+mh+block) | +7.4% | -79.5% | 과적용 |

5K+ 코퍼스 또는 실 사용자 피드백 도달 전엔 디폴트 보류. lesson 15 인프라(force/min_docs/metadata_blocking 옵션화)는 정상 작동 — 트리거 도달 시 코드 변경 0건으로 켤 수 있음.

### 처리 이력 (Phase 62~64에서 흡수 또는 완료)

| # | 항목 | 처리 결과 |
|---|------|----------|
| 1 | threshold 디폴트 상향 (0.7→0.8) | ✅ 2026-04-30 — config 기본값 0.8 + FieldMeta 동기화 |
| 3a | BGE-M3 Python production | ❌ Phase 62 fastembed 채택으로 폐기 |
| 3b | BGE-M3 Rust 네이티브 (ort 대기) | ❌ Phase 62 fastembed가 ort-sys 정적 링크로 즉시 가능 |
| 3c | BGE-M3 Sparse + Cross-Encoder | ✅ Phase 62/63 흡수 — fastembed Reranker + Sparse 어댑터 |
| 5 | ColBERT late interaction | ❌ BGE-M3 Reranker로 대체. 별도 진행 불필요 |
| 9 | Cross-Encoder 리랭커 | ✅ Phase 62 흡수 — fastembed BGE-Reranker-v2-M3 채택 |
| 11 | onnx feature 폐기 + vendor 정리 | ✅ 2026-04-30 — onnx_embed.rs(260줄) + ort/tokenizers dep + vendor/onnxruntime 394MB 삭제 |
| 12 | bench p95 회귀 재측정 | ✅ 2026-04-30 — 3회 중앙값 23.62 docs/s, p95 48.3ms (회귀 없음, +22% 향상) |

## 완료된 고도화 (참고)

모든 ✅ 항목은 이전 Phase에서 구현 완료. 상세: 각 Phase 섹션 참조.

## 제거됨

| 항목 | 제거 사유 |
|------|-----------|
| PDF/OCR 전처리 (6-6) | 파일 가공은 claude_cli 전용. 별도 전처리기 불필요 |
| 민감 config 내용 기반 탐지 | watcher 스킵으로 충분, 별도 구현 불필요 (2026-04-14) |
| Qdrant embedded 모드 | Qdrant 완전 제거됨 (Phase 44). 복귀 가능성 없음 (2026-04-21) |
| Qdrant named vector | Qdrant 완전 제거됨 (Phase 44). LocalVectorStore 단일 (2026-04-21) |
| 모바일 빌드 (iOS/Android) | Desktop 전용으로 결정 (2026-04-22) |
| GraphDB (json_graph/neo4j_graph) | Phase 58에서 코드 삭제. KG는 LocalVectorStore find_related로 충분 |
| Feedback 탭 | Phase 55에서 비활성, Phase 58에서 JS dead code 삭제. Rust 코드 유지 |
