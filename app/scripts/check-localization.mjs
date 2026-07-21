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
const timelineOverviewSource = await readFile(
  new URL("../src/components/TimelineOverview.tsx", import.meta.url),
  "utf8",
);
const valueEffectEditorSource = await readFile(
  new URL("../src/components/ValueEffectEditorPanel.tsx", import.meta.url),
  "utf8",
);

assert.equal(localization.uiLocaleFromUnknown("ja"), "ja");
assert.equal(localization.uiLocaleFromUnknown("en"), "en");
assert.equal(localization.uiLocaleFromUnknown("future"), "en");
assert.equal(localization.translateUiText("Save", "ja"), "保存");
assert.equal(localization.translateUiText("  Save  ", "ja"), "  保存  ");
assert.equal(localization.translateUiText("3 steps", "ja"), "3 手順");
assert.equal(localization.translateUiText("Install v1.2.0", "ja"), "v1.2.0をインストール");
assert.equal(localization.translateUiText("Jump", "ja"), "ジャンプ");
assert.equal(localization.translateUiText("Live audio input stopped.", "ja"), "ライブ音声入力は停止中です。");
assert.equal(localization.translateUiText("Bass level", "ja"), "低域レベル");
assert.equal(localization.translateUiText("0 percent", "ja"), "0パーセント");
assert.equal(localization.translateUiText("Checking", "ja"), "確認中");
assert.equal(localization.translateUiText("Clear Pending", "ja"), "クリア待ち");
assert.equal(localization.translateUiText("Audio Reactive", "ja"), "オーディオリアクティブ");
assert.equal(localization.translateUiText("Create Mapping", "ja"), "マッピングを作成");
assert.equal(localization.translateUiText("Audio feature input level", "ja"), "音声特徴量の入力レベル");
assert.equal(localization.translateUiText("Open Rack", "ja"), "ラックを開く");
assert.equal(
  localization.translateUiText("Built-in FX for Program Texture (layer 12)", "ja"),
  "Program Texture（レイヤー12）の内蔵FX",
);
assert.equal(
  localization.translateUiText("FX enabled for Program Texture (layer 12)", "ja"),
  "Program Texture（レイヤー12）のFX有効状態",
);
assert.equal(
  localization.translateUiText("Threshold FX enabled for Program Texture (layer 12)", "ja"),
  "Program Texture（レイヤー12）のThreshold FX有効状態",
);
assert.equal(
  localization.translateUiText("useSourceAlpha for Threshold FX on Program Texture (layer 12)", "ja"),
  "Program Texture（レイヤー12）のThreshold FXのuseSourceAlpha",
);
assert.equal(
  localization.translateUiText("FX stack for Program Texture (layer 12)", "ja"),
  "Program Texture（レイヤー12）のFXスタック",
);
assert.equal(localization.translateUiText("Move FX 3 down", "ja"), "FX 3を下へ移動");
assert.equal(localization.translateUiText("Reset FX 2", "ja"), "FX 2をリセット");
assert.equal(
  localization.translateUiText("Trigger pulse for Program Texture (layer 12)", "ja"),
  "Program Texture（レイヤー12）のpulseをトリガー",
);
assert.equal(
  localization.translateUiText("Advanced FX controls for Program Texture (layer 12)", "ja"),
  "Program Texture（レイヤー12）のFX詳細操作",
);
assert.equal(
  localization.translateUiText("FX error for Program Texture (layer 12): shader compile failed", "ja"),
  "Program Texture（レイヤー12）のFXエラー: shader compile failed",
);
assert.equal(localization.translateUiText("Video output selection", "ja"), "映像出力の選択");
assert.equal(localization.translateUiText("Live", "ja"), "ライブ");
assert.equal(localization.translateUiText("BO", "ja"), "BO");
assert.equal(localization.translateUiText("Off", "ja"), "OFF");
assert.equal(
  localization.translateUiText("Open Audio Reactive Rack", "ja"),
  "オーディオリアクティブ・ラックを開く",
);
assert.equal(localization.translateUiText("SAFE ZERO", "ja"), "安全ゼロ");
assert.equal(
  localization.translateUiText("System default audio input", "ja"),
  "システム既定の音声入力",
);
assert.equal(localization.translateUiText("Audio input backend", "ja"), "音声入力バックエンド");
assert.equal(localization.translateUiText("NOT BUILT", "ja"), "未ビルド");
assert.equal(localization.translateUiText("Select ASIO driver", "ja"), "ASIOドライバを選択");
assert.equal(localization.translateUiText("Select fixed buffer", "ja"), "固定バッファを選択");
assert.equal(localization.translateUiText("Reselect input", "ja"), "入力を再選択");
assert.equal(
  localization.translateUiText("Found 2 audio input device(s).", "ja"),
  "音声入力デバイスが2件見つかりました。",
);
assert.equal(
  localization.translateUiText("Live audio FFT input started.", "ja"),
  "ライブ音声FFT入力を開始しました。",
);
assert.equal(
  localization.translateUiText(
    "The previous audio input could not be identified safely after Refresh. Select an input again; Start is locked.",
    "ja",
  ),
  "更新後に以前の音声入力を安全に特定できませんでした。入力を選び直してください。開始はロックされています。",
);
assert.match(source, /"aria-valuetext"/, "dynamic meter aria-valuetext must be localized");
assert.equal(localization.translateUiText("Blocks 13-24 / 500", "ja"), "ブロック 13-24 / 500");
assert.equal(
  localization.translateUiText("Edit Source for Cue Opening Wash", "ja"),
  "キュー Opening Wash のソースを編集",
);
assert.equal(localization.translateUiText("Lighting overlap ×250", "ja"), "照明の重複 ×250");
assert.equal(
  localization.translateUiText("Lane 3: Front Wash, 42 items", "ja"),
  "レーン 3: Front Wash、42件",
);
assert.equal(
  localization.translateUiText("Expand Lighting timeline section", "ja"),
  "照明タイムラインセクションを展開",
);
assert.equal(
  localization.translateUiText("Collapse Video timeline section", "ja"),
  "映像タイムラインセクションを折りたたむ",
);
assert.equal(
  localization.translateUiText("Video overlap (250); 0 to 256000 ms; inspect 250 overlapping blocks", "ja"),
  "映像の重複。0〜256000 ms、250ブロックを確認",
);
assert.equal(
  localization.translateUiText(
    "Lighting overlap groups (3); 1000 to 9000 ms; inspect 42 blocks across 3 overlap groups",
    "ja",
  ),
  "照明の重複グループ。1000〜9000 ms、3グループ内の42ブロックを確認",
);
assert.equal(
  localization.translateUiText("L 3 · V 2 · FX 1 · Fade 320 ms", "ja"),
  "照明 3 · 映像 2 · FX 1 · フェード 320 ms",
);
assert.equal(
  localization.translateUiText("1000 ms × 4 = 4000 ms span", "ja"),
  "1000 ms × 4回 = 4000 ms",
);
assert.equal(
  localization.translateUiText("Opening Wash / Lighting / 0 ms / 1000 ms x 4", "ja"),
  "Opening Wash / 照明 / 0 ms / 1000 ms × 4回",
);
assert.equal(
  localization.translateUiText("Added linked Scene Block 9 at 1200 ms (500 ms × 4)", "ja"),
  "連動シーンブロック 9 を 1200 ms に追加しました（500 ms × 4回）",
);
assert.equal(
  localization.translateUiText("Moved Scene Block 9 to 2400 ms", "ja"),
  "シーンブロック 9 を 2400 ms へ移動しました",
);
assert.equal(
  localization.translateUiText("Block 12 · 4.2 Moving Head @ 8000ms", "ja"),
  "ブロック 12 · 4.2 Moving Head @ 8000ms",
);
assert.equal(
  localization.translateUiText("Jump · Block 12 · 4.2 Moving Head @ 8000ms", "ja"),
  "ジャンプ · ブロック 12 · 4.2 Moving Head @ 8000ms",
);
assert.equal(
  localization.translateUiText("Repeat this block · Block 12 · 4.2 Moving Head @ 8000ms", "ja"),
  "このブロックを繰り返す · ブロック 12 · 4.2 Moving Head @ 8000ms",
);
assert.equal(
  localization.translateUiText("80 shown · 500 total", "ja"),
  "80件表示 · 全500件",
);
assert.equal(
  localization.translateUiText(
    "Created an unsaved project from Festival Base (2 embedded profiles, 4 MIDI, 1 OSC mappings). All DMX and video outputs are disabled and blacked out.",
    "ja",
  ),
  "Festival Baseから未保存プロジェクトを作成しました（埋め込みプロファイル 2、MIDI 4、OSC 1）。DMXと映像出力はすべて無効・ブラックアウトです。",
);
assert.equal(localization.translateUiText("Custom fixture", "ja"), "Custom fixture");
assert.equal(localization.translateUiText("Effect family chooser", "ja"), "エフェクトファミリーチューザー");
assert.equal(localization.translateUiText("COLOR FX", "ja"), "カラーFX");
assert.equal(localization.translateUiText("SUPER SCENE", "ja"), "SUPER SCENE");
assert.equal(localization.translateUiText("Current effect waveform preview", "ja"), "現在のエフェクト波形プレビュー");
assert.equal(localization.translateUiText("Move graphical preview", "ja"), "ムーブのグラフィカルプレビュー");
assert.equal(localization.translateUiText("Value graphical preview", "ja"), "バリューのグラフィカルプレビュー");
assert.equal(localization.translateUiText("REL @ 50%", "ja"), "相対 @ 50%");
assert.equal(
  localization.translateUiText("Output preview · neutral 50% base", "ja"),
  "出力プレビュー · 基準値50%",
);
assert.equal(localization.translateUiText("Path point 3, X 0.125, Y 0.875", "ja"), "パスポイント3、X 0.125、Y 0.875");
assert.equal(localization.translateUiText("4 points", "ja"), "4ポイント");
assert.equal(
  localization.translateUiText(
    "Absolute Line value envelope with 4 points. Double-click to add. Focus a point and use Arrow keys to nudge, Shift coarse, Alt fine, Delete to remove.",
    "ja",
  ),
  "絶対・線のバリューエンベロープ、4ポイント。ダブルクリックで追加。ポイントにフォーカスし、矢印キーで微調整、Shiftで粗く、Altで細かく、Deleteで削除。",
);
assert.equal(
  localization.translateUiText(
    "Absolute Step value envelope with 2 points. Double-click to add. Focus a point and use Arrow keys to nudge, Shift coarse, Alt fine, Delete to remove.",
    "ja",
  ),
  "絶対・ステップのバリューエンベロープ、2ポイント。ダブルクリックで追加。ポイントにフォーカスし、矢印キーで微調整、Shiftで粗く、Altで細かく、Deleteで削除。",
);
assert.equal(
  localization.translateUiText(
    "Relative Smooth value envelope with 3 points. Double-click to add. Focus a point and use Arrow keys to nudge, Shift coarse, Alt fine, Delete to remove.",
    "ja",
  ),
  "相対・スムーズのバリューエンベロープ、3ポイント。ダブルクリックで追加。ポイントにフォーカスし、矢印キーで微調整、Shiftで粗く、Altで細かく、Deleteで削除。",
);
assert.equal(
  localization.translateUiText("Envelope point 2, position 0.5, value 0.75", "ja"),
  "エンベロープポイント2、位置 0.5、値 0.75",
);
assert.equal(localization.translateUiText("Select envelope point 2", "ja"), "エンベロープポイント2を選択");
assert.equal(localization.translateUiText("Position for envelope point 2", "ja"), "エンベロープポイント2の位置");
assert.equal(localization.translateUiText("Value for envelope point 2", "ja"), "エンベロープポイント2の値");
assert.equal(localization.translateUiText("Remove envelope point 2", "ja"), "エンベロープポイント2を削除");
assert.match(
  valueEffectEditorSource,
  /aria-label=\{`\$\{props\.mode\} \$\{props\.interpolation\} value envelope with \$\{props\.points\.length\} points\./,
  "Value FX canvas aria-label must use the localized dynamic envelope contract",
);
for (const dynamicAriaPrefix of [
  "Envelope point ",
  "Select envelope point ",
  "Position for envelope point ",
  "Value for envelope point ",
  "Remove envelope point ",
]) {
  assert.ok(
    valueEffectEditorSource.includes(`aria-label={\`${dynamicAriaPrefix}\${`),
    `Value FX must keep its localized ${dynamicAriaPrefix.trim()} aria-label contract`,
  );
}
assert.equal(localization.translateUiText("Perlin", "ja"), "パーリン");
assert.equal(localization.translateUiText("Live modifier defaults", "ja"), "ライブモディファイア初期値");
assert.equal(localization.translateUiText("Live speed for Cue 新宝島", "ja"), "キュー 新宝島 のライブ速度");
assert.equal(
  localization.translateUiText("Reset live modifier for Cue Amber", "ja"),
  "キュー Amber のライブモディファイアをリセット",
);
assert.equal(localization.translateUiText("Flash Cue Strobe", "ja"), "キュー Strobe をフラッシュ");
assert.equal(localization.translateUiText("Flash mode for White", "ja"), "White のフラッシュモード");
assert.equal(localization.translateUiText("Size", "ja"), "サイズ");
assert.equal(localization.translateUiText("Save", "en"), "Save");

