import type { MidiOutputSummary } from "./types";

/** What the backend feedback output last handed to the MIDI driver. */
export interface MidiFeedbackOutputStatus {
  name: string;
  messages_sent: number;
  last_message: number[] | null;
  last_sent_unix_ms: number | null;
}

/**
 * Windows names the second and later ports of a multi-port USB MIDI device
 * "MIDIIN<n> (Device)" / "MIDIOUT<n> (Device)", so the output paired with such
 * an input has a deterministic name. Any other mismatch stays unpaired.
 */
const pairedWindowsOutputName = (inputName: string) => {
  const match = /^MIDIIN(\d+) \((.+)\)$/.exec(inputName);
  return match ? `MIDIOUT${match[1]} (${match[2]})` : null;
};

/** A learned input never authorizes an arbitrary first MIDI output. */
export function learnedFeedbackOutput(
  inputName: string | null,
  outputs: readonly MidiOutputSummary[],
  preservedOutputName: string | null = null,
): MidiOutputSummary | null {
  const names = preservedOutputName
    ? [preservedOutputName]
    : inputName ? [inputName, pairedWindowsOutputName(inputName)] : [];
  for (const name of names) {
    if (!name) continue;
    const candidates = outputs.filter(output => output.name === name);
    if (candidates.length > 0) return candidates.length === 1 ? candidates[0] : null;
  }
  return null;
}

export const formatMidiFeedbackMessage = (bytes: readonly number[]) =>
  bytes.map(byte => byte.toString(16).toUpperCase().padStart(2, "0")).join(" ");
