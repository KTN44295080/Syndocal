import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const [
  appSource,
  editorSource,
  colorEditorSource,
  typesSource,
  defaultsSource,
  sceneSettingsSource,
  spatialCompatibilitySource,
] = await Promise.all([
  readFile(new URL("../src/App.tsx", import.meta.url), "utf8"),
  readFile(new URL("../src/components/ValueEffectEditorPanel.tsx", import.meta.url), "utf8"),
  readFile(new URL("../src/components/ColorEffectEditorPanel.tsx", import.meta.url), "utf8"),
  readFile(new URL("../src/types.ts", import.meta.url), "utf8"),
  readFile(new URL("../src/sceneFxDefaults.ts", import.meta.url), "utf8"),
  readFile(new URL("../src/components/SceneSettingsPane.tsx", import.meta.url), "utf8"),
  readFile(new URL("../src/spatialRecipeCompatibility.ts", import.meta.url), "utf8"),
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
  "Sweep",
  "Sparkle",
  "RandomFill",
  "Perlin",
]) {
  assert.ok(
    editorSource.includes(`<option value="${generator}">`),
    `Scene Settings must expose verified VALUE generator ${generator}`,
  );
  assert.ok(
    spatialCompatibilitySource.includes(`case "${generator}":`),
    `VALUE generator ${generator} must have an authored default recipe`,
  );
}
for (const consolidatedEditorSource of [editorSource, colorEditorSource]) {
  assert.ok(
    consolidatedEditorSource.includes('from "../spatialRecipeCompatibility"')
      && consolidatedEditorSource.includes("defaultSpatialRecipe(kind)"),
    "COLOR and VALUE editors must share one unified authored-default layer",
  );
}
assert.ok(
  colorEditorSource.includes("perlinHasMappingPlacement")
    && colorEditorSource.includes("props.spatialPattern?.placement !== undefined"),
  "placed COLOR Perlin must gate horizontal symmetry and rotation on real mapping placement",
);
assert.ok(
  colorEditorSource.includes('["KnightRider", "Burst", "Sweep", "RandomFill", "Sparkle", "Spiral", "Butterfly", "Plasma"]')
    && colorEditorSource.includes("data-color-random-fill-point-height")
    && colorEditorSource.includes('<Show when={props.spatialPattern?.placement}><label>Point height %')
    && colorEditorSource.includes('min="1" max="10" step="1" value={spatialNumber("source_point_height", 1)}')
    && colorEditorSource.includes("data-color-random-fill-point-width-2d")
    && colorEditorSource.includes('min="1" max="10" step="1" value={spatialNumber("point_width", 1)}')
    && colorEditorSource.includes("data-color-random-fill-point-width-1d")
    && colorEditorSource.includes('min="0.01" max="100000" step="0.1" value={spatialNumber("point_width", 10)}')
    && colorEditorSource.includes("recipe.RandomFill.source_point_height = 1"),
  "placed COLOR Random Fill must expose its integer 1..10 width/height while preserving the unplaced width domain",
);
assert.match(
  typesSource,
  /RandomFill: \{[^}]*source_point_height\?: number \| null/,
  "Random Fill Point height must remain an additive optional field for legacy unplaced JSON",
);
assert.ok(
  colorEditorSource.includes("recipe: { [kind]: { ...spatialValues(), ...patch } }")
    && colorEditorSource.includes("source_point_height: clamp"),
  "editing placed Random Fill must preserve an imported height until that field is explicitly changed",
);
assert.doesNotMatch(
  editorSource,
  /horizontal_symmetry|rotation_degrees/,
  "one-dimensional VALUE Perlin must not invent placed mapping controls",
);
assert.doesNotMatch(
  spatialCompatibilitySource,
  /daslight_exact|syndocal_corrected|EvaluatorPatch/,
  "the shared default layer must not reintroduce evaluator routes",
);
assert.ok(
  editorSource.includes("data-value-sweep-direction-change"),
  "VALUE Sweep must expose Daslight's Direction Change switch",
);
assert.doesNotMatch(editorSource, /data-value-sweep-evaluator|DVC corrected|Enhanced/);
assert.ok(
  editorSource.includes("data-value-knight-transform"),
  "VALUE Knight Rider must expose its unified transform field",
);
assert.ok(
  editorSource.includes("data-value-sweep-transform"),
  "VALUE Sweep must expose the now-proven Daslight Transform switch",
);
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

for (const effectEditorSource of [editorSource, colorEditorSource]) {
  assert.equal(
    [...effectEditorSource.matchAll(/<option value="daslight">DVC corrected<\/option>/g)].length,
    0,
    "unified recipes must not expose evaluator routing options",
  );
  assert.doesNotMatch(
    effectEditorSource,
    /40 ms compatibility|daslightExactTiming|Math\.floor\([^\n]*\/ 40\)/,
    "DVC evaluators must not quantize authored periods to recovered 40 ms frames",
  );
  assert.ok(
    effectEditorSource.includes('Size %<input type="number" min="0.01" max="100000" step="0.1"'),
    "Knight Rider Size must use the unified strip-percentage domain",
  );
  assert.ok(
    effectEditorSource.includes("Simultaneous particles created per 40 ms generation")
      && effectEditorSource.includes("Effect-time milliseconds, scaling with clock sync and BPM speed"),
    "Sparkle number and lifetime must state their corrected generation semantics",
  );
}
assert.doesNotMatch(
  appSource,
  /if \(interpolation\.startsWith\("Daslight"\)\) \{\s*setEffectPeriod/,
  "switching MOVE interpolation must preserve the authored period",
);

for (const randomEditorSource of [editorSource, colorEditorSource]) {
  assert.doesNotMatch(randomEditorSource, /setRandomEvaluator|Syndocal corrected|Legacy lifespan/);
  assert.ok(
    randomEditorSource.includes('rng_seed", 0)'),
    "missing rng_seed UI state must display the runtime default zero",
  );
  assert.ok(
    randomEditorSource.includes("Point width %")
      && randomEditorSource.includes("Sparkle width %")
      && randomEditorSource.includes("Lifetime ms"),
    "Random fill and Sparkle must expose only the unified native domains",
  );
}

console.log("value effect generator contracts ok");
