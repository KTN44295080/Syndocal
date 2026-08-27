import {
  projectAuthorityHasDirtyMappings,
  type ProjectAuthoritySyncState,
  type ProjectAuthorityToken,
} from "./projectAuthority.ts";
import type { ProjectPublicationRequestSeedV1 } from "./projectPublicationStorage.ts";
import type { ProjectAuthorityBundle, ProjectPublicationStatusV1 } from "./types.ts";

/**
 * This module is loaded only by explicit Save/recovery flushes. Keeping the
 * bounded retry protocol outside App keeps the hot operator UI bundle smaller
 * and gives the asynchronous ownership race a deterministic test seam.
 */
export type ProjectControlMappingsPersistKind = "acknowledged" | "retryable" | "untrusted";

export type ProjectControlMappingsPersistResult = {
  kind: ProjectControlMappingsPersistKind;
  authority: ProjectAuthorityToken;
};

export type ProjectControlMappingsFlushBridgeState = {
  disposed: boolean;
  authorityReady: boolean;
  sync: ProjectAuthoritySyncState;
  mappingSyncInFlight: boolean;
  timerFlushOwner: symbol | null;
};

export type ProjectControlMappingsFlushBridgeResult = {
  ownAcknowledgements: ProjectAuthorityToken[];
  trusted: boolean;
};

export type ProjectControlMappingsFlushBridgeOptions = {
  persist: (flushOwner: symbol) => Promise<ProjectControlMappingsPersistResult>;
  cancelTimer: (flushOwner?: symbol) => boolean;
  state: () => ProjectControlMappingsFlushBridgeState;
};

export type TrustedProjectPublicationAuthorityBridgeOptions = {
  authorityReady: () => boolean;
  captureAuthority: () => ProjectAuthorityToken;
  identityGeneration: () => number;
  flush: () => Promise<ProjectControlMappingsFlushBridgeResult>;
  mappingProvenance: (
    expected: ProjectAuthorityToken,
    current: ProjectAuthorityToken,
    flushed: ProjectControlMappingsFlushBridgeResult,
  ) => "unchanged" | "own_mapping_ack" | null;
  ownerId: string;
  sourcePath: string | null;
};

export type PrepareMediaAssetOperationStartBridgeOptions = {
  expectedAuthority: ProjectAuthorityToken;
  operatorAllowed: boolean;
  operatorMode: string | null;
  flush: () => Promise<ProjectControlMappingsFlushBridgeResult>;
  captureAuthority: () => ProjectAuthorityToken;
  mappingProvenance: (
    expected: ProjectAuthorityToken,
    current: ProjectAuthorityToken,
    ownAcknowledgements: readonly ProjectAuthorityToken[],
  ) => "unchanged" | "own_mapping_ack" | null;
};

export type ApplyProjectPublicationTerminalBridgeOptions = {
  status: ProjectPublicationStatusV1;
  currentAuthority: () => ProjectAuthorityToken;
  authorityIsCurrent: (
    captured: ProjectAuthorityToken,
    current: ProjectAuthorityToken,
  ) => boolean;
  applyRuntimeStatus: (bundle: ProjectAuthorityBundle) => void;
  refreshBackups: () => Promise<void>;
  rememberRecentPath: (path: string | null) => void;
  setMessage: (message: string) => void;
};

export type ProjectControlMappingsFlushDecision = "acknowledged" | "retry" | "fail";

export const projectControlMappingsFlushDecision = (
  kind: ProjectControlMappingsPersistKind,
  sync: ProjectAuthoritySyncState,
  requestIdentityGeneration: number,
  mappingSyncInFlight: boolean,
  replayBudget: number,
): ProjectControlMappingsFlushDecision => {
  if (kind === "acknowledged") return "acknowledged";
  if (kind !== "retryable"
    || replayBudget <= 0
    || mappingSyncInFlight
    || sync.identityGeneration !== requestIdentityGeneration
    || !projectAuthorityHasDirtyMappings(sync)) {
    return "fail";
  }
  return "retry";
};

/**
 * A terminal explicit flush consumes only the retry timer scheduled by one of
 * its own persist attempts. An ordinary edit may replace that timer while the
 * flush waits and must retain its independently requested autosave.
 */
export const projectControlMappingsFlushTimerAction = (
  terminalUntrusted: boolean,
  timerFlushOwner: symbol | null,
  flushOwner: symbol,
): "cancel" | "preserve" => terminalUntrusted && timerFlushOwner === flushOwner
  ? "cancel"
  : "preserve";

/**
 * Close a poll-won successful mapping write with one exact ACK request. A
 * second stale response, authority replacement, validation/CAS/worker error,
 * or a newer ordinary edit stays untrusted; no indefinite Save retry escapes.
 */
