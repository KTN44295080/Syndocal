import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import ts from "typescript";

async function importTsModule(path) {
  const source = await readFile(new URL(path, import.meta.url), "utf8");
  return importTsSource(source, path);
}

async function importTsSource(source, fileName) {
  const transpiled = ts.transpileModule(source, {
    compilerOptions: {
      module: ts.ModuleKind.ESNext,
      target: ts.ScriptTarget.ES2022,
      importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
    },
    fileName,
  });
  return import(`data:text/javascript;base64,${Buffer.from(transpiled.outputText).toString("base64")}`);
}

function balancedSourceBlock(source, start, label) {
  const open = source.indexOf("{", start);
  assert.ok(open >= start, `${label} opening brace is missing`);
  let depth = 0;
  for (let index = open; index < source.length; index += 1) {
    if (source[index] === "{") depth += 1;
    if (source[index] === "}") {
      depth -= 1;
      if (depth === 0) return source.slice(start, index + 1);
    }
  }
  assert.fail(`${label} closing brace is missing`);
}

const authority = await importTsModule("../src/mediaAssetAuthority.ts");
const liveSnapshotState = await importTsModule("../src/engineSnapshotLiveState.ts");
const controller = await readFile(new URL("../src/createVideoRuntimeController.ts", import.meta.url), "utf8");
const app = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
const mediaAuthority = await readFile(new URL("../src/mediaAssetAuthority.ts", import.meta.url), "utf8");
const availabilityFenceStart = app.indexOf("export function createMediaAssetAvailabilityApplyFence()");
assert.ok(availabilityFenceStart >= 0, "availability apply fence is missing from the production App");
const availabilityFence = await importTsSource(
  balancedSourceBlock(app, availabilityFenceStart, "availability apply fence"),
  "App.availability-fence.ts",
);
const mediaLibraryStatusLeaseStart = app.indexOf("export function createMediaLibraryStatusLease()");
assert.ok(mediaLibraryStatusLeaseStart >= 0, "Media Library shared status lease is missing from the production App");
const mediaLibraryStatusLease = await importTsSource(
  balancedSourceBlock(app, mediaLibraryStatusLeaseStart, "Media Library shared status lease"),
  "App.media-library-status-lease.ts",
);
const availabilityRowsStart = app.indexOf("export function createMediaAssetAvailabilityRowAuthority()");
assert.ok(availabilityRowsStart >= 0, "availability row-authority ledger is missing from the production App");
const availabilityRows = await importTsSource(
  balancedSourceBlock(app, availabilityRowsStart, "availability row-authority ledger"),
  "App.availability-rows.ts",
);
const mappingPreflightStart = app.indexOf("export function mediaAssetMappingPreflightProvenance(");
assert.ok(mappingPreflightStart >= 0, "mapping media preflight provenance is missing from the production App");
const mappingPreflight = await importTsSource(
  balancedSourceBlock(app, mappingPreflightStart, "mapping media preflight provenance"),
  "App.mapping-media-preflight.ts",
);

const liveSnapshot = (mediaAssets = []) => ({
  video: {
    layers: [],
    media_assets: mediaAssets,
    compositions: [],
    outputs: [],
    mapping_presets: [],
    master_opacity: 1,
    blackout: false,
  },
});

// Native serde omits an empty media_assets list from full and whole-video delta
// IPC. The live merge boundary makes the renderer contract total without
// retaining a stale catalog when the video object itself was replaced.
{
  const current = liveSnapshot([{ id: "asset-current" }]);
  const fullWithoutCatalog = liveSnapshot();
  delete fullWithoutCatalog.video.media_assets;
  const normalizedRaw = liveSnapshotState.normalizeEngineSnapshotVideoMediaAssets(fullWithoutCatalog);
  assert.deepEqual(normalizedRaw.video.media_assets, [], "the exported raw-snapshot normalizer fills an omitted empty media catalog");
  const normalizedFull = liveSnapshotState.mergeEngineSnapshotSyncResponse(current, {
    revision: 1,
    full: fullWithoutCatalog,
  });
  assert.deepEqual(normalizedFull.video.media_assets, [], "full snapshots normalize an omitted empty media catalog");

  const replacementVideo = {
    ...current.video,
    layers: [{ id: 99 }],
  };
  delete replacementVideo.media_assets;
  const normalizedWholeVideoDelta = liveSnapshotState.mergeEngineSnapshotSyncResponse(current, {
    revision: 2,
    delta: { video: replacementVideo },
  });
  assert.deepEqual(
    normalizedWholeVideoDelta.video.media_assets,
    [],
    "a whole-video delta with no catalog clears rather than retaining the prior catalog",
  );

  const normalizedNonVideoDelta = liveSnapshotState.mergeEngineSnapshotSyncResponse(current, {
    revision: 3,
    delta: { blackout: true },
  });
  assert.equal(
    normalizedNonVideoDelta.video,
    current.video,
    "a delta without video preserves the current video object",
  );
  assert.deepEqual(
    normalizedNonVideoDelta.video.media_assets,
    current.video.media_assets,
    "a delta without video preserves the current media catalog",
  );
}

const allocatedRequestIdA = authority.allocateMediaAssetRequestId();
const allocatedRequestIdB = authority.allocateMediaAssetRequestId();
assert.ok(Number.isSafeInteger(allocatedRequestIdA) && allocatedRequestIdA > 0, "client request ID stays exact and positive");
assert.equal(allocatedRequestIdB, allocatedRequestIdA + 1, "one renderer allocates monotonic request IDs");

// Exercise the actual App helpers, extracted from production source rather
// than parallel models. Per-asset ownership means a newer Verify [1] wins for
// asset 1 while the earlier Verify All [1,2] still supplies asset 2. Relink
// does not reserve/invalidate Verify by invocation order; its exact terminal
// E/R/H decides whether an already-published row is stale.
{
  const fence = availabilityFence.createMediaAssetAvailabilityApplyFence();
  const apply = (target, reservation, entries) => {
    for (const entry of entries) {
      if (fence.canApply(reservation, entry.assetId)) target[entry.assetId] = entry.value;
    }
  };
  const requestA = fence.reserveVerification([1, 2]);
  const requestB = fence.reserveVerification([1]);
  const applied = {};
  apply(applied, requestB.assets, [{ assetId: 1, value: "B" }]);
  apply(applied, requestA.assets, [{ assetId: 1, value: "A stale" }, { assetId: 2, value: "A" }]);
  assert.deepEqual(
    applied,
    { 1: "B", 2: "A" },
    "B then delayed A preserves B for the overlapping asset and applies A's non-overlapping asset",
  );

  const requestBeforeReplacement = fence.reserveVerification([4]);
  fence.reset();
  assert.equal(
    fence.canApply(requestBeforeReplacement.assets, 4),
    false,
    "project replacement/reset invalidates every outstanding availability result",
  );
  const requestAfterReplacement = fence.reserveVerification([4]);
  assert.equal(
    fence.canApply(requestBeforeReplacement.assets, 4),
    false,
    "a reused asset ID cannot resurrect a prior project's availability result",
  );
  assert.equal(fence.canApply(requestAfterReplacement.assets, 4), true, "the replacement project's new request applies normally");

  const publish = (reservation, text, target) => {
    if (fence.canPublishStatus(reservation)) target.value = text;
  };
  const status = { value: "initial" };
  const statusA = fence.reserveVerification([11, 12]);
  const statusB = fence.reserveVerification([11]);
  // B first then delayed A: A can still merge non-overlapping 12, but both
  // success and cancellation/error terminal status are globally stale.
  publish(statusB, "B success", status);
  publish(statusA, "A delayed success", status);
  publish(statusA, "A delayed cancellation", status);
  publish(statusA, "A delayed error", status);
  assert.equal(status.value, "B success", "a delayed partially applicable A never replaces B's status");

  const statusC = fence.reserveVerification([13]);
  publish(statusC, "C success", status);
  const statusD = fence.reserveVerification([13]);
  publish(statusD, "D cancellation", status);
  assert.equal(status.value, "D cancellation", "the newer completion wins when replies arrive in invocation order");
  publish(statusC, "C fully superseded", status);
  assert.equal(status.value, "D cancellation", "a fully superseded result does not publish a status");

  const statusBeforeReset = fence.reserveVerification([14]);
  fence.reset();
  publish(statusBeforeReset, "old-project error", status);
  assert.equal(status.value, "D cancellation", "reset suppresses an old project's terminal error");
  const statusAfterReset = fence.reserveVerification([14]);
  publish(statusAfterReset, "reused-id success", status);
  assert.equal(status.value, "reused-id success", "a replacement project reusing an asset ID owns its new status");

}

