import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { createServer } from 'node:net';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { randomUUID, randomBytes } from 'node:crypto';
import { fileURLToPath } from 'node:url';
import { parseOptions, toolDefinitions } from './server.mjs';

const temporary = await fs.mkdtemp(path.join(os.tmpdir(), 'syndocal-mcp-test-'));
const descriptorPath = path.join(temporary, 'bridge.json');
const token = 'test-secret-' + randomUUID();
const requests = [];
const sockets = new Set();
let respond = (request, socket) => socket.end(JSON.stringify({ requestId: request.requestId, status: 'completed', result: { ok: true, fixtures: [], project: { project_epoch: 1, project_revision: 1, checkpoint_hash: 'test' } } }) + '\n');
const broker = createServer((socket) => {
  sockets.add(socket); socket.on('close', () => sockets.delete(socket)); socket.on('error', () => {});
  let data = '';
  socket.on('data', (chunk) => {
    data += chunk;
    if (!data.includes('\n')) return;
    const request = JSON.parse(data.slice(0, data.indexOf('\n')));
    requests.push(request);
    assert.equal(request.token, token);
    respond(request, socket);
  });
});
await new Promise((resolve) => broker.listen(0, '127.0.0.1', resolve));
const descriptor = { protocolVersion: 1, port: broker.address().port, token, instanceId: randomBytes(16).toString('hex'), processId: process.pid, executablePath: process.execPath };
await fs.writeFile(descriptorPath, JSON.stringify(descriptor));
const script = fileURLToPath(new URL('./server.mjs', import.meta.url));
const child = spawn(process.execPath, [script, '--descriptor', descriptorPath, '--expected-executable', process.execPath], { windowsHide: true, stdio: ['pipe', 'pipe', 'pipe'] });
let stderr = '';
let stdout = '';
let pendingText = '';
const replies = [];
child.stderr.on('data', (chunk) => { stderr += chunk; });
child.stdout.on('data', (chunk) => {
  stdout += chunk; pendingText += chunk;
  let newline;
  while ((newline = pendingText.indexOf('\n')) >= 0) {
    replies.push(JSON.parse(pendingText.slice(0, newline)));
    pendingText = pendingText.slice(newline + 1);
  }
});
let counter = 0;
const delay = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
async function waitReply(id) {
  const deadline = Date.now() + 12000;
  while (Date.now() < deadline) {
    const index = replies.findIndex((reply) => reply.id === id);
    if (index >= 0) return replies.splice(index, 1)[0];
    assert.equal(child.exitCode, null, `child exited: ${stderr}`);
    await delay(10);
  }
  throw new Error(`Response deadline for ${id}`);
}
async function rpc(method, params) {
  const id = ++counter;
  child.stdin.write(JSON.stringify({ jsonrpc: '2.0', id, method, ...(params === undefined ? {} : { params }) }) + '\n');
  return waitReply(id);
}
const call = (name, args = {}) => rpc('tools/call', { name, arguments: args });
const decode = (reply) => JSON.parse(reply.result.content[0].text);
let checks = 0;
try {
  assert.throws(() => parseOptions([]));
  assert.throws(() => parseOptions(['--expected-executable', 'relative', '--descriptor', descriptorPath]));
  assert.throws(() => parseOptions(['--expected-executable', process.execPath, '--descriptor', descriptorPath, '--unknown', 'value']));
  checks++;
  assert.equal((await rpc('tools/list')).error.code, -32002);
  const initialize = await rpc('initialize', { protocolVersion: '2025-03-26', capabilities: {}, clientInfo: { name: 'test', version: '1' } });
  assert.equal(initialize.result.protocolVersion, '2025-11-25');
  child.stdin.write(JSON.stringify({ jsonrpc: '2.0', method: 'notifications/initialized' }) + '\n');
  assert.deepEqual((await rpc('ping')).result, {});
  const list = await rpc('tools/list');
  assert.equal(list.result.tools.length, 9);
  assert.ok(list.result.tools.every((tool) => tool.inputSchema.additionalProperties === false));
  assert.equal(toolDefinitions.find((tool) => tool.name === 'syndocal_execute_control_plane').inputSchema.properties.request.additionalProperties, true);
  const videoTool = list.result.tools.find((tool) => tool.name === 'syndocal_set_video_blackout');
  assert.deepEqual(Object.keys(videoTool.inputSchema.properties), ['requestId', 'enabled', 'expectedProject']);
  assert.equal(videoTool.inputSchema.properties.enabled.type, 'boolean');
  assert.equal(videoTool.inputSchema.properties.expectedProject.properties.checkpoint_hash.pattern, '^[0-9a-f]{64}$');
  checks++;
  assert.equal((await rpc('unknown')).error.code, -32601);
  assert.equal((await call('syndocal_list_fixtures', { extra: 1 })).error.code, -32602);
  assert.equal((await call('syndocal_get_runtime_status', { extra: 1 })).error.code, -32602);
  assert.equal((await call('syndocal_get_fixture', { fixtureId: '1' })).error.code, -32602);
  assert.equal((await call('syndocal_get_request_status', { requestId: [randomUUID()] })).error.code, -32602);
  assert.equal((await call('syndocal_get_request_status', { requestId: 'abcdefab-cdef-4abc-8def-abcdefabcdef'.toUpperCase() })).error.code, -32602);
  const videoMutation = {
    requestId: randomUUID(),
    enabled: false,
    expectedProject: { project_epoch: 1, project_revision: 2, checkpoint_hash: 'a'.repeat(64) },
  };
  for (const invalid of [
    { ...videoMutation, enabled: 'false' },
    { ...videoMutation, extra: true },
    { ...videoMutation, expectedProject: { ...videoMutation.expectedProject, extra: true } },
    { ...videoMutation, expectedProject: { ...videoMutation.expectedProject, project_epoch: Number.MAX_SAFE_INTEGER + 1 } },
    { ...videoMutation, expectedProject: { ...videoMutation.expectedProject, checkpoint_hash: 'A'.repeat(64) } },
    { ...videoMutation, expectedProject: { ...videoMutation.expectedProject, checkpoint_hash: 'a'.repeat(63) } },
  ]) assert.equal((await call('syndocal_set_video_blackout', invalid)).error.code, -32602);
  assert.equal(requests.length, 0);
  checks++;
  assert.equal(decode(await call('syndocal_list_fixtures')).status, 'completed');
  assert.equal(requests.at(-1).method, 'fixtures.list');
  await call('syndocal_get_fixture', { fixtureId: 7 });
  assert.deepEqual(requests.at(-1).params, { fixtureId: 7 });
  checks++;
  const runtime = await call('syndocal_get_runtime_status');
  assert.equal(runtime.result.isError, false);
  assert.equal(decode(runtime).status, 'completed');
  assert.equal(requests.at(-1).method, 'runtime.get');
  assert.deepEqual(requests.at(-1).params, {});
  const capabilities = await call('syndocal_get_control_plane_capabilities');
  assert.equal(capabilities.result.isError, false);
  assert.equal(decode(capabilities).status, 'completed');
  assert.equal(requests.at(-1).method, 'control_plane.get_capabilities');
  assert.deepEqual(requests.at(-1).params, {});
  const canonicalQuery = {
    requestId: randomUUID(),
    operationId: 'syndocal.query.control_plane.capabilities.v1',
    request: {},
  };
  const canonical = await call('syndocal_execute_control_plane', canonicalQuery);
  assert.equal(canonical.result.isError, false);
  assert.equal(decode(canonical).status, 'completed');
  assert.equal(requests.at(-1).requestId, canonicalQuery.requestId);
  assert.equal(requests.at(-1).method, 'control_plane.execute');
  assert.deepEqual(requests.at(-1).params, {
    operationId: canonicalQuery.operationId,
    request: {},
  });
  for (const invalid of [
    { ...canonicalQuery, operationId: 'syndocal.query.not_reviewed.v1' },
    { ...canonicalQuery, operationId: 'syndocal.query.control_plane.capabilities.v1', request: [] },
    { ...canonicalQuery, extra: true },
  ]) assert.equal((await call('syndocal_execute_control_plane', invalid)).error.code, -32602);
  checks++;
  const recording = await call('syndocal_get_recording_status');
  assert.equal(recording.result.isError, false);
  assert.equal(decode(recording).status, 'completed');
  assert.equal(requests.at(-1).method, 'recording.get_status');
  assert.deepEqual(requests.at(-1).params, {});
  checks++;
  respond = (req, socket) => socket.end(JSON.stringify({ requestId: req.requestId, status: 'completed', result: { ok: false, error: { code: 'lease_required', message: 'Video output ownership is not active.' } } }) + '\n');
  const videoFalse = await call('syndocal_set_video_blackout', videoMutation);
  assert.equal(videoFalse.result.isError, true);
  const videoFalseResult = decode(videoFalse);
  assert.equal(videoFalseResult.status, 'completed');
  assert.equal(videoFalseResult.result.ok, false);
  assert.equal(requests.at(-1).requestId, videoMutation.requestId);
  assert.equal(requests.at(-1).method, 'output.set_video_blackout');
  assert.deepEqual(requests.at(-1).params, {
    enabled: false,
    expectedProject: videoMutation.expectedProject,
  });
  checks++;
  const videoPendingId = randomUUID();
  const videoPendingMutation = { ...videoMutation, requestId: videoPendingId, enabled: true };
  respond = (req, socket) => socket.end(JSON.stringify({ requestId: req.requestId, status: 'pending' }) + '\n');
  const videoPending = await call('syndocal_set_video_blackout', videoPendingMutation);
  assert.equal(videoPending.result.isError, true);
  assert.equal(decode(videoPending).status, 'pending');
  const videoPendingRequests = requests.length;
  await delay(100);
  assert.equal(requests.length, videoPendingRequests);
  respond = (req, socket) => socket.end(JSON.stringify({ requestId: req.params.requestId, status: 'unknown' }) + '\n');
  const videoStatus = await call('syndocal_get_request_status', { requestId: videoPendingId });
  assert.equal(videoStatus.result.isError, true);
  assert.equal(decode(videoStatus).status, 'unknown');
  assert.equal(requests.at(-1).method, 'request.status');
  assert.equal(requests.at(-1).params.requestId, videoPendingId);
  assert.notEqual(requests.at(-1).requestId, videoPendingId);
  checks++;
  const mutationId = randomUUID();
  const mutation = { requestId: mutationId, fixtureId: 7, position: { x: 1, y: 2, z: 3 }, rotation: { pitch: 0, yaw: 45, roll: 0 }, expectedProject: { project_epoch: 1, project_revision: 2, checkpoint_hash: 'a'.repeat(64) } };
  respond = (req, socket) => socket.end(JSON.stringify({ requestId: req.requestId, status: 'pending' }) + '\n');
  const pending = await call('syndocal_set_fixture_transform', mutation);
  assert.equal(pending.result.isError, true);
  assert.equal(decode(pending).requestId, mutationId);
  assert.equal(requests.at(-1).requestId, mutationId);
  assert.equal(requests.at(-1).method, 'fixtures.set_transform');
  assert.equal(Object.hasOwn(requests.at(-1).params, 'requestId'), false);
  const sent = requests.length;
  await delay(100); assert.equal(requests.length, sent);
  checks++;
  respond = (req, socket) => socket.end(JSON.stringify({ requestId: req.params.requestId, status: 'unknown' }) + '\n');
  const status = await call('syndocal_get_request_status', { requestId: mutationId });
  assert.equal(status.result.isError, true);
  assert.equal(decode(status).requestId, mutationId);
  assert.equal(requests.at(-1).method, 'request.status');
  assert.equal(requests.at(-1).params.requestId, mutationId);
  assert.notEqual(requests.at(-1).requestId, mutationId);
  checks++;
  respond = (req, socket) => socket.end(JSON.stringify({ requestId: req.requestId, status: 'rejected', error: `do not leak ${token}` }) + '\n');
  const rejected = await call('syndocal_get_fixture', { fixtureId: 7 });
  assert.ok(rejected.result.content[0].text.includes('[REDACTED]'));
  assert.ok(!stdout.includes(token)); assert.ok(!stderr.includes(token));
  respond = (req, socket) => socket.end(JSON.stringify({ requestId: req.requestId, status: 'completed', result: { ok: false, error: { code: 'verification_failed', message: 'Fixture transform did not match.' } } }) + '\n');
  assert.equal((await call('syndocal_get_fixture', { fixtureId: 7 })).result.isError, true);
  checks++;
  await fs.writeFile(descriptorPath, JSON.stringify({ ...descriptor, executablePath: script }));
  const beforeWrong = requests.length;
  assert.equal(decode(await call('syndocal_list_fixtures')).status, 'rejected');
  assert.equal(requests.length, beforeWrong);
  await fs.writeFile(descriptorPath, JSON.stringify({ ...descriptor, instanceId: randomUUID() }));
  assert.equal(decode(await call('syndocal_list_fixtures')).status, 'rejected');
  assert.equal(requests.length, beforeWrong);
  await fs.writeFile(descriptorPath, JSON.stringify(descriptor));
  assert.match(descriptor.instanceId, /^[0-9a-f]{32}$/);
  checks++;
  respond = (_req, socket) => socket.end('{bad}\n');
  assert.equal(decode(await call('syndocal_get_fixture', { fixtureId: 7 })).status, 'rejected');
  respond = (_req, socket) => socket.end('x'.repeat(256 * 1024 + 1) + '\n');
  assert.equal(decode(await call('syndocal_get_fixture', { fixtureId: 7 })).status, 'rejected');
  checks++;
  respond = () => {};
  const timeoutId = randomUUID();
  const timeoutCall = call('syndocal_set_fixture_transform', { ...mutation, requestId: timeoutId });
  await delay(20);
  const overlap = await call('syndocal_list_fixtures');
  assert.equal(overlap.result.isError, true);
  const timeout = await timeoutCall;
  assert.equal(decode(timeout).status, 'unknown');
  assert.equal(decode(timeout).requestId, timeoutId);
  assert.equal(requests.filter((req) => req.requestId === timeoutId).length, 1);
  checks++;
  child.stdin.write('{bad}\n');
  assert.equal((await waitReply(null)).error.code, -32700);
  child.stdin.write('x'.repeat(65537));
  child.stdin.write('\n');
  assert.equal((await waitReply(null)).error.code, -32600);
  assert.deepEqual((await rpc('ping')).result, {});
  checks++;
  assert.ok(!stdout.includes(token)); assert.ok(!stderr.includes(token));
  console.log(`PASS ${checks} adapter integration groups; fake loopback only, no Syndocal/device calls`);
} finally {
  child.stdin.end();
  child.kill();
  for (const socket of sockets) socket.destroy();
  await new Promise((resolve) => broker.close(resolve));
  await fs.rm(temporary, { recursive: true, force: true });
}
