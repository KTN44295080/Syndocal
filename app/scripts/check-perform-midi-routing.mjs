// Perform MIDI coverage: stable-id targets, renderer routing through the ticketed
// commands, beat-division triggers, and the Rust/TypeScript contract that ties
// them together. Pure model checks; the pad/Take/select markup is asserted
// against the source and in check-perform-layout, native behaviour in cargo.
import assert from "node:assert/strict";
import { readFileSync, existsSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { createRequire } from "node:module";
import ts from "typescript";

const require = createRequire(import.meta.url);
const cache = new Map();
const here = dirname(fileURLToPath(import.meta.url));
const root = resolve(here, "../src");
function load(file) {
  if (cache.has(file)) return cache.get(file);
  const exports = {};
  cache.set(file, exports);
  const js = ts.transpileModule(readFileSync(file, "utf8"), { compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 } }).outputText;
  new Function("require", "exports", js)((name) => {
    if (!name.startsWith(".")) return require(name);
    const target = resolve(dirname(file), name.endsWith(".ts") ? name : `${name}.ts`);
    assert.ok(existsSync(target), `${name} resolves to a source file`);
    return load(target);
  }, exports);
  return exports;
}
const perform = load(resolve(root, "controlMappingPerform.ts"));
const dispatcherModule = load(resolve(root, "performControlDispatcher.ts"));
const learn = load(resolve(root, "controlMappingLearn.ts"));
const faderInput = load(resolve(root, "midiFaderInput.ts"));

// ---------------------------------------------------------------- contract with Rust
const protocolSource = readFileSync(resolve(here, "../../crates/protocol/src/lib.rs"), "utf8");
const enumBody = (name) => protocolSource.match(new RegExp(`pub enum ${name} \\{([\\s\\S]*?)\\n\\}`))[1];
const rustActions = [...enumBody("MidiControlAction").matchAll(/^\s{4}([A-Z]\w+),/gm)].map((match) => match[1]);
for (const action of perform.performRoutedActions) assert.ok(rustActions.includes(action), `${action} is a Rust MidiControlAction`);
for (const action of perform.performRoutedActions) assert.ok(enumBody("OscControlAction").includes(action), action + " is shared by OSC/DMX");
const rustFnVariants = (fn) => [...protocolSource.match(new RegExp(`pub fn ${fn}\\(&self\\) -> bool \\{([\\s\\S]*?)\\n    \\}`))[1].matchAll(/Self::(\w+)/g)].map((match) => match[1]).sort();
assert.deepEqual(rustFnVariants("is_renderer_routed"), [...perform.performRoutedActions].sort(), "Rust and TypeScript agree on the renderer-routed actions");
assert.deepEqual(rustFnVariants("is_renderer_routed_trigger"), perform.performRoutedActions.filter(perform.isPerformTriggerAction).sort(), "Rust and TypeScript agree on which of them are triggers");
const rustBeats = protocolSource.match(/pub const BEAT_DIVISION_BEATS: \[f64; (\d+)\] = \[([^\]]+)\]/);
assert.deepEqual(rustBeats[2].split(",").map(Number), perform.beatDivisions.map((division) => division.beats), "beat division table matches the protocol crate");
assert.equal(Number(rustBeats[1]), perform.beatDivisions.length);
perform.beatDivisions.forEach((division, index) => assert.equal(division.number, index, "the mapping number is the table index"));
assert.ok(enumBody("MidiControlMessage").includes("BeatDivision"));
const typesSource = readFileSync(resolve(root, "types.ts"), "utf8");
for (const action of perform.performRoutedActions) assert.ok(typesSource.includes(`| "${action}"`), `${action} is in the TypeScript MidiControlAction`);
assert.ok(typesSource.includes('| "BeatDivision"'));
const nativeEvent = readFileSync(resolve(here, "../src-tauri/src/midi_renderer_routed.rs"), "utf8").match(/EVENT: &str = "([^"]+)"/)[1];
const appSource = readFileSync(resolve(root, "App.tsx"), "utf8");
assert.ok(appSource.includes(`listen<unknown>("${nativeEvent}"`), "the renderer listens to the native routed-input event");
assert.match(appSource, /midiFaderInput\.acceptRouted\(payload\)/);
assert.match(appSource, /routePerformInput = \(input\) => \{ void performDispatcher\.dispatch\(input\); \}/);
assert.match(appSource, /beatTriggers\.observe\(clock\)/);
assert.match(appSource, /window\.setInterval\(\(\) => beatTriggers\.advance\(\), 25\)/);
assert.match(appSource, /if \(paneWindow\) return;[\s\S]{0,260}untrack\(\(\) => beatTriggers\.observe\(clock\)\)/, "only the main window runs the beat clock, and it does not re-observe on mapping edits");
assert.match(appSource, /const timer = window\.setInterval\(\(\) => beatTriggers\.advance\(\), 25\);/);
for (const source of ["osc", "dmx"]) assert.ok(appSource.includes("syndocal://" + source + "-perform-control-v1"), source + " listens for its native Perform events");
assert.match(appSource, /applyRouted: \(input, sourceIsCurrent\) => routePerformInput\(\{ \.\.\.input, source, sourceIsCurrent \}\)/);
const mainSource = readFileSync(resolve(here, "../src-tauri/src/main.rs"), "utf8");
assert.match(mainSource, /MidiControlEvent::RendererRouted\(input\) => \{[\s\S]*?midi_renderer_routed::EVENT[\s\S]*?return;\s*\}/, "the native worker forwards routed input and issues no engine command");

// ---------------------------------------------------------------- selectors and beats
assert.deepEqual([0, 0.09, 0.1, 0.5, 0.99, 1, 2, -1, NaN].map((position) => perform.selectorIndex(position, 10)), [0, 0, 1, 5, 9, 9, 9, 0, 0]);
assert.equal(perform.performBlendModes.length, 10);
assert.equal(perform.performTransitionKinds.length, 9);
assert.deepEqual([...perform.performBlendModes].slice(0, 2), ["Normal", "Add"]);
assert.ok(perform.beatDivisionCrossed(3.9, 4.1, 4));
assert.ok(!perform.beatDivisionCrossed(4.1, 4.4, 4), "inside one division");
assert.ok(perform.beatDivisionCrossed(3.95, 4, 1), "landing on the boundary fires");
assert.ok(!perform.beatDivisionCrossed(4, 4.2, 1), "the boundary already fired");
assert.ok(!perform.beatDivisionCrossed(4.1, 3.9, 4), "a step backwards is not a boundary");
assert.ok(!perform.beatDivisionCrossed(1, 9, 4), "a jump over two beats resynchronizes instead of firing");
assert.ok(!perform.beatDivisionCrossed(NaN, 4, 4));

