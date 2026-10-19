// The Learn-mode source menu in a real browser: choices, keyboard, dismissal,
// viewport fit and contrast in every theme. Never native or physical MIDI.
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { dirname, resolve } from 'node:path';
import { homedir } from 'node:os';
import { fileURLToPath } from 'node:url';
import { mkdir, writeFile, unlink } from 'node:fs/promises';
import { existsSync } from 'node:fs';
import { createServer } from 'vite';
import { createServer as createTcpServer } from 'node:net';

const require = createRequire(import.meta.url);
let chromium;
try { ({ chromium } = require('playwright')); } catch { ({ chromium } = require(resolve(homedir(), '.cache/codex-runtimes/codex-primary-runtime/dependencies/node/node_modules/playwright'))); }
const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const name = '.assign-menu-proof-' + process.pid + '.tsx';
const fixture = resolve(root, name);
const artifacts = resolve(process.env.SYNDOCAL_ASSIGN_MENU_ARTIFACT_DIR ?? resolve(root, '../target/qa/assign-menu'));
await mkdir(artifacts, { recursive: true });
let server, browser;
try {
  await writeFile(fixture, `
import { createSignal } from 'solid-js';
import { render } from 'solid-js/web';
import { ControlAssignMenu } from '/src/components/ControlAssignMenu';
import '/src/theme.css';
import '/src/styles.css';
const calls = [];
window.proof = { calls, windowEscapes: 0 };
window.addEventListener('keydown', (event) => { if (event.key === 'Escape') window.proof.windowEscapes++; });
const query = new URLSearchParams(location.search);
const [state, setState] = createSignal({ x: Number(query.get('x') ?? 300), y: Number(query.get('y') ?? 200), eligible: true, assignments: [
  { mapping: { id: 'cc' }, kind: 'MIDI', detail: 'CC 21 · ch 3', beat: false },
  { mapping: { id: 'beat' }, kind: 'Beat division', detail: '1 bar', beat: true },
] });
window.proof.set = (next) => setState({ ...state(), ...next });
render(() => <div style="min-height:100vh"><button id="outside" type="button">Outside</button><ControlAssignMenu x={state().x} y={state().y} label="Layer 1 clip 2" beatEligible={state().eligible} assignments={state().assignments}
  onLearn={() => calls.push('learn')} onBeat={(division) => calls.push('beat:' + division)} onRemove={(assignment) => calls.push('remove:' + assignment.mapping.id)} onDismiss={() => calls.push('dismiss')} /></div>, document.body);
`);
  const reservation = createTcpServer();
  await new Promise((done, reject) => { reservation.once('error', reject); reservation.listen(0, '127.0.0.1', done); });
  const reservedPort = reservation.address().port;
  await new Promise((done, reject) => reservation.close((error) => error ? reject(error) : done()));
  server = await createServer({
    root, server: { host: '127.0.0.1', port: reservedPort, strictPort: true }, logLevel: 'error',
    plugins: [{ name: 'assign-menu-proof', configureServer(s) { s.middlewares.use((request, response, next) => {
      if (!request.url?.startsWith('/assign-proof')) return next();
      response.setHeader('Content-Type', 'text/html');
      response.end('<html><head><link rel="icon" href="data:,"></head><body><script type="module" src="/' + name + '"></script></body></html>');
    }); } }],
  });
  await server.listen();
  const port = server.httpServer.address().port;
  const executablePath = ['C:/Program Files/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe'].find(existsSync);
  browser = await chromium.launch({ headless: true, ...(executablePath ? { executablePath } : {}) });
  const page = await browser.newPage();
  const errors = [];
  page.on('pageerror', (error) => errors.push(String(error)));
  page.on('console', (message) => { if (message.type() === 'error') errors.push(message.text()); });
  const results = [];
  const calls = () => page.evaluate(() => window.proof.calls.splice(0));
  const luminance = ([r, g, b]) => [r, g, b].map((v) => { const c = v / 255; return c <= 0.03928 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4; }).reduce((sum, c, i) => sum + c * [0.2126, 0.7152, 0.0722][i], 0);
  const contrast = (a, b) => { const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x); return (hi + 0.05) / (lo + 0.05); };
  const rgb = (value) => value.match(/\d+(\.\d+)?/g).slice(0, 3).map(Number);

  for (const [width, height] of [[1280, 720], [375, 640]]) {
    await page.setViewportSize({ width, height });
    await page.goto('http://127.0.0.1:' + port + '/assign-proof');
    const menu = page.locator('[data-control-assign-menu]');
    await menu.waitFor();
    // Trigger menu: all eight divisions, in order, with the existing assignments.
    const divisions = await menu.locator('[data-control-assign-beat]').allTextContents();
    assert.deepEqual(divisions, ['1/4 beat', '1/2 beat', '1 beat', '2 beats', '1 bar', '2 bars', '4 bars', '8 bars']);
    assert.equal(await menu.locator('[data-control-assign-row]').count(), 2);
    assert.equal(await menu.locator('[data-control-assign-row="beat"]').innerText().then((text) => text.includes('1 bar')), true);
    for (const button of await menu.locator('button').all()) {
      const box = await button.boundingBox();
      assert.ok(box.height >= 32 && box.width >= 44, 'hit target ' + JSON.stringify(box) + ' at ' + width);
    }
    // Choices call back with the table index and stable row identity.
    await menu.locator('[data-control-assign-beat="4"]').click();
    await menu.locator('[data-control-assign-learn]').click();
    await menu.locator('[data-control-assign-row="beat"] [data-control-assign-remove]').click();
    assert.deepEqual(await calls(), ['beat:4', 'learn', 'remove:beat']);
    // The menu stays inside the viewport wherever it opens.
    for (const [x, y] of [[width - 4, height - 4], [0, 0], [Math.round(width / 2), height - 20]]) {
      await page.goto('http://127.0.0.1:' + port + '/assign-proof?x=' + x + '&y=' + y);
      await menu.waitFor();
      const bounds = await menu.boundingBox();
      assert.ok(bounds.x >= 0 && bounds.y >= 0 && bounds.x + bounds.width <= width + 0.5 && bounds.y + bounds.height <= height + 0.5, 'menu opened at ' + x + ',' + y + ' fits ' + width + 'x' + height + ' ' + JSON.stringify(bounds));
    }
    await page.goto('http://127.0.0.1:' + port + '/assign-proof?x=' + (width - 4) + '&y=' + (height - 4));
    await menu.waitFor();
    // Keyboard: the first choice owns focus, arrows walk the choices, Escape closes only the menu.
    assert.equal(await page.evaluate(() => document.activeElement?.hasAttribute('data-control-assign-learn')), true);
    await page.keyboard.press('ArrowDown');
    assert.equal(await page.evaluate(() => document.activeElement?.getAttribute('data-control-assign-beat')), '0');
    await page.keyboard.press('ArrowUp');
    assert.equal(await page.evaluate(() => document.activeElement?.hasAttribute('data-control-assign-learn')), true);
    await page.keyboard.press('Escape');
    assert.deepEqual(await calls(), ['dismiss']);
    assert.equal(await page.evaluate(() => window.proof.windowEscapes), 0, 'Escape must not also leave Learn');
    await page.locator('#outside').click();
    assert.deepEqual(await calls(), ['dismiss'], 'a press outside closes the menu');
    // A continuous control only learns: no beat choices, an explanation instead.
    await page.evaluate(() => window.proof.set({ eligible: false }));
    assert.equal(await menu.locator('[data-control-assign-beat]').count(), 0);
    assert.ok((await menu.innerText()).includes('Beat divisions fire triggers only'));
    await page.evaluate(() => window.proof.set({ eligible: true }));
    await page.screenshot({ path: resolve(artifacts, 'assign-menu-' + width + '.png') });
    // Legible in every theme.
    const themes = ['tally', 'graphite', 'ice', 'cobalt', 'forest', 'ember', 'neon', 'safe'];
    const contrasts = {};
    for (const theme of themes) {
      await page.evaluate((id) => document.documentElement.setAttribute('data-theme', id), theme);
      const colors = await page.evaluate(() => {
        const menuElement = document.querySelector('[data-control-assign-menu]');
        const style = getComputedStyle(menuElement);
        const note = getComputedStyle(menuElement.querySelector('.controlAssignMenuNote, .controlAssignMenuKicker'));
        const button = getComputedStyle(menuElement.querySelector('button'));
        return { menuBackground: style.backgroundColor, text: style.color, dim: note.color, buttonBackground: button.backgroundColor, buttonText: button.color };
      });
      const ratios = { text: contrast(rgb(colors.text), rgb(colors.menuBackground)), dim: contrast(rgb(colors.dim), rgb(colors.menuBackground)), button: contrast(rgb(colors.buttonText), rgb(colors.buttonBackground)) };
      for (const [part, ratio] of Object.entries(ratios)) assert.ok(ratio >= 4.5, theme + ' ' + part + ' contrast ' + ratio.toFixed(2) + ' ' + JSON.stringify(colors));
      contrasts[theme] = Object.fromEntries(Object.entries(ratios).map(([part, ratio]) => [part, Number(ratio.toFixed(2))]));
    }
    await page.evaluate(() => document.documentElement.removeAttribute('data-theme'));
    results.push({ width, height, divisions: divisions.length, contrasts });
  }
  assert.deepEqual(errors, []);
  await writeFile(resolve(artifacts, 'report.json'), JSON.stringify({ passed: true, results, errors, physicalMidi: false, nativeWindow: false }, null, 2));
  console.log('PASS: Learn source menu at 1280x720 and 375x640; eight beat divisions, stable callbacks, keyboard, Escape/outside dismissal, viewport clamp, 8-theme contrast; ' + artifacts);
} finally {
  await browser?.close();
  await server?.close();
  await unlink(fixture).catch(() => {});
}
