# Lesson 33 — C1 2단계 + A1 무효화 + startup 트리거 + A2/B1 inspector

## 상황 (2026-05-15)

즉시 가능 4건 일괄 처리:
1. C1 2단계 — suggested entry confirm → toml 자동 적용
2. A1 캐시 무효화 UI (모델/프롬프트 변경 후 사용)
3. 자동 추천 정기 트리거 (startup 1회)
4. A2/B1 활성화 토글을 inspector configFields로 노출

## 문제 / 결정 사항

### C1 2단계 — 별도 함수 vs setup_review 재사용

`setup_review::apply_advice_full`은 SetupAdvice 객체를 요구 (profile + scenario + summary). C1 entry 하나만 적용하려면 overhead 큼.

대안:
1. SetupAdvice 가짜 구성 후 호출
2. `write_toml_path`만 노출 + `auto_suggester::apply_suggested` 신규

(2) 채택. write_toml_path를 pub 으로 바꾸고 auto_suggester에서 직접 호출. 함수 표면적 최소.

### startup 트리거 — 동기 vs 비동기 vs 스레드

Tauri `.setup()`은 동기 클로저. blocking suggest_from_counters를 동기로 호출하면 UI 시작 지연 가능. `std::thread::spawn`으로 별도 스레드 — 100ms 미만이지만 안전.

### A2/B1 inspector 노출 — Phase 73 configFields 패턴

기존 configFields가 vector_db 그룹에 search_top_k를 두는 등 그룹 경계가 불분명. 신규 ("search", ...) 그룹 분리. expand_kg_hops/diversity_threshold 기본 0 유지 (lesson 30 패턴).

## 원인

1. **C1 2단계 함수 표면적**: setup_review가 SetupAdvice 모델에 강결합 — 단일 entry 적용에는 무겁다. 헬퍼 (write_toml_path) 공개로 분리.
2. **startup 함수 호출 동기성**: Tauri `.setup()` 콜백은 동기. blocking 호출은 별도 스레드로 격리.
3. **configFields 그룹 경계**: 기존이 toml 섹션 ([vector_db]) 와 일치하지만 search_top_k 등 일부는 다른 의미. 신규 추가 시 toml 섹션 단위 권장 — 사용자 멘탈 모델과 일치.

## 개선

### C1 2단계 적용 흐름

```
사용자: Decision Log 열기 → suggested entry 검토
↓ Accept 클릭
Tauri: accept_suggested_decision(decision_id)
  → auto_suggester::apply_suggested(db, cfg_path, id)
    → get_decision → after_value 추출
    → toml_edit으로 path 쓰기 (주석 보존)
    → .toml.bak 백업
    → decision: suggested → accepted
↓ 갱신된 카드 표시
```

### 이중 처리 방지

```rust
if entry.decision != "suggested" {
    anyhow::bail!("이미 처리된 entry (decision={}): id={}", entry.decision, decision_id);
}
```

테스트로 보장: `test_apply_suggested_writes_toml_and_marks_accepted` 마지막 단계에서 재호출 → 에러 확인.

### startup 트리거 표준 패턴

```rust
.setup(|app| {
    // ... 기존 백그라운드 thread ...

    // C1 startup 자동 추천 (blocking call을 메인 스레드 밖으로)
    std::thread::spawn(|| {
        let data_dir = config::find_data_dir(None);
        match SettingsDb::open_or_migrate(&data_dir) {
            Ok(db) => match auto_suggester::suggest_from_counters(&db) {
                Ok(n) if n > 0 => tracing::info!("[c1-startup] {}건 INSERT", n),
                Ok(_) => tracing::debug!("[c1-startup] no-op"),
                Err(e) => tracing::warn!("[c1-startup] 실패: {}", e),
            },
            Err(e) => tracing::warn!("[c1-startup] DB 열기 실패: {}", e),
        }
    });
})
```

### configFields 그룹 추가 시 체크리스트

1. toml 섹션과 1:1 매칭 권장 (사용자 멘탈 모델)
2. 기본값 = 비활성 시 명시 (lesson 30)
3. 디폴트 변경 트리거 조건을 description에 포함

## 결과

- workspace lib 321 → 323 (shared 88 → 90: apply/reject 2건 신규)
- Tauri commands 64 → 67 (clear_llm_cache + accept/reject_suggested_decision)
- MCP tools 24 → 27
- configFields: search 그룹 신규 (expand_kg_hops + diversity_threshold)
- 컴파일 경고 0, Tauri GUI check 통과

## 후속

- Dashboard에 Decision Log 카드 (현재 stage 2 API만, UI는 setup_decision_log_list MCP tool 또는 향후 카드)
- startup 트리거를 watcher tick 주기로 확장 (1시간/4시간) 검토
- A2/B1 디폴트 변경은 5K 코퍼스 측정 후 (lesson 30 트리거)
