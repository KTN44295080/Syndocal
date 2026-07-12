import { For, Show, createMemo } from "solid-js";
import type { VideoOutputConfigDraft } from "../editorDrafts";
import type {
  CompositionSummary,
  VideoOutputMapping,
  VideoOutputMappingPresetSummary,
  VideoOutputSummary,
} from "../types";
import { mappingCorrectionReadout } from "../videoOutputMapping";
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
  selectedOutputId: number | null;
  previewOutputId: number | null;
  previewMode: VideoOutputPreviewMode;
  previewInfo: string;
  previewUrl: string;
  configDraftFor: (output: VideoOutputSummary) => VideoOutputConfigDraft;
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

export function VideoOutputListPanel(props: VideoOutputListPanelProps) {
  const selectedOutput = createMemo(() => {
    const id = props.selectedOutputId;
    return props.outputs.find((output) => output.id === id) ?? props.outputs[0] ?? null;
  });

  const compositionLabel = (output: VideoOutputSummary) =>
    props.compositions.find((composition) => composition.id === output.composition_id)?.label ??
    `Composition ${output.composition_id}`;

  return (
    <div class="videoOutputSetupShell">
      <Show when={props.outputs.length > 0} fallback={<span class="emptyState">No video outputs</span>}>
        <div class="videoOutputDeckList" aria-label="Video outputs">
          <For each={props.outputs}>
            {(output) => {
              const active = () => selectedOutput()?.id === output.id;
              return (
                <button
                  type="button"
                  class={active() ? "videoOutputDeck active" : "videoOutputDeck"}
                  aria-pressed={active()}
                  onClick={() => props.onSelectOutput(output.id)}
                >
                  <strong data-no-localize>{output.label}</strong>
                  <span>
                    {output.kind} / {output.width}x{output.height} / {Math.round(output.opacity * 100)}%
                  </span>
                  <small>
                    {compositionLabel(output)} / {mappingCorrectionReadout(output.mapping)}
                    {!output.enabled ? " / Disabled" : ""}
                    {output.blackout ? " / Blackout" : ""}
                  </small>
                </button>
              );
            }}
          </For>
        </div>

        <Show when={selectedOutput()}>
          {(output) => (
            <div class="videoOutputDetailPane">
              <div class="videoOutputDetailHeader">
                <div>
                  <h3 data-no-localize>{output().label}</h3>
                  <span>
                    {output().kind} / {output().width}x{output().height} / {compositionLabel(output())}
                  </span>
                </div>
                <strong>{output().enabled ? (output().blackout ? "Blackout" : "Live") : "Disabled"}</strong>
              </div>
              <VideoOutputConfigPanel
                output={output()}
                draft={props.configDraftFor(output())}
                onDraft={props.onConfigDraft}
                onApply={props.onApplyConfig}
              />
              <label>
                Route
                <select
                  value={output().composition_id}
                  onInput={(event) => void props.onSetRouting(output().id, Number(event.currentTarget.value))}
                >
                  <For each={props.compositions}>
                    {(composition) => <option data-no-localize value={composition.id}>{composition.label}</option>}
                  </For>
                </select>
              </label>
              <VideoOutputMappingPanel
                output={output()}
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
                onImportBitmapMask={props.onImportBitmapMask}
                onClearBitmapMask={props.onClearBitmapMask}
              />
              <VideoOutputActionsPanel
                output={output()}
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
        </Show>
      </Show>
    </div>
  );
}
