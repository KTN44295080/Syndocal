import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const source = await readFile(new URL("../src/fixtureCatalog.ts", import.meta.url), "utf8");
const transpiled = ts.transpileModule(source, {
  compilerOptions: {
    module: ts.ModuleKind.ESNext,
    target: ts.ScriptTarget.ES2022,
    importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
  },
  fileName: "fixtureCatalog.ts",
});
const catalog = await import(`data:text/javascript;base64,${Buffer.from(transpiled.outputText).toString("base64")}`);
const componentSource = await readFile(new URL("../src/components/FixtureCatalogPanel.tsx", import.meta.url), "utf8");
const tauriSource = await readFile(new URL("../src-tauri/src/main.rs", import.meta.url), "utf8");

const ridFixture = { rid: 7, uuid: "uuid", manufacturer: "Robe", fixture: "Mega", revision: "R1" };
const uuidFixture = { rid: null, uuid: " ABC-123 ", manufacturer: "Robe", fixture: "Mega", revision: "R1" };
const nameFixture = { rid: null, uuid: null, manufacturer: " Robe ", fixture: " Mega ", revision: " R1 " };
assert.equal(catalog.fixtureCatalogFavoriteKey(ridFixture), "share:rid:7");
assert.equal(catalog.fixtureCatalogFavoriteKey(uuidFixture), "share:uuid:abc-123");
assert.equal(catalog.fixtureCatalogFavoriteKey(nameFixture), "share:name:robe:mega:r1");
assert.equal(catalog.verifiedFixtureFavoriteKey(" RGBW-PAR "), "verified:rgbw-par");
assert.deepEqual(catalog.fixtureCatalogFavoritesFromUnknown(["a", "a", "", 2, " b "]), ["a", "b"]);
assert.deepEqual(catalog.toggledFixtureCatalogFavorites(["a"], "b"), ["a", "b"]);
assert.deepEqual(catalog.toggledFixtureCatalogFavorites(["a", "b"], "a"), ["b"]);
assert.deepEqual(catalog.fixtureFootprintBandBounds("any"), { min: null, max: null });
assert.deepEqual(catalog.fixtureFootprintBandBounds("1-4"), { min: 1, max: 4 });
assert.deepEqual(catalog.fixtureFootprintBandBounds("33-512"), { min: 33, max: 512 });
assert.equal(catalog.fixtureCatalogIdentityMatches(ridFixture, { ...ridFixture, path: "cache" }), true);
assert.equal(catalog.fixtureCatalogHealthLabel("fallback"), "Fallback");
assert.equal(catalog.fixtureCatalogHealthLabel("invalid"), "Invalid");
assert.equal(catalog.fixtureCatalogSearchTextMatches({ ...nameFixture, modes: [{ name: "Extended", dmx_footprint: 24 }] }, "extended"), true);
assert.equal(catalog.fixtureCatalogSearchTextMatches({ ...nameFixture, modes: [] }, "ETC"), false);

assert.equal(catalog.universityRigFixtureCount, 8);
assert.equal(catalog.universityRigModeCount, 40);
assert.equal(catalog.universityRigProfiles.length, 40);
assert.deepEqual(
  Object.fromEntries([...new Set(catalog.universityRigProfiles.map((entry) => entry.fixture_family))]
    .map((fixture) => [fixture, catalog.universityRigProfiles.filter((entry) => entry.fixture_family === fixture).length])),
  {
    "ePAR64 RGBA": 2,
    "STAGE EVOLUTION MINI SPOT30": 2,
    "Pinspot LED Quad DMX": 1,
    "ADJ SABER SPOT RGBW": 12,
    "ADJ Encore FR50Z": 6,
    "ADJ MEGA BAR RGBA": 7,
    "MEGA 64 Profile EP": 5,
    "MEGA 64 Profile Plus": 5,
  },
);
assert.equal(new Set(catalog.universityRigProfiles.map((entry) => entry.id)).size, 40);
assert.equal(catalog.universityRigProfiles.every((entry) => entry.manual_url?.startsWith("https://")), true);
assert.deepEqual(catalog.verifiedFixtureProfileRequest("university-epar64-rgba-8ch")?.attributes, [
  "ColorAdd_R@1:8", "ColorAdd_G@2:8", "ColorAdd_B@3:8", "ColorAdd_A@4:8",
  "Color1@5:8", "Control1@6:8", "Control2@7:8", "Dimmer@8:8",
]);
assert.deepEqual(catalog.verifiedFixtureProfileRequest("university-mini-spot30-11ch")?.attributes, [
  "Pan@1:16", "Tilt@3:16", "Color1@5:8", "Gobo1@6:8", "Shutter1@7:8",
  "Dimmer@8:8", "PanTiltSpeed@9:8", "Control1@10:8", "Control2@11:8",
]);
assert.deepEqual(catalog.verifiedFixtureProfileRequest("university-encore-fr50z-2-3ch")?.attributes, [
  "Dimmer@1:16", "Control1@3:8",
]);
assert.deepEqual(catalog.verifiedFixtureProfileRequest("university-mega-64-profile-plus-9ch")?.attributes.slice(0, 4), [
  "ColorAdd_R@1:8", "ColorAdd_G@2:8", "ColorAdd_B@3:8", "ColorAdd_UV@4:8",
]);
assert.deepEqual(catalog.verifiedFixtureProfileRequest("university-mega-bar-rgba-34ch")?.attributes.slice(-2), [
  "Shutter1@33:8", "Dimmer@34:8",
]);
assert.equal(catalog.verifiedFixtureProfileRequest("university-saber-spot-rgbw-programs-fine-12ch")?.attributes.at(-1), "Dimmer2@12:8");

