import assert from "node:assert/strict";
import { createHash } from "node:crypto";
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

const authority = await importTsModule("../src/mediaAssetAuthority.ts");
const liveSnapshotState = await importTsModule("../src/engineSnapshotLiveState.ts");
const controller = await readFile(new URL("../src/createVideoRuntimeController.ts", import.meta.url), "utf8");
const app = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
const mediaAuthority = await readFile(new URL("../src/mediaAssetAuthority.ts", import.meta.url), "utf8");

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

// A dirty mapping flush may abort already-running media work. The new
// operation must not be registered until that flush and its epoch fence settle.
{
  const older = new AbortController();
  let current = null;
  const order = [];
  const started = await authority.preflightAndBeginMediaAssetOperation(
    44,
    async (expectedEpoch) => {
      order.push("flush");
      assert.equal(current, null, "the initiating AbortController does not exist during mapping flush");
      older.abort();
      return expectedEpoch;
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
    })),
    /requires every selected media file/,
  );
  assert.equal(calls.some(({ command }) => command === "finalize_prepared_media_assets"), false);
  assert.equal(calls.some(({ command }) => command === "commit_prepared_bootstrap_vj_show_authoritative"), false);
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
assert.match(app, /const terminalRecovery = mediaAssetTerminalRecoveryCommands\.has\(command\);[\s\S]*?!terminalRecovery && !operatorCommandAllowed/, "Full Lock still permits exact terminal query/cancel cleanup");
const mediaStartFence = app.slice(
  app.indexOf("const prepareMediaAssetOperationStart = async"),
  app.indexOf("  createEffect(() =>", app.indexOf("const prepareMediaAssetOperationStart = async")),
);
assert.ok(
  mediaStartFence.indexOf("operatorCommandAllowed") < mediaStartFence.indexOf("flushProjectControlMappingsBeforeMutation"),
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
