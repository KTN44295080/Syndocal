import {
  DJ_MAPPING_KEYS,
  DJ_LINK_MAX_MAPPINGS,
  DJ_RETRIGGER_POLICIES,
  DJ_SELECTOR_KEYS,
  SHOW_TITLE_CONTAINS,
  blocked,
  compact,
  isObject,
  isSafePositiveInteger,
  isUtf8WireString,
  passed,
  readOwn,
  unknownKeys,
} from "./primitives.mjs";

function parseSelector(mapping, index) {
  const selector = mapping?.selector;
  if (!isObject(selector)) {
    return { error: `dj_track_triggers[${index}].selector must be an object` };
  }
  const unknown = unknownKeys(selector, DJ_SELECTOR_KEYS);
  if (unknown.length > 0) {
    return {
      error: `dj_track_triggers[${index}].selector has unknown field(s): ${unknown.join(", ")}; Rust serde requires exact camelCase selector keys`,
    };
  }

  const contentId = compact(readOwn(selector, "contentId"));
  const title = compact(readOwn(selector, "title"), true);
  const artist = compact(readOwn(selector, "artist"), true);
  const titleContains = compact(readOwn(selector, "titleContains"), true);
  const fallbackValue = readOwn(selector, "fallbackDeck");
  const fallbackDeck = fallbackValue === null || fallbackValue === undefined ? null : fallbackValue;

  if (fallbackDeck !== null && fallbackDeck !== 1) {
    return { error: `dj_track_triggers[${index}] fallbackDeck must be exactly 1 when present` };
  }
  if (titleContains) {
    if (contentId || title || artist) {
      return { error: `dj_track_triggers[${index}] titleContains cannot include exact selector fields` };
    }
    if (!isUtf8WireString(titleContains)) {
      return { error: `dj_track_triggers[${index}] titleContains is not a valid bounded wire string` };
    }
    return {
      mode: "title_contains",
      canonicalKey: `title_contains:${titleContains}`,
      titleContains,
      fallbackDeck,
    };
  }
  if (fallbackDeck !== null) {
    return { error: `dj_track_triggers[${index}] fallbackDeck requires titleContains` };
  }
  if (contentId) {
    if (title || artist || !isUtf8WireString(contentId)) {
      return { error: `dj_track_triggers[${index}] contentId selector is malformed` };
    }
    return {
      mode: "content",
      canonicalKey: `content:${contentId}`,
      titleContains: null,
      fallbackDeck: null,
    };
  }
  if (!title || !artist || !isUtf8WireString(title) || !isUtf8WireString(artist)) {
    return { error: `dj_track_triggers[${index}] requires contentId or title+artist` };
  }
  return {
    mode: "title_artist",
    canonicalKey: `title_artist:${title}\u001f${artist}`,
    title,
    artist,
    titleContains: null,
    fallbackDeck: null,
  };
}

