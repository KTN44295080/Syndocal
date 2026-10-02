import type { EngineSnapshot } from "./types";

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
