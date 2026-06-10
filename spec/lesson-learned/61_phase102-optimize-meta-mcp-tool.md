# Lesson Learned: Phase 102 비전문가용 통합 메타 MCP 도구 `optimize`

## 상황

2026-05-26 Phase 102. 사용자 시나리오 5단계 검증 후 자동화 갭 3건 식별 — "설정 최적화 해줘" 단일 자연어 요청으로 통합 동작이 안 됨. 사용자 명시 트리거: "Q1 지금 구현하자. 사용자가 전문가 아닐 가능성이 크므로 자동 최적화 기능이 꼭 필요해".

기존 MCP 도구 5종(setup_review / setup_apply / setup_dryrun / setup_modules_list / setup_apply_modules / setup_decision_log_list / auto_suggest_from_counters)을 비전문가 사용자가 적절히 조합·호출하기 어려움. 누적 카운터가 임계 미달 시 setup_decision_log_list가 0건 반환 → 사용자 혼란.

## 문제

### 문제 1 — 비전문가 진입점 부재

기존 흐름:
1. `setup_decision_log_list` 호출 → 카운터 누적 부족 시 0건 (왜 0건인지 모름)
2. `auto_suggest_from_counters` 호출 → 임계 미달 시 신규 추천 0건
3. `setup_review` 호출 → scenario 텍스트 인자 필요 (사용자가 시나리오 작성 필요)
4. 도구 5종 중 어느 것을 호출해야 할지 불명

비전문가 사용자가 "설정 최적화 해줘" 자연어 요청 시 Claude Code가 어느 도구를 호출할지 추론 필요 — 자동 호출 보장 안 됨.

### 문제 2 — 누적 진행률 정보 부재

기존 도구는 "현재 누적 X / 임계 Y" 정보 미반환. 사용자가 임계값 도달 여부 모름. 임계값 디폴트(100 / 50 / 30) 자체도 노출 안 됨.

### 문제 3 — 사용자 친화 next_actions 부재

기존 응답은 raw 데이터 (entries 배열 등). 다음 무엇을 해야 할지 명시 안 됨.

### 문제 4 — 추정 빗나감 9번째 누적 (메타 룰 26 자기 적용 실패)

본 phase 구현 중 `scenario_advice.get("changes").and_then(|v| v.as_array())` 패턴 사용 — `build_advice` 반환 타입이 `SetupAdvice` 구조체(Vec<ConfigChange> 직접 필드)인데 JSON serde_json::Value로 추정. E0282 빌드 에러 1건.

메타 룰 26 정식 승격(Phase 99) 후에도 사전 grep 미실시 — **메타 룰 25(자기 적용 의무)와 메타 룰 8(grep 먼저) 동시 위반**.

## 원인

### 직접 원인
1. (문제 1) 기존 도구는 단일 기능 단위. "비전문가 통합 진입점" 메타 도구 부재
2. (문제 2) 임계값은 코드 디폴트로만 정의 — 응답에 포함 안 됨
3. (문제 3) MCP 도구 응답이 머신 친화 (JSON entries) 중심. 자연어 안내 부재
4. (문제 4) 코드 작성 시점에 setup_review.rs 시그니처 grep 미실시. SetupAdvice 구조체 직접 접근 가능했음

### 구조적 원인
- 기존 5종 setup_* 도구는 각 phase에서 단일 기능으로 추가됨 (Phase 73 review / 76 apply / 77 snapshot / 78 dryrun / 80 modules / 82 decision_log)
- 비전문가 사용자 시나리오 검증 부재 — 직전 사용자 검증(Phase 102 진입 직전)이 자동화 갭 발견의 첫 사례
- 메타 룰 26 정식 승격 후 자기 적용 체크리스트 phase 진입 시점 미실행 (메타 룰 25 §체크리스트 자기 위반)

## 개선

### 즉시 적용 (본 Phase 102 완료)

#### `mcp__file-pipeline__optimize` 신규 메타 MCP 도구

- ✅ `crates/shared/src/mcp_server.rs::handle_optimize` 신규 (140줄)
- ✅ 호출 1회로 5단계 통합:
  1. C1 누적 카운터 분석 (suggest_from_counters 호출, run_analysis=true 디폴트)
  2. 누적 진행률 측정 + 임계값 비교 (`progress.search_count / crag_count / processed_count`)
  3. 검토 대기 추천 목록 (decision_log decision="suggested" 필터)
  4. 시나리오 권고 (선택 인자, setup_review 결과 포함)
  5. 사용자 친화 `next_actions` 작성 (조건별 분기 5종)

- ✅ MCP 도구 카탈로그 등재 (`McpToolMetadata { name: "optimize", mutates: false, category: Settings, cost: Free }`)
- ✅ make_tool spec 작성 + 라우팅 분기 추가
- ✅ MCP 도구 24 → 25개
- ✅ **제안만 반환** — 자동 적용 0건 (lesson 30 Ruflo 패턴 완전 준수)
- ✅ 사용자 응답 끝 `note` 필드: "본 도구는 제안만 반환합니다. 자동 적용은 없으며 사용자가 setup_apply / accept_suggested_decision으로 명시 적용해야 합니다."

