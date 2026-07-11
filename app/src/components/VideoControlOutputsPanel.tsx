import { createEffect, createMemo, createSignal, For, Show } from "solid-js";
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
  compact?: boolean;
  compositions: CompositionSummary[];
  selectedOutputId: number | null;
  fadeMs: number;
  windowSummary: string;
  onSelectOutput: (outputId: number) => void;
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
      <div class="videoMasterReadout">
        <strong>Master</strong>
        <span>{Math.round(props.masterOpacity * 100)}%</span>
      </div>
      <label class="videoMasterFader">
        <span>Level</span>
        <input
          type="range"
          min="0"
          max="1"
          step="0.01"
          value={props.masterOpacity}
          onInput={(event) => void props.onSetMasterOpacity(Number(event.currentTarget.value))}
        />
      </label>
      <div class="buttonRow">
        <button onClick={() => void props.onSetMasterOpacity(0)} disabled={props.masterOpacity <= 0}>
          Out
        </button>
        <button onClick={() => void props.onSetMasterOpacity(1)} disabled={props.masterOpacity >= 1}>
          Full
        </button>
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
  const [page, setPage] = createSignal(0);
  const pageSize = () => (props.compact ? 1 : Math.max(1, props.outputs.length));
  const pageCount = createMemo(() => Math.max(1, Math.ceil(props.outputs.length / pageSize())));
  const visibleOutputs = createMemo(() => {
    const start = page() * pageSize();
    return props.outputs.slice(start, start + pageSize());
  });

  createEffect(() => {
    if (page() >= pageCount()) setPage(pageCount() - 1);
  });

  const compositionLabel = (output: VideoOutputSummary) =>
    props.compositions.find((composition) => composition.id === output.composition_id)?.label ??
    `Composition ${output.composition_id}`;

  return (
    <div class={`videoOutputControlList ${props.compact ? "compact" : ""}`}>
      <div class="sectionHeader">
        <h3>Video Outputs</h3>
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
      <Show when={props.compact && props.outputs.length > 0}>
        <div class="deckPager">
          <strong>Output</strong>
          <span>{page() + 1} / {props.outputs.length}</span>
          <button onClick={() => setPage(Math.max(0, page() - 1))} disabled={page() === 0} aria-label="Previous video output">
            Prev
          </button>
          <button
            onClick={() => setPage(Math.min(pageCount() - 1, page() + 1))}
            disabled={page() >= pageCount() - 1}
            aria-label="Next video output"
          >
            Next
          </button>
        </div>
      </Show>
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
        <For each={visibleOutputs()}>
          {(output) => {
            const renderPlanState = () => props.renderPlanState(output);
            const outputWindowStatus = () => props.windowStatusForOutput(output.id);
            const windowState = () => props.windowState(output.id);
            const outputLive = () => renderPlanState().stateLabel === "Live" && output.opacity > 0;
            const selected = () => props.selectedOutputId === output.id;
            const selectOutput = () => props.onSelectOutput(output.id);
            return (
              <div
                class={`videoOutputControlItem ${outputLive() ? "active" : ""} ${selected() ? "selected" : ""}`}
              >
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
                <div class="videoMixerOutputDeck">
                  <label>
                    Opacity
                    <input
                      type="range"
                      min="0"
                      max="1"
                      step="0.01"
                      value={output.opacity}
                      onInput={(event) => {
                        selectOutput();
                        void props.onSetOutputOpacity(output.id, Number(event.currentTarget.value));
                      }}
                    />
                    <strong>{Math.round(output.opacity * 100)}%</strong>
                  </label>
                  <div class="buttonRow">
                    <button
                      class={selected() ? "active selected" : ""}
                      onClick={selectOutput}
                      title="Select this video output for stage mapping and live preview"
                    >
                      Sel
                    </button>
                    <button
                      class={output.enabled ? "active" : ""}
                      onClick={() => {
                        selectOutput();
                        void props.onSetOutputEnabled(output.id, !output.enabled);
                      }}
                    >
                      {output.enabled ? "On" : "Off"}
                    </button>
                    <button
                      class={output.blackout ? "active" : ""}
                      onClick={() => {
                        selectOutput();
                        void props.onSetOutputBlackout(output.id, !output.blackout);
                      }}
                    >
                      {output.blackout ? "Clear BO" : "BO"}
                    </button>
                    <button
                      onClick={() => {
                        selectOutput();
                        void props.onFadeOutputOpacity(output.id, 0);
                      }}
                    >
                      Out
                    </button>
                    <button
                      onClick={() => {
                        selectOutput();
                        void props.onFadeOutputOpacity(output.id, 1);
                      }}
                    >
                      In
                    </button>
                    <button
                      class="mixerSecondary"
                      onClick={() => {
                        selectOutput();
                        void props.onSetOutputOpacity(output.id, 1);
                      }}
                    >
                      Full
                    </button>
                    <Show when={output.kind === "Display"}>
                      <button
                        onClick={() => {
                          selectOutput();
                          void props.onOpenOutputWindow(output.id);
                        }}
                      >
                        Window
                      </button>
                      <button
                        onClick={() => {
                          selectOutput();
                          void props.onOpenOutputWindow(output.id, true);
                        }}
                      >
                        Pattern
                      </button>
                    </Show>
                  </div>
                </div>
                <div class="buttonRow videoOutputDetailActions">
                  <button
                    class={selected() ? "selected" : ""}
                    onClick={selectOutput}
                    title="Select this video output for stage mapping and live preview"
                  >
                    Select
                  </button>
                  <button
                    onClick={() => {
                      selectOutput();
                      void props.onSetOutputEnabled(output.id, !output.enabled);
                    }}
                  >
                    {output.enabled ? "Disable" : "Enable"}
                  </button>
                  <button
                    onClick={() => {
                      selectOutput();
                      void props.onSetOutputBlackout(output.id, !output.blackout);
                    }}
                  >
                    {output.blackout ? "Clear" : "Blackout"}
                  </button>
                  <button
                    onClick={() => {
                      selectOutput();
                      void props.onFadeOutputOpacity(output.id, 0);
                    }}
                  >
                    Fade Out
                  </button>
                  <button
                    onClick={() => {
                      selectOutput();
                      void props.onFadeOutputOpacity(output.id, 1);
                    }}
                  >
                    Fade In
                  </button>
                  <button
                    onClick={() => {
                      selectOutput();
                      void props.onSetOutputOpacity(output.id, 1);
                    }}
                  >
                    Full
                  </button>
                  <Show when={output.kind === "Display"}>
                    <button
                      onClick={() => {
                        selectOutput();
                        void props.onOpenOutputWindow(output.id);
                      }}
                    >
                      Window
                    </button>
                    <button
                      onClick={() => {
                        selectOutput();
                        void props.onOpenOutputWindow(output.id, true);
                      }}
                    >
                      Pattern
                    </button>
                    <button
                      onClick={() => {
                        selectOutput();
                        void props.onSyncOutputWindow(output.id);
                      }}
                    >
                      Sync
                    </button>
                    <button
                      onClick={() => {
                        selectOutput();
                        void props.onSyncOutputWindow(output.id, true);
                      }}
                    >
                      Sync Pattern
                    </button>
                    <Show when={outputWindowStatus()?.live_open}>
                      <button
                        onClick={() => {
                          selectOutput();
                          void props.onCloseOutputWindow(output.id);
                        }}
                      >
                        Close
                      </button>
                    </Show>
                    <Show when={outputWindowStatus()?.test_pattern_open}>
                      <button
                        onClick={() => {
                          selectOutput();
                          void props.onCloseOutputWindow(output.id, true);
                        }}
                      >
                        Close Pattern
                      </button>
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
