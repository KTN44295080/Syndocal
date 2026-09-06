import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const read = (relativePath) => readFile(new URL(`../${relativePath}`, import.meta.url), "utf8");
const hash = (character) => character.repeat(64);
const lease = (generation = 1) => ({ lease_id: "lease-0000000000000001", generation });
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

const controllerSource = await read("src/outputControlController.ts");
const canonicalShowArtNetActionKind = "enable_show_art_net_loopback_route";
const canonicalShowSerialDmxEnableActionKind = "enable_show_serial_dmx_safety_blackout_route";
const canonicalShowSerialDmxStopActionKind = "stop_show_serial_dmx_safety_blackout_route";
const retiredShowArtNetActionKind = "enable_show_art" + "net_loopback_route";
const protocolCommandSource = await read("../crates/protocol/src/control_plane_command.rs");
const protocolCommandTestModuleOffset = protocolCommandSource.indexOf("#[cfg(test)]");
assert.notEqual(protocolCommandTestModuleOffset, -1, "protocol test boundary must remain explicit");
const protocolCommandProductionSource = protocolCommandSource.slice(
  0,
  protocolCommandTestModuleOffset,
);
const runtime = await import(`data:text/javascript;base64,${Buffer.from(ts.transpileModule(
  controllerSource,
  {
    compilerOptions: {
      module: ts.ModuleKind.ESNext,
      target: ts.ScriptTarget.ES2022,
      importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
    },
    fileName: "outputControlController.ts",
  },
).outputText).toString("base64")}`);

assert.match(
  controllerSource,
  /kind: "enable_show_art_net_loopback_route"/,
  "the Art-Net show route action must use the canonical serde discriminant",
);
assert.doesNotMatch(
  controllerSource,
  new RegExp(`kind:\\s*[\"']${retiredShowArtNetActionKind}[\"']`),
  "the retired Art-Net show route action discriminant must be absent from the controller",
);
assert.match(
  controllerSource,
  /kind: "enable_show_serial_dmx_safety_blackout_route"/,
  "the USB-DMX start action must use its canonical serde discriminant",
);
assert.match(
  controllerSource,
  /kind: "stop_show_serial_dmx_safety_blackout_route"/,
  "the USB-DMX stop action must use its canonical serde discriminant",
);

const resourcesFor = (action) => action.kind === "enable_output" || action.role === "both"
  || action.kind === "add_display" || action.kind === "assign_video_output_composition"
  || action.kind === canonicalShowArtNetActionKind || action.kind === "send_dsf2026_artnet_acceptance_probe"
  || action.kind === canonicalShowSerialDmxEnableActionKind
  || action.kind === canonicalShowSerialDmxStopActionKind
  || action.kind === "acknowledge_dsf2026_artnet_acceptance_probe_in_doubt"
  || action.kind === "enable_show_spout_outputs"
  ? ["lighting", "video"] : action.role === "lighting" ? ["lighting"] : ["video"];
const operationFor = (action) => ({
  enable_output: runtime.OUTPUT_ENABLE_OPERATION_ID,
  [canonicalShowArtNetActionKind]: runtime.OUTPUT_SHOW_ARTNET_LOOPBACK_ROUTE_ENABLE_OPERATION_ID,
  [canonicalShowSerialDmxEnableActionKind]: runtime.OUTPUT_SHOW_SERIAL_DMX_SAFETY_BLACKOUT_ROUTE_ENABLE_OPERATION_ID,
  [canonicalShowSerialDmxStopActionKind]: runtime.OUTPUT_SHOW_SERIAL_DMX_SAFETY_BLACKOUT_ROUTE_STOP_OPERATION_ID,
  send_dsf2026_artnet_acceptance_probe: runtime.OUTPUT_DSF2026_ARTNET_ACCEPTANCE_PROBE_OPERATION_ID,
  acknowledge_dsf2026_artnet_acceptance_probe_in_doubt:
    runtime.OUTPUT_DSF2026_ARTNET_ACCEPTANCE_PROBE_RECONCILE_OPERATION_ID,
  enable_show_spout_outputs: runtime.OUTPUT_SHOW_SPOUT_OUTPUTS_ENABLE_OPERATION_ID,
  reset_show_spout_outputs: runtime.OUTPUT_SHOW_SPOUT_OUTPUTS_RESET_OPERATION_ID,
  arm: runtime.OUTPUT_OWNERSHIP_ARM_OPERATION_ID,
  release_blackout: runtime.OUTPUT_BLACKOUT_RELEASE_OPERATION_ID,
  set_blackout: runtime.OUTPUT_BLACKOUT_SET_OPERATION_ID,
  take_over_standby: runtime.OUTPUT_STANDBY_TAKEOVER_OPERATION_ID,
  add_display: runtime.OUTPUT_DISPLAY_ADD_OPERATION_ID,
  assign_video_output_composition: runtime.OUTPUT_VIDEO_COMPOSITION_ASSIGN_OPERATION_ID,
  acquire_lease: runtime.OUTPUT_LEASE_ACQUIRE_OPERATION_ID,
  renew_lease: runtime.OUTPUT_LEASE_RENEW_OPERATION_ID,
  recover_lease: runtime.OUTPUT_LEASE_RECOVER_OPERATION_ID,
  relinquish_output_lease: runtime.OUTPUT_LEASE_RELINQUISH_OPERATION_ID,
  force_transfer_lease: runtime.OUTPUT_LEASE_FORCE_TRANSFER_OPERATION_ID,
})[action.kind];
const commandFor = (action) => ({
  enable_output: "enable_output_control_v2",
  [canonicalShowArtNetActionKind]: "enable_show_art_net_loopback_route_v1",
  [canonicalShowSerialDmxEnableActionKind]: "enable_show_serial_dmx_safety_blackout_route_v1",
  [canonicalShowSerialDmxStopActionKind]: "stop_show_serial_dmx_safety_blackout_route_v1",
  send_dsf2026_artnet_acceptance_probe: "send_dsf2026_artnet_acceptance_probe_v1",
  acknowledge_dsf2026_artnet_acceptance_probe_in_doubt:
    "acknowledge_dsf2026_artnet_acceptance_probe_in_doubt_v1",
  enable_show_spout_outputs: "enable_show_spout_outputs_v2",
  reset_show_spout_outputs: "reset_show_spout_outputs_v1",
  arm: "arm_output_control_v2",
  release_blackout: "release_blackout_output_control_v2",
  set_blackout: "set_blackout_output_control_v2",
  take_over_standby: "take_over_output_control_v2",
  add_display: "add_display_output_v2",
  assign_video_output_composition: "assign_video_output_composition_v2",
  acquire_lease: "acquire_output_lease_v2",
  renew_lease: "renew_output_lease_v2",
  recover_lease: "recover_output_lease_v2",
  relinquish_output_lease: "relinquish_output_lease_v2",
  force_transfer_lease: "force_transfer_output_lease_v2",
})[action.kind];

const enableAction = { kind: "enable_output" };
const showArtNetLoopbackRouteAction = { kind: canonicalShowArtNetActionKind, lease: lease() };
const showSerialDmxEnableAction = { kind: canonicalShowSerialDmxEnableActionKind, lease: lease() };
const showSerialDmxStopAction = { kind: canonicalShowSerialDmxStopActionKind, lease: lease() };
const dsf2026ArtNetAcceptanceProbeAction = { kind: "send_dsf2026_artnet_acceptance_probe", lease: lease() };
const dsf2026ArtNetAcceptanceProbeWireFixture = Object.freeze({
  kind: "send_dsf2026_artnet_acceptance_probe",
  lease: lease(),
});
const dsf2026ArtNetAcceptanceProbeReconcileAction = {
  kind: "acknowledge_dsf2026_artnet_acceptance_probe_in_doubt", lease: lease(),
};
assert.deepEqual(
  dsf2026ArtNetAcceptanceProbeAction,
  dsf2026ArtNetAcceptanceProbeWireFixture,
  "the TypeScript DSF2026 action must serialize to the exact Rust wire fixture",
);
assert.match(
  protocolCommandSource,
  /#\[serde\(rename = "send_dsf2026_artnet_acceptance_probe"\)\]\s*SendDsf2026ArtNetAcceptanceProbe/,
  "the Rust tagged-union discriminant must equal the TypeScript wire fixture",
);
assert.doesNotMatch(
  protocolCommandSource,
  /"send_dsf2026_art_net_acceptance_probe"/,
  "the legacy split Art-Net spelling must not remain on the wire",
);
const probeStatusCalls = [];
assert.deepEqual(
  await runtime.queryDsf2026ArtNetAcceptanceProbeStatus(async (command, args) => {
    probeStatusCalls.push({ command, args });
    return {
      operationId: runtime.OUTPUT_DSF2026_ARTNET_ACCEPTANCE_PROBE_STATUS_QUERY_OPERATION_ID,
      status: "consumed",
    };
  }),
  {
    operationId: runtime.OUTPUT_DSF2026_ARTNET_ACCEPTANCE_PROBE_STATUS_QUERY_OPERATION_ID,
    status: "consumed",
  },
  "the durable DSF2026 status query must accept only its fixed terminal shape",
);
assert.deepEqual(probeStatusCalls, [{
  command: "query_dsf2026_artnet_acceptance_probe_status_v1",
  args: undefined,
}]);
await assert.rejects(
  runtime.queryDsf2026ArtNetAcceptanceProbeStatus(async () => ({
    operationId: runtime.OUTPUT_DSF2026_ARTNET_ACCEPTANCE_PROBE_STATUS_QUERY_OPERATION_ID,
    status: "available",
    unexpected: true,
  })),
  /status was invalid/,
  "unknown durable probe status fields must fail closed",
);
const showSpoutOutputsAction = { kind: "enable_show_spout_outputs", lease: lease() };
const showSpoutResetAction = Object.freeze({ kind: "reset_show_spout_outputs" });
const ordinaryActions = [
  { kind: "arm", role: "lighting", lease: lease() },
  { kind: "release_blackout", lease: lease() },
  {
    kind: "take_over_standby",
    force: true,
    standby_session_id: "standby-session-1",
    standby_generation: 7,
    lease: lease(),
  },
  {
    kind: "add_display",
    spec: {
      label: "LED panel",
      monitor_identity: hash("d"),
      monitor_index: 1,
      width: 1920,
      height: 1080,
      fullscreen: true,
    },
    lease: lease(),
  },
  {
    kind: "assign_video_output_composition",
    output_id: 42,
    composition_id: 7,
    lease: lease(),
  },
  showArtNetLoopbackRouteAction,
  showSerialDmxEnableAction,
  showSerialDmxStopAction,
  dsf2026ArtNetAcceptanceProbeAction,
  dsf2026ArtNetAcceptanceProbeReconcileAction,
  showSpoutOutputsAction,
  ...["lighting", "video", "both"].flatMap(target => [false, true].map(enabled => ({ kind: "set_blackout", target, enabled, lease: lease() }))),
];
const lifecycleActions = [
  { kind: "acquire_lease", role: "both" },
  { kind: "renew_lease", lease: lease() },
  { kind: "recover_lease", lease: lease() },
  { kind: "relinquish_output_lease", lease: lease() },
  { kind: "force_transfer_lease", lease: lease() },
];

const queryFor = (action, state = "active") => {
  if (action.kind === "acquire_lease" || action.kind === "enable_output") {
    return {
      operation_id: runtime.OUTPUT_LEASE_AUTHORITY_QUERY_OPERATION_ID,
      statuses: [{ status: "unavailable" }],
    };
  }
  return {
    operation_id: runtime.OUTPUT_LEASE_AUTHORITY_QUERY_OPERATION_ID,
    statuses: [{
      status: state === "orphaned" ? "held_orphaned" : "held_active",
      authority: action.lease,
      resources: action.kind === "arm" && state !== "wrong"
        ? resourcesFor(action)
        : action.kind === "add_display" || action.kind === "assign_video_output_composition"
          ? ["lighting", "video"] : ["lighting", "video"],
    }],
  };
};

const receiptFor = (request, action, enableRecovery = false, enableRecoveryGenerationDelta = 1) => {
  if (action.kind === "reset_show_spout_outputs") {
    return {
      type: "receipt",
      receipt: {
        operation_id: request.operation_id,
        request_id: request.request_id,
        shape_sha256: hash("a"),
        argument_fingerprint: hash("b"),
        audit_sequence: 1,
        fence_before: structuredClone(request.expected_fence),
        fence_after: {
          ...structuredClone(request.expected_fence),
          project_revision: request.expected_fence.project_revision + 1,
          project_checkpoint_hash: hash("d"),
          project_publication_generation: request.expected_fence.project_publication_generation + 1,
        },
        outcome: "applied",
        lease_result: null,
      },
    };
  }
  const recoveringEnable = action.kind === "enable_output" && enableRecovery;
  const resources = action.kind === "acquire_lease" || action.kind === "enable_output"
    ? resourcesFor(action)
    : action.kind === "arm" ? resourcesFor(action)
      : action.kind === "add_display" || action.kind === "assign_video_output_composition"
        ? ["lighting", "video"] : ["lighting", "video"];
  const inputGeneration = action.kind === "acquire_lease" || action.kind === "enable_output" && !enableRecovery
    ? null : recoveringEnable ? 2 : action.lease.generation;
  const terminalGeneration = inputGeneration === null ? 1
    : recoveringEnable ? inputGeneration + enableRecoveryGenerationDelta
      : action.kind === "relinquish_output_lease" || action.kind === "renew_lease"
        || action.kind === "recover_lease" || action.kind === "force_transfer_lease"
        ? action.lease.generation + 1 : action.lease.generation;
  const relinquished = action.kind === "relinquish_output_lease";
  const outcome = ({
    arm: "authorized", set_blackout: "authorized", release_blackout: "authorized", take_over_standby: "authorized",
    add_display: "authorized", assign_video_output_composition: "authorized",
    [canonicalShowArtNetActionKind]: "authorized",
    [canonicalShowSerialDmxEnableActionKind]: "authorized",
    [canonicalShowSerialDmxStopActionKind]: "authorized",
    send_dsf2026_artnet_acceptance_probe: "authorized",
    acknowledge_dsf2026_artnet_acceptance_probe_in_doubt: "authorized",
    enable_show_spout_outputs: "authorized",
    acquire_lease: "acquired", enable_output: recoveringEnable ? "recovered" : "acquired",
    renew_lease: "renewed", recover_lease: "recovered", relinquish_output_lease: "relinquished",
    force_transfer_lease: "transferred",
  })[action.kind];
  const persistedArtNetMutation = action.kind === canonicalShowArtNetActionKind || action.kind === "set_blackout";
  return {
    type: "receipt",
    receipt: {
      operation_id: request.operation_id,
      request_id: request.request_id,
      shape_sha256: hash("a"),
      argument_fingerprint: hash("b"),
      audit_sequence: 1,
      fence_before: structuredClone(request.expected_fence),
      fence_after: persistedArtNetMutation ? {
        ...structuredClone(request.expected_fence),
        project_revision: request.expected_fence.project_revision + 1,
        project_checkpoint_hash: hash("e"),
        project_publication_generation: request.expected_fence.project_publication_generation + 1,
      } : structuredClone(request.expected_fence),
      outcome: persistedArtNetMutation || action.kind === canonicalShowSerialDmxEnableActionKind
        || action.kind === canonicalShowSerialDmxStopActionKind
        || action.kind === "send_dsf2026_artnet_acceptance_probe"
        || action.kind === "acknowledge_dsf2026_artnet_acceptance_probe_in_doubt"
        ? "applied" : "no_op",
      lease_result: {
        authority: action.kind === "acquire_lease" || action.kind === "enable_output" && !enableRecovery
          ? lease(1) : lease(terminalGeneration),
        resources,
        phase: relinquished ? "unclaimed" : "held_active",
        outcome,
        audit_sequence: 1,
        changes: [{
          lease_id: "lease-0000000000000001",
          before_generation: inputGeneration,
          after_generation: terminalGeneration,
          before_resources: inputGeneration === null ? [] : resources,
          after_resources: relinquished ? [] : resources,
          before_phase: inputGeneration === null ? null : recoveringEnable ? "held_orphaned" : "held_active",
          after_phase: relinquished ? "unclaimed" : "held_active",
        }],
      },
    },
  };
};

