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

const mediaAssetHasVideo = (asset: MediaAssetSummary) =>
  asset.source.kind === "StillImage" ||
  (asset.source.metadata?.width != null && asset.source.metadata?.height != null);

const mediaAssetHasAudio = (asset: MediaAssetSummary) => asset.source.metadata?.has_audio === true;

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
  if (availability && availability.kind !== "available_verified" && availability.kind !== "available_unverified") {
    reject(`Media Asset ${asset.label} is unavailable for Timeline placement (${availability.kind}).`);
    return false;
  }
  const hasVideo = mediaAssetHasVideo(asset);
  const hasAudio = mediaAssetHasAudio(asset);
  if ((source.lane_kind === "Video" && !hasVideo) || (source.lane_kind === "Audio" && !hasAudio)) {
    reject(`${source.lane_kind} source is unavailable for ${asset.label}.`);
    return false;
  }
  const resolveExactLayer = (id: number | null, kind: TimelineLayerSummary["kind"]) => {
    if (id === null) {
      reject(`Select an unlocked ${kind} Timeline lane before placing media.`);
      return null;
    }
    const layer = layers.find((candidate) => candidate.id === id);
    if (!layer || layer.kind !== kind || layer.locked) {
      reject(`Selected ${kind} Timeline lane is no longer available.`);
      return null;
    }
    return layer;
  };
  const videoLayer = hasVideo ? resolveExactLayer(source.video_layer_id, "Video") : null;
  const audioLayer = hasAudio ? resolveExactLayer(source.audio_layer_id, "Audio") : null;
  if ((hasVideo && !videoLayer) || (hasAudio && !audioLayer)) {
    return false;
  }
  const selectedDropLayer = source.lane_kind === "Video" ? videoLayer : audioLayer;
  if (!selectedDropLayer || selectedDropLayer.id !== targetLayer.id) {
    reject("Drop target no longer matches the selected Timeline lane.");
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
