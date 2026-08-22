import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

async function importTsModule(path) {
  const source = await readFile(new URL(path, import.meta.url), "utf8");
  const transpiled = ts.transpileModule(source, {
    compilerOptions: {
      module: ts.ModuleKind.ESNext,
      target: ts.ScriptTarget.ES2022,
      importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
    },
    fileName: path,
  });
  return import(`data:text/javascript;base64,${Buffer.from(transpiled.outputText).toString("base64")}`);
}

const storage = new Map();
let failSetCount = 0;
let failRemoveCount = 0;
const priorWindow = globalThis.window;
const priorCrypto = globalThis.crypto;
globalThis.window = {
  localStorage: {
    getItem: (key) => storage.get(key) ?? null,
    setItem: (key, value) => {
      if (failSetCount > 0) {
        failSetCount -= 1;
        throw new Error("injected localStorage.setItem failure");
      }
      storage.set(key, value);
    },
    removeItem: (key) => {
      if (failRemoveCount > 0) {
        failRemoveCount -= 1;
        throw new Error("injected localStorage.removeItem failure");
      }
      storage.delete(key);
    },
  },
};
Object.defineProperty(globalThis, "crypto", {
  configurable: true,
  value: { randomUUID: () => "renderer_origin-000000000000000000000000000000000000000000000000000000000000000001" },
});

const backupSuccessStatus = (request, overrides = {}) => ({
  request: structuredClone(request),
  shapeHash: "b".repeat(64),
  surface: "backup",
  state: "succeeded",
  targetPath: "C:/shows/backups/1.json",
  artifactDigest: "c".repeat(64),
  backup: {
    id: 1,
    created_at_unix_ms: 1_800_000_000_000,
    source_path: request.sourcePath,
    reason: request.reason,
    bytes: 4096,
  },
  recoveryAuthoritySerial: 4,
  authority: null,
  error: null,
  ...overrides,
});

const pendingStatus = (request, state, overrides = {}) => ({
  request: structuredClone(request),
  shapeHash: "d".repeat(64),
  surface: request.surface,
  state,
  targetPath: state === "selected" || state === "prepared"
    ? `C:/shows/selected${request.surface === "backup" ? ".json" : request.surface === "user_template" ? ".sdctemplate" : ".sdc"}`
    : null,
  ...(state === "prepared" ? { artifactDigest: "e".repeat(64) } : {}),
  backup: null,
  recoveryAuthoritySerial: 4,
  authority: null,
  error: null,
  ...overrides,
});

const canonicalAuthority = (request, recoveryAuthoritySerial = 4) => ({
  project_epoch: request.expectedProjectEpoch,
  project_revision: request.expectedProjectRevision,
  checkpoint_hash: request.expectedCheckpointHash,
  publication_generation: 9,
  publication_kind: "mutation",
  mapping_replacement_generation: 3,
  authority_disposition_generation: 5,
  authority_disposition: "clean_at_path",
  recovery_authority_serial: recoveryAuthoritySerial,
  recovery_authority_last_transition: { kind: "clean_save" },
  path_generation: 2,
  history_generation: 6,
  current_project_path: request.sourcePath,
  snapshot: {
    fixtures: [], cues: [], cue_lists: [], palettes: [], playback_executors: [],
    playback_master: 1, programmer: {}, timeline: {}, video: {}, effects: [], node_graphs: [],
    output: {}, dmx_outputs: [], lighting_master: 1, submasters: [], blackout: false,
    clock: {}, stage_map: {}, stage_map_presets: [], stage_objects: [], dmx_preview: [],
    dmx_previews: [], telemetry: {},
  },
  profiles: [],
  fixture_groups: [],
  operator_policy: null,
  midi_mappings: [],
  osc_mappings: [],
  dmx_mappings: [],
  dj_track_triggers: [],
  history: {
    can_undo: false,
    can_redo: false,
    undo_depth: 0,
    redo_depth: 0,
    undo_label: null,
    redo_label: null,
    project_epoch: request.expectedProjectEpoch,
    project_revision: request.expectedProjectRevision,
    checkpoint_hash: request.expectedCheckpointHash,
    history_generation: 6,
    undo_entry_id: null,
    undo_checkpoint_hash: null,
    redo_entry_id: null,
    redo_checkpoint_hash: null,
  },
  input_runtime: {
    project_input_runtime_generation: 2,
    mapping_input_runtime_generation: 3,
    midi_clock_active: false,
    midi_control_active: false,
    midi_feedback_output_active: false,
    midi_feedback_runtime_active: false,
    osc_active: false,
    dmx_active: false,
  },
});

