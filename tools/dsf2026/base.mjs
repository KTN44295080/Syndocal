import {
  equalJson,
  fail,
  isObject,
  parseJsonValue,
} from "./common.mjs";
import { TEMPO_METER_MAP_VERSION } from "./manifest.mjs";

const ALLOWED_TIMELINE_KEYS = new Set([
  "id",
  "label",
  "layers",
  "events",
  "automations",
  "video_automations",
  "audio",
  "audio_clips",
  "video_clips",
  "phases",
  "item_groups",
  "loop_region",
  "follow",
  "guide_enabled",
  "audio_offset_ms",
  "audio_muted",
  "metronome_enabled",
  "count_in_beats",
  "tempo_meter_map",
  "tempo_meter_map_version",
  "playing",
  "position_ms",
  "duration_ms",
  "count_in_remaining_ms",
  "audio_transport_revision",
  "transport_epoch",
  "transport_generation",
  "active_child_transports",
  "loop_runtime",
  "follow_runtime",
  "guide_cues",
  "click_events",
  "click_schedule_generation",
  "click_queue_overflow",
]);

export function authoredTimelineProjection(timeline) {
  return {
    id: timeline.id,
    label: timeline.label,
    layers: timeline.layers ?? [],
    events: timeline.events ?? [],
    automations: timeline.automations ?? [],
    video_automations: timeline.video_automations ?? [],
    audio: timeline.audio ?? null,
    audio_clips: timeline.audio_clips ?? [],
    video_clips: timeline.video_clips ?? [],
    phases: timeline.phases ?? [],
    item_groups: timeline.item_groups ?? [],
    loop_region: timeline.loop_region ?? null,
    follow: timeline.follow ?? null,
    guide_enabled: timeline.guide_enabled ?? false,
    audio_offset_ms: timeline.audio_offset_ms ?? 0,
    audio_muted: timeline.audio_muted ?? false,
    metronome_enabled: timeline.metronome_enabled ?? false,
    count_in_beats: timeline.count_in_beats ?? 4,
    tempo_meter_map: timeline.tempo_meter_map ?? [],
    tempo_meter_map_version: timeline.tempo_meter_map_version ?? TEMPO_METER_MAP_VERSION,
    duration_ms: timeline.duration_ms,
  };
}

export function validateEmptyTimeline(timeline, label) {
  if (!isObject(timeline)) fail(`${label} must be an object`);
  const unknown = Object.keys(timeline).filter((key) => !ALLOWED_TIMELINE_KEYS.has(key));
  if (unknown.length > 0) fail(`${label} has unsupported authored field(s): ${unknown.join(", ")}`);
  if (timeline.id !== 1 || timeline.label !== "Timeline 1") fail(`${label} must be explicit empty Timeline 1 (id 1)`);
  for (const field of ["layers", "events", "automations", "video_automations", "audio_clips", "video_clips", "phases", "item_groups", "tempo_meter_map"]) {
    if (timeline[field] !== undefined && (!Array.isArray(timeline[field]) || timeline[field].length !== 0)) fail(`${label}.${field} must be empty/default before authoring`);
  }
  for (const field of ["audio", "loop_region", "follow", "click_queue_overflow"]) {
    if (timeline[field] !== undefined && timeline[field] !== null) fail(`${label}.${field} must be empty/default before authoring`);
  }
  const defaults = {
    guide_enabled: false,
    audio_offset_ms: 0,
    audio_muted: false,
    metronome_enabled: false,
    count_in_beats: 4,
    tempo_meter_map_version: TEMPO_METER_MAP_VERSION,
    playing: false,
    position_ms: 0,
    duration_ms: 0,
    count_in_remaining_ms: 0,
    audio_transport_revision: 0,
    transport_epoch: 0,
    transport_generation: 0,
    click_schedule_generation: 0,
  };
  for (const [field, expected] of Object.entries(defaults)) {
    if (timeline[field] !== undefined && !equalJson(timeline[field], expected)) fail(`${label}.${field} must remain its empty/default value`);
  }
  for (const field of ["active_child_transports", "guide_cues", "click_events"]) {
    if (timeline[field] !== undefined && (!Array.isArray(timeline[field]) || timeline[field].length !== 0)) fail(`${label}.${field} must be empty/default before authoring`);
  }
  if (timeline.loop_runtime !== undefined && !equalJson(timeline.loop_runtime, {})) fail(`${label}.loop_runtime must not contain runtime state`);
  if (timeline.follow_runtime !== undefined && !equalJson(timeline.follow_runtime, {})) fail(`${label}.follow_runtime must not contain runtime state`);
  return timeline;
}

