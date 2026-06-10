#!/usr/bin/env node
/**
 * dead_selector_scan_v2.js — JS AST 기반 정밀화 (#8, 2026-05-20)
 *
 * grep 기반 dead_selector_scan.sh의 한계 해소:
 * - 템플릿 보간 `getElementById(`row-${id}`)` 같은 동적 ID 검출 (정적 부분만 매칭하여 잠재 위험 보고)
 * - innerHTML 안에 정적 id="..." 패턴 정확 추출 (HTML 정의로 인정)
 * - createElement + .id 할당 패턴 자동 인식
 *
 * 사용:
 *   npx acorn --version   # 사전 설치 확인 (npx가 자동 다운로드)
 *   node spec/benchmarks/scripts/dead_selector_scan_v2.js
 *
 * 의존: acorn (npx로 자동 설치)
 * Exit: 0=PASS / 1=DEAD 발견 / 2=환경 오류
 */

const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

let acorn;
try {
  acorn = require('acorn');
} catch (e) {
  // 자동 설치 시도
  try {
    console.error('[setup] acorn 미설치, npx로 임시 설치 중...');
    execSync('npm install acorn --no-save --silent', {
      cwd: path.resolve(__dirname),
      stdio: 'pipe',
    });
    acorn = require(path.resolve(__dirname, 'node_modules/acorn'));
  } catch (err) {
    console.error('ERROR: acorn 설치 실패. 수동: npm install acorn');
    console.error(err.message);
    process.exit(2);
  }
}

// 스크립트 위치 기준 ROOT 추정 (spec/benchmarks/scripts/ → file-pipeline 루트)
const SCRIPT_DIR = __dirname;
const ROOT = path.resolve(SCRIPT_DIR, '../../..');
// src/ui 또는 ui 자동 감지
const UI_DIR = fs.existsSync(path.join(ROOT, 'src/ui')) ? path.join(ROOT, 'src/ui')
              : fs.existsSync(path.join(ROOT, 'ui')) ? path.join(ROOT, 'ui')
              : null;
if (!UI_DIR) {
  console.error('ERROR: ui/ 디렉토리 미존재 (src/ui 또는 ui)');
  process.exit(2);
}

const jsPath = path.join(UI_DIR, 'dashboard.js');
const htmlPath = path.join(UI_DIR, 'index.html');

if (!fs.existsSync(jsPath) || !fs.existsSync(htmlPath)) {
  console.error('ERROR: ui/dashboard.js 또는 ui/index.html 미존재');
  process.exit(2);
}

const jsSrc = fs.readFileSync(jsPath, 'utf8');
const htmlSrc = fs.readFileSync(htmlPath, 'utf8');

// 1. HTML에 정적 정의된 ID (id="xxx" / id='xxx')
const htmlIds = new Set();
const htmlIdPattern = /\bid=["']([^"']+)["']/g;
let m;
while ((m = htmlIdPattern.exec(htmlSrc)) !== null) {
  htmlIds.add(m[1]);
}

// 2. JS AST 파싱
let ast;
try {
  ast = acorn.parse(jsSrc, { ecmaVersion: 'latest', allowReturnOutsideFunction: true });
} catch (e) {
  console.error(`ERROR: dashboard.js 파싱 실패 — ${e.message}`);
  process.exit(2);
}

// 3. AST walker — getElementById('xxx') 호출 + innerHTML 안의 id="xxx" 추출 + createElement + .id 할당
const getElementByIdCalls = []; // {id: string|null, isDynamic: bool, line: number}
const dynamicIds = new Set();   // innerHTML 안에 정적으로 들어간 id
const createdIds = new Set();   // createElement + .id 할당

function walk(node, parent) {
  if (!node || typeof node !== 'object') return;

  // getElementById('xxx') 또는 getElementById(`${var}`) 등
  if (node.type === 'CallExpression'
      && node.callee.type === 'MemberExpression'
      && node.callee.property.name === 'getElementById'
      && node.arguments.length === 1) {
    const arg = node.arguments[0];
    if (arg.type === 'Literal' && typeof arg.value === 'string') {
      getElementByIdCalls.push({ id: arg.value, isDynamic: false, line: node.loc?.start.line || 0 });
    } else if (arg.type === 'TemplateLiteral') {
      // 정적 부분만 결합 (동적 부분은 ${EXPR}로 표시)
      const staticParts = arg.quasis.map(q => q.value.cooked).join('${EXPR}');
      getElementByIdCalls.push({ id: staticParts, isDynamic: arg.expressions.length > 0, line: node.loc?.start.line || 0 });
    } else {
      getElementByIdCalls.push({ id: '<computed>', isDynamic: true, line: node.loc?.start.line || 0 });
    }
  }

  // String literal / Template literal 안의 id="xxx" 검출 (innerHTML 패턴)
  if ((node.type === 'Literal' && typeof node.value === 'string')
      || (node.type === 'TemplateElement' && node.value && node.value.cooked)) {
    const raw = node.type === 'Literal' ? node.value : node.value.cooked;
    const re = /\bid=["']([^"'$]+)["']/g; // ${...} 보간 없는 정적 ID
    let mm;
    while ((mm = re.exec(raw)) !== null) {
      dynamicIds.add(mm[1]);
    }
  }

  // assignment: el.id = "xxx"
  if (node.type === 'AssignmentExpression'
      && node.operator === '='
      && node.left.type === 'MemberExpression'
      && node.left.property.name === 'id'
      && node.right.type === 'Literal'
      && typeof node.right.value === 'string') {
    createdIds.add(node.right.value);
  }

  // 재귀
  for (const key of Object.keys(node)) {
    const child = node[key];
    if (Array.isArray(child)) {
      child.forEach(c => walk(c, node));
    } else if (child && typeof child === 'object' && child.type) {
      walk(child, node);
    }
  }
}

walk(ast, null);

// 4. 분류
const allDefined = new Set([...htmlIds, ...dynamicIds, ...createdIds]);
const dead = [];
const dynamicSuspects = [];

const seenStaticIds = new Set();
for (const call of getElementByIdCalls) {
  if (call.isDynamic) {
    // 동적 ID: 정적 부분만 매칭 검사 (잠재 위험 보고)
    if (call.id && call.id !== '${EXPR}' && !call.id.includes('${EXPR}')) continue;
    if (call.id !== '<computed>') {
      dynamicSuspects.push({ id: call.id, line: call.line });
    }
    continue;
  }
  if (!call.id) continue;
  if (seenStaticIds.has(call.id)) continue;
  seenStaticIds.add(call.id);

  if (!allDefined.has(call.id)) {
    dead.push({ id: call.id, line: call.line });
  }
}

// 5. 출력
if (dead.length === 0) {
  console.log(`OK: getElementById 정적 ID ${seenStaticIds.size}개 모두 HTML/JS에 정의됨`);
  console.log(`    동적 ID 호출: ${dynamicSuspects.length}건 (정적 부분만 추적 가능)`);
  console.log(`    HTML 정의: ${htmlIds.size}개 / innerHTML 동적 생성: ${dynamicIds.size}개 / createElement: ${createdIds.size}개`);
  process.exit(0);
}

console.log(`DEAD selector 발견 (${dead.length}건):`);
for (const d of dead) {
  console.log(`  - ${d.id}  (dashboard.js:${d.line})`);
}
console.log('');
console.log('조치: lesson 47 §개선 + lesson 19 10단계 체크리스트 적용');
process.exit(1);
