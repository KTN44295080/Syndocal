import type { Accessor, Setter } from "solid-js";
import type { FrontendTauriInvokeCommand } from "./tauriInvokeCommands";
import { timelinePlacementDisplayEndMs } from "./timelineSceneBlocks";
import { effectiveTimelineLayers, timelineLayerIdForEvent } from "./timelineLayers";
import type {
  ChildTimelineSummary,
  AutomationKeyframeSummary,
  CueSummary,
  EngineSnapshot,
  TimelineAutomationSummary,
  TimelineCueEventSummary,
  TimelineLayerSummary,
  TimelineSnapshot,
  TimelineTrackKind,
  TimelineVideoAutomationSummary,
  VideoAutomationKeyframeSummary,
  VideoParam,
} from "./types";

export type TimelineCommandDispatcherOptions = {
  viewportFixture: () => string | null;
  timelineAuthorityReady: (operation: string) => boolean;
  invoke: <T>(command: FrontendTauriInvokeCommand, args?: Record<string, unknown>) => Promise<T>;
  timelineChildCueId: () => number | null;
  timelineChildCue: () => CueSummary | null;
  normalizedChildTimeline: (child: ChildTimelineSummary) => ChildTimelineSummary;
  persistChildTimeline: (
    cueId: number,
    update: (child: ChildTimelineSummary) => ChildTimelineSummary,
  ) => Promise<void>;
  childTimelineNextAutomationId: () => number;
  childTimelineDurationForKeyframes: (keyframes: Array<{ time_ms: number }>) => number;
  snapshot: Accessor<EngineSnapshot>;
  activeTimeline: Accessor<TimelineSnapshot>;
  timelineLayers: Accessor<TimelineLayerSummary[]>;
  requireBankAuthority: () => unknown;
  bankAuthorityIssueMessage: () => string;
  timelineSceneBlockCueAllowed: (cueId: number, childCueId: number | null) => boolean;
  timelineSceneBlockCueRejectionMessage: (cueId: number, childCueId: number | null) => string;
  setSnapshot: Setter<EngineSnapshot>;
};

const isSceneFixture = (fixture: string | null) => fixture === "timeline-layered"
  || fixture === "scene-block-large"
  || fixture === "scene-block-hour"
  || fixture === "scene-matrix";

