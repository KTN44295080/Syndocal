import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const importTypeScript = async (relativeUrl, fileName, options = {}) => {
  let source = await readFile(new URL(relativeUrl, import.meta.url), "utf8");
  if (options.solid) {
    source = source.replace('from "solid-js";', `from "${import.meta.resolve("solid-js")}";`);
  }
  const transpiled = ts.transpileModule(source, {
    compilerOptions: {
      module: ts.ModuleKind.ESNext,
      target: ts.ScriptTarget.ES2022,
      importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
      jsx: options.solid ? ts.JsxEmit.React : ts.JsxEmit.None,
    },
    fileName,
  });
  return import(`data:text/javascript;base64,${Buffer.from(transpiled.outputText).toString("base64")}`);
};

const visualization = await importTypeScript(
  "../src/effectVisualization.ts",
  "effectVisualization.ts",
);
const moveEffect = await importTypeScript(
  "../src/moveEffect.ts",
  "moveEffect.ts",
);
const chooser = await importTypeScript(
  "../src/components/EffectFamilyChooser.tsx",
  "EffectFamilyChooser.tsx",
  { solid: true },
);
const chooserSource = await readFile(
  new URL("../src/components/EffectFamilyChooser.tsx", import.meta.url),
  "utf8",
);
const appSource = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
const sceneSettingsSource = await readFile(
  new URL("../src/components/SceneSettingsPane.tsx", import.meta.url),
  "utf8",
);
const sceneFxDefaultsSource = await readFile(
  new URL("../src/sceneFxDefaults.ts", import.meta.url),
  "utf8",
);
const tauriSource = await readFile(
  new URL("../src-tauri/src/main.rs", import.meta.url),
  "utf8",
);
const moveEditorSource = await readFile(
  new URL("../src/components/MoveEffectEditorPanel.tsx", import.meta.url),
  "utf8",
);
const curveEditorSource = await readFile(
  new URL("../src/components/CurveEffectEditorPanel.tsx", import.meta.url),
  "utf8",
);
const mappingEditorSource = await readFile(
  new URL("../src/components/MappingEffectEditorPanel.tsx", import.meta.url),
  "utf8",
);
const colorMappingEditorSource = await readFile(
  new URL("../src/components/ColorMappingEffectEditorPanel.tsx", import.meta.url),
  "utf8",
);
const protocolSource = await readFile(
  new URL("../../crates/protocol/src/lib.rs", import.meta.url),
  "utf8",
);
const engineSource = await readFile(
  new URL("../../crates/engine/src/lib.rs", import.meta.url),
  "utf8",
);

const expectedFamilies = [
  "STEPS",
  "COLOR FX",
  "CHASER FX",
  "MOVE FX",
  "VALUE FX",
  "CURVE FX",
  "2D MAPPING",
  "SUPER SCENE",
];
assert.deepEqual(
  chooser.effectChooserFamilies,
  expectedFamilies,
  "the chooser must expose the T16 families in the exact operator order",
);
assert.equal(chooser.effectChooserFamilies.length, 8, "the chooser must expose exactly eight operator families");
assert.equal(
  new Set(chooser.effectChooserFamilies).size,
  chooser.effectChooserFamilies.length,
  "every chooser family must be unique",
);
const initialFamily = appSource.match(
  /createSignal<EffectChooserFamily>\("([^"]+)"\)/,
)?.[1];
assert.ok(initialFamily, "App must declare an initial effect chooser family");
assert.equal(initialFamily, "CURVE FX", "the scene FX chooser must start on the Curve family");
assert.match(
  sceneSettingsSource,
  /data-scene-fx-chooser[\s\S]*?<EffectFamilyChooser/,
  "the nine-family chooser must live on the Scene Settings FX surface",
);
assert.match(
  chooserSource,
  /family === "SUPER SCENE" \? "TIMELINE" : family/,
  "the internal Super Scene family must render with the product-facing Timeline label",
);

