import { defineConfig } from "vite";
import solid from "vite-plugin-solid";

export default defineConfig({
  plugins: [
    solid({
      babel: {
        compact: false,
      },
    }),
  ],
  clearScreen: false,
  build: {
    rollupOptions: {
      output: {
        manualChunks(id) {
          const normalizedId = id.replace(/\\/g, "/");
          if (normalizedId.includes("/src/uiLocalization")) {
            return "localization";
          }
          if (normalizedId.includes("/src/viewportFixtureData")) {
            return "qa-fixtures";
          }
          if (
            normalizedId.includes("/src/destructiveActions") ||
            normalizedId.includes("/src/desktopWindowMode") ||
            normalizedId.includes("/src/effectVisualization") ||
            normalizedId.includes("/src/hotkeyHelpers") ||
            normalizedId.includes("/src/identityColor") ||
            normalizedId.includes("/src/channelFunctionHelpers") ||
            normalizedId.includes("/src/fixtureControlRuntime") ||
            normalizedId.includes("/src/numericHelpers") ||
            normalizedId.includes("/src/sceneMatrixBankMove") ||
            normalizedId.includes("/src/snapshotRequestGuard") ||
            normalizedId.includes("/src/projectRecentStorage") ||
            normalizedId.includes("/src/projectRecoveryStorage") ||
            normalizedId.includes("/src/numberDisplay") ||
            normalizedId.includes("/src/uiModes") ||
            normalizedId.includes("/src/workspaceLayoutStorage") ||
            normalizedId.includes("/src/workspaceProfiles")
          ) {
            return "shared-helpers";
          }
          if (
            normalizedId.includes("/src/components/FixtureCatalogPanel") ||
            normalizedId.includes("/src/fixtureCatalog")
          ) {
            return "fixture-catalog";
          }
          if (normalizedId.includes("/src/components/FaderGridPanel")) {
            return "fader-controls";
          }
          if (
            normalizedId.includes("/src/components/CueManagementPanel") ||
            normalizedId.includes("/src/components/TimelineCueEventsPanel") ||
            normalizedId.includes("/src/components/TimelineSceneBlocksEditor") ||
            normalizedId.includes("/src/components/TimelineLightingAutomationPanel") ||
            normalizedId.includes("/src/components/TimelineOverview") ||
            normalizedId.includes("/src/components/VideoTimelineAutomationPanel") ||
            normalizedId.includes("/src/createTimelineAutomationController") ||
            normalizedId.includes("/src/createTimelineKeyframeController") ||
            normalizedId.includes("/src/createTimelineOverviewAutomationController") ||
            normalizedId.includes("/src/timelineSceneBlocks") ||
            normalizedId.includes("/src/timelineAutomationHelpers") ||
            normalizedId.includes("/src/components/TimelineLayerToolbar") ||
            normalizedId.includes("/src/createTimelineLayerController") ||
            normalizedId.includes("/src/timelineCueDrag") ||
            normalizedId.includes("/src/timelineLayers")
          ) {
            return "timeline-panels";
          }
          if (
            normalizedId.includes("/src/components/SetupVideoPanel") ||
            normalizedId.includes("/src/components/VideoCompositionSetupPanel") ||
            normalizedId.includes("/src/components/VideoOutputActionsPanel") ||
            normalizedId.includes("/src/components/VideoOutputConfigPanel") ||
            normalizedId.includes("/src/components/VideoOutputCreatePanel") ||
            normalizedId.includes("/src/components/VideoOutputListPanel")
          ) {
            return "video-setup-panels";
          }
          if (
            normalizedId.includes("/src/components/VideoControlPanel") ||
            normalizedId.includes("/src/components/VideoControlOutputsPanel") ||
            normalizedId.includes("/src/components/VideoLayerListPanel") ||
            normalizedId.includes("/src/components/VideoPreviewDiagnosticsPanel") ||
            normalizedId.includes("/src/components/VideoRuntimeStatusPanels") ||
            normalizedId.includes("/src/components/VideoSourceCreatePanel") ||
            normalizedId.includes("/src/createLiveVideoMonitorController") ||
            normalizedId.includes("/src/createVideoRuntimeController") ||
            normalizedId.includes("/src/liveVideoMonitorPacket")
          ) {
            return "video-control-panels";
          }
          if (normalizedId.includes("/src/components/EditableTouchSurface")) {
            return "touch-panels";
          }
          if (
            normalizedId.includes("/src/createMappingViewportModel") ||
            normalizedId.includes("/src/createMappingRenderModel") ||
            normalizedId.includes("/src/createMappingInteractionController") ||
            normalizedId.includes("/src/createMappingLayoutController") ||
            normalizedId.includes("/src/createStageMapController") ||
            normalizedId.includes("/src/fixtureLiveColor") ||
            normalizedId.includes("/src/mappingRuntime") ||
            normalizedId.includes("/src/stageLabelLayout") ||
            normalizedId.includes("/src/components/SetupMappingWorkspace") ||
            normalizedId.includes("/src/components/StageGlyphs") ||
            normalizedId.includes("/src/components/StagePreview2D") ||
            normalizedId.includes("/src/components/Mapping")
          ) {
            return "mapping-panels";
          }
          if (
            normalizedId.includes("/src/components/ProjectorMapEditor") ||
            normalizedId.includes("/src/components/VideoColorFxEditor") ||
            normalizedId.includes("/src/components/VideoCropEditor") ||
            normalizedId.includes("/src/components/VideoPlaybackTimeline") ||
            normalizedId.includes("/src/components/VideoTransformEditor")
          ) {
            return "mapping-editors";
          }
          if (
            normalizedId.includes("/src/createControlInputController") ||
            normalizedId.includes("/src/controlMappingLabels") ||
            normalizedId.includes("/src/components/MidiControlMappingPanel") ||
            normalizedId.includes("/src/components/OscControlMappingPanel")
          ) {
            return "input-panels";
          }
          if (
            normalizedId.includes("/src/createOutputDiagnosticsController") ||
            normalizedId.includes("/src/components/DmxOutputConfigPanel") ||
            normalizedId.includes("/src/components/EngineTelemetryPanel") ||
            normalizedId.includes("/src/components/OutputDiagnosticsPanel")
          ) {
            return "output-panels";
          }
          if (
            normalizedId.includes("/src/components/ChannelFunctionPanel") ||
            normalizedId.includes("/src/components/CategoryQuickPanel") ||
            normalizedId.includes("/src/components/ColorControlPanel") ||
            normalizedId.includes("/src/components/DimmerControlPanel") ||
            normalizedId.includes("/src/components/OpticsControlPanel") ||
            normalizedId.includes("/src/components/PositionControlPanel") ||
            normalizedId.includes("/src/components/SceneMatrixPanel") ||
            normalizedId.includes("/src/components/WheelSlotPanel")
          ) {
            return "control-panels";
          }
          if (
            normalizedId.includes("/src/components/EffectActionControlsPanel") ||
            normalizedId.includes("/src/components/MoveEffectEditorPanel") ||
            normalizedId.includes("/src/components/ValueEffectEditorPanel") ||
            normalizedId.includes("/src/components/CurveEffectEditorPanel") ||
            normalizedId.includes("/src/components/MappingEffectEditorPanel") ||
            normalizedId.includes("/src/components/ColorMappingEffectEditorPanel") ||
            normalizedId.includes("/src/components/ChaserEffectEditorPanel") ||
            normalizedId.includes("/src/components/ColorEffectEditorPanel") ||
            normalizedId.includes("/src/components/EffectFamilyChooser") ||
            normalizedId.includes("/src/components/SceneSettingsPane") ||
            normalizedId.includes("/src/moveEffect")
          ) {
            return "effect-panels";
          }
          if (
            normalizedId.includes("/src/createAppKeyboardController") ||
            normalizedId.includes("/src/statusModel") ||
            normalizedId.includes("/src/components/DesktopWindowModeController") ||
            normalizedId.includes("/src/components/AppStatusLine") ||
            normalizedId.includes("/src/components/GroupChip") ||
            normalizedId.includes("/src/components/WorkspaceChrome") ||
            normalizedId.includes("/src/components/WorkspaceOperationsMenu")
          ) {
            return "workspace-chrome";
          }
          if (!normalizedId.includes("node_modules")) {
            return undefined;
          }
          if (normalizedId.includes("solid-js")) {
            return "solid";
          }
          if (normalizedId.includes("@tauri-apps")) {
            return "tauri";
          }
          return "vendor";
        },
      },
    },
  },
  server: {
    port: 5173,
    strictPort: true,
  },
});
