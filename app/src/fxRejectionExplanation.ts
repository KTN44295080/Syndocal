export interface FxRejectionExplanation {
  /** One-line plain-language summary of what is wrong. */
  title: string;
  /** What the shader or runtime did, including the input or line when known. */
  detail: string;
  /** What the operator can do next. */
  action: string;
  /** 1-based line in the imported file when it can be established. */
  line?: number;
  input?: string;
  /** The unmodified engine text, kept for experts. */
  raw: string;
}

type Words = { en: string; ja: string };
interface Rule {
  test: (raw: string, source: string) => boolean;
  title: Words;
  detail: (found: Found) => Words;
  action: Words;
}
interface Found { input?: string; line?: number; count?: string; token?: string; source: string }

const word = (en: string, ja: string): Words => ({ en, ja });
const at = (found: Found): Words => found.line ? word(` (line ${found.line})`, `（${found.line}行目付近）`) : word("", "");
const named = (found: Found, name = found.input): Words => name ? word(` "${name}"`, `「${name}」`) : word("", "");

/** First 1-based line of `needle` in `source`, ignoring case. `bodyOnly` skips the ISF header comment. */
function lineOf(source: string, needle: string, bodyOnly = false): number | undefined {
  if (!source || !needle) return undefined;
  const bodyStart = bodyOnly ? Math.max(0, source.indexOf("*/") + 2) : 0;
  const index = source.toLowerCase().indexOf(needle.toLowerCase(), bodyStart);
  return index < 0 ? undefined : source.slice(0, index).split(/\r\n|\r|\n/).length;
}

const unknownName = (raw: string) => raw.match(/UnknownVariable\("([^"]+)"\)/)?.[1] ?? raw.match(/unknown variable\s+['"]?([A-Za-z_]\w*)/i)?.[1];
/** True when `name` is the image argument of an IMG_* call: the shader reads an image it never declared. */
const isReadAsImage = (source: string, name?: string) => Boolean(name) && new RegExp(`IMG_\\w+\\s*\\(\\s*${name}\\b`).test(source);

/** The ISF header JSON starts after the first slash-star, so its line 1 is that line. */
function headerLine(source: string, jsonLine: number): number | undefined {
  const open = source.indexOf("/*");
  if (open < 0) return undefined;
  const afterOpen = source.slice(open + 2);
  const leading = afterOpen.length - afterOpen.trimStart().length;
  const firstLine = source.slice(0, open + 2 + leading).split(/\r\n|\r|\n/).length;
  return firstLine + jsonLine - 1;
}

