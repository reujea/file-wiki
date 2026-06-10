---
date: 2026-04-28
phase: 60 (진행 중)
---

# module 워크스페이스 분리 — 골격 생성 시 placeholder 필수

## 상황

- `C:\dev\claude_workspaces\module\`에 재사용 라이브러리 워크스페이스를 신설하면서 8 크레이트(api 4 + 구현체 4) 구조로 시작
- module/Cargo.toml의 [workspace] members에 8개를 모두 등록하고, module-secrets만 먼저 채우려 함

## 문제

`cargo check -p module-secrets` 실행 시 즉시 실패:

```
error: failed to load manifest for workspace member `module-storage`
       failed to read `module-storage/Cargo.toml`
       지정된 경로를 찾을 수 없습니다. (os error 3)
```

cargo는 `-p` 옵션으로 단일 크레이트만 빌드하더라도 **워크스페이스 manifest 로드 단계에서 모든 멤버를 검사**한다. members에 등록만 하고 폴더가 없으면 빌드 0건 진행 안 됨.

## 원인

- 관심사 분리 차원에서 "필요할 때만 폴더 생성"하려 했으나 cargo workspace는 그것을 허용하지 않음
- `cargo new`나 `cargo init`을 모든 멤버에 미리 실행하지 않아 발생

## 개선

각 멤버의 최소 골격을 미리 생성:

```
module-{name}/
├── Cargo.toml          # [package] name + 비어있는 [dependencies]
└── src/lib.rs          # `//! Step N에서 구현됨` 한 줄
```

이러면 workspace 전체 cargo check가 통과하고, 이후 단계에서 한 멤버씩 채워나가면 됨. **단계별 빌드 검증이 가능해진다는 게 핵심**.

## 재발 방지

- 워크스페이스 신설 시 **모든 멤버 폴더에 placeholder Cargo.toml + src/lib.rs를 동시에 생성**한 뒤 첫 cargo check를 통과시킨다
- "차차 만들겠다"는 위험 — workspace는 partial state를 허용하지 않음
- 단일 멤버부터 부분 분리하는 전략은 멤버를 등록하지 않고 완성된 후에만 members에 추가하는 형태로 가능 (단, 각 멤버의 의존을 즉시 검증할 수 없음)