const fired = [];
let suspended = false;
const midi = (message, number, action, extra = {}) => ({ channel: message === "BeatDivision" ? null : 0, message, number, action, low: 0, high: 1, ...extra });
let mappings = [
  midi("BeatDivision", 2, "VideoClipSlotTrigger", { layer_id: 1, clip_slot_id: 101 }),
  midi("BeatDivision", 4, "VideoTake"),
  midi("BeatDivision", 2, "VideoIsfStageEnabled", { layer_id: 1, isf_stage_id: 900 }),
  midi("ControlChange", 7, "VideoClipSlotTrigger", { layer_id: 1, clip_slot_id: 102 }),
  midi("BeatDivision", 2, "VideoTransitionTime"),
];
const scheduler = perform.createBeatTriggerScheduler({ mappings: () => mappings, dispatch: (input) => fired.push(input), suspended: () => suspended });
assert.equal(scheduler.tick(undefined), 0);
assert.equal(scheduler.tick(0.9), 0, "the first position only seeds the clock");
assert.equal(scheduler.tick(1.1), 2, "the 1 beat boundary fires the clip and the stage once each");
assert.deepEqual(fired.map((input) => input.action), ["VideoClipSlotTrigger", "VideoIsfStageEnabled"], "a continuous target and a controller mapping never fire from a beat");
assert.equal(fired[0].layer_id, 1);
assert.equal(fired[0].clip_slot_id, 101);
assert.equal(fired[1].toggle, true, "a beat flips an effect stage");
assert.equal(fired[0].toggle, false);
assert.equal(scheduler.tick(1.4), 0);
assert.equal(scheduler.tick(7.95), 0, "a gap over four beats is a resync");
assert.equal(scheduler.tick(8.05), 3, "bar boundary fires the 1 bar Take as well as the 1 beat triggers");
assert.equal(scheduler.tick(8.0), 0, "small clock jitter backwards is ignored");
assert.equal(scheduler.tick(8.06), 0, "and does not re-fire the boundary");
suspended = true;
assert.equal(scheduler.tick(9.1), 0, "Learn suspends beat triggers");
suspended = false;
assert.equal(scheduler.tick(9.2), 0, "the suspended boundary is not replayed");
scheduler.reset();
assert.equal(scheduler.tick(10.1), 0, "reset reseeds");
mappings = [];
assert.equal(scheduler.tick(11.1), 0);
assert.ok(perform.beatDivisionCrossed(1.0, 4.9, 1), "a four beat advance still counts as playing on");
assert.ok(!perform.beatDivisionCrossed(1.0, 5.1, 1), "more than four beats is a restart or seek");

// A boundary that queues a clip and takes it queues first, whatever order the mappings were made in.
{
  const order = [];
  const both = perform.createBeatTriggerScheduler({
    mappings: () => [midi("BeatDivision", 2, "VideoTake"), midi("BeatDivision", 2, "VideoClipSlotTrigger", { layer_id: 1, clip_slot_id: 101 }), midi("BeatDivision", 2, "VideoLayerStop", { layer_id: 2 })],
    dispatch: (input) => order.push(input.action),
  });
  both.tick(0.9);
  both.tick(1.1);
  assert.deepEqual(order, ["VideoClipSlotTrigger", "VideoLayerStop", "VideoTake"]);
}

// Between snapshots the clock is extrapolated at the snapshot's tempo, so a division fires near its boundary.
{
  const extrapolated = [];
  let clockMs = 0;
  let live = [midi("BeatDivision", 2, "VideoTake")];
  const timed = perform.createBeatTriggerScheduler({ mappings: () => live, dispatch: (input) => extrapolated.push([Math.round(clockMs), input.action]), now: () => clockMs });
  const clock = (beat_counter, beat_phase, bpm = 120) => ({ beat_counter, beat_phase, bpm });
  assert.equal(timed.advance(), 0, "nothing to extrapolate before the first snapshot");
  timed.observe(clock(7, 0.8));
  // 120 BPM is one beat per 500 ms: the beat 8 boundary is 100 ms after the snapshot.
  for (clockMs = 25; clockMs <= 250; clockMs += 25) timed.advance();
  assert.deepEqual(extrapolated, [[100, "VideoTake"]], "fires on the timer tick that reaches the boundary, not at the next snapshot");
  clockMs = 250;
  timed.observe(clock(8, 0.5));
  assert.equal(extrapolated.length, 1, "the snapshot that confirms the boundary does not fire it twice");
  // The snapshot lands behind the extrapolated position (tempo drift): no double fire, no miss.
  clockMs = 500;
  timed.observe(clock(8, 0.99));
  for (clockMs = 525; clockMs <= 650; clockMs += 25) timed.advance();
  assert.equal(extrapolated.length, 2, "beat 9 fires once");
  // A snapshot that repeats the clock it already had carries no news: the anchor stays put.
  const repeated = [];
  clockMs = 0;
  const repeatScheduler = perform.createBeatTriggerScheduler({ mappings: () => live, dispatch: () => repeated.push(Math.round(clockMs)), now: () => clockMs });
  repeatScheduler.observe(clock(10, 0));
  clockMs = 400;
  assert.equal(repeatScheduler.observe(clock(10, 0)), 0);
  clockMs = 500;
  repeatScheduler.advance();
  assert.deepEqual(repeated, [500], "the boundary at 500 ms fires because the repeated clock did not re-anchor");
  // A stalled poll stops extrapolating after the limit.
  const stalled = [];
  clockMs = 1000;
  const stall = perform.createBeatTriggerScheduler({ mappings: () => live, dispatch: (input) => stalled.push(input.action), now: () => clockMs });
  stall.observe(clock(20, 0.0));
  clockMs = 1000 + perform.beatExtrapolationLimitMs + 5000;
  assert.equal(stall.advance(), 1, "extrapolation is capped at the limit, which crosses beat 21 at most once");
  clockMs += 1000;
  assert.equal(stall.advance(), 0, "and then holds");
  // No tempo means no extrapolation, only measured positions.
  const noTempo = perform.createBeatTriggerScheduler({ mappings: () => live, dispatch: () => {}, now: () => clockMs });
  noTempo.observe(clock(1, 0.5, 0));
  clockMs += 10_000;
  assert.equal(noTempo.advance(), 0);
  assert.equal(noTempo.observe(clock(2, 0.1, 0)), 1, "a measured boundary still fires");
  assert.equal(noTempo.observe(null), 0);
  assert.equal(noTempo.advance(), 0, "an unreadable clock clears the anchor");
  live = [];
}

// ---------------------------------------------------------------- payload decode
const payload = { action: "VideoIsfParam", layer_id: 4, isf_stage_id: 9, control: "amount", position: 0.5, value: 0.25, active: true };
assert.deepEqual(perform.performRoutedInputFromPayload(payload), { ...payload, clip_slot_id: null, toggle: false });
assert.equal(perform.performRoutedInputFromPayload({ ...payload, toggle: true }).toggle, true);
for (const bad of [
  null, "x", [], { ...payload, action: "SetBpm" }, { ...payload, action: "VideoLayerFade" }, { ...payload, layer_id: 0 }, { ...payload, layer_id: -3 },
  { ...payload, layer_id: 1.5 }, { ...payload, clip_slot_id: "7" }, { ...payload, control: "" }, { ...payload, control: "x".repeat(129) },
  { ...payload, control: 7 }, { ...payload, position: Infinity }, { ...payload, value: NaN }, { ...payload, active: 1 }, { ...payload, toggle: "yes" },
  { ...payload, layer_id: Number.MAX_SAFE_INTEGER + 2 },
]) assert.equal(perform.performRoutedInputFromPayload(bad), null, JSON.stringify(bad));

