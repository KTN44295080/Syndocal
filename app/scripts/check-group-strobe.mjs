import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const source = await readFile(new URL("../src/groupStrobe.ts", import.meta.url), "utf8");
const transpiled = ts.transpileModule(source, {
  compilerOptions: {
    module: ts.ModuleKind.ESNext,
    target: ts.ScriptTarget.ES2022,
    importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
  },
  fileName: "groupStrobe.ts",
});
const helpers = await import(
  `data:text/javascript;base64,${Buffer.from(transpiled.outputText).toString("base64")}`
);
const appSource = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
const stripSource = await readFile(new URL("../src/components/GroupLiveMixerStrip.tsx", import.meta.url), "utf8");
const touchSource = await readFile(new URL("../src/components/TouchFixtureActionsPanel.tsx", import.meta.url), "utf8");

const calibrated = {
  attribute: "Shutter1",
  functions: [{
    name: "Strobe",
    attribute: "Shutter1Strobe",
    dmx_from: 16_384,
    dmx_to: 65_535,
    physical_from: 1,
    physical_to: 25,
  }],
};
const nameOnly = {
  attribute: "Shutter1",
  channel_name: "Strobe 1-25 Hz",
  functions: [{
    name: "Strobe 1-25 Hz",
    attribute: "Shutter1",
    dmx_from: 16_384,
    dmx_to: 65_535,
    physical_from: 1,
    physical_to: 25,
  }],
};
const missingPhysical = {
  attribute: "Shutter1",
  functions: [{
    name: "Strobe",
    attribute: "Shutter1Strobe",
    dmx_from: 16_384,
    dmx_to: 65_535,
  }],
};
const pulse = {
  attribute: "Shutter1",
  functions: [{
    name: "Pulse",
    attribute: "Shutter1StrobePulse",
    dmx_from: 16_384,
    dmx_to: 65_535,
    physical_from: 1,
    physical_to: 25,
  }],
};

assert.equal(helpers.normalizedGroupStrobeFunctionAttribute("Shutter1Strobe"), "shutterstrobe");
assert.equal(helpers.fixtureHasGroupStrobeMetadata({ controls: [calibrated] }), true);
assert.equal(helpers.fixtureHasGroupStrobeMetadata({ controls: [nameOnly] }), false, "names cannot guess strobe semantics");
assert.equal(helpers.fixtureHasGroupStrobeMetadata({ controls: [missingPhysical] }), false, "physical Hz metadata is mandatory");
assert.equal(helpers.fixtureHasGroupStrobeMetadata({ controls: [pulse] }), false, "pulse/random variants are not continuous strobe");
assert.equal(
  helpers.groupStrobeCompatibleFixtureCount([
    { controls: [calibrated] },
    { controls: [nameOnly] },
    { controls: [calibrated] },
  ]),
  2,
);
assert.equal(helpers.GROUP_STROBE_MAX_HZ, 30);

assert.match(appSource, /invoke\("set_group_strobe", \{ groupId, rateHz \}\)/, "desktop control must invoke the engine command");
assert.match(appSource, /<GroupLiveMixerStrip[\s\S]*?onSetStrobe=\{setGroupStrobe\}/, "Control Live must expose the group mixer strip");
assert.match(stripSource, /data-strobe-compatible-count/, "desktop strip must disclose compatible fixture coverage");
assert.match(stripSource, /onSetSolo/, "group solo must be directly available beside strobe");
assert.match(touchSource, /Touch group strobe rate/, "Touch must expose the same group strobe control");

console.log("group strobe helpers: 16 assertions passed");
