import type {
  ProjectAuthorityBundle,
  ProjectAuthorityPublicationKind,
  ProjectInputRuntimeStatus,
} from "./types.ts";
import type {
  ProjectRecoveryCheckpoint,
  ProjectRecoveryIntent,
  ProjectRecoveryStorageState,
} from "./projectRecoveryStorage.ts";
import {
  beginProjectAuthorityApplication,
  projectAuthorityApplicationIsCurrent,
  projectAuthorityBundleMayConsumeRecoveryIntent,
  projectAuthorityCanApply,
  projectAuthorityDispositionTransition,
  projectAuthorityGenerationIsValid,
  projectAuthorityInputRuntimeVerdict,
  projectAuthorityPublicationRequiresMappingHydration,
  projectAuthorityPollMustHydrateMappings,
  projectAuthorityPollPreservesDirtyMappings,
  projectAuthorityReplacementVerdict,
  projectAuthorityTokenIsCurrent,
  projectRecoveryAcknowledgementApplicationIsCurrent,
  projectRecoveryAcknowledgementCanClear,
  projectRecoveryIntentDeliveryForAuthority,
  projectRecoveryIntentStartupAction,
  invalidateProjectAuthorityIdentity,
  type ProjectAuthorityApplication,
  type ProjectAuthorityDispositionState,
  type ProjectAuthoritySyncState,
  type ProjectAuthorityToken,
  type ProjectAuthorityReplacementVerdict,
  type ProjectRecoveryIntentToken,
} from "./projectAuthority.ts";

export type ProjectRecoveryAcknowledgementRequest = {
  expectedEpoch: number;
  expectedRevision: number;
  expectedCheckpointHash: string;
  expectedAuthorityDispositionGeneration: number;
  recoveryRequestId: string;
};

export type ProjectRecoveryIntentConsumerEffects = {
  getActiveIntent: () => ProjectRecoveryIntent | null;
  setActiveIntent: (intent: ProjectRecoveryIntent | null) => void;
  currentAuthority: () => ProjectAuthorityToken;
  currentRecoverySerial: () => number | null;
  currentDraftSignature: () => string;
  expectedDraftSignature: (
    bundle: ProjectAuthorityBundle,
    intent: ProjectRecoveryIntent,
  ) => string;
  stageDrafts: (
    bundle: ProjectAuthorityBundle,
    intent: ProjectRecoveryIntent,
  ) => string;
  persistCheckpoint: (intent: ProjectRecoveryIntent, serial: number) => boolean;
  setRecoveryOffer: (intent: ProjectRecoveryIntent | null) => void;
  setCheckpointNeedsReplacement: (required: boolean) => void;
  resetRecoveryCaptureSignature: () => void;
  tombstone: (serial: number) => void;
  acknowledge: (
    request: ProjectRecoveryAcknowledgementRequest,
  ) => Promise<ProjectAuthorityBundle>;
  applyAcknowledgementRuntimeStatus: (bundle: ProjectAuthorityBundle) => void;
  message: (message: string) => void;
  recoveredMessage: (intent: ProjectRecoveryIntent) => string;
};

export type ProjectRecoveryIntentConsumer = {
  consume: (bundle: ProjectAuthorityBundle) => Promise<void>;
};

export type ProjectRecoveryStartupEffects = {
  setRecoveryOffer: (checkpoint: ProjectRecoveryCheckpoint | null) => void;
  setActiveIntent: (intent: ProjectRecoveryIntent) => void;
  applyReplacement: (bundle: ProjectAuthorityBundle) => ProjectAuthorityReplacementVerdict;
  currentRuntimeState: () => ProjectAuthorityRuntimeState;
  consume: (bundle: ProjectAuthorityBundle) => void;
};

export type ProjectAuthorityBundleApplicationDisposition = "applied" | "duplicate" | "stale";

export type ProjectAuthorityRuntimeState = {
  authority: ProjectAuthorityToken;
  authorityReady: boolean;
  sync: ProjectAuthoritySyncState;
  lastAppliedProjectReplacement: ProjectAuthorityToken | null;
  observedProjectInputRuntimeGeneration: number;
  observedMappingInputRuntimeGeneration: number;
  observedProjectInputRuntime: ProjectInputRuntimeStatus | null;
  observedProjectPathGeneration: number;
  observedProjectPathInitialized: boolean;
  observedProjectHistoryGeneration: number;
  observedProjectHistoryInitialized: boolean;
  observedMappingReplacementGeneration: number;
  observedAuthorityDispositionGeneration: number;
  observedRecoveryAuthoritySerial: number;
  disposition: ProjectAuthorityDispositionState;
  mappingSyncInFlight: boolean;
  hasDirtyMappings: boolean;
};

