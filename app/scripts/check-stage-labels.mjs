import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const source = await readFile(new URL("../src/stageLabelLayout.ts", import.meta.url), "utf8");
const geometryLayerSource = await readFile(
  new URL("../src/components/MappingGeometryLayer.tsx", import.meta.url),
  "utf8",
);
const mappingFixturesLayerSource = await readFile(
  new URL("../src/components/MappingFixturesLayer.tsx", import.meta.url),
  "utf8",
);
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
  showLabels: true,
  pickedFixtureId: 100,
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
  showLabels: true,
  pickedFixtureId: 101,
}).labels[0];
assert.equal(multiCellLabel.x, 41.25, "an eight-cell glyph label must anchor beyond the complete 40-unit footprint");
assert.equal(multiCellLabel.z, 16.25, "multi-cell labels keep the same one-cell vertical clearance");

assert.equal(
  winningFixtureId([fixture(1), fixture(2)], { pickedFixtureId: 2, hoveredFixtureId: 1 }),
  1,
  "the current hover label must temporarily outrank an overlapping picked label",
);
assert.equal(
  winningFixtureId([fixture(1, { highlighted: true }), fixture(2)], { hoveredFixtureId: 2 }),
  2,
  "hovered labels must render while a highlighted-only fixture remains unlabeled",
);
assert.equal(
  winningFixtureId(
    [fixture(1, { addressOrder: 99 }), fixture(2, { addressOrder: 1 })],
    { pickedFixtureIds: new Set([1, 2]) },
  ),
  2,
  "lower DMX address order must win when two picked labels overlap",
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
  showLabels: true,
  pickedFixtureId: 41,
  hoveredFixtureId: 40,
});
assert.equal(denseResult.visibleFixtureCount, 41);
assert.equal(denseResult.candidateCount, 2, "only picked and hovered fixtures may become label candidates");
assert.equal(denseResult.labelByFixtureId.has(40), true, "the current hover fixture must keep a label");
assert.equal(denseResult.labelByFixtureId.has(41), false, "an overlapping picked label must yield while another fixture is hovered");
assert.equal(denseResult.labelByFixtureId.has(39), false, "highlight alone must not expose a fixture label");
assert.equal(denseResult.labelByFixtureId.has(1), false, "an unflagged dense fixture must not receive a label");
assertNoLabelOverlaps(denseResult.labels, "the 41-fixture dense layout must have zero label rectangle overlaps");

const spacedFixtures = Array.from({ length: 41 }, (_, index) =>
  fixture(index + 1, {
    label: "F",
    x: (index % 9) * 11,
    z: 10 + Math.floor(index / 9) * 15,
  }),
);
const zoomedUnpickedResult = layout.planStageFixtureLabels({
  fixtures: spacedFixtures,
  viewport,
  showLabels: true,
});
assert.equal(zoomedUnpickedResult.candidateCount, 0, "zoom must never expose unpicked and unhovered fixture labels");
assert.equal(zoomedUnpickedResult.labels.length, 0, "unpicked and unhovered fixtures stay unlabeled at every zoom");

const allPickedResult = layout.planStageFixtureLabels({
  fixtures: spacedFixtures,
  viewport,
  showLabels: true,
  pickedFixtureIds: new Set(spacedFixtures.map((candidate) => candidate.id)),
});
assert.equal(allPickedResult.candidateCount, 41, "every explicitly picked fixture must become a label candidate");
assert.equal(allPickedResult.labels.length, 41, "non-overlapping picked fixtures must render all labels");
assertNoLabelOverlaps(allPickedResult.labels, "the picked 41-fixture layout must have zero label rectangle overlaps");

const labelsOffResult = layout.planStageFixtureLabels({
  fixtures: denseFixtures,
  viewport,
  showLabels: false,
  pickedFixtureId: 41,
  hoveredFixtureId: 40,
});
assert.equal(labelsOffResult.labels.length, 0, "the explicit labels toggle must override zoom and priority states");

const globalOnlySelectedResult = layout.planStageFixtureLabels({
  fixtures: [fixture(42)],
  viewport,
  showLabels: true,
  pickedFixtureId: 42,
});
assert.equal(
  globalOnlySelectedResult.labels.length,
  1,
  "the generic planner still supports a single selected fixture for non-Mapping consumers",
);
assert.match(
  mappingFixturesLayerSource,
  /pickedFixtureIds: props\.selectedFixtureIds/,
  "Mapping labels must use the mapping-owned selected set",
);
assert.doesNotMatch(
  mappingFixturesLayerSource,
  /pickedFixtureId:\s*props\.selectedFixtureId/,
  "Mapping labels must not promote the global selected fixture by itself",
);

assert.match(
  geometryLayerSource,
  /<Show when=\{props\.showLabels && geometry\.selected\}>/,
  "geometry labels must require both the labels toggle and an explicitly selected fixture",
);
assert.doesNotMatch(
  geometryLayerSource,
  /<Show when=\{props\.showLabels\}>/,
  "geometry labels must not render for every fixture when labels are enabled",
);

console.log("T24-A stage label footprint anchoring, shortening, selected-or-hovered eligibility, priority, and overlap contracts ok");
