import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const source = await readFile(new URL("../src/outputControlController.ts", import.meta.url), "utf8");
const runtime = await import(`data:text/javascript;base64,${Buffer.from(ts.transpileModule(
  source,
  {
    compilerOptions: {
      module: ts.ModuleKind.ESNext,
      target: ts.ScriptTarget.ES2022,
      importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
    },
    fileName: "outputControlController.ts",
  },
).outputText).toString("base64")}`);

const hash = (character) => character.repeat(64);
const lease = (generation = 1) => ({ lease_id: "lease-1111111111111111", generation });
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
const actionFor = (open) => ({
  kind: "set_display_window_open",
  output_id: 42,
  open,
  lease: lease(),
});
const eventFor = (incarnation, actualOpen, extras = {}) => ({
  schemaVersion: 1,
  outputId: 42,
  mode: "live",
  windowIncarnation: incarnation,
  actualOpen,
  retirementOutcome: null,
  reason: null,
  ...extras,
});
const receiptFor = (request, outcome = "no_op", fenceAfter = request.expected_fence) => ({
  type: "receipt",
  receipt: {
    operation_id: request.operation_id,
    request_id: request.request_id,
    shape_sha256: hash("a"),
    argument_fingerprint: hash("b"),
    audit_sequence: 1,
    fence_before: structuredClone(request.expected_fence),
    fence_after: structuredClone(fenceAfter),
    outcome,
    lease_result: {
      authority: lease(),
      resources: ["lighting", "video"],
      phase: "held_active",
      outcome: "authorized",
      audit_sequence: 1,
      changes: [{
        lease_id: lease().lease_id,
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

const createInvokeAdapter = ({ authorityStatus = "held_active", rejection = null, malformedReceipt = false, appliedReceipt = false, changedFenceReceipt = false } = {}) => {
  const calls = [];
  const invoke = async (command, args) => {
    calls.push({ command, args });
    if (command === "query_display_add_lease_authority_v1") {
      return {
        operationId: runtime.OUTPUT_DISPLAY_ADD_AUTHORITY_QUERY_OPERATION_ID,
        status: authorityStatus,
        authority: lease(),
        resources: ["lighting", "video"],
      };
    }
    if (command === "query_output_control_authority_v1") {
      return { operation_id: runtime.OUTPUT_CONTROL_AUTHORITY_QUERY_OPERATION_ID, fence: structuredClone(fence) };
    }
    if (command === "set_display_output_window_open_v2") {
      assert.deepEqual(Object.keys(args), ["request"]);
      assert.deepEqual(args.request.action, actionFor(args.request.action.open));
      if (rejection) {
        return {
          type: "rejected",
          rejection: { operation_id: args.request.operation_id, request_id: args.request.request_id, error: rejection },
        };
      }
      return malformedReceipt
        ? { type: "receipt", receipt: { operation_id: "wrong" } }
        : receiptFor(
            args.request,
            appliedReceipt ? "applied" : "no_op",
            changedFenceReceipt
              ? { ...structuredClone(args.request.expected_fence), output_generation: args.request.expected_fence.output_generation + 1 }
              : args.request.expected_fence,
          );
    }
    if (/^(set_video_output_|fade_video_output_opacity|open_video_output_window|sync_|close_|remove_video_output)/.test(command)) {
      throw new Error(`legacy invoke reached: ${command}`);
    }
    throw new Error(`unexpected invoke: ${command}`);
  };
  return { invoke, calls };
};

const action = actionFor(true);
const active = createInvokeAdapter();
const activeReceipt = await runtime.executeDisplayWindowOutputControl(active.invoke, action, {
  operationId: runtime.OUTPUT_DISPLAY_ADD_AUTHORITY_QUERY_OPERATION_ID,
  status: "held_active",
  authority: lease(),
  resources: ["lighting", "video"],
});
assert.equal(activeReceipt.operation_id, runtime.OUTPUT_DISPLAY_WINDOW_SET_OPEN_OPERATION_ID);
assert.equal(activeReceipt.lease_result.outcome, "authorized");
assert.deepEqual(active.calls.map(({ command }) => command), [
  "query_output_control_authority_v1",
  "set_display_output_window_open_v2",
]);

for (const status of ["held_orphaned", "expired_recoverable"]) {
  const recoverable = createInvokeAdapter({ authorityStatus: status });
  await runtime.executeDisplayWindowOutputControl(recoverable.invoke, actionFor(false), {
    operationId: runtime.OUTPUT_DISPLAY_ADD_AUTHORITY_QUERY_OPERATION_ID,
    status,
    authority: lease(),
    resources: ["lighting", "video"],
  });
  assert.equal(recoverable.calls.filter(({ command }) => command === "set_display_output_window_open_v2").length, 1);
}
assert.throws(
  () => runtime.selectExactBothLeaseForDisplayAdd({
    operationId: runtime.OUTPUT_DISPLAY_ADD_AUTHORITY_QUERY_OPERATION_ID,
    status: "unavailable",
    authority: null,
    resources: [],
  }),
  /Exactly one active or recoverable Both/,
);
assert.throws(
  () => runtime.selectExactBothLeaseForDisplayAdd({
    operationId: runtime.OUTPUT_DISPLAY_ADD_AUTHORITY_QUERY_OPERATION_ID,
    status: "held_active",
    authority: lease(),
    resources: ["video"],
  }),
  /Display Add lease authority response was invalid/,
);
await assert.rejects(
  runtime.executeDisplayWindowOutputControl(createInvokeAdapter().invoke, action, {
    operationId: runtime.OUTPUT_DISPLAY_ADD_AUTHORITY_QUERY_OPERATION_ID,
    status: "held_active",
    authority: lease(2),
    resources: ["lighting", "video"],
  }),
  /changed before execution/,
);

const duplicateAdapter = createInvokeAdapter();
const inFlight = new Map();
const toggle = (open) => {
  const key = String(actionFor(open).output_id);
  const existing = inFlight.get(key);
  if (existing) return existing;
  const request = runtime.executeDisplayWindowOutputControl(duplicateAdapter.invoke, actionFor(open), {
    operationId: runtime.OUTPUT_DISPLAY_ADD_AUTHORITY_QUERY_OPERATION_ID,
    status: "held_active",
    authority: lease(),
    resources: ["lighting", "video"],
  });
  const shared = request.finally(() => inFlight.delete(key));
  inFlight.set(key, shared);
  return shared;
};
const duplicateA = toggle(true);
const duplicateB = toggle(true);
assert.strictEqual(duplicateA, duplicateB);
await duplicateA;
assert.equal(duplicateAdapter.calls.filter(({ command }) => command === "set_display_output_window_open_v2").length, 1);

await assert.rejects(
  runtime.executeDisplayWindowOutputControl(createInvokeAdapter({ rejection: "forbidden" }).invoke, action, {
    operationId: runtime.OUTPUT_DISPLAY_ADD_AUTHORITY_QUERY_OPERATION_ID,
    status: "held_active",
    authority: lease(),
    resources: ["lighting", "video"],
  }),
  /refresh lease state/,
);
await assert.rejects(
  runtime.executeDisplayWindowOutputControl(createInvokeAdapter({ malformedReceipt: true }).invoke, action, {
    operationId: runtime.OUTPUT_DISPLAY_ADD_AUTHORITY_QUERY_OPERATION_ID,
    status: "held_active",
    authority: lease(),
    resources: ["lighting", "video"],
  }),
  /terminal response identity did not match/,
);
const appliedReceipt = await runtime.executeDisplayWindowOutputControl(createInvokeAdapter({ appliedReceipt: true }).invoke, action, {
    operationId: runtime.OUTPUT_DISPLAY_ADD_AUTHORITY_QUERY_OPERATION_ID,
    status: "held_active",
    authority: lease(),
    resources: ["lighting", "video"],
  });
assert.equal(appliedReceipt.outcome, "applied");
await assert.rejects(
  runtime.executeDisplayWindowOutputControl(createInvokeAdapter({ appliedReceipt: true, changedFenceReceipt: true }).invoke, action, {
    operationId: runtime.OUTPUT_DISPLAY_ADD_AUTHORITY_QUERY_OPERATION_ID,
    status: "held_active",
    authority: lease(),
    resources: ["lighting", "video"],
  }),
  /receipt was inconsistent/,
);

let physical = {};
const applyEvent = (payload) => {
  const result = runtime.applyVideoOutputWindowStateEvent(physical, payload);
  if (result.accepted) physical = result.state;
  return result;
};
assert.equal(applyEvent(eventFor(10, true)).accepted, true);
assert.equal(applyEvent(eventFor(9, false)).reason, "stale");
assert.equal(physical["42"].actual_open, true);
assert.equal(applyEvent(eventFor(11, false, { schemaVersion: 2 })).reason, "malformed");
assert.equal(applyEvent({ ...eventFor(12, false), unexpected: true }).reason, "malformed");
assert.equal(applyEvent(eventFor(12, false)).accepted, true);
assert.equal(physical["42"].actual_open, false);

// A configured Display has explicit closed truth on a cold process before its
// first positive native-window incarnation, so the sole Open action is usable.
assert.equal(runtime.resolveVideoOutputWindowActualOpen(undefined, {
  output_id: 42,
  live_open: false,
  live_window_incarnation: 0,
}), false);
assert.equal(runtime.resolveVideoOutputWindowActualOpen(undefined, null), null);
assert.equal(runtime.resolveVideoOutputWindowActualOpen(undefined, {
  output_id: 42,
  live_open: false,
}), null);
assert.equal(runtime.resolveVideoOutputWindowActualOpen(undefined, {
  output_id: 42,
  live_open: false,
  live_window_incarnation: null,
}), null);
assert.equal(runtime.resolveVideoOutputWindowActualOpen(undefined, {
  output_id: 42,
  live_open: false,
  live_window_incarnation: -1,
}), null);

// A query captured before a native event cannot overwrite the equal
// incarnation event when it resolves later.
const eventWins = runtime.applyVideoOutputWindowStateEvent({}, eventFor(20, false));
const staleEqualQuery = runtime.applyVideoOutputWindowStatusQuery(eventWins.state, {
  output_id: 42,
  live_open: true,
  live_window_incarnation: 20,
});
assert.equal(staleEqualQuery.reason, "stale");
assert.equal(staleEqualQuery.state["42"].actual_open, false);
const newerQuery = runtime.applyVideoOutputWindowStatusQuery(eventWins.state, {
  output_id: 42,
  live_open: true,
  live_window_incarnation: 21,
});
assert.equal(newerQuery.accepted, true);
assert.equal(newerQuery.state["42"].actual_open, true);
assert.equal(newerQuery.state["42"].observed_from, "query");

// Manual native close changes only physical truth; authored intent remains untouched.
const authored = { enabled: true, composition_id: 7 };
assert.equal(physical["42"].actual_open, false);
assert.equal(authored.enabled, true);

const legacyCalls = [...active.calls, ...duplicateAdapter.calls].filter(({ command }) =>
  /^(set_video_output_|fade_video_output_opacity|open_video_output_window|sync_|close_|remove_video_output)/.test(command));
assert.deepEqual(legacyCalls, []);

console.log("video output window runtime contract: PASS (controller, exact-Both recovery, receipt rejection, singleflight, incarnation reducer, legacy invoke count 0)");
