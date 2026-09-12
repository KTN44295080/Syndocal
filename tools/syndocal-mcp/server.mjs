import fs from 'node:fs/promises';
import path from 'node:path';
import net from 'node:net';
import http from 'node:http';
import { createHash, createHmac, randomBytes, randomUUID } from 'node:crypto';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { pathToFileURL } from 'node:url';

const execFileAsync = promisify(execFile);
const VERSION = '2025-11-25';
const INPUT_LIMIT = 64 * 1024;
const RESPONSE_LIMIT = 256 * 1024;
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;
const CANONICAL_OPERATION_IDS = new Set([
  'syndocal.query.control_plane.registry.v1',
  'syndocal.query.control_plane.canonical_registry.v3',
  'syndocal.query.control_plane.schemas.v1',
  'syndocal.query.control_plane.capabilities.v1',
  'syndocal.query.project.authority.v1',
  'syndocal.query.runtime.generations.v1',
  'syndocal.query.runtime.timeline.transport.authority.v1',
  'syndocal.query.runtime.timeline.loop.authority.v1',
  'syndocal.query.runtime.timeline.follow.abort.authority.v1',
  'syndocal.query.output.control.authority.v1',
  'syndocal.query.output.dsf2026_artnet_acceptance_probe.status.v1',
  'syndocal.query.output.display.add.authority.v1',
  'syndocal.query.output.ownership.v1',
  'syndocal.query.video.display_monitors.v1',
  'syndocal.query.video.camera_profiles.v1',
  'syndocal.query.video.camera_profile_probe.v1',
  'syndocal.query.video.output_window_observation.v1',
  'syndocal.query.events.observations.v1',
  'syndocal.effects.set_enabled.v1',
  'syndocal.runtime.timeline.transport.set_playing.v1',
  'syndocal.runtime.timeline.loop.commit.v1',
  'syndocal.runtime.timeline.follow.abort.v1',
  'syndocal.safety.blackout.engage.v1',
  'syndocal.output.blackout.release.v2',
  'syndocal.output.blackout.set.v2',
  'syndocal.output.ownership.arm.v2',
  'syndocal.output.standby.takeover.v2',
  'syndocal.output.display.add.v2',
  'syndocal.output.display.window.set_open.v2',
  'syndocal.output.video.composition.assign.v2',
  'syndocal.output.show_artnet_loopback_route.enable.v1',
  'syndocal.output.show_serial_dmx_s0_route.enable.v1',
  'syndocal.output.show_serial_dmx_s0_route.stop.v1',
  'syndocal.output.show_spout_outputs.enable.v2',
  'syndocal.output.show_spout_outputs.reset.v1',
  'syndocal.output.dsf2026_artnet_acceptance_probe.send.v1',
  'syndocal.output.dsf2026_artnet_acceptance_probe.reconcile.v1',
  'syndocal.output.enable.v2',
  'syndocal.output.lease.acquire.v2',
  'syndocal.output.lease.renew.v2',
  'syndocal.output.lease.recover.v2',
  'syndocal.output.lease.relinquish.v2',
  'syndocal.output.lease.force_transfer.v2',
  'syndocal.cue_lists.reorder.v1',
  'syndocal.cue_lists.rename.v1',
  'syndocal.cue_lists.delete.v1',
  'syndocal.scenes.create.v1',
]);
const record = (v) => v !== null && typeof v === 'object' && !Array.isArray(v);
const exact = (v, keys, required = keys) => record(v)
  && Object.keys(v).every((k) => keys.includes(k)) && required.every((k) => Object.hasOwn(v, k));
