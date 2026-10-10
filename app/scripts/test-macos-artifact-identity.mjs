import test from 'node:test';
import assert from 'node:assert/strict';
import { inspectProcessIdentity } from './macos-artifact-identity.mjs';

const executable = '/extracted/Syndocal.app/Contents/MacOS/syndocal';
const options = { pid: 123, nodeExecutable: '/actual/Node', realpath: async value => value };
function runner(change = {}) {
  const calls = [];
  const run = async (command, args, limits) => {
    calls.push({ command, args, limits });
    if (calls.length === 1) return { stdout: JSON.stringify({
      schemaVersion: 1, processId: 123, executablePath: '/actual/Node', ...change.identity,
    }) };
    if (change.acceptMalformed) return { stdout: '' };
    const error = new Error('exit 1');
    error.result = { reaped: true, code: 1, signal: null, reason: null,
      stdout: '', stderr: 'PID must be canonical positive decimal', ...change.failure };
    throw error;
  };
  return { run, calls };
}
test('extract inspection executes exact product with bounded calls and rejects malformed startup', async () => {
  const { run, calls } = runner();
  const result = await inspectProcessIdentity(executable, run, options);
  assert.equal(result.malformedRequestsRejected, 2);
  assert.equal(result.cliExecutablePath, executable);
  assert.deepEqual(calls.map(c => c.args), [
    ['--syndocal-process-path-v1', '123'], ['--syndocal-process-path-v1', '0'],
    ['unexpected', '--syndocal-process-path-v1', '123'],
  ]);
  for (const call of calls) {
    assert.equal(call.command, executable);
    assert.deepEqual(call.limits, { timeoutMs: 10000, outputLimit: 8192 });
  }
});
for (const identity of [
  { processId: 124 }, { schemaVersion: 2 }, { executablePath: '/actual/node' },
  { executablePath: 'relative' }, { unexpected: true },
]) test('rejects mismatching extracted CLI response ' + JSON.stringify(identity), async () => {
  const { run } = runner({ identity });
  await assert.rejects(inspectProcessIdentity(executable, run, options));
});
for (const failure of [
  { reaped: false }, { code: 0 }, { signal: 'SIGTERM' }, { reason: 'Command timed out' },
  { stdout: '{}' }, { stderr: '' },
]) test('rejects unsuccessful malformed-request proof ' + JSON.stringify(failure), async () => {
  const { run } = runner({ failure });
  await assert.rejects(inspectProcessIdentity(executable, run, options));
});
test('accepting malformed CLI blocks package acceptance', async () => {
  const { run } = runner({ acceptMalformed: true });
  await assert.rejects(inspectProcessIdentity(executable, run, options));
});
