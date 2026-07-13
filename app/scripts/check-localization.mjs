import assert from "node:assert/strict";
import { readdir, readFile } from "node:fs/promises";
import ts from "typescript";

const source = await readFile(new URL("../src/uiLocalization.ts", import.meta.url), "utf8");
const transpiled = ts.transpileModule(source, {
  compilerOptions: {
    module: ts.ModuleKind.ESNext,
    target: ts.ScriptTarget.ES2022,
    importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
  },
  fileName: "uiLocalization.ts",
});
const localization = await import(
  `data:text/javascript;base64,${Buffer.from(transpiled.outputText).toString("base64")}`
);

assert.equal(localization.uiLocaleFromUnknown("ja"), "ja");
assert.equal(localization.uiLocaleFromUnknown("en"), "en");
assert.equal(localization.uiLocaleFromUnknown("future"), "en");
assert.equal(localization.translateUiText("Save", "ja"), "保存");
assert.equal(localization.translateUiText("  Save  ", "ja"), "  保存  ");
assert.equal(localization.translateUiText("3 steps", "ja"), "3 手順");
assert.equal(localization.translateUiText("Install v1.2.0", "ja"), "v1.2.0をインストール");
assert.equal(
  localization.translateUiText(
    "Created an unsaved project from Festival Base (2 embedded profiles, 4 MIDI, 1 OSC mappings). All DMX and video outputs are disabled and blacked out.",
    "ja",
  ),
  "Festival Baseから未保存プロジェクトを作成しました（埋め込みプロファイル 2、MIDI 4、OSC 1）。DMXと映像出力はすべて無効・ブラックアウトです。",
);
assert.equal(localization.translateUiText("Custom fixture", "ja"), "Custom fixture");
assert.equal(localization.translateUiText("Save", "en"), "Save");

for (const operatorText of [
  "New",
  "Save",
  "Load",
  "Undo",
  "Redo",
  "User Templates",
  "Application Updates",
  "Setup",
  "Control",
  "Touch",
  "Patch",
  "Video I/O",
  "Mapping",
  "Remote",
  "Live Edit",
  "Timeline",
  "VJ Desk",
  "Cues",
  "Playback",
  "Attributes",
  "Effects",
  "Position",
  "Color",
  "Dimmer",
  "Fixtures",
  "Lighting",
  "DMX Output",
  "DMX Blackout",
  "Video Outputs",
  "Video Blackout",
  "Projection Mapping",
  "Web Remote",
  "UI language",
]) {
  assert.notEqual(localization.translateUiText(operatorText, "ja"), operatorText, `${operatorText} must be localized`);
}

const values = new Map();
globalThis.window = {
  localStorage: {
    getItem: (key) => values.get(key) ?? null,
    setItem: (key, value) => values.set(key, value),
  },
};
assert.equal(localization.loadUiLocale(), "en");
assert.equal(localization.saveUiLocale("ja"), true);
assert.equal(localization.loadUiLocale(), "ja");

globalThis.window.localStorage.setItem = () => {
  throw new Error("storage denied");
};
assert.equal(localization.saveUiLocale("en"), false);
delete globalThis.window;

const sourceRoot = new URL("../src/", import.meta.url);
const tsxFiles = [];
async function collectTsx(directory) {
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const path = new URL(entry.name + (entry.isDirectory() ? "/" : ""), directory);
    if (entry.isDirectory()) await collectTsx(path);
    else if (entry.name.endsWith(".tsx")) tsxFiles.push(path);
  }
}
await collectTsx(sourceRoot);

