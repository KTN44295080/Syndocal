import type { Accessor, Setter } from "solid-js";
import type { TimelineOverviewAutomationRange } from "./components/TimelineOverview";
import {
  timelineAutomationDraftFromSummary,
  timelineVideoAutomationDraftFromSummary,
  type TimelineAutomationDraft,
  type TimelineVideoAutomationDraft,
} from "./editorDrafts";
import { clampRange } from "./numericHelpers";
import {
  draftRangeFromKeyframes,
  movedTimelineKeyframe,
  resizedTimelineKeyframes,
  shiftedTimelineKeyframes,
  timelineKeyframeMoveTime,
} from "./timelineAutomationHelpers";
import type { EngineSnapshot } from "./types";

type Invoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

interface TimelineOverviewAutomationControllerOptions {
  snapshot: Accessor<EngineSnapshot>;
  snapTimeMs: (timeMs: number) => number;
  timelineOverviewDurationMs: Accessor<number>;
  invoke: Invoke;
  setTimelineAutomationDrafts: Setter<Record<number, TimelineAutomationDraft>>;
  setTimelineVideoAutomationDrafts: Setter<Record<number, TimelineVideoAutomationDraft>>;
  setMessage: (message: string) => unknown;
  refreshSnapshot: () => Promise<EngineSnapshot | null>;
}

