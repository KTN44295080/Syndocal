import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import {
  acknowledgeProjectAuthorityPersist,
  beginProjectAuthorityApplication,
  beginProjectAuthorityRequest,
  createProjectAuthoritySyncState,
  invalidateProjectAuthorityIdentity,
  noteLocalProjectAuthorityEdit,
  projectAuthorityApplicationIsCurrent,
  projectAuthorityCanApply,
  projectAuthorityCanApplyHistoryStatus,
  projectAuthorityBundleMayConsumeRecoveryIntent,
  projectAuthorityInputRuntimeVerdict,
  projectAuthorityDispositionClearsRecovery,
  projectAuthorityDispositionTransition,
  projectAuthorityGenerationIsValid,
  projectAuthorityRecoveryTombstonePreflight,
  projectAuthorityHasDirtyMappings,
  projectAuthorityMappingCommitRetiredInputs,
  projectAuthorityPollPreservesDirtyMappings,
  projectAuthorityPollMustHydrateMappings,
  projectAuthorityPublicationRequiresMappingHydration,
  projectAuthorityReplacementVerdict,
  projectAuthorityReplacementContinuationIsCurrent,
  projectRecoveryCaptureIsCurrent,
  projectRecoveryAuthoritySignature,
  projectRecoveryAcknowledgementCanClear,
  projectRecoveryAcknowledgementApplicationIsCurrent,
  projectRecoveryIntentDeliveryForAuthority,
  projectRecoveryIntentStartupAction,
  projectAuthorityResponseIsCurrent,
  projectAuthorityShouldRetryPersist,
  successfulStaleProjectControlMappingsReplyIsRetryable,
  projectAuthorityTokenIsCurrent,
  rebaseDirtyProjectAuthorityMappings,
  runAfterProjectAuthorityFlush,
} from "../src/projectAuthority.ts";
import {
  flushProjectControlMappingsAuthorityBridge,
  projectControlMappingsFlushDecision,
  projectControlMappingsFlushTimerAction,
} from "../src/projectControlMappingsAuthorityBridge.ts";
import {
  applyPolledProjectAuthorityBundleProduction,
  applyProjectAuthorityBundleProduction,
  projectAuthorityBundleGenerationsAreValid,
  projectAuthorityFallbackIsCurrent,
  projectAuthorityStartupRecoveryDeliveryIsCurrent,
  deliverProjectAuthorityStartupRecoveryIntentProduction,
  applyProjectAuthorityReplacementProduction,
  applyProjectAuthorityRuntimeStatusProduction,
  beginProjectAuthorityRuntimeApplication,
  projectAuthorityInlineReplacementIsCurrent,
} from "../src/projectAuthorityRuntime.ts";
import {
  createProjectRecoveryCheckpoint,
  loadProjectRecoveryCheckpoint,
  loadProjectRecoveryStorageState,
  projectRecoveryCheckpointIsTombstoned,
  registerProjectRecoveryIntent,
  saveProjectRecoveryCheckpoint,
  tombstoneProjectRecoveryCheckpoint,
} from "../src/projectRecoveryStorage.ts";
import {
  projectTransactionRecoveryCanAdopt,
  projectTransactionRecoveryIsTerminal,
  projectTransactionShapeFingerprint,
} from "../src/types.ts";

let state = createProjectAuthoritySyncState();
state = noteLocalProjectAuthorityEdit(state);
const delayedA = beginProjectAuthorityRequest(state);
state = delayedA.state;

// A mapping persist starts for identity A, then B hydrates before A responds.
state = invalidateProjectAuthorityIdentity(state);
const afterBHydrate = state;
assert.equal(projectAuthorityResponseIsCurrent(afterBHydrate, delayedA.request), false);
assert.equal(
  acknowledgeProjectAuthorityPersist(afterBHydrate, delayedA.request, delayedA.state.localGeneration),
  afterBHydrate,
  "a delayed A persist response must neither acknowledge nor retry into B",
);

// B starts UI work, C applies while B waits for snapshot/history refresh.
const delayedBApplication = beginProjectAuthorityApplication(state);
state = delayedBApplication.state;
const appliedC = beginProjectAuthorityApplication(state);
state = appliedC.state;
assert.equal(projectAuthorityApplicationIsCurrent(state, delayedBApplication.application), false);
assert.equal(projectAuthorityApplicationIsCurrent(state, appliedC.application), true);

// An edit while R1 is in flight stays dirty after R1's ack and schedules R2.
state = createProjectAuthoritySyncState();
state = noteLocalProjectAuthorityEdit(state);
const r1 = beginProjectAuthorityRequest(state);
state = r1.state;
const sentGeneration = state.localGeneration;
state = noteLocalProjectAuthorityEdit(state);
state = acknowledgeProjectAuthorityPersist(state, r1.request, sentGeneration);
assert.equal(projectAuthorityHasDirtyMappings(state), true);
assert.equal(projectAuthorityShouldRetryPersist(state, r1.request.identityGeneration), true);

// A local B edit can coexist with an externally authored C project revision.
// The poll must adopt C as B's CAS base, invalidate R1, and retain B rather
// than hydrating C's arrays over the user's newer local mappings.
const externalC = { project_epoch: 0, project_revision: 1, checkpoint_hash: "C" };
const localA = { project_epoch: 0, project_revision: 0, checkpoint_hash: "A" };
let rebased = createProjectAuthoritySyncState();
rebased = noteLocalProjectAuthorityEdit(rebased); // local B arrays are dirty
const delayedPersistB = beginProjectAuthorityRequest(rebased);
rebased = delayedPersistB.state;
assert.equal(
  projectAuthorityPollPreservesDirtyMappings(externalC, localA, rebased, true),
  true,
  "A local mapping edit must survive an external C poll while B persists",
);
const rebasedRequestGeneration = rebased.requestGeneration;
rebased = rebaseDirtyProjectAuthorityMappings(rebased, true);
assert.equal(rebased.requestGeneration, rebasedRequestGeneration + 1);
assert.equal(projectAuthorityResponseIsCurrent(rebased, delayedPersistB.request), false);
assert.equal(projectAuthorityHasDirtyMappings(rebased), true);
assert.equal(projectAuthorityShouldRetryPersist(rebased, delayedPersistB.request.identityGeneration), true);

// The mapping writer may commit B, while an ordinary authority poll returns
// that exact B token before the writer's IPC reply. The poll invalidates R1's
// renderer request generation but does not make B foreign: one R2 ACK is safe
// and is required before Save can obtain trusted mapping authority.
const committedB = { project_epoch: 0, project_revision: 1, checkpoint_hash: "B" };
assert.equal(
  successfulStaleProjectControlMappingsReplyIsRetryable(
    rebased,
    delayedPersistB.request,
    committedB,
    committedB,
    false,
  ),
  true,
  "an exact poll-observed successful B mapping reply may receive one ACK replay",
);
assert.equal(
  projectControlMappingsFlushDecision(
    "retryable",
    rebased,
    delayedPersistB.request.identityGeneration,
    false,
    1,
  ),
  "retry",
  "flush must continue from stale R1 to exact R2 instead of failing Save",
);
const r2 = beginProjectAuthorityRequest(rebased);
let afterR2 = acknowledgeProjectAuthorityPersist(
  r2.state,
  r2.request,
  r2.state.localGeneration,
);
assert.equal(projectAuthorityHasDirtyMappings(afterR2), false, "R2 exact ACK clears the local B generation");
assert.equal(
  projectControlMappingsFlushDecision(
    "acknowledged",
    afterR2,
    r2.request.identityGeneration,
    false,
    0,
  ),
  "acknowledged",
  "Save may proceed only after the exact R2 acknowledgement",
);
assert.equal(
  successfulStaleProjectControlMappingsReplyIsRetryable(
    rebased,
    delayedPersistB.request,
    committedB,
    { project_epoch: 0, project_revision: 2, checkpoint_hash: "foreign-C" },
    false,
  ),
  false,
  "a later foreign C authority must not be replayed over",
);
const replacedIdentity = invalidateProjectAuthorityIdentity(rebased);
assert.equal(
  successfulStaleProjectControlMappingsReplyIsRetryable(
    replacedIdentity,
    delayedPersistB.request,
    committedB,
    committedB,
    false,
  ),
  false,
  "identity replacement must reject a delayed B reply",
);
assert.equal(
  projectControlMappingsFlushDecision(
    "untrusted",
    rebased,
    delayedPersistB.request.identityGeneration,
    false,
    1,
  ),
  "fail",
  "CAS, validation, and worker-retirement failures remain fail-closed",
);
assert.equal(
  projectControlMappingsFlushDecision(
    "retryable",
    rebased,
    delayedPersistB.request.identityGeneration,
    false,
    0,
  ),
  "fail",
  "a second stale reply must not make Save retry forever",
);