const createHarness = ({ action, queryState, authorityFence = fence, loseFirstReply = false, typedRejection = false, enableRecovery = false, enableRecoveryGenerationDelta = 1, receiptTransform } = {}) => {
  const operationId = operationFor(action);
  const command = commandFor(action);
  const calls = [];
  const executeArgs = [];
  let executeCalls = 0;
  const invoke = async (actualCommand, args) => {
    calls.push({ command: actualCommand, args });
    if (actualCommand === "query_output_control_authority_v1") {
      assert.equal(args, undefined);
      return { operation_id: runtime.OUTPUT_CONTROL_AUTHORITY_QUERY_OPERATION_ID, fence: structuredClone(authorityFence) };
    }
    if (actualCommand === "query_output_lease_authority_v1") {
      assert.equal(args, undefined);
      return queryFor(action, queryState);
    }
    assert.equal(actualCommand, command);
    assert.deepEqual(Object.keys(args), ["request"]);
    assert.deepEqual(Object.keys(args.request).sort(), ["action", "expected_fence", "operation_id", "request_id"]);
    assert.equal(Object.hasOwn(args.request, "consent_token"), false);
    assert.deepEqual(args.request.action, action);
    assert.equal(args.request.operation_id, operationId);
    executeArgs.push(args);
    if (loseFirstReply && executeCalls === 0) {
      executeCalls += 1;
      throw new Error("synthetic lost transport reply");
    }
    executeCalls += 1;
    if (typedRejection) return {
      type: "rejected",
      rejection: { operation_id: operationId, request_id: args.request.request_id, error: "forbidden" },
    };
    const receipt = receiptFor(args.request, action, enableRecovery, enableRecoveryGenerationDelta);
    return receiptTransform ? receiptTransform(receipt) : receipt;
  };
  return { invoke, calls, executeArgs, get executeCalls() { return executeCalls; } };
};

for (const action of [enableAction, ...ordinaryActions, ...lifecycleActions]) {
  const harness = createHarness({ action, queryState: action.kind === "recover_lease" ? "orphaned" : "active" });
  const receipt = action.kind === "acquire_lease"
    ? await runtime.executeOutputLeaseLifecycle(harness.invoke, action)
    : action.kind === "enable_output" || action.kind === "arm" || action.kind === "release_blackout"
      || action.kind === "take_over_standby" || action.kind === "add_display"
      || action.kind === "assign_video_output_composition"
      || action.kind === canonicalShowArtNetActionKind
      || action.kind === canonicalShowSerialDmxEnableActionKind
      || action.kind === canonicalShowSerialDmxStopActionKind
      || action.kind === "send_dsf2026_artnet_acceptance_probe"
      || action.kind === "acknowledge_dsf2026_artnet_acceptance_probe_in_doubt"
      || action.kind === "enable_show_spout_outputs"
      ? await runtime.executeOutputControl(harness.invoke, action)
      : await runtime.executeOutputLeaseLifecycle(harness.invoke, action);
  assert.equal(receipt.operation_id, operationFor(action));
  assert.equal(harness.executeCalls, 1);
}

const showArtNetLoopbackHarness = createHarness({ action: showArtNetLoopbackRouteAction, queryState: "active" });
const showArtNetLoopbackReceipt = await runtime.executeOutputControl(showArtNetLoopbackHarness.invoke, showArtNetLoopbackRouteAction);
assert.equal(showArtNetLoopbackReceipt.operation_id, runtime.OUTPUT_SHOW_ARTNET_LOOPBACK_ROUTE_ENABLE_OPERATION_ID);
assert.equal(
  showArtNetLoopbackHarness.executeArgs[0].request.action.kind,
  canonicalShowArtNetActionKind,
  "the emitted Art-Net show route request must carry the canonical action kind",
);
assert.equal(
  showArtNetLoopbackHarness.calls.find((call) => call.command === "enable_show_art_net_loopback_route_v1")?.command,
  "enable_show_art_net_loopback_route_v1",
  "the emitted Art-Net show route request must invoke the canonical Tauri command",
);
assert.deepEqual(
  Object.keys(showArtNetLoopbackHarness.executeArgs[0].request.action).sort(),
  ["kind", "lease"],
  "the show route action must not carry route/protocol/port fields",
);
assert.equal(showArtNetLoopbackReceipt.outcome, "applied");
assert.equal(
  showArtNetLoopbackReceipt.fence_after.project_revision,
  showArtNetLoopbackReceipt.fence_before.project_revision + 1,
  "enabling the authored Art-Net route must return its committed project revision",
);
assert.notEqual(
  showArtNetLoopbackReceipt.fence_after.project_checkpoint_hash,
  showArtNetLoopbackReceipt.fence_before.project_checkpoint_hash,
  "enabling the authored Art-Net route must return its committed project hash",
);
assert.equal(
  showArtNetLoopbackReceipt.fence_after.project_publication_generation,
  showArtNetLoopbackReceipt.fence_before.project_publication_generation + 1,
  "enabling the authored Art-Net route must return its committed publication generation",
);
assert.equal(showArtNetLoopbackReceipt.fence_after.output_epoch, showArtNetLoopbackReceipt.fence_before.output_epoch);
assert.equal(showArtNetLoopbackReceipt.fence_after.output_generation, showArtNetLoopbackReceipt.fence_before.output_generation);
assert.equal(showArtNetLoopbackReceipt.fence_after.safety_blackout_epoch, showArtNetLoopbackReceipt.fence_before.safety_blackout_epoch);
assert.equal(showArtNetLoopbackReceipt.fence_after.safety_blackout_generation, showArtNetLoopbackReceipt.fence_before.safety_blackout_generation);

for (const [name, mutate] of [
  ["unchanged project fence", (receipt) => { receipt.receipt.fence_after = structuredClone(receipt.receipt.fence_before); }],
  ["skipped project revision", (receipt) => { receipt.receipt.fence_after.project_revision += 1; }],
  ["unchanged project hash", (receipt) => { receipt.receipt.fence_after.project_checkpoint_hash = receipt.receipt.fence_before.project_checkpoint_hash; }],
  ["mutated output generation", (receipt) => { receipt.receipt.fence_after.output_generation += 1; }],
]) {
  const harness = createHarness({
    action: showArtNetLoopbackRouteAction,
    queryState: "active",
    receiptTransform: (receipt) => {
      mutate(receipt);
      return receipt;
    },
  });
  await assert.rejects(
    runtime.executeOutputControl(harness.invoke, showArtNetLoopbackRouteAction),
    /receipt was inconsistent; physical output state is unknown/,
    `Art-Net receipt must reject ${name}`,
  );
}

const concurrentS0Harness = createHarness({
  action: showArtNetLoopbackRouteAction,
  queryState: "active",
  receiptTransform: (receipt) => {
    receipt.receipt.fence_after.safety_blackout_generation += 1;
    return receipt;
  },
});
const concurrentS0Receipt = await runtime.executeOutputControl(
  concurrentS0Harness.invoke,
  showArtNetLoopbackRouteAction,
);
assert.equal(
  concurrentS0Receipt.fence_after.safety_blackout_generation,
  concurrentS0Receipt.fence_before.safety_blackout_generation + 1,
  "an independently advanced priority S0 authority must remain reportable",
);

for (const [action, operationId, command] of [
  [showSerialDmxEnableAction, runtime.OUTPUT_SHOW_SERIAL_DMX_SAFETY_BLACKOUT_ROUTE_ENABLE_OPERATION_ID, "enable_show_serial_dmx_safety_blackout_route_v1"],
  [showSerialDmxStopAction, runtime.OUTPUT_SHOW_SERIAL_DMX_SAFETY_BLACKOUT_ROUTE_STOP_OPERATION_ID, "stop_show_serial_dmx_safety_blackout_route_v1"],
]) {
  const harness = createHarness({ action, queryState: "active" });
  const receipt = await runtime.executeOutputControl(harness.invoke, action);
  assert.equal(receipt.operation_id, operationId);
  assert.equal(
    harness.calls.find((call) => call.command === command)?.command,
    command,
    "the USB-DMX action must invoke only its fixed Tauri command",
  );
  assert.deepEqual(
    Object.keys(harness.executeArgs[0].request.action).sort(),
    ["kind", "lease"],
    "USB-DMX actions must never carry COM/PnP/protocol/channel/frame data",
  );
  assert.deepEqual(
    receipt.fence_before,
    receipt.fence_after,
    "USB-DMX route start/stop must not mutate project or output-control fences",
  );
}

const showSpoutOutputsHarness = createHarness({ action: showSpoutOutputsAction, queryState: "active" });
const showSpoutOutputsReceipt = await runtime.executeOutputControl(showSpoutOutputsHarness.invoke, showSpoutOutputsAction);
assert.equal(showSpoutOutputsReceipt.operation_id, runtime.OUTPUT_SHOW_SPOUT_OUTPUTS_ENABLE_OPERATION_ID);
assert.deepEqual(
  Object.keys(showSpoutOutputsHarness.executeArgs[0].request.action).sort(),
  ["kind", "lease"],
  "the strict show Spout action must not carry names, dimensions, or generic route fields",
);

const showSpoutResetHarness = createHarness({ action: showSpoutResetAction });
const showSpoutResetReceipt = await runtime.executeOutputControl(
  showSpoutResetHarness.invoke,
  showSpoutResetAction,
);
assert.equal(showSpoutResetReceipt.operation_id, runtime.OUTPUT_SHOW_SPOUT_OUTPUTS_RESET_OPERATION_ID);
assert.equal(showSpoutResetReceipt.lease_result, null);
assert.notDeepEqual(
  showSpoutResetReceipt.fence_before,
  showSpoutResetReceipt.fence_after,
  "a reset that retires authored outputs must advance its project/output fence",
);
assert.deepEqual(
  Object.keys(showSpoutResetHarness.executeArgs[0].request.action),
  ["kind"],
  "the reset action must be payloadless and may not carry a lease or a target",
);
assert.equal(
  showSpoutResetHarness.calls.some((call) => call.command === "query_output_lease_authority_v1"),
  false,
  "reset must remain usable from Standby without an active output lease",
);

const dsf2026ArtNetAcceptanceProbeHarness = createHarness({ action: dsf2026ArtNetAcceptanceProbeAction, queryState: "active" });
const dsf2026ArtNetAcceptanceProbeReceipt = await runtime.executeOutputControl(
  dsf2026ArtNetAcceptanceProbeHarness.invoke,
  dsf2026ArtNetAcceptanceProbeAction,
);
assert.equal(
  dsf2026ArtNetAcceptanceProbeReceipt.operation_id,
  runtime.OUTPUT_DSF2026_ARTNET_ACCEPTANCE_PROBE_OPERATION_ID,
);
assert.equal(
  dsf2026ArtNetAcceptanceProbeReceipt.outcome,
  "applied",
  "a successfully accepted one-shot probe is an Applied physical receipt, not NoOp",
);
assert.deepEqual(
  dsf2026ArtNetAcceptanceProbeReceipt.fence_before,
  dsf2026ArtNetAcceptanceProbeReceipt.fence_after,
  "the fixed probe must leave every project/output fence unchanged",
);
assert.deepEqual(
  Object.keys(dsf2026ArtNetAcceptanceProbeHarness.executeArgs[0].request.action).sort(),
  ["kind", "lease"],
  "the DSF2026 probe action must not carry a target, universe, payload, or retry field",
);

const dsf2026ArtNetAcceptanceProbeReconcileHarness = createHarness({
  action: dsf2026ArtNetAcceptanceProbeReconcileAction,
  queryState: "active",
});
const dsf2026ArtNetAcceptanceProbeReconcileReceipt = await runtime.executeOutputControl(
  dsf2026ArtNetAcceptanceProbeReconcileHarness.invoke,
  dsf2026ArtNetAcceptanceProbeReconcileAction,
);
assert.equal(
  dsf2026ArtNetAcceptanceProbeReconcileReceipt.operation_id,
  runtime.OUTPUT_DSF2026_ARTNET_ACCEPTANCE_PROBE_RECONCILE_OPERATION_ID,
);
assert.equal(
  dsf2026ArtNetAcceptanceProbeReconcileReceipt.outcome,
  "applied",
  "an explicit no-send reconciliation is a terminal Applied receipt",
);
assert.deepEqual(
  dsf2026ArtNetAcceptanceProbeReconcileReceipt.fence_before,
  dsf2026ArtNetAcceptanceProbeReconcileReceipt.fence_after,
  "probe reconciliation must not mutate a project/output fence",
);

const assignmentAction = ordinaryActions.find((action) => action.kind === "assign_video_output_composition");
assert.ok(assignmentAction, "canonical video-output assignment must remain an ordinary OutputControl action");
const orphanedAssignmentHarness = createHarness({ action: assignmentAction, queryState: "orphaned" });
await assert.rejects(
  runtime.executeOutputControl(orphanedAssignmentHarness.invoke, assignmentAction),
  /Selected output lease is unavailable, orphaned, stale, or has the wrong resources/,
  "route assignment must not recover or use an orphaned lease",
);
assert.equal(orphanedAssignmentHarness.executeCalls, 0);

