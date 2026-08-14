import { createComponent, createSignal } from "solid-js";
import { render } from "solid-js/web";
import { TimelineOverview } from "../../src/components/TimelineOverview.tsx";
import "../../src/styles.css";

const layers = [
  { id: 1, label: "Audio", order: 0, muted: false, locked: false, solo: false, expanded: true, kind: "Audio" },
  { id: 2, label: "Lighting", order: 1, muted: false, locked: false, solo: false, expanded: true, kind: "Lighting" },
  { id: 3, label: "Video", order: 2, muted: false, locked: false, solo: false, expanded: true, kind: "Video" },
];
const scene = (id, startMs) => ({
  id,
  cue_id: id,
  cue_label: `Scene ${id}`,
  track: "Lighting",
  layer_id: 2,
  time_ms: startMs,
  duration_ms: 800,
  loop_count: 1,
  conform_to_tempo: false,
  loop_fill: false,
  authored_beats: null,
  rate: null,
  total_duration_ms: 800,
  fade_in_ms: 0,
  fade_out_ms: 0,
  x: startMs / 12_000 * 100,
  width: 800 / 12_000 * 100,
  y: 0,
  under_playhead: false,
  is_super_scene: false,
});
const audio = (id, startMs) => ({
  id,
  layer_id: 1,
  media_asset_id: id,
  path: `C:/fixture/audio-${id}.wav`,
  start_ms: startMs,
  offset_ms: 400,
  duration_ms: 800,
  gain: 1,
  fade_in_ms: 40,
  fade_out_ms: 40,
});
const video = (id, startMs) => ({
  id,
  layer_id: 3,
  media_asset_id: id,
  start_ms: startMs,
  offset_ms: 400,
  duration_ms: 800,
  fade_in_ms: 40,
  fade_out_ms: 40,
});
const automation = (id, kind, startMs) => ({
  id: `${kind}-${id}`,
  kind,
  automation_id: id,
  target_id: id,
  label: `${kind} automation ${id}`,
  track: kind === "lighting" ? "Lighting" : "Video",
  layer_id: kind === "lighting" ? 2 : 3,
  start_ms: startMs,
  end_ms: startMs + 800,
  keyframes: [],
  x: startMs / 12_000 * 100,
  width: 800 / 12_000 * 100,
  y: 0,
  enabled: true,
});

const events = [scene(101, 500), scene(102, 1_800)];
const audioClips = [audio(201, 3_100), audio(202, 4_400)];
const videoClips = [video(301, 5_700), video(302, 7_000)];
const automationRanges = [
  automation(401, "lighting", 8_300),
  automation(402, "lighting", 9_600),
  automation(501, "video", 8_300),
  automation(502, "video", 9_600),
];

const itemKey = (item) => item.kind === "lighting_event"
  ? `${item.kind}:${item.event_id}`
  : item.kind.endsWith("automation")
    ? `${item.kind}:${item.automation_id}`
    : `${item.kind}:${item.clip_id}`;
const linkedKeys = new Set([
  "lighting_event:101",
  "audio_clip:201",
  "video_clip:301",
  "lighting_automation:401",
  "video_automation:501",
]);

const [selectedEventId, setSelectedEventId] = createSignal(null);
const [selectedAudioIds, setSelectedAudioIds] = createSignal([]);
const [selectedVideoIds, setSelectedVideoIds] = createSignal([]);
const [selectedRangeId, setSelectedRangeId] = createSignal(null);
const calls = { trim: [], scene: [], audio: [], video: [], automation: [] };
window.__timelineDirectResizeFixture = { ready: false, calls };

const props = {
  layers,
  legacyMode: false,
  cueDrag: null,
  events,
  layerItemCounts: new Map([[1, 2], [2, 4], [3, 4]]),
  audioClips,
  videoClips,
  mediaAssets: [],
  audioAnalysis: null,
  executionLive: false,
  markerAriaLabel: (event) => event.cue_label,
  automationRanges,
  superSceneEmptyHintCount: 0,
  overlapClusters: [],
  overlapLayerIds: [],
  get selectedRangeId() { return selectedRangeId(); },
  get selectedEventId() { return selectedEventId(); },
  get selectedAudioClipId() { return selectedAudioIds()[0] ?? null; },
  get selectedVideoClipId() { return selectedVideoIds()[0] ?? null; },
  get selectedAudioClipIds() { return selectedAudioIds(); },
  get selectedVideoClipIds() { return selectedVideoIds(); },
  playheadX: 0,
  visibleWindow: { start_ms: 0, end_ms: 12_000 },
  bpm: 120,
  stretchMode: "WINDOW",
  magnetEnabled: false,
  armedCue: null,
  snapTimeMs: (timeMs) => Math.round(timeMs),
  onSeekTime: () => {},
  onSelectAutomationRange: (range) => setSelectedRangeId(range.id),
  onSelectEvent: (eventId) => setSelectedEventId(eventId),
  onOpenSuperScene: () => {},
  onSelectAudioClip: (clipId) => setSelectedAudioIds([clipId]),
  onSelectVideoClip: (clipId) => setSelectedVideoIds([clipId]),
  onOpenItemContextMenu: () => {},
  onInspectOverlapCluster: () => {},
  onUpdateLayer: () => {},
  onOpenLayerMenu: () => {},
  onAddAudioClip: () => {},
  onUpdateAudioClip: (clip) => calls.audio.push(structuredClone(clip)),
  onUpdateVideoClip: (clip) => calls.video.push(structuredClone(clip)),
  isTimelineItemLinked: (item) => linkedKeys.has(itemKey(item)),
  onTrimTimelineItem: (item, edge, boundaryMs, isolate) => calls.trim.push({
    item: structuredClone(item),
    edge,
    boundary_ms: boundaryMs,
    isolate,
  }),
  onStatus: () => {},
  onMoveEventPlacement: () => {},
  onResizeEventTime: (eventId, edge, timeMs) => calls.scene.push({ event_id: eventId, edge, boundary_ms: timeMs }),
  onSetEventFade: () => {},
  onSetVisibleWindow: () => {},
  onZoomAt: () => {},
  onPlaceArmedCue: () => {},
  onMoveAutomationRangeTime: () => {},
  onResizeAutomationRangeTime: (range, edge, timeMs) => calls.automation.push({
    kind: range.kind,
    automation_id: range.automation_id,
    edge,
    boundary_ms: timeMs,
  }),
  onMoveAutomationKeyframeTime: () => {},
};

render(() => createComponent(TimelineOverview, props), document.getElementById("root"));
requestAnimationFrame(() => {
  window.__timelineDirectResizeFixture.ready = true;
});
