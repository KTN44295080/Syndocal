import type { Accessor, Setter } from "solid-js";
import {
  timelineAutomationDraftFromSummary,
  timelineVideoAutomationDraftFromSummary,
  type TimelineAutomationDraft,
  type TimelineVideoAutomationDraft,
} from "./editorDrafts";
import { clampDmxValue } from "./numericHelpers";
import {
  draftRangeFromKeyframes,
  evaluateTimelineKeyframes,
  interpolationForInsertedKeyframe,
  keyframesWithInterpolationAtIndex,
  keyframesWithValueAtIndex,
  keyframesWithoutIndex,
  keyframesWithoutPlayheadKeyframe,
  upsertTimelineKeyframe,
} from "./timelineAutomationHelpers";
import type {
  AutomationInterpolation,
  AutomationKeyframeSummary,
  EngineSnapshot,
  TimelineAutomationSummary,
  TimelineVideoAutomationSummary,
  VideoAutomationKeyframeSummary,
} from "./types";

type Invoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

interface TimelineKeyframeControllerOptions {
  snapshot: Accessor<EngineSnapshot>;
  snapTimeMs: (timeMs: number) => number;
  timelinePlacementNudgeMs: Accessor<number>;
  setAutomationStartMs: Setter<number>;
  setAutomationEndMs: Setter<number>;
  setVideoAutomationStartMs: Setter<number>;
  setVideoAutomationEndMs: Setter<number>;
  timelineAutomationDraft: (automation: TimelineAutomationSummary) => TimelineAutomationDraft;
  timelineVideoAutomationDraft: (automation: TimelineVideoAutomationSummary) => TimelineVideoAutomationDraft;
  updateTimelineAutomationDraft: (
    automation: TimelineAutomationSummary,
    updates: Partial<TimelineAutomationDraft>,
  ) => void;
  updateTimelineVideoAutomationDraft: (
    automation: TimelineVideoAutomationSummary,
    updates: Partial<TimelineVideoAutomationDraft>,
  ) => void;
  currentLightingAutomationValue: (automation: TimelineAutomationSummary) => number | null;
  currentVideoAutomationValue: (automation: TimelineVideoAutomationSummary) => number | null;
  keyframeDeleteToleranceMs: Accessor<number>;
  invoke: Invoke;
  setTimelineAutomationDrafts: Setter<Record<number, TimelineAutomationDraft>>;
  setTimelineVideoAutomationDrafts: Setter<Record<number, TimelineVideoAutomationDraft>>;
  setMessage: (message: string) => unknown;
  refreshSnapshot: () => Promise<EngineSnapshot | null>;
}