export const invokeTimelineEditingCommand = async <T,>(
  options: TimelineCommandDispatcherOptions,
  command: FrontendTauriInvokeCommand,
  args?: Record<string, unknown>,
): Promise<T> => {
  const localFixture = isSceneFixture(options.viewportFixture());
  if (!localFixture && !options.timelineAuthorityReady(command)) {
    throw new Error("Timeline authority is not ready; command was not sent.");
  }
  const childCueId = options.timelineChildCueId();
  if (childCueId === null) return options.invoke<T>(command, args);
  if (command === "seek_timeline") {
    return options.invoke<T>("seek_direct_child_timeline", { cueId: childCueId, ...args });
  }
  if (command === "set_timeline_metronome") {
    const enabled = Boolean(args?.enabled);
    const countInBeats = Math.max(0, Math.min(16, Math.round(Number(args?.countInBeats) || 0)));
    await options.persistChildTimeline(childCueId, (current) => ({
      ...current, metronome_enabled: enabled, count_in_beats: countInBeats,
    }));
    return undefined as T;
  }
  const child = options.normalizedChildTimeline(options.timelineChildCue()?.child_timeline ?? {});
  if (command === "set_timeline_automation_enabled") {
    const automationId = Number(args?.automationId);
    const enabled = Boolean(args?.enabled);
    await options.persistChildTimeline(childCueId, (current) => ({
      ...current,
      automations: (current.automations ?? []).map((automation) => automation.id === automationId
        ? { ...automation, enabled } : automation),
      video_automations: (current.video_automations ?? []).map((automation) => automation.id === automationId
        ? { ...automation, enabled } : automation),
    }));
    return undefined as T;
  }
  if (command === "add_timeline_automation") {
    const automationId = options.childTimelineNextAutomationId();
    const keyframes = (args?.keyframes ?? []) as AutomationKeyframeSummary[];
    const automation: TimelineAutomationSummary = {
      id: automationId, fixture_id: Number(args?.fixtureId), attribute: String(args?.attribute ?? ""),
      track: "Lighting", keyframes, enabled: true,
    };
    await options.persistChildTimeline(childCueId, (current) => ({
      ...current, automations: [...(current.automations ?? []), automation],
      duration_ms: Math.max(current.duration_ms ?? 0, options.childTimelineDurationForKeyframes(keyframes)),
    }));
    return automationId as T;
  }
  if (command === "add_timeline_group_automation") {
    const groupId = String(args?.groupId ?? "");
    const attribute = String(args?.attribute ?? "");
    const keyframes = (args?.keyframes ?? []) as AutomationKeyframeSummary[];
    const fixtures = options.snapshot().fixtures.filter((fixture) => fixture.group_ids.includes(groupId));
    const compatible = fixtures.filter((fixture) => fixture.controls.some((control) => control.attribute === attribute));
    let nextId = options.childTimelineNextAutomationId();
    const automations = compatible.map((fixture): TimelineAutomationSummary => ({
      id: nextId++, fixture_id: fixture.id, attribute, track: "Lighting", keyframes, enabled: true,
    }));
    await options.persistChildTimeline(childCueId, (current) => ({
      ...current, automations: [...(current.automations ?? []), ...automations],
      duration_ms: Math.max(current.duration_ms ?? 0, options.childTimelineDurationForKeyframes(keyframes)),
    }));
    return { automation_ids: automations.map((automation) => automation.id), applied_count: automations.length,
      skipped_count: fixtures.length - automations.length } as T;
  }
  if (command === "set_timeline_automation") {
    const automationId = Number(args?.automationId);
    const keyframes = (args?.keyframes ?? []) as AutomationKeyframeSummary[];
    if (!child.automations?.some((automation) => automation.id === automationId)) {
      throw new Error(`Child lighting automation ${automationId} was not found`);
    }
    await options.persistChildTimeline(childCueId, (current) => ({
      ...current,
      automations: (current.automations ?? []).map((automation) => automation.id === automationId
        ? { ...automation, fixture_id: Number(args?.fixtureId), attribute: String(args?.attribute ?? ""), keyframes }
        : automation),
      duration_ms: Math.max(current.duration_ms ?? 0, options.childTimelineDurationForKeyframes(keyframes)),
    }));
    return undefined as T;
  }
  if (command === "add_timeline_video_automation") {
    const automationId = options.childTimelineNextAutomationId();
    const keyframes = (args?.keyframes ?? []) as VideoAutomationKeyframeSummary[];
    const automation: TimelineVideoAutomationSummary = {
      id: automationId, layer_id: Number(args?.layerId), param: args?.param as VideoParam,
      track: "Video", keyframes, enabled: true,
    };
    await options.persistChildTimeline(childCueId, (current) => ({
      ...current, video_automations: [...(current.video_automations ?? []), automation],
      duration_ms: Math.max(current.duration_ms ?? 0, options.childTimelineDurationForKeyframes(keyframes)),
    }));
    return automationId as T;
  }
  if (command === "set_timeline_video_automation") {
    const automationId = Number(args?.automationId);
    const keyframes = (args?.keyframes ?? []) as VideoAutomationKeyframeSummary[];
    if (!child.video_automations?.some((automation) => automation.id === automationId)) {
      throw new Error(`Child video automation ${automationId} was not found`);
    }
    await options.persistChildTimeline(childCueId, (current) => ({
      ...current,
      video_automations: (current.video_automations ?? []).map((automation) => automation.id === automationId
        ? { ...automation, layer_id: Number(args?.layerId), param: args?.param as VideoParam, keyframes }
        : automation),
      duration_ms: Math.max(current.duration_ms ?? 0, options.childTimelineDurationForKeyframes(keyframes)),
    }));
    return undefined as T;
  }
  if (command === "remove_timeline_automation") {
    const automationId = Number(args?.automationId);
    await options.persistChildTimeline(childCueId, (current) => ({
      ...current, automations: (current.automations ?? []).filter((automation) => automation.id !== automationId),
      video_automations: (current.video_automations ?? []).filter((automation) => automation.id !== automationId),
    }));
    return undefined as T;
  }
  return options.invoke<T>(command, args);
};

