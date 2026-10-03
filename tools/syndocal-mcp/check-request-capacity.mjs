import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import net from 'node:net';
import { createHmac, randomBytes, randomUUID } from 'node:crypto';
import { dispatchRpc, nativeRequest, serveHttp } from './server.mjs';

// Real process verification, credential read, HMAC and owned loopback sockets;
// the private broker is a protocol fixture, not native-product acceptance.
const directory = await fs.mkdtemp(path.join(os.tmpdir(), 'syndocal-request-capacity-'));
const credential = randomBytes(32).toString('hex');
const token = randomBytes(32).toString('hex');
const sessionNonce = randomBytes(32).toString('hex');
const sockets = new Set();
const requests = [];
const errors = [];
let respond;
let httpServer, websocket;
const broker = net.createServer(socket => {
  sockets.add(socket);
  socket.on('close', () => sockets.delete(socket));
  socket.on('error', () => {});
  let bytes = '';
  socket.on('data', chunk => {
    bytes += chunk.toString();
    if (!bytes.includes('\n')) return;
    try {
      const request = JSON.parse(bytes.split('\n')[0]);
      assert.equal(request.token, token);
      assert.equal(request.auth.principalId, 'capacity-test');
      assert.equal(request.auth.principalIncarnation, 1);
      assert.equal(request.auth.proof, createHmac('sha256', Buffer.from(credential, 'hex'))
        .update(`${sessionNonce}\0${request.auth.clientNonce}\0${request.requestId}\0${request.method}`).digest('hex'));
      requests.push(request);
      respond(request, socket);
    } catch (error) { errors.push(error); socket.destroy(); }
  });
});
const pause = ms => new Promise(resolve => setTimeout(resolve, ms));
const reply = (request, socket, status, result) => socket.end(`${JSON.stringify({
  requestId: request.method === 'request.status' ? request.params.requestId : request.requestId,
  status, ...(result ? { result } : {}) })}\n`);
