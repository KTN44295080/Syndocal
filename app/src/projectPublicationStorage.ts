import type {
  ProjectAuthorityBundle,
  ProjectBackupSummary,
  ProjectPublicationRequestV1,
  ProjectPublicationStateV1,
  ProjectPublicationStatusV1,
  ProjectPublicationSurfaceV1,
  ProjectPublicationTargetPolicyV1,
} from "./types";
import { projectAuthorityBundleTimelineRuntimeFromUnknown } from "./timelineRuntimeSnapshotWire";

const projectPublicationStorageKey = "syndocal.projectPublication.v1";
const projectPublicationStorageVersion = 1;
const projectPublicationRequestSchemaVersion = 1 as const;
const maxSafePublicationNumber = Number.MAX_SAFE_INTEGER;

type UnknownRecord = Record<string, unknown>;

const isRecord = (value: unknown): value is UnknownRecord =>
  typeof value === "object" && value !== null && !Array.isArray(value);

const hasExactKeys = (
  value: UnknownRecord,
  required: readonly string[],
  optional: readonly string[] = [],
): boolean => {
  const allowed = new Set([...required, ...optional]);
  return required.every((key) => Object.prototype.hasOwnProperty.call(value, key))
    && Object.keys(value).every((key) => allowed.has(key));
};

const utf8Length = (value: string) => new TextEncoder().encode(value).length;

export type ProjectPublicationRequestSeedV1 = Omit<
  ProjectPublicationRequestV1,
  "schemaVersion" | "originId" | "requestId"
>;

export interface ProjectPublicationIntentV1 {
  version: 1;
  request: ProjectPublicationRequestV1;
}

type ProjectPublicationStorageEnvelopeV1 = {
  version: 1;
  origin_id: string;
  next_request_id: number;
  intent: ProjectPublicationIntentV1 | null;
  acknowledgement: ProjectPublicationRequestV1 | null;
};

const isSafePositiveInteger = (value: unknown): value is number =>
  typeof value === "number" && Number.isSafeInteger(value) && value > 0;

const isSafeNonNegativeInteger = (value: unknown): value is number =>
  typeof value === "number" && Number.isSafeInteger(value) && value >= 0;

const isSha256 = (value: unknown): value is string =>
  typeof value === "string" && /^[0-9a-f]{64}$/.test(value);

const isOriginId = (value: unknown): value is string =>
  typeof value === "string" && /^[A-Za-z0-9_-]{1,96}$/.test(value);

const isCanonicalOwnerId = (value: unknown): value is string =>
  typeof value === "string"
  && utf8Length(value) >= 1
  && utf8Length(value) <= 128
  && value === value.trim()
  && /^[A-Za-z0-9_.:-]+$/.test(value);

const isSurface = (value: unknown): value is ProjectPublicationSurfaceV1 =>
  value === "save" || value === "save_as" || value === "user_template" || value === "backup";

const isTargetPolicy = (value: unknown): value is ProjectPublicationTargetPolicyV1 =>
  value === "current_or_dialog" || value === "dialog" || value === "managed_unique";

const isPublicationState = (value: unknown): value is ProjectPublicationStateV1 =>
  value === "reserved" || value === "selecting" || value === "selected" || value === "prepared"
  || value === "succeeded" || value === "cancelled" || value === "abandoned" || value === "failed"
  || value === "indeterminate";

const targetPolicyForSurface = (surface: ProjectPublicationSurfaceV1): ProjectPublicationTargetPolicyV1 => {
  switch (surface) {
    case "save": return "current_or_dialog";
    case "save_as":
    case "user_template": return "dialog";
    case "backup": return "managed_unique";
  }
};

/** Mirrors the backend V1 request normalizer before the intent reaches IPC. */
export const normalizeProjectPublicationReason = (reason: string | null): string | null => {
  if (reason === null) return null;
  const characters = Array.from(reason.trim()).slice(0, 80);
  while (characters.length > 0 && utf8Length(characters.join("")) > 160) characters.pop();
  return characters.join("") || null;
};

export const projectPublicationRequestFromUnknown = (candidate: unknown): ProjectPublicationRequestV1 | null => {
  if (!isRecord(candidate) || !hasExactKeys(candidate, [
    "schemaVersion",
    "originId",
    "requestId",
    "ownerId",
    "surface",
    "expectedProjectEpoch",
    "expectedProjectRevision",
    "expectedCheckpointHash",
    "mappingAuthorityHash",
    "sourcePath",
    "reason",
    "targetPolicy",
  ])) return null;
  const source = candidate as unknown as Partial<ProjectPublicationRequestV1>;
  if (source.schemaVersion !== projectPublicationRequestSchemaVersion
    || !isOriginId(source.originId)
    || !isSafePositiveInteger(source.requestId)
    || !isCanonicalOwnerId(source.ownerId)
    || !isSurface(source.surface)
    || !isSafeNonNegativeInteger(source.expectedProjectEpoch)
    || !isSafeNonNegativeInteger(source.expectedProjectRevision)
    || !isSha256(source.expectedCheckpointHash)
    || !isSha256(source.mappingAuthorityHash)
    || !(source.sourcePath === null || typeof source.sourcePath === "string" && utf8Length(source.sourcePath) <= 4096)
    || !(source.reason === null || typeof source.reason === "string")
    || source.reason !== normalizeProjectPublicationReason(source.reason)
    || !isTargetPolicy(source.targetPolicy)
    || source.targetPolicy !== targetPolicyForSurface(source.surface)
    || source.mappingAuthorityHash !== source.expectedCheckpointHash) {
    return null;
  }
  return {
    schemaVersion: projectPublicationRequestSchemaVersion,
    originId: source.originId,
    requestId: source.requestId,
    ownerId: source.ownerId,
    surface: source.surface,
    expectedProjectEpoch: source.expectedProjectEpoch,
    expectedProjectRevision: source.expectedProjectRevision,
    expectedCheckpointHash: source.expectedCheckpointHash,
    mappingAuthorityHash: source.mappingAuthorityHash,
    sourcePath: source.sourcePath,
    reason: source.reason,
    targetPolicy: source.targetPolicy,
  };
};

