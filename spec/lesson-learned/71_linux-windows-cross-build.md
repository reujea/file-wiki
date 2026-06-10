---
created: 2026-06-04
phase: 원격 빌드 검증 (cargo-xwin MSVC cross-compile)
remote_host: ubuntu@172.16.13.45 (Ubuntu 22.04, 4core, 87G 여유)
meta_rules:
  - 메타 룰 17 (release 재빌드 + 배포 의무) — **강화 3건째 = 2026-06-05 강화 정식 승격** (Phase 106 + Phase 107 + 본 lesson 누적 3건 도달, 메타 룰 23 §승격 3요소 모두 충족)
  - 메타 룰 18 (추정 빗나감) — 12번째 (icon.png / llvm-rc / Linux Tauri)
  - 메타 룰 1 sub-rule 1g (CLAUDE.md stale) — lesson 67 동시 영역
  - 메타 룰 25 (자기 적용 의무) — 메타 룰 17 강화 정식 승격 직후 cross-build 사전 확인 의무를 신규 작업 체크리스트에 포함
---

# Lesson 71 — Linux → Windows cross-build (cargo-xwin) 사이드 발견 5건

## 상황

사용자 "tasty 원격 서버에서 빌드/테스트 진행. Windows 빌드까지 원격에서 완료". cargo-xwin (MSVC x86_64-pc-windows-msvc 타깃) 선택. 다음 사이드 발견 5건 누적.

## 사이드 발견 1: icon.png 누락 (Tauri Linux 빌드)

- Windows 빌드는 `icon.ico`만으로 통과
- Tauri `generate_context!` 매크로의 Linux 분기가 `.png` 강제 요구
- 1차 빌드 실패 → ImageMagick `convert icon.ico → icon.png` 1-bit colormap 생성 → RGBA 아님으로 또 실패
- 2차 해소: `convert "icon.ico[0]" -resize 256x256 -define png:color-type=6 PNG32:icon.png` → 256×256 RGBA 8-bit 통과

### 개선
- 영구 해결: `tauri.conf.json`에 `.png` 추가 또는 Linux 빌드 시점에만 자동 생성 스크립트
- 본 세션은 임시 해결 (사용자 결정으로 placeholder만)

## 사이드 발견 2: llvm-rc 미설치 (tauri-winres 의존)

- cargo-xwin은 MSVC SDK는 다운로드하지만 **Windows resource compiler (rc.exe / llvm-rc)는 별도 의존**
- `tauri-winres-0.3.5` 1차 빌드 실패: `NotAttempted("llvm-rc")`
- 해소: `apt install llvm` → `/usr/bin/llvm-rc` 가용 → `PATH=/usr/bin:$PATH cargo xwin build` 통과

### 개선
- 원격 환경 사전 점검 체크리스트에 `llvm` 패키지 추가 의무
- Tauri cross-build 시 cargo-xwin + llvm 패키지 묶음으로 표시

## 사이드 발견 3: cargo-xwin 사전 설치

- 원격에 cargo-xwin v0.22.0 + x86_64-pc-windows-msvc 타깃 이미 설치되어 있었음
- 이전 세션 흔적이지만 prd/spec에 미등재
- 환경 사전 조사 메타 룰 후보

### 개선
- 본 lesson에 cargo-xwin 가용 사실 등재
- 신규 외부 환경 사용 시 환경 상태 사전 grep 의무 (메타 룰 18 확장)

## 사이드 발견 4: unused_mut service.rs:17

- 2회 빌드 모두 동일 경고: `let (db, mut cfg, registry) = config::load_from_db(None)?;`
- `mut cfg` 사용하지 않음 → 영구 정리 후보
- 본 세션은 빌드 완료 우선이라 미정리

### 개선
- 다음 위생 phase에서 정리

## 사이드 발견 5: workspace cross-build 시간 차이

- 로컬 Windows 빌드 (Phase 90 측정): 약 25분 cold full 추정
- 원격 Linux native release: **6m 15s** (4배 빠름)
- 원격 Linux cross MSVC: workspace 3m 26s + Tauri 5m 55s = 약 9m
- **결론**: 원격 cross-build는 로컬 cold full 대비 약 3배 빠름 (4 core VM 기준)

## 결과 산출

| 산출 | 크기 | sha256 |
|------|------|--------|
| `D:\file-test\pipeline.exe` (Tauri rename) | 19.03 MB | `6afbf8a9...68fc7b` |
| 원격 ↔ 로컬 sha256 일치 | ✅ | |

## 메타 룰 적용

| 메타 룰 | 적용 |
|---------|------|
| 17 (release 재빌드 의무) | 신규 사례 — Linux cross-build도 자동 배포 (D:\file-test) 영역 확장 후보 |
| 18 (추정 빗나감) | 12번째 (icon.png / llvm-rc 사전 추정) |
| 22 (사용자 정책 합의) | cross-build 방식 (cargo-xwin) + 배포 대상 (Windows만) 2축 합의 |

## 메타 룰 17 강화 후보 +1 (총 3건 누적)

| Phase | 사례 |
|-------|------|
| Phase 106 | D:\file-test 재배포 누락 (Windows 빌드) |
| Phase 107 | release 재빌드 의무 (Windows 빌드) |
| **본 세션** | **Linux cross-build 후 D:\file-test 재배포** (신규 환경) |

→ 3건 도달 — 다음 phase 메타 룰 17 강화 정식 승격 검토.
