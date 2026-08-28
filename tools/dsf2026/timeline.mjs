import { fail } from "./common.mjs";
import {
  DESTINATION_DURATION_MS,
  SOURCE_DURATION_MS,
  TEMPO_METER_MAP_VERSION,
} from "./manifest.mjs";

export const SOURCE_PHASES = Object.freeze([
  ["Intro", "intro", 1],
  ["Verse", "verse", 18],
  ["Pre Chorus", "pre_chorus", 26],
  ["Chorus", "chorus", 34],
  ["Interlude", "interlude", 50],
  ["Verse", "verse", 58],
  ["Pre Chorus", "pre_chorus", 66],
  ["Chorus", "chorus", 82],
  ["Breakdown", "breakdown", 114],
  ["Chorus", "chorus", 117],
  ["Outro", "outro", 142],
]);

export const DESTINATION_PHASES = Object.freeze([
  ["Verse", "verse", 19],
  ["Pre Chorus", "pre_chorus", 43],
  ["Chorus", "chorus", 77],
  ["Interlude", "interlude", 101],
  ["Verse", "verse", 125],
  ["Pre Chorus", "pre_chorus", 147],
  ["Chorus", "chorus", 164],
  ["Outro", "outro", 192],
]);

export function roundFrameToMs(frame) {
  return Math.round((frame * 1_000) / 48_000);
}

function clickKey(song, measure, beat) {
  return `${song}:${measure}:${beat}`;
}

export function buildTempoMapOne() {
  return [
    { position_sixteenth_steps: 0, bpm: 170, numerator: 4, denominator: 4, interpolation: "Step", measure_number: 1 },
    { position_sixteenth_steps: (149 - 1) * 16, bpm: 170, numerator: 4, denominator: 4, interpolation: "Linear", measure_number: 149 },
    { position_sixteenth_steps: 156 * 16, bpm: 194, numerator: 4, denominator: 4, interpolation: "Step", measure_number: 157 },
  ];
}

export function buildTempoMapTwo() {
  return [
    [0, 4, 1],
    [272, 6, 18],
    [296, 4, 19],
    [1864, 5, 117],
    [1924, 6, 120],
    [1948, 5, 121],
    [2008, 6, 124],
    [2032, 4, 125],
  ].map(([position_sixteenth_steps, numerator, measure_number]) => ({
    position_sixteenth_steps,
    bpm: 194,
    numerator,
    denominator: 4,
    interpolation: "Step",
    measure_number,
  }));
}

export function findClick(clicks, song, measure, beat = 1) {
  const event = clicks.get(clickKey(song, measure, beat));
  if (!event) fail(`manifest chart is missing ${song} measure ${measure} beat ${beat}`);
  return event;
}

export function buildPhases(clicks, song, descriptors, timelineId, durationMs) {
  const starts = descriptors.map(([label, role, measure], index) => ({
    id: timelineId * 100 + index + 1,
    label,
    role,
    start_ms: roundFrameToMs(findClick(clicks, song, measure).localFrame),
  }));
  return starts.map((phase, index) => ({
    ...phase,
    end_ms: starts[index + 1]?.start_ms ?? durationMs,
  }));
}

export function emptyTimelineBase(id, label, durationMs) {
  return {
    id,
    label,
    layers: [],
    events: [],
    automations: [],
    video_automations: [],
    audio: null,
    audio_clips: [],
    video_clips: [],
    phases: [],
    item_groups: [],
    loop_region: null,
    follow: null,
    guide_enabled: true,
    audio_offset_ms: 0,
    audio_muted: false,
    metronome_enabled: false,
    count_in_beats: 4,
    tempo_meter_map_version: TEMPO_METER_MAP_VERSION,
    tempo_meter_map: [],
    playing: false,
    position_ms: 0,
    duration_ms: durationMs,
  };
}

function nextPositiveId(used, label) {
  let candidate = 1;
  while (used.has(candidate)) {
    candidate += 1;
    if (!Number.isSafeInteger(candidate)) fail(`${label} ID allocation overflow`);
  }
  used.add(candidate);
  return candidate;
}

const AUTHORED_LIGHTING_LAYER_ID = 1;
const AUTHORED_SOURCE_AUDIO_LAYER_ID = 2;
const AUTHORED_DESTINATION_AUDIO_LAYER_ID = 3;

function buildReferenceAudioLayer(id, order = 0) {
  return {
    id,
    label: "Reference Audio",
    order,
    muted: true,
    locked: false,
    solo: false,
    expanded: true,
    kind: "Audio",
  };
}

function buildLightingLayer(id = AUTHORED_LIGHTING_LAYER_ID, order = 1) {
  return {
    id,
    label: "Lighting",
    order,
    muted: false,
    locked: false,
    solo: false,
    expanded: true,
    kind: "Lighting",
  };
}

function validatePositiveLayerId(layerId, label) {
  if (!Number.isSafeInteger(layerId) || layerId <= 0) {
    fail(`${label} requires a positive timeline layer ID`);
  }
}