const integer = (v) => Number.isSafeInteger(v) && v >= 0;
const vector = (v, keys) => exact(v, keys) && keys.every((k) => typeof v[k] === 'number' && Number.isFinite(v[k]));
const schema = (properties) => ({ type: 'object', properties, required: Object.keys(properties), additionalProperties: false });
const number = { type: 'number' };
const identifier = { type: 'integer', minimum: 1, maximum: Number.MAX_SAFE_INTEGER };
const uuid = { type: 'string', format: 'uuid', pattern: UUID.source };
const position = schema({ x: number, y: number, z: number });
const rotation = schema({ pitch: number, yaw: number, roll: number });
const expectedProject = schema({ project_epoch: { type: 'integer', minimum: 0, maximum: Number.MAX_SAFE_INTEGER }, project_revision: { type: 'integer', minimum: 0, maximum: Number.MAX_SAFE_INTEGER }, checkpoint_hash: { type: 'string', minLength: 64, maxLength: 64, pattern: '^[0-9a-f]{64}$' } });
export const toolDefinitions = [
  { name: 'syndocal_list_fixtures', description: 'Read fixtures and current project identity from the selected running Syndocal instance.', inputSchema: schema({}), annotations: { readOnlyHint: true } },
  { name: 'syndocal_get_fixture', description: 'Read one fixture and current project identity.', inputSchema: schema({ fixtureId: identifier }), annotations: { readOnlyHint: true } },
  { name: 'syndocal_set_fixture_transform', description: 'Set a complete fixture transform against the exact observed project. Supply a new UUID for a new intent. If pending or unknown, query its status; never repeat the mutation with a new ID to recover a timeout.', inputSchema: schema({ requestId: uuid, fixtureId: identifier, position, rotation, expectedProject }), annotations: { readOnlyHint: false, destructiveHint: false, idempotentHint: true, openWorldHint: false } },
  { name: 'syndocal_set_video_blackout', description: 'Set Video BO against the exact observed project. This requires both lighting and video output ownership to already be active; disabling may reveal that existing output. It never arms, acquires, or enables output.', inputSchema: schema({ requestId: uuid, enabled: { type: 'boolean' }, expectedProject }), annotations: { readOnlyHint: false, destructiveHint: false, idempotentHint: true, openWorldHint: false } },
  { name: 'syndocal_get_request_status', description: 'Read a previously submitted request by its original UUID. Unknown does not mean safe to resend.', inputSchema: schema({ requestId: uuid }), annotations: { readOnlyHint: true } },
  { name: 'syndocal_get_runtime_status', description: 'Read bounded project runtime diagnostics, timeline state, video outputs, and a separate output-ownership observation from the selected running Syndocal instance. This never changes output state.', inputSchema: schema({}), annotations: { readOnlyHint: true } },
  { name: 'syndocal_get_control_plane_capabilities', description: 'Read the backend-owned canonical operation registry as a bounded capability inventory. It reports which operations have an explicit local-window adapter; FailClosed entries are discovery-only and cannot be invoked through MCP.', inputSchema: schema({}), annotations: { readOnlyHint: true } },
  { name: 'syndocal_get_recording_status', description: 'Read bounded video recording status from the selected running Syndocal instance. This never starts, stops, finalizes, or replaces a recording.', inputSchema: schema({}), annotations: { readOnlyHint: true } },
  { name: 'syndocal_execute_control_plane', description: 'Execute one of the 47 reviewed canonical backend operations through a static typed Tauri adapter. operationId must come from the capability registry and request must be that operation’s exact typed object. Unreviewed or FailClosed inventory entries are rejected. Supply a fresh requestId; if pending or unknown, query its status before any retry.', inputSchema: schema({ requestId: uuid, operationId: { type: 'string', minLength: 1, maxLength: 512 }, request: { type: 'object', additionalProperties: true } }), annotations: { readOnlyHint: false, destructiveHint: true, idempotentHint: false, openWorldHint: false } },
];

const projectFence = (value) => exact(value, ['project_epoch', 'project_revision', 'checkpoint_hash'])
  && integer(value.project_epoch) && integer(value.project_revision)
  && typeof value.checkpoint_hash === 'string' && /^[0-9a-f]{64}$/.test(value.checkpoint_hash);

