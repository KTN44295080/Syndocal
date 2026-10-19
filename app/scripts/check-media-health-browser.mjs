import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { existsSync } from 'node:fs';
import { mkdir, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createServer } from 'vite';

// Media health view and the relink review flow, with the real dialog component
// and scripted callbacks. Boundary: this proves the view, the folder-match
// review and the callbacks the dialog makes. The native pickers and the ticketed
// relink operation are covered by their own contracts, and no native window or
// real media file is involved.
const require = createRequire(import.meta.url);
let chromium;
try { ({ chromium } = require('playwright')); } catch {
  ({ chromium } = require(process.env.PLAYWRIGHT_MODULE_PATH ?? path.join(homedir(), '.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright')));
}
const appRoot = fileURLToPath(new URL('../', import.meta.url));
const evidence = path.resolve(process.env.MEDIA_HEALTH_QA_DIR ?? path.join(appRoot, '../target/qa/media-health/browser'));
const port = Number(process.env.SYNDOCAL_CHECK_PORT ?? 5320);
await mkdir(evidence, { recursive: true });
const chrome = [process.env.CHROME_PATH, path.join(homedir(), 'AppData/Local/Google/Chrome/Application/chrome.exe'),
  'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe', chromium.executablePath()].find(p => p && existsSync(p));
assert.ok(chrome, 'A browser executable is required');
const server = await createServer({ root: appRoot, cacheDir: path.join(evidence, 'vite-cache'), clearScreen: false,
  server: { host: '127.0.0.1', port, strictPort: true }, plugins: [{
    name: 'media-health-fixture', configureServer(instance) {
      instance.middlewares.use((req, res, next) => {
        if (!req.url?.startsWith('/__media-health/')) return next();
        res.setHeader('Content-Type', 'text/html');
        res.end('<!doctype html><html><body><div id="fixture-root"></div><script type="module" src="/scripts/fixtures/media-health.tsx"></script></body></html>');
      });
    },
  }] });

const exactFile = { path: 'E:\\restored\\sub\\intro.MP4', file_name: 'intro.MP4', byte_size: 1_048_576 };
const scan = { directory: 'E:\\restored', truncated: false, files: [
  exactFile,
  { path: 'E:\\restored\\loop.mov', file_name: 'loop.mov', byte_size: 5 },
  { path: 'E:\\restored\\a\\twins.mp4', file_name: 'twins.mp4', byte_size: 600 },
  { path: 'E:\\restored\\b\\twins.mp4', file_name: 'twins.mp4', byte_size: 600 },
  { path: 'E:\\restored\\unrelated.mp4', file_name: 'unrelated.mp4', byte_size: 1 },
] };

