import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const source = await readFile(new URL("../src/stageLabelLayout.ts", import.meta.url), "utf8");
const transpiled = ts.transpileModule(source, {
  compilerOptions: {
    module: ts.ModuleKind.ESNext,
    target: ts.ScriptTarget.ES2022,
    importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
  },
  fileName: "stageLabelLayout.ts",
});
const layout = await import(
  `data:text/javascript;base64,${Buffer.from(transpiled.outputText).toString("base64")}`
);

const viewport = { x: 0, z: 0, width: 100, height: 100 };
const fixture = (
  id,
  {
    x = 20,
    z = 20,
    label = `Fixture profile ${id}`,
    addressOrder = id,
    highlighted = false,
  } = {},
) => ({
  id,
  label,
  x,
  z,
  width: 5,
  height: 5,
  addressOrder,
  highlighted,
});
const assertNoLabelOverlaps = (labels, message) => {
  for (let left = 0; left < labels.length; left += 1) {
    for (let right = left + 1; right < labels.length; right += 1) {
      assert.equal(
        layout.stageLabelRectsOverlap(labels[left].rect, labels[right].rect),
        false,
        `${message}: ${labels[left].fixtureId} overlaps ${labels[right].fixtureId}`,
      );
    }
  }
};
const winningFixtureId = (fixtures, options = {}) => {
  const result = layout.planStageFixtureLabels({
    fixtures,
    viewport,
    zoom: 1,
    showLabels: true,
    ...options,
  });
  assert.equal(result.labels.length, 1, "the priority fixture pair must collapse to one label");
  return result.labels[0].fixtureId;
};

assert.equal(layout.shortenStageFixtureLabel("123456789012"), "123456789012");
assert.equal(
  layout.shortenStageFixtureLabel("1234567890123"),
  "123456789012…",
  "labels longer than twelve Unicode characters must use twelve characters plus one ellipsis",
);
assert.equal(
  Array.from(layout.shortenStageFixtureLabel("ムービングライトプロファイル名")).length,
  13,
  "the twelve-character rule must count Unicode characters rather than bytes",
);

const singleCellLabel = layout.planStageFixtureLabels({
  fixtures: [fixture(100, { x: 20, z: 20, label: "Single" })],
  viewport,
  zoom: 1,
  showLabels: true,
}).labels[0];
assert.equal(singleCellLabel.x, 23.75, "a 5-unit single-cell glyph label must clear its right edge by 1.25 units");
assert.equal(singleCellLabel.z, 16.25, "a 5-unit single-cell glyph label must clear its top edge by 1.25 units");

const multiCellLabel = layout.planStageFixtureLabels({
  fixtures: [{
    ...fixture(101, { x: 20, z: 20, label: "Mega Bar" }),
    width: 40,
    height: 5,
  }],
  viewport,
  zoom: 1,
  showLabels: true,
}).labels[0];
assert.equal(multiCellLabel.x, 41.25, "an eight-cell glyph label must anchor beyond the complete 40-unit footprint");
assert.equal(multiCellLabel.z, 16.25, "multi-cell labels keep the same one-cell vertical clearance");

assert.equal(
  winningFixtureId([fixture(1), fixture(2)], { pickedFixtureId: 2, hoveredFixtureId: 1 }),
  2,
  "picked labels must outrank hovered labels",
);
assert.equal(
  winningFixtureId([fixture(1, { highlighted: true }), fixture(2)], { hoveredFixtureId: 2 }),
  2,
  "hovered labels must outrank highlighted labels",
);
assert.equal(
  winningFixtureId([fixture(1, { addressOrder: 1 }), fixture(2, { addressOrder: 99, highlighted: true })]),
  2,
  "highlighted labels must outrank unflagged labels",
);
assert.equal(
  winningFixtureId([fixture(1, { addressOrder: 99 }), fixture(2, { addressOrder: 1 })]),
  2,
  "lower DMX address order must win the unflagged tie",
);

const denseFixtures = Array.from({ length: 41 }, (_, index) =>
  fixture(index + 1, {
    x: 20 + (index % 3) * 0.4,
    z: 20 + (index % 4) * 0.4,
    highlighted: index === 38,
  }),
);
const denseResult = layout.planStageFixtureLabels({
  fixtures: denseFixtures,
  viewport,
  zoom: 1,
  showLabels: true,
  pickedFixtureId: 41,
  hoveredFixtureId: 40,
});
assert.equal(denseResult.visibleFixtureCount, 41);
assert.equal(denseResult.decluttered, true, "more than 25 visible fixtures must activate decluttering below 150% zoom");
assert.equal(denseResult.candidateCount, 3, "only picked, hovered, and highlighted labels may survive dense eligibility");
assert.equal(denseResult.labelByFixtureId.has(41), true, "the picked fixture must keep a label while decluttering");
assert.equal(denseResult.labelByFixtureId.has(1), false, "an unflagged dense fixture must not receive a label");
assertNoLabelOverlaps(denseResult.labels, "the 41-fixture dense layout must have zero label rectangle overlaps");

const spacedFixtures = Array.from({ length: 41 }, (_, index) =>
  fixture(index + 1, {
    label: "F",
    x: (index % 9) * 11,
    z: 10 + Math.floor(index / 9) * 15,
  }),
);
const restoredResult = layout.planStageFixtureLabels({
  fixtures: spacedFixtures,
  viewport,
  zoom: 1.5,
  showLabels: true,
});
assert.equal(restoredResult.decluttered, false, "150% zoom must disable density eligibility filtering");
assert.equal(restoredResult.candidateCount, 41, "150% zoom must restore every visible fixture as a label candidate");
assert.equal(restoredResult.labels.length, 41, "non-overlapping fixtures must render all labels again at 150% zoom");
assertNoLabelOverlaps(restoredResult.labels, "the restored 41-fixture layout must have zero label rectangle overlaps");

const labelsOffResult = layout.planStageFixtureLabels({
  fixtures: denseFixtures,
  viewport,
  zoom: 2,
  showLabels: false,
  pickedFixtureId: 41,
  hoveredFixtureId: 40,
});
assert.equal(labelsOffResult.labels.length, 0, "the explicit labels toggle must override zoom and priority states");

console.log("T24-A stage label footprint anchoring, shortening, density, priority, zoom restore, and 41-fixture overlap contracts ok");