function validateArguments(name, args) {
  if (name === 'syndocal_list_fixtures') return exact(args, []);
  if (name === 'syndocal_get_runtime_status') return exact(args, []);
  if (name === 'syndocal_get_control_plane_capabilities') return exact(args, []);
  if (name === 'syndocal_get_recording_status') return exact(args, []);
  if (name === 'syndocal_execute_control_plane') return exact(args, ['requestId', 'operationId', 'request'])
    && typeof args.requestId === 'string' && UUID.test(args.requestId)
    && typeof args.operationId === 'string' && args.operationId.length > 0 && args.operationId.length <= 512
    && CANONICAL_OPERATION_IDS.has(args.operationId) && record(args.request);
  if (name === 'syndocal_get_fixture') return exact(args, ['fixtureId']) && integer(args.fixtureId) && args.fixtureId > 0;
  if (name === 'syndocal_get_request_status') return exact(args, ['requestId']) && typeof args.requestId === 'string' && UUID.test(args.requestId);
  if (name === 'syndocal_set_video_blackout') return exact(args, ['requestId', 'enabled', 'expectedProject'])
    && typeof args.requestId === 'string' && UUID.test(args.requestId)
    && typeof args.enabled === 'boolean' && projectFence(args.expectedProject);
  if (name !== 'syndocal_set_fixture_transform' || !exact(args, ['requestId', 'fixtureId', 'position', 'rotation', 'expectedProject'])) return false;
  return typeof args.requestId === 'string' && UUID.test(args.requestId) && integer(args.fixtureId) && args.fixtureId > 0
    && vector(args.position, ['x', 'y', 'z']) && vector(args.rotation, ['pitch', 'yaw', 'roll'])
    && projectFence(args.expectedProject);
}

export function parseOptions(argv, env = process.env) {
  const opts = {};
  for (let i = 0; i < argv.length; i += 2) {
    const key = {
      '--descriptor': 'descriptor',
      '--expected-executable': 'executable',
      '--principal-id': 'principalId',
      '--principal-incarnation': 'principalIncarnation',
      '--credential-file': 'credentialFile',
      '--http-port': 'httpPort',
    }[argv[i]];
    if (!key || opts[key] || !argv[i + 1]) throw new Error('usage');
    opts[key] = argv[i + 1];
  }
  opts.descriptor ??= env.LOCALAPPDATA ? path.join(env.LOCALAPPDATA, 'jp.seraf.ktn.syndocal', 'agent-bridge-v1.json') : undefined;
  if (!opts.descriptor || !opts.executable || !path.isAbsolute(opts.descriptor) || !path.isAbsolute(opts.executable)) throw new Error('usage');
  const authOptions = [opts.principalId, opts.principalIncarnation, opts.credentialFile];
  if (authOptions.some((value) => value !== undefined) && authOptions.some((value) => value === undefined)) throw new Error('usage');
  if (opts.principalId !== undefined && (typeof opts.principalId !== 'string' || !/^[^\s]{1,128}$/.test(opts.principalId))) throw new Error('usage');
  if (opts.principalIncarnation !== undefined) {
    if (!/^\d+$/.test(opts.principalIncarnation)) throw new Error('usage');
    opts.principalIncarnation = Number(opts.principalIncarnation);
    if (!Number.isSafeInteger(opts.principalIncarnation) || opts.principalIncarnation < 1) throw new Error('usage');
  }
  if (opts.credentialFile !== undefined && !path.isAbsolute(opts.credentialFile)) throw new Error('usage');
  if (opts.httpPort !== undefined) {
    if (!/^\d+$/.test(opts.httpPort)) throw new Error('usage');
    opts.httpPort = Number(opts.httpPort);
    if (!Number.isSafeInteger(opts.httpPort) || opts.httpPort > 65535) throw new Error('usage');
  }
  return opts;
}

async function processExecutable(pid) {
  if (process.platform === 'win32') {
    const shell = path.join(process.env.SystemRoot ?? 'C:\\Windows', 'System32', 'WindowsPowerShell', 'v1.0', 'powershell.exe');
    // Fixed OS introspection only: no supplied command, path or expression is evaluated.
    const { stdout } = await execFileAsync(shell, ['-NoProfile', '-NonInteractive', '-Command',
      '[Console]::OutputEncoding=[System.Text.UTF8Encoding]::new(); $p=Get-CimInstance Win32_Process -Filter ("ProcessId=" + [int]$env:SYNDOCAL_BRIDGE_PID); if (!$p -or !$p.ExecutablePath) { exit 2 }; [Console]::Write($p.ExecutablePath)'],
    { windowsHide: true, timeout: 2000, maxBuffer: 8192, env: { ...process.env, SYNDOCAL_BRIDGE_PID: String(pid) } });
    return stdout.trim();
  }
  return fs.readlink(`/proc/${pid}/exe`);
}

