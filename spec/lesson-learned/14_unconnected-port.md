# 14. 미연결 포트는 코드 부담만 남긴다

## 상황

Phase 41에서 `GraphDBPort` trait + `JsonGraphDb` + `Neo4jGraphDb` 어댑터 2종을 추가했음. KG 영속화/쿼리에 사용할 의도였으나, `FileProcessingService` 등 어떤 진입점에서도 이 포트를 호출하지 않음.

Phase 55 전문가 자문에서 "GraphDB 비활성화" 결정. config UI에서만 제거하고 코드는 보존.

Phase 58에서 코드 완전 삭제. 영향 범위:
- `src/crates/adapters/src/driven/graph_db/` (3 파일, ~150줄)
- `src/crates/core/src/ports/output.rs`의 `GraphDBPort` trait 정의
- 포트 12개 → 11개

## 문제

- 7개 Phase(41~58)에 걸쳐 미연결 코드가 잔존
- "이 포트는 곧 연결될 거야"라는 가정이 잊혀지면서 dead code화
- 외부 자문에서 "필요 없음" 판정 받기 전까지 아무도 결정 권한이 없는 상태로 묶임
- 컴파일은 통과(trait 자체는 유효) → cargo check로 잡히지 않음

## 원인

포트(trait)는 추상화이므로, **구현체가 있어도 호출자가 없으면 dead code**다. 그러나 Rust 컴파일러는 trait+impl을 dead로 표시하지 않는다 (`pub`이므로). 결과적으로 정적 검증으로는 탐지 불가.

## 개선

새 포트 추가 시 다음 중 하나를 **명시적으로** 결정:

| 상태 | 표기 | 액션 |
|------|------|------|
| 즉시 사용 | (기본) | 동일 PR에서 service에 wiring |
| 다음 Phase 사용 | `#[allow(dead_code)] // [Phase X] 연결 예정` | 해당 Phase까지의 명확한 데드라인 |
| 실험/POC | `pub(crate)` + 별도 모듈 + `cfg(feature)` | 활성 기능 외부에 격리 |
| 미래 옵션 | 포트 추가 보류 | trait이 아니라 issue/discussion으로 |

## 재발 방지

월 1회 또는 Phase 종료 시 다음 점검:

```bash
# 어댑터 구현체 목록
grep -rn "^impl.*Port for " src/crates/adapters/

# 각 impl이 service.rs/build_service에서 호출되는지 확인
grep -rn "Arc::new(.*Adapter)" src/crates/shared/src/
```

호출 경로가 없는 impl은 **삭제** 또는 **명시적 보류 마커** 추가.
