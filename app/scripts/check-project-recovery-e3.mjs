import assert from "node:assert/strict";
import { readFileSync } from "node:fs";

// First run the production authority event/reply/poll matrix. That checker
// imports App's actual projectAuthorityRuntime entry points; this E3 driver
// then adds the browser process-boundary and registered-command proof without
// reimplementing either state machine.
await import("./check-project-authority.mjs");

import {
  createProjectRecoveryCheckpoint,
  loadProjectRecoveryStorageState,
  saveProjectRecoveryCheckpoint,
  startProjectRecoveryPublication,
} from "../src/projectRecoveryStorage.ts";
import {
  projectRecoveryIntentStartupAction,
} from "../src/projectAuthority.ts";
import {
  applyProjectRecoveryStartupProduction,
  createProjectRecoveryIntentConsumerProduction,
} from "../src/projectAuthorityRuntime.ts";

const mainSource = readFileSync(new URL("../src-tauri/src/main.rs", import.meta.url), "utf8");
const appSource = readFileSync(new URL("../src/App.tsx", import.meta.url), "utf8");
const invokeManifest = JSON.parse(readFileSync(
  new URL("../src/tauri-invoke-manifest.json", import.meta.url),
  "utf8",
));
const typedInvokeSource = readFileSync(new URL("../src/tauriInvokeCommands.ts", import.meta.url), "utf8");
const handlerBlock = mainSource.slice(
  mainSource.indexOf("tauri::generate_handler!"),
  mainSource.indexOf("tauri::generate_handler!") + 20_000,
);
for (const command of [
  "get_project_recovery_authority_status",
  "get_project_authority_bundle",
  "poll_project_authority_bundle",
  "load_project_checkpoint",
  "acknowledge_project_recovery_applied",
]) {
  assert.match(handlerBlock, new RegExp(`\\b${command}\\b`), `${command} must remain registered`);
  assert.ok(invokeManifest.includes(command), `${command} must remain in the frontend invoke manifest`);
  assert.match(typedInvokeSource, new RegExp(`"${command}"`), `${command} must remain in the typed invoke tuple`);
}
for (const command of ["load_project_checkpoint", "acknowledge_project_recovery_applied"]) {
  assert.match(appSource, new RegExp(`"${command}"`), `${command} must be invoked by App production orchestration`);
}
assert.match(
  mainSource,
  /fn load_project_checkpoint[\s\S]*?load_project_checkpoint_core[\s\S]*?fn load_project_checkpoint_core[\s\S]*?prepare_project_recovery_load[\s\S]*?replace_prepared_project_snapshot_with_platform/,
  "registered recovery load and the driver must share the production lifecycle/publication service",
);
assert.match(
  mainSource,
  /fn acknowledge_project_recovery_applied[\s\S]*?acknowledge_project_recovery_applied_service[\s\S]*?fn acknowledge_project_recovery_applied_service[\s\S]*?acknowledge_project_recovery_applied_core/,
  "registered recovery ACK and the driver must share the admission/CAS service",
);

const storage = new Map();
globalThis.window = {
  localStorage: {
    getItem: (key) => storage.get(key) ?? null,
    setItem: (key, value) => storage.set(key, value),
    removeItem: (key) => storage.delete(key),
  },
};

const project = {
  version: 1,
  app: "Syndocal",
  snapshot: { fixtures: [], cues: [] },
};
const checkpointA = createProjectRecoveryCheckpoint(
  project,
  "C:/shows/A.sdc",
  "checkpoint-A",
);
assert.equal(saveProjectRecoveryCheckpoint(checkpointA, 4), true);

const order = [];
let activeIntent = null;
let rejectReply;
const publication = startProjectRecoveryPublication(
  checkpointA,
  4,
  1,
  (intent) => {
    order.push("owner");
    activeIntent = intent;
    assert.equal(loadProjectRecoveryStorageState(4).kind, "intent");
  },
  (intent) => {
    order.push("invoke");
    assert.equal(activeIntent, intent, "renderer owner must exist before invoke");
    assert.equal(loadProjectRecoveryStorageState(4).kind, "intent", "v3 intent must be durable before invoke");
    return new Promise((_resolve, reject) => { rejectReply = reject; });
  },
);
assert.ok(publication);
assert.deepEqual(order, ["owner", "invoke"]);
assert.equal(
  projectRecoveryIntentStartupAction(
    publication.intent,
    4,
    "runtime_sanitize",
    { kind: "project_publication" },
    "runtime-default",
  ),
  "offer_checkpoint",
  "crash after durable intent but before B publication re-offers A",
);

