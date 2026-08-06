import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import {
  encodedAttributeFootprint,
  fixtureIdentity,
} from "./profile-library-common.mjs";

const appDir = join(dirname(fileURLToPath(import.meta.url)), "..");
const readBundle = (name) => JSON.parse(readFileSync(join(appDir, "src", "generated", name), "utf8"));
const ofl = readBundle("oflLibrary.json");
const qlc = readBundle("qlcLibrary.json");
const failures = [];
const assert = (condition, message) => {
  if (!condition) failures.push(message);
};

const validateBundle = (id, bundle) => {
  assert(bundle.v === 2, `${id}: expected bundle version 2`);
  assert(/^[0-9a-f]{40}$/.test(bundle.sourceRevision), `${id}: source revision is not a pinned 40-character SHA`);
  assert(Array.isArray(bundle.fixtures) && bundle.fixtures.length > 0, `${id}: fixture payload is empty`);
  assert(bundle.audit.bundledFixtures === bundle.fixtures.length, `${id}: fixture audit count drifted`);
  const modeCount = bundle.fixtures.reduce((total, fixture) => total + fixture.modes.length, 0);
  assert(bundle.audit.bundledModes === modeCount, `${id}: mode audit count drifted`);

  const fixtureKeys = new Set();
  for (const fixture of bundle.fixtures) {
    const fixtureKey = fixtureIdentity(fixture.m, fixture.n);
    assert(!fixtureKeys.has(fixtureKey), `${id}: duplicate fixture identity ${fixture.m} / ${fixture.n}`);
    fixtureKeys.add(fixtureKey);
    const modeNames = new Set();
    for (const mode of fixture.modes) {
      assert(!modeNames.has(mode.n), `${id}: duplicate mode name ${fixture.m} / ${fixture.n} / ${mode.n}`);
      modeNames.add(mode.n);
      assert(mode.a.length > 0, `${id}: empty mode ${fixture.m} / ${fixture.n} / ${mode.n}`);
      let expectedOffset = 1;
      const attributes = new Set();
      for (const encoded of mode.a) {
        const match = encoded.match(/^(.+)@(\d+):(8|16)$/);
        assert(Boolean(match), `${id}: invalid encoded attribute ${encoded}`);
        if (!match) continue;
        const attribute = match[1].toLocaleLowerCase();
        assert(!attributes.has(attribute), `${id}: duplicate attribute ${encoded}`);
        attributes.add(attribute);
        assert(Number(match[2]) === expectedOffset, `${id}: non-contiguous offset ${encoded}`);
        expectedOffset += match[3] === "16" ? 2 : 1;
      }
      const footprint = encodedAttributeFootprint(mode.a);
      assert(footprint === expectedOffset - 1, `${id}: footprint/offset mismatch in ${mode.n}`);
      assert(footprint >= 1 && footprint <= 512, `${id}: invalid ${footprint}ch footprint in ${mode.n}`);
    }
  }
  return { fixtureKeys, modeCount };
};

const oflMetrics = validateBundle("OFL", ofl);
const qlcMetrics = validateBundle("QLC+", qlc);
const overlaps = [...qlcMetrics.fixtureKeys].filter((key) => oflMetrics.fixtureKeys.has(key));
assert(overlaps.length === 0, `source precedence failed: ${overlaps.length} QLC+ identities overlap OFL`);
assert(ofl.audit.inputModes === ofl.audit.bundledModes + ofl.audit.skippedModes, "OFL: input-mode audit does not reconcile");
assert(ofl.audit.matrixModesExpanded === 714, "OFL: expected all 714 matrix modes to be expanded");
assert(ofl.audit.skippedModes === 0, "OFL: modes were skipped");
assert(ofl.audit.malformedFixtures === 0, "OFL: malformed fixtures were skipped");
assert(
  qlc.audit.inputModes === qlc.audit.bundledModes
    + qlc.audit.deduplicatedModesByOfl
    + qlc.audit.duplicateModesRemoved
    + qlc.audit.skippedModes,
  "QLC+: input-mode audit does not reconcile",
);
assert(qlc.audit.skippedModes === 0, "QLC+: modes were skipped");
assert(qlc.audit.malformedFixtures === 0, "QLC+: malformed fixtures were skipped");
assert(qlc.audit.deduplicatedByOfl > 0, "QLC+: OFL precedence did not deduplicate any fixtures");

const fixtureCount = ofl.fixtures.length + qlc.fixtures.length;
const modeCount = oflMetrics.modeCount + qlcMetrics.modeCount;
assert(fixtureCount >= 2_200, `combined fixture count regressed to ${fixtureCount}`);
assert(modeCount >= 7_300, `combined mode count regressed to ${modeCount}`);

if (failures.length > 0) {
  console.error(`profile library failed (${failures.length}):`);
  for (const failure of failures) console.error(`- ${failure}`);
  process.exit(1);
}
console.log(
  `profile library passed: ${fixtureCount} fixtures / ${modeCount} modes; ` +
    `OFL ${ofl.sourceRevision.slice(0, 7)} + QLC+ ${qlc.sourceRevision.slice(0, 7)}; ` +
    `${ofl.audit.matrixModesExpanded} matrix modes expanded; ` +
    `${qlc.audit.deduplicatedByOfl} QLC+ conflicts superseded by OFL`,
);