// The receipt identity must be exact, but do not use JSON field order as part
// of that test: Tauri/Rust may serialize the same camelCase object in a
// different key order from localStorage.
export const projectPublicationRequestMatches = (left: ProjectPublicationRequestV1, right: ProjectPublicationRequestV1) =>
  left.schemaVersion === right.schemaVersion
  && left.originId === right.originId
  && left.requestId === right.requestId
  && left.ownerId === right.ownerId
  && left.surface === right.surface
  && left.expectedProjectEpoch === right.expectedProjectEpoch
  && left.expectedProjectRevision === right.expectedProjectRevision
  && left.expectedCheckpointHash === right.expectedCheckpointHash
  && left.mappingAuthorityHash === right.mappingAuthorityHash
  && left.sourcePath === right.sourcePath
  && left.reason === right.reason
  && left.targetPolicy === right.targetPolicy;

const projectPublicationRequestMatchesExceptOwner = (
  left: ProjectPublicationRequestV1,
  right: ProjectPublicationRequestV1,
) => projectPublicationRequestMatches({ ...left, ownerId: right.ownerId }, right);

const stringOrNull = (value: unknown): value is string | null => value === null || typeof value === "string";

const optionalSafeIntegerOrNull = (value: unknown): boolean =>
  value === null || value === undefined || isSafeNonNegativeInteger(value);

const optionalSha256OrNull = (value: unknown): boolean =>
  value === null || value === undefined || isSha256(value);

const projectBackupSummaryFromUnknown = (candidate: unknown): ProjectBackupSummary | null => {
  if (!isRecord(candidate) || !hasExactKeys(candidate, [
    "id",
    "created_at_unix_ms",
    "source_path",
    "reason",
    "bytes",
  ])) return null;
  if (!isSafeNonNegativeInteger(candidate.id)
    || !isSafeNonNegativeInteger(candidate.created_at_unix_ms)
    || !stringOrNull(candidate.source_path)
    || typeof candidate.reason !== "string"
    || candidate.reason.length === 0
    || utf8Length(candidate.reason) > 160
    || !isSafeNonNegativeInteger(candidate.bytes)) return null;
  return candidate as unknown as ProjectBackupSummary;
};

const projectRecoveryTransitionIsValid = (candidate: unknown): boolean => {
  if (!isRecord(candidate) || typeof candidate.kind !== "string") return false;
  switch (candidate.kind) {
    case "legacy_unknown":
    case "project_publication":
    case "history_navigation":
    case "clean_save":
      return hasExactKeys(candidate, ["kind"]);
    case "recovery_publication":
      return hasExactKeys(candidate, ["kind", "source_serial", "request_id", "target_checkpoint_hash"])
        && isSafeNonNegativeInteger(candidate.source_serial)
        && typeof candidate.request_id === "string"
        && (candidate.target_checkpoint_hash === "" || isSha256(candidate.target_checkpoint_hash));
    case "recovery_acknowledged":
      return hasExactKeys(candidate, ["kind", "recovery_publication_serial", "request_id", "target_checkpoint_hash"])
        && isSafeNonNegativeInteger(candidate.recovery_publication_serial)
        && typeof candidate.request_id === "string"
        && (candidate.target_checkpoint_hash === "" || isSha256(candidate.target_checkpoint_hash));
    default:
      return false;
  }
};

const projectHistoryStatusIsValid = (candidate: unknown): boolean => {
  if (!isRecord(candidate) || !hasExactKeys(candidate, [
    "can_undo",
    "can_redo",
    "undo_depth",
    "redo_depth",
    "undo_label",
    "redo_label",
    "project_epoch",
    "project_revision",
    "checkpoint_hash",
    "history_generation",
    "undo_entry_id",
    "undo_checkpoint_hash",
    "redo_entry_id",
    "redo_checkpoint_hash",
  ])) return false;
  return typeof candidate.can_undo === "boolean"
    && typeof candidate.can_redo === "boolean"
    && isSafeNonNegativeInteger(candidate.undo_depth)
    && isSafeNonNegativeInteger(candidate.redo_depth)
    && stringOrNull(candidate.undo_label)
    && stringOrNull(candidate.redo_label)
    && isSafeNonNegativeInteger(candidate.project_epoch)
    && isSafeNonNegativeInteger(candidate.project_revision)
    && isSha256(candidate.checkpoint_hash)
    && isSafeNonNegativeInteger(candidate.history_generation)
    && optionalSafeIntegerOrNull(candidate.undo_entry_id)
    && optionalSha256OrNull(candidate.undo_checkpoint_hash)
    && optionalSafeIntegerOrNull(candidate.redo_entry_id)
    && optionalSha256OrNull(candidate.redo_checkpoint_hash);
};