const rules: Rule[] = [
  {
    test: (raw) => /audio\/audioFFT|audioFFT|"audio"/.test(raw) && /not supported|unsupported/i.test(raw),
    title: word("Audio inputs are not supported", "音声の入力には対応していません"),
    detail: (found) => word(`The shader asks for an audio texture${named(found, found.input ?? "audio").en}. Syndocal cannot supply one.`, `シェーダーが音声テクスチャー${named(found, found.input ?? "audio").ja}を要求していますが、Syndocalでは扱えません。`),
    action: word("Remove the audio input, or drive the shader's controls from Syndocal's Audio FFT nodes instead.", "音声の入力を取り除くか、Syndocalの音声ノード（Audio FFT）でパラメーターを動かしてください。"),
  },
  {
    test: (raw) => /unsupported isf input type/i.test(raw),
    title: word("This input type is not supported", "この入力タイプには対応していません"),
    detail: (found) => word(`INPUTS contains a TYPE${named(found).en}${at(found).en} that Syndocal cannot supply.`, `INPUTSに、Syndocalで扱えないTYPE${named(found).ja}があります${at(found).ja}。`),
    action: word("Use only image, float, bool, long, point2D, color and event inputs, remove the unsupported INPUT, then import again.", "使えるのは image・float・bool・long・point2D・color・event です。対応していないINPUTを取り除いて、もう一度読み込んでください。"),
  },
  {
    test: (raw) => /target-buffer|persistent|multi-pass|\bpass(es)?\b.*not supported/i.test(raw),
    title: word("The shader needs render passes Syndocal cannot run", "複数パスやバッファーを使うシェーダーは実行できません"),
    detail: (found) => word(`PASSES uses a TARGET buffer, a persistent buffer or more than one pass${at(found).en}. Syndocal runs single-pass filters only.`, `PASSESがTARGETのバッファー・永続バッファー・複数パスのいずれかを使っています${at(found).ja}。Syndocalは1パスのフィルターだけを実行できます。`),
    action: word("Merge the work into one pass without TARGET or PERSISTENT, or import a single-pass version of the shader.", "TARGETやPERSISTENTを使わず1パスにまとめるか、1パス版のシェーダーを読み込んでください。"),
  },
  {
    test: (raw, source) => /unknown ?variable|undeclared identifier/i.test(raw) && isReadAsImage(source, unknownName(raw)),
    title: word("A referenced image or pass target is missing", "参照している画像やパスのターゲットが見つかりません"),
    detail: (found) => word(`The shader reads an image${named(found).en} that is not an INPUT of type image and is not a PASS TARGET${at(found).en}.`, `シェーダーが画像${named(found).ja}を読み取っていますが、TYPEがimageのINPUTにも、PASSESのTARGETにもありません${at(found).ja}。`),
    action: word("Declare it as an image INPUT, or rewrite the shader so it reads only its single image input.", "TYPEがimageのINPUTとして宣言するか、画像入力1つだけを読むように書き直してください。"),
  },
  {
    test: (raw) => /unknown ?variable|undeclared identifier/i.test(raw),
    title: word("The shader uses a name that is not defined", "定義されていない名前を使っています"),
    detail: (found) => word(`The code refers to${named(found).en || " a name"}${at(found).en} that is not an INPUT, a built-in or a local variable.`, `コードが${named(found).ja || "ある名前"}を参照していますが${at(found).ja}、INPUT・組み込み変数・ローカル変数のどれにもありません。`),
    action: word("Check the spelling against INPUTS and the code, declare the missing INPUT, then import again.", "INPUTSやコードのつづりを確認し、足りないINPUTを追加して、もう一度読み込んでください。"),
  },
  {
    test: (raw) => /forbidden or unsupported construct/i.test(raw),
    title: word("The shader uses code that is not allowed", "使用できない記述が含まれています"),
    detail: (found) => word(`The code contains ${found.token ? `"${found.token}"` : "a construct"}${at(found).en}. Loops, uniform declarations, #version and similar constructs are blocked to keep live output safe.`, `コードに${found.token ? `「${found.token}」` : "使用できない記述"}があります${at(found).ja}。ライブ映像の安全のため、ループ・uniform宣言・#versionなどは許可されていません。`),
    action: word("Rewrite that part without loops or low-level declarations, or use another version of the shader.", "その部分をループや低レベルな宣言を使わない形に書き換えるか、別の版のシェーダーを使ってください。"),
  },
  {
    test: (raw) => /exactly one image input/i.test(raw),
    title: word("Exactly one image input is required", "画像の入力はちょうど1つ必要です"),
    detail: (found) => word(`INPUTS must contain one input of TYPE image (found ${found.count ?? "a different number"}). Generators and transitions are not filters.`, `INPUTSにはTYPEがimageの入力がちょうど1つ必要です（見つかった数：${found.count ?? "不明"}）。ジェネレーターや切り替え用のシェーダーはフィルターとして使えません。`),
    action: word("Add or keep a single image INPUT, then import again.", "TYPEがimageのINPUTを1つだけにして、もう一度読み込んでください。"),
  },
  {
    test: (raw) => /not a safe identifier/i.test(raw),
    title: word("An input name cannot be used", "使えない入力名があります"),
    detail: (found) => word(`The INPUT name${named(found).en}${at(found).en} is reserved or contains characters other than letters, digits and underscores.`, `入力名${named(found).ja}${at(found).ja}は予約語か、英数字と_以外の文字を含んでいます。`),
    action: word("Rename it to start with a letter or underscore and use only letters, digits and underscores.", "英字か_で始まり、英数字と_だけの名前に変更してください。"),
  },
  {
    test: (raw) => /is duplicated/i.test(raw),
    title: word("Two inputs share one name", "同じ名前の入力が重複しています"),
    detail: (found) => word(`More than one INPUT is called${named(found).en}${at(found).en}.`, `${named(found).ja}という名前のINPUTが複数あります${at(found).ja}。`),
    action: word("Give each INPUT its own NAME, then import again.", "それぞれのINPUTに別のNAMEを付けて、もう一度読み込んでください。"),
  },
  {
    test: (raw) => /non-finite|unsupported value shape|outside the supported range|must contain only numbers/i.test(raw),
    title: word("An input has an invalid default or range", "入力の初期値や範囲の指定が正しくありません"),
    detail: (found) => word(`A DEFAULT, MIN or MAX value${named(found).en}${at(found).en} is not a finite number or a number list Syndocal understands.`, `DEFAULT・MIN・MAXの値${named(found).ja}${at(found).ja}が、有限の数値または対応する数値リストではありません。`),
    action: word("Use plain numbers in INPUTS (an [r, g, b, a] list for colours, [x, y] for points), then import again.", "INPUTSでは通常の数値を使ってください（色は[r, g, b, a]、点は[x, y]の配列）。修正して、もう一度読み込んでください。"),
  },
  {
    test: (raw) => /imported image resources/i.test(raw),
    title: word("External images are not supported", "外部の画像ファイルには対応していません"),
    detail: (found) => word(`IMPORTED loads image files${at(found).en}. Syndocal cannot read them.`, `IMPORTEDで画像ファイルを読み込んでいます${at(found).ja}。Syndocalでは読み取れません。`),
    action: word("Remove IMPORTED, or bake the image into the shader's code.", "IMPORTEDを取り除くか、画像の内容をシェーダーのコードに組み込んでください。"),
  },
  {
    test: (raw) => /safety limit|controls; the safety limit/i.test(raw),
    title: word("The shader is larger than the safety limit", "シェーダーが安全上の上限を超えています"),
    detail: () => word("The file or its number of controls exceeds what Syndocal accepts for live use.", "ファイルの大きさか操作項目の数が、ライブで使える上限を超えています。"),
    action: word("Remove unused code or controls, or split the effect into smaller shaders.", "不要なコードや操作項目を減らすか、エフェクトを小さなシェーダーに分けてください。"),
  },
  {
    test: (raw) => /must define void main/i.test(raw),
    title: word("The main function is missing", "main関数がありません"),
    detail: (found) => word(`The code does not define void main()${at(found).en}.`, `コードに void main() がありません${at(found).ja}。`),
    action: word("Add void main() that writes gl_FragColor, then import again.", "gl_FragColorへ出力する void main() を追加して、もう一度読み込んでください。"),
  },
  {
    test: (raw) => /missing its top metadata comment|must be the first source element|metadata comment is not terminated/i.test(raw),
    title: word("The ISF header is missing or misplaced", "ISFのヘッダーがない、または位置が正しくありません"),
    detail: () => word("An ISF file must start with a /* { ... } */ comment that holds its settings.", "ISFのファイルは、設定を書いた /* { ... } */ コメントで始まる必要があります。"),
    action: word("Place the header comment at the very top and close it with */, then import again.", "ヘッダーのコメントをファイルの先頭に置き、*/ で閉じて、もう一度読み込んでください。"),
  },
  {
    test: (raw) => /metadata json is invalid|invalid.*json|\bjson\b.*(?:expected|invalid)/i.test(raw),
    title: word("The ISF header is not valid JSON", "ISFのヘッダーがJSONとして正しくありません"),
    detail: (found) => word(`The settings inside /* ... */ cannot be read${at(found).en}.`, `/* ... */ の中の設定を読み取れません${at(found).ja}。`),
    action: word("Check commas, quotation marks and brackets in the header, then import again.", "ヘッダーのカンマ・引用符・括弧を確認して、もう一度読み込んでください。"),
  },
  {
    test: (raw) => /\bgpu\b|readback|wgpu/i.test(raw),
    title: word("The graphics card could not run the effect", "グラフィックスカードでエフェクトを実行できませんでした"),
    detail: () => word("Syndocal could not use the graphics adapter to render this shader.", "このシェーダーを描画するためのグラフィックスカードを使えませんでした。"),
    action: word("Update the graphics driver and restart Syndocal. If it persists, the shader may be too heavy for this GPU.", "グラフィックスドライバーを更新して、Syndocalを再起動してください。解決しない場合は、このGPUには重いシェーダーかもしれません。"),
  },
  {
    test: (raw) => /syntax|parse|compil|glsl|unexpected|expected|validation failed|translation failed|invalidtoken/i.test(raw),
    title: word("The shader code has an error", "シェーダーのコードにエラーがあります"),
    detail: (found) => word(`The GLSL could not be compiled${at(found).en}${found.token ? `: ${found.token}` : ""}.`, `GLSLをコンパイルできませんでした${at(found).ja}${found.token ? `：${found.token}` : ""}。`),
    action: word("Check brackets, semicolons and variable types near the reported spot, then import again. Technical details may point to the cause; line numbers can refer to generated code.", "報告された箇所の近くで、括弧・セミコロン・変数の型を確認して、もう一度読み込んでください。技術的な詳細に原因の手がかりがあります。行番号は変換後のコードを指すことがあります。"),
  },
];

