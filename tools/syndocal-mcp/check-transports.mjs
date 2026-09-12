import assert from 'node:assert/strict';
import { createServer as createNetServer } from 'node:net';
import { once } from 'node:events';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { createHmac, randomBytes } from 'node:crypto';
import { serveHttp } from './server.mjs';

const temporary = await fs.mkdtemp(path.join(os.tmpdir(), 'syndocal-mcp-transport-'));
const descriptorPath = path.join(temporary, 'bridge.json');
const credentialPath = path.join(temporary, 'credential.txt');
const token = `transport-${randomBytes(16).toString('hex')}`;
const sessionNonce = randomBytes(32).toString('hex');
const credential = randomBytes(32).toString('hex');
const brokerRequests = [];
const broker = createNetServer((socket) => {
  let data = '';
  socket.on('data', (chunk) => {
    data += chunk.toString('utf8');
    const newline = data.indexOf('\n');
    if (newline < 0) return;
    const request = JSON.parse(data.slice(0, newline));
    brokerRequests.push(request);
    assert.equal(request.token, token);
    assert.equal(request.auth.principalId, 'transport-client');
    assert.equal(request.auth.principalIncarnation, 7);
    assert.match(request.auth.clientNonce, /^[0-9a-f]{64}$/u);
    assert.equal(request.auth.proof, createHmac('sha256', Buffer.from(credential, 'hex'))
      .update(`${sessionNonce}\0${request.auth.clientNonce}\0${request.requestId}\0${request.method}`)
      .digest('hex'));
    socket.end(JSON.stringify({ requestId: request.requestId, status: 'completed', result: { ok: true } }) + '\n');
  });
});
await new Promise((resolve) => broker.listen(0, '127.0.0.1', resolve));
await fs.writeFile(credentialPath, `${credential}\n`, { encoding: 'utf8', mode: 0o600 });
await fs.writeFile(descriptorPath, JSON.stringify({ protocolVersion: 1, port: broker.address().port, token, sessionNonce, instanceId: randomBytes(16).toString('hex'), processId: process.pid, executablePath: process.execPath }));
const options = { descriptor: descriptorPath, executable: process.execPath, principalId: 'transport-client', principalIncarnation: 7, credentialFile: credentialPath };
const httpServer = serveHttp(options, { port: 0 });
await once(httpServer, 'listening');
const port = httpServer.address().port;
const startupOnlyServer = serveHttp({ descriptor: path.join(temporary, 'not-started.json'), executable: path.join(temporary, 'not-started.exe'), principalId: 'transport-client', principalIncarnation: 7, credentialFile: credentialPath }, { port: 0 });
await once(startupOnlyServer, 'listening');
const startupOnlyPort = startupOnlyServer.address().port;

const request = async (method, pathname, body, headers = {}) => {
  const response = await fetch(`http://127.0.0.1:${port}${pathname}`, {
    method,
    headers: { ...(body === undefined ? {} : { 'content-type': 'application/json' }), ...headers },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const text = await response.text();
  return { status: response.status, body: text ? JSON.parse(text) : null };
};

let ws;
try {
  const health = await request('GET', '/healthz');
  assert.equal(health.status, 200);
  assert.equal(health.body.ok, true);
  assert.equal(health.body.transport, 'streamable-http');
  const startupOnlyHealth = await fetch(`http://127.0.0.1:${startupOnlyPort}/healthz`);
  assert.equal(startupOnlyHealth.status, 200);
  assert.equal((await startupOnlyHealth.json()).ok, true);

  const session = { 'x-syndocal-session': 'http-client' };
  const initialize = await request('POST', '/rpc', { jsonrpc: '2.0', id: 1, method: 'initialize', params: { protocolVersion: '2025-11-25', capabilities: {}, clientInfo: { name: 'check', version: '1' } } }, session);
  assert.equal(initialize.status, 200);
  assert.equal(initialize.body.result.protocolVersion, '2025-11-25');
  assert.equal((await request('POST', '/rpc', { jsonrpc: '2.0', method: 'notifications/initialized' }, session)).status, 202);
  const list = await request('POST', '/rpc', { jsonrpc: '2.0', id: 2, method: 'tools/list', params: {} }, session);
  assert.equal(list.status, 200);
  assert.equal(list.body.result.tools.length, 9);
  assert.equal(brokerRequests.at(-1).method, 'control_plane.get_capabilities');

  const rest = await request('POST', '/rest/tools/syndocal_get_runtime_status', {}, {});
  assert.equal(rest.status, 200);
  assert.equal(JSON.parse(rest.body.content[0].text).status, 'completed');
  assert.equal(brokerRequests.at(-1).method, 'runtime.get');

  ws = new WebSocket(`ws://127.0.0.1:${port}/ws`);
  await new Promise((resolve, reject) => {
    ws.addEventListener('open', resolve, { once: true });
    ws.addEventListener('error', reject, { once: true });
  });
  const wsRpc = (id, method, params) => new Promise((resolve, reject) => {
    const timeout = setTimeout(() => reject(new Error(`WebSocket response timeout: ${id}`)), 3000);
    const listener = (event) => {
      const value = JSON.parse(event.data);
      if (value.id !== id) return;
      clearTimeout(timeout); ws.removeEventListener('message', listener); resolve(value);
    };
    ws.addEventListener('message', listener);
    ws.send(JSON.stringify({ jsonrpc: '2.0', id, method, ...(params === undefined ? {} : { params }) }));
  });
  const wsInit = await wsRpc(1, 'initialize', { protocolVersion: '2025-11-25', capabilities: {}, clientInfo: { name: 'ws-check', version: '1' } });
  assert.equal(wsInit.result.protocolVersion, '2025-11-25');
  ws.send(JSON.stringify({ jsonrpc: '2.0', method: 'notifications/initialized' }));
  const wsList = await wsRpc(2, 'tools/list', {});
  assert.equal(wsList.result.tools.length, 9);
  assert.equal(brokerRequests.at(-1).method, 'control_plane.get_capabilities');
  ws.close();
  console.log('PASS HTTP health/JSON-RPC/REST and WebSocket transport groups; loopback fake broker only');
} finally {
  ws?.close();
  await new Promise((resolve) => httpServer.close(resolve));
  await new Promise((resolve) => startupOnlyServer.close(resolve));
  await new Promise((resolve) => broker.close(resolve));
  await fs.rm(temporary, { recursive: true, force: true });
}
