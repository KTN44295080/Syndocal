import test from 'node:test';
import assert from 'node:assert/strict';
import { EventEmitter } from 'node:events';
import { PassThrough } from 'node:stream';
import * as fs from 'node:fs/promises';
import path from 'node:path';
import os from 'node:os';
import { fileURLToPath } from 'node:url';
import { selectDmg, compareVersions, validateInfo, parseLoadCommands, validateMachO } from './macos-artifact-policy.mjs';
import { createCommandRunner } from './macos-artifact-process.mjs';
import { cleanupSession } from './check-macos-artifact.mjs';
const exe = 'Contents/MacOS/syndocal', lib = 'Contents/Frameworks/libavcodec.62.dylib';
const image = (file, more = {}) => ({ path: file, architectures: ['arm64'], minimum: '12.0', rpaths: [], dependencies: [], ...more });
const graph = () => [image(exe, { rpaths: ['@executable_path/../Frameworks'], dependencies: ['@rpath/libavcodec.62.dylib'] }), image(lib)];
const load = `example with spaces:\nLoad command 0\n      cmd LC_BUILD_VERSION\n platform 1\n    minos 12.0\nLoad command 1\n      cmd LC_RPATH\n     path @executable_path/../Frameworks (offset 12)\nLoad command 2\n      cmd LC_LOAD_DYLIB\n     name @rpath/libavcodec.62.dylib (offset 24)\n`;
test('selects exactly one correctly versioned arm64 DMG', () => {
  assert.equal(selectDmg(['Syndocal_1.2.0-alpha.69_arm64.dmg', 'notes.txt'], '1.2.0-alpha.69'), 'Syndocal_1.2.0-alpha.69_arm64.dmg');
});
for (const names of [[], ['old.dmg'], ['Syndocal_v_x86_64.dmg'], ['one.dmg', 'two.dmg']]) {
  test(`rejects missing/stale/ambiguous DMG: ${names}`, () => assert.throws(() => selectDmg(names, 'v')));
}
test('compares numeric minimum OS versions', () => {
  assert.equal(compareVersions('12', '12.0.0'), 0); assert.equal(compareVersions('12.1', '12.0'), 1);
  assert.equal(compareVersions('11.9', '12'), -1);
  for (const value of [undefined, '', '12.beta', '12.0.0.1']) assert.throws(() => compareVersions(value, '12'));
});
test('validates bundle identity and minimum OS without treating it as runtime proof', () => {
  const expected = { version: '1.2.0-alpha.69', identifier: 'jp.seraf.ktn.syndocal', minimum: '12.0' };
  const info = { CFBundleExecutable: 'syndocal', CFBundleIdentifier: expected.identifier,
    CFBundleShortVersionString: expected.version, LSMinimumSystemVersion: '12.0' };
  validateInfo(info, expected);
  for (const field of Object.keys(info)) assert.throws(() => validateInfo({ ...info, [field]: 'wrong' }, expected));
});
test('parses load commands including CRLF and legacy minimum OS', () => {
  assert.equal(parseLoadCommands(load).minimum, '12.0');
  assert.deepEqual(parseLoadCommands(load), parseLoadCommands(load.replaceAll('\n', '\r\n')));
  assert.equal(parseLoadCommands(load.replace('LC_BUILD_VERSION\n platform 1\n    minos', 'LC_VERSION_MIN_MACOSX\n  version')).minimum, '12.0');
});
for (const broken of ['', load.replace('platform 1', 'platform 2'), load.replace('minos 12.0', 'minos ???'),
  load.replace('name @rpath/libavcodec.62.dylib (offset 24)', 'name ???'), load + 'Load command 3\n cmd LC_VERSION_MIN_MACOSX\n version 13.0\n']) {
  test(`rejects unparseable or contradictory Mach-O metadata ${broken.length}`, () => assert.throws(() => parseLoadCommands(broken)));
}
test('resolves bundled and dyld-cache system dependencies', () => {
  const images = graph(); images[1].dependencies.push('/usr/lib/libSystem.B.dylib');
  assert.equal(validateMachO(images, exe, '12.0').length, 2);
});
for (const [name, mutate] of [
  ['wrong architecture', xs => { xs[1].architectures = ['x86_64']; }],
  ['too-new library', xs => { xs[1].minimum = '13.0'; }],
  ['missing dependency', xs => { xs.pop(); }],
  ['external dependency', xs => { xs[1].dependencies = ['/opt/homebrew/lib/secret.dylib']; }],
  ['external rpath', xs => { xs[0].rpaths = ['/Users/runner/build/lib']; }],
  ['escaping rpath', xs => { xs[0].rpaths = ['@executable_path/../../../outside']; }],
  ['orphan binary', xs => { xs.push(image('Contents/MacOS/unreviewed')); }],
]) test(`rejects ${name}`, () => { const xs = graph(); mutate(xs); assert.throws(() => validateMachO(xs, exe, '12.0')); });
test('inherits parent runpaths and resolves loader-relative dependencies', () => {
  const xs = graph(); xs[1].dependencies = ['@loader_path/child.dylib'];
  xs.push(image('Contents/Frameworks/child.dylib', { dependencies: ['@rpath/libavcodec.62.dylib'] }));
  assert.equal(validateMachO(xs, exe, '12').length, 3);
});
test('rejects ambiguous rpath resolution and system-path traversal', () => {
  const xs = graph(); xs[0].rpaths.push('@executable_path/../Other');
  xs.push(image('Contents/Other/libavcodec.62.dylib'));
  assert.throws(() => validateMachO(xs, exe, '12'), /ambiguous/);
  assert.throws(() => validateMachO([image(exe, { dependencies: ['/usr/lib/../../tmp/evil.dylib'] })], exe, '12'));
});
const session = { root: '/owned', mount: '/owned/volume', mountAttempted: true };
function cleanupFixture() {
  let mounted = true; const calls = [];
  return { calls, hasUnreaped: () => false, isMounted: async () => mounted,
    detach: async target => { calls.push(['detach', target]); mounted = false; },
    remove: async target => { calls.push(['remove', target]); } };
}
test('cleanup detaches its exact mount then deletes only its owned directory', async () => {
  const io = cleanupFixture(); await cleanupSession(session, io);
  assert.deepEqual(io.calls, [['detach', '/owned/volume'], ['remove', '/owned']]);
});
for (const fault of ['unreaped', 'detach', 'unknown', 'still_mounted']) test(`cleanup preserves evidence on ${fault}`, async () => {
  const io = cleanupFixture();
  if (fault === 'unreaped') io.hasUnreaped = () => true;
  if (fault === 'detach') io.detach = async () => { throw new Error('detach failed'); };
  if (fault === 'unknown') io.isMounted = async () => { throw new Error('cannot inspect mounts'); };
  if (fault === 'still_mounted') io.isMounted = async () => true;
  await assert.rejects(cleanupSession(session, io)); assert.equal(io.calls.some(([op]) => op === 'remove'), false);
});
test('failed attach without a mount is cleaned without detach', async () => {
  const io = cleanupFixture(); io.isMounted = async () => false;
  await cleanupSession(session, io); assert.deepEqual(io.calls, [['remove', '/owned']]);
});
test('runs a real child and records close/exit evidence', async () => {
  const log = [], runner = createCommandRunner(log);
  assert.equal((await runner.run(process.execPath, ['-e', 'console.log("ok")'])).stdout.trim(), 'ok');
  assert.equal(log[0].reaped, true); assert.equal(runner.hasUnreaped(), false);
});
for (const [name, args, options] of [
  ['exit failure', ['-e', 'process.exit(7)'], {}],
  ['early smoke exit', ['-e', 'process.exit(0)'], { survivalMs: 500 }],
  ['timeout', ['-e', 'setInterval(()=>{},1000)'], { timeoutMs: 100 }],
  ['output overflow', ['-e', 'console.log("x".repeat(10000))'], { outputLimit: 16 }],
]) test(`real child failure is not a pass: ${name}`, async () => {
  const log = [], runner = createCommandRunner(log);
  await assert.rejects(runner.run(process.execPath, args, options));
  assert.equal(log[0].reaped, true); assert.equal(runner.hasUnreaped(), false);
});
test('survival smoke terminates and reaps its own child', async () => {
  const runner = createCommandRunner([]);
  const result = await runner.run(process.execPath, ['-e', 'setInterval(()=>{},1000)'], { survivalMs: 150 });
  assert.ok(result.survivalObservedMs >= 150); assert.equal(result.reaped, true);
  assert.equal(runner.hasUnreaped(), false);
});
test('missing command is a recorded failure', async () => {
  const log = [], runner = createCommandRunner(log);
  await assert.rejects(runner.run('syndocal-test-nonexistent-command-6524', [])); assert.equal(log.length, 1);
});
test('interruption fails the operation and reaps the owned child', async () => {
  const runner = createCommandRunner([]);
  const operation = runner.run(process.execPath, ['-e', 'setInterval(()=>{},1000)']);
  setTimeout(runner.interrupt, 50);
  await assert.rejects(operation, /interrupted/); assert.equal(runner.hasUnreaped(), false);
  await assert.rejects(runner.run(process.execPath, ['-e', 'process.exit(0)']), /interrupted/);
});
test('failed OS termination retains ownership until close, not a detached success', async () => {
  const child = Object.assign(new EventEmitter(), { pid: 123, exitCode: null, signalCode: null,
    stdout: new PassThrough(), stderr: new PassThrough(), kill: () => false });
  const runner = createCommandRunner([], () => child);
  await assert.rejects(runner.run('fixture', [], { timeoutMs: 10, graceMs: 10, reapMs: 30 }));
  assert.equal(runner.hasUnreaped(), true);
  child.emit('close', null, 'SIGKILL'); assert.equal(runner.hasUnreaped(), false);
});
test('CLI rejects non-hosted execution, writes failure/not_run evidence, preserves existing report', async () => {
  const tmp = await fs.mkdtemp(path.join(os.tmpdir(), 'syndocal-gate-test-'));
  try {
    const runner = createCommandRunner([]), report = path.join(tmp, 'report.json');
    const script = fileURLToPath(new URL('./check-macos-artifact.mjs', import.meta.url));
    const args = [script, '--dmg-dir', tmp, '--report', report];
    await assert.rejects(runner.run(process.execPath, args, { env: { ...process.env, GITHUB_ACTIONS: 'false' } }));
    const original = await fs.readFile(report, 'utf8'), value = JSON.parse(original);
    assert.equal(value.status, 'fail'); assert.equal(value.checks.environment.status, 'fail');
    assert.equal(value.checks.process_survival.status, 'not_run'); assert.equal(value.commands.length, 0);
    await assert.rejects(runner.run(process.execPath, args));
    assert.equal(await fs.readFile(report, 'utf8'), original);
  } finally { await fs.rm(tmp, { recursive: true }); }
});

