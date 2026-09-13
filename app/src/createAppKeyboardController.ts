import type { Accessor, Setter } from "solid-js";
import {
  isEditableShortcutTarget,
  isNativeUndoShortcutTarget,
  mappingLayerToggleFromHotkey,
  mappingSelectionActionFromHotkey,
  mappingSelectionFlagFromHotkey,
  mappingSelectionManagementActionFromHotkey,
  mappingViewportActionFromHotkey,
} from "./hotkeyHelpers";
import { executeAppShortcut, resolveAppShortcut, type AppShortcutExecutor } from "./appShortcutActions";
import type { MappingDragState, MappingMarqueeState, MappingViewportPanDragState } from "./mappingRuntime";
import type { MappingStageTool } from "./mappingViewPresets";
import { dispatchProjectFileShortcut } from "./projectFileShortcuts";
import type { EngineSnapshot, TimelineSnapshot } from "./types";
import {
  controlModeLabel,
  type ControlMode,
  setupSubTabLabel,
  type SetupSubTab,
  type WorkspaceTab,
  workspaceLabel,
} from "./uiModes";

type MaybePromise = void | Promise<unknown>;

interface AppKeyboardControllerOptions {
  workspaceTab: Accessor<WorkspaceTab>;
  selectWorkspaceTab: (tab: WorkspaceTab) => boolean;
  setupSubTab: Accessor<SetupSubTab>;
  selectSetupMode: (tab: SetupSubTab) => void;
  setControlMode: (mode: ControlMode) => unknown;
  setMessage: (message: string) => unknown;
  saveProject: () => MaybePromise;
  saveProjectAs: () => MaybePromise;
  loadProject: () => MaybePromise;
  newProject: () => MaybePromise;
  undoProject: () => MaybePromise;
  redoProject: () => MaybePromise;
  mappingHotkeyHelpOpen: Accessor<boolean>;
  setMappingHotkeyHelpOpen: Setter<boolean>;
  applyMappingSelectionManagementAction: (action: NonNullable<ReturnType<typeof mappingSelectionManagementActionFromHotkey>>) => void;
  duplicateSelectedMappingFixtures: () => MaybePromise;
  mappingStageTool: Accessor<MappingStageTool>;
  setMappingStageTool: Setter<MappingStageTool>;
  toggleMappingLayer: (layer: NonNullable<ReturnType<typeof mappingLayerToggleFromHotkey>>) => void;
  toggleMappingSelectionFlag: (flag: NonNullable<ReturnType<typeof mappingSelectionFlagFromHotkey>>) => void;
  applyMappingViewportAction: (action: NonNullable<ReturnType<typeof mappingViewportActionFromHotkey>>) => void;
  applyMappingSelectionAction: (action: NonNullable<ReturnType<typeof mappingSelectionActionFromHotkey>>) => void;
  normalizedMappingSnapSize: Accessor<number>;
  nudgeSelectedMappingFixtures: (deltaX: number, deltaZ: number) => MaybePromise;
  removeSelectedMappingFixtures: () => MaybePromise;
  mappingDrag: Accessor<MappingDragState | null>;
  setMappingDrag: Setter<MappingDragState | null>;
  mappingMarquee: Accessor<MappingMarqueeState | null>;
  setMappingMarquee: Setter<MappingMarqueeState | null>;
  mappingViewportPanDrag: Accessor<MappingViewportPanDragState | null>;
  setMappingViewportPanDrag: Setter<MappingViewportPanDragState | null>;
  snapshot: Accessor<EngineSnapshot>;
  activeTimeline: Accessor<Pick<TimelineSnapshot, "playing" | "duration_ms">>;
  triggerPreviousCue: () => MaybePromise;
  triggerNextCue: () => MaybePromise;
  triggerCue: (cueId: number) => MaybePromise;
  cuePadStartIndex: Accessor<number>;
  setCueFadePaused: (paused: boolean) => MaybePromise;
  playTimeline: () => MaybePromise;
  pauseTimeline: () => MaybePromise;
  timelineSurfaceActive: Accessor<boolean>;
  timelineLoopEnabled: Accessor<boolean>;
  timelineLoopAvailable: Accessor<boolean>;
  setTimelineLoopEnabled: (enabled: boolean) => MaybePromise;
  scaleTimelineLoop: (scale: "half" | "double") => MaybePromise;
  setTimelineLoopA: () => MaybePromise;
  setTimelineLoopB: () => MaybePromise;
  setAllBlackout: (enabled: boolean) => MaybePromise;
  setBlackout: (enabled: boolean) => MaybePromise;
  setVideoBlackout: (enabled: boolean) => MaybePromise;
  tapBpm: () => MaybePromise;
}

