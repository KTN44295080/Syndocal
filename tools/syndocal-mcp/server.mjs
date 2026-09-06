import fs from 'node:fs/promises';
import path from 'node:path';
import net from 'node:net';
import { randomUUID } from 'node:crypto';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { pathToFileURL } from 'node:url';

const execFileAsync = promisify(execFile);
const VERSION = '2025-11-25';
const INPUT_LIMIT = 64 * 1024;
const RESPONSE_LIMIT = 256 * 1024;
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;
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
];

const projectFence = (value) => exact(value, ['project_epoch', 'project_revision', 'checkpoint_hash'])
  && integer(value.project_epoch) && integer(value.project_revision)
  && typeof value.checkpoint_hash === 'string' && /^[0-9a-f]{64}$/.test(value.checkpoint_hash);

function validateArguments(name, args) {
  if (name === 'syndocal_list_fixtures') return exact(args, []);
  if (name === 'syndocal_get_runtime_status') return exact(args, []);
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
    const key = { '--descriptor': 'descriptor', '--expected-executable': 'executable' }[argv[i]];
    if (!key || opts[key] || !argv[i + 1]) throw new Error('usage');
    opts[key] = argv[i + 1];
  }
  opts.descriptor ??= env.LOCALAPPDATA ? path.join(env.LOCALAPPDATA, 'jp.seraf.ktn.syndocal', 'agent-bridge-v1.json') : undefined;
  if (!opts.descriptor || !opts.executable || !path.isAbsolute(opts.descriptor) || !path.isAbsolute(opts.executable)) throw new Error('usage');
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
  if (!exact(descriptor, ['protocolVersion', 'port', 'token', 'instanceId', 'processId', 'executablePath'])
    || descriptor.protocolVersion !== 1 || !integer(descriptor.port) || descriptor.port < 1 || descriptor.port > 65535
    || !integer(descriptor.processId) || descriptor.processId < 1
    || typeof descriptor.token !== 'string' || descriptor.token.length < 16 || descriptor.token.length > 4096
    || typeof descriptor.instanceId !== 'string' || !/^[0-9a-f]{32}$/.test(descriptor.instanceId)
    || typeof descriptor.executablePath !== 'string' || !path.isAbsolute(descriptor.executablePath)) throw new Error('descriptor');
  const canonical = async (p) => (await fs.realpath(p)).toLowerCase();
  const expected = await canonical(options.executable);
  if (await canonical(descriptor.executablePath) !== expected || await canonical(await processExecutable(descriptor.processId)) !== expected) throw new Error('identity');
  return descriptor;
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
  const wire = Buffer.from(JSON.stringify({ token: descriptor.token, requestId, method, params }) + '\n');
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
        const redacted = redactCredential(value, descriptor.token);
        done(redacted);
      } catch { fail('Native response was not valid UTF-8 JSON.'); }
    });
  });
}

export function serve(options, input = process.stdin, output = process.stdout) {
  let initialized = false;
  let negotiated = false;
  let active = false;
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
    if (!exact(req, ['jsonrpc', 'id', 'method', 'params'], ['jsonrpc', 'method']) || req.jsonrpc !== '2.0' || typeof req.method !== 'string'
      || (Object.hasOwn(req, 'id') && !(typeof req.id === 'string' || (typeof req.id === 'number' && Number.isSafeInteger(req.id))))) {
      error(null, -32600, 'Invalid JSON-RPC request.'); return;
    }
    const notification = !Object.hasOwn(req, 'id');
    if (notification) {
      if (req.method === 'notifications/initialized' && negotiated && (req.params === undefined || exact(req.params, []))) initialized = true;
      return;
    }
    const success = (result) => emit({ jsonrpc: '2.0', id: req.id, result });
    const params = req.params ?? {};
    if (req.method === 'initialize') {
      if (negotiated || !exact(params, ['protocolVersion', 'capabilities', 'clientInfo', '_meta'], ['protocolVersion', 'capabilities', 'clientInfo'])
        || typeof params.protocolVersion !== 'string' || !record(params.capabilities)
        || !record(params.clientInfo) || typeof params.clientInfo.name !== 'string' || typeof params.clientInfo.version !== 'string') return error(req.id, -32602, 'Invalid initialize parameters or already initialized.');
      negotiated = true;
      return success({ protocolVersion: VERSION, capabilities: { tools: { listChanged: false } }, serverInfo: { name: 'syndocal-mcp', version: '1.0.0' } });
    }
    if (req.method === 'ping') {
      if (!exact(params, [])) return error(req.id, -32602, 'Ping takes no parameters.');
      return success({});
    }
    if (!initialized) return error(req.id, -32002, 'Initialize and send notifications/initialized first.');
    if (req.method === 'tools/list') {
      if (!exact(params, ['_meta'], [])) return error(req.id, -32602, 'Unexpected tools/list parameters.');
      return success({ tools: toolDefinitions });
    }
    if (req.method !== 'tools/call') return error(req.id, -32601, 'Method not found.');
    if (!exact(params, ['name', 'arguments', '_meta'], ['name']) || typeof params.name !== 'string'
      || !validateArguments(params.name, params.arguments ?? {})) return error(req.id, -32602, 'Unknown tool or invalid arguments. Use tools/list for the exact schema.');
    if (active) return success({ content: [{ type: 'text', text: 'Another native request is active. This request was not sent; wait for its response.' }], isError: true });
    active = true;
    try {
      const args = params.arguments ?? {};
      const mutation = params.name === 'syndocal_set_fixture_transform' || params.name === 'syndocal_set_video_blackout';
      const requestId = mutation ? args.requestId : randomUUID();
      const method = { syndocal_list_fixtures: 'fixtures.list', syndocal_get_fixture: 'fixtures.get', syndocal_set_fixture_transform: 'fixtures.set_transform', syndocal_set_video_blackout: 'output.set_video_blackout', syndocal_get_request_status: 'request.status', syndocal_get_runtime_status: 'runtime.get' }[params.name];
      const nativeParams = mutation ? Object.fromEntries(Object.entries(args).filter(([key]) => key !== 'requestId')) : args;
      const result = await nativeRequest(options, method, nativeParams, requestId, mutation);
      if (result.status !== 'completed') result.nextAction = 'Query syndocal_get_request_status with the original requestId. Do not automatically resubmit an unknown or pending mutation.';
      success({ content: [{ type: 'text', text: JSON.stringify(result) }], isError: result.status !== 'completed' || result.result?.ok !== true });
    } catch { success({ content: [{ type: 'text', text: 'Bridge request failed. No automatic retry was performed; query the original requestId.' }], isError: true }); }
    finally { active = false; }
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

if (process.argv[1] && import.meta.url === pathToFileURL(path.resolve(process.argv[1])).href) {
  try { serve(parseOptions(process.argv.slice(2))); }
  catch { process.stderr.write('Usage: node server.mjs --expected-executable <absolute path> [--descriptor <absolute path>]\n'); process.exitCode = 2; }
}
