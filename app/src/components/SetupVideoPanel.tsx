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
import type { VideoDisplayMonitorDescriptor } from "./VideoOutputCreatePanel";
import { VideoOutputListPanel } from "./VideoOutputListPanel";
import type { FrontendTauriInvoke } from "../tauriInvokeCommands";

type MaybePromise = void | Promise<unknown>;

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
  previewInfo: string;
  previewUrl: string;
  liveAudioInput: ComponentProps<typeof LiveAudioInputRail>;
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
  invokeCommand: FrontendTauriInvoke;
  onAddDisplayOutput: (monitor: VideoDisplayMonitorDescriptor) => Promise<void>;
  onSelectOutput: (outputId: number) => void;
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
  windowStateForOutput: (outputId: number) => {
    stateLabel: string;
    stateClass: string;
    detail: string;
    actualOpen: boolean | null;
    actionLabel: string;
    actionDisabled: boolean;
    error: string | null;
  };
  windowActionBusyForOutput: (outputId: number) => boolean;
  onToggleWindow: (outputId: number, open: boolean) => MaybePromise;
  routingBusyForOutput: (outputId: number) => boolean;
  onAssignComposition: (outputId: number, compositionId: number) => MaybePromise;
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
                  invokeCommand={props.invokeCommand}
                  onAddDisplayOutput={props.onAddDisplayOutput}
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
            previewInfo={props.previewInfo}
            previewUrl={props.previewUrl}
            onSelectOutput={props.onSelectOutput}
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
            windowStateForOutput={props.windowStateForOutput}
            windowActionBusyForOutput={props.windowActionBusyForOutput}
            onToggleWindow={props.onToggleWindow}
            routingBusyForOutput={props.routingBusyForOutput}
            onAssignComposition={props.onAssignComposition}
          />
        </div>
      </div>
    </section>
  );
}
