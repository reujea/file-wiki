# Lesson 27: 구조체 필드 추가 시 통합 테스트 초기화 누락 (lesson 21 재발)

## 상황

Phase 82-prep (2026-05-14)에서 `FileProcessingService`에 `metrics_recorder: Option<Arc<dyn ProcessingMetricsPort>>` 필드를 추가. service.rs의 `build_service` 호출처와 service.rs 내 테스트 헬퍼는 갱신했으나, `modals/cli/tests/`의 통합 테스트 12개 파일 13곳이 누락. `cargo check --workspace`는 lib만 검사하므로 통과. 다음 세션에서 통합 테스트 실행 시 E0063 13건 동시 발생.

## 문제

- 누락된 파일: actor_scenarios / benchmark / bench_micro / bench_prompt_compare / bench_real / bench_real_corpus / bench_real_docs / bench_scale / e2e_embedded(2곳) / real_env_tests / scenarios / search_accuracy
- 모든 파일이 `FileProcessingService { ... }` 직접 초기화 사용 (Default impl 없음)
- 빌드 환경에서 `cargo check` 통과해도 `cargo build --tests` 실패

## 원인

1. **lesson 21 패턴 재발**: 2026-04-15 "service.rs 필드 추가 시 테스트 5곳 누락"에서 builder 패턴 도입 검토만 하고 미적용. 6개월 후 동일 패턴 재발.
2. **헥사고날 비용**: core 크레이트는 std만 의존 (lesson 28/90)이라 builder 매크로(`derive_builder` 등) 도입에 cargo toml 검토 필요. 미루는 사이 누적.
3. **테스트 헬퍼 분산**: 12개 통합 테스트가 각자 `FileProcessingService { ... }`를 직접 빌드. 공통 헬퍼(`setup_service`) 사용은 `bench_micro` 등 일부에 한정.
4. **`cargo check` 의 한계**: workspace lib만 검사. 통합 테스트는 `cargo build --tests` 또는 실행 단계에서만 잡힘.

## 개선 (즉시 처리)

13곳에 `metrics_recorder: None,` 추가. 마지막 필드 자리(`crossref_interval_secs` 다음)에 일관 삽입. PowerShell 단순 문자열 replace로 처리 (lesson 6 sed 일괄 후 수동 검토 원칙 — `cargo build --tests` 통과로 검증).

## 재발 방지

### 단기 (즉시 적용)
- **체크리스트**: `FileProcessingService` 또는 다른 핵심 도메인 구조체 필드 추가 시:
  1. `grep -rln "{StructName} {" --include="*.rs" src/` → 모든 초기화 파일 식별
  2. 핵심 lib 갱신 + 모든 통합 테스트 파일 동시 갱신
  3. `cargo build --tests --workspace` 통과 확인 (lib check만으로 부족)
- **CLAUDE.md 승격**: "구조체 필드 추가 = lib + tests 동시" 규칙은 반복이라 src/CLAUDE.md 빌드 섹션에 명시할 가치 있음.

### 중기 (트리거 대기)
- **`FileProcessingService` Default impl** 또는 `ServiceBuilder` 패턴:
  - Option<T> 필드는 Default::default() = None으로 자동 처리
  - 비-Option 필드(`vector_db: Arc<dyn VectorDBPort>` 등)는 builder로 강제 주입
  - 테스트는 `ServiceBuilder::default().with_vector_db(...).build()` 형태로 → 신규 필드 추가 시 테스트 변경 0건
- 비용 평가: 핵심 구조체 1회 마이그레이션 + 12 테스트 파일 변환. 다음 큰 phase 작업과 결합 시 수행.

### 장기 (구조 원칙)
- core 도메인 구조체에 `#[derive(Default)]` 가능한 필드 설계 우선 (Option<T> 또는 단순 타입)
- 테스트 헬퍼 중앙화: `crates/core/src/service.rs` 내 `FileProcessingService::for_tests(base: &Path)` 같은 공식 테스트 빌더 제공
- "lib 통과 ≠ tests 통과" 자동화: CI에 `cargo build --tests --workspace` 또는 nextest 추가 (현 환경엔 nextest 미설치)

## 참고
- lesson 21 (2026-04-15): "service.rs에 summary 필드 추가 시 테스트 5곳 누락" — 원본 패턴
- lesson 26 (2026-05-13): DB schema 이중 정의 동기화 누락 — "같은 사실이 여러 곳에 있을 때 동기화" 메타 패턴
- 본 lesson은 lesson 21/26과 같은 메타 패턴. **META.md 인덱스 추가 검토**.

## 해소 표시
- 즉시 처리 완료 (2026-05-14): 12파일 13곳 `metrics_recorder: None,` 추가. 통합 테스트 빌드 통과 + 58/59 실행 통과 (1건은 lesson 28의 사전 결함).
- **중기 builder 도입 완료 (2026-05-14)**: `crates/shared/src/test_helpers.rs`에 `ServiceBuilder` 추가. 모든 도메인 필드를 안전한 stub 기본값으로 초기화 + with_* 메서드로 커스텀 어댑터 주입. 자체 단위 테스트 2건 통과 (shared 77→79). 향후 `FileProcessingService` 필드 추가 시 본 빌더의 `build()`만 수정하면 새 테스트는 변경 0건.
- **빌더 확장 2026-05-14**: 도메인 스칼라 전체 노출 — semantic_dup_threshold / max_retry / verification_enabled / fragment_threshold / crossref_supersedes_threshold / crossref_keyword_overlap_min / crossref_top_k / crossref_caps / minhash / metadata_blocking / embed_instruction_prefix / global_thresholds. 12파일 마이그레이션 시 필요한 모든 변경점 흡수.
- **🎉 기존 12 통합 테스트 점진 마이그레이션 12/12 완료 (2026-05-14)**:
  - ✅ scenarios.rs (50줄 → 10줄, 10/10 통과)
  - ✅ actor_scenarios.rs (51줄 → 11줄, 4/4 통과)
  - ✅ search_accuracy.rs (50줄 → 11줄, 12/12 통과)
  - ✅ bench_prompt_compare.rs (50줄 → 12줄, 빌드 통과 — claude CLI 없으면 자동 스킵)
  - ✅ real_env_tests.rs (58줄 → 18줄, 13/13 통과)
  - ✅ bench_real.rs (41줄 → 10줄, 빌드 통과 — PIPELINE_REAL_BENCH 환경변수 의존)
  - ✅ benchmark.rs (44줄 → 10줄, 3/3 통과)
  - ✅ bench_real_docs.rs (64줄 → 9줄, 빌드 통과)
  - ✅ bench_real_corpus.rs (88줄+82줄 → 13줄+13줄, 빌드+실 코퍼스 24.4 docs/s 통과)
  - ✅ bench_micro.rs (71줄 → 14줄, 6/6 통과 343s)
  - ✅ bench_scale.rs (53줄 → 14줄, scale_100 12.1 docs/s + scale_1000 19.6 docs/s 회귀 기준선 통과)
  - ✅ **e2e_embedded.rs (85줄+85줄 → 19줄+19줄, 21/21 통과)** — 마지막 1건, 4.61s

**lesson 21/27 근본 차단 완료**: 향후 `FileProcessingService` 도메인 필드 추가 시 통합 테스트 12파일 모두 변경 0건. `ServiceBuilder::build()` 한 곳만 수정하면 됨.
- **신규 통합 테스트는 `ServiceBuilder` 사용 의무** (src/CLAUDE.md 아키텍처 규칙에 반영됨).
