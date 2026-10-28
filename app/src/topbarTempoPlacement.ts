/**
 * BPM and Tap live in the single header row. Only when that row cannot hold
 * them at full size (after the project name has already been dropped, e.g. a
 * locked header at 110% UI scale in a 960px window) do they move into Tools.
 * They return once the measured free width fits their measured footprint, so
 * the placement cannot oscillate.
 */
export type TopbarTempoPlacement = "header" | "tools";

export interface TopbarTempoPlacementInput {
  placement: TopbarTempoPlacement;
  /** The header row overflows with the tempo cluster in it. */
  headerOverflows: boolean;
  /** Free width the row currently gives the project cell. */
  freeWidth: number;
  /** Footprint of the cluster (width plus its gap) when it was last in the row. */
  tempoFootprint: number;
}

export const nextTopbarTempoPlacement = (input: TopbarTempoPlacementInput): TopbarTempoPlacement => {
  if (input.placement === "header") return input.headerOverflows ? "tools" : "header";
  return input.tempoFootprint > 0 && input.freeWidth >= input.tempoFootprint ? "header" : "tools";
};
