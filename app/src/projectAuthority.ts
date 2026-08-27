import type {
  ProjectAuthorityDisposition,
  ProjectAuthorityPublicationKind,
  ProjectInputRuntimeStatus,
  ProjectRecoveryAuthorityTransition,
} from "./types";

/**
 * Pure coordinator-facing state for the frontend mapping authority bridge.
 *
 * A mapping reply is valid only for the identity and request that issued it;
 * a loaded-project reply has an additional application generation because it
 * performs asynchronous snapshot/history/UI work after hydrating mappings.
 * Keeping these transitions data-only gives the UI an executable proof seam
 * without coupling it to Tauri or Solid signals.
 */

export interface ProjectAuthorityToken {
  project_epoch: number;
  project_revision: number;
  checkpoint_hash: string;
}

export interface ProjectAuthoritySyncState {
  identityGeneration: number;
  requestGeneration: number;
  applicationGeneration: number;
  localGeneration: number;
  persistedGeneration: number;
}

export interface ProjectAuthorityRequest {
  identityGeneration: number;
  requestGeneration: number;
}

export interface ProjectAuthorityApplication {
  applicationGeneration: number;
}

export const projectAuthorityGenerationIsValid = (value: number): boolean =>
  Number.isSafeInteger(value) && value >= 0;

export type ProjectAuthorityInputRuntimeVerdict = "apply" | "duplicate" | "stale";

const projectInputRuntimeFlagsEqual = (
  left: ProjectInputRuntimeStatus,
  right: ProjectInputRuntimeStatus,
): boolean => left.midi_clock_active === right.midi_clock_active
  && left.midi_control_active === right.midi_control_active
  && left.midi_feedback_output_active === right.midi_feedback_output_active
  && left.midi_feedback_runtime_active === right.midi_feedback_runtime_active
  && left.osc_active === right.osc_active
  && left.dmx_active === right.dmx_active;

/**
 * Input worker generations are two independent monotonic fences which must
 * be compared as one pair.  A candidate which regresses either member is an
 * old event/reply and cannot update any runtime flag.  Equal generations are
 * still allowed to apply a changed liveness flag: DMX packet/liveness polling
 * is runtime-only and must converge without manufacturing a new worker epoch.
 */
export const projectAuthorityInputRuntimeVerdict = (
  candidate: ProjectInputRuntimeStatus,
  observed: Pick<
    ProjectInputRuntimeStatus,
    "project_input_runtime_generation" | "mapping_input_runtime_generation"
  >,
  current: ProjectInputRuntimeStatus | null,
): ProjectAuthorityInputRuntimeVerdict => {
  const candidateProjectGeneration = candidate.project_input_runtime_generation;
  const candidateMappingGeneration = candidate.mapping_input_runtime_generation;
  const observedProjectGeneration = observed.project_input_runtime_generation;
  const observedMappingGeneration = observed.mapping_input_runtime_generation;
  if (![candidateProjectGeneration, candidateMappingGeneration,
    observedProjectGeneration, observedMappingGeneration].every(projectAuthorityGenerationIsValid)) {
    return "stale";
  }
  if (candidateProjectGeneration < observedProjectGeneration
    || candidateMappingGeneration < observedMappingGeneration) {
    return "stale";
  }
  if (current === null
    || candidateProjectGeneration > observedProjectGeneration
    || candidateMappingGeneration > observedMappingGeneration) {
    return "apply";
  }
  return projectInputRuntimeFlagsEqual(candidate, current) ? "duplicate" : "apply";
};

export type ProjectAuthorityReplacementVerdict = "apply" | "duplicate" | "stale";

export const createProjectAuthoritySyncState = (): ProjectAuthoritySyncState => ({
  identityGeneration: 0,
  requestGeneration: 0,
  applicationGeneration: 0,
  localGeneration: 0,
  persistedGeneration: 0,
});

const advance = (value: number): number => {
  if (!Number.isSafeInteger(value) || value >= Number.MAX_SAFE_INTEGER) {
    throw new Error("Project authority generation is exhausted; reload Syndocal before continuing");
  }
  return value + 1;
};