export function createTimelineOverviewAutomationController(
  options: TimelineOverviewAutomationControllerOptions,
) {
  const automationBoundsFromResizeRatio = (
    range: TimelineOverviewAutomationRange,
    edge: "start" | "end",
    ratio: number,
  ) => {
    const targetMs = options.snapTimeMs(
      Math.round(clampRange(ratio, 0, 1) * options.timelineOverviewDurationMs()),
    );
    const startMs = Math.max(0, range.start_ms);
    const endMs = Math.max(startMs + 1, range.end_ms);
    return edge === "start"
      ? { startMs: Math.max(0, Math.min(targetMs, endMs - 1)), endMs }
      : { startMs, endMs: Math.max(startMs + 1, targetMs) };
  };

  const moveTimelineAutomationRangeToRatio = async (
    range: TimelineOverviewAutomationRange,
    ratio: number,
  ) => {
    const nextStartMs = options.snapTimeMs(
      Math.round(clampRange(ratio, 0, 1) * options.timelineOverviewDurationMs()),
    );
    if (range.kind === "lighting") {
      const automation = options.snapshot().timeline.automations.find(
        (candidate) => candidate.id === range.automation_id,
      );
      if (!automation) {
        options.setMessage(`Lighting automation ${range.automation_id} was not found.`);
        return;
      }
      const keyframes = shiftedTimelineKeyframes(automation.keyframes, nextStartMs);
      if (!keyframes) {
        options.setMessage(`Lighting automation ${automation.id} has no keyframes.`);
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
          [automation.id]: {
            ...(current[automation.id] ?? timelineAutomationDraftFromSummary(automation)),
            ...draftRangeFromKeyframes(keyframes),
          },
        }));
        options.setMessage(`Moved automation ${automation.id} to ${nextStartMs} ms`);
        await options.refreshSnapshot();
      } catch (error) {
        options.setMessage(String(error));
      }
      return;
    }

    const automation = options.snapshot().timeline.video_automations.find(
      (candidate) => candidate.id === range.automation_id,
    );
    if (!automation) {
      options.setMessage(`Video automation ${range.automation_id} was not found.`);
      return;
    }
    const keyframes = shiftedTimelineKeyframes(automation.keyframes, nextStartMs);
    if (!keyframes) {
      options.setMessage(`Video automation ${automation.id} has no keyframes.`);
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
        [automation.id]: {
          ...(current[automation.id] ?? timelineVideoAutomationDraftFromSummary(automation)),
          ...draftRangeFromKeyframes(keyframes),
        },
      }));
      options.setMessage(`Moved video automation ${automation.id} to ${nextStartMs} ms`);
      await options.refreshSnapshot();
    } catch (error) {
      options.setMessage(String(error));
    }
  };

  const resizeTimelineAutomationRangeToRatio = async (
    range: TimelineOverviewAutomationRange,
    edge: "start" | "end",
    ratio: number,
  ) => {
    const bounds = automationBoundsFromResizeRatio(range, edge, ratio);
    if (range.kind === "lighting") {
      const automation = options.snapshot().timeline.automations.find(
        (candidate) => candidate.id === range.automation_id,
      );
      if (!automation) {
        options.setMessage(`Lighting automation ${range.automation_id} was not found.`);
        return;
      }
      const keyframes = resizedTimelineKeyframes(automation.keyframes, bounds.startMs, bounds.endMs);
      if (!keyframes) {
        options.setMessage(`Lighting automation ${automation.id} has no keyframes.`);
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
          [automation.id]: {
            ...(current[automation.id] ?? timelineAutomationDraftFromSummary(automation)),
            ...draftRangeFromKeyframes(keyframes),
          },
        }));
        options.setMessage(`Resized automation ${automation.id} to ${bounds.startMs}-${bounds.endMs} ms`);
        await options.refreshSnapshot();
      } catch (error) {
        options.setMessage(String(error));
      }
      return;
    }

    const automation = options.snapshot().timeline.video_automations.find(
      (candidate) => candidate.id === range.automation_id,
    );
    if (!automation) {
      options.setMessage(`Video automation ${range.automation_id} was not found.`);
      return;
    }
    const keyframes = resizedTimelineKeyframes(automation.keyframes, bounds.startMs, bounds.endMs);
    if (!keyframes) {
      options.setMessage(`Video automation ${automation.id} has no keyframes.`);
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
        [automation.id]: {
          ...(current[automation.id] ?? timelineVideoAutomationDraftFromSummary(automation)),
          ...draftRangeFromKeyframes(keyframes),
        },
      }));
      options.setMessage(`Resized video automation ${automation.id} to ${bounds.startMs}-${bounds.endMs} ms`);
      await options.refreshSnapshot();
    } catch (error) {
      options.setMessage(String(error));
    }
  };

  const moveTimelineAutomationKeyframeToRatio = async (
    range: TimelineOverviewAutomationRange,
    keyframeIndex: number,
    ratio: number,
  ) => {
    const nextTimeMs = options.snapTimeMs(
      Math.round(clampRange(ratio, 0, 1) * options.timelineOverviewDurationMs()),
    );
    const lighting = range.kind === "lighting";
    const automation = lighting
      ? options.snapshot().timeline.automations.find((candidate) => candidate.id === range.automation_id)
      : options.snapshot().timeline.video_automations.find((candidate) => candidate.id === range.automation_id);
    if (!automation) {
      options.setMessage(`${lighting ? "Lighting" : "Video"} automation ${range.automation_id} was not found.`);
      return;
    }
    const keyframes = movedTimelineKeyframe(automation.keyframes, keyframeIndex, nextTimeMs);
    if (!keyframes) {
      options.setMessage(`${lighting ? "Lighting" : "Video"} automation keyframe ${keyframeIndex + 1} was not found.`);
      return;
    }
    const movedTimeMs = timelineKeyframeMoveTime(automation.keyframes, keyframeIndex, nextTimeMs) ?? nextTimeMs;
    try {
      if (lighting) {
        const item = automation as EngineSnapshot["timeline"]["automations"][number];
        await options.invoke("set_timeline_automation", {
          automationId: item.id,
          fixtureId: item.fixture_id,
          attribute: item.attribute,
          keyframes,
        });
        options.setTimelineAutomationDrafts((current) => ({
          ...current,
          [item.id]: {
            ...(current[item.id] ?? timelineAutomationDraftFromSummary(item)),
            ...draftRangeFromKeyframes(keyframes),
          },
        }));
      } else {
        const item = automation as EngineSnapshot["timeline"]["video_automations"][number];
        await options.invoke("set_timeline_video_automation", {
          automationId: item.id,
          layerId: item.layer_id,
          param: item.param,
          keyframes,
        });
        options.setTimelineVideoAutomationDrafts((current) => ({
          ...current,
          [item.id]: {
            ...(current[item.id] ?? timelineVideoAutomationDraftFromSummary(item)),
            ...draftRangeFromKeyframes(keyframes),
          },
        }));
      }
      options.setMessage(
        `Moved ${lighting ? "automation" : "video automation"} ${automation.id} key ${keyframeIndex + 1} to ${movedTimeMs} ms`,
      );
      await options.refreshSnapshot();
    } catch (error) {
      options.setMessage(String(error));
    }
  };

  return {
    moveTimelineAutomationRangeToRatio,
    resizeTimelineAutomationRangeToRatio,
    moveTimelineAutomationKeyframeToRatio,
  };
}
