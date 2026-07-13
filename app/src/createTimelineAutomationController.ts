import type { Accessor, Setter } from "solid-js";
import {
  type TimelineAutomationDraft,
  type TimelineVideoAutomationDraft,
} from "./editorDrafts";
import { clampDmxValue } from "./numericHelpers";
import { keyframesWithDraftEndpoints } from "./timelineAutomationHelpers";
import type {
  AutomationKeyframeSummary,
  EngineSnapshot,
  PatchedFixtureSummary,
  TimelineAutomationSummary,
  TimelineGroupAutomationAddResult,
  TimelineVideoAutomationSummary,
  VideoAutomationKeyframeSummary,
  VideoParam,
} from "./types";

type Invoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

interface TimelineAutomationControllerOptions {
  snapshot: Accessor<EngineSnapshot>;
  selectedFixture: Accessor<PatchedFixtureSummary | undefined>;
  selectedFixtureGroupFilter: Accessor<string | null>;
  selectedTimelineAutomationAttribute: Accessor<string>;
  selectedFixtureSupportsTimelineAutomationAttribute: Accessor<boolean>;
  automationStartMs: Accessor<number>;
  automationEndMs: Accessor<number>;
  automationStartValue: Accessor<number>;
  automationEndValue: Accessor<number>;
  automationInterpolation: Accessor<AutomationKeyframeSummary["interpolation"]>;
  selectedVideoAutomationLayerId: Accessor<number | null>;
  videoAutomationParam: Accessor<VideoParam>;
  videoAutomationStartMs: Accessor<number>;
  videoAutomationEndMs: Accessor<number>;
  videoAutomationStartValue: Accessor<number>;
  videoAutomationEndValue: Accessor<number>;
  videoAutomationInterpolation: Accessor<VideoAutomationKeyframeSummary["interpolation"]>;
  timelineAutomationDraft: (automation: TimelineAutomationSummary) => TimelineAutomationDraft;
  timelineVideoAutomationDraft: (automation: TimelineVideoAutomationSummary) => TimelineVideoAutomationDraft;
  setTimelineAutomationDrafts: Setter<Record<number, TimelineAutomationDraft>>;
  setTimelineVideoAutomationDrafts: Setter<Record<number, TimelineVideoAutomationDraft>>;
  snapTimeMs: (timeMs: number) => number;
  invoke: Invoke;
  setMessage: (message: string) => unknown;
  refreshSnapshot: () => Promise<EngineSnapshot | null>;
}