const localeInvariantText = new Set([
  "Syndocal", "DMX", "MIDI", "OSC", "RDM", "ISF", "NDI", "Spout", "Syphon", "HAP", "FFmpeg",
  "GO", "SET", "HTP", "LTP", "RGB", "BPM", "A", "B", "R", "G", "X", "Y", "Z", "Pan", "Tilt",
  "Art-Net", "sACN", "sACN / E1.31", "sACN E1.31", "Enttec USB PRO", "Syphon Server", "Spout Sender",
  "NDI Sender", "Perlin", "Bezier", "Linear", "Full", "Half", "Scale X", "Scale Y",
  "ms", "x", "U", "ch", "CH", "deg", "m", "Hz", "P", "S", "O", "V", "D", "H", "L",
  "Key R", "Key G", "Key B", "Key Threshold", "Pan +", "Pan -", "Tilt +", "Tilt -",
  "ms /", "/ Z", "/512 used", "ch max free", "label, U1 A24, group", ", Y", ", Z", "· PID 0x",
  "· subdevices", "/ D", "/ native", "/ Tilt", "/ U", "/ V", "/ X", "/touchosc/page/*/fader/1",
  "&gt;", "% / Tilt", "|&lt;", "0.25x", "0.5x", "1x", "2x", "4x", "1234:56789ABC", "16-bit",
  "8-bit", "BL X", "BL Y", "BR X", "BR Y", "C:\\path\\fixture.gdtf", "CC", "ch · personality", "ch /",
  "COM3 or /dev/ttyUSB0", "Crop B", "Crop L", "Crop R", "Crop T", "ctl", "ctrl", "Cue #", "DECK A",
  "DECK B", "deg / Sat", "Dimmer@1:8, Pan@2:16, Tilt@4:16, ColorRed@6:8", "Dir X", "Dir Y", "Dir Z",
  "DMXKing ultraDMX", "e/", "Enttec Open DMX", "ENTTEC USB Pro", "fps", "front, bars",
  "front, movers, floor", "FX", "GDTF", "GDTF Share", "GET", "Hi", "HL",
  "https://gdtf-share.com/.../fixture.gdtf", "Key H", "Key V", "Keystone H", "Keystone V", "Keystone X",
  "Keystone Y", "LFO", "LIVE LINK", "m /", "MID", "MIB targets (", "ms ·", "ms · max", "NACK reason 0x",
  "PID (hex)", "Port-Address",
  "BO", "DMX BO", "C:\\\\path\\\\fixture.gdtf", "Pan %", "Rev", "s",
  "T", "Tilt %", "TL X", "TL Y", "TR X", "TR Y", "us", "v", "Video L", "W", "wl", "x/",
]);
const untranslated = new Map();
const unprotectedUserText = [];
const localizedDynamicTextAllow = new Set([
  "ColorEffectEditorPanel.tsx:preset.label",
  "CueCapturePreviewPanel.tsx:row.label",
  "EffectSourceControlsPanel.tsx:preset.label",
  "MappingHotkeyHelp.tsx:group.label",
  "MappingProjectorControlsPanel.tsx:preset.label",
  "OpticsControlPanel.tsx:preset.label",
  "ProjectorMapEditor.tsx:preset.label",
]);
let staticTextCount = 0;
let localizedStaticTextCount = 0;
for (const file of tsxFiles) {
  const sourceText = await readFile(file, "utf8");
  const sourceFile = ts.createSourceFile(file.pathname, sourceText, ts.ScriptTarget.ES2022, true, ts.ScriptKind.TSX);
  const visit = (node) => {
    const candidates = [];
    if (ts.isJsxText(node)) candidates.push(node.getText(sourceFile).replace(/\s+/g, " ").trim());
    if (
      ts.isJsxAttribute(node) &&
      ["title", "aria-label", "placeholder"].includes(node.name.getText(sourceFile)) &&
      node.initializer &&
      ts.isStringLiteral(node.initializer)
    ) {
      candidates.push(node.initializer.text.trim());
    }
    for (const text of candidates) {
      if (!text || !/[A-Za-z]/.test(text)) continue;
      staticTextCount += 1;
      if (localeInvariantText.has(text) || localization.translateUiText(text, "ja") !== text) {
        localizedStaticTextCount += 1;
      } else {
        untranslated.set(text, (untranslated.get(text) ?? 0) + 1);
      }
    }
    if (ts.isJsxExpression(node) && ts.isJsxElement(node.parent) && node.expression) {
      const expressionText = node.expression.getText(sourceFile).replace(/\s+/g, "");
      if (
        /(?:^|\.)(?:fixture|cue|layer|output|composition|palette|group|object|surface|event|automation|preset|profile|submaster|report|row)[A-Za-z0-9_?!.]*\.(?:label|name|groupId|group_id|path)$/.test(expressionText)
      ) {
        const attributes = node.parent.openingElement.attributes.properties;
        const protectedText = attributes.some(
          (attribute) => ts.isJsxAttribute(attribute) && attribute.name.getText(sourceFile) === "data-no-localize",
        );
        const sourceName = file.pathname.split("/").pop();
        if (!protectedText && !localizedDynamicTextAllow.has(`${sourceName}:${expressionText}`)) {
          const position = sourceFile.getLineAndCharacterOfPosition(node.getStart(sourceFile));
          unprotectedUserText.push(`${sourceName}:${position.line + 1} ${expressionText}`);
        }
      }
    }
    ts.forEachChild(node, visit);
  };
  visit(sourceFile);
}
const coverage = staticTextCount === 0 ? 1 : localizedStaticTextCount / staticTextCount;
// Diagnostic output is emitted below before the final coverage assertion.
const reportOffset = Number.parseInt(process.env.LOCALIZATION_REPORT_OFFSET ?? "0", 10);
const reportLimit = Number.parseInt(process.env.LOCALIZATION_REPORT_LIMIT ?? "20", 10);
console.log(
  `static Japanese UI coverage: ${localizedStaticTextCount}/${staticTextCount} (${(coverage * 100).toFixed(1)}%)`,
);
console.log(`unprotected bare user-data labels: ${unprotectedUserText.length}`);
if (unprotectedUserText.length > 0) {
  console.log(unprotectedUserText.slice(0, Number.parseInt(process.env.USER_TEXT_REPORT_LIMIT ?? "30", 10)).join(" | "));
}
assert.equal(unprotectedUserText.length, 0, "bare user-data labels must opt out of UI localization");
console.log(
  "most frequent untranslated static UI:",
  [...untranslated.entries()]
    .sort((left, right) => right[1] - left[1] || left[0].localeCompare(right[0]))
    .slice(reportOffset, reportOffset + reportLimit)
    .map(([text, count]) => `${count}× ${text}`)
    .join(" | "),
);

assert.equal(coverage, 1, `static Japanese UI coverage regressed to ${(coverage * 100).toFixed(1)}%`);

console.log("ui localization helpers ok");
