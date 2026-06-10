#!/usr/bin/env node
/**
 * dead_selector_scan_v3.js — v2 + CSS rule scanner (lesson 47 v3, Phase 98 2026-05-26)
 *
 * v2의 JS↔HTML ID 매칭에 더해 CSS 클래스 rule도 검사:
 * - dashboard.css의 .class-name { ... } rule 추출
 * - dashboard.js + index.html에서 사용 클래스 추출 (className / classList.add / class="...")
 * - 미사용 CSS rule 검출 (lesson 47 pb-subtab 5 rule 잔존 패턴 회귀 차단)
 *
 * 사용:
 *   node spec/benchmarks/scripts/dead_selector_scan_v3.js
 *
 * 의존: acorn (v2와 동일, npx로 자동 설치)
 * Exit: 0=PASS / 1=DEAD 발견 (ID 또는 CSS rule) / 2=환경 오류
 *
 * 한계:
 * - CSS 의사 클래스 / 자식 셀렉터 / 속성 셀렉터는 단순화하여 클래스명만 추출
 * - JS에서 동적 생성된 className(`${var}-active`)는 정적 부분만 매칭
 * - 외부 CDN / 부모 페이지에서 사용하는 클래스는 검출 못 함 (단일 SPA 가정)
 */

const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

let acorn;
try {
  acorn = require('acorn');
} catch (e) {
  try {
    console.error('[setup] acorn 미설치, npm으로 임시 설치 중...');
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

const SCRIPT_DIR = __dirname;
const ROOT = path.resolve(SCRIPT_DIR, '../../..');
const UI_DIR = fs.existsSync(path.join(ROOT, 'src/ui')) ? path.join(ROOT, 'src/ui')
              : fs.existsSync(path.join(ROOT, 'ui')) ? path.join(ROOT, 'ui')
              : null;
if (!UI_DIR) {
  console.error('ERROR: ui/ 디렉토리 미존재 (src/ui 또는 ui)');
  process.exit(2);
}

const jsPath = path.join(UI_DIR, 'dashboard.js');
const cssPath = path.join(UI_DIR, 'dashboard.css');
const htmlPath = path.join(UI_DIR, 'index.html');

for (const p of [jsPath, cssPath, htmlPath]) {
  if (!fs.existsSync(p)) {
    console.error(`ERROR: ${path.basename(p)} 미존재 in ${UI_DIR}`);
    process.exit(2);
  }
}

const jsSrc = fs.readFileSync(jsPath, 'utf8');
const cssSrc = fs.readFileSync(cssPath, 'utf8');
const htmlSrc = fs.readFileSync(htmlPath, 'utf8');

// ===== Part 1: ID 검사 (v2 동일 로직) =====

const htmlIds = new Set();
const htmlIdPattern = /\bid=["']([^"']+)["']/g;
let m;
while ((m = htmlIdPattern.exec(htmlSrc)) !== null) {
  htmlIds.add(m[1]);
}

let ast;
try {
  ast = acorn.parse(jsSrc, { ecmaVersion: 'latest', allowReturnOutsideFunction: true });
} catch (e) {
  console.error(`ERROR: dashboard.js 파싱 실패 — ${e.message}`);
  process.exit(2);
}

const getElementByIdCalls = [];
const dynamicIds = new Set();
const createdIds = new Set();

// ===== Part 2: CSS rule 추출 =====

// CSS rule selector 추출 — .class-name 패턴 (의사 클래스/조합 무시)
const cssRules = new Map(); // class-name -> [line]
const cssRulePattern = /\.([a-zA-Z_][\w-]*)(?:[:\s,.{>+~\[]|$)/g;
let cssLine = 1;
let cssOffset = 0;
let mm;
while ((mm = cssRulePattern.exec(cssSrc)) !== null) {
  const cls = mm[1];
  // 라인 번호 계산 (offset → line)
  while (cssOffset <= mm.index) {
    const next = cssSrc.indexOf('\n', cssOffset);
    if (next === -1 || next > mm.index) break;
    cssOffset = next + 1;
    cssLine++;
  }
  if (!cssRules.has(cls)) {
    cssRules.set(cls, []);
  }
  cssRules.get(cls).push(cssLine);
}

// ===== Part 3: 사용 클래스 수집 =====

const usedClasses = new Set();

// HTML class="..." 또는 class='...'
const htmlClassPattern = /\bclass=["']([^"']+)["']/g;
while ((m = htmlClassPattern.exec(htmlSrc)) !== null) {
  m[1].split(/\s+/).filter(Boolean).forEach(c => usedClasses.add(c));
}

// JS AST walker — className / classList.add / class="..." in templates
function walk(node, parent) {
  if (!node || typeof node !== 'object') return;

  // ID 검사 (v2 동일)
  if (node.type === 'CallExpression'
      && node.callee.type === 'MemberExpression'
      && node.callee.property.name === 'getElementById'
      && node.arguments.length === 1) {
    const arg = node.arguments[0];
    if (arg.type === 'Literal' && typeof arg.value === 'string') {
      getElementByIdCalls.push({ id: arg.value, isDynamic: false, line: node.loc?.start.line || 0 });
    } else if (arg.type === 'TemplateLiteral') {
      const staticParts = arg.quasis.map(q => q.value.cooked).join('${EXPR}');
      getElementByIdCalls.push({ id: staticParts, isDynamic: arg.expressions.length > 0, line: node.loc?.start.line || 0 });
    } else {
      getElementByIdCalls.push({ id: '<computed>', isDynamic: true, line: node.loc?.start.line || 0 });
    }
  }

  // String Literal 안의 id="xxx" / class="xxx"
  if (node.type === 'Literal' && typeof node.value === 'string') {
    let mm;
    const idRe = /\bid=["']([^"'$]+)["']/g;
    while ((mm = idRe.exec(node.value)) !== null) {
      dynamicIds.add(mm[1]);
    }
    const clsRe = /\bclass=["']([^"']*)["']/g;
    while ((mm = clsRe.exec(node.value)) !== null) {
      const cleaned = mm[1].replace(/\$\{[^}]*\}/g, ' ');
      cleaned.split(/\s+/).filter(t => t && /^[a-zA-Z_-][\w-]*$/.test(t))
        .forEach(c => usedClasses.add(c));
    }
  }

  // TemplateLiteral 전체 — quasi를 ${EXPR}로 결합 후 정규식 적용
  // (TemplateElement 개별 처리 시 class="..." 패턴이 ${} 경계에서 잘려 매칭 실패)
  if (node.type === 'TemplateLiteral') {
    const combined = node.quasis.map(q => q.value.cooked).join('${EXPR}');
    let mm;
    const idRe = /\bid=["']([^"']*)["']/g;
    while ((mm = idRe.exec(combined)) !== null) {
      const cleaned = mm[1].replace(/\$\{[^}]*\}/g, '');
      if (cleaned && /^[a-zA-Z_-][\w-]*$/.test(cleaned)) {
        dynamicIds.add(cleaned);
      }
    }
    const clsRe = /\bclass=["']([^"']*)["']/g;
    while ((mm = clsRe.exec(combined)) !== null) {
      const cleaned = mm[1].replace(/\$\{[^}]*\}/g, ' ');
      cleaned.split(/\s+/).filter(t => t && /^[a-zA-Z_-][\w-]*$/.test(t))
        .forEach(c => usedClasses.add(c));
    }
  }

  // el.className = "xxx" 또는 el.className = `xxx ${...}`
  if (node.type === 'AssignmentExpression'
      && node.operator === '='
      && node.left.type === 'MemberExpression'
      && node.left.property.name === 'className') {
    if (node.right.type === 'Literal' && typeof node.right.value === 'string') {
      node.right.value.split(/\s+/).filter(Boolean).forEach(c => usedClasses.add(c));
    } else if (node.right.type === 'TemplateLiteral') {
      node.right.quasis.forEach(q => {
        q.value.cooked.split(/\s+/).filter(Boolean).forEach(c => usedClasses.add(c));
      });
    }
  }

  // el.id = "xxx" (v2 동일)
  if (node.type === 'AssignmentExpression'
      && node.operator === '='
      && node.left.type === 'MemberExpression'
      && node.left.property.name === 'id'
      && node.right.type === 'Literal'
      && typeof node.right.value === 'string') {
    createdIds.add(node.right.value);
  }

  // classList.add('xxx', 'yyy') / classList.toggle / classList.remove
  if (node.type === 'CallExpression'
      && node.callee.type === 'MemberExpression'
      && node.callee.object?.type === 'MemberExpression'
      && node.callee.object.property?.name === 'classList'
      && ['add', 'toggle', 'remove', 'contains'].includes(node.callee.property.name)) {
    node.arguments.forEach(arg => {
      if (arg.type === 'Literal' && typeof arg.value === 'string') {
        usedClasses.add(arg.value);
      } else if (arg.type === 'TemplateLiteral') {
        arg.quasis.forEach(q => {
          q.value.cooked.split(/\s+/).filter(Boolean).forEach(c => usedClasses.add(c));
        });
      }
    });
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

// ===== Part 4: 분류 =====

// ID dead 검출 (v2 동일)
const allDefinedIds = new Set([...htmlIds, ...dynamicIds, ...createdIds]);
const deadIds = [];
const dynamicIdSuspects = [];
const seenStaticIds = new Set();
for (const call of getElementByIdCalls) {
  if (call.isDynamic) {
    if (call.id && call.id !== '${EXPR}' && !call.id.includes('${EXPR}')) continue;
    if (call.id !== '<computed>') {
      dynamicIdSuspects.push({ id: call.id, line: call.line });
    }
    continue;
  }
  if (!call.id) continue;
  if (seenStaticIds.has(call.id)) continue;
  seenStaticIds.add(call.id);
  if (!allDefinedIds.has(call.id)) {
    deadIds.push({ id: call.id, line: call.line });
  }
}

// CSS dead rule 검출 (Phase 98 신규)
const deadCssRules = [];
const builtinClasses = new Set([
  // CSS 의사 클래스/조합에서 분리된 가짜 양성 제외용 (필요 시 확장)
  'active', 'disabled', 'hover', 'focus', 'selected', 'open', 'visible', 'hidden',
]);

for (const [cls, lines] of cssRules.entries()) {
  if (builtinClasses.has(cls)) continue;
  if (!usedClasses.has(cls)) {
    deadCssRules.push({ cls, lines });
  }
}

// ===== Part 5: 출력 =====

let exitCode = 0;
console.log('== ID 매칭 검사 (v2 패턴) ==');
if (deadIds.length === 0) {
  console.log(`  OK: getElementById 정적 ID ${seenStaticIds.size}개 모두 정의됨`);
  console.log(`      동적 ID 호출 ${dynamicIdSuspects.length}건 (정적 부분만 추적)`);
  console.log(`      HTML 정의 ${htmlIds.size} / innerHTML 동적 ${dynamicIds.size} / createElement ${createdIds.size}`);
} else {
  console.log(`  FAIL: ${deadIds.length}건 DEAD ID 검출`);
  deadIds.forEach(d => console.log(`    L${d.line}: getElementById('${d.id}')`));
  exitCode = 1;
}

console.log('');
console.log('== CSS rule 검사 (Phase 98 신규) ==');
if (deadCssRules.length === 0) {
  console.log(`  OK: CSS rule ${cssRules.size}개 모두 사용 중 (used classes ${usedClasses.size}개 매칭)`);
} else {
  console.log(`  FAIL: ${deadCssRules.length}건 DEAD CSS rule 검출`);
  deadCssRules.slice(0, 30).forEach(d => {
    console.log(`    .${d.cls} (L${d.lines.join(', L')})`);
  });
  if (deadCssRules.length > 30) {
    console.log(`    ... 및 ${deadCssRules.length - 30}건 더`);
  }
  console.log('');
  console.log('  조치: lesson 47 pb-subtab 5 rule 잔존 패턴 — HTML/JS 변경 시 dashboard.css 동시 정리');
  console.log('  알려진 한계: 부모 셀렉터/속성 셀렉터/외부 CDN 사용은 검출 못 함');
  exitCode = 1;
}

process.exit(exitCode);