try {
  const publication = await importTsModule("../src/projectPublicationStorage.ts");
  const operator = await importTsModule("../src/operatorPolicy.ts");
  const seed = {
    surface: "backup",
    ownerId: "renderer:publication-e4-checker",
    expectedProjectEpoch: 7,
    expectedProjectRevision: 12,
    expectedCheckpointHash: "a".repeat(64),
    mappingAuthorityHash: "a".repeat(64),
    sourcePath: "C:/shows/main.sdc",
    reason: "autosave",
    targetPolicy: "managed_unique",
  };

  const createControllerHarness = ({
    invokeStatus,
    adoptOwner = async () => assert.fail("unexpected project publication owner adoption"),
    currentOwnerId = seed.ownerId,
    ensureMutationAllowed = () => {},
    confirmAbandon = () => false,
    applyTerminal = async () => {},
    acknowledgeTerminal = async (request) => {
      if (!publication.acknowledgeProjectPublicationIntent(request)) {
        throw new Error("injected local ACK clear failure");
      }
    },
  }) => {
    const calls = [];
    const controller = publication.createProjectPublicationControllerV1({
      invokeStatus: async (command, request) => {
        calls.push(["invoke", command, structuredClone(request)]);
        return invokeStatus(command, request, controller);
      },
      adoptOwner: async (request, newOwnerId) => {
        calls.push(["adopt-owner", structuredClone(request), newOwnerId]);
        return adoptOwner(request, newOwnerId, controller);
      },
      currentOwnerId,
      publishQueuedAcknowledgement: async () => {
        calls.push(["publish-queued-ack"]);
        const queued = publication.loadProjectPublicationAcknowledgement();
        if (queued) await acknowledgeTerminal(queued);
      },
      settleTerminal: (status) => publication.settleProjectPublicationStatusV1(
        status,
        async (terminal) => {
          calls.push(["apply-terminal", terminal.state]);
          await applyTerminal(terminal);
        },
        async (request) => {
          calls.push(["native-ack", request.requestId]);
          await acknowledgeTerminal(request);
        },
      ),
      applyIndeterminate: async (status) => {
        calls.push(["apply-indeterminate", status.state]);
        await applyTerminal(status);
      },
      captureSeed: async () => {
        calls.push(["capture-seed"]);
        return structuredClone(seed);
      },
      ensureMutationAllowed: (command) => {
        calls.push(["admission", command]);
        ensureMutationAllowed(command);
      },
      confirmAbandon: async (intent, status, requestedSurface) => {
        calls.push(["confirm-abandon", status.state, requestedSurface, intent.request.requestId]);
        return confirmAbandon(intent, status, requestedSurface);
      },
      onPending: (status) => calls.push(["pending", status.state]),
      onMissing: (intent) => calls.push(["missing", intent.request.requestId]),
      onAbandoned: (intent, surface) => calls.push(["abandoned", intent.request.requestId, surface]),
    });
    return { controller, calls };
  };

  const first = publication.beginProjectPublicationIntent(seed);
  assert.ok(first, "the first request must be durable before IPC");
  assert.deepEqual(first.request, {
    schemaVersion: 1,
    originId: "renderer_origin-000000000000000000000000000000000000000000000000000000000000000001",
    requestId: 1,
    ...seed,
  });
  const rawFirst = JSON.parse(storage.get("syndocal.projectPublication.v1"));
  assert.deepEqual(rawFirst.intent.request, first.request, "storage must preserve the exact wire preimage");
  assert.equal("schema_version" in rawFirst.intent.request, false, "snake_case request keys are forbidden on the camelCase IPC wire");
  assert.equal(
    publication.projectPublicationRequestFromUnknown({
      schema_version: 1,
      origin_id: first.request.originId,
      request_id: first.request.requestId,
      owner_id: first.request.ownerId,
      ...seed,
    }),
    null,
    "wrong-case request payloads must fail closed before they can reach IPC",
  );

  const exactRetry = publication.beginProjectPublicationIntent({ ...seed });
  assert.deepEqual(exactRetry, first, "only a canonical exact seed may reuse the unfinished identity");
  assert.equal(storage.get("syndocal.projectPublication.v1"), JSON.stringify(rawFirst), "exact retry must not mutate the local journal");

  const beforeChangedAction = storage.get("syndocal.projectPublication.v1");
  assert.equal(
    publication.beginProjectPublicationIntent({ ...seed, surface: "save_as", reason: null, targetPolicy: "dialog" }),
    null,
    "a different surface must not inherit an unfinished Backup identity",
  );
  assert.equal(
    publication.beginProjectPublicationIntent({ ...seed, reason: "update-preflight" }),
    null,
    "a changed reason must not inherit an unfinished identity",
  );
  assert.equal(
    publication.beginProjectPublicationIntent({ ...seed, ownerId: "renderer:other-owner" }),
    null,
    "a different renderer owner must not inherit an unfinished mutation identity",
  );
  assert.equal(storage.get("syndocal.projectPublication.v1"), beforeChangedAction, "blocked UI actions must leave the local journal unchanged");
  assert.equal(
    publication.beginProjectPublicationIntent({ ...seed, reason: " autosave " }),
    null,
    "the storage boundary rejects noncanonical reason text before IPC",
  );
  assert.equal(publication.normalizeProjectPublicationReason(" autosave "), "autosave");

  // Simulate a terminal response whose request was echoed from the durable
  // backend receipt, then lose the ACK reply. The local ACK queue must survive
  // a renderer restart and only clear after an explicit exact ACK success.
  // Rust is free to serialize object keys in a different order. Exact receipt
  // matching must compare canonical fields, not fragile JSON insertion order.
  const terminal = backupSuccessStatus({
    targetPolicy: first.request.targetPolicy,
    reason: first.request.reason,
    sourcePath: first.request.sourcePath,
    mappingAuthorityHash: first.request.mappingAuthorityHash,
    expectedCheckpointHash: first.request.expectedCheckpointHash,
    expectedProjectRevision: first.request.expectedProjectRevision,
    expectedProjectEpoch: first.request.expectedProjectEpoch,
    surface: first.request.surface,
    requestId: first.request.requestId,
    originId: first.request.originId,
    ownerId: first.request.ownerId,
    schemaVersion: first.request.schemaVersion,
  });
  assert.deepEqual(
    publication.projectPublicationStatusForRequestFromUnknown(terminal, first.request),
    terminal,
    "the production parser accepts a canonical exact receipt regardless of request key order",
  );
  const saveRequest = {
    ...first.request,
    surface: "save",
    reason: null,
    targetPolicy: "current_or_dialog",
  };
  const saveAuthority = canonicalAuthority(saveRequest);
  const saveTerminal = {
    request: saveRequest,
    shapeHash: "f".repeat(64),
    surface: "save",
    state: "succeeded",
    targetPath: "C:/shows/main.sdc",
    artifactDigest: "1".repeat(64),
    backup: null,
    recoveryAuthoritySerial: 4,
    authority: saveAuthority,
    error: null,
  };
  assert.deepEqual(
    publication.projectPublicationStatusForRequestFromUnknown(saveTerminal, saveRequest),
    saveTerminal,
    "a complete JS-safe authority bundle is accepted only when it matches the terminal E/R/H/serial",
  );
  assert.equal(
    publication.projectPublicationStatusForRequestFromUnknown({
      ...saveTerminal,
      authority: { ...saveAuthority, publication_generation: Number.MAX_SAFE_INTEGER + 1 },
    }, saveRequest),
    null,
    "unsafe nested authority generations fail before authority application",
  );
  assert.deepEqual(terminal.request, publication.loadProjectPublicationIntent().request);
  const applyOrder = [];
  await publication.settleProjectPublicationStatusV1(
    terminal,
    async () => applyOrder.push("apply"),
    async (request) => {
      applyOrder.push("native-ack");
      assert.deepEqual(publication.loadProjectPublicationAcknowledgement(), request, "native ACK starts only after the exact local queue is durable");
      // Inject the lost ACK reply: the exact queue must survive this throw.
      throw new Error("injected lost ACK reply");
    },
  ).catch((error) => assert.match(String(error), /lost ACK reply/));
  assert.deepEqual(applyOrder, ["apply", "native-ack"]);
  assert.equal(publication.loadProjectPublicationIntent(), null, "local application clears active intent before ACK");
  assert.deepEqual(publication.loadProjectPublicationAcknowledgement(), terminal.request, "lost ACK reply retains the exact durable key");
  assert.equal(publication.beginProjectPublicationIntent(seed), null, "a queued ACK blocks every new action");
  failSetCount = 1;
  assert.equal(
    publication.acknowledgeProjectPublicationIntent(terminal.request),
    false,
    "a local clear failure after native ACK must report failure",
  );
  assert.deepEqual(
    publication.loadProjectPublicationAcknowledgement(),
    terminal.request,
    "a local clear failure preserves the exact idempotent ACK retry",
  );
  assert.equal(publication.acknowledgeProjectPublicationIntent(terminal.request), true);
  assert.equal(publication.loadProjectPublicationAcknowledgement(), null);
  const second = publication.beginProjectPublicationIntent(seed);
  assert.equal(second.request.requestId, 2, "request IDs stay monotonic across terminal cleanup");

  const secondTerminal = backupSuccessStatus(second.request, {
    backup: {
      id: 2,
      created_at_unix_ms: 1_800_000_000_001,
      source_path: second.request.sourcePath,
      reason: second.request.reason,
      bytes: 8192,
    },
  });
  const { ownerId: _omittedOwnerId, ...ownerlessRequest } = secondTerminal.request;
  const beforeMalformedReplies = storage.get("syndocal.projectPublication.v1");
  for (const [label, malformed] of [
    ["missing mutation owner", { ...secondTerminal, request: ownerlessRequest }],
    ["future request schema", { ...secondTerminal, request: { ...secondTerminal.request, schemaVersion: 2 } }],
    ["unsafe status generation", { ...secondTerminal, recoveryAuthoritySerial: Number.MAX_SAFE_INTEGER + 1 }],
    ["invalid shape hash", { ...secondTerminal, shapeHash: "not-a-sha" }],
    ["invalid artifact digest", { ...secondTerminal, artifactDigest: "not-a-sha" }],
    ["malformed nested authority", { ...secondTerminal, authority: {} }],
    ["unknown additive field", { ...secondTerminal, futureField: true }],
    ["different exact request", { ...secondTerminal, request: { ...secondTerminal.request, requestId: 99 } }],
    ["different mutation owner", { ...secondTerminal, request: { ...secondTerminal.request, ownerId: "renderer:other-owner" } }],
    ["noncanonical mutation owner", { ...secondTerminal, request: { ...secondTerminal.request, ownerId: " renderer:publication-e4-checker " } }],
  ]) {
    assert.equal(
      publication.projectPublicationStatusForRequestFromUnknown(malformed, second.request),
      null,
      `${label} must fail before apply/ACK`,
    );
    assert.equal(storage.get("syndocal.projectPublication.v1"), beforeMalformedReplies, `${label} must not mutate durable browser state`);
  }

  // Native publication has already succeeded, but the browser queue write
  // fails. The production settlement helper must not invoke ACK and must keep
  // the active request queryable for the next recovery attempt.
  failSetCount = 1;
  let nativeAckCount = 0;
  await publication.settleProjectPublicationStatusV1(
    secondTerminal,
    async () => {},
    async () => { nativeAckCount += 1; },
  ).then(
    () => assert.fail("queue failure must reject settlement"),
    (error) => assert.match(String(error), /not available for durable acknowledgement/),
  );
  assert.equal(nativeAckCount, 0, "native ACK must not run without a durable local ACK queue");
  assert.deepEqual(publication.loadProjectPublicationIntent(), second, "setItem failure after native success retains the active intent");

  // The updater's backup-list refresh is ancillary. A refresh rejection is
  // caught by the production apply callback, after which exact ACK settlement
  // still completes and the published backup remains the durable outcome.
  const updateOrder = [];
  await publication.settleProjectPublicationStatusV1(
    secondTerminal,
    async () => {
      updateOrder.push("apply-backup-success");
      try {
        await Promise.reject(new Error("injected refreshProjectBackups rejection"));
      } catch {
        updateOrder.push("refresh-nonfatal");
      }
    },
    async (request) => {
      updateOrder.push("native-ack");
      assert.equal(publication.acknowledgeProjectPublicationIntent(request), true);
    },
  );
  assert.deepEqual(updateOrder, ["apply-backup-success", "refresh-nonfatal", "native-ack"]);
  assert.equal(publication.loadProjectPublicationIntent(), null);
  assert.equal(publication.loadProjectPublicationAcknowledgement(), null);

  // `removeItem` is test-only because the production envelope preserves its
  // origin/high-water identity. Even an injected removal failure must leave
  // the durable origin untouched rather than silently minting a new one.
  const beforeRemoveFailure = storage.get("syndocal.projectPublication.v1");
  failRemoveCount = 1;
  publication.clearProjectPublicationStorageForTests();
  assert.equal(storage.get("syndocal.projectPublication.v1"), beforeRemoveFailure);

  // Drive the same production controller App uses. A Selecting retry queries
  // the exact request and offers deliberate abandon, but never re-invokes the
  // dialog command or allocates a second identity when the operator declines.
  publication.clearProjectPublicationStorageForTests();
  const selectingSeed = {
    ...seed,
    surface: "save_as",
    reason: null,
    targetPolicy: "dialog",
  };
  const selectingHarness = createControllerHarness({
    invokeStatus: async (command, request) => {
      if (command === "save_project_as_v1" || command === "get_project_publication_receipt_v1") {
        return pendingStatus(request, "selecting");
      }
      assert.fail(`unexpected Selecting command ${command}`);
    },
  });
  const firstSelecting = await selectingHarness.controller.start("save_as");
  const selectingIntent = publication.loadProjectPublicationIntent();
  assert.equal(firstSelecting.state, "selecting");
  assert.ok(selectingIntent);
  const selectingRetry = await selectingHarness.controller.start("save_as");
  assert.equal(selectingRetry.state, "selecting");
  assert.equal(
    selectingHarness.calls.filter((call) => call[0] === "invoke" && call[1] === "save_project_as_v1").length,
    1,
    "Selecting retry must not reopen the native dialog",
  );
  assert.equal(
    selectingHarness.calls.filter((call) => call[0] === "confirm-abandon").length,
    1,
    "Selecting remains reachable through an explicit exact abandon decision",
  );
  assert.deepEqual(publication.loadProjectPublicationIntent(), selectingIntent, "declined abandon preserves the exact durable identity");

  // Event-before-reply: a Selected event wins over a stale Selecting command
  // reply through the controller's real parser/rank path.
  publication.clearProjectPublicationStorageForTests();
  const eventFirstHarness = createControllerHarness({
    invokeStatus: async (command, request, controller) => {
      assert.equal(command, "save_project_as_v1");
      controller.observeStatus(pendingStatus(request, "selected"), request);
      return pendingStatus(request, "selecting");
    },
  });
  const eventFirst = await eventFirstHarness.controller.start("save_as");
  assert.equal(eventFirst.state, "selected", "event-before-reply must not regress to Selecting");
  assert.deepEqual(
    eventFirstHarness.calls.filter((call) => call[0] === "pending").map((call) => call[1]),
    ["selected"],
  );

  // Reply-before-event: once Selected was observed, a late stale Selecting
  // event returns the monotonic Selected observation and changes no journal.
  publication.clearProjectPublicationStorageForTests();
  const replyFirstHarness = createControllerHarness({
    invokeStatus: async (command, request) => {
      assert.equal(command, "save_project_as_v1");
      return pendingStatus(request, "selected");
    },
  });
  const replyFirst = await replyFirstHarness.controller.start("save_as");
  const beforeLateEvent = storage.get("syndocal.projectPublication.v1");
  const lateEvent = replyFirstHarness.controller.observeStatus(
    pendingStatus(replyFirst.request, "selecting"),
    replyFirst.request,
  );
  assert.equal(lateEvent.state, "selected", "reply-before-event must reject a stale phase regression");
  assert.equal(storage.get("syndocal.projectPublication.v1"), beforeLateEvent, "late events cannot mutate durable browser state");

  // Malformed/future native replies are rejected by the production controller
  // before apply, ACK queueing, or any pending UI callback.
  publication.clearProjectPublicationStorageForTests();
  const malformedHarness = createControllerHarness({
    invokeStatus: async (_command, request) => pendingStatus(
      { ...request, schemaVersion: 2 },
      "selected",
    ),
  });
  await assert.rejects(
    malformedHarness.controller.start("save_as"),
    /malformed or unsupported project publication receipt/,
  );
  assert.equal(malformedHarness.calls.some((call) => call[0] === "pending" || call[0] === "native-ack"), false);
  assert.equal(publication.loadProjectPublicationIntent().request.schemaVersion, 1, "malformed reply leaves the canonical intent recoverable");

  // Native success followed by local queue failure is exercised through the
  // production controller: no ACK may escape and the exact intent survives.
  publication.clearProjectPublicationStorageForTests();
  let controllerNativeAckCount = 0;
  const storageFailureHarness = createControllerHarness({
    invokeStatus: async (command, request) => {
      assert.equal(command, "save_project_backup_v1");
      failSetCount = 1;
      return backupSuccessStatus(request);
    },
    acknowledgeTerminal: async () => { controllerNativeAckCount += 1; },
  });
  await assert.rejects(
    storageFailureHarness.controller.start("backup", "autosave"),
    /not available for durable acknowledgement/,
  );
  assert.equal(controllerNativeAckCount, 0, "storage failure after native Success must prevent ACK");
  assert.equal(publication.loadProjectPublicationIntent().request.surface, "backup");

  // Update preflight uses deferred controller Success, then an ancillary
  // refresh rejection and a lost ACK reply. The durable ACK queue survives
  // and the same exact request is safely retryable.
  publication.clearProjectPublicationStorageForTests();
  let updateAckAttempts = 0;
  const updateHarness = createControllerHarness({
    invokeStatus: async (command, request) => {
      assert.equal(command, "save_project_backup_v1");
      return backupSuccessStatus(request);
    },
    applyTerminal: async () => {
      try {
        await Promise.reject(new Error("injected refreshProjectBackups rejection"));
      } catch {
        // App treats refresh as ancillary; durable Success still settles.
      }
    },
    acknowledgeTerminal: async () => {
      updateAckAttempts += 1;
      throw new Error("injected lost ACK reply");
    },
  });
  const updateTerminal = await updateHarness.controller.start("backup", "before update 1.2.3", true);
  assert.equal(updateTerminal.state, "succeeded");
  assert.equal(publication.loadProjectPublicationAcknowledgement(), null, "deferred Success remains unacked for updater admission");
  await publication.settleProjectPublicationStatusV1(
    updateTerminal,
    async () => {
      try { await Promise.reject(new Error("refresh rejection")); } catch { /* nonfatal */ }
    },
    async () => {
      updateAckAttempts += 1;
      throw new Error("injected lost ACK reply");
    },
  ).catch((error) => assert.match(String(error), /lost ACK reply/));
  assert.equal(updateAckAttempts, 1);
  assert.deepEqual(publication.loadProjectPublicationAcknowledgement(), updateTerminal.request, "lost update ACK reply retains the exact durable retry key");
  assert.equal(publication.acknowledgeProjectPublicationIntent(updateTerminal.request), true);

  const retiredOwnerId = "renderer:retired-publication-owner";
  const adoptedOwnerId = "renderer:adopted-publication-owner";
  const adoptionCases = [
    { surface: "save", state: "reserved", reason: null, targetPolicy: "current_or_dialog", resumes: true },
    { surface: "save_as", state: "selecting", reason: null, targetPolicy: "dialog", resumes: false },
    { surface: "user_template", state: "selected", reason: null, targetPolicy: "dialog", resumes: true },
    { surface: "backup", state: "prepared", reason: "autosave", targetPolicy: "managed_unique", resumes: false },
  ];
  for (const adoptionCase of adoptionCases) {
    publication.clearProjectPublicationStorageForTests();
    const oldIntent = publication.beginProjectPublicationIntent({
      ...seed,
      surface: adoptionCase.surface,
      ownerId: retiredOwnerId,
      reason: adoptionCase.reason,
      targetPolicy: adoptionCase.targetPolicy,
    });
    assert.ok(oldIntent);
    let mutationResumeCount = 0;
    const oldStatus = pendingStatus(oldIntent.request, adoptionCase.state, { shapeHash: "2".repeat(64) });
    const adoptionHarness = createControllerHarness({
      currentOwnerId: adoptedOwnerId,
      invokeStatus: async (command, request) => {
        if (command === "get_project_publication_receipt_v1") {
          return request.ownerId === adoptedOwnerId
            ? pendingStatus(request, adoptionCase.state, { shapeHash: "3".repeat(64) })
            : oldStatus;
        }
        mutationResumeCount += 1;
        assert.equal(
          publication.loadProjectPublicationIntent().request.ownerId,
          adoptedOwnerId,
          `${adoptionCase.surface}/${adoptionCase.state} must durably rewrite owner before resume`,
        );
        return pendingStatus(request, adoptionCase.state, { shapeHash: "4".repeat(64) });
      },
      adoptOwner: async (request, newOwnerId) => {
        assert.deepEqual(request, oldIntent.request, "adoption sends the exact stored old request");
        assert.equal(newOwnerId, adoptedOwnerId);
        return pendingStatus(
          { ...request, ownerId: newOwnerId },
          adoptionCase.state,
          { shapeHash: "3".repeat(64) },
        );
      },
    });
    await adoptionHarness.controller.recover();
    assert.equal(
      publication.loadProjectPublicationIntent().request.ownerId,
      adoptedOwnerId,
      `${adoptionCase.surface}/${adoptionCase.state} startup recovery must durably adopt the new owner`,
    );
    const adoptedStatus = await adoptionHarness.controller.start(adoptionCase.surface, adoptionCase.reason);
    assert.equal(adoptedStatus.request.ownerId, adoptedOwnerId);
    assert.equal(publication.loadProjectPublicationIntent().request.ownerId, adoptedOwnerId);
    assert.equal(mutationResumeCount, adoptionCase.resumes ? 1 : 0);
    assert.equal(adoptionHarness.calls.filter((call) => call[0] === "adopt-owner").length, 1);
    const afterAdoption = storage.get("syndocal.projectPublication.v1");
    assert.equal(
      publication.adoptProjectPublicationIntentOwner(oldIntent.request, adoptedStatus.request),
      false,
      "a stale old owner cannot rewrite an already-adopted intent",
    );
    assert.equal(storage.get("syndocal.projectPublication.v1"), afterAdoption, "stale old-owner adoption is delta0");
  }

  // A lost adoption reply leaves the old browser intent. On the next query,
  // the backend may return the exact owner-only adopted identity; the same
  // controller durably converges it before any mutation resumes.
  publication.clearProjectPublicationStorageForTests();
  const lostReplyIntent = publication.beginProjectPublicationIntent({
    ...seed,
    surface: "save",
    ownerId: retiredOwnerId,
    reason: null,
    targetPolicy: "current_or_dialog",
  });
  let backendAdopted = false;
  let replyLossResumeCount = 0;
  const replyLossHarness = createControllerHarness({
    currentOwnerId: adoptedOwnerId,
    invokeStatus: async (command, request) => {
      if (command === "get_project_publication_receipt_v1") {
        return pendingStatus(
          backendAdopted ? { ...request, ownerId: adoptedOwnerId } : request,
          "reserved",
          { shapeHash: (backendAdopted ? "6" : "5").repeat(64) },
        );
      }
      replyLossResumeCount += 1;
      assert.equal(publication.loadProjectPublicationIntent().request.ownerId, adoptedOwnerId);
      return pendingStatus(request, "reserved", { shapeHash: "7".repeat(64) });
    },
    adoptOwner: async () => {
      backendAdopted = true;
      throw new Error("injected lost adoption reply");
    },
  });
  await assert.rejects(replyLossHarness.controller.start("save"), /lost adoption reply/);
  assert.deepEqual(publication.loadProjectPublicationIntent(), lostReplyIntent, "lost adopt reply preserves the old exact local intent");
  const convergedStatus = await replyLossHarness.controller.start("save");
  assert.equal(convergedStatus.request.ownerId, adoptedOwnerId);
  assert.equal(replyLossResumeCount, 1, "query convergence resumes only after durable owner rewrite");

  // Wrong/stale adoption replies and operator denial are fail-closed and make
  // no local journal delta. Shape must change because ownerId is in its hash.
  for (const [label, replyFor] of [
    ["wrong origin", (request) => pendingStatus({ ...request, originId: "other-origin" }, "selected", { shapeHash: "9".repeat(64) })],
    ["wrong request", (request) => pendingStatus({ ...request, requestId: request.requestId + 1 }, "selected", { shapeHash: "9".repeat(64) })],
    ["wrong new owner", (request) => pendingStatus({ ...request, ownerId: "renderer:not-current" }, "selected", { shapeHash: "9".repeat(64) })],
    ["stale shape", (request, oldStatus) => pendingStatus({ ...request, ownerId: adoptedOwnerId }, "selected", { shapeHash: oldStatus.shapeHash })],
  ]) {
    publication.clearProjectPublicationStorageForTests();
    const oldIntent = publication.beginProjectPublicationIntent({
      ...seed,
      surface: "user_template",
      ownerId: retiredOwnerId,
      reason: null,
      targetPolicy: "dialog",
    });
    const oldStatus = pendingStatus(oldIntent.request, "selected", { shapeHash: "8".repeat(64) });
    const beforeRejectedAdoption = storage.get("syndocal.projectPublication.v1");
    let resumed = false;
    const rejectedHarness = createControllerHarness({
      currentOwnerId: adoptedOwnerId,
      invokeStatus: async (command) => {
        if (command === "get_project_publication_receipt_v1") return oldStatus;
        resumed = true;
        return oldStatus;
      },
      adoptOwner: async (request) => replyFor(request, oldStatus),
    });
    await assert.rejects(rejectedHarness.controller.start("user_template"));
    assert.equal(storage.get("syndocal.projectPublication.v1"), beforeRejectedAdoption, `${label} adoption reply is delta0`);
    assert.equal(resumed, false, `${label} adoption reply cannot resume mutation`);
  }

  publication.clearProjectPublicationStorageForTests();
  const deniedIntent = publication.beginProjectPublicationIntent({
    ...seed,
    ownerId: retiredOwnerId,
  });
  const beforeDeniedAdoption = storage.get("syndocal.projectPublication.v1");
  let deniedAdoptInvokes = 0;
  const deniedHarness = createControllerHarness({
    currentOwnerId: adoptedOwnerId,
    invokeStatus: async () => pendingStatus(deniedIntent.request, "reserved", { shapeHash: "a".repeat(64) }),
    adoptOwner: async () => { deniedAdoptInvokes += 1; },
    ensureMutationAllowed: (command) => {
      if (command === "adopt_project_publication_owner_v1") throw new Error("operator lock denied adoption");
    },
  });
  await assert.rejects(deniedHarness.controller.start("backup", "autosave"), /operator lock denied adoption/);
  assert.equal(deniedAdoptInvokes, 0);
  assert.equal(storage.get("syndocal.projectPublication.v1"), beforeDeniedAdoption, "operator-denied adoption is delta0");

  publication.clearProjectPublicationStorageForTests();
  const rewriteFailureIntent = publication.beginProjectPublicationIntent({
    ...seed,
    surface: "save",
    ownerId: retiredOwnerId,
    reason: null,
    targetPolicy: "current_or_dialog",
  });
  let rewriteFailureResumes = 0;
  const rewriteFailureHarness = createControllerHarness({
    currentOwnerId: adoptedOwnerId,
    invokeStatus: async (command) => {
      if (command === "get_project_publication_receipt_v1") {
        return pendingStatus(rewriteFailureIntent.request, "reserved", { shapeHash: "b".repeat(64) });
      }
      rewriteFailureResumes += 1;
      return pendingStatus(rewriteFailureIntent.request, "reserved");
    },
    adoptOwner: async (request) => {
      failSetCount = 1;
      return pendingStatus({ ...request, ownerId: adoptedOwnerId }, "reserved", { shapeHash: "c".repeat(64) });
    },
  });
  await assert.rejects(rewriteFailureHarness.controller.start("save"), /could not be durably recorded/);
  assert.deepEqual(publication.loadProjectPublicationIntent(), rewriteFailureIntent);
  assert.equal(rewriteFailureResumes, 0, "storage rewrite failure prevents mutation resume");

  publication.clearProjectPublicationStorageForTests();
  const terminalOldIntent = publication.beginProjectPublicationIntent({ ...seed, ownerId: retiredOwnerId });
  const terminalHarness = createControllerHarness({
    currentOwnerId: adoptedOwnerId,
    invokeStatus: async () => pendingStatus(terminalOldIntent.request, "cancelled"),
  });
  const terminalOldStatus = await terminalHarness.controller.start("backup", "autosave");
  assert.equal(terminalOldStatus.state, "cancelled");
  assert.equal(terminalHarness.calls.some((call) => call[0] === "adopt-owner"), false, "terminal receipts are never adopted");

  publication.clearProjectPublicationStorageForTests();
  storage.set("syndocal.projectPublication.v1", JSON.stringify({
    version: 1,
    origin_id: "bad-legacy-origin",
    next_request_id: 2,
    intent: {
      version: 1,
      request: {
        schema_version: 1,
        origin_id: "bad-legacy-origin",
        request_id: 1,
        owner_id: seed.ownerId,
        ...seed,
      },
    },
    acknowledgement: null,
  }));
  assert.equal(publication.loadProjectPublicationIntent(), null, "wrong-case persisted wire shapes fail closed");
  assert.equal(publication.beginProjectPublicationIntent(seed), null, "a corrupt journal never rolls to a replacement origin");

  for (const maintenance of ["get_project_publication_receipt_v1", "acknowledge_project_publication_receipt_v1"]) {
    assert.equal(operator.operatorCommandAllowed("Full", maintenance, false), true, `${maintenance} must remain available for durable receipt recovery under Full Lock`);
    assert.equal(operator.operatorCommandAllowed("Partial", maintenance, false), true, `${maintenance} must remain available for durable receipt recovery under Partial Lock`);
  }
  for (const publicationMutation of [
    "adopt_project_publication_owner_v1",
    "save_project_v1",
    "save_project_as_v1",
    "save_user_template_v1",
    "save_project_backup_v1",
    "abandon_project_publication_v1",
  ]) {
    assert.equal(operator.operatorCommandAllowed("Full", publicationMutation, true), false, `${publicationMutation} must remain blocked by Full Lock`);
    assert.equal(operator.operatorCommandAllowed("Partial", publicationMutation, true), false, `${publicationMutation} must remain blocked by Partial Lock`);
  }
} finally {
  if (priorWindow === undefined) delete globalThis.window;
  else globalThis.window = priorWindow;
  Object.defineProperty(globalThis, "crypto", { configurable: true, value: priorCrypto });
}