export type ProjectAuthorityRuntimeApplicationResult = {
  state: ProjectAuthorityRuntimeState;
  disposition: ProjectAuthorityBundleApplicationDisposition;
};

export type ProjectAuthorityReplacementApplicationResult = {
  state: ProjectAuthorityRuntimeState;
  verdict: ProjectAuthorityReplacementVerdict;
  token: ProjectAuthorityToken;
  applicationGenerationAtDecision: number;
  bundle: ProjectAuthorityBundle | null;
};

export type ProjectAuthorityRuntimeStatusPlan = {
  applyPath: boolean;
  applyHistory: boolean;
  applyInput: boolean;
  dispositionChanged: boolean;
  recoveryChanged: boolean;
};

export type ProjectAuthorityBundleCommitOptions = {
  replacement: boolean;
  preserveDirtyMappings: boolean;
  applyInputRuntime: boolean;
};

export type ProjectAuthorityRuntimeEffects = {
  preflightRecoveryDisposition: (bundle: ProjectAuthorityBundle) => boolean;
  prepareBundle: (bundle: ProjectAuthorityBundle) => unknown | null;
  commitBundle: (
    bundle: ProjectAuthorityBundle,
    prepared: unknown | null,
    options: ProjectAuthorityBundleCommitOptions,
  ) => void;
  invalidateMappingIdentity: () => void;
  commitRuntimeStatus: (
    bundle: ProjectAuthorityBundle,
    plan: ProjectAuthorityRuntimeStatusPlan,
  ) => void;
  applyDirtyState: (bundle: ProjectAuthorityBundle) => void;
  consumeRecoveryIntent: (bundle: ProjectAuthorityBundle) => void;
};

export type ProjectAuthorityPollEffects = {
  applyRuntimeStatus: (
    state: ProjectAuthorityRuntimeState,
    bundle: ProjectAuthorityBundle,
  ) => ProjectAuthorityRuntimeApplicationResult;
  applyReplacement: (
    state: ProjectAuthorityRuntimeState,
    bundle: ProjectAuthorityBundle,
  ) => ProjectAuthorityRuntimeApplicationResult;
  applyOrdinaryBundle: (
    state: ProjectAuthorityRuntimeState,
    bundle: ProjectAuthorityBundle,
    preserveDirtyMappings: boolean,
  ) => ProjectAuthorityRuntimeApplicationResult;
};

export type ProjectAuthorityReplacementEffects = {
  applyBundle: (
    state: ProjectAuthorityRuntimeState,
    bundle: ProjectAuthorityBundle,
    application: ProjectAuthorityApplication,
  ) => ProjectAuthorityRuntimeApplicationResult;
};

const authorityPublicationKinds: readonly ProjectAuthorityPublicationKind[] = [
  "runtime_status",
  "mutation",
  "identity_replacement",
  "history_navigation",
];

const authorityDispositions = [
  "clean_at_path",
  "unsaved_replacement",
  "recovery_pending_ack",
  "history_navigation",
  "runtime_sanitize",
] as const;

const recoveryTransitionKinds = [
  "legacy_unknown",
  "project_publication",
  "recovery_publication",
  "history_navigation",
  "clean_save",
  "recovery_acknowledged",
] as const;

const projectInputRuntimeFlagsAreBoolean = (input: ProjectInputRuntimeStatus): boolean => [
  input.midi_clock_active,
  input.midi_control_active,
  input.midi_feedback_output_active,
  input.midi_feedback_runtime_active,
  input.osc_active,
  input.dmx_active,
].every((value) => typeof value === "boolean");

