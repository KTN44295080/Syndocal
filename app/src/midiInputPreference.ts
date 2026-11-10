// Machine-local memory of the MIDI input the operator chose last. A controller
// often exposes several ports (USB, Bluetooth, MIDIIN2); the first enumerated
// one is rarely the one in use. Hardened WebViews may block localStorage, so
// every accessor degrades to "no preference".
import type { MidiInputSummary } from "./types";

const midiInputPreferenceKey = "syndocal.midiInputName.v1";

export const rememberMidiInputName = (name: string | null | undefined) => {
  if (typeof window === "undefined") return;
  try {
    if (name) window.localStorage.setItem(midiInputPreferenceKey, name);
    else window.localStorage.removeItem(midiInputPreferenceKey);
  } catch {
    /* storage unavailable: keep the in-memory selection only */
  }
};

const rememberedMidiInputName = (): string | null => {
  if (typeof window === "undefined") return null;
  try {
    return window.localStorage.getItem(midiInputPreferenceKey);
  } catch {
    return null;
  }
};

/** The remembered input when it is present (exact name), else the first one. */
export const defaultMidiInputIndex = (inputs: readonly MidiInputSummary[]): number | null => {
  if (inputs.length === 0) return null;
  const remembered = rememberedMidiInputName();
  return inputs.find((input) => input.name === remembered)?.index ?? inputs[0].index;
};
