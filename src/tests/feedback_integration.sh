#!/bin/bash
# 피드백 통합 테스트 — 소스 모드에서 실행
# 사전 조건: git init + initial commit 완료, claude CLI 설치
#
# 테스트 시나리오:
# 1. 단건 피드백 실행 + 이력 확인 + diff + undo
# 2. 3건 동시 피드백 (다른 요소)
# 3. 3건 동시 피드백 (같은 요소 → 충돌 테스트)

set -e
cd "$(dirname "$0")/.."

echo "========================================="
echo "  피드백 통합 테스트"
echo "========================================="

# git 상태 확인
echo ""
echo "[0] Git 상태 확인"
git log --oneline -3 || { echo "ERROR: git 커밋 없음. 먼저 git init + commit 하세요."; exit 1; }
echo "OK: git 레포 확인됨"

# claude CLI 확인
echo ""
echo "[0] Claude CLI 확인"
claude --version 2>/dev/null || { echo "ERROR: claude CLI를 찾을 수 없습니다."; exit 1; }
echo "OK: Claude CLI 확인됨"

# 테스트용 백업
echo ""
echo "[0] UI 백업"
cp ui/dashboard.js ui/dashboard.js.bak
cp ui/dashboard.css ui/dashboard.css.bak
cp ui/index.html ui/index.html.bak
echo "OK: 백업 완료"

# ── 테스트 1: 단건 피드백 ──
echo ""
echo "========================================="
echo "[1] 단건 피드백 테스트"
echo "========================================="

echo "[1a] 피드백 실행: h1 타이틀 색상 변경"
BEFORE_HASH=$(git rev-parse HEAD)
claude --print --output-format text --max-turns 5 \
  --prompt "file-pipeline Dashboard UI를 수정해주세요.

피드백: h1 타이틀의 색상을 var(--color-primary)로 변경해줘
대상 요소: h1
수정 대상 파일: $(pwd)/ui/ 디렉토리의 dashboard.css
규칙: CSS 변수만 사용
수정 후 반드시 git add + git commit 하세요. 커밋 메시지: \"feedback: h1 색상 변경\"" \
  2>&1 | tail -5

# 변경 확인
AFTER_HASH=$(git rev-parse HEAD)
if [ "$BEFORE_HASH" = "$AFTER_HASH" ]; then
  echo "WARNING: 커밋이 생성되지 않았습니다. 수동 커밋 시도..."
  git add ui/ && git commit -m "feedback: h1 색상 변경" 2>/dev/null || echo "변경 없음"
  AFTER_HASH=$(git rev-parse HEAD)
fi

echo "[1b] diff 확인"
git diff ${BEFORE_HASH}..${AFTER_HASH} --stat -- ui/
echo ""

echo "[1c] undo (revert) 테스트"
if [ "$BEFORE_HASH" != "$AFTER_HASH" ]; then
  git revert --no-edit $AFTER_HASH
  echo "OK: revert 성공"
  git log --oneline -3
else
  echo "SKIP: 변경 없어서 revert 불필요"
fi

# ── 테스트 2: 3건 동시 피드백 (다른 요소) ──
echo ""
echo "========================================="
echo "[2] 3건 동시 피드백 (다른 요소)"
echo "========================================="

BEFORE_HASH=$(git rev-parse HEAD)

# 3건 병렬 실행
echo "[2a] 3건 동시 실행..."

claude --print --output-format text --max-turns 5 \
  --prompt "dashboard.css에서 .card 클래스의 border-radius를 var(--radius)에서 12px로 변경하세요. git add ui/ && git commit -m 'feedback: card radius'" \
  2>&1 | tail -2 &
PID1=$!

claude --print --output-format text --max-turns 5 \
  --prompt "index.html에서 h1 태그의 텍스트를 'File Pipeline'으로 변경하세요. git add ui/ && git commit -m 'feedback: h1 text'" \
  2>&1 | tail -2 &
PID2=$!

claude --print --output-format text --max-turns 5 \
  --prompt "dashboard.css에서 body의 font-size를 15px로 변경하세요. git add ui/ && git commit -m 'feedback: body font'" \
  2>&1 | tail -2 &
PID3=$!

# 대기
echo "[2b] 완료 대기..."
wait $PID1; R1=$?
wait $PID2; R2=$?
wait $PID3; R3=$?

echo "[2c] 결과: PID1=$R1, PID2=$R2, PID3=$R3"
AFTER_HASH=$(git rev-parse HEAD)
echo "[2d] 커밋 이력:"
git log --oneline ${BEFORE_HASH}..HEAD

# ── 테스트 3: 3건 동시 피드백 (같은 요소 → 충돌) ──
echo ""
echo "========================================="
echo "[3] 3건 동시 피드백 (같은 요소 — 충돌 테스트)"
echo "========================================="

BEFORE_HASH=$(git rev-parse HEAD)

echo "[3a] 3건 동시 실행 (같은 .card 클래스 수정)..."

claude --print --output-format text --max-turns 5 \
  --prompt "dashboard.css에서 .card의 background를 var(--color-bg)로 변경하세요. git add ui/ && git commit -m 'feedback: card bg 1'" \
  2>&1 | tail -2 &
PID1=$!

claude --print --output-format text --max-turns 5 \
  --prompt "dashboard.css에서 .card의 background를 #1a1a2e로 변경하세요. git add ui/ && git commit -m 'feedback: card bg 2'" \
  2>&1 | tail -2 &
PID2=$!

claude --print --output-format text --max-turns 5 \
  --prompt "dashboard.css에서 .card의 background를 transparent로 변경하세요. git add ui/ && git commit -m 'feedback: card bg 3'" \
  2>&1 | tail -2 &
PID3=$!

echo "[3b] 완료 대기..."
wait $PID1; R1=$?
wait $PID2; R2=$?
wait $PID3; R3=$?

echo "[3c] 결과: PID1=$R1, PID2=$R2, PID3=$R3"
echo "[3d] 커밋 이력:"
git log --oneline ${BEFORE_HASH}..HEAD

# 충돌 확인
echo "[3e] 충돌 여부:"
git status | grep -i conflict && echo "⚠️ CONFLICT 감지됨!" || echo "✅ 충돌 없음"

# ── 정리 ──
echo ""
echo "========================================="
echo "테스트 완료"
echo "========================================="
echo ""
echo "UI 원본 복원하려면:"
echo "  cp ui/dashboard.js.bak ui/dashboard.js"
echo "  cp ui/dashboard.css.bak ui/dashboard.css"
echo "  cp ui/index.html.bak ui/index.html"
echo "  git add ui/ && git commit -m 'restore: UI 원본 복원'"
echo ""
echo "전체 이력:"
git log --oneline -10