// R2 can be invalidated only by a separate refresh/authority request; an
// equal-B ordinary poll returns no bundle and cannot create this path. Model
// that R2 stale result with the retry timer its finally scheduled. The bridge
// must execute exactly R1/R2, consume its own R3 timer, and return untrusted.
const r2AuthorityWrite = beginProjectAuthorityRequest(rebased);
const separateRefreshDuringR2 = beginProjectAuthorityRequest(r2AuthorityWrite.state);
assert.equal(
  projectAuthorityResponseIsCurrent(separateRefreshDuringR2.state, r2AuthorityWrite.request),
  false,
  "only a separate refresh/request can invalidate R2; an equal-B poll cannot",
);
const flushTimerOwner = Symbol("explicit-flush");
let pendingRetry = { owner: flushTimerOwner, fired: false };
let bridgeSync = rebased;
const cancelFlushOwnedRetry = (owner) => {
  if (pendingRetry?.owner !== owner) return false;
  pendingRetry = null;
  return true;
};
const r2StaleTerminal = projectControlMappingsFlushDecision(
  "retryable",
  rebased,
  delayedPersistB.request.identityGeneration,
  false,
  0,
);
assert.equal(r2StaleTerminal, "fail", "R2 stale after the one replay is terminal");
let bridgeAttempts = 0;
const bridgeResult = await flushProjectControlMappingsAuthorityBridge({
  persist: async (owner) => {
    bridgeAttempts += 1;
    pendingRetry = { owner, fired: false };
    if (bridgeAttempts === 2) {
      // This is the production-only second-stale cause proven above: a
      // separate refresh wins while R2 awaits its backend response.
      bridgeSync = separateRefreshDuringR2.state;
    }
    return { kind: "retryable", authority: committedB };
  },
  cancelTimer: (owner) => owner === undefined
    ? cancelFlushOwnedRetry(pendingRetry?.owner)
    : cancelFlushOwnedRetry(owner),
  state: () => ({
    disposed: false,
    authorityReady: true,
    sync: bridgeSync,
    mappingSyncInFlight: false,
    timerFlushOwner: pendingRetry?.owner ?? null,
  }),
});
assert.equal(bridgeAttempts, 2, "flush retries R1 once as R2 and never dispatches R3");
assert.equal(bridgeResult.trusted, false, "R2 stale remains terminally untrusted");
assert.equal(pendingRetry, null, "no R3 timer remains after R2 terminal failure");
pendingRetry = { owner: null, fired: false };
assert.equal(
  projectControlMappingsFlushTimerAction(r2StaleTerminal === "fail", pendingRetry.owner, flushTimerOwner),
  "preserve",
  "a normal autosave timer is not owned by the explicit flush",
);
assert.notEqual(pendingRetry, null, "normal autosave remains available after flush failure");

// A poll can arrive after an identity/Undo replacement event was lost. These
// publications own their mapping arrays, so a dirty A debounce must not be
// rebased into B. A later ordinary backend mutation does not erase the
// durable mapping-replacement generation evidence.
assert.equal(projectAuthorityPublicationRequiresMappingHydration("identity_replacement"), true);
assert.equal(projectAuthorityPublicationRequiresMappingHydration("history_navigation"), true);
assert.equal(projectAuthorityPublicationRequiresMappingHydration("mutation"), false);
assert.equal(
  projectAuthorityPollPreservesDirtyMappings(externalC, localA, rebased, true, "history_navigation"),
  false,
  "a history-navigation poll must hydrate authoritative mappings instead of rebasing dirty A",
);
assert.equal(
  projectAuthorityPollMustHydrateMappings(1, 0),
  true,
  "Undo's mapping replacement forces hydration even if a later Mutation is last",
);
assert.equal(
  projectAuthorityPollMustHydrateMappings(1, 1),
  false,
  "after Undo was applied, a later ordinary Mutation may rebase a dirty local mapping draft",
);

const events = [];
await runAfterProjectAuthorityFlush(
  async () => { events.push("flush"); },
  async () => { events.push("start"); },
);
assert.deepEqual(events, ["flush", "start"], "input construction must wait for the authority flush");

const empty = { project_epoch: 0, project_revision: 0, checkpoint_hash: "" };
const a = { project_epoch: 2, project_revision: 3, checkpoint_hash: "a" };
const b = { project_epoch: 3, project_revision: 0, checkpoint_hash: "b" };
assert.equal(projectAuthorityCanApply(a, empty), true);
assert.equal(projectAuthorityCanApply(b, a), true);
assert.equal(projectAuthorityCanApply(a, b), false);
assert.equal(projectAuthorityCanApply({ ...b, checkpoint_hash: "different" }, b), false);
assert.equal(projectAuthorityMappingCommitRetiredInputs({ mapping_runtimes_retired: true }), true);
assert.equal(projectAuthorityMappingCommitRetiredInputs({}), false);

// Authority disposition is durable in the coordinator bundle, so event-only,
// reply-first, and poll-after-event-loss all make the same dirty/recovery
// decision. A saved A becomes clean again only when Undo returns to A's exact
// saved identity/hash/path; Redo B and an unrelated C remain dirty.
const savedA = {
  project_epoch: 30,
  project_revision: 0,
  checkpoint_hash: "saved-A",
  current_project_path: "C:/shows/A.sdc",
};
let disposition = projectAuthorityDispositionTransition(
  {
    baseline: null,
    observedDispositionGeneration: 0,
    dispositionInitialized: false,
    dirty: true,
  },
  { ...savedA, authority_disposition_generation: 1, authority_disposition: "clean_at_path" },
);
assert.equal(disposition.dirty, false, "CleanAtPath establishes the saved authority baseline");
disposition = projectAuthorityDispositionTransition(
  disposition,
  { ...savedA, authority_disposition_generation: 2, authority_disposition: "history_navigation" },
);
assert.equal(disposition.dirty, false, "saved A → edit B → Undo A is clean again");
disposition = projectAuthorityDispositionTransition(
  disposition,
  {
    project_epoch: 30,
    project_revision: 2,
    checkpoint_hash: "edited-B",
    current_project_path: "C:/shows/A.sdc",
    authority_disposition_generation: 3,
    authority_disposition: "history_navigation",
  },
);
assert.equal(disposition.dirty, true, "Redo B remains dirty against saved A");
disposition = projectAuthorityDispositionTransition(
  disposition,
  {
    project_epoch: 30,
    project_revision: 3,
    checkpoint_hash: "different-C",
    current_project_path: "C:/shows/A.sdc",
    authority_disposition_generation: 4,
    authority_disposition: "history_navigation",
  },
);
assert.equal(disposition.dirty, true, "Undo to a non-saved C must not claim clean");

