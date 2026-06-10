---
purpose: architecture.md에서 분리한 Phase 79 이하 처리 이력 아카이브
分離日: 2026-05-15 (Phase 85에서 65~78 추가 분리)
분리 범위: Phase 65~78 (2026-05-04 ~ 2026-05-07) + 트리거 #1/#11/#12 + release 빌드 검증 + Phase 61~64
원본: spec/architecture.md (현재 본문에는 Phase 80 이후만 유지)
---

# Architecture Archive — Phase 79 이하

본 문서는 `architecture.md` 본문에서 분리한 옛 Phase 처리 이력이다. **읽기 전용 아카이브**. 신규 갱신은 `architecture.md` 본문에 하고, 옛 Phase가 새 결정에 영향을 줄 때만 이 문서를 인용한다.

> **단일 진실원 위임** (2026-05-20, 옵션 A): "무엇이 삭제·폐기·보류되었는가"는 `spec/deprecated.md` 가 단일 진실원. 본 아카이브는 **"왜 그 Phase에 그 결정을 했는가"** (결정 맥락 / build_service 영향 / 검증 결과) 만 보존하고, 삭제 항목 인벤토리는 `deprecated.md` 로 링크 위임. 메타 룰 1 "다중 위치 동기화 누락" 자기 적용.

목차 (최신 → 옛순):
- Phase 78 처리 (2026-05-07): setup_dryrun + 사용 패턴 자동 프로파일링
- Phase 77 처리 (2026-05-07): ConfigSnapshot + 자동 롤백
- Phase 76 처리 (2026-05-07): 5축 SetupProfile + 선언적 룰 + Critical 차단
- Phase 75 처리 (2026-05-06): AI 설정 도우미 MCP 안내 모달
- Phase 73+74 처리 (2026-05-06): setup_review 백엔드 + Dashboard 모달
- Phase 72 처리 (2026-05-06): 26 어댑터 단위 테스트 +25
- Phase 70+71 처리 (2026-05-06): config 신규 4섹션 + 노드 3건 + Settings 그룹
- Phase 69 처리 (2026-05-04): configFields 메타 + 검색 18→20노드
- Phase 68 처리 (2026-05-04): 노드 시각 마커 + 인스펙터 3영역 분리
- Phase 67 처리 (2026-05-04): 가운데 4서브탭 폐기 + 인스펙터 480px 편집
- Phase 66 처리 (2026-05-04): Phase 65 부분 원복 + Pipeline 3컬럼
- Phase 65 처리 (2026-05-04): 3계층 IA + fastembed 고정 + dead config 5건 제거
- 트리거 #11 처리: onnx feature 폐기 + vendor/onnxruntime 정리 (2026-04-30)
- 트리거 #12 처리: bench p95 회귀 재측정 (2026-04-30)
- 트리거 #1 처리: threshold 디폴트 0.8 상향 (2026-04-30)
- release 빌드 검증 (2026-04-30)
- Phase 64: CLI/UI dead code 정리 + 매핑 정합성 회복 (2026-04-30)
- Phase 63: fastembed Sparse 어댑터 (2026-04-29)
- Phase 61: 청킹 메타데이터 고도화 (2026-04-29)
- Phase 62: fastembed 통합 (2026-04-29)

---

## Phase 78 처리 (2026-05-07)

setup_dryrun + 사용 패턴 자동 프로파일링 (전문가 답변 §4.4 + §1.4 장기안).

### 신규 모듈 `setup_dryrun.rs`
- `diff_configs(before, after)` → DryRunReport: 단계 분류(Chunking/Embedding/Verify/...) + 경고(차원 변경=재색인, retention=데이터 손실)
- `infer_profile_from_usage(CorpusUsageStats)` → SetupProfile (doc_type 분포 → ContentType 매핑, sensitive_ratio → Sensitivity, weekly_recent → Volume, 검색 모드 분포 → SearchIntent)
- `detect_mismatch(saved, inferred)` → 저장된 프로파일과 실제 사용 패턴의 축별 불일치 표시