const projectInputRuntimeIsValid = (candidate: unknown): boolean => {
  if (!isRecord(candidate) || !hasExactKeys(candidate, [
    "project_input_runtime_generation",
    "mapping_input_runtime_generation",
    "midi_clock_active",
    "midi_control_active",
    "midi_feedback_output_active",
    "midi_feedback_runtime_active",
    "osc_active",
    "dmx_active",
  ])) return false;
  return isSafeNonNegativeInteger(candidate.project_input_runtime_generation)
    && isSafeNonNegativeInteger(candidate.mapping_input_runtime_generation)
    && [
      candidate.midi_clock_active,
      candidate.midi_control_active,
      candidate.midi_feedback_output_active,
      candidate.midi_feedback_runtime_active,
      candidate.osc_active,
      candidate.dmx_active,
    ].every((value) => typeof value === "boolean");
};

const operatorPolicyIsValid = (candidate: unknown): boolean => {
  if (candidate === null) return true;
  if (!isRecord(candidate) || !hasExactKeys(candidate, ["lock_mode", "lock_on_load", "credential"])
    || (candidate.lock_mode !== "Full" && candidate.lock_mode !== "Partial")
    || typeof candidate.lock_on_load !== "boolean"
    || !isRecord(candidate.credential)
    || !hasExactKeys(candidate.credential, ["scheme", "iterations", "salt_b64", "verifier_b64"])) return false;
  return candidate.credential.scheme === "PBKDF2-SHA256"
    && isSafePositiveInteger(candidate.credential.iterations)
    && typeof candidate.credential.salt_b64 === "string"
    && typeof candidate.credential.verifier_b64 === "string";
};

const recordArray = (value: unknown): boolean => Array.isArray(value) && value.every(isRecord);

const engineSnapshotHasRequiredShape = (candidate: unknown): boolean => {
  if (!isRecord(candidate)) return false;
  return [
    "fixtures", "cues", "cue_lists", "palettes", "playback_executors", "effects", "node_graphs",
    "dmx_outputs", "submasters", "stage_map_presets", "stage_objects", "dmx_preview", "dmx_previews",
  ].every((key) => Array.isArray(candidate[key]))
    && ["programmer", "timeline", "video", "output", "clock", "stage_map", "telemetry"]
      .every((key) => isRecord(candidate[key]))
    && typeof candidate.playback_master === "number" && Number.isFinite(candidate.playback_master)
    && typeof candidate.lighting_master === "number" && Number.isFinite(candidate.lighting_master)
    && typeof candidate.blackout === "boolean";
};

const projectAuthorityBundleFromUnknown = (candidate: unknown): ProjectAuthorityBundle | null => {
  if (!isRecord(candidate) || !hasExactKeys(candidate, [
    "project_epoch",
    "project_revision",
    "checkpoint_hash",
    "timeline_transport_epoch",
    "timeline_transport_generation",
    "timeline_runtime",
    "publication_generation",
    "publication_kind",
    "mapping_replacement_generation",
    "authority_disposition_generation",
    "authority_disposition",
    "recovery_authority_serial",
    "recovery_authority_last_transition",
    "path_generation",
    "history_generation",
    "current_project_path",
    "snapshot",
    "profiles",
    "fixture_groups",
    "operator_policy",
    "midi_mappings",
    "osc_mappings",
    "dmx_mappings",
    "dj_track_triggers",
    "history",
    "input_runtime",
  ])) return null;
  const safeNumbers = [
    candidate.project_epoch,
    candidate.project_revision,
    candidate.timeline_transport_epoch,
    candidate.timeline_transport_generation,
    candidate.publication_generation,
    candidate.mapping_replacement_generation,
    candidate.authority_disposition_generation,
    candidate.recovery_authority_serial,
    candidate.path_generation,
    candidate.history_generation,
  ];
  const timelineRuntime = projectAuthorityBundleTimelineRuntimeFromUnknown(candidate);
  if (!safeNumbers.every(isSafeNonNegativeInteger)
    || !isSha256(candidate.checkpoint_hash)
    || timelineRuntime === null
    || !["runtime_status", "mutation", "identity_replacement", "history_navigation"].includes(String(candidate.publication_kind))
    || !["clean_at_path", "unsaved_replacement", "recovery_pending_ack", "history_navigation", "runtime_sanitize"].includes(String(candidate.authority_disposition))
    || !projectRecoveryTransitionIsValid(candidate.recovery_authority_last_transition)
    || !stringOrNull(candidate.current_project_path)
    || !engineSnapshotHasRequiredShape(candidate.snapshot)
    || !recordArray(candidate.profiles)
    || !recordArray(candidate.fixture_groups)
    || !operatorPolicyIsValid(candidate.operator_policy)
    || !recordArray(candidate.midi_mappings)
    || !recordArray(candidate.osc_mappings)
    || !recordArray(candidate.dmx_mappings)
    || !recordArray(candidate.dj_track_triggers)
    || !projectHistoryStatusIsValid(candidate.history)
    || !projectInputRuntimeIsValid(candidate.input_runtime)) return null;
  const history = candidate.history as UnknownRecord;
  if (history.project_epoch !== candidate.project_epoch
    || history.project_revision !== candidate.project_revision
    || history.checkpoint_hash !== candidate.checkpoint_hash
    || history.history_generation !== candidate.history_generation) return null;
  return candidate as unknown as ProjectAuthorityBundle;
};