assert.equal(catalog.personalRigFixtureCount, 4);
assert.equal(catalog.personalRigModeCount, 7);
assert.equal(catalog.personalRigProfiles.length, 7);
assert.deepEqual(
  Object.fromEntries([...new Set(catalog.personalRigProfiles.map((entry) => entry.fixture_family))]
    .map((fixture) => [fixture, catalog.personalRigProfiles.filter((entry) => entry.fixture_family === fixture).length])),
  {
    "960 sound waves strongpoint": 1,
    "Mini Moving Head Gobo Light": 1,
    "F3200A Laser": 2,
    wristband: 3,
  },
);
assert.equal(new Set(catalog.personalRigProfiles.map((entry) => entry.id)).size, 7);
assert.equal(catalog.personalRigProfiles.filter((entry) => entry.description.includes("deployed in DSF2026.dvc")).length, 4);
assert.deepEqual(catalog.verifiedFixtureProfileRequest("personal-960-sound-waves-strongpoint-13ch")?.attributes, [
  "Dimmer@1:8",
  "ColorAdd_R@2:8", "ColorAdd_G@3:8", "ColorAdd_B@4:8",
  "ColorAdd_R2@5:8", "ColorAdd_G2@6:8", "ColorAdd_B2@7:8",
  "ColorAdd_R3@8:8", "ColorAdd_G3@9:8", "ColorAdd_B3@10:8",
  "ColorAdd_R4@11:8", "ColorAdd_G4@12:8", "ColorAdd_B4@13:8",
]);
assert.deepEqual(catalog.verifiedFixtureProfileRequest("personal-mini-moving-head-gobo-light-10ch")?.attributes, [
  "Pan@1:8", "Tilt@2:8", "Color1@3:8", "Gobo1@4:8", "Shutter1@5:8",
  "Dimmer@6:8", "PanTiltSpeed@7:8", "Generic: Other@8:8", "Generic: Other 2@9:8", "Generic: Other 3@10:8",
]);
assert.deepEqual(catalog.verifiedFixtureProfileRequest("personal-f3200a-laser-6ch")?.attributes, [
  "Generic: Light Off/On@1:8",
  "Generic: Movement Sound/Auto Control@2:8",
  "Generic: Effect Library Selection@3:8",
  "Generic: Scene Selection@4:8",
  "Generic: Color Selection@5:8",
  "Generic: Movement Speed@6:8",
]);
assert.equal(catalog.verifiedFixtureProfileRequest("personal-f3200a-laser-34ch")?.attributes.length, 34);
assert.deepEqual(catalog.verifiedFixtureProfileRequest("personal-f3200a-laser-34ch")?.attributes.slice(-3), [
  "Generic: Gradual Drawing Control 2@32:8",
  "Generic: Pattern Distortion Effect's Miscellaneous Function Control 2@33:8",
  "Generic: Projection Range Control@34:8",
]);
assert.deepEqual(catalog.verifiedFixtureProfileRequest("personal-wristband-8ch")?.attributes, [
  "Shutter1@1:8", "ColorAdd_R@2:8", "ColorAdd_G@3:8", "ColorAdd_B@4:8",
  "Shutter2@5:8", "ColorAdd_R2@6:8", "ColorAdd_G2@7:8", "ColorAdd_B2@8:8",
]);
assert.equal(catalog.verifiedFixtureProfileCount, 107);
assert.equal(catalog.verifiedFixtureCategoryCount, 8);

assert.match(componentSource, /type="password"[\s\S]*?autocomplete="off"/, "credentials must remain an in-memory input");
assert.doesNotMatch(componentSource, /localStorage[\s\S]*?(user|password)|(user|password)[\s\S]*?localStorage/i, "credentials must not be persisted by the component");
assert.match(componentSource, /search_gdtf_share/, "online catalog must use the structured Share command");
assert.match(componentSource, /\[releaseOnly, setReleaseOnly\] = createSignal\(false\)/, "unknown public-API revision status must not hide the catalog by default");
assert.match(componentSource, /filter_support[\s\S]*?filters fail closed/, "optional Share metadata support must be disclosed");
assert.match(componentSource, /list_gdtf_fixture_cache/, "offline cache must be visible");
assert.match(componentSource, /get_fixture_profile_health/, "project profile health must be visible");
assert.match(componentSource, /create_custom_fixture_profile/, "verified fixture packs must be loadable through the custom-profile boundary");
assert.match(componentSource, /onRepair/, "missing profile repair must use the App mutation boundary");
assert.match(componentSource, /if \(cachedEntry\)[\s\S]*?loadCachedProfile\(cachedEntry,\s*true\)/, "cached online results must load without another authenticated download");
assert.match(tauriSource, /GdtfFixtureCacheMetadata[\s\S]*?manufacturer:[\s\S]*?revision:/, "cache sidecar must contain only fixture identity metadata");
assert.doesNotMatch(
  tauriSource.match(/struct GdtfFixtureCacheMetadata \{[\s\S]*?\n\}/)?.[0] ?? "",
  /user|password/i,
  "cache metadata schema must not contain credentials",
);
assert.match(tauriSource, /fixture_profile_repair_layout_matches/, "repair must compare the exact DMX layout");
assert.match(tauriSource, /\?rid=\{rid\}[\s\S]*?downloadFile\.php/, "Share downloads must use the public revision-ID GET contract");

console.log("fixture catalog helpers: 55 assertions passed");
