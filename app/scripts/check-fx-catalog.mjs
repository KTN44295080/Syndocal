import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const rustSource = await readFile(new URL("../../crates/video/src/builtin_isf.rs", import.meta.url), "utf8");
const load = async (name, transform = (source) => source) => {
  const source = transform(await readFile(new URL(`../src/${name}.ts`, import.meta.url), "utf8"));
  const output = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 } }).outputText;
  return import(`data:text/javascript;base64,${Buffer.from(output).toString("base64")}`);
};
const catalog = await load("fxCatalogModel", (source) => source.replace(/import builtinSource from [^;]+;/, `const builtinSource = ${JSON.stringify(rustSource)};`));
const rejection = await load("fxRejectionExplanation");
const builtins = catalog.buildFxCatalog();
const rustIds = [...rustSource.matchAll(/id:\s*"([^"]+)"/g)].map((match) => match[1]);
assert.deepEqual(builtins.map((entry) => entry.builtinId), rustIds, "every canonical built-in appears once");
for (const entry of builtins) {
  assert.ok(entry.source.includes("void main()"));
  assert.ok(/[\u3040-\u30ff\u4e00-\u9fff]/.test(entry.description.ja), `${entry.label} has Japanese description`);
  assert.notEqual(entry.description.en, entry.description.ja);
  assert.ok(entry.description.en.length > 20);
}
const importedSource = '/*{"DESCRIPTION":"Soft edge wash","CATEGORIES":["Blur"],"INPUTS":[{"NAME":"inputImage","TYPE":"image"}]}*/ void main(){gl_FragColor=vec4(1.0);}';
const imported = { label: "My imported blur", source: importedSource, categories: ["Wrong"], controls: [], enabled: true };
const entries = catalog.buildFxCatalog({ imported: [imported, imported] });
assert.equal(entries.length, builtins.length + 1, "imported source is deduplicated");
const crlfBuiltin = { ...imported, source: builtins[0].source.replaceAll("\n", "\r\n") };
assert.equal(catalog.buildFxCatalog({ imported: [crlfBuiltin] }).length, builtins.length, "native CRLF source does not duplicate Rust LF built-in");
assert.equal(catalog.fxSourceKey(importedSource.replaceAll("\n", "\r\n")), catalog.fxSourceKey(importedSource), "source identity normalizes line endings");
assert.notEqual(catalog.fxSourceKey("costarring"), catalog.fxSourceKey("liquid"), "known FNV-1a collision resolves by full source identity");
assert.equal(catalog.fxSourceKey("costarring"), catalog.fxSourceKey("costarring"), "collision resolution retains stable identity");
assert.equal(catalog.buildFxCatalog({ layers: [{ id: 2, isf_effect: crlfBuiltin }], runtimeErrors: [{ layer_id: 2, stage_index: 0, message: "failed" }] })[0].rejection, "failed", "runtime matching normalizes line endings");
const blur = entries.at(-1);
assert.equal(blur.category, "Blur/Sharpen");
assert.equal(blur.description.en, "Soft edge wash", "metadata is authoritative");
assert.equal(catalog.filterFxCatalog(entries, "edge", "All").at(-1)?.id, blur.id);
assert.equal(catalog.filterFxCatalog(entries, "白黒", "Colour", "ja")[0]?.builtinId, "monochrome");
assert.equal(catalog.filterFxCatalog(entries, "My imported", "Distort").length, 0);
const bare = catalog.buildFxCatalog({ imported: [{ ...imported, source: "void main(){}", description: null, categories: [] }] }).at(-1);
assert.deepEqual(bare.description, { en: "Imported shader", ja: "読み込んだシェーダー" });
const layer = { id: 2, isf_effect: { ...imported, stack: [] } };
assert.equal(catalog.buildFxCatalog({ imported: [imported], layers: [layer], runtimeErrors: [{ layer_id: 2, stage_index: 0, message: "GPU validation failed" }] }).at(-1).rejection, "GPU validation failed");