const publicationTransition = {
  kind: "recovery_publication",
  source_serial: 4,
  request_id: publication.intent.request_id,
  target_checkpoint_hash: "checkpoint-B",
};
const bundle = ({ serial, disposition, transition, hash, epoch }) => ({
  project_epoch: epoch,
  project_revision: 0,
  checkpoint_hash: hash,
  authority_disposition_generation: epoch,
  recovery_authority_serial: serial,
  authority_disposition: disposition,
  recovery_authority_last_transition: transition,
  snapshot: { timeline: { events: [] } },
});
const bundleB = bundle({
  serial: 5,
  disposition: "recovery_pending_ack",
  transition: publicationTransition,
  hash: "checkpoint-B",
  epoch: 2,
});

// Lose the command reply after B publication. The one v3 envelope remains
// restartable; event/poll ownership is not undone by the rejected transport.
rejectReply(new Error("injected reply loss after B publication"));
await assert.rejects(publication.reply, /injected reply loss/);
assert.equal(loadProjectRecoveryStorageState(5).kind, "intent");
assert.equal(
  projectRecoveryIntentStartupAction(
    publication.intent,
    5,
    "recovery_pending_ack",
    publicationTransition,
    "checkpoint-B",
  ),
  "resume_intent",
  "renderer restart inside B resumes the exact request instead of republishing B",
);
assert.equal(
  projectRecoveryIntentStartupAction(
    publication.intent,
    5,
    "runtime_sanitize",
    publicationTransition,
    "runtime-default",
  ),
  "offer_checkpoint",
  "native restart after B re-offers A when the runtime B image no longer exists",
);

// A fresh renderer coordinator owns the actual App startup and recovery
// consumer cores. Event, reply and poll deliveries coalesce onto one injected
// registered ACK after the coherent status+bundle handoff resumes the intent.
activeIntent = null;
let currentAuthority = { project_epoch: 2, project_revision: 0, checkpoint_hash: "checkpoint-B" };
let currentSerial = 5;
let drafts = "{}";
let offer = null;
let needsReplacement = false;
let captureSignatureResets = 0;
let appliedAckStatus = 0;
const messages = [];
const acknowledgements = new Map();
let acknowledgementCalls = 0;
const makeConsumer = () => createProjectRecoveryIntentConsumerProduction({
  getActiveIntent: () => activeIntent,
  setActiveIntent: (intent) => { activeIntent = intent; },
  currentAuthority: () => currentAuthority,
  currentRecoverySerial: () => currentSerial,
  currentDraftSignature: () => drafts,
  expectedDraftSignature: () => "{}",
  stageDrafts: () => {
    drafts = "{}";
    return drafts;
  },
  persistCheckpoint: (intent, serial) => saveProjectRecoveryCheckpoint(intent.checkpoint, serial),
  setRecoveryOffer: (intent) => { offer = intent?.checkpoint ?? null; },
  setCheckpointNeedsReplacement: (required) => { needsReplacement = required; },
  resetRecoveryCaptureSignature: () => { captureSignatureResets += 1; },
  tombstone: () => {},
  acknowledge: (request) => {
    acknowledgementCalls += 1;
    return new Promise((resolve, reject) => acknowledgements.set(request.recoveryRequestId, { resolve, reject }));
  },
  applyAcknowledgementRuntimeStatus: (acknowledged) => {
    appliedAckStatus += 1;
    currentAuthority = {
      project_epoch: acknowledged.project_epoch,
      project_revision: acknowledged.project_revision,
      checkpoint_hash: acknowledged.checkpoint_hash,
    };
    currentSerial = acknowledged.recovery_authority_serial;
  },
  message: (message) => messages.push(message),
  recoveredMessage: () => "recovered",
});

const restartedConsumer = makeConsumer();
let startupConsumption = Promise.resolve();
assert.equal(applyProjectRecoveryStartupProduction(
  bundleB,
  loadProjectRecoveryStorageState(5),
  {
    setRecoveryOffer: (checkpoint) => { offer = checkpoint; },
    setActiveIntent: (intent) => { activeIntent = intent; },
    applyReplacement: (candidate) => {
      currentAuthority = {
        project_epoch: candidate.project_epoch,
        project_revision: candidate.project_revision,
        checkpoint_hash: candidate.checkpoint_hash,
      };
      currentSerial = candidate.recovery_authority_serial;
      return "apply";
    },
    currentRuntimeState: () => { throw new Error("apply path must not use duplicate startup state"); },
    consume: (candidate) => { startupConsumption = restartedConsumer.consume(candidate); },
  },
), "resume");
const eventDelivery = restartedConsumer.consume(bundleB);
const replyDelivery = restartedConsumer.consume(bundleB);
const pollDelivery = restartedConsumer.consume(bundleB);
assert.equal(acknowledgementCalls, 1, "event/reply/poll B must issue one registered ACK");
const acknowledgementTransition = {
  kind: "recovery_acknowledged",
  recovery_publication_serial: 5,
  request_id: publication.intent.request_id,
  target_checkpoint_hash: "checkpoint-B",
};
const acknowledgedB = bundle({
  serial: 6,
  disposition: "unsaved_replacement",
  transition: acknowledgementTransition,
  hash: "checkpoint-B",
  epoch: 2,
});
acknowledgements.get(publication.intent.request_id).resolve(acknowledgedB);
await Promise.all([startupConsumption, eventDelivery, replyDelivery, pollDelivery]);
assert.equal(appliedAckStatus, 1);
assert.equal(activeIntent, null);
assert.equal(needsReplacement, true);
assert.equal(captureSignatureResets, 1);
assert.equal(offer, null);
assert.equal(
  loadProjectRecoveryStorageState(6).kind,
  "checkpoint",
  "ACK must retain the coherent checkpoint through browser restart",
);
assert.equal(
  projectRecoveryIntentStartupAction(
    publication.intent,
    6,
    "runtime_sanitize",
    acknowledgementTransition,
    "runtime-default",
  ),
  "offer_checkpoint",
  "crash after durable ACK but before renderer cleanup retains recovery",
);
assert.equal(
  loadProjectRecoveryStorageState(7).kind,
  "none",
  "a later coherent CleanSave serial retires the acknowledged checkpoint",
);

