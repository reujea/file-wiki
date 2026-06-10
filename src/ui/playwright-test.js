const { chromium } = require('playwright');

(async () => {
  const browser = await chromium.launch({ headless: true });
  const page = await browser.newPage();
  const results = [];

  function check(name, passed, detail) {
    results.push({ name, passed, detail: detail || '' });
    console.log(`${passed ? 'PASS' : 'FAIL'} ${name}${detail ? ' — ' + detail : ''}`);
  }

  try {
    // 1. 페이지 로드
    await page.goto('http://localhost:8765/test.html', { waitUntil: 'networkidle', timeout: 15000 });
    await page.waitForTimeout(2000); // dashboard.js 로드 대기
    check('페이지 로드', true);

    // 2. 타이틀 확인
    const title = await page.title();
    check('타이틀', title.includes('Dashboard') || title.includes('Test'), title);

    // 3. 메인 헤더
    const h1 = await page.textContent('h1');
    check('메인 헤더', h1 && h1.includes('Pipeline'), h1);

    // 4. 탭 존재 확인
    const tabs = await page.$$('[data-tab]');
    check('탭 수', tabs.length >= 5, `${tabs.length}개`);

    // 5. Stats 카드 (문서 수)
    const statsText = await page.textContent('.header-groups');
    check('Stats 카드', statsText !== null, statsText ? statsText.substring(0, 100) : 'null');

    // 6. Documents 탭 클릭
    const docsTab = await page.$('[data-tab="documents"]');
    if (docsTab) {
      await docsTab.click();
      await page.waitForTimeout(500);
      const docsContent = await page.textContent('#tab-documents');
      check('Documents 탭', docsContent !== null, docsContent ? docsContent.substring(0, 80) : 'null');
    } else {
      check('Documents 탭', false, '탭 없음');
    }

    // 7. Settings 탭 클릭
    const settingsTab = await page.$('[data-tab="settings"]');
    if (settingsTab) {
      await settingsTab.click();
      await page.waitForTimeout(500);
      const settingsContent = await page.textContent('#tab-settings');
      check('Settings 탭', settingsContent !== null && settingsContent.length > 10, settingsContent ? settingsContent.substring(0, 80) : 'null');
    } else {
      check('Settings 탭', false, '탭 없음');
    }

    // 8. Credentials 탭 클릭
    const credTab = await page.$('[data-tab="credentials"]');
    if (credTab) {
      await credTab.click();
      await page.waitForTimeout(500);
      const credContent = await page.textContent('#tab-credentials');
      check('Credentials 탭', credContent !== null, credContent ? credContent.substring(0, 80) : 'null');
    } else {
      check('Credentials 탭', false, '탭 없음');
    }

    // 9. Pipeline 탭 클릭
    const pipelineTab = await page.$('[data-tab="pipeline"]');
    if (pipelineTab) {
      await pipelineTab.click();
      await page.waitForTimeout(500);
      const pipeContent = await page.textContent('#tab-pipeline');
      check('Pipeline 탭', pipeContent !== null, pipeContent ? pipeContent.substring(0, 80) : 'null');
    } else {
      check('Pipeline 탭', false, '탭 없음');
    }

    // 10. Processing 탭
    const procTab = await page.$('[data-tab="processing"]');
    if (procTab) {
      await procTab.click();
      await page.waitForTimeout(500);
      check('Processing 탭', true);
    } else {
      check('Processing 탭', false, '탭 없음');
    }

    // 11. 콘솔 에러 수집
    const consoleErrors = [];
    page.on('console', msg => {
      if (msg.type() === 'error') consoleErrors.push(msg.text());
    });
    // 다시 로드해서 에러 확인
    await page.goto('http://localhost:8765/test.html', { waitUntil: 'networkidle', timeout: 15000 });
    await page.waitForTimeout(2000);
    check('콘솔 에러 없음', consoleErrors.length === 0, consoleErrors.join('; ').substring(0, 200));

    // 12. 스크린샷
    await page.screenshot({ path: 'test-screenshot.png', fullPage: true });
    check('스크린샷 저장', true, 'test-screenshot.png');

  } catch (e) {
    check('테스트 실행', false, e.message);
  }

  await browser.close();

  // 결과 요약
  const passed = results.filter(r => r.passed).length;
  const failed = results.filter(r => !r.passed).length;
  console.log(`\n=== GUI 테스트 결과: ${passed} PASS, ${failed} FAIL (총 ${results.length}) ===`);

  if (failed > 0) {
    console.log('\nFAILED:');
    results.filter(r => !r.passed).forEach(r => console.log(`  ${r.name}: ${r.detail}`));
  }

  process.exit(failed > 0 ? 1 : 0);
})();
