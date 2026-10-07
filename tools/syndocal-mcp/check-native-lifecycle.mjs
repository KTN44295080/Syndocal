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
import { nativeDiagnosticExports } from './native-diagnostic-exports.mjs';
import { nativeExternalHighRisk } from './native-external-high-risk.mjs';
import { nativeHighRiskRevocation } from './native-high-risk-revocation.mjs';
import { nativeTapBpm } from './native-tap-bpm.mjs';
import { nativeLeaseExpiry } from './native-lease-expiry.mjs';
import { nativeProjectJson } from './native-project-json.mjs';
import { nativeBackupJson } from './native-backup-json.mjs';
import { nativeControllerOutput } from './native-controller-output.mjs';
import { nativeRuntimeAuthority } from './native-runtime-authority.mjs';
import { nativeDisplayWindow } from './native-display-window.mjs';
import { nativeRecoveryStorage } from './native-recovery-storage.mjs';
import { nativeProjectReplacement } from './native-project-replacement.mjs';
import { nativeProjectFile } from './native-project-file.mjs';
import { nativeProjectBackup } from './native-project-backup.mjs';
import { nativeProjectBackupRestore } from './native-project-backup-restore.mjs';
import { nativeFileQueryPressure } from './native-file-query-pressure.mjs';

const exec = promisify(execFile);
const args = process.argv.slice(2);
const fileQueryPressure = args.includes('--file-query-pressure');
if (fileQueryPressure) {
  args.splice(args.indexOf('--file-query-pressure'), 1);
  assert.ok(!args.some(arg => /^--(?:project-|recovery-storage|display-window|runtime-authority|controller-|authored-controls|input-diagnostics|backup-json|project-json|lease-expiry|tap-bpm|diagnostics|external-)/.test(arg)),
    'File query pressure owns a separate private fixture and native read lane');
}
const projectBackup = args.includes('--project-backup');
if (projectBackup) args.splice(args.indexOf('--project-backup'), 1);
const projectFile = args.includes('--project-file') || projectBackup;
if (projectFile) {
  if (args.includes('--project-file')) args.splice(args.indexOf('--project-file'), 1);
  assert.ok(!args.some(arg => /^--(?:project-replacement|recovery-storage|display-window|runtime-authority|controller-|authored-controls|input-diagnostics|backup-json|project-json|lease-expiry|tap-bpm|diagnostics|external-)/.test(arg)),
    'Typed file publication owns a separate native private-file lane');
}
const projectReplacement = args.includes('--project-replacement');
if (projectReplacement) {
  args.splice(args.indexOf('--project-replacement'), 1);
  assert.ok(!args.some(arg => ['--project-replacement-preflight', '--recovery-storage', '--display-window', '--runtime-authority',
    '--controller-event-pressure', '--controller-safety-pressure', '--controller-burst', '--controller-inflight',
    '--controller-expiry', '--controller-restart', '--controller-output', '--authored-controls', '--input-diagnostics',
    '--backup-json', '--project-json', '--lease-expiry', '--tap-bpm', '--diagnostics', '--external-high-risk', '--external-revocation'].includes(arg)),
  'Typed project replacement owns a separate native project/file lane');
}
const projectReplacementPreflight = args.includes('--project-replacement-preflight');
if (projectReplacementPreflight) {
  args.splice(args.indexOf('--project-replacement-preflight'), 1);
  assert.ok(!args.some(arg => ['--recovery-storage', '--display-window', '--runtime-authority', '--controller-event-pressure',
    '--controller-safety-pressure', '--controller-burst', '--controller-inflight', '--controller-expiry',
    '--controller-restart', '--controller-output', '--authored-controls', '--input-diagnostics', '--backup-json',
    '--project-json', '--lease-expiry', '--tap-bpm', '--diagnostics', '--external-high-risk', '--external-revocation'].includes(arg)),
  'Project replacement preflight owns a separate native project/managed-output lane');
}
const recoveryStorage = args.includes('--recovery-storage');
if (recoveryStorage) {
  args.splice(args.indexOf('--recovery-storage'), 1);
  assert.ok(!args.some(arg => ['--display-window', '--runtime-authority', '--controller-event-pressure',
    '--controller-safety-pressure', '--controller-burst', '--controller-inflight', '--controller-expiry',
    '--controller-restart', '--controller-output', '--authored-controls', '--input-diagnostics', '--backup-json',
    '--project-json', '--lease-expiry', '--tap-bpm', '--diagnostics', '--external-high-risk', '--external-revocation'].includes(arg)),
  'Recovery storage acceptance owns a separate native profile/restart lane');
}
const displayWindow = args.includes('--display-window');
if (displayWindow) {
  args.splice(args.indexOf('--display-window'), 1);
  assert.ok(!args.some(arg => ['--runtime-authority', '--controller-event-pressure', '--controller-safety-pressure',
    '--controller-burst', '--controller-inflight', '--controller-expiry', '--controller-restart', '--controller-output',
    '--authored-controls', '--input-diagnostics', '--backup-json', '--project-json', '--lease-expiry', '--tap-bpm',
    '--diagnostics', '--external-high-risk', '--external-revocation'].includes(arg)),
  'Display shell acceptance owns a separate native window/project/output lane');
}
const runtimeAuthority = args.includes('--runtime-authority');
if (runtimeAuthority) {
  args.splice(args.indexOf('--runtime-authority'), 1);
  assert.ok(!args.some(arg => ['--controller-event-pressure', '--controller-safety-pressure', '--controller-burst',
    '--controller-inflight', '--controller-expiry', '--controller-restart', '--controller-output', '--authored-controls',
    '--input-diagnostics', '--backup-json', '--project-json', '--lease-expiry', '--tap-bpm', '--diagnostics',
    '--external-high-risk', '--external-revocation'].includes(arg)), 'Runtime authority reads own a separate read-only lane');
}
const controllerEventPressure = args.includes('--controller-event-pressure');
if (controllerEventPressure) args.splice(args.indexOf('--controller-event-pressure'), 1);
const controllerSafetyPressure = args.includes('--controller-safety-pressure');
if (controllerSafetyPressure) args.splice(args.indexOf('--controller-safety-pressure'), 1);
const controllerBurst = args.includes('--controller-burst');
if (controllerBurst) args.splice(args.indexOf('--controller-burst'), 1);
const controllerInFlight = args.includes('--controller-inflight');
if (controllerInFlight) args.splice(args.indexOf('--controller-inflight'), 1);
const controllerExpiry = args.includes('--controller-expiry');
if (controllerExpiry) args.splice(args.indexOf('--controller-expiry'), 1);
const controllerRestart = args.includes('--controller-restart');
if (controllerRestart) args.splice(args.indexOf('--controller-restart'), 1);
const controllerOutput = args.includes('--controller-output');
if (controllerOutput) args.splice(args.indexOf('--controller-output'), 1);
const authoredControls = args.includes('--authored-controls');
if (authoredControls) args.splice(args.indexOf('--authored-controls'), 1);
const inputDiagnostics = args.includes('--input-diagnostics');
if (inputDiagnostics) args.splice(args.indexOf('--input-diagnostics'), 1);
const backupJson = args.includes('--backup-json');
if (backupJson) args.splice(args.indexOf('--backup-json'), 1);
const projectJson = args.includes('--project-json');
if (projectJson) args.splice(args.indexOf('--project-json'), 1);
const leaseExpiry = args.includes('--lease-expiry');
if (leaseExpiry) args.splice(args.indexOf('--lease-expiry'), 1);
const tapBpm = args.includes('--tap-bpm');
if (tapBpm) args.splice(args.indexOf('--tap-bpm'), 1);
const diagnostics = args.includes('--diagnostics');
if (diagnostics) args.splice(args.indexOf('--diagnostics'), 1);
const externalHighRisk = args.includes('--external-high-risk');
if (externalHighRisk) args.splice(args.indexOf('--external-high-risk'), 1);
const externalRevocation = args.includes('--external-revocation');
if (externalRevocation) args.splice(args.indexOf('--external-revocation'), 1);
assert.ok(!leaseExpiry || !(externalHighRisk || externalRevocation), 'Lease-expiry drill owns a separate lease/request-rate lane');
assert.ok(!projectJson || !(leaseExpiry || externalHighRisk || externalRevocation || tapBpm || diagnostics),
  'Project-file admission owns a separate project/authority lane');