// ---------------------------------------------------------------- input fences
let epoch = 5;
const applied = [];
const faders = [];
const input = faderInput.createMidiFaderInput({ projectEpoch: () => epoch, apply: (values) => faders.push(values), applyRouted: (routed) => applied.push(routed) });
const fence = { schema_version: 1, project_epoch: 5, input_session: 11 };
assert.equal(input.acceptRouted({ ...fence, sequence: 1, input: payload }), false, "no connection yet");
input.setConnection(fence);
assert.equal(input.acceptRouted({ ...fence, sequence: 1, input: payload }), true);
assert.equal(applied.length, 1);
assert.equal(input.acceptRouted({ ...fence, sequence: 1, input: payload }), false, "a replay is rejected");
assert.equal(input.acceptRouted({ ...fence, sequence: 2, input: { ...payload, action: "Nope" } }), false, "a malformed input is rejected");
assert.equal(input.acceptRouted({ ...fence, sequence: 2, input: payload }), true, "a rejected input does not consume its sequence");
assert.equal(input.accept({ ...fence, sequence: 2, values: [{ fixture_id: 1, attribute: "Dimmer", value: 10 }] }), false, "faders and routed input share one sequence series");
assert.equal(input.accept({ ...fence, sequence: 3, values: [{ fixture_id: 1, attribute: "Dimmer", value: 10 }] }), true);
assert.equal(input.acceptRouted({ ...fence, sequence: 4, input_session: 12, input: payload }), false, "a retired session is rejected");
assert.equal(input.acceptRouted({ ...fence, sequence: 4, schema_version: 2, input: payload }), false);
epoch = 6;
assert.equal(input.acceptRouted({ ...fence, sequence: 4, input: payload }), false, "an old project is rejected");
epoch = 5;
const noRouting = faderInput.createMidiFaderInput({ projectEpoch: () => 5, apply: () => {} });
noRouting.setConnection(fence);
assert.equal(noRouting.acceptRouted({ ...fence, sequence: 1, input: payload }), false, "routing is opt-in");

// OSC and DMX share the decoder but own independent connection/sequence fences.
{
  let transportEpoch = 5;
  const routed = [];
  const receivers = Object.fromEntries(["osc", "dmx"].map(source => [source, faderInput.createMidiFaderInput({ projectEpoch: () => transportEpoch, apply: () => assert.fail("Perform transport cannot apply fixture faders"), applyRouted: input => routed.push({ ...input, source }) })]));
  const oscFence = { schema_version: 1, project_epoch: 5, input_session: 20 };
  const dmxFence = { ...oscFence, input_session: 21 };
  receivers.osc.setConnection(faderInput.midiFaderConnection(oscFence, "OSC"));
  receivers.dmx.setConnection(faderInput.midiFaderConnection(dmxFence, "DMX"));
  assert.ok(receivers.osc.acceptRouted({ ...oscFence, sequence: 1, input: payload }));
  assert.ok(receivers.dmx.acceptRouted({ ...dmxFence, sequence: 1, input: payload }), "DMX does not consume OSC sequence numbers");
  assert.deepEqual(routed.map(input => input.source), ["osc", "dmx"]);
  assert.equal(receivers.osc.acceptRouted({ ...dmxFence, sequence: 2, input: payload }), false, "cross-transport session rejected");
  assert.equal(receivers.osc.acceptRouted({ ...oscFence, sequence: 1, input: payload }), false, "OSC replay rejected");
  assert.equal(receivers.dmx.acceptRouted({ ...dmxFence, sequence: 2, input: { ...payload, value: NaN } }), false);
  assert.ok(receivers.dmx.acceptRouted({ ...dmxFence, sequence: 2, input: payload }), "malformed input does not consume DMX sequence");
  receivers.osc.setConnection(oscFence);
  assert.equal(receivers.osc.acceptRouted({ ...oscFence, sequence: 1, input: payload }), false, "same connection callback cannot reset sequence");
  receivers.osc.setConnection(null);
  receivers.osc.setConnection(oscFence);
  assert.equal(receivers.osc.acceptRouted({ ...oscFence, sequence: 3, input: payload }), false, "stop or mapping edit retires connection");
  const reconnected = { ...oscFence, input_session: 22 };
  receivers.osc.setConnection(reconnected);
  receivers.osc.setConnection(oscFence);
  assert.equal(receivers.osc.acceptRouted({ ...oscFence, sequence: 99, input: payload }), false, "old session remains retired after restart");
  assert.ok(receivers.osc.acceptRouted({ ...reconnected, sequence: 1, input: payload }));
  transportEpoch = 6;
  assert.equal(receivers.dmx.acceptRouted({ ...dmxFence, sequence: 3, input: payload }), false, "project replacement retires DMX");
  assert.equal(receivers.osc.acceptRouted({ ...reconnected, sequence: 2, input: payload }), false, "project replacement retires OSC");
  assert.throws(() => faderInput.midiFaderConnection({ ...oscFence, input_session: 0 }, "OSC"), /OSC input connection identity/);
  assert.throws(() => faderInput.midiFaderConnection({ ...dmxFence, schema_version: 2 }, "DMX"), /DMX input connection identity/);
}

