import { readFile } from "node:fs/promises";
import assert from "node:assert/strict";
import ts from "typescript";

const helperUrl = new URL("../src/timelineAutomationHelpers.ts", import.meta.url);
const helperSource = await readFile(helperUrl, "utf8");
const transpiled = ts.transpileModule(helperSource, {
  compilerOptions: {
    module: ts.ModuleKind.ESNext,
    target: ts.ScriptTarget.ES2022,
    importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
  },
  fileName: "timelineAutomationHelpers.ts",
});

const moduleUrl = `data:text/javascript;base64,${Buffer.from(transpiled.outputText).toString("base64")}`;
const helpers = await import(moduleUrl);

const linearKeys = [
  { time_ms: 0, value: 0, interpolation: "Linear" },
  { time_ms: 1000, value: 100, interpolation: "Step" },
];
assert.equal(helpers.evaluateTimelineKeyframes(linearKeys, 500), 50);

const stepKeys = [
  { time_ms: 0, value: 10, interpolation: "Step" },
  { time_ms: 1000, value: 100, interpolation: "Step" },
];
assert.equal(helpers.evaluateTimelineKeyframes(stepKeys, 500), 10);

const bezierKeys = [
  { time_ms: 0, value: 0, interpolation: "Bezier" },
  { time_ms: 1000, value: 100, interpolation: "Step" },
];
assert.equal(helpers.evaluateTimelineKeyframes(bezierKeys, 500), 50);
assert.equal(helpers.evaluateTimelineKeyframes(bezierKeys, 250), 15.625);

assert.deepEqual(
  helpers.shiftedTimelineKeyframes(
    [
      { time_ms: 100, value: 1 },
      { time_ms: 300, value: 2 },
    ],
    200,
  ),
  [
    { time_ms: 200, value: 1 },
    { time_ms: 400, value: 2 },
  ],
);

assert.deepEqual(
  helpers.resizedTimelineKeyframes(
    [
      { time_ms: 100, value: 1 },
      { time_ms: 200, value: 2 },
      { time_ms: 300, value: 3 },
    ],
    0,
    1000,
  ),
  [
    { time_ms: 0, value: 1 },
    { time_ms: 500, value: 2 },
    { time_ms: 1000, value: 3 },
  ],
);

assert.deepEqual(
  helpers.upsertTimelineKeyframe(
    [
      { time_ms: 0, value: 0 },
      { time_ms: 100, value: 1 },
    ],
    { time_ms: 100, value: 2 },
  ),
  [
    { time_ms: 0, value: 0 },
    { time_ms: 100, value: 2 },
  ],
);

assert.equal(
  helpers.timelineKeyframeMoveTime(
    [
      { time_ms: 0 },
      { time_ms: 500 },
      { time_ms: 1000 },
    ],
    1,
    1200,
  ),
  999,
);

assert.deepEqual(
  helpers.movedTimelineKeyframe(
    [
      { time_ms: 0, value: "a" },
      { time_ms: 500, value: "b" },
      { time_ms: 1000, value: "c" },
    ],
    1,
    250,
  ),
  [
    { time_ms: 0, value: "a" },
    { time_ms: 250, value: "b" },
    { time_ms: 1000, value: "c" },
  ],
);

assert.deepEqual(
  helpers.snappedTimelineKeyframes(
    [
      { time_ms: 10, value: "a" },
      { time_ms: 80, value: "b" },
      { time_ms: 140, value: "c" },
    ],
    (timeMs) => Math.round(timeMs / 100) * 100,
  ),
  [
    { time_ms: 0, value: "a" },
    { time_ms: 100, value: "b" },
    { time_ms: 101, value: "c" },
  ],
);

assert.deepEqual(
  helpers.keyframesWithoutPlayheadKeyframe(
    [
      { time_ms: 0, value: "a" },
      { time_ms: 500, value: "b" },
      { time_ms: 1000, value: "c" },
    ],
    505,
    10,
  ),
  {
    keyframes: [
      { time_ms: 0, value: "a" },
      { time_ms: 1000, value: "c" },
    ],
    keyframeIndex: 1,
    timeMs: 500,
  },
);

assert.deepEqual(
  helpers.keyframesWithoutIndex(
    [
      { time_ms: 0, value: "a" },
      { time_ms: 500, value: "b" },
      { time_ms: 1000, value: "c" },
    ],
    2,
  ),
  {
    keyframes: [
      { time_ms: 0, value: "a" },
      { time_ms: 500, value: "b" },
    ],
    keyframeIndex: 2,
    timeMs: 1000,
  },
);