export const projectAuthorityCanApply = (
  candidate: ProjectAuthorityToken,
  current: ProjectAuthorityToken,
): boolean =>
  // The UI begins before the first backend authority response. That bootstrap
  // response establishes epoch 0/revision 0; afterwards equality must also
  // mean the same canonical checkpoint, never merely the same numbers.
  current.checkpoint_hash.length === 0
  || candidate.project_epoch > current.project_epoch
  || (candidate.project_epoch === current.project_epoch
    && (candidate.project_revision > current.project_revision
      || (candidate.project_revision === current.project_revision
        && candidate.checkpoint_hash === current.checkpoint_hash)));

/**
 * Learn/reconnect flows retain an opaque authority token across an await.
 * Unlike the monotonic hydrate predicate, an in-flight operation must require
 * exact equality: any identity/revision/hash change means its captured input
 * and reconnect target belong to an older project and must be discarded.
 */
export const projectAuthorityTokenIsCurrent = (
  captured: ProjectAuthorityToken,
  current: ProjectAuthorityToken,
): boolean => captured.project_epoch === current.project_epoch
  && captured.project_revision === current.project_revision
  && captured.checkpoint_hash === current.checkpoint_hash;

/**
 * A successful mapping RPC may return after a poll has already adopted that
 * exact committed token. This is retryable only inside the same identity; a
 * replacement or foreign authority always remains fail-closed.
 */
export const successfulStaleProjectControlMappingsReplyIsRetryable = (
  sync: ProjectAuthoritySyncState,
  request: ProjectAuthorityRequest,
  returnedAuthority: ProjectAuthorityToken,
  currentAuthority: ProjectAuthorityToken,
  disposed: boolean,
): boolean => !disposed
  && sync.identityGeneration === request.identityGeneration
  && projectAuthorityTokenIsCurrent(returnedAuthority, currentAuthority);

/**
 * Recovery intent delivery is a side effect of an accepted current bundle,
 * never of a rejected or exact-duplicate poll/reply. Keep this gate beside
 * the token predicate so all production entry points use the same ownership
 * rule before touching browser storage or issuing an ACK.
 */
export const projectAuthorityBundleMayConsumeRecoveryIntent = (
  application: "applied" | "duplicate" | "stale",
  candidate: ProjectAuthorityToken,
  current: ProjectAuthorityToken,
): boolean => application === "applied"
  && projectAuthorityTokenIsCurrent(candidate, current);

/**
 * A replacement result has stronger semantics than a regular status refresh:
 * applying it invalidates local mapping work and resets retired-input UI.
 * Keep the observed coordinator token separate from the last replacement
 * token so an event and its command reply for the exact same publication are
 * a strict no-op, while bootstrap and a status-only observation can still be
 * hydrated once.
 */
export const projectAuthorityReplacementVerdict = (
  candidate: ProjectAuthorityToken,
  observed: ProjectAuthorityToken,
  lastAppliedReplacement: ProjectAuthorityToken | null,
  initialized: boolean,
): ProjectAuthorityReplacementVerdict => {
  if (initialized && lastAppliedReplacement !== null
    && projectAuthorityTokenIsCurrent(candidate, lastAppliedReplacement)) {
    return "duplicate";
  }
  return projectAuthorityCanApply(candidate, observed) ? "apply" : "stale";
};

/**
 * A direct command may receive B after its B authority event already applied.
 * That reply is a duplicate (no second hydrate), but its local continuation
 * is still safe until a later C application changes either token or app
 * generation. Event listeners deliberately do not use this continuation.
 */
export const projectAuthorityReplacementContinuationIsCurrent = (
  verdict: ProjectAuthorityReplacementVerdict,
  token: ProjectAuthorityToken,
  applicationGenerationAtDecision: number,
  current: ProjectAuthorityToken,
  state: ProjectAuthoritySyncState,
): boolean => verdict !== "stale"
  && projectAuthorityTokenIsCurrent(token, current)
  && state.applicationGeneration === applicationGenerationAtDecision;

export const beginProjectAuthorityRequest = (
  state: ProjectAuthoritySyncState,
): { state: ProjectAuthoritySyncState; request: ProjectAuthorityRequest } => {
  const requestGeneration = advance(state.requestGeneration);
  const next = { ...state, requestGeneration };
  return {
    state: next,
    request: {
      identityGeneration: next.identityGeneration,
      requestGeneration,
    },
  };
};

export const projectAuthorityResponseIsCurrent = (
  state: ProjectAuthoritySyncState,
  request: ProjectAuthorityRequest,
): boolean =>
  state.identityGeneration === request.identityGeneration
  && state.requestGeneration === request.requestGeneration;

