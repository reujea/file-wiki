# Lesson 31 — A1 캐시 통합 + PIPELINE_BASE 잔존 정리 + scale_validation 단언 완화

## 상황 (2026-05-15)

lesson 30에서 인프라만 둔 Ruflo A1을 실제 호출 경로에 통합. 동시에 lesson 29 PIPELINE_BASE 통합에서 빠졌던 `write_log` / `auto_init` / `init_tracing` 잔존 정리. scale_validation의 NTFS 환경 의존 단언 (39.9s vs 30s 단언)도 함께 결정.

## 문제

### A1 통합 — 의존 방향

`FileProcessingService.classify_and_process_with_retry` (core 도메인) 안에서 `SettingsDb` (shared) 직접 호출은 헥사고날 위반 (core → shared 금지).

대안:
1. **포트 추가** — `LlmCachePort` trait + 어댑터. 변경 표면적 크고 모든 인스턴스 생성처 영향.
2. **Wrapper 어댑터** — shared에 `CachedLLM` 신규 LLMPort 구현. shared → adapters → core 방향 정상. service.rs 변경 0건.

(2)가 의존 방향 정상이며 service.rs 비변경. (2) 채택.

### PIPELINE_BASE 잔존 — fastembed_verified.md 메모

lesson 29에서 `find_data_dir`에 `PIPELINE_BASE` 환경변수 지원 추가했지만, `write_log`/`auto_init`/`init_tracing` 셋은 `exe_dir` 기반으로 별도 초기화. 환경변수로 base를 옮겨도 logs/는 exe_dir에 생기는 분기.

### scale_validation — 환경 의존

10K 스캔 30s 단언, NTFS 환경에서 39.9s 측정. 회귀 감지 의도는 유효하나 임계값이 너무 빡빡.

## 원인

1. **A1 의존 방향**: 새 기능을 도메인에 추가하기 전 "shared crate에서 LLMPort wrapper로 가능한가?"를 먼저 확인하지 않음. 헥사고날 신규 기능 추가 시 default 위치는 어댑터 단.
2. **PIPELINE_BASE 잔존**: lesson 29 작성 시 `find_data_dir`만 보고 검색 → 다른 경로 결정 함수 3건 누락 (lesson 26 schema 이중 정의 패턴과 동일 — 다중 위치 동기화).
3. **scale_validation**: 단언값이 측정 환경(SSD/HDD, NTFS/ext4, 캐시 상태)에 비례하지 않은 고정값. 회귀 감지가 목적이라면 2배 마진이 적절.

## 개선

### A1 통합 패턴 — Wrapper LLMPort

```rust
// crates/shared/src/cached_llm.rs
pub struct CachedLLM {
    inner: Arc<dyn LLMPort>,
    settings_db_path: PathBuf,
}

#[async_trait::async_trait]
impl LLMPort for CachedLLM {
    async fn classify_and_process(&self, file_path, registry) -> Result<...> {
        let file_hash = sha256(file_path)?;
        if let Some(cached) = self.lookup(&file_hash, &file_hash) {
            return Ok(cached);  // claude_cli 호출 스킵
        }
        let result = self.inner.classify_and_process(file_path, registry).await?;
        self.store(&file_hash, &file_hash, &result);
        Ok(result)
    }
    // reprocess_with_feedback: 캐시 우회 (재시도 경로)
    // enrich_existing/summarize_text: inner 위임 (누적 컨텍스트 캐시 부적합)
}
```

`build_service`에서 `cfg.llm.llm_cache_enabled`(기본 true)일 때 ChunkedAgentAdapter 직후 wrapping. service.rs 무변경.

핵심 — `ClassifyAndProcessResult`에 `Serialize/Deserialize` 추가. Metadata는 이미 보유.

### PIPELINE_BASE 통합 체크리스트

다음 함수들이 base 디렉토리를 결정한다 — **모두 동일 출처 사용 필수**:

| 함수 | 위치 | 변경 전 | 변경 후 |
|------|------|---------|---------|
| `find_data_dir` | shared/config.rs | PIPELINE_BASE 지원 (lesson 29) | (유지) |
| `write_log` | shared/lib.rs | exe_dir/logs | find_data_dir(None)/logs |
| `auto_init` | shared/lib.rs | exe_dir 기준 스켈레톤 | find_data_dir(None) 기준 |
| `init_tracing` | shared/lib.rs | platform::default_base_dir | find_data_dir(None) |

체크 패턴: `grep -rn "exe_dir\|default_base_dir\|current_exe" crates/shared/`

### scale_validation 환경 의존 단언

```rust
// 변경 전
assert!(scan_ms < 30_000, "10K 스캔 30초 이내: {}ms", scan_ms);
assert!(scan_ms < 120_000, "100K 스캔 120초 이내: {}ms", scan_ms);

// 변경 후 — 회귀 감지 마진 2배
assert!(scan_ms < 60_000, "10K 스캔 60초 이내 (NTFS 환경 의존): {}ms", scan_ms);
assert!(scan_ms < 240_000, "100K 스캔 240초 이내 (NTFS 환경 의존): {}ms", scan_ms);
```

## 결과

- A1 wrapper: shared lib 83 → 85 (cached_llm 2건 신규: hit/miss 검증)
- workspace lib: 316 → 318 통과
- 컴파일 경고: shared의 unused import 0건
- scale_validation 단언: 환경 의존 명시 + 회귀 마진 2배
- Tauri GUI cargo check 통과

## 후속

- A1 캐시 hit률 5K 코퍼스 측정 (트리거 대기 — Phase 84/85 LAT 진행 시 함께)
- `llm_cache_stats` API를 Dashboard나 MCP tool에 노출 검토