const generic: Rule = {
  test: () => true,
  title: word("This effect cannot be used", "このエフェクトは使用できません"),
  detail: () => word("Importing or rendering failed. The effect cannot be added.", "読み込みまたは描画でエラーが発生しました。このエフェクトは追加できません。"),
  action: word("Open the technical details, fix the ISF file and import it again.", "技術的な詳細を確認し、ISFファイルを修正して、もう一度読み込んでください。"),
};

/** Classifies import/runtime diagnostics into plain language while keeping the original text. */
export function explainFxRejection(raw: string, locale = "en", source = ""): FxRejectionExplanation {
  const lang = locale === "ja" ? "ja" : "en";
  const inputName = raw.match(/(?:ISF )?input(?: name)?\s+'([^']+)'/i)?.[1]
    ?? unknownName(raw)
    ?? raw.match(/input type\s+'([^']+)'/i)?.[1]
    ?? raw.match(/TARGET\s+'([^']+)'/)?.[1]
    ?? (/audioFFT/.test(raw) ? "audioFFT" : undefined);
  const token = raw.match(/construct\s+'([^']+)'/i)?.[1];
  const count = raw.match(/found\s+(\d+)/i)?.[1];
  // Prefer an explicit "line N" from the engine; the JSON parser's line is relative to the header.
  let line: number | undefined;
  const explicit = raw.match(/\bline\s*[:=]?\s*(\d+)/i) ?? raw.match(/(?:ERROR:\s*)?\d+:(\d+):/);
  if (explicit) line = /metadata json is invalid/i.test(raw) && source ? headerLine(source, Number(explicit[1])) ?? Number(explicit[1]) : Number(explicit[1]);
  if (line === undefined && source) {
    if (token) line = lineOf(source, token, true);
    else if (unknownName(raw)) line = lineOf(source, unknownName(raw)!, true);
    else if (inputName) line = lineOf(source, `"${inputName}"`) ?? lineOf(source, inputName);
  }
  const found: Found = { input: inputName, line, count, token, source };
  const rule = rules.find((candidate) => candidate.test(raw, source)) ?? generic;
  return { title: rule.title[lang], detail: rule.detail(found)[lang], action: rule.action[lang], line, input: inputName, raw };
}

