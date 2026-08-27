import type { DjTrackSelector, DjTrackTriggerMapping } from "./types";

export const DEFAULT_DJ_TITLE_CONTAINS = "人生オーバー";

export type DjTrackSelectorMode = "content" | "title_artist" | "title_contains";

export interface DjTrackSelectorDraft {
  mode: DjTrackSelectorMode;
  contentId: string;
  title: string;
  artist: string;
  titleContains: string;
  fallbackDeck1: boolean;
}

export type DjTrackMappingPolicyResult<T> =
  | { ok: true; value: T }
  | { ok: false; error: string };

const MAX_MAPPING_COUNT = 128;
const MAX_STRING_BYTES = 256;
const CONTROL_CHARACTER = /[\u0000-\u001f\u007f-\u009f]/u;
const UTF8 = new TextEncoder();

const compact = (value: string | null | undefined, normalizeUnicode = false) => {
  const trimmed = value?.trim() ?? "";
  return normalizeUnicode ? trimmed.normalize("NFC") : trimmed;
};

const validWireString = (value: string) => value.length > 0
  && UTF8.encode(value).byteLength <= MAX_STRING_BYTES
  && !CONTROL_CHARACTER.test(value);

interface NormalizedSelector {
  selector: DjTrackSelector;
  canonicalKey: string;
  mode: DjTrackSelectorMode;
}

function normalizeSelector(selector: DjTrackSelector): DjTrackMappingPolicyResult<NormalizedSelector> {
  const contentId = compact(selector.contentId);
  const title = compact(selector.title, true);
  const artist = compact(selector.artist, true);
  const titleContains = compact(selector.titleContains, true);
  const fallbackDeck = selector.fallbackDeck ?? null;

  if (titleContains) {
    if (contentId || title || artist) {
      return { ok: false, error: "Title contains cannot be combined with exact track selectors." };
    }
    if (fallbackDeck !== null && fallbackDeck !== 1) {
      return { ok: false, error: "Title contains fallback must be Deck 1." };
    }
    if (!validWireString(titleContains)) {
      return { ok: false, error: "DJ Link mapping values must be 1–256 UTF-8 bytes without control characters." };
    }
    return {
      ok: true,
      value: {
        mode: "title_contains",
        canonicalKey: `title_contains:${titleContains}`,
        selector: {
          contentId: null,
          title: null,
          artist: null,
          titleContains,
          fallbackDeck,
        },
      },
    };
  }

  if (fallbackDeck !== null) {
    return { ok: false, error: "Deck 1 fallback requires Title contains." };
  }
  if (contentId) {
    if (title || artist) {
      return { ok: false, error: "Content ID cannot be combined with Title or Artist." };
    }
    if (!validWireString(contentId)) {
      return { ok: false, error: "DJ Link mapping values must be 1–256 UTF-8 bytes without control characters." };
    }
    return {
      ok: true,
      value: {
        mode: "content",
        canonicalKey: `content:${contentId}`,
        selector: {
          contentId,
          title: null,
          artist: null,
          titleContains: null,
          fallbackDeck: null,
        },
      },
    };
  }

  if (!title || !artist) {
    return { ok: false, error: "Each DJ Link mapping needs one Content ID, exact Title + Artist pair, or Title contains selector." };
  }
  if (!validWireString(title) || !validWireString(artist)) {
    return { ok: false, error: "DJ Link mapping values must be 1–256 UTF-8 bytes without control characters." };
  }
  return {
    ok: true,
    value: {
      mode: "title_artist",
      canonicalKey: `title_artist:${title}\u001f${artist}`,
      selector: {
        contentId: null,
        title,
        artist,
        titleContains: null,
        fallbackDeck: null,
      },
    },
  };
}

export function djTrackSelectorMode(selector: DjTrackSelector): DjTrackSelectorMode | null {
  const normalized = normalizeSelector(selector);
  return normalized.ok ? normalized.value.mode : null;
}

export function buildDjTrackSelector(
  draft: DjTrackSelectorDraft,
): DjTrackMappingPolicyResult<DjTrackSelector> {
  const selector: DjTrackSelector = draft.mode === "content"
    ? {
      contentId: draft.contentId,
      title: null,
      artist: null,
      titleContains: null,
      fallbackDeck: null,
    }
    : draft.mode === "title_artist"
      ? {
        contentId: null,
        title: draft.title,
        artist: draft.artist,
        titleContains: null,
        fallbackDeck: null,
      }
      : {
        contentId: null,
        title: null,
        artist: null,
        titleContains: draft.titleContains,
        fallbackDeck: draft.fallbackDeck1 ? 1 : null,
      };
  const normalized = normalizeSelector(selector);
  return normalized.ok
    ? { ok: true, value: normalized.value.selector }
    : normalized;
}

export function normalizeDjTrackTriggerMappings(
  mappings: DjTrackTriggerMapping[],
): DjTrackMappingPolicyResult<DjTrackTriggerMapping[]> {
  if (mappings.length > MAX_MAPPING_COUNT) {
    return { ok: false, error: "DJ Link mappings must contain at most 128 unique IDs." };
  }

  const ids = new Set<string>();
  const selectors = new Set<string>();
  const normalized: DjTrackTriggerMapping[] = [];
  for (const mapping of mappings) {
    const id = mapping.id.trim();
    if (!validWireString(id) || ids.has(id)) {
      return { ok: false, error: "DJ Link mappings must contain at most 128 unique IDs." };
    }
    if (!Number.isSafeInteger(mapping.timelineId) || mapping.timelineId <= 0) {
      return { ok: false, error: "Choose an authored Timeline target." };
    }
    const normalizedSelector = normalizeSelector(mapping.selector);
    if (!normalizedSelector.ok) return normalizedSelector;
    if (selectors.has(normalizedSelector.value.canonicalKey)) {
      return { ok: false, error: "DJ Link track selectors must be unique." };
    }
    ids.add(id);
    selectors.add(normalizedSelector.value.canonicalKey);
    normalized.push({
      id,
      selector: normalizedSelector.value.selector,
      timelineId: mapping.timelineId,
      retrigger: "once_per_play_session",
    });
  }
  return { ok: true, value: normalized };
}
