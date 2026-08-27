import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const read = (relativePath) => readFile(new URL(`../${relativePath}`, import.meta.url), "utf8");
const hash = (character) => character.repeat(64);
const lease = { lease_id: "lease-0000000000000001", generation: 1 };
const fence = {
  process_incarnation: 1,
  session_incarnation: 2,
  project_epoch: 3,
  project_revision: 4,
  project_checkpoint_hash: hash("c"),
  project_publication_generation: 5,
  output_epoch: 6,
  output_generation: 7,
  safety_blackout_epoch: 8,
  safety_blackout_generation: 9,
};
const fenceAfter = {
  ...fence,
  project_revision: 5,
  project_checkpoint_hash: hash("d"),
  project_publication_generation: 6,
};
const snapshotFor = (compositionId = 1) => ({
  video: {
    outputs: [{ id: 42, composition_id: compositionId, label: "LED" }],
    compositions: [{ id: 1, label: "Main" }, { id: 7, label: "Program" }],
  },
});
const outputControllerSource = await read("src/outputControlController.ts");
const outputControllerUrl = `data:text/javascript;base64,${Buffer.from(ts.transpileModule(
  outputControllerSource,
  { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022, importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove } },
).outputText).toString("base64")}`;
const routingSource = await read("src/videoOutputRoutingController.ts");
const [protocolSource, engineSource, mainSource, runtimeSource, controlPlaneSource, manifestSource, invokeSource, appSource] = await Promise.all([
  read("../crates/protocol/src/control_plane_command.rs"),
  read("../crates/engine/src/lib.rs"),
  read("src-tauri/src/main.rs"),
  read("src-tauri/src/control_plane_runtime.rs"),
  read("src-tauri/src/control_plane.rs"),
  read("src/tauri-invoke-manifest.json"),
  read("src/tauriInvokeCommands.ts"),
  read("src/App.tsx"),
]);
assert.match(protocolSource, /OUTPUT_VIDEO_COMPOSITION_ASSIGN_OPERATION_ID:\s*&str\s*=\s*\n?\s*"syndocal\.output\.video\.composition\.assign\.v2"/);
assert.match(protocolSource, /AssignVideoOutputComposition\s*\{\s*output_id:\s*u64,\s*composition_id:\s*u64,\s*lease:/s);
assert.match(protocolSource, /output\.push\(11\);[\s\S]*append_u64\(output, \*output_id\);[\s\S]*append_u64\(output, \*composition_id\);/);
assert.match(engineSource, /pub fn set_video_output_routing_published\([\s\S]*submit_authoritative_snapshot_mutation/s);
assert.match(engineSource, /SetVideoOutputRoutingPublished \{[\s\S]*expected_from_composition_id:[\s\S]*PendingCommandRollback::RestoreMediaAssetTransaction/s);
assert.match(engineSource, /output\.summary\.composition_id != expected_from_composition_id/);
assert.doesNotMatch(engineSource, /\bSetVideoOutputRouting\s*\{/);
const assignmentCore = mainSource.slice(
  mainSource.indexOf("fn assign_video_output_composition_with_output_control_fence("),
  mainSource.indexOf("#[cfg(test)]\nmacro_rules! display_output_control_request"),
);
assert.notEqual(assignmentCore.length, 0, "native assignment core must exist");
assert.match(assignmentCore, /video_output_composition_assignment_candidate_snapshot/);
assert.match(assignmentCore, /set_video_output_routing_published\(/);
assert.match(assignmentCore, /SnapshotPublicationFailure::Definitive/);
assert.match(assignmentCore, /SnapshotPublicationFailure::Indeterminate/);
assert.match(assignmentCore, /project_transaction_publication_faulted/);
assert.match(assignmentCore, /actual_hash == expected_candidate_checkpoint_hash/);
assert.match(runtimeSource, /OutputControlActionV2::AssignVideoOutputComposition[\s\S]*assign_video_output_composition_with_output_control_fence/s);
assert.match(runtimeSource, /OutputControlActionV2::AssignVideoOutputComposition[\s\S]*output_action_requires_native_danger_confirmation/s);
assert.match(controlPlaneSource, /"assign_video_output_composition_v2"\s*=>\s*\{[\s\S]*AssignVideoOutputComposition/s);
const retiredRoute = ["set", "video", "output", "routing"].join("_");
const registeredTauriRoutes = [...mainSource.matchAll(/#\[tauri::command\]\r?\n(?:async )?fn ([a-z0-9_]+)\(/g)]
  .map((match) => match[1]);
assert.equal(registeredTauriRoutes.includes(retiredRoute), false, "retired direct route must not be Tauri registered");
assert.equal(JSON.parse(manifestSource).includes(retiredRoute), false, "retired direct route must not be frontend manifest-visible");
assert.equal(invokeSource.includes(`"${retiredRoute}"`), false, "retired direct route must not be frontend-invokable");
assert.equal(appSource.includes(`"${retiredRoute}"`), false, "retired direct route must not enter App mutation routing");
assert.equal(controlPlaneSource.includes(`"${retiredRoute}"`), false, "retired direct route must have no control-plane classification");
assert.match(routingSource, /refreshSnapshotAndVideoOutputRenderPlans/);
assert.match(routingSource, /refreshed\.composition_id !== compositionId/);
assert.doesNotMatch(routingSource, /setSnapshot/);
const routingModuleSource = routingSource.replace('from "./outputControlController"', `from "${outputControllerUrl}"`);
const routing = await import(`data:text/javascript;base64,${Buffer.from(ts.transpileModule(
  routingModuleSource,
  { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022, importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove } },
).outputText).toString("base64")}`);

const successfulReceipt = (request, outcome = "applied") => ({
  type: "receipt",
  receipt: {
    operation_id: request.operation_id,
    request_id: request.request_id,
    shape_sha256: hash("a"),
    argument_fingerprint: hash("b"),
    audit_sequence: 1,
    fence_before: structuredClone(fence),
    fence_after: outcome === "no_op" ? structuredClone(fence) : structuredClone(fenceAfter),
    outcome,
    lease_result: {
      authority: lease,
      resources: ["lighting", "video"],
      phase: "held_active",
      outcome: "authorized",
      audit_sequence: 1,
      changes: [{
        lease_id: lease.lease_id,
        before_generation: 1,
        after_generation: 1,
        before_resources: ["lighting", "video"],
        after_resources: ["lighting", "video"],
        before_phase: "held_active",
        after_phase: "held_active",
      }],
    },
  },
});

const run = async ({ refreshedCompositionId = 7, outcome = "applied" } = {}) => {
  let current = snapshotFor(1);
  const messages = [];
  const busy = [];
  const calls = [];
  const invoke = async (command, args) => {
    calls.push({ command, args });
    if (command === "query_output_lease_authority_v1") {
      return {
        operation_id: "syndocal.output.lease.authority.query.v1",
        statuses: [{ status: "held_active", authority: lease, resources: ["lighting", "video"] }],
      };
    }
    if (command === "query_output_control_authority_v1") {
      return { operation_id: "syndocal.query.output.control.authority.v1", fence: structuredClone(fence) };
    }
    assert.equal(command, "assign_video_output_composition_v2");
    assert.deepEqual(args.request.action, {
      kind: "assign_video_output_composition",
      output_id: 42,
      composition_id: 7,
      lease,
    });
    return successfulReceipt(args.request, outcome);
  };
  const controller = routing.createVideoOutputRoutingController({
    invoke,
    snapshot: () => current,
    refreshSnapshotAndVideoOutputRenderPlans: async () => { current = snapshotFor(refreshedCompositionId); },
    setMessage: (message) => messages.push(message),
    setBusy: (outputId, value) => busy.push([outputId, value]),
  });
  return { result: await controller.assign(42, 7), calls, messages, busy };
};

const applied = await run();
assert.equal(applied.result, true);
assert.deepEqual(applied.calls.map((call) => call.command), [
  "query_output_lease_authority_v1",
  "query_output_control_authority_v1",
  "query_output_lease_authority_v1",
  "assign_video_output_composition_v2",
]);
assert.deepEqual(applied.busy, [[42, true], [42, false]]);
assert.match(applied.messages.at(-1), /authoritative route verified/);
assert.equal(applied.calls.some((call) => call.command === retiredRoute), false);

const noOp = await run({ refreshedCompositionId: 7, outcome: "no_op" });
assert.equal(noOp.result, true, "a terminal no-op must still refresh and verify the exact route");

const mismatch = await run({ refreshedCompositionId: 1 });
assert.equal(mismatch.result, false, "a receipt without an exact refreshed route is not success");
assert.match(mismatch.messages.at(-1), /route state is unknown/);

let staleInvokes = 0;
const stale = routing.createVideoOutputRoutingController({
  invoke: async () => { staleInvokes += 1; },
  snapshot: () => snapshotFor(1),
  refreshSnapshotAndVideoOutputRenderPlans: async () => undefined,
  setMessage: () => undefined,
  setBusy: () => undefined,
});
assert.equal(await stale.assign(42, 999), false);
assert.equal(staleInvokes, 0, "a stale composition must fail before any authority or mutation invoke");

let concurrentSnapshot = snapshotFor(1);
let resolveMutation;
let concurrentMutationCalls = 0;
const concurrentBusy = [];
const concurrent = routing.createVideoOutputRoutingController({
  invoke: async (command, args) => {
    if (command === "query_output_lease_authority_v1") {
      return {
        operation_id: "syndocal.output.lease.authority.query.v1",
        statuses: [{ status: "held_active", authority: lease, resources: ["lighting", "video"] }],
      };
    }
    if (command === "query_output_control_authority_v1") {
      return { operation_id: "syndocal.query.output.control.authority.v1", fence: structuredClone(fence) };
    }
    assert.equal(command, "assign_video_output_composition_v2");
    concurrentMutationCalls += 1;
    return new Promise((resolve) => { resolveMutation = () => resolve(successfulReceipt(args.request)); });
  },
  snapshot: () => concurrentSnapshot,
  refreshSnapshotAndVideoOutputRenderPlans: async () => { concurrentSnapshot = snapshotFor(7); },
  setMessage: () => undefined,
  setBusy: (outputId, value) => concurrentBusy.push([outputId, value]),
});
const firstConcurrent = concurrent.assign(42, 7);
await new Promise((resolve) => setTimeout(resolve, 0));
assert.equal(await concurrent.assign(42, 7), false, "a second route click must be rejected while the first is in flight");
assert.equal(concurrentMutationCalls, 1, "one output route has exactly one in-flight mutation");
assert.deepEqual(concurrentBusy, [[42, true]]);
assert.ok(resolveMutation, "the first route mutation must be pending before it can be released");
resolveMutation();
assert.equal(await firstConcurrent, true);
assert.deepEqual(concurrentBusy, [[42, true], [42, false]]);

console.log("video output routing R4 contract: PASS");
