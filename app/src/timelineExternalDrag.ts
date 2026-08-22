import type { TimelineLayerKind } from "./types";

/**
 * The only browser data accepted by the shared Timeline canvas.  Keep this
 * payload intentionally small: it identifies an existing authored source and
 * the one lane kind it may enter.  The drop handler resolves all other state
 * from the authoritative App snapshot before invoking the backend.
 */
export const TIMELINE_EXTERNAL_DRAG_MIME = "application/x-syndocal-timeline-source";
export const TIMELINE_EXTERNAL_DRAG_SCHEMA = 1 as const;

export type TimelineExternalDragPayload =
  | {
      schema: typeof TIMELINE_EXTERNAL_DRAG_SCHEMA;
      kind: "media_asset";
      media_asset_id: number;
      lane_kind: "Video" | "Audio";
      /**
       * Shelf-selected targets are part of a media placement's intent.  They
       * are carried through browser DnD but revalidated from the current
       * authored Timeline immediately before the backend command.
       */
      video_layer_id: number | null;
      audio_layer_id: number | null;
    }
  | {
      schema: typeof TIMELINE_EXTERNAL_DRAG_SCHEMA;
      kind: "scene";
      cue_id: number;
      lane_kind: "Lighting";
    };

const isPlainRecord = (value: unknown): value is Record<string, unknown> =>
  typeof value === "object" && value !== null && !Array.isArray(value) &&
  (Object.getPrototypeOf(value) === Object.prototype || Object.getPrototypeOf(value) === null);

const isPositiveSafeInteger = (value: unknown): value is number =>
  typeof value === "number" && Number.isSafeInteger(value) && value > 0;

const hasExactKeys = (value: Record<string, unknown>, keys: readonly string[]) => {
  const actual = Object.keys(value).sort();
  const expected = [...keys].sort();
  return actual.length === expected.length && actual.every((key, index) => key === expected[index]);
};

/** Strict, fail-closed parser used by the production drop handler and checks. */
export const parseTimelineExternalDragPayload = (raw: string): TimelineExternalDragPayload | null => {
  if (typeof raw !== "string" || raw.length === 0 || raw.length > 2048) return null;
  let candidate: unknown;
  try {
    candidate = JSON.parse(raw);
  } catch {
    return null;
  }
  if (!isPlainRecord(candidate) || candidate.schema !== TIMELINE_EXTERNAL_DRAG_SCHEMA) return null;
  if (candidate.kind === "media_asset") {
    if (!hasExactKeys(candidate, ["audio_layer_id", "kind", "lane_kind", "media_asset_id", "schema", "video_layer_id"])) return null;
    if (!isPositiveSafeInteger(candidate.media_asset_id)) return null;
    if (candidate.lane_kind !== "Video" && candidate.lane_kind !== "Audio") return null;
    if (candidate.video_layer_id !== null && !isPositiveSafeInteger(candidate.video_layer_id)) return null;
    if (candidate.audio_layer_id !== null && !isPositiveSafeInteger(candidate.audio_layer_id)) return null;
    return {
      schema: TIMELINE_EXTERNAL_DRAG_SCHEMA,
      kind: "media_asset",
      media_asset_id: candidate.media_asset_id,
      lane_kind: candidate.lane_kind,
      video_layer_id: candidate.video_layer_id,
      audio_layer_id: candidate.audio_layer_id,
    };
  }
  if (candidate.kind === "scene") {
    if (!hasExactKeys(candidate, ["cue_id", "kind", "lane_kind", "schema"])) return null;
    if (!isPositiveSafeInteger(candidate.cue_id) || candidate.lane_kind !== "Lighting") return null;
    return {
      schema: TIMELINE_EXTERNAL_DRAG_SCHEMA,
      kind: "scene",
      cue_id: candidate.cue_id,
      lane_kind: "Lighting",
    };
  }
  return null;
};

export const serializeTimelineExternalDragPayload = (payload: TimelineExternalDragPayload) =>
  JSON.stringify(payload);

export const timelineExternalDragPayloadForMedia = (
  mediaAssetId: number,
  laneKind: "Video" | "Audio",
  videoLayerId: number | null,
  audioLayerId: number | null,
): TimelineExternalDragPayload | null =>
  isPositiveSafeInteger(mediaAssetId) &&
  (videoLayerId === null || isPositiveSafeInteger(videoLayerId)) &&
  (audioLayerId === null || isPositiveSafeInteger(audioLayerId))
    ? {
        schema: TIMELINE_EXTERNAL_DRAG_SCHEMA,
        kind: "media_asset",
        media_asset_id: mediaAssetId,
        lane_kind: laneKind,
        video_layer_id: videoLayerId,
        audio_layer_id: audioLayerId,
      }
    : null;

export const timelineExternalDragPayloadForScene = (cueId: number): TimelineExternalDragPayload | null =>
  isPositiveSafeInteger(cueId)
    ? { schema: TIMELINE_EXTERNAL_DRAG_SCHEMA, kind: "scene", cue_id: cueId, lane_kind: "Lighting" }
    : null;

export const setTimelineExternalDragPayload = (
  event: DragEvent,
  payload: TimelineExternalDragPayload,
) => {
  const dataTransfer = event.dataTransfer;
  if (!dataTransfer) return;
  dataTransfer.setData(TIMELINE_EXTERNAL_DRAG_MIME, serializeTimelineExternalDragPayload(payload));
  dataTransfer.effectAllowed = "copy";
};

export const parseTimelineExternalDragDataTransfer = (
  dataTransfer: DataTransfer | null | undefined,
) => {
  if (!dataTransfer) return null;
  try {
    return parseTimelineExternalDragPayload(dataTransfer.getData(TIMELINE_EXTERNAL_DRAG_MIME));
  } catch {
    return null;
  }
};

/** The MIME type survives protected dragover mode even when getData is empty. */
export const timelineExternalDragDataIsAdvertised = (
  dataTransfer: DataTransfer | null | undefined,
) => Boolean(
  dataTransfer && Array.from(dataTransfer.types).includes(TIMELINE_EXTERNAL_DRAG_MIME),
);

/** One browser drop event may be delivered through more than one bubbling path. */
export const createTimelineExternalDropGate = () => {
  const handled = new WeakSet<Event>();
  return (event: Event) => {
    if (handled.has(event)) return false;
    handled.add(event);
    return true;
  };
};

export const isTimelineLayerKind = (value: unknown): value is TimelineLayerKind =>
  value === "Lighting" || value === "Video" || value === "Audio";
