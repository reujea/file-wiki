#!/usr/bin/env bash
# release_rebuild_required.sh — release 재빌드 필요 여부 자동 감지 (메타 룰 17 자동화)
#
# 사용:
#   bash spec/benchmarks/scripts/release_rebuild_required.sh
#
# 검사 로직 (메타 룰 17 §분류 체크리스트):
#   재빌드 필요: .rs / ui/*.{js,css,html} 변경
#   재빌드 불필요: *.md / *.toml / *.sh
#
# 감지 모드 (자동 선택):
#   1) git 저장소 → git diff --name-only HEAD
#   2) git 미저장 환경 → find -newer .last-release (마커 파일 기준)
#
# 출력:
#   - REBUILD: 재빌드 필요 파일 목록
#   - 0건이면 exit 0 (재빌드 불필요)
#   - 1건+이면 exit 1 + 빌드 명령 안내
#
# 관련: lesson 46 Phase 90 G-3 / META.md 메타 룰 17 / Phase 95 release 의무 적용

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
MARKER="$ROOT/.last-release"

# 변경 감지 모드 결정
CHANGES=""
MODE=""

if git -C "$ROOT" rev-parse --git-dir >/dev/null 2>&1; then
  MODE="git"
  CHANGES=$(git -C "$ROOT" diff --name-only HEAD 2>/dev/null \
    | grep -E '\.rs$|ui/.*\.(js|css|html)$' || true)
  # git status도 확인 (untracked 포함)
  UNTRACKED=$(git -C "$ROOT" status --porcelain 2>/dev/null \
    | awk '{print $2}' \
    | grep -E '\.rs$|ui/.*\.(js|css|html)$' || true)
  if [ -n "$UNTRACKED" ]; then
    CHANGES=$(printf "%s\n%s" "$CHANGES" "$UNTRACKED" | sort -u | sed '/^$/d')
  fi
elif [ -f "$MARKER" ]; then
  MODE="marker (find -newer .last-release)"
  CHANGES=$(find "$ROOT/src" -type f \( -name '*.rs' -o -path '*/ui/*.js' -o -path '*/ui/*.css' -o -path '*/ui/*.html' \) \
    -newer "$MARKER" 2>/dev/null | sed "s|$ROOT/||g" || true)
else
  MODE="no-baseline (마커 + git 미존재)"
  echo "WARN: $MARKER 미존재 + git 저장소 아님"
  echo "  최초 release 빌드 후 다음 명령으로 마커 생성:"
  echo "    touch $MARKER"
  echo "  이후 본 스크립트가 마커 기준 변경 감지"
  exit 0
fi

echo "== Release 재빌드 필요 여부 ($MODE) =="

if [ -z "$CHANGES" ]; then
  echo "PASS: 코드 변경 없음 → release 재빌드 불필요"
  exit 0
fi

echo "FAIL: 다음 파일 변경 감지 → release 재빌드 필요"
echo "$CHANGES" | sed 's/^/  - /'
echo ""
echo "재빌드 명령 (메타 룰 17 의무):"
echo "  cd src && cargo build --release --all          # workspace (~2~3분)"
echo "  cd src/modals/app && cargo build --release     # Tauri GUI (~5~20분)"
echo ""
echo "fastembed 활성 빌드 (필요 시):"
echo "  cd src && cargo build --release --all --features file-pipeline-shared/fastembed"
echo "  cd src/modals/app && cargo build --release --features fastembed"
echo ""
echo "빌드 후 마커 갱신:"
echo "  touch $MARKER"
exit 1
