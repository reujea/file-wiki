# 교훈 #17 — module-api/impl 분리 시 의존 누수 점검 6단계

## 상황

Phase 60 단계 1에서 `module-storage-api`(trait + 타입만)와 `module-storage`(zstd/S3/WebDAV/Network/Null 5개 raw 구현)를 외부 워크스페이스 `C:\dev\claude_workspaces\module\`에 분리. file-pipeline 측 5개 storage 어댑터는 thin wrapper로 전환하여 `RemoteStoragePort`(core trait)는 잔류시키고 내부에서 module raw 호출 + `StorageError → anyhow::Error` 변환만 수행.

## 문제

`cargo check --workspace` 통과만으로는 module 크레이트가 `file-pipeline` 도메인 또는 `anyhow` 같은 호출자 의존을 우발적으로 끌어들이는 누수를 잡을 수 없다. 형제 프로젝트(약 20개)가 module만 의존했을 때 의존 트리가 폭발하거나 컴파일이 실패할 가능성이 있다. 특히 다음 두 케이스가 빈번하다:

- `anyhow::bail!`/`anyhow::Result`를 module 내부에 두면 호출자가 `anyhow`를 강제로 가져가게 됨
- `pub use file_pipeline_*::...`를 무심코 작성하여 워크스페이스 외부 의존이 들어옴

## 원인

- thiserror Error enum 미정의 시 `anyhow::bail!(...)`로 메시지를 흘려보냄 → API 시그니처에 `anyhow` 노출
- workspace transitive dependency는 `cargo build`/`cargo check`가 **무시**한다 (직접 의존만 확인)
- thin wrapper 작성 시 `anyhow::Error::msg(e.to_string())` 변환을 잊고 module 측에 anyhow를 두는 실수가 잦음

## 개선 (점검 6단계 체크리스트)

매 module 분리 단계 종료 시 아래 6단계를 PR/커밋 본문에 명시한다:

1. **`module-{name}-api` Error는 `thiserror::Error` enum**: `Backend / NotFound / Auth / Codec / Io / Config / Other` 등 의미 있는 변형으로 정의. `anyhow::Result` 노출 금지.
2. **impl은 외부 라이브러리 에러를 `map_*_err()`로 thiserror enum에 흡수**: `module-secrets`의 `map_keyring_err` / `module-storage`의 `map_io / map_codec / map_backend` 패턴 따름.
3. **`cd module && cargo tree -p module-{name} | grep file_pipeline_` → 0건**: transitive 누수 검사.
4. **`module/Cargo.toml`의 `[dependencies]`에 file-pipeline path dep 0건**: 단방향 의존(`file-pipeline → module`)만 허용.
5. **`grep -rn "file_pipeline\|file-pipeline" module-{name}/src` → 코드 라인 0건** (주석만 허용): module 코드는 도메인 무관 raw 수준만.
6. **형제 시뮬레이션** (Phase 60 마지막 1회 — 매 단계 X): 빈 임시 크레이트에서 `module-{name}`만 의존하여 import → `cargo check` 통과 여부 확인. 의존 트리 폭발/충돌 검사.

### 단계 1 실측 결과 (module-storage)

| 점검 | 결과 |
|------|------|
| 1. thiserror Error enum | `StorageError::{Io, Codec, Backend, Auth, Config, NotFound, Other}` ✅ |
| 2. map_* 변환 함수 | `map_io / map_codec / map_backend` 정의 ✅ |
| 3. cargo tree file_pipeline 매치 | 0건 ✅ |
| 4. module/Cargo.toml file-pipeline path dep | 0건 ✅ |
| 5. module-storage/src grep | 주석 1건 (코드 0건) ✅ |
| 6. 형제 시뮬레이션 | Phase 60 마지막에 1회 수행 예정 |

추가 발견:
- file-pipeline thin wrapper에서 `anyhow::Error::msg(e.to_string())` 변환이 일관되도록 storage/mod.rs에 `pub(crate) fn map_err()` 헬퍼 1개로 통일 (5개 파일 중복 제거).
- adapters Cargo.toml에서 zstd/sha2/hex direct dep 제거 (module로 이관됨, dead dep 정리).
- bench_scale.rs:79 `% content_lines.len()` 0 나누기 패닉 — 빈 파일 fixture 의존 사전 결함을 동시 정리(Q3 정책).

## 재발 방지

- 위 6단계 체크리스트를 매 module 분리 단계 종료 시 commit message에 명시 (Phase 60 단계 1/2/3-1/3-3/3-2 5회).
- thiserror Error 없이 `anyhow`를 노출하는 module은 머지 금지.
- 형제 시뮬레이션은 Phase 60 마지막 1회 (단계 4)에 일괄 검증 — 매 단계 반복은 비용 대비 효과 낮음 (Q2 결정).
- thin wrapper는 자유 변환 함수 1개로 통일하여 변환 누락 방지 (`map_err` 헬퍼 패턴).