// A template/DVC/backup event has no direct command continuation, yet it is
// unambiguously unsaved. Its duplicate reply must be a no-op and cannot erase
// a user action after the event applied. Recovery has the same dirty truth but
// never clears browser drafts until its explicit acknowledgement path.
const eventOnlyUnsaved = {
  project_epoch: 31,
  project_revision: 0,
  checkpoint_hash: "template-D",
  current_project_path: null,
  authority_disposition_generation: 5,
  authority_disposition: "unsaved_replacement",
};
const afterEventOnlyUnsaved = projectAuthorityDispositionTransition(disposition, eventOnlyUnsaved);
assert.equal(afterEventOnlyUnsaved.dirty, true, "event-only DVC/template replacement is dirty");
assert.equal(
  projectAuthorityDispositionTransition(afterEventOnlyUnsaved, eventOnlyUnsaved),
  afterEventOnlyUnsaved,
  "reply after event is an idempotent disposition no-op",
);
const replyFirstUnsaved = projectAuthorityDispositionTransition(disposition, eventOnlyUnsaved);
assert.equal(replyFirstUnsaved.dirty, true, "reply-first DVC/template replacement is dirty");
assert.equal(
  projectAuthorityDispositionTransition(replyFirstUnsaved, eventOnlyUnsaved),
  replyFirstUnsaved,
  "event after reply is an idempotent disposition no-op",
);
const eventLostUnsaved = projectAuthorityDispositionTransition(disposition, eventOnlyUnsaved);
assert.equal(eventLostUnsaved.dirty, true, "poll-only delivery after event loss remains dirty");
assert.equal(projectAuthorityDispositionClearsRecovery("unsaved_replacement"), false);
assert.equal(projectAuthorityDispositionClearsRecovery("recovery_pending_ack"), false);
assert.equal(projectAuthorityDispositionClearsRecovery("clean_at_path"), true);
const afterRecoveryEvent = projectAuthorityDispositionTransition(
  afterEventOnlyUnsaved,
  {
    ...eventOnlyUnsaved,
    authority_disposition_generation: 6,
    authority_disposition: "recovery_pending_ack",
  },
);
assert.equal(afterRecoveryEvent.dirty, true, "event-only recovery remains unsaved and retains drafts");
const afterRuntimeSanitize = projectAuthorityDispositionTransition(
  afterRecoveryEvent,
  {
    ...eventOnlyUnsaved,
    authority_disposition_generation: 7,
    authority_disposition: "runtime_sanitize",
  },
);
assert.equal(afterRuntimeSanitize.dirty, true, "RuntimeSanitize does not alter dirty/recovery truth");

// Generation zero is a valid coordinator default, not an implicit
// uninitialized sentinel. The first zero initializes the durable state; a
// later same-zero Mutation is a no-op so App can separately recompute dirty
// state from its saved authority baseline.
const genZeroInitial = projectAuthorityDispositionTransition(
  { baseline: null, observedDispositionGeneration: 0, dispositionInitialized: false, dirty: true },
  {
    ...savedA,
    authority_disposition_generation: 0,
    authority_disposition: "clean_at_path",
  },
);
assert.equal(genZeroInitial.dispositionInitialized, true);
assert.equal(genZeroInitial.dirty, false);
assert.equal(
  projectAuthorityDispositionTransition(
    genZeroInitial,
    {
      ...savedA,
      authority_disposition_generation: 0,
      authority_disposition: "runtime_sanitize",
    },
  ),
  genZeroInitial,
  "a later same-zero disposition is an idempotent no-op",
);

// History status has a second, UI-visible generation.  An old A event or
// refresh may have the same checkpoint token as A, but it must never replace
// the newer B history stack after an identity publication.  Likewise, an old
// H1 reply may not replace a same-token H2 Clear/Commit/Undo transition.
const historyA = { project_epoch: 9, project_revision: 1, checkpoint_hash: "history-A" };
const historyB = { project_epoch: 10, project_revision: 0, checkpoint_hash: "history-B" };
assert.equal(
  projectAuthorityCanApplyHistoryStatus({ ...historyA, history_generation: 1 }, historyB, 0),
  false,
  "a delayed generic transaction event from A cannot overwrite B history",
);
assert.equal(
  projectAuthorityCanApplyHistoryStatus({ ...historyB, history_generation: 1 }, historyB, 2),
  false,
  "a delayed H1 refresh cannot overwrite a newer same-token H2 status",
);
assert.equal(
  projectAuthorityCanApplyHistoryStatus({ ...historyB, history_generation: 3 }, historyB, 2),
  true,
  "the latest same-token history status remains applicable",
);

// A fenced replacement is delivered both as an immediate command reply and
// an authority event. Once B has actually hydrated, a duplicate B must not
// invalidate a newer local mapping edit or reset B input/path state again.
const observedB = { project_epoch: 4, project_revision: 2, checkpoint_hash: "B" };
assert.equal(
  projectAuthorityReplacementVerdict(observedB, observedB, observedB, true),
  "duplicate",
  "event B then reply B must be a strict replacement no-op",
);
assert.equal(
  projectAuthorityReplacementVerdict(observedB, observedB, null, true),
  "apply",
  "an observed status-only B still needs its first replacement hydrate",
);
assert.equal(
  projectAuthorityReplacementVerdict(observedB, { project_epoch: 0, project_revision: 0, checkpoint_hash: "" }, null, false),
  "apply",
  "bootstrap remains able to apply its first authority bundle",
);
assert.equal(
  projectAuthorityReplacementVerdict(
    { project_epoch: 4, project_revision: 1, checkpoint_hash: "B" },
    observedB,
    observedB,
    true,
  ),
  "stale",
  "a late older B must not follow up after C is already observed",
);
assert.equal(
  projectAuthorityReplacementVerdict(
    { project_epoch: 5, project_revision: 0, checkpoint_hash: "D" },
    observedB,
    observedB,
    true,
  ),
  "apply",
  "a newer D replacement remains eligible",
);
let replacementState = createProjectAuthoritySyncState();
const eventB = beginProjectAuthorityApplication(replacementState);
replacementState = eventB.state;
assert.equal(
  projectAuthorityReplacementContinuationIsCurrent(
    "duplicate",
    observedB,
    replacementState.applicationGeneration,
    observedB,
    replacementState,
  ),
  true,
  "event B then reply B permits only the originating reply continuation",
);
const eventC = beginProjectAuthorityApplication(replacementState);
replacementState = eventC.state;
assert.equal(
  projectAuthorityReplacementContinuationIsCurrent(
    "duplicate",
    observedB,
    eventB.application.applicationGeneration,
    { project_epoch: 5, project_revision: 0, checkpoint_hash: "C" },
    replacementState,
  ),
  false,
  "event B then C makes the delayed B reply continuation stale",
);

// MIDI, OSC, and DMX Learn all retain this exact production identity token
// before waiting on hardware. A replacement while waiting must discard the
// old result rather than edit B mappings or reconnect an A worker.
const learnA = { project_epoch: 7, project_revision: 4, checkpoint_hash: "learn-A" };
const learnB = { project_epoch: 8, project_revision: 0, checkpoint_hash: "learn-B" };
for (const source of ["MIDI", "OSC", "DMX"]) {
  assert.equal(
    projectAuthorityTokenIsCurrent(learnA, learnB),
    false,
    `${source} Learn completion from A must be ignored after B publishes`,
  );
  assert.equal(projectAuthorityTokenIsCurrent(learnB, learnB), true);
}

