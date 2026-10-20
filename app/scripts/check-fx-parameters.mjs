import assert from "node:assert/strict";
import { mkdir, mkdtemp, readFile, writeFile, rm } from "node:fs/promises";
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
  // Keep this model/disk round trip runnable in a clean worktree before the native gate.
  const projectPath = process.env.SYNDOCAL_FX_PARAMETER_PROJECT ?? resolve(appRoot, "../samples/phase1-mini-show.sdc");
  const project = JSON.parse(await readFile(projectPath, "utf8"));
  assert.ok(project.snapshot?.video, "the project fixture contains a video snapshot");
  project.snapshot.video.effect_presets = saved.effect_presets;
  if (project.snapshot.authored_video) project.snapshot.authored_video.effect_presets = saved.effect_presets;
  const temporaryRoot = resolve(appRoot, "../target/qa/fx-parameters");
  await mkdir(temporaryRoot, { recursive: true });
  temporary = await mkdtemp(join(temporaryRoot, "preset-"));
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

  const imageModel = await server.ssrLoadModule("/src/fxImageInputModel.ts");
  const imageStage = { ...stage, source_path: "C:/pack/blend.fs", source: '/*{"INPUTS":[{"NAME":"overlay","LABEL":"Overlay picture","TYPE":"image"},{"NAME":"inputImage","TYPE":"image"},{"NAME":"mask","TYPE":"image"}]}*/void main(){}', controls: [] };
  assert.deepEqual(imageModel.fxAdditionalImageInputs(imageStage.source), [{ name: "overlay", label: "Overlay picture" }, { name: "mask", label: "mask" }], "inputImage remains primary even when not first");
  assert.deepEqual(imageModel.fxAdditionalImageInputs('/*{"INPUTS":[{"NAME":"from","TYPE":"image"},{"NAME":"to","TYPE":"image"}]}*/'), [{ name: "to", label: "to" }], "without inputImage the first image is primary");
  assert.deepEqual(imageModel.fxImageInputs('/*{"INPUTS":[]}*/'), [], "zero-image generators expose no image bindings");
  assert.deepEqual(imageModel.fxImageSource(imageStage, "overlay"), { kind: "none" }, "old stages default additional inputs to transparent");
  const layerBound = imageModel.withFxImageSource(imageStage, "overlay", { kind: "layer", layer_id: 42 });
  const stillBound = imageModel.withFxImageSource(layerBound, "mask", { kind: "imported", path: "textures/mask.png" });
  assert.deepEqual(imageModel.fxImageSource(stillBound, "overlay"), { kind: "layer", layer_id: 42 }, "editing one binding preserves others");
  assert.equal("image_bindings" in imageStage, false, "binding edits preserve the original undo image");
  const clonedImages = model.cloneFxStage(stillBound);
  clonedImages.image_bindings[0].source.layer_id = 99;
  assert.equal(stillBound.image_bindings[0].source.layer_id, 42, "cloned binding source objects are independent");
  assert.deepEqual(imageModel.fxImageSource(imageModel.withFxImageSource(stillBound, "overlay", { kind: "none" }), "overlay"), { kind: "none" });
  for (const invalidPath of ["../mask.png", "C:/mask.png", "/mask.png", "https://host/mask.png", "textures/../mask.png", "mask.gif", ""]) assert.ok(imageModel.fxImportedImagePathError(invalidPath, imageStage.source_path), invalidPath);
  assert.ok(imageModel.fxImportedImagePathError("mask.png", null), "imported stills need the shader directory");
  assert.equal(imageModel.fxImportedImagePathError("textures\\mask.jpg", imageStage.source_path), null);
  assert.throws(() => imageModel.withFxImageSource(imageStage, "inputImage", { kind: "none" }), /additional image input/, "the parameter editor leaves the primary host image alone");
  assert.throws(() => imageModel.withFxImageSource(imageStage, "missing", { kind: "none" }), /declared additional/);
  const imageSaved = model.saveFxStagePreset(catalog, stillBound, "Image blend");
  const imageReopened = JSON.parse(JSON.stringify(imageSaved));
  assert.deepEqual(imageReopened.effect_presets[0].payload.stages[0].effect.effect.image_bindings, stillBound.image_bindings, "project preset JSON round-trips stable IDs and still paths");
  const edge = (to) => ({ ...imageStage, image_bindings: [{ name: "overlay", source: { kind: "layer", layer_id: to } }] });
  const dependencyLayers = [{ id: 7, label: "Current", isf_effect: null }, { id: 42, label: "Source", isf_effect: edge(81) }, { id: 81, label: "Nested", isf_effect: edge(7) }, { id: 99, label: "Free", isf_effect: null }];
  assert.equal(imageModel.fxImageLayerWouldCycle(7, 7, dependencyLayers), true, "self reference rejected");
  assert.equal(imageModel.fxImageLayerWouldCycle(7, 42, dependencyLayers), true, "indirect cycle rejected");
  assert.equal(imageModel.fxImageLayerWouldCycle(7, 99, dependencyLayers), false, "independent layer accepted");
  assert.equal(imageModel.fxImageLayerWouldCycle(7, 42, [...dependencyLayers].reverse()), true, "reorder cannot alter stable-ID dependencies");
  const clipEdges = { ...catalog, effect_chains: [{ id: 10, scope: { scope: "clip", layer_id: 99, slot_id: 12 }, bypassed: true, stages: [{ id: 11, enabled: false, label: "Dormant", effect: { id: 12, kind: { kind: "isf", effect: edge(7) } } }] }] };
  assert.equal(imageModel.fxImageLayerWouldCycle(7, 99, dependencyLayers, clipEdges), true, "disabled Clip FX dependencies are checked too");
  const auditionModel = await server.ssrLoadModule("/src/fxAuditionModel.ts");
  const idA = { ...imageStage, image_bindings: [{ name: "overlay", source: { kind: "layer", layer_id: 16777216 } }] };
  const idB = { ...imageStage, image_bindings: [{ name: "overlay", source: { kind: "layer", layer_id: 16777217 } }] };
  assert.equal(auditionModel.fxChainsEqual(idA, idB), false, "stable layer IDs must not be rounded to f32 by audition equality");
  assert.deepEqual(auditionModel.cloneFxChain(stillBound).image_bindings, stillBound.image_bindings, "audition clone preserves image bindings");

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
  const authoredSynced = structuredClone(synced);
  for (const [bpm, expected] of [[60, 2], [90, 3], [120, 4], [150, 5], [240, 8]]) {
    assert.equal(model.fxTempoSyncState(synced, bpm).target, expected);
    assert.equal(model.effectiveFxValue(synced, bpm)[0], expected, `live rate at ${bpm} BPM`);
  }
  assert.deepEqual(synced, authoredSynced, "clock reads never mutate authored values or sync metadata");
  assert.equal(model.effectiveFxValue(synced, 150)[0], 5, "the next clock sample changes the effective rate");
  assert.equal(model.effectiveFxValue(synced, 150)[0], 5, "retained engine BPM keeps the nominal rate");
  for (const bad of [0, -5, NaN, Infinity, undefined]) {
    assert.deepEqual(model.effectiveFxValue(synced, bad), synced.value, "unusable tempo retains the authored fallback");
  }
  assert.deepEqual(model.effectiveFxValue(rate, 150), rate.value, "ordinary controls do not follow tempo");
  assert.deepEqual(model.effectiveFxValue({ ...stage.controls[1], tempo_sync: synced.tempo_sync }, 150), stage.controls[1].value, "non-Float controls do not follow tempo");
  for (const sync of [{ unit: "beats", count: 0 }, { unit: "beats", count: 3 }, { unit: "beats", count: NaN }, { unit: "seconds", count: 1 }]) {
    assert.ok(Number.isNaN(model.fxTempoSyncHz(120, sync)), "unsupported metadata cannot derive a live rate");
  }
  const reversedBounds = { ...synced, minimum: [10,0,0,0], maximum: [1,0,0,0] };
  assert.deepEqual(model.fxTempoSyncState(reversedBounds, 120), { target: 4, clamped: false }, "native-accepted reversed bounds use their ordered interval");
  assert.deepEqual(model.fxTempoSyncState(reversedBounds, 15), { target: 1, clamped: true });
  assert.deepEqual(model.fxTempoSyncState(reversedBounds, 600), { target: 10, clamped: true });
  assert.equal(model.fxTempoSyncState(rate, 120), null, "unsynced controls have no sync state");
  assert.equal(model.fxTempoSyncState(synced, undefined), null, "no usable tempo, no state");
  assert.equal(model.fxTempoSyncState(model.withFxTempoSync(rate, { unit: "bars", count: 4 }, 120), 120).clamped, true, "0.125 Hz is below the 0.5 Hz minimum");
  assert.equal(model.withFxTempoSync(rate, { unit: "bars", count: 4 }, 120).value[0], 0.5, "the range limit is used when the tempo asks for less");
  assert.equal(model.withFxTempoSync(rate, { unit: "beats", count: 0.25 }, 600).value[0], 30, "and the upper limit when it asks for more");
  const unsynced = model.withFxTempoSync(synced, null, 120);
  assert.equal("tempo_sync" in unsynced, false, "Hz removes the sync");
  assert.equal(unsynced.value[0], 4, "and keeps the effective value as a plain number");
  const changedTempoHz = model.withFxTempoSync(synced, null, 150);
  assert.equal(changedTempoHz.value[0], 5, "switching to Hz captures the current effective rate after a tempo change");
  assert.equal("tempo_sync" in changedTempoHz, false);
  assert.deepEqual(synced, authoredSynced, "switching units preserves the source image");
  assert.equal(model.withFxTempoSync(synced, null, undefined).value[0], 4, "without tempo, Hz captures the authored fallback");
  const clampedSync = model.withFxTempoSync(rate, { unit: "beats", count: 0.25 }, 120);
  assert.equal(model.withFxTempoSync(clampedSync, null, 600).value[0], 30, "Hz captures the current clamped rate");
  assert.equal(model.withFxTempoSync(rate, { unit: "beats", count: 1 }, undefined).value[0], 8, "without a tempo the value is left alone");
  const syncedStage = { ...strobe, controls: [synced, duty] };
  const resynced = model.resyncFxStage(syncedStage, 150);
  assert.equal(resynced.controls[0].value[0], 5, "explicit preset recall captures the current tempo (150 BPM, 1/2 beat)");
  assert.deepEqual(resynced.controls[1], duty, "ordinary controls are untouched");
  assert.equal(syncedStage.controls[0].value[0], 4, "preset recall never mutates the stage it reads");
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
  console.log("PASS FX parameters: ISF labels/type presentation; validated ranges/defaults; colour/point arity; declared choice/random bounds; exact undo; stale identity rejection; tempo-synced beat/bar units (live BPM following, authored-value preservation, range clamp, Hz capture, project round trip); insert-style stage reorder; presets saved/reopened from a project specimen as UTF-8 JSON (native save/load remains a separate gate).");
} finally { await server.close(); if (temporary) await rm(temporary, { recursive: true, force: true }); }