export async function readDescriptor(options) {
  // Open/stat/read the same handle so replacing the descriptor cannot bypass its bound.
  const handle = await fs.open(options.descriptor, 'r');
  let descriptor;
  try {
    if ((await handle.stat()).size > INPUT_LIMIT) throw new Error('descriptor');
    const buffer = Buffer.alloc(INPUT_LIMIT + 1);
    const { bytesRead } = await handle.read(buffer, 0, buffer.length, 0);
    if (bytesRead > INPUT_LIMIT) throw new Error('descriptor');
    descriptor = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(buffer.subarray(0, bytesRead)));
  } finally { await handle.close(); }
  if (!exact(descriptor, ['protocolVersion', 'port', 'token', 'sessionNonce', 'instanceId', 'processId', 'executablePath'])
    || descriptor.protocolVersion !== 1 || !integer(descriptor.port) || descriptor.port < 1 || descriptor.port > 65535
    || !integer(descriptor.processId) || descriptor.processId < 1
    || typeof descriptor.token !== 'string' || descriptor.token.length < 16 || descriptor.token.length > 4096
    || typeof descriptor.sessionNonce !== 'string' || !/^[0-9a-f]{64}$/.test(descriptor.sessionNonce)
    || typeof descriptor.instanceId !== 'string' || !/^[0-9a-f]{32}$/.test(descriptor.instanceId)
    || typeof descriptor.executablePath !== 'string' || !path.isAbsolute(descriptor.executablePath)) throw new Error('descriptor');
  const canonical = async (p) => (await fs.realpath(p)).toLowerCase();
  const expected = await canonical(options.executable);
  if (await canonical(descriptor.executablePath) !== expected || await canonical(await processExecutable(descriptor.processId)) !== expected) throw new Error('identity');
  return descriptor;
}

async function readCredential(options) {
  if (options.principalId === undefined || options.principalIncarnation === undefined || options.credentialFile === undefined) {
    throw new Error('authentication configuration is required');
  }
  const value = (await fs.readFile(options.credentialFile, 'utf8')).trim();
  if (!/^[0-9a-f]{64}$/.test(value)) throw new Error('credential');
  return value;
}

function proofMessage(sessionNonce, clientNonce, requestId, method) {
  return `${sessionNonce}\0${clientNonce}\0${requestId}\0${method}`;
}

function redactCredential(value, token) {
  if (typeof value === 'string') return value.split(token).join('[REDACTED]');
  if (Array.isArray(value)) return value.map((entry) => redactCredential(entry, token));
  if (record(value)) return Object.fromEntries(Object.entries(value).map(([key, entry]) => [key.split(token).join('[REDACTED]'), redactCredential(entry, token)]));
  return value;
}

export async function nativeRequest(options, method, params, requestId, mutation = false) {
  const resultId = method === 'request.status' ? params.requestId : requestId;
  let descriptor;
  try { descriptor = await readDescriptor(options); }
  catch { return { requestId: resultId, status: 'rejected', error: 'Selected Syndocal descriptor/process/executable could not be verified. Start the expected executable and verify --descriptor and --expected-executable.' }; }
  let credential;
  try { credential = await readCredential(options); }
  catch { return { requestId: resultId, status: 'rejected', error: 'Sidecar principal authentication is not configured or its OS-protected credential could not be read.' }; }
  const clientNonce = randomBytes(32).toString('hex');
  const proof = createHmac('sha256', Buffer.from(credential, 'hex'))
    .update(proofMessage(descriptor.sessionNonce, clientNonce, requestId, method))
    .digest('hex');
  const wire = Buffer.from(JSON.stringify({
    token: descriptor.token,
    requestId,
    method,
    params,
    auth: {
      principalId: options.principalId,
      principalIncarnation: options.principalIncarnation,
      clientNonce,
      proof,
    },
  }) + '\n');
  if (wire.length > INPUT_LIMIT) return { requestId: resultId, status: 'rejected', error: 'Native request exceeds 64 KiB.' };
  return new Promise((resolve) => {
    let sent = false;
    let finished = false;
    let chunks = [];
    let size = 0;
    const socket = net.createConnection({ host: '127.0.0.1', port: descriptor.port });
    let timer = setTimeout(() => fail('Connection deadline exceeded.'), 1000);
    function done(value) {
      if (finished) return;
      finished = true;
      clearTimeout(timer);
      socket.destroy();
      resolve(value);
    }
    function fail(reason) {
      done({ requestId: resultId, status: mutation && sent ? 'unknown' : 'rejected', error: `${reason} No automatic retry was performed. Query syndocal_get_request_status with the original requestId before any further mutation.` });
    }
    socket.on('connect', () => {
      clearTimeout(timer);
      timer = setTimeout(() => fail('Native response deadline exceeded.'), 3000);
      sent = true;
      socket.write(wire);
    });
    socket.on('error', () => fail('Native bridge connection failed.'));
    socket.on('end', () => fail('Native bridge closed without a complete response.'));
    socket.on('data', (chunk) => {
      const newline = chunk.indexOf(10);
      const part = newline < 0 ? chunk : chunk.subarray(0, newline);
      size += part.length;
      if (size > RESPONSE_LIMIT) return fail('Native response exceeds 256 KiB.');
      chunks.push(part);
      if (newline < 0) return;
      try {
        const text = new TextDecoder('utf-8', { fatal: true }).decode(Buffer.concat(chunks));
        const value = JSON.parse(text);
        if (!exact(value, ['requestId', 'status', 'result', 'error'], ['requestId', 'status']) || value.requestId !== resultId
          || !['pending', 'completed', 'unknown', 'rejected'].includes(value.status)
          || (value.error !== undefined && typeof value.error !== 'string')) return fail('Native response contract mismatch.');
        // Treat broker payloads as data and redact the credential even on a malicious echo.
        const redacted = redactCredential(redactCredential(value, descriptor.token), credential);
        done(redacted);
      } catch { fail('Native response was not valid UTF-8 JSON.'); }
    });
  });
}

