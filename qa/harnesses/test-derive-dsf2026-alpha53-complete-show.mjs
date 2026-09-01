import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

import {
  ALLOWED_CHANGED_PATHS,
  CANONICAL_AUTHORITY_PATH,
  CURRENT_MEDIA_SOURCE_PATH,
  FIXED_FOREGROUND_DURATION_MS,
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
const sourceForeground = currentProject.snapshot.video.layers[2];
const derivedForeground = derived.output.snapshot.video.layers[2];
assert.equal(derivedForeground.id, 3);
assert.equal(derivedForeground.media_asset_id, 5);
assert.equal(derivedForeground.state.playing, true);
assert.equal(derivedForeground.state.loop_enabled, true);
assert.equal(derivedForeground.state.loop_start_ms, 0);
assert.equal(derivedForeground.state.loop_end_ms, FIXED_FOREGROUND_DURATION_MS);
assert.equal(derivedForeground.state.position_ms, sourceForeground.state.position_ms);
assert.equal(derivedForeground.clip_slots.length, 1);
assert.equal(derivedForeground.default_clip_slot_id, 3);
assert.equal(derivedForeground.clip_slots[0].loop_mode, "Loop");
assert.equal(derivedForeground.clip_slots[0].in_point_ms, 0);
assert.equal(derivedForeground.clip_slots[0].out_point_ms, FIXED_FOREGROUND_DURATION_MS);
const expectedForeground = structuredClone(sourceForeground);
expectedForeground.state.playing = true;
expectedForeground.state.loop_enabled = true;
expectedForeground.state.loop_start_ms = 0;
expectedForeground.state.loop_end_ms = FIXED_FOREGROUND_DURATION_MS;
expectedForeground.clip_slots[0].loop_mode = "Loop";
expectedForeground.clip_slots[0].out_point_ms = FIXED_FOREGROUND_DURATION_MS;
assert.deepEqual(derivedForeground, expectedForeground, "foreground derivation must preserve every non-loop field");
assert.deepEqual(derived.output.snapshot.timeline_bank.slice(1), currentProject.snapshot.timeline_bank.slice(1));
assert.equal(derived.output.snapshot.timeline.follow.destination_start_mode, "wait_for_pedal");

const pausedStateMismatch = structuredClone(currentProject);
pausedStateMismatch.snapshot.video.layers[2].state.playing = true;
assert.throws(
  () => deriveCompleteShowProject(pausedStateMismatch, authorityProject),
  /must start paused at position 0/,
  "a foreground source that is already playing must fail closed",
);

const loopModeMismatch = structuredClone(currentProject);
loopModeMismatch.snapshot.video.layers[2].clip_slots[0].loop_mode = "Loop";
assert.throws(
  () => deriveCompleteShowProject(loopModeMismatch, authorityProject),
  /must start as Once with no authored out point/,
  "a foreground source with a pre-authored loop must fail closed",
);

const outPointMismatch = structuredClone(currentProject);
outPointMismatch.snapshot.video.layers[2].clip_slots[0].out_point_ms = FIXED_FOREGROUND_DURATION_MS;
assert.throws(
  () => deriveCompleteShowProject(outPointMismatch, authorityProject),
  /must start as Once with no authored out point/,
  "a foreground source with a pre-authored out point must fail closed",
);

const sourceDurationMismatch = structuredClone(currentProject);
sourceDurationMismatch.snapshot.video.layers[2].source.metadata.duration_ms = FIXED_FOREGROUND_DURATION_MS + 1;
assert.throws(
  () => deriveCompleteShowProject(sourceDurationMismatch, authorityProject),
  /exact 3008 ms 1280x1080 asset/,
  "a foreground source with ambiguous duration must fail closed",
);

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
