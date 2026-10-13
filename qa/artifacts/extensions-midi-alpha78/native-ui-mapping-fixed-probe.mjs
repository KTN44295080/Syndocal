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
  cwd: path.dirname(executable), windowsHide: true, stdio: ['ignore','ignore','pipe'],
  env: { ...process.env, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `${oldDebugArgs} --remote-debugging-address=127.0.0.1 --remote-debugging-port=${cdpPort}`.trim() },
});
child.stderr.on("data",d=>process.stderr.write(d));
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

  const transpiled = ts.transpileModule(await fs.readFile(new URL('../../../app/src/outputControlController.ts', import.meta.url), 'utf8'), { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.CommonJS } }).outputText;
  proof = {};
  proof.extensions = await backend.invoke('get_extensions_status');
  if (!proof.extensions.configured) { const configure = await backend.evaluate('window.__TAURI_INTERNALS__.invoke("set_extensions", {djLinkEnabled:false}).catch(e=>({error:String(e)}))'); console.log('extensions configure',configure); }
  await backend.evaluate("(()=>{document.querySelector(\".appMenuButton\")?.click();document.querySelector('.appProjectMenu button[aria-keyshortcuts=\"Control+N Meta+N\"]')?.click();})()");
  for(let n=0;n<100;n++){ if(await backend.evaluate('document.body.innerText.includes("Created new untitled project.")')) break; await pause(100); }
  proof.snapshot = (await backend.invoke('get_snapshot')).snapshot;
  assert.ok(proof.snapshot.dmx_outputs.every(o => !o.enabled), 'No physical DMX output allowed');
  assert.ok(proof.snapshot.video.outputs.every(o => !o.enabled), 'No physical video output allowed');
  const run = async body => backend.evaluate('(async()=>{const exports={};'+transpiled+';const invoke=(c,a)=>window.__TAURI_INTERNALS__.invoke(c,a);'+body+'})()');
  await pause(800);
  proof.enable = await run('try{return await exports.enableOutput(invoke);}catch(e){return {error:String(e)}}');
  proof.releaseSafety = await run('try{return await exports.executeBlackoutRelease(invoke);}catch(e){return {error:String(e)}}');
  console.log("releaseSafety",proof.releaseSafety);
  proof.toggles = [];
  for (const target of ['lighting','video','both']) for (const enabled of [true,false]) {
    const result = await run('try{return await exports.executeTargetBlackout(invoke,'+JSON.stringify(target)+','+enabled+');}catch(e){return {error:String(e)}}');
    proof.toggles.push({target,enabled,result});
  }
  const final = (await backend.invoke('get_snapshot')).snapshot;
  proof.snapshotAfter = { blackout: final.blackout, video_blackout: final.video.blackout, safety_blackout_engaged: final.safety_blackout_engaged };
  proof.snapshot = { dmx_outputs: proof.snapshot.dmx_outputs, video_outputs: proof.snapshot.video.outputs };
  await fs.writeFile(evidence,JSON.stringify(proof,null,2),{flag:'wx'});
  console.log(JSON.stringify({enable:proof.enable,toggles:proof.toggles}));
} finally { backend?.close(); if (child.exitCode === null) child.kill(); }
