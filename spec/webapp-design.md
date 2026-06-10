---
updated: 2026-06-05 (헤더만 갱신 — 본질 재정의 2차(2026-06-04, tasty 패턴 흡수) 미반영. 본문 6탭 ↔ 본질 1도메인 host 불일치 점검은 Phase 208 GUI Plugins 탭 진입 시 본격 처리 예정. 메타 룰 30 자기 적용 7건째)
purpose: file-pipeline Tauri GUI 현행 IA·UX 명세. Claude 세션 간 GUI 구조 복원용
predecessor: webapp-design-phase56.md (없음 — Phase 56 자문 컨텍스트는 Phase 98 재작성으로 폐기)
status_note: 본문 § "탭 구성 (7개)" ~ § "Settings 5그룹"은 Phase 107(6탭 통합) + 본질 재정의 2차(2026-06-04) 미반영. 단일 진실원: prd/research/plugin-architecture-2026-06-04.md / spec/architecture.md §본질 재정의 2차
---

# File Pipeline 웹앱 — 현행 IA·UX 명세 (Phase 106)

## 프로젝트 개요

로컬 파일을 자동 분류·가공·압축·색인하여 Claude Code(MCP)로 검색 가능하게 만드는 Windows 데스크톱 파이프라인.

- **단일 사용자 데스크톱** 도메인 (lesson 50 RBAC 보류 / 메타 룰 20 자기 적용)
- Tauri 2.0 WebView GUI, 단일 바이너리 **22.09 MB** (Phase 106 release)
- Rust 백엔드 + 단일 JS 프론트엔드 (`ui/dashboard.js`)
- MCP 서버 **36 도구** (실측, 2026-06-01 정밀 분류 — 검색 8 / 추천 13 / 카운터 4 / C1 임계값 2 / C2 PII 3 / A1 캐시 2 / Todo 2 / optimize 1 / lint 1) + Tauri commands 65개 + 11 도메인 포트. **Phase 108~115 분리 시 외부 8 / 잔류 28 분할** (`prd/research/search-extraction-plan.md` §1.3)
- **UI 라벨 정책** (Phase 101 메타 룰 28 정식): 내부 코드명(C1/A1/G1 등) + Phase 번호 노출 금지. JS 함수명·HTML id는 추적성 보존
- **온보딩** (Phase 106): 헤더 🧭 온보드 버튼 → 4-step 모달 무상태 흐름. 별도 DB 없음 — 사용자 명시 클릭 트리거

## 탭 구성 (7개)

```
┌───────────────────────────────────────────────────────────┐
│ Pipeline Manager [● 감지 ON]     [🧭 온보드] [🤖 AI 설정] │
├───────────────────────────────────────────────────────────┤
│ 헤더 카드 15개:                                            │
│   문서(4) | KG(3) | 토큰사용(3) | LLM 캐시(5)              │
├───────────────────────────────────────────────────────────┤
│ [Documents] [Processing] [Todos] [Verification]           │
│ [Topics] [Pipeline] [Settings]                            │
└───────────────────────────────────────────────────────────┘
```

## 헤더 우측 버튼 (2종, Phase 106)

| 버튼 | data-action | 흐름 |
|------|-----------|------|
| 🧭 **온보드** (Phase 106 신규) | `open-onboarding` | 4-step 모달 (환영/크레덴셜+기본 설정/inbox/optimize). 무상태 (DB 미사용) — 사용자 클릭 시마다 1부터 |
| 🤖 **AI 설정 도우미** (Phase 75) | `open-setup-assistant` | Claude Code MCP 안내 모달 |

### 온보딩 4-step 흐름

```
[🧭 클릭]
  ↓
1/4 환영 — 4단계 안내 (다음 →)
  ↓
2/4 크레덴셜 등록 — "크레덴셜 등록 폼 열기"(showCredentialForm 재사용) | "다음(건너뛰기)"
  ↓ 저장 성공 시
[기본 크레덴셜 설정 모달] — 3 분기 (신규/기존 다른 덮어쓰기/이미 본인)
  ↓ "기본으로 설정 + 다음 →"
3/4 inbox에 파일 넣기 — 감지 ON / Processing / Documents 안내 (다음 →)
  ↓
4/4 100개 도달 후 설정 최적화 — Settings 자동 추천 카드 안내 (완료)
```

취소·ESC·X = 온보딩 중단. 다시 🧭 클릭 시 1부터.