export const projectAuthorityBundleGenerationsAreValid = (bundle: ProjectAuthorityBundle): boolean => {
  if (bundle === null || typeof bundle !== "object") return false;
  const inputRuntime = bundle.input_runtime;
  const recoveryTransition = bundle.recovery_authority_last_transition;
  if (inputRuntime === null || typeof inputRuntime !== "object"
    || recoveryTransition === null || typeof recoveryTransition !== "object") {
    return false;
  }
  return [
    bundle.project_epoch,
    bundle.project_revision,
    bundle.publication_generation,
    bundle.mapping_replacement_generation,
    bundle.authority_disposition_generation,
    bundle.recovery_authority_serial,
    bundle.path_generation,
    bundle.history_generation,
    inputRuntime.project_input_runtime_generation,
    inputRuntime.mapping_input_runtime_generation,
  ].every(projectAuthorityGenerationIsValid)
    && typeof bundle.checkpoint_hash === "string"
    && authorityPublicationKinds.includes(bundle.publication_kind)
    && authorityDispositions.includes(bundle.authority_disposition)
    && recoveryTransitionKinds.includes(recoveryTransition.kind)
    && projectInputRuntimeFlagsAreBoolean(inputRuntime);
};

const projectAuthorityBundleProjectGenerationsAreCurrent = (
  state: ProjectAuthorityRuntimeState,
  bundle: ProjectAuthorityBundle,
): boolean => (!state.observedProjectPathInitialized
  || bundle.path_generation >= state.observedProjectPathGeneration)
  && (!state.observedProjectHistoryInitialized
    || bundle.history_generation >= state.observedProjectHistoryGeneration)
  && bundle.mapping_replacement_generation >= state.observedMappingReplacementGeneration
  && (!state.disposition.dispositionInitialized
    || bundle.authority_disposition_generation >= state.observedAuthorityDispositionGeneration)
  && bundle.recovery_authority_serial >= state.observedRecoveryAuthoritySerial;

const authorityToken = (bundle: ProjectAuthorityBundle): ProjectAuthorityToken => ({
  project_epoch: bundle.project_epoch,
  project_revision: bundle.project_revision,
  checkpoint_hash: bundle.checkpoint_hash,
});

const nextCountersForBundle = (
  state: ProjectAuthorityRuntimeState,
  bundle: ProjectAuthorityBundle,
): ProjectAuthorityRuntimeState => ({
  ...state,
  observedProjectPathGeneration: Math.max(state.observedProjectPathGeneration, bundle.path_generation),
  observedProjectPathInitialized: true,
  observedProjectHistoryGeneration: Math.max(state.observedProjectHistoryGeneration, bundle.history_generation),
  observedProjectHistoryInitialized: true,
  observedMappingReplacementGeneration: Math.max(
    state.observedMappingReplacementGeneration,
    bundle.mapping_replacement_generation,
  ),
  observedAuthorityDispositionGeneration: state.observedAuthorityDispositionGeneration,
  observedRecoveryAuthoritySerial: Math.max(
    state.observedRecoveryAuthoritySerial,
    bundle.recovery_authority_serial,
  ),
});

/**
 * Production authority-bundle entry point. App supplies the Solid batch and
 * persistence callbacks, while this function owns admission, staging order,
 * generation advancement, and the single recovery-consumption edge.
 */
