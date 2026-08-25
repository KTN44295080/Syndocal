import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

export const strictJsonMaxDepth = 1000;

export class StrictJsonError extends Error {
  constructor(message, detail = message) {
    super(message);
    this.name = "StrictJsonError";
    this.detail = detail;
  }
}

function isHexadecimalDigit(unit) {
  return (unit >= 0x30 && unit <= 0x39) || (unit >= 0x41 && unit <= 0x46) || (unit >= 0x61 && unit <= 0x66);
}

function isDecimalDigit(unit) {
  return unit >= 0x30 && unit <= 0x39;
}

function describeUnit(unit) {
  if (Number.isNaN(unit)) return "end of input";
  const character = String.fromCharCode(unit);
  if (unit < 0x20 || unit === 0x7f) return `control character U+${unit.toString(16).padStart(4, "0")}`;
  return JSON.stringify(character);
}

// Full RFC 8259 grammar validation plus duplicate-key rejection at every object
// depth. Keys are compared after escape decoding, so "a" and "\u0061" collide.
// Strings are decoded exactly, so braces, escapes, and structural characters
// inside strings can never affect nesting or key identity.
export function scanStrictJson(text, label = "JSON document") {
  if (typeof text !== "string") {
    throw new StrictJsonError(
      `${label} must be a string before strict scanning; received ${typeof text}.`,
      `${label} must be a string before strict scanning; received ${typeof text}.`,
    );
  }
  const length = text.length;
  let pos = 0;

  const fail = (detail) => {
    throw new StrictJsonError(`${detail} (offset ${pos})`, `${detail} (offset ${pos})`);
  };

  const skipWhitespace = () => {
    while (pos < length) {
      const unit = text.charCodeAt(pos);
      if (unit === 0x20 || unit === 0x09 || unit === 0x0a || unit === 0x0d) pos += 1;
      else break;
    }
  };

  const scanString = () => {
    pos += 1;
    let decoded = "";
    for (;;) {
      if (pos >= length) fail("unterminated string; the closing '\"' is missing");
      const unit = text.charCodeAt(pos);
      if (unit === 0x22) {
        pos += 1;
        return decoded;
      }
      if (unit === 0x5c) {
        pos += 1;
        if (pos >= length) fail("unterminated escape sequence at end of input");
        const escape = text.charCodeAt(pos);
        if (escape === 0x22) decoded += "\"";
        else if (escape === 0x5c) decoded += "\\";
        else if (escape === 0x2f) decoded += "/";
        else if (escape === 0x62) decoded += "\b";
        else if (escape === 0x66) decoded += "\f";
        else if (escape === 0x6e) decoded += "\n";
        else if (escape === 0x72) decoded += "\r";
        else if (escape === 0x74) decoded += "\t";
        else if (escape === 0x75) {
          if (pos + 4 >= length) fail("truncated \\u escape; exactly four hexadecimal digits are required");
          let value = 0;
          for (let index = 1; index <= 4; index += 1) {
            const digit = text.charCodeAt(pos + index);
            if (!isHexadecimalDigit(digit)) fail(`invalid \\u escape digit ${describeUnit(digit)}`);
            value = value * 16 + (digit <= 0x39 ? digit - 0x30 : digit <= 0x46 ? digit - 0x37 : digit - 0x57);
          }
          decoded += String.fromCharCode(value);
          pos += 4;
        } else {
          fail(`invalid escape sequence \\${String.fromCharCode(escape)}; only \" \\ / b f n r t u are permitted`);
        }
        pos += 1;
        continue;
      }
      if (unit < 0x20) fail(`raw control character U+${unit.toString(16).padStart(4, "0")} inside a string; a \\u escape is required`);
      decoded += text[pos];
      pos += 1;
    }
  };

  const scanLiteral = () => {
    if (text.startsWith("true", pos)) {
      pos += 4;
      return;
    }
    if (text.startsWith("false", pos)) {
      pos += 5;
      return;
    }
    if (text.startsWith("null", pos)) {
      pos += 4;
      return;
    }
    fail(`invalid literal starting with ${JSON.stringify(text.slice(pos, pos + 5))}; only true, false, and null are permitted`);
  };

  const scanNumber = (path) => {
    const start = pos;
    if (text.charCodeAt(pos) === 0x2d) pos += 1;
    const lead = text.charCodeAt(pos);
    if (!isDecimalDigit(lead)) {
      pos = start;
      fail(`unexpected ${describeUnit(text.charCodeAt(start))}; expected a JSON value at ${path}`);
    }
    if (lead === 0x30) {
      pos += 1;
    } else {
      while (pos < length && isDecimalDigit(text.charCodeAt(pos))) pos += 1;
    }
    if (text.charCodeAt(pos) === 0x2e) {
      pos += 1;
      if (!isDecimalDigit(text.charCodeAt(pos))) fail("a decimal digit is required after '.' in a number");
      while (pos < length && isDecimalDigit(text.charCodeAt(pos))) pos += 1;
    }
    const exponent = text.charCodeAt(pos);
    if (exponent === 0x65 || exponent === 0x45) {
      pos += 1;
      const sign = text.charCodeAt(pos);
      if (sign === 0x2b || sign === 0x2d) pos += 1;
      if (!isDecimalDigit(text.charCodeAt(pos))) fail("a decimal digit is required in a number exponent");
      while (pos < length && isDecimalDigit(text.charCodeAt(pos))) pos += 1;
    }
  };

  const scanObject = (path, depth) => {
    if (depth > strictJsonMaxDepth) fail(`maximum nesting depth ${strictJsonMaxDepth} exceeded at ${path}`);
    pos += 1;
    skipWhitespace();
    if (text.charCodeAt(pos) === 0x7d) {
      pos += 1;
      return;
    }
    const seenKeys = new Set();
    for (;;) {
      skipWhitespace();
      if (text.charCodeAt(pos) !== 0x22) {
        fail(
          pos >= length
            ? `unexpected end of input inside the object at ${path}; an object key string is required`
            : `expected an object key string at ${path} but found ${describeUnit(text.charCodeAt(pos))}`,
        );
      }
      const key = scanString();
      const keyPath = `${path}[${JSON.stringify(key)}]`;
      if (seenKeys.has(key)) {
        fail(`duplicate object key ${JSON.stringify(key)} at ${keyPath}; duplicates are rejected before any authoritative document is trusted`);
      }
      seenKeys.add(key);
      skipWhitespace();
      if (text.charCodeAt(pos) !== 0x3a) fail(`expected ':' after the object key ${keyPath}`);
      pos += 1;
      scanValue(keyPath, depth + 1);
      skipWhitespace();
      const unit = text.charCodeAt(pos);
      if (unit === 0x2c) {
        pos += 1;
        continue;
      }
      if (unit === 0x7d) {
        pos += 1;
        return;
      }
      fail(
        pos >= length
          ? `unexpected end of input inside the object at ${path}; expected ',' or '}'`
          : `expected ',' or '}' in the object at ${path} but found ${describeUnit(unit)}`,
      );
    }
  };

  const scanArray = (path, depth) => {
    if (depth > strictJsonMaxDepth) fail(`maximum nesting depth ${strictJsonMaxDepth} exceeded at ${path}`);
    pos += 1;
    skipWhitespace();
    if (text.charCodeAt(pos) === 0x5d) {
      pos += 1;
      return;
    }
    for (let index = 0;; index += 1) {
      scanValue(`${path}[${index}]`, depth + 1);
      skipWhitespace();
      const unit = text.charCodeAt(pos);
      if (unit === 0x2c) {
        pos += 1;
        continue;
      }
      if (unit === 0x5d) {
        pos += 1;
        return;
      }
      fail(
        pos >= length
          ? `unexpected end of input inside the array at ${path}; expected ',' or ']'`
          : `expected ',' or ']' in the array at ${path} but found ${describeUnit(unit)}`,
      );
    }
  };

  function scanValue(path, depth) {
    skipWhitespace();
    if (pos >= length) fail(`unexpected end of input; a JSON value is required at ${path}`);
    switch (text.charCodeAt(pos)) {
      case 0x7b:
        return scanObject(path, depth);
      case 0x5b:
        return scanArray(path, depth);
      case 0x22:
        scanString();
        return undefined;
      case 0x74:
      case 0x66:
      case 0x6e:
        scanLiteral();
        return undefined;
      default:
        scanNumber(path);
        return undefined;
    }
  }

  scanValue("$", 1);
  skipWhitespace();
  if (pos !== length) {
    fail(`unexpected trailing content after the root value: ${JSON.stringify(text.slice(pos, pos + 24))}`);
  }
}

