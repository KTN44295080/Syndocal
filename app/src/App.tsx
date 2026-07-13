import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen as tauriListen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { createEffect, createMemo, createSignal, For, onCleanup, Show } from "solid-js";
import { CueManagementPanel } from "./components/CueManagementPanel";
import { AppStatusLine } from "./components/AppStatusLine";
import { ArtRdmPanel } from "./components/ArtRdmPanel";
import { CustomProfileEditorPanel } from "./components/CustomProfileEditorPanel";
import {
  type DmxAddressCell,
  type DmxPatchSegment,
  type DmxPatchViewMode,
  type DmxUniverseMap,
} from "./components/DmxPatchMapPanel";
import { DmxOutputConfigPanel } from "./components/DmxOutputConfigPanel";
import { DmxInputPanel } from "./components/DmxInputPanel";
import { DmxRawMonitor } from "./components/DmxRawMonitor";
import { EffectActionControlsPanel } from "./components/EffectActionControlsPanel";
import { EffectGroupTargetPanel } from "./components/EffectGroupTargetPanel";
import { EffectListPanel } from "./components/EffectListPanel";
import { EffectSourceControlsPanel } from "./components/EffectSourceControlsPanel";
import { FaderAttributeEditorPanel } from "./components/FaderAttributeEditorPanel";
import { FaderAuxiliaryAttributePanels } from "./components/FaderAuxiliaryAttributePanels";
import { FaderFixtureControlPanel } from "./components/FaderFixtureControlPanel";
import { FaderGridPanel } from "./components/FaderGridPanel";
import { FaderPrimaryAttributePanels } from "./components/FaderPrimaryAttributePanels";
import { LightingRuntimeControlsPanel } from "./components/LightingRuntimeControlsPanel";
import { LoadedProfileSummaryPanel } from "./components/LoadedProfileSummaryPanel";
import { MidiControlMappingPanel } from "./components/MidiControlMappingPanel";
import { NodeGraphEditorPanel } from "./components/NodeGraphEditorPanel";
import { OscControlMappingPanel } from "./components/OscControlMappingPanel";
import { OutputDiagnosticsPanel } from "./components/OutputDiagnosticsPanel";
import { type OpticsControlEntry } from "./components/OpticsControlPanel";
import { PatchFixtureFormPanel, type FixtureLayoutMode } from "./components/PatchFixtureFormPanel";
import { type PositionFavorite } from "./components/PositionControlPanel";
import { ProfileLoadPanel } from "./components/ProfileLoadPanel";
import { ProgrammerPanel } from "./components/ProgrammerPanel";
import { PlaybackExecutorPanel } from "./components/PlaybackExecutorPanel";
import { ReferencePalettePanel } from "./components/ReferencePalettePanel";
import { RemoteControlPanel } from "./components/RemoteControlPanel";
import { SampleEffectPresetPanel, sampleEffectPresetSupportsTarget, type SampleEffectPreset } from "./components/SampleEffectPresetPanel";
import { SetupMappingWorkspace } from "./components/SetupMappingWorkspace";
import { SetupVideoPanel } from "./components/SetupVideoPanel";
import { StagePreview2D } from "./components/StagePreview2D";
import { VideoControlPanel } from "./components/VideoControlPanel";
import { VideoEffectTargetPanel } from "./components/VideoEffectTargetPanel";
import { readVideoOutputTestPattern, readVideoOutputWindowId, VideoOutputWindow } from "./components/VideoOutputWindow";
import { TimelineCueEventsPanel } from "./components/TimelineCueEventsPanel";
import { TimelineLightingAutomationPanel } from "./components/TimelineLightingAutomationPanel";
import { TouchColorPalettePanel } from "./components/TouchColorPalettePanel";
import { TouchCuePanel } from "./components/TouchCuePanel";
import { TouchDimmerControlPanel } from "./components/TouchDimmerControlPanel";
import { TouchFixturePanel } from "./components/TouchFixturePanel";
import { TouchGenericAttributeGrid } from "./components/TouchGenericAttributeGrid";
import { TouchPanTiltPad } from "./components/TouchPanTiltPad";
import { TouchRemotePanel } from "./components/TouchRemotePanel";
import { TouchVideoPanel } from "./components/TouchVideoPanel";
import type { TimelineOverviewAutomationRange, TimelineOverviewEvent } from "./components/TimelineOverview";
import { WorkspaceChrome } from "./components/WorkspaceChrome";
import {
  defaultWorkspaceLayout,
  loadWorkspaceLayout,
  saveWorkspaceLayout,
} from "./workspaceLayoutStorage";
import { type ColorWheelFunctionEntry, type GoboSlotPattern, type GoboWheelFunctionEntry } from "./components/WheelSlotPanel";
import {
  customProfileAttributeDraftChannelLabel,
  customProfileAttributeDraftsFromText,
  customProfileAttributeTemplates,
  customProfileAttributeTextFromDrafts,
  customProfilePreviewFromText,
  customAttributeGeometryName,
  profileGeometryRows,
  unresolvedGeometryReferences,
  type CustomProfileAttributeDraft,
} from "./customFixtureProfile";
import {
  addressRange,
  buildOccupiedDmxRanges,
  findFreeDmxAddress,
  findNextFreePatchAddress,
  fixtureFootprint,
  rangesOverlap,
  reserveDmxAddressRange,
} from "./dmxAddressing";
import { confirmDestructiveAction } from "./destructiveActions";
import type {
  ApplicationUpdateCheck,
  ApplicationUpdateConfiguration,
  ApplicationUpdateProgress,
  AttributeControl,
  AttributeResolution,
  ArtRdmRequest,
  ArtRdmResponse,
  AudioAnalysisSummary,
  AudioSpectrumBand,
  AudioSpectrumSource,
  AutomationKeyframeSummary,
  AutomationInterpolation,
  CompositionSummary,
  CustomFixtureProfileRequest,
  CueListSummary,
  CueSummary,
  PaletteKind,
  PlaybackExecutorSummary,
  DmxOutputConfig,
  DmxInputConfig,
  DmxInputStatus,
  EffectBlendMode,
  EffectKind,
  EffectSummary,
  EngineSnapshot,
  EngineSnapshotSyncResponse,
  ExternalVideoIoPlans,
  ExternalVideoTransportDriverEvent,
  ExternalVideoTransportStatus,
  ExternalVideoTransportSyncResponse,
  ExternalVideoTransportSyncReport,
  FixtureLimits,
  FixturePresetGroupLoadResult,
  FixtureProfileSummary,
  GeometrySummary,
  LearnedMidiControl,
  LearnedOscControl,
  LfoShape,
  LiveAudioInputStatus,
  LfoEffectRequest,
  MidiControlAction,
  MidiControlMapping,
  MidiControlMessage,
  MidiInputSummary,
  MidiOutputSummary,
  NodeGraphSummary,
  NodeGraphTransformOp,
  OscControlAction,
  OscControlMapping,
  OscInputConfig,
  PatchFixtureRequest,
  PatchedFixtureSummary,
  Phase1SmokeReport,
  PositionWaveEffectRequest,
  ProjectBackupSummary,
  ProjectFile,
  ProjectHistoryStatus,
  ProjectLoadResult,
  UserTemplateLoadResult,
  ReferencePaletteSummary,
  RemoteControlConfig,
  RemoteControlStatus,
  SerialPortSummary,
  StageMapConfig,
  StageMapPresetSummary,
  StageObjectKind,
  StageObjectSummary,
  TimelineCueEventSummary,
  TimelineAutomationSummary,
  TimelineGroupAutomationAddResult,
  TimelineTrackKind,
  TimelineVideoAutomationSummary,
  UsbRdmRequest,
  VideoAutomationKeyframeSummary,
  VideoAudioMonitorStatus,
  VideoBlendMode,
  VideoBitmapMaskImportResult,
  VideoDecoderDiagnostics,
  VideoFrame,
  VideoEffectTarget,
  VideoLayerSummary,
  VideoLayerState,
  VideoOutputKind,
  VideoOutputMapping,
  VideoOutputRenderPlan,
  VideoOutputSummary,
  VideoOutputWindowCloseSummary,
  VideoOutputWindowStatus,
  VideoOutputWindowSyncSummary,
  VideoParam,
  VideoPreviewDiagnostics,
  VideoRecordingStatus,
  VideoRuntimeStatus,
  VideoSourceKind,
  VisualizerRenderPayload,
} from "./types";
import { videoFrameToDataUrl } from "./videoFrameCanvas";
import { browserViewportFixture, viewportFixtureData, viewportPatchedFixture } from "./viewportFixtureData";
import { defaultColorAdjust, defaultFxAdjust, defaultTransform } from "./videoLayerDefaults";
import {
  colorQuickLooks,
  defaultColorPalette,
  panTiltNudgeSteps,
  panTiltTargetPoints,
} from "./uiPresets";
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
import {
  applyAxisLimit,
  defaultFixtureLimits,
  dimmerValueWithinLimits,
  effectivePanTiltValues,
  normalizeLimitRange,
  sourcePanTiltValues,
  sourceValueForAxisLimit,
} from "./fixtureLimits";
import {
  opticsRoleForControl,
  quickLookValueForControl,
  quickLooksForCategory,
  type CategoryQuickLook,
} from "./controlCategory";
import {
  controlCategories,
  controlCategoryForAttribute,
  controlModeForShortcut,
  setupSubTabForShortcut,
  workspaceTabForShortcut,
  type ControlMode,
  type ControlCategory,
  type SetupSubTab,
  type WorkspaceTab,
  type EditDeskSurface,
  type TimelineDeskSurface,
} from "./uiModes";
import { projectSnapshotSignature } from "./projectSnapshot";
import { effectDraftTargetPlan } from "./effectDraft";
import { createMappingViewportModel } from "./createMappingViewportModel";
import { createMappingRenderModel } from "./createMappingRenderModel";
import { createMappingInteractionController } from "./createMappingInteractionController";
import { createMappingLayoutController, mappingFixtureSelectionCenter } from "./createMappingLayoutController";
import {
  createOutputDiagnosticsController,
  defaultOutput,
  isSerialDmxProtocol,
  outputProtocolLabel,
} from "./createOutputDiagnosticsController";
import { createInitialEngineSnapshot } from "./initialEngineSnapshot";
import { createTimelineOverviewAutomationController } from "./createTimelineOverviewAutomationController";
import { createTimelineKeyframeController } from "./createTimelineKeyframeController";
import { createTimelineAutomationController } from "./createTimelineAutomationController";
import { createVideoRuntimeController } from "./createVideoRuntimeController";
import { createAppKeyboardController } from "./createAppKeyboardController";
import { createStageMapController } from "./createStageMapController";
import { appStatusFromMessage } from "./statusModel";
import {
  bulkPatchLabel,
  colorCandidates,
  findControlAttribute,
  findControlAttributeInControls,
  readFixtureAttribute,
  type ColorControlSet,
  type ColorExtraChannelKey,
  type ColorExtraControl,
  type DimmerControlSet,
  type MovementLimitDragState,
  type MovementLimitPoint,
  type PositionControlSet,
  type TouchDimmerQuickEntry,
  type TouchDimmerQuickTarget,
  type TouchDimmerRestoreState,
} from "./fixtureControlRuntime";
import {
  surfaceWorldHalfSize,
  type MappingAxis,
  type MappingBulkGroupMode,
  type MappingDragState,
  type MappingFixtureTypeRow,
  type MappingMarqueeState,
  type MappingSvgBounds,
  type MappingViewportPanDragState,
  type VisualizerFixture,
  type VisualizerStageObject2d,
  type VisualizerVideoSurface2d,
  type WaveStageDragMode,
} from "./mappingRuntime";
import { stageObjectDefaultColor } from "./stageObjects";
import {
  defaultVideoOutputMapping,
  outputAspectRatio,
} from "./videoOutputMapping";
import {
  canLoadWheelMedia,
  wheelMediaCacheKey,
  wheelMediaPayloadToObjectUrl,
  wheelSlotMediaPath,
  type WheelMediaPayload,
} from "./wheelMedia";
import {
  clamp01,
  clampDmxValue,
  clampRange,
  cumulativeGeometryMatrix,
  defaultColorFavorites,
  dmxValueToPercent,
  finiteOr,
  formatDmxPercent,
  formatShortDmxPercent,
  geometryIdentityMatrix,
  geometryMatrixTranslation,
  hsvToRgb,
  multiplyGeometryMatrix,
  normalizedGeometryMatrix,
  normalizeHexColor,
  percentToDmxValue,
  rgbToHex,
  rgbToHsv,
  rotateStageOffsetYaw,
  valueToHexByte,
} from "./numericHelpers";
import {
  channelFunctionDetail,
  channelFunctionLabel,
  channelFunctionRangeLabel,
  channelFunctionValue,
  goboPatternForFunction,
  indexedFunctionValue,
  isColorWheelFunction,
  isGoboWheelFunction,
  normalizedFunctionText,
  pickFunctionValue,
  rankedFunctionValue,
  sortedChannelFunctions,
} from "./channelFunctionHelpers";
import {
  downloadTextFile,
  inlineComputedSvgStyles,
  safeExportFileNamePart,
  standaloneSvgExportSelectorsToRemove,
} from "./svgExportHelpers";
import {
  defaultPositionFavorites,
  loadColorFavorites,
  loadPositionFavorites,
  saveColorFavorites,
  savePositionFavorites,
} from "./favoritesStorage";
import {
  loadMappingViewPresets,
  saveMappingViewPresets,
  type MappingStageTool,
  type MappingViewPreset,
} from "./mappingViewPresets";
import {
  loadRecentProjectPaths,
  saveRecentProjectPaths,
  touchRecentProjectPath,
} from "./projectRecentStorage";
import {
  clearProjectRecoveryCheckpoint,
  createProjectRecoveryCheckpoint,
  loadProjectRecoveryCheckpoint,
  projectRecoverySourceLabel,
  projectRecoveryTimeLabel,
  saveProjectRecoveryCheckpoint,
  type ProjectRecoveryCheckpoint,
} from "./projectRecoveryStorage";
import {
  cueMetadataDraftFromSummary,
  timelineAutomationDraftFromSummary,
  timelineEventDraftFromSummary,
  timelineVideoAutomationDraftFromSummary,
  videoOutputConfigDraftFromSummary,
  type CueMetadataDraft,
  type TimelineAutomationDraft,
  type TimelineEventDraft,
  type TimelineVideoAutomationDraft,
  type VideoOutputConfigDraft,
} from "./editorDrafts";
import {
  fixtureTypeKey,
  fixtureTypeLabel,
  fixtureVisualKind,
  mappingFixtureStageSize,
  type MappingFixtureVisualKind,
} from "./fixtureVisuals";
import {
  beamPoints,
  stagePadding,
  stageViewBoxSize,
  stageWorldToSvgPoint,
  svgPointToStageWorld,
  type StageWorldBounds,
} from "./stageGeometry";
import {
  formatDuration,
  formatVideoTime,
  mediaLabelFromPath,
  shouldReplaceVideoLayerDraftLabel,
  videoSourceCanBrowseFile,
  videoSourceKindLabel,
} from "./videoHelpers";
import { clockSourceLabel, clockSyncStatusLabel, formatShowTimecode } from "./clockDisplay";
import {
  fixtureFlagClearKinds,
  fixtureFlagMappingActions,
  groupFlagMappingActions,
  isFixtureFlagMappingAction,
  isGroupFlagMappingAction,
  isVideoLayerMappingAction,
  isVideoOutputMappingAction,
  normalizeFixtureFlagClearKind,
  videoLayerMappingActions,
  videoOutputMappingActions,
  videoOutputMappingFieldOption,
  videoOutputMappingFieldOptions,
  type FixtureFlagClearKind,
  type MappingFixtureFlag,
  type NumericVideoOutputMappingField,
} from "./controlMappingActions";
import { controlMappingTargetLabel } from "./controlMappingLabels";
import { createControlInputController } from "./createControlInputController";
import {
  installUiLocalization,
  loadUiLocale,
  saveUiLocale,
  type UiLocale,
} from "./uiLocalization";
import {
  draftRangeFromKeyframes,
  evaluateTimelineKeyframes,
  interpolationForInsertedKeyframe,
  keyframesWithDraftEndpoints,
  keyframesWithInterpolationAtIndex,
  keyframesWithValueAtIndex,
  keyframesWithoutIndex,
  keyframesWithoutPlayheadKeyframe,
  movedTimelineKeyframe,
  resizedTimelineKeyframes,
  shiftedTimelineKeyframes,
  snappedTimelineKeyframes,
  sortedTimelineKeyframes,
  timelineKeyframeMoveTime,
  upsertTimelineKeyframe,
  videoAutomationValueFromState,
} from "./timelineAutomationHelpers";

const tauriBackendUnavailableMessage = "Syndocal desktop backend is not connected in this browser preview.";

const isTauriRuntime = () =>
  typeof window !== "undefined" && Boolean((window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__);

const projectHistoryChangedEvent = "syndocal:project-history-changed";

const projectMutationCommands = new Set([
  "analyze_audio_file",
  "clear_timeline_audio",
  "create_custom_fixture_profile",
  "use_fixture_profile",
  "patch_fixture",
  "patch_fixtures",
  "remove_fixture",
  "set_fixture_patch",
  "set_fixture_limits",
  "set_group_fixture_limits",
  "set_fixture_groups",
  "set_attribute",
  "set_group_attribute",
  "commit_programmer",
  "set_fixture_transform",
  "set_stage_map_config",
  "set_output_config",
  "set_dmx_outputs",
  "create_cue_from_current",
  "create_cue_list",
  "rename_cue_list",
  "remove_cue_list",
  "set_cue_list",
  "create_reference_palette",
  "update_reference_palette",
  "remove_reference_palette",
  "apply_reference_palette",
  "set_cue_palette_targets",
  "create_playback_executor",
  "update_playback_executor",
  "remove_playback_executor",
  "update_cue_from_current",
  "set_cue_metadata",
  "move_cue",
  "duplicate_cue",
  "remove_cue",
  "add_timeline_cue_event",
  "set_timeline_cue_event",
  "remove_timeline_event",
  "add_timeline_automation",
  "add_timeline_group_automation",
  "set_timeline_automation",
  "add_timeline_video_automation",
  "set_timeline_video_automation",
  "set_timeline_automation_enabled",
  "remove_timeline_automation",
  "add_video_file_layer",
  "add_still_image_layer",
  "add_local_media_layers",
  "refresh_video_layer_metadata",
  "add_video_input_layer",
  "duplicate_video_layer",
  "remove_video_layer",
  "set_video_layer_order",
  "set_video_layer_label",
  "set_video_layer_state",
  "set_video_layer_isf_effect",
  "fade_video_layer_opacity",
  "launch_video_clip",
  "take_video_clip",
  "set_video_ab_mix",
  "stop_video_clip",
  "add_video_cue_point",
  "remove_video_cue_point",
  "set_video_cue_point",
  "set_video_layer_blend_mode",
  "add_video_composition",
  "remove_video_composition",
  "set_video_composition_layers",
  "add_video_output",
  "remove_video_output",
  "set_video_output_config",
  "set_video_output_enabled",
  "set_video_output_routing",
  "set_video_output_opacity",
  "fade_video_output_opacity",
  "set_video_output_mapping",
  "set_video_output_mapping_field",
  "save_video_output_mapping_preset",
  "apply_video_output_mapping_preset",
  "remove_video_output_mapping_preset",
  "load_video_output_mapping_preset_file",
  "add_lfo_effect",
  "add_position_wave_effect",
  "update_lfo_effect",
  "update_position_wave_effect",
  "save_node_graph",
  "set_node_graph_enabled",
  "remove_node_graph",
  "load_node_graph_preset_file",
  "set_effect_enabled",
  "set_effect_video_target_position",
  "move_effect",
  "duplicate_effect",
  "remove_effect",
  "load_effect_preset",
  "load_effect_preset_for_target",
  "load_sample_effect_preset",
  "load_sample_effect_bundle",
  "load_fixture_preset",
  "load_fixture_preset_for_group",
  "load_fixture_preset_for_all_matching",
  "save_stage_map_preset",
  "apply_stage_map_preset",
  "remove_stage_map_preset",
  "add_stage_object",
  "set_stage_object",
  "remove_stage_object",
]);

const projectMutationLabel = (command: string) =>
  command
    .split("_")
    .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
    .join(" ");

const projectMutationCoalesceKey = (command: string, args?: Record<string, unknown>) => {
  if (!/^(set|update|move|fade|refresh)_/.test(command) || !args) {
    return "";
  }
  const targetEntries = Object.entries(args).filter(([key]) =>
    /(id|ids|attribute|field|index|kind|scope|universe|channel|parameter)$/i.test(key),
  );
  return targetEntries.length > 0 ? `${command}:${JSON.stringify(Object.fromEntries(targetEntries))}` : "";
};

const invoke = async <T,>(command: string, args?: Record<string, unknown>): Promise<T> => {
  if (!isTauriRuntime()) {
    throw new Error(tauriBackendUnavailableMessage);
  }
  if (!projectMutationCommands.has(command)) {
    return tauriInvoke<T>(command, args);
  }
  const transactionId = await tauriInvoke<number>("begin_project_transaction", {
    label: projectMutationLabel(command),
    coalesceKey: projectMutationCoalesceKey(command, args),
  });
  try {
    const result = await tauriInvoke<T>(command, args);
    const status = await tauriInvoke<ProjectHistoryStatus>("commit_project_transaction", { transactionId });
    window.dispatchEvent(new CustomEvent<ProjectHistoryStatus>(projectHistoryChangedEvent, { detail: status }));
    return result;
  } catch (error) {
    await tauriInvoke("cancel_project_transaction", { transactionId }).catch(() => undefined);
    throw error;
  }
};

const listen = <T,>(event: string, handler: (event: { payload: T }) => void) => {
  if (!isTauriRuntime()) {
    return Promise.reject(new Error(tauriBackendUnavailableMessage));
  }
  return tauriListen<T>(event, handler);
};

const isSyndocalProjectPath = (path: string) => path.trim().toLowerCase().endsWith(".sdc");
const uiScaleStorageKey = "syndocal.uiScale.v1";
type UiScale = 90 | 100 | 110;
const loadUiScale = (): UiScale => {
  try {
    const value = Number.parseInt(localStorage.getItem(uiScaleStorageKey) ?? "100", 10);
    return value === 90 || value === 110 ? value : 100;
  } catch {
    return 100;
  }
};

const profileLoadMessage = (prefix: string, profile: FixtureProfileSummary) => {
  const warningSuffix =
    profile.warnings.length > 0
      ? ` (${profile.warnings.length} warning${profile.warnings.length === 1 ? "" : "s"})`
      : "";
  return `${prefix} ${profile.manufacturer} ${profile.name}${warningSuffix}`;
};

type TimelineSnapMode = "Off" | "Beat" | "Bar" | "Grid";
type VideoOutputPreviewMode = "output" | "test";
type EffectTargetMode = "fixture" | "selection" | "group" | "video";
type CueCaptureScopeMode = "all" | "lighting" | "selectedFixture" | "selectedGroup" | "video";
type TimelineLightingAutomationRowScope = "all" | "current";
type TimelineVideoAutomationRowScope = "all" | "layer";
type SelectedTimelineAutomation = {
  kind: "lighting" | "video";
  automationId: number;
};

interface EffectTargetOverride {
  fixture_ids: number[];
  target_group_ids: string[];
  attribute: string;
  video_targets: VideoEffectTarget[];
}

type CueCaptureScopeRequest =
  | { kind: "all" }
  | { kind: "lightingOnly" }
  | { kind: "selectedFixture"; fixtureId: number }
  | { kind: "selectedGroup"; groupId: string }
  | { kind: "videoOnly" };

const cuePadSize = 10;
const defaultCustomAttributesText = "Dimmer@1:8, Pan@2:16, Tilt@4:16, ColorRed@6:8, ColorGreen@7:8, ColorBlue@8:8";


export default function App() {
  const outputWindowId = readVideoOutputWindowId();
  if (outputWindowId !== null) {
    return <VideoOutputWindow outputId={outputWindowId} testPattern={readVideoOutputTestPattern()} />;
  }

  const setupPanelRefs: Partial<Record<SetupSubTab, HTMLElement>> = {};
  const initialWorkspaceLayout = loadWorkspaceLayout();
  const [gdtfPath, setGdtfPath] = createSignal("");
  const [gdtfShareUrl, setGdtfShareUrl] = createSignal("");
  const [currentProjectPath, setCurrentProjectPath] = createSignal<string | null>(null);
  const [projectDirty, setProjectDirty] = createSignal(false);
  const [projectDropState, setProjectDropState] = createSignal<"project" | "invalid" | null>(null);
  const [recentProjectPaths, setRecentProjectPaths] = createSignal<string[]>(loadRecentProjectPaths());
  const [projectRecoveryCheckpoint, setProjectRecoveryCheckpoint] = createSignal<ProjectRecoveryCheckpoint | null>(
    loadProjectRecoveryCheckpoint(),
  );
  const [projectBackups, setProjectBackups] = createSignal<ProjectBackupSummary[]>([]);
  const [applicationUpdateConfiguration, setApplicationUpdateConfiguration] =
    createSignal<ApplicationUpdateConfiguration | null>(null);
  const [applicationUpdateCheck, setApplicationUpdateCheck] = createSignal<ApplicationUpdateCheck | null>(null);
  const [applicationUpdateProgress, setApplicationUpdateProgress] = createSignal<ApplicationUpdateProgress | null>(null);
  const [applicationUpdateBusy, setApplicationUpdateBusy] = createSignal(false);
  const [applicationUpdateError, setApplicationUpdateError] = createSignal<string | null>(null);
  const [projectHistoryStatus, setProjectHistoryStatus] = createSignal<ProjectHistoryStatus>({
    can_undo: false,
    can_redo: false,
    undo_depth: 0,
    redo_depth: 0,
    undo_label: null,
    redo_label: null,
  });
  const [cleanProjectSignature, setCleanProjectSignature] = createSignal<string | null>(null);
  const [workspaceTab, setWorkspaceTab] = createSignal<WorkspaceTab>(initialWorkspaceLayout.workspace_tab);
  const [uiScale, setUiScale] = createSignal<UiScale>(loadUiScale());
  const [uiLocale, setUiLocale] = createSignal<UiLocale>(loadUiLocale());
  const [setupSubTab, setSetupSubTab] = createSignal<SetupSubTab>(initialWorkspaceLayout.setup_sub_tab);
  const [controlMode, setControlMode] = createSignal<ControlMode>(initialWorkspaceLayout.control_mode);
  const [timelineDeskSurface, setTimelineDeskSurface] = createSignal<TimelineDeskSurface>(
    initialWorkspaceLayout.timeline_desk_surface,
  );
  const [editDeskSurface, setEditDeskSurface] = createSignal<EditDeskSurface>(initialWorkspaceLayout.edit_desk_surface);
  const [profile, setProfile] = createSignal<FixtureProfileSummary | null>(null);
  const [selectedMode, setSelectedMode] = createSignal("");
  const [customManufacturer, setCustomManufacturer] = createSignal("Syndocal");
  const [customProfileName, setCustomProfileName] = createSignal("Custom Fixture");
  const [customModeName, setCustomModeName] = createSignal("Default");
  const [customAttributes, setCustomAttributes] = createSignal(defaultCustomAttributesText);
  const [customAttributeDrafts, setCustomAttributeDrafts] = createSignal<CustomProfileAttributeDraft[]>(
    customProfileAttributeDraftsFromText(defaultCustomAttributesText),
  );
  const [selectedCustomAttributeIndex, setSelectedCustomAttributeIndex] = createSignal<number | null>(null);
  const [label, setLabel] = createSignal("Fixture 1");
  const [address, setAddress] = createSignal(1);
  const [universe, setUniverse] = createSignal(0);
  const [patchCount, setPatchCount] = createSignal(1);
  const [patchAddressStride, setPatchAddressStride] = createSignal(0);
  const [patchLayoutMode, setPatchLayoutMode] = createSignal<FixtureLayoutMode>("line");
  const [patchGridColumns, setPatchGridColumns] = createSignal(4);
  const [patchCircleRadius, setPatchCircleRadius] = createSignal(4);
  const [patchX, setPatchX] = createSignal(0);
  const [patchY, setPatchY] = createSignal(0);
  const [patchZ, setPatchZ] = createSignal(0);
  const [patchXStep, setPatchXStep] = createSignal(1);
  const [patchZStep, setPatchZStep] = createSignal(0);
  const [patchPitch, setPatchPitch] = createSignal(0);
  const [patchYaw, setPatchYaw] = createSignal(0);
  const [patchRoll, setPatchRoll] = createSignal(0);
  const [groupText, setGroupText] = createSignal("");
  const [selectedFixtureGroupFilter, setSelectedFixtureGroupFilter] = createSignal<string | null>(null);
  const [selectedFixtureLabelDraft, setSelectedFixtureLabelDraft] = createSignal("");
  const [selectedFixtureUniverseDraft, setSelectedFixtureUniverseDraft] = createSignal(0);
  const [selectedFixtureAddressDraft, setSelectedFixtureAddressDraft] = createSignal(1);
  const [selectedFixtureGroupText, setSelectedFixtureGroupText] = createSignal("");
  const [selectedFixtureLimitsDraft, setSelectedFixtureLimitsDraft] = createSignal<FixtureLimits>(defaultFixtureLimits);
  const [movementLimitDrag, setMovementLimitDrag] = createSignal<MovementLimitDragState | null>(null);
  const [selectedFixtureId, setSelectedFixtureId] = createSignal<number | null>(null);
  const [selectedMappingFixtureIds, setSelectedMappingFixtureIds] = createSignal<number[]>([]);
  const [mappingSelectionGroupText, setMappingSelectionGroupText] = createSignal("");
  const [mappingStageTool, setMappingStageTool] = createSignal<MappingStageTool>("select");
  const [mappingHotkeyHelpOpen, setMappingHotkeyHelpOpen] = createSignal(false);
  const [mappingViewportZoom, setMappingViewportZoom] = createSignal(1);
  const [mappingViewportCenterX, setMappingViewportCenterX] = createSignal(stageViewBoxSize / 2);
  const [mappingViewportCenterZ, setMappingViewportCenterZ] = createSignal(stageViewBoxSize / 2);
  const [mappingSnapEnabled, setMappingSnapEnabled] = createSignal(false);
  const [mappingSnapSize, setMappingSnapSize] = createSignal(0.5);
  const [mappingFixtureSearch, setMappingFixtureSearch] = createSignal("");
  const [mappingStageCursorWorld, setMappingStageCursorWorld] = createSignal<{ x: number; z: number } | null>(null);
  const [selectedFixtureTypeFilter, setSelectedFixtureTypeFilter] = createSignal<string | null>(null);
  const [selectedVideoOutputId, setSelectedVideoOutputId] = createSignal<number | null>(null);
  const [mappingShowLabels, setMappingShowLabels] = createSignal(true);
  const [mappingShowBeams, setMappingShowBeams] = createSignal(true);
  const [mappingShowProjectors, setMappingShowProjectors] = createSignal(true);
  const [mappingShowLevels, setMappingShowLevels] = createSignal(false);
  const [mappingShowGeometry, setMappingShowGeometry] = createSignal(false);
  const [mappingShowStageObjects, setMappingShowStageObjects] = createSignal(true);
  const [selectedStageObjectId, setSelectedStageObjectId] = createSignal<number | null>(null);
  const [stageObjectLabel, setStageObjectLabel] = createSignal("Stage reference");
  const [stageObjectKind, setStageObjectKind] = createSignal<StageObjectKind>("Stage");
  const [stageObjectWidth, setStageObjectWidth] = createSignal(8);
  const [stageObjectDepth, setStageObjectDepth] = createSignal(4);
  const [stageObjectRotation, setStageObjectRotation] = createSignal(0);
  const [stageObjectColor, setStageObjectColor] = createSignal(stageObjectDefaultColor("Stage"));
  const [mappingDrag, setMappingDrag] = createSignal<MappingDragState | null>(null);
  const [mappingMarquee, setMappingMarquee] = createSignal<MappingMarqueeState | null>(null);
  const [mappingViewportPanDrag, setMappingViewportPanDrag] = createSignal<MappingViewportPanDragState | null>(null);
  const [patchGridUniverse, setPatchGridUniverse] = createSignal(0);
  const [dmxPatchViewMode, setDmxPatchViewMode] = createSignal<DmxPatchViewMode>("grid");
  const [controlCategory, setControlCategory] = createSignal<ControlCategory>(initialWorkspaceLayout.control_category);
  const [panTiltNudgeAmount, setPanTiltNudgeAmount] = createSignal(2048);
  const [positionFavoriteLabel, setPositionFavoriteLabel] = createSignal("");
  const [positionFavorites, setPositionFavorites] = createSignal<PositionFavorite[]>(loadPositionFavorites());
  const [colorAutoWhite, setColorAutoWhite] = createSignal(false);
  const [colorFavorites, setColorFavorites] = createSignal<string[]>(loadColorFavorites());
  const [wheelMediaUrls, setWheelMediaUrls] = createSignal<Record<string, string>>({});
  const [wheelMediaLoading, setWheelMediaLoading] = createSignal<Record<string, true>>({});
  const [wheelMediaMissing, setWheelMediaMissing] = createSignal<Record<string, true>>({});
  const [faderValues, setFaderValues] = createSignal<Record<string, number>>({});
  const [rawDmxUniverse, setRawDmxUniverse] = createSignal(0);
  const [bpmDraft, setBpmDraft] = createSignal("120");
  const [midiInputs, setMidiInputs] = createSignal<MidiInputSummary[]>([]);
  const [midiOutputs, setMidiOutputs] = createSignal<MidiOutputSummary[]>([]);
  const [selectedMidiInput, setSelectedMidiInput] = createSignal<number | null>(null);
  const [selectedMidiOutput, setSelectedMidiOutput] = createSignal<number | null>(null);
  const [midiConnected, setMidiConnected] = createSignal(false);
  const [midiControlConnected, setMidiControlConnected] = createSignal(false);
  const [midiFeedbackConnected, setMidiFeedbackConnected] = createSignal(false);
  const [midiFeedbackEnabled, setMidiFeedbackEnabled] = createSignal(false);
  const [midiMappings, setMidiMappings] = createSignal<MidiControlMapping[]>([]);
  const [midiMapMessage, setMidiMapMessage] = createSignal<MidiControlMessage>("ControlChange");
  const [midiMapChannel, setMidiMapChannel] = createSignal(-1);
  const [midiMapNumber, setMidiMapNumber] = createSignal(7);
  const [midiMapAction, setMidiMapAction] = createSignal<MidiControlAction>("FixtureAttribute");
  const [midiMapAttribute, setMidiMapAttribute] = createSignal("Dimmer");
  const [midiMapGroupId, setMidiMapGroupId] = createSignal("front");
  const [midiMapCueId, setMidiMapCueId] = createSignal<number | null>(null);
  const [midiMapEffectId, setMidiMapEffectId] = createSignal<number | null>(null);
  const [midiMapNodeGraphId, setMidiMapNodeGraphId] = createSignal<number | null>(null);
  const [midiMapLayerId, setMidiMapLayerId] = createSignal<number | null>(null);
  const [midiMapVideoOutputId, setMidiMapVideoOutputId] = createSignal<number | null>(null);
  const [midiMapVideoOutputMappingField, setMidiMapVideoOutputMappingField] = createSignal("keystone_x");
  const [midiMapVideoOutputMappingPresetLabel, setMidiMapVideoOutputMappingPresetLabel] = createSignal("");
  const [midiMapVideoParam, setMidiMapVideoParam] = createSignal<VideoParam>("Opacity");
  const [midiMapCuePointIndex, setMidiMapCuePointIndex] = createSignal(0);
  const [midiMapDurationMs, setMidiMapDurationMs] = createSignal(1000);
  const [midiMapLow, setMidiMapLow] = createSignal(0);
  const [midiMapHigh, setMidiMapHigh] = createSignal(65535);
  const [serialPorts, setSerialPorts] = createSignal<SerialPortSummary[]>([]);
  const [dmxInputConfig, setDmxInputConfig] = createSignal<DmxInputConfig>({
    protocol: "ArtNet",
    bind_ip: "0.0.0.0",
    port: 6454,
    universe: 0,
    merge_mode: "Htp",
    timeout_ms: 2500,
  });
  const [dmxInputStatus, setDmxInputStatus] = createSignal<DmxInputStatus>({
    running: false,
    signal_present: false,
    packets_received: 0,
    invalid_packets: 0,
    last_packet_unix_ms: null,
    source_address: null,
  });
  const [oscBindIp, setOscBindIp] = createSignal("0.0.0.0");
  const [oscPort, setOscPort] = createSignal(9000);
  const [oscRunning, setOscRunning] = createSignal(false);
  const [oscMappings, setOscMappings] = createSignal<OscControlMapping[]>([]);
  const [oscMapAddress, setOscMapAddress] = createSignal("/touchosc/fader1");
  const [oscMapAction, setOscMapAction] = createSignal<OscControlAction>("FixtureAttribute");
  const [oscMapAttribute, setOscMapAttribute] = createSignal("Dimmer");
  const [oscMapGroupId, setOscMapGroupId] = createSignal("front");
  const [oscMapCueId, setOscMapCueId] = createSignal<number | null>(null);
  const [oscMapEffectId, setOscMapEffectId] = createSignal<number | null>(null);
  const [oscMapNodeGraphId, setOscMapNodeGraphId] = createSignal<number | null>(null);
  const [oscMapLayerId, setOscMapLayerId] = createSignal<number | null>(null);
  const [oscMapVideoOutputId, setOscMapVideoOutputId] = createSignal<number | null>(null);
  const [oscMapVideoOutputMappingField, setOscMapVideoOutputMappingField] = createSignal("keystone_x");
  const [oscMapVideoOutputMappingPresetLabel, setOscMapVideoOutputMappingPresetLabel] = createSignal("");
  const [oscMapVideoParam, setOscMapVideoParam] = createSignal<VideoParam>("Opacity");
  const [oscMapCuePointIndex, setOscMapCuePointIndex] = createSignal(0);
  const [oscMapDurationMs, setOscMapDurationMs] = createSignal(1000);
  const [oscMapLow, setOscMapLow] = createSignal(0);
  const [oscMapHigh, setOscMapHigh] = createSignal(65535);
  const [remoteBindIp, setRemoteBindIp] = createSignal("127.0.0.1");
  const [remotePort, setRemotePort] = createSignal(9100);
  const createPairingPin = () => {
    const value = globalThis.crypto
      ? globalThis.crypto.getRandomValues(new Uint32Array(1))[0]
      : Math.floor(Math.random() * 1_000_000);
    return String(value % 1_000_000).padStart(6, "0");
  };
  const [remotePairingPin, setRemotePairingPin] = createSignal(createPairingPin());
  const [remoteAllowLan, setRemoteAllowLan] = createSignal(false);
  const [remoteMaxConnections, setRemoteMaxConnections] = createSignal(8);
  const [remoteMaxMessageBytes, setRemoteMaxMessageBytes] = createSignal(64 * 1024);
  const [remoteMaxMessagesPerSecond, setRemoteMaxMessagesPerSecond] = createSignal(60);
  const [remoteRunning, setRemoteRunning] = createSignal(false);
  const [remoteAccessUrls, setRemoteAccessUrls] = createSignal<string[]>([]);
  const [remoteStatus, setRemoteStatus] = createSignal<RemoteControlStatus>({
    running: false,
    active_connections: 0,
    rejected_connections: 0,
    clients: [],
  });
  const [cueLabel, setCueLabel] = createSignal("Cue 1");
  const [cueFadeMs, setCueFadeMs] = createSignal(1000);
  const [cueCaptureScope, setCueCaptureScope] = createSignal<CueCaptureScopeMode>("all");
  const [selectedCueListId, setSelectedCueListId] = createSignal(1);
  const [cueListLabel, setCueListLabel] = createSignal("Main");
  const [cueMetadataDrafts, setCueMetadataDrafts] = createSignal<Record<number, CueMetadataDraft>>({});
  const [cuePadBank, setCuePadBank] = createSignal(0);
  const [cuePadFollowActive, setCuePadFollowActive] = createSignal(true);
  const [timelineCueId, setTimelineCueId] = createSignal<number | null>(null);
  const [timelineEventTimeMs, setTimelineEventTimeMs] = createSignal(0);
  const [timelineTrack, setTimelineTrack] = createSignal<TimelineTrackKind>("Lighting");
  const [timelineEventDrafts, setTimelineEventDrafts] = createSignal<Record<number, TimelineEventDraft>>({});
  const [timelineSnapMode, setTimelineSnapMode] = createSignal<TimelineSnapMode>("Off");
  const [timelineGridMs, setTimelineGridMs] = createSignal(500);
  const [selectedTimelineAutomation, setSelectedTimelineAutomation] =
    createSignal<SelectedTimelineAutomation | null>(null);
  const [timelineAutomationDrafts, setTimelineAutomationDrafts] = createSignal<Record<number, TimelineAutomationDraft>>({});
  const [lightingAutomationRowScope, setLightingAutomationRowScope] =
    createSignal<TimelineLightingAutomationRowScope>("all");
  const [automationStartMs, setAutomationStartMs] = createSignal(0);
  const [automationEndMs, setAutomationEndMs] = createSignal(1000);
  const [automationStartValue, setAutomationStartValue] = createSignal(0);
  const [automationEndValue, setAutomationEndValue] = createSignal(65535);
  const [automationInterpolation, setAutomationInterpolation] = createSignal<AutomationInterpolation>("Linear");
  const [videoLabel, setVideoLabel] = createSignal("Video Layer 1");
  const [videoSourceKind, setVideoSourceKind] = createSignal<VideoSourceKind>("File");
  const [videoPath, setVideoPath] = createSignal("");
  const [videoPreviewInfo, setVideoPreviewInfo] = createSignal("No preview");
  const [videoPreviewUrl, setVideoPreviewUrl] = createSignal("");
  const [videoClipThumbnails, setVideoClipThumbnails] = createSignal<Record<number, string>>({});
  const [videoAudioMonitorStatus, setVideoAudioMonitorStatus] = createSignal<VideoAudioMonitorStatus>({
    output_open: false,
    active_layer_ids: [],
    resync_count: 0,
    last_drift_ms: 0,
    max_abs_drift_ms: 0,
    last_sync_error: null,
  });
  const [videoAudioMonitorVolume, setVideoAudioMonitorVolume] = createSignal(0.8);
  const [videoProgramAudioEnabled, setVideoProgramAudioEnabled] = createSignal(false);
  const [audioOutputDevices, setAudioOutputDevices] = createSignal<string[]>([]);
  const [selectedAudioOutputDevice, setSelectedAudioOutputDevice] = createSignal("");
  const [videoDeckALayerId, setVideoDeckALayerId] = createSignal<number | null>(null);
  const [videoDeckBLayerId, setVideoDeckBLayerId] = createSignal<number | null>(null);
  const [videoAbMix, setVideoAbMix] = createSignal(0);
  const [videoRecordingStatus, setVideoRecordingStatus] = createSignal<VideoRecordingStatus>({
    active: false,
    output_id: null,
    path: null,
    width: 0,
    height: 0,
    frame_rate: 30,
    frames_written: 0,
    dropped_frames: 0,
    audio_requested: false,
    audio_included: false,
    audio_track_count: 0,
    started_unix_ms: null,
    last_error: null,
  });
  const [liveAudioInputDevices, setLiveAudioInputDevices] = createSignal<string[]>([]);
  const [selectedLiveAudioInputDevice, setSelectedLiveAudioInputDevice] = createSignal("");
  const [liveAudioInputStatus, setLiveAudioInputStatus] = createSignal<LiveAudioInputStatus>({
    running: false,
    device_name: null,
    sample_rate: 0,
    channels: 0,
    bass: 0,
    mid: 0,
    high: 0,
    analyzed_windows: 0,
    dropped_chunks: 0,
    last_error: null,
  });
  const [videoPreviewDiagnostics, setVideoPreviewDiagnostics] = createSignal<VideoPreviewDiagnostics | null>(null);
  const [videoOutputRenderPlans, setVideoOutputRenderPlans] = createSignal<VideoOutputRenderPlan[] | null>(null);
  const [videoOutputWindowStatuses, setVideoOutputWindowStatuses] = createSignal<VideoOutputWindowStatus[] | null>(null);
  const [videoRuntimeStatus, setVideoRuntimeStatus] = createSignal<VideoRuntimeStatus | null>(null);
  const [externalVideoIoPlans, setExternalVideoIoPlans] = createSignal<ExternalVideoIoPlans | null>(null);
  const [externalVideoTransportStatus, setExternalVideoTransportStatus] =
    createSignal<ExternalVideoTransportStatus | null>(null);
  const [externalVideoTransportReport, setExternalVideoTransportReport] =
    createSignal<ExternalVideoTransportSyncReport | null>(null);
  const [externalVideoTransportEvents, setExternalVideoTransportEvents] =
    createSignal<ExternalVideoTransportDriverEvent[]>([]);
  const [videoOutputPreviewInfo, setVideoOutputPreviewInfo] = createSignal("No output preview");
  const [videoOutputPreviewUrl, setVideoOutputPreviewUrl] = createSignal("");
  const [videoOutputPreviewId, setVideoOutputPreviewId] = createSignal<number | null>(null);
  const [videoOutputPreviewMode, setVideoOutputPreviewMode] = createSignal<VideoOutputPreviewMode>("output");
  const [phase1SmokeReport, setPhase1SmokeReport] = createSignal<Phase1SmokeReport | null>(null);
  const [videoOutputLabel, setVideoOutputLabel] = createSignal("Video Output 1");
  const [videoOutputKind, setVideoOutputKind] = createSignal<VideoOutputKind>("Display");
  const [videoOutputWidth, setVideoOutputWidth] = createSignal(1920);
  const [videoOutputHeight, setVideoOutputHeight] = createSignal(1080);
  const [videoOutputFullscreen, setVideoOutputFullscreen] = createSignal(true);
  const [videoOutputMonitorId, setVideoOutputMonitorId] = createSignal(0);
  const [videoOutputEndpoint, setVideoOutputEndpoint] = createSignal("");
  const [videoOutputConfigDrafts, setVideoOutputConfigDrafts] = createSignal<Record<number, VideoOutputConfigDraft>>({});
  const [videoOutputFadeMs, setVideoOutputFadeMs] = createSignal(1000);
  const [videoOutputMappingPresetLabel, setVideoOutputMappingPresetLabel] = createSignal("Output mapping preset");
  const [selectedVideoOutputMappingPresetLabel, setSelectedVideoOutputMappingPresetLabel] = createSignal("");
  const [stageMapPresetLabel, setStageMapPresetLabel] = createSignal("Stage map preset");
  const [selectedStageMapPresetLabel, setSelectedStageMapPresetLabel] = createSignal("");
  const [mappingViewPresetLabel, setMappingViewPresetLabel] = createSignal("Current view");
  const [selectedMappingViewPresetId, setSelectedMappingViewPresetId] = createSignal("overview");
  const [mappingViewPresets, setMappingViewPresets] = createSignal<MappingViewPreset[]>(loadMappingViewPresets());
  let mappingStageSvgElement: SVGSVGElement | undefined;
  const [videoCompositionLabel, setVideoCompositionLabel] = createSignal("Composition 1");
  const [videoCompositionLayerIds, setVideoCompositionLayerIds] = createSignal<number[]>([]);
  const [videoAutomationLayerId, setVideoAutomationLayerId] = createSignal<number | null>(null);
  const [videoAutomationParam, setVideoAutomationParam] = createSignal<VideoParam>("Opacity");
  const [videoAutomationStartMs, setVideoAutomationStartMs] = createSignal(0);
  const [videoAutomationEndMs, setVideoAutomationEndMs] = createSignal(1000);
  const [videoAutomationStartValue, setVideoAutomationStartValue] = createSignal(1);
  const [videoAutomationEndValue, setVideoAutomationEndValue] = createSignal(0);
  const [videoAutomationInterpolation, setVideoAutomationInterpolation] =
    createSignal<AutomationInterpolation>("Linear");
  const [timelineVideoAutomationDrafts, setTimelineVideoAutomationDrafts] = createSignal<
    Record<number, TimelineVideoAutomationDraft>
  >({});
  const [videoAutomationRowScope, setVideoAutomationRowScope] =
    createSignal<TimelineVideoAutomationRowScope>("all");
  const [effectShape, setEffectShape] = createSignal<LfoShape>("Sine");
  const [effectType, setEffectType] = createSignal<EffectKind>("Lfo");
  const [effectTargetMode, setEffectTargetMode] = createSignal<EffectTargetMode>("fixture");
  const [effectTargetGroups, setEffectTargetGroups] = createSignal("");
  const [effectVideoLayerId, setEffectVideoLayerId] = createSignal<number | null>(null);
  const [effectVideoParam, setEffectVideoParam] = createSignal<VideoParam>("Opacity");
  const [effectVideoLow, setEffectVideoLow] = createSignal(0);
  const [effectVideoHigh, setEffectVideoHigh] = createSignal(1);
  const [effectVideoPositionX, setEffectVideoPositionX] = createSignal(0);
  const [effectVideoPositionY, setEffectVideoPositionY] = createSignal(0);
  const [effectVideoPositionZ, setEffectVideoPositionZ] = createSignal(0);
  const [effectVideoTargetLinked, setEffectVideoTargetLinked] = createSignal(false);
  const [sampleEffectPreset, setSampleEffectPreset] = createSignal<SampleEffectPreset>("pulse");
  const [editingEffectId, setEditingEffectId] = createSignal<number | null>(null);
  const [effectPeriod, setEffectPeriod] = createSignal(1000);
  const [effectClockSyncBeats, setEffectClockSyncBeats] = createSignal<number | null>(null);
  const [effectLow, setEffectLow] = createSignal(0);
  const [effectHigh, setEffectHigh] = createSignal(65535);
  const [effectPhase, setEffectPhase] = createSignal(0);
  const [effectBlendMode, setEffectBlendMode] = createSignal<EffectBlendMode>("Override");
  const [effectAttribute, setEffectAttribute] = createSignal("");
  const [nodeGraphLabel, setNodeGraphLabel] = createSignal("Graph 1");
  const [nodeGraphSourceMode, setNodeGraphSourceMode] = createSignal<"Effect" | "Audio">("Effect");
  const [nodeGraphAudioBand, setNodeGraphAudioBand] = createSignal<AudioSpectrumBand>("Bass");
  const [nodeGraphAudioSource, setNodeGraphAudioSource] = createSignal<AudioSpectrumSource>("Timeline");
  const [nodeGraphAudioGain, setNodeGraphAudioGain] = createSignal(1);
  const [nodeGraphAudioBias, setNodeGraphAudioBias] = createSignal(0);
  const [nodeGraphTransformOp, setNodeGraphTransformOp] = createSignal<NodeGraphTransformOp>("Scale");
  const [nodeGraphTransformAmount, setNodeGraphTransformAmount] = createSignal(1);
  const [nodeGraphTransformMin, setNodeGraphTransformMin] = createSignal(0);
  const [nodeGraphTransformMax, setNodeGraphTransformMax] = createSignal(1);
  const [waveOriginX, setWaveOriginX] = createSignal(0);
  const [waveOriginY, setWaveOriginY] = createSignal(0);
  const [waveOriginZ, setWaveOriginZ] = createSignal(0);
  const [waveDirectionX, setWaveDirectionX] = createSignal(1);
  const [waveDirectionY, setWaveDirectionY] = createSignal(0);
  const [waveDirectionZ, setWaveDirectionZ] = createSignal(0);
  const [waveStageDrag, setWaveStageDrag] = createSignal<WaveStageDragMode | null>(null);
  const [waveSpeed, setWaveSpeed] = createSignal(1);
  const [waveWavelength, setWaveWavelength] = createSignal(2);
  const [snapshot, setSnapshot] = createSignal<EngineSnapshot>(createInitialEngineSnapshot());
  const [snapshotRevision, setSnapshotRevision] = createSignal<number | null>(null);
  const viewportFixture = browserViewportFixture(isTauriRuntime());
  if (viewportFixture === "timeline") {
    setWorkspaceTab("control");
    setSelectedFixtureGroupFilter("front");
    setProfile(viewportFixtureData.profile);
    setGdtfPath(viewportFixtureData.profile.source_path);
    setSelectedMode(viewportFixtureData.profile.dmx_modes[0]?.name ?? "");
    setLabel("Viewport Par");
    setUniverse(0);
    setAddress(25);
    setPatchCount(2);
    setPatchAddressStride(0);
    setGroupText("front");
    setSelectedFixtureId(1);
    setSelectedMappingFixtureIds([1]);
    setSelectedFixtureLabelDraft("Viewport Par L");
    setSelectedFixtureUniverseDraft(0);
    setSelectedFixtureAddressDraft(1);
    setSelectedFixtureGroupText("front");
    setSelectedFixtureLimitsDraft(defaultFixtureLimits);
    setSnapshot((current) => ({
      ...current,
      fixtures: [
        viewportPatchedFixture(1, "Video", 1, -4, -2),
        viewportPatchedFixture(2, "Save", 9, 0, -2),
        viewportPatchedFixture(3, "Output", 17, 4, -2),
      ],
      submasters: [{ group_id: "front", label: "front", level: 1 }],
      video: {
        ...current.video,
        layers: [viewportFixtureData.videoLayer],
        compositions: [viewportFixtureData.composition],
        outputs: [viewportFixtureData.videoOutput],
        mapping_presets: [{ label: "Viewport 16:9", mapping: viewportFixtureData.projectorMapping }],
      },
      stage_objects: [viewportFixtureData.stageObject],
      timeline: {
        ...current.timeline,
        duration_ms: 4000,
        position_ms: 1000,
        automations: [
          {
            id: 1,
            fixture_id: 1,
            attribute: "Dimmer",
            track: "Lighting",
            enabled: true,
            keyframes: [
              { time_ms: 0, value: 65535, interpolation: "Linear" },
              { time_ms: 4000, value: 0, interpolation: "Step" },
            ],
          },
        ],
        video_automations: [
          {
            id: 2,
            layer_id: 1,
            param: "Opacity",
            track: "Video",
            enabled: true,
            keyframes: [
              { time_ms: 0, value: 1, interpolation: "Linear" },
              { time_ms: 4000, value: 0, interpolation: "Step" },
            ],
          },
        ],
      },
    }));
  } else if (viewportFixture === "large-show") {
    const fixtures = Array.from({ length: 2_000 }, (_, index) => {
      const id = index + 1;
      const column = index % 40;
      const row = Math.floor(index / 40);
      return viewportPatchedFixture(
        id,
        `Large Fixture ${id.toString().padStart(4, "0")}`,
        1 + ((index * 8) % 504),
        column * 0.25 - 5,
        row * 0.25 - 5,
      );
    });
    setWorkspaceTab("setup");
    setSetupSubTab("mapping");
    setSelectedFixtureGroupFilter("front");
    setProfile(viewportFixtureData.profile);
    setSelectedFixtureId(1);
    setSnapshot((current) => ({
      ...current,
      fixtures,
      submasters: [{ group_id: "front", label: "front", level: 1 }],
    }));
  }
  let lastRecoverySignature = projectRecoveryCheckpoint()?.signature ?? null;
  let lastDesktopBackupSignature: string | null = null;
  let lastDesktopBackupAt = 0;
  const handleProjectHistoryChanged = (event: Event) => {
    setProjectHistoryStatus((event as CustomEvent<ProjectHistoryStatus>).detail);
  };
  window.addEventListener(projectHistoryChangedEvent, handleProjectHistoryChanged);
  onCleanup(() => window.removeEventListener(projectHistoryChangedEvent, handleProjectHistoryChanged));
  createEffect(() => {
    const scale = uiScale();
    document.documentElement.style.setProperty("--ui-scale", String(scale / 100));
    document.documentElement.style.setProperty("--ui-scale-inverse", `${10000 / scale}%`);
    try {
      localStorage.setItem(uiScaleStorageKey, String(scale));
    } catch {
      // The preference remains active for this session when storage is unavailable.
    }
  });
  const uiLocalization = installUiLocalization(document.body, uiLocale);
  onCleanup(uiLocalization.dispose);
  createEffect(() => {
    const locale = uiLocale();
    saveUiLocale(locale);
    uiLocalization.refresh();
  });
  createEffect(() => {
    saveWorkspaceLayout({
      workspace_tab: workspaceTab(),
      setup_sub_tab: setupSubTab(),
      control_mode: controlMode(),
      timeline_desk_surface: timelineDeskSurface(),
      edit_desk_surface: editDeskSurface(),
      control_category: controlCategory(),
    });
  });
  const [appStatus, setAppStatus] = createSignal(appStatusFromMessage("Ready"));
  const message = () => appStatus().text;
  const setMessage = (text: string) => {
    setAppStatus(appStatusFromMessage(text));
    return text;
  };
  const resetWorkspaceLayout = () => {
    setWorkspaceTab(defaultWorkspaceLayout.workspace_tab);
    setSetupSubTab(defaultWorkspaceLayout.setup_sub_tab);
    setControlMode(defaultWorkspaceLayout.control_mode);
    setTimelineDeskSurface(defaultWorkspaceLayout.timeline_desk_surface);
    setEditDeskSurface(defaultWorkspaceLayout.edit_desk_surface);
    setControlCategory(defaultWorkspaceLayout.control_category);
    setMessage("Workspace layout reset to the default desk.");
  };
  const {
    output,
    setOutput,
    dmxOutputRoutes,
    setDmxOutputRoutes,
    engineTelemetryReport,
    dmxTestChannel,
    setDmxTestChannel,
    dmxTestWidth,
    setDmxTestWidth,
    dmxTestValue,
    setDmxTestValue,
    refreshEngineTelemetryReport,
    resetEngineTelemetry,
    saveEngineTelemetryReport,
    applyOutput,
    sendDmxTestFrame,
    sendDmxRoutesTestFrame,
    dmxRouteLabel,
    applyCurrentDmxRoutes,
    addCurrentDmxRoute,
    removeDmxRoute,
    setOutputProtocol,
    refreshSerialPorts,
  } = createOutputDiagnosticsController({
    invoke,
    setMessage,
    refreshSnapshot: () => refreshSnapshot(),
    serialPorts,
    setSerialPorts,
  });
  const refreshDmxInputStatus = async () => {
    if (!isTauriRuntime()) return;
    try {
      setDmxInputStatus(await invoke<DmxInputStatus>("dmx_input_status"));
    } catch (error) {
      setMessage(`DMX input status failed: ${String(error)}`);
    }
  };
  const startDmxInput = async () => {
    try {
      await invoke("start_dmx_input", { config: dmxInputConfig() });
      await refreshDmxInputStatus();
      setMessage("DMX input started.");
    } catch (error) {
      setMessage(`DMX input start failed: ${String(error)}`);
    }
  };
  const stopDmxInput = async () => {
    try {
      await invoke("stop_dmx_input");
      await refreshDmxInputStatus();
      setMessage("DMX input stopped and the merged input frame was cleared.");
    } catch (error) {
      setMessage(`DMX input stop failed: ${String(error)}`);
    }
  };
  const sendArtRdmRequest = (request: ArtRdmRequest) =>
    invoke<ArtRdmResponse>("send_art_rdm_request", { request });
  const sendUsbRdmRequest = (request: UsbRdmRequest) =>
    invoke<ArtRdmResponse>("send_usb_rdm_request", { request });
  const discoverUsbRdmDevices = (serialPort: string, sourceUid: string) =>
    invoke<string[]>("discover_usb_rdm_devices", { serialPort, sourceUid, timeoutMs: 60_000 });
  const discoverArtRdmDevices = (gatewayIp: string, portAddress: number) =>
    invoke<string[]>("discover_art_rdm_devices", { gatewayIp, portAddress, timeoutMs: 1_000 });
  const startArtRdmFullDiscovery = (gatewayIp: string, portAddress: number) =>
    invoke<void>("start_art_rdm_full_discovery", { gatewayIp, portAddress });
  const audioAnalysis = createMemo<AudioAnalysisSummary | null>(() => snapshot().timeline.audio ?? null);
  const [touchDimmerRestore, setTouchDimmerRestore] = createSignal<TouchDimmerRestoreState | null>(null);

  createEffect(() => {
    savePositionFavorites(positionFavorites());
  });

  createEffect(() => {
    saveColorFavorites(colorFavorites());
  });

  const selectedFixture = createMemo<PatchedFixtureSummary | undefined>(() =>
    snapshot().fixtures.find((fixture) => fixture.id === selectedFixtureId()),
  );
  const faderValue = (fixtureId: number, attribute: string, defaultValue: number) => {
    const localValue = faderValues()[`${fixtureId}:${attribute}`];
    if (localValue !== undefined) {
      return localValue;
    }
    const stagedValue = snapshot().programmer.enabled
      ? snapshot().programmer.values.find(
        (value) => value.fixture_id === fixtureId && value.attribute === attribute,
      )?.value
      : undefined;
    if (stagedValue !== undefined) {
      return stagedValue;
    }
    const fixture = snapshot().fixtures.find((candidate) => candidate.id === fixtureId);
    return fixture?.attribute_values.find((value) => value.attribute === attribute)?.value ?? defaultValue;
  };
  const normalizedSelectedFixtureLimitsDraft = createMemo<FixtureLimits>(() => {
    const limits = selectedFixtureLimitsDraft();
    const dimmer = normalizeLimitRange(limits.dimmer_min, limits.dimmer_max);
    const pan = normalizeLimitRange(limits.pan_min, limits.pan_max);
    const tilt = normalizeLimitRange(limits.tilt_min, limits.tilt_max);
    return {
      ...limits,
      dimmer_min: dimmer.min,
      dimmer_max: dimmer.max,
      pan_min: pan.min,
      pan_max: pan.max,
      tilt_min: tilt.min,
      tilt_max: tilt.max,
    };
  });
  const selectedFixtureLimitWindowStyle = createMemo(() => {
    const limits = normalizedSelectedFixtureLimitsDraft();
    const panWidth = ((limits.pan_max - limits.pan_min) / 65_535) * 100;
    const tiltHeight = ((limits.tilt_max - limits.tilt_min) / 65_535) * 100;
    return {
      left: `${(limits.pan_min / 65_535) * 100}%`,
      width: `${Math.max(1.5, panWidth)}%`,
      top: `${((65_535 - limits.tilt_max) / 65_535) * 100}%`,
      height: `${Math.max(1.5, tiltHeight)}%`,
    };
  });
  const selectedFixtureLimits = createMemo(() => selectedFixture()?.limits ?? defaultFixtureLimits);
  const selectedFixtureLimitOverlayStyle = createMemo(() => {
    const limits = selectedFixtureLimits();
    const panRange = normalizeLimitRange(limits.pan_min, limits.pan_max);
    const tiltRange = normalizeLimitRange(limits.tilt_min, limits.tilt_max);
    const panWidth = ((panRange.max - panRange.min) / 65_535) * 100;
    const tiltHeight = ((tiltRange.max - tiltRange.min) / 65_535) * 100;
    return {
      left: `${(panRange.min / 65_535) * 100}%`,
      width: `${Math.max(1.5, panWidth)}%`,
      top: `${((65_535 - tiltRange.max) / 65_535) * 100}%`,
      height: `${Math.max(1.5, tiltHeight)}%`,
    };
  });
  const fixtureGroupRows = createMemo(() => {
    const counts = new Map<string, number>();
    for (const fixture of snapshot().fixtures) {
      for (const groupId of fixture.group_ids) {
        counts.set(groupId, (counts.get(groupId) ?? 0) + 1);
      }
    }
    return [...counts.entries()]
      .sort(([left], [right]) => left.localeCompare(right))
      .map(([groupId, count]) => ({ groupId, count }));
  });
  const filteredFixtures = createMemo(() => {
    const groupId = selectedFixtureGroupFilter();
    if (!groupId) {
      return snapshot().fixtures;
    }
    return snapshot().fixtures.filter((fixture) => fixture.group_ids.includes(groupId));
  });
  const fixtureTypeRows = createMemo<MappingFixtureTypeRow[]>(() => {
    const rows = new Map<string, MappingFixtureTypeRow>();
    for (const fixture of filteredFixtures()) {
      const key = fixtureTypeKey(fixture);
      const current = rows.get(key);
      if (current) {
        current.count += 1;
      } else {
        rows.set(key, {
          key,
          label: fixtureTypeLabel(fixture),
          manufacturer: fixture.manufacturer,
          mode: fixture.mode_name,
          visualKind: fixtureVisualKind(fixture),
          count: 1,
        });
      }
    }
    return [...rows.values()].sort((left, right) => left.label.localeCompare(right.label));
  });
  const fixtureMatchesMappingSearch = (fixture: PatchedFixtureSummary, search: string) => {
    if (!search) {
      return true;
    }
    const statusText = [
      fixture.highlighted ? "highlight" : "",
      fixture.soloed ? "solo" : "",
      fixture.parked ? "park" : "",
    ].filter(Boolean).join(" ");
    const haystack = [
      fixture.label,
      fixture.manufacturer,
      fixture.profile_name,
      fixture.mode_name,
      `u${fixture.universe}`,
      `a${fixture.address}`,
      `u${fixture.universe} a${fixture.address}`,
      fixture.group_ids.join(" "),
      statusText,
    ].join(" ").toLowerCase();
    return haystack.includes(search);
  };
  const mappingFilteredFixtures = createMemo(() => {
    const typeKey = selectedFixtureTypeFilter();
    const search = mappingFixtureSearch().trim().toLowerCase();
    return filteredFixtures().filter((fixture) => {
      if (typeKey && fixtureTypeKey(fixture) !== typeKey) {
        return false;
      }
      return fixtureMatchesMappingSearch(fixture, search);
    });
  });
  const selectedMappingFixtureIdSet = createMemo(() => new Set(selectedMappingFixtureIds()));
  const selectedMappingFixtures = createMemo(() => {
    const selectedIds = selectedMappingFixtureIdSet();
    return snapshot().fixtures.filter((fixture) => selectedIds.has(fixture.id));
  });
  const selectedMappingFlagState = createMemo(() => {
    const fixtures = selectedMappingFixtures();
    const count = fixtures.length;
    return {
      count,
      anyHighlighted: fixtures.some((fixture) => fixture.highlighted),
      allHighlighted: count > 0 && fixtures.every((fixture) => fixture.highlighted),
      anySoloed: fixtures.some((fixture) => fixture.soloed),
      allSoloed: count > 0 && fixtures.every((fixture) => fixture.soloed),
      anyParked: fixtures.some((fixture) => fixture.parked),
      allParked: count > 0 && fixtures.every((fixture) => fixture.parked),
    };
  });
  const selectedMappingFixture = createMemo(() => {
    const selectedId = selectedFixtureId();
    if (selectedId === null || !selectedMappingFixtureIdSet().has(selectedId)) {
      return null;
    }
    return snapshot().fixtures.find((fixture) => fixture.id === selectedId) ?? null;
  });
  const selectedMappingGeometryRows = createMemo(() => {
    const fixture = selectedMappingFixture();
    return fixture ? profileGeometryRows(fixture.geometries, fixture.controls) : [];
  });
  const selectedMappingUnresolvedGeometryReferences = createMemo(() => {
    const fixture = selectedMappingFixture();
    return fixture ? unresolvedGeometryReferences(fixture.geometries, fixture.controls) : [];
  });
  const selectedGroupFixtures = createMemo(() => {
    if (!selectedFixtureGroupFilter()) {
      return [];
    }
    return filteredFixtures();
  });
  const selectedGroupFlagState = createMemo(() => {
    const fixtures = selectedGroupFixtures();
    return {
      count: fixtures.length,
      anyHighlighted: fixtures.some((fixture) => fixture.highlighted),
      anySoloed: fixtures.some((fixture) => fixture.soloed),
      anyParked: fixtures.some((fixture) => fixture.parked),
    };
  });
  const globalFixtureFlagState = createMemo(() => {
    const fixtures = snapshot().fixtures;
    return {
      count: fixtures.length,
      anyHighlighted: fixtures.some((fixture) => fixture.highlighted),
      anySoloed: fixtures.some((fixture) => fixture.soloed),
      anyParked: fixtures.some((fixture) => fixture.parked),
      anyFlagged: fixtures.some((fixture) => fixture.highlighted || fixture.soloed || fixture.parked),
    };
  });
  const selectedGroupSubmaster = createMemo(() => {
    const groupId = selectedFixtureGroupFilter();
    if (!groupId) {
      return undefined;
    }
    return snapshot().submasters.find((submaster) => submaster.group_id === groupId);
  });

  const selectedModeSummary = createMemo(() => {
    const imported = profile();
    if (!imported) {
      return undefined;
    }
    return imported.dmx_modes.find((mode) => mode.name === selectedMode()) ?? imported.dmx_modes[0];
  });

  const selectedFootprint = createMemo(() => {
    const mode = selectedModeSummary();
    if (!mode) {
      return 0;
    }
    return Math.max(0, ...mode.controls.flatMap((control) => control.offsets));
  });
  const selectedModeDmxCells = createMemo(() =>
    (selectedModeSummary()?.controls ?? [])
      .flatMap((control) =>
        control.offsets.map((offset, index) => ({
          channel: offset,
          control,
          category: controlCategoryForAttribute(control.attribute),
          bitLabel: control.resolution === "SixteenBit" ? (index === 0 ? "MSB" : "LSB") : "8-bit",
        })),
      )
      .sort((first, second) => first.channel - second.channel),
  );
  const selectedModeFunctionEntries = createMemo(() =>
    (selectedModeSummary()?.controls ?? [])
      .flatMap((control) =>
        sortedChannelFunctions(control).map((fn) => ({
          control,
          fn,
          category: controlCategoryForAttribute(control.attribute),
        })),
      )
      .sort(
        (first, second) =>
          (first.control.offsets[0] ?? 0) - (second.control.offsets[0] ?? 0) ||
          first.fn.dmx_from - second.fn.dmx_from,
      ),
  );
  const visibleSelectedModeFunctionEntries = createMemo(() => selectedModeFunctionEntries().slice(0, 12));
  const selectedModeGeometryRows = createMemo(() =>
    profileGeometryRows(profile()?.geometries ?? [], selectedModeSummary()?.controls ?? []),
  );
  const selectedModeUnresolvedGeometryReferences = createMemo(() =>
    unresolvedGeometryReferences(profile()?.geometries ?? [], selectedModeSummary()?.controls ?? []),
  );

  const patchCountValue = createMemo(() => Math.min(256, Math.max(1, Math.floor(patchCount() || 1))));
  const patchAddressStrideValue = createMemo(() => {
    const manualStride = Math.floor(patchAddressStride() || 0);
    return manualStride > 0 ? manualStride : Math.max(selectedFootprint(), 1);
  });
  const patchAddressRanges = createMemo(() =>
    Array.from({ length: patchCountValue() }, (_, index) => {
      const start = address() + index * patchAddressStrideValue();
      return {
        index,
        start,
        range: addressRange(start, selectedFootprint()),
      };
    }),
  );
  const endAddress = createMemo(() => patchAddressRanges().at(-1)?.range?.[1] ?? address());
  const patchAddressConflictText = createMemo(() => {
    const ranges = patchAddressRanges();
    for (const candidate of ranges) {
      if (!candidate.range) {
        continue;
      }
      const conflict = snapshot().fixtures.find((fixture) => {
        if (fixture.universe !== universe()) {
          return false;
        }
        const existingRange = addressRange(fixture.address, fixtureFootprint(fixture));
        return existingRange ? rangesOverlap(candidate.range!, existingRange) : false;
      });
      if (conflict) {
        const conflictRange = addressRange(conflict.address, fixtureFootprint(conflict));
        const rangeText = conflictRange ? `A${conflictRange[0]}-${conflictRange[1]}` : `A${conflict.address}`;
        return `Fixture ${candidate.index + 1} overlaps ${conflict.label} (${rangeText})`;
      }
      for (const existing of ranges.slice(0, candidate.index)) {
        if (existing.range && rangesOverlap(candidate.range, existing.range)) {
          return `Fixture ${candidate.index + 1} overlaps new fixture ${existing.index + 1} (A${existing.range[0]}-${existing.range[1]})`;
        }
      }
    }
    return "";
  });
  const patchAddressInvalid = createMemo(() => selectedFootprint() === 0 || endAddress() > 512 || Boolean(patchAddressConflictText()));
  const nextFreePatchAddress = createMemo(() => {
    const targetUniverse = universe();
    return findNextFreePatchAddress(
      snapshot().fixtures,
      targetUniverse,
      selectedFootprint(),
      patchCountValue(),
      patchAddressStrideValue(),
    );
  });
  const patchGridColumnsValue = createMemo(() => Math.min(64, Math.max(1, Math.floor(patchGridColumns() || 1))));
  const patchCircleRadiusValue = createMemo(() => Math.max(0.1, Number.isFinite(patchCircleRadius()) ? patchCircleRadius() : 4));
  const patchFixturePosition = (index: number, count: number) => {
    if (patchLayoutMode() === "grid") {
      const column = index % patchGridColumnsValue();
      const row = Math.floor(index / patchGridColumnsValue());
      return {
        x: patchX() + column * patchXStep(),
        y: patchY(),
        z: patchZ() + row * patchZStep(),
      };
    }
    if (patchLayoutMode() === "circle" && count > 1) {
      const angle = (index / count) * Math.PI * 2 - Math.PI / 2;
      const radius = patchCircleRadiusValue();
      return {
        x: patchX() + Math.cos(angle) * radius,
        y: patchY(),
        z: patchZ() + Math.sin(angle) * radius,
      };
    }
    return {
      x: patchX() + index * patchXStep(),
      y: patchY(),
      z: patchZ() + index * patchZStep(),
    };
  };
  const dmxUniverseMaps = createMemo<DmxUniverseMap[]>(() => {
    const universes = new Map<number, DmxPatchSegment[]>();
    for (const fixture of snapshot().fixtures) {
      const footprint = fixtureFootprint(fixture);
      const range = addressRange(fixture.address, footprint);
      if (!range) {
        continue;
      }
      const segments = universes.get(fixture.universe) ?? [];
      segments.push({
        fixture,
        start: range[0],
        end: Math.min(512, range[1]),
        left: ((range[0] - 1) / 512) * 100,
        width: ((Math.min(512, range[1]) - range[0] + 1) / 512) * 100,
      });
      universes.set(fixture.universe, segments);
    }
    if (!universes.has(universe())) {
      universes.set(universe(), []);
    }

    return Array.from(universes.entries())
      .sort(([firstUniverse], [secondUniverse]) => firstUniverse - secondUniverse)
      .map(([universeId, segments]) => {
        const sortedSegments = [...segments].sort((first, second) => first.start - second.start);
        let cursor = 1;
        let largestFree = 0;
        for (const segment of sortedSegments) {
          if (segment.start > cursor) {
            largestFree = Math.max(largestFree, segment.start - cursor);
          }
          cursor = Math.max(cursor, segment.end + 1);
        }
        largestFree = Math.max(largestFree, 513 - cursor);
        const used = sortedSegments.reduce((total, segment) => total + Math.max(0, segment.end - segment.start + 1), 0);
        return {
          universe: universeId,
          used,
          free: Math.max(0, 512 - used),
          largestFree,
          segments: sortedSegments,
        };
      });
  });

  const patchGridUniverseOptions = createMemo(() => {
    const universeIds = new Set<number>([universe(), output().universe, patchGridUniverse()]);
    for (const route of dmxOutputRoutes()) {
      universeIds.add(route.universe);
    }
    for (const fixture of snapshot().fixtures) {
      universeIds.add(fixture.universe);
    }
    return [...universeIds].sort((first, second) => first - second);
  });

  const activePatchGridUniverse = createMemo(() => {
    const options = patchGridUniverseOptions();
    return options.includes(patchGridUniverse()) ? patchGridUniverse() : (options[0] ?? 0);
  });

  createEffect(() => {
    const activeUniverse = activePatchGridUniverse();
    if (activeUniverse !== patchGridUniverse()) {
      setPatchGridUniverse(activeUniverse);
    }
  });

  const activePatchGridMap = createMemo<DmxUniverseMap>(() => {
    const activeUniverse = activePatchGridUniverse();
    return (
      dmxUniverseMaps().find((map) => map.universe === activeUniverse) ?? {
        universe: activeUniverse,
        used: 0,
        free: 512,
        largestFree: 512,
        segments: [],
      }
    );
  });

  const patchPlannedRanges = createMemo(() =>
    universe() === activePatchGridUniverse()
      ? patchAddressRanges().filter((candidate): candidate is { index: number; start: number; range: [number, number] } =>
          Boolean(candidate.range),
        )
      : [],
  );
  const plannedAddressSummary = createMemo(() => {
    const ranges = patchPlannedRanges();
    if (ranges.length === 0) {
      return "No pending patch in this universe";
    }
    const first = ranges[0].range;
    const last = ranges[ranges.length - 1].range;
    const stepText = patchAddressStrideValue() === selectedFootprint() ? "" : `, step ${patchAddressStrideValue()}ch`;
    return ranges.length === 1
      ? `Pending A${first[0]}-${first[1]}`
      : `Pending A${first[0]}-${last[1]} (${ranges.length} fixtures${stepText})`;
  });

  const dmxAddressCells = createMemo<DmxAddressCell[]>(() => {
    const map = activePatchGridMap();
    const selectedId = selectedFixtureId();
    const plannedRanges = patchPlannedRanges();
    return Array.from({ length: 512 }, (_, index) => {
      const channel = index + 1;
      const segment = map.segments.find((candidate) => channel >= candidate.start && channel <= candidate.end) ?? null;
      const planned = plannedRanges.find((candidate) => channel >= candidate.range[0] && channel <= candidate.range[1]);
      return {
        channel,
        segment,
        isStart: segment?.start === channel,
        isSelected: Boolean(segment && segment.fixture.id === selectedId),
        plannedIndex: planned?.index ?? null,
        plannedStart: planned?.range[0] === channel,
        plannedConflict: Boolean(planned && segment),
      };
    });
  });

  const commonAttributeControls = (fixtures: PatchedFixtureSummary[]) => {
    if (fixtures.length === 0) {
      return [];
    }
    const attributeCounts = new Map<string, number>();
    for (const fixture of fixtures) {
      const fixtureAttributes = new Set(fixture.controls.map((control) => control.attribute.toLowerCase()));
      for (const attribute of fixtureAttributes) {
        attributeCounts.set(attribute, (attributeCounts.get(attribute) ?? 0) + 1);
      }
    }
    const returnedAttributes = new Set<string>();
    return fixtures[0].controls.filter((control) => {
      const key = control.attribute.toLowerCase();
      if (returnedAttributes.has(key) || attributeCounts.get(key) !== fixtures.length) {
        return false;
      }
      returnedAttributes.add(key);
      return true;
    });
  };
  const selectedControlTargetFixtures = createMemo<PatchedFixtureSummary[]>(() => {
    const groupId = selectedFixtureGroupFilter();
    if (groupId) {
      return filteredFixtures();
    }
    const fixture = selectedFixture();
    return fixture ? [fixture] : [];
  });
  const selectedControlReferenceFixture = createMemo<PatchedFixtureSummary | null>(() => {
    const fixtures = selectedControlTargetFixtures();
    if (fixtures.length === 0) {
      return null;
    }
    const selected = selectedFixture();
    return selected && fixtures.some((fixture) => fixture.id === selected.id) ? selected : fixtures[0];
  });
  const activeControls = createMemo(() => {
    const groupId = selectedFixtureGroupFilter();
    return groupId ? commonAttributeControls(selectedControlTargetFixtures()) : (selectedFixture()?.controls ?? []);
  });
  const timelineAutomationControls = createMemo<AttributeControl[]>(() => {
    const groupId = selectedFixtureGroupFilter();
    const fixtures = groupId
      ? filteredFixtures()
      : selectedFixture()
        ? [selectedFixture() as PatchedFixtureSummary]
        : [];
    const controlsByAttribute = new Map<string, AttributeControl>();
    for (const fixture of fixtures) {
      for (const control of fixture.controls) {
        if (!controlsByAttribute.has(control.attribute)) {
          controlsByAttribute.set(control.attribute, control);
        }
      }
    }
    return [...controlsByAttribute.values()];
  });
  const selectedTimelineAutomationAttribute = createMemo(() => {
    const controls = timelineAutomationControls();
    const current = effectAttribute();
    if (controls.some((control) => control.attribute === current)) {
      return current;
    }
    return controls[0]?.attribute ?? "";
  });
  const selectedFixtureSupportsTimelineAutomationAttribute = createMemo(() => {
    const fixture = selectedFixture();
    const attribute = selectedTimelineAutomationAttribute();
    return Boolean(fixture && attribute && fixture.controls.some((control) => control.attribute === attribute));
  });
  const canAddTimelineAutomation = createMemo(() => selectedFixtureSupportsTimelineAutomationAttribute());
  const canAddTimelineGroupAutomation = createMemo(() =>
    Boolean(selectedFixtureGroupFilter() && selectedTimelineAutomationAttribute() && timelineAutomationControls().length > 0),
  );
  const selectedDimmerControl = createMemo<DimmerControlSet | undefined>(() => {
    const fixture = selectedControlReferenceFixture();
    if (!fixture) {
      return undefined;
    }
    const attribute = findControlAttributeInControls(activeControls(), ["Dimmer", "Intensity", "MasterIntensity"]);
    if (!attribute) {
      return undefined;
    }
    return {
      attribute,
      value: dimmerValueWithinLimits(fixture, readFixtureAttribute(fixture, faderValues(), [attribute]) ?? 0),
    };
  });
  const touchDimmerEntryForFixture = (
    fixture: PatchedFixtureSummary,
    currentValues: Record<string, number>,
  ): TouchDimmerQuickEntry | undefined => {
    const attribute = findControlAttribute(fixture, ["Dimmer", "Intensity", "MasterIntensity"]);
    if (!attribute) {
      return undefined;
    }
    const outValue = dimmerValueWithinLimits(fixture, 0);
    const fullValue = dimmerValueWithinLimits(fixture, 65_535);
    return {
      fixtureId: fixture.id,
      label: fixture.label,
      attribute,
      value: dimmerValueWithinLimits(fixture, readFixtureAttribute(fixture, currentValues, [attribute]) ?? 0),
      outValue,
      halfValue: clampDmxValue((outValue + fullValue) / 2),
      fullValue,
    };
  };
  const selectedTouchDimmerTarget = createMemo<TouchDimmerQuickTarget | undefined>(() => {
    const groupId = selectedFixtureGroupFilter();
    const currentValues = faderValues();
    if (groupId) {
      const entries = filteredFixtures()
        .map((fixture) => touchDimmerEntryForFixture(fixture, currentValues))
        .filter((entry): entry is TouchDimmerQuickEntry => Boolean(entry));
      return entries.length > 0
        ? {
            kind: "group",
            label: `Group ${groupId}`,
            entries,
          }
        : undefined;
    }
    const fixture = selectedFixture();
    if (!fixture) {
      return undefined;
    }
    const entry = touchDimmerEntryForFixture(fixture, currentValues);
    return entry
      ? {
          kind: "fixture",
          label: fixture.label,
          entries: [entry],
        }
      : undefined;
  });
  const touchDimmerQuickActive = createMemo(() => Boolean(touchDimmerRestore()));
  const selectedPositionControls = createMemo<PositionControlSet | undefined>(() => {
    const fixture = selectedControlReferenceFixture();
    if (!fixture) {
      return undefined;
    }
    const controls = activeControls();
    const pan = findControlAttributeInControls(controls, ["Pan"]);
    const tilt = findControlAttributeInControls(controls, ["Tilt"]);
    if (!pan || !tilt) {
      return undefined;
    }
    const currentValues = faderValues();
    const rawPanValue = readFixtureAttribute(fixture, currentValues, [pan]) ?? 32768;
    const rawTiltValue = readFixtureAttribute(fixture, currentValues, [tilt]) ?? 32768;
    const effective = effectivePanTiltValues(fixture, rawPanValue, rawTiltValue);
    return {
      pan,
      tilt,
      panValue: effective.pan,
      tiltValue: effective.tilt,
    };
  });
  const selectedColorControls = createMemo<ColorControlSet | undefined>(() => {
    const fixture = selectedControlReferenceFixture();
    if (!fixture) {
      return undefined;
    }
    const controls = activeControls();
    const red = findControlAttributeInControls(controls, colorCandidates.red);
    const green = findControlAttributeInControls(controls, colorCandidates.green);
    const blue = findControlAttributeInControls(controls, colorCandidates.blue);
    if (!red || !green || !blue) {
      return undefined;
    }
    const currentValues = faderValues();
    const redValue = readFixtureAttribute(fixture, currentValues, [red]) ?? 0;
    const greenValue = readFixtureAttribute(fixture, currentValues, [green]) ?? 0;
    const blueValue = readFixtureAttribute(fixture, currentValues, [blue]) ?? 0;
    const extras = [
      {
        key: "white" as const,
        label: "White",
        shortLabel: "W",
        attribute: findControlAttributeInControls(controls, colorCandidates.white),
      },
      {
        key: "amber" as const,
        label: "Amber",
        shortLabel: "A",
        attribute: findControlAttributeInControls(controls, colorCandidates.amber),
      },
      {
        key: "uv" as const,
        label: "UV",
        shortLabel: "UV",
        attribute: findControlAttributeInControls(controls, colorCandidates.uv),
      },
    ]
      .filter(
        (extra): extra is {
          key: ColorExtraChannelKey;
          label: string;
          shortLabel: string;
          attribute: string;
        } => Boolean(extra.attribute),
      )
      .map((extra) => ({
        ...extra,
        value: readFixtureAttribute(fixture, currentValues, [extra.attribute]) ?? 0,
      }));
    return {
      red,
      green,
      blue,
      redValue,
      greenValue,
      blueValue,
      extras,
      value: `#${valueToHexByte(redValue)}${valueToHexByte(greenValue)}${valueToHexByte(blueValue)}`,
    };
  });
  const selectedColorHsv = createMemo(() => {
    const color = selectedColorControls()?.value ?? "#ffffff";
    return rgbToHsv(
      Number.parseInt(color.slice(1, 3), 16),
      Number.parseInt(color.slice(3, 5), 16),
      Number.parseInt(color.slice(5, 7), 16),
    );
  });
  const selectedColorHex = createMemo(() => normalizeHexColor(selectedColorControls()?.value) ?? "#000000");
  const selectedColorChannelValues = createMemo(() => {
    const controls = selectedColorControls();
    if (controls) {
      return {
        red: controls.redValue,
        green: controls.greenValue,
        blue: controls.blueValue,
      };
    }
    const color = selectedColorHex();
    return {
      red: Number.parseInt(color.slice(1, 3), 16) * 257,
      green: Number.parseInt(color.slice(3, 5), 16) * 257,
      blue: Number.parseInt(color.slice(5, 7), 16) * 257,
    };
  });
  const controlCategoryCounts = createMemo(() => {
    const counts = new Map<ControlCategory, number>();
    for (const category of controlCategories) {
      counts.set(category.id, 0);
    }
    const controls = activeControls();
    counts.set("fader", controls.length);
    for (const control of controls) {
      const category = controlCategoryForAttribute(control.attribute);
      counts.set(category, (counts.get(category) ?? 0) + 1);
    }
    return counts;
  });
  const categoryHasVisualControl = (category: ControlCategory) =>
    (category === "dimmer" && Boolean(selectedDimmerControl())) ||
    (category === "position" && Boolean(selectedPositionControls())) ||
    (category === "color" && Boolean(selectedColorControls())) ||
    category === "fader";
  const activeControlCategory = createMemo<ControlCategory>(() => {
    const requested = controlCategory();
    const counts = controlCategoryCounts();
    if ((counts.get(requested) ?? 0) > 0 || categoryHasVisualControl(requested)) {
      return requested;
    }
    return (
      controlCategories.find((category) => (counts.get(category.id) ?? 0) > 0 || categoryHasVisualControl(category.id))
        ?.id ?? "fader"
    );
  });
  const controlCategoryRows = createMemo(() => {
    const counts = controlCategoryCounts();
    return controlCategories.map((category) => ({
      ...category,
      count: counts.get(category.id) ?? 0,
      hasVisual: categoryHasVisualControl(category.id),
    }));
  });
  const visibleControls = createMemo(() => {
    const category = activeControlCategory();
    const controls = activeControls();
    if (category === "fader") {
      return controls;
    }
    return controls.filter((control) => controlCategoryForAttribute(control.attribute) === category);
  });
  const showDimmerPanel = createMemo(() => Boolean(selectedDimmerControl()) && activeControlCategory() === "dimmer");
  const showPositionPad = createMemo(() => Boolean(selectedPositionControls()) && activeControlCategory() === "position");
  const showColorPad = createMemo(() => Boolean(selectedColorControls()) && activeControlCategory() === "color");
  const showCategoryQuickPanel = createMemo(() =>
    Boolean(selectedFixture()) &&
    visibleControls().length > 0 &&
    !["dimmer", "color", "position"].includes(activeControlCategory()),
  );
  const categoryQuickLooks = createMemo(() => quickLooksForCategory(activeControlCategory()));
  const visibleFunctionControls = createMemo(() =>
    visibleControls()
      .map((control) => ({
        control,
        functions: sortedChannelFunctions(control),
      }))
      .filter((entry) => entry.functions.length > 0),
  );
  const controlTargetLabel = createMemo(() =>
    selectedFixtureGroupFilter()
      ? `Group ${selectedFixtureGroupFilter()}`
      : selectedFixture()?.label ?? "No fixture selected",
  );
  const controlTargetKind = createMemo<"fixture" | "group" | "empty">(() =>
    selectedFixtureGroupFilter() ? "group" : selectedFixture() ? "fixture" : "empty",
  );
  const controlTargetDetail = createMemo(() => {
    const targetCount = selectedControlTargetFixtures().length;
    const attributeCount = activeControls().length;
    if (selectedFixtureGroupFilter()) {
      return `${targetCount} fixture${targetCount === 1 ? "" : "s"} / ${attributeCount} common attr${attributeCount === 1 ? "" : "s"}`;
    }
    return attributeCount > 0 ? `${attributeCount} attr${attributeCount === 1 ? "" : "s"}` : "No controls";
  });
  const controlReferenceLabel = createMemo(() => {
    const reference = selectedControlReferenceFixture();
    if (!reference) {
      return "No readout";
    }
    return selectedFixtureGroupFilter() ? `Ref ${reference.label}` : `U${reference.universe} A${reference.address}`;
  });
  const activeControlCategoryLabel = createMemo(() =>
    controlCategories.find((category) => category.id === activeControlCategory())?.label ?? activeControlCategory(),
  );
  const dimmerSliderRange = createMemo(() =>
    normalizeLimitRange(selectedFixtureLimits().dimmer_min, selectedFixtureLimits().dimmer_max),
  );
  const colorPreviewForSaturation = (saturation: number) => {
    const hsv = selectedColorHsv();
    const { red, green, blue } = hsvToRgb(hsv.hue, clamp01(saturation), Math.max(0.05, hsv.value));
    return rgbToHex(red, green, blue);
  };
  const colorSaturationRamp = createMemo(() =>
    `linear-gradient(to right, ${colorPreviewForSaturation(0)}, ${colorPreviewForSaturation(0.35)}, ${colorPreviewForSaturation(1)})`,
  );
  const currentControlValue = (control: AttributeControl) => {
    const fixture = selectedFixture();
    return fixture ? faderValue(fixture.id, control.attribute, control.default_value) : control.default_value;
  };
  const channelFunctionContainsValue = (
    fn: NonNullable<AttributeControl["functions"]>[number],
    value: number,
  ) => {
    const from = clampDmxValue(fn.dmx_from);
    const to = clampDmxValue(fn.dmx_to);
    return clampDmxValue(value) >= Math.min(from, to) && clampDmxValue(value) <= Math.max(from, to);
  };
  const channelFunctionBandStyle = (
    control: AttributeControl,
    fn: NonNullable<AttributeControl["functions"]>[number],
  ) => {
    const from = clampDmxValue(fn.dmx_from);
    const to = clampDmxValue(fn.dmx_to);
    const left = (Math.min(from, to) / 65_535) * 100;
    const width = Math.max(1, ((Math.abs(to - from) + 1) / 65_535) * 100);
    const swatch = normalizeHexColor(fn.wheel_slot_color);
    const normalized = normalizedFunctionText(control, fn);
    const background =
      swatch ??
      (/\b(open|clear|white)\b/.test(normalized)
        ? "rgba(245, 245, 245, 0.68)"
        : /\b(red)\b/.test(normalized)
          ? "rgba(255, 62, 62, 0.64)"
          : /\b(green)\b/.test(normalized)
            ? "rgba(69, 223, 64, 0.64)"
            : /\b(blue)\b/.test(normalized)
              ? "rgba(79, 119, 255, 0.64)"
              : "rgba(242, 193, 78, 0.42)");
    return {
      left: `${left}%`,
      width: `${width}%`,
      "background-color": background,
    };
  };
  const channelFunctionSwatchColor = (
    control: AttributeControl,
    fn: NonNullable<AttributeControl["functions"]>[number],
  ) => {
    const swatch = normalizeHexColor(fn.wheel_slot_color);
    if (swatch) {
      return swatch;
    }
    const normalized = normalizedFunctionText(control, fn);
    if (/\b(red)\b/.test(normalized)) {
      return "#ff3030";
    }
    if (/\b(green)\b/.test(normalized)) {
      return "#20e240";
    }
    if (/\b(blue)\b/.test(normalized)) {
      return "#3358ff";
    }
    if (/\b(amber|orange)\b/.test(normalized)) {
      return "#ff8a00";
    }
    if (/\b(cyan)\b/.test(normalized)) {
      return "#00d8ff";
    }
    if (/\b(magenta|pink)\b/.test(normalized)) {
      return "#ff2bd6";
    }
    if (/\b(white|open|clear)\b/.test(normalized)) {
      return "#ffffff";
    }
    return null;
  };
  const colorWheelEntries = createMemo<ColorWheelFunctionEntry[]>(() => {
    if (activeControlCategory() !== "color") {
      return [];
    }
    return visibleFunctionControls().flatMap((entry) =>
      entry.functions
        .filter((fn) => isColorWheelFunction(entry.control, fn))
        .map((fn) => {
          const currentValue = currentControlValue(entry.control);
          return {
            control: entry.control,
            fn,
            color: channelFunctionSwatchColor(entry.control, fn) ?? "#e7edf3",
            label: fn.wheel_slot_name || channelFunctionLabel(fn),
            currentValue,
            active: channelFunctionContainsValue(fn, currentValue),
          };
        }),
    );
  });
  const goboWheelEntries = createMemo<GoboWheelFunctionEntry[]>(() => {
    if (activeControlCategory() !== "gobo") {
      return [];
    }
    return visibleFunctionControls().flatMap((entry) =>
      entry.functions
        .filter((fn) => isGoboWheelFunction(entry.control, fn))
        .map((fn) => {
          const currentValue = currentControlValue(entry.control);
          return {
            control: entry.control,
            fn,
            label: fn.wheel_slot_name || channelFunctionLabel(fn),
            currentValue,
            active: channelFunctionContainsValue(fn, currentValue),
            pattern: goboPatternForFunction(entry.control, fn),
          };
        }),
    );
  });
  const opticsEntries = createMemo<OpticsControlEntry[]>(() => {
    const category = activeControlCategory();
    if (category !== "beam" && category !== "focus") {
      return [];
    }
    return visibleControls().map((control) => {
      const value = currentControlValue(control);
      const functions = sortedChannelFunctions(control);
      return {
        control,
        value,
        role: opticsRoleForControl(control, category),
        activeFunction: functions.find((fn) => channelFunctionContainsValue(fn, value)),
      };
    });
  });
  const showColorWheelPanel = createMemo(() => activeControlCategory() === "color" && colorWheelEntries().length > 0);
  const showGoboWheelPanel = createMemo(() => activeControlCategory() === "gobo" && goboWheelEntries().length > 0);
  const showOpticsPanel = createMemo(() =>
    (activeControlCategory() === "beam" || activeControlCategory() === "focus") && opticsEntries().length > 0,
  );
  const wheelMediaUrlForCurrentFixture = (media: string | null | undefined) => {
    const fixture = selectedFixture();
    if (!fixture || !media || !canLoadWheelMedia(fixture.profile_source_path)) {
      return null;
    }
    return wheelMediaUrls()[wheelMediaCacheKey(fixture.profile_source_path, media)] ?? null;
  };
  const visibleWheelMediaRequests = createMemo(() => {
    const fixture = selectedFixture();
    if (!fixture || !canLoadWheelMedia(fixture.profile_source_path)) {
      return [];
    }
    const mediaNames = new Set<string>();
    for (const entry of [...colorWheelEntries(), ...goboWheelEntries()]) {
      const media = entry.fn.wheel_slot_media?.trim();
      if (media) {
        mediaNames.add(media);
      }
    }
    return [...mediaNames].map((media) => ({
      key: wheelMediaCacheKey(fixture.profile_source_path, media),
      path: fixture.profile_source_path,
      media,
    }));
  });
  createEffect(() => {
    const urls = wheelMediaUrls();
    const loading = wheelMediaLoading();
    const missing = wheelMediaMissing();
    for (const request of visibleWheelMediaRequests()) {
      if (urls[request.key] || loading[request.key] || missing[request.key]) {
        continue;
      }
      setWheelMediaLoading((current) => ({ ...current, [request.key]: true }));
      void (async () => {
        try {
          const payload = await invoke<WheelMediaPayload | null>("load_gdtf_wheel_media", {
            path: request.path,
            media: request.media,
          });
          const bytes = payload?.bytes;
          const byteLength = bytes instanceof ArrayBuffer ? bytes.byteLength : bytes?.length ?? 0;
          if (!payload || !bytes || byteLength === 0) {
            setWheelMediaMissing((current) => ({ ...current, [request.key]: true }));
            return;
          }
          const objectUrl = wheelMediaPayloadToObjectUrl(payload);
          setWheelMediaUrls((current) => ({ ...current, [request.key]: objectUrl }));
        } catch {
          setWheelMediaMissing((current) => ({ ...current, [request.key]: true }));
        } finally {
          setWheelMediaLoading((current) => {
            const next = { ...current };
            delete next[request.key];
            return next;
          });
        }
      })();
    }
  });
  onCleanup(() => {
    for (const objectUrl of Object.values(wheelMediaUrls())) {
      URL.revokeObjectURL(objectUrl);
    }
  });
  const opticsPanelTitle = createMemo(() => (activeControlCategory() === "focus" ? "Focus / Optics" : "Beam / Optics"));
  const opticsPreviewClass = (role: string) => {
    const normalized = role.toLowerCase();
    if (normalized.includes("iris")) {
      return "iris";
    }
    if (normalized.includes("focus")) {
      return "focus";
    }
    if (normalized.includes("frost")) {
      return "frost";
    }
    if (normalized.includes("strobe")) {
      return "strobe";
    }
    if (normalized.includes("prism")) {
      return "prism";
    }
    return "beam";
  };
  const opticsPreviewStyle = (entry: OpticsControlEntry) => {
    const ratio = clamp01(entry.value / 65_535);
    const role = entry.role.toLowerCase();
    const isIris = role.includes("iris");
    const isFocus = role.includes("focus");
    const isFrost = role.includes("frost");
    const isPrism = role.includes("prism");
    const isStrobe = role.includes("strobe");
    const size = isIris
      ? 18 + ratio * 78
      : isFocus
        ? 70
        : isFrost
          ? 72 + ratio * 18
          : 36 + ratio * 72;
    const blur = isFocus || isFrost ? 1 + ratio * 10 : 0.4 + (1 - ratio) * 2.2;
    return {
      "--optics-level": `${Math.round(ratio * 100)}%`,
      "--optics-size": `${Math.round(size)}px`,
      "--optics-blur": `${Number(blur.toFixed(2))}px`,
      "--optics-opacity": `${isStrobe ? 0.45 + ratio * 0.55 : 0.72 + ratio * 0.24}`,
      "--optics-ring-size": `${Math.round(42 + ratio * 88)}px`,
      "--optics-prism-opacity": `${isPrism ? 0.35 + ratio * 0.65 : 0}`,
      "--optics-prism-rotation": `${Math.round(ratio * 360)}deg`,
    };
  };
  const opticsPresetButtons = (entry: OpticsControlEntry) => {
    const role = entry.role.toLowerCase();
    if (role.includes("shutter") || role.includes("strobe")) {
      return [
        { label: "Off", value: 0 },
        { label: "Open", value: 65_535 },
        { label: "Pulse", value: 49_152 },
      ];
    }
    if (role.includes("iris")) {
      return [
        { label: "Small", value: 8192 },
        { label: "Mid", value: 32_768 },
        { label: "Open", value: 65_535 },
      ];
    }
    if (role.includes("focus")) {
      return [
        { label: "Near", value: 0 },
        { label: "Mid", value: 32_768 },
        { label: "Far", value: 65_535 },
      ];
    }
    if (role.includes("frost") || role.includes("prism")) {
      return [
        { label: "Off", value: 0 },
        { label: "Mid", value: 32_768 },
        { label: "On", value: 65_535 },
      ];
    }
    return [
      { label: "Tight", value: 8192 },
      { label: "Mid", value: 32_768 },
      { label: "Wide", value: 65_535 },
    ];
  };
  const effectTargetFixtures = createMemo<PatchedFixtureSummary[]>(() => {
    switch (effectTargetMode()) {
      case "fixture": {
        const fixture = selectedFixture();
        return fixture ? [fixture] : [];
      }
      case "selection":
        return selectedMappingFixtures();
      case "group": {
        const groups = parseGroupIds(effectTargetGroups());
        if (groups.length === 0) {
          return [];
        }
        return snapshot().fixtures.filter((fixture) => groups.some((groupId) => fixture.group_ids.includes(groupId)));
      }
      case "video":
      default:
        return [];
    }
  });
  const effectTargetControls = createMemo<AttributeControl[]>(() => commonAttributeControls(effectTargetFixtures()));
  const selectedEffectAttribute = createMemo(() => {
    const controls = effectTargetControls();
    const current = effectAttribute();
    if (controls.some((control) => control.attribute === current)) {
      return current;
    }
    return controls[0]?.attribute ?? "";
  });
  const effectTargetSummary = createMemo(() => {
    const fixtureCount = effectTargetFixtures().length;
    const attributeCount = effectTargetControls().length;
    switch (effectTargetMode()) {
      case "selection":
        return `${fixtureCount} mapped fixture${fixtureCount === 1 ? "" : "s"} / ${attributeCount} common attribute${attributeCount === 1 ? "" : "s"}`;
      case "group": {
        const groups = parseGroupIds(effectTargetGroups());
        return groups.length > 0
          ? `${groups.join(", ")} / ${fixtureCount} fixture${fixtureCount === 1 ? "" : "s"} / ${attributeCount} common attribute${attributeCount === 1 ? "" : "s"}`
          : "No group target";
      }
      case "video":
        return selectedEffectVideoLayerId() === null
          ? "No video layer target"
          : `Video Layer ${selectedEffectVideoLayerId()} / ${effectVideoParam()} ${Math.round(effectVideoLow() * 100)}-${Math.round(effectVideoHigh() * 100)}%`;
      case "fixture":
      default:
        return selectedFixture()?.label ?? "No fixture selected";
    }
  });
  const effectDraftSummary = createMemo(() => {
    if (effectType() === "PositionWave") {
      return `Wave O ${waveOriginX().toFixed(1)},${waveOriginY().toFixed(1)},${waveOriginZ().toFixed(1)} / D ${waveDirectionX().toFixed(1)},${waveDirectionY().toFixed(1)},${waveDirectionZ().toFixed(1)} / ${waveWavelength().toFixed(1)}m`;
    }
    const sync = effectClockSyncBeats();
    return sync === null ? `LFO ${effectShape()} / ${effectPeriod()}ms` : `LFO ${effectShape()} / ${sync} beat`;
  });
  const editingEffectSummary = createMemo<EffectSummary | null>(() => {
    const effectId = editingEffectId();
    if (effectId === null) {
      return null;
    }
    return snapshot().effects.find((effect) => effect.id === effectId) ?? null;
  });
  const dmxPreviewOptions = createMemo(() =>
    snapshot().dmx_previews.length > 0
      ? snapshot().dmx_previews
      : [{ universe: snapshot().output.universe, values: snapshot().dmx_preview }],
  );
  const activeDmxPreview = createMemo(() => {
    const previews = dmxPreviewOptions();
    return previews.find((preview) => preview.universe === rawDmxUniverse()) ?? previews[0];
  });
  const activeDmxPreviewUniverse = createMemo(() => activeDmxPreview()?.universe ?? snapshot().output.universe);
  const activeDmxPreviewValues = createMemo(() => activeDmxPreview()?.values ?? snapshot().dmx_preview);
  const dmxCells = createMemo(() =>
    activeDmxPreviewValues().map((value, index) => ({
      channel: index + 1,
      value,
    })),
  );
  const nonZeroDmxCount = createMemo(() => activeDmxPreviewValues().filter((value) => value !== 0).length);
  const audioWaveformPoints = createMemo(() => {
    const analysis = audioAnalysis();
    if (!analysis || analysis.waveform.length === 0) {
      return "";
    }
    const width = 100;
    const height = 36;
    const duration = Math.max(analysis.duration_ms, 1);
    return analysis.waveform
      .map((point) => {
        const x = Math.min(width, (point.time_ms / duration) * width);
        const y = height - Math.min(height, point.peak * height);
        return `${x.toFixed(2)},${y.toFixed(2)}`;
      })
      .join(" ");
  });
  const audioBeatMarkers = createMemo(() => {
    const analysis = audioAnalysis();
    if (!analysis || analysis.duration_ms === 0) {
      return [];
    }
    return analysis.beats.slice(0, 128).map((beat) => ({
      time_ms: beat,
      x: Math.min(100, (beat / analysis.duration_ms) * 100),
    }));
  });
  const audioSpectrumPaths = createMemo(() => {
    const analysis = audioAnalysis();
    const empty = { bass: "", mid: "", high: "" };
    if (!analysis || analysis.spectrum.length === 0 || analysis.duration_ms === 0) {
      return empty;
    }
    const step = Math.max(1, Math.ceil(analysis.spectrum.length / 128));
    const rows = [
      { key: "bass" as const, top: 0 },
      { key: "mid" as const, top: 12 },
      { key: "high" as const, top: 24 },
    ];
    return Object.fromEntries(rows.map(({ key, top }) => [
      key,
      analysis.spectrum
        .filter((_, index) => index % step === 0 || index === analysis.spectrum.length - 1)
        .map((point) => {
          const x = Math.min(100, (point.time_ms / analysis.duration_ms) * 100);
          const y = top + 11 - Math.min(10, point[key] * 10);
          return `${x.toFixed(2)},${y.toFixed(2)}`;
        })
        .join(" "),
    ])) as { bass: string; mid: string; high: string };
  });
  const audioBeatTimes = createMemo(() => audioAnalysis()?.beats ?? []);
  const beatIntervalMs = createMemo(() => {
    const bpm = audioAnalysis()?.estimated_bpm ?? snapshot().clock.bpm;
    return bpm > 0 ? 60_000 / bpm : 500;
  });
  const nearestTimelineValue = (timeMs: number, candidates: number[]) => {
    if (candidates.length === 0) {
      return Math.max(0, Math.round(timeMs));
    }
    let nearest = candidates[0];
    let nearestDistance = Math.abs(timeMs - nearest);
    for (let index = 1; index < candidates.length; index += 1) {
      const candidate = candidates[index];
      const distance = Math.abs(timeMs - candidate);
      if (distance < nearestDistance) {
        nearest = candidate;
        nearestDistance = distance;
      }
    }
    return Math.max(0, Math.round(nearest));
  };
  const snapTimeMs = (timeMs: number) => {
    const clamped = Math.max(0, Number.isFinite(timeMs) ? timeMs : 0);
    switch (timelineSnapMode()) {
      case "Beat": {
        const beats = audioBeatTimes();
        if (beats.length > 0) {
          return nearestTimelineValue(clamped, beats);
        }
        const beat = Math.max(1, beatIntervalMs());
        return Math.round(Math.round(clamped / beat) * beat);
      }
      case "Bar": {
        const bar = Math.max(1, beatIntervalMs() * 4);
        return Math.round(Math.round(clamped / bar) * bar);
      }
      case "Grid": {
        const grid = Math.max(1, timelineGridMs());
        return Math.round(clamped / grid) * grid;
      }
      default:
        return Math.round(clamped);
    }
  };
  const timelineCueOptions = createMemo(() => snapshot().cues.map((cue) => ({ id: cue.id, label: cue.label })));
  const selectedTimelineCueId = createMemo(() => {
    const current = timelineCueId();
    const cues = timelineCueOptions();
    if (current !== null && cues.some((cue) => cue.id === current)) {
      return current;
    }
    return cues[0]?.id ?? null;
  });
  const timelineEventRows = createMemo(() =>
    snapshot().timeline.events.map((event) => {
      const cue = snapshot().cues.find((candidate) => candidate.id === event.cue_id);
      return {
        ...event,
        cue_label: cue?.label ?? `Cue ${event.cue_id}`,
      };
    }),
  );
  const timelineOverviewDurationMs = createMemo(() => {
    const lightingAutomationEndMs = snapshot().timeline.automations.flatMap((automation) =>
      automation.keyframes.map((keyframe) => keyframe.time_ms),
    );
    const videoAutomationEndMs = snapshot().timeline.video_automations.flatMap((automation) =>
      automation.keyframes.map((keyframe) => keyframe.time_ms),
    );
    return Math.max(
      1,
      snapshot().timeline.duration_ms,
      ...snapshot().timeline.events.map((event) => event.time_ms + 1000),
      ...lightingAutomationEndMs,
      ...videoAutomationEndMs,
    );
  });
  const timelineOverviewPlayheadX = createMemo(() =>
    clampRange((snapshot().timeline.position_ms / timelineOverviewDurationMs()) * 100, 0, 100),
  );
  const timelineOverviewEvents = createMemo<TimelineOverviewEvent[]>(() =>
    timelineEventRows().map((event) => ({
      id: event.id,
      cue_id: event.cue_id,
      cue_label: event.cue_label,
      track: event.track,
      time_ms: event.time_ms,
      x: clampRange((event.time_ms / timelineOverviewDurationMs()) * 100, 0, 100),
      y: event.track === "Lighting" ? 14 : 31,
      active: snapshot().active_cue_id === event.cue_id,
    })),
  );
  const timelineOverviewAutomationRanges = createMemo<TimelineOverviewAutomationRange[]>(() => {
    const duration = timelineOverviewDurationMs();
    const currentSnapshot = snapshot();
    const lightingGroupId = selectedFixtureGroupFilter();
    const lightingFixtureId = selectedFixtureId();
    const lightingGroupFixtureIds =
      lightingAutomationRowScope() === "current" && lightingGroupId
        ? new Set(
            currentSnapshot.fixtures
              .filter((fixture) => fixture.group_ids.includes(lightingGroupId))
              .map((fixture) => fixture.id),
          )
        : null;
    const visibleLightingAutomation = (fixtureId: number) => {
      if (lightingAutomationRowScope() === "all") {
        return true;
      }
      if (lightingGroupFixtureIds) {
        return lightingGroupFixtureIds.has(fixtureId);
      }
      return lightingFixtureId === null || fixtureId === lightingFixtureId;
    };
    const visibleVideoAutomationLayerId = (() => {
      const current = videoAutomationLayerId();
      if (current !== null && currentSnapshot.video.layers.some((layer) => layer.id === current)) {
        return current;
      }
      return currentSnapshot.video.layers[0]?.id ?? null;
    })();
    const visibleVideoAutomation = (layerId: number) =>
      videoAutomationRowScope() === "all" ||
      visibleVideoAutomationLayerId === null ||
      layerId === visibleVideoAutomationLayerId;
    const rangeFor = (
      id: string,
      kind: "lighting" | "video",
      automationId: number,
      targetId: number,
      label: string,
      track: TimelineTrackKind,
      keyframes: { time_ms: number }[],
      enabled: boolean,
    ): TimelineOverviewAutomationRange | null => {
      if (keyframes.length === 0) {
        return null;
      }
      const times = keyframes.map((keyframe) => keyframe.time_ms);
      const startMs = Math.min(...times);
      const endMs = Math.max(...times);
      const x = clampRange((startMs / duration) * 100, 0, 100);
      const width = Math.max(0.75, clampRange(((endMs - startMs) / duration) * 100, 0, 100 - x));
      return {
        id,
        kind,
        automation_id: automationId,
        target_id: targetId,
        label,
        track,
        start_ms: startMs,
        end_ms: endMs,
        keyframes: keyframes
          .map((keyframe, keyframeIndex) => ({
            keyframe_index: keyframeIndex,
            time_ms: keyframe.time_ms,
          }))
          .sort((left, right) => left.time_ms - right.time_ms)
          .map((keyframe) => ({
            keyframe_index: keyframe.keyframe_index,
            time_ms: keyframe.time_ms,
          })),
        x,
        width,
        y: track === "Lighting" ? 17.2 : 35.2,
        enabled,
      };
    };
    const lightingRanges = currentSnapshot.timeline.automations
      .filter((automation) => visibleLightingAutomation(automation.fixture_id))
      .map((automation) => {
        const fixture = currentSnapshot.fixtures.find((candidate) => candidate.id === automation.fixture_id);
        return rangeFor(
          `l-${automation.id}`,
          "lighting",
          automation.id,
          automation.fixture_id,
          `${fixture?.label ?? `Fixture ${automation.fixture_id}`} ${automation.attribute}`,
          automation.track,
          automation.keyframes,
          automation.enabled,
        );
      })
      .filter((range): range is TimelineOverviewAutomationRange => Boolean(range));
    const videoRanges = currentSnapshot.timeline.video_automations
      .filter((automation) => visibleVideoAutomation(automation.layer_id))
      .map((automation) => {
        const layer = currentSnapshot.video.layers.find((candidate) => candidate.id === automation.layer_id);
        return rangeFor(
          `v-${automation.id}`,
          "video",
          automation.id,
          automation.layer_id,
          `${layer?.label ?? `Video Layer ${automation.layer_id}`} ${automation.param}`,
          automation.track,
          automation.keyframes,
          automation.enabled,
        );
      })
      .filter((range): range is TimelineOverviewAutomationRange => Boolean(range));
    return [...lightingRanges, ...videoRanges];
  });
  const selectedTimelineAutomationRangeId = createMemo(() => {
    const selected = selectedTimelineAutomation();
    if (!selected) {
      return null;
    }
    return `${selected.kind === "lighting" ? "l" : "v"}-${selected.automationId}`;
  });
  const selectedLightingTimelineAutomationId = createMemo(() => {
    const selected = selectedTimelineAutomation();
    return selected?.kind === "lighting" ? selected.automationId : null;
  });
  const selectedVideoTimelineAutomationId = createMemo(() => {
    const selected = selectedTimelineAutomation();
    return selected?.kind === "video" ? selected.automationId : null;
  });
  const cueTimelinePlacementsForCue = (cueId: number) =>
    timelineEventRows()
      .filter((event) => event.cue_id === cueId)
      .sort((left, right) => left.time_ms - right.time_ms || left.id - right.id);
  const timelinePlacementNudgeMs = createMemo(() => {
    switch (timelineSnapMode()) {
      case "Beat":
        return Math.max(1, beatIntervalMs());
      case "Bar":
        return Math.max(1, beatIntervalMs() * 4);
      case "Grid":
        return Math.max(1, timelineGridMs());
      default:
        return 1000;
    }
  });
  const fixtureAttributeOptions = (fixtureId: number) =>
    snapshot().fixtures.find((fixture) => fixture.id === fixtureId)?.controls.map((control) => control.attribute) ?? [];
  const allTimelineAutomationRows = createMemo(() =>
    snapshot().timeline.automations.map((automation) => {
      const fixture = snapshot().fixtures.find((candidate) => candidate.id === automation.fixture_id);
      return {
        ...automation,
        fixture_label: fixture?.label ?? `Fixture ${automation.fixture_id}`,
      };
    }),
  );
  const timelineAutomationRows = createMemo(() => {
    const rows = allTimelineAutomationRows();
    if (lightingAutomationRowScope() === "all") {
      return rows;
    }
    const groupId = selectedFixtureGroupFilter();
    if (groupId) {
      const groupFixtureIds = new Set(
        snapshot()
          .fixtures.filter((fixture) => fixture.group_ids.includes(groupId))
          .map((fixture) => fixture.id),
      );
      return rows.filter((automation) => groupFixtureIds.has(automation.fixture_id));
    }
    const fixtureId = selectedFixtureId();
    return fixtureId === null ? rows : rows.filter((automation) => automation.fixture_id === fixtureId);
  });
  const selectedVideoAutomationLayerId = createMemo(() => {
    const current = videoAutomationLayerId();
    const layers = snapshot().video.layers;
    if (current !== null && layers.some((layer) => layer.id === current)) {
      return current;
    }
    return layers[0]?.id ?? null;
  });
  const selectedMidiCueId = createMemo(() => {
    const current = midiMapCueId();
    const cues = snapshot().cues;
    if (current !== null && cues.some((cue) => cue.id === current)) {
      return current;
    }
    return cues[0]?.id ?? null;
  });
  const selectedMidiEffectId = createMemo(() => {
    const current = midiMapEffectId();
    const effects = snapshot().effects;
    if (current !== null && effects.some((effect) => effect.id === current)) {
      return current;
    }
    return effects[0]?.id ?? null;
  });
  const selectedMidiNodeGraphId = createMemo(() => {
    const current = midiMapNodeGraphId();
    const nodeGraphs = snapshot().node_graphs;
    if (current !== null && nodeGraphs.some((graph) => graph.id === current)) {
      return current;
    }
    return nodeGraphs[0]?.id ?? null;
  });
  const selectedMidiLayerId = createMemo(() => {
    const current = midiMapLayerId();
    const layers = snapshot().video.layers;
    if (current !== null && layers.some((layer) => layer.id === current)) {
      return current;
    }
    return layers[0]?.id ?? null;
  });
  const selectedMidiVideoOutputId = createMemo(() => {
    const current = midiMapVideoOutputId();
    const outputs = snapshot().video.outputs;
    if (current !== null && outputs.some((output) => output.id === current)) {
      return current;
    }
    return outputs[0]?.id ?? null;
  });
  const selectedMidiVideoOutputMappingField = createMemo(
    () => videoOutputMappingFieldOption(midiMapVideoOutputMappingField()).value,
  );
  const selectedMidiVideoOutputMappingPresetLabel = createMemo(() => {
    const current = midiMapVideoOutputMappingPresetLabel().trim();
    const presets = snapshot().video.mapping_presets;
    if (current && presets.some((preset) => preset.label === current)) {
      return current;
    }
    return presets[0]?.label ?? current;
  });
  const selectedOscCueId = createMemo(() => {
    const current = oscMapCueId();
    const cues = snapshot().cues;
    if (current !== null && cues.some((cue) => cue.id === current)) {
      return current;
    }
    return cues[0]?.id ?? null;
  });
  const selectedOscEffectId = createMemo(() => {
    const current = oscMapEffectId();
    const effects = snapshot().effects;
    if (current !== null && effects.some((effect) => effect.id === current)) {
      return current;
    }
    return effects[0]?.id ?? null;
  });
  const selectedOscNodeGraphId = createMemo(() => {
    const current = oscMapNodeGraphId();
    const nodeGraphs = snapshot().node_graphs;
    if (current !== null && nodeGraphs.some((graph) => graph.id === current)) {
      return current;
    }
    return nodeGraphs[0]?.id ?? null;
  });
  const selectedOscLayerId = createMemo(() => {
    const current = oscMapLayerId();
    const layers = snapshot().video.layers;
    if (current !== null && layers.some((layer) => layer.id === current)) {
      return current;
    }
    return layers[0]?.id ?? null;
  });
  const selectedOscVideoOutputId = createMemo(() => {
    const current = oscMapVideoOutputId();
    const outputs = snapshot().video.outputs;
    if (current !== null && outputs.some((output) => output.id === current)) {
      return current;
    }
    return outputs[0]?.id ?? null;
  });
  const selectedOscVideoOutputMappingField = createMemo(
    () => videoOutputMappingFieldOption(oscMapVideoOutputMappingField()).value,
  );
  const selectedOscVideoOutputMappingPresetLabel = createMemo(() => {
    const current = oscMapVideoOutputMappingPresetLabel().trim();
    const presets = snapshot().video.mapping_presets;
    if (current && presets.some((preset) => preset.label === current)) {
      return current;
    }
    return presets[0]?.label ?? current;
  });
  const midiClearFixtureFlagKind = createMemo(() => normalizeFixtureFlagClearKind(midiMapAttribute()));
  const oscClearFixtureFlagKind = createMemo(() => normalizeFixtureFlagClearKind(oscMapAttribute()));
  const setMidiVideoOutputMappingFieldTarget = (field: string) => {
    const option = videoOutputMappingFieldOption(field);
    setMidiMapVideoOutputMappingField(option.value);
    setMidiMapLow(option.low);
    setMidiMapHigh(option.high);
  };
  const setOscVideoOutputMappingFieldTarget = (field: string) => {
    const option = videoOutputMappingFieldOption(field);
    setOscMapVideoOutputMappingField(option.value);
    setOscMapLow(option.low);
    setOscMapHigh(option.high);
  };
  const setMidiControlMappingAction = (action: MidiControlAction) => {
    setMidiMapAction(action);
    if (action === "VideoOutputMappingField") {
      setMidiVideoOutputMappingFieldTarget(selectedMidiVideoOutputMappingField());
    }
  };
  const setOscControlMappingAction = (action: OscControlAction) => {
    setOscMapAction(action);
    if (action === "VideoOutputMappingField") {
      setOscVideoOutputMappingFieldTarget(selectedOscVideoOutputMappingField());
    }
  };
  const selectedEffectVideoLayerId = createMemo(() => {
    const current = effectVideoLayerId();
    const layers = snapshot().video.layers;
    if (current !== null && layers.some((layer) => layer.id === current)) {
      return current;
    }
    return layers[0]?.id ?? null;
  });
  const videoDecoderDiagnosticsLabel = (diagnostics: VideoDecoderDiagnostics) =>
    `routes HAP ${diagnostics.hap_successes}/${diagnostics.hap_requests}, libav ${diagnostics.libav_successes}/${diagnostics.libav_requests}, CLI ${diagnostics.cli_fallback_successes}/${diagnostics.cli_fallback_requests}, deferred ${diagnostics.deferred_requests}, failed ${diagnostics.decode_failures}, cache ${diagnostics.hap_cache_len}+${diagnostics.libav_cache_len}+${diagnostics.cli_cache_len}`;
  const videoPreviewDiagnosticsText = createMemo(() => {
    const diagnostics = videoPreviewDiagnostics();
    if (!diagnostics) {
      return "No preview diagnostics";
    }
    const queuedFrames = diagnostics.layer_queues.reduce((total, row) => total + row.queue_len, 0);
    const activeQueues = diagnostics.layer_queues.filter((row) => row.queue_len > 0).length;
    const readyQueues = diagnostics.layer_queues.filter((row) => row.ready).length;
    const bpm = diagnostics.bpm && Number.isFinite(diagnostics.bpm) ? `, bpm ${diagnostics.bpm.toFixed(1)}` : "";
    const isf = `, ISF pipelines ${diagnostics.isf_pipeline_count}${diagnostics.last_isf_error ? " error" : ""}`;
    return `queues ${activeQueues}/${diagnostics.queue_count}, ready ${readyQueues}/${diagnostics.layer_queues.length}, frames ${queuedFrames}, cap ${diagnostics.frame_queue_capacity}, still ${diagnostics.still_image_cache_len}, decode ${diagnostics.decoder_cache_len}, ${videoDecoderDiagnosticsLabel(diagnostics.decoder_diagnostics)}, prefetch ${diagnostics.prefetch_count}x${diagnostics.prefetch_interval_ms}ms${bpm}${isf}`;
  });
  const videoPreviewLayerDiagnostics = createMemo(() => videoPreviewDiagnostics()?.layer_queues ?? []);
  const videoPreviewPrefetchPlanLabel = (row: VideoPreviewDiagnostics["layer_queues"][number]) => {
    const positions = row.expected_positions_ms.slice(0, 4).map((position) => formatVideoTime(position));
    const suffix = row.expected_positions_ms.length > positions.length ? "..." : "";
    const plan = [...positions, suffix].filter(Boolean).join(" -> ");
    return plan ? `plan ${plan}` : "plan none";
  };
  const videoPreviewLayerDiagnosticLabel = (row: VideoPreviewDiagnostics["layer_queues"][number]) =>
    `${videoSourceKindLabel(row.source_kind)} / ${row.playing ? "playing" : "paused"} / ${row.effective_speed.toFixed(2)}x / ${formatVideoTime(row.position_ms, row.source_duration_ms)} / queue ${row.queue_len}/${row.expected_queue_len} / ${videoPreviewPrefetchPlanLabel(row)}`;
  const videoPreviewLayerDiagnosticClass = (row: VideoPreviewDiagnostics["layer_queues"][number]) =>
    row.ready ? "previewDiagnosticRow ready" : row.queue_len > 0 ? "previewDiagnosticRow warming" : "previewDiagnosticRow missing";
  const videoPreviewOutputDecodePlans = createMemo(() => videoPreviewDiagnostics()?.output_decode_previews ?? []);
  const videoPreviewOutputDecodePlanLabel = (row: VideoPreviewDiagnostics["output_decode_previews"][number]) => {
    if (row.error) {
      return `decode plan error / ${row.error}`;
    }
    const report = row.report;
    if (!report) {
      return "decode plan unavailable";
    }
    return `layers ${report.layers_considered}, requests ${report.requests_attempted}, current/lookahead ${report.inserted + report.duplicate + report.reprioritized}, rejected ${report.rejected_full}, pending ${report.pending}`;
  };
  const videoPreviewOutputDecodePlanClass = (row: VideoPreviewDiagnostics["output_decode_previews"][number]) => {
    if (row.error) {
      return "previewDiagnosticRow missing";
    }
    if (row.blackout || !row.enabled || (row.report?.requests_attempted ?? 0) === 0) {
      return "previewDiagnosticRow warming";
    }
    return row.report?.rejected_full ? "previewDiagnosticRow missing" : "previewDiagnosticRow ready";
  };
  const videoOutputRenderPlanSummary = createMemo(() => {
    const plans = videoOutputRenderPlans();
    if (!plans) {
      return "Not checked";
    }
    const activeOutputs = plans.filter((plan) => plan.enabled && !plan.output_blackout).length;
    const blackoutOutputs = plans.filter((plan) => plan.output_blackout).length;
    const activeLayers = plans.reduce((total, plan) => total + plan.composition.layers.length, 0);
    return `${plans.length} video output(s), ${activeOutputs} active, ${activeLayers} render layer(s), ${blackoutOutputs} blackout`;
  });
  const videoOutputMappingDiagnosticLabel = (mapping: VideoOutputMapping) => {
    const cornerWarp =
      Math.abs(mapping.corner_top_left_x) +
      Math.abs(mapping.corner_top_left_y) +
      Math.abs(mapping.corner_top_right_x) +
      Math.abs(mapping.corner_top_right_y) +
      Math.abs(mapping.corner_bottom_right_x) +
      Math.abs(mapping.corner_bottom_right_y) +
      Math.abs(mapping.corner_bottom_left_x) +
      Math.abs(mapping.corner_bottom_left_y);
    const warpParts = [
      mapping.aspect_mode !== "Stretch" ? mapping.aspect_mode : null,
      Math.abs(mapping.lens_distortion) > 0.001 ? `lens ${mapping.lens_distortion.toFixed(2)}` : null,
      Math.abs(mapping.keystone_x) > 0.001 || Math.abs(mapping.keystone_y) > 0.001
        ? `key ${mapping.keystone_x.toFixed(2)},${mapping.keystone_y.toFixed(2)}`
        : null,
      cornerWarp > 0.001 ? "corner warp" : null,
    ].filter((part): part is string => Boolean(part));
    return warpParts.length > 0 ? warpParts.join(" / ") : "flat mapping";
  };
  const videoOutputRenderPlanRows = createMemo(() =>
    (videoOutputRenderPlans() ?? []).map((plan) => {
      const layerCount = plan.composition.layers.length;
      const stateLabel = !plan.enabled
        ? "Disabled"
        : plan.output_blackout
          ? "Blackout"
          : layerCount > 0
            ? "Live"
            : "Empty";
      const stateClass = !plan.enabled
        ? "disabled"
        : plan.output_blackout
          ? "blackout"
          : layerCount > 0
            ? "live"
            : "empty";
      const endpoint = plan.kind === "Display" ? `Monitor ${plan.monitor_id ?? 0}` : plan.endpoint_name ?? plan.label;
      const layerLabels = plan.composition.layers.map((layer) => `${layer.label} ${Math.round(layer.opacity * 100)}%`);
      return {
        id: plan.output_id,
        label: plan.label,
        endpoint,
        stateLabel,
        stateClass,
        detail: `${plan.kind} / ${plan.width}x${plan.height} / ${plan.composition.label} / ${layerCount} layer(s) / ${Math.round(
          plan.output_opacity * 100,
        )}% / ${videoOutputMappingDiagnosticLabel(plan.mapping)}${layerLabels.length > 0 ? ` / ${layerLabels.join(", ")}` : ""}`,
      };
    }),
  );
  const videoOutputRenderPlanForOutput = (outputId: number) =>
    videoOutputRenderPlans()?.find((plan) => plan.output_id === outputId) ?? null;
  const videoOutputRenderPlanState = (output: VideoOutputSummary) => {
    const plan = videoOutputRenderPlanForOutput(output.id);
    const layerCount = plan?.composition.layers.length ?? 0;
    const stateLabel = plan
      ? !plan.enabled
        ? "Disabled"
        : plan.output_blackout
          ? "Blackout"
          : layerCount > 0
            ? "Live"
            : "Empty"
      : "Plan not checked";
    const stateClass = plan
      ? !plan.enabled
        ? "disabled"
        : plan.output_blackout
          ? "blackout"
          : layerCount > 0
            ? "live"
            : "empty"
      : "unchecked";
    const layerLabels = plan?.composition.layers.map((layer) => `${layer.label} ${Math.round(layer.opacity * 100)}%`) ?? [];
    const detail = plan
      ? `${plan.composition.label} / ${layerCount} layer(s) / ${Math.round(plan.output_opacity * 100)}% / ${videoOutputMappingDiagnosticLabel(
          plan.mapping,
        )}${layerLabels.length > 0 ? ` / ${layerLabels.join(", ")}` : ""}`
      : "Run Check Plans to inspect the routed composition.";
    return { plan, layerCount, stateLabel, stateClass, detail };
  };
  const videoOutputWindowStatusForOutput = (outputId: number) =>
    videoOutputWindowStatuses()?.find((status) => status.output_id === outputId) ?? null;
  const videoOutputWindowSummary = createMemo(() => {
    const statuses = videoOutputWindowStatuses();
    if (!statuses) {
      return "Windows not checked";
    }
    const liveOpen = statuses.filter((status) => status.live_open).length;
    const patternOpen = statuses.filter((status) => status.test_pattern_open).length;
    return `${liveOpen} live, ${patternOpen} pattern open / ${statuses.length} display video output(s)`;
  });
  const videoOutputWindowState = (outputId: number) => {
    const status = videoOutputWindowStatusForOutput(outputId);
    if (!status) {
      return {
        stateLabel: "Window not checked",
        stateClass: "unchecked",
        detail: "Run Check Windows to inspect Live/Test output windows.",
      };
    }
    const openCount = Number(status.live_open) + Number(status.test_pattern_open);
    const performance = status.performance;
    const budgetDetail = performance
      ? performance.frame_budget_pass === true
        ? " / 60fps budget pass"
        : performance.frame_budget_pass === false
          ? " / 60fps budget fail"
          : ` / budget sampling ${performance.frame_count}/120`
      : "";
    const decoderDetail = performance
      ? ` / ${videoDecoderDiagnosticsLabel(performance.decoder_diagnostics)}`
      : "";
    const performanceDetail = performance
      ? performance.warmup_remaining > 0
        ? ` / ${performance.width}x${performance.height} / warming ${performance.warmup_remaining} frame(s) / GPU alloc ${performance.output_reallocations}+${performance.layer_reallocations} / BC ${performance.compressed_layer_uploads}${budgetDetail}${decoderDetail}`
        : ` / ${performance.width}x${performance.height} / avg ${(performance.average_frame_us / 1000).toFixed(2)} ms / max ${(
            performance.max_frame_us / 1000
          ).toFixed(2)} ms / ${performance.deadline_miss_count} late / GPU alloc ${performance.output_reallocations}+${
            performance.layer_reallocations
          } / BC ${performance.compressed_layer_uploads}${budgetDetail}${decoderDetail}`
      : "";
    if (performance?.last_error) {
      return {
        stateLabel: "Output error",
        stateClass: "error",
        detail: `${performance.last_error}${performanceDetail}`,
      };
    }
    const baseStateLabel =
      openCount === 2
        ? "Live + Pattern open"
        : status.live_open
          ? "Live window open"
          : status.test_pattern_open
            ? "Pattern window open"
            : "Window closed";
    const stateLabel = performance
      ? `${baseStateLabel} / ${performance.width}x${performance.height} / ${(performance.average_frame_us / 1000).toFixed(1)}ms / ${
          performance.frame_budget_pass === true
            ? "60 PASS"
            : performance.frame_budget_pass === false
              ? "60 FAIL"
              : `${performance.frame_count}/120`
        }`
      : baseStateLabel;
    return {
      stateLabel,
      stateClass: openCount > 0 ? "open" : "closed",
      detail: `Live ${status.live_open ? "open" : "closed"} / Pattern ${
        status.test_pattern_open ? "open" : "closed"
      }${performanceDetail}`,
    };
  };
  const videoRuntimeBackendCounts = createMemo(() => {
    const backends = videoRuntimeStatus()?.backends ?? [];
    return {
      available: backends.filter((backend) => backend.state === "Available").length,
      missing: backends.filter((backend) => backend.state === "Missing").length,
      notBuilt: backends.filter((backend) => backend.state === "NotBuilt").length,
      total: backends.length,
    };
  });
  const videoRuntimeBackendSummary = createMemo(() => {
    const counts = videoRuntimeBackendCounts();
    if (counts.total === 0) {
      return "Not checked";
    }
    return `${counts.available}/${counts.total} available, ${counts.missing} missing, ${counts.notBuilt} not built`;
  });
  const videoRuntimeBackendClass = (state: VideoRuntimeStatus["backends"][number]["state"]) =>
    `videoBackendPill state-${state.toLowerCase()}`;
  const externalVideoIoPlanSummary = createMemo(() => {
    const plans = externalVideoIoPlans();
    if (!plans) {
      return "Not checked";
    }
    const liveInputs = plans.inputs.filter((plan) => plan.live).length;
    const liveOutputs = plans.outputs.filter((plan) => plan.live).length;
    const blockedRoutes = [...plans.inputs, ...plans.outputs].filter((plan) => !plan.ready).length;
    const blocked = blockedRoutes > 0 ? `, ${blockedRoutes} blocked` : "";
    return `${plans.inputs.length} in (${liveInputs} live), ${plans.outputs.length} out (${liveOutputs} live)${blocked}`;
  });
  const externalVideoTransportSummary = createMemo(() => {
    const report = externalVideoTransportReport();
    if (!report) {
      const status = externalVideoTransportStatus();
      return status ? `active ${status.active_count}, not synced` : "Routes not synced";
    }
    const failed = report.start_failed.length + report.stop_failed.length;
    const failedText = failed > 0 ? `, failed ${failed}` : "";
    return `active ${report.active_count}, +${report.started.length}, =${report.kept.length}, -${report.stopped.length}, blocked ${report.blocked.length}, idle ${report.idle.length}${failedText}`;
  });
  const externalVideoIoPlanRows = createMemo(() => {
    const plans = externalVideoIoPlans();
    if (!plans) {
      return [];
    }
    return [
      ...plans.inputs.map((plan) => ({
        id: `in-${plan.layer_id}`,
        direction: "IN",
        backend: plan.backend_id.toUpperCase(),
        label: plan.label,
        endpoint: plan.endpoint_name,
        detail: `${videoSourceKindLabel(plan.kind)} / Video Layer ${plan.layer_id} / ${
          plan.issue ?? plan.backend_detail ?? (plan.ready ? "Ready" : "Unavailable")
        }`,
        stateLabel: plan.live ? "Live" : plan.ready ? (plan.enabled ? "Ready" : "Disabled") : "Blocked",
        stateClass: plan.live ? "enabled" : plan.ready ? "available" : "blocked",
      })),
      ...plans.outputs.map((plan) => ({
        id: `out-${plan.output_id}`,
        direction: "OUT",
        backend: plan.backend_id.toUpperCase(),
        label: plan.label,
        endpoint: plan.endpoint_name,
        detail: `${plan.kind} / ${plan.width}x${plan.height} / Comp ${plan.composition_id} / ${Math.round(
          plan.opacity * 100,
        )}%${plan.blackout ? " / Blackout" : ""} / ${
          plan.issue ?? plan.backend_detail ?? (plan.ready ? "Ready" : "Unavailable")
        }`,
        stateLabel: plan.live ? "Live" : plan.ready ? (plan.enabled ? "Ready" : "Disabled") : "Blocked",
        stateClass: plan.live ? "enabled" : plan.ready ? "available" : "blocked",
      })),
    ];
  });
  const externalVideoTransportActiveRows = createMemo(() => {
    const report = externalVideoTransportReport();
    const status = externalVideoTransportStatus();
    if (report || !status) {
      return [];
    }
    return status.active_routes.map((route) => ({
      id: `active-${route.direction}-${route.route_id}-${route.backend_id}-${route.endpoint_name}`,
      direction: route.direction === "Input" ? "IN" : "OUT",
      backend: route.backend_id.toUpperCase(),
      label: route.label,
      endpoint: route.endpoint_name,
      stateLabel: "Active",
      stateClass: "kept",
      detail: `${route.direction} ${route.route_id} / active transport route`,
    }));
  });
  const externalVideoTransportRows = createMemo(() => {
    const report = externalVideoTransportReport();
    if (!report) {
      return [];
    }
    const rowForRoute = (
      stateLabel: string,
      stateClass: string,
      route: ExternalVideoTransportSyncReport["started"][number],
      issue?: string,
    ) => ({
      id: `${stateClass}-${route.direction}-${route.route_id}-${route.backend_id}-${route.endpoint_name}`,
      direction: route.direction === "Input" ? "IN" : "OUT",
      backend: route.backend_id.toUpperCase(),
      label: route.label,
      endpoint: route.endpoint_name,
      stateLabel,
      stateClass,
      detail: `${route.direction} ${route.route_id} / ${issue ?? stateLabel}`,
    });
    return [
      ...report.started.map((route) => rowForRoute("Started", "started", route)),
      ...report.kept.map((route) => rowForRoute("Kept", "kept", route)),
      ...report.stopped.map((route) => rowForRoute("Stopped", "stopped", route)),
      ...report.idle.map((route) => rowForRoute("Idle", "idle", route)),
      ...report.blocked.map((blocked) => rowForRoute("Blocked", "blocked", blocked.route, blocked.issue)),
      ...report.start_failed.map((failed) => rowForRoute("Start Failed", "failed", failed.route, failed.issue)),
      ...report.stop_failed.map((failed) => rowForRoute("Stop Failed", "failed", failed.route, failed.issue)),
    ];
  });
  const externalVideoTransportEventRows = createMemo(() =>
    externalVideoTransportEvents().slice(-6).map((event) => ({
      id: `${event.sequence}-${event.action}-${event.route.direction}-${event.route.route_id}`,
      direction: event.route.direction === "Input" ? "IN" : "OUT",
      backend: event.route.backend_id.toUpperCase(),
      label: event.route.label,
      endpoint: event.route.endpoint_name,
      stateLabel: event.action,
      stateClass: event.action === "Start" ? "driver-start" : "driver-stop",
      detail: event.message,
    })),
  );
  const externalVideoIoPlanClass = (stateClass: string) =>
    `videoBackendPill videoIoPlanPill state-${stateClass}`;
  const externalVideoTransportClass = (stateClass: string) =>
    `videoBackendPill videoTransportPill state-${stateClass}`;
  const projectFileLabel = createMemo(() => {
    const path = currentProjectPath();
    const label = (() => {
      if (!path) {
        return "Untitled.sdc";
      }
      const normalizedPath = path.replaceAll("\\", "/");
      return normalizedPath.split("/").pop() || path;
    })();
    return projectDirty() ? `${label} *` : label;
  });
  const customProfilePreview = createMemo(() => customProfilePreviewFromText(customAttributes()));
  const selectedCustomAttributeIndexValue = createMemo(() => {
    const index = selectedCustomAttributeIndex();
    return index !== null && index >= 0 && index < customAttributeDrafts().length ? index : null;
  });
  const customProfileDraftAnalysis = createMemo(() => {
    const occupied = new Map<number, number>();
    const rowErrors = new Map<number, string[]>();
    const conflictChannels = new Set<number>();
    const rowConflicts = new Set<number>();
    const seenAttributes = new Map<string, number>();
    const controls: Array<{
      attribute: string;
      resolution: AttributeResolution;
      offsets: number[];
      geometry: "Body" | "Head" | "Beam";
      rowIndex: number;
    }> = [];
    let nextOffset = 1;
    let footprint = 0;

    const addRowError = (rowIndex: number, error: string) => {
      rowErrors.set(rowIndex, [...(rowErrors.get(rowIndex) ?? []), error]);
      rowConflicts.add(rowIndex);
    };

    customAttributeDrafts().forEach((draft, rowIndex) => {
      const attribute = draft.attribute.trim();
      if (!attribute) {
        addRowError(rowIndex, "Attribute name is required");
        return;
      }

      const attributeKey = attribute.toLowerCase();
      const duplicateRow = seenAttributes.get(attributeKey);
      if (duplicateRow !== undefined) {
        addRowError(rowIndex, `Duplicate of row ${duplicateRow + 1}`);
        rowConflicts.add(duplicateRow);
        return;
      }
      seenAttributes.set(attributeKey, rowIndex);

      const width = draft.resolution === "SixteenBit" ? 2 : 1;
      const trimmedStart = draft.startOffset.trim();
      const startOffset = trimmedStart.length === 0 ? nextOffset : Number(trimmedStart);
      if (!Number.isInteger(startOffset) || startOffset < 1 || startOffset > 512) {
        addRowError(rowIndex, `Invalid start channel '${trimmedStart || "auto"}'`);
        return;
      }

      const offsets = Array.from({ length: width }, (_, index) => startOffset + index);
      if (offsets.some((offset) => offset < 1 || offset > 512)) {
        addRowError(rowIndex, "Exceeds 512 DMX channels");
        return;
      }

      const overlappingOffsets = offsets.filter((offset) => occupied.has(offset));
      if (overlappingOffsets.length > 0) {
        addRowError(rowIndex, `Overlaps CH ${overlappingOffsets.join("/")}`);
        for (const offset of overlappingOffsets) {
          conflictChannels.add(offset);
          const existingRow = occupied.get(offset);
          if (existingRow !== undefined) {
            rowConflicts.add(existingRow);
          }
        }
        return;
      }

      for (const offset of offsets) {
        occupied.set(offset, rowIndex);
      }
      footprint = Math.max(footprint, ...offsets);
      nextOffset = Math.max(nextOffset, startOffset + width);
      controls.push({
        attribute,
        resolution: draft.resolution,
        offsets,
        geometry: customAttributeGeometryName(attribute),
        rowIndex,
      });
    });

    return {
      controls,
      footprint,
      rowErrors,
      conflictChannels,
      rowConflicts,
    };
  });
  const customProfileDraftRowErrors = (index: number) => customProfileDraftAnalysis().rowErrors.get(index) ?? [];
  const customProfileDraftRowStatusText = (index: number) => {
    const errors = customProfileDraftRowErrors(index);
    if (errors.length > 0) {
      return errors.join("; ");
    }
    if (customProfileDraftAnalysis().rowConflicts.has(index)) {
      return "Conflict";
    }
    return customProfileAttributeDraftChannelLabel(customAttributeDrafts(), index);
  };
  const customProfileDmxCells = createMemo(() => {
    const analysis = customProfileDraftAnalysis();
    const selectedIndex = selectedCustomAttributeIndexValue();
    const cellCount = Math.min(512, Math.max(32, customProfilePreview().footprint, analysis.footprint));
    return Array.from({ length: cellCount }, (_, index) => {
      const channel = index + 1;
      const control = analysis.controls.find((candidate) => candidate.offsets.includes(channel)) ?? null;
      const controlIndex = control?.rowIndex ?? -1;
      return {
        channel,
        control,
        controlIndex,
        selected: selectedIndex !== null && controlIndex === selectedIndex,
        conflict: analysis.conflictChannels.has(channel),
      };
    });
  });
  const hasCueSources = createMemo(
    () => snapshot().fixtures.length > 0 || snapshot().video.layers.length > 0 || snapshot().video.outputs.length > 0,
  );
  const selectedCueList = createMemo<CueListSummary>(() =>
    snapshot().cue_lists.find((cueList) => cueList.id === selectedCueListId())
      ?? snapshot().cue_lists[0]
      ?? { id: 1, label: "Main", active_cue_id: null },
  );
  const selectedCueListCues = createMemo(() =>
    snapshot().cues.filter((cue) => cue.cue_list_id === selectedCueList().id),
  );
  createEffect(() => {
    const selected = selectedCueList();
    if (selected.id !== selectedCueListId()) setSelectedCueListId(selected.id);
    setCueListLabel(selected.label);
  });
  const cueCaptureScopeError = createMemo(() => {
    switch (cueCaptureScope()) {
      case "all":
        return hasCueSources() ? "" : "Patch fixtures or add video layers/outputs first.";
      case "lighting":
        return snapshot().fixtures.length > 0 ? "" : "Patch fixtures before storing a lighting cue.";
      case "selectedFixture":
        return selectedFixture() ? "" : "Select a fixture before storing a selected-fixture cue.";
      case "selectedGroup":
        return selectedFixtureGroupFilter() ? "" : "Select a group before storing a group cue.";
      case "video":
        return snapshot().video.layers.length > 0 || snapshot().video.outputs.length > 0
          ? ""
          : "Add video layers or outputs before storing a video cue.";
    }
  });
  const cueCaptureScopeRequest = (): CueCaptureScopeRequest | null => {
    switch (cueCaptureScope()) {
      case "all":
        return { kind: "all" };
      case "lighting":
        return { kind: "lightingOnly" };
      case "selectedFixture": {
        const fixture = selectedFixture();
        return fixture ? { kind: "selectedFixture", fixtureId: fixture.id } : null;
      }
      case "selectedGroup": {
        const groupId = selectedFixtureGroupFilter();
        return groupId ? { kind: "selectedGroup", groupId } : null;
      }
      case "video":
        return { kind: "videoOnly" };
    }
  };
  const allTimelineVideoAutomationRows = createMemo(() =>
    snapshot().timeline.video_automations.map((automation) => {
      const layer = snapshot().video.layers.find((candidate) => candidate.id === automation.layer_id);
      return {
        ...automation,
        layer_label: layer?.label ?? `Video Layer ${automation.layer_id}`,
      };
    }),
  );
  const timelineVideoAutomationRows = createMemo(() => {
    const rows = allTimelineVideoAutomationRows();
    if (videoAutomationRowScope() === "all") {
      return rows;
    }
    const layerId = selectedVideoAutomationLayerId();
    return layerId === null ? rows : rows.filter((automation) => automation.layer_id === layerId);
  });
  createEffect(() => {
    const selected = selectedTimelineAutomation();
    if (!selected) {
      return;
    }
    const exists = selected.kind === "lighting"
      ? snapshot().timeline.automations.some((automation) => automation.id === selected.automationId)
      : snapshot().timeline.video_automations.some((automation) => automation.id === selected.automationId);
    if (!exists) {
      setSelectedTimelineAutomation(null);
    }
  });
  const activeCue = createMemo(() => {
    const activeCueId = snapshot().active_cue_id;
    return snapshot().cues.find((cue) => cue.id === activeCueId) ?? null;
  });
  const nextCue = createMemo(() => {
    const cues = snapshot().cues;
    if (cues.length === 0) {
      return null;
    }
    const activeCueId = snapshot().active_cue_id;
    const activeIndex = activeCueId === null || activeCueId === undefined
      ? -1
      : cues.findIndex((cue) => cue.id === activeCueId);
    return cues[(activeIndex + 1 + cues.length) % cues.length];
  });
  const cuePadBankCount = createMemo(() => Math.max(1, Math.ceil(snapshot().cues.length / cuePadSize)));
  const cuePadStartIndex = createMemo(() => cuePadBank() * cuePadSize);
  const cuePadRangeLabel = createMemo(() => {
    const cueCount = snapshot().cues.length;
    if (cueCount === 0) {
      return "No cues";
    }
    return `${cuePadStartIndex() + 1}-${Math.min(cuePadStartIndex() + cuePadSize, cueCount)} / ${cueCount}`;
  });
  const activeCueIndex = createMemo(() => {
    const activeCueId = snapshot().active_cue_id;
    if (activeCueId === null || activeCueId === undefined) {
      return -1;
    }
    return snapshot().cues.findIndex((cue) => cue.id === activeCueId);
  });
  const liveCuePads = createMemo(() =>
    Array.from({ length: cuePadSize }, (_, index) => ({
      slot: index === 9 ? "0" : String(index + 1),
      index: cuePadStartIndex() + index,
      cue: snapshot().cues[cuePadStartIndex() + index] ?? null,
    })),
  );
  const enabledDmxOutputCount = createMemo(() => snapshot().dmx_outputs.filter((route) => route.enabled).length);
  const enabledVideoOutputCount = createMemo(() => snapshot().video.outputs.filter((output) => output.enabled).length);
  const activeEffectCount = createMemo(() => snapshot().effects.filter((effect) => effect.enabled).length);
  const touchLayoutClass = createMemo(() =>
    workspaceTab() === "setup"
      ? `layoutSetup setupMode-${setupSubTab()}`
      : workspaceTab() === "touch"
        ? "layoutTouch"
        : `layoutControl controlMode${controlMode()[0].toUpperCase()}${controlMode().slice(1)}`,
  );
  const remoteConfig = createMemo<RemoteControlConfig>(() => ({
    bind_ip: remoteBindIp(),
    port: remotePort(),
    pairing_pin: remotePairingPin(),
    allow_lan: remoteAllowLan(),
    max_connections: remoteMaxConnections(),
    max_message_bytes: remoteMaxMessageBytes(),
    max_messages_per_second: remoteMaxMessagesPerSecond(),
  }));
  const fallbackRemoteUrl = createMemo(() => {
    const host = remoteBindIp().trim();
    const displayHost = host === "" || host === "0.0.0.0" ? "localhost" : host;
    return `http://${displayHost}:${remotePort()}/remote?token=${encodeURIComponent(remotePairingPin())}`;
  });
  const remoteUrls = createMemo(() => {
    const urls = remoteAccessUrls();
    return urls.length > 0 ? urls : [fallbackRemoteUrl()];
  });
  const autoStageWorldBounds = createMemo<StageWorldBounds>(() => {
    const fixtures = snapshot().fixtures;
    const outputPoints = snapshot().video.outputs.flatMap((output) => {
      const size = surfaceWorldHalfSize(output);
      const x = output.mapping.stage_x;
      const z = output.mapping.stage_z;
      return [
        { x: x - size.width, z: z - size.height },
        { x: x + size.width, z: z + size.height },
      ];
    });
    const stageObjectPoints = snapshot().stage_objects.flatMap((object) => {
      const halfWidth = Math.max(0.05, object.width / 2);
      const halfDepth = Math.max(0.05, object.depth / 2);
      return [
        { x: object.x - halfWidth, z: object.z - halfDepth },
        { x: object.x + halfWidth, z: object.z + halfDepth },
      ];
    });
    const points = [
      ...fixtures.map((fixture) => ({ x: fixture.position.x, z: fixture.position.z })),
      ...outputPoints,
      ...stageObjectPoints,
    ];
    const halfX = Math.max(10, ...points.map((point) => Math.abs(point.x))) * 1.1;
    const halfZ = Math.max(10, ...points.map((point) => Math.abs(point.z))) * 1.1;
    return {
      minX: -halfX,
      maxX: halfX,
      minZ: -halfZ,
      maxZ: halfZ,
    };
  });
  const stageWorldBounds = createMemo<StageWorldBounds>(() => {
    const stageMap = snapshot().stage_map;
    if (
      stageMap.locked &&
      Number.isFinite(stageMap.min_x) &&
      Number.isFinite(stageMap.max_x) &&
      Number.isFinite(stageMap.min_z) &&
      Number.isFinite(stageMap.max_z) &&
      stageMap.max_x > stageMap.min_x &&
      stageMap.max_z > stageMap.min_z
    ) {
      return {
        minX: stageMap.min_x,
        maxX: stageMap.max_x,
        minZ: stageMap.min_z,
        maxZ: stageMap.max_z,
      };
    }
    return autoStageWorldBounds();
  });
  const stageOrigin2d = createMemo(() => stageWorldToSvgPoint(0, 0, stageWorldBounds()));
  const cueCapturePreview = createMemo(() => {
    const current = snapshot();
    const scope = cueCaptureScope();
    const currentValues = faderValues();
    const bounds = stageWorldBounds();
    const selectedGroupId = selectedFixtureGroupFilter();
    const fixtures = (() => {
      if (scope === "video") {
        return [];
      }
      if (scope === "selectedFixture") {
        const fixture = selectedFixture();
        return fixture ? [fixture] : [];
      }
      if (scope === "selectedGroup") {
        return selectedGroupId
          ? current.fixtures.filter((fixture) => fixture.group_ids.includes(selectedGroupId))
          : [];
      }
      return current.fixtures;
    })();
    const includeVideo = scope === "all" || scope === "video";
    const layers = includeVideo ? current.video.layers : [];
    const outputs = includeVideo ? current.video.outputs : [];
    const nodeGraphs = includeVideo ? current.node_graphs : [];
    const scopeLabel =
      scope === "all"
        ? "Lighting + Video"
        : scope === "lighting"
          ? "Lighting Only"
          : scope === "selectedFixture"
            ? "Selected Fixture"
            : scope === "selectedGroup"
              ? "Selected Group"
              : "Video Only";
    const scopeDetail =
      scope === "selectedFixture"
        ? selectedFixture()?.label ?? "No fixture selected"
        : scope === "selectedGroup"
          ? selectedGroupId ?? "No group selected"
          : scopeLabel;
    const previewFixtures = fixtures.map((fixture) => {
      const point = stageWorldToSvgPoint(fixture.position.x, fixture.position.z, bounds);
      const dimmer = readFixtureAttribute(fixture, currentValues, ["Dimmer", "Intensity"]) ?? 0;
      const red = readFixtureAttribute(fixture, currentValues, ["ColorRed", "Red"]);
      const green = readFixtureAttribute(fixture, currentValues, ["ColorGreen", "Green"]);
      const blue = readFixtureAttribute(fixture, currentValues, ["ColorBlue", "Blue"]);
      const color =
        red !== undefined || green !== undefined || blue !== undefined
          ? `#${valueToHexByte(clampDmxValue(red ?? 0))}${valueToHexByte(clampDmxValue(green ?? 0))}${valueToHexByte(clampDmxValue(blue ?? 0))}`
          : "#58a7f6";
      return {
        id: fixture.id,
        label: fixture.label,
        dmxLabel: `U${fixture.universe} A${fixture.address}`,
        groupLabel: fixture.group_ids.length > 0 ? fixture.group_ids.join(", ") : "No group",
        x: point.x,
        z: point.z,
        color,
        intensity: dmxValueToPercent(dimmer),
        attributes: fixture.attribute_values.length,
      };
    });
    const videoRows = [
      ...layers.map((layer) => ({
        label: layer.label,
        meta: `Layer / ${layer.blend_mode} / ${Math.round(layer.state.opacity * 100)}%`,
      })),
      ...outputs.map((output) => ({
        label: output.label,
        meta: `Output / ${output.kind} / ${Math.round(output.opacity * 100)}%`,
      })),
      ...nodeGraphs.map((graph) => ({
        label: graph.label,
        meta: graph.enabled ? "Node graph enabled" : "Node graph disabled",
      })),
    ];
    return {
      scopeLabel,
      scopeDetail,
      fixtures: previewFixtures,
      videoRows,
      layerCount: layers.length,
      outputCount: outputs.length,
      nodeGraphCount: nodeGraphs.length,
      fixtureAttributeCount: fixtures.reduce((total, fixture) => total + fixture.attribute_values.length, 0),
    };
  });
  const {
    normalizedMappingViewportZoom,
    mappingViewportBox,
    mappingStageViewBox,
    mappingStageCursorSvgPoint,
    mappingStageCursorLabel,
    mappingPlacePreview,
    mappingViewportZoomLabel,
    setMappingViewport,
    zoomMappingViewport,
    zoomMappingViewportAtPoint,
    resetMappingViewport,
    normalizedMappingSnapSize,
    snapStageCoordinate,
    snapStagePoint,
    snapStageLength,
    snapStagePosition,
    mappingSnapLines,
    mappingMarqueeBox,
  } = createMappingViewportModel({
    viewportZoom: mappingViewportZoom,
    viewportCenterX: mappingViewportCenterX,
    viewportCenterZ: mappingViewportCenterZ,
    setViewportZoom: setMappingViewportZoom,
    setViewportCenterX: setMappingViewportCenterX,
    setViewportCenterZ: setMappingViewportCenterZ,
    stageCursorWorld: mappingStageCursorWorld,
    stageWorldBounds,
    selectedFixture,
    stageTool: mappingStageTool,
    snapEnabled: mappingSnapEnabled,
    snapSize: mappingSnapSize,
    marquee: mappingMarquee,
    onStatus: setMessage,
  });
  const mappingViewPresetFromCurrent = (label: string, existingId?: string): MappingViewPreset => ({
    id: existingId ?? `view-${Date.now().toString(36)}`,
    label,
    viewportZoom: Number(normalizedMappingViewportZoom().toFixed(3)),
    viewportCenterX: Number(mappingViewportCenterX().toFixed(3)),
    viewportCenterZ: Number(mappingViewportCenterZ().toFixed(3)),
    snapEnabled: mappingSnapEnabled(),
    snapSize: Number(normalizedMappingSnapSize().toFixed(3)),
    showLabels: mappingShowLabels(),
    showBeams: mappingShowBeams(),
    showGeometry: mappingShowGeometry(),
    showProjectors: mappingShowProjectors(),
    showStageObjects: mappingShowStageObjects(),
    showLevels: mappingShowLevels(),
    stageTool: mappingStageTool(),
  });
  const saveMappingViewPreset = () => {
    const label = mappingViewPresetLabel().trim();
    if (!label) {
      setMessage("Mapping view preset label is required.");
      return;
    }
    const existing = mappingViewPresets().find((preset) => preset.label.toLowerCase() === label.toLowerCase());
    const preset = mappingViewPresetFromCurrent(label.slice(0, 28), existing?.id);
    const nextPresets = [
      ...mappingViewPresets().filter((candidate) => candidate.id !== preset.id),
      preset,
    ].slice(-18);
    setMappingViewPresets(nextPresets);
    saveMappingViewPresets(nextPresets);
    setSelectedMappingViewPresetId(preset.id);
    setMappingViewPresetLabel(preset.label);
    setMessage(`Saved mapping view preset ${preset.label} (${mappingViewPresetObjectLabel(preset)}).`);
  };
  const applyMappingViewPreset = (id: string) => {
    const preset = mappingViewPresets().find((candidate) => candidate.id === id);
    if (!preset) {
      setMessage("Select a mapping view preset first.");
      return;
    }
    setMappingViewport(preset.viewportZoom, preset.viewportCenterX, preset.viewportCenterZ);
    setMappingSnapEnabled(preset.snapEnabled);
    setMappingSnapSize(preset.snapSize);
    setMappingShowLabels(preset.showLabels);
    setMappingShowBeams(preset.showBeams);
    setMappingShowGeometry(preset.showGeometry);
    setMappingShowProjectors(preset.showProjectors);
    setMappingShowStageObjects(preset.showStageObjects);
    setMappingShowLevels(preset.showLevels);
    setMappingStageTool(preset.stageTool);
    setSelectedMappingViewPresetId(preset.id);
    setMappingViewPresetLabel(preset.label);
    setMessage(`Applied mapping view preset ${preset.label} (${mappingViewPresetObjectLabel(preset)}).`);
  };
  const removeMappingViewPreset = (id: string) => {
    const preset = mappingViewPresets().find((candidate) => candidate.id === id);
    if (!preset) {
      return;
    }
    const nextPresets = mappingViewPresets().filter((candidate) => candidate.id !== id);
    setMappingViewPresets(nextPresets);
    saveMappingViewPresets(nextPresets);
    if (selectedMappingViewPresetId() === id) {
      setSelectedMappingViewPresetId(nextPresets[0]?.id ?? "");
    }
    setMessage(`Removed mapping view preset ${preset.label}.`);
  };
  const exportMappingStageSvg = () => {
    if (!mappingStageSvgElement) {
      setMessage("2D mapping stage is not ready for SVG export.");
      return;
    }

    const clone = mappingStageSvgElement.cloneNode(true) as SVGSVGElement;
    inlineComputedSvgStyles(mappingStageSvgElement, clone);
    clone.querySelectorAll(standaloneSvgExportSelectorsToRemove).forEach((element) => element.remove());
    clone.classList.remove("dragging", "placeMode", "rotateMode", "panMode", "selectMode");
    clone.setAttribute("xmlns", "http://www.w3.org/2000/svg");
    clone.setAttribute("width", "1600");
    clone.setAttribute("height", "1600");
    clone.setAttribute("data-syndocal-export", "stage-plot-v1");
    clone.setAttribute("data-syndocal-project", projectFileLabel().replace(/\s+\*$/, ""));
    clone.setAttribute("data-syndocal-view", mappingStageViewBox());

    const namespace = "http://www.w3.org/2000/svg";
    const title = document.createElementNS(namespace, "title");
    title.textContent = `Syndocal 2D Stage Plot - ${projectFileLabel().replace(/\s+\*$/, "")}`;
    const description = document.createElementNS(namespace, "desc");
    description.textContent = [
      `Exported ${new Date().toISOString()}`,
      `${mappingFilteredFixtures().length} fixture(s)`,
      `${snapshot().video.outputs.length} projection surface(s)`,
      `${snapshot().stage_objects.length} stage object(s)`,
      `viewBox ${mappingStageViewBox()}`,
    ].join(" / ");
    clone.insertBefore(description, clone.firstChild);
    clone.insertBefore(title, clone.firstChild);

    const serializer = new XMLSerializer();
    const svgText = `<?xml version="1.0" encoding="UTF-8"?>\n${serializer.serializeToString(clone)}\n`;
    const fileName = `${safeExportFileNamePart(projectFileLabel().replace(/\s+\*$/, ""))}-2d-stage.svg`;
    downloadTextFile(fileName, svgText, "image/svg+xml;charset=utf-8");
    setMessage(`Exported 2D stage SVG ${fileName}.`);
  };
  const exportVisualizerRenderPayload = async () => {
    try {
      const payload = await invoke<VisualizerRenderPayload>("get_visualizer_render_payload", { config: null });
      const projectLabel = projectFileLabel().replace(/\s+\*$/, "");
      const envelope = {
        version: 1,
        software: "Syndocal",
        kind: "visualizer-render-payload",
        project: projectLabel,
        exported_at: new Date().toISOString(),
        payload,
      };
      const jsonText = `${JSON.stringify(envelope, null, 2)}\n`;
      const fileName = `${safeExportFileNamePart(projectLabel)}-visualizer-scene.json`;
      downloadTextFile(fileName, jsonText, "application/json;charset=utf-8");
      setMessage(
        `Exported visualizer scene JSON ${fileName} (${payload.scene.fixtures.length} fixture(s), ${payload.scene.video_surfaces.length} projection surface(s)).`,
      );
    } catch (error) {
      setMessage(String(error));
    }
  };
  const {
    dragWorldDelta,
    mappingOutputHandleAngleDeg,
    mappingOutputHandleDistance,
    mappingVideoSurfaceCornerLocals,
    mappingVideoSurfaceCornerPointList,
    mappingWorldToStageObjectLocal,
    mappingVideoOutputPreviewMapping,
    mappingFixturePosition,
    mappingFixtureYawFromPoint,
    mappingFixtureYaw,
    mappingVideoOutputMapping,
    mappingStageObjectPreview,
    isMappingStageObjectDrag,
    isDraggingMappingFixture,
    isDraggingMappingVideoOutput,
    isDraggingMappingStageObject,
    mappingGeometryNodes2d,
    visualizerFixtures,
    visualizerVideoSurfaces2d,
    visualizerStageObjects2d,
  } = createMappingRenderModel({
    mappingDrag,
    mappingShowGeometry,
    mappingFilteredFixtures,
    stageWorldBounds,
    selectedMappingFixtureIdSet,
    selectedFixtureGroupFilter,
    selectedFixtureId,
    faderValues,
    snapshot,
    selectedStageObjectId,
    snapStagePoint,
    snapStagePosition,
    snapStageLength,
  });

  const waveOriginSvgPoint = createMemo(() => stageWorldToSvgPoint(waveOriginX(), waveOriginZ(), stageWorldBounds()));
  const waveDirectionLength = createMemo(() => {
    const x = waveDirectionX();
    const y = waveDirectionY();
    const z = waveDirectionZ();
    return Math.sqrt(x * x + y * y + z * z);
  });
  const waveDirectionIsRadial = createMemo(() => waveDirectionLength() <= Number.EPSILON);
  const waveDirectionSvgPoint = createMemo(() => {
    const bounds = stageWorldBounds();
    const length = waveDirectionLength();
    if (length <= Number.EPSILON) {
      return waveOriginSvgPoint();
    }
    const scale = Math.min(bounds.maxX - bounds.minX, bounds.maxZ - bounds.minZ) * 0.18;
    return stageWorldToSvgPoint(
      waveOriginX() + (waveDirectionX() / length) * scale,
      waveOriginZ() + (waveDirectionZ() / length) * scale,
      bounds,
    );
  });
  const waveRadialRadius = createMemo(() => {
    const bounds = stageWorldBounds();
    const edge = stageWorldToSvgPoint(waveOriginX() + Math.min(bounds.maxX - bounds.minX, bounds.maxZ - bounds.minZ) * 0.16, waveOriginZ(), bounds);
    return Math.max(4, Math.abs(edge.x - waveOriginSvgPoint().x));
  });
  const waveTargetFixtureIds = createMemo(() => {
    const mode = effectTargetMode();
    if (mode === "fixture") {
      const fixture = selectedFixture();
      return new Set(fixture ? [fixture.id] : []);
    }
    if (mode === "selection") {
      return selectedMappingFixtureIdSet();
    }
    if (mode === "group") {
      const groups = parseGroupIds(effectTargetGroups());
      return new Set(
        snapshot()
          .fixtures.filter((fixture) => groups.some((groupId) => fixture.group_ids.includes(groupId)))
          .map((fixture) => fixture.id),
      );
    }
    return new Set<number>();
  });

  const selectedMappingVideoOutput = createMemo(() => {
    const outputId = selectedVideoOutputId();
    if (outputId === null) {
      return snapshot().video.outputs[0] ?? null;
    }
    return snapshot().video.outputs.find((output) => output.id === outputId) ?? snapshot().video.outputs[0] ?? null;
  });
  const selectedTouchVideoOutput = createMemo(() => {
    const outputId = selectedVideoOutputId();
    return outputId === null ? null : snapshot().video.outputs.find((output) => output.id === outputId) ?? null;
  });
  const selectedStageObject = createMemo(() => {
    const objectId = selectedStageObjectId();
    return objectId === null ? null : snapshot().stage_objects.find((object) => object.id === objectId) ?? null;
  });
  const fixtureSvgBounds = (fixture: VisualizerFixture): MappingSvgBounds => {
    const halfWidth = Math.max(2.4, fixture.width / 2 + 1.6);
    const halfHeight = Math.max(2.4, fixture.height / 2 + 1.6);
    return {
      minX: fixture.x - halfWidth,
      maxX: fixture.x + halfWidth,
      minZ: fixture.z - halfHeight,
      maxZ: fixture.z + halfHeight,
    };
  };
  const videoSurfaceSvgBounds = (surface: VisualizerVideoSurface2d): MappingSvgBounds => {
    const angle = (surface.rotationDeg * Math.PI) / 180;
    const cos = Math.cos(angle);
    const sin = Math.sin(angle);
    const output = snapshot().video.outputs.find((candidate) => candidate.id === surface.id);
    const mapping = output ? mappingVideoOutputMapping(output) : defaultVideoOutputMapping;
    const localCorners = output
      ? mappingVideoSurfaceCornerLocals(surface, mapping).map((corner) => ({ x: corner.x, z: corner.z }))
      : [
          { x: -surface.width / 2, z: -surface.height / 2 },
          { x: surface.width / 2, z: -surface.height / 2 },
          { x: surface.width / 2, z: surface.height / 2 },
          { x: -surface.width / 2, z: surface.height / 2 },
        ];
    const corners = localCorners.map((corner) => ({
      x: surface.x + corner.x * cos - corner.z * sin,
      z: surface.z + corner.x * sin + corner.z * cos,
    }));
    return {
      minX: Math.min(...corners.map((corner) => corner.x)) - 2,
      maxX: Math.max(...corners.map((corner) => corner.x)) + 2,
      minZ: Math.min(...corners.map((corner) => corner.z)) - 2,
      maxZ: Math.max(...corners.map((corner) => corner.z)) + 2,
    };
  };
  const stageObjectSvgBounds = (object: VisualizerStageObject2d): MappingSvgBounds => {
    const angle = (object.rotationDeg * Math.PI) / 180;
    const cos = Math.cos(angle);
    const sin = Math.sin(angle);
    const corners = [
      { x: -object.width / 2, z: -object.depth / 2 },
      { x: object.width / 2, z: -object.depth / 2 },
      { x: object.width / 2, z: object.depth / 2 },
      { x: -object.width / 2, z: object.depth / 2 },
    ].map((corner) => ({
      x: object.x + corner.x * cos - corner.z * sin,
      z: object.z + corner.x * sin + corner.z * cos,
    }));
    return {
      minX: Math.min(...corners.map((corner) => corner.x)) - 2,
      maxX: Math.max(...corners.map((corner) => corner.x)) + 2,
      minZ: Math.min(...corners.map((corner) => corner.z)) - 2,
      maxZ: Math.max(...corners.map((corner) => corner.z)) + 2,
    };
  };
  const mergeSvgBounds = (bounds: MappingSvgBounds[]) => {
    if (bounds.length === 0) {
      return null;
    }
    return {
      minX: Math.min(...bounds.map((bound) => bound.minX)),
      maxX: Math.max(...bounds.map((bound) => bound.maxX)),
      minZ: Math.min(...bounds.map((bound) => bound.minZ)),
      maxZ: Math.max(...bounds.map((bound) => bound.maxZ)),
    };
  };
  const fitMappingViewportToSvgBounds = (bounds: MappingSvgBounds | null, label: string) => {
    if (!bounds) {
      setMessage(`Nothing to fit for ${label}.`);
      return;
    }
    const width = Math.max(1, bounds.maxX - bounds.minX);
    const height = Math.max(1, bounds.maxZ - bounds.minZ);
    const size = clampRange(Math.max(width, height) * 1.28, stageViewBoxSize / 4, stageViewBoxSize);
    const zoom = stageViewBoxSize / size;
    setMappingViewport(zoom, (bounds.minX + bounds.maxX) / 2, (bounds.minZ + bounds.maxZ) / 2);
    setMessage(`Fit 2D mapping viewport to ${label}.`);
  };
  const fitMappingViewportToVisible = () => {
    const fixtureBounds = visualizerFixtures().map(fixtureSvgBounds);
    const surfaceBounds = visualizerVideoSurfaces2d().map(videoSurfaceSvgBounds);
    const objectBounds = visualizerStageObjects2d().map(stageObjectSvgBounds);
    fitMappingViewportToSvgBounds(mergeSvgBounds([...fixtureBounds, ...surfaceBounds, ...objectBounds]), "visible stage items");
  };
  const compactMappingStageViewBox = createMemo(() => {
    const bounds = mergeSvgBounds([
      ...visualizerFixtures().map(fixtureSvgBounds),
      ...visualizerVideoSurfaces2d().map(videoSurfaceSvgBounds),
      ...visualizerStageObjects2d().map(stageObjectSvgBounds),
    ]);
    if (!bounds) {
      return `0 0 ${stageViewBoxSize} ${stageViewBoxSize}`;
    }
    const width = Math.max(1, bounds.maxX - bounds.minX);
    const height = Math.max(1, bounds.maxZ - bounds.minZ);
    const size = clampRange(Math.max(width, height) * 1.18, 18, stageViewBoxSize);
    const centerX = (bounds.minX + bounds.maxX) / 2;
    const centerZ = (bounds.minZ + bounds.maxZ) / 2;
    const x = clampRange(centerX - size / 2, 0, stageViewBoxSize - size);
    const z = clampRange(centerZ - size / 2, 0, stageViewBoxSize - size);
    return `${x} ${z} ${size} ${size}`;
  });
  const fitMappingViewportToSelection = () => {
    const selectedIds = selectedMappingFixtureIdSet();
    const fixtureBounds = visualizerFixtures()
      .filter((fixture) => selectedIds.has(fixture.id))
      .map(fixtureSvgBounds);
    const surfaceBounds = selectedVideoOutputId() === null
      ? []
      : visualizerVideoSurfaces2d()
          .filter((surface) => surface.id === selectedVideoOutputId())
          .map(videoSurfaceSvgBounds);
    const objectBounds =
      selectedStageObjectId() === null
        ? []
        : visualizerStageObjects2d()
            .filter((object) => object.id === selectedStageObjectId())
            .map(stageObjectSvgBounds);
    fitMappingViewportToSvgBounds(mergeSvgBounds([...fixtureBounds, ...surfaceBounds, ...objectBounds]), "selection");
  };
  const canFitMappingViewportToVisible = createMemo(
    () => visualizerFixtures().length > 0 || visualizerVideoSurfaces2d().length > 0 || visualizerStageObjects2d().length > 0,
  );
  const canFitMappingViewportToSelection = createMemo(
    () => selectedMappingFixtures().length > 0 || selectedVideoOutputId() !== null || selectedStageObjectId() !== null,
  );
  const effectVideoTargetPosition = () => ({
    x: effectVideoPositionX(),
    y: effectVideoPositionY(),
    z: effectVideoPositionZ(),
  });
  const effectVideoTargetSvgPoint = createMemo(() =>
    stageWorldToSvgPoint(effectVideoPositionX(), effectVideoPositionZ(), stageWorldBounds()),
  );
  const setEffectVideoPosition = (position: { x: number; y: number; z: number }, nextMessage?: string) => {
    setEffectVideoPositionX(Number(position.x.toFixed(2)));
    setEffectVideoPositionY(Number(position.y.toFixed(2)));
    setEffectVideoPositionZ(Number(position.z.toFixed(2)));
    if (nextMessage) {
      setMessage(nextMessage);
    }
  };
  const setEffectVideoPositionFromStagePoint = (point: { x: number; z: number }) => {
    setEffectVideoPosition({ x: point.x, y: effectVideoPositionY(), z: point.z });
  };
  const setEffectVideoPositionFromVideoOutput = (output: VideoOutputSummary, nextMessage?: string) => {
    const mapping = mappingVideoOutputMapping(output);
    setSelectedVideoOutputId(output.id);
    setEffectVideoPosition(
      {
        x: mapping.stage_x,
        y: 0,
        z: mapping.stage_z,
      },
      nextMessage ?? `Video effect target uses ${output.label} stage position.`,
    );
  };
  const setEffectVideoPositionFromSelectedOutput = () => {
    const output = selectedMappingVideoOutput();
    if (!output) {
      setMessage("Add or select a video output before using its stage position.");
      return;
    }
    setEffectVideoPositionFromVideoOutput(output);
  };
  const setEffectVideoPositionFromWaveOrigin = () => {
    setEffectVideoPosition(
      {
        x: waveOriginX(),
        y: waveOriginY(),
        z: waveOriginZ(),
      },
      "Video effect target uses the current wave origin.",
    );
  };
  const setEffectVideoPositionFromStageCenter = () => {
    const bounds = stageWorldBounds();
    setEffectVideoPosition(
      {
        x: (bounds.minX + bounds.maxX) / 2,
        y: 0,
        z: (bounds.minZ + bounds.maxZ) / 2,
      },
      "Video effect target uses stage center.",
    );
  };
  const setEffectVideoPositionFromSelectedStageObject = () => {
    const object = selectedStageObject();
    if (!object) {
      setMessage("Select a stage object before using it as the video target position.");
      return;
    }
    setEffectVideoPosition(
      {
        x: object.x,
        y: 0,
        z: object.z,
      },
      `Video effect target uses ${object.label}.`,
    );
  };

  const parseGroupIds = (value: string) =>
    value
      .split(",")
      .map((group) => group.trim())
      .filter((group) => group.length > 0);

  const uniqueGroupIds = (groupIds: string[]) => [...new Set(groupIds)];

  const registerSetupPanel = (tabs: SetupSubTab[]) => (element: HTMLElement) => {
    for (const tab of tabs) {
      setupPanelRefs[tab] = element;
    }
  };

  const selectSetupMode = (tab: SetupSubTab) => {
    setSetupSubTab(tab);
    requestAnimationFrame(() => {
      const panel = setupPanelRefs[tab];
      if (!panel) {
        return;
      }
      panel.focus({ preventScroll: true });
    });
  };

  const setupPanelClass = (baseClass: string, tabs: SetupSubTab[]) =>
    `${baseClass} ${tabs.includes(setupSubTab()) ? "setupPanelFocus" : ""}`;

  const activateFixture = (fixture: PatchedFixtureSummary) => {
    setSelectedFixtureId(fixture.id);
    setSelectedFixtureLabelDraft(fixture.label);
    setSelectedFixtureUniverseDraft(fixture.universe);
    setSelectedFixtureAddressDraft(fixture.address);
    setSelectedFixtureGroupText(fixture.group_ids.join(", "));
    setSelectedFixtureLimitsDraft(fixture.limits ?? defaultFixtureLimits);
  };

  const selectFixture = (fixture: PatchedFixtureSummary) => {
    activateFixture(fixture);
    setSelectedMappingFixtureIds([fixture.id]);
  };

  const isAdditiveMappingSelectionEvent = (event: Pick<MouseEvent, "ctrlKey" | "metaKey" | "shiftKey">) =>
    event.ctrlKey || event.metaKey || event.shiftKey;

  const selectMappingFixture = (
    fixture: PatchedFixtureSummary,
    event?: Pick<MouseEvent, "ctrlKey" | "metaKey" | "shiftKey">,
  ) => {
    if (!event || !isAdditiveMappingSelectionEvent(event)) {
      selectFixture(fixture);
      return;
    }

    const currentIds = selectedMappingFixtureIds();
    if (currentIds.includes(fixture.id)) {
      if (currentIds.length === 1) {
        activateFixture(fixture);
        setSelectedMappingFixtureIds([fixture.id]);
        return;
      }
      const nextIds = currentIds.filter((id) => id !== fixture.id);
      const nextActiveFixture =
        selectedFixtureId() === fixture.id
          ? snapshot().fixtures.find((candidate) => candidate.id === nextIds[0])
          : fixture;
      if (nextActiveFixture) {
        activateFixture(nextActiveFixture);
      }
      setSelectedMappingFixtureIds(nextIds);
      return;
    }

    activateFixture(fixture);
    setSelectedMappingFixtureIds([...currentIds, fixture.id]);
  };

  const pickVisibleMappingFixtures = () => {
    const fixtures = mappingFilteredFixtures();
    if (fixtures.length === 0) {
      setSelectedMappingFixtureIds([]);
      setMessage("No visible fixtures to pick in 2D mapping.");
      return;
    }
    activateFixture(fixtures[0]);
    setSelectedMappingFixtureIds(fixtures.map((fixture) => fixture.id));
    setMessage(`Picked ${fixtures.length} visible fixture${fixtures.length === 1 ? "" : "s"} in 2D mapping.`);
  };

  const clearMappingFixtureSelection = () => {
    const count = selectedMappingFixtureIds().length;
    setSelectedMappingFixtureIds([]);
    setMessage(count > 0 ? `Cleared ${count} mapped fixture pick${count === 1 ? "" : "s"}.` : "No mapped fixtures are picked.");
  };

  const prepareMappingSelectionEffectTarget = () => {
    const fixtures = selectedMappingFixtures();
    if (fixtures.length === 0) {
      setMessage("Select one or more fixtures on the 2D mapping stage first.");
      return null;
    }
    const activeFixture = selectedFixture();
    if (!activeFixture || !fixtures.some((fixture) => fixture.id === activeFixture.id)) {
      activateFixture(fixtures[0]);
    }
    const center = mappingFixtureSelectionCenter(fixtures);
    const averageY = fixtures.reduce((sum, fixture) => sum + fixture.position.y, 0) / fixtures.length;
    setWaveOriginX(Number(center.x.toFixed(3)));
    setWaveOriginY(Number(averageY.toFixed(3)));
    setWaveOriginZ(Number(center.z.toFixed(3)));
    setEffectTargetMode("selection");
    setWorkspaceTab("control");
    setControlMode("edit");
    setEditDeskSurface("effects");
    return { center, averageY };
  };

  const useMappingSelectionAsEffectTarget = () => {
    const fixtures = selectedMappingFixtures();
    if (!prepareMappingSelectionEffectTarget()) {
      return;
    }
    setMessage(`Using ${fixtures.length} mapped fixture${fixtures.length === 1 ? "" : "s"} as the effect target.`);
  };

  const useMappingSelectionAsWaveEffectTarget = () => {
    const fixtures = selectedMappingFixtures();
    if (!prepareMappingSelectionEffectTarget()) {
      return;
    }
    const minX = Math.min(...fixtures.map((fixture) => fixture.position.x));
    const maxX = Math.max(...fixtures.map((fixture) => fixture.position.x));
    const minZ = Math.min(...fixtures.map((fixture) => fixture.position.z));
    const maxZ = Math.max(...fixtures.map((fixture) => fixture.position.z));
    const width = Math.abs(maxX - minX);
    const depth = Math.abs(maxZ - minZ);
    const span = Math.max(width, depth);
    const controls = commonAttributeControls(fixtures);
    const preferredAttribute =
      controls.find((control) => ["dimmer", "intensity", "masterintensity"].includes(control.attribute.toLowerCase())) ??
      controls[0];
    if (preferredAttribute) {
      setEffectAttribute(preferredAttribute.attribute);
    }
    setEffectType("PositionWave");
    setEffectShape("Sine");
    setEffectBlendMode("Override");
    setEffectLow(0);
    setEffectHigh(65_535);
    setEffectPhase(0);
    setWaveSpeed(1);
    setWaveWavelength(Number(clampRange(span > 0 ? span / 2 : 2, 1, 8).toFixed(2)));
    setWaveDirectionPreset(width >= depth ? 1 : 0, 0, depth > width ? 1 : 0);
    setEffectClockSyncPreset(1);
    setMessage(
      preferredAttribute
        ? `Prepared a position wave draft for ${fixtures.length} mapped fixture${fixtures.length === 1 ? "" : "s"} on ${preferredAttribute.attribute}.`
        : `Prepared a position wave draft for ${fixtures.length} mapped fixture${fixtures.length === 1 ? "" : "s"}, but no common light attribute was found.`,
    );
  };

  const handleDmxAddressCellClick = (cell: DmxAddressCell) => {
    setUniverse(activePatchGridUniverse());
    setAddress(cell.segment?.start ?? cell.channel);
    if (cell.segment) {
      selectFixture(cell.segment.fixture);
      setMessage(`Selected ${cell.segment.fixture.label} at U${activePatchGridUniverse()} A${cell.segment.start}-${cell.segment.end}.`);
      return;
    }
    setMessage(`Patch start set to U${activePatchGridUniverse()} A${cell.channel}.`);
  };

  const selectNextFreePatchAddress = () => {
    const nextAddress = nextFreePatchAddress();
    if (nextAddress === null) {
      setMessage(`No free ${selectedFootprint()}ch range for ${patchCountValue()} fixture(s) in universe ${universe()}`);
      return;
    }
    setAddress(nextAddress);
    setPatchGridUniverse(universe());
    setDmxPatchViewMode("grid");
  };

  const selectFixtureGroupFilter = (groupId: string | null) => {
    setSelectedFixtureGroupFilter(groupId);
    if (!groupId) {
      return;
    }
    setEffectTargetMode("group");
    setEffectTargetGroups(groupId);
    const firstFixture = snapshot().fixtures.find((fixture) => fixture.group_ids.includes(groupId));
    if (firstFixture) {
      selectFixture(firstFixture);
    }
  };

  const toggleEffectTargetGroup = (groupId: string) => {
    const currentGroups = parseGroupIds(effectTargetGroups());
    const nextGroups = currentGroups.includes(groupId)
      ? currentGroups.filter((candidate) => candidate !== groupId)
      : [...currentGroups, groupId];
    setEffectTargetMode("group");
    setEffectTargetGroups(nextGroups.join(", "));
  };

  const snapshotFaderValues = (next: EngineSnapshot) => {
    const values: Record<string, number> = {};
    for (const fixture of next.fixtures) {
      for (const attributeValue of fixture.attribute_values) {
        values[`${fixture.id}:${attributeValue.attribute}`] = attributeValue.value;
      }
    }
    return values;
  };

  const syncVideoOutputConfigDrafts = (outputs: VideoOutputSummary[]) => {
    setVideoOutputConfigDrafts((current) => {
      const nextDrafts: Record<number, VideoOutputConfigDraft> = {};
      for (const output of outputs) {
        nextDrafts[output.id] = current[output.id] ?? videoOutputConfigDraftFromSummary(output);
      }
      return nextDrafts;
    });
  };

  const videoOutputConfigDraft = (output: VideoOutputSummary) =>
    videoOutputConfigDrafts()[output.id] ?? videoOutputConfigDraftFromSummary(output);

  const updateVideoOutputConfigDraft = (
    output: VideoOutputSummary,
    patch: Partial<VideoOutputConfigDraft>,
  ) => {
    setVideoOutputConfigDrafts((current) => ({
      ...current,
      [output.id]: {
        ...(current[output.id] ?? videoOutputConfigDraftFromSummary(output)),
        ...patch,
      },
    }));
  };

  const syncCueMetadataDrafts = (cues: CueSummary[]) => {
    setCueMetadataDrafts((current) => {
      const nextDrafts: Record<number, CueMetadataDraft> = {};
      for (const cue of cues) {
        nextDrafts[cue.id] = current[cue.id] ?? cueMetadataDraftFromSummary(cue);
      }
      return nextDrafts;
    });
  };

  const cueMetadataDraft = (cue: CueSummary) =>
    cueMetadataDrafts()[cue.id] ?? cueMetadataDraftFromSummary(cue);

  const updateCueMetadataDraft = (cue: CueSummary, patch: Partial<CueMetadataDraft>) => {
    setCueMetadataDrafts((current) => ({
      ...current,
      [cue.id]: {
        ...(current[cue.id] ?? cueMetadataDraftFromSummary(cue)),
        ...patch,
      },
    }));
  };

  const syncTimelineEventDrafts = (events: TimelineCueEventSummary[]) => {
    setTimelineEventDrafts((current) => {
      const nextDrafts: Record<number, TimelineEventDraft> = {};
      for (const event of events) {
        nextDrafts[event.id] = current[event.id] ?? timelineEventDraftFromSummary(event);
      }
      return nextDrafts;
    });
  };

  const timelineEventDraft = (event: TimelineCueEventSummary) =>
    timelineEventDrafts()[event.id] ?? timelineEventDraftFromSummary(event);

  const updateTimelineEventDraft = (event: TimelineCueEventSummary, patch: Partial<TimelineEventDraft>) => {
    setTimelineEventDrafts((current) => ({
      ...current,
      [event.id]: {
        ...(current[event.id] ?? timelineEventDraftFromSummary(event)),
        ...patch,
      },
    }));
  };

  const syncTimelineAutomationDrafts = (automations: TimelineAutomationSummary[]) => {
    setTimelineAutomationDrafts((current) => {
      const nextDrafts: Record<number, TimelineAutomationDraft> = {};
      for (const automation of automations) {
        nextDrafts[automation.id] = current[automation.id] ?? timelineAutomationDraftFromSummary(automation);
      }
      return nextDrafts;
    });
  };

  const timelineAutomationDraft = (automation: TimelineAutomationSummary) =>
    timelineAutomationDrafts()[automation.id] ?? timelineAutomationDraftFromSummary(automation);

  const updateTimelineAutomationDraft = (
    automation: TimelineAutomationSummary,
    patch: Partial<TimelineAutomationDraft>,
  ) => {
    setTimelineAutomationDrafts((current) => {
      const currentDraft = current[automation.id] ?? timelineAutomationDraftFromSummary(automation);
      const nextDraft = { ...currentDraft, ...patch };
      if (patch.fixture_id !== undefined) {
        const attributes = fixtureAttributeOptions(patch.fixture_id);
        if (!attributes.includes(nextDraft.attribute)) {
          nextDraft.attribute = attributes[0] ?? "";
        }
      }
      return {
        ...current,
        [automation.id]: nextDraft,
      };
    });
  };

  const syncTimelineVideoAutomationDrafts = (automations: TimelineVideoAutomationSummary[]) => {
    setTimelineVideoAutomationDrafts((current) => {
      const nextDrafts: Record<number, TimelineVideoAutomationDraft> = {};
      for (const automation of automations) {
        nextDrafts[automation.id] =
          current[automation.id] ?? timelineVideoAutomationDraftFromSummary(automation);
      }
      return nextDrafts;
    });
  };

  const timelineVideoAutomationDraft = (automation: TimelineVideoAutomationSummary) =>
    timelineVideoAutomationDrafts()[automation.id] ?? timelineVideoAutomationDraftFromSummary(automation);

  const updateTimelineVideoAutomationDraft = (
    automation: TimelineVideoAutomationSummary,
    patch: Partial<TimelineVideoAutomationDraft>,
  ) => {
    setTimelineVideoAutomationDrafts((current) => ({
      ...current,
      [automation.id]: {
        ...(current[automation.id] ?? timelineVideoAutomationDraftFromSummary(automation)),
        ...patch,
      },
    }));
  };

  const markProjectClean = (nextSnapshot = snapshot()) => {
    setCleanProjectSignature(projectSnapshotSignature(nextSnapshot));
    setProjectDirty(false);
  };

  const clearProjectRecovery = () => {
    lastRecoverySignature = null;
    setProjectRecoveryCheckpoint(null);
    clearProjectRecoveryCheckpoint();
  };

  const refreshProjectBackups = async () => {
    if (!isTauriRuntime()) {
      return;
    }
    try {
      setProjectBackups(await invoke<ProjectBackupSummary[]>("list_project_backups"));
    } catch (error) {
      setMessage(`Unable to list project backups: ${String(error)}`);
    }
  };

  const checkForApplicationUpdate = async (silent = false) => {
    const configuration = applicationUpdateConfiguration();
    if (!configuration?.enabled || applicationUpdateBusy()) {
      if (!silent && configuration?.reason) {
        setMessage(configuration.reason);
      }
      return;
    }
    setApplicationUpdateBusy(true);
    setApplicationUpdateError(null);
    try {
      const result = await invoke<ApplicationUpdateCheck>("check_application_update");
      setApplicationUpdateCheck(result);
      if (result.available && result.version) {
        setMessage(`Syndocal ${result.version} is available on the ${result.channel} channel.`);
      } else if (!silent) {
        setMessage(`Syndocal ${result.current_version} is current on the ${result.channel} channel.`);
      }
    } catch (error) {
      const detail = String(error);
      setApplicationUpdateError(detail);
      if (!silent) {
        setMessage(`Update check failed: ${detail}`);
      }
    } finally {
      setApplicationUpdateBusy(false);
    }
  };

  const initializeApplicationUpdate = async () => {
    if (!isTauriRuntime()) {
      return;
    }
    try {
      const configuration = await invoke<ApplicationUpdateConfiguration>("get_application_update_configuration");
      setApplicationUpdateConfiguration(configuration);
      if (configuration.enabled) {
        window.setTimeout(() => void checkForApplicationUpdate(true), 1_500);
      }
    } catch (error) {
      setApplicationUpdateError(String(error));
    }
  };

  const installAvailableApplicationUpdate = async () => {
    const update = applicationUpdateCheck();
    if (!update?.available || !update.version || applicationUpdateBusy()) {
      return;
    }
    if (
      !window.confirm(
        `Install Syndocal ${update.version}?\n\nA verified project backup will be created before download. The signed installer may close and restart Syndocal.`,
      )
    ) {
      return;
    }
    setApplicationUpdateBusy(true);
    setApplicationUpdateError(null);
    setApplicationUpdateProgress({ phase: "downloading", downloaded_bytes: 0, total_bytes: null });
    setMessage(`Downloading signed Syndocal ${update.version} update...`);
    try {
      await invoke<ProjectBackupSummary>("install_application_update", { expectedVersion: update.version });
      await refreshProjectBackups();
      setMessage(`Syndocal ${update.version} was verified and handed to the platform installer.`);
    } catch (error) {
      const detail = String(error);
      setApplicationUpdateError(detail);
      setMessage(`Update install failed: ${detail}`);
    } finally {
      setApplicationUpdateBusy(false);
    }
  };

  const refreshProjectHistoryStatus = async () => {
    if (!isTauriRuntime()) {
      return;
    }
    try {
      setProjectHistoryStatus(await invoke<ProjectHistoryStatus>("get_project_history_status"));
    } catch (error) {
      setMessage(`Unable to read Undo history: ${String(error)}`);
    }
  };

  const resetProjectHistory = async () => {
    if (!isTauriRuntime()) {
      return;
    }
    setProjectHistoryStatus(await invoke<ProjectHistoryStatus>("clear_project_history"));
  };

  const undoProject = async () => {
    if (!projectHistoryStatus().can_undo) {
      setMessage("Nothing to undo.");
      return;
    }
    try {
      const status = await invoke<ProjectHistoryStatus>("undo_project_transaction");
      setProjectHistoryStatus(status);
      await refreshSnapshot();
      setMessage(`Undid ${status.redo_label ?? "last edit"}.`);
    } catch (error) {
      setMessage(`Undo failed: ${String(error)}`);
    }
  };

  const redoProject = async () => {
    if (!projectHistoryStatus().can_redo) {
      setMessage("Nothing to redo.");
      return;
    }
    try {
      const status = await invoke<ProjectHistoryStatus>("redo_project_transaction");
      setProjectHistoryStatus(status);
      await refreshSnapshot();
      setMessage(`Redid ${status.undo_label ?? "last edit"}.`);
    } catch (error) {
      setMessage(`Redo failed: ${String(error)}`);
    }
  };

  const saveProjectRecovery = async () => {
    if (!isTauriRuntime() || !projectDirty()) {
      return;
    }
    const signature = projectSnapshotSignature(snapshot());
    try {
      if (signature !== lastRecoverySignature) {
        const project = await invoke<ProjectFile>("get_project_checkpoint");
        const checkpoint = createProjectRecoveryCheckpoint(project, currentProjectPath(), signature);
        if (!saveProjectRecoveryCheckpoint(checkpoint)) {
          throw new Error("browser storage is unavailable or full");
        }
        lastRecoverySignature = signature;
        setProjectRecoveryCheckpoint(checkpoint);
      }
      const now = Date.now();
      if (signature !== lastDesktopBackupSignature && now - lastDesktopBackupAt >= 60_000) {
        await invoke<ProjectBackupSummary>("save_project_backup", {
          sourcePath: currentProjectPath(),
          reason: "autosave",
        });
        lastDesktopBackupSignature = signature;
        lastDesktopBackupAt = now;
        await refreshProjectBackups();
      }
    } catch (error) {
      setMessage(`Recovery checkpoint failed: ${String(error)}`);
    }
  };

  const rememberRecentProjectPath = (path: string | null) => {
    if (!path) {
      return;
    }
    setRecentProjectPaths((current) => {
      const next = touchRecentProjectPath(current, path);
      saveRecentProjectPaths(next);
      return next;
    });
  };

  const clearRecentProjects = () => {
    setRecentProjectPaths([]);
    saveRecentProjectPaths([]);
    setMessage("Recent projects cleared.");
  };

  const confirmDiscardProjectChanges = (actionLabel: string) => {
    if (!projectDirty()) {
      return true;
    }
    return window.confirm(`Discard unsaved changes and ${actionLabel}?`);
  };

  const applyEngineSnapshot = (next: EngineSnapshot, syncProjectState = true) => {
    setSnapshot(next);
    if (syncProjectState) {
      const signature = projectSnapshotSignature(next);
      const cleanSignature = cleanProjectSignature();
      if (cleanSignature === null) {
        setCleanProjectSignature(signature);
        setProjectDirty(false);
      } else {
        setProjectDirty(signature !== cleanSignature);
      }
      setOutput(next.output);
      setDmxOutputRoutes(next.dmx_outputs.length > 0 ? next.dmx_outputs : [next.output]);
      setFaderValues((current) => ({ ...current, ...snapshotFaderValues(next) }));
      syncVideoOutputConfigDrafts(next.video.outputs);
      syncCueMetadataDrafts(next.cues);
      syncTimelineEventDrafts(next.timeline.events);
      syncTimelineAutomationDrafts(next.timeline.automations);
      syncTimelineVideoAutomationDrafts(next.timeline.video_automations);
    }
    const groupId = selectedFixtureGroupFilter();
    if (groupId && !next.fixtures.some((fixture) => fixture.group_ids.includes(groupId))) {
      setSelectedFixtureGroupFilter(null);
    }
    const selectedId = selectedFixtureId();
    const selectedExists = selectedId !== null && next.fixtures.some((fixture) => fixture.id === selectedId);
    const liveFixtureIds = new Set(next.fixtures.map((fixture) => fixture.id));
    const nextMappingSelection = selectedMappingFixtureIds().filter((id) => liveFixtureIds.has(id));
    if (!selectedExists && next.fixtures.length > 0) {
      selectFixture(next.fixtures[0]);
    } else if (!selectedExists) {
      setSelectedFixtureId(null);
      setSelectedMappingFixtureIds([]);
      setSelectedFixtureLabelDraft("");
      setSelectedFixtureUniverseDraft(0);
      setSelectedFixtureAddressDraft(1);
      setSelectedFixtureGroupText("");
    } else if (nextMappingSelection.length !== selectedMappingFixtureIds().length) {
      setSelectedMappingFixtureIds(nextMappingSelection.length > 0 ? nextMappingSelection : [selectedId]);
    }
  };

  const refreshSnapshot = async (syncProjectState = true) => {
    try {
      const next = await invoke<EngineSnapshot>("get_snapshot");
      applyEngineSnapshot(next, syncProjectState);
      return next;
    } catch (error) {
      setMessage(String(error));
      return null;
    }
  };

  const refreshSnapshotDelta = async () => {
    try {
      const response = await invoke<EngineSnapshotSyncResponse>("get_snapshot_delta", {
        clientRevision: snapshotRevision(),
      });
      const next = response.full ?? ({ ...snapshot(), ...(response.delta ?? {}) } as EngineSnapshot);
      setSnapshotRevision(response.revision);
      applyEngineSnapshot(next, false);
      return next;
    } catch (error) {
      setSnapshotRevision(null);
      setMessage(String(error));
      return null;
    }
  };

  let snapshotPollTimer: number | null = null;
  const scheduleSnapshotPoll = () => {
    if (!isTauriRuntime()) {
      return;
    }
    const intervalMs = document.hidden ? 2_000 : workspaceTab() === "setup" ? 1_000 : 250;
    snapshotPollTimer = window.setTimeout(async () => {
      await refreshSnapshotDelta();
      scheduleSnapshotPoll();
    }, intervalMs);
  };
  scheduleSnapshotPoll();
  const telemetryReportTimer = isTauriRuntime() ? window.setInterval(refreshEngineTelemetryReport, 1000) : null;
  const dmxInputStatusTimer = isTauriRuntime() ? window.setInterval(refreshDmxInputStatus, 1000) : null;
  const recoveryTimer = isTauriRuntime() ? window.setInterval(() => void saveProjectRecovery(), 10_000) : null;
  onCleanup(() => {
    if (snapshotPollTimer !== null) {
      window.clearTimeout(snapshotPollTimer);
    }
    if (telemetryReportTimer !== null) {
      window.clearInterval(telemetryReportTimer);
    }
    if (dmxInputStatusTimer !== null) {
      window.clearInterval(dmxInputStatusTimer);
    }
    if (recoveryTimer !== null) {
      window.clearInterval(recoveryTimer);
    }
  });
  createEffect(() => {
    if (!isTauriRuntime()) {
      setMessage(tauriBackendUnavailableMessage);
      return;
    }
    void refreshSnapshot();
    void refreshEngineTelemetryReport();
    void refreshMidiInputs();
    void refreshMidiOutputs();
    void refreshSerialPorts();
    void refreshDmxInputStatus();
    void refreshProjectBackups();
    void refreshProjectHistoryStatus();
    void initializeApplicationUpdate();
  });
  createEffect(() => {
    if (!isTauriRuntime()) {
      return;
    }
    let disposed = false;
    let unlistenUpdateProgress: (() => void) | null = null;
    void listen<ApplicationUpdateProgress>("syndocal://application-update-progress", (event) => {
      setApplicationUpdateProgress(event.payload);
    })
      .then((unlisten) => {
        if (disposed) {
          unlisten();
        } else {
          unlistenUpdateProgress = unlisten;
        }
      })
      .catch((error) => setApplicationUpdateError(String(error)));
    onCleanup(() => {
      disposed = true;
      unlistenUpdateProgress?.();
    });
  });
  createEffect(() => {
    if (!isTauriRuntime()) {
      return;
    }
    void loadStartupProject();
    void loadQueuedOpenProjects();
    let disposed = false;
    let unlistenOpenProject: (() => void) | null = null;
    let unlistenProjectDrop: (() => void) | null = null;
    void listen<string[]>("syndocal://open-project", (event) => {
      const paths = Array.isArray(event.payload) ? event.payload : [];
      const path = paths[paths.length - 1];
      if (path) {
        void loadProjectPath(path);
      }
    })
      .then((unlisten) => {
        if (disposed) {
          unlisten();
        } else {
          unlistenOpenProject = unlisten;
        }
      })
      .catch((error) => setMessage(String(error)));
    void getCurrentWebview()
      .onDragDropEvent((event) => {
        if (event.payload.type === "enter") {
          setProjectDropState(event.payload.paths.some(isSyndocalProjectPath) ? "project" : "invalid");
          return;
        }
        if (event.payload.type === "leave") {
          setProjectDropState(null);
          return;
        }
        if (event.payload.type !== "drop") {
          return;
        }
        setProjectDropState(null);
        const projectPath = event.payload.paths.find(isSyndocalProjectPath);
        if (!projectPath) {
          if (event.payload.paths.length > 0) {
            setMessage("Drop a .sdc project file to open it.");
          }
          return;
        }
        void loadProjectPath(projectPath);
      })
      .then((unlisten) => {
        if (disposed) {
          unlisten();
        } else {
          unlistenProjectDrop = unlisten;
        }
      })
      .catch((error) => setMessage(String(error)));
    onCleanup(() => {
      disposed = true;
      unlistenOpenProject?.();
      unlistenProjectDrop?.();
    });
  });
  createEffect(() => {
    const attribute = selectedEffectAttribute();
    if (attribute && !midiMapAttribute()) {
      setMidiMapAttribute(attribute);
    }
    if (attribute && !oscMapAttribute()) {
      setOscMapAttribute(attribute);
    }
  });
  createEffect(() => {
    const lastBank = cuePadBankCount() - 1;
    if (cuePadBank() > lastBank) {
      setCuePadBank(lastBank);
    }
  });
  createEffect(() => {
    if (!cuePadFollowActive()) {
      return;
    }
    const index = activeCueIndex();
    if (index < 0) {
      return;
    }
    const activeBank = Math.floor(index / cuePadSize);
    if (cuePadBank() !== activeBank) {
      setCuePadBank(activeBank);
    }
  });
  createEffect(() => {
    const typeKey = selectedFixtureTypeFilter();
    if (typeKey && !fixtureTypeRows().some((row) => row.key === typeKey)) {
      setSelectedFixtureTypeFilter(null);
    }
  });
  createEffect(() => {
    const outputs = snapshot().video.outputs;
    const outputId = selectedVideoOutputId();
    if (outputs.length === 0) {
      if (outputId !== null) {
        setSelectedVideoOutputId(null);
      }
      return;
    }
    if (outputId === null || !outputs.some((output) => output.id === outputId)) {
      setSelectedVideoOutputId(outputs[0].id);
    }
  });

  const selectGdtfFile = async () => {
    try {
      const path = await invoke<string | null>("select_gdtf_file");
      if (path) {
        setGdtfPath(path);
        setMessage(`Selected ${path}`);
      }
    } catch (error) {
      setMessage(String(error));
    }
  };

  const loadGdtfProfile = async (path: string, loadedMessage = "Loaded") => {
    const imported = await invoke<FixtureProfileSummary>("import_gdtf", { path });
    setProfile(imported);
    setSelectedMode(imported.dmx_modes[0]?.name ?? "");
    setMessage(profileLoadMessage(loadedMessage, imported));
  };

  const importGdtf = async () => {
    try {
      await loadGdtfProfile(gdtfPath());
    } catch (error) {
      setMessage(String(error));
    }
  };

  const selectVideoSourceFile = async () => {
    const sourceKind = videoSourceKind();
    if (!videoSourceCanBrowseFile(sourceKind)) {
      setMessage("Named network/GPU sources do not use a local file picker.");
      return;
    }
    try {
      const path = await invoke<string | null>("select_video_source_file", { kind: sourceKind });
      if (!path) {
        setMessage("Video source selection canceled.");
        return;
      }
      setVideoPath(path);
      if (shouldReplaceVideoLayerDraftLabel(videoLabel())) {
        setVideoLabel(mediaLabelFromPath(path));
      }
      setMessage(`Selected ${path}`);
    } catch (error) {
      setMessage(String(error));
    }
  };

  const downloadGdtfFromUrl = async () => {
    try {
      const path = await invoke<string | null>("download_gdtf_from_url", { url: gdtfShareUrl() });
      if (!path) {
        setMessage("GDTF download canceled.");
        return;
      }
      setGdtfPath(path);
      await loadGdtfProfile(path, "Downloaded and loaded");
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setCustomAttributesText = (value: string) => {
    setCustomAttributes(value);
    setCustomAttributeDrafts(customProfileAttributeDraftsFromText(value));
    setSelectedCustomAttributeIndex(null);
  };

  const commitCustomAttributeDrafts = (drafts: CustomProfileAttributeDraft[], selectedIndex?: number | null) => {
    setCustomAttributeDrafts(drafts);
    setCustomAttributes(customProfileAttributeTextFromDrafts(drafts));
    setSelectedCustomAttributeIndex(
      selectedIndex !== undefined
        ? selectedIndex
        : selectedCustomAttributeIndexValue() !== null && selectedCustomAttributeIndexValue()! < drafts.length
          ? selectedCustomAttributeIndexValue()
          : drafts.length > 0
            ? drafts.length - 1
            : null,
    );
  };

  const updateCustomAttributeDraft = (index: number, updates: Partial<CustomProfileAttributeDraft>) => {
    const drafts = customAttributeDrafts();
    if (index < 0 || index >= drafts.length) {
      return;
    }
    commitCustomAttributeDrafts(
      drafts.map((draft, draftIndex) => (draftIndex === index ? { ...draft, ...updates } : draft)),
      index,
    );
  };

  const addCustomAttributeDraft = () => {
    const drafts = customAttributeDrafts();
    commitCustomAttributeDrafts(
      [...drafts, { attribute: `Attribute${drafts.length + 1}`, resolution: "EightBit", startOffset: "" }],
      drafts.length,
    );
  };

  const appendCustomAttributeTemplate = (rows: CustomProfileAttributeDraft[]) => {
    const drafts = customAttributeDrafts();
    const nextRows = rows.map((row) => ({ ...row }));
    commitCustomAttributeDrafts([...drafts, ...nextRows], drafts.length);
  };

  const removeCustomAttributeDraft = (index: number) => {
    const drafts = customAttributeDrafts();
    if (index < 0 || index >= drafts.length) {
      return;
    }
    const next = drafts.filter((_, draftIndex) => draftIndex !== index);
    commitCustomAttributeDrafts(next, next.length === 0 ? null : Math.min(index, next.length - 1));
  };

  const moveCustomAttributeDraft = (index: number, delta: -1 | 1) => {
    const drafts = [...customAttributeDrafts()];
    const nextIndex = index + delta;
    if (index < 0 || nextIndex < 0 || index >= drafts.length || nextIndex >= drafts.length) {
      return;
    }
    const [draft] = drafts.splice(index, 1);
    drafts.splice(nextIndex, 0, draft);
    commitCustomAttributeDrafts(drafts, nextIndex);
  };

  const customProfileRequest = (): CustomFixtureProfileRequest => ({
    manufacturer: customManufacturer(),
    name: customProfileName(),
    mode_name: customModeName(),
    attributes: customAttributes()
      .split(",")
      .map((attribute) => attribute.trim())
      .filter(Boolean),
  });

  const createCustomProfile = async () => {
    const request = customProfileRequest();
    try {
      const created = await invoke<FixtureProfileSummary>("create_custom_fixture_profile", { request });
      setProfile(created);
      setGdtfPath(created.source_path);
      setSelectedMode(created.dmx_modes[0]?.name ?? "");
      setMessage(profileLoadMessage("Created custom profile", created));
    } catch (error) {
      setMessage(String(error));
    }
  };

  const saveCustomProfile = async () => {
    const request = customProfileRequest();
    try {
      const path = await invoke<string | null>("save_custom_fixture_profile", { request });
      setMessage(path ? `Saved custom profile ${path}` : "Custom profile save canceled.");
    } catch (error) {
      setMessage(String(error));
    }
  };

  const loadCustomProfile = async () => {
    try {
      const created = await invoke<FixtureProfileSummary | null>("load_custom_fixture_profile");
      if (!created) {
        setMessage("Custom profile load canceled.");
        return;
      }
      setProfile(created);
      setGdtfPath(created.source_path);
      setSelectedMode(created.dmx_modes[0]?.name ?? "");
      setCustomManufacturer(created.manufacturer);
      setCustomProfileName(created.name);
      setCustomModeName(created.dmx_modes[0]?.name ?? "Default");
      setCustomAttributesText(
        created.dmx_modes[0]?.controls
          .map((control) => `${control.attribute}@${control.offsets[0] ?? 1}:${control.resolution === "SixteenBit" ? "16" : "8"}`)
          .join(", ") ?? "",
      );
      setMessage(profileLoadMessage("Loaded custom profile", created));
    } catch (error) {
      setMessage(String(error));
    }
  };

  const useFixtureProfileForPatch = async (fixture: PatchedFixtureSummary) => {
    try {
      const imported = await invoke<FixtureProfileSummary>("use_fixture_profile", { fixtureId: fixture.id });
      const footprint = Math.max(1, fixtureFootprint(fixture));
      setProfile(imported);
      setGdtfPath(imported.source_path);
      setSelectedMode(fixture.mode_name);
      setLabel(`${fixture.label} Copy`);
      setUniverse(fixture.universe);
      setAddress(Math.min(512, fixture.address + footprint));
      setPatchCount(1);
      setPatchAddressStride(footprint);
      setGroupText(fixture.group_ids.join(", "));
      setPatchX(fixture.position.x + 1);
      setPatchY(fixture.position.y);
      setPatchZ(fixture.position.z);
      setPatchPitch(fixture.rotation.pitch);
      setPatchYaw(fixture.rotation.yaw);
      setPatchRoll(fixture.rotation.roll);
      setMessage(`Using ${fixture.label}'s profile for patching`);
    } catch (error) {
      setMessage(String(error));
    }
  };

  const duplicateFixture = async (fixture: PatchedFixtureSummary) => {
    try {
      const footprint = Math.max(1, fixtureFootprint(fixture));
      const occupiedRanges = buildOccupiedDmxRanges(snapshot().fixtures);
      const address = findFreeDmxAddress(occupiedRanges, fixture.universe, footprint, fixture.address + footprint);
      if (address === null) {
        setMessage(`Cannot duplicate ${fixture.label}: no ${footprint}ch gap remains in universe ${fixture.universe}.`);
        return;
      }
      const imported = await invoke<FixtureProfileSummary>("use_fixture_profile", { fixtureId: fixture.id });
      const request: PatchFixtureRequest = {
        profile_path: imported.source_path,
        mode_name: fixture.mode_name,
        label: `${fixture.label} Copy`,
        universe: fixture.universe,
        address,
        group_ids: [...fixture.group_ids],
        position: {
          x: Number((fixture.position.x + 1).toFixed(2)),
          y: Number(fixture.position.y.toFixed(2)),
          z: Number(fixture.position.z.toFixed(2)),
        },
        rotation: { ...fixture.rotation },
      };
      const fixtureIds = await invoke<number[]>("patch_fixtures", { requests: [request] });
      const fixtureId = fixtureIds[0];
      if (fixtureId === undefined) {
        setMessage("Fixture duplicate did not return a fixture id.");
        return;
      }
      await invoke("set_fixture_limits", { fixtureId, limits: fixture.limits });
      const copiedValues: Record<string, number> = {};
      for (const value of fixture.attribute_values) {
        await invoke("set_attribute", {
          fixtureId,
          attribute: value.attribute,
          value: value.value,
        });
        copiedValues[`${fixtureId}:${value.attribute}`] = value.value;
      }
      setSelectedFixtureId(fixtureId);
      setSelectedMappingFixtureIds([fixtureId]);
      setSelectedFixtureLabelDraft(request.label);
      setSelectedFixtureUniverseDraft(request.universe);
      setSelectedFixtureAddressDraft(request.address);
      setSelectedFixtureGroupText(request.group_ids.join(", "));
      setSelectedFixtureLimitsDraft(fixture.limits);
      setFaderValues((current) => ({ ...current, ...copiedValues }));
      setMessage(
        `Duplicated ${fixture.label} as ${request.label} at U${request.universe} A${request.address}-${
          request.address + footprint - 1
        }.`,
      );
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const duplicateSelectedMappingFixtures = async () => {
    const fixtures = [...selectedMappingFixtures()].sort((left, right) =>
      left.universe === right.universe ? left.address - right.address : left.universe - right.universe,
    );
    if (fixtures.length === 0) {
      setMessage("Select fixtures on the mapping stage first.");
      return;
    }

    const occupiedRanges = buildOccupiedDmxRanges(snapshot().fixtures);
    const nextAddressByUniverse = new Map<number, number>();
    for (const [universeId, ranges] of occupiedRanges) {
      const maxEnd = Math.max(0, ...ranges.map((range) => range.end));
      nextAddressByUniverse.set(universeId, maxEnd + 1);
    }
    const offset = mappingSnapEnabled() ? normalizedMappingSnapSize() : 1;
    const sourceFixtures: PatchedFixtureSummary[] = [];
    const requests: PatchFixtureRequest[] = [];

    try {
      for (const fixture of fixtures) {
        const imported = await invoke<FixtureProfileSummary>("use_fixture_profile", { fixtureId: fixture.id });
        const footprint = Math.max(1, fixtureFootprint(fixture));
        const preferredAddress = Math.max(
          fixture.address + footprint,
          nextAddressByUniverse.get(fixture.universe) ?? 1,
        );
        const nextAddress = findFreeDmxAddress(occupiedRanges, fixture.universe, footprint, preferredAddress);
        if (nextAddress === null) {
          setMessage(`Cannot duplicate ${fixture.label}: no ${footprint}ch gap remains in universe ${fixture.universe}.`);
          return;
        }
        reserveDmxAddressRange(occupiedRanges, fixture.universe, nextAddress, footprint);
        nextAddressByUniverse.set(fixture.universe, nextAddress + footprint);
        sourceFixtures.push(fixture);
        requests.push({
          profile_path: imported.source_path,
          mode_name: fixture.mode_name,
          label: `${fixture.label} Copy`,
          universe: fixture.universe,
          address: nextAddress,
          group_ids: [...fixture.group_ids],
          position: snapStagePosition({
            x: fixture.position.x + offset,
            y: fixture.position.y,
            z: fixture.position.z + offset,
          }),
          rotation: { ...fixture.rotation },
        });
      }

      const fixtureIds = await invoke<number[]>("patch_fixtures", { requests });
      if (fixtureIds.length !== requests.length) {
        setMessage(`Fixture duplicate returned ${fixtureIds.length}/${requests.length} fixture id(s).`);
        await refreshSnapshot();
        return;
      }

      const nextFaderValues: Record<string, number> = {};
      for (const [index, fixtureId] of fixtureIds.entries()) {
        const source = sourceFixtures[index];
        await invoke("set_fixture_limits", { fixtureId, limits: source.limits });
        for (const value of source.attribute_values) {
          await invoke("set_attribute", {
            fixtureId,
            attribute: value.attribute,
            value: value.value,
          });
          nextFaderValues[`${fixtureId}:${value.attribute}`] = value.value;
        }
      }

      setFaderValues((current) => ({ ...current, ...nextFaderValues }));
      setSelectedMappingFixtureIds(fixtureIds);
      const firstRequest = requests[0];
      const firstId = fixtureIds[0];
      if (firstRequest && firstId !== undefined) {
        setSelectedFixtureId(firstId);
        setSelectedFixtureLabelDraft(firstRequest.label);
        setSelectedFixtureUniverseDraft(firstRequest.universe);
        setSelectedFixtureAddressDraft(firstRequest.address);
        setSelectedFixtureGroupText(firstRequest.group_ids.join(", "));
        setSelectedFixtureLimitsDraft(sourceFixtures[0]?.limits ?? defaultFixtureLimits);
      }
      setMessage(
        fixtures.length === 1
          ? `Duplicated ${fixtures[0].label} as ${firstRequest?.label ?? "copy"}.`
          : `Duplicated ${fixtures.length} selected fixtures.`,
      );
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const patchFixture = async () => {
    const imported = profile();
    if (!imported) {
      setMessage("Load a GDTF profile first.");
      return;
    }
    if (selectedFootprint() === 0) {
      setMessage("Selected GDTF mode has no DMX channel offsets.");
      return;
    }
    if (endAddress() > 512) {
      setMessage(`Fixture exceeds DMX universe: start ${address()}, footprint ${selectedFootprint()}ch, end ${endAddress()}`);
      return;
    }
    const conflict = patchAddressConflictText();
    if (conflict) {
      setMessage(`DMX address conflict: ${patchAddressConflictText()}`);
      return;
    }
    const count = patchCountValue();
    const addressStride = patchAddressStrideValue();
    const footprint = selectedFootprint();
    const groupIds = parseGroupIds(groupText());
    const baseLabel = label().trim() || "Fixture";
    const requests: PatchFixtureRequest[] = Array.from({ length: count }, (_, index) => {
      const position = patchFixturePosition(index, count);
      return {
        profile_path: imported.source_path,
        mode_name: selectedMode() || null,
        label: bulkPatchLabel(baseLabel, index, count),
        universe: universe(),
        address: address() + index * addressStride,
        group_ids: groupIds,
        position: {
          x: Number(position.x.toFixed(2)),
          y: Number(position.y.toFixed(2)),
          z: Number(position.z.toFixed(2)),
        },
        rotation: { pitch: patchPitch(), yaw: patchYaw(), roll: patchRoll() },
      };
    });

    try {
      const fixtureIds = await invoke<number[]>("patch_fixtures", { requests });
      const fixtureId = fixtureIds[fixtureIds.length - 1];
      const request = requests[requests.length - 1];
      if (fixtureId === undefined || request === undefined) {
        setMessage("Patch did not return a fixture id.");
        return;
      }
      setSelectedFixtureId(fixtureId);
      setSelectedFixtureLabelDraft(request.label);
      setSelectedFixtureUniverseDraft(request.universe);
      setSelectedFixtureAddressDraft(request.address);
      setSelectedFixtureGroupText(request.group_ids.join(", "));
      const initialValues: Record<string, number> = {};
      for (const patchedFixtureId of fixtureIds) {
        for (const control of selectedModeSummary()?.controls ?? []) {
          initialValues[`${patchedFixtureId}:${control.attribute}`] = control.default_value;
        }
      }
      setFaderValues((current) => ({ ...current, ...initialValues }));
      const next = await refreshSnapshot();
      const preferredNextAddress = request.address + addressStride;
      const nextAddress = next
        ? findNextFreePatchAddress(next.fixtures, request.universe, footprint, count, addressStride, preferredNextAddress)
        : null;
      if (nextAddress !== null) {
        setUniverse(request.universe);
        setAddress(nextAddress);
        setPatchGridUniverse(request.universe);
        setDmxPatchViewMode("grid");
      }
      const firstRequest = requests[0];
      const patchedRangeText =
        count === 1
          ? `U${request.universe} A${request.address}-${request.address + footprint - 1}`
          : firstRequest
            ? `U${request.universe} A${firstRequest.address}-${request.address + footprint - 1}${
                addressStride === footprint ? "" : ` (${count} fixtures, step ${addressStride}ch)`
              }`
            : `U${request.universe}`;
      const nextText = nextAddress !== null ? ` Next free A${nextAddress}.` : " No matching free range remains.";
      setMessage(
        count === 1
          ? `Patched ${request.label} at ${patchedRangeText}.${nextText}`
          : `Patched ${fixtureIds.length} fixtures at ${patchedRangeText}.${nextText}`,
      );
    } catch (error) {
      setMessage(String(error));
    }
  };

  const removeFixture = async (fixtureId: number) => {
    const fixture = snapshot().fixtures.find((candidate) => candidate.id === fixtureId);
    if (fixture && !confirmDestructiveAction("fixture", fixture.label)) {
      return;
    }
    try {
      await invoke("remove_fixture", { fixtureId });
      if (selectedFixtureId() === fixtureId) {
        setSelectedFixtureId(null);
        setSelectedFixtureLabelDraft("");
        setSelectedFixtureUniverseDraft(0);
        setSelectedFixtureAddressDraft(1);
        setSelectedFixtureGroupText("");
      }
      setFaderValues((current) => {
        const next = { ...current };
        for (const key of Object.keys(next)) {
          if (key.startsWith(`${fixtureId}:`)) {
            delete next[key];
          }
        }
        return next;
      });
      setMessage(`Removed fixture ${fixtureId}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setAttribute = async (fixtureId: number, attribute: string, value: number) => {
    setFaderValues((current) => ({ ...current, [`${fixtureId}:${attribute}`]: value }));
    try {
      await invoke(snapshot().programmer.enabled ? "set_programmer_attribute" : "set_attribute", {
        fixtureId,
        attribute,
        value,
      });
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setGroupAttribute = async (groupId: string, attribute: string, value: number) => {
    const fixtureIds = snapshot()
      .fixtures
      .filter((fixture) => fixture.group_ids.includes(groupId))
      .map((fixture) => fixture.id);
    setFaderValues((current) => {
      const next = { ...current };
      for (const fixtureId of fixtureIds) {
        next[`${fixtureId}:${attribute}`] = value;
      }
      return next;
    });
    try {
      await invoke(snapshot().programmer.enabled ? "set_programmer_group_attribute" : "set_group_attribute", {
        groupId,
        attribute,
        value,
      });
    } catch (error) {
      setMessage(String(error));
    }
  };

  const paletteKindIncludesAttribute = (kind: PaletteKind, attribute: string) => {
    if (kind === "All") return true;
    const category = controlCategoryForAttribute(attribute);
    if (kind === "Intensity") return category === "dimmer";
    if (kind === "Position") return category === "position";
    if (kind === "Color") return category === "color";
    return category === "beam" || category === "focus" || category === "gobo";
  };

  const referencePaletteValuesFromFixture = (fixture: PatchedFixtureSummary, kind: PaletteKind) => {
    const values = new Map<string, number>();
    for (const control of fixture.controls) {
      if (paletteKindIncludesAttribute(kind, control.attribute)) {
        values.set(
          control.attribute,
          faderValue(fixture.id, control.attribute, control.default_value),
        );
      }
    }
    return [...values].map(([attribute, value]) => ({ attribute, value }));
  };

  const createReferencePalette = async (label: string, kind: PaletteKind) => {
    const fixture = selectedFixture();
    if (!fixture) {
      setMessage("Select a fixture before capturing a palette.");
      return;
    }
    const values = referencePaletteValuesFromFixture(fixture, kind);
    if (values.length === 0) {
      setMessage(`${fixture.label} has no ${kind.toLowerCase()} attributes to capture.`);
      return;
    }
    try {
      const paletteId = await invoke<number>("create_reference_palette", { label, kind, values });
      setMessage(`Captured palette ${paletteId} from ${fixture.label}.`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const updateReferencePalette = async (palette: ReferencePaletteSummary) => {
    const fixture = selectedFixture();
    if (!fixture) {
      setMessage("Select a fixture before recapturing a palette.");
      return;
    }
    const values = referencePaletteValuesFromFixture(fixture, palette.kind);
    if (values.length === 0) {
      setMessage(`${fixture.label} has no ${palette.kind.toLowerCase()} attributes to capture.`);
      return;
    }
    try {
      await invoke("update_reference_palette", {
        paletteId: palette.id,
        label: palette.label,
        kind: palette.kind,
        values,
      });
      setMessage(`Recaptured ${palette.label} from ${fixture.label}; linked Cues will use it on their next GO.`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const applyReferencePalette = async (palette: ReferencePaletteSummary) => {
    const fixtures = selectedControlTargetFixtures();
    if (fixtures.length === 0) {
      setMessage("Select a fixture or group before applying a palette.");
      return;
    }
    try {
      await invoke("apply_reference_palette", {
        paletteId: palette.id,
        fixtureIds: fixtures.map((fixture) => fixture.id),
        programmer: snapshot().programmer.enabled,
      });
      setMessage(`Applied ${palette.label} to ${controlTargetLabel()}.`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const removeReferencePalette = async (palette: ReferencePaletteSummary) => {
    if (!confirmDestructiveAction("palette", palette.label)) return;
    try {
      await invoke("remove_reference_palette", { paletteId: palette.id });
      setMessage(`Removed ${palette.label} and its Cue references.`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setProgrammerMode = async (enabled: boolean, blind: boolean) => {
    try {
      await invoke("set_programmer_mode", { enabled, blind });
      if (!enabled) setFaderValues({});
      setMessage(!enabled ? "Direct live editing enabled." : blind ? "Programmer Blind enabled." : "Programmer Live Preview enabled.");
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const clearProgrammer = async () => {
    try {
      await invoke("clear_programmer");
      setFaderValues({});
      setMessage("Cleared staged Programmer values.");
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const commitProgrammer = async () => {
    try {
      await invoke("commit_programmer");
      setMessage("Committed Programmer values to the live base state. Undo is available.");
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setFixtureTransform = async (
    fixture: PatchedFixtureSummary,
    next: {
      position?: PatchFixtureRequest["position"];
      rotation?: PatchFixtureRequest["rotation"];
    },
    refresh = true,
  ) => {
    try {
      await invoke("set_fixture_transform", {
        fixtureId: fixture.id,
        position: next.position ?? fixture.position,
        rotation: next.rotation ?? fixture.rotation,
      });
      if (refresh) {
        await refreshSnapshot();
      }
    } catch (error) {
      setMessage(String(error));
    }
  };

  const {
    layoutFixturePositions,
    layoutSelectedMappingFixtures,
    layoutSelectedFixturesOnStageObject,
    stageObjectFixtureCounts,
    pickFixturesInsideSelectedStageObject,
    alignSelectedMappingFixtures,
    distributeSelectedMappingFixtures,
    nudgeSelectedMappingFixtures,
    mirrorSelectedMappingFixtures,
    rotateSelectedMappingFixtures,
  } = createMappingLayoutController({
    snapshot,
    filteredFixtures,
    selectedFixtureGroupFilter,
    selectedMappingFixtures,
    selectedStageObject,
    selectedFixture,
    selectedMappingFixtureIds,
    setSelectedMappingFixtureIds,
    activateFixture,
    setFixtureTransform,
    snapStagePosition,
    mappingWorldToStageObjectLocal,
    refreshSnapshot,
    setMessage,
  });

  const {
    addStageObjectAtCenter,
    setStageObject,
    removeStageObject,
    setStageMapConfig,
    lockStageMapToCurrentBounds,
    stageMapPresetObjectCountLabel,
    mappingViewPresetObjectLabel,
    saveStageMapPreset,
    applyStageMapPreset,
    removeStageMapPreset,
    exportStageMapPreset,
    importStageMapPreset,
  } = createStageMapController({
    invoke,
    snapshot,
    refreshSnapshot,
    setMessage,
    stageObjectLabel,
    stageObjectKind,
    stageObjectWidth,
    stageObjectDepth,
    stageObjectRotation,
    stageObjectColor,
    stageWorldBounds,
    autoStageWorldBounds,
    setSelectedStageObjectId,
    selectedStageObjectId,
    setMappingShowStageObjects,
    stageMapPresetLabel,
    setStageMapPresetLabel,
    selectedStageMapPresetLabel,
    setSelectedStageMapPresetLabel,
  });

  const applyMappingSelectionGroups = async (mode: MappingBulkGroupMode) => {
    const fixtures = selectedMappingFixtures();
    if (fixtures.length === 0) {
      setMessage("Select fixtures on the mapping stage first.");
      return;
    }
    const groupIds = uniqueGroupIds(parseGroupIds(mappingSelectionGroupText()));
    if (groupIds.length === 0) {
      setMessage("Enter one or more group IDs.");
      return;
    }
    const groupIdSet = new Set(groupIds);
    try {
      for (const fixture of fixtures) {
        const nextGroupIds =
          mode === "set"
            ? groupIds
            : mode === "add"
              ? uniqueGroupIds([...fixture.group_ids, ...groupIds])
              : fixture.group_ids.filter((groupId) => !groupIdSet.has(groupId));
        await invoke("set_fixture_groups", {
          fixtureId: fixture.id,
          groupIds: nextGroupIds,
        });
      }
      setMappingSelectionGroupText(groupIds.join(", "));
      const actionLabel = mode === "set" ? "Set" : mode === "add" ? "Added" : "Removed";
      setMessage(`${actionLabel} groups for ${fixtures.length} selected fixture(s).`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setMappingSelectionFlag = async (flag: MappingFixtureFlag, enabled: boolean) => {
    const fixtures = selectedMappingFixtures();
    if (fixtures.length === 0) {
      setMessage("Select fixtures on the mapping stage first.");
      return;
    }
    const command =
      flag === "highlight"
        ? "set_fixture_highlight"
        : flag === "solo"
          ? "set_fixture_solo"
          : "set_fixture_park";
    try {
      for (const fixture of fixtures) {
        await invoke(command, { fixtureId: fixture.id, enabled });
      }
      const label = flag === "highlight" ? "Highlight" : flag === "solo" ? "Solo" : "Park";
      setMessage(`${enabled ? "Set" : "Cleared"} ${label} for ${fixtures.length} selected fixture(s).`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const removeSelectedMappingFixtures = async () => {
    const fixtures = selectedMappingFixtures();
    if (fixtures.length === 0) {
      setMessage("Select fixtures on the mapping stage first.");
      return;
    }
    const removeLabel = fixtures.length === 1 ? fixtures[0].label : `${fixtures.length} selected fixtures`;
    if (!window.confirm(`Remove ${removeLabel}?`)) {
      return;
    }
    const removedIds = new Set(fixtures.map((fixture) => fixture.id));
    try {
      for (const fixture of fixtures) {
        await invoke("remove_fixture", { fixtureId: fixture.id });
      }
      setFaderValues((current) => {
        const next = { ...current };
        for (const key of Object.keys(next)) {
          const fixtureId = Number(key.split(":")[0]);
          if (removedIds.has(fixtureId)) {
            delete next[key];
          }
        }
        return next;
      });
      setSelectedMappingFixtureIds([]);
      setSelectedFixtureId(null);
      setSelectedFixtureLabelDraft("");
      setSelectedFixtureUniverseDraft(0);
      setSelectedFixtureAddressDraft(1);
      setSelectedFixtureGroupText("");
      setMessage(`Removed ${removeLabel}.`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setFixturePatch = async (fixture: PatchedFixtureSummary) => {
    try {
      await invoke("set_fixture_patch", {
        fixtureId: fixture.id,
        label: selectedFixtureLabelDraft(),
        universe: selectedFixtureUniverseDraft(),
        address: selectedFixtureAddressDraft(),
      });
      setMessage(`Updated patch for ${selectedFixtureLabelDraft().trim() || fixture.label}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setFixtureGroups = async (fixture: PatchedFixtureSummary) => {
    const groupIds = parseGroupIds(selectedFixtureGroupText());
    try {
      await invoke("set_fixture_groups", {
        fixtureId: fixture.id,
        groupIds,
      });
      setSelectedFixtureGroupText(groupIds.join(", "));
      setMessage(`Updated groups for ${fixture.label}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const updateSelectedFixtureLimit = <Key extends keyof FixtureLimits>(
    key: Key,
    value: FixtureLimits[Key],
  ) => {
    setSelectedFixtureLimitsDraft((limits) => ({
      ...limits,
      [key]: typeof value === "number" ? clampDmxValue(value) : value,
    }));
  };

  const movementLimitPointFromPointer = (
    event: PointerEvent & { currentTarget: HTMLElement },
  ): MovementLimitPoint => {
    const bounds = event.currentTarget.getBoundingClientRect();
    const x = clamp01((event.clientX - bounds.left) / bounds.width);
    const y = clamp01((event.clientY - bounds.top) / bounds.height);
    return {
      pan: clampDmxValue(x * 65_535),
      tilt: clampDmxValue((1 - y) * 65_535),
    };
  };

  const setMovementLimitRange = (anchor: MovementLimitPoint, point: MovementLimitPoint) => {
    setSelectedFixtureLimitsDraft((limits) => ({
      ...limits,
      pan_min: Math.min(anchor.pan, point.pan),
      pan_max: Math.max(anchor.pan, point.pan),
      tilt_min: Math.min(anchor.tilt, point.tilt),
      tilt_max: Math.max(anchor.tilt, point.tilt),
    }));
  };

  const startMovementLimitDrag = (event: PointerEvent & { currentTarget: HTMLElement }) => {
    const anchor = movementLimitPointFromPointer(event);
    event.currentTarget.setPointerCapture(event.pointerId);
    setMovementLimitDrag({ anchor });
    setMovementLimitRange(anchor, anchor);
  };

  const dragMovementLimit = (event: PointerEvent & { currentTarget: HTMLElement }) => {
    const drag = movementLimitDrag();
    if (!drag || event.buttons !== 1) {
      return;
    }
    setMovementLimitRange(drag.anchor, movementLimitPointFromPointer(event));
  };

  const endMovementLimitDrag = (event: PointerEvent & { currentTarget: HTMLElement }) => {
    const drag = movementLimitDrag();
    if (drag) {
      setMovementLimitRange(drag.anchor, movementLimitPointFromPointer(event));
    }
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    setMovementLimitDrag(null);
  };

  const setFixtureLimits = async (fixture: PatchedFixtureSummary) => {
    const limits = normalizedSelectedFixtureLimitsDraft();
    try {
      await invoke("set_fixture_limits", {
        fixtureId: fixture.id,
        limits,
      });
      setSelectedFixtureLimitsDraft(limits);
      setMessage(`Updated limits for ${fixture.label}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const savePreset = async () => {
    const fixture = selectedFixture();
    if (!fixture) {
      setMessage("Select a fixture first.");
      return;
    }
    try {
      const path = await invoke<string | null>("save_fixture_preset", { fixtureId: fixture.id });
      setMessage(path ? `Saved preset ${path}` : "Preset save canceled.");
    } catch (error) {
      setMessage(String(error));
    }
  };

  const resetSelectedDimmerLimits = () => {
    setSelectedFixtureLimitsDraft((limits) => ({
      ...limits,
      dimmer_min: defaultFixtureLimits.dimmer_min,
      dimmer_max: defaultFixtureLimits.dimmer_max,
    }));
  };

  const resetSelectedMovementLimits = () => {
    setSelectedFixtureLimitsDraft((limits) => ({
      ...limits,
      pan_min: defaultFixtureLimits.pan_min,
      pan_max: defaultFixtureLimits.pan_max,
      tilt_min: defaultFixtureLimits.tilt_min,
      tilt_max: defaultFixtureLimits.tilt_max,
      invert_pan: defaultFixtureLimits.invert_pan,
      invert_tilt: defaultFixtureLimits.invert_tilt,
      swap_pan_tilt: defaultFixtureLimits.swap_pan_tilt,
    }));
  };

  const applySelectedFixtureLimits = () => {
    const fixture = selectedFixture();
    if (!fixture) {
      setMessage("Select a fixture before applying limits.");
      return;
    }
    void setFixtureLimits(fixture);
  };

  const newProject = async () => {
    if (!confirmDiscardProjectChanges("create a new project")) {
      setMessage("New project canceled.");
      return;
    }
    try {
      await invoke("new_project");
      await resetProjectHistory();
      setCurrentProjectPath(null);
      setWorkspaceTab("setup");
      setSetupSubTab("patch");
      setMessage("Created new untitled project.");
      const next = await refreshSnapshot();
      if (next) {
        markProjectClean(next);
      }
      clearProjectRecovery();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const saveUserTemplate = async () => {
    try {
      const path = await invoke<string | null>("save_user_template", {
        midiMappings: midiMappings(),
        oscMappings: oscMappings(),
      });
      setMessage(path ? `Saved user template ${path}` : "Template save canceled.");
    } catch (error) {
      setMessage(`Template save failed: ${String(error)}`);
    }
  };

  const loadUserTemplate = async () => {
    if (!confirmDiscardProjectChanges("create a project from a user template")) {
      setMessage("Template load canceled.");
      return;
    }
    try {
      const result = await invoke<UserTemplateLoadResult | null>("load_user_template");
      if (!result) {
        setMessage("Template load canceled.");
        return;
      }
      if (midiControlConnected()) {
        await invoke("disconnect_midi_control");
        setMidiControlConnected(false);
      }
      if (oscRunning()) {
        await invoke("stop_osc_input");
        setOscRunning(false);
      }
      setMidiMappings(result.midi_mappings);
      setOscMappings(result.osc_mappings);
      await resetProjectHistory();
      setCurrentProjectPath(null);
      setWorkspaceTab("setup");
      setSetupSubTab("patch");
      const next = await refreshSnapshot();
      if (next) {
        setCleanProjectSignature("__syndocal_template_unsaved__");
        setProjectDirty(true);
      }
      clearProjectRecovery();
      setMessage(
        `Created an unsaved project from ${result.label} (${result.profiles.length} embedded profiles, ${result.midi_mappings.length} MIDI, ${result.osc_mappings.length} OSC mappings). All DMX and video outputs are disabled and blacked out.`,
      );
    } catch (error) {
      setMessage(`Template load failed: ${String(error)}`);
    }
  };

  const saveProject = async () => {
    try {
      const path = await invoke<string | null>("save_project");
      if (path) {
        setCurrentProjectPath(path);
        rememberRecentProjectPath(path);
        const next = await refreshSnapshot();
        if (next) {
          markProjectClean(next);
        }
        clearProjectRecovery();
      }
      setMessage(path ? `Saved project ${path}` : "Project save canceled.");
    } catch (error) {
      setMessage(String(error));
    }
  };

  const saveProjectAs = async () => {
    try {
      const path = await invoke<string | null>("save_project_as");
      if (path) {
        setCurrentProjectPath(path);
        rememberRecentProjectPath(path);
        const next = await refreshSnapshot();
        if (next) {
          markProjectClean(next);
        }
        clearProjectRecovery();
      }
      setMessage(path ? `Saved project ${path}` : "Project save canceled.");
    } catch (error) {
      setMessage(String(error));
    }
  };

  const loadedProjectMessage = (result: ProjectLoadResult) => {
    const profileLabel = result.profiles.length === 1 ? "1 embedded profile" : `${result.profiles.length} embedded profiles`;
    return `Loaded project ${result.path} (${profileLabel})`;
  };

  const applyLoadedProjectResult = async (result: ProjectLoadResult, currentPath: string | null) => {
    await resetProjectHistory();
    setCurrentProjectPath(currentPath);
    rememberRecentProjectPath(currentPath);
    setMessage(loadedProjectMessage(result));
    const next = await refreshSnapshot();
    if (next) {
      markProjectClean(next);
    }
    clearProjectRecovery();
  };

  const loadProject = async () => {
    if (!confirmDiscardProjectChanges("load another project")) {
      setMessage("Project load canceled.");
      return;
    }
    try {
      const result = await invoke<ProjectLoadResult | null>("load_project");
      if (!result) {
        setMessage("Project load canceled.");
        return;
      }
      await applyLoadedProjectResult(result, result.path);
    } catch (error) {
      setMessage(String(error));
    }
  };

  const loadProjectPath = async (path: string) => {
    if (!confirmDiscardProjectChanges(`open ${path}`)) {
      setMessage("Project open canceled.");
      return;
    }
    try {
      const result = await invoke<ProjectLoadResult>("load_project_path", { path });
      await applyLoadedProjectResult(result, result.path);
    } catch (error) {
      setMessage(String(error));
    }
  };

  const loadRecentProject = (path: string) => {
    void loadProjectPath(path);
  };

  const loadProjectRecovery = async () => {
    const checkpoint = projectRecoveryCheckpoint();
    if (!checkpoint) {
      setMessage("No recovery checkpoint is available.");
      return;
    }
    if (!confirmDiscardProjectChanges("recover the autosaved project")) {
      setMessage("Project recovery canceled.");
      return;
    }
    try {
      const result = await invoke<ProjectLoadResult>("load_project_checkpoint", {
        project: checkpoint.project,
        label: `Recovery ${checkpoint.saved_at}`,
        currentPath: checkpoint.source_path,
      });
      await resetProjectHistory();
      setCurrentProjectPath(checkpoint.source_path);
      setMessage(
        `Recovered ${projectRecoverySourceLabel(checkpoint)} from ${projectRecoveryTimeLabel(checkpoint)} (${result.profiles.length} embedded profiles). Save to keep it.`,
      );
      const next = await refreshSnapshot();
      if (next) {
        setCleanProjectSignature("__syndocal_recovered_unsaved__");
        setProjectDirty(true);
      }
      clearProjectRecovery();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const discardProjectRecovery = () => {
    clearProjectRecovery();
    setMessage("Recovery checkpoint discarded.");
  };

  const loadProjectBackup = async (backup: ProjectBackupSummary) => {
    if (!confirmDiscardProjectChanges("restore a project backup")) {
      setMessage("Project backup restore canceled.");
      return;
    }
    try {
      const result = await invoke<ProjectLoadResult>("load_project_backup", { backupId: backup.id });
      await resetProjectHistory();
      setCurrentProjectPath(backup.source_path ?? null);
      setMessage(
        `Restored backup from ${new Date(backup.created_at_unix_ms).toLocaleString()}. Save the project to keep it.`,
      );
      const next = await refreshSnapshot();
      if (next) {
        setCleanProjectSignature("__syndocal_backup_recovered_unsaved__");
        setProjectDirty(true);
      }
      clearProjectRecovery();
      if (result.profiles.length > 0) {
        setMessage(
          `Restored backup from ${new Date(backup.created_at_unix_ms).toLocaleString()} (${result.profiles.length} embedded profiles). Save to keep it.`,
        );
      }
    } catch (error) {
      setMessage(`Project backup restore failed: ${String(error)}`);
    }
  };

  const deleteProjectBackup = async (backup: ProjectBackupSummary) => {
    if (!window.confirm(`Delete the backup from ${new Date(backup.created_at_unix_ms).toLocaleString()}?`)) {
      return;
    }
    try {
      await invoke("delete_project_backup", { backupId: backup.id });
      await refreshProjectBackups();
      setMessage("Project backup deleted.");
    } catch (error) {
      setMessage(`Unable to delete project backup: ${String(error)}`);
    }
  };

  const exportDiagnosticPackage = async () => {
    try {
      const path = await invoke<string | null>("export_diagnostic_package");
      setMessage(path ? `Exported diagnostic package ${path}` : "Diagnostic export canceled.");
    } catch (error) {
      setMessage(`Diagnostic export failed: ${String(error)}`);
    }
  };

  const loadStartupProject = async () => {
    try {
      const result = await invoke<ProjectLoadResult | null>("load_startup_project");
      if (result) {
        await applyLoadedProjectResult(result, result.path);
      }
    } catch (error) {
      setMessage(String(error));
    }
  };

  const loadQueuedOpenProjects = async () => {
    try {
      const paths = await invoke<string[]>("take_open_project_paths");
      const path = paths.at(-1);
      if (path) {
        await loadProjectPath(path);
      }
    } catch (error) {
      setMessage(String(error));
    }
  };

  const loadPhase1SampleProject = async () => {
    if (!confirmDiscardProjectChanges("load the Phase 1 sample project")) {
      setMessage("Sample project load canceled.");
      return;
    }
    try {
      const result = await invoke<ProjectLoadResult>("load_phase1_sample_project");
      await resetProjectHistory();
      setPhase1SmokeReport(null);
      setCurrentProjectPath(null);
      setWorkspaceTab("setup");
      setSetupSubTab("patch");
      setMessage(loadedProjectMessage(result));
      const next = await refreshSnapshot();
      if (next) {
        markProjectClean(next);
      }
      clearProjectRecovery();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const runPhase1Smoke = async () => {
    try {
      const report = await invoke<Phase1SmokeReport>("run_phase1_smoke");
      await resetProjectHistory();
      setPhase1SmokeReport(report);
      setCurrentProjectPath(null);
      setWorkspaceTab("setup");
      setSetupSubTab("video");
      setRawDmxUniverse(0);
      setDmxTestChannel(1);
      setDmxTestWidth(8);
      setDmxTestValue(255);
      const afterSmoke = await refreshSnapshot();
      if (afterSmoke) {
        markProjectClean(afterSmoke);
      }
      clearProjectRecovery();
      const values = report.first_8.map((value) => value.toString(16).padStart(2, "0").toUpperCase()).join(" ");
      const expected = report.expected_first_8.map((value) => value.toString(16).padStart(2, "0").toUpperCase()).join(" ");
      setMessage(
        report.passed
          ? `Smoke passed: ${report.path}, ${report.cue_label}, ${report.primary_output_label}, U0 A1-A8 ${values}.`
          : `Smoke failed: ${report.primary_output_label}, active cue ${report.active_cue_id ?? "none"}, got ${values}, expected ${expected}.`,
      );
    } catch (error) {
      setMessage(String(error));
    }
  };

  const loadPreset = async () => {
    const fixture = selectedFixture();
    if (!fixture) {
      setMessage("Select a fixture first.");
      return;
    }
    try {
      const path = await invoke<string | null>("load_fixture_preset", { fixtureId: fixture.id });
      if (path) {
        await refreshSnapshot();
        setMessage(`Loaded preset ${path}`);
      } else {
        setMessage("Preset load canceled.");
      }
    } catch (error) {
      setMessage(String(error));
    }
  };

  const fixturePresetGroupLoadMessage = (result: FixturePresetGroupLoadResult, scope: string) =>
    `Loaded preset ${result.path} to ${scope}: ${result.applied_count} applied, ${result.skipped_count} skipped.`;

  const loadPresetForSelectedGroup = async (groupId: string) => {
    if (!groupId) {
      setMessage("Select a group first.");
      return;
    }
    try {
      const result = await invoke<FixturePresetGroupLoadResult | null>("load_fixture_preset_for_group", { groupId });
      if (result) {
        await refreshSnapshot();
        setMessage(fixturePresetGroupLoadMessage(result, `group ${groupId}`));
      } else {
        setMessage("Group preset load canceled.");
      }
    } catch (error) {
      setMessage(String(error));
    }
  };

  const loadPresetForAllMatching = async () => {
    try {
      const result = await invoke<FixturePresetGroupLoadResult | null>("load_fixture_preset_for_all_matching");
      if (result) {
        await refreshSnapshot();
        setMessage(fixturePresetGroupLoadMessage(result, "matching fixtures"));
      } else {
        setMessage("Matching preset load canceled.");
      }
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setFixtureColor = async (hexColor: string) => {
    const fixture = selectedControlReferenceFixture();
    const controls = selectedColorControls();
    if (!fixture || !controls) {
      setMessage(selectedFixtureGroupFilter() ? "Selected group has no common RGB color controls." : "Selected fixture has no RGB color controls.");
      return;
    }
    const red = Number.parseInt(hexColor.slice(1, 3), 16) * 257;
    const green = Number.parseInt(hexColor.slice(3, 5), 16) * 257;
    const blue = Number.parseInt(hexColor.slice(5, 7), 16) * 257;
    const updates = [
      { attribute: controls.red, value: red },
      { attribute: controls.green, value: green },
      { attribute: controls.blue, value: blue },
    ];
    const whiteChannel = controls.extras.find((extra) => extra.key === "white");
    if (colorAutoWhite() && whiteChannel) {
      updates.push({ attribute: whiteChannel.attribute, value: Math.min(red, green, blue) });
    }
    const groupId = selectedFixtureGroupFilter();
    const fixtureIds = groupId ? selectedControlTargetFixtures().map((candidate) => candidate.id) : [fixture.id];
    setFaderValues((current) => {
      const next = { ...current };
      for (const fixtureId of fixtureIds) {
        for (const update of updates) {
          next[`${fixtureId}:${update.attribute}`] = update.value;
        }
      }
      return next;
    });

    try {
      for (const update of updates) {
        if (groupId) {
          await invoke("set_group_attribute", {
            groupId,
            attribute: update.attribute,
            value: update.value,
          });
        } else {
          await invoke("set_attribute", {
            fixtureId: fixture.id,
            attribute: update.attribute,
            value: update.value,
          });
        }
      }
      setMessage(`Set ${groupId ? `group ${groupId}` : fixture.label} color ${hexColor.toUpperCase()}`);
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setPanTiltFromPointer = async (event: PointerEvent) => {
    const fixture = selectedControlReferenceFixture();
    const controls = selectedPositionControls();
    if (!fixture || !controls) {
      return;
    }
    const bounds = (event.currentTarget as HTMLElement).getBoundingClientRect();
    const x = clamp01((event.clientX - bounds.left) / bounds.width);
    const y = clamp01((event.clientY - bounds.top) / bounds.height);
    const panValue = Math.round(x * 65535);
    const tiltValue = Math.round((1 - y) * 65535);
    const sourceValues = sourcePanTiltValues(fixture, panValue, tiltValue);
    const groupId = selectedFixtureGroupFilter();
    if (groupId) {
      await setGroupAttribute(groupId, controls.pan, sourceValues.pan);
      await setGroupAttribute(groupId, controls.tilt, sourceValues.tilt);
    } else {
      await setAttribute(fixture.id, controls.pan, sourceValues.pan);
      await setAttribute(fixture.id, controls.tilt, sourceValues.tilt);
    }
  };

  const setPanTiltValues = async (panValue: number, tiltValue: number) => {
    const fixture = selectedControlReferenceFixture();
    const controls = selectedPositionControls();
    if (!fixture || !controls) {
      setMessage(selectedFixtureGroupFilter() ? "Selected group has no common Pan/Tilt controls." : "Selected fixture has no Pan/Tilt controls.");
      return;
    }
    const { pan: nextPan, tilt: nextTilt } = sourcePanTiltValues(fixture, panValue, tiltValue);
    const groupId = selectedFixtureGroupFilter();
    if (groupId) {
      await setGroupAttribute(groupId, controls.pan, nextPan);
      await setGroupAttribute(groupId, controls.tilt, nextTilt);
    } else {
      await setAttribute(fixture.id, controls.pan, nextPan);
      await setAttribute(fixture.id, controls.tilt, nextTilt);
    }
  };

  const setPanTiltPercent = (axis: "pan" | "tilt", percent: number) => {
    const controls = selectedPositionControls();
    if (!controls) {
      return;
    }
    const value = percentToDmxValue(percent);
    void setPanTiltValues(
      axis === "pan" ? value : controls.panValue,
      axis === "tilt" ? value : controls.tiltValue,
    );
  };

  const centerPanTilt = () => {
    const fixture = selectedFixture();
    if (!fixture) {
      void setPanTiltValues(32768, 32768);
      return;
    }
    const limits = fixture.limits ?? defaultFixtureLimits;
    const panRange = normalizeLimitRange(limits.pan_min, limits.pan_max);
    const tiltRange = normalizeLimitRange(limits.tilt_min, limits.tilt_max);
    void setPanTiltValues((panRange.min + panRange.max) / 2, (tiltRange.min + tiltRange.max) / 2);
  };

  const nudgePanTilt = (deltaPan: number, deltaTilt: number) => {
    const controls = selectedPositionControls();
    if (!controls) {
      return;
    }
    void setPanTiltValues(controls.panValue + deltaPan, controls.tiltValue + deltaTilt);
  };

  const setPanTiltTargetPoint = (panRatio: number, tiltRatio: number) => {
    const limits = selectedFixtureLimits();
    const panRange = normalizeLimitRange(limits.pan_min, limits.pan_max);
    const tiltRange = normalizeLimitRange(limits.tilt_min, limits.tilt_max);
    void setPanTiltValues(
      panRange.min + (panRange.max - panRange.min) * clamp01(panRatio),
      tiltRange.min + (tiltRange.max - tiltRange.min) * clamp01(tiltRatio),
    );
  };

  const mirrorPanTiltAxis = (axis: "pan" | "tilt" | "both") => {
    const controls = selectedPositionControls();
    if (!controls) {
      return;
    }
    const limits = selectedFixtureLimits();
    const panRange = normalizeLimitRange(limits.pan_min, limits.pan_max);
    const tiltRange = normalizeLimitRange(limits.tilt_min, limits.tilt_max);
    const panValue = axis === "pan" || axis === "both" ? panRange.min + panRange.max - controls.panValue : controls.panValue;
    const tiltValue = axis === "tilt" || axis === "both" ? tiltRange.min + tiltRange.max - controls.tiltValue : controls.tiltValue;
    void setPanTiltValues(panValue, tiltValue);
  };

  const handlePanTiltPadKeyDown = (event: KeyboardEvent) => {
    const step = event.shiftKey ? panTiltNudgeAmount() * 4 : event.altKey ? Math.max(1, Math.round(panTiltNudgeAmount() / 4)) : panTiltNudgeAmount();
    if (event.key === "ArrowLeft") {
      event.preventDefault();
      nudgePanTilt(-step, 0);
      return;
    }
    if (event.key === "ArrowRight") {
      event.preventDefault();
      nudgePanTilt(step, 0);
      return;
    }
    if (event.key === "ArrowUp") {
      event.preventDefault();
      nudgePanTilt(0, step);
      return;
    }
    if (event.key === "ArrowDown") {
      event.preventDefault();
      nudgePanTilt(0, -step);
      return;
    }
    if (event.key === "Home" || event.key === "c" || event.key === "C") {
      event.preventDefault();
      centerPanTilt();
    }
  };

  const addCurrentPositionFavorite = () => {
    const controls = selectedPositionControls();
    if (!controls) {
      setMessage("Selected fixture has no Pan/Tilt controls.");
      return;
    }
    const label = positionFavoriteLabel().trim() || `P${positionFavorites().length + 1}`;
    const favorite = {
      id: `position-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 7)}`,
      label,
      pan: clampDmxValue(controls.panValue),
      tilt: clampDmxValue(controls.tiltValue),
    };
    setPositionFavorites((current) => [
      favorite,
      ...current.filter((candidate) => candidate.pan !== favorite.pan || candidate.tilt !== favorite.tilt),
    ].slice(0, 24));
    setPositionFavoriteLabel("");
    setMessage(`Stored position ${favorite.label} (${formatShortDmxPercent(favorite.pan)} / ${formatShortDmxPercent(favorite.tilt)})`);
  };

  const removePositionFavorite = (favoriteId: string) => {
    setPositionFavorites((current) => current.filter((favorite) => favorite.id !== favoriteId));
  };

  const resetPositionFavorites = () => {
    setPositionFavorites(defaultPositionFavorites());
    setMessage("Reset position favorites.");
  };

  const setFixtureColorFromHsv = (hue: number, saturation: number, value: number) => {
    const { red, green, blue } = hsvToRgb(hue, saturation, value);
    void setFixtureColor(rgbToHex(red, green, blue));
  };

  const setColorHsvValue = (updates: Partial<{ hue: number; saturation: number; value: number }>) => {
    const hsv = selectedColorHsv();
    setFixtureColorFromHsv(
      updates.hue ?? hsv.hue,
      updates.saturation ?? hsv.saturation,
      updates.value ?? hsv.value,
    );
  };

  const setColorChannelValue = (channel: "red" | "green" | "blue", value: number) => {
    const current = selectedColorChannelValues();
    const next = {
      ...current,
      [channel]: clampDmxValue(value),
    };
    void setFixtureColor(`#${valueToHexByte(next.red)}${valueToHexByte(next.green)}${valueToHexByte(next.blue)}`);
  };

  const setColorExtraChannelValue = (extra: ColorExtraControl, value: number) => {
    const fixture = selectedControlReferenceFixture();
    if (!fixture) {
      setMessage(selectedFixtureGroupFilter() ? "Selected group has no common extra color channel." : "Select a fixture before setting an extra color channel.");
      return;
    }
    const nextValue = clampDmxValue(value);
    const groupId = selectedFixtureGroupFilter();
    if (groupId) {
      void setGroupAttribute(groupId, extra.attribute, nextValue);
    } else {
      void setAttribute(fixture.id, extra.attribute, nextValue);
    }
    setMessage(`Set ${groupId ? `group ${groupId}` : fixture.label} ${extra.label} ${formatShortDmxPercent(nextValue)}`);
  };

  const setColorFromPointer = (event: PointerEvent) => {
    const bounds = (event.currentTarget as HTMLElement).getBoundingClientRect();
    const x = clamp01((event.clientX - bounds.left) / bounds.width);
    const y = clamp01((event.clientY - bounds.top) / bounds.height);
    const current = selectedColorHsv();
    setFixtureColorFromHsv(x * 360, current.saturation > 0.05 ? current.saturation : 1, 1 - y);
  };

  const addCurrentColorFavorite = () => {
    const color = normalizeHexColor(selectedColorControls()?.value);
    if (!color) {
      return;
    }
    setColorFavorites((current) => [color, ...current.filter((candidate) => candidate !== color)].slice(0, 12));
  };

  const removeColorFavorite = (color: string) => {
    setColorFavorites((current) => current.filter((candidate) => candidate !== color));
  };

  const resetColorFavorites = () => {
    setColorFavorites(defaultColorFavorites());
    setMessage("Reset color favorites.");
  };

  const setDimmerValue = (value: number) => {
    const fixture = selectedControlReferenceFixture();
    const control = selectedDimmerControl();
    if (!fixture || !control) {
      setMessage(selectedFixtureGroupFilter() ? "Selected group has no common dimmer control." : "Selected fixture has no dimmer control.");
      return;
    }
    const nextValue = dimmerValueWithinLimits(fixture, value);
    const groupId = selectedFixtureGroupFilter();
    if (groupId) {
      void setGroupAttribute(groupId, control.attribute, nextValue);
    } else {
      void setAttribute(fixture.id, control.attribute, nextValue);
    }
  };

  const applyTouchDimmerEntries = (
    entries: TouchDimmerQuickEntry[],
    valueForEntry: (entry: TouchDimmerQuickEntry) => number,
  ) => {
    for (const entry of entries) {
      void setAttribute(entry.fixtureId, entry.attribute, valueForEntry(entry));
    }
  };

  const setTouchDimmerQuickLevel = (level: "out" | "half" | "full") => {
    const target = selectedTouchDimmerTarget();
    if (!target) {
      setMessage("Selected fixture or group has no dimmer control.");
      return;
    }
    const label =
      level === "out" ? "Out" : level === "half" ? "Half" : "Full";
    applyTouchDimmerEntries(target.entries, (entry) =>
      level === "out" ? entry.outValue : level === "half" ? entry.halfValue : entry.fullValue,
    );
    setMessage(`${label} ${target.label} dimmer (${target.entries.length} fixture${target.entries.length === 1 ? "" : "s"})`);
  };

  const startTouchDimmerFlash = () => {
    if (touchDimmerRestore()) {
      return;
    }
    const target = selectedTouchDimmerTarget();
    if (!target) {
      setMessage("Selected fixture or group has no dimmer control.");
      return;
    }
    setTouchDimmerRestore({
      label: target.label,
      entries: target.entries.map((entry) => ({
        fixtureId: entry.fixtureId,
        attribute: entry.attribute,
        value: entry.value,
      })),
    });
    applyTouchDimmerEntries(target.entries, (entry) => entry.fullValue);
    setMessage(`Flash ${target.label} dimmer`);
  };

  const endTouchDimmerFlash = () => {
    const restore = touchDimmerRestore();
    if (!restore) {
      return;
    }
    setTouchDimmerRestore(null);
    for (const entry of restore.entries) {
      void setAttribute(entry.fixtureId, entry.attribute, entry.value);
    }
    setMessage(`Restored ${restore.label} dimmer`);
  };

  const handleTouchDimmerFlashKeyDown = (event: KeyboardEvent) => {
    if (event.key === " " || event.key === "Enter") {
      event.preventDefault();
      startTouchDimmerFlash();
    }
  };

  const handleTouchDimmerFlashKeyUp = (event: KeyboardEvent) => {
    if (event.key === " " || event.key === "Enter") {
      event.preventDefault();
      endTouchDimmerFlash();
    }
  };

  const setControlAttributeValue = (control: AttributeControl, value: number) => {
    const fixture = selectedControlReferenceFixture();
    if (!fixture) {
      setMessage(selectedFixtureGroupFilter() ? "Selected group has no compatible fixture for this control." : "Select a fixture before setting a control value.");
      return;
    }
    const nextValue = clampDmxValue(value);
    const groupId = selectedFixtureGroupFilter();
    if (groupId) {
      void setGroupAttribute(groupId, control.attribute, nextValue);
    } else {
      void setAttribute(fixture.id, control.attribute, nextValue);
    }
  };

  const setOpticsValueFromPointer = (event: PointerEvent, control: AttributeControl) => {
    const bounds = (event.currentTarget as HTMLElement).getBoundingClientRect();
    const ratio = clamp01((event.clientX - bounds.left) / Math.max(1, bounds.width));
    setControlAttributeValue(control, ratio * 65_535);
  };

  const setOpticsValueFromKey = (event: KeyboardEvent, control: AttributeControl, value: number) => {
    const step = event.shiftKey ? 4096 : event.altKey ? 64 : 1024;
    if (event.key === "ArrowLeft" || event.key === "ArrowDown") {
      event.preventDefault();
      setControlAttributeValue(control, value - step);
      return;
    }
    if (event.key === "ArrowRight" || event.key === "ArrowUp") {
      event.preventDefault();
      setControlAttributeValue(control, value + step);
      return;
    }
    if (event.key === "Home") {
      event.preventDefault();
      setControlAttributeValue(control, 0);
      return;
    }
    if (event.key === "End") {
      event.preventDefault();
      setControlAttributeValue(control, 65_535);
    }
  };

  const applyVisibleControlValues = async (mode: "zero" | "mid" | "full" | "default") => {
    const fixture = selectedFixture();
    const controls = visibleControls();
    if (!fixture || controls.length === 0) {
      return;
    }
    const groupId = selectedFixtureGroupFilter();
    for (const control of controls) {
      const value =
        mode === "default"
          ? control.default_value
          : mode === "full"
            ? 65535
            : mode === "mid"
              ? 32768
              : 0;
      if (groupId) {
        await setGroupAttribute(groupId, control.attribute, value);
      } else {
        await setAttribute(fixture.id, control.attribute, value);
      }
    }
    setMessage(
      `Set ${activeControlCategory()} ${mode} for ${groupId ? `group ${groupId}` : fixture.label} (${controls.length} attributes)`,
    );
  };

  const applyCategoryQuickLook = async (look: CategoryQuickLook) => {
    const fixture = selectedFixture();
    const controls = visibleControls();
    const category = activeControlCategory();
    if (!fixture || controls.length === 0) {
      return;
    }
    const groupId = selectedFixtureGroupFilter();
    for (const control of controls) {
      const value = quickLookValueForControl(category, look.id, control);
      if (groupId) {
        await setGroupAttribute(groupId, control.attribute, value);
      } else {
        await setAttribute(fixture.id, control.attribute, value);
      }
    }
    setMessage(
      `Applied ${category} look ${look.label} to ${groupId ? `group ${groupId}` : fixture.label} (${controls.length} attributes)`,
    );
  };

  const applyChannelFunction = async (
    control: AttributeControl,
    fn: NonNullable<AttributeControl["functions"]>[number],
  ) => {
    const fixture = selectedFixture();
    if (!fixture) {
      return;
    }
    const groupId = selectedFixtureGroupFilter();
    const value = channelFunctionValue(fn);
    if (groupId) {
      await setGroupAttribute(groupId, control.attribute, value);
    } else {
      await setAttribute(fixture.id, control.attribute, value);
    }
    setMessage(
      `Applied ${control.attribute} ${channelFunctionLabel(fn)} to ${groupId ? `group ${groupId}` : fixture.label}`,
    );
  };

  const setFixtureHighlight = async (fixtureId: number, enabled: boolean) => {
    try {
      await invoke("set_fixture_highlight", { fixtureId, enabled });
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setFixtureSolo = async (fixtureId: number, enabled: boolean) => {
    try {
      await invoke("set_fixture_solo", { fixtureId, enabled });
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setFixturePark = async (fixtureId: number, enabled: boolean) => {
    try {
      await invoke("set_fixture_park", { fixtureId, enabled });
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setGroupHighlight = async (groupId: string, enabled: boolean) => {
    try {
      await invoke("set_group_highlight", { groupId, enabled });
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setGroupSolo = async (groupId: string, enabled: boolean) => {
    try {
      await invoke("set_group_solo", { groupId, enabled });
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setGroupPark = async (groupId: string, enabled: boolean) => {
    try {
      await invoke("set_group_park", { groupId, enabled });
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const clearFixtureFlags = async (kind: FixtureFlagClearKind) => {
    try {
      await invoke("clear_fixture_flags", { kind });
      setMessage(kind === "all" ? "Cleared all fixture flags." : `Cleared fixture ${kind}.`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setBlackout = async (enabled: boolean) => {
    try {
      await invoke("set_blackout", { enabled });
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setAllBlackout = async (enabled: boolean) => {
    try {
      await invoke("set_all_blackout", { enabled });
      setMessage(enabled ? "All blackout enabled." : "All blackout cleared.");
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setLightingMaster = async (master: number) => {
    try {
      await invoke("set_lighting_master", { master });
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setGroupSubmaster = async (groupId: string, level: number) => {
    try {
      await invoke("set_group_submaster", { groupId, level });
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const applyBpm = async () => {
    const bpm = Number(bpmDraft());
    try {
      await invoke("set_bpm", { bpm });
      setMessage(`BPM set to ${bpm.toFixed(1)}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const tapBpm = async () => {
    try {
      await invoke("tap_bpm");
      await refreshSnapshot();
      setBpmDraft(snapshot().clock.bpm.toFixed(1));
      setMessage(`Tapped BPM ${snapshot().clock.bpm.toFixed(1)}`);
    } catch (error) {
      setMessage(String(error));
    }
  };

  const analyzeAudioFile = async () => {
    try {
      const analysis = await invoke<AudioAnalysisSummary | null>("analyze_audio_file");
      if (!analysis) {
        setMessage("Audio analysis canceled.");
        return;
      }
      if (analysis.estimated_bpm) {
        setBpmDraft(analysis.estimated_bpm.toFixed(1));
      }
      await refreshSnapshot();
      setMessage(
        `Analyzed audio ${Math.round(analysis.duration_ms / 1000)}s${
          analysis.estimated_bpm ? ` / ${analysis.estimated_bpm.toFixed(1)} BPM` : ""
        }`,
      );
    } catch (error) {
      setMessage(String(error));
    }
  };

  const applyAudioBpm = async () => {
    const bpm = audioAnalysis()?.estimated_bpm;
    if (!bpm) {
      setMessage("Analyze audio with a BPM estimate first.");
      return;
    }
    try {
      await invoke("set_bpm", { bpm });
      await refreshSnapshot();
      setBpmDraft(bpm.toFixed(1));
      setMessage(`Applied audio BPM ${bpm.toFixed(1)}`);
    } catch (error) {
      setMessage(String(error));
    }
  };

  const clearTimelineAudio = async () => {
    try {
      await invoke("clear_timeline_audio");
      await refreshSnapshot();
      setMessage("Cleared audio analysis.");
    } catch (error) {
      setMessage(String(error));
    }
  };

  const {
    refreshMidiInputs,
    refreshMidiOutputs,
    connectMidiClock,
    disconnectMidiClock,
    addMidiMapping,
    removeMidiMapping,
    learnMidiControl,
    saveMidiMappings,
    loadMidiMappings,
    connectMidiControl,
    disconnectMidiControl,
    connectMidiFeedback,
    disconnectMidiFeedback,
    sendMidiFeedback,
    addOscMapping,
    removeOscMapping,
    saveOscMappings,
    loadOscMappings,
    learnOscControl,
    startOscInput,
    stopOscInput,
  } = createControlInputController({
    invoke,
    setMessage,
    selectedFixture,
    selectedEffectAttribute,
    setMidiInputs,
    setMidiOutputs,
    selectedMidiInput,
    setSelectedMidiInput,
    selectedMidiOutput,
    setSelectedMidiOutput,
    setMidiConnected,
    setMidiControlConnected,
    midiFeedbackConnected,
    setMidiFeedbackConnected,
    midiFeedbackEnabled,
    setMidiFeedbackEnabled,
    midiMappings,
    setMidiMappings,
    midiMapMessage,
    setMidiMapMessage,
    midiMapChannel,
    setMidiMapChannel,
    midiMapNumber,
    setMidiMapNumber,
    midiMapAction,
    midiMapAttribute,
    midiMapGroupId,
    midiClearFixtureFlagKind,
    selectedMidiCueId,
    selectedMidiEffectId,
    selectedMidiNodeGraphId,
    selectedMidiLayerId,
    selectedMidiVideoOutputId,
    selectedMidiVideoOutputMappingField,
    selectedMidiVideoOutputMappingPresetLabel,
    midiMapVideoParam,
    midiMapCuePointIndex,
    midiMapDurationMs,
    midiMapLow,
    midiMapHigh,
    oscRunning,
    setOscRunning,
    oscMappings,
    setOscMappings,
    oscBindIp,
    oscPort,
    oscMapAddress,
    setOscMapAddress,
    oscMapAction,
    oscMapAttribute,
    oscMapGroupId,
    oscClearFixtureFlagKind,
    selectedOscCueId,
    selectedOscEffectId,
    selectedOscNodeGraphId,
    selectedOscLayerId,
    selectedOscVideoOutputId,
    selectedOscVideoOutputMappingField,
    selectedOscVideoOutputMappingPresetLabel,
    oscMapVideoParam,
    oscMapCuePointIndex,
    oscMapDurationMs,
    oscMapLow,
    oscMapHigh,
  });

  const refreshRemoteAccessUrls = async (config: RemoteControlConfig = remoteConfig()) => {
    if (!isTauriRuntime()) {
      setRemoteAccessUrls([]);
      return [];
    }
    try {
      const urls = await invoke<string[]>("remote_access_urls", { config });
      setRemoteAccessUrls(urls);
      return urls;
    } catch {
      setRemoteAccessUrls([]);
      return [];
    }
  };

  const copyRemoteUrl = async (url: string) => {
    try {
      if (navigator.clipboard?.writeText) {
        await navigator.clipboard.writeText(url);
      } else {
        const textArea = document.createElement("textarea");
        textArea.value = url;
        textArea.setAttribute("readonly", "");
        textArea.style.position = "fixed";
        textArea.style.left = "-9999px";
        document.body.appendChild(textArea);
        textArea.select();
        document.execCommand("copy");
        document.body.removeChild(textArea);
      }
      setMessage(`Copied remote URL: ${url}`);
    } catch (error) {
      setMessage(`Could not copy remote URL: ${String(error)}`);
    }
  };

  const startRemoteServer = async (config: RemoteControlConfig = remoteConfig()) => {
    await invoke("start_remote_control", { config });
    const urls = await refreshRemoteAccessUrls(config);
    setRemoteRunning(true);
    await refreshRemoteControlStatus();
    return urls;
  };

  const refreshRemoteControlStatus = async () => {
    if (!isTauriRuntime()) {
      return remoteStatus();
    }
    try {
      const status = await invoke<RemoteControlStatus>("remote_control_status");
      setRemoteStatus(status);
      setRemoteRunning(status.running);
      return status;
    } catch {
      return remoteStatus();
    }
  };

  const disconnectRemoteClient = async (clientId: number) => {
    try {
      await invoke("disconnect_remote_client", { clientId });
      await refreshRemoteControlStatus();
      setMessage(`Disconnected remote client #${clientId}.`);
    } catch (error) {
      setMessage(String(error));
    }
  };

  const openRemoteUrl = async (url: string) => {
    let targetUrl = url;
    try {
      if (!remoteRunning() && isTauriRuntime()) {
        const urls = await startRemoteServer();
        targetUrl = urls.includes(url) ? url : urls[0] ?? url;
      }
      const opened = window.open(targetUrl, "_blank", "noopener,noreferrer");
      if (opened) {
        setMessage(`Opened remote URL: ${targetUrl}`);
      } else {
        setMessage(`Could not open remote URL. Copy it instead: ${targetUrl}`);
      }
    } catch (error) {
      setMessage(`Could not start remote before opening: ${String(error)}`);
    }
  };

  createEffect(() => {
    const config = remoteConfig();
    void refreshRemoteAccessUrls(config);
  });

  const startRemoteControl = async () => {
    try {
      if (!/^\d{6}$/.test(remotePairingPin())) {
        setMessage("Remote pairing PIN must contain exactly 6 digits.");
        return;
      }
      const urls = await startRemoteServer();
      setMessage(`Remote listening: ${(urls[0] ?? fallbackRemoteUrl())}`);
    } catch (error) {
      setMessage(String(error));
    }
  };

  const stopRemoteControl = async () => {
    try {
      await invoke("stop_remote_control");
      setRemoteRunning(false);
      setRemoteStatus({ running: false, active_connections: 0, rejected_connections: 0, clients: [] });
      void refreshRemoteAccessUrls();
      setMessage("Remote WebSocket stopped.");
    } catch (error) {
      setMessage(String(error));
    }
  };

  const remoteStatusTimer = isTauriRuntime()
    ? window.setInterval(() => {
        if (remoteRunning()) {
          void refreshRemoteControlStatus();
        }
      }, 1_000)
    : null;
  onCleanup(() => {
    if (remoteStatusTimer !== null) {
      window.clearInterval(remoteStatusTimer);
    }
  });

  const createCue = async () => {
    const scopeError = cueCaptureScopeError();
    if (scopeError) {
      setMessage(scopeError);
      return;
    }
    const captureScope = cueCaptureScopeRequest();
    if (!captureScope) {
      setMessage("Select a valid cue capture scope.");
      return;
    }
    try {
      const cueId = await invoke<number>("create_cue_from_current", {
        label: cueLabel(),
        fadeMs: cueFadeMs(),
        captureScope,
        cueListId: selectedCueList().id,
      });
      setCueLabel(`Cue ${snapshot().cues.length + 2}`);
      setMessage(`Created cue ${cueId}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const createCueList = async () => {
    const label = cueListLabel().trim();
    if (!label) {
      setMessage("Cue List label is required.");
      return;
    }
    try {
      const cueListId = await invoke<number>("create_cue_list", { label });
      await refreshSnapshot();
      setSelectedCueListId(cueListId);
      setCueListLabel(label);
      setMessage(`Created Cue List ${label}`);
    } catch (error) {
      setMessage(String(error));
    }
  };

  const renameCueList = async () => {
    try {
      await invoke("rename_cue_list", { cueListId: selectedCueList().id, label: cueListLabel() });
      setMessage(`Renamed Cue List ${selectedCueList().id}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const removeCueList = async () => {
    const cueList = selectedCueList();
    if (cueList.id === 1 || !confirmDestructiveAction("cue list", cueList.label)) return;
    try {
      await invoke("remove_cue_list", { cueListId: cueList.id });
      await refreshSnapshot();
      setSelectedCueListId(1);
      setMessage(`Removed Cue List ${cueList.label}; its cues moved to Main.`);
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setCueList = async (cueId: number, cueListId: number) => {
    try {
      await invoke("set_cue_list", { cueId, cueListId });
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setCuePalette = async (cue: CueSummary, paletteId: number, enabled: boolean) => {
    const withoutPalette = cue.palette_targets.filter((target) => target.palette_id !== paletteId);
    const fixtureIds = [...new Set(cue.targets.map((target) => target.fixture_id))].sort((left, right) => left - right);
    if (enabled && fixtureIds.length === 0) {
      setMessage("Store lighting fixture targets in this Cue before linking a palette.");
      return;
    }
    const paletteTargets = enabled
      ? [...withoutPalette, { palette_id: paletteId, fixture_ids: fixtureIds }]
      : withoutPalette;
    try {
      await invoke("set_cue_palette_targets", { cueId: cue.id, paletteTargets });
      setMessage(`${enabled ? "Linked" : "Unlinked"} palette ${paletteId} ${enabled ? "to" : "from"} Cue ${cue.cue_number || cue.id}.`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const triggerCueList = async (cueListId: number, direction: "next" | "previous") => {
    try {
      await invoke(direction === "next" ? "trigger_cue_list_next" : "trigger_cue_list_previous", { cueListId });
      setMessage(direction === "next" ? `GO List ${cueListId}` : `Back List ${cueListId}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const createPlaybackExecutor = async (label: string, cueListId: number, page: number, slot: number) => {
    try {
      const executorId = await invoke<number>("create_playback_executor", { label, cueListId, page, slot });
      setMessage(`Created Playback Executor ${executorId} on page ${page}, slot ${slot}.`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const updatePlaybackExecutor = async (
    executor: PlaybackExecutorSummary,
    patch: Partial<PlaybackExecutorSummary>,
  ) => {
    const next = { ...executor, ...patch };
    try {
      await invoke("update_playback_executor", {
        executorId: next.id,
        label: next.label,
        cueListId: next.cue_list_id,
        page: next.page,
        slot: next.slot,
      });
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const removePlaybackExecutor = async (executor: PlaybackExecutorSummary) => {
    if (!confirmDestructiveAction("playback executor", executor.label)) return;
    try {
      await invoke("remove_playback_executor", { executorId: executor.id });
      setMessage(`Removed Playback Executor ${executor.label}.`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setPlaybackExecutorLevel = async (executorId: number, level: number) => {
    try {
      await invoke("set_playback_executor_level", { executorId, level });
      await refreshSnapshot(false);
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setPlaybackMaster = async (level: number) => {
    try {
      await invoke("set_playback_master", { level });
      await refreshSnapshot(false);
    } catch (error) {
      setMessage(String(error));
    }
  };

  const triggerPlaybackExecutor = async (executorId: number, direction: "next" | "previous") => {
    try {
      await invoke("trigger_playback_executor", { executorId, direction });
      setMessage(`${direction === "next" ? "GO" : "Back"} Playback Executor ${executorId}.`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const triggerCue = async (cueId: number) => {
    try {
      await invoke("trigger_cue", { cueId });
      setMessage(`Triggered cue ${cueId}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const updateCue = async (cueId: number, label: string, fadeMs: number) => {
    const normalizedFadeMs = Math.max(0, Math.round(Number.isFinite(fadeMs) ? fadeMs : cueFadeMs()));
    const scopeError = cueCaptureScopeError();
    if (scopeError) {
      setMessage(scopeError);
      return;
    }
    const captureScope = cueCaptureScopeRequest();
    if (!captureScope) {
      setMessage("Select a valid cue capture scope.");
      return;
    }
    try {
      await invoke("update_cue_from_current", { cueId, label, fadeMs: normalizedFadeMs, captureScope });
      setMessage(`Updated cue ${cueId}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setCueMetadata = async (cue: CueSummary) => {
    const draft = cueMetadataDraft(cue);
    const fadeMs = Math.max(0, Math.round(Number.isFinite(draft.fade_ms) ? draft.fade_ms : cue.fade_ms));
    const preWaitMs = Math.max(0, Math.round(Number.isFinite(draft.pre_wait_ms) ? draft.pre_wait_ms : 0));
    const followMs = draft.follow_ms === null || !Number.isFinite(draft.follow_ms)
      ? null
      : Math.max(0, Math.round(draft.follow_ms));
    const normalizeOptionalTiming = (value: number | null | undefined) =>
      value === null || value === undefined || !Number.isFinite(value)
        ? null
        : Math.max(0, Math.round(value));
    const normalizeTiming = (value: number) => Math.max(0, Math.round(Number.isFinite(value) ? value : 0));
    const ifcbTiming = {
      intensity_fade_ms: normalizeOptionalTiming(draft.ifcb_timing.intensity_fade_ms),
      intensity_delay_ms: normalizeTiming(draft.ifcb_timing.intensity_delay_ms),
      focus_fade_ms: normalizeOptionalTiming(draft.ifcb_timing.focus_fade_ms),
      focus_delay_ms: normalizeTiming(draft.ifcb_timing.focus_delay_ms),
      color_fade_ms: normalizeOptionalTiming(draft.ifcb_timing.color_fade_ms),
      color_delay_ms: normalizeTiming(draft.ifcb_timing.color_delay_ms),
      beam_fade_ms: normalizeOptionalTiming(draft.ifcb_timing.beam_fade_ms),
      beam_delay_ms: normalizeTiming(draft.ifcb_timing.beam_delay_ms),
    };
    const parts = draft.parts.map((part) => ({
      number: Math.max(1, Math.min(999, Math.round(part.number))),
      label: part.label.trim(),
      delay_ms: Math.max(0, Math.round(Number.isFinite(part.delay_ms) ? part.delay_ms : 0)),
      fade_ms: part.fade_ms === null || part.fade_ms === undefined || !Number.isFinite(part.fade_ms)
        ? null
        : Math.max(0, Math.round(part.fade_ms)),
      fixture_ids: [...new Set(part.fixture_ids)].sort((left, right) => left - right),
      video_layer_ids: [...new Set(part.video_layer_ids)].sort((left, right) => left - right),
      video_output_ids: [...new Set(part.video_output_ids)].sort((left, right) => left - right),
    }));
    const mibFixtureIds = [...new Set(draft.mib_fixture_ids)].sort((left, right) => left - right);
    try {
      await invoke("set_cue_metadata", {
        cueId: cue.id,
        cueNumber: draft.cue_number,
        label: draft.label,
        fadeMs,
        preWaitMs,
        followMs,
        ifcbTiming,
        parts,
        mark: draft.mark,
        mibFixtureIds,
        tracking: draft.tracking,
        notes: draft.notes,
      });
      setCueMetadataDrafts((current) => ({
        ...current,
        [cue.id]: {
          cue_number: draft.cue_number,
          label: draft.label,
          fade_ms: fadeMs,
          pre_wait_ms: preWaitMs,
          follow_ms: followMs,
          ifcb_timing: ifcbTiming,
          parts,
          mark: draft.mark,
          mib_fixture_ids: mibFixtureIds,
          tracking: draft.tracking,
          notes: draft.notes,
        },
      }));
      setMessage(`Saved cue ${cue.id}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const moveCue = async (cueId: number, delta: -1 | 1) => {
    try {
      await invoke("move_cue", { cueId, delta });
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const duplicateCue = async (cue: CueSummary) => {
    const draft = cueMetadataDraft(cue);
    const baseLabel = draft.label.trim() || cue.label;
    try {
      const cueId = await invoke<number>("duplicate_cue", {
        sourceCueId: cue.id,
        label: `${baseLabel} Copy`,
      });
      setMessage(`Duplicated cue ${cue.id} as ${cueId}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const triggerNextCue = async () => {
    try {
      await invoke("trigger_next_cue");
      setMessage("GO");
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const triggerPreviousCue = async () => {
    try {
      await invoke("trigger_previous_cue");
      setMessage("Back");
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setCueFadePaused = async (paused: boolean) => {
    try {
      await invoke("set_cue_fade_paused", { paused });
      setMessage(paused ? "Cue fade paused." : "Cue fade resumed.");
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const removeCue = async (cueId: number) => {
    const cue = snapshot().cues.find((candidate) => candidate.id === cueId);
    if (cue && !confirmDestructiveAction("cue", cue.label)) {
      return;
    }
    try {
      await invoke("remove_cue", { cueId });
      setMessage(`Removed cue ${cueId}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const snapTimelineDrafts = () => {
    setTimelineEventTimeMs(snapTimeMs(timelineEventTimeMs()));
    setAutomationStartMs(snapTimeMs(automationStartMs()));
    setAutomationEndMs(snapTimeMs(automationEndMs()));
    setVideoAutomationStartMs(snapTimeMs(videoAutomationStartMs()));
    setVideoAutomationEndMs(snapTimeMs(videoAutomationEndMs()));
    setMessage(`Snapped timeline inputs to ${timelineSnapMode().toLowerCase()}.`);
  };

  const snapTimelineItems = async () => {
    if (timelineSnapMode() === "Off") {
      setMessage("Choose Beat, Bar, or Grid before snapping timeline items.");
      return;
    }
    const timeline = snapshot().timeline;
    const cueEventUpdates = timeline.events.map((event) => ({
      event,
      timeMs: snapTimeMs(event.time_ms),
    }));
    const lightingAutomationUpdates = timeline.automations.map((automation) => ({
      automation,
      keyframes: snappedTimelineKeyframes(automation.keyframes, snapTimeMs),
    }));
    const videoAutomationUpdates = timeline.video_automations.map((automation) => ({
      automation,
      keyframes: snappedTimelineKeyframes(automation.keyframes, snapTimeMs),
    }));
    const itemCount = cueEventUpdates.length + lightingAutomationUpdates.length + videoAutomationUpdates.length;
    if (itemCount === 0) {
      setMessage("No timeline items to snap.");
      return;
    }
    try {
      await Promise.all([
        ...cueEventUpdates.map(({ event, timeMs }) =>
          invoke("set_timeline_cue_event", {
            eventId: event.id,
            cueId: event.cue_id,
            timeMs,
            track: event.track,
          }),
        ),
        ...lightingAutomationUpdates.map(({ automation, keyframes }) =>
          invoke("set_timeline_automation", {
            automationId: automation.id,
            fixtureId: automation.fixture_id,
            attribute: automation.attribute,
            keyframes,
          }),
        ),
        ...videoAutomationUpdates.map(({ automation, keyframes }) =>
          invoke("set_timeline_video_automation", {
            automationId: automation.id,
            layerId: automation.layer_id,
            param: automation.param,
            keyframes,
          }),
        ),
      ]);
      setTimelineEventDrafts((current) => {
        const next = { ...current };
        for (const { event, timeMs } of cueEventUpdates) {
          next[event.id] = {
            cue_id: event.cue_id,
            time_ms: timeMs,
            track: event.track,
          };
        }
        return next;
      });
      setTimelineAutomationDrafts((current) => {
        const next = { ...current };
        for (const { automation, keyframes } of lightingAutomationUpdates) {
          next[automation.id] = {
            ...(current[automation.id] ?? timelineAutomationDraftFromSummary(automation)),
            ...draftRangeFromKeyframes(keyframes),
          };
        }
        return next;
      });
      setTimelineVideoAutomationDrafts((current) => {
        const next = { ...current };
        for (const { automation, keyframes } of videoAutomationUpdates) {
          next[automation.id] = {
            ...(current[automation.id] ?? timelineVideoAutomationDraftFromSummary(automation)),
            ...draftRangeFromKeyframes(keyframes),
          };
        }
        return next;
      });
      setMessage(`Snapped ${itemCount} timeline item(s) to ${timelineSnapMode().toLowerCase()}.`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const addTimelineCueEventAt = async (cueId: number | null, timeMs: number, track: TimelineTrackKind, nextDraftTime = true) => {
    if (cueId === null) {
      setMessage("Create a cue before adding timeline events.");
      return;
    }
    const snappedTimeMs = snapTimeMs(timeMs);
    try {
      const eventId = await invoke<number>("add_timeline_cue_event", {
        cueId,
        timeMs: snappedTimeMs,
        track,
      });
      if (nextDraftTime) {
        setTimelineEventTimeMs(snapTimeMs(snappedTimeMs + 1000));
      } else {
        setTimelineEventTimeMs(snappedTimeMs);
      }
      setMessage(`Added timeline event ${eventId} at ${snappedTimeMs} ms`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const addTimelineCueEvent = async () => {
    await addTimelineCueEventAt(selectedTimelineCueId(), timelineEventTimeMs(), timelineTrack());
  };

  const addTimelineCueEventAtPlayhead = async () => {
    await addTimelineCueEventAt(selectedTimelineCueId(), snapshot().timeline.position_ms, timelineTrack(), false);
  };

  const setTimelineCueEvent = async (event: TimelineCueEventSummary) => {
    const draft = timelineEventDraft(event);
    const cueId = Math.max(0, Math.round(draft.cue_id));
    const timeMs = snapTimeMs(draft.time_ms);
    try {
      await invoke("set_timeline_cue_event", {
        eventId: event.id,
        cueId,
        timeMs,
        track: draft.track,
      });
      setTimelineEventDrafts((current) => ({
        ...current,
        [event.id]: {
          cue_id: cueId,
          time_ms: timeMs,
          track: draft.track,
        },
      }));
      setMessage(`Saved timeline event ${event.id}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const moveTimelineCueEvent = async (event: TimelineCueEventSummary, deltaMs: number) => {
    const timeMs = snapTimeMs(Math.max(0, event.time_ms + deltaMs));
    try {
      await invoke("set_timeline_cue_event", {
        eventId: event.id,
        cueId: event.cue_id,
        timeMs,
        track: event.track,
      });
      setTimelineEventDrafts((current) => ({
        ...current,
        [event.id]: {
          cue_id: event.cue_id,
          time_ms: timeMs,
          track: event.track,
        },
      }));
      setMessage(`Moved timeline event ${event.id} to ${timeMs} ms`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const moveTimelineCueEventToRatio = async (eventId: number, ratio: number) => {
    const event = timelineEventRows().find((candidate) => candidate.id === eventId);
    if (!event) {
      setMessage(`Timeline event ${eventId} was not found.`);
      return;
    }
    const timeMs = snapTimeMs(Math.round(clampRange(ratio, 0, 1) * timelineOverviewDurationMs()));
    try {
      await invoke("set_timeline_cue_event", {
        eventId: event.id,
        cueId: event.cue_id,
        timeMs,
        track: event.track,
      });
      setTimelineEventDrafts((current) => ({
        ...current,
        [event.id]: {
          cue_id: event.cue_id,
          time_ms: timeMs,
          track: event.track,
        },
      }));
      setMessage(`Moved ${event.cue_label} to ${timeMs} ms`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const currentLightingAutomationValue = (automation: TimelineAutomationSummary) => {
    const fixture = snapshot().fixtures.find((candidate) => candidate.id === automation.fixture_id);
    if (!fixture) {
      return null;
    }
    const control = fixture?.controls.find((candidate) => candidate.attribute === automation.attribute);
    const fallbackValue = fixture?.attribute_values.find((value) => value.attribute === automation.attribute)?.value ?? control?.default_value;
    return faderValue(automation.fixture_id, automation.attribute, fallbackValue ?? 0);
  };

  const currentVideoAutomationValue = (automation: TimelineVideoAutomationSummary) => {
    const layer = snapshot().video.layers.find((candidate) => candidate.id === automation.layer_id);
    const state = layer?.state;
    if (!state) {
      return null;
    }
    return videoAutomationValueFromState(state, automation.param);
  };

  const selectTimelineAutomationRange = (range: TimelineOverviewAutomationRange) => {
    setSelectedTimelineAutomation({
      kind: range.kind,
      automationId: range.automation_id,
    });
  };

  const keyframeDeleteToleranceMs = () => Math.max(1, Math.min(100, Math.round(timelinePlacementNudgeMs() / 4)));

  const {
    moveTimelineAutomationRangeToRatio,
    resizeTimelineAutomationRangeToRatio,
    moveTimelineAutomationKeyframeToRatio,
  } = createTimelineOverviewAutomationController({
    snapshot,
    snapTimeMs,
    timelineOverviewDurationMs,
    invoke,
    setTimelineAutomationDrafts,
    setTimelineVideoAutomationDrafts,
    setMessage,
    refreshSnapshot,
  });

  const removeTimelineEvent = async (eventId: number) => {
    try {
      await invoke("remove_timeline_event", { eventId });
      setMessage(`Removed timeline event ${eventId}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const {
    usePlayheadForLightingAutomation,
    usePlayheadForVideoAutomation,
    alignLightingAutomationDraftToPlayhead,
    alignVideoAutomationDraftToPlayhead,
    addLightingAutomationKeyframeAtPlayhead,
    addVideoAutomationKeyframeAtPlayhead,
    removeLightingAutomationKeyframeAtPlayhead,
    removeLightingAutomationKeyframe,
    setLightingAutomationKeyframeInterpolation,
    setLightingAutomationKeyframeValue,
    removeVideoAutomationKeyframeAtPlayhead,
    removeVideoAutomationKeyframe,
    setVideoAutomationKeyframeInterpolation,
    setVideoAutomationKeyframeValue,
  } = createTimelineKeyframeController({
    snapshot,
    snapTimeMs,
    timelinePlacementNudgeMs,
    setAutomationStartMs,
    setAutomationEndMs,
    setVideoAutomationStartMs,
    setVideoAutomationEndMs,
    timelineAutomationDraft,
    timelineVideoAutomationDraft,
    updateTimelineAutomationDraft,
    updateTimelineVideoAutomationDraft,
    currentLightingAutomationValue,
    currentVideoAutomationValue,
    keyframeDeleteToleranceMs,
    invoke,
    setTimelineAutomationDrafts,
    setTimelineVideoAutomationDrafts,
    setMessage,
    refreshSnapshot,
  });

  const {
    setTimelineAutomationEnabled,
    setLightingAutomationRowsEnabled,
    setVideoAutomationRowsEnabled,
    addTimelineAutomation,
    addTimelineGroupAutomation,
    setTimelineAutomation,
    addTimelineVideoAutomation,
    setTimelineVideoAutomation,
    removeTimelineAutomation,
    playTimeline,
    pauseTimeline,
    seekTimeline,
    seekTimelineFromOverviewRatio,
  } = createTimelineAutomationController({
    snapshot,
    selectedFixture,
    selectedFixtureGroupFilter,
    selectedTimelineAutomationAttribute,
    selectedFixtureSupportsTimelineAutomationAttribute,
    automationStartMs,
    automationEndMs,
    automationStartValue,
    automationEndValue,
    automationInterpolation,
    selectedVideoAutomationLayerId,
    videoAutomationParam,
    videoAutomationStartMs,
    videoAutomationEndMs,
    videoAutomationStartValue,
    videoAutomationEndValue,
    videoAutomationInterpolation,
    timelineAutomationDraft,
    timelineVideoAutomationDraft,
    setTimelineAutomationDrafts,
    setTimelineVideoAutomationDrafts,
    snapTimeMs,
    timelineOverviewDurationMs,
    invoke,
    setMessage,
    refreshSnapshot,
  });

  const {
    addVideoLayer,
    importMediaFiles,
    launchVideoClip,
    takeVideoClip,
    stopVideoClip,
    playVideoLayerAudioMonitor,
    stopVideoLayerAudioMonitor,
    setVideoLayerAudioMonitorVolume,
    refreshVideoAudioMonitorStatus,
    refreshAudioOutputDevices,
    startVideoOutputRecording,
    stopVideoOutputRecording,
    refreshVideoRecordingStatus,
    removeVideoLayer,
    duplicateVideoLayer,
    moveVideoLayer,
    setVideoLayerLabel,
    refreshVideoLayerMetadata,
    importVideoLayerIsf,
    setVideoLayerIsfEffect,
    renderDebugVideoPreview,
    loadVideoLayerThumbnail,
    refreshVideoPreviewDiagnostics,
    refreshVideoOutputRenderPlans,
    refreshVideoOutputWindowStatuses,
    refreshSnapshotAndVideoOutputRenderPlans,
    syncOpenVideoOutputWindows,
    closeOpenVideoOutputWindows,
    openAllVideoOutputWindows,
    refreshVideoRuntimeStatus,
    refreshExternalVideoIoPlans,
    syncExternalVideoTransports,
    renderDebugVideoOutputPreview,
    setVideoLayerState,
    setVideoLayerTransform,
    setVideoLayerColor,
    setVideoLayerFx,
    addVideoCuePoint,
    removeVideoCuePoint,
    jumpVideoCuePoint,
    setVideoLayerBlendMode,
    setVideoMasterOpacity,
    setVideoBlackout,
  } = createVideoRuntimeController({
    invoke,
    snapshot,
    refreshSnapshot,
    setMessage,
    videoSourceKind,
    videoLabel,
    setVideoLabel,
    videoPath,
    setVideoPreviewUrl,
    setVideoPreviewInfo,
    setVideoPreviewDiagnostics,
    setVideoOutputRenderPlans,
    setVideoOutputWindowStatuses,
    setVideoRuntimeStatus,
    setVideoAudioMonitorStatus,
    setAudioOutputDevices,
    setVideoRecordingStatus,
    setExternalVideoIoPlans,
    setExternalVideoTransportStatus,
    setExternalVideoTransportReport,
    setExternalVideoTransportEvents,
    setVideoOutputPreviewUrl,
    setVideoOutputPreviewInfo,
    setVideoOutputPreviewId,
    setVideoOutputPreviewMode,
  });
  const videoThumbnailSourceSignature = createMemo(() => JSON.stringify(
    snapshot().video.layers.map((layer) => ({
      id: layer.id,
      kind: layer.source.kind,
      path: layer.source.path ?? null,
      name: layer.source.name ?? null,
    })),
  ));
  let videoThumbnailGeneration = 0;
  let videoThumbnailUrlCache: Record<number, string> = {};
  const videoThumbnailSignatures = new Map<number, string>();
  createEffect(() => {
    const sources = JSON.parse(videoThumbnailSourceSignature()) as Array<{
      id: number;
      kind: VideoSourceKind;
      path: string | null;
      name: string | null;
    }>;
    const generation = ++videoThumbnailGeneration;
    const activeIds = new Set(sources.map((source) => source.id));
    for (const layerId of videoThumbnailSignatures.keys()) {
      if (!activeIds.has(layerId)) videoThumbnailSignatures.delete(layerId);
    }
    videoThumbnailUrlCache = Object.fromEntries(
      Object.entries(videoThumbnailUrlCache).filter(([layerId]) => activeIds.has(Number(layerId))),
    );
    if (!isTauriRuntime()) {
      setVideoClipThumbnails(videoThumbnailUrlCache);
      return;
    }
    void (async () => {
      const nextUrls = { ...videoThumbnailUrlCache };
      const nextSignatures = new Map(videoThumbnailSignatures);
      for (const source of sources) {
        const signature = JSON.stringify(source);
        if (nextSignatures.get(source.id) === signature && nextUrls[source.id]) continue;
        try {
          nextUrls[source.id] = await loadVideoLayerThumbnail(source.id);
          nextSignatures.set(source.id, signature);
        } catch {
          delete nextUrls[source.id];
          nextSignatures.delete(source.id);
        }
        if (generation !== videoThumbnailGeneration) return;
      }
      if (generation !== videoThumbnailGeneration) return;
      videoThumbnailUrlCache = nextUrls;
      videoThumbnailSignatures.clear();
      for (const [layerId, signature] of nextSignatures) {
        videoThumbnailSignatures.set(layerId, signature);
      }
      setVideoClipThumbnails(nextUrls);
    })();
  });
  const videoLayerHasMonitorableAudio = (layerId: number) => {
    const layer = snapshot().video.layers.find((candidate) => candidate.id === layerId);
    return layer?.source.kind === "File" && layer.source.metadata?.has_audio !== false;
  };
  const launchVideoClipFromGrid = async (layerId: number, fadeMs: number) => {
    const launched = await launchVideoClip(layerId, fadeMs);
    if (launched && videoProgramAudioEnabled() && videoLayerHasMonitorableAudio(layerId)) {
      await playVideoLayerAudioMonitor(layerId, videoAudioMonitorVolume(), selectedAudioOutputDevice());
    }
  };
  const takeVideoClipFromGrid = async (layerId: number, fadeMs: number) => {
    const taken = await takeVideoClip(layerId, fadeMs);
    if (taken && videoProgramAudioEnabled()) {
      for (const activeLayerId of videoAudioMonitorStatus().active_layer_ids) {
        if (activeLayerId !== layerId) await stopVideoLayerAudioMonitor(activeLayerId);
      }
      if (videoLayerHasMonitorableAudio(layerId)) {
        await playVideoLayerAudioMonitor(layerId, videoAudioMonitorVolume(), selectedAudioOutputDevice());
      } else if (videoAudioMonitorStatus().active_layer_ids.includes(layerId)) {
        await stopVideoLayerAudioMonitor(layerId);
      }
    }
  };
  const stopVideoClipFromGrid = async (layerId: number, fadeMs: number) => {
    const stopped = await stopVideoClip(layerId, fadeMs);
    if (stopped && videoAudioMonitorStatus().active_layer_ids.includes(layerId)) {
      await stopVideoLayerAudioMonitor(layerId);
    }
  };
  const assignVideoDeck = (deck: "A" | "B", layerId: number) => {
    if (deck === "A") {
      setVideoDeckALayerId(layerId);
      if (videoDeckBLayerId() === layerId) setVideoDeckBLayerId(null);
    } else {
      setVideoDeckBLayerId(layerId);
      if (videoDeckALayerId() === layerId) setVideoDeckALayerId(null);
    }
  };
  const applyVideoAbMix = async (mix: number, refresh = false) => {
    const normalized = Math.max(0, Math.min(1, mix));
    setVideoAbMix(normalized);
    try {
      await invoke("set_video_ab_mix", {
        layerAId: videoDeckALayerId(),
        layerBId: videoDeckBLayerId(),
        mix: normalized,
      });
      const baseGain = videoAudioMonitorVolume();
      const audioA = baseGain * Math.sqrt(1 - normalized);
      const audioB = baseGain * Math.sqrt(normalized);
      if (videoDeckALayerId() !== null && videoAudioMonitorStatus().active_layer_ids.includes(videoDeckALayerId()!)) {
        await setVideoLayerAudioMonitorVolume(videoDeckALayerId()!, audioA);
      }
      if (videoDeckBLayerId() !== null && videoAudioMonitorStatus().active_layer_ids.includes(videoDeckBLayerId()!)) {
        await setVideoLayerAudioMonitorVolume(videoDeckBLayerId()!, audioB);
      }
      if (refresh) await refreshSnapshot();
    } catch (error) { setMessage(String(error)); }
  };
  const launchVideoDeck = async (deck: "A" | "B") => {
    const layerId = deck === "A" ? videoDeckALayerId() : videoDeckBLayerId();
    if (layerId === null) return;
    const launched = await launchVideoClip(layerId, 0);
    if (!launched) return;
    if (videoProgramAudioEnabled() && videoLayerHasMonitorableAudio(layerId)) {
      const deckGain = deck === "A" ? Math.sqrt(1 - videoAbMix()) : Math.sqrt(videoAbMix());
      await playVideoLayerAudioMonitor(
        layerId,
        videoAudioMonitorVolume() * deckGain,
        selectedAudioOutputDevice(),
      );
    }
    await applyVideoAbMix(videoAbMix(), true);
  };
  createEffect(() => {
    const ids = new Set(snapshot().video.layers.map((layer) => layer.id));
    if (videoDeckALayerId() !== null && !ids.has(videoDeckALayerId()!)) setVideoDeckALayerId(null);
    if (videoDeckBLayerId() !== null && !ids.has(videoDeckBLayerId()!)) setVideoDeckBLayerId(null);
  });
  const refreshLiveAudioInputDevices = async () => {
    try {
      const devices = await invoke<string[]>("list_audio_input_devices");
      setLiveAudioInputDevices(devices);
      setMessage(`Found ${devices.length} audio input device(s).`);
    } catch (error) { setMessage(String(error)); }
  };
  const startLiveAudioInput = async () => {
    try {
      setLiveAudioInputStatus(await invoke<LiveAudioInputStatus>("start_live_audio_input", {
        deviceName: selectedLiveAudioInputDevice().trim() || null,
      }));
      setMessage("Live audio FFT input started.");
    } catch (error) { setMessage(String(error)); }
  };
  const stopLiveAudioInput = async () => {
    try {
      setLiveAudioInputStatus(await invoke<LiveAudioInputStatus>("stop_live_audio_input"));
      setMessage("Live audio FFT input stopped.");
    } catch (error) { setMessage(String(error)); }
  };
  const refreshLiveAudioInputStatus = async () => {
    try {
      setLiveAudioInputStatus(await invoke<LiveAudioInputStatus>("live_audio_input_status"));
    } catch { /* Background meter polling is best-effort. */ }
  };
  const videoOutputMetricsTimer = isTauriRuntime()
    ? window.setInterval(() => {
        if (videoOutputWindowStatuses()?.some((status) => status.live_open)) {
          void refreshVideoOutputWindowStatuses(true);
        }
        if (videoAudioMonitorStatus().active_layer_ids.length > 0) {
          void refreshVideoAudioMonitorStatus(true);
        }
        if (videoRecordingStatus().active) {
          void refreshVideoRecordingStatus();
        }
        if (liveAudioInputStatus().running) {
          void refreshLiveAudioInputStatus();
        }
      }, 1000)
    : null;
  onCleanup(() => {
    if (videoOutputMetricsTimer !== null) {
      window.clearInterval(videoOutputMetricsTimer);
    }
  });

  const toggleVideoCompositionLayer = (layerId: number, checked: boolean) => {
    setVideoCompositionLayerIds((current) => {
      if (checked) {
        return current.includes(layerId) ? current : [...current, layerId];
      }
      return current.filter((candidate) => candidate !== layerId);
    });
  };

  const addVideoComposition = async () => {
    try {
      const compositionId = await invoke<number>("add_video_composition", {
        label: videoCompositionLabel(),
        layerIds: videoCompositionLayerIds(),
      });
      setVideoCompositionLabel(`Composition ${snapshot().video.compositions.length + 1}`);
      setVideoCompositionLayerIds([]);
      setMessage(`Added video composition ${compositionId}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const removeVideoComposition = async (compositionId: number) => {
    try {
      await invoke("remove_video_composition", { compositionId });
      setMessage(`Removed video composition ${compositionId}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setVideoCompositionLayers = async (compositionId: number, layerIds: number[]) => {
    try {
      await invoke("set_video_composition_layers", { compositionId, layerIds });
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const moveVideoCompositionLayer = (
    compositionId: number,
    layerIds: number[],
    layerId: number,
    delta: -1 | 1,
  ) => {
    const index = layerIds.indexOf(layerId);
    const nextIndex = index + delta;
    if (index < 0 || nextIndex < 0 || nextIndex >= layerIds.length) {
      return;
    }
    const nextLayerIds = [...layerIds];
    [nextLayerIds[index], nextLayerIds[nextIndex]] = [nextLayerIds[nextIndex], nextLayerIds[index]];
    void setVideoCompositionLayers(compositionId, nextLayerIds);
  };

  const addVideoOutput = async () => {
    try {
      const outputId = await invoke<number>("add_video_output", {
        label: videoOutputLabel(),
        kind: videoOutputKind(),
        width: videoOutputWidth(),
        height: videoOutputHeight(),
        fullscreen: videoOutputFullscreen(),
        monitorId: videoOutputKind() === "Display" ? videoOutputMonitorId() : null,
        endpointName: videoOutputKind() === "Display" ? null : videoOutputEndpoint(),
      });
      setVideoOutputLabel(`Video Output ${snapshot().video.outputs.length + 2}`);
      setMessage(`Added video output ${outputId}`);
      await refreshSnapshotAndVideoOutputRenderPlans();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const removeVideoOutput = async (outputId: number) => {
    const output = snapshot().video.outputs.find((candidate) => candidate.id === outputId);
    if (output && !confirmDestructiveAction("video output", output.label)) {
      return;
    }
    try {
      await invoke("remove_video_output", { outputId });
      setMessage(`Removed video output ${outputId}`);
      await refreshSnapshotAndVideoOutputRenderPlans();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setVideoOutputConfig = async (output: VideoOutputSummary) => {
    const draft = videoOutputConfigDraft(output);
    const width = Math.max(1, Math.round(Number.isFinite(draft.width) ? draft.width : output.width));
    const height = Math.max(1, Math.round(Number.isFinite(draft.height) ? draft.height : output.height));
    const monitorId = Math.max(0, Math.round(Number.isFinite(draft.monitor_id) ? draft.monitor_id : 0));
    try {
      await invoke("set_video_output_config", {
        outputId: output.id,
        label: draft.label,
        kind: draft.kind,
        width,
        height,
        fullscreen: draft.kind === "Display" ? draft.fullscreen : false,
        monitorId: draft.kind === "Display" ? monitorId : null,
        endpointName: draft.kind === "Display" ? null : draft.endpoint_name,
      });
      setVideoOutputConfigDrafts((current) => ({
        ...current,
        [output.id]: {
          ...draft,
          width,
          height,
          fullscreen: draft.kind === "Display" ? draft.fullscreen : false,
          monitor_id: draft.kind === "Display" ? monitorId : 0,
          endpoint_name: draft.kind === "Display" ? "" : draft.endpoint_name,
        },
      }));
      setMessage(`Updated video output ${output.id}`);
      await refreshSnapshotAndVideoOutputRenderPlans();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setVideoOutputEnabled = async (outputId: number, enabled: boolean) => {
    try {
      await invoke("set_video_output_enabled", { outputId, enabled });
      await refreshSnapshotAndVideoOutputRenderPlans();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setVideoOutputRouting = async (outputId: number, compositionId: number) => {
    try {
      await invoke("set_video_output_routing", { outputId, compositionId });
      await refreshSnapshotAndVideoOutputRenderPlans();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setVideoOutputOpacity = async (outputId: number, opacity: number) => {
    try {
      await invoke("set_video_output_opacity", { outputId, opacity });
      await refreshSnapshotAndVideoOutputRenderPlans();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const fadeVideoOutputOpacity = async (outputId: number, opacity: number) => {
    try {
      await invoke("fade_video_output_opacity", {
        outputId,
        opacity,
        durationMs: Math.max(0, Math.round(videoOutputFadeMs())),
      });
      await refreshSnapshotAndVideoOutputRenderPlans();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setVideoOutputBlackout = async (outputId: number, blackout: boolean) => {
    try {
      await invoke("set_video_output_blackout", { outputId, blackout });
      await refreshSnapshotAndVideoOutputRenderPlans();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setVideoOutputMapping = async (outputId: number, mapping: VideoOutputMapping) => {
    try {
      await invoke("set_video_output_mapping", { outputId, mapping });
      await refreshSnapshot();
      return true;
    } catch (error) {
      setMessage(String(error));
      return false;
    }
  };

  const importVideoOutputBitmapMask = async (output: VideoOutputSummary) => {
    try {
      const imported = await invoke<VideoBitmapMaskImportResult | null>("import_video_output_bitmap_mask");
      if (!imported) {
        setMessage("Luma mask import canceled.");
        return false;
      }
      const words = Array.from({ length: 32 }, (_, index) => imported.luma_words[index] ?? 0);
      const updated = await setVideoOutputMapping(output.id, {
        ...output.mapping,
        bitmap_mask_width: imported.width,
        bitmap_mask_height: imported.height,
        bitmap_mask_luma_words: words,
      });
      if (updated) {
        setMessage(
          `Imported ${imported.source_name} as an embedded ${imported.width}x${imported.height} luma mask.`,
        );
      }
      return updated;
    } catch (error) {
      setMessage(String(error));
      return false;
    }
  };

  const clearVideoOutputBitmapMask = async (output: VideoOutputSummary) => {
    const updated = await setVideoOutputMapping(output.id, {
      ...output.mapping,
      bitmap_mask_width: 0,
      bitmap_mask_height: 0,
      bitmap_mask_luma_words: Array(32).fill(0),
    });
    if (updated) {
      setMessage(`Cleared the embedded luma mask from ${output.label}.`);
    }
    return updated;
  };

  const {
    handleMappingStageWheel,
    beginMappingFixtureDrag,
    beginMappingFixtureYawDrag,
    beginMappingVideoOutputDrag,
    beginMappingVideoOutputRotate,
    beginMappingVideoOutputScale,
    beginMappingVideoOutputCornerDrag,
    beginMappingStageObjectDrag,
    beginMappingStageObjectRotate,
    beginMappingStageObjectResize,
    handleMappingStagePointerDown,
    handleMappingStagePointerMove,
    finishMappingStageDrag,
  } = createMappingInteractionController({
    snapshot,
    mappingViewportBox,
    stageWorldBounds,
    zoomMappingViewportAtPoint,
    selectedFixture,
    snapStagePoint,
    setFixtureTransform,
    mappingStageTool,
    isAdditiveMappingSelectionEvent,
    selectMappingFixture,
    selectedMappingFixtureIdSet,
    selectedMappingFixtureIds,
    setSelectedMappingFixtureIds,
    selectFixture,
    activateFixture,
    setSelectedFixtureId,
    setSelectedStageObjectId,
    setSelectedVideoOutputId,
    clearMappingFixtureSelection,
    mappingDrag,
    setMappingDrag,
    mappingMarquee,
    setMappingMarquee,
    mappingViewportPanDrag,
    setMappingViewportPanDrag,
    normalizedMappingViewportZoom,
    setMappingViewport,
    setMappingStageCursorWorld,
    mappingOutputHandleAngleDeg,
    mappingOutputHandleDistance,
    mappingFixtureYawFromPoint,
    dragWorldDelta,
    snapStagePosition,
    refreshSnapshot,
    setMessage,
    isMappingStageObjectDrag,
    mappingStageObjectPreview,
    setStageObject,
    mappingVideoOutputPreviewMapping,
    setVideoOutputMapping,
    visualizerFixtures,
  });

  const fitVideoOutputToStageObject = async (output: VideoOutputSummary, object: StageObjectSummary) => {
    const outputAspect = outputAspectRatio(output.width, output.height);
    const objectAspect =
      object.depth > 0.5 ? clampRange(object.width / object.depth, 0.25, 4) : outputAspect;
    const targetAspect = Number(objectAspect.toFixed(4));
    const targetDepth =
      object.kind === "Screen" && object.depth <= 0.5
        ? object.width / Math.max(0.25, targetAspect)
        : object.depth;
    const scaleY = clampRange(targetDepth / 4.5, 0.25, 3);
    const scaleX = clampRange(object.width / Math.max(0.001, 4.5 * scaleY * targetAspect), 0.25, 3);
    const updated = await setVideoOutputMapping(output.id, {
      ...output.mapping,
      stage_x: Number(object.x.toFixed(2)),
      stage_z: Number(object.z.toFixed(2)),
      rotation_deg: Number(object.rotation_deg.toFixed(1)),
      offset_x: 0,
      offset_y: 0,
      scale_x: Number(scaleX.toFixed(3)),
      scale_y: Number(scaleY.toFixed(3)),
      aspect_ratio: targetAspect,
      aspect_mode: "Fit",
      lens_distortion: 0,
      keystone_x: 0,
      keystone_y: 0,
      corner_top_left_x: 0,
      corner_top_left_y: 0,
      corner_top_right_x: 0,
      corner_top_right_y: 0,
      corner_bottom_right_x: 0,
      corner_bottom_right_y: 0,
      corner_bottom_left_x: 0,
      corner_bottom_left_y: 0,
    });
    if (!updated) {
      return;
    }
    setSelectedVideoOutputId(output.id);
    setSelectedStageObjectId(object.id);
    setMappingShowProjectors(true);
    setMappingShowStageObjects(true);
    setMessage(`Fit ${output.label} to ${object.label}.`);
  };

  const saveVideoOutputMappingPreset = async (mapping: VideoOutputMapping) => {
    try {
      const label = await invoke<string>("save_video_output_mapping_preset", {
        label: videoOutputMappingPresetLabel(),
        mapping,
      });
      setVideoOutputMappingPresetLabel(label);
      setSelectedVideoOutputMappingPresetLabel(label);
      await refreshSnapshot();
      setMessage(`Saved video output mapping preset ${label}.`);
    } catch (error) {
      setMessage(String(error));
    }
  };

  const applyVideoOutputMappingPreset = async (outputId: number, label: string) => {
    const preset = snapshot().video.mapping_presets.find((candidate) => candidate.label === label);
    if (!preset) {
      return;
    }
    await setVideoOutputMapping(outputId, preset.mapping);
    setMessage(`Applied video output mapping preset ${preset.label}.`);
  };

  const removeVideoOutputMappingPreset = async (label: string) => {
    try {
      await invoke("remove_video_output_mapping_preset", { label });
      if (selectedVideoOutputMappingPresetLabel() === label) {
        setSelectedVideoOutputMappingPresetLabel("");
      }
      await refreshSnapshot();
      setMessage(`Removed video output mapping preset ${label}.`);
    } catch (error) {
      setMessage(String(error));
    }
  };

  const exportVideoOutputMappingPreset = async (mapping: VideoOutputMapping) => {
    try {
      const path = await invoke<string | null>("save_video_output_mapping_preset_file", {
        label: videoOutputMappingPresetLabel(),
        mapping,
      });
      setMessage(path ? `Exported video output mapping preset ${path}` : "Video output mapping preset export canceled.");
    } catch (error) {
      setMessage(String(error));
    }
  };

  const importVideoOutputMappingPreset = async () => {
    try {
      const label = await invoke<string | null>("load_video_output_mapping_preset_file");
      if (!label) {
        setMessage("Video output mapping preset import canceled.");
        return;
      }
      setVideoOutputMappingPresetLabel(label);
      setSelectedVideoOutputMappingPresetLabel(label);
      await refreshSnapshot();
      setMessage(`Imported video output mapping preset ${label}.`);
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setWaveOriginFromStageCenter = () => {
    const bounds = stageWorldBounds();
    setWaveOriginX(Number(((bounds.minX + bounds.maxX) / 2).toFixed(2)));
    setWaveOriginY(0);
    setWaveOriginZ(Number(((bounds.minZ + bounds.maxZ) / 2).toFixed(2)));
  };

  const setWaveOriginFromSelectedFixture = () => {
    const fixture = selectedFixture();
    if (!fixture) {
      setMessage("Select a fixture before using it as the wave origin.");
      return;
    }
    setWaveOriginX(Number(fixture.position.x.toFixed(2)));
    setWaveOriginY(Number(fixture.position.y.toFixed(2)));
    setWaveOriginZ(Number(fixture.position.z.toFixed(2)));
  };

  const setWaveOriginFromSelectedStageObject = () => {
    const object = selectedStageObject();
    if (!object) {
      setMessage("Select a stage object before using it as the wave origin.");
      return;
    }
    setWaveOriginX(Number(object.x.toFixed(2)));
    setWaveOriginY(0);
    setWaveOriginZ(Number(object.z.toFixed(2)));
    setMessage(`Wave origin uses ${object.label}.`);
  };

  const setWaveDirectionPreset = (x: number, y: number, z: number) => {
    setWaveDirectionX(x);
    setWaveDirectionY(y);
    setWaveDirectionZ(z);
  };

  const setEffectClockSyncPreset = (beats: number | null) => {
    setEffectClockSyncBeats(beats);
    if (beats === null) {
      return;
    }
    const bpm = Number.isFinite(snapshot().clock.bpm) && snapshot().clock.bpm > 0 ? snapshot().clock.bpm : 120;
    setEffectPeriod(Math.max(10, Math.round((60_000 / bpm) * beats)));
  };

  const waveStageWorldFromPointer = (event: PointerEvent & { currentTarget: SVGElement }) => {
    const svg = event.currentTarget instanceof SVGSVGElement ? event.currentTarget : event.currentTarget.ownerSVGElement;
    if (!svg) {
      return null;
    }
    const rect = svg.getBoundingClientRect();
    const svgX = clampRange(((event.clientX - rect.left) / rect.width) * stageViewBoxSize, 0, stageViewBoxSize);
    const svgZ = clampRange(((event.clientY - rect.top) / rect.height) * stageViewBoxSize, 0, stageViewBoxSize);
    return svgPointToStageWorld(svgX, svgZ, stageWorldBounds());
  };

  const setWaveOriginFromStagePoint = (point: { x: number; z: number }) => {
    setWaveOriginX(Number(point.x.toFixed(2)));
    setWaveOriginY(0);
    setWaveOriginZ(Number(point.z.toFixed(2)));
  };

  const setWaveDirectionFromStagePoint = (point: { x: number; z: number }) => {
    const dx = point.x - waveOriginX();
    const dz = point.z - waveOriginZ();
    const length = Math.sqrt(dx * dx + dz * dz);
    if (length <= 0.001) {
      setWaveDirectionPreset(0, 0, 0);
      return;
    }
    setWaveDirectionPreset(Number((dx / length).toFixed(3)), 0, Number((dz / length).toFixed(3)));
  };

  const updateWaveStageFromPointer = (event: PointerEvent & { currentTarget: SVGElement }, mode: WaveStageDragMode) => {
    const point = waveStageWorldFromPointer(event);
    if (!point) {
      return;
    }
    if (mode === "origin") {
      setWaveOriginFromStagePoint(point);
    } else if (mode === "direction") {
      setWaveDirectionFromStagePoint(point);
    } else {
      setEffectVideoPositionFromStagePoint(point);
    }
  };

  const startWaveStageDrag = (event: PointerEvent & { currentTarget: SVGElement }, mode: WaveStageDragMode) => {
    event.preventDefault();
    event.stopPropagation();
    event.currentTarget.setPointerCapture(event.pointerId);
    setWaveStageDrag(mode);
    updateWaveStageFromPointer(event, mode);
  };

  const moveWaveStageDrag = (event: PointerEvent & { currentTarget: SVGElement }) => {
    const mode = waveStageDrag();
    if (!mode) {
      return;
    }
    event.preventDefault();
    updateWaveStageFromPointer(event, mode);
  };

  const endWaveStageDrag = (event: PointerEvent & { currentTarget: SVGElement }) => {
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    setWaveStageDrag(null);
  };

  const openVideoOutputWindow = async (outputId: number, testPattern = false) => {
    try {
      await invoke("open_video_output_window", { outputId, testPattern });
      await Promise.all([refreshVideoOutputRenderPlans(true), refreshVideoOutputWindowStatuses(true)]);
      setMessage(`Opened video output ${outputId} ${testPattern ? "test pattern" : "window"}.`);
    } catch (error) {
      setMessage(String(error));
    }
  };

  const syncVideoOutputWindow = async (outputId: number, testPattern = false) => {
    try {
      await invoke("sync_video_output_window", { outputId, testPattern });
      await Promise.all([refreshVideoOutputRenderPlans(true), refreshVideoOutputWindowStatuses(true)]);
      setMessage(`Synced video output ${outputId} ${testPattern ? "test pattern" : "window"}.`);
    } catch (error) {
      setMessage(String(error));
    }
  };

  const closeVideoOutputWindow = async (outputId: number, testPattern = false) => {
    try {
      await invoke("close_video_output_window", { outputId, testPattern });
      await refreshVideoOutputWindowStatuses(true);
      setMessage(`Closed video output ${outputId} ${testPattern ? "test pattern" : "window"}.`);
    } catch (error) {
      setMessage(String(error));
    }
  };

  const buildEffectVideoTargets = (includePosition: boolean, forceVideoTarget = false): VideoEffectTarget[] => {
    const videoLayerId = selectedEffectVideoLayerId();
    const shouldIncludeVideoTarget = forceVideoTarget || effectTargetMode() === "video" || effectVideoTargetLinked();
    if (!shouldIncludeVideoTarget || videoLayerId === null) {
      return [];
    }
    return [
      {
        layer_ids: [videoLayerId],
        param: effectVideoParam(),
        low: effectVideoLow(),
        high: effectVideoHigh(),
        position: includePosition ? effectVideoTargetPosition() : null,
      },
    ];
  };
  const effectSubmitDisabled = createMemo(() => {
    const linkedVideoMissing = effectVideoTargetLinked() && selectedEffectVideoLayerId() === null;
    switch (effectTargetMode()) {
      case "video":
        return snapshot().video.layers.length === 0;
      case "selection":
        return selectedMappingFixtures().length === 0 || !selectedEffectAttribute() || linkedVideoMissing;
      case "group":
        return parseGroupIds(effectTargetGroups()).length === 0 || !selectedEffectAttribute() || linkedVideoMissing;
      case "fixture":
      default:
        return !selectedFixture() || !selectedEffectAttribute() || linkedVideoMissing;
    }
  });

  type EffectRequestDraft =
    | { effectType: "Lfo"; request: LfoEffectRequest }
    | { effectType: "PositionWave"; request: PositionWaveEffectRequest };

  const buildEffectRequestFromForm = (): EffectRequestDraft | null => {
    const fixture = selectedFixture();
    const attribute = selectedEffectAttribute();
    const targetMode = effectTargetMode();
    const isVideoTarget = targetMode === "video";
    const includesVideoTarget = isVideoTarget || effectVideoTargetLinked();
    if (targetMode === "fixture" && (!fixture || !attribute)) {
      setMessage("Select a fixture and attribute first.");
      return null;
    }
    const selectedMapFixtureIds = selectedMappingFixtures().map((candidate) => candidate.id);
    if (targetMode === "selection" && selectedMapFixtureIds.length === 0) {
      setMessage("Select one or more fixtures on the 2D mapping stage first.");
      return null;
    }
    if (targetMode === "selection" && !attribute) {
      setMessage("Select a fixture profile attribute before targeting a map selection.");
      return null;
    }
    const targetGroupIds = parseGroupIds(effectTargetGroups());
    if (targetMode === "group" && targetGroupIds.length === 0) {
      setMessage("Enter at least one target group.");
      return null;
    }
    if (targetMode === "group" && !attribute) {
      setMessage("Select a fixture profile attribute before targeting a group.");
      return null;
    }
    const videoLayerId = selectedEffectVideoLayerId();
    if (includesVideoTarget && videoLayerId === null) {
      setMessage(isVideoTarget ? "Add a video layer before adding a video effect." : "Add a video layer before linking video to this effect.");
      return null;
    }
    const lightAttribute = attribute ?? "";
    const videoTargets = buildEffectVideoTargets(effectType() === "PositionWave");
    const clockSyncBeats = effectClockSyncBeats();
    const labelTarget = !isVideoTarget && videoTargets.length > 0 ? `${lightAttribute} + ${effectVideoParam()}` : isVideoTarget ? effectVideoParam() : lightAttribute;
    const requestBase = {
      label: `${labelTarget} ${effectType()}`,
      fixture_ids:
        targetMode === "selection"
          ? selectedMapFixtureIds
          : targetMode === "fixture" && fixture
            ? [fixture.id]
            : [],
      target_group_ids: targetMode === "group" ? targetGroupIds : [],
      attribute: isVideoTarget ? "" : lightAttribute,
      video_targets: videoTargets,
      shape: effectShape(),
      clock_sync: clockSyncBeats === null ? null : { beats: clockSyncBeats },
      low: effectLow(),
      high: effectHigh(),
      phase: effectPhase(),
      blend_mode: effectBlendMode(),
    };
    if (effectType() === "Lfo") {
      return {
        effectType: "Lfo",
        request: {
          ...requestBase,
          period_ms: effectPeriod(),
        },
      };
    }
    return {
      effectType: "PositionWave",
      request: {
        ...requestBase,
        origin: { x: waveOriginX(), y: waveOriginY(), z: waveOriginZ() },
        direction: { x: waveDirectionX(), y: waveDirectionY(), z: waveDirectionZ() },
        speed: waveSpeed(),
        wavelength: waveWavelength(),
      },
    };
  };

  const addEffect = async () => {
    const draft = buildEffectRequestFromForm();
    if (!draft) {
      return;
    }
    try {
      const effectId =
        draft.effectType === "Lfo"
          ? await invoke<number>("add_lfo_effect", {
              request: draft.request,
            })
          : await invoke<number>("add_position_wave_effect", {
              request: draft.request,
            });
      setEditingEffectId(null);
      setMessage(`Added ${draft.effectType === "PositionWave" ? "position wave" : "LFO"} effect ${effectId}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const updateEditingEffect = async () => {
    const effectId = editingEffectId();
    if (effectId === null) {
      await addEffect();
      return;
    }
    const draft = buildEffectRequestFromForm();
    if (!draft) {
      return;
    }
    try {
      if (draft.effectType === "Lfo") {
        await invoke("update_lfo_effect", { effectId, request: draft.request });
      } else {
        await invoke("update_position_wave_effect", { effectId, request: draft.request });
      }
      setMessage(`Updated effect ${effectId}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const cancelEffectEdit = () => {
    const effectId = editingEffectId();
    setEditingEffectId(null);
    setMessage(effectId === null ? "No effect draft edit is active." : `Canceled edit for effect ${effectId}.`);
  };

  const removeEffect = async (effectId: number) => {
    const effect = snapshot().effects.find((candidate) => candidate.id === effectId);
    if (effect && !confirmDestructiveAction("effect", effect.label)) {
      return;
    }
    try {
      await invoke("remove_effect", { effectId });
      if (editingEffectId() === effectId) {
        setEditingEffectId(null);
      }
      setMessage(`Removed effect ${effectId}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const duplicateEffect = async (effectId: number) => {
    try {
      const duplicateId = await invoke<number>("duplicate_effect", { effectId });
      setMessage(`Duplicated effect ${effectId} as ${duplicateId}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const useEffectAsDraft = (effect: EffectSummary) => {
    const targetPlan = effectDraftTargetPlan(effect, snapshot().fixtures);
    setEditingEffectId(effect.id);
    setEffectType(effect.effect_type);
    setEffectShape(effect.shape);
    setEffectClockSyncBeats(effect.clock_sync?.beats ?? null);
    if (effect.period_ms) {
      setEffectPeriod(effect.period_ms);
    }
    setEffectLow(effect.low);
    setEffectHigh(effect.high);
    setEffectPhase(effect.phase);
    setEffectBlendMode(effect.blend_mode);
    if (effect.origin) {
      setWaveOriginX(effect.origin.x);
      setWaveOriginY(effect.origin.y);
      setWaveOriginZ(effect.origin.z);
    }
    if (effect.direction) {
      setWaveDirectionX(effect.direction.x);
      setWaveDirectionY(effect.direction.y);
      setWaveDirectionZ(effect.direction.z);
    }
    if (effect.speed !== null && effect.speed !== undefined) {
      setWaveSpeed(effect.speed);
    }
    if (effect.wavelength !== null && effect.wavelength !== undefined) {
      setWaveWavelength(effect.wavelength);
    }

    const firstVideoTarget = targetPlan.videoTarget;
    if (firstVideoTarget) {
      setEffectVideoLayerId(firstVideoTarget.layer_ids[0] ?? null);
      setEffectVideoParam(firstVideoTarget.param);
      setEffectVideoLow(firstVideoTarget.low);
      setEffectVideoHigh(firstVideoTarget.high);
      if (firstVideoTarget.position) {
        setEffectVideoPositionX(firstVideoTarget.position.x);
        setEffectVideoPositionY(firstVideoTarget.position.y);
        setEffectVideoPositionZ(firstVideoTarget.position.z);
      }
    }
    setEffectVideoTargetLinked(targetPlan.videoTargetLinked);
    setEffectTargetMode(targetPlan.mode === "source" ? effectTargetMode() : targetPlan.mode);
    setEffectTargetGroups(targetPlan.targetGroups);
    setEffectAttribute(targetPlan.attribute);
    if (targetPlan.mode === "selection") {
      setSelectedMappingFixtureIds(targetPlan.fixtureIds);
    }
    if (targetPlan.activeFixtureId !== null) {
      const fixture = snapshot().fixtures.find((candidate) => candidate.id === targetPlan.activeFixtureId);
      if (fixture) {
        activateFixture(fixture);
      }
    }
    setMessage(targetPlan.message);
  };

  const saveEffectPreset = async (effectId: number) => {
    try {
      const path = await invoke<string | null>("save_effect_preset", { effectId });
      setMessage(path ? `Saved effect preset ${path}` : "Effect preset save canceled.");
    } catch (error) {
      setMessage(String(error));
    }
  };

  type EffectTargetOverrideOptions = {
    forceVideoTarget?: boolean;
    requireLightTarget?: boolean;
  };

  const sampleEffectTargetOverrideOptions = (preset: SampleEffectPreset): EffectTargetOverrideOptions =>
    preset === "shared" ? { forceVideoTarget: true, requireLightTarget: true } : {};

  const effectTargetOverrideError = (options: EffectTargetOverrideOptions = {}) => {
    const targetMode = effectTargetMode();
    const fixture = selectedFixture();
    const attribute = selectedEffectAttribute();
    if (options.requireLightTarget && targetMode === "video") {
      return "Select a fixture or group target for this shared lighting + video preset.";
    }
    if (targetMode === "fixture" && (!fixture || !attribute)) {
      return "Select a fixture and attribute first.";
    }
    if (targetMode === "selection") {
      if (selectedMappingFixtures().length === 0) {
        return "Select one or more fixtures on the 2D mapping stage first.";
      }
      if (!attribute) {
        return "Select a fixture profile attribute before targeting a map selection.";
      }
    }
    if (targetMode === "group") {
      if (parseGroupIds(effectTargetGroups()).length === 0) {
        return "Enter at least one target group.";
      }
      if (!attribute) {
        return "Select a fixture profile attribute before targeting a group.";
      }
    }
    if ((targetMode === "video" || effectVideoTargetLinked() || options.forceVideoTarget) && selectedEffectVideoLayerId() === null) {
      return targetMode === "video"
        ? "Add a video layer before loading a video effect preset."
        : "Add a video layer before loading this lighting + video preset.";
    }
    return "";
  };

  const sampleEffectTargetOverrideError = (preset: SampleEffectPreset) =>
    effectTargetOverrideError(sampleEffectTargetOverrideOptions(preset));

  const effectTargetOverrideFromForm = (options: EffectTargetOverrideOptions = {}): EffectTargetOverride | null => {
    if (effectTargetOverrideError(options)) {
      return null;
    }
    const targetMode = effectTargetMode();
    const fixture = selectedFixture();
    const attribute = selectedEffectAttribute();
    const videoTargets = buildEffectVideoTargets(true, Boolean(options.forceVideoTarget));
    return {
      fixture_ids:
        targetMode === "selection"
          ? selectedMappingFixtures().map((candidate) => candidate.id)
          : targetMode === "fixture" && fixture
            ? [fixture.id]
            : [],
      target_group_ids: targetMode === "group" ? parseGroupIds(effectTargetGroups()) : [],
      attribute: targetMode === "video" ? "" : attribute,
      video_targets: videoTargets,
    };
  };

  const nodeGraphTargetLabel = (graph: NodeGraphSummary) => {
    const outputs = graph.nodes
      .filter((node) => node.kind === "Output" && node.output)
      .map((node) => node.output!)
      .flatMap((output) => [
        output.fixture_ids.length > 0 ? `${output.fixture_ids.length} fixture(s)` : "",
        output.target_group_ids.length > 0 ? `groups ${output.target_group_ids.join(",")}` : "",
        output.video_targets.length > 0
          ? `video ${output.video_targets.flatMap((target) => target.layer_ids).length} layer target(s)`
          : "",
        output.attribute ? output.attribute : "",
      ])
      .filter(Boolean);
    return outputs.length > 0 ? outputs.join(" / ") : "no target";
  };

  const nodeGraphSourceLabel = () =>
    nodeGraphSourceMode() === "Audio"
      ? `${nodeGraphAudioSource() === "Live" ? "Live" : "Timeline"} Audio ${nodeGraphAudioBand()}`
      : effectType() === "PositionWave"
        ? "Position Wave"
        : "LFO";

  const nodeGraphTransformLabel = () => {
    const op = nodeGraphTransformOp();
    if (op === "Invert" || op === "Abs") {
      return op;
    }
    if (op === "Clamp") {
      return `${op} ${nodeGraphTransformMin()}-${nodeGraphTransformMax()}`;
    }
    return `${op} ${nodeGraphTransformAmount()}`;
  };

  const nodeGraphClockSync = () => {
    const beats = effectClockSyncBeats();
    return beats === null ? null : { beats };
  };

  const nodeGraphSourceDetail = () => {
    if (nodeGraphSourceMode() === "Audio") {
      const availability = nodeGraphAudioSource() === "Live"
        ? liveAudioInputStatus().running ? "Live input" : "Live stopped"
        : audioAnalysis()?.spectrum.length ? "FFT ready" : "No FFT";
      return `${nodeGraphAudioGain().toFixed(1)}x ${nodeGraphAudioBias() >= 0 ? "+" : ""}${nodeGraphAudioBias().toFixed(2)} / ${availability}`;
    }
    const sync = nodeGraphClockSync();
    return sync ? `${effectShape()} / ${sync.beats}b` : effectShape();
  };

  const buildNodeGraphFromForm = (): NodeGraphSummary | null => {
    const target = effectTargetOverrideFromForm();
    if (!target) {
      return null;
    }
    const lightLow = Math.max(0, Math.min(65_535, Math.round(effectLow())));
    const lightHigh = Math.max(0, Math.min(65_535, Math.round(effectHigh())));
    const label =
      nodeGraphLabel().trim() ||
      `${effectTargetMode() === "video" ? effectVideoParam() : selectedEffectAttribute()} ${nodeGraphSourceLabel()}`;
    const clockSync = nodeGraphClockSync();
    const sourceNode =
      nodeGraphSourceMode() === "Audio"
        ? {
            id: 1,
            label: `Audio ${nodeGraphAudioBand()}`,
            kind: "Audio" as const,
            x: 18,
            y: 26,
            lfo: null,
            position_wave: null,
            audio: {
              source: nodeGraphAudioSource(),
              band: nodeGraphAudioBand(),
              gain: nodeGraphAudioGain(),
              bias: nodeGraphAudioBias(),
            },
            transform: null,
            output: null,
          }
        : effectType() === "PositionWave"
        ? {
            id: 1,
            label: "Wave",
            kind: "PositionWave" as const,
            x: 18,
            y: 26,
            lfo: null,
            position_wave: {
              shape: effectShape(),
              origin: { x: waveOriginX(), y: waveOriginY(), z: waveOriginZ() },
              direction: { x: waveDirectionX(), y: waveDirectionY(), z: waveDirectionZ() },
              speed: waveSpeed(),
              wavelength: waveWavelength(),
              clock_sync: clockSync,
              phase: effectPhase(),
            },
            audio: null,
            transform: null,
            output: null,
          }
        : {
            id: 1,
            label: "LFO",
            kind: "Lfo" as const,
            x: 18,
            y: 26,
            lfo: {
              shape: effectShape(),
              period_ms: Math.max(10, Math.round(effectPeriod())),
              clock_sync: clockSync,
              phase: effectPhase(),
              amplitude: 1,
              bias: 0,
            },
            position_wave: null,
            audio: null,
            transform: null,
            output: null,
          };
    return {
      id: 0,
      label,
      enabled: true,
      nodes: [
        sourceNode,
        {
          id: 2,
          label: nodeGraphTransformOp(),
          kind: "Transform",
          x: 50,
          y: 26,
          lfo: null,
          position_wave: null,
          audio: null,
          transform: {
            op: nodeGraphTransformOp(),
            amount: nodeGraphTransformAmount(),
            min: nodeGraphTransformMin(),
            max: nodeGraphTransformMax(),
          },
          output: null,
        },
        {
          id: 3,
          label: "Output",
          kind: "Output",
          x: 82,
          y: 26,
          lfo: null,
          position_wave: null,
          audio: null,
          transform: null,
          output: {
            fixture_ids: target.fixture_ids,
            target_group_ids: target.target_group_ids,
            attribute: target.attribute,
            video_targets: target.video_targets,
            low: lightLow,
            high: lightHigh,
            blend_mode: effectBlendMode(),
          },
        },
      ],
      edges: [
        { from_node: 1, from_port: "value", to_node: 2, to_port: "input" },
        { from_node: 2, from_port: "value", to_node: 3, to_port: "input" },
      ],
    };
  };

  const saveNodeGraphFromForm = async () => {
    const graph = buildNodeGraphFromForm();
    if (!graph) {
      setMessage(effectTargetOverrideError() || "Select a valid node graph target.");
      return;
    }
    try {
      const graphId = await invoke<number>("save_node_graph", { graph });
      setNodeGraphLabel(`Graph ${snapshot().node_graphs.length + 2}`);
      setMessage(`Saved node graph ${graphId}.`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setNodeGraphEnabled = async (graphId: number, enabled: boolean) => {
    try {
      await invoke("set_node_graph_enabled", { graphId, enabled });
      setMessage(`${enabled ? "Enabled" : "Disabled"} node graph ${graphId}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const removeNodeGraph = async (graphId: number) => {
    try {
      await invoke("remove_node_graph", { graphId });
      setMessage(`Removed node graph ${graphId}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const saveNodeGraphPreset = async (graphId: number) => {
    try {
      const path = await invoke<string | null>("save_node_graph_preset_file", { graphId });
      setMessage(path ? `Saved node graph preset ${path}` : "Node graph preset save canceled.");
    } catch (error) {
      setMessage(String(error));
    }
  };

  const loadNodeGraphPreset = async () => {
    try {
      const graphId = await invoke<number | null>("load_node_graph_preset_file");
      if (graphId === null) {
        setMessage("Node graph preset load canceled.");
        return;
      }
      setMessage(`Loaded node graph preset as graph ${graphId}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const loadEffectPreset = async () => {
    try {
      const effectId = await invoke<number | null>("load_effect_preset");
      if (effectId === null) {
        setMessage("Effect preset load canceled.");
        return;
      }
      setMessage(`Loaded effect preset as effect ${effectId}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const loadSampleEffectPreset = async (preset: SampleEffectPreset, useCurrentTarget = false) => {
    if (useCurrentTarget && !sampleEffectPresetSupportsTarget(preset)) {
      setMessage("Circle sample creates a Pan/Tilt pair and cannot be retargeted to one current attribute.");
      return;
    }
    const targetOverrideOptions = sampleEffectTargetOverrideOptions(preset);
    const targetOverride = useCurrentTarget ? effectTargetOverrideFromForm(targetOverrideOptions) : null;
    if (useCurrentTarget && !targetOverride) {
      setMessage(sampleEffectTargetOverrideError(preset));
      return;
    }
    try {
      if (preset === "circle") {
        const effectIds = await invoke<number[]>("load_sample_effect_bundle", { preset });
        setMessage(`Loaded sample circle bundle as effects ${effectIds.join(", ")}`);
        await refreshSnapshot();
        return;
      }
      const effectId = await invoke<number>("load_sample_effect_preset", {
        preset,
        targetOverride,
      });
      setMessage(
        useCurrentTarget
          ? preset === "shared"
            ? `Loaded sample ${preset} effect as effect ${effectId} for current lighting + video targets`
            : `Loaded sample ${preset} effect as effect ${effectId} for current target`
          : `Loaded sample ${preset} effect as effect ${effectId}`,
      );
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const loadEffectPresetForCurrentTarget = async () => {
    const targetOverride = effectTargetOverrideFromForm();
    if (!targetOverride) {
      setMessage(effectTargetOverrideError());
      return;
    }
    try {
      const effectId = await invoke<number | null>("load_effect_preset_for_target", { targetOverride });
      if (effectId === null) {
        setMessage("Effect preset load canceled.");
        return;
      }
      setMessage(`Loaded effect preset as effect ${effectId} for current target`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setEffectVideoTargetsFromSelectedOutput = async (effectId: number, targets: VideoEffectTarget[]) => {
    const output = selectedMappingVideoOutput();
    if (!output) {
      setMessage("Add or select a video output before updating video target positions.");
      return;
    }
    const layerIds = [...new Set(targets.flatMap((target) => target.layer_ids))];
    if (layerIds.length === 0) {
      setMessage(`Effect ${effectId} has no video target layers.`);
      return;
    }
    const mapping = mappingVideoOutputMapping(output);
    const position = {
      x: mapping.stage_x,
      y: 0,
      z: mapping.stage_z,
    };
    try {
      await Promise.all(
        layerIds.map((layerId) =>
          invoke("set_effect_video_target_position", {
            effectId,
            layerId,
            position,
          }),
        ),
      );
      setEffectVideoPosition(position);
      setSelectedVideoOutputId(output.id);
      setMessage(`Updated effect ${effectId} video target position from ${output.label}.`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setEffectEnabled = async (effectId: number, enabled: boolean) => {
    try {
      await invoke("set_effect_enabled", { effectId, enabled });
      setMessage(`${enabled ? "Enabled" : "Disabled"} effect ${effectId}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const moveEffect = async (effectId: number, delta: -1 | 1) => {
    try {
      await invoke("move_effect", { effectId, delta });
      setMessage(`Moved effect ${effectId}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const toggleMappingLayer = (layer: NonNullable<ReturnType<typeof mappingLayerToggleFromHotkey>>) => {
    switch (layer) {
      case "labels": {
        const next = !mappingShowLabels();
        setMappingShowLabels(next);
        setMessage(`2D mapping labels ${next ? "shown" : "hidden"}.`);
        return;
      }
      case "beams": {
        const next = !mappingShowBeams();
        setMappingShowBeams(next);
        setMessage(`2D mapping beams ${next ? "shown" : "hidden"}.`);
        return;
      }
      case "geometry": {
        const next = !mappingShowGeometry();
        setMappingShowGeometry(next);
        setMessage(`2D mapping geometry ${next ? "shown" : "hidden"}.`);
        return;
      }
      case "projectors": {
        const next = !mappingShowProjectors();
        setMappingShowProjectors(next);
        setMessage(`2D mapping projection surfaces ${next ? "shown" : "hidden"}.`);
        return;
      }
      case "objects": {
        const next = !mappingShowStageObjects();
        setMappingShowStageObjects(next);
        setMessage(`2D mapping stage objects ${next ? "shown" : "hidden"}.`);
        return;
      }
      case "levels": {
        const next = !mappingShowLevels();
        setMappingShowLevels(next);
        setMessage(`2D mapping levels ${next ? "shown" : "hidden"}.`);
        return;
      }
    }
  };

  const applyMappingViewportAction = (action: NonNullable<ReturnType<typeof mappingViewportActionFromHotkey>>) => {
    switch (action) {
      case "fitVisible":
        fitMappingViewportToVisible();
        return;
      case "fitSelection":
        fitMappingViewportToSelection();
        return;
      case "zoomIn":
        zoomMappingViewport(1);
        setMessage(`2D mapping zoom ${mappingViewportZoomLabel()}.`);
        return;
      case "zoomOut":
        zoomMappingViewport(-1);
        setMessage(`2D mapping zoom ${mappingViewportZoomLabel()}.`);
        return;
      case "reset":
        resetMappingViewport();
        return;
    }
  };

  const applyMappingSelectionAction = (action: NonNullable<ReturnType<typeof mappingSelectionActionFromHotkey>>) => {
    switch (action) {
      case "layoutLine":
        void layoutSelectedMappingFixtures("line");
        return;
      case "layoutGrid":
        void layoutSelectedMappingFixtures("grid");
        return;
      case "layoutCircle":
        void layoutSelectedMappingFixtures("circle");
        return;
      case "layoutObjectLine":
        void layoutSelectedFixturesOnStageObject("line");
        return;
      case "layoutObjectGrid":
        void layoutSelectedFixturesOnStageObject("grid");
        return;
      case "alignX":
        void alignSelectedMappingFixtures("x");
        return;
      case "alignZ":
        void alignSelectedMappingFixtures("z");
        return;
      case "distributeX":
        void distributeSelectedMappingFixtures("x");
        return;
      case "distributeZ":
        void distributeSelectedMappingFixtures("z");
        return;
      case "mirrorX":
        void mirrorSelectedMappingFixtures("x");
        return;
      case "mirrorZ":
        void mirrorSelectedMappingFixtures("z");
        return;
      case "rotateLeft":
        void rotateSelectedMappingFixtures(-15);
        return;
      case "rotateRight":
        void rotateSelectedMappingFixtures(15);
        return;
      case "flip180":
        void rotateSelectedMappingFixtures(180);
        return;
    }
  };

  const applyMappingSelectionManagementAction = (
    action: NonNullable<ReturnType<typeof mappingSelectionManagementActionFromHotkey>>,
  ) => {
    switch (action) {
      case "pickVisible":
        pickVisibleMappingFixtures();
        return;
      case "clearPick":
        clearMappingFixtureSelection();
        return;
      case "pickInside":
        pickFixturesInsideSelectedStageObject("replace");
        return;
      case "addInside":
        pickFixturesInsideSelectedStageObject("add");
        return;
    }
  };

  const toggleMappingSelectionFlag = (flag: NonNullable<ReturnType<typeof mappingSelectionFlagFromHotkey>>) => {
    const flagState = selectedMappingFlagState();
    const enabled =
      flag === "highlight"
        ? !flagState.allHighlighted
        : flag === "solo"
          ? !flagState.allSoloed
          : !flagState.allParked;
    void setMappingSelectionFlag(flag, enabled);
  };

  const selectControlMode = (mode: ControlMode) => {
    setControlMode(mode);
    if (mode === "edit") setEditDeskSurface("attributes");
    if (mode === "live") setTimelineDeskSurface("show");
  };

  const { handleControlKeyDown } = createAppKeyboardController({
    workspaceTab,
    setWorkspaceTab,
    setupSubTab,
    selectSetupMode,
    setControlMode: selectControlMode,
    setMessage,
    saveProject,
    saveProjectAs,
    loadProject,
    newProject,
    undoProject,
    redoProject,
    mappingHotkeyHelpOpen,
    setMappingHotkeyHelpOpen,
    applyMappingSelectionManagementAction,
    duplicateSelectedMappingFixtures,
    setMappingStageTool,
    toggleMappingLayer,
    toggleMappingSelectionFlag,
    applyMappingViewportAction,
    applyMappingSelectionAction,
    normalizedMappingSnapSize,
    nudgeSelectedMappingFixtures,
    removeSelectedMappingFixtures,
    setMappingDrag,
    setMappingMarquee,
    setMappingViewportPanDrag,
    snapshot,
    triggerPreviousCue,
    triggerNextCue,
    triggerCue,
    cuePadStartIndex,
    setCueFadePaused,
    playTimeline,
    pauseTimeline,
    setAllBlackout,
    setBlackout,
    setVideoBlackout,
    tapBpm,
  });

  let nativeCloseApproved = false;
  let closeRequestListenerDisposed = false;
  let unlistenCloseRequested: (() => void) | undefined;
  const approveNativeCloseOnce = () => {
    nativeCloseApproved = true;
    window.setTimeout(() => {
      nativeCloseApproved = false;
    }, 1000);
  };

  const handleBeforeUnload = (event: BeforeUnloadEvent) => {
    if (nativeCloseApproved || !projectDirty()) {
      return;
    }
    event.preventDefault();
    event.returnValue = "";
  };

  window.addEventListener("keydown", handleControlKeyDown);
  window.addEventListener("beforeunload", handleBeforeUnload);
  if (isTauriRuntime()) {
    void getCurrentWindow()
      .onCloseRequested((event) => {
        if (!projectDirty()) {
          return;
        }
        if (confirmDiscardProjectChanges("close Syndocal")) {
          approveNativeCloseOnce();
          return;
        }
        event.preventDefault();
        setMessage("Close canceled.");
      })
      .then((unlisten) => {
        if (closeRequestListenerDisposed) {
          unlisten();
          return;
        }
        unlistenCloseRequested = unlisten;
      })
      .catch((error) => setMessage(String(error)));
  }
  onCleanup(() => {
    closeRequestListenerDisposed = true;
    unlistenCloseRequested?.();
    window.removeEventListener("keydown", handleControlKeyDown);
    window.removeEventListener("beforeunload", handleBeforeUnload);
  });

  return (
    <main class="app">
      <WorkspaceChrome
        workspaceTab={workspaceTab()}
        setupSubTab={setupSubTab()}
        controlMode={controlMode()}
        blackout={snapshot().blackout}
        videoBlackout={snapshot().video.blackout}
        bpm={snapshot().clock.bpm}
        tickMs={Math.round(snapshot().telemetry.last_tick_interval_us / 1000)}
        jitterUs={Math.round(snapshot().telemetry.tick_jitter_stddev_us)}
        packetBytes={snapshot().telemetry.last_packet_bytes}
        dmxSuccessCount={snapshot().telemetry.last_dmx_send_success_count}
        dmxOutputCount={snapshot().telemetry.last_dmx_output_count}
        projectLabel={projectFileLabel()}
        projectDirty={projectDirty()}
        currentProjectPath={currentProjectPath()}
        recentProjectPaths={recentProjectPaths()}
        recoveryCheckpoint={projectRecoveryCheckpoint()}
        projectBackups={projectBackups()}
        historyStatus={projectHistoryStatus()}
        applicationUpdateConfiguration={applicationUpdateConfiguration()}
        applicationUpdateCheck={applicationUpdateCheck()}
        applicationUpdateProgress={applicationUpdateProgress()}
        applicationUpdateBusy={applicationUpdateBusy()}
        applicationUpdateError={applicationUpdateError()}
        uiScale={uiScale()}
        uiLocale={uiLocale()}
        canGo={snapshot().cues.length > 0}
        nextCueLabel={nextCue()?.label ?? "No cue"}
        onWorkspaceTab={setWorkspaceTab}
        onSetupSubTab={selectSetupMode}
        onControlMode={selectControlMode}
        onGo={() => void triggerNextCue()}
        onNewProject={newProject}
        onSaveUserTemplate={() => void saveUserTemplate()}
        onLoadUserTemplate={() => void loadUserTemplate()}
        onSaveProject={saveProject}
        onSaveProjectAs={saveProjectAs}
        onLoadProject={loadProject}
        onLoadRecentProject={loadRecentProject}
        onClearRecentProjects={clearRecentProjects}
        onLoadRecovery={loadProjectRecovery}
        onDiscardRecovery={discardProjectRecovery}
        onLoadBackup={(backup) => void loadProjectBackup(backup)}
        onDeleteBackup={(backup) => void deleteProjectBackup(backup)}
        onExportDiagnostics={() => void exportDiagnosticPackage()}
        onCheckForUpdates={() => void checkForApplicationUpdate(false)}
        onInstallUpdate={() => void installAvailableApplicationUpdate()}
        onUiLocale={setUiLocale}
        onUndo={() => void undoProject()}
        onRedo={() => void redoProject()}
        onUiScale={(scale) => setUiScale(scale)}
        onResetWorkspaceLayout={resetWorkspaceLayout}
        onLoadSample={loadPhase1SampleProject}
        onRunSmoke={runPhase1Smoke}
      />
      <Show when={projectDropState()}>
        <div class={`projectDropOverlay ${projectDropState() === "invalid" ? "invalid" : ""}`}>
          <strong>{projectDropState() === "project" ? "Open Syndocal Project" : "Unsupported File"}</strong>
          <span>{projectDropState() === "project" ? "Drop to load the .sdc project." : "Drop a .sdc project file."}</span>
        </div>
      </Show>

      <section class={`layout ${touchLayoutClass()}`}>
        <Show when={workspaceTab() === "control"}>
        <section class="panel liveControlPanel controlPanel">
          <div class="panelHeader">
            <h2>Live Desk</h2>
            <span>{snapshot().blackout || snapshot().video.blackout ? "Guarded" : "Ready"}</span>
          </div>
          <div class="liveStatusGrid">
            <div class="liveStatusItem">
              <span>Active cue</span>
              <strong>{activeCue()?.label ?? "None"}</strong>
            </div>
            <div class="liveStatusItem">
              <span>Next cue</span>
              <strong>{nextCue()?.label ?? "None"}</strong>
            </div>
            <div class="liveStatusItem">
              <span>Fixtures</span>
              <strong>{snapshot().fixtures.length}</strong>
            </div>
            <div class="liveStatusItem">
              <span>Effects</span>
              <strong>{activeEffectCount()} / {snapshot().effects.length}</strong>
            </div>
            <div class="liveStatusItem">
              <span>DMX routes</span>
              <strong>{enabledDmxOutputCount()} / {snapshot().dmx_outputs.length}</strong>
            </div>
            <div class="liveStatusItem">
              <span>Video outs</span>
              <strong>{enabledVideoOutputCount()} / {snapshot().video.outputs.length}</strong>
            </div>
            <div class="liveStatusItem">
              <span>Timeline</span>
              <strong>{snapshot().timeline.playing ? "Playing" : "Stopped"}</strong>
            </div>
            <div class="liveStatusItem">
              <span>BPM</span>
              <strong>{snapshot().clock.bpm.toFixed(1)}</strong>
            </div>
            <div class="liveStatusItem">
              <span>Clock</span>
              <strong>{clockSourceLabel(snapshot().clock.source)}</strong>
            </div>
            <div class="liveStatusItem">
              <span>Sync</span>
              <strong class={`clockSyncLabel ${snapshot().clock.external_sync_locked ? "locked" : ""}`}>
                {clockSyncStatusLabel(snapshot().clock)}
              </strong>
            </div>
            <div class="liveStatusItem">
              <span>Timecode</span>
              <strong class="tabularNums" data-no-localize>{formatShowTimecode(snapshot().timeline.position_ms)}</strong>
            </div>
          </div>
          <div class="liveTransportGrid">
            <button onClick={triggerPreviousCue} disabled={snapshot().cues.length === 0}>
              Back
            </button>
            <button class="primary liveGoButton" onClick={triggerNextCue} disabled={snapshot().cues.length === 0}>
              GO
            </button>
            <button
              onClick={() => void setCueFadePaused(!snapshot().active_fade?.paused)}
              disabled={!snapshot().active_fade}
            >
              {snapshot().active_fade?.paused ? "Resume Fade" : "Pause Fade"}
            </button>
            <button
              onClick={() => void (snapshot().timeline.playing ? pauseTimeline() : playTimeline())}
              disabled={!snapshot().timeline.playing && snapshot().timeline.duration_ms === 0}
            >
              {snapshot().timeline.playing ? "Pause Timeline" : "Play Timeline"}
            </button>
            <button
              class={snapshot().blackout ? "primary" : ""}
              onClick={() => void setBlackout(!snapshot().blackout)}
            >
              {snapshot().blackout ? "Clear DMX BO" : "DMX BO"}
            </button>
            <button
              class={snapshot().video.blackout ? "primary" : ""}
              onClick={() => void setVideoBlackout(!snapshot().video.blackout)}
            >
              {snapshot().video.blackout ? "Clear Video BO" : "Video BO"}
            </button>
            <button
              class={snapshot().blackout && snapshot().video.blackout ? "primary" : ""}
              onClick={() => void setAllBlackout(true)}
              disabled={snapshot().blackout && snapshot().video.blackout}
            >
              All BO
            </button>
            <button
              onClick={() => void setAllBlackout(false)}
              disabled={!snapshot().blackout && !snapshot().video.blackout}
            >
              All Clear
            </button>
            <button disabled={!globalFixtureFlagState().anyFlagged} onClick={() => void clearFixtureFlags("all")}>
              Clear Flags
            </button>
          </div>
          <div class="liveCuePadHeader">
            <h3>Cue Pads</h3>
            <span>{cuePadRangeLabel()}</span>
            <label class="checkbox compactCheckbox">
              <input
                type="checkbox"
                checked={cuePadFollowActive()}
                onChange={(event) => setCuePadFollowActive(event.currentTarget.checked)}
              />
              Follow
            </label>
            <button
              onClick={() => {
                setCuePadFollowActive(false);
                setCuePadBank(Math.max(0, cuePadBank() - 1));
              }}
              disabled={cuePadBank() === 0}
            >
              Prev
            </button>
            <button
              onClick={() => {
                setCuePadFollowActive(false);
                setCuePadBank(Math.min(cuePadBankCount() - 1, cuePadBank() + 1));
              }}
              disabled={cuePadBank() >= cuePadBankCount() - 1}
            >
              Next
            </button>
          </div>
          <div class="liveCuePadGrid">
            <For each={liveCuePads()}>
              {(pad) => (
                <button
                  class={`liveCuePad ${pad.cue?.id === snapshot().active_cue_id ? "active" : ""} ${
                    pad.cue?.id === nextCue()?.id ? "next" : ""
                  }`}
                  disabled={!pad.cue}
                  onClick={() => {
                    if (pad.cue) {
                      void triggerCue(pad.cue.id);
                    }
                  }}
                >
                  <span>{pad.slot}</span>
                  <strong>{pad.cue?.label ?? "Empty"}</strong>
                  <small>{pad.cue ? `#${pad.index + 1} / ${pad.cue.fade_ms} ms` : "-"}</small>
                </button>
              )}
            </For>
          </div>
          <Show when={snapshot().active_fade}>
            {(fade) => (
              <div class="liveFadeMeter">
                <span>
                  Fade {Math.round(fade().progress * 100)}% / {fade().remaining_ms} ms
                </span>
                <progress max="1" value={fade().progress} />
              </div>
            )}
          </Show>
          <div class="liveMasterGrid">
            <label>
              Lighting Master
              <input
                type="range"
                min="0"
                max="1"
                step="0.01"
                value={snapshot().lighting_master}
                onChange={(event) => void setLightingMaster(Number(event.currentTarget.value))}
              />
            </label>
            <label>
              Video Master
              <input
                type="range"
                min="0"
                max="1"
                step="0.01"
                value={snapshot().video.master_opacity}
                onChange={(event) => void setVideoMasterOpacity(Number(event.currentTarget.value))}
              />
            </label>
            <label>
              BPM
              <input
                type="number"
                min="20"
                max="300"
                step="0.1"
                value={bpmDraft()}
                onInput={(event) => setBpmDraft(event.currentTarget.value)}
              />
            </label>
            <button class="primary" onClick={tapBpm}>
              Tap
            </button>
          </div>
        </section>
        <section class="panel controlPanel controlStagePanel">
          <div class="controlStageToolbar">
            <strong>Stage</strong>
            <div class="controlStageGroups" aria-label="Fixture groups">
              <button
                class={!selectedFixtureGroupFilter() ? "active" : ""}
                onClick={() => selectFixtureGroupFilter(null)}
              >
                All
              </button>
              <For each={fixtureGroupRows().slice(0, 8)}>
                {(group) => (
                  <button
                    class={selectedFixtureGroupFilter() === group.groupId ? "active" : ""}
                    onClick={() => selectFixtureGroupFilter(group.groupId)}
                    title={`${group.count} fixture(s)`}
                  >
                    <span data-no-localize>{group.groupId}</span>
                    <span>{group.count}</span>
                  </button>
                )}
              </For>
            </div>
            <span>{visualizerFixtures().length} fixtures</span>
          </div>
          <StagePreview2D
            className="controlStage"
            patternId="control-stage-grid"
            compact
            viewAspectRatio={2.4}
            stageOrigin={stageOrigin2d()}
            fixtures={visualizerFixtures()}
            videoSurfaces={visualizerVideoSurfaces2d()}
            stageObjects={visualizerStageObjects2d()}
            selectedFixtureId={selectedFixtureId()}
            selectedVideoOutputId={selectedVideoOutputId()}
            selectedFixtureGroupFilter={selectedFixtureGroupFilter()}
            selectedFixtureTypeFilter={selectedFixtureTypeFilter()}
            stageObjectClassName="controlStageObject"
            surfaceMinOpacity={0.22}
            beamMinOpacity={0.08}
            beamIntensityScale={0.55}
            fixtureRadiusIntensityScale={1.4}
            onSelectVideoOutput={setSelectedVideoOutputId}
            onSelectFixture={(fixtureId) => {
              const fixture = snapshot().fixtures.find((candidate) => candidate.id === fixtureId);
              if (fixture) activateFixture(fixture);
            }}
          />
          <div class="controlStageSelection">
            <strong>{selectedFixture()?.label ?? selectedFixtureGroupFilter() ?? "No selection"}</strong>
            <span>{controlTargetDetail()}</span>
            <button onClick={() => selectSetupMode("mapping")}>Edit Map</button>
          </div>
        </section>
        </Show>
        <Show when={workspaceTab() === "touch"}>
        <TouchCuePanel
          snapshot={snapshot()}
          activeCue={activeCue()}
          nextCue={nextCue()}
          cuePads={liveCuePads()}
          cuePadRangeLabel={cuePadRangeLabel()}
          cuePadBank={cuePadBank()}
          cuePadBankCount={cuePadBankCount()}
          onPreviousBank={() => {
            setCuePadFollowActive(false);
            setCuePadBank(Math.max(0, cuePadBank() - 1));
          }}
          onNextBank={() => {
            setCuePadFollowActive(false);
            setCuePadBank(Math.min(cuePadBankCount() - 1, cuePadBank() + 1));
          }}
          onTriggerPreviousCue={triggerPreviousCue}
          onTriggerNextCue={triggerNextCue}
          onTriggerCue={triggerCue}
          onSetCueFadePaused={setCueFadePaused}
          onSetLightingMaster={setLightingMaster}
          onSetVideoMasterOpacity={setVideoMasterOpacity}
          onSetBlackout={setBlackout}
          onSetVideoBlackout={setVideoBlackout}
          onSetAllBlackout={setAllBlackout}
        />
        <section class="panel touchPanel touchStagePanel">
          <div class="panelHeader">
            <h2>Touch Stage</h2>
            <span>
              {visualizerFixtures().length} fixture(s) / {snapshot().video.outputs.length} projection surface(s) /{" "}
              {visualizerStageObjects2d().length} ref(s)
            </span>
          </div>
          <StagePreview2D
            className="touchStage"
            patternId="touch-stage-grid"
            stageOrigin={stageOrigin2d()}
            fixtures={visualizerFixtures()}
            videoSurfaces={visualizerVideoSurfaces2d()}
            stageObjects={visualizerStageObjects2d()}
            selectedFixtureId={selectedFixtureId()}
            selectedVideoOutputId={selectedVideoOutputId()}
            selectedFixtureGroupFilter={selectedFixtureGroupFilter()}
            stageObjectClassName="touchStageObject"
            surfaceMinOpacity={0.2}
            beamMinOpacity={0.06}
            beamIntensityScale={0.48}
            fixtureRadiusIntensityScale={1.2}
            onSelectVideoOutput={setSelectedVideoOutputId}
            onSelectFixture={(fixtureId) => {
              const patchedFixture = snapshot().fixtures.find((candidate) => candidate.id === fixtureId);
              if (patchedFixture) {
                selectFixture(patchedFixture);
              }
            }}
          />
          <div class="touchStageSelectionBar">
            <Show
              when={selectedFixture()}
              fallback={
                <div class="touchStageSelectionItem empty">
                  <strong>No fixture selected</strong>
                  <span>Selection standby</span>
                </div>
              }
            >
              {(fixture) => (
                <div class="touchStageSelectionItem">
                  <div>
                    <strong>{fixture().label}</strong>
                    <span>U{fixture().universe} A{fixture().address} / {fixture().group_ids.join(", ") || fixture().mode_name}</span>
                  </div>
                  <button
                    class={touchDimmerQuickActive() ? "momentary active" : "momentary"}
                    disabled={!selectedTouchDimmerTarget()}
                    onPointerDown={startTouchDimmerFlash}
                    onPointerUp={endTouchDimmerFlash}
                    onPointerCancel={endTouchDimmerFlash}
                    onPointerLeave={endTouchDimmerFlash}
                    onBlur={endTouchDimmerFlash}
                    onKeyDown={handleTouchDimmerFlashKeyDown}
                    onKeyUp={handleTouchDimmerFlashKeyUp}
                  >
                    Flash
                  </button>
                  <button
                    class={fixture().highlighted ? "active" : ""}
                    onClick={() => void setFixtureHighlight(fixture().id, !fixture().highlighted)}
                  >
                    HL
                  </button>
                  <button
                    class={fixture().soloed ? "active" : ""}
                    onClick={() => void setFixtureSolo(fixture().id, !fixture().soloed)}
                  >
                    Solo
                  </button>
                  <button
                    class={fixture().parked ? "active" : ""}
                    onClick={() => void setFixturePark(fixture().id, !fixture().parked)}
                  >
                    Park
                  </button>
                </div>
              )}
            </Show>
            <Show when={selectedTouchVideoOutput()}>
              {(output) => (
                <div class="touchStageSelectionItem projector">
                  <div>
                    <strong>{output().label}</strong>
                    <span>
                      {output().width}x{output().height} / {output().enabled ? "Enabled" : "Disabled"} /{" "}
                      {output().blackout ? "Blackout" : `${Math.round(output().opacity * 100)}%`}
                    </span>
                  </div>
                  <button
                    class={output().blackout ? "active" : ""}
                    onClick={() => void setVideoOutputBlackout(output().id, !output().blackout)}
                  >
                    BO
                  </button>
                  <button onClick={() => void fadeVideoOutputOpacity(output().id, 0)}>
                    Out
                  </button>
                  <button onClick={() => void fadeVideoOutputOpacity(output().id, 1)}>
                    In
                  </button>
                  <button onClick={() => void openVideoOutputWindow(output().id, true)}>
                    Pattern
                  </button>
                </div>
              )}
            </Show>
          </div>
          <div class="touchStageLegend">
            <button
              class={!selectedFixtureGroupFilter() ? "active" : ""}
              onClick={() => selectFixtureGroupFilter(null)}
            >
              All
            </button>
            <For each={fixtureGroupRows().slice(0, 6)}>
              {(group) => (
                <button
                  class={selectedFixtureGroupFilter() === group.groupId ? "active" : ""}
                  onClick={() => selectFixtureGroupFilter(group.groupId)}
                >
                  <span data-no-localize>{group.groupId}</span>
                </button>
              )}
            </For>
          </div>
        </section>
        <TouchFixturePanel
          selectedGroupId={selectedFixtureGroupFilter()}
          selectedFixture={selectedFixture() ?? null}
          groups={fixtureGroupRows()}
          fixtures={filteredFixtures()}
          totalFixtureCount={snapshot().fixtures.length}
          selectedFixtureId={selectedFixtureId()}
          groupFlagState={selectedGroupFlagState()}
          globalAnyFlagged={globalFixtureFlagState().anyFlagged}
          categories={controlCategoryRows()}
          activeCategory={activeControlCategory()}
          activeCategoryLabel={activeControlCategoryLabel()}
          visibleControlCount={visibleControls().length}
          onSelectGroup={selectFixtureGroupFilter}
          onSelectFixture={selectFixture}
          onSetFixtureHighlight={setFixtureHighlight}
          onSetFixtureSolo={setFixtureSolo}
          onSetFixturePark={setFixturePark}
          onSetGroupHighlight={setGroupHighlight}
          onSetGroupSolo={setGroupSolo}
          onSetGroupPark={setGroupPark}
          onClearFixtureFlags={() => clearFixtureFlags("all")}
          onCategory={setControlCategory}
        >
          <Show when={showDimmerPanel() ? selectedDimmerControl() : undefined}>
            {(dimmer) => (
              <TouchDimmerControlPanel
                value={dimmer().value}
                sliderMin={dimmerSliderRange().min}
                sliderMax={dimmerSliderRange().max}
                canControl={Boolean(selectedTouchDimmerTarget())}
                flashActive={touchDimmerQuickActive()}
                formatDmxPercent={formatDmxPercent}
                onSetValue={setDimmerValue}
                onQuickLevel={setTouchDimmerQuickLevel}
                onFlashStart={startTouchDimmerFlash}
                onFlashEnd={endTouchDimmerFlash}
                onFlashKeyDown={handleTouchDimmerFlashKeyDown}
                onFlashKeyUp={handleTouchDimmerFlashKeyUp}
              />
            )}
          </Show>
          <Show when={showPositionPad() ? selectedPositionControls() : undefined}>
            {(positionControls) => (
              <TouchPanTiltPad
                panValue={positionControls().panValue}
                tiltValue={positionControls().tiltValue}
                limitOverlayStyle={selectedFixtureLimitOverlayStyle()}
                onPointerValue={setPanTiltFromPointer}
              />
            )}
          </Show>
          <Show when={showColorPad() ? selectedColorControls() : undefined}>
            <TouchColorPalettePanel colors={defaultColorPalette.slice(0, 8)} onSetColor={setFixtureColor} />
          </Show>
          <Show when={!showDimmerPanel() && !showPositionPad() && !showColorPad() && visibleControls().length > 0}>
            <TouchGenericAttributeGrid
              controls={visibleControls()}
              currentValue={currentControlValue}
              formatValue={formatShortDmxPercent}
              onSetValue={setControlAttributeValue}
            />
          </Show>
        </TouchFixturePanel>
        <TouchRemotePanel
          running={remoteRunning()}
          remoteUrls={remoteUrls()}
          bindIp={remoteBindIp()}
          port={remotePort()}
          pairingPin={remotePairingPin()}
          allowLan={remoteAllowLan()}
          status={remoteStatus()}
          bpmDraft={bpmDraft()}
          submasters={snapshot().submasters}
          onBindIp={setRemoteBindIp}
          onPort={setRemotePort}
          onPairingPin={setRemotePairingPin}
          onRegeneratePairingPin={() => setRemotePairingPin(createPairingPin())}
          onAllowLan={(value) => {
            setRemoteAllowLan(value);
            setRemoteBindIp(value ? "0.0.0.0" : "127.0.0.1");
          }}
          onCopyRemoteUrl={copyRemoteUrl}
          onOpenRemoteUrl={openRemoteUrl}
          onStart={startRemoteControl}
          onStop={stopRemoteControl}
          onBpmDraft={setBpmDraft}
          onApplyBpm={applyBpm}
          onTapBpm={tapBpm}
          onSetSubmaster={setGroupSubmaster}
        />
        <TouchVideoPanel
          layers={snapshot().video.layers}
          outputs={snapshot().video.outputs}
          selectedOutputId={selectedVideoOutputId()}
          onSetLayerState={setVideoLayerState}
          onAddCuePoint={addVideoCuePoint}
          onJumpCuePoint={jumpVideoCuePoint}
          onSelectOutput={setSelectedVideoOutputId}
          onSetOutputEnabled={setVideoOutputEnabled}
          onSetOutputBlackout={setVideoOutputBlackout}
          onSetOutputOpacity={setVideoOutputOpacity}
          onFadeOutputOpacity={fadeVideoOutputOpacity}
          onOpenOutputWindow={openVideoOutputWindow}
        />
        </Show>
        <Show when={workspaceTab() === "setup" && ["library", "profiles", "patch"].includes(setupSubTab())}>
        <aside
          class={setupPanelClass("panel setup setupPanel", ["library", "profiles", "patch"])}
          ref={registerSetupPanel(["library", "profiles", "patch"])}
          tabIndex={-1}
        >
          <Show when={setupSubTab() === "library" || setupSubTab() === "patch"}>
          <ProfileLoadPanel
            title={setupSubTab() === "library" ? "Fixture Library" : "Patch Source"}
            gdtfPath={gdtfPath()}
            gdtfShareUrl={gdtfShareUrl()}
            onGdtfPath={setGdtfPath}
            onGdtfShareUrl={setGdtfShareUrl}
            onBrowse={selectGdtfFile}
            onLoadGdtf={importGdtf}
            onDownloadGdtf={downloadGdtfFromUrl}
          />
          </Show>
          <Show when={setupSubTab() === "profiles"}>
          <CustomProfileEditorPanel
            manufacturer={customManufacturer()}
            profileName={customProfileName()}
            modeName={customModeName()}
            attributesText={customAttributes()}
            templates={customProfileAttributeTemplates}
            drafts={customAttributeDrafts()}
            selectedIndex={selectedCustomAttributeIndexValue()}
            rowConflicts={customProfileDraftAnalysis().rowConflicts}
            dmxCells={customProfileDmxCells()}
            preview={customProfilePreview()}
            rowStatusText={customProfileDraftRowStatusText}
            onManufacturer={setCustomManufacturer}
            onProfileName={setCustomProfileName}
            onModeName={setCustomModeName}
            onAttributesText={setCustomAttributesText}
            onAppendTemplate={appendCustomAttributeTemplate}
            onSelectIndex={setSelectedCustomAttributeIndex}
            onUpdateDraft={updateCustomAttributeDraft}
            onRemoveDraft={removeCustomAttributeDraft}
            onAddDraft={addCustomAttributeDraft}
            onMoveDraft={moveCustomAttributeDraft}
            onClearDrafts={() => commitCustomAttributeDrafts([], null)}
            onCreate={createCustomProfile}
            onSave={saveCustomProfile}
            onLoad={loadCustomProfile}
          />
          </Show>

          <Show when={setupSubTab() !== "profiles" && profile()}>
            {(loaded) => (
              <div class="profile">
                <LoadedProfileSummaryPanel
                  profile={loaded()}
                  selectedMode={selectedMode()}
                  selectedModeSummary={selectedModeSummary()}
                  selectedFootprint={selectedFootprint()}
                  dmxCells={selectedModeDmxCells()}
                  functionEntries={selectedModeFunctionEntries()}
                  visibleFunctionEntries={visibleSelectedModeFunctionEntries()}
                  geometryRows={selectedModeGeometryRows()}
                  unresolvedGeometryReferences={selectedModeUnresolvedGeometryReferences()}
                  functionLabel={channelFunctionLabel}
                  functionRangeLabel={channelFunctionRangeLabel}
                  functionDetail={channelFunctionDetail}
                  onSelectedMode={setSelectedMode}
                />
                <Show when={setupSubTab() === "patch"}>
                <PatchFixtureFormPanel
                  label={label()}
                  universe={universe()}
                  address={address()}
                  count={patchCount()}
                  addressStride={patchAddressStride()}
                  layoutMode={patchLayoutMode()}
                  gridColumns={patchGridColumns()}
                  circleRadius={patchCircleRadius()}
                  x={patchX()}
                  y={patchY()}
                  z={patchZ()}
                  xStep={patchXStep()}
                  zStep={patchZStep()}
                  pitch={patchPitch()}
                  yaw={patchYaw()}
                  roll={patchRoll()}
                  groupText={groupText()}
                  footprint={selectedFootprint()}
                  normalizedCount={patchCountValue()}
                  normalizedAddressStride={patchAddressStrideValue()}
                  normalizedGridColumns={patchGridColumnsValue()}
                  normalizedCircleRadius={patchCircleRadiusValue()}
                  endAddress={endAddress()}
                  conflictText={patchAddressConflictText()}
                  invalid={patchAddressInvalid()}
                  nextFreeAddress={nextFreePatchAddress()}
                  warnings={loaded().warnings}
                  onLabel={setLabel}
                  onUniverse={setUniverse}
                  onAddress={setAddress}
                  onCount={setPatchCount}
                  onAddressStride={setPatchAddressStride}
                  onLayoutMode={setPatchLayoutMode}
                  onGridColumns={setPatchGridColumns}
                  onCircleRadius={setPatchCircleRadius}
                  onX={setPatchX}
                  onY={setPatchY}
                  onZ={setPatchZ}
                  onXStep={setPatchXStep}
                  onZStep={setPatchZStep}
                  onPitch={setPatchPitch}
                  onYaw={setPatchYaw}
                  onRoll={setPatchRoll}
                  onGroupText={setGroupText}
                  onPatch={patchFixture}
                  onNextFreeAddress={selectNextFreePatchAddress}
                />
                </Show>
              </div>
            )}
          </Show>
        </aside>
        </Show>

        <Show when={workspaceTab() === "setup" && ["patch", "mapping"].includes(setupSubTab())}>
        <SetupMappingWorkspace
          className={setupPanelClass("panel fixtures setupPanel", ["patch", "mapping"])}
          panelRef={registerSetupPanel(["patch"])}
          compact={setupSubTab() === "patch"}
          onOpenMapping={() => selectSetupMode("mapping")}
          fixtureList={{
            fixtures: filteredFixtures(),
            totalFixtureCount: snapshot().fixtures.length,
            selectedGroupId: selectedFixtureGroupFilter(),
            groupRows: fixtureGroupRows(),
            selectedFixtureId: selectedFixtureId(),
            onSelectGroup: selectFixtureGroupFilter,
            onSelectFixture: selectFixture,
          }}
          patchMap={{
            activeUniverse: activePatchGridUniverse(),
            universeOptions: patchGridUniverseOptions(),
            viewMode: dmxPatchViewMode(),
            nextFreeAddress: nextFreePatchAddress(),
            activeMap: activePatchGridMap(),
            addressCells: dmxAddressCells(),
            universeMaps: dmxUniverseMaps(),
            fixtureCount: snapshot().fixtures.length,
            selectedFixtureId: selectedFixtureId(),
            plannedAddressSummary: plannedAddressSummary(),
            onNextFreeAddress: selectNextFreePatchAddress,
            onUniverse: setPatchGridUniverse,
            onViewMode: setDmxPatchViewMode,
            onSelectFixture: selectFixture,
            onAddressCell: handleDmxAddressCellClick,
          }}
          fixtureEditor={selectedFixture() ? {
            fixture: selectedFixture()!,
            labelDraft: selectedFixtureLabelDraft(),
            universeDraft: selectedFixtureUniverseDraft(),
            addressDraft: selectedFixtureAddressDraft(),
            groupText: selectedFixtureGroupText(),
            limitsDraft: selectedFixtureLimitsDraft(),
            normalizedLimits: normalizedSelectedFixtureLimitsDraft(),
            limitWindowStyle: selectedFixtureLimitWindowStyle(),
            movementLimitDragging: Boolean(movementLimitDrag()),
            filteredFixtureCount: filteredFixtures().length,
            formatDmxPercent,
            onUseProfileForPatch: useFixtureProfileForPatch,
            onDuplicateFixture: duplicateFixture,
            onLabelDraft: setSelectedFixtureLabelDraft,
            onUniverseDraft: setSelectedFixtureUniverseDraft,
            onAddressDraft: setSelectedFixtureAddressDraft,
            onApplyPatch: setFixturePatch,
            onGroupText: setSelectedFixtureGroupText,
            onApplyGroups: setFixtureGroups,
            onUpdateNumericLimit: (field, value) => updateSelectedFixtureLimit(field, value),
            onUpdateToggleLimit: (field, value) => updateSelectedFixtureLimit(field, value),
            onResetLimits: () => setSelectedFixtureLimitsDraft(defaultFixtureLimits),
            onApplyLimits: setFixtureLimits,
            onMovementLimitPointerDown: startMovementLimitDrag,
            onMovementLimitPointerMove: dragMovementLimit,
            onMovementLimitPointerEnd: endMovementLimitDrag,
            onSetTransform: setFixtureTransform,
            onLayoutFixtures: layoutFixturePositions,
          } : null}
          fixtureCount={mappingFilteredFixtures().length}
          projectorCount={snapshot().video.outputs.length}
          filters={{
            fixtureCount: snapshot().fixtures.length,
            filteredFixtureCount: filteredFixtures().length,
            selectedGroupId: selectedFixtureGroupFilter(),
            groupRows: fixtureGroupRows(),
            selectedTypeKey: selectedFixtureTypeFilter(),
            fixtureTypeRows: fixtureTypeRows(),
            onSelectGroup: selectFixtureGroupFilter,
            onSelectType: setSelectedFixtureTypeFilter,
          }}
          toolRail={{
            stageTool: mappingStageTool(),
            showLabels: setupSubTab() === "patch" ? false : mappingShowLabels(),
            showBeams: mappingShowBeams(),
            showGeometry: mappingShowGeometry(),
            showProjectors: mappingShowProjectors(),
            showStageObjects: mappingShowStageObjects(),
            showLevels: setupSubTab() === "patch" ? false : mappingShowLevels(),
            helpOpen: mappingHotkeyHelpOpen(),
            onStageTool: setMappingStageTool,
            onToggleLabels: () => setMappingShowLabels((value) => !value),
            onToggleBeams: () => setMappingShowBeams((value) => !value),
            onToggleGeometry: () => setMappingShowGeometry((value) => !value),
            onToggleProjectors: () => setMappingShowProjectors((value) => !value),
            onToggleStageObjects: () => setMappingShowStageObjects((value) => !value),
            onToggleLevels: () => setMappingShowLevels((value) => !value),
            onToggleHelp: () => setMappingHotkeyHelpOpen((open) => !open),
          }}
          viewportControls={{
            stageTool: mappingStageTool(),
            fixtureCount: mappingFilteredFixtures().length,
            outputCount: snapshot().video.outputs.length,
            objectCount: snapshot().stage_objects.length,
            cursorReadout: mappingStageCursorWorld()
              ? `${mappingStageCursorLabel()} / ${mappingStageTool().toUpperCase()}`
              : null,
            canFitVisible: canFitMappingViewportToVisible(),
            canFitSelection: canFitMappingViewportToSelection(),
            zoomLabel: mappingViewportZoomLabel(),
            canZoomOut: normalizedMappingViewportZoom() > 1.001,
            canZoomIn: normalizedMappingViewportZoom() < 3.999,
            canResetZoom: normalizedMappingViewportZoom() > 1.001,
            snapEnabled: mappingSnapEnabled(),
            snapSize: normalizedMappingSnapSize(),
            showLabels: mappingShowLabels(),
            showBeams: mappingShowBeams(),
            showGeometry: mappingShowGeometry(),
            showProjectors: mappingShowProjectors(),
            showStageObjects: mappingShowStageObjects(),
            showLevels: mappingShowLevels(),
            onFitVisible: fitMappingViewportToVisible,
            onFitSelection: fitMappingViewportToSelection,
            onZoomOut: () => zoomMappingViewport(-1),
            onZoomIn: () => zoomMappingViewport(1),
            onResetZoom: resetMappingViewport,
            onSnapOff: () => setMappingSnapEnabled(false),
            onSnapPreset: (preset) => {
              setMappingSnapSize(preset);
              setMappingSnapEnabled(true);
            },
            onSnapSizeChange: (size) => {
              setMappingSnapSize(size);
              setMappingSnapEnabled(true);
            },
            onShowLabels: setMappingShowLabels,
            onShowBeams: setMappingShowBeams,
            onShowGeometry: setMappingShowGeometry,
            onShowProjectors: setMappingShowProjectors,
            onShowStageObjects: setMappingShowStageObjects,
            onShowLevels: setMappingShowLevels,
          }}
          stageConfig={{
            stageMap: snapshot().stage_map,
            stageWorldBounds: stageWorldBounds(),
            stageMapPresetLabel: stageMapPresetLabel(),
            selectedStageMapPresetLabel: selectedStageMapPresetLabel(),
            stageMapPresets: snapshot().stage_map_presets,
            mappingViewPresetLabel: mappingViewPresetLabel(),
            selectedMappingViewPresetId: selectedMappingViewPresetId(),
            mappingViewPresets: mappingViewPresets(),
            stageMapPresetLabelFor: stageMapPresetObjectCountLabel,
            mappingViewPresetLabelFor: mappingViewPresetObjectLabel,
            onLockToCurrentBounds: lockStageMapToCurrentBounds,
            onSetStageMapConfig: setStageMapConfig,
            onStageMapPresetLabel: setStageMapPresetLabel,
            onSelectedStageMapPresetLabel: setSelectedStageMapPresetLabel,
            onSaveStageMapPreset: saveStageMapPreset,
            onExportStageMapPreset: exportStageMapPreset,
            onImportStageMapPreset: importStageMapPreset,
            onApplyStageMapPreset: applyStageMapPreset,
            onRemoveStageMapPreset: removeStageMapPreset,
            onMappingViewPresetLabel: setMappingViewPresetLabel,
            onSelectedMappingViewPresetId: setSelectedMappingViewPresetId,
            onSaveMappingViewPreset: saveMappingViewPreset,
            onApplyMappingViewPreset: applyMappingViewPreset,
            onRemoveMappingViewPreset: removeMappingViewPreset,
            onExportMappingStageSvg: exportMappingStageSvg,
            onExportVisualizerRenderPayload: exportVisualizerRenderPayload,
          }}
          editableStage={{
            svgRef: (element) => { mappingStageSvgElement = element; },
            dragging: Boolean(mappingDrag() || mappingViewportPanDrag()),
            stageTool: mappingStageTool(),
            viewBox: setupSubTab() === "patch" ? compactMappingStageViewBox() : mappingStageViewBox(),
            stageOrigin: stageOrigin2d(),
            cursorPoint: mappingStageCursorSvgPoint(),
            cursorLabel: mappingStageCursorLabel(),
            snapEnabled: mappingSnapEnabled(),
            snapLines: mappingSnapLines(),
            marqueeBox: mappingMarqueeBox(),
            onPointerDown: handleMappingStagePointerDown,
            onPointerMove: handleMappingStagePointerMove,
            onPointerUp: finishMappingStageDrag,
            onPointerLeave: () => setMappingStageCursorWorld(null),
            onWheel: handleMappingStageWheel,
          }}
          stageLayers={{
            showStageObjects: mappingShowStageObjects(),
            showProjectors: mappingShowProjectors(),
            showBeams: mappingShowBeams(),
            showGeometry: mappingShowGeometry(),
            showLabels: mappingShowLabels(),
            showLevels: mappingShowLevels(),
            stageTool: mappingStageTool(),
            stageObjects: visualizerStageObjects2d(),
            videoSurfaces: visualizerVideoSurfaces2d(),
            beamFixtures: visualizerFixtures(),
            geometryNodes: mappingGeometryNodes2d(),
            fixtures: visualizerFixtures(),
            selectedFixtureIds: selectedMappingFixtureIdSet(),
            selectedFixtureId: selectedFixtureId(),
            selectedGroupId: selectedFixtureGroupFilter(),
            selectedTypeKey: selectedFixtureTypeFilter(),
            selectedVideoOutputId: selectedVideoOutputId(),
            placePreview: mappingPlacePreview(),
            isDraggingStageObject: isDraggingMappingStageObject,
            isDraggingVideoOutput: isDraggingMappingVideoOutput,
            isDraggingFixture: isDraggingMappingFixture,
            isYawDragging: (fixtureId) => {
              const drag = mappingDrag();
              return drag?.kind === "fixtureYaw" && drag.fixtureId === fixtureId;
            },
            surfaceMapping: (surface) => {
              const output = snapshot().video.outputs.find((candidate) => candidate.id === surface.id);
              return output ? mappingVideoOutputMapping(output) : defaultVideoOutputMapping;
            },
            cornerLocals: mappingVideoSurfaceCornerLocals,
            cornerPointList: mappingVideoSurfaceCornerPointList,
            onBeginStageObjectDrag: beginMappingStageObjectDrag,
            onBeginStageObjectRotate: beginMappingStageObjectRotate,
            onBeginStageObjectResize: beginMappingStageObjectResize,
            onSelectVideoOutput: setSelectedVideoOutputId,
            onBeginVideoOutputDrag: beginMappingVideoOutputDrag,
            onBeginVideoOutputCornerDrag: beginMappingVideoOutputCornerDrag,
            onBeginVideoOutputRotate: beginMappingVideoOutputRotate,
            onBeginVideoOutputScale: beginMappingVideoOutputScale,
            onBeginFixtureYawDrag: beginMappingFixtureYawDrag,
            onFixturePointerDown: (event, fixtureId) => {
              if (mappingStageTool() === "pan") return;
              event.stopPropagation();
              const fixture = snapshot().fixtures.find((candidate) => candidate.id === fixtureId);
              if (fixture && mappingStageTool() === "rotate") selectMappingFixture(fixture, event);
              beginMappingFixtureDrag(event, fixtureId);
            },
          }}
          selectionSidebar={{
            selectedFixtureCount: selectedMappingFixtures().length,
            filteredFixtureCount: mappingFilteredFixtures().length,
            fixtureSearch: mappingFixtureSearch(),
            groupText: mappingSelectionGroupText(),
            groupTokenCount: parseGroupIds(mappingSelectionGroupText()).length,
            stageObjects: snapshot().stage_objects,
            stageObjectFixtureCounts: stageObjectFixtureCounts(),
            selectedStageObject: selectedStageObject(),
            selectedStageObjectId: selectedStageObjectId(),
            stageObjectDraftLabel: stageObjectLabel(),
            stageObjectDraftKind: stageObjectKind(),
            stageObjectDraftWidth: stageObjectWidth(),
            stageObjectDraftDepth: stageObjectDepth(),
            stageObjectDraftRotation: stageObjectRotation(),
            stageObjectDraftColor: stageObjectColor(),
            flagState: selectedMappingFlagState(),
            snapSize: normalizedMappingSnapSize(),
            selectedFixture: selectedMappingFixture(),
            selectedGeometryRows: selectedMappingGeometryRows(),
            unresolvedGeometryReferences: selectedMappingUnresolvedGeometryReferences(),
            filteredFixtures: mappingFilteredFixtures(),
            selectedFixtureIds: selectedMappingFixtureIdSet(),
            outputs: snapshot().video.outputs,
            selectedOutput: selectedMappingVideoOutput(),
            selectedOutputId: selectedVideoOutputId(),
            onSearch: setMappingFixtureSearch,
            onGroupText: setMappingSelectionGroupText,
            onPickVisible: pickVisibleMappingFixtures,
            onDuplicateSelected: duplicateSelectedMappingFixtures,
            onRemoveSelected: removeSelectedMappingFixtures,
            onClearSelection: clearMappingFixtureSelection,
            onApplyGroups: applyMappingSelectionGroups,
            onStageObjectDraftLabel: setStageObjectLabel,
            onStageObjectDraftKind: setStageObjectKind,
            onStageObjectDraftWidth: setStageObjectWidth,
            onStageObjectDraftDepth: setStageObjectDepth,
            onStageObjectDraftRotation: setStageObjectRotation,
            onStageObjectDraftColor: setStageObjectColor,
            onAddStageObjectCenter: addStageObjectAtCenter,
            onSelectStageObject: setSelectedStageObjectId,
            onSetStageObject: setStageObject,
            onRemoveStageObject: removeStageObject,
            onPickInsideStageObject: pickFixturesInsideSelectedStageObject,
            onLayoutOnStageObject: layoutSelectedFixturesOnStageObject,
            onSetSelectionFlag: setMappingSelectionFlag,
            onNudgeSelection: nudgeSelectedMappingFixtures,
            onLayoutSelection: layoutSelectedMappingFixtures,
            onAlignSelection: alignSelectedMappingFixtures,
            onDistributeSelection: distributeSelectedMappingFixtures,
            onMirrorSelection: mirrorSelectedMappingFixtures,
            onRotateSelection: rotateSelectedMappingFixtures,
            onControlActive: () => setWorkspaceTab("control"),
            onUseSelectionAsEffectTarget: useMappingSelectionAsEffectTarget,
            onUseSelectionAsWaveEffectTarget: useMappingSelectionAsWaveEffectTarget,
            onSetFixtureTransform: setFixtureTransform,
            onSetFixtureHighlight: setFixtureHighlight,
            onSetFixtureSolo: setFixtureSolo,
            onSetFixturePark: setFixturePark,
            onControlFixture: () => setWorkspaceTab("control"),
            onPatchFixture: () => selectSetupMode("patch"),
            onSelectFixture: selectMappingFixture,
            onSelectOutput: setSelectedVideoOutputId,
            onSetOutputEnabled: setVideoOutputEnabled,
            onSetOutputBlackout: setVideoOutputBlackout,
            onOpenOutputWindow: openVideoOutputWindow,
            onSyncOutputWindow: syncVideoOutputWindow,
            onFitOutputToStageObject: fitVideoOutputToStageObject,
            onSetOutputMapping: setVideoOutputMapping,
          }}
          hotkeyHelpOpen={mappingHotkeyHelpOpen()}
          onCloseHotkeyHelp={() => setMappingHotkeyHelpOpen(false)}
        />
        </Show>

        <Show when={workspaceTab() === "setup" && setupSubTab() === "video"}>
        <SetupVideoPanel
          className={setupPanelClass("panel videoSetupPanel setupPanel", ["video"])}
          panelRef={registerSetupPanel(["video"])}
          outputs={snapshot().video.outputs}
          compositions={snapshot().video.compositions}
          layers={snapshot().video.layers}
          mappingPresets={snapshot().video.mapping_presets}
          compositionLabel={videoCompositionLabel()}
          compositionLayerIds={videoCompositionLayerIds()}
          outputLabel={videoOutputLabel()}
          outputKind={videoOutputKind()}
          outputWidth={videoOutputWidth()}
          outputHeight={videoOutputHeight()}
          outputFadeMs={videoOutputFadeMs()}
          outputMonitorId={videoOutputMonitorId()}
          outputFullscreen={videoOutputFullscreen()}
          outputEndpoint={videoOutputEndpoint()}
          mappingPresetLabel={videoOutputMappingPresetLabel()}
          selectedMappingPresetLabel={selectedVideoOutputMappingPresetLabel()}
          selectedOutputId={selectedVideoOutputId()}
          previewOutputId={videoOutputPreviewId()}
          previewMode={videoOutputPreviewMode()}
          previewInfo={videoOutputPreviewInfo()}
          previewUrl={videoOutputPreviewUrl()}
          configDraftFor={videoOutputConfigDraft}
          onCompositionLabel={setVideoCompositionLabel}
          onToggleCompositionLayer={toggleVideoCompositionLayer}
          onAddComposition={addVideoComposition}
          onRemoveComposition={removeVideoComposition}
          onSetCompositionLayers={setVideoCompositionLayers}
          onMoveCompositionLayer={moveVideoCompositionLayer}
          onOutputLabel={setVideoOutputLabel}
          onOutputKind={setVideoOutputKind}
          onOutputWidth={setVideoOutputWidth}
          onOutputHeight={setVideoOutputHeight}
          onOutputFadeMs={setVideoOutputFadeMs}
          onOutputMonitorId={setVideoOutputMonitorId}
          onOutputFullscreen={setVideoOutputFullscreen}
          onOutputEndpoint={setVideoOutputEndpoint}
          onAddOutput={addVideoOutput}
          onConfigDraft={updateVideoOutputConfigDraft}
          onApplyConfig={setVideoOutputConfig}
          onSelectOutput={setSelectedVideoOutputId}
          onSetRouting={setVideoOutputRouting}
          onMappingPresetLabel={setVideoOutputMappingPresetLabel}
          onSelectedMappingPresetLabel={setSelectedVideoOutputMappingPresetLabel}
          onSaveMappingPreset={saveVideoOutputMappingPreset}
          onExportMappingPreset={exportVideoOutputMappingPreset}
          onImportMappingPreset={importVideoOutputMappingPreset}
          onApplyMappingPreset={applyVideoOutputMappingPreset}
          onRemoveMappingPreset={removeVideoOutputMappingPreset}
          onSetMapping={setVideoOutputMapping}
          onImportBitmapMask={importVideoOutputBitmapMask}
          onClearBitmapMask={clearVideoOutputBitmapMask}
          onSetEnabled={setVideoOutputEnabled}
          onSetBlackout={setVideoOutputBlackout}
          onSetOpacity={setVideoOutputOpacity}
          onFadeOpacity={fadeVideoOutputOpacity}
          onPreview={renderDebugVideoOutputPreview}
          onOpenWindow={openVideoOutputWindow}
          onSyncWindow={syncVideoOutputWindow}
          onRemoveOutput={removeVideoOutput}
        />
        </Show>

        <Show when={workspaceTab() === "control"}>
        <VideoControlPanel
          mixer={controlMode() === "mixer"}
          layerCount={snapshot().video.layers.length}
          previewDiagnostics={{
            get layerCount() { return snapshot().video.layers.length; },
            get info() { return videoPreviewInfo(); },
            get diagnosticsText() { return videoPreviewDiagnosticsText(); },
            get layerDiagnostics() { return videoPreviewLayerDiagnostics(); },
            get outputDecodePlans() { return videoPreviewOutputDecodePlans(); },
            onRenderPreview: renderDebugVideoPreview,
            onRefreshDiagnostics: refreshVideoPreviewDiagnostics,
            layerClass: videoPreviewLayerDiagnosticClass,
            layerLabel: videoPreviewLayerDiagnosticLabel,
            outputClass: videoPreviewOutputDecodePlanClass,
            outputLabel: videoPreviewOutputDecodePlanLabel,
          }}
          renderPlanStatus={{
            get summary() { return videoOutputRenderPlanSummary(); },
            get checked() { return videoOutputRenderPlans() !== null; },
            get rows() { return videoOutputRenderPlanRows(); },
            onRefresh: refreshVideoOutputRenderPlans,
          }}
          backendStatus={{
            get summary() { return videoRuntimeBackendSummary(); },
            get backends() { return videoRuntimeStatus()?.backends ?? null; },
            backendClass: videoRuntimeBackendClass,
            onRefresh: refreshVideoRuntimeStatus,
          }}
          externalIoStatus={{
            get ioSummary() { return externalVideoIoPlanSummary(); },
            get transportSummary() { return externalVideoTransportSummary(); },
            get checked() { return externalVideoIoPlans() !== null; },
            get planRows() { return externalVideoIoPlanRows(); },
            get activeTransportRows() { return externalVideoTransportActiveRows(); },
            get transportRows() { return externalVideoTransportRows(); },
            get transportEventRows() { return externalVideoTransportEventRows(); },
            planClass: externalVideoIoPlanClass,
            transportClass: externalVideoTransportClass,
            onRefreshPlans: refreshExternalVideoIoPlans,
            onSyncRoutes: syncExternalVideoTransports,
          }}
          masterControls={{
            get masterOpacity() { return snapshot().video.master_opacity; },
            get blackout() { return snapshot().video.blackout; },
            onSetMasterOpacity: setVideoMasterOpacity,
            onSetBlackout: setVideoBlackout,
          }}
          outputControls={{
            get outputs() { return snapshot().video.outputs; },
            get compositions() { return snapshot().video.compositions; },
            get selectedOutputId() { return selectedVideoOutputId(); },
            get fadeMs() { return videoOutputFadeMs(); },
            get windowSummary() { return videoOutputWindowSummary(); },
            onSelectOutput: setSelectedVideoOutputId,
            onSetFadeMs: setVideoOutputFadeMs,
            onRefreshWindows: refreshVideoOutputWindowStatuses,
            onOpenAllWindows: openAllVideoOutputWindows,
            onSyncOpenWindows: syncOpenVideoOutputWindows,
            onCloseOpenWindows: closeOpenVideoOutputWindows,
            renderPlanState: videoOutputRenderPlanState,
            windowStatusForOutput: videoOutputWindowStatusForOutput,
            windowState: videoOutputWindowState,
            onSetOutputEnabled: setVideoOutputEnabled,
            onSetOutputBlackout: setVideoOutputBlackout,
            onFadeOutputOpacity: fadeVideoOutputOpacity,
            onSetOutputOpacity: setVideoOutputOpacity,
            onOpenOutputWindow: openVideoOutputWindow,
            onSyncOutputWindow: syncVideoOutputWindow,
            onCloseOutputWindow: closeVideoOutputWindow,
          }}
          previewImage={{
            get previewUrl() { return videoPreviewUrl(); },
            get layerCount() { return snapshot().video.layers.length; },
          }}
          sourceCreate={{
            get sourceKind() { return videoSourceKind(); },
            get label() { return videoLabel(); },
            get path() { return videoPath(); },
            onSetSourceKind: setVideoSourceKind,
            onSetLabel: setVideoLabel,
            onSetPath: setVideoPath,
            onBrowseSource: selectVideoSourceFile,
            onImportMultiple: importMediaFiles,
            onAddLayer: addVideoLayer,
          }}
          clipGrid={{
            get layers() { return snapshot().video.layers; },
            get thumbnails() { return videoClipThumbnails(); },
            get fadeMs() { return videoOutputFadeMs(); },
            get audioMonitorVolume() { return videoAudioMonitorVolume(); },
            get audioMonitorLayerIds() { return videoAudioMonitorStatus().active_layer_ids; },
            get audioMonitorStatus() { return videoAudioMonitorStatus(); },
            get programAudioEnabled() { return videoProgramAudioEnabled(); },
            get audioOutputDevices() { return audioOutputDevices(); },
            get selectedAudioOutputDevice() { return selectedAudioOutputDevice(); },
            get deckALayerId() { return videoDeckALayerId(); },
            get deckBLayerId() { return videoDeckBLayerId(); },
            get abMix() { return videoAbMix(); },
            get selectedOutputId() { return selectedVideoOutputId(); },
            get recordingStatus() { return videoRecordingStatus(); },
            get liveAudioInputDevices() { return liveAudioInputDevices(); },
            get selectedLiveAudioInputDevice() { return selectedLiveAudioInputDevice(); },
            get liveAudioInputStatus() { return liveAudioInputStatus(); },
            onSetFadeMs: setVideoOutputFadeMs,
            onSetAudioMonitorVolume: setVideoAudioMonitorVolume,
            onSetProgramAudioEnabled: setVideoProgramAudioEnabled,
            onSetAudioOutputDevice: setSelectedAudioOutputDevice,
            onRefreshAudioOutputDevices: refreshAudioOutputDevices,
            onAssignDeck: assignVideoDeck,
            onSetAbMix: applyVideoAbMix,
            onCommitAbMix: (mix) => applyVideoAbMix(mix, true),
            onLaunchDeck: launchVideoDeck,
            onStartRecording: startVideoOutputRecording,
            onStopRecording: stopVideoOutputRecording,
            onSetLiveAudioInputDevice: setSelectedLiveAudioInputDevice,
            onRefreshLiveAudioInputDevices: refreshLiveAudioInputDevices,
            onStartLiveAudioInput: startLiveAudioInput,
            onStopLiveAudioInput: stopLiveAudioInput,
            onImportMedia: importMediaFiles,
            onLaunch: launchVideoClipFromGrid,
            onTake: takeVideoClipFromGrid,
            onStop: stopVideoClipFromGrid,
            onMonitorAudio: (layerId, volume) => playVideoLayerAudioMonitor(layerId, volume, selectedAudioOutputDevice()),
            onStopAudio: stopVideoLayerAudioMonitor,
          }}
          layerList={{
            get layers() { return snapshot().video.layers; },
            get isfRuntimeError() { return videoPreviewDiagnostics()?.last_isf_error; },
            onSetLayerLabel: setVideoLayerLabel,
            onMoveLayer: moveVideoLayer,
            onDuplicateLayer: duplicateVideoLayer,
            onRefreshMetadata: refreshVideoLayerMetadata,
            onSetBlendMode: setVideoLayerBlendMode,
            onSetLayerState: setVideoLayerState,
            onSetLayerTransform: setVideoLayerTransform,
            onSetLayerColor: setVideoLayerColor,
            onSetLayerFx: setVideoLayerFx,
            onImportIsf: importVideoLayerIsf,
            onSetIsfEffect: setVideoLayerIsfEffect,
            onAddCuePoint: addVideoCuePoint,
            onJumpCuePoint: jumpVideoCuePoint,
            onRemoveCuePoint: removeVideoCuePoint,
            onRemoveLayer: removeVideoLayer,
          }}
          timelineAutomation={{
            get layers() { return snapshot().video.layers; },
            get selectedLayerId() { return selectedVideoAutomationLayerId(); },
            get param() { return videoAutomationParam(); },
            get interpolation() { return videoAutomationInterpolation(); },
            get startMs() { return videoAutomationStartMs(); },
            get endMs() { return videoAutomationEndMs(); },
            get startValue() { return videoAutomationStartValue(); },
            get endValue() { return videoAutomationEndValue(); },
            get automations() { return timelineVideoAutomationRows(); },
            get selectedAutomationId() { return selectedVideoTimelineAutomationId(); },
            get rowScope() { return videoAutomationRowScope(); },
            get allRowsCount() { return allTimelineVideoAutomationRows().length; },
            draftForAutomation: timelineVideoAutomationDraft,
            onSetLayerId: setVideoAutomationLayerId,
            onSetParam: setVideoAutomationParam,
            onSetInterpolation: setVideoAutomationInterpolation,
            onSetStartMs: setVideoAutomationStartMs,
            onSetEndMs: setVideoAutomationEndMs,
            onSetStartValue: setVideoAutomationStartValue,
            onSetEndValue: setVideoAutomationEndValue,
            onRowScope: setVideoAutomationRowScope,
            onUsePlayheadRange: usePlayheadForVideoAutomation,
            onAddAutomation: addTimelineVideoAutomation,
            onUpdateAutomationDraft: updateTimelineVideoAutomationDraft,
            onAlignDraftToPlayhead: alignVideoAutomationDraftToPlayhead,
            onAddKeyframeAtPlayhead: addVideoAutomationKeyframeAtPlayhead,
            onRemoveKeyframeAtPlayhead: removeVideoAutomationKeyframeAtPlayhead,
            onRemoveKeyframe: removeVideoAutomationKeyframe,
            onSetKeyframeInterpolation: setVideoAutomationKeyframeInterpolation,
            onSetKeyframeValue: setVideoAutomationKeyframeValue,
            onSetAutomationEnabled: (automation, enabled) => setTimelineAutomationEnabled(automation.id, enabled),
            onSetRowsEnabled: setVideoAutomationRowsEnabled,
            onSeekKeyframe: seekTimeline,
            onSaveAutomation: setTimelineVideoAutomation,
            onRemoveAutomation: removeTimelineAutomation,
          }}
        />

        <section
          class={`panel faders controlPanel timelineDesk-${timelineDeskSurface()} editDesk-${editDeskSurface()}`}
        >
          <div class="panelHeader">
            <h2>Faders</h2>
            <Show when={controlMode() === "live"}>
              <nav class="timelineDeskTabs" aria-label="Timeline desk surface">
                <button class={timelineDeskSurface() === "show" ? "active" : ""} onClick={() => setTimelineDeskSurface("show")}>Show</button>
                <button class={timelineDeskSurface() === "cues" ? "active" : ""} onClick={() => setTimelineDeskSurface("cues")}>Cues</button>
                <button class={timelineDeskSurface() === "automation" ? "active" : ""} onClick={() => setTimelineDeskSurface("automation")}>Automation</button>
                <button class={timelineDeskSurface() === "playback" ? "active" : ""} onClick={() => setTimelineDeskSurface("playback")}>Playback</button>
              </nav>
            </Show>
            <Show when={controlMode() === "edit"}>
              <nav class="editDeskTabs" aria-label="Live edit desk surface">
                <button class={editDeskSurface() === "attributes" ? "active" : ""} onClick={() => setEditDeskSurface("attributes")}>Attributes</button>
                <button class={editDeskSurface() === "effects" ? "active" : ""} onClick={() => setEditDeskSurface("effects")}>Effects</button>
                <button class={editDeskSurface() === "dmx" ? "active" : ""} onClick={() => setEditDeskSurface("dmx")}>DMX</button>
              </nav>
            </Show>
            <Show when={controlMode() === "mixer"}>
              <span>{selectedFixtureGroupFilter() ? `Group ${selectedFixtureGroupFilter()}` : selectedFixture()?.label}</span>
            </Show>
          </div>
          <FaderFixtureControlPanel
            selectedGroupId={selectedFixtureGroupFilter()}
            selectedGroupFixtureCount={filteredFixtures().length}
            patchedFixtureCount={snapshot().fixtures.length}
            selectedGroupSubmasterLevel={selectedGroupSubmaster()?.level ?? 1}
            selectedFixture={selectedFixture() ?? null}
            groupFlagState={selectedGroupFlagState()}
            globalAnyFlagged={globalFixtureFlagState().anyFlagged}
            onSetGroupSubmaster={setGroupSubmaster}
            onSavePreset={savePreset}
            onLoadPreset={loadPreset}
            onLoadPresetForGroup={loadPresetForSelectedGroup}
            onLoadPresetForAllMatching={loadPresetForAllMatching}
            onDuplicateFixture={duplicateFixture}
            onRemoveFixture={removeFixture}
            onSetFixtureHighlight={setFixtureHighlight}
            onSetFixtureSolo={setFixtureSolo}
            onSetFixturePark={setFixturePark}
            onSetGroupHighlight={setGroupHighlight}
            onSetGroupSolo={setGroupSolo}
            onSetGroupPark={setGroupPark}
            onClearFixtureFlags={() => clearFixtureFlags("all")}
            onSetFixtureTransform={setFixtureTransform}
          />
          <div class="playbackDeskSurface">
            <ProgrammerPanel
              programmer={snapshot().programmer}
              onSetMode={setProgrammerMode}
              onCommit={commitProgrammer}
              onClear={clearProgrammer}
            />
            <ReferencePalettePanel
              palettes={snapshot().palettes}
              captureFixtureLabel={selectedFixture()?.label ?? null}
              targetLabel={controlTargetLabel()}
              targetFixtureCount={selectedControlTargetFixtures().length}
              onCreate={createReferencePalette}
              onUpdate={updateReferencePalette}
              onApply={applyReferencePalette}
              onRemove={removeReferencePalette}
            />
            <PlaybackExecutorPanel
              executors={snapshot().playback_executors}
              cueLists={snapshot().cue_lists}
              cues={snapshot().cues}
              playbackMaster={snapshot().playback_master}
              onCreate={createPlaybackExecutor}
              onUpdate={updatePlaybackExecutor}
              onRemove={removePlaybackExecutor}
              onSetLevel={setPlaybackExecutorLevel}
              onSetMaster={setPlaybackMaster}
              onTrigger={triggerPlaybackExecutor}
            />
          </div>
          <FaderAttributeEditorPanel
            categories={controlCategoryRows()}
            activeCategory={activeControlCategory()}
            targetKind={controlTargetKind()}
            targetLabel={controlTargetLabel()}
            targetDetail={controlTargetDetail()}
            referenceLabel={controlReferenceLabel()}
            onCategory={setControlCategory}
          >
          <FaderPrimaryAttributePanels
            dimmerControl={showDimmerPanel() ? selectedDimmerControl() : undefined}
            positionControls={showPositionPad() ? selectedPositionControls() : undefined}
            colorControls={showColorPad() ? selectedColorControls() : undefined}
            dimmerSliderMin={dimmerSliderRange().min}
            dimmerSliderMax={dimmerSliderRange().max}
            canEditLimits={Boolean(selectedFixture())}
            limitsDraft={selectedFixtureLimitsDraft()}
            normalizedLimits={normalizedSelectedFixtureLimitsDraft()}
            applyLimitsLabel={selectedFixture() ? "Apply to Fixture" : "No Fixture"}
            nudgeAmount={panTiltNudgeAmount()}
            nudgeSteps={panTiltNudgeSteps}
            targetPoints={panTiltTargetPoints}
            limitOverlayStyle={selectedFixtureLimitOverlayStyle()}
            limitWindowStyle={selectedFixtureLimitWindowStyle()}
            movementLimitDragging={Boolean(movementLimitDrag())}
            positionFavorites={positionFavorites()}
            positionFavoriteLabel={positionFavoriteLabel()}
            colorHsv={selectedColorHsv()}
            colorSaturationRamp={colorSaturationRamp()}
            colorAutoWhite={colorAutoWhite()}
            colorQuickLooks={colorQuickLooks}
            colorPalette={defaultColorPalette}
            colorFavorites={colorFavorites()}
            targetLabel={controlTargetLabel()}
            formatDmxPercent={formatDmxPercent}
            formatShortDmxPercent={formatShortDmxPercent}
            dmxValueToPercent={dmxValueToPercent}
            clampDmxValue={clampDmxValue}
            colorPreviewForSaturation={colorPreviewForSaturation}
            onSetDimmerValue={setDimmerValue}
            onUpdateDimmerLimit={(field, value) => updateSelectedFixtureLimit(field, value)}
            onResetDimmerLimits={resetSelectedDimmerLimits}
            onApplyLimits={applySelectedFixtureLimits}
            onNudgePanTilt={nudgePanTilt}
            onCenterPanTilt={centerPanTilt}
            onMirrorPanTiltAxis={mirrorPanTiltAxis}
            onSetPanTiltTarget={setPanTiltTargetPoint}
            onPanTiltPointerPad={(event) => void setPanTiltFromPointer(event)}
            onPanTiltPadKeyDown={handlePanTiltPadKeyDown}
            onSetPanTiltValues={(panValue, tiltValue) => void setPanTiltValues(panValue, tiltValue)}
            onSetPanTiltPercent={setPanTiltPercent}
            onSetPanTiltNudgeAmount={setPanTiltNudgeAmount}
            onMovementLimitPointerDown={startMovementLimitDrag}
            onMovementLimitPointerMove={dragMovementLimit}
            onMovementLimitPointerEnd={endMovementLimitDrag}
            onUpdateMovementLimit={(field, value) => updateSelectedFixtureLimit(field, value)}
            onUpdateMovementLimitToggle={(field, value) => updateSelectedFixtureLimit(field, value)}
            onResetMovementLimits={resetSelectedMovementLimits}
            onSetPositionFavoriteLabel={setPositionFavoriteLabel}
            onAddPositionFavorite={addCurrentPositionFavorite}
            onResetPositionFavorites={resetPositionFavorites}
            onRemovePositionFavorite={removePositionFavorite}
            onApplyPositionFavorite={(pan, tilt) => void setPanTiltValues(pan, tilt)}
            onPointerColor={setColorFromPointer}
            onSetColor={(color) => void setFixtureColor(color)}
            onSetColorAutoWhite={setColorAutoWhite}
            onSetColorChannel={setColorChannelValue}
            onSetColorExtraChannel={setColorExtraChannelValue}
            onSetColorHsv={setColorHsvValue}
            onAddColorFavorite={addCurrentColorFavorite}
            onResetColorFavorites={resetColorFavorites}
            onRemoveColorFavorite={removeColorFavorite}
          />
          <FaderAuxiliaryAttributePanels
            showColorWheel={showColorWheelPanel()}
            showGoboWheel={showGoboWheelPanel()}
            showOptics={showOpticsPanel()}
            showCategoryQuick={showCategoryQuickPanel()}
            targetLabel={controlTargetLabel()}
            categoryLabel={activeControlCategoryLabel()}
            attributeCount={visibleControls().length}
            colorWheelEntries={colorWheelEntries()}
            goboWheelEntries={goboWheelEntries()}
            wheelMediaUrlFor={wheelMediaUrlForCurrentFixture}
            wheelSlotMediaPath={wheelSlotMediaPath}
            opticsTitle={opticsPanelTitle()}
            opticsEntries={opticsEntries()}
            formatShortDmxPercent={formatShortDmxPercent}
            clampDmxValue={clampDmxValue}
            opticsPreviewClass={opticsPreviewClass}
            opticsPreviewStyle={opticsPreviewStyle}
            opticsPresetButtons={opticsPresetButtons}
            sortedFunctions={sortedChannelFunctions}
            functionContainsValue={channelFunctionContainsValue}
            functionBandStyle={channelFunctionBandStyle}
            functionLabel={channelFunctionLabel}
            functionRangeLabel={channelFunctionRangeLabel}
            functionDetail={channelFunctionDetail}
            functionSwatchColor={channelFunctionSwatchColor}
            quickLooks={categoryQuickLooks()}
            functionEntries={visibleFunctionControls()}
            currentValue={currentControlValue}
            onOpticsPointerValue={setOpticsValueFromPointer}
            onOpticsKeyValue={setOpticsValueFromKey}
            onSetValue={setControlAttributeValue}
            onApplyFunction={(control, fn) => void applyChannelFunction(control, fn)}
            onSetValueMode={(mode) => void applyVisibleControlValues(mode)}
            onApplyLook={(look) => void applyCategoryQuickLook(look)}
          />
          <FaderGridPanel
            controls={visibleControls()}
            selectedFixtureId={selectedFixture()?.id ?? null}
            selectedGroupId={selectedFixtureGroupFilter()}
            valueForControl={(control) => {
              const fixture = selectedControlReferenceFixture();
              return fixture ? faderValue(fixture.id, control.attribute, control.default_value) : control.default_value;
            }}
            onSetFixtureAttribute={setAttribute}
            onSetGroupAttribute={setGroupAttribute}
          />
          </FaderAttributeEditorPanel>
          <CueManagementPanel
            mode={controlMode() === "live" ? "live" : "edit"}
            cues={selectedCueListCues()}
            allCues={snapshot().cues}
            cueLists={snapshot().cue_lists}
            palettes={snapshot().palettes}
            selectedCueListId={selectedCueList().id}
            cueListLabel={cueListLabel()}
            activeCueId={snapshot().active_cue_id}
            activeFade={snapshot().active_fade}
            timelinePositionMs={snapshot().timeline.position_ms}
            timelineTrack={timelineTrack()}
            cueLabel={cueLabel()}
            cueFadeMs={cueFadeMs()}
            cueCaptureScope={cueCaptureScope()}
            cueCaptureScopeError={cueCaptureScopeError()}
            hasCueSources={hasCueSources()}
            cueCapturePreview={cueCapturePreview()}
            stageViewBoxSize={stageViewBoxSize}
            stageOrigin={stageOrigin2d()}
            selectedFixtureId={selectedFixtureId()}
            timelinePlacementNudgeMs={timelinePlacementNudgeMs()}
            cueMetadataDraft={cueMetadataDraft}
            cueTimelinePlacementsForCue={cueTimelinePlacementsForCue}
            onCueLabel={setCueLabel}
            onCueFadeMs={setCueFadeMs}
            onCueCaptureScope={setCueCaptureScope}
            onSelectCueList={setSelectedCueListId}
            onCueListLabel={setCueListLabel}
            onCreateCueList={createCueList}
            onRenameCueList={renameCueList}
            onRemoveCueList={removeCueList}
            onSetCueList={setCueList}
            onSetCuePalette={setCuePalette}
            onTriggerCueList={triggerCueList}
            onCreateCue={createCue}
            onSelectFixture={setSelectedFixtureId}
            onTriggerPreviousCue={() => triggerCueList(selectedCueList().id, "previous")}
            onTriggerNextCue={() => triggerCueList(selectedCueList().id, "next")}
            onSetCueFadePaused={setCueFadePaused}
            onUpdateCueMetadataDraft={updateCueMetadataDraft}
            onMoveCue={moveCue}
            onSetCueMetadata={setCueMetadata}
            onDuplicateCue={duplicateCue}
            onUpdateCue={updateCue}
            onTriggerCue={triggerCue}
            onAddTimelineCueEventAt={addTimelineCueEventAt}
            onRemoveCue={removeCue}
            onSeekTimeline={seekTimeline}
            onMoveTimelineCueEvent={moveTimelineCueEvent}
            onRemoveTimelineEvent={removeTimelineEvent}
          />
          <div class="timelinePanel">
            <div class="timelineShowSurface">
            <TimelineCueEventsPanel
              positionMs={snapshot().timeline.position_ms}
              durationMs={snapshot().timeline.duration_ms}
              playing={snapshot().timeline.playing}
              cuesCount={snapshot().cues.length}
              lightingAutomationCount={snapshot().timeline.automations.length}
              videoAutomationCount={snapshot().timeline.video_automations.length}
              overviewEvents={timelineOverviewEvents()}
              overviewAutomationRanges={timelineOverviewAutomationRanges()}
              selectedAutomationRangeId={selectedTimelineAutomationRangeId()}
              overviewPlayheadX={timelineOverviewPlayheadX()}
              audioAnalysis={audioAnalysis()}
              audioWaveformPoints={audioWaveformPoints()}
              audioSpectrumPaths={audioSpectrumPaths()}
              audioBeatMarkers={audioBeatMarkers()}
              snapMode={timelineSnapMode()}
              gridMs={timelineGridMs()}
              selectedCueId={selectedTimelineCueId()}
              eventTimeMs={timelineEventTimeMs()}
              track={timelineTrack()}
              cueOptions={timelineCueOptions()}
              eventRows={timelineEventRows()}
              timelineEventDraft={timelineEventDraft}
              onSeek={seekTimeline}
              onPause={pauseTimeline}
              onPlay={playTimeline}
              onSeekRatio={seekTimelineFromOverviewRatio}
              onMoveEventRatio={moveTimelineCueEventToRatio}
              onSelectAutomationRange={selectTimelineAutomationRange}
              onMoveAutomationRangeRatio={moveTimelineAutomationRangeToRatio}
              onResizeAutomationRangeRatio={resizeTimelineAutomationRangeToRatio}
              onMoveAutomationKeyframeRatio={moveTimelineAutomationKeyframeToRatio}
              onAnalyzeAudio={analyzeAudioFile}
              onClearAudio={clearTimelineAudio}
              onApplyAudioBpm={applyAudioBpm}
              onSnapMode={setTimelineSnapMode}
              onGridMs={setTimelineGridMs}
              onSnapDrafts={snapTimelineDrafts}
              onSnapItems={snapTimelineItems}
              onSelectedCueId={setTimelineCueId}
              onEventTimeMs={setTimelineEventTimeMs}
              onTrack={setTimelineTrack}
              onAddEvent={addTimelineCueEvent}
              onAddEventAtPlayhead={addTimelineCueEventAtPlayhead}
              onUpdateEventDraft={updateTimelineEventDraft}
              onSaveEvent={setTimelineCueEvent}
              onRemoveEvent={removeTimelineEvent}
            />
            </div>
            <div class="timelineAutomationSurface">
            <TimelineLightingAutomationPanel
              activeControls={timelineAutomationControls()}
              selectedAttribute={selectedTimelineAutomationAttribute()}
              canAddAutomation={canAddTimelineAutomation()}
              selectedGroupId={selectedFixtureGroupFilter()}
              canAddGroupAutomation={canAddTimelineGroupAutomation()}
              rowScope={lightingAutomationRowScope()}
              allRowsCount={allTimelineAutomationRows().length}
              startMs={automationStartMs()}
              endMs={automationEndMs()}
              startValue={automationStartValue()}
              endValue={automationEndValue()}
              interpolation={automationInterpolation()}
              rows={timelineAutomationRows()}
              selectedAutomationId={selectedLightingTimelineAutomationId()}
              fixtureOptions={snapshot().fixtures.map((fixture) => ({ id: fixture.id, label: fixture.label }))}
              timelineAutomationDraft={timelineAutomationDraft}
              fixtureAttributeOptions={fixtureAttributeOptions}
              onAttribute={setEffectAttribute}
              onStartMs={setAutomationStartMs}
              onEndMs={setAutomationEndMs}
              onStartValue={setAutomationStartValue}
              onEndValue={setAutomationEndValue}
              onInterpolation={setAutomationInterpolation}
              onRowScope={setLightingAutomationRowScope}
              onUsePlayheadRange={usePlayheadForLightingAutomation}
              onAddAutomation={addTimelineAutomation}
              onAddGroupAutomation={addTimelineGroupAutomation}
              onUpdateDraft={updateTimelineAutomationDraft}
              onAlignDraftToPlayhead={alignLightingAutomationDraftToPlayhead}
              onAddKeyframeAtPlayhead={addLightingAutomationKeyframeAtPlayhead}
              onRemoveKeyframeAtPlayhead={removeLightingAutomationKeyframeAtPlayhead}
              onRemoveKeyframe={removeLightingAutomationKeyframe}
              onSetKeyframeInterpolation={setLightingAutomationKeyframeInterpolation}
              onSetKeyframeValue={setLightingAutomationKeyframeValue}
              onSetAutomationEnabled={(automation, enabled) => setTimelineAutomationEnabled(automation.id, enabled)}
              onSetRowsEnabled={setLightingAutomationRowsEnabled}
              onSeekKeyframe={seekTimeline}
              onSaveAutomation={setTimelineAutomation}
              onRemoveAutomation={removeTimelineAutomation}
            />
            </div>
          </div>
          <div class="effectEditor">
            <div class="panelHeader">
              <h2>Effects</h2>
              <div class="panelHeaderActions">
                <span>{snapshot().effects.length}</span>
                <button onClick={loadEffectPreset}>Load</button>
                <button onClick={loadEffectPresetForCurrentTarget} disabled={Boolean(effectTargetOverrideError())}>
                  Load Target
                </button>
              </div>
            </div>
            <SampleEffectPresetPanel
              selectedPreset={sampleEffectPreset()}
              targetErrorForPreset={sampleEffectTargetOverrideError}
              onSelectPreset={setSampleEffectPreset}
              onLoadPreset={(preset) => loadSampleEffectPreset(preset)}
              onLoadPresetForTarget={(preset) => loadSampleEffectPreset(preset, true)}
            />
            <div class="effectForm">
              <div class="effectTargetHint">
                <strong>{effectTargetMode() === "selection" ? "Map selection" : effectTargetMode()}</strong>
                <span title={`${effectTargetSummary()} / ${effectDraftSummary()}`}>
                  <span data-no-localize>{effectTargetSummary()}</span> / {effectDraftSummary()}
                </span>
              </div>
              <Show when={effectTargetMode() !== "video"}>
                <label>
                  Attribute
                  <select
                    value={selectedEffectAttribute()}
                    disabled={effectTargetControls().length === 0}
                    onInput={(event) => setEffectAttribute(event.currentTarget.value)}
                  >
                    <For each={effectTargetControls()}>
                      {(control) => <option value={control.attribute}>{control.attribute}</option>}
                    </For>
                  </select>
                </label>
              </Show>
              <div class="split">
                <label>
                  Target
                  <select
                    value={effectTargetMode()}
                    onInput={(event) => {
                      const nextMode = event.currentTarget.value as EffectTargetMode;
                      setEffectTargetMode(nextMode);
                      if (nextMode === "video") {
                        setEffectVideoTargetLinked(false);
                      }
                    }}
                  >
                    <option value="fixture">Selected fixture</option>
                    <option value="selection">Map selection ({selectedMappingFixtures().length})</option>
                    <option value="group">Group</option>
                    <option value="video">Video layer</option>
                  </select>
                </label>
                <label>
                  Type
                  <select value={effectType()} onInput={(event) => setEffectType(event.currentTarget.value as EffectKind)}>
                    <option value="Lfo">LFO</option>
                    <option value="PositionWave">Position Wave</option>
                  </select>
                </label>
              </div>
              <Show when={effectTargetMode() !== "video"}>
                <label class="checkbox inlineCheckbox effectLinkedVideoToggle">
                  <input
                    type="checkbox"
                    checked={effectVideoTargetLinked()}
                    disabled={snapshot().video.layers.length === 0}
                    onChange={(event) => setEffectVideoTargetLinked(event.currentTarget.checked)}
                  />
                  Link video layer
                </label>
              </Show>
              <Show when={effectTargetMode() === "group"}>
                <EffectGroupTargetPanel
                  value={effectTargetGroups()}
                  groups={fixtureGroupRows()}
                  activeGroupIds={parseGroupIds(effectTargetGroups())}
                  onValue={setEffectTargetGroups}
                  onToggleGroup={toggleEffectTargetGroup}
                />
              </Show>
              <Show when={effectTargetMode() === "video" || effectVideoTargetLinked()}>
                <VideoEffectTargetPanel
                  layers={snapshot().video.layers}
                  outputsCount={snapshot().video.outputs.length}
                  selectedLayerId={selectedEffectVideoLayerId()}
                  param={effectVideoParam()}
                  low={effectVideoLow()}
                  high={effectVideoHigh()}
                  effectType={effectType()}
                  positionX={effectVideoPositionX()}
                  positionY={effectVideoPositionY()}
                  positionZ={effectVideoPositionZ()}
                  hasSelectedStageObject={Boolean(selectedStageObject())}
                  onSetLayerId={setEffectVideoLayerId}
                  onSetParam={setEffectVideoParam}
                  onSetLow={setEffectVideoLow}
                  onSetHigh={setEffectVideoHigh}
                  onUseSelectedOutput={setEffectVideoPositionFromSelectedOutput}
                  onUseWaveOrigin={setEffectVideoPositionFromWaveOrigin}
                  onUseStageCenter={setEffectVideoPositionFromStageCenter}
                  onUseSelectedStageObject={setEffectVideoPositionFromSelectedStageObject}
                  onSetPositionX={setEffectVideoPositionX}
                  onSetPositionY={setEffectVideoPositionY}
                  onSetPositionZ={setEffectVideoPositionZ}
                />
              </Show>
              <EffectSourceControlsPanel
                effectType={effectType()}
                shape={effectShape()}
                periodMs={effectPeriod()}
                bpm={snapshot().clock.bpm}
                clockSyncBeats={effectClockSyncBeats()}
                stageViewBoxSize={stageViewBoxSize}
                originX={waveOriginX()}
                originY={waveOriginY()}
                originZ={waveOriginZ()}
                directionX={waveDirectionX()}
                directionY={waveDirectionY()}
                directionZ={waveDirectionZ()}
                speed={waveSpeed()}
                wavelength={waveWavelength()}
                stageOrigin={stageOrigin2d()}
                stageBounds={stageWorldBounds()}
                originPoint={waveOriginSvgPoint()}
                directionPoint={waveDirectionSvgPoint()}
                videoTargetPoint={effectVideoTargetSvgPoint()}
                directionIsRadial={waveDirectionIsRadial()}
                radialRadius={waveRadialRadius()}
                fixtures={visualizerFixtures()}
                videoSurfaces={visualizerVideoSurfaces2d()}
                stageObjects={visualizerStageObjects2d()}
                targetFixtureIds={waveTargetFixtureIds()}
                selectedFixtureId={selectedFixtureId()}
                selectedVideoOutputId={selectedVideoOutputId()}
                videoTargetMode={effectTargetMode() === "video" || effectVideoTargetLinked()}
                dragging={waveStageDrag()}
                hasSelectedFixture={Boolean(selectedFixture())}
                hasSelectedStageObject={Boolean(selectedStageObject())}
                onShape={setEffectShape}
                onPeriodMs={setEffectPeriod}
                onClockSyncBeats={setEffectClockSyncPreset}
                onUseStageCenter={setWaveOriginFromStageCenter}
                onUseSelectedFixture={setWaveOriginFromSelectedFixture}
                onUseSelectedStageObject={setWaveOriginFromSelectedStageObject}
                onDirectionPreset={setWaveDirectionPreset}
                onStagePointerDown={startWaveStageDrag}
                onStagePointerMove={moveWaveStageDrag}
                onStagePointerUp={endWaveStageDrag}
                onPickVideoSurface={(surfaceId) => {
                  const output = snapshot().video.outputs.find((candidate) => candidate.id === surfaceId);
                  if (output) {
                    setEffectVideoPositionFromVideoOutput(output);
                  }
                }}
                onOriginX={setWaveOriginX}
                onOriginY={setWaveOriginY}
                onOriginZ={setWaveOriginZ}
                onDirectionX={setWaveDirectionX}
                onDirectionY={setWaveDirectionY}
                onDirectionZ={setWaveDirectionZ}
                onSpeed={setWaveSpeed}
                onWavelength={setWaveWavelength}
              />
              <EffectActionControlsPanel
                showLightRange={effectTargetMode() !== "video"}
                low={effectLow()}
                high={effectHigh()}
                phase={effectPhase()}
                blendMode={effectBlendMode()}
                addDisabled={effectSubmitDisabled()}
                submitLabel={editingEffectId() === null ? "Add Effect" : `Update Effect ${editingEffectId()}`}
                editing={editingEffectId() !== null}
                editingLabel={editingEffectSummary()?.label ?? null}
                onLow={setEffectLow}
                onHigh={setEffectHigh}
                onPhase={setEffectPhase}
                onBlendMode={setEffectBlendMode}
                onSubmitEffect={updateEditingEffect}
                onCancelEdit={cancelEffectEdit}
              />
            </div>
            <NodeGraphEditorPanel
              graphCount={snapshot().node_graphs.length}
              label={nodeGraphLabel()}
              transformOp={nodeGraphTransformOp()}
              transformAmount={nodeGraphTransformAmount()}
              transformMin={nodeGraphTransformMin()}
              transformMax={nodeGraphTransformMax()}
              effectType={effectType()}
              sourceMode={nodeGraphSourceMode()}
              audioBand={nodeGraphAudioBand()}
              audioSource={nodeGraphAudioSource()}
              audioGain={nodeGraphAudioGain()}
              audioBias={nodeGraphAudioBias()}
              sourceLabel={nodeGraphSourceLabel()}
              sourceDetail={nodeGraphSourceDetail()}
              transformLabel={nodeGraphTransformLabel()}
              targetMode={effectTargetMode()}
              canSave={!effectTargetOverrideError()}
              graphs={snapshot().node_graphs}
              targetLabel={nodeGraphTargetLabel}
              onLoadPreset={loadNodeGraphPreset}
              onLabel={setNodeGraphLabel}
              onSourceMode={setNodeGraphSourceMode}
              onAudioBand={setNodeGraphAudioBand}
              onAudioSource={setNodeGraphAudioSource}
              onAudioGain={setNodeGraphAudioGain}
              onAudioBias={setNodeGraphAudioBias}
              onTransformOp={setNodeGraphTransformOp}
              onTransformAmount={setNodeGraphTransformAmount}
              onTransformMin={setNodeGraphTransformMin}
              onTransformMax={setNodeGraphTransformMax}
              onSaveGraph={saveNodeGraphFromForm}
              onResetTransform={() => {
                setNodeGraphTransformOp("Scale");
                setNodeGraphTransformAmount(1);
                setNodeGraphTransformMin(0);
                setNodeGraphTransformMax(1);
              }}
              onSetGraphEnabled={setNodeGraphEnabled}
              onSaveGraphPreset={saveNodeGraphPreset}
              onRemoveGraph={removeNodeGraph}
            />
            <EffectListPanel
              effects={snapshot().effects}
              onMoveEffect={moveEffect}
              onSetEnabled={setEffectEnabled}
              onUseOutputPosition={setEffectVideoTargetsFromSelectedOutput}
              onDuplicateEffect={duplicateEffect}
              onUseAsDraft={useEffectAsDraft}
              onSavePreset={saveEffectPreset}
              onRemoveEffect={removeEffect}
            />
          </div>
          <DmxRawMonitor
            previews={dmxPreviewOptions()}
            activeUniverse={activeDmxPreviewUniverse()}
            activeCount={nonZeroDmxCount()}
            cells={dmxCells()}
            onUniverseChange={setRawDmxUniverse}
          />
        </section>
        </Show>

        <Show when={workspaceTab() === "control" || (workspaceTab() === "setup" && ["dmx", "midi", "osc", "remote"].includes(setupSubTab()))}>
        <aside
          class={setupPanelClass("panel output setupIoPanel setupPanel controlPanel", ["dmx", "midi", "osc", "remote"])}
          ref={registerSetupPanel(["dmx", "midi", "osc", "remote"])}
          tabIndex={-1}
        >
          <Show when={workspaceTab() === "control" || setupSubTab() === "dmx"}>
          <div class="dmxEndpointDesk">
          <DmxOutputConfigPanel
            output={output()}
            serialPorts={serialPorts()}
            isSerialProtocol={isSerialDmxProtocol}
            onOutputChange={(nextOutput) => setOutput(nextOutput)}
            onProtocolChange={setOutputProtocol}
            onRefreshSerialPorts={refreshSerialPorts}
            onApply={applyOutput}
          />
          <DmxInputPanel
            config={dmxInputConfig()}
            status={dmxInputStatus()}
            onConfig={setDmxInputConfig}
            onStart={startDmxInput}
            onStop={stopDmxInput}
          />
          <ArtRdmPanel
            gatewayIp={output().target_ip}
            portAddress={output().universe}
            serialPorts={serialPorts()}
            onRequest={sendArtRdmRequest}
            onUsbRequest={sendUsbRdmRequest}
            onUsbDiscover={discoverUsbRdmDevices}
            onRefreshSerialPorts={refreshSerialPorts}
            onDiscover={discoverArtRdmDevices}
            onStartFullDiscovery={startArtRdmFullDiscovery}
          />
          </div>
          <OutputDiagnosticsPanel
            protocolLabel={outputProtocolLabel(output().protocol)}
            testChannel={dmxTestChannel()}
            testWidth={dmxTestWidth()}
            testValue={dmxTestValue()}
            routes={dmxOutputRoutes()}
            telemetry={snapshot().telemetry}
            telemetryBudget={engineTelemetryReport()?.budget ?? null}
            phase1SmokeReport={phase1SmokeReport()}
            routeLabel={dmxRouteLabel}
            onTestChannel={setDmxTestChannel}
            onTestWidth={setDmxTestWidth}
            onTestValue={setDmxTestValue}
            onSendTest={sendDmxTestFrame}
            onSendRoutes={sendDmxRoutesTestFrame}
            onAddCurrentRoute={addCurrentDmxRoute}
            onApplyRoutes={applyCurrentDmxRoutes}
            onRemoveRoute={removeDmxRoute}
            onResetTelemetry={resetEngineTelemetry}
            onSaveTelemetryReport={saveEngineTelemetryReport}
          />
          <LightingRuntimeControlsPanel
            lightingMaster={snapshot().lighting_master}
            submasters={snapshot().submasters}
            clock={snapshot().clock}
            bpmDraft={bpmDraft()}
            midiInputs={midiInputs()}
            selectedMidiInput={selectedMidiInput()}
            midiConnected={midiConnected()}
            onLightingMaster={setLightingMaster}
            onSubmaster={setGroupSubmaster}
            onBlackout={setBlackout}
            onAllBlackout={setAllBlackout}
            onBpmDraft={setBpmDraft}
            onApplyBpm={applyBpm}
            onTapBpm={tapBpm}
            onRefreshMidi={() => {
              void refreshMidiInputs();
              void refreshMidiOutputs();
            }}
            onDisconnectMidiClock={disconnectMidiClock}
            onSelectedMidiInput={setSelectedMidiInput}
            onConnectMidiClock={connectMidiClock}
          />
          </Show>
          <Show when={workspaceTab() === "control" || setupSubTab() === "midi"}>
          <MidiControlMappingPanel
            snapshot={snapshot()}
            midiOutputs={midiOutputs()}
            selectedMidiOutput={selectedMidiOutput()}
            feedbackConnected={midiFeedbackConnected()}
            feedbackEnabled={midiFeedbackEnabled()}
            midiInputsCount={midiInputs().length}
            controlConnected={midiControlConnected()}
            mappings={midiMappings()}
            mapMessage={midiMapMessage()}
            mapNumber={midiMapNumber()}
            mapChannel={midiMapChannel()}
            mapAction={midiMapAction()}
            mapAttribute={midiMapAttribute()}
            clearFixtureFlagKind={midiClearFixtureFlagKind()}
            selectedCueId={selectedMidiCueId()}
            selectedEffectId={selectedMidiEffectId()}
            selectedNodeGraphId={selectedMidiNodeGraphId()}
            mapGroupId={midiMapGroupId()}
            selectedLayerId={selectedMidiLayerId()}
            selectedOutputId={selectedMidiVideoOutputId()}
            selectedMappingField={selectedMidiVideoOutputMappingField()}
            selectedMappingPresetLabel={selectedMidiVideoOutputMappingPresetLabel()}
            mapVideoParam={midiMapVideoParam()}
            mapCuePointIndex={midiMapCuePointIndex()}
            mapDurationMs={midiMapDurationMs()}
            mapLow={midiMapLow()}
            mapHigh={midiMapHigh()}
            mappingTargetLabel={controlMappingTargetLabel}
            onSelectedMidiOutput={setSelectedMidiOutput}
            onConnectFeedback={connectMidiFeedback}
            onDisconnectFeedback={disconnectMidiFeedback}
            onSendFeedback={sendMidiFeedback}
            onFeedbackEnabled={setMidiFeedbackEnabled}
            onMapMessage={setMidiMapMessage}
            onMapNumber={setMidiMapNumber}
            onMapChannel={setMidiMapChannel}
            onMapAction={setMidiControlMappingAction}
            onMapAttribute={setMidiMapAttribute}
            onMapCueId={setMidiMapCueId}
            onMapEffectId={setMidiMapEffectId}
            onMapNodeGraphId={setMidiMapNodeGraphId}
            onMapGroupId={setMidiMapGroupId}
            onMapLayerId={setMidiMapLayerId}
            onMapOutputId={setMidiMapVideoOutputId}
            onMappingField={setMidiVideoOutputMappingFieldTarget}
            onMappingPresetLabel={setMidiMapVideoOutputMappingPresetLabel}
            onMapVideoParam={setMidiMapVideoParam}
            onMapCuePointIndex={setMidiMapCuePointIndex}
            onMapDurationMs={setMidiMapDurationMs}
            onMapLow={setMidiMapLow}
            onMapHigh={setMidiMapHigh}
            onLearn={learnMidiControl}
            onAddMapping={addMidiMapping}
            onConnectControl={connectMidiControl}
            onDisconnectControl={disconnectMidiControl}
            onLoadMappings={loadMidiMappings}
            onSaveMappings={saveMidiMappings}
            onRemoveMapping={removeMidiMapping}
          />
          </Show>
          <Show when={workspaceTab() === "control" || setupSubTab() === "osc"}>
          <OscControlMappingPanel
            snapshot={snapshot()}
            bindIp={oscBindIp()}
            port={oscPort()}
            running={oscRunning()}
            mappings={oscMappings()}
            mapAddress={oscMapAddress()}
            mapAction={oscMapAction()}
            mapAttribute={oscMapAttribute()}
            clearFixtureFlagKind={oscClearFixtureFlagKind()}
            selectedCueId={selectedOscCueId()}
            selectedEffectId={selectedOscEffectId()}
            selectedNodeGraphId={selectedOscNodeGraphId()}
            mapGroupId={oscMapGroupId()}
            selectedLayerId={selectedOscLayerId()}
            selectedOutputId={selectedOscVideoOutputId()}
            selectedMappingField={selectedOscVideoOutputMappingField()}
            selectedMappingPresetLabel={selectedOscVideoOutputMappingPresetLabel()}
            mapVideoParam={oscMapVideoParam()}
            mapCuePointIndex={oscMapCuePointIndex()}
            mapDurationMs={oscMapDurationMs()}
            mapLow={oscMapLow()}
            mapHigh={oscMapHigh()}
            mappingTargetLabel={controlMappingTargetLabel}
            onBindIp={setOscBindIp}
            onPort={setOscPort}
            onStart={startOscInput}
            onStop={stopOscInput}
            onMapAddress={setOscMapAddress}
            onMapAction={setOscControlMappingAction}
            onMapAttribute={setOscMapAttribute}
            onMapCueId={setOscMapCueId}
            onMapEffectId={setOscMapEffectId}
            onMapNodeGraphId={setOscMapNodeGraphId}
            onMapGroupId={setOscMapGroupId}
            onMapLayerId={setOscMapLayerId}
            onMapOutputId={setOscMapVideoOutputId}
            onMappingField={setOscVideoOutputMappingFieldTarget}
            onMappingPresetLabel={setOscMapVideoOutputMappingPresetLabel}
            onMapVideoParam={setOscMapVideoParam}
            onMapCuePointIndex={setOscMapCuePointIndex}
            onMapDurationMs={setOscMapDurationMs}
            onMapLow={setOscMapLow}
            onMapHigh={setOscMapHigh}
            onLearn={learnOscControl}
            onAddMapping={addOscMapping}
            onLoadMappings={loadOscMappings}
            onSaveMappings={saveOscMappings}
            onRemoveMapping={removeOscMapping}
          />
          </Show>
          <Show when={workspaceTab() === "control" || setupSubTab() === "remote"}>
          <RemoteControlPanel
            bindIp={remoteBindIp()}
            port={remotePort()}
            pairingPin={remotePairingPin()}
            allowLan={remoteAllowLan()}
            maxConnections={remoteMaxConnections()}
            maxMessageBytes={remoteMaxMessageBytes()}
            maxMessagesPerSecond={remoteMaxMessagesPerSecond()}
            running={remoteRunning()}
            remoteUrls={remoteUrls()}
            status={remoteStatus()}
            onBindIp={setRemoteBindIp}
            onPort={setRemotePort}
            onPairingPin={setRemotePairingPin}
            onRegeneratePairingPin={() => setRemotePairingPin(createPairingPin())}
            onAllowLan={(value) => {
              setRemoteAllowLan(value);
              setRemoteBindIp(value ? "0.0.0.0" : "127.0.0.1");
            }}
            onMaxConnections={setRemoteMaxConnections}
            onMaxMessageBytes={setRemoteMaxMessageBytes}
            onMaxMessagesPerSecond={setRemoteMaxMessagesPerSecond}
            onCopyRemoteUrl={copyRemoteUrl}
            onOpenRemoteUrl={openRemoteUrl}
            onStart={startRemoteControl}
            onStop={stopRemoteControl}
            onDisconnectClient={disconnectRemoteClient}
          />
          </Show>
        </aside>
        </Show>
      </section>

      <AppStatusLine status={appStatus()} />
    </main>
  );
}