export const applyProjectAuthorityBundleProduction = (
  state: ProjectAuthorityRuntimeState,
  bundle: ProjectAuthorityBundle,
  application: ProjectAuthorityApplication,
  replacement: boolean,
  preserveDirtyMappings: boolean,
  effects: ProjectAuthorityRuntimeEffects,
): ProjectAuthorityRuntimeApplicationResult => {
  const candidate = authorityToken(bundle);
  if (!projectAuthorityBundleGenerationsAreValid(bundle)
    || !projectAuthorityBundleProjectGenerationsAreCurrent(state, bundle)
    || !projectAuthorityCanApply(candidate, state.authority)
    || !projectAuthorityApplicationIsCurrent(state.sync, application)) {
    return { state, disposition: "stale" };
  }

  const prepared = preserveDirtyMappings ? null : effects.prepareBundle(bundle);
  if ((!preserveDirtyMappings && prepared === null)
    || !projectAuthorityApplicationIsCurrent(state.sync, application)) {
    return { state, disposition: "stale" };
  }

  const inputRuntimeVerdict = projectAuthorityInputRuntimeVerdict(
    bundle.input_runtime,
    {
      project_input_runtime_generation: state.observedProjectInputRuntimeGeneration,
      mapping_input_runtime_generation: state.observedMappingInputRuntimeGeneration,
    },
    state.observedProjectInputRuntime,
  );
  const applyInputRuntime = inputRuntimeVerdict === "apply";
  const dispositionBefore = state.disposition;
  const dispositionAfter = projectAuthorityDispositionTransition(dispositionBefore, bundle);
  const dispositionChanged = dispositionAfter !== dispositionBefore;
  const recoveryChanged = bundle.recovery_authority_serial > state.observedRecoveryAuthoritySerial;
  if ((dispositionChanged || recoveryChanged) && !effects.preflightRecoveryDisposition(bundle)) {
    return { state, disposition: "stale" };
  }
  const nextSync = replacement
    ? invalidateProjectAuthorityIdentity(state.sync)
    : state.sync;
  if (replacement) effects.invalidateMappingIdentity();
  effects.commitBundle(bundle, prepared, {
    replacement,
    preserveDirtyMappings,
    applyInputRuntime,
  });

  let nextState = nextCountersForBundle(state, bundle);
  if (applyInputRuntime) {
    nextState = {
      ...nextState,
      observedProjectInputRuntimeGeneration: bundle.input_runtime.project_input_runtime_generation,
      observedMappingInputRuntimeGeneration: bundle.input_runtime.mapping_input_runtime_generation,
      observedProjectInputRuntime: bundle.input_runtime,
    };
  }
  nextState = {
    ...nextState,
    authority: candidate,
    authorityReady: true,
    disposition: dispositionAfter,
    observedAuthorityDispositionGeneration: dispositionAfter.observedDispositionGeneration,
    sync: replacement
      ? { ...nextSync, persistedGeneration: nextSync.localGeneration }
      : nextSync,
  };
  // A token-accepted bundle owns the snapshot/mapping batch. Disposition and
  // recovery effects remain independently fenced so a newer token carrying an
  // older ancillary generation cannot tombstone/clear/ACK an active intent.
  if (dispositionChanged || bundle.publication_kind === "mutation") {
    effects.applyDirtyState(bundle);
  }
  if (recoveryChanged && projectAuthorityBundleMayConsumeRecoveryIntent("applied", candidate, candidate)) {
    effects.consumeRecoveryIntent(bundle);
  }
  return { state: nextState, disposition: "applied" };
};

/** Production same-token event/reply/poll status entry point. */
export const applyProjectAuthorityRuntimeStatusProduction = (
  state: ProjectAuthorityRuntimeState,
  bundle: ProjectAuthorityBundle,
  applyInputRuntime: boolean,
  effects: ProjectAuthorityRuntimeEffects,
): ProjectAuthorityRuntimeApplicationResult => {
  const candidate = authorityToken(bundle);
  if (!projectAuthorityBundleGenerationsAreValid(bundle)
    || !projectAuthorityTokenIsCurrent(candidate, state.authority)) {
    return { state, disposition: "stale" };
  }
  const applyPath = !state.observedProjectPathInitialized
    || bundle.path_generation > state.observedProjectPathGeneration;
  const applyHistory = !state.observedProjectHistoryInitialized
    || bundle.history_generation > state.observedProjectHistoryGeneration;
  const nextDisposition = projectAuthorityDispositionTransition(state.disposition, bundle);
  const dispositionChanged = nextDisposition !== state.disposition;
  const recoveryChanged = bundle.recovery_authority_serial > state.observedRecoveryAuthoritySerial;
  const inputRuntimeVerdict = applyInputRuntime
    ? projectAuthorityInputRuntimeVerdict(
      bundle.input_runtime,
      {
        project_input_runtime_generation: state.observedProjectInputRuntimeGeneration,
        mapping_input_runtime_generation: state.observedMappingInputRuntimeGeneration,
      },
      state.observedProjectInputRuntime,
    )
    : "duplicate";
  const applyInput = inputRuntimeVerdict === "apply";
  const applied = applyPath || applyHistory || applyInput || dispositionChanged || recoveryChanged;
  if (!applied) return { state, disposition: "duplicate" };

  const plan = { applyPath, applyHistory, applyInput, dispositionChanged, recoveryChanged };
  if ((dispositionChanged || recoveryChanged) && !effects.preflightRecoveryDisposition(bundle)) {
    return { state, disposition: "stale" };
  }
  effects.commitRuntimeStatus(bundle, plan);
  let nextState = {
    ...state,
    disposition: nextDisposition,
    observedAuthorityDispositionGeneration: nextDisposition.observedDispositionGeneration,
    observedMappingReplacementGeneration: Math.max(
      state.observedMappingReplacementGeneration,
      bundle.mapping_replacement_generation,
    ),
    observedRecoveryAuthoritySerial: Math.max(
      state.observedRecoveryAuthoritySerial,
      bundle.recovery_authority_serial,
    ),
  };
  if (applyPath) {
    nextState = {
      ...nextState,
      observedProjectPathGeneration: bundle.path_generation,
      observedProjectPathInitialized: true,
    };
  }
  if (applyHistory) {
    nextState = {
      ...nextState,
      observedProjectHistoryGeneration: bundle.history_generation,
      observedProjectHistoryInitialized: true,
    };
  }
  if (applyInput) {
    nextState = {
      ...nextState,
      observedProjectInputRuntimeGeneration: bundle.input_runtime.project_input_runtime_generation,
      observedMappingInputRuntimeGeneration: bundle.input_runtime.mapping_input_runtime_generation,
      observedProjectInputRuntime: bundle.input_runtime,
    };
  }
  if (dispositionChanged) effects.applyDirtyState(bundle);
  if (recoveryChanged && projectAuthorityBundleMayConsumeRecoveryIntent("applied", candidate, candidate)) {
    effects.consumeRecoveryIntent(bundle);
  }
  return { state: nextState, disposition: "applied" };
};