// Verify and Relink contend for one visible Media Library status line, without
// changing Verify's per-row E/R/H ledger. The status owner is selected when an
// action starts, not when an async response happens to return.
{
  const lease = mediaLibraryStatusLease.createMediaLibraryStatusLease();
  const status = { value: "initial" };
  const publish = (owner, text) => {
    if (lease.isCurrent(owner)) status.value = text;
  };

  const verifyA = lease.begin();
  const relinkB = lease.begin();
  publish(relinkB, "B relink canceled");
  publish(verifyA, "A verify success");
  publish(verifyA, "A verify error");
  assert.equal(status.value, "B relink canceled", "Verify then Relink keeps the newer Relink cancel over either delayed Verify terminal response");

  const relinkC = lease.begin();
  const verifyD = lease.begin();
  publish(verifyD, "D verify success");
  publish(relinkC, "C relink error");
  assert.equal(status.value, "D verify success", "Relink then Verify keeps the newer Verify over a delayed Relink error");

  const verifyE = lease.begin();
  const relinkF = lease.begin();
  publish(verifyE, "E verify success first");
  assert.equal(status.value, "D verify success", "an older Verify cannot publish even when it responds before the newer Relink");
  publish(relinkF, "F relink outcome");
  assert.equal(status.value, "F relink outcome", "the newer Relink publishes when its later outcome arrives");

  const relinkG = lease.begin();
  const verifyH = lease.begin();
  publish(relinkG, "G relink no-op first");
  assert.equal(status.value, "F relink outcome", "an older Relink no-op cannot publish before a newer Verify completes");
  publish(verifyH, "H verify error");
  assert.equal(status.value, "H verify error", "the newer Verify terminal error owns the shared status line");

  const oldProjectRelink = lease.begin();
  lease.reset();
  const reusedAssetVerify = lease.begin();
  publish(oldProjectRelink, "old reused-ID relink outcome");
  publish(reusedAssetVerify, "replacement reused-ID verify success");
  assert.equal(status.value, "replacement reused-ID verify success", "reset rejects old Relink status before a replacement project reuses an asset ID");
}

// Availability row truth belongs to the operation whose terminal E/R/H was
// actually applied. R starts on A, then V(A) publishes, then R commits B: A
// must clear. But if V(B) publishes before R's delayed frontend return, B is
// already the post-relink truth and must remain. A canceled relink has no
// reservation and cannot suppress an independent Verify.
{
  const rows = availabilityRows.createMediaAssetAvailabilityRowAuthority();
  const A = { project_epoch: 4, project_revision: 10, checkpoint_hash: "A" };
  const B = { project_epoch: 4, project_revision: 11, checkpoint_hash: "B" };
  const C = { project_epoch: 5, project_revision: 1, checkpoint_hash: "C" };
  rows.record(7, A);
  assert.equal(rows.shouldClearAfterRelink(7, B), true, "R(B) clears an older V(A) row");

  rows.record(7, B);
  assert.equal(rows.shouldClearAfterRelink(7, B), false, "a V(B) row survives a delayed R(B) frontend return");

  const verifyA = { terminal: A, current: B };
  const verifyB = { terminal: B, current: B };
  const canApplyTerminal = ({ terminal, current }) => terminal.project_epoch === current.project_epoch
    && terminal.project_revision === current.project_revision
    && terminal.checkpoint_hash === current.checkpoint_hash;
  assert.equal(canApplyTerminal(verifyA), false, "a pending V(A) completion is suppressed after R(B)");
  assert.equal(canApplyTerminal(verifyB), true, "V(B) remains applicable after R(B)");

  const verification = availabilityFence.createMediaAssetAvailabilityApplyFence().reserveVerification([7]);
  assert.equal(verification.assets.has(7), true, "relink cancellation owns no availability reservation and cannot suppress Verify");

  rows.reset();
  rows.record(7, C);
  const staleRelinkMayTouchCurrent = canApplyTerminal({ terminal: B, current: C });
  assert.equal(staleRelinkMayTouchCurrent, false, "a B relink cannot touch a reused C asset ID");
}

// Mapping preflight accepts only unchanged E/R/H or the exact final value of a
// successful local set_project_control_mappings ACK. It never infers ownership
// from a same-epoch revision number or a recovery read.
{
  const A = { project_epoch: 9, project_revision: 2, checkpoint_hash: "A" };
  const ownAck = { project_epoch: 9, project_revision: 3, checkpoint_hash: "own-ack" };
  const externalSameEpoch = { project_epoch: 9, project_revision: 3, checkpoint_hash: "external" };
  const externalAfterAck = { project_epoch: 9, project_revision: 4, checkpoint_hash: "external-after-ack" };
  assert.equal(
    mappingPreflight.mediaAssetMappingPreflightProvenance(A, A, []),
    "unchanged",
    "unchanged exact E/R/H is admitted",
  );
  assert.equal(
    mappingPreflight.mediaAssetMappingPreflightProvenance(A, ownAck, [ownAck]),
    "own_mapping_ack",
    "the exact final local ACK is admitted",
  );
  assert.equal(
    mappingPreflight.mediaAssetMappingPreflightProvenance(A, externalSameEpoch, []),
    null,
    "an external same-epoch mutation is rejected rather than guessed as a local ACK",
  );
  assert.equal(
    mappingPreflight.mediaAssetMappingPreflightProvenance(A, externalSameEpoch, [ownAck]),
    null,
    "CAS recovery/read authority is rejected even when its revision resembles an ACK",
  );
  assert.equal(
    mappingPreflight.mediaAssetMappingPreflightProvenance(A, externalAfterAck, [ownAck]),
    null,
    "an external change after a local ACK rejects the media preflight",
  );
  const strictAdoptDecisionStillCurrent = (decision, postPreflight) => decision.project_epoch === postPreflight.project_epoch
    && decision.project_revision === postPreflight.project_revision
    && decision.checkpoint_hash === postPreflight.checkpoint_hash;
  assert.equal(
    strictAdoptDecisionStillCurrent(A, ownAck),
    false,
    "an Adopt decision is invalidated by any mapping advance and requires confirmation again",
  );
}

