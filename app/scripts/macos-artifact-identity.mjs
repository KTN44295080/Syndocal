import * as fs from 'node:fs/promises';
import path from 'node:path';
import { parseMacosProcessPath } from '../../tools/syndocal-mcp/process-identity.mjs';
import { requireCondition } from './macos-artifact-policy.mjs';

// Exercise the extracted product, not a fixture built from the same source.
export async function inspectProcessIdentity(executable, run, {
  pid = process.pid, nodeExecutable = process.execPath, realpath = fs.realpath,
} = {}) {
  const result = await run(executable, ['--syndocal-process-path-v1', String(pid)],
    { timeoutMs: 10000, outputLimit: 8192 });
  const observed = parseMacosProcessPath(result.stdout, pid);
  const [actual, expected] = await Promise.all([realpath(observed), realpath(nodeExecutable)]);
  requireCondition(actual === expected, 'Extracted app inspected a different executable');
  let rejected = 0;
  for (const args of [['--syndocal-process-path-v1', '0'],
    ['unexpected', '--syndocal-process-path-v1', String(pid)]]) {
    try {
      await run(executable, args, { timeoutMs: 10000, outputLimit: 8192 });
      throw new Error('Malformed process inspection was accepted');
    } catch (error) {
      const failure = error.result;
      requireCondition(failure?.reaped === true && failure.code === 1
        && failure.signal === null && failure.reason === null && failure.stdout === ''
        && failure.stderr.trim().length > 0, 'Malformed inspection did not exit cleanly without identity');
      rejected += 1;
    }
  }
  return { schemaVersion: 1, processId: pid, executablePath: actual,
    inspectedExecutablePath: expected, cliExecutablePath: path.posix.resolve(executable),
    malformedRequestsRejected: rejected };
}