/** Production replacement route shared by authority events and replies. */
export const applyProjectAuthorityReplacementProduction = (
  state: ProjectAuthorityRuntimeState,
  bundle: ProjectAuthorityBundle,
  effects: ProjectAuthorityReplacementEffects,
): ProjectAuthorityReplacementApplicationResult => {
  const candidate = authorityToken(bundle);
  const verdict = projectAuthorityReplacementVerdict(
    candidate,
    state.authority,
    state.lastAppliedProjectReplacement,
    state.authorityReady,
  );
  if (verdict !== "apply") {
    return {
      state,
      verdict,
      token: candidate,
      applicationGenerationAtDecision: state.sync.applicationGeneration,
      bundle: verdict === "duplicate" ? bundle : null,
    };
  }
  const started = beginProjectAuthorityRuntimeApplication(state);
  const applied = effects.applyBundle(started.state, bundle, started.application);
  if (applied.disposition !== "applied") {
    return {
      state: applied.state,
      verdict: "stale",
      token: candidate,
      applicationGenerationAtDecision: applied.state.sync.applicationGeneration,
      bundle: null,
    };
  }
  return {
    state: {
      ...applied.state,
      lastAppliedProjectReplacement: candidate,
    },
    verdict: "apply",
    token: candidate,
    applicationGenerationAtDecision: started.application.applicationGeneration,
    bundle,
  };
};

/** Production poll route; callbacks are the actual App apply entry points. */
export const applyPolledProjectAuthorityBundleProduction = (
  state: ProjectAuthorityRuntimeState,
  bundle: ProjectAuthorityBundle,
  effects: ProjectAuthorityPollEffects,
): ProjectAuthorityRuntimeApplicationResult => {
  const candidate = authorityToken(bundle);
  if (!projectAuthorityBundleGenerationsAreValid(bundle)) {
    return { state, disposition: "stale" };
  }
  if (projectAuthorityTokenIsCurrent(candidate, state.authority)) {
    return effects.applyRuntimeStatus(state, bundle);
  }
  if (!projectAuthorityCanApply(candidate, state.authority)) {
    return { state, disposition: "stale" };
  }
  if (projectAuthorityPublicationRequiresMappingHydration(bundle.publication_kind)
    || projectAuthorityPollMustHydrateMappings(
      bundle.mapping_replacement_generation,
      state.observedMappingReplacementGeneration,
    )) {
    return effects.applyReplacement(state, bundle);
  }
  const preserveDirtyMappings = projectAuthorityPollPreservesDirtyMappings(
    candidate,
    state.authority,
    state.sync,
    state.mappingSyncInFlight,
    bundle.publication_kind,
  );
  return effects.applyOrdinaryBundle(state, bundle, preserveDirtyMappings);
};

/**
 * One browser-recovery handoff can be observed by a Tauri event, the command
 * reply, or a later authority poll. This controller is the sole production
 * draft-staging and acknowledgement owner used by App and the E3 restart
 * driver. Its private single-flight promise prevents duplicate deliveries
 * from issuing a second acknowledgement.
 */
