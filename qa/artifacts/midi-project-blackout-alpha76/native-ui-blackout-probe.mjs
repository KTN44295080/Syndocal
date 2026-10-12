import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { spawn, execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { createHash } from 'node:crypto';
import ts from '../../../app/node_modules/typescript/lib/typescript.js';
import { openNativeBackendSession } from '../../../tools/syndocal-mcp/native-backend-session.mjs';
import { parseOptions, readDescriptor } from '../../../tools/syndocal-mcp/server.mjs';

const exec = promisify(execFile);
const args = process.argv.slice(2);
const domainFlag = args.indexOf('--domain');
const domain = domainFlag >= 0 ? args.splice(domainFlag, 2)[1] : 'both';
assert.ok(['both', 'lighting', 'video'].includes(domain), 'Domain must be both, lighting, or video');
const take = name => {
  const index = args.indexOf(name);
  assert.ok(index >= 0 && index + 1 < args.length, `Required: ${name}`);
  return args.splice(index, 2)[1];
};
const executable = take('--expected-executable');
const evidence = take('--evidence');
const screenshot = take('--screenshot');
const cdpPort = Number(take('--cdp-port'));
assert.equal(args.length, 0);
assert.equal(process.platform, 'win32');
assert.ok([executable, evidence, screenshot].every(path.isAbsolute));
assert.ok(Number.isSafeInteger(cdpPort) && cdpPort >= 1024 && cdpPort <= 65535);
assert.notEqual(path.resolve(evidence).toLowerCase(), path.resolve(screenshot).toLowerCase());
const isolatedExe = path.join(os.tmpdir(), 'syndocal-native-acceptance-target', 'release', 'syndocal.exe');
assert.equal((await fs.realpath(executable)).toLowerCase(), (await fs.realpath(isolatedExe)).toLowerCase());
for (const target of [evidence, screenshot]) {
  await fs.access(path.dirname(target));
  assert.equal(await fs.stat(target).then(() => true, error => {
    if (error.code === 'ENOENT') return false;
    throw error;
  }), false, `${target} must be new`);
}
const oldDebugArgs = process.env.WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS ?? '';
assert.ok(!/--remote-debugging-(?:port|address)/i.test(oldDebugArgs));
await exec('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command',
  `if (@(Get-NetTCPConnection -LocalPort ${cdpPort} -State Listen -ErrorAction SilentlyContinue).Count -gt 0) { throw 'QA debugger port occupied' }`],
  { windowsHide: true, timeout: 10000 });

