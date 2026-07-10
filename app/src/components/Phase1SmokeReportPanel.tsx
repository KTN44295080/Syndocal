import { For, Show } from "solid-js";
import type { Phase1SmokeReport } from "../types";

interface Phase1SmokeReportPanelProps {
  report: Phase1SmokeReport | null;
}

const hexByte = (value: number) => value.toString(16).padStart(2, "0").toUpperCase();
const percent = (value: number | null | undefined) =>
  value === null || value === undefined ? "n/a" : `${Math.round(value * 100)}%`;

export function Phase1SmokeReportPanel(props: Phase1SmokeReportPanelProps) {
  return (
    <div class="phase1SmokePanel">
      <div class="dmxOutputDiagnosticsHeader">
        <div>
          <h3>Phase 1 Smoke</h3>
          <span>{props.report ? props.report.path : "Run Smoke from the top bar"}</span>
        </div>
        <span class={props.report?.passed ? "healthBadge ok" : props.report ? "healthBadge bad" : "healthBadge idle"}>
          <b>{props.report?.passed ? "Pass" : props.report ? "Fail" : "Idle"}</b>
          <small>Mini show</small>
        </span>
      </div>
      <Show
        when={props.report}
        fallback={
          <small class="phase1SmokeHint">
            Expected path: load mini show, trigger first cue, verify U0 A1-A8, video layer, and shared timeline probe.
          </small>
        }
      >
        {(report) => (
          <>
            <div class="phase1SmokeSummary">
              <span>
                <small>Cue</small>
                <strong>{report().cue_label}</strong>
              </span>
              <span>
                <small>Active</small>
                <strong>{report().active_cue_id ?? "none"}</strong>
              </span>
              <span>
                <small>Non-zero</small>
                <strong>{report().non_zero_first_8}/8</strong>
              </span>
              <span>
                <small>Timeline</small>
                <strong>{report().timeline_event_count}e/{report().timeline_automation_count}+{report().timeline_video_automation_count}</strong>
              </span>
              <span>
                <small>Video</small>
                <strong>{report().video_layer_playing ? "Playing" : "Stopped"}</strong>
              </span>
              <span>
                <small>Probe {report().timeline_probe_ms}ms</small>
                <strong>D{report().timeline_probe_dimmer_byte} / V{percent(report().timeline_probe_video_opacity)}</strong>
              </span>
            </div>
            <div class="phase1SmokeTimeline">
              <span>
                <small>DMX output</small>
                <strong>{report().enabled_dmx_output_count}/{report().dmx_output_count} enabled</strong>
              </span>
              <span>
                <small>Route</small>
                <strong title={report().primary_output_label}>{report().primary_output_label}</strong>
              </span>
              <span>
                <small>Preview</small>
                <strong>{report().dmx_preview_universe_count} universe(s)</strong>
              </span>
              <span>
                <small>Duration</small>
                <strong>{report().timeline_duration_ms}ms</strong>
              </span>
              <span>
                <small>Video Layer</small>
                <strong>{report().video_layer_label ?? `${report().video_layer_count} layer(s)`}</strong>
              </span>
              <span>
                <small>Cue opacity</small>
                <strong>{percent(report().video_layer_opacity)}</strong>
              </span>
            </div>
            <div class="phase1SmokeBytes" aria-label="Phase 1 smoke DMX bytes">
              <For each={report().first_8}>
                {(value, index) => {
                  const expected = () => report().expected_first_8[index()] ?? 0;
                  const matched = () => value === expected();
                  return (
                    <span class={matched() ? "ok" : "bad"} title={`Expected ${expected()}`}>
                      <small>A{index() + 1}</small>
                      <strong>{hexByte(value)}</strong>
                      <em>{value}</em>
                    </span>
                  );
                }}
              </For>
            </div>
          </>
        )}
      </Show>
    </div>
  );
}
