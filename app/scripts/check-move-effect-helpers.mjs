import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const source = await readFile(new URL("../src/moveEffectDrag.ts", import.meta.url), "utf8");
const transpiled = ts.transpileModule(source, {
  compilerOptions: {
    module: ts.ModuleKind.ESNext,
    target: ts.ScriptTarget.ES2022,
    importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
  },
  fileName: "moveEffectDrag.ts",
});
const drag = await import(`data:text/javascript;base64,${Buffer.from(transpiled.outputText).toString("base64")}`);

assert.equal(drag.moveEffectDragThresholdPx, 4, "Move point drag threshold must remain exactly 4px");

const start = drag.beginMoveEffectPointDrag({
  index: 1,
  pointerId: 42,
  clientX: 100,
  clientY: 200,
  surfaceWidth: 200,
  surfaceHeight: 100,
  point: { x: 0.25, y: 0.75 },
});
assert.deepEqual(start.originalPoint, { x: 0.25, y: 0.75 });
assert.deepEqual(start.draftPoint, { x: 0.25, y: 0.75 });
assert.equal(start.moved, false);

const letterboxedSurface = drag.moveEffectDragSurfaceSizeFromCtm(
  { a: 3.125, b: 0, c: 0, d: 3.125 },
  484,
  312.5,
);
assert.deepEqual(
  letterboxedSurface,
  { width: 312.5, height: 312.5 },
  "the Move drag surface must use the SVG screen CTM instead of the letterboxed element width",
);
const ctmMoved = drag.updateMoveEffectPointDrag(
  drag.beginMoveEffectPointDrag({
    index: 0,
    pointerId: 9,
    clientX: 200,
    clientY: 200,
    surfaceWidth: letterboxedSurface.width,
    surfaceHeight: letterboxedSurface.height,
    point: { x: 0.5, y: 0.5 },
  }),
  { clientX: 206, clientY: 194 },
);
assert.deepEqual(
  ctmMoved.draftPoint,
  { x: 0.5192, y: 0.5192 },
  "a rendered 6px delta must map through the square viewBox CTM on both axes",
);

const atThreshold = drag.updateMoveEffectPointDrag(start, { clientX: 104, clientY: 200 });
assert.equal(atThreshold.moved, false, "exactly 4px remains a click instead of starting a drag");
assert.deepEqual(atThreshold.draftPoint, start.originalPoint, "sub-threshold motion must not alter the ghost point");

const diagonalPastThreshold = drag.updateMoveEffectPointDrag(start, { clientX: 103, clientY: 203 });
assert.equal(diagonalPastThreshold.moved, true, "the 4px threshold must use radial pointer distance");

const moved = drag.updateMoveEffectPointDrag(start, { clientX: 120, clientY: 210 });
assert.equal(moved.moved, true);
assert.deepEqual(
  moved.draftPoint,
  { x: 0.35, y: 0.65 },
  "pointer deltas must be relative to the drag origin and Y must invert into protocol coordinates",
);

const movedAgain = drag.updateMoveEffectPointDrag(moved, { clientX: 110, clientY: 190 });
assert.deepEqual(
  movedAgain.draftPoint,
  { x: 0.3, y: 0.85 },
  "each update must remain relative to the original point instead of accumulating deltas",
);

const clamped = drag.updateMoveEffectPointDrag(start, { clientX: 1_000, clientY: 1_000 });
assert.deepEqual(clamped.draftPoint, { x: 1, y: 0 }, "dragged points must clamp to the XY pad bounds");
const clampedOpposite = drag.updateMoveEffectPointDrag(start, { clientX: -1_000, clientY: -1_000 });
assert.deepEqual(clampedOpposite.draftPoint, { x: 0, y: 1 }, "both clamp extremes must be enforced");

const authoredPoints = [
  { x: 0.1, y: 0.2, label: "first" },
  { x: 0.25, y: 0.75, label: "dragged" },
  { x: 0.9, y: 0.8, label: "last" },
];
const authoredBefore = structuredClone(authoredPoints);
assert.equal(
  drag.commitMoveEffectPointDrag(authoredPoints, atThreshold),
  null,
  "a click below the threshold must not produce an authored mutation",
);
const committed = drag.commitMoveEffectPointDrag(authoredPoints, moved);
assert.deepEqual(
  committed,
  [
    { x: 0.1, y: 0.2, label: "first" },
    { x: 0.35, y: 0.65, label: "dragged" },
    { x: 0.9, y: 0.8, label: "last" },
  ],
  "commit must replace only the dragged XY coordinates and preserve point metadata",
);
assert.deepEqual(authoredPoints, authoredBefore, "commit must not mutate the saved point array in place");
assert.notEqual(committed, authoredPoints, "commit must return a new point array");
assert.notEqual(committed[0], authoredPoints[0], "commit must clone non-dragged points as well");
assert.equal(
  drag.commitMoveEffectPointDrag(authoredPoints, { ...moved, index: 99 }),
  null,
  "an invalid point index must never produce a partial commit",
);

const zeroSurface = drag.updateMoveEffectPointDrag(
  drag.beginMoveEffectPointDrag({
    index: 0,
    pointerId: 7,
    clientX: 0,
    clientY: 0,
    surfaceWidth: 0,
    surfaceHeight: 0,
    point: { x: 0.4, y: 0.6 },
  }),
  { clientX: 20, clientY: 20 },
);
assert.deepEqual(zeroSurface.draftPoint, { x: 0.4, y: 0.6 }, "a zero-size pad must not corrupt saved values");

console.log("T16 Move 4px threshold, delta, Y inversion, clamp, and commit contracts ok");
