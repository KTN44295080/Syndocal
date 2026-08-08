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

const strongpoint13Fixture = {
  ...fixture,
  id: 13,
  label: "960 sound waves strongpoint 13ch",
  profile_name: "960 sound waves strongpoint",
  mode_name: "13-channel · master + 4 blocks",
  address: 30,
  controls: [
    control("Dimmer", 1),
    control("ColorAdd_R", 2),
    control("ColorAdd_G", 3),
    control("ColorAdd_B", 4),
    control("ColorAdd_R2", 5),
    control("ColorAdd_G2", 6),
    control("ColorAdd_B2", 7),
    control("ColorAdd_R3", 8),
    control("ColorAdd_G3", 9),
    control("ColorAdd_B3", 10),
    control("ColorAdd_R4", 11),
    control("ColorAdd_G4", 12),
    control("ColorAdd_B4", 13),
  ],
  attribute_values: [
    { attribute: "Dimmer", value: 65_535 },
    { attribute: "ColorAdd_R", value: 65_535 },
    { attribute: "ColorAdd_G", value: 0 },
    { attribute: "ColorAdd_B", value: 0 },
    { attribute: "ColorAdd_R2", value: 0 },
    { attribute: "ColorAdd_G2", value: 65_535 },
    { attribute: "ColorAdd_B2", value: 0 },
    { attribute: "ColorAdd_R3", value: 0 },
    { attribute: "ColorAdd_G3", value: 0 },
    { attribute: "ColorAdd_B3", value: 65_535 },
    { attribute: "ColorAdd_R4", value: 65_535 },
    { attribute: "ColorAdd_G4", value: 65_535 },
    { attribute: "ColorAdd_B4", value: 0 },
  ],
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
  const {
    colorCandidates,
    findControlAttributeInControls,
  } = await server.ssrLoadModule("/src/fixtureControlRuntime.ts");
  assert.equal(liveFixtureDarkColor, dark);

  const gdtfAdditiveControls = [
    control("ColorAdd_R", 1),
    control("ColorAdd_G", 2),
    control("ColorAdd_B", 3),
    control("ColorAdd_WW", 4),
    control("ColorAdd_CW", 5),
    control("ColorAdd_A", 6),
    control("ColorAdd_UV", 7),
    control("Generic: UV", 8),
  ];
  const additiveResolution = {
    red: findControlAttributeInControls(gdtfAdditiveControls, colorCandidates.red),
    green: findControlAttributeInControls(gdtfAdditiveControls, colorCandidates.green),
    blue: findControlAttributeInControls(gdtfAdditiveControls, colorCandidates.blue),
    white: findControlAttributeInControls(gdtfAdditiveControls, colorCandidates.white),
    amber: findControlAttributeInControls(gdtfAdditiveControls, colorCandidates.amber),
    uv: findControlAttributeInControls(gdtfAdditiveControls, colorCandidates.uv),
  };
  assert.deepEqual(additiveResolution, {
    red: "ColorAdd_R",
    green: "ColorAdd_G",
    blue: "ColorAdd_B",
    white: "ColorAdd_WW",
    amber: "ColorAdd_A",
    uv: "ColorAdd_UV",
  });
  assert.ok(colorCandidates.white.includes("ColorAdd_CW"));
  assert.ok(colorCandidates.uv.includes("Generic: UV"));

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

  const strongpoint13 = fixtureLiveColor(strongpoint13Fixture, new Map());
  assert.equal(strongpoint13.segmentCount, 4);
  assert.deepEqual(
    strongpoint13.segments.map((segment) => segment.color),
    ["rgb(255, 0, 0)", "rgb(0, 255, 0)", "rgb(0, 0, 255)", "rgb(255, 255, 0)"],
  );

  console.log(
    "pass fixture live-color source selection " +
      `previewZero=${previewZero.valueSource}:${previewZero.intensity}x${previewZero.segmentCount} ` +
      `previewNonzero=${previewNonzero.valueSource}:${JSON.stringify(previewNonzero.segments.map((segment) => segment.color))} ` +
      `previewAbsent=${previewAbsent.valueSource}:${JSON.stringify(previewAbsent.segments.map((segment) => segment.color))} ` +
      `strongpoint13=${strongpoint13.segmentCount}:${JSON.stringify(strongpoint13.segments.map((segment) => segment.color))} ` +
      `compactZero=${compactZero.valueSource}:${compactZero.intensity}x${compactZero.segmentCount} ` +
      `gdtfAdditive=${additiveResolution.red}/${additiveResolution.green}/${additiveResolution.blue} ` +
      `extras=${additiveResolution.white}/${additiveResolution.amber}/${additiveResolution.uv}`,
  );
} finally {
  await server.close();
}
