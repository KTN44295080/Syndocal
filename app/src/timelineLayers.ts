import type {
  CueSummary,
  TimelineCueEventSummary,
  TimelineLayerKind,
  TimelineLayerSummary,
} from "./types";

export const timelineLayerKindOrder: readonly TimelineLayerKind[] = ["Audio", "Lighting", "Video"];

export const implicitTimelineLayers = (): TimelineLayerSummary[] => [
  { id: 0, label: "Lighting", order: 0, muted: false, locked: false, solo: false, kind: "Lighting" },
  { id: 1, label: "Video", order: 1, muted: false, locked: false, solo: false, kind: "Video" },
];

export const timelineLayerKindRank = (kind: TimelineLayerKind) => timelineLayerKindOrder.indexOf(kind);

export const sortedTimelineLayers = (layers: readonly TimelineLayerSummary[]) => [...layers].sort(
  (left, right) => timelineLayerKindRank(left.kind) - timelineLayerKindRank(right.kind)
    || left.order - right.order
    || left.id - right.id,
);

export const effectiveTimelineLayers = (layers: readonly TimelineLayerSummary[] | undefined) =>
  sortedTimelineLayers(layers && layers.length > 0 ? layers : implicitTimelineLayers());

export const sameTimelineLayerSummaries = (
  previous: readonly TimelineLayerSummary[],
  next: readonly TimelineLayerSummary[],
) => previous.length === next.length && previous.every((layer, index) => {
  const candidate = next[index];
  return layer.id === candidate.id
    && layer.label === candidate.label
    && layer.order === candidate.order
    && layer.muted === candidate.muted
    && layer.locked === candidate.locked
    && layer.solo === candidate.solo
    && layer.kind === candidate.kind;
});

export const isLegacyTimelineLayerSet = (layers: readonly TimelineLayerSummary[]) => {
  const ordered = sortedTimelineLayers(layers);
  return ordered.length === 2
    && ordered[0].id === 0
    && ordered[0].label === "Lighting"
    && ordered[0].kind === "Lighting"
    && ordered[0].order === 0
    && !ordered[0].muted
    && !ordered[0].locked
    && !ordered[0].solo
    && ordered[1].id === 1
    && ordered[1].label === "Video"
    && ordered[1].kind === "Video"
    && ordered[1].order === 1
    && !ordered[1].muted
    && !ordered[1].locked
    && !ordered[1].solo;
};

export const timelineLayerForEvent = (
  layers: readonly TimelineLayerSummary[],
  event: Pick<TimelineCueEventSummary, "layer_id" | "track">,
) => layers.find((layer) => layer.id === event.layer_id)
  ?? layers.find((layer) => layer.kind === event.track)
  ?? null;

export const timelineLayerIdForEvent = (
  layers: readonly TimelineLayerSummary[],
  event: Pick<TimelineCueEventSummary, "layer_id" | "track">,
) => timelineLayerForEvent(layers, event)?.id ?? (event.track === "Lighting" ? 0 : 1);

export const cueDropDurationMs = (cue: CueSummary, bpm: number, fallbackMs: number) => {
  const authoredBeats = cue.authored_beats;
  if (
    authoredBeats !== null
    && authoredBeats !== undefined
    && Number.isFinite(authoredBeats)
    && authoredBeats > 0
    && Number.isFinite(bpm)
    && bpm > 0
  ) {
    return Math.max(1, Math.round(authoredBeats * 60_000 / bpm));
  }
  return Math.max(1, Math.round(Number.isFinite(fallbackMs) ? fallbackMs : 1_000));
};
