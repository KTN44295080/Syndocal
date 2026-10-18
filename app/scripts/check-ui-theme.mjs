import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const loadTs = async (path) => {
  const source = await readFile(new URL(path, import.meta.url), "utf8");
  const result = ts.transpileModule(source, {
    compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
  });
  return import(`data:text/javascript;base64,${Buffer.from(result.outputText).toString("base64")}`);
};

const theme = await loadTs("../src/uiTheme.ts");
const localization = await loadTs("../src/uiLocalization.ts");
const storage = new Map([
  ["syndocal.uiScale.v1", "110"],
  ["syndocal.uiLocale.v1", "ja"],
]);
globalThis.window = {
  localStorage: {
    getItem: (key) => storage.get(key) ?? null,
    setItem: (key, value) => storage.set(key, value),
  },
};
globalThis.document = { documentElement: { dataset: {} } };

assert.deepEqual(theme.uiThemeOptions.map(({ id }) => id), ["tally", "graphite", "ice", "cobalt", "forest", "ember", "neon", "safe"]);
assert.equal(theme.loadUiTheme(), "tally", "new installations use the approved Tally palette");
for (const { id, label } of theme.uiThemeOptions) {
  assert.equal(theme.uiThemeFromUnknown(id), id);
  assert.equal(theme.saveUiTheme(id), true);
  assert.equal(storage.get("syndocal.uiTheme.v1"), id);
  assert.equal(theme.loadUiTheme(), id, "saved palette survives a fresh preference load");
  theme.applyUiTheme(id);
  assert.equal(document.documentElement.dataset.theme, id);
  assert.notEqual(localization.translateUiText(label, "ja"), label, `${label} has a Japanese name`);
}
for (const value of [null, undefined, "future", "ICE", "ice ", {}, 100]) {
  assert.equal(theme.uiThemeFromUnknown(value), "tally", "unknown theme IDs never reach CSS");
}
storage.set(theme.uiThemeStorageKey, "future");
assert.equal(theme.loadUiTheme(), "tally");
assert.equal(storage.get("syndocal.uiScale.v1"), "110");
assert.equal(storage.get("syndocal.uiLocale.v1"), "ja");
assert.equal(localization.translateUiText("Theme", "ja"), "テーマ");
window.localStorage = {
  getItem() { throw new Error("storage unavailable"); },
  setItem() { throw new Error("storage unavailable"); },
};
assert.equal(theme.loadUiTheme(), "tally");
assert.equal(theme.saveUiTheme("ice"), false);
theme.applyUiTheme("ice");
assert.equal(document.documentElement.dataset.theme, "ice", "storage failure keeps the session preference usable");
console.log("UI theme preference contract passed: eight palettes, persistence, invalid values, Japanese names, storage failure.");