### MCP 도구 2개 추가
- `setup_dryrun` — 추천 적용 결과 미리보기 (실행 없음). diffs / stage_summary / warnings.
- `setup_profile_infer` — 코퍼스 통계에서 SetupProfile 자동 산출. saved_profile 입력 시 mismatch 함께 반환.

### 단위 테스트 +6
- diff_configs (chunking/dim/retention 경고)
- infer_profile (meeting corpus/high sensitivity/temporal intent)
- detect_mismatch volume

## Phase 77 처리 (2026-05-07)

ConfigSnapshot + 효과 측정 + 자동 롤백 (전문가 답변 §4).

### 신규 모듈 `config_snapshot.rs`
- `ConfigSnapshot` 구조: id/created_at/config_hash/config_backup/profile_json/applied_paths/metrics_json/rolled_back/rollback_reason
- `create_snapshot` — apply 직전 pipeline.toml 원본을 SHA256 해시와 함께 보존
- `rollback_snapshot` — config_backup을 파일로 복원, 현재는 .pre-rollback.bak으로 보존
- `evaluate_rollback(before, after, thresholds)` — 4개 트리거(verify_pass_drop_pp=15%p / quarantine_rate_max=10% / process_time_factor_max=2.0)

### settings.db 신규 테이블
- `config_snapshots` (id PK, created_at, config_hash, config_backup, profile_json, applied_paths JSON, metrics_json, rolled_back, rollback_reason)
- 인덱스: `idx_snapshots_created` (DESC)

### setup_apply 흐름 변경
- `apply_advice_full(path, advice, accepted, apply_critical, db?)` → `ApplyResult { applied, snapshot_id }`
- DB 제공 시 자동으로 ConfigSnapshot 생성·저장. 호환 wrapper `apply_advice_with_options`도 유지.

### MCP 도구 3개 추가
- `setup_snapshot_list` — 최근 스냅샷 목록 (id/created_at/applied_paths/rolled_back/has_metrics)
- `setup_snapshot_rollback` — 지정 ID로 복원, .pre-rollback.bak 보존, DB 마킹
- `setup_snapshot_measure` — 현재 metrics 측정·저장 + (compare_to 또는 직전 측정) 자동 비교 → should_rollback 권고

### Tauri 명령 2개 추가
- `setup_snapshot_list`, `setup_snapshot_rollback`

### 단위 테스트 +6
- create_snapshot / rollback_restores / evaluate_rollback (quarantine/verify_drop/no_trigger) / metrics_serde

### 알려진 한계
- `verify_pass_rate` / `quarantine_rate` / `avg_process_time_ms`는 vector_db.stats()에 노출되지 않아 placeholder 0. 추후 service.summary()를 MCP로 노출하면 채워짐. (Phase 82-prep에서 해소)

## Phase 76 처리 (2026-05-07)

다축 프로파일 + 선언적 룰 엔진 + 충돌 해소 + Critical 차단 + toml_edit 주석 보존.

전문가 답변 (`prd/queries/initial-setup-advisor-experts.answer.md`)의 §1~3 권장안 전면 반영.

### setup_review.rs 전면 재설계

**5축 SetupProfile** (단일축 시나리오 5종 폐기):
1. `content_mix: Vec<(ContentType, f32)>` — meeting/research/code/legal/general 비율
2. `sensitivity` — low/medium/high/regulated
3. `volume` — light/moderate/heavy
4. `search_intent` — precision/exploration/temporal
5. `collaboration` — solo/small_team/team

`infer_profile_from_text(s)` — 자유 텍스트에서 5축 추론 (호환). 키워드 매칭 기반.

### 룰 테이블 (`setup_rules.toml`)
- 별도 TOML 파일 (include_str! 임베드, 빌드 타임 검증)
- 4개 priority(P0/P1/P2) × 4개 risk(low/medium/high/critical) × 4개 evidence(heuristic/benchmark/literature/user_feedback) × 3개 confidence
- 전문가 답변 §3.2~3.4 매트릭스 그대로 이관: content축 27룰 + sensitivity축 7룰 + volume축 6룰 + search_intent축 4룰 + collaboration축 2룰 (총 46룰)

