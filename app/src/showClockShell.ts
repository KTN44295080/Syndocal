import type { AppStatusTone } from "./statusModel";
import type { ShowClockIpcStatus } from "./types";

export type ShowClockShellState = "unchecked" | "stopped" | "acquiring" | "locked" | "hold" | "stale" | "fault";

export interface ShowClockShellSummary {
  state: ShowClockShellState;
  label: string;
  detail: string;
  tone: AppStatusTone;
}

const phaseState = (status: ShowClockIpcStatus): ShowClockShellState => {
  if (!status.running || status.state === "STOPPED") return "stopped";
  switch (status.state) {
    case "ACQUIRING": return "acquiring";
    case "LOCKED": return "locked";
    case "HOLD": return "hold";
    case "STALE": return "stale";
    case "FAULT": return "fault";
    default: return "fault";
  }
};

const phaseLabel = (state: ShowClockShellState): string => {
  switch (state) {
    case "unchecked": return "Unchecked";
    case "stopped": return "Stopped";
    case "acquiring": return "Acquiring";
    case "locked": return "Locked";
    case "hold": return "Hold";
    case "stale": return "Stale";
    case "fault": return "Fault";
  }
};

export const summarizeShowClockForShell = (
  status: ShowClockIpcStatus | null,
  error: string | null = null,
): ShowClockShellSummary => {
  if (error) {
    return {
      state: "fault",
      label: "Unavailable",
      detail: `Show Clock status is unavailable: ${error}`,
      tone: "error",
    };
  }
  if (!status) {
    return {
      state: "unchecked",
      label: "Unchecked",
      detail: "Show Clock status has not been checked in this window.",
      tone: "warning",
    };
  }
  const state = phaseState(status);
  const output = status.output_armed ? "Armed" : "Fenced";
  const detail = status.last_error
    ? `Show Clock ${status.state}: ${status.last_error}`
    : `Show Clock ${status.state} · ${output} · ${status.role ?? "unassigned"}`;
  return {
    state,
    label: `${phaseLabel(state)} / ${output}`,
    detail,
    tone: state === "fault" || state === "stale" ? "error" : state === "locked" ? "success" : "warning",
  };
};