/**
 * Keep generated scene events and ordinary Timeline Audio Clips on explicit,
 * typed lanes once a project has authored any lanes.  Native project loading
 * permits a null event layer only for a truly legacy timeline with no lanes;
 * reference-audio authoring adds Audio lanes, so every generated event must
 * carry its compatible Lighting/Video lane identity explicitly.
 */
export function validateGeneratedTimelineLayerReferences(timeline, label = "authored Timeline") {
  if (!timeline || typeof timeline !== "object") fail(`${label} must be an object`);
  const layers = Array.isArray(timeline.layers) ? timeline.layers : [];
  const layerById = new Map();
  for (const [index, layer] of layers.entries()) {
    validatePositiveLayerId(layer?.id, `${label} layer ${index}`);
    if (layerById.has(layer.id)) fail(`${label} contains duplicate timeline layer ID ${layer.id}`);
    layerById.set(layer.id, layer);
  }
  for (const [index, event] of (Array.isArray(timeline.events) ? timeline.events : []).entries()) {
    const layerId = event?.layer_id;
    validatePositiveLayerId(layerId, `${label} event ${event?.id ?? index + 1}`);
    const layer = layerById.get(layerId);
    if (!layer) fail(`${label} event ${event?.id ?? index + 1} references missing timeline layer ${layerId}`);
    const track = event?.track;
    const expectedKind = track === "Lighting" || track === "Video" ? track : null;
    if (expectedKind === null) fail(`${label} event ${event?.id ?? index + 1} has unsupported track ${String(track)}`);
    if (layer.kind !== expectedKind) {
      fail(`${label} event ${event?.id ?? index + 1} uses ${track} track with incompatible ${layer.kind} timeline layer ${layerId}`);
    }
  }
  for (const [index, clip] of (Array.isArray(timeline.audio_clips) ? timeline.audio_clips : []).entries()) {
    const layerId = clip?.layer_id;
    validatePositiveLayerId(layerId, `${label} audio clip ${clip?.id ?? index + 1}`);
    const layer = layerById.get(layerId);
    if (!layer) fail(`${label} audio clip ${clip?.id ?? index + 1} references missing timeline layer ${layerId}`);
    if (layer.kind !== "Audio") {
      fail(`${label} audio clip ${clip?.id ?? index + 1} references non-Audio timeline layer ${layerId}`);
    }
  }
  return timeline;
}

function validateReferenceAudioDescriptor(descriptor, key) {
  if (!descriptor || typeof descriptor !== "object") fail(`reference audio ${key} descriptor is missing`);
  if (typeof descriptor.path !== "string" || descriptor.path.trim().length === 0) {
    fail(`reference audio ${key} requires a managed path`);
  }
  if (!Number.isSafeInteger(descriptor.durationMs) || descriptor.durationMs <= 0) {
    fail(`reference audio ${key} requires a measured positive duration`);
  }
  if (!Number.isSafeInteger(descriptor.byteSize) || descriptor.byteSize <= 0) {
    fail(`reference audio ${key} requires a positive byte size`);
  }
  if (typeof descriptor.sha256 !== "string" || !/^[a-f0-9]{64}$/u.test(descriptor.sha256)) {
    fail(`reference audio ${key} requires a lowercase SHA-256 identity`);
  }
}

/**
 * Add two ordinary, muted Audio Timeline lanes backed by the existing Media
 * Library catalog shape. This is deliberately an additive authoring helper:
 * playback still flows through the engine's regular Timeline Audio Clip
 * transport, and the layer mute (not Timeline master mute) keeps these files
 * silent for production until an operator explicitly enables the lanes.
 */
