#!/usr/bin/env bash
# action_catalog.sh — 80개 액션 카탈로그 grep 추출 (lesson 46 "잘한 것" 1)
#
# 사용:
#   bash spec/benchmarks/scripts/action_catalog.sh           # 카탈로그 출력
#   bash spec/benchmarks/scripts/action_catalog.sh --count   # 카운트만 (회귀 게이트용)
#   bash spec/benchmarks/scripts/action_catalog.sh --diff <기준수>  # 기준 대비 변동 검사
#
# 출력: data-action 속성값 (정렬·중복 제거)
# 회귀 게이트: phase 종결 시 액션 수 비교로 누락/중복 감지
#
# 관련: lesson 46 §"잘한 것" 1 + spec/architecture.md "GUI 액션 카탈로그 (80개)"

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
if [ -d "$ROOT/src/ui" ]; then UI_DIR="$ROOT/src/ui"
elif [ -d "$ROOT/ui" ]; then UI_DIR="$ROOT/ui"
else echo "ERROR: ui/ 미존재" >&2; exit 2
fi

ACTIONS=$(grep -ohE 'data-action="[^"]+"' "$UI_DIR"/index.html "$UI_DIR"/dashboard.js \
  | sed -E 's/data-action="([^"]+)"/\1/' \
  | sort -u)

MODE="${1:-}"

case "$MODE" in
  --count)
    echo "$ACTIONS" | wc -l
    ;;
  --diff)
    BASELINE="${2:-}"
    [ -n "$BASELINE" ] || { echo "ERROR: --diff <baseline_count> 필요" >&2; exit 2; }
    CURRENT=$(echo "$ACTIONS" | wc -l)
    DELTA=$((CURRENT - BASELINE))
    echo "baseline=$BASELINE current=$CURRENT delta=$DELTA"
    # 회귀 게이트: 변동 시 1 exit (사용자 검토 트리거)
    [ "$DELTA" -eq 0 ] || exit 1
    ;;
  *)
    echo "$ACTIONS"
    ;;
esac