export const invalidateProjectAuthorityIdentity = (
  state: ProjectAuthoritySyncState,
): ProjectAuthoritySyncState => {
  const localGeneration = advance(state.localGeneration);
  return {
    ...state,
    identityGeneration: advance(state.identityGeneration),
    requestGeneration: advance(state.requestGeneration),
    localGeneration,
    // Drop unsent/local work from the former identity. Its delayed ack cannot
    // retry into the authoritative replacement.
    persistedGeneration: localGeneration,
  };
};

export const noteLocalProjectAuthorityEdit = (
  state: ProjectAuthoritySyncState,
): ProjectAuthoritySyncState => ({
  ...state,
  localGeneration: advance(state.localGeneration),
});

export const acknowledgeProjectAuthorityPersist = (
  state: ProjectAuthoritySyncState,
  request: ProjectAuthorityRequest,
  sentGeneration: number,
): ProjectAuthoritySyncState => {
  if (!projectAuthorityResponseIsCurrent(state, request)) return state;
  return {
    ...state,
    persistedGeneration: Math.max(state.persistedGeneration, sentGeneration),
  };
};

/**
 * Hydration from a fenced identity replacement is already authoritative: it
 * supersedes any local debounce work for the previous identity.  Mark the
 * current generation clean without manufacturing a request acknowledgement.
 */
export const markProjectAuthorityPersisted = (
  state: ProjectAuthoritySyncState,
): ProjectAuthoritySyncState => ({
  ...state,
  persistedGeneration: state.localGeneration,
});

export const projectAuthorityHasDirtyMappings = (
  state: ProjectAuthoritySyncState,
): boolean => state.localGeneration > state.persistedGeneration;

export const projectAuthorityShouldRetryPersist = (
  state: ProjectAuthoritySyncState,
  requestIdentityGeneration: number,
): boolean => state.identityGeneration === requestIdentityGeneration
  && projectAuthorityHasDirtyMappings(state);

/**
 * A poll may observe a newer persistent project token while this browser has
 * a local, unsent mapping edit.  That observation must advance the CAS base
 * without replacing the local arrays: the next write is deliberately rebased
 * onto the observed token. Identity replacements still invalidate local work
 * through `invalidateProjectAuthorityIdentity` before this helper is used.
 */
export const rebaseDirtyProjectAuthorityMappings = (
  state: ProjectAuthoritySyncState,
  hasDirtyMappings: boolean,
): ProjectAuthoritySyncState => hasDirtyMappings
  ? {
    ...state,
    // Invalidate an in-flight R1 response while retaining the local B
    // generation. The caller schedules R2 with the newly observed CAS token.
    requestGeneration: advance(state.requestGeneration),
  }
  : state;

/** A newer regular poll must not overwrite unsent local mapping arrays. */
export const projectAuthorityPollPreservesDirtyMappings = (
  candidate: ProjectAuthorityToken,
  current: ProjectAuthorityToken,
  state: ProjectAuthoritySyncState,
  persistInFlight: boolean,
  publicationKind: ProjectAuthorityPublicationKind = "mutation",
): boolean => !projectAuthorityTokenIsCurrent(candidate, current)
  && !projectAuthorityPublicationRequiresMappingHydration(publicationKind)
  && (persistInFlight || projectAuthorityHasDirtyMappings(state));

/**
 * Identity publication and history navigation both replace the backend-owned
 * mapping arrays. A local A debounce must be discarded rather than rebased
 * onto B, even when Undo/Redo keeps the same epoch.
 */
export const projectAuthorityPublicationRequiresMappingHydration = (
  kind: ProjectAuthorityPublicationKind,
): boolean => kind === "identity_replacement" || kind === "history_navigation";

/**
 * The durable replacement counter, not merely the last publication kind,
 * decides whether poll may retain a local mapping draft. An ordinary mutation
 * after Undo/Redo must not hide the preceding mapping replacement.
 */
export const projectAuthorityPollMustHydrateMappings = (
  candidateMappingReplacementGeneration: number,
  observedMappingReplacementGeneration: number,
): boolean => Number.isSafeInteger(candidateMappingReplacementGeneration)
  && candidateMappingReplacementGeneration > observedMappingReplacementGeneration;

/**
 * History has its own monotonically increasing status generation. A delayed
 * A status reply/event is never allowed to overwrite B even when both carry
 * the same content token; an identity/revision/hash mismatch is rejected
 * first, then the generation comparison handles same-token history changes.
 */
