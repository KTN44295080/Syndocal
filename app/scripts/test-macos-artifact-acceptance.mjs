import test from 'node:test';
import assert from 'node:assert/strict';
import * as fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { createHash, randomUUID } from 'node:crypto';
import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { fileURLToPath } from 'node:url';
import { ARTIFACT_CHECKS } from './macos-artifact-policy.mjs';
import { validateAcceptanceReport, verifyAcceptanceFiles } from './macos-artifact-acceptance.mjs';

function fixture(root = path.resolve('fixture')) {
  const expected = { validationOutcome: 'success', commit: 'a'.repeat(40), runId: '123',
    runAttempt: '2', version: '1.2.0-test.1', reportPath: path.join(root, 'report.json'), dmgDirectory: root };
  const payload = Buffer.from('fixture image bytes; not an actual DMG');
  const name = `Syndocal_${expected.version}_arm64.dmg`;
  const dmg = { name, path: path.join(root, name), size: payload.length,
    sha256: createHash('sha256').update(payload).digest('hex') };
  const report = { schemaVersion: 1, status: 'pass', interrupted: false,
    commit: expected.commit, runId: expected.runId, runAttempt: expected.runAttempt,
    host: { platform: 'darwin', architecture: 'arm64' },
    checks: Object.fromEntries(ARTIFACT_CHECKS.map(key => [key, { status: 'pass' }])),
    publication: { workflowSuccessRequired: true, finalPath: expected.reportPath,
      stagingPath: `${expected.reportPath}.${randomUUID()}.pending`, stagingPolicy: 'success requires alias removal' } };
  report.checks.dmg.value = dmg;
  report.checks.bundle_info.value = { version: expected.version };
  report.checks.process_survival.value = { observedMs: 8000, reaped: true, exitCode: null, signal: 'SIGTERM' };
  return { expected, report, payload, dmg };
}

test('matching report passes only with successful step and exact provenance', () => {
  const { expected, report, dmg } = fixture();
  assert.deepEqual(validateAcceptanceReport(report, expected).dmg, dmg);
  report.checks.process_survival.value = { observedMs: 8100, reaped: true, exitCode: 0, signal: null };
  validateAcceptanceReport(report, expected);
});
for (const key of ARTIFACT_CHECKS) {
  for (const state of ['fail', 'not_run', 'missing']) test(`acceptance rejects ${key}/${state}`, () => {
    const { expected, report } = fixture();
    if (state === 'missing') delete report.checks[key]; else report.checks[key].status = state;
    assert.throws(() => validateAcceptanceReport(report, expected));
  });
}
for (const outcome of [undefined, 'failure', 'cancelled', 'skipped']) {
  test(`a JSON pass cannot override validation outcome ${outcome}`, () => {
    const { expected, report } = fixture(); expected.validationOutcome = outcome;
    assert.throws(() => validateAcceptanceReport(report, expected), /outcome/);
  });
}
for (const key of ['commit', 'runId', 'runAttempt']) test(`rejects another ${key}`, () => {
  const { expected, report } = fixture(); report[key] = 'different';
  assert.throws(() => validateAcceptanceReport(report, expected), /another/);
});
for (const terminal of [{ exitCode: 7, signal: null }, { exitCode: null, signal: 'SIGSEGV' },
  { exitCode: null, signal: 'SIGKILL' }, { exitCode: 0, signal: 'SIGTERM' }]) {
  test(`rejects abnormal survival evidence ${JSON.stringify(terminal)}`, () => {
    const { expected, report } = fixture(); Object.assign(report.checks.process_survival.value, terminal);
    assert.throws(() => validateAcceptanceReport(report, expected), /termination/);
  });
}
for (const [name, mutate] of [
  ['interruption', r => { r.interrupted = true; }],
  ['cleanup failure', r => { r.cleanupError = 'fixture'; }],
  ['publication failure', r => { r.publicationError = 'fixture'; }],
  ['wrong host', r => { r.host.platform = 'win32'; }],
  ['wrong architecture', r => { r.host.architecture = 'x64'; }],
  ['wrong version', r => { r.checks.bundle_info.value.version = 'other'; }],
  ['extra check', r => { r.checks.unreviewed = { status: 'pass' }; }],
  ['too short', r => { r.checks.process_survival.value.observedMs = 7999; }],
  ['non-finite interval', r => { r.checks.process_survival.value.observedMs = Infinity; }],
  ['unreaped', r => { r.checks.process_survival.value.reaped = false; }],
  ['no workflow requirement', r => { r.publication.workflowSuccessRequired = false; }],
  ['wrong final path', r => { r.publication.finalPath += '.other'; }],
  ['bad staging id', r => { r.publication.stagingPath = `${r.publication.finalPath}.${'-'.repeat(36)}.pending`; }],
  ['bad hash', r => { r.checks.dmg.value.sha256 = 'bad'; }],
  ['empty image', r => { r.checks.dmg.value.size = 0; }],
]) test(`rejects report fault: ${name}`, () => {
  const { expected, report } = fixture(); mutate(report);
  assert.throws(() => validateAcceptanceReport(report, expected));
});

async function withFiles(run) {
  const root = await fs.mkdtemp(path.join(os.tmpdir(), 'syndocal-artifact-acceptance-'));
  try {
    const data = fixture(root);
    await fs.writeFile(data.dmg.path, data.payload, { flag: 'wx' });
    await fs.writeFile(`${data.dmg.path}.sha256`, `${data.dmg.sha256}  ${data.dmg.name}\n`, { flag: 'wx' });
    await run(data);
  } finally { await fs.rm(root, { recursive: true }); }
}

test('exact file pair and SHA-256 are checked using real temporary files', () => withFiles(async ({ report, expected, dmg }) => {
  assert.deepEqual(await verifyAcceptanceFiles(report, expected), dmg);
}));
for (const fault of ['staging alias', 'missing image', 'changed image', 'changed sidecar', 'extra image', 'extra sidecar', 'sidecar directory']) {
  test(`upload rejects ${fault}`, () => withFiles(async ({ report, expected, dmg, payload }) => {
    if (fault === 'staging alias') await fs.link(dmg.path, report.publication.stagingPath);
    if (fault === 'missing image') await fs.unlink(dmg.path);
    if (fault === 'changed image') await fs.writeFile(dmg.path, Buffer.alloc(payload.length, 7));
    if (fault === 'changed sidecar') await fs.writeFile(`${dmg.path}.sha256`, 'wrong');
    if (fault === 'extra image') await fs.writeFile(path.join(expected.dmgDirectory, 'old.dmg'), 'old');
    if (fault === 'extra sidecar') await fs.writeFile(path.join(expected.dmgDirectory, 'old.dmg.sha256'), 'old');
    if (fault === 'sidecar directory') { await fs.unlink(`${dmg.path}.sha256`); await fs.mkdir(`${dmg.path}.sha256`); }
    await assert.rejects(verifyAcceptanceFiles(report, expected));
  }));
}
test('upload CLI refuses a local process even with a purported successful report', () => withFiles(async ({ report, expected }) => {
  await fs.writeFile(expected.reportPath, JSON.stringify(report), { flag: 'wx' });
  const command = fileURLToPath(new URL('./macos-artifact-acceptance.mjs', import.meta.url));
  await assert.rejects(promisify(execFile)(process.execPath, [command,
    '--dmg-dir', expected.dmgDirectory, '--report', expected.reportPath], {
    env: { ...process.env, GITHUB_ACTIONS: 'false', MACOS_VALIDATION_OUTCOME: 'success' }, timeout: 5000,
  }), error => error.code === 1 && error.stderr.includes('GitHub-hosted'));
}));
