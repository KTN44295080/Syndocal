import type { VideoOutputConfigDraft } from "../editorDrafts";
import type { ComponentProps } from "solid-js";
import type {
  CompositionSummary,
  VideoLayerSummary,
  VideoOutputKind,
  VideoOutputMapping,
  VideoOutputMappingPresetSummary,
  VideoOutputSummary,
} from "../types";
import { VideoCompositionSetupPanel } from "./VideoCompositionSetupPanel";
import { LiveAudioInputRail } from "./LiveAudioInputRail";
import { VideoOutputCreatePanel } from "./VideoOutputCreatePanel";
import { VideoOutputListPanel } from "./VideoOutputListPanel";

type MaybePromise = void | Promise<unknown>;
type VideoOutputPreviewMode = "output" | "test";

type SetupVideoPanelProps = {
  className: string;
  panelRef: (element: HTMLElement) => void;
  outputs: VideoOutputSummary[];
  compositions: CompositionSummary[];
  layers: VideoLayerSummary[];
  mappingPresets: VideoOutputMappingPresetSummary[];
  compositionLabel: string;
  compositionLayerIds: number[];
  outputLabel: string;
  outputKind: VideoOutputKind;
  outputWidth: number;
  outputHeight: number;
  outputFadeMs: number;
  outputMonitorId: number;
  outputFullscreen: boolean;
  outputEndpoint: string;
  mappingPresetLabel: string;
  selectedMappingPresetLabel: string;
  selectedOutputId: number | null;
  previewOutputId: number | null;
  previewMode: VideoOutputPreviewMode;
  previewInfo: string;
  previewUrl: string;
  liveAudioInput: ComponentProps<typeof LiveAudioInputRail>;
  configDraftFor: (output: VideoOutputSummary) => VideoOutputConfigDraft;
  onCompositionLabel: (value: string) => void;
  onToggleCompositionLayer: (layerId: number, checked: boolean) => void;
  onAddComposition: () => MaybePromise;
  onRemoveComposition: (compositionId: number) => MaybePromise;
  onSetCompositionLayers: (compositionId: number, layerIds: number[]) => MaybePromise;
  onMoveCompositionLayer: (
    compositionId: number,
    layerIds: number[],
    layerId: number,
    direction: -1 | 1,
  ) => MaybePromise;
  onOutputLabel: (value: string) => void;
  onOutputKind: (value: VideoOutputKind) => void;
  onOutputWidth: (value: number) => void;
  onOutputHeight: (value: number) => void;
  onOutputFadeMs: (value: number) => void;
  onOutputMonitorId: (value: number) => void;
  onOutputFullscreen: (value: boolean) => void;
  onOutputEndpoint: (value: string) => void;
  onAddOutput: () => MaybePromise;
  onConfigDraft: (output: VideoOutputSummary, patch: Partial<VideoOutputConfigDraft>) => void;
  onApplyConfig: (output: VideoOutputSummary) => MaybePromise;
  onSelectOutput: (outputId: number) => void;
  onSetRouting: (outputId: number, compositionId: number) => MaybePromise;
  onMappingPresetLabel: (value: string) => void;
  onSelectedMappingPresetLabel: (value: string) => void;
  onSaveMappingPreset: (mapping: VideoOutputMapping) => MaybePromise;
  onExportMappingPreset: (mapping: VideoOutputMapping) => MaybePromise;
  onImportMappingPreset: () => MaybePromise;
  onApplyMappingPreset: (outputId: number, label: string) => MaybePromise;
  onRemoveMappingPreset: (label: string) => MaybePromise;
  onSetMapping: (outputId: number, mapping: VideoOutputMapping) => MaybePromise;
  onImportBitmapMask: (output: VideoOutputSummary) => MaybePromise;
  onClearBitmapMask: (output: VideoOutputSummary) => MaybePromise;
  onSetEnabled: (outputId: number, enabled: boolean) => MaybePromise;
  onSetBlackout: (outputId: number, blackout: boolean) => MaybePromise;
  onSetOpacity: (outputId: number, opacity: number) => MaybePromise;
  onFadeOpacity: (outputId: number, opacity: number) => MaybePromise;
  onPreview: (outputId: number, testPattern?: boolean) => MaybePromise;
  onOpenWindow: (outputId: number, testPattern?: boolean) => MaybePromise;
  onSyncWindow: (outputId: number, testPattern?: boolean) => MaybePromise;
  onRemoveOutput: (outputId: number) => MaybePromise;
};