export const projectAuthorityCanApplyHistoryStatus = (
  candidate: ProjectAuthorityToken & { history_generation: number },
  current: ProjectAuthorityToken,
  observedHistoryGeneration: number,
): boolean => projectAuthorityTokenIsCurrent(candidate, current)
  && Number.isSafeInteger(candidate.history_generation)
  && candidate.history_generation >= observedHistoryGeneration;

/** A committed mapping mutation retires mapping-driven live input workers. */
export const projectAuthorityMappingCommitRetiredInputs = (
  authority: { mapping_runtimes_retired?: boolean },
): boolean => authority.mapping_runtimes_retired === true;

export const beginProjectAuthorityApplication = (
  state: ProjectAuthoritySyncState,
): { state: ProjectAuthoritySyncState; application: ProjectAuthorityApplication } => {
  const applicationGeneration = advance(state.applicationGeneration);
  return {
    state: { ...state, applicationGeneration },
    application: { applicationGeneration },
  };
};

export const projectAuthorityApplicationIsCurrent = (
  state: ProjectAuthoritySyncState,
  application: ProjectAuthorityApplication,
): boolean => state.applicationGeneration === application.applicationGeneration;

/**
 * Mapping-driven input constructors use this single sequencing primitive.
 * The callback is evaluated only after the frontend's authoritative mapping
 * flush settles, so Learn/manual Connect cannot pass a debounce-era clone.
 */
export const runAfterProjectAuthorityFlush = async <T>(
  flush: () => Promise<void>,
  start: () => Promise<T>,
): Promise<T> => {
  await flush();
  return start();
};

/** Saved-file dirty truth is an authority baseline, never a partial UI hash. */
export type ProjectAuthoritySavedBaseline = ProjectAuthorityToken & {
  current_project_path: string | null;
};

export type ProjectAuthorityDispositionState = {
  baseline: ProjectAuthoritySavedBaseline | null;
  observedDispositionGeneration: number;
  /**
   * Generation zero is a valid first durable disposition.  Keep its
   * initialization separate from the numeric value so a later RuntimeStatus
   * or Mutation carrying the same zero cannot replay clean/recovery effects.
   */
  dispositionInitialized: boolean;
  dirty: boolean;
};

export type ProjectAuthorityDispositionCandidate = ProjectAuthorityToken & {
  current_project_path: string | null;
  authority_disposition_generation: number;
  authority_disposition: ProjectAuthorityDisposition;
};

export const projectAuthorityDispositionTransition = (
  state: ProjectAuthorityDispositionState,
  candidate: ProjectAuthorityDispositionCandidate,
): ProjectAuthorityDispositionState => {
  if (!Number.isSafeInteger(candidate.authority_disposition_generation)
    || (state.dispositionInitialized
      && candidate.authority_disposition_generation < state.observedDispositionGeneration)) {
    return state;
  }
  if (state.dispositionInitialized
    && candidate.authority_disposition_generation === state.observedDispositionGeneration) {
    return state;
  }
  const observedDispositionGeneration = candidate.authority_disposition_generation;
  switch (candidate.authority_disposition) {
    case "clean_at_path":
      return {
        baseline: {
          project_epoch: candidate.project_epoch,
          project_revision: candidate.project_revision,
          checkpoint_hash: candidate.checkpoint_hash,
          current_project_path: candidate.current_project_path,
        },
        observedDispositionGeneration,
        dispositionInitialized: true,
        dirty: false,
      };
    case "unsaved_replacement":
    case "recovery_pending_ack":
      return {
        baseline: null,
        observedDispositionGeneration,
        dispositionInitialized: true,
        dirty: true,
      };
    case "history_navigation": {
      const baseline = state.baseline;
      const clean = baseline !== null
        && baseline.project_epoch === candidate.project_epoch
        && baseline.checkpoint_hash === candidate.checkpoint_hash
        && baseline.current_project_path === candidate.current_project_path;
      return {
        baseline,
        observedDispositionGeneration,
        dispositionInitialized: true,
        dirty: !clean,
      };
    }
    case "runtime_sanitize":
      return {
        ...state,
        observedDispositionGeneration,
        dispositionInitialized: true,
      };
  }
};

