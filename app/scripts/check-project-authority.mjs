import assert from "node:assert/strict";
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
  projectAuthorityDispositionClearsRecovery,
  projectAuthorityDispositionTransition,
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
  projectAuthorityTokenIsCurrent,
  rebaseDirtyProjectAuthorityMappings,
  runAfterProjectAuthorityFlush,
} from "../src/projectAuthority.ts";
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
