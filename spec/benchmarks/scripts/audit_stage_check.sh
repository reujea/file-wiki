#!/usr/bin/env bash
# audit_stage_check.sh — audit.record stage 명명 규칙 검사 (메타 룰 24 후보 자동화)
#
# 사용:
#   bash spec/benchmarks/scripts/audit_stage_check.sh
#
# 검사 규칙 (메타 룰 24 후보 — Phase 95 lesson 54 §stage 명명 규칙):
#   {영역}.{도구명}[.{sub}]
#
# 허용 영역 (현재 누적):
#   - llm        (classify, verify_reprocess)
#   - mcp        (search, kg_neighbors, kg_paths, get_document, list_documents)
#   - tauri      (search)
#   - remote     (remote.{backend}.upload.{processed|origin})
#   - verify     (예약 — 향후 도메인 검증 단계 부착 시)
#   - service    (예약 — 향후 도메인 서비스 단계 부착 시)
#
# 출력:
#   - VIOLATION: 허용 영역 prefix가 아닌 stage 명명
#   - 0건이면 exit 0 (게이트 통과)
#   - 1건+이면 exit 1 + 신규 영역 추가 검토 권고
#
# 관련: lesson 54 §메타 룰 24 후보 / META.md 메타 룰 24 후보 / Phase 95 stage 정형화

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
if [ -d "$ROOT/src/crates" ]; then SRC_DIR="$ROOT/src"
elif [ -d "$ROOT/crates" ]; then SRC_DIR="$ROOT"
else echo "ERROR: crates/ 미존재 (탐색: $ROOT/src/crates, $ROOT/crates)" >&2; exit 2
fi

# 허용 prefix (정규식 OR)
ALLOWED='^(llm|mcp|tauri|remote|verify|service)\.'

# audit.record(...) 호출의 두 번째 인자(stage 문자열) 추출
STAGES=$(grep -rohnE 'audit\.record\([^,]+,\s*"[^"]+"' "$SRC_DIR/crates" "$SRC_DIR/modals" 2>/dev/null \
  | sed -E 's/.*audit\.record\([^,]+,\s*"([^"]+)".*/\1/' \
  | sort -u || true)

# format!()로 동적 생성된 stage는 prefix만 검사
# 예: format!("remote.{}.upload.processed", backend) → "remote." prefix 확인
DYNAMIC_STAGES=$(grep -rohnE 'audit\.record\([^,]+,\s*&format!\("[^"]+"' "$SRC_DIR/crates" "$SRC_DIR/modals" 2>/dev/null \
  | sed -E 's/.*format!\("([^{]+)\{.*/\1/' \
  | sed -E 's/\.$//' \
  | sort -u || true)

if [ -z "$STAGES$DYNAMIC_STAGES" ]; then
  echo "WARN: audit.record 호출 0건 — 검사 대상 없음"
  exit 0
fi

VIOLATIONS=""
echo "== 정적 stage 검사 =="
while IFS= read -r stage; do
  [ -z "$stage" ] && continue
  if echo "$stage" | grep -qE "$ALLOWED"; then
    echo "  ✓ $stage"
  else
    echo "  ✗ VIOLATION: $stage"
    VIOLATIONS="$VIOLATIONS\n  - $stage"
  fi
done <<< "$STAGES"

echo ""
echo "== 동적 stage prefix 검사 (format!) =="
while IFS= read -r stage; do
  [ -z "$stage" ] && continue
  if echo "$stage." | grep -qE "$ALLOWED"; then
    echo "  ✓ $stage.*"
  else
    echo "  ✗ VIOLATION (dynamic): $stage.*"
    VIOLATIONS="$VIOLATIONS\n  - $stage.* (dynamic)"
  fi
done <<< "$DYNAMIC_STAGES"

echo ""
if [ -n "$VIOLATIONS" ]; then
  echo "FAIL: stage 명명 규칙 위반 검출"
  echo -e "$VIOLATIONS"
  echo ""
  echo "조치: 본 스크립트의 ALLOWED 영역에 신규 prefix 추가 검토"
  echo "  또는 audit.record 호출 stage 명명 변경 ({영역}.{도구명})"
  exit 1
fi

echo "PASS: 모든 stage가 메타 룰 24 후보 명명 규칙 준수"
