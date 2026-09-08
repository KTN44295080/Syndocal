import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const protocol = (await readFile(new URL("../../crates/protocol/src/lib.rs", import.meta.url), "utf8")).replaceAll("\r\n", "\n");
const engine = await readFile(new URL("../../crates/engine/src/lib.rs", import.meta.url), "utf8");
const importer = await readFile(new URL("../src-tauri/src/dvc_import.rs", import.meta.url), "utf8");
const backend = await readFile(new URL("../src-tauri/src/main.rs", import.meta.url), "utf8");
const app = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
const editor = await readFile(new URL("../src/components/EditableTouchSurface.tsx", import.meta.url), "utf8");

const checks = [
  [protocol.includes("pub struct TouchFeaturePresetTarget"), "protocol keeps explicit fixture/attribute targets"],
  [protocol.includes("FeaturePreset {\n        targets: Vec<TouchFeaturePresetTarget>"), "Touch binding persists Feature Presets"],
  [importer.includes('"223" => match parse_touch_feature_preset_binding'), "DVC Action 223 is imported"],
  [importer.includes('target == ":-1:4"'), "verified Generic Dimmer target remains explicit"],
  [importer.includes("dvc_local_homecoming_laser_touch_faders_map_verified_feature_presets_when_present"), "local Daslight golden is retained"],
  [engine.includes("SetFixtureAttributeBatch"), "engine applies Feature Preset targets as one command"],
  [engine.includes("fixture_attribute_batch_rejects_invalid_target_without_partial_update"), "batch mutation is atomic on validation failure"],
  [backend.includes("requires 1..512 Feature Preset targets"), "project validation bounds imported target sets"],
  [app.includes('"set_fixture_attribute_batch"'), "Touch faders invoke the batch path"],
  [editor.includes("Exact fixture/attribute selection and fader range are preserved."), "Touch editor identifies preserved imports"],
];

for (const [condition, message] of checks) assert.ok(condition, message);
console.log(`dvc touch feature preset ok: ${checks.length} assertions`);
