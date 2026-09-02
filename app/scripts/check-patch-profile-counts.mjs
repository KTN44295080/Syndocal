import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const source = (await readFile(
  resolve(fileURLToPath(new URL("../src/components/PatchProfileBrowserPanel.tsx", import.meta.url))),
  "utf8",
)).replace(/\r\n?/gu, "\n");

assert.match(
  source,
  /const visibleVerifiedFixtureCount = createMemo\(\(\) => visibleVerifiedFixtures\(\)\.length\);/u,
  "verified pack count must be the number of fixture families, not modes",
);
assert.match(
  source,
  /const visibleBundledFixtureCount = createMemo\(\(\) => visibleBundledFixtures\(\)\.length\);/u,
  "bundled library count must be the number of fixture families, not modes",
);

const verifiedSection = source.match(
  /<section class="patchProfileBrowserSection" data-patch-profile-section="verified">[\s\S]*?<\/section>/u,
)?.[0] ?? "";
const bundledSection = source.match(
  /<section class="patchProfileBrowserSection" data-patch-profile-section="bundled">[\s\S]*?<\/section>/u,
)?.[0] ?? "";
const cacheSection = source.match(
  /<section class="patchProfileBrowserSection" data-patch-profile-section="cache">[\s\S]*?<\/section>/u,
)?.[0] ?? "";

assert.match(verifiedSection, /data-patch-profile-section-count="fixtures"/u);
assert.match(verifiedSection, /visibleVerifiedFixtureCount\(\)/u);
assert.doesNotMatch(verifiedSection, /visibleVerifiedProfileCount\(\)/u);
assert.match(bundledSection, /data-patch-profile-section-count="fixtures"/u);
assert.match(bundledSection, /visibleBundledFixtureCount\(\)/u);
assert.doesNotMatch(bundledSection, /visibleBundledProfileCount\(\)/u);
assert.match(cacheSection, /visibleCacheFixtures\(\)\.length[\s\S]*?<span>\{visibleCacheFixtures\(\)\.length === 1 \? "fixture" : "fixtures"\}<\/span>/u);
assert.doesNotMatch(cacheSection, /cacheEntries\(\)\.length[\s\S]*?<span>profiles<\/span>/u);

const treeSource = await readFile(
  resolve(fileURLToPath(new URL("../src/components/GdtfProfileTree.tsx", import.meta.url))),
  "utf8",
);
assert.match(
  treeSource,
  /<b data-no-localize>\{fixture\.modeCount\}<\/b>[\s\S]*?\{fixture\.modeCount === 1 \? "Mode" : "Modes"\}/u,
  "mode counts must remain visible on each fixture row where they describe selectable modes",
);

console.log("patch profile count checks passed (10 assertions)");
