import type { MidiControlMapping } from "./types";
import type { ControlMappingTarget } from "./controlMappingLearn";
import type { PerformRoutedInput } from "./controlMappingPerform";

export const audioSources = [
  { value: "Low", label: "Low band" }, { value: "Mid", label: "Mid band" },
  { value: "High", label: "High band" }, { value: "Level", label: "Audio level" },
  { value: "Peak", label: "Peak level" }, { value: "SpectralFlux", label: "Spectral flux" },
  { value: "SpectralCentroid", label: "Spectral centroid" },
  { value: "DensityFast", label: "Fast spectral density" },
  { value: "DensitySlow", label: "Slow spectral density" },
  { value: "Kick", label: "Kick strength" }, { value: "Snare", label: "Snare strength" },
  { value: "Onset", label: "Onset strength" },
] as const;
export type AudioMappingSource = typeof audioSources[number]["value"];
export interface AudioMappingSettings {
  id: string; source: AudioMappingSource; gain: number; attack_ms: number;
  release_ms: number; threshold: number; mode: "Continuous" | "Threshold" | "Onset";
}
export type AudioControlMapping = MidiControlMapping & { message: "Audio"; audio: AudioMappingSettings };
export interface AudioAssignmentSettings extends Omit<AudioMappingSettings, "id"> { low: number; high: number }
export interface AudioRuntimeValue { mappingId: string; value: number }
export interface AudioRuntimeBatch { audioGeneration: number; featureSequence: number; values: AudioRuntimeValue[] }
export interface AudioMappingObservation {
  generation: number; feature_sequence: number; running: boolean; stale: boolean;
  safety_clear_pending: boolean; bass: number; mid: number; high: number; rms: number;
  peak: number; spectral_flux: number; spectral_centroid: number; spectral_density_fast: number;
  spectral_density_slow: number; kick_strength: number; snare_strength: number;
  onset: boolean; onset_strength: number; observedAt: number;
}
export const audioMappingIntervalMs = 50;
export const audioMappingFreshnessMs = 250;
export const maxAudioMappings = 128;
const clamp01 = (value: number) => Math.max(0, Math.min(1, value));
export const audioTargetIsTrigger = (target: { action: string }) =>
  target.action === "VideoClipSlotTrigger" || target.action === "VideoTake";
export const audioTargetIsContinuous = (target: { action: string; video_param?: string | null }) =>
  (target.action === "VideoParam" && target.video_param === "Opacity")
  || target.action === "VideoLayerFade" || target.action === "VideoMaster" || target.action === "VideoIsfParam";
export const audioTargetIsEligible = (target: ControlMappingTarget) =>
  !target.audio_disabled && (audioTargetIsTrigger(target) || audioTargetIsContinuous(target));
export const isAudioMapping = (mapping: MidiControlMapping): mapping is AudioControlMapping =>
  mapping.message === "Audio" && mapping.audio !== undefined && mapping.audio !== null;
export const audioSourceLabel = (source: AudioMappingSource) =>
  audioSources.find((candidate) => candidate.value === source)?.label ?? "Audio";
export function audioMappingsFromTargets(targets: readonly ControlMappingTarget[], settings: AudioAssignmentSettings,
  makeId: () => string = () => crypto.randomUUID()): AudioControlMapping[] {
  return targets.filter(audioTargetIsEligible).map((target) => ({
    channel: null, message: "Audio", number: 0, action: target.action, fixture_id: null,
    attribute: target.attribute ?? null, group_id: target.group_id ?? null, cue_id: null,
    layer_id: target.layer_id ?? null, output_id: null, video_param: target.video_param ?? null,
    cue_point_index: null, duration_ms: null, clip_slot_id: target.clip_slot_id ?? null,
    isf_stage_id: target.isf_stage_id ?? null, low: settings.low, high: settings.high,
    audio: { id: makeId(), source: settings.source, gain: settings.gain, attack_ms: settings.attack_ms,
      release_ms: settings.release_ms, threshold: settings.threshold,
      mode: audioTargetIsTrigger(target) ? (settings.mode === "Onset" ? "Onset" : "Threshold") : "Continuous" },
  }));
}
export function audioMappingSignal(source: AudioMappingSource, levels: AudioMappingObservation): number {
  const value = ({ Low: levels.bass, Mid: levels.mid, High: levels.high, Level: levels.rms, Peak: levels.peak,
    SpectralFlux: levels.spectral_flux, SpectralCentroid: levels.spectral_centroid,
    DensityFast: levels.spectral_density_fast, DensitySlow: levels.spectral_density_slow,
    Kick: levels.kick_strength, Snare: levels.snare_strength, Onset: levels.onset_strength })[source];
  return Number.isFinite(value) ? clamp01(value) : 0;
}
export function smoothAudioMapping(previous: number, next: number, elapsedMs: number, attackMs: number, releaseMs: number) {
  const duration = next > previous ? attackMs : releaseMs;
  return duration <= 0 ? next : previous + (next - previous) * (1 - Math.exp(-Math.max(0, elapsedMs) / duration));
}
export function audioMappingValue(signal: number, gain: number, low: number, high: number) {
  return low + (high - low) * clamp01(signal * gain);
}
function validSettings(mapping: AudioControlMapping): boolean {
  const a = mapping.audio;
  return /^[A-Za-z0-9_-]{1,80}$/.test(a.id) && audioSources.some((source) => source.value === a.source)
    && [a.gain, a.attack_ms, a.release_ms, a.threshold, mapping.low, mapping.high].every(Number.isFinite)
    && a.gain >= 0 && a.gain <= 16 && a.attack_ms >= 0 && a.attack_ms <= 10_000
    && a.release_ms >= 0 && a.release_ms <= 10_000 && a.threshold >= 0 && a.threshold <= 1
    && mapping.low >= 0 && mapping.low <= 1 && mapping.high >= 0 && mapping.high <= 1
    && ["Continuous", "Threshold", "Onset"].includes(a.mode);
}
interface MappingState { signature: string; value: number; above: boolean; at: number }
/** A new source, configuration or suspension establishes an edge baseline without
 * firing. Only fresh publications trigger; continuous values renew a short native lease. */
