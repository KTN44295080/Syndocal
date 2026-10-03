import assert from 'node:assert/strict';
import fs from 'node:fs/promises';

// Private, empty-project schema failures. Never print a hostile diagnostic.
export async function nativeInputDiagnostics({ seed, backup, write, load, observe, before, digest, checks }) {
  const sentinel = 'PRIVATE_VALUE_MUST_NOT_BE_ECHOED';
  const hostile = `${sentinel}\u001b\n\t${'漢😀'.repeat(8192)}`;
  const cases = [
    ['foreign-app', value => { value.app = hostile; }, /Unsupported .*app/],
    ['unknown-protocol', value => { (backup ? value.project : value).snapshot.output.protocol = hostile; }, /snapshot\.output\.protocol/],
    ['wrong-bpm-type', value => { (backup ? value.project : value).snapshot.clock.bpm = hostile; }, /snapshot\.clock\.bpm/],
  ];
  const check = { check: `native-${backup ? 'backup' : 'project'}-input-diagnostics-bounded-with-state-preserved`, passed: false,
    beforeToken: before.token, cases: [] };
  checks.push(check);
  for (const [index, [name, change, expected]] of cases.entries()) {
    const value = structuredClone(seed);
    change(value);
    const { file, target, bytes } = await write(index, name, value);
    const hash = digest(bytes);
    const result = await load(target);
    assert.equal(result.ok, false, `${name}: invalid input must reject`);
    const after = await observe();
    assert.deepEqual(after, before, `${name}: rejection must preserve active project, authority and output`);
    assert.equal(digest(await fs.readFile(file)), hash, `${name}: source bytes must remain unchanged`);
    const diagnosticBytes = Buffer.byteLength(result.error, 'utf8');
    check.cases.push({ name, byteLength: bytes.length, sourceSha256: hash, diagnosticBytes, stateAndSourcePreserved: true });
    assert.ok(diagnosticBytes <= 2048, `${name}: native diagnostic is ${diagnosticBytes} bytes, limit 2048 including path context`);
    assert.ok(!result.error.includes(sentinel), `${name}: failing values must not be echoed`);
    assert.doesNotMatch(result.error, /[\u0000-\u001f\u007f]/, `${name}: no raw control characters`);
    assert.match(result.error, expected, `${name}: diagnostic must identify the rejected app or schema field`);
  }
  Object.assign(check, { passed: true, afterToken: (await observe()).token });
}
