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

console.log("fixture catalog helpers: 41 assertions passed");
