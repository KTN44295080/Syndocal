import type { MediaAssetAvailability, MediaAssetSummary, TimelineLayerSummary } from "./types";
import type { TimelineExternalDragPayload } from "./timelineExternalDrag";

export interface TimelineExternalMediaPlacement {
  startMs: number;
  videoLayerId: number | null;
  audioLayerId: number | null;
  linkedAudio: boolean;
  linkedVideo: boolean;
}

export interface TimelineExternalDropCallbacks {
  insertMedia: (
    mediaAssetId: number,
    placement: TimelineExternalMediaPlacement,
  ) => void | boolean | Promise<void | boolean>;
  placeScene: (cueId: number, targetLayerId: number, startMs: number) => void | boolean | Promise<void | boolean>;
  reject?: (message: string) => void;
  /** Localizes the complete operator-facing runtime message at the App edge. */
  localize?: (source: string) => string;
}

export type TimelineExternalLayerResolution = {
  candidates: TimelineLayerSummary[];
  selected: TimelineLayerSummary | null;
  mode: "none" | "unique" | "selected" | "ambiguous" | "stale";
};

/**
 * Resolve a lane for an accessible Shelf click.  A click has no canvas drop
 * point, so it may use a persisted explicit choice or the only unlocked lane.
 * It must not silently choose among multiple lanes.
 */
export const resolveTimelineExternalLayer = (
  layers: readonly TimelineLayerSummary[],
  kind: TimelineLayerSummary["kind"],
  selectedId: number | null = null,
): TimelineExternalLayerResolution => {
  const candidates = layers.filter((layer) => layer.kind === kind && !layer.locked);
  const selected = selectedId === null
    ? null
    : candidates.find((layer) => layer.id === selectedId) ?? null;
  if (selected) return { candidates, selected, mode: "selected" };
  // An explicit choice is operator intent. If that exact lane disappears,
  // changes kind, or becomes locked, never retarget the click to a different
  // lane merely because only one candidate remains.
  if (selectedId !== null) return { candidates, selected: null, mode: "stale" };
  if (candidates.length === 1) return { candidates, selected: candidates[0], mode: "unique" };
  if (candidates.length === 0) return { candidates, selected: null, mode: "none" };
  return { candidates, selected: null, mode: "ambiguous" };
};

const mediaAssetHasVideo = (asset: MediaAssetSummary) =>
  asset.source.kind === "StillImage" ||
  (asset.source.metadata?.width != null && asset.source.metadata?.height != null);

const mediaAssetHasAudio = (asset: MediaAssetSummary) => asset.source.metadata?.has_audio === true;

/**
 * Timeline placement is a machine-local admission decision. An omitted
 * availability entry is not a permissive default: the current machine has not
 * inspected that source yet. `live_source` is issued only by that inspection
 * for a non-file source and is therefore an explicit admission too.
 */
export const mediaAssetAvailabilityAllowsTimelinePlacement = (
  availability: MediaAssetAvailability | undefined,
) => availability?.kind === "available_verified"
  || availability?.kind === "available_unverified"
  || availability?.kind === "live_source";

/**
 * Shared production orchestration for external Timeline sources.  It owns
 * exact lane/companion resolution; callers only provide the authoritative
 * snapshot and the existing backend-backed insert/scene callbacks.
 */
export const executeTimelineExternalDrop = async (
  source: TimelineExternalDragPayload,
  targetLayer: TimelineLayerSummary,
  layers: readonly TimelineLayerSummary[],
  mediaAssets: readonly MediaAssetSummary[],
  sceneIds: ReadonlySet<number>,
  startMs: number,
  callbacks: TimelineExternalDropCallbacks,
  availabilityById: Readonly<Record<number, MediaAssetAvailability>> = {},
) => {
  const reject = (source: string) => callbacks.reject?.(callbacks.localize?.(source) ?? source);
  if (source.lane_kind !== targetLayer.kind) {
    reject(`${source.lane_kind} sources can only be dropped on a ${source.lane_kind} lane.`);
    return false;
  }
  if (targetLayer.locked) {
    reject(`Timeline lane ${targetLayer.label} is locked. No source was placed.`);
    return false;
  }
  // The drop point is the primary lane for a drag. Revalidate its identity
  // against the current authored snapshot before resolving any companion so
  // a Shelf selection cannot override the lane the operator actually hit.
  const currentTargetLayer = layers.find((layer) => layer.id === targetLayer.id);
  if (!currentTargetLayer || currentTargetLayer.kind !== targetLayer.kind || currentTargetLayer.locked) {
    reject(`Selected ${targetLayer.kind} Timeline lane is no longer available.`);
    return false;
  }
  if (source.kind === "scene") {
    if (targetLayer.kind !== "Lighting") return false;
    if (!sceneIds.has(source.cue_id)) {
      reject(`Scene ${source.cue_id} is no longer available.`);
      return false;
    }
    const placed = await callbacks.placeScene(source.cue_id, targetLayer.id, Math.max(0, Math.round(startMs)));
    return placed !== false;
  }
  const asset = mediaAssets.find((candidate) => candidate.id === source.media_asset_id);
  if (!asset) {
    reject(`Media Asset ${source.media_asset_id} is no longer available.`);
    return false;
  }
  const availability = availabilityById[asset.id];
  if (!mediaAssetAvailabilityAllowsTimelinePlacement(availability)) {
    reject(availability
      ? `Media Asset ${asset.label} is unavailable for Timeline placement (${availability.kind}).`
      : `Media Asset ${asset.label} is unavailable for Timeline placement.`);
    return false;
  }
  const hasVideo = mediaAssetHasVideo(asset);
  const hasAudio = mediaAssetHasAudio(asset);
  if ((source.lane_kind === "Video" && !hasVideo) || (source.lane_kind === "Audio" && !hasAudio)) {
    reject(`${source.lane_kind} source is unavailable for ${asset.label}.`);
    return false;
  }
  const resolveLinkedLayer = (id: number | null, kind: TimelineLayerSummary["kind"]) => {
    const resolution = resolveTimelineExternalLayer(layers, kind, id);
    if (id !== null && resolution.mode !== "selected") {
      reject(`Selected ${kind} Timeline lane is no longer available.`);
      return null;
    }
    if (resolution.mode === "none") {
      reject(`An unlocked ${kind} Timeline lane is required.`);
      return null;
    }
    if (resolution.mode === "ambiguous") {
      reject(`Select an unlocked ${kind} Timeline lane before placing media.`);
      return null;
    }
    return resolution.selected;
  };
  const videoLayer = hasVideo
    ? source.lane_kind === "Video"
      ? currentTargetLayer
      : resolveLinkedLayer(source.video_layer_id, "Video")
    : null;
  const audioLayer = hasAudio
    ? source.lane_kind === "Audio"
      ? currentTargetLayer
      : resolveLinkedLayer(source.audio_layer_id, "Audio")
    : null;
  if ((hasVideo && !videoLayer) || (hasAudio && !audioLayer)) {
    return false;
  }
  const placement: TimelineExternalMediaPlacement = {
    startMs: Math.max(0, Math.round(startMs)),
    videoLayerId: videoLayer?.id ?? null,
    audioLayerId: audioLayer?.id ?? null,
    linkedAudio: hasVideo && hasAudio,
    linkedVideo: hasVideo && hasAudio,
  };
  return Boolean(await callbacks.insertMedia(source.media_asset_id, placement));
};
