import assert from 'node:assert/strict';
import { once } from 'node:events';
import { randomUUID } from 'node:crypto';
import { serveHttp } from './server.mjs';

// Real loopback clients to the real native broker; the adapter lives in this
// harness process. Only runtime/status reads are issued, never device commands.
export async function nativeNetworkReads(options, { revoked = false } = {}) {
  const server = serveHttp(options);
  const sockets = new Set();
  server.on('connection', socket => {
    sockets.add(socket); socket.once('close', () => sockets.delete(socket));
  });
  let ws;
  try {
    await once(server, 'listening');
    const port = server.address().port;
    const base = `http://127.0.0.1:${port}`;
    const session = randomUUID();
    let sequence = 0;
    const initialize = { protocolVersion: '2025-11-25', capabilities: {}, clientInfo: { name: 'native-network-read', version: '1' } };
    const post = async (pathname, body, headers = {}) => {
      const response = await fetch(`${base}${pathname}`, {
        method: 'POST', headers: { 'content-type': 'application/json', ...headers },
        body: JSON.stringify(body), signal: AbortSignal.timeout(15000),
      });
      assert.ok([200, 202].includes(response.status), 'Network adapter rejected transport');
      return response.status === 202 ? null : response.json();
    };
    const httpRpc = (method, params, notification = false) => post('/rpc', {
      jsonrpc: '2.0', ...(notification ? {} : { id: ++sequence }), method, params,
    }, { 'x-syndocal-session': session });
    assert.equal((await httpRpc('initialize', initialize)).result.protocolVersion, initialize.protocolVersion);
    await httpRpc('notifications/initialized', {}, true);
    ws = new WebSocket(`ws://127.0.0.1:${port}/ws`);
    await new Promise((resolve, reject) => {
      const timer = setTimeout(() => reject(new Error('WebSocket connect deadline')), 5000);
      ws.addEventListener('open', () => { clearTimeout(timer); resolve(); }, { once: true });
      ws.addEventListener('error', () => { clearTimeout(timer); reject(new Error('WebSocket connect failed')); }, { once: true });
    });
    const wsRpc = (method, params) => new Promise((resolve, reject) => {
      const id = ++sequence;
      const cleanup = () => { clearTimeout(timer); ws.removeEventListener('message', listener); };
      const timer = setTimeout(() => { cleanup(); reject(new Error('WebSocket response deadline')); }, 15000);
      const listener = event => {
        try {
          const result = JSON.parse(event.data);
          if (result.id === id) { cleanup(); resolve(result); }
        } catch { cleanup(); reject(new Error('Invalid WebSocket JSON')); }
      };
      ws.addEventListener('message', listener);
      ws.send(JSON.stringify({ jsonrpc: '2.0', id, method, params }));
    });
    assert.equal((await wsRpc('initialize', initialize)).result.protocolVersion, initialize.protocolVersion);
    ws.send(JSON.stringify({ jsonrpc: '2.0', method: 'notifications/initialized' }));
    const calls = [
      ['http-jsonrpc', async (name, args) => (await httpRpc('tools/call', { name, arguments: args })).result],
      ['rest', (name, args) => post(`/rest/tools/${name}`, args)],
      ['websocket', async (name, args) => (await wsRpc('tools/call', { name, arguments: args })).result],
    ];
    const results = [];
    for (const [transport, call] of calls) {
      let result = await call('syndocal_get_runtime_status', {});
      let receipt = JSON.parse(result.content[0].text);
      const requestId = receipt.requestId;
      const deadline = Date.now() + 15000;
      while (!revoked && ['pending', 'unknown'].includes(receipt.status) && Date.now() < deadline) {
        await new Promise(resolve => setTimeout(resolve, 100));
        result = await call('syndocal_get_request_status', { requestId });
        receipt = JSON.parse(result.content[0].text);
        assert.equal(receipt.requestId, requestId);
      }
      if (revoked) {
        assert.equal(result.isError, true);
        assert.equal(receipt.error, 'agent_principal_revoked');
        assert.equal(receipt.result, undefined);
      } else {
        assert.equal(result.isError, false, `${transport} native read failed`);
        assert.equal(receipt.status, 'completed');
        assert.equal(receipt.result.ok, true);
      }
      results.push({ check: `${transport}-${revoked ? 'revoked-denied' : 'native-terminal-read'}`, passed: true });
    }
    return results;
  } finally {
    ws?.close();
    const closed = new Promise(resolve => server.close(resolve));
    for (const socket of sockets) socket.destroy();
    await closed;
  }
}