### RecommendationEngine
- `evaluate(profile, current)` — 매칭 룰 수집 → path별 그룹핑 → 충돌 해소 → 현재 값과 비교 → ConfigChange 생성
- 충돌 해소: 동일 path 다중 매칭 시 (1) 같은 값이면 P0 우선, (2) 다른 값이면 비율 가중 우세 그룹 선택, (3) 손실된 그룹은 `conflict_note`로 표시

### ConfigChange 확장
- `priority` (P0/P1/P2)
- `risk` (low/medium/high/critical)
- `evidence` (heuristic/benchmark/literature/user_feedback)
- `confidence` (low/medium/high)
- `reversible`, `needs_restart`, `conflict_note`

### apply_advice 재작성 (toml_edit)
- toml_edit::DocumentMut 사용 — 기존 키의 decor(주석/공백) 보존
- Critical 등급은 `apply_critical=true` 명시 시에만 적용 (retention.enabled 보호)
- 적용 후 PipelineConfig::load_from_str로 재파싱 검증 — 파싱 실패 시 적용 거부

### MCP/Tauri 시그니처 변경
- `setup_review` / `setup_apply`: scenario(자유 텍스트) + profile(다축) 둘 중 하나. profile이 우세.
- `setup_apply`에 `apply_critical` 추가

### Tauri UI (5축 폼)
- 시나리오 5 카드 → 5축 입력 폼: content_mix(슬라이더 5개) + 4개 select
- ConfigChange 표시: priority/risk 배지 + evidence 태그 + conflict_note 경고
- Critical 항목은 기본 미체크 + "Critical 적용 동의" 토글 필수

### 단위 테스트 +14
- default_rules_parse / infer_profile (meeting/mixed/general) / classify_scenario_compat
- evaluate (meeting/code/conflict) / evidence_priority / sensitivity_keywords / heavy_volume
- apply_subset / apply_creates_backup / apply_blocks_critical / toml_edit_preserves_comments

### 의존성 추가
- workspace shared/Cargo.toml: `toml_edit = "0.22"`

## Phase 75 처리 (2026-05-06)

Phase 74의 정적 시나리오 모달을 사용자 의도(AI와 대화하며 리뷰)에 맞춰 MCP 안내 모달로 재구성.

### 헤더 진입점
- "🤖 AI 설정 도우미" 버튼 (라벨 변경: 설정 도우미 → AI 설정 도우미)

### 메인 모달 — Claude Code MCP 안내
3섹션 구조:
1. MCP 서버 진입 명령 (`file-pipeline-tauri.exe serve`)
2. Claude Code mcpServers JSON 등록 예시
3. 자연어 사용 예시 3건 (회의/연구/코드)
+ 등록된 MCP 도구 13개 카테고리별 표시
+ 하단 "직접 추천 받기 (Tauri 단독)" 폴백 버튼

### 폴백 모달 (Tauri 단독)
- Phase 74의 정적 시나리오 카드 + 자유 입력 그대로 유지
- AI 대화형 리뷰 사용 불가 환경(Claude Code 미설치)에서 사용

### 사용자 의도 흐름
```
Dashboard 사용자
  → "🤖 AI 설정 도우미" 클릭
  → MCP 안내 + Claude Code 등록 가이드
  → Claude Code에서 자연어 ("회의록 위주 — 추천해줘")
    → AI가 setup_review MCP 호출
    → AI가 추천 + 사용자에게 확인
    → 사용자 승인 후 AI가 setup_apply 호출
    → ✓ 적용됨

  대안 (Claude Code 없을 때):
  → "직접 추천 받기" → Phase 74 정적 모달
```

### 설계 결정
- AI tool_use 자율 호출 → 사용자 명시 승인 (Q3 권장안 B, lesson 12 패턴)
- LLM 우선순위: 외부 Claude Code가 관리 (default_credential 의존 없음)
- 채팅 히스토리: 외부 Claude Code 세션이 관리

## Phase 73+74 처리 (2026-05-06)

설정 항목 75+ 필드 부담 해소를 위해 시나리오 기반 자동 추천 도입.

