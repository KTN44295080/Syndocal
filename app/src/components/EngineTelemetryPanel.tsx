import { For, Show } from "solid-js";
import type { EngineSnapshot, EngineTelemetryBudgetReport, TelemetryBudgetStatus } from "../types";

type EngineTelemetry = EngineSnapshot["telemetry"];

interface EngineTelemetryPanelProps {
  telemetry: EngineTelemetry;
  budget?: EngineTelemetryBudgetReport | null;
  onReset: () => void | Promise<void>;
  onSaveReport: () => void | Promise<void>;
}

const metric = (label: string, value: string | number) => ({ label, value });

const budgetStatusLabel = (status: TelemetryBudgetStatus) =>
  status === "InsufficientSamples" ? "Sampling" : status;

const budgetStatusClass = (status: TelemetryBudgetStatus) => {
  switch (status) {
    case "Pass":
      return "ok";
    case "Warn":
      return "warn";
    case "Fail":
      return "bad";
    default:
      return "idle";
  }
};

export function EngineTelemetryPanel(props: EngineTelemetryPanelProps) {
  const metrics = () => [
    metric("Frame", props.telemetry.frame_counter),
    metric("Queue", `${props.telemetry.queue_depth} / max ${props.telemetry.queue_depth_abs_max}`),
    metric("Queue drops", props.telemetry.queue_push_failure_count),
    metric("Tick", `${Math.round(props.telemetry.last_tick_interval_us / 1000)} ms`),
    metric("Jitter last", `${props.telemetry.tick_jitter_last_us} us`),
    metric("Jitter p95", `${props.telemetry.tick_jitter_p95_us} us`),
    metric("Jitter p99", `${props.telemetry.tick_jitter_p99_us} us`),
    metric("Jitter max", `${props.telemetry.tick_jitter_abs_max_us} us`),
    metric("Cmd queue p95", `${props.telemetry.command_queue_latency_p95_us} us`),
    metric("Cmd queue p99", `${props.telemetry.command_queue_latency_p99_us} us`),
    metric("Cmd-DMX p95", `${props.telemetry.command_to_dmx_tick_latency_p95_us} us`),
    metric("Cmd-DMX p99", `${props.telemetry.command_to_dmx_tick_latency_p99_us} us`),
    metric("Drain", `${props.telemetry.last_command_drain_count} / max ${props.telemetry.command_drain_abs_max}`),
    metric("Drain limit", props.telemetry.command_drain_limit_hit_count),
    metric("DMX interval", `${props.telemetry.last_dmx_send_interval_us} us`),
    metric("DMX interval max", `${props.telemetry.dmx_send_interval_max_us} us`),
    metric("DMX out", `${props.telemetry.last_dmx_send_success_count}/${props.telemetry.last_dmx_output_count}`),
    metric("DMX fail", `${props.telemetry.last_dmx_send_failure_count} / total ${props.telemetry.total_dmx_send_failure_count}`),
    metric("Packet", `${props.telemetry.last_packet_bytes} bytes`),
    metric(
      "Low latency",
      `${props.telemetry.low_latency_dmx_tick_advance_count}/${props.telemetry.low_latency_dmx_tick_request_count}`,
    ),
  ];

  return (
    <div class="telemetry">
      <div class="dmxOutputDiagnosticsHeader">
        <div>
          <h3>Engine Telemetry</h3>
          <span>{props.telemetry.tick_jitter_samples} jitter sample(s)</span>
        </div>
        <div class="buttonRow">
          <button onClick={props.onReset}>Reset</button>
          <button onClick={props.onSaveReport}>Save Report</button>
        </div>
      </div>
      <Show when={props.budget}>
        {(budget) => (
          <>
            <div class="telemetryHealthRow">
              <span class={`healthBadge ${budgetStatusClass(budget().overall)}`}>
                <b>{budgetStatusLabel(budget().overall)}</b>
                <small>Overall budget</small>
              </span>
              <span class="healthBadge idle">
                <b>{budget().target_dmx_frame_rate_hz}Hz</b>
                <small>DMX target</small>
              </span>
            </div>
            <div class="timelineList">
              <For each={budget().checks}>
                {(check) => (
                  <div class="timelineItem">
                    <strong>{check.name}</strong>
                    <span class={`healthBadge ${budgetStatusClass(check.status)}`}>
                      {budgetStatusLabel(check.status)}
                    </span>
                    <small>{check.measured_us ?? "-"}us</small>
                    <small>{check.target_us ?? "-"}us target</small>
                    <small>{check.detail}</small>
                  </div>
                )}
              </For>
            </div>
          </>
        )}
      </Show>
      <div class="telemetryGrid">
        <For each={metrics()}>
          {(entry) => (
            <span>
              <small>{entry.label}</small>
              <strong>{entry.value}</strong>
            </span>
          )}
        </For>
      </div>
      <Show when={props.telemetry.last_dmx_route_results.length > 0}>
        <div class="timelineList">
          <For each={props.telemetry.last_dmx_route_results}>
            {(route) => (
              <div class="timelineItem">
                <strong>Route {route.index}</strong>
                <span>{route.attempted ? (route.success ? "OK" : "Failed") : "Idle"}</span>
                <small>U{route.universe}</small>
                <small>{route.bytes} bytes</small>
                <Show when={route.error}>
                  {(error) => <small class="telemetryError">{error()}</small>}
                </Show>
              </div>
            )}
          </For>
        </div>
      </Show>
      <Show when={props.telemetry.last_error}>
        {(error) => <strong class="telemetryError">{error()}</strong>}
      </Show>
    </div>
  );
}
