# Lesson 29: CLI ↔ Tauri GUI 데이터 격리 (exe_dir 기준 경로)

## 상황

2026-05-14 Tauri GUI 실 파일 가공 검증 중, pipeline.exe stats로 가공 결과를 확인하려 했으나 "총 문서 수: 0" 반환. Tauri의 .local-store.json에는 **documents 5건**이 정상 영속화되어 있었음.

원인 추적:
- pipeline.exe 위치: `src/target/release/pipeline.exe` → `.local-store.json`을 `src/target/release/`에서 조회
- Tauri GUI 위치: `src/modals/app/target/release/file-pipeline-tauri.exe` → `.local-store.json`을 `src/modals/app/target/release/`에서 조회
- **같은 cwd에서 두 명령을 실행해도 데이터는 서로 다른 디렉토리에 격리**

## 문제

1. 사용자가 GUI에서 가공한 데이터를 CLI stats로 확인 불가
2. CLI batch 처리한 결과를 GUI Dashboard에서 미인지
3. **stats 카운트 = 0** 같은 오해 가능 (실제로는 데이터가 다른 곳에 있음)
4. 단일 바이너리 통합 (Phase 13 — "GUI+CLI 단일 바이너리")라는 architecture 원칙이 무색해짐

## 원인

`platform::default_base_dir()` 또는 `std::env::current_exe().parent()` 기반 경로 결정 (lesson 54). 의도는 "exe 옆에 데이터 둠" — 첫 실행 UX에 유리. 하지만:
- 빌드 경로가 분리되면 (`target/release` vs `modals/app/target/release`) 데이터도 분리
- 배포 시 두 .exe를 같은 폴더에 두면 해결되지만, 개발 환경에서는 분리됨
- 환경변수 `PIPELINE_BASE`로 override 가능하지만 사용자가 알아야 함

## 개선 (옵션)

### 옵션 1: 빌드 시 통합 (현재 architecture 권장)
- 배포 산출물에 GUI + CLI 모두 같은 폴더에 → exe_dir 동일 → 데이터 공유
- 단점: 개발 환경 (target/ 분리)에서는 여전히 격리

### 옵션 2: 명시적 --data-dir 옵션
- 두 진입점에 `--data-dir <PATH>` 옵션 추가
- 환경변수 `PIPELINE_BASE`는 이미 지원 — 명시 옵션은 그것의 wrapper
- 사용자 가이드에 "GUI와 CLI를 같이 쓰려면 PIPELINE_BASE 설정" 명시

### 옵션 3: 사용자 홈 디렉토리 default (lesson 54의 반대 방향)
- `~/.file-pipeline/` 같은 표준 위치 default
- 첫 실행 UX는 약간 떨어지나 (auto_init 시 안내 필요) 두 진입점 자동 통합
- breaking change 위험

### 적용 결정 (2026-05-14)

**옵션 2 채택** — 사용자 가이드에 PIPELINE_BASE 환경변수 설정 안내. 코드 변경 없음 (이미 지원).

`doc/gui-test-scenarios.md` 환경 준비 섹션에 다음 추가:
```powershell
# CLI와 Tauri GUI가 같은 데이터를 공유하려면
$env:PIPELINE_BASE = "$env:TEMP\file-pipeline-data"
```

## 재발 방지

- 신규 진입점(예: MCP 서버, daemon) 추가 시 데이터 디렉토리 결정 로직 동일하게 사용
- 배포 패키지는 GUI + CLI 단일 폴더 강제
- 사용자 가이드에 "데이터 위치는 exe 옆 / PIPELINE_BASE override" 명시

## 참고
- lesson 54: auto_init이 cwd가 아닌 exe_dir 사용 — 원본 원칙
- lesson 60: Named Mutex로 GUI 단일 인스턴스 보장 — 같은 exe_dir만 막음
- architecture.md "Phase 13: 단일 바이너리 통합" — 원래 의도

## 후속 보강 (✅ Phase 85/88에서 종결)

- **Phase 85 B-4 (lesson 38)**: `resolve_paths` base 결정을 `find_data_dir`에 위임 — CLI/Tauri가 같은 분기 트리(PIPELINE_BASE → cwd settings.db/toml → exe_dir → APPDATA) 사용 통일
- **Phase 88 측정 중 발견 (lesson 42)**: `LocalVectorStore::new()`가 PIPELINE_BASE 미통합 (별도 `current_exe` 사용) → `resolve_data_base()` 헬퍼 추가로 동일 분기 트리 적용. 헥사고날상 adapters는 shared 의존 금지이므로 코드 사본 유지 (4번 분기 APPDATA 생략).
- 다음 점검: cwd/exe_dir 사용처 grep 검증 (월 1회). META.md "같은 의미 함수 다중 정의" 사례.