### Phase 73: setup_review 모듈 (백엔드)
- `crates/shared/src/setup_review.rs` 신규
  - `SetupAdvice` / `ConfigChange` 구조체
  - 5종 시나리오 분류기 (`classify_scenario`):
    meeting / research / code / mixed / general
  - `build_advice(scenario, role, current_config)` — 추천 변경사항 생성
  - `apply_advice(path, advice, accepted_paths)` — .bak 백업 후 적용
- 단위 테스트 11건 (분류 5종 + advice 4종 + apply 2종)

### MCP 도구 2개 추가 (Claude Code 사용자용)
- `setup_review(scenario, user_role)` — 시나리오 분석 → 추천 변경
- `setup_apply(scenario, accepted_paths)` — 사용자 승인 후 적용
- 총 MCP 도구 11 → 13개

### Phase 74: Dashboard 설정 도우미 모달 (Tauri 사용자용)
- 헤더에 "🤖 설정 도우미" 버튼 추가
- 모달 구조:
  - 시나리오 5종 카드 선택 또는 자유 텍스트 입력
  - 추천 변경 사항 표 (적용 체크 + 현재 vs 추천 + 이유)
  - 부분 선택 적용 + 백업 안내 + 재시작 필요 표시
- Tauri commands 추가: `setup_review`, `setup_apply`
- 적용 후 자동 reload (Pipeline/Settings 재렌더)

### 시나리오별 추천 매핑
| 시나리오 | 추천 변경 |
|---------|----------|
| meeting | chunking.target_bytes=2000 / crossref.cap_related=30 / lint=6h |
| research | rerank.enabled=true / similarity_threshold=0.85 / 코드펜스 보존 / 무제한 보존 |
| code | 코드펜스 보존 / sensitive.extensions += [.env/.pem/.key/.p12] / pdf=marker |
| mixed | (변경 없음 — 기본값 유지) |
| general | lint_interval_hours=24 |

### 안전장치
- pipeline.toml.bak 자동 백업 (lesson 12 패턴)
- 부분 적용 (사용자가 체크박스로 선택)
- needs_restart 플래그 자동 표시
- 알 수 없는 path는 적용 거부 (apply_single_change 안전 매칭)

### 테스트 통계
- 이전 260 → 271 (+11 setup_review)

## Phase 72 처리 (2026-05-06)

26개 어댑터(driven 9카테고리) + driving 3 + stub 5 전수 점검. 책임 분리·헥사고날 위반 0건 확인.

### 단위 테스트 +25건 추가 (260 = 252 + 8 신규 + nextest 차이)
- notification/format.rs: 8건 (텔레그램/슬랙 포매팅 + summary truncate)
- stub.rs: 9건 (StubLlm/StubEmbedder/PlainText/Duplicate/Sensitive)
- storage/remote_null.rs: 4건
- storage/webdav_storage.rs: 2건 (is_configured 항상 true 검증)
- storage/s3_storage.rs: 2건

### 의도된 단위 테스트 비대상
- Telegram/Slack/Anthropic/OpenAI/Gemini/Ollama (HTTP/API 호출 → 통합 테스트)
- WebDAV/S3 (외부 서버 → is_configured만 단위 검증)
- claude_verifier/claude_reranker (Claude CLI → 통합 테스트)

### 검증
- workspace cargo test --all --lib --features fastembed: 260 통과 / 0 실패
- adapters 99 / core 130 / shared 31

## Phase 70+71 처리 (2026-05-06)

Phase 69 누락 점검 결과 우선순위 1~2 일괄 구현. 신규 config 섹션 4개 + 노드 3개 + Settings 그룹 1개.

### config.rs 신규 섹션 (Phase 71)
```
[memory_tier]   hot_days(7) / warm_days(30) / cold_days(90)
[search]        window_lines(5) / mmr_lambda(0.5) / sparse_weight(1.0) / time_weight(0.10)
[notification_batch] summary_interval_secs(30)
[crossref]      flush_interval_secs(30) — 신규 필드 추가
```

### 신규 가공 노드 3건 (24노드)
```
⛔ Quarantine 분기   — Verify 2-Pass 실패 후 격리 (configFields verification.on_fail)
🌡 Memory Tier      — 자동 분류 + hot/warm/cold 임계 노출 (configSections memory_tier)
🩺 Lint              — 정합성 검사 주기 (configFields schedule.lint_interval_hours)
```