const profile = path.join(process.env.LOCALAPPDATA, 'jp.seraf.ktn.syndocal.qa.mcp-lifecycle');
const options = parseOptions(['--expected-executable', executable, '--descriptor', path.join(profile, 'agent-bridge-v1.json')]);
const normalDescriptorPath = path.join(process.env.LOCALAPPDATA, 'jp.seraf.ktn.syndocal', 'agent-bridge-v1.json');
const readNormalDescriptor = () => fs.readFile(normalDescriptorPath).catch(error => {
  if (error.code === 'ENOENT') return null;
  throw error;
});
const beforeNormal = await readNormalDescriptor();
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const child = spawn(executable, [], {
  cwd: path.dirname(executable), windowsHide: true, stdio: 'ignore',
  env: { ...process.env, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `${oldDebugArgs} --remote-debugging-address=127.0.0.1 --remote-debugging-port=${cdpPort}`.trim() },
});
let backend;
let failure;
let proof;
let spawnFailure = false;
child.once('error', () => { spawnFailure = true; });
try {
  let ready = false;
  const until = Date.now() + 60000;
  while (Date.now() < until) {
    assert.ok(!spawnFailure && child.exitCode === null, 'QA app exited before readiness');
    try {
      const descriptor = await readDescriptor(options);
      assert.equal(descriptor.processId, child.pid);
      const pages = await (await fetch(`http://127.0.0.1:${cdpPort}/json/list`, { signal: AbortSignal.timeout(1000) })).json();
      ready = pages.some(page => page.type === 'page' && /^https?:\/\/tauri\.localhost\//.test(page.url));
      if (ready) break;
    } catch { /* The exact QA process is still starting. */ }
    await pause(200);
  }
  assert.ok(ready, 'QA bridge and WebView did not become ready');
  backend = await openNativeBackendSession(options, cdpPort);
  assert.equal(backend.descriptor.processId, child.pid);
  assert.equal(await backend.evaluate('window.__TAURI_INTERNALS__.metadata.currentWindow.label'), 'main');
  await backend.invoke('plugin:window|maximize', { label: 'main' });
  const windowGate = await exec('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', `
Add-Type -TypeDefinition 'using System; using System.Runtime.InteropServices; public static class QaControlWindow { [DllImport("user32.dll")] public static extern bool IsZoomed(IntPtr hWnd); }'
$windows = @(Get-Process syndocal | Where-Object { $_.Path -eq $env:SYNDOCAL_QA_EXPECTED_EXE -and $_.MainWindowHandle -ne 0 })
if ($windows.Count -ne 1 -or $windows[0].Id -ne ${child.pid} -or !$windows[0].Responding -or $windows[0].MainWindowTitle -ne 'Syndocal QA - MCP Lifecycle' -or ![QaControlWindow]::IsZoomed($windows[0].MainWindowHandle)) { throw 'QA native window gate failed' }
'verified'
`], { windowsHide: true, timeout: 10000, env: { ...process.env, SYNDOCAL_QA_EXPECTED_EXE: await fs.realpath(executable) } });
  assert.equal(windowGate.stdout.trim(), 'verified');

  const click = async selector => backend.evaluate(`(() => {
    const element = document.querySelector(${JSON.stringify(selector)});
    if (!(element instanceof HTMLButtonElement)) return false;
    element.click();
    return true;
  })()`);

  assert.equal(await click('.appMenuButton'),true);
  assert.equal(await click('.appProjectMenu button[aria-keyshortcuts="Control+N Meta+N"]'),true);
  let menuCreated=false;
  for(let n=0;n<200;n++) {
    const s=(await backend.invoke('get_snapshot')).snapshot;
    if(s.fixtures.length===0&&!s.output.enabled&&s.dmx_outputs.every(r=>!r.enabled)&&!s.safety_blackout_engaged) {menuCreated=true;break;}
    await pause(50);
  }
  assert(menuCreated,'Project menu New must publish empty disarmed project');
  let navigated = false;
  for (let attempt = 0; attempt < 50 && !navigated; attempt++) {
    navigated = await click('[data-workspace-option="touch"]');
    if (!navigated) await pause(100);
  }
  assert.ok(navigated, 'Control workspace navigation was not ready');
  assert.equal(await click(`[data-control-domain="${domain}"]`), true);
  await pause(250);
  proof = domain === 'both' ? await backend.evaluate(`(() => {
    const layout = document.querySelector('.layoutTouch.touchDomainBoth');
    const desk = layout?.querySelector(':scope > .touchBothLivePanel');
    const tabs = layout?.querySelector(':scope > .touchControlDomainTabs');
    const rect = node => node?.getBoundingClientRect();
    const visible = node => Boolean(node && rect(node).width > 0 && rect(node).height > 0 && getComputedStyle(node).display !== 'none');
    const lower = ['lower-left', 'lower-right'].map(name => layout?.querySelector('[data-workspace-pane="' + name + '"]'));
    const controls = [...(desk?.querySelectorAll('button, input[type="range"]') ?? [])].filter(visible);
    return {
      viewport: [window.innerWidth, window.innerHeight],
      controlSelected: document.querySelector('[data-workspace-option="touch"]')?.getAttribute('aria-pressed') === 'true',
      bothSelected: document.querySelector('[data-control-domain="both"]')?.getAttribute('aria-pressed') === 'true',
      tabsLeft: Boolean(tabs && layout && rect(tabs).left <= rect(layout).left + 8),
      upperFullWidth: Boolean(desk && layout && Math.abs(rect(desk).width - rect(layout).width) <= 2),
      laneCount: desk?.querySelectorAll('.touchBothLane').length ?? 0,
      splitHidden: !visible(layout?.querySelector(':scope > .touchSurfacePanel')) && !visible(layout?.querySelector(':scope > .touchVideoPanel')),
      lowerSideBySide: lower.every(visible) && lower.every(node => rect(node).top >= rect(desk).bottom - 2) && rect(lower[0]).right <= rect(lower[1]).left + 2,
      controlCount: controls.length,
      minimumControlHeight: Math.min(...controls.map(node => rect(node).height)),
      outerOverflow: window.scrollX !== 0 || window.scrollY !== 0 || document.documentElement.scrollWidth > window.innerWidth + 1,
    };
  })()`) : domain === 'lighting' ? await backend.evaluate(`(() => {
    const layout = document.querySelector('.layoutTouch.touchDomainLighting');
    const upper = layout?.querySelector(':scope > .touchSurfacePanel');
    const tabs = layout?.querySelector(':scope > .touchControlDomainTabs');
    const rect = node => node?.getBoundingClientRect();
    const visible = node => Boolean(node && rect(node).width > 0 && rect(node).height > 0 && getComputedStyle(node).display !== 'none');
    const lower = ['lower-left', 'lower-right'].map(name => layout?.querySelector('[data-workspace-pane="' + name + '"]'));
    return {
      viewport: [window.innerWidth, window.innerHeight],
      controlSelected: document.querySelector('[data-workspace-option="touch"]')?.getAttribute('aria-pressed') === 'true',
      lightingSelected: document.querySelector('[data-control-domain="lighting"]')?.getAttribute('aria-pressed') === 'true',
      tabsLeft: Boolean(tabs && layout && rect(tabs).left <= rect(layout).left + 8),
      upperFullWidth: Boolean(upper && layout && Math.abs(rect(upper).width - rect(layout).width) <= 2),
      touchGridVisible: visible(upper?.querySelector('.touchSurfaceGrid')),
      placedControlCount: upper?.querySelectorAll('.touchPlacedControl').length ?? 0,
      lowerSideBySide: lower.every(visible) && lower.every(node => rect(node).top >= rect(upper).bottom - 2) && rect(lower[0]).right <= rect(lower[1]).left + 2,
      outerOverflow: window.scrollX !== 0 || window.scrollY !== 0 || document.documentElement.scrollWidth > window.innerWidth + 1,
    };
  })()`) : await backend.evaluate(`(() => {
    const layout = document.querySelector('.layoutTouch.touchDomainVideo');
    const upper = layout?.querySelector(':scope > .touchVideoPanel');
    const tabs = layout?.querySelector(':scope > .touchControlDomainTabs');
    const rect = node => node?.getBoundingClientRect();
    const visible = node => Boolean(node && rect(node).width > 0 && rect(node).height > 0 && getComputedStyle(node).display !== 'none');
    const lower = ['lower-left', 'lower-right'].map(name => layout?.querySelector('[data-workspace-pane="' + name + '"]'));
    return {
      viewport: [window.innerWidth, window.innerHeight],
      controlSelected: document.querySelector('[data-workspace-option="touch"]')?.getAttribute('aria-pressed') === 'true',
      videoSelected: document.querySelector('[data-control-domain="video"]')?.getAttribute('aria-pressed') === 'true',
      tabsLeft: Boolean(tabs && layout && rect(tabs).left <= rect(layout).left + 8),
      upperFullWidth: Boolean(upper && layout && Math.abs(rect(upper).width - rect(layout).width) <= 2),
      clipBankVisible: visible(upper?.querySelector('.videoClipSlotBankPanel')),
      emptyStateVisible: visible(upper?.querySelector('.videoClipSlotEmptyState')),
      emptySlotGridPresent: Boolean(upper?.querySelector('.videoClipSlotBankGrid')),
      emptyToolbarControlCount: upper?.querySelectorAll('.videoClipSlotBankToolbar button, .videoClipSlotBankToolbar select').length ?? 0,
      previewTransportPresent: Boolean(upper?.querySelector('.vjPreviewTransport')),
      lowerSideBySide: lower.every(visible) && lower.every(node => rect(node).top >= rect(upper).bottom - 2) && rect(lower[0]).right <= rect(lower[1]).left + 2,
      outerOverflow: window.scrollX !== 0 || window.scrollY !== 0 || document.documentElement.scrollWidth > window.innerWidth + 1,
    };
  })()`);
  if (domain === 'both') assert.deepEqual(
    [proof.controlSelected, proof.bothSelected, proof.tabsLeft, proof.upperFullWidth, proof.laneCount, proof.splitHidden, proof.lowerSideBySide, proof.controlCount >= 8, proof.minimumControlHeight >= 47.5, proof.outerOverflow],
    [true, true, true, true, 2, true, true, true, true, false],
    JSON.stringify(proof),
  );
  else if (domain === 'lighting') assert.deepEqual(
    [proof.controlSelected, proof.lightingSelected, proof.tabsLeft, proof.upperFullWidth, proof.touchGridVisible, proof.placedControlCount > 0, proof.lowerSideBySide, proof.outerOverflow],
    [true, true, true, true, true, true, true, false],
    JSON.stringify(proof),
  );
  else assert.deepEqual(
    [proof.controlSelected, proof.videoSelected, proof.tabsLeft, proof.upperFullWidth, proof.clipBankVisible, proof.emptyStateVisible, proof.emptySlotGridPresent, proof.emptyToolbarControlCount, proof.previewTransportPresent, proof.lowerSideBySide, proof.outerOverflow],
    [true, true, true, true, true, true, false, 1, false, true, false],
    JSON.stringify(proof),
  );

  const readImage = async () => (await backend.invoke('get_snapshot')).snapshot;
  const noOutput = snapshot => !snapshot.output.enabled && snapshot.dmx_outputs.every(route => !route.enabled) && snapshot.video.outputs.every(output => !output.enabled);
  assert(noOutput(await readImage()), 'BO probe requires every physical output disabled');
  const untilState = async predicate => { for (let n=0;n<100;n++){const snapshot=await readImage();assert(noOutput(snapshot));if(predicate(snapshot))return snapshot;await pause(50);}throw new Error('Native header BO state did not settle'); };
  const controllerExports={};
  new Function('exports',ts.transpileModule(await fs.readFile(new URL('../../../app/src/outputControlController.ts',import.meta.url),'utf8'),{compilerOptions:{module:ts.ModuleKind.CommonJS,target:ts.ScriptTarget.ES2022}}).outputText)(controllerExports);
  await controllerExports.enableOutput(backend.invoke);
  assert(noOutput(await readImage()), 'Lease preparation must preserve disabled routes');
  proof.createdThroughProjectMenu=true;
  proof.blackoutActions=[];
  for (const [action,match] of [['dmx-blackout', s=>s.authored_blackout],['video-blackout',s=>s.video.blackout],['all-blackout',s=>s.authored_blackout&&s.video.blackout]]) {
    await untilState(s=>!s.authored_blackout&&!s.video.blackout&&!s.safety_blackout_engaged);
    assert.equal(await click('[data-global-operator-action="'+action+'"]'),true);
    await untilState(s=>match(s));
    assert.equal(await click('[data-global-operator-action="'+action+'"]'),true);
    await untilState(s=>!s.authored_blackout&&!s.video.blackout&&!s.safety_blackout_engaged);
    proof.blackoutActions.push({action,engaged:true,cleared:true,everyOutputDisabled:true});
  }
  await backend.invoke('safety_blackout_engage_v1', {request:{operation_id:'syndocal.safety.blackout.engage.v1',request_id:1}});
  await untilState(s=>s.safety_blackout_engaged);
  for(let n=0;n<100;n++){if(await backend.evaluate("document.querySelector('[data-global-operator-action=\"dmx-blackout\"]')?.getAttribute(\"aria-pressed\")===\"true\""))break;await pause(50);}
  assert.equal(await click('[data-global-operator-action="dmx-blackout"]'),true);
  await untilState(s=>!s.safety_blackout_engaged&&!s.authored_blackout);
  proof.runtimeLatchClearedThroughDmxBo=true;
  assert.equal(await backend.evaluate("Boolean(document.querySelector('[data-global-operator-action=\"safety-blackout-release\"]'))"),false);
  assert.equal(await backend.evaluate('document.body.textContent.includes("Project changed since this video")'),false);
  proof.noStaleVideoReadError=true;
  const capture = await backend.send('Page.captureScreenshot', { format: 'png', fromSurface: true });
  assert.ok(capture.data && typeof capture.data === 'string');
  const png = Buffer.from(capture.data, 'base64');
  assert.ok(png.subarray(0, 8).equals(Buffer.from('89504e470d0a1a0a', 'hex')));
  await fs.writeFile(screenshot, png, { flag: 'wx' });
  proof.screenshotSha256 = createHash('sha256').update(png).digest('hex');
} catch (error) { failure = error; }
finally {
  backend?.close();
  if (child.exitCode === null && child.signalCode === null) {
    child.kill();
    const until = Date.now() + 10000;
    while (child.exitCode === null && child.signalCode === null && Date.now() < until) await pause(100);
    if (child.exitCode === null && child.signalCode === null) failure ??= new Error('QA process did not exit');
  }
}
const afterNormal = await readNormalDescriptor();
const normalAppIdentityUnchanged = beforeNormal === null ? afterNormal === null : Boolean(afterNormal?.equals(beforeNormal));
if (!normalAppIdentityUnchanged) {
  failure ??= new Error('Normal app identity changed');
}
await fs.writeFile(evidence, `${JSON.stringify({ schemaVersion: 1, timestamp: new Date().toISOString(), passed: !failure, domain, executable,
  executableSha256: createHash('sha256').update(await fs.readFile(executable)).digest('hex'),
  runnerSha256: createHash('sha256').update(await fs.readFile(new URL('./native-ui-blackout-probe.mjs', import.meta.url))).digest('hex'),
  backendHarnessSha256: createHash('sha256').update(await fs.readFile(new URL('../../../tools/syndocal-mcp/native-backend-session.mjs', import.meta.url))).digest('hex'),
  processId: child.pid, screenshot: proof?.screenshotSha256 ? screenshot : null, proof,
  normalAppIdentityUnchanged,
  error: failure?.message ?? null,
  nonclaims: ['Real native header DMX/VID/ALL BO engage and clear; runtime latch clear through DMX BO with all routes disabled', 'No output, physical device, recording, or venue workflow'],
}, null, 2)}\n`, { flag: 'wx' });
if (failure) throw failure;
console.log(`PASS native Control > ${domain} surface: ${proof.viewport.join('x')}; screenshot ${screenshot}`);
