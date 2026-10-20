import { Show } from "solid-js";
import type { VideoOutputConfigDraft } from "../editorDrafts";
import type { VideoOutputKind, VideoOutputSummary } from "../types";

type MaybePromise = void | Promise<unknown>;

type VideoOutputConfigPanelProps = {
  output: VideoOutputSummary;
  draft: VideoOutputConfigDraft;
  syphonAvailable?: boolean;
  onDraft: (output: VideoOutputSummary, patch: Partial<VideoOutputConfigDraft>) => void;
  onApply: (output: VideoOutputSummary) => MaybePromise;
};

export function VideoOutputConfigPanel(props: VideoOutputConfigPanelProps) {
  return (
    <div class="videoOutputConfig">
      <h3>Video Output Config</h3>
      <div class="split">
        <label>
          Label
          <input
            value={props.draft.label}
            onInput={(event) => props.onDraft(props.output, { label: event.currentTarget.value })}
          />
        </label>
        <label>
          Kind
          <select
            value={props.draft.kind}
            onInput={(event) =>
              props.onDraft(props.output, {
                kind: event.currentTarget.value as VideoOutputKind,
              })
            }
          >
            <option value="Display">Display</option>
            <option value="NdiSender">NDI Sender</option>
            <option value="SpoutSender">Spout Sender</option>
            <Show when={props.syphonAvailable} fallback={
              <Show when={props.draft.kind === "SyphonServer"}>
                <option value="SyphonServer" disabled>Syphon Server (macOS only)</option>
              </Show>
            }>
              <option value="SyphonServer">Syphon Server</option>
            </Show>
          </select>
        </label>
      </div>
      <div class="split">
        <label>
          Width
          <input
            type="number"
            min="1"
            value={props.draft.width}
            onInput={(event) => props.onDraft(props.output, { width: Number(event.currentTarget.value) })}
          />
        </label>
        <label>
          Height
          <input
            type="number"
            min="1"
            value={props.draft.height}
            onInput={(event) => props.onDraft(props.output, { height: Number(event.currentTarget.value) })}
          />
        </label>
      </div>
      <Show when={props.draft.kind === "Display"}>
        <div class="split">
          <label>
            Monitor
            <input
              type="number"
              min="0"
              value={props.draft.monitor_id}
              onInput={(event) => props.onDraft(props.output, { monitor_id: Number(event.currentTarget.value) })}
            />
          </label>
          <label class="checkbox inlineCheckbox">
            <input
              type="checkbox"
              checked={props.draft.fullscreen}
              onChange={(event) => props.onDraft(props.output, { fullscreen: event.currentTarget.checked })}
            />
            Fullscreen
          </label>
        </div>
      </Show>
      <Show when={props.draft.kind === "SyphonServer"}>
        <p class={props.syphonAvailable ? "hint" : "inlineWarning"} role="status" data-syphon-output-help>
          {props.syphonAvailable
            ? "Publishes video to Syphon clients on this Mac. Enable Output to start; Standby stops transmission."
            : "Syphon output is available on macOS only. This saved output is preserved; choose another type to use it on this computer."}
        </p>
      </Show>
      <Show when={props.draft.kind !== "Display"}>
        <label>
          {props.draft.kind === "SyphonServer" ? "Server name" : "Endpoint"}
          <input
            value={props.draft.endpoint_name}
            onInput={(event) => props.onDraft(props.output, { endpoint_name: event.currentTarget.value })}
          />
        </label>
      </Show>
      <div class="buttonRow">
        <button
          class="primary"
          disabled={props.draft.kind === "SyphonServer" && !props.syphonAvailable}
          onClick={() => void props.onApply(props.output)}
        >
          Apply Output
        </button>
      </div>
    </div>
  );
}
