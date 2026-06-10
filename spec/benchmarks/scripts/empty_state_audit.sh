#!/usr/bin/env bash
# empty_state_audit.sh — 빈 객체 truthy 가드 후보 식별 (G-4 (b) 메타 패턴)
#
# 사용:
#   bash spec/benchmarks/scripts/empty_state_audit.sh
#
# 검출 패턴:
#   1. `state.X = await API.Y()` 또는 `data = await API.Y()` 후 단순 `if (!X)` 분기
#      → 빈 객체 {}는 truthy라 placeholder 분기 통과 → undefined 노출 위험
#   2. 권장 패턴: `if (!X || typeof X.field !== 'expected')` 또는 빈 객체 명시 확인
#
# 출력: 의심 후보 line 번호 (수동 검토 필요)
# 회귀 게이트 아님 — 후보 발견 시 0 exit로 진행 (사용자 검토)
#
# 관련: G-4 (b) renderVerificationMetrics 사례 (dashboard.js:1772)

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
if [ -f "$ROOT/src/ui/dashboard.js" ]; then JS_FILE="$ROOT/src/ui/dashboard.js"
elif [ -f "$ROOT/ui/dashboard.js" ]; then JS_FILE="$ROOT/ui/dashboard.js"
else echo "ERROR: dashboard.js 미존재" >&2; exit 2
fi

# Stage 1: `await API.X()` 응답을 state 또는 const에 직접 할당하는 라인
ASSIGNMENTS=$(grep -nE "(state\.[a-zA-Z_]+|const [a-z][a-zA-Z]*|let [a-z][a-zA-Z]*)\s*=\s*await\s+API\." "$JS_FILE" || true)

# Stage 2: 그 직후 5줄 내 단순 `if (!변수)` 패턴 검출
SUSPECTS=()
while IFS= read -r line; do
  LINENO=$(echo "$line" | cut -d: -f1)
  # 변수명 추출 (간단 휴리스틱)
  VAR=$(echo "$line" | sed -nE 's/.*(state\.[a-zA-Z_]+|const ([a-z][a-zA-Z]*)|let ([a-z][a-zA-Z]*)).*/\1\2\3/p' | head -1 | sed 's/^const //;s/^let //')
  [ -z "$VAR" ] && continue

  # 다음 15줄 내 `if (!변수)` 패턴 검색 (변수가 객체 응답이면 빈 객체 {} 가드 미흡)
  WINDOW_START=$((LINENO + 1))
  WINDOW_END=$((LINENO + 15))
  GUARD=$(sed -n "${WINDOW_START},${WINDOW_END}p" "$JS_FILE" | grep -nE "if\s*\(\s*!${VAR}\s*\)" || true)
  if [ -n "$GUARD" ]; then
    SUSPECTS+=("L${LINENO}: ${VAR} = await API.* (다음 15줄 내 단순 !${VAR} 가드)")
  fi
done <<< "$ASSIGNMENTS"

if [ ${#SUSPECTS[@]} -eq 0 ]; then
  echo "OK: 빈 객체 truthy 가드 후보 0건"
  exit 0
else
  echo "후보 ${#SUSPECTS[@]}건 (수동 검토 필요):"
  for s in "${SUSPECTS[@]}"; do
    echo "  - $s"
  done
  echo ""
  echo "조치: \`if (!data)\` 대신 \`if (!data || typeof data.field !== 'expected')\` 패턴 검토"
  echo "참고: G-4 (b) renderVerificationMetrics 사례 (dashboard.js:1772)"
  exit 0  # 게이트 아님 — 후보 출력만
fi