/** Only an explicit saved baseline may retire browser recovery storage. */
export const projectAuthorityDispositionClearsRecovery = (
  disposition: ProjectAuthorityDisposition,
): boolean => disposition === "clean_at_path";

export type ProjectRecoveryTombstonePreflight = {
  /** Backend authority must never be held hostage by browser storage. */
  applyAuthority: true;
  /** Hide old A recovery UI immediately, even if durable storage is down. */
  suppressRecoveryPrompt: boolean;
  /** Explain that crash-safe suppression could not be persisted. */
  warnRecoveryStorageUnavailable: boolean;
};

/**
 * Decide recovery handling before an authority bundle is committed. A browser
 * storage failure is not a reason to reject an already-published backend B:
 * callers always apply B, suppress A in memory, and surface one truthful
 * warning until a later coherent recovery write can restore crash safety.
 */
export const projectAuthorityRecoveryTombstonePreflight = (
  state: ProjectAuthorityDispositionState,
  candidate: ProjectAuthorityDispositionCandidate,
  tombstoneWritten: boolean,
): ProjectRecoveryTombstonePreflight => {
  const next = projectAuthorityDispositionTransition(state, candidate);
  const isNewUnsavedReplacement = next !== state
    && candidate.authority_disposition === "unsaved_replacement";
  return {
    applyAuthority: true,
    suppressRecoveryPrompt: isNewUnsavedReplacement,
    warnRecoveryStorageUnavailable: isNewUnsavedReplacement && !tombstoneWritten,
  };
};

export type ProjectRecoveryCaptureGuard = {
  writeGeneration: number;
  readGeneration: number;
  authority: ProjectAuthorityToken;
  draftSignature: string;
};

/**
 * Recovery captures must publish one project root, one authority token, and
 * one editor-draft image. This pure gate keeps reverse timer completion and
 * identity replacement from stitching any A component into B.
 */
export const projectRecoveryCaptureIsCurrent = (
  captured: ProjectRecoveryCaptureGuard,
  current: ProjectRecoveryCaptureGuard,
  returnedAuthority: ProjectAuthorityToken,
): boolean => captured.writeGeneration === current.writeGeneration
  && captured.readGeneration === current.readGeneration
  && captured.draftSignature === current.draftSignature
  && projectAuthorityTokenIsCurrent(captured.authority, current.authority)
  && projectAuthorityTokenIsCurrent(captured.authority, returnedAuthority);

/**
 * Recovery/backup eligibility includes the backend checkpoint token, not a
 * partial rendered-snapshot signature. Ancillary-only profile/group edits can
 * preserve that UI signature while still changing persistence and therefore
 * must produce a new coherent recovery image.
 */
export const projectRecoveryAuthoritySignature = (
  uiSignature: string,
  draftSignature: string,
  authority: ProjectAuthorityToken,
): string => `${uiSignature}|scene-block-drafts:${draftSignature}|authority:${authority.project_epoch}:${authority.project_revision}:${authority.checkpoint_hash}`;

/**
 * A recovery acknowledgement changes backend disposition only. It may clear
 * browser recovery storage solely if the recovered draft image and authority
 * application are still exactly the ones which initiated the acknowledgement.
 */
export const projectRecoveryAcknowledgementCanClear = (
  capturedDraftSignature: string,
  currentDraftSignature: string,
  expectedAuthority: ProjectAuthorityToken,
  acknowledgedAuthority: ProjectAuthorityToken,
  currentAuthority: ProjectAuthorityToken,
  applicationCurrent: boolean,
): boolean => applicationCurrent
  && capturedDraftSignature === currentDraftSignature
  && projectAuthorityTokenIsCurrent(expectedAuthority, acknowledgedAuthority)
  && projectAuthorityTokenIsCurrent(expectedAuthority, currentAuthority);

/**
 * A recovery acknowledgement bundle returns after an async backend round trip.
 * By the time it resolves a later project C may already be the live authority:
 * C's own application invalidates this intent (so it is no longer active)
 * and/or replaces the authority token. Applying the stale B acknowledgement
 * bundle would then rewind C's MIDI/OSC/DMX/path/history/disposition UI. It may
 * be applied only while the originating intent is still active and its captured
 * authority token is still the live one; otherwise C must win with no stale
 * runtime/UI side effect. This is the single gate the consumer uses to decide
 * whether the acknowledged bundle is applied at all, so a stale ACK never edits
 * a later C. The durable acknowledged transition remains stored/recoverable.
 */
