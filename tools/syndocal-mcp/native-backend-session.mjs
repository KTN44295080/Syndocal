import assert from 'node:assert/strict';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { readDescriptor } from './server.mjs';
const exec = promisify(execFile);

// Bind administrative QA bootstrap to the exact native executable and process.
export async function openNativeBackendSession(options, cdpPort) {
  assert.ok(Number.isSafeInteger(cdpPort) && cdpPort >= 1024 && cdpPort <= 65535);
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
  socket.onclose = () => {
    for (const call of pending.values()) { clearTimeout(call.timer); call.reject(new Error('Native debugger closed')); }
    pending.clear();
  };
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

  return { descriptor, invoke, evaluate, close() {
    for (const call of pending.values()) { clearTimeout(call.timer); call.reject(new Error('Native session closed')); }
    pending.clear(); socket.close();
  } };
}