const enableHarness = createHarness({ action: enableAction });
const enableReceipt = await runtime.enableOutput(enableHarness.invoke);
assert.equal(enableReceipt.operation_id, runtime.OUTPUT_ENABLE_OPERATION_ID);
assert.equal(enableReceipt.lease_result.outcome, "acquired");
assert.deepEqual(enableReceipt.lease_result.resources, ["lighting", "video"]);
assert.equal(enableHarness.executeCalls, 1);
assert.equal(Object.hasOwn(enableHarness.executeArgs[0].request.action, "lease"), false,
  "normal Enable Output must not require a preselected lease");

const enableRecoveryHarness = createHarness({ action: enableAction, enableRecovery: true });
const enableRecoveryReceipt = await runtime.enableOutput(enableRecoveryHarness.invoke);
assert.equal(enableRecoveryReceipt.lease_result.outcome, "recovered");
assert.equal(enableRecoveryReceipt.lease_result.changes[0].before_phase, "held_orphaned");
assert.equal(enableRecoveryReceipt.lease_result.changes[0].after_phase, "held_active");
assert.equal(enableRecoveryHarness.executeCalls, 1);
for (const enableRecoveryGenerationDelta of [0, -1]) {
  const forgedRecoveryHarness = createHarness({
    action: enableAction,
    enableRecovery: true,
    enableRecoveryGenerationDelta,
  });
  await assert.rejects(
    runtime.enableOutput(forgedRecoveryHarness.invoke),
    /lease result was invalid/,
    "Enable recovery must reject equal or decreasing receipt generations",
  );
  assert.equal(forgedRecoveryHarness.executeCalls, 1);
}