export function applyReferenceAudioToTimelines(source, destination, references, existingMediaAssets = []) {
  if (references == null) return { source, destination, mediaAssets: existingMediaAssets };
  if (!references || typeof references !== "object") fail("reference audio descriptors must be an object");
  const entries = [
    ["jinseiOverPath", source, "Jinsei Over Reference Audio"],
    ["madowHoshiPath", destination, "Madow Hoshi Reference Audio"],
  ];
  const mediaAssets = Array.isArray(existingMediaAssets) ? [...existingMediaAssets] : [];
  const usedAssetIds = new Set(mediaAssets.map((asset) => asset?.id).filter(Number.isSafeInteger));
  const usedLayerIds = new Set([source, destination].flatMap((timeline) => (timeline.layers ?? []).map((layer) => layer?.id)).filter(Number.isSafeInteger));
  const usedClipIds = new Set([source, destination].flatMap((timeline) => (timeline.audio_clips ?? []).map((clip) => clip?.id)).filter(Number.isSafeInteger));

  for (const [key, timeline, label] of entries) {
    const descriptor = references[key];
    validateReferenceAudioDescriptor(descriptor, key);
    const assetId = nextPositiveId(usedAssetIds, "MediaAsset");
    const existingAudioLayer = (timeline.layers ?? []).find((layer) => layer?.kind === "Audio");
    let layerId;
    if (existingAudioLayer) {
      validatePositiveLayerId(existingAudioLayer.id, `${label} Audio layer`);
      layerId = existingAudioLayer.id;
    } else {
      layerId = nextPositiveId(usedLayerIds, "Timeline Audio layer");
    }
    const clipId = nextPositiveId(usedClipIds, "Timeline Audio clip");
    mediaAssets.push({
      id: assetId,
      label,
      source: {
        kind: "File",
        path: descriptor.path.trim(),
        name: null,
        codec: "mp3",
        metadata: {
          duration_ms: descriptor.durationMs,
          width: null,
          height: null,
          frame_rate: null,
          has_audio: true,
        },
      },
      content_hash: { algorithm: "Sha256", hex: descriptor.sha256 },
      byte_size: descriptor.byteSize,
    });
    if (!existingAudioLayer) {
      const order = (timeline.layers ?? []).reduce(
        (maximum, layer) => Math.max(maximum, Number.isSafeInteger(layer?.order) ? layer.order : -1),
        -1,
      ) + 1;
      timeline.layers = [
        ...(timeline.layers ?? []),
        buildReferenceAudioLayer(layerId, order),
      ];
    }
    timeline.audio_clips = [
      ...(timeline.audio_clips ?? []),
      {
        id: clipId,
        layer_id: layerId,
        media_asset_id: assetId,
        path: "",
        start_ms: 0,
        offset_ms: 0,
        duration_ms: descriptor.durationMs,
        gain: 1,
        fade_in_ms: 0,
        fade_out_ms: 0,
      },
    ];
  }
  return { source, destination, mediaAssets };
}

export function buildBoundaryLightingEvents(lightingCues, boundaryMs, lightingLayerId = AUTHORED_LIGHTING_LAYER_ID) {
  if (!Number.isSafeInteger(boundaryMs) || boundaryMs < 0) fail("lighting cue boundary must be a safe non-negative millisecond position");
  validatePositiveLayerId(lightingLayerId, "authored lighting boundary");
  const required = ["all_white", "all_max"];
  if (!lightingCues || required.some((label) => !lightingCues[label])) {
    fail("authored lighting boundary requires the exact existing all_white and all_max Cues");
  }
  const ids = required.map((label) => lightingCues[label].id);
  if (ids.some((id) => !Number.isSafeInteger(id) || id <= 0) || new Set(ids).size !== ids.length) {
    fail("authored lighting boundary requires unique existing positive Cue IDs");
  }
  return required.map((label, index) => ({
    id: index + 1,
    cue_id: lightingCues[label].id,
    time_ms: boundaryMs,
    time_beats: null,
    track: "Lighting",
    layer_id: lightingLayerId,
    duration_ms: 0,
    duration_beats: null,
    conform_to_tempo: false,
    loop_fill: false,
    source_offset_ms: 0,
    rate: null,
    fade_in_ms: 0,
    fade_out_ms: 0,
    loop_count: 1,
    jump_to_event_id: null,
  }));
}

export function buildAuthoredTimelines(manifest, lightingCues) {
  const clicks = new Map(manifest.clickEvents.map((event) => [clickKey(event.song, event.measure, event.beat), event]));
  const source = emptyTimelineBase(1, "人生オーバー", SOURCE_DURATION_MS);
  source.layers = [
    buildReferenceAudioLayer(AUTHORED_SOURCE_AUDIO_LAYER_ID, 0),
    buildLightingLayer(AUTHORED_LIGHTING_LAYER_ID, 1),
  ];
  source.tempo_meter_map = buildTempoMapOne();
  source.loop_region = {
    a_ms: roundFrameToMs(findClick(clicks, "jinsei-over", 98).localFrame),
    b_ms: roundFrameToMs(findClick(clicks, "jinsei-over", 99).localFrame),
    enabled: true,
    musical_length_beats: 4,
  };
  source.events = buildBoundaryLightingEvents(lightingCues, source.loop_region.b_ms, AUTHORED_LIGHTING_LAYER_ID);
  source.phases = buildPhases(clicks, "jinsei-over", SOURCE_PHASES, 1, SOURCE_DURATION_MS);
  source.follow = {
    enabled: true,
    next_timeline_id: 2,
    duration: { unit: "Bars", value_milliunits: 1000 },
    curve: "Linear",
    video_kind: "Crossfade",
    lighting_policy: "hold_then_cut",
    destination_bpm: null,
    preroll_ms: 0,
    trans_cadence_bars: 2,
    trans_target_measures: [149, 151, 153, 155],
    hold_first_destination_measure: true,
    fault_policy: "hold",
  };

  const destination = emptyTimelineBase(2, "惑う星", DESTINATION_DURATION_MS);
  destination.layers = [buildReferenceAudioLayer(AUTHORED_DESTINATION_AUDIO_LAYER_ID, 0)];
  destination.tempo_meter_map = buildTempoMapTwo();
  destination.phases = buildPhases(clicks, "madow-hoshi", DESTINATION_PHASES, 2, DESTINATION_DURATION_MS);
  return { source, destination };
}