const complete = (request, socket) => reply(request, socket, 'completed', { ok: true });
const state = () => ({ initialized: true, negotiated: true, active: false });
const discovery = id => ({ jsonrpc: '2.0', id, method: 'tools/list', params: {} });
const call = id => ({ jsonrpc: '2.0', id, method: 'tools/call', params: { name: 'syndocal_get_runtime_status', arguments: {} } });
let groups = 0;
try {
  await new Promise(resolve => broker.listen(0, '127.0.0.1', resolve));
  const descriptor = path.join(directory, 'bridge.json');
  const credentialFile = path.join(directory, 'credential');
  await fs.writeFile(credentialFile, credential, { mode: 0o600 });
  await fs.writeFile(descriptor, JSON.stringify({ protocolVersion: 1, port: broker.address().port,
    token, sessionNonce, instanceId: randomBytes(16).toString('hex'), processId: process.pid,
    executablePath: process.execPath }));
  const options = { descriptor, executable: process.execPath, principalId: 'capacity-test',
    principalIncarnation: 1, credentialFile };
  respond = (request, socket) => setTimeout(() => complete(request, socket), 200);
  const shared = state();
  const replies = await Promise.all(Array.from({ length: 4 }, (_, i) => dispatchRpc(options, shared, discovery(i))));
  console.log(`Discovery burst: native dispatches=${requests.length}, completed=${replies.filter(r => r.result).length}`);
  assert.equal(requests.length, 1, 'Concurrent discovery must share the same pre-dispatch slot as tool calls');
  assert.equal(replies.filter(r => r.result).length, 1);
  assert.equal(replies.filter(r => r.error?.code === -32005).length, 3);
  assert.equal(shared.active, false);
  groups++;

  const beforePending = requests.length;
  respond = (request, socket) => request.method === 'control_plane.get_capabilities'
    ? reply(request, socket, 'pending') : complete(request, socket);
  assert.ok((await dispatchRpc(options, state(), discovery('pending-discovery'))).result);
  const pendingRequests = requests.slice(beforePending);
  assert.deepEqual(pendingRequests.map(r => r.method), ['control_plane.get_capabilities', 'request.status']);
  assert.equal(pendingRequests[1].params.requestId, pendingRequests[0].requestId);
  groups++;

  const beforeIncomplete = requests.length;
  respond = (request, socket) => reply(request, socket, 'pending');
  const incompleteState = state();
  const incomplete = await dispatchRpc(options, incompleteState, discovery('incomplete-discovery'));
  assert.equal(incomplete.error.code, -32003);
  assert.equal(incompleteState.active, false);
  assert.equal(requests.slice(beforeIncomplete).filter(r => r.method === 'control_plane.get_capabilities').length, 1);
  assert.equal(requests.length - beforeIncomplete, 5, 'Pending discovery has at most four status lookups');
  groups++;
  respond = (request, socket) => request.method === 'control_plane.get_capabilities'
    ? reply(request, socket, 'pending') : socket.end(`${JSON.stringify({ requestId: request.params.requestId,
      status: 'rejected', error: 'agent_principal_revoked' })}\n`);
  assert.equal((await dispatchRpc(options, state(), discovery('revoked-discovery'))).error.code, -32001);
  respond = (request, socket) => reply(request, socket, 'completed', { ok: false });
  assert.equal((await dispatchRpc(options, state(), discovery('failed-discovery'))).error.code, -32003);
  groups++;
  respond = (request, socket) => setTimeout(() => complete(request, socket), 200);

  const beforeCross = requests.length;
  const cross = state();
  const crossReplies = await Promise.all([dispatchRpc(options, cross, discovery('d')),
    dispatchRpc(options, cross, call('c'))]);
  assert.equal(requests.length, beforeCross + 1);
  const rejectedCall = JSON.parse(crossReplies[1].result.content[0].text);
  assert.equal(rejectedCall.status, 'rejected');
  assert.equal(rejectedCall.error, 'sidecar_overloaded');
  assert.equal(crossReplies[1].result.isError, true);
  assert.equal(cross.active, false);
  const statusId = randomUUID(), mutationId = randomUUID();
  const busy = { ...state(), active: true };
  for (const [name, args, expectedId] of [
    ['syndocal_get_request_status', { requestId: statusId }, statusId],
    ['syndocal_export_diagnostics', { requestId: mutationId, destination: path.join(directory, 'unsent.zip') }, mutationId],
  ]) {
    const response = await dispatchRpc(options, busy, { jsonrpc: '2.0', id: name,
      method: 'tools/call', params: { name, arguments: args } });
    const receipt = JSON.parse(response.result.content[0].text);
    assert.equal(receipt.requestId, expectedId);
    assert.equal(receipt.error, 'sidecar_overloaded');
  }
  assert.equal(requests.length, beforeCross + 1, 'Busy status/mutation never dispatch');
  groups++;

  const held = [];
  respond = (request, socket) => held.push({ request, socket });
  const beforeBurst = requests.length;
  // The first eight own work; all others must reject before filesystem/process
  // inspection or native connection. No mutation is retried or queued.
  const unreadableOptions = new Proxy(options, { get() { assert.fail('Overload must precede any descriptor, process or credential access'); } });
  const burst = Array.from({ length: 10_000 }, (_, i) => nativeRequest(i < 8 ? options : unreadableOptions, 'runtime.get', {}, randomUUID()));
  const excess = await Promise.all(burst.slice(8));
  assert.equal(excess.length, 9992);
  assert.ok(excess.every(r => r.status === 'rejected' && r.error === 'sidecar_overloaded'));
  const globalDiscovery = state();
  const globalResponse = await dispatchRpc(unreadableOptions, globalDiscovery, discovery('global-busy'));
  assert.equal(globalResponse.error.code, -32005, 'Global overload is not an authentication failure');
  assert.equal(globalDiscovery.active, false);
  const deadline = Date.now() + 15000;
  while (held.length < 8 && Date.now() < deadline) await pause(10);
  assert.equal(held.length, 8);
  assert.equal(requests.length, beforeBurst + 8);
  httpServer = serveHttp(options, { port: 0 });
  await new Promise(resolve => httpServer.once('listening', resolve));
  const port = httpServer.address().port;
  const post = async (route, body) => {
    const response = await fetch(`http://127.0.0.1:${port}${route}`, { method: 'POST',
      headers: { 'content-type': 'application/json', 'x-syndocal-session': 'capacity-http' },
      body: JSON.stringify(body), signal: AbortSignal.timeout(5000) });
    return response.status === 202 ? null : response.json();
  };
  const initialize = { jsonrpc: '2.0', id: 1, method: 'initialize', params: {
    protocolVersion: '2025-11-25', capabilities: {}, clientInfo: { name: 'capacity', version: '1' } } };
  assert.ok((await post('/rpc', initialize)).result);
  await post('/rpc', { jsonrpc: '2.0', method: 'notifications/initialized' });
  assert.equal((await post('/rpc', discovery(2))).error.code, -32005);
  const rest = await post('/rest/tools/syndocal_get_runtime_status', {});
  assert.equal(JSON.parse(rest.content[0].text).error, 'sidecar_overloaded');
  websocket = new WebSocket(`ws://127.0.0.1:${port}/ws`);
  await new Promise((resolve, reject) => {
    websocket.addEventListener('open', resolve, { once: true });
    websocket.addEventListener('error', reject, { once: true });
  });
  const wsRpc = request => new Promise((resolve, reject) => {
    const timer = setTimeout(() => { websocket.removeEventListener('message', listener); reject(new Error('Capacity WebSocket deadline')); }, 5000);
    const listener = event => {
      const response = JSON.parse(event.data);
      if (response.id !== request.id) return;
      clearTimeout(timer); websocket.removeEventListener('message', listener); resolve(response);
    };
    websocket.addEventListener('message', listener);
    websocket.send(JSON.stringify(request));
  });
  assert.ok((await wsRpc(initialize)).result);
  websocket.send(JSON.stringify({ jsonrpc: '2.0', method: 'notifications/initialized' }));
  assert.equal((await wsRpc(discovery(2))).error.code, -32005);
  assert.equal(requests.length, beforeBurst + 8, 'Independent HTTP/REST/WebSocket sessions share the process-wide budget');
  groups++;
  for (const item of held) complete(item.request, item.socket);
  assert.ok((await Promise.all(burst.slice(0, 8))).every(r => r.status === 'completed'));
  console.log('Global burst: 10000 intents, eight native dispatches, 9992 pre-dispatch rejections');
  groups++;

  respond = complete;
  assert.equal((await nativeRequest(options, 'runtime.get', {}, randomUUID())).status, 'completed');
  const missing = await nativeRequest({ ...options, descriptor: path.join(directory, 'missing') }, 'runtime.get', {}, randomUUID());
  assert.equal(missing.status, 'rejected');
  assert.equal((await nativeRequest(options, 'runtime.get', {}, randomUUID())).status, 'completed');
  groups++;

  const beforeLoss = requests.length;
  respond = (_request, socket) => socket.end();
  const lostId = randomUUID();
  const lost = await nativeRequest(options, 'diagnostics.export', { destination: path.join(directory, 'not-written.zip') }, lostId, true);
  assert.equal(lost.status, 'unknown');
  assert.equal(lost.requestId, lostId);
  assert.equal(requests.length, beforeLoss + 1, 'Lost mutation reply must never retry');
  respond = complete;
  assert.equal((await nativeRequest(options, 'runtime.get', {}, randomUUID())).status, 'completed');
  groups++;
  assert.deepEqual(errors, []);
  console.log(`PASS request capacity: ${groups} groups; bounded authenticated discovery, revocation/failure rejection, discovery/call exclusion, 10000 global intents, shared HTTP/REST/WebSocket budget, error/unknown release and no retries`);
} finally {
  if (websocket && websocket.readyState !== WebSocket.CLOSED) {
    await new Promise(resolve => { websocket.addEventListener('close', resolve, { once: true }); websocket.close(); });
  }
  if (httpServer) await new Promise(resolve => httpServer.close(resolve));
  for (const socket of sockets) socket.destroy();
  await new Promise(resolve => broker.close(resolve));
  assert.equal(path.dirname(path.resolve(directory)), path.resolve(os.tmpdir()));
  assert.ok(path.basename(directory).startsWith('syndocal-request-capacity-'));
  await fs.rm(directory, { recursive: true, force: true });
}
