import type {
  VideoClipLayerRuntimeSummary,
  VideoClipSlotAuthoritativeCommitKind,
  VideoClipSlotId,
  VideoClipSlotSummary,
} from "./types";

export const VIDEO_CLIP_SLOT_BANK_SIZE = 32;

export type VideoClipSlotBankMode = "edit" | "control";

export interface VideoClipSlotBankCell {
  index: number;
  slot: VideoClipSlotSummary | null;
  active: boolean;
  queued: boolean;
  pending: boolean;
}

/**
 * The pad bank is one canonical 32-cell projection of authored slots plus
 * runtime truth.  Never derive active/queued state from `VideoLayerState`.
 */
export const videoClipSlotBankCells = (
  slots: readonly VideoClipSlotSummary[] | undefined,
  runtime: VideoClipLayerRuntimeSummary | null | undefined,
): VideoClipSlotBankCell[] => Array.from({ length: VIDEO_CLIP_SLOT_BANK_SIZE }, (_, index) => {
  const slot = slots?.[index] ?? null;
  return {
    index,
    slot,
    active: slot?.id === runtime?.active_slot_id,
    queued: slot?.id === runtime?.queued_slot_id,
    pending: slot?.id === runtime?.pending_launch?.slot_id,
  };
});

export const videoClipSlotBankActionLabels = (mode: VideoClipSlotBankMode): readonly string[] =>
  mode === "edit" ? ["Select", "Preview", "More"] : ["Queue", "Preview", "More"];

export const videoClipSlotBankPrimaryActionCount = (mode: VideoClipSlotBankMode) =>
  videoClipSlotBankActionLabels(mode).length;

export const videoClipSlotCommandKindMatches = (
  actual: VideoClipSlotAuthoritativeCommitKind,
  expected: VideoClipSlotAuthoritativeCommitKind,
) => actual === expected;

export const videoClipSlotRuntimeGenerationCanApply = (
  currentEpoch: number | null,
  currentGeneration: number,
  incomingEpoch: number,
  incomingGeneration: number,
): boolean => {
  if (!Number.isSafeInteger(incomingGeneration) || incomingGeneration < 0) {
    throw new Error("Clip Slot runtime generation is outside JavaScript's exact integer range.");
  }
  return currentEpoch === null || currentEpoch !== incomingEpoch || incomingGeneration >= currentGeneration;
};

export const videoClipSlotReorderedIds = (
  orderedSlotIds: readonly VideoClipSlotId[],
  sourceSlotId: VideoClipSlotId,
  destinationIndex: number,
): VideoClipSlotId[] => {
  const withoutSource = orderedSlotIds.filter((slotId) => slotId !== sourceSlotId);
  const boundedDestination = Math.max(0, Math.min(withoutSource.length, Math.trunc(destinationIndex)));
  return [
    ...withoutSource.slice(0, boundedDestination),
    sourceSlotId,
    ...withoutSource.slice(boundedDestination),
  ];
};

export const videoClipSlotDropTarget = (target: Element | null): {
  layerId: number;
  beforeSlotId: VideoClipSlotId | null;
} | null => {
  const slotTarget = target?.closest<HTMLElement>("[data-video-clip-slot-layer-id][data-video-clip-slot-id]");
  if (slotTarget) {
    const layerId = Number(slotTarget.dataset.videoClipSlotLayerId);
    const beforeSlotId = Number(slotTarget.dataset.videoClipSlotId);
    return Number.isFinite(layerId) && Number.isFinite(beforeSlotId)
      ? { layerId, beforeSlotId }
      : null;
  }
  const layerTarget = target?.closest<HTMLElement>("[data-video-clip-slot-layer-id]");
  if (!layerTarget) return null;
  const layerId = Number(layerTarget.dataset.videoClipSlotLayerId);
  return Number.isFinite(layerId) ? { layerId, beforeSlotId: null } : null;
};