// A dirty mapping flush may abort already-running media work. The new
// operation must not be registered until that flush and its epoch fence settle.
{
  const older = new AbortController();
  let current = null;
  const order = [];
  const gestureAuthority = { project_epoch: 44, project_revision: 8, checkpoint_hash: "before-flush" };
  const postFlushAuthority = { project_epoch: 44, project_revision: 9, checkpoint_hash: "after-flush" };
  const started = await authority.preflightAndBeginMediaAssetOperation(
    gestureAuthority,
    async (expectedAuthority) => {
      order.push("flush");
      assert.equal(current, null, "the initiating AbortController does not exist during mapping flush");
      older.abort();
      assert.deepEqual(expectedAuthority, gestureAuthority, "the picker retains its original complete E/R/H fence through preflight");
      return { authority: postFlushAuthority, provenance: "own_mapping_ack" };
    },
    () => {
      order.push("register");
      current = new AbortController();
      return { signal: current.signal, release: () => undefined };
    },
  );
  assert.deepEqual(order, ["flush", "register"]);
  assert.equal(older.signal.aborted, true, "mapping publication still retires older work");
  assert.equal(started.operation.signal.aborted, false, "the operation being started never self-aborts");
  assert.deepEqual(started.authority, postFlushAuthority, "the operation binds the post-flush complete E/R/H, not its stale picker snapshot");

  let replacementRegistered = false;
  await assert.rejects(
    authority.preflightAndBeginMediaAssetOperation(
      gestureAuthority,
      async () => ({ authority: { ...postFlushAuthority, project_epoch: 45 }, provenance: "replacement" }),
      () => {
        replacementRegistered = true;
        return { signal: new AbortController().signal, release: () => undefined };
      },
    ),
    /Project changed before the media operation started/,
    "a project replacement during preflight rejects before registering a controller",
  );
  assert.equal(replacementRegistered, false, "a replacement project cannot acquire the old gesture's AbortController");
}

assert.equal(authority.trimLikeRust("\u3000 Layer A \u3000"), "Layer A", "Rust Unicode whitespace is trimmed");
assert.equal(
  authority.trimLikeRust("\ufeffLayer A\ufeff"),
  "\ufeffLayer A\ufeff",
  "U+FEFF is retained exactly like Rust str::trim for terminal fingerprints",
);

const startReport = (requestId = 4101, overrides = {}) => ({
  request_id: requestId,
  operation_generation: 73,
  project_epoch: 44,
  project_revision: 8,
  checkpoint_hash: "checkpoint-a",
  ...overrides,
});

const importReport = (args = {}, overrides = {}) => ({
  request_id: args.requestId ?? 4101,
  operation_generation: args.operationGeneration ?? 73,
  prepared_import_token: 991,
  project_epoch: 44,
  project_revision: 8,
  checkpoint_hash: "checkpoint-a",
  prepared: 1,
  imported: 0,
  reused: 0,
  skipped: 0,
  failed: 0,
  entries: [{ input_index: 0, path: "C:/show/a.mov", status: "prepared", asset_id: null, message: null }],
  ...overrides,
});

const relinkReport = (args = {}, overrides = {}) => ({
  request_id: args.requestId ?? 4101,
  operation_generation: args.operationGeneration ?? 73,
  prepared_relink_token: 992,
  project_epoch: 44,
  project_revision: 8,
  checkpoint_hash: "checkpoint-a",
  outcome: null,
  ...overrides,
});

// A same-epoch change between renderer preflight and Start must be rejected
// before Prepare opens a file or Commit can mutate the newer project. Start's
// returned reservation is still named exactly once for cleanup.
{
  const calls = [];
  const expectedAuthority = { project_epoch: 44, project_revision: 8, checkpoint_hash: "checkpoint-a" };
  const invoke = async (command, args = {}) => {
    calls.push({ command, args });
    if (command === "start_media_asset_operation") {
      return startReport(args.requestId, { project_revision: 9, checkpoint_hash: "checkpoint-b" });
    }
    if (command === "cancel_media_asset_operation") return true;
    throw new Error(`Unexpected command ${command}`);
  };
  await assert.rejects(
    authority.prepareFinalizeAndCommitMediaAssets({
      invoke,
      kind: "File",
      paths: ["C:/show/one.mov"],
      expectedEpoch: expectedAuthority.project_epoch,
      expectedAuthority,
      ownerId: "renderer:test",
      requestId: 4101,
      commitCommand: "commit_prepared_media_assets_authoritative",
    }),
    /different project authority/,
    "Start refuses a same-epoch E/R/H replacement that occurred after preflight",
  );
  assert.deepEqual(calls.map(({ command }) => command), [
    "start_media_asset_operation",
    "cancel_media_asset_operation",
  ]);
}

const mutation = (revision = 9) => ({
  history_status: { project_epoch: 44, project_revision: revision, checkpoint_hash: `checkpoint-${revision}` },
  authority: { project_epoch: 44, project_revision: revision, checkpoint_hash: `checkpoint-${revision}` },
});

const appendLengthPrefixed = (hash, value) => {
  const bytes = Buffer.from(value, "utf8");
  const length = Buffer.alloc(8);
  length.writeBigUInt64LE(BigInt(bytes.length));
  hash.update(length);
  hash.update(bytes);
};

const shapeFingerprint = (kind, fields = []) => {
  const hash = createHash("sha256");
  hash.update("syndocal-media-authoritative-shape-v1", "utf8");
  appendLengthPrefixed(hash, kind);
  fields.forEach((field) => appendLengthPrefixed(hash, field));
  return hash.digest("hex");
};

const baseOptions = (invoke, overrides = {}) => ({
  invoke,
  kind: "File",
  paths: ["C:/show/a.mov"],
  expectedEpoch: 44,
  ownerId: "renderer:test",
  requestId: 4101,
  commitCommand: "commit_prepared_media_assets_authoritative",
  ...overrides,
});

// Normal catalog import: Start -> Reserved Prepare -> Finalize -> direct
// authoritative commit. There is no renderer Begin/Commit ticket in the helper.
{
  const calls = [];
  const invoke = async (command, args = {}) => {
    calls.push({ command, args });
    if (command === "start_media_asset_operation") return startReport(args.requestId);
    if (command === "prepare_reserved_media_assets") return importReport(args);
    if (command === "finalize_prepared_media_assets") return importReport(args);
    if (command === "commit_prepared_media_assets_authoritative") {
      assert.equal(args.preparedImportToken, 991);
      assert.equal(args.expectedEpoch, 44);
      assert.equal(args.expectedRevision, 8);
      assert.equal(args.expectedCheckpointHash, "checkpoint-a");
      assert.equal(args.ownerId, "renderer:test");
      assert.equal(args.__expectedProjectEpoch, 44, "App receives the picker/start epoch fence");
      assert.equal(args.__shouldAbortProjectMutation(), false, "App receives a live abort-before-dispatch predicate");
      assert.equal(args.projectTransactionId, undefined, "authoritative commit has no renderer transaction ticket");
      return { report: importReport(args, { prepared_import_token: null, imported: 1 }), mutation: mutation() };
    }
    throw new Error(`Unexpected command ${command}`);
  };
  const result = await authority.prepareFinalizeAndCommitMediaAssets(baseOptions(invoke));
  assert.equal(result.committed.imported, 1, "catalog result comes from the authoritative terminal report");
  assert.deepEqual(
    result.terminalAuthority,
    { project_epoch: 44, project_revision: 9, checkpoint_hash: "checkpoint-9" },
    "callers receive the terminal B token for a second live check before UI side effects",
  );
  assert.deepEqual(
    calls.map(({ command }) => command),
    [
      "start_media_asset_operation",
      "prepare_reserved_media_assets",
      "finalize_prepared_media_assets",
      "commit_prepared_media_assets_authoritative",
    ],
    "authoritative phase order is exact",
  );
}

