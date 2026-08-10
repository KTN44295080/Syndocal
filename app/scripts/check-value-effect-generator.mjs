import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const [
  appSource,
  editorSource,
  colorEditorSource,
  typesSource,
  defaultsSource,
  sceneSettingsSource,
  randomCompatibilitySource,
  spatialCompatibilitySource,
] = await Promise.all([
  readFile(new URL("../src/App.tsx", import.meta.url), "utf8"),
  readFile(new URL("../src/components/ValueEffectEditorPanel.tsx", import.meta.url), "utf8"),
  readFile(new URL("../src/components/ColorEffectEditorPanel.tsx", import.meta.url), "utf8"),
  readFile(new URL("../src/types.ts", import.meta.url), "utf8"),
  readFile(new URL("../src/sceneFxDefaults.ts", import.meta.url), "utf8"),
  readFile(new URL("../src/components/SceneSettingsPane.tsx", import.meta.url), "utf8"),
  readFile(new URL("../src/randomEffectCompatibility.ts", import.meta.url), "utf8"),
  readFile(new URL("../src/spatialRecipeCompatibility.ts", import.meta.url), "utf8"),
]);
const randomCompatibilityTranspiled = ts.transpileModule(randomCompatibilitySource, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
  fileName: "randomEffectCompatibility.ts",
});
const randomCompatibility = await import(
  `data:text/javascript;base64,${Buffer.from(randomCompatibilityTranspiled.outputText).toString("base64")}`
);

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
      && consolidatedEditorSource.includes("defaultSpatialRecipe(kind)")
      && consolidatedEditorSource.includes("burstEvaluatorPatch(daslightExact,")
      && consolidatedEditorSource.includes("sweepEvaluatorPatch(daslightExact)")
      && consolidatedEditorSource.includes("perlinEvaluatorPatch(daslightExact,"),
    "COLOR and VALUE editors must share one authored-default and Enhanced/DVC conversion layer",
  );
  assert.ok(
    consolidatedEditorSource.includes('perlinHasMappingPlacement')
      && consolidatedEditorSource.includes('props.spatialPattern?.placement !== undefined'),
    "Perlin horizontal symmetry and rotation must stay gated on real mapping placement",
  );
}
assert.doesNotMatch(
  spatialCompatibilitySource,
  /daslight_exact: true,\s*grayscale: false,/,
  "the corrected DVC route must retain the authored qGray state instead of clearing it",
);
assert.ok(
  spatialCompatibilitySource.includes('grayscale: values.boolean("grayscale")'),
  "corrected Perlin must carry the authored qGray state forward",
);
assert.ok(
  editorSource.includes("data-value-sweep-direction-change"),
  "VALUE Sweep must expose Daslight's Direction Change switch",
);
assert.ok(
  editorSource.includes("data-value-sweep-evaluator"),
  "VALUE Sweep must expose the Enhanced / DVC-corrected compatibility boundary",
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
    3,
    "Burst, Sweep, and Perlin must expose the corrected DVC evaluator next to Enhanced",
  );
  assert.doesNotMatch(
    effectEditorSource,
    /40 ms compatibility|daslightExactTiming|Math\.floor\([^\n]*\/ 40\)/,
    "DVC evaluators must not quantize authored periods to recovered 40 ms frames",
  );
  assert.ok(
    effectEditorSource.includes('Size<input type="number" min="1" max="100" step="1"'),
    "Knight Rider Size must stay inside the recovered DVC 1..100 source domain",
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

assert.equal(randomCompatibility.materializedRandomEffectSeed(undefined), 0);
assert.equal(randomCompatibility.materializedRandomEffectSeed(Number.NaN), 0);
assert.equal(randomCompatibility.materializedRandomEffectSeed(-1), 0);
assert.equal(randomCompatibility.materializedRandomEffectSeed(1.6), 2);
assert.equal(randomCompatibility.materializedRandomEffectSeed(0x1_0000_0000), 0xffff_ffff);
assert.equal(randomCompatibility.correctedSparkleLifetimeMsFromLegacyPercent(undefined, undefined), 100);
assert.equal(randomCompatibility.correctedSparkleLifetimeMsFromLegacyPercent(2000, undefined), 100);
assert.equal(randomCompatibility.correctedSparkleLifetimeMsFromLegacyPercent(undefined, 25), 100);
assert.equal(randomCompatibility.correctedSparkleLifetimeMsFromLegacyPercent(Number.NaN, 25), 100);
assert.equal(randomCompatibility.correctedSparkleLifetimeMsFromLegacyPercent(2000, Number.NaN), 100);
assert.equal(randomCompatibility.correctedSparkleLifetimeMsFromLegacyPercent(1000, 0), 100);
assert.equal(randomCompatibility.correctedSparkleLifetimeMsFromLegacyPercent(1000, 5), 100);
assert.equal(randomCompatibility.correctedSparkleLifetimeMsFromLegacyPercent(2000, 25), 500);
assert.equal(randomCompatibility.correctedSparkleLifetimeMsFromLegacyPercent(4000, 50), 1000);
assert.equal(randomCompatibility.correctedSparkleLifetimeMsFromLegacyPercent(1025, 100), 1000);
assert.equal(randomCompatibility.correctedSparkleLifetimeMsFromLegacyPercent(1025, 50), 513);
for (const randomEditorSource of [editorSource, colorEditorSource]) {
  assert.equal(
    [...randomEditorSource.matchAll(/setRandomEvaluator\(event\.currentTarget\.value === "corrected"\)/g)].length,
    2,
    "both Random fill and Sparkle evaluator switches must materialize corrected fields",
  );
  assert.ok(
    randomEditorSource.includes('rng_seed", 0)'),
    "missing rng_seed UI state must display the runtime default zero",
  );
  assert.ok(
    randomEditorSource.includes('lifespan", 0)'),
    "Sparkle Legacy lifespan must display the authored default zero, not a phantom 25",
  );
  assert.ok(
    randomEditorSource.includes("correctedSparkleLifetimeMsFromLegacyPercent"),
    "Sparkle Legacy-to-Corrected must materialize lifetime_ms",
  );
}

console.log("value effect generator contracts ok");
