// Dual-format timeline time input (UI redesign T3): the inspector accepts
// both a compact clock ("M:SS.ff", "1:23.50", "0:04") and raw milliseconds
// ("83500"), so operators can type whichever form they are reading off the
// canvas stamps. Pure helpers - no Solid/state dependencies.

/** Parse "M:SS", "M:SS.ff", "H:MM:SS.ff" or plain milliseconds into ms. */
export const parseTimelineTimeInput = (raw: string): number | null => {
  const text = raw.trim();
  if (!text) return null;
  if (/^\d+$/.test(text)) {
    const ms = Number(text);
    return Number.isFinite(ms) ? ms : null;
  }
  const clock = text.match(/^(?:(\d+):)?(\d{1,2}):(\d{1,2})(?:\.(\d{1,2}))?$/);
  if (!clock) return null;
  const hours = clock[1] ? Number(clock[1]) : 0;
  const minutes = Number(clock[2]);
  const seconds = Number(clock[3]);
  // ".5" means 50 hundredths, ".05" means 5 - pad to two digits first.
  const hundredths = clock[4] ? Number(clock[4].padEnd(2, "0")) : 0;
  if (seconds >= 60 || (clock[1] !== undefined && minutes >= 60)) return null;
  return (((hours * 60 + minutes) * 60 + seconds) * 1000) + hundredths * 10;
};

/** Format ms as the same "M:SS.ff" shape the canvas duration stamps use. */
export const formatTimelineTimeInput = (timeMs: number): string => {
  const clamped = Math.max(0, Math.round(timeMs));
  const totalSeconds = Math.floor(clamped / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  const hundredths = Math.round((clamped % 1000) / 10);
  return `${minutes}:${String(seconds).padStart(2, "0")}.${String(hundredths).padStart(2, "0")}`;
};