export const invokeTimelineSceneBlockCommand = async <T,>(
  options: TimelineCommandDispatcherOptions,
  command: FrontendTauriInvokeCommand,
  args?: Record<string, unknown>,
): Promise<T> => {
  const localFixture = isSceneFixture(options.viewportFixture());
  if (!localFixture && !options.timelineAuthorityReady(command)) {
    throw new Error("Timeline authority is not ready; command was not sent.");
  }
  if (!options.requireBankAuthority()) throw new Error(options.bankAuthorityIssueMessage());
  if (command === "add_timeline_scene_block" || command === "set_timeline_scene_block" || command === "set_timeline_cue_event") {
    const cueId = Number(args?.cueId);
    const childCueId = options.timelineChildCueId();
    if (!options.timelineSceneBlockCueAllowed(cueId, childCueId)) {
      throw new Error(options.timelineSceneBlockCueRejectionMessage(cueId, childCueId));
    }
  }
  const childCueId = options.timelineChildCueId();
  if (childCueId !== null) {
    const current = options.activeTimeline();
    if (command === "add_timeline_scene_block") {
      const eventId = Math.max(0, ...options.snapshot().timeline.events.map((event) => event.id),
        ...options.snapshot().cues.flatMap((cue) => cue.child_timeline?.events?.map((event) => event.id) ?? [])) + 1;
      const durationMs = Number(args?.durationMs ?? 1_000);
      const cueId = Number(args?.cueId);
      const authoredBeats = options.snapshot().cues.find((cue) => cue.id === cueId)?.authored_beats ?? null;
      const conformToTempo = Boolean(args?.conformToTempo);
      const loopFill = Boolean(args?.loopFill);
      const event: TimelineCueEventSummary = {
        id: eventId, cue_id: cueId, time_ms: Number(args?.timeMs ?? 0),
        time_beats: args?.timeBeats === null || args?.timeBeats === undefined ? null : Number(args.timeBeats),
        track: (args?.track ?? "Lighting") as TimelineTrackKind,
        layer_id: args?.layerId === null || args?.layerId === undefined ? null : Number(args.layerId),
        duration_ms: durationMs,
        duration_beats: args?.durationBeats === null || args?.durationBeats === undefined ? null : Number(args.durationBeats),
        conform_to_tempo: conformToTempo, loop_fill: loopFill,
        source_offset_ms: Math.round(Number(args?.sourceOffsetMs ?? 0)),
        rate: conformToTempo && !loopFill && authoredBeats !== null && durationMs > 0
          ? (authoredBeats * 60_000 / options.snapshot().clock.bpm) / durationMs : null,
        fade_in_ms: Math.min(Number(args?.fadeInMs ?? 0), durationMs),
        fade_out_ms: Math.min(Number(args?.fadeOutMs ?? 0), durationMs),
        loop_count: Number(args?.loopCount ?? 1),
        jump_to_event_id: args?.jumpToEventId === null || args?.jumpToEventId === undefined ? null : Number(args.jumpToEventId),
      };
      await options.persistChildTimeline(childCueId, (child) => ({
        ...child, events: [...(child.events ?? []), event],
        duration_ms: Math.max(child.duration_ms ?? 0, timelinePlacementDisplayEndMs(event)),
      }));
      return eventId as T;
    }
    if (command === "set_timeline_scene_block" || command === "set_timeline_cue_event") {
      const eventId = Number(args?.eventId);
      const source = current.events.find((event) => event.id === eventId);
      if (!source) throw new Error(`Child timeline event ${eventId} was not found`);
      const durationMs = command === "set_timeline_scene_block" ? Number(args?.durationMs ?? source.duration_ms) : 0;
      const cueId = Number(args?.cueId ?? source.cue_id);
      const authoredBeats = options.snapshot().cues.find((cue) => cue.id === cueId)?.authored_beats ?? null;
      const conformToTempo = command === "set_timeline_scene_block" && Boolean(args?.conformToTempo);
      const loopFill = command === "set_timeline_scene_block" && Boolean(args?.loopFill);
      const nextEvent: TimelineCueEventSummary = {
        ...source, cue_id: cueId, time_ms: Number(args?.timeMs ?? source.time_ms),
        time_beats: args?.timeBeats === null || args?.timeBeats === undefined ? null : Number(args.timeBeats),
        track: (args?.track ?? source.track) as TimelineTrackKind,
        layer_id: args?.layerId === null || args?.layerId === undefined ? null : Number(args.layerId),
        duration_ms: durationMs,
        duration_beats: command === "set_timeline_scene_block" && args?.durationBeats !== null && args?.durationBeats !== undefined ? Number(args.durationBeats) : null,
        conform_to_tempo: conformToTempo, loop_fill: loopFill,
        source_offset_ms: command === "set_timeline_scene_block" ? Math.round(Number(args?.sourceOffsetMs ?? source.source_offset_ms ?? 0)) : 0,
        rate: conformToTempo && !loopFill && authoredBeats !== null && durationMs > 0
          ? (authoredBeats * 60_000 / options.snapshot().clock.bpm) / durationMs : null,
        fade_in_ms: command === "set_timeline_scene_block" ? Math.min(Number(args?.fadeInMs ?? source.fade_in_ms ?? 0), durationMs) : 0,
        fade_out_ms: command === "set_timeline_scene_block" ? Math.min(Number(args?.fadeOutMs ?? source.fade_out_ms ?? 0), durationMs) : 0,
        loop_count: command === "set_timeline_scene_block" ? Number(args?.loopCount ?? source.loop_count) : 1,
        jump_to_event_id: command === "set_timeline_scene_block" && args?.jumpToEventId !== null && args?.jumpToEventId !== undefined ? Number(args.jumpToEventId) : null,
      };
      await options.persistChildTimeline(childCueId, (child) => ({ ...child,
        events: (child.events ?? []).map((event) => event.id === eventId ? nextEvent : event) }));
      return undefined as T;
    }
    if (command === "remove_timeline_scene_block" || command === "remove_timeline_event") {
      const eventId = Number(args?.eventId);
      await options.persistChildTimeline(childCueId, (child) => ({ ...child,
        events: (child.events ?? []).filter((event) => event.id !== eventId) }));
      return undefined as T;
    }
  }
  if (!localFixture) return options.invoke<T>(command, args);
  if (command === "add_timeline_scene_block") {
    const eventId = Math.max(0, ...options.snapshot().timeline.events.map((event) => event.id),
      ...options.snapshot().cues.flatMap((cue) => cue.child_timeline?.events?.map((event) => event.id) ?? [])) + 1;
    options.setSnapshot((current) => {
      const durationMs = Number(args?.durationMs ?? 1_000);
      const cue = current.cues.find((candidate) => candidate.id === Number(args?.cueId));
      const authoredBeats = cue?.authored_beats ?? null;
      const conformToTempo = Boolean(args?.conformToTempo);
      const loopFill = Boolean(args?.loopFill);
      return { ...current, timeline: { ...current.timeline, events: [...current.timeline.events, {
        id: eventId, cue_id: Number(args?.cueId), time_ms: Number(args?.timeMs ?? 0),
        time_beats: args?.timeBeats === null || args?.timeBeats === undefined ? null : Number(args.timeBeats),
        track: (args?.track ?? "Lighting") as TimelineTrackKind,
        layer_id: args?.layerId === null || args?.layerId === undefined ? null : Number(args.layerId),
        duration_ms: durationMs,
        duration_beats: args?.durationBeats === null || args?.durationBeats === undefined ? null : Number(args.durationBeats),
        conform_to_tempo: conformToTempo, loop_fill: loopFill, source_offset_ms: Math.round(Number(args?.sourceOffsetMs ?? 0)),
        rate: conformToTempo && !loopFill && authoredBeats !== null && durationMs > 0 ? (authoredBeats * 60_000 / current.clock.bpm) / durationMs : null,
        fade_in_ms: Math.min(Number(args?.fadeInMs ?? 0), durationMs), fade_out_ms: Math.min(Number(args?.fadeOutMs ?? 0), durationMs),
        loop_count: Number(args?.loopCount ?? 1),
        jump_to_event_id: args?.jumpToEventId === null || args?.jumpToEventId === undefined ? null : Number(args.jumpToEventId),
      }] } };
    });
    return eventId as T;
  }
  if (command === "set_timeline_scene_block" || command === "set_timeline_cue_event") {
    const eventId = Number(args?.eventId);
    options.setSnapshot((current) => ({ ...current, timeline: { ...current.timeline,
      events: current.timeline.events.map((event) => {
        if (event.id !== eventId) return event;
        const durationMs = command === "set_timeline_scene_block" ? Number(args?.durationMs ?? event.duration_ms) : 0;
        const cueId = Number(args?.cueId ?? event.cue_id);
        const authoredBeats = current.cues.find((candidate) => candidate.id === cueId)?.authored_beats ?? null;
        const conformToTempo = command === "set_timeline_scene_block" && Boolean(args?.conformToTempo);
        const loopFill = command === "set_timeline_scene_block" && Boolean(args?.loopFill);
        return { ...event, cue_id: cueId, time_ms: Number(args?.timeMs ?? event.time_ms),
          time_beats: args?.timeBeats === null || args?.timeBeats === undefined ? null : Number(args.timeBeats),
          track: (args?.track ?? event.track) as TimelineTrackKind,
          layer_id: args?.layerId === null || args?.layerId === undefined ? null : Number(args.layerId),
          duration_ms: durationMs,
          duration_beats: command === "set_timeline_scene_block" && args?.durationBeats !== null && args?.durationBeats !== undefined ? Number(args.durationBeats) : null,
          conform_to_tempo: conformToTempo, loop_fill: loopFill,
          source_offset_ms: command === "set_timeline_scene_block" ? Math.round(Number(args?.sourceOffsetMs ?? event.source_offset_ms ?? 0)) : 0,
          rate: conformToTempo && !loopFill && authoredBeats !== null && durationMs > 0 ? (authoredBeats * 60_000 / current.clock.bpm) / durationMs : null,
          fade_in_ms: command === "set_timeline_scene_block" ? Math.min(Number(args?.fadeInMs ?? event.fade_in_ms ?? 0), durationMs) : 0,
          fade_out_ms: command === "set_timeline_scene_block" ? Math.min(Number(args?.fadeOutMs ?? event.fade_out_ms ?? 0), durationMs) : 0,
          loop_count: command === "set_timeline_scene_block" ? Number(args?.loopCount ?? event.loop_count) : 1,
          jump_to_event_id: command === "set_timeline_scene_block" && args?.jumpToEventId !== null && args?.jumpToEventId !== undefined ? Number(args.jumpToEventId) : null };
      }) } }));
    return undefined as T;
  }
  if (command === "remove_timeline_scene_block" || command === "remove_timeline_event") {
    const eventId = Number(args?.eventId);
    options.setSnapshot((current) => ({ ...current, timeline: { ...current.timeline,
      events: current.timeline.events.filter((event) => event.id !== eventId) } }));
    return undefined as T;
  }
  return options.invoke<T>(command, args);
};

