import { For, Show } from "solid-js";
import { outputMissedWarningThreshold, type LiveVideoOutputHealth } from "../liveVideoMonitorPolicy";
import { liveVideoPerformanceBudget } from "../liveVideoPerformanceBudget";

/** Numbers are completed presents/sends and missed cadence slots, not scanout or receiver proof.
 * "Missed" is the recent 10 s window; the cumulative total is in the tooltip. */
const recentMissed = (output: LiveVideoOutputHealth) => output.recentMissedFrames ?? 0;
const compactWarning = (warning: string) => warning === "Frame preparation over budget" ? "Over budget"
  : warning === "Output missing frames" ? "Frames missed" : "Stalled";
export function LiveVideoOutputHealthReadout(props: {
  outputs?: readonly LiveVideoOutputHealth[];
  error?: string | null;
}) {
  return <div class="liveVideoOutputHealth" aria-label="Real output health" data-live-video-output-health>
    <Show when={props.error}><span class="liveVideoOutputHealthWarning">Output health unavailable</span></Show>
    <For each={props.outputs ?? []}>{(output) => {
      const performance = () => liveVideoPerformanceBudget(output);
      return <span
      class="liveVideoOutputHealthItem tabularNums"
      classList={{ stale: output.lastFrameAgeMs !== null && output.lastFrameAgeMs > 1_000 }}
      title={output.measurementBoundary === "present"
        ? "Display presents completed; drops estimate missed output intervals. Physical scanout is not measured."
        : "Sender accepted frames; drops estimate missed output intervals. Receiver delivery is not measured."}
      data-output-health-id={`${output.kind}:${output.outputId}`}
    >
      <span class="liveVideoOutputHealthCadence">
      <span data-no-localize>{output.kind === "display" ? "Display" : output.kind === "spout" ? "Spout" : output.kind === "syphon" ? "Syphon" : "NDI"} #{output.outputId}</span>
      <span>Output</span>
      <span data-no-localize>{output.sampleCount >= 2 && (output.lastFrameAgeMs ?? 0) < 1_000 ? output.deliveredFps.toFixed(1) : "—"}fps</span>
      <span title={`Missed output intervals in the last 10 s: ${recentMissed(output)}; total since output start: ${output.droppedFrames}`}>Missed (10 s)</span>
      <span
        class="liveVideoOutputHealthMissed"
        classList={{ idle: recentMissed(output) === 0, warning: recentMissed(output) > outputMissedWarningThreshold }}
        data-output-health-missed={recentMissed(output)}
        data-output-health-missed-total={output.droppedFrames}
        data-no-localize
      >{recentMissed(output)}</span>
      </span>
      <span class="liveVideoPerformanceRow">
      <span
        class="liveVideoPerformanceBudget"
        classList={{ warning: performance().warning !== null }}
        data-output-performance-budget
        title="Prepare p95 is the 95th percentile of up to 600 recent successful frame preparations, including decode. Display GPU composition and presentation, output transport, and pacing sleep are excluded. Budget is one frame at the target rate."
      >
        <span>Prepare p95</span>
        <span data-no-localize>{performance().renderP95Ms?.toFixed(1) ?? "—"} / {performance().budgetMs?.toFixed(1) ?? "—"} ms</span>
      </span>
      <Show when={performance().warning}>{(warning) => <span class="liveVideoPerformanceWarning" title={warning()} aria-label={warning()}>{compactWarning(warning())}</span>}</Show>
      </span>
    </span>;
    }}</For>
  </div>;
}