const nagaSyntax = 'ISF GLSL compile failed: ParseErrors { errors: [Error { kind: InvalidToken(Semicolon, [Rparen]), meta: Span { start: 412, end: 413 } }] }';
const cases = [
  { name: "persistent requires target", source: "", raw: "ISF PERSISTENT pass requires a named TARGET", title: /render pass configuration/, ja: /描画パス/ },
  { name: "target name collision", source: "", raw: "ISF target 'inputImage' is duplicated or conflicts with an input", title: /render pass configuration/, ja: /描画パス/ },
  { name: "invalid pass expression", source: "", raw: "ISF pass size expression is invalid at byte 4: expected ')'", title: /invalid size/, ja: /サイズ/ },
  { name: "oversized pass", source: "", raw: "ISF pass size 1000000x1 is invalid; dimensions must be 1..=8192", title: /invalid size/, ja: /サイズ/ },
  { name: "GPU memory budget", source: "", raw: "ISF pass size exceeds the GPU buffer memory budget", title: /invalid size/, ja: /サイズ/ },
  { name: "missing imported image", source: "", raw: "ISF imported image 'tex' cannot resolve PATH 'missing.png': missing", title: /could not be loaded/, ja: /読み取れません/ },
  { name: "import path traversal", source: "", raw: "ISF imported image 'tex' PATH must stay inside the shader directory", title: /could not be loaded/, ja: /読み取れません/ },
  { name: "import decode", source: "", raw: "ISF imported image 'tex' cannot be decoded within its safety limits: invalid PNG", title: /could not be loaded/, ja: /読み取れません/ },
  { name: "import name collision", source: "", raw: "ISF imported image 'tex' conflicts with an input or target", title: /name is already used/, ja: /重複/ },
  { name: "vertex companion", source: "", raw: "ISF vertex shaders are not supported; companion 'filter.vs' requires a custom vertex stage", title: /Custom vertex shaders/, ja: /頂点シェーダー/ },
  { name: "FLOAT capability", source: "", raw: "ISF FLOAT buffers require 32-bit float texture filtering, which this GPU does not support", title: /cannot use FLOAT/, ja: /FLOATバッファー/ },
  { name: "GLSL syntax error", source: '/*{"INPUTS":[{"NAME":"inputImage","TYPE":"image"}]}*/\nvoid main(){\n gl_FragColor = ;\n}', raw: nagaSyntax, title: /code has an error/, ja: /コードにエラー/ },
  { name: "explicit line", source: "", raw: "GLSL parse error: syntax error at line 7", title: /code has an error/, ja: /コードにエラー/, line: 7 },
  { name: "unsupported INPUT type", source: '/*{\n"INPUTS":[\n{"NAME":"inputImage","TYPE":"image"},\n{"NAME":"warp","TYPE":"color3"}]}*/ void main(){}', raw: "Unsupported ISF input type 'color3'", title: /input type is not supported/, ja: /入力タイプ/, line: 4, input: "color3" },
  { name: "audio texture width", source: '/*{"INPUTS":[{"NAME":"music","TYPE":"audioFFT","MAX":513}]}*/ void main(){}', raw: "ISF audio texture MAX must be an integer from 1 to 512 for input 'music'", title: /Audio texture width/, ja: /音声/, line: 1 },
  { name: "missing PASS target reference", source: '/*{"INPUTS":[{"NAME":"inputImage","TYPE":"image"}]}*/\nvoid main(){\n gl_FragColor = IMG_THIS_PIXEL(bufferA);\n}', raw: 'ISF GLSL compile failed: ParseErrors { errors: [Error { kind: UnknownVariable("bufferA"), meta: Span { start: 1, end: 2 } }] }', title: /pass target is missing/, ja: /ターゲット/, line: 3, input: "bufferA" },
  { name: "forbidden loop", source: '/*{"INPUTS":[]}*/\nvoid main(){\n for (int i=0;i<9;i++){}\n}', raw: "ISF source uses forbidden or unsupported construct 'for ('", title: /not allowed/, ja: /使用できない記述/, line: 3 },
  { name: "invalid header JSON", source: '\n\n/*{\n"INPUTS":[,]}*/ void main(){}', raw: "ISF metadata JSON is invalid: expected value at line 2 column 11", title: /not valid JSON/, ja: /JSON/, line: 4 },
  { name: "shared texture budget", source: "", raw: "ISF declares too many textures; the safety limit is 16", title: /too many textures/, ja: /テクスチャー数/ },
  { name: "image input limit", source: "", raw: "ISF declares too many image inputs; the safety limit is 4", title: /too many image inputs/, ja: /画像入力の数/ },
  { name: "image binding cycle", source: "", raw: "ISF image input dependency cycle: 7 -> 42 -> 7", title: /invalid dependency/, ja: /参照関係/ },
  { name: "runtime image cycle", source: "", raw: "ISF image input cycle detected at layer 7", title: /invalid dependency/, ja: /参照関係/ },
  { name: "image dependency depth", source: "", raw: "ISF image input dependency depth exceeds 32 links", title: /invalid dependency/, ja: /参照関係/ },
  { name: "missing input layer", source: "", raw: "ISF image input references missing layer 42", title: /cannot be identified/, ja: /特定できません/ },
  { name: "unavailable input context", source: "", raw: "ISF image input layer 42 is unavailable in this render context", title: /input layer is unavailable/, ja: /利用できません/ },
  { name: "input produced no frame", source: "", raw: "ISF image input layer 42 did not produce a frame", title: /input layer is unavailable/, ja: /利用できません/ },
  { name: "input render failed", source: "", raw: "ISF image input layer 42 failed to render: GPU error", title: /input layer is unavailable/, ja: /利用できません/ },
  { name: "unknown bound input", source: "", raw: "ISF image binding 'overlay' has no declared image input", title: /binding is invalid/, ja: /割り当て/ },
  { name: "duplicate binding", source: "", raw: "ISF image binding 'overlay' is duplicated", title: /binding is invalid/, ja: /割り当て/ },
  { name: "input memory bound", source: "", raw: "ISF image inputs exceed the GPU buffer memory budget", title: /exceed the memory limit/, ja: /メモリー/ },
  { name: "input bytes", source: "", raw: "ISF image input byte length does not match its dimensions", title: /input image data is invalid/, ja: /データ/ },
  { name: "date clock", source: "", raw: "ISF DATE contains an invalid date or time", title: /render clock/, ja: /時計/ },
  { name: "follow layer scope", source: "", raw: "ISF layer image bindings are unavailable for Timeline Follow transitions; choose an imported still or none", title: /unavailable for this transition/, ja: /トランジション/ },
  // A typo in an ordinary filter that reads its declared image must not be blamed on pass targets.
  { name: "typo in a normal filter", source: '/*{"INPUTS":[{"NAME":"inputImage","TYPE":"image"}]}*/\nvoid main(){\n vec4 c = IMG_THIS_PIXEL(inputImage);\n gl_FragColor = c * foo;\n}', raw: 'ISF GLSL compile failed: ParseErrors { errors: [Error { kind: UnknownVariable("foo"), meta: Span { start: 1, end: 2 } }] }', title: /name that is not defined/, ja: /定義されていない名前/, line: 4, input: "foo" },
  { name: "typo in a PASSES shader", source: '/*{"PASSES":[{}],"INPUTS":[{"NAME":"inputImage","TYPE":"image"}]}*/\nvoid main(){ gl_FragColor = IMG_THIS_PIXEL(inputImage) * gian; }', raw: 'ISF GLSL compile failed: ParseErrors { errors: [Error { kind: UnknownVariable("gian"), meta: Span { start: 1, end: 2 } }] }', title: /name that is not defined/, ja: /定義されていない名前/, line: 2, input: "gian" },
  { name: "unsafe identifier", source: '/*{"INPUTS":[{"NAME":"in put","TYPE":"image"}]}*/ void main(){}', raw: "ISF input name 'in put' is not a safe identifier", title: /input name cannot be used/, ja: /使えない入力名/, input: "in put", line: 1 },
  { name: "invalid default value", source: '/*{"INPUTS":[{"NAME":"inputImage","TYPE":"image"},{"NAME":"gain","TYPE":"float","DEFAULT":"x"}]}*/ void main(){}', raw: "ISF input default/min/max has an unsupported value shape", title: /invalid default or range/, ja: /初期値や範囲/ },
  { name: "GPU pipeline failure", source: '', raw: "ISF GPU pipeline validation failed: device lost", title: /graphics card/, ja: /グラフィックスカード/ },
];
for (const item of cases) {
  const explained = rejection.explainFxRejection(item.raw, "en", item.source);
  assert.match(explained.title, item.title, item.name);
  assert.equal(explained.line, item.line, `${item.name}: line`);
  if (item.input) assert.equal(explained.input, item.input, `${item.name}: input`);
  assert.equal(explained.raw, item.raw, `${item.name}: raw text is preserved`);
  assert.doesNotMatch(explained.title + explained.detail + explained.action, /single-pass filters only|External images are not supported|Generators and transitions are not filters|Exactly one image input is required|Audio inputs are not supported|Syndocal cannot supply one/, `${item.name}: current support boundaries`);
  assert.ok(explained.detail.length > 20 && explained.action.length > 20, `${item.name}: detail and action`);
  const japanese = rejection.explainFxRejection(item.raw, "ja", item.source);
  assert.match(japanese.title, item.ja, `${item.name}: Japanese title`);
  assert.match(japanese.detail + japanese.action, /[぀-ヿ一-鿿]/, `${item.name}: Japanese body`);
  const rejected = catalog.buildFxCatalog({ rejections: [{ label: "bad.fs", source: item.source, message: item.raw }] }).at(-1);
  assert.equal(rejected.rejection, item.raw);
}
assert.match(rejection.explainFxRejection("Something unforeseen").title, /cannot be used/, "unknown failures keep a generic explanation and raw text");
const data = new Map();
const transfer = { setData: (key, value) => data.set(key, value), getData: (key) => data.get(key) ?? "" };
catalog.writeFxDragPayload(transfer, builtins[0]);
assert.equal(catalog.readFxDragPayload(transfer, builtins)?.id, builtins[0].id);
transfer.setData(catalog.FX_DRAG_MIME, "untrusted source");
assert.equal(catalog.readFxDragPayload(transfer, builtins), undefined);
transfer.setData(catalog.FX_DRAG_MIME, builtins[0].id);
assert.equal(catalog.readFxDragPayload(transfer, [{ ...builtins[0], rejection: "bad" }]), undefined);
const css = await readFile(new URL("../src/components/VideoEffectBrowser.css", import.meta.url), "utf8");
assert.doesNotMatch(css, /#[\da-f]{3,8}\b|rgba?\(|hsla?\(/i, "catalog chrome uses theme tokens");
assert.match(css, /min-height:\s*44px/);
console.log(`FX catalog: ${builtins.length} canonical built-ins, import metadata/search, runtime states, drag validation and ${cases.length} bilingual rejection fixtures passed.`);
