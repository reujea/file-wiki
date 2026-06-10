#!/usr/bin/env bash
# data_flow_trace.sh — frontend → invoke → commands.rs → service → DB 6단계 grep 추적
#
# 사용:
#   bash spec/benchmarks/scripts/data_flow_trace.sh <action_or_cmd>
#   예: bash spec/benchmarks/scripts/data_flow_trace.sh search
#       bash spec/benchmarks/scripts/data_flow_trace.sh save_config
#       bash spec/benchmarks/scripts/data_flow_trace.sh pii-add
#
# 검증 단계 (lesson 46 "잘한 것" 3번 정형화):
#   1. ui/index.html data-action="<arg>" 위치
#   2. ui/dashboard.js data-action 핸들러 (handlePBAction or action===)
#   3. ui/dashboard.js API.<X>() 호출 정의
#   4. modals/app/src/commands.rs Tauri command 함수
#   5. crates/core/src/service.rs 또는 adapters에서 호출
#   6. settings.db / .local-store.json / file 영속화
#
# 회귀 게이트 아님 — 각 단계 매칭/누락 출력

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
if [ -d "$ROOT/src/ui" ]; then SRC="$ROOT/src"
elif [ -d "$ROOT/ui" ]; then SRC="$ROOT"
else echo "ERROR: src/ 미존재" >&2; exit 2
fi

ARG="${1:-}"
[ -n "$ARG" ] || { echo "사용: $0 <action_or_cmd>" >&2; exit 2; }

# Stage 1: data-action 정의 위치
echo "── Stage 1: data-action=\"$ARG\" 정의 ──"
grep -nE "data-action=[\"']${ARG}[\"']" "$SRC"/ui/index.html "$SRC"/ui/dashboard.js 2>/dev/null | head -10 || echo "  (없음)"

# Stage 2: 핸들러 (handlePBAction case 또는 action === '<arg>')
echo ""
echo "── Stage 2: action 핸들러 (dashboard.js) ──"
grep -nE "(case [\"']${ARG}[\"']|action === [\"']${ARG}[\"']|action == [\"']${ARG}[\"'])" "$SRC"/ui/dashboard.js | head -10 || echo "  (없음)"

# Stage 3: API.<X>() 정의 — kebab-case → camelCase 추정
CAMEL=$(echo "$ARG" | awk -F'[-_]' '{result=$1; for(i=2;i<=NF;i++){result=result toupper(substr($i,1,1)) substr($i,2)}; print result}')
echo ""
echo "── Stage 3: API.${CAMEL}() 또는 invoke('${ARG}') 호출 ──"
grep -nE "(API\.${CAMEL}\(|invoke\([\"']${ARG}[\"']|call\([\"']${ARG}[\"'])" "$SRC"/ui/dashboard.js | head -10 || echo "  (없음 — kebab/snake/camel 변환 확인 필요)"

# Stage 4: Tauri command 함수 (snake_case 또는 ARG 그대로)
SNAKE=$(echo "$ARG" | tr '-' '_')
echo ""
echo "── Stage 4: Tauri command (modals/app/src/commands.rs) ──"
grep -nE "(fn ${SNAKE}\(|#\[tauri::command\][^{]*${SNAKE})" "$SRC"/modals/app/src/commands.rs 2>/dev/null | head -10 || echo "  (없음 — ARG=${SNAKE})"

# Stage 5: service / adapters 호출
echo ""
echo "── Stage 5: core/service 또는 adapters 호출 ──"
grep -rln "${SNAKE}" "$SRC"/crates/core/src "$SRC"/crates/adapters/src "$SRC"/crates/shared/src 2>/dev/null | head -10 || echo "  (없음)"

# Stage 6: 영속화 (settings.db 테이블 / .local-store.json / file)
echo ""
echo "── Stage 6: 영속화 흔적 (settings.db 테이블 / 파일 I/O) ──"
grep -rnE "(${SNAKE}|${ARG})" "$SRC"/crates/shared/src/settings_db.rs 2>/dev/null | head -5 || true
echo "  (수동: settings.db 테이블명 또는 .local-store.json 필드 확인)"

echo ""
echo "── 추적 완료 ── (lesson 46 \"잘한 것\" 3번 정형화)"
