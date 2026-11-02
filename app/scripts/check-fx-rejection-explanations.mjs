import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const source = await readFile(new URL("../src/fxRejectionExplanation.ts", import.meta.url), "utf8");
const compiled = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
}).outputText;
const { explainFxRejection } = await import(`data:text/javascript;base64,${Buffer.from(compiled).toString("base64")}`);
const corpus = JSON.parse(await readFile(new URL("../../crates/video/tests/fixtures/isf-corpus-rejections.json", import.meta.url), "utf8"));
assert.equal(corpus.length, 6);

// The Rust corpus asserts the engine emits these exact raw diagnostics. This
// contract asserts their complete operator-facing EN/JA cards, not keywords.
const expected = {
  uniform: {
    en: ["Declare shader inputs in the ISF header", "The code declares a raw GLSL uniform (line 2). Syndocal can supply inputs declared in the ISF header.", "Remove the uniform declaration and add its supported input type to INPUTS in the ISF header."],
    ja: ["シェーダーの入力はISFヘッダーで宣言してください", "コードにGLSLのuniform宣言があります（2行目付近）。Syndocalが供給できるのはISFヘッダーで宣言した入力です。", "uniform宣言を取り除き、対応する入力タイプをISFヘッダーのINPUTSへ追加してください。"],
    line: 2,
  },
  passes: {
    en: ["The shader has too many render passes", "An ISF shader supports at most 16 passes. Display output also checks the total render workload and available memory.", "Reduce the PASSES list or split the effect into smaller shaders."],
    ja: ["シェーダーの描画パス数が上限を超えています", "ISFシェーダーの描画パスは16個までです。Display出力では、描画処理の合計と利用可能なメモリーも確認します。", "PASSESの項目を減らすか、エフェクトを小さなシェーダーに分けてください。"],
    line: 1,
  },
  path: {
    en: ["An imported image could not be loaded", "An IMPORTED resource is missing, has an unsafe path or cannot be decoded.", "Import the shader from its original folder with its image files. Check each relative PATH and use PNG or JPEG."],
    ja: ["読み込み画像を読み取れませんでした", "IMPORTEDの画像が見つからない、パスが安全な範囲外にある、または画像を解析できない状態です。", "画像ファイルが揃った元のフォルダーからシェーダーを読み込んでください。各PATHの相対パスを確認し、PNGかJPEGを使ってください。"],
    line: 1, input: "noise",
  },
};
for (const fixture of corpus) {
  const card = fixture.kind === "directive" ? {
    en: ["This GLSL directive is not supported", `The code uses ${fixture.token} (line 2). Syndocal supplies the shader version, graphics capabilities and input declarations.`, "Use an ISF fragment shader without #version, #extension or #include, and declare supported inputs in its ISF header."],
    ja: ["このGLSLディレクティブには対応していません", `コードに${fixture.token}があります（2行目付近）。シェーダーのバージョン・GPU機能・入力宣言はSyndocalが用意します。`, "#version・#extension・#includeを使わないISFフラグメントシェーダーを用意し、対応する入力をISFヘッダーで宣言してください。"],
    line: 2,
  } : expected[fixture.kind];
  for (const locale of ["en", "ja"]) {
    const [title, detail, action] = card[locale];
    assert.deepEqual(explainFxRejection(fixture.raw, locale, fixture.source), {
      title, detail, action, line: card.line, input: card.input, raw: fixture.raw,
    }, `${fixture.name}: exact ${locale} explanation`);
  }
}
console.log("ISF corpus rejection cards: 6 exact engine diagnostics × EN/JA passed.");
