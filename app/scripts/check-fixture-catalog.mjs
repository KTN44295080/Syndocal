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

assert.match(componentSource, /type="password"[\s\S]*?autocomplete="off"/, "credentials must remain an in-memory input");
assert.doesNotMatch(componentSource, /localStorage[\s\S]*?(user|password)|(user|password)[\s\S]*?localStorage/i, "credentials must not be persisted by the component");
assert.match(componentSource, /search_gdtf_share/, "online catalog must use the structured Share command");
assert.match(componentSource, /\[releaseOnly, setReleaseOnly\] = createSignal\(false\)/, "unknown public-API revision status must not hide the catalog by default");
assert.match(componentSource, /filter_support[\s\S]*?filters fail closed/, "optional Share metadata support must be disclosed");
assert.match(componentSource, /list_gdtf_fixture_cache/, "offline cache must be visible");
assert.match(componentSource, /get_fixture_profile_health/, "project profile health must be visible");
assert.match(componentSource, /load_verified_fixture_profile/, "verified common-rig pack must be loadable");
assert.match(componentSource, /onRepair/, "missing profile repair must use the App mutation boundary");
assert.match(componentSource, /if \(cachedEntry\)[\s\S]*?loadCachedProfile\(cachedEntry\)/, "cached online results must load without another authenticated download");
assert.match(tauriSource, /GdtfFixtureCacheMetadata[\s\S]*?manufacturer:[\s\S]*?revision:/, "cache sidecar must contain only fixture identity metadata");
assert.doesNotMatch(
  tauriSource.match(/struct GdtfFixtureCacheMetadata \{[\s\S]*?\n\}/)?.[0] ?? "",
  /user|password/i,
  "cache metadata schema must not contain credentials",
);
assert.match(tauriSource, /fixture_profile_repair_layout_matches/, "repair must compare the exact DMX layout");
assert.match(tauriSource, /\?rid=\{rid\}[\s\S]*?downloadFile\.php/, "Share downloads must use the public revision-ID GET contract");

console.log("fixture catalog helpers: 29 assertions passed");
