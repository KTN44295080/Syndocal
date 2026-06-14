// Keyboard-shortcut helpers extracted from App.tsx: editable-target guard, cue digit
// hotkey mapping, and mapping stage-tool hotkeys. Pure; no SolidJS/state deps.
import type { MappingStageTool } from "./mappingViewPresets";

export const isEditableShortcutTarget = (target: EventTarget | null) => {
  if (!(target instanceof HTMLElement)) {
    return false;
  }
  const tagName = target.tagName.toLowerCase();
  return target.isContentEditable || tagName === "input" || tagName === "textarea" || tagName === "select";
};

export const controlCueHotkeyIndex = (code: string) => {
  const digitMatch = code.match(/^(Digit|Numpad)(\d)$/);
  if (!digitMatch) {
    return null;
  }
  const digit = Number(digitMatch[2]);
  return digit === 0 ? 9 : digit - 1;
};

export const mappingStageToolFromHotkey = (code: string): MappingStageTool | null => {
  switch (code) {
    case "KeyS":
      return "select";
    case "KeyP":
      return "place";
    case "KeyR":
      return "rotate";
    case "KeyH":
      return "pan";
    default:
      return null;
  }
};
