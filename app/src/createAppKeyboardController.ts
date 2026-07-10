import type { Accessor, Setter } from "solid-js";
import {
  controlCueHotkeyIndex,
  isEditableShortcutTarget,
  mappingLayerToggleFromHotkey,
  mappingSelectionActionFromHotkey,
  mappingSelectionFlagFromHotkey,
  mappingSelectionManagementActionFromHotkey,
  mappingStageObjectSelectionActionFromHotkey,
  mappingStageToolFromHotkey,
  mappingViewportActionFromHotkey,
} from "./hotkeyHelpers";
import type { MappingDragState, MappingMarqueeState, MappingViewportPanDragState } from "./mappingRuntime";
import type { MappingStageTool } from "./mappingViewPresets";
import type { EngineSnapshot } from "./types";
import {
  controlModeForShortcut,
  setupSubTabForShortcut,
  workspaceTabForShortcut,
  type ControlMode,
  type SetupSubTab,
  type WorkspaceTab,
} from "./uiModes";

type MaybePromise = void | Promise<unknown>;

interface AppKeyboardControllerOptions {
  workspaceTab: Accessor<WorkspaceTab>;
  setWorkspaceTab: Setter<WorkspaceTab>;
  setupSubTab: Accessor<SetupSubTab>;
  selectSetupMode: (tab: SetupSubTab) => void;
  setControlMode: Setter<ControlMode>;
  setMessage: (message: string) => unknown;
  saveProject: () => MaybePromise;
  saveProjectAs: () => MaybePromise;
  loadProject: () => MaybePromise;
  newProject: () => MaybePromise;
  mappingHotkeyHelpOpen: Accessor<boolean>;
  setMappingHotkeyHelpOpen: Setter<boolean>;
  applyMappingSelectionManagementAction: (action: NonNullable<ReturnType<typeof mappingSelectionManagementActionFromHotkey>>) => void;
  duplicateSelectedMappingFixtures: () => MaybePromise;
  setMappingStageTool: Setter<MappingStageTool>;
  toggleMappingLayer: (layer: NonNullable<ReturnType<typeof mappingLayerToggleFromHotkey>>) => void;
  toggleMappingSelectionFlag: (flag: NonNullable<ReturnType<typeof mappingSelectionFlagFromHotkey>>) => void;
  applyMappingViewportAction: (action: NonNullable<ReturnType<typeof mappingViewportActionFromHotkey>>) => void;
  applyMappingSelectionAction: (action: NonNullable<ReturnType<typeof mappingSelectionActionFromHotkey>>) => void;
  normalizedMappingSnapSize: Accessor<number>;
  nudgeSelectedMappingFixtures: (deltaX: number, deltaZ: number) => MaybePromise;
  removeSelectedMappingFixtures: () => MaybePromise;
  setMappingDrag: Setter<MappingDragState | null>;
  setMappingMarquee: Setter<MappingMarqueeState | null>;
  setMappingViewportPanDrag: Setter<MappingViewportPanDragState | null>;
  snapshot: Accessor<EngineSnapshot>;
  triggerPreviousCue: () => MaybePromise;
  triggerNextCue: () => MaybePromise;
  triggerCue: (cueId: number) => MaybePromise;
  cuePadStartIndex: Accessor<number>;
  setCueFadePaused: (paused: boolean) => MaybePromise;
  playTimeline: () => MaybePromise;
  pauseTimeline: () => MaybePromise;
  setAllBlackout: (enabled: boolean) => MaybePromise;
  setBlackout: (enabled: boolean) => MaybePromise;
  setVideoBlackout: (enabled: boolean) => MaybePromise;
  tapBpm: () => MaybePromise;
}

