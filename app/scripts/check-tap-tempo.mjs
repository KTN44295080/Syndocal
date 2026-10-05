import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { tapTempo, tapTempoMessage } from "../src/tapTempo.ts";

const deferred = () => {
  let resolve;
  const promise = new Promise(done => { resolve = done; });
  return { promise, resolve };
};
const clock = { bpm: 80, tap_count: 2, source: "Tap" };
assert.equal(tapTempoMessage(clock), "Tapped BPM 80.0");
for (const source of ["Manual", "Tap"]) {
  assert.equal(tapTempoMessage({ ...clock, tap_count: 1, source }), "Tap again to measure BPM.");
}
for (const source of ["AbletonLink", "DjLink", "MidiClock", "MidiTimecode", "Ltc"]) {
  const message = tapTempoMessage({ ...clock, source });
  assert.match(message, /current clock source:/);
  assert.doesNotMatch(message, /^Tapped BPM/);
  assert.match(tapTempoMessage({ ...clock, tap_count: 1, source }), /^Tap again.*Current clock source:/);
}
let cases = 0;
const fixture = () => {
  const state = { epoch: 1, authority: 0, applied: [], calls: [], snapshot: { clock } };
  const options = {
    projectEpoch: () => state.epoch,
    invokeTap: async () => { state.calls.push("tap"); },
    inFlightAuthorityPoll: () => null,
    refreshAuthority: async () => { state.calls.push("authority"); state.authority++; },
    // Production rejects a read captured under the pre-tap E/R/H identity.
    refreshSnapshot: async () => { state.calls.push("snapshot"); return state.authority ? state.snapshot : null; },
    applied: value => { state.applied.push(value); },
  };
  return { state, options };
};
{
  const { state, options } = fixture();
  assert.equal(await options.refreshSnapshot(), null, "pre-tap authority rejects the read");
  state.calls.length = 0;
  await tapTempo(options);
  assert.deepEqual(state.calls, ["tap", "authority", "snapshot"]);
  assert.deepEqual(state.applied, [clock]); cases++;
}
{
  const { state, options } = fixture(), prior = deferred();
  options.inFlightAuthorityPoll = () => prior.promise;
  const pending = tapTempo(options);
  await Promise.resolve();
  assert.deepEqual(state.calls, ["tap"], "pre-tap poll must settle before a fresh read");
  prior.resolve(); await pending;
  assert.deepEqual(state.calls, ["tap", "authority", "snapshot"]); cases++;
}
for (const boundary of ["invokeTap", "refreshAuthority", "refreshSnapshot"]) {
  const { state, options } = fixture(), original = options[boundary];
  options[boundary] = async () => { const result = await original(); state.epoch++; return result; };
  await tapTempo(options);
  assert.deepEqual(state.applied, [], `replacement during ${boundary} must not report an old tempo`);
  assert.equal(state.calls.filter(value => value === "tap").length, 1); cases++;
}
{
  const { state, options } = fixture(); options.refreshSnapshot = async () => null;
  await tapTempo(options);
  assert.deepEqual(state.applied, [], "stale refresh must not report a cached BPM"); cases++;
}
for (const boundary of ["invokeTap", "refreshAuthority", "refreshSnapshot"]) {
  const { state, options } = fixture();
  options[boundary] = async () => { state.calls.push(boundary); throw new Error("injected failure"); };
  await assert.rejects(tapTempo(options), /injected failure/);
  assert.deepEqual(state.applied, []);
  assert.ok(state.calls.filter(value => value === "tap" || value === "invokeTap").length <= 1,
    "unknown or failed mutations must never be retried"); cases++;
}
const app = readFileSync(new URL("../src/App.tsx", import.meta.url), "utf8");
assert.match(app, /await tapTempo\(\{[\s\S]*?inFlightAuthorityPoll: \(\) => projectAuthorityPollInFlight,[\s\S]*?refreshAuthority: pollProjectAuthorityBundle,[\s\S]*?refreshSnapshot,/);
assert.match(app, /setMessage\(tapTempoMessage\(clock\)\)/);
assert.match(app, /VITE_SYNDOCAL_NATIVE_TAP_QA === "1" && isTauriRuntime\(\)/);
console.log(`PASS Tap tempo: ${cases} async regression cases; authority-before-snapshot, stale/replacement fences and no mutation retries.`);
