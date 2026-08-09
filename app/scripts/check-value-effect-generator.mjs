import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const [appSource, editorSource, typesSource, defaultsSource, sceneSettingsSource] = await Promise.all([
  readFile(new URL("../src/App.tsx", import.meta.url), "utf8"),
  readFile(new URL("../src/components/ValueEffectEditorPanel.tsx", import.meta.url), "utf8"),
  readFile(new URL("../src/types.ts", import.meta.url), "utf8"),
  readFile(new URL("../src/sceneFxDefaults.ts", import.meta.url), "utf8"),
  readFile(new URL("../src/components/SceneSettingsPane.tsx", import.meta.url), "utf8"),
]);

assert.match(
  typesSource,
  /export interface ValueEffectRequest[\s\S]*?spatial_pattern\?: ColorEffectSpatialPattern \| null;/,
  "Value request must keep the additive optional generator body",
);
assert.ok(
  appSource.includes("createSignal<ColorEffectSpatialPattern | null>(null)"),
  "App draft state must distinguish Custom Envelope from a VALUE generator",
);
for (const contract of [
  "spatial_pattern: valueSpatialPattern()",
  'interpolation: valueSpatialPattern() ? "Line" : valueInterpolation()',
  'mode: valueSpatialPattern() ? "Absolute" : valueMode()',
  'direction: valueSpatialPattern() ? "Forward" : valueDirection()',
  "setValueSpatialPattern(value.spatial_pattern ?? null)",
  "spatialPattern: valueSpatialPattern()",
  "onSpatialPattern: setValueSpatialPattern",
]) {
  assert.ok(appSource.includes(contract), `App must retain VALUE generator contract: ${contract}`);
}

for (const generator of [
  "ColorRainbow",
  "Burst",
  "Plasma",
  "KnightRider",
  "Sparkle",
  "RandomFill",
  "Perlin",
]) {
  assert.ok(
    editorSource.includes(`<option value="${generator}">`),
    `Scene Settings must expose verified VALUE generator ${generator}`,
  );
  assert.ok(
    editorSource.includes(`case "${generator}":`),
    `VALUE generator ${generator} must have an authored default recipe`,
  );
}
assert.ok(
  editorSource.includes('<option value="CustomEnvelope">Custom envelope</option>'),
  "Syndocal's richer Custom Envelope mode must remain available",
);
assert.ok(
  editorSource.includes("beam_targets: props.spatialPattern?.beam_targets ?? []"),
  "changing a VALUE recipe must preserve exact imported beam targets",
);
assert.ok(
  editorSource.includes('generatorKind() === "CustomEnvelope"'),
  "custom-only controls must remain separated from canonical generator fields",
);
assert.ok(
  editorSource.includes("Black 0 White 100 value palette"),
  "generator accessibility text must describe Daslight's scalar palette model",
);
assert.match(
  defaultsSource,
  /if \(family === "VALUE FX"\) \{[\s\S]*?return \{\s*Value: \{[\s\S]*?spatial_pattern: \{[\s\S]*?ColorRainbow:/,
  "one-gesture Scene Settings VALUE FX creation must create Value + Rainbow generator, not an LFO",
);
assert.doesNotMatch(
  defaultsSource,
  /if \(family === "VALUE FX"\) \{[\s\S]*?return \{\s*Lfo:/,
  "VALUE FX must not silently create a Square LFO",
);
assert.ok(
  sceneSettingsSource.includes('"VALUE FX": "Black / White generator"'),
  "Scene Settings must describe the generator it actually creates",
);

console.log("value effect generator contracts ok");
