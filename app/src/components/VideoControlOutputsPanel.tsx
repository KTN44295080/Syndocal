import { For, Show } from "solid-js";
import type { CompositionSummary, VideoOutputSummary, VideoOutputWindowStatus } from "../types";

export interface VideoOutputRenderPlanStateView {
  layerCount: number;
  stateLabel: string;
  stateClass: string;
  detail: string;
}

export interface VideoOutputWindowStateView {
  stateLabel: string;
  stateClass: string;
  detail: string;
}

interface VideoMasterControlsPanelProps {
  masterOpacity: number;
  blackout: boolean;
  onSetMasterOpacity: (opacity: number) => void | Promise<void>;
  onSetBlackout: (blackout: boolean) => void | Promise<void>;
}

interface VideoOutputControlListPanelProps {
  outputs: VideoOutputSummary[];
  compositions: CompositionSummary[];
  fadeMs: number;
  windowSummary: string;
  onSetFadeMs: (fadeMs: number) => void;
  onRefreshWindows: () => void | Promise<void>;
  onOpenAllWindows: (testPattern?: boolean) => void | Promise<void>;
  onSyncOpenWindows: () => void | Promise<void>;
  onCloseOpenWindows: () => void | Promise<void>;
  renderPlanState: (output: VideoOutputSummary) => VideoOutputRenderPlanStateView;
  windowStatusForOutput: (outputId: number) => VideoOutputWindowStatus | null;
  windowState: (outputId: number) => VideoOutputWindowStateView;
  onSetOutputEnabled: (outputId: number, enabled: boolean) => void | Promise<void>;
  onSetOutputBlackout: (outputId: number, blackout: boolean) => void | Promise<void>;
  onFadeOutputOpacity: (outputId: number, opacity: number) => void | Promise<void>;
  onSetOutputOpacity: (outputId: number, opacity: number) => void | Promise<void>;
  onOpenOutputWindow: (outputId: number, testPattern?: boolean) => void | Promise<void>;
  onSyncOutputWindow: (outputId: number, testPattern?: boolean) => void | Promise<void>;
  onCloseOutputWindow: (outputId: number, testPattern?: boolean) => void | Promise<void>;
}

export function VideoMasterControlsPanel(props: VideoMasterControlsPanelProps) {
  return (
    <div class="videoMasterControls">
      <label>
        Master
        <input
          type="number"
          min="0"
          max="1"
          step="0.01"
          value={props.masterOpacity}
          onChange={(event) => void props.onSetMasterOpacity(Number(event.currentTarget.value))}
        />
      </label>
      <div class="buttonRow">
        <button onClick={() => void props.onSetBlackout(true)} disabled={props.blackout}>
          V Blackout
        </button>
        <button onClick={() => void props.onSetBlackout(false)} disabled={!props.blackout}>
          V Clear
        </button>
      </div>
    </div>
  );
}

export function VideoOutputControlListPanel(props: VideoOutputControlListPanelProps) {
  const compositionLabel = (output: VideoOutputSummary) =>
    props.compositions.find((composition) => composition.id === output.composition_id)?.label ??
    `Composition ${output.composition_id}`;

  return (
    <div class="videoOutputControlList">
      <div class="sectionHeader">
        <h3>Outputs</h3>
        <label>
          Fade ms
          <input
            type="number"
            min="0"
            step="10"
            value={props.fadeMs}
            onInput={(event) => props.onSetFadeMs(Number(event.currentTarget.value))}
          />
        </label>
      </div>
      <div class="videoWindowStatusBar">
        <span>{props.windowSummary}</span>
        <div class="buttonRow">
          <button onClick={() => void props.onRefreshWindows()}>Check Windows</button>
          <button onClick={() => void props.onOpenAllWindows()}>Open All</button>
          <button onClick={() => void props.onOpenAllWindows(true)}>Pattern All</button>
          <button onClick={() => void props.onSyncOpenWindows()}>Sync Open</button>
          <button onClick={() => void props.onCloseOpenWindows()}>Close Open</button>
        </div>
      </div>
      <Show when={props.outputs.length > 0} fallback={<span class="emptyState">No video outputs</span>}>
        <For each={props.outputs}>
          {(output) => {
            const renderPlanState = () => props.renderPlanState(output);
            const outputWindowStatus = () => props.windowStatusForOutput(output.id);
            const windowState = () => props.windowState(output.id);
            const outputLive = () => renderPlanState().stateLabel === "Live" && output.opacity > 0;
            return (
              <div class={outputLive() ? "videoOutputControlItem active" : "videoOutputControlItem"}>
                <div>
                  <strong>{output.label}</strong>
                  <span>
                    {output.kind} / {compositionLabel(output)} / {Math.round(output.opacity * 100)}%
                    {!output.enabled ? " / Disabled" : ""}
                    {output.blackout ? " / Blackout" : ""}
                  </span>
                  <span class={`outputRenderPlanStatus state-${renderPlanState().stateClass}`}>
                    {renderPlanState().stateLabel} / {renderPlanState().layerCount} layer
                    {renderPlanState().layerCount === 1 ? "" : "s"}
                  </span>
                  <small title={renderPlanState().detail}>{renderPlanState().detail}</small>
                  <Show when={output.kind === "Display"}>
                    <span class={`outputWindowStatus state-${windowState().stateClass}`}>{windowState().stateLabel}</span>
                    <small title={windowState().detail}>{windowState().detail}</small>
                  </Show>
                  <div class="outputOpacityMeter">
                    <span style={{ width: `${Math.round(output.opacity * 100)}%` }} />
                  </div>
                </div>
                <div class="buttonRow">
                  <button onClick={() => void props.onSetOutputEnabled(output.id, !output.enabled)}>
                    {output.enabled ? "Disable" : "Enable"}
                  </button>
                  <button onClick={() => void props.onSetOutputBlackout(output.id, !output.blackout)}>
                    {output.blackout ? "Clear" : "Blackout"}
                  </button>
                  <button onClick={() => void props.onFadeOutputOpacity(output.id, 0)}>Fade Out</button>
                  <button onClick={() => void props.onFadeOutputOpacity(output.id, 1)}>Fade In</button>
                  <button onClick={() => void props.onSetOutputOpacity(output.id, 1)}>Full</button>
                  <Show when={output.kind === "Display"}>
                    <button onClick={() => void props.onOpenOutputWindow(output.id)}>Window</button>
                    <button onClick={() => void props.onOpenOutputWindow(output.id, true)}>Pattern</button>
                    <button onClick={() => void props.onSyncOutputWindow(output.id)}>Sync</button>
                    <button onClick={() => void props.onSyncOutputWindow(output.id, true)}>Sync Pattern</button>
                    <Show when={outputWindowStatus()?.live_open}>
                      <button onClick={() => void props.onCloseOutputWindow(output.id)}>Close</button>
                    </Show>
                    <Show when={outputWindowStatus()?.test_pattern_open}>
                      <button onClick={() => void props.onCloseOutputWindow(output.id, true)}>Close Pattern</button>
                    </Show>
                  </Show>
                </div>
              </div>
            );
          }}
        </For>
      </Show>
    </div>
  );
}