export const projectRecoveryAcknowledgementApplicationIsCurrent = (
  intentStillActive: boolean,
  capturedAuthority: ProjectAuthorityToken,
  currentAuthority: ProjectAuthorityToken,
): boolean => intentStillActive
  && projectAuthorityTokenIsCurrent(capturedAuthority, currentAuthority);

export type ProjectRecoveryIntentToken = {
  source_serial: number;
  expected_target_serial: number;
  request_generation: number;
  request_id: string;
};

export type ProjectRecoveryIntentDelivery =
  | "consume"
  | "acknowledged"
  | "offer"
  | "keep"
  | "invalidate";

/**
 * The source serial is A and the fenced RecoveryPendingAck publication is
 * exactly A+1. Any other same-or-later authority has superseded this local
 * intent; allowing it to stage A drafts into C would corrupt the UI.
 */
export const projectRecoveryIntentDeliveryForAuthority = (
  intent: ProjectRecoveryIntentToken,
  candidate: {
    recovery_authority_serial: number;
    authority_disposition: ProjectAuthorityDisposition;
    recovery_authority_last_transition: ProjectRecoveryAuthorityTransition;
    checkpoint_hash: string;
  },
): ProjectRecoveryIntentDelivery => {
  if (!Number.isSafeInteger(candidate.recovery_authority_serial)) return "invalidate";
  const transition = candidate.recovery_authority_last_transition;
  if (candidate.recovery_authority_serial === intent.expected_target_serial
    && candidate.authority_disposition === "recovery_pending_ack"
    && transition.kind === "recovery_publication"
    && transition.source_serial === intent.source_serial
    && transition.request_id === intent.request_id
    && transition.target_checkpoint_hash === candidate.checkpoint_hash) {
    return "consume";
  }
  if (candidate.recovery_authority_serial === intent.expected_target_serial
    && candidate.authority_disposition !== "recovery_pending_ack"
    && transition.kind === "recovery_publication"
    && transition.source_serial === intent.source_serial
    && transition.request_id === intent.request_id) {
    return "offer";
  }
  if (candidate.recovery_authority_serial === intent.expected_target_serial + 1
    && candidate.authority_disposition === "unsaved_replacement"
    && transition.kind === "recovery_acknowledged"
    && transition.recovery_publication_serial === intent.expected_target_serial
    && transition.request_id === intent.request_id
    && transition.target_checkpoint_hash === candidate.checkpoint_hash) {
    return "acknowledged";
  }
  if (candidate.recovery_authority_serial >= intent.expected_target_serial) return "invalidate";
  return "keep";
};

/** Renderer restart policy for the one-key v3 intent envelope. */
export const projectRecoveryIntentStartupAction = (
  intent: ProjectRecoveryIntentToken,
  currentSerial: number,
  currentDisposition: ProjectAuthorityDisposition,
  lastTransition: ProjectRecoveryAuthorityTransition,
  currentCheckpointHash: string,
): "offer_checkpoint" | "resume_intent" | "suppress" => {
  if (currentSerial === intent.expected_target_serial
    && currentDisposition === "recovery_pending_ack"
    && lastTransition.kind === "recovery_publication"
    && lastTransition.source_serial === intent.source_serial
    && lastTransition.request_id === intent.request_id
    && lastTransition.target_checkpoint_hash === currentCheckpointHash) {
    return "resume_intent";
  }
  // A full native-process restart restores the durable journal serial before
  // any project publication, but intentionally starts with a fresh runtime
  // disposition. If the recovery publication had reached its target serial
  // and the process died before ACK, re-offer the user's explicit checkpoint
  // rather than silently discarding the only remaining recovery intent.
  if (currentSerial === intent.source_serial
    && currentDisposition !== "recovery_pending_ack") {
    return "offer_checkpoint";
  }
  if (currentSerial === intent.expected_target_serial
    && lastTransition.kind === "recovery_publication"
    && lastTransition.source_serial === intent.source_serial
    && lastTransition.request_id === intent.request_id
    && currentDisposition !== "recovery_pending_ack") return "offer_checkpoint";
  if (currentSerial === intent.expected_target_serial + 1
    && lastTransition.kind === "recovery_acknowledged"
    && lastTransition.recovery_publication_serial === intent.expected_target_serial
    && lastTransition.request_id === intent.request_id) {
    return "offer_checkpoint";
  }
  return "suppress";
};