// OSC/DMX startup data has no effect until its exact native connection is acknowledged.
{
  let currentEpoch = 5;
  const values = [];
  const buffered = faderInput.createMidiFaderInput({ projectEpoch: () => currentEpoch, apply: () => assert.fail("routed buffer never applies fixture faders"), applyRouted: input => values.push(input.value), bufferRoutedUntilConnected: true });
  const identity = { schema_version: 1, project_epoch: 5, input_session: 101 };
  const event = (sequence, extra = {}) => ({ ...identity, sequence, input: { ...payload, value: sequence }, ...extra });
  for (const sequence of [3, 1, 2]) assert.equal(buffered.acceptRouted(event(sequence)), false);
  assert.deepEqual(values, [], "no side effects before acknowledgement");
  buffered.acceptRouted(event(2, { input: { ...payload, value: 99 } }));
  for (const malformed of [
    event(4, { schema_version: 2, input_session: 999 }),
    event(4, { project_epoch: 6, input_session: 999 }),
    event(0, { input_session: 999 }),
    event(4, { input_session: 999, input: { ...payload, value: NaN } }),
    event(4, { input_session: 999, input: { ...payload, clip_slot_id: -1 } }),
  ]) assert.equal(buffered.acceptRouted(malformed), false);
  buffered.setConnection({ ...identity, project_epoch: 6 });
  buffered.setConnection({ ...identity, input_session: 100 });
  assert.deepEqual(values, [], "wrong project/session cannot drain queued data");
  buffered.setConnection(identity);
  assert.deepEqual(values, [1, 2, 3], "exact reply drains validated input once in sequence");
  buffered.setConnection(identity);
  assert.equal(buffered.acceptRouted(event(3)), false);
  assert.equal(buffered.acceptRouted(event(4)), true);
  buffered.acceptRouted(event(1, { input_session: 102 }));
  buffered.setConnection(null);
  buffered.setConnection(identity);
  assert.equal(buffered.acceptRouted(event(5)), false);
  buffered.setConnection({ ...identity, input_session: 102 });
  assert.deepEqual(values, [1, 2, 3, 4], "stop clears unconfirmed startup data");
  buffered.acceptRouted(event(1, { input_session: 103 }));
  currentEpoch = 6;
  buffered.setConnection({ ...identity, input_session: 103 });
  buffered.setConnection({ ...identity, project_epoch: 6, input_session: 104 });
  assert.deepEqual(values, [1, 2, 3, 4], "project replacement discards its predecessor's buffer");
  assert.equal(buffered.acceptRouted(event(1, { project_epoch: 6, input_session: 104 })), true);
}
{
  const values = [];
  const buffered = faderInput.createMidiFaderInput({ projectEpoch: () => 5, apply: () => {}, applyRouted: input => values.push(input.value), bufferRoutedUntilConnected: true });
  const identity = { schema_version: 1, project_epoch: 5, input_session: 201 };
  const event = (input_session, sequence, value) => ({ ...identity, input_session, sequence, input: { ...payload, value } });
  buffered.acceptRouted(event(201, 1, 11));
  buffered.acceptRouted(event(202, 1, 22));
  buffered.acceptRouted(event(201, 2, 12));
  buffered.setConnection(identity);
  assert.deepEqual(values, []);
  assert.equal(buffered.acceptRouted(event(201, 3, 13)), true, "unconfirmed input cannot replace the active connection");
  buffered.setConnection({ ...identity, input_session: 202 });
  assert.deepEqual(values, [13, 22]);
}
{
  const values = [];
  const buffered = faderInput.createMidiFaderInput({ projectEpoch: () => 5, apply: () => {}, applyRouted: input => values.push(input.value), bufferRoutedUntilConnected: true });
  const identity = { schema_version: 1, project_epoch: 5, input_session: 301 };
  const limit = faderInput.routedInputStartupBufferLimit;
  assert.equal(limit, 256);
  for (let sequence = 1; sequence <= limit + 7; sequence++) buffered.acceptRouted({ ...identity, sequence, input: { ...payload, value: sequence } });
  assert.deepEqual(values, []);
  buffered.setConnection(identity);
  assert.deepEqual(values, Array.from({ length: limit }, (_, index) => index + 8));
  const unbuffered = faderInput.createMidiFaderInput({ projectEpoch: () => 5, apply: () => {}, applyRouted: input => values.push(input.value) });
  unbuffered.acceptRouted({ ...identity, sequence: 1, input: payload });
  unbuffered.setConnection(identity);
  assert.equal(values.length, limit, "MIDI's default behavior remains unbuffered");
}

// ---------------------------------------------------------------- dispatcher
const control = (name, kind, extra = {}) => ({ name, kind, value: [1, 0, 0, 0], default: [1, 0, 0, 0], minimum: [0, 0, 0, 0], maximum: [2, 0, 0, 0], labels: [], values: [], ...extra });
const gain = control("gain", "Float");
const pick = control("mode", "Long", { value: [1, 0, 0, 0], minimum: [1, 0, 0, 0], maximum: [4, 0, 0, 0], values: [1, 2, 4], labels: ["A", "B", "C"] });
const flag = control("flag", "Bool", { value: [0, 0, 0, 0], maximum: [1, 0, 0, 0] });
const pulse = control("flash", "Event");
const tint = control("tint", "Color");
const makeStage = (label, controls, enabled = true) => ({ enabled, label, source: "", categories: [], controls });
const layer = (id, slots, extra = {}) => ({
  id, label: `Layer ${id}`, blend_mode: "Normal", state: { enabled: true }, clip_slots: slots.map((slotId) => ({ id: slotId })),
  isf_effect: { ...makeStage("Stage A", [gain, pick, flag, pulse, tint]), stack: [makeStage("Stage B", [control("depth", "Float")], false)] },
  ...extra,
});
let layers = [layer(1, [101, 102]), layer(2, [201])];
let catalog = { effect_chains: [{ id: 5, scope: { scope: "layer", layer_id: 1 }, bypassed: false, stages: [{ id: 900 }, { id: 901 }] }], effect_presets: [], layer_groups: [], transition_buses: [] };
const log = [];
let takeEnabled = false;
let locked = false;
let kind = "Cut";
let milliunits = 1000;
let queueResult = { ok: true };
const pending = [];
let projectEpoch = 1;
let fxHold = false;
const fxPending = [];
const controlTimes = [];
let idleGate = Promise.resolve(true);
let queueHold = false;
const queuePending = [];
const heldFx = async () => { if (fxHold) await new Promise((done) => fxPending.push(done)); };
const deps = {
  layers: () => layers,
  effectCatalog: () => catalog,
  clipRuntime: () => null,
  projectEpoch: () => projectEpoch,
  fxIdle: () => idleGate,
  takeEnabled: () => takeEnabled,
  selectLayer: (id) => log.push(["selectLayer", id]),
  selectSlot: (id) => log.push(["selectSlot", id]),
  queueSlot: async (layerId, slotId) => { log.push(["queue", layerId, slotId]); if (queueHold) await new Promise((done) => queuePending.push(done)); return queueResult; },
  take: async () => { log.push(["take"]); return true; },
  stopLayer: async (id) => { log.push(["stop", id]); },
  setBlendMode: (id, mode) => new Promise((done) => { log.push(["blend", id, mode]); pending.push(done); }),
  transitionKind: () => kind,
  transitionDurationMilliunits: () => milliunits,
  transitionLocked: () => locked,
  setTransitionKind: (next) => { log.push(["kind", next]); kind = next; },
  setTransitionDurationMilliunits: (next) => { log.push(["time", next]); milliunits = next; },
  setIsfStageEnabled: async (layerId, index, enabled) => { log.push(["stageEnabled", layerId, index, enabled]); await heldFx(); },
  setIsfControl: async (layerId, index, name, value) => { controlTimes.push(Date.now()); log.push(["control", layerId, index, name, value]); await heldFx(); },
  triggerIsfEvent: async (layerId, index, name) => { log.push(["event", layerId, index, name]); await heldFx(); },
  report: (message) => log.push(["report", message]),
};
const dispatcher = dispatcherModule.createPerformControlDispatcher(deps);
const drain = async () => { for (let i = 0; i < 12; i++) await Promise.resolve(); };
const act = (action, extra = {}) => ({ action, layer_id: null, clip_slot_id: null, isf_stage_id: null, control: null, position: 1, value: 1, active: true, toggle: false, ...extra });
const take = () => log.splice(0, log.length);