### 검색 노드 매핑 확장 (Phase 71)
```
🔤 query_expand → search 섹션 (검색 모드 5종 안내)
🔀 hybrid       → search.sparse_weight
🪄 fuse         → search.time_weight (vector_db에서 search로 이동)
🪟 win          → search.window_lines
🌈 mmr          → search.mmr_lambda
```

### Settings 4그룹 (Phase 70 — Hooks 추가)
```
크레덴셜 관리 / 일반 / 이벤트 훅 / 마이그레이션
```
- 이벤트 훅: HookDefinition 배열 표시 (편집은 TOML 직접, Phase 84에서 CRUD 모달 추가)

## Phase 69 처리 (2026-05-04, 부분 보완)

Phase 67/68 작업 후 노드/설정 누락 점검 결과 우선순위 1~2 항목 즉시 보완.

### configFields 메타 신규 (섹션 내 특정 필드만 추출)
기존 `configSections` (섹션 전체)와 별개로 `configFields: [['section', 'field'], ...]` 추가.
한 노드에 다른 섹션의 일부 필드만 매핑할 때 사용.

### 가공 노드 보완
- Fragment 감지 → schedule.fragment_threshold 노출 (configFields 사용)
- 벡터DB 색인 → vector_db.search_top_k + rrf_multiplier 노출

### 검색 파이프라인 18 → 20노드
가공 후처리와 미러 보완:
- 🔗 관련 문서 첨부 (kg_attach) — 가공의 교차참조 미러
- 🏷 엔티티 하이라이트 (entity_hl) — 가공의 엔티티 추출 미러
- paging 노드 → vector_db (search_top_k + rrf_multiplier) 매핑

## Phase 68 처리 (2026-05-04)

Phase 67 인스펙터 고도화에서 "어떤 노드가 설정 가능한지" 시각 구분 부재. 정보/설정/자동 영역도 평면 배치라 사용자 시선 분리가 약함.

### 노드 카드 시각 마커
- `_pbNodeHasSettings(def)` — configSections 또는 fields 존재 시 true
- 설정 가능 노드: ⚙ 아이콘 + 시안 보더 + 100% 불투명
- 자동 노드: 마커 없음 + 75% 불투명
- 검색 파이프라인 노드도 동일 마커 (`_searchNodeHasSettings`)

### 인스펙터 3영역 분리
- **헤더**: 아이콘 + 라벨 + 배지 (⚙ 설정 가능 / 자동 동작)
- **info-block** (회색 surface): 설명 + 왜? + 어떻게?
- **settings-block** (시안 액센트 배경 + 보더): 설정 form + 저장 버튼
  - 글로벌 설정 ↔ 노드 옵션 구분 (.setting-scope / .setting-scope.step)
- **auto-block** (점선 보더 + dim): 자동 동작 노드 표시

## Phase 67 처리 (2026-05-04)

Phase 66 가운데 4서브탭 중복 폐기 + 인스펙터로 편집 통합. 노드 누락 보완.

### 가공 파이프라인 17 → 21노드 (4 추가)
- ✂ Chunking — Preprocess와 LLM 사이 신규
- ☑ Todo 병합 — 후처리 신규 (체크박스 + 키워드 7종)
- 🗂 토픽 자동 병합 — 후처리 신규 (auto_merge_threshold 도달 시)
- (input_source는 Phase 66 추가)

### 가공 파이프라인 순서 (사용자 spec 기준)
```
공통 전처리 (5):
  📂 입력 소스 → 🔒 민감 → 📋 Fragment → #️⃣ SHA-256 → 🔄 증분
파이프라인 스텝 (4):
  📄 Preprocess → ✂ Chunking → 🤖 LLM → ✅ Verify
공통 후처리 (12):
  🧮 Embedding → 🔍 의미중복 → 📦 저장+압축 → ☁ 원격업로드
  → 🗄 벡터DB → 🔗 교차참조 → 🏷 엔티티 → ☑ Todo 병합
  → 🗂 토픽 병합 → 🔔 알림 → 📅 증분기록
```

