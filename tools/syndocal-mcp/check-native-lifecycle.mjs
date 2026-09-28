import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import net from 'node:net';
import { spawn, execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { createHash, createHmac, randomBytes, randomUUID } from 'node:crypto';
import { parseOptions, readDescriptor, nativeRequest } from './server.mjs';
import { openNativeBackendSession } from './native-backend-session.mjs';

const exec = promisify(execFile);
const args = process.argv.slice(2);
const take = name => { const index = args.indexOf(name); assert.ok(index >= 0 && index + 1 < args.length, `Required: ${name}`); return args.splice(index, 2)[1]; };
const executable = take('--expected-executable');
const evidence = take('--evidence');
const cdpPort = Number(take('--cdp-port'));
assert.equal(args.length, 0);
assert.equal(process.platform, 'win32');
assert.ok(path.isAbsolute(executable) && path.isAbsolute(evidence));
assert.ok(Number.isSafeInteger(cdpPort) && cdpPort >= 1024 && cdpPort <= 65535);
assert.equal(await fs.stat(evidence).then(() => true, error => { if (error.code === 'ENOENT') return false; throw error; }), false);
await fs.access(path.dirname(evidence));
const profile = path.join(process.env.LOCALAPPDATA, 'jp.seraf.ktn.syndocal.qa.mcp-lifecycle');
const descriptorPath = path.join(profile, 'agent-bridge-v1.json');
// Never run lifecycle termination against the normal checkout executable.
const isolatedExe = path.join(os.tmpdir(), 'syndocal-native-acceptance-target', 'release', 'syndocal.exe');
assert.equal((await fs.realpath(executable)).toLowerCase(), (await fs.realpath(isolatedExe)).toLowerCase());
const oldDebugArgs = process.env.WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS ?? '';
assert.ok(!/--remote-debugging-(?:port|address)/i.test(oldDebugArgs));
const principalId = `lifecycle-${randomUUID()}`;
const options = parseOptions(['--expected-executable', executable, '--descriptor', descriptorPath]);
const checks = [];
let child;
let backend;
let approval;
let credentialDirectory;
let cleanupNeeded = false;
let failure;
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const readNormalDescriptor = () => fs.readFile(path.join(process.env.LOCALAPPDATA, 'jp.seraf.ktn.syndocal', 'agent-bridge-v1.json')).catch(error => {
  if (error.code === 'ENOENT') return null;
  throw error;
});
const beforeNormal = await readNormalDescriptor();
const start = async () => {
  const preflight = await exec('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command',
    `if (@(Get-NetTCPConnection -LocalPort ${cdpPort} -State Listen -ErrorAction SilentlyContinue).Count -gt 0) { throw 'QA debugger port occupied' }`], { windowsHide: true, timeout: 10000 });
  assert.equal(preflight.stderr.trim(), '');
  child = spawn(executable, [], { cwd: path.dirname(executable), windowsHide: true, stdio: 'ignore',
    env: { ...process.env, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `${oldDebugArgs} --remote-debugging-address=127.0.0.1 --remote-debugging-port=${cdpPort}`.trim() } });
  let spawnFailure;
  child.once('error', () => { spawnFailure = true; });
  const until = Date.now() + 60000;
  while (Date.now() < until) {
    assert.ok(!spawnFailure && child.exitCode === null, 'Isolated native process failed to start');
    try {
      const descriptor = await readDescriptor(options);
      assert.equal(descriptor.processId, child.pid);
      const pages = await (await fetch(`http://127.0.0.1:${cdpPort}/json/list`, { signal: AbortSignal.timeout(1000) })).json();
      if (pages.some(page => page.type === 'page' && /^https?:\/\/tauri\.localhost\//.test(page.url))) break;
    } catch { /* Wait only for this newly launched process and its endpoint. */ }
    await pause(200);
  }
  backend = await openNativeBackendSession(options, cdpPort);
  assert.equal(backend.descriptor.processId, child.pid);
  assert.equal(await backend.evaluate('window.__TAURI_INTERNALS__.metadata.currentWindow.label'), 'main');
  await backend.invoke('plugin:window|maximize', { label: 'main' });
  const windowProof = await exec('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', `
Add-Type -TypeDefinition 'using System; using System.Runtime.InteropServices; public static class QaWindow { [DllImport("user32.dll")] public static extern bool IsZoomed(IntPtr hWnd); }'
$windows = @(Get-Process syndocal | Where-Object { $_.Path -eq $env:SYNDOCAL_QA_EXPECTED_EXE -and $_.MainWindowHandle -ne 0 })
if ($windows.Count -ne 1 -or $windows[0].Id -ne ${child.pid} -or !$windows[0].Responding -or $windows[0].MainWindowTitle -ne 'Syndocal QA - MCP Lifecycle' -or ![QaWindow]::IsZoomed($windows[0].MainWindowHandle)) { throw 'QA native window gate failed' }
'verified'
`], { windowsHide: true, timeout: 10000, env: { ...process.env, SYNDOCAL_QA_EXPECTED_EXE: await fs.realpath(executable) } });
  assert.equal(windowProof.stdout.trim(), 'verified');
  // A fresh QA identifier must never adopt the operator's project.
  const blank = await backend.evaluate(`window.__TAURI_INTERNALS__.invoke('get_project_authority_bundle').then(b => ({fixtures:b.snapshot.fixtures.length, playing:b.snapshot.timeline.playing, outputs:b.snapshot.video.outputs.length}))`);
  assert.deepEqual(blank, { fixtures: 0, playing: false, outputs: 0 });
  return backend.descriptor;
};
const stop = async (graceful = false) => {
  if (!child || child.exitCode !== null || child.signalCode !== null) { backend?.close(); backend = undefined; return; }
  await exec('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command',
    `$p = Get-Process -Id ${child.pid} -ErrorAction Stop; if ($p.Path -ne $env:SYNDOCAL_QA_EXPECTED_EXE) { throw 'QA process executable mismatch' }`],
  { windowsHide: true, timeout: 10000, env: { ...process.env, SYNDOCAL_QA_EXPECTED_EXE: await fs.realpath(executable) } });
  if (graceful) {
    await backend.invoke('plugin:window|close', { label: 'main' }).catch(() => {});
  } else child.kill();
  const until = Date.now() + 10000;
  while (child.exitCode === null && child.signalCode === null && Date.now() < until) await pause(100);
  assert.ok(child.exitCode !== null || child.signalCode !== null, 'QA process did not exit');
  backend?.close(); backend = undefined;
  // WebView2 children can briefly retain the debugger socket after parent exit.
  const released = Date.now() + 10000;
  while (Date.now() < released) {
    try { await fetch(`http://127.0.0.1:${cdpPort}/json/list`, { signal: AbortSignal.timeout(300) }); }
    catch { return; }
    await pause(100);
  }
  throw new Error('Owned QA debugger did not close');
};
const pair = async () => {
  const challenge = await backend.invoke('agent_authority_begin_pairing_v1', { principalId });
  approval = await backend.invoke('agent_authority_approve_pairing_v1', { challengeId: challenge.challengeId, challenge: challenge.challenge });
  cleanupNeeded = true;
  options.principalId = principalId;
  options.principalIncarnation = approval.principalIncarnation;
};
const install = async () => {
  await fs.writeFile(options.credentialFile, approval.credential);
  await backend.invoke('agent_authority_grant_v1', { principalId, principalIncarnation: approval.principalIncarnation,
    grant: { adapter: 'external_mcp', capability: 'read', operation_id: 'syndocal.query.agent_bridge.runtime.v1', project_id: null } });
};
const read = async () => {
  const id = randomUUID();
  let receipt = await nativeRequest(options, 'runtime.get', {}, id);
  const until = Date.now() + 15000;
  while (receipt.status === 'pending' && Date.now() < until) {
    await pause(100);
    receipt = await nativeRequest(options, 'request.status', { requestId: id }, randomUUID());
  }
  assert.equal(receipt.status, 'completed');
  assert.equal(receipt.requestId, id);
  assert.equal(receipt.result.ok, true);
  return id;
};
try {
  const first = await start();
  credentialDirectory = await fs.mkdtemp(path.join(os.tmpdir(), 'syndocal-lifecycle-credential-'));
  const { stdout } = await exec('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', '[System.Security.Principal.WindowsIdentity]::GetCurrent().User.Value'], { windowsHide: true, timeout: 5000 });
  assert.match(stdout.trim(), /^S-1-\d+(?:-\d+)+$/);
  await exec('icacls.exe', [credentialDirectory, '/inheritance:r', '/grant:r', `*${stdout.trim()}:(OI)(CI)F`], { windowsHide: true, timeout: 5000 });
  options.credentialFile = path.join(credentialDirectory, 'credential');
  await pair(); await install();
  const oldRequest = await read();
  checks.push({ check: 'isolated-first-launch-read', passed: true });
  await stop();
  const stopped = await nativeRequest(options, 'runtime.get', {}, randomUUID());
  assert.equal(stopped.status, 'rejected');
  assert.match(stopped.error, /^Selected Syndocal descriptor\/process\/executable could not be verified/);
  checks.push({ check: 'dead-process-descriptor-rejected', passed: true });
  const second = await start();
  assert.notEqual(second.instanceId, first.instanceId);
  assert.ok(second.sessionNonce !== first.sessionNonce, 'Launch nonce must rotate');
  assert.ok(second.token !== first.token, 'Broker token must rotate');
  const retired = await nativeRequest(options, 'runtime.get', {}, randomUUID());
  assert.equal(retired.status, 'rejected');
  assert.equal(retired.error, 'agent_unknown_principal');
  assert.equal(retired.result, undefined);
  checks.push({ check: 'crash-restart-retired-principal-rejected', passed: true });
  await pair();
  const oldCredential = await nativeRequest(options, 'runtime.get', {}, randomUUID());
  assert.equal(oldCredential.status, 'rejected');
  assert.equal(oldCredential.error, 'agent_auth_proof_invalid');
  assert.equal(oldCredential.result, undefined);
  await install();
  const oldReceipt = await nativeRequest(options, 'request.status', { requestId: oldRequest }, randomUUID());
  assert.equal(oldReceipt.status, 'unknown');
  assert.equal(oldReceipt.result, undefined);
  checks.push({ check: 'old-credential-and-receipt-not-adopted', passed: true });
  const requestId = randomUUID();
  const clientNonce = randomBytes(32).toString('hex');
  const wire = { token: second.token, requestId, method: 'runtime.get', params: {}, auth: {
    principalId, principalIncarnation: approval.principalIncarnation, clientNonce,
    proof: createHmac('sha256', Buffer.from(approval.credential, 'hex')).update(`${first.sessionNonce}\0${clientNonce}\0${requestId}\0runtime.get`).digest('hex'),
  } };
  const replay = await new Promise((resolve, reject) => {
    const socket = net.createConnection({ host: '127.0.0.1', port: second.port });
    let data = '';
    const timer = setTimeout(() => { socket.destroy(); reject(new Error('Replay probe deadline')); }, 3000);
    socket.on('connect', () => socket.write(`${JSON.stringify(wire)}\n`));
    socket.on('error', () => { clearTimeout(timer); reject(new Error('Replay connection failed')); });
    socket.on('data', chunk => {
      data += chunk.toString();
      if (data.length > 65536) { clearTimeout(timer); socket.destroy(); reject(new Error('Replay response bound')); return; }
      if (data.includes('\n')) {
        clearTimeout(timer); socket.destroy();
        try { resolve(JSON.parse(data.split('\n')[0])); } catch { reject(new Error('Invalid replay response')); }
      }
    });
  });
  assert.equal(replay.status, 'rejected');
  assert.equal(replay.error, 'agent_auth_proof_invalid');
  await read();
  checks.push({ check: 'old-launch-proof-rejected-fresh-read-succeeds', passed: true });
  await backend.invoke('agent_authority_revoke_v1', { principalId, principalIncarnation: approval.principalIncarnation });
  cleanupNeeded = false;
  await stop(true);
  checks.push({ check: 'graceful-native-close', passed: true });
  const third = await start();
  assert.notEqual(third.instanceId, second.instanceId);
  const afterGraceful = await nativeRequest(options, 'runtime.get', {}, randomUUID());
  assert.equal(afterGraceful.status, 'rejected');
  assert.equal(afterGraceful.error, 'agent_unknown_principal');
  checks.push({ check: 'graceful-restart-does-not-restore-grant', passed: true });
} catch (error) { failure = error; }
finally {
  if (cleanupNeeded && backend) {
    try {
      // After a crash, the process-local principal is absent; re-pairing the
      // same unique QA name replaces only this test's orphan credential.
      const state = await backend.invoke('agent_authority_status_v1');
      const own = state.principals.find(entry => entry.principalId === principalId && !entry.revoked);
      if (own) approval = { principalIncarnation: own.principalIncarnation };
      else await pair();
      await backend.invoke('agent_authority_revoke_v1', { principalId, principalIncarnation: approval.principalIncarnation });
      cleanupNeeded = false;
    } catch { failure ??= new Error('QA credential revocation failed'); }
  }
  try { await stop(); } catch { failure ??= new Error('QA process cleanup failed'); }
  if (credentialDirectory) {
    assert.equal(path.dirname(path.resolve(credentialDirectory)), path.resolve(os.tmpdir()));
    assert.ok(path.basename(credentialDirectory).startsWith('syndocal-lifecycle-credential-'));
    await fs.rm(credentialDirectory, { recursive: true, force: true });
  }
}
const afterNormal = await readNormalDescriptor();
assert.ok(beforeNormal === null ? afterNormal === null : afterNormal?.equals(beforeNormal), 'Normal app identity must be unchanged');
await fs.writeFile(evidence, `${JSON.stringify({ schemaVersion: 1, timestamp: new Date().toISOString(), passed: !failure && !cleanupNeeded,
  executable, executableSha256: createHash('sha256').update(await fs.readFile(executable)).digest('hex'),
  sidecarSha256: createHash('sha256').update(await fs.readFile(new URL('./server.mjs', import.meta.url))).digest('hex'),
  runnerSha256: createHash('sha256').update(await fs.readFile(new URL('./check-native-lifecycle.mjs', import.meta.url))).digest('hex'),
  backendHarnessSha256: createHash('sha256').update(await fs.readFile(new URL('./native-backend-session.mjs', import.meta.url))).digest('hex'),
  profile: 'jp.seraf.ktn.syndocal.qa.mcp-lifecycle', checks, normalAppIdentityUnchanged: true,
  credentialRevoked: !cleanupNeeded, credentialFileRemoved: true,
  nonclaims: ['QA identifier release build, not the distributed artifact', 'No authored mutation, device or output command', 'No durable mutation/crash publication acceptance'],
}, null, 2)}\n`, { flag: 'wx' });
if (failure) throw failure;
console.log(`PASS isolated native lifecycle: ${checks.length} checks; normal instance unchanged; QA process and credential cleaned up.`);