| 탭 | 주요 기능 | 데이터 소스 | 갱신 방식 |
|----|----------|-----------|----------|
| **Documents** | 문서 목록 + KG 시각화 + 검색 + 상세 보기 + Metadata 보조 필드(needs_verification / open_questions) | vector_db | 5초 자동 + 탭 진입 |
| **Processing** | 큐 현황(4카드) + 교차참조 + 작업 테이블 + 실패 재처리 | work_queue.json | 5초 자동 |
| **Todos** | Pending/Completed 카드 + 할일 목록 + CRUD | settings.db | 탭 진입 + 새로고침 |
| **Verification** | pass/fail/warning 카드 + 메트릭 테이블 + **강한 주장** + **자동 이상 감지** (Phase 93 H1) | 메모리 + audit_anomaly | 탭 진입 |
| **Topics** | 토픽 카드 + 검색/정렬 + 모달 편집 | 파일시스템 .md | 탭 진입 + 새로고침 |
| **Pipeline** | 2컬럼: 사이드바(시뮬레이션+로그) + 메인(4서브탭 + 인스펙터) | config + decision_log | 탭 진입 |
| **Settings** | 5그룹 네비 + 카드 7+종 (MCP 카탈로그 / PII / Decision Log / 자기학습 / 운영 카드 등) | config + settings.db | 탭 진입 |

## Pipeline 탭 구조 (Phase 66~67 인스펙터 정착)

```
┌────────────────────┬─────────────────────────────────────┐
│  좌 사이드바(320px) │  우 메인 (가운데 캔버스 + 우 인스펙터)│
│                    │                                     │
│ 시뮬레이션         │  [데이터 가공] [외부 저장소]           │
│  텍스트 → 실행     │  [청킹] [보존 & Purge]                │
│                    │                                     │
│ 전처리 테스트      │  ─── 서브탭 콘텐츠 ───                │
│  파일 → 테스트     │  Preprocess / LLM / Verify /         │
│                    │  Embedding / Storage 노드 카드        │
│ ── 결과 ──         │  또는 외부저장소(remote_upload) /     │
│  노드별 pass/fail  │  청킹+교차참조 / 보존 정책 +          │
│                    │  Purge Dry Run/Execute                │
│ ── 로그 ──         │                                     │
│  시뮬레이션 출력   │  ─── 우 인스펙터 (480px) ───          │
│                    │  선택 노드의 configFields 편집        │
│                    │  + Notion mode 경고 (Phase 93)        │
└────────────────────┴─────────────────────────────────────┘
```

### Pipeline 노드 수치 (Phase 90 GUI 검증 실측)

- **가공 노드**: 23개 (사전검사 + 스텝 + 후처리 + Quarantine 분기 + Memory Tier + Lint)
- **검색 노드**: 20개 (Dense/Hybrid/Filtered trace + 시뮬레이션 단계 추가)

## Settings 5그룹 (Phase 100 사용자 피드백 재배치 — settings-nav 4→5)

```
크레덴셜 관리    — 카드 UI (프로바이더별 아이콘 + CRUD + 기본 설정)
일반             — 로깅
운영             — 자동 추천 · 임계값 · PII 패턴 · 출력 PII 마스킹 · MCP 도구 분류 (5 카드)
                   ※ Phase 100 신규 그룹. settings-ops-cards 단일 진실원 컨테이너 + DOM mount 패턴 (메타 룰 19)
이벤트 훅        — HookDefinition CRUD (5 이벤트: file_detected / process_start / process_complete / verify_fail / search_query)
마이그레이션     — 임베딩 재생성 / 벡터DB 재구축 / 전체 재가공
```

### Settings 운영 그룹 5 카드 (Phase 84~93 누적, Phase 100 좌측 그룹 이관)

> **UI 라벨 정책** (Phase 101 메타 룰 28 정식): 카드 제목에서 내부 코드명 (C1)/(C2)/(A1)/(H1)/(Phase 92 H1) 제거.

| 카드 (현행 라벨) | 내부 식별 | Phase | 역할 |
|---------------|----------|-------|------|
| 🧠 자동 추천 | Ruflo C1 | 82 / 33 / 100 | 카운터 기반 자동 추천 + 4시간 주기 + Decision Log 필터/정렬 |
| ⚙️ 자동 추천 임계값 | C1 thresholds | 34 | 발화 조건 7 임계값 사용자 가변 (settings.db.c1_rule_thresholds) |
| 🔒 PII 검출 패턴 | Ruflo C2 | 34 / 84 | regex 추가/제거 + live reload (재시작 불필요) |
| 🛡 출력 PII 마스킹 | A2 | 91 / 93 | 검색 결과·MCP 응답 PII 토글 (디폴트 ON) |
| 🧰 MCP 도구 분류 | Mirage H3 | 92 / 93 | 25 도구 카테고리/mutates/cost (Phase 102 optimize 포함) |

## 헤더 카드 그룹 (15개)

```
┌───────────────────────────────────────────────────────────┐
│ 문서 그룹 (4)    | KG 그룹 (3)     | 토큰 사용 (3) | A1 캐시 (5) │
│ ─ 전체 N개      | ─ 관계 N개      | ─ classify   | ─ hit N    │
│ ─ 가공된 N개    | ─ 평균 degree   | ─ process    | ─ miss N   │
│ ─ 큐 N개        | ─ 고립 N개      | ─ verify     | ─ hit rate │
│ ─ KG 노드 N개   |                 |              | ─ saved    │
│                 |                 |              | ─ LRU evict│
└───────────────────────────────────────────────────────────┘
```

## 공통 UI 패턴

