// Keyboard-shortcut helpers extracted from App.tsx: editable-target guard, cue digit
// hotkey mapping, and mapping stage-tool hotkeys. Pure; no SolidJS/state deps.
import type { MappingStageTool } from "./mappingViewPresets";

export type MappingLayerToggle = "labels" | "beams" | "geometry" | "projectors" | "objects" | "levels";
export type MappingSelectionFlagHotkey = "highlight" | "solo" | "park";
export type MappingSelectionManagementAction = "pickVisible" | "clearPick" | "pickInside" | "addInside";
export type MappingSelectionAction =
  | "layoutLine"
  | "layoutGrid"
  | "layoutCircle"
  | "layoutObjectLine"
  | "layoutObjectGrid"
  | "alignX"
  | "alignZ"
  | "distributeX"
  | "distributeZ"
  | "mirrorX"
  | "mirrorZ"
  | "rotateLeft"
  | "rotateRight"
  | "flip180";
export type MappingViewportAction = "fitVisible" | "fitSelection" | "zoomIn" | "zoomOut" | "reset";

export const isEditableContextMenuTarget = (target: EventTarget | null) => {
  if (!(target instanceof HTMLElement)) {
    return false;
  }
  const editableTarget = target.closest("input, textarea, [contenteditable]");
  if (!(editableTarget instanceof HTMLElement)) {
    return false;
  }
  const tagName = editableTarget.tagName.toLowerCase();
  return editableTarget.isContentEditable || tagName === "input" || tagName === "textarea";
};

export const isEditableShortcutTarget = (target: EventTarget | null) => {
  if (!(target instanceof HTMLElement)) {
    return false;
  }
  const tagName = target.tagName.toLowerCase();
  return isEditableContextMenuTarget(target) || tagName === "select";
};

/** Only text editors own the browser's native Undo/Redo stack. Keep this
 * separate from the broader shortcut guard: focused sliders, checkboxes and
 * selects must still suppress transport/tool shortcuts while allowing history. */
export const isNativeUndoShortcutTarget = (target: EventTarget | null) => {
  if (!(target instanceof HTMLElement)) return false;
  const editor = target.closest("input, textarea, [contenteditable]");
  if (!(editor instanceof HTMLElement)) return false;
  if (editor.isContentEditable) return true;
  const tagName = editor.tagName.toLowerCase();
  if (tagName === "textarea") return !(editor as HTMLTextAreaElement).readOnly;
  if (tagName !== "input") return false;
  const input = editor as HTMLInputElement;
  return !input.readOnly && ["text", "search", "url", "tel", "email", "password", "number"].includes(input.type);
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

export const mappingLayerToggleFromHotkey = (code: string, shiftKey: boolean): MappingLayerToggle | null => {
  switch (code) {
    case "KeyL":
      return "labels";
    case "KeyB":
      return "beams";
    case "KeyG":
      return "geometry";
    case "KeyV":
      return "projectors";
    case "KeyO":
      return "objects";
    case "Digit5":
    case "Numpad5":
      return shiftKey ? "levels" : null;
    default:
      return null;
  }
};

export const mappingViewportActionFromHotkey = (code: string, shiftKey: boolean): MappingViewportAction | null => {
  switch (code) {
    case "KeyF":
      return shiftKey ? "fitSelection" : "fitVisible";
    case "Equal":
    case "NumpadAdd":
      return "zoomIn";
    case "Minus":
    case "NumpadSubtract":
      return "zoomOut";
    case "Digit0":
    case "Numpad0":
      return "reset";
    default:
      return null;
  }
};

export const mappingSelectionManagementActionFromHotkey = (
  code: string,
  shiftKey: boolean,
  modifierKey: boolean,
): MappingSelectionManagementAction | null => {
  if (!modifierKey || code !== "KeyA") {
    return null;
  }
  return shiftKey ? "clearPick" : "pickVisible";
};

export const mappingStageObjectSelectionActionFromHotkey = (
  code: string,
  shiftKey: boolean,
): MappingSelectionManagementAction | null => {
  if (code !== "KeyI") {
    return null;
  }
  return shiftKey ? "addInside" : "pickInside";
};

export const mappingSelectionFlagFromHotkey = (code: string): MappingSelectionFlagHotkey | null => {
  switch (code) {
    case "KeyQ":
      return "highlight";
    case "KeyW":
      return "solo";
    case "KeyE":
      return "park";
    default:
      return null;
  }
};

export const mappingSelectionActionFromHotkey = (code: string, shiftKey: boolean): MappingSelectionAction | null => {
  switch (code) {
    case "Digit1":
    case "Numpad1":
      return shiftKey ? "layoutObjectLine" : "layoutLine";
    case "Digit2":
    case "Numpad2":
      return shiftKey ? "layoutObjectGrid" : "layoutGrid";
    case "Digit3":
    case "Numpad3":
      return shiftKey ? null : "layoutCircle";
    case "Digit4":
    case "Numpad4":
      return shiftKey ? null : "flip180";
    case "KeyX":
      return shiftKey ? "distributeX" : "alignX";
    case "KeyZ":
      return shiftKey ? "distributeZ" : "alignZ";
    case "BracketLeft":
      return shiftKey ? "mirrorX" : "rotateLeft";
    case "BracketRight":
      return shiftKey ? "mirrorZ" : "rotateRight";
    default:
      return null;
  }
};
