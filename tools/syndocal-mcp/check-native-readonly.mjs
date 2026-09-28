// Real native acceptance, not the fake broker used by check.mjs. Bootstrap
// authority through the local main-window backend; all product reads use MCP.
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { execFile, spawn } from 'node:child_process';
import { createInterface } from 'node:readline';
import { fileURLToPath } from 'node:url';
import { promisify } from 'node:util';
import { createHash, randomUUID } from 'node:crypto';
import { parseOptions, readDescriptor } from './server.mjs';
import { nativeNetworkReads } from './native-network-reads.mjs';

const exec = promisify(execFile);
const args = process.argv.slice(2);
const take = (name) => {
  const index = args.indexOf(name);
  assert.ok(index >= 0 && index + 1 < args.length, `Required: ${name}`);
  return args.splice(index, 2)[1];
};
const executable = take('--expected-executable');
const cdpPort = Number(take('--cdp-port'));
const evidence = take('--evidence');
assert.equal(args.length, 0, 'Unknown arguments');
assert.equal(process.platform, 'win32', 'Windows native acceptance only');
assert.ok(path.isAbsolute(executable) && path.isAbsolute(evidence), 'Absolute paths required');
await fs.access(path.dirname(evidence));
assert.equal(await fs.stat(evidence).then(() => true, (error) => { if (error.code === 'ENOENT') return false; throw error; }), false, 'Evidence path already exists');
assert.ok(Number.isSafeInteger(cdpPort) && cdpPort >= 1024 && cdpPort <= 65535);
const options = parseOptions(['--expected-executable', executable]);
const descriptor = await readDescriptor(options);