const accessibleSceneBlock = {
  id: 12,
  cue_label: "Opening Wash",
  track: "Lighting",
  time_ms: 1_500,
  duration_ms: 750,
  loop_count: 4,
  total_duration_ms: 3_000,
};
const accessiblePointEvent = {
  id: 13,
  cue_label: "Video Hit",
  track: "Video",
  time_ms: 4_250,
  duration_ms: 0,
  loop_count: 99,
  total_duration_ms: 0,
};
const englishSceneBlockLabel = localization.timelineOverviewMarkerAriaLabel(accessibleSceneBlock, "en");
assert.equal(
  englishSceneBlockLabel,
  "Scene Block #12, Opening Wash, Lighting, starts at 1500 milliseconds, base duration 750 milliseconds, loop count 4, effective span 3000 milliseconds",
);
assert.match(englishSceneBlockLabel, /base duration 750 milliseconds/);
assert.match(englishSceneBlockLabel, /loop count 4/);
assert.match(englishSceneBlockLabel, /effective span 3000 milliseconds/);
assert.equal(
  localization.timelineOverviewMarkerAriaLabel(accessibleSceneBlock, "ja"),
  "シーンブロック #12、Opening Wash、照明、開始 1500ミリ秒、基本時間 750ミリ秒、ループ回数 4、実効範囲 3000ミリ秒",
);
const englishPointEventLabel = localization.timelineOverviewMarkerAriaLabel(accessiblePointEvent, "en");
assert.equal(englishPointEventLabel, "Point event #13, Video Hit, Video, at 4250 milliseconds");
assert.doesNotMatch(englishPointEventLabel, /Block|duration|loop|span/);
assert.equal(
  localization.timelineOverviewMarkerAriaLabel(accessiblePointEvent, "ja"),
  "ポイントイベント #13、Video Hit、映像、4250ミリ秒",
);
assert.match(
  timelineOverviewSource,
  /aria-label=\{props\.markerAriaLabel\(event\)\}/,
  "TimelineOverview must use the tested localized marker formatter",
);
assert.doesNotMatch(
  timelineOverviewSource,
  /aria-label=\{`Block #\$\{event\.id\}/,
  "the old generic Block label must not bypass point/Scene Block semantics",
);
const appSource = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
assert.match(
  appSource,
  /overviewMarkerAriaLabel=\{\(event\) => timelineOverviewMarkerAriaLabel\(event, uiLocale\(\)\)\}/,
  "App must inject the tested uiLocalization formatter into TimelineOverview",
);

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
  "GO", "SET", "FLASH", "HTP", "LTP", "RGB", "BPM", "A", "B", "R", "G", "X", "Y", "Z", "Pan", "Tilt",
  "Art-Net", "sACN", "sACN / E1.31", "sACN E1.31", "Enttec USB PRO", "Syphon Server", "Spout Sender",
  "NDI Sender", "Perlin", "Bezier", "Linear", "Full", "Half", "Scale X", "Scale Y",
  "ms", "x", "U", "ch", "CH", "deg", "m", "Hz", "P", "S", "O", "V", "D", "H", "L",
  "f", "· BUF", "f · CB", "· CB", "C→W", "I/O", "k ·", "kHz", "OVR", "XRUN", "Q",
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
  "ChaserEffectEditorPanel.tsx:preset.label",
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