const projectPublicationTargetPathIsValid = (
  surface: ProjectPublicationSurfaceV1,
  targetPath: string,
): boolean => {
  const expectedExtension = surface === "user_template" ? ".sdctemplate"
    : surface === "backup" ? ".json" : ".sdc";
  return targetPath.endsWith(expectedExtension);
};

/** Validate the complete V1 receipt before any UI application or ACK mutation. */
export const projectPublicationStatusFromUnknown = (candidate: unknown): ProjectPublicationStatusV1 | null => {
  if (!isRecord(candidate) || !hasExactKeys(candidate, [
    "request",
    "shapeHash",
    "surface",
    "state",
    "targetPath",
    "backup",
    "recoveryAuthoritySerial",
    "authority",
    "error",
  ], ["artifactDigest", "warning"])) return null;
  const request = projectPublicationRequestFromUnknown(candidate.request);
  const authority = candidate.authority === null ? null : projectAuthorityBundleFromUnknown(candidate.authority);
  const backup = candidate.backup === null ? null : projectBackupSummaryFromUnknown(candidate.backup);
  if (!request
    || !isSha256(candidate.shapeHash)
    || !isSurface(candidate.surface)
    || candidate.surface !== request.surface
    || !isPublicationState(candidate.state)
    || !stringOrNull(candidate.targetPath)
    || !(candidate.artifactDigest === undefined || isSha256(candidate.artifactDigest))
    || (candidate.backup !== null && !backup)
    || !isSafeNonNegativeInteger(candidate.recoveryAuthoritySerial)
    || (candidate.authority !== null && !authority)
    || !stringOrNull(candidate.error)
    || !(candidate.warning === undefined
      || typeof candidate.warning === "string" && candidate.warning.length > 0 && utf8Length(candidate.warning) <= 512)) return null;

  const noArtifact = candidate.artifactDigest === undefined;
  const noWarning = candidate.warning === undefined;
  const hasTarget = typeof candidate.targetPath === "string" && candidate.targetPath.length > 0;
  if (hasTarget && !projectPublicationTargetPathIsValid(request.surface, candidate.targetPath as string)) return null;
  const hasError = typeof candidate.error === "string" && candidate.error.length > 0;
  switch (candidate.state) {
    case "reserved":
    case "selecting":
      if (candidate.state === "selecting" && request.targetPolicy === "managed_unique") return null;
      if (candidate.targetPath !== null || !noArtifact || backup || authority || candidate.error !== null || !noWarning) return null;
      break;
    case "selected":
      if (!hasTarget || !noArtifact || backup || authority || candidate.error !== null || !noWarning) return null;
      break;
    case "prepared":
      if (!hasTarget || noArtifact || backup || authority || !(candidate.error === null || hasError) || !noWarning) return null;
      break;
    case "succeeded":
      if (!hasTarget || noArtifact || candidate.error !== null) return null;
      if (candidate.surface === "backup") {
        if (!backup || authority) return null;
      } else if (backup || candidate.warning !== undefined || (candidate.surface === "user_template" && authority)) {
        return null;
      }
      break;
    case "cancelled":
    case "abandoned":
      if (candidate.targetPath !== null || !noArtifact || backup || authority || candidate.error !== null || !noWarning) return null;
      break;
    case "failed":
      if (candidate.targetPath !== null || !noArtifact || backup || authority || !hasError || !noWarning) return null;
      break;
    case "indeterminate":
      if (!noArtifact || backup || authority || !hasError || !noWarning) return null;
      break;
  }
  if (authority && (authority.project_epoch !== request.expectedProjectEpoch
    || authority.project_revision !== request.expectedProjectRevision
    || authority.checkpoint_hash !== request.expectedCheckpointHash
    || authority.recovery_authority_serial !== candidate.recoveryAuthoritySerial)) return null;
  return {
    request,
    shapeHash: candidate.shapeHash,
    surface: candidate.surface,
    state: candidate.state,
    targetPath: candidate.targetPath,
    ...(candidate.artifactDigest === undefined ? {} : { artifactDigest: candidate.artifactDigest }),
    backup,
    recoveryAuthoritySerial: candidate.recoveryAuthoritySerial,
    authority,
    error: candidate.error,
    ...(candidate.warning === undefined ? {} : { warning: candidate.warning }),
  };
};

/** Parse a native/event reply and bind it to the exact durable request. */
export const projectPublicationStatusForRequestFromUnknown = (
  candidate: unknown,
  expectedRequest: ProjectPublicationRequestV1,
): ProjectPublicationStatusV1 | null => {
  const status = projectPublicationStatusFromUnknown(candidate);
  return status && projectPublicationRequestMatches(expectedRequest, status.request) ? status : null;
};

export const projectPublicationStatusIsTerminal = (state: ProjectPublicationStateV1): boolean =>
  state === "succeeded" || state === "cancelled" || state === "abandoned" || state === "failed";

