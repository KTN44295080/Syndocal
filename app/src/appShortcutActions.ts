import {
  controlCueHotkeyIndex,
  mappingLayerToggleFromHotkey,
  mappingSelectionActionFromHotkey,
  mappingSelectionFlagFromHotkey,
  mappingSelectionManagementActionFromHotkey,
  mappingStageObjectSelectionActionFromHotkey,
  mappingStageToolFromHotkey,
  mappingViewportActionFromHotkey,
} from "./hotkeyHelpers";
import {
  controlModeForShortcut,
  setupSubTabForShortcut,
  workspaceTabForShortcut,
  type ControlMode,
  type SetupSubTab,
  type WorkspaceTab,
} from "./uiModes";

export type AppShortcutAction =
  | { kind: "newProject" }
  | { kind: "undoProject" }
  | { kind: "redoProject" }
  | { kind: "setWorkspaceTab"; tab: WorkspaceTab }
  | { kind: "selectSetupMode"; tab: SetupSubTab }
  | { kind: "toggleMappingHotkeyHelp" }
  | { kind: "closeMappingHotkeyHelp" }
  | { kind: "applyMappingSelectionManagement"; action: "pickVisible" | "clearPick" | "pickInside" | "addInside" }
  | { kind: "duplicateSelectedMappingFixtures" }
  | { kind: "setMappingStageTool"; tool: "select" | "place" | "rotate" | "pan" }
  | { kind: "toggleMappingLayer"; layer: "labels" | "beams" | "geometry" | "projectors" | "objects" | "levels" }
  | { kind: "toggleMappingSelectionFlag"; flag: "highlight" | "solo" | "park" }
  | { kind: "applyMappingViewportAction"; action: "fitVisible" | "fitSelection" | "zoomIn" | "zoomOut" | "reset" }
  | { kind: "applyMappingSelectionAction"; action: "layoutLine" | "layoutGrid" | "layoutCircle" | "layoutObjectLine" | "layoutObjectGrid" | "alignX" | "alignZ" | "distributeX" | "distributeZ" | "mirrorX" | "mirrorZ" | "rotateLeft" | "rotateRight" | "flip180" }
  | { kind: "nudgeSelectedMappingFixtures"; deltaX: number; deltaZ: number }
  | { kind: "removeSelectedMappingFixtures" }
  | { kind: "cancelMappingInteraction" }
  | { kind: "setControlMode"; mode: ControlMode }
  | { kind: "triggerPreviousCue"; enabled: boolean }
  | { kind: "triggerNextCue"; enabled: boolean }
  | { kind: "triggerCue"; cueId: number }
  | { kind: "toggleCueFadePaused"; paused: boolean | null }
  | { kind: "toggleTimelinePlayback"; operation: "play" | "pause" | "none" }
  | { kind: "toggleTimelineLoop"; enabled: boolean | null }
  | { kind: "scaleTimelineLoop"; scale: "half" | "double"; enabled: boolean }
  | { kind: "setTimelineLoopA" }
  | { kind: "setTimelineLoopB" }
  | { kind: "toggleBlackout"; enabled: boolean }
  | { kind: "toggleVideoBlackout"; enabled: boolean }
  | { kind: "tapBpm" };

/**
 * The single keyed source-of-truth for App keyboard action inventory.  The
 * checked-in control-plane JSON is mechanically derived from these exact
 * keys by `check:project-shortcuts`; keep this object literal static so a
 * shortcut cannot enter the inventory through a dynamic value.
 */
export const APP_SHORTCUT_ACTION_SOURCE_MANIFEST = {
  newProject: true,
  undoProject: true,
  redoProject: true,
  setWorkspaceTab: true,
  selectSetupMode: true,
  toggleMappingHotkeyHelp: true,
  closeMappingHotkeyHelp: true,
  applyMappingSelectionManagement: true,
  duplicateSelectedMappingFixtures: true,
  setMappingStageTool: true,
  toggleMappingLayer: true,
  toggleMappingSelectionFlag: true,
  applyMappingViewportAction: true,
  applyMappingSelectionAction: true,
  nudgeSelectedMappingFixtures: true,
  removeSelectedMappingFixtures: true,
  cancelMappingInteraction: true,
  setControlMode: true,
  triggerPreviousCue: true,
  triggerNextCue: true,
  triggerCue: true,
  toggleCueFadePaused: true,
  toggleTimelinePlayback: true,
  toggleTimelineLoop: true,
  scaleTimelineLoop: true,
  setTimelineLoopA: true,
  setTimelineLoopB: true,
  toggleBlackout: true,
  toggleVideoBlackout: true,
  tapBpm: true,
} as const satisfies Record<AppShortcutAction["kind"], true>;