// Rust retains U+FEFF around a label. Lost-reply recovery must compute the same
// shape rather than ECMAScript-trimming it away.
{
  const label = "\ufeffLayer A\ufeff";
  let commitAttempts = 0;
  const invoke = async (command, args = {}) => {
    if (command === "start_media_asset_operation") return startReport(args.requestId);
    if (command === "prepare_reserved_media_assets" || command === "finalize_prepared_media_assets") return importReport(args);
    if (command === "commit_prepared_video_file_layer_authoritative") {
      commitAttempts += 1;
      throw new Error("response lost");
    }
    if (command === "get_media_asset_operation_terminal_result") {
      return {
        command_kind: "video_file_layer",
        shape_fingerprint: shapeFingerprint("video_file_layer", [label]),
        terminal: { kind: "layers", result: { layer_ids: [17], mutation: mutation() } },
      };
    }
    throw new Error(`Unexpected command ${command}`);
  };
  const result = await authority.prepareFinalizeAndCommitMediaAssets(baseOptions(invoke, {
    commitCommand: "commit_prepared_video_file_layer_authoritative",
    commitArgs: { label },
  }));
  assert.equal(result.committed, 17, "U+FEFF label receipt is recoverable after both direct replies are lost");
  assert.equal(commitAttempts, 2);
}

// Lost commit response: exact retry also loses transport, then the same
// operation identity queries the terminal receipt and resolves one success.
{
  const calls = [];
  let commitAttempts = 0;
  const terminalResult = { report: importReport({}, { prepared_import_token: null, imported: 1 }), mutation: mutation() };
  const invoke = async (command, args = {}) => {
    calls.push({ command, args });
    if (command === "start_media_asset_operation") return startReport(args.requestId);
    if (command === "prepare_reserved_media_assets" || command === "finalize_prepared_media_assets") return importReport(args);
    if (command === "commit_prepared_media_assets_authoritative") {
      commitAttempts += 1;
      throw new Error(commitAttempts === 1 ? "response lost after publish" : "retry transport lost");
    }
    if (command === "get_media_asset_operation_terminal_result") {
      assert.deepEqual(args, {
        preparedToken: 991,
        requestId: 4101,
        operationGeneration: 73,
        expectedEpoch: 44,
        expectedRevision: 8,
        expectedCheckpointHash: "checkpoint-a",
        ownerId: "renderer:test",
      }, "terminal query retains the exact Start authority and operation identity");
      return {
        command_kind: "import",
        shape_fingerprint: shapeFingerprint("import"),
        terminal: { kind: "import", result: terminalResult },
      };
    }
    throw new Error(`Unexpected command ${command}`);
  };
  const result = await authority.prepareFinalizeAndCommitMediaAssets(baseOptions(invoke));
  assert.equal(result.committed.imported, 1, "lost reply resolves through the canonical terminal receipt");
  assert.equal(commitAttempts, 2, "recovery retries the exact authoritative command once");
  assert.equal(calls.filter(({ command }) => command === "get_media_asset_operation_terminal_result").length, 1, "one terminal query settles the operation");
  assert.equal(calls.some(({ command }) => command === "cancel_media_asset_operation"), false, "a resolved terminal success is never cancelled");
}

// A delayed B terminal after project C is still definitive, but App marks that
// its paired authority/history was rejected by the monotonic guard. Callers can
// then avoid presenting a current-project success message.
{
  const invoke = async (command, args = {}) => {
    if (command === "start_media_asset_operation") return startReport(args.requestId);
    if (command === "prepare_reserved_media_assets" || command === "finalize_prepared_media_assets") return importReport(args);
    if (command === "commit_prepared_media_assets_authoritative") {
      const response = { report: importReport(args, { prepared_import_token: null, imported: 1 }), mutation: mutation() };
      Object.defineProperty(response, "__syndocalAuthoritativeApplicationCurrent", { value: false });
      return response;
    }
    throw new Error(`Unexpected command ${command}`);
  };
  const result = await authority.prepareFinalizeAndCommitMediaAssets(baseOptions(invoke));
  assert.equal(result.applicationCurrent, false, "stale B terminal is exposed as non-current without republishing");
}

// Abort before dispatch: Finalize can complete, but no authoritative commit is
// sent and exact cancellation names the already-reserved operation.
{
  const abortController = new AbortController();
  const calls = [];
  const invoke = async (command, args = {}) => {
    calls.push({ command, args });
    if (command === "start_media_asset_operation") return startReport(args.requestId);
    if (command === "prepare_reserved_media_assets") return importReport(args);
    if (command === "finalize_prepared_media_assets") {
      abortController.abort();
      return importReport(args);
    }
    if (command === "cancel_media_asset_operation") return true;
    throw new Error(`Unexpected command ${command}`);
  };
  await assert.rejects(
    authority.prepareFinalizeAndCommitMediaAssets(baseOptions(invoke, { signal: abortController.signal })),
    (error) => error?.name === "AbortError",
  );
  assert.equal(calls.some(({ command }) => command.includes("_authoritative")), false, "abort before dispatch makes zero backend commit calls");
  const cancel = calls.find(({ command }) => command === "cancel_media_asset_operation");
  assert.equal(cancel.args.requestId, 4101);
  assert.equal(cancel.args.operationGeneration, 73);
}

// Abort after backend admission: cancel loses the backend CAS and the UI waits
// for/returns the definitive success instead of reporting Cancelled.
{
  const abortController = new AbortController();
  let cancelCalls = 0;
  const invoke = async (command, args = {}) => {
    if (command === "start_media_asset_operation") return startReport(args.requestId);
    if (command === "prepare_reserved_media_assets" || command === "finalize_prepared_media_assets") return importReport(args);
    if (command === "cancel_media_asset_operation") {
      cancelCalls += 1;
      return false;
    }
    if (command === "commit_prepared_media_assets_authoritative") {
      abortController.abort();
      await Promise.resolve();
      return { report: importReport(args, { prepared_import_token: null, imported: 1 }), mutation: mutation() };
    }
    throw new Error(`Unexpected command ${command}`);
  };
  const result = await authority.prepareFinalizeAndCommitMediaAssets(baseOptions(invoke, { signal: abortController.signal }));
  assert.equal(result.committed.imported, 1, "admitted publication returns terminal success after abort");
  assert.equal(cancelCalls, 1, "abort still submits one exact backend cancellation request");
}

