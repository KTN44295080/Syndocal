import { For } from "solid-js";
import type { VideoOutputConfigDraft } from "../editorDrafts";
import type {
  CompositionSummary,
  VideoOutputMapping,
  VideoOutputMappingPresetSummary,
  VideoOutputSummary,
} from "../types";
import { VideoOutputActionsPanel } from "./VideoOutputActionsPanel";
import { VideoOutputConfigPanel } from "./VideoOutputConfigPanel";
import { VideoOutputMappingPanel } from "./VideoOutputMappingPanel";

type MaybePromise = void | Promise<unknown>;
type VideoOutputPreviewMode = "output" | "test";

type VideoOutputListPanelProps = {
  outputs: VideoOutputSummary[];
  compositions: CompositionSummary[];
  mappingPresets: VideoOutputMappingPresetSummary[];
  mappingPresetLabel: string;
  selectedMappingPresetLabel: string;
  previewOutputId: number | null;
  previewMode: VideoOutputPreviewMode;
  previewInfo: string;
  previewUrl: string;
  configDraftFor: (output: VideoOutputSummary) => VideoOutputConfigDraft;
  onConfigDraft: (output: VideoOutputSummary, patch: Partial<VideoOutputConfigDraft>) => void;
  onApplyConfig: (output: VideoOutputSummary) => MaybePromise;
  onSetRouting: (outputId: number, compositionId: number) => MaybePromise;
  onMappingPresetLabel: (value: string) => void;
  onSelectedMappingPresetLabel: (value: string) => void;
  onSaveMappingPreset: (mapping: VideoOutputMapping) => MaybePromise;
  onExportMappingPreset: (mapping: VideoOutputMapping) => MaybePromise;
  onImportMappingPreset: () => MaybePromise;
  onApplyMappingPreset: (outputId: number, label: string) => MaybePromise;
  onRemoveMappingPreset: (label: string) => MaybePromise;
  onSetMapping: (outputId: number, mapping: VideoOutputMapping) => MaybePromise;
  onSetEnabled: (outputId: number, enabled: boolean) => MaybePromise;
  onSetBlackout: (outputId: number, blackout: boolean) => MaybePromise;
  onSetOpacity: (outputId: number, opacity: number) => MaybePromise;
  onFadeOpacity: (outputId: number, opacity: number) => MaybePromise;
  onPreview: (outputId: number, testPattern?: boolean) => MaybePromise;
  onOpenWindow: (outputId: number, testPattern?: boolean) => MaybePromise;
  onSyncWindow: (outputId: number, testPattern?: boolean) => MaybePromise;
  onRemoveOutput: (outputId: number) => MaybePromise;
};

export function VideoOutputListPanel(props: VideoOutputListPanelProps) {
  return (
    <div class="timelineList">
      <For each={props.outputs}>
        {(output) => (
          <div class="timelineItem">
            <strong>{output.label}</strong>
            <span>
              {output.kind} / {output.width}x{output.height} / C{output.composition_id}
              {output.endpoint_name ? ` / ${output.endpoint_name}` : ""}
            </span>
            <VideoOutputConfigPanel
              output={output}
              draft={props.configDraftFor(output)}
              onDraft={props.onConfigDraft}
              onApply={props.onApplyConfig}
            />
            <label>
              Route
              <select
                value={output.composition_id}
                onInput={(event) => void props.onSetRouting(output.id, Number(event.currentTarget.value))}
              >
                <For each={props.compositions}>
                  {(composition) => <option value={composition.id}>{composition.label}</option>}
                </For>
              </select>
            </label>
            <VideoOutputMappingPanel
              output={output}
              mappingPresetLabel={props.mappingPresetLabel}
              selectedMappingPresetLabel={props.selectedMappingPresetLabel}
              mappingPresets={props.mappingPresets}
              previewOutputId={props.previewOutputId}
              previewMode={props.previewMode}
              previewInfo={props.previewInfo}
              previewUrl={props.previewUrl}
              onMappingPresetLabel={props.onMappingPresetLabel}
              onSelectedMappingPresetLabel={props.onSelectedMappingPresetLabel}
              onSavePreset={props.onSaveMappingPreset}
              onExportPreset={props.onExportMappingPreset}
              onImportPreset={props.onImportMappingPreset}
              onApplyPreset={props.onApplyMappingPreset}
              onRemovePreset={props.onRemoveMappingPreset}
              onSetMapping={props.onSetMapping}
            />
            <VideoOutputActionsPanel
              output={output}
              onSetEnabled={props.onSetEnabled}
              onSetBlackout={props.onSetBlackout}
              onSetOpacity={props.onSetOpacity}
              onFadeOpacity={props.onFadeOpacity}
              onPreview={props.onPreview}
              onOpenWindow={props.onOpenWindow}
              onSyncWindow={props.onSyncWindow}
              onRemove={props.onRemoveOutput}
            />
          </div>
        )}
      </For>
    </div>
  );
}
