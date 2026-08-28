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

    if (expectedSegments > 1) {
      assert.deepEqual(
        visuals.mappingFixtureSegmentCell(grid, 0, order),
        { column: 0, row: rows - 1 },
        `${footprint}ch segment 1 starts at the bottom-left`,
      );
      assert.deepEqual(
        visuals.mappingFixtureSegmentCell(grid, rows - 1, order),
        { column: 0, row: 0 },
        `${footprint}ch left column runs bottom-to-top`,
      );
      assert.deepEqual(
        visuals.mappingFixtureSegmentCell(grid, rows, order),
        { column: 1, row: rows - 1 },
        `${footprint}ch next segment restarts at the next column bottom`,
      );
      assert.deepEqual(
        visuals.mappingFixtureSegmentCell(grid, expectedSegments - 1, order),
        { column: columns - 1, row: 0 },
        `${footprint}ch final segment ends at the top-right`,
      );
    }
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
  dvc13.stage_layout = {
    version: 1,
    cell_width: 5,
    cell_depth: 5,
    cells: Array.from({ length: 4 }, (_, beamIndex) => ({
      beam_index: beamIndex,
      offset_x: beamIndex * 5,
      offset_z: 0,
      logical_segment_index: beamIndex,
    })),
    logical_segments: Array.from({ length: 4 }, (_, logicalIndex) => ({
      logical_index: logicalIndex,
      color_controls: [
        { role: "Red", control_index: 1 + logicalIndex * 3 },
        { role: "Green", control_index: 2 + logicalIndex * 3 },
        { role: "Blue", control_index: 3 + logicalIndex * 3 },
      ],
    })),
    global_dimmer_control_index: 0,
    global_strobe_control_index: null,
  };
  const exactRoleLevels = [
    65_535,
    65_535, 0, 0,
    0, 65_535, 0,
    0, 0, 65_535,
    65_535, 65_535, 0,
  ];
  dvc13.controls = dvc13.controls.map((control, index) => ({
    ...control,
    attribute: `Opaque control ${index}`,
    channel_name: `Opaque channel ${index}`,
  }));
  dvc13.attribute_values = exactRoleLevels.map((value, index) => ({
    attribute: `Opaque control ${index}`,
    value,
  }));
  const dvc13Live = liveColor.fixtureLiveColor(dvc13, new Map());
  assert.equal(dvc13Live.segmentCount, 4, "DVC-style unsuffixed first RGB must be segment 1");
  assert.deepEqual(
    dvc13Live.segments.map((segment) => segment.color),
    ["rgb(255, 0, 0)", "rgb(0, 255, 0)", "rgb(0, 0, 255)", "rgb(255, 255, 0)"],
    "persisted color roles must drive cells without re-inferring opaque control names",
  );

  const glyphSource = await readFile(resolve(appRoot, "src/components/StageGlyphs.tsx"), "utf8");
  const renderSource = await readFile(resolve(appRoot, "src/createMappingRenderModel.ts"), "utf8");
  assert.match(glyphSource, /data-stage-fixture-segment-column/);
  assert.match(glyphSource, /data-stage-fixture-segment-row/);
  assert.match(glyphSource, /data-stage-physical-beam-index/);
  assert.match(glyphSource, /data-stage-physical-cell-space="fixture-local"/);
  assert.match(glyphSource, /mappingFixtureSegmentCell/);
  assert.match(renderSource, /mappingFixtureSegmentCell/);
  assert.match(renderSource, /localZ/);
  assert.match(renderSource, /rotateStageOffsetYaw\(\{ x: localX, z: localZ \}/);
  assert.match(renderSource, /fixture\.stage_layout\?\.cells/);
  assert.match(renderSource, /cell\.offset_x \* fixtureWorldToSvgScale/);

  console.log(
    "pass strongpoint segment layout " +
      "13ch=4x1 25ch=4x2 61ch=4x5 121ch=4x10 " +
      "order=left-columns-bottom-to-top " +
      `dvc13=${JSON.stringify(dvc13Live.segments.map((segment) => segment.color))}`,
  );
} finally {
  await server.close();
}
