import { For,mergeProps as mergeThumbnailViewProps,Show } from "solid-js";
import type { AppWorkspaceViewModel, FrontendViewInvoke } from "../App";
import { audioTargetIsEligible,audioTargetIsTrigger } from "../audioControlMapping";
import {
channelFunctionDetail,
channelFunctionLabel,
channelFunctionRangeLabel
} from "../channelFunctionHelpers";
import { formatShowTimecode } from "../clockDisplay";
import { AgentAuthorityPanel } from "./AgentAuthorityPanel";
import { AppStatusLine } from "./AppStatusLine";
import { ControlAssignMenu } from "./ControlAssignMenu";
import { ControlBothPanel } from "./ControlBothPanel";
import { ControlTimelinePlaybackPanel } from "./ControlTimelinePlaybackPanel";
import { DvcImportReportPanel } from "./DvcImportReportPanel";
import { EditableTouchSurface } from "./EditableTouchSurface";
import { InAppDialogHost } from "./InAppDialogHost";
import { LazyPanelBoundary } from "./LazyPanelBoundary";
import { liveVideoMonitorStatusLabel } from "./LiveVideoMonitorPanel";
import { OperatorLockOverlay } from "./OperatorLockOverlay";
import { PatchFixtureFormPanel } from "./PatchFixtureFormPanel";
import {
PatchProfileBrowserPanel
} from "./PatchProfileBrowserPanel";
import { PersistentWorkspaceView } from "./PersistentWorkspaceView";
import { SceneMatrixPanel } from "./SceneMatrixPanel";
import { SetupIoWorkspaceView } from "./SetupIoWorkspaceView";
import { SetupMappingWorkspace } from "./SetupMappingWorkspace";
import { SetupVideoPanel } from "./SetupVideoPanel";
import { StagePreview2D } from "./StagePreview2D";
import { TouchColorPalettePanel } from "./TouchColorPalettePanel";
import { TouchCuePanel } from "./TouchCuePanel";
import { TouchDimmerControlPanel } from "./TouchDimmerControlPanel";
import { TouchFixturePanel } from "./TouchFixturePanel";
import { TouchGenericAttributeGrid } from "./TouchGenericAttributeGrid";
import { TouchPanTiltPad } from "./TouchPanTiltPad";
import { TouchVideoPanel } from "./TouchVideoPanel";
import { VideoControlPanel } from "./VideoControlPanel";
import { WorkspaceChrome } from "./WorkspaceChrome";
import { WorkspaceOperationsMenu } from "./WorkspaceOperationsMenu";
import { WorkspaceSplitHandle } from "./WorkspaceSplitHandle";
import {
isPerformTriggerAction
} from "../controlMappingPerform";
import {
customProfileAttributeTemplates
} from "../customFixtureProfile";
import { cueIdentityCss } from "../identityColor";
import { buildNdiInputHealthRows } from "../ndiInputHealth";
import {
formatDmxPercent,
formatShortDmxPercent
} from "../numericHelpers";
import type {
MidiControlMapping,
OperatorLockMode
} from "../types";
import {
translateUiText
} from "../uiLocalization";
import {
defaultColorPalette
} from "../uiPresets";

