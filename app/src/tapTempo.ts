import type { EngineSnapshot } from "./types";
import { clockSourceLabel } from "./clockDisplay.ts";

export function tapTempoMessage(clock: EngineSnapshot["clock"]): string {
  const external = clock.source !== "Manual" && clock.source !== "Tap";
  if (clock.tap_count < 2) {
    return external
      ? `Tap again to measure BPM. Current clock source: ${clockSourceLabel(clock.source)}.`
      : "Tap again to measure BPM.";
  }
  return external
    ? `BPM ${clock.bpm.toFixed(1)}; current clock source: ${clockSourceLabel(clock.source)}.`
    : `Tapped BPM ${clock.bpm.toFixed(1)}`;
}

export type TapTempoOptions = {
  projectEpoch: () => number;
  invokeTap: () => Promise<unknown>;
  inFlightAuthorityPoll: () => Promise<unknown> | null;
  refreshAuthority: () => Promise<unknown>;
  refreshSnapshot: () => Promise<EngineSnapshot | null>;
  applied: (clock: EngineSnapshot["clock"]) => void;
};

/** A tap changes persistent tempo and therefore the project's read identity.
 * Settle any pre-tap authority read, then obtain a fresh canonical identity
 * before reading the clock. Never replay a tap or publish an old UI cache.
 */
export async function tapTempo(options: TapTempoOptions): Promise<void> {
  const epoch = options.projectEpoch();
  await options.invokeTap();
  const prior = options.inFlightAuthorityPoll();
  if (prior) await prior;
  if (options.projectEpoch() !== epoch) return;
  await options.refreshAuthority();
  if (options.projectEpoch() !== epoch) return;
  const refreshed = await options.refreshSnapshot();
  if (refreshed === null || options.projectEpoch() !== epoch) return;
  options.applied(refreshed.clock);
}
