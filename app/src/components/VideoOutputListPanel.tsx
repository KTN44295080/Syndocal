import { For, Show, createEffect, createMemo, createSignal, type JSX } from "solid-js";
import type {
  CompositionSummary,
  VideoOutputMapping,
  VideoOutputMappingPresetSummary,
  VideoOutputSummary,
} from "../types";
import { mappingCorrectionReadout } from "../videoOutputMapping";
import { VideoOutputActionsPanel } from "./VideoOutputActionsPanel";
import { VideoOutputMappingPanel } from "./VideoOutputMappingPanel";

type MaybePromise = void | Promise<unknown>;

export interface VideoOutputSetupWindowState {
  stateLabel: string;
  stateClass: string;
  detail: string;
  actualOpen: boolean | null;
  actionLabel: string;
  actionDisabled: boolean;
  error: string | null;
}

type VideoOutputListPanelProps = {
  setupTools: JSX.Element;
  outputs: VideoOutputSummary[];
  compositions: CompositionSummary[];
  mappingPresets: VideoOutputMappingPresetSummary[];
  mappingPresetLabel: string;
  selectedMappingPresetLabel: string;
  selectedOutputId: number | null;
  previewOutputId: number | null;
  previewInfo: string;
  previewUrl: string;
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
  windowStateForOutput: (outputId: number) => VideoOutputSetupWindowState;
  windowActionBusyForOutput: (outputId: number) => boolean;
  onToggleWindow: (outputId: number, open: boolean) => MaybePromise;
  routingBusyForOutput: (outputId: number) => boolean;
  onAssignComposition: (outputId: number, compositionId: number) => MaybePromise;
};

export function VideoOutputListPanel(props: VideoOutputListPanelProps) {
  const selectedOutput = createMemo(() => {
    const id = props.selectedOutputId;
    return props.outputs.find((output) => output.id === id) ?? props.outputs[0] ?? null;
  });
  const [routeCompositionId, setRouteCompositionId] = createSignal<number | null>(null);
  createEffect(() => {
    const output = selectedOutput();
    setRouteCompositionId(output?.composition_id ?? null);
  });

  const compositionForOutput = (output: VideoOutputSummary) =>
    props.compositions.find((composition) => composition.id === output.composition_id) ?? null;

  return (
    <div class="videoOutputSetupShell">
      <aside class="videoSetupRoutingPane" aria-labelledby="video-output-routing-heading">
        <div class="videoSetupPaneHeader">
          <h3 id="video-output-routing-heading">Outputs / Sources</h3>
          <span>{props.outputs.length} configured</span>
        </div>
        <div class="videoOutputRail">
          <div class="videoOutputDeckList" aria-label="Video outputs">
            <Show when={props.outputs.length > 0} fallback={
              <div class="videoOutputEmptyState" data-empty-state="video-output">
                <strong>No configured video outputs.</strong>
                <span>Add a Display output to control its live window.</span>
              </div>
            }>
              <For each={props.outputs}>
                {(output) => {
                  const active = () => selectedOutput()?.id === output.id;
                  const windowState = () => props.windowStateForOutput(output.id);
                  return (
                    <button
                      type="button"
                      class={active() ? "videoOutputDeck active" : "videoOutputDeck"}
                      aria-pressed={active()}
                      onClick={() => props.onSelectOutput(output.id)}
                    >
                      <strong data-no-localize>{output.label}</strong>
                      <span>{output.kind} / {output.width}x{output.height}</span>
                      <small>
                        Authored enabled: {output.enabled ? "yes" : "no"} / {mappingCorrectionReadout(output.mapping)}
                      </small>
                      <small class={`outputWindowStatus state-${windowState().stateClass}`}>
                        {windowState().stateLabel}
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

      <Show when={selectedOutput()} fallback={
        <main class="videoSetupMapPane videoOutputNoSelection" aria-live="polite">
          <h3>Video output status</h3>
          <p>No configured video outputs. Add a Display output to control its live window.</p>
        </main>
      }>
        {(output) => {
          const windowState = () => props.windowStateForOutput(output().id);
          const busy = () => props.windowActionBusyForOutput(output().id);
          const routingBusy = () => props.routingBusyForOutput(output().id);
          const ownershipBlocked = () => windowState().stateClass === "blocked";
          return (
            <main
              class="videoSetupMapPane videoOutputStatusPane"
              aria-labelledby={`video-output-status-heading-${output().id}`}
            >
              <div class="videoOutputDetailHeader">
                <div>
                  <h3 id={`video-output-status-heading-${output().id}`} data-no-localize>{output().label}</h3>
                  <span>{output().kind} / {output().width}x{output().height}</span>
                </div>
                <strong>{output().enabled ? "Authored enabled" : "Authored disabled"}</strong>
              </div>

              <div class="videoOutputStatusSummary" role={windowState().stateClass === "error" ? "alert" : "status"} aria-live="polite">
                <div>
                  <span>Physical window state</span>
                  <strong class={`outputWindowStatus state-${windowState().stateClass}`}>
                    {windowState().stateLabel}
                  </strong>
                </div>
                <small>{windowState().detail}</small>
              </div>

              <div class="videoOutputRouteSummary">
                <div>
                  <span>Route</span>
                  <Show
                    when={compositionForOutput(output())}
                    fallback={(
                      <strong>
                        Composition <span data-no-localize>{output().composition_id}</span>
                      </strong>
                    )}
                  >
                    {(composition) => <strong data-no-localize>{composition().label}</strong>}
                  </Show>
                </div>
                <label>
                  <span>Composition</span>
                  <select
                    value={String(routeCompositionId() ?? output().composition_id)}
                    disabled={routingBusy() || props.compositions.length === 0}
                    onChange={(event) => {
                      const compositionId = Number(event.currentTarget.value);
                      setRouteCompositionId(Number.isSafeInteger(compositionId) && compositionId > 0
                        ? compositionId
                        : null);
                    }}
                  >
                    <For each={props.compositions}>
                      {(composition) => (
                        <option data-no-localize value={composition.id}>{composition.label}</option>
                      )}
                    </For>
                  </select>
                </label>
                <button
                  type="button"
                  disabled={routingBusy() || routeCompositionId() === null}
                  onClick={() => {
                    const compositionId = routeCompositionId();
                    if (compositionId !== null) void props.onAssignComposition(output().id, compositionId);
                  }}
                >
                  {routingBusy() ? "Applying route…" : "Apply route"}
                </button>
                <small>Changing the route requires local output authority and confirmation.</small>
              </div>

              <Show when={props.previewOutputId === output().id}>
                <div class="videoOutputPreviewCard" aria-label="Output Preview">
                  <div class="sectionHeader">
                    <h4>Output Preview</h4>
                    <span>{props.previewInfo}</span>
                  </div>
                  <Show when={props.previewUrl} fallback={<span class="emptyState">No preview</span>}>
                    {(url) => <img src={url()} alt={`${output().label} preview`} />}
                  </Show>
                </div>
              </Show>

              <VideoOutputActionsPanel
                output={output()}
                actualOpen={windowState().actualOpen}
                busy={busy()}
                blocked={ownershipBlocked() || windowState().actionDisabled}
                error={windowState().error}
                onToggleWindow={props.onToggleWindow}
              />

              <details class="advancedProjectionCorrection">
                <summary>Advanced projection correction</summary>
                <p>Ordinary fullscreen display output does not need this; use it for projector and corner correction.</p>
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
                  readOnly
                />
              </details>
            </main>
          );
        }}
      </Show>
    </div>
  );
}