assert.ok(!backupJson || !(projectJson || leaseExpiry || externalHighRisk || externalRevocation || tapBpm || diagnostics),
  'Backup-file admission owns a separate project/authority lane');
assert.ok(!inputDiagnostics || projectJson || backupJson, 'Input diagnostics require the isolated project or backup lane');
assert.ok(!authoredControls || projectJson, 'Authored control round trip requires the isolated project-file lane');
assert.ok(!controllerOutput || !(projectJson || backupJson || leaseExpiry || externalHighRisk || externalRevocation || tapBpm || diagnostics),
  'Live software loopback owns a separate project/output/lease lane');
assert.ok(!controllerRestart || !(controllerOutput || projectJson || backupJson || leaseExpiry || externalHighRisk || externalRevocation || tapBpm || diagnostics),
  'Live native restart owns a separate project/output/process lane');
assert.ok(!controllerExpiry || !(controllerOutput || controllerRestart || projectJson || backupJson || leaseExpiry || externalHighRisk || externalRevocation || tapBpm || diagnostics),
  'Live native expiry owns a separate project/output/lease lane');
assert.ok(!controllerInFlight || !(controllerOutput || controllerRestart || controllerExpiry || projectJson || backupJson || leaseExpiry || externalHighRisk || externalRevocation || tapBpm || diagnostics),
  'Live native in-flight crash owns a separate project/output/file/process lane');
assert.ok(!controllerBurst || !(controllerInFlight || controllerOutput || controllerRestart || controllerExpiry || projectJson || backupJson || leaseExpiry || externalHighRisk || externalRevocation || tapBpm || diagnostics),
  'Live native burst owns a separate project/output/request lane');
assert.ok(!controllerSafetyPressure || !(controllerBurst || controllerInFlight || controllerOutput || controllerRestart || controllerExpiry || projectJson || backupJson || leaseExpiry || externalHighRisk || externalRevocation || tapBpm || diagnostics),
  'Live native safety pressure owns a separate project/output/request/revocation lane');
assert.ok(!controllerEventPressure || !(controllerSafetyPressure || controllerBurst || controllerInFlight || controllerOutput || controllerRestart || controllerExpiry || projectJson || backupJson || leaseExpiry || externalHighRisk || externalRevocation || tapBpm || diagnostics),
  'Live native event pressure owns a separate project/output/observation lane');
