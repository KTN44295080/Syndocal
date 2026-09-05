import { createEffect, createMemo, For, Show } from "solid-js";
import { createTimelineOutputMonitorController, type TimelineOutputMonitorFrame } from "../createTimelineOutputMonitorController";
import type { FrontendTauriInvoke } from "../tauriInvokeCommands";
import type { VideoOutputSummary } from "../types";
import "./TimelineOutputPreview.css";

export interface TimelineOutputPreviewProps {
  outputs: readonly VideoOutputSummary[];
  invoke: FrontendTauriInvoke;
  backendAvailable: boolean;
  projectEpoch: number;
}

/** The canvas remains mounted across frames; clearing a retired frame is
 * synchronous and does not wait for browser image decoding or an object URL. */
export function TimelineOutputPreviewCanvas(props: { frame: TimelineOutputMonitorFrame | null }) {
  let canvas!: HTMLCanvasElement;
  createEffect(() => {
    const frame = props.frame;
    const context = canvas.getContext("2d");
    if (!context) throw new Error("Output preview requires a 2D canvas context.");
    if (!frame) {
      context.clearRect(0, 0, canvas.width, canvas.height);
      return;
    }
    if (canvas.width !== frame.width) canvas.width = frame.width;
    if (canvas.height !== frame.height) canvas.height = frame.height;
    context.putImageData(new ImageData(frame.data, frame.width, frame.height), 0, 0);
  });
  return <canvas ref={canvas} role="img" aria-label="Video output preview"
    style={{ visibility: props.frame ? "visible" : "hidden" }} />;
}

export function TimelineOutputPreview(props: TimelineOutputPreviewProps) {
  const monitor = createTimelineOutputMonitorController({
    invoke: props.invoke,
    outputs: () => props.outputs,
    backendAvailable: () => props.backendAvailable,
    projectEpoch: () => props.projectEpoch,
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
              const frame = createMemo(() => state()?.frame ?? null);
              const validSize = () => Number.isFinite(output().width) && Number.isFinite(output().height)
                && output().width > 0 && output().height > 0;
              return (
                <section class="timelineOutputPreviewCard" data-timeline-output-id={id}>
                  <header>
                    <strong data-no-localize>{output().label}</strong>
                    <span data-no-localize>{output().width} × {output().height}</span>
                  </header>
                  <Show when={validSize()} fallback={<p>Invalid output resolution.</p>}>
                    <div class="timelineOutputPreviewFrameArea">
                      <div class="timelineOutputPreviewViewport" style={{ "aspect-ratio": `${output().width} / ${output().height}`, "--output-aspect": output().width / output().height }}>
                        <TimelineOutputPreviewCanvas frame={frame()} />
                        <Show when={!frame()}><span>{props.backendAvailable ? "Waiting for output frame." : "Open the desktop app for live video."}</span></Show>
                      </div>
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