/**
 * Shared production settlement ordering. UI application completes first, then
 * the exact request moves durably into the ACK queue, and only then may native
 * ACK be invoked. Any storage or lost-reply failure leaves a retryable record.
 */
export const settleProjectPublicationStatusV1 = async (
  status: ProjectPublicationStatusV1,
  applyTerminal: (status: ProjectPublicationStatusV1) => Promise<void>,
  acknowledgeTerminal: (request: ProjectPublicationRequestV1) => Promise<void>,
): Promise<void> => {
  await applyTerminal(status);
  if (!projectPublicationStatusIsTerminal(status.state)) return;
  if (!queueProjectPublicationAcknowledgement(status.request)) {
    throw new Error("The project publication intent was not available for durable acknowledgement.");
  }
  await acknowledgeTerminal(status.request);
};

export type ProjectPublicationCommandV1 =
  | "save_project_v1"
  | "save_project_as_v1"
  | "save_user_template_v1"
  | "save_project_backup_v1";

export type ProjectPublicationStatusCommandV1 = ProjectPublicationCommandV1
  | "get_project_publication_receipt_v1"
  | "abandon_project_publication_v1";

export type ProjectPublicationAdoptionCommandV1 = "adopt_project_publication_owner_v1";

export interface ProjectPublicationControllerV1Options {
  invokeStatus: (
    command: ProjectPublicationStatusCommandV1,
    request: ProjectPublicationRequestV1,
  ) => Promise<unknown>;
  adoptOwner: (
    request: ProjectPublicationRequestV1,
    newOwnerId: string,
  ) => Promise<unknown>;
  currentOwnerId: string;
  publishQueuedAcknowledgement: () => Promise<void>;
  settleTerminal: (status: ProjectPublicationStatusV1) => Promise<void>;
  applyIndeterminate: (status: ProjectPublicationStatusV1) => Promise<void>;
  captureSeed: () => Promise<ProjectPublicationRequestSeedV1>;
  ensureMutationAllowed: (
    command: ProjectPublicationCommandV1
      | ProjectPublicationAdoptionCommandV1
      | "abandon_project_publication_v1",
  ) => void;
  confirmAbandon: (
    intent: ProjectPublicationIntentV1,
    status: ProjectPublicationStatusV1,
    requestedSurface: ProjectPublicationSurfaceV1,
  ) => Promise<boolean> | boolean;
  onPending: (status: ProjectPublicationStatusV1) => void;
  onMissing: (intent: ProjectPublicationIntentV1) => void;
  onAbandoned: (
    intent: ProjectPublicationIntentV1,
    requestedSurface: ProjectPublicationSurfaceV1,
  ) => void;
}

const publicationCommandForSurface = (surface: ProjectPublicationSurfaceV1): ProjectPublicationCommandV1 => {
  switch (surface) {
    case "save": return "save_project_v1";
    case "save_as": return "save_project_as_v1";
    case "user_template": return "save_user_template_v1";
    case "backup": return "save_project_backup_v1";
  }
};

const publicationStatusRank = (state: ProjectPublicationStateV1): number => {
  switch (state) {
    case "reserved": return 0;
    case "selecting": return 1;
    case "selected": return 2;
    case "prepared": return 3;
    case "succeeded":
    case "cancelled":
    case "abandoned":
    case "failed":
    case "indeterminate": return 4;
  }
};

const publicationObservationKey = (request: ProjectPublicationRequestV1) =>
  `${request.originId}:${request.requestId}`;

/**
 * Actual renderer publication state machine used by App and the focused E4
 * checker. Native replies and optional event observations share one exact
 * parser/rank path, so reply/event reordering cannot regress Selecting or
 * reopen its dialog through a stale lower-phase observation.
 */
