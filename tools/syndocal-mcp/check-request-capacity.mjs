import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import net from 'node:net';
import { mock } from 'node:test';
import { setTimeout as realSetTimeout, clearTimeout as realClearTimeout } from 'node:timers';
import { performance } from 'node:perf_hooks';
import { createHmac, randomBytes, randomUUID } from 'node:crypto';
import { dispatchRpc, nativeRequest, serveHttp } from './server.mjs';
import { settleAuthenticatedDiscovery } from './authenticated-discovery.mjs';

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
let httpServer, websocket, restoreCapacityDeadlines;
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
const deferred = () => {
  let resolve;
  const promise = new Promise(done => { resolve = done; });
  return { promise, resolve };
};
const bounded = async promise => {
  let timer;
  try {
    return await Promise.race([promise, new Promise((_, reject) => {
      timer = realSetTimeout(() => reject(new Error('Capacity fixture barrier deadline')), 15000);
    })]);
  } finally { realClearTimeout(timer); }
};
// A capacity fixture owns its replies explicitly. Suspend only the native
// response deadline (server.mjs: 3000ms and its specific failure callback), so
// unrelated HTTP/fetch timers continue to run and slow process verification cannot
// release a slot before the transport assertions. Real connection deadlines,
// fixture watchdogs and every production timer remain unchanged outside here.
const controlCapacityDeadlines = () => {
  const pending = new Set();
  const timeout = mock.method(globalThis, 'setTimeout', (callback, delay, ...args) => {
    if (delay !== 3000 || !Function.prototype.toString.call(callback).includes('Native response deadline exceeded.')) {
      return realSetTimeout(callback, delay, ...args);
    }
    const handle = realSetTimeout(() => {}, 0);
    realClearTimeout(handle);
    pending.add(handle);
    return handle;
  });
  const clear = mock.method(globalThis, 'clearTimeout', handle => {
    pending.delete(handle);
    realClearTimeout(handle);
  });
  return {
    pending,
    restore() { timeout.mock.restore(); clear.mock.restore(); },
  };
};
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
  const discoveryEntered = deferred(), discoveryRelease = deferred();
  respond = async (request, socket) => {
    discoveryEntered.resolve();
    await discoveryRelease.promise;
    complete(request, socket);
  };
  const shared = state();
  const discoveryBurst = Array.from({ length: 4 }, (_, i) => dispatchRpc(options, shared, discovery(i)));
  await bounded(discoveryEntered.promise);
  assert.ok((await Promise.all(discoveryBurst.slice(1))).every(r => r.error?.code === -32005));
  discoveryRelease.resolve();
  const replies = await Promise.all(discoveryBurst);
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
  const incompleteRequests = requests.slice(beforeIncomplete);
  assert.ok(incompleteRequests.length >= 2 && incompleteRequests.length <= 5,
    'Pending discovery stops at four status lookups or the earlier 3s settlement deadline');
  assert.ok(incompleteRequests.slice(1).every(r => r.method === 'request.status'
    && r.params.requestId === incompleteRequests[0].requestId), 'Every lookup retains the original discovery identity');
  // Prove the count bound without Windows process-inspection latency consuming
  // the independent elapsed-time bound. Also prove that the elapsed bound wins.
  for (const elapsed of [0, 3001]) {
    let now = 0;
    const clock = mock.method(performance, 'now', () => now);
    const delay = mock.method(globalThis, 'setTimeout', (callback, milliseconds, ...args) => {
      assert.equal(milliseconds, 100, 'Discovery settlement uses its existing bounded poll interval');
      now += elapsed;
      queueMicrotask(() => callback(...args));
    });
    const methods = [];
    try {
      const receipt = await settleAuthenticatedDiscovery('bounded-discovery', async (method, params) => {
        methods.push(method);
        if (method === 'request.status') assert.equal(params.requestId, 'bounded-discovery');
        return { status: 'pending' };
      });
      assert.equal(receipt.status, 'pending');
      assert.deepEqual(methods, ['control_plane.get_capabilities',
        ...Array(elapsed === 0 ? 4 : 1).fill('request.status')]);
    } finally { clock.mock.restore(); delay.mock.restore(); }
  }
  groups++;
  respond = (request, socket) => request.method === 'control_plane.get_capabilities'
    ? reply(request, socket, 'pending') : socket.end(`${JSON.stringify({ requestId: request.params.requestId,
      status: 'rejected', error: 'agent_principal_revoked' })}\n`);
  assert.equal((await dispatchRpc(options, state(), discovery('revoked-discovery'))).error.code, -32001);
  respond = (request, socket) => reply(request, socket, 'completed', { ok: false });
  assert.equal((await dispatchRpc(options, state(), discovery('failed-discovery'))).error.code, -32003);
  groups++;
  const crossEntered = deferred(), crossRelease = deferred();
  respond = async (request, socket) => {
    crossEntered.resolve();
    await crossRelease.promise;
    complete(request, socket);
  };

  const beforeCross = requests.length;
  const cross = state();
  const crossBurst = [dispatchRpc(options, cross, discovery('d')), dispatchRpc(options, cross, call('c'))];
  await bounded(crossEntered.promise);
  const rejectedCross = await crossBurst[1];
  crossRelease.resolve();
  const crossReplies = [await crossBurst[0], rejectedCross];
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
  const allHeld = deferred();
  respond = (request, socket) => {
    held.push({ request, socket });
    if (held.length === 8) allHeld.resolve();
  };
  const capacityDeadlines = controlCapacityDeadlines();
  restoreCapacityDeadlines = () => capacityDeadlines.restore();
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
  await bounded(allHeld.promise);
  assert.equal(held.length, 8);
  assert.equal(capacityDeadlines.pending.size, 8, 'All eight socket owners retain their native response deadline');
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
  assert.equal(capacityDeadlines.pending.size, 0, 'Every completed socket clears its owned deadline');
  restoreCapacityDeadlines();
  restoreCapacityDeadlines = undefined;
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
  restoreCapacityDeadlines?.();
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
