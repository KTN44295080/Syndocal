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

  const click = async selector => backend.evaluate(`(()=>{let e=document.querySelector(${JSON.stringify(selector)}); if(!e)return false;e.click();return true;})()`);
  proof={};
  proof.ipc=await backend.evaluate(`(()=>{window.__qaCalls=[];const original=window.fetch.bind(window);window.fetch=async(...a)=>{let log={url:String(a[0]),init:a[1]};window.__qaCalls.push(log);try{let r=await original(...a);log.status=r.status;log.body=await r.clone().text(); if(log.url.endsWith("/learn_midi_control")) return new Response(JSON.stringify({channel:0,message:"ControlChange",number:window.__qaLearnNumber??27}),{status:r.status,headers:r.headers}); if(log.url.endsWith("/set_project_control_mappings") && window.__qaPauseMappingAck) {window.__qaAckHeld=true;await new Promise(resolve=>window.__qaReleaseAck=resolve);window.__qaPauseMappingAck=false;} return r}catch(e){log.error=String(e);throw e}};return {ipc:window.__TAURI_INTERNALS__.ipc.toString(),props:Object.fromEntries(Object.entries(Object.getOwnPropertyDescriptors(window.__TAURI_INTERNALS__)).map(([k,d])=>[k,{w:d.writable,c:d.configurable}]))};})()`);
  await click('.appMenuButton'); await click('.appProjectMenu button[aria-keyshortcuts="Control+N Meta+N"]');
  for(let n=0;n<100;n++){if(await backend.evaluate('document.body.innerText.includes("Created new untitled project.")'))break;await pause(100);}
  await pause(400);
  await click('[data-workspace-option="control"]'); await pause(400);
  proof.before= await backend.evaluate('document.body.innerText');
  await click('[data-scene-matrix-create-scene="1"]'); await pause(1300);
  await click('[data-scene-matrix-create-scene="1"]'); await pause(1300);
  assert.equal(await click('[data-scene-matrix-create-bank]'),true);
  await backend.evaluate(`(()=>{let i=document.querySelector('.sceneMatrixBankEditor input');if(!i)throw Error('Bank editor missing');i.value='Authored Bank';i.dispatchEvent(new Event('input',{bubbles:true}));})()`);
  assert.equal(await click('[data-scene-matrix-save-bank]'),true);await pause(700);
  const authored=(await backend.invoke('get_snapshot')).snapshot;assert.equal(authored.cue_lists.length,2);proof.authoredBanksBeforeNew=authored.cue_lists.map(b=>b.label);
  proof.afterCreate=await backend.evaluate('document.body.innerText');
  proof.snapshot=await backend.invoke('get_snapshot');
  proof.buttons=await backend.evaluate('[...document.querySelectorAll("button")].map(e=>({text:e.innerText,label:e.getAttribute("aria-label"),data:[...e.attributes].filter(a=>a.name.startsWith("data-")).map(a=>[a.name,a.value])}))');
  await click('[data-control-learn-toggle="midi"]'); await pause(400);
  proof.targets=await backend.evaluate('[...document.querySelectorAll("[data-control-map-target]")].slice(0,8).map(e=>({tag:e.tagName,label:e.getAttribute("data-control-map-label"),target:e.getAttribute("data-control-map-target")}))');
  await click('[data-control-map-target]'); await pause(12000);
  proof.afterLearn=await backend.evaluate('document.body.innerText');
  proof.mappings=await backend.invoke('get_project_control_mappings');
  proof.calls=await backend.evaluate('window.__qaCalls');
  proof.counts=proof.calls.reduce((a,x)=>{let k=x.url.split('/').pop();a[k]=(a[k]??0)+1;return a},{});
  assert.equal(proof.mappings.midi_mappings.length,1);
  assert.match(proof.afterLearn,/MIDI mapped ControlChange/,'Learn must finish persistence and reconnect');
  assert.ok((proof.counts.list_midi_inputs??0)<=1,'Bootstrap must not repeat after ACK');
  assert.ok((proof.counts.set_project_control_mappings??0)<=2,'No repeated writes of one mapping');
  assert.equal(await click('[data-control-learn-toggle="midi"]'),true);
  assert.equal(await backend.evaluate(`(()=>{let e=document.querySelector('[data-scene-matrix-cue-id="1"] [data-scene-matrix-cue-name]'); if(!e)return false;e.dispatchEvent(new MouseEvent('dblclick',{bubbles:true}));return true;})()`),true);
  await pause(150);
  assert.equal(await backend.evaluate(`(()=>{let i=document.querySelector('[data-scene-matrix-rename-input]');if(!i)return false;i.value='Renamed persisted';i.dispatchEvent(new Event('input',{bubbles:true}));i.dispatchEvent(new KeyboardEvent('keydown',{key:'Enter',bubbles:true}));return true;})()`),true);
  await pause(1200);
  const afterRename=(await backend.invoke('get_snapshot')).snapshot;
  assert.equal(afterRename.cues.find(c=>c.id===1)?.label,'Renamed persisted');
  assert.equal(afterRename.active_cue_id,null,'Name edit cannot trigger scene');
  assert.equal(await backend.evaluate(`document.querySelector('[data-scene-matrix-cue-id="1"] [data-scene-matrix-cue-name]')?.textContent`),'Renamed persisted');
  proof.inlineRename={persisted:true,noSceneTrigger:true,reflectedInBank:true};
  const capture=await backend.send('Page.captureScreenshot',{format:'png',fromSurface:true});const picture=Buffer.from(capture.data,'base64');proof.screenshotSha256=createHash('sha256').update(picture).digest('hex');await fs.writeFile(screenshot,picture,{flag:'wx'});
  await backend.evaluate('window.__qaLearnNumber=28;window.__qaPauseMappingAck=true;window.__qaAckHeld=false;true');
  assert.equal(await click('[data-control-learn-toggle="midi"]'),true);
  assert.equal(await click('[data-control-map-target]'),true);
  for(let n=0;n<160;n++){if(await backend.evaluate('window.__qaAckHeld===true'))break;await pause(100);}
  assert.equal(await backend.evaluate('window.__qaAckHeld'),true,'Second Learn must reach a real held mapping ACK');
  const newCallsBefore=await backend.evaluate('window.__qaCalls.filter(c=>c.url.endsWith("/new_project")).length');
  assert.equal(await click('.appMenuButton'),true);assert.equal(await click('.appProjectMenu button[aria-keyshortcuts="Control+N Meta+N"]'),true);await pause(250);await click('[data-project-discard-confirm]');
  await pause(150);
  assert.equal(await backend.evaluate('window.__qaCalls.filter(c=>c.url.endsWith("/new_project")).length'),newCallsBefore,'New must not race ahead of its pending mapping ACK');
  await backend.evaluate('window.__qaReleaseAck();true');
  proof.newWaitedForMappingAck=true;

  await pause(1400);
  const reset=(await backend.invoke('get_snapshot')).snapshot;const resetMaps=await backend.invoke('get_project_control_mappings');
  assert.equal(reset.cues.length,0);assert.deepEqual(reset.cue_lists.map(b=>({id:b.id,label:b.label})),[{id:1,label:'Bank 1'}]);assert.equal(resetMaps.midi_mappings.length,0);
  assert.equal(reset.fixtures.length,0);assert.ok(!reset.output.enabled&&reset.dmx_outputs.every(r=>!r.enabled));
  assert.equal(await backend.evaluate('Boolean(document.querySelector("[data-control-map-selected]"))'),false);
  const resetBundle=await backend.invoke("get_project_authority_bundle");assert.equal(resetBundle.input_runtime.midi_control_active,false);
  proof.newProject={clearedScenes:true,clearedAuthoredBank:true,clearedMappings:true,defaultEmptyBankOnly:true,outputsDisabled:true,midiRuntimeStopped:true};
  // Keep evidence small: response previews and command counts, no repeated snapshots.
  proof.calls=proof.calls.map(x=>({url:x.url,status:x.status,request:x.init?.body,replyPreview:x.body?.slice(0,220)}));


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
  runnerSha256: createHash('sha256').update(await fs.readFile(new URL('./native-ui-mapping-fixed-probe.mjs', import.meta.url))).digest('hex'),
  backendHarnessSha256: createHash('sha256').update(await fs.readFile(new URL('../../../tools/syndocal-mcp/native-backend-session.mjs', import.meta.url))).digest('hex'),
  processId: child.pid, screenshot: proof?.screenshotSha256 ? screenshot : null, proof,
  normalAppIdentityUnchanged,
  error: failure?.message ?? null,
  nonclaims: ['MIDI learned sample injected after actual native input timeout; physical MIDI message capture not observed', 'Actual native mapping persistence, reconnect, scene rename and New with every output disabled; no physical output or macOS GUI claim'],
}, null, 2)}\n`, { flag: 'wx' });
if (failure) throw failure;
console.log('Native reproduction completed: '+evidence);