export function mappingSummary(project, authoredTimelineIds = null) {
  const mappings = project?.dj_track_triggers;
  if (!Array.isArray(mappings)) {
    return {
      check: blocked("dj_mapping", "Project root is missing the authored dj_track_triggers array"),
      mappings: [],
      production: [],
      error: "Project root is missing the authored dj_track_triggers array",
    };
  }
  if (mappings.length === 0) {
    return {
      check: blocked("dj_mapping", "dj_track_triggers is empty; the explicit production title mapping is missing"),
      mappings: [],
      production: [],
      error: "dj_track_triggers is empty",
    };
  }

  const ids = new Set();
  const selectors = new Set();
  const parsed = [];
  const errors = [];
  if (mappings.length > DJ_LINK_MAX_MAPPINGS) {
    errors.push(`DJ track mapping count exceeds ${DJ_LINK_MAX_MAPPINGS}`);
  }
  for (const [index, mapping] of mappings.entries()) {
    if (!isObject(mapping)) {
      errors.push(`dj_track_triggers[${index}] must be an object`);
      continue;
    }
    const unknown = unknownKeys(mapping, DJ_MAPPING_KEYS);
    if (unknown.length > 0) {
      errors.push(
        `dj_track_triggers[${index}] has unknown field(s): ${unknown.join(", ")}; Rust serde requires exact mapping keys`,
      );
      continue;
    }
    const id = compact(mapping.id);
    if (!isUtf8WireString(id) || ids.has(id)) {
      errors.push(`dj_track_triggers[${index}] id must be non-empty and unique`);
    }
    ids.add(id);
    const selector = parseSelector(mapping, index);
    if (selector.error) {
      errors.push(selector.error);
      continue;
    }
    if (selectors.has(selector.canonicalKey)) {
      errors.push(`dj_track_triggers[${index}] selector is a duplicate`);
    }
    selectors.add(selector.canonicalKey);
    const retrigger = readOwn(mapping, "retrigger");
    if (retrigger !== undefined && !DJ_RETRIGGER_POLICIES.includes(retrigger)) {
      errors.push(`dj_track_triggers[${index}] retrigger must be the Rust serde value once_per_play_session`);
    }
    const timelineId = readOwn(mapping, "timelineId");
    if (!isSafePositiveInteger(timelineId)) {
      errors.push(`dj_track_triggers[${index}] timelineId must be an explicit positive integer`);
    } else if (authoredTimelineIds && !authoredTimelineIds.has(timelineId)) {
      errors.push(`dj_track_triggers[${index}] timelineId ${timelineId} must reference an existing authored Timeline ID`);
    }
    parsed.push({ index, id, timelineId, ...selector });
  }
  const production = parsed.filter((mapping) =>
    mapping.mode === "title_contains" && mapping.titleContains === SHOW_TITLE_CONTAINS,
  );
  if (production.length !== 1) {
    errors.push(
      `expected exactly one titleContains=${SHOW_TITLE_CONTAINS} mapping with fallbackDeck=1; found ${production.length}`,
    );
  } else if (production[0].fallbackDeck !== 1) {
    errors.push(`the ${SHOW_TITLE_CONTAINS} mapping must explicitly set fallbackDeck=1`);
  }
  const fallbackMappings = parsed.filter((mapping) => mapping.fallbackDeck === 1);
  if (fallbackMappings.length > 1) {
    errors.push(`fallbackDeck=1 must identify exactly one mapping; found ${fallbackMappings.length}`);
  } else if (fallbackMappings.length === 1
      && (production.length !== 1 || fallbackMappings[0] !== production[0])) {
    errors.push(`fallbackDeck=1 must be reserved for the unique ${SHOW_TITLE_CONTAINS} production mapping`);
  }
  const normalizedProductionTitle = SHOW_TITLE_CONTAINS.normalize("NFC");
  const additionalTitleContains = parsed.filter((mapping) =>
    mapping.mode === "title_contains" && mapping.titleContains !== normalizedProductionTitle,
  );
  for (const mapping of additionalTitleContains) {
    errors.push(`only titleContains=${SHOW_TITLE_CONTAINS} may be authored for this show; titleContains=${mapping.titleContains} is an additional selector and would make runtime matching ambiguous`);
  }
  const overlappingExactTitles = parsed.filter((mapping) =>
    mapping.mode === "title_artist"
      && mapping.title.includes(normalizedProductionTitle),
  );
  for (const mapping of overlappingExactTitles) {
    errors.push(`exact title+artist selector title=${mapping.title} overlaps production titleContains=${SHOW_TITLE_CONTAINS}; runtime primary matching would be ambiguous`);
  }

  return {
    check: errors.length === 0
      ? passed("dj_mapping", `one unique titleContains=${SHOW_TITLE_CONTAINS} mapping explicitly falls back to Deck 1`)
      : blocked("dj_mapping", errors.join("; ")),
    mappings: parsed,
    production,
    error: errors.length === 0 ? null : errors.join("; "),
  };
}