export interface FxProblem {
  key: string;
  origin: "runtime" | "import";
  /** Where it happened, e.g. "Layer 2 · Strobe Wash · stage 1"; empty for an import without a layer. */
  where: string;
  explanation: FxRejectionExplanation;
}

interface FxProblemInputs {
  layers: readonly { id: number; label: string; isf_effect?: { source: string; stack?: readonly { source: string }[] } | null }[];
  runtimeErrors: readonly { layer_id: number; stage_index?: number | null; stage_label?: string | null; message: string }[];
  importRejections: readonly { label: string; source?: string; message: string }[];
  locale?: string;
}

/** Every explained effect problem the operator can act on, for the media health area. */
export function collectFxProblems({ layers, runtimeErrors, importRejections, locale = "en" }: FxProblemInputs): FxProblem[] {
  const ja = locale === "ja";
  const problems: FxProblem[] = [];
  runtimeErrors.forEach((error, index) => {
    const layer = layers.find((candidate) => candidate.id === error.layer_id);
    const stages = layer?.isf_effect ? [layer.isf_effect, ...(layer.isf_effect.stack ?? [])] : [];
    const stageIndex = error.stage_index ?? 0;
    const source = stages[stageIndex]?.source ?? "";
    const stage = error.stage_label ? ` · ${error.stage_label}` : "";
    problems.push({
      key: `runtime:${error.layer_id}:${stageIndex}:${index}`, origin: "runtime",
      where: `${ja ? "レイヤー" : "Layer"} ${error.layer_id}${layer ? ` · ${layer.label}` : ""}${stage}`,
      explanation: explainFxRejection(error.message, locale, source),
    });
  });
  importRejections.forEach((rejection, index) => problems.push({
    key: `import:${index}:${rejection.message}`, origin: "import", where: rejection.label,
    explanation: explainFxRejection(rejection.message, locale, rejection.source ?? ""),
  }));
  return problems;
}
