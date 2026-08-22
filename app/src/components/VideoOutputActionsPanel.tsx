import { Show } from "solid-js";
import type { VideoOutputSummary } from "../types";

type MaybePromise = void | Promise<unknown>;

type VideoOutputActionsPanelProps = {
  output: VideoOutputSummary;
  actualOpen: boolean | null;
  busy: boolean;
  blocked: boolean;
  error: string | null;
  onToggleWindow: (outputId: number, open: boolean) => MaybePromise;
};

/** The only routine action exposed for a Display output in Setup. */
export function VideoOutputActionsPanel(props: VideoOutputActionsPanelProps) {
  if (props.output.kind !== "Display") return null;
  const actionUnavailable = props.blocked || props.actualOpen === null;
  const label = props.busy
    ? props.actualOpen ? "Closing output window…" : "Opening output…"
    : props.actualOpen ? "Close output window" : "Open/Reopen output";
  return (
    <div class="videoOutputActionDock" data-action="display-window">
      <button
        type="button"
        class="primary"
        data-action="set-display-window-open"
        disabled={actionUnavailable || props.busy}
        aria-busy={props.busy}
        onClick={() => {
          if (props.actualOpen === null) return;
          void props.onToggleWindow(props.output.id, !props.actualOpen);
        }}
      >
        {label}
      </button>
      <small>
        A native Windows warning may appear when this output targets the editor display.
      </small>
      <Show when={props.error}>
        {(error) => (
          <div class="videoOutputActionError" role="alert" aria-live="assertive">
            <strong>Window action failed.</strong>{" "}
            <span data-no-localize>{error()}</span>
          </div>
        )}
      </Show>
    </div>
  );
}