export const APP_SHORTCUT_ACTION_KINDS = Object.freeze(
  Object.keys(APP_SHORTCUT_ACTION_SOURCE_MANIFEST) as AppShortcutAction["kind"][],
);

export interface AppShortcutEvent {
  code: string;
  key: string;
  repeat: boolean;
  altKey: boolean;
  ctrlKey: boolean;
  metaKey: boolean;
  shiftKey: boolean;
}

export interface AppShortcutContext {
  workspaceTab: WorkspaceTab;
  editable: boolean;
  mappingHotkeyHelpOpen: boolean;
  mappingStageTool: "select" | "place" | "rotate" | "pan";
  mappingInteractionActive: boolean;
  normalizedMappingSnapSize: number;
  cueIds: readonly number[];
  cuePadStartIndex: number;
  activeFadePaused: boolean | null;
  timelinePlaying: boolean;
  timelineDurationMs: number;
  timelineSurfaceActive: boolean;
  timelineLoopEnabled: boolean;
  timelineLoopAvailable: boolean;
  blackout: boolean;
  videoBlackout: boolean;
}

type MaybePromise = void | Promise<unknown>;

export interface AppShortcutExecutor {
  newProject: () => MaybePromise;
  undoProject: () => MaybePromise;
  redoProject: () => MaybePromise;
  setWorkspaceTab: (tab: WorkspaceTab) => MaybePromise;
  selectSetupMode: (tab: SetupSubTab) => MaybePromise;
  toggleMappingHotkeyHelp: () => MaybePromise;
  closeMappingHotkeyHelp: () => MaybePromise;
  applyMappingSelectionManagement: (action: "pickVisible" | "clearPick" | "pickInside" | "addInside") => MaybePromise;
  duplicateSelectedMappingFixtures: () => MaybePromise;
  setMappingStageTool: (tool: "select" | "place" | "rotate" | "pan") => MaybePromise;
  toggleMappingLayer: (layer: "labels" | "beams" | "geometry" | "projectors" | "objects" | "levels") => MaybePromise;
  toggleMappingSelectionFlag: (flag: "highlight" | "solo" | "park") => MaybePromise;
  applyMappingViewportAction: (action: "fitVisible" | "fitSelection" | "zoomIn" | "zoomOut" | "reset") => MaybePromise;
  applyMappingSelectionAction: (action: "layoutLine" | "layoutGrid" | "layoutCircle" | "layoutObjectLine" | "layoutObjectGrid" | "alignX" | "alignZ" | "distributeX" | "distributeZ" | "mirrorX" | "mirrorZ" | "rotateLeft" | "rotateRight" | "flip180") => MaybePromise;
  nudgeSelectedMappingFixtures: (deltaX: number, deltaZ: number) => MaybePromise;
  removeSelectedMappingFixtures: () => MaybePromise;
  cancelMappingInteraction: () => MaybePromise;
  setControlMode: (mode: ControlMode) => MaybePromise;
  triggerPreviousCue: () => MaybePromise;
  triggerNextCue: () => MaybePromise;
  triggerCue: (cueId: number) => MaybePromise;
  setCueFadePaused: (paused: boolean) => MaybePromise;
  playTimeline: () => MaybePromise;
  pauseTimeline: () => MaybePromise;
  setTimelineLoopEnabled: (enabled: boolean) => MaybePromise;
  scaleTimelineLoop: (scale: "half" | "double") => MaybePromise;
  setTimelineLoopA: () => MaybePromise;
  setTimelineLoopB: () => MaybePromise;
  setBlackout: (enabled: boolean) => MaybePromise;
  setVideoBlackout: (enabled: boolean) => MaybePromise;
  tapBpm: () => MaybePromise;
}