export function parseStrictJson(text, label = "JSON document") {
  try {
    scanStrictJson(text, label);
  } catch (error) {
    if (error instanceof StrictJsonError) {
      throw new StrictJsonError(`${label} is not valid strict JSON: ${error.detail}`, error.detail);
    }
    throw error;
  }
  let parsed;
  try {
    parsed = JSON.parse(text);
  } catch (error) {
    const detail = `${label} passed the strict duplicate-key scan but was rejected by JSON.parse (${String(error)}); failing closed instead of trusting a divergent parser.`;
    throw new StrictJsonError(detail, detail);
  }
  return parsed;
}

const guardedProductionFiles = Object.freeze([
  "windows-runtime-inventory.mjs",
  "check-windows-release-artifacts.mjs",
  "check-release-metadata.mjs",
]);

async function runStrictJsonSelfTest() {
  const scriptDir = dirname(fileURLToPath(import.meta.url));
  let assertions = 0;
  const pass = (condition, message) => {
    assert.ok(condition, "strict-json self-test failure: " + message);
    assertions += 1;
  };
  const rejectsAsStrictJsonError = (action, pattern, message) => {
    let thrown;
    try {
      action();
    } catch (error) {
      thrown = error;
    }
    pass(thrown instanceof StrictJsonError, message + " must fail with StrictJsonError");
    pass(pattern.test(thrown?.detail ?? ""), message + ` detail must match ${pattern}; got: ${thrown?.detail ?? "(none)"}`);
  };
  const acceptsEqually = (text, message) => {
    let parsed;
    try {
      parsed = parseStrictJson(text, "self-test");
    } catch (error) {
      throw new Error(`strict-json self-test failure: ${message} was rejected unexpectedly: ${error.message}`);
    }
    const baseline = JSON.parse(text);
    pass(JSON.stringify(parsed) === JSON.stringify(baseline), message + " must parse to exactly what JSON.parse produces");
  };

  const validDocuments = [
    ["empty object", "{}"],
    ["empty array", "[]"],
    ["empty string value", '[\"\"]'],
    ["simple object", '{"a":1,"b":2}'],
    ["same key in sibling objects", '{"a":{"k":1},"b":{"k":1}}'],
    ["same key across array elements", '[{"k":1},{"k":1},{"k":1}]'],
    ["braces and structural characters inside strings", '{"s":"}{[]:,\\\"\\\\"}'],
    ["escaped and raw unicode keys", '{"\\u0041\\u0042":"x","AB2":"y","名前":"例"}'],
    ["lone surrogate escape", '{"\\uDC00 alone":1}'],
    ["case-distinct keys", '{"a":1,"A":2}'],
    ["literal backslash-u key is not an escape", '{"\\\\u0041":1,"A":2}'],
    ["empty and space keys are distinct", '{"":1," ":2,"a b":3}'],
    ["prototype-looking keys stay inert", '{"__proto__":{"x":1},"constructor":2}'],
    ["numbers", "[0,-0,0.5,-12.75,1e10,1E-10,1e+3,123456789,0.0001]"],
    ["large numbers lose precision identically to JSON.parse", "[12345678901234567890]"],
    ["nested arrays and mixed values", '[[[]],{"n":[null,true,false,-0.5e-2]},"text"]'],
    ["whitespace torture", " \t\r\n{\t\"a\"\r:\n1 }\t\n\r "],
    ["moderately deep nesting parses identically", `${"[".repeat(200)}1${"]".repeat(200)}`],
  ];
  for (const [message, text] of validDocuments) acceptsEqually(text, message);

  const duplicateRejections = [
    ["duplicate top-level key", '{"schema_version":1,"platform":"x","schema_version":9}', /duplicate object key "schema_version"/],
    ["duplicate nested key", '{"outer":{"x":1,"x":2}}', /duplicate object key "x" at \$\["outer"\]\["x"\]/],
    ["duplicate inside array element objects", '{"list":[{"a":1},{"b":{"c":1,"c":2}}]}', /duplicate object key "c"/],
    ["escape-equivalent duplicate key", '{"a":1,"\\u0061":2}', /duplicate object key "a"/],
    ["slash-equivalent duplicate key", '{"\\/":1,"/":2}', /duplicate object key "\/"/],
    ["astral-equivalent duplicate key", '{"😀":1,"\\uD83D\\uDE00":2}', /duplicate object key "😀"/],
    ["triple duplicate key", '{"k":1,"k":2,"k":3}', /duplicate object key "k"/],
    ["duplicate key after a nested close", '{"o":{"z":1},"o":2}', /duplicate object key "o"/],
    ["duplicate __proto__ key", '{"__proto__":1,"__proto__":2}', /duplicate object key "__proto__"/],
    ["duplicate key regardless of value equality", '{"sha256":"aa","note":1,"sha256":"aa"}', /duplicate object key "sha256"/],
  ];
  for (const [message, text, pattern] of duplicateRejections) {
    rejectsAsStrictJsonError(() => scanStrictJson(text, "self-test"), pattern, message + " must be rejected by the scanner");
    rejectsAsStrictJsonError(() => parseStrictJson(text, "self-test"), pattern, message + " must be rejected by the parser");
  }

  const grammarRejections = [
    ["trailing comma in object", '{"a":1,}', /expected an object key string/],
    ["unquoted object key", "{a:1}", /expected an object key string/],
    ["missing colon", '{"a" 1}', /expected ':'/],
    ["missing comma between members", '{"a":1 "b":2}', /expected ',' or '\}'/],
    ["trailing comma in array", "[1,]", /expected a JSON value/],
    ["single-quoted string", "['x']", /unexpected "'"/],
    ["NaN literal", "[NaN]", /unexpected "N"/],
    ["Infinity literal", "[Infinity]", /unexpected "I"/],
    ["leading zero number", "01", /unexpected trailing content/],
    ["trailing decimal point", "[1.]", /digit is required after/],
    ["bare decimal point", "[.5]", /unexpected "\."/],
    ["plus-signed number", "[+1]", /unexpected "\+"/],
    ["double negative", "[--1]", /unexpected "-"/],
    ["exponent without digits", "[1e]", /digit is required in a number exponent/],
    ["exponent sign without digits", "[1e+]", /digit is required in a number exponent/],
    ["unterminated string", '{"unclosed', /unterminated string/],
    ["invalid escape", '"bad\\x00escape"', /invalid escape sequence/],
    ["truncated unicode escape", '"ab\\u00"', /truncated \\u escape|invalid \\u escape/],
    ["raw tab inside string", '"a\tb"', /raw control character/],
    ["raw newline inside string", '"a\nb"', /raw control character/],
    ["trailing garbage after root", "{} extra", /unexpected trailing content/],
    ["empty input", "", /unexpected end of input/],
    ["whitespace-only input", "   ", /unexpected end of input/],
    ["byte-order mark prefix", "\uFEFF{}", /unexpected "(?:\\uFEFF|\uFEFF)"/u],
    ["true misspelled with suffix", "truex", /unexpected trailing content/],
    ["truncated object", '{"a":', /unexpected end of input/],
    ["unterminated escape at end", '"abc\\', /unterminated escape sequence/],
  ];
  for (const [message, text, pattern] of grammarRejections) {
    rejectsAsStrictJsonError(() => scanStrictJson(text, "self-test"), pattern, message + " must be rejected");
  }

  rejectsAsStrictJsonError(
    () => scanStrictJson(`${"[".repeat(strictJsonMaxDepth + 1)}1${"]".repeat(strictJsonMaxDepth + 1)}`, "self-test"),
    /maximum nesting depth/,
    "nesting beyond strictJsonMaxDepth must be rejected",
  );
  scanStrictJson(`${"[".repeat(strictJsonMaxDepth)}1${"]".repeat(strictJsonMaxDepth)}`, "self-test");
  assertions += 1;
  rejectsAsStrictJsonError(
    () => scanStrictJson(42, "self-test"),
    /must be a string/,
    "non-string input must be rejected",
  );

  const workspaceRoot = resolve(scriptDir, "../..");
  const readRepoFile = (relativePath) => readFileSync(resolve(workspaceRoot, relativePath), "utf8");

  for (const name of guardedProductionFiles) {
    const source = readFileSync(join(scriptDir, name), "utf8");
    pass(!/\bJSON\s*\.\s*parse\s*\(/u.test(source), name + " must contain no bare JSON.parse; every authoritative parse routes through strict-json");
    pass(source.includes('from "./strict-json.mjs"'), name + " must import its authoritative JSON parsing from ./strict-json.mjs");
  }

  acceptsEqually(readRepoFile("qa/FFMPEG_WINDOWS_RUNTIME_INVENTORY.json"), "the live Windows runtime inventory");

  const { validateAsioPackagingBoundary, validateStaticReleaseMetadata, parseRuntimeUpdaterIdentity } = await import("./check-release-metadata.mjs");

  const injectDuplicateTopLevelKey = (text) => {
    const parsed = JSON.parse(text);
    const key = Object.keys(parsed)[0];
    const evilValue = typeof parsed[key] === "string" ? 0 : "syndocal-duplicate-key-probe";
    const at = text.indexOf("{");
    if (at < 0) throw new Error("strict-json self-test fixture bug: no opening brace in " + JSON.stringify(text.slice(0, 40)));
    return text.slice(0, at + 1) + JSON.stringify(key) + ":" + JSON.stringify(evilValue) + "," + text.slice(at + 1);
  };

  for (const target of ["app/package.json", "app/src-tauri/tauri.conf.json", "app/src-tauri/tauri.updater.conf.json"]) {
    const duplicatedText = injectDuplicateTopLevelKey(readRepoFile(target));
    assert.throws(
      () => validateStaticReleaseMetadata((path) => (path === target ? duplicatedText : readRepoFile(path))),
      (error) => error instanceof Error && /duplicate object key/.test(error.message),
      `${target} duplicate top-level key must be rejected before validateStaticReleaseMetadata trusts any field`,
    );
    assertions += 1;
  }
  for (const target of ["qa/ASIO_SDK_PIN.json", "app/src-tauri/tauri.windows.conf.json"]) {
    const duplicatedText = injectDuplicateTopLevelKey(readRepoFile(target));
    assert.throws(
      () => validateAsioPackagingBoundary((path) => (path === target ? duplicatedText : readRepoFile(path)), { verifyWindowsRuntimeSources: false }),
      (error) => error instanceof Error && /duplicate object key/.test(error.message),
      `${target} duplicate top-level key must be rejected through parseRequiredJson`,
    );
    assertions += 1;
  }

  assert.throws(
    () =>
      parseRuntimeUpdaterIdentity(
        '{"endpoint":"https://releases.example.invalid/x","endpoint":"https://releases.example.invalid/y","channel":"beta","publicKeyFingerprint":"' + "a".repeat(64) + '"}',
      ),
    (error) => error instanceof Error && /duplicate object key "endpoint"/.test(error.message),
    "runtime updater diagnostic duplicate endpoint must be rejected",
  );
  assertions += 1;

  console.log("strict JSON duplicate-key self-test passed: " + assertions + " assertions");
}

async function main(argv = process.argv.slice(2)) {
  if (argv.length === 1 && argv[0] === "--self-test") {
    await runStrictJsonSelfTest();
    return;
  }
  throw new Error("Usage: strict-json.mjs --self-test");
}

if (resolve(process.argv[1] ?? "") === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error(error);
    process.exitCode = 1;
  });
}
