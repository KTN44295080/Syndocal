import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const helperSource = await readFile(new URL("../src/virtualList.ts", import.meta.url), "utf8");
const transpiled = ts.transpileModule(helperSource, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
  fileName: "virtualList.ts",
});
const { virtualListRange } = await import(
  `data:text/javascript;base64,${Buffer.from(transpiled.outputText).toString("base64")}`
);

assert.deepEqual(
  virtualListRange({ itemCount: 1_000, itemHeight: 104, scrollTop: 0, viewportHeight: 480, overscan: 5 }),
  { startIndex: 0, endIndex: 10, offsetPx: 0, totalHeightPx: 104_000 },
);

const middle = virtualListRange({
  itemCount: 1_000,
  itemHeight: 104,
  scrollTop: 5_200,
  viewportHeight: 480,
  overscan: 5,
});
assert.deepEqual(middle, { startIndex: 45, endIndex: 60, offsetPx: 4_680, totalHeightPx: 104_000 });
assert.ok(middle.endIndex - middle.startIndex <= 15, "A 1,000-row show must keep the rendered fixture window bounded.");

assert.deepEqual(
  virtualListRange({ itemCount: 3, itemHeight: 104, scrollTop: 999_999, viewportHeight: 480, overscan: 5 }),
  { startIndex: 0, endIndex: 3, offsetPx: 0, totalHeightPx: 312 },
);
assert.deepEqual(
  virtualListRange({ itemCount: 0, itemHeight: 0, scrollTop: Number.NaN, viewportHeight: Number.NaN }),
  { startIndex: 0, endIndex: 0, offsetPx: 0, totalHeightPx: 0 },
);

const fixturePanelSource = await readFile(new URL("../src/components/SetupFixtureListPanel.tsx", import.meta.url), "utf8");
assert.match(fixturePanelSource, /fixtureVirtualizationThreshold\s*=\s*80/);
assert.match(fixturePanelSource, /visibleFixtures\(\)/);
assert.match(fixturePanelSource, /aria-rowcount=\{props\.fixtures\.length\}/);

const cuePanelSource = await readFile(new URL("../src/components/CueManagementPanel.tsx", import.meta.url), "utf8");
assert.match(cuePanelSource, /cuesPerPage\s*=\s*12/);
assert.match(cuePanelSource, /<For each=\{visibleCues\(\)\}>/);
assert.match(cuePanelSource, /aria-setsize=\{props\.cues\.length\}/);

const mappingFixtureSource = await readFile(new URL("../src/components/MappingFixtureSelectionPanel.tsx", import.meta.url), "utf8");
assert.match(mappingFixtureSource, /mappingFixtureVirtualizationThreshold\s*=\s*60/);
assert.match(mappingFixtureSource, /virtualizedMappingFixtureList/);
assert.match(mappingFixtureSource, /<For each=\{visibleFixtures\(\)\}>/);

console.log("large-show UI rendering boundaries ok");
