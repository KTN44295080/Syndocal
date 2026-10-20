// Layer-group Learn regression: identity, transport parity, queued-write safety,
// and the real controller's preflight/authoritative-catalog boundary.
import assert from "node:assert/strict";
import { readFileSync, existsSync } from "node:fs";
import { resolve, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import { createRequire } from "node:module";
import ts from "typescript";

const require = createRequire(import.meta.url);
const root = resolve(dirname(fileURLToPath(import.meta.url)), "../src");
const modules = new Map();
function load(file) {
  if (modules.has(file)) return modules.get(file);
  const exports = {};
  modules.set(file, exports);
  const js = ts.transpileModule(readFileSync(file, "utf8"), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
  }).outputText;
  new Function("require", "exports", js)((name) => {
    if (!name.startsWith(".")) return require(name);
    const target = resolve(dirname(file), name.endsWith(".ts") ? name : name + ".ts");
    assert.ok(existsSync(target), name);
    return load(target);
  }, exports);
  return exports;
}
const perform = load(resolve(root, "controlMappingPerform.ts"));
const learn = load(resolve(root, "controlMappingLearn.ts"));
const { createPerformControlDispatcher } = load(resolve(root, "performControlDispatcher.ts"));
const { createVideoRuntimeController } = load(resolve(root, "createVideoRuntimeController.ts"));
const { controlMappingTargetLabel } = load(resolve(root, "controlMappingLabels.ts"));
const actions = ["VideoGroupOpacity", "VideoGroupSolo", "VideoGroupBypass", "VideoGroupBlendMode", "VideoGroupEnabled"];
const flags = actions.filter(perform.isPerformToggleAction);
const targets = actions.map(action => ({ action, video_group_id: 41, label: "Group A " + action }));
const midiInput = { channel: 2, message: "ControlChange", number: 11, value: 64 };
const factories = [
  list => learn.midiMappingsFromLearnedControl(list, midiInput),
  list => learn.oscMappingsFromLearnedControl(list, { address: "/group", value: 0.5, argument_count: 1 }),
  list => learn.dmxMappingsFromLearnedControl(list, { universe: 1, channel: 8, value: 128 }),
];
for (const make of factories) {
  const mappings = make(targets);
  assert.deepEqual(mappings.map(mapping => [mapping.action, mapping.video_group_id, mapping.group_id]), actions.map(action => [action, 41, null]));
  assert.deepEqual(JSON.parse(JSON.stringify(mappings)), mappings, "all sources persist stable video ids");
  assert.ok(!("video_group_id" in make([{ action: "LightingMaster", label: "Master" }])[0]), "legacy JSON shape is unchanged");
}
for (const mode of ["midi", "osc", "dmx"]) assert.deepEqual(learn.controlMappingTargetsForMode(mode, targets), targets);
const [mapping] = factories[0](targets);
assert.ok(learn.midiMappingDrivesTarget(mapping, { ...targets[0], label: "Renamed group" }));
assert.ok(!learn.midiMappingDrivesTarget(mapping, { ...targets[0], video_group_id: 42 }));
assert.notEqual(learn.controlMappingTargetKey(targets[0]), learn.controlMappingTargetKey({ ...targets[0], video_group_id: 42 }));
assert.notEqual(perform.performMappingKey(mapping), perform.performMappingKey({ ...mapping, video_group_id: 42 }));
for (const [index, entry] of factories[0](targets).entries()) {
  assert.equal(controlMappingTargetLabel(entry), "Video Group 41 " + ["opacity", "solo", "bypass", "blend", "enabled"][index]);
}
assert.deepEqual(JSON.parse(learn.controlMappingTargetData(targets[0])["data-control-map-target"])[0].video_group_id, 41);
const beats = learn.beatDivisionMappingsFromTargets(targets, 2);
assert.deepEqual(beats.map(mapping => mapping.action), flags, "only flags take tempo triggers");
for (const beat of beats) {
  const input = perform.performRoutedInputFromBeatMapping(beat);
  assert.equal(input.video_group_id, 41);
  assert.equal(input.toggle, true);
  assert.equal(input.source, "beat");
}
const fired = [];
const clock = perform.createBeatTriggerScheduler({ mappings: () => beats, dispatch: input => fired.push(input) });
clock.tick(0.9);
assert.equal(clock.tick(1), 3);
assert.equal(clock.tick(1.1), 0);
assert.deepEqual(fired.map(input => [input.action, input.video_group_id]), flags.map(action => [action, 41]));
assert.equal(perform.performRoutedInputFromBeatMapping({ ...beats[0], video_group_id: null }), null);

const input = (action, extra = {}) => ({
  action, video_group_id: 41, position: 1, value: 1, active: true, toggle: false, ...extra,
});
for (const action of actions) assert.equal(perform.performRoutedInputFromPayload(input(action)).video_group_id, 41);
for (const extra of [
  { video_group_id: undefined }, { video_group_id: 0 }, { video_group_id: -1 }, { video_group_id: 1.5 },
  { video_group_id: "41" }, { video_group_id: Number.MAX_SAFE_INTEGER + 1 },
  { layer_id: 1 }, { clip_slot_id: 3 }, { isf_stage_id: 4 }, { control: "solo" }, { value: NaN },
]) assert.equal(perform.performRoutedInputFromPayload(input(actions[0], extra)), null, JSON.stringify(extra));
assert.equal(perform.performRoutedInputFromPayload(input("VideoTake")), null, "a video group id never aliases another target");

