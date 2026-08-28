import { parseStrictJson } from "../../app/scripts/strict-json.mjs";

export class ShowAuthorError extends Error {
  constructor(message) {
    super(`BLOCKED: ${message}`);
    this.name = "ShowAuthorError";
  }
}

export function fail(message) {
  throw new ShowAuthorError(message);
}

export function isObject(value) {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

export function exactKeys(value, expected, label) {
  if (!isObject(value)) fail(`${label} must be an object`);
  const actual = Object.keys(value).sort();
  const wanted = [...expected].sort();
  if (actual.length !== wanted.length || actual.some((key, index) => key !== wanted[index])) {
    fail(`${label} shape differs; expected keys ${wanted.join(",")}`);
  }
}

export function stableValue(value) {
  if (Array.isArray(value)) return value.map((entry) => stableValue(entry));
  if (isObject(value)) {
    const output = {};
    for (const key of Object.keys(value).sort()) output[key] = stableValue(value[key]);
    return output;
  }
  return value;
}

export function equalJson(left, right) {
  return JSON.stringify(stableValue(left)) === JSON.stringify(stableValue(right));
}

export function assertEqual(actual, expected, label) {
  if (!equalJson(actual, expected)) fail(`${label} differs from the canonical expected value`);
}

export function assertNumber(value, label) {
  if (typeof value !== "number" || !Number.isFinite(value)) fail(`${label} must be a finite number`);
}

export function assertSafeInteger(value, label) {
  if (!Number.isSafeInteger(value)) fail(`${label} must be a safe integer`);
}

export function round9(value) {
  return Math.round(value * 1_000_000_000) / 1_000_000_000;
}

export function decodeUtf8Strict(bytes, label) {
  try {
    // Preserve a byte-order mark so the strict JSON scanner can reject it;
    // JSON text is not accepted with a BOM in this authoring contract.
    return new TextDecoder("utf-8", { fatal: true, ignoreBOM: true }).decode(bytes);
  } catch (error) {
    fail(`${label} is not valid UTF-8: ${String(error?.message ?? error)}`);
  }
}

export function parseJsonText(text, label) {
  try {
    return parseStrictJson(text, label);
  } catch (error) {
    fail(String(error?.message ?? error));
  }
}

export function parseJsonValue(input, label) {
  if (typeof input === "string") return parseJsonText(input, label);
  if (!isObject(input)) fail(`${label} must be a parsed JSON object or strict JSON text`);
  return input;
}
