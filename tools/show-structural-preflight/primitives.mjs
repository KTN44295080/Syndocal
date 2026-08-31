export const SHOW_TITLE_CONTAINS = "人生オーバー";
export const SHOW_RELEASE_TRIGGER = "F13";
export const SHOW_FOLLOW_BAR_MILLIUNITS = 1_000;
export const TIMELINE_TEMPO_METER_MAP_VERSION = 1;
const FOLLOW_VIDEO_KINDS = Object.freeze([
  "Cut",
  "Crossfade",
  "Dip",
  "Wipe",
  "Luma",
  "Displacement",
  "Blur",
  "Glitch",
  "Custom",
]);
const FOLLOW_CURVES = Object.freeze(["Linear", "EaseIn", "EaseOut", "EaseInOut"]);
const FOLLOW_LIGHTING_POLICIES = Object.freeze(["hold_then_cut", "linear_merge"]);
const FOLLOW_FAULT_POLICIES = Object.freeze(["hold", "cut", "fault"]);
const DJ_RETRIGGER_POLICIES = Object.freeze(["once_per_play_session"]);
const DJ_LINK_MAX_MAPPINGS = 128;
const TIMELINE_TEMPO_METER_MAX_POINTS = 4096;
const TIMELINE_TEMPO_METER_MAX_SIXTEENTH_STEPS = 4_611_686_018_427_387_903n;
const TIMELINE_TEMPO_METER_MAX_MEASURE = 1_152_921_504_606_846_975n;
const TIMELINE_FOLLOW_MAX_RESOLVED_DURATION_MS = 600_000;
const TIMELINE_FOLLOW_MAX_DURATION_BEAT_MILLIUNITS = 200_000;
const TIMELINE_FOLLOW_MAX_DURATION_BAR_MILLIUNITS = 50_000;
const TIMELINE_FOLLOW_MAX_PREROLL_MS = TIMELINE_FOLLOW_MAX_RESOLVED_DURATION_MS;
const TIMELINE_FOLLOW_MAX_TRANS_CADENCE_BARS = 256;
const TIMELINE_FOLLOW_MAX_TRANS_TARGETS = 64;
const DJ_SELECTOR_KEYS = new Set([
  "contentId",
  "title",
  "artist",
  "titleContains",
  "fallbackDeck",
]);
const DJ_MAPPING_KEYS = new Set(["id", "selector", "timelineId", "retrigger"]);

const REQUIRED_CHECK_IDS = Object.freeze([
  "sdc_shape",
  "dj_mapping",
  "explicit_ids",
  "active_bank_projection",
  "adjacent_bank_entries",
  "follow",
  "source_measure_transition",
  "destination_first_measure",
  "destination_pedal_wait",
  "source_loop",
]);

const FINITE_LOOP_KEYS = Object.freeze([
  "loopTotalPasses",
  "loopAddedBeats",
  "repeatCount",
  "repeat_count",
  "repeat_count_beats",
  "passes",
  "totalPasses",
  "loop_count",
]);

export {
  FOLLOW_VIDEO_KINDS,
  FOLLOW_CURVES,
  FOLLOW_LIGHTING_POLICIES,
  FOLLOW_FAULT_POLICIES,
  DJ_RETRIGGER_POLICIES,
  DJ_LINK_MAX_MAPPINGS,
  TIMELINE_TEMPO_METER_MAX_POINTS,
  TIMELINE_TEMPO_METER_MAX_SIXTEENTH_STEPS,
  TIMELINE_TEMPO_METER_MAX_MEASURE,
  TIMELINE_FOLLOW_MAX_RESOLVED_DURATION_MS,
  TIMELINE_FOLLOW_MAX_DURATION_BEAT_MILLIUNITS,
  TIMELINE_FOLLOW_MAX_DURATION_BAR_MILLIUNITS,
  TIMELINE_FOLLOW_MAX_PREROLL_MS,
  TIMELINE_FOLLOW_MAX_TRANS_CADENCE_BARS,
  TIMELINE_FOLLOW_MAX_TRANS_TARGETS,
  DJ_SELECTOR_KEYS,
  DJ_MAPPING_KEYS,
  REQUIRED_CHECK_IDS,
  FINITE_LOOP_KEYS,
};

export const readOwn = (value, camel, snake = camel) => {
  if (!isObject(value)) return undefined;
  if (Object.prototype.hasOwnProperty.call(value, camel)) return value[camel];
  if (snake !== camel && Object.prototype.hasOwnProperty.call(value, snake)) return value[snake];
  return undefined;
};

export const hasOwn = (value, key) => isObject(value) && Object.prototype.hasOwnProperty.call(value, key);

export function unknownKeys(value, allowedKeys) {
  if (!isObject(value)) return [];
  return Object.keys(value).filter((key) => !allowedKeys.has(key));
}

export function isObject(value) {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

export function isSafePositiveInteger(value) {
  return Number.isSafeInteger(value) && value > 0;
}

export function isSafeNonNegativeInteger(value) {
  return Number.isSafeInteger(value) && value >= 0;
}

export function isFinitePositiveNumber(value) {
  return typeof value === "number" && Number.isFinite(value) && value > 0;
}

export function isUtf8WireString(value) {
  if (typeof value !== "string" || value.length === 0) return false;
  if (new TextEncoder().encode(value).byteLength > 256) return false;
  return !/[\u0000-\u001f\u007f-\u009f]/u.test(value);
}

export function compact(value, normalizeUnicode = false) {
  if (typeof value !== "string") return "";
  const trimmed = value.trim();
  return normalizeUnicode ? trimmed.normalize("NFC") : trimmed;
}

export function resultCheck(id, passed, detail, evidence = "authored") {
  return {
    id,
    status: passed ? "PASS" : "BLOCKED",
    evidence,
    detail,
  };
}

export function blocked(id, detail) {
  return resultCheck(id, false, detail);
}

export function passed(id, detail, evidence = "authored") {
  return resultCheck(id, true, detail, evidence);
}

function stableValue(value) {
  if (Array.isArray(value)) return value.map((entry) => stableValue(entry));
  if (isObject(value)) {
    const sorted = {};
    for (const key of Object.keys(value).sort()) sorted[key] = stableValue(value[key]);
    return sorted;
  }
  return value;
}

export function stableJson(value) {
  return JSON.stringify(stableValue(value));
}