let browser; const results = [];
try {
  await server.listen();
  browser = await chromium.launch({ headless: true, executablePath: chrome });
  const cases = [
    { locale: 'en', width: 1920, height: 1080 },
    { locale: 'en', width: 1280, height: 720 },
    { locale: 'ja', width: 1280, height: 720 },
    { locale: 'en', width: 390, height: 780 },
  ];
  for (const { locale, width, height } of cases) {
    const ja = locale === 'ja';
    const tx = (en, jaText) => ja ? jaText : en;
    const label = `${locale}-${width}x${height}`;
    const page = await browser.newPage({ viewport: { width, height } }); const errors = [];
    page.setDefaultTimeout(15000); page.setDefaultNavigationTimeout(30000);
    page.on('pageerror', e => { errors.push(e.message); console.error(e.stack); });
    page.on('response', r => { if (r.status() >= 400 && !r.url().endsWith('/favicon.ico')) errors.push(r.status() + ' ' + r.url()); });
    await page.goto(`http://127.0.0.1:${port}/__media-health/?locale=${locale}`);
    await page.waitForFunction(() => Boolean(window.__mediaHealthFixture));
    const fx = (name, ...args) => page.evaluate(([n, a]) => window.__mediaHealthFixture[n](...a), [name, args]);
    const dialog = page.locator('[data-media-health-dialog]');
    await page.locator('[data-fixture-open]').click();
    await dialog.waitFor({ state: 'visible' });

    // Groups in operator-priority order, with counts, for every non-live asset.
    assert.deepEqual(await page.$$eval('[data-media-health-group]', els => els.map(e => e.dataset.mediaHealthGroup)),
      ['missing', 'unreadable', 'hash_mismatch', 'unverified', 'available'], `${label}: group order`);
    const chips = await page.$$eval('[data-media-health-chip]', els => Object.fromEntries(els.map(e => [e.dataset.mediaHealthChip, e.querySelector('strong').textContent])));
    assert.deepEqual(chips, { missing: '4', unreadable: '1', hash_mismatch: '1', unverified: '1', available: '1', live: '1' }, `${label}: chips`);
    assert.equal(await page.locator('[data-media-health-row]').count(), 8, `${label}: the camera asset is not listed as a file`);
    const headings = await page.$$eval('[data-media-health-group] > h3', els => els.map(e => e.textContent.replace(/\s+/g, ' ').trim()));
    assert.deepEqual(headings, ja
      ? ['欠損 (4)', '読み取り不能 (1)', 'ハッシュ不一致 (1)', '未検証 (1)', '利用可能 (1)']
      : ['Missing (4)', 'Unreadable (1)', 'Hash mismatch (1)', 'Not verified (1)', 'Available (1)'], `${label}: group headings`);

    // Path, size and last check per asset.
    const intro = page.locator('[data-media-health-row="1"]');
    assert.equal(await intro.locator('.mediaHealthPath').innerText(), 'D:\\old\\Intro.mp4', `${label}: path`);
    assert.equal(await intro.locator('[data-media-health-size]').innerText(), '1.0 MiB', `${label}: size`);
    assert.equal(await intro.locator('[data-media-health-checked]').innerText(), tx('Not checked', '未確認'), `${label}: never checked`);
    const good = page.locator('[data-media-health-row="8"]');
    assert.equal(await good.locator('[data-media-health-size]').innerText(), '5.0 GiB', `${label}: large size`);
    assert.match(await good.locator('[data-media-health-checked]').innerText(), ja ? /^確認時刻\s+\d{2}:\d{2}:\d{2}$/ : /^Checked\s+\d{2}:\d{2}:\d{2}$/, `${label}: last check time`);
    assert.equal(await page.locator('[data-media-health-row="7"] [data-media-health-size]').innerText(), '', `${label}: unknown size is blank`);
    assert.match(await page.locator('[data-media-health-row="5"] .mediaHealthDetail').innerText(), /Access is denied/, `${label}: unreadable reason`);
    assert.match(await page.locator('[data-media-health-row="6"] .mediaHealthDetail').innerText(), /Expected e{64}; found f{64}/, `${label}: both hashes`);

    // 44px targets and no horizontal overflow.
    const small = await dialog.locator('button, input[type=checkbox]').evaluateAll(els => els.filter(e => {
      const r = e.getBoundingClientRect(); return e.type === 'checkbox' ? false : r.height < 44 || r.width < 44;
    }).map(e => e.textContent));
    assert.deepEqual(small, [], `${label}: every button is at least 44px`);
    const overflow = await dialog.evaluate(e => e.scrollWidth - e.clientWidth);
    assert.ok(overflow <= 1, `${label}: dialog does not scroll horizontally (${overflow})`);
    await page.screenshot({ path: path.join(evidence, `health-${label}.png`) });

    // Every theme keeps the dialog's text readable (WCAG AA 4.5:1) on the surface it sits on.
    if (label === 'en-1280x720') {
      const pairs = [
        ['.mediaHealthHeader h2', 'dialog'], ['.mediaHealthNote', 'dialog'], ['.mediaHealthGroup > h3', 'dialog'],
        ['[data-media-health-row="1"] strong', '[data-media-health-row="1"]'], ['[data-media-health-row="1"] .mediaHealthPath', '[data-media-health-row="1"]'],
        ['[data-media-health-row="1"] [data-media-health-checked]', '[data-media-health-row="1"]'], ['[data-media-health-row="5"] .mediaHealthDetail', '[data-media-health-row="5"]'],
        ['[data-media-health-chip="missing"]', '[data-media-health-chip="missing"]'], ['[data-media-health-verify-all]', '[data-media-health-verify-all]'],
      ];
      for (const theme of ['cobalt', 'ember', 'forest', 'graphite', 'ice', 'neon', 'safe', 'tally']) {
        await page.evaluate(t => { document.documentElement.dataset.theme = t; }, theme);
        const failures = await page.evaluate(pairsIn => {
          const ctx = document.createElement('canvas').getContext('2d', { willReadFrequently: true });
          const rgba = css => { ctx.clearRect(0, 0, 1, 1); ctx.fillStyle = '#000'; ctx.fillStyle = css; ctx.fillRect(0, 0, 1, 1); return [...ctx.getImageData(0, 0, 1, 1).data]; };
          const lum = ([r, g, b]) => { const f = v => { v /= 255; return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4; }; return 0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b); };
          const dialog = document.querySelector('dialog[data-media-health-dialog]');
          const out = [];
          for (const [sel, bgSel] of pairsIn) {
            const el = dialog.querySelector(sel); const bgEl = bgSel === 'dialog' ? dialog : dialog.querySelector(bgSel);
            if (!el || !bgEl) { out.push(`${sel}: missing`); continue; }
            const fg = rgba(getComputedStyle(el).color); const bg = rgba(getComputedStyle(bgEl).backgroundColor);
            const [hi, lo] = [lum(fg), lum(bg)].sort((a, b) => b - a);
            const ratio = (hi + 0.05) / (lo + 0.05);
            if (ratio < 4.5) out.push(`${sel}: ${ratio.toFixed(2)}`);
          }
          return out;
        }, pairs);
        assert.deepEqual(failures, [], `${label}: ${theme} contrast`);
      }
      await page.evaluate(() => { document.documentElement.dataset.theme = 'tally'; });
    }

    // Verify All covers every file asset and never the live source.
    await page.locator('[data-media-health-verify-all]').click();
    assert.deepEqual((await fx('calls')).verify.at(-1), [1, 2, 3, 4, 5, 6, 7, 8], `${label}: Verify All ids`);
    await page.locator('[data-media-health-verify="2"]').click();
    assert.deepEqual((await fx('calls')).verify.at(-1), [2], `${label}: row Verify`);
    // The last-check time of what was just verified is visible.
    assert.match(await intro.locator('[data-media-health-checked]').innerText(), /\d{2}:\d{2}:\d{2}/, `${label}: verified row shows a time`);

    // Single-file relink: a relinked asset is re-verified, a canceled one is not.
    const verifyCount = async () => (await fx('calls')).verify.length;
    let before = await verifyCount();
    await page.locator('[data-media-health-relink="1"]').click();
    await page.waitForFunction(n => window.__mediaHealthFixture.calls().verify.length === n + 1, before);
    assert.deepEqual((await fx('calls')).relink, [1], `${label}: relink id`);
    assert.deepEqual((await fx('calls')).verify.at(-1), [1], `${label}: relinked asset re-verified`);
    await fx('setRelinkResult', false);
    before = await verifyCount();
    await page.locator('[data-media-health-relink="4"]').click();
    await page.waitForFunction(() => window.__mediaHealthFixture.calls().relink.length === 2);
    assert.equal(await verifyCount(), before, `${label}: a canceled relink does not verify`);
    await fx('setRelinkResult', true);

    // Folder relink: canceled picker leaves no review.
    await fx('setScan', null);
    await page.locator('[data-media-health-relink-folder]').click();
    await page.waitForFunction(() => window.__mediaHealthFixture.calls().scan === 1);
    assert.equal(await page.locator('[data-media-relink-plan]').count(), 0, `${label}: canceled picker shows no plan`);

    // Folder relink: review, deselect, cancel, then apply.
    await fx('setScan', scan);
    await page.locator('[data-media-health-relink-folder]').click();
    const plan = page.locator('[data-media-relink-plan]');
    await plan.waitFor({ state: 'visible' });
    assert.equal(await plan.locator('.mediaHealthPath').first().innerText(), 'E:\\restored', `${label}: folder shown`);
    assert.deepEqual(await plan.locator('[data-media-relink-proposal]').evaluateAll(els => els.map(e => [e.dataset.mediaRelinkProposal, e.dataset.confidence])),
      [['1', 'exact'], ['2', 'name_only']], `${label}: exact and name-only proposals`);
    assert.equal(await plan.locator('[data-media-relink-proposal="1"] [data-confidence-badge]').innerText(), tx('Same name and size', '名前とサイズが一致'), `${label}: exact badge`);
    assert.equal(await plan.locator('[data-media-relink-proposal="2"] [data-confidence-badge]').innerText(), tx('Same name, size differs', '名前は一致・サイズが異なる'), `${label}: name-only badge`);
    assert.equal(await plan.locator('[data-media-relink-proposal="1"] .mediaHealthPath').nth(1).innerText(), exactFile.path, `${label}: replacement path shown`);
    assert.equal(await plan.locator('[data-media-relink-ambiguous] summary span').first().innerText(), '1', `${label}: ambiguous count`);
    assert.equal(await plan.locator('[data-media-relink-unmatched] summary span').first().innerText(), '1', `${label}: unmatched count (Lost)`);
    assert.deepEqual(await plan.locator('[data-media-relink-proposal]').evaluateAll(els => els.map(e => [e.dataset.mediaRelinkProposal, e.querySelector('input').checked])),
      [['1', true], ['2', false]], `${label}: only same-name-and-size matches start selected`);
    assert.equal(await plan.locator('[data-media-relink-truncated]').count(), 0, `${label}: no truncation notice`);
    await page.screenshot({ path: path.join(evidence, `plan-${label}.png`) });

    assert.equal(await plan.locator('[data-media-relink-apply]').innerText(), `${tx('Apply selected', '選択を適用')} (1)`, `${label}: count starts at the exact matches`);
    await plan.locator('[data-media-relink-proposal="2"] input').check();
    assert.equal(await plan.locator('[data-media-relink-apply]').innerText(), `${tx('Apply selected', '選択を適用')} (2)`, `${label}: count follows selection`);
    await plan.locator('[data-media-relink-proposal="2"] input').uncheck();
    await plan.locator('[data-media-relink-proposal="1"] input').uncheck();
    assert.equal(await plan.locator('[data-media-relink-apply]').isDisabled(), true, `${label}: nothing selected disables Apply`);
    await plan.locator('[data-media-relink-cancel]').click();
    assert.equal(await page.locator('[data-media-relink-plan]').count(), 0, `${label}: Cancel closes the review`);
    assert.equal((await fx('calls')).apply.length, 0, `${label}: Cancel applies nothing`);

    await page.locator('[data-media-health-relink-folder]').click();
    await plan.waitFor({ state: 'visible' });
    await fx('setApply', { relinked: [1], differentContent: [{ assetId: 2, label: 'Loop' }], failed: [{ assetId: 4, label: 'Lost', message: 'Fixture failure' }], skipped: [] });
    before = await verifyCount();
    await plan.locator('[data-media-relink-apply]').click();
    const report = page.locator('[data-media-health-report]');
    await report.waitFor({ state: 'visible' });
    assert.deepEqual((await fx('calls')).apply, [[{ assetId: 1, label: 'Intro', previousPath: 'D:\\old\\Intro.mp4', replacementPath: exactFile.path }]], `${label}: only selected proposals are applied`);
    assert.equal(await page.locator('[data-media-relink-plan]').count(), 0, `${label}: review closes after Apply`);
    const reportText = await report.innerText();
    assert.ok(reportText.includes(tx('Folder relink: 1 relinked, 1 have different content, 1 failed.', 'フォルダー再リンク: 1件 再リンク、1件 内容が異なる、1件 失敗。')), `${label}: report summary: ${reportText}`);
    assert.ok(reportText.includes('Loop') && reportText.includes('Fixture failure'), `${label}: report names the skipped and failed assets`);
    assert.ok(reportText.includes(tx('Relink them one at a time', '1件ずつ再リンク')), `${label}: different content is deferred to a confirmed single relink`);
    await page.waitForFunction(n => window.__mediaHealthFixture.calls().verify.length === n + 1, before);
    assert.deepEqual((await fx('calls')).verify.at(-1), [1], `${label}: relinked assets are re-verified`);

    // A review is void once the library it was made from changes (for example a replaced project).
    await fx('setScan', scan);
    await page.locator('[data-media-health-relink-folder]').click();
    await plan.waitFor({ state: 'visible' });
    await fx('replaceAssetPath', 1, 'D:\\other\\Intro.mp4');
    await plan.waitFor({ state: 'detached' });
    assert.equal((await fx('calls')).apply.length, 1, `${label}: a stale review applies nothing`);
    await fx('replaceAssetPath', 1, 'D:\\old\\Intro.mp4');

    // Closing the dialog discards an open review and the last report.
    await page.locator('[data-media-health-relink-folder]').click();
    await plan.waitFor({ state: 'visible' });
    await page.locator('[data-media-health-close]').click();
    await dialog.waitFor({ state: 'hidden' });
    await page.locator('[data-fixture-open]').click();
    await dialog.waitFor({ state: 'visible' });
    assert.equal(await page.locator('[data-media-relink-plan], [data-media-health-report]').count(), 0, `${label}: reopening starts clean`);

    // A scan above the bound warns that matches may be missing.
    await fx('setScan', { ...scan, truncated: true });
    await page.locator('[data-media-health-relink-folder]').click();
    await plan.waitFor({ state: 'visible' });
    assert.equal(await plan.locator('[data-media-relink-truncated]').count(), 1, `${label}: truncation notice`);
    await plan.locator('[data-media-relink-cancel]').click();

    // Another media operation running: new ones wait.
    await fx('setBusy', true);
    assert.equal(await page.locator('[data-media-health-busy]').count(), 1, `${label}: busy note`);
    for (const selector of ['[data-media-health-verify-all]', '[data-media-health-relink-folder]', '[data-media-health-relink="1"]', '[data-media-health-verify="1"]']) {
      assert.equal(await page.locator(selector).isDisabled(), true, `${label}: ${selector} waits while busy`);
    }
    await fx('setBusy', false);
    assert.equal(await page.locator('[data-media-health-busy]').count(), 0);

    // Without folder callbacks the folder action is not offered.
    await fx('setFolderEnabled', false);
    assert.equal(await page.locator('[data-media-health-relink-folder]').count(), 0, `${label}: folder relink hidden without callbacks`);
    await fx('setFolderEnabled', true);

    // Escape closes the modal and returns focus to the opener.
    await page.keyboard.press('Escape');
    await dialog.waitFor({ state: 'hidden' });
    assert.equal(await page.evaluate(() => document.activeElement?.hasAttribute('data-fixture-open')), true, `${label}: focus returns to the opener`);
    assert.deepEqual(errors, [], `${label}: no page errors`);
    results.push({ case: label, status: 'pass' });
    await page.close();
  }
  console.log(JSON.stringify({ status: 'pass', results, boundary: 'real health dialog with scripted callbacks; not native picker or real-file evidence' }, null, 2));
} finally {
  await browser?.close(); await server.close();
  await writeFile(path.join(evidence, 'result.json'), JSON.stringify({ status: results.length === 4 ? 'pass' : 'fail', results, cleanup: 'owned browser and Vite closed' }, null, 2));
}
