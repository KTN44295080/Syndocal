import assert from "node:assert/strict";
import fs from "node:fs/promises";
import ts from "typescript";

const moduleUrls = new Map();
async function loadTsModuleUrl(url) {
  if (moduleUrls.has(url.href)) return moduleUrls.get(url.href);
  let js = ts.transpileModule(await fs.readFile(url, "utf8"), {
    compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext },
  }).outputText;
  const ast = ts.createSourceFile(url.href, js, ts.ScriptTarget.ES2022, true, ts.ScriptKind.JS);
  const edits = [];
  for (const node of ast.statements) {
    if ((!ts.isImportDeclaration(node) && !ts.isExportDeclaration(node))
      || !node.moduleSpecifier || !ts.isStringLiteral(node.moduleSpecifier)) continue;
    const name = node.moduleSpecifier.text;
    assert.ok(name.startsWith("."), "focused runtime modules have no external dependencies");
    const child = await loadTsModuleUrl(new URL(/\.[cm]?tsx?$/.test(name) ? name : name + ".ts", url));
    edits.push({ start: node.moduleSpecifier.getStart(ast), end: node.moduleSpecifier.getEnd(), text: JSON.stringify(child) });
  }
  for (const edit of edits.sort((a, b) => b.start - a.start)) js = js.slice(0, edit.start) + edit.text + js.slice(edit.end);
  const result = "data:text/javascript;base64," + Buffer.from(js).toString("base64");
  moduleUrls.set(url.href, result);
  return result;
}
const audio = await import(await loadTsModuleUrl(new URL("../src/audioControlMapping.ts", import.meta.url)));
const runtime = await import(await loadTsModuleUrl(new URL("../src/audioControlRuntime.ts", import.meta.url)));
const { createPerformControlDispatcher } = await import(await loadTsModuleUrl(new URL("../src/performControlDispatcher.ts", import.meta.url)));
const learn = await import(await loadTsModuleUrl(new URL("../src/controlMappingLearn.ts", import.meta.url)));
let checks = 0;
async function test(name, run) { await run(); checks++; console.log("PASS " + name); }
const near = (actual, expected) => assert.ok(Math.abs(actual - expected) < 1e-9, actual + " != " + expected);
const settings = { source: "Level", gain: 1, attack_ms: 0, release_ms: 0, threshold: 0.5, mode: "Continuous", low: 0, high: 1 };
function mapping(action = "VideoParam", id = "audio-test", overrides = {}) {
  const base = audio.audioMappingsFromTargets([{
    action, video_param: action === "VideoParam" ? "Opacity" : null,
    layer_id: action === "VideoTake" ? null : 7,
    clip_slot_id: action === "VideoClipSlotTrigger" ? 91 : null, label: "Test",
  }], { ...settings, mode: action === "VideoTake" || action === "VideoClipSlotTrigger" ? "Threshold" : "Continuous" }, () => id)[0];
  return { ...base, ...overrides, audio: { ...base.audio, ...overrides.audio } };
}
function frame(sequence, level, at = 0, overrides = {}) {
  return {
    generation: 1, feature_sequence: sequence, running: true, stale: false, safety_clear_pending: false,
    bass: level, mid: level, high: level, rms: level, peak: level, spectral_flux: level,
    spectral_centroid: level, spectral_density_fast: level, spectral_density_slow: level,
    kick_strength: level, snare_strength: level, onset: false, onset_strength: level, observedAt: at, ...overrides,
  };
}
const sample = (e, mappings, seq, level, at, extra = {}) => e.evaluate(mappings, frame(seq, level, at, extra), at);
const empty = result => { assert.deepEqual(result.batch.values, []); assert.deepEqual(result.triggers, []); };
const deferred = () => { let resolve; const promise = new Promise(done => { resolve = done; }); return { promise, resolve }; };