export function executeAppShortcut(action: AppShortcutAction, executor: AppShortcutExecutor): void {
  switch (action.kind) {
    case "newProject": void executor.newProject(); break;
    case "undoProject": void executor.undoProject(); break;
    case "redoProject": void executor.redoProject(); break;
    case "setWorkspaceTab": void executor.setWorkspaceTab(action.tab); break;
    case "selectSetupMode": void executor.selectSetupMode(action.tab); break;
    case "toggleMappingHotkeyHelp": void executor.toggleMappingHotkeyHelp(); break;
    case "closeMappingHotkeyHelp": void executor.closeMappingHotkeyHelp(); break;
    case "applyMappingSelectionManagement": void executor.applyMappingSelectionManagement(action.action); break;
    case "duplicateSelectedMappingFixtures": void executor.duplicateSelectedMappingFixtures(); break;
    case "setMappingStageTool": void executor.setMappingStageTool(action.tool); break;
    case "toggleMappingLayer": void executor.toggleMappingLayer(action.layer); break;
    case "toggleMappingSelectionFlag": void executor.toggleMappingSelectionFlag(action.flag); break;
    case "applyMappingViewportAction": void executor.applyMappingViewportAction(action.action); break;
    case "applyMappingSelectionAction": void executor.applyMappingSelectionAction(action.action); break;
    case "nudgeSelectedMappingFixtures": void executor.nudgeSelectedMappingFixtures(action.deltaX, action.deltaZ); break;
    case "removeSelectedMappingFixtures": void executor.removeSelectedMappingFixtures(); break;
    case "cancelMappingInteraction": void executor.cancelMappingInteraction(); break;
    case "setControlMode": void executor.setControlMode(action.mode); break;
    case "triggerPreviousCue": if (action.enabled) void executor.triggerPreviousCue(); break;
    case "triggerNextCue": if (action.enabled) void executor.triggerNextCue(); break;
    case "triggerCue": void executor.triggerCue(action.cueId); break;
    case "toggleCueFadePaused": if (action.paused !== null) void executor.setCueFadePaused(action.paused); break;
    case "toggleTimelinePlayback":
      if (action.operation === "pause") void executor.pauseTimeline();
      else if (action.operation === "play") void executor.playTimeline();
      break;
    case "toggleTimelineLoop": if (action.enabled !== null) void executor.setTimelineLoopEnabled(action.enabled); break;
    case "scaleTimelineLoop": if (action.enabled) void executor.scaleTimelineLoop(action.scale); break;
    case "setTimelineLoopA": void executor.setTimelineLoopA(); break;
    case "setTimelineLoopB": void executor.setTimelineLoopB(); break;
    case "toggleBlackout": void executor.setBlackout(action.enabled); break;
    case "toggleVideoBlackout": void executor.setVideoBlackout(action.enabled); break;
    case "tapBpm": void executor.tapBpm(); break;
    default: {
      const unreachable: never = action;
      return unreachable;
    }
  }
}

