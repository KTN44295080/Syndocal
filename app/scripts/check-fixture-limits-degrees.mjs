import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const importTsModule = async (path, replacements = {}) => {
  const source = await readFile(new URL(path, import.meta.url), "utf8");
  const transpiled = ts.transpileModule(source, {
    compilerOptions: {
      module: ts.ModuleKind.ESNext,
      target: ts.ScriptTarget.ES2022,
      importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
    },
    fileName: path,
  });
  let output = transpiled.outputText;
  for (const [specifier, replacement] of Object.entries(replacements)) {
    output = output.replaceAll(`"${specifier}"`, JSON.stringify(replacement));
    output = output.replaceAll(`'${specifier}'`, JSON.stringify(replacement));
  }
  const url = `data:text/javascript;base64,${Buffer.from(output).toString("base64")}`;
  return { module: await import(url), url };
};

const numeric = await importTsModule("../src/numericHelpers.ts");
const limits = (await importTsModule("../src/fixtureLimits.ts", {
  "./numericHelpers": numeric.url,
})).module;

const movementFunction = (physicalFrom, physicalTo, dmxFrom = 0, dmxTo = 65_535) => ({
  name: "Movement",
  attribute: "Pan",
  dmx_from: dmxFrom,
  dmx_to: dmxTo,
  physical_from: physicalFrom,
  physical_to: physicalTo,
});

const fixtureWithRanges = (panRange, tiltRange = panRange) => ({
  controls: [
    { attribute: "Pan", functions: [movementFunction(...panRange)] },
    {
      attribute: "Tilt",
      functions: [
        {
          ...movementFunction(...tiltRange),
          attribute: "Tilt",
        },
      ],
    },
  ],
});

const fullSweep = fixtureWithRanges([-270, 270], [-135, 135]);
const panRange = limits.fixturePhysicalAxisRange(fullSweep, "pan");
const tiltRange = limits.fixturePhysicalAxisRange(fullSweep, "tilt");
assert.deepEqual(panRange, { from: -270, to: 270 }, "Pan must use the profile-authored 540 degree span");
assert.deepEqual(tiltRange, { from: -135, to: 135 }, "Tilt must use the profile-authored 270 degree span");
assert.equal(
  limits.formatFixturePhysicalDegrees(limits.fixturePhysicalDegreesForDmxValue(0, panRange)),
  "-270°",
);
assert.equal(
  limits.formatFixturePhysicalDegrees(limits.fixturePhysicalDegreesForDmxValue(32_768, panRange)),
  "0°",
);
assert.equal(
  limits.formatFixturePhysicalDegrees(limits.fixturePhysicalDegreesForDmxValue(65_535, tiltRange)),
  "135°",
);

const alternateRange = fixtureWithRanges([0, 360]);
const alternatePan = limits.fixturePhysicalAxisRange(alternateRange, "pan");
assert.deepEqual(alternatePan, { from: 0, to: 360 }, "alternate physical ranges must be preserved");
assert.equal(
  limits.formatFixturePhysicalDegrees(limits.fixturePhysicalDegreesForDmxValue(16_384, alternatePan)),
  "90°",
);

const rawLimits = {
  pan_min: 16_384,
  pan_max: 49_151,
  tilt_min: 8_192,
  tilt_max: 57_343,
  invert_pan: true,
  invert_tilt: true,
  swap_pan_tilt: true,
};
assert.equal(
  limits.formatFixturePhysicalDegrees(limits.fixturePhysicalDegreesForDmxValue(rawLimits.pan_min, panRange)),
  "-135°",
  "Pan min must be read from the raw persisted DMX limit");
assert.equal(
  limits.formatFixturePhysicalDegrees(limits.fixturePhysicalDegreesForDmxValue(rawLimits.pan_max, panRange)),
  "135°",
  "Pan max must be read from the raw persisted DMX limit");
assert.equal(
  limits.formatFixturePhysicalDegrees(limits.fixturePhysicalDegreesForDmxValue(rawLimits.tilt_min, tiltRange)),
  "-101.2°",
  "Tilt min must use the same degree conversion regardless of toggles");
assert.equal(
  limits.formatFixturePhysicalDegrees(limits.fixturePhysicalDegreesForDmxValue(rawLimits.tilt_max, tiltRange)),
  "101.2°",
  "Tilt max must use the same degree conversion regardless of toggles");

const missingRange = fixtureWithRanges([null, null]);
assert.equal(limits.fixturePhysicalAxisRange(missingRange, "pan"), null, "missing physical range must fail closed");
assert.equal(
  limits.formatFixturePhysicalDegrees(limits.fixturePhysicalDegreesForDmxValue(32_768, null)),
  "Unavailable",
  "missing physical range must have an explicit unavailable marker");
const invalidRange = fixtureWithRanges([Number.NaN, 270]);
assert.equal(limits.fixturePhysicalAxisRange(invalidRange, "pan"), null, "invalid physical range must fail closed");
const partialRange = fixtureWithRanges([-270, 270], [-135, 135]);
partialRange.controls[0].functions[0].dmx_to = 32_767;
assert.equal(limits.fixturePhysicalAxisRange(partialRange, "pan"), null, "partial DMX coverage must fail closed");

console.log("fixture limit degree conversion, raw min/max, toggles, and fail-closed contracts ok");