export function createTimelineAutomationController(options: TimelineAutomationControllerOptions) {
  const setTimelineAutomationEnabled = async (automationId: number, enabled: boolean) => {
    try {
      await options.invoke("set_timeline_automation_enabled", { automationId, enabled });
      options.setMessage(`${enabled ? "Enabled" : "Disabled"} automation ${automationId}`);
      await options.refreshSnapshot();
    } catch (error) {
      options.setMessage(String(error));
    }
  };

  const setAutomationRowsEnabled = async (
    automations: Array<{ id: number; enabled: boolean }>,
    enabled: boolean,
    label: "lighting" | "video",
  ) => {
    const automationIds = automations.filter((automation) => automation.enabled !== enabled).map((automation) => automation.id);
    if (automationIds.length === 0) {
      options.setMessage(`${label === "lighting" ? "Lighting" : "Video"} automation rows are already ${enabled ? "enabled" : "disabled"}.`);
      return;
    }
    try {
      for (const automationId of automationIds) {
        await options.invoke("set_timeline_automation_enabled", { automationId, enabled });
      }
      options.setMessage(`${enabled ? "Enabled" : "Disabled"} ${automationIds.length} ${label} automation row(s).`);
      await options.refreshSnapshot();
    } catch (error) {
      options.setMessage(String(error));
    }
  };

  const setLightingAutomationRowsEnabled = (automations: TimelineAutomationSummary[], enabled: boolean) =>
    setAutomationRowsEnabled(automations, enabled, "lighting");
  const setVideoAutomationRowsEnabled = (automations: TimelineVideoAutomationSummary[], enabled: boolean) =>
    setAutomationRowsEnabled(automations, enabled, "video");

  const addTimelineAutomation = async () => {
    const fixture = options.selectedFixture();
    const attribute = options.selectedTimelineAutomationAttribute();
    if (!fixture || !attribute || !options.selectedFixtureSupportsTimelineAutomationAttribute()) {
      options.setMessage("Select a fixture and supported automation attribute first.");
      return;
    }
    const startMs = options.snapTimeMs(options.automationStartMs());
    const endMs = Math.max(startMs, options.snapTimeMs(options.automationEndMs()));
    try {
      const automationId = await options.invoke<number>("add_timeline_automation", {
        fixtureId: fixture.id,
        attribute,
        keyframes: [
          { time_ms: startMs, value: options.automationStartValue(), interpolation: options.automationInterpolation() },
          { time_ms: endMs, value: options.automationEndValue(), interpolation: "Step" },
        ],
      });
      options.setMessage(`Added automation ${automationId}`);
      await options.refreshSnapshot();
    } catch (error) {
      options.setMessage(String(error));
    }
  };

  const addTimelineGroupAutomation = async () => {
    const groupId = options.selectedFixtureGroupFilter();
    const attribute = options.selectedTimelineAutomationAttribute();
    if (!groupId || !attribute) {
      options.setMessage("Select a group and attribute first.");
      return;
    }
    const startMs = options.snapTimeMs(options.automationStartMs());
    const endMs = Math.max(startMs, options.snapTimeMs(options.automationEndMs()));
    try {
      const result = await options.invoke<TimelineGroupAutomationAddResult>("add_timeline_group_automation", {
        groupId,
        attribute,
        keyframes: [
          { time_ms: startMs, value: options.automationStartValue(), interpolation: options.automationInterpolation() },
          { time_ms: endMs, value: options.automationEndValue(), interpolation: "Step" },
        ],
      });
      options.setMessage(
        `Added ${result.applied_count} automation(s) to ${groupId}; ${result.skipped_count} incompatible fixture(s) skipped.`,
      );
      await options.refreshSnapshot();
    } catch (error) {
      options.setMessage(String(error));
    }
  };

  const setTimelineAutomation = async (automation: TimelineAutomationSummary) => {
    const draft = options.timelineAutomationDraft(automation);
    if (!draft.attribute.trim()) {
      options.setMessage("Select an automation attribute.");
      return;
    }
    const startMs = options.snapTimeMs(draft.start_ms);
    const endMs = Math.max(startMs, options.snapTimeMs(draft.end_ms));
    const startValue = clampDmxValue(draft.start_value);
    const endValue = clampDmxValue(draft.end_value);
    const keyframes = keyframesWithDraftEndpoints<AutomationKeyframeSummary>(
      automation.keyframes,
      startMs,
      endMs,
      startValue,
      endValue,
      draft.interpolation,
    );
    try {
      await options.invoke("set_timeline_automation", {
        automationId: automation.id,
        fixtureId: Math.max(0, Math.round(draft.fixture_id)),
        attribute: draft.attribute,
        keyframes,
      });
      options.setTimelineAutomationDrafts((current) => ({
        ...current,
        [automation.id]: { ...draft, start_ms: startMs, end_ms: endMs, start_value: startValue, end_value: endValue },
      }));
      options.setMessage(`Saved automation ${automation.id}`);
      await options.refreshSnapshot();
    } catch (error) {
      options.setMessage(String(error));
    }
  };

  const addTimelineVideoAutomation = async () => {
    const layerId = options.selectedVideoAutomationLayerId();
    if (layerId === null) {
      options.setMessage("Add a video layer before adding video automation.");
      return;
    }
    const startMs = options.snapTimeMs(options.videoAutomationStartMs());
    const endMs = Math.max(startMs, options.snapTimeMs(options.videoAutomationEndMs()));
    try {
      const automationId = await options.invoke<number>("add_timeline_video_automation", {
        layerId,
        param: options.videoAutomationParam(),
        keyframes: [
          { time_ms: startMs, value: options.videoAutomationStartValue(), interpolation: options.videoAutomationInterpolation() },
          { time_ms: endMs, value: options.videoAutomationEndValue(), interpolation: "Step" },
        ],
      });
      options.setMessage(`Added video automation ${automationId}`);
      await options.refreshSnapshot();
    } catch (error) {
      options.setMessage(String(error));
    }
  };

  const setTimelineVideoAutomation = async (automation: TimelineVideoAutomationSummary) => {
    const draft = options.timelineVideoAutomationDraft(automation);
    if (!Number.isFinite(draft.start_value) || !Number.isFinite(draft.end_value)) {
      options.setMessage("Video automation values must be finite.");
      return;
    }
    const startMs = options.snapTimeMs(draft.start_ms);
    const endMs = Math.max(startMs, options.snapTimeMs(draft.end_ms));
    const keyframes = keyframesWithDraftEndpoints<VideoAutomationKeyframeSummary>(
      automation.keyframes,
      startMs,
      endMs,
      draft.start_value,
      draft.end_value,
      draft.interpolation,
    );
    try {
      await options.invoke("set_timeline_video_automation", {
        automationId: automation.id,
        layerId: Math.max(0, Math.round(draft.layer_id)),
        param: draft.param,
        keyframes,
      });
      options.setTimelineVideoAutomationDrafts((current) => ({
        ...current,
        [automation.id]: { ...draft, start_ms: startMs, end_ms: endMs },
      }));
      options.setMessage(`Saved video automation ${automation.id}`);
      await options.refreshSnapshot();
    } catch (error) {
      options.setMessage(String(error));
    }
  };

  const removeTimelineAutomation = async (automationId: number) => {
    try {
      await options.invoke("remove_timeline_automation", { automationId });
      options.setMessage(`Removed automation ${automationId}`);
      await options.refreshSnapshot();
    } catch (error) {
      options.setMessage(String(error));
    }
  };

  const setTimelinePlaying = async (playing: boolean) => {
    try {
      await options.invoke("set_timeline_playing", { playing });
      options.setMessage(playing ? "Timeline playing." : "Timeline paused.");
      await options.refreshSnapshot();
    } catch (error) {
      options.setMessage(String(error));
    }
  };
  const playTimeline = () => setTimelinePlaying(true);
  const pauseTimeline = () => setTimelinePlaying(false);

  const seekTimeline = async (positionMs: number) => {
    try {
      await options.invoke("seek_timeline", { positionMs });
      options.setMessage(`Timeline seek ${positionMs}ms`);
      await options.refreshSnapshot();
    } catch (error) {
      options.setMessage(String(error));
    }
  };

  const seekTimelineFromOverviewTime = (timeMs: number) => {
    void seekTimeline(options.snapTimeMs(Math.max(0, Math.round(timeMs))));
  };

  return {
    setTimelineAutomationEnabled,
    setLightingAutomationRowsEnabled,
    setVideoAutomationRowsEnabled,
    addTimelineAutomation,
    addTimelineGroupAutomation,
    setTimelineAutomation,
    addTimelineVideoAutomation,
    setTimelineVideoAutomation,
    removeTimelineAutomation,
    playTimeline,
    pauseTimeline,
    seekTimeline,
    seekTimelineFromOverviewTime,
  };
}
