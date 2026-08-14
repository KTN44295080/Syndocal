import type { TimelineEventDraft } from "./editorDrafts";
import type { ProjectFile } from "./types";

const projectRecoveryStorageKey = "syndocal.projectRecovery.v1";
const projectRecoveryEnvelopeVersion = 3;

export interface ProjectRecoveryCheckpoint {
  /** The original project/checkpoint payload format. */
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

/**
 * A browser-local recovery load has crossed the Tauri invoke boundary but its
 * `RecoveryPendingAck` publication may arrive by event, reply, or poll. The
 * checkpoint remains in the same atomic localStorage envelope so a renderer
 * restart can either resume that acknowledgement or safely re-offer A.
 */
export interface ProjectRecoveryIntent {
  version: 1;
  source_serial: number;
  expected_target_serial: number;
  request_generation: number;
  request_id: string;
  checkpoint: ProjectRecoveryCheckpoint;
}

export type ProjectRecoveryStorageState =
  | {
    kind: "checkpoint";
    recovery_authority_serial: number;
    checkpoint: ProjectRecoveryCheckpoint;
  }
  | {
    kind: "tombstone";
    recovery_authority_serial: number;
  }
  | {
    kind: "intent";
    intent: ProjectRecoveryIntent;
  }
  | { kind: "none" };

type ProjectRecoveryEnvelope = Exclude<ProjectRecoveryStorageState, { kind: "none" }> & {
  version: typeof projectRecoveryEnvelopeVersion;
};

const isRecoveryAuthoritySerial = (candidate: unknown): candidate is number =>
  Number.isSafeInteger(candidate) && (candidate as number) >= 0;

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
      source_offset_ms: draft.source_offset_ms ?? 0,
      fade_in_ms: draft.fade_in_ms ?? 0,
      fade_out_ms: draft.fade_out_ms ?? 0,
    };
  }
  return { version: 1, timeline_events: timelineEvents };
};

const isProjectFile = (candidate: unknown): candidate is ProjectFile => {
  if (!candidate || typeof candidate !== "object") return false;
  const source = candidate as Partial<ProjectFile>;
  const snapshot = source.snapshot as Partial<ProjectFile["snapshot"]> | undefined;
  return source.version === 1
    && source.app === "Syndocal"
    && Boolean(snapshot)
    && Array.isArray(snapshot?.fixtures)
    && Array.isArray(snapshot?.cues);
};

/** Parse both a legacy v1 direct checkpoint and a checkpoint nested in v3. */
export const recoveryCheckpointFromUnknown = (candidate: unknown): ProjectRecoveryCheckpoint | null => {
  if (!candidate || typeof candidate !== "object") return null;
  const source = candidate as Partial<ProjectRecoveryCheckpoint>;
  if (source.version !== 1
    || source.app !== "Syndocal"
    || typeof source.saved_at !== "string"
    || typeof source.signature !== "string"
    || !isProjectFile(source.project)) {
    return null;
  }
  const editorDrafts = editorDraftsFromUnknown(source.editor_drafts);
  return {
    version: 1,
    app: "Syndocal",
    saved_at: source.saved_at,
    source_path: typeof source.source_path === "string" && source.source_path.trim()
      ? source.source_path.trim()
      : null,
    signature: source.signature,
    project: source.project,
    ...(editorDrafts ? { editor_drafts: editorDrafts } : {}),
  };
};

const recoveryIntentFromUnknown = (candidate: unknown): ProjectRecoveryIntent | null => {
  if (!candidate || typeof candidate !== "object") return null;
  const source = candidate as Partial<ProjectRecoveryIntent>;
  const checkpoint = recoveryCheckpointFromUnknown(source.checkpoint);
  if (source.version !== 1
    || !isRecoveryAuthoritySerial(source.source_serial)
    || !isRecoveryAuthoritySerial(source.expected_target_serial)
    || source.expected_target_serial !== source.source_serial + 1
    || typeof source.request_generation !== "number"
    || !Number.isSafeInteger(source.request_generation)
    || source.request_generation <= 0
    || typeof source.request_id !== "string"
    || source.request_id.length < 16
    || source.request_id.length > 160
    || !checkpoint) {
    return null;
  }
  return {
    version: 1,
    source_serial: source.source_serial,
    expected_target_serial: source.expected_target_serial,
    request_generation: source.request_generation,
    request_id: source.request_id,
    checkpoint,
  };
};

const recoveryEnvelopeFromUnknown = (candidate: unknown): ProjectRecoveryStorageState | null => {
  if (!candidate || typeof candidate !== "object") return null;
  const source = candidate as Partial<ProjectRecoveryEnvelope>;
  if (source.version !== projectRecoveryEnvelopeVersion || typeof source.kind !== "string") return null;
  if (source.kind === "checkpoint") {
    const checkpoint = recoveryCheckpointFromUnknown(source.checkpoint);
    if (!isRecoveryAuthoritySerial(source.recovery_authority_serial) || !checkpoint) return null;
    return {
      kind: "checkpoint",
      recovery_authority_serial: source.recovery_authority_serial,
      checkpoint,
    };
  }
  if (source.kind === "tombstone") {
    return isRecoveryAuthoritySerial(source.recovery_authority_serial)
      ? { kind: "tombstone", recovery_authority_serial: source.recovery_authority_serial }
      : null;
  }
  if (source.kind === "intent") {
    const intent = recoveryIntentFromUnknown(source.intent);
    return intent ? { kind: "intent", intent } : null;
  }
  return null;
};

