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
    test: (raw) => /ISF image input binding update (?:rejected|was not confirmed)/i.test(raw),
    title: word("The image binding was not saved", "画像入力の割り当てが保存されませんでした"),
    detail: () => word("The project did not confirm the image-input change. The previous binding remains in use.", "プロジェクトが画像入力の変更を確定できなかったため、以前の割り当てを使います。"),
    action: word("Check the project status, then choose the image source again.", "プロジェクトの状態を確認してから、画像の入力元を選び直してください。"),
  },
  {
    test: (raw) => /ISF image input (?:dependency (?:cycle|depth)|cycle detected)/i.test(raw),
    title: word("The image inputs form an invalid dependency", "画像入力の参照関係を使用できません"),
    detail: () => word("A layer would read its own output directly or indirectly, or the dependency chain exceeds the limit.", "レイヤーが自分の出力を直接・間接的に参照しているか、参照の段数が上限を超えています。"),
    action: word("Choose an independent layer, an imported still, or None.", "依存していないレイヤーか、読み込み画像、「なし」を選んでください。"),
  },
  {
    test: (raw) => /ISF image input (?:layer.*(?:missing|identity is duplicated)|references missing layer)/i.test(raw),
    title: word("The input layer cannot be identified", "入力元のレイヤーを特定できません"),
    detail: () => word("The saved layer ID is missing or duplicated. Layer order is not used to choose an input.", "保存されたレイヤーIDが存在しないか、重複しています。入力元はレイヤーの並び順では決まりません。"),
    action: word("Choose an existing layer with a unique ID, an imported still, or None.", "IDが重複していないレイヤーか、読み込み画像、「なし」を選んでください。"),
  },
  {
    test: (raw) => /ISF layer image bindings are unavailable for Timeline Follow/i.test(raw),
    title: word("Layer images are unavailable for this transition", "このトランジションではレイヤー画像を利用できません"),
    detail: () => word("Timeline Follow transitions do not have a project layer output context for image bindings.", "Timeline Followトランジションでは、画像入力に必要なプロジェクトのレイヤー出力を参照できません。"),
    action: word("Choose an imported still or None for this image input.", "この画像入力には、読み込み画像か「なし」を選んでください。"),
  },
  {
    test: (raw) => /ISF image input layer.*(?:unavailable in this render context|did not produce a frame|failed to render)/i.test(raw),
    title: word("The input layer is unavailable", "入力元のレイヤーを利用できません"),
    detail: () => word("This view has no usable output from the selected layer. The layer may have no frame or an effect may have failed.", "この表示には入力元のレイヤーの有効な出力がありません。映像がないか、エフェクトの処理に失敗している可能性があります。"),
    action: word("Check the layer in project Output or Preview, or choose an imported still or None.", "プロジェクトの出力かプレビューで入力元を確認するか、読み込み画像または「なし」を選んでください。"),
  },
  {
    test: (raw) => /ISF image inputs exceed the GPU buffer memory budget/i.test(raw),
    title: word("The input images exceed the memory limit", "入力画像のメモリー使用量が上限を超えています"),
    detail: () => word("The combined input textures need more GPU memory than this effect is allowed to allocate.", "入力テクスチャーの合計サイズが、このエフェクトで使えるGPUメモリーの上限を超えています。"),
    action: word("Use smaller source images or lower the source layer resolution.", "入力画像を小さくするか、入力元のレイヤーの解像度を下げてください。"),
  },
  {
    test: (raw) => /ISF image input byte length does not match its dimensions/i.test(raw),
    title: word("The input image data is invalid", "入力画像のデータが正しくありません"),
    detail: () => word("The image byte count does not match the declared width and height.", "画像のデータ量が、指定された幅と高さに一致していません。"),
    action: word("Check or reload the source layer, or choose another input image.", "入力元のレイヤーを確認して読み込み直すか、別の入力画像を選んでください。"),
  },
  {
    test: (raw) => /ISF DATE contains an invalid date or time/i.test(raw),
    title: word("The render clock returned an invalid date", "描画用の時計が無効な日時を返しました"),
    detail: () => word("DATE needs a valid year, month, day and seconds since midnight.", "DATEには、有効な年月日と、午前0時からの経過秒数が必要です。"),
    action: word("Check the render clock in the technical details and restart the render with a valid date.", "技術的な詳細で描画用の時計を確認し、有効な日時で描画をやり直してください。"),
  },
  {
    test: (raw) => /ISF declares too many image inputs|ISF image input.*(?:limit exceeded|at most|maximum|safety limit)|ISF shader.*image inputs.*limit/i.test(raw),
    title: word("There are too many image inputs", "画像入力の数が上限を超えています"),
    detail: () => word("An effect supports at most four image inputs in total. The layer supplies the primary image by default.", "1つのエフェクトで使える画像入力は合計4つまでです。元画像は通常、レイヤーから供給されます。"),
    action: word("Reduce the image inputs or remove unused image bindings.", "画像入力を減らすか、使っていない割り当てを取り除いてください。"),
  },
  {
    test: (raw) => /ISF image (?:input|binding).*(?:name is invalid|duplicated|unknown|undeclared|primary|not declared|not an additional|no declared|binding|already has an imported still)/i.test(raw),
    title: word("An image input binding is invalid", "画像入力の割り当てが正しくありません"),
    detail: () => word("Each binding must name a declared image input exactly once. The selected source must match that input.", "割り当てには、宣言済みの画像入力の名前を重複なく指定してください。選択した入力元はその画像入力に対応する必要があります。"),
    action: word("Select the image input again and remove duplicate or obsolete bindings.", "画像入力を選び直し、重複した割り当てや不要な割り当てを取り除いてください。"),
  },
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
    test: (raw) => /ISF pass size/i.test(raw),
    title: word("A render pass has an invalid size", "描画パスのサイズが正しくありません"),
    detail: () => word("A WIDTH or HEIGHT expression is invalid, or the requested buffers exceed the dimension or memory limits.", "WIDTH・HEIGHTの式が正しくないか、バッファーのサイズやメモリー使用量が上限を超えています。"),
    action: word("Check the WIDTH and HEIGHT expressions, use positive dimensions and reduce the buffer sizes. Open the technical details for the exact limit.", "WIDTH・HEIGHTの式を確認し、正のサイズを指定してバッファーを小さくしてください。具体的な上限は技術的な詳細で確認できます。"),
  },
  {
    test: (raw) => /ISF FLOAT buffers require/i.test(raw),
    title: word("The graphics card cannot use FLOAT buffers", "このグラフィックスカードではFLOATバッファーを使えません"),
    detail: () => word("The shader requires filtering of 32-bit floating-point textures, which this graphics adapter does not support.", "シェーダーには32ビット浮動小数点テクスチャーのフィルタリングが必要ですが、このグラフィックスカードは対応していません。"),
    action: word("Use a graphics adapter with 32-bit float texture filtering, or use a shader designed for ordinary image buffers.", "32ビット浮動小数点テクスチャーのフィルタリングに対応したGPUか、通常の画像バッファー用のシェーダーを使ってください。"),
  },
  {
    test: (raw) => /ISF vertex shaders are not supported/i.test(raw),
    title: word("Custom vertex shaders are not supported", "独自の頂点シェーダーには対応していません"),
    detail: () => word("This ISF file has a companion vertex shader. Syndocal renders ISF filters with its own full-screen vertex stage.", "このISFファイルには頂点シェーダーが付属しています。Syndocalは独自の全画面用頂点ステージでISFフィルターを描画します。"),
    action: word("Use a fragment-only version of the effect whose output does not depend on the companion vertex shader.", "付属の頂点シェーダーに依存しない、フラグメントシェーダーだけの版を使ってください。"),
  },
  {
    test: (raw) => /ISF (?:PERSISTENT pass|pass (?:PERSISTENT|FLOAT)|target\b)|target-buffer|multi-pass|\bpass(es)?\b.*not supported/i.test(raw),
    title: word("The render pass configuration cannot be used", "描画パスの設定を使用できません"),
    detail: () => word("A pass flag or TARGET declaration is invalid or unsupported. Multiple passes and persistent named buffers are supported within the runtime limits.", "パスのフラグやTARGETの宣言が正しくないか、対応していない設定です。複数パスと名前付きの永続バッファーは、実行時の上限内で使えます。"),
    action: word("Check the technical details. Give persistent passes a TARGET, use unique target names and correct the pass flags before importing again.", "技術的な詳細を確認してください。永続パスにTARGETを指定し、重複しないターゲット名と正しいフラグに修正して、もう一度読み込んでください。"),
  },
  {
    test: (raw) => /ISF imported image.*conflicts with/i.test(raw),
    title: word("An imported image name is already used", "読み込み画像の名前が重複しています"),
    detail: () => word("An IMPORTED name conflicts with an input or pass target.", "IMPORTEDの名前が入力またはパスのターゲットと重複しています。"),
    action: word("Give the imported image a unique name and update its IMG_* references.", "読み込み画像に重複しない名前を付け、IMG_*の参照も変更してください。"),
  },
  {
    test: (raw) => /ISF imported image|imported image resources/i.test(raw),
    title: word("An imported image could not be loaded", "読み込み画像を読み取れませんでした"),
    detail: () => word("An IMPORTED resource is missing, has an unsafe path or cannot be decoded.", "IMPORTEDの画像が見つからない、パスが安全な範囲外にある、または画像を解析できない状態です。"),
    action: word("Import the shader from its original folder with its image files. Check each relative PATH and use PNG or JPEG.", "画像ファイルが揃った元のフォルダーからシェーダーを読み込んでください。各PATHの相対パスを確認し、PNGかJPEGを使ってください。"),
  },
  {
    test: (raw, source) => /unknown ?variable|undeclared identifier/i.test(raw) && isReadAsImage(source, unknownName(raw)),
    title: word("A referenced image or pass target is missing", "参照している画像やパスのターゲットが見つかりません"),
    detail: () => word("The referenced image is not declared in INPUTS, PASSES or IMPORTED.", "参照している画像がINPUTS・PASSES・IMPORTEDのどこにも宣言されていません。"),
    action: word("Declare the image in INPUTS or IMPORTED, or add the intended named PASS TARGET. Check the spelling of its IMG_* references.", "画像をINPUTSかIMPORTEDで宣言するか、対応する名前付きのPASS TARGETを追加してください。IMG_*の参照名も確認してください。"),
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
    ?? raw.match(/ISF imported image\s+'([^']+)'/i)?.[1]
    ?? raw.match(/ISF target\s+'([^']+)'/i)?.[1]
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