// A pad press selects the layer and clip, then queues it by id.
assert.deepEqual(await dispatcher.dispatch(act("VideoClipSlotTrigger", { layer_id: 1, clip_slot_id: 102 })), { handled: true });
assert.deepEqual(take(), [["selectLayer", 1], ["selectSlot", 102], ["queue", 1, 102]]);
// The same stable ids keep working after the bank is reordered (a column index would retarget).
layers = [layer(1, [102, 101]), layer(2, [201])];
await dispatcher.dispatch(act("VideoClipSlotTrigger", { layer_id: 1, clip_slot_id: 102 }));
assert.deepEqual(take().at(-1), ["queue", 1, 102], "reordering the columns does not retarget a mapping");
assert.deepEqual(await dispatcher.dispatch(act("VideoClipSlotTrigger", { layer_id: 1, clip_slot_id: 999 })), { handled: false, reason: "missing-slot" });
assert.deepEqual(await dispatcher.dispatch(act("VideoClipSlotTrigger", { layer_id: 9, clip_slot_id: 101 })), { handled: false, reason: "missing-layer" });
assert.deepEqual(await dispatcher.dispatch(act("VideoClipSlotTrigger", { layer_id: 2, clip_slot_id: 101 })), { handled: false, reason: "missing-slot" }, "a slot id is never interpreted against another layer");
assert.deepEqual(take(), [], "a stale target issues no command");

// Take needs something queued: the runtime says so, or this dispatcher just queued it.
const fresh = dispatcherModule.createPerformControlDispatcher(deps);
assert.deepEqual(await fresh.dispatch(act("VideoTake")), { handled: false, reason: "nothing-queued" });
assert.deepEqual(take(), [["report", "Take ignored: no clip is queued."]]);
takeEnabled = true;
assert.deepEqual(await fresh.dispatch(act("VideoTake")), { handled: true });
assert.deepEqual(take(), [["take"]]);
takeEnabled = false;
queueResult = null;
await fresh.dispatch(act("VideoClipSlotTrigger", { layer_id: 1, clip_slot_id: 101 }));
take();
assert.deepEqual(await fresh.dispatch(act("VideoTake")), { handled: false, reason: "nothing-queued" }, "a rejected queue does not arm Take");
take();
queueResult = { ok: true };
// A pad and the Take right after it keep their order even before the runtime poll catches up.
const quick = [fresh.dispatch(act("VideoClipSlotTrigger", { layer_id: 1, clip_slot_id: 101 })), fresh.dispatch(act("VideoTake"))];
await Promise.all(quick);
assert.deepEqual(take().filter((entry) => ["queue", "take"].includes(entry[0])), [["queue", 1, 101], ["take"]]);
await fresh.dispatch(act("VideoLayerStop", { layer_id: 2 }));
assert.deepEqual(take(), [["stop", 2]]);
assert.deepEqual(await fresh.dispatch(act("VideoLayerStop", { layer_id: 77 })), { handled: false, reason: "missing-layer" });

// Selectors: the input position chooses the entry; an unchanged choice is not re-sent.
assert.deepEqual(await dispatcher.dispatch(act("VideoLayerBlendMode", { layer_id: 1, value: 0 })), { handled: false, reason: "unchanged" });
const blendCalls = [
  dispatcher.dispatch(act("VideoLayerBlendMode", { layer_id: 1, value: 0.15 })),
  dispatcher.dispatch(act("VideoLayerBlendMode", { layer_id: 1, value: 0.35 })),
  dispatcher.dispatch(act("VideoLayerBlendMode", { layer_id: 1, value: 0.55 })),
  dispatcher.dispatch(act("VideoLayerBlendMode", { layer_id: 1, value: 1 })),
];
await drain();
assert.deepEqual(take(), [["blend", 1, "Add"]], "only the first blend is in flight");
pending.shift()();
await drain();
assert.deepEqual(take(), [["blend", 1, "SoftLight"]], "a controller's newest value wins over the ones it skipped");
pending.shift()();
await Promise.all(blendCalls);
kind = "Cut";
await dispatcher.dispatch(act("VideoTransitionKind", { value: 0.2 }));
assert.deepEqual(take(), [["kind", "Crossfade"]]);
assert.deepEqual(await dispatcher.dispatch(act("VideoTransitionKind", { value: 0.2 })), { handled: false, reason: "unchanged" });
locked = true;
assert.deepEqual(await dispatcher.dispatch(act("VideoTransitionKind", { value: 0.9 })), { handled: false, reason: "transition-running" });
assert.deepEqual(await dispatcher.dispatch(act("VideoTransitionTime", { value: 2 })), { handled: false, reason: "transition-running" });
locked = false;
await dispatcher.dispatch(act("VideoTransitionTime", { value: 2.5 }));
await dispatcher.dispatch(act("VideoTransitionTime", { value: 9999 }));
await dispatcher.dispatch(act("VideoTransitionTime", { value: -3 }));
assert.deepEqual(take(), [["time", 2500], ["time", 600000], ["time", 1]], "time is stored in thousandths and clamped to the editor's bounds");

// Effect stages resolve by stable id, not by position.
assert.equal(dispatcherModule.isfStageIndexForId(catalog, 1, 901), 1);
assert.equal(dispatcherModule.isfStageIdForIndex(catalog, 1, 0), 900);
assert.equal(dispatcherModule.isfStageIndexForId(catalog, 2, 900), -1, "a stage id never resolves on another layer");
assert.equal(dispatcherModule.isfStageIndexForId(undefined, 1, 900), -1);
await dispatcher.dispatch(act("VideoIsfStageEnabled", { layer_id: 1, isf_stage_id: 901, toggle: true }));
assert.deepEqual(take(), [["stageEnabled", 1, 1, true]], "a press flips the disabled stage on");
await dispatcher.dispatch(act("VideoIsfStageEnabled", { layer_id: 1, isf_stage_id: 900, toggle: true }));
assert.deepEqual(take(), [["stageEnabled", 1, 0, false]], "and the enabled stage off");
assert.deepEqual(await dispatcher.dispatch(act("VideoIsfStageEnabled", { layer_id: 1, isf_stage_id: 900, active: true })), { handled: false, reason: "unchanged" });
catalog = { ...catalog, effect_chains: [{ ...catalog.effect_chains[0], stages: [{ id: 901 }, { id: 900 }] }] };
layers = [layer(1, [101, 102], { isf_effect: { ...makeStage("Stage B", [control("depth", "Float")], false), stack: [makeStage("Stage A", [gain, pick, flag, pulse, tint])] } }), layer(2, [201])];
await dispatcher.dispatch(act("VideoIsfStageEnabled", { layer_id: 1, isf_stage_id: 901, active: true }));
assert.deepEqual(take(), [["stageEnabled", 1, 0, true]], "after the chain is reordered the same stage id still reaches its stage");
assert.deepEqual(await dispatcher.dispatch(act("VideoIsfStageEnabled", { layer_id: 1, isf_stage_id: 12345, active: true })), { handled: false, reason: "missing-stage" });
assert.deepEqual(await dispatcher.dispatch(act("VideoIsfStageEnabled", { layer_id: 1, active: true })), { handled: false, reason: "missing-stage" });
layers = [layer(1, [101, 102]), layer(2, [201])];
catalog = { ...catalog, effect_chains: [{ ...catalog.effect_chains[0], stages: [{ id: 900 }, { id: 901 }] }] };

