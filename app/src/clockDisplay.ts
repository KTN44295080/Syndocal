import type { ClockSource, EngineSnapshot } from "./types";

export const clockSourceLabel = (source: ClockSource) => {
  switch (source) {
    case "MidiClock": return "MIDI Clock";
    case "MidiTimecode": return "MTC";
    case "Ltc": return "LTC";
    case "AbletonLink": return "Ableton Link";
    default: return source;
  }
};

export const isExternalClockSource = (source: ClockSource) =>
  source === "MidiClock" || source === "MidiTimecode" || source === "Ltc" || source === "AbletonLink";

export const clockSyncStatusLabel = (clock: EngineSnapshot["clock"]) =>
  isExternalClockSource(clock.source)
    ? (clock.external_sync_locked ? "LOCK" : "STALE")
    : "INTERNAL";

export const formatShowTimecode = (positionMs: number) => {
  const totalMs = Math.max(0, Math.floor(Number.isFinite(positionMs) ? positionMs : 0));
  const milliseconds = totalMs % 1_000;
  const totalSeconds = Math.floor(totalMs / 1_000);
  const seconds = totalSeconds % 60;
  const totalMinutes = Math.floor(totalSeconds / 60);
  const minutes = totalMinutes % 60;
  const hours = Math.floor(totalMinutes / 60);
  return `${hours.toString().padStart(2, "0")}:${minutes.toString().padStart(2, "0")}:${seconds.toString().padStart(2, "0")}.${milliseconds.toString().padStart(3, "0")}`;
};

/**
 * Compact `M:SS.ff` clock (minutes : seconds . centiseconds). Used for canvas
 * time stamps on the timeline (block duration stamps, drag readouts) so no raw
 * millisecond integers are painted inside the overview. Purely numeric output
 * (digits, `:`, `.`) keeps it locale-invariant.
 */
export const formatCompactClock = (timeMs: number) => {
  const totalMs = Math.max(0, Math.floor(Number.isFinite(timeMs) ? timeMs : 0));
  const centiseconds = Math.floor((totalMs % 1_000) / 10);
  const totalSeconds = Math.floor(totalMs / 1_000);
  const seconds = totalSeconds % 60;
  const minutes = Math.floor(totalSeconds / 60);
  return `${minutes}:${seconds.toString().padStart(2, "0")}.${centiseconds.toString().padStart(2, "0")}`;
};
