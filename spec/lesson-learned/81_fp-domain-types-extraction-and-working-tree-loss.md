# 81. fp-domain-types 추출 (d2) — working tree 타입 손실 사고 + 인코딩 함정 + 추출 입도

## 상황

cycle 7 module-storage-db-1 step-d2. cycle 6에서 d1이 BLOCKED 됐다 (DB 본체 = core 도메인 타입 깊은 결합 → 외부 crate 분리 시 `module-storage-db → file_pipeline_core` 순환). 사용자 결정 = **순수 도메인 타입 + 출력 포트 trait를 `fp-domain-types` crate로 추출** 후 core/module-storage-db 양쪽이 의존. core는 re-export shim으로 호출처 경로 호환 유지.

## 문제

세션 중 3개의 별도 문제가 누적됐다.

### 문제 1 — PowerShell `Get-Content -Raw`의 UTF-8 한글 오독 (lesson #6 재발)

경로 치환(`crate::domain::` → `crate::`)을 PowerShell `Get-Content -Raw | -replace | Set-Content -Encoding utf8`로 처리. `Get-Content -Raw`가 UTF-8 파일을 시스템 코드페이지(CP949)로 **잘못 읽어** config_models.rs/models.rs의 한글 24/7건이 mojibake(`以??섎굹`)로 깨졌다. 원격 빌드에서 `unknown start of token: \u{2478}` + `mismatched closing delimiter`로 발견.

### 문제 2 — git HEAD 복원으로 cycle 6 미커밋 타입 4건 영구 손실 (메타 룰 18)

문제 1 복구 과정에서 손상 파일을 "원본"으로 되돌리려 **git HEAD에서 복원**했다. 그러나 cycle 6 `cli-prompt-remove-1`(5/5 완결)이 **미커밋 상태**였고(file-pipeline 단독 git, 마지막 커밋 86c4f4f은 cycle 5 baseline), HEAD에는 해당 변경이 없었다. 결과로 다음 4건이 손실:

- `DuplicateResolutionConfig` (config_models)
- `SensitiveResolutionConfig` (config_models)
- `SensitiveAction` enum (models)
- `DuplicateAction::from_config_str` (models)

adapters/driving/auto_*.rs(working tree 생존)가 이들을 import하는데 정의가 사라져 `unresolved import` 4건. `git log --all -S`로도 history에 전무(미커밋 확정).

### 문제 3 — d1 baseline 추정("self-contained") 4건 빗나감 + d3 게이트 5번째 (메타 룰 18)

d1은 "models.rs는 비테스트 core 의존 0"이라 판단했으나, 본 구현에서 models/포트가 참조하는 순수 타입이 다른 도메인 파일에 섞여 있음이 차례로 드러났다:

| 누락 의존 | 원본 파일 | 참조처 |
|----------|----------|--------|
| `VerificationThresholds` | verification.rs | DocTypeDef.thresholds 필드 + thresholds_for* |
| `HookDefinition`/`HookEvent` | hooks.rs | PipelineConfig.hooks |
| `KgQueryResult/Node/Edge/Stats` | wiki_export.rs | GraphDBPort 반환형 |
| `ChunkQualityMetrics` | chunking_quality.rs | Metadata.chunk_quality |
| `MinHashIndex`(+TaskQueue 등) | crossref_optimizer.rs | LocalVectorStore 필드 (**d3 게이트에서 발견**) |

## 원인

- (1) PowerShell `Get-Content`/`Set-Content`는 기본 인코딩이 시스템 로캘 의존. UTF-8 한글 파일에 안전하지 않다. lesson #6("자동 치환 후 수동 검토")의 인코딩 축 재발.
- (2) "git HEAD = 안전한 원본"이라는 추정. 단독 git + 미커밋 작업 누적 상태에서 HEAD는 **여러 cycle 뒤처진 과거**. 손상 복구를 외부 진실원에서 할 때 그 진실원이 최신인지 미검증.
- (3) d1 baseline이 "파일 단위 import"만 보고 "self-contained"로 단정. 실제 결합은 **필드·반환형 단위**(struct 본문 안 `crate::domain::X::Y`)라 import 라인만으로는 안 잡힌다.

## 개선

1. **한글 포함 파일 텍스트 치환 = Python `io.open(encoding="utf-8")`만 사용**. PowerShell `Get-Content -Raw`/`Set-Content` 금지. 치환 후 mojibake 스캔(`以/⑸땲/섎굹` 등) 의무. → 메타 룰 (lesson #6) 인코딩 sub-rule 강화.
2. **손상/손실 복구 시 진실원 최신성 검증 의무**. git HEAD에서 복원하기 전 `git log --oneline -3`로 HEAD가 현 작업 cycle을 포함하는지 확인. 미커밋 변경이 있으면 HEAD ≠ working tree. 복구 불가 시 spec(deprecated.md 복구 방법 entry) + 사용처 역설계로 재구성. (본 건은 deprecated.md §68 + plan step-c2 + adapters 사용처로 4건 결정적 재구성 성공.)
3. **추출 baseline은 import 라인이 아닌 "필드/반환형 타입" 단위로 grep**. `crate::domain::` 전체 출현을 struct 본문 포함해 스캔. "self-contained" 단정 금지 — 메타 룰 18(추정 재검증) 추출 작업 sub-rule.
4. **추출 입도 패턴 확립**: 순수 데이터 파일은 통째 이관(models/config_models/settings_models/vec_io/crossref_optimizer), 로직과 섞인 파일은 **순수 타입만 분리 모듈**(verification_thresholds/kg_types/chunk_quality/hooks) + 원본은 로직 잔류 + re-export. 6개 도메인 파일이 자연스럽게 이 2 분류로 갈렸다.

## 결과

- `fp-domain-types` crate 신설: 순수 타입 5파일 통째 + 순수 타입 4모듈 분리 + 포트 2 + crossref_optimizer.
- core = re-export shim 7 + 로직 잔류 5 (verification ROUGE-L / wiki_export KgQueryEngine / chunking_quality compute_* / hooks HookRegistry / crossref 없음=통째). **호출처 변경 0**.
- 의존 방향 단방향 확정: `core → fp-domain-types ← module-storage-db(예정)`. DB 본체 6 모듈 의존이 전부 fp-domain-types로 닫힘 → d1 BLOCKED 해소, d3 진입 게이트 통과.
- 빌드 5종 PASS (fp-domain-types / core / 전체 workspace check --all 경고0 / _rust_module 26멤버 / Tauri GUI). nextest 비-bench 전부 PASS (실패 3건 = bench 병렬 비결정성, d2 무관).

## 사이드 발견

`bench_crossref_variants`가 전체 nextest 병렬 실행 시 FAIL(변형>baseline 단언) / 단독 PASS. metadata blocking이 baseline과 동률(5402) 경계 + read_dir 순서 의존 추정. `bench_scale_1000/5000`은 병렬 CPU 경합 timeout. 전부 d2 무관 — bench는 단독/순차 게이트로 분리 필요 (lesson #4 병렬 편향 변형). 후속 트리거 후보.