const group = (id, extra = {}) => ({ id, label: "Group " + id, composition_id: 1, layer_ids: [id],
  enabled: true, opacity: 1, blend_mode: "Normal", solo: false, bypassed: false, transform: null, ...extra });
const emptyCatalog = () => ({ effect_chains: [], effect_presets: [], layer_groups: [group(41), group(42)], transition_buses: [] });
const deferred = () => { let resolve; const promise = new Promise(done => { resolve = done; }); return { promise, resolve }; };
const drain = async () => { for (let i = 0; i < 20; i++) await Promise.resolve(); };
function fixture() {
  let catalog = emptyCatalog(), epoch = 1, suspended = false, idle = Promise.resolve(true), hold = null, concurrency = 0, maxConcurrency = 0;
  const calls = [];
  const dispatcher = createPerformControlDispatcher({
    layers: () => [],
    effectCatalog: () => catalog,
    projectEpoch: () => epoch,
    inputSuspended: () => suspended,
    fxIdle: () => idle,
    patchGroup: async (id, patch, isCurrent) => {
      concurrency++;
      maxConcurrency = Math.max(maxConcurrency, concurrency);
      calls.push([id, patch]);
      if (hold) { const gate = hold; hold = null; await gate; }
      if (isCurrent()) catalog = { ...catalog, layer_groups: catalog.layer_groups.map(entry => entry.id === id ? { ...entry, ...patch } : entry) };
      concurrency--;
      return true;
    },
  });
  return { dispatcher, calls, catalog: () => catalog, replaceCatalog: value => { catalog = value; },
    replaceProject: () => { epoch++; }, suspend: value => { suspended = value; },
    idle: value => { idle = value; }, hold: value => { hold = value; }, maxConcurrency: () => maxConcurrency };
}
{
  const f = fixture();
  f.replaceCatalog({ ...f.catalog(), layer_groups: [group(42), group(41)] });
  await f.dispatcher.dispatch(input("VideoGroupOpacity", { value: 0.25 }));
  assert.deepEqual(f.calls, [[41, { opacity: 0.25 }]], "reordering never changes the target");
  await f.dispatcher.dispatch(input("VideoGroupBlendMode", { value: 1 }));
  assert.equal(f.catalog().layer_groups.find(group => group.id === 41).blend_mode, "SoftLight");
  await f.dispatcher.dispatch(input("VideoGroupBlendMode", { value: -1 }));
  assert.equal(f.catalog().layer_groups.find(group => group.id === 41).blend_mode, "Normal");
  await f.dispatcher.dispatch(input("VideoGroupOpacity", { value: 2 }));
  assert.equal(f.catalog().layer_groups.find(group => group.id === 41).opacity, 1);
  assert.deepEqual(await f.dispatcher.dispatch(input("VideoGroupOpacity", { value: 1 })), { handled: false, reason: "unchanged" });
  assert.deepEqual(await f.dispatcher.dispatch(input("VideoGroupOpacity", { video_group_id: 99 })), { handled: false, reason: "missing-group" });
  assert.equal(f.catalog().layer_groups.find(group => group.id === 42).opacity, 1);
}
for (const [action, field] of [["VideoGroupSolo", "solo"], ["VideoGroupBypass", "bypassed"], ["VideoGroupEnabled", "enabled"]]) {
  const f = fixture(), gate = deferred();
  const initial = f.catalog().layer_groups[0][field];
  f.hold(gate.promise);
  const first = f.dispatcher.dispatch(input(action, { toggle: true }));
  await drain();
  const second = f.dispatcher.dispatch(input(action, { toggle: true }));
  gate.resolve();
  await Promise.all([first, second]);
  assert.deepEqual(f.calls, [[41, { [field]: !initial }], [41, { [field]: initial }]], action + " retains two rapid toggles");
  await f.dispatcher.dispatch(input(action, { active: false }));
  assert.equal(f.catalog().layer_groups[0][field], false, action + " absolute input can clear the flag");
}
{
  const f = fixture(), gate = deferred();
  f.hold(gate.promise);
  const first = f.dispatcher.dispatch(input("VideoGroupOpacity", { value: 0.1 }));
  await drain();
  const burst = Array.from({ length: 60 }, (_, index) => f.dispatcher.dispatch(input("VideoGroupOpacity", { value: (index + 20) / 100 })));
  gate.resolve();
  await Promise.all([first, ...burst]);
  assert.deepEqual(f.calls, [[41, { opacity: 0.1 }], [41, { opacity: 0.79 }]], "continuous input retains only its latest pending value");
}
{
  const f = fixture(), gate = deferred();
  f.hold(gate.promise);
  const first = f.dispatcher.dispatch(input("VideoGroupOpacity", { value: 0.3 }));
  await drain();
  const solo = f.dispatcher.dispatch(input("VideoGroupSolo"));
  const other = f.dispatcher.dispatch(input("VideoGroupBypass", { video_group_id: 42 }));
  await drain();
  assert.equal(f.calls.length, 1, "fields and groups share one catalog lane");
  gate.resolve();
  await Promise.all([first, solo, other]);
  assert.equal(f.maxConcurrency(), 1);
  assert.deepEqual(f.catalog().layer_groups.map(group => [group.opacity, group.solo, group.bypassed]), [[0.3, true, false], [1, false, true]]);
}
for (const invalidation of ["delete", "project", "source", "modal"]) {
  const f = fixture(), gate = deferred();
  let current = true;
  f.idle(gate.promise);
  const work = f.dispatcher.dispatch(input("VideoGroupSolo", { sourceIsCurrent: () => current }));
  await drain();
  if (invalidation === "delete") f.replaceCatalog({ ...f.catalog(), layer_groups: [group(42)] });
  if (invalidation === "project") f.replaceProject();
  if (invalidation === "source") current = false;
  if (invalidation === "modal") f.suspend(true);
  gate.resolve(true);
  await work;
  assert.deepEqual(f.calls, [], invalidation + " fences queued group input");
}
{
  const f = fixture();
  f.suspend(true);
  assert.deepEqual(await f.dispatcher.dispatch(input("VideoGroupSolo")), { handled: false, reason: "blocked" });
  assert.deepEqual(f.calls, []);
  f.suspend(false);
  await f.dispatcher.dispatch(input("VideoGroupSolo"));
  assert.equal(f.calls.length, 1, "input resumes after the modal/Learn mode closes");
}