// Parameters: the mapping range is a fraction of the control's own range.
const param = (name, value, extra = {}) => act("VideoIsfParam", { layer_id: 1, isf_stage_id: 900, control: name, value, ...extra });
await dispatcher.dispatch(param("gain", 0.25));
assert.deepEqual(take(), [["control", 1, 0, "gain", [0.5, 0, 0, 0]]]);
assert.deepEqual(await dispatcher.dispatch(param("gain", 0.5)), { handled: false, reason: "unchanged" }, "the control already sits at 1");
await dispatcher.dispatch(param("mode", 0.9));
assert.deepEqual(take(), [["control", 1, 0, "mode", [4, 0, 0, 0]]], "an enumerated control picks one of its own values");
await dispatcher.dispatch(param("flag", 0.9));
assert.deepEqual(take(), [["control", 1, 0, "flag", [1, 0, 0, 0]]]);
await dispatcher.dispatch(param("flash", 1, { active: true }));
assert.deepEqual(take(), [["event", 1, 0, "flash"]], "an event control pulses on a press");
assert.deepEqual(await dispatcher.dispatch(param("flash", 0, { active: false })), { handled: false, reason: "unchanged" });
assert.deepEqual(await dispatcher.dispatch(param("tint", 0.5)), { handled: false, reason: "missing-control" }, "colour has no single hardware value");
assert.deepEqual(await dispatcher.dispatch(param("missing", 0.5)), { handled: false, reason: "missing-control" });
await dispatcher.dispatch(act("VideoIsfParam", { layer_id: 1, isf_stage_id: 901, control: "depth", value: 1 }));
assert.deepEqual(take(), [["control", 1, 1, "depth", [2, 0, 0, 0]]], "stage 2 of the stack addresses its own controls");
assert.equal(dispatcherModule.isfControlValueAt(gain, 2), 2, "positions clamp");
assert.equal(dispatcherModule.isfControlValueAt(tint, 0.5), null);

// A controller's newest value wins even while the rendered state still shows the old one.
{
  const sweep = dispatcherModule.createPerformControlDispatcher(deps);
  const first = sweep.dispatch(act("VideoLayerBlendMode", { layer_id: 1, value: 0.15 }));
  await drain();
  const back = sweep.dispatch(act("VideoLayerBlendMode", { layer_id: 1, value: 0 }));
  await drain();
  assert.deepEqual(take(), [["blend", 1, "Add"]], "Add is in flight");
  pending.shift()();
  await drain();
  assert.deepEqual(take(), [["blend", 1, "Normal"]], "Add and back to Normal ends on Normal although the layer still renders Normal");
  pending.shift()();
  await Promise.all([first, back]);
  assert.deepEqual(await sweep.dispatch(act("VideoLayerBlendMode", { layer_id: 1, value: 0 })), { handled: false, reason: "unchanged" }, "the lane forgets its intent once idle");
}

// Two presses of a stage toggle inside one command round trip still alternate.
{
  const presses = dispatcherModule.createPerformControlDispatcher(deps);
  fxHold = true;
  const press = () => presses.dispatch(act("VideoIsfStageEnabled", { layer_id: 1, isf_stage_id: 901, toggle: true }));
  const one = press();
  await drain();
  const two = press();
  await drain();
  assert.deepEqual(take(), [["stageEnabled", 1, 1, true]], "the first press enables the disabled stage");
  fxPending.shift()();
  await drain();
  assert.deepEqual(take(), [["stageEnabled", 1, 1, false]], "the second press, made before the first finished, flips it back");
  fxPending.shift()();
  await Promise.all([one, two]);
  fxHold = false;
}

// The stage index is resolved after the live FX queue is idle, so a move that was running cannot misdirect it.
{
  const gated = dispatcherModule.createPerformControlDispatcher(deps);
  let open;
  idleGate = new Promise((done) => { open = done; });
  const call = gated.dispatch(act("VideoIsfStageEnabled", { layer_id: 1, isf_stage_id: 901, active: true }));
  await drain();
  assert.deepEqual(take(), [], "nothing runs while an effect command is in flight");
  const savedLayers = layers;
  const savedCatalog = catalog;
  catalog = { ...catalog, effect_chains: [{ ...catalog.effect_chains[0], stages: [{ id: 901 }, { id: 900 }] }] };
  layers = [layer(1, [101, 102], { isf_effect: { ...makeStage("Stage B", [control("depth", "Float")], false), stack: [makeStage("Stage A", [gain, pick, flag, pulse, tint])] } }), layer(2, [201])];
  open(true);
  await call;
  assert.deepEqual(take(), [["stageEnabled", 1, 0, true]], "stage 901 moved to index 0 while waiting and is addressed there");
  layers = savedLayers;
  catalog = savedCatalog;
  idleGate = Promise.resolve(false);
  const skipped = await gated.dispatch(act("VideoIsfStageEnabled", { layer_id: 1, isf_stage_id: 900, active: false }));
  assert.deepEqual(skipped, { handled: true });
  assert.deepEqual(take().map((entry) => entry[0]), ["report"], "a command that never found the FX queue idle is reported, not applied");
  idleGate = Promise.resolve(true);
}

// A pulse already running owns the next one; a controller stream is a single pulse.
{
  const pulses = dispatcherModule.createPerformControlDispatcher(deps);
  fxHold = true;
  const first = pulses.dispatch(act("VideoIsfParam", { layer_id: 1, isf_stage_id: 900, control: "flash", active: true }));
  await drain();
  const dropped = pulses.dispatch(act("VideoIsfParam", { layer_id: 1, isf_stage_id: 900, control: "flash", active: true }));
  await drain();
  assert.deepEqual(take(), [["event", 1, 0, "flash"]]);
  fxPending.shift()();
  await Promise.all([first, dropped]);
  fxHold = false;
  await pulses.dispatch(act("VideoIsfParam", { layer_id: 1, isf_stage_id: 900, control: "flash", active: true }));
  assert.deepEqual(take(), [["event", 1, 0, "flash"]], "the next pulse runs once the first has finished");
}

// Authoritative ISF commits are thinned to the spacing, newest value last.
{
  const thinned = dispatcherModule.createPerformControlDispatcher(deps);
  controlTimes.length = 0;
  const calls = [0.1, 0.2, 0.3, 0.4].map((position) => thinned.dispatch(act("VideoIsfParam", { layer_id: 1, isf_stage_id: 900, control: "gain", value: position })));
  await Promise.all(calls);
  assert.equal(controlTimes.length, 2, "four quick values become the first and the newest");
  assert.ok(controlTimes[1] - controlTimes[0] >= dispatcherModule.isfParamCommitSpacingMs - 20, "the second commit waited out the spacing: " + (controlTimes[1] - controlTimes[0]));
  assert.deepEqual(take().map((entry) => entry[4][0]), [0.2, 0.8], "the first value goes out at once; the last arrival replaces the ones in between");
}

// Queued commands never run on another project, and a Take needs a queue from this project.
{
  const switching = dispatcherModule.createPerformControlDispatcher(deps);
  queueHold = true;
  const padOne = switching.dispatch(act("VideoClipSlotTrigger", { layer_id: 1, clip_slot_id: 101 }));
  await drain();
  const padTwo = switching.dispatch(act("VideoClipSlotTrigger", { layer_id: 1, clip_slot_id: 102 }));
  const takeNow = switching.dispatch(act("VideoTake"));
  projectEpoch = 2;
  queueHold = false;
  queuePending.shift()();
  const results = await Promise.all([padOne, padTwo, takeNow]);
  assert.deepEqual(take().filter((entry) => ["queue", "take", "selectLayer"].includes(entry[0])), [["selectLayer", 1], ["queue", 1, 101]], "only the command already running finished; the queued pad and Take were dropped");
  assert.deepEqual(results[2], { handled: false, reason: "unsupported" });
  projectEpoch = 1;
}