export function SetupVideoPanel(props: SetupVideoPanelProps) {
  return (
    <section class={props.className} ref={(element) => props.panelRef(element)} tabIndex={-1}>
      <div class="panelHeader">
        <h2>Video Setup</h2>
        <span>{props.outputs.length} video output(s)</span>
      </div>
      <div class="videoSetupWorkbench">
        <div class="videoSetupOutputDesk">
          <VideoOutputListPanel
            setupTools={
              <div class="videoSetupSidebar" aria-label="Video setup tools">
                <VideoCompositionSetupPanel
                  compositions={props.compositions}
                  layers={props.layers}
                  draftLabel={props.compositionLabel}
                  draftLayerIds={props.compositionLayerIds}
                  onDraftLabel={props.onCompositionLabel}
                  onToggleDraftLayer={props.onToggleCompositionLayer}
                  onAddComposition={props.onAddComposition}
                  onRemoveComposition={props.onRemoveComposition}
                  onSetCompositionLayers={props.onSetCompositionLayers}
                  onMoveCompositionLayer={props.onMoveCompositionLayer}
                />
                <VideoOutputCreatePanel
                  label={props.outputLabel}
                  kind={props.outputKind}
                  width={props.outputWidth}
                  height={props.outputHeight}
                  fadeMs={props.outputFadeMs}
                  monitorId={props.outputMonitorId}
                  fullscreen={props.outputFullscreen}
                  endpoint={props.outputEndpoint}
                  onLabel={props.onOutputLabel}
                  onKind={props.onOutputKind}
                  onWidth={props.onOutputWidth}
                  onHeight={props.onOutputHeight}
                  onFadeMs={props.onOutputFadeMs}
                  onMonitorId={props.onOutputMonitorId}
                  onFullscreen={props.onOutputFullscreen}
                  onEndpoint={props.onOutputEndpoint}
                  onAddOutput={props.onAddOutput}
                />
                <div
                  id="setup-output-audio-input"
                  class="setupOutputAudioInput"
                  role="region"
                  aria-label="Audio input settings"
                  tabIndex={-1}
                >
                  <div class="setupOutputAudioInputHeader">
                    <strong>Audio Input</strong>
                    <span>Live analysis</span>
                  </div>
                  <LiveAudioInputRail {...props.liveAudioInput} compact />
                </div>
              </div>
            }
            outputs={props.outputs}
            compositions={props.compositions}
            mappingPresets={props.mappingPresets}
            mappingPresetLabel={props.mappingPresetLabel}
            selectedMappingPresetLabel={props.selectedMappingPresetLabel}
            selectedOutputId={props.selectedOutputId}
            previewOutputId={props.previewOutputId}
            previewMode={props.previewMode}
            previewInfo={props.previewInfo}
            previewUrl={props.previewUrl}
            configDraftFor={props.configDraftFor}
            onConfigDraft={props.onConfigDraft}
            onApplyConfig={props.onApplyConfig}
            onSelectOutput={props.onSelectOutput}
            onSetRouting={props.onSetRouting}
            onMappingPresetLabel={props.onMappingPresetLabel}
            onSelectedMappingPresetLabel={props.onSelectedMappingPresetLabel}
            onSaveMappingPreset={props.onSaveMappingPreset}
            onExportMappingPreset={props.onExportMappingPreset}
            onImportMappingPreset={props.onImportMappingPreset}
            onApplyMappingPreset={props.onApplyMappingPreset}
            onRemoveMappingPreset={props.onRemoveMappingPreset}
            onSetMapping={props.onSetMapping}
            onImportBitmapMask={props.onImportBitmapMask}
            onClearBitmapMask={props.onClearBitmapMask}
            onSetEnabled={props.onSetEnabled}
            onSetBlackout={props.onSetBlackout}
            onSetOpacity={props.onSetOpacity}
            onFadeOpacity={props.onFadeOpacity}
            onPreview={props.onPreview}
            onOpenWindow={props.onOpenWindow}
            onSyncWindow={props.onSyncWindow}
            onRemoveOutput={props.onRemoveOutput}
          />
        </div>
      </div>
    </section>
  );
}