export function createAppKeyboardController(options: AppKeyboardControllerOptions) {
  const handleControlKeyDown = (event: KeyboardEvent) => {
    if (event.repeat || event.altKey) return;

    const commandModifier = event.ctrlKey || event.metaKey;
    if (commandModifier) {
      if (event.code === "KeyS") {
        event.preventDefault();
        void (event.shiftKey ? options.saveProjectAs() : options.saveProject());
        return;
      }
      if (!event.shiftKey && event.code === "KeyO") {
        event.preventDefault();
        void options.loadProject();
        return;
      }
      if (!event.shiftKey && event.code === "KeyN") {
        event.preventDefault();
        void options.newProject();
        return;
      }
    }

    const nextWorkspaceTab = workspaceTabForShortcut(event.code);
    if (nextWorkspaceTab) {
      event.preventDefault();
      options.setWorkspaceTab(nextWorkspaceTab);
      options.setMessage(`Workspace: ${nextWorkspaceTab.toUpperCase()}.`);
      return;
    }
    if (isEditableShortcutTarget(event.target)) return;

    if (options.workspaceTab() === "setup") {
      const nextSetupSubTab = setupSubTabForShortcut(event.code);
      if (nextSetupSubTab) {
        event.preventDefault();
        options.selectSetupMode(nextSetupSubTab);
        options.setMessage(`Setup mode: ${nextSetupSubTab.toUpperCase()}.`);
        return;
      }
    }

    if (options.workspaceTab() === "setup" && options.setupSubTab() === "mapping") {
      if (event.key === "?" || (event.code === "Slash" && event.shiftKey)) {
        event.preventDefault();
        options.setMappingHotkeyHelpOpen((open) => !open);
        return;
      }
      if (event.code === "Escape" && options.mappingHotkeyHelpOpen()) {
        event.preventDefault();
        options.setMappingHotkeyHelpOpen(false);
        return;
      }
      const management = mappingSelectionManagementActionFromHotkey(event.code, event.shiftKey, commandModifier);
      if (management) {
        event.preventDefault();
        options.applyMappingSelectionManagementAction(management);
        return;
      }
      if (commandModifier && event.code === "KeyD") {
        event.preventDefault();
        void options.duplicateSelectedMappingFixtures();
        return;
      }
      if (commandModifier) return;

      const nextTool = mappingStageToolFromHotkey(event.code);
      if (nextTool) {
        event.preventDefault();
        options.setMappingStageTool(nextTool);
        options.setMessage(`2D mapping tool: ${nextTool.toUpperCase()}.`);
        return;
      }
      const layerToggle = mappingLayerToggleFromHotkey(event.code, event.shiftKey);
      if (layerToggle) {
        event.preventDefault();
        options.toggleMappingLayer(layerToggle);
        return;
      }
      const selectionFlag = mappingSelectionFlagFromHotkey(event.code);
      if (selectionFlag) {
        event.preventDefault();
        options.toggleMappingSelectionFlag(selectionFlag);
        return;
      }
      const stageObjectAction = mappingStageObjectSelectionActionFromHotkey(event.code, event.shiftKey);
      if (stageObjectAction) {
        event.preventDefault();
        options.applyMappingSelectionManagementAction(stageObjectAction);
        return;
      }
      const viewportAction = mappingViewportActionFromHotkey(event.code, event.shiftKey);
      if (viewportAction) {
        event.preventDefault();
        options.applyMappingViewportAction(viewportAction);
        return;
      }
      const selectionAction = mappingSelectionActionFromHotkey(event.code, event.shiftKey);
      if (selectionAction) {
        event.preventDefault();
        options.applyMappingSelectionAction(selectionAction);
        return;
      }
      const nudgeAmount = options.normalizedMappingSnapSize() * (event.shiftKey ? 5 : 1);
      const nudge = event.code === "ArrowLeft" ? [-nudgeAmount, 0]
        : event.code === "ArrowRight" ? [nudgeAmount, 0]
        : event.code === "ArrowUp" ? [0, -nudgeAmount]
        : event.code === "ArrowDown" ? [0, nudgeAmount]
        : null;
      if (nudge) {
        event.preventDefault();
        void options.nudgeSelectedMappingFixtures(nudge[0], nudge[1]);
        return;
      }
      if (event.code === "Delete" || event.code === "Backspace") {
        event.preventDefault();
        void options.removeSelectedMappingFixtures();
        return;
      }
      if (event.code === "Escape") {
        event.preventDefault();
        options.setMappingStageTool("select");
        options.setMappingDrag(null);
        options.setMappingMarquee(null);
        options.setMappingViewportPanDrag(null);
        return;
      }
    }

    if (options.workspaceTab() !== "control" || commandModifier) return;
    const nextControlMode = controlModeForShortcut(event.code);
    if (nextControlMode) {
      event.preventDefault();
      options.setControlMode(nextControlMode);
      options.setMessage(`Control mode: ${nextControlMode.toUpperCase()}.`);
      return;
    }
    if (event.code === "Space") {
      event.preventDefault();
      if (options.snapshot().cues.length > 0) {
        void (event.shiftKey ? options.triggerPreviousCue() : options.triggerNextCue());
      }
      return;
    }
    if (event.shiftKey) return;
    const cueHotkeyIndex = controlCueHotkeyIndex(event.code);
    if (cueHotkeyIndex !== null) {
      const cue = options.snapshot().cues[options.cuePadStartIndex() + cueHotkeyIndex];
      if (cue) {
        event.preventDefault();
        void options.triggerCue(cue.id);
      }
      return;
    }
    if (event.code === "KeyG" || event.code === "Enter") {
      event.preventDefault();
      if (options.snapshot().cues.length > 0) void options.triggerNextCue();
      return;
    }
    if (event.code === "KeyP") {
      event.preventDefault();
      if (options.snapshot().active_fade) void options.setCueFadePaused(!options.snapshot().active_fade?.paused);
      return;
    }
    if (event.code === "KeyT") {
      event.preventDefault();
      if (options.snapshot().timeline.playing) void options.pauseTimeline();
      else if (options.snapshot().timeline.duration_ms > 0) void options.playTimeline();
      return;
    }
    if (event.code === "KeyB") {
      event.preventDefault();
      void options.setBlackout(!options.snapshot().blackout);
      return;
    }
    if (event.code === "KeyV") {
      event.preventDefault();
      void options.setVideoBlackout(!options.snapshot().video.blackout);
      return;
    }
    if (event.code === "KeyK") {
      event.preventDefault();
      void options.tapBpm();
    }
  };

  return { handleControlKeyDown };
}