export function createAppKeyboardController(options: AppKeyboardControllerOptions) {
  const shortcutExecutor: AppShortcutExecutor = {
    newProject: options.newProject,
    undoProject: options.undoProject,
    redoProject: options.redoProject,
    setWorkspaceTab: (tab) => {
      if (options.selectWorkspaceTab(tab)) options.setMessage(`Workspace: ${workspaceLabel(tab)}.`);
    },
    selectSetupMode: (tab) => {
      options.selectSetupMode(tab);
      options.setMessage(`Setup: ${setupSubTabLabel(tab)}.`);
    },
    toggleMappingHotkeyHelp: () => { options.setMappingHotkeyHelpOpen((open) => !open); },
    closeMappingHotkeyHelp: () => { options.setMappingHotkeyHelpOpen(false); },
    applyMappingSelectionManagement: options.applyMappingSelectionManagementAction,
    duplicateSelectedMappingFixtures: options.duplicateSelectedMappingFixtures,
    setMappingStageTool: (tool) => {
      options.setMappingStageTool(tool);
      options.setMessage(`2D mapping tool: ${tool.toUpperCase()}.`);
    },
    toggleMappingLayer: options.toggleMappingLayer,
    toggleMappingSelectionFlag: options.toggleMappingSelectionFlag,
    applyMappingViewportAction: options.applyMappingViewportAction,
    applyMappingSelectionAction: options.applyMappingSelectionAction,
    nudgeSelectedMappingFixtures: options.nudgeSelectedMappingFixtures,
    removeSelectedMappingFixtures: options.removeSelectedMappingFixtures,
    cancelMappingInteraction: () => {
      const hasMappingInteraction =
        options.mappingStageTool() !== "select" ||
        options.mappingDrag() !== null ||
        options.mappingMarquee() !== null ||
        options.mappingViewportPanDrag() !== null;
      if (!hasMappingInteraction) return;
      options.setMappingStageTool("select");
      options.setMappingDrag(null);
      options.setMappingMarquee(null);
      options.setMappingViewportPanDrag(null);
    },
    setControlMode: (mode) => {
      const accepted = options.setControlMode(mode);
      if (accepted !== false) options.setMessage(`Edit: ${controlModeLabel(mode)}.`);
    },
    triggerPreviousCue: options.triggerPreviousCue,
    triggerNextCue: options.triggerNextCue,
    triggerCue: options.triggerCue,
    setCueFadePaused: options.setCueFadePaused,
    playTimeline: options.playTimeline,
    pauseTimeline: options.pauseTimeline,
    setTimelineLoopEnabled: options.setTimelineLoopEnabled,
    scaleTimelineLoop: options.scaleTimelineLoop,
    setTimelineLoopA: options.setTimelineLoopA,
    setTimelineLoopB: options.setTimelineLoopB,
    setBlackout: options.setBlackout,
    setVideoBlackout: options.setVideoBlackout,
    tapBpm: options.tapBpm,
  };

  const handleControlKeyDown = (event: KeyboardEvent) => {
    if (dispatchProjectFileShortcut(event, options)) return;
    const snapshot = options.snapshot();
    const timeline = options.activeTimeline();
    const action = resolveAppShortcut(event, {
      workspaceTab: options.workspaceTab(),
      editable: isEditableShortcutTarget(event.target),
      nativeUndo: isNativeUndoShortcutTarget(event.target),
      mappingHotkeyHelpOpen: options.mappingHotkeyHelpOpen(),
      mappingStageTool: options.mappingStageTool(),
      mappingInteractionActive:
        options.mappingStageTool() !== "select" ||
        options.mappingDrag() !== null ||
        options.mappingMarquee() !== null ||
        options.mappingViewportPanDrag() !== null,
      normalizedMappingSnapSize: options.normalizedMappingSnapSize(),
      cueIds: snapshot.cues.map((cue) => cue.id),
      cuePadStartIndex: options.cuePadStartIndex(),
      activeFadePaused: snapshot.active_fade?.paused ?? null,
      timelinePlaying: timeline.playing,
      timelineDurationMs: timeline.duration_ms,
      timelineSurfaceActive: options.timelineSurfaceActive(),
      timelineLoopEnabled: options.timelineLoopEnabled(),
      timelineLoopAvailable: options.timelineLoopAvailable(),
      blackout: snapshot.blackout,
      videoBlackout: snapshot.video.blackout,
    });
    if (!action) return;
    event.preventDefault();
    executeAppShortcut(action, shortcutExecutor);
  };

  return { handleControlKeyDown };
}
