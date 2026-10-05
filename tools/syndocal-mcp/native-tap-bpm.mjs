import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import path from 'node:path';

// Invoke the real App Tap callback via the opt-in acceptance-build receiver.
// Backend diagnostics only; no DOM click, physical output or second Tap implementation.
export async function nativeTapBpm(backend, checks, { evidencePath } = {}) {
  const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
  assert.equal(await backend.evaluate('typeof window.__syndocalQaTapTempo'), 'function',
    'Build the isolated QA artifact with VITE_SYNDOCAL_NATIVE_TAP_QA=1');
  const observe = () => backend.evaluate(`window.__TAURI_INTERNALS__.invoke('get_snapshot').then(wire=>({
    clock:wire.snapshot.clock,timelineRuntime:wire.timeline_runtime,
    readout:document.querySelector('.topbarBpmValue')?.textContent,
    footer:document.querySelector('.appStatusLine')?.textContent,
    headerHeight:document.querySelector('.topbar')?.getBoundingClientRect().height,
    frontend:window.__syndocalQaTapTempoState?.()}))`);
  const before = await observe();
  assert.equal(before.clock.bpm, 120, 'Isolated project starts at 120 BPM');
  const tapTarget = await backend.evaluate(`(() => {
    const button = document.querySelector('[data-topbar-tap]');
    if (!button) return null;
    const rect = button.getBoundingClientRect(), style = getComputedStyle(button);
    const hit = document.elementFromPoint(rect.x + rect.width / 2, rect.y + rect.height / 2);
    return { disabled: button.disabled, pointerEvents: style.pointerEvents,
      visible: style.visibility === 'visible' && style.display !== 'none' && rect.width > 0 && rect.height > 0,
      unobstructed: hit === button || button.contains(hit),
      bounds: { x: rect.x, y: rect.y, width: rect.width, height: rect.height } };
  })()`);
  assert.ok(tapTarget?.visible && tapTarget.unobstructed && !tapTarget.disabled
    && tapTarget.pointerEvents !== 'none', 'The visible header Tap must have an enabled, unobstructed target');
  const samples = [], start = Date.now() + 100;
  const check = { check: 'native-app-tap-changes-engine-and-header-bpm', passed: false, before, tapTarget, samples };
  checks.push(check);
  const capture = async name => {
    if (!evidencePath) return;
    assert.ok(path.isAbsolute(evidencePath));
    const screenshot = `${evidencePath}.${name}.png`;
    const result = await backend.send('Page.captureScreenshot', { format: 'png' });
    await fs.writeFile(screenshot, Buffer.from(result.data, 'base64'), { flag: 'wx' });
    (check.screenshots ??= {})[name] = screenshot;
  };
  for (let index = 0; index < 5; index++) {
    const target = start + index * 750;
    if (Date.now() < target) await pause(target - Date.now());
    const issuedAt = Date.now();
    await backend.evaluate('window.__syndocalQaTapTempo().then(()=>true)');
    const sample = { issuedAt, ...(await observe()) };
    samples.push(sample);
    assert.equal(Number(sample.readout), Math.round(sample.clock.bpm),
      `Tap ${index + 1} callback must finish with the native readout reflecting the engine: ${JSON.stringify(sample)}`);
    assert.match(sample.footer, index === 0 ? /Tap again to measure BPM/ : /Tapped BPM/,
      'The first Tap waits for an interval; later taps report the applied result');
    assert.equal(sample.headerHeight, 42, 'One-row header sizing is preserved');
    if (index === 0) await capture('first-tap');
  }
  const settled = await observe();
  assert.ok(settled.clock.bpm > 70 && settled.clock.bpm < 90, `750ms taps should approach 80 BPM: ${JSON.stringify(samples)}`);
  assert.equal(settled.clock.source, 'Tap');
  await capture('measured');
  await pause(2200);
  await backend.evaluate('window.__syndocalQaTapTempo().then(()=>true)');
  const reset = await observe();
  assert.equal(reset.clock.tap_count, 1, 'Long pause restarts the Tap interval history');
  assert.equal(reset.clock.bpm, settled.clock.bpm, 'The first tap after a long pause retains the existing tempo');
  assert.match(reset.footer, /Tap again to measure BPM/, 'A reset interval must not claim a newly applied BPM');
  Object.assign(check, { passed: true, settled, reset });
}