export function resolveAppShortcut(
  event: AppShortcutEvent,
  context: AppShortcutContext,
): AppShortcutAction | null {
  if (event.repeat) return null;

  const commandModifier = event.ctrlKey || event.metaKey;
  if (!event.altKey && commandModifier) {
    if (!event.shiftKey && event.code === "KeyN") return { kind: "newProject" };
    if (!context.editable && event.code === "KeyZ") {
      return { kind: event.shiftKey ? "redoProject" : "undoProject" };
    }
    if (!context.editable && !event.shiftKey && event.code === "KeyY") return { kind: "redoProject" };
  }

  if (!event.altKey) {
    const tab = workspaceTabForShortcut(event.code);
    if (tab) return { kind: "setWorkspaceTab", tab };
  }

  if (context.editable) return null;

  if (context.workspaceTab === "setup" && event.altKey && !commandModifier && !event.shiftKey) {
    const tab = setupSubTabForShortcut(event.code);
    return tab ? { kind: "selectSetupMode", tab } : null;
  }
  if (event.altKey) return null;

  if (context.workspaceTab === "setup") {
    if (event.key === "?" || (event.code === "Slash" && event.shiftKey)) return { kind: "toggleMappingHotkeyHelp" };
    if (event.code === "Escape" && context.mappingHotkeyHelpOpen) return { kind: "closeMappingHotkeyHelp" };

    const management = mappingSelectionManagementActionFromHotkey(event.code, event.shiftKey, commandModifier);
    if (management) return { kind: "applyMappingSelectionManagement", action: management };
    if (commandModifier && event.code === "KeyD") return { kind: "duplicateSelectedMappingFixtures" };
    if (commandModifier) return null;

    const tool = mappingStageToolFromHotkey(event.code);
    if (tool) return { kind: "setMappingStageTool", tool };
    const layer = mappingLayerToggleFromHotkey(event.code, event.shiftKey);
    if (layer) return { kind: "toggleMappingLayer", layer };
    const flag = mappingSelectionFlagFromHotkey(event.code);
    if (flag) return { kind: "toggleMappingSelectionFlag", flag };
    const stageObjectAction = mappingStageObjectSelectionActionFromHotkey(event.code, event.shiftKey);
    if (stageObjectAction) return { kind: "applyMappingSelectionManagement", action: stageObjectAction };
    const viewportAction = mappingViewportActionFromHotkey(event.code, event.shiftKey);
    if (viewportAction) return { kind: "applyMappingViewportAction", action: viewportAction };
    const selectionAction = mappingSelectionActionFromHotkey(event.code, event.shiftKey);
    if (selectionAction) return { kind: "applyMappingSelectionAction", action: selectionAction };

    const nudgeAmount = context.normalizedMappingSnapSize * (event.shiftKey ? 5 : 1);
    const nudge = event.code === "ArrowLeft" ? [-nudgeAmount, 0]
      : event.code === "ArrowRight" ? [nudgeAmount, 0]
      : event.code === "ArrowUp" ? [0, -nudgeAmount]
      : event.code === "ArrowDown" ? [0, nudgeAmount]
      : null;
    if (nudge) return { kind: "nudgeSelectedMappingFixtures", deltaX: nudge[0], deltaZ: nudge[1] };
    if (event.code === "Delete" || event.code === "Backspace") return { kind: "removeSelectedMappingFixtures" };
    if (event.code === "Escape" && context.mappingInteractionActive) return { kind: "cancelMappingInteraction" };
  }

  if (context.workspaceTab !== "control" || commandModifier) return null;

  // Timeline-local commands win while the timeline surface is active. Outside
  // that surface, L keeps its established Control-mode switch behavior.
  if (context.timelineSurfaceActive) {
    if (event.code === "KeyL" && !event.shiftKey) {
      return { kind: "toggleTimelineLoop", enabled: context.timelineLoopAvailable ? !context.timelineLoopEnabled : null };
    }
    if (event.code === "BracketLeft" && !event.shiftKey) {
      return { kind: "scaleTimelineLoop", scale: "half", enabled: context.timelineLoopAvailable };
    }
    if (event.code === "BracketRight" && !event.shiftKey) {
      return { kind: "scaleTimelineLoop", scale: "double", enabled: context.timelineLoopAvailable };
    }
    if (event.code === "KeyA" && event.shiftKey) return { kind: "setTimelineLoopA" };
    if (event.code === "KeyB" && event.shiftKey) return { kind: "setTimelineLoopB" };
  }

  const mode = controlModeForShortcut(event.code);
  if (mode) return { kind: "setControlMode", mode };
  if (event.code === "Space") {
    return { kind: event.shiftKey ? "triggerPreviousCue" : "triggerNextCue", enabled: context.cueIds.length > 0 };
  }
  if (event.shiftKey) return null;

  const cueIndex = controlCueHotkeyIndex(event.code);
  if (cueIndex !== null) {
    const cueId = context.cueIds[context.cuePadStartIndex + cueIndex];
    return cueId === undefined ? null : { kind: "triggerCue", cueId };
  }
  if (event.code === "KeyG" || event.code === "Enter") {
    return { kind: "triggerNextCue", enabled: context.cueIds.length > 0 };
  }
  if (event.code === "KeyP") return { kind: "toggleCueFadePaused", paused: context.activeFadePaused === null ? null : !context.activeFadePaused };
  if (event.code === "KeyT") {
    const operation = context.timelinePlaying ? "pause" : context.timelineDurationMs > 0 ? "play" : "none";
    return { kind: "toggleTimelinePlayback", operation };
  }
  if (event.code === "KeyB") return { kind: "toggleBlackout", enabled: !context.blackout };
  if (event.code === "KeyV") return { kind: "toggleVideoBlackout", enabled: !context.videoBlackout };
  if (event.code === "KeyK") return { kind: "tapBpm" };
  return null;
}
