// Editor draft models and pure "from summary" builders extracted from App.tsx.
// Drafts are the editable form-state shape; builders seed a draft from an engine summary.
import type {
  AutomationInterpolation,
  CueSummary,
  TimelineAutomationSummary,
  TimelineCueEventSummary,
  TimelineTrackKind,
  TimelineVideoAutomationSummary,
  VideoOutputKind,
  VideoOutputSummary,
  VideoParam,
} from "./types";

export interface VideoOutputConfigDraft {
  label: string;
  kind: VideoOutputKind;
  width: number;
  height: number;
  fullscreen: boolean;
  monitor_id: number;
  endpoint_name: string;
}

export interface CueMetadataDraft {
  label: string;
  fade_ms: number;
}

export interface TimelineEventDraft {
  cue_id: number;
  time_ms: number;
  track: TimelineTrackKind;
}

export interface TimelineAutomationDraft {
  fixture_id: number;
  attribute: string;
  start_ms: number;
  end_ms: number;
  start_value: number;
  end_value: number;
  interpolation: AutomationInterpolation;
}

export interface TimelineVideoAutomationDraft {
  layer_id: number;
  param: VideoParam;
  start_ms: number;
  end_ms: number;
  start_value: number;
  end_value: number;
  interpolation: AutomationInterpolation;
}

export const videoOutputConfigDraftFromSummary = (output: VideoOutputSummary): VideoOutputConfigDraft => ({
  label: output.label,
  kind: output.kind,
  width: output.width,
  height: output.height,
  fullscreen: output.fullscreen,
  monitor_id: output.monitor_id ?? 0,
  endpoint_name: output.endpoint_name ?? "",
});

export const cueMetadataDraftFromSummary = (cue: CueSummary): CueMetadataDraft => ({
  label: cue.label,
  fade_ms: cue.fade_ms,
});

export const timelineEventDraftFromSummary = (event: TimelineCueEventSummary): TimelineEventDraft => ({
  cue_id: event.cue_id,
  time_ms: event.time_ms,
  track: event.track,
});

export const timelineAutomationDraftFromSummary = (automation: TimelineAutomationSummary): TimelineAutomationDraft => {
  const first = automation.keyframes[0];
  const last = automation.keyframes[automation.keyframes.length - 1] ?? first;
  return {
    fixture_id: automation.fixture_id,
    attribute: automation.attribute,
    start_ms: first?.time_ms ?? 0,
    end_ms: last?.time_ms ?? first?.time_ms ?? 0,
    start_value: first?.value ?? 0,
    end_value: last?.value ?? first?.value ?? 0,
    interpolation: first?.interpolation ?? "Linear",
  };
};

export const timelineVideoAutomationDraftFromSummary = (
  automation: TimelineVideoAutomationSummary,
): TimelineVideoAutomationDraft => {
  const first = automation.keyframes[0];
  const last = automation.keyframes[automation.keyframes.length - 1] ?? first;
  return {
    layer_id: automation.layer_id,
    param: automation.param,
    start_ms: first?.time_ms ?? 0,
    end_ms: last?.time_ms ?? first?.time_ms ?? 0,
    start_value: first?.value ?? 0,
    end_value: last?.value ?? first?.value ?? 0,
    interpolation: first?.interpolation ?? "Linear",
  };
};