await test("gain clamps before output range; inverted ranges are preserved", () => {
  near(audio.audioMappingValue(0.4, 2, 0.2, 0.7), 0.6);
  near(audio.audioMappingValue(1, 2, 0.2, 0.7), 0.7);
  near(audio.audioMappingValue(0.25, 1, 1, 0), 0.75);
  assert.equal(audio.audioMappingValue(0, 1, 1, 0), 1);
});
await test("attack/release use independent exponential time constants", () => {
  near(audio.smoothAudioMapping(0, 1, 100, 100, 1000), 1 - Math.exp(-1));
  near(audio.smoothAudioMapping(1, 0, 100, 100, 1000), Math.exp(-0.1));
  assert.equal(audio.smoothAudioMapping(0, 1, 10, 0, 100), 1);
  assert.equal(audio.smoothAudioMapping(1, 0, 10, 100, 0), 0);
});
await test("all twelve feature selectors resolve independently and sanitize nonfinite input", () => {
  const keys = ["bass", "mid", "high", "rms", "peak", "spectral_flux", "spectral_centroid",
    "spectral_density_fast", "spectral_density_slow", "kick_strength", "snare_strength", "onset_strength"];
  const levels = frame(1, 0);
  keys.forEach((key, i) => { levels[key] = (i + 1) / 20; });
  audio.audioSources.forEach((source, i) => near(audio.audioMappingSignal(source.value, levels), (i + 1) / 20));
  assert.equal(audio.audioMappingSignal("Level", { ...levels, rms: NaN }), 0);
});
await test("persistence keeps stable ISF/group ids; Audio-only and Event controls cannot become hardware/Audio bindings", () => {
  const targets = [
    { action: "VideoIsfParam", layer_id: 7, isf_stage_id: 31, attribute: "amount", label: "Amount" },
    { action: "VideoParam", video_param: "Opacity", group_id: "57", audio_only: true, label: "Group opacity" },
  ];
  let id = 0;
  const saved = audio.audioMappingsFromTargets(targets, { ...settings, low: 1, high: 0 }, () => "stable-" + ++id);
  assert.deepEqual(JSON.parse(JSON.stringify(saved)), saved);
  assert.equal(saved[0].isf_stage_id, 31); assert.equal(saved[0].attribute, "amount");
  assert.equal(saved[1].group_id, "57"); assert.equal(saved[1].layer_id, null);
  assert.deepEqual(learn.midiMappingsFromLearnedControl([targets[1]], { channel: 1, message: "ControlChange", number: 7 }), []);
  assert.deepEqual(audio.audioMappingsFromTargets([{ action: "VideoIsfParam", audio_disabled: true, label: "Event" }], settings), []);
  const hardware = { ...mapping(), message: "ControlChange" }; delete hardware.audio;
  empty(sample(audio.createAudioMappingEvaluator(), [hardware], 1, 1, 0));
});
await test("continuous heartbeat applies smoothing and never mutates persisted mappings", () => {
  const e = audio.createAudioMappingEvaluator();
  const m = mapping("VideoParam", "audio-test", { low: 0.2, high: 0.8, audio: { attack_ms: 100, release_ms: 1000 } });
  const before = JSON.stringify(m);
  sample(e, [m], 1, 0, 0);
  near(sample(e, [m], 2, 1, 100).batch.values[0].value, 0.2 + 0.6 * (1 - Math.exp(-1)));
  assert.equal(e.evaluate([m], frame(2, 1, 100), 150).batch.values.length, 1);
  for (let seq = 3; seq < 503; seq++) sample(e, [m], seq, (seq % 10) / 10, seq * 100);
  assert.equal(JSON.stringify(m), before);
});
await test("mapping count is bounded and duplicate ids are emitted once", () => {
  const e = audio.createAudioMappingEvaluator();
  const many = Array.from({ length: 200 }, (_, i) => mapping("VideoParam", "audio-" + i));
  assert.equal(sample(e, many, 1, 1, 0).batch.values.length, audio.maxAudioMappings);
  e.reset(); assert.equal(sample(e, [many[0], many[0]], 1, 1, 0).batch.values.length, 1);
});
await test("threshold hysteresis fires one edge and rearms at silence including zero/low thresholds", () => {
  for (const threshold of [0, 0.02, 0.5]) {
    const e = audio.createAudioMappingEvaluator(); const m = mapping("VideoTake", "edge", { audio: { threshold } });
    assert.equal(sample(e, [m], 1, 0, 0).triggers.length, 0);
    assert.equal(sample(e, [m], 2, 0.8, 50).triggers.length, 1);
    assert.equal(sample(e, [m], 3, 0.8, 100).triggers.length, 0);
    assert.equal(sample(e, [m], 4, 0, 150).triggers.length, 0);
    assert.equal(sample(e, [m], 5, 0.8, 200).triggers.length, 1);
  }
});
await test("duplicate publications cannot consume an edge crossed by smoothing", () => {
  const e = audio.createAudioMappingEvaluator(); const m = mapping("VideoTake", "smooth", { audio: { attack_ms: 100 } });
  sample(e, [m], 1, 0, 0);
  assert.equal(sample(e, [m], 2, 1, 50).triggers.length, 0);
  assert.equal(e.evaluate([m], frame(2, 1, 50), 100).triggers.length, 0);
  assert.equal(sample(e, [m], 3, 1, 150).triggers.length, 1);
  assert.equal(sample(e, [m], 4, 1, 200).triggers.length, 0);
});
await test("source/configuration changes establish edge baselines without firing", () => {
  const e = audio.createAudioMappingEvaluator(); const m = mapping("VideoTake");
  assert.equal(sample(e, [m], 1, 1, 0).triggers.length, 0);
  sample(e, [m], 2, 0, 50);
  assert.equal(sample(e, [m], 1, 1, 100, { generation: 2 }).triggers.length, 0);
  const changed = { ...m, audio: { ...m.audio, threshold: 0.25 } };
  assert.equal(sample(e, [changed], 2, 1, 150, { generation: 2 }).triggers.length, 0);
});
await test("old sequences and source generations cannot lower the high-water mark", () => {
  const e = audio.createAudioMappingEvaluator(); const m = mapping();
  sample(e, [m], 10, 0.5, 0, { generation: 11 });
  for (let at = 50; at <= 150; at += 50) empty(sample(e, [m], 9, 1, at, { generation: 11 }));
  empty(sample(e, [m], 100, 1, 200, { generation: 10 }));
  empty(sample(e, [m], 101, 1, 250, { generation: 10 }));
  assert.equal(sample(e, [m], 11, 0.25, 300, { generation: 11 }).batch.values[0].value, 0.25);
});
await test("onset pulses require fresh sequences; clip queue precedes Take", () => {
  const e = audio.createAudioMappingEvaluator();
  const maps = [mapping("VideoTake", "take", { audio: { mode: "Onset" } }),
    mapping("VideoClipSlotTrigger", "clip", { audio: { mode: "Onset" } })];
  assert.equal(sample(e, maps, 1, 1, 0, { onset: true }).triggers.length, 0);
  const fired = sample(e, maps, 2, 1, 50, { onset: true }).triggers;
  assert.deepEqual(fired.map(input => input.action), ["VideoClipSlotTrigger", "VideoTake"]);
  assert.equal(fired[0].source, "audio"); assert.equal(fired[0].clip_slot_id, 91);
  assert.equal(e.evaluate(maps, frame(2, 1, 50, { onset: true }), 75).triggers.length, 0);
  assert.equal(sample(e, maps, 3, 1, 100, { onset: true }).triggers.length, 2);
});
await test("stale/unsafe/invalid inputs clear at the exact 250 ms boundary", () => {
  const e = audio.createAudioMappingEvaluator(); const m = mapping();
  assert.equal(e.evaluate([m], frame(1, 1, 100), 349).batch.values.length, 1);
  empty(e.evaluate([m], frame(1, 1, 100), 350));
  for (const extra of [{ running: false }, { stale: true }, { safety_clear_pending: true },
    { generation: 0 }, { generation: NaN }, { feature_sequence: 1.5 }, { observedAt: NaN }, { observedAt: 101 }]) {
    empty(sample(audio.createAudioMappingEvaluator(), [m], 1, 1, 100, extra));
  }
  empty(e.evaluate([m], null, 400));
});
await test("invalid settings and removed mappings cannot reach the runtime", () => {
  for (const change of [{ gain: NaN }, { gain: 16.1 }, { gain: -1 }, { attack_ms: 10001 },
    { release_ms: -1 }, { threshold: 1.1 }, { id: "" }, { id: "bad id" }, { source: "Unknown" }]) {
    empty(sample(audio.createAudioMappingEvaluator(), [mapping("VideoParam", "bad", { audio: change })], 1, 1, 0));
  }
  empty(sample(audio.createAudioMappingEvaluator(), [mapping("VideoParam", "bad", { high: 2 })], 1, 1, 0));
  const e = audio.createAudioMappingEvaluator(); sample(e, [mapping()], 1, 1, 0);
  empty(sample(e, [], 2, 1, 50));
});
await test("poll repetition cannot refresh or revive cleared observations", () => {
  let now = 0; const source = runtime.createAudioMappingSource(() => now);
  source.accept(frame(10, 0.4)); assert.equal(source.current().observedAt, 0);
  now = 200; source.accept(frame(10, 0.9)); assert.equal(source.current().observedAt, 0);
  source.clear(); source.accept(frame(10, 0.4)); assert.equal(source.current(), null);
  now = 250; source.accept(frame(11, 0.7)); assert.equal(source.current().observedAt, 250);
  source.accept(frame(100, 1, 0, { generation: 0 })); assert.equal(source.current(), null);
  source.accept(frame(1, 0.2, 0, { generation: 2 })); assert.equal(source.current().generation, 2);
  source.accept(frame(500, 1, 0, { generation: 1 })); assert.equal(source.current().generation, 2);
});
await test("runtime cadence and trigger backlog are bounded; suspension/project change revoke queued leases", async () => {
  let now = 0; let epoch = 1; let suspended = false; let maps = [mapping(), mapping("VideoTake", "take")];
  const source = runtime.createAudioMappingSource(() => now); const batches = []; const inputs = []; const gate = deferred();
  const driver = runtime.createAudioControlRuntime({
    mappings: () => maps, observation: source.current, projectEpoch: () => epoch, suspended: () => suspended, now: () => now,
    dispatch: input => { inputs.push(input); return gate.promise; },
    dispatchRuntime: async (batch, current) => { batches.push({ batch, current }); },
  });
  const tick = (at, sequence, value) => { now = at; source.accept(frame(sequence, value)); driver.advance(); };
  tick(0, 1, 0); tick(10, 2, 0); tick(49, 3, 0); assert.equal(batches.length, 1);
  tick(50, 4, 1); assert.equal(inputs.length, 1); const old = inputs[0].sourceIsCurrent;
  assert.equal(old(), true);
  suspended = true; tick(100, 5, 1); assert.equal(old(), false);
  suspended = false; tick(150, 6, 1); assert.equal(old(), false);
  for (let at = 200; at <= 500; at += 50) tick(at, at, (at / 50) % 2);
  assert.equal(inputs.length, 1, "a blocked trigger owns the pulse; no backlog");
  epoch = 2; tick(550, 550, 1); assert.equal(batches.at(-2).current(), false);
  maps = []; tick(600, 600, 1); assert.deepEqual(batches.at(-1).batch.values, []);
  driver.dispose(); gate.resolve(); await gate.promise;
});