const closeTo = (actual, expected, message, epsilon = 1e-10) => {
  assert.ok(
    Math.abs(actual - expected) <= epsilon,
    `${message}: expected ${expected}, received ${actual}`,
  );
};

const lfoExpectations = [
  ["Sine", 0.25, 1],
  ["Cosine", 0.5, 0],
  ["Triangle", 0.25, 0.5],
  ["Saw", 0.75, 0.75],
  ["Square", 0.5, 0],
  ["Random", 0, 0.007765133401324305],
  ["Perlin", 0.0625, 0.31504236238893174],
];
for (const [shape, phase, expected] of lfoExpectations) {
  closeTo(
    visualization.evaluateLfoShape(shape, phase),
    expected,
    `${shape} must mirror the engine evaluator`,
  );
  closeTo(
    visualization.evaluateLfoShape(shape, phase + 3),
    expected,
    `${shape} phase must wrap above one`,
  );
  closeTo(
    visualization.evaluateLfoShape(shape, phase - 2),
    expected,
    `${shape} phase must wrap below zero`,
  );
}
closeTo(
  visualization.evaluateLfoShape("Random", 0.06249),
  visualization.evaluateLfoShape("Random", 0),
  "Random must remain stepped inside one of its sixteen buckets",
);
closeTo(
  visualization.evaluateLfoShape("Random", 0.0625),
  0.6223195913765392,
  "Random must advance to the deterministic next hash bucket",
);
closeTo(
  visualization.evaluateLfoShape("Perlin", 0.03125),
  0.10378926745995164,
  "Perlin must use the engine's eased periodic interpolation",
);
assert.equal(
  visualization.buildLfoPreviewPath("Saw", 0.25, 100, 100, 4),
  "M 0 75 L 25 50 L 50 25 L 75 100 L 100 75",
  "the LFO path must apply the authored phase before plotting",
);
assert.equal(
  visualization.buildLfoPreviewPath("Saw", 0.25, 100, 100, 4, 16_384, 49_151, true),
  "M 0 62.5 L 25 50 L 50 37.5 L 75 75 L 100 62.5",
  "the LFO path must place the authored low/high range on the 16-bit output axis",
);
assert.equal(
  visualization.buildLfoPreviewPath("Saw", 0.25, 100, 100, 4, 49_151, 16_384, true),
  "M 0 37.5 L 25 50 L 50 62.5 L 75 25 L 100 37.5",
  "directed LFO ranges must preserve an authored high-to-low inversion",
);
assert.equal(
  visualization.buildLfoPreviewPath("Saw", 0.25, 100, 100, 4, 49_151, 16_384, false),
  "M 0 62.5 L 25 50 L 50 37.5 L 75 75 L 100 62.5",
  "Position Wave ranges must mirror the runtime's sorted low/high scaling",
);

const red = { red: 65_535, green: 0, blue: 0 };
const green = { red: 0, green: 65_535, blue: 0 };
const blue = { red: 0, green: 0, blue: 65_535 };
assert.deepEqual(
  visualization.interpolateEffectColor(red, blue, 0.5, "Rgb"),
  { red: 32_768, green: 0, blue: 32_768 },
  "RGB interpolation must interpolate saved 16-bit channels directly",
);
assert.deepEqual(
  visualization.interpolateEffectColor(red, green, 0.5, "HsvShortest"),
  { red: 65_535, green: 65_535, blue: 0 },
  "HSV shortest must travel from red to green through yellow",
);
assert.deepEqual(
  visualization.interpolateEffectColor(red, green, 0.5, "HsvLongest"),
  blue,
  "HSV longest must travel from red to green through blue",
);
assert.deepEqual(
  visualization.interpolateEffectColor(red, red, 0.5, "HsvLongest"),
  { red: 0, green: 65_535, blue: 65_535 },
  "HSV longest must preserve the engine's full-turn same-hue behavior",
);