const readRecoveryStorage = (): unknown | null => {
  if (typeof window === "undefined") return null;
  try {
    const raw = window.localStorage.getItem(projectRecoveryStorageKey);
    return raw ? JSON.parse(raw) : null;
  } catch {
    try {
      window.localStorage.removeItem(projectRecoveryStorageKey);
    } catch {
      // A restricted storage origin can reject cleanup too; callers still
      // fail closed with no recovery payload.
    }
    return null;
  }
};

const writeRecoveryStorage = (state: ProjectRecoveryStorageState): boolean => {
  if (typeof window === "undefined" || state.kind === "none") return false;
  const envelope: ProjectRecoveryEnvelope = {
    version: projectRecoveryEnvelopeVersion,
    ...state,
  };
  try {
    window.localStorage.setItem(projectRecoveryStorageKey, JSON.stringify(envelope));
    return true;
  } catch {
    return false;
  }
};

/**
 * Read a recovery payload only after the backend exposes its machine-local
 * authority serial. A legacy direct v1 checkpoint has no serial, so it is
 * intentionally eligible only for a fresh journal at serial zero. This makes
 * migration fail closed after any pre-publication invalidation.
 */
export const loadProjectRecoveryStorageState = (
  recoveryAuthoritySerial: number | null,
): ProjectRecoveryStorageState => {
  if (!isRecoveryAuthoritySerial(recoveryAuthoritySerial)) return { kind: "none" };
  const raw = readRecoveryStorage();
  if (raw === null) return { kind: "none" };
  const envelope = recoveryEnvelopeFromUnknown(raw);
  if (envelope) {
    if (envelope.kind === "tombstone") return envelope;
    if (envelope.kind === "checkpoint") {
      return envelope.recovery_authority_serial === recoveryAuthoritySerial
        ? envelope
        : { kind: "none" };
    }
    if (envelope.kind === "none") return envelope;
    return envelope.intent.source_serial === recoveryAuthoritySerial
      || envelope.intent.expected_target_serial === recoveryAuthoritySerial
      || envelope.intent.expected_target_serial + 1 === recoveryAuthoritySerial
      ? envelope
      : { kind: "none" };
  }
  // Legacy v1 direct checkpoint migration: never assume it belongs to a
  // journal which already advanced on this machine.
  const checkpoint = recoveryCheckpointFromUnknown(raw);
  return checkpoint && recoveryAuthoritySerial === 0
    ? { kind: "checkpoint", recovery_authority_serial: 0, checkpoint }
    : { kind: "none" };
};

/** Compatibility convenience for callers which only offer a checkpoint UI. */
export const loadProjectRecoveryCheckpoint = (recoveryAuthoritySerial = 0) => {
  const state = loadProjectRecoveryStorageState(recoveryAuthoritySerial);
  return state.kind === "checkpoint" ? state.checkpoint : null;
};

export const projectRecoveryCheckpointIsTombstoned = () =>
  recoveryEnvelopeFromUnknown(readRecoveryStorage())?.kind === "tombstone";

export const saveProjectRecoveryCheckpoint = (
  checkpoint: ProjectRecoveryCheckpoint,
  recoveryAuthoritySerial = 0,
) => isRecoveryAuthoritySerial(recoveryAuthoritySerial)
  && writeRecoveryStorage({
    kind: "checkpoint",
    recovery_authority_serial: recoveryAuthoritySerial,
    checkpoint,
  });

/** Persist the exact event/reply/poll rendezvous before recovery publication. */
export const registerProjectRecoveryIntent = (
  checkpoint: ProjectRecoveryCheckpoint,
  sourceSerial: number,
  requestGeneration: number,
): ProjectRecoveryIntent | null => {
  if (!isRecoveryAuthoritySerial(sourceSerial)
    || sourceSerial >= Number.MAX_SAFE_INTEGER
    || !Number.isSafeInteger(requestGeneration)
    || requestGeneration <= 0
    || typeof globalThis.crypto?.randomUUID !== "function") {
    return null;
  }
  const intent: ProjectRecoveryIntent = {
    version: 1,
    source_serial: sourceSerial,
    expected_target_serial: sourceSerial + 1,
    request_generation: requestGeneration,
    request_id: globalThis.crypto.randomUUID(),
    checkpoint,
  };
  return writeRecoveryStorage({ kind: "intent", intent }) ? intent : null;
};

/** Atomically hide a stale checkpoint after an unsaved authority replacement. */
export const tombstoneProjectRecoveryCheckpoint = (recoveryAuthoritySerial: number) =>
  isRecoveryAuthoritySerial(recoveryAuthoritySerial)
  && writeRecoveryStorage({
    kind: "tombstone",
    recovery_authority_serial: recoveryAuthoritySerial,
  });

export const clearProjectRecoveryCheckpoint = () => {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.removeItem(projectRecoveryStorageKey);
  } catch {
    // Recovery storage is optional; the backend remains authoritative.
  }
};

export const projectRecoverySourceLabel = (checkpoint: ProjectRecoveryCheckpoint) => {
  if (!checkpoint.source_path) return "Untitled.sdc";
  const normalized = checkpoint.source_path.replaceAll("\\", "/");
  return normalized.split("/").pop()?.trim() || checkpoint.source_path;
};

export const projectRecoveryTimeLabel = (checkpoint: ProjectRecoveryCheckpoint) => {
  const date = new Date(checkpoint.saved_at);
  if (Number.isNaN(date.getTime())) return "Recovery";
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
