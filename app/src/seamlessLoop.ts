// Crossfade length for a derived seamless loop: one second, shortened for
// short clips so the unblended middle keeps at least 100 ms. Mirrors the
// backend bounds in media_seamless_loop.rs (100 ms – 10 s, 2F + 100 ms <= L).
const DEFAULT_CROSSFADE_MS = 1_000;
const MIN_CROSSFADE_MS = 100;
const MIN_PLAIN_MS = 100;

export const seamlessLoopCrossfadeMs = (durationMs: number): number | null => {
  if (!Number.isFinite(durationMs)) return null;
  const fitted = Math.min(DEFAULT_CROSSFADE_MS, Math.floor((durationMs - MIN_PLAIN_MS) / 2));
  return fitted >= MIN_CROSSFADE_MS ? fitted : null;
};