// Input runtime is one coherent pair of callback fences. A delayed A event,
// reply, or poll cannot lower either member or overwrite B's live truth; a
// same-generation DMX liveness change remains applicable because it is
// runtime-only and does not imply a worker replacement.
const runtimeA = {
  project_input_runtime_generation: 8,
  mapping_input_runtime_generation: 12,
  midi_clock_active: true,
  midi_control_active: true,
  midi_feedback_output_active: true,
  midi_feedback_runtime_active: true,
  osc_active: true,
  dmx_active: false,
};
const runtimeB = {
  ...runtimeA,
  project_input_runtime_generation: 9,
  mapping_input_runtime_generation: 13,
  dmx_active: true,
};
assert.equal(
  projectAuthorityInputRuntimeVerdict(runtimeB, runtimeA, runtimeA),
  "apply",
  "a newer project+mapping runtime pair applies",
);
assert.equal(
  projectAuthorityInputRuntimeVerdict(runtimeA, runtimeB, runtimeB),
  "stale",
  "a delayed A runtime pair cannot rewind B",
);
assert.equal(
  projectAuthorityInputRuntimeVerdict(
    { ...runtimeB, mapping_input_runtime_generation: 12 },
    runtimeB,
    runtimeB,
  ),
  "stale",
  "a mixed pair which regresses one callback fence is rejected as a whole",
);
assert.equal(
  projectAuthorityInputRuntimeVerdict({ ...runtimeB }, runtimeB, runtimeB),
  "duplicate",
  "an exact duplicate runtime delivery is side-effect free",
);
assert.equal(
  projectAuthorityInputRuntimeVerdict({ ...runtimeB, dmx_active: false }, runtimeB, runtimeB),
  "apply",
  "same-generation runtime-only DMX liveness still converges",
);
assert.equal(projectAuthorityGenerationIsValid(0), true, "generation zero is valid");
assert.equal(projectAuthorityGenerationIsValid(Number.MAX_SAFE_INTEGER), true, "safe max generation is valid");
assert.equal(projectAuthorityGenerationIsValid(Number.MAX_SAFE_INTEGER + 1), false, "unsafe generation fails closed");
assert.equal(
  projectAuthorityInputRuntimeVerdict(
    { ...runtimeB, project_input_runtime_generation: Number.MAX_SAFE_INTEGER + 1 },
    runtimeB,
    runtimeB,
  ),
  "stale",
  "an unsafe runtime generation cannot poison the observed fence",
);
assert.equal(
  projectAuthorityBundleMayConsumeRecoveryIntent("applied", learnB, learnB),
  true,
  "only an accepted current bundle owns recovery intent consumption",
);
assert.equal(
  projectAuthorityBundleMayConsumeRecoveryIntent("stale", learnA, learnB),
  false,
  "a rejected stale poll cannot tombstone or ACK the current recovery intent",
);
assert.equal(
  projectAuthorityBundleMayConsumeRecoveryIntent("duplicate", learnB, learnB),
  false,
  "an exact duplicate bundle cannot consume recovery twice",
);
let recoveryIntentSideEffects = 0;
const deliverRecoveryIntent = (application, candidate, current) => {
  if (projectAuthorityBundleMayConsumeRecoveryIntent(application, candidate, current)) {
    recoveryIntentSideEffects += 1;
  }
};
// Active C recovery intent: a delayed A event/reply/poll and a duplicate B
// poll are all rejected before they can abandon, tombstone, write, message,
// or stage drafts. Only the one accepted B application owns delivery.
deliverRecoveryIntent("stale", learnA, learnB);
deliverRecoveryIntent("stale", learnB, learnB);
deliverRecoveryIntent("duplicate", learnB, learnB);
assert.equal(recoveryIntentSideEffects, 0, "stale/duplicate A/B poll deliveries have no recovery side effect");
deliverRecoveryIntent("applied", learnB, learnB);
deliverRecoveryIntent("duplicate", learnB, learnB);
assert.equal(recoveryIntentSideEffects, 1, "one accepted B application consumes recovery exactly once");

// The following tests drive the imported production orchestration used by
// App.tsx. Effects are observable callbacks around the real staged-batch,
// runtime-status, replacement, and poll entry points; no verdict helper is
// reimplemented in this harness.
const makeProductionBundle = (token, input, overrides = {}) => ({
  project_epoch: token.project_epoch,
  project_revision: token.project_revision,
  checkpoint_hash: token.checkpoint_hash,
  publication_generation: overrides.publication_generation ?? token.project_revision + 1,
  publication_kind: overrides.publication_kind ?? "mutation",
  mapping_replacement_generation: overrides.mapping_replacement_generation ?? 0,
  authority_disposition_generation: overrides.authority_disposition_generation ?? 1,
  authority_disposition: overrides.authority_disposition ?? "runtime_sanitize",
  recovery_authority_serial: overrides.recovery_authority_serial ?? 1,
  recovery_authority_last_transition: overrides.recovery_authority_last_transition ?? { kind: "project_publication" },
  path_generation: overrides.path_generation ?? 1,
  history_generation: overrides.history_generation ?? 1,
  current_project_path: overrides.current_project_path ?? null,
  snapshot: overrides.snapshot ?? {},
  profiles: overrides.profiles ?? [],
  fixture_groups: overrides.fixture_groups ?? [],
  operator_policy: overrides.operator_policy ?? null,
  midi_mappings: overrides.midi_mappings ?? [],
  osc_mappings: overrides.osc_mappings ?? [],
  dmx_mappings: overrides.dmx_mappings ?? [],
  dj_track_triggers: overrides.dj_track_triggers ?? [],
  history: overrides.history ?? { history_generation: overrides.history_generation ?? 1 },
  input_runtime: input,
});

const makeProductionState = (authority) => ({
  authority,
  authorityReady: true,
  sync: createProjectAuthoritySyncState(),
  lastAppliedProjectReplacement: authority,
  observedProjectInputRuntimeGeneration: 0,
  observedMappingInputRuntimeGeneration: 0,
  observedProjectInputRuntime: null,
  observedProjectPathGeneration: 0,
  observedProjectPathInitialized: false,
  observedProjectHistoryGeneration: 0,
  observedProjectHistoryInitialized: false,
  observedMappingReplacementGeneration: 0,
  observedAuthorityDispositionGeneration: 0,
  observedRecoveryAuthoritySerial: 0,
  disposition: {
    baseline: null,
    observedDispositionGeneration: 0,
    dispositionInitialized: false,
    dirty: true,
  },
  mappingSyncInFlight: false,
  hasDirtyMappings: false,
});

const makeProductionEffects = () => {
  const trace = {
    prepares: 0,
    commits: [],
    runtimeCommits: [],
    preflights: 0,
    dirty: 0,
    recovery: 0,
    invalidations: 0,
    visible: {
      mapping: "A-mapping",
      snapshot: "A-snapshot",
      groups: "A-groups",
      policy: "A-policy",
      runtime: "A-runtime",
    },
  };
  return {
    trace,
    effects: {
      preflightRecoveryDisposition: () => {
        trace.preflights += 1;
        return true;
      },
      prepareBundle: () => {
        trace.prepares += 1;
        return { prepared: true };
      },
      invalidateMappingIdentity: () => { trace.invalidations += 1; },
      commitBundle: (bundle, _prepared, options) => {
        trace.commits.push(options);
        if (!options.preserveDirtyMappings) trace.visible.mapping = bundle.midi_mappings;
        trace.visible.snapshot = bundle.snapshot;
        trace.visible.groups = bundle.fixture_groups;
        trace.visible.policy = bundle.operator_policy;
        if (options.applyInputRuntime) trace.visible.runtime = bundle.input_runtime;
      },
      commitRuntimeStatus: (bundle, plan) => {
        trace.runtimeCommits.push(plan);
        if (plan.applyInput) trace.visible.runtime = bundle.input_runtime;
      },
      applyDirtyState: () => { trace.dirty += 1; },
      consumeRecoveryIntent: () => { trace.recovery += 1; },
    },
  };
};

const productionTokenA = { project_epoch: 20, project_revision: 0, checkpoint_hash: "production-A" };
const productionTokenB = { project_epoch: 20, project_revision: 1, checkpoint_hash: "production-B" };
const productionTokenC = { project_epoch: 20, project_revision: 2, checkpoint_hash: "production-C" };
const productionTokenD = { project_epoch: 20, project_revision: 3, checkpoint_hash: "production-D" };
const productionInputA = { ...runtimeA, project_input_runtime_generation: 8, mapping_input_runtime_generation: 12 };
const productionInputB = { ...runtimeB, project_input_runtime_generation: 9, mapping_input_runtime_generation: 13 };
const productionBundleB = makeProductionBundle(productionTokenB, productionInputB, {
  snapshot: { image: "B-snapshot" },
  fixture_groups: [{ id: 2, label: "B-groups" }],
  operator_policy: { lock_on_load: false, lock_mode: null },
  midi_mappings: [{ id: "B-mapping" }],
});
assert.equal(
  projectAuthorityBundleGenerationsAreValid(
    makeProductionBundle(productionTokenB, productionInputB, {
      publication_generation: Number.MAX_SAFE_INTEGER + 1,
    }),
  ),
  false,
  "an exhausted publication generation fails closed before any batch or recovery side effect",
);
const boundedHarness = makeProductionEffects();
const boundedStarted = beginProjectAuthorityRuntimeApplication(makeProductionState(productionTokenA));
const boundedResult = applyProjectAuthorityBundleProduction(
  boundedStarted.state,
  makeProductionBundle(productionTokenB, productionInputB, {
    publication_generation: Number.MAX_SAFE_INTEGER + 1,
  }),
  boundedStarted.application,
  true,
  false,
  boundedHarness.effects,
);
assert.equal(boundedResult.disposition, "stale", "an exhausted bundle is rejected by the production entry");
assert.equal(boundedHarness.trace.commits.length, 0, "an exhausted bundle stages no Solid batch");
assert.equal(boundedHarness.trace.preflights, 0, "an exhausted bundle runs no recovery preflight");