export async function dispatchRpc(options, state, req) {
  if (!exact(req, ['jsonrpc', 'id', 'method', 'params'], ['jsonrpc', 'method']) || req.jsonrpc !== '2.0' || typeof req.method !== 'string'
    || (Object.hasOwn(req, 'id') && !(typeof req.id === 'string' || (typeof req.id === 'number' && Number.isSafeInteger(req.id))))) {
    return { jsonrpc: '2.0', id: null, error: { code: -32600, message: 'Invalid JSON-RPC request.' } };
  }
  const notification = !Object.hasOwn(req, 'id');
  if (notification) {
    if (req.method === 'notifications/initialized' && state.negotiated && (req.params === undefined || exact(req.params, []))) state.initialized = true;
    return null;
  }
  const error = (code, message) => ({ jsonrpc: '2.0', id: req.id, error: { code, message } });
  const success = (result) => ({ jsonrpc: '2.0', id: req.id, result });
  const params = req.params ?? {};
  if (req.method === 'initialize') {
    if (state.negotiated || !exact(params, ['protocolVersion', 'capabilities', 'clientInfo', '_meta'], ['protocolVersion', 'capabilities', 'clientInfo'])
      || typeof params.protocolVersion !== 'string' || !record(params.capabilities)
      || !record(params.clientInfo) || typeof params.clientInfo.name !== 'string' || typeof params.clientInfo.version !== 'string') {
      return error(-32602, 'Invalid initialize parameters or already initialized.');
    }
    state.negotiated = true;
    return success({ protocolVersion: VERSION, capabilities: { tools: { listChanged: false } }, serverInfo: { name: 'syndocal-mcp', version: '1.0.0' } });
  }
  if (req.method === 'ping') {
    if (!exact(params, [])) return error(-32602, 'Ping takes no parameters.');
    return success({});
  }
  if (!state.initialized) return error(-32002, 'Initialize and send notifications/initialized first.');
  if (req.method === 'tools/list') {
    if (!exact(params, ['_meta'], [])) return error(-32602, 'Unexpected tools/list parameters.');
    const authentication = await nativeRequest(options, 'control_plane.get_capabilities', {}, randomUUID());
    if (authentication.status !== 'completed') return error(-32001, 'Authenticated principal is required before tool discovery.');
    return success({ tools: toolDefinitions });
  }
  if (req.method !== 'tools/call') return error(-32601, 'Method not found.');
  if (!exact(params, ['name', 'arguments', '_meta'], ['name']) || typeof params.name !== 'string'
    || !validateArguments(params.name, params.arguments ?? {})) return error(-32602, 'Unknown tool or invalid arguments. Use tools/list for the exact schema.');
  if (state.active) return success({ content: [{ type: 'text', text: 'Another native request is active. This request was not sent; wait for its response.' }], isError: true });
  state.active = true;
  try {
    const args = params.arguments ?? {};
    const mutation = params.name === 'syndocal_set_fixture_transform' || params.name === 'syndocal_set_video_blackout' || params.name === 'syndocal_execute_control_plane';
    const requestId = mutation ? args.requestId : randomUUID();
    const method = { syndocal_list_fixtures: 'fixtures.list', syndocal_get_fixture: 'fixtures.get', syndocal_set_fixture_transform: 'fixtures.set_transform', syndocal_set_video_blackout: 'output.set_video_blackout', syndocal_get_request_status: 'request.status', syndocal_get_runtime_status: 'runtime.get', syndocal_get_control_plane_capabilities: 'control_plane.get_capabilities', syndocal_get_recording_status: 'recording.get_status' }[params.name];
    const nativeMethod = params.name === 'syndocal_execute_control_plane' ? 'control_plane.execute' : method;
    const nativeParams = params.name === 'syndocal_execute_control_plane'
      ? { operationId: args.operationId, request: args.request }
      : mutation ? Object.fromEntries(Object.entries(args).filter(([key]) => key !== 'requestId')) : args;
    const result = await nativeRequest(options, nativeMethod, nativeParams, requestId, mutation);
    if (result.status !== 'completed') result.nextAction = 'Query syndocal_get_request_status with the original requestId. Do not automatically resubmit an unknown or pending mutation.';
    return success({ content: [{ type: 'text', text: JSON.stringify(result) }], isError: result.status !== 'completed' || result.result?.ok !== true });
  } catch {
    return success({ content: [{ type: 'text', text: 'Bridge request failed. No automatic retry was performed; query the original requestId.' }], isError: true });
  } finally { state.active = false; }
}

