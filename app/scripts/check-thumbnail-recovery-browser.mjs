import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { existsSync } from 'node:fs';
import { mkdir, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createServer } from 'vite';
const require = createRequire(import.meta.url);
let chromium;
try { ({ chromium } = require('playwright')); } catch {
  ({ chromium } = require(process.env.PLAYWRIGHT_MODULE_PATH ?? path.join(homedir(), '.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright')));
}
const appRoot = fileURLToPath(new URL('../', import.meta.url));
const evidence = path.resolve(process.env.THUMBNAIL_QA_DIR ?? path.join(appRoot, '../target/qa/thumbnail-recovery-20260908/browser'));
await mkdir(evidence, { recursive: true });
const chrome = [process.env.CHROME_PATH, path.join(homedir(), 'AppData/Local/Google/Chrome/Application/chrome.exe'), chromium.executablePath()].find(p => p && existsSync(p));
assert.ok(chrome, 'A browser executable is required');
const server = await createServer({ root: appRoot, server: { host: '127.0.0.1', port: 5199, strictPort: true }, plugins: [{
  name: 'thumbnail-recovery-fixture', configureServer(server) {
    server.middlewares.use((req, res, next) => {
      if (req.url !== '/__thumbnail-recovery/') return next();
      res.setHeader('Content-Type', 'text/html');
      res.end('<!doctype html><html><body><div id="fixture-root"></div><script type="module" src="/scripts/fixtures/thumbnail-recovery.tsx"></script></body></html>');
    });
  },
}] });
let browser; const results = [];
try {
  await server.listen();
  browser = await chromium.launch({ headless: true, executablePath: chrome });
  for (const viewport of [{ width: 1280, height: 720 }, { width: 1920, height: 1080 }]) {
    const page = await browser.newPage({ viewport }); const errors = [];
    page.setDefaultTimeout(15000); page.setDefaultNavigationTimeout(15000);
    page.on('pageerror', e => { errors.push(e.message); console.error(e.stack); });
    await page.goto('http://127.0.0.1:5199/__thumbnail-recovery/');
    await page.waitForFunction(() => Boolean(window.__thumbnailRecoveryFixture));
    const layer = page.locator('[data-vj-thumbnail-request]');
    const asset = page.locator('[data-media-library-thumbnail-request]');
    await layer.waitFor({ state: 'visible' }); await asset.waitFor({ state: 'visible' });
    assert.deepEqual(await page.evaluate(() => window.__thumbnailRecoveryFixture.counts()), { layer: 0, asset: 0 });
    await page.evaluate(() => window.__thumbnailRecoveryFixture.setNativeAvailable(false));
    assert.equal(await layer.isDisabled(), true); assert.equal(await asset.isDisabled(), true);
    assert.equal(await layer.getAttribute('title'), 'Desktop required');
    await page.evaluate(() => document.querySelector('[data-vj-thumbnail-request]').click());
    assert.deepEqual(await page.evaluate(() => window.__thumbnailRecoveryFixture.counts()), { layer: 0, asset: 0 });
    await page.evaluate(() => window.__thumbnailRecoveryFixture.setNativeAvailable(true));
    await layer.click();
    assert.equal(await layer.isDisabled(), true); assert.equal(await asset.isDisabled(), true);
    assert.equal(await layer.getAttribute('aria-busy'), 'true');
    assert.equal(await layer.innerText(), 'Loading Thumbnails');
    await page.evaluate(() => { for (let i = 0; i < 20; i++) document.querySelector('[data-vj-thumbnail-request]').click(); });
    assert.deepEqual(await page.evaluate(() => window.__thumbnailRecoveryFixture.counts()), { layer: 1, asset: 1 });
    await page.evaluate(() => window.__thumbnailRecoveryFixture.reset());
    await page.waitForFunction(() => JSON.stringify(window.__thumbnailRecoveryFixture.aborts()) === JSON.stringify({ layer: 1, asset: 1 }));
    await page.evaluate(() => window.__thumbnailRecoveryFixture.authorize());
    await page.waitForFunction(() => JSON.stringify(window.__thumbnailRecoveryFixture.counts()) === JSON.stringify({ layer: 2, asset: 2 }));
    assert.deepEqual(await page.evaluate(() => window.__thumbnailRecoveryFixture.aborts()), { layer: 1, asset: 1 });
    await page.evaluate(() => window.__thumbnailRecoveryFixture.settle('asset', 1, false));
    await page.waitForFunction(() => document.querySelector('[data-media-library-thumbnail-request]')?.textContent === 'Retry Thumbnails');
    assert.equal(await asset.isEnabled(), true); assert.equal(await layer.isDisabled(), true);
    await asset.focus(); await page.keyboard.press('Enter');
    await page.waitForFunction(() => window.__thumbnailRecoveryFixture.counts().asset === 3);
    assert.deepEqual(await page.evaluate(() => window.__thumbnailRecoveryFixture.counts()), { layer: 2, asset: 3 });
    await page.evaluate(() => { window.__thumbnailRecoveryFixture.settle('asset', 2, true); window.__thumbnailRecoveryFixture.settle('layer', 1, false); });
    await page.waitForFunction(() => document.querySelector('[data-vj-thumbnail-request]')?.textContent === 'Retry Thumbnails');
    assert.equal(await asset.count(), 0);
    const bounds = await layer.boundingBox();
    assert.ok(bounds && bounds.width >= 44 && bounds.height >= 24 && bounds.x >= 0 && bounds.x + bounds.width <= viewport.width);
    await page.screenshot({ path: path.join(evidence, `retry-${viewport.width}.png`) });
    await layer.click();
    await page.waitForFunction(() => window.__thumbnailRecoveryFixture.counts().layer === 3);
    assert.deepEqual(await page.evaluate(() => window.__thumbnailRecoveryFixture.counts()), { layer: 3, asset: 3 });
    await page.evaluate(() => window.__thumbnailRecoveryFixture.settle('layer', 2, true));
    await layer.waitFor({ state: 'detached' }); assert.equal(await asset.count(), 0);
    await page.evaluate(() => window.__thumbnailRecoveryFixture.reset());
    await layer.waitFor({ state: 'visible' }); await asset.waitFor({ state: 'visible' });
    assert.equal(await layer.innerText(), 'Load Thumbnails');
    assert.deepEqual(await page.evaluate(() => window.__thumbnailRecoveryFixture.counts()), { layer: 3, asset: 3 });
    await page.evaluate(() => window.__thumbnailRecoveryFixture.useLiveCatalog());
    await asset.waitFor({ state: 'detached' });
    await page.evaluate(() => { window.__thumbnailRecoveryFixture.setNativeAvailable(false); window.__thumbnailRecoveryFixture.authorize(); });
    await layer.waitFor({ state: 'detached' });
    assert.deepEqual(await page.evaluate(() => window.__thumbnailRecoveryFixture.counts()), { layer: 3, asset: 3 });
    assert.deepEqual(errors, []);
    results.push({ viewport, status: 'pass', retryButton: bounds,
      checks: ['no unauthorized reads', 'disabled repeat click', 'reset aborts active readers', 'post-reset reauthorization', 'independent lanes', 'keyboard asset retry', 'layer retry', 'successful cache reuse', 'live-only catalog', 'no native capability: disabled load and no dead retry'] });
    await page.close();
  }
  console.log(JSON.stringify({ status: 'pass', results, boundary: 'real components with deferred readers; not native/device acceptance' }, null, 2));
} finally {
  await browser?.close(); await server.close();
  await writeFile(path.join(evidence, 'result.json'), JSON.stringify({ status: results.length === 2 ? 'pass' : 'fail', results, cleanup: 'owned browser and Vite closed' }, null, 2));
}
