import assert from "node:assert/strict";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";

const scriptsDir = dirname(fileURLToPath(import.meta.url));
const appRoot = resolve(scriptsDir, "..");
const dark = "rgb(31, 38, 46)";

const control = (attribute, offset) => ({
  attribute,
  channel_name: attribute,
  offsets: [offset],
  resolution: "EightBit",
  default_value: 0,
  functions: [],
});

const fixture = {
  id: 1,
  label: "Live color source fixture",
  profile_source_path: "unit://fixture-live-color",
  profile_name: "Live color source fixture",
  manufacturer: "Syndocal QA",
  mode_name: "7CH two segment",
  universe: 0,
  address: 10,
  group_ids: [],
  position: { x: 0, y: 0, z: 0 },
  rotation: { pitch: 0, yaw: 0, roll: 0 },
  geometries: [],
  controls: [
    control("Dimmer", 1),
    control("Red1", 2),
    control("Green1", 3),
    control("Blue1", 4),
    control("Red2", 5),
    control("Green2", 6),
    control("Blue2", 7),
  ],
  attribute_values: [
    { attribute: "Dimmer", value: 32_768 },
    { attribute: "Red1", value: 65_535 },
    { attribute: "Green1", value: 0 },
    { attribute: "Blue1", value: 0 },
    { attribute: "Red2", value: 0 },
    { attribute: "Green2", value: 65_535 },
    { attribute: "Blue2", value: 0 },
  ],
  highlighted: false,
  soloed: false,
  parked: false,
};

const server = await createServer({
  root: appRoot,
  configFile: false,
  logLevel: "error",
  appType: "custom",
  optimizeDeps: { noDiscovery: true },
  server: { middlewareMode: true },
});

try {
  const {
    fixtureLiveColor,
    liveFixtureDarkColor,
  } = await server.ssrLoadModule("/src/fixtureLiveColor.ts");
  assert.equal(liveFixtureDarkColor, dark);

  const explicitZeroPreview = Array.from({ length: 512 }, () => 0);
  const previewZero = fixtureLiveColor(
    fixture,
    new Map([[fixture.universe, explicitZeroPreview]]),
  );
  assert.equal(previewZero.valueSource, "preview");
  assert.equal(previewZero.intensity, 0);
  assert.equal(previewZero.color, dark);
  assert.deepEqual(
    previewZero.segments.map((segment) => [segment.color, segment.intensity]),
    [[dark, 0], [dark, 0]],
  );

  const compactZero = fixtureLiveColor(
    fixture,
    new Map([[fixture.universe, []]]),
  );
  assert.equal(compactZero.valueSource, "preview");
  assert.equal(compactZero.intensity, 0);
  assert.deepEqual(compactZero.segments.map((segment) => segment.color), [dark, dark]);

  const livePreview = Array.from({ length: 512 }, () => 0);
  livePreview[fixture.address - 1] = 128;
  livePreview[fixture.address + 2] = 255;
  livePreview[fixture.address + 3] = 255;
  const previewNonzero = fixtureLiveColor(
    fixture,
    new Map([[fixture.universe, livePreview]]),
  );
  assert.equal(previewNonzero.valueSource, "preview");
  assert.deepEqual(
    previewNonzero.segments.map((segment) => segment.color),
    ["rgb(0, 0, 128)", "rgb(128, 0, 0)"],
  );

  const previewAbsent = fixtureLiveColor(fixture, new Map());
  assert.equal(previewAbsent.valueSource, "attribute");
  assert.deepEqual(
    previewAbsent.segments.map((segment) => segment.color),
    ["rgb(128, 0, 0)", "rgb(0, 128, 0)"],
  );

  console.log(
    "pass fixture live-color source selection " +
      `previewZero=${previewZero.valueSource}:${previewZero.intensity}x${previewZero.segmentCount} ` +
      `previewNonzero=${previewNonzero.valueSource}:${JSON.stringify(previewNonzero.segments.map((segment) => segment.color))} ` +
      `previewAbsent=${previewAbsent.valueSource}:${JSON.stringify(previewAbsent.segments.map((segment) => segment.color))} ` +
      `compactZero=${compactZero.valueSource}:${compactZero.intensity}x${compactZero.segmentCount}`,
  );
} finally {
  await server.close();
}