// Use the actual controller, with only the native boundary mocked. A preflight
// can flush another catalog edit, delete a group, or replace its input source.
function controllerFixture() {
  let catalog = emptyCatalog(), current = true, pulseBusy = false;
  let authority = { project_epoch: 1, project_revision: 2, checkpoint_hash: "b".repeat(64) };
  const preflight = deferred(), calls = [], messages = [];
  const controller = createVideoRuntimeController({
    isIsfEventPulseBusy: () => pulseBusy,
    getCurrentProjectAuthority: () => authority,
    prepareMediaAssetOperationStart: async () => { await preflight.promise; return { authority }; },
    snapshot: () => ({ authored_video: catalog, video: catalog }),
    invoke: async (command, args) => {
      calls.push({ command, args });
      if (command === "apply_video_effect_catalog_authoritative") {
        catalog = args.request;
        return { mutation: { authority } };
      }
      if (command === "get_video_preview_diagnostics") return {};
      throw new Error("Unexpected command " + command);
    },
    authoritativeApplicationIsCurrent: () => true,
    isProjectAuthorityCurrent: () => true,
    refreshSnapshot: async () => {},
    setVideoPreviewDiagnostics: () => {},
    setMessage: message => messages.push(message),
    projectTransactionOwnerId: "t28-group-test",
  });
  return { controller, calls, messages, preflight, catalog: () => catalog, replaceCatalog: next => { catalog = next; },
    isCurrent: () => current, retire: () => { current = false; }, pulseBusy: value => { pulseBusy = value; },
    advance: () => { authority = { ...authority, project_revision: 3, checkpoint_hash: "c".repeat(64) }; } };
}
{
  const f = controllerFixture();
  const work = f.controller.patchVideoLayerGroup(41, { opacity: 0.4 }, f.isCurrent);
  await drain();
  f.replaceCatalog({ ...f.catalog(), effect_presets: [{ id: 99, label: "New preset" }],
    layer_groups: [group(42, { solo: true }), group(41, { layer_ids: [7, 8], blend_mode: "Screen" })] });
  f.advance();
  f.preflight.resolve();
  assert.notEqual(await work, null);
  const { command, args } = f.calls[0];
  assert.equal(command, "apply_video_effect_catalog_authoritative");
  assert.equal(args.expectedRevision, 3);
  assert.deepEqual(args.request.effect_presets, [{ id: 99, label: "New preset" }]);
  assert.deepEqual(args.request.layer_groups.map(group => [group.id, group.opacity, group.solo, group.blend_mode, group.layer_ids]),
    [[42, 1, true, "Normal", [42]], [41, 0.4, false, "Screen", [7, 8]]], "fresh post-preflight state preserves concurrent edits");
  assert.deepEqual(f.messages, []);
}
for (const invalidation of ["source", "delete", "FX pulse"]) {
  const f = controllerFixture();
  const work = f.controller.patchVideoLayerGroup(41, { solo: true }, f.isCurrent);
  await drain();
  if (invalidation === "source") f.retire();
  else if (invalidation === "FX pulse") f.pulseBusy(true);
  else f.replaceCatalog({ ...f.catalog(), layer_groups: [group(42)] });
  f.preflight.resolve();
  assert.equal(await work, null);
  assert.deepEqual(f.calls, [], invalidation + " during preflight sends no native mutation");
}
console.log("PASS: group mapping identities, MIDI/OSC/DMX/beat persistence, toggles, coalescing, catalog serialization and post-preflight safety");