export function resolveExistingLightingCues(base) {
  const cues = base?.snapshot?.cues;
  if (!Array.isArray(cues)) fail("base snapshot.cues is missing; an authored lighting event cannot invent a Cue");
  const cueLists = base?.snapshot?.cue_lists;
  if (!Array.isArray(cueLists)) fail("base snapshot.cue_lists is missing; Cue bank identity cannot be verified");
  const required = [
    ["all_white", "color"],
    ["all_max", "dimmer"],
  ];
  const resolved = {};
  const ids = new Set();
  const listIds = new Set();
  for (const [label, listLabel] of required) {
    const matches = cues.filter((cue) => isObject(cue) && cue.label === label);
    if (matches.length !== 1) fail(`base snapshot.cues must contain exactly one existing Cue labeled ${label}`);
    const cue = matches[0];
    if (!Number.isSafeInteger(cue.id) || cue.id <= 0 || ids.has(cue.id)) fail(`base Cue ${label} must have an existing unique positive integer id`);
    if (cue.recall_mode !== "Coexist") fail(`base Cue ${label} must retain its existing Coexist recall mode`);
    if (!Number.isSafeInteger(cue.cue_list_id) || cue.cue_list_id <= 0) fail(`base Cue ${label} must retain an existing Cue list id`);
    const listMatches = cueLists.filter((list) => isObject(list) && list.id === cue.cue_list_id && list.label === listLabel);
    if (listMatches.length !== 1) fail(`base Cue ${label} must reference exactly one existing ${listLabel} Cue list`);
    if (listIds.has(cue.cue_list_id)) fail(`base Cue ${label} must retain a distinct existing Cue list identity`);
    if (cue.group_id !== listLabel) fail(`base Cue ${label} must retain its existing ${listLabel} group identity`);
    ids.add(cue.id);
    listIds.add(cue.cue_list_id);
    resolved[label] = { id: cue.id, label: cue.label, cue_list_id: cue.cue_list_id, list_label: listLabel };
  }
  return resolved;
}

export const resolveExistingCue = (base, label) => resolveExistingLightingCues(base)[label];

export const SHOW_DMX_PROTOCOL = "EnttecOpenDmx";
export const SHOW_DMX_SERIAL_PORT = "";
export const SHOW_DMX_SERIAL_BAUD_RATE = 250_000;
const SHOW_DMX_MACHINE_LOCAL_BOUNDARY = "physical port is machine-local; project route serial_port must be empty";

export function stageDmxOutput(base) {
  const output = base?.snapshot?.output;
  const routes = base?.snapshot?.dmx_outputs;
  if (!isObject(output)) fail(`base snapshot.output is missing; the staged show route is ambiguous; ${SHOW_DMX_MACHINE_LOCAL_BOUNDARY}`);
  if (!Array.isArray(routes) || routes.length !== 1 || !isObject(routes[0])) {
    fail(`base snapshot.dmx_outputs must contain exactly one existing route for deterministic staging; ${SHOW_DMX_MACHINE_LOCAL_BOUNDARY}`);
  }
  const route = {
    ...routes[0],
    enabled: false,
    protocol: SHOW_DMX_PROTOCOL,
    serial_port: SHOW_DMX_SERIAL_PORT,
    serial_baud_rate: SHOW_DMX_SERIAL_BAUD_RATE,
  };
  return {
    output: {
      ...output,
      enabled: false,
      protocol: SHOW_DMX_PROTOCOL,
      serial_port: SHOW_DMX_SERIAL_PORT,
      serial_baud_rate: SHOW_DMX_SERIAL_BAUD_RATE,
    },
    dmx_outputs: [route],
  };
}

