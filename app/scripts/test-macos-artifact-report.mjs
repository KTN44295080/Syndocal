import test from 'node:test';
import assert from 'node:assert/strict';
import * as fs from 'node:fs/promises';
import * as syncFs from 'node:fs';
import { setImmediate as nextTurn } from 'node:timers/promises';
import os from 'node:os';
import path from 'node:path';
import { reserveReport } from './macos-artifact-report.mjs';

async function fixture(run) {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), 'syndocal-report-test-'));
  try { await run(path.join(root, 'report.json')); }
  finally { await fs.rm(root, { recursive: true }); }
}
const absent = async file => assert.rejects(fs.lstat(file), { code: 'ENOENT' });
const candidate = () => ({ status: 'pass', interrupted: false, checks: { sample: { status: 'pass' } } });

test('final report appears complete, exclusively, with workflow-success boundary', () => fixture(async file => {
  const reservation = await reserveReport(file); await absent(file);
  const report = candidate();
  assert.equal(await reservation.publish(report, () => false), 'pass');
  const stored = JSON.parse(await fs.readFile(file, 'utf8'));
  assert.equal(stored.status, 'pass'); assert.equal(stored.publication.workflowSuccessRequired, true);
  assert.equal(stored.publication.finalPath, file);
  await absent(reservation.pending);
  await assert.rejects(reservation.publish(report, () => false), /already consumed/);
}));

for (const boundary of ['before', 'write', 'sync', 'close', 'yield']) {
  test(`interruption at ${boundary} publishes failure, never earlier pass`, () => fixture(async file => {
    let interrupted = boundary === 'before';
    const io = { fs: { ...fs }, sync: syncFs,
      nextTurn: async () => { await nextTurn(); if (boundary === 'yield') interrupted = true; } };
    io.fs.open = async (...args) => {
      const handle = await fs.open(...args);
      return {
        writeFile: async bytes => { await absent(file); await handle.writeFile(bytes); if (boundary === 'write') interrupted = true; },
        sync: async () => { await absent(file); await handle.sync(); if (boundary === 'sync') interrupted = true; },
        close: async () => { await handle.close(); await absent(file); if (boundary === 'close') interrupted = true; },
      };
    };
    const reservation = await reserveReport(file, io);
    const report = candidate(); await reservation.publish(report, () => interrupted);
    const stored = JSON.parse(await fs.readFile(file, 'utf8'));
    assert.equal(stored.status, 'fail'); assert.equal(stored.interrupted, true);
    assert.match(stored.error, /interrupted/); assert.equal(report.status, 'fail');
  }));
}

test('an earlier failed check cannot become a pass during publication', () => fixture(async file => {
  const report = { status: 'fail', error: 'cleanup failed' };
  await (await reserveReport(file)).publish(report, () => false);
  assert.equal(JSON.parse(await fs.readFile(file, 'utf8')).status, 'fail');
}));

for (const boundary of ['write', 'sync', 'close']) {
  test(`asynchronous ${boundary} failure leaves no final report`, () => fixture(async file => {
    const io = { fs: { ...fs }, sync: syncFs, nextTurn };
    io.fs.open = async (...args) => {
      const handle = await fs.open(...args);
      return {
        writeFile: async bytes => { if (boundary === 'write') throw new Error('write fixture'); await handle.writeFile(bytes); },
        sync: async () => { if (boundary === 'sync') throw new Error('sync fixture'); await handle.sync(); },
        close: async () => { await handle.close(); if (boundary === 'close') throw new Error('close fixture'); },
      };
    };
    const reservation = await reserveReport(file, io);
    await assert.rejects(reservation.publish(candidate(), () => false), /fixture/);
    await absent(file);
  }));
}
for (const boundary of ['writeFileSync', 'fsyncSync', 'linkSync']) {
  test(`commit ${boundary} failure leaves no final report`, () => fixture(async file => {
    const io = { fs, sync: { ...syncFs, [boundary]: () => { throw new Error(`${boundary} fixture`); } }, nextTurn };
    const reservation = await reserveReport(file, io);
    await assert.rejects(reservation.publish(candidate(), () => false), /publication failed/);
    await absent(file);
  }));
}

test('existing and concurrently created final evidence is never overwritten', () => fixture(async file => {
  await fs.writeFile(file, 'earlier evidence');
  await assert.rejects(reserveReport(file), /already exists/);
  assert.equal(await fs.readFile(file, 'utf8'), 'earlier evidence');
  await fs.unlink(file);
  const reservation = await reserveReport(file);
  await fs.writeFile(file, 'concurrent evidence');
  await assert.rejects(reservation.publish(candidate(), () => false), /publication failed/);
  assert.equal(await fs.readFile(file, 'utf8'), 'concurrent evidence');
}));

test('interruption observed immediately before final link rewrites prepared success', () => fixture(async file => {
  let interrupted = false;
  const io = { fs, nextTurn, sync: { ...syncFs,
    writeFileSync: (...args) => { syncFs.writeFileSync(...args); interrupted = true; },
  } };
  const reservation = await reserveReport(file, io);
  await reservation.publish(candidate(), () => interrupted);
  assert.equal(JSON.parse(await fs.readFile(file, 'utf8')).status, 'fail');
}));

test('unserializable evidence cannot publish a partial final record', () => fixture(async file => {
  const report = candidate(); report.cycle = report;
  await assert.rejects((await reserveReport(file)).publish(report, () => false), /circular/i);
  await absent(file);
}));

test('unused reservations hold no open file or pending artifact', () => fixture(async file => {
  let opens = 0;
  const io = { fs: { ...fs, open: (...args) => { opens++; return fs.open(...args); } }, sync: syncFs, nextTurn };
  const reservation = await reserveReport(file, io);
  assert.equal(opens, 0); await absent(reservation.pending); await absent(file);
}));

test('successful publication leaves no staging alias capable of mutating final evidence', () => fixture(async file => {
  const reservation = await reserveReport(file);
  await reservation.publish(candidate(), () => false);
  const final = await fs.readFile(file);
  await fs.writeFile(reservation.pending, 'unrelated subsequent file', { flag: 'wx' });
  assert.deepEqual(await fs.readFile(file), final);
}));

test('alias removal failure rejects publication even when a checks record is linked', () => fixture(async file => {
  const io = { fs, nextTurn, sync: { ...syncFs, unlinkSync: () => { throw new Error('unlink fixture'); } } };
  const report = candidate();
  const reservation = await reserveReport(file, io);
  await assert.rejects(reservation.publish(report, () => false), /publication failed/);
  assert.equal(report.status, 'fail');
  const stored = JSON.parse(await fs.readFile(file, 'utf8'));
  assert.equal(stored.publication.workflowSuccessRequired, true);
  assert.equal(stored.publication.stagingPolicy, 'success requires alias removal');
}));
