import { For, Show } from "solid-js";
import { createTimelineOutputMonitorController } from "../createTimelineOutputMonitorController";
import type { FrontendTauriInvoke } from "../tauriInvokeCommands";
import type { VideoOutputSummary } from "../types";
import "./TimelineOutputPreview.css";

export interface TimelineOutputPreviewProps {
  outputs: readonly VideoOutputSummary[];
  invoke: FrontendTauriInvoke;
  backendAvailable: boolean;
}

export function TimelineOutputPreview(props: TimelineOutputPreviewProps) {
  const monitor = createTimelineOutputMonitorController({
    invoke: props.invoke,
    outputs: () => props.outputs,
    backendAvailable: () => props.backendAvailable,
    active: () => true,
  });
  return (
    <div class="timelineOutputPreview" data-timeline-output-preview>
      <Show when={props.outputs.length > 0} fallback={<p>No video outputs configured.</p>}>
        <div class="timelineOutputPreviewGrid">
          <For each={props.outputs.map((output) => output.id)}>
            {(id) => {
              const output = () => props.outputs.find((candidate) => candidate.id === id)!;
              const state = () => monitor.states().get(id);
              const validSize = () => Number.isFinite(output().width) && Number.isFinite(output().height)
                && output().width > 0 && output().height > 0;
              return (
                <section class="timelineOutputPreviewCard" data-timeline-output-id={id}>
                  <header>
                    <strong data-no-localize>{output().label}</strong>
                    <span data-no-localize>{output().width} × {output().height}</span>
                  </header>
                  <Show when={validSize()} fallback={<p>Invalid output resolution.</p>}>
                    <div class="timelineOutputPreviewViewport" style={{ "aspect-ratio": `${output().width} / ${output().height}`, width: `min(100%, ${180 * output().width / output().height}px)` }}>
                      <Show when={state()?.frameUrl} fallback={<span>{props.backendAvailable ? "Waiting for output frame." : "Open the desktop app for live video."}</span>}>
                        {(url) => <img src={url()} alt="Video output preview" draggable={false} />}
                      </Show>
                    </div>
                  </Show>
                  <Show when={!output().enabled}><span>Output disabled</span></Show>
                  <Show when={output().blackout}><span>Blackout</span></Show>
                  <Show when={state()?.error}>
                    {(error) => <p class="timelineOutputPreviewError" role="status" data-no-localize>{error()}</p>}
                  </Show>
                </section>
              );
            }}
          </For>
        </div>
      </Show>
    </div>
  );
}
