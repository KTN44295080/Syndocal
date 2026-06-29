import { Show } from "solid-js";
import type { VideoOutputKind } from "../types";

type MaybePromise = void | Promise<unknown>;

type VideoOutputCreatePanelProps = {
  label: string;
  kind: VideoOutputKind;
  width: number;
  height: number;
  fadeMs: number;
  monitorId: number;
  fullscreen: boolean;
  endpoint: string;
  onLabel: (value: string) => void;
  onKind: (value: VideoOutputKind) => void;
  onWidth: (value: number) => void;
  onHeight: (value: number) => void;
  onFadeMs: (value: number) => void;
  onMonitorId: (value: number) => void;
  onFullscreen: (value: boolean) => void;
  onEndpoint: (value: string) => void;
  onAddOutput: () => MaybePromise;
};

export function VideoOutputCreatePanel(props: VideoOutputCreatePanelProps) {
  return (
    <div class="videoOutputForm">
      <h3>Composition Output</h3>
      <div class="split">
        <label>
          Output label
          <input value={props.label} onInput={(event) => props.onLabel(event.currentTarget.value)} />
        </label>
        <label>
          Kind
          <select value={props.kind} onInput={(event) => props.onKind(event.currentTarget.value as VideoOutputKind)}>
            <option value="Display">Display</option>
            <option value="NdiSender">NDI Sender</option>
            <option value="SpoutSender">Spout Sender</option>
            <option value="SyphonServer">Syphon Server</option>
          </select>
        </label>
      </div>
      <div class="split">
        <label>
          Width
          <input type="number" min="1" value={props.width} onInput={(event) => props.onWidth(Number(event.currentTarget.value))} />
        </label>
        <label>
          Height
          <input type="number" min="1" value={props.height} onInput={(event) => props.onHeight(Number(event.currentTarget.value))} />
        </label>
      </div>
      <label>
        Output Fade ms
        <input
          type="number"
          min="0"
          step="10"
          value={props.fadeMs}
          onInput={(event) => props.onFadeMs(Number(event.currentTarget.value))}
        />
      </label>
      <Show when={props.kind === "Display"}>
        <div class="split">
          <label>
            Monitor
            <input type="number" min="0" value={props.monitorId} onInput={(event) => props.onMonitorId(Number(event.currentTarget.value))} />
          </label>
          <label class="checkbox inlineCheckbox">
            <input type="checkbox" checked={props.fullscreen} onChange={(event) => props.onFullscreen(event.currentTarget.checked)} />
            Fullscreen
          </label>
        </div>
      </Show>
      <Show when={props.kind !== "Display"}>
        <label>
          Endpoint
          <input value={props.endpoint} onInput={(event) => props.onEndpoint(event.currentTarget.value)} />
        </label>
      </Show>
      <button class="primary" onClick={() => void props.onAddOutput()}>
        Add Output
      </button>
    </div>
  );
}