// Same-epoch authority drift between Start and Prepare is rejected before any
// commit; the only cleanup request uses the exact echoed operation identity.
{
  const calls = [];
  const invoke = async (command, args = {}) => {
    calls.push({ command, args });
    if (command === "start_media_asset_operation") return startReport(args.requestId);
    if (command === "prepare_reserved_media_assets") return importReport(args, { project_revision: 9 });
    if (command === "cancel_media_asset_operation") return true;
    throw new Error(`Unexpected command ${command}`);
  };
  await assert.rejects(
    authority.prepareFinalizeAndCommitMediaAssets(baseOptions(invoke)),
    /project authority changed/,
  );
  assert.equal(calls.some(({ command }) => command.includes("_authoritative")), false, "authority A->B drift publishes nothing");
}

// First-run remains all-or-nothing: mixed Prepare truth cancels before
// Finalize and before the authoritative Bootstrap command.
{
  const calls = [];
  let rejectedReport = null;
  const invoke = async (command, args = {}) => {
    calls.push({ command, args });
    if (command === "start_media_asset_operation") return startReport(args.requestId);
    if (command === "prepare_reserved_media_assets") {
      return importReport(args, {
        prepared: 1,
        failed: 1,
        entries: [
          { input_index: 0, path: "C:/show/a.mov", status: "prepared" },
          { input_index: 1, path: "C:/show/missing.mov", status: "failed" },
        ],
      });
    }
    if (command === "cancel_media_asset_operation") return true;
    throw new Error(`Unexpected command ${command}`);
  };
  await assert.rejects(
    authority.prepareFinalizeAndCommitMediaAssets(baseOptions(invoke, {
      paths: ["C:/show/a.mov", "C:/show/missing.mov"],
      commitCommand: "commit_prepared_bootstrap_vj_show_authoritative",
      commitArgs: { kind: "File" },
      requireAllPrepared: true,
      onReport: (report) => { rejectedReport = report; },
    })),
    /requires every selected media file/,
  );
  assert.equal(calls.some(({ command }) => command === "finalize_prepared_media_assets"), false);
  assert.equal(calls.some(({ command }) => command === "commit_prepared_bootstrap_vj_show_authoritative"), false);
  assert.equal(rejectedReport.entries[1].path, "C:/show/missing.mov", "mixed rejection still publishes its per-entry report to the UI");
  assert.equal(rejectedReport.entries[1].status, "failed");
}

// Relink uses the same Start authority, Reserved Prepare, Finalize, direct
// authoritative commit and typed terminal outcome.
{
  const calls = [];
  const invoke = async (command, args = {}) => {
    calls.push({ command, args });
    if (command === "start_media_asset_operation") return startReport(args.requestId);
    if (command === "prepare_reserved_media_asset_relink" || command === "finalize_prepared_media_asset_relink") {
      return relinkReport(args);
    }
    if (command === "commit_prepared_media_asset_relink_authoritative") {
      assert.equal(args.preparedRelinkToken, 992);
      assert.equal(args.expectedRevision, 8);
      return {
        report: relinkReport(args, {
          prepared_relink_token: null,
          outcome: { kind: "relinked", asset_id: 7, adopted_replacement: false },
        }),
        mutation: mutation(),
      };
    }
    throw new Error(`Unexpected command ${command}`);
  };
  const result = await authority.prepareFinalizeAndCommitMediaAssetRelink({
    invoke,
    expectedEpoch: 44,
    ownerId: "renderer:test",
    requestId: 4101,
    assetId: 7,
    replacementPath: "C:/show/a.mov",
    policy: "RequireContentMatch",
  });
  assert.equal(result.committed, true);
  assert.equal(result.outcome.kind, "relinked");
  assert.deepEqual(calls.map(({ command }) => command), [
    "start_media_asset_operation",
    "prepare_reserved_media_asset_relink",
    "finalize_prepared_media_asset_relink",
    "commit_prepared_media_asset_relink_authoritative",
  ]);
}

// A receipt for another semantic shape is never accepted as this operation's
// result, even if the operation ID was somehow aliased by a hostile renderer.
{
  const invoke = async (command, args = {}) => {
    if (command === "start_media_asset_operation") return startReport(args.requestId);
    if (command === "prepare_reserved_media_assets" || command === "finalize_prepared_media_assets") return importReport(args);
    if (command === "commit_prepared_video_file_layer_authoritative") throw new Error("reply lost");
    if (command === "get_media_asset_operation_terminal_result") {
      return {
        command_kind: "video_file_layer",
        shape_fingerprint: shapeFingerprint("video_file_layer", ["Different label"]),
        terminal: { kind: "layers", result: { layer_ids: [17], mutation: mutation() } },
      };
    }
    if (command === "cancel_media_asset_operation") return false;
    throw new Error(`Unexpected command ${command}`);
  };
  await assert.rejects(
    authority.prepareFinalizeAndCommitMediaAssets(baseOptions(invoke, {
      commitCommand: "commit_prepared_video_file_layer_authoritative",
      commitArgs: { label: "Expected label" },
    })),
    /different operation shape/,
  );
}

// Verified availability has its own non-mutating reservation. It must carry
// exact request/generation/E/R/H into the sole inspect command and expose the
// truthful long-I/O phase without entering any project commit path.
{
  const calls = [];
  const phases = [];
  const invoke = async (command, args = {}) => {
    calls.push({ command, args });
    if (command === "start_media_asset_availability_operation") {
      assert.deepEqual(args, { requestId: 4101, expectedEpoch: 44, ownerId: "renderer:test" });
      return startReport(args.requestId);
    }
    if (command === "inspect_reserved_media_asset_availability") {
      assert.deepEqual(args, {
        requestId: 4101,
        operationGeneration: 73,
        assetIds: [7, 9],
        verifyHash: true,
        expectedEpoch: 44,
        ownerId: "renderer:test",
      });
      return {
        ...startReport(args.requestId),
        availability: [
          { kind: "available_verified", asset_id: 7 },
          { kind: "missing", asset_id: 9 },
        ],
      };
    }
    throw new Error(`Unexpected command ${command}`);
  };
  const result = await authority.inspectMediaAssetAvailability({
    invoke,
    expectedEpoch: 44,
    ownerId: "renderer:test",
    requestId: 4101,
    assetIds: [7, 9],
    verifyHash: true,
    onPhase: (phase) => phases.push(phase),
  });
  assert.deepEqual(phases, ["preparing", "hashing"], "availability exposes only phases it actually performs");
  assert.deepEqual(result.terminalAuthority, {
    project_epoch: 44,
    project_revision: 8,
    checkpoint_hash: "checkpoint-a",
  });
  assert.deepEqual(calls.map(({ command }) => command), [
    "start_media_asset_availability_operation",
    "inspect_reserved_media_asset_availability",
  ]);
  assert.equal(calls.some(({ command }) => command.includes("commit")), false, "availability opens no project commit/history path");
}

