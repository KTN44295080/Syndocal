import type { MidiOutputSummary } from "./types";

/** A learned input never authorizes an arbitrary first MIDI output. */
export function learnedFeedbackOutput(
  inputName: string | null,
  outputs: readonly MidiOutputSummary[],
  preservedOutputName: string | null = null,
): MidiOutputSummary | null {
  const name = preservedOutputName ?? inputName;
  if (!name) return null;
  const candidates = outputs.filter(output => output.name === name);
  return candidates.length === 1 ? candidates[0] : null;
}
