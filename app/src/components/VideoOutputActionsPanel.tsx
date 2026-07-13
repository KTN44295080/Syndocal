import { Show } from "solid-js";
import type { VideoOutputSummary } from "../types";

type MaybePromise = void | Promise<unknown>;

type VideoOutputActionsPanelProps = {
  output: VideoOutputSummary;
  onSetEnabled: (outputId: number, enabled: boolean) => MaybePromise;
  onSetBlackout: (outputId: number, blackout: boolean) => MaybePromise;
  onSetOpacity: (outputId: number, opacity: number) => MaybePromise;
  onFadeOpacity: (outputId: number, opacity: number) => MaybePromise;
  onPreview: (outputId: number, testPattern?: boolean) => MaybePromise;
  onOpenWindow: (outputId: number, testPattern?: boolean) => MaybePromise;
  onSyncWindow: (outputId: number, testPattern?: boolean) => MaybePromise;
  onRemove: (outputId: number) => MaybePromise;
};

export function VideoOutputActionsPanel(props: VideoOutputActionsPanelProps) {
  return (
    <div class="buttonRow videoOutputActionDock">
      <button
        data-action="toggle-enabled"
        aria-pressed={props.output.enabled}
        onClick={() => void props.onSetEnabled(props.output.id, !props.output.enabled)}
      >
        {props.output.enabled ? "Disable" : "Enable"}
      </button>
      <button
        data-action="toggle-blackout"
        aria-pressed={props.output.blackout}
        onClick={() => void props.onSetBlackout(props.output.id, !props.output.blackout)}
      >
        {props.output.blackout ? "Clear" : "Blackout"}
      </button>
      <button data-action="toggle-opacity" onClick={() => void props.onSetOpacity(props.output.id, props.output.opacity >= 1 ? 0.5 : 1)}>
        {props.output.opacity >= 1 ? "Half" : "Full"}
      </button>
      <button data-action="fade-out" onClick={() => void props.onFadeOpacity(props.output.id, 0)}>
        Fade Out
      </button>
      <button data-action="fade-in" onClick={() => void props.onFadeOpacity(props.output.id, 1)}>
        Fade In
      </button>
      <button data-action="preview" onClick={() => void props.onPreview(props.output.id)}>
        Preview
      </button>
      <button data-action="preview-pattern" onClick={() => void props.onPreview(props.output.id, true)}>
        Pattern Preview
      </button>
      <Show when={props.output.kind === "Display"}>
        <button data-action="open-window" onClick={() => void props.onOpenWindow(props.output.id)}>
          Open Window
        </button>
        <button data-action="open-test-pattern" onClick={() => void props.onOpenWindow(props.output.id, true)}>
          Test Pattern
        </button>
        <button data-action="sync-window" onClick={() => void props.onSyncWindow(props.output.id)}>
          Sync Window
        </button>
        <button data-action="sync-pattern" onClick={() => void props.onSyncWindow(props.output.id, true)}>
          Sync Pattern
        </button>
      </Show>
      <button data-action="remove-output" class="danger" onClick={() => void props.onRemove(props.output.id)}>Remove</button>
    </div>
  );
}