import { hasMountedImage } from './macos-artifact-policy.mjs';
import { finalizeChecks } from './check-macos-artifact.mjs';
test('mount metadata resolves normalized exact paths and rejects unknown shapes', () => {
  const info = { images: [{ 'system-entities': [{ 'dev-entry': '/dev/disk4' }, { 'mount-point': '/private/tmp/owned/volume' }] }] };
  assert.equal(hasMountedImage(info, '/private/tmp/owned/./volume'), true);
  assert.equal(hasMountedImage(info, '/private/tmp/other/volume'), false);
  assert.equal(hasMountedImage({ images: [] }, '/private/tmp/owned/volume'), false);
  for (const broken of [null, {}, { images: [{}] }, { images: [null] },
    { images: [{ 'system-entities': [null] }] }, { images: [{ 'system-entities': [{ 'mount-point': 42 }] }] }]) {
    assert.throws(() => hasMountedImage(broken, '/private/tmp/owned/volume'));
  }
});
function completeReport() {
  const names = ['environment', 'dmg', 'image_integrity', 'extraction', 'bundle_info', 'macho', 'signature', 'process_survival', 'cleanup'];
  return { checks: { ...Object.fromEntries(names.map(name => [name, { status: 'pass' }])), checksum: { status: 'not_run' } } };
}
test('checksum success is required before publishing overall pass', async () => {
  const report = completeReport(); let writes = 0;
  await finalizeChecks(report, () => false, async () => { writes++; });
  assert.equal(writes, 1); assert.equal(report.status, 'pass'); assert.equal(report.checks.checksum.status, 'pass');
});
test('checksum publication failure is recorded as failure, not earlier success', async () => {
  const report = completeReport();
  await finalizeChecks(report, () => false, async () => { throw new Error('disk full'); });
  assert.equal(report.status, 'fail'); assert.equal(report.checks.checksum.status, 'fail');
  assert.match(report.error, /disk full/);
});