| 패턴 | 구현 |
|------|------|
| **모달** | `Modal.open(title, bodyHtml, { onSave, wide })` — Credentials/DocType/Topics/Todos/Prompts |
| **카드** | `cred-card-grid` (크레덴셜), `stat-card` (헤더), `doc-card` (문서) |
| **테이블** | `array-table` (경로 추가/삭제), `doc-table` (문서/처리) |
| **검색** | 설정 검색(하이라이트), 문서 검색(필터), 토픽 검색 |
| **자동 저장** | Pipeline 노드 설정 변경 시 1초 debounce `_pbAutoSave` |
| **자동 갱신** | `refreshDashboard()` 5초 간격 (Promise.all 병렬 4 API) |
| **접기/펼치기** | `.section-toggle` + `.collapsed` + `.section-body` |
| **라이브 리로드** | PII 패턴 + C1 임계값 (Phase 84) — 재시작 불필요 |

## 데이터 아키텍처

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│ JS (UI)     │     │ Tauri Cmd   │     │ Rust 서비스  │
│             │     │  65개       │     │             │
│ API.xxx()   │────>│ invoke()    │────>│ vector_db   │
│ + 5초 폴링  │     │             │     │ settings_db │
│ vm.state    │<────│ JSON 응답   │<────│ config      │
│             │     │             │     │ audit_trace │
└─────────────┘     └─────────────┘     └─────────────┘

       │                                       │
       └────────────── HTTP fetch ────────────►│  (mcp_server, 24 도구)
                                               │  Claude Code MCP 클라이언트
```

## 메타 룰 13 4단계 UI 노출 영역 (사용자 가시화 완성)

| 인프라 | 단계 | UI 노출 위치 |
|--------|------|------------|
| A1 LLM 캐시 (Ruflo) | 4단계 ✅ | 헤더 5카드 + Settings 카드 |
| C1 자기학습 | 4단계 ✅ | Settings Decision Log + 임계값 |
| C2 PII | 4단계 ✅ | Settings 사용자 패턴 카드 |
| MCP 카탈로그 (H3) | 4단계 ✅ | Settings MCP 도구 분류 |
| A2 PII mask | 4단계 ✅ | Settings 출력 PII 마스킹 토글 |
| H1 audit_anomaly | 4단계 ✅ | Verification 자동 이상 감지 카드 |
| H5 Notion capability | 4단계 ✅ | Pipeline remote_upload 인스펙터 |
| Metadata 보조 필드 | 4단계 ✅ | Verification 강한 주장 카드 |

## 제거/비활성화된 기능

> 단일 진실원: `spec/deprecated.md` (메타 룰 19 자기 적용). 본 표는 GUI 영향 요약만.

| 기능 | GUI 상태 | 사유 |
|------|---------|------|
| DashboardConfig (port/auth) | 제거 | Tauri WebView, 포트 불필요 |
| GraphDB UI | 비활성 | 실사용 가치 미검증, 코드 보존 |
| Notification UI | Settings 제거 | NullNotification 기본 |
| retention/purge UI | Settings 제거 | 기능 자체 제거 |
| Credentials 독립 탭 | 제거 | Settings > 크레덴셜 관리 통합 |
| pb-subtabs | 제거 (G-4) | lesson 47 dead-code |
| Tauri commands 10건 (G-7) | 제거 | frontend 호출처 0건 |

## 회귀 게이트 (Phase 97 시점 7종)

GUI 변경 phase 종결 시 자동 검증:

```bash
bash spec/benchmarks/scripts/dead_selector_scan.sh        # ID 매칭 (lesson 47)
bash spec/benchmarks/scripts/dead_selector_scan_v3.js     # ID + CSS rule (Phase 98 신규)
bash spec/benchmarks/scripts/gui_http_smoke.sh             # 5/5 통과
bash spec/benchmarks/scripts/action_catalog.sh --count     # baseline=68
bash spec/benchmarks/scripts/audit_stage_check.sh          # 메타 룰 24
bash spec/benchmarks/scripts/release_rebuild_required.sh   # 메타 룰 17
bash spec/benchmarks/scripts/empty_state_audit.sh
```

## 현행 IA 안정성 평가 (Phase 65~67 IA 재설계 → 정착)

- Phase 65: 3계층 IA 시도 → Phase 66 7탭 원복 (lesson 21 IA 부분 원복 5패턴)
- Phase 67: 인스펙터 480px 편집 정착 (lesson 22 인스펙터 기반 5패턴)
- Phase 80+: IA 구조 변동 없음, 카드·인스펙터 확장만 누적

→ **현행 7탭 + Pipeline 2컬럼 + Settings 5그룹은 안정 단계 도달**. 다음 IA 변동은 사용자 명시 요구 시점에 검토.

## 변경 이력

- Phase 65~95: spec/architecture-archive.md / architecture.md 단일 진실원 위임
- Phase 56 자문 컨텍스트: 본 재작성으로 폐기 (메타 룰 19 자기 적용)
- Phase 98 (2026-05-26 C-8): 전면 재작성, 변동 이력 섹션 제거, Phase 97 현행 IA 기준