export function validateDmxOutputStaging(project) {
  const output = project?.snapshot?.output;
  const routes = project?.snapshot?.dmx_outputs;
  if (!isObject(output)) return { error: `snapshot.output is missing; ${SHOW_DMX_MACHINE_LOCAL_BOUNDARY}` };
  if (!Array.isArray(routes) || routes.length !== 1 || !isObject(routes[0])) return { error: `snapshot.dmx_outputs must contain exactly one route; ${SHOW_DMX_MACHINE_LOCAL_BOUNDARY}` };
  const values = [output, routes[0]];
  if (values.some((route) => route.enabled !== false || route.protocol !== SHOW_DMX_PROTOCOL || route.serial_port !== SHOW_DMX_SERIAL_PORT || route.serial_baud_rate !== SHOW_DMX_SERIAL_BAUD_RATE)) {
    return { error: `primary and persisted DMX routes must be disabled EnttecOpenDmx at 250000 baud; ${SHOW_DMX_MACHINE_LOCAL_BOUNDARY}` };
  }
  return { detail: "primary and persisted DMX routes are staged as disabled EnttecOpenDmx at 250000 baud; physical port is machine-local; project route serial_port is empty" };
}

export function validateLightingBoundary(project) {
  try {
    const cues = resolveExistingLightingCues(project);
    const source = project?.snapshot?.timeline;
    const bank = project?.snapshot?.timeline_bank;
    if (!isObject(source) || !Array.isArray(bank)) return { error: "authored source Timeline or bank is missing" };
    const sourceBank = bank.find((timeline) => timeline?.id === 1);
    const destination = bank.find((timeline) => timeline?.id === 2);
    if (!sourceBank || !destination) return { error: "authored source/destination Timeline entries are missing" };
    if (!Array.isArray(source.events) || source.events.length !== 2) return { error: "source Timeline must contain exactly the two post-loop lighting Scene Blocks" };
    if (!Array.isArray(destination.events) || destination.events.length !== 0) return { error: "destination Timeline must retain empty events" };
    if (!isObject(source.loop_region) || !Number.isSafeInteger(source.loop_region.b_ms)) return { error: "source loop boundary is missing" };
    const expectedIds = [cues.all_white.id, cues.all_max.id];
    const seenIds = new Set();
    for (const [index, event] of source.events.entries()) {
      if (!isObject(event) || event.id !== index + 1 || seenIds.has(event.id)) return { error: "source lighting boundary event IDs must be stable and unique" };
      seenIds.add(event.id);
      if (event.cue_id !== expectedIds[index]
          || event.track !== "Lighting"
          || event.time_ms !== source.loop_region.b_ms
          || event.time_ms < source.loop_region.b_ms
          || !Number.isSafeInteger(source.duration_ms)
          || source.duration_ms <= event.time_ms
          || event.duration_ms !== source.duration_ms - event.time_ms) {
        return { error: "source lighting boundary Cues must be all_white/all_max Scene Blocks spanning from the loop exit to the source Timeline end" };
      }
    }
    if (!equalJson(source.events, sourceBank.events)) return { error: "active source Timeline and bank source lighting boundary events differ" };
    return { detail: `all_white Cue ${cues.all_white.id} and all_max Cue ${cues.all_max.id} coexist as Scene Blocks from m99 downbeat ${source.loop_region.b_ms} ms through source Timeline end ${source.duration_ms} ms` };
  } catch (error) {
    return { error: String(error?.message ?? error) };
  }
}

export function validateBaseProject(baseInput) {
  const base = parseJsonValue(baseInput, "base project");
  if (base.version !== 1) fail("base project version must be exactly 1");
  if (base.app !== "Syndocal") fail("base project app must be exactly Syndocal");
  if (base.dj_track_triggers !== undefined && (!Array.isArray(base.dj_track_triggers) || base.dj_track_triggers.length !== 0)) {
    fail("base project dj_track_triggers must be absent or empty; an existing mapping is not overwritten");
  }
  if (!isObject(base.snapshot)) fail("base project snapshot is missing");
  if (!isObject(base.snapshot.timeline)) fail("base project active Timeline is missing");
  if (!Array.isArray(base.snapshot.timeline_bank) || base.snapshot.timeline_bank.length !== 1) {
    fail("base project snapshot.timeline_bank must contain exactly one imported empty Timeline");
  }
  validateEmptyTimeline(base.snapshot.timeline, "base snapshot.timeline");
  validateEmptyTimeline(base.snapshot.timeline_bank[0], "base snapshot.timeline_bank[0]");
  if (!equalJson(authoredTimelineProjection(base.snapshot.timeline), authoredTimelineProjection(base.snapshot.timeline_bank[0]))) {
    fail("base active Timeline and its sole bank entry must be the same authored empty Timeline 1");
  }
  resolveExistingLightingCues(base);
  stageDmxOutput(base);
  return base;
}