export const createProjectRecoveryIntentConsumerProduction = (
  effects: ProjectRecoveryIntentConsumerEffects,
): ProjectRecoveryIntentConsumer => {
  let inFlight: { intent: ProjectRecoveryIntent; promise: Promise<void> } | null = null;

  const retainAcknowledgedCheckpoint = (
    intent: ProjectRecoveryIntent,
    recoveryAuthoritySerial: number,
  ): boolean => {
    if (!effects.persistCheckpoint(intent, recoveryAuthoritySerial)) {
      effects.message(
        "Recovered project is active, but its crash-recovery checkpoint could not be updated. Keep Syndocal open and save the project.",
      );
      return false;
    }
    effects.setActiveIntent(null);
    effects.setCheckpointNeedsReplacement(true);
    effects.resetRecoveryCaptureSignature();
    effects.setRecoveryOffer(null);
    effects.message(effects.recoveredMessage(intent));
    return true;
  };

  const consume = (bundle: ProjectAuthorityBundle): Promise<void> => {
    const intent = effects.getActiveIntent();
    if (!intent) return Promise.resolve();
    const delivery = projectRecoveryIntentDeliveryForAuthority(intent, bundle);
    if (delivery === "keep") return Promise.resolve();
    if (delivery === "offer") {
      if (effects.persistCheckpoint(intent, bundle.recovery_authority_serial)) {
        effects.setActiveIntent(null);
        effects.setCheckpointNeedsReplacement(false);
        effects.setRecoveryOffer(intent);
        effects.message(
          "Project recovery did not publish. The recovery checkpoint remains available to retry.",
        );
      } else {
        effects.message(
          "Project recovery did not publish, and browser storage could not be refreshed. Keep Syndocal open and retry recovery.",
        );
      }
      return Promise.resolve();
    }
    if (delivery === "invalidate") {
      effects.setActiveIntent(null);
      effects.setCheckpointNeedsReplacement(true);
      effects.resetRecoveryCaptureSignature();
      effects.setRecoveryOffer(null);
      effects.tombstone(bundle.recovery_authority_serial);
      return Promise.resolve();
    }
    if (delivery === "acknowledged") {
      const expectedDraftSignature = effects.expectedDraftSignature(bundle, intent);
      if (effects.currentDraftSignature() === expectedDraftSignature) {
        retainAcknowledgedCheckpoint(intent, bundle.recovery_authority_serial);
      } else {
        effects.message(
          "Recovered project acknowledgement arrived after local draft changes. The recovery intent remains available until the project is saved.",
        );
      }
      return Promise.resolve();
    }
    if (inFlight?.intent === intent) return inFlight.promise;

    const capturedIntent = intent;
    const capturedAuthority = authorityToken(bundle);
    const work = (async () => {
      if (effects.getActiveIntent() !== capturedIntent
        || !projectAuthorityTokenIsCurrent(capturedAuthority, effects.currentAuthority())
        || effects.currentRecoverySerial() !== capturedIntent.expected_target_serial) {
        return;
      }
      const stagedDraftSignature = effects.stageDrafts(bundle, capturedIntent);
      try {
        const acknowledged = await effects.acknowledge({
          expectedEpoch: bundle.project_epoch,
          expectedRevision: bundle.project_revision,
          expectedCheckpointHash: bundle.checkpoint_hash,
          expectedAuthorityDispositionGeneration: bundle.authority_disposition_generation,
          recoveryRequestId: capturedIntent.request_id,
        });
        const acknowledgementApplicationCurrent = projectRecoveryAcknowledgementApplicationIsCurrent(
          effects.getActiveIntent() === capturedIntent,
          capturedAuthority,
          effects.currentAuthority(),
        );
        if (!acknowledgementApplicationCurrent) return;

        effects.applyAcknowledgementRuntimeStatus(acknowledged);
        if (acknowledged.recovery_authority_serial === capturedIntent.expected_target_serial + 1
          && projectRecoveryAcknowledgementCanClear(
            stagedDraftSignature,
            effects.currentDraftSignature(),
            capturedAuthority,
            authorityToken(acknowledged),
            effects.currentAuthority(),
            acknowledgementApplicationCurrent,
          )) {
          retainAcknowledgedCheckpoint(capturedIntent, acknowledged.recovery_authority_serial);
        } else {
          // B is still authoritative but local drafts changed while ACK was in
          // flight. Preserve those drafts and force a coherent replacement
          // capture instead of clearing browser recovery.
          effects.setActiveIntent(null);
          effects.setCheckpointNeedsReplacement(true);
          effects.resetRecoveryCaptureSignature();
        }
      } catch (error) {
        if (effects.getActiveIntent() === capturedIntent) {
          effects.message(`Recovered project is waiting for acknowledgement: ${String(error)}`);
        }
      }
    })();
    const settled = work.finally(() => {
      if (inFlight?.promise === settled) inFlight = null;
    });
    inFlight = { intent: capturedIntent, promise: settled };
    return settled;
  };

  return { consume };
};