export function serve(options, input = process.stdin, output = process.stdout) {
  const state = { initialized: false, negotiated: false, active: false };
  let tail = Buffer.alloc(0);
  let discarding = false;
  function emit(message) {
    let encoded = JSON.stringify(message);
    if (Buffer.byteLength(encoded) > RESPONSE_LIMIT) encoded = JSON.stringify({ jsonrpc: '2.0', id: message.id ?? null, error: { code: -32603, message: 'Response exceeds MCP limit. Use syndocal_get_fixture for a smaller result.' } });
    if (!output.write(encoded + '\n')) input.pause();
  }
  output.on('drain', () => input.resume());
  const error = (id, code, message) => emit({ jsonrpc: '2.0', id, error: { code, message } });
  async function message(line) {
    let req;
    try { req = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(line)); }
    catch { error(null, -32700, 'Invalid UTF-8 JSON.'); return; }
    const response = await dispatchRpc(options, state, req);
    if (response) emit(response);
  }
  input.on('data', (chunk) => {
    // Process raw bytes; a multi-byte UTF-8 character may span chunks.
    let remaining = Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk);
    while (remaining.length) {
      const newline = remaining.indexOf(10);
      const segment = newline < 0 ? remaining : remaining.subarray(0, newline);
      if (!discarding && tail.length + segment.length > INPUT_LIMIT) {
        tail = Buffer.alloc(0); discarding = true; error(null, -32600, 'Input frame exceeds 64 KiB.');
      }
      if (!discarding) tail = Buffer.concat([tail, segment]);
      if (newline < 0) break;
      if (!discarding) void message(tail);
      tail = Buffer.alloc(0); discarding = false;
      remaining = remaining.subarray(newline + 1);
    }
  });
  input.on('end', () => { if (tail.length && !discarding) error(null, -32700, 'Input ended before newline.'); });
}

const HTTP_CONNECTION_LIMIT = 16;
const HTTP_SESSION_LIMIT = 64;
const WEBSOCKET_CONNECTION_LIMIT = 8;

function loopbackHost(host) {
  return host === '127.0.0.1' || host === '::1';
}

function boundedHttpBody(request) {
  return new Promise((resolve, reject) => {
    const chunks = [];
    let size = 0;
    let rejected = false;
    request.on('data', (chunk) => {
      if (rejected) return;
      size += chunk.length;
      if (size > INPUT_LIMIT) {
        rejected = true;
        request.resume();
        reject(new Error('body-too-large'));
        return;
      }
      chunks.push(chunk);
    });
    request.on('end', () => {
      if (!rejected) resolve(Buffer.concat(chunks));
    });
    request.on('error', (error) => {
      if (!rejected) { rejected = true; reject(error); }
    });
  });
}

