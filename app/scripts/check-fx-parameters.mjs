import assert from "node:assert/strict";
import { mkdtemp, readFile, writeFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";

const appRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const server = await createServer({ root: appRoot, configFile: false, logLevel: "error", appType: "custom",
  optimizeDeps: { noDiscovery: true }, server: { middlewareMode: true } });
let temporary;
try {
  const model = await server.ssrLoadModule("/src/fxParameterModel.ts");
  const { builtinFxEntries } = await server.ssrLoadModule("/src/fxCatalogModel.ts");
  const control = (name, kind, minimum, maximum, value, extras = {}) => ({ name, kind, minimum, maximum, value,
    default: [...value], values: [], labels: [], ...extras });
  const stage = { enabled: true, label: "Imported", source: '/*{"INPUTS":[{"NAME":"gain","LABEL":"Light gain","TYPE":"float","MIN":0,"MAX":1,"DEFAULT":0.4}]}*/\nvoid main() {}', categories: [], controls: [
    control("gain", "Float", [0,0,0,0], [1,1,1,1], [0.4,0,0,0]),
    control("steps", "Long", [2,0,0,0], [8,0,0,0], [4,0,0,0], { values: [2,4,8], labels: ["Low", "Medium", "High"] }),
    control("center", "Point2d", [-2,-3,0,0], [2,4,0,0], [0.5,0.5,0,0]),
    control("tint", "Color", [0,0,0,0], [1,1,1,1], [0.2,0.4,0.8,1]),
    control("enabled", "Bool", [0,0,0,0], [1,0,0,0], [1,0,0,0]),
    control("pulse", "Event", [0,0,0,0], [1,0,0,0], [0,0,0,0]),
  ] };
  assert.equal(model.fxInputMetadata(stage.source)[0].TYPE, "float");
  assert.equal(model.fxControlLabel(stage.controls[0], stage.source), "Light gain");
  assert.equal(model.fxControlLabel({ ...stage.controls[0], name: "edgeSoftness" }, "broken"), "Edge Softness");
  assert.deepEqual(model.fxInputMetadata("/*{bad}*/"), []);
  assert.equal(model.fxControlValueText(stage.controls[0], stage), "40 %");
  assert.equal(model.fxControlValueText(stage.controls[1], stage), "Medium");
  assert.deepEqual(model.boundedFxValue(stage.controls[2], [99,-99,0,0]), [2,-3,0,0]);
  assert.deepEqual(model.boundedFxValue(stage.controls[0], stage.controls[0].default), [0.4,0,0,0]);
  assert.equal(model.fxComponentCount(stage.controls[2]), 2);
  assert.equal(model.fxComponentCount(stage.controls[3]), 4);
  const rotate = builtinFxEntries.find((entry) => entry.builtinId === "rotate");
  assert.equal(model.fxControlValueText({ ...stage.controls[0], name: "angle", value: [Math.PI/2,0,0,0], maximum: [Math.PI,0,0,0], minimum: [-Math.PI,0,0,0] }, { ...stage, label: "Rotate", source: rotate.source.replace(/\r\n/g, "\n"), categories: ["Syndocal Built-in"] }), "90 °");
  assert.equal(model.fxControlValueText({ ...stage.controls[0], name: "angle", maximum: [10,0,0,0] }, stage), "0.4", "unknown shader angles must not invent degree units");
  const original = model.cloneFxStage(stage);
  for (const random of [() => 0, () => 0.5, () => 1, () => -10, () => 10, () => NaN]) {
    const randomized = model.randomizeFxStage(stage, random);
    for (const [index, next] of randomized.controls.entries()) {
      const before = stage.controls[index];
      if (next.kind === "Event") { assert.deepEqual(next.value, before.value); continue; }
      for (let component=0; component<model.fxComponentCount(next); component++) {
        assert.ok(Number.isFinite(next.value[component]));
        assert.ok(next.value[component] >= before.minimum[component] && next.value[component] <= before.maximum[component]);
      }
      if (next.kind === "Long") assert.ok(next.values.includes(next.value[0]));
      if (next.kind === "Bool") assert.ok([0,1].includes(next.value[0]));
    }
    const full = { ...stage, stack: [{ ...stage, label: "Second" }] };
    const changed = model.replaceFxStage(full, 1, randomized);
    assert.deepEqual(changed.controls, stage.controls, "randomize preserves other stages");
    assert.deepEqual(model.replaceFxStage(changed, 1, original).stack[0], original, "undo restores exact control image");
  }
  assert.deepEqual(stage, original, "randomization never mutates its undo image");
  const incompatible = { ...stage, source: "another shader" };
  const full = { ...stage, stack: [] };
  assert.equal(model.replaceFxStage(full, 0, incompatible), full, "stale source identity fails closed");
  const catalog = { effect_chains: [], effect_presets: [], layer_groups: [{ id: 4, label: "Preserved", composition_id: 1, layer_ids: [2] }], transition_buses: [] };
  const saved = model.saveFxStagePreset(catalog, stage, "  Evening tint  ");
  assert.equal(saved.effect_presets[0].label, "Evening tint");
  assert.equal(saved.effect_presets[0].id, 0, "native allocator owns new IDs");
  assert.equal(model.saveFxStagePreset(catalog, stage, "  "), catalog);
  assert.deepEqual(saved.layer_groups, catalog.layer_groups);
  const project = JSON.parse(await readFile(resolve(appRoot, "../target/qa/vj-perform-ui/vj-perform-native.sdc"), "utf8"));
  project.snapshot.video.effect_presets = saved.effect_presets;
  if (project.snapshot.authored_video) project.snapshot.authored_video.effect_presets = saved.effect_presets;
  temporary = await mkdtemp(join(tmpdir(), "syndocal-fx-parameters-"));
  const path = join(temporary, "preset-project.sdc");
  await writeFile(path, JSON.stringify(project), "utf8");
  const reopened = JSON.parse(await readFile(path, "utf8"));
  const recalled = model.matchingFxStagePresets({ ...catalog, effect_presets: reopened.snapshot.video.effect_presets }, stage);
  assert.equal(recalled.length, 1);
  assert.deepEqual(recalled[0].payload.stages[0].effect.effect.controls, stage.controls);
  assert.equal(model.matchingFxStagePresets(saved, incompatible).length, 0);
  const builtin = { ...stage, label: "Monochrome", source: builtinFxEntries.find((entry) => entry.builtinId === "monochrome").source, categories: ["Syndocal Built-in"], controls: [{ ...stage.controls[0], name: "amount" }] };
  assert.equal(model.builtinFxStagePresets(builtin).length, 2);
  assert.equal(model.builtinFxStagePresets({ ...builtin, source: stage.source }).length, 0, "imported categories cannot spoof built-in recipes/units");
  assert.equal(model.builtinFxStagePresets(stage).length, 0);
  assert.equal(model.fxControlValueText(stage.controls[3], stage), "#3366CC · 100 %", "colour reads as hex plus opacity");
  const rotation = { ...stage, label: "Rotate", source: rotate.source.replace(/\r\n/g, "\n"), categories: ["Syndocal Built-in"] };
  const angle = { ...stage.controls[0], name: "angle", value: [Math.PI / 6, 0, 0, 0] };
  assert.equal(model.fxControlValueText(angle, rotation), "30 °", "degrees are rounded to one decimal");
  assert.ok(Math.abs(model.fxDisplayScale(angle, rotation) - 180 / Math.PI) < 1e-9, "typed values for known angles are degrees");
  assert.equal(model.fxDisplayScale(angle, stage), 1, "unknown shaders never invent a unit scale");
  // ---- Tempo sync: beat and bar units derived from the global clock ----
  const strobeEntry = builtinFxEntries.find((entry) => entry.builtinId === "strobe");
  const glitchEntry = builtinFxEntries.find((entry) => entry.builtinId === "glitch-shift");
  const strobe = { ...stage, label: "Strobe", source: strobeEntry.source.replace(/\r\n/g, "\n"), categories: ["Syndocal Built-in"], controls: [
    control("rate", "Float", [0.5,0,0,0], [30,0,0,0], [8,0,0,0]), control("duty", "Float", [0,0,0,0], [1,0,0,0], [0.5,0,0,0]) ] };
  const [rate, duty] = strobe.controls;
  assert.equal(model.fxIsTimeControl(rate, strobe), true, "Strobe rate is a time control");
  assert.equal(model.fxIsTimeControl(duty, strobe), false, "Strobe duty is a ratio, not a time control");
  const glitch = { ...stage, label: "Glitch Shift", source: glitchEntry.source.replace(/\r\n/g, "\n"), categories: ["Syndocal Built-in"], controls: [control("speed", "Float", [0,0,0,0], [30,0,0,0], [8,0,0,0]), control("amount", "Float", [0,0,0,0], [1,0,0,0], [0.5,0,0,0])] };
  assert.equal(model.fxIsTimeControl(glitch.controls[0], glitch), true);
  assert.equal(model.fxIsTimeControl(glitch.controls[1], glitch), false);
  assert.equal(model.fxIsTimeControl({ ...stage.controls[0], name: "speed" }, strobe), false, "built-ins only expose the controls with known cycles-per-second maths");
  assert.equal(model.fxIsTimeControl({ ...stage.controls[0], name: "lfoSpeed" }, stage), true, "imported rate/speed/LFO floats qualify");
  assert.equal(model.fxIsTimeControl({ ...stage.controls[0], name: "wobble_freq" }, stage), true);
  assert.equal(model.fxIsTimeControl({ ...stage.controls[0], name: "gain" }, stage), false);
  assert.equal(model.fxIsTimeControl({ ...stage.controls[1], name: "speed" }, stage), false, "only Float controls qualify");
  assert.equal(model.fxIsTimeControl({ ...stage.controls[0], name: "speed", minimum: [-1,0,0,0] }, stage), false, "signed controls are not frequencies");
  // Hz = bpm / 60 / beats-per-cycle, with 4 beats per bar.
  assert.deepEqual(model.FX_TEMPO_SYNC_COUNTS, [0.25, 0.5, 1, 2, 4]);
  assert.equal(model.fxTempoSyncHz(120, { unit: "beats", count: 1 }), 2);
  assert.equal(model.fxTempoSyncHz(120, { unit: "beats", count: 0.25 }), 8);
  assert.equal(model.fxTempoSyncHz(120, { unit: "beats", count: 4 }), 0.5);
  assert.equal(model.fxTempoSyncHz(120, { unit: "bars", count: 1 }), 0.5, "one bar is four beats");
  assert.equal(model.fxTempoSyncHz(90, { unit: "bars", count: 0.5 }), 0.75);
  for (const bad of [0, -5, NaN, Infinity, undefined]) assert.ok(Number.isNaN(model.fxTempoSyncHz(bad, { unit: "beats", count: 1 })), `unusable tempo ${bad}`);
  const synced = model.withFxTempoSync(rate, { unit: "beats", count: 0.5 }, 120);
  assert.deepEqual(synced.tempo_sync, { unit: "beats", count: 0.5 });
  assert.equal(synced.value[0], 4, "choosing a sync moves the value to the clock-derived target");
  assert.equal(rate.tempo_sync, undefined, "the original control is never mutated");
  assert.deepEqual(rate.value, [8,0,0,0]);
  assert.equal(model.fxTempoSyncState(synced, 120).inSync, true);
  assert.equal(model.fxTempoSyncState(synced, 140).inSync, false, "a tempo change leaves the synced value stale until re-sync");
  assert.equal(model.fxTempoSyncState(rate, 120), null, "unsynced controls have no sync state");
  assert.equal(model.fxTempoSyncState(synced, undefined), null, "no usable tempo, no state");
  assert.equal(model.fxTempoSyncState(model.withFxTempoSync(rate, { unit: "bars", count: 4 }, 120), 120).clamped, true, "0.125 Hz is below the 0.5 Hz minimum");
  assert.equal(model.withFxTempoSync(rate, { unit: "bars", count: 4 }, 120).value[0], 0.5, "the range limit is used when the tempo asks for less");
  assert.equal(model.withFxTempoSync(rate, { unit: "beats", count: 0.25 }, 600).value[0], 30, "and the upper limit when it asks for more");
  const unsynced = model.withFxTempoSync(synced, null, 120);
  assert.equal("tempo_sync" in unsynced, false, "Hz removes the sync");
  assert.equal(unsynced.value[0], 4, "and keeps the last value as a plain number");
  assert.equal(model.withFxTempoSync(rate, { unit: "beats", count: 1 }, undefined).value[0], 8, "without a tempo the value is left alone");
  const syncedStage = { ...strobe, controls: [synced, duty] };
  const resynced = model.resyncFxStage(syncedStage, 150);
  assert.equal(resynced.controls[0].value[0], 5, "re-sync follows the new tempo (150 BPM, 1/2 beat)");
  assert.deepEqual(resynced.controls[1], duty, "ordinary controls are untouched");
  assert.equal(syncedStage.controls[0].value[0], 4, "re-sync never mutates the stage it reads");
  // Randomize and the clone/replace round trip keep the choice.
  for (const random of [() => 0, () => 0.5, () => 1]) {
    const randomized = model.randomizeFxStage(syncedStage, random);
    assert.deepEqual(randomized.controls[0].tempo_sync, { unit: "beats", count: 0.5 });
    assert.equal(randomized.controls[0].value[0], 4, "Randomize leaves a synced control on the clock");
  }
  assert.deepEqual(model.cloneFxStage(syncedStage).controls[0].tempo_sync, { unit: "beats", count: 0.5 });
  assert.deepEqual(model.replaceFxStage({ ...syncedStage, stack: [] }, 0, syncedStage).controls[0].tempo_sync, { unit: "beats", count: 0.5 });
  assert.equal(model.builtinFxStagePresets(syncedStage).every((preset) => preset.stage.controls[0].tempo_sync === undefined), true, "a recipe rate is a raw number and replaces the sync");
  const projectShaped = JSON.parse(JSON.stringify({ ...syncedStage, stack: [] }));
  assert.deepEqual(projectShaped.controls[0].tempo_sync, { unit: "beats", count: 0.5 }, "the choice survives the project JSON round trip");
  const unitLabels = [[{ unit: "beats", count: 0.25 }, "1/4 beat"], [{ unit: "beats", count: 1 }, "1 beat"], [{ unit: "beats", count: 2 }, "2 beats"], [{ unit: "bars", count: 0.5 }, "1/2 bar"], [{ unit: "bars", count: 4 }, "4 bars"]];
  for (const [sync, text] of unitLabels) assert.equal(model.fxTempoSyncLabel(sync), text);
  assert.equal(model.fxTempoSyncLabel({ unit: "beats", count: 0.25 }, "ja"), "1/4拍");
  assert.equal(model.fxTempoSyncLabel({ unit: "bars", count: 2 }, "ja"), "2小節");
  // The audition draft keeps and clears the choice like the live chain does.
  const audition = await server.ssrLoadModule("/src/fxAuditionModel.ts");
  const chain = { ...syncedStage, stack: [{ ...stage, label: "B" }, { ...stage, label: "C" }] };
  assert.deepEqual(audition.fxChainStages(chain)[0].controls[0].tempo_sync, { unit: "beats", count: 0.5 });
  assert.equal("tempo_sync" in audition.fxChainStages(audition.auditionResetStage(chain, 0))[0].controls[0], false, "Reset ends the sync");
  // Drag-and-drop reorder inserts the dragged stage (a swap would scramble a longer move).
  const order = (value) => audition.fxChainStages(value).map((entry) => entry.label);
  const four = { ...stage, label: "A", stack: ["B", "C", "D"].map((label) => ({ ...stage, label })) };
  assert.deepEqual(order(audition.moveFxStageTo(four, 0, 3)), ["B", "C", "D", "A"]);
  assert.deepEqual(order(audition.moveFxStageTo(four, 3, 0)), ["D", "A", "B", "C"]);
  assert.deepEqual(order(audition.moveFxStageTo(four, 1, 2)), ["A", "C", "B", "D"]);
  assert.deepEqual(order(audition.moveFxStageTo(four, 2, 0)), ["C", "A", "B", "D"]);
  assert.equal(audition.moveFxStageTo(four, 1, 1), four, "no-op returns the same chain");
  assert.equal(audition.moveFxStageTo(four, 1, 9), four, "out of range is refused");
  assert.equal(audition.moveFxStageTo(four, -1, 2), four);
  assert.deepEqual(audition.fxChainStages(audition.moveFxStageTo(chain, 0, 2))[2].controls[0].tempo_sync, { unit: "beats", count: 0.5 }, "reorder carries the control state with the stage");
  assert.deepEqual(audition.moveFxStageTo(chain, 0, 2).stack.length, 2, "the root/stack split is rebuilt");
  console.log("PASS FX parameters: ISF labels/type presentation; validated ranges/defaults; colour/point arity; declared choice/random bounds; exact undo; stale identity rejection; tempo-synced beat/bar units (clock maths, range clamp, re-sync, project round trip); insert-style stage reorder; presets saved/reopened from project-shaped UTF-8 disk file (native save/load verified separately).");
} finally { await server.close(); if (temporary) await rm(temporary, { recursive: true, force: true }); }