### 검색 파이프라인 17 → 18노드
- 🌈 MMR 다양성 신규 (Parent Expand 다음)

### 가운데 영역 단순화
- pb-subtab-content 컨테이너 제거 (4서브탭 폐기)
- _renderPBSubtabs / _renderPBSubtabContent 호출 제거
- 가운데 = 21노드 플로우만 (가공) / 18노드 (검색) / 3섹션 (배치)

### 우측 인스펙터 고도화 (320 → 480px)
- 노드별 configSections 매핑으로 form 직접 노출
- 가공 노드: paths/sensitive/preprocessing/chunking/models/verification/verification.thresholds/vector_db/compression/remote_storage/crossref/preprocessing(토픽)/notification 등
- 검색 노드: sensitive/vector_db/rerank
- 배치 섹션: schedule/max_workers/retention/preprocessing(토픽)
- 저장 버튼 인스펙터 푸터 통합 + 상태 표시 (저장 중/✓ 저장됨)
- credential 드롭다운 + 프롬프트 편집 모달 진입

## Phase 66 처리 (2026-05-04, Phase 65 부분 원복)

Phase 65 3계층 IA(입력·가공/검색·MCP/운영)를 7탭으로 원복하면서 Pipeline 탭 내부를 새 구조로 재설계.

### 7탭 원복
```
[Documents] [Pipeline] [Processing] [Todos] [Settings] [Topics] [Verification]
```

### Pipeline 탭 3컬럼 + 가운데 3탭 + 인스펙터
좌측 사이드바(280px) + 가운데 [가공/검색/배치] 탭 + 우측 인스펙터(320px). Phase 67에서 가운데 4서브탭 폐기 + 인스펙터 480px로 확장.

### 가공/검색/배치 파이프라인
- 가공: input_source 첫 노드 추가 (18→20노드)
- 검색: 17노드 신규 (가공과 1:1 미러)
- 배치: schedule + max_workers + retention 흡수

### Settings 탭 5그룹 → 3그룹 축소
- 크레덴셜 관리 / 일반 / 마이그레이션 (유지)
- 스케줄·경로 → Pipeline 배치로 이전
- 인프라(리랭킹) → Pipeline 검색 Rerank 노드 인스펙터로 이전

## Phase 65 처리 (2026-05-04)

3계층 IA + fastembed 고정 + dead config 5건 제거.

