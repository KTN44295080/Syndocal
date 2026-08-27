import {
  TIMELINE_TEMPO_METER_MAP_VERSION,
  blocked,
  isObject,
  isSafePositiveInteger,
  passed,
  stableJson,
} from "./primitives.mjs";
import { followEnumError, followTimingError } from "./timeline.mjs";
import { parseStrictJson } from "../../app/scripts/strict-json.mjs";

export function timelineBank(project) {
  const bank = project?.snapshot?.timeline_bank;
  if (!Array.isArray(bank)) return { error: "snapshot.timeline_bank is missing" };
  if (bank.length < 2) return { error: "snapshot.timeline_bank must contain adjacent source and destination entries" };
  const ids = new Set();
  for (const [index, timeline] of bank.entries()) {
    if (!isObject(timeline) || !isSafePositiveInteger(timeline.id)) {
      return { error: `snapshot.timeline_bank[${index}] requires an explicit positive Timeline id` };
    }
    if (ids.has(timeline.id)) return { error: `snapshot.timeline_bank has duplicate Timeline id ${timeline.id}` };
    const enumError = followEnumError(timeline.follow, `snapshot.timeline_bank[${index}] Follow`);
    if (enumError) return { error: enumError };
    const timingError = followTimingError(timeline.follow, timeline, `snapshot.timeline_bank[${index}]`);
    if (timingError) return { error: timingError };
    ids.add(timeline.id);
  }
  return { bank, ids };
}

/**
 * Compare only authored Timeline fields. Transport/playhead/runtime summaries
 * are intentionally excluded because Rust clears or skips them when publishing
 * the bank; a mismatch in this projection is an authored active/bank conflict.
 */
export function authoredTimelineProjection(timeline) {
  const id = timeline?.id ?? 1;
  return {
    id,
    label: timeline?.label ?? `Timeline ${id}`,
    layers: Array.isArray(timeline?.layers) ? timeline.layers : [],
    events: Array.isArray(timeline?.events) ? timeline.events : [],
    automations: Array.isArray(timeline?.automations) ? timeline.automations : [],
    video_automations: Array.isArray(timeline?.video_automations) ? timeline.video_automations : [],
    audio: timeline?.audio ?? null,
    audio_clips: Array.isArray(timeline?.audio_clips) ? timeline.audio_clips : [],
    video_clips: Array.isArray(timeline?.video_clips) ? timeline.video_clips : [],
    phases: Array.isArray(timeline?.phases) ? timeline.phases : [],
    item_groups: Array.isArray(timeline?.item_groups) ? timeline.item_groups : [],
    loop_region: timeline?.loop_region ?? null,
    follow: timeline?.follow ?? null,
    guide_enabled: timeline?.guide_enabled ?? false,
    audio_offset_ms: timeline?.audio_offset_ms ?? 0,
    audio_muted: timeline?.audio_muted ?? false,
    metronome_enabled: timeline?.metronome_enabled ?? false,
    count_in_beats: timeline?.count_in_beats ?? 4,
    tempo_meter_map: Array.isArray(timeline?.tempo_meter_map) ? timeline.tempo_meter_map : [],
    tempo_meter_map_version: timeline?.tempo_meter_map_version ?? TIMELINE_TEMPO_METER_MAP_VERSION,
    duration_ms: timeline?.duration_ms,
  };
}

export function activeBankProjectionCheck(project, bankResult) {
  if (!bankResult.bank) return blocked("active_bank_projection", bankResult.error);
  const active = project.snapshot.timeline;
  if (!isSafePositiveInteger(active.id)) {
    return blocked("active_bank_projection", "snapshot.timeline.id must be an explicit positive ID before comparing its bank projection");
  }
  const bankActive = bankResult.bank.find((timeline) => timeline.id === active.id);
  if (!bankActive) {
    return blocked("active_bank_projection", `snapshot.timeline ID ${active.id} has no same-ID authored Timeline bank projection`);
  }
  const activeProjection = authoredTimelineProjection(active);
  const bankProjection = authoredTimelineProjection(bankActive);
  const mismatches = Object.keys(activeProjection).filter((key) =>
    stableJson(activeProjection[key]) !== stableJson(bankProjection[key]),
  );
  return mismatches.length === 0
    ? passed("active_bank_projection", `snapshot.timeline ${active.id} matches its same-ID authored bank projection; runtime-only transport fields are excluded`)
    : blocked("active_bank_projection", `snapshot.timeline ${active.id} conflicts with its same-ID authored bank projection in: ${mismatches.join(", ")}`);
}

export function shapeCheck(project) {
  if (!isObject(project)) return blocked("sdc_shape", "parsed .sdc root must be a JSON object");
  if (project.version !== 1) return blocked("sdc_shape", "current .sdc version must be exactly 1");
  if (typeof project.app !== "string" || project.app.trim() !== "Syndocal") {
    return blocked("sdc_shape", "current .sdc app must be Syndocal");
  }
  if (!isObject(project.snapshot)) return blocked("sdc_shape", "parsed .sdc root is missing snapshot");
  if (!isObject(project.snapshot.timeline)) return blocked("sdc_shape", "parsed .sdc snapshot is missing timeline");
  const enumError = followEnumError(project.snapshot.timeline.follow, "snapshot.timeline Follow");
  if (enumError) return blocked("sdc_shape", enumError);
  return passed("sdc_shape", "current JSON .sdc root/snapshot/timeline shape is readable");
}

export function projectFromInput(input) {
  if (typeof input === "string") {
    try {
      return { project: parseStrictJson(input, "Syndocal .sdc"), parseError: null };
    } catch (error) {
      return { project: null, parseError: String(error?.message ?? error) };
    }
  }
  return { project: input, parseError: null };
}