function dispatcherDeps(overrides = {}) {
  const forbidden = () => { throw new Error("Audio called an authored parameter setter"); };
  return {
    layers: () => [{ id: 7, clip_slots: [{ id: 91 }] }], effectCatalog: () => undefined,
    projectEpoch: () => 1, clipRuntime: () => null, takeEnabled: () => false,
    selectLayer: () => {}, selectSlot: () => {}, queueSlot: async () => true, take: async () => {},
    stopLayer: async () => {}, setBlendMode: forbidden, transitionKind: () => "Cut",
    transitionDurationMilliunits: () => 1000, transitionLocked: () => false,
    setTransitionKind: forbidden, setTransitionDurationMilliunits: forbidden, fxIdle: async () => true,
    setIsfStageEnabled: forbidden, setIsfControl: forbidden, triggerIsfEvent: forbidden, ...overrides,
  };
}
await test("shared dispatcher keeps one in-flight Audio batch and only the latest pending value", async () => {
  const gate = deferred(); const writes = [];
  const dispatcher = createPerformControlDispatcher(dispatcherDeps({
    setAudioRuntime: async batch => { writes.push(batch.values[0].value); if (writes.length === 1) await gate.promise; },
  }));
  const batch = value => ({ audioGeneration: 1, featureSequence: 1, values: [{ mappingId: "test", value }] });
  const first = dispatcher.dispatchAudioRuntime(batch(0.1), () => true);
  await dispatcher.dispatchAudioRuntime(batch(0.2), () => true);
  await dispatcher.dispatchAudioRuntime(batch(0.3), () => true);
  assert.deepEqual(writes, [0.1]);
  gate.resolve(); await first; assert.deepEqual(writes, [0.1, 0.3]);
});
await test("queued Audio runtime batches recheck project and source lease before writing", async () => {
  let epoch = 1; const gate = deferred(); const writes = [];
  const dispatcher = createPerformControlDispatcher(dispatcherDeps({
    projectEpoch: () => epoch,
    setAudioRuntime: async batch => { writes.push(batch.values.length); if (writes.length === 1) await gate.promise; },
  }));
  const batch = { audioGeneration: 1, featureSequence: 1, values: [{ mappingId: "test", value: 0.2 }] };
  const first = dispatcher.dispatchAudioRuntime(batch, () => true);
  await dispatcher.dispatchAudioRuntime(batch, () => false);
  epoch = 2; gate.resolve(); await first;
  assert.deepEqual(writes, [1]);
});
await test("Audio Queue/Take share the ticketed ordered lane; automatic empty Take stays quiet", async () => {
  let current = true; const calls = []; const gate = deferred();
  const dispatcher = createPerformControlDispatcher(dispatcherDeps({
    queueSlot: async () => { calls.push("queue"); await gate.promise; return true; },
    take: async () => { calls.push("take"); }, report: message => calls.push(message),
  }));
  const clip = { action: "VideoClipSlotTrigger", layer_id: 7, clip_slot_id: 91, position: 1, value: 1, active: true, source: "audio", sourceIsCurrent: () => current };
  const take = { action: "VideoTake", position: 1, value: 1, active: true, source: "audio", sourceIsCurrent: () => current };
  const queued = dispatcher.dispatch(clip); const taken = dispatcher.dispatch(take);
  await Promise.resolve(); assert.deepEqual(calls, ["queue"]);
  gate.resolve(); await Promise.all([queued, taken]); assert.deepEqual(calls, ["queue", "take"]);
  await dispatcher.dispatch(take); assert.deepEqual(calls, ["queue", "take"]);
  current = false; await dispatcher.dispatch(clip); assert.deepEqual(calls, ["queue", "take"]);
});
console.log("Audio control mapping/runtime: " + checks + " checks passed.");
