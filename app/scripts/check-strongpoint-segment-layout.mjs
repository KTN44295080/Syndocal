import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";

const scriptsDir = dirname(fileURLToPath(import.meta.url));
const appRoot = resolve(scriptsDir, "..");

const server = await createServer({
  root: appRoot,
  configFile: false,
  logLevel: "error",
  appType: "custom",
  optimizeDeps: { noDiscovery: true },
  server: { middlewareMode: true },
});

const controlFromSpec = (spec) => {
  const [attribute, addressSpec] = spec.split("@");
  const [offsetText, resolutionText] = addressSpec.split(":");
  const offset = Number(offsetText);
  const resolution = resolutionText === "16" ? "SixteenBit" : "EightBit";
  return {
    attribute,
    channel_name: attribute,
    offsets: resolution === "SixteenBit" ? [offset, offset + 1] : [offset],
    resolution,
    default_value: 0,
    functions: [],
  };
};

const fixtureFromRequest = (request, id) => ({
  id,
  label: request.name,
  profile_source_path: `verified://${id}`,
  profile_name: request.name,
  manufacturer: request.manufacturer,
  mode_name: request.mode_name,
  universe: 0,
  address: 1,
  group_ids: [],
  position: { x: 0, y: 0, z: 0 },
  rotation: { pitch: 0, yaw: 0, roll: 0 },
  geometries: [],
  controls: request.attributes.map(controlFromSpec),
  attribute_values: [],
  limits: {
    dimmer_min: 0,
    dimmer_max: 1,
    pan_min: -270,
    pan_max: 270,
    tilt_min: -135,
    tilt_max: 135,
    invert_pan: false,
    invert_tilt: false,
    swap_pan_tilt: false,
  },
  highlighted: false,
  soloed: false,
  parked: false,
});

try {
  const catalog = await server.ssrLoadModule("/src/fixtureCatalog.ts");
  const liveColor = await server.ssrLoadModule("/src/fixtureLiveColor.ts");
  const visuals = await server.ssrLoadModule("/src/fixtureVisuals.ts");

  const modes = [
    [3, 1, 1, 1],
    [4, 1, 1, 1],
    [8, 1, 1, 1],
    [12, 4, 4, 1],
    [13, 4, 4, 1],
    [24, 8, 4, 2],
    [25, 8, 4, 2],
    [60, 20, 4, 5],
    [61, 20, 4, 5],
    [120, 40, 4, 10],
    [121, 40, 4, 10],
  ];

  for (const [footprint, expectedSegments, columns, rows] of modes) {
    const request = catalog.verifiedFixtureProfileRequest(
      `personal-960-sound-waves-strongpoint-${footprint}ch`,
    );
    assert.ok(request, `missing strongpoint ${footprint}ch profile`);
    const fixture = fixtureFromRequest(request, footprint);
    const skeleton = liveColor.fixtureLiveSegmentSkeleton(fixture);
    const grid = visuals.mappingFixtureSegmentGrid(fixture, skeleton.length);
    const order = visuals.mappingFixtureSegmentOrder(fixture);
    const size = visuals.mappingFixtureStageSize(
      visuals.fixtureVisualKind(fixture),
      grid.columns,
      1,
      grid.rows,
    );
    assert.equal(skeleton.length, expectedSegments, `${footprint}ch segment count`);
    assert.deepEqual(grid, { columns, rows }, `${footprint}ch control grid`);
    assert.equal(order, "column-major-bottom-left", `${footprint}ch physical segment order`);
    assert.deepEqual(size, { width: columns * 5, height: rows * 5 }, `${footprint}ch stage size`);
    assert.equal(visuals.fixtureVisualKind(fixture), "panel", `${footprint}ch visual kind`);
  }

  const strongpointOrder = "column-major-bottom-left";
  assert.deepEqual(
    visuals.mappingFixtureSegmentCell({ columns: 4, rows: 10 }, 0, strongpointOrder),
    { column: 0, row: 9 },
    "segment 1 is the bottom cell of the leftmost column",
  );
  assert.deepEqual(
    visuals.mappingFixtureSegmentCell({ columns: 4, rows: 10 }, 9, strongpointOrder),
    { column: 0, row: 0 },
    "segment 10 is the top cell of the leftmost column",
  );
  assert.deepEqual(
    visuals.mappingFixtureSegmentCell({ columns: 4, rows: 10 }, 10, strongpointOrder),
    { column: 1, row: 9 },
    "segment 11 restarts at the bottom of the second column",
  );
  assert.deepEqual(
    visuals.mappingFixtureSegmentCell({ columns: 4, rows: 10 }, 39, strongpointOrder),
    { column: 3, row: 0 },
    "segment 40 is the top cell of the rightmost column",
  );

  const request13 = catalog.verifiedFixtureProfileRequest(
    "personal-960-sound-waves-strongpoint-13ch",
  );
  const dvc13 = fixtureFromRequest(request13, 1300);
  dvc13.mode_name = "Daslight Mode 0";
  dvc13.controls = dvc13.controls.map((control) => {
    const renamed = control.attribute
      .replace("ColorAdd_R", "ColorRed ")
      .replace("ColorAdd_G", "ColorGreen ")
      .replace("ColorAdd_B", "ColorBlue ")
      .trim();
    return { ...control, attribute: renamed, channel_name: renamed };
  });
  dvc13.attribute_values = [
    ["Dimmer", 65_535],
    ["ColorRed", 65_535],
    ["ColorGreen", 0],
    ["ColorBlue", 0],
    ["ColorRed 2", 0],
    ["ColorGreen 2", 65_535],
    ["ColorBlue 2", 0],
    ["ColorRed 3", 0],
    ["ColorGreen 3", 0],
    ["ColorBlue 3", 65_535],
    ["ColorRed 4", 65_535],
    ["ColorGreen 4", 65_535],
    ["ColorBlue 4", 0],
  ].map(([attribute, value]) => ({ attribute, value }));
  const dvc13Live = liveColor.fixtureLiveColor(dvc13, new Map());
  assert.equal(dvc13Live.segmentCount, 4, "DVC-style unsuffixed first RGB must be segment 1");
  assert.deepEqual(
    dvc13Live.segments.map((segment) => segment.color),
    ["rgb(255, 0, 0)", "rgb(0, 255, 0)", "rgb(0, 0, 255)", "rgb(255, 255, 0)"],
    "segment 1 RGB must not leak into later cells",
  );

  const glyphSource = await readFile(resolve(appRoot, "src/components/StageGlyphs.tsx"), "utf8");
  const renderSource = await readFile(resolve(appRoot, "src/createMappingRenderModel.ts"), "utf8");
  assert.match(glyphSource, /data-stage-fixture-segment-column/);
  assert.match(glyphSource, /data-stage-fixture-segment-row/);
  assert.match(glyphSource, /mappingFixtureSegmentCell/);
  assert.match(renderSource, /mappingFixtureSegmentCell/);
  assert.match(renderSource, /localZ/);
  assert.match(renderSource, /rotateStageOffsetYaw\(\{ x: localX, z: localZ \}/);

  console.log(
    "pass strongpoint segment layout " +
      "13ch=4x1 25ch=4x2 61ch=4x5 121ch=4x10 " +
      "order=left-columns-bottom-to-top " +
      `dvc13=${JSON.stringify(dvc13Live.segments.map((segment) => segment.color))}`,
  );
} finally {
  await server.close();
}