export const createProjectPublicationControllerV1 = (
  options: ProjectPublicationControllerV1Options,
) => {
  const observations = new Map<string, ProjectPublicationStatusV1>();
  if (!isCanonicalOwnerId(options.currentOwnerId)) {
    throw new Error("Current project publication owner ID is not canonical.");
  }

  const recordStatus = (
    candidate: unknown,
    expectedRequest: ProjectPublicationRequestV1,
  ): ProjectPublicationStatusV1 => {
    const parsed = projectPublicationStatusForRequestFromUnknown(candidate, expectedRequest);
    if (!parsed) throw new Error("Backend returned a malformed or unsupported project publication receipt.");
    const key = publicationObservationKey(expectedRequest);
    const previous = observations.get(key);
    if (!previous || publicationStatusRank(parsed.state) >= publicationStatusRank(previous.state)) {
      observations.set(key, parsed);
    }
    if (observations.size > 64) observations.delete(observations.keys().next().value as string);
    return observations.get(key) as ProjectPublicationStatusV1;
  };

  const observeStatus = (
    candidate: unknown,
    expectedRequest: ProjectPublicationRequestV1,
  ): ProjectPublicationStatusV1 => recordStatus(candidate, expectedRequest);

  const invokeParsed = async (
    command: ProjectPublicationStatusCommandV1,
    request: ProjectPublicationRequestV1,
    allowMissing = false,
  ): Promise<ProjectPublicationStatusV1 | null> => {
    const reply = await options.invokeStatus(command, request);
    if (reply === null && allowMissing) {
      return observations.get(publicationObservationKey(request)) ?? null;
    }
    return recordStatus(reply, request);
  };

  const ownerAdoptionState = (state: ProjectPublicationStateV1) =>
    state === "reserved" || state === "selecting" || state === "selected" || state === "prepared";

  const persistOwnerAdoption = (
    intent: ProjectPublicationIntentV1,
    status: ProjectPublicationStatusV1,
  ): ProjectPublicationIntentV1 => {
    if (status.request.ownerId !== options.currentOwnerId
      || !projectPublicationRequestMatchesExceptOwner(intent.request, status.request)
      || !adoptProjectPublicationIntentOwner(intent.request, status.request)) {
      throw new Error("The adopted project publication owner could not be durably recorded.");
    }
    return { version: 1, request: status.request };
  };

  const queryExisting = async (
    intent: ProjectPublicationIntentV1,
  ): Promise<{ intent: ProjectPublicationIntentV1; status: ProjectPublicationStatusV1 | null }> => {
    const reply = await options.invokeStatus("get_project_publication_receipt_v1", intent.request);
    const candidate = reply === null
      ? observations.get(publicationObservationKey(intent.request)) ?? null
      : projectPublicationStatusFromUnknown(reply);
    if (reply !== null && candidate === null) {
      throw new Error("Backend returned a malformed or unsupported project publication receipt.");
    }
    if (candidate === null) return { intent, status: null };
    if (projectPublicationRequestMatches(candidate.request, intent.request)) {
      return { intent, status: recordStatus(candidate, intent.request) };
    }
    // An adopt reply may be lost after the backend commits. An exact query of
    // the old durable key is then allowed to return the same request under the
    // newly registered current owner. No other request delta may converge.
    if (intent.request.ownerId === options.currentOwnerId
      || candidate.request.ownerId !== options.currentOwnerId
      || !projectPublicationRequestMatchesExceptOwner(intent.request, candidate.request)) {
      throw new Error("Backend returned a stale or mismatched project publication owner receipt.");
    }
    const adopted = recordStatus(candidate, candidate.request);
    return { intent: persistOwnerAdoption(intent, adopted), status: adopted };
  };

  const adoptOwnerIfRequired = async (
    intent: ProjectPublicationIntentV1,
    status: ProjectPublicationStatusV1,
  ): Promise<{ intent: ProjectPublicationIntentV1; status: ProjectPublicationStatusV1 }> => {
    if (intent.request.ownerId === options.currentOwnerId || !ownerAdoptionState(status.state)) {
      return { intent, status };
    }
    options.ensureMutationAllowed("adopt_project_publication_owner_v1");
    const expectedRequest = { ...intent.request, ownerId: options.currentOwnerId };
    const reply = await options.adoptOwner(intent.request, options.currentOwnerId);
    const adopted = recordStatus(reply, expectedRequest);
    if (!ownerAdoptionState(adopted.state) || adopted.shapeHash === status.shapeHash) {
      throw new Error("Backend returned a stale or terminal receipt from a pending owner adoption.");
    }
    return { intent: persistOwnerAdoption(intent, adopted), status: adopted };
  };

  const settleObserved = async (
    status: ProjectPublicationStatusV1,
    deferTerminalAcknowledgement: boolean,
  ) => {
    if (projectPublicationStatusIsTerminal(status.state)) {
      if (!(deferTerminalAcknowledgement && status.state === "succeeded")) {
        await options.settleTerminal(status);
      }
    } else if (status.state === "indeterminate") {
      await options.applyIndeterminate(status);
    } else {
      options.onPending(status);
    }
  };

  const abandon = async (
    intent: ProjectPublicationIntentV1,
    status: ProjectPublicationStatusV1,
    requestedSurface: ProjectPublicationSurfaceV1,
  ): Promise<ProjectPublicationStatusV1 | null> => {
    options.ensureMutationAllowed("abandon_project_publication_v1");
    if (!await options.confirmAbandon(intent, status, requestedSurface)) return null;
    const abandoned = await invokeParsed("abandon_project_publication_v1", intent.request);
    if (!abandoned || (abandoned.state !== "abandoned" && !projectPublicationStatusIsTerminal(abandoned.state))) {
      throw new Error("The project publication was not terminal after the explicit abandon request.");
    }
    await options.settleTerminal(abandoned);
    options.onAbandoned(intent, requestedSurface);
    return abandoned;
  };

  const start = async (
    surface: ProjectPublicationSurfaceV1,
    reason: string | null = null,
    deferTerminalAcknowledgement = false,
  ): Promise<ProjectPublicationStatusV1> => {
    await options.publishQueuedAcknowledgement();
    let existing = loadProjectPublicationIntent();
    const requestedReason = normalizeProjectPublicationReason(reason);
    const requestedPolicy = targetPolicyForSurface(surface);
    if (existing) {
      const metadataMatches = existing.request.surface === surface
        && existing.request.reason === requestedReason
        && existing.request.targetPolicy === requestedPolicy;
      const queried = await queryExisting(existing);
      existing = queried.intent;
      let observed = queried.status;
      if (observed) {
        const adopted = await adoptOwnerIfRequired(existing, observed);
        existing = adopted.intent;
        observed = adopted.status;
        await settleObserved(observed, deferTerminalAcknowledgement);
        if (projectPublicationStatusIsTerminal(observed.state) || observed.state === "indeterminate") {
          if (!metadataMatches) {
            throw new Error("A different durable project publication request was completed or reconciled. Choose the requested action again to create its own request.");
          }
          return observed;
        }
        if (observed.state === "prepared") {
          if (!metadataMatches) {
            throw new Error("The durable prepared project publication cannot be abandoned. Recover it before starting another action.");
          }
          return observed;
        }
        if (!metadataMatches) {
          const abandoned = await abandon(existing, observed, surface);
          if (abandoned) return abandoned;
          throw new Error("A different durable project publication request was recovered. It remains pending until explicitly abandoned.");
        }
        if (observed.state === "selecting") {
          return await abandon(existing, observed, surface) ?? observed;
        }
        // Reserved/Selected resume only with their persisted request. A stale
        // Selecting reply cannot win over an already-observed Selected event.
      } else if (!metadataMatches) {
        throw new Error(
          "A different durable project publication request is unresolved. Query/recover or explicitly abandon it before starting another action.",
        );
      }
      const resumed = await invokeParsed(
        publicationCommandForSurface(existing.request.surface),
        existing.request,
      );
      if (!resumed) throw new Error("Backend did not return the resumed project publication receipt.");
      await settleObserved(resumed, deferTerminalAcknowledgement);
      return resumed;
    }

    const command = publicationCommandForSurface(surface);
    options.ensureMutationAllowed(command);
    const authoritySeed = await options.captureSeed();
    const intent = beginProjectPublicationIntent({
      ...authoritySeed,
      surface,
      reason: requestedReason,
      targetPolicy: requestedPolicy,
    });
    if (!intent) throw new Error("Unable to durably record the project publication request before it was sent.");
    const status = await invokeParsed(command, intent.request);
    if (!status) throw new Error("Backend did not return the project publication receipt.");
    await settleObserved(status, deferTerminalAcknowledgement);
    return status;
  };

  const recover = async (): Promise<void> => {
    await options.publishQueuedAcknowledgement();
    let intent = loadProjectPublicationIntent();
    if (!intent) return;
    const queried = await queryExisting(intent);
    intent = queried.intent;
    let status = queried.status;
    if (!status) {
      options.onMissing(intent);
      return;
    }
    const adopted = await adoptOwnerIfRequired(intent, status);
    status = adopted.status;
    await settleObserved(status, false);
  };

  return { start, recover, observeStatus };
};

