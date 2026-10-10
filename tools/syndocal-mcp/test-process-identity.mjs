import assert from 'node:assert/strict';
import test from 'node:test';
import { canonicalExecutable, parseMacosProcessPath, processExecutable } from './process-identity.mjs';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { spawn } from 'node:child_process';
import { readDescriptor } from './server.mjs';

const reply = (extra = {}) => Buffer.from(JSON.stringify({ schemaVersion: 1, processId: 123, executablePath: '/Applications/Syndocal.app/Contents/MacOS/syndocal', ...extra }));
test('only Windows folds executable path case', () => {
  assert.equal(canonicalExecutable('/App/Syndocal', 'win32'), '/app/syndocal');
  for (const platform of ['linux', 'darwin']) assert.notEqual(canonicalExecutable('/App/Syndocal', platform), canonicalExecutable('/app/syndocal', platform));
});
test('production inspection exits before Engine/Tauri initialization', async () => {
  const source = await fs.readFile(new URL('../../app/src-tauri/src/main.rs', import.meta.url), 'utf8');
  const main = source.slice(source.indexOf('\nfn main() {'));
  const start = main.indexOf('process_identity_cli::requested(env::args_os().skip(1))');
  const engine = main.indexOf('EngineHandle::start(');
  assert.ok(start >= 0 && engine > start);
  const inspection = main.slice(start, main.indexOf('video_recording_renderer_process::worker_cli_requested'));
  assert.ok(inspection.includes('process_identity_cli::run(env::args_os().skip(1))'));
  assert.ok(inspection.includes('std::process::exit(1);'));
  assert.ok(inspection.includes('return;'));
  assert.ok(!inspection.includes('tauri::Builder'));
});
test('macOS response binds exact PID, schema and complete absolute UTF-8 path', () => {
  assert.equal(parseMacosProcessPath(reply(), 123), '/Applications/Syndocal.app/Contents/MacOS/syndocal');
  for (const bytes of [reply({ processId: 124 }), reply({ schemaVersion: 2 }), reply({ extra: true }), reply({ executablePath: 'relative' }), reply({ executablePath: '/a\0b' }), reply({ executablePath: '/' + 'x'.repeat(4096) }), Buffer.from('null'), Buffer.from('[]'), Buffer.from('{'), Buffer.from([0xff]), Buffer.alloc(8193)]) {
    assert.throws(() => parseMacosProcessPath(bytes, 123));
  }
});
test('macOS launches only selected binary with fixed read-only args and bounded output', async () => {
  let calls = 0;
  const options = { platform: 'darwin', readlink: () => assert.fail('macOS must not use /proc'), run: async (file, args, opts) => {
    calls++; assert.equal(file, '/selected/Syndocal');
    assert.deepEqual(args, ['--syndocal-process-path-v1', '123']);
    assert.equal(opts.timeout, 10000); assert.equal(opts.maxBuffer, 8192); assert.equal(opts.encoding, 'buffer');
    return { stdout: reply() };
  } };
  for (let i = 0; i < 2; i++) await processExecutable(123, '/selected/Syndocal', options);
  assert.equal(calls, 2, 'no cached process proof');
});
test('inspection failure, unsupported platform and invalid PID never fall back', async () => {
  await assert.rejects(processExecutable(123, '/selected', { platform: 'darwin', run: async () => { throw new Error('gone'); } }), /gone/);
  const options = { platform: 'darwin', run: () => assert.fail('invalid PID must not spawn'), readlink: () => assert.fail('no fallback') };
  for (const pid of [0, -1, 1.1, '123', 2147483648, Infinity]) await assert.rejects(processExecutable(pid, '/selected', options));
  await assert.rejects(processExecutable(123, '/selected', { ...options, platform: 'freebsd' }), /unavailable/);
});
test('Linux keeps its exact kernel executable query', async () => {
  assert.equal(await processExecutable(123, '/unused', { platform: 'linux', run: () => assert.fail('no child'), readlink: async (name) => { assert.equal(name, '/proc/123/exe'); return '/Exact/App'; } }), '/Exact/App');
});
test('Linux rejects distinct case-only executables before broker forwarding', { skip: process.platform !== 'linux' }, async () => {
  const directory = await fs.mkdtemp(path.join(os.tmpdir(), 'syndocal-case-identity-'));
  let child;
  try {
    const actual = path.join(directory, 'Node'); const expected = path.join(directory, 'node');
    await fs.copyFile(process.execPath, actual); await fs.chmod(actual, 0o700);
    await fs.writeFile(expected, 'distinct executable');
    child = spawn(actual, ['-e', 'process.stdout.write("ready");setInterval(()=>{},1000)'], { stdio: ['ignore', 'pipe', 'pipe'] });
    await new Promise((resolve, reject) => { child.stdout.once('data', resolve); child.once('error', reject); child.once('exit', () => reject(new Error('identity fixture exited early'))); });
    const descriptor = path.join(directory, 'bridge.json');
    await fs.writeFile(descriptor, JSON.stringify({ protocolVersion: 1, port: 1, token: 'a'.repeat(32), sessionNonce: 'b'.repeat(64), instanceId: 'c'.repeat(32), processId: child.pid, executablePath: actual }));
    await assert.rejects(readDescriptor({ descriptor, executable: expected }), /identity/);
    assert.equal(await processExecutable(child.pid, expected), actual);
  } finally {
    if (child && child.exitCode === null && child.signalCode === null) { child.kill('SIGTERM'); await new Promise(resolve => child.once('exit', resolve)); }
    await fs.rm(directory, { recursive: true, force: true });
  }
});