// Selection happens in order with each command, so a Take acts on the layer of the pad before it.
{
  const ordering = dispatcherModule.createPerformControlDispatcher(deps);
  queueHold = true;
  const padA = ordering.dispatch(act("VideoClipSlotTrigger", { layer_id: 1, clip_slot_id: 101 }));
  const padB = ordering.dispatch(act("VideoClipSlotTrigger", { layer_id: 2, clip_slot_id: 201 }));
  await drain();
  assert.deepEqual(take(), [["selectLayer", 1], ["selectSlot", 101], ["queue", 1, 101]], "the second pad has not selected yet");
  queueHold = false;
  queuePending.shift()();
  await Promise.all([padA, padB]);
  assert.deepEqual(take(), [["selectLayer", 2], ["selectSlot", 201], ["queue", 2, 201]]);
}

// A beat that fires Take with nothing queued stays quiet; a controller press explains itself.
{
  const quiet = dispatcherModule.createPerformControlDispatcher(deps);
  takeEnabled = false;
  assert.deepEqual(await quiet.dispatch(act("VideoTake", { source: "beat" })), { handled: false, reason: "nothing-queued" });
  assert.deepEqual(take(), []);
  await quiet.dispatch(act("VideoTake"));
  assert.deepEqual(take(), [["report", "Take ignored: no clip is queued."]]);
}


// Source leases are checked after asynchronous waits, without fencing ordinary gesture revisions.
{
  const writes = [];
  let currentEpoch = 30;
  let releaseIdle;
  let firstWait = true;
  const leaseDispatcher = dispatcherModule.createPerformControlDispatcher({
    ...deps,
    layers: () => [layer(1, [101, 102])],
    effectCatalog: () => ({ effect_chains: [{ id: 5, scope: { scope: "layer", layer_id: 1 }, bypassed: false, stages: [{ id: 900 }, { id: 901 }] }], effect_presets: [], layer_groups: [], transition_buses: [] }),
    projectEpoch: () => currentEpoch,
    fxIdle: () => firstWait ? (firstWait = false, new Promise(resolve => { releaseIdle = resolve; })) : Promise.resolve(true),
    setIsfControl: async (...args) => writes.push(args),
    setIsfStageEnabled: async (...args) => writes.push(args),
    triggerIsfEvent: async (...args) => writes.push(args),
  });
  let current = true;
  const old = leaseDispatcher.dispatch(act("VideoIsfParam", { layer_id: 1, isf_stage_id: 900, control: "gain", value: 0.8, sourceIsCurrent: () => current }));
  await drain();
  assert.equal(typeof releaseIdle, "function");
  current = false;
  const next = leaseDispatcher.dispatch(act("VideoIsfParam", { layer_id: 1, isf_stage_id: 900, control: "gain", value: 0.8, sourceIsCurrent: () => true }));
  releaseIdle(true);
  await Promise.all([old, next]);
  assert.equal(writes.length, 1, "retired source cannot write; the new source's identical final value is retained");
  assert.equal(writes[0][3][0], 1.6, "the new source applies the normalized 0.8 position within the control range");
  for (const input of [
    act("VideoIsfParam", { layer_id: 1, isf_stage_id: 900, control: "gain", value: 0.7 }),
    act("VideoIsfParam", { layer_id: 1, isf_stage_id: 900, control: "flash", value: 1 }),
    act("VideoIsfStageEnabled", { layer_id: 1, isf_stage_id: 901, active: true }),
  ]) {
    firstWait = true;
    const work = leaseDispatcher.dispatch(input);
    await drain();
    currentEpoch++;
    releaseIdle(true);
    await work;
  }
  assert.equal(writes.length, 1, "project replacement during fxIdle rejects parameter, pulse and stage writes");
}
{
  let currentEpoch = 5;
  const leases = [];
  const receiver = faderInput.createMidiFaderInput({ projectEpoch: () => currentEpoch, apply: () => {}, applyRouted: (_input, stillCurrent) => leases.push(stillCurrent) });
  const identity = { schema_version: 1, project_epoch: 5, input_session: 601 };
  receiver.setConnection(identity);
  receiver.acceptRouted({ ...identity, sequence: 1, input: payload });
  assert.equal(leases[0](), true);
  receiver.setConnection(null);
  assert.equal(leases[0](), false);
  receiver.setConnection({ ...identity, input_session: 602 });
  assert.equal(leases[0](), false);
  receiver.acceptRouted({ ...identity, input_session: 602, sequence: 1, input: payload });
  currentEpoch++;
  assert.equal(leases[1](), false);
}

