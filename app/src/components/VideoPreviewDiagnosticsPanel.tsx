import { For, Show } from "solid-js";
import type { VideoPreviewDiagnostics } from "../types";

type VideoPreviewLayerDiagnostic = VideoPreviewDiagnostics["layer_queues"][number];
type VideoPreviewOutputDecodePlan = VideoPreviewDiagnostics["output_decode_previews"][number];

interface VideoPreviewDiagnosticsPanelProps {
  layerCount: number;
  info: string;
  diagnosticsText: string;
  layerDiagnostics: VideoPreviewLayerDiagnostic[];
  outputDecodePlans: VideoPreviewOutputDecodePlan[];
  onRenderPreview: () => void | Promise<void>;
  onRefreshDiagnostics: () => void | Promise<void>;
  layerClass: (row: VideoPreviewLayerDiagnostic) => string;
  layerLabel: (row: VideoPreviewLayerDiagnostic) => string;
  outputClass: (row: VideoPreviewOutputDecodePlan) => string;
  outputLabel: (row: VideoPreviewOutputDecodePlan) => string;
}

export function VideoPreviewDiagnosticsPanel(props: VideoPreviewDiagnosticsPanelProps) {
  return (
    <div class="previewDebug">
      <button onClick={() => void props.onRenderPreview()} disabled={props.layerCount === 0}>
        Preview (Reference)
      </button>
      <button onClick={() => void props.onRefreshDiagnostics()}>Preview Status</button>
      <span>{props.info}</span>
      <small>{props.diagnosticsText}</small>
      <Show when={props.layerDiagnostics.length > 0}>
        <div class="previewDiagnosticList">
          <For each={props.layerDiagnostics}>
            {(row) => (
              <div class={props.layerClass(row)}>
                <strong>{row.ready ? "Ready" : row.queue_len > 0 ? "Warm" : "Wait"}</strong>
                <span data-no-localize>{row.label}</span>
                <small title={props.layerLabel(row)}>{props.layerLabel(row)}</small>
              </div>
            )}
          </For>
        </div>
      </Show>
      <Show when={props.outputDecodePlans.length > 0}>
        <div class="previewDiagnosticList">
          <For each={props.outputDecodePlans}>
            {(row) => (
              <div class={props.outputClass(row)}>
                <strong>{row.error ? "Error" : row.report?.rejected_full ? "Full" : "Plan"}</strong>
                <span data-no-localize>{row.label}</span>
                <small title={props.outputLabel(row)}>{props.outputLabel(row)}</small>
              </div>
            )}
          </For>
        </div>
      </Show>
    </div>
  );
}
