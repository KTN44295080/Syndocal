import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

const workflow = (await readFile(new URL('../../.github/workflows/macos-installer.yml', import.meta.url), 'utf8')).replaceAll('\r\n', '\n');
// Focused wiring contract, not a substitute for GitHub's YAML/workflow parser.
function checkWorkflow(source) {
  const trigger = source.slice(source.indexOf('\non:\n'), source.indexOf('\npermissions:'));
  assert.equal(trigger.trim(), 'on:\n  workflow_dispatch:');
  assert.equal(source.match(/^  [\w-]+:\n    name:/gm)?.length, 1, 'one explicit macOS job');
  assert.match(source, /runs-on: macos-15\n/);
  assert.doesNotMatch(source, /continue-on-error:|\|\|\s*true/);
  const blocks = source.split(/^      - name: /m).slice(1);
  const step = name => {
    const matches = blocks.filter(block => block.split('\n')[0] === name);
    assert.equal(matches.length, 1, `exact step: ${name}`); return matches[0];
  };
  const tests = step('Test macOS artifact validator');
  assert.match(tests, /set -o pipefail/);
  for (const name of ['test-macos-artifact.mjs', 'test-macos-artifact-report.mjs',
    'test-macos-artifact-acceptance.mjs', 'test-macos-artifact-workflow.mjs']) assert.ok(tests.includes(`app/scripts/${name}`));
  const validate = step('Validate application extracted from final DMG');
  assert.match(validate, /\n        id: validate\n/);
  assert.ok(validate.includes('node app/scripts/check-macos-artifact.mjs'));
  const acceptance = step('Verify final DMG evidence before upload');
  assert.match(acceptance, /\n        id: acceptance\n/);
  assert.ok(acceptance.includes("if: ${{ success() && !cancelled() && steps.validate.outcome == 'success' }}"));
  assert.ok(acceptance.includes('MACOS_VALIDATION_OUTCOME: ${{ steps.validate.outcome }}'));
  assert.ok(acceptance.includes('node app/scripts/macos-artifact-acceptance.mjs'));
  for (const block of [validate, acceptance]) {
    assert.ok(block.includes('--dmg-dir target/release/bundle/dmg'));
    assert.ok(block.includes('--report target/qa/macos-artifact/report.json'));
  }
  const evidence = step('Upload macOS validation evidence');
  assert.match(evidence, /\n        if: always\(\)\n/);
  assert.ok(evidence.includes('path: target/qa/macos-artifact/'));
  const upload = step('Upload macOS DMG');
  assert.ok(upload.includes("if: ${{ success() && !cancelled() && steps.validate.outcome == 'success' && steps.acceptance.outcome == 'success' }}"));
  assert.ok(upload.includes('target/release/bundle/dmg/*.dmg\n'));
  assert.ok(upload.includes('target/release/bundle/dmg/*.dmg.sha256'));
  assert.ok(upload.includes('if-no-files-found: error'));
  assert.ok(blocks.indexOf(validate) < blocks.indexOf(acceptance));
  assert.ok(blocks.indexOf(acceptance) < blocks.indexOf(evidence));
  assert.ok(blocks.indexOf(evidence) < blocks.indexOf(upload));
}
test('manual workflow requires validation and acceptance before upload', () => checkWorkflow(workflow));
for (const [name, oldValue, newValue] of [
  ['automatic trigger', '  workflow_dispatch:', '  push:'],
  ['no test result propagation', 'set -o pipefail', 'echo no_pipefail'],
  ['omitted report tests', 'app/scripts/test-macos-artifact-report.mjs ', ''],
  ['omitted acceptance tests', 'app/scripts/test-macos-artifact-acceptance.mjs ', ''],
  ['wrong step id', 'id: validate', 'id: unrelated'],
  ['wrong outcome source', 'MACOS_VALIDATION_OUTCOME: ${{ steps.validate.outcome }}', 'MACOS_VALIDATION_OUTCOME: success'],
]) test(`workflow rejects ${name}`, () => {
  assert.ok(workflow.includes(oldValue)); assert.throws(() => checkWorkflow(workflow.replace(oldValue, newValue)));
});
for (const [name, change] of [
  ['unconditional upload', s => s.replace(" && steps.acceptance.outcome == 'success'", '')],
  ['hidden validation failure', s => s.replace('id: validate', 'id: validate\n        continue-on-error: true')],
  ['missing failure logs', s => s.replace('if: always()', 'if: success()')],
  ['different evidence path', s => s.replaceAll('--report target/qa/macos-artifact/report.json', '--report elsewhere.json')],
]) test(`workflow rejects ${name}`, () => assert.throws(() => checkWorkflow(change(workflow))));