#### 사용자 흐름 (Phase 102 후)

```
Claude Code (사용자 자연어): "설정 최적화 해줘"
   ↓ Claude Code가 mcp__file-pipeline__optimize 호출 추론
   ↓
{
  "progress": {
    "search_count": { "current": 12, "threshold": 100, "ready": false },
    "crag_count": { "current": 0, "threshold": 50, "ready": false },
    "processed_count": { "current": 8, "threshold": 30, "ready": false }
  },
  "newly_inserted": 0,
  "pending_suggestions": { "count": 0, "entries": [] },
  "next_actions": [
    "아직 데이터가 부족합니다. inbox에 파일 더 투입 또는 검색 더 사용하세요. (현재: 검색 12/100 · CRAG 0/50 · 가공 8/30)",
    "scenario 인자로 사용자 시나리오 직접 입력 시 즉시 권고 가능 (예: \"회의록 위주 가공 중\")",
    "Settings → 운영 → 자동 추천 임계값 카드에서 임계값 낮춤 가능"
  ],
  "note": "본 도구는 제안만 반환합니다..."
}
```

→ 비전문가 사용자가 즉시 다음 행동 파악 가능.

### 추정 빗나감 9번째 사례 + 즉시 해소

- (메타 룰 26 자기 적용 실패) `scenario_advice.get("changes")` → 실제 `scenario_advice.changes` 구조체 필드 접근으로 수정
- 메타 룰 18 누적 8 → **9** (다음 phase 검토 시 패턴 강화 후보)

### 메타 룰 강화 (다음 phase 자기 적용)

- [ ] **메타 룰 26 강화**: match 케이스 외 **외부 함수 반환 타입 추정**도 grep 의무 (Phase 102 사례 — `build_advice` 반환이 JSON 추정)
- [ ] **메타 룰 25 자기 적용 체크리스트 phase 진입 시점 실행 의무 강화** — 메타 룰 26 정식 후 본 phase 시작 시 자기 적용 누락
- [ ] **메타 룰 28(UI 코드명 노출 금지) 정식 승격** — 직전 Phase 101에서 후보 등재, Phase 102 후속 처리 대상

### Pipeline 이관 검토 결과 (Phase 101 후속)

직전 phase 보고 옵션 A(이관 안 함) / B(PII 마스킹만) / C(확대) 중 사용자 결정 대기. 본 phase는 별도 작업으로 보류.

## 다음 세션 플래그

- 사용자가 GUI 재실행 + Claude Code MCP 등록 후 `optimize` 도구 호출 실측
- 누적 진행률 응답이 비전문가에게 충분히 명확한지 사용자 검증
- 메타 룰 28 정식 승격 (Phase 101 잔여)
- Pipeline 이관 옵션 사용자 결정 후 구현
- 메타 룰 26 강화 ("외부 함수 반환 타입 추정도 grep 의무")

## 회귀 기준선

| 지표 | Phase 101 | Phase 102 | 차이 |
|------|---------|---------|------|
| MCP 도구 | 24 | **25** | +1 (optimize) |
| handle_optimize 코드 | — | **+140 줄** | 신규 |
| workspace cargo check | 통과 | 통과 (41.85s) | — |
| workspace lib 테스트 | 383 | **383** | 회귀 0 |
| audit_stage_check | PASS (10 정적 + 1 동적) | PASS (동일, optimize는 audit.record 미사용) | 0 |
| pipeline.exe (CLI release) | 17.92 MB | **17.99 MB** | +60 KB |
| file-pipeline-tauri.exe (GUI release) | 21.05 MB | **21.08 MB** | +24 KB |
| Tauri release 빌드 시간 | 11m 53s (incremental) | 15m 00s (incremental, optimize 코드) | +27% |
| D:\file-test\pipeline.exe | Phase 101 | **Phase 102** (재배포) | 갱신 |
| 추정 빗나감 누적 | 8 (Phase 97) | **9** (Phase 102 SetupAdvice 구조체 추정) | +1 |

## 사이드 발견

- `SetupAdvice` 구조체에 `Serialize` 파생 — serde_json 응답에 직접 직렬화 가능. JSON 변환 불필요
- 메타 룰 26 정식 승격(Phase 99) 후 즉시 자기 적용 체크 누락 — Phase 102 진입 시점에 메타 룰 25 §체크리스트 grep 의무 미실시
- optimize 도구는 audit.record 미사용 — read-only 도구라 audit 가치 낮음. 단 향후 사용자 호출 빈도 측정 가치 있으면 추가 검토
- Claude Code MCP 호출 시 도구 spec description이 자연어 trigger로 작동 — "비전문가용 통합" 키워드 포함으로 "설정 최적화" 자연어 자동 매칭 가능성 ↑
