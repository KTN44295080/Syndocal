// Editor draft models and pure "from summary" builders extracted from App.tsx.
// Drafts are the editable form-state shape; builders seed a draft from an engine summary.
import type {
  AutomationInterpolation,
  CueSummary,
  CueEffectTarget,
  CueIfcbTiming,
  CuePartSummary,
  RecallMode,
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
  cue_number: string;
  label: string;
  group_id: string | null;
  recall_mode: RecallMode;
  fade_ms: number;
  authored_beats: number | null;
  pre_wait_ms: number;
  follow_ms: number | null;
  ifcb_timing: CueIfcbTiming;
  parts: CuePartSummary[];
  mark: boolean;
  mib_fixture_ids: number[];
  tracking: boolean;
  notes: string;
  effect_targets: CueEffectTarget[];
}

export interface TimelineEventDraft {
  cue_id: number;
  time_ms: number;
  time_beats: number | null;
  track: TimelineTrackKind;
  layer_id: number | null;
  duration_ms: number;
  duration_beats: number | null;
  conform_to_tempo: boolean;
  loop_fill: boolean;
  fade_in_ms: number;
  fade_out_ms: number;
  loop_count: number;
  jump_to_event_id: number | null;
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
  cue_number: cue.cue_number || String(cue.id),
  label: cue.label,
  group_id: cue.group_id ?? null,
  recall_mode: cue.recall_mode ?? "Coexist",
  fade_ms: cue.fade_ms,
  authored_beats: cue.authored_beats ?? null,
  pre_wait_ms: cue.pre_wait_ms ?? 0,
  follow_ms: cue.follow_ms ?? null,
  ifcb_timing: {
    intensity_fade_ms: cue.ifcb_timing?.intensity_fade_ms ?? null,
    intensity_delay_ms: cue.ifcb_timing?.intensity_delay_ms ?? 0,
    focus_fade_ms: cue.ifcb_timing?.focus_fade_ms ?? null,
    focus_delay_ms: cue.ifcb_timing?.focus_delay_ms ?? 0,
    color_fade_ms: cue.ifcb_timing?.color_fade_ms ?? null,
    color_delay_ms: cue.ifcb_timing?.color_delay_ms ?? 0,
    beam_fade_ms: cue.ifcb_timing?.beam_fade_ms ?? null,
    beam_delay_ms: cue.ifcb_timing?.beam_delay_ms ?? 0,
  },
  parts: (cue.parts ?? []).map((part) => ({
    ...part,
    fixture_ids: [...part.fixture_ids],
    video_layer_ids: [...(part.video_layer_ids ?? [])],
    video_output_ids: [...(part.video_output_ids ?? [])],
  })),
  mark: cue.mark ?? false,
  mib_fixture_ids: [...(cue.mib_fixture_ids ?? [])],
  tracking: cue.tracking ?? true,
  notes: cue.notes ?? "",
  effect_targets: (cue.effect_targets ?? []).map((target) => ({ ...target })),
});

export const timelineEventDraftFromSummary = (event: TimelineCueEventSummary): TimelineEventDraft => ({
  cue_id: event.cue_id,
  time_ms: event.time_ms,
  time_beats: event.time_beats ?? null,
  track: event.track,
  layer_id: event.layer_id ?? null,
  duration_ms: event.duration_ms ?? 0,
  duration_beats: event.duration_beats ?? null,
  conform_to_tempo: event.conform_to_tempo ?? false,
  loop_fill: event.loop_fill ?? false,
  fade_in_ms: event.fade_in_ms ?? 0,
  fade_out_ms: event.fade_out_ms ?? 0,
  loop_count: event.loop_count ?? 1,
  jump_to_event_id: event.jump_to_event_id ?? null,
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