const [appSource, storageSource, typesSource, mainSource, invokeSource, manifestSource] = await Promise.all([
  readFile(new URL("../src/App.tsx", import.meta.url), "utf8"),
  readFile(new URL("../src/projectPublicationStorage.ts", import.meta.url), "utf8"),
  readFile(new URL("../src/types.ts", import.meta.url), "utf8"),
  readFile(new URL("../src-tauri/src/main.rs", import.meta.url), "utf8"),
  readFile(new URL("../src/tauriInvokeCommands.ts", import.meta.url), "utf8"),
  readFile(new URL("../src/tauri-invoke-manifest.json", import.meta.url), "utf8"),
]);
const manifest = JSON.parse(manifestSource);
const handlerBlock = mainSource.slice(
  mainSource.indexOf("tauri::generate_handler!"),
  mainSource.indexOf("tauri::generate_handler!") + 25_000,
);
const v1Commands = [
  "adopt_project_publication_owner_v1",
  "save_project_v1",
  "save_project_as_v1",
  "save_user_template_v1",
  "save_project_backup_v1",
  "get_project_publication_receipt_v1",
  "acknowledge_project_publication_receipt_v1",
  "abandon_project_publication_v1",
];
for (const command of v1Commands) {
  assert.ok(manifest.includes(command), `${command} must be in the generated frontend manifest`);
  assert.match(invokeSource, new RegExp(`"${command}"`), `${command} must be in the typed invoke registry`);
  assert.match(handlerBlock, new RegExp(`\\b${command}\\b`), `${command} must be registered by Tauri`);
}
for (const legacy of ["save_project", "save_project_as", "save_user_template", "save_project_backup"]) {
  assert.equal(manifest.includes(legacy), false, `${legacy} must be retired from the frontend manifest`);
  assert.doesNotMatch(invokeSource, new RegExp(`"${legacy}"`), `${legacy} must be retired from the typed invoke registry`);
}
assert.match(mainSource, /struct ProjectPublicationRequestV1[\s\S]*?serde\(rename_all = "camelCase"\)/, "V1 request must retain camelCase wire serialization");
assert.match(
  mainSource,
  /struct ProjectPublicationRequestV1[\s\S]{0,500}?owner_id: String/,
  "the native V1 request must require the same camelCase ownerId mutation admission identity",
);
assert.match(mainSource, /struct ProjectPublicationStatusV1[\s\S]*?serde\(rename_all = "camelCase"\)/, "V1 status must retain camelCase wire serialization");
for (const camelKey of ["schemaVersion", "originId", "requestId", "ownerId", "expectedProjectEpoch", "mappingAuthorityHash", "targetPolicy", "shapeHash", "targetPath", "artifactDigest", "recoveryAuthoritySerial", "warning"]) {
  assert.match(typesSource, new RegExp(`\\b${camelKey}\\b`), `frontend status/request type must use ${camelKey}`);
}
assert.match(appSource, /mediaAssetMappingPreflightProvenance\([\s\S]*?flushed\.ownAcknowledgements/, "trusted publication flush must admit only unchanged or own mapping ACK authority");
assert.match(appSource, /captureSeed: requireTrustedProjectPublicationAuthority/, "App must source publication identity from its trusted authority capture");
assert.match(appSource, /ownerId: projectTransactionOwnerId/, "every new durable publication request must bind the registered renderer transaction owner");
assert.match(
  appSource,
  /createProjectPublicationControllerV1\(\{[\s\S]*?captureSeed: requireTrustedProjectPublicationAuthority[\s\S]*?\}\)/,
  "App must wire its production save/recovery callbacks into the shared controller used by this checker",
);
assert.match(
  appSource,
  /const startProjectPublication = \([\s\S]*?projectPublicationController\.start\(/,
  "every App publication surface must enter the shared production controller",
);
assert.match(appSource, /await projectPublicationController\.recover\(\)/, "startup recovery must enter the same production controller");
const controllerOffset = storageSource.indexOf("export const createProjectPublicationControllerV1");
const controllerBlock = storageSource.slice(controllerOffset, controllerOffset + 15_000);
assert.ok(controllerOffset >= 0, "shared production V1 publication controller must exist");
assert.ok(
  controllerBlock.indexOf("const queried = await queryExisting(existing)")
    < controllerBlock.indexOf("options.captureSeed()"),
  "stored intent must be queried before any fresh mapping flush/current authority capture",
);
assert.match(controllerBlock, /different durable project publication request is unresolved/, "new UI actions must block rather than reuse a different pending request");
assert.match(controllerBlock, /beginProjectPublicationIntent\(\{[\s\S]*?authoritySeed/, "only the no-intent branch may allocate a new request identity");
assert.ok(
  controllerBlock.lastIndexOf("options.ensureMutationAllowed(command)")
    < controllerBlock.lastIndexOf("options.captureSeed()"),
  "Full/Partial lock must block a fresh publication before it can flush mappings",
);
assert.match(controllerBlock, /confirmAbandon\(intent, status, requestedSurface\)[\s\S]*?return null/, "declining recovery abandon must not mutate the stored request");
assert.match(controllerBlock, /invokeParsed\("abandon_project_publication_v1", intent\.request\)/, "explicit abandon must invoke only the durable stored request");
assert.match(controllerBlock, /await options\.settleTerminal\(abandoned\)/, "explicit abandon must settle and ACK its exact terminal receipt");
assert.match(controllerBlock, /observed\.state === "prepared"[\s\S]*?cannot be abandoned/, "Prepared/indeterminate recovery remains fail-closed instead of offering abandon");
assert.match(storageSource, /await applyTerminal\(status\)[\s\S]*?queueProjectPublicationAcknowledgement\(status\.request\)[\s\S]*?await acknowledgeTerminal\(status\.request\)/, "production settlement must apply, durably queue, then invoke exact ACK");
assert.match(controllerBlock, /projectPublicationStatusForRequestFromUnknown\(candidate, expectedRequest\)/, "every native/event status must cross the exact runtime parser before use");
assert.match(storageSource, /left\.ownerId === right\.ownerId/, "receipt identity matching must include the exact durable mutation owner");
assert.match(controllerBlock, /adoptOwner\(intent\.request, options\.currentOwnerId\)[\s\S]*?persistOwnerAdoption\(intent, adopted\)/, "owner adoption must return before the exact local intent is durably rewritten");
assert.match(storageSource, /adoptProjectPublicationIntentOwner[\s\S]*?projectPublicationRequestMatchesExceptOwner[\s\S]*?writeEnvelope/, "local adoption may change only ownerId and must durably write the replacement intent");
assert.match(appSource, /await projectTransactionOwnerRegistration[\s\S]*?adopt_project_publication_owner_v1[\s\S]*?newOwnerId/, "App must register the new owner before invoking exact adoption");
assert.doesNotMatch(appSource, /adopt_project_publication_owner_v1[\s\S]{0,500}(?:shapeHash|windowLabel)/, "the renderer must not forge native adoption shape/window authority");
assert.match(appSource, /backupRequest: updateBackup\.request/, "update preflight must pass the exact durable Backup receipt identity");
assert.match(appSource, /startProjectPublication\("backup", `before update \$\{update\.version\}`, true\)/, "update backup reason must bind the exact expected version before updater admission");
assert.match(controllerBlock, /!\(deferTerminalAcknowledgement && status\.state === "succeeded"\)[\s\S]*?options\.settleTerminal\(status\)/, "deferred Backup Success must return to the updater before any fallible UI application");
const refreshBackupsOffset = appSource.indexOf("const refreshProjectBackups = async");
const refreshBackupsBlock = appSource.slice(refreshBackupsOffset, refreshBackupsOffset + 900);
assert.match(refreshBackupsBlock, /try \{[\s\S]*?list_project_backups[\s\S]*?\} catch \(error\) \{/, "backup-list refresh rejection must remain nonfatal to terminal settlement");
assert.match(appSource, /finally \{[\s\S]{0,1200}updateBackup\?\.state === "succeeded"[\s\S]{0,900}settleProjectPublicationTerminal\(updateBackup\)/, "an update or refresh error after Backup Success must apply and ACK/clear the captured receipt instead of stranding future publications");
assert.doesNotMatch(appSource, /save_project_backup"|save_project_as"|save_user_template"|save_project"/, "App must not invoke retired raw publication commands");
assert.doesNotMatch(appSource, /install_application_update[\s\S]{0,500}midiMappings/, "update installation must not send renderer mapping arrays");

console.log("project publication E4 checks passed");