for (const fault of ['earlier', 'cleanup', 'interrupt_before', 'interrupt_during']) {
  test(`final evidence cannot pass after ${fault}`, async () => {
    const report = completeReport(); let writes = 0, interrupted = fault === 'interrupt_before';
    if (fault === 'earlier') report.error = 'Earlier failure';
    if (fault === 'cleanup') report.checks.cleanup.status = 'fail';
    await finalizeChecks(report, () => interrupted, async () => {
      writes++; if (fault === 'interrupt_during') interrupted = true;
    });
    assert.equal(report.status, 'fail');
    assert.equal(writes, fault === 'interrupt_during' ? 1 : 0);
  });
}

function terminalFixture(code, signal) {
  const child = Object.assign(new EventEmitter(), { pid: 123, exitCode: null, signalCode: null,
    stdout: new PassThrough(), stderr: new PassThrough() });
  child.kill = () => {
    queueMicrotask(() => {
      child.exitCode = code; child.signalCode = signal; child.emit('close', code, signal);
    });
    return true;
  };
  return child;
}
for (const [code, signal] of [[7, null], [null, 'SIGSEGV'], [null, 'SIGKILL']]) {
  test(`survival rejects abnormal termination: ${code}/${signal}`, async () => {
    const runner = createCommandRunner([], () => terminalFixture(code, signal));
    await assert.rejects(runner.run('fixture', [], { survivalMs: 10, graceMs: 10, reapMs: 50 }));
    assert.equal(runner.hasUnreaped(), false);
  });
}
for (const [code, signal] of [[0, null], [null, 'SIGTERM']]) {
  test(`survival accepts requested termination: ${code}/${signal}`, async () => {
    const runner = createCommandRunner([], () => terminalFixture(code, signal));
    const result = await runner.run('fixture', [], { survivalMs: 10, graceMs: 10, reapMs: 50 });
    assert.equal(result.reaped, true);
  });
}
