#!/usr/bin/env bash
# single_source_check.sh — spec 본문의 단일 진실원 위임 패턴 검증
#                        (메타 룰 19 / 30 / lesson 49 sub-rule 1g 자동화 — 2026-06-05 신규)
#
# 사용:
#   bash spec/benchmarks/scripts/single_source_check.sh
#   bash spec/benchmarks/scripts/single_source_check.sh --verbose
#
# 검사 로직:
#   1. spec 본문 5개(architecture / domain-map / webapp-design / deprecated / scenarios)에서
#      "삭제/폐기/제거" 키워드 줄을 추출
#   2. 각 줄에 단일 진실원 위임 표시(`deprecated.md` / "단일 진실원" / "→ 참조" / "위임")가
#      포함되었는지 확인
#   3. 위임 표시 없는 경우 = 메타 룰 19 자기 위반 후보 → 출력
#
# 분류 (메타 룰 27):
#   게이트 — 명시적 위임 표기는 결정적 grep 가능 (false positive 최소). 단 시간축(Why)
#   기록은 위임 불필요하므로 단순 grep으로는 게이트 승격 어려움. 본 도구는 **점검** 분류.
#
# 출력:
#   - PASS: 위임 누락 0건 → exit 0
#   - WARN: 위임 의심 N건 → 후보 출력 + exit 0 (점검 도구, 게이트 아님)
#   - --verbose 시 상세 줄 출력
#
# 관련: lesson 49 옵션 A (시간축 Why / 상태축 What 분리)
#       메타 룰 19 (단일 진실원 위임 패턴, Phase 94 META 정식)
#       메타 룰 30 (spec 본문 phase별 즉시 갱신, 2026-06-04 META 정식)
#       META.md sub-rule 1g (spec 자기 위반)

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
SPEC_DIR="$ROOT/spec"
VERBOSE=0

for arg in "$@"; do
  case "$arg" in
    --verbose|-v) VERBOSE=1 ;;
    --help|-h)
      sed -n '2,30p' "$0"
      exit 0
      ;;
  esac
done

# 검사 대상 본문 (architecture-archive는 시간축 보존 영역 — 예외)
TARGETS=(
  "architecture.md"
  "domain-map.md"
  "webapp-design.md"
  "deprecated.md"
  "scenarios.md"
)

# 위임 표시 패턴 (어느 하나라도 포함되면 단일 진실원 위임으로 간주)
DELEGATION_PATTERNS='deprecated\.md|단일 진실원|→ 참조|위임|진실원 위임|단방향|상위 결정|→ `[a-z]'

# 위임 불필요 패턴 (정당한 결정 맥락 / 측정 결과 / 코드 그래프 표기)
SKIP_PATTERNS='dead_selector|dead_code|cargo clippy|invoke_handler 등록|^>|^---|^##|^|^### '

# "삭제/폐기/제거" 키워드 grep (action 항목만 — 결정 맥락 줄 제외 위해 줄 길이 휴리스틱)
echo "== Single Source 위임 패턴 검사 =="
echo "  spec dir: $SPEC_DIR"
echo "  targets:  ${#TARGETS[@]} 파일"
echo ""

TOTAL_WARN=0
ALL_WARN=""

for target in "${TARGETS[@]}"; do
  file="$SPEC_DIR/$target"
  [ -f "$file" ] || { echo "WARN: $target 부재 — 스킵"; continue; }

  # 1단계: action 키워드 줄 (`삭제/폐기/제거/dead`) — 단 표 줄과 헤더는 제외
  candidates=$(grep -nE "(삭제|폐기|제거).{0,80}(완료|예정|결정|함수|파일|어댑터|디렉토리|컴포넌트)" "$file" \
    | grep -vE "$SKIP_PATTERNS" || true)

  [ -z "$candidates" ] && continue

  # 2단계: 위임 표시 누락 후보 추출
  missing=""
  while IFS= read -r line; do
    [ -z "$line" ] && continue
    # 줄 안에 위임 표시 있으면 PASS
    if echo "$line" | grep -qE "$DELEGATION_PATTERNS"; then
      continue
    fi
    missing="$missing
$line"
  done <<< "$candidates"

  # 3단계: missing 누적
  missing=$(echo "$missing" | sed '/^$/d')
  if [ -n "$missing" ]; then
    count=$(echo "$missing" | wc -l)
    TOTAL_WARN=$((TOTAL_WARN + count))
    echo "  $target: $count 후보"
    if [ "$VERBOSE" -eq 1 ]; then
      echo "$missing" | sed 's/^/      /'
      echo ""
    fi
    ALL_WARN="$ALL_WARN
$target ($count):
$missing"
  fi
done

echo ""
if [ "$TOTAL_WARN" -eq 0 ]; then
  echo "PASS: 단일 진실원 위임 누락 의심 0건"
  exit 0
fi

echo "WARN: 단일 진실원 위임 누락 의심 $TOTAL_WARN건 (점검 도구 — 게이트 아님)"
echo ""
echo "후속 액션:"
echo "  1. --verbose 로 상세 라인 확인"
echo "  2. 각 줄이 결정 맥락 보존(시간축 Why)인지 판단"
echo "  3. 상태축(What) 정보면 deprecated.md 위임 표시 추가"
echo "  4. 정당한 보존이면 본 grep 룰의 SKIP_PATTERNS에 추가 검토"
echo ""
echo "lesson 49 옵션 A 패턴: 시간축 보존 + 상태축 deprecated.md 단일 진실원 위임"
exit 0  # 점검 도구 — exit 0 (게이트 아님, 메타 룰 27)
