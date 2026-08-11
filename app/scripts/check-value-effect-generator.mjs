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
for (const [kind, defaultRecipe] of [
  ["Grid", "Grid: { grayscale: false, size: 1, width: 2 }"],
  ["Lines", "Lines: { grayscale: false, size: 2 }"],
  ["Graph", "Graph: { grayscale: false, height: 10, width: 10, pitch: 10, frequency: 2, amplitude: 1, offset: 0 }"],
]) {
  assert.ok(
    spatialCompatibilitySource.includes(`case "${kind}":`) && spatialCompatibilitySource.includes(defaultRecipe),
    `COLOR MAPPINGS ${kind} must keep its exact authored default recipe`,
  );
}
assert.match(typesSource, /\| \{ Grid: \{ grayscale: boolean; size: number; width: number \} \}/);
assert.match(typesSource, /\| \{ Lines: \{ grayscale: boolean; size: number \} \}/);
assert.match(typesSource, /\| \{ Graph: \{ grayscale: boolean; height: number; width: number; pitch: number; frequency: number; amplitude: number; offset: number \} \}/);
assert.ok(
  colorEditorSource.includes('<option value="Grid" disabled={spatialKindUnavailable("Grid")}>Grid mapping</option>')
    && colorEditorSource.includes('<option value="Lines" disabled={spatialKindUnavailable("Lines")}>Lines mapping</option>')
    && colorEditorSource.includes('<option value="Graph" disabled={spatialKindUnavailable("Graph")}>Graph mapping</option>')
    && colorEditorSource.includes('data-color-grid-controls')
    && colorEditorSource.includes('data-color-grid-size')
    && colorEditorSource.includes('data-color-grid-width')
    && colorEditorSource.includes('data-color-lines-controls')
    && colorEditorSource.includes('data-color-lines-size'),
  "COLOR MAPPINGS Grid, Lines, and Graph must remain selectable with their dedicated controls",
);
assert.ok(
  colorEditorSource.includes("const palettePolicyForSpatialKind")
    && colorEditorSource.includes('if (kind === "Grid") return gridColorPalettePolicy;')
    && colorEditorSource.includes('if (kind === "Graph") return graphColorPalettePolicy;')
    && colorEditorSource.includes('if (kind === "Lines") return placedColorPalettePolicy;')
    && colorEditorSource.includes("if (spatialKindUnavailable(kind)) return;"),
  "COLOR MAPPINGS Grid, Lines, and Graph must share one prospective placement and palette-policy guard",
);
assert.ok(
  colorEditorSource.includes("minStops: 2,\n  maxStops: 5,")
    && colorEditorSource.includes("minStops: 2,\n  maxStops: 10,")
    && colorEditorSource.includes("minStops: 2,\n  maxStops: DASLIGHT_FX_PALETTE_MAX_STOPS,")
    && colorEditorSource.includes("props.stops.length < policy.minStops")
    && colorEditorSource.includes("props.stops.length > policy.maxStops"),
  "Grid must enforce 2..5, Graph 2..10, and Lines 2..255 stops at selection, including rejection at 11 Graph stops",
);
assert.ok(
  colorEditorSource.includes("orderedStops().length <= currentPalettePolicy().minStops")
    && colorEditorSource.includes("stops.length >= currentPalettePolicy().maxStops")
    && colorEditorSource.includes("orderedStops().length >= currentPalettePolicy().maxStops")
    && colorEditorSource.includes("{orderedStops().length} / {currentPalettePolicy().maxStops} stops")
    && colorEditorSource.includes("return policy.countError;"),
  "palette errors, add/remove guards, disabled controls, and footer must share current recipe bounds",
);
assert.ok(
  colorEditorSource.includes('["KnightRider", "Burst", "Sweep", "RandomFill", "Sparkle", "Spiral", "Butterfly", "Plasma", "Grid", "Lines", "Graph"]')
    && colorEditorSource.includes('min="1" max="5" step="1" value={spatialNumber("size", 1)}')
    && colorEditorSource.includes('min="2" max="20" step="1" value={spatialNumber("width", 2)}')
    && colorEditorSource.includes('min="2" max="20" step="1" value={spatialNumber("size", 2)}'),
  "placed COLOR MAPPINGS Grid, Lines, and Graph must reuse placement Transform/Rotation and preserve their integer domains",
);
const gridControls = colorEditorSource.match(/<Show when=\{spatialKind\(\) === "Grid"\}>([\s\S]*?)<\/Show>/)?.[1] ?? "";
const linesControls = colorEditorSource.match(/<Show when=\{spatialKind\(\) === "Lines"\}>([\s\S]*?)<\/Show>/)?.[1] ?? "";
const graphControls = colorEditorSource.match(/<Show when=\{spatialKind\(\) === "Graph"\}>([\s\S]*?)<\/Show>/)?.[1] ?? "";
for (const [kind, controls] of [["Grid", gridControls], ["Lines", linesControls], ["Graph", graphControls]]) {
  assert.match(controls, /checked=\{spatialBoolean\("grayscale"\)\}/, `COLOR MAPPINGS ${kind} must expose Grayscale`);
  assert.doesNotMatch(controls, /<label>Transform/, `COLOR MAPPINGS ${kind} must not duplicate placed Mapping Transform`);
}
for (const contract of [
  'data-color-graph-controls',
  'data-color-graph-height',
  'data-color-graph-width',
  'data-color-graph-pitch',
  'data-color-graph-frequency',
  'data-color-graph-amplitude',
  'data-color-graph-offset',
  'min="1" max="100" step="1" value={spatialNumber("height", 10)}',
  'min="1" max="100" step="1" value={spatialNumber("width", 10)}',
  'min="0" max="100" step="1" value={spatialNumber("pitch", 10)}',
  'min="0" max="10" step="1" value={spatialNumber("frequency", 2)}',
  'min="0" max="2" step="0.1" value={spatialNumber("amplitude", 1)}',
  'min="-1" max="1" step="0.1" value={spatialNumber("offset", 0)}',
]) {
  assert.ok(graphControls.includes(contract), `COLOR MAPPINGS Graph must expose ${contract}`);
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
  colorEditorSource.includes('["KnightRider", "Burst", "Sweep", "RandomFill", "Sparkle", "Spiral", "Butterfly", "Plasma", "Grid", "Lines", "Graph"]')
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