assert.equal(helpers.keyframesWithoutIndex([{ time_ms: 0 }, { time_ms: 1000 }], 0), null);

assert.deepEqual(
  helpers.keyframesWithInterpolationAtIndex(
    [
      { time_ms: 0, value: 0, interpolation: "Linear" },
      { time_ms: 1000, value: 100, interpolation: "Step" },
    ],
    0,
    "Bezier",
  ),
  [
    { time_ms: 0, value: 0, interpolation: "Bezier" },
    { time_ms: 1000, value: 100, interpolation: "Step" },
  ],
);

assert.equal(
  helpers.keyframesWithInterpolationAtIndex([{ time_ms: 0, value: 0, interpolation: "Linear" }], 4, "Step"),
  null,
);

assert.deepEqual(
  helpers.keyframesWithValueAtIndex(
    [
      { time_ms: 0, value: 0, interpolation: "Linear" },
      { time_ms: 1000, value: 100, interpolation: "Step" },
    ],
    1,
    64,
  ),
  [
    { time_ms: 0, value: 0, interpolation: "Linear" },
    { time_ms: 1000, value: 64, interpolation: "Step" },
  ],
);

assert.equal(
  helpers.keyframesWithValueAtIndex([{ time_ms: 0, value: 0, interpolation: "Linear" }], 0, Number.NaN),
  null,
);

assert.equal(
  helpers.keyframesWithoutPlayheadKeyframe(
    [
      { time_ms: 0, value: "a" },
      { time_ms: 1000, value: "b" },
    ],
    0,
    10,
  ),
  null,
);

assert.deepEqual(
  helpers.keyframesWithDraftEndpoints(
    [
      { time_ms: 100, value: 10, interpolation: "Linear" },
      { time_ms: 200, value: 20, interpolation: "Linear" },
      { time_ms: 300, value: 30, interpolation: "Step" },
    ],
    0,
    1000,
    1,
    99,
    "Bezier",
  ),
  [
    { time_ms: 0, value: 1, interpolation: "Bezier" },
    { time_ms: 500, value: 20, interpolation: "Linear" },
    { time_ms: 1000, value: 99, interpolation: "Step" },
  ],
);

const state = {
  opacity: 0.42,
  speed: 1.5,
  position_ms: 1234,
  bpm_sync: { enabled: true, ratio: 0.5, loop_bars: 2 },
  transform: {
    x: 10,
    y: 20,
    scale_x: 1.1,
    scale_y: 1.2,
    rotation_deg: 45,
    crop_left: 0.1,
    crop_top: 0.2,
    crop_right: 0.3,
    crop_bottom: 0.4,
  },
  color: {
    brightness: 1.1,
    contrast: 1.2,
    hue_deg: 180,
    saturation: 0.8,
    gamma: 2.2,
  },
  fx: {
    pixelate: 3,
    blur: 4,
    glow: 5,
    edge: 6,
    key_red: 0.1,
    key_green: 0.2,
    key_blue: 0.3,
    key_threshold: 0.4,
  },
};

assert.equal(helpers.videoAutomationValueFromState(state, "Opacity"), 0.42);
const videoParamExpectations = [
  ["Speed", 1.5],
  ["PositionMs", 1234],
  ["BpmSyncEnabled", 1],
  ["BpmSyncRatio", 0.5],
  ["BpmSyncLoopBars", 2],
  ["TransformX", 10],
  ["TransformY", 20],
  ["TransformScaleX", 1.1],
  ["TransformScaleY", 1.2],
  ["TransformRotationDeg", 45],
  ["TransformCropLeft", 0.1],
  ["TransformCropTop", 0.2],
  ["TransformCropRight", 0.3],
  ["TransformCropBottom", 0.4],
  ["ColorBrightness", 1.1],
  ["ColorContrast", 1.2],
  ["ColorHueDeg", 180],
  ["ColorSaturation", 0.8],
  ["ColorGamma", 2.2],
  ["FxPixelate", 3],
  ["FxBlur", 4],
  ["FxGlow", 5],
  ["FxEdge", 6],
  ["FxKeyRed", 0.1],
  ["FxKeyGreen", 0.2],
  ["FxKeyBlue", 0.3],
  ["FxKeyThreshold", 0.4],
];
for (const [param, expected] of videoParamExpectations) {
  assert.equal(helpers.videoAutomationValueFromState(state, param), expected, param);
}

console.log("timeline automation helpers ok");
