# Lesson 36 — C1/C2 사용자 정의 GUI + A1 LRU GC + clippy 잔존 정리

## 상황 (2026-05-15)

남은 항목 4건 일괄 처리:
1. C2 PII 패턴 add/remove UI (Settings 탭)
2. C1 임계값 GUI 조정 폼 (Settings 탭)
3. A1 캐시 크기 제한 + LRU GC (config + DB API + 주기 task 통합)
4. clippy 잔존 warning 18 → 8 (수동 5건)

## 문제 / 결정 사항

### A1 LRU GC — 매 store vs 주기 task

옵션:
1. CachedLLM.store 시 매번 count → 비용 매 호출
2. store 후 N% 확률로 GC → 무작위성
3. 주기 task (4시간 등)에서 1회 GC → 결정적, 비용 0

(3) 채택. C1 자동 추천이 이미 4시간 주기 task로 동작 — 동일 task 안에서 GC도 함께 실행 (`a1-gc` 로그). 사용자 명시 호출 불필요.

`gc_llm_cache_to(max)` 시그니처가 max=0이면 no-op. 디폴트 `llm_cache_max_entries=10000` (LLM 가공 평균 10KB × 10000 = ~100MB JSON 저장 상한).

### GUI 노출 위치 — 새 탭 vs Settings 탭 vs 인스펙터

Settings 탭 상단에 카드 그룹으로 통합:
1. Decision Log (C1 흐름) — 기존
2. C1 자동 추천 임계값 — 신규
3. PII 검출 패턴 — 신규

새 탭은 사용 빈도 낮은 기능을 격리해야 할 때 적합. C1/C2 임계값은 설정과 본질적으로 같은 카테고리.

### LRU 정렬 키 — last_hit_at 만 vs 다중 기준

`last_hit_at NULL DESC, last_hit_at ASC, hits ASC`:
- NULL DESC: 한 번도 hit 안 된 항목 최우선 삭제 (가치 입증 안 됨)
- last_hit_at ASC: 가장 오래된 hit 다음
- hits ASC: 동률 시 hits 적은 것 먼저

단순 last_hit_at만으로는 모두 NULL일 때 random 삭제. 다중 기준이 가치 있는 항목 보존에 유리.

## 원인

1. **A1 무제한 누적 위험**: 캐시 hit 통계만 노출하고 정리 메커니즘 부재. lesson 14 (미연결 포트) 의 변형 — "API만 있고 운영 자동화 없음".
2. **C1/C2 임계값 코드 고정 → DB 가변 → GUI 부재**: 3단계 중 마지막 누락. API 정의만 노출 vs Dashboard 통합 차이 (lesson 32 재확인).
3. **clippy 안전 패턴 잔존**: `doc list item / wildcard / clamp / &PathBuf / sort_by_key` 다섯 가지는 의도성 0 — 즉시 정리 가능. too_many_arguments는 도메인 설계 트레이드오프.

## 개선

### 운영 데이터 자동 정리 패턴

```
1. config 추가: max_entries (0=무제한, 기본값)
2. SettingsDb::gc_*_to(max) 메서드 (단순 SQL: ORDER BY + LIMIT + DELETE)
3. 주기 task에서 1회/주기 호출 (기존 task에 통합 권장)
4. 단위 테스트로 LRU 정렬 검증
```

### Settings 탭 동적 카드 패턴

```js
// switchTab('settings') 진입 시
this.loadDecisionLog();      // C1 흐름
this.loadC1Thresholds();     // C1 룰
this.loadPiiPatterns();      // C2 룰
this.loadSettings();         // 일반 config (기존)
```

4개 카드가 독립 fetch — 한 카드 실패가 다른 카드 차단 안 함. catch는 카드 host에 에러 메시지 표시.

### clippy 잔존 카테고리

| 카테고리 | 처리 |
|---------|------|
| doc list item without indentation | 빈 줄 1개 + 들여쓰기 4칸 |
| wildcard_in_or_patterns (`"x" | _`) | `_` 패턴만 유지 + 주석으로 alias 명시 |
| clamp 패턴 (`.min(1.0).max(0.0)`) | `.clamp(0.0, 1.0)` |
| &PathBuf → &Path | std::path::Path 직접 사용 |
| sort_by 정렬 키 | `sort_by_key(\|e\| std::cmp::Reverse(...))` |
| too_many_arguments (7+) | ✅ **Phase 85에서 종결** — `CrossRefUpdateContext` / `DecisionDraft` / `NewTodo` 입력 구조체로 4건 모두 해소 (lesson 38) |
| very_complex_type | ✅ **Phase 84에서 종결** — `ProgressCallback` type alias 도입 |
| loop_var_index | ✅ **Phase 84에서 종결** — `iter_mut().skip().take()` / `iter_mut().enumerate()` 리팩터링 |

## 결과

- workspace lib 331 → **332** (shared 92 → 93: gc_llm_cache 1건 신규)
- Tauri commands 72 유지, MCP tools 32 유지
- configField: llm.llm_cache_enabled / llm.llm_cache_max_entries 신규
- Settings 탭 카드: 1 → **4** (Decision Log + C1 임계값 + PII 패턴 + 기존 config 폼)
- clippy warning 18 → **8** (-56%)
- Tauri GUI check + workspace check 통과, 경고 0

## 후속

- clippy 잔존 8건 — too_many_arguments는 빌더 패턴 도입 검토 (lesson 21/27 후속)
- C1/C2 GUI에 "재시작 후 반영" 안내 → live reload 검토
- llm_cache_max_entries GUI에서 즉시 GC 트리거 버튼
- LRU GC 결과 (삭제 건수) 를 stat 카드로 노출 검토
