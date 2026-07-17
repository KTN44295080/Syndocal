import type { ProjectFile } from "./types";
import type { TimelineEventDraft } from "./editorDrafts";

const projectRecoveryStorageKey = "syndocal.projectRecovery.v1";

export interface ProjectRecoveryCheckpoint {
  version: 1;
  app: "Syndocal";
  saved_at: string;
  source_path: string | null;
  signature: string;
  project: ProjectFile;
  editor_drafts?: {
    version: 1;
    timeline_events: Record<number, TimelineEventDraft>;
  };
}

const isTimelineEventDraft = (candidate: unknown): candidate is TimelineEventDraft => {
  if (!candidate || typeof candidate !== "object") return false;
  const draft = candidate as Partial<TimelineEventDraft>;
  return Number.isFinite(draft.cue_id) &&
    Number.isFinite(draft.time_ms) &&
    (draft.track === "Lighting" || draft.track === "Video") &&
    (draft.layer_id === undefined || draft.layer_id === null ||
      (Number.isInteger(draft.layer_id) && draft.layer_id >= 0)) &&
    Number.isFinite(draft.duration_ms) &&
    Number.isFinite(draft.loop_count) &&
    (draft.jump_to_event_id === null || Number.isFinite(draft.jump_to_event_id));
};

const editorDraftsFromUnknown = (candidate: unknown): ProjectRecoveryCheckpoint["editor_drafts"] => {
  if (!candidate || typeof candidate !== "object") return undefined;
  const source = candidate as { version?: unknown; timeline_events?: unknown };
  if (source.version !== 1 || !source.timeline_events || typeof source.timeline_events !== "object") {
    return undefined;
  }
  const timelineEvents: Record<number, TimelineEventDraft> = {};
  for (const [eventId, draft] of Object.entries(source.timeline_events)) {
    const numericEventId = Number(eventId);
    if (!Number.isInteger(numericEventId) || numericEventId <= 0 || !isTimelineEventDraft(draft)) continue;
    timelineEvents[numericEventId] = {
      ...draft,
      time_beats: draft.time_beats ?? null,
      layer_id: draft.layer_id ?? null,
      duration_beats: draft.duration_beats ?? null,
      conform_to_tempo: draft.conform_to_tempo ?? false,
      loop_fill: draft.loop_fill ?? false,
    };
  }
  return { version: 1, timeline_events: timelineEvents };
};

const isProjectFile = (candidate: unknown): candidate is ProjectFile => {
  if (!candidate || typeof candidate !== "object") {
    return false;
  }
  const source = candidate as Partial<ProjectFile>;
  const snapshot = source.snapshot as Partial<ProjectFile["snapshot"]> | undefined;
  return (
    source.version === 1 &&
    source.app === "Syndocal" &&
    Boolean(snapshot) &&
    Array.isArray(snapshot?.fixtures) &&
    Array.isArray(snapshot?.cues)
  );
};

export const projectRecoverySourceLabel = (checkpoint: ProjectRecoveryCheckpoint) => {
  if (!checkpoint.source_path) {
    return "Untitled.sdc";
  }
  const normalized = checkpoint.source_path.replaceAll("\\", "/");
  return normalized.split("/").pop()?.trim() || checkpoint.source_path;
};

export const projectRecoveryTimeLabel = (checkpoint: ProjectRecoveryCheckpoint) => {
  const date = new Date(checkpoint.saved_at);
  if (Number.isNaN(date.getTime())) {
    return "Recovery";
  }
  return date.toLocaleString(undefined, {
    month: "short",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
};

export const createProjectRecoveryCheckpoint = (
  project: ProjectFile,
  sourcePath: string | null,
  signature: string,
  timelineEventDrafts?: Record<number, TimelineEventDraft>,
): ProjectRecoveryCheckpoint => ({
  version: 1,
  app: "Syndocal",
  saved_at: new Date().toISOString(),
  source_path: sourcePath,
  signature,
  project,
  ...(timelineEventDrafts && Object.keys(timelineEventDrafts).length > 0
    ? { editor_drafts: { version: 1 as const, timeline_events: timelineEventDrafts } }
    : {}),
});

export const recoveryCheckpointFromUnknown = (candidate: unknown): ProjectRecoveryCheckpoint | null => {
  if (!candidate || typeof candidate !== "object") {
    return null;
  }
  const source = candidate as Partial<ProjectRecoveryCheckpoint>;
  if (
    source.version !== 1 ||
    source.app !== "Syndocal" ||
    typeof source.saved_at !== "string" ||
    typeof source.signature !== "string" ||
    !isProjectFile(source.project)
  ) {
    return null;
  }
  const editorDrafts = editorDraftsFromUnknown(source.editor_drafts);
  return {
    version: 1,
    app: "Syndocal",
    saved_at: source.saved_at,
    source_path: typeof source.source_path === "string" && source.source_path.trim() ? source.source_path.trim() : null,
    signature: source.signature,
    project: source.project,
    ...(editorDrafts ? { editor_drafts: editorDrafts } : {}),
  };
};

export const loadProjectRecoveryCheckpoint = () => {
  if (typeof window === "undefined") {
    return null;
  }
  try {
    const raw = window.localStorage.getItem(projectRecoveryStorageKey);
    if (!raw) {
      return null;
    }
    const checkpoint = recoveryCheckpointFromUnknown(JSON.parse(raw));
    if (!checkpoint) {
      window.localStorage.removeItem(projectRecoveryStorageKey);
    }
    return checkpoint;
  } catch {
    try {
      window.localStorage.removeItem(projectRecoveryStorageKey);
    } catch {
      // Storage can be unavailable; recovery remains absent for this session.
    }
    return null;
  }
};

export const saveProjectRecoveryCheckpoint = (checkpoint: ProjectRecoveryCheckpoint) => {
  if (typeof window === "undefined") {
    return false;
  }
  try {
    window.localStorage.setItem(projectRecoveryStorageKey, JSON.stringify(checkpoint));
    return true;
  } catch {
    // localStorage can be unavailable or full; the explicit project-save path remains authoritative.
    return false;
  }
};

export const clearProjectRecoveryCheckpoint = () => {
  if (typeof window === "undefined") {
    return;
  }
  try {
    window.localStorage.removeItem(projectRecoveryStorageKey);
  } catch {
    // Ignore storage cleanup failures.
  }
};
