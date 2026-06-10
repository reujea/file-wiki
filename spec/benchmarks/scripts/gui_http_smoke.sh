#!/usr/bin/env bash
# gui_http_smoke.sh — HTTP 모드 정형 검증 (browser-automation MCP 의존 없이)
#
# 사용:
#   bash spec/benchmarks/scripts/gui_http_smoke.sh
#   bash spec/benchmarks/scripts/gui_http_smoke.sh --port 8765
#
# 검증 항목:
#   1. python http.server 시작 + index.html 200 응답
#   2. dashboard.js 200 응답 + 라인 수 (회귀 감지)
#   3. dashboard.css 200 응답
#   4. body innerText에 "undefined" 문자열 0건 (G-4 (b) 회귀 방지)
#   5. 6탭 link 존재 (Documents/Processing/Todos/Topics/Pipeline/Settings) — Phase 107 Verification → Processing 흡수
#
# 의존: python3, curl, grep
# browser-automation MCP는 보다 정밀한 동적 검증에 사용 (본 스크립트는 정적 + 응답 검증만)

set -euo pipefail

PORT=8765
if [ "${1:-}" = "--port" ]; then PORT="${2:-8765}"; fi

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
if [ -d "$ROOT/src/ui" ]; then UI_DIR="$ROOT/src/ui"
elif [ -d "$ROOT/ui" ]; then UI_DIR="$ROOT/ui"
else echo "ERROR: ui/ 미존재" >&2; exit 2
fi

# 포트 충돌 확인
if curl -sf "http://localhost:$PORT/" -o /dev/null --connect-timeout 1 2>/dev/null; then
  echo "ERROR: 포트 $PORT 이미 사용 중" >&2
  exit 2
fi

# http.server 백그라운드 시작
cd "$UI_DIR"
python -m http.server "$PORT" >/dev/null 2>&1 &
SERVER_PID=$!
trap 'kill $SERVER_PID 2>/dev/null || true' EXIT

# 서버 ready 대기 (최대 10초)
for _ in {1..20}; do
  curl -sf "http://localhost:$PORT/" -o /dev/null --connect-timeout 1 2>/dev/null && break
  sleep 0.5
done

PASS=0
FAIL=0

# Test 1: index.html 200
if [ "$(curl -s -o /dev/null -w '%{http_code}' "http://localhost:$PORT/")" = "200" ]; then
  echo "PASS  1. index.html 200 응답"
  PASS=$((PASS+1))
else
  echo "FAIL  1. index.html 200 응답 실패"
  FAIL=$((FAIL+1))
fi

# Test 2: dashboard.js 200 + 라인 수
JS_STATUS=$(curl -s -o /dev/null -w '%{http_code}' "http://localhost:$PORT/dashboard.js")
JS_LINES=$(curl -s "http://localhost:$PORT/dashboard.js" | wc -l)
if [ "$JS_STATUS" = "200" ] && [ "$JS_LINES" -gt 0 ]; then
  echo "PASS  2. dashboard.js 200 응답 (${JS_LINES} 라인)"
  PASS=$((PASS+1))
else
  echo "FAIL  2. dashboard.js (status=$JS_STATUS, lines=$JS_LINES)"
  FAIL=$((FAIL+1))
fi

# Test 3: dashboard.css 200
if [ "$(curl -s -o /dev/null -w '%{http_code}' "http://localhost:$PORT/dashboard.css")" = "200" ]; then
  echo "PASS  3. dashboard.css 200 응답"
  PASS=$((PASS+1))
else
  echo "FAIL  3. dashboard.css 200 응답 실패"
  FAIL=$((FAIL+1))
fi

# Test 4: index.html 응답 본문에 "undefined" 0건 (정적 HTML이라 본문 검사는 한정적이지만 회귀 감지)
HTML_BODY=$(curl -s "http://localhost:$PORT/")
UNDEF_COUNT=$(echo "$HTML_BODY" | grep -c "undefined" || true)
if [ "$UNDEF_COUNT" = "0" ]; then
  echo "PASS  4. index.html 본문에 'undefined' 0건"
  PASS=$((PASS+1))
else
  echo "FAIL  4. index.html 본문에 'undefined' ${UNDEF_COUNT}건"
  FAIL=$((FAIL+1))
fi

# Test 5: 6탭 존재 확인 (Phase 107: Verification → Processing 흡수, "처리 현황" 단일 탭)
EXPECTED_TABS="documents pipeline processing todos settings topics"
MISSING=""
for tab in $EXPECTED_TABS; do
  echo "$HTML_BODY" | grep -qE "data-tab=[\"']${tab}[\"']" || MISSING="$MISSING $tab"
done
if [ -z "$MISSING" ]; then
  echo "PASS  5. 6탭 모두 존재"
  PASS=$((PASS+1))
else
  echo "FAIL  5. 누락 탭:$MISSING"
  FAIL=$((FAIL+1))
fi

echo ""
echo "결과: ${PASS} pass / ${FAIL} fail"
[ "$FAIL" -eq 0 ] || exit 1
