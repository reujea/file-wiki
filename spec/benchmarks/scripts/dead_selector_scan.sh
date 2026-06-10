#!/usr/bin/env bash
# dead_selector_scan.sh — JS getElementById ↔ HTML id 매칭 검사 (lesson 47)
#
# 사용:
#   bash spec/benchmarks/scripts/dead_selector_scan.sh
#
# 출력:
#   - DEAD ID: js에 getElementById('xxx')는 있지만 HTML id="xxx" 없음 (dead 식별)
#   - 0건이면 exit 0 (게이트 통과), 1건+이면 exit 1
#
# 관련: lesson 47 §개선 / META.md 메타 룰 1 12번째 단계 / G-4 (a) 진단 발견 사례
#
# 알려진 한계:
#   - 동적 ID (템플릿 보간 `${var}`)는 검사 불가 — 정적 문자열 ID만 검출
#   - querySelector로 클래스/속성 선택자는 별도 (본 스크립트는 ID 한정)

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
# src 저장소 보정: $ROOT/src/ui 우선, 없으면 $ROOT/ui
if [ -d "$ROOT/src/ui" ]; then UI_DIR="$ROOT/src/ui"
elif [ -d "$ROOT/ui" ]; then UI_DIR="$ROOT/ui"
else echo "ERROR: ui/ 미존재 (탐색: $ROOT/src/ui, $ROOT/ui)" >&2; exit 2
fi

# JS getElementById('xxx') 호출의 ID 추출 (작은따옴표 / 큰따옴표 모두)
JS_IDS=$(grep -ohE "getElementById\(['\"][^'\"]+['\"]\)" "$UI_DIR"/dashboard.js \
  | sed -E "s/getElementById\(['\"]([^'\"]+)['\"]\)/\1/" \
  | sort -u)

DEAD=()
for id in $JS_IDS; do
  # 1. 템플릿 보간 ID 스킵 (예: ${id}, ${nodeId})
  [[ "$id" == *'${'* ]] && continue

  # 2. HTML에 id="xxx" 또는 id='xxx' 존재 확인 (정적 정의)
  if grep -qE "id=[\"']${id}[\"']" "$UI_DIR"/index.html; then
    continue
  fi

  # 3. dashboard.js 내부에서 동적으로 생성하는 ID 스킵 (innerHTML 또는 Modal.open으로 같은 ID를 정의)
  #    예: `id="modules-apply-critical"` 가 dashboard.js의 innerHTML 템플릿 안에 있으면 동적 생성
  if grep -qE "id=[\\\\]?[\"']${id}[\\\\]?[\"']" "$UI_DIR"/dashboard.js; then
    continue
  fi

  # 4. createElement + .id 할당 패턴 스킵 (dynamic fallback)
  #    예: settings-no-results — 검색 결과 0건 시 동적 createElement
  if grep -qE "\.id\s*=\s*[\"']${id}[\"']" "$UI_DIR"/dashboard.js; then
    continue
  fi

  DEAD+=("$id")
done

if [ ${#DEAD[@]} -eq 0 ]; then
  echo "OK: 모든 getElementById ID가 HTML에 존재 ($(echo "$JS_IDS" | wc -l)개 검증)"
  exit 0
else
  echo "DEAD selector 발견 (${#DEAD[@]}건):"
  for id in "${DEAD[@]}"; do
    echo "  - $id  (dashboard.js의 getElementById('$id') 호출 있음, index.html에 매칭 ID 없음)"
  done
  echo ""
  echo "조치: lesson 47 §개선 + lesson 19 10단계 체크리스트 적용"
  exit 1
fi