function httpJson(response, status, value) {
  let body = JSON.stringify(value);
  if (Buffer.byteLength(body) > RESPONSE_LIMIT) {
    status = 500;
    body = JSON.stringify({ error: 'Response exceeds 256 KiB.' });
  }
  response.writeHead(status, { 'content-type': 'application/json; charset=utf-8', 'content-length': Buffer.byteLength(body), 'cache-control': 'no-store' });
  response.end(body);
}

function websocketFrame(payload, opcode = 1) {
  const length = payload.length;
  if (length > RESPONSE_LIMIT) throw new Error('frame-too-large');
  if (length < 126) return Buffer.concat([Buffer.from([0x80 | opcode, length]), payload]);
  if (length <= 0xffff) {
    const header = Buffer.alloc(4);
    header[0] = 0x80 | opcode; header[1] = 126; header.writeUInt16BE(length, 2);
    return Buffer.concat([header, payload]);
  }
  const header = Buffer.alloc(10);
  header[0] = 0x80 | opcode; header[1] = 127; header.writeBigUInt64BE(BigInt(length), 2);
  return Buffer.concat([header, payload]);
}

function websocketClose(socket, code, reason = '') {
  if (socket.destroyed) return;
  const detail = Buffer.from(reason, 'utf8').subarray(0, 120);
  socket.end(websocketFrame(Buffer.concat([Buffer.from([(code >> 8) & 0xff, code & 0xff]), detail]), 8));
}

function attachWebSocket(socket, options, onClose) {
  const state = { initialized: false, negotiated: false, active: false };
  let tail = Buffer.alloc(0);
  let closed = false;
  let consuming = false;
  const send = (response) => {
    if (closed || !response) return;
    try {
      const payload = Buffer.from(JSON.stringify(response));
      socket.write(websocketFrame(payload));
    } catch { websocketClose(socket, 1011, 'Response encoding failed.'); }
  };
  const consume = async () => {
    while (tail.length >= 2 && !closed) {
      const first = tail[0];
      const second = tail[1];
      if (!(first & 0x80) || (first & 0x70) !== 0) return websocketClose(socket, 1002, 'Fragmentation is not supported.');
      const opcode = first & 0x0f;
      const masked = Boolean(second & 0x80);
      let length = second & 0x7f;
      let headerLength = 2;
      if (length === 126) {
        if (tail.length < 4) return;
        length = tail.readUInt16BE(2); headerLength = 4;
      } else if (length === 127) {
        if (tail.length < 10) return;
        const value = tail.readBigUInt64BE(2);
        if (value > BigInt(INPUT_LIMIT)) return websocketClose(socket, 1009, 'Frame exceeds 64 KiB.');
        length = Number(value); headerLength = 10;
      }
      if (!masked) return websocketClose(socket, 1002, 'Client frames must be masked.');
      if (length > INPUT_LIMIT) return websocketClose(socket, 1009, 'Frame exceeds 64 KiB.');
      const frameLength = headerLength + 4 + length;
      if (tail.length < frameLength) return;
      const mask = tail.subarray(headerLength, headerLength + 4);
      const payload = Buffer.from(tail.subarray(headerLength + 4, frameLength));
      tail = tail.subarray(frameLength);
      for (let index = 0; index < payload.length; index += 1) payload[index] ^= mask[index % 4];
      if (opcode === 8) { websocketClose(socket, 1000); return; }
      if (opcode === 9) { socket.write(websocketFrame(payload, 10)); continue; }
      if (opcode !== 1) return websocketClose(socket, 1003, 'Only text frames are supported.');
      let request;
      try { request = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(payload)); }
      catch { send({ jsonrpc: '2.0', id: null, error: { code: -32700, message: 'Invalid UTF-8 JSON.' } }); continue; }
      send(await dispatchRpc(options, state, request));
    }
  };
  const pump = () => {
    if (consuming || closed) return;
    consuming = true;
    void consume().finally(() => {
      consuming = false;
      if (tail.length && !closed) pump();
    });
  };
  socket.on('data', (chunk) => {
    if (closed) return;
    tail = Buffer.concat([tail, Buffer.isBuffer(chunk) ? chunk : Buffer.from(chunk)]);
    if (tail.length > INPUT_LIMIT + 14) return websocketClose(socket, 1009, 'Frame exceeds 64 KiB.');
    pump();
  });
  const close = () => { closed = true; onClose(); };
  socket.on('close', close); socket.on('error', close);
  socket.setNoDelay(true);
}

