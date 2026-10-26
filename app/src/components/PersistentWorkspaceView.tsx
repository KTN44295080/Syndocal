import { Show } from "solid-js";
import { Portal } from "solid-js/web";
import type { PersistentWorkspaceViewModel, FrontendViewInvoke } from "../App";
import {
channelFunctionDetail,
channelFunctionLabel,
channelFunctionRangeLabel,
sortedChannelFunctions
} from "../channelFunctionHelpers";
import { ControlFaderWriteHeader } from "./ControlFaderWriteHeader";
import { EditVideoInspector } from "./EditVideoInspector";
import { FaderAttributeEditorPanel } from "./FaderAttributeEditorPanel";
import { FaderAuxiliaryAttributePanels } from "./FaderAuxiliaryAttributePanels";
import { FaderFixtureControlPanel } from "./FaderFixtureControlPanel";
import { FaderGridPanel } from "./FaderGridPanel";
import { FaderPrimaryAttributePanels } from "./FaderPrimaryAttributePanels";
import { FixtureTypeAttributeColumns } from "./FixtureTypeAttributeColumns";
import { MappingPersistentWorkspaceBand } from "./MappingPersistentWorkspaceBand";
import { PlaybackExecutorPanel } from "./PlaybackExecutorPanel";
import { ProgrammerPanel } from "./ProgrammerPanel";
import { ReferencePalettePanel } from "./ReferencePalettePanel";
import { TimelineCueEventsPanel } from "./TimelineCueEventsPanel";
import { TimelineLightingAutomationPanel } from "./TimelineLightingAutomationPanel";
import { TimelineOutputPreview } from "./TimelineOutputPreview";
import { TimelineSourceShelf } from "./TimelineSourceShelf";
import {
defaultFixtureLimits
} from "../fixtureLimits";
import { groupIdentityCss } from "../identityColor";
import {
clampDmxValue,
dmxValueToPercent,
formatDmxPercent,
formatShortDmxPercent
} from "../numericHelpers";
import {
specializedTimelineSelectionFromItems
} from "../timelineAdvancedResult";
import {
timelineOverviewMarkerAriaLabel,
translateUiText
} from "../uiLocalization";
import {
colorQuickLooks,
defaultColorPalette,
panTiltNudgeSteps,
panTiltTargetPoints,
} from "../uiPresets";
import {
videoSourceKindLabel
} from "../videoHelpers";
import {
wheelSlotMediaPath
} from "../wheelMedia";

