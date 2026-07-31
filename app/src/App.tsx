import { invoke as tauriInvoke } from "@tauri-apps/api/core";
import { listen as tauriListen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { createEffect, createMemo, createSignal, For, onCleanup, Show } from "solid-js";
import { CueManagementPanel } from "./components/CueManagementPanel";
import { SceneMatrixPanel } from "./components/SceneMatrixPanel";
import {
  SceneSettingsPane,
  type SceneEffectEditorModel,
  type SceneSettingsSurface,
} from "./components/SceneSettingsPane";
import { AppStatusLine } from "./components/AppStatusLine";
import { ArtRdmPanel } from "./components/ArtRdmPanel";
import { defaultColorEffectStops } from "./components/ColorEffectEditorPanel";
import {
  defaultColorMappingRaster,
} from "./components/ColorMappingEffectEditorPanel";
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
import { DvcImportReportPanel } from "./components/DvcImportReportPanel";
import { ControlFaderWriteHeader } from "./components/ControlFaderWriteHeader";
import { FaderAttributeEditorPanel } from "./components/FaderAttributeEditorPanel";
import { FaderAuxiliaryAttributePanels } from "./components/FaderAuxiliaryAttributePanels";
import { FaderFixtureControlPanel } from "./components/FaderFixtureControlPanel";
import { FaderGridPanel } from "./components/FaderGridPanel";
import { ChannelFunctionPanel } from "./components/ChannelFunctionPanel";
import { FixtureTypeAttributeColumns } from "./components/FixtureTypeAttributeColumns";
import { FixtureCatalogPanel } from "./components/FixtureCatalogPanel";
import { GroupLiveMixerStrip } from "./components/GroupLiveMixerStrip";
import { FaderPrimaryAttributePanels } from "./components/FaderPrimaryAttributePanels";
import { LightingRuntimeControlsPanel } from "./components/LightingRuntimeControlsPanel";
import { LoadedProfileSummaryPanel } from "./components/LoadedProfileSummaryPanel";
import { MidiControlMappingPanel } from "./components/MidiControlMappingPanel";
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
import {
  type EffectChooserFamily,
  type EffectRecipeFamily,
} from "./components/EffectFamilyChooser";
import { SetupMappingWorkspace } from "./components/SetupMappingWorkspace";
import { ControlModeSegment, MappingPersistentWorkspaceBand } from "./components/MappingPersistentWorkspaceBand";
import { SetupVideoPanel } from "./components/SetupVideoPanel";
import { StagePreview2D } from "./components/StagePreview2D";
import { VideoControlPanel } from "./components/VideoControlPanel";
import { defaultAutoVjSnapshot } from "./components/AutoVjStrip";
import { readVideoOutputTestPattern, readVideoOutputWindowId, VideoOutputWindow } from "./components/VideoOutputWindow";
import { TimelineCueEventsPanel } from "./components/TimelineCueEventsPanel";
import { TimelineLightingAutomationPanel } from "./components/TimelineLightingAutomationPanel";
import { EditableTouchSurface } from "./components/EditableTouchSurface";
import { TouchColorPalettePanel } from "./components/TouchColorPalettePanel";
import { TouchCuePanel } from "./components/TouchCuePanel";
import { TouchDimmerControlPanel } from "./components/TouchDimmerControlPanel";
import { TouchFixturePanel } from "./components/TouchFixturePanel";
import { TouchGenericAttributeGrid } from "./components/TouchGenericAttributeGrid";
import { TouchPanTiltPad } from "./components/TouchPanTiltPad";
import { TouchSafetyDeck } from "./components/TouchSafetyDeck";
import { TouchVideoPanel } from "./components/TouchVideoPanel";
import type {
  TimelineOverviewAutomationRange,
  TimelineOverviewEvent,
  TimelineOverviewOverlapCluster,
} from "./components/TimelineOverview";
import {
  buildTimelineSceneBlockCueOptions,
  buildTimelineSceneBlockRows,
  buildTimelineSceneBlockSnapPlacements,
  createTimelineSceneBlockController,
  createTimelineSceneBlockHourViewportFixture,
  createTimelineSceneBlockLargeViewportFixture,
  createTimelineSceneBlockViewportFixture,
  timelinePlacementDisplayEndMs,
  reconcileTimelineEventDrafts,
  timelineSceneBlockCueOptionsEqual,
  timelineEventDraftMatchesSummary,
  timelineExecutionIsLive,
  timelineSceneBlockRowsEqual,
  timelineSceneBlockSpanMs,
} from "./timelineSceneBlocks";
import {
  createTimelineViewportState,
  fitTimelineVisibleWindow,
  normalizeTimelineVisibleWindow,
  panTimelineVisibleWindow,
  reconcileTimelineViewportState,
  revealTimelineVisibleRange,
  timelineClosedRangeIntersectsVisibleWindow,
  timelineRangeIntersectsVisibleWindow,
  timelineTimeToVisibleRawRatio,
  timelineVisibleRatioToTimeMs,
  timelineVisibleWindowSpanMs,
  zoomTimelineVisibleWindow,
} from "./timelineViewport";
import { buildTimelineOverlapClusters } from "./timelineOverlapClusters";
import { createTimelineLayerController } from "./createTimelineLayerController";
import {
  cueDropDurationMs,
  effectiveTimelineLayers,
  sameTimelineLayerSummaries,
  timelineLayerIdForEvent,
} from "./timelineLayers";
import {
  updateTimelineCueDrag,
  type TimelineCueDragPoint,
  type TimelineCueDragState,
} from "./timelineCueDrag";
import { WorkspaceChrome } from "./components/WorkspaceChrome";
import { TOPBAR_PULSE_STALE_MS } from "./components/TopbarPulseMeter";
import { WorkspaceOperationsMenu } from "./components/WorkspaceOperationsMenu";
import { OperatorLockOverlay } from "./components/OperatorLockOverlay";
import { WorkspaceSplitHandle } from "./components/WorkspaceSplitHandle";
import {
  defaultWorkspaceLayout,
  loadWorkspaceLayout,
  saveWorkspaceLayout,
  type WorkspaceLayout,
} from "./workspaceLayoutStorage";
import {
  loadNamedWorkspaces,
  paneWindowKinds,
  saveNamedWorkspaces,
  upsertNamedWorkspace,
  type NamedWorkspaceProfile,
  type PaneWindowKind,
  type PaneWindowPlacement,
} from "./workspaceProfiles";
import {
  createOperatorPolicy,
  operatorCommandAllowed,
  operatorPolicyFromUnknown,
  verifyOperatorPassword,
} from "./operatorPolicy";
import { type ColorWheelFunctionEntry, type GoboSlotPattern, type GoboWheelFunctionEntry } from "./components/WheelSlotPanel";
import { nearestColorWheelEntry } from "./colorWheelApproximation";
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
import { confirmCueRemoval, confirmDestructiveAction } from "./destructiveActions";
import { createLiveAudioInputStatusRequestGate } from "./liveAudioInputStatusSync";
import type {
  ApplicationUpdateCheck,
  ApplicationUpdateConfiguration,
  ApplicationUpdateProgress,
  AttributeControl,
  AttributeResolution,
  ArtRdmRequest,
  ArtRdmResponse,
  AudioAnalysisSummary,
  AutoVjConfig,
  AutomationKeyframeSummary,
  AutomationInterpolation,
  CompositionSummary,
  ChaserDirection,
  ChaserEffectRequest,
  ChaserFeature,
  ChaserStep,
  ColorEffectAlgorithm,
  ColorEffectInterpolation,
  ColorEffectRequest,
  ColorEffectSpatialPattern,
  ColorEffectStop,
  ColorMappingCellTarget,
  ColorMappingEffectRequest,
  ColorMappingFrame,
  ColorMappingPlaybackDirection,
  ColorMappingSampling,
  ColorMappingSourceKind,
  ColorMappingWrapMode,
  CurveEffectPoint,
  CurveEffectRequest,
  CustomFixtureProfileRequest,
  CueEffectTarget,
  CueListSummary,
  CueLiveDirection,
  CueLiveModifierSettings,
  CueStepSummary,
  CueSummary,
  PaletteKind,
  PlaybackExecutorSummary,
  DmxOutputConfig,
  DvcImportReport,
  DmxInputConfig,
  DmxInputStatus,
  EffectBlendMode,
  EffectKind,
  EffectParamsSnapshot,
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
  LiveAudioChannelMix,
  LiveAudioInputBackendId,
  LiveAudioInputBackendSummary,
  LiveAudioInputCapabilities,
  LiveAudioInputDeviceSummary,
  LiveAudioInputStartRequest,
  LiveAudioInputStatus,
  LfoEffectRequest,
  MappingEffectDirection,
  MappingEffectRequest,
  MidiControlAction,
  MidiControlMapping,
  MidiControlMessage,
  MidiInputSummary,
  MidiOutputSummary,
  MoveCoordinateMode,
  MoveDirection,
  MoveEffectRequest,
  MoveInterpolation,
  MovePathPoint,
  ValueEffectDirection,
  ValueEffectInterpolation,
  ValueEffectMode,
  ValueEffectPoint,
  ValueEffectRequest,
  OscControlAction,
  OscControlMapping,
  OscInputConfig,
  OperatorLockMode,
  OperatorPolicy,
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
  ChildTimelineSummary,
  TimelineCueEventSummary,
  TimelineAudioClipSummary,
  TimelineAutomationSummary,
  TimelineGroupAutomationAddResult,
  TimelineLayerSummary,
  TimelineTrackKind,
  TimelineSnapshot,
  TimelineVideoAutomationSummary,
  TouchControlBinding,
  TouchSurfaceSummary,
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
  VjPreviewTransportSummary,
  VisualizerRenderPayload,
} from "./types";
import { videoFrameToDataUrl } from "./videoFrameCanvas";
import {
  defaultMovePathPoints,
  moveEffectDraftError,
  movePathRecipes,
  movePathRecipePoints,
  type MovePathRecipe,
  type MovePathPreset,
} from "./moveEffect";
import { browserPoppedPanes, browserViewportFixture, paneWindowMode, viewportFixtureData, viewportPatchedFixture } from "./viewportFixtureData";
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
  isEditableContextMenuTarget,
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
  type TimelineContextDrawer,
  type TimelineDeskSurface,
} from "./uiModes";
import { projectSnapshotSignature } from "./projectSnapshot";
import { effectDraftTargetPlan } from "./effectDraft";
import {
  canonicalChaserAttribute,
  chaserDefaultSeed,
  chaserDraftError,
  chaserStepsFromTargets,
  clampChaserUnit,
  clampChaserWings,
} from "./chaserDraft";
import {
  currentCueEffectTargets,
  eligibleCueEffects,
  groupMatches,
  normalizedCueEffectTargets,
  syncCueEffectCaptureTargets,
} from "./cueEffectRecall";
import type { CueEffectRecallChange } from "./cueEffectRecall";
import { createSnapshotRequestGuard } from "./snapshotRequestGuard";
import { createMappingViewportModel } from "./createMappingViewportModel";
import { createMappingRenderModel } from "./createMappingRenderModel";
import { liveDmxPollIntervalMs } from "./fixtureLiveColor";
import { createMappingInteractionController } from "./createMappingInteractionController";
import { createMappingLayoutController } from "./createMappingLayoutController";
import {
  createOutputDiagnosticsController,
  defaultOutput,
  isSerialDmxProtocol,
  outputProtocolLabel,
} from "./createOutputDiagnosticsController";
import { createInitialEngineSnapshot } from "./initialEngineSnapshot";
import {
  mergeEngineSnapshotSyncResponse,
  snapshotLiveFixtures,
} from "./engineSnapshotLiveState";
import { groupStrobeCompatibleFixtureCount } from "./groupStrobe";
import { createTimelineOverviewAutomationController } from "./createTimelineOverviewAutomationController";
import { createTimelineKeyframeController } from "./createTimelineKeyframeController";
import { createTimelineAutomationController } from "./createTimelineAutomationController";
import { createVideoRuntimeController } from "./createVideoRuntimeController";
import { createLiveVideoMonitorController } from "./createLiveVideoMonitorController";
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
import { cueIdentityCss, type CueIdentitySource } from "./identityColor";
import {
  applySceneMatrixCueMove,
  sceneMatrixCueMetadataArgs,
} from "./sceneMatrixBankMove";
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
  channelFunctionWheelColor,
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
  timelineAutomationDraftMatchesSummary,
  timelineEventDraftFromSummary,
  timelineVideoAutomationDraftFromSummary,
  timelineVideoAutomationDraftMatchesSummary,
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
  commonFixtureTypeControls,
  fixtureControlForAttribute,
  groupPickedFixturesByType,
} from "./fixtureTypeLiveEdit";
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
import { displayNumber } from "./numberDisplay";
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
  timelineOverviewMarkerAriaLabel,
  translateUiText,
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

const emptyVjPreviewTransport = (): VjPreviewTransportSummary => ({
  layer_id: null,
  playing: false,
  position_ms: 0,
  duration_ms: null,
  speed: 1,
  loop_enabled: false,
  loop_start_ms: 0,
  loop_end_ms: 0,
  updated_at_ms: 0,
  generation: 0,
  source_name: null,
});

const emptyAutoVjSnapshot = defaultAutoVjSnapshot();

const isTauriRuntime = () =>
  typeof window !== "undefined" && Boolean((window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__);

const projectHistoryChangedEvent = "syndocal:project-history-changed";
let activeOperatorLockMode: OperatorLockMode | null = null;

const projectMutationCommands = new Set([
  "analyze_audio_file",
  "clear_timeline_audio",
  "add_timeline_audio_clip",
  "update_timeline_audio_clip",
  "remove_timeline_audio_clip",
  "set_timeline_audio_master",
  "create_custom_fixture_profile",
  "use_fixture_profile",
  "patch_fixture",
  "patch_fixtures",
  "remove_fixture",
  "repair_fixture_profile",
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
  "set_cue_effect_targets",
  "set_cue_metadata",
  "set_cue_child_timeline",
  "set_cue_steps",
  "set_cue_color",
  "set_cue_live_modifier_defaults",
  "set_group_color",
  "move_cue",
  "duplicate_cue",
  "remove_cue",
  "add_timeline_cue_event",
  "set_timeline_cue_event",
  "add_timeline_scene_block",
  "set_timeline_scene_block",
  "remove_timeline_scene_block",
  "snap_timeline_items",
  "remove_timeline_event",
  "add_timeline_layer",
  "update_timeline_layer",
  "remove_timeline_layer",
  "reorder_timeline_layers",
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
  "bootstrap_vj_show",
  "refresh_video_layer_metadata",
  "add_video_input_layer",
  "duplicate_video_layer",
  "remove_video_layer",
  "set_video_layer_order",
  "set_video_layer_label",
  "set_video_layer_state",
  "set_video_layer_isf_effect",
  "apply_builtin_video_isf_effect",
  "add_video_layer_isf_effect",
  "add_builtin_video_isf_effect",
  "move_video_layer_isf_effect",
  "remove_video_layer_isf_effect",
  "set_video_layer_isf_effect_enabled",
  "reset_video_layer_isf_effect",
  "set_video_layer_isf_control",
  "fade_video_layer_opacity",
  "launch_video_clip",
  "take_video_clip",
  "set_auto_vj_config",
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
  "add_color_effect",
  "add_chaser_effect",
  "add_move_effect",
  "add_value_effect",
  "add_curve_effect",
  "add_mapping_effect",
  "add_color_mapping_effect",
  "update_lfo_effect",
  "update_position_wave_effect",
  "update_color_effect",
  "update_chaser_effect",
  "update_move_effect",
  "update_value_effect",
  "update_curve_effect",
  "update_mapping_effect",
  "update_color_mapping_effect",
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
  "set_touch_surface",
  "set_operator_policy",
  "clear_operator_policy",
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
    /(id|ids|attribute|field|index|kind|scope|universe|channel|parameter|name)$/i.test(key),
  );
  return targetEntries.length > 0 ? `${command}:${JSON.stringify(Object.fromEntries(targetEntries))}` : "";
};

const invoke = async <T,>(command: string, args?: Record<string, unknown>): Promise<T> => {
  if (!isTauriRuntime()) {
    throw new Error(tauriBackendUnavailableMessage);
  }
  if (!operatorCommandAllowed(activeOperatorLockMode, command, projectMutationCommands.has(command))) {
    throw new Error(
      activeOperatorLockMode === "Full"
        ? "Operator Full Lock allows only status reads and emergency blackout controls."
        : "Operator Partial Lock blocks programming and project replacement commands.",
    );
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
const timelineAudioFileExtensions = ["wav", "mp3", "m4a", "aac", "flac", "aiff", "aif", "ogg", "opus"];
const isTimelineAudioFilePath = (path: string) => {
  const normalized = path.trim().toLowerCase();
  return timelineAudioFileExtensions.some((extension) => normalized.endsWith(`.${extension}`));
};
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
type CueCaptureScopeMode = "all" | "lighting" | "effects" | "selectedFixture" | "selectedGroup" | "video";
type ControlFaderWriteMode = "live" | "edit";
type TimelineLightingAutomationRowScope = "all" | "current";
type TimelineVideoAutomationRowScope = "all" | "layer";
type SelectedTimelineAutomation = {
  kind: "lighting" | "video";
  automationId: number;
};
type ControlEditCaptureTarget =
  | { kind: "selectedFixture"; fixtureId: number }
  | { kind: "selectedGroup"; groupId: string };
type ViewportControlEditHistoryEntry = {
  cueId: number;
  beforeTargets: CueSummary["targets"];
  afterTargets: CueSummary["targets"];
};
type ViewportSceneMatrixBankMoveHistoryEntry = {
  beforeCues: CueSummary[];
  afterCues: CueSummary[];
};

interface VjFirstRunSetupResult {
  layer_ids: number[];
  composition_id: number;
  output_id: number;
}

interface LiveAudioInputLevels {
  running: boolean;
  stale: boolean;
  safety_clear_pending: boolean;
  bass: number;
  mid: number;
  high: number;
  bands: number[];
  band_count: number;
  rms: number;
  peak: number;
  spectral_flux: number;
  spectral_centroid: number;
  spectral_density_fast: number;
  spectral_density_slow: number;
  kick_strength: number;
  snare_strength: number;
  kick_event: boolean;
  snare_event: boolean;
  onset: boolean;
  onset_strength: number;
  bpm?: number | null;
  bpm_confidence: number;
  beat_phase: number;
  feature_sequence: number;
}

type CueCaptureScopeRequest =
  | { kind: "all" }
  | { kind: "lightingOnly" }
  | { kind: "effectsOnly" }
  | { kind: "selectedFixture"; fixtureId: number }
  | { kind: "selectedGroup"; groupId: string }
  | { kind: "videoOnly" };

const cuePadSize = 10;
const defaultCustomAttributesText = "Dimmer@1:8, Pan@2:16, Tilt@4:16, ColorRed@6:8, ColorGreen@7:8, ColorBlue@8:8";

const chaserTraversalStepCount = (effect: EffectSummary) => {
  const chaser = effect.chaser;
  if (!chaser) return 0;
  const stepCount = chaser.steps.length;
  if (stepCount <= 1) return Math.max(1, stepCount);
  return chaser.direction === "Bounce" ? stepCount * 2 - 2 : stepCount;
};

const authoredBeatsForEffectClock = (effect: EffectSummary) => {
  switch (effect.effect_type) {
    case "Color":
      return effect.color?.clock_sync?.beats ?? effect.clock_sync?.beats ?? null;
    case "Chaser":
      return effect.chaser?.clock_sync?.beats ?? effect.clock_sync?.beats ?? null;
    case "Move":
      return effect.move_effect?.clock_sync?.beats ?? effect.clock_sync?.beats ?? null;
    case "Value":
      return effect.value?.clock_sync?.beats ?? effect.clock_sync?.beats ?? null;
    case "Curve":
      return effect.curve?.clock_sync?.beats ?? effect.clock_sync?.beats ?? null;
    case "Mapping":
      return effect.mapping?.clock_sync?.beats ?? effect.clock_sync?.beats ?? null;
    case "ColorMapping":
      return effect.color_mapping?.clock_sync?.beats ?? effect.clock_sync?.beats ?? null;
    default:
      return effect.clock_sync?.beats ?? null;
  }
};

const inferredCueAuthoredBeats = (effects: EffectSummary[], targets: CueEffectTarget[]) => {
  const effectsById = new Map(effects.map((effect) => [effect.id, effect]));
  const candidates = targets.filter((target) => target.enabled).flatMap((target) => {
    const effect = effectsById.get(target.effect_id);
    if (!effect) return [];
    if (effect.effect_type === "Chaser") {
      const traversalSteps = chaserTraversalStepCount(effect);
      if (traversalSteps <= 0) return [];
      const syncBeats = authoredBeatsForEffectClock(effect);
      return [(syncBeats ?? 1) * traversalSteps];
    }
    const syncBeats = authoredBeatsForEffectClock(effect);
    return syncBeats === null ? [] : [syncBeats];
  });
  if (candidates.length === 0) return null;
  if (candidates.some((candidate) => !Number.isFinite(candidate) || candidate < 0.25 || candidate > 1024)) {
    return null;
  }
  const first = candidates[0];
  return candidates.every((candidate) => Math.abs(candidate - first) <= 1e-6) ? first : null;
};

const cueOwnedEffectSummary = (
  target: CueEffectTarget,
  source: EffectSummary | null,
): EffectSummary | null => {
  const params = target.params;
  if (!params) return source;
  const shell = (
    effectType: EffectKind,
    label: string,
    fixtureIds: number[],
    targetGroupIds: string[],
    attribute: string,
    periodMs: number | null,
    clockSync: EffectSummary["clock_sync"],
    low: number,
    high: number,
    phase: number,
    blendMode: EffectBlendMode,
  ): EffectSummary => ({
    id: target.effect_id,
    label,
    effect_type: effectType,
    fixture_ids: fixtureIds,
    target_group_ids: targetGroupIds,
    attribute,
    video_targets: [],
    shape: source?.shape ?? "Sine",
    period_ms: periodMs,
    clock_sync: clockSync,
    low,
    high,
    phase,
    blend_mode: blendMode,
    enabled: target.enabled,
    color: null,
    chaser: null,
    move_effect: null,
    value: null,
    curve: null,
    mapping: null,
    color_mapping: null,
  });
  if ("Lfo" in params) {
    const request = params.Lfo;
    return {
      ...shell(
        "Lfo",
        request.label,
        request.fixture_ids,
        request.target_group_ids,
        request.attribute,
        request.period_ms,
        request.clock_sync,
        request.low,
        request.high,
        request.phase,
        request.blend_mode,
      ),
      video_targets: request.video_targets,
      shape: request.shape,
    };
  }
  if ("PositionWave" in params) {
    const request = params.PositionWave;
    return {
      ...shell(
        "PositionWave",
        request.label,
        request.fixture_ids,
        request.target_group_ids,
        request.attribute,
        source?.period_ms ?? null,
        request.clock_sync,
        request.low,
        request.high,
        request.phase,
        request.blend_mode,
      ),
      video_targets: request.video_targets,
      shape: request.shape,
      origin: request.origin,
      direction: request.direction,
      speed: request.speed,
      wavelength: request.wavelength,
    };
  }
  if ("Color" in params) {
    const request = params.Color;
    return {
      ...shell(
        "Color",
        request.label,
        request.fixture_ids,
        request.target_group_ids,
        "",
        request.period_ms,
        request.clock_sync,
        0,
        65_535,
        request.phase,
        request.blend_mode,
      ),
      color: request,
    };
  }
  if ("Chaser" in params) {
    const request = params.Chaser;
    const fixtureIds = [...new Set(request.steps.flatMap((step) => step.fixture_ids))];
    const groupIds = [...new Set(request.steps.flatMap((step) => step.target_group_ids))];
    const feature = request.features[0];
    return {
      ...shell(
        "Chaser",
        request.label,
        fixtureIds,
        groupIds,
        feature?.attribute ?? "",
        request.step_duration_ms,
        request.clock_sync,
        feature?.low ?? 0,
        feature?.high ?? 65_535,
        request.phase,
        request.blend_mode,
      ),
      chaser: request,
    };
  }
  if ("Move" in params) {
    const request = params.Move;
    return {
      ...shell(
        "Move",
        request.label,
        request.fixture_ids,
        request.target_group_ids,
        "Pan/Tilt",
        request.period_ms,
        request.clock_sync,
        0,
        65_535,
        request.phase,
        request.blend_mode,
      ),
      move_effect: request,
    };
  }
  if ("Value" in params) {
    const request = params.Value;
    return {
      ...shell(
        "Value",
        request.label,
        request.fixture_ids,
        request.target_group_ids,
        request.attribute,
        request.period_ms,
        request.clock_sync,
        request.low,
        request.high,
        request.phase,
        request.blend_mode,
      ),
      value: request,
    };
  }
  if ("Curve" in params) {
    const request = params.Curve;
    return {
      ...shell(
        "Curve",
        request.label,
        request.fixture_ids,
        request.target_group_ids,
        request.attribute,
        request.period_ms,
        request.clock_sync,
        request.low,
        request.high,
        request.phase,
        request.blend_mode,
      ),
      curve: request,
    };
  }
  if ("Mapping" in params) {
    const request = params.Mapping;
    return {
      ...shell(
        "Mapping",
        request.label,
        request.fixture_ids,
        request.target_group_ids,
        request.attribute,
        request.period_ms,
        request.clock_sync,
        request.low,
        request.high,
        request.phase,
        request.blend_mode,
      ),
      shape: request.shape,
      mapping: request,
    };
  }
  const request = params.ColorMapping;
  return {
    ...shell(
      "ColorMapping",
      request.label,
      request.fixture_ids,
      request.target_group_ids,
      "Colour Mapping",
      request.period_ms,
      request.clock_sync,
      0,
      65_535,
      request.phase,
      request.blend_mode,
    ),
    color_mapping: request,
  };
};


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
  const [dvcImportReport, setDvcImportReport] = createSignal<DvcImportReport | null>(null);
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
  const [namedWorkspaces, setNamedWorkspaces] = createSignal<NamedWorkspaceProfile[]>(loadNamedWorkspaces());
  const [selectedNamedWorkspaceId, setSelectedNamedWorkspaceId] = createSignal<string | null>(null);
  const [operatorPolicy, setOperatorPolicy] = createSignal<OperatorPolicy | null>(null);
  const [operatorPolicyReady, setOperatorPolicyReady] = createSignal(false);
  const [operatorLockMode, setOperatorLockMode] = createSignal<OperatorLockMode | null>(null);
  const [cleanProjectSignature, setCleanProjectSignature] = createSignal<string | null>(null);
  const [workspaceTab, setWorkspaceTab] = createSignal<WorkspaceTab>(initialWorkspaceLayout.workspace_tab);
  const [topSplitRatio, setTopSplitRatio] = createSignal(initialWorkspaceLayout.top_split_ratio);
  const [lowerSplitRatio, setLowerSplitRatio] = createSignal(initialWorkspaceLayout.lower_split_ratio);
  const [selectionsDrawerOpen, setSelectionsDrawerOpen] = createSignal(
    initialWorkspaceLayout.selections_drawer_open,
  );
  const [uiScale, setUiScale] = createSignal<UiScale>(loadUiScale());
  const [uiLocale, setUiLocale] = createSignal<UiLocale>(loadUiLocale());
  const [setupSubTab, setSetupSubTab] = createSignal<SetupSubTab>(initialWorkspaceLayout.setup_sub_tab);
  const [controlMode, setControlMode] = createSignal<ControlMode>(initialWorkspaceLayout.control_mode);
  const [timelineDeskSurface, setTimelineDeskSurface] = createSignal<TimelineDeskSurface>(
    initialWorkspaceLayout.timeline_desk_surface,
  );
  const [timelineContextDrawer, setTimelineContextDrawer] = createSignal<TimelineContextDrawer>(
    initialWorkspaceLayout.timeline_context_drawer,
  );
  const [timelineChildCueId, setTimelineChildCueId] = createSignal<number | null>(null);
  const [controlLiveView, setControlLiveView] = createSignal<"matrix" | "pads">("matrix");
  const [liveStatusExpanded, setLiveStatusExpanded] = createSignal(false);
  const [editDeskSurface, setEditDeskSurface] = createSignal<EditDeskSurface>(initialWorkspaceLayout.edit_desk_surface);
  const [revealedSourceCueId, setRevealedSourceCueId] = createSignal<number | null>(null);
  const [revealedSourceCueRevision, setRevealedSourceCueRevision] = createSignal(0);
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
  // Projection surfaces are reference-only on the lighting floor plan and default OFF;
  // the V hotkey and the toolbar/viewport toggles still turn them on.
  const [mappingShowProjectors, setMappingShowProjectors] = createSignal(false);
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
  const [colorWheelPickerSelection, setColorWheelPickerSelection] = createSignal<{
    targetKey: string;
    color: string;
    entryKey: string;
  } | null>(null);
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
  const [cueAuthoredBeats, setCueAuthoredBeats] = createSignal<number | null>(null);
  const [cueAuthoredBeatsManual, setCueAuthoredBeatsManual] = createSignal(false);
  const [cueCaptureScope, setCueCaptureScope] = createSignal<CueCaptureScopeMode>("all");
  const [cueEffectCaptureTargets, setCueEffectCaptureTargets] = createSignal<CueEffectTarget[]>([]);
  const [cueEffectCaptureStateOverrideIds, setCueEffectCaptureStateOverrideIds] = createSignal<number[]>([]);
  const [selectedCueListId, setSelectedCueListId] = createSignal(1);
  const [selectedSceneCueId, setSelectedSceneCueId] = createSignal<number | null>(null);
  const [selectedSceneEffectId, setSelectedSceneEffectId] = createSignal<number | null>(null);
  const [sceneSettingsSurface, setSceneSettingsSurface] =
    createSignal<SceneSettingsSurface>("contents");
  const [controlFaderWriteMode, setControlFaderWriteMode] =
    createSignal<ControlFaderWriteMode>("live");
  const [viewportControlEditUndo, setViewportControlEditUndo] =
    createSignal<ViewportControlEditHistoryEntry[]>([]);
  const [viewportControlEditRedo, setViewportControlEditRedo] =
    createSignal<ViewportControlEditHistoryEntry[]>([]);
  const [viewportSceneMatrixBankMoveUndo, setViewportSceneMatrixBankMoveUndo] =
    createSignal<ViewportSceneMatrixBankMoveHistoryEntry | null>(null);
  const [cueListLabel, setCueListLabel] = createSignal("Main");
  const [cueMetadataDrafts, setCueMetadataDrafts] = createSignal<Record<number, CueMetadataDraft>>({});
  const [cuePadBank, setCuePadBank] = createSignal(0);
  const [cuePadFollowActive, setCuePadFollowActive] = createSignal(true);
  const [timelineCueId, setTimelineCueId] = createSignal<number | null>(null);
  const [timelineEventTimeMs, setTimelineEventTimeMs] = createSignal(0);
  const [timelineBlockDurationMs, setTimelineBlockDurationMs] = createSignal(1000);
  const [timelineBlockLoopCount, setTimelineBlockLoopCount] = createSignal(1);
  const [timelineBlockJumpToEventId, setTimelineBlockJumpToEventId] = createSignal<number | null>(null);
  const [timelineTrack, setTimelineTrack] = createSignal<TimelineTrackKind>("Lighting");
  const [timelineCueDrag, setTimelineCueDrag] = createSignal<TimelineCueDragState | null>(null);
  const [timelineEventDrafts, setTimelineEventDrafts] = createSignal<Record<number, TimelineEventDraft>>({});
  const [selectedTimelineSceneBlockEventId, setSelectedTimelineSceneBlockEventId] = createSignal<number | null>(null);
  const [timelineSceneBlockSelectionRevision, setTimelineSceneBlockSelectionRevision] = createSignal(0);
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
  const [videoPreviewLayerId, setVideoPreviewLayerId] = createSignal<number | null>(null);
  const [isfEventPulseBusy, setIsfEventPulseBusy] = createSignal(false);
  const [vjPreviewTransport, setVjPreviewTransport] = createSignal<VjPreviewTransportSummary>(
    emptyVjPreviewTransport(),
  );
  const [vjPreviewTransportBusy, setVjPreviewTransportBusy] = createSignal(false);
  const [vjPreviewTransportError, setVjPreviewTransportError] = createSignal<string | null>(null);
  const [vjFirstRunBusy, setVjFirstRunBusy] = createSignal(false);
  const [vjFirstRunError, setVjFirstRunError] = createSignal<string | null>(null);
  const [vjFirstRunAwaitingSync, setVjFirstRunAwaitingSync] = createSignal(false);
  const [autoVjBusy, setAutoVjBusy] = createSignal(false);
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
  const [liveAudioInputDevices, setLiveAudioInputDevices] = createSignal<LiveAudioInputDeviceSummary[]>([]);
  const [liveAudioInputBackends, setLiveAudioInputBackends] =
    createSignal<LiveAudioInputBackendSummary[]>([]);
  const [selectedLiveAudioInputBackend, setSelectedLiveAudioInputBackend] =
    createSignal<LiveAudioInputBackendId>("wasapi_shared");
  const [liveAudioInputBackendsKnown, setLiveAudioInputBackendsKnown] = createSignal(false);
  const [liveAudioInputBackendsBusy, setLiveAudioInputBackendsBusy] = createSignal(false);
  const [liveAudioInputBackendError, setLiveAudioInputBackendError] = createSignal<string | null>(null);
  const [selectedLiveAudioInputDevice, setSelectedLiveAudioInputDevice] = createSignal("");
  const [liveAudioInputCapabilities, setLiveAudioInputCapabilities] =
    createSignal<LiveAudioInputCapabilities | null>(null);
  const [liveAudioInputCapabilitiesBusy, setLiveAudioInputCapabilitiesBusy] = createSignal(false);
  const [liveAudioInputSampleRate, setLiveAudioInputSampleRate] = createSignal<number | null>(null);
  const [liveAudioInputBufferFrames, setLiveAudioInputBufferFrames] = createSignal<number | null>(null);
  const [liveAudioInputChannelMix, setLiveAudioInputChannelMix] = createSignal<LiveAudioChannelMix>({
    mode: "average_all",
  });
  const [liveAudioInputStatus, setLiveAudioInputStatus] = createSignal<LiveAudioInputStatus>({
    running: false,
    stale: false,
    safety_clear_pending: false,
    device_id: null,
    device_name: null,
    backend: null,
    sample_format: null,
    sample_rate: 0,
    channels: 0,
    configured_buffer_frames: null,
    applied_buffer_frames: null,
    channel_mix: { mode: "average_all" },
    bass: 0,
    mid: 0,
    high: 0,
    bands: Array.from({ length: 16 }, () => 0),
    band_count: 0,
    rms: 0,
    peak: 0,
    spectral_flux: 0,
    spectral_centroid: 0,
    spectral_density_fast: 0,
    spectral_density_slow: 0,
    kick_strength: 0,
    snare_strength: 0,
    kick_event: false,
    snare_event: false,
    onset: false,
    onset_strength: 0,
    bpm: null,
    bpm_confidence: 0,
    beat_phase: 0,
    feature_sequence: 0,
    analyzed_windows: 0,
    dropped_chunks: 0,
    dropped_frames: 0,
    backend_xruns: 0,
    callback_count: 0,
    last_callback_frames: 0,
    min_callback_frames: 0,
    max_callback_frames: 0,
    capture_to_worker_us: 0,
    max_capture_to_worker_us: 0,
    queue_depth: 0,
    queue_capacity: 0,
    queue_depth_high_water: 0,
    last_error: null,
  });
  const [liveAudioInputStatusKnown, setLiveAudioInputStatusKnown] = createSignal(!isTauriRuntime());
  const [liveAudioInputBusy, setLiveAudioInputBusy] = createSignal(false);
  const [liveAudioInputTelemetryFresh, setLiveAudioInputTelemetryFresh] = createSignal(false);
  let liveAudioInputTelemetryAcceptedAt = 0;
  const clearLiveAudioInputTelemetryFreshness = () => {
    liveAudioInputTelemetryAcceptedAt = 0;
    setLiveAudioInputTelemetryFresh(false);
  };
  const acceptLiveAudioInputTelemetry = () => {
    liveAudioInputTelemetryAcceptedAt = performance.now();
    setLiveAudioInputTelemetryFresh(true);
  };
  const expireLiveAudioInputTelemetry = () => {
    if (
      liveAudioInputTelemetryFresh() &&
      performance.now() - liveAudioInputTelemetryAcceptedAt >= TOPBAR_PULSE_STALE_MS
    ) {
      clearLiveAudioInputTelemetryFreshness();
    }
  };
  const liveAudioStatusRequests = createLiveAudioInputStatusRequestGate();
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
  let mappingStageResizeObserver: ResizeObserver | undefined;
  const [mappingStageViewportPixelSize, setMappingStageViewportPixelSize] =
    createSignal({ width: 640, height: 390 });
  const bindMappingStageSvgElement = (element: SVGSVGElement) => {
    mappingStageSvgElement = element;
    mappingStageResizeObserver?.disconnect();
    const measure = () => {
      const rect = element.getBoundingClientRect();
      const style = getComputedStyle(element);
      const contentWidth = rect.width
        - (Number.parseFloat(style.borderLeftWidth) || 0)
        - (Number.parseFloat(style.borderRightWidth) || 0);
      const contentHeight = rect.height
        - (Number.parseFloat(style.borderTopWidth) || 0)
        - (Number.parseFloat(style.borderBottomWidth) || 0);
      setMappingStageViewportPixelSize({
        width: Math.max(1, contentWidth || element.clientWidth),
        height: Math.max(1, contentHeight || element.clientHeight),
      });
    };
    measure();
    if (typeof ResizeObserver !== "undefined") {
      mappingStageResizeObserver = new ResizeObserver(measure);
      mappingStageResizeObserver.observe(element);
    }
  };
  onCleanup(() => mappingStageResizeObserver?.disconnect());
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
  const [effectType, setEffectType] = createSignal<EffectKind>("Curve");
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
  const [effectChooserFamily, setEffectChooserFamily] = createSignal<EffectRecipeFamily>("CURVE FX");
  const [editingEffectId, setEditingEffectId] = createSignal<number | null>(null);
  const [effectPeriod, setEffectPeriod] = createSignal(1000);
  const [effectClockSyncBeats, setEffectClockSyncBeats] = createSignal<number | null>(null);
  const [colorEffectStops, setColorEffectStops] = createSignal<ColorEffectStop[]>(
    defaultColorEffectStops.map((stop) => ({ ...stop, color: { ...stop.color } })),
  );
  const [colorEffectAlgorithm, setColorEffectAlgorithm] = createSignal<ColorEffectAlgorithm>("Cycle");
  const [colorEffectInterpolation, setColorEffectInterpolation] =
    createSignal<ColorEffectInterpolation>("HsvShortest");
  const [colorEffectFixtureSpread, setColorEffectFixtureSpread] = createSignal(0);
  const [colorEffectSpatialPattern, setColorEffectSpatialPattern] =
    createSignal<ColorEffectSpatialPattern | null>(null);
  const [chaserSteps, setChaserSteps] = createSignal<ChaserStep[]>([]);
  const [chaserFeatures, setChaserFeatures] = createSignal<ChaserFeature[]>([
    { attribute: "Dimmer", low: 0, high: 65_535 },
  ]);
  const [chaserStepDuration, setChaserStepDuration] = createSignal(250);
  const [chaserDirection, setChaserDirection] = createSignal<ChaserDirection>("Forward");
  const [chaserWings, setChaserWings] = createSignal(1);
  const [chaserActiveStepCount, setChaserActiveStepCount] = createSignal(1);
  const [chaserDutyCycle, setChaserDutyCycle] = createSignal(1);
  const [chaserOverlap, setChaserOverlap] = createSignal(0);
  const [chaserFixtureSpread, setChaserFixtureSpread] = createSignal(0);
  const [chaserRandomSeed, setChaserRandomSeed] = createSignal(chaserDefaultSeed);
  const [movePathPoints, setMovePathPoints] = createSignal<MovePathPoint[]>(defaultMovePathPoints());
  const [movePathClosed, setMovePathClosed] = createSignal(true);
  const [moveInterpolation, setMoveInterpolation] = createSignal<MoveInterpolation>("Smooth");
  const [moveCoordinateMode, setMoveCoordinateMode] = createSignal<MoveCoordinateMode>("Absolute");
  const [moveCenterX, setMoveCenterX] = createSignal(0.5);
  const [moveCenterY, setMoveCenterY] = createSignal(0.5);
  const [moveSizeX, setMoveSizeX] = createSignal(1);
  const [moveSizeY, setMoveSizeY] = createSignal(1);
  const [moveRotationDegrees, setMoveRotationDegrees] = createSignal(0);
  const [moveDirection, setMoveDirection] = createSignal<MoveDirection>("Forward");
  const [moveFixtureSpread, setMoveFixtureSpread] = createSignal(0);
  const [movePathRecipe, setMovePathRecipe] = createSignal<MovePathRecipe>("Circle");
  const [valuePoints, setValuePoints] = createSignal<ValueEffectPoint[]>([
    { position: 0, value: 0 },
    { position: 0.5, value: 1 },
    { position: 1, value: 0 },
  ]);
  const [valueInterpolation, setValueInterpolation] = createSignal<ValueEffectInterpolation>("Smooth");
  const [valueMode, setValueMode] = createSignal<ValueEffectMode>("Absolute");
  const [valueDirection, setValueDirection] = createSignal<ValueEffectDirection>("Forward");
  const [valueFixtureSpread, setValueFixtureSpread] = createSignal(0);
  const [curvePoints, setCurvePoints] = createSignal<CurveEffectPoint[]>([
    { position: 0, value: 0, in_tangent: 0, out_tangent: 1 },
    { position: 1, value: 1, in_tangent: 1, out_tangent: 0 },
  ]);
  const [curveMode, setCurveMode] = createSignal<ValueEffectMode>("Absolute");
  const [curveDirection, setCurveDirection] = createSignal<ValueEffectDirection>("Forward");
  const [curveFixtureSpread, setCurveFixtureSpread] = createSignal(0);
  const [mappingMode, setMappingMode] = createSignal<ValueEffectMode>("Absolute");
  const [mappingDirection, setMappingDirection] = createSignal<MappingEffectDirection>("Forward");
  const [mappingFixtureSpread, setMappingFixtureSpread] = createSignal(1);
  const [mappingRepetitions, setMappingRepetitions] = createSignal(1);
  const initialColorMappingRaster = defaultColorMappingRaster();
  const [colorMappingSourceKind, setColorMappingSourceKind] = createSignal<ColorMappingSourceKind>("Image");
  const [colorMappingWidth, setColorMappingWidth] = createSignal(initialColorMappingRaster.width);
  const [colorMappingHeight, setColorMappingHeight] = createSignal(initialColorMappingRaster.height);
  const [colorMappingFrames, setColorMappingFrames] = createSignal<ColorMappingFrame[]>(initialColorMappingRaster.frames);
  const [colorMappingCells, setColorMappingCells] = createSignal<ColorMappingCellTarget[]>([]);
  const [colorMappingPlaybackDirection, setColorMappingPlaybackDirection] =
    createSignal<ColorMappingPlaybackDirection>("Forward");
  const [colorMappingOffsetU, setColorMappingOffsetU] = createSignal(0);
  const [colorMappingOffsetV, setColorMappingOffsetV] = createSignal(0);
  const [colorMappingScaleU, setColorMappingScaleU] = createSignal(1);
  const [colorMappingScaleV, setColorMappingScaleV] = createSignal(1);
  const [colorMappingRotationDegrees, setColorMappingRotationDegrees] = createSignal(0);
  const [colorMappingWrapMode, setColorMappingWrapMode] = createSignal<ColorMappingWrapMode>("Clamp");
  const [colorMappingSampling, setColorMappingSampling] = createSignal<ColorMappingSampling>("Nearest");
  const [effectLow, setEffectLow] = createSignal(0);
  const [effectHigh, setEffectHigh] = createSignal(65535);
  const [effectPhase, setEffectPhase] = createSignal(0);
  const [effectBlendMode, setEffectBlendMode] = createSignal<EffectBlendMode>("Override");
  const [effectAttribute, setEffectAttribute] = createSignal("");
  const [waveOriginX, setWaveOriginX] = createSignal(0);
  const [waveOriginY, setWaveOriginY] = createSignal(0);
  const [waveOriginZ, setWaveOriginZ] = createSignal(0);
  const [waveDirectionX, setWaveDirectionX] = createSignal(1);
  const [waveDirectionY, setWaveDirectionY] = createSignal(0);
  const [waveDirectionZ, setWaveDirectionZ] = createSignal(0);
  const [waveStageDrag, setWaveStageDrag] = createSignal<WaveStageDragMode | null>(null);
  const [waveSpeed, setWaveSpeed] = createSignal(1);
  const [waveWavelength, setWaveWavelength] = createSignal(2);
  const initialEngineSnapshot = createInitialEngineSnapshot();
  const [snapshot, setSnapshot] = createSignal<EngineSnapshot>(initialEngineSnapshot);
  const engineDmxPreviews = (next: EngineSnapshot) => {
    const previews = next.dmx_previews ?? [];
    const legacyPreview = next.dmx_preview ?? [];
    return previews.length > 0
      ? previews
      : legacyPreview.length > 0
        ? [{ universe: next.output.universe, values: legacyPreview }]
        : [];
  };
  const [liveDmxPreviews, setLiveDmxPreviews] = createSignal(engineDmxPreviews(initialEngineSnapshot));
  const [liveFixtures, setLiveFixtures] = createSignal(snapshotLiveFixtures(initialEngineSnapshot));
  let latestEngineSnapshot = initialEngineSnapshot;
  const [snapshotRevision, setSnapshotRevision] = createSignal<number | null>(null);
  let loadSceneEffectDraft = (_cueId: number, _effectId: number) => {};
  const selectSceneCue = (cueId: number) => {
    const cue = snapshot().cues.find((candidate) => candidate.id === cueId);
    if (!cue) return;
    const currentEffectId = selectedSceneEffectId();
    const effectId = cue.effect_targets.some((target) => target.effect_id === currentEffectId)
      ? currentEffectId
      : cue.effect_targets[0]?.effect_id ?? null;
    setSelectedSceneCueId(cueId);
    setSelectedSceneEffectId(effectId);
    setSceneSettingsSurface("contents");
    if (effectId !== null) loadSceneEffectDraft(cueId, effectId);
  };
  const closeSceneSettings = () => {
    setSelectedSceneCueId(null);
    setSelectedSceneEffectId(null);
    setSceneSettingsSurface("contents");
  };
  const openCueEditor = () => {
    setTimelineDeskSurface("show");
    setTimelineContextDrawer("cue");
  };
  const snapshotCues = createMemo(() => snapshot().cues);
  const timelineChildCue = createMemo(() => {
    const cueId = timelineChildCueId();
    return cueId === null
      ? null
      : snapshot().cues.find((cue) => cue.id === cueId && cue.child_timeline) ?? null;
  });
  const normalizedChildTimeline = (child: ChildTimelineSummary): ChildTimelineSummary => ({
    layers: child.layers ?? [],
    events: child.events ?? [],
    automations: child.automations ?? [],
    video_automations: child.video_automations ?? [],
    audio: child.audio ?? null,
    audio_clips: child.audio_clips ?? [],
    duration_ms: Math.max(0, child.duration_ms ?? 0),
  });
  const activeTimeline = createMemo<TimelineSnapshot>(() => {
    const child = timelineChildCue()?.child_timeline;
    if (!child) return snapshot().timeline;
    const normalized = normalizedChildTimeline(child);
    return {
      layers: normalized.layers,
      events: normalized.events ?? [],
      automations: normalized.automations ?? [],
      video_automations: normalized.video_automations ?? [],
      audio: normalized.audio ?? null,
      audio_clips: normalized.audio_clips ?? [],
      audio_offset_ms: 0,
      audio_muted: false,
      playing: false,
      position_ms: 0,
      duration_ms: normalized.duration_ms ?? 0,
    };
  });
  const snapshotTimelineEvents = createMemo(() => activeTimeline().events);
  const snapshotTimelineEventById = createMemo(() => new Map(
    snapshotTimelineEvents().map((event) => [event.id, event]),
  ));
  const dirtyTimelineEventDrafts = createMemo(() => {
    const drafts = timelineEventDrafts();
    const draftEntries = Object.entries(drafts);
    if (draftEntries.length === 0) return {};
    const eventsById = snapshotTimelineEventById();
    const dirtyDrafts: Record<number, TimelineEventDraft> = {};
    for (const [eventId, draft] of draftEntries) {
      const numericEventId = Number(eventId);
      const event = eventsById.get(numericEventId);
      if (!event || !timelineEventDraftMatchesSummary(event, draft)) {
        dirtyDrafts[numericEventId] = draft;
      }
    }
    return dirtyDrafts;
  });
  const timelineEventEditorDirty = createMemo(() => Object.keys(dirtyTimelineEventDrafts()).length > 0);
  const timelineAutomationEditorDirty = createMemo(() => {
    const timeline = activeTimeline();
    const lightingDrafts = timelineAutomationDrafts();
    const videoDrafts = timelineVideoAutomationDrafts();
    return (
      timeline.automations.some((automation) => {
        const draft = lightingDrafts[automation.id];
        return Boolean(draft && !timelineAutomationDraftMatchesSummary(automation, draft));
      }) ||
      timeline.video_automations.some((automation) => {
        const draft = videoDrafts[automation.id];
        return Boolean(draft && !timelineVideoAutomationDraftMatchesSummary(automation, draft));
      })
    );
  });
  const timelineEditorDirty = createMemo(() => timelineEventEditorDirty() || timelineAutomationEditorDirty());
  const visibleProjectDirty = createMemo(() => projectDirty() || timelineEditorDirty());
  const snapshotRequestGuard = createSnapshotRequestGuard();
  const viewportFixture = browserViewportFixture(isTauriRuntime());
  // T12: pane windows collapse the shell to one pane; popped panes are the
  // main window's record of which panes live in separate windows. Window
  // placement is machine-specific, so persistence is localStorage, not .sdc.
  const paneWindow = paneWindowMode();
  const autoOpenPaneWindows = new URLSearchParams(window.location.search).get("syndocalAutoPaneWindows") === "1";
  const initialPoppedPanes = (): PaneWindowKind[] => {
    const fromParam = browserPoppedPanes();
    if (fromParam.length > 0) return fromParam;
    if (autoOpenPaneWindows) return [];
    if (!isTauriRuntime() || paneWindow) return [];
    try {
      const raw = window.localStorage.getItem("syndocal.paneWindows.v1");
      const parsed: unknown = raw ? JSON.parse(raw) : [];
      return Array.isArray(parsed)
        ? parsed.filter((pane): pane is PaneWindowKind =>
            typeof pane === "string" && paneWindowKinds.includes(pane as PaneWindowKind))
        : [];
    } catch {
      return [];
    }
  };
  const [poppedPanes, setPoppedPanes] = createSignal<PaneWindowKind[]>(initialPoppedPanes());
  const persistPoppedPanes = (panes: PaneWindowKind[]) => {
    try {
      window.localStorage.setItem("syndocal.paneWindows.v1", JSON.stringify(panes));
    } catch {
      // Persistence loss only affects window restore on next launch.
    }
  };
  const acknowledgePaneWindowClosed = (pane: string) => {
    setPoppedPanes((current) => {
      const next = current.filter((candidate) => candidate !== pane);
      persistPoppedPanes(next);
      return next;
    });
  };
  const markPaneWindowOpen = (pane: PaneWindowKind) => {
    setPoppedPanes((current) => {
      const next = current.includes(pane) ? current : [...current, pane];
      persistPoppedPanes(next);
      return next;
    });
  };
  let paneWindowEventsReady: Promise<boolean> = Promise.resolve(true);
  const openPaneWindow = async (pane: PaneWindowKind, placement: PaneWindowPlacement | null = null) => {
    if (isTauriRuntime()) {
      if (!await paneWindowEventsReady) {
        setMessage("Pane window events are unavailable.");
        return;
      }
      markPaneWindowOpen(pane);
      try {
        await invoke("open_pane_window", { pane, placement });
      } catch (error) {
        acknowledgePaneWindowClosed(pane);
        setMessage(`Pane window failed: ${error}`);
        return;
      }
      return;
    } else {
      const params = new URLSearchParams(window.location.search);
      params.set("syndocalPaneWindow", pane);
      params.delete("syndocalPoppedPanes");
      window.open(`${window.location.pathname}?${params}`, `syndocal-pane-${pane}`);
    }
    markPaneWindowOpen(pane);
  };
  const closePaneWindow = async (pane: PaneWindowKind) => {
    if (isTauriRuntime()) {
      if (!await paneWindowEventsReady) {
        setMessage("Pane window events are unavailable.");
        return;
      }
      try {
        await invoke("close_pane_window", { pane });
      } catch (error) {
        setMessage(`Pane window close failed: ${error}`);
        return;
      }
      // The child may prevent CloseRequested while it owns unsaved editor
      // drafts. Keep the main-window placeholder until Rust confirms the
      // actual Destroyed event through syndocal://pane-window-closed.
      return;
    }
    acknowledgePaneWindowClosed(pane);
  };
  const togglePaneWindow = (pane: PaneWindowKind) => {
    void (poppedPanes().includes(pane) ? closePaneWindow(pane) : openPaneWindow(pane));
  };
  if (isTauriRuntime() && !paneWindow) {
    let listenerDisposed = false;
    let unlistenPaneWindowClosed: (() => void) | undefined;
    paneWindowEventsReady = listen<string>("syndocal://pane-window-closed", ({ payload }) => {
      acknowledgePaneWindowClosed(payload);
    })
      .then(async (unlisten) => {
        if (listenerDisposed) {
          unlisten();
          return false;
        }
        unlistenPaneWindowClosed = unlisten;
        await Promise.all(poppedPanes().map(async (pane) => {
          try {
            await invoke("open_pane_window", { pane, placement: null });
          } catch (error) {
            acknowledgePaneWindowClosed(pane);
            setMessage(`Pane window restore failed: ${error}`);
          }
        }));
        return true;
      })
      .catch((error) => {
        for (const pane of poppedPanes()) acknowledgePaneWindowClosed(pane);
        setMessage(`Pane window listener failed: ${error}`);
        return false;
      });
    onCleanup(() => {
      listenerDisposed = true;
      unlistenPaneWindowClosed?.();
    });
  }
  if (isTauriRuntime() && !paneWindow && autoOpenPaneWindows) {
    void paneWindowEventsReady.then(async (ready) => {
      if (!ready) return;
      for (const [index, pane] of paneWindowKinds.entries()) {
        await openPaneWindow(pane, {
          pane,
          x: 48 + index * 42,
          y: 64 + index * 34,
          width: 860,
          height: 520,
          maximized: false,
        });
      }
    });
  }
  if (paneWindow) {
    if (paneWindow === "setup") {
      setWorkspaceTab("setup");
    } else if (paneWindow === "touch") {
      setWorkspaceTab("touch");
    } else {
      setWorkspaceTab("control");
      setControlMode(paneWindow === "programmer" ? "edit" : paneWindow === "mixer" ? "mixer" : "live");
    }
  }
  const capturePaneWindowPlacements = async (): Promise<PaneWindowPlacement[]> => {
    if (isTauriRuntime()) {
      return invoke<PaneWindowPlacement[]>("capture_pane_window_placements");
    }
    return poppedPanes().map((pane, index) => ({
      pane,
      x: 80 + index * 48,
      y: 80 + index * 36,
      width: 1_280,
      height: 720,
      maximized: false,
    }));
  };
  const saveNamedWorkspace = async (name: string) => {
    try {
      const placements = await capturePaneWindowPlacements();
      const next = upsertNamedWorkspace(namedWorkspaces(), name, currentWorkspaceLayout(), placements);
      if (!saveNamedWorkspaces(next)) throw new Error("Local workspace storage is unavailable.");
      setNamedWorkspaces(next);
      const saved = next.find((profile) => profile.name.localeCompare(name.trim(), undefined, { sensitivity: "accent" }) === 0);
      setSelectedNamedWorkspaceId(saved?.id ?? null);
      setMessage(`Saved local workspace ${saved?.name ?? name.trim()} with ${placements.length} pane window(s).`);
    } catch (error) {
      setMessage(`Workspace save failed: ${String(error)}`);
    }
  };
  const applyNamedWorkspace = async (profile: NamedWorkspaceProfile) => {
    if (operatorLockMode() !== null) {
      setMessage("Unlock operator mode before changing the workspace layout.");
      return;
    }
    try {
      applyWorkspaceLayout(profile.layout);
      const desired = new Map(profile.pane_windows.map((placement) => [placement.pane, placement]));
      await Promise.all(poppedPanes()
        .filter((pane) => !desired.has(pane))
        .map((pane) => closePaneWindow(pane)));
      await Promise.all(profile.pane_windows.map((placement) => openPaneWindow(placement.pane, placement)));
      const desiredPanes = profile.pane_windows.map((placement) => placement.pane);
      setPoppedPanes(desiredPanes);
      persistPoppedPanes(desiredPanes);
      setSelectedNamedWorkspaceId(profile.id);
      setMessage(`Applied local workspace ${profile.name}.`);
    } catch (error) {
      setMessage(`Workspace restore failed: ${String(error)}`);
    }
  };
  const deleteNamedWorkspace = (profile: NamedWorkspaceProfile) => {
    const next = namedWorkspaces().filter((candidate) => candidate.id !== profile.id);
    if (!saveNamedWorkspaces(next)) {
      setMessage("Workspace delete failed: local workspace storage is unavailable.");
      return;
    }
    setNamedWorkspaces(next);
    if (selectedNamedWorkspaceId() === profile.id) setSelectedNamedWorkspaceId(null);
    setMessage(`Deleted local workspace ${profile.name}.`);
  };

  const configureOperatorPolicy = async (
    password: string,
    lockMode: OperatorLockMode,
    lockOnLoad: boolean,
  ) => {
    try {
      const policy = await createOperatorPolicy(password, lockMode, lockOnLoad);
      await invoke("set_operator_policy", { policy });
      setOperatorPolicy(policy);
      setOperatorPolicyReady(true);
      setCleanProjectSignature("__syndocal_operator_policy_changed__");
      setProjectDirty(true);
      setMessage(`Operator ${lockMode} Lock policy saved to this project; no plaintext password is stored.`);
      return true;
    } catch (error) {
      setMessage(`Operator policy failed: ${String(error)}`);
      return false;
    }
  };
  const clearOperatorPolicy = async () => {
    try {
      await invoke("clear_operator_policy");
      setOperatorSessionLock(null);
      setOperatorPolicy(null);
      setCleanProjectSignature("__syndocal_operator_policy_changed__");
      setProjectDirty(true);
      setMessage("Operator lock policy removed from this project.");
      return true;
    } catch (error) {
      setMessage(`Operator policy removal failed: ${String(error)}`);
      return false;
    }
  };
  const unlockOperator = async (password: string) => {
    const policy = operatorPolicy();
    if (!policy) return false;
    try {
      if (!await verifyOperatorPassword(policy, password)) {
        setMessage("Operator password is incorrect.");
        return false;
      }
      setOperatorSessionLock(null);
      setMessage("Operator lock released.");
      return true;
    } catch (error) {
      setMessage(`Operator unlock failed: ${String(error)}`);
      return false;
    }
  };
  if (
    viewportFixture === "timeline"
    || viewportFixture === "patch"
    || viewportFixture === "timeline-layered"
    || viewportFixture === "scene-block-large"
    || viewportFixture === "scene-block-hour"
    || viewportFixture === "cue-recall"
    || viewportFixture === "cue-recall-large"
    || viewportFixture === "cue-node-graph"
    || viewportFixture === "scene-matrix"
    || viewportFixture === "workspace-operator"
    || viewportFixture === "touch-composed"
    || viewportFixture === "fx-visual"
    || viewportFixture === "live-edit-types"
    || viewportFixture === "edit-live"
    || viewportFixture === "color-wheel"
    || viewportFixture === "control-stage-edit"
  ) {
    const sceneBlockLargeFixture = viewportFixture === "scene-block-large";
    const sceneBlockHourFixture = viewportFixture === "scene-block-hour";
    const timelineLayeredFixture = viewportFixture === "timeline-layered";
    const controlStageEditFixture = viewportFixture === "control-stage-edit";
    const timelineFixture =
      viewportFixture === "timeline"
      || viewportFixture === "patch"
      || timelineLayeredFixture
      || sceneBlockLargeFixture
      || sceneBlockHourFixture
      || controlStageEditFixture;
    const cueRecallFixture = viewportFixture === "cue-recall";
    const cueRecallLargeFixture = viewportFixture === "cue-recall-large";
    const fxVisualFixture = viewportFixture === "fx-visual";
    const cueFixture = cueRecallFixture || cueRecallLargeFixture || fxVisualFixture;
    const cueNodeGraphFixture = viewportFixture === "cue-node-graph";
    const editLiveFixture = viewportFixture === "edit-live";
    const sceneMatrixFixture =
      viewportFixture === "scene-matrix"
      || viewportFixture === "workspace-operator"
      || editLiveFixture
      || controlStageEditFixture;
    const touchComposedFixture = viewportFixture === "touch-composed";
    const liveEditTypeFixture = viewportFixture === "live-edit-types" || editLiveFixture;
    const colorWheelFixture = viewportFixture === "color-wheel";
    const cueFixtureEffects = fxVisualFixture
      ? structuredClone(viewportFixtureData.fxVisualizationEffects)
      : cueRecallLargeFixture
      ? Array.from({ length: 500 }, (_, index) => ({
          ...viewportFixtureData.cueRecallEffect,
          id: viewportFixtureData.cueRecallEffect.id + index,
          label: `Viewport Effect ${index + 1}`,
          enabled: index % 3 !== 0,
        }))
      : [viewportFixtureData.cueRecallEffect];
    const cueFixtureCues = fxVisualFixture
      ? [structuredClone(viewportFixtureData.fxVisualizationCue)]
      : cueRecallLargeFixture
      ? Array.from({ length: 12 }, (_, index) => ({
          ...viewportFixtureData.cueRecallCue,
          id: viewportFixtureData.cueRecallCue.id + index,
          cue_number: String(index + 1),
          label: `Viewport Cue ${index + 1}`,
          effect_targets: [{
            effect_id: cueFixtureEffects[index].id,
            enabled: cueFixtureEffects[index].enabled,
          }],
        }))
      : [viewportFixtureData.cueRecallCue];
    const fixtureCues = sceneMatrixFixture
      ? structuredClone(viewportFixtureData.sceneMatrixCues)
      : cueFixtureCues;
    if (sceneMatrixFixture && fixtureCues[0]) {
      // T7: one persisted cue color so the harness can assert it wins the hash hue.
      fixtureCues[0].color = "#ff3366";
    }
    const timelineSceneBlockViewport = sceneBlockLargeFixture
      ? createTimelineSceneBlockLargeViewportFixture(viewportFixtureData.cueRecallCue)
      : sceneBlockHourFixture
        ? createTimelineSceneBlockHourViewportFixture(viewportFixtureData.cueRecallCue)
        : createTimelineSceneBlockViewportFixture(
          viewportFixtureData.cueRecallCue,
          viewportFixtureData.videoLayer,
        );
    const timelineFixtureCues = timelineLayeredFixture
      ? [...timelineSceneBlockViewport.cues, structuredClone(viewportFixtureData.layeredTimelineSuperSceneCue)]
      : timelineSceneBlockViewport.cues;
    const timelineFixtureEvents = timelineLayeredFixture
      ? [
          ...timelineSceneBlockViewport.events.map((event, index) => ({
            ...event,
            layer_id: event.track === "Lighting" ? (index === 2 ? 13 : 12) : 14,
          })),
          structuredClone(viewportFixtureData.layeredTimelineSuperSceneEvent),
        ]
      : timelineSceneBlockViewport.events;
    const activeCueId = fixtureCues[0].id;
    setWorkspaceTab(touchComposedFixture ? "touch" : "control");
    if (fxVisualFixture) {
      setControlMode("live");
      setTimelineDeskSurface("show");
      setSceneSettingsSurface("fx");
      setEffectChooserFamily("VALUE FX");
      setEffectType("Value");
    }
    if (sceneMatrixFixture) {
      setTimelineDeskSurface("show");
      setTimelineContextDrawer("none");
      setControlLiveView("matrix");
    }
    if (liveEditTypeFixture || colorWheelFixture) {
      setControlMode("edit");
      setEditDeskSurface("attributes");
      setControlCategory(colorWheelFixture ? "color" : "dimmer");
    }
    setSelectedFixtureGroupFilter(colorWheelFixture ? "moving" : liveEditTypeFixture ? "" : "front");
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
    setSelectedMappingFixtureIds(
      liveEditTypeFixture || colorWheelFixture
        ? (colorWheelFixture ? viewportFixtureData.colorWheelFixtures : viewportFixtureData.liveEditTypeFixtures)
            .map((fixture) => fixture.id)
        : [1],
    );
    setSelectedFixtureLabelDraft(
      colorWheelFixture ? "stage evolution mini spot 30 1" : liveEditTypeFixture ? "GENERIC 1" : "Viewport Par L",
    );
    setSelectedFixtureUniverseDraft(0);
    setSelectedFixtureAddressDraft(1);
    setSelectedFixtureGroupText("front");
    setSelectedFixtureLimitsDraft(defaultFixtureLimits);
    const controlStageEditFixtures = controlStageEditFixture
      ? structuredClone(viewportFixtureData.mappingLiveColorFixtures)
      : null;
    if (controlStageEditFixtures) {
      setLiveFixtures(controlStageEditFixtures);
    }
    setSnapshot((current) => ({
      ...current,
      fixtures: cueNodeGraphFixture
        ? []
        : liveEditTypeFixture || colorWheelFixture
          ? structuredClone(
              colorWheelFixture ? viewportFixtureData.colorWheelFixtures : viewportFixtureData.liveEditTypeFixtures,
            )
          : controlStageEditFixtures
            ? controlStageEditFixtures
            : sceneMatrixFixture
              ? [
                  { ...viewportPatchedFixture(1, "Front L", 1, -4, -2), group_ids: ["front"] },
                  { ...viewportPatchedFixture(2, "Front R", 9, 0, -2), group_ids: ["front"] },
                  { ...viewportPatchedFixture(3, "Back", 17, 4, -2), group_ids: ["back"] },
                ]
              : [
                  viewportPatchedFixture(1, "Video", 1, -4, -2),
                  viewportPatchedFixture(2, "Save", 9, 0, -2),
                  viewportPatchedFixture(3, "Output", 17, 4, -2),
                ],
      active_fade: cueNodeGraphFixture
        ? null
        : {
            cue_id: cueFixture || timelineFixture || sceneMatrixFixture ? activeCueId : 1,
            progress: 0.42,
            remaining_ms: 580,
            paused: false,
          },
      active_cue_id: cueFixture || timelineFixture || sceneMatrixFixture || touchComposedFixture
        ? activeCueId
        : current.active_cue_id,
      active_group_cue_ids: sceneMatrixFixture ? { front: activeCueId } : current.active_group_cue_ids,
      // T7: persisted identity colors under test in the scene-matrix fixture.
      group_colors: sceneMatrixFixture ? { back: "#22aa88" } : current.group_colors,
      cues: cueFixture || sceneMatrixFixture || touchComposedFixture
        ? fixtureCues
        : timelineFixture
          ? timelineFixtureCues
          : current.cues,
      cue_lists: cueFixture || timelineFixture || sceneMatrixFixture || touchComposedFixture
        ? [{ id: 1, label: "Main", active_cue_id: activeCueId }]
        : current.cue_lists,
      effects: cueFixture ? cueFixtureEffects : sceneMatrixFixture ? [viewportFixtureData.cueRecallEffect] : current.effects,
      node_graphs: cueFixture || cueNodeGraphFixture ? [viewportFixtureData.cueRecallNodeGraph] : current.node_graphs,
      submasters: [{
        group_id: "front",
        label: "front",
        level: 1,
        strobe_hz: 0,
        strobe_fixture_count: sceneMatrixFixture ? 2 : 3,
      }],
      video: cueNodeGraphFixture
        ? current.video
        : {
            ...current.video,
            layers: [viewportFixtureData.videoLayer],
            compositions: [viewportFixtureData.composition],
            outputs: [viewportFixtureData.videoOutput],
            mapping_presets: [{ label: "Viewport 16:9", mapping: viewportFixtureData.projectorMapping }],
          },
      stage_objects: [viewportFixtureData.stageObject],
      touch_surface: touchComposedFixture
        ? structuredClone(viewportFixtureData.touchSurface)
        : current.touch_surface,
      timeline: {
        ...current.timeline,
        layers: timelineLayeredFixture
          ? structuredClone(viewportFixtureData.layeredTimelineLayers)
          : current.timeline.layers,
        audio: timelineLayeredFixture
          ? structuredClone(viewportFixtureData.layeredTimelineAudioAnalysis)
          : current.timeline.audio,
        audio_clips: timelineLayeredFixture
          ? structuredClone(viewportFixtureData.layeredTimelineAudioClips)
          : current.timeline.audio_clips,
        audio_offset_ms: timelineLayeredFixture ? 0 : current.timeline.audio_offset_ms,
        audio_muted: timelineLayeredFixture ? false : current.timeline.audio_muted,
        playing: timelineFixture ? true : current.timeline.playing,
        duration_ms: sceneBlockHourFixture
          ? 3_600_000
          : sceneBlockLargeFixture
          ? Math.max(...timelineFixtureEvents.map(timelinePlacementDisplayEndMs))
          : timelineFixture ? 5000 : 4000,
        position_ms: sceneBlockHourFixture ? 1_800_000 : 1000,
        events: timelineFixture ? timelineFixtureEvents : current.timeline.events,
        automations: [
          {
            id: 1,
            fixture_id: 1,
            attribute: "Dimmer",
            track: "Lighting",
            enabled: true,
            keyframes: [
              { time_ms: 0, value: 65535, interpolation: "Linear" },
              { time_ms: sceneBlockHourFixture ? 3_600_000 : 4000, value: 0, interpolation: "Step" },
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
              { time_ms: sceneBlockHourFixture ? 3_600_000 : 4000, value: 0, interpolation: "Step" },
            ],
          },
        ],
      },
    }));
  } else if (viewportFixture === "audio-reactive") {
    const layer = structuredClone(viewportFixtureData.videoLayer);
    layer.id = 1;
    layer.label = "Reactive Program";
    layer.source = {
      kind: "File",
      path: "viewport://audio-reactive.mp4",
      name: layer.label,
      codec: "H264",
      metadata: { duration_ms: 4_000, width: 1_920, height: 1_080, frame_rate: 60, has_audio: true },
    };
    setWorkspaceTab("control");
    setControlMode("mixer");
    setLiveAudioInputStatus((current) => ({
      ...current,
      running: true,
      stale: false,
      safety_clear_pending: false,
      backend: "WASAPI",
      sample_rate: 48_000,
      channels: 2,
      bass: 0.64,
      mid: 0.38,
      high: 0.22,
      bands: [0.7, 0.64, 0.58, 0.5, 0.42, 0.36, 0.3, 0.25, 0.2, 0.17, 0.14, 0.12, 0.1, 0.08, 0.06, 0.04],
      band_count: 16,
      rms: 0.44,
      peak: 0.72,
      spectral_flux: 0.58,
      spectral_centroid: 0.35,
      spectral_density_fast: 0.55,
      spectral_density_slow: 0.32,
      kick_strength: 0.78,
      snare_strength: 0.41,
      kick_event: true,
      snare_event: false,
      onset: true,
      onset_strength: 0.67,
      bpm: 128,
      bpm_confidence: 0.86,
      beat_phase: 0.25,
      feature_sequence: 42,
    }));
    setLiveAudioInputStatusKnown(true);
    setSnapshot((current) => ({
      ...current,
      video: {
        ...current.video,
        layers: [layer],
        compositions: [{ ...viewportFixtureData.composition, layer_ids: [layer.id] }],
        outputs: [viewportFixtureData.videoOutput],
      },
      node_graphs: [{
        id: 301,
        label: "Kick → Program Opacity",
        enabled: true,
        nodes: [
          {
            id: 1,
            label: "Kick strength",
            kind: "Audio",
            x: 18,
            y: 26,
            lfo: null,
            position_wave: null,
            audio: {
              source: "Live",
              band: "Bass",
              feature: "KickStrength",
              band_index: 0,
              gain: 1.2,
              bias: 0,
              attack_ms: 20,
              release_ms: 180,
              gate: 0.08,
              curve: "Smoothstep",
              invert: false,
              hold_ms: 60,
            },
            transform: null,
            output: null,
          },
          {
            id: 2,
            label: "Scale",
            kind: "Transform",
            x: 50,
            y: 26,
            lfo: null,
            position_wave: null,
            audio: null,
            transform: { op: "Scale", amount: 1, min: 0, max: 1 },
            output: null,
          },
          {
            id: 3,
            label: "Program opacity",
            kind: "Output",
            x: 82,
            y: 26,
            lfo: null,
            position_wave: null,
            audio: null,
            transform: null,
            output: {
              fixture_ids: [],
              target_group_ids: [],
              attribute: "",
              video_targets: [{ layer_ids: [1], param: "Opacity", low: 0, high: 1, position: null }],
              low: 0,
              high: 65_535,
              blend_mode: "Override",
            },
          },
        ],
        edges: [
          { from_node: 1, from_port: "value", to_node: 2, to_port: "input" },
          { from_node: 2, from_port: "value", to_node: 3, to_port: "input" },
        ],
        audio_runtime: [{
          node_id: 1,
          input_value: 0.78,
          output_value: 0,
          source_available: false,
          safety_zeroed: true,
          held: false,
          feature_sequence: 42,
        }],
      }],
    }));
  } else if (viewportFixture === "operator-vj") {
    const layers = structuredClone(viewportFixtureData.operatorVjLayers);
    const composition = structuredClone(viewportFixtureData.operatorVjComposition);
    const outputs = structuredClone(viewportFixtureData.operatorVjOutputs);
    setWorkspaceTab("control");
    setControlMode("mixer");
    setSelectedVideoOutputId(outputs[0]?.id ?? null);
    setProjectHistoryStatus({
      can_undo: true,
      can_redo: true,
      undo_depth: 4,
      redo_depth: 2,
      undo_label: "Existing edit",
      redo_label: "Existing redo",
    });
    setVideoProgramAudioEnabled(true);
    setVideoAudioMonitorStatus({
      output_open: true,
      active_layer_ids: [layers[0]?.id ?? 1],
      resync_count: 1,
      last_drift_ms: 2,
      max_abs_drift_ms: 7,
      last_sync_error: null,
    });
    setVideoRecordingStatus({
      active: true,
      output_id: outputs[0]?.id ?? 1,
      path: "viewport://operator-vj/program-recording.mp4",
      width: 1_920,
      height: 1_080,
      frame_rate: 60,
      frames_written: 3_600,
      dropped_frames: 2,
      audio_requested: true,
      audio_included: true,
      audio_track_count: 1,
      started_unix_ms: 1_700_000_000_000,
      last_error: null,
    });
    setSnapshot((current) => ({
      ...current,
      video: {
        ...current.video,
        layers,
        compositions: [composition],
        outputs,
      },
    }));
  } else if (viewportFixture === "vj-bank") {
    const layers = structuredClone(viewportFixtureData.vjBankLayers);
    const composition = structuredClone(viewportFixtureData.vjBankComposition);
    const outputs = structuredClone(viewportFixtureData.vjBankOutputs);
    setWorkspaceTab("control");
    setControlMode("mixer");
    setSelectedVideoOutputId(outputs[0]?.id ?? null);
    setVideoProgramAudioEnabled(true);
    setSnapshot((current) => ({
      ...current,
      video: {
        ...current.video,
        layers,
        compositions: [composition],
        outputs,
      },
    }));
  } else if (viewportFixture === "auto-vj") {
    const labels = ["Video", "Output", "Signal Echo"];
    const layers = labels.map((layerLabel, index) => {
      const layer = structuredClone(viewportFixtureData.videoLayer);
      layer.id = index + 1;
      layer.label = layerLabel;
      layer.source = {
        kind: "File",
        path: `viewport://auto-vj-${index + 1}.mp4`,
        name: layerLabel,
        codec: "H264",
        metadata: {
          duration_ms: 4_000,
          width: 1_920,
          height: 1_080,
          frame_rate: 60,
          has_audio: index < 2,
        },
      };
      return layer;
    });
    setWorkspaceTab("control");
    setControlMode("mixer");
    setVideoProgramAudioEnabled(true);
    setVideoAudioMonitorStatus({
      output_open: true,
      active_layer_ids: [1],
      resync_count: 0,
      last_drift_ms: 0,
      max_abs_drift_ms: 0,
      last_sync_error: null,
    });
    setSnapshot((current) => ({
      ...current,
      video: {
        ...current.video,
        layers,
        compositions: [{
          ...viewportFixtureData.composition,
          layer_ids: layers.map((layer) => layer.id),
        }],
        outputs: [viewportFixtureData.videoOutput],
        auto_vj: defaultAutoVjSnapshot(),
      },
    }));
  } else if (viewportFixture === "mapping-live-color" || viewportFixture === "mapping-viewport-conformance") {
    const fixtures = structuredClone(viewportFixtureData.mappingLiveColorFixtures);
    const conformanceFixture = viewportFixture === "mapping-viewport-conformance";
    setWorkspaceTab("setup");
    setSetupSubTab("mapping");
    setControlMode("edit");
    setSelectedFixtureGroupFilter("front");
    setSelectedFixtureId(1);
    setSelectedMappingFixtureIds([1]);
    if (conformanceFixture) setSelectedStageObjectId(1);
    setLiveDmxPreviews([]);
    setLiveFixtures(fixtures);
    setSnapshot((current) => ({
      ...current,
      fixtures,
      dmx_preview: [],
      dmx_previews: [],
      stage_objects: conformanceFixture
        ? [{
            ...viewportFixtureData.stageObject,
            kind: "Stage",
            x: 0,
            z: 0,
            width: 2,
            depth: 2,
            rotation_deg: 90,
          }]
        : current.stage_objects,
      stage_map: conformanceFixture
        ? {
            ...current.stage_map,
            locked: true,
            min_x: -177.67,
            max_x: 177.67,
            min_z: -177.67,
            max_z: 177.67,
          }
        : current.stage_map,
    }));
  } else if (viewportFixture === "mapping-live-snapshot") {
    const fixtures = structuredClone(viewportFixtureData.mappingLiveSnapshotFixtures);
    const cue = structuredClone(viewportFixtureData.mappingLiveSnapshotAmberCue);
    const fixtureSnapshot: EngineSnapshot = {
      ...snapshot(),
      fixtures,
      cues: [cue],
      active_cue_id: null,
      active_group_cue_ids: {},
      dmx_preview: [],
      dmx_previews: [],
    };
    setWorkspaceTab("setup");
    setSetupSubTab("mapping");
    setControlMode("edit");
    setSelectedFixtureGroupFilter("front");
    setSelectedFixtureId(1);
    setSelectedMappingFixtureIds([1]);
    latestEngineSnapshot = fixtureSnapshot;
    setLiveDmxPreviews([]);
    setLiveFixtures(snapshotLiveFixtures(fixtureSnapshot));
    setSnapshot(fixtureSnapshot);
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
  const sceneBlockLargeSnapshotCloneTimer = viewportFixture === "scene-block-large"
    ? window.setInterval(() => {
        setSnapshot((current) => ({
          ...current,
          cues: current.cues.map((cue) => ({ ...cue })),
          timeline: {
            ...current.timeline,
            events: current.timeline.events.map((event) => ({ ...event })),
            automations: current.timeline.automations.map((automation) => ({
              ...automation,
              keyframes: automation.keyframes.map((keyframe) => ({ ...keyframe })),
            })),
            video_automations: current.timeline.video_automations.map((automation) => ({
              ...automation,
              keyframes: automation.keyframes.map((keyframe) => ({ ...keyframe })),
            })),
          },
        }));
      }, 250)
    : null;
  const sceneBlockFixtureWindow = window as Window & {
    __syndocalSetSceneBlockFixtureState?: (positionMs: number, playing: boolean) => void;
    __syndocalPauseSceneBlockFixtureChurn?: () => void;
    __syndocalReadAutoVjFixtureSnapshot?: () => EngineSnapshot;
    __syndocalReadOperatorVjFixtureSnapshot?: () => EngineSnapshot;
    __syndocalReadEditLiveFixtureSnapshot?: () => EngineSnapshot;
    __syndocalReadEditLiveFixtureHistory?: () => ProjectHistoryStatus;
    __syndocalReadControlStageEditFixtureSnapshot?: () => EngineSnapshot;
    __syndocalSelectMappingViewportStageObject?: () => void;
    __syndocalSetMappingLiveDmx?: (channelValues: Record<number, number>) => void;
    __syndocalCloneCueSnapshot?: () => void;
    __syndocalCloneFixtureSnapshot?: () => number;
    __syndocalSetControlFixtureSelection?: (
      fixtureIds: number[],
      selectedFixtureId: number | null,
      groupId: string,
    ) => void;
  };
  if (viewportFixture === "scene-block-large") {
    sceneBlockFixtureWindow.__syndocalSetSceneBlockFixtureState = (positionMs, playing) => {
      setSnapshot((current) => ({
        ...current,
        timeline: { ...current.timeline, position_ms: positionMs, playing },
      }));
    };
    sceneBlockFixtureWindow.__syndocalPauseSceneBlockFixtureChurn = () => {
      if (sceneBlockLargeSnapshotCloneTimer !== null) {
        window.clearInterval(sceneBlockLargeSnapshotCloneTimer);
      }
    };
  }
  if (viewportFixture === "auto-vj") {
    sceneBlockFixtureWindow.__syndocalReadAutoVjFixtureSnapshot = () => snapshot();
  }
  if (viewportFixture === "operator-vj") {
    sceneBlockFixtureWindow.__syndocalReadOperatorVjFixtureSnapshot = () => snapshot();
  }
  if (viewportFixture === "edit-live") {
    sceneBlockFixtureWindow.__syndocalReadEditLiveFixtureSnapshot = () => snapshot();
    sceneBlockFixtureWindow.__syndocalReadEditLiveFixtureHistory = () => projectHistoryStatus();
  }
  if (viewportFixture === "control-stage-edit" || viewportFixture === "mapping-viewport-conformance") {
    sceneBlockFixtureWindow.__syndocalReadControlStageEditFixtureSnapshot = () => snapshot();
  }
  if (viewportFixture === "mapping-viewport-conformance") {
    sceneBlockFixtureWindow.__syndocalSelectMappingViewportStageObject = () => setSelectedStageObjectId(1);
  }
  if (viewportFixture === "scene-matrix") {
    sceneBlockFixtureWindow.__syndocalCloneCueSnapshot = () => {
      setSnapshot((current) => ({
        ...current,
        cues: current.cues.map((cue) => ({
          ...cue,
          live_modifiers: cue.live_modifiers ? { ...cue.live_modifiers } : cue.live_modifiers,
          steps: cue.steps?.map((step) => ({ ...step })),
          effect_targets: cue.effect_targets.map((target) => ({ ...target })),
        })),
      }));
    };
  }
  let fixtureSnapshotCloneRevision = 0;
  if (viewportFixture === "live-edit-types" || viewportFixture === "edit-live") {
    sceneBlockFixtureWindow.__syndocalCloneFixtureSnapshot = () => {
      fixtureSnapshotCloneRevision += 1;
      setSnapshot((current) => {
        const fixtures = structuredClone(current.fixtures);
        const churnFixture = fixtures[fixtures.length - 1];
        if (churnFixture) churnFixture.highlighted = !churnFixture.highlighted;
        return { ...current, fixtures };
      });
      return fixtureSnapshotCloneRevision;
    };
  }
  if (viewportFixture === "mapping-live-color") {
    sceneBlockFixtureWindow.__syndocalSetMappingLiveDmx = (channelValues) => {
      const currentValues = liveDmxPreviews().find((preview) => preview.universe === 0)?.values ?? [];
      const values = Array.from({ length: 512 }, (_, index) => currentValues[index] ?? 0);
      for (const [channelText, value] of Object.entries(channelValues)) {
        const channel = Number(channelText);
        if (Number.isInteger(channel) && channel >= 1 && channel <= 512) {
          values[channel - 1] = Math.max(0, Math.min(255, Math.round(value)));
        }
      }
      const previews = [{ universe: 0, values }];
      setLiveDmxPreviews(previews);
      setSnapshot((current) => ({ ...current, dmx_preview: values, dmx_previews: previews }));
    };
  }
  if (
    viewportFixture === "patch"
    || viewportFixture === "edit-live"
    || viewportFixture === "live-edit-types"
    || viewportFixture === "mapping-live-color"
    || viewportFixture === "workspace-operator"
  ) {
    sceneBlockFixtureWindow.__syndocalSetControlFixtureSelection = (
      fixtureIds,
      selectedFixture,
      groupId,
    ) => {
      setSelectedMappingFixtureIds(fixtureIds);
      setSelectedFixtureId(selectedFixture);
      setSelectedFixtureGroupFilter(groupId);
    };
  }
  // Viewport fixtures seed their own primary workspace. Pane windows remain
  // authoritative and must re-select the surface they were opened to host.
  if (paneWindow) {
    if (paneWindow === "setup") {
      setWorkspaceTab("setup");
    } else if (paneWindow === "touch") {
      setWorkspaceTab("touch");
    } else {
      setWorkspaceTab("control");
      setControlMode(paneWindow === "programmer" ? "edit" : paneWindow === "mixer" ? "mixer" : "live");
    }
  }
  onCleanup(() => {
    if (sceneBlockLargeSnapshotCloneTimer !== null) {
      window.clearInterval(sceneBlockLargeSnapshotCloneTimer);
    }
    delete sceneBlockFixtureWindow.__syndocalSetSceneBlockFixtureState;
    delete sceneBlockFixtureWindow.__syndocalPauseSceneBlockFixtureChurn;
    delete sceneBlockFixtureWindow.__syndocalReadAutoVjFixtureSnapshot;
    delete sceneBlockFixtureWindow.__syndocalReadOperatorVjFixtureSnapshot;
    delete sceneBlockFixtureWindow.__syndocalReadEditLiveFixtureSnapshot;
    delete sceneBlockFixtureWindow.__syndocalReadEditLiveFixtureHistory;
    delete sceneBlockFixtureWindow.__syndocalReadControlStageEditFixtureSnapshot;
    delete sceneBlockFixtureWindow.__syndocalSelectMappingViewportStageObject;
    delete sceneBlockFixtureWindow.__syndocalSetMappingLiveDmx;
    delete sceneBlockFixtureWindow.__syndocalCloneCueSnapshot;
    delete sceneBlockFixtureWindow.__syndocalCloneFixtureSnapshot;
    delete sceneBlockFixtureWindow.__syndocalSetControlFixtureSelection;
  });
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
    if (paneWindow) return;
    saveWorkspaceLayout({
      workspace_tab: workspaceTab(),
      setup_sub_tab: setupSubTab(),
      control_mode: controlMode(),
      timeline_desk_surface: timelineDeskSurface(),
      timeline_context_drawer: timelineContextDrawer() === "block" ? "none" : timelineContextDrawer(),
      edit_desk_surface: editDeskSurface(),
      control_category: controlCategory(),
      top_split_ratio: topSplitRatio(),
      lower_split_ratio: lowerSplitRatio(),
      selections_drawer_open: selectionsDrawerOpen(),
    });
  });
  const [appStatus, setAppStatus] = createSignal(appStatusFromMessage("Ready"));
  const [daslightProjectImportBusy, setDaslightProjectImportBusy] = createSignal(false);
  const message = () => appStatus().text;
  const setMessage = (text: string, key?: string) => {
    setAppStatus(appStatusFromMessage(text, key));
    return text;
  };
  const currentWorkspaceLayout = (): WorkspaceLayout => ({
    workspace_tab: workspaceTab(),
    setup_sub_tab: setupSubTab(),
    control_mode: controlMode(),
    timeline_desk_surface: timelineDeskSurface(),
    timeline_context_drawer: timelineContextDrawer() === "block" ? "none" : timelineContextDrawer(),
    edit_desk_surface: editDeskSurface(),
    control_category: controlCategory(),
    top_split_ratio: topSplitRatio(),
    lower_split_ratio: lowerSplitRatio(),
    selections_drawer_open: selectionsDrawerOpen(),
  });
  const applyWorkspaceLayout = (layout: WorkspaceLayout) => {
    setWorkspaceTab(layout.workspace_tab);
    setSetupSubTab(layout.setup_sub_tab);
    setControlMode(layout.control_mode);
    setTimelineDeskSurface(layout.timeline_desk_surface);
    setTimelineContextDrawer(layout.timeline_context_drawer);
    setEditDeskSurface(layout.edit_desk_surface);
    setControlCategory(layout.control_category);
    setTopSplitRatio(layout.top_split_ratio);
    setLowerSplitRatio(layout.lower_split_ratio);
    setSelectionsDrawerOpen(layout.selections_drawer_open);
  };

  const operatorLockSessionStorageKey = "syndocal.operatorLockSession.v1";
  const matchingStoredOperatorLock = (policy: OperatorPolicy | null): OperatorLockMode | null => {
    if (!policy) return null;
    try {
      const raw = window.localStorage.getItem(operatorLockSessionStorageKey);
      if (!raw) return null;
      const value = JSON.parse(raw) as Record<string, unknown>;
      return value.verifier === policy.credential.verifier_b64 &&
        (value.mode === "Full" || value.mode === "Partial")
        ? value.mode
        : null;
    } catch {
      return null;
    }
  };
  const setOperatorSessionLock = (mode: OperatorLockMode | null) => {
    const policy = operatorPolicy();
    if (mode !== null && !policy) {
      setMessage("Configure an operator policy before locking the show.");
      return;
    }
    setOperatorLockMode(mode);
  };
  const refreshOperatorPolicy = async (projectBoundary = false) => {
    if (!isTauriRuntime()) {
      if (viewportFixture === "workspace-operator") {
        const policy: OperatorPolicy = {
          lock_mode: "Partial",
          lock_on_load: true,
          credential: {
            scheme: "PBKDF2-SHA256",
            iterations: 600_000,
            salt_b64: "MTExMTExMTExMTExMTExMQ==",
            verifier_b64: "p6enp6enp6enp6enp6enp6enp6enp6enp6enp6enp6c=",
          },
        };
        setOperatorPolicy(policy);
        const fixtureLock = new URLSearchParams(window.location.search).get("syndocalOperatorLock");
        setOperatorLockMode(fixtureLock === "full" ? "Full" : fixtureLock === "partial" ? "Partial" : null);
      } else {
        setOperatorPolicy(null);
        setOperatorLockMode(null);
      }
      setOperatorPolicyReady(true);
      return operatorPolicy();
    }
    try {
      const raw = await tauriInvoke<unknown>("get_operator_policy");
      const policy = raw === null ? null : operatorPolicyFromUnknown(raw);
      if (raw !== null && !policy) throw new Error("Project operator policy failed frontend validation.");
      setOperatorPolicy(policy);
      setOperatorPolicyReady(true);
      const storedMode = matchingStoredOperatorLock(policy);
      if (storedMode) {
        setOperatorLockMode(storedMode);
      } else if (projectBoundary) {
        setOperatorLockMode(policy?.lock_on_load ? policy.lock_mode : null);
      }
      return policy;
    } catch (error) {
      setOperatorPolicy(null);
      setOperatorLockMode(null);
      setOperatorPolicyReady(true);
      setMessage(`Operator policy unavailable: ${String(error)}`);
      return null;
    }
  };
  createEffect(() => {
    if (!operatorPolicyReady()) return;
    const mode = operatorLockMode();
    const policy = operatorPolicy();
    activeOperatorLockMode = mode;
    try {
      if (mode && policy) {
        window.localStorage.setItem(operatorLockSessionStorageKey, JSON.stringify({
          mode,
          verifier: policy.credential.verifier_b64,
        }));
      } else {
        window.localStorage.removeItem(operatorLockSessionStorageKey);
      }
    } catch {
      // The lock remains active in this WebView if storage synchronization is unavailable.
    }
  });
  const handleOperatorLockStorage = (event: StorageEvent) => {
    if (event.key !== operatorLockSessionStorageKey) return;
    setOperatorLockMode(matchingStoredOperatorLock(operatorPolicy()));
  };
  window.addEventListener("storage", handleOperatorLockStorage);
  onCleanup(() => window.removeEventListener("storage", handleOperatorLockStorage));
  void refreshOperatorPolicy(true);
  createEffect(() => {
    if (operatorLockMode() !== "Partial") return;
    if (workspaceTab() !== "control" || controlMode() === "edit") {
      setWorkspaceTab("control");
      setControlMode("live");
    }
  });
  const resetWorkspaceLayout = () => {
    applyWorkspaceLayout(defaultWorkspaceLayout);
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
  if (
    viewportFixture === "timeline"
    || viewportFixture === "cue-recall"
    || viewportFixture === "cue-recall-large"
    || viewportFixture === "cue-node-graph"
  ) {
    setSerialPorts([
      {
        name: "COM9",
        port_type: "USB 0403:6001 ENTTEC DMX USB Pro",
        usb_vid: 0x0403,
        usb_pid: 0x6001,
        serial_number: "VIEWPORT-DMX-PRO",
        manufacturer: "ENTTEC",
        product: "DMX USB Pro",
        recommended_protocol: "EnttecUsbPro",
      },
    ]);
    setDmxOutputRoutes(
      Array.from({ length: 128 }, (_, universe) => ({
        ...defaultOutput,
        universe,
      })),
    );
  }
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
  const audioAnalysis = createMemo<AudioAnalysisSummary | null>(() => activeTimeline().audio ?? null);
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
  const sceneMatrixGroupIds = createMemo(() => {
    // Cue group IDs are scene banks; fixture group IDs describe patch selections.
    const groupIds: string[] = [];
    const seen = new Set<string>();
    for (const cue of snapshot().cues) {
      const groupId = cue.group_id?.trim();
      if (!groupId || seen.has(groupId)) continue;
      seen.add(groupId);
      groupIds.push(groupId);
    }
    return groupIds;
  });
  const filteredFixtures = createMemo(() => {
    const groupId = selectedFixtureGroupFilter();
    if (!groupId) {
      return snapshot().fixtures;
    }
    return snapshot().fixtures.filter((fixture) =>
      fixture.group_ids.some((fixtureGroup) => groupMatches(fixtureGroup, groupId)));
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
  const mappingFixtureById = createMemo(() =>
    new Map(snapshot().fixtures.map((fixture) => [fixture.id, fixture])),
  );
  const orderedMappingSelectionFixtures = createMemo(() => {
    const fixturesById = mappingFixtureById();
    return selectedMappingFixtureIds().flatMap((fixtureId) => {
      const fixture = fixturesById.get(fixtureId);
      return fixture ? [fixture] : [];
    });
  });
  const pickedFixtureTypeGroups = createMemo(() =>
    groupPickedFixturesByType(orderedMappingSelectionFixtures()),
  );
  const hasMultiplePickedFixtures = createMemo(() =>
    orderedMappingSelectionFixtures().length > 1,
  );
  const showFixtureTypeAttributeColumns = createMemo(() =>
    !selectedFixtureGroupFilter() && hasMultiplePickedFixtures(),
  );
  const mappingEffectOrderFixtures = createMemo(() => {
    if (effectTargetMode() === "selection") return orderedMappingSelectionFixtures();
    if (effectTargetMode() === "fixture") {
      const fixture = selectedFixture();
      return fixture ? [fixture] : [];
    }
    if (effectTargetMode() === "group") {
      const groupIds = new Set(parseGroupIds(effectTargetGroups()));
      return snapshot().fixtures.filter((fixture) => fixture.group_ids.some((groupId) => groupIds.has(groupId)));
    }
    return [];
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
  const selectedGroupStrobeFixtureCount = createMemo(() =>
    selectedGroupSubmaster()?.strobe_fixture_count
      ?? groupStrobeCompatibleFixtureCount(selectedGroupFixtures()));

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
    if (showFixtureTypeAttributeColumns()) {
      return orderedMappingSelectionFixtures();
    }
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
  const fixtureControlIsWritten = (fixture: PatchedFixtureSummary, control: AttributeControl) =>
    faderValue(fixture.id, control.attribute, control.default_value) !== control.default_value;
  const controlIsWritten = (control: AttributeControl) =>
    selectedControlTargetFixtures().some((fixture) => {
      const fixtureControl = fixtureControlForAttribute(fixture, control.attribute);
      return fixtureControl ? fixtureControlIsWritten(fixture, fixtureControl) : false;
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
    if (showFixtureTypeAttributeColumns()) {
      for (const group of pickedFixtureTypeGroups()) {
        const controls = commonFixtureTypeControls(group);
        counts.set("fader", (counts.get("fader") ?? 0) + controls.length);
        for (const control of controls) {
          const category = controlCategoryForAttribute(control.attribute);
          counts.set(category, (counts.get(category) ?? 0) + 1);
        }
      }
      return counts;
    }
    const controls = activeControls();
    counts.set("fader", controls.length);
    for (const control of controls) {
      const category = controlCategoryForAttribute(control.attribute);
      counts.set(category, (counts.get(category) ?? 0) + 1);
    }
    return counts;
  });
  const categoryHasVisualControl = (category: ControlCategory) => {
    if (showFixtureTypeAttributeColumns()) {
      return (controlCategoryCounts().get(category) ?? 0) > 0;
    }
    return (
      (category === "dimmer" && Boolean(selectedDimmerControl())) ||
      (category === "position" && Boolean(selectedPositionControls())) ||
      (category === "color" && Boolean(selectedColorControls())) ||
      category === "fader"
    );
  };
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
      hasWritten: selectedControlTargetFixtures().some((fixture) =>
        fixture.controls
          .filter((control) =>
            category.id === "fader" || controlCategoryForAttribute(control.attribute) === category.id
          )
          .some((control) => fixtureControlIsWritten(fixture, control))
      ),
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
    !["dimmer", "color", "position", "fader"].includes(activeControlCategory()),
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
  const controlTargetLabel = createMemo(() => {
    if (showFixtureTypeAttributeColumns()) {
      return `${pickedFixtureTypeGroups().length} fixture types`;
    }
    return selectedFixtureGroupFilter()
      ? `Group ${selectedFixtureGroupFilter()}`
      : selectedFixture()?.label ?? "No fixture selected";
  });
  const controlTargetKind = createMemo<"fixture" | "group" | "selection" | "empty">(() =>
    showFixtureTypeAttributeColumns()
      ? "selection"
      : selectedFixtureGroupFilter()
        ? "group"
        : selectedFixture()
          ? "fixture"
          : "empty",
  );
  const controlCompactReadout = createMemo(() => {
    const targetCount = selectedControlTargetFixtures().length;
    const attributeCount = activeControls().length;
    const reference = selectedControlReferenceFixture();
    const range = `${targetCount} fixture${targetCount === 1 ? "" : "s"}/${attributeCount} attr${attributeCount === 1 ? "" : "s"}`;
    return `${range} · ${reference ? `U${reference.universe} A${reference.address}` : "No readout"}`;
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
    const fixture = selectedControlReferenceFixture();
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
    const swatch = channelFunctionWheelColor(control, fn);
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
    const swatch = channelFunctionWheelColor(control, fn);
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
  const colorWheelPickerTargetKey = () => {
    const groupId = selectedFixtureGroupFilter();
    return groupId ? `group:${groupId}` : `fixture:${selectedControlReferenceFixture()?.id ?? "none"}`;
  };
  const colorWheelEntryKey = (entry: ColorWheelFunctionEntry) =>
    `${entry.control.attribute}\u0000${entry.fn.dmx_from}\u0000${entry.fn.dmx_to}`;
  const colorWheelPickerResolution = createMemo(() => {
    const entries = colorWheelEntries();
    const selection = colorWheelPickerSelection();
    if (selection?.targetKey === colorWheelPickerTargetKey()) {
      const entry =
        entries.find((candidate) => colorWheelEntryKey(candidate) === selection.entryKey) ??
        nearestColorWheelEntry(entries, selection.color);
      if (entry) {
        return { color: selection.color, entry };
      }
    }
    const entry = entries.find((candidate) => candidate.active) ?? entries[0];
    return {
      color: entry?.color ?? "#ffffff",
      entry,
    };
  });
  const colorWheelPickerColor = createMemo(() => {
    return colorWheelPickerResolution().color;
  });
  const colorWheelPickerHsv = createMemo(() => {
    const color = colorWheelPickerColor();
    return rgbToHsv(
      Number.parseInt(color.slice(1, 3), 16),
      Number.parseInt(color.slice(3, 5), 16),
      Number.parseInt(color.slice(5, 7), 16),
    );
  });
  const colorWheelApproximationLabel = createMemo(() => {
    const entry = colorWheelPickerResolution().entry;
    return entry ? `→ ${entry.label} (${channelFunctionRangeLabel(entry.fn)})` : "";
  });
  const colorWheelPreviewForSaturation = (saturation: number) => {
    const hsv = colorWheelPickerHsv();
    const { red, green, blue } = hsvToRgb(hsv.hue, clamp01(saturation), Math.max(0.05, hsv.value));
    return rgbToHex(red, green, blue);
  };
  const colorWheelSaturationRamp = createMemo(() =>
    `linear-gradient(to right, ${colorWheelPreviewForSaturation(0)}, ${colorWheelPreviewForSaturation(0.35)}, ${colorWheelPreviewForSaturation(1)})`,
  );
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
  const showColorWheelPickerPanel = createMemo(() => showColorWheelPanel() && !selectedColorControls());
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
        return snapshot().fixtures.filter((fixture) =>
          groups.some((groupId) => fixture.group_ids.some((candidate) => groupMatches(candidate, groupId)))
        );
      }
      case "video":
      default:
        return [];
    }
  });
  const effectTargetControls = createMemo<AttributeControl[]>(() => commonAttributeControls(effectTargetFixtures()));
  const chaserAttributeOptions = createMemo(() => {
    const attributes: string[] = [];
    const seen = new Set<string>();
    for (const fixture of effectTargetFixtures()) {
      for (const control of fixture.controls) {
        const canonical = canonicalChaserAttribute(control.attribute);
        if (canonical && !seen.has(canonical)) {
          seen.add(canonical);
          attributes.push(control.attribute);
        }
      }
    }
    return attributes;
  });
  const chaserAttributeCoverage = createMemo<Record<string, string>>(() => {
    const fixtures = effectTargetFixtures();
    return Object.fromEntries(chaserAttributeOptions().map((attribute) => {
      const canonical = canonicalChaserAttribute(attribute);
      const compatible = fixtures.filter((fixture) =>
        fixture.controls.some((control) => canonicalChaserAttribute(control.attribute) === canonical)
      ).length;
      return [canonical, `${compatible}/${fixtures.length} fixtures`];
    }));
  });
  const selectedEffectAttribute = createMemo(() => {
    const controls = effectTargetControls();
    const current = effectAttribute();
    if (controls.some((control) => control.attribute === current)) {
      return current;
    }
    return controls[0]?.attribute ?? "";
  });
  const selectedChaserAttribute = createMemo(() => {
    const attributes = chaserAttributeOptions();
    const current = canonicalChaserAttribute(effectAttribute());
    return attributes.find((attribute) => canonicalChaserAttribute(attribute) === current) ?? attributes[0] ?? "";
  });
  const moveCompatibleTargetFixtures = createMemo(() => effectTargetFixtures().filter((fixture) => {
    const axes = fixture.controls.reduce((counts, control) => {
      const normalized = control.attribute.toLowerCase().replace(/[^a-z0-9]/g, "");
      if (normalized.includes("tilt")) counts.tilt += 1;
      else if (normalized.includes("pan")) counts.pan += 1;
      return counts;
    }, { pan: 0, tilt: 0 });
    return axes.pan === 1 && axes.tilt === 1;
  }));
  const chaserCurrentTargetSteps = createMemo(() =>
    chaserStepsFromTargets(
      effectTargetMode() === "video" ? [] : effectTargetFixtures().map((fixture) => fixture.id),
      [],
      false,
    ),
  );
  const prepareChaserDraftFromCurrentTarget = (forceReset = false) => {
    const fixtureIds = effectTargetMode() === "video"
      ? selectedFixture() ? [selectedFixture()!.id] : []
      : effectTargetFixtures().map((fixture) => fixture.id);
    const nextSteps = chaserStepsFromTargets(fixtureIds, [], true);
    const replacesSteps = forceReset || chaserSteps().length === 0;
    if (replacesSteps) {
      setChaserSteps(nextSteps);
    }
    if (forceReset || chaserFeatures().length === 0 || chaserFeatures()[0]?.attribute === "Dimmer") {
      const attribute = selectedChaserAttribute() || chaserFeatures()[0]?.attribute || "Dimmer";
      setChaserFeatures([{ attribute, low: Math.round(effectLow()), high: Math.round(effectHigh()) }]);
    }
    const stepCount = replacesSteps ? nextSteps.length : chaserSteps().length;
    const maxActiveSteps = Math.max(1, Math.min(stepCount || fixtureIds.length, 64));
    const maxWings = Math.max(1, Math.min(stepCount || fixtureIds.length, 16));
    setChaserActiveStepCount((current) => forceReset ? 1 : Math.max(1, Math.min(current, maxActiveSteps)));
    setChaserWings((current) => forceReset ? 1 : Math.max(1, Math.min(current, maxWings)));
  };
  const applyMovePathRecipe = (recipe: MovePathPreset) => {
    setMovePathRecipe(recipe);
    setMovePathPoints(movePathRecipePoints(recipe));
    setMovePathClosed(recipe !== "Line");
    setMoveInterpolation(recipe === "Line" || recipe === "Triangle" || recipe === "Square" ? "Line" : "Smooth");
  };
  const prepareMoveDraft = (forceReset = false) => {
    setEffectVideoTargetLinked(false);
    if (effectTargetMode() === "video") setEffectTargetMode(selectedFixture() ? "fixture" : "selection");
    setEffectBlendMode("Override");
    if (forceReset) {
      applyMovePathRecipe("Circle");
      setMoveCoordinateMode("Absolute");
      setMoveCenterX(0.5);
      setMoveCenterY(0.5);
      setMoveSizeX(1);
      setMoveSizeY(1);
      setMoveRotationDegrees(0);
      setMoveDirection("Forward");
      setMoveFixtureSpread(0);
    }
  };
  const chooserFamilyForEffectType = (nextType: EffectKind): EffectRecipeFamily => {
    if (nextType === "Color") return "COLOR FX";
    if (nextType === "Chaser") return "CHASER FX";
    if (nextType === "Move") return "MOVE FX";
    if (nextType === "Value") return "VALUE FX";
    if (nextType === "Curve") return "CURVE FX";
    if (nextType === "Mapping") return "MAPPINGS";
    if (nextType === "ColorMapping") return "COLOUR MAPPINGS";
    if (nextType === "PositionWave") return "MAPPINGS";
    return "CURVE FX";
  };
  const selectEffectType = (nextType: EffectKind, preserveChooserFamily = false) => {
    const previousType = effectType();
    const sourceEffectId = editingEffectId();
    const startsNewEffect = sourceEffectId !== null && nextType !== previousType;
    if (startsNewEffect) {
      setEditingEffectId(null);
      setMessage(`Effect ${sourceEffectId} remains unchanged; Type change starts a new ${nextType} effect.`);
    }
    setEffectType(nextType);
    if (!preserveChooserFamily) {
      const family = chooserFamilyForEffectType(nextType);
      setEffectChooserFamily(family);
    }
    if (nextType === "Color") {
      setEffectVideoTargetLinked(false);
      if (effectTargetMode() === "video") setEffectTargetMode("fixture");
      if (!startsNewEffect) setMessage("Prepared a multi-color draft for whole-fixture colour output.");
    } else if (nextType === "Chaser") {
      setEffectVideoTargetLinked(false);
      if (effectTargetMode() === "video") setEffectTargetMode("fixture");
      prepareChaserDraftFromCurrentTarget(nextType !== previousType);
      if (!startsNewEffect) setMessage("Prepared an ordered fixture-index Chaser draft from the current target.");
    } else if (nextType === "Move") {
      prepareMoveDraft(nextType !== previousType);
      if (!startsNewEffect) setMessage("Prepared an independent paired Pan/Tilt Move path from the current target.");
    } else if (nextType === "Value") {
      setEffectVideoTargetLinked(false);
      if (effectTargetMode() === "video") setEffectTargetMode("fixture");
      if (!startsNewEffect) setMessage("Prepared an envelope Value draft for the current scalar attribute.");
    } else if (nextType === "Curve") {
      setEffectVideoTargetLinked(false);
      if (effectTargetMode() === "video") setEffectTargetMode("fixture");
      if (!startsNewEffect) setMessage("Prepared an independent cubic Curve draft for the current scalar attribute.");
    } else if (nextType === "Mapping") {
      setEffectVideoTargetLinked(false);
      if (effectTargetMode() === "video") setEffectTargetMode("fixture");
      if (nextType !== previousType) {
        setMappingMode("Absolute");
        setMappingDirection("Forward");
        setMappingFixtureSpread(1);
        setMappingRepetitions(1);
      }
      if (!startsNewEffect) setMessage("Prepared a fixture-order Mapping draft for the current scalar attribute.");
    } else if (nextType === "ColorMapping") {
      setEffectVideoTargetLinked(false);
      if (effectTargetMode() === "video") setEffectTargetMode("fixture");
      if (nextType !== previousType) {
        const raster = defaultColorMappingRaster();
        setColorMappingSourceKind("Image");
        setColorMappingWidth(raster.width);
        setColorMappingHeight(raster.height);
        setColorMappingFrames(raster.frames);
        setColorMappingCells([]);
        setColorMappingPlaybackDirection("Forward");
        setColorMappingOffsetU(0);
        setColorMappingOffsetV(0);
        setColorMappingScaleU(1);
        setColorMappingScaleV(1);
        setColorMappingRotationDegrees(0);
        setColorMappingWrapMode("Clamp");
        setColorMappingSampling("Nearest");
      }
      if (!startsNewEffect) setMessage("Prepared an independent 2D Colour Mapping draft for lighting fixtures.");
    }
  };
  const effectTargetSummary = createMemo(() => {
    const fixtureCount = effectTargetFixtures().length;
    const attributeCount = effectType() === "Chaser" ? chaserAttributeOptions().length : effectTargetControls().length;
    const wholeFixtureColor = effectType() === "Color" || effectType() === "ColorMapping";
    const moveEffect = effectType() === "Move";
    const moveCompatibleCount = moveCompatibleTargetFixtures().length;
    const moveCompatibility = `${moveCompatibleCount}/${fixtureCount} paired Pan/Tilt`;
    switch (effectTargetMode()) {
      case "selection":
        if (moveEffect) {
          return `${fixtureCount} mapped fixture${fixtureCount === 1 ? "" : "s"} / ${moveCompatibility}`;
        }
        if (wholeFixtureColor) {
          return `${fixtureCount} mapped fixture${fixtureCount === 1 ? "" : "s"} / whole-fixture colour`;
        }
        return `${fixtureCount} mapped fixture${fixtureCount === 1 ? "" : "s"} / ${attributeCount} common attribute${attributeCount === 1 ? "" : "s"}`;
      case "group": {
        const groups = parseGroupIds(effectTargetGroups());
        return groups.length > 0
          ? moveEffect
            ? `${groups.join(", ")} / ${moveCompatibility}`
            : wholeFixtureColor
            ? `${groups.join(", ")} / ${fixtureCount} fixture${fixtureCount === 1 ? "" : "s"} / whole-fixture colour`
            : `${groups.join(", ")} / ${fixtureCount} fixture${fixtureCount === 1 ? "" : "s"} / ${attributeCount} common attribute${attributeCount === 1 ? "" : "s"}`
          : "No group target";
      }
      case "video":
        return selectedEffectVideoLayerId() === null
          ? "No video layer target"
          : `Video Layer ${selectedEffectVideoLayerId()} / ${effectVideoParam()} ${Math.round(effectVideoLow() * 100)}-${Math.round(effectVideoHigh() * 100)}%`;
      case "fixture":
      default:
        return selectedFixture()
          ? `${selectedFixture()!.label}${wholeFixtureColor ? " / whole-fixture colour" : moveEffect ? ` / ${moveCompatibility}` : ""}`
          : "No fixture selected";
    }
  });
  const effectDraftSummary = createMemo(() => {
    if (effectType() === "PositionWave") {
      return `Wave O ${waveOriginX().toFixed(1)},${waveOriginY().toFixed(1)},${waveOriginZ().toFixed(1)} / D ${waveDirectionX().toFixed(1)},${waveDirectionY().toFixed(1)},${waveDirectionZ().toFixed(1)} / ${waveWavelength().toFixed(1)}m`;
    }
    const sync = effectClockSyncBeats();
    if (effectType() === "Color") {
      const clock = sync === null ? `${effectPeriod()}ms` : `${sync} beat`;
      return `Color ${colorEffectAlgorithm()} / ${colorEffectStops().length} stops / ${clock}`;
    }
    if (effectType() === "Chaser") {
      const clock = sync === null ? `${chaserStepDuration()}ms` : `${sync} beat`;
      return `Chaser ${chaserSteps().length} steps / ${chaserFeatures().length} features / ${chaserDirection()} / ${chaserActiveStepCount()} pixels on / ${clock}`;
    }
    if (effectType() === "Move") {
      const clock = sync === null ? `${effectPeriod()}ms` : `${sync} beat`;
      return `Move ${movePathPoints().length} points / ${moveInterpolation()} / ${moveDirection()} / ${clock}`;
    }
    if (effectType() === "Value") {
      const clock = sync === null ? `${effectPeriod()}ms` : `${sync} beat`;
      return `Value ${valuePoints().length} points / ${valueInterpolation()} / ${valueMode()} / ${valueDirection()} / ${clock}`;
    }
    if (effectType() === "Curve") {
      const clock = sync === null ? `${effectPeriod()}ms` : `${sync} beat`;
      return `Curve ${curvePoints().length} points / Cubic / ${curveMode()} / ${curveDirection()} / ${clock}`;
    }
    if (effectType() === "Mapping") {
      const clock = sync === null ? `${effectPeriod()}ms` : `${sync} beat`;
      return `Mapping ${mappingEffectOrderFixtures().length} fixtures / ${effectShape()} / ${mappingDirection()} / ${mappingRepetitions().toFixed(2)}× / ${clock}`;
    }
    if (effectType() === "ColorMapping") {
      const clock = sync === null ? `${effectPeriod()}ms` : `${sync} beat`;
      return `Colour Mapping ${colorMappingWidth()}×${colorMappingHeight()} / ${colorMappingFrames().length} frames / ${colorMappingCells().length || effectTargetFixtures().length} cells / ${clock}`;
    }
    return sync === null ? `LFO ${effectShape()} / ${effectPeriod()}ms` : `LFO ${effectShape()} / ${sync} beat`;
  });
  const currentValueDraftError = createMemo<string>(() => {
    const points = valuePoints();
    if (points.length < 2 || points.length > 32) {
      return "Value effect requires between 2 and 32 envelope points.";
    }
    let previous: number | null = null;
    for (const point of points) {
      if (!Number.isFinite(point.position) || point.position < 0 || point.position > 1) {
        return "Value envelope positions must be within 0..1.";
      }
      if (!Number.isFinite(point.value) || point.value < 0 || point.value > 1) {
        return "Value envelope values must be within 0..1.";
      }
      if (previous !== null && point.position <= previous) {
        return "Value envelope positions must be strictly increasing.";
      }
      previous = point.position;
    }
    return "";
  });
  const currentCurveDraftError = createMemo<string>(() => {
    const points = curvePoints();
    if (points.length < 2 || points.length > 32) return "Curve effect requires between 2 and 32 points.";
    let previous: number | null = null;
    for (const point of points) {
      if (!Number.isFinite(point.position) || point.position < 0 || point.position > 1) return "Curve positions must be within 0..1.";
      if (!Number.isFinite(point.value) || point.value < 0 || point.value > 1) return "Curve values must be within 0..1.";
      if (!Number.isFinite(point.in_tangent) || !Number.isFinite(point.out_tangent) || Math.abs(point.in_tangent) > 32 || Math.abs(point.out_tangent) > 32) return "Curve tangents must be finite and within -32..32.";
      if (previous !== null && point.position <= previous) return "Curve positions must be strictly increasing.";
      previous = point.position;
    }
    return "";
  });
  const currentMappingDraftError = createMemo<string>(() => {
    if (!Number.isFinite(effectPeriod()) || effectPeriod() < 10) return "Mapping period must be at least 10 ms.";
    const clockSyncBeats = effectClockSyncBeats();
    if (clockSyncBeats !== null && (!Number.isFinite(clockSyncBeats) || clockSyncBeats <= 0)) return "Mapping clock beats must be positive.";
    if (!Number.isFinite(effectPhase()) || effectPhase() < 0 || effectPhase() > 1) return "Mapping phase must be within 0..1.";
    if (!Number.isFinite(mappingFixtureSpread()) || mappingFixtureSpread() < 0 || mappingFixtureSpread() > 1) return "Mapping fixture spread must be within 0..1.";
    if (!Number.isFinite(mappingRepetitions()) || mappingRepetitions() < 0.25 || mappingRepetitions() > 16) return "Mapping repetitions must be within 0.25..16.";
    return "";
  });
  const currentColorMappingDraftError = createMemo<string>(() => {
    const width = colorMappingWidth();
    const height = colorMappingHeight();
    const frames = colorMappingFrames();
    if (!Number.isInteger(width) || !Number.isInteger(height) || width < 1 || width > 64 || height < 1 || height > 64) {
      return "Colour Mapping raster dimensions must each be within 1..64.";
    }
    if (frames.length < 1 || frames.length > 64) return "Colour Mapping requires between 1 and 64 frames.";
    if (colorMappingSourceKind() === "Video" && frames.length < 2) return "Colour Mapping video requires at least 2 frames.";
    if (colorMappingSourceKind() !== "Video" && frames.length !== 1) return "Image and text mappings require exactly 1 frame.";
    const expectedPixels = width * height;
    if (frames.some((frame) => frame.pixels.length !== expectedPixels)) return `Every Colour Mapping frame must contain ${expectedPixels} pixels.`;
    if (frames.some((frame) => frame.pixels.some((pixel) => !Number.isSafeInteger(pixel) || pixel < 0 || pixel > 281_474_976_710_655))) {
      return "Colour Mapping pixels must be exact packed RGB16 integers.";
    }
    if (!Number.isFinite(effectPeriod()) || effectPeriod() < 10) return "Colour Mapping period must be at least 10 ms.";
    const beats = effectClockSyncBeats();
    if (beats !== null && (!Number.isFinite(beats) || beats <= 0)) return "Colour Mapping clock beats must be positive.";
    if (!Number.isFinite(effectPhase()) || effectPhase() < 0 || effectPhase() > 1) return "Colour Mapping phase must be within 0..1.";
    if (![colorMappingOffsetU(), colorMappingOffsetV()].every((value) => Number.isFinite(value) && value >= -16 && value <= 16)) return "Colour Mapping offsets must be within -16..16.";
    if (![colorMappingScaleU(), colorMappingScaleV()].every((value) => Number.isFinite(value) && Math.abs(value) >= 0.01 && Math.abs(value) <= 16)) return "Colour Mapping scale magnitude must be within 0.01..16.";
    if (!Number.isFinite(colorMappingRotationDegrees()) || colorMappingRotationDegrees() < -3600 || colorMappingRotationDegrees() > 3600) return "Colour Mapping rotation must be within -3600..3600 degrees.";
    if (colorMappingCells().some((cell) => !Number.isFinite(cell.u) || !Number.isFinite(cell.v) || cell.u < -16 || cell.u > 16 || cell.v < -16 || cell.v > 16)) return "Colour Mapping cell coordinates must be within -16..16.";
    return "";
  });
  const editingEffectSummary = createMemo<EffectSummary | null>(() => {
    const effectId = editingEffectId();
    if (effectId === null) {
      return null;
    }
    return snapshot().effects.find((effect) => effect.id === effectId) ?? null;
  });
  const dmxPreviewOptions = createMemo(() => liveDmxPreviews());
  const activeDmxPreview = createMemo(() => {
    const previews = dmxPreviewOptions();
    return previews.find((preview) => preview.universe === rawDmxUniverse()) ?? previews[0];
  });
  const activeDmxPreviewUniverse = createMemo(() => activeDmxPreview()?.universe ?? snapshot().output.universe);
  const activeDmxPreviewValues = createMemo(() => activeDmxPreview()?.values ?? []);
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
  const snappedTimeBeats = (timeMs: number) => {
    if (timelineSnapMode() !== "Beat" && timelineSnapMode() !== "Bar") return undefined;
    const beatMs = Math.max(1, beatIntervalMs());
    return Number((Math.max(0, timeMs) / beatMs).toFixed(6));
  };
  const timeBeatsAtCurrentBpm = (timeMs: number) => {
    const beatMs = 60_000 / Math.max(1, snapshot().clock.bpm);
    return Number((Math.max(0, timeMs) / beatMs).toFixed(6));
  };
  const timelineLayers = createMemo(
    () => effectiveTimelineLayers(activeTimeline().layers),
    [] as TimelineLayerSummary[],
    { equals: sameTimelineLayerSummaries },
  );
  const timelineCueOptions = createMemo(
    () => buildTimelineSceneBlockCueOptions(
      timelineChildCueId() === null
        ? snapshotCues()
        : snapshotCues().filter((cue) => cue.id !== timelineChildCueId() && !cue.child_timeline),
    ),
    [],
    { equals: timelineSceneBlockCueOptionsEqual },
  );
  const selectedTimelineCueId = createMemo(() => {
    const current = timelineCueId();
    const cues = timelineCueOptions();
    if (current !== null && cues.some((cue) => cue.id === current)) {
      return current;
    }
    return cues[0]?.id ?? null;
  });
  const timelineEventRows = createMemo(
    () => buildTimelineSceneBlockRows(snapshotTimelineEvents(), snapshotCues()),
    [],
    { equals: timelineSceneBlockRowsEqual },
  );
  const timelineEventRowById = createMemo(() => new Map(
    timelineEventRows().map((event) => [event.id, event]),
  ));
  createEffect(() => {
    const selectedEventId = selectedTimelineSceneBlockEventId();
    if (selectedEventId !== null && !timelineEventRowById().has(selectedEventId)) {
      setSelectedTimelineSceneBlockEventId(null);
    }
  });
  const timelineAutomationFixtureOptions = createMemo(
    () => snapshot().fixtures.map((fixture) => ({ id: fixture.id, label: fixture.label })),
    [],
    {
      equals: (previous, next) => previous.length === next.length && previous.every((fixture, index) =>
        fixture.id === next[index].id && fixture.label === next[index].label),
    },
  );
  const timelineOverviewContentEndMsForTimeline = (timeline: TimelineSnapshot) => {
    const lightingAutomationEndMs = timeline.automations.flatMap((automation) =>
      automation.keyframes.map((keyframe) => keyframe.time_ms),
    );
    const videoAutomationEndMs = timeline.video_automations.flatMap((automation) =>
      automation.keyframes.map((keyframe) => keyframe.time_ms),
    );
    return Math.max(
      1,
      timeline.duration_ms,
      ...timeline.events.map(timelinePlacementDisplayEndMs),
      ...(timeline.audio_clips ?? []).map((clip) => clip.start_ms + clip.duration_ms),
      ...lightingAutomationEndMs,
      ...videoAutomationEndMs,
    );
  };
  const timelineOverviewContentEndMs = createMemo(() => timelineOverviewContentEndMsForTimeline(activeTimeline()));
  const [timelineViewportState, setTimelineViewportState] = createSignal(
    createTimelineViewportState(timelineOverviewContentEndMs()),
  );
  const timelineOverviewShowDurationMs = () => timelineViewportState().show_duration_ms;
  const timelineOverviewEditExtentMs = () => timelineViewportState().edit_extent_ms;
  const timelineVisibleWindow = () => timelineViewportState().visible_window;
  createEffect(() => {
    const showDurationMs = timelineOverviewContentEndMs();
    setTimelineViewportState((current) => reconcileTimelineViewportState(current, showDurationMs));
  });
  const timelineOverviewPlayheadX = createMemo(() =>
    timelineTimeToVisibleRawRatio(activeTimeline().position_ms, timelineVisibleWindow()) * 100,
  );
  const fitTimelineOverview = () => {
    setTimelineViewportState((current) => ({
      ...reconcileTimelineViewportState(current, timelineOverviewContentEndMs()),
      mode: "fit",
      visible_window: fitTimelineVisibleWindow(timelineOverviewContentEndMs()),
    }));
  };
  const timelineViewportAnchorMs = () => {
    const visibleWindow = timelineVisibleWindow();
    const selected = selectedTimelineSceneBlockEventId() === null
      ? undefined
      : timelineEventRowById().get(selectedTimelineSceneBlockEventId()!);
    const selectedCenterMs = selected
      ? selected.time_ms + timelineSceneBlockSpanMs(selected) / 2
      : Number.NaN;
    if (Number.isFinite(selectedCenterMs)) {
      const ratio = timelineTimeToVisibleRawRatio(selectedCenterMs, visibleWindow);
      if (ratio >= 0 && ratio <= 1) return selectedCenterMs;
    }
    const playheadMs = activeTimeline().position_ms;
    const playheadRatio = timelineTimeToVisibleRawRatio(playheadMs, visibleWindow);
    return playheadRatio >= 0 && playheadRatio <= 1
      ? playheadMs
      : visibleWindow.start_ms + timelineVisibleWindowSpanMs(visibleWindow) / 2;
  };
  const zoomTimelineOverview = (scale: number) => {
    const anchorMs = timelineViewportAnchorMs();
    setTimelineViewportState((current) => ({
      ...current,
      mode: "manual",
      visible_window: zoomTimelineVisibleWindow(
        current.visible_window,
        current.edit_extent_ms,
        scale,
        anchorMs,
      ),
    }));
  };
  const panTimelineOverview = (direction: -1 | 1) => {
    setTimelineViewportState((current) => ({
      ...current,
      mode: "manual",
      visible_window: panTimelineVisibleWindow(
        current.visible_window,
        current.edit_extent_ms,
        direction,
      ),
    }));
  };
  const revealTimelineSceneBlock = (eventId: number) => {
    const event = timelineEventRowById().get(eventId);
    if (!event) return;
    setTimelineViewportState((current) => ({
      ...current,
      mode: "manual",
      visible_window: revealTimelineVisibleRange(
        current.visible_window,
        current.edit_extent_ms,
        event.time_ms,
        event.duration_ms > 0 ? event.time_ms + timelineSceneBlockSpanMs(event) : event.time_ms,
      ),
    }));
  };
  const revealTimelinePlayhead = () => {
    const positionMs = activeTimeline().position_ms;
    setTimelineViewportState((current) => ({
      ...current,
      mode: "manual",
      visible_window: revealTimelineVisibleRange(
        current.visible_window,
        current.edit_extent_ms,
        positionMs,
        positionMs,
      ),
    }));
  };
  const selectTimelineSceneBlockEvent = (eventId: number, reveal: boolean) => {
    setSelectedTimelineSceneBlockEventId(eventId);
    setTimelineSceneBlockSelectionRevision((revision) => revision + 1);
    if (reveal) revealTimelineSceneBlock(eventId);
  };
  const timelineExecutionLive = createMemo(() => {
    if (timelineChildCue()) return false;
    const current = snapshot();
    return timelineExecutionIsLive(
      current.timeline.playing,
      current.clock.source,
      current.clock.external_sync_locked,
      current.clock.external_sync_age_ms,
    );
  });
  const timelinePositionMs = createMemo(() => activeTimeline().position_ms);
  const timelineOverviewEvents = createMemo<TimelineOverviewEvent[]>(() => {
    const visibleWindow = timelineVisibleWindow();
    const visibleSpanMs = timelineVisibleWindowSpanMs(visibleWindow);
    const currentPositionMs = timelinePositionMs();
    const layers = timelineLayers();
    const cueAuthoredBeatsById = new Map(snapshotCues().map((cue) => [cue.id, cue.authored_beats ?? null]));
    return timelineEventRows().filter((event) => timelineRangeIntersectsVisibleWindow(
      event.time_ms,
      event.duration_ms > 0 ? event.time_ms + timelineSceneBlockSpanMs(event) : event.time_ms,
      visibleWindow,
    )).map((event) => {
      const underPlayhead = event.duration_ms > 0
        ? currentPositionMs >= event.time_ms && currentPositionMs < event.time_ms + timelineSceneBlockSpanMs(event)
        : currentPositionMs === event.time_ms;
      return {
        id: event.id,
        cue_id: event.cue_id,
        cue_label: event.cue_label,
        track: event.track,
        layer_id: timelineLayerIdForEvent(layers, event),
        time_ms: event.time_ms,
        duration_ms: event.duration_ms,
        loop_count: event.loop_count,
        conform_to_tempo: event.conform_to_tempo ?? false,
        loop_fill: event.loop_fill ?? false,
        source_offset_ms: event.source_offset_ms ?? 0,
        rate: event.rate ?? null,
        authored_beats: cueAuthoredBeatsById.get(event.cue_id) ?? null,
        total_duration_ms: timelineSceneBlockSpanMs(event),
        fade_in_ms: Math.max(0, event.fade_in_ms ?? 0),
        fade_out_ms: Math.max(0, event.fade_out_ms ?? 0),
        x: timelineTimeToVisibleRawRatio(event.time_ms, visibleWindow) * 100,
        width: event.duration_ms > 0
          ? (timelineSceneBlockSpanMs(event) / visibleSpanMs) * 100
          : 0,
        y: event.track === "Lighting" ? 14 : 31,
        under_playhead: underPlayhead,
        is_super_scene: Boolean(snapshotCues().find((cue) => cue.id === event.cue_id)?.child_timeline),
      };
    });
  });
  const timelineOverlapClusterEvents = createMemo(
    () => timelineEventRows().map((event) => ({
      id: event.id,
      track: `${event.track}:${timelineLayerIdForEvent(timelineLayers(), event)}`,
      time_ms: event.time_ms,
      total_duration_ms: timelineSceneBlockSpanMs(event),
    })),
    [],
    {
      equals: (previous, next) => previous.length === next.length && previous.every((event, index) => {
        const candidate = next[index];
        return event.id === candidate.id &&
          event.track === candidate.track &&
          event.time_ms === candidate.time_ms &&
          event.total_duration_ms === candidate.total_duration_ms;
      }),
    },
  );
  const timelineOverlapClusters = createMemo(() => buildTimelineOverlapClusters(
    timelineOverlapClusterEvents(),
  ).map((cluster) => {
    const separatorIndex = cluster.track.lastIndexOf(":");
    const track = cluster.track.slice(0, separatorIndex) as TimelineTrackKind;
    const layerId = Number(cluster.track.slice(separatorIndex + 1));
    return {
      ...cluster,
      label: `${track} overlap (${cluster.count})`,
      track,
      layer_id: layerId,
    };
  }));
  const timelineOverviewOverlapClusters = createMemo<TimelineOverviewOverlapCluster[]>(() => {
    const visibleWindow = timelineVisibleWindow();
    const visibleSpanMs = timelineVisibleWindowSpanMs(visibleWindow);
    return timelineOverlapClusters()
      .filter((cluster) => timelineRangeIntersectsVisibleWindow(
        cluster.start_ms,
        cluster.end_ms,
        visibleWindow,
      ))
      .map((cluster) => ({
        ...cluster,
        member_ids: cluster.member_ids.map(Number),
        x: timelineTimeToVisibleRawRatio(cluster.start_ms, visibleWindow) * 100,
        width: ((cluster.end_ms - cluster.start_ms) / visibleSpanMs) * 100,
      }));
  });
  const timelineOverviewAutomationRanges = createMemo<TimelineOverviewAutomationRange[]>(() => {
    const visibleWindow = timelineVisibleWindow();
    const visibleSpanMs = timelineVisibleWindowSpanMs(visibleWindow);
    const currentSnapshot = snapshot();
    const timeline = activeTimeline();
    const overviewLayers = timelineLayers();
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
      if (!timelineClosedRangeIntersectsVisibleWindow(startMs, endMs, visibleWindow)) return null;
      const x = timelineTimeToVisibleRawRatio(startMs, visibleWindow) * 100;
      const width = Math.max(0.75, ((endMs - startMs) / visibleSpanMs) * 100);
      return {
        id,
        kind,
        automation_id: automationId,
        target_id: targetId,
        label,
        track,
        layer_id: overviewLayers.find((layer) => layer.kind === track)?.id ?? (track === "Lighting" ? 0 : 1),
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
    const lightingRanges = timeline.automations
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
    const videoRanges = timeline.video_automations
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
  const cueTimelinePlacementMap = createMemo(() => {
    const placements = new Map<number, TimelineCueEventSummary[]>();
    for (const event of timelineEventRows()) {
      const cuePlacements = placements.get(event.cue_id) ?? [];
      cuePlacements.push(event);
      placements.set(event.cue_id, cuePlacements);
    }
    for (const cuePlacements of placements.values()) {
      cuePlacements.sort((left, right) => left.time_ms - right.time_ms || left.id - right.id);
    }
    return placements;
  });
  const cueTimelinePlacementsForCue = (cueId: number) => cueTimelinePlacementMap().get(cueId) ?? [];
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
  const automationKeyframesEqual = (
    previous: { time_ms: number; value: number; interpolation: string }[],
    next: { time_ms: number; value: number; interpolation: string }[],
  ) => previous.length === next.length && previous.every((keyframe, index) => {
    const candidate = next[index];
    return keyframe.time_ms === candidate.time_ms &&
      keyframe.value === candidate.value &&
      keyframe.interpolation === candidate.interpolation;
  });
  const allTimelineAutomationRows = createMemo(
    () => activeTimeline().automations.map((automation) => {
        const fixture = snapshot().fixtures.find((candidate) => candidate.id === automation.fixture_id);
        return {
          ...automation,
          fixture_label: fixture?.label ?? `Fixture ${automation.fixture_id}`,
        };
      }),
    [],
    {
      equals: (previous, next) => previous.length === next.length && previous.every((automation, index) => {
        const candidate = next[index];
        return automation.id === candidate.id &&
          automation.fixture_id === candidate.fixture_id &&
          automation.attribute === candidate.attribute &&
          automation.track === candidate.track &&
          automation.enabled === candidate.enabled &&
          automation.fixture_label === candidate.fixture_label &&
          automationKeyframesEqual(automation.keyframes, candidate.keyframes);
      }),
    },
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
    `routes HAP ${diagnostics.hap_successes}/${diagnostics.hap_requests}, libav ${diagnostics.libav_successes}/${diagnostics.libav_requests}, CLI ${diagnostics.cli_fallback_successes}/${diagnostics.cli_fallback_requests}, deferred ${diagnostics.deferred_requests}, failed ${diagnostics.decode_failures}, cache ${diagnostics.hap_cache_len}+${diagnostics.libav_cache_len}+${diagnostics.cli_cache_len}, libav sessions ${diagnostics.libav_session_count} (open ${diagnostics.libav_session_open_count}, reset ${diagnostics.libav_session_reset_count}, seq ${diagnostics.libav_sequential_continue_count}, reuse ${diagnostics.libav_frame_reuse_count}, evict ${diagnostics.libav_working_set_eviction_count}, error ${diagnostics.libav_session_error_count})`;
  const videoPreviewDiagnosticsText = createMemo(() => {
    const diagnostics = videoPreviewDiagnostics();
    if (!diagnostics) {
      return "No preview diagnostics";
    }
    const queuedFrames = diagnostics.layer_queues.reduce((total, row) => total + row.queue_len, 0);
    const activeQueues = diagnostics.layer_queues.filter((row) => row.queue_len > 0).length;
    const readyQueues = diagnostics.layer_queues.filter((row) => row.ready).length;
    const bpm = diagnostics.bpm && Number.isFinite(diagnostics.bpm) ? `, bpm ${diagnostics.bpm.toFixed(1)}` : "";
    const isf = `, ISF pipelines ${diagnostics.isf_pipeline_count}, stack ${diagnostics.isf_last_stack_stage_count}, render ${diagnostics.isf_last_stack_render_us}us${diagnostics.last_isf_error ? " error" : ""}`;
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
    return visibleProjectDirty() ? `${label} *` : label;
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
    () => snapshot().fixtures.length > 0
      || snapshot().video.layers.length > 0
      || snapshot().video.outputs.length > 0
      || snapshot().node_graphs.length > 0,
  );
  const cueCaptureEligibleEffects = createMemo(() => eligibleCueEffects(
    snapshot().effects,
    cueCaptureScope(),
    {
      fixtures: snapshot().fixtures,
      selectedFixtureId: selectedFixtureId(),
      selectedGroupId: selectedFixtureGroupFilter(),
    },
  ));
  let lastCueEffectEligibilityIds = "";
  let lastCueEffectEligibilityStates = "";
  createEffect(() => {
    const effects = cueCaptureEligibleEffects();
    const idSignature = effects.map((effect) => effect.id).join(",");
    const stateSignature = effects.map((effect) => `${effect.id}:${effect.enabled ? 1 : 0}`).join(",");
    if (idSignature === lastCueEffectEligibilityIds && stateSignature === lastCueEffectEligibilityStates) return;
    const idsChanged = idSignature !== lastCueEffectEligibilityIds;
    lastCueEffectEligibilityIds = idSignature;
    lastCueEffectEligibilityStates = stateSignature;
    setCueEffectCaptureTargets((current) => syncCueEffectCaptureTargets(
      effects,
      current,
      cueEffectCaptureStateOverrideIds(),
      idsChanged,
    ));
    const eligibleIds = new Set(effects.map((effect) => effect.id));
    setCueEffectCaptureStateOverrideIds((current) => current.filter((effectId) => eligibleIds.has(effectId)));
  });
  const updateCueEffectCaptureTargets = (targets: CueEffectTarget[], change: CueEffectRecallChange) => {
    setCueEffectCaptureTargets(targets);
    const targetIds = new Set(targets.map((target) => target.effect_id));
    setCueEffectCaptureStateOverrideIds((current) => {
      if (change.kind === "captureCurrent" || change.kind === "clear") return [];
      if (change.kind === "state") {
        return [...new Set([...current.filter((effectId) => targetIds.has(effectId)), change.effectId])];
      }
      if (change.kind === "include") {
        return current.filter((effectId) => effectId !== change.effectId && targetIds.has(effectId));
      }
      return current.filter((effectId) => targetIds.has(effectId));
    });
  };
  const inferredCueCaptureAuthoredBeats = createMemo(() => inferredCueAuthoredBeats(
    snapshot().effects,
    cueEffectCaptureTargets(),
  ));
  let lastCueAuthoredBeatsSeedSignature = "";
  createEffect(() => {
    const effectsById = new Map(snapshot().effects.map((effect) => [effect.id, effect]));
    const signature = JSON.stringify(cueEffectCaptureTargets().map((target) => {
      const effect = effectsById.get(target.effect_id);
      return [
        target.effect_id,
        target.enabled,
        effect?.effect_type ?? null,
        effect ? authoredBeatsForEffectClock(effect) : null,
        effect?.chaser?.steps.length ?? null,
        effect?.chaser?.direction ?? null,
      ];
    }));
    if (signature === lastCueAuthoredBeatsSeedSignature) return;
    lastCueAuthoredBeatsSeedSignature = signature;
    const inferred = inferredCueCaptureAuthoredBeats();
    if (inferred !== null) {
      setCueAuthoredBeats(inferred);
      setCueAuthoredBeatsManual(false);
    } else if (!cueAuthoredBeatsManual()) {
      setCueAuthoredBeats(null);
    }
  });
  const updateCueAuthoredBeats = (value: number | null) => {
    setCueAuthoredBeats(value);
    setCueAuthoredBeatsManual(true);
  };
  const cueAuthoredBeatsSeeded = createMemo(() => {
    const inferred = inferredCueCaptureAuthoredBeats();
    return !cueAuthoredBeatsManual()
      && inferred !== null
      && cueAuthoredBeats() !== null
      && Math.abs(cueAuthoredBeats()! - inferred) <= 1e-6;
  });
  const cueAuthoredBeatsError = createMemo(() => {
    const value = cueAuthoredBeats();
    return value !== null && (!Number.isFinite(value) || value < 0.25 || value > 1024)
      ? "Authored beats must be from 0.25 to 1024."
      : null;
  });
  const hasCueEffectCaptureTargets = createMemo(() => cueEffectCaptureTargets().length > 0);
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
  const cueCaptureScopeErrorForEffectSelection = (hasEffectTargets: boolean) => {
    switch (cueCaptureScope()) {
      case "all":
        return hasCueSources() || hasEffectTargets
          ? ""
          : "Patch fixtures, add video, create a Node Graph, or include at least one Effect.";
      case "lighting":
        return snapshot().fixtures.length > 0 || hasEffectTargets
          ? ""
          : "Patch fixtures or include at least one Effect before storing this Cue.";
      case "effects":
        return hasEffectTargets ? "" : "Include at least one Effect before storing an Effects Only Cue.";
      case "selectedFixture":
        return selectedFixture() ? "" : "Select a fixture before storing a selected-fixture cue.";
      case "selectedGroup":
        return selectedFixtureGroupFilter() ? "" : "Select a group before storing a group cue.";
      case "video":
        return snapshot().video.layers.length > 0
          || snapshot().video.outputs.length > 0
          || snapshot().node_graphs.length > 0
          ? ""
          : "Add video layers, outputs, or Node Graphs before storing a video cue.";
    }
  };
  const cueCaptureScopeError = createMemo(() => cueCaptureScopeErrorForEffectSelection(hasCueEffectCaptureTargets()));
  const cueCaptureScopeRequest = (): CueCaptureScopeRequest | null => {
    switch (cueCaptureScope()) {
      case "all":
        return { kind: "all" };
      case "lighting":
        return { kind: "lightingOnly" };
      case "effects":
        return { kind: "effectsOnly" };
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
  const allTimelineVideoAutomationRows = createMemo(
    () => activeTimeline().video_automations.map((automation) => {
        const layer = snapshot().video.layers.find((candidate) => candidate.id === automation.layer_id);
        return {
          ...automation,
          layer_label: layer?.label ?? `Video Layer ${automation.layer_id}`,
        };
      }),
    [],
    {
      equals: (previous, next) => previous.length === next.length && previous.every((automation, index) => {
        const candidate = next[index];
        return automation.id === candidate.id &&
          automation.layer_id === candidate.layer_id &&
          automation.param === candidate.param &&
          automation.track === candidate.track &&
          automation.enabled === candidate.enabled &&
          automation.layer_label === candidate.layer_label &&
          automationKeyframesEqual(automation.keyframes, candidate.keyframes);
      }),
    },
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
      ? activeTimeline().automations.some((automation) => automation.id === selected.automationId)
      : activeTimeline().video_automations.some((automation) => automation.id === selected.automationId);
    if (!exists) {
      setSelectedTimelineAutomation(null);
    }
  });
  const activeCue = createMemo(() => {
    const activeCueId = snapshot().active_cue_id;
    return snapshot().cues.find((cue) => cue.id === activeCueId) ?? null;
  });
  const selectedSceneCue = createMemo(() => {
    const cueId = selectedSceneCueId();
    return cueId === null
      ? null
      : snapshot().cues.find((cue) => cue.id === cueId) ?? null;
  });
  const selectedSceneEffects = createMemo(() => {
    const cue = selectedSceneCue();
    if (!cue) return [];
    const effectsById = new Map(snapshot().effects.map((effect) => [effect.id, effect]));
    return cue.effect_targets.flatMap((target) => {
      const effect = cueOwnedEffectSummary(target, effectsById.get(target.effect_id) ?? null);
      return effect ? [effect] : [];
    });
  });
  const selectedSceneIsRunning = createMemo(() => {
    const cueId = selectedSceneCueId();
    if (cueId === null) return false;
    return snapshot().active_cue_id === cueId
      || Object.values(snapshot().active_group_cue_ids ?? {}).includes(cueId);
  });
  // Components that only carry cue ids receive enough identity context to
  // preserve cue > group > cue-hash priority across Timeline consumers.
  const cueIdentities = createMemo<Record<number, CueIdentitySource>>(() => {
    const map: Record<number, CueIdentitySource> = {};
    const colors = snapshot().group_colors ?? {};
    for (const cue of snapshot().cues) {
      map[cue.id] = {
        color: cue.color,
        groupId: cue.group_id,
        groupColor: cue.group_id ? colors[cue.group_id] : null,
      };
    }
    return map;
  });
  const groupColors = createMemo<Record<string, string>>(() => snapshot().group_colors ?? {});
  const setCueColor = async (cueId: number, color: string | null) => {
    if (viewportFixture) {
      setSnapshot((current) => ({
        ...current,
        cues: current.cues.map((cue) => (cue.id === cueId ? { ...cue, color } : cue)),
      }));
      return;
    }
    try {
      await invoke("set_cue_color", { cueId, color });
      await refreshSnapshot();
      setMessage(color ? `Cue color updated (${color})` : "Cue color cleared");
    } catch (error) {
      setMessage(`Cue color update failed: ${error}`);
    }
  };
  const setCueLiveModifierDefaults = async (
    cueId: number,
    settings: CueLiveModifierSettings | null,
  ) => {
    if (viewportFixture) {
      setSnapshot((current) => ({
        ...current,
        cues: current.cues.map((cue) =>
          cue.id === cueId ? { ...cue, live_modifiers: settings } : cue,
        ),
      }));
      return;
    }
    try {
      await invoke("set_cue_live_modifier_defaults", { cueId, settings });
      await refreshSnapshot();
      setMessage(settings ? "Live modifier defaults updated" : "Live modifier defaults cleared");
    } catch (error) {
      setMessage(`Live modifier defaults update failed: ${error}`);
    }
  };
  const setGroupColor = async (groupId: string, color: string | null) => {
    if (viewportFixture) {
      setSnapshot((current) => {
        const nextColors = { ...(current.group_colors ?? {}) };
        if (color) nextColors[groupId] = color;
        else delete nextColors[groupId];
        return { ...current, group_colors: nextColors };
      });
      return;
    }
    try {
      await invoke("set_group_color", { groupId, color });
      await refreshSnapshot();
      setMessage(color ? `Group color updated (${color})` : "Group color cleared");
    } catch (error) {
      setMessage(`Group color update failed: ${error}`);
    }
  };
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
  const superSceneCueCount = createMemo(() =>
    snapshotCues().reduce((count, cue) => count + (cue.child_timeline ? 1 : 0), 0));
  const cueOwnedEffectCount = createMemo(() =>
    snapshotCues().reduce((count, cue) => count + cue.effect_targets.length, 0));
  const faderDeskTitle = createMemo(() => {
    if (controlMode() === "edit") return editDeskSurface() === "faders" ? "Faders" : "Attributes";
    if (controlMode() !== "live") return "Faders";
    switch (timelineDeskSurface()) {
      case "automation": return "Automation";
      case "playback": return "Playback";
      default: return "Show Timeline";
    }
  });
  const touchLayoutClass = createMemo(() =>
    workspaceTab() === "setup"
      ? `layoutSharedWorkspace layoutSetup setupMode-${setupSubTab()}`
      : workspaceTab() === "touch"
        ? "layoutTouch"
        : `${controlMode() === "mixer" ? "" : "layoutSharedWorkspace "}layoutControl controlMode${controlMode()[0].toUpperCase()}${controlMode().slice(1)}${
            controlMode() === "edit" ? ` editDesk-${editDeskSurface()}` : ""
          }`,
  );
  const sharedWorkspaceVisible = createMemo(
    () => workspaceTab() === "setup" || (workspaceTab() === "control" && controlMode() !== "mixer"),
  );
  const liveMappingStageVisible = createMemo(
    () => (workspaceTab() === "setup" && setupSubTab() === "mapping")
      || (workspaceTab() === "control" && controlMode() !== "mixer"),
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
      if (scope === "video" || scope === "effects") {
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
        ? "All Sources"
        : scope === "lighting"
          ? "Lighting Only"
          : scope === "effects"
            ? "Effects Only"
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
    mappingViewportMaxZoom,
    mappingViewportBox,
    mappingStageViewBox,
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
    viewportPixelSize: mappingStageViewportPixelSize,
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
    mappingStageFixtures,
    visualizerVideoSurfaces2d,
    visualizerStageObjects2d,
  } = createMappingRenderModel({
    mappingDrag,
    mappingShowGeometry,
    mappingFilteredFixtures,
    liveFixtures,
    mappingViewportBox,
    dmxPreviews: dmxPreviewOptions,
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
  const openLiveAudioInputSettings = () => {
    setWorkspaceTab("setup");
    selectSetupMode("video");
    requestAnimationFrame(() => {
      requestAnimationFrame(() => {
        const target = document.getElementById("setup-output-audio-input");
        target?.focus({ preventScroll: true });
        target?.scrollIntoView({ block: "nearest", inline: "nearest" });
      });
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

  const selectPickedFixtureType = (typeKey: string) => {
    const group = pickedFixtureTypeGroups().find((candidate) => candidate.key === typeKey);
    const firstFixture = group?.fixtures[0];
    if (!group || !firstFixture) {
      return;
    }
    setSelectedFixtureGroupFilter("");
    activateFixture(firstFixture);
    setSelectedMappingFixtureIds(group.fixtures.map((fixture) => fixture.id));
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

  const openSceneFxFromMapping = () => {
    setWorkspaceTab("control");
    setControlMode("live");
    setTimelineDeskSurface("show");
    setTimelineContextDrawer("none");
    setSceneSettingsSurface("fx");
    setMessage(
      selectedSceneCue()
        ? `Opened Scene FX for ${selectedSceneCue()!.label}.`
        : "Select a scene with its right-edge identity strip, then open the FX surface.",
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

  const syncCueMetadataDrafts = (cues: CueSummary[], effects: EffectSummary[]) => {
    setCueMetadataDrafts((current) => {
      const nextDrafts: Record<number, CueMetadataDraft> = {};
      for (const cue of cues) {
        const draft = current[cue.id] ?? cueMetadataDraftFromSummary(cue);
        nextDrafts[cue.id] = {
          ...draft,
          effect_targets: normalizedCueEffectTargets(effects, draft.effect_targets),
        };
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
    setTimelineEventDrafts((current) => reconcileTimelineEventDrafts(events, current));
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
  const confirmDiscardTimelineEditorDrafts = () =>
    !timelineEditorDirty() || window.confirm(translateUiText("Discard unsaved Timeline edits?", uiLocale()));

  const persistChildTimeline = async (
    cueId: number,
    update: (child: ChildTimelineSummary) => ChildTimelineSummary,
  ) => {
    const cue = snapshot().cues.find((candidate) => candidate.id === cueId);
    if (!cue?.child_timeline) throw new Error(`Super Scene Cue ${cueId} was not found`);
    const childTimeline = normalizedChildTimeline(update(normalizedChildTimeline(cue.child_timeline)));
    const localFixture = viewportFixture === "timeline-layered"
      || viewportFixture === "scene-block-large"
      || viewportFixture === "scene-block-hour";
    if (!localFixture) {
      await invoke("set_cue_child_timeline", { cueId, childTimeline });
      await refreshSnapshot();
      return;
    }
    setSnapshot((current) => ({
      ...current,
      cues: current.cues.map((candidate) => candidate.id === cueId
        ? { ...candidate, child_timeline: childTimeline }
        : candidate),
    }));
  };

  const childTimelineNextAutomationId = () => Math.max(
    0,
    ...snapshot().timeline.automations.map((automation) => automation.id),
    ...snapshot().timeline.video_automations.map((automation) => automation.id),
    ...snapshot().cues.flatMap((cue) => [
      ...(cue.child_timeline?.automations?.map((automation) => automation.id) ?? []),
      ...(cue.child_timeline?.video_automations?.map((automation) => automation.id) ?? []),
    ]),
  ) + 1;

  const childTimelineDurationForKeyframes = (keyframes: Array<{ time_ms: number }>) =>
    keyframes.reduce((duration, keyframe) => Math.max(duration, Math.max(0, keyframe.time_ms)), 0);

  const invokeTimelineEditingCommand = async <T,>(
    command: string,
    args?: Record<string, unknown>,
  ): Promise<T> => {
    const childCueId = timelineChildCueId();
    if (childCueId === null || command === "set_timeline_playing" || command === "seek_timeline") {
      return invoke<T>(command, args);
    }
    const child = normalizedChildTimeline(timelineChildCue()?.child_timeline ?? {});
    if (command === "set_timeline_automation_enabled") {
      const automationId = Number(args?.automationId);
      const enabled = Boolean(args?.enabled);
      await persistChildTimeline(childCueId, (current) => ({
        ...current,
        automations: (current.automations ?? []).map((automation) => automation.id === automationId
          ? { ...automation, enabled }
          : automation),
        video_automations: (current.video_automations ?? []).map((automation) => automation.id === automationId
          ? { ...automation, enabled }
          : automation),
      }));
      return undefined as T;
    }
    if (command === "add_timeline_automation") {
      const automationId = childTimelineNextAutomationId();
      const keyframes = (args?.keyframes ?? []) as AutomationKeyframeSummary[];
      const automation: TimelineAutomationSummary = {
        id: automationId,
        fixture_id: Number(args?.fixtureId),
        attribute: String(args?.attribute ?? ""),
        track: "Lighting",
        keyframes,
        enabled: true,
      };
      await persistChildTimeline(childCueId, (current) => ({
        ...current,
        automations: [...(current.automations ?? []), automation],
        duration_ms: Math.max(current.duration_ms ?? 0, childTimelineDurationForKeyframes(keyframes)),
      }));
      return automationId as T;
    }
    if (command === "add_timeline_group_automation") {
      const groupId = String(args?.groupId ?? "");
      const attribute = String(args?.attribute ?? "");
      const keyframes = (args?.keyframes ?? []) as AutomationKeyframeSummary[];
      const fixtures = snapshot().fixtures.filter((fixture) => fixture.group_ids.includes(groupId));
      const compatible = fixtures.filter((fixture) => fixture.controls.some((control) => control.attribute === attribute));
      let nextId = childTimelineNextAutomationId();
      const automations = compatible.map((fixture): TimelineAutomationSummary => ({
        id: nextId++,
        fixture_id: fixture.id,
        attribute,
        track: "Lighting",
        keyframes,
        enabled: true,
      }));
      await persistChildTimeline(childCueId, (current) => ({
        ...current,
        automations: [...(current.automations ?? []), ...automations],
        duration_ms: Math.max(current.duration_ms ?? 0, childTimelineDurationForKeyframes(keyframes)),
      }));
      return {
        automation_ids: automations.map((automation) => automation.id),
        applied_count: automations.length,
        skipped_count: fixtures.length - automations.length,
      } as T;
    }
    if (command === "set_timeline_automation") {
      const automationId = Number(args?.automationId);
      const keyframes = (args?.keyframes ?? []) as AutomationKeyframeSummary[];
      if (!child.automations?.some((automation) => automation.id === automationId)) {
        throw new Error(`Child lighting automation ${automationId} was not found`);
      }
      await persistChildTimeline(childCueId, (current) => ({
        ...current,
        automations: (current.automations ?? []).map((automation) => automation.id === automationId
          ? {
              ...automation,
              fixture_id: Number(args?.fixtureId),
              attribute: String(args?.attribute ?? ""),
              keyframes,
            }
          : automation),
        duration_ms: Math.max(current.duration_ms ?? 0, childTimelineDurationForKeyframes(keyframes)),
      }));
      return undefined as T;
    }
    if (command === "add_timeline_video_automation") {
      const automationId = childTimelineNextAutomationId();
      const keyframes = (args?.keyframes ?? []) as VideoAutomationKeyframeSummary[];
      const automation: TimelineVideoAutomationSummary = {
        id: automationId,
        layer_id: Number(args?.layerId),
        param: args?.param as VideoParam,
        track: "Video",
        keyframes,
        enabled: true,
      };
      await persistChildTimeline(childCueId, (current) => ({
        ...current,
        video_automations: [...(current.video_automations ?? []), automation],
        duration_ms: Math.max(current.duration_ms ?? 0, childTimelineDurationForKeyframes(keyframes)),
      }));
      return automationId as T;
    }
    if (command === "set_timeline_video_automation") {
      const automationId = Number(args?.automationId);
      const keyframes = (args?.keyframes ?? []) as VideoAutomationKeyframeSummary[];
      if (!child.video_automations?.some((automation) => automation.id === automationId)) {
        throw new Error(`Child video automation ${automationId} was not found`);
      }
      await persistChildTimeline(childCueId, (current) => ({
        ...current,
        video_automations: (current.video_automations ?? []).map((automation) => automation.id === automationId
          ? {
              ...automation,
              layer_id: Number(args?.layerId),
              param: args?.param as VideoParam,
              keyframes,
            }
          : automation),
        duration_ms: Math.max(current.duration_ms ?? 0, childTimelineDurationForKeyframes(keyframes)),
      }));
      return undefined as T;
    }
    if (command === "remove_timeline_automation") {
      const automationId = Number(args?.automationId);
      await persistChildTimeline(childCueId, (current) => ({
        ...current,
        automations: (current.automations ?? []).filter((automation) => automation.id !== automationId),
        video_automations: (current.video_automations ?? []).filter((automation) => automation.id !== automationId),
      }));
      return undefined as T;
    }
    return invoke<T>(command, args);
  };

  const invokeTimelineSceneBlockCommand = async <T,>(command: string, args?: Record<string, unknown>): Promise<T> => {
    const childCueId = timelineChildCueId();
    if (childCueId !== null) {
      const current = activeTimeline();
      if (command === "add_timeline_scene_block") {
        const eventId = Math.max(
          0,
          ...snapshot().timeline.events.map((event) => event.id),
          ...snapshot().cues.flatMap((cue) => cue.child_timeline?.events?.map((event) => event.id) ?? []),
        ) + 1;
        const durationMs = Number(args?.durationMs ?? 1_000);
        const cueId = Number(args?.cueId);
        const authoredBeats = snapshot().cues.find((cue) => cue.id === cueId)?.authored_beats ?? null;
        const conformToTempo = Boolean(args?.conformToTempo);
        const loopFill = Boolean(args?.loopFill);
        const event: TimelineCueEventSummary = {
          id: eventId,
          cue_id: cueId,
          time_ms: Number(args?.timeMs ?? 0),
          time_beats: args?.timeBeats === null || args?.timeBeats === undefined ? null : Number(args.timeBeats),
          track: (args?.track ?? "Lighting") as TimelineTrackKind,
          layer_id: args?.layerId === null || args?.layerId === undefined ? null : Number(args.layerId),
          duration_ms: durationMs,
          duration_beats: args?.durationBeats === null || args?.durationBeats === undefined
            ? null
            : Number(args.durationBeats),
          conform_to_tempo: conformToTempo,
          loop_fill: loopFill,
          source_offset_ms: Math.max(0, Number(args?.sourceOffsetMs ?? 0)),
          rate: conformToTempo && !loopFill && authoredBeats !== null && durationMs > 0
            ? (authoredBeats * 60_000 / snapshot().clock.bpm) / durationMs
            : null,
          fade_in_ms: Math.min(Number(args?.fadeInMs ?? 0), durationMs),
          fade_out_ms: Math.min(Number(args?.fadeOutMs ?? 0), durationMs),
          loop_count: Number(args?.loopCount ?? 1),
          jump_to_event_id: args?.jumpToEventId === null || args?.jumpToEventId === undefined
            ? null
            : Number(args.jumpToEventId),
        };
        await persistChildTimeline(childCueId, (child) => ({
          ...child,
          events: [...(child.events ?? []), event],
          duration_ms: Math.max(child.duration_ms ?? 0, timelinePlacementDisplayEndMs(event)),
        }));
        return eventId as T;
      }
      if (command === "set_timeline_scene_block" || command === "set_timeline_cue_event") {
        const eventId = Number(args?.eventId);
        const source = current.events.find((event) => event.id === eventId);
        if (!source) throw new Error(`Child timeline event ${eventId} was not found`);
        const durationMs = command === "set_timeline_scene_block"
          ? Number(args?.durationMs ?? source.duration_ms)
          : 0;
        const cueId = Number(args?.cueId ?? source.cue_id);
        const authoredBeats = snapshot().cues.find((cue) => cue.id === cueId)?.authored_beats ?? null;
        const conformToTempo = command === "set_timeline_scene_block" && Boolean(args?.conformToTempo);
        const loopFill = command === "set_timeline_scene_block" && Boolean(args?.loopFill);
        const nextEvent: TimelineCueEventSummary = {
          ...source,
          cue_id: cueId,
          time_ms: Number(args?.timeMs ?? source.time_ms),
          time_beats: args?.timeBeats === null || args?.timeBeats === undefined ? null : Number(args.timeBeats),
          track: (args?.track ?? source.track) as TimelineTrackKind,
          layer_id: args?.layerId === null || args?.layerId === undefined ? null : Number(args.layerId),
          duration_ms: durationMs,
          duration_beats: command === "set_timeline_scene_block" && args?.durationBeats !== null
            && args?.durationBeats !== undefined ? Number(args.durationBeats) : null,
          conform_to_tempo: conformToTempo,
          loop_fill: loopFill,
          source_offset_ms: command === "set_timeline_scene_block"
            ? Math.max(0, Number(args?.sourceOffsetMs ?? source.source_offset_ms ?? 0))
            : 0,
          rate: conformToTempo && !loopFill && authoredBeats !== null && durationMs > 0
            ? (authoredBeats * 60_000 / snapshot().clock.bpm) / durationMs
            : null,
          fade_in_ms: command === "set_timeline_scene_block"
            ? Math.min(Number(args?.fadeInMs ?? source.fade_in_ms ?? 0), durationMs) : 0,
          fade_out_ms: command === "set_timeline_scene_block"
            ? Math.min(Number(args?.fadeOutMs ?? source.fade_out_ms ?? 0), durationMs) : 0,
          loop_count: command === "set_timeline_scene_block" ? Number(args?.loopCount ?? source.loop_count) : 1,
          jump_to_event_id: command === "set_timeline_scene_block" && args?.jumpToEventId !== null
            && args?.jumpToEventId !== undefined ? Number(args.jumpToEventId) : null,
        };
        await persistChildTimeline(childCueId, (child) => ({
          ...child,
          events: (child.events ?? []).map((event) => event.id === eventId ? nextEvent : event),
        }));
        return undefined as T;
      }
      if (command === "remove_timeline_scene_block" || command === "remove_timeline_event") {
        const eventId = Number(args?.eventId);
        await persistChildTimeline(childCueId, (child) => ({
          ...child,
          events: (child.events ?? []).filter((event) => event.id !== eventId),
        }));
        return undefined as T;
      }
    }
    const localFixture = viewportFixture === "timeline-layered"
      || viewportFixture === "scene-block-large"
      || viewportFixture === "scene-block-hour"
      || viewportFixture === "scene-matrix";
    if (localFixture && command === "add_timeline_scene_block") {
      const eventId = Math.max(
        0,
        ...snapshot().timeline.events.map((event) => event.id),
        ...snapshot().cues.flatMap((cue) => cue.child_timeline?.events?.map((event) => event.id) ?? []),
      ) + 1;
      setSnapshot((current) => ({
        ...current,
        timeline: {
          ...current.timeline,
          events: [...current.timeline.events, {
            id: eventId,
            cue_id: Number(args?.cueId),
            time_ms: Number(args?.timeMs ?? 0),
            time_beats: args?.timeBeats === null || args?.timeBeats === undefined ? null : Number(args.timeBeats),
            track: (args?.track ?? "Lighting") as TimelineTrackKind,
            layer_id: args?.layerId === null || args?.layerId === undefined ? null : Number(args.layerId),
            duration_ms: Number(args?.durationMs ?? 1_000),
            duration_beats: args?.durationBeats === null || args?.durationBeats === undefined
              ? null
              : Number(args.durationBeats),
            conform_to_tempo: Boolean(args?.conformToTempo),
            loop_fill: Boolean(args?.loopFill),
            source_offset_ms: Math.max(0, Number(args?.sourceOffsetMs ?? 0)),
            rate: Boolean(args?.conformToTempo) && !Boolean(args?.loopFill)
              ? (() => {
                  const cue = current.cues.find((candidate) => candidate.id === Number(args?.cueId));
                  const authoredBeats = cue?.authored_beats ?? null;
                  const durationMs = Number(args?.durationMs ?? 1_000);
                  return authoredBeats !== null && durationMs > 0
                    ? (authoredBeats * 60_000 / current.clock.bpm) / durationMs
                    : null;
                })()
              : null,
            fade_in_ms: Math.min(Number(args?.fadeInMs ?? 0), Number(args?.durationMs ?? 1_000)),
            fade_out_ms: Math.min(Number(args?.fadeOutMs ?? 0), Number(args?.durationMs ?? 1_000)),
            loop_count: Number(args?.loopCount ?? 1),
            jump_to_event_id: args?.jumpToEventId === null || args?.jumpToEventId === undefined
              ? null
              : Number(args.jumpToEventId),
          }],
        },
      }));
      return eventId as T;
    }
    if (localFixture && (command === "set_timeline_scene_block" || command === "set_timeline_cue_event")) {
      const eventId = Number(args?.eventId);
      setSnapshot((current) => ({
        ...current,
        timeline: {
          ...current.timeline,
          events: current.timeline.events.map((event) => event.id === eventId
            ? {
                ...event,
                cue_id: Number(args?.cueId ?? event.cue_id),
                time_ms: Number(args?.timeMs ?? event.time_ms),
                time_beats: args?.timeBeats === null || args?.timeBeats === undefined
                  ? null
                  : Number(args.timeBeats),
                track: (args?.track ?? event.track) as TimelineTrackKind,
                layer_id: args?.layerId === null || args?.layerId === undefined
                  ? null
                  : Number(args.layerId),
                duration_ms: command === "set_timeline_scene_block"
                  ? Number(args?.durationMs ?? event.duration_ms)
                  : 0,
                duration_beats: command === "set_timeline_scene_block"
                  ? (args?.durationBeats === null || args?.durationBeats === undefined
                      ? null
                      : Number(args.durationBeats))
                  : null,
                conform_to_tempo: command === "set_timeline_scene_block"
                  ? Boolean(args?.conformToTempo)
                  : false,
                loop_fill: command === "set_timeline_scene_block"
                  ? Boolean(args?.loopFill)
                  : false,
                source_offset_ms: command === "set_timeline_scene_block"
                  ? Math.max(0, Number(args?.sourceOffsetMs ?? event.source_offset_ms ?? 0))
                  : 0,
                rate: command === "set_timeline_scene_block" && Boolean(args?.conformToTempo) && !Boolean(args?.loopFill)
                  ? (() => {
                      const cue = current.cues.find((candidate) => candidate.id === Number(args?.cueId ?? event.cue_id));
                      const authoredBeats = cue?.authored_beats ?? null;
                      const durationMs = Number(args?.durationMs ?? event.duration_ms);
                      return authoredBeats !== null && durationMs > 0
                        ? (authoredBeats * 60_000 / current.clock.bpm) / durationMs
                        : null;
                    })()
                  : null,
                fade_in_ms: command === "set_timeline_scene_block"
                  ? Math.min(Number(args?.fadeInMs ?? event.fade_in_ms ?? 0), Number(args?.durationMs ?? event.duration_ms))
                  : 0,
                fade_out_ms: command === "set_timeline_scene_block"
                  ? Math.min(Number(args?.fadeOutMs ?? event.fade_out_ms ?? 0), Number(args?.durationMs ?? event.duration_ms))
                  : 0,
                loop_count: command === "set_timeline_scene_block"
                  ? Number(args?.loopCount ?? event.loop_count)
                  : 1,
                jump_to_event_id: command === "set_timeline_scene_block"
                  ? (args?.jumpToEventId === null || args?.jumpToEventId === undefined
                      ? null
                      : Number(args.jumpToEventId))
                  : null,
              }
            : event),
        },
      }));
      return undefined as T;
    }
    if (localFixture && (command === "remove_timeline_scene_block" || command === "remove_timeline_event")) {
      const eventId = Number(args?.eventId);
      setSnapshot((current) => ({
        ...current,
        timeline: {
          ...current.timeline,
          events: current.timeline.events.filter((event) => event.id !== eventId),
        },
      }));
      return undefined as T;
    }
    return invoke<T>(command, args);
  };
  const zoomTimelineOverviewAt = (anchorMs: number, scale: number) => {
    setTimelineViewportState((current) => ({
      ...current,
      mode: "manual",
      visible_window: zoomTimelineVisibleWindow(
        current.visible_window,
        current.edit_extent_ms,
        scale,
        anchorMs,
      ),
    }));
  };
  const setTimelineVisibleWindowDirect = (visibleWindow: { start_ms: number; end_ms: number }) => {
    setTimelineViewportState((current) => ({
      ...current,
      mode: "manual",
      visible_window: normalizeTimelineVisibleWindow(
        visibleWindow,
        current.edit_extent_ms,
      ),
    }));
  };

  const timelineSceneBlocks = createTimelineSceneBlockController({
    invoke: invokeTimelineSceneBlockCommand,
    snapTimeMs,
    snappedTimeBeats,
    timeBeatsAtCurrentBpm,
    hasEventId: (eventId) => activeTimeline().events.some((event) => event.id === eventId),
    getEventById: (eventId) => timelineEventRowById().get(eventId),
    getEventDraft: timelineEventDraft,
    setEventDraft: (eventId, draft) => setTimelineEventDrafts((current) => ({
      ...current,
      [eventId]: draft,
    })),
    getAddDurationMs: timelineBlockDurationMs,
    getAddLoopCount: timelineBlockLoopCount,
    getAddJumpToEventId: timelineBlockJumpToEventId,
    getBpm: () => snapshot().clock.bpm,
    getCueAuthoredBeats: (cueId) => snapshot().cues.find((cue) => cue.id === cueId)?.authored_beats ?? null,
    setNextStartMs: setTimelineEventTimeMs,
    setMessage,
    refreshSnapshot: () => viewportFixture === "timeline-layered"
      || viewportFixture === "scene-block-large"
      || viewportFixture === "scene-block-hour"
      || viewportFixture === "scene-matrix"
      ? Promise.resolve(snapshot())
      : refreshSnapshot(),
  });

  const invokeTimelineLayerCommand = async <T,>(command: string, args?: Record<string, unknown>): Promise<T> => {
    const childCueId = timelineChildCueId();
    if (childCueId !== null) {
      if (command === "add_timeline_layer") {
        const layerId = Math.max(1, ...timelineLayers().map((layer) => layer.id)) + 1;
        const kind = String(args?.kind ?? "Lighting") as TimelineLayerSummary["kind"];
        await persistChildTimeline(childCueId, (child) => {
          const baseLayers = effectiveTimelineLayers(child.layers);
          return {
            ...child,
            layers: [...baseLayers, {
              id: layerId,
              label: String(args?.label ?? `${kind} Layer`),
              order: baseLayers.length,
              muted: false,
              locked: false,
              solo: false,
              kind,
            }],
          };
        });
        return layerId as T;
      }
      if (command === "update_timeline_layer") {
        const layer = args?.layer as TimelineLayerSummary;
        await persistChildTimeline(childCueId, (child) => ({
          ...child,
          layers: effectiveTimelineLayers(child.layers)
            .map((candidate) => candidate.id === layer.id ? layer : candidate),
        }));
        return undefined as T;
      }
      if (command === "remove_timeline_layer") {
        const layerId = Number(args?.layerId);
        const reassignToLayerId = args?.reassignToLayerId === null || args?.reassignToLayerId === undefined
          ? null
          : Number(args.reassignToLayerId);
        await persistChildTimeline(childCueId, (child) => {
          const baseLayers = effectiveTimelineLayers(child.layers);
          const source = baseLayers.find((layer) => layer.id === layerId);
          if (!source) throw new Error(`Timeline layer ${layerId} was not found`);
          if (source.locked) {
            throw new Error(`Timeline layer ${layerId} is locked; unlock it before removal`);
          }
          if (baseLayers.length <= 1) throw new Error("Timeline must contain at least one layer");
          const events = child.events ?? [];
          const audioClips = child.audio_clips ?? [];
          const hasEvents = events.some((event) => timelineLayerIdForEvent(baseLayers, event) === layerId);
          const hasAudioClips = audioClips.some((clip) => clip.layer_id === layerId);
          if (reassignToLayerId === layerId) {
            throw new Error("Timeline layer cannot be reassigned to itself");
          }
          const target = reassignToLayerId === null
            ? null
            : baseLayers.find((layer) => layer.id === reassignToLayerId) ?? null;
          if (reassignToLayerId !== null && !target) {
            throw new Error(`Timeline layer reassignment target ${reassignToLayerId} was not found`);
          }
          if (target?.locked) {
            throw new Error(`Timeline layer ${target.id} is locked; unlock it before reassignment`);
          }
          if (target?.kind === "Audio" && hasEvents) {
            throw new Error(`Cue events cannot be reassigned to Audio timeline layer ${target.id}`);
          }
          if (target && target.kind !== "Audio" && hasAudioClips) {
            throw new Error(`Audio clips can only be reassigned to Audio timeline layer ${target.id}`);
          }
          if (!target && (hasEvents || hasAudioClips)) {
            throw new Error(`Timeline layer ${layerId} is not empty; provide a reassign target`);
          }
          const remainingLayers = baseLayers.filter((layer) => layer.id !== layerId);
          return {
            ...child,
            layers: remainingLayers.map((layer, order) => ({ ...layer, order })),
            events: events.map((event) =>
              timelineLayerIdForEvent(baseLayers, event) === layerId && target
                ? { ...event, layer_id: target.id, track: target.kind as TimelineTrackKind }
                : event),
            audio_clips: audioClips.map((clip) =>
              clip.layer_id === layerId && target?.kind === "Audio"
                ? { ...clip, layer_id: target.id }
                : clip),
          };
        });
        return undefined as T;
      }
      if (command === "reorder_timeline_layers") {
        const layerIds = (args?.layerIds as number[]) ?? [];
        const orderById = new Map(layerIds.map((layerId, order) => [layerId, order]));
        await persistChildTimeline(childCueId, (child) => ({
          ...child,
          layers: effectiveTimelineLayers(child.layers).map((layer) => ({
            ...layer,
            order: orderById.get(layer.id) ?? layer.order,
          })),
        }));
        return undefined as T;
      }
    }
    if (viewportFixture !== "timeline-layered") return invoke<T>(command, args);
    if (command === "add_timeline_layer") {
      const layerId = Math.max(1, ...timelineLayers().map((layer) => layer.id)) + 1;
      const kind = String(args?.kind ?? "Lighting") as TimelineLayerSummary["kind"];
      setSnapshot((current) => ({
        ...current,
        timeline: {
          ...current.timeline,
          layers: [...(current.timeline.layers ?? []), {
            id: layerId,
            label: String(args?.label ?? `${kind} Layer`),
            order: current.timeline.layers?.length ?? 0,
            muted: false,
            locked: false,
            solo: false,
            kind,
          }],
        },
      }));
      return layerId as T;
    }
    if (command === "update_timeline_layer") {
      const layer = args?.layer as TimelineLayerSummary;
      setSnapshot((current) => ({
        ...current,
        timeline: {
          ...current.timeline,
          layers: (current.timeline.layers ?? []).map((candidate) => candidate.id === layer.id ? layer : candidate),
        },
      }));
      return undefined as T;
    }
    if (command === "remove_timeline_layer") {
      const layerId = Number(args?.layerId);
      const reassignToLayerId = args?.reassignToLayerId === null || args?.reassignToLayerId === undefined
        ? null
        : Number(args.reassignToLayerId);
      const currentLayers = timelineLayers();
      const source = currentLayers.find((layer) => layer.id === layerId);
      if (!source) throw new Error(`Timeline layer ${layerId} was not found`);
      if (reassignToLayerId === layerId) throw new Error("Timeline layer cannot be reassigned to itself");
      if (source.locked) throw new Error(`Timeline layer ${layerId} is locked; unlock it before removal`);
      if (currentLayers.length <= 1) throw new Error("Timeline must contain at least one layer");
      const affectedEvents = activeTimeline().events.filter((event) =>
        timelineLayerIdForEvent(currentLayers, event) === layerId);
      if (affectedEvents.length > 0 && reassignToLayerId === null) {
        throw new Error(`Timeline layer ${layerId} is not empty; provide a reassign target`);
      }
      const target = reassignToLayerId === null
        ? null
        : currentLayers.find((layer) => layer.id === reassignToLayerId) ?? null;
      if (reassignToLayerId !== null && !target) {
        throw new Error(`Timeline layer reassignment target ${reassignToLayerId} was not found`);
      }
      if (target?.locked) {
        throw new Error(`Timeline layer ${target.id} is locked; unlock it before reassignment`);
      }
      if (target?.kind === "Audio" && affectedEvents.length > 0) {
        throw new Error(`Cue events cannot be reassigned to Audio timeline layer ${target.id}`);
      }
      setSnapshot((current) => ({
        ...current,
        timeline: {
          ...current.timeline,
          layers: (current.timeline.layers ?? []).filter((layer) => layer.id !== layerId),
          events: current.timeline.events.map((event) =>
            timelineLayerIdForEvent(currentLayers, event) === layerId && target !== null
            ? { ...event, layer_id: target.id, track: target.kind as TimelineTrackKind }
            : event),
        },
      }));
      return undefined as T;
    }
    if (command === "reorder_timeline_layers") {
      const layerIds = (args?.layerIds as number[]) ?? [];
      const orderById = new Map(layerIds.map((layerId, order) => [layerId, order]));
      setSnapshot((current) => ({
        ...current,
        timeline: {
          ...current.timeline,
          layers: (current.timeline.layers ?? []).map((layer) => ({
            ...layer,
            order: orderById.get(layer.id) ?? layer.order,
          })),
        },
      }));
      return undefined as T;
    }
    return invoke<T>(command, args);
  };

  const timelineLayerController = createTimelineLayerController({
    layers: timelineLayers,
    invoke: invokeTimelineLayerCommand,
    setMessage,
    refreshSnapshot: () => viewportFixture === "timeline-layered"
      ? Promise.resolve(snapshot())
      : refreshSnapshot(),
  });

  const removeTimelineLayer = async (layerId: number, reassignToLayerId: number | null) => {
    const layers = timelineLayers();
    const affectedEvents = activeTimeline().events.filter((event) =>
      timelineLayerIdForEvent(layers, event) === layerId);
    const target = reassignToLayerId === null
      ? null
      : layers.find((layer) => layer.id === reassignToLayerId) ?? null;
    const targetTrack: TimelineTrackKind | null = target?.kind === "Lighting" || target?.kind === "Video"
      ? target.kind
      : null;
    const removed = await timelineLayerController.remove(layerId, reassignToLayerId);
    if (!removed || affectedEvents.length === 0 || target === null || targetTrack === null) return;
    setTimelineEventDrafts((current) => {
      const next = { ...current };
      for (const event of affectedEvents) {
        const draft = current[event.id];
        if (!draft) continue;
        next[event.id] = {
          ...draft,
          layer_id: target.id,
          track: targetTrack,
        };
      }
      return next;
    });
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

  createEffect(() => {
    const timeline = activeTimeline();
    syncTimelineEventDrafts(timeline.events);
    syncTimelineAutomationDrafts(timeline.automations);
    syncTimelineVideoAutomationDrafts(timeline.video_automations);
  });

  createEffect(() => {
    const cueId = timelineChildCueId();
    if (cueId !== null && !timelineChildCue()) setTimelineChildCueId(null);
  });

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

  const projectStateSignature = (nextSnapshot = snapshot()) =>
    `${projectSnapshotSignature(nextSnapshot)}|operator:${JSON.stringify(operatorPolicy())}`;
  const markProjectClean = (nextSnapshot = snapshot()) => {
    setCleanProjectSignature(projectStateSignature(nextSnapshot));
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
    if (isfEventPulseBusy()) {
      setMessage("Wait for the active FX Event pulse to finish before Undo.");
      return;
    }
    if (!projectHistoryStatus().can_undo) {
      setMessage("Nothing to undo.");
      return;
    }
    if (!confirmDiscardTimelineEditorDrafts()) {
      setMessage("Undo canceled; unsaved Timeline edits were kept.");
      return;
    }
    if (viewportFixture === "scene-matrix") {
      const entry = viewportSceneMatrixBankMoveUndo();
      if (!entry) return;
      setSnapshot((current) => ({
        ...current,
        cues: structuredClone(entry.beforeCues),
      }));
      setViewportSceneMatrixBankMoveUndo(null);
      setProjectHistoryStatus({
        can_undo: false,
        can_redo: false,
        undo_depth: 0,
        redo_depth: 0,
        undo_label: null,
        redo_label: null,
      });
      setMessage("Undid Move Cue Between Scene Banks.");
      return;
    }
    if (viewportFixture === "edit-live") {
      const undoEntries = viewportControlEditUndo();
      const entry = undoEntries[undoEntries.length - 1];
      if (!entry) return;
      const nextUndo = undoEntries.slice(0, -1);
      const nextRedo = [...viewportControlEditRedo(), entry];
      setSnapshot((current) => ({
        ...current,
        cues: current.cues.map((cue) =>
          cue.id === entry.cueId
            ? { ...cue, targets: cloneCueTargets(entry.beforeTargets) }
            : cue
        ),
      }));
      setViewportControlEditUndo(nextUndo);
      setViewportControlEditRedo(nextRedo);
      setProjectHistoryStatus({
        can_undo: nextUndo.length > 0,
        can_redo: true,
        undo_depth: nextUndo.length,
        redo_depth: nextRedo.length,
        undo_label: nextUndo.length > 0 ? "Update Cue From Current" : null,
        redo_label: "Update Cue From Current",
      });
      setMessage("Undid Update Cue From Current.");
      return;
    }
    try {
      const status = await invoke<ProjectHistoryStatus>("undo_project_transaction");
      setProjectHistoryStatus(status);
      await refreshSnapshot(true, true);
      await refreshVideoPreviewDiagnostics(true);
      setMessage(`Undid ${status.redo_label ?? "last edit"}.`);
    } catch (error) {
      setMessage(`Undo failed: ${String(error)}`);
    }
  };

  const redoProject = async () => {
    if (isfEventPulseBusy()) {
      setMessage("Wait for the active FX Event pulse to finish before Redo.");
      return;
    }
    if (!projectHistoryStatus().can_redo) {
      setMessage("Nothing to redo.");
      return;
    }
    if (!confirmDiscardTimelineEditorDrafts()) {
      setMessage("Redo canceled; unsaved Timeline edits were kept.");
      return;
    }
    if (viewportFixture === "edit-live") {
      const redoEntries = viewportControlEditRedo();
      const entry = redoEntries[redoEntries.length - 1];
      if (!entry) return;
      const nextRedo = redoEntries.slice(0, -1);
      const nextUndo = [...viewportControlEditUndo(), entry];
      setSnapshot((current) => ({
        ...current,
        cues: current.cues.map((cue) =>
          cue.id === entry.cueId
            ? { ...cue, targets: cloneCueTargets(entry.afterTargets) }
            : cue
        ),
      }));
      setViewportControlEditUndo(nextUndo);
      setViewportControlEditRedo(nextRedo);
      setProjectHistoryStatus({
        can_undo: true,
        can_redo: nextRedo.length > 0,
        undo_depth: nextUndo.length,
        redo_depth: nextRedo.length,
        undo_label: "Update Cue From Current",
        redo_label: nextRedo.length > 0 ? "Update Cue From Current" : null,
      });
      setMessage("Redid Update Cue From Current.");
      return;
    }
    try {
      const status = await invoke<ProjectHistoryStatus>("redo_project_transaction");
      setProjectHistoryStatus(status);
      await refreshSnapshot(true, true);
      await refreshVideoPreviewDiagnostics(true);
      setMessage(`Redid ${status.undo_label ?? "last edit"}.`);
    } catch (error) {
      setMessage(`Redo failed: ${String(error)}`);
    }
  };

  const saveProjectRecovery = async () => {
    if (!isTauriRuntime() || (!projectDirty() && !timelineEventEditorDirty())) {
      return;
    }
    const sceneBlockDrafts = dirtyTimelineEventDrafts();
    const signature = `${projectStateSignature(snapshot())}|scene-block-drafts:${JSON.stringify(
      Object.entries(sceneBlockDrafts).sort(([left], [right]) => Number(left) - Number(right)),
    )}`;
    try {
      if (signature !== lastRecoverySignature) {
        const project = await invoke<ProjectFile>("get_project_checkpoint");
        const checkpoint = createProjectRecoveryCheckpoint(
          project,
          currentProjectPath(),
          signature,
          sceneBlockDrafts,
        );
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
    if (!projectDirty() && !timelineEditorDirty()) {
      return true;
    }
    const changeKind = projectDirty() && timelineEditorDirty()
      ? "project changes and Timeline edits"
      : timelineEditorDirty()
        ? "Timeline edits"
        : "project changes";
    return window.confirm(`Discard unsaved ${changeKind} and ${actionLabel}?`);
  };

  const applyEngineSnapshot = (
    next: EngineSnapshot,
    syncProjectState = true,
    resetEditorDrafts = false,
  ) => {
    latestEngineSnapshot = next;
    setLiveDmxPreviews(engineDmxPreviews(next));
    setLiveFixtures(snapshotLiveFixtures(next));
    if (resetEditorDrafts) {
      setSelectedTimelineSceneBlockEventId(null);
      setVideoOutputConfigDrafts({});
      setCueMetadataDrafts({});
      setTimelineEventDrafts({});
      setTimelineAutomationDrafts({});
      setTimelineVideoAutomationDrafts({});
      const captureEffects = eligibleCueEffects(next.effects, cueCaptureScope(), {
        fixtures: next.fixtures,
        selectedFixtureId: selectedFixtureId(),
        selectedGroupId: selectedFixtureGroupFilter(),
      });
      setCueEffectCaptureTargets(currentCueEffectTargets(captureEffects));
      setCueEffectCaptureStateOverrideIds([]);
      lastCueEffectEligibilityIds = captureEffects.map((effect) => effect.id).join(",");
      lastCueEffectEligibilityStates = captureEffects
        .map((effect) => `${effect.id}:${effect.enabled ? 1 : 0}`)
        .join(",");
    }
    setSnapshot(next);
    if (resetEditorDrafts) {
      setTimelineViewportState((current) => reconcileTimelineViewportState(
        current,
        timelineOverviewContentEndMsForTimeline(next.timeline),
        { project_replaced: true },
      ));
    }
    if (syncProjectState) {
      const signature = projectStateSignature(next);
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
      syncCueMetadataDrafts(next.cues, next.effects);
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

  type SnapshotRefreshWaiter = {
    syncProjectState: boolean;
    resetEditorDrafts: boolean;
    resolve: (snapshot: EngineSnapshot | null) => void;
  };
  const pendingFullSnapshotRefreshes: SnapshotRefreshWaiter[] = [];
  let fullSnapshotRefreshRunning = false;
  const runFullSnapshotRefreshes = async () => {
    if (fullSnapshotRefreshRunning) return;
    fullSnapshotRefreshRunning = true;
    try {
      while (pendingFullSnapshotRefreshes.length > 0) {
        const batch = pendingFullSnapshotRefreshes.splice(0);
        snapshotRequestGuard.beginFull();
        let next: EngineSnapshot | null = null;
        try {
          next = await invoke<EngineSnapshot>("get_snapshot");
          setSnapshotRevision(null);
          if (batch.some((waiter) => waiter.resetEditorDrafts)) {
            await refreshOperatorPolicy(true);
          }
          applyEngineSnapshot(
            next,
            batch.some((waiter) => waiter.syncProjectState),
            batch.some((waiter) => waiter.resetEditorDrafts),
          );
        } catch (error) {
          setMessage(String(error));
        } finally {
          snapshotRequestGuard.finishFull();
        }
        for (const waiter of batch) waiter.resolve(next);
      }
    } finally {
      fullSnapshotRefreshRunning = false;
    }
  };
  const refreshSnapshot = (
    syncProjectState = true,
    resetEditorDrafts = false,
  ): Promise<EngineSnapshot | null> =>
    new Promise((resolve) => {
      pendingFullSnapshotRefreshes.push({ syncProjectState, resetEditorDrafts, resolve });
      void runFullSnapshotRefreshes();
    });

  let lastSnapshotUiApplyAt = 0;
  const applyEngineSnapshotSyncResponse = (
    response: EngineSnapshotSyncResponse,
    syncUiState: boolean,
  ) => {
    const next = mergeEngineSnapshotSyncResponse(latestEngineSnapshot, response);
    latestEngineSnapshot = next;
    setLiveDmxPreviews(engineDmxPreviews(next));
    setLiveFixtures(snapshotLiveFixtures(next));
    setSnapshotRevision(response.revision);
    if (syncUiState || response.full) {
      applyEngineSnapshot(next, false);
      lastSnapshotUiApplyAt = performance.now();
    }
    return next;
  };
  if (viewportFixture === "mapping-live-snapshot") {
    applyEngineSnapshotSyncResponse({
      revision: 28,
      delta: {
        active_cue_id: viewportFixtureData.mappingLiveSnapshotAmberCue.id,
      },
    }, true);
  }
  const refreshSnapshotDelta = async (syncUiState = true) => {
    const requestGeneration = snapshotRequestGuard.beginDelta();
    if (requestGeneration === null) return null;
    try {
      const response = await invoke<EngineSnapshotSyncResponse>("get_snapshot_delta", {
        clientRevision: snapshotRevision(),
      });
      if (!snapshotRequestGuard.canApplyDelta(requestGeneration)) {
        return null;
      }
      return applyEngineSnapshotSyncResponse(response, syncUiState);
    } catch (error) {
      if (!snapshotRequestGuard.canApplyDelta(requestGeneration)) {
        return null;
      }
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
    const uiIntervalMs = workspaceTab() === "setup" ? 1_000 : 250;
    const intervalMs = document.hidden
      ? 2_000
      : liveMappingStageVisible()
        ? liveDmxPollIntervalMs
        : uiIntervalMs;
    snapshotPollTimer = window.setTimeout(async () => {
      const syncUiState = performance.now() - lastSnapshotUiApplyAt >= uiIntervalMs;
      await refreshSnapshotDelta(syncUiState);
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
          if (event.payload.paths.some(isTimelineAudioFilePath)) {
            setProjectDropState(null);
            return;
          }
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
        const audioPath = event.payload.paths.find(isTimelineAudioFilePath);
        if (audioPath) {
          const scale = Math.max(1, window.devicePixelRatio || 1);
          const target = document
            .elementFromPoint(event.payload.position.x / scale, event.payload.position.y / scale)
            ?.closest<HTMLElement>("[data-timeline-audio-layer-id], [data-timeline-layer-kind='Audio'][data-timeline-layer-id]");
          const layerId = Number(
            target?.dataset.timelineAudioLayerId
              ?? target?.dataset.timelineLayerId,
          );
          if (!Number.isFinite(layerId)) {
            setMessage("Drop Audio Clips on the Audio section header or an Audio lane.");
            return;
          }
          void addTimelineAudioClipPath(layerId, audioPath);
          return;
        }
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

  const pendingControlEditTargets = new Map<number, Map<string, ControlEditCaptureTarget>>();
  let controlEditWriteTimer: number | null = null;
  let controlEditFlushChain = Promise.resolve();
  const cloneCueTargets = (targets: CueSummary["targets"]): CueSummary["targets"] =>
    targets.map((target) => ({
      fixture_id: target.fixture_id,
      values: target.values.map((value) => ({ ...value })),
    }));
  const controlEditCueIdForWrite = () =>
    workspaceTab() === "control"
    && controlMode() === "edit"
    && editDeskSurface() === "attributes"
    && controlFaderWriteMode() === "edit"
      ? selectedSceneCue()?.id ?? null
      : null;
  const controlEditTargetKey = (target: ControlEditCaptureTarget) =>
    target.kind === "selectedFixture"
      ? `fixture:${target.fixtureId}`
      : `group:${target.groupId}`;
  const captureViewportCueTargets = (
    current: EngineSnapshot,
    cue: CueSummary,
    captureTargets: ControlEditCaptureTarget[],
  ) => {
    const fixtureIds = new Set<number>();
    for (const target of captureTargets) {
      if (target.kind === "selectedFixture") {
        fixtureIds.add(target.fixtureId);
      } else {
        for (const fixture of current.fixtures) {
          if (fixture.group_ids.some((groupId) => groupMatches(groupId, target.groupId))) {
            fixtureIds.add(fixture.id);
          }
        }
      }
    }
    const nextTargets = cloneCueTargets(cue.targets);
    const localValues = faderValues();
    for (const fixtureId of fixtureIds) {
      const fixture = current.fixtures.find((candidate) => candidate.id === fixtureId);
      if (!fixture) continue;
      const captured = {
        fixture_id: fixture.id,
        values: fixture.controls.map((control) => ({
          attribute: control.attribute,
          value:
            localValues[`${fixture.id}:${control.attribute}`]
            ?? fixture.attribute_values.find((entry) => entry.attribute === control.attribute)?.value
            ?? control.default_value,
        })),
      };
      const existingIndex = nextTargets.findIndex((target) => target.fixture_id === fixture.id);
      if (existingIndex >= 0) nextTargets[existingIndex] = captured;
      else nextTargets.push(captured);
    }
    return nextTargets;
  };
  const updateViewportControlEditLook = (
    cue: CueSummary,
    captureTargets: ControlEditCaptureTarget[],
  ) => {
    let historyEntry: ViewportControlEditHistoryEntry | null = null;
    setSnapshot((current) => {
      const currentCue = current.cues.find((candidate) => candidate.id === cue.id);
      if (!currentCue) return current;
      const beforeTargets = cloneCueTargets(currentCue.targets);
      const afterTargets = captureViewportCueTargets(current, currentCue, captureTargets);
      if (JSON.stringify(beforeTargets) === JSON.stringify(afterTargets)) return current;
      historyEntry = {
        cueId: cue.id,
        beforeTargets,
        afterTargets: cloneCueTargets(afterTargets),
      };
      return {
        ...current,
        cues: current.cues.map((candidate) =>
          candidate.id === cue.id ? { ...candidate, targets: afterTargets } : candidate
        ),
      };
    });
    if (!historyEntry) return;
    const nextUndo = [...viewportControlEditUndo(), historyEntry];
    setViewportControlEditUndo(nextUndo);
    setViewportControlEditRedo([]);
    setProjectHistoryStatus({
      can_undo: true,
      can_redo: false,
      undo_depth: nextUndo.length,
      redo_depth: 0,
      undo_label: "Update Cue From Current",
      redo_label: null,
    });
    setProjectDirty(true);
  };
  const runControlEditLookUpdate = async (
    cueId: number,
    captureTargets: ControlEditCaptureTarget[],
  ) => {
    const cue = snapshot().cues.find((candidate) => candidate.id === cueId);
    if (!cue || captureTargets.length === 0) return;
    if (viewportFixture === "edit-live") {
      updateViewportControlEditLook(cue, captureTargets);
      setMessage(`Updated cue ${cue.id} look from the current Store Scope.`);
      return;
    }
    if (!isTauriRuntime()) return;
    if (!operatorCommandAllowed(activeOperatorLockMode, "update_cue_from_current", true)) {
      throw new Error(
        activeOperatorLockMode === "Full"
          ? "Operator Full Lock allows only status reads and emergency blackout controls."
          : "Operator Partial Lock blocks programming and project replacement commands.",
      );
    }
    const scopeSignature = captureTargets
      .map(controlEditTargetKey)
      .sort((left, right) => left.localeCompare(right))
      .join(",");
    const transactionId = await tauriInvoke<number>("begin_project_transaction", {
      label: projectMutationLabel("update_cue_from_current"),
      coalesceKey: `update_cue_from_current:cue:${cue.id}:${scopeSignature}`,
    });
    try {
      for (const captureScope of captureTargets) {
        await tauriInvoke("update_cue_from_current", {
          cueId: cue.id,
          label: cue.label,
          fadeMs: cue.fade_ms,
          captureScope,
        });
      }
      const status = await tauriInvoke<ProjectHistoryStatus>("commit_project_transaction", {
        transactionId,
      });
      window.dispatchEvent(
        new CustomEvent<ProjectHistoryStatus>(projectHistoryChangedEvent, { detail: status }),
      );
      setMessage(`Updated cue ${cue.id} look from the current Store Scope.`);
      await refreshSnapshot();
    } catch (error) {
      await tauriInvoke("cancel_project_transaction", { transactionId }).catch(() => undefined);
      throw error;
    }
  };
  const flushControlEditLookUpdates = () => {
    if (controlEditWriteTimer !== null) {
      window.clearTimeout(controlEditWriteTimer);
      controlEditWriteTimer = null;
    }
    controlEditFlushChain = controlEditFlushChain.then(async () => {
      const queued = [...pendingControlEditTargets.entries()].map(([cueId, targets]) => ({
        cueId,
        targets: [...targets.values()],
      }));
      pendingControlEditTargets.clear();
      for (const entry of queued) {
        try {
          await runControlEditLookUpdate(entry.cueId, entry.targets);
        } catch (error) {
          setMessage(String(error));
        }
      }
    });
    return controlEditFlushChain;
  };
  const queueControlEditLookUpdate = (
    cueId: number | null,
    captureTarget: ControlEditCaptureTarget,
  ) => {
    if (cueId === null) return;
    const targets = pendingControlEditTargets.get(cueId) ?? new Map<string, ControlEditCaptureTarget>();
    targets.set(controlEditTargetKey(captureTarget), captureTarget);
    pendingControlEditTargets.set(cueId, targets);
    if (controlEditWriteTimer !== null) window.clearTimeout(controlEditWriteTimer);
    controlEditWriteTimer = window.setTimeout(() => {
      void flushControlEditLookUpdates();
    }, 240);
  };
  const changeControlFaderWriteMode = (mode: ControlFaderWriteMode) => {
    if (controlFaderWriteMode() === mode) return;
    if (mode === "live") void flushControlEditLookUpdates();
    setControlFaderWriteMode(mode);
  };
  createEffect(() => {
    const armed =
      workspaceTab() === "control"
      && controlMode() === "edit"
      && editDeskSurface() === "attributes"
      && controlFaderWriteMode() === "edit";
    if (!armed) return;
    const flush = () => {
      if (pendingControlEditTargets.size > 0) void flushControlEditLookUpdates();
    };
    window.addEventListener("pointerup", flush, true);
    window.addEventListener("change", flush, true);
    window.addEventListener("keyup", flush, true);
    onCleanup(() => {
      window.removeEventListener("pointerup", flush, true);
      window.removeEventListener("change", flush, true);
      window.removeEventListener("keyup", flush, true);
    });
  });
  onCleanup(() => {
    if (controlEditWriteTimer !== null) window.clearTimeout(controlEditWriteTimer);
  });

  const setAttribute = async (fixtureId: number, attribute: string, value: number) => {
    const editCueId = controlEditCueIdForWrite();
    setFaderValues((current) => ({ ...current, [`${fixtureId}:${attribute}`]: value }));
    if (viewportFixture === "edit-live" || viewportFixture === "color-wheel") {
      queueControlEditLookUpdate(editCueId, { kind: "selectedFixture", fixtureId });
      return;
    }
    try {
      await invoke(snapshot().programmer.enabled ? "set_programmer_attribute" : "set_attribute", {
        fixtureId,
        attribute,
        value,
      });
      queueControlEditLookUpdate(editCueId, { kind: "selectedFixture", fixtureId });
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setGroupAttribute = async (groupId: string, attribute: string, value: number) => {
    const editCueId = controlEditCueIdForWrite();
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
    if (viewportFixture === "edit-live" || viewportFixture === "color-wheel") {
      queueControlEditLookUpdate(editCueId, { kind: "selectedGroup", groupId });
      return;
    }
    try {
      await invoke(snapshot().programmer.enabled ? "set_programmer_group_attribute" : "set_group_attribute", {
        groupId,
        attribute,
        value,
      });
      queueControlEditLookUpdate(editCueId, { kind: "selectedGroup", groupId });
    } catch (error) {
      setMessage(String(error));
    }
  };

  const pickedFixtureTypeGroup = (typeKey: string) =>
    pickedFixtureTypeGroups().find((group) => group.key === typeKey);

  const setFixtureTypeControlValue = (
    typeKey: string,
    attribute: string,
    value: number,
  ) => {
    const nextValue = clampDmxValue(value);
    const group = pickedFixtureTypeGroup(typeKey);
    if (!group) {
      return;
    }
    for (const fixture of group.fixtures) {
      const control = fixtureControlForAttribute(fixture, attribute);
      if (control) {
        void setAttribute(fixture.id, control.attribute, nextValue);
      }
    }
  };

  const resetFixtureTypeControlValue = (
    typeKey: string,
    attribute: string,
  ) => {
    const group = pickedFixtureTypeGroup(typeKey);
    if (!group) {
      return;
    }
    for (const fixture of group.fixtures) {
      const control = fixtureControlForAttribute(fixture, attribute);
      if (control) {
        void setAttribute(fixture.id, control.attribute, control.default_value);
      }
    }
  };

  const setFixtureTypeColor = (typeKey: string, hexColor: string) => {
    const group = pickedFixtureTypeGroup(typeKey);
    const color = normalizeHexColor(hexColor);
    if (!group || !color) {
      return;
    }
    const values = {
      red: Number.parseInt(color.slice(1, 3), 16) * 257,
      green: Number.parseInt(color.slice(3, 5), 16) * 257,
      blue: Number.parseInt(color.slice(5, 7), 16) * 257,
    };
    for (const fixture of group.fixtures) {
      const red = findControlAttribute(fixture, colorCandidates.red);
      const green = findControlAttribute(fixture, colorCandidates.green);
      const blue = findControlAttribute(fixture, colorCandidates.blue);
      if (!red || !green || !blue) {
        continue;
      }
      void setAttribute(fixture.id, red, values.red);
      void setAttribute(fixture.id, green, values.green);
      void setAttribute(fixture.id, blue, values.blue);
    }
  };

  const setFixtureTypePosition = (
    typeKey: string,
    panAttribute: string,
    tiltAttribute: string,
    panValue: number,
    tiltValue: number,
    usesFixtureLimits: boolean,
  ) => {
    const group = pickedFixtureTypeGroup(typeKey);
    if (!group) {
      return;
    }
    for (const fixture of group.fixtures) {
      const pan = fixtureControlForAttribute(fixture, panAttribute);
      const tilt = fixtureControlForAttribute(fixture, tiltAttribute);
      if (!pan || !tilt) {
        continue;
      }
      const source = usesFixtureLimits
        ? sourcePanTiltValues(fixture, panValue, tiltValue)
        : { pan: clampDmxValue(panValue), tilt: clampDmxValue(tiltValue) };
      void setAttribute(fixture.id, pan.attribute, source.pan);
      void setAttribute(fixture.id, tilt.attribute, source.tilt);
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
      const next = await refreshSnapshot(true, true);
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
      const next = await refreshSnapshot(true, true);
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
    if (timelineEditorDirty()) {
      setMessage("Save each modified Timeline row before saving the project.", "timeline-drafts-block-save");
      return;
    }
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
    if (timelineEditorDirty()) {
      setMessage("Save each modified Timeline row before using Save As.", "timeline-drafts-block-save-as");
      return;
    }
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
    const warnings = result.warnings ?? [];
    const warningLabel = warnings.length === 1 ? "1 validation warning" : `${warnings.length} validation warnings`;
    return warnings.length > 0
      ? `Loaded project ${result.path} (${profileLabel}; ${warningLabel}): ${warnings.join(" | ")}`
      : `Loaded project ${result.path} (${profileLabel})`;
  };

  const applyLoadedProjectResult = async (result: ProjectLoadResult, currentPath: string | null) => {
    await resetProjectHistory();
    setCurrentProjectPath(currentPath);
    rememberRecentProjectPath(currentPath);
    setMessage(loadedProjectMessage(result));
    const next = await refreshSnapshot(true, true);
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

  const importDaslightProject = async () => {
    if (daslightProjectImportBusy()) {
      return;
    }
    if (!confirmDiscardProjectChanges("import a Daslight Project (.dvc)")) {
      setMessage("Daslight Project import canceled.");
      return;
    }
    setDaslightProjectImportBusy(true);
    setMessage("Importing Daslight Project...", "daslight-project-import-busy");
    try {
      const report = await invoke<DvcImportReport | null>("import_daslight_project", { path: null });
      if (!report) {
        setMessage("Daslight Project import canceled.");
        return;
      }
      await resetProjectHistory();
      setCurrentProjectPath(null);
      setWorkspaceTab("setup");
      setSetupSubTab("patch");
      const next = await refreshSnapshot(true, true);
      if (next) {
        setCleanProjectSignature("__syndocal_dvc_import_unsaved__");
        setProjectDirty(true);
      }
      clearProjectRecovery();
      setDvcImportReport(report);
      setMessage(
        `Imported Daslight Project (.dvc): ${report.summary.fixtures} fixtures, ${report.summary.cues} cues. Save As to create a Syndocal Project (.sdc).`,
      );
    } catch (error) {
      setMessage(`Daslight Project import failed: ${String(error)}`);
    } finally {
      setDaslightProjectImportBusy(false);
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
      const next = await refreshSnapshot(true, true);
      if (next) {
        const recoveredTimelineEventDrafts = checkpoint.editor_drafts?.timeline_events ?? {};
        setTimelineEventDrafts(reconcileTimelineEventDrafts(next.timeline.events, recoveredTimelineEventDrafts));
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
      const next = await refreshSnapshot(true, true);
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
      const next = await refreshSnapshot(true, true);
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
      const afterSmoke = await refreshSnapshot(true, true);
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
    const editCueId = controlEditCueIdForWrite();
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

    if (viewportFixture === "edit-live") {
      queueControlEditLookUpdate(
        editCueId,
        groupId
          ? { kind: "selectedGroup", groupId }
          : { kind: "selectedFixture", fixtureId: fixture.id },
      );
      return;
    }
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
      queueControlEditLookUpdate(
        editCueId,
        groupId
          ? { kind: "selectedGroup", groupId }
          : { kind: "selectedFixture", fixtureId: fixture.id },
      );
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

  const touchRgbAttributes = (fixture: PatchedFixtureSummary) => {
    const normalized = (value: string) => value.toLowerCase().replace(/[^a-z0-9]/g, "");
    const find = (candidates: string[]) => fixture.controls.find((control) =>
      candidates.includes(normalized(control.attribute)),
    )?.attribute;
    const red = find(["colorred", "coloraddr", "red"]);
    const green = find(["colorgreen", "coloraddg", "green"]);
    const blue = find(["colorblue", "coloraddb", "blue"]);
    return red && green && blue ? { red, green, blue } : null;
  };

  const setTouchSurfaceLayout = async (surface: TouchSurfaceSummary) => {
    const previous = snapshot().touch_surface ?? { pages: [] };
    setSnapshot((current) => ({ ...current, touch_surface: surface }));
    setProjectDirty(true);
    if (!isTauriRuntime()) return;
    try {
      await invoke("set_touch_surface", { surface });
      await refreshSnapshot();
    } catch (error) {
      setSnapshot((current) => ({ ...current, touch_surface: previous }));
      setMessage(String(error));
    }
  };

  const triggerTouchBinding = async (binding: TouchControlBinding) => {
    switch (binding.kind) {
      case "cue":
        await triggerCue(binding.cue_id);
        break;
      case "cue_next":
        await triggerNextCue();
        break;
      case "cue_previous":
        await triggerPreviousCue();
        break;
      case "cue_fade_pause":
        await setCueFadePaused(!snapshot().active_fade?.paused);
        break;
      case "blackout":
        await setBlackout(!snapshot().blackout);
        break;
      case "video_blackout":
        await setVideoBlackout(!snapshot().video.blackout);
        break;
      case "all_blackout":
        await setAllBlackout(!(snapshot().blackout && snapshot().video.blackout));
        break;
      default:
        break;
    }
  };

  const setTouchBindingValue = async (binding: TouchControlBinding, normalizedValue: number) => {
    const value = Math.round(clamp01(normalizedValue) * 65_535);
    switch (binding.kind) {
      case "fixture_attribute":
        await setAttribute(binding.fixture_id, binding.attribute, value);
        break;
      case "group_attribute":
        await setGroupAttribute(binding.group_id, binding.attribute, value);
        break;
      case "selected_fixture_attribute": {
        const groupId = selectedFixtureGroupFilter();
        const fixtureId = selectedFixtureId();
        if (groupId) await setGroupAttribute(groupId, binding.attribute, value);
        else if (fixtureId !== null) await setAttribute(fixtureId, binding.attribute, value);
        break;
      }
      case "group_submaster":
        await setGroupSubmaster(binding.group_id, clamp01(normalizedValue));
        break;
      case "lighting_master":
        await setLightingMaster(clamp01(normalizedValue));
        break;
      case "video_master":
        await setVideoMasterOpacity(clamp01(normalizedValue));
        break;
      default:
        break;
    }
  };

  const setTouchBindingColor = async (binding: TouchControlBinding, color: string) => {
    if (binding.kind === "selected_fixture_color") {
      await setFixtureColor(color);
      return;
    }
    const fixture = binding.kind === "fixture_color"
      ? snapshot().fixtures.find((candidate) => candidate.id === binding.fixture_id)
      : binding.kind === "group_color"
        ? snapshot().fixtures.find((candidate) => candidate.group_ids.includes(binding.group_id))
        : undefined;
    if (!fixture) return;
    const attributes = touchRgbAttributes(fixture);
    if (!attributes) return;
    const values = {
      [attributes.red]: Number.parseInt(color.slice(1, 3), 16) * 257,
      [attributes.green]: Number.parseInt(color.slice(3, 5), 16) * 257,
      [attributes.blue]: Number.parseInt(color.slice(5, 7), 16) * 257,
    };
    for (const [attribute, value] of Object.entries(values)) {
      if (binding.kind === "fixture_color") {
        await setAttribute(binding.fixture_id, attribute, value);
      } else if (binding.kind === "group_color") {
        await setGroupAttribute(binding.group_id, attribute, value);
      }
    }
  };

  const setTouchBindingXy = async (binding: TouchControlBinding, x: number, y: number) => {
    const pan = Math.round(clamp01(x) * 65_535);
    const tilt = Math.round(clamp01(y) * 65_535);
    switch (binding.kind) {
      case "fixture_pan_tilt":
        await setAttribute(binding.fixture_id, binding.pan_attribute, pan);
        await setAttribute(binding.fixture_id, binding.tilt_attribute, tilt);
        break;
      case "group_pan_tilt":
        await setGroupAttribute(binding.group_id, binding.pan_attribute, pan);
        await setGroupAttribute(binding.group_id, binding.tilt_attribute, tilt);
        break;
      case "selected_fixture_pan_tilt":
        await setPanTiltValues(pan, tilt);
        break;
      default:
        break;
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

  const rememberColorWheelPickerEntry = (
    control: AttributeControl,
    fn: NonNullable<AttributeControl["functions"]>[number],
  ) => {
    const entry = colorWheelEntries().find((candidate) =>
      candidate.control.attribute === control.attribute &&
      candidate.fn.dmx_from === fn.dmx_from &&
      candidate.fn.dmx_to === fn.dmx_to
    );
    if (entry) {
      setColorWheelPickerSelection({
        targetKey: colorWheelPickerTargetKey(),
        color: entry.color,
        entryKey: colorWheelEntryKey(entry),
      });
    }
  };

  const applyColorWheelFunction = (
    control: AttributeControl,
    fn: NonNullable<AttributeControl["functions"]>[number],
  ) => {
    rememberColorWheelPickerEntry(control, fn);
    void applyChannelFunction(control, fn);
  };

  const setColorWheelPickerColor = (hexColor: string) => {
    const color = normalizeHexColor(hexColor);
    const entry = color ? nearestColorWheelEntry(colorWheelEntries(), color) : undefined;
    if (!color || !entry) {
      return;
    }
    setColorWheelPickerSelection({
      targetKey: colorWheelPickerTargetKey(),
      color,
      entryKey: colorWheelEntryKey(entry),
    });
    void applyChannelFunction(entry.control, entry.fn);
  };

  const setColorWheelPickerHsvValue = (updates: Partial<{ hue: number; saturation: number; value: number }>) => {
    const hsv = colorWheelPickerHsv();
    const { red, green, blue } = hsvToRgb(
      updates.hue ?? hsv.hue,
      updates.saturation ?? hsv.saturation,
      updates.value ?? hsv.value,
    );
    setColorWheelPickerColor(rgbToHex(red, green, blue));
  };

  const setColorWheelPickerFromPointer = (event: PointerEvent) => {
    const bounds = (event.currentTarget as HTMLElement).getBoundingClientRect();
    const x = clamp01((event.clientX - bounds.left) / Math.max(1, bounds.width));
    const y = clamp01((event.clientY - bounds.top) / Math.max(1, bounds.height));
    const current = colorWheelPickerHsv();
    setColorWheelPickerHsvValue({
      hue: x * 360,
      saturation: current.saturation > 0.05 ? current.saturation : 1,
      value: 1 - y,
    });
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
    if (viewportFixture) {
      setSnapshot((current) => ({
        ...current,
        fixtures: current.fixtures.map((fixture) => ({
          ...fixture,
          soloed: fixture.group_ids.some((fixtureGroup) => groupMatches(fixtureGroup, groupId))
            ? enabled
            : fixture.soloed,
        })),
      }));
      return;
    }
    try {
      await invoke("set_group_solo", { groupId, enabled });
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setGroupStrobe = async (groupId: string, rateHz: number) => {
    if (viewportFixture) {
      setSnapshot((current) => ({
        ...current,
        submasters: current.submasters.map((submaster) => submaster.group_id === groupId
          ? { ...submaster, strobe_hz: rateHz }
          : submaster),
      }));
      return;
    }
    try {
      await invoke("set_group_strobe", { groupId, rateHz });
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const useCatalogProfile = (imported: FixtureProfileSummary, loadedMessage: string) => {
    setProfile(imported);
    setGdtfPath(imported.source_path);
    setSelectedMode(imported.dmx_modes[0]?.name ?? "");
    setMessage(profileLoadMessage(loadedMessage, imported));
  };

  const repairCatalogFixtureProfile = async (
    fixtureId: number,
    profilePath: string,
    modeName: string | null,
  ) => {
    await invoke("repair_fixture_profile", { fixtureId, profilePath, modeName });
    await refreshSnapshot();
    setMessage(`Repaired fixture ${fixtureId} profile source with an exact DMX layout match.`);
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
      const childCueId = timelineChildCueId();
      const analysis = await invoke<AudioAnalysisSummary | null>(
        childCueId === null ? "analyze_audio_file" : "select_timeline_audio_clip_file",
      );
      if (!analysis) {
        setMessage("Audio analysis canceled.");
        return;
      }
      if (analysis.estimated_bpm) {
        setBpmDraft(analysis.estimated_bpm.toFixed(1));
      }
      if (childCueId === null) {
        await refreshSnapshot();
      } else {
        await persistChildTimeline(childCueId, (child) => ({
          ...child,
          audio: analysis,
          duration_ms: Math.max(child.duration_ms ?? 0, analysis.duration_ms),
        }));
      }
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
      const childCueId = timelineChildCueId();
      if (childCueId !== null) {
        await persistChildTimeline(childCueId, (child) => ({
          ...child,
          audio: null,
          audio_clips: [],
        }));
        setMessage("Cleared child timeline audio.");
        return;
      }
      await invoke("clear_timeline_audio");
      await refreshSnapshot();
      setMessage("Cleared audio analysis.");
    } catch (error) {
      setMessage(String(error));
    }
  };

  const commitTimelineAudioClip = async (
    layerId: number,
    analysis: AudioAnalysisSummary,
    startMs: number,
  ) => {
    const layer = timelineLayers().find((candidate) => candidate.id === layerId);
    if (!layer || layer.kind !== "Audio") {
      setMessage("Audio Clips can only be placed on Audio lanes.");
      return;
    }
    if (layer.locked) {
      setMessage(`Timeline layer ${layer.label} is locked. Unlock it before adding Audio Clips.`);
      return;
    }
    const childCueId = timelineChildCueId();
    if (childCueId !== null) {
      const id = Math.max(
        0,
        ...(snapshot().timeline.audio_clips ?? []).map((clip) => clip.id),
        ...snapshot().cues.flatMap((cue) => cue.child_timeline?.audio_clips?.map((clip) => clip.id) ?? []),
      ) + 1;
      await persistChildTimeline(childCueId, (child) => ({
        ...child,
        audio: child.audio ?? analysis,
        audio_clips: [...(child.audio_clips ?? []), {
          id,
          layer_id: layerId,
          path: analysis.path,
          start_ms: startMs,
          offset_ms: 0,
          duration_ms: Math.max(1, analysis.duration_ms),
          gain: 1,
          fade_in_ms: 0,
          fade_out_ms: 0,
        }],
        duration_ms: Math.max(child.duration_ms ?? 0, startMs + Math.max(1, analysis.duration_ms)),
      }));
      setMessage(`Added child Audio Clip ${analysis.path}.`);
      return;
    }
    if (viewportFixture === "timeline-layered") {
      const id = Math.max(0, ...(activeTimeline().audio_clips ?? []).map((clip) => clip.id)) + 1;
      setSnapshot((current) => ({
        ...current,
        timeline: {
          ...current.timeline,
          audio: analysis,
          audio_clips: [...(current.timeline.audio_clips ?? []), {
            id,
            layer_id: layerId,
            path: analysis.path,
            start_ms: startMs,
            offset_ms: 0,
            duration_ms: Math.max(1, analysis.duration_ms),
            gain: 1,
            fade_in_ms: 0,
            fade_out_ms: 0,
          }],
        },
      }));
      setMessage(`Added Audio Clip ${analysis.path}.`);
      return;
    }
    await invoke<number>("add_timeline_audio_clip", {
      layerId,
      path: analysis.path,
      startMs,
      offsetMs: 0,
      durationMs: Math.max(1, analysis.duration_ms),
      gain: 1,
      fadeInMs: 0,
      fadeOutMs: 0,
    });
    await refreshSnapshot();
    setMessage(`Added Audio Clip ${analysis.path}.`);
  };

  const addTimelineAudioClip = async (layerId: number) => {
    try {
      if (viewportFixture === "timeline-layered") {
        const analysis = audioAnalysis() ?? {
          path: "C:/fixture/audio/added-loop.wav",
          sample_rate: 48_000,
          channels: 2,
          duration_ms: 4_000,
          estimated_bpm: 120,
          waveform: [],
          spectrum: [],
          beats: [],
        };
        await commitTimelineAudioClip(layerId, analysis, snapTimeMs(activeTimeline().position_ms));
        return;
      }
      const analysis = await invoke<AudioAnalysisSummary | null>("select_timeline_audio_clip_file");
      if (!analysis) {
        setMessage("Add Audio Clip canceled.");
        return;
      }
      await commitTimelineAudioClip(layerId, analysis, snapTimeMs(activeTimeline().position_ms));
    } catch (error) {
      setMessage(String(error));
    }
  };

  const addTimelineAudioClipPath = async (layerId: number, path: string) => {
    try {
      const analysis = await invoke<AudioAnalysisSummary>("analyze_timeline_audio_clip_path", { path });
      await commitTimelineAudioClip(layerId, analysis, snapTimeMs(activeTimeline().position_ms));
    } catch (error) {
      setMessage(String(error));
    }
  };

  const updateTimelineAudioClip = async (clip: TimelineAudioClipSummary) => {
    try {
      const childCueId = timelineChildCueId();
      if (childCueId !== null) {
        const durationMs = Math.max(1, Math.round(clip.duration_ms));
        const fadeInMs = Math.min(durationMs, Math.max(0, Math.round(clip.fade_in_ms)));
        const normalized = {
          ...clip,
          start_ms: Math.max(0, Math.round(clip.start_ms)),
          offset_ms: Math.max(0, Math.round(clip.offset_ms)),
          duration_ms: durationMs,
          gain: Math.min(2, Math.max(0, Number.isFinite(clip.gain) ? clip.gain : 1)),
          fade_in_ms: fadeInMs,
          fade_out_ms: Math.min(durationMs - fadeInMs, Math.max(0, Math.round(clip.fade_out_ms))),
        };
        await persistChildTimeline(childCueId, (child) => ({
          ...child,
          audio_clips: (child.audio_clips ?? []).map((candidate) => candidate.id === clip.id
            ? normalized
            : candidate),
          duration_ms: Math.max(child.duration_ms ?? 0, normalized.start_ms + normalized.duration_ms),
        }));
        return;
      }
      if (viewportFixture === "timeline-layered") {
        const durationMs = Math.max(1, Math.round(clip.duration_ms));
        const fadeInMs = Math.min(durationMs, Math.max(0, Math.round(clip.fade_in_ms)));
        const normalized = {
          ...clip,
          start_ms: Math.max(0, Math.round(clip.start_ms)),
          offset_ms: Math.max(0, Math.round(clip.offset_ms)),
          duration_ms: durationMs,
          gain: Math.min(2, Math.max(0, Number.isFinite(clip.gain) ? clip.gain : 1)),
          fade_in_ms: fadeInMs,
          fade_out_ms: Math.min(durationMs - fadeInMs, Math.max(0, Math.round(clip.fade_out_ms))),
        };
        setSnapshot((current) => ({
          ...current,
          timeline: {
            ...current.timeline,
            audio_clips: (current.timeline.audio_clips ?? []).map((candidate) =>
              candidate.id === clip.id ? normalized : candidate),
          },
        }));
        return;
      }
      await invoke("update_timeline_audio_clip", {
        id: clip.id,
        layerId: clip.layer_id,
        path: clip.path,
        startMs: clip.start_ms,
        offsetMs: clip.offset_ms,
        durationMs: clip.duration_ms,
        gain: clip.gain,
        fadeInMs: clip.fade_in_ms,
        fadeOutMs: clip.fade_out_ms,
      });
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const removeTimelineAudioClip = async (clipId: number) => {
    try {
      const childCueId = timelineChildCueId();
      if (childCueId !== null) {
        await persistChildTimeline(childCueId, (child) => ({
          ...child,
          audio_clips: (child.audio_clips ?? []).filter((clip) => clip.id !== clipId),
        }));
        return;
      }
      if (viewportFixture === "timeline-layered") {
        setSnapshot((current) => ({
          ...current,
          timeline: {
            ...current.timeline,
            audio_clips: (current.timeline.audio_clips ?? []).filter((clip) => clip.id !== clipId),
          },
        }));
        return;
      }
      await invoke("remove_timeline_audio_clip", { clipId });
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setTimelineAudioMaster = async (offsetMs: number, muted: boolean) => {
    try {
      if (timelineChildCueId() !== null) {
        setMessage("Child timeline audio follows its clip gain and the parent transport.");
        return;
      }
      if (viewportFixture === "timeline-layered") {
        setSnapshot((current) => ({
          ...current,
          timeline: { ...current.timeline, audio_offset_ms: offsetMs, audio_muted: muted },
        }));
        return;
      }
      await invoke("set_timeline_audio_master", { offsetMs, muted });
      await refreshSnapshot();
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
    const authoredBeatsError = cueAuthoredBeatsError();
    if (authoredBeatsError) {
      setMessage(authoredBeatsError);
      return;
    }
    try {
      const cueId = await invoke<number>("create_cue_from_current", {
        label: cueLabel(),
        fadeMs: cueFadeMs(),
        authoredBeats: cueAuthoredBeats(),
        captureScope,
        cueListId: selectedCueList().id,
        effectTargets: cueEffectCaptureTargets(),
      });
      setCueLabel(`Cue ${snapshot().cues.length + 2}`);
      setSelectedSceneCueId(cueId);
      setSelectedSceneEffectId(null);
      setSceneSettingsSurface("contents");
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
    if (viewportFixture === "scene-matrix") {
      setSnapshot((current) => {
        const cue = current.cues.find((candidate) => candidate.id === cueId);
        if (!cue) return current;
        const activeGroupCueIds = { ...(current.active_group_cue_ids ?? {}) };
        if (cue.group_id) activeGroupCueIds[cue.group_id] = cue.id;
        return {
          ...current,
          active_cue_id: cue.id,
          active_group_cue_ids: activeGroupCueIds,
          // T17 reset rule: a fresh trigger drops the latched live modifier.
          cue_live_modifiers: (current.cue_live_modifiers ?? []).filter(
            (state) => state.cue_id !== cueId,
          ),
        };
      });
      setMessage(`Triggered cue ${cueId}`);
      return;
    }
    try {
      await invoke("trigger_cue", { cueId });
      setMessage(`Triggered cue ${cueId}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const releaseCueById = async (cueId: number) => {
    if (viewportFixture === "scene-matrix") {
      setSnapshot((current) => {
        const cue = current.cues.find((candidate) => candidate.id === cueId);
        if (!cue) return current;
        const activeGroupCueIds = { ...(current.active_group_cue_ids ?? {}) };
        if (cue.group_id && activeGroupCueIds[cue.group_id] === cueId) {
          delete activeGroupCueIds[cue.group_id];
        }
        return {
          ...current,
          active_cue_id: current.active_cue_id === cueId ? null : current.active_cue_id,
          active_group_cue_ids: activeGroupCueIds,
          // T17 reset rule: release drops the latched live modifier.
          cue_live_modifiers: (current.cue_live_modifiers ?? []).filter(
            (state) => state.cue_id !== cueId,
          ),
        };
      });
      setMessage(`Released cue ${cueId}`);
      return;
    }
    try {
      await invoke("release_cue", { cueId });
      setMessage(`Released cue ${cueId}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setCueLiveModifierLive = async (
    cueId: number,
    speed: number,
    size: number,
    phase: number,
    direction: CueLiveDirection,
    segment: number,
  ) => {
    if (viewportFixture === "scene-matrix") {
      setSnapshot((current) => ({
        ...current,
        cue_live_modifiers: [
          ...(current.cue_live_modifiers ?? []).filter((state) => state.cue_id !== cueId),
          { cue_id: cueId, speed, size, phase, direction, segment },
        ],
      }));
      return;
    }
    try {
      await invoke("set_cue_live_modifier", {
        cueId,
        speed,
        size,
        phase,
        direction,
        segment,
      });
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const clearCueLiveModifierLive = async (cueId: number) => {
    if (viewportFixture === "scene-matrix") {
      setSnapshot((current) => ({
        ...current,
        cue_live_modifiers: (current.cue_live_modifiers ?? []).filter(
          (state) => state.cue_id !== cueId,
        ),
      }));
      return;
    }
    try {
      await invoke("clear_cue_live_modifier", { cueId });
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const updateCue = async (
    cueId: number,
    label: string,
    fadeMs: number,
  ) => {
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
      await invoke("update_cue_from_current", {
        cueId,
        label,
        fadeMs: normalizedFadeMs,
        captureScope,
        effectTargets: cueEffectCaptureTargets(),
      });
      setMessage(`Updated cue ${cueId} look from the current Store Scope.`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setCueEffectTargets = async (cueId: number, effectTargets: CueEffectTarget[]) => {
    try {
      await invoke("set_cue_effect_targets", { cueId, effectTargets });
      setMessage(`Saved cue ${cueId} Effect Recall only.`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setCueSteps = async (cueId: number, steps: CueStepSummary[]) => {
    try {
      await invoke("set_cue_steps", { cueId, steps });
      setMessage(`Saved ${steps.length} Static step(s) for cue ${cueId}.`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setCueMetadata = async (cue: CueSummary) => {
    const draft = cueMetadataDraft(cue);
    const fadeMs = Math.max(0, Math.round(Number.isFinite(draft.fade_ms) ? draft.fade_ms : cue.fade_ms));
    const authoredBeats = draft.authored_beats === null || !Number.isFinite(draft.authored_beats)
      ? null
      : draft.authored_beats;
    if (authoredBeats !== null && (authoredBeats < 0.25 || authoredBeats > 1024)) {
      setMessage("Authored beats must be from 0.25 to 1024.");
      return;
    }
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
        groupId: draft.group_id,
        recallMode: draft.recall_mode,
        fadeMs,
        authoredBeats,
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
          group_id: draft.group_id,
          recall_mode: draft.recall_mode,
          fade_ms: fadeMs,
          authored_beats: authoredBeats,
          pre_wait_ms: preWaitMs,
          follow_ms: followMs,
          ifcb_timing: ifcbTiming,
          parts,
          mark: draft.mark,
          mib_fixture_ids: mibFixtureIds,
          tracking: draft.tracking,
          notes: draft.notes,
          effect_targets: draft.effect_targets.map((target) => ({ ...target })),
        },
      }));
      setMessage(`Saved cue ${cue.id} details only. Effect Recall is unchanged.`);
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

  const runSceneMatrixBankMoveTransaction = async (
    cue: CueSummary,
    groupId: string | null,
    delta: -1 | 1,
    stepCount: number,
  ) => {
    if (!isTauriRuntime()) return null;
    if (!operatorCommandAllowed(activeOperatorLockMode, "set_cue_metadata", true)) {
      throw new Error(
        activeOperatorLockMode === "Full"
          ? "Operator Full Lock allows only status reads and emergency blackout controls."
          : "Operator Partial Lock blocks programming and project replacement commands.",
      );
    }
    const transactionId = await tauriInvoke<number>("begin_project_transaction", {
      label: "Move Cue Between Scene Banks",
      coalesceKey: `scene_matrix_bank_move:cue:${cue.id}`,
    });
    try {
      await tauriInvoke("set_cue_metadata", sceneMatrixCueMetadataArgs(cue, groupId));
      for (let step = 0; step < stepCount; step += 1) {
        await tauriInvoke("move_cue", { cueId: cue.id, delta });
      }
      const status = await tauriInvoke<ProjectHistoryStatus>("commit_project_transaction", {
        transactionId,
      });
      window.dispatchEvent(
        new CustomEvent<ProjectHistoryStatus>(projectHistoryChangedEvent, { detail: status }),
      );
      return status;
    } catch (error) {
      await tauriInvoke("cancel_project_transaction", { transactionId }).catch(() => undefined);
      throw error;
    }
  };

  const reorderSceneMatrixCue = async (
    sourceCueId: number,
    targetCueId: number | null,
    position: "before" | "after",
    targetGroupId: string | null,
  ) => {
    if (sourceCueId === targetCueId) return;
    const cues = snapshot().cues;
    const sourceCue = cues.find((cue) => cue.id === sourceCueId);
    const targetCue = targetCueId === null
      ? null
      : cues.find((cue) => cue.id === targetCueId) ?? null;
    if (!sourceCue || (targetCueId !== null && !targetCue)) {
      setMessage("Drop the Cue on a scene cell, bank column, or Timeline lane.");
      return;
    }
    const sourceGroupId = sourceCue.group_id?.trim() || null;
    const normalizedTargetGroupId = targetGroupId?.trim() || null;
    const crossesBank = sourceGroupId !== normalizedTargetGroupId;
    if (!crossesBank && !targetCue) {
      setMessage("Drop the Cue on another cell in this column or on a Timeline lane.");
      return;
    }
    if (targetCue && sourceCue.cue_list_id !== targetCue.cue_list_id) {
      setMessage("Cues must share a Cue List before they can be reordered.");
      return;
    }

    const cueList = cues.filter((cue) => cue.cue_list_id === sourceCue.cue_list_id);
    const sourceIndex = cueList.findIndex((cue) => cue.id === sourceCueId);
    const withoutSource = cueList.filter((cue) => cue.id !== sourceCueId);
    const targetIndex = targetCue
      ? withoutSource.findIndex((cue) => cue.id === targetCue.id)
      : -1;
    if (sourceIndex < 0 || (targetCue && targetIndex < 0)) return;
    const insertionIndex = targetCue
      ? targetIndex + (position === "after" ? 1 : 0)
      : sourceIndex;
    const moveCount = insertionIndex - sourceIndex;
    if (!crossesBank && moveCount === 0) return;
    const delta: -1 | 1 = moveCount < 0 ? -1 : 1;
    const stepCount = Math.abs(moveCount);

    try {
      if (crossesBank) {
        await runSceneMatrixBankMoveTransaction(
          sourceCue,
          normalizedTargetGroupId,
          delta,
          stepCount,
        );
      } else if (viewportFixture !== "scene-matrix" && viewportFixture !== "workspace-operator") {
        for (let step = 0; step < stepCount; step += 1) {
          await invoke("move_cue", { cueId: sourceCueId, delta });
        }
      }

      if (viewportFixture === "scene-matrix" || viewportFixture === "workspace-operator") {
        const beforeCues = structuredClone(cues);
        const afterCues = applySceneMatrixCueMove(
          cues,
          sourceCueId,
          normalizedTargetGroupId,
          crossesBank,
          delta,
          stepCount,
        );
        setSnapshot((current) => ({ ...current, cues: structuredClone(afterCues) }));
        if (crossesBank && viewportFixture === "scene-matrix") {
          setViewportSceneMatrixBankMoveUndo({ beforeCues, afterCues: structuredClone(afterCues) });
          if (!isTauriRuntime()) {
            setProjectHistoryStatus({
              can_undo: true,
              can_redo: false,
              undo_depth: 1,
              redo_depth: 0,
              undo_label: "Move Cue Between Scene Banks",
              redo_label: null,
            });
          }
        }
      } else {
        await refreshSnapshot();
      }
      if (crossesBank) {
        setCueMetadataDrafts((current) => {
          const draft = current[sourceCueId];
          return draft
            ? { ...current, [sourceCueId]: { ...draft, group_id: normalizedTargetGroupId } }
            : current;
        });
      }
      setMessage(crossesBank ? "Cue moved between Scene Matrix banks." : "Cue order updated.");
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
    if (cue) {
      const linkedPlacements = snapshot().timeline.events.filter((event) => event.cue_id === cueId);
      const linkedPlacementIds = new Set(linkedPlacements.map((event) => event.id));
      const incomingJumpCount = snapshot().timeline.events.filter((event) =>
        event.cue_id !== cueId &&
        event.jump_to_event_id !== null &&
        event.jump_to_event_id !== undefined &&
        linkedPlacementIds.has(event.jump_to_event_id),
      ).length;
      if (!confirmCueRemoval(cue.label, linkedPlacements.length, incomingJumpCount)) {
        return;
      }
    }
    try {
      await invoke("remove_cue", { cueId });
      setMessage(`Removed cue ${cueId}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const openTimelineSourceCue = (cueId: number) => {
    const cue = snapshot().cues.find((candidate) => candidate.id === cueId);
    if (!cue) {
      setMessage(`Source Cue ${cueId} is no longer available.`);
      return;
    }
    setWorkspaceTab("control");
    setControlMode("live");
    setTimelineDeskSurface("show");
    setTimelineContextDrawer("cue");
    setSelectedCueListId(cue.cue_list_id);
    setRevealedSourceCueId(cue.id);
    setRevealedSourceCueRevision((revision) => revision + 1);
    setMessage(`Opened source Cue ${cue.cue_number || cue.id} in Cue List ${cue.cue_list_id}.`);
  };

  const openOrCreateSuperScene = async (cueId: number) => {
    const cue = snapshot().cues.find((candidate) => candidate.id === cueId);
    if (!cue) {
      setMessage(`Source Cue ${cueId} is no longer available.`);
      return false;
    }
    if (!confirmDiscardTimelineEditorDrafts()) {
      setMessage("Super Scene open canceled; unsaved Timeline edits were kept.");
      return false;
    }
    if (!cue.child_timeline) {
      const childTimeline: ChildTimelineSummary = {
        layers: [
          { id: 1, label: "Child Audio", order: 0, muted: false, locked: false, solo: false, kind: "Audio" },
          { id: 2, label: "Child Lighting", order: 1, muted: false, locked: false, solo: false, kind: "Lighting" },
          { id: 3, label: "Child Video", order: 2, muted: false, locked: false, solo: false, kind: "Video" },
        ],
        events: [],
        automations: [],
        video_automations: [],
        audio: null,
        audio_clips: [],
        duration_ms: Math.max(1_000, timelineBlockDurationMs()),
      };
      const localFixture = viewportFixture === "timeline-layered"
        || viewportFixture === "scene-block-large"
        || viewportFixture === "scene-block-hour"
        || viewportFixture === "fx-visual";
      if (localFixture) {
        setSnapshot((current) => ({
          ...current,
          cues: current.cues.map((candidate) => candidate.id === cueId
            ? { ...candidate, child_timeline: childTimeline }
            : candidate),
        }));
      } else {
        await invoke("set_cue_child_timeline", { cueId, childTimeline });
        await refreshSnapshot();
      }
    }
    setTimelineChildCueId(cueId);
    setSelectedTimelineSceneBlockEventId(null);
    setTimelineEventDrafts({});
    setTimelineAutomationDrafts({});
    setTimelineVideoAutomationDrafts({});
    setTimelineViewportState(createTimelineViewportState(
      Math.max(1, snapshot().cues.find((candidate) => candidate.id === cueId)?.child_timeline?.duration_ms ?? 1),
    ));
    setMessage(`Opened Super Scene ${cue.label}.`);
    return true;
  };

  const effectChooserCueId = () => {
    const cues = snapshot().cues;
    const activeCueId = snapshot().active_cue_id;
    if (activeCueId !== null && activeCueId !== undefined && cues.some((cue) => cue.id === activeCueId)) {
      return activeCueId;
    }
    const explicitTimelineCueId = timelineCueId();
    if (explicitTimelineCueId !== null && cues.some((cue) => cue.id === explicitTimelineCueId)) {
      return explicitTimelineCueId;
    }
    return cues[0]?.id ?? null;
  };

  const selectEffectFamily = async (family: EffectChooserFamily) => {
    const cueId = effectChooserCueId();
    if (family === "STEPS") {
      if (cueId === null) {
        setMessage("Add or select a Cue before opening STEPS.");
        return;
      }
      openTimelineSourceCue(cueId);
      return;
    }
    if (family === "SUPER SCENE") {
      if (cueId === null) {
        setMessage("Add or select a Cue before opening SUPER SCENE.");
        return;
      }
      const opened = await openOrCreateSuperScene(cueId);
      if (!opened) return;
      setWorkspaceTab("control");
      setControlMode("live");
      setTimelineDeskSurface("show");
      setTimelineContextDrawer("none");
      return;
    }
    const recipeFamily = family as EffectRecipeFamily;
    setEffectChooserFamily(recipeFamily);
    const nextType: EffectKind = family === "COLOR FX"
      ? "Color"
      : family === "COLOUR MAPPINGS"
        ? "ColorMapping"
      : family === "CHASER FX"
        ? "Chaser"
        : family === "MOVE FX"
          ? "Move"
          : family === "VALUE FX"
            ? "Value"
          : family === "MAPPINGS"
            ? "Mapping"
            : family === "CURVE FX"
              ? "Curve"
              : "Lfo";
    selectEffectType(nextType, true);
  };

  const exitSuperScene = () => {
    if (!confirmDiscardTimelineEditorDrafts()) {
      setMessage("Show timeline open canceled; unsaved Timeline edits were kept.");
      return;
    }
    setTimelineChildCueId(null);
    setSelectedTimelineSceneBlockEventId(null);
    setTimelineEventDrafts({});
    setTimelineAutomationDrafts({});
    setTimelineVideoAutomationDrafts({});
    fitTimelineOverview();
    setMessage("Opened Show timeline.");
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
    const timeline = activeTimeline();
    const cueEventUpdates = buildTimelineSceneBlockSnapPlacements(
      timeline.events,
      timelineEventDraft,
      snapTimeMs,
      (eventId) => timeline.events.some((event) => event.id === eventId),
      snappedTimeBeats,
    );
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
      const childCueId = timelineChildCueId();
      if (childCueId !== null) {
        const eventUpdates = new Map(cueEventUpdates.map(({ request }) => [request.event_id, request]));
        const lightingUpdates = new Map(lightingAutomationUpdates.map(({ automation, keyframes }) => [automation.id, keyframes]));
        const videoUpdates = new Map(videoAutomationUpdates.map(({ automation, keyframes }) => [automation.id, keyframes]));
        await persistChildTimeline(childCueId, (child) => ({
          ...child,
          events: (child.events ?? []).map((event) => {
            const request = eventUpdates.get(event.id);
            return request ? {
              ...event,
              cue_id: request.cue_id,
              time_ms: request.time_ms,
              time_beats: request.time_beats,
              track: request.track,
              layer_id: request.layer_id,
              duration_ms: request.duration_ms,
              duration_beats: request.duration_beats,
              conform_to_tempo: request.conform_to_tempo,
              loop_fill: request.loop_fill,
              fade_in_ms: request.fade_in_ms,
              fade_out_ms: request.fade_out_ms,
              loop_count: request.loop_count,
              jump_to_event_id: request.jump_to_event_id,
            } : event;
          }),
          automations: (child.automations ?? []).map((automation) => ({
            ...automation,
            keyframes: lightingUpdates.get(automation.id) ?? automation.keyframes,
          })),
          video_automations: (child.video_automations ?? []).map((automation) => ({
            ...automation,
            keyframes: videoUpdates.get(automation.id) ?? automation.keyframes,
          })),
        }));
      } else {
        await invoke("snap_timeline_items", {
          request: {
            event_placements: cueEventUpdates.map(({ request }) => request),
            lighting_automations: lightingAutomationUpdates.map(({ automation, keyframes }) => ({
              automation_id: automation.id,
              keyframes,
            })),
            video_automations: videoAutomationUpdates.map(({ automation, keyframes }) => ({
              automation_id: automation.id,
              keyframes,
            })),
          },
        });
      }
      setTimelineEventDrafts((current) => {
        const next = { ...current };
        for (const { event, draft } of cueEventUpdates) {
          next[event.id] = draft;
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
      if (childCueId === null) await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const addTimelineCueEventAt = timelineSceneBlocks.addAt;

  const addTimelineCueEvent = async () => {
    await addTimelineCueEventAt(selectedTimelineCueId(), timelineEventTimeMs(), timelineTrack());
  };

  const addTimelineCueEventAtPlayhead = async () => {
    await addTimelineCueEventAt(selectedTimelineCueId(), activeTimeline().position_ms, timelineTrack(), false);
  };

  const setTimelineCueEvent = timelineSceneBlocks.save;
  const moveTimelineCueEvent = timelineSceneBlocks.moveBy;
  const moveTimelineCueEventToPlacement = timelineSceneBlocks.moveToPlacement;
  const resizeTimelineCueEventToTime = timelineSceneBlocks.resizeToTime;
  const setTimelineCueEventFade = timelineSceneBlocks.setFade;
  const placeArmedTimelineCue = async (
    cueId: number,
    timeMs: number,
    layerId: number,
    durationMs: number,
    stretchMode: "RATE" | "WINDOW",
    snapEnabled: boolean,
  ) => timelineSceneBlocks.addAt(cueId, timeMs, "Lighting", false, {
    layerId,
    durationMs,
    authoredBeats: snapshot().cues.find((cue) => cue.id === cueId)?.authored_beats ?? null,
    stretchMode,
    snapEnabled,
    loopCount: 1,
  });

  const beginTimelineCueDrag = (
    cue: CueSummary,
    point: TimelineCueDragPoint,
    sourceSurface: TimelineCueDragState["source_surface"] = "cue-editor",
  ) => {
    setTimelineCueDrag({
      cue_id: cue.id,
      cue_label: cue.label,
      source_surface: sourceSurface,
      pointer_id: point.pointerId,
      start_client_x: point.clientX,
      start_client_y: point.clientY,
      client_x: point.clientX,
      client_y: point.clientY,
      moved: false,
    });
    setTimelineDeskSurface("show");
    setTimelineContextDrawer("none");
  };

  const moveTimelineCueDrag = (point: TimelineCueDragPoint) => {
    setTimelineCueDrag((drag) => drag && drag.pointer_id === point.pointerId
      ? updateTimelineCueDrag(drag, point)
      : drag);
  };

  const endTimelineCueDrag = async (point: TimelineCueDragPoint, moved: boolean, canceled: boolean) => {
    const current = timelineCueDrag();
    const drag = current && current.pointer_id === point.pointerId
      ? updateTimelineCueDrag(current, point)
      : null;
    setTimelineCueDrag(null);
    if (!drag || canceled || (!drag.moved && !moved)) return;
    const hitElement = document.elementFromPoint(point.clientX, point.clientY);
    const matrixColumn = drag.source_surface === "scene-matrix"
      ? hitElement?.closest<HTMLElement>("[data-scene-matrix-column]")
      : null;
    if (matrixColumn) {
      const matrixTarget = hitElement?.closest<HTMLElement>("[data-scene-matrix-cue-id]");
      const rawTargetCueId = Number(matrixTarget?.dataset.sceneMatrixCueId);
      const targetCueId = matrixTarget && Number.isFinite(rawTargetCueId)
        ? rawTargetCueId
        : null;
      const targetRect = matrixTarget?.getBoundingClientRect();
      const columnId = matrixColumn.dataset.sceneMatrixColumn ?? "Show";
      await reorderSceneMatrixCue(
        drag.cue_id,
        targetCueId,
        targetRect && point.clientY < targetRect.top + targetRect.height / 2 ? "before" : "after",
        columnId === "Show" ? null : columnId,
      );
      return;
    }
    const target = hitElement
      ?.closest<HTMLElement>("[data-timeline-layer-id]");
    const layerId = Number(target?.dataset.timelineLayerId);
    const layer = timelineLayers().find((candidate) => candidate.id === layerId);
    const canvas = document.querySelector<SVGSVGElement>(".timelineOverview");
    if (!layer || !canvas) {
      setMessage("Drop the Cue inside a timeline lane.");
      return;
    }
    if (layer.locked) {
      setMessage(`Cannot drop Cue on locked layer ${layer.label}.`);
      return;
    }
    if (layer.kind === "Audio") {
      setMessage("Audio lanes accept audio files in a later tranche; Cue drops require a Lighting lane.");
      return;
    }
    if (layer.kind === "Video") {
      setMessage("Video lanes accept video sources in a later tranche; Cue drops require a Lighting lane.");
      return;
    }
    const cue = snapshotCues().find((candidate) => candidate.id === drag.cue_id);
    if (!cue) {
      setMessage(`Cue ${drag.cue_id} was not found.`);
      return;
    }
    const canvasRect = canvas.getBoundingClientRect();
    if (point.clientX < canvasRect.left || point.clientX > canvasRect.right) {
      setMessage("Drop the Cue inside the timeline canvas.");
      return;
    }
    const rawTimeMs = timelineVisibleRatioToTimeMs(
      Math.min(1, Math.max(0, (point.clientX - canvasRect.left) / Math.max(1, canvasRect.width))),
      timelineVisibleWindow(),
    );
    await addTimelineCueEventAt(cue.id, rawTimeMs, "Lighting", false, {
      layerId: layer.id,
      durationMs: cueDropDurationMs(cue, snapshot().clock.bpm, timelineBlockDurationMs()),
    });
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
  const timelineEditingSnapshot = (): EngineSnapshot => ({ ...snapshot(), timeline: activeTimeline() });
  const refreshTimelineEditingSnapshot = () => timelineChildCueId() === null
    ? refreshSnapshot()
    : Promise.resolve(snapshot());

  const {
    moveTimelineAutomationRangeToTime,
    resizeTimelineAutomationRangeToTime,
    moveTimelineAutomationKeyframeToTime,
  } = createTimelineOverviewAutomationController({
    snapshot: timelineEditingSnapshot,
    snapTimeMs,
    invoke: invokeTimelineEditingCommand,
    setTimelineAutomationDrafts,
    setTimelineVideoAutomationDrafts,
    setMessage,
    refreshSnapshot: refreshTimelineEditingSnapshot,
  });

  const removeTimelineEvent = timelineSceneBlocks.remove;

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
    snapshot: timelineEditingSnapshot,
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
    invoke: invokeTimelineEditingCommand,
    setTimelineAutomationDrafts,
    setTimelineVideoAutomationDrafts,
    setMessage,
    refreshSnapshot: refreshTimelineEditingSnapshot,
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
    seekTimelineFromOverviewTime,
  } = createTimelineAutomationController({
    snapshot: timelineEditingSnapshot,
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
    invoke: invokeTimelineEditingCommand,
    setMessage,
    refreshSnapshot: refreshTimelineEditingSnapshot,
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
    applyBuiltinVideoIsfEffect,
    setVideoLayerIsfEffect,
    moveVideoLayerIsfEffect,
    removeVideoLayerIsfEffect,
    setVideoLayerIsfEffectEnabled,
    resetVideoLayerIsfEffect,
    setVideoLayerIsfControl,
    triggerVideoLayerIsfEvent,
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
    isIsfEventPulseBusy: isfEventPulseBusy,
    setIsfEventPulseBusy,
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
  let vjPreviewRequestGeneration = 0;
  let vjPreviewPollInFlight: Promise<VjPreviewTransportSummary | null> | null = null;
  const isVjPreviewTransportSummary = (value: unknown): value is VjPreviewTransportSummary => {
    if (!value || typeof value !== "object") return false;
    const candidate = value as Partial<VjPreviewTransportSummary>;
    return (candidate.layer_id === null || typeof candidate.layer_id === "number") &&
      typeof candidate.playing === "boolean" &&
      typeof candidate.position_ms === "number" &&
      (candidate.duration_ms === null || typeof candidate.duration_ms === "number") &&
      typeof candidate.speed === "number" &&
      typeof candidate.loop_enabled === "boolean";
  };
  const applyVjPreviewTransport = (summary: VjPreviewTransportSummary) => {
    setVjPreviewTransport({
      ...emptyVjPreviewTransport(),
      ...summary,
      position_ms: Math.max(0, summary.position_ms),
      duration_ms: summary.duration_ms === null ? null : Math.max(0, summary.duration_ms),
    });
    setVideoPreviewLayerId(summary.layer_id);
  };
  const readVjPreviewTransport = (showError = false): Promise<VjPreviewTransportSummary | null> => {
    if (!isTauriRuntime() || vjPreviewTransportBusy()) return Promise.resolve(null);
    if (vjPreviewPollInFlight) return vjPreviewPollInFlight;
    const requestGeneration = ++vjPreviewRequestGeneration;
    const request = (async () => {
      try {
        const summary = await invoke<VjPreviewTransportSummary>("get_vj_preview_transport");
        if (requestGeneration !== vjPreviewRequestGeneration) return null;
        applyVjPreviewTransport(summary);
        setVjPreviewTransportError(null);
        return summary;
      } catch (error) {
        if (requestGeneration === vjPreviewRequestGeneration && showError) {
          setVjPreviewTransportError(String(error));
        }
        return null;
      }
    })();
    vjPreviewPollInFlight = request;
    void request.finally(() => {
      if (vjPreviewPollInFlight === request) vjPreviewPollInFlight = null;
    });
    return request;
  };
  const runVjPreviewTransportCommand = async (
    command: string,
    args?: Record<string, unknown>,
  ) => {
    if (!isTauriRuntime()) {
      setVjPreviewTransportError(tauriBackendUnavailableMessage);
      return null;
    }
    if (vjPreviewTransportBusy()) return null;
    setVjPreviewTransportBusy(true);
    setVjPreviewTransportError(null);
    ++vjPreviewRequestGeneration;
    const requestGeneration = ++vjPreviewRequestGeneration;
    try {
      const response = await invoke<unknown>(command, args);
      const summary = isVjPreviewTransportSummary(response)
        ? response
        : await invoke<VjPreviewTransportSummary>("get_vj_preview_transport");
      if (requestGeneration !== vjPreviewRequestGeneration) return null;
      applyVjPreviewTransport(summary);
      return summary;
    } catch (error) {
      if (requestGeneration === vjPreviewRequestGeneration) {
        setVjPreviewTransportError(String(error));
      }
      return null;
    } finally {
      if (requestGeneration === vjPreviewRequestGeneration) setVjPreviewTransportBusy(false);
    }
  };
  const stageVjPreviewLayer = async (layerId: number) => {
    const summary = await runVjPreviewTransportCommand("stage_vj_preview_layer", { layerId });
    if (!summary) return false;
    if (summary.layer_id !== layerId) {
      setVjPreviewTransportError(`Preview staged layer ${summary.layer_id ?? "none"}, expected ${layerId}.`);
      return false;
    }
    return true;
  };
  const clearVjPreview = async () => {
    const summary = await runVjPreviewTransportCommand("clear_vj_preview");
    return summary !== null && summary.layer_id === null;
  };
  const setVjPreviewPlaying = async (playing: boolean) => {
    await runVjPreviewTransportCommand("set_vj_preview_playing", { playing });
  };
  const seekVjPreview = async (positionMs: number) => {
    await runVjPreviewTransportCommand("seek_vj_preview", {
      positionMs: Math.max(0, Math.round(positionMs)),
    });
  };
  const setVjPreviewSpeed = async (speed: number) => {
    await runVjPreviewTransportCommand("set_vj_preview_speed", {
      speed: Math.max(-4, Math.min(4, speed)),
    });
  };
  createEffect(() => {
    const active = workspaceTab() === "control" && controlMode() === "mixer";
    if (!isTauriRuntime()) {
      applyVjPreviewTransport(emptyVjPreviewTransport());
      return;
    }
    if (!active || vjPreviewTransportBusy()) return;
    void readVjPreviewTransport(true);
    const intervalId = window.setInterval(() => void readVjPreviewTransport(), 250);
    onCleanup(() => window.clearInterval(intervalId));
  });
  const liveVideoMonitors = createLiveVideoMonitorController({
    invoke,
    backendAvailable: () => isTauriRuntime(),
    active: () => workspaceTab() === "control" && controlMode() === "mixer",
    layerCount: () => snapshot().video.layers.length,
    previewLayerId: videoPreviewLayerId,
    programOutputId: selectedVideoOutputId,
  });
  const liveVideoPreviewLabel = createMemo(() =>
    snapshot().video.layers.find((layer) => layer.id === videoPreviewLayerId())?.label ?? null,
  );
  const liveVideoProgramLabel = createMemo(() =>
    snapshot().video.outputs.find((output) => output.id === selectedVideoOutputId())?.label ?? null,
  );
  const vjFirstRunAvailable = createMemo(() =>
    !vjFirstRunAwaitingSync() &&
    snapshot().video.layers.length === 0 &&
    snapshot().video.outputs.length === 0 &&
    snapshot().video.compositions.every((composition) => composition.id === 1),
  );
  createEffect(() => {
    if (
      vjFirstRunAwaitingSync() &&
      (snapshot().video.layers.length > 0 || snapshot().video.outputs.length > 0)
    ) {
      setVjFirstRunAwaitingSync(false);
    }
  });
  const createFirstRunVjShow = async () => {
    if (vjFirstRunBusy()) return;
    if (!vjFirstRunAvailable()) {
      setMessage("First-run VJ setup is only available for an empty video show.");
      return;
    }
    setVjFirstRunBusy(true);
    setVjFirstRunError(null);
    try {
      const paths = await invoke<string[]>("select_video_source_files", { kind: "File" });
      if (paths.length === 0) {
        setMessage("VJ setup canceled. No project changes were made.");
        return;
      }
      const result = await invoke<VjFirstRunSetupResult>("bootstrap_vj_show", {
        kind: "File",
        paths,
      });
      setVjFirstRunAwaitingSync(true);
      const refreshed = await refreshSnapshot();
      setSelectedVideoOutputId(result.output_id);
      const firstLayerId = result.layer_ids[0] ?? null;
      const previewStaged = firstLayerId === null ? false : await stageVjPreviewLayer(firstLayerId);
      setVideoLabel(`Video Layer ${result.layer_ids.length + 1}`);
      if (!refreshed) {
        const detail = uiLocale() === "ja"
          ? "VJショーは安全に作成されましたが、画面を再同期できませんでした。スナップショット同期で再試行します。"
          : "The VJ show was created safely, but the local view could not refresh. Automatic snapshot sync will retry.";
        setVjFirstRunError(detail);
        setMessage(detail);
        return;
      }
      setVjFirstRunAwaitingSync(false);
      if (!previewStaged) {
        const detail = uiLocale() === "ja"
          ? "VJショーは安全に作成されましたが、最初のクリップをPREVIEWへ送れませんでした。Pボタンで再試行できます。"
          : "The VJ show was created safely, but the first clip could not be staged. Use its P button to retry.";
        setVjFirstRunError(detail);
        setMessage(detail);
        return;
      }
      setMessage(
        `VJ show ready with ${result.layer_ids.length} clip(s). VJ Program remains Off and Blackout until you enable it explicitly.`,
      );
    } catch (error) {
      const detail = String(error);
      setVjFirstRunError(uiLocale() === "ja" ? `VJセットアップに失敗しました。${detail}` : detail);
      setMessage(detail);
    } finally {
      setVjFirstRunBusy(false);
    }
  };
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
      await refreshVideoAudioMonitorStatus(true);
    }
  };
  const stopVideoClipFromGrid = async (layerId: number, fadeMs: number) => {
    const stopped = await stopVideoClip(layerId, fadeMs);
    if (stopped && videoAudioMonitorStatus().active_layer_ids.includes(layerId)) {
      await stopVideoLayerAudioMonitor(layerId);
    }
  };
  const updateAutoVj = async (
    command: "set_auto_vj_config" | "set_auto_vj_armed" | "set_auto_vj_hold",
    args: Record<string, unknown>,
  ) => {
    if (autoVjBusy()) return;
    setAutoVjBusy(true);
    try {
      await invoke(command, args);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    } finally {
      setAutoVjBusy(false);
    }
  };
  const setAutoVjConfig = (config: AutoVjConfig) => {
    const eligibleLayerIds = config.eligible_layer_ids.length > 0
      ? config.eligible_layer_ids
      : snapshot().video.layers.map((layer) => layer.id);
    if (eligibleLayerIds.length === 0) {
      setMessage("Add at least one video clip before configuring Auto VJ.");
      return Promise.resolve();
    }
    return updateAutoVj("set_auto_vj_config", {
      config: { ...config, eligible_layer_ids: eligibleLayerIds },
    });
  };
  const setAutoVjArmed = async (armed: boolean) => {
    if (autoVjBusy()) return;
    setAutoVjBusy(true);
    try {
      if (armed) {
        const current = snapshot().video.auto_vj?.config ?? emptyAutoVjSnapshot.config;
        if (current.eligible_layer_ids.length === 0) {
          const eligibleLayerIds = snapshot().video.layers.map((layer) => layer.id);
          if (eligibleLayerIds.length === 0) {
            setMessage("Add at least one video clip before arming Auto VJ.");
            return;
          }
          await invoke("set_auto_vj_config", {
            config: { ...current, eligible_layer_ids: eligibleLayerIds },
          });
        }
      }
      await invoke("set_auto_vj_armed", { armed });
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    } finally {
      setAutoVjBusy(false);
    }
  };
  const setAutoVjHold = (hold: boolean) =>
    updateAutoVj("set_auto_vj_hold", { hold });
  let desiredProgramAudioHandoffConfig: {
    enabled: boolean;
    volume: number;
    deviceName: string | null;
  } | null = null;
  let desiredProgramAudioHandoffSignature: string | null = null;
  let appliedProgramAudioHandoffSignature: string | null = null;
  let programAudioHandoffSyncInFlight = false;
  let programAudioHandoffDisposed = false;
  const syncProgramAudioHandoffConfig = async () => {
    if (programAudioHandoffSyncInFlight || programAudioHandoffDisposed || !isTauriRuntime()) return;
    programAudioHandoffSyncInFlight = true;
    let failed = false;
    try {
      while (
        !programAudioHandoffDisposed &&
        desiredProgramAudioHandoffConfig &&
        desiredProgramAudioHandoffSignature !== appliedProgramAudioHandoffSignature
      ) {
        const config = desiredProgramAudioHandoffConfig;
        const signature = desiredProgramAudioHandoffSignature;
        await invoke("set_program_audio_handoff_config", config);
        appliedProgramAudioHandoffSignature = signature;
      }
    } catch (error) {
      failed = true;
      if (!programAudioHandoffDisposed) setMessage(String(error));
    } finally {
      programAudioHandoffSyncInFlight = false;
      if (
        !failed &&
        !programAudioHandoffDisposed &&
        desiredProgramAudioHandoffSignature !== appliedProgramAudioHandoffSignature
      ) {
        void syncProgramAudioHandoffConfig();
      }
    }
  };
  createEffect(() => {
    const config = {
      enabled: videoProgramAudioEnabled(),
      volume: videoAudioMonitorVolume(),
      deviceName: selectedAudioOutputDevice().trim() || null,
    };
    desiredProgramAudioHandoffConfig = config;
    desiredProgramAudioHandoffSignature = JSON.stringify(config);
    if (isTauriRuntime()) void syncProgramAudioHandoffConfig();
  });
  onCleanup(() => {
    programAudioHandoffDisposed = true;
  });
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
  let liveAudioInputBackendsEpoch = 0;
  let liveAudioInputBackendsRefreshInFlight: Promise<boolean> | null = null;
  let liveAudioInputCapabilitiesEpoch = 0;
  let liveAudioInputDevicesRefreshEpoch = 0;
  let liveAudioInputDevicesRefreshInFlight: Promise<void> | null = null;
  let liveAudioInputLevelsEpoch = 0;
  let liveAudioInputLevelsPollInFlight = false;
  const invalidateLiveAudioInputLevels = () => {
    liveAudioInputLevelsEpoch += 1;
  };
  const refreshLiveAudioInputBackends = (announce = true): Promise<boolean> => {
    if (liveAudioInputBackendsRefreshInFlight) return liveAudioInputBackendsRefreshInFlight;
    const epoch = ++liveAudioInputBackendsEpoch;
    setLiveAudioInputBackendsBusy(true);

    let request!: Promise<boolean>;
    request = (async () => {
      try {
        const backends = await invoke<LiveAudioInputBackendSummary[]>("live_audio_input_backends");
        if (epoch !== liveAudioInputBackendsEpoch) return false;
        const currentBackend = selectedLiveAudioInputBackend();
        const nextBackend = backends.find((backend) => backend.id === currentBackend)
          ?? backends.find((backend) => backend.id === "wasapi_shared")
          ?? backends[0];
        setLiveAudioInputBackends(backends);
        setLiveAudioInputBackendsKnown(true);
        setLiveAudioInputBackendError(null);
        if (nextBackend && nextBackend.id !== currentBackend) {
          setSelectedLiveAudioInputBackend(nextBackend.id);
          setLiveAudioInputDevices([]);
          setSelectedLiveAudioInputDevice("");
          setLiveAudioInputCapabilities(null);
          setLiveAudioInputSampleRate(null);
          setLiveAudioInputBufferFrames(null);
          setLiveAudioInputChannelMix({ mode: "average_all" });
        }
        return Boolean(nextBackend?.built);
      } catch (error) {
        if (epoch !== liveAudioInputBackendsEpoch) return false;
        const detail = String(error);
        setLiveAudioInputBackends([]);
        setLiveAudioInputBackendsKnown(true);
        setLiveAudioInputBackendError(detail);
        setLiveAudioInputDevices([]);
        setLiveAudioInputCapabilities(null);
        if (announce) setMessage(detail);
        return false;
      }
    })().finally(() => {
      if (liveAudioInputBackendsRefreshInFlight === request) {
        liveAudioInputBackendsRefreshInFlight = null;
      }
      if (epoch === liveAudioInputBackendsEpoch) {
        setLiveAudioInputBackendsBusy(false);
      }
    });
    liveAudioInputBackendsRefreshInFlight = request;
    return request;
  };
  const refreshLiveAudioInputCapabilities = async (
    deviceId = selectedLiveAudioInputDevice(),
    announce = true,
    sampleRate = liveAudioInputSampleRate(),
    backendId = selectedLiveAudioInputBackend(),
  ): Promise<boolean> => {
    const epoch = ++liveAudioInputCapabilitiesEpoch;
    const backend = liveAudioInputBackends().find((candidate) => candidate.id === backendId);
    if (!backend?.built || (backend.requires_explicit_device && !deviceId.trim())) {
      setLiveAudioInputCapabilities(null);
      setLiveAudioInputCapabilitiesBusy(false);
      return false;
    }
    setLiveAudioInputCapabilitiesBusy(true);
    try {
      const capabilities = await invoke<LiveAudioInputCapabilities>(
        "get_live_audio_input_capabilities",
        { backend: backendId, deviceId: deviceId || null, sampleRate },
      );
      if (epoch === liveAudioInputCapabilitiesEpoch) {
        setLiveAudioInputCapabilities(capabilities);
        return true;
      }
    } catch (error) {
      if (epoch === liveAudioInputCapabilitiesEpoch) {
        setLiveAudioInputCapabilities(null);
        if (announce) setMessage(String(error));
      }
    } finally {
      if (epoch === liveAudioInputCapabilitiesEpoch) {
        setLiveAudioInputCapabilitiesBusy(false);
      }
    }
    return false;
  };
  const selectLiveAudioInputDevice = (deviceId: string) => {
    if (liveAudioInputBusy() || liveAudioInputBackendsBusy()) return;
    setSelectedLiveAudioInputDevice(deviceId);
    setLiveAudioInputSampleRate(null);
    setLiveAudioInputBufferFrames(null);
    setLiveAudioInputChannelMix({ mode: "average_all" });
    void refreshLiveAudioInputCapabilities(
      deviceId,
      true,
      null,
      selectedLiveAudioInputBackend(),
    );
  };
  const selectLiveAudioInputSampleRate = (sampleRate: number | null) => {
    setLiveAudioInputSampleRate(sampleRate);
    setLiveAudioInputBufferFrames(null);
    setLiveAudioInputChannelMix({ mode: "average_all" });
    void refreshLiveAudioInputCapabilities(
      selectedLiveAudioInputDevice(),
      true,
      sampleRate,
      selectedLiveAudioInputBackend(),
    );
  };
  const refreshLiveAudioInputDevices = (announce = true): Promise<void> => {
    if (liveAudioInputDevicesRefreshInFlight) return liveAudioInputDevicesRefreshInFlight;
    if (liveAudioInputBusy() || liveAudioInputBackendsBusy() || liveAudioInputStatus().running) {
      return Promise.resolve();
    }

    const previousDeviceId = selectedLiveAudioInputDevice();
    const previousDevices = liveAudioInputDevices();
    const previousDevice = previousDevices.find((device) => device.id === previousDeviceId);
    const epoch = ++liveAudioInputDevicesRefreshEpoch;
    liveAudioInputCapabilitiesEpoch += 1;
    setLiveAudioInputBusy(true);
    setLiveAudioInputCapabilitiesBusy(true);

    let request!: Promise<void>;
    request = (async () => {
      try {
        if (!liveAudioInputBackendsKnown() || liveAudioInputBackendError()) {
          const backendReady = await refreshLiveAudioInputBackends(announce);
          if (!backendReady) return;
        }
        const backendId = selectedLiveAudioInputBackend();
        const backend = liveAudioInputBackends().find((candidate) => candidate.id === backendId);
        if (!backend?.built) {
          setLiveAudioInputDevices([]);
          setLiveAudioInputCapabilities(null);
          if (announce && backend) {
            setMessage("Selected audio capture backend is not built into this application.");
          }
          return;
        }
        const devices = await invoke<LiveAudioInputDeviceSummary[]>("list_audio_input_devices", {
          backend: backendId,
        });
        if (epoch !== liveAudioInputDevicesRefreshEpoch) return;

        let selectedDeviceId = "";
        let selectionRequiresConfirmation = false;
        if (previousDeviceId) {
          if (devices.some((device) => device.id === previousDeviceId)) {
            selectedDeviceId = previousDeviceId;
          } else if (backendId === "wasapi_shared") {
            const previousIdentityMatches = previousDevice
              ? previousDevices.filter(
                  (device) =>
                    device.backend === previousDevice.backend && device.name === previousDevice.name,
                )
              : [];
            const refreshedIdentityMatches = previousDevice
              ? devices.filter(
                  (device) =>
                    device.backend === previousDevice.backend && device.name === previousDevice.name,
                )
              : [];
            if (previousIdentityMatches.length === 1 && refreshedIdentityMatches.length === 1) {
              selectedDeviceId = refreshedIdentityMatches[0].id;
            } else {
              selectionRequiresConfirmation = true;
              selectedDeviceId = previousDeviceId;
            }
          } else {
            selectionRequiresConfirmation = true;
            selectedDeviceId = previousDeviceId;
          }
        }

        setLiveAudioInputDevices(devices);
        if (selectionRequiresConfirmation) {
          setLiveAudioInputCapabilities(null);
          if (announce) {
            setMessage(
              "The previous audio input could not be identified safely after Refresh. Select an input again; Start is locked.",
            );
          }
          return;
        }
        if (selectedDeviceId !== previousDeviceId) {
          setSelectedLiveAudioInputDevice(selectedDeviceId);
        }
        const capabilitiesReady = await refreshLiveAudioInputCapabilities(
          selectedDeviceId,
          announce,
          liveAudioInputSampleRate(),
          backendId,
        );
        if (epoch === liveAudioInputDevicesRefreshEpoch && capabilitiesReady && announce) {
          setMessage(`Found ${devices.length} audio input device(s).`);
        }
      } catch (error) {
        if (epoch !== liveAudioInputDevicesRefreshEpoch) return;
        setLiveAudioInputDevices([]);
        setLiveAudioInputCapabilities(null);
        if (announce) setMessage(String(error));
      }
    })().finally(() => {
      if (liveAudioInputDevicesRefreshInFlight === request) {
        liveAudioInputDevicesRefreshInFlight = null;
      }
      if (epoch === liveAudioInputDevicesRefreshEpoch) {
        setLiveAudioInputCapabilitiesBusy(false);
        setLiveAudioInputBusy(false);
      }
    });
    liveAudioInputDevicesRefreshInFlight = request;
    return request;
  };
  const selectLiveAudioInputBackend = (backendId: LiveAudioInputBackendId) => {
    if (
      backendId === selectedLiveAudioInputBackend() ||
      liveAudioInputBusy() ||
      liveAudioInputBackendsBusy() ||
      liveAudioInputStatus().running
    ) {
      return;
    }
    liveAudioInputDevicesRefreshEpoch += 1;
    liveAudioInputCapabilitiesEpoch += 1;
    setSelectedLiveAudioInputBackend(backendId);
    setLiveAudioInputDevices([]);
    setSelectedLiveAudioInputDevice("");
    setLiveAudioInputCapabilities(null);
    setLiveAudioInputSampleRate(null);
    setLiveAudioInputBufferFrames(null);
    setLiveAudioInputChannelMix({ mode: "average_all" });
    setLiveAudioInputBackendError(null);
    const backend = liveAudioInputBackends().find((candidate) => candidate.id === backendId);
    if (!backend?.built) {
      if (backend) setMessage("Selected audio capture backend is not built into this application.");
      return;
    }
    void refreshLiveAudioInputDevices(false);
  };
  const startLiveAudioInput = async () => {
    if (liveAudioInputBusy() || liveAudioInputBackendsBusy()) return;
    const backend = liveAudioInputBackends().find(
      (candidate) => candidate.id === selectedLiveAudioInputBackend(),
    );
    if (!liveAudioInputBackendsKnown() || !backend?.built) {
      setMessage("Select a built audio input backend before Start.");
      return;
    }
    if (backend.requires_explicit_device && !selectedLiveAudioInputDevice().trim()) {
      setMessage("Select an ASIO driver before Start. Automatic driver selection is disabled.");
      return;
    }
    if (
      backend.requires_explicit_device &&
      (liveAudioInputSampleRate() === null || liveAudioInputBufferFrames() === null)
    ) {
      setMessage("Select an explicit ASIO sample rate and fixed buffer before Start.");
      return;
    }
    const resolvedConfig = liveAudioInputCapabilities()?.resolved_config;
    if (!resolvedConfig) {
      setMessage("Resolve a supported live audio input configuration before Start.");
      return;
    }
    invalidateLiveAudioInputLevels();
    clearLiveAudioInputTelemetryFreshness();
    const requestEpoch = liveAudioStatusRequests.beginCommand();
    setLiveAudioInputBusy(true);
    setLiveAudioInputStatusKnown(false);
    try {
      const request: LiveAudioInputStartRequest = {
        backend: selectedLiveAudioInputBackend(),
        device_id: selectedLiveAudioInputDevice().trim() || null,
        sample_rate: liveAudioInputSampleRate(),
        stream_channels: resolvedConfig.channels,
        sample_format: resolvedConfig.sample_format,
        buffer_frames: liveAudioInputBufferFrames(),
        channel_mix: liveAudioInputChannelMix(),
      };
      const nextStatus = await invoke<LiveAudioInputStatus>("start_live_audio_input", { request });
      if (liveAudioStatusRequests.accepts(requestEpoch)) {
        setLiveAudioInputStatus(nextStatus);
        setLiveAudioInputStatusKnown(true);
        if (nextStatus.running && !nextStatus.stale && !nextStatus.safety_clear_pending) {
          acceptLiveAudioInputTelemetry();
        } else {
          clearLiveAudioInputTelemetryFreshness();
        }
        setMessage("Live audio FFT input started.");
      }
    } catch (error) {
      if (liveAudioStatusRequests.accepts(requestEpoch)) {
        setLiveAudioInputStatusKnown(false);
        setMessage(String(error));
      }
    } finally {
      if (liveAudioStatusRequests.accepts(requestEpoch)) {
        liveAudioStatusRequests.endCommand(requestEpoch);
        setLiveAudioInputBusy(false);
      }
    }
  };
  const stopLiveAudioInput = async () => {
    if (liveAudioInputBusy()) return;
    invalidateLiveAudioInputLevels();
    clearLiveAudioInputTelemetryFreshness();
    const requestEpoch = liveAudioStatusRequests.beginCommand();
    setLiveAudioInputBusy(true);
    setLiveAudioInputStatusKnown(false);
    try {
      const nextStatus = await invoke<LiveAudioInputStatus>("stop_live_audio_input");
      if (liveAudioStatusRequests.accepts(requestEpoch)) {
        setLiveAudioInputStatus(nextStatus);
        setLiveAudioInputStatusKnown(true);
        setMessage(
          nextStatus.safety_clear_pending || nextStatus.running
            ? "Live audio Stop requested; safety clear is pending."
            : "Live audio FFT input stopped.",
        );
      }
    } catch (error) {
      if (liveAudioStatusRequests.accepts(requestEpoch)) {
        setLiveAudioInputStatusKnown(false);
        setMessage(String(error));
      }
    } finally {
      if (liveAudioStatusRequests.accepts(requestEpoch)) {
        liveAudioStatusRequests.endCommand(requestEpoch);
        setLiveAudioInputBusy(false);
      }
    }
  };
  const refreshLiveAudioInputStatus = async () => {
    const requestEpoch = liveAudioStatusRequests.beginPoll();
    if (requestEpoch === null) return;
    try {
      const nextStatus = await invoke<LiveAudioInputStatus>("live_audio_input_status");
      if (liveAudioStatusRequests.accepts(requestEpoch)) {
        invalidateLiveAudioInputLevels();
        setLiveAudioInputStatus(nextStatus);
        setLiveAudioInputStatusKnown(true);
        if (!nextStatus.running || nextStatus.stale || nextStatus.safety_clear_pending) {
          clearLiveAudioInputTelemetryFreshness();
        }
      }
    } catch (error) {
      if (liveAudioStatusRequests.accepts(requestEpoch)) {
        invalidateLiveAudioInputLevels();
        setLiveAudioInputStatus((current) => ({
          ...current,
          stale: current.running || current.stale,
          safety_clear_pending: current.running || current.safety_clear_pending,
          bass: 0,
          mid: 0,
          high: 0,
          bands: Array.from({ length: 16 }, () => 0),
          band_count: 0,
          rms: 0,
          peak: 0,
          spectral_flux: 0,
          spectral_centroid: 0,
          spectral_density_fast: 0,
          spectral_density_slow: 0,
          kick_strength: 0,
          snare_strength: 0,
          kick_event: false,
          snare_event: false,
          onset: false,
          onset_strength: 0,
          bpm: null,
          bpm_confidence: 0,
          beat_phase: 0,
          feature_sequence: 0,
          last_error: `Live audio status unavailable: ${String(error)}`,
        }));
        clearLiveAudioInputTelemetryFreshness();
        setLiveAudioInputStatusKnown(false);
      }
    } finally {
      liveAudioStatusRequests.endPoll();
    }
  };
  const refreshLiveAudioInputLevels = async () => {
    expireLiveAudioInputTelemetry();
    const current = liveAudioInputStatus();
    if (
      liveAudioInputLevelsPollInFlight ||
      liveAudioInputBusy() ||
      !liveAudioInputStatusKnown() ||
      !current.running
    ) {
      return;
    }

    const epoch = liveAudioInputLevelsEpoch;
    liveAudioInputLevelsPollInFlight = true;
    try {
      const levels = await invoke<LiveAudioInputLevels>("live_audio_input_levels");
      if (
        epoch !== liveAudioInputLevelsEpoch ||
        liveAudioInputBusy() ||
        !liveAudioInputStatusKnown()
      ) {
        return;
      }
      const safe = levels.running && !levels.stale && !levels.safety_clear_pending;
      if (safe) {
        acceptLiveAudioInputTelemetry();
      } else {
        clearLiveAudioInputTelemetryFreshness();
      }
      setLiveAudioInputStatus((status) => {
        if (!status.running) return status;
        const presentationSafe = safe && !status.stale && !status.safety_clear_pending;
        return {
          ...status,
          bass: presentationSafe ? levels.bass : 0,
          mid: presentationSafe ? levels.mid : 0,
          high: presentationSafe ? levels.high : 0,
          bands: presentationSafe ? levels.bands : Array.from({ length: 16 }, () => 0),
          band_count: presentationSafe ? levels.band_count : 0,
          rms: presentationSafe ? levels.rms : 0,
          peak: presentationSafe ? levels.peak : 0,
          spectral_flux: presentationSafe ? levels.spectral_flux : 0,
          spectral_centroid: presentationSafe ? levels.spectral_centroid : 0,
          spectral_density_fast: presentationSafe ? levels.spectral_density_fast : 0,
          spectral_density_slow: presentationSafe ? levels.spectral_density_slow : 0,
          kick_strength: presentationSafe ? levels.kick_strength : 0,
          snare_strength: presentationSafe ? levels.snare_strength : 0,
          kick_event: presentationSafe && levels.kick_event,
          snare_event: presentationSafe && levels.snare_event,
          onset: presentationSafe && levels.onset,
          onset_strength: presentationSafe ? levels.onset_strength : 0,
          bpm: presentationSafe ? levels.bpm : null,
          bpm_confidence: presentationSafe ? levels.bpm_confidence : 0,
          beat_phase: presentationSafe ? levels.beat_phase : 0,
          feature_sequence: presentationSafe ? levels.feature_sequence : 0,
        };
      });
    } catch {
      if (epoch === liveAudioInputLevelsEpoch) {
        clearLiveAudioInputTelemetryFreshness();
        setLiveAudioInputStatus((status) =>
          status.running
            ? {
                ...status,
                bass: 0,
                mid: 0,
                high: 0,
                bands: Array.from({ length: 16 }, () => 0),
                band_count: 0,
                rms: 0,
                peak: 0,
                spectral_flux: 0,
                spectral_centroid: 0,
                spectral_density_fast: 0,
                spectral_density_slow: 0,
                kick_strength: 0,
                snare_strength: 0,
                kick_event: false,
                snare_event: false,
                onset: false,
                onset_strength: 0,
                bpm: null,
                bpm_confidence: 0,
                beat_phase: 0,
                feature_sequence: 0,
              }
            : status,
        );
      }
    } finally {
      liveAudioInputLevelsPollInFlight = false;
    }
  };
  if (isTauriRuntime()) {
    void refreshLiveAudioInputStatus();
    void refreshLiveAudioInputBackends(false).then((backendReady) => {
      if (backendReady) void refreshLiveAudioInputDevices(false);
    });
  }
  let liveAudioInputLevelsTimer: number | null = null;
  const stopLiveAudioInputLevelsPolling = () => {
    if (liveAudioInputLevelsTimer === null) return;
    window.clearInterval(liveAudioInputLevelsTimer);
    liveAudioInputLevelsTimer = null;
  };
  const liveAudioInputLevelsPollingActive = createMemo(() => {
    const status = liveAudioInputStatus();
    return (
      liveAudioInputStatusKnown() &&
      status.running &&
      !liveAudioInputBusy() &&
      isTauriRuntime()
    );
  });
  createEffect(() => {
    if (!liveAudioInputLevelsPollingActive()) {
      stopLiveAudioInputLevelsPolling();
      clearLiveAudioInputTelemetryFreshness();
      return;
    }
    if (liveAudioInputLevelsTimer !== null) return;
    void refreshLiveAudioInputLevels();
    liveAudioInputLevelsTimer = window.setInterval(
      () => void refreshLiveAudioInputLevels(),
      33,
    );
  });
  const videoOutputMetricsTimer = isTauriRuntime()
    ? window.setInterval(() => {
        if (videoOutputWindowStatuses()?.some((status) => status.live_open)) {
          void refreshVideoOutputWindowStatuses(true);
        }
        if (
          videoProgramAudioEnabled() ||
          videoAudioMonitorStatus().active_layer_ids.length > 0 ||
          videoAudioMonitorStatus().last_sync_error
        ) {
          void refreshVideoAudioMonitorStatus(true);
        }
        if (videoRecordingStatus().active) {
          void refreshVideoRecordingStatus();
        }
        void refreshLiveAudioInputStatus();
      }, 1000)
    : null;
  onCleanup(() => {
    liveAudioStatusRequests.invalidate();
    liveAudioInputBackendsEpoch += 1;
    liveAudioInputCapabilitiesEpoch += 1;
    liveAudioInputDevicesRefreshEpoch += 1;
    invalidateLiveAudioInputLevels();
    stopLiveAudioInputLevelsPolling();
    clearLiveAudioInputTelemetryFreshness();
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
    handleMappingStageAuxClick,
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
  const colorEffectDraftValid = createMemo(() => {
    const stops = colorEffectStops();
    if (stops.length < 2 || stops.length > 16) return false;
    return stops.every((stop, index) =>
      Number.isFinite(stop.position)
      && stop.position >= 0
      && stop.position <= 1
      && (index === 0 || stop.position > stops[index - 1].position)
      && [stop.color.red, stop.color.green, stop.color.blue].every(
        (channel) => Number.isFinite(channel) && channel >= 0 && channel <= 65_535,
      ));
  });
  const currentChaserDraftError = createMemo(() => chaserDraftError({
    steps: chaserSteps(),
    features: chaserFeatures(),
    stepDurationMs: chaserStepDuration(),
    clockSyncBeats: effectClockSyncBeats(),
    wings: chaserWings(),
    activeStepCount: chaserActiveStepCount(),
    dutyCycle: chaserDutyCycle(),
    overlap: chaserOverlap(),
    phase: effectPhase(),
    fixtureSpread: chaserFixtureSpread(),
    randomSeed: chaserRandomSeed(),
  }));
  const moveDraftFixtureIds = () => {
    if (effectTargetMode() === "selection") {
      return selectedMappingFixtures().map((fixture) => fixture.id);
    }
    if (effectTargetMode() === "fixture") {
      const fixture = selectedFixture();
      return fixture ? [fixture.id] : [];
    }
    return [];
  };
  const moveDraftTargetGroupIds = () =>
    effectTargetMode() === "group" ? parseGroupIds(effectTargetGroups()) : [];
  const currentMoveDraftError = createMemo(() => {
    if (effectTargetMode() === "video" || effectVideoTargetLinked()) {
      return "Move effects target paired Pan/Tilt fixture controls and cannot link a video parameter.";
    }
    const error = moveEffectDraftError({
      fixtureIds: moveDraftFixtureIds(),
      targetGroupIds: moveDraftTargetGroupIds(),
      points: movePathPoints(),
      centerX: moveCenterX(),
      centerY: moveCenterY(),
      sizeX: moveSizeX(),
      sizeY: moveSizeY(),
      rotationDegrees: moveRotationDegrees(),
      periodMs: effectPeriod(),
      clockSyncBeats: effectClockSyncBeats(),
      phase: effectPhase(),
      fixtureSpread: moveFixtureSpread(),
      coordinateMode: moveCoordinateMode(),
      blendMode: effectBlendMode(),
    });
    if (error) return error;
    if (moveCompatibleTargetFixtures().length === 0) {
      return "The current target has no fixture with exactly one paired Pan and Tilt control.";
    }
    return "";
  });
  const effectSubmitDisabled = createMemo(() => {
    const linkedVideoMissing = effectVideoTargetLinked() && selectedEffectVideoLayerId() === null;
    if (effectType() === "Move") {
      return Boolean(currentMoveDraftError());
    }
    if (effectType() === "Chaser") {
      return Boolean(currentChaserDraftError()) || effectVideoTargetLinked() || effectTargetMode() === "video";
    }
    if (effectType() === "Color") {
      if (!colorEffectDraftValid()) return true;
      if (effectVideoTargetLinked() || effectTargetMode() === "video") return true;
      switch (effectTargetMode()) {
        case "selection":
          return selectedMappingFixtures().length === 0;
        case "group":
          return parseGroupIds(effectTargetGroups()).length === 0;
        case "fixture":
        default:
          return !selectedFixture();
      }
    }
    if (effectType() === "ColorMapping") {
      if (currentColorMappingDraftError()) return true;
      if (effectVideoTargetLinked() || effectTargetMode() === "video") return true;
      switch (effectTargetMode()) {
        case "selection":
          return selectedMappingFixtures().length === 0 && colorMappingCells().length === 0;
        case "group":
          return parseGroupIds(effectTargetGroups()).length === 0;
        case "fixture":
        default:
          return !selectedFixture() && colorMappingCells().length === 0;
      }
    }
    if (effectType() === "Value") {
      if (currentValueDraftError()) return true;
      switch (effectTargetMode()) {
        case "selection":
          return selectedMappingFixtures().length === 0 || !selectedEffectAttribute();
        case "group":
          return parseGroupIds(effectTargetGroups()).length === 0 || !selectedEffectAttribute();
        case "fixture":
        default:
          return !selectedFixture() || !selectedEffectAttribute();
      }
    }
    if (effectType() === "Curve") {
      if (currentCurveDraftError()) return true;
      if (effectVideoTargetLinked() || effectTargetMode() === "video") return true;
      switch (effectTargetMode()) {
        case "selection":
          return selectedMappingFixtures().length === 0 || !selectedEffectAttribute();
        case "group":
          return parseGroupIds(effectTargetGroups()).length === 0 || !selectedEffectAttribute();
        case "fixture":
        default:
          return !selectedFixture() || !selectedEffectAttribute();
      }
    }
    if (effectType() === "Mapping") {
      if (currentMappingDraftError()) return true;
      if (effectVideoTargetLinked() || effectTargetMode() === "video") return true;
      switch (effectTargetMode()) {
        case "selection":
          return orderedMappingSelectionFixtures().length === 0 || !selectedEffectAttribute();
        case "group":
          return parseGroupIds(effectTargetGroups()).length === 0 || !selectedEffectAttribute();
        case "fixture":
        default:
          return !selectedFixture() || !selectedEffectAttribute();
      }
    }
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
    | { effectType: "PositionWave"; request: PositionWaveEffectRequest }
    | { effectType: "Color"; request: ColorEffectRequest }
    | { effectType: "Chaser"; request: ChaserEffectRequest }
    | { effectType: "Move"; request: MoveEffectRequest }
    | { effectType: "Value"; request: ValueEffectRequest }
    | { effectType: "Curve"; request: CurveEffectRequest }
    | { effectType: "Mapping"; request: MappingEffectRequest }
    | { effectType: "ColorMapping"; request: ColorMappingEffectRequest };

  const effectParamsSnapshotFromDraft = (draft: EffectRequestDraft): EffectParamsSnapshot => {
    switch (draft.effectType) {
      case "Lfo": return { Lfo: draft.request };
      case "PositionWave": return { PositionWave: draft.request };
      case "Color": return { Color: draft.request };
      case "Chaser": return { Chaser: draft.request };
      case "Move": return { Move: draft.request };
      case "Value": return { Value: draft.request };
      case "Curve": return { Curve: draft.request };
      case "Mapping": return { Mapping: draft.request };
      case "ColorMapping": return { ColorMapping: draft.request };
    }
  };

  const buildEffectRequestFromForm = (): EffectRequestDraft | null => {
    const fixture = selectedFixture();
    const attribute = selectedEffectAttribute();
    const targetMode = effectTargetMode();
    const isVideoTarget = targetMode === "video";
    const colorEffect = effectType() === "Color";
    const colorMappingEffect = effectType() === "ColorMapping";
    const wholeFixtureColorEffect = colorEffect || colorMappingEffect;
    const chaserEffect = effectType() === "Chaser";
    const moveEffect = effectType() === "Move";
    if (moveEffect) {
      const error = currentMoveDraftError();
      if (error) {
        setMessage(error);
        return null;
      }
      return {
        effectType: "Move",
        request: {
          label: editingEffectSummary()?.label ?? "Pan/Tilt Move",
          fixture_ids: moveDraftFixtureIds(),
          target_group_ids: moveDraftTargetGroupIds(),
          points: movePathPoints().map((point) => ({ ...point })),
          closed: movePathClosed(),
          interpolation: moveInterpolation(),
          coordinate_mode: moveCoordinateMode(),
          center_x: moveCenterX(),
          center_y: moveCenterY(),
          size_x: moveSizeX(),
          size_y: moveSizeY(),
          rotation_degrees: moveRotationDegrees(),
          period_ms: Math.round(effectPeriod()),
          clock_sync: effectClockSyncBeats() === null ? null : { beats: effectClockSyncBeats()! },
          direction: moveDirection(),
          phase: effectPhase(),
          fixture_spread: moveFixtureSpread(),
          blend_mode: effectBlendMode(),
        },
      };
    }
    if (chaserEffect) {
      const error = currentChaserDraftError();
      if (error) {
        setMessage(error);
        return null;
      }
      const features = chaserFeatures().map((feature) => ({
        attribute: feature.attribute.trim(),
        low: Math.round(feature.low),
        high: Math.round(feature.high),
      }));
      return {
        effectType: "Chaser",
        request: {
          label: `${features[0]?.attribute ?? "Fixture"} Chaser`,
          steps: chaserSteps().map((step) => ({
            fixture_ids: [...step.fixture_ids],
            target_group_ids: [...step.target_group_ids],
            level: Math.round(step.level),
          })),
          features,
          step_duration_ms: Math.max(10, Math.round(chaserStepDuration())),
          clock_sync: effectClockSyncBeats() === null ? null : { beats: effectClockSyncBeats()! },
          direction: chaserDirection(),
          wings: clampChaserWings(chaserWings()),
          active_step_count: Math.round(chaserActiveStepCount()),
          duty_cycle: Math.max(0.01, clampChaserUnit(chaserDutyCycle())),
          overlap: clampChaserUnit(chaserOverlap()),
          phase: clampChaserUnit(effectPhase()),
          fixture_spread: clampChaserUnit(chaserFixtureSpread()),
          random_seed: Math.round(chaserRandomSeed()),
          blend_mode: effectBlendMode(),
        },
      };
    }
    const includesVideoTarget = !wholeFixtureColorEffect && (isVideoTarget || effectVideoTargetLinked());
    if (wholeFixtureColorEffect && (isVideoTarget || effectVideoTargetLinked())) {
      setMessage("Color and Colour Mapping effects target complete lighting fixtures and cannot link a video parameter.");
      return null;
    }
    if (targetMode === "fixture" && (!fixture || (!wholeFixtureColorEffect && !attribute))) {
      setMessage("Select a fixture and attribute first.");
      return null;
    }
    const selectedMapFixtureIds = selectedMappingFixtures().map((candidate) => candidate.id);
    if (targetMode === "selection" && selectedMapFixtureIds.length === 0) {
      setMessage("Select one or more fixtures on the 2D mapping stage first.");
      return null;
    }
    if (!wholeFixtureColorEffect && targetMode === "selection" && !attribute) {
      setMessage("Select a fixture profile attribute before targeting a map selection.");
      return null;
    }
    const targetGroupIds = parseGroupIds(effectTargetGroups());
    if (targetMode === "group" && targetGroupIds.length === 0) {
      setMessage("Enter at least one target group.");
      return null;
    }
    if (!wholeFixtureColorEffect && targetMode === "group" && !attribute) {
      setMessage("Select a fixture profile attribute before targeting a group.");
      return null;
    }
    const videoLayerId = selectedEffectVideoLayerId();
    if (includesVideoTarget && videoLayerId === null) {
      setMessage(isVideoTarget ? "Add a video layer before adding a video effect." : "Add a video layer before linking video to this effect.");
      return null;
    }
    const lightAttribute = attribute ?? "";
    const videoTargets = wholeFixtureColorEffect ? [] : buildEffectVideoTargets(effectType() === "PositionWave");
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
    if (effectType() === "Value") {
      const error = currentValueDraftError();
      if (error) {
        setMessage(error);
        return null;
      }
      return {
        effectType: "Value",
        request: {
          label: `${lightAttribute} Value`,
          fixture_ids: requestBase.fixture_ids,
          target_group_ids: requestBase.target_group_ids,
          attribute: lightAttribute,
          points: valuePoints().map((point) => ({ ...point })),
          interpolation: valueInterpolation(),
          mode: valueMode(),
          direction: valueDirection(),
          period_ms: Math.round(effectPeriod()),
          clock_sync: requestBase.clock_sync,
          low: effectLow(),
          high: effectHigh(),
          phase: effectPhase(),
          fixture_spread: valueFixtureSpread(),
          blend_mode: effectBlendMode(),
        },
      };
    }
    if (effectType() === "Curve") {
      const error = currentCurveDraftError();
      if (error) {
        setMessage(error);
        return null;
      }
      return {
        effectType: "Curve",
        request: {
          label: `${lightAttribute} Curve`,
          fixture_ids: requestBase.fixture_ids,
          target_group_ids: requestBase.target_group_ids,
          attribute: lightAttribute,
          points: curvePoints().map((point) => ({ ...point })),
          mode: curveMode(),
          direction: curveDirection(),
          period_ms: Math.round(effectPeriod()),
          clock_sync: requestBase.clock_sync,
          low: effectLow(),
          high: effectHigh(),
          phase: effectPhase(),
          fixture_spread: curveFixtureSpread(),
          blend_mode: effectBlendMode(),
        },
      };
    }
    if (effectType() === "Mapping") {
      const error = currentMappingDraftError();
      if (error) {
        setMessage(error);
        return null;
      }
      return {
        effectType: "Mapping",
        request: {
          label: `${lightAttribute} Mapping`,
          fixture_ids: targetMode === "selection"
            ? orderedMappingSelectionFixtures().map((candidate) => candidate.id)
            : requestBase.fixture_ids,
          target_group_ids: requestBase.target_group_ids,
          attribute: lightAttribute,
          shape: effectShape(),
          mode: mappingMode(),
          direction: mappingDirection(),
          period_ms: Math.round(effectPeriod()),
          clock_sync: requestBase.clock_sync,
          low: effectLow(),
          high: effectHigh(),
          phase: effectPhase(),
          fixture_spread: mappingFixtureSpread(),
          repetitions: mappingRepetitions(),
          blend_mode: effectBlendMode(),
        },
      };
    }
    if (colorMappingEffect) {
      const error = currentColorMappingDraftError();
      if (error) {
        setMessage(error);
        return null;
      }
      const targetFixtureIds = new Set(requestBase.fixture_ids);
      const cells = targetMode === "group"
        ? []
        : colorMappingCells()
            .filter((cell) => targetFixtureIds.has(cell.fixture_id))
            .map((cell) => ({ ...cell }));
      return {
        effectType: "ColorMapping",
        request: {
          label: editingEffectSummary()?.label ?? `Colour Mapping ${targetMode === "group" ? "Group" : targetMode === "selection" ? "Selection" : "Fixture"}`,
          fixture_ids: requestBase.fixture_ids,
          target_group_ids: requestBase.target_group_ids,
          source_kind: colorMappingSourceKind(),
          width: colorMappingWidth(),
          height: colorMappingHeight(),
          frames: colorMappingFrames().map((frame) => ({ pixels: [...frame.pixels] })),
          cells,
          playback_direction: colorMappingPlaybackDirection(),
          period_ms: Math.round(effectPeriod()),
          clock_sync: requestBase.clock_sync,
          phase: effectPhase(),
          offset_u: colorMappingOffsetU(),
          offset_v: colorMappingOffsetV(),
          scale_u: colorMappingScaleU(),
          scale_v: colorMappingScaleV(),
          rotation_degrees: colorMappingRotationDegrees(),
          wrap_mode: colorMappingWrapMode(),
          sampling: colorMappingSampling(),
          blend_mode: effectBlendMode(),
        },
      };
    }
    if (colorEffect) {
      if (!colorEffectDraftValid()) {
        setMessage("Color effects require 2 to 16 ordered palette stops with valid colors.");
        return null;
      }
      return {
        effectType: "Color",
        request: {
          label: `Color ${targetMode === "group" ? "Group" : targetMode === "selection" ? "Selection" : "Fixture"}`,
          fixture_ids: requestBase.fixture_ids,
          target_group_ids: requestBase.target_group_ids,
          stops: colorEffectStops().map((stop) => ({ ...stop, color: { ...stop.color } })),
          algorithm: colorEffectAlgorithm(),
          interpolation: colorEffectInterpolation(),
          period_ms: effectPeriod(),
          clock_sync: requestBase.clock_sync,
          phase: effectPhase(),
          fixture_spread: colorEffectFixtureSpread(),
          blend_mode: effectBlendMode(),
          spatial_pattern: colorEffectSpatialPattern(),
        },
      };
    }
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
      return null;
    }
    try {
      const effectId = draft.effectType === "Lfo"
        ? await invoke<number>("add_lfo_effect", { request: draft.request })
        : draft.effectType === "PositionWave"
          ? await invoke<number>("add_position_wave_effect", { request: draft.request })
          : draft.effectType === "Color"
            ? await invoke<number>("add_color_effect", { request: draft.request })
            : draft.effectType === "Chaser"
              ? await invoke<number>("add_chaser_effect", { request: draft.request })
              : draft.effectType === "Value"
                ? await invoke<number>("add_value_effect", { request: draft.request })
                : draft.effectType === "Curve"
                  ? await invoke<number>("add_curve_effect", { request: draft.request })
                : draft.effectType === "Mapping"
                  ? await invoke<number>("add_mapping_effect", { request: draft.request })
                : draft.effectType === "ColorMapping"
                  ? await invoke<number>("add_color_mapping_effect", { request: draft.request })
                : await invoke<number>("add_move_effect", { request: draft.request });
      setEditingEffectId(null);
      setMessage(`Added ${draft.effectType === "PositionWave" ? "position wave" : draft.effectType === "Color" ? "color" : draft.effectType === "Chaser" ? "Chaser" : draft.effectType === "Move" ? "Move" : draft.effectType === "Value" ? "Value" : draft.effectType === "Curve" ? "Curve" : draft.effectType === "Mapping" ? "Mapping" : draft.effectType === "ColorMapping" ? "Colour Mapping" : "LFO"} effect ${effectId}`);
      await refreshSnapshot();
      return effectId;
    } catch (error) {
      setMessage(String(error));
      return null;
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
      } else if (draft.effectType === "PositionWave") {
        await invoke("update_position_wave_effect", { effectId, request: draft.request });
      } else if (draft.effectType === "Color") {
        await invoke("update_color_effect", { effectId, request: draft.request });
      } else if (draft.effectType === "Chaser") {
        await invoke("update_chaser_effect", { effectId, request: draft.request });
      } else if (draft.effectType === "Value") {
        await invoke("update_value_effect", { effectId, request: draft.request });
      } else if (draft.effectType === "Curve") {
        await invoke("update_curve_effect", { effectId, request: draft.request });
      } else if (draft.effectType === "Mapping") {
        await invoke("update_mapping_effect", { effectId, request: draft.request });
      } else if (draft.effectType === "ColorMapping") {
        await invoke("update_color_mapping_effect", { effectId, request: draft.request });
      } else {
        await invoke("update_move_effect", { effectId, request: draft.request });
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
    const effectFamily = chooserFamilyForEffectType(effect.effect_type);
    setEffectChooserFamily(effectFamily);
    if (effect.effect_type === "Chaser") {
      const chaser = effect.chaser;
      if (!chaser) {
        setEditingEffectId(null);
        setMessage(`Chaser effect ${effect.id} is missing its editor body.`);
        return;
      }
      setChaserSteps(chaser.steps.map((step) => ({
        fixture_ids: [...step.fixture_ids],
        target_group_ids: [...step.target_group_ids],
        level: step.level,
      })));
      setChaserFeatures(chaser.features.map((feature) => ({ ...feature })));
      setChaserStepDuration(chaser.step_duration_ms);
      setEffectClockSyncBeats(chaser.clock_sync?.beats ?? null);
      setChaserDirection(chaser.direction);
      setChaserWings(chaser.wings);
      setChaserActiveStepCount(chaser.active_step_count);
      setChaserDutyCycle(chaser.duty_cycle);
      setChaserOverlap(chaser.overlap);
      setEffectPhase(chaser.phase);
      setChaserFixtureSpread(chaser.fixture_spread);
      setChaserRandomSeed(chaser.random_seed);
      setEffectBlendMode(chaser.blend_mode);
      setEffectVideoTargetLinked(false);
    }
    if (effect.effect_type === "Color") {
      const color = effect.color;
      if (!color) {
        setEditingEffectId(null);
        setMessage(`Color effect ${effect.id} is missing its editor body.`);
        return;
      }
      setColorEffectStops(color.stops.map((stop) => ({ ...stop, color: { ...stop.color } })));
      setColorEffectAlgorithm(color.algorithm);
      setColorEffectInterpolation(color.interpolation);
      setColorEffectFixtureSpread(color.fixture_spread);
      setColorEffectSpatialPattern(color.spatial_pattern ?? null);
      setEffectPeriod(color.period_ms);
      setEffectClockSyncBeats(color.clock_sync?.beats ?? null);
      setEffectPhase(color.phase);
      setEffectBlendMode(color.blend_mode);
      setEffectVideoTargetLinked(false);
    }
    if (effect.effect_type === "Move") {
      const move = effect.move_effect;
      if (!move) {
        setEditingEffectId(null);
        setMessage(`Move effect ${effect.id} is missing its editor body.`);
        return;
      }
      setMovePathPoints(move.points.map((point) => ({ ...point })));
      setMovePathClosed(move.closed);
      setMoveInterpolation(move.interpolation);
      setMoveCoordinateMode(move.coordinate_mode);
      setMoveCenterX(move.center_x);
      setMoveCenterY(move.center_y);
      setMoveSizeX(move.size_x);
      setMoveSizeY(move.size_y);
      setMoveRotationDegrees(move.rotation_degrees);
      setEffectPeriod(move.period_ms);
      setEffectClockSyncBeats(move.clock_sync?.beats ?? null);
      setMoveDirection(move.direction);
      setEffectPhase(move.phase);
      setMoveFixtureSpread(move.fixture_spread);
      setEffectBlendMode(move.blend_mode);
      setMovePathRecipe("Custom");
      setEffectVideoTargetLinked(false);
    }
    if (effect.effect_type === "Value") {
      const value = effect.value;
      if (!value) {
        setEditingEffectId(null);
        setMessage(`Value effect ${effect.id} is missing its editor body.`);
        return;
      }
      setValuePoints(value.points.map((point) => ({ ...point })));
      setValueInterpolation(value.interpolation);
      setValueMode(value.mode);
      setValueDirection(value.direction);
      setValueFixtureSpread(value.fixture_spread);
      setEffectPeriod(value.period_ms);
      setEffectClockSyncBeats(value.clock_sync?.beats ?? null);
      setEffectPhase(value.phase);
      setEffectBlendMode(value.blend_mode);
      setEffectVideoTargetLinked(false);
    }
    if (effect.effect_type === "Curve") {
      const curve = effect.curve;
      if (!curve) {
        setEditingEffectId(null);
        setMessage(`Curve effect ${effect.id} is missing its editor body.`);
        return;
      }
      setCurvePoints(curve.points.map((point) => ({ ...point })));
      setCurveMode(curve.mode);
      setCurveDirection(curve.direction);
      setCurveFixtureSpread(curve.fixture_spread);
      setEffectPeriod(curve.period_ms);
      setEffectClockSyncBeats(curve.clock_sync?.beats ?? null);
      setEffectPhase(curve.phase);
      setEffectBlendMode(curve.blend_mode);
      setEffectVideoTargetLinked(false);
    }
    if (effect.effect_type === "Mapping") {
      const mapping = effect.mapping;
      if (!mapping) {
        setEditingEffectId(null);
        setMessage(`Mapping effect ${effect.id} is missing its editor body.`);
        return;
      }
      setEffectShape(mapping.shape);
      setMappingMode(mapping.mode);
      setMappingDirection(mapping.direction);
      setEffectPeriod(mapping.period_ms);
      setEffectClockSyncBeats(mapping.clock_sync?.beats ?? null);
      setEffectLow(mapping.low);
      setEffectHigh(mapping.high);
      setEffectPhase(mapping.phase);
      setMappingFixtureSpread(mapping.fixture_spread);
      setMappingRepetitions(mapping.repetitions);
      setEffectBlendMode(mapping.blend_mode);
      setEffectVideoTargetLinked(false);
    }
    if (effect.effect_type === "ColorMapping") {
      const mapping = effect.color_mapping;
      if (!mapping) {
        setEditingEffectId(null);
        setMessage(`Colour Mapping effect ${effect.id} is missing its editor body.`);
        return;
      }
      setColorMappingSourceKind(mapping.source_kind);
      setColorMappingWidth(mapping.width);
      setColorMappingHeight(mapping.height);
      setColorMappingFrames(mapping.frames.map((frame) => ({ pixels: [...frame.pixels] })));
      setColorMappingCells((mapping.cells ?? []).map((cell) => ({ ...cell })));
      setColorMappingPlaybackDirection(mapping.playback_direction);
      setEffectPeriod(mapping.period_ms);
      setEffectClockSyncBeats(mapping.clock_sync?.beats ?? null);
      setEffectPhase(mapping.phase);
      setColorMappingOffsetU(mapping.offset_u);
      setColorMappingOffsetV(mapping.offset_v);
      setColorMappingScaleU(mapping.scale_u);
      setColorMappingScaleV(mapping.scale_v);
      setColorMappingRotationDegrees(mapping.rotation_degrees);
      setColorMappingWrapMode(mapping.wrap_mode);
      setColorMappingSampling(mapping.sampling);
      setEffectBlendMode(mapping.blend_mode);
      setEffectVideoTargetLinked(false);
    }
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
    setMessage(
      effect.effect_type === "Chaser"
        ? `Loaded Chaser effect ${effect.id} with ${effect.chaser?.steps.length ?? 0} ordered steps and ${effect.chaser?.features.length ?? 0} features.`
        : effect.effect_type === "Move"
          ? `Loaded Move effect ${effect.id} with ${effect.move_effect?.points.length ?? 0} path points into the paired Pan/Tilt editor.`
          : effect.effect_type === "Mapping"
            ? `Loaded Mapping effect ${effect.id} with ${effect.fixture_ids.length} authored fixtures into the fixture-order editor.`
          : effect.effect_type === "ColorMapping"
            ? `Loaded Colour Mapping effect ${effect.id} with ${effect.color_mapping?.frames.length ?? 0} embedded frames into the 2D editor.`
          : targetPlan.message,
    );
  };

  const sceneEffectForCue = (cueId: number, effectId: number) => {
    const cue = snapshot().cues.find((candidate) => candidate.id === cueId);
    const target = cue?.effect_targets.find((candidate) => candidate.effect_id === effectId);
    if (!cue || !target) return null;
    const source = snapshot().effects.find((candidate) => candidate.id === effectId) ?? null;
    return cueOwnedEffectSummary(target, source);
  };

  loadSceneEffectDraft = (cueId: number, effectId: number) => {
    const effect = sceneEffectForCue(cueId, effectId);
    if (effect) useEffectAsDraft(effect);
  };

  const selectSceneEffect = (effectId: number) => {
    const cue = selectedSceneCue();
    if (!cue || !cue.effect_targets.some((target) => target.effect_id === effectId)) return;
    setSelectedSceneEffectId(effectId);
    loadSceneEffectDraft(cue.id, effectId);
  };

  const persistSceneEffectTargets = async (
    cueId: number,
    effectTargets: CueEffectTarget[],
  ) => {
    if (viewportFixture) {
      setSnapshot((current) => ({
        ...current,
        cues: current.cues.map((cue) =>
          cue.id === cueId
            ? { ...cue, effect_targets: effectTargets.map((target) => ({ ...target })) }
            : cue
        ),
      }));
      return true;
    }
    try {
      await invoke("set_cue_effect_targets", { cueId, effectTargets });
      await refreshSnapshot();
      return true;
    } catch (error) {
      setMessage(String(error));
      return false;
    }
  };

  const prepareSceneEffectTarget = (cue: CueSummary) => {
    const fixtureIds = [...new Set(cue.targets.map((target) => target.fixture_id))]
      .filter((fixtureId) => snapshot().fixtures.some((fixture) => fixture.id === fixtureId));
    const attribute = cue.targets
      .flatMap((target) => target.values)
      .find((value) => value.attribute.trim())?.attribute
      ?? snapshot().fixtures
        .find((fixture) => fixture.id === fixtureIds[0])
        ?.controls[0]?.attribute
      ?? "";
    setEditingEffectId(null);
    setEffectVideoTargetLinked(false);
    setSelectedMappingFixtureIds(fixtureIds);
    if (fixtureIds.length > 0) {
      setEffectTargetMode("selection");
      const firstFixture = snapshot().fixtures.find((fixture) => fixture.id === fixtureIds[0]);
      if (firstFixture) activateFixture(firstFixture);
    } else if (cue.group_id?.trim()) {
      setEffectTargetMode("group");
      setEffectTargetGroups(cue.group_id);
    } else {
      setEffectTargetMode("fixture");
    }
    setEffectAttribute(attribute);
  };

  const createSceneEffect = async (family: EffectChooserFamily) => {
    const cue = selectedSceneCue();
    if (!cue) return;
    if (family === "STEPS") {
      openTimelineSourceCue(cue.id);
      return;
    }
    if (family === "SUPER SCENE") {
      const opened = await openOrCreateSuperScene(cue.id);
      if (!opened) return;
      setWorkspaceTab("control");
      setControlMode("live");
      setTimelineDeskSurface("show");
      setTimelineContextDrawer("none");
      return;
    }
    prepareSceneEffectTarget(cue);
    await selectEffectFamily(family);
    const draft = buildEffectRequestFromForm();
    if (!draft) return;
    const params = effectParamsSnapshotFromDraft(draft);
    if (viewportFixture) {
      const effectId = Math.max(900, ...snapshot().effects.map((effect) => effect.id)) + 1;
      const effectTarget: CueEffectTarget = { effect_id: effectId, enabled: true, params };
      const effect = cueOwnedEffectSummary(effectTarget, null);
      if (!effect) return;
      setSnapshot((current) => ({
        ...current,
        effects: [...current.effects, effect],
        cues: current.cues.map((candidate) =>
          candidate.id === cue.id
            ? { ...candidate, effect_targets: [...candidate.effect_targets, effectTarget] }
            : candidate
        ),
      }));
      setSelectedSceneEffectId(effectId);
      useEffectAsDraft(effect);
      setMessage(`Created cue-owned ${draft.effectType} FX for scene ${cue.id}.`);
      return;
    }
    const effectId = await addEffect();
    if (effectId === null) return;
    const currentCue = snapshot().cues.find((candidate) => candidate.id === cue.id) ?? cue;
    const saved = await persistSceneEffectTargets(cue.id, [
      ...currentCue.effect_targets,
      { effect_id: effectId, enabled: true, params },
    ]);
    if (!saved) return;
    setSelectedSceneEffectId(effectId);
    loadSceneEffectDraft(cue.id, effectId);
    setMessage(`Created cue-owned ${draft.effectType} FX for scene ${cue.id}.`);
  };

  const saveSceneEffectDraft = async () => {
    const cue = selectedSceneCue();
    const effectId = selectedSceneEffectId();
    if (!cue || effectId === null) return;
    const draft = buildEffectRequestFromForm();
    if (!draft) return;
    const params = effectParamsSnapshotFromDraft(draft);
    const effectTargets = cue.effect_targets.map((target) =>
      target.effect_id === effectId ? { ...target, enabled: true, params } : target
    );
    const saved = await persistSceneEffectTargets(cue.id, effectTargets);
    if (!saved) return;
    const effect = cueOwnedEffectSummary(
      { ...effectTargets.find((target) => target.effect_id === effectId)!, params },
      snapshot().effects.find((candidate) => candidate.id === effectId) ?? null,
    );
    if (effect) useEffectAsDraft(effect);
    setMessage(`Saved cue-owned ${draft.effectType} FX for scene ${cue.id}.`);
  };

  const sceneEffectEditor = createMemo<SceneEffectEditorModel>(() => ({
    effectType: effectType(),
    attribute: selectedEffectAttribute(),
    attributeOptions: effectTargetControls().map((control) => control.attribute),
    onAttribute: setEffectAttribute,
    color: {
      stops: colorEffectStops(),
      algorithm: colorEffectAlgorithm(),
      interpolation: colorEffectInterpolation(),
      periodMs: effectPeriod(),
      bpm: snapshot().clock.bpm,
      clockSyncBeats: effectClockSyncBeats(),
      fixtureSpread: colorEffectFixtureSpread(),
      spatialPattern: colorEffectSpatialPattern(),
      onStops: setColorEffectStops,
      onAlgorithm: setColorEffectAlgorithm,
      onInterpolation: setColorEffectInterpolation,
      onPeriodMs: setEffectPeriod,
      onClockSyncBeats: setEffectClockSyncPreset,
      onFixtureSpread: setColorEffectFixtureSpread,
      onSpatialPattern: setColorEffectSpatialPattern,
    },
    chaser: {
      steps: chaserSteps(),
      features: chaserFeatures(),
      fixtureOptions: snapshot().fixtures.map((fixture) => ({ id: fixture.id, label: fixture.label })),
      attributeOptions: chaserAttributeOptions(),
      attributeCoverage: chaserAttributeCoverage(),
      currentTargetLabel: effectTargetSummary(),
      currentTargetSteps: chaserCurrentTargetSteps(),
      stepDurationMs: chaserStepDuration(),
      bpm: snapshot().clock.bpm,
      clockSyncBeats: effectClockSyncBeats(),
      direction: chaserDirection(),
      wings: chaserWings(),
      activeStepCount: chaserActiveStepCount(),
      dutyCycle: chaserDutyCycle(),
      overlap: chaserOverlap(),
      phase: effectPhase(),
      fixtureSpread: chaserFixtureSpread(),
      randomSeed: chaserRandomSeed(),
      error: currentChaserDraftError(),
      onSteps: setChaserSteps,
      onFeatures: setChaserFeatures,
      onStepDurationMs: setChaserStepDuration,
      onClockSyncBeats: setEffectClockSyncPreset,
      onDirection: setChaserDirection,
      onWings: setChaserWings,
      onActiveStepCount: setChaserActiveStepCount,
      onDutyCycle: setChaserDutyCycle,
      onOverlap: setChaserOverlap,
      onFixtureSpread: setChaserFixtureSpread,
      onRandomSeed: setChaserRandomSeed,
    },
    move: {
      points: movePathPoints(),
      closed: movePathClosed(),
      interpolation: moveInterpolation(),
      coordinateMode: moveCoordinateMode(),
      center: { x: moveCenterX(), y: moveCenterY() },
      size: { x: moveSizeX(), y: moveSizeY() },
      rotation: moveRotationDegrees(),
      periodMs: effectPeriod(),
      bpm: snapshot().clock.bpm,
      clockSyncBeats: effectClockSyncBeats(),
      direction: moveDirection(),
      phase: effectPhase(),
      spread: moveFixtureSpread(),
      onPoints: (points) => {
        setMovePathRecipe("Custom");
        setMovePathPoints(points);
      },
      onClosed: (closed) => {
        setMovePathRecipe("Custom");
        setMovePathClosed(closed);
      },
      onInterpolation: setMoveInterpolation,
      onCoordinateMode: setMoveCoordinateMode,
      onCenter: (center) => {
        setMoveCenterX(center.x);
        setMoveCenterY(center.y);
      },
      onSize: (size) => {
        setMoveSizeX(size.x);
        setMoveSizeY(size.y);
      },
      onRotation: setMoveRotationDegrees,
      onPeriodMs: setEffectPeriod,
      onClockSyncBeats: setEffectClockSyncPreset,
      onDirection: setMoveDirection,
      onPhase: setEffectPhase,
      onSpread: setMoveFixtureSpread,
    },
    value: {
      points: valuePoints(),
      interpolation: valueInterpolation(),
      mode: valueMode(),
      direction: valueDirection(),
      periodMs: effectPeriod(),
      bpm: snapshot().clock.bpm,
      clockSyncBeats: effectClockSyncBeats(),
      phase: effectPhase(),
      spread: valueFixtureSpread(),
      onPoints: setValuePoints,
      onInterpolation: setValueInterpolation,
      onMode: setValueMode,
      onDirection: setValueDirection,
      onPeriodMs: setEffectPeriod,
      onClockSyncBeats: setEffectClockSyncPreset,
      onPhase: setEffectPhase,
      onSpread: setValueFixtureSpread,
    },
    curve: {
      points: curvePoints(),
      mode: curveMode(),
      direction: curveDirection(),
      periodMs: effectPeriod(),
      bpm: snapshot().clock.bpm,
      clockSyncBeats: effectClockSyncBeats(),
      phase: effectPhase(),
      spread: curveFixtureSpread(),
      onPoints: setCurvePoints,
      onMode: setCurveMode,
      onDirection: setCurveDirection,
      onPeriodMs: setEffectPeriod,
      onClockSyncBeats: setEffectClockSyncPreset,
      onPhase: setEffectPhase,
      onSpread: setCurveFixtureSpread,
    },
    mapping: {
      shape: effectShape(),
      mode: mappingMode(),
      direction: mappingDirection(),
      periodMs: effectPeriod(),
      bpm: snapshot().clock.bpm,
      clockSyncBeats: effectClockSyncBeats(),
      phase: effectPhase(),
      spread: mappingFixtureSpread(),
      repetitions: mappingRepetitions(),
      fixtures: mappingEffectOrderFixtures().map((fixture) => ({ id: fixture.id, label: fixture.label })),
      orderEditable: effectTargetMode() === "selection",
      onShape: setEffectShape,
      onMode: setMappingMode,
      onDirection: setMappingDirection,
      onPeriodMs: setEffectPeriod,
      onClockSyncBeats: setEffectClockSyncPreset,
      onPhase: setEffectPhase,
      onSpread: setMappingFixtureSpread,
      onRepetitions: setMappingRepetitions,
      onFixtureOrder: setSelectedMappingFixtureIds,
    },
    colorMapping: {
      sourceKind: colorMappingSourceKind(),
      width: colorMappingWidth(),
      height: colorMappingHeight(),
      frames: colorMappingFrames(),
      cells: colorMappingCells(),
      fixtures: effectTargetFixtures().map((fixture) => ({
        id: fixture.id,
        label: fixture.label,
        x: fixture.position.x,
        z: fixture.position.z,
      })),
      playbackDirection: colorMappingPlaybackDirection(),
      periodMs: effectPeriod(),
      bpm: snapshot().clock.bpm,
      clockSyncBeats: effectClockSyncBeats(),
      phase: effectPhase(),
      offsetU: colorMappingOffsetU(),
      offsetV: colorMappingOffsetV(),
      scaleU: colorMappingScaleU(),
      scaleV: colorMappingScaleV(),
      rotationDegrees: colorMappingRotationDegrees(),
      wrapMode: colorMappingWrapMode(),
      sampling: colorMappingSampling(),
      onRaster: (kind, width, height, frames) => {
        setColorMappingSourceKind(kind);
        setColorMappingWidth(width);
        setColorMappingHeight(height);
        setColorMappingFrames(frames);
      },
      onCells: setColorMappingCells,
      onPlaybackDirection: setColorMappingPlaybackDirection,
      onPeriodMs: setEffectPeriod,
      onClockSyncBeats: setEffectClockSyncPreset,
      onPhase: setEffectPhase,
      onOffsetU: setColorMappingOffsetU,
      onOffsetV: setColorMappingOffsetV,
      onScaleU: setColorMappingScaleU,
      onScaleV: setColorMappingScaleV,
      onRotationDegrees: setColorMappingRotationDegrees,
      onWrapMode: setColorMappingWrapMode,
      onSampling: setColorMappingSampling,
    },
    action: {
      showLightRange: effectTargetMode() !== "video"
        && !["Color", "ColorMapping", "Chaser", "Move"].includes(effectType()),
      showPhase: !["Move", "Value", "Curve", "Mapping", "ColorMapping"].includes(effectType()),
      lockBlendMode: effectType() === "Move",
      low: effectLow(),
      high: effectHigh(),
      phase: effectPhase(),
      blendMode: effectBlendMode(),
      addDisabled: effectSubmitDisabled(),
      submitLabel: "Save cue-owned FX",
      editing: true,
      editingLabel: selectedSceneEffects()
        .find((effect) => effect.id === selectedSceneEffectId())?.label ?? null,
      onLow: setEffectLow,
      onHigh: setEffectHigh,
      onPhase: setEffectPhase,
      onBlendMode: setEffectBlendMode,
      onSubmitEffect: saveSceneEffectDraft,
      onCancelEdit: () => {
        const cue = selectedSceneCue();
        const effectId = selectedSceneEffectId();
        if (cue && effectId !== null) loadSceneEffectDraft(cue.id, effectId);
      },
    },
  }));

  const setNodeGraphEnabled = async (graphId: number, enabled: boolean) => {
    try {
      await invoke("set_node_graph_enabled", { graphId, enabled });
      setMessage(`${enabled ? "Enabled" : "Disabled"} node graph ${graphId}`);
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

  const effectEnableCaptureGenerations = new Map<number, number>();
  const setEffectEnabled = async (effectId: number, enabled: boolean) => {
    const generation = (effectEnableCaptureGenerations.get(effectId) ?? 0) + 1;
    effectEnableCaptureGenerations.set(effectId, generation);
    const previousCaptureTarget = cueEffectCaptureTargets().find((target) => target.effect_id === effectId);
    const mirrorsLiveState = Boolean(previousCaptureTarget)
      && !cueEffectCaptureStateOverrideIds().includes(effectId);
    if (mirrorsLiveState) {
      setCueEffectCaptureTargets((current) => current.map((target) =>
        target.effect_id === effectId ? { ...target, enabled } : target
      ));
    }
    try {
      await invoke("set_effect_enabled", { effectId, enabled });
      setMessage(`${enabled ? "Enabled" : "Disabled"} effect ${effectId}`);
      await refreshSnapshot();
    } catch (error) {
      if (
        mirrorsLiveState
        && previousCaptureTarget
        && effectEnableCaptureGenerations.get(effectId) === generation
        && !cueEffectCaptureStateOverrideIds().includes(effectId)
      ) {
        setCueEffectCaptureTargets((current) => current.map((target) =>
          target.effect_id === effectId && target.enabled === enabled
            ? { ...target, enabled: previousCaptureTarget.enabled }
            : target
        ));
      }
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
    if (paneWindow === "timeline" && mode !== "live") return;
    setControlMode(mode);
    if (mode === "edit") setEditDeskSurface("attributes");
    if (mode === "live") {
      setTimelineDeskSurface("show");
      setTimelineContextDrawer((current) => current === "cue" ? "cue" : "none");
    }
  };

  const selectEditDeskSurface = (surface: EditDeskSurface) => {
    setEditDeskSurface(surface);
    if (surface === "faders") {
      setControlCategory("fader");
    } else if (controlCategory() === "fader") {
      setControlCategory("dimmer");
    }
  };

  const selectTimelineDeskSurface = (surface: TimelineDeskSurface) => {
    setTimelineDeskSurface(surface);
    setTimelineContextDrawer("none");
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

  const handleAppKeyDown = (event: KeyboardEvent) => {
    if (
      event.key === "Escape" &&
      !event.defaultPrevented &&
      document.querySelector("dialog[open]")
    ) {
      event.stopImmediatePropagation();
      return;
    }
    if (
      event.key === "Escape" &&
      !event.defaultPrevented &&
      workspaceTab() === "control" &&
      controlMode() === "live" &&
      timelineContextDrawer() !== "none"
    ) {
      event.preventDefault();
      setTimelineContextDrawer("none");
      return;
    }
    if (
      paneWindow &&
      (
        workspaceTabForShortcut(event.code) !== null ||
        (!isEditableShortcutTarget(event.target) && controlModeForShortcut(event.code) !== null)
      )
    ) {
      event.preventDefault();
      return;
    }
    handleControlKeyDown(event);
  };

  const handleAppContextMenu = (event: MouseEvent) => {
    if (!isEditableContextMenuTarget(event.target)) {
      event.preventDefault();
    }
  };

  let nativeCloseApproved = false;
  let closeRequestListenerDisposed = false;
  let unlistenCloseRequested: (() => void) | undefined;
  const approveNativeCloseOnce = () => {
    nativeCloseApproved = true;
    window.setTimeout(() => {
      nativeCloseApproved = false;
    }, 1000);
  };

  const closeProtectedEditsDirty = () => paneWindow === "timeline"
    ? timelineEditorDirty()
    : !paneWindow && (projectDirty() || timelineEditorDirty());
  const confirmProtectedClose = () => paneWindow === "timeline"
    ? window.confirm(translateUiText(
        "Discard unsaved Timeline edits and close Timeline window?",
        uiLocale(),
      ))
    : confirmDiscardProjectChanges("close Syndocal");

  const handleBeforeUnload = (event: BeforeUnloadEvent) => {
    if (nativeCloseApproved || !closeProtectedEditsDirty()) {
      return;
    }
    event.preventDefault();
    event.returnValue = "";
  };

  window.addEventListener("keydown", handleAppKeyDown);
  window.addEventListener("contextmenu", handleAppContextMenu, true);
  if (!paneWindow || paneWindow === "timeline") {
    window.addEventListener("beforeunload", handleBeforeUnload);
  }
  if (isTauriRuntime() && (!paneWindow || paneWindow === "timeline")) {
    void getCurrentWindow()
      .onCloseRequested((event) => {
        if (!closeProtectedEditsDirty()) {
          return;
        }
        if (confirmProtectedClose()) {
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
    window.removeEventListener("keydown", handleAppKeyDown);
    window.removeEventListener("contextmenu", handleAppContextMenu, true);
    window.removeEventListener("beforeunload", handleBeforeUnload);
  });

  return (
    <main
      class={`app${paneWindow ? ` paneWindow paneWindow-${paneWindow}` : ""}`}
      style={paneWindow ? "grid-template-rows: minmax(0, 1fr) auto !important" : undefined}
      data-pane-window-mode={paneWindow || "main"}
    >
      <WorkspaceChrome
        workspaceTab={workspaceTab()}
        setupSubTab={setupSubTab()}
        blackout={snapshot().blackout}
        videoBlackout={snapshot().video.blackout}
        lightingMaster={snapshot().lighting_master}
        videoMaster={snapshot().video.master_opacity}
        bpm={snapshot().clock.bpm}
        liveAudioInputRunning={liveAudioInputStatus().running}
        liveAudioInputStale={liveAudioInputStatus().stale}
        liveAudioInputSafetyClearPending={liveAudioInputStatus().safety_clear_pending}
        liveAudioInputTelemetryFresh={liveAudioInputTelemetryFresh()}
        liveAudioInputRms={liveAudioInputStatus().rms}
        liveAudioInputPeak={liveAudioInputStatus().peak}
        tickMs={Math.round(snapshot().telemetry.last_tick_interval_us / 1000)}
        jitterUs={Math.round(snapshot().telemetry.tick_jitter_stddev_us)}
        packetBytes={snapshot().telemetry.last_packet_bytes}
        dmxSuccessCount={snapshot().telemetry.last_dmx_send_success_count}
        dmxOutputCount={snapshot().telemetry.last_dmx_output_count}
        projectLabel={projectFileLabel()}
        projectDirty={visibleProjectDirty()}
        currentProjectPath={currentProjectPath()}
        recentProjectPaths={recentProjectPaths()}
        recoveryCheckpoint={projectRecoveryCheckpoint()}
        projectBackups={projectBackups()}
        daslightProjectImportBusy={daslightProjectImportBusy()}
        historyStatus={{
          ...projectHistoryStatus(),
          can_undo: !isfEventPulseBusy() && projectHistoryStatus().can_undo,
          can_redo: !isfEventPulseBusy() && projectHistoryStatus().can_redo,
        }}
        applicationUpdateConfiguration={applicationUpdateConfiguration()}
        applicationUpdateCheck={applicationUpdateCheck()}
        applicationUpdateProgress={applicationUpdateProgress()}
        applicationUpdateBusy={applicationUpdateBusy()}
        applicationUpdateError={applicationUpdateError()}
        uiScale={uiScale()}
        uiLocale={uiLocale()}
        canGo={snapshot().cues.length > 0}
        nextCueLabel={nextCue()?.label ?? "No cue"}
        operatorLockMode={operatorLockMode()}
        operations={
          <WorkspaceOperationsMenu
            profiles={namedWorkspaces()}
            selectedProfileId={selectedNamedWorkspaceId()}
            poppedPanes={poppedPanes()}
            operatorPolicy={operatorPolicy()}
            operatorLockMode={operatorLockMode()}
            onSelectProfile={setSelectedNamedWorkspaceId}
            onSaveProfile={saveNamedWorkspace}
            onApplyProfile={applyNamedWorkspace}
            onDeleteProfile={deleteNamedWorkspace}
            onTogglePane={togglePaneWindow}
            onConfigurePolicy={configureOperatorPolicy}
            onClearPolicy={clearOperatorPolicy}
            onLock={setOperatorSessionLock}
            onUnlock={unlockOperator}
          />
        }
        onWorkspaceTab={setWorkspaceTab}
        onSetupSubTab={selectSetupMode}
        onGo={() => void triggerNextCue()}
        onLightingMaster={setLightingMaster}
        onVideoMaster={setVideoMasterOpacity}
        onTapBpm={tapBpm}
        onOpenLiveAudioInputSettings={openLiveAudioInputSettings}
        onNewProject={newProject}
        onSaveUserTemplate={() => void saveUserTemplate()}
        onLoadUserTemplate={() => void loadUserTemplate()}
        onSaveProject={saveProject}
        onSaveProjectAs={saveProjectAs}
        onLoadProject={loadProject}
        onImportDaslightProject={() => void importDaslightProject()}
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
      <Show when={dvcImportReport()}>
        {(report) => <DvcImportReportPanel report={report()} onClose={() => setDvcImportReport(null)} />}
      </Show>
      <Show when={projectDropState()}>
        <div class={`projectDropOverlay ${projectDropState() === "invalid" ? "invalid" : ""}`}>
          <strong>{projectDropState() === "project" ? "Open Syndocal Project" : "Unsupported File"}</strong>
          <span>{projectDropState() === "project" ? "Drop to load the .sdc project." : "Drop a .sdc project file."}</span>
        </div>
      </Show>

      <section
        class={`layout ${touchLayoutClass()}`}
        style={
          paneWindow
            ? "grid-template-rows: minmax(0, 1fr) !important"
            : undefined
        }
        data-workspace-split-root={sharedWorkspaceVisible() ? "true" : undefined}
        data-upper-lower-ratio={sharedWorkspaceVisible() ? topSplitRatio() : undefined}
        data-lower-left-right-ratio={sharedWorkspaceVisible() ? lowerSplitRatio() : undefined}
      >
        <Show when={workspaceTab() === "control" && controlMode() !== "mixer"}>
        <section
          class={`panel liveControlPanel controlPanel${liveStatusExpanded() ? " liveStatusExpanded" : ""}`}
          data-workspace-pane="upper"
          data-live-status-expanded={liveStatusExpanded() ? "true" : "false"}
        >
          <div
            id="live-status-inspector"
            class="liveStatusGrid"
            classList={{ sceneSettingsVisible: Boolean(selectedSceneCue()) }}
            role="region"
            aria-label="Live status details"
            data-scene-settings-visible={selectedSceneCue() ? "true" : "false"}
          >
            <div class="liveStatusItem liveCueStatusCell">
              <span class="uiMicroLabel">Active cue</span>
              <strong data-no-localize>
                <Show when={activeCue()}>
                  <i
                    class="liveCueIdentityChip"
                    aria-hidden="true"
                    style={{
                      background: cueIdentityCss(
                        activeCue()!.id,
                        activeCue()!.color,
                        "band",
                        activeCue()!.group_id,
                        activeCue()!.group_id ? groupColors()[activeCue()!.group_id!] : null,
                      ),
                    }}
                  />
                </Show>
                {activeCue()?.label ?? "None"}
              </strong>
            </div>
            <div class="liveStatusItem liveCueStatusCell">
              <span class="uiMicroLabel">Next cue</span>
              <strong data-no-localize>
                <Show when={nextCue()}>
                  <i
                    class="liveCueIdentityChip"
                    aria-hidden="true"
                    style={{
                      background: cueIdentityCss(
                        nextCue()!.id,
                        nextCue()!.color,
                        "band",
                        nextCue()!.group_id,
                        nextCue()!.group_id ? groupColors()[nextCue()!.group_id!] : null,
                      ),
                    }}
                  />
                </Show>
                {nextCue()?.label ?? "None"}
              </strong>
            </div>
            <Show when={selectedSceneCue()} fallback={<>
            <div class="liveStatusItem liveDeskSceneStatus">
              <span>Show scenes</span>
              <strong data-live-desk-scene-readout>{timelineTrack()} · {snapshot().cues.length} scenes</strong>
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
            </>}>
              {(cue) => (
                <SceneSettingsPane
                  cue={cue()}
                  groupColors={groupColors()}
                  draft={cueMetadataDraft(cue())}
                  effects={selectedSceneEffects()}
                  selectedEffectId={selectedSceneEffectId()}
                  activeFamily={effectChooserFamily()}
                  activeSurface={sceneSettingsSurface()}
                  running={selectedSceneIsRunning()}
                  liveStates={snapshot().cue_live_modifiers}
                  editor={sceneEffectEditor()}
                  onSurface={setSceneSettingsSurface}
                  onDraft={(patch) => updateCueMetadataDraft(cue(), patch)}
                  onSaveMetadata={() => setCueMetadata(cue())}
                  onSetColor={(color) => setCueColor(cue().id, color)}
                  onSetLiveModifier={setCueLiveModifierLive}
                  onClearLiveModifier={clearCueLiveModifierLive}
                  onSetLiveModifierDefaults={setCueLiveModifierDefaults}
                  onClose={closeSceneSettings}
                  onEditSource={() => openTimelineSourceCue(cue().id)}
                  onSelectEffect={selectSceneEffect}
                  onSelectFamily={createSceneEffect}
                />
              )}
            </Show>
          </div>
          <div class="liveTransportGrid" data-live-desk-toolbar>
            <button
              type="button"
              class="liveDeskIconButton liveTransportIconButton"
              data-live-transport-action="back"
              title="Back"
              aria-label="Back"
              onClick={triggerPreviousCue}
              disabled={snapshot().cues.length === 0}
            >
              <svg viewBox="0 0 20 20" aria-hidden="true">
                <path d="M5 4v12M15 5l-7 5 7 5z" />
              </svg>
            </button>
            <button
              type="button"
              class="liveDeskIconButton liveTransportIconButton"
              data-live-transport-action="fade"
              title={snapshot().active_fade?.paused ? "Resume Fade" : "Pause Fade"}
              aria-label={snapshot().active_fade?.paused ? "Resume Fade" : "Pause Fade"}
              aria-pressed={Boolean(snapshot().active_fade?.paused)}
              onClick={() => void setCueFadePaused(!snapshot().active_fade?.paused)}
              disabled={!snapshot().active_fade}
            >
              <svg viewBox="0 0 20 20" aria-hidden="true">
                <path d="M3.5 15.5 16.5 4.5" />
                <Show
                  when={snapshot().active_fade?.paused}
                  fallback={<path d="M7 5.5 14 10l-7 4.5z" />}
                >
                  <path d="M7 5.5v9M12.5 5.5v9" />
                </Show>
              </svg>
            </button>
            <button
              type="button"
              class="liveDeskIconButton liveTransportIconButton"
              data-live-transport-action="timeline"
              title={snapshot().timeline.playing ? "Pause Timeline" : "Play Timeline"}
              aria-label={snapshot().timeline.playing ? "Pause Timeline" : "Play Timeline"}
              aria-pressed={snapshot().timeline.playing}
              onClick={() => void (snapshot().timeline.playing ? pauseTimeline() : playTimeline())}
              disabled={!snapshot().timeline.playing && snapshot().timeline.duration_ms === 0}
            >
              <svg viewBox="0 0 20 20" aria-hidden="true">
                <path d="M3.5 16h13M5 14v4M10 14v4M15 14v4" />
                <Show
                  when={snapshot().timeline.playing}
                  fallback={<path d="M7 4.5 14 9l-7 4.5z" />}
                >
                  <path d="M7 4.5v9M12.5 4.5v9" />
                </Show>
              </svg>
            </button>
            <button
              class={`killButton${snapshot().blackout ? " engaged" : ""}`}
              data-live-transport-action="dmx-blackout"
              onClick={() => void setBlackout(!snapshot().blackout)}
            >
              {snapshot().blackout ? "Clear DMX BO" : "DMX BO"}
            </button>
            <button
              class={`killButton${snapshot().video.blackout ? " engaged" : ""}`}
              data-live-transport-action="video-blackout"
              onClick={() => void setVideoBlackout(!snapshot().video.blackout)}
            >
              {snapshot().video.blackout ? "Clear Video BO" : "Video BO"}
            </button>
            <button
              class={`killButton${snapshot().blackout && snapshot().video.blackout ? " engaged" : ""}`}
              data-live-transport-action="all-blackout"
              onClick={() => void setAllBlackout(true)}
              disabled={snapshot().blackout && snapshot().video.blackout}
            >
              All BO
            </button>
            <button
              class="killClear"
              data-live-transport-action="all-clear"
              onClick={() => void setAllBlackout(false)}
              disabled={!snapshot().blackout && !snapshot().video.blackout}
            >
              All Clear
            </button>
            <button
              type="button"
              class="liveDeskIconButton liveTransportIconButton"
              data-live-transport-action="clear-flags"
              title="Clear Flags"
              aria-label="Clear Flags"
              disabled={!globalFixtureFlagState().anyFlagged}
              onClick={() => void clearFixtureFlags("all")}
            >
              <svg viewBox="0 0 20 20" aria-hidden="true">
                <path d="M5 17V3.5M5 4h8l-1.5 3L13 10H5M10 13l5 5M15 13l-5 5" />
              </svg>
            </button>
            <div class="liveDeskToolbarActions" data-live-desk-toolbar-actions>
              <button
                type="button"
                class={`liveDeskIconButton liveDeskViewIconButton liveCueEditorToggle${timelineContextDrawer() === "cue" ? " active" : ""}`}
                data-scene-matrix-open-cue-editor
                data-live-desk-view-action="cue-editor"
                aria-label="Cue editor"
                title="Cue editor"
                aria-pressed={timelineContextDrawer() === "cue"}
                onClick={openCueEditor}
              >
                <svg viewBox="0 0 20 20" aria-hidden="true">
                  <path d="M4 4.5h8M4 8h7M4 11.5h4M11 15.5l4.8-4.8 1.5 1.5-4.8 4.8-2.5.5z" />
                </svg>
              </button>
              <nav class="liveDeskViewToggle" aria-label="Live desk view">
                <button
                  type="button"
                  class={`liveDeskIconButton liveDeskViewIconButton${controlLiveView() === "matrix" ? " active" : ""}`}
                  data-live-desk-view-action="matrix"
                  title="Matrix"
                  aria-label="Matrix"
                  aria-pressed={controlLiveView() === "matrix"}
                  onClick={() => setControlLiveView("matrix")}
                >
                  <svg viewBox="0 0 20 20" aria-hidden="true">
                    <path d="M3.5 3.5h5v5h-5zM11.5 3.5h5v5h-5zM3.5 11.5h5v5h-5zM11.5 11.5h5v5h-5z" />
                  </svg>
                </button>
                <button
                  type="button"
                  class={`liveDeskIconButton liveDeskViewIconButton${controlLiveView() === "pads" ? " active" : ""}`}
                  data-live-desk-view-action="cue-pads"
                  title="Cue Pads"
                  aria-label="Cue Pads"
                  aria-pressed={controlLiveView() === "pads"}
                  onClick={() => setControlLiveView("pads")}
                >
                  <svg viewBox="0 0 20 20" aria-hidden="true">
                    <path d="M4 4h3v3H4zM8.5 4h3v3h-3zM13 4h3v3h-3zM4 8.5h3v3H4zM8.5 8.5h3v3h-3zM13 8.5h3v3h-3zM4 13h3v3H4zM8.5 13h3v3h-3zM13 13h3v3h-3z" />
                  </svg>
                </button>
              </nav>
              <button
                type="button"
                class="liveDeskIconButton liveDeskViewIconButton liveStatusToggle"
                data-live-status-toggle
                data-live-desk-view-action="status"
                aria-controls="live-status-inspector"
                aria-expanded={liveStatusExpanded()}
                aria-label={liveStatusExpanded() ? "Hide live status details" : "Show live status details"}
                title={liveStatusExpanded() ? "Hide live status details" : "Show live status details"}
                onClick={() => setLiveStatusExpanded((expanded) => !expanded)}
              >
                <svg viewBox="0 0 20 20" aria-hidden="true">
                  <path d="M3.5 3.5h13v13h-13zM7 6v8M9.5 6.5H14M9.5 10H14M9.5 13.5H14" />
                </svg>
              </button>
            </div>
          </div>
          <Show when={controlLiveView() === "matrix"}>
            <SceneMatrixPanel
              cues={snapshot().cues}
              groupColors={groupColors()}
              onSetGroupColor={setGroupColor}
              groupIds={sceneMatrixGroupIds()}
              activeCueId={snapshot().active_cue_id}
              activeGroupCueIds={snapshot().active_group_cue_ids ?? {}}
              selectedCueId={selectedSceneCueId()}
              activeFade={snapshot().active_fade}
              cueLiveModifiers={snapshot().cue_live_modifiers}
              onSetCueLiveModifier={setCueLiveModifierLive}
              onClearCueLiveModifier={clearCueLiveModifierLive}
              onReleaseCue={releaseCueById}
              timelineTrack={timelineTrack()}
              onTriggerCue={triggerCue}
              onSelectCue={selectSceneCue}
              onOpenSuperScene={(cueId) => void openOrCreateSuperScene(cueId)}
              onOpenCueEditor={openCueEditor}
              onBeginTimelineCueDrag={beginTimelineCueDrag}
              onMoveTimelineCueDrag={moveTimelineCueDrag}
              onEndTimelineCueDrag={(point, moved, canceled) => void endTimelineCueDrag(point, moved, canceled)}
            />
          </Show>
          <Show when={controlLiveView() === "pads"}>
            <div class="liveCuePadSurface">
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
                      style={pad.cue ? {
                        "--identity": cueIdentityCss(
                          pad.cue.id,
                          pad.cue.color,
                          "text",
                          pad.cue.group_id,
                          pad.cue.group_id ? groupColors()[pad.cue.group_id] : null,
                        ),
                      } : undefined}
                      disabled={!pad.cue}
                      onClick={() => {
                        if (pad.cue) {
                          void triggerCue(pad.cue.id);
                        }
                      }}
                    >
                      <span>{pad.slot}</span>
                      <strong>{pad.cue?.label ?? "Empty"}</strong>
                      <small>{pad.cue ? `#${pad.index + 1} / ${displayNumber(pad.cue.fade_ms, 0)} ms` : "-"}</small>
                    </button>
                  )}
                </For>
              </div>
            </div>
          </Show>
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
        </section>
        </Show>
        <Show when={workspaceTab() === "touch"}>
        <TouchSafetyDeck
          snapshot={snapshot()}
          onTriggerPreviousCue={triggerPreviousCue}
          onTriggerNextCue={triggerNextCue}
          onSetCueFadePaused={setCueFadePaused}
          onSetLightingMaster={setLightingMaster}
          onSetVideoMasterOpacity={setVideoMasterOpacity}
          onSetBlackout={setBlackout}
          onSetVideoBlackout={setVideoBlackout}
          onSetAllBlackout={setAllBlackout}
        />
        <EditableTouchSurface
          snapshot={snapshot()}
          surface={snapshot().touch_surface}
          selectedFixtureId={selectedFixtureId()}
          onSurfaceChange={setTouchSurfaceLayout}
          onTrigger={triggerTouchBinding}
          onValue={setTouchBindingValue}
          onColor={setTouchBindingColor}
          onXy={setTouchBindingXy}
          onReleaseCue={releaseCueById}
          onSetCueLiveModifier={setCueLiveModifierLive}
          onClearCueLiveModifier={clearCueLiveModifierLive}
        />
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
          onTriggerCue={triggerCue}
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
            fixtures={mappingStageFixtures()}
            videoSurfaces={visualizerVideoSurfaces2d()}
            stageObjects={visualizerStageObjects2d()}
            selectedFixtureId={selectedFixtureId()}
            selectedVideoOutputId={selectedVideoOutputId()}
            selectedFixtureGroupFilter={selectedFixtureGroupFilter()}
            stageObjectClassName="touchStageObject"
            surfaceMinOpacity={0.2}
            beamMinOpacity={0.06}
            beamIntensityScale={0.48}
            showBeams={mappingShowBeams()}
            showLabels={mappingShowLabels()}
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
          groupColors={groupColors()}
          fixtures={filteredFixtures()}
          totalFixtureCount={snapshot().fixtures.length}
          selectedFixtureId={selectedFixtureId()}
          groupFlagState={selectedGroupFlagState()}
          globalAnyFlagged={globalFixtureFlagState().anyFlagged}
          groupStrobeHz={selectedGroupSubmaster()?.strobe_hz ?? 0}
          groupStrobeFixtureCount={selectedGroupStrobeFixtureCount()}
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
          onSetGroupStrobe={setGroupStrobe}
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
            foldSources={setupSubTab() === "patch"}
            gdtfPath={gdtfPath()}
            gdtfShareUrl={gdtfShareUrl()}
            onGdtfPath={setGdtfPath}
            onGdtfShareUrl={setGdtfShareUrl}
            onBrowse={selectGdtfFile}
            onLoadGdtf={importGdtf}
            onDownloadGdtf={downloadGdtfFromUrl}
          />
          <Show when={setupSubTab() === "library"}>
            <FixtureCatalogPanel
              backendAvailable={isTauriRuntime() && !viewportFixture}
              selectedFixtureId={selectedFixtureId()}
              selectedProfile={profile()}
              selectedMode={selectedMode()}
              onProfileLoaded={useCatalogProfile}
              onRepair={repairCatalogFixtureProfile}
              onMessage={setMessage}
            />
          </Show>
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
              </div>
            )}
          </Show>
        </aside>
        </Show>

        <Show when={workspaceTab() === "setup" && setupSubTab() === "patch"}>
        <SetupMappingWorkspace
          className={setupPanelClass("panel fixtures setupPanel", ["patch"])}
          panelRef={registerSetupPanel(["patch"])}
          patchForm={
            <Show when={profile()}>
              {(loaded) => (
                <PatchFixtureFormPanel
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
                  onPatch={patchFixture}
                  onNextFreeAddress={selectNextFreePatchAddress}
                />
              )}
            </Show>
          }
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
            onUseProfileForPatch: useFixtureProfileForPatch,
            onDuplicateFixture: duplicateFixture,
            onLabelDraft: setSelectedFixtureLabelDraft,
            onUniverseDraft: setSelectedFixtureUniverseDraft,
            onAddressDraft: setSelectedFixtureAddressDraft,
            onApplyPatch: setFixturePatch,
            onGroupText: setSelectedFixtureGroupText,
            onApplyGroups: setFixtureGroups,
          } : null}
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
          liveAudioInput={{
            get liveAudioInputBackends() { return liveAudioInputBackends(); },
            get selectedLiveAudioInputBackend() { return selectedLiveAudioInputBackend(); },
            get liveAudioInputBackendsKnown() { return liveAudioInputBackendsKnown(); },
            get liveAudioInputBackendsBusy() { return liveAudioInputBackendsBusy(); },
            get liveAudioInputBackendError() { return liveAudioInputBackendError(); },
            get liveAudioInputDevices() { return liveAudioInputDevices(); },
            get selectedLiveAudioInputDevice() { return selectedLiveAudioInputDevice(); },
            get liveAudioInputCapabilities() { return liveAudioInputCapabilities(); },
            get liveAudioInputCapabilitiesBusy() { return liveAudioInputCapabilitiesBusy(); },
            get liveAudioInputSampleRate() { return liveAudioInputSampleRate(); },
            get liveAudioInputBufferFrames() { return liveAudioInputBufferFrames(); },
            get liveAudioInputChannelMix() { return liveAudioInputChannelMix(); },
            get liveAudioInputStatus() { return liveAudioInputStatus(); },
            get liveAudioInputStatusKnown() { return liveAudioInputStatusKnown(); },
            get liveAudioInputBusy() { return liveAudioInputBusy(); },
            onSetLiveAudioInputBackend: selectLiveAudioInputBackend,
            onSetLiveAudioInputDevice: selectLiveAudioInputDevice,
            onSetLiveAudioInputSampleRate: selectLiveAudioInputSampleRate,
            onSetLiveAudioInputBufferFrames: setLiveAudioInputBufferFrames,
            onSetLiveAudioInputChannelMix: setLiveAudioInputChannelMix,
            onRefreshLiveAudioInputDevices: refreshLiveAudioInputDevices,
            onStartLiveAudioInput: startLiveAudioInput,
            onStopLiveAudioInput: stopLiveAudioInput,
          }}
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

        <Show when={workspaceTab() === "control" && controlMode() === "mixer"}>
        <VideoControlPanel
          mixer={controlMode() === "mixer"}
          layerCount={snapshot().video.layers.length}
          modeTabs={
            <ControlModeSegment
              class="mixerContextModeTabs"
              controlMode={controlMode()}
              operatorLockMode={operatorLockMode()}
              onControlMode={selectControlMode}
            />
          }
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
          liveMonitors={{
            get preview() { return liveVideoMonitors.preview(); },
            get program() { return liveVideoMonitors.program(); },
            get previewLabel() { return liveVideoPreviewLabel(); },
            get programLabel() { return liveVideoProgramLabel(); },
            get previewTransport() { return vjPreviewTransport(); },
            get previewTransportBusy() { return vjPreviewTransportBusy(); },
            get previewTransportError() { return vjPreviewTransportError(); },
            previewTransportBackendAvailable: isTauriRuntime(),
            onSetPreviewPlaying: setVjPreviewPlaying,
            onSeekPreview: seekVjPreview,
            onSetPreviewSpeed: setVjPreviewSpeed,
            onClearPreview: clearVjPreview,
            onRetry: liveVideoMonitors.retry,
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
            get liveAudioInputBackends() { return liveAudioInputBackends(); },
            get selectedLiveAudioInputBackend() { return selectedLiveAudioInputBackend(); },
            get liveAudioInputBackendsKnown() { return liveAudioInputBackendsKnown(); },
            get liveAudioInputBackendsBusy() { return liveAudioInputBackendsBusy(); },
            get liveAudioInputBackendError() { return liveAudioInputBackendError(); },
            get liveAudioInputDevices() { return liveAudioInputDevices(); },
            get selectedLiveAudioInputDevice() { return selectedLiveAudioInputDevice(); },
            get liveAudioInputCapabilities() { return liveAudioInputCapabilities(); },
            get liveAudioInputCapabilitiesBusy() { return liveAudioInputCapabilitiesBusy(); },
            get liveAudioInputSampleRate() { return liveAudioInputSampleRate(); },
            get liveAudioInputBufferFrames() { return liveAudioInputBufferFrames(); },
            get liveAudioInputChannelMix() { return liveAudioInputChannelMix(); },
            get liveAudioInputStatus() { return liveAudioInputStatus(); },
            get liveAudioInputStatusKnown() { return liveAudioInputStatusKnown(); },
            get liveAudioInputBusy() { return liveAudioInputBusy(); },
            get previewLayerId() { return videoPreviewLayerId(); },
            get previewBusy() { return vjPreviewTransportBusy(); },
            get previewError() { return vjPreviewTransportError(); },
            previewBackendAvailable: isTauriRuntime(),
            get firstRunAvailable() { return vjFirstRunAvailable(); },
            get firstRunBusy() { return vjFirstRunBusy(); },
            get firstRunError() { return vjFirstRunError(); },
            firstRunBackendAvailable: isTauriRuntime(),
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
            onSetLiveAudioInputBackend: selectLiveAudioInputBackend,
            onSetLiveAudioInputDevice: selectLiveAudioInputDevice,
            onSetLiveAudioInputSampleRate: selectLiveAudioInputSampleRate,
            onSetLiveAudioInputBufferFrames: setLiveAudioInputBufferFrames,
            onSetLiveAudioInputChannelMix: setLiveAudioInputChannelMix,
            onRefreshLiveAudioInputDevices: refreshLiveAudioInputDevices,
            onStartLiveAudioInput: startLiveAudioInput,
            onStopLiveAudioInput: stopLiveAudioInput,
            onStagePreview: stageVjPreviewLayer,
            onImportMedia: importMediaFiles,
            onCreateFirstRunShow: createFirstRunVjShow,
            onLaunch: launchVideoClipFromGrid,
            onTake: takeVideoClipFromGrid,
            onStop: stopVideoClipFromGrid,
            onMonitorAudio: (layerId, volume) => playVideoLayerAudioMonitor(layerId, volume, selectedAudioOutputDevice()),
            onStopAudio: stopVideoLayerAudioMonitor,
          }}
          autoVj={{
            get snapshot() { return snapshot().video.auto_vj ?? emptyAutoVjSnapshot; },
            get layers() { return snapshot().video.layers; },
            get busy() { return autoVjBusy(); },
            onSetConfig: setAutoVjConfig,
            onSetArmed: setAutoVjArmed,
            onSetHold: setAutoVjHold,
          }}
          audioReactive={{
            get graphs() { return snapshot().node_graphs; },
            onSetEnabled: setNodeGraphEnabled,
          }}
          layerList={{
            get layers() { return snapshot().video.layers; },
            get isfRuntimeErrors() { return videoPreviewDiagnostics()?.isf_stage_errors ?? []; },
            get isfEventPulseBusy() { return isfEventPulseBusy(); },
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
            onApplyBuiltinIsf: applyBuiltinVideoIsfEffect,
            onSetIsfEffect: setVideoLayerIsfEffect,
            onMoveIsfEffect: moveVideoLayerIsfEffect,
            onRemoveIsfEffect: removeVideoLayerIsfEffect,
            onSetIsfEffectEnabled: setVideoLayerIsfEffectEnabled,
            onResetIsfEffect: resetVideoLayerIsfEffect,
            onSetIsfControl: setVideoLayerIsfControl,
            onTriggerIsfEvent: triggerVideoLayerIsfEvent,
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
        </Show>

        <Show when={sharedWorkspaceVisible() && !paneWindow}>
        <WorkspaceSplitHandle
          axis="horizontal"
          ratio={topSplitRatio()}
          defaultRatio={defaultWorkspaceLayout.top_split_ratio}
          minFirstPx={280}
          minSecondPx={310}
          firstTrackBonusPx={workspaceTab() === "control" ? 36 : 0}
          label="Resize upper and lower workspace panes"
          splitter="upper-lower"
          onCommit={setTopSplitRatio}
        />
        </Show>

        <Show when={sharedWorkspaceVisible()}>
        <MappingPersistentWorkspaceBand
          poppedPanes={poppedPanes()}
          lowerSplitRatio={lowerSplitRatio()}
          selectionsDrawerOpen={selectionsDrawerOpen()}
          onTogglePaneWindow={togglePaneWindow}
          onLowerSplitRatio={setLowerSplitRatio}
          onSelectionsDrawerOpen={setSelectionsDrawerOpen}
          onOpenMapping={() => {
            setWorkspaceTab("setup");
            selectSetupMode("patch");
          }}
          workspace={workspaceTab() === "control" ? "control" : "setup"}
          controlMode={controlMode()}
          controlHeaderTitle={faderDeskTitle()}
          controlHeaderTools={
            <>
              <Show when={controlMode() === "live"}>
                <nav class="timelineDeskTabs" aria-label="Timeline desk surface">
                  <button
                    type="button"
                    class={timelineDeskSurface() === "show" ? "active" : ""}
                    title="Show Timeline"
                    aria-label="Show Timeline"
                    aria-pressed={timelineDeskSurface() === "show"}
                    data-timeline-desk-surface="show"
                    onClick={() => selectTimelineDeskSurface("show")}
                  >
                    <span class="timelineToolIcon" aria-hidden="true" data-no-localize>▤</span>
                  </button>
                  <button
                    type="button"
                    class={timelineDeskSurface() === "automation" ? "active" : ""}
                    title="Automation"
                    aria-label="Automation"
                    aria-pressed={timelineDeskSurface() === "automation"}
                    data-timeline-desk-surface="automation"
                    onClick={() => selectTimelineDeskSurface("automation")}
                  >
                    <span class="timelineToolIcon" aria-hidden="true" data-no-localize>∿</span>
                  </button>
                  <button
                    type="button"
                    class={timelineDeskSurface() === "playback" ? "active" : ""}
                    title="Playback"
                    aria-label="Playback"
                    aria-pressed={timelineDeskSurface() === "playback"}
                    data-timeline-desk-surface="playback"
                    onClick={() => selectTimelineDeskSurface("playback")}
                  >
                    <span class="timelineToolIcon" aria-hidden="true" data-no-localize>▦</span>
                  </button>
                </nav>
              </Show>
              <Show when={controlMode() === "edit"}>
                <nav class="editDeskTabs" aria-label="Live edit desk surface">
                  <button
                    class={editDeskSurface() === "faders" ? "active" : ""}
                    aria-pressed={editDeskSurface() === "faders"}
                    onClick={() => selectEditDeskSurface("faders")}
                  >
                    Faders
                  </button>
                  <button
                    class={editDeskSurface() === "attributes" ? "active" : ""}
                    aria-pressed={editDeskSurface() === "attributes"}
                    onClick={() => selectEditDeskSurface("attributes")}
                  >
                    Attributes
                  </button>
                </nav>
                <ControlFaderWriteHeader
                  targetKind={controlTargetKind()}
                  compactReadout={controlCompactReadout()}
                  writeMode={controlFaderWriteMode()}
                  editingSceneLabel={selectedSceneCue()?.label ?? null}
                  editingSceneIdentity={
                    selectedSceneCue()
                      ? cueIdentityCss(
                          selectedSceneCue()!.id,
                          selectedSceneCue()!.color,
                          "fill",
                          selectedSceneCue()!.group_id,
                          selectedSceneCue()!.group_id ? groupColors()[selectedSceneCue()!.group_id!] : null,
                        )
                      : null
                  }
                  editingSceneIdentityText={
                    selectedSceneCue()
                      ? cueIdentityCss(
                          selectedSceneCue()!.id,
                          selectedSceneCue()!.color,
                          "text",
                          selectedSceneCue()!.group_id,
                          selectedSceneCue()!.group_id ? groupColors()[selectedSceneCue()!.group_id!] : null,
                        )
                      : null
                  }
                  onWriteMode={changeControlFaderWriteMode}
                />
              </Show>
            </>
          }
          operatorLockMode={operatorLockMode()}
          onControlMode={selectControlMode}
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
            showLabels: mappingShowLabels(),
            showBeams: mappingShowBeams(),
            showGeometry: mappingShowGeometry(),
            showProjectors: mappingShowProjectors(),
            showStageObjects: mappingShowStageObjects(),
            showLevels: mappingShowLevels(),
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
            zoomValue: normalizedMappingViewportZoom(),
            zoomMax: mappingViewportMaxZoom(),
            canZoomOut: normalizedMappingViewportZoom() > 1.001,
            canZoomIn: normalizedMappingViewportZoom() < mappingViewportMaxZoom() - 0.001,
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
            onZoomLevel: (zoom) => setMappingViewport(zoom),
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
          editableStage={{
            svgRef: bindMappingStageSvgElement,
            dragging: Boolean(mappingDrag() || mappingViewportPanDrag()),
            stageTool: mappingStageTool(),
            viewBox: mappingStageViewBox(),
            stageWorldBounds: stageWorldBounds(),
            stageOrigin: stageOrigin2d(),
            snapEnabled: mappingSnapEnabled(),
            snapLines: mappingSnapLines(),
            marqueeBox: mappingMarqueeBox(),
            onPointerDown: handleMappingStagePointerDown,
            onAuxClick: handleMappingStageAuxClick,
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
            labelViewport: {
              x: mappingViewportBox().x,
              z: mappingViewportBox().z,
              width: mappingViewportBox().size,
              height: mappingViewportBox().size,
            },
            labelViewportPixelSize: mappingStageViewportPixelSize(),
            labelZoom: normalizedMappingViewportZoom(),
            showLevels: mappingShowLevels(),
            stageTool: mappingStageTool(),
            stageObjects: visualizerStageObjects2d(),
            videoSurfaces: visualizerVideoSurfaces2d(),
            beamFixtures: mappingStageFixtures().filter((fixture) => fixture.liveColorApplied),
            geometryNodes: mappingGeometryNodes2d(),
            fixtures: mappingStageFixtures(),
            labelFixtures: visualizerFixtures(),
            selectedFixtureIds: selectedMappingFixtureIdSet(),
            selectedFixtureId: selectedFixtureId(),
            selectedGroupId: selectedFixtureGroupFilter(),
            selectedTypeKey: selectedFixtureTypeFilter(),
            selectedVideoOutputId: selectedVideoOutputId(),
            placePreview: mappingPlacePreview(),
            isDraggingStageObject: isDraggingMappingStageObject,
            isDraggingFixture: isDraggingMappingFixture,
            isYawDragging: (fixtureId) => {
              const drag = mappingDrag();
              return drag?.kind === "fixtureYaw" && drag.fixtureId === fixtureId;
            },
            onBeginStageObjectDrag: beginMappingStageObjectDrag,
            onBeginStageObjectRotate: beginMappingStageObjectRotate,
            onBeginStageObjectResize: beginMappingStageObjectResize,
            onSelectVideoOutput: setSelectedVideoOutputId,
            onBeginFixtureYawDrag: beginMappingFixtureYawDrag,
            onFixturePointerDown: (event, fixtureId) => {
              if (event.button === 1 || mappingStageTool() === "pan") return;
              event.stopPropagation();
              const fixture = snapshot().fixtures.find((candidate) => candidate.id === fixtureId);
              if (workspaceTab() === "control") {
                beginMappingFixtureDrag(event, fixtureId);
                return;
              }
              if (fixture && mappingStageTool() === "rotate") selectMappingFixture(fixture, event);
              beginMappingFixtureDrag(event, fixtureId);
            },
          }}
          selection={{
            selectedFixtureCount: selectedMappingFixtures().length,
            selectedFixtures: selectedMappingFixtures(),
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
            fixtureLimitsEditor: selectedMappingFixture() ? {
              fixture: selectedMappingFixture()!,
              limitsDraft: selectedFixtureLimitsDraft(),
              normalizedLimits: normalizedSelectedFixtureLimitsDraft(),
              limitWindowStyle: selectedFixtureLimitWindowStyle(),
              movementLimitDragging: Boolean(movementLimitDrag()),
              formatDmxPercent,
              onUpdateNumericLimit: (field, value) => updateSelectedFixtureLimit(field, value),
              onUpdateToggleLimit: (field, value) => updateSelectedFixtureLimit(field, value),
              onResetLimits: () => setSelectedFixtureLimitsDraft(defaultFixtureLimits),
              onApplyLimits: setFixtureLimits,
              onMovementLimitPointerDown: startMovementLimitDrag,
              onMovementLimitPointerMove: dragMovementLimit,
              onMovementLimitPointerEnd: endMovementLimitDrag,
            } : null,
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
            onLayoutFixtures: layoutFixturePositions,
            onControlActive: () => setWorkspaceTab("control"),
            onOpenSceneFx: openSceneFxFromMapping,
            onSetFixtureTransform: setFixtureTransform,
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
            onEditOutputProjection: (outputId) => {
              setSelectedVideoOutputId(outputId);
              selectSetupMode("video");
            },
          }}
          hotkeyHelpOpen={mappingHotkeyHelpOpen()}
          onCloseHotkeyHelp={() => setMappingHotkeyHelpOpen(false)}
        >
        <Show when={workspaceTab() === "control"}>
        <section
          class={`panel faders controlPanel timelineDesk-${timelineDeskSurface()} timelineDrawer-${timelineContextDrawer()} editDesk-${editDeskSurface()}`}
          data-timeline-context-drawer={timelineContextDrawer()}
        >
          <Show when={controlMode() === "live"}>
            <GroupLiveMixerStrip
              groupId={selectedFixtureGroupFilter()}
              fixtureCount={selectedGroupFixtures().length}
              strobeFixtureCount={selectedGroupStrobeFixtureCount()}
              submasterLevel={selectedGroupSubmaster()?.level ?? 1}
              strobeHz={selectedGroupSubmaster()?.strobe_hz ?? 0}
              soloed={selectedGroupFlagState().anySoloed}
              onSetSubmaster={setGroupSubmaster}
              onSetStrobe={setGroupStrobe}
              onSetSolo={setGroupSolo}
            />
          </Show>
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
            categories={controlCategoryRows().filter((category) =>
              editDeskSurface() === "attributes" ? category.id !== "fader" : category.id === "fader"
            )}
            activeCategory={activeControlCategory()}
            onCategory={(category) => {
              setControlCategory(category);
              setEditDeskSurface(category === "fader" ? "faders" : "attributes");
            }}
          >
          <Show
            when={showFixtureTypeAttributeColumns()}
            fallback={
              <>
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
            showColorWheelPicker={showColorWheelPickerPanel()}
            showGoboWheel={showGoboWheelPanel()}
            showOptics={showOpticsPanel()}
            showCategoryQuick={showCategoryQuickPanel()}
            showFunctions={activeControlCategory() !== "fader"}
            targetLabel={controlTargetLabel()}
            categoryLabel={activeControlCategoryLabel()}
            attributeCount={visibleControls().length}
            colorWheelEntries={colorWheelEntries()}
            goboWheelEntries={goboWheelEntries()}
            colorWheelPickerColor={colorWheelPickerColor()}
            colorWheelPickerHsv={colorWheelPickerHsv()}
            colorWheelSaturationRamp={colorWheelSaturationRamp()}
            colorWheelApproximationLabel={colorWheelApproximationLabel()}
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
            onColorWheelPointerColor={setColorWheelPickerFromPointer}
            onSetColorWheelColor={setColorWheelPickerColor}
            onSetColorWheelHsv={setColorWheelPickerHsvValue}
            onOpticsPointerValue={setOpticsValueFromPointer}
            onOpticsKeyValue={setOpticsValueFromKey}
            onSetValue={setControlAttributeValue}
            onApplyColorFunction={applyColorWheelFunction}
            onApplyFunction={(control, fn) => void applyChannelFunction(control, fn)}
            onSetValueMode={(mode) => void applyVisibleControlValues(mode)}
            onApplyLook={(look) => void applyCategoryQuickLook(look)}
          />
          <FaderGridPanel
            controls={visibleControls()}
            fullWidth={activeControlCategory() === "fader"}
            selectedFixtureId={selectedControlReferenceFixture()?.id ?? null}
            selectedGroupId={selectedFixtureGroupFilter()}
            valueForControl={(control) => {
              const fixture = selectedControlReferenceFixture();
              return fixture ? faderValue(fixture.id, control.attribute, control.default_value) : control.default_value;
            }}
            isControlWritten={controlIsWritten}
            onSetFixtureAttribute={setAttribute}
            onSetGroupAttribute={setGroupAttribute}
          />
              </>
            }
          >
            <FixtureTypeAttributeColumns
              groups={pickedFixtureTypeGroups()}
              activeCategory={activeControlCategory()}
              valueForControl={(fixture, control) =>
                faderValue(fixture.id, control.attribute, control.default_value)
              }
              onSelectType={selectPickedFixtureType}
              onSetControl={setFixtureTypeControlValue}
              onResetControl={resetFixtureTypeControlValue}
              onSetColor={setFixtureTypeColor}
              onSetPosition={setFixtureTypePosition}
            />
          </Show>
          </FaderAttributeEditorPanel>
          <Show when={controlMode() === "live" && timelineContextDrawer() === "cue"}>
          <aside
            class="timelineContextDrawer"
            data-timeline-context-drawer-panel="cue"
            aria-label="Cue editor drawer"
          >
            <header class="timelineContextDrawerHeader">
              <div>
                <strong>Cue editor</strong>
                <span data-no-localize>{selectedCueList().label}</span>
              </div>
              <button
                type="button"
                class="timelineContextDrawerClose"
                aria-label="Close Cue editor"
                title="Close Cue editor"
                onClick={() => setTimelineContextDrawer("none")}
              >
                <span aria-hidden="true" data-no-localize>×</span>
              </button>
            </header>
            <div class="timelineContextDrawerBody">
          <CueManagementPanel
            mode={controlMode() === "live" ? "live" : "edit"}
            onSetCueColor={setCueColor}
            groupColors={groupColors()}
            onSetCueLiveModifierDefaults={setCueLiveModifierDefaults}
            cues={selectedCueListCues()}
            allCues={snapshot().cues}
            cueLists={snapshot().cue_lists}
            groupIds={fixtureGroupRows().map((row) => row.groupId)}
            palettes={snapshot().palettes}
            effects={snapshot().effects}
            cueCaptureEffects={cueCaptureEligibleEffects()}
            selectedCueListId={selectedCueList().id}
            cueListLabel={cueListLabel()}
            activeCueId={snapshot().active_cue_id}
            revealCueId={revealedSourceCueId()}
            revealCueRevision={revealedSourceCueRevision()}
            activeFade={snapshot().active_fade}
            timelinePositionMs={snapshot().timeline.position_ms}
            bpm={snapshot().clock.bpm}
            timelineTrack={timelineTrack()}
            cueLabel={cueLabel()}
            cueFadeMs={cueFadeMs()}
            cueAuthoredBeats={cueAuthoredBeats()}
            cueAuthoredBeatsSeeded={cueAuthoredBeatsSeeded()}
            cueAuthoredBeatsError={cueAuthoredBeatsError()}
            cueCaptureScope={cueCaptureScope()}
            cueCaptureScopeError={cueCaptureScopeError()}
            hasCueSources={hasCueSources()}
            cueEffectCaptureTargets={cueEffectCaptureTargets()}
            cueCapturePreview={cueCapturePreview()}
            stageViewBoxSize={stageViewBoxSize}
            stageOrigin={stageOrigin2d()}
            selectedFixtureId={selectedFixtureId()}
            timelinePlacementNudgeMs={timelinePlacementNudgeMs()}
            cueMetadataDraft={cueMetadataDraft}
            cueTimelinePlacementsForCue={cueTimelinePlacementsForCue}
            onCueLabel={setCueLabel}
            onCueFadeMs={setCueFadeMs}
            onCueAuthoredBeats={updateCueAuthoredBeats}
            onCueCaptureScope={setCueCaptureScope}
            onCueEffectCaptureTargets={updateCueEffectCaptureTargets}
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
            onSetCueEffectTargets={setCueEffectTargets}
            onSetCueSteps={setCueSteps}
            onDuplicateCue={duplicateCue}
            onUpdateCue={updateCue}
            onTriggerCue={triggerCue}
            onAddTimelineCueEventAt={addTimelineCueEventAt}
            onRemoveCue={removeCue}
            onSeekTimeline={seekTimeline}
            onOpenTimeline={() => selectTimelineDeskSurface("show")}
            onMoveTimelineCueEvent={moveTimelineCueEvent}
            onRemoveTimelineEvent={removeTimelineEvent}
            onBeginTimelineCueDrag={beginTimelineCueDrag}
            onMoveTimelineCueDrag={moveTimelineCueDrag}
            onEndTimelineCueDrag={(point, moved, canceled) => void endTimelineCueDrag(point, moved, canceled)}
          />
            </div>
          </aside>
          </Show>
          <div class="timelinePanel">
            <div
              class="timelineShowSurface"
              classList={{ timelineShowSurfaceBlockDrawerOpen: timelineContextDrawer() === "block" }}
            >
            <TimelineCueEventsPanel
              contextDrawer={timelineContextDrawer()}
              childTimelineLabel={timelineChildCue()?.label ?? null}
              cueIdentities={cueIdentities()}
              positionMs={activeTimeline().position_ms}
              bpm={snapshot().clock.bpm}
              durationMs={activeTimeline().duration_ms}
              playing={activeTimeline().playing}
              executingLive={timelineExecutionLive()}
              cuesCount={snapshot().cues.length}
              superSceneCueCount={superSceneCueCount()}
              lightingAutomationCount={activeTimeline().automations.length}
              videoAutomationCount={activeTimeline().video_automations.length}
              overviewEvents={timelineOverviewEvents()}
              timelineLayers={timelineLayers()}
              timelineCueDrag={timelineCueDrag()}
              overviewMarkerAriaLabel={(event) => timelineOverviewMarkerAriaLabel(event, uiLocale())}
              overviewAutomationRanges={timelineOverviewAutomationRanges()}
              overviewOverlapClusters={timelineOverviewOverlapClusters()}
              overviewOverlapLayerIds={[
                ...new Set(timelineOverlapClusters().map((cluster) => cluster.layer_id)),
              ]}
              overlapClusterMemberships={timelineOverlapClusters().map((cluster) => ({
                id: cluster.id,
                member_ids: cluster.member_ids.map(Number),
              }))}
              selectedAutomationRangeId={selectedTimelineAutomationRangeId()}
              overviewPlayheadX={timelineOverviewPlayheadX()}
              visibleWindow={timelineVisibleWindow()}
              overviewShowDurationMs={timelineOverviewShowDurationMs()}
              overviewEditExtentMs={timelineOverviewEditExtentMs()}
              selectedEventId={selectedTimelineSceneBlockEventId()}
              selectionRevision={timelineSceneBlockSelectionRevision()}
              audioAnalysis={audioAnalysis()}
              audioClips={activeTimeline().audio_clips ?? []}
              audioOffsetMs={activeTimeline().audio_offset_ms ?? 0}
              audioMuted={activeTimeline().audio_muted ?? false}
              audioWaveformPoints={audioWaveformPoints()}
              audioSpectrumPaths={audioSpectrumPaths()}
              audioBeatMarkers={audioBeatMarkers()}
              snapMode={timelineSnapMode()}
              gridMs={timelineGridMs()}
              selectedCueId={selectedTimelineCueId()}
              selectedCueIsSuperScene={Boolean(
                snapshotCues().find((cue) => cue.id === selectedTimelineCueId())?.child_timeline,
              )}
              eventTimeMs={timelineEventTimeMs()}
              blockDurationMs={timelineBlockDurationMs()}
              blockLoopCount={timelineBlockLoopCount()}
              blockJumpToEventId={timelineBlockJumpToEventId()}
              track={timelineTrack()}
              cueOptions={timelineCueOptions()}
              eventRows={timelineEventRows()}
              timelineEventDraft={timelineEventDraft}
              onSeek={seekTimeline}
              onExitChildTimeline={exitSuperScene}
              onOpenSuperScene={(cueId) => void openOrCreateSuperScene(cueId)}
              onPause={pauseTimeline}
              onPlay={playTimeline}
              onSeekOverviewTime={seekTimelineFromOverviewTime}
              onMoveEventPlacement={moveTimelineCueEventToPlacement}
              onResizeEventTime={resizeTimelineCueEventToTime}
              onSetEventFade={setTimelineCueEventFade}
              onSelectAutomationRange={selectTimelineAutomationRange}
              onMoveAutomationRangeTime={moveTimelineAutomationRangeToTime}
              onResizeAutomationRangeTime={resizeTimelineAutomationRangeToTime}
              onMoveAutomationKeyframeTime={moveTimelineAutomationKeyframeToTime}
              onSelectEvent={selectTimelineSceneBlockEvent}
              onFitOverview={fitTimelineOverview}
              onZoomOverview={zoomTimelineOverview}
              onPanOverview={panTimelineOverview}
              onSetVisibleWindow={setTimelineVisibleWindowDirect}
              onZoomOverviewAt={zoomTimelineOverviewAt}
              onRevealSelected={() => {
                const eventId = selectedTimelineSceneBlockEventId();
                if (eventId !== null) revealTimelineSceneBlock(eventId);
              }}
              onRevealPlayhead={revealTimelinePlayhead}
              onAnalyzeAudio={analyzeAudioFile}
              onClearAudio={clearTimelineAudio}
              onApplyAudioBpm={applyAudioBpm}
              onAddAudioClip={addTimelineAudioClip}
              onUpdateAudioClip={updateTimelineAudioClip}
              onRemoveAudioClip={removeTimelineAudioClip}
              onSetAudioMaster={setTimelineAudioMaster}
              onSnapMode={setTimelineSnapMode}
              onGridMs={setTimelineGridMs}
              onSnapDrafts={snapTimelineDrafts}
              onSnapItems={snapTimelineItems}
              snapTimeMs={snapTimeMs}
              onPlaceArmedCue={placeArmedTimelineCue}
              onSelectedCueId={setTimelineCueId}
              onOpenOrCreateSuperScene={async (cueId) => { await openOrCreateSuperScene(cueId); }}
              onEventTimeMs={setTimelineEventTimeMs}
              onBlockDurationMs={setTimelineBlockDurationMs}
              onBlockLoopCount={setTimelineBlockLoopCount}
              onBlockJumpToEventId={setTimelineBlockJumpToEventId}
              onTrack={setTimelineTrack}
              onAddEvent={addTimelineCueEvent}
              onAddEventAtPlayhead={addTimelineCueEventAtPlayhead}
              onUpdateEventDraft={updateTimelineEventDraft}
              onSaveEvent={setTimelineCueEvent}
              onRemoveEvent={removeTimelineEvent}
              onOpenSourceCue={openTimelineSourceCue}
              onAddTimelineLayer={timelineLayerController.add}
              onUpdateTimelineLayer={timelineLayerController.update}
              onRemoveTimelineLayer={removeTimelineLayer}
              onReorderTimelineLayer={timelineLayerController.reorder}
              onTimelineStatus={setMessage}
              onContextDrawer={setTimelineContextDrawer}
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
              fixtureOptions={timelineAutomationFixtureOptions()}
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
        </section>
        </Show>
        </MappingPersistentWorkspaceBand>
        </Show>

        <Show when={workspaceTab() === "setup" && ["dmx", "midi", "osc", "remote"].includes(setupSubTab())}>
        <aside
          class={setupPanelClass("panel output setupIoPanel setupPanel controlPanel", ["dmx", "midi", "osc", "remote"])}
          ref={registerSetupPanel(["dmx", "midi", "osc", "remote"])}
          tabIndex={-1}
        >
          <Show when={setupSubTab() === "dmx"}>
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
          <DmxRawMonitor
            previews={dmxPreviewOptions()}
            activeUniverse={activeDmxPreviewUniverse()}
            activeCount={nonZeroDmxCount()}
            cells={dmxCells()}
            onUniverseChange={setRawDmxUniverse}
          >
            <Show when={visibleFunctionControls().length > 0}>
              <div class="dmxGdtfFunctionReadout">
                <ChannelFunctionPanel
                  categoryLabel={activeControlCategoryLabel()}
                  entries={visibleFunctionControls()}
                  currentValue={currentControlValue}
                  clampDmxValue={clampDmxValue}
                  functionContainsValue={channelFunctionContainsValue}
                  functionBandStyle={channelFunctionBandStyle}
                  functionLabel={channelFunctionLabel}
                  functionRangeLabel={channelFunctionRangeLabel}
                  functionDetail={channelFunctionDetail}
                  functionSwatchColor={channelFunctionSwatchColor}
                  onApplyFunction={(control, fn) => void applyChannelFunction(control, fn)}
                />
              </div>
            </Show>
          </DmxRawMonitor>
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
          <Show when={setupSubTab() === "midi"}>
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
          <Show when={setupSubTab() === "osc"}>
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
          <Show when={setupSubTab() === "remote"}>
          <RemoteControlPanel
            backendAvailable={isTauriRuntime()}
            invokeCommand={invoke}
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

      <Show when={
        operatorLockMode() === "Full"
          ? "Full"
          : operatorLockMode() === "Partial" && (paneWindow === "setup" || paneWindow === "programmer")
            ? "Partial"
            : null
      }>
        {(mode) => (
          <OperatorLockOverlay
            mode={mode() as OperatorLockMode}
            restrictedPane={mode() === "Partial"}
            blackout={snapshot().blackout}
            videoBlackout={snapshot().video.blackout}
            onSetBlackout={(enabled) => void setBlackout(enabled)}
            onSetAllBlackout={(enabled) => void setAllBlackout(enabled)}
            onUnlock={unlockOperator}
          />
        )}
      </Show>

      <AppStatusLine status={appStatus()} />
    </main>
  );
}