const applyProductionReplacement = (state, bundle, harness) => applyProjectAuthorityReplacementProduction(
  state,
  bundle,
  {
    applyBundle: (startedState, candidate, application) => applyProjectAuthorityBundleProduction(
      startedState,
      candidate,
      application,
      true,
      false,
      harness.effects,
    ),
  },
);

// Event -> reply and reply -> event both apply B once. A local edit after B
// survives the exact duplicate and the duplicate emits no batch/dirty/recovery
// callback.
let productionHarness = makeProductionEffects();
let productionResult = applyProductionReplacement(
  makeProductionState(productionTokenA),
  productionBundleB,
  productionHarness,
);
assert.equal(productionResult.verdict, "apply");
let productionAfterEvent = {
  ...productionResult.state,
  sync: noteLocalProjectAuthorityEdit(productionResult.state.sync),
};
productionHarness.trace.visible.mapping = "local-post-B-edit";
const productionDuplicateReply = applyProductionReplacement(
  productionAfterEvent,
  productionBundleB,
  productionHarness,
);
assert.equal(productionDuplicateReply.verdict, "duplicate", "event then reply B is an exact duplicate");
assert.equal(productionHarness.trace.commits.length, 1, "event then reply stages one Solid batch");
assert.equal(productionHarness.trace.dirty, 1, "event then reply runs dirty side effects once");
assert.equal(productionDuplicateReply.state.sync.localGeneration, productionAfterEvent.sync.localGeneration);
assert.equal(
  productionHarness.trace.visible.mapping,
  "local-post-B-edit",
  "event then reply preserves a local mapping edit after B",
);

productionHarness = makeProductionEffects();
let productionReplyFirst = applyProductionReplacement(
  makeProductionState(productionTokenA),
  productionBundleB,
  productionHarness,
);
const productionEventDuplicate = applyProductionReplacement(
  productionReplyFirst.state,
  productionBundleB,
  productionHarness,
);
assert.equal(productionEventDuplicate.verdict, "duplicate", "reply then event B is an exact duplicate");
assert.equal(productionHarness.trace.commits.length, 1, "reply then event stages one Solid batch");

// Startup recovery handshake race: B may already be hydrated before the
// renderer installs the stored intent. Only the startup duplicate consumes
// that newly installed intent; later event/poll duplicates remain no-ops.
const startupIntent = {
  source_serial: 4,
  expected_target_serial: 5,
  request_generation: 1,
  request_id: "startup-recovery-request",
};
const startupBundleB = makeProductionBundle(productionTokenB, productionInputB, {
  authority_disposition: "recovery_pending_ack",
  recovery_authority_serial: 5,
  recovery_authority_last_transition: {
    kind: "recovery_publication",
    source_serial: startupIntent.source_serial,
    request_id: startupIntent.request_id,
    target_checkpoint_hash: productionTokenB.checkpoint_hash,
  },
});
const startupHarness = makeProductionEffects();
let startupIntentActive = false;
startupHarness.effects.consumeRecoveryIntent = () => {
  if (startupIntentActive) {
    startupHarness.trace.recovery += 1;
    startupIntentActive = false;
  }
};
const earlyStartupB = applyProductionReplacement(
  makeProductionState(productionTokenA),
  startupBundleB,
  startupHarness,
);
assert.equal(earlyStartupB.verdict, "apply", "early B replacement hydrates before recovery handshake");
assert.equal(startupHarness.trace.recovery, 0, "early B with no installed intent does not consume");
startupIntentActive = true;
const startupDuplicateB = applyProductionReplacement(
  earlyStartupB.state,
  startupBundleB,
  startupHarness,
);
assert.equal(startupDuplicateB.verdict, "duplicate", "handshake sees the already-applied B as duplicate");
assert.equal(
  projectAuthorityStartupRecoveryDeliveryIsCurrent(startupDuplicateB.state, startupIntent, startupBundleB),
  true,
  "startup duplicate matches exact B token, serial, transition, and current authority",
);
assert.equal(
  deliverProjectAuthorityStartupRecoveryIntentProduction(
    startupDuplicateB.state,
    startupIntent,
    startupBundleB,
    (bundle) => startupHarness.effects.consumeRecoveryIntent(bundle),
  ),
  true,
  "startup production handoff delivers the admitted intent",
);
assert.equal(startupHarness.trace.recovery, 1, "startup duplicate consumes/ACKs/stages the intent once");
const startupEventDuplicate = applyProductionReplacement(
  startupDuplicateB.state,
  startupBundleB,
  startupHarness,
);
assert.equal(startupEventDuplicate.verdict, "duplicate", "later duplicate event remains a no-op");
const startupPollDuplicate = applyPolledProjectAuthorityBundleProduction(
  startupDuplicateB.state,
  startupBundleB,
  {
    applyRuntimeStatus: (state, bundle) => applyProjectAuthorityRuntimeStatusProduction(
      state,
      bundle,
      true,
      startupHarness.effects,
    ),
    applyReplacement: () => { throw new Error("duplicate startup poll reached replacement"); },
    applyOrdinaryBundle: () => { throw new Error("duplicate startup poll reached ordinary apply"); },
  },
);
assert.equal(startupPollDuplicate.disposition, "duplicate", "later duplicate poll remains a no-op");
assert.equal(startupHarness.trace.recovery, 1, "later event/poll duplicates do not consume twice");
assert.equal(
  deliverProjectAuthorityStartupRecoveryIntentProduction(
    startupDuplicateB.state,
    startupIntent,
    makeProductionBundle(productionTokenC, productionInputB, {
      authority_disposition: "recovery_pending_ack",
      recovery_authority_serial: 5,
      recovery_authority_last_transition: startupBundleB.recovery_authority_last_transition,
    }),
    () => { throw new Error("mismatched token reached startup consume"); },
  ),
  false,
  "mismatched startup token cannot consume",
);
assert.equal(
  deliverProjectAuthorityStartupRecoveryIntentProduction(
    startupDuplicateB.state,
    startupIntent,
    makeProductionBundle(productionTokenB, productionInputB, {
      authority_disposition: "recovery_pending_ack",
      recovery_authority_serial: 4,
      recovery_authority_last_transition: startupBundleB.recovery_authority_last_transition,
    }),
    () => { throw new Error("mismatched serial reached startup consume"); },
  ),
  false,
  "mismatched startup serial cannot consume",
);
assert.equal(
  deliverProjectAuthorityStartupRecoveryIntentProduction(
    startupDuplicateB.state,
    startupIntent,
    makeProductionBundle(productionTokenA, productionInputA, {
      authority_disposition: "recovery_pending_ack",
      recovery_authority_serial: 5,
      recovery_authority_last_transition: startupBundleB.recovery_authority_last_transition,
    }),
    () => { throw new Error("older bundle reached startup consume"); },
  ),
  false,
  "older startup bundle cannot consume",
);