// Same-epoch authority drift is rejected and cleanup names only the exact
// availability reservation returned by Start.
{
  const calls = [];
  const invoke = async (command, args = {}) => {
    calls.push({ command, args });
    if (command === "start_media_asset_availability_operation") return startReport(args.requestId);
    if (command === "inspect_reserved_media_asset_availability") {
      return { ...startReport(args.requestId, { project_revision: 9 }), availability: [] };
    }
    if (command === "cancel_media_asset_operation") return true;
    throw new Error(`Unexpected command ${command}`);
  };
  await assert.rejects(
    authority.inspectMediaAssetAvailability({
      invoke,
      expectedEpoch: 44,
      ownerId: "renderer:test",
      requestId: 4101,
      assetIds: [7],
      verifyHash: true,
    }),
    /project authority changed/,
  );
  assert.deepEqual(calls.at(-1), {
    command: "cancel_media_asset_operation",
    args: { requestId: 4101, operationGeneration: 73, ownerId: "renderer:test" },
  });
}

// Once backend completion wins, an AbortSignal delivered with the successful
// report cannot make the renderer report a false cancellation.
{
  const abortController = new AbortController();
  let cancelCalls = 0;
  const invoke = async (command, args = {}) => {
    if (command === "start_media_asset_availability_operation") return startReport(args.requestId);
    if (command === "inspect_reserved_media_asset_availability") {
      abortController.abort();
      await Promise.resolve();
      return { ...startReport(args.requestId), availability: [{ kind: "available_verified", asset_id: 7 }] };
    }
    if (command === "cancel_media_asset_operation") {
      cancelCalls += 1;
      return false;
    }
    throw new Error(`Unexpected command ${command}`);
  };
  const result = await authority.inspectMediaAssetAvailability({
    invoke,
    expectedEpoch: 44,
    ownerId: "renderer:test",
    requestId: 4101,
    assetIds: [7],
    verifyHash: true,
    signal: abortController.signal,
  });
  assert.equal(result.report.availability[0].kind, "available_verified");
  assert.equal(cancelCalls, 1, "late cancel still reaches the backend completion CAS exactly once");
}