// The isolated WebView2 debugger must belong to this exact app process tree.
// Only bounded integers enter this fixed query; no user-supplied shell program.
const identityScript = `
$ErrorActionPreference='Stop'
$listeners=@(Get-NetTCPConnection -LocalPort ${cdpPort} -State Listen)
if($listeners.Count -ne 1 -or $listeners[0].LocalAddress -ne '127.0.0.1'){throw 'Debugger must have one IPv4 loopback listener'}
$candidate=[int]$listeners[0].OwningProcess
$matched=$false
for($depth=0;$depth -lt 12;$depth++){
  if($candidate -eq ${descriptor.processId}){$matched=$true;break}
  $entry=Get-CimInstance Win32_Process -Filter "ProcessId=$candidate"
  if(!$entry){break}
  $candidate=[int]$entry.ParentProcessId
}
if(!$matched){throw 'Debugger owner is outside expected app process tree'}
'verified'
`;
const identity = await exec('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', identityScript], { windowsHide: true, timeout: 10000 });
assert.equal(identity.stdout.trim(), 'verified');
const pages = await (await fetch(`http://127.0.0.1:${cdpPort}/json/list`, { signal: AbortSignal.timeout(3000) })).json();
const targets = pages.filter((page) => page.type === 'page' && /^https?:\/\/tauri\.localhost\/(?:\?.*)?$/.test(page.url));
assert.equal(targets.length, 1, 'Exactly one native main page required');
const wsUrl = new URL(targets[0].webSocketDebuggerUrl);
assert.ok(['127.0.0.1', 'localhost'].includes(wsUrl.hostname) && Number(wsUrl.port) === cdpPort);
const socket = new WebSocket(wsUrl);
const pending = new Map();
let sequence = 0;
socket.onmessage = ({ data }) => {
  const message = JSON.parse(data);
  const call = pending.get(message.id);
  if (!call) return;
  clearTimeout(call.timer);
  pending.delete(message.id);
  if (message.error) call.reject(new Error('Native debugger command rejected'));
  else call.resolve(message.result);
};
await new Promise((resolve, reject) => {
  const timer = setTimeout(() => { socket.close(); reject(new Error('Debugger connection deadline')); }, 3000);
  socket.onopen = () => { clearTimeout(timer); resolve(); };
  socket.onerror = () => { clearTimeout(timer); reject(new Error('Debugger connection failed')); };
});
const evaluate = async (expression) => {
  const result = await new Promise((resolve, reject) => {
    const id = ++sequence;
    const timer = setTimeout(() => { pending.delete(id); reject(new Error('Native backend deadline')); }, 10000);
    pending.set(id, { resolve, reject, timer });
    socket.send(JSON.stringify({ id, method: 'Runtime.evaluate', params: { expression, awaitPromise: true, returnByValue: true } }));
  });
  assert.ok(!result.exceptionDetails, 'Native backend operation failed');
  return result.result.value;
};
const invoke = (command, params = {}) => evaluate(`window.__TAURI_INTERNALS__.invoke(${JSON.stringify(command)},${JSON.stringify(params)})`);
const principalId = `native-readonly-${randomUUID()}`;
let approval;
let credentialDirectory;
const results = [];
let runError;
let revoked = false;
let sidecar;
let responses;
const rpcPending = new Map();
try {
  assert.equal(await evaluate('window.__TAURI_INTERNALS__.metadata.currentWindow.label'), 'main');
  await invoke('plugin:window|maximize', { label: 'main' });
  const challenge = await invoke('agent_authority_begin_pairing_v1', { principalId });
  approval = await invoke('agent_authority_approve_pairing_v1', { challengeId: challenge.challengeId, challenge: challenge.challenge });
  assert.equal(approval.principalId, principalId);
  credentialDirectory = await fs.mkdtemp(path.join(os.tmpdir(), 'syndocal-native-readonly-'));
  const { stdout: sid } = await exec('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command', '[System.Security.Principal.WindowsIdentity]::GetCurrent().User.Value'], { windowsHide: true, timeout: 5000 });
  assert.match(sid.trim(), /^S-1-\d+(?:-\d+)+$/);
  await exec('icacls.exe', [credentialDirectory, '/inheritance:r', '/grant:r', `*${sid.trim()}:(OI)(CI)F`], { windowsHide: true, timeout: 5000 });
  options.principalId = principalId;
  options.principalIncarnation = approval.principalIncarnation;
  options.credentialFile = path.join(credentialDirectory, 'credential');
  await fs.writeFile(options.credentialFile, approval.credential, { flag: 'wx', mode: 0o600 });
  approval.credential = undefined;
  sidecar = spawn(process.execPath, [fileURLToPath(new URL('./server.mjs', import.meta.url)),
    '--expected-executable', executable, '--principal-id', principalId,
    '--principal-incarnation', String(approval.principalIncarnation), '--credential-file', options.credentialFile],
  { windowsHide: true, stdio: ['pipe', 'pipe', 'ignore'] });
  const failRpc = () => {
    for (const call of rpcPending.values()) {
      clearTimeout(call.timer);
      call.reject(new Error('MCP transport failed'));
    }
    rpcPending.clear();
  };
  sidecar.on('error', failRpc);
  sidecar.stdin.on('error', failRpc);
  sidecar.on('exit', failRpc);
  responses = createInterface({ input: sidecar.stdout });
  responses.on('line', (line) => {
    let response;
    try { response = JSON.parse(line); } catch { failRpc(); return; }
    const call = rpcPending.get(response.id);
    if (!call) return;
    clearTimeout(call.timer);
    rpcPending.delete(response.id);
    call.resolve(response);
  });
  let requestSequence = 0;
  const rpc = (method, params) => new Promise((resolve, reject) => {
    const id = ++requestSequence;
    const timer = setTimeout(() => { rpcPending.delete(id); reject(new Error('MCP response deadline')); }, 15000);
    rpcPending.set(id, { resolve, reject, timer });
    sidecar.stdin.write(`${JSON.stringify({ jsonrpc: '2.0', id, method, params })}\n`);
  });
  assert.ok((await rpc('initialize', { protocolVersion: '2025-11-25', capabilities: {}, clientInfo: { name: 'native-readonly-acceptance', version: '1' } })).result);
  sidecar.stdin.write(`${JSON.stringify({ jsonrpc: '2.0', method: 'notifications/initialized' })}\n`);
  const checks = [
    ['syndocal_list_fixtures', 'syndocal.query.agent_bridge.fixtures.list.v1'],
    ['syndocal_get_runtime_status', 'syndocal.query.agent_bridge.runtime.v1'],
    ['syndocal_get_control_plane_capabilities', 'syndocal.query.control_plane.capabilities.v1'],
    ['syndocal_get_recording_status', 'syndocal.query.recording.status.v1'],
  ];
  // Prove lack of grant is rejected before adding exact R0 permissions.
  const denied = await rpc('tools/call', { name: checks[0][0], arguments: {} });
  assert.equal(denied.result?.isError, true, 'Ungrantable default must reject');
  results.push({ check: 'ungranted-read-rejected', passed: true });
  let lastRequestId;
  for (const [tool, operationId] of checks) {
    await invoke('agent_authority_grant_v1', { principalId, principalIncarnation: approval.principalIncarnation,
      grant: { adapter: 'external_mcp', capability: 'read', operation_id: operationId, project_id: null } });
    let response = await rpc('tools/call', { name: tool, arguments: {} });
    let receipt = JSON.parse(response.result.content[0].text);
    const originalRequestId = receipt.requestId;
    lastRequestId = originalRequestId;
    const deadline = Date.now() + 15000;
    while (['pending', 'unknown'].includes(receipt.status) && Date.now() < deadline) {
      await new Promise((resolve) => setTimeout(resolve, 200));
      response = await rpc('tools/call', { name: 'syndocal_get_request_status', arguments: { requestId: originalRequestId } });
      receipt = JSON.parse(response.result.content[0].text);
      assert.equal(receipt.requestId, originalRequestId, 'Terminal read must retain original intent');
    }
    if (response.result?.isError) {
      const diagnostic = typeof receipt.error === 'string' ? receipt.error : receipt.result?.error?.code;
      results.push({ check: tool, passed: false, status: receipt.status, diagnostic });
    }
    assert.equal(response.result?.isError, false, `${tool} failed; see bounded evidence diagnostic`);
    assert.equal(receipt.status, 'completed');
    assert.equal(receipt.result.ok, true);
    results.push({ check: tool, passed: true, status: receipt.status });
  }
  const status = await invoke('agent_authority_status_v1');
  const own = status.principals.find((entry) => entry.principalId === principalId);
  assert.equal(own?.mode, 'safe');
  assert.equal(own.grants.length, 4);
  assert.ok(own.grants.every((grant) => grant.capability === 'read'));
  results.push(...await nativeNetworkReads(options));
  await invoke('agent_authority_revoke_v1', { principalId, principalIncarnation: approval.principalIncarnation });
  revoked = true;
  const deniedAfter = await rpc('tools/call', { name: checks[0][0], arguments: {} });
  assert.equal(deniedAfter.result?.isError, true, 'Revoked principal must reject');
  results.push({ check: 'revoked-read-rejected', passed: true });
  const deniedReceipt = await rpc('tools/call', { name: 'syndocal_get_request_status', arguments: { requestId: lastRequestId } });
  assert.equal(deniedReceipt.result?.isError, true);
  const revokedReceipt = JSON.parse(deniedReceipt.result.content[0].text);
  assert.equal(revokedReceipt.requestId, lastRequestId);
  assert.equal(revokedReceipt.error, 'agent_principal_revoked');
  assert.equal(revokedReceipt.result, undefined);
  results.push({ check: 'revoked-receipt-rejected-original-id', passed: true });
  results.push(...await nativeNetworkReads(options, { revoked: true }));
} catch (error) {
  runError = error;
} finally {
  if (approval && !revoked) {
    try {
      await invoke('agent_authority_revoke_v1', { principalId, principalIncarnation: approval.principalIncarnation });
      revoked = true;
    } catch { runError ??= new Error('Native authority cleanup failed'); }
  }
  socket.close();
  responses?.close();
  if (sidecar && sidecar.exitCode === null) {
    const exited = new Promise((resolve, reject) => {
      const timer = setTimeout(() => reject(new Error('MCP sidecar cleanup deadline')), 5000);
      sidecar.once('exit', () => { clearTimeout(timer); resolve(); });
    });
    sidecar.kill();
    try { await exited; } catch { runError ??= new Error('MCP sidecar cleanup deadline'); }
  }
  if (credentialDirectory) {
    // This path is exclusively the mkdtemp directory owned by this run.
    assert.equal(path.dirname(path.resolve(credentialDirectory)), path.resolve(os.tmpdir()));
    assert.ok(path.basename(credentialDirectory).startsWith('syndocal-native-readonly-'));
    await fs.rm(credentialDirectory, { recursive: true, force: true });
  }
}
const finalDescriptor = await readDescriptor(options);
assert.equal(finalDescriptor.instanceId, descriptor.instanceId, 'App instance changed during acceptance');
const summary = {
  schemaVersion: 1, timestamp: new Date().toISOString(), passed: !runError,
  executable: await fs.realpath(executable), processId: descriptor.processId,
  executableSha256: createHash('sha256').update(await fs.readFile(executable)).digest('hex'),
  sidecarSha256: createHash('sha256').update(await fs.readFile(new URL('./server.mjs', import.meta.url))).digest('hex'),
  runnerSha256: createHash('sha256').update(await fs.readFile(fileURLToPath(import.meta.url))).digest('hex'),
  networkHarnessSha256: createHash('sha256').update(await fs.readFile(new URL('./native-network-reads.mjs', import.meta.url))).digest('hex'),
  transport: 'External stdio MCP sidecar and real HTTP/REST/WebSocket clients to authenticated native loopback broker',
  bootstrap: 'process-verified local main-window backend via loopback CDP; no DOM actions',
  principalRevoked: revoked, credentialRemoved: true, checks: results,
  nonclaims: ['No authored/runtime/output mutation', 'No physical device acceptance', 'No clean installation or release acceptance'],
};
await fs.writeFile(evidence, `${JSON.stringify(summary, null, 2)}\n`, { flag: 'wx' });
if (runError) throw runError;
console.log(`Native MCP read-only acceptance passed: ${results.length} checks; principal revoked; credential removed.`);
