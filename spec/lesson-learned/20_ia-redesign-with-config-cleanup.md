## 상황 (Phase 65, 2026-05-04)

7탭 + Settings 5그룹 구조에 5건의 IA 불일치가 누적: extra_inboxes/max_workers/lint_interval_hours가 Settings에 있지만 파이프라인 영역, rerank가 Settings에 있지만 검색이 UI에 없음, 임베딩+리랭킹이 분리된 같은 BGE-M3 패밀리. 동시에 Phase 64 onnx feature 폐기로 dead config 5건 누적: `vector_db.qdrant_url`, `collection`, `auto_start`, `embedding.sensitive_model`, `embedding.onnx_model_dir`.

## 문제

대규모 IA 재설계(7탭 → 3계층 17서브탭) + dead config 제거 + fastembed 고정 정책을 동시에 적용해야 하는 상황. 한 번에 진행하면 회귀 범위가 크고 디버깅 비용이 폭증. 단계별로 분리하면 중간 상태가 작동하지 않거나 사용자가 어색한 상태를 보게 됨.

`config.rs`의 코드 default와 UI FieldMeta default가 불일치(default_embed_model="open_ai_small" vs UI "fastembed")라는 이중 진실 문제도 발견. 사용자가 "저장"을 누르지 않으면 UI 표시와 실제 동작이 달라짐.

## 원인

1. **IA 결정과 데이터 모델 결정의 분리 부재**: 새 그룹/서브탭을 만들면서도 기존 config 섹션 구조는 그대로 두는 방식이 안전한 절충점이라는 사실을 초기에 인지하지 못함.
2. **default 함수와 FieldMeta default의 동기화 부재**: 두 곳에서 default를 별도 정의해 drift 발생. 단일 진실 원천 정책 부재.
3. **Phase 64 onnx feature 폐기 시 dead config 동반 정리 누락**: 코드만 삭제하고 config 섹션은 그대로 두어 사용자가 의미 없는 필드를 계속 봄.

## 개선

### 패턴 A — IA 변경은 5단계로 분할

```
65-1: 라우팅 + 1차 그룹 nav (기능 변경 0)
65-2: 입력·가공 그룹 (기능 일부 이동)
65-3: 검색·MCP 그룹 (신규 영역 추가)
65-4: 운영 그룹 + Settings 흡수 (가장 큰 분산)
65-5: 검증 + 문서 갱신
```

각 단계는 독립 커밋 가능 + 기능 회귀 0 유지. 4번 단계가 가장 위험해서 가장 마지막에 배치.

### 패턴 B — Settings 탭 폐기 시 데이터 모델은 유지

UI 그룹 라벨만 변경하고 TOML 섹션 구조는 그대로 둔다. 예:
- FieldMeta 그룹명: `("rerank", vec![...])` 그대로 유지
- UI 표시 위치: 검색·MCP 그룹의 리랭킹 서브탭으로 이동
- TOML save 시 동일한 `[rerank]` 섹션에 저장 → 호환성 유지

이 패턴으로 사용자가 가지고 있던 기존 `pipeline.toml`이 손상 없이 그대로 동작. 마이그레이션 코드 0줄.

### 패턴 C — dead config 정리 트리거

기능 코드 폐기와 config 필드 폐기를 동시에 처리하는 체크리스트:

1. 사용처 grep으로 0건 확인 (`vector_db\.qdrant_url`, `cfg\.embedding\.sensitive_model` 등)
2. struct 필드 + default 함수 + Default impl + FieldMeta + needs_restart 5곳 일괄 제거
3. 테스트 assertion 갱신 (default 변경 시 테스트도 동기화)
4. pipeline.toml 샘플 + lib.rs `auto_init()` 템플릿 갱신
5. spec/architecture.md 수치 업데이트

### 패턴 D — 코드 default + UI default 단일화

```rust
fn default_embed_model() -> String { "fastembed".into() }  // 단일 정의

// UI FieldMeta
("default_model", field!("...", "select:...", "fastembed", restart)),  // 같은 값
```

두 곳을 명시적으로 동기화하지 않으면 drift 발생. 이상적으로는 FieldMeta가 default 함수를 참조해야 하지만 const 매크로 제약으로 어려움. 차선책: PR 리뷰 체크리스트에 "default 함수와 FieldMeta default 일치" 추가.

### 패턴 E — fastembed 고정 같은 정책 변경 시 사용자 영향 제로

기존 `pipeline.toml`을 가진 사용자가 영향을 받지 않도록:
- 코드 default 변경(open_ai_small → fastembed)은 신규 사용자만 영향
- 기존 사용자의 toml에 `default_model = "open_ai_small"`이 명시되어 있으면 그대로 사용
- UI에서는 model 드롭다운 제거 — 변경할 수 없게 잠금
- 사용자가 fastembed로 마이그레이션하려면 toml에서 해당 필드를 삭제하거나 "fastembed"로 변경
- 강제 마이그레이션 코드 0줄

이 패턴은 "하위 호환 유지 + 신규 사용자에 권장 default + UI 잠금"의 3축을 동시에 달성.

## 적용 범위

- Phase 65 외에도 향후 IA 재설계 시 5단계 분할 패턴 재사용
- 새 feature 도입 시 default 동기화 체크리스트 적용
- Phase 단위로 dead config 정리 별도 phase 또는 동반 phase로 처리

## 사후 점검

향후 다음 시점에 회귀 점검:
- 사용자가 기존 pipeline.toml로 새 빌드 실행 시 정상 동작 (호환성)
- Settings 탭 폐기 후 사용자가 모든 설정에 도달 가능한지 (분산 노출 검증)
- search_with_trace + MCP 도구 enable/disable 등 placeholder를 실제 구현으로 대체 시 IA 재변경 없이 가능한 구조인지

## 참고
- Phase 64 dead code 정리 패턴 — lesson 19 (10단계 체크리스트)
- vector 관리 규칙 #1: 인덱싱·검색 동일 모델·버전 → 임베딩 default 변경은 재인덱싱 트리거
- "만들기에서 쓰기로 전환" Phase 58 정책 — IA 재설계는 사용자 피드백 기반이 아니라 정합성 정리 목적이라 정책과 부분적 충돌. 다만 dead config 정리는 정합성 정리 효과 큼.