// B runtime truth cannot be rewound by lower-generation A through either the
// poll route or the recovery-status route, and neither route consumes C's
// active recovery intent.
const productionRuntimeState = {
  ...makeProductionState(productionTokenB),
  observedProjectInputRuntimeGeneration: productionInputB.project_input_runtime_generation,
  observedMappingInputRuntimeGeneration: productionInputB.mapping_input_runtime_generation,
  observedProjectInputRuntime: productionInputB,
  observedProjectPathGeneration: 1,
  observedProjectPathInitialized: true,
  observedProjectHistoryGeneration: 1,
  observedProjectHistoryInitialized: true,
  observedMappingReplacementGeneration: 0,
  observedAuthorityDispositionGeneration: 1,
  observedRecoveryAuthoritySerial: 1,
  disposition: {
    baseline: null,
    observedDispositionGeneration: 1,
    dispositionInitialized: true,
    dirty: true,
  },
};
const staleGenerationHarness = makeProductionEffects();
const staleGenerationStarted = beginProjectAuthorityRuntimeApplication(productionRuntimeState);
const staleGenerationResult = applyProjectAuthorityBundleProduction(
  staleGenerationStarted.state,
  makeProductionBundle(productionTokenC, productionInputB, { path_generation: 0 }),
  staleGenerationStarted.application,
  true,
  false,
  staleGenerationHarness.effects,
);
assert.equal(staleGenerationResult.disposition, "stale", "a newer token cannot carry a regressed path generation");
assert.equal(staleGenerationHarness.trace.commits.length, 0, "a regressed project generation stages no batch");
assert.equal(staleGenerationHarness.trace.preflights, 0, "a regressed project generation runs no recovery preflight");
const delayedProductionA = makeProductionBundle(productionTokenB, productionInputA);
const delayedRuntimeHarness = makeProductionEffects();
delayedRuntimeHarness.trace.visible.runtime = "B-runtime";
const delayedPoll = applyPolledProjectAuthorityBundleProduction(
  productionRuntimeState,
  delayedProductionA,
  {
    applyRuntimeStatus: (state, bundle) => applyProjectAuthorityRuntimeStatusProduction(
      state,
      bundle,
      true,
      delayedRuntimeHarness.effects,
    ),
    applyReplacement: (state) => ({ state, disposition: "stale" }),
    applyOrdinaryBundle: (state) => ({ state, disposition: "stale" }),
  },
);
assert.equal(delayedPoll.disposition, "duplicate", "poll route rejects lower same-token input as a no-op");
const delayedRecoveryStatus = applyProjectAuthorityRuntimeStatusProduction(
  delayedPoll.state,
  delayedProductionA,
  false,
  delayedRuntimeHarness.effects,
);
assert.equal(delayedRecoveryStatus.disposition, "duplicate", "recovery-status route rejects lower input");
assert.equal(delayedRuntimeHarness.trace.runtimeCommits.length, 0, "lower A emits no runtime batch");
assert.equal(delayedRuntimeHarness.trace.preflights, 0, "lower A emits no recovery preflight");
assert.equal(delayedRuntimeHarness.trace.dirty, 0, "lower A emits no dirty side effect");
assert.equal(delayedRuntimeHarness.trace.recovery, 0, "stale poll/recovery cannot consume active C intent");
assert.equal(delayedRuntimeHarness.trace.visible.runtime, "B-runtime", "lower A cannot rewind B runtime truth");

let activeRecoveryIntent = true;
const stalePollHarness = makeProductionEffects();
stalePollHarness.trace.visible = {
  mapping: "B-mapping",
  snapshot: "B-snapshot",
  groups: "B-groups",
  policy: "B-policy",
  runtime: "B-runtime",
};
stalePollHarness.effects.consumeRecoveryIntent = () => {
  activeRecoveryIntent = false;
  stalePollHarness.trace.recovery += 1;
};
const stalePoll = applyPolledProjectAuthorityBundleProduction(
  productionRuntimeState,
  makeProductionBundle(productionTokenA, productionInputA, { recovery_authority_serial: 0 }),
  {
    applyRuntimeStatus: () => { throw new Error("stale poll reached runtime status"); },
    applyReplacement: () => { throw new Error("stale poll reached replacement"); },
    applyOrdinaryBundle: () => { throw new Error("stale poll reached ordinary apply"); },
  },
);
assert.equal(stalePoll.disposition, "stale", "older-token poll is rejected at the production poll entry");
assert.equal(activeRecoveryIntent, true, "stale poll cannot abandon the active recovery intent");
assert.equal(stalePollHarness.trace.preflights, 0, "stale poll cannot run recovery preflight");
assert.equal(stalePollHarness.trace.recovery, 0, "stale poll cannot consume the active recovery intent");
assert.deepEqual(
  stalePollHarness.trace.visible,
  {
    mapping: "B-mapping",
    snapshot: "B-snapshot",
    groups: "B-groups",
    policy: "B-policy",
    runtime: "B-runtime",
  },
  "stale A cannot overwrite B snapshot/group/policy/runtime state",
);

// Dirty A is forced to hydrate for identity/history B, while ordinary C
// rebases the local mapping edit and keeps the local arrays.
const dirtyAState = {
  ...makeProductionState(productionTokenA),
  sync: noteLocalProjectAuthorityEdit(createProjectAuthoritySyncState()),
};
const identityB = makeProductionBundle(productionTokenB, productionInputB, {
  publication_kind: "identity_replacement",
  mapping_replacement_generation: 1,
});
const mappingHarness = makeProductionEffects();
const identityResult = applyPolledProjectAuthorityBundleProduction(
  dirtyAState,
  identityB,
  {
    applyRuntimeStatus: (state, bundle) => applyProjectAuthorityRuntimeStatusProduction(
      state,
      bundle,
      true,
      mappingHarness.effects,
    ),
    applyReplacement: (state, bundle) => {
      const replacement = applyProductionReplacement(state, bundle, mappingHarness);
      return {
        state: replacement.state,
        disposition: replacement.verdict === "apply"
          ? "applied"
          : replacement.verdict === "duplicate" ? "duplicate" : "stale",
      };
    },
    applyOrdinaryBundle: (state) => ({ state, disposition: "stale" }),
  },
);
assert.equal(identityResult.disposition, "applied", "identity B takes the forced-hydrate route");
assert.equal(mappingHarness.trace.commits.length, 1);
assert.equal(mappingHarness.trace.commits[0].preserveDirtyMappings, false);

const dirtyBState = {
  ...identityResult.state,
  sync: noteLocalProjectAuthorityEdit(identityResult.state.sync),
};
const ordinaryC = makeProductionBundle(productionTokenC, productionInputB, {
  publication_kind: "mutation",
  mapping_replacement_generation: 1,
});
const ordinaryHarness = makeProductionEffects();
let ordinaryRebased = false;
const ordinaryResult = applyPolledProjectAuthorityBundleProduction(
  dirtyBState,
  ordinaryC,
  {
    applyRuntimeStatus: (state, bundle) => applyProjectAuthorityRuntimeStatusProduction(
      state,
      bundle,
      true,
      ordinaryHarness.effects,
    ),
    applyReplacement: (state) => ({ state, disposition: "stale" }),
    applyOrdinaryBundle: (state, bundle, preserveDirtyMappings) => {
      const started = beginProjectAuthorityRuntimeApplication(state);
      const rebased = preserveDirtyMappings
        ? { ...started.state, sync: rebaseDirtyProjectAuthorityMappings(started.state.sync, true) }
        : started.state;
      ordinaryRebased = preserveDirtyMappings;
      return applyProjectAuthorityBundleProduction(
        rebased,
        bundle,
        started.application,
        false,
        preserveDirtyMappings,
        ordinaryHarness.effects,
      );
    },
  },
);
assert.equal(ordinaryResult.disposition, "applied", "ordinary C applies");
assert.equal(ordinaryRebased, true, "ordinary C rebases dirty A mappings");
assert.equal(ordinaryHarness.trace.commits[0].preserveDirtyMappings, true);

const historyD = makeProductionBundle(productionTokenD, productionInputB, {
  publication_kind: "history_navigation",
  mapping_replacement_generation: 2,
});
const historyHarness = makeProductionEffects();
const historyResult = applyPolledProjectAuthorityBundleProduction(
  ordinaryResult.state,
  historyD,
  {
    applyRuntimeStatus: (state, bundle) => applyProjectAuthorityRuntimeStatusProduction(
      state,
      bundle,
      true,
      historyHarness.effects,
    ),
    applyReplacement: (state, bundle) => {
      const replacement = applyProductionReplacement(state, bundle, historyHarness);
      return {
        state: replacement.state,
        disposition: replacement.verdict === "apply" ? "applied" : replacement.verdict === "duplicate" ? "duplicate" : "stale",
      };
    },
    applyOrdinaryBundle: (state) => ({ state, disposition: "stale" }),
  },
);
assert.equal(historyResult.disposition, "applied", "history B takes the forced-hydrate route");
assert.equal(historyHarness.trace.commits[0].preserveDirtyMappings, false);