// Production wiring: new commands are a dedicated mutation class, are routed
// through mapping/epoch/abort fences, and return before generic Begin.
for (const command of [
  "commit_prepared_media_assets_authoritative",
  "commit_prepared_media_asset_relink_authoritative",
  "commit_prepared_video_file_layer_authoritative",
  "commit_prepared_still_image_layer_authoritative",
  "commit_prepared_local_media_layers_authoritative",
  "commit_prepared_bootstrap_vj_show_authoritative",
]) {
  assert.match(app, new RegExp(`serverAuthoritativeProjectMutationCommands[\\s\\S]*?"${command}"`), `${command} stays classified as a mutation`);
}
assert.match(app, /const projectMutation = rendererTicketedMutation \|\| serverAuthoritativeMutation;/, "operator policy sees authoritative commands as mutations");
assert.match(app, /const currentEpoch = await flushProjectControlMappingsBeforeProjectMutation[\s\S]*?if \(shouldAbortProjectMutation\?\.\(\)\)[\s\S]*?if \(serverAuthoritativeMutation\)[\s\S]*?return result;[\s\S]*?begin_project_transaction/, "mapping flush and abort fence precede direct authoritative dispatch and bypass Begin");
assert.match(app, /const terminalRecovery = mediaAssetTerminalRecoveryCommands\.has\(command\);[\s\S]*?!terminalRecovery && !mediaAssetAvailabilityReadOnly[\s\S]*?!operatorCommandAllowed/, "Full Lock still permits exact terminal query/cancel cleanup");
const mediaStartFence = app.slice(
  app.indexOf("const prepareMediaAssetOperationStart = async"),
  app.indexOf("  createEffect(() =>", app.indexOf("const prepareMediaAssetOperationStart = async")),
);
assert.ok(
  mediaStartFence.indexOf("operatorCommandAllowed") < mediaStartFence.indexOf("flushProjectControlMappingsAuthority"),
  "Operator admission is checked before a pre-Start mapping flush",
);
assert.match(mediaStartFence, /No new Media AbortController exists yet/, "mapping flush documents and enforces no self-abort registration");
assert.match(app, /dispatchProjectHistoryMutationFromUnknown\(result\)/, "direct reply applies paired history/authority through the common event path");
assert.match(app, /applyServerAuthoritativeProjectMutationResult = applyProjectHistoryMutationResult/, "direct result uses the production monotonic authority/history helper synchronously");
assert.match(app, /markAuthoritativeApplicationCurrent\(result, current\)/, "stale terminal application truth is returned to the caller");
assert.match(app, /get_media_asset_operation_terminal_result[\s\S]*?dispatchProjectHistoryMutationFromUnknown/, "terminal query applies the same paired result path");
const historyApply = app.slice(
  app.indexOf("const applyProjectHistoryMutationResult ="),
  app.indexOf("applyServerAuthoritativeProjectMutationResult =", app.indexOf("const applyProjectHistoryMutationResult =")),
);
const applyEngineSnapshotSource = app.slice(
  app.indexOf("const applyEngineSnapshot = ("),
  app.indexOf("  type SnapshotRefreshWaiter =", app.indexOf("const applyEngineSnapshot = (")),
);
assert.match(
  applyEngineSnapshotSource,
  /const applyEngineSnapshot = \(\s*incoming: EngineSnapshot,[\s\S]*?const next = normalizeEngineSnapshotVideoMediaAssets\(incoming\);/,
  "raw full and authority snapshots normalize at the common apply boundary",
);
assert.match(
  historyApply,
  /projectAuthorityTokenIsCurrent[\s\S]*?applyAuthoritativeProjectHistoryStatus\(result\.history_status\);[\s\S]*?return true;/,
  "an exact current content token remains current when only a newer history status won",
);
assert.match(app, /lock_project_operator_session/, "UI lock synchronizes the backend owner session");
assert.match(app, /unlock_project_operator_session/, "UI unlock is verified by the backend owner session");
const operatorSessionLock = app.slice(
  app.indexOf("const setOperatorSessionLock ="),
  app.indexOf("const refreshOperatorPolicy =", app.indexOf("const setOperatorSessionLock =")),
);
assert.match(
  operatorSessionLock,
  /catch[\s\S]*?setOperatorLockMode\(mode\)[\s\S]*?remains locked/,
  "a lost lock reply keeps the requested local lock mode fail-closed",
);
const configureOperator = app.slice(
  app.indexOf("const configureOperatorPolicy = async"),
  app.indexOf("const clearOperatorPolicy = async"),
);
assert.match(configureOperator, /desiredSessionMode = policy\.lock_on_load \? policy\.lock_mode : null/, "policy change mirrors the backend owner-session default");
assert.match(configureOperator, /lock_project_operator_session/, "lock-on-load policy synchronizes the current backend owner immediately");

assert.match(controller, /commit_prepared_video_file_layer_authoritative/, "File Add uses authoritative single-layer commit");
assert.match(controller, /commit_prepared_still_image_layer_authoritative/, "Still Add uses authoritative single-layer commit");
assert.match(controller, /commit_prepared_media_assets_authoritative/, "ordinary Import remains authoritative catalog-only");
assert.match(controller, /prepareFinalizeAndCommitMediaAssetRelink/, "metadata refresh uses authoritative staged relink");
assert.match(controller, /preflightAndBeginMediaAssetOperation/, "all media routes flush before registering the operation AbortController");
assert.match(controller, /firstOutcome\?\.kind !== "needs_explicit_adoption"[\s\S]*?confirmMediaAssetAdoption[\s\S]*?"AdoptReplacement"/, "AdoptReplacement is reachable only after the backend's dedicated decision and explicit confirmation");
assert.match(controller, /decisionAuthority = first\.terminalAuthority[\s\S]*?!options\.isProjectAuthorityCurrent\(decisionAuthority\)[\s\S]*?Choose the replacement again/, "AdoptReplacement cannot cross a project authority replacement during confirmation");
const adoptionRetryHelper = controller.slice(
  controller.indexOf("const runMediaAssetRelink = async"),
  controller.indexOf("const relinkMediaAsset = async"),
);
assert.match(adoptionRetryHelper, /const expectedAuthority = decisionAuthority \?\? options\.getCurrentProjectAuthority\(\);[\s\S]*?preflightAndBeginMediaAssetOperation[\s\S]*?const operationAuthority = preparedStart\.authority;/, "the adoption retry carries its captured decision through preflight");
assert.ok(
  adoptionRetryHelper.includes("if (decisionAuthority && !sameProjectAuthority(decisionAuthority, operationAuthority))")
    && adoptionRetryHelper.includes("Project changed while replacement approval was being confirmed; choose the replacement again."),
  "AdoptReplacement rejects any post-decision mapping/authority advance before its first backend mutation",
);
assert.match(adoptionRetryHelper, /!options\.isProjectAuthorityCurrent\(operationAuthority\)[\s\S]*?expectedAuthority: operationAuthority/, "the retry binds its own post-preflight authority through Start and terminal commit");
assert.match(controller, /policy: "RequireContentMatch"/, "asset picker relink always attempts exact content match first");
assert.match(
  controller,
  /const initiatingAuthority = options\.getCurrentProjectAuthority\(\);[\s\S]*?const operationAuthority = preparedStart\.authority;[\s\S]*?select_video_source_file[\s\S]*?!options\.isProjectAuthorityCurrent\(operationAuthority\)[\s\S]*?prepareFinalizeAndCommitMediaAssetRelink/,
  "relink preserves its initiating epoch but uses post-preflight E/R/H across the picker and suppresses C's stale continuation",
);
assert.match(
  controller,
  /await options\.refreshSnapshot\(\);[\s\S]*?!options\.isProjectAuthorityCurrent\(first\.terminalAuthority\)[\s\S]*?terminalAuthority: first\.terminalAuthority/,
  "relink rechecks terminal authority after refresh before reporting/returning a reusable asset ID",
);
assert.match(
  controller,
  /setRelinkMessageIfAuthorityCurrent\(sideEffectAuthority, String\(error\)\)/,
  "relink errors are suppressed by both its initiating authority and the App-owned status lease",
);
assert.match(mediaAuthority, /start_media_asset_availability_operation[\s\S]*?inspect_reserved_media_asset_availability/, "availability uses its dedicated reservation and inspect commands");
assert.match(mediaAuthority, /assertReportContinuity\("Reserved Availability Inspect"/, "availability verifies exact request\/generation\/E\/R\/H continuity");
assert.match(
  app,
  /const mediaAssetAvailabilityReadOnlyCommands = new Set\(\[[\s\S]*?"start_media_asset_availability_operation",[\s\S]*?"inspect_reserved_media_asset_availability",[\s\S]*?\]\);/,
  "Availability Start and Inspect remain explicitly classified as machine-local read-only commands",
);
assert.match(
  app,
  /const mediaAssetAvailabilityReadOnly = mediaAssetAvailabilityReadOnlyCommands\.has\(command\);[\s\S]*?!terminalRecovery && !mediaAssetAvailabilityReadOnly[\s\S]*?!operatorCommandAllowed/,
  "Full/Partial lock admission exempts only Availability reads while retaining the normal mutation gate",
);
const availabilityPreflight = app.slice(
  app.indexOf("const prepareMediaAssetAvailabilityInspection = async"),
  app.indexOf("  createEffect(() =>", app.indexOf("const prepareMediaAssetAvailabilityInspection = async")),
);
assert.match(availabilityPreflight, /const authority = captureProjectAuthorityIdentity\(\);/, "Availability preflight captures its full current E/R/H");
assert.match(availabilityPreflight, /authority\.project_epoch !== expectedAuthority\.project_epoch/, "Availability preflight rejects a changed epoch");
assert.match(availabilityPreflight, /return \{ authority, provenance: "read_only" \};/, "Availability preflight returns the exact E/R/H that later side effects must use");
assert.doesNotMatch(availabilityPreflight, /flushProjectControlMappingsBeforeMutation|operatorCommandAllowed/, "Availability preflight performs no mutation flush or lock gate");
const availabilityWorkflow = app.slice(
  app.indexOf("const inspectMediaAssetIds = async"),
  app.indexOf("const relinkMediaLibraryAsset", app.indexOf("const inspectMediaAssetIds = async")),
);
assert.match(availabilityWorkflow, /mediaAssetAvailabilityApplyFence\.reserveVerification\(requested\)[\s\S]*?prepareMediaAssetAvailabilityInspection[\s\S]*?inspectMediaAssetAvailability/, "Availability reserves per-asset apply and verification-status ownership before inspect completion");
assert.match(availabilityWorkflow, /const statusLease = mediaLibraryStatusLease\.begin\(\);[\s\S]*?const setMediaLibraryStatus = \(message: string\) => \{[\s\S]*?mediaLibraryStatusLease\.isCurrent\(statusLease\)/, "Verify reserves the shared Media Library status lease at invocation time, including no-op verification");
assert.match(availabilityWorkflow, /mediaAssetAvailabilityApplyFence\.canApply\(availabilityReservation\.assets, availability\.asset_id\)/, "Availability only merges entries still owned by that request");
assert.match(availabilityWorkflow, /mediaAssetAvailabilityRowAuthority\.record\(availability\.asset_id, inspected\.terminalAuthority\)/, "every applied availability row records its exact terminal E/R/H");
assert.match(availabilityWorkflow, /mediaAssetAvailabilityApplyFence\.canPublishStatus\(availabilityReservation\)[\s\S]*?mediaLibraryStatusLease\.isCurrent\(statusLease\)/, "Availability status, cancellation, error, and supersession require both newest Verify and newest Media Library invocation");
assert.match(availabilityWorkflow, /const initiatingAuthority = captureProjectAuthorityIdentity\(\);[\s\S]*?let sideEffectAuthority = initiatingAuthority;[\s\S]*?sideEffectAuthority = preparedStart\.authority;[\s\S]*?isProjectAuthorityIdentityCurrent\(sideEffectAuthority\)/, "Availability preserves its initiating epoch and binds its returned E/R/H through hashing");
const mediaAssetUiReset = app.slice(
  app.indexOf("const resetMediaAssetUiForProjectReplacement = () => {"),
  app.indexOf("\n  };", app.indexOf("const resetMediaAssetUiForProjectReplacement = () => {")),
);
assert.match(mediaAssetUiReset, /mediaAssetAvailabilityApplyFence\.reset\(\)/, "project replacement invalidates outstanding per-asset availability ownership");
assert.match(mediaAssetUiReset, /mediaLibraryStatusLease\.reset\(\)/, "project replacement invalidates Verify and Relink shared status ownership before IDs may be reused");
assert.match(mediaAssetUiReset, /mediaAssetAvailabilityRowAuthority\.reset\(\)/, "project replacement clears row authority before IDs may be reused");
assert.match(mediaAssetUiReset, /setVjFirstRunError\(null\)/, "project replacement clears a retired first-run persistent error");
const sourcePicker = app.slice(
  app.indexOf("const selectVideoSourceFile = async"),
  app.indexOf("const downloadGdtfFromUrl", app.indexOf("const selectVideoSourceFile = async")),
);
assert.match(sourcePicker, /const initiatingAuthority = captureProjectAuthorityIdentity\(\);[\s\S]*?let sideEffectAuthority = initiatingAuthority;[\s\S]*?sideEffectAuthority = preparedStart\.authority;[\s\S]*?select_video_source_file[\s\S]*?if \(!sideEffectsAreCurrent\(\)\) return;/, "Browse preserves its initiating epoch and cannot publish cancel/path/error into C after binding post-preflight E/R/H");
assert.match(app, /setMediaAssetImportReport: \(report\) => \{[\s\S]*?isProjectAuthorityIdentityCurrent\(\{[\s\S]*?report\.project_epoch[\s\S]*?report\.project_revision[\s\S]*?report\.checkpoint_hash/, "delayed Prepare truth cannot repopulate a replacement project's issue rail");
const relinkLibraryWorkflow = app.slice(
  app.indexOf("const relinkMediaLibraryAsset = async"),
  app.indexOf("const videoLayerHasMonitorableAudio", app.indexOf("const relinkMediaLibraryAsset = async")),
);
assert.doesNotMatch(relinkLibraryWorkflow, /reserveAssets/, "Relink does not supersede availability by invocation reservation");
assert.match(relinkLibraryWorkflow, /const statusLease = mediaLibraryStatusLease\.begin\(\);[\s\S]*?relinkMediaAsset\(assetId, \(message\) => \{[\s\S]*?mediaLibraryStatusLease\.isCurrent\(statusLease\)/, "Relink reserves the same shared status lease and passes a guarded publisher into the controller");
assert.match(relinkLibraryWorkflow, /isProjectAuthorityIdentityCurrent\(result\.terminalAuthority\)[\s\S]*?mediaAssetAvailabilityRowAuthority\.shouldClearAfterRelink\(assetId, result\.terminalAuthority\)/, "Relink clears only a row not verified at its exact terminal E/R/H");
const relinkController = controller.slice(
  controller.indexOf("const relinkMediaAsset = async"),
  controller.indexOf("const setVideoLayerIsfEffect", controller.indexOf("const relinkMediaAsset = async")),
);
assert.match(relinkController, /setRelinkMessage: \(message: string\) => unknown = options\.setMessage/, "controller accepts an App-scoped guarded Relink status publisher");
assert.match(relinkController, /setRelinkMessageIfAuthorityCurrent[\s\S]*?setRelinkMessage\(message\)/, "Relink's authority guard publishes through the App-scoped status lease");
assert.doesNotMatch(relinkController, /options\.setMessage\(/, "all Relink cancel, no-op, decision, outcome, and error statuses use the guarded publisher");
assert.match(mediaAuthority, /assertStartAuthorityMatchesPreflight\("Start"[\s\S]*?options\.expectedAuthority/, "ordinary media Start rejects a post-preflight E/R/H change before hashing");
assert.match(mediaAuthority, /assertStartAuthorityMatchesPreflight\("Relink Start"[\s\S]*?options\.expectedAuthority/, "Relink Start rejects a post-preflight E/R/H change before hashing");
assert.match(mediaAuthority, /assertStartAuthorityMatchesPreflight\("Availability Start"[\s\S]*?options\.expectedAuthority/, "Availability Start rejects a post-preflight E/R/H change before inspection");
assert.match(app, /const persistProjectControlMappings = \(\): Promise<ProjectControlMappingsPersistResult>/, "mapping persist returns explicit acknowledgement provenance");
assert.match(app, /kind: "untrusted"[\s\S]*?get_project_control_mappings/, "CAS recovery/read is explicitly non-provenance for media preflight");
assert.match(app, /mediaAssetMappingPreflightProvenance\([\s\S]*?flushed\.ownAcknowledgements/, "media preflight accepts only unchanged or exact own-ACK provenance");
assert.ok(
  (controller.match(/isProjectAuthorityCurrent\(staged\.terminalAuthority\)/g) ?? []).length >= 3,
  "File\/Still, catalog Import, and Relink recheck the live terminal token immediately before UI effects",
);
assert.doesNotMatch(controller, /commit_prepared_(video_file_layer|still_image_layer|media_assets)"/, "production controller has no old staged commit route");
assert.match(app, /commitCommand: "commit_prepared_bootstrap_vj_show_authoritative"/, "first-run uses authoritative atomic Bootstrap");
const firstRun = app.slice(
  app.indexOf("const createFirstRunVjShow = async"),
  app.indexOf("const videoThumbnailSourceSignature", app.indexOf("const createFirstRunVjShow = async")),
);
assert.match(
  firstRun,
  /const initiatingAuthority = captureProjectAuthorityIdentity\(\);[\s\S]*?sideEffectAuthority = preparedStart\.authority;[\s\S]*?select_video_source_files[\s\S]*?if \(!sideEffectsAreCurrent\(\)\) return;/,
  "first-run picker/hash route binds post-preflight E/R/H and suppresses all B status/error/cancel effects after C applies",
);
assert.ok(
  firstRun.indexOf("preflightAndBeginMediaAssetOperation") < firstRun.indexOf("select_video_source_files"),
  "Bootstrap flushes and registers its operation before opening the picker",
);
assert.ok(
  (firstRun.match(/if \(!terminalIsCurrent\(\)\)/g) ?? []).length >= 3,
  "Bootstrap rechecks B after snapshot/Preview awaits before applying B IDs or messages to C",
);
assert.match(
  firstRun,
  /stageVjPreviewLayer\(firstLayerId, staged\.terminalAuthority\)/,
  "Bootstrap sends exact terminal authority into the Preview staging side effect",
);
const previewTransport = app.slice(
  app.indexOf("const runVjPreviewTransportCommand ="),
  app.indexOf("const clearVjPreview =", app.indexOf("const runVjPreviewTransportCommand =")),
);
assert.match(
  previewTransport,
  /resultIsCurrent[\s\S]*?await invoke[\s\S]*?resultIsCurrent[\s\S]*?applyVjPreviewTransport/,
  "stale Preview replies are discarded before local transport state is applied",
);
assert.match(mediaAuthority, /get_media_asset_operation_terminal_result/, "helper resolves a lost reply from the exact terminal receipt");
assert.match(mediaAuthority, /shape_fingerprint !== expectedShapeFingerprint/, "terminal recovery validates semantic shape");
assert.doesNotMatch(mediaAuthority, /projectTransactionId|begin_project_transaction|commit_project_transaction/, "media helper owns no renderer project ticket");

console.log("media asset authoritative frontend checks passed (phase order, reply loss/query, cancellation CAS, authority continuity, operator classification, direct paired application, snapshot empty-catalog normalization)");
