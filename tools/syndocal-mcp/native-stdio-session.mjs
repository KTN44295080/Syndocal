import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { createInterface } from 'node:readline';
import { fileURLToPath } from 'node:url';

// A real owned MCP sidecar; administrative bootstrap stays in the native session.
export async function openNativeStdioSession(options) {
  const child = spawn(process.execPath, [fileURLToPath(new URL('./server.mjs', import.meta.url)),
    '--expected-executable', options.executable, '--descriptor', options.descriptor,
    '--principal-id', options.principalId, '--principal-incarnation', String(options.principalIncarnation),
    '--credential-file', options.credentialFile], { windowsHide: true, stdio: ['pipe', 'pipe', 'ignore'] });
  const pending = new Map();
  let sequence = 0;
  let closed = false;
  const fail = () => {
    closed = true;
    for (const call of pending.values()) { clearTimeout(call.timer); call.reject(new Error('Owned MCP sidecar stopped')); }
    pending.clear();
  };
  child.on('error', fail);
  child.on('exit', fail);
  child.stdin.on('error', fail);
  const lines = createInterface({ input: child.stdout });
  lines.on('line', line => {
    let response;
    try {
      assert.ok(Buffer.byteLength(line) <= 256 * 1024);
      response = JSON.parse(line);
    } catch { fail(); return; }
    const call = pending.get(response.id);
    if (!call) return;
    clearTimeout(call.timer);
    pending.delete(response.id);
    call.resolve(response);
  });
  const rpc = (method, params) => new Promise((resolve, reject) => {
    if (closed) { reject(new Error('Owned MCP sidecar stopped')); return; }
    const id = ++sequence;
    const timer = setTimeout(() => { pending.delete(id); reject(new Error('MCP response deadline')); }, 15000);
    pending.set(id, { resolve, reject, timer });
    child.stdin.write(`${JSON.stringify({ jsonrpc: '2.0', id, method, params })}\n`);
  });
  const close = async () => {
    lines.close();
    fail();
    if (child.exitCode !== null || child.signalCode !== null) return;
    const exited = new Promise((resolve, reject) => {
      const timer = setTimeout(() => reject(new Error('Owned MCP sidecar cleanup deadline')), 5000);
      child.once('exit', () => { clearTimeout(timer); resolve(); });
    });
    child.kill();
    await exited;
  };
  try {
    assert.ok((await rpc('initialize', { protocolVersion: '2025-11-25', capabilities: {},
      clientInfo: { name: 'native-high-risk-acceptance', version: '1' } })).result);
    child.stdin.write(`${JSON.stringify({ jsonrpc: '2.0', method: 'notifications/initialized' })}\n`);
  } catch (error) { await close(); throw error; }
  return {
    processId: child.pid,
    async call(name, args) {
      const response = await rpc('tools/call', { name, arguments: args });
      assert.equal(response.error, undefined);
      assert.equal(response.result.content.length, 1);
      assert.equal(response.result.content[0].type, 'text');
      const receipt = JSON.parse(response.result.content[0].text);
      assert.equal(response.result.isError, receipt.status !== 'completed' || receipt.result?.ok !== true);
      return receipt;
    },
    close,
  };
}