// A paired reply can synchronously own B while the renderer still observes A.
// Its inline admission must allow the normal monotonic bundle apply without
// weakening either a mismatched returned token or a later C application.
const inlineBState = makeProductionState(productionTokenA);
const inlineBStarted = beginProjectAuthorityRuntimeApplication(inlineBState);
assert.equal(
  projectAuthorityInlineReplacementIsCurrent(
    inlineBStarted.state,
    productionTokenB,
    inlineBStarted.application,
    productionTokenB,
  ),
  true,
  "paired inline B is current before any later application starts",
);
const inlineBHarness = makeProductionEffects();
const inlineBResult = applyProjectAuthorityBundleProduction(
  inlineBStarted.state,
  identityB,
  inlineBStarted.application,
  true,
  false,
  inlineBHarness.effects,
);
assert.equal(inlineBResult.disposition, "applied", "paired inline B applies over renderer A");
assert.deepEqual(inlineBResult.state.authority, productionTokenB);
assert.equal(
  projectAuthorityInlineReplacementIsCurrent(
    inlineBStarted.state,
    productionTokenB,
    inlineBStarted.application,
    productionTokenC,
  ),
  false,
  "a mismatched inline C bundle cannot satisfy paired B",
);
const inlineCStarted = beginProjectAuthorityRuntimeApplication({
  ...inlineBStarted.state,
  authority: productionTokenC,
});
assert.equal(
  projectAuthorityInlineReplacementIsCurrent(
    inlineCStarted.state,
    productionTokenB,
    inlineBStarted.application,
    productionTokenB,
  ),
  false,
  "paired inline B is rejected after a later C application starts",
);

// A compatibility fallback captured for B cannot apply after C advances the
// authority/application generation; no raw refresh/storage/reset callback is
// reachable from this guard.
const fallbackBState = makeProductionState(productionTokenB);
const fallbackBStarted = beginProjectAuthorityRuntimeApplication(fallbackBState);
const fallbackCStarted = beginProjectAuthorityRuntimeApplication({
  ...fallbackBStarted.state,
  authority: productionTokenC,
});
assert.equal(
  projectAuthorityFallbackIsCurrent(
    fallbackCStarted.state,
    productionTokenB,
    fallbackBStarted.application,
    productionTokenB,
  ),
  false,
  "late B fallback is rejected after C without stale refresh side effects",
);

