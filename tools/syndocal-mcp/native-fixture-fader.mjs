// Exact native backend acceptance; all physical output remains disabled.
import assert from 'node:assert/strict';
import fs from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { randomUUID, createHash } from 'node:crypto';
import { parseOptions } from './server.mjs';
import { openNativeBackendSession } from './native-backend-session.mjs';

const args = process.argv.slice(2);
const take = key => args[args.indexOf(key) + 1];
const executable = take('--expected-executable');
const descriptor = take('--descriptor');
const evidence = take('--evidence');
assert.equal(path.basename(path.dirname(descriptor)), 'jp.seraf.ktn.syndocal.qa.mcp-lifecycle.video-blackout-20261007', 'Only the isolated fixture QA profile may be mutated');
assert.ok(path.isAbsolute(evidence));
await assert.rejects(fs.access(evidence));
const backend = await openNativeBackendSession(parseOptions(['--expected-executable', executable, '--descriptor', descriptor]), Number(take('--cdp-port')));
const directory = await fs.mkdtemp(path.join(os.tmpdir(), 'syndocal-fixture-native-'));
const ownerId = `fixture-native-${randomUUID()}`;
const checkpoint = () => backend.invoke('get_project_checkpoint', { midiMappings: [], oscMappings: [], dmxMappings: [] });
const authority = () => backend.invoke('get_project_authority_bundle');
const history = () => backend.invoke('get_project_history_status');
const fixtures = async () => (await checkpoint()).snapshot.fixtures;
const dimmer = image => image[0].attribute_values.find(value => value.attribute === 'Dimmer').value;
const checks = [];
let activeTransaction = null;
const safe = async () => {
  const own = await backend.invoke('get_output_ownership_status');
  assert.equal(own.lighting_allowed, false); assert.equal(own.video_allowed, false);
  const project = await checkpoint();
  assert.equal(project.snapshot.output.enabled, false);
  assert.ok(project.snapshot.dmx_outputs.every(route => !route.enabled));
  assert.ok(project.snapshot.video.outputs.every(route => !route.enabled));
};
const load = async file => {
  const bundle = await authority();
  await backend.invoke('load_project_path', { path: file, ownerId, expectedEpoch: bundle.project_epoch,
    expectedRevision: bundle.project_revision, expectedCheckpointHash: bundle.checkpoint_hash });
};
const begin = async commandName => {
  const bundle = await authority();
  const label = 'Fixture native acceptance', coalesceKey = '';
  const identity = { ownerId, commandName, schemaVersion: 1, clientOperationId: randomUUID(),
    shapeFingerprint: `project-transaction-v1|command=${commandName}|label=${label}|coalesce=${coalesceKey}` };
  const ticket = await backend.invoke('begin_project_transaction', { ...identity, label, coalesceKey,
    expectedEpoch: bundle.project_epoch, expectedRevision: bundle.project_revision, expectedCheckpointHash: bundle.checkpoint_hash });
  activeTransaction = { identity, transactionId: ticket.transaction_id, expectedEpoch: ticket.project_epoch };
  return activeTransaction;
};
const commit = async transaction => {
  const result = await backend.invoke('commit_project_transaction', { ...transaction.identity,
    transactionId: transaction.transactionId, expectedEpoch: transaction.expectedEpoch });
  await backend.invoke('acknowledge_project_transaction', transaction.identity);
  activeTransaction = null;
  return result.history_status;
};
const navigate = async undo => {
  const status = await history();
  return backend.invoke(undo ? 'undo_project_transaction' : 'redo_project_transaction', {
    ownerId, expectedEpoch: status.project_epoch,
    expectedEntryId: undo ? status.undo_entry_id : status.redo_entry_id,
    expectedCheckpointHash: undo ? status.undo_checkpoint_hash : status.redo_checkpoint_hash,
  });
};
let cleanup;
try {
  await safe();
  const empty = await checkpoint();
  assert.deepEqual(empty.snapshot.fixtures, [], 'Use only an empty isolated QA profile');
  await backend.invoke('register_project_transaction_owner', { ownerId });
  cleanup = path.join(directory, 'empty.sdc');
  await fs.writeFile(cleanup, JSON.stringify(empty), { flag: 'wx' });
  const source = JSON.parse(await fs.readFile(new URL('../../qa/migration/authored-control-project.json', import.meta.url), 'utf8'));
  const project = structuredClone(empty);
  project.custom_profiles = source.custom_profiles;
  const first = structuredClone(source.snapshot.fixtures[0]);
  first.group_ids = []; first.address = 1;
  first.attribute_values.find(value => value.attribute === 'Dimmer').value = 1000;
  const second = structuredClone(first); second.id = first.id + 1; second.address = 33; second.label = 'Second';
  project.snapshot.fixtures = [first, second];
  const file = path.join(directory, 'fixtures.sdc');
  await fs.writeFile(file, JSON.stringify(project), { flag: 'wx' });
  await load(file); await safe();
  const initialDepth = (await history()).undo_depth;
  const transaction = await begin('stream_fixture_attribute_v1');
  const write = (sequence, value) => ({ request: { targets: [{ fixture_id: first.id, attribute: 'Dimmer' }], value,
    sequence, projectTransactionId: transaction.transactionId, expectedEpoch: transaction.expectedEpoch, ownerId } });
  const latencies = [];
  for (let sequence = 1; sequence <= 60; sequence++) {
    const started = performance.now();
    await backend.invoke('stream_fixture_attribute_v1', write(sequence, sequence * 1000));
    latencies.push(performance.now() - started);
    if (sequence === 30) assert.equal(dimmer(await fixtures()), 30000);
  }
  assert.equal(dimmer(await fixtures()), 60000);
  assert.equal((await history()).undo_depth, initialDepth);
  await assert.rejects(backend.invoke('stream_fixture_attribute_v1', write(60, 0)));
  const invalid = write(61, 0); invalid.request.targets.push({ fixture_id: 999999, attribute: 'Dimmer' });
  await assert.rejects(backend.invoke('stream_fixture_attribute_v1', invalid));
  assert.equal(dimmer(await fixtures()), 60000);
  assert.equal((await commit(transaction)).undo_depth, initialDepth + 1);
  await navigate(true); assert.equal(dimmer(await fixtures()), 1000);
  await navigate(false); assert.equal(dimmer(await fixtures()), 60000);
  checks.push('60 live samples, intermediate/final readback, strict replay/invalid rejection, one Undo/Redo');

  const before = await fixtures();
  const moving = await begin('set_fixture_patches');
  const changes = before.map(fixture => ({ fixtureId: fixture.id, expectedUniverse: fixture.universe,
    expectedAddress: fixture.address, expectedFootprint: Math.max(1, ...fixture.controls.flatMap(control => control.offsets)), label: fixture.label,
    universe: fixture.universe + 1, address: fixture.address + 64 }));
  await backend.invoke('set_fixture_patches', { changes, ownerId, projectTransactionId: moving.transactionId, expectedEpoch: moving.expectedEpoch });
  await commit(moving);
  let image = await fixtures();
  assert.deepEqual(image.map(fixture => fixture.address), [65, 97]);
  assert.equal(dimmer(image), 60000);
  await navigate(true); assert.deepEqual((await fixtures()).map(fixture => fixture.address), [1, 33]);
  await navigate(false); assert.deepEqual((await fixtures()).map(fixture => fixture.address), [65, 97]);
  checks.push('Atomic two-fixture repatch preserves spacing/values and one Undo/Redo');
  await safe(); await load(cleanup); cleanup = null;
  checks.push('All physical outputs remained disarmed; isolated project restored');
  await fs.mkdir(path.dirname(evidence), { recursive: true });
  await fs.writeFile(evidence, JSON.stringify({ schemaVersion: 1, passed: true, timestamp: new Date().toISOString(),
    executable: await fs.realpath(executable), executableSha256: createHash('sha256').update(await fs.readFile(executable)).digest('hex'),
    checks, nativeIpcLatencyMs: { samples: latencies.length, mean: latencies.reduce((a,b) => a+b,0)/latencies.length, max: Math.max(...latencies) },
    nonclaims: ['No Control DOM interaction', 'No physical USB-DMX timing or output observation'] }, null, 2) + '\n', { flag: 'wx' });
  console.log('PASS native fixture fader/repatch backend:', checks);
} finally {
  try {
    if (activeTransaction) {
      await backend.invoke('cancel_project_transaction', { ...activeTransaction.identity,
        transactionId: activeTransaction.transactionId, expectedEpoch: activeTransaction.expectedEpoch });
      await backend.invoke('acknowledge_project_transaction', activeTransaction.identity);
    }
    if (cleanup) await load(cleanup);
  } finally { backend.close(); }
}