const requestMatchesSeed = (
  request: ProjectPublicationRequestV1,
  seed: ProjectPublicationRequestSeedV1,
) => request.surface === seed.surface
  && request.ownerId === seed.ownerId
  && request.expectedProjectEpoch === seed.expectedProjectEpoch
  && request.expectedProjectRevision === seed.expectedProjectRevision
  && request.expectedCheckpointHash === seed.expectedCheckpointHash
  && request.mappingAuthorityHash === seed.mappingAuthorityHash
  && request.sourcePath === seed.sourcePath
  && request.reason === seed.reason
  && request.targetPolicy === seed.targetPolicy;

const storageEnvelopeFromUnknown = (candidate: unknown): ProjectPublicationStorageEnvelopeV1 | null => {
  if (!isRecord(candidate) || !hasExactKeys(candidate, [
    "version", "origin_id", "next_request_id", "intent", "acknowledgement",
  ])) return null;
  const source = candidate as Partial<ProjectPublicationStorageEnvelopeV1>;
  const intentRequest = isRecord(source.intent)
    ? projectPublicationRequestFromUnknown(source.intent.request)
    : null;
  const acknowledgement = source.acknowledgement === null
    ? null
    : projectPublicationRequestFromUnknown(source.acknowledgement);
  if (source.version !== projectPublicationStorageVersion
    || !isOriginId(source.origin_id)
    || !isSafePositiveInteger(source.next_request_id)
    || source.next_request_id > maxSafePublicationNumber
    || !(source.intent === null || isRecord(source.intent)
      && hasExactKeys(source.intent, ["version", "request"])
      && (source.intent as Partial<ProjectPublicationIntentV1>).version === 1 && intentRequest)
    || !(source.acknowledgement === null || acknowledgement)
    || (intentRequest && intentRequest.originId !== source.origin_id)
    || (acknowledgement && acknowledgement.originId !== source.origin_id)
    || (intentRequest && intentRequest.requestId >= source.next_request_id)
    || (acknowledgement && acknowledgement.requestId >= source.next_request_id)) {
    return null;
  }
  return {
    version: projectPublicationStorageVersion,
    origin_id: source.origin_id,
    next_request_id: source.next_request_id,
    intent: intentRequest ? { version: 1, request: intentRequest } : null,
    acknowledgement,
  };
};

/** `undefined` is corrupt/unavailable and must not be replaced by a new origin. */
const readEnvelope = (): ProjectPublicationStorageEnvelopeV1 | null | undefined => {
  if (typeof window === "undefined") return null;
  try {
    const raw = window.localStorage.getItem(projectPublicationStorageKey);
    if (raw === null) return null;
    return storageEnvelopeFromUnknown(JSON.parse(raw)) ?? undefined;
  } catch {
    return undefined;
  }
};