const appSource = readFileSync(new URL("../src/App.tsx", import.meta.url), "utf8");
assert.match(appSource, /invoke<FixtureProfileSummary>\("preview_custom_fixture_profile"/);
assert.doesNotMatch(
  appSource.slice(appSource.indexOf("const projectMutationCommands"), appSource.indexOf("const projectMutationCommands") + 2_000),
  /create_custom_fixture_profile/,
);
assert.match(appSource, /captureProjectAuthorityIdentity\(\)[\s\S]*?preview_custom_fixture_profile[\s\S]*?isProjectAuthorityIdentityCurrent/);
const loadedProjectApplicationSource = appSource.slice(
  appSource.indexOf("const applyLoadedProjectResult"),
  appSource.indexOf("const resetRetiredProjectControlInputUi"),
);
assert.match(
  loadedProjectApplicationSource,
  /const inlineAuthority = result\.authority;[\s\S]*?const bundle = inlineAuthority \?\? await fetchProjectAuthorityBundle\(result\);[\s\S]*?inlineAuthority[\s\S]*?projectAuthorityInlineReplacementIsCurrent[\s\S]*?: projectAuthorityFallbackIsCurrent/,
  "paired project-load authority uses the synchronous inline guard while compatibility fetches retain the fallback guard",
);
assert.doesNotMatch(
  loadedProjectApplicationSource,
  /const bundle = await fetchProjectAuthorityBundle\(result\);/,
  "paired project-load authority must not unconditionally yield to the compatibility fetch",
);

// Recovery invalidation uses one storage key. A v1 payload remains readable
// until an UnsavedReplacement atomically overwrites it with a v3 tombstone;
// a later coherent B recovery write replaces that tombstone in one write.
const recoveryStorage = new Map();
globalThis.window = {
  localStorage: {
    getItem: (key) => recoveryStorage.get(key) ?? null,
    setItem: (key, value) => recoveryStorage.set(key, value),
    removeItem: (key) => recoveryStorage.delete(key),
  },
};
const recoveryProject = { version: 1, app: "Syndocal", snapshot: { fixtures: [], cues: [] } };
const recoveryA = createProjectRecoveryCheckpoint(recoveryProject, "C:/shows/A.sdc", "A");
assert.equal(saveProjectRecoveryCheckpoint(recoveryA), true);
assert.equal(loadProjectRecoveryCheckpoint()?.signature, "A");
assert.equal(tombstoneProjectRecoveryCheckpoint(4), true);
assert.equal(projectRecoveryCheckpointIsTombstoned(), true);
assert.equal(loadProjectRecoveryCheckpoint(), null, "B must not offer stale A after a crash/reload");
const recoveryB = createProjectRecoveryCheckpoint(recoveryProject, null, "B");
assert.equal(saveProjectRecoveryCheckpoint(recoveryB, 4), true);
assert.equal(projectRecoveryCheckpointIsTombstoned(), false);
assert.equal(loadProjectRecoveryCheckpoint(4)?.signature, "B");
assert.equal(loadProjectRecoveryStorageState(5).kind, "none", "a checkpoint stamped for serial 4 cannot cross serial 5");

const recoveryIntent = registerProjectRecoveryIntent(recoveryB, 4, 1);
assert.ok(recoveryIntent);
assert.equal(loadProjectRecoveryStorageState(4).kind, "intent");
assert.equal(loadProjectRecoveryStorageState(5).kind, "intent");
assert.equal(loadProjectRecoveryStorageState(6).kind, "intent");
assert.equal(loadProjectRecoveryStorageState(7).kind, "none");
const recoveryPublication = {
  kind: "recovery_publication",
  source_serial: 4,
  request_id: recoveryIntent.request_id,
  target_checkpoint_hash: "recovery-B",
};
const recoveryAcknowledged = {
  kind: "recovery_acknowledged",
  recovery_publication_serial: 5,
  request_id: recoveryIntent.request_id,
  target_checkpoint_hash: "recovery-B",
};
assert.equal(
  projectRecoveryIntentDeliveryForAuthority(recoveryIntent, {
    recovery_authority_serial: 5,
    authority_disposition: "recovery_pending_ack",
    recovery_authority_last_transition: recoveryPublication,
    checkpoint_hash: "recovery-B",
  }),
  "consume",
);
assert.equal(
  projectRecoveryIntentDeliveryForAuthority(recoveryIntent, {
    recovery_authority_serial: 5,
    authority_disposition: "clean_at_path",
    recovery_authority_last_transition: recoveryPublication,
    checkpoint_hash: "still-A",
  }),
  "offer",
  "journal-first recovery publication failure must keep the explicit checkpoint retryable",
);
const competingRecoveryIntent = registerProjectRecoveryIntent(recoveryB, 4, 2);
assert.ok(competingRecoveryIntent);
assert.notEqual(competingRecoveryIntent.request_id, recoveryIntent.request_id);
assert.equal(
  projectRecoveryIntentDeliveryForAuthority(competingRecoveryIntent, {
    recovery_authority_serial: 5,
    authority_disposition: "recovery_pending_ack",
    recovery_authority_last_transition: recoveryPublication,
    checkpoint_hash: "recovery-B",
  }),
  "invalidate",
  "a second renderer intent at the same source serial cannot alias the published request",
);
assert.equal(
  projectRecoveryIntentDeliveryForAuthority(recoveryIntent, {
    recovery_authority_serial: 5,
    authority_disposition: "clean_at_path",
    recovery_authority_last_transition: { kind: "project_publication" },
    checkpoint_hash: "recovery-B",
  }),
  "invalidate",
);
assert.equal(
  projectRecoveryIntentDeliveryForAuthority(recoveryIntent, {
    recovery_authority_serial: 6,
    authority_disposition: "unsaved_replacement",
    recovery_authority_last_transition: recoveryAcknowledged,
    checkpoint_hash: "recovery-B",
  }),
  "acknowledged",
);
assert.equal(
  projectRecoveryIntentDeliveryForAuthority(recoveryIntent, {
    recovery_authority_serial: 6,
    authority_disposition: "unsaved_replacement",
    recovery_authority_last_transition: { kind: "project_publication" },
    checkpoint_hash: "other-C",
  }),
  "invalidate",
  "an unrelated C publication at the ACK serial must not masquerade as recovery acknowledgement",
);
assert.equal(projectRecoveryIntentStartupAction(
  recoveryIntent,
  4,
  "runtime_sanitize",
  { kind: "legacy_unknown" },
  "runtime-default",
), "offer_checkpoint");
assert.equal(projectRecoveryIntentStartupAction(
  recoveryIntent,
  5,
  "recovery_pending_ack",
  recoveryPublication,
  "recovery-B",
), "resume_intent");
assert.equal(
  projectRecoveryIntentStartupAction(
    recoveryIntent,
    5,
    "runtime_sanitize",
    recoveryPublication,
    "runtime-default",
  ),
  "offer_checkpoint",
  "a full native restart after B publication must re-offer the explicit recovery intent",
);
assert.equal(
  projectRecoveryIntentStartupAction(
    recoveryIntent,
    6,
    "runtime_sanitize",
    recoveryAcknowledged,
    "runtime-default",
  ),
  "offer_checkpoint",
  "a crash after durable ACK but before browser cleanup must retain the recovery offer",
);
assert.equal(projectRecoveryIntentStartupAction(
  recoveryIntent,
  6,
  "runtime_sanitize",
  { kind: "project_publication" },
  "runtime-default",
), "suppress");

// Browser storage is an optional recovery aid, never an authority admission
// gate. A failing tombstone write still applies backend B and hides stale A in
// this live UI; one exact-generation duplicate has neither a second warning
// nor a second suppression side effect.
globalThis.window = {
  localStorage: {
    getItem: () => null,
    setItem: () => { throw new Error("storage quota denied"); },
    removeItem: () => {},
  },
};
assert.equal(tombstoneProjectRecoveryCheckpoint(77), false);
const unavailableDispositionState = {
  baseline: null,
  observedDispositionGeneration: 0,
  dispositionInitialized: false,
  dirty: true,
};
const unavailableCandidate = {
  ...eventOnlyUnsaved,
  authority_disposition_generation: 77,
};
const unavailableDecision = projectAuthorityRecoveryTombstonePreflight(
  unavailableDispositionState,
  unavailableCandidate,
  false,
);
assert.equal(unavailableDecision.applyAuthority, true, "storage failure must never keep UI at A after backend B");
assert.equal(unavailableDecision.suppressRecoveryPrompt, true);
assert.equal(unavailableDecision.warnRecoveryStorageUnavailable, true);
let inMemoryRecoveryPrompt = recoveryA;
if (unavailableDecision.suppressRecoveryPrompt) inMemoryRecoveryPrompt = null;
assert.equal(inMemoryRecoveryPrompt, null, "stale A recovery is suppressed for the live B session");
const duplicateUnavailable = projectAuthorityRecoveryTombstonePreflight(
  projectAuthorityDispositionTransition(unavailableDispositionState, unavailableCandidate),
  unavailableCandidate,
  false,
);
assert.equal(duplicateUnavailable.applyAuthority, true);
assert.equal(duplicateUnavailable.suppressRecoveryPrompt, false);
assert.equal(duplicateUnavailable.warnRecoveryStorageUnavailable, false, "exact duplicate must not spam storage warnings");

// R1 may be delayed while R2 or an identity replacement proceeds. Only the
// one guard that still owns write/read/authority/drafts may commit recovery;
// this is the production gate used immediately before localStorage writes.
const recoveryGuardA = {
  writeGeneration: 1,
  readGeneration: 7,
  authority: { project_epoch: 1, project_revision: 2, checkpoint_hash: "A" },
  draftSignature: "draft-A",
};
assert.equal(
  projectRecoveryCaptureIsCurrent(
    recoveryGuardA,
    { ...recoveryGuardA, writeGeneration: 2, draftSignature: "draft-B" },
    recoveryGuardA.authority,
  ),
  false,
  "R1 reverse completion cannot overwrite R2",
);
assert.equal(
  projectRecoveryCaptureIsCurrent(
    recoveryGuardA,
    {
      ...recoveryGuardA,
      readGeneration: 8,
      authority: { project_epoch: 2, project_revision: 0, checkpoint_hash: "B" },
    },
    recoveryGuardA.authority,
  ),
  false,
  "A drafts plus a B replacement bundle are rejected",
);
assert.equal(
  projectRecoveryCaptureIsCurrent(recoveryGuardA, recoveryGuardA, recoveryGuardA.authority),
  true,
  "a stable A capture remains eligible",
);
assert.notEqual(
  projectRecoveryAuthoritySignature("same-rendered-ui", "same-drafts", {
    project_epoch: 2,
    project_revision: 4,
    checkpoint_hash: "groups-A",
  }),
  projectRecoveryAuthoritySignature("same-rendered-ui", "same-drafts", {
    project_epoch: 2,
    project_revision: 5,
    checkpoint_hash: "groups-B",
  }),
  "fixture-group/profile-only authority mutation must schedule a fresh recovery/backup capture",
);
assert.equal(
  projectRecoveryAcknowledgementCanClear(
    "recovered-drafts-A",
    "user-edited-drafts-B",
    recoveryGuardA.authority,
    recoveryGuardA.authority,
    recoveryGuardA.authority,
    true,
  ),
  false,
  "a delayed recovery ACK must not clear a newer draft image",
);
assert.equal(
  projectRecoveryAcknowledgementCanClear(
    "recovered-drafts-A",
    "recovered-drafts-A",
    recoveryGuardA.authority,
    recoveryGuardA.authority,
    recoveryGuardA.authority,
    true,
  ),
  true,
  "an unchanged recovered draft image may acknowledge and clear recovery",
);

// A recovery acknowledgement resolves after an async backend round trip. The
// deterministic hazard is: B ACK is dispatched -> a later project C applies (it
// becomes the live authority and its own application invalidates B's intent) ->
// B's delayed ACK finally resolves. The production gate must report the delayed
// B ACK as non-current so applyProjectAuthorityRuntimeStatus is skipped entirely
// and C's MIDI/OSC/DMX/path/history/disposition UI is never rewound to stale B.
const recoveryAckTokenB = { project_epoch: 12, project_revision: 3, checkpoint_hash: "recovery-ack-B" };
const laterProjectTokenC = { project_epoch: 13, project_revision: 0, checkpoint_hash: "project-C" };
// While B is still the live authority and its intent is active, the ACK applies.
assert.equal(
  projectRecoveryAcknowledgementApplicationIsCurrent(true, recoveryAckTokenB, recoveryAckTokenB),
  true,
  "a recovery ACK that resolves while B is still the live authority applies its runtime status",
);
// Model C winning: C applied its own runtime status (the live authority is now
// C) and invalidated B's recovery intent. Drive the exact production gate over a
// mock live runtime authority to prove the stale B ACK applies no side effect.
let liveRuntimeAuthority = laterProjectTokenC;
const delayedBAckAppliesOverC = projectRecoveryAcknowledgementApplicationIsCurrent(
  false, // C's application invalidated B's recovery intent
  recoveryAckTokenB,
  liveRuntimeAuthority,
);
assert.equal(
  delayedBAckAppliesOverC,
  false,
  "B ACK -> C applies -> delayed B ACK must be reported non-current",
);
// This mirrors production: applyProjectAuthorityRuntimeStatus runs only inside
// the guarded branch, so a false gate leaves C's authority untouched.
if (delayedBAckAppliesOverC) liveRuntimeAuthority = recoveryAckTokenB;
assert.deepEqual(
  liveRuntimeAuthority,
  laterProjectTokenC,
  "the stale B acknowledgement bundle must not rewind C's runtime authority",
);
// The token check alone still rejects the delayed B ACK even if a defensive
// caller reports the intent as still active, so a C that changed only the
// authority token still wins.
assert.equal(
  projectRecoveryAcknowledgementApplicationIsCurrent(true, recoveryAckTokenB, laterProjectTokenC),
  false,
  "a changed authority token alone makes the delayed B ACK non-current",
);

// E1 uses the production frontend canonicalizer and recovery classifier. A
// retry may adopt only the exact pending receipt; terminal outcomes are never
// cast to an arbitrary mutation command result.
const e1Shape = projectTransactionShapeFingerprint(
  "update_cue_from_current",
  "Update Cue From Current",
  "cue:7",
);
assert.equal(
  e1Shape,
  "project-transaction-v1|command=update_cue_from_current|label=Update Cue From Current|coalesce=cue:7",
);
assert.equal(projectTransactionRecoveryCanAdopt("pending"), true);
assert.equal(projectTransactionRecoveryCanAdopt("committed"), false);
assert.equal(projectTransactionRecoveryIsTerminal("cancelled"), true);
assert.equal(projectTransactionRecoveryIsTerminal("acknowledged"), true);

console.log("project authority deterministic checks passed");