export const invokeTimelineLayerCommand = async <T,>(
  options: TimelineCommandDispatcherOptions,
  command: FrontendTauriInvokeCommand,
  args?: Record<string, unknown>,
): Promise<T> => {
  const localFixture = options.viewportFixture() === "timeline-layered";
  if (!localFixture && !options.timelineAuthorityReady(command)) {
    throw new Error("Timeline authority is not ready; command was not sent.");
  }
  const childCueId = options.timelineChildCueId();
  if (childCueId !== null) {
    if (command === "add_timeline_layer") {
      const layerId = Math.max(1, ...options.timelineLayers().map((layer) => layer.id)) + 1;
      const kind = String(args?.kind ?? "Lighting") as TimelineLayerSummary["kind"];
      await options.persistChildTimeline(childCueId, (child) => {
        const baseLayers = effectiveTimelineLayers(child.layers);
        return { ...child, layers: [...baseLayers, { id: layerId, label: String(args?.label ?? `${kind} Layer`),
          order: baseLayers.length, muted: false, locked: false, solo: false, expanded: false, kind }] };
      });
      return layerId as T;
    }
    if (command === "update_timeline_layer") {
      const layer = args?.layer as TimelineLayerSummary;
      await options.persistChildTimeline(childCueId, (child) => ({ ...child,
        layers: effectiveTimelineLayers(child.layers).map((candidate) => candidate.id === layer.id ? layer : candidate) }));
      return undefined as T;
    }
    if (command === "remove_timeline_layer") {
      const layerId = Number(args?.layerId);
      const reassignToLayerId = args?.reassignToLayerId === null || args?.reassignToLayerId === undefined ? null : Number(args.reassignToLayerId);
      await options.persistChildTimeline(childCueId, (child) => {
        const baseLayers = effectiveTimelineLayers(child.layers);
        const source = baseLayers.find((layer) => layer.id === layerId);
        if (!source) throw new Error(`Timeline layer ${layerId} was not found`);
        if (source.locked) throw new Error(`Timeline layer ${layerId} is locked; unlock it before removal`);
        if (baseLayers.length <= 1) throw new Error("Timeline must contain at least one layer");
        const events = child.events ?? [];
        const audioClips = child.audio_clips ?? [];
        const hasEvents = events.some((event) => timelineLayerIdForEvent(baseLayers, event) === layerId);
        const hasAudioClips = audioClips.some((clip) => clip.layer_id === layerId);
        if (reassignToLayerId === layerId) throw new Error("Timeline layer cannot be reassigned to itself");
        const target = reassignToLayerId === null ? null : baseLayers.find((layer) => layer.id === reassignToLayerId) ?? null;
        if (reassignToLayerId !== null && !target) throw new Error(`Timeline layer reassignment target ${reassignToLayerId} was not found`);
        if (target?.locked) throw new Error(`Timeline layer ${target.id} is locked; unlock it before reassignment`);
        if (target?.kind === "Audio" && hasEvents) throw new Error(`Cue events cannot be reassigned to Audio timeline layer ${target.id}`);
        if (target && target.kind !== "Audio" && hasAudioClips) throw new Error(`Audio clips can only be reassigned to Audio timeline layer ${target.id}`);
        if (!target && (hasEvents || hasAudioClips)) throw new Error(`Timeline layer ${layerId} is not empty; provide a reassign target`);
        const remainingLayers = baseLayers.filter((layer) => layer.id !== layerId);
        return { ...child, layers: remainingLayers.map((layer, order) => ({ ...layer, order })),
          events: events.map((event) => timelineLayerIdForEvent(baseLayers, event) === layerId && target
            ? { ...event, layer_id: target.id, track: target.kind as TimelineTrackKind } : event),
          audio_clips: audioClips.map((clip) => clip.layer_id === layerId && target?.kind === "Audio" ? { ...clip, layer_id: target.id } : clip) };
      });
      return undefined as T;
    }
    if (command === "reorder_timeline_layers") {
      const orderById = new Map(((args?.layerIds as number[]) ?? []).map((layerId, order) => [layerId, order]));
      await options.persistChildTimeline(childCueId, (child) => ({ ...child,
        layers: effectiveTimelineLayers(child.layers).map((layer) => ({ ...layer, order: orderById.get(layer.id) ?? layer.order })) }));
      return undefined as T;
    }
  }
  if (!localFixture) return options.invoke<T>(command, args);
  if (command === "add_timeline_layer") {
    const layerId = Math.max(1, ...options.timelineLayers().map((layer) => layer.id)) + 1;
    const kind = String(args?.kind ?? "Lighting") as TimelineLayerSummary["kind"];
    options.setSnapshot((current) => ({ ...current, timeline: { ...current.timeline,
      layers: [...(current.timeline.layers ?? []), { id: layerId, label: String(args?.label ?? `${kind} Layer`),
        order: current.timeline.layers?.length ?? 0, muted: false, locked: false, solo: false, expanded: false, kind }] } }));
    return layerId as T;
  }
  if (command === "update_timeline_layer") {
    const layer = args?.layer as TimelineLayerSummary;
    options.setSnapshot((current) => ({ ...current, timeline: { ...current.timeline,
      layers: (current.timeline.layers ?? []).map((candidate) => candidate.id === layer.id ? layer : candidate) } }));
    return undefined as T;
  }
  if (command === "remove_timeline_layer") {
    const layerId = Number(args?.layerId);
    const reassignToLayerId = args?.reassignToLayerId === null || args?.reassignToLayerId === undefined ? null : Number(args.reassignToLayerId);
    const currentLayers = options.timelineLayers();
    const source = currentLayers.find((layer) => layer.id === layerId);
    if (!source) throw new Error(`Timeline layer ${layerId} was not found`);
    if (reassignToLayerId === layerId) throw new Error("Timeline layer cannot be reassigned to itself");
    if (source.locked) throw new Error(`Timeline layer ${layerId} is locked; unlock it before removal`);
    if (currentLayers.length <= 1) throw new Error("Timeline must contain at least one layer");
    const affectedEvents = options.activeTimeline().events.filter((event) => timelineLayerIdForEvent(currentLayers, event) === layerId);
    if (affectedEvents.length > 0 && reassignToLayerId === null) throw new Error(`Timeline layer ${layerId} is not empty; provide a reassign target`);
    const target = reassignToLayerId === null ? null : currentLayers.find((layer) => layer.id === reassignToLayerId) ?? null;
    if (reassignToLayerId !== null && !target) throw new Error(`Timeline layer reassignment target ${reassignToLayerId} was not found`);
    if (target?.locked) throw new Error(`Timeline layer ${target.id} is locked; unlock it before reassignment`);
    if (target?.kind === "Audio" && affectedEvents.length > 0) throw new Error(`Cue events cannot be reassigned to Audio timeline layer ${target.id}`);
    options.setSnapshot((current) => ({ ...current, timeline: { ...current.timeline,
      layers: (current.timeline.layers ?? []).filter((layer) => layer.id !== layerId),
      events: current.timeline.events.map((event) => timelineLayerIdForEvent(currentLayers, event) === layerId && target
        ? { ...event, layer_id: target.id, track: target.kind as TimelineTrackKind } : event) } }));
    return undefined as T;
  }
  if (command === "reorder_timeline_layers") {
    const orderById = new Map(((args?.layerIds as number[]) ?? []).map((layerId, order) => [layerId, order]));
    options.setSnapshot((current) => ({ ...current, timeline: { ...current.timeline,
      layers: (current.timeline.layers ?? []).map((layer) => ({ ...layer, order: orderById.get(layer.id) ?? layer.order })) } }));
    return undefined as T;
  }
  return options.invoke<T>(command, args);
};