const authoredStops = [
  { position: 0.75, color: blue },
  { position: 0.25, color: red },
];
const stopsBefore = structuredClone(authoredStops);
assert.deepEqual(
  visualization.sampleColorStops(authoredStops, "Rgb", 0.1),
  red,
  "positions before the first authored stop must hold its color",
);
assert.deepEqual(
  visualization.sampleColorStops(authoredStops, "Rgb", 0.5),
  { red: 32_768, green: 0, blue: 32_768 },
  "the midpoint must be relative to the authored stop positions",
);
assert.deepEqual(
  visualization.sampleColorStops(authoredStops, "Rgb", 0.9),
  blue,
  "positions after the last authored stop must hold its color",
);
assert.deepEqual(authoredStops, stopsBefore, "sampling must not reorder or mutate authored stops");
assert.equal(
  visualization.buildColorGradient(authoredStops, "Rgb", 4),
  "linear-gradient(90deg, #ff0000 0%, #ff0000 25%, #800080 50%, #0000ff 75%, #0000ff 100%)",
  "the CSS gradient must reflect authored stop positions and interpolation",
);

const movePoints = [
  { x: 0, y: 100 },
  { x: 50, y: 50 },
  { x: 100, y: 0 },
];
const moveBefore = structuredClone(movePoints);
assert.deepEqual(
  visualization.sampleMovePath(movePoints, "Line", false),
  movePoints,
  "line interpolation must retain the authored vertices",
);
const smoothMove = visualization.sampleMovePath(movePoints, "Smooth", false);
assert.equal(smoothMove.length, 65, "two smooth segments must receive 32 samples each");
closeTo(smoothMove[32].x, 50, "the first smooth segment must finish on its authored vertex");
closeTo(smoothMove[32].y, 50, "the first smooth segment must finish on its authored vertex");
closeTo(smoothMove.at(-1).x, 100, "the smooth path must finish on its final authored vertex");
closeTo(smoothMove.at(-1).y, 0, "the smooth path must finish on its final authored vertex");
assert.deepEqual(movePoints, moveBefore, "path sampling must not mutate authored vertices");
const circleSquare = [
  { x: 25, y: 50 },
  { x: 50, y: 25 },
  { x: 75, y: 50 },
  { x: 50, y: 75 },
];
const circleMove = visualization.sampleMovePath(circleSquare, "Circle", true);
assert.equal(circleMove.length, 129, "four Circle segments must receive equal 32-sample time slices");
closeTo(circleMove[16].x, 32.32233047, "Circle first-segment midpoint X", 1e-8);
closeTo(circleMove[16].y, 32.32233047, "Circle first-segment midpoint Y", 1e-8);
const circleBoundaries = circleMove.filter((_, index) => index % 32 === 0);
for (const [index, expected] of [...circleSquare, circleSquare[0]].entries()) {
  closeTo(circleBoundaries[index].x, expected.x, `Circle equal-time boundary ${index} X`);
  closeTo(circleBoundaries[index].y, expected.y, `Circle equal-time boundary ${index} Y`);
}
const inflectionCircle = visualization.sampleMovePath([
  { x: 5.8411848, y: 10.201688 },
  { x: 13.235159, y: 95.50081 },
  { x: 9.455622, y: 95.10565 },
  { x: 83.738565, y: 75.331795 },
], "Circle", true);
closeTo(
  inflectionCircle[24].x,
  24.79715493,
  "Circle inflection must reflect the raw first-half arc before final X clamp",
  2e-6,
);
closeTo(
  inflectionCircle[24].y,
  73.01356758,
  "Circle inflection must reflect the raw first-half arc before final Y clamp",
  2e-6,
);
const validMoveDraft = {
  fixtureIds: [1],
  targetGroupIds: [],
  points: [{ x: 0, y: 0 }, { x: 1, y: 1 }],
  beamTargets: [],
  closed: true,
  interpolation: "Circle",
  centerX: 0.5,
  centerY: 0.5,
  sizeX: 1,
  sizeY: 1,
  rotationDegrees: 0,
  periodMs: 1_000,
  clockSyncBeats: null,
  phase: 0,
  fixtureSpread: 0,
  coordinateMode: "Absolute",
  blendMode: "Override",
};
const circleFanoutRequest = {
  period_ms: 1_000,
  direction: "Forward",
  phase: 0,
  fixture_spread: 1,
  symmetry: false,
  interpolation: "Circle",
};
assert.deepEqual(
  visualization.moveFanoutPreviewState(circleFanoutRequest, 250, 1, 4),
  { progress: 0, mirrorPan: false },
  "Circle fan-out must subtract first-wing selection spread",
);
assert.deepEqual(
  visualization.moveFanoutPreviewState({ ...circleFanoutRequest, symmetry: true }, 250, 2, 4),
  { progress: 0.25, mirrorPan: false },
  "Circle symmetry must reverse second-wing traversal from half-cycle",
);
assert.deepEqual(
  visualization.moveFanoutPreviewState({
    ...circleFanoutRequest,
    interpolation: "Smooth",
    symmetry: true,
  }, 250, 3, 4),
  { progress: 0, mirrorPan: true },
  "legacy Smooth fan-out must retain positive spread and second-wing Pan mirroring",
);
assert.equal(moveEffect.moveEffectDraftError(validMoveDraft), "", "a closed two-point Circle draft must remain valid");
assert.equal(
  moveEffect.moveEffectDraftError({
    ...validMoveDraft,
    targetGroupIds: ["Moving"],
    beamTargets: [{ fixture_id: 1, beam_index: 3, selection_index: 0 }],
  }),
  "Move beam targets cannot be combined with group targets.",
);
assert.equal(
  moveEffect.moveEffectDraftError({
    ...validMoveDraft,
    beamTargets: [{ fixture_id: 2, beam_index: 3, selection_index: 0 }],
  }),
  "Move beam target is absent from fixture targets.",
);
assert.equal(
  moveEffect.moveEffectDraftError({
    ...validMoveDraft,
    beamTargets: [
      { fixture_id: 1, beam_index: 3, selection_index: 0 },
      { fixture_id: 1, beam_index: 3, selection_index: 1 },
    ],
  }),
  "Move beam target fixture/beam pairs must be unique.",
);
assert.match(
  appSource,
  /const cancelEffectEdit = \(\) => \{[\s\S]*?setEditingEffectId\(null\);[\s\S]*?setMoveBeamTargets\(\[\]\);/,
  "canceling a Move edit must clear source-specific beam identities before a new draft",
);
assert.equal(
  moveEffect.moveEffectDraftError({ ...validMoveDraft, closed: false }),
  "Move Circle interpolation requires a closed path.",
  "Circle drafts must expose their recovered closed-path requirement before save",
);
assert.equal(
  moveEffect.moveEffectDraftError({ ...validMoveDraft, points: Array.from({ length: 256 }, (_, index) => ({ x: index / 255, y: 0.5 })) }),
  "Move paths require between 2 and 255 points.",
  "Circle drafts must enforce the recovered POINTS maximum",
);
assert.equal(
  moveEffect.moveEffectDraftError({
    ...validMoveDraft,
    interpolation: "Smooth",
    closed: false,
    points: Array.from({ length: 256 }, (_, index) => ({ x: index / 255, y: 0.5 })),
  }),
  "",
  "the Circle limit must not shrink the legacy Line/Smooth 256-point domain",
);
const duplicateMovePoints = [
  { x: 0, y: 0 },
  { x: 0, y: 0 },
  { x: 0.5, y: 0.5 },
  { x: 0, y: 0 },
];
const duplicateMoveBefore = structuredClone(duplicateMovePoints);
assert.deepEqual(
  visualization.normalizeMovePathPoints(duplicateMovePoints, true),
  [{ x: 0, y: 0 }, { x: 0.5, y: 0.5 }],
  "runtime Move preview points must remove consecutive duplicates and a repeated closed endpoint",
);
assert.deepEqual(
  visualization.normalizeMovePathPoints([
    { x: 0, y: 0 },
    { x: 0.0000005, y: 0 },
    { x: 0.25, y: 0.25 },
  ], false),
  [{ x: 0, y: 0 }, { x: 0.25, y: 0.25 }],
  "runtime Move preview normalization must use the engine's 1e-6 distance threshold",
);
assert.deepEqual(duplicateMovePoints, duplicateMoveBefore, "Move normalization must not mutate authored vertices");
const duplicateCirclePoints = [
  { x: 0.1, y: 0.2 },
  { x: 0.1, y: 0.2 },
  { x: 0.8, y: 0.2 },
  { x: 0.4, y: 0.9 },
];
const duplicateCircleRuntime = visualization.normalizeMovePathPoints(
  duplicateCirclePoints,
  true,
  "Circle",
);
assert.deepEqual(
  duplicateCircleRuntime,
  duplicateCirclePoints,
  "Circle normalization must retain degenerate points as equal-time segment boundaries",
);
const duplicateCircleSamples = visualization.sampleMovePath(
  duplicateCircleRuntime.map((point) => ({ x: point.x * 100, y: point.y * 100 })),
  "Circle",
  true,
);
assert.equal(duplicateCircleSamples.length, 129, "four authored Circle points must retain four time segments");
closeTo(duplicateCircleSamples[32].x, 10, "degenerate Circle boundary X");
closeTo(duplicateCircleSamples[32].y, 20, "the degenerate first Circle segment must consume its full time slice");
closeTo(duplicateCircleSamples[64].x, 80, "the next Circle boundary must remain at half-cycle X");
closeTo(duplicateCircleSamples[64].y, 20, "the next Circle boundary must remain at half-cycle Y");
assert.deepEqual(
  visualization.movePointToPreview({ x: 1.2, y: -0.2 }),
  { x: 100, y: 100 },
  "protocol coordinates must clamp and invert Y for the screen preview",
);

const transformRequest = {
  coordinate_mode: "Absolute",
  center_x: 0.5,
  center_y: 0.5,
  size_x: 0.5,
  size_y: 2,
  rotation_degrees: 90,
};
const transformInput = [
  { x: 100, y: 50 },
  { x: 50, y: 0 },
];
const transformBefore = structuredClone(transformInput);
const transformed = visualization.transformMovePreview(transformInput, transformRequest);
closeTo(transformed[0].x, 50, "a positive rotation must retain the center X for the right vertex");
closeTo(transformed[0].y, 75, "positive editor rotation must move the right vertex clockwise");
closeTo(transformed[1].x, 100, "runtime output must clamp a rotated vertex at the right boundary");
closeTo(transformed[1].y, 50, "the rotated top vertex must land on the center Y");
assert.deepEqual(transformInput, transformBefore, "Move transforms must not mutate authored preview vertices");
assert.ok(
  transformed.every((point) => point.x >= 0 && point.x <= 100 && point.y >= 0 && point.y <= 100),
  "every transformed Move output must remain inside the runtime's normalized output range",
);
assert.deepEqual(
  visualization.transformMovePreview(
    [{ x: 50, y: 50 }, { x: 100, y: 50 }],
    {
      coordinate_mode: "Relative",
      center_x: 0.75,
      center_y: 0.25,
      size_x: 2,
      size_y: 1,
      rotation_degrees: 0,
    },
  ),
  [{ x: 75, y: 75 }, { x: 100, y: 75 }],
  "Relative previews must use a neutral 50% base and clamp after applying the authored delta",
);
const relativeFromKnownBase = visualization.transformMovePreview(
  [{ x: 50, y: 50 }, { x: 100, y: 50 }],
  {
    coordinate_mode: "Relative",
    center_x: 0.25,
    center_y: 0.75,
    size_x: 1,
    size_y: 1,
    rotation_degrees: 0,
  },
  { x: 0.8, y: 0.1 },
);
closeTo(relativeFromKnownBase[0].x, 55, "Relative known-base center X");
closeTo(relativeFromKnownBase[0].y, 65, "Relative known-base center Y");
closeTo(relativeFromKnownBase[1].x, 100, "Relative known-base output X must clamp");
closeTo(relativeFromKnownBase[1].y, 65, "Relative known-base output Y");
assert.match(
  moveEditorSource,
  /import \{[^}]*transformMovePreview[^}]*\} from "\.\.\/effectVisualization";/,
  "the Move editor must use the shared runtime-aligned transform helper",
);
assert.match(
  moveEditorSource,
  /transformMovePreview\(editablePathSamples\(\), previewTransform\(\)\)/,
  "the Move editor output path must use the shared runtime-aligned transform helper",
);
assert.doesNotMatch(
  moveEditorSource,
  /const transformedPathSamples = createMemo\(\(\) => \{[\s\S]*?Math\.cos/,
  "the Move editor must not maintain a divergent transform matrix",
);
assert.equal(
  visualization.buildPointPath(movePoints, true),
  "M 0 100 L 50 50 L 100 0 Z",
  "the plotted Move path must preserve vertex order and closure",
);

const valuePoints = [
  { position: 0, value: 0 },
  { position: 1, value: 1 },
];
assert.equal(visualization.sampleValueEnvelope(valuePoints, "Step", 0.75), 0);
assert.equal(visualization.sampleValueEnvelope(valuePoints, "Line", 0.75), 0.75);
assert.equal(
  visualization.buildValuePreviewPath(valuePoints, "Line", "Forward", 0, 100, 20, 2),
  "M 0 20 L 50 10 L 100 20",
  "Forward Value preview must plot saved values and wrap at the period boundary",
);
assert.equal(
  visualization.buildValuePreviewPath(valuePoints, "Line", "Reverse", 0, 100, 20, 2),
  "M 0 0 L 50 10 L 100 0",
  "Reverse Value preview must invert the saved envelope traversal",
);
assert.equal(
  visualization.buildValuePreviewPath(valuePoints, "Line", "Bounce", 0, 100, 20, 2),
  "M 0 20 L 50 0 L 100 20",
  "Bounce Value preview must travel out and back across the saved envelope",
);

const curvePoints = [
  { position: 0, value: 0, in_tangent: 0, out_tangent: 0 },
  { position: 1, value: 1, in_tangent: 0, out_tangent: 0 },
];
closeTo(
  visualization.sampleCurveFunction(curvePoints, 0.25),
  0.15625,
  "Curve sampling must match the engine's compiled cubic Hermite function",
);
closeTo(
  visualization.sampleCurveFunction(curvePoints, 0.75),
  0.84375,
  "Curve sampling must preserve the engine's right-side cubic tangent result",
);
assert.match(
  curveEditorSource,
  /buildCurvePreviewPath\(props\.points, props\.direction, 0/,
  "the independent Curve editor must use the shared runtime-aligned cubic preview helper",
);
assert.match(
  curveEditorSource,
  /in_tangent[\s\S]*?out_tangent/,
  "the independent Curve editor must expose both authored tangent handles",
);
assert.match(
  appSource,
  /family === "CURVE FX"\s*\? "Curve"/,
  "the CURVE FX family must select the independent Curve kind",
);
assert.match(
  sceneFxDefaultsSource,
  /if \(request\.beam_targets\?\.length\) return fixtures;/,
  "fixture-level scene preview must leave explicit beam bodies dormant instead of collapsing beam identity",
);
assert.match(
  sceneFxDefaultsSource,
  /sampled\.length - 1[\s\S]*?Math\.floor\(fanout\.progress \* sampleIntervals\)/,
  "Move scene preview must map phase across sampled path intervals rather than array length",
);
assert.match(
  tauriSource,
  /"curve"\s*\|\s*"curve-saw"/,
  "the backend Curve sample preset command must remain available after removing its global UI entry",
);

assert.match(
  mappingEditorSource,
  /buildLfoPreviewPath\(props\.shape, props\.phase/,
  "the independent Mapping editor must use the shared runtime-aligned LFO preview helper",
);
assert.match(
  mappingEditorSource,
  /props\.onFixtureOrder\(next\)/,
  "the independent Mapping editor must reorder authored fixture order",
);
assert.match(
  mappingEditorSource,
  /aria-label="Authored fixture order"/,
  "the independent Mapping editor must expose authored fixture order",
);
assert.match(
  appSource,
  /family === "2D MAPPING"\s*\? "ColorMapping"/,
  "the unified 2D MAPPING family must start in raster colour output mode",
);
for (const preset of ["wave", "ball", "fan"]) {
  assert.match(
    tauriSource,
    new RegExp(`"${preset}"`),
    `${preset} must remain available in the backend sample preset command`,
  );
}

assert.match(
  appSource,
  /nextType === "Mapping" \|\| nextType === "ColorMapping"\) return "2D MAPPING"/,
  "both persisted mapping kinds must return to the unified authoring family",
);
assert.match(
  tauriSource,
  /"colour-chase"/,
  "the Colour Chase backend sample preset must remain available",
);
assert.match(
  colorMappingEditorSource,
  /createImageBitmap\(file\)/,
  "the Colour Mapping editor must decode images before command submission",
);
assert.match(
  colorMappingEditorSource,
  /getImageData\(0, 0, canvas\.width, canvas\.height\)/,
  "the Colour Mapping editor must embed raster pixels in the authored request",
);
assert.match(
  colorMappingEditorSource,
  /Math\.max\(2, Math\.min\(MAX_FRAMES, Math\.ceil\(duration \* 8\)\)\)/,
  "video extraction must remain bounded to at most 64 embedded frames",
);
assert.match(
  colorMappingEditorSource,
  /Freeze stage positions as cells/,
  "the Colour Mapping editor must expose explicit fixture-cell authoring",
);
assert.match(
  colorMappingEditorSource,
  /no decoder runs at 44 Hz/,
  "the Colour Mapping editor must disclose the command-time decode contract",
);
assert.match(
  colorMappingEditorSource,
  /aria-label="2D Mapping output mode"[\s\S]*?>Colour<[\s\S]*?>Feature</,
  "the unified 2D Mapping editor must expose Colour and Feature output modes",
);
assert.match(
  colorMappingEditorSource,
  /feature_low: Math\.round\(clamp\(feature\.low, 0, 65_535\)\)[\s\S]*?feature_high: Math\.round\(clamp\(feature\.high, 0, 65_535\)\)/,
  "2D Mapping Feature output must author independent low/high ranges",
);
assert.match(
  protocolSource,
  /pub struct ColorMappingCellTarget[\s\S]*?serde\(default, skip_serializing_if = "Option::is_none"\)[\s\S]*?pub feature_low: Option<u16>[\s\S]*?pub feature_high: Option<u16>/,
  "2D Mapping Feature ranges must remain additive for legacy project and DVC-derived JSON",
);
assert.match(
  engineSource,
  /feature_range[\s\S]*?scale_effect_u16_directed/,
  "the engine must scale 2D Mapping luminance into the authored Feature range",
);

console.log("T19 effect family, LFO, palette, Move, Value, independent Curve, legacy fixture-order Mapping, and unified 2D Mapping visualization contracts ok");
