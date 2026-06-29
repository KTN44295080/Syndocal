import { For, Show } from "solid-js";
import type { Phase1SmokeReport } from "../types";

interface Phase1SmokeReportPanelProps {
  report: Phase1SmokeReport | null;
}

const hexByte = (value: number) => value.toString(16).padStart(2, "0").toUpperCase();

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
        fallback={<small class="phase1SmokeHint">Expected path: load mini show, trigger first cue, verify U0 A1-A8.</small>}
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