const competingC = {
  ...bundle({
    serial: 8,
    disposition: "unsaved_replacement",
    transition: { kind: "project_publication" },
    hash: "checkpoint-C",
    epoch: 4,
  }),
};

// Delayed ACK for D cannot rewind C, and its private single-flight cannot
// block a new E intent which is registered before that delayed ACK settles.
assert.equal(saveProjectRecoveryCheckpoint(checkpointA, 6), true);
let rejectDReply;
const publicationD = startProjectRecoveryPublication(
  checkpointA,
  6,
  2,
  (intent) => { activeIntent = intent; },
  () => new Promise((_resolve, reject) => { rejectDReply = reject; }),
);
assert.ok(publicationD);
currentAuthority = { project_epoch: 3, project_revision: 0, checkpoint_hash: "checkpoint-D" };
currentSerial = 7;
const bundleD = bundle({
  serial: 7,
  disposition: "recovery_pending_ack",
  transition: {
    kind: "recovery_publication",
    source_serial: 6,
    request_id: publicationD.intent.request_id,
    target_checkpoint_hash: "checkpoint-D",
  },
  hash: "checkpoint-D",
  epoch: 3,
});
const consumer = makeConsumer();
const delayedD = consumer.consume(bundleD);
assert.equal(acknowledgementCalls, 2);
currentAuthority = { project_epoch: 4, project_revision: 0, checkpoint_hash: "checkpoint-C" };
currentSerial = 8;
await consumer.consume(competingC);
assert.equal(activeIntent, null, "C invalidates D before delayed ACK");

assert.equal(saveProjectRecoveryCheckpoint(checkpointA, 8), true);
const publicationE = startProjectRecoveryPublication(
  checkpointA,
  8,
  3,
  (intent) => { activeIntent = intent; },
  () => Promise.reject(new Error("reply intentionally lost")),
);
assert.ok(publicationE);
void publicationE.reply.catch(() => {});
currentAuthority = { project_epoch: 5, project_revision: 0, checkpoint_hash: "checkpoint-E" };
currentSerial = 9;
const bundleE = bundle({
  serial: 9,
  disposition: "recovery_pending_ack",
  transition: {
    kind: "recovery_publication",
    source_serial: 8,
    request_id: publicationE.intent.request_id,
    target_checkpoint_hash: "checkpoint-E",
  },
  hash: "checkpoint-E",
  epoch: 5,
});
const applyingE = consumer.consume(bundleE);
assert.equal(acknowledgementCalls, 3, "new E is not blocked by delayed D single-flight");
acknowledgements.get(publicationD.intent.request_id).resolve(bundle({
  serial: 8,
  disposition: "unsaved_replacement",
  transition: {
    kind: "recovery_acknowledged",
    recovery_publication_serial: 7,
    request_id: publicationD.intent.request_id,
    target_checkpoint_hash: "checkpoint-D",
  },
  hash: "checkpoint-D",
  epoch: 3,
}));
await delayedD;
assert.equal(currentAuthority.checkpoint_hash, "checkpoint-E", "late D ACK cannot roll back E");
acknowledgements.get(publicationE.intent.request_id).resolve(bundle({
  serial: 10,
  disposition: "unsaved_replacement",
  transition: {
    kind: "recovery_acknowledged",
    recovery_publication_serial: 9,
    request_id: publicationE.intent.request_id,
    target_checkpoint_hash: "checkpoint-E",
  },
  hash: "checkpoint-E",
  epoch: 5,
}));
await applyingE;
assert.equal(currentAuthority.checkpoint_hash, "checkpoint-E");
assert.equal(loadProjectRecoveryStorageState(10).kind, "checkpoint");
rejectDReply(new Error("lost D reply"));
await assert.rejects(publicationD.reply, /lost D reply/);

console.log("E3 project recovery production driver passed");