// Keep the initial workspace eager; transient editors own their lazy boundary.
export function AppWorkspaceView(props: { model: AppWorkspaceViewModel; invokeCommand: FrontendViewInvoke }) {
  const invoke = props.invokeCommand;
  const {
    paneWindow,
    controlLearnMode,
    controlLearnBusy,
    touchControlDomain,
    selectControlDomain,
    openVideoLiveControls,
    workspaceTab,
    setupSubTab,
    controlMode,
    snapshot,
    blackoutBlockedReason,
    liveAudioInputStatus,
    liveAudioInputTelemetryFresh,
    projectFileLabel,
    visibleProjectDirty,
    currentProjectPath,
    recentProjectPaths,
    projectRecoveryCheckpoint,
    projectBackups,
    daslightProjectImportBusy,
    projectHistoryStatus,
    isfEventPulseBusy,
    projectHistoryNavigationBusy,
    hasPendingHistoryMappings,
    applicationUpdateConfiguration,
    applicationUpdateCheck,
    applicationUpdateProgress,
    applicationUpdateBusy,
    applicationUpdateError,
    uiScale,
    uiLocale,
    uiTheme,
    monitorQuality,
    globalFixtureFlagState,
    nextCue,
    operatorLockMode,
    controlLearnTargetLabel,
    showClockStatus,
    showClockError,
    namedWorkspaces,
    selectedNamedWorkspaceId,
    poppedPanes,
    pendingPaneWindowOperations,
    workspaceApplyBusy,
    operatorPolicy,
    setSelectedNamedWorkspaceId,
    saveNamedWorkspace,
    applyNamedWorkspace,
    deleteNamedWorkspace,
    togglePaneWindow,
    configureOperatorPolicy,
    clearOperatorPolicy,
    setOperatorSessionLock,
    unlockOperator,
    selectWorkspaceTab,
    selectSetupMode,
    selectControlMode,
    triggerPreviousCue,
    triggerNextCue,
    setCueFadePaused,
    pauseTimeline,
    playTimeline,
    setBlackout,
    requestPerformVideoBlackout,
    setAllBlackout,
    clearFixtureFlags,
    setControlLearnMode,
    setLightingMaster,
    setVideoMasterOpacity,
    tapBpm,
    setOperatorBpm,
    openLiveAudioInputSettings,
    newProject,
    saveUserTemplate,
    loadUserTemplate,
    saveProject,
    saveProjectAs,
    loadProject,
    importDaslightProject,
    loadRecentProject,
    clearRecentProjects,
    loadProjectRecovery,
    discardProjectRecovery,
    loadProjectBackup,
    deleteProjectBackup,
    exportDiagnosticPackage,
    checkForApplicationUpdate,
    installAvailableApplicationUpdate,
    setUiLocale,
    setUiTheme,
    setMonitorQuality,
    undoProject,
    redoProject,
    setUiScale,
    resetWorkspaceLayout,
    loadPhase1SampleProject,
    runPhase1Smoke,
    dvcImportReport,
    setDvcImportReport,
    projectDiscardRequest,
    completeProjectDiscardRequest,
    projectDiscardTitle,
    projectDiscardDetail,
    protectedCloseRequest,
    cancelProtectedClose,
    setProtectedCloseRequest,
    protectedCloseCopy,
    completeProtectedClose,
    controlAssignRequest,
    closeControlAssignMenu,
    assignAudioForTargets,
    controlAssignments,
    assignBeatDivisionForTargets,
    removeMidiAssignments,
    pendingPatchGroupRegistration,
    skipPatchedFixtureGroupRegistration,
    patchGroupRegistrationName,
    setPatchGroupRegistrationName,
    registerPatchedFixturesAsGroup,
    projectDropState,
    touchLayoutClass,
    sharedWorkspaceVisible,
    persistentBandVisible,
    activeTopSplitRatio,
    lowerSplitRatio,
    selectedSceneCue,
    liveStatusExpanded,
    controlLiveView,
    projectMappingsAuthority,
    bankAuthority,
    message,
    liveDeskViewActions,
    selectedCueListId,
    setSelectedCueListId,
    setCueListLabel,
    createCueList,
    renameCueList,
    removeCueList,
    createSceneInCueList,
    renameScene,
    duplicateCue,
    removeCue,
    cueRemovalImpact,
    reorderCueLists,
    groupColors,
    setGroupColor,
    sceneMatrixGroupIds,
    selectedSceneCueId,
    setCueLiveModifierLive,
    clearCueLiveModifierLive,
    releaseCueById,
    timelineTrack,
    triggerCue,
    selectSceneCue,
    openOrCreateSuperScene,
    openCueEditor,
    beginTimelineCueDrag,
    moveTimelineCueDrag,
    endTimelineCueDrag,
    renderLightingCuePads,
    renderLightingStatusInspector,
    activeCue,
    enabledDmxOutputCount,
    enabledVideoOutputCount,
    liveVideoPreviewLabel,
    liveVideoMonitors,
    liveVideoProgramLabel,
    selectedVideoClipSlotLayer,
    videoRecordingStatus,
    setVideoBlackout,
    renderTimelineDeskHeaderTools,
    timelineDeskSurface,
    timelineNavigatorOpen,
    setTimelineNavigatorOpen,
    timelinePaneExpanded,
    timelinePaneActionLabel,
    setTimelinePaneExpanded,
    setTimelineUpperHost,
    activeTimeline,
    timelineChildCue,
    seekTimeline,
    registerVideoLiveControlsDialog,
    closeVideoLiveControls,
    setVideoClipSlotInspectorOpen,
    selectedVideoOutputId,
    setVideoLayerState,
    addVideoCuePoint,
    jumpVideoCuePoint,
    setSelectedVideoOutputId,
    videoOutputWindowState,
    selectedVideoClipSlotRuntime,
    mediaAssetThumbnails,
    selectedVideoClipSlotId,
    selectVideoClipSlotLayer,
    selectVideoClipSlot,
    queueVideoClipSlot,
    previewVideoClipSlot,
    openVideoClipSlotInspector,
    importMediaFiles,
    cancelQueuedVideoClipSlot,
    updateVideoClipSlotQuantization,
    videoClipTakeKind,
    videoClipTakeDurationUnit,
    videoClipTakeDurationMilliunits,
    setVideoClipTakeKind,
    setVideoClipTakeDurationUnit,
    setVideoClipTakeDurationMilliunits,
    videoClipSlotTakeEnabled,
    videoClipSlotTakeLabel,
    takeSelectedVideoClipSlot,
    videoEffectCatalog,
    videoTransitionRuntime,
    applyVideoEffectCatalog,
    launchVideoLayerTransitionBus,
    releaseVideoLayerTransitionBus,
    videoClipSlotInspectorOpen,
    renderVideoClipSlotInspector,
    selectedFixtureId,
    selectedFixtureGroupFilter,
    selectedColorControls,
    setTouchSurfaceLayout,
    triggerTouchBinding,
    setTouchBindingValue,
    setTouchBindingColor,
    setTouchBindingXy,
    liveCuePads,
    cuePadRangeLabel,
    cuePadBank,
    cuePadBankCount,
    setCuePadFollowActive,
    setCuePadBank,
    visualizerFixtures,
    visualizerStageObjects2d,
    stageOrigin2d,
    mappingStageFixtures,
    visualizerVideoSurfaces2d,
    mappingShowBeams,
    mappingShowLabels,
    selectFixture,
    selectedFixture,
    touchDimmerQuickActive,
    selectedTouchDimmerTarget,
    startTouchDimmerFlash,
    endTouchDimmerFlash,
    handleTouchDimmerFlashKeyDown,
    handleTouchDimmerFlashKeyUp,
    setFixtureHighlight,
    setFixtureSolo,
    setFixturePark,
    selectedTouchVideoOutput,
    selectFixtureGroupFilter,
    fixtureGroupRows,
    filteredFixtures,
    selectedGroupFlagState,
    selectedGroupSubmaster,
    selectedGroupStrobeFixtureCount,
    controlCategoryRows,
    activeControlCategory,
    activeControlCategoryLabel,
    visibleControls,
    setGroupHighlight,
    setGroupSolo,
    setGroupStrobe,
    setGroupPark,
    setControlCategory,
    showDimmerPanel,
    selectedDimmerControl,
    dimmerSliderRange,
    setDimmerValue,
    setTouchDimmerQuickLevel,
    showPositionPad,
    selectedPositionControls,
    selectedFixtureLimitOverlayStyle,
    setPanTiltFromPointer,
    showColorPad,
    setFixtureColor,
    currentControlValue,
    setControlAttributeValue,
    setupPanelClass,
    registerSetupPanel,
    customWorkbenchOpen,
    isTauriRuntime,
    gdtfShareUser,
    gdtfSharePassword,
    profile,
    selectedMode,
    recentPatchProfiles,
    gdtfPath,
    gdtfShareUrl,
    setGdtfPath,
    setGdtfShareUrl,
    selectGdtfFile,
    importGdtf,
    downloadGdtfFromUrl,
    usePatchVerifiedProfile,
    usePatchBundledProfile,
    usePatchCachedProfile,
    usePatchRecentProfile,
    usePatchProjectProfile,
    setGdtfShareUser,
    setGdtfSharePassword,
    setCustomWorkbenchOpen,
    repairablePatchFixture,
    patchRepairOperationBusy,
    repairSelectedPatchFixture,
    beginPatchProfileDrag,
    clearPatchProfileDrag,
    setMessage,
    universe,
    address,
    patchCount,
    patchAddressStride,
    patchLayoutMode,
    patchGridColumns,
    patchCircleRadius,
    patchX,
    patchY,
    patchZ,
    patchXStep,
    patchZStep,
    patchPitch,
    patchYaw,
    patchRoll,
    selectedFootprint,
    patchCountValue,
    patchAddressStrideValue,
    patchGridColumnsValue,
    patchCircleRadiusValue,
    endAddress,
    patchAddressConflictText,
    patchAddressInvalid,
    nextFreePatchAddress,
    setUniverse,
    setAddress,
    setPatchCount,
    setPatchAddressStride,
    setPatchLayoutMode,
    setPatchGridColumns,
    setPatchCircleRadius,
    setPatchX,
    setPatchY,
    setPatchZ,
    setPatchXStep,
    setPatchZStep,
    setPatchPitch,
    setPatchYaw,
    setPatchRoll,
    patchFixture,
    selectNextFreePatchAddress,
    CustomProfileEditorPanel,
    customManufacturer,
    customProfileName,
    customModeName,
    customAttributes,
    customAttributeDrafts,
    selectedCustomAttributeIndexValue,
    customProfileDraftAnalysis,
    customProfileDmxCells,
    customProfilePreview,
    customProfileDraftRowStatusText,
    setCustomManufacturer,
    setCustomProfileName,
    setCustomModeName,
    setCustomAttributesText,
    appendCustomAttributeTemplate,
    setSelectedCustomAttributeIndex,
    updateCustomAttributeDraft,
    removeCustomAttributeDraft,
    addCustomAttributeDraft,
    moveCustomAttributeDraft,
    commitCustomAttributeDrafts,
    createCustomProfile,
    saveCustomProfile,
    loadCustomProfile,
    activePatchGridUniverse,
    patchGridUniverseOptions,
    dmxPatchViewMode,
    patchGridRevealRequest,
    activePatchGridMap,
    dmxAddressCells,
    dmxUniverseMaps,
    plannedAddressSummary,
    patchProfileDragSource,
    patchProfileDndStatus,
    patchFixtureDragController,
    setPatchGridUniverse,
    setDmxPatchViewMode,
    handleDmxAddressCellClick,
    hoverPatchProfileAddress,
    leavePatchProfileGrid,
    dropPatchProfileAtAddress,
    beginPatchFixtureDrag,
    clearPatchFixtureDrag,
    hoverPatchFixtureAddress,
    leavePatchFixtureGrid,
    dropPatchFixtureAtAddress,
    selectedModeSummary,
    selectedModeDmxCells,
    selectedModeFunctionEntries,
    visibleSelectedModeFunctionEntries,
    selectedModeGeometryRows,
    selectedModeUnresolvedGeometryReferences,
    setSelectedMode,
    selectedFixtureLabelDraft,
    selectedFixtureUniverseDraft,
    selectedFixtureAddressDraft,
    selectedFixtureGroupText,
    fixtureGroupEntities,
    duplicateFixture,
    setSelectedFixtureLabelDraft,
    setSelectedFixtureUniverseDraft,
    setSelectedFixtureAddressDraft,
    setFixturePatch,
    setSelectedFixtureGroupText,
    setFixtureGroups,
    videoCompositionLabel,
    videoCompositionLayerIds,
    videoOutputLabel,
    videoOutputKind,
    videoOutputWidth,
    videoOutputHeight,
    videoOutputFadeMs,
    videoOutputMonitorId,
    videoOutputFullscreen,
    videoOutputEndpoint,
    videoRuntimeStatus,
    videoOutputMappingPresetLabel,
    selectedVideoOutputMappingPresetLabel,
    videoOutputPreviewId,
    videoOutputPreviewInfo,
    videoOutputPreviewUrl,
    setVideoCompositionLabel,
    toggleVideoCompositionLayer,
    addVideoComposition,
    removeVideoComposition,
    setVideoCompositionLayers,
    setVideoCompositionTimelineLayers,
    moveVideoCompositionLayer,
    setVideoOutputLabel,
    setVideoOutputKind,
    setVideoOutputWidth,
    setVideoOutputHeight,
    setVideoOutputFadeMs,
    setVideoOutputMonitorId,
    setVideoOutputFullscreen,
    setVideoOutputEndpoint,
    addDisplayVideoOutput,
    addExternalVideoOutput,
    videoOutputConfigDraft,
    updateVideoOutputConfigDraft,
    setVideoOutputConfig,
    removeVideoOutput,
    enableShowSpoutOutputs,
    resetShowSpoutOutputs,
    setVideoOutputMappingPresetLabel,
    setSelectedVideoOutputMappingPresetLabel,
    saveVideoOutputMappingPreset,
    exportVideoOutputMappingPreset,
    importVideoOutputMappingPreset,
    applyVideoOutputMappingPreset,
    removeVideoOutputMappingPreset,
    setVideoOutputMapping,
    importVideoOutputBitmapMask,
    clearVideoOutputBitmapMask,
    videoOutputWindowActionBusy,
    setDisplayVideoWindowOpen,
    videoOutputRoutingBusy,
    setVideoOutputRouting,
    videoClipRuntime,
    performTopSplitRatio,
    performLowerSplitRatio,
    setPerformTopSplitRatio,
    setPerformLowerSplitRatio,
    renderFxBrowser,
    fxBrowserEntries,
    fxRejections,
    addFxBrowserEntry,
    selectedMediaLibraryAssetId,
    setSelectedMediaLibraryAssetId,
    videoPreviewInfo,
    videoPreviewDiagnosticsText,
    videoPreviewLayerDiagnostics,
    videoPreviewOutputDecodePlans,
    renderDebugVideoPreview,
    refreshVideoPreviewDiagnostics,
    videoPreviewLayerDiagnosticClass,
    videoPreviewLayerDiagnosticLabel,
    videoPreviewOutputDecodePlanClass,
    videoPreviewOutputDecodePlanLabel,
    videoOutputRenderPlanSummary,
    videoOutputRenderPlans,
    videoOutputRenderPlanRows,
    refreshVideoOutputRenderPlans,
    videoRuntimeBackendSummary,
    videoRuntimeBackendClass,
    refreshVideoRuntimeStatus,
    externalVideoIoPlanSummary,
    externalVideoTransportSummary,
    externalVideoIoPlans,
    externalVideoIoPlanRows,
    externalVideoTransportActiveRows,
    externalVideoTransportLiveSourceRows,
    externalVideoTransportLiveSourceFaultRows,
    externalVideoTransportCaptureFaultRows,
    externalVideoTransportRows,
    externalVideoTransportEventRows,
    externalVideoIoPlanClass,
    externalVideoTransportClass,
    refreshExternalVideoIoPlans,
    syncExternalVideoTransports,
    videoOutputWindowSummary,
    refreshVideoOutputWindowStatuses,
    videoOutputRenderPlanState,
    videoOutputWindowStatusForOutput,
    liveVideoOutputHealth,
    vjPreviewTransport,
    vjPreviewTransportBusy,
    vjPreviewTransportError,
    setVjPreviewPlaying,
    seekVjPreview,
    setVjPreviewSpeed,
    clearVjPreview,
    videoSourceKind,
    videoLabel,
    videoPath,
    setVideoSourceKind,
    setVideoLabel,
    setVideoPathFromOperator,
    selectVideoSourceFile,
    addVideoLayer,
    thumbnailController,
    videoAudioMonitorVolume,
    videoAudioMonitorStatus,
    videoProgramAudioEnabled,
    audioOutputDevices,
    selectedAudioOutputDevice,
    videoDeckALayerId,
    videoDeckBLayerId,
    videoAbMix,
    liveAudioInputBackends,
    selectedLiveAudioInputBackend,
    liveAudioInputBackendsKnown,
    liveAudioInputBackendsBusy,
    liveAudioInputBackendError,
    liveAudioInputDevices,
    selectedLiveAudioInputDevice,
    lastKnownLiveAudioInputDeviceIdentity,
    liveAudioInputCapabilities,
    liveAudioInputCapabilitiesBusy,
    liveAudioInputSampleRate,
    liveAudioInputBufferFrames,
    liveAudioInputChannelMix,
    liveAudioInputStatusKnown,
    liveAudioInputBusy,
    liveAudioInputSavedSelectionUiState,
    liveAudioInputPersistError,
    liveAudioInputAsioRevalidationEligible,
    liveAudioInputAsioRevalidationArmed,
    videoPreviewLayerId,
    vjFirstRunAvailable,
    vjFirstRunBusy,
    vjFirstRunError,
    setVideoAudioMonitorVolume,
    setVideoProgramAudioEnabled,
    setSelectedAudioOutputDevice,
    refreshAudioOutputDevices,
    assignVideoDeck,
    applyVideoAbMix,
    launchVideoDeck,
    startVideoOutputRecording,
    stopVideoOutputRecording,
    selectLiveAudioInputBackend,
    selectLiveAudioInputDevice,
    selectLiveAudioInputSampleRate,
    selectLiveAudioInputBufferFrames,
    selectLiveAudioInputChannelMix,
    refreshLiveAudioInputOperatorState,
    armLiveAudioInputAsioRevalidation,
    startLiveAudioInput,
    stopLiveAudioInput,
    stageVjPreviewLayer,
    createFirstRunVjShow,
    launchVideoClipFromGrid,
    takeVideoClipFromGrid,
    stopVideoClipFromGrid,
    playVideoLayerAudioMonitor,
    stopVideoLayerAudioMonitor,
    authorizeVideoThumbnailAccess,
    externalVideoTransportStatus,
    mediaAssetAvailabilityById,
    activeMediaAssetOperations,
    lastMediaAssetImportReport,
    mediaAssetCheckedAtById,
    inspectMediaAssetIds,
    relinkMediaLibraryAsset,
    scanMediaLibraryRelinkFolder,
    applyMediaLibraryRelinkRequests,
    beginMediaAssetPreview,
    loadMediaAssetPreviewFrame,
    endMediaAssetPreview,
    cancelMediaAssetOperation,
    emptyAutoVjSnapshot,
    autoVjBusy,
    setAutoVjConfig,
    setAutoVjArmed,
    setAutoVjHold,
    setNodeGraphEnabled,
    videoLayerListProps,
    loadVideoGroupCanvasPreview,
    importVideoEffectScopeIsf,
    applyVideoEffectPreset,
    removeVideoEffectChain,
    selectedVideoAutomationLayerId,
    videoAutomationParam,
    videoAutomationInterpolation,
    videoAutomationStartMs,
    videoAutomationEndMs,
    videoAutomationStartValue,
    videoAutomationEndValue,
    timelineVideoAutomationRows,
    selectedVideoTimelineAutomationId,
    videoAutomationRowScope,
    allTimelineVideoAutomationRows,
    timelineVideoAutomationDraft,
    setVideoAutomationLayerId,
    setVideoAutomationParam,
    setVideoAutomationInterpolation,
    setVideoAutomationStartMs,
    setVideoAutomationEndMs,
    setVideoAutomationStartValue,
    setVideoAutomationEndValue,
    setVideoAutomationRowScope,
    usePlayheadForVideoAutomation,
    addTimelineVideoAutomation,
    updateTimelineVideoAutomationDraft,
    alignVideoAutomationDraftToPlayhead,
    addVideoAutomationKeyframeAtPlayhead,
    removeVideoAutomationKeyframeAtPlayhead,
    removeVideoAutomationKeyframe,
    setVideoAutomationKeyframeInterpolation,
    setVideoAutomationKeyframeValue,
    setTimelineAutomationEnabled,
    setVideoAutomationRowsEnabled,
    setTimelineVideoAutomation,
    removeTimelineAutomation,
    activeTopSplitDefaultRatio,
    commitActiveTopSplitRatio,
    persistentWorkspaceModel,
    setupIoModel,
    targetBlackout,
    appStatus,
    fixtureGroupDeleteUndoAvailable,
    undoDeleteFixtureGroup
  } = props.model;
  return (
    <main
      class={`app${paneWindow ? ` paneWindow paneWindow-${paneWindow}` : ""}`}
      style={paneWindow ? "grid-template-rows: minmax(0, 1fr) auto !important" : undefined}
      data-pane-window-mode={paneWindow || "main"}
      data-control-learn-mode={controlLearnMode() ?? undefined}
      data-control-learn-busy={controlLearnBusy() ? "true" : undefined}
    >
      <WorkspaceChrome
        touchControlDomain={touchControlDomain()}
        onControlDomain={selectControlDomain}
        onOpenVideoLiveControls={openVideoLiveControls}
        workspaceTab={workspaceTab()}
        setupSubTab={setupSubTab()}
        controlMode={controlMode()}
        blackout={snapshot().authored_blackout || snapshot().safety_blackout_engaged}
        videoBlackout={snapshot().video.blackout}
        blackoutBlockedReason={blackoutBlockedReason()}
        lightingMaster={snapshot().lighting_master}
        videoMaster={snapshot().video.master_opacity}
        bpm={snapshot().clock.bpm}
        clock={snapshot().clock}
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
          can_undo: !isfEventPulseBusy() && !projectHistoryNavigationBusy()
            && (projectHistoryStatus().can_undo || hasPendingHistoryMappings()),
          can_redo: !isfEventPulseBusy() && !projectHistoryNavigationBusy()
            && projectHistoryStatus().can_redo && !hasPendingHistoryMappings(),
        }}
        applicationUpdateConfiguration={applicationUpdateConfiguration()}
        applicationUpdateCheck={applicationUpdateCheck()}
        applicationUpdateProgress={applicationUpdateProgress()}
        applicationUpdateBusy={applicationUpdateBusy()}
        applicationUpdateError={applicationUpdateError()}
        uiScale={uiScale()}
        uiLocale={uiLocale()}
        uiTheme={uiTheme()}
        monitorQuality={monitorQuality()}
        canBack={snapshot().cues.length > 0}
        canGo={snapshot().cues.length > 0}
        canPauseFade={Boolean(snapshot().active_fade)}
        fadePaused={Boolean(snapshot().active_fade?.paused)}
        canToggleTimeline={snapshot().timeline.playing || snapshot().timeline.duration_ms > 0}
        timelinePlaying={snapshot().timeline.playing}
        anyFixtureFlags={globalFixtureFlagState().anyFlagged}
        nextCueLabel={nextCue()?.label ?? "No cue"}
        operatorLockMode={operatorLockMode()}
        controlLearnMode={controlLearnMode()}
        controlLearnBusy={controlLearnBusy()}
        controlLearnTargetLabel={controlLearnTargetLabel()}
        showClockStatus={showClockStatus()}
        showClockError={showClockError()}
        operations={
          // Pane children run main-only open/close/capture commands that
          // reject them, so they must never render the operations menu or its
          // popout toggles at all.
          paneWindow ? null : (
            <WorkspaceOperationsMenu
              profiles={namedWorkspaces()}
              selectedProfileId={selectedNamedWorkspaceId()}
              poppedPanes={poppedPanes()}
              paneTransitions={pendingPaneWindowOperations()}
              workspaceBusy={workspaceApplyBusy()}
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
          )
        }
        onWorkspaceTab={selectWorkspaceTab}
        onSetupSubTab={selectSetupMode}
        onControlMode={selectControlMode}
        onBack={() => void triggerPreviousCue()}
        onGo={() => void triggerNextCue()}
        onToggleFade={() => void setCueFadePaused(!snapshot().active_fade?.paused)}
        onToggleTimeline={() => void (snapshot().timeline.playing ? pauseTimeline() : playTimeline())}
        onSetBlackout={(enabled) => void setBlackout(enabled)}
        onSetVideoBlackout={(enabled) => void requestPerformVideoBlackout(enabled)}
        onSetAllBlackout={(enabled) => void setAllBlackout(enabled)}
        onClearFixtureFlags={() => void clearFixtureFlags("all")}
        onControlLearnMode={setControlLearnMode}
        onLightingMaster={setLightingMaster}
        onVideoMaster={setVideoMasterOpacity}
        onTapBpm={tapBpm}
        onSetBpm={setOperatorBpm}
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
        onUiTheme={setUiTheme}
        onMonitorQuality={setMonitorQuality}
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
      <Show when={projectDiscardRequest()}>
        <dialog
          ref={(dialog) => queueMicrotask(() => {
            if (!dialog.open) dialog.showModal();
            dialog.querySelector<HTMLButtonElement>("[data-project-discard-cancel]")?.focus();
          })}
          class="protectedCloseDialog"
          data-project-discard-dialog
          role="alertdialog"
          aria-labelledby="project-discard-title"
          aria-describedby="project-discard-detail"
          onCancel={(event) => {
            event.preventDefault();
            completeProjectDiscardRequest(false);
          }}
        >
          <section class="protectedCloseFrame">
            <header>
              <span>{translateUiText("UNSAVED CHANGES", uiLocale())}</span>
              <strong>{translateUiText("SYNDOCAL", uiLocale())}</strong>
            </header>
            <div class="protectedCloseBody">
              <div class="protectedCloseMark" aria-hidden="true">!</div>
              <div>
                <h2 id="project-discard-title" class="textBalance">
                  {projectDiscardTitle()}
                </h2>
                <p id="project-discard-detail" class="textPretty">{projectDiscardDetail()}</p>
              </div>
            </div>
            <div class="protectedCloseActions">
              <button
                type="button"
                data-project-discard-cancel
                onClick={() => completeProjectDiscardRequest(false)}
              >
                {translateUiText("Keep Editing", uiLocale())}
              </button>
              <button
                type="button"
                class="danger"
                data-project-discard-confirm
                onClick={() => completeProjectDiscardRequest(true)}
              >
                {translateUiText("Discard and Continue", uiLocale())}
              </button>
            </div>
          </section>
        </dialog>
      </Show>
      <Show when={protectedCloseRequest()}>
        <dialog
          ref={(dialog) => queueMicrotask(() => {
            if (!dialog.open) dialog.showModal();
            dialog.querySelector<HTMLButtonElement>("[data-protected-close-cancel]")?.focus();
          })}
          class="protectedCloseDialog"
          data-protected-close-dialog
          role="alertdialog"
          aria-labelledby="protected-close-title"
          aria-describedby="protected-close-detail"
          onCancel={(event) => {
            event.preventDefault();
            cancelProtectedClose();
          }}
          onClose={() => setProtectedCloseRequest(null)}
        >
          <section class="protectedCloseFrame">
            <header>
              <span>{protectedCloseCopy().eyebrow}</span>
              <strong>{translateUiText("SYNDOCAL", uiLocale())}</strong>
            </header>
            <div class="protectedCloseBody">
              <div class="protectedCloseMark" aria-hidden="true">!</div>
              <div>
                <h2 id="protected-close-title" class="textBalance">{protectedCloseCopy().title}</h2>
                <p id="protected-close-detail" class="textPretty">{protectedCloseCopy().detail}</p>
              </div>
            </div>
            <div class="protectedCloseActions">
              <button
                type="button"
                data-protected-close-cancel
                onClick={cancelProtectedClose}
              >
                {protectedCloseCopy().cancel}
              </button>
              <button
                type="button"
                class="danger"
                data-protected-close-confirm
                onClick={() => void completeProtectedClose()}
              >
                {protectedCloseCopy().confirm}
              </button>
            </div>
          </section>
        </dialog>
      </Show>
      <InAppDialogHost locale={uiLocale} />
      <Show when={controlAssignRequest()}>
        {(request) => (
          <ControlAssignMenu
            x={request().x}
            y={request().y}
            label={request().targets[0]?.label ?? ""}
            beatEligible={request().targets.length > 0
              && request().targets.every((target) => isPerformTriggerAction(target.midi_action ?? target.action))}
            audioEligible={request().targets.length > 0 && request().targets.every(audioTargetIsEligible)}
            audioTrigger={request().targets.every(audioTargetIsTrigger)}
            audioLow={request().targets[0]?.low}
            audioHigh={request().targets[0]?.high}
            onAudio={(settings) => {
              const targets = request().targets;
              closeControlAssignMenu();
              void assignAudioForTargets(targets, settings);
            }}
            assignments={controlAssignments()}
            onLearn={() => { const learn = request().learn; closeControlAssignMenu(); learn(); }}
            onBeat={(division) => {
              const targets = request().targets;
              closeControlAssignMenu();
              void assignBeatDivisionForTargets(targets, division);
            }}
            onRemove={(assignment) => {
              const label = request().targets[0]?.label ?? "";
              closeControlAssignMenu();
              void removeMidiAssignments([assignment.mapping as MidiControlMapping], label);
            }}
            onDismiss={(restoreFocus) => closeControlAssignMenu(restoreFocus)}
          />
        )}
      </Show>
      <Show when={pendingPatchGroupRegistration()}>
        {(pending) => (
          <dialog
            ref={(dialog) => queueMicrotask(() => {
              if (!dialog.open) dialog.showModal();
              dialog.querySelector<HTMLInputElement>("input")?.focus();
            })}
            class="patchGroupRegistrationDialog"
            data-patch-group-registration-dialog
            data-fixed-frame-modal="patch-group-registration"
            aria-labelledby="patch-group-registration-title"
            onCancel={(event) => {
              event.preventDefault();
              skipPatchedFixtureGroupRegistration();
            }}
          >
            <div class="patchGroupRegistrationFrame">
              <header>
                <span>Patch complete</span>
                <strong>{pending().fixtureIds.length} fixtures</strong>
              </header>
              <h3 id="patch-group-registration-title">Register new fixtures as a group?</h3>
              <label>
                Group name
                <input
                  data-patch-group-registration-name
                  value={patchGroupRegistrationName()}
                  placeholder={pending().defaultName}
                  onInput={(event) => setPatchGroupRegistrationName(event.currentTarget.value)}
                  onKeyDown={(event) => {
                    if (event.key === "Enter") {
                      event.preventDefault();
                      void registerPatchedFixturesAsGroup();
                    }
                  }}
                />
              </label>
              <div class="dialogActions">
                <button
                  type="button"
                  data-patch-group-registration-skip
                  onClick={skipPatchedFixtureGroupRegistration}
                >
                  Skip
                </button>
                <button
                  type="button"
                  class="primary"
                  data-patch-group-registration-register
                  disabled={!patchGroupRegistrationName().trim()}
                  onClick={() => void registerPatchedFixturesAsGroup()}
                >
                  Register group
                </button>
              </div>
            </div>
          </dialog>
        )}
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
          paneWindow || (sharedWorkspaceVisible() && !persistentBandVisible())
            ? "grid-template-rows: minmax(0, 1fr) !important"
            : undefined
        }
        data-workspace-split-root={persistentBandVisible() ? "true" : undefined}
        data-upper-lower-ratio={persistentBandVisible() ? activeTopSplitRatio() : undefined}
        data-lower-left-right-ratio={persistentBandVisible() ? lowerSplitRatio() : undefined}
      >
        {/* Keep every persistent domain tab associated with a real panel while
            inactive panels stay unmounted: these placeholders are hidden,
            hold no application state, and disappear as their domain mounts. */}
        <Show when={workspaceTab() === "control" && controlMode() !== "edit"}>
          <section id="edit-domain-panel-edit" role="tabpanel" aria-labelledby="edit-domain-tab-edit" hidden />
        </Show>
        <Show when={workspaceTab() === "control" && controlMode() !== "mixer"}>
          <section id="edit-domain-panel-mixer" role="tabpanel" aria-labelledby="edit-domain-tab-mixer" hidden />
        </Show>
        <Show when={workspaceTab() === "control" && controlMode() !== "live"}>
          <section id="edit-domain-panel-live" role="tabpanel" aria-labelledby="edit-domain-tab-live" hidden />
        </Show>
        <Show when={workspaceTab() === "touch"}>
          <Show when={controlMode() !== "edit"}>
            <section id="control-domain-panel-lighting" role="tabpanel" aria-labelledby="control-domain-lighting" hidden />
          </Show>
          <Show when={controlMode() !== "perform"}>
            <section id="control-domain-panel-video" role="tabpanel" aria-labelledby="control-domain-video" hidden />
          </Show>
          <Show when={controlMode() !== "both"}>
            <section id="control-domain-panel-both" role="tabpanel" aria-labelledby="control-domain-both" hidden />
          </Show>
          <Show when={touchControlDomain() !== "timeline"}>
            <section id="control-domain-panel-timeline" role="tabpanel" aria-labelledby="control-domain-timeline" hidden />
          </Show>
        </Show>
        <Show when={workspaceTab() === "control" && controlMode() === "edit"}>
          <section
            id="edit-domain-panel-edit"
            role="tabpanel"
            aria-labelledby="edit-domain-tab-edit"
            class={`panel editDomainUpperPanel controlPanel${selectedSceneCue() ? " lightingSceneSettingsVisible" : liveStatusExpanded() ? " lightingStatusVisible" : ""}`}
            data-workspace-pane="upper"
            data-live-status-expanded={liveStatusExpanded() ? "true" : "false"}
            aria-label="Lighting Banks and Scenes"
          >
            <Show when={controlLiveView() === "matrix"}>
              <SceneMatrixPanel
              projectEpoch={projectMappingsAuthority().project_epoch}
              bankAuthority={bankAuthority()}
              bankMutationFailureMessage={message()}
              toolbar={liveDeskViewActions()}
              selectedCueListId={selectedCueListId()}
              onSelectCueList={setSelectedCueListId}
              onCueListLabel={setCueListLabel}
              onCreateCueList={createCueList}
              onRenameCueList={renameCueList}
              onRemoveCueList={removeCueList}
              onCreateSceneForCueList={createSceneInCueList}
              onRenameCue={renameScene}
              onDuplicateCue={duplicateCue}
              onRemoveCue={removeCue}
              cueRemovalImpact={cueRemovalImpact}
              onReorderCueLists={reorderCueLists}
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
              onOpenCueEditor={() => openCueEditor()}
              onBeginTimelineCueDrag={beginTimelineCueDrag}
              onMoveTimelineCueDrag={moveTimelineCueDrag}
              onEndTimelineCueDrag={(point, moved, canceled) => void endTimelineCueDrag(point, moved, canceled)}
              />
            </Show>
            <Show when={controlLiveView() === "pads"}>
              {renderLightingCuePads()}
            </Show>
            <Show when={Boolean(selectedSceneCue()) || liveStatusExpanded()}>
              {renderLightingStatusInspector()}
            </Show>
          </section>
        </Show>
        <Show when={workspaceTab() === "touch" && controlMode() === "both"}>
          <ControlBothPanel
            activeCueId={snapshot().active_cue_id ?? null}
            activeCueLabel={activeCue()?.label ?? null}
            nextCueLabel={nextCue()?.label ?? null}
            cueCount={snapshot().cues.length}
            timelinePlaying={snapshot().timeline.playing}
            timecode={formatShowTimecode(snapshot().timeline.position_ms)}
            lightingMaster={snapshot().lighting_master}
            videoMaster={snapshot().video.master_opacity}
            blackout={snapshot().authored_blackout}
            videoBlackout={snapshot().video.blackout}
            enabledDmxOutputCount={enabledDmxOutputCount()}
            dmxOutputCount={snapshot().dmx_outputs.length}
            enabledVideoOutputCount={enabledVideoOutputCount()}
            videoOutputCount={snapshot().video.outputs.length}
            previewLabel={liveVideoPreviewLabel()}
            previewStatus={liveVideoMonitorStatusLabel("preview", liveVideoMonitors.preview().status)}
            programLabel={liveVideoProgramLabel()}
            programStatus={liveVideoMonitorStatusLabel("program", liveVideoMonitors.program().status)}
            clipCount={snapshot().video.layers.length}
            selectedClipLabel={selectedVideoClipSlotLayer()?.label ?? null}
            recordingLabel={videoRecordingStatus().active ? "Recording" : videoRecordingStatus().state ?? "Idle"}
            onOpenLighting={() => selectControlDomain("lighting")}
            onOpenVideo={() => selectControlDomain("video")}
            onBack={() => void triggerPreviousCue()}
            onGo={() => void triggerNextCue()}
            onRelease={() => {
              const cueId = snapshot().active_cue_id;
              if (cueId !== null && cueId !== undefined) void releaseCueById(cueId);
            }}
            onSetLightingMaster={setLightingMaster}
            onSetVideoMaster={setVideoMasterOpacity}
            onSetBlackout={(enabled) => void setBlackout(enabled)}
            onSetVideoBlackout={(enabled) => void setVideoBlackout(enabled)}
            onSetAllBlackout={(enabled) => void setAllBlackout(enabled)}
          />
        </Show>
        <Show when={workspaceTab() === "control" && controlMode() === "live"}>
        <section
          class={`panel liveControlPanel controlPanel${liveStatusExpanded() ? " liveStatusExpanded" : ""}`}
          id="edit-domain-panel-live"
          role="tabpanel"
          aria-labelledby="edit-domain-tab-live"
          data-workspace-pane="upper"
          data-live-status-expanded={liveStatusExpanded() ? "true" : "false"}
        >
          <header class="panelHeader controlContextHeader timelineArrangerHeader" data-timeline-arranger-header>
            <h2>Timeline</h2>
            <div class="controlContextHeaderTools">
              {renderTimelineDeskHeaderTools()}
              <Show when={timelineDeskSurface() === "show"}>
                <button type="button" class={timelineNavigatorOpen() ? "active" : ""}
                  data-timeline-navigator-toggle title="タイムライン一覧" aria-label="タイムライン一覧"
                  aria-expanded={timelineNavigatorOpen()} aria-controls="timeline-navigator"
                  onClick={() => setTimelineNavigatorOpen((open) => !open)}>
                  タイムライン一覧
                </button>
              </Show>
              <Show when={!paneWindow}>
                <button
                  type="button"
                  class={`timelinePaneExpandToggle${timelinePaneExpanded() ? " expanded" : ""}`}
                  data-timeline-pane-expand-toggle
                  title={timelinePaneActionLabel()}
                  aria-label={timelinePaneActionLabel()}
                  aria-expanded={timelinePaneExpanded()}
                  onClick={() => setTimelinePaneExpanded((expanded) => !expanded)}
                >
                  <svg viewBox="0 0 16 16" aria-hidden="true">
                    <path d={timelinePaneExpanded() ? "M3 6h3V3M13 6h-3V3M13 10h-3v3M3 10h3v3" : "M6 3H3v3M10 3h3v3M13 10v3h-3M3 10v3h3"} />
                  </svg>
                </button>
              </Show>
            </div>
          </header>
          <div
            ref={(host) => {
              setTimelineUpperHost(undefined);
              queueMicrotask(() => {
                if (host.isConnected) setTimelineUpperHost(host);
              });
            }}
            class="timelineArrangerUpperHost controlContextPane"
            data-timeline-arranger-upper
            aria-label="Timeline arranger"
          />
        </section>
        </Show>
        <Show when={workspaceTab() === "touch" && touchControlDomain() === "timeline"}>
          <ControlTimelinePlaybackPanel
            cues={snapshot().cues}
            cueLists={snapshot().cue_lists}
            activeCueId={snapshot().active_cue_id ?? null}
            nextCueId={nextCue()?.id ?? null}
            activeFade={snapshot().active_fade ?? null}
            timelineEvents={activeTimeline().events}
            timelinePositionMs={activeTimeline().position_ms}
            timelineDurationMs={activeTimeline().duration_ms}
            timelinePlaying={activeTimeline().playing}
            childTimelineLabel={timelineChildCue()?.label ?? null}
            blackout={snapshot().authored_blackout || snapshot().safety_blackout_engaged}
            videoBlackout={snapshot().video.blackout}
            safetyBlackoutEngaged={snapshot().safety_blackout_engaged}
            blackoutBlockedReason={blackoutBlockedReason()}
            cueIdentity={(cue) => cueIdentityCss(
              cue.id,
              cue.color,
              "band",
              cue.group_id,
              cue.group_id ? groupColors()[cue.group_id] : null,
            )}
            onGo={() => void triggerNextCue()}
            onBack={() => void triggerPreviousCue()}
            onRelease={() => {
              const cueId = snapshot().active_cue_id;
              if (cueId !== null && cueId !== undefined) void releaseCueById(cueId);
            }}
            onTriggerCue={(cueId) => void triggerCue(cueId)}
            onToggleFadePause={() => void setCueFadePaused(!snapshot().active_fade?.paused)}
            onPlay={() => void playTimeline()}
            onPause={() => void pauseTimeline()}
            onSeek={(positionMs) => void seekTimeline(positionMs)}
            onSetBlackout={(enabled) => void setBlackout(enabled)}
            onSetVideoBlackout={(enabled) => void requestPerformVideoBlackout(enabled)}
            onSetAllBlackout={(enabled) => void setAllBlackout(enabled)}
            onOpenTimelineEditor={paneWindow ? undefined : () => selectControlMode("live")}
          />
        </Show>
        <Show when={workspaceTab() === "touch" && touchControlDomain() === "video"}>
          <dialog class="controlVideoToolsDialog" ref={registerVideoLiveControlsDialog} aria-labelledby="control-video-tools-title"
            onClose={() => setVideoClipSlotInspectorOpen(false)}>
            <header class="panelHeader">
              <h2 id="control-video-tools-title">Video live controls</h2>
              <button type="button" autofocus onClick={() => {
                // Do not wait for the asynchronous dialog close event to reap the nested inspector.
                setVideoClipSlotInspectorOpen(false);
                closeVideoLiveControls();
              }}>Close</button>
            </header>
        <TouchVideoPanel
          layers={snapshot().video.layers}
          outputs={snapshot().video.outputs}
          selectedOutputId={selectedVideoOutputId()}
          onSetLayerState={setVideoLayerState}
          onAddCuePoint={addVideoCuePoint}
          onJumpCuePoint={jumpVideoCuePoint}
          onSelectOutput={setSelectedVideoOutputId}
          windowStateForOutput={(outputId) => {
            const state = videoOutputWindowState(outputId);
            return { stateLabel: state.stateLabel, detail: state.detail };
          }}
          clipSlotBank={{
            mode: "control",
            get layers() { return snapshot().video.layers; },
            get layer() { return selectedVideoClipSlotLayer(); },
            get runtime() { return selectedVideoClipSlotRuntime(); },
            get assets() { return snapshot().video.media_assets; },
            get thumbnails() { return mediaAssetThumbnails(); },
            get selectedSlotId() { return selectedVideoClipSlotId(); },
            onSelectLayer: selectVideoClipSlotLayer,
            onSelect: selectVideoClipSlot,
            onQueue: (slotId) => {
              const layer = selectedVideoClipSlotLayer();
              return layer ? queueVideoClipSlot(layer.id, slotId) : Promise.resolve(null);
            },
            onPreview: previewVideoClipSlot,
            onMore: openVideoClipSlotInspector,
            onImport: () => importMediaFiles("File"),
            onCancelQueue: () => {
              const layer = selectedVideoClipSlotLayer();
              return layer ? cancelQueuedVideoClipSlot(layer.id) : Promise.resolve(null);
            },
            onSetQuantization: updateVideoClipSlotQuantization,
            get transitionKind() { return videoClipTakeKind(); },
            get transitionDurationUnit() { return videoClipTakeDurationUnit(); },
            get transitionDurationMilliunits() { return videoClipTakeDurationMilliunits(); },
            onSetTransitionKind: setVideoClipTakeKind,
            onSetTransitionDurationUnit: setVideoClipTakeDurationUnit,
            onSetTransitionDurationMilliunits: (value) => setVideoClipTakeDurationMilliunits(
              Math.max(1, Math.min(600_000, Math.round(value || 1))),
            ),
          }}
          clipSlotTake={{
            get enabled() { return videoClipSlotTakeEnabled(); },
            get label() { return videoClipSlotTakeLabel(); },
            onTake: takeSelectedVideoClipSlot,
          }}
          transitionBuses={{
            get catalog() { return videoEffectCatalog(); },
            get compositions() { return snapshot().video.compositions; },
            get runtime() { return videoTransitionRuntime(); },
            get busy() { return isfEventPulseBusy(); },
            onApplyCatalog: applyVideoEffectCatalog,
            onLaunch: launchVideoLayerTransitionBus,
            onRelease: releaseVideoLayerTransitionBus,
          }}
        />
        <Show when={videoClipSlotInspectorOpen() && selectedVideoClipSlotLayer()}>
          {renderVideoClipSlotInspector(false)}
        </Show>
          </dialog>
        </Show>
        <Show when={workspaceTab() === "touch" && touchControlDomain() === "lighting"}>
        <EditableTouchSurface
          bankAuthority={bankAuthority()}
          snapshot={snapshot()}
          surface={snapshot().touch_surface}
          selectedFixtureId={selectedFixtureId()}
          selectedGroupId={selectedFixtureGroupFilter()}
          colorPalette={selectedColorControls() ? defaultColorPalette.slice(0, 8) : []}
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
          bankAuthority={bankAuthority()}
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
                  <small role="status">
                    Physical window: {videoOutputWindowState(output().id).stateLabel}
                  </small>
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
                  <span data-no-localize>{group.label}</span>
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
        </Show>
        <Show when={workspaceTab() === "setup" && setupSubTab() === "patch"}>
        <aside
          class={setupPanelClass("panel setup setupPanel", ["patch"])}
          ref={registerSetupPanel(["patch"])}
          tabIndex={-1}
        >
          <Show when={!customWorkbenchOpen()}>
            <PatchProfileBrowserPanel
              backendAvailable={isTauriRuntime()}
              invokeCommand={invoke}
              shareUser={gdtfShareUser()}
              sharePassword={gdtfSharePassword()}
              selectedProfile={profile()}
              selectedMode={selectedMode()}
              recentProfiles={recentPatchProfiles()}
              projectFixtures={snapshot().fixtures}
              gdtfPath={gdtfPath()}
              gdtfShareUrl={gdtfShareUrl()}
              onGdtfPath={setGdtfPath}
              onGdtfShareUrl={setGdtfShareUrl}
              onBrowse={selectGdtfFile}
              onLoadGdtf={importGdtf}
              onDownloadGdtf={downloadGdtfFromUrl}
              onLoadVerified={usePatchVerifiedProfile}
              onLoadBundled={usePatchBundledProfile}
              onLoadCached={usePatchCachedProfile}
              onLoadRecent={usePatchRecentProfile}
              onLoadProject={usePatchProjectProfile}
              onShareUser={setGdtfShareUser}
              onSharePassword={setGdtfSharePassword}
              onOpenWorkbench={() => setCustomWorkbenchOpen(true)}
              repairableFixtureLabel={repairablePatchFixture()?.label ?? null}
              operationBusy={patchRepairOperationBusy()}
              onRepairSelectedFixture={() => void repairSelectedPatchFixture()}
              onProfileDragStart={beginPatchProfileDrag}
              onProfileDragEnd={clearPatchProfileDrag}
              onMessage={setMessage}
            />
            <PatchFixtureFormPanel
              armed={Boolean(profile())}
              operationBusy={patchRepairOperationBusy()}
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
              warnings={profile()?.warnings ?? []}
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
          </Show>
          <Show when={customWorkbenchOpen()}>
          <div class="customWorkbenchBar" data-custom-workbench-bar>
            <button
              type="button"
              class="customWorkbenchBack"
              onClick={() => setCustomWorkbenchOpen(false)}
              data-custom-workbench-back
            >
              Back to Patch
            </button>
            <span>Custom profile workbench</span>
          </div>
          <LazyPanelBoundary label="Custom profile workbench">
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
          </LazyPanelBoundary>
          </Show>
        </aside>
        </Show>

        <Show when={workspaceTab() === "setup" && setupSubTab() === "patch"}>
        <SetupMappingWorkspace
          className={setupPanelClass("panel fixtures setupPanel", ["patch"])}
          panelRef={registerSetupPanel(["patch"])}
          patchMap={{
            activeUniverse: activePatchGridUniverse(),
            universeOptions: patchGridUniverseOptions(),
            viewMode: dmxPatchViewMode(),
            revealAddressRequest: patchGridRevealRequest(),
            activeMap: activePatchGridMap(),
            addressCells: dmxAddressCells(),
            universeMaps: dmxUniverseMaps(),
            fixtureCount: snapshot().fixtures.length,
            selectedFixtureId: selectedFixtureId(),
            plannedAddressSummary: plannedAddressSummary(),
            profileDragActive: Boolean(patchProfileDragSource()),
            profileDndStatus: patchProfileDndStatus(),
            fixtureDragActive: Boolean(patchFixtureDragController.source()),
            fixtureDndStatus: patchFixtureDragController.dndStatus(),
            onUniverse: setPatchGridUniverse,
            onViewMode: setDmxPatchViewMode,
            onSelectFixture: selectFixture,
            onAddressCell: handleDmxAddressCellClick,
            onProfileDragHover: hoverPatchProfileAddress,
            onProfileDragLeave: leavePatchProfileGrid,
            onProfileDrop: dropPatchProfileAtAddress,
            onFixtureDragStart: beginPatchFixtureDrag,
            onFixtureDragEnd: clearPatchFixtureDrag,
            onFixtureDragHover: hoverPatchFixtureAddress,
            onFixtureDragLeave: leavePatchFixtureGrid,
            onFixtureDrop: dropPatchFixtureAtAddress,
          }}
          profileSummary={profile() ? {
            profile: profile()!,
            selectedMode: selectedMode(),
            selectedModeSummary: selectedModeSummary(),
            selectedFootprint: selectedFootprint(),
            dmxCells: selectedModeDmxCells(),
            functionEntries: selectedModeFunctionEntries(),
            visibleFunctionEntries: visibleSelectedModeFunctionEntries(),
            geometryRows: selectedModeGeometryRows(),
            unresolvedGeometryReferences: selectedModeUnresolvedGeometryReferences(),
            functionLabel: channelFunctionLabel,
            functionRangeLabel: channelFunctionRangeLabel,
            functionDetail: channelFunctionDetail,
            onSelectedMode: setSelectedMode,
          } : null}
          fixtureEditor={selectedFixture() ? {
            fixture: selectedFixture()!,
            labelDraft: selectedFixtureLabelDraft(),
            universeDraft: selectedFixtureUniverseDraft(),
            addressDraft: selectedFixtureAddressDraft(),
            groupText: selectedFixtureGroupText(),
            availableGroups: fixtureGroupEntities(),
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
          timelineId={snapshot().timeline.id ?? 0}
          timelineLayers={snapshot().timeline.layers ?? []}
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
          spoutAvailable={videoRuntimeStatus()?.backends.some((backend) => backend.id === "spout" && backend.state === "Available") ?? false}
          ndiAvailable={videoRuntimeStatus()?.backends.some((backend) => backend.id === "ndi" && backend.state === "Available") ?? false}
          syphonAvailable={videoRuntimeStatus()?.backends.some(
            (backend) => backend.id === "syphon" && backend.state === "Available",
          ) ?? false}
          mappingPresetLabel={videoOutputMappingPresetLabel()}
          selectedMappingPresetLabel={selectedVideoOutputMappingPresetLabel()}
          selectedOutputId={selectedVideoOutputId()}
          previewOutputId={videoOutputPreviewId()}
          previewInfo={videoOutputPreviewInfo()}
          previewUrl={videoOutputPreviewUrl()}
          onCompositionLabel={setVideoCompositionLabel}
          onToggleCompositionLayer={toggleVideoCompositionLayer}
          onAddComposition={addVideoComposition}
          onRemoveComposition={removeVideoComposition}
          onSetCompositionLayers={setVideoCompositionLayers}
          onSetCompositionTimelineLayers={setVideoCompositionTimelineLayers}
          onMoveCompositionLayer={moveVideoCompositionLayer}
          onOutputLabel={setVideoOutputLabel}
          onOutputKind={setVideoOutputKind}
          onOutputWidth={setVideoOutputWidth}
          onOutputHeight={setVideoOutputHeight}
          onOutputFadeMs={setVideoOutputFadeMs}
          onOutputMonitorId={setVideoOutputMonitorId}
          onOutputFullscreen={setVideoOutputFullscreen}
          onOutputEndpoint={setVideoOutputEndpoint}
          invokeCommand={invoke}
          onAddDisplayOutput={addDisplayVideoOutput}
          onAddExternalOutput={addExternalVideoOutput}
          outputConfigDraft={videoOutputConfigDraft}
          onOutputConfigDraft={updateVideoOutputConfigDraft}
          onApplyOutputConfig={setVideoOutputConfig}
          onRemoveOutput={removeVideoOutput}
          onEnableShowSpoutOutputs={enableShowSpoutOutputs}
          onResetShowSpoutOutputs={resetShowSpoutOutputs}
          onSelectOutput={setSelectedVideoOutputId}
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
          windowStateForOutput={videoOutputWindowState}
          windowActionBusyForOutput={(outputId) => Boolean(videoOutputWindowActionBusy()[String(outputId)])}
          onToggleWindow={setDisplayVideoWindowOpen}
          routingBusyForOutput={(outputId) => Boolean(videoOutputRoutingBusy()[String(outputId)])}
          onAssignComposition={setVideoOutputRouting}
        />
        </Show>

        <Show when={((workspaceTab() === "control" && controlMode() === "mixer") || (workspaceTab() === "touch" && controlMode() === "perform"))}>
        <VideoControlPanel
          mixer={true}
          performance={controlMode() === "perform"}
          performContext={{
            get runtimes() { return videoClipRuntime().layers; },
            get keyboardEnabled() { return !controlLearnMode() && operatorLockMode() === null; },
            get controlLearnActive() { return controlLearnMode() !== null; },
            onOpenEditor: () => selectControlMode("mixer"),
            layout: {
              get topRatio() { return performTopSplitRatio(); },
              get lowerRatio() { return performLowerSplitRatio(); },
              onTopRatio: setPerformTopSplitRatio,
              onLowerRatio: setPerformLowerSplitRatio,
            },
            renderClipInspector: () => renderVideoClipSlotInspector(true),
            renderEffectBrowser: renderFxBrowser,
            get effectEntries() { return fxBrowserEntries(); },
            get isfImportRejections() { return fxRejections(); },
            get locale() { return uiLocale(); },
            onAddEffect: addFxBrowserEntry,
          }}
          invokeCommand={invoke}
          selectedMediaAssetId={selectedMediaLibraryAssetId()}
          onSelectMediaAsset={setSelectedMediaLibraryAssetId}
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
            get liveSourceRows() { return externalVideoTransportLiveSourceRows(); },
            get liveSourceFaultRows() { return externalVideoTransportLiveSourceFaultRows(); },
            get captureFaultRows() { return externalVideoTransportCaptureFaultRows(); },
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
            get blackoutBlockedReason() { return blackoutBlockedReason(); },
            onSetMasterOpacity: setVideoMasterOpacity,
            onSetBlackout: requestPerformVideoBlackout,
          }}
          outputControls={{
            get outputs() { return snapshot().video.outputs; },
            get compositions() { return snapshot().video.compositions; },
            get selectedOutputId() { return selectedVideoOutputId(); },
            get windowSummary() { return videoOutputWindowSummary(); },
            onSelectOutput: setSelectedVideoOutputId,
            onRefreshWindows: () => { void refreshVideoOutputWindowStatuses(); },
            renderPlanState: videoOutputRenderPlanState,
            windowStatusForOutput: videoOutputWindowStatusForOutput,
            windowState: videoOutputWindowState,
          }}
          liveMonitors={{
            get outputHealth() { return liveVideoOutputHealth.outputs(); },
            get outputHealthError() { return liveVideoOutputHealth.error(); },
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
            onSetPath: setVideoPathFromOperator,
            onBrowseSource: selectVideoSourceFile,
            onImportMultiple: importMediaFiles,
            onAddLayer: addVideoLayer,
          }}
          clipGrid={mergeThumbnailViewProps(thumbnailController.layerThumbnailView, {
            get layers() { return snapshot().video.layers; },
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
            get lastKnownLiveAudioInputDeviceIdentity() { return lastKnownLiveAudioInputDeviceIdentity(); },
            localize: (source: string) => translateUiText(source, uiLocale()),
            get liveAudioInputCapabilities() { return liveAudioInputCapabilities(); },
            get liveAudioInputCapabilitiesBusy() { return liveAudioInputCapabilitiesBusy(); },
            get liveAudioInputSampleRate() { return liveAudioInputSampleRate(); },
            get liveAudioInputBufferFrames() { return liveAudioInputBufferFrames(); },
            get liveAudioInputChannelMix() { return liveAudioInputChannelMix(); },
            get liveAudioInputStatus() { return liveAudioInputStatus(); },
            get liveAudioInputStatusKnown() { return liveAudioInputStatusKnown(); },
            get liveAudioInputBusy() { return liveAudioInputBusy(); },
            get liveAudioInputSavedSelection() { return liveAudioInputSavedSelectionUiState(); },
            get liveAudioInputPersistError() { return liveAudioInputPersistError(); },
            get liveAudioInputAsioRevalidationEligible() { return liveAudioInputAsioRevalidationEligible(); },
            get liveAudioInputAsioRevalidationArmed() { return liveAudioInputAsioRevalidationArmed(); },
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
            onCommitAbMix: (mix: number) => applyVideoAbMix(mix, true),
            onLaunchDeck: launchVideoDeck,
            onStartRecording: startVideoOutputRecording,
            onStopRecording: stopVideoOutputRecording,
            onSetLiveAudioInputBackend: selectLiveAudioInputBackend,
            onSetLiveAudioInputDevice: selectLiveAudioInputDevice,
            onSetLiveAudioInputSampleRate: selectLiveAudioInputSampleRate,
            onSetLiveAudioInputBufferFrames: selectLiveAudioInputBufferFrames,
            onSetLiveAudioInputChannelMix: selectLiveAudioInputChannelMix,
            onRefreshLiveAudioInputDevices: refreshLiveAudioInputOperatorState,
            onArmLiveAudioInputAsioRevalidation: armLiveAudioInputAsioRevalidation,
            onStartLiveAudioInput: startLiveAudioInput,
            onStopLiveAudioInput: stopLiveAudioInput,
            onStagePreview: stageVjPreviewLayer,
            onImportMedia: importMediaFiles,
            onCreateFirstRunShow: createFirstRunVjShow,
            onLaunch: launchVideoClipFromGrid,
            onTake: takeVideoClipFromGrid,
            onStop: stopVideoClipFromGrid,
            onMonitorAudio: (layerId: number, volume: number) => playVideoLayerAudioMonitor(layerId, volume, selectedAudioOutputDevice()),
            onStopAudio: stopVideoLayerAudioMonitor,
            onRequestThumbnails: authorizeVideoThumbnailAccess,
          })}
          clipSlotBank={{
            get mode() { return controlMode() === "perform" ? "control" as const : "edit" as const; },
            get layers() { return snapshot().video.layers; },
            get layer() { return selectedVideoClipSlotLayer(); },
            get runtime() { return selectedVideoClipSlotRuntime(); },
            get assets() { return snapshot().video.media_assets; },
            get thumbnails() { return mediaAssetThumbnails(); },
            get selectedSlotId() { return selectedVideoClipSlotId(); },
            onSelectLayer: selectVideoClipSlotLayer,
            onSelect: selectVideoClipSlot,
            onQueue: (slotId) => {
              const layer = selectedVideoClipSlotLayer();
              return layer ? queueVideoClipSlot(layer.id, slotId) : Promise.resolve(null);
            },
            onPreview: previewVideoClipSlot,
            onMore: openVideoClipSlotInspector,
            onImport: () => importMediaFiles("File"),
            onCancelQueue: () => {
              const layer = selectedVideoClipSlotLayer();
              return layer ? cancelQueuedVideoClipSlot(layer.id) : Promise.resolve(null);
            },
            onSetQuantization: updateVideoClipSlotQuantization,
            get transitionKind() { return videoClipTakeKind(); },
            get transitionDurationUnit() { return videoClipTakeDurationUnit(); },
            get transitionDurationMilliunits() { return videoClipTakeDurationMilliunits(); },
            onSetTransitionKind: setVideoClipTakeKind,
            onSetTransitionDurationUnit: setVideoClipTakeDurationUnit,
            onSetTransitionDurationMilliunits: (value) => setVideoClipTakeDurationMilliunits(
              Math.max(1, Math.min(600_000, Math.round(value || 1))),
            ),
          }}
          clipSlotTake={{
            get enabled() { return videoClipSlotTakeEnabled(); },
            get label() { return videoClipSlotTakeLabel(); },
            onTake: takeSelectedVideoClipSlot,
          }}
          mediaLibrary={mergeThumbnailViewProps(thumbnailController.assetThumbnailView, {
            get assets() { return snapshot().video.media_assets; },
            get ndiInputs() { return buildNdiInputHealthRows(snapshot().video.layers, externalVideoTransportStatus()); },
            get availabilityById() { return mediaAssetAvailabilityById(); },
            get activeOperations() { return activeMediaAssetOperations(); },
            get lastImportReport() { return lastMediaAssetImportReport(); },
            backendAvailable: isTauriRuntime(),
            get checkedAtById() { return mediaAssetCheckedAtById(); },
            onVerify: inspectMediaAssetIds,
            onRelink: relinkMediaLibraryAsset,
            onScanRelinkFolder: scanMediaLibraryRelinkFolder,
            onApplyRelinkRequests: applyMediaLibraryRelinkRequests,
            onPreviewStart: beginMediaAssetPreview,
            onPreviewFrame: loadMediaAssetPreviewFrame,
            onPreviewEnd: endMediaAssetPreview,
            onCancelOperation: cancelMediaAssetOperation,
          })}
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
          layerList={videoLayerListProps}
          effectScopes={{
            get catalog() { return videoEffectCatalog(); },
            get compositions() { return snapshot().video.compositions; },
            get outputs() { return snapshot().video.outputs; },
            get transitionRuntime() { return videoTransitionRuntime(); },
            get busy() { return isfEventPulseBusy(); },
            onApplyCatalog: applyVideoEffectCatalog,
            get canvasAuthority() { return JSON.stringify(projectMappingsAuthority()); },
            onLoadGroupPreview: loadVideoGroupCanvasPreview,
            onImportIsf: importVideoEffectScopeIsf,
            onApplyPreset: applyVideoEffectPreset,
            onRemoveChain: removeVideoEffectChain,
            onLaunchTransition: launchVideoLayerTransitionBus,
            onReleaseTransition: releaseVideoLayerTransitionBus,
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
        <Show when={controlMode() === "mixer" && videoClipSlotInspectorOpen() && selectedVideoClipSlotLayer()}>
          {renderVideoClipSlotInspector(false)}
        </Show>
        </Show>

        <Show when={persistentBandVisible() && !paneWindow}>
        <WorkspaceSplitHandle
          axis="horizontal"
          ratio={activeTopSplitRatio()}
          defaultRatio={activeTopSplitDefaultRatio()}
          minFirstPx={280}
          // At the compact-height floor, prioritize the Video desk above while
          // retaining a scrollable persistent stage pane below it.
          minSecondPx={workspaceTab() === "control" && controlMode() === "mixer" ? 220 : 310}
          firstTrackBonusPx={workspaceTab() === "control" ? 36 : 0}
          label="Resize upper and lower workspace panes"
          splitter="upper-lower"
          onCommit={commitActiveTopSplitRatio}
        />
        </Show>

        <Show when={persistentBandVisible()}>
        <PersistentWorkspaceView model={persistentWorkspaceModel} invokeCommand={invoke} />
        </Show>

        <Show when={workspaceTab() === "setup" && setupSubTab() === "authority"}>
          <AgentAuthorityPanel
            invokeCommand={invoke}
          />
        </Show>
        <Show when={workspaceTab() === "setup" && setupSubTab() === "io"}>
        <SetupIoWorkspaceView model={setupIoModel} invokeCommand={invoke} />
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
            blackout={snapshot().safety_blackout_engaged}
            onSetAllBlackout={() => void targetBlackout.engageSafetyBlackout()}
            onUnlock={unlockOperator}
          />
        )}
      </Show>

      <AppStatusLine
        status={appStatus()}
        actionLabel={fixtureGroupDeleteUndoAvailable() ? "Undo group delete" : null}
        onAction={fixtureGroupDeleteUndoAvailable() ? () => void undoDeleteFixtureGroup() : undefined}
      />
    </main>
  );
}
