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
  console.log("PASS FX parameters: ISF labels/type presentation; validated ranges/defaults; colour/point arity; declared choice/random bounds; exact undo; stale identity rejection; presets saved/reopened from project-shaped UTF-8 disk file (native save/load verified separately).");
} finally { await server.close(); if (temporary) await rm(temporary, { recursive: true, force: true }); }