export function serveHttp(options, { host = '127.0.0.1', port = 0 } = {}) {
  if (!loopbackHost(host) || !Number.isSafeInteger(port) || port < 0 || port > 65535) throw new Error('loopback-only HTTP transport');
  const sessions = new Map();
  const websockets = new Set();
  const server = http.createServer(async (request, response) => {
    const url = new URL(request.url ?? '/', `http://${host}`);
    if (request.method === 'GET' && url.pathname === '/healthz') return httpJson(response, 200, { ok: true, protocolVersion: VERSION, transport: 'streamable-http' });
    if (request.method !== 'POST' || (url.pathname !== '/rpc' && !url.pathname.startsWith('/rest/tools/'))) return httpJson(response, 404, { error: 'Not found.' });
    let body;
    try {
      const bytes = await boundedHttpBody(request);
      body = bytes.length ? JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(bytes)) : {};
    } catch (error) {
      return httpJson(response, error?.message === 'body-too-large' ? 413 : 400, { error: 'Request body must be bounded UTF-8 JSON.' });
    }
    let sessionId = request.headers['x-syndocal-session'];
    if (Array.isArray(sessionId)) sessionId = sessionId[0];
    if (sessionId === undefined && url.pathname.startsWith('/rest/tools/')) sessionId = 'rest-default';
    if (typeof sessionId !== 'string' || !/^[A-Za-z0-9._-]{1,128}$/.test(sessionId)) return httpJson(response, 400, { error: 'X-Syndocal-Session is required and must be bounded.' });
    let state = sessions.get(sessionId);
    if (!state) {
      if (sessions.size >= HTTP_SESSION_LIMIT) return httpJson(response, 429, { error: 'overloaded' });
      state = { initialized: false, negotiated: false, active: false, touched: Date.now() };
      sessions.set(sessionId, state);
    }
    state.touched = Date.now();
    if (url.pathname.startsWith('/rest/tools/')) {
      const name = decodeURIComponent(url.pathname.slice('/rest/tools/'.length));
      state.negotiated = true; state.initialized = true;
      const rpc = { jsonrpc: '2.0', id: randomUUID(), method: 'tools/call', params: { name, arguments: body } };
      const result = await dispatchRpc(options, state, rpc);
      return httpJson(response, result?.error ? 400 : 200, result?.error ?? result?.result ?? { error: 'Empty response.' });
    }
    let result;
    try { result = await dispatchRpc(options, state, body); }
    catch { return httpJson(response, 500, { error: 'Request processing failed.' }); }
    if (!result) return response.writeHead(202).end();
    return httpJson(response, 200, result);
  });
  server.maxConnections = HTTP_CONNECTION_LIMIT;
  server.keepAliveTimeout = 5000;
  server.headersTimeout = 5000;
  server.requestTimeout = 5000;
  server.on('upgrade', (request, socket) => {
    const url = new URL(request.url ?? '/', `http://${host}`);
    const key = request.headers['sec-websocket-key'];
    if (url.pathname !== '/ws' || request.headers.upgrade?.toLowerCase() !== 'websocket' || request.headers['sec-websocket-version'] !== '13' || typeof key !== 'string' || websockets.size >= WEBSOCKET_CONNECTION_LIMIT) {
      socket.destroy(); return;
    }
    const accept = createHash('sha1').update(`${key}258EAFA5-E914-47DA-95CA-C5AB0DC85B11`).digest('base64');
    socket.write(`HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Accept: ${accept}\r\n\r\n`);
    websockets.add(socket);
    attachWebSocket(socket, options, () => websockets.delete(socket));
  });
  server.listen(port, host);
  return server;
}

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  try {
    const options = parseOptions(process.argv.slice(2));
    if (options.httpPort !== undefined) serveHttp(options, { port: options.httpPort });
    else serve(options);
  }
  catch { process.stderr.write('Usage: node server.mjs --expected-executable <absolute path> [--descriptor <absolute path>] [--principal-id <id> --principal-incarnation <n> --credential-file <absolute path>] [--http-port <0-65535>]\n'); process.exitCode = 2; }
}
