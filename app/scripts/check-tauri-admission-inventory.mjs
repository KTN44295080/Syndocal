import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';

// Source-inventory identity only. Rust/native checks still own route classes and authorization.
const marker = 'tauri::generate_handler![';
const asioCfg = '#[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]';
function registeredNames(source) {
  assert.equal(source.split(marker).length, 2, 'Expected exactly one Tauri handler inventory');
  const remainder = source.slice(source.indexOf(marker) + marker.length);
  const endings = ['](invoke)', '],\r\n', '],\n', '];', ']) ', '])\r\n', '])\n', '])', '\n]'];
  const end = endings.find(value => remainder.includes(value));
  assert.ok(end, 'Missing Tauri handler closing bracket');
  const names = []; let pendingCfg = false;
  for (const line of remainder.slice(0, remainder.indexOf(end)).split(/\r?\n/)) {
    const name = line.trim().replace(/,$/, '').trim();
    if (!name) continue;
    if (name === asioCfg) { assert.equal(pendingCfg, false, 'Duplicate cfg'); pendingCfg = true; continue; }
    assert.match(name, /^[a-z][a-z0-9_]*$/, 'Unsupported handler syntax');
    names.push(name); pendingCfg = false;
  }
  assert.equal(pendingCfg, false, 'Dangling cfg');
  assert.ok(names.length > 0 && names.length <= 2048, 'Invalid handler count');
  assert.equal(new Set(names).size, names.length, 'Duplicate handler');
  return names.sort();
}
const fingerprint = names => createHash('sha256').update(names.join('\n')).digest('hex');
function oneMatch(source, pattern, label) {
  const matches = [...source.matchAll(pattern)];
  assert.equal(matches.length, 1, `Expected one frozen ${label}`);
  return matches[0][1];
}
function validateInventory(source, policy) {
  const names = registeredNames(source);
  const count = Number(oneMatch(policy, /^const FROZEN_TAURI_ROUTE_ADMISSION_COUNT: usize = (\d+);$/gm, 'count'));
  const sha256 = oneMatch(policy, /^const FROZEN_TAURI_ROUTE_ADMISSION_SHA256: &str =\s*"([0-9a-f]{64})";/gm, 'hash');
  assert.equal(names.length, count, 'Native Tauri inventory count drifted; review and classify the exact delta');
  assert.equal(fingerprint(names), sha256, 'Native Tauri inventory hash drifted; do not bypass admission');
  return { count, sha256 };
}
const fixture = names => `${marker}\n${names.join(',\n')}\n];`;
const names = ['get_snapshot', 'known_command'];
const policy = `const FROZEN_TAURI_ROUTE_ADMISSION_COUNT: usize = 2;\nconst FROZEN_TAURI_ROUTE_ADMISSION_SHA256: &str =\n "${fingerprint(names)}";`;
let rejected = 0;
validateInventory(fixture(names), policy);
validateInventory(fixture([...names].reverse()).replaceAll('\n', '\r\n'), policy);
assert.deepEqual(registeredNames(fixture(names).replace('\n', `\n${asioCfg}\n`)), names);
for (const bad of [
  '', fixture([]), fixture([...names, names[0]]), `${fixture(names)}\n${fixture(names)}`,
  fixture([asioCfg]), fixture([asioCfg, asioCfg, ...names]), fixture(['module::command']),
  fixture(['get_snapshot // comment']), fixture(['#[cfg(unreviewed)]', ...names]),
  fixture([...names, 'new_unreviewed_command']), fixture(['get_snapshot', 'renamed_command']),
]) { assert.throws(() => validateInventory(bad, policy)); rejected++; }
for (const bad of ['', `${policy}\n${policy}`, policy.replace('usize = 2', 'usize = 3'), policy.replace(fingerprint(names), '0'.repeat(64))]) {
  assert.throws(() => validateInventory(fixture(names), bad)); rejected++;
}
const main = readFileSync(new URL('../src-tauri/src/main.rs', import.meta.url), 'utf8');
const control = readFileSync(new URL('../src-tauri/src/control_plane.rs', import.meta.url), 'utf8');
const result = validateInventory(main, control);
console.log(`Native Tauri admission inventory exact: ${result.count} commands, SHA-256 ${result.sha256}; ${rejected} negative fixtures rejected`);