export function createTimelineKeyframeController(options: TimelineKeyframeControllerOptions) {
  const automationRangeFromPlayhead = () => {
    const startMs = options.snapTimeMs(options.snapshot().timeline.position_ms);
    const endMs = Math.max(startMs, options.snapTimeMs(startMs + options.timelinePlacementNudgeMs()));
    return { startMs, endMs };
  };

  const usePlayheadForLightingAutomation = () => {
    const { startMs, endMs } = automationRangeFromPlayhead();
    options.setAutomationStartMs(startMs);
    options.setAutomationEndMs(endMs);
    options.setMessage(`Lighting automation draft ${startMs}-${endMs} ms`);
  };

  const usePlayheadForVideoAutomation = () => {
    const { startMs, endMs } = automationRangeFromPlayhead();
    options.setVideoAutomationStartMs(startMs);
    options.setVideoAutomationEndMs(endMs);
    options.setMessage(`Video automation draft ${startMs}-${endMs} ms`);
  };

  const automationDraftRangeAtPlayhead = (startMs: number, endMs: number) => {
    const nextStartMs = options.snapTimeMs(options.snapshot().timeline.position_ms);
    const sourceDurationMs = Math.max(0, Math.round(endMs - startMs));
    const durationMs = sourceDurationMs > 0 ? sourceDurationMs : Math.max(1, options.timelinePlacementNudgeMs());
    let nextEndMs = options.snapTimeMs(nextStartMs + durationMs);
    if (nextEndMs <= nextStartMs) nextEndMs = nextStartMs + Math.max(1, durationMs);
    return { startMs: nextStartMs, endMs: nextEndMs };
  };

  const alignLightingAutomationDraftToPlayhead = (automation: TimelineAutomationSummary) => {
    const draft = options.timelineAutomationDraft(automation);
    const range = automationDraftRangeAtPlayhead(draft.start_ms, draft.end_ms);
    options.updateTimelineAutomationDraft(automation, { start_ms: range.startMs, end_ms: range.endMs });
    options.setMessage(`Aligned automation ${automation.id} draft to ${range.startMs}-${range.endMs} ms`);
  };

  const alignVideoAutomationDraftToPlayhead = (automation: TimelineVideoAutomationSummary) => {
    const draft = options.timelineVideoAutomationDraft(automation);
    const range = automationDraftRangeAtPlayhead(draft.start_ms, draft.end_ms);
    options.updateTimelineVideoAutomationDraft(automation, { start_ms: range.startMs, end_ms: range.endMs });
    options.setMessage(`Aligned video automation ${automation.id} draft to ${range.startMs}-${range.endMs} ms`);
  };

  const addLightingAutomationKeyframeAtPlayhead = async (automation: TimelineAutomationSummary) => {
    const timeMs = options.snapTimeMs(options.snapshot().timeline.position_ms);
    const capturedValue = options.currentLightingAutomationValue(automation);
    const evaluatedValue = capturedValue ?? evaluateTimelineKeyframes(automation.keyframes, timeMs);
    if (evaluatedValue === null || !Number.isFinite(evaluatedValue)) {
      options.setMessage(`Lighting automation ${automation.id} has no keyframes.`);
      return;
    }
    const value = clampDmxValue(evaluatedValue);
    const keyframes = upsertTimelineKeyframe<AutomationKeyframeSummary>(automation.keyframes, {
      time_ms: timeMs,
      value,
      interpolation: interpolationForInsertedKeyframe(automation.keyframes, timeMs),
    });
    try {
      await options.invoke("set_timeline_automation", {
        automationId: automation.id,
        fixtureId: automation.fixture_id,
        attribute: automation.attribute,
        keyframes,
      });
      options.setTimelineAutomationDrafts((current) => ({
        ...current,
        [automation.id]: {
          ...(current[automation.id] ?? timelineAutomationDraftFromSummary(automation)),
          ...draftRangeFromKeyframes(keyframes),
        },
      }));
      options.setMessage(`Captured automation ${automation.id} key at ${timeMs} ms = ${value}`);
      await options.refreshSnapshot();
    } catch (error) {
      options.setMessage(String(error));
    }
  };

  const addVideoAutomationKeyframeAtPlayhead = async (automation: TimelineVideoAutomationSummary) => {
    const timeMs = options.snapTimeMs(options.snapshot().timeline.position_ms);
    const capturedValue = options.currentVideoAutomationValue(automation);
    const evaluatedValue = capturedValue ?? evaluateTimelineKeyframes(automation.keyframes, timeMs);
    if (evaluatedValue === null || !Number.isFinite(evaluatedValue)) {
      options.setMessage(`Video automation ${automation.id} has no finite keyframe value.`);
      return;
    }
    const keyframes = upsertTimelineKeyframe<VideoAutomationKeyframeSummary>(automation.keyframes, {
      time_ms: timeMs,
      value: evaluatedValue,
      interpolation: interpolationForInsertedKeyframe(automation.keyframes, timeMs),
    });
    try {
      await options.invoke("set_timeline_video_automation", {
        automationId: automation.id,
        layerId: automation.layer_id,
        param: automation.param,
        keyframes,
      });
      options.setTimelineVideoAutomationDrafts((current) => ({
        ...current,
        [automation.id]: {
          ...(current[automation.id] ?? timelineVideoAutomationDraftFromSummary(automation)),
          ...draftRangeFromKeyframes(keyframes),
        },
      }));
      options.setMessage(`Captured video automation ${automation.id} key at ${timeMs} ms = ${evaluatedValue.toFixed(3)}`);
      await options.refreshSnapshot();
    } catch (error) {
      options.setMessage(String(error));
    }
  };

  const removeLightingAutomationKeyframeResult = async (
    automation: TimelineAutomationSummary,
    result: { keyframes: AutomationKeyframeSummary[]; keyframeIndex: number; timeMs: number } | null,
  ) => {
    if (!result) {
      options.setMessage(`Automation ${automation.id} needs at least two keyframes.`);
      return;
    }
    try {
      await options.invoke("set_timeline_automation", {
        automationId: automation.id,
        fixtureId: automation.fixture_id,
        attribute: automation.attribute,
        keyframes: result.keyframes,
      });
      options.setTimelineAutomationDrafts((current) => ({
        ...current,
        [automation.id]: timelineAutomationDraftFromSummary({ ...automation, keyframes: result.keyframes }),
      }));
      options.setMessage(`Removed automation ${automation.id} key ${result.keyframeIndex + 1} at ${result.timeMs} ms`);
      await options.refreshSnapshot();
    } catch (error) {
      options.setMessage(String(error));
    }
  };

  const removeLightingAutomationKeyframeAtPlayhead = async (automation: TimelineAutomationSummary) => {
    if (automation.keyframes.length <= 2) {
      options.setMessage(`Automation ${automation.id} needs at least two keyframes.`);
      return;
    }
    const result = keyframesWithoutPlayheadKeyframe(
      automation.keyframes,
      options.snapshot().timeline.position_ms,
      options.keyframeDeleteToleranceMs(),
    );
    if (!result) {
      options.setMessage(`Seek to a keyframe before removing it from automation ${automation.id}.`);
      return;
    }
    await removeLightingAutomationKeyframeResult(automation, result);
  };

  const removeLightingAutomationKeyframe = async (automation: TimelineAutomationSummary, keyframeIndex: number) => {
    await removeLightingAutomationKeyframeResult(automation, keyframesWithoutIndex(automation.keyframes, keyframeIndex));
  };

  const setLightingAutomationKeyframeInterpolation = async (
    automation: TimelineAutomationSummary,
    keyframeIndex: number,
    interpolation: AutomationInterpolation,
  ) => {
    const keyframes = keyframesWithInterpolationAtIndex(automation.keyframes, keyframeIndex, interpolation);
    if (!keyframes) {
      options.setMessage(`Lighting automation key ${keyframeIndex + 1} was not found.`);
      return;
    }
    try {
      await options.invoke("set_timeline_automation", {
        automationId: automation.id,
        fixtureId: automation.fixture_id,
        attribute: automation.attribute,
        keyframes,
      });
      options.setTimelineAutomationDrafts((current) => ({
        ...current,
        [automation.id]: timelineAutomationDraftFromSummary({ ...automation, keyframes }),
      }));
      options.setMessage(`Set automation ${automation.id} key ${keyframeIndex + 1} curve ${interpolation}`);
      await options.refreshSnapshot();
    } catch (error) {
      options.setMessage(String(error));
    }
  };

  const setLightingAutomationKeyframeValue = async (
    automation: TimelineAutomationSummary,
    keyframeIndex: number,
    value: number,
  ) => {
    const nextValue = clampDmxValue(value);
    const keyframes = keyframesWithValueAtIndex(automation.keyframes, keyframeIndex, nextValue);
    if (!keyframes) {
      options.setMessage(`Lighting automation key ${keyframeIndex + 1} needs a finite DMX value.`);
      return;
    }
    try {
      await options.invoke("set_timeline_automation", {
        automationId: automation.id,
        fixtureId: automation.fixture_id,
        attribute: automation.attribute,
        keyframes,
      });
      options.setTimelineAutomationDrafts((current) => ({
        ...current,
        [automation.id]: timelineAutomationDraftFromSummary({ ...automation, keyframes }),
      }));
      options.setMessage(`Set automation ${automation.id} key ${keyframeIndex + 1} value ${nextValue}`);
      await options.refreshSnapshot();
    } catch (error) {
      options.setMessage(String(error));
    }
  };

  const removeVideoAutomationKeyframeResult = async (
    automation: TimelineVideoAutomationSummary,
    result: { keyframes: VideoAutomationKeyframeSummary[]; keyframeIndex: number; timeMs: number } | null,
  ) => {
    if (!result) {
      options.setMessage(`Video automation ${automation.id} needs at least two keyframes.`);
      return;
    }
    try {
      await options.invoke("set_timeline_video_automation", {
        automationId: automation.id,
        layerId: automation.layer_id,
        param: automation.param,
        keyframes: result.keyframes,
      });
      options.setTimelineVideoAutomationDrafts((current) => ({
        ...current,
        [automation.id]: timelineVideoAutomationDraftFromSummary({ ...automation, keyframes: result.keyframes }),
      }));
      options.setMessage(`Removed video automation ${automation.id} key ${result.keyframeIndex + 1} at ${result.timeMs} ms`);
      await options.refreshSnapshot();
    } catch (error) {
      options.setMessage(String(error));
    }
  };

  const removeVideoAutomationKeyframeAtPlayhead = async (automation: TimelineVideoAutomationSummary) => {
    if (automation.keyframes.length <= 2) {
      options.setMessage(`Video automation ${automation.id} needs at least two keyframes.`);
      return;
    }
    const result = keyframesWithoutPlayheadKeyframe(
      automation.keyframes,
      options.snapshot().timeline.position_ms,
      options.keyframeDeleteToleranceMs(),
    );
    if (!result) {
      options.setMessage(`Seek to a keyframe before removing it from video automation ${automation.id}.`);
      return;
    }
    await removeVideoAutomationKeyframeResult(automation, result);
  };

  const removeVideoAutomationKeyframe = async (
    automation: TimelineVideoAutomationSummary,
    keyframeIndex: number,
  ) => {
    await removeVideoAutomationKeyframeResult(automation, keyframesWithoutIndex(automation.keyframes, keyframeIndex));
  };

  const setVideoAutomationKeyframeInterpolation = async (
    automation: TimelineVideoAutomationSummary,
    keyframeIndex: number,
    interpolation: AutomationInterpolation,
  ) => {
    const keyframes = keyframesWithInterpolationAtIndex(automation.keyframes, keyframeIndex, interpolation);
    if (!keyframes) {
      options.setMessage(`Video automation key ${keyframeIndex + 1} was not found.`);
      return;
    }
    try {
      await options.invoke("set_timeline_video_automation", {
        automationId: automation.id,
        layerId: automation.layer_id,
        param: automation.param,
        keyframes,
      });
      options.setTimelineVideoAutomationDrafts((current) => ({
        ...current,
        [automation.id]: timelineVideoAutomationDraftFromSummary({ ...automation, keyframes }),
      }));
      options.setMessage(`Set video automation ${automation.id} key ${keyframeIndex + 1} curve ${interpolation}`);
      await options.refreshSnapshot();
    } catch (error) {
      options.setMessage(String(error));
    }
  };

  const setVideoAutomationKeyframeValue = async (
    automation: TimelineVideoAutomationSummary,
    keyframeIndex: number,
    value: number,
  ) => {
    const keyframes = keyframesWithValueAtIndex(automation.keyframes, keyframeIndex, value);
    if (!keyframes) {
      options.setMessage(`Video automation key ${keyframeIndex + 1} needs a finite value.`);
      return;
    }
    try {
      await options.invoke("set_timeline_video_automation", {
        automationId: automation.id,
        layerId: automation.layer_id,
        param: automation.param,
        keyframes,
      });
      options.setTimelineVideoAutomationDrafts((current) => ({
        ...current,
        [automation.id]: timelineVideoAutomationDraftFromSummary({ ...automation, keyframes }),
      }));
      options.setMessage(`Set video automation ${automation.id} key ${keyframeIndex + 1} value ${value}`);
      await options.refreshSnapshot();
    } catch (error) {
      options.setMessage(String(error));
    }
  };

  return {
    usePlayheadForLightingAutomation,
    usePlayheadForVideoAutomation,
    alignLightingAutomationDraftToPlayhead,
    alignVideoAutomationDraftToPlayhead,
    addLightingAutomationKeyframeAtPlayhead,
    addVideoAutomationKeyframeAtPlayhead,
    removeLightingAutomationKeyframeAtPlayhead,
    removeLightingAutomationKeyframe,
    setLightingAutomationKeyframeInterpolation,
    setLightingAutomationKeyframeValue,
    removeVideoAutomationKeyframeAtPlayhead,
    removeVideoAutomationKeyframe,
    setVideoAutomationKeyframeInterpolation,
    setVideoAutomationKeyframeValue,
  };
}
