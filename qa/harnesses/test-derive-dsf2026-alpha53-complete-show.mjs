import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

import {
  ALLOWED_CHANGED_PATHS,
  CANONICAL_AUTHORITY_PATH,
  CURRENT_MEDIA_SOURCE_PATH,
  EXPECTED_DURATION_MS,
  OUTPUT_PATH,
  deriveCompleteShowProject,
} from "./derive-dsf2026-alpha53-complete-show.mjs";

function readProject(path) {
  return JSON.parse(readFileSync(path, "utf8"));
}

const currentProject = readProject(CURRENT_MEDIA_SOURCE_PATH);
const authorityProject = readProject(CANONICAL_AUTHORITY_PATH);
const derived = deriveCompleteShowProject(currentProject, authorityProject);

assert.deepEqual(derived.changedPaths, [...ALLOWED_CHANGED_PATHS].sort());
assert.equal(derived.output.snapshot.timeline.events[0].duration_ms, EXPECTED_DURATION_MS);
assert.equal(derived.output.snapshot.timeline.events[1].duration_ms, EXPECTED_DURATION_MS);
assert.equal(derived.output.snapshot.timeline_bank[0].events[0].duration_ms, EXPECTED_DURATION_MS);
assert.equal(derived.output.snapshot.timeline_bank[0].events[1].duration_ms, EXPECTED_DURATION_MS);
assert.deepEqual(derived.output.snapshot.video.outputs, currentProject.snapshot.video.outputs);
assert.deepEqual(derived.output.snapshot.video.compositions, currentProject.snapshot.video.compositions);
assert.deepEqual(derived.output.snapshot.video.media_assets, currentProject.snapshot.video.media_assets);
assert.deepEqual(derived.output.snapshot.timeline_bank.slice(1), currentProject.snapshot.timeline_bank.slice(1));
assert.equal(derived.output.snapshot.timeline.follow.destination_start_mode, "wait_for_pedal");

const sourceMismatch = structuredClone(currentProject);
sourceMismatch.snapshot.timeline.label = "future timeline";
assert.throws(
  () => deriveCompleteShowProject(sourceMismatch, authorityProject),
  /primary Timeline identity/,
  "source identity changes must fail closed",
);

const futureAuthority = structuredClone(authorityProject);
futureAuthority.snapshot.timeline.events[0].duration_ms = EXPECTED_DURATION_MS + 1;
assert.throws(
  () => deriveCompleteShowProject(currentProject, futureAuthority),
  /duration_ms must be exactly/,
  "future canonical duration ambiguity must fail closed",
);

const extraAuthorityEvent = structuredClone(authorityProject);
extraAuthorityEvent.snapshot.timeline.events.push(structuredClone(extraAuthorityEvent.snapshot.timeline.events[1]));
assert.throws(
  () => deriveCompleteShowProject(currentProject, extraAuthorityEvent),
  /exactly the two authored Timelines|exactly the two approved authored events/,
  "future canonical event additions must fail closed",
);

const harnessPath = fileURLToPath(new URL("./derive-dsf2026-alpha53-complete-show.mjs", import.meta.url));
const verify = spawnSync(process.execPath, [harnessPath, "--verify-only"], {
  cwd: fileURLToPath(new URL("../../", import.meta.url)),
  encoding: "utf8",
});
assert.equal(verify.status, 0, verify.stderr || verify.stdout);
const verifyResult = JSON.parse(verify.stdout);
assert.equal(verifyResult.status, "PASS");
assert.deepEqual(verifyResult.changedPaths, [...ALLOWED_CHANGED_PATHS].sort());

console.log("alpha53 complete-show derivation checks passed");