export function createAudioMappingEvaluator() {
  const states = new Map<string, MappingState>();
  let generation: number | null = null;
  let sequence: number | null = null;
  return {
    reset() { states.clear(); generation = null; sequence = null; },
    evaluate(mappings: readonly MidiControlMapping[], observation: AudioMappingObservation | null,
      now: number, suspended = false): { batch: AudioRuntimeBatch; triggers: PerformRoutedInput[] } {
      const empty = { batch: { audioGeneration: 0, featureSequence: 0, values: [] }, triggers: [] };
      if (suspended || !observation || !observation.running || observation.stale || observation.safety_clear_pending
        || !Number.isSafeInteger(observation.generation) || observation.generation <= 0
        || !Number.isSafeInteger(observation.feature_sequence) || observation.feature_sequence <= 0
        || !Number.isFinite(now) || !Number.isFinite(observation.observedAt)
        || now < observation.observedAt || now - observation.observedAt >= audioMappingFreshnessMs) {
        this.reset(); return empty;
      }
      if (generation !== null && observation.generation < generation) {
        states.clear(); return empty;
      }
      const sourceChanged = generation !== observation.generation;
      if (sourceChanged) { states.clear(); sequence = null; }
      if (!sourceChanged && sequence !== null && observation.feature_sequence < sequence) {
        states.clear(); return empty;
      }
      const fresh = sequence !== observation.feature_sequence;
      generation = observation.generation; sequence = observation.feature_sequence;
      const values: AudioRuntimeValue[] = [];
      const triggers: PerformRoutedInput[] = [];
      const activeIds = new Set<string>();
      for (const mapping of mappings.filter(isAudioMapping).slice(0, maxAudioMappings)) {
        if (!validSettings(mapping)) continue;
        const a = mapping.audio;
        if (activeIds.has(a.id)) continue;
        activeIds.add(a.id);
        const signature = JSON.stringify(mapping);
        const before = states.get(a.id);
        const previous = before?.signature === signature ? before : undefined;
        const signal = clamp01(audioMappingSignal(a.source, observation) * a.gain);
        const smoothed = previous ? smoothAudioMapping(previous.value, signal,
          Math.min(1000, Math.max(0, now - previous.at)), a.attack_ms, a.release_ms) : signal;
        const above = previous?.above
          ? smoothed > Math.max(0, a.threshold - 0.05)
          : smoothed > 0 && smoothed >= a.threshold;
        // Duplicate publications may advance smoothing, but never consume an edge.
        states.set(a.id, { signature, value: smoothed, above: fresh ? above : previous?.above ?? above, at: now });
        if (a.mode === "Continuous" && audioTargetIsContinuous(mapping)) {
          values.push({ mappingId: a.id, value: audioMappingValue(smoothed, 1, mapping.low, mapping.high) });
        } else if (fresh && previous && audioTargetIsTrigger(mapping)
          && (a.mode === "Onset" ? observation.onset : above && !previous.above)) {
          triggers.push({ action: mapping.action as "VideoClipSlotTrigger" | "VideoTake",
            layer_id: mapping.layer_id, clip_slot_id: mapping.clip_slot_id, position: 1, value: 1,
            active: true, source: "audio", audioMappingId: a.id });
        }
      }
      for (const id of states.keys()) if (!activeIds.has(id)) states.delete(id);
      triggers.sort((a, b) => Number(a.action === "VideoTake") - Number(b.action === "VideoTake"));
      return { batch: { audioGeneration: generation, featureSequence: sequence, values }, triggers };
    },
  };
}