### 65-1: 1차 그룹 nav 도입
**[입력·가공] [검색·MCP] [운영]** — URL 해시 라우팅(#group/tab) + 그룹 내 마지막 탭 기억. (Phase 66에서 7탭으로 원복)

### 65-2: fastembed 고정 정책
- `default_embed_model` open_ai_small → fastembed
- `default_rerank_provider` claude_cli → fastembed
- `RerankConfig.enabled` false → true
- **dead config 5건 제거**: → `spec/deprecated.md` 의 "dead config 5건 (Phase 65-2)" 참조
- Pipeline Embedding 노드 단순화: 모델 드롭다운 + onnx_model_dir 제거 → "fastembed BGE-M3 (1024차원, 고정)" 표시

### 65-3: 검색·MCP 그룹 신설
서브탭: Documents / 검색 시뮬레이션 / 리랭킹 / MCP 도구.
검색 시뮬레이션은 search 호출 결과를 단계별 표시 (search_with_trace는 후속 구현 — Phase 84에서 완성).
MCP 도구는 11개 목록 + 호출 통계 (enable/disable 토글은 Phase 84에서 완성).

### 65-4: 운영 그룹 + Settings 흡수
**Settings 탭 완전 폐기** (Phase 66에서 부분 원복). 5그룹을 운영 그룹의 시스템 서브탭으로 분산. `[notification]` / `[sensitive]` / `[retention]`이 처음으로 UI에 노출됨.

---

## 트리거 #11 처리: onnx feature 폐기 + vendor/onnxruntime 정리 (2026-04-30)

**결정 맥락**: Phase 62 fastembed가 ort-sys 정적 링크로 동일 기능을 더 안정하게 제공. Rust 네이티브 ort load-dynamic 방식의 onnx_embed.rs는 deprecate에서 완전 폐기로 진행.

**삭제 항목 인벤토리**: → `spec/deprecated.md` 의 `vendor/onnxruntime/` 디렉토리 + `ort` + `tokenizers` optional dep + `[features] onnx` 섹션 참조

**build_service 단순화**:
- `default_model = "onnx" | "bge_m3"` 분기에서 onnx feature cfg 제거
- PythonOnnx legacy fallback만 남음 (Python 환경 보유 사용자)
- 권장 메시지 추가: "default_model=fastembed로 변경 권장"

**검증**: workspace + Tauri 빌드 통과 / lib 테스트 31/31 / 컴파일 경고 0건

## 트리거 #12 처리: bench p95 회귀 재측정 (2026-04-30)

이전 단일 측정에서 발견된 p95 67.7→153.35ms 회귀 신호를 3회 중앙값으로 재측정.

| 회차 | docs/s | p95 (ms) | avg (ms) |
|------|--------|----------|----------|
| 1 | 20.14 | 84.5 | 49.4 |
| **2 (중앙값)** | **23.62** | **48.3** | **42.2** |
| 3 | 28.16 | 42.7 | 35.4 |

**결정**: 회귀 없음. 이전 단일 실행은 lesson 04 "캐시 편향" 패턴 (다른 벤치 동시 실행 환경 노이즈). 모든 기준선 통과 + Phase 60~64 누적 변경 후 **+22% 향상** (이전 19.36 → 현재 23.62).

## 트리거 #1 처리: threshold 디폴트 0.8 상향 (2026-04-30)

`config::default_similarity_threshold` 0.7 → 0.8 변경. Phase 59 `bench_crossref_variants` HashEmbedder 100문서 실측: 0.8에서 관계 -57.9% (노이즈 감소). FieldMeta 디폴트도 동기화.

bench_scale은 명시 0.7 사용으로 무관. lib 테스트 31/31 통과.

## release 빌드 검증 (2026-04-30)

- workspace release: ✅ 1m 22s
- Tauri release: ✅ 4m 01s
- 바이너리: pipeline.exe **15.5MB** / file-pipeline-tauri.exe **19.4MB**
- bench_scale 100/1000 release 단독: ✅ 통과
- bench_micro_100 단독: ✅ 통과 (다중 동시 실행에서는 자원 경합으로 회귀 — lesson 04 패턴)
- 트리거 #12에서 3회 중앙값 재측정 결과 회귀 없음 — 23.62 docs/s, p95 48.3ms (이전 19.36 → +22%)

## Phase 64: CLI/UI dead code 정리 + 매핑 정합성 회복 (2026-04-30)

**결정 맥락**: 어댑터별 기능 매핑 분석 + MCP Playwright 단위 테스트 중 발견된 9개 부족 영역 처리.

**삭제 항목 인벤토리**: → `spec/deprecated.md` 참조
- `feedback_*` Tauri commands 7건
- `credential_store_*` Tauri commands 4건
- `cli.rs` 파일 (`modals/cli/src/cli.rs`)
- `get_health` / `get_lint` / `delete_document` / `fix_backlinks` / `get_retention_config` / `get_pipeline` / `save_pipeline` Tauri commands

**Frontend 측 정리 효과** (architectural 변경, deprecated.md 비대상):
- `renderSearchResults` 함수 — `#search-results` ID 누락으로 silent fail (Phase 61 hierarchy UI 미작동의 진짜 원인)
- `_renderCredBindings` + 연쇄 dead `_onRoleCredChange / _updateRoleModelOptions` 117줄 삭제
- 백엔드 commands: 60 → **50개**. API 함수: 52 → **45개**. dashboard.js: ~2935줄 → 2805줄

**Phase 61 hierarchy UI 수정**:
- `doSearch`에서 hierarchy/access_count/topic을 `state.documents`에 매핑
- `renderDocList`에 계층 컬럼 + 접근수 컬럼 + breadcrumb 렌더 (4셀 → **6셀**)
- `td.hierarchy` 스타일 (`.crumb` / `.sep`)
- 빈 hierarchy → "-" 표시

**MCP Playwright 단위 테스트 17 시나리오** (모두 통과):
- 7탭 전환 / 헤더 토글 / 검색 입력 / Pipeline 4 서브탭 / Settings 4 버튼 / Processing 카드+필터+로그 / Todos / Topics / Verification / Modal open+close / 16 data-action / Enter 키 → doSearch / hierarchy breadcrumb 렌더

**lesson 갱신**:
- lesson 19: 8단계 → **10단계** 체크리스트 (UI 기능 제거 시 frontend 정합성 검증 추가)
- frontend grep 패턴 명시: `grep -oE "call\(['\"]([a-z_]+)" ui/dashboard.js` ↔ 백엔드 `#[tauri::command]` 대조

## Phase 63: fastembed Sparse 어댑터 (2026-04-29)

`FastEmbedSparseAdapter` + `SparseVector{indices, values}` + `dot()` 유사도 계산. BGE-M3 sparse(lexical) 출력 활용.

- 위치: `crates/adapters/src/driven/embedding/fastembed_sparse.rs`
- `fastembed` feature 격리 (Phase 62와 동일 빌드 요구사항)
- 단위 테스트 3건 통과

LocalVectorStore 통합은 트리거 대기 #10 — 현재 `keyword_index`(HashMap<String, Vec<doc_id>>)와 비호환이라 별도 sparse_index 신규 작업 시점은 실 코퍼스 측정 후.

## Phase 61: 청킹 메타데이터 고도화 (2026-04-29)

원문 스마트 청킹 자료의 ① 계층적 청킹 + ⑦ 인덱싱 메타데이터 표준화 통합 적용. 비용 낮음 (~0.5일 실측), 외부 의존 0.

**변경 사항**:
- `SemanticChunk.title_path: Vec<String>` — H1>H2>H3 경로 추적
- `Metadata.hierarchy: Vec<String>` + `content_type: String` — lesson 5 적용 (Default + serde default)
- `SimilarDoc.hierarchy: Vec<String>` — 검색 결과에서 즉시 노출
- `LocalVectorStore::upsert`가 hierarchy를 keyword_index에 합침 → 제목 매칭 검색
- MCP/Tauri search 응답 JSON에 hierarchy 포함
- dashboard.js 검색 결과에 breadcrumb 표시 (`A › B › C`), CSS 토큰 기반

**호환성**:
- 기존 인덱스(hierarchy 없음)는 `serde(default)` + `Default` 구현으로 자동 호환
- 재인덱싱 시 hierarchy 자동 부착

## Phase 62: fastembed 통합 (2026-04-29)

전문가 자문 결과 채택. fastembed v5.13 (`pykeio/ort` 기반, 순수 Rust)으로 BGE-M3 Dense + BGE-Reranker-v2-M3 Cross-Encoder 도입.

**검증 실측** (BGE-M3, Windows 11):
- 모델 로드: 66초 (앱 시작 시 1회)
- 단건 임베딩: 64ms/건
- 배치 100건: 6.1초 (61ms/건)
- RSS 메모리: 1.5~1.7GB

**개선 효과**:
- MRR@5: 0.65 (Claude CLI) → 0.975 (fastembed) = +50%
- 임베딩 속도: 15초 → 64ms/건 (234x)
- 1K문서 초기 투입: 4.2시간 → ~1분

**빌드 요구사항**:
- VS 2022 Build Tools v17.8+ (MSVC v14.38+) — ort-sys 정적 라이브러리 호환
- Windows SDK 10.0.19041.0+
- `cargo build --features fastembed` (또는 modals/app `--features fastembed`)

**구현**:
- `crates/adapters/src/driven/embedding/fastembed_adapter.rs` — `FastEmbedAdapter`
- `crates/adapters/src/driven/reranking/fastembed_reranker.rs` — `FastEmbedReranker`
- `feature = "fastembed"` 플래그로 격리 (기본 빌드는 영향 없음)
- `Arc<Mutex<TextEmbedding>>` + `tokio::task::spawn_blocking` (fastembed가 동기 + `&mut self`)
- `EmbeddingConfig.default_model = "fastembed"` (기본값)
- `RerankConfig.provider = "fastembed"` (기본값)
- Fallback 체인: fastembed → Claude CLI → HashEmbedder