const leaseQuery = (statuses) => ({
  operation_id: runtime.OUTPUT_LEASE_AUTHORITY_QUERY_OPERATION_ID,
  statuses,
});
assert.equal(
  runtime.hasOnlyActiveOutputLease(leaseQuery([{
    status: "held_active",
    authority: lease(),
    resources: ["lighting", "video"],
  }]), ["lighting", "video"]),
  true,
  "normal Enable remains satisfied only while exactly one active Both lease exists",
);
assert.match(
  controllerSource,
  /export async function executeBlackoutRelease\(/,
  "blackout release helper must remain an explicit controller boundary",
);
const blackoutReleaseSourceStart = controllerSource.indexOf(
  "export async function executeBlackoutRelease(",
);
const blackoutReleaseSourceEnd = controllerSource.indexOf(
  "\nexport ",
  blackoutReleaseSourceStart + 1,
);
assert.notEqual(
  blackoutReleaseSourceStart,
  -1,
  "blackout release helper source must be discoverable",
);
const blackoutReleaseSource = controllerSource.slice(
  blackoutReleaseSourceStart,
  blackoutReleaseSourceEnd === -1 ? controllerSource.length : blackoutReleaseSourceEnd,
);
assert.match(
  blackoutReleaseSource,
  /hasOnlyActiveOutputLease[\s\S]*await enableOutput\(invoke\)[\s\S]*executeOutputControl\(invoke, \{ kind: "release_blackout", lease \}\)/,
  "blackout release must recover the canonical Both lease only through the existing Enable path",
);
assert.match(
  blackoutReleaseSource,
  /const enableReceipt = await enableOutput\(invoke\)[\s\S]*enabledLease = enableReceipt\.lease_result[\s\S]*skipPublicLeaseQuery: "enabled-lease-proof"/,
  "blackout recovery must consume the validated Enable receipt as its immediate release proof",
);
assert.doesNotMatch(
  blackoutReleaseSource,
  /await enableOutput\(invoke\)[\s\S]*await queryOutputLeaseAuthority\(invoke\)[\s\S]*selectOnlyActiveOutputLease/,
  "blackout recovery must not depend on a second fail-fast lease read after Enable",
);
assert.equal(
  runtime.hasOnlyActiveOutputLease(leaseQuery([]), ["lighting", "video"]),
  false,
  "expired or missing authority must re-enable the normal one-click Enable path",
);

const blackoutReleaseAction = { kind: "release_blackout", lease: lease() };
const blackoutRecoveryCalls = [];
const blackoutRecoveryInvoke = async (command, args) => {
  blackoutRecoveryCalls.push(command);
  if (command === "query_output_control_authority_v1") {
    assert.equal(args, undefined);
    return { operation_id: runtime.OUTPUT_CONTROL_AUTHORITY_QUERY_OPERATION_ID, fence: structuredClone(fence) };
  }
  if (command === "query_output_lease_authority_v1") {
    assert.equal(args, undefined);
    // The public read lane intentionally remains stale after Enable.  The
    // Enable receipt, not a second fail-fast query, is the proof consumed by
    // the immediate blackout Release bridge.
    return leaseQuery([{ status: "unavailable" }]);
  }
  if (command === "enable_output_control_v2") {
    assert.deepEqual(args.request.action, enableAction);
    return receiptFor(args.request, enableAction);
  }
  if (command === "release_blackout_output_control_v2") {
    assert.deepEqual(args.request.action, blackoutReleaseAction);
    return receiptFor(args.request, blackoutReleaseAction);
  }
  throw new Error(`unexpected blackout recovery command: ${command}`);
};
const blackoutRecoveryReceipt = await runtime.executeBlackoutRelease(blackoutRecoveryInvoke);
assert.equal(blackoutRecoveryReceipt.operation_id, runtime.OUTPUT_BLACKOUT_RELEASE_OPERATION_ID);
assert.deepEqual(blackoutRecoveryCalls, [
  "query_output_lease_authority_v1",
  "query_output_control_authority_v1",
  "query_output_lease_authority_v1",
  "enable_output_control_v2",
  "query_output_control_authority_v1",
  "release_blackout_output_control_v2",
], "a missing lease may recover exactly once before the fenced release");

const activeBlackoutHarness = createHarness({ action: blackoutReleaseAction, queryState: "active" });
await runtime.executeBlackoutRelease(activeBlackoutHarness.invoke);
assert.equal(activeBlackoutHarness.executeCalls, 1, "an active Both lease must release without an Enable mutation");

const splitBlackoutCalls = [];
await assert.rejects(
  runtime.executeBlackoutRelease(async (command, args) => {
    splitBlackoutCalls.push(command);
    if (command === "query_output_lease_authority_v1") return leaseQuery([{
      status: "held_active",
      authority: lease(),
      resources: ["lighting"],
    }]);
    throw new Error(`unexpected split blackout command: ${command}`);
  }),
  /requires one active local Both output lease/,
  "an active split lease must remain fail-closed and must not be auto-recovered",
);
assert.deepEqual(splitBlackoutCalls, ["query_output_lease_authority_v1"]);

assert.equal(
  runtime.hasOnlyActiveOutputLease(leaseQuery([
    { status: "held_active", authority: lease(1), resources: ["lighting", "video"] },
    { status: "held_active", authority: lease(2), resources: ["lighting", "video"] },
  ]), ["lighting", "video"]),
  false,
  "ambiguous active Both leases must not look like a usable normal authority",
);
assert.deepEqual(
  runtime.selectOnlyActiveOutputLease(leaseQuery([{
    status: "held_active",
    authority: lease(),
    resources: ["lighting", "video"],
  }]), ["lighting", "video"]),
  lease(),
  "Display add accepts the exact Both lease created by normal Enable",
);
assert.throws(
  () => runtime.selectOnlyActiveOutputLease(leaseQuery([{
    status: "held_active",
    authority: lease(),
    resources: ["video"],
  }]), ["lighting", "video"]),
  /Exactly one active output lease/,
  "Lighting-only/Video-only lease candidates must not authorize Display add",
);
assert.throws(
  () => runtime.selectOnlyActiveOutputLease(leaseQuery([{
    status: "held_active",
    authority: lease(),
    resources: ["lighting"],
  }]), ["lighting", "video"]),
  /Exactly one active output lease/,
  "Lighting-only lease candidates must not authorize Display add",
);
assert.throws(
  () => runtime.selectOnlyActiveOutputLease(leaseQuery([{ status: "unavailable" }]), ["lighting", "video"]),
  /Exactly one active output lease/,
  "No lease must fail closed before Display add",
);
assert.throws(
  () => runtime.selectOnlyActiveOutputLease(leaseQuery([
    { status: "held_active", authority: lease(1), resources: ["lighting", "video"] },
    { status: "held_active", authority: lease(2), resources: ["lighting", "video"] },
  ]), ["lighting", "video"]),
  /Exactly one active output lease/,
  "Multiple Both leases must fail closed before Display add",
);

for (const field of ["project_epoch", "project_revision", "project_publication_generation"]) {
  const zeroHarness = createHarness({ action: ordinaryActions[0], authorityFence: { ...fence, [field]: 0 } });
  const zeroReceipt = await runtime.executeOutputControl(zeroHarness.invoke, ordinaryActions[0]);
  assert.equal(zeroReceipt.operation_id, runtime.OUTPUT_OWNERSHIP_ARM_OPERATION_ID);
  assert.equal(zeroHarness.executeCalls, 1);
}
for (const field of ["project_epoch", "project_revision", "project_publication_generation"]) {
  for (const invalid of [-1, 1.5, Number.MAX_SAFE_INTEGER + 1]) {
    const harness = createHarness({ action: ordinaryActions[0], authorityFence: { ...fence, [field]: invalid } });
    await assert.rejects(runtime.executeOutputControl(harness.invoke, ordinaryActions[0]), /authority response was invalid/);
    assert.equal(harness.executeCalls, 0);
  }
}
const unknownFenceHarness = createHarness({ action: ordinaryActions[0], authorityFence: { ...fence, unexpected: 1 } });
await assert.rejects(runtime.executeOutputControl(unknownFenceHarness.invoke, ordinaryActions[0]), /authority response was invalid/);
assert.equal(unknownFenceHarness.executeCalls, 0);

const malformedHarness = createHarness({ action: ordinaryActions[0] });
malformedHarness.invoke = async (command, args) => command === "query_output_lease_authority_v1"
  ? { operation_id: runtime.OUTPUT_LEASE_AUTHORITY_QUERY_OPERATION_ID, statuses: [{ status: "unavailable" }, { status: "held_active", authority: lease(), resources: ["lighting"] }] }
  : createHarness({ action: ordinaryActions[0] }).invoke(command, args);
await assert.rejects(runtime.executeOutputControl(malformedHarness.invoke, ordinaryActions[0]), /query was invalid/);
assert.equal(malformedHarness.calls.filter((call) => call.command === "prepare_output_consent_v1").length, 0);

for (const queryState of ["orphaned", "wrong"]) {
  const action = { kind: "arm", role: "video", lease: lease() };
  const harness = createHarness({ action, queryState });
  await assert.rejects(runtime.executeOutputControl(harness.invoke, action), /wrong resources|orphaned/);
  assert.equal(harness.executeCalls, 0);
}

const replyLossAction = ordinaryActions[2];
const replyLossHarness = createHarness({ action: replyLossAction, loseFirstReply: true });
await runtime.executeOutputControl(replyLossHarness.invoke, replyLossAction);
assert.equal(replyLossHarness.executeCalls, 2);
assert.equal(replyLossHarness.executeArgs[0], replyLossHarness.executeArgs[1]);
assert.equal(Object.isFrozen(replyLossHarness.executeArgs[0]), true);

const dsf2026ProbeReplyLossHarness = createHarness({
  action: dsf2026ArtNetAcceptanceProbeAction,
  queryState: "active",
  loseFirstReply: true,
});
await assert.rejects(
  runtime.executeOutputControl(
    dsf2026ProbeReplyLossHarness.invoke,
    dsf2026ArtNetAcceptanceProbeAction,
  ),
  /no IPC retry was attempted/,
  "a lost fixed-probe IPC reply must not issue a second native command that could send again",
);
assert.equal(
  dsf2026ProbeReplyLossHarness.executeCalls,
  1,
  "the fixed probe reply-loss path makes exactly one IPC command invocation",
);
assert.equal(
  dsf2026ProbeReplyLossHarness.executeArgs.length,
  1,
  "the fixed probe has no transparent reply-retry request",
);
assert.match(
  controllerSource,
  /send_dsf2026_artnet_acceptance_probe[\s\S]*no IPC retry was attempted/,
  "the production fixed-probe reply-loss boundary must remain explicitly non-retrying",
);

const rejectionHarness = createHarness({ action: ordinaryActions[0], typedRejection: true });
await assert.rejects(runtime.executeOutputControl(rejectionHarness.invoke, ordinaryActions[0]), /refresh lease state/);
assert.equal(rejectionHarness.executeCalls, 1);


const exhaustedSource = controllerSource.replace(
  "let nextOutputControlRequestId = 1;",
  "let nextOutputControlRequestId = Number.MAX_SAFE_INTEGER + 1;",
);
const exhaustedRuntime = await import(`data:text/javascript;base64,${Buffer.from(ts.transpileModule(
  exhaustedSource,
  { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022, importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove }, fileName: "outputControlController.ts" },
).outputText).toString("base64")}`);
let exhaustedInvokes = 0;
await assert.rejects(exhaustedRuntime.executeOutputControl(async () => { exhaustedInvokes += 1; }, ordinaryActions[0]), /request identity is exhausted/);
assert.equal(exhaustedInvokes, 0);

const [
  invokeSource,
  manifestSource,
  appSource,
  standbySource,
  dmxOutputPanelSource,
  ioConnectionDeckSource,
  outputDiagnosticsSource,
  mainSource,
  runtimeSource,
  querySource,
  controlPlaneSource,
  showSerialDmxRouteSource,
  serialDmxStatusPollerSource,
  serialDmxStatusValidationSource,
  engineShowSerialDmxStatusSource,
  engineSource,
  engineShowSerialDmxTestsSource,
  registrySource,
] = await Promise.all([
  read("src/tauriInvokeCommands.ts"),
  read("src/tauri-invoke-manifest.json"),
  read("src/App.tsx"),
  read("src/components/StandbySyncPanel.tsx"),
  read("src/components/DmxOutputConfigPanel.tsx"),
  read("src/components/IoConnectionDeck.tsx"),
  read("src/createOutputDiagnosticsController.ts"),
  read("src-tauri/src/main.rs"),
  read("src-tauri/src/control_plane_runtime.rs"),
  read("src-tauri/src/control_plane_query.rs"),
  read("src-tauri/src/control_plane.rs"),
  read("src-tauri/src/show_serial_dmx_route.rs"),
  read("src/serialDmxStatusPoller.ts"),
  read("src/serialDmxStatusValidation.ts"),
  read("../crates/engine/src/show_serial_dmx_status.rs"),
  read("../crates/engine/src/lib.rs"),
  read("../crates/engine/src/show_serial_dmx_tests.rs"),
  read("../crates/protocol/src/control_plane_registry_v2.rs"),
]);
const serialDmxStatusPollerRuntime = await import(`data:text/javascript;base64,${Buffer.from(ts.transpileModule(
  serialDmxStatusPollerSource,
  {
    compilerOptions: {
      module: ts.ModuleKind.ESNext,
      target: ts.ScriptTarget.ES2022,
      importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
    },
    fileName: "serialDmxStatusPoller.ts",
  },
).outputText).toString("base64")}`);
const serialDmxStatusValidationRuntime = await import(`data:text/javascript;base64,${Buffer.from(ts.transpileModule(
  serialDmxStatusValidationSource,
  {
    compilerOptions: {
      module: ts.ModuleKind.ESNext,
      target: ts.ScriptTarget.ES2022,
      importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
    },
    fileName: "serialDmxStatusValidation.ts",
  },
).outputText).toString("base64")}`);
assert.match(
  outputDiagnosticsSource,
  /kind: "enable_show_art_net_loopback_route"/,
  "the diagnostics controller must emit the canonical Art-Net show route action",
);
assert.doesNotMatch(
  outputDiagnosticsSource,
  new RegExp(`kind:\\s*[\"']${retiredShowArtNetActionKind}[\"']`),
  "the retired Art-Net show route action discriminant must be absent from diagnostics",
);
const commandBody = (source, commandName) => {
  const functionMarker = `fn ${commandName}(`;
  const functionStart = source.indexOf(functionMarker);
  assert.notEqual(functionStart, -1, `${commandName} must exist`);
  const attributeStart = source.lastIndexOf("#[tauri::command]", functionStart);
  assert.notEqual(attributeStart, -1, `${commandName} must remain a Tauri command`);
  const nextAttribute = source.indexOf("\n#[tauri::command]", functionStart + functionMarker.length);
  return source.slice(attributeStart, nextAttribute === -1 ? source.length : nextAttribute);
};
const requiredCommands = [
  "enable_output_control_v2", "add_display_output_v2", "arm_output_control_v2",
  "assign_video_output_composition_v2",
  "acknowledge_dsf2026_artnet_acceptance_probe_in_doubt_v1",
  "query_dsf2026_artnet_acceptance_probe_status_v1",
  "enable_show_art_net_loopback_route_v1",
  "enable_show_serial_dmx_safety_blackout_route_v1",
  "stop_show_serial_dmx_safety_blackout_route_v1",
  "get_serial_dmx_machine_binding_status_v1",
  "get_show_serial_dmx_safety_blackout_route_status_v1",
  "select_serial_dmx_machine_binding_v1",
  "send_dsf2026_artnet_acceptance_probe_v1",
  "release_blackout_output_control_v2", "set_blackout_output_control_v2", "take_over_output_control_v2",
  "acquire_output_lease_v2", "force_transfer_output_lease_v2", "query_output_lease_authority_v1",
  "recover_output_lease_v2", "relinquish_output_lease_v2", "renew_output_lease_v2",
];
const legacyCommands = [
  "enable_output_control_v1", "add_display_output_v1", "arm_output_control_v1",
  "release_blackout_output_control_v1", "take_over_output_control_v1", "acquire_output_lease_v1",
  "force_transfer_output_lease_v1", "recover_output_lease_v1", "relinquish_output_lease_v1",
  "renew_output_lease_v1", "prepare_output_consent_v1", "query_output_consent_status_v1",
];
const canonicalOutputMutationWrappers = [
  "acknowledge_dsf2026_artnet_acceptance_probe_in_doubt_v1",
  "acquire_output_lease_v2",
  "add_display_output_v2",
  "arm_output_control_v2",
  "assign_video_output_composition_v2",
  "enable_output_control_v2",
  "enable_show_art_net_loopback_route_v1",
  "enable_show_serial_dmx_safety_blackout_route_v1",
  "force_transfer_output_lease_v2",
  "recover_output_lease_v2",
  "release_blackout_output_control_v2",
  "relinquish_output_lease_v2",
  "renew_output_lease_v2",
  "send_dsf2026_artnet_acceptance_probe_v1",
  "set_blackout_output_control_v2",
  "stop_show_serial_dmx_safety_blackout_route_v1",
  "take_over_output_control_v2",
];
const outputControlWrappers = new Set([
  "acknowledge_dsf2026_artnet_acceptance_probe_in_doubt_v1",
  "add_display_output_v2",
  "assign_video_output_composition_v2",
  "arm_output_control_v2",
  "enable_output_control_v2",
  "enable_show_art_net_loopback_route_v1",
  "enable_show_serial_dmx_safety_blackout_route_v1",
  "send_dsf2026_artnet_acceptance_probe_v1",
  "stop_show_serial_dmx_safety_blackout_route_v1",
  "release_blackout_output_control_v2",
  "set_blackout_output_control_v2",
  "take_over_output_control_v2",
]);
const manifest = JSON.parse(manifestSource);
const tuple = [...invokeSource.matchAll(/^\s+"([^"]+)",$/gm)].map((match) => match[1]);
assert.deepEqual(tuple, [...tuple].sort(), "frontend invoke tuple must remain bytewise sorted");
assert.deepEqual(manifest, [...manifest].sort(), "Tauri invoke manifest must remain bytewise sorted");
for (const command of requiredCommands) {
  assert.ok(tuple.includes(command), `${command} missing from invoke tuple`);
  assert.ok(manifest.includes(command), `${command} missing from invoke manifest`);
  assert.match(mainSource, new RegExp(`\\b${command}\\b`));
}
for (const command of legacyCommands) {
  assert.doesNotMatch(tuple.join("\n"), new RegExp(`\\b${command}\\b`));
  assert.doesNotMatch(manifest.join("\n"), new RegExp(`\\b${command}\\b`));
  assert.doesNotMatch(mainSource, new RegExp(`\\b${command}\\b`));
}
const detectedAsyncOutputMutationWrappers = [
  ...mainSource.matchAll(
    /#\[tauri::command\]\r?\nasync fn ((?:[a-z0-9_]+_v2|enable_show_art_net_loopback_route_v1|enable_show_serial_dmx_safety_blackout_route_v1|stop_show_serial_dmx_safety_blackout_route_v1|send_dsf2026_artnet_acceptance_probe_v1|acknowledge_dsf2026_artnet_acceptance_probe_in_doubt_v1))\(/g,
  ),
].map((match) => match[1])
  .filter((command) => canonicalOutputMutationWrappers.includes(command))
  .sort();
assert.deepEqual(
  detectedAsyncOutputMutationWrappers,
  canonicalOutputMutationWrappers,
  "every canonical output/lease mutation wrapper must be async and the exact set must remain audited",
);
for (const command of canonicalOutputMutationWrappers) {
  const start = mainSource.indexOf(`async fn ${command}(`);
  assert.notEqual(start, -1, `${command} must remain async`);
  const nextCommand = mainSource.indexOf("\n#[tauri::command]", start + 1);
  const body = mainSource.slice(start, nextCommand === -1 ? undefined : nextCommand);
  const requiredHelper = command === "enable_show_serial_dmx_safety_blackout_route_v1"
    ? "show_serial_dmx_route::enable_command"
    : command === "stop_show_serial_dmx_safety_blackout_route_v1"
      ? "show_serial_dmx_route::stop_command"
      : outputControlWrappers.has(command)
    ? "execute_output_control_off_event_loop"
    : "execute_output_lease_lifecycle_off_event_loop";
  assert.match(
    body,
    new RegExp(`\\b${requiredHelper}\\(`),
    `${command} must dispatch through ${requiredHelper}`,
  );
  assert.doesNotMatch(
    body,
    /execute_output_(?:control|lease_lifecycle)_for_operation\(/,
    `${command} must not call the blocking canonical dispatcher inline`,
  );
}
const queryWrapper = commandBody(querySource, "query_output_lease_authority_v1");
assert.match(
  queryWrapper,
  /pub\(crate\) async fn query_output_lease_authority_v1\(/,
  "lease authority query must remain an async query wrapper",
);
assert.match(
  queryWrapper,
  /spawn_blocking[\s\S]*state::<ControlPlaneQueryState>/,
  "lease authority query must run off the event loop",
);
const outputAuthorityQueryBody = commandBody(mainSource, "query_output_control_authority_v1");
assert.match(outputAuthorityQueryBody, /async fn query_output_control_authority_v1\(/);
assert.match(
  outputAuthorityQueryBody,
  /spawn_blocking[\s\S]*issue_output_control_authority_for_window_label/,
  "output authority query must run off the event loop",
);
const dsf2026ProbeStatusQueryBody = commandBody(mainSource, "query_dsf2026_artnet_acceptance_probe_status_v1");
assert.match(dsf2026ProbeStatusQueryBody, /async fn query_dsf2026_artnet_acceptance_probe_status_v1\(/);
assert.match(
  dsf2026ProbeStatusQueryBody,
  /spawn_blocking[\s\S]*query_dsf2026_artnet_acceptance_probe_status_for_window/,
  "fixed probe status query must run off the event loop through its read-only status helper",
);
const standbyStatusBody = commandBody(mainSource, "standby_sync_status");
assert.match(standbyStatusBody, /async fn standby_sync_status\(/);
assert.match(standbyStatusBody, /spawn_blocking/, "standby disclosure status must run off the event loop");
const ownershipStatusBody = commandBody(mainSource, "get_output_ownership_status");
assert.match(ownershipStatusBody, /async fn get_output_ownership_status\(/);
assert.match(ownershipStatusBody, /spawn_blocking/, "output ownership disclosure status must run off the event loop");
for (const queryName of [
  "get_engine_telemetry_report",
  "remote_control_status",
  "dmx_input_status",
  "get_snapshot_delta",
]) {
  const body = commandBody(mainSource, queryName);
  assert.match(body, new RegExp(`async fn ${queryName}\\(`),
    `${queryName} must be an async Tauri query`);
  assert.match(body, /spawn_blocking/, `${queryName} must run off the event loop`);
}
const remoteAccessUrlsBody = commandBody(mainSource, "remote_access_urls");
assert.match(
  remoteAccessUrlsBody,
  /fn remote_access_urls\(/,
  "remote_access_urls must remain a short synchronous read-only query",
);
assert.doesNotMatch(
  remoteAccessUrlsBody,
  /async fn remote_access_urls|spawn_blocking/,
  "remote_access_urls must not carry a Tauri invoke context across a worker completion",
);
for (const queryName of [
  "query_control_plane_project_authority",
  "query_control_plane_runtime_generations",
  "query_control_plane_output_ownership",
  "poll_control_plane_observation_events",
]) {
  const body = commandBody(querySource, queryName);
  assert.match(
    body,
    new RegExp(`pub\\(crate\\) async fn ${queryName}\\(`),
    `${queryName} must remain an async query wrapper`,
  );
  assert.match(
    body,
    /spawn_blocking/,
    `${queryName} must run off the event loop`,
  );
}
const projectAuthorityPollBody = commandBody(mainSource, "poll_project_authority_bundle");
assert.match(projectAuthorityPollBody, /async fn poll_project_authority_bundle\(/);
assert.match(
  projectAuthorityPollBody,
  /spawn_blocking[\s\S]*poll_project_authority_bundle_seqlock/,
  "project authority poll must run off the event loop",
);
const displayEnumerationBody = commandBody(mainSource, "list_video_display_monitors");
assert.match(displayEnumerationBody, /async fn list_video_display_monitors\(/);
assert.match(
  displayEnumerationBody,
  /spawn_blocking[\s\S]*available_monitors/,
  "display enumeration must not block the event loop",
);
assert.match(
  querySource,
  /fn capture_source_once\([\s\S]*project_coordinator\.try_lock\(\)/,
  "read-only authority capture must fail fast on coordinator contention",
);
assert.match(
  mainSource,
  /fn apply_native_video_output_window_shell_with_monitor\([\s\S]*missing its authoritative monitor descriptor/,
  "native AddDisplay shell must consume a captured descriptor",
);
assert.match(
  mainSource,
  /start_native_video_live_output\([\s\S]*Some\(\(output\.width, output\.height\)\)/,
  "candidate output worker must not call inner_size before publication",
);
const nativeDisplayPresentationRevalidation = mainSource.slice(
  mainSource.indexOf("fn revalidate_native_display_presentation_authority("),
  mainSource.indexOf("#[derive(Debug)]\nenum NativeDisplayPresentError"),
);
assert.doesNotMatch(
  nativeDisplayPresentationRevalidation,
  /current\.video\s*!=\s*authority\.video/,
  "ordinary render-time video progress must not be treated as a presentation-authority mutation",
);
assert.match(
  nativeDisplayPresentationRevalidation,
  /output\s*!=\s*&authority\.output\s*\|\|\s*!output\.enabled/,
  "native Display presentation must retain the exact enabled output identity fence",
);
assert.match(
  nativeDisplayPresentationRevalidation,
  /sample\.config_token\s*!=\s*authority\.presentation_config_token/,
  "native Display presentation must retain its semantic configuration-token fence",
);
const nativeDisplayPresentBoundary = mainSource.slice(
  mainSource.indexOf("fn present_native_display_frame_if_authorized("),
  mainSource.indexOf("fn native_display_presentation_contract("),
);
assert(
  nativeDisplayPresentBoundary.indexOf("revalidate_native_display_presentation_authority")
    < nativeDisplayPresentBoundary.indexOf("engine.video_presentation_config_token()"),
  "native Display must revalidate its semantic authority before the final bare token load",
);
assert.match(
  nativeDisplayPresentBoundary,
  /engine\.video_presentation_config_token\(\)\s*!=\s*authority\.presentation_config_token[\s\S]*present\(\)/,
  "a semantic mutation after preparation must reach the final token fence before any physical present",
);
const nativeDisplayLiveOutputStart = mainSource.slice(
  mainSource.indexOf("fn start_native_video_live_output("),
  mainSource.indexOf("fn native_video_output_performance("),
);
assert.match(
  nativeDisplayLiveOutputStart,
  /let first_result[\s\S]*prepare_native_video_output[\s\S]*NativeVideoOutputFrame::Presentable\(first_frame\)[\s\S]*present_native_display_frame_if_authorized/,
  "the production first-frame path must use the same fenced native Display present boundary",
);
assert.match(
  mainSource,
  /native_display_playhead_progress_does_not_revoke_unchanged_presentation_authority/,
  "a deterministic native Display regression must prove ordinary progress can present",
);
assert.match(
  mainSource,
  /native_output_qa_driver_uses_canonical_v2_add_path_for_four_sub_displays/,
  "the backend QA driver must exercise canonical v2 AddDisplay requests",
);
assert.match(
  mainSource,
  /standby_disclosure_queries_are_bounded_after_failed_add_and_expired_lease/,
  "the failed-Add disclosure contention regression must remain covered",
);
assert.doesNotMatch(
  mainSource,
  /#\[tauri::command\]\s*(?:async\s+)?fn\s+native_output_qa_driver/,
  "the backend QA driver must not be a production Tauri command",
);
const postAdmissionRuntime = runtimeSource.slice(
  runtimeSource.indexOf("let external_admission = match"),
  runtimeSource.indexOf("let operation_result = match"),
);
assert.doesNotMatch(
  postAdmissionRuntime,
  /validate_display_output_monitor\(window/,
  "post-dialog project/coordinator admission must not call Webview monitor RPCs",
);
const addDisplayBackend = mainSource.slice(
  mainSource.indexOf("fn add_display_output_with_output_control_fence_core<"),
  mainSource.indexOf("fn set_display_output_window_open_with_output_control_fence("),
);
const addDisplayCallback = addDisplayBackend.slice(
  addDisplayBackend.indexOf("let (applied, lease_receipt) ="),
);
assert.doesNotMatch(
  addDisplayCallback,
  /validate_editor_monitor_for_window\(editor_window\)/,
  "AddDisplay callback must not re-enter Webview monitor RPCs under project locks",
);
assert.doesNotMatch(controllerSource, /consent_token|prepare_output_consent|query_output_consent|physical-confirmation|Raw Input|physical Enter/);
for (const operationId of [
  "syndocal.output.enable.v2",
  "syndocal.output.ownership.arm.v2",
  "syndocal.output.blackout.release.v2",
  "syndocal.output.standby.takeover.v2",
  "syndocal.output.display.add.v2",
  "syndocal.output.video.composition.assign.v2",
  "syndocal.output.show_artnet_loopback_route.enable.v1",
  "syndocal.output.show_serial_dmx_s0_route.enable.v1",
  "syndocal.output.show_serial_dmx_s0_route.stop.v1",
  "syndocal.output.dsf2026_artnet_acceptance_probe.send.v1",
  "syndocal.output.lease.acquire.v2",
  "syndocal.output.lease.renew.v2",
  "syndocal.output.lease.recover.v2",
  "syndocal.output.lease.relinquish.v2",
  "syndocal.output.lease.force_transfer.v2",
]) {
  assert.match(controllerSource, new RegExp(operationId.replaceAll(".", "\\.")));
}
assert.doesNotMatch(
  controllerSource,
  /syndocal\.output\.(?:enable|ownership\.arm|blackout\.release|standby\.takeover|display\.add|lease\.(?:acquire|renew|recover|relinquish|force_transfer))\.v1/,
  "frontend mutating OutputControl operation IDs must not use the retired v1 boundary",
);
assert.doesNotMatch(standbySource, /challengeNotice|physical Enter|Raw Input|physical confirmation/);
assert.doesNotMatch(appSource, /displayCode|physical keyboard|prepare_output_consent|query_output_consent/);
assert.doesNotMatch(runtimeSource, /prepare_output_consent|query_output_consent|control_plane_security|consent_token/);
assert.doesNotMatch(controlPlaneSource, /prepare_output_consent|query_output_consent|PreparedPhysicalConfirmation/);
assert.doesNotMatch(registrySource, /PreparedPhysicalConfirmation|prepare_output_consent|query_output_consent/);
const nativeDangerConfirmation = runtimeSource.slice(
  runtimeSource.indexOf("fn output_action_requires_native_danger_confirmation("),
  runtimeSource.indexOf("pub(crate) fn issue_output_control_authority_for_window_label("),
);
assert.match(
  nativeDangerConfirmation,
  /MessageDialog::new\(\)[\s\S]*MessageButtons::YesNo[\s\S]*set_parent\(window\)[\s\S]*MessageDialogResult::Yes/,
  "advanced output mutations require a parented native Yes-only dialog",
);
for (const action of ["ReleaseBlackout", "Arm", "TakeOverStandby", "AddDisplay", "AssignVideoOutputComposition", "EnableShowArtNetLoopbackRoute", "EnableShowSerialDmxSafetyBlackoutRoute", "StopShowSerialDmxSafetyBlackoutRoute", "SendDsf2026ArtNetAcceptanceProbe", "AcknowledgeDsf2026ArtNetAcceptanceProbeInDoubt", "EnableShowSpoutOutputs", "ResetShowSpoutOutputs", "ForceTransferLease"]) {
  assert.match(nativeDangerConfirmation, new RegExp(`OutputControlActionV2::${action}`));
}
const outputExecution = runtimeSource.slice(
  runtimeSource.indexOf("fn execute_output_control_with_confirmation<F>("),
  runtimeSource.indexOf("pub(crate) fn exact_output_control_fence_matches("),
);
assert(
  outputExecution.indexOf("recheck_output_control_terminal")
    < outputExecution.indexOf("output_confirmation_gate"),
  "terminal replay must precede native confirmation",
);
const normalEnableDispatch = outputExecution.slice(
  outputExecution.indexOf("OutputControlActionV2::EnableOutput =>"),
  outputExecution.indexOf("OutputControlActionV2::ReleaseBlackout"),
);
assert.doesNotMatch(
  normalEnableDispatch,
  /MessageDialog|confirm_native_dangerous_output_action/,
  "normal one-click Enable must never call the native danger dialog",
);
assert.match(
  outputExecution,
  /output_confirmation_gate\([\s\S]*?store_output_control_terminal\([\s\S]*?Forbidden/,
  "native dialog cancel/close must become a terminal rejection before admission and mutation",
);
assert.match(standbySource, /data-io-control="enable-output"/);
assert.match(standbySource, /role="status" aria-live="polite">Output enabled/);
assert.match(standbySource, /hasOnlyActiveOutputLease\(leaseQuery\(\), \["lighting", "video"\]\)/);
assert.match(standbySource, /function boundedPromise<[\s\S]*STATUS_POLL_INVOKE_TIMEOUT_MS/);
assert.match(standbySource, /let pollFlight: Promise<void> \| null = null/);
assert.match(standbySource, /if \(pollFlight\) return pollWaiter \?\? pollFlight/);
assert.match(standbySource, /generation === pollGeneration/);
assert.match(standbySource, /Promise\.allSettled\(rawEndpointFlights\)/);
assert.match(standbySource, /Promise\.allSettled\(boundedEndpointFlights\)/);
assert.match(standbySource, /if \(!pollFlight && !enableMutationFlight\) void pollStatus\(\)/);
assert.match(standbySource, /let enableMutationFlight: ReturnType<typeof enableOutput> \| null = null/);
assert.match(standbySource, /let enableOutcomeUnknown = false/);
assert.match(standbySource, /beginEnableMutation[\s\S]*OUTPUT_ENABLE_TIMEOUT_MS[\s\S]*setBusy\(false\)[\s\S]*void pollStatus\(\)/);
assert.match(standbySource, /final outcome is unknown/);
assert.match(standbySource, /enableGeneration/);
assert.match(standbySource, /if \(enableOutcomeUnknown && authoritativeBothReady\)[\s\S]*setActionError\(null\)/);
assert.match(standbySource, /if \(!enableOutcomeUnknown\) setActionError\(String\(error\)\)/);
assert.doesNotMatch(
  standbySource,
  /setActionError\(String\(error\)\);\r?\n\s+await pollStatus\(\)/,
  "action catch paths must not start an unbounded second refresh",
);
assert.match(standbySource, /<details class="advancedOutputControls">/);
assert.doesNotMatch(standbySource, /<details class="advancedOutputControls"[^>]*\bopen\b/);
assert.match(standbySource, /I have fenced or disconnected every old Primary/);
assert.match(
  mainSource,
  /async fn enable_output_control_v2[\s\S]*execute_output_control_off_event_loop/,
  "normal Enable is dispatched through the off-event-loop output lane",
);
assert.match(
  mainSource,
  /async fn execute_output_control_off_event_loop[\s\S]*execute_output_control_for_operation/,
  "the off-event-loop lane reaches the canonical operation dispatcher",
);
assert.match(mainSource, /OutputControlActionV2::AddDisplay[\s\S]*vec!\[OutputLeaseResource::Lighting, OutputLeaseResource::Video\]/);
assert.match(appSource, /const setBlackout = targetBlackout.setLightingBlackout/);
assert.match(appSource, /const setAllBlackout = targetBlackout.setAllBlackout/);
assert.match(appSource, /fullscreen: target\.fullscreen/);
assert.match(appSource, /width: target\.width[\s\S]*height: target\.height/);
const setupIoFixtureStart = appSource.indexOf('if (viewportFixture === "setup-io") {');
const setupIoFixtureEnd = appSource.indexOf("const refreshDmxInputStatus", setupIoFixtureStart);
assert.notEqual(setupIoFixtureStart, -1, "the Setup I/O viewport fixture must remain explicit");
assert.notEqual(setupIoFixtureEnd, -1, "the Setup I/O viewport fixture must have a bounded source slice");
const setupIoFixtureSource = appSource.slice(setupIoFixtureStart, setupIoFixtureEnd);
assert.match(setupIoFixtureSource, /enabled: false/);
assert.match(setupIoFixtureSource, /protocol: "ArtNet" as const/);
assert.match(setupIoFixtureSource, /target_ip: "127\.0\.0\.1"/);
assert.match(setupIoFixtureSource, /port: 6454/);
assert.match(setupIoFixtureSource, /universe: 0/);
assert.match(setupIoFixtureSource, /serial_port: ""/);
assert.doesNotMatch(setupIoFixtureSource, /EnttecOpenDmx|serial_baud_rate: 250_000/,
  "the Setup I/O fixture must stage the exact Art-Net show route, not a retired serial route");
assert.match(runtimeSource, /OutputControlActionV2::EnableOutput[\s\S]*enable_output_with_output_control_fence/);
assert.match(
  runtimeSource,
  new RegExp(`OutputControlActionV2::EnableShowArtNetLoopbackRoute[\\s\\S]*${retiredShowArtNetActionKind}_with_output_control_fence`),
);
for (const [variant, routeHelper] of [
  ["EnableShowSerialDmxSafetyBlackoutRoute", "show_serial_dmx_route::enable_with_output_control_fence"],
  ["StopShowSerialDmxSafetyBlackoutRoute", "show_serial_dmx_route::stop_with_output_control_fence"],
]) {
  assert.match(
    runtimeSource,
    new RegExp(`OutputControlActionV2::${variant}[\\s\\S]*${routeHelper.replaceAll(".", "\\\\.")}`),
    `the USB-DMX ${variant} action must reach its exact fenced route helper`,
  );
}
assert.match(runtimeSource, /OutputControlActionV2::SendDsf2026ArtNetAcceptanceProbe[\s\S]*send_dsf2026_artnet_acceptance_probe_with_output_control_fence/);
assert.match(runtimeSource, /OutputControlActionV2::AcknowledgeDsf2026ArtNetAcceptanceProbeInDoubt[\s\S]*acknowledge_dsf2026_artnet_acceptance_probe_in_doubt_with_output_control_fence/);
assert.match(runtimeSource, /OutputControlActionV2::EnableShowSpoutOutputs[\s\S]*enable_show_spout_outputs_with_output_control_fence/);
assert.match(runtimeSource, /OutputControlActionV2::ResetShowSpoutOutputs[\s\S]*reset_show_spout_outputs_without_output_lease/);
assert.match(controlPlaneSource, /enable_output_control_v2/);
assert.match(controlPlaneSource, /enable_show_art_net_loopback_route_v1/);
assert.match(controlPlaneSource, /enable_show_serial_dmx_safety_blackout_route_v1/);
assert.match(controlPlaneSource, /stop_show_serial_dmx_safety_blackout_route_v1/);
assert.match(controlPlaneSource, /get_serial_dmx_machine_binding_status_v1/);
assert.match(controlPlaneSource, /get_show_serial_dmx_safety_blackout_route_status_v1/);
assert.match(controlPlaneSource, /select_serial_dmx_machine_binding_v1/);
assert.match(controlPlaneSource, /send_dsf2026_artnet_acceptance_probe_v1/);
assert.match(controlPlaneSource, /acknowledge_dsf2026_artnet_acceptance_probe_in_doubt_v1/);
assert.match(controlPlaneSource, /query_dsf2026_artnet_acceptance_probe_status_v1/);
assert.match(controlPlaneSource, /enable_show_spout_outputs_v2/);
assert.match(controlPlaneSource, /reset_show_spout_outputs_v1/);
assert.doesNotMatch(controlPlaneSource, /enable_show_spout_outputs_v1/);
assert.doesNotMatch(
  protocolCommandProductionSource,
  /show_spout_outputs\.enable\.v1/,
  "the retired V1 Spout enable route may appear only in negative tests",
);
assert.match(
  appSource,
  /<DmxOutputConfigPanel[\s\S]*output=\{output\(\)\}[\s\S]*onEnableStagedShowArtNetLoopbackRoute=\{enableStagedShowArtNetLoopbackRoute\}/,
  "the DMX workbench must pass its actual route to the fixed Art-Net show-route surface",
);
assert.match(
  appSource,
  /<DmxOutputConfigPanel[\s\S]*showSerialDmxSafetyBlackoutRouteStatus=\{showSerialDmxSafetyBlackoutRouteStatus\(\)\}[\s\S]*onEnableShowSerialDmxSafetyBlackoutRoute=\{enableShowSerialDmxSafetyBlackoutRoute\}[\s\S]*onStopShowSerialDmxSafetyBlackoutRoute=\{stopShowSerialDmxSafetyBlackoutRoute\}/,
  "the DMX workbench must render the live USB-DMX worker status and fixed actions",
);
assert.match(
  appSource,
  /const telemetryReportTimer = isTauriRuntime\(\) \? window\.setInterval\(refreshEngineTelemetryReport, 1000\) : null/,
  "the independent telemetry diagnostics cadence must remain available",
);
assert.match(
  appSource,
  /const serialDmxRuntimeStatusTimer = isTauriRuntime\(\)\s*\? window\.setInterval\(\(\) => void refreshSerialDmxRuntimeStatuses\(\), 1000\)\s*:\s*null/,
  "USB-DMX worker truth must have its own one-second poll rather than wait for telemetry",
);
assert.match(
  appSource,
  /window\.clearInterval\(serialDmxRuntimeStatusTimer\);[\s\S]*disposeSerialDmxRuntimeStatuses\(\);/,
  "unmount must stop the dedicated USB-DMX poll and invalidate late results",
);
assert.match(dmxOutputPanelSource, /output\.protocol === "ArtNet"/);
assert.match(dmxOutputPanelSource, /output\.target_ip === "127\.0\.0\.1"/);
assert.match(dmxOutputPanelSource, /output\.port === 6454/);
assert.match(dmxOutputPanelSource, /output\.serial_port === ""/);
assert.match(dmxOutputPanelSource, /output\.universe === 0/);
assert.match(dmxOutputPanelSource, /data-io-control="dmx-enable-staged-show-artnet-loopback-route"/);
assert.match(dmxOutputPanelSource, /data-io-usb-dmx-route/);
assert.match(dmxOutputPanelSource, /<strong>Enttec Open DMX<\/strong>/,
  "the routine route list must use the concise USB-DMX label");
assert.match(dmxOutputPanelSource, /Worker unavailable · S0 required/,
  "unknown USB-DMX worker state must remain visible without a prose block");
assert.match(
  dmxOutputPanelSource,
  /const hasExactMachineLocalOpenDmxIdentity[\s\S]*port\.usb_vid > 0[\s\S]*port\.usb_pid > 0[\s\S]*serial_number[\s\S]*windows_device_instance_id/,
  "a USB-DMX Confirm candidate must require a complete nonzero USB/PnP identity",
);
assert.match(
  dmxOutputPanelSource,
  /disabled=\{!hasExactMachineLocalOpenDmxIdentity\(port\)\}/,
  "enumerated rows with missing or zero VID/PID must remain diagnostic-only and disabled",
);
assert.match(
  dmxOutputPanelSource,
  /disabled=\{!hasExactMachineLocalOpenDmxIdentity\(selectedSerialPort\(\)\) \|\| !serialBindingMutationAdmissible\(\)\}/,
  "Confirm must use the same complete nonzero identity predicate and conservative route-admission gate as its option",
);
assert.match(
  dmxOutputPanelSource,
  /disabled=\{!serialWorkerArmAdmissible\(\) \|\| !serialWorkerStatusKnown\(\) \|\| !bindingReady\(\) \|\| !props\.safetyBlackoutEngaged \|\| !artNetUnityMirrorEnabled\(\)\}/,
  "USB-DMX start must fail closed while worker truth, bounded shutdown, or the exact enabled Art-Net mirror is unavailable",
);
assert.match(
  dmxOutputPanelSource,
  /disabled=\{!serialWorkerActive\(\) \|\| !serialWorkerStatusKnown\(\) \|\| !props\.safetyBlackoutEngaged\}/,
  "USB-DMX stop must not claim a stale worker can be controlled after status loss",
);
assert.match(dmxOutputPanelSource, /<dl class="dmxProtocolFacts">[\s\S]*wire U0 · 512ch · 40–44fps[\s\S]*logical U0 · ≈32\.5fps/,
  "the USB-DMX UI must keep the concise Art-Net and USB route facts distinct");
assert.match(dmxOutputPanelSource, /Detailed diagnostics are written to the application log\./,
  "verbose USB-DMX diagnostics must be directed to the application log");
assert.doesNotMatch(
  dmxOutputPanelSource,
  /generic FTDI VID\/PID is not auto-selected|queue acceptance from the bounded physical zero transaction|latest-frame mirror[\s\S]*does not promise physical delivery/,
  "the routine UI must not expose implementation prose for USB-DMX internals",
);
assert.match(dmxOutputPanelSource, /data-io-control="dmx-send-dsf2026-artnet-acceptance-probe"/);
assert.match(dmxOutputPanelSource, /probeStatus\(\)\?\.status !== "available"/,
  "a successful or unknown durable one-shot status must keep the probe disabled");
assert.match(dmxOutputPanelSource, /probeStatus\(\)\?\.status !== "in_doubt"/,
  "only a durable InDoubt outcome may enable the no-send reconciliation action");
assert.match(dmxOutputPanelSource, /Consumed · no repeat/,
  "the fixed probe UI must show the terminal no-repeat state concisely");
assert.match(
  dmxOutputPanelSource,
  /disabled=\{!exactRoute\(\) \|\| props\.output\.enabled\}/,
  "the show-route enable action must fail closed for a logical-route mismatch",
);
assert.match(dmxOutputPanelSource, /role="alert"[\s\S]*127\.0\.0\.1:6454/);
assert.doesNotMatch(
  dmxOutputPanelSource,
  /onAddDmxRoute|onRemoveDmxRoute|onApplyDmxRoute|setOutputProtocol/,
  "the show DMX panel must not reintroduce a generic route editor",
);
assert.match(ioConnectionDeckSource, /id: "dmx"[\s\S]*summary: "Output routing and optional input"/);
assert.doesNotMatch(
  ioConnectionDeckSource,
  /dmx-enable-staged-show-artnet-loopback-route/,
  "the route mutation stays in the DMX workbench, not the compact connection selector",
);
assert.match(outputDiagnosticsSource, /enableStagedShowArtNetLoopbackRoute[\s\S]*enable_show_art_net_loopback_route/);
assert.match(outputDiagnosticsSource, /enableShowSerialDmxSafetyBlackoutRoute[\s\S]*enable_show_serial_dmx_safety_blackout_route/);
assert.match(outputDiagnosticsSource, /stopShowSerialDmxSafetyBlackoutRoute[\s\S]*stop_show_serial_dmx_safety_blackout_route/);
const telemetryRefreshSource = outputDiagnosticsSource.slice(
  outputDiagnosticsSource.indexOf("const refreshEngineTelemetryReport = async () =>"),
  outputDiagnosticsSource.indexOf("const resetEngineTelemetry = async () =>"),
);
assert.doesNotMatch(
  telemetryRefreshSource,
  /refreshSerialDmxRuntimeStatuses|serialDmxStatusPoller/,
  "a hung telemetry invoke must not block USB-DMX status refreshes",
);
assert.match(
  outputDiagnosticsSource,
  /const serialDmxStatusPoller = createSerialDmxStatusPoller\(\{[\s\S]*queryBinding:[\s\S]*get_serial_dmx_machine_binding_status_v1[\s\S]*queryRoute:[\s\S]*get_show_serial_dmx_safety_blackout_route_status_v1[\s\S]*commit:/,
  "the dedicated USB-DMX poll must retrieve binding and worker truth together",
);
assert.match(
  outputDiagnosticsSource,
  /const refreshAuthoritativeSerialDmxRuntimeStatuses = async \(\) => \{[\s\S]*serialDmxStatusPoller\.invalidate\(\);[\s\S]*await refreshSerialDmxRuntimeStatuses\(\);/,
  "USB-DMX mutations must invalidate stale reads before their authoritative refresh",
);
assert.match(
  outputDiagnosticsSource,
  /const enableShowSerialDmxSafetyBlackoutRoute = async \(\) => \{[\s\S]*serialDmxStatusPoller\.invalidate\(\);[\s\S]*finally \{[\s\S]*await refreshAuthoritativeSerialDmxRuntimeStatuses\(\);/,
  "USB-DMX enable must not use a direct stale status refresh",
);
assert.match(
  outputDiagnosticsSource,
  /const stopShowSerialDmxSafetyBlackoutRoute = async \(\) => \{[\s\S]*serialDmxStatusPoller\.invalidate\(\);[\s\S]*finally \{[\s\S]*await refreshAuthoritativeSerialDmxRuntimeStatuses\(\);/,
  "USB-DMX stop must not use a direct stale status refresh",
);
assert.match(
  serialDmxStatusPollerSource,
  /SERIAL_DMX_STATUS_QUERY_TIMEOUT_MS = 1_500[\s\S]*awaitBounded[\s\S]*rawFlight[\s\S]*generation[\s\S]*disposed/,
  "the USB-DMX status poller must have a bounded, generation-fenced lifecycle",
);
assert.match(
  serialDmxStatusPollerSource,
  /Promise\.allSettled\(\[binding, route\]\)[\s\S]*if \(rawFlight\) return Promise\.resolve\(\)/,
  "a timed-out or invalidated USB-DMX UI waiter must retain one raw native invoke pair until both endpoints settle",
);
assert.match(
  serialDmxStatusPollerSource,
  /generation \+= 1;[\s\S]*commitUnknown\(\);/,
  "a status query failure must become Unknown before a late result can commit",
);
assert.match(
  serialDmxStatusPollerSource,
  /dispose: \(\) => \{[\s\S]*disposed = true;[\s\S]*generation \+= 1;[\s\S]*refreshFlight = null;/,
  "dispose must invalidate every late USB-DMX status response",
);
assert.match(
  serialDmxStatusPollerSource,
  /validateSnapshot[\s\S]*generation \+= 1;[\s\S]*commitUnknown\(\);/,
  "a malformed successful USB-DMX status response must become Unknown before it can commit",
);
assert.match(
  serialDmxStatusPollerSource,
  /minimumRouteStatusRevision[\s\S]*lastCommittedRouteStatusRevision[\s\S]*invalidateAtOrAfterRouteStatusRevision[\s\S]*setRouteStatusEventFenceAvailable/,
  "the USB-DMX poller must fence spontaneous native transitions with monotonic route revisions",
);
assert.match(
  serialDmxStatusPollerSource,
  /if \(!routeStatusEventFenceAvailable\)[\s\S]*commitUnknown\(\);[\s\S]*return Promise\.resolve\(\);/,
  "a missing native event fence must keep USB-DMX status persistently Unknown without issuing a fresh query",
);
assert.match(
  outputDiagnosticsSource,
  /validateSnapshot: validateSerialDmxStatusSnapshot[\s\S]*onInvalidSnapshot:[\s\S]*console\.warn\(`USB-DMX status payload rejected: \$\{reason\}`\)/,
  "the strict USB-DMX status validator must be wired with a redacted diagnostic reason",
);
assert.match(
  outputDiagnosticsSource,
  /coherentRouteStatusRevision:[\s\S]*binding\.routeStatusRevision === route\.routeStatusRevision[\s\S]*requireRouteStatusEventFence: true/,
  "USB-DMX binding and route reads must share a native revision fence before a status commit",
);
assert.match(
  outputDiagnosticsSource,
  /handleSerialDmxRouteStatusEvent[\s\S]*parseSerialDmxRouteStatusEventRevision[\s\S]*invalidateAtOrAfterRouteStatusRevision[\s\S]*void refreshSerialDmxRuntimeStatuses\(\)/,
  "a native USB-DMX fault event must invalidate before its independent authoritative refresh",
);
assert.match(
  serialDmxStatusValidationSource,
  /binding USB VID[\s\S]*binding USB PID[\s\S]*hasExactKeys\(route,[\s\S]*routeStatusRevision[\s\S]*workerShutdownCompleted[\s\S]*zeroFramePhysicalWriteCompleted[\s\S]*faulted USB-DMX route cannot be active/,
  "the USB-DMX status parser must reject extra fields, zero identity, and incoherent route state",
);
assert.match(
  appSource,
  /listen<unknown>\("syndocal:\/\/show-serial-dmx-route-status-v1"[\s\S]*setSerialDmxRouteStatusEventFenceAvailable\(true\)[\s\S]*\.catch\([\s\S]*setSerialDmxRouteStatusEventFenceAvailable\(false\)[\s\S]*translateUiText\("Worker status unavailable — S0 required", uiLocale\(\)\)/,
  "listener installation failure must leave USB-DMX controls in localized persistent Unknown state",
);
assert.match(
  mainSource,
  /set_show_serial_dmx_safety_blackout_route_status_observer[\s\S]*SHOW_SERIAL_DMX_ROUTE_STATUS_EVENT[\s\S]*route_status_event/,
  "the native engine revision observer must emit the exact USB-DMX route status event",
);
assert.match(
  showSerialDmxRouteSource,
  /route_status_revision: String,[\s\S]*worker_shutdown_completed: bool[\s\S]*try_show_serial_dmx_safety_blackout_route_status_snapshot/,
  "both strict USB-DMX IPC status endpoints must carry the same revisioned worker shutdown truth",
);
assert.match(
  dmxOutputPanelSource,
  /serialBindingMutationAdmissible[\s\S]*workerShutdownCompleted[\s\S]*zeroFrameQueued[\s\S]*status\.faulted && !props\.safetyBlackoutEngaged/,
  "the UI must conservatively disable machine-local replacement during live, incomplete-zero, detached, or unlatchable states",
);
assert.match(
  dmxOutputPanelSource,
  /serialWorkerArmAdmissible[\s\S]*status\.active \|\| status\.liveFrameQueued[\s\S]*workerShutdownCompleted[\s\S]*status\.faulted && !props\.safetyBlackoutEngaged[\s\S]*disabled=\{!serialWorkerArmAdmissible\(\)/,
  "the UI must prevent Arm from creating a replacement worker while shutdown is detached, while retaining only S0-latched joined-fault recovery",
);
assert.match(
  engineShowSerialDmxStatusSource,
  /fn require_show_serial_dmx_enable_admission[\s\S]*status\.active \|\| status\.live_frame_queued[\s\S]*!status\.worker_shutdown_completed[\s\S]*status\.zero_frame_queued[\s\S]*!status\.faulted[\s\S]*status\.faulted && \(!safety_authority_engaged \|\| !physical_s0_latched\)/,
  "native USB-DMX replacement admission must fence active/live, detached shutdown, nonfaulted incomplete S0, and unlatchable fault recovery",
);
const engineSerialEnableBody = engineSource.slice(
  engineSource.indexOf("fn apply_show_serial_dmx_safety_blackout_route_enable_with_sender_factory"),
  engineSource.indexOf("/// An Open-DMX worker exists at this point"),
);
assert.match(
  engineSerialEnableBody,
  /try_show_serial_dmx_route_status_snapshot[\s\S]*require_show_serial_dmx_enable_admission[\s\S]*let mut sender = create_sender/,
  "native USB-DMX admission must read authoritative status and reject before opening a replacement sender",
);
assert.match(
  engineShowSerialDmxTestsSource,
  /show_serial_dmx_stop_wedged_gate_stays_s0_faulted_and_refuses_a_replacement_worker[\s\S]*replacement_factory_called[\s\S]*did not complete bounded shutdown/,
  "the deterministic engine proof must reject a replacement while an old bounded shutdown was detached",
);
assert.match(
  engineShowSerialDmxTestsSource,
  /show_serial_dmx_joined_fault_recovery_requires_s0_then_clears_only_after_a_fresh_zero[\s\S]*fail_next_write[\s\S]*fresh B initial S0 failure[\s\S]*failed_b_status\.faulted[\s\S]*only the fresh B initial physical S0 zero may clear the joined fault/,
  "the deterministic engine proof must retain a joined fault through B initial-zero failure and clear it only after a fresh zero receipt",
);
assert.match(mainSource, /mod show_serial_dmx_route;/,
  "USB-DMX orchestration must stay separated from the oversized main runtime module");
for (const marker of [
  "fn get_machine_binding_status(",
  "fn select_machine_binding(",
  "fn route_status(",
  "async fn enable_command(",
  "async fn stop_command(",
  "fn enable_with_output_control_fence(",
  "fn stop_with_output_control_fence(",
  "never infers a protocol from",
]) {
  assert.match(showSerialDmxRouteSource, new RegExp(marker.replaceAll("(", "\\(").replaceAll(".", "\\.")),
    `the extracted USB-DMX route module must retain ${marker}`);
}
assert.match(
  protocolCommandSource,
  /#\[serde\(tag = "kind", rename_all = "snake_case", deny_unknown_fields\)\]\s*enum OutputControlActionV2Wire \{[\s\S]*EnableShowSerialDmxSafetyBlackoutRoute[\s\S]*StopShowSerialDmxSafetyBlackoutRoute/,
  "USB-DMX start/stop must use the exact deny-unknown-fields snake-case tagged union",
);
assert.match(outputDiagnosticsSource, /sendDsf2026ArtNetAcceptanceProbe[\s\S]*send_dsf2026_artnet_acceptance_probe/);
assert.match(outputDiagnosticsSource, /acknowledgeDsf2026ArtNetAcceptanceProbeInDoubt[\s\S]*acknowledge_dsf2026_artnet_acceptance_probe_in_doubt/);
assert.match(outputDiagnosticsSource, /refreshDsf2026ArtNetAcceptanceProbeStatus[\s\S]*queryDsf2026ArtNetAcceptanceProbeStatus/);
assert.match(outputDiagnosticsSource, /sendDsf2026ArtNetAcceptanceProbe[\s\S]*status\?\.status !== "available"/,
  "the controller must refuse a new probe before lease selection when its durable status is not available");
assert.match(outputDiagnosticsSource, /sendDsf2026ArtNetAcceptanceProbe[\s\S]*status: "consumed"/,
  "a successful native receipt must leave the frontend probe control permanently disabled");
assert.match(outputDiagnosticsSource, /enableShowSpoutOutputs[\s\S]*enable_show_spout_outputs/);
assert.match(outputDiagnosticsSource, /resetShowSpoutOutputs[\s\S]*reset_show_spout_outputs/);
assert.doesNotMatch(
  outputDiagnosticsSource,
  /invoke(?:<[^>]*>)?\(\s*["']set_dmx_outputs["']/,
  "the diagnostics controller must not regain a generic set_dmx_outputs mutation path",
);

// Deterministic frontend poll contract seam. This mirrors the component's
// raw-flight/UI-waiter split and keeps the timeout, overlap, and stale-result
// failures cheap to exercise in Node.
const sleep = (milliseconds) => new Promise((resolve) => setTimeout(resolve, milliseconds));
const deferred = () => {
  let resolve;
  let reject;
  const promise = new Promise((promiseResolve, promiseReject) => {
    resolve = promiseResolve;
    reject = promiseReject;
  });
  return { promise, resolve, reject };
};

// USB-DMX status lifecycle proof. The Tauri transport has no cancellation
// primitive, so the test deliberately resolves an old Active answer after a
// newer stop/fault answer and after disposal.
const serialBindingPresent = { state: "selected_and_present" };
const serialRouteActive = { active: true, faulted: false, liveFrameQueued: true };
const serialRouteStopped = { active: false, faulted: false, liveFrameQueued: false };
const serialRouteFaulted = { active: false, faulted: true, liveFrameQueued: false };
const exactSerialBindingPayload = {
  state: "selected_and_present",
  selected: {
    port_name: "COM3",
    port_type: "USB 0403:6001 USB Serial Port",
    usb_vid: 0x0403,
    usb_pid: 0x6001,
    serial_number: "SHOW-A",
    manufacturer: "FTDI",
    product: "USB Serial Port",
    windows_device_instance_id: "FTDIBUS\\A\\0000",
  },
  detail: "Exact machine-local identity is present.",
  routeStatusRevision: "41",
};
const exactSerialRoutePayload = {
  routeStatusRevision: "41",
  active: true,
  zeroFrameQueued: true,
  zeroFramePhysicalWriteCompleted: true,
  liveFrameQueued: true,
  workerShutdownCompleted: false,
  faulted: false,
  artnetMirrorLive: true,
  artnetMirrorDetail: "Exact Art-Net sender is present.",
  detail: "Open DMX worker is active.",
};
const exactSerialBindingAt = (routeStatusRevision) => ({
  ...exactSerialBindingPayload,
  routeStatusRevision,
});
const exactSerialRouteAt = (routeStatusRevision, patch = {}) => ({
  ...exactSerialRoutePayload,
  routeStatusRevision,
  ...patch,
});
const validateSerialDmxStatusSnapshot = serialDmxStatusValidationRuntime.validateSerialDmxStatusSnapshot;
assert.equal(validateSerialDmxStatusSnapshot(exactSerialBindingPayload, exactSerialRoutePayload), null,
  "the exact native USB-DMX status shape must remain admissible");
assert.match(
  validateSerialDmxStatusSnapshot(
    { ...exactSerialBindingPayload, injected: true },
    exactSerialRoutePayload,
  ),
  /missing or unexpected fields/,
  "an extra binding status field must fail closed",
);
assert.match(
  validateSerialDmxStatusSnapshot(
    {
      ...exactSerialBindingPayload,
      selected: { ...exactSerialBindingPayload.selected, usb_vid: 0 },
    },
    exactSerialRoutePayload,
  ),
  /nonzero u16/,
  "zero USB VID must not be promoted through a successful-looking status response",
);
assert.match(
  validateSerialDmxStatusSnapshot(
    {
      ...exactSerialBindingPayload,
      selected: { ...exactSerialBindingPayload.selected, usb_pid: 0 },
    },
    exactSerialRoutePayload,
  ),
  /nonzero u16/,
  "zero USB PID must not be promoted through a successful-looking status response",
);
assert.match(
  validateSerialDmxStatusSnapshot(
    exactSerialBindingPayload,
    { ...exactSerialRoutePayload, active: "true" },
  ),
  /active must be boolean/,
  "a non-boolean Active field must fail closed",
);
assert.match(
  validateSerialDmxStatusSnapshot(
    exactSerialBindingPayload,
    { ...exactSerialRoutePayload, detail: "" },
  ),
  /detail must be a nonempty string/,
  "an empty route detail must not be treated as an authoritative live status",
);
for (const malformedWorkerShutdownCompleted of ["false", 0, null]) {
  assert.match(
    validateSerialDmxStatusSnapshot(
      exactSerialBindingPayload,
      { ...exactSerialRoutePayload, workerShutdownCompleted: malformedWorkerShutdownCompleted },
    ),
    /workerShutdownCompleted must be boolean/,
    "workerShutdownCompleted must remain a strict boolean rather than a truthy IPC value",
  );
}
const { workerShutdownCompleted: omittedWorkerShutdownCompleted, ...missingWorkerShutdownCompleted } = exactSerialRoutePayload;
assert.match(
  validateSerialDmxStatusSnapshot(exactSerialBindingPayload, missingWorkerShutdownCompleted),
  /missing or unexpected fields/,
  "an omitted workerShutdownCompleted field must fail closed",
);
assert.match(
  validateSerialDmxStatusSnapshot(
    exactSerialBindingAt("42"),
    exactSerialRouteAt("41"),
  ),
  /coherent fence/,
  "binding and route status answers from different native revisions must not form a live snapshot",
);
const parseSerialDmxRouteStatusEventRevision =
  serialDmxStatusValidationRuntime.parseSerialDmxRouteStatusEventRevision;
assert.equal(parseSerialDmxRouteStatusEventRevision({ routeStatusRevision: "42" }), "42",
  "the exact native route-event schema must carry one canonical revision");
for (const malformedEvent of [
  {},
  { routeStatusRevision: "0" },
  { routeStatusRevision: 42 },
  { routeStatusRevision: "42", unexpected: true },
]) {
  assert.equal(parseSerialDmxRouteStatusEventRevision(malformedEvent), null,
    "malformed or extra native route-event payloads must not authorize a status commit");
}
const createSerialStatusPollHarness = (routeAnswers) => {
  let snapshot = { binding: null, route: null };
  const commits = [];
  let bindingInvokes = 0;
  let routeInvokes = 0;
  const poller = serialDmxStatusPollerRuntime.createSerialDmxStatusPoller({
    queryBinding: () => {
      bindingInvokes += 1;
      return Promise.resolve(serialBindingPresent);
    },
    queryRoute: () => {
      routeInvokes += 1;
      const answer = routeAnswers.shift();
      assert.notEqual(answer, undefined, "each status refresh must use an explicit fake backend answer");
      return answer;
    },
    commit: (next) => {
      snapshot = next;
      commits.push(next);
    },
    timeoutMs: 15,
  });
  return {
    poller,
    commits,
    get bindingInvokes() { return bindingInvokes; },
    get routeInvokes() { return routeInvokes; },
    get snapshot() { return snapshot; },
  };
};
const lateActiveAfterStop = deferred();
const stopHarness = createSerialStatusPollHarness([
  lateActiveAfterStop.promise,
  Promise.resolve(serialRouteStopped),
]);
await stopHarness.poller.refresh();
assert.deepEqual(stopHarness.snapshot, { binding: null, route: null },
  "a bounded status timeout must make the USB-DMX UI Unknown");
stopHarness.poller.invalidate();
await stopHarness.poller.refresh();
assert.equal(stopHarness.bindingInvokes, 1,
  "a timeout/invalidation must retain the real binding invoke until it settles");
assert.equal(stopHarness.routeInvokes, 1,
  "a timeout/invalidation must retain the real route invoke until it settles");
lateActiveAfterStop.resolve(serialRouteActive);
await sleep(0);
assert.deepEqual(stopHarness.snapshot, { binding: null, route: null },
  "late Active from before Stop must not overwrite Unknown after invalidation");
await stopHarness.poller.refresh();
assert.equal(stopHarness.bindingInvokes, 2);
assert.equal(stopHarness.routeInvokes, 2);
assert.equal(stopHarness.snapshot.route.active, false,
  "only a fresh post-Stop raw flight may show stopped worker truth");

const lateActiveAfterFault = deferred();
const faultHarness = createSerialStatusPollHarness([
  lateActiveAfterFault.promise,
  Promise.resolve(serialRouteFaulted),
]);
await faultHarness.poller.refresh();
faultHarness.poller.invalidate();
await faultHarness.poller.refresh();
assert.equal(faultHarness.bindingInvokes, 1,
  "a Fault invalidation must not overlap a pending native binding invoke");
assert.equal(faultHarness.routeInvokes, 1,
  "a Fault invalidation must not overlap a pending native route invoke");
lateActiveAfterFault.resolve(serialRouteActive);
await sleep(0);
assert.deepEqual(faultHarness.snapshot, { binding: null, route: null },
  "late Active from before Fault must not erase fail-closed Unknown");
await faultHarness.poller.refresh();
assert.equal(faultHarness.snapshot.route.faulted, true,
  "only a fresh post-Fault raw flight may show fault/S0-visible truth");

const lateBindingAfterUnmount = deferred();
const lateRouteAfterUnmount = deferred();
let unmountSnapshot = { binding: null, route: null };
const unmountPoller = serialDmxStatusPollerRuntime.createSerialDmxStatusPoller({
  queryBinding: () => lateBindingAfterUnmount.promise,
  queryRoute: () => lateRouteAfterUnmount.promise,
  commit: (next) => { unmountSnapshot = next; },
  timeoutMs: 100,
});
const unmountFlight = unmountPoller.refresh();
await sleep(0);
unmountPoller.dispose();
lateBindingAfterUnmount.resolve(serialBindingPresent);
lateRouteAfterUnmount.resolve(serialRouteActive);
await unmountFlight;
assert.deepEqual(unmountSnapshot, { binding: null, route: null },
  "late Active must not commit after App cleanup invalidates the USB-DMX poll");

const telemetryHang = deferred();
let telemetrySettled = false;
void telemetryHang.promise.finally(() => { telemetrySettled = true; });
const telemetryIndependentHarness = createSerialStatusPollHarness([
  Promise.resolve(serialRouteFaulted),
]);
await telemetryIndependentHarness.poller.refresh();
assert.equal(telemetryIndependentHarness.snapshot.route.faulted, true,
  "the dedicated USB-DMX status poll must complete while unrelated telemetry is hung");
assert.equal(telemetrySettled, false,
  "the telemetry promise remains hung; the serial status result did not await it");
telemetryHang.resolve();

// Exercise the real poller rather than the broader lifecycle harness below:
// two same-turn callers must share one binding invoke and one route invoke.
const overlapBinding = deferred();
const overlapRoute = deferred();
let overlapBindingInvokes = 0;
let overlapRouteInvokes = 0;
let overlapSnapshot = { binding: null, route: null };
const overlapPoller = serialDmxStatusPollerRuntime.createSerialDmxStatusPoller({
  queryBinding: () => {
    overlapBindingInvokes += 1;
    return overlapBinding.promise;
  },
  queryRoute: () => {
    overlapRouteInvokes += 1;
    return overlapRoute.promise;
  },
  commit: (next) => { overlapSnapshot = next; },
  timeoutMs: 100,
});
const overlapFirstRefresh = overlapPoller.refresh();
const overlapSecondRefresh = overlapPoller.refresh();
assert.strictEqual(overlapSecondRefresh, overlapFirstRefresh,
  "same-generation USB-DMX refresh callers must share the real poller flight");
await sleep(0);
assert.equal(overlapBindingInvokes, 1,
  "same-generation USB-DMX refresh must invoke binding status exactly once");
assert.equal(overlapRouteInvokes, 1,
  "same-generation USB-DMX refresh must invoke route status exactly once");
overlapBinding.resolve(serialBindingPresent);
overlapRoute.resolve(serialRouteActive);
await overlapFirstRefresh;
assert.equal(overlapSnapshot.route.active, true,
  "the shared real-poller flight must still commit its current status exactly once");

// Exercise the actual raw-flight lifetime across the UI timeout boundary.
// Tauri cannot cancel either invoke, so neither an interval tick nor an
// authoritative action refresh may dispatch a second pair until *both* old
// endpoint promises settle.
const pendingBinding = deferred();
const pendingRoute = deferred();
let rawFlightBindingInvokes = 0;
let rawFlightRouteInvokes = 0;
let rawFlightSnapshot = { binding: null, route: null };
const rawFlightPoller = serialDmxStatusPollerRuntime.createSerialDmxStatusPoller({
  queryBinding: () => {
    rawFlightBindingInvokes += 1;
    return rawFlightBindingInvokes === 1 ? pendingBinding.promise : Promise.resolve(serialBindingPresent);
  },
  queryRoute: () => {
    rawFlightRouteInvokes += 1;
    return rawFlightRouteInvokes === 1 ? pendingRoute.promise : Promise.resolve(serialRouteStopped);
  },
  commit: (next) => { rawFlightSnapshot = next; },
  timeoutMs: 15,
});
const timedOutRawFlight = rawFlightPoller.refresh();
await sleep(0);
assert.equal(rawFlightBindingInvokes, 1);
assert.equal(rawFlightRouteInvokes, 1);
await timedOutRawFlight;
rawFlightPoller.invalidate();
await rawFlightPoller.refresh();
assert.equal(rawFlightBindingInvokes, 1,
  "an invalidated poll must not overlap either still-pending native binding invoke");
assert.equal(rawFlightRouteInvokes, 1,
  "an invalidated poll must not overlap either still-pending native route invoke");
pendingBinding.resolve(serialBindingPresent);
await sleep(0);
await rawFlightPoller.refresh();
assert.equal(rawFlightBindingInvokes, 1,
  "one settled endpoint is insufficient to admit a new raw binding invoke");
assert.equal(rawFlightRouteInvokes, 1,
  "one settled endpoint is insufficient to admit a new raw route invoke");
pendingRoute.resolve(serialRouteActive);
await sleep(0);
await sleep(0);
await rawFlightPoller.refresh();
assert.equal(rawFlightBindingInvokes, 2,
  "only after both old native endpoints settle may a new binding flight start");
assert.equal(rawFlightRouteInvokes, 2,
  "only after both old native endpoints settle may a new route flight start");
assert.equal(rawFlightSnapshot.route.active, false,
  "the fresh post-invalidation flight, not a late Active response, owns the UI truth");

// The actual event-fenced poller starts Unknown until the App installs its
// native listener. A spontaneous-fault revision then invalidates an older
// in-flight Active pair before either endpoint can commit it.
const preFaultBinding = deferred();
const preFaultRoute = deferred();
let revisionBindingInvokes = 0;
let revisionRouteInvokes = 0;
let revisionSnapshot = { binding: null, route: null };
const revisionCommits = [];
const revisionPoller = serialDmxStatusPollerRuntime.createSerialDmxStatusPoller({
  queryBinding: () => {
    revisionBindingInvokes += 1;
    return revisionBindingInvokes === 1
      ? preFaultBinding.promise
      : Promise.resolve(exactSerialBindingAt("42"));
  },
  queryRoute: () => {
    revisionRouteInvokes += 1;
    return revisionRouteInvokes === 1
      ? preFaultRoute.promise
      : Promise.resolve(exactSerialRouteAt("42", {
        active: false,
        zeroFrameQueued: false,
        zeroFramePhysicalWriteCompleted: false,
        liveFrameQueued: false,
        workerShutdownCompleted: true,
        faulted: true,
        detail: "USB-DMX worker fault is latched under S0.",
      }));
  },
  commit: (next) => {
    revisionSnapshot = next;
    revisionCommits.push(next);
  },
  validateSnapshot: validateSerialDmxStatusSnapshot,
  coherentRouteStatusRevision: (binding, route) =>
    binding.routeStatusRevision === route.routeStatusRevision ? route.routeStatusRevision : null,
  requireRouteStatusEventFence: true,
  timeoutMs: 100,
});
await revisionPoller.refresh();
assert.equal(revisionBindingInvokes, 0,
  "initially unavailable native event listener must prevent any plausible Active query");
assert.equal(revisionRouteInvokes, 0);
assert.deepEqual(revisionSnapshot, { binding: null, route: null });
revisionPoller.setRouteStatusEventFenceAvailable(true);
const preFaultFlight = revisionPoller.refresh();
await sleep(0);
assert.equal(revisionBindingInvokes, 1);
assert.equal(revisionRouteInvokes, 1);
assert.equal(revisionPoller.invalidateAtOrAfterRouteStatusRevision("42"), true,
  "a valid native status event must advance the authoritative revision floor");
assert.deepEqual(revisionSnapshot, { binding: null, route: null },
  "the native fault event must visibly invalidate before the fresh status query finishes");
preFaultBinding.resolve(exactSerialBindingAt("41"));
preFaultRoute.resolve(exactSerialRouteAt("41"));
await preFaultFlight;
await sleep(0);
assert.deepEqual(revisionSnapshot, { binding: null, route: null },
  "late pre-fault Active at an older revision must not overwrite event-invalidated Unknown");
await revisionPoller.refresh();
assert.equal(revisionBindingInvokes, 2);
assert.equal(revisionRouteInvokes, 2);
assert.equal(revisionSnapshot.route.faulted, true,
  "only a fresh coherent fault/S0 status at or beyond the event fence may commit");
assert.equal(revisionSnapshot.route.routeStatusRevision, "42");
assert.equal(
  revisionCommits.some((snapshot) => snapshot.route?.active === true),
  false,
  "no stale Active status can cross the event revision fence",
);
assert.equal(revisionPoller.invalidateAtOrAfterRouteStatusRevision("not-a-revision"), false,
  "a malformed native event must retain fail-closed Unknown rather than lower the revision floor");
assert.deepEqual(revisionSnapshot, { binding: null, route: null });

// A listener that fails while an uncancellable Active pair is pending must
// remain Unknown across the late completion and must not dispatch another
// status pair until a successful listener re-establishes the event fence.
const listenerFailureBinding = deferred();
const listenerFailureRoute = deferred();
let listenerFailureBindingInvokes = 0;
let listenerFailureRouteInvokes = 0;
let listenerFailureSnapshot = { binding: null, route: null };
const listenerFailurePoller = serialDmxStatusPollerRuntime.createSerialDmxStatusPoller({
  queryBinding: () => {
    listenerFailureBindingInvokes += 1;
    return listenerFailureBindingInvokes === 1
      ? listenerFailureBinding.promise
      : Promise.resolve(exactSerialBindingAt("44"));
  },
  queryRoute: () => {
    listenerFailureRouteInvokes += 1;
    return listenerFailureRouteInvokes === 1
      ? listenerFailureRoute.promise
      : Promise.resolve(exactSerialRouteAt("44"));
  },
  commit: (next) => { listenerFailureSnapshot = next; },
  validateSnapshot: validateSerialDmxStatusSnapshot,
  coherentRouteStatusRevision: (binding, route) =>
    binding.routeStatusRevision === route.routeStatusRevision ? route.routeStatusRevision : null,
  requireRouteStatusEventFence: true,
  timeoutMs: 100,
});
listenerFailurePoller.setRouteStatusEventFenceAvailable(true);
const listenerFailureFlight = listenerFailurePoller.refresh();
await sleep(0);
assert.equal(listenerFailureBindingInvokes, 1);
assert.equal(listenerFailureRouteInvokes, 1);
listenerFailurePoller.setRouteStatusEventFenceAvailable(false);
await listenerFailurePoller.refresh();
assert.equal(listenerFailureBindingInvokes, 1,
  "listener failure must not create a second binding query while the old native promise is retained");
assert.equal(listenerFailureRouteInvokes, 1);
listenerFailureBinding.resolve(exactSerialBindingAt("43"));
listenerFailureRoute.resolve(exactSerialRouteAt("43"));
await listenerFailureFlight;
await sleep(0);
await listenerFailurePoller.refresh();
assert.deepEqual(listenerFailureSnapshot, { binding: null, route: null },
  "late Active after listener failure must remain persistent Unknown");
assert.equal(listenerFailureBindingInvokes, 1,
  "listener failure keeps polling disabled even after both old native promises settle");
assert.equal(listenerFailureRouteInvokes, 1);
listenerFailurePoller.setRouteStatusEventFenceAvailable(true);
await listenerFailurePoller.refresh();
assert.equal(listenerFailureBindingInvokes, 2,
  "only a successfully re-established listener may admit a fresh status pair");
assert.equal(listenerFailureRouteInvokes, 2);
assert.equal(listenerFailureSnapshot.route.routeStatusRevision, "44");

let malformedSnapshot = { binding: null, route: null };
const malformedReasons = [];
const malformedPoller = serialDmxStatusPollerRuntime.createSerialDmxStatusPoller({
  queryBinding: () => Promise.resolve({
    ...exactSerialBindingPayload,
    selected: { ...exactSerialBindingPayload.selected, usb_pid: 0 },
  }),
  queryRoute: () => Promise.resolve(exactSerialRoutePayload),
  validateSnapshot: validateSerialDmxStatusSnapshot,
  onInvalidSnapshot: (reason) => malformedReasons.push(reason),
  commit: (next) => { malformedSnapshot = next; },
  timeoutMs: 100,
});
await malformedPoller.refresh();
assert.deepEqual(malformedSnapshot, { binding: null, route: null },
  "a malformed successful USB-DMX status payload must commit Unknown, not Active");
assert.deepEqual(malformedReasons, ["binding USB PID must be a nonzero u16"],
  "the invalid payload diagnostic must expose only the redacted failing field");

const createEnableMutationHarness = (timeoutMs = 20) => {
  let rawMutation = null;
  let mutationCount = 0;
  let authority = "ready";
  let actionError = null;
  const boundedEnable = (factory) => new Promise((resolve, reject) => {
    let settled = false;
    const timer = setTimeout(() => {
      if (settled) return;
      settled = true;
      reject(new Error("Output enable timed out"));
    }, timeoutMs);
    Promise.resolve().then(factory).then((value) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      resolve(value);
    }, (error) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      reject(error);
    });
  });
  const enable = (factory) => {
    if (rawMutation) return Promise.reject(new Error("Output enable is still in progress"));
    mutationCount += 1;
    const raw = Promise.resolve().then(factory);
    const retained = raw.then(
      () => { if (rawMutation === retained) rawMutation = null; },
      () => { if (rawMutation === retained) rawMutation = null; },
    );
    rawMutation = retained;
    return boundedEnable(() => retained).catch((error) => {
      authority = "unavailable";
      actionError = "Output enable timed out; final outcome is unknown.";
      return error;
    });
  };
  const authoritativePoll = (state) => {
    if (state === "ReadyBoth") {
      actionError = null;
    }
  };
  return {
    enable,
    authoritativePoll,
    get authority() { return authority; },
    get actionError() { return actionError; },
    get mutationCount() { return mutationCount; },
    get rawPending() { return Boolean(rawMutation); },
  };
};
const enableMutationHarness = createEnableMutationHarness();
const enableTerminal = deferred();
await enableMutationHarness.enable(() => enableTerminal.promise);
assert.equal(enableMutationHarness.authority, "unavailable", "Enable timeout must fail closed");
assert.match(enableMutationHarness.actionError, /outcome is unknown/);
enableMutationHarness.authoritativePoll("failed");
assert.match(enableMutationHarness.actionError, /outcome is unknown/,
  "a failed authoritative poll must not clear the Enable unknown-outcome error");
enableMutationHarness.authoritativePoll("unknown");
assert.match(enableMutationHarness.actionError, /outcome is unknown/,
  "an incomplete authoritative poll must not clear the Enable unknown-outcome error");
enableMutationHarness.authoritativePoll("ReadyBoth");
assert.equal(enableMutationHarness.actionError, null,
  "only an authoritative Ready+Both poll may clear the Enable unknown-outcome error");
await assert.rejects(
  enableMutationHarness.enable(() => Promise.resolve("duplicate")),
  /still in progress/,
  "a second click must not start a duplicate mutation while the raw request is pending",
);
assert.equal(enableMutationHarness.mutationCount, 1);
enableTerminal.resolve("late-terminal");
await sleep(0);
assert.equal(enableMutationHarness.rawPending, false);
assert.equal(enableMutationHarness.authority, "unavailable",
  "a late terminal response must not directly restore enabled UI state");

for (const target of ["lighting", "video", "both"]) {
  for (const enabled of [false, true]) {
    const action = { kind: "set_blackout", target, enabled, lease: lease() };
    const harness = createHarness({ action });
    await runtime.executeTargetBlackout(harness.invoke, target, enabled);
    assert.equal(harness.executeCalls, 1);
    assert.ok(!harness.calls.some(call => /enable_output|release_blackout/.test(call.command)));
  }
}
const expectedTargetProject = {
  project_epoch: fence.project_epoch,
  project_revision: fence.project_revision,
  checkpoint_hash: fence.project_checkpoint_hash,
};
const expectedTargetHarness = createHarness({ action: { kind: "set_blackout", target: "video", enabled: true, lease: lease() } });
let targetDispatchObserved = false;
await runtime.executeTargetBlackout(
  expectedTargetHarness.invoke,
  "video",
  true,
  { expectedProject: expectedTargetProject, onMutationDispatch: () => { targetDispatchObserved = true; } },
);
assert.equal(targetDispatchObserved, true, "the dispatch marker must run only after the live authority fence matches");
const staleTargetHarness = createHarness({
  action: { kind: "set_blackout", target: "video", enabled: true, lease: lease() },
  authorityFence: {
    ...fence,
    project_revision: fence.project_revision + 1,
    project_checkpoint_hash: hash("e"),
    project_publication_generation: fence.project_publication_generation + 1,
  },
});
await assert.rejects(
  runtime.executeTargetBlackout(staleTargetHarness.invoke, "video", true, { expectedProject: expectedTargetProject }),
  /project authority changed before dispatch/,
  "a stale live project fence must reject before the native blackout command",
);
assert.equal(staleTargetHarness.executeCalls, 0);
const inactiveCalls = [];
await assert.rejects(runtime.executeTargetBlackout(async command => {
  inactiveCalls.push(command);
  return { operation_id: runtime.OUTPUT_LEASE_AUTHORITY_QUERY_OPERATION_ID, statuses: [{ status: "unavailable" }] };
}, "lighting", true), /Exactly one active output lease/);
assert.deepEqual(inactiveCalls, ["query_output_lease_authority_v1"], "blackout must not momentarily arm a previously inactive output");

for (const field of ["safety_blackout_epoch", "safety_blackout_generation", "output_epoch", "project_publication_generation"]) {
  const action = { kind: "set_blackout", target: "lighting", enabled: true, lease: lease() };
  const harness = createHarness({ action, receiptTransform(response) {
    response.receipt.fence_after[field]++;
    return response;
  } });
  await assert.rejects(runtime.executeTargetBlackout(harness.invoke, "lighting", true), /receipt was inconsistent/);
}

console.log("output control runtime contract: PASS (v8 output commands, fixed same-PC Art-Net loopback/DSF2026 probe plus no-send reconciliation/strict Spout V2 reset, revision-fenced USB-DMX status, strict receipts, fail-closed query)");
