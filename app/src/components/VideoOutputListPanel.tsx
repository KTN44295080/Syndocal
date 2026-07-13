import { For, Show, createMemo, type JSX } from "solid-js";
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
  setupTools: JSX.Element;
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

  const previewLabel = (output: VideoOutputSummary) =>
    props.previewOutputId === output.id
      ? `${props.previewMode === "test" ? "Pattern" : "Output"} / ${props.previewInfo}`
      : "No preview";

  return (
    <div class="videoOutputSetupShell">
      <aside class="videoSetupRoutingPane" aria-labelledby="video-output-routing-heading">
        <div class="videoSetupPaneHeader">
          <h3 id="video-output-routing-heading">Outputs / Sources</h3>
          <span>{props.outputs.length} configured</span>
        </div>
        <div class="videoOutputRail">
          <div class="videoOutputDeckList" aria-label="Video outputs">
            <Show when={props.outputs.length > 0} fallback={<span class="emptyState">No video outputs</span>}>
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
            </Show>
          </div>
          {props.setupTools}
        </div>
      </aside>

      <Show when={selectedOutput()}>
        {(output) => (
          <>
            <section
              class="videoSetupMapPane"
              aria-labelledby={`video-output-map-heading-${output().id}`}
            >
              <div class="videoOutputDetailHeader">
                <div>
                  <h3 id={`video-output-map-heading-${output().id}`} data-no-localize>{output().label}</h3>
                  <span>
                    {output().kind} / {output().width}x{output().height} / {compositionLabel(output())}
                  </span>
                </div>
                <strong>{output().enabled ? (output().blackout ? "Blackout" : "Live") : "Disabled"}</strong>
              </div>
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
            </section>
            <aside
              class="videoSetupInspectorPane videoOutputDetailPane"
              aria-labelledby={`video-output-inspector-heading-${output().id}`}
            >
              <div class="videoSetupPaneHeader">
                <h3 id={`video-output-inspector-heading-${output().id}`}>Output Controls</h3>
                <span>Configuration and live actions</span>
              </div>
              <div class="videoOutputPreviewCard">
                <div class="sectionHeader">
                  <h4>Output Preview</h4>
                  <span>{previewLabel(output())}</span>
                </div>
                <Show
                  when={props.previewOutputId === output().id && props.previewUrl}
                  fallback={<span class="emptyState">No preview</span>}
                >
                  {(url) => <img src={url()} alt={`${output().label} preview`} />}
                </Show>
              </div>
              <VideoOutputConfigPanel
                output={output()}
                draft={props.configDraftFor(output())}
                onDraft={props.onConfigDraft}
                onApply={props.onApplyConfig}
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
            </aside>
          </>
        )}
      </Show>
    </div>
  );
}
