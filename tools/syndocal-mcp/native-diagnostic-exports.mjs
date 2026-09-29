import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { createHash } from 'node:crypto';

// Invoked only after the lifecycle runner verifies its blank isolated QA app.
// No GUI actions, project writes or external MCP permission changes.
export async function nativeDiagnosticExports(invoke) {
  const directory = await fs.mkdtemp(path.join(os.tmpdir(), 'syndocal-diagnostic-qa-'));
  const destination = path.join(directory, 'support.zip');
  const absent = async () => assert.equal(await fs.stat(destination).then(() => true, error => {
    if (error.code === 'ENOENT') return false;
    throw error;
  }), false, 'No destination may be created before approval');
  const prepare = () => invoke('prepare_diagnostic_export_v1', { destination });
  const finish = (preview, approved, sha256 = preview.sha256) => invoke('finish_diagnostic_export_v1', {
    captureId: preview.captureId, sha256, approved,
  });
  try {
    await assert.rejects(invoke('prepare_diagnostic_export_v1', { destination: 'relative.zip' }));
    const cancelled = await prepare();
    assert.equal(cancelled.destination, destination);
    assert.equal(cancelled.validForMs, 120000);
    assert.match(cancelled.summary, /Excludes project\/media files, paths, credentials/);
    await absent();
    assert.equal(await finish(cancelled, false), null);
    await assert.rejects(finish(cancelled, true));
    await absent();
    const preview = await prepare();
    await assert.rejects(finish(preview, true, '0'.repeat(64)));
    await absent();
    assert.equal(await finish(preview, true), destination);
    const bytes = await fs.readFile(destination);
    assert.equal(createHash('sha256').update(bytes).digest('hex'), preview.sha256);
    assert.equal(bytes.readUInt32LE(0), 0x04034b50);
    assert.ok(bytes.length <= 160 * 1024);
    assert.ok(!bytes.includes(Buffer.from(directory)), 'Package must not expose export path');
    await assert.rejects(finish(preview, true));
    assert.equal(createHash('sha256').update(await fs.readFile(destination)).digest('hex'), preview.sha256);
    const race = path.join(directory, 'race.zip');
    const racingPreview = await invoke('prepare_diagnostic_export_v1', { destination: race });
    await fs.writeFile(race, 'new occupant', { flag: 'wx' });
    await assert.rejects(finish(racingPreview, true));
    assert.equal(await fs.readFile(race, 'utf8'), 'new occupant', 'Later file must not be replaced');
    await assert.rejects(invoke('prepare_diagnostic_export_v1', { destination: race }));
    // Leave one in-memory capture to prove it cannot survive the runner's restart.
    const retired = await invoke('prepare_diagnostic_export_v1', {
      destination: path.join(directory, 'retired.zip'),
    });
    return { retired, check: { check: 'native-diagnostic-preview-cancel-digest-new-file-race-replay', passed: true,
      archiveBytes: bytes.length, archiveSha256: preview.sha256 } };
  } finally {
    assert.equal(path.dirname(path.resolve(directory)), path.resolve(os.tmpdir()));
    assert.ok(path.basename(directory).startsWith('syndocal-diagnostic-qa-'));
    for (const entry of await fs.readdir(directory)) {
      assert.ok(['support.zip', 'race.zip', 'retired.zip'].includes(entry)
        || /^\.syndocal-diagnostics-\d+-[0-9a-f]{32}\.partial\.zip$/.test(entry),
      'Only this QA run may create files in its private directory');
      await fs.unlink(path.join(directory, entry));
    }
    await fs.rmdir(directory);
  }
}
