#!/usr/bin/env bash
# Phase 91 A3: trace_id 단위로 audit_trace 행을 재구성하여 결정 흐름 출력.
#
# Usage:
#   bash replay_trace.sh <trace_id>            # settings.db 자동 탐색 (PIPELINE_BASE)
#   PIPELINE_BASE=/path/to/data replay_trace.sh <trace_id>
#
# 메타 룰 18 "추정 재검증 의무"의 인프라. lesson 46 G-1 같은 "외부 일시 요인" 추정을
# trace로 재현하여 root cause 확정.

set -euo pipefail

TRACE_ID="${1:-}"
if [[ -z "$TRACE_ID" ]]; then
    echo "Usage: $0 <trace_id>" >&2
    echo "Example: $0 18f3a1b2c4d5e6f0-00000001" >&2
    exit 1
fi

BASE="${PIPELINE_BASE:-$HOME/.file-pipeline}"
DB="$BASE/settings.db"

if [[ ! -f "$DB" ]]; then
    echo "settings.db not found at: $DB" >&2
    echo "Set PIPELINE_BASE to project data dir." >&2
    exit 1
fi

if ! command -v sqlite3 >/dev/null; then
    echo "sqlite3 not found in PATH" >&2
    exit 1
fi

echo "=== Audit trace: $TRACE_ID ==="
echo "DB: $DB"
echo

sqlite3 -separator $'\t' "$DB" <<SQL
.headers on
SELECT
    id,
    stage,
    COALESCE(inputs_hash, '-') AS inputs_hash,
    COALESCE(output_summary, '-') AS output_summary,
    COALESCE(applied_rule, '-') AS applied_rule,
    created_at
FROM audit_trace
WHERE trace_id = '$TRACE_ID'
ORDER BY id ASC;
SQL

N=$(sqlite3 "$DB" "SELECT COUNT(*) FROM audit_trace WHERE trace_id='$TRACE_ID';")
echo
echo "Total events: $N"