/**
 * Compatibility-load guard used by the real post-load fallback path. It is
 * intentionally independent from raw snapshot/history refreshes: once C has
 * advanced either authority or application generation, a late B is stale.
 */
export const projectAuthorityFallbackIsCurrent = (
  state: ProjectAuthorityRuntimeState,
  capturedAuthority: ProjectAuthorityToken,
  capturedApplication: ProjectAuthorityApplication,
  returnedAuthority: ProjectAuthorityToken,
): boolean => projectAuthorityTokenIsCurrent(capturedAuthority, returnedAuthority)
  && projectAuthorityTokenIsCurrent(capturedAuthority, state.authority)
  && projectAuthorityApplicationIsCurrent(state.sync, capturedApplication);

/**
 * Startup-only recovery delivery fence. A renderer may install a stored
 * recovery intent after an early B replacement event already hydrated the
 * exact same bundle. Only that duplicate startup handoff may consume the
 * intent; ordinary duplicate poll/event paths never call this predicate.
 */
export const projectAuthorityStartupRecoveryDeliveryIsCurrent = (
  state: ProjectAuthorityRuntimeState,
  intent: ProjectRecoveryIntentToken,
  bundle: ProjectAuthorityBundle,
): boolean => {
  const candidate = authorityToken(bundle);
  return state.authorityReady
    && state.lastAppliedProjectReplacement !== null
    && projectAuthorityTokenIsCurrent(candidate, state.authority)
    && projectAuthorityTokenIsCurrent(candidate, state.lastAppliedProjectReplacement)
    && state.observedRecoveryAuthoritySerial === bundle.recovery_authority_serial
    && projectRecoveryIntentDeliveryForAuthority(intent, bundle) === "consume";
};

/** Execute the startup-only handoff after the predicate has admitted it. */
export const deliverProjectAuthorityStartupRecoveryIntentProduction = (
  state: ProjectAuthorityRuntimeState,
  intent: ProjectRecoveryIntentToken,
  bundle: ProjectAuthorityBundle,
  consume: (bundle: ProjectAuthorityBundle) => void,
): boolean => {
  if (!projectAuthorityStartupRecoveryDeliveryIsCurrent(state, intent, bundle)) return false;
  consume(bundle);
  return true;
};

/**
 * Coherent startup handoff after the native journal status and authority
 * bundle have selected one v3 browser envelope. App and the E3 restart driver
 * both use this exact route; no caller independently interprets the stored
 * intent or invents a duplicate-delivery exception.
 */
export const applyProjectRecoveryStartupProduction = (
  bundle: ProjectAuthorityBundle,
  stored: ProjectRecoveryStorageState,
  effects: ProjectRecoveryStartupEffects,
): "offer" | "resume" | "suppress" => {
  if (stored.kind === "checkpoint") {
    effects.setRecoveryOffer(stored.checkpoint);
    return "offer";
  }
  if (stored.kind !== "intent") return "suppress";
  const startup = projectRecoveryIntentStartupAction(
    stored.intent,
    bundle.recovery_authority_serial,
    bundle.authority_disposition,
    bundle.recovery_authority_last_transition,
    bundle.checkpoint_hash,
  );
  if (startup === "offer_checkpoint") {
    effects.setRecoveryOffer(stored.intent.checkpoint);
    return "offer";
  }
  if (startup !== "resume_intent") return "suppress";

  effects.setActiveIntent(stored.intent);
  const verdict = effects.applyReplacement(bundle);
  if (verdict === "apply") {
    effects.consume(bundle);
  } else if (verdict === "duplicate") {
    deliverProjectAuthorityStartupRecoveryIntentProduction(
      effects.currentRuntimeState(),
      stored.intent,
      bundle,
      effects.consume,
    );
  }
  return "resume";
};

/** Small helper for production tests to reserve an application generation. */
export const beginProjectAuthorityRuntimeApplication = (
  state: ProjectAuthorityRuntimeState,
): { state: ProjectAuthorityRuntimeState; application: ProjectAuthorityApplication } => {
  const started = beginProjectAuthorityApplication(state.sync);
  return { state: { ...state, sync: started.state }, application: started.application };
};