// ---------------------------------------------------------------- learn model
const slotTarget = { action: "VideoClipSlotTrigger", label: "Layer 1 clip 2", layer_id: 1, clip_slot_id: 102 };
const stageTarget = { action: "VideoIsfStageEnabled", label: "Layer 1 FX 1", layer_id: 1, isf_stage_id: 900 };
const paramTarget = { action: "VideoIsfParam", label: "Layer 1 gain", layer_id: 1, isf_stage_id: 900, attribute: "gain" };
const blendTarget = { action: "VideoLayerBlendMode", label: "Layer 1 blend", layer_id: 1 };
const timeTarget = { action: "VideoTransitionTime", label: "Transition time", low: 0.1, high: 10 };
const learned = { channel: 2, message: "ControlChange", number: 21, value: 90 };
const [slotMapping] = learn.midiMappingsFromLearnedControl([slotTarget], learned);
assert.deepEqual(slotMapping, { channel: 2, message: "ControlChange", number: 21, action: "VideoClipSlotTrigger", fixture_id: null, attribute: null, group_id: null, cue_id: null, layer_id: 1, output_id: null, video_param: null, cue_point_index: null, duration_ms: null, low: 0, high: 1, clip_slot_id: 102 });
assert.ok(!("isf_stage_id" in slotMapping), "ids are written only for the action that owns them");
const [paramMapping] = learn.midiMappingsFromLearnedControl([paramTarget], learned);
assert.deepEqual([paramMapping.action, paramMapping.attribute, paramMapping.isf_stage_id, paramMapping.layer_id], ["VideoIsfParam", "gain", 900, 1]);
assert.deepEqual(learn.midiMappingsFromLearnedControl([timeTarget], learned).map((mapping) => [mapping.low, mapping.high]), [[0.1, 10]]);
const [legacy] = learn.midiMappingsFromLearnedControl([{ action: "LightingMaster", label: "Master" }], learned);
assert.ok(!("clip_slot_id" in legacy) && !("isf_stage_id" in legacy), "every other mapping keeps its exact shape");
// A mapping drives a target by identity; another clip, stage or control does not match.
assert.ok(learn.midiMappingDrivesTarget(slotMapping, slotTarget));
assert.ok(!learn.midiMappingDrivesTarget(slotMapping, { ...slotTarget, clip_slot_id: 101 }));
assert.ok(!learn.midiMappingDrivesTarget(slotMapping, { ...slotTarget, layer_id: 2 }));
assert.ok(learn.midiMappingDrivesTarget(paramMapping, paramTarget));
assert.ok(!learn.midiMappingDrivesTarget(paramMapping, { ...paramTarget, attribute: "pan" }));
assert.ok(!learn.midiMappingDrivesTarget(paramMapping, { ...paramTarget, isf_stage_id: 901 }));
assert.ok(!learn.midiMappingDrivesTarget(paramMapping, stageTarget));
// Every Learn transport preserves the same stable Perform target identities.
const allPerformTargets = [slotTarget, { action: "VideoTake", label: "Take" }, { action: "VideoLayerStop", label: "Stop", layer_id: 1 }, blendTarget, { action: "VideoTransitionKind", label: "Transition" }, timeTarget, stageTarget, paramTarget];
const oscLearned = { address: "/perform", value: 1, argument_count: 1 };
const dmxLearned = { universe: 0, channel: 1, value: 255 };
const oscPerform = learn.oscMappingsFromLearnedControl(allPerformTargets, oscLearned);
const dmxPerform = learn.dmxMappingsFromLearnedControl(allPerformTargets, dmxLearned);
for (const mappings of [oscPerform, dmxPerform]) {
  assert.deepEqual(mappings.map(mapping => mapping.action), allPerformTargets.map(target => target.action));
  assert.equal(mappings[0].clip_slot_id, 102);
  assert.deepEqual([mappings[6].layer_id, mappings[6].isf_stage_id], [1, 900]);
  assert.deepEqual([mappings[7].layer_id, mappings[7].isf_stage_id, mappings[7].attribute], [1, 900, "gain"]);
  assert.deepEqual([mappings[5].low, mappings[5].high], [0.1, 10]);
  assert.deepEqual(JSON.parse(JSON.stringify(mappings)), mappings, "project JSON retains stable Perform targets");
}
for (const mode of ["midi", "osc", "dmx"]) assert.equal(learn.controlMappingTargetsForMode(mode, allPerformTargets).length, 8);
assert.deepEqual(learn.controlMappingTargetsForMode("osc", [{ ...slotTarget, midi_only: true }]), [], "explicit transport restrictions remain enforced");
for (const mapping of [learn.oscMappingsFromLearnedControl([{ action: "LightingMaster", label: "Master" }], oscLearned)[0], learn.dmxMappingsFromLearnedControl([{ action: "LightingMaster", label: "Master" }], dmxLearned)[0]]) assert.ok(!("clip_slot_id" in mapping) && !("isf_stage_id" in mapping), "legacy mappings keep exact JSON shape");
const data = learn.controlMappingTargetData(slotTarget);
const encoded = JSON.parse(data["data-control-map-target"]);
assert.equal(encoded[0].midi_only, undefined, "Perform controls are available in all Learn modes");
assert.equal(encoded[0].clip_slot_id, 102);
assert.equal(JSON.parse(learn.controlMappingTargetData({ action: "VideoMaster", label: "Master" })["data-control-map-target"])[0].midi_only, undefined);
// Beat division mappings exist for triggers only.
const beats = learn.beatDivisionMappingsFromTargets([slotTarget, stageTarget, blendTarget, paramTarget, { action: "TriggerCue", label: "Scene", cue_id: 3 }], 4);
assert.deepEqual(beats.map((mapping) => [mapping.action, mapping.message, mapping.number, mapping.channel]), [["VideoClipSlotTrigger", "BeatDivision", 4, null], ["VideoIsfStageEnabled", "BeatDivision", 4, null]]);
assert.equal(beats[0].clip_slot_id, 102);
assert.ok(learn.midiMappingDrivesTarget(beats[0], slotTarget), "a beat assignment is found by the same identity as a MIDI one");
assert.equal(perform.midiSourceDescription(beats[0]), "1 bar");
assert.equal(perform.midiSourceDescription(slotMapping), "CC 21 · ch 3");
assert.equal(perform.midiSourceDescription({ message: "NoteOn", number: 36, channel: null }), "Note 36 · any channel");

// ---------------------------------------------------------------- markup
const read = (path) => readFileSync(resolve(root, path), "utf8");
const matrix = read("components/VideoPerformMatrix.tsx");
const desk = read("components/VideoPerformDesk.tsx");
const quickFx = read("components/VideoPerformQuickFx.tsx");
const editor = read("components/VideoFxParameterEditor.tsx");
const mappingPanel = read("components/MidiControlMappingPanel.tsx");
assert.match(mappingPanel, /<Show when=\{mapping\.message !== "BeatDivision" && !isPerformRoutedAction\(mapping\.action\)\}>\s*<details class="mappingFeedbackEditor">/, "tempo sources and renderer-owned targets offer no controller feedback");
assert.match(desk, /disabled=\{Boolean\(props\.clipSlotBank\.runtime\?\.transition\) && !props\.performContext\?\.controlLearnActive\}/, "transition controls stay reachable for Learn");
assert.match(matrix, /data-video-clip-slot-pad-index=\{cell\.index\} \{\.\.\.controlMappingTargetData\(cell\.slot \? \{ action: "VideoClipSlotTrigger"[\s\S]{0,160}clip_slot_id: cell\.slot\.id \} : null\)\}/, "every occupied pad targets its stable slot id");
assert.match(matrix, /data-video-layer-stop=\{layer\(\)\.id\} \{\.\.\.controlMappingTargetData\(\{ action: "VideoLayerStop"/);
assert.match(matrix, /data-video-layer-blend=\{layer\(\)\.id\} \{\.\.\.controlMappingTargetData\(\{ action: "VideoLayerBlendMode"/);
assert.match(desk, /data-video-perform-transition-kind[\s\S]{0,200}action: "VideoTransitionKind"/);
assert.match(desk, /data-video-perform-transition-duration[\s\S]{0,260}action: "VideoTransitionTime"/);
assert.match(desk, /\{\.\.\.controlMappingTargetData\(\{ action: "VideoTake", label: "Take" \}\)\}/);
assert.match(desk, /disabled=\{!props\.clipSlotTake\.enabled && !props\.performContext\?\.controlLearnActive\}/, "Take stays reachable for Learn");
assert.doesNotMatch(desk, /not supported by the current control model/, "the legend no longer says these targets are unsupported");
assert.doesNotMatch(quickFx, /MIDI Learn is unavailable/);
assert.match(quickFx, /action: "VideoIsfStageEnabled"[\s\S]{0,200}isf_stage_id: stage\.stageId/);
assert.match(quickFx, /stageId=\{stage\(\)\.stageId\}/);
assert.match(editor, /action: "VideoIsfParam"[\s\S]{0,300}isf_stage_id: props\.stageId[\s\S]{0,60}attribute: control\.name/);
assert.equal((editor.match(/controlMappingTargetData\(mapTarget\(control\)\)/g) ?? []).length, 4, "event, bool, enumerated and scalar controls are targets");
assert.match(editor, /props\.auditioning/, "a Preview-only audition draft is never a MIDI target");

console.log("PASS: Perform MIDI/OSC/DMX targets use stable ids; renderer routing, selectors, parameters, beat division triggers and the Rust/TypeScript contract hold");
