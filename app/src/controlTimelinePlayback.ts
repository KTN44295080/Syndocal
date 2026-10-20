import type { ActiveFadeSummary, TimelineCueEventSummary } from "./types";

/**
 * Read-only timing projections for the Control > Timeline playback surface.
 * Nothing here talks to the backend: GO, Back, Release, cue triggers and the
 * Timeline transport stay the existing ticketed commands owned by App.
 */

export type CurrentCueTiming =
  | { kind: "none" }
  | { kind: "held" }
  | { kind: "fading"; remainingMs: number; progress: number; paused: boolean };

export type NextCueTiming =
  | { kind: "none" }
  | { kind: "manual" }
  | { kind: "timeline"; remainingMs: number; playing: boolean };

const finiteNonNegative = (value: number): number =>
  Number.isFinite(value) ? Math.max(0, value) : 0;

/** The current cue is fading in while the engine's active fade names it. */
export function currentCueTiming(
  activeCueId: number | null | undefined,
  activeFade: ActiveFadeSummary | null | undefined,
): CurrentCueTiming {
  if (activeCueId === null || activeCueId === undefined) return { kind: "none" };
  if (activeFade && activeFade.cue_id === activeCueId) {
    return {
      kind: "fading",
      remainingMs: finiteNonNegative(activeFade.remaining_ms),
      progress: Math.min(1, finiteNonNegative(activeFade.progress)),
      paused: activeFade.paused,
    };
  }
  return { kind: "held" };
}

/**
 * The next cue counts down only when the Timeline will itself trigger it: the
 * earliest authored event for that cue at or after the playhead. Every other
 * next cue is a manual GO.
 */
export function nextCueTiming(
  nextCueId: number | null | undefined,
  events: readonly Pick<TimelineCueEventSummary, "cue_id" | "time_ms">[],
  positionMs: number,
  playing: boolean,
): NextCueTiming {
  if (nextCueId === null || nextCueId === undefined) return { kind: "none" };
  const position = finiteNonNegative(positionMs);
  let earliest: number | null = null;
  for (const event of events) {
    if (event.cue_id !== nextCueId || !Number.isFinite(event.time_ms) || event.time_ms < position) continue;
    if (earliest === null || event.time_ms < earliest) earliest = event.time_ms;
  }
  return earliest === null ? { kind: "manual" } : { kind: "timeline", remainingMs: earliest - position, playing };
}

/** Seconds with one decimal below a minute, then M:SS. Purely numeric, so locale-invariant. */
export function formatCountdown(remainingMs: number): string {
  const totalMs = Math.floor(finiteNonNegative(remainingMs));
  if (totalMs < 60_000) return `${(Math.floor(totalMs / 100) / 10).toFixed(1)}s`;
  const totalSeconds = Math.floor(totalMs / 1_000);
  return `${Math.floor(totalSeconds / 60)}:${String(totalSeconds % 60).padStart(2, "0")}`;
}

/** Clamp a seek request to the timeline; the range input already does, this guards direct callers. */
export function clampSeekMs(positionMs: number, durationMs: number): number {
  return Math.min(finiteNonNegative(durationMs), Math.round(finiteNonNegative(positionMs)));
}