const writeEnvelope = (envelope: ProjectPublicationStorageEnvelopeV1): boolean => {
  if (typeof window === "undefined") return false;
  try {
    window.localStorage.setItem(projectPublicationStorageKey, JSON.stringify(envelope));
    return true;
  } catch {
    return false;
  }
};

const newOriginId = (): string | null => {
  const origin = globalThis.crypto?.randomUUID?.();
  return origin && isOriginId(origin) ? origin : null;
};

const readOrCreateEnvelope = (): ProjectPublicationStorageEnvelopeV1 | null => {
  const existing = readEnvelope();
  if (existing !== null) return existing ?? null;
  const originId = newOriginId();
  if (!originId) return null;
  const created: ProjectPublicationStorageEnvelopeV1 = {
    version: projectPublicationStorageVersion,
    origin_id: originId,
    next_request_id: 1,
    intent: null,
    acknowledgement: null,
  };
  return writeEnvelope(created) ? created : null;
};

/** The one unfinished publication request, if a renderer restart occurred. */
export const loadProjectPublicationIntent = (): ProjectPublicationIntentV1 | null =>
  readEnvelope()?.intent ?? null;

/** A terminal receipt ACK that was durably queued before the backend ACK. */
export const loadProjectPublicationAcknowledgement = (): ProjectPublicationRequestV1 | null =>
  readEnvelope()?.acknowledgement ?? null;

/**
 * Persist a full, immutable request before its first invoke. A new UI action
 * cannot replace an unresolved request: retry/query/abandon must settle that
 * exact durable identity instead.
 */
export const beginProjectPublicationIntent = (
  seed: ProjectPublicationRequestSeedV1,
): ProjectPublicationIntentV1 | null => {
  const envelope = readOrCreateEnvelope();
  if (!envelope || envelope.acknowledgement) return null;
  if (seed.reason !== normalizeProjectPublicationReason(seed.reason)) return null;
  // An unresolved ID is immutable. It can only be retried when this caller
  // presents the exact same canonical publication shape; a different button,
  // reason, path, or authority must not inherit a prior Backup/Save request.
  if (envelope.intent) {
    return requestMatchesSeed(envelope.intent.request, seed) ? envelope.intent : null;
  }
  if (!isSafePositiveInteger(envelope.next_request_id)
    || envelope.next_request_id > maxSafePublicationNumber) return null;
  const request: ProjectPublicationRequestV1 = {
    schemaVersion: projectPublicationRequestSchemaVersion,
    originId: envelope.origin_id,
    requestId: envelope.next_request_id,
    ...seed,
  };
  if (!projectPublicationRequestFromUnknown(request)) return null;
  if (request.requestId >= maxSafePublicationNumber) return null;
  const next: ProjectPublicationStorageEnvelopeV1 = {
    ...envelope,
    next_request_id: request.requestId + 1,
    intent: { version: 1, request },
  };
  return writeEnvelope(next) ? next.intent : null;
};

/**
 * Persist the backend-authorized owner adoption before any resumed mutation.
 * Every request field except ownerId is immutable; a stale/misrouted reply or
 * storage failure leaves the old exact intent available for query recovery.
 */
export const adoptProjectPublicationIntentOwner = (
  oldRequest: ProjectPublicationRequestV1,
  adoptedRequest: ProjectPublicationRequestV1,
): boolean => {
  const oldCanonical = projectPublicationRequestFromUnknown(oldRequest);
  const adoptedCanonical = projectPublicationRequestFromUnknown(adoptedRequest);
  if (!oldCanonical
    || !adoptedCanonical
    || oldCanonical.ownerId === adoptedCanonical.ownerId
    || !projectPublicationRequestMatchesExceptOwner(oldCanonical, adoptedCanonical)) return false;
  const envelope = readEnvelope();
  if (!envelope
    || envelope.acknowledgement
    || !envelope.intent
    || !projectPublicationRequestMatches(envelope.intent.request, oldCanonical)) return false;
  return writeEnvelope({
    ...envelope,
    intent: { version: 1, request: adoptedCanonical },
  });
};

/** Move the active request to the durable ACK queue after local application. */
export const queueProjectPublicationAcknowledgement = (request: ProjectPublicationRequestV1): boolean => {
  const envelope = readEnvelope();
  if (!envelope) return false;
  if (envelope.acknowledgement && projectPublicationRequestMatches(envelope.acknowledgement, request)) return true;
  if (!envelope.intent || !projectPublicationRequestMatches(envelope.intent.request, request)) return false;
  return writeEnvelope({ ...envelope, intent: null, acknowledgement: request });
};

/** Clear only the acknowledgement that has been successfully delivered. */
export const acknowledgeProjectPublicationIntent = (request: ProjectPublicationRequestV1): boolean => {
  const envelope = readEnvelope();
  if (!envelope || !envelope.acknowledgement
    || !projectPublicationRequestMatches(envelope.acknowledgement, request)) return false;
  return writeEnvelope({ ...envelope, acknowledgement: null });
};

/** Keep terminal cleanup explicit; normal publication code must use the ACK queue. */
export const clearProjectPublicationStorageForTests = () => {
  if (typeof window === "undefined") return;
  try {
    window.localStorage.removeItem(projectPublicationStorageKey);
  } catch {
    // Tests and recovery callers fail closed when localStorage is unavailable.
  }
};