const take = name => { const index = args.indexOf(name); assert.ok(index >= 0 && index + 1 < args.length, `Required: ${name}`); return args.splice(index, 2)[1]; };
const executable = take('--expected-executable');
const evidence = take('--evidence');
const cdpPort = Number(take('--cdp-port'));
const profileId = args.includes('--profile') ? take('--profile') : 'jp.seraf.ktn.syndocal.qa.mcp-lifecycle';
assert.ok(['jp.seraf.ktn.syndocal.qa.mcp-lifecycle',
  'jp.seraf.ktn.syndocal.qa.mcp-lifecycle.backup-restore-20261005',
  'jp.seraf.ktn.syndocal.qa.mcp-lifecycle.backup-delete-20261006'].includes(profileId), 'Only checked-in private QA profiles allowed');
assert.equal(args.length, 0);
assert.equal(process.platform, 'win32');
assert.ok(path.isAbsolute(executable) && path.isAbsolute(evidence));
assert.ok(Number.isSafeInteger(cdpPort) && cdpPort >= 1024 && cdpPort <= 65535);
assert.equal(await fs.stat(evidence).then(() => true, error => { if (error.code === 'ENOENT') return false; throw error; }), false);
await fs.access(path.dirname(evidence));
const profile = path.join(process.env.LOCALAPPDATA, profileId);
const descriptorPath = path.join(profile, 'agent-bridge-v1.json');
// Never run lifecycle termination against the normal checkout executable.
const isolatedExe = path.join(os.tmpdir(), 'syndocal-native-acceptance-target', 'release', 'syndocal.exe');
assert.equal((await fs.realpath(executable)).toLowerCase(), (await fs.realpath(isolatedExe)).toLowerCase());
const oldDebugArgs = process.env.WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS ?? '';
assert.ok(!/--remote-debugging-(?:port|address)/i.test(oldDebugArgs));
const principalId = `lifecycle-${randomUUID()}`;
const options = parseOptions(['--expected-executable', executable, '--descriptor', descriptorPath]);
options.profileId = profileId;
const checks = [];
let child;
let backend;
let approval;
let credentialDirectory;
let cleanupNeeded = false;
let failure;
let nativeStderr = '';
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
  child = spawn(executable, [], { cwd: path.dirname(executable), windowsHide: true, stdio: ['ignore', 'ignore', 'pipe'],
    env: { ...process.env, WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `${oldDebugArgs} --remote-debugging-address=127.0.0.1 --remote-debugging-port=${cdpPort}`.trim() } });
  child.stderr.on('data', chunk => { nativeStderr = (nativeStderr + chunk.toString()).slice(-65536); });
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
  let first = await start();
  if (recoveryStorage) {
    await nativeRecoveryStorage(backend, checks, { currentBackend: () => backend, restart: async () => {
      await stop(true); await start(); return backend;
    } });
    first = backend.descriptor;
  }
  if (backupJson) await nativeBackupJson(backend, checks, { inputDiagnostics });
  if (projectJson) await nativeProjectJson(backend, checks, { inputDiagnostics, authoredControls });
  if (tapBpm) await nativeTapBpm(backend, checks, { evidencePath: evidence });
  const diagnostic = diagnostics ? await nativeDiagnosticExports(backend.invoke) : undefined;
  if (diagnostic) checks.push(diagnostic.check);
  credentialDirectory = await fs.mkdtemp(path.join(os.tmpdir(), 'syndocal-lifecycle-credential-'));
  const { stdout } = await exec('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', '[System.Security.Principal.WindowsIdentity]::GetCurrent().User.Value'], { windowsHide: true, timeout: 5000 });
  assert.match(stdout.trim(), /^S-1-\d+(?:-\d+)+$/);
  await exec('icacls.exe', [credentialDirectory, '/inheritance:r', '/grant:r', `*${stdout.trim()}:(OI)(CI)F`], { windowsHide: true, timeout: 5000 });
  options.credentialFile = path.join(credentialDirectory, 'credential');
  await pair(); await install();
  if (fileQueryPressure) await nativeFileQueryPressure(backend, options, checks);
  if (projectReplacement) await nativeProjectReplacement(backend, options, checks);
  if (projectFile) await nativeProjectFile(backend, options, checks);
  if (projectBackup) await nativeProjectBackup(backend, options, checks);
  if (projectBackup) await nativeProjectBackupRestore(backend, options, checks);
  if (runtimeAuthority) await nativeRuntimeAuthority(backend, options, checks);
  if (displayWindow) await nativeDisplayWindow(backend, options, checks);
  if (leaseExpiry) await nativeLeaseExpiry(backend, options, checks);
  if (externalHighRisk) await nativeExternalHighRisk(backend, options, checks);
  if (controllerOutput) await nativeControllerOutput(backend, options, checks);
  if (projectReplacementPreflight) await nativeControllerOutput(backend, options, checks, { projectReplacementPreflight: true });
  if (controllerBurst) await nativeControllerOutput(backend, options, checks, { burstRequests: true });
  if (controllerEventPressure) {
    await nativeControllerOutput(backend, options, checks, { eventPressure: true });
    const priorInstance = backend.descriptor.instanceId;
    await stop(); await start(); await pair(); await install();
    assert.notEqual(backend.descriptor.instanceId, priorInstance);
    checks.push({ check: 'event-pressure-owned-process-reset-before-common-lifecycle', passed: true });
    first = backend.descriptor;
  }
  if (controllerSafetyPressure) {
    for (const safetyPressure of ['saturation', 'kill_switch']) {
      await nativeControllerOutput(backend, options, checks, { safetyPressure });
      const priorInstance = backend.descriptor.instanceId;
      await stop();
      await start();
      await pair(); await install();
      assert.notEqual(backend.descriptor.instanceId, priorInstance);
      checks.push({ check: `native-${safetyPressure}-owned-restart-restores-empty-disarmed-qa`, passed: true });
    }
    first = await readDescriptor(options);
  }
  if (controllerExpiry) await nativeControllerOutput(backend, options, checks, { leaseExpiry: true });
  if (controllerRestart || controllerInFlight) {
    await nativeControllerOutput(backend, options, checks, { inFlightCrash: controllerInFlight, restartLifecycle: async observeStopped => {
      await stop();
      await observeStopped();
      await start();
      await pair(); await install();
      return backend;
    } });
    // Base lifecycle assertions bind the actual current launch after the
    // extra live-output restart, rather than the already terminated first one.
    first = await readDescriptor(options);
  }
  const oldRequest = await read();
  checks.push({ check: 'isolated-first-launch-read', passed: true });
  await stop();
  const stopped = await nativeRequest(options, 'runtime.get', {}, randomUUID());
  assert.equal(stopped.status, 'rejected');
  assert.match(stopped.error, /^Selected Syndocal descriptor\/process\/executable could not be verified/);
  checks.push({ check: 'dead-process-descriptor-rejected', passed: true });
  const second = await start();
  if (diagnostic) {
    const retiredParams = { captureId: diagnostic.retired.captureId, sha256: diagnostic.retired.sha256, approved: true };
    const retiredResult = await backend.evaluate(`window.__TAURI_INTERNALS__.invoke('finish_diagnostic_export_v1', ${JSON.stringify(retiredParams)}).then(() => ({ok:true}), error => ({error:String(error)}))`);
    assert.deepEqual(retiredResult, { error: 'Diagnostic capture unknown, consumed or expired; prepare again' });
    checks.push({ check: 'diagnostic-capture-rejected-after-native-restart', passed: true });
  }
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
  if (externalRevocation || projectReplacement || projectFile) await nativeHighRiskRevocation(backend, options, checks, { projectReplacement, projectFile, projectBackup });
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
const normalAppIdentityUnchanged = beforeNormal === null ? afterNormal === null : afterNormal?.equals(beforeNormal) === true;
if (!normalAppIdentityUnchanged && !failure) failure = new Error('Normal app identity must be unchanged');
const nativePanicLocations = nativeStderr.split(/\r?\n/).filter(line => /panicked at .*\.rs:\d+:\d+/.test(line)).map(line => line.slice(0, 512));
if (nativePanicLocations.length > 0) failure ??= new Error('Native worker panic observed during QA');
await fs.writeFile(evidence, `${JSON.stringify({ schemaVersion: 1, timestamp: new Date().toISOString(), passed: !failure && !cleanupNeeded,
  executable, executableSha256: createHash('sha256').update(await fs.readFile(executable)).digest('hex'),
  sidecarSha256: createHash('sha256').update(await fs.readFile(new URL('./server.mjs', import.meta.url))).digest('hex'),
  runnerSha256: createHash('sha256').update(await fs.readFile(new URL('./check-native-lifecycle.mjs', import.meta.url))).digest('hex'),
  backendHarnessSha256: createHash('sha256').update(await fs.readFile(new URL('./native-backend-session.mjs', import.meta.url))).digest('hex'),
  ...(recoveryStorage ? {
    recoveryStorageHarnessSha256: createHash('sha256').update(await fs.readFile(new URL('./native-recovery-storage.mjs', import.meta.url))).digest('hex'),
    recoveryStorageSourceSha256: createHash('sha256').update(await fs.readFile(new URL('../../app/src/projectRecoveryStorage.ts', import.meta.url))).digest('hex'),
    recoveryStartupAppSourceSha256: createHash('sha256').update(await fs.readFile(new URL('../../app/src/App.tsx', import.meta.url))).digest('hex'),
    recoveryStorageTransport: 'Real isolated WebView2 localStorage and production App startup after owned native restarts; process-verified backend diagnostics, no DOM actions',
  } : {}),
  ...(controllerOutput || controllerRestart || controllerExpiry || controllerInFlight || projectReplacementPreflight ? {
    controllerOutputHarnessSha256: createHash('sha256').update(await fs.readFile(new URL('./native-controller-output.mjs', import.meta.url))).digest('hex'),
    stdioHarnessSha256: createHash('sha256').update(await fs.readFile(new URL('./native-stdio-session.mjs', import.meta.url))).digest('hex'),
    controllerOutputTransport: 'Separate authenticated stdio MCP sidecar, actual native loopback-only Art-Net sender, owned ephemeral 127.0.0.1 UDP receiver',
    ...(controllerRestart ? { controllerRestartHarnessSha256: createHash('sha256').update(await fs.readFile(new URL('./native-controller-restart.mjs', import.meta.url))).digest('hex') } : {}),
    ...(controllerExpiry ? { controllerExpiryHarnessSha256: createHash('sha256').update(await fs.readFile(new URL('./native-controller-expiry.mjs', import.meta.url))).digest('hex') } : {}),
    ...(controllerInFlight ? { controllerInFlightHarnessSha256: createHash('sha256').update(await fs.readFile(new URL('./native-controller-inflight.mjs', import.meta.url))).digest('hex') } : {}),
  } : {}),
  ...(projectReplacementPreflight ? {
    projectReplacementPreflightHarnessSha256: createHash('sha256').update(await fs.readFile(new URL('./native-project-replacement-preflight.mjs', import.meta.url))).digest('hex'),
    projectReplacementSourceSha256: createHash('sha256').update(await fs.readFile(new URL('../../app/src-tauri/src/main.rs', import.meta.url))).digest('hex'),
    managedRetirementEngineSha256: createHash('sha256').update(await fs.readFile(new URL('../../crates/engine/src/lib.rs', import.meta.url))).digest('hex'),
    managedNetworkRetirementSourceSha256: createHash('sha256').update(await fs.readFile(new URL('../../crates/engine/src/managed_network_dmx_retirement.rs', import.meta.url))).digest('hex'),
    managedKeepalivePortsSha256: createHash('sha256').update(await fs.readFile(new URL('../../app/src-tauri/src/output_lease_keepalive_runtime.rs', import.meta.url))).digest('hex'),
  } : {}),
  ...(inputDiagnostics ? { inputDiagnosticHarnessSha256: createHash('sha256').update(await fs.readFile(new URL('./native-input-diagnostics.mjs', import.meta.url))).digest('hex') } : {}),
  ...(externalHighRisk ? {
    highRiskHarnessSha256: createHash('sha256').update(await fs.readFile(new URL('./native-external-high-risk.mjs', import.meta.url))).digest('hex'),
    stdioHarnessSha256: createHash('sha256').update(await fs.readFile(new URL('./native-stdio-session.mjs', import.meta.url))).digest('hex'),
    highRiskTransport: 'Separate stdio MCP sidecar process forwarding to the authenticated native broker',
  } : {}),
  ...(diagnostics ? { diagnosticHarnessSha256: createHash('sha256').update(await fs.readFile(new URL('./native-diagnostic-exports.mjs', import.meta.url))).digest('hex') } : {}),
  ...((diagnostics||externalHighRisk||projectBackup) ? {diagnosticAuditSources:await Promise.all([
    'diagnostic_audit.rs','diagnostic_audit_capture.rs','diagnostic_package.rs','agent_authority_service.rs',
    'agent_bridge_execution.rs','control_plane_runtime.rs','output_lease.rs','project_file_control_plane.rs','project_replacement_control_plane.rs',
    'project_file_audit.rs',
  ].map(async name=>({name,sha256:createHash('sha256').update(await fs.readFile(new URL(`../../app/src-tauri/src/${name}`,import.meta.url))).digest('hex')})))} : {}),
  ...(leaseExpiry ? {
    leaseExpiryHarnessSha256: createHash('sha256').update(await fs.readFile(new URL('./native-lease-expiry.mjs', import.meta.url))).digest('hex'),
    stdioHarnessSha256: createHash('sha256').update(await fs.readFile(new URL('./native-stdio-session.mjs', import.meta.url))).digest('hex'),
    leaseExpiryTransport: 'Two separately owned authenticated stdio MCP processes; real backend monotonic TTL, no synthetic clock or implicit renewal',
  } : {}),
  ...(projectJson ? {
    projectJsonHarnessSha256: createHash('sha256').update(await fs.readFile(new URL('./native-project-json.mjs', import.meta.url))).digest('hex'),
    projectJsonTransport: `Exact native load_project_path command on private isolated ${authoredControls ? 'empty and authored-control' : 'empty'} project files; no DOM/dialog action`,
    ...(authoredControls ? { authoredControlHarnessSha256: createHash('sha256').update(await fs.readFile(new URL('./native-authored-control-project.mjs', import.meta.url))).digest('hex') } : {}),
  } : {}),
  ...(backupJson ? {
    backupJsonHarnessSha256: createHash('sha256').update(await fs.readFile(new URL('./native-backup-json.mjs', import.meta.url))).digest('hex'),
    backupJsonTransport: 'Exact native load_project_backup/list_project_backups on individually owned isolated QA-profile files; no DOM/dialog action',
  } : {}),
  ...(tapBpm ? {
    tapHarnessSha256: createHash('sha256').update(await fs.readFile(new URL('./native-tap-bpm.mjs', import.meta.url))).digest('hex'),
    tapControllerSha256: createHash('sha256').update(await fs.readFile(new URL('../../app/src/tapTempo.ts', import.meta.url))).digest('hex'),
    tapQaReceiverSha256: createHash('sha256').update(await fs.readFile(new URL('../../app/src/nativeTapTempoQa.ts', import.meta.url))).digest('hex'),
  } : {}),
  ...(projectReplacement ? {
    projectReplacementHarnessSha256: createHash('sha256').update(await fs.readFile(new URL('./native-project-replacement.mjs', import.meta.url))).digest('hex'),
    projectReplacementControllerSha256: createHash('sha256').update(await fs.readFile(new URL('../../app/src-tauri/src/project_replacement_control_plane.rs', import.meta.url))).digest('hex'),
    projectPreparationEngineSha256: createHash('sha256').update(await fs.readFile(new URL('../../crates/engine/src/lib.rs', import.meta.url))).digest('hex'),
    projectPreparationPolicySha256: createHash('sha256').update(await fs.readFile(new URL('../../crates/engine/src/project_load_preparation.rs', import.meta.url))).digest('hex'),
    projectReplacementProtocolSha256: createHash('sha256').update(await fs.readFile(new URL('../../crates/protocol/src/control_plane_project.rs', import.meta.url))).digest('hex'),
    canonicalQueryAdapterSha256: createHash('sha256').update(await fs.readFile(new URL('../../app/src/agentBridgeControlPlane.ts', import.meta.url))).digest('hex'),
    nativeExecutorSha256: createHash('sha256').update(await fs.readFile(new URL('../../app/src-tauri/src/agent_bridge_execution.rs', import.meta.url))).digest('hex'),
    stdioHarnessSha256: createHash('sha256').update(await fs.readFile(new URL('./native-stdio-session.mjs', import.meta.url))).digest('hex'),
  } : {}),
  ...(projectBackup ? {
    diagnosticHarnessSha256: createHash('sha256').update(await fs.readFile(new URL('./native-diagnostic-exports.mjs', import.meta.url))).digest('hex'),
    projectBackupHarnessSha256: createHash('sha256').update(await fs.readFile(new URL('./native-project-backup.mjs', import.meta.url))).digest('hex'),
    backupDeletionManagementHarnessSha256: createHash('sha256').update(await fs.readFile(new URL('./native-backup-deletion-management.mjs', import.meta.url))).digest('hex'),
    backupDeletionManagementPolicySha256: createHash('sha256').update(await fs.readFile(new URL('../../app/src-tauri/src/project_backup_deletion_management.rs', import.meta.url))).digest('hex'),
    backupDeletionManagementProtocolSha256: createHash('sha256').update(await fs.readFile(new URL('../../crates/protocol/src/control_plane_backup_management.rs', import.meta.url))).digest('hex'),
    managedBackupPolicySha256: createHash('sha256').update(await fs.readFile(new URL('../../app/src-tauri/src/project_file_managed_backup.rs', import.meta.url))).digest('hex'),
    backupInspectionPolicySha256: createHash('sha256').update(await fs.readFile(new URL('../../app/src-tauri/src/project_backup_inspection.rs', import.meta.url))).digest('hex'),
    backupListingPolicySha256: createHash('sha256').update(await fs.readFile(new URL('../../app/src-tauri/src/project_backup_listing.rs', import.meta.url))).digest('hex'),
    backupDeletionPolicySha256: createHash('sha256').update(await fs.readFile(new URL('../../app/src-tauri/src/project_backup_deletion.rs', import.meta.url))).digest('hex'),
    backupDeletionJournalPolicySha256: createHash('sha256').update(await fs.readFile(new URL('../../app/src-tauri/src/project_backup_deletion_journal.rs', import.meta.url))).digest('hex'),
    backupRestoreHarnessSha256: createHash('sha256').update(await fs.readFile(new URL('./native-project-backup-restore.mjs', import.meta.url))).digest('hex'),
    backupRestorePolicySha256: createHash('sha256').update(await fs.readFile(new URL('../../app/src-tauri/src/project_backup_restoration.rs', import.meta.url))).digest('hex'),
  } : {}),
  ...(projectFile ? {
    projectFileHarnessSha256: createHash('sha256').update(await fs.readFile(new URL('./native-project-file.mjs', import.meta.url))).digest('hex'),
    projectFileControllerSha256: createHash('sha256').update(await fs.readFile(new URL('../../app/src-tauri/src/project_file_control_plane.rs', import.meta.url))).digest('hex'),
    projectFileProtocolSha256: createHash('sha256').update(await fs.readFile(new URL('../../crates/protocol/src/control_plane_file.rs', import.meta.url))).digest('hex'),
    projectFileNativeSha256: createHash('sha256').update(await fs.readFile(new URL('../../app/src-tauri/src/main.rs', import.meta.url))).digest('hex'),
    nativeExecutorSha256: createHash('sha256').update(await fs.readFile(new URL('../../app/src-tauri/src/agent_bridge_execution.rs', import.meta.url))).digest('hex'),
  } : {}),
  ...(fileQueryPressure ? {
    fileQueryPressureHarnessSha256: createHash('sha256').update(await fs.readFile(new URL('./native-file-query-pressure.mjs', import.meta.url))).digest('hex'),
    fileQueryControllerSha256: createHash('sha256').update(await fs.readFile(new URL('../../app/src-tauri/src/project_file_control_plane.rs', import.meta.url))).digest('hex'),
    nativeAdapterErrorSha256: createHash('sha256').update(await fs.readFile(new URL('../../app/src-tauri/src/native_adapter_error.rs', import.meta.url))).digest('hex'),
    queryCaptureSha256: createHash('sha256').update(await fs.readFile(new URL('../../app/src-tauri/src/control_plane_query.rs', import.meta.url))).digest('hex'),
    projectFileNativeSha256: createHash('sha256').update(await fs.readFile(new URL('../../app/src-tauri/src/main.rs', import.meta.url))).digest('hex'),
    nativeExecutorSha256: createHash('sha256').update(await fs.readFile(new URL('../../app/src-tauri/src/agent_bridge_execution.rs', import.meta.url))).digest('hex'),
    stdioHarnessSha256: createHash('sha256').update(await fs.readFile(new URL('./native-stdio-session.mjs', import.meta.url))).digest('hex'),
    fileQueryPressureTransport: 'Actual local IPC and authenticated stdio MCP file/backup authority reads with real asynchronous native checkpoint-bundle capture pressure; no lock-hold command, operation retry, DOM or physical output',
  } : {}),
  ...(controllerBurst ? {
    controllerBurstHarnessSha256: createHash('sha256').update(await fs.readFile(new URL('./native-controller-burst.mjs', import.meta.url))).digest('hex'),
    admissionPolicySha256: createHash('sha256').update(await fs.readFile(new URL('./native-request-admission.mjs', import.meta.url))).digest('hex'),
    stdioHarnessSha256: createHash('sha256').update(await fs.readFile(new URL('./native-stdio-session.mjs', import.meta.url))).digest('hex'),
    controllerOutputHarnessSha256: createHash('sha256').update(await fs.readFile(new URL('./native-controller-output.mjs', import.meta.url))).digest('hex'),
  } : {}),
  ...(controllerEventPressure ? {
    eventPressureHarnessSha256: createHash('sha256').update(await fs.readFile(new URL('./native-controller-event-pressure.mjs', import.meta.url))).digest('hex'),
    canonicalQueryAdapterSha256: createHash('sha256').update(await fs.readFile(new URL('../../app/src/agentBridgeControlPlane.ts', import.meta.url))).digest('hex'),
    controllerOutputHarnessSha256: createHash('sha256').update(await fs.readFile(new URL('./native-controller-output.mjs', import.meta.url))).digest('hex'),
    stdioHarnessSha256: createHash('sha256').update(await fs.readFile(new URL('./native-stdio-session.mjs', import.meta.url))).digest('hex'),
  } : {}),
  ...(runtimeAuthority ? {
    runtimeAuthorityHarnessSha256: createHash('sha256').update(await fs.readFile(new URL('./native-runtime-authority.mjs', import.meta.url))).digest('hex'),
    runtimeAuthoritySourceSha256: createHash('sha256').update(await fs.readFile(new URL('../../app/src-tauri/src/control_plane_runtime.rs', import.meta.url))).digest('hex'),
    runtimeAuthorityAdapterSha256: createHash('sha256').update(await fs.readFile(new URL('../../app/src/agentBridgeTools.ts', import.meta.url))).digest('hex'),
    stdioHarnessSha256: createHash('sha256').update(await fs.readFile(new URL('./native-stdio-session.mjs', import.meta.url))).digest('hex'),
  } : {}),
  ...(displayWindow ? {
    displayWindowHarnessSha256: createHash('sha256').update(await fs.readFile(new URL('./native-display-window.mjs', import.meta.url))).digest('hex'),
    displayWindowControllerSha256: createHash('sha256').update(await fs.readFile(new URL('../../app/src-tauri/src/control_plane_runtime.rs', import.meta.url))).digest('hex'),
    displayWindowPhysicalCoreSha256: createHash('sha256').update(await fs.readFile(new URL('../../app/src-tauri/src/main.rs', import.meta.url))).digest('hex'),
    stdioHarnessSha256: createHash('sha256').update(await fs.readFile(new URL('./native-stdio-session.mjs', import.meta.url))).digest('hex'),
  } : {}),
  ...(controllerSafetyPressure ? {
    safetyPressureHarnessSha256: createHash('sha256').update(await fs.readFile(new URL('./native-controller-safety-pressure.mjs', import.meta.url))).digest('hex'),
    controllerOutputHarnessSha256: createHash('sha256').update(await fs.readFile(new URL('./native-controller-output.mjs', import.meta.url))).digest('hex'),
    stdioHarnessSha256: createHash('sha256').update(await fs.readFile(new URL('./native-stdio-session.mjs', import.meta.url))).digest('hex'),
  } : {}),
  ...(externalRevocation ? {
    revocationHarnessSha256: createHash('sha256').update(await fs.readFile(new URL('./native-high-risk-revocation.mjs', import.meta.url))).digest('hex'),
    revocationTransport: 'Separate stdio MCP requests, native claim/revoke/execute commands; QA renderer generation retired immediately before graceful close',
    ...(!externalHighRisk ? { stdioHarnessSha256: createHash('sha256').update(await fs.readFile(new URL('./native-stdio-session.mjs', import.meta.url))).digest('hex') } : {}),
  } : {}),
  profile: profileId, checks, normalAppIdentityUnchanged,
  credentialRevoked: !cleanupNeeded, credentialFileRemoved: true,
  nativePanicLocations,
  nonclaims: ['QA identifier release build, not the distributed artifact',
    projectReplacementPreflight ? 'Actual managed Enable, live 512-channel loopback ArtDMX, rejected stale New/Open authority and valid replacement retirement. No simultaneous authority-change atomicity, external MCP New/Open exposure, general serial/device retirement, physical video/fixture/venue or release acceptance'
    : recoveryStorage ? 'Actual isolated WebView2 recovery storage preservation and actionable production startup error across native restarts; unchanged empty native checkpoint/journal and closed output gates. No recovery load/discard UI action, successful recovery acknowledgement, browser write/quota fault, older-generation fallback, full upgrade/template/backup/recovery matrix, physical output or release acceptance'
    : displayWindow ? 'Actual native small decorated enabled Display shell/GPU lifecycle on the editor monitor through authenticated MCP, exact grants/fences/lease and terminal receipt replay. Private empty composition, no media/audio/active DMX route or fixture. No visible content/color/pixel, fullscreen/topology/hotplug, AddDisplay, all output operations, hardware/venue or release acceptance'
    : controllerEventPressure ? 'Actual native 2048-record retention gap, authenticated MCP slow subscriber, authoritative resnapshot and exact runtime-delta convergence under 64-slot pending broker pressure with live whole-image loopback output. Local fenced Loop changes keep the established 4/s rate; real production query adapter completes individually claimed reads. No Engine command-queue saturation, every adapter/event, complete tick/frame/audio/UI budget, physical device, venue or release acceptance'
    : controllerSafetyPressure ? 'Actual 64-slot external broker pending saturation, 10000 R4 intents and local native S0 all-zero loopback output before/after revocation and while Kill Switch is active. Existing renderer registration retains immutable external work; no Engine command-queue saturation, complete tick/frame/audio/UI budget, publisher event-gap, physical device, venue or release acceptance'
      : controllerBurst ? 'Actual 10000 stdio discovery and 10000 R4 master intents with software loopback ArtDMX reception and measured packet intervals only; no complete output tick/frame/audio/UI budget, publisher event-gap, local priority Blackout under saturation, physical device, venue or release acceptance'
      : controllerInFlight ? 'Actual native queued, claimed and native-committed R4 master/R5 diagnostic requests interrupted before broker completion; unknown restart status never re-executes, software loopback re-Arm remains explicit and owned export bytes remain intact. No physical fixtures, recording/authored crash matrix, worker-internal interruption, full durable mutation completion or release acceptance'
      : controllerRestart ? 'Actual native forced process termination/restart and explicit re-Arm with software loopback reception only; no physical fixtures, venue, serial DMX, video, worker-specific failure, in-flight mutation crash durability or whole controller-loss/re-arm acceptance'
      : controllerExpiry ? 'Actual native live monotonic lease expiry, stale-operation rejection and explicit recovery with software loopback reception only; no physical fixtures, venue, serial DMX, video, worker-specific failure, in-flight mutation crash durability or whole controller-loss/re-arm acceptance'
      : controllerOutput ? 'Actual native software loopback ArtDMX reception only; no device, venue, serial DMX, video, worker/process-loss or whole controller-loss/re-arm acceptance'
      : 'No physical output activation; optional high-risk or lease-expiry probe mutates isolated backend lease authority only; optional Tap probe changes isolated empty-project tempo; optional project-file probe loads private JSON with no file media or video source activation',
    'No durable authored/output mutation crash publication acceptance',
    ...(projectReplacement ? ['Typed external R5 New/Open, issued-fence and exact File-grant admission, immutable native execution, private source-byte preservation and bounded terminal replay only; no active managed output, physical devices, mid-parser revocation/crash, full file-operation or AI8/release acceptance'] : []),
  ...(projectFile ? ['Typed external Save/Save As/template/status/ack on owned private files only; no mid-publication external rename CAS, crash/restart, all remaining File/AI8/release or physical acceptance'] : []),
    ...(fileQueryPressure ? ['Private valid unbound-label Touch fixture, then bounded real local and authenticated external authority reads under concurrent asynchronous checkpoint capture only; no proof of the historical generic query failure cause, complete contention/rate/real-time budget, file publication, physical output or release acceptance'] : []),
    ...(projectBackup ? ['Typed managed backup creation/inspection/list/restore/delete, complete private authored-image and four mapping-family restoration, exact terminal/ack/replay, bounded deletion-journal/status guards and owned cleanup only; no full retention-race/mid-publication revocation, actual deletion crash/power-loss/restart reconciliation, operator resolution/ack/retention, all File or physical acceptance'] : []),
    ...(projectJson ? ['Native .sdc admission and rejected-load preservation only; no complete migration corpus, backup/recovery/upgrade, other file formats or physical output acceptance'] : []),
    ...(runtimeAuthority ? ['Three native Timeline authority-read paths and ordinary concurrent read observations only; no command replay, output activation, complete lock-wait or realtime budget, full adapter/security/AI8 or physical acceptance'] : []),
    ...(backupJson ? ['Native backup byte/JSON/identity admission and private restore preservation only; no full O1-O4, browser recovery, crash/fallback/upgrade, physical output or release acceptance'] : []),
    ...(leaseExpiry ? ['Stdio disconnect is adapter loss, not native registered-owner retirement; unchanged persisted output configuration and closed runtime gates are not a physical signal or whole controller-loss/re-arm proof'] : []),
    ...(diagnostics ? ['Diagnostic publication covers a private temporary destination only; no crash-during-publication, removable-filesystem or external MCP consent acceptance'] : [])],
}, null, 2)}\n`, { flag: 'wx' });
if (failure) throw failure;
console.log(`PASS isolated native lifecycle: ${checks.length} checks; normal instance unchanged; QA process and credential cleaned up.`);
