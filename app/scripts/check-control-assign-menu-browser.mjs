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
let server, browser, page;
try {
  await writeFile(fixture, `
import { createSignal } from 'solid-js';
import { render } from 'solid-js/web';
import { ControlAssignMenu } from '/src/components/ControlAssignMenu';
import { VideoGroupInspector } from '/src/components/VideoGroupInspector';
import { installControlLearnCapture } from '/src/createControlLearnCapture';
import { controlMappingTargetsFromElement, controlMappingTargetsForMode } from '/src/controlMappingLearn';
import { audioTargetIsEligible } from '/src/audioControlMapping';
import { installUiLocalization } from '/src/uiLocalization';
import '/src/theme.css';
import '/src/styles.css';
const calls = [];
window.proof = { calls, windowEscapes: 0 };
window.addEventListener('keydown', (event) => { if (event.key === 'Escape') window.proof.windowEscapes++; });
const query = new URLSearchParams(location.search);
const [state, setState] = createSignal({ x: Number(query.get('x') ?? 300), y: Number(query.get('y') ?? 200), label: 'Layer 1 clip 2', eligible: true, audioEligible: false, audioTrigger: false, assignments: [
  { mapping: { id: 'cc' }, kind: 'MIDI', detail: 'CC 21 · ch 3', beat: false },
  { mapping: { id: 'beat' }, kind: 'Beat division', detail: '1 bar', beat: true },
] });
window.proof.set = (next) => setState({ ...state(), ...next });
const group = { id: 57, label: 'Group 57', composition_id: 1, layer_ids: [], enabled: true,
  opacity: 0.75, blend_mode: 'Normal', solo: false, bypassed: false, transform: null };
const effects = { catalog: { effect_chains: [], effect_presets: [], layer_groups: [group], transition_buses: [] },
  compositions: [], outputs: [], busy: false, onApplyCatalog: () => calls.push('ordinary-group-edit'),
  onImportIsf: () => {}, onApplyPreset: () => {}, onRemoveChain: () => {} };
window.proof.groupMenu = () => {
  const targets = controlMappingTargetsFromElement(document.querySelector('[data-video-group-opacity="57"]'));
  setState({ ...state(), label: targets[0].label, eligible: false,
    audioEligible: targets.length > 0 && targets.every(audioTargetIsEligible), audioTrigger: false, assignments: [] });
  return targets;
};
let groupMode = null, groupBusy = false;
window.proof.groupMode = value => { groupMode = value; };
installControlLearnCapture({
  mode: () => groupMode, busy: () => groupBusy, setBusy: value => { groupBusy = value; },
  targets: element => controlMappingTargetsForMode(groupMode ?? 'midi', controlMappingTargetsFromElement(element)),
  select: () => {}, message: message => calls.push({ message }),
  learn: async (mode, targets) => { calls.push({ groupLearn: mode, targets }); },
});
render(() => <div style="min-height:100vh"><button id="outside" type="button">Outside</button><VideoGroupInspector group={group} effects={effects} /><ControlAssignMenu x={state().x} y={state().y} label={state().label} beatEligible={state().eligible} audioEligible={state().audioEligible} audioTrigger={state().audioTrigger} assignments={state().assignments}
  onAudio={(settings) => calls.push({ audio: settings })} onLearn={() => calls.push('learn')} onBeat={(division) => calls.push('beat:' + division)} onRemove={(assignment) => calls.push('remove:' + assignment.mapping.id)} onDismiss={() => calls.push('dismiss')} /></div>, document.body);
let locale = 'en';
const localization = installUiLocalization(document.body, () => locale);
window.proof.locale = (value) => { locale = value; localization.refresh(); };
`);
  const reservation = createTcpServer();
  await new Promise((done, reject) => { reservation.once('error', reject); reservation.listen(Number(process.env.SYNDOCAL_VITE_PORT ?? 0), '127.0.0.1', done); });
  const reservedPort = reservation.address().port;
  await new Promise((done, reject) => reservation.close((error) => error ? reject(error) : done()));
  server = await createServer({
    root, optimizeDeps: { entries: [fixture] }, server: { host: '127.0.0.1', port: reservedPort, strictPort: true }, logLevel: 'error',
    plugins: [{ name: 'assign-menu-proof', configureServer(s) { s.middlewares.use((request, response, next) => {
      if (!request.url?.startsWith('/assign-proof')) return next();
      response.setHeader('Content-Type', 'text/html');
      response.end('<html><head><link rel="icon" href="data:,"></head><body><script type="module" src="/' + name + '"></script></body></html>');
    }); } }],
  });
  await server.listen();
  const port = server.httpServer.address().port;
  const executablePath = ['C:/Program Files/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe'].find(existsSync);
  browser = await chromium.launch({ headless: true, args: process.env.SYNDOCAL_CDP_PORT ? ['--remote-debugging-port=' + process.env.SYNDOCAL_CDP_PORT] : [], ...(executablePath ? { executablePath } : {}) });
  page = await browser.newPage();
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
    // Audio form: bounded settings, native field keys, inner scrolling, EN/JA.
    await page.evaluate(() => window.proof.set({ eligible: false, audioEligible: true }));
    await menu.locator('[data-control-assign-audio]').click();
    await menu.getByLabel('Audio source', { exact: true }).selectOption('Low');
    const gain = menu.getByLabel('Audio gain', { exact: true });
    await gain.fill('2');
    await gain.focus();
    await page.keyboard.press('ArrowUp');
    assert.equal(await gain.evaluate(element => document.activeElement === element), true, 'field arrows do not navigate menu');
    await gain.fill('2');
    await menu.getByLabel('Attack (ms)', { exact: true }).fill('30');
    await menu.getByLabel('Release (ms)', { exact: true }).fill('200');
    await menu.getByLabel('Output minimum', { exact: true }).fill('0.2');
    await menu.getByLabel('Output maximum', { exact: true }).fill('0.8');
    await gain.fill('17');
    assert.equal(await menu.locator('[data-control-assign-audio-save]').isDisabled(), true);
    await gain.fill('2');
    await menu.locator('[data-control-assign-audio-save]').click();
    assert.deepEqual(await calls(), [{ audio: { source: 'Low', gain: 2, attack_ms: 30, release_ms: 200, threshold: 0.5, mode: 'Continuous', low: 0.2, high: 0.8 } }]);
    const expanded = await menu.boundingBox();
    assert.ok(expanded.y + expanded.height <= height + 0.5, 'expanded audio form fits viewport');
    assert.ok(await menu.locator('.controlAssignAudioFields input').first().evaluate(element => parseFloat(getComputedStyle(element).fontSize) >= 14));
    await page.evaluate(() => window.proof.locale('ja'));
    assert.ok((await menu.innerText()).includes('音声ソース'));
    assert.ok((await menu.innerText()).includes('音声を割り当て'));
    await page.screenshot({ path: resolve(artifacts, 'assign-audio-ja-' + width + '.png') });
    await page.evaluate(() => window.proof.locale('en'));
    assert.ok((await menu.innerText()).includes('Audio source'));
    await page.screenshot({ path: resolve(artifacts, 'assign-audio-en-' + width + '.png') });
    await page.evaluate(() => window.proof.set({ audioTrigger: true }));
    await menu.getByLabel('Trigger mode', { exact: true }).selectOption('Onset');
    assert.equal(await menu.getByLabel('Audio gain', { exact: true }).count(), 0, 'onset pulse has no ignored gain control');
    await menu.locator('[data-control-assign-audio-save]').click();
    const onset = (await calls())[0].audio;
    assert.equal(onset.mode, 'Onset'); assert.equal(onset.source, 'Onset');
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
  // The production group range supplies one canonical target to all source paths.
  await page.setViewportSize({ width: 1280, height: 720 });
  await page.goto('http://127.0.0.1:' + port + '/assign-proof');
  await page.locator('[data-video-group-opacity="57"]').waitFor();
  const groupTargets = await page.evaluate(() => window.proof.groupMenu());
  assert.equal(groupTargets.length, 1);
  assert.equal(groupTargets[0].action, 'VideoGroupOpacity');
  assert.equal(groupTargets[0].video_group_id, 57);
  assert.equal(groupTargets[0].group_id ?? null, null);
  assert.equal(Object.hasOwn(groupTargets[0], 'audio_only'), false);
  const groupMenu = page.locator('[data-control-assign-menu]');
  assert.equal(await groupMenu.locator('[data-control-assign-learn]').isVisible(), true);
  assert.equal(await groupMenu.locator('[data-control-assign-audio]').isVisible(), true);
  assert.equal(await groupMenu.locator('[data-control-assign-beat]').count(), 0);
  assert.equal(await page.locator('[data-video-group-inspector] [data-control-map-unsupported]').count(), 0);
  for (const mode of ['midi', 'osc', 'dmx']) {
    await page.evaluate(value => window.proof.groupMode(value), mode);
    await page.locator('[data-video-group-opacity="57"]').dispatchEvent('pointerdown', { button: 0, bubbles: true });
    await page.waitForFunction(value => window.proof.calls.some(call => call?.groupLearn === value), mode);
    const learned = (await calls()).filter(call => call?.groupLearn);
    assert.equal(learned.length, 1);
    assert.equal(learned[0].groupLearn, mode);
    assert.equal(learned[0].targets.length, 1);
    assert.equal(learned[0].targets[0].action, 'VideoGroupOpacity');
    assert.equal(learned[0].targets[0].video_group_id, 57);
  }
  await page.evaluate(() => window.proof.groupMode(null));
  results.push({ canonicalGroup: 57, sources: ['MIDI', 'OSC', 'DMX', 'Audio'] });
  assert.deepEqual(errors, []);
  await writeFile(resolve(artifacts, 'report.json'), JSON.stringify({ passed: true, results, errors, physicalMidi: false, nativeWindow: false }, null, 2));
  console.log('PASS: Learn source menu at 1280x720 and 375x640; eight beat divisions, stable callbacks, keyboard, Escape/outside dismissal, viewport clamp, Audio settings/EN-JA/form keys, canonical group MIDI/OSC/DMX/Audio, 8-theme contrast; ' + artifacts);
} catch (error) {
  if (page && !page.isClosed()) {
    await page.screenshot({ path: resolve(artifacts, 'failure.png') }).catch(() => {});
    await writeFile(resolve(artifacts, 'failure.html'), await page.content()).catch(() => {});
  }
  throw error;
} finally {
  await browser?.close();
  await server?.close();
  await unlink(fixture).catch(() => {});
}
