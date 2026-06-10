# Lesson 32 — A1 가시화 + C1 자동 추천 + 통합 테스트 위생

## 상황 (2026-05-15)

권장안 4건 일괄 처리:
1. A1 LLM 캐시 hit률을 Dashboard 카드 + MCP tool로 노출
2. 5K 합성 코퍼스 생성 PowerShell 스크립트 (트리거 #2/#4/A2/B1 측정 의존성 해소)
3. C1 자기학습 1단계 — Phase 80 카운터 → decision_log 자동 추천
4. 통합 테스트 unused import/variable 경고 28건 정리

## 문제 / 결정 사항

### A1 가시화 — Dashboard vs MCP 양쪽 vs 한쪽

Phase 80 카운터 API들은 frontend에서 호출처가 없는 상태. 일관성을 위해 A1도 API만 두려 했으나 사용자가 "Dashboard 카드 + MCP tool 양쪽"을 명시. 헤더 stat-card 그룹에 새 그룹 "LLM 캐시" 추가.

### C1 — 제안만 vs 자동 적용

자동 추천을 자동 적용까지 확장하면 사용자 의도와 무관한 config 변경 위험. lesson 30 패턴 (인프라 + 디폴트 비활성) 그대로 — `decision_log`에 source="auto_suggestion" INSERT만, 적용은 사용자가 `setup_decision_log_list` → 수동 결정.

임계값 보수적:
- 검색 mode: 100회 + dominant 60% 초과
- CRAG: 50회 + incorrect 25% 초과

너무 빠르게 (예: 10회 누적) 제안하면 노이즈 우세 — 통계적 안정성 확보 후 제안.

### 통합 테스트 위생 — cargo fix 활용

`cargo fix --tests --workspace --allow-dirty`로 27건 자동 수정. real_env_tests.rs 1건만 수동 (`VectorDBPort` import). 회귀 0건.

## 원인

1. **API 정의만 vs 카드 통합 차이**: API를 정의해도 frontend에서 호출하지 않으면 사용자 가시성 0. Phase 80 카운터 카드가 빠진 것과 동일 누락 패턴. → A1은 즉시 카드로 노출.
2. **자동 추천 임계값 설계**: 측정 표본이 작을 때 제안하면 신호 < 노이즈. 통계적 검정력 확보 (≥50~100건)이 필요.
3. **cargo fix 누락 사례**: dirty workspace에서는 `--allow-dirty` 필요. real_env_tests처럼 단순 import만 있는 케이스는 자동 수정 안 되는 경우 존재 (rust-analyzer 보수적 판단). → fix 후 잔존 경고 grep 의무.

## 개선

### A1 가시화 패턴

```
index.html: header stat-card 그룹 ("LLM 캐시")
dashboard.js: renderStats() 갱신 + init/refreshDashboard에서 API.getLlmCacheStats()
commands.rs: #[tauri::command] get_llm_cache_stats → SettingsDb.llm_cache_stats()
mcp_server.rs: make_tool + dispatch + handle_get_llm_cache_stats
```

4곳 동기화 필요 (lesson 19 frontend-backend 매핑 정합성 패턴).

### C1 자동 추천 설계 원칙

1. **임계값 통계 안정성**: 100회 미만은 노이즈 우세, 100회 이상 + dominant 비율 60% 권장
2. **decision="suggested"**로 명시: 사용자 수동 결정 (accepted/rejected) 와 구분
3. **source="auto_suggestion"**: 다른 추천 (manual/setup_review) 와 필터 가능
4. **config 변경 절대 금지**: 본 모듈은 제안만. 적용은 별도 명시적 호출

### 통합 테스트 위생 절차

```bash
cargo fix --tests --workspace --allow-dirty --allow-staged
cargo build --tests --workspace 2>&1 | grep -E "warning:"  # 잔존 확인
# 잔존이 있으면 grep 패턴으로 수동 처리
```

## 결과

- workspace lib 318 → 321 (shared 85 → 88: auto_suggester 3건 신규)
- 통합 테스트 경고 28 → 0건
- Tauri commands 62 → 64 (get_llm_cache_stats + auto_suggest_from_counters)
- MCP tools 22 → 24 (get_llm_cache_stats + auto_suggest_from_counters)
- 5K 합성 코퍼스 스크립트 — `spec/benchmarks/scripts/gen_synthetic_corpus.ps1`
- Dashboard 헤더에 LLM 캐시 카드 그룹 추가
