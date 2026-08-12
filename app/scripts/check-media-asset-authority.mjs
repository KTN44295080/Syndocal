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

const authority = await importTsModule("../src/mediaAssetAuthority.ts");
const controller = await readFile(new URL("../src/createVideoRuntimeController.ts", import.meta.url), "utf8");
const app = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");

const allocatedRequestIdA = authority.allocateMediaAssetRequestId();
const allocatedRequestIdB = authority.allocateMediaAssetRequestId();
assert.ok(Number.isSafeInteger(allocatedRequestIdA) && allocatedRequestIdA > 0, "client request ID stays a positive exact integer");
assert.equal(allocatedRequestIdB, allocatedRequestIdA + 1, "same renderer allocates monotonic request IDs");

const report = (overrides = {}) => ({
  request_id: 4101,
  operation_generation: 73,
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

// Prepare and full-byte finalization occur before the fake generic mutation
// wrapper reaches its Begin/Commit boundary. The helper itself deliberately
// owns neither history ticket nor transaction completion.
{
  const order = [];
  const calls = [];
  const invoke = async (command, args = {}) => {
    calls.push({ command, args });
    if (command === "prepare_local_media_assets") {
      order.push("prepare");
      return report();
    }
    if (command === "finalize_prepared_media_assets") {
      order.push("finalize");
      return report();
    }
    if (command === "commit_prepared_media_assets") {
      // This models App's generic project-mutation wrapper: Begin happens only
      // when the staged command is invoked, after finalization has settled.
      order.push("begin");
      assert.equal(typeof args.__onProjectTransactionOpened, "function", "staged commit receives the private ticket-open callback");
      args.__onProjectTransactionOpened();
      order.push("ticket-opened");
      order.push("commit");
      return report({ prepared_import_token: null, imported: 1, entries: [{ input_index: 0, path: "C:/show/a.mov", status: "imported", asset_id: 12, message: null }] });
    }
    throw new Error(`unexpected command ${command}`);
  };
  const result = await authority.prepareFinalizeAndCommitMediaAssets({
    invoke,
    kind: "File",
    paths: ["C:/show/a.mov"],
    expectedEpoch: 44,
    ownerId: "renderer:test",
    requestId: 4101,
    commitCommand: "commit_prepared_media_assets",
  });
  assert.deepEqual(order, ["prepare", "finalize", "begin", "ticket-opened", "commit"], "Prepare → Finalize must precede generic Begin/ticket/Commit");
  assert.equal(calls[0].args.expectedEpoch, 44, "Prepare receives picker-era epoch");
  assert.equal(calls[1].args.expectedEpoch, 44, "Finalize receives the same epoch");
  assert.equal(calls[2].args.__expectedProjectEpoch, 44, "only staged Commit enters generic ticket wrapper with the same epoch");
  assert.equal(result.committed.imported, 1, "catalog commit returns the definitive rich report");
}

// This harness makes the generic invocation reject after an AbortSignal but
// before its ticket callback. It proves only that the helper submits a
// best-effort cancellation request for the known operation identity in that
// rejection path. Production App does not automatically stop at this point;
// backend Cancelable→Admitted linearization/receipt is required to guarantee
// whether cancellation or publication wins.
{
  const abort = new AbortController();
  const calls = [];
  const invoke = async (command, args = {}) => {
    calls.push({ command, args });
    if (command === "prepare_local_media_assets" || command === "finalize_prepared_media_assets") return report();
    if (command === "commit_prepared_media_assets") {
      assert.equal(typeof args.__onProjectTransactionOpened, "function");
      queueMicrotask(() => abort.abort());
      await Promise.resolve(); // simulate App's flush before Begin returns a ticket
      throw new Error("operation cancelled before ticket admission");
    }
    if (command === "cancel_media_asset_operation") return true;
    throw new Error(`unexpected command ${command}`);
  };
  await assert.rejects(
    authority.prepareFinalizeAndCommitMediaAssets({
      invoke,
      kind: "File",
      paths: ["C:/show/a.mov"],
      expectedEpoch: 44,
      ownerId: "renderer:pre-begin-abort",
      requestId: 4107,
      signal: abort.signal,
      commitCommand: "commit_prepared_media_assets",
    }),
    /cancelled before ticket admission/,
  );
  assert.deepEqual(calls.map(({ command }) => command), [
    "prepare_local_media_assets",
    "finalize_prepared_media_assets",
    "commit_prepared_media_assets",
    "cancel_media_asset_operation",
  ], "pre-ticket rejection submits a best-effort cancellation request before the callback");
}

// Once the wrapper invokes the callback, the helper classifies the operation
// as post-ticket and does not send a second signal-driven cancel request. This
// mock result is not proof of the real backend's publication outcome.
{
  const abort = new AbortController();
  const calls = [];
  const invoke = async (command, args = {}) => {
    calls.push({ command, args });
    if (command === "prepare_local_media_assets" || command === "finalize_prepared_media_assets") return report();
    if (command === "commit_prepared_media_assets") {
      args.__onProjectTransactionOpened();
      queueMicrotask(() => abort.abort());
      await Promise.resolve();
      return report({ prepared_import_token: null, imported: 1 });
    }
    if (command === "cancel_media_asset_operation") throw new Error("post-ticket cancel must not run");
    throw new Error(`unexpected command ${command}`);
  };
  const result = await authority.prepareFinalizeAndCommitMediaAssets({
    invoke,
    kind: "File",
    paths: ["C:/show/a.mov"],
    expectedEpoch: 44,
    ownerId: "renderer:post-begin-abort",
    requestId: 4108,
    signal: abort.signal,
    commitCommand: "commit_prepared_media_assets",
  });
  assert.equal(result.committed.imported, 1, "the mock wrapper result is returned after callback classification");
  assert.deepEqual(calls.map(({ command }) => command), [
    "prepare_local_media_assets",
    "finalize_prepared_media_assets",
    "commit_prepared_media_assets",
  ], "post-ticket abort does not send a second frontend cancellation request");
}

// Once Prepare exposes the server generation, cancellation requests carry that
// specific request/generation/owner identity. In this helper-local abort path,
// Finalize and the mutation wrapper are not entered.
{
  const abort = new AbortController();
  const calls = [];
  const invoke = async (command, args = {}) => {
    calls.push({ command, args });
    if (command === "prepare_local_media_assets") {
      queueMicrotask(() => abort.abort());
      return report();
    }
    if (command === "cancel_media_asset_operation") return true;
    throw new Error(`unexpected command ${command}`);
  };
  await assert.rejects(
    authority.prepareFinalizeAndCommitMediaAssets({
      invoke,
      kind: "File",
      paths: ["C:/show/a.mov"],
      expectedEpoch: 44,
      ownerId: "renderer:abort",
      requestId: 4102,
      signal: abort.signal,
      commitCommand: "commit_prepared_media_assets",
    }),
    { name: "AbortError" },
  );
  assert.deepEqual(calls.map(({ command }) => command), [
    "prepare_local_media_assets",
    "cancel_media_asset_operation",
  ], "helper-local abort stops before Finalize and ticket wrapper invocation");
  assert.deepEqual(calls[1].args, {
    requestId: 4101,
    operationGeneration: 73,
    ownerId: "renderer:abort",
  }, "known cancellation request carries the specific backend operation identity");
}

// Any pre-ticket failure after Prepare still submits best-effort cleanup for
// the known server record.
{
  const calls = [];
  const invoke = async (command, args = {}) => {
    calls.push({ command, args });
    if (command === "prepare_local_media_assets") return report();
    if (command === "finalize_prepared_media_assets") throw new Error("replacement changed");
    if (command === "cancel_media_asset_operation") return true;
    throw new Error(`unexpected command ${command}`);
  };
  await assert.rejects(
    authority.prepareFinalizeAndCommitMediaAssets({
      invoke,
      kind: "StillImage",
      paths: ["C:/show/a.png"],
      expectedEpoch: 44,
      ownerId: "renderer:failure",
      requestId: 4103,
      commitCommand: "commit_prepared_media_assets",
    }),
    /replacement changed/,
  );
  assert.deepEqual(calls.map(({ command }) => command), [
    "prepare_local_media_assets",
    "finalize_prepared_media_assets",
    "cancel_media_asset_operation",
  ], "finalization failure submits cleanup before the ticket wrapper is invoked");
  assert.deepEqual(calls[2].args, {
    requestId: 4101,
    operationGeneration: 73,
    ownerId: "renderer:failure",
  }, "failure cleanup retains the known generation");
}

// A staged-commit error can occur before or after App opens its short ticket.
// The helper still submits best-effort cleanup for the known identity; it never
// reports a post-ticket abort as a successful cancellation.
{
  const calls = [];
  const invoke = async (command, args = {}) => {
    calls.push({ command, args });
    if (command === "prepare_local_media_assets") return report();
    if (command === "finalize_prepared_media_assets") return report();
    if (command === "commit_prepared_media_assets") throw new Error("engine rejected staged catalog");
    if (command === "cancel_media_asset_operation") return false;
    throw new Error(`unexpected command ${command}`);
  };
  await assert.rejects(
    authority.prepareFinalizeAndCommitMediaAssets({
      invoke,
      kind: "File",
      paths: ["C:/show/a.mov"],
      expectedEpoch: 44,
      ownerId: "renderer:commit-error",
      requestId: 4105,
      commitCommand: "commit_prepared_media_assets",
    }),
    /engine rejected staged catalog/,
  );
  assert.deepEqual(calls.map(({ command }) => command), [
    "prepare_local_media_assets",
    "finalize_prepared_media_assets",
    "commit_prepared_media_assets",
    "cancel_media_asset_operation",
  ], "commit failure still performs best-effort identity-targeted cleanup without claiming cancellation");
}

// A tokenless report has no server-held staged work and must not manufacture
// an empty catalog history entry.
{
  const calls = [];
  const invoke = async (command, args = {}) => {
    calls.push({ command, args });
    if (command === "prepare_local_media_assets") return report({ prepared_import_token: null, prepared: 0, failed: 1 });
    throw new Error(`unexpected command ${command}`);
  };
  const result = await authority.prepareFinalizeAndCommitMediaAssets({
    invoke,
    kind: "File",
    paths: ["C:/show/missing.mov"],
    expectedEpoch: 44,
    ownerId: "renderer:empty",
    requestId: 4104,
    commitCommand: "commit_prepared_media_assets",
  });
  assert.equal(result.committed, null, "tokenless Prepare result has no Commit");
  assert.deepEqual(calls.map(({ command }) => command), ["prepare_local_media_assets"]);
}

// First-run is intentionally stricter than ordinary catalog import: a mixed
// prepared/failed batch must submit cleanup for the known server operation
// before Finalize/Commit, never entering the partial first-run show route.
{
  const calls = [];
  const invoke = async (command, args = {}) => {
    calls.push({ command, args });
    if (command === "prepare_local_media_assets") return report({ prepared: 1, failed: 1 });
    if (command === "cancel_media_asset_operation") return true;
    throw new Error(`unexpected command ${command}`);
  };
  await assert.rejects(
    authority.prepareFinalizeAndCommitMediaAssets({
      invoke,
      kind: "File",
      paths: ["C:/show/valid.mov", "C:/show/missing.mov"],
      expectedEpoch: 44,
      ownerId: "renderer:first-run",
      requestId: 4106,
      commitCommand: "commit_prepared_bootstrap_vj_show",
      requireAllPrepared: true,
    }),
    /requires every selected media file to prepare successfully/,
  );
  assert.deepEqual(calls.map(({ command }) => command), [
    "prepare_local_media_assets",
    "cancel_media_asset_operation",
  ], "first-run mixed batch submits cleanup before Finalize/Commit");
}

// Production V01 semantics are catalog-only: the import entry point never
// reaches legacy layer-creating IPC, while explicit Add/first-run use staged
// compatibility commits and metadata refresh uses staged relink.
for (const legacy of [
  "add_video_file_layer",
  "add_still_image_layer",
  "add_local_media_layers",
  "bootstrap_vj_show",
  "refresh_video_layer_metadata",
]) {
  assert.equal(controller.includes(`\"${legacy}\"`), false, `controller must not call legacy ${legacy}`);
  assert.equal(app.includes(`\"${legacy}\"`), false, `App must not call legacy ${legacy}`);
}
assert.match(controller, /commitCommand:\s*"commit_prepared_media_assets"/, "batch Import Media is catalog-only");
assert.match(controller, /commit_prepared_video_file_layer/, "single File layer uses staged commit");
assert.match(controller, /commit_prepared_still_image_layer/, "single Still layer uses staged commit");
assert.match(controller, /prepareFinalizeAndCommitMediaAssetRelink/, "metadata refresh uses staged relink workflow");
assert.match(controller, /policy:\s*"RequireContentMatch"/, "metadata refresh stays fail-closed by default");
assert.match(controller, /media_asset_id/, "metadata refresh derives the catalog identity from the layer");
assert.match(app, /commitCommand:\s*"commit_prepared_bootstrap_vj_show"/, "first-run uses staged bootstrap commit");
assert.match(app, /requireAllPrepared:\s*true/, "first-run rejects partial prepared batches before any project ticket");
assert.match(app, /"commit_prepared_media_assets"/, "catalog staged commit is a generic project mutation");
assert.match(app, /"commit_prepared_media_asset_relink"/, "relink staged commit is a generic project mutation");
assert.equal(app.includes('"prepare_local_media_assets"'), false, "Prepare stays outside the generic mutation set");
assert.equal(app.includes('"finalize_prepared_media_assets"'), false, "Finalize stays outside the generic mutation set");
assert.equal(app.includes('"cancel_media_asset_operation"'), false, "Cancel stays outside the generic mutation set");
const privateCallbackRead = app.indexOf("const onProjectTransactionOpened = typeof args?.__onProjectTransactionOpened");
const privateCallbackStrip = app.indexOf("delete commandArgs.__onProjectTransactionOpened;", privateCallbackRead);
const beginTicket = app.indexOf('await tauriInvoke<ProjectTransactionTicket>("begin_project_transaction"', privateCallbackStrip);
const privateCallbackFire = app.indexOf("onProjectTransactionOpened?.();", beginTicket);
const stagedCommandInvoke = app.indexOf("const result = await tauriInvoke<T>(command", privateCallbackFire);
assert.ok(
  privateCallbackRead >= 0 && privateCallbackStrip > privateCallbackRead
    && beginTicket > privateCallbackStrip && privateCallbackFire > beginTicket
    && stagedCommandInvoke > privateCallbackFire,
  "private ticket-classification callback is stripped locally and fires after Begin returns, before staged command IPC",
);
const pickerEpochIndex = app.indexOf("const expectedEpoch = captureProjectAuthorityIdentity().project_epoch;", app.indexOf("const selectVideoSourceFile"));
const pickerInvokeIndex = app.indexOf('invoke<string | null>("select_video_source_file"', app.indexOf("const selectVideoSourceFile"));
const pickerStoreIndex = app.indexOf("setVideoSourceSelectionEpoch(expectedEpoch);", pickerInvokeIndex);
assert.ok(pickerEpochIndex >= 0 && pickerEpochIndex < pickerInvokeIndex, "single-source Browse captures epoch before picker");
assert.ok(pickerStoreIndex > pickerInvokeIndex, "successful Browse stores its one-shot epoch fence");
assert.match(controller, /consumeVideoSourceExpectedEpoch\(\)/, "single local layer consumes the pre-picker epoch");
assert.match(app, /onSetPath: setVideoPathFromOperator/, "manual path edits clear the one-shot picker epoch");

console.log("Media Asset authority contract ok: staged phase order, cancellation-request identity/classification, picker-era epoch fence, tokenless no-op, catalog-only import, staged first-run/layer/relink routes verified");
console.log("Limitation: Abort assertions prove frontend request sequencing only; backend Cancelable→Admitted linearization/receipt remains required to guarantee cancellation outcome before publication.");
