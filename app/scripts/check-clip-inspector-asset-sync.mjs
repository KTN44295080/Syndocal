import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { existsSync } from 'node:fs';
import { homedir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { createServer } from 'vite';
// Regression gate: the Clip Inspector "Media asset" select must show the inspected slot's
// real asset (not the first option), re-sync on slot change, survive asset-list refreshes,
// keep an unsaved choice for the same slot, and never allow Assign without a deliberate change.
const require = createRequire(import.meta.url);
let chromium;
try { ({ chromium } = require('playwright')); } catch {
  ({ chromium } = require(process.env.PLAYWRIGHT_MODULE_PATH ?? path.join(homedir(), '.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright')));
}
const appRoot = fileURLToPath(new URL('../', import.meta.url));
const port = Number(process.env.CLIP_INSPECTOR_QA_PORT ?? 5203);
const chrome = [process.env.CHROME_PATH, path.join(homedir(), 'AppData/Local/Google/Chrome/Application/chrome.exe'), chromium.executablePath()].find(p => p && existsSync(p));
assert.ok(chrome, 'A browser executable is required');
const server = await createServer({ root: appRoot, server: { host: '127.0.0.1', port, strictPort: true }, plugins: [{
  name: 'clip-inspector-asset-sync-fixture', configureServer(server) {
    server.middlewares.use((req, res, next) => {
      if (req.url !== '/__clip-inspector/') return next();
      res.setHeader('Content-Type', 'text/html');
      res.end('<!doctype html><html><body><div id="fixture-root"></div><script type="module" src="/scripts/fixtures/clip-inspector-asset-sync.tsx"></script></body></html>');
    });
  },
}] });
let browser;
try {
  await server.listen();
  browser = await chromium.launch({ headless: true, executablePath: chrome });
  const page = await browser.newPage({ viewport: { width: 1280, height: 720 } });
  const errors = [];
  page.setDefaultTimeout(15000); page.setDefaultNavigationTimeout(15000);
  page.on('pageerror', e => errors.push(e.message));
  await page.goto(`http://127.0.0.1:${port}/__clip-inspector/`);
  await page.waitForFunction(() => Boolean(window.__clipInspectorFixture));
  const select = page.locator('[data-video-clip-slot-inspector] select').first();
  const assign = page.getByRole('button', { name: 'Assign', exact: true });
  const fx = (fn, ...args) => page.evaluate(([name, a]) => window.__clipInspectorFixture[name](...a), [fn, args]);
  const shown = async () => ({ value: await select.inputValue(), text: await select.evaluate(el => el.selectedOptions[0]?.textContent ?? null) });
  const settle = () => page.evaluate(() => new Promise(r => requestAnimationFrame(() => requestAnimationFrame(r))));
  await select.waitFor({ state: 'visible' });
  await settle();

  // 1. Opening on slot 5 (asset 6, the LAST option) shows that asset and cannot Assign yet.
  assert.deepEqual(await shown(), { value: '6', text: 'Pattern 06' }, 'opens on the slot asset, not the first option');
  assert.equal(await assign.isDisabled(), true, 'Assign is disabled while the draft equals the current asset');
  await assign.click({ force: true });
  assert.deepEqual(await fx('calls'), [], 'a disabled Assign never reaches the callback');

  // 2. Asset-list and layer refreshes (new object identities, same content) keep the real asset.
  await fx('refreshAssets'); await settle();
  assert.equal((await shown()).value, '6', 'asset list refresh does not fall back to the first option');
  await fx('refreshLayer'); await settle();
  assert.equal((await shown()).value, '6', 'authoritative layer refresh keeps the real asset');
  await fx('refreshAssets'); await fx('refreshLayer'); await settle();
  assert.equal((await shown()).value, '6', 'combined refresh keeps the real asset');

  // 3. Switching slots re-syncs to each slot's asset.
  await fx('setSlotId', 3); await settle();
  assert.deepEqual(await shown(), { value: '3', text: 'Pattern 03' }, 'switching to slot 3 shows asset 3');
  await fx('setSlotId', 1); await settle();
  assert.deepEqual(await shown(), { value: '1', text: 'h264_1080p30' }, 'switching to slot 1 shows asset 1');
  await fx('setSlotId', 5); await settle();
  assert.equal((await shown()).value, '6', 'switching back to slot 5 shows asset 6');

  // 4. An in-progress choice survives refreshes for the same slot and enables Assign.
  await select.selectOption('2'); await settle();
  assert.equal(await assign.isEnabled(), true, 'Assign enables after a deliberate change');
  await fx('refreshAssets'); await fx('refreshLayer'); await settle();
  assert.equal((await shown()).value, '2', 'unsaved choice survives refreshes for the same slot');
  assert.equal(await assign.isEnabled(), true);
  await select.selectOption('6'); await settle();
  assert.equal(await assign.isDisabled(), true, 'returning to the current asset disables Assign');
  await select.selectOption('2'); await settle();

  // 5. Switching slots discards the draft of the previous slot.
  await fx('setSlotId', 3); await settle();
  assert.equal((await shown()).value, '3', 'draft from slot 5 does not leak into slot 3');
  assert.equal(await assign.isDisabled(), true, 'Assign is disabled on the new slot until the operator changes it');
  await fx('setSlotId', 5); await settle();
  assert.equal((await shown()).value, '6', 'draft is discarded when leaving the slot');

  // 6. Assign applies exactly the chosen asset.
  await select.selectOption('4'); await settle();
  await assign.click();
  assert.deepEqual(await fx('calls'), [{ kind: 'assign', slotId: 5, assetId: 4 }], 'Assign sends the operator-chosen asset');
  await fx('setSlotAsset', 5, 4); await settle();
  assert.deepEqual(await shown(), { value: '4', text: 'Pattern 04' });
  assert.equal(await assign.isDisabled(), true, 'Assign disables once the authority matches the draft');

  // 7. An authoritative asset change from elsewhere re-syncs even over an unsaved draft.
  await select.selectOption('2'); await settle();
  await fx('setSlotAsset', 5, 5); await settle();
  assert.equal((await shown()).value, '5', 'external authoritative asset change wins over a stale draft');

  // 8. A slot asset missing from the library is shown explicitly, never silently replaced by the first option.
  await fx('setSlotAsset', 5, 99); await settle();
  assert.equal((await shown()).value, '99', 'missing asset keeps its own placeholder option');
  assert.equal(await assign.isDisabled(), true, 'Assign stays disabled on the missing-asset placeholder');
  assert.equal((await fx('calls')).filter(call => call.kind === 'assign').length, 1, 'no unintended Assign happened');
  assert.deepEqual(errors, [], 'no page errors');
  console.log('Clip Inspector asset sync check passed.');
} finally {
  await browser?.close();
  await server.close();
}