export function PersistentWorkspaceView(props: { model: PersistentWorkspaceViewModel; invokeCommand: FrontendViewInvoke }) {
  const invoke = props.invokeCommand;
  const {
    poppedPanes,
    lowerSplitRatio,
    timelinePaneExpanded,
    togglePaneWindow,
    paneWindowOperationPending,
    setLowerSplitRatio,
    setWorkspaceTab,
    selectSetupMode,
    paneWindow,
    workspaceTab,
    controlMode,
    selectedMediaLibraryAsset,
    mediaAssetThumbnails,
    mediaAssetAvailabilityById,
    videoLayerListProps,
    videoEffectCatalog,
    snapshot,
    videoTransitionRuntime,
    isfEventPulseBusy,
    applyVideoEffectCatalog,
    projectMappingsAuthority,
    loadVideoGroupCanvasPreview,
    importVideoEffectScopeIsf,
    applyVideoEffectPreset,
    removeVideoEffectChain,
    launchVideoLayerTransitionBus,
    releaseVideoLayerTransitionBus,
    bankAuthority,
    timelineCueOptions,
    timelineChildCueId,
    uiLocale,
    inspectMediaAssetIds,
    timelineLayers,
    activeTimeline,
    snapTimeMs,
    placeTimelineExternalSource,
    setMessage,
    timelineLowerContextMode,
    setTimelineLowerContextMode,
    renderTimelineInspector,
    isTauriRuntime,
    faderDeskTitle,
    controlFaderModeChanging,
    controlTargetKind,
    controlCompactReadout,
    controlFaderWriteMode,
    selectedSceneCue,
    changeControlFaderWriteMode,
    toggleBlindEditing,
    commitProgrammer,
    clearProgrammer,
    stageWorldBounds,
    stageMapPresetLabel,
    selectedStageMapPresetLabel,
    mappingViewPresetLabel,
    selectedMappingViewPresetId,
    mappingViewPresets,
    stageMapPresetObjectCountLabel,
    mappingViewPresetObjectLabel,
    lockStageMapToCurrentBounds,
    setStageMapConfig,
    setStageMapPresetLabel,
    setSelectedStageMapPresetLabel,
    saveStageMapPreset,
    exportStageMapPreset,
    importStageMapPreset,
    applyStageMapPreset,
    removeStageMapPreset,
    setMappingViewPresetLabel,
    setSelectedMappingViewPresetId,
    saveMappingViewPreset,
    applyMappingViewPreset,
    removeMappingViewPreset,
    exportMappingStageSvg,
    exportVisualizerRenderPayload,
    filteredFixtures,
    selectedFixtureGroupFilter,
    fixtureGroupRows,
    selectedFixtureTypeFilter,
    fixtureTypeRows,
    selectFixtureGroupFilter,
    createFixtureGroup,
    renameFixtureGroup,
    deleteFixtureGroup,
    recolorFixtureGroup,
    setSelectedFixtureTypeFilter,
    mappingStageTool,
    mappingHotkeyHelpOpen,
    setMappingStageTool,
    setMappingHotkeyHelpOpen,
    mappingFilteredFixtures,
    mappingStageCursorWorld,
    mappingStageCursorLabel,
    canFitMappingViewportToVisible,
    canFitMappingViewportToSelection,
    mappingViewportZoomLabel,
    normalizedMappingViewportZoom,
    mappingViewportMaxZoom,
    mappingSnapEnabled,
    normalizedMappingSnapSize,
    mappingShowLabels,
    mappingShowBeams,
    mappingShowGeometry,
    mappingShowProjectors,
    mappingShowStageObjects,
    mappingShowLevels,
    fitMappingViewportToVisible,
    fitMappingViewportToSelection,
    zoomMappingViewport,
    setMappingViewport,
    resetMappingViewport,
    setMappingSnapEnabled,
    setMappingSnapSize,
    setMappingShowLabels,
    setMappingShowBeams,
    setMappingShowGeometry,
    setMappingShowProjectors,
    setMappingShowStageObjects,
    setMappingShowLevels,
    bindMappingStageSvgElement,
    mappingDrag,
    mappingViewportPanDrag,
    mappingStageViewBox,
    mappingViewportBox,
    stageOrigin2d,
    mappingSnapLines,
    mappingMarqueeBox,
    handleMappingStagePointerDown,
    handleMappingStageAuxClick,
    handleMappingStagePointerMove,
    finishMappingStageDrag,
    setMappingStageCursorWorld,
    handleMappingStageWheel,
    mappingStageViewportPixelSize,
    visualizerStageObjects2d,
    visualizerVideoSurfaces2d,
    mappingStageFixtures,
    mappingGeometryNodes2d,
    visualizerFixtures,
    selectedMappingFixtureIdSet,
    selectedFixtureId,
    selectedVideoOutputId,
    mappingPlacePreview,
    isDraggingMappingStageObject,
    isDraggingMappingFixture,
    beginMappingStageObjectDrag,
    beginMappingStageObjectRotate,
    beginMappingStageObjectResize,
    setSelectedVideoOutputId,
    beginMappingFixtureYawDrag,
    beginMappingFixtureDrag,
    selectMappingFixture,
    selectedMappingFixtures,
    mappingFixtureSearch,
    mappingSelectionGroupText,
    parseGroupIds,
    fixtureGroupEntities,
    stageObjectFixtureCounts,
    selectedStageObject,
    selectedStageObjectId,
    stageObjectLabel,
    stageObjectKind,
    stageObjectWidth,
    stageObjectDepth,
    stageObjectRotation,
    stageObjectColor,
    selectedMappingFlagState,
    selectedMappingFixture,
    selectedMappingGeometryRows,
    selectedMappingUnresolvedGeometryReferences,
    selectedFixtureLimitsDraft,
    normalizedSelectedFixtureLimitsDraft,
    selectedFixtureLimitWindowStyle,
    movementLimitDrag,
    panTiltNudgeAmount,
    updateSelectedFixtureLimit,
    setSelectedFixtureLimitsDraft,
    resetSelectedMovementLimits,
    setFixtureLimits,
    startMovementLimitDrag,
    dragMovementLimit,
    endMovementLimitDrag,
    selectedMappingVideoOutput,
    setMappingFixtureSearch,
    setMappingSelectionGroupText,
    pickVisibleMappingFixtures,
    duplicateSelectedMappingFixtures,
    removeSelectedMappingFixtures,
    clearMappingFixtureSelection,
    applyMappingSelectionGroups,
    setStageObjectLabel,
    setStageObjectKind,
    setStageObjectWidth,
    setStageObjectDepth,
    setStageObjectRotation,
    setStageObjectColor,
    addStageObjectAtCenter,
    setSelectedStageObjectId,
    setStageObject,
    removeStageObject,
    pickFixturesInsideSelectedStageObject,
    layoutSelectedFixturesOnStageObject,
    setMappingSelectionFlag,
    nudgeSelectedMappingFixtures,
    layoutSelectedMappingFixtures,
    alignSelectedMappingFixtures,
    distributeSelectedMappingFixtures,
    mirrorSelectedMappingFixtures,
    rotateSelectedMappingFixtures,
    matchSelectedMappingFixtureOrientations,
    aimSelectedMappingFixtureInstallationAxes,
    layoutFixturePositions,
    openSceneFxFromMapping,
    setFixtureTransform,
    timelineDeskSurface,
    timelineContextDrawer,
    editDeskSurface,
    selectedFixtureGroupLabel,
    selectedGroupSubmaster,
    selectedFixture,
    selectedGroupFlagState,
    globalFixtureFlagState,
    setGroupSubmaster,
    savePreset,
    loadPreset,
    loadPresetForSelectedGroup,
    loadPresetForAllMatching,
    duplicateFixture,
    removeFixture,
    setFixtureHighlight,
    setFixtureSolo,
    setFixturePark,
    setGroupHighlight,
    setGroupSolo,
    setGroupPark,
    clearFixtureFlags,
    timelineUpperHost,
    setProgrammerMode,
    referencePalettes,
    controlTargetLabel,
    selectedControlTargetFixtures,
    createReferencePalette,
    updateReferencePalette,
    applyReferencePalette,
    removeReferencePalette,
    createPlaybackExecutor,
    updatePlaybackExecutor,
    removePlaybackExecutor,
    setPlaybackExecutorLevel,
    setPlaybackMaster,
    triggerPlaybackExecutor,
    controlFaderWriteBlocked,
    controlCategoryRows,
    activeControlCategory,
    setControlCategory,
    showFixtureTypeAttributeColumns,
    showDimmerPanel,
    selectedDimmerControl,
    showPositionPad,
    selectedPositionControls,
    showColorPad,
    selectedColorControls,
    dimmerSliderRange,
    selectedFixtureLimitOverlayStyle,
    positionFavorites,
    positionFavoriteLabel,
    selectedColorHsv,
    colorSaturationRamp,
    colorAutoWhite,
    colorFavorites,
    colorPreviewForSaturation,
    setDimmerValue,
    resetSelectedDimmerLimits,
    applySelectedFixtureLimits,
    nudgePanTilt,
    centerPanTilt,
    mirrorPanTiltAxis,
    setPanTiltTargetPoint,
    setPanTiltFromPointer,
    handlePanTiltPadKeyDown,
    setPanTiltValues,
    setPanTiltPercent,
    setPanTiltNudgeAmount,
    setPositionFavoriteLabel,
    addCurrentPositionFavorite,
    resetPositionFavorites,
    removePositionFavorite,
    setColorFromPointer,
    setFixtureColor,
    setColorAutoWhite,
    setColorChannelValue,
    setColorExtraChannelValue,
    setColorHsvValue,
    addCurrentColorFavorite,
    resetColorFavorites,
    removeColorFavorite,
    showColorWheelPanel,
    showColorWheelPickerPanel,
    showGoboWheelPanel,
    showOpticsPanel,
    showCategoryQuickPanel,
    activeControlCategoryLabel,
    visibleControls,
    colorWheelEntries,
    goboWheelEntries,
    colorWheelPickerColor,
    colorWheelPickerHsv,
    colorWheelSaturationRamp,
    colorWheelApproximationLabel,
    wheelMediaUrlForCurrentFixture,
    opticsPanelTitle,
    opticsEntries,
    opticsPreviewClass,
    opticsPreviewStyle,
    opticsPresetButtons,
    channelFunctionContainsValue,
    channelFunctionBandStyle,
    channelFunctionSwatchColor,
    categoryQuickLooks,
    visibleFunctionControls,
    currentControlValue,
    setColorWheelPickerFromPointer,
    setColorWheelPickerColor,
    setColorWheelPickerHsvValue,
    setOpticsValueFromPointer,
    setOpticsValueFromKey,
    setControlAttributeValue,
    applyColorWheelFunction,
    applyChannelFunction,
    applyVisibleControlValues,
    applyCategoryQuickLook,
    selectedControlReferenceFixture,
    faderValue,
    controlIsWritten,
    setAttribute,
    setGroupAttribute,
    pickedFixtureTypeGroups,
    selectPickedFixtureType,
    setFixtureTypeControlValue,
    resetFixtureTypeControlValue,
    setFixtureTypeColor,
    setFixtureTypePosition,
    timelineNavigatorOpen,
    setTimelineNavigatorOpen,
    openRootTimelineFromNavigator,
    openChildTimelineFromNavigator,
    snapshotCues,
    timelineInlineChildParents,
    timelineChildCue,
    cueIdentities,
    timelineBank,
    displayedTimelineFollowRuntime,
    timelineFollowAbortBusy,
    timelineFollowAbortFocusFence,
    timelineExecutionLive,
    superSceneCueCount,
    timelineOverviewEvents,
    timelineCueDrag,
    timelineOverviewAutomationRanges,
    timelineItemLanePlacements,
    timelineOverviewOverlapClusters,
    timelineOverlapClusters,
    selectedTimelineAutomationRangeId,
    timelineOverviewPlayheadX,
    timelineVisibleWindow,
    timelineOverviewShowDurationMs,
    timelineOverviewEditExtentMs,
    selectedTimelineSceneBlockEventId,
    timelineSelection,
    timelineSceneBlockSelectionRevision,
    audioAnalysis,
    audioWaveformPoints,
    audioSpectrumPaths,
    audioBeatMarkers,
    timelineSnapMode,
    timelineGridMs,
    selectedTimelineCueId,
    timelineEventTimeMs,
    timelineBlockDurationMs,
    timelineBlockLoopCount,
    timelineBlockJumpToEventId,
    timelineTrack,
    timelineEventRows,
    timelineStretchMode,
    timelineMagnetEnabled,
    timelineArmedCueId,
    timelineArmedCue,
    timelineEventDraft,
    seekTimeline,
    exitSuperScene,
    openOrCreateSuperScene,
    pauseTimeline,
    playTimeline,
    setTimelineMetronome,
    setTimelineGuideEnabled,
    setTimelinePhases,
    setTimelineLoopRegion,
    setTimelineLoopEnabled,
    scaleTimelineLoop,
    createTimeline,
    duplicateTimeline,
    removeTimeline,
    reorderTimelines,
    selectTimeline,
    setTimelineFollow,
    abortTimelineFollow,
    seekTimelineFromOverviewTime,
    moveTimelineCueEventToPlacement,
    resizeTimelineCueEventToTime,
    setTimelineCueEventFade,
    selectTimelineAutomationRange,
    selectTimelineItem,
    moveTimelineAutomationRangeToTime,
    resizeTimelineAutomationRangeToTime,
    moveTimelineAutomationKeyframeToTime,
    selectTimelineSceneBlockEvent,
    fitTimelineOverview,
    zoomTimelineOverview,
    panTimelineOverview,
    setTimelineVisibleWindowDirect,
    zoomTimelineOverviewAt,
    revealTimelineSceneBlock,
    revealTimelinePlayhead,
    analyzeAudioFile,
    clearTimelineAudio,
    applyAudioBpm,
    addTimelineAudioClip,
    updateTimelineAudioClip,
    setTimelineAudioClipOutputBus,
    updateTimelineVideoClip,
    groupTimelineItems,
    ungroupTimelineItem,
    duplicateTimelineItems,
    nudgeTimelineItems,
    rippleTimelineItems,
    quantizeTimelineItems,
    pasteTimelineItems,
    trimTimelineItems,
    splitTimelineItems,
    moveTimelineItemsToLanes,
    setSelectedTimelineSceneBlockEventId,
    setSelectedTimelineAutomation,
    removeTimelineItems,
    removeTimelineAudioClip,
    setTimelineAudioMaster,
    setTimelineSnapMode,
    setTimelineGridMs,
    snapTimelineDrafts,
    snapTimelineItems,
    placeArmedTimelineCue,
    setTimelineCueId,
    setTimelineEventTimeMs,
    setTimelineBlockDurationMs,
    setTimelineBlockLoopCount,
    setTimelineBlockJumpToEventId,
    setTimelineTrack,
    addTimelineCueEvent,
    addTimelineCueEventAtPlayhead,
    updateTimelineEventDraft,
    setTimelineCueEvent,
    removeTimelineEvent,
    openTimelineSourceCue,
    timelineLayerController,
    removeTimelineLayer,
    setTimelineStretchMode,
    setTimelineMagnetEnabled,
    toggleTimelineArmedCue,
    setTimelineContextDrawer,
    timelineAutomationControls,
    selectedTimelineAutomationAttribute,
    canAddTimelineAutomation,
    canAddTimelineGroupAutomation,
    lightingAutomationRowScope,
    allTimelineAutomationRows,
    automationStartMs,
    automationEndMs,
    automationStartValue,
    automationEndValue,
    automationInterpolation,
    timelineAutomationRows,
    selectedLightingTimelineAutomationId,
    timelineAutomationFixtureOptions,
    timelineAutomationDraft,
    fixtureAttributeOptions,
    setEffectAttribute,
    setAutomationStartMs,
    setAutomationEndMs,
    setAutomationStartValue,
    setAutomationEndValue,
    setAutomationInterpolation,
    setLightingAutomationRowScope,
    usePlayheadForLightingAutomation,
    addTimelineAutomation,
    addTimelineGroupAutomation,
    updateTimelineAutomationDraft,
    alignLightingAutomationDraftToPlayhead,
    addLightingAutomationKeyframeAtPlayhead,
    removeLightingAutomationKeyframeAtPlayhead,
    removeLightingAutomationKeyframe,
    setLightingAutomationKeyframeInterpolation,
    setLightingAutomationKeyframeValue,
    setTimelineAutomationEnabled,
    setLightingAutomationRowsEnabled,
    setTimelineAutomation,
    removeTimelineAutomation
  } = props.model;
  return (
    <MappingPersistentWorkspaceBand
          poppedPanes={poppedPanes()}
          lowerSplitRatio={lowerSplitRatio()}
          timelinePaneExpanded={timelinePaneExpanded()}
          onTogglePaneWindow={togglePaneWindow}
          paneOperationPending={paneWindowOperationPending}
          onLowerSplitRatio={setLowerSplitRatio}
          onOpenMapping={() => {
            setWorkspaceTab("setup");
            selectSetupMode("patch");
          }}
          lowerLeftContent={
            paneWindow !== "stage" && workspaceTab() === "control" && controlMode() === "mixer" ? (
              <section class="panel editVideoPreviewPane" data-edit-video-preview aria-label="Selected media thumbnail">
                <header class="panelHeader"><h2>Thumbnail</h2><span>Selected media</span></header>
                <Show when={selectedMediaLibraryAsset()} fallback={<div class="emptyState">Select Media Library item to view its thumbnail.</div>}>
                  {(asset) => <div class="editVideoPreviewContent">
                    <Show when={mediaAssetThumbnails()[asset().id]} fallback={<div class="videoMediaLibraryThumbnailPlaceholder" aria-hidden="true" />}>
                      <img src={mediaAssetThumbnails()[asset().id]} alt="" />
                    </Show>
                    <strong data-no-localize>{asset().label}</strong>
                    <span data-no-localize>{asset().source.path ?? asset().source.name ?? videoSourceKindLabel(asset().source.kind)}</span>
                  </div>}
                </Show>
              </section>
            ) : undefined
          }
          contextContent={
            workspaceTab() === "control" && controlMode() === "mixer" ? (
              <EditVideoInspector
                asset={selectedMediaLibraryAsset()}
                availabilityById={mediaAssetAvailabilityById()}
                layerFx={videoLayerListProps}
                groupEffects={{
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
              />
            ) : workspaceTab() === "control" && controlMode() === "live" ? (
              <TimelineSourceShelf
                bankAuthority={bankAuthority()}
                cueOptions={timelineCueOptions()}
                timelineChildCueId={timelineChildCueId()}
                uiLocale={uiLocale()}
                mediaAssets={snapshot().video.media_assets}
                mediaAssetAvailabilityById={mediaAssetAvailabilityById()}
                onVerify={inspectMediaAssetIds}
                timelineLayers={timelineLayers()}
                positionMs={activeTimeline().position_ms}
                snapTimeMs={snapTimeMs}
                onPlace={placeTimelineExternalSource}
                onStatus={setMessage}
                contextMode={timelineLowerContextMode()}
                onContextModeChange={setTimelineLowerContextMode}
                onOpenInspector={() => setTimelineLowerContextMode("inspector")}
                inspectorContent={renderTimelineInspector()}
                previewContent={<TimelineOutputPreview outputs={snapshot().video.outputs} invoke={invoke} backendAvailable={isTauriRuntime()} projectEpoch={projectMappingsAuthority().project_epoch} />}
              />
            ) : undefined
          }
          keepChildrenMounted={workspaceTab() === "control" && controlMode() === "live"}
          workspace={workspaceTab() === "setup" ? "setup" : workspaceTab() === "touch" ? "touch" : "control"}
          controlMode={workspaceTab() === "touch" ? "edit" : controlMode()}
          controlHeaderTitle={workspaceTab() === "touch" ? "Faders" : faderDeskTitle()}
          controlHeaderTools={
            <>
              <ControlFaderWriteHeader
                busy={controlFaderModeChanging()}
                targetKind={controlTargetKind()}
                compactReadout={controlCompactReadout()}
                writeMode={controlFaderWriteMode()}
                editingSceneLabel={selectedSceneCue()?.label ?? null}
                editingSceneIdentity={
                  selectedSceneCue()
                    ? groupIdentityCss(`bank:${selectedSceneCue()!.cue_list_id}`, undefined, "fill")
                    : null
                }
                editingSceneIdentityText={
                  selectedSceneCue()
                    ? groupIdentityCss(`bank:${selectedSceneCue()!.cue_list_id}`, undefined, "text")
                    : null
                }
                blindActive={snapshot().programmer.blind}
                blindStagedCount={snapshot().programmer.values.length}
                onWriteMode={changeControlFaderWriteMode}
                onBlindToggle={toggleBlindEditing}
                onBlindCommit={commitProgrammer}
                onBlindDiscard={clearProgrammer}
              />
            </>
          }
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
            onCreateGroup: async (label) => { await createFixtureGroup(label); },
            onRenameGroup: renameFixtureGroup,
            onDeleteGroup: deleteFixtureGroup,
            onRecolorGroup: recolorFixtureGroup,
            onSelectType: setSelectedFixtureTypeFilter,
          }}
          toolRail={{
            stageTool: mappingStageTool(),
            helpOpen: mappingHotkeyHelpOpen(),
            onStageTool: setMappingStageTool,
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
            viewport: mappingViewportBox(),
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
            blindActive: snapshot().programmer.blind,
            showStageObjects: mappingShowStageObjects(),
            showProjectors: mappingShowProjectors(),
            showBeams: mappingShowBeams(),
            showGeometry: mappingShowGeometry(),
            showLabels: mappingShowLabels(),
            labelViewport: {
              x: mappingViewportBox().x,
              z: mappingViewportBox().z,
              width: mappingViewportBox().width,
              height: mappingViewportBox().height,
            },
            labelViewportPixelSize: mappingStageViewportPixelSize(),
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
            availableGroups: fixtureGroupEntities(),
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
              movementLimitNudgeAmount: panTiltNudgeAmount(),
              formatDmxPercent,
              onUpdateNumericLimit: (field, value) => updateSelectedFixtureLimit(field, value),
              onUpdateToggleLimit: (field, value) => updateSelectedFixtureLimit(field, value),
              onResetLimits: () => setSelectedFixtureLimitsDraft(defaultFixtureLimits),
              onResetMovementLimits: resetSelectedMovementLimits,
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
            onMatchSelectionOrientations: matchSelectedMappingFixtureOrientations,
            onAimSelectionInstallationAxes: aimSelectedMappingFixtureInstallationAxes,
            onLayoutFixtures: layoutFixturePositions,
            onControlActive: () => setWorkspaceTab("control"),
            onOpenSceneFx: openSceneFxFromMapping,
            onSetFixtureTransform: setFixtureTransform,
            onControlFixture: () => setWorkspaceTab("control"),
            onPatchFixture: () => selectSetupMode("patch"),
            onSelectFixture: selectMappingFixture,
            onSelectOutput: setSelectedVideoOutputId,
            onEditOutputProjection: (outputId) => {
              setSelectedVideoOutputId(outputId);
              selectSetupMode("video");
            },
          }}
          hotkeyHelpOpen={mappingHotkeyHelpOpen()}
          onCloseHotkeyHelp={() => setMappingHotkeyHelpOpen(false)}
        >
        <Show when={workspaceTab() === "control" || workspaceTab() === "touch"}>
        <section
          class={`panel faders controlPanel timelineDesk-${timelineDeskSurface()} timelineDrawer-${timelineContextDrawer()} editDesk-${editDeskSurface()}`}
          data-timeline-context-drawer={timelineContextDrawer()}
        >
          <FaderFixtureControlPanel
            selectedGroupId={selectedFixtureGroupFilter()}
            selectedGroupLabel={selectedFixtureGroupLabel()}
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
          <Show when={workspaceTab() === "control" && controlMode() === "live" && timelineUpperHost()?.isConnected && timelineDeskSurface() === "playback"}>
          <Portal mount={timelineUpperHost()!}>
          <section class="panel faders controlPanel timelineDesk-playback">
          <div class="playbackDeskSurface">
            <ProgrammerPanel
              programmer={snapshot().programmer}
              onSetMode={setProgrammerMode}
              onCommit={commitProgrammer}
              onClear={clearProgrammer}
            />
            <ReferencePalettePanel
              palettes={referencePalettes()}
              captureFixtureLabel={selectedFixture()?.label ?? null}
              targetLabel={controlTargetLabel()}
              targetFixtureCount={selectedControlTargetFixtures().length}
              onCreate={createReferencePalette}
              onUpdate={updateReferencePalette}
              onApply={applyReferencePalette}
              onRemove={removeReferencePalette}
            />
            <PlaybackExecutorPanel
              bankAuthority={bankAuthority()}
              playbackMaster={snapshot().playback_master}
              onCreate={createPlaybackExecutor}
              onUpdate={updatePlaybackExecutor}
              onRemove={removePlaybackExecutor}
              onSetLevel={setPlaybackExecutorLevel}
              onSetMaster={setPlaybackMaster}
              onTrigger={triggerPlaybackExecutor}
            />
          </div>
          </section>
          </Portal>
          </Show>
          <FaderAttributeEditorPanel
            disabled={controlFaderWriteBlocked()}
            disabledMessage={!selectedSceneCue() ? "Select a scene to edit, or switch to Live." : undefined}
            categories={controlCategoryRows()}
            activeCategory={activeControlCategory()}
            onCategory={setControlCategory}
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
          <div class="timelinePanel">
        <Show when={workspaceTab() === "control" && controlMode() === "live" && timelineUpperHost()?.isConnected && timelineDeskSurface() === "show"}>
            <Portal mount={timelineUpperHost()!}>
            <section class="panel faders controlPanel timelineDesk-show">
            <div class="timelinePanel">
            <div
              class="timelineShowSurface"
              classList={{
                timelineShowSurfaceBlockDrawerOpen: timelineContextDrawer() === "block",
              }}
            >
            <TimelineCueEventsPanel
              navigatorOpen={timelineNavigatorOpen()}
              onCloseNavigator={() => setTimelineNavigatorOpen(false)}
              onNavigateRootTimeline={openRootTimelineFromNavigator}
              onNavigateChildTimeline={openChildTimelineFromNavigator}
              childTimelineCues={snapshotCues()}
              inlineChildParents={timelineInlineChildParents()}
              bankAuthority={bankAuthority()}
              embeddedControls={false}
              contextDrawer={timelineContextDrawer()}
              timelineChildCueId={timelineChildCueId()}
              childTimelineLabel={timelineChildCue()?.label ?? null}
              cueIdentities={cueIdentities()}
              positionMs={activeTimeline().position_ms}
              projectEpoch={projectMappingsAuthority().project_epoch}
              bpm={snapshot().clock.bpm}
              durationMs={activeTimeline().duration_ms}
              playing={activeTimeline().playing}
              metronomeEnabled={activeTimeline().metronome_enabled ?? false}
              countInBeats={activeTimeline().count_in_beats ?? 4}
              countInRemainingMs={activeTimeline().count_in_remaining_ms ?? 0}
              phases={activeTimeline().phases ?? []}
              guideEnabled={activeTimeline().guide_enabled ?? false}
              loopRegion={activeTimeline().loop_region ?? null}
              loopRuntime={activeTimeline().loop_runtime ?? {
                generation: 0,
                status: "disabled",
                wrap_count: 0,
              }}
              timelineBank={timelineBank()}
              activeTimelineId={activeTimeline().id ?? snapshot().timeline.id ?? 0}
              followRuntime={displayedTimelineFollowRuntime()}
              followAbortBusy={timelineFollowAbortBusy()}
              followAbortFocusFence={timelineFollowAbortFocusFence}
              executingLive={timelineExecutionLive()}
              cuesCount={snapshot().cues.length}
              superSceneCueCount={superSceneCueCount()}
              lightingAutomationCount={activeTimeline().automations.length}
              videoAutomationCount={activeTimeline().video_automations.length}
              overviewEvents={timelineOverviewEvents()}
              timelineLayers={timelineLayers()}
              timelineCueDrag={timelineCueDrag()}
              onDropExternalSource={placeTimelineExternalSource}
              overviewMarkerAriaLabel={(event) => timelineOverviewMarkerAriaLabel(event, uiLocale())}
              overviewAutomationRanges={timelineOverviewAutomationRanges()}
              itemLanePlacements={timelineItemLanePlacements()}
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
              timelineSelection={timelineSelection()}
              selectionRevision={timelineSceneBlockSelectionRevision()}
              audioAnalysis={audioAnalysis()}
              audioClips={activeTimeline().audio_clips ?? []}
              videoClips={activeTimeline().video_clips ?? []}
              mediaAssets={snapshot().video.media_assets}
              mediaAssetAvailabilityById={mediaAssetAvailabilityById()}
              itemGroups={activeTimeline().item_groups ?? []}
              audioOffsetMs={activeTimeline().audio_offset_ms ?? 0}
              audioMuted={activeTimeline().audio_muted ?? false}
              audioWaveformPoints={audioWaveformPoints()}
              audioSpectrumPaths={audioSpectrumPaths()}
              audioBeatMarkers={audioBeatMarkers()}
              snapMode={timelineSnapMode()}
              gridMs={timelineGridMs()}
              selectedCueId={selectedTimelineCueId()}
              selectedCueIsSuperScene={bankAuthority().issue === null && Boolean(
                bankAuthority().cueById.get(selectedTimelineCueId() ?? -1)?.child_timeline,
              )}
              eventTimeMs={timelineEventTimeMs()}
              blockDurationMs={timelineBlockDurationMs()}
              blockLoopCount={timelineBlockLoopCount()}
              blockJumpToEventId={timelineBlockJumpToEventId()}
              track={timelineTrack()}
              cueOptions={timelineCueOptions()}
              eventRows={timelineEventRows()}
              stretchMode={timelineStretchMode()}
              magnetEnabled={timelineMagnetEnabled()}
              armedCueId={bankAuthority().issue === null ? timelineArmedCueId() : null}
              armedCue={timelineArmedCue()}
              timelineEventDraft={timelineEventDraft}
              onSeek={seekTimeline}
              onExitChildTimeline={exitSuperScene}
              onOpenSuperScene={(cueId) => void openOrCreateSuperScene(cueId)}
              onPause={pauseTimeline}
              onPlay={playTimeline}
              onSetMetronome={setTimelineMetronome}
              onSetGuideEnabled={setTimelineGuideEnabled}
              onSetPhases={setTimelinePhases}
              onSetLoopRegion={setTimelineLoopRegion}
              onSetLoopEnabled={setTimelineLoopEnabled}
              onScaleLoop={scaleTimelineLoop}
              onCreateTimeline={createTimeline}
              onDuplicateTimeline={duplicateTimeline}
              onRemoveTimeline={removeTimeline}
              onReorderTimelines={reorderTimelines}
              onSelectTimeline={selectTimeline}
              onSetFollow={setTimelineFollow}
              onAbortFollow={abortTimelineFollow}
              onSeekOverviewTime={seekTimelineFromOverviewTime}
              onMoveEventPlacement={moveTimelineCueEventToPlacement}
              onResizeEventTime={resizeTimelineCueEventToTime}
              onSetEventFade={setTimelineCueEventFade}
              onSelectAutomationRange={selectTimelineAutomationRange}
              onTimelineSelectionChange={selectTimelineItem}
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
              onSetAudioClipOutputBus={setTimelineAudioClipOutputBus}
              onUpdateVideoClip={updateTimelineVideoClip}
              onGroupItems={groupTimelineItems}
              onUngroupItem={ungroupTimelineItem}
              onDuplicateItems={duplicateTimelineItems}
              onNudgeItems={nudgeTimelineItems}
              onRippleItems={rippleTimelineItems}
              onQuantizeItems={quantizeTimelineItems}
              onPasteItems={pasteTimelineItems}
              onTrimItems={trimTimelineItems}
              onSplitItems={splitTimelineItems}
              onMoveItemsToLanes={moveTimelineItemsToLanes}
              onRestoreReturnedItemSelection={(items) => {
                const specialized = specializedTimelineSelectionFromItems(items);
                const selected = specialized.event_id !== null
                  ? { kind: "lighting_event" as const, event_id: specialized.event_id }
                  : specialized.automation?.kind === "lighting"
                    ? { kind: "lighting_automation" as const, automation_id: specialized.automation.automationId }
                    : specialized.automation?.kind === "video"
                      ? { kind: "video_automation" as const, automation_id: specialized.automation.automationId }
                      : items[0] ?? null;
                selectTimelineItem(selected);
                // A linked result can legitimately keep one Lighting block
                // and one automation range visibly selected at the same time.
                // The tagged primary above owns the Inspector; these legacy
                // specialized states retain the multi-domain canvas highlight.
                setSelectedTimelineSceneBlockEventId(specialized.event_id);
                setSelectedTimelineAutomation(specialized.automation);
              }}
              onRemoveItems={removeTimelineItems}
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
              onTimelineStatus={(source) => setMessage(translateUiText(source, uiLocale()))}
              onStretchMode={setTimelineStretchMode}
              onMagnetEnabled={setTimelineMagnetEnabled}
              onArmCue={toggleTimelineArmedCue}
              onContextDrawer={setTimelineContextDrawer}
            />
            </div>
            </div>
            </section>
            </Portal>
            </Show>
            <Show when={workspaceTab() === "control" && controlMode() === "live" && timelineUpperHost()?.isConnected && timelineDeskSurface() === "automation"}>
            <Portal mount={timelineUpperHost()!}>
            <section class="panel faders controlPanel timelineDesk-automation">
            <div class="timelinePanel">
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
            </Portal>
            </Show>
          </div>
        </section>
        </Show>
        </MappingPersistentWorkspaceBand>
  );
}