export const flushProjectControlMappingsAuthorityBridge = async (
  options: ProjectControlMappingsFlushBridgeOptions,
): Promise<ProjectControlMappingsFlushBridgeResult> => {
  options.cancelTimer();
  const flushOwner = Symbol("project-control-mappings-flush");
  const ownAcknowledgements: ProjectAuthorityToken[] = [];
  let trusted = true;
  let replayBudget = 1;
  let current = options.state();
  while (
    !current.disposed
    && current.authorityReady
    && projectAuthorityHasDirtyMappings(current.sync)
  ) {
    const identityGeneration = current.sync.identityGeneration;
    const persisted = await options.persist(flushOwner);
    current = options.state();
    const decision = projectControlMappingsFlushDecision(
      persisted.kind,
      current.sync,
      identityGeneration,
      current.mappingSyncInFlight,
      replayBudget,
    );
    if (decision === "acknowledged") {
      ownAcknowledgements.push(persisted.authority);
    } else if (decision === "retry") {
      replayBudget -= 1;
      // R1's finally schedules an owned 0ms retry. This explicit path owns
      // R2 instead, so consume precisely that timer before continuing.
      options.cancelTimer(flushOwner);
      current = options.state();
      continue;
    } else {
      trusted = false;
      break;
    }
    current = options.state();
    if (identityGeneration !== current.sync.identityGeneration || current.mappingSyncInFlight) {
      trusted = false;
      break;
    }
    if (!projectAuthorityHasDirtyMappings(current.sync)) break;
  }
  current = options.state();
  const flushTrusted = trusted
    && !current.disposed
    && !current.mappingSyncInFlight
    && !projectAuthorityHasDirtyMappings(current.sync);
  if (projectControlMappingsFlushTimerAction(
    !flushTrusted,
    current.timerFlushOwner,
    flushOwner,
  ) === "cancel") {
    options.cancelTimer(flushOwner);
  }
  return { ownAcknowledgements, trusted: flushTrusted };
};

/**
 * Publication only needs this capture when an explicit operator action starts.
 * Keeping it alongside the flush protocol keeps the large App entry chunk out
 * of this authority-only orchestration without relaxing E/R/H provenance.
 */
export const requireTrustedProjectPublicationAuthorityBridge = async (
  options: TrustedProjectPublicationAuthorityBridgeOptions,
): Promise<ProjectPublicationRequestSeedV1> => {
  if (!options.authorityReady()) {
    throw new Error("Project authority is still initializing; wait before saving.");
  }
  const beforeFlush = options.captureAuthority();
  const beforeIdentityGeneration = options.identityGeneration();
  const flushed = await options.flush();
  const authority = options.captureAuthority();
  const provenance = options.mappingProvenance(
    beforeFlush,
    authority,
    flushed,
  );
  if (!flushed.trusted
    || !provenance
    || beforeIdentityGeneration !== options.identityGeneration()
    || !options.authorityReady()
    || !/^[0-9a-f]{64}$/.test(authority.checkpoint_hash)) {
    throw new Error("Project control mappings could not be durably synchronized; save was not started.");
  }
  return {
    surface: "save",
    ownerId: options.ownerId,
    expectedProjectEpoch: authority.project_epoch,
    expectedProjectRevision: authority.project_revision,
    expectedCheckpointHash: authority.checkpoint_hash,
    mappingAuthorityHash: authority.checkpoint_hash,
    sourcePath: options.sourcePath,
    reason: null,
    targetPolicy: "current_or_dialog",
  };
};

export const prepareMediaAssetOperationStartBridge = async (
  options: PrepareMediaAssetOperationStartBridgeOptions,
): Promise<{
  authority: ProjectAuthorityToken;
  provenance: "unchanged" | "own_mapping_ack";
}> => {
  if (!options.operatorAllowed) {
    throw new Error(
      options.operatorMode === "Full"
        ? "Operator Full Lock allows only status reads and emergency blackout controls."
        : "Operator Partial Lock blocks programming and project replacement commands.",
    );
  }
  const flushed = await options.flush();
  const authority = options.captureAuthority();
  if (authority.project_epoch !== options.expectedAuthority.project_epoch || !flushed.trusted) {
    throw new Error("Project changed while the media operation was starting; nothing was applied.");
  }
  const provenance = options.mappingProvenance(
    options.expectedAuthority,
    authority,
    flushed.ownAcknowledgements,
  );
  if (!provenance) {
    throw new Error("Project control mappings changed outside this media operation; choose the media action again.");
  }
  return { authority, provenance };
};

/** Publication receipts are an explicit save/recovery path, not hot UI state. */
export const applyProjectPublicationTerminalBridge = async (
  options: ApplyProjectPublicationTerminalBridgeOptions,
): Promise<void> => {
  const { status } = options;
  if (status.state === "indeterminate") {
    options.setMessage(`Project publication outcome is indeterminate: ${status.error ?? "restart to reconcile the durable receipt"}`);
    return;
  }
  if (status.state === "cancelled") {
    options.setMessage(
      status.surface === "user_template" ? "Template save canceled." : "Project publication canceled.",
    );
    return;
  }
  if (status.state === "abandoned") {
    options.setMessage("The pending project publication was abandoned.");
    return;
  }
  if (status.state === "failed") {
    options.setMessage(`Project publication failed: ${status.error ?? "unknown durable failure"}`);
    return;
  }
  if (status.state !== "succeeded") return;

  if (status.surface === "backup") {
    await options.refreshBackups();
    if (status.warning) {
      options.setMessage(`Backup completed, but retention cleanup needs attention: ${status.warning}`);
    }
    return;
  }
  if (status.surface === "user_template") {
    options.setMessage(status.targetPath ? `Saved user template ${status.targetPath}` : "Saved user template.");
    return;
  }

  const saved = status.authority;
  if (saved && options.authorityIsCurrent(saved, options.currentAuthority())) {
    options.applyRuntimeStatus(saved);
    if (options.authorityIsCurrent(saved, options.currentAuthority())) {
      if (status.targetPath) options.rememberRecentPath(status.targetPath);
      options.setMessage(status.targetPath ? `Saved project ${status.targetPath}` : "Saved project.");
      return;
    }
  }
  options.setMessage(
    status.targetPath
      ? `Saved an earlier project image to ${status.targetPath}; the current project changed before the acknowledgement arrived.`
      : "Saved an earlier project image; the current project changed before the acknowledgement arrived.",
  );
};
