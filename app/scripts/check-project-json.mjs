import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { requireExactMsvcLinkerFirst, tauriCommandEnvironment } from './run-tauri.mjs';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const expected = new Map([['input_diagnostic_', 6], ['project_json_', 7], ['project_backup', 12], ['application_update_', 2], ['migration_corpus_', 22], ['standby_checkpoint', 2], ['project_snapshot_for_save_drops_volatile_runtime_state', 1]]);
const filter = process.argv[2];
assert.equal(process.argv.length, 3, 'Usage: node app/scripts/check-project-json.mjs <input_diagnostic_|project_json_|project_backup|application_update_|migration_corpus_|standby_checkpoint|project_snapshot_for_save_drops_volatile_runtime_state>');
assert.ok(expected.has(filter), 'Only owned non-device parser/corpus filters are allowed');
const env = tauriCommandEnvironment(['build']);
if (process.platform === 'win32') {
  console.log(`CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER=${env.CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER}`);
  console.log(`where.exe link.exe:\n${requireExactMsvcLinkerFirst(env).join('\n')}`);
}
const args = ['test', '--manifest-path', 'app/src-tauri/Cargo.toml', '--release', '--locked', '-j', '1',
  filter, '--', '--nocapture', '--test-threads=1'];
console.log(`cargo ${args.join(' ')}`);
const result = spawnSync('cargo', args, { cwd: root, env, encoding: 'utf8', maxBuffer: 16 * 1024 * 1024,
  stdio: ['ignore', 'pipe', 'pipe'] });
process.stdout.write(result.stdout ?? '');
process.stderr.write(result.stderr ?? '');
if (result.error) throw result.error;
assert.equal(result.status, 0);
const summaries = [...(result.stdout ?? '').matchAll(/test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;/g)];
assert.equal(summaries.length, 1, 'Exactly one real test-binary result required');
assert.deepEqual(summaries[0].slice(1).map(Number), [expected.get(filter), 0, 0]);
assert.equal((result.stderr ?? '').match(/^warning(?:\[|:)/gm)?.length ?? 0, 0, 'No first-party compiler warnings allowed');
console.log(`PASS ${filter}: ${expected.get(filter)} selected, none failed or ignored; release test binary, not native window acceptance`);
