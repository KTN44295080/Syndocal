import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const source = (file) => readFile(new URL(`../src/${file}`, import.meta.url), "utf8");
const dataUrl = (text) => `data:text/javascript;base64,${Buffer.from(text).toString("base64")}`;
const moduleUrl = async (file, replacements = {}) => {
  const compiled = ts.transpileModule(await source(file), {
    fileName: file,
    compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
    reportDiagnostics: true,
  });
  assert.equal(compiled.diagnostics?.filter((item) => item.category === ts.DiagnosticCategory.Error).length, 0);
  let javascript = compiled.outputText;
  for (const [specifier, url] of Object.entries(replacements)) javascript = javascript.replaceAll(`from "${specifier}"`, `from "${url}"`);
  return dataUrl(javascript);
};
const networkUrl = await moduleUrl("networkDmxRoutes.ts");
const controller = await import(await moduleUrl("outputControlController.ts", { "./networkDmxRoutes": networkUrl }));
const policy = await import(await moduleUrl("videoOutputAuthoring.ts"));
const { translateUiText } = await import(await moduleUrl("uiLocalization.ts"));
const backends = ["ndi", "spout", "syphon"].map((id) => ({ id, state: "Available" }));
const spec = { kind: "SpoutSender", label: "映像出力", width: 1920, height: 1080, endpoint_name: "Syndocal", composition_id: 1 };
for (const kind of ["SpoutSender", "NdiSender", "SyphonServer"]) {
  assert.equal(policy.isVideoOutputKindAvailable(kind, backends), true);
  assert.equal(policy.isVideoOutputKindAvailable(kind, undefined), false);
  assert.equal(policy.isVideoOutputKindAvailable(kind, backends.map((b) => ({ ...b, state: "Unavailable" }))), false);
  assert.doesNotThrow(() => policy.validateVideoOutputAuthoringSpec({ ...spec, kind }, [], backends));
}
assert.throws(() => policy.validateVideoOutputAuthoringSpec(spec, [], []), /unavailable/);
assert.throws(() => policy.validateVideoOutputAuthoringSpec({ ...spec, label: "あ".repeat(43) }, [], backends), /128/);
assert.throws(() => policy.validateVideoOutputAuthoringSpec({ ...spec, label: "path/name" }, [], backends), /slashes/);
for (const width of [0, 8193, 2.5, NaN]) assert.throws(() => policy.validateVideoOutputAuthoringSpec({ ...spec, width }, [], backends), /whole-number/);
for (const endpoint_name of ["", " untrimmed", "a\nb", "あ".repeat(86)]) assert.throws(() => policy.validateVideoOutputAuthoringSpec({ ...spec, endpoint_name }, [], backends), /255/);
const existing = { ...spec, id: 10, endpoint_name: "SYNDOCAL" };
assert.throws(() => policy.validateVideoOutputAuthoringSpec(spec, [existing], backends), /already uses/);
assert.doesNotThrow(() => policy.validateVideoOutputAuthoringSpec(spec, [existing], backends, 10));
assert.doesNotThrow(() => policy.validateVideoOutputAuthoringSpec({ ...spec, kind: "NdiSender" }, [existing], backends));
const display = { kind: "Display", label: "Display", width: 1920, height: 1080, fullscreen: true, monitor_index: 1, monitor_identity: "d".repeat(64), endpoint_name: null };
assert.throws(() => policy.validateVideoOutputAuthoringSpec(display, [{ id: 12, kind: "Display", monitor_id: 1 }], backends), /already uses/);
assert.throws(() => policy.validateVideoOutputAuthoringSpec(display, [{ id: 12, kind: "Display", monitor_id: 2, monitor_identity: display.monitor_identity }], backends), /already uses/);

const initialFence = () => ({
  process_incarnation: 1, session_incarnation: 1, project_epoch: 2, project_revision: 3,
  project_checkpoint_hash: "a".repeat(64), project_publication_generation: 4,
  output_epoch: 1, output_generation: 1, safety_blackout_epoch: 1, safety_blackout_generation: 1,
});
const intents = [
  { kind: "add_external_output", spec },
  { kind: "configure_video_output", output_id: 9, spec: { ...spec, composition_id: undefined, fullscreen: false, monitor_index: null, monitor_identity: null } },
  { kind: "remove_video_output", output_id: 9 },
];
delete intents[1].spec.composition_id;
const routes = {
  add_external_output: ["add_external_output_v2", controller.OUTPUT_EXTERNAL_ADD_OPERATION_ID],
  configure_video_output: ["configure_video_output_v2", controller.OUTPUT_VIDEO_CONFIGURE_OPERATION_ID],
  remove_video_output: ["remove_video_output_v2", controller.OUTPUT_VIDEO_REMOVE_OPERATION_ID],
};

function machine(options = {}) {
  let fence = initialFence();
  let phase = options.phase ?? "held_active";
  let generation = 1;
  let resources = options.resources ?? ["lighting", "video"];
  let nextRequest = 40;
  let audit = 10;
  let committed = 0;
  let lost = false;
  let projectDrifted = false;
  const calls = [], replay = new Map();
  const authority = () => ({ lease_id: "lease-0000000000000001", generation });
  const invoke = async (command, args) => {
    calls.push({ command, args });
    if (command === "query_output_control_authority_v1") return { operation_id: controller.OUTPUT_CONTROL_AUTHORITY_QUERY_OPERATION_ID, fence: { ...fence } };
    if (command === "query_output_lease_authority_v1") {
      if (options.driftOnFirstLeaseQuery && !projectDrifted) {
        projectDrifted = true;
        fence = { ...fence, ...options.driftOnFirstLeaseQuery };
      }
      return {
        operation_id: controller.OUTPUT_LEASE_AUTHORITY_QUERY_OPERATION_ID,
        statuses: phase === "unavailable" ? [{ status: "unavailable" }] : [{ status: phase, authority: authority(), resources: [...resources] }],
      };
    }
    if (command === "allocate_output_control_request_id_v1") return ++nextRequest;
    const request = args.request;
    if (replay.has(request.request_id)) return replay.get(request.request_id);
    assert.ok(Object.isFrozen(request));
    assert.deepEqual(request.expected_fence, fence);
    const action = request.action;
    const before = { ...fence };
    let outcome = "authorized", beforePhase = phase, beforeGeneration = generation, beforeResources = [...resources];
    if (action.kind === "acquire_lease") {
      assert.equal(command, "acquire_output_lease_v2");
      assert.equal(action.role, "both", "fresh setup retains Display's exact-Both authority without arming");
      beforePhase = null; beforeGeneration = null; beforeResources = [];
      resources = ["lighting", "video"]; phase = "held_active"; outcome = "acquired";
      fence.output_generation += 1;
    } else if (action.kind === "recover_lease") {
      assert.equal(command, "recover_output_lease_v2");
      phase = "held_active"; generation += 1; outcome = "recovered"; fence.output_generation += 1;
    } else {
      assert.equal(command, routes[action.kind][0]);
      assert.equal(request.operation_id, routes[action.kind][1]);
      assert.equal(action.lease.generation, generation);
      if (options.reject) return { type: "rejected", rejection: { operation_id: request.operation_id, request_id: request.request_id, error: options.reject } };
      if (!options.noOp) {
        fence.project_revision += options.badFence ? 2 : 1;
        fence.project_publication_generation += 1;
        fence.project_checkpoint_hash = fence.project_checkpoint_hash === "a".repeat(64) ? "b".repeat(64) : "a".repeat(64);
        committed += 1;
      }
    }
    const response = { type: "receipt", receipt: {
      operation_id: request.operation_id, request_id: request.request_id,
      shape_sha256: "1".repeat(64), argument_fingerprint: "2".repeat(64), audit_sequence: ++audit,
      fence_before: before, fence_after: { ...fence }, outcome: options.noOp && routes[action.kind] ? "no_op" : "applied",
      lease_result: {
        authority: authority(), resources: [...resources], phase, outcome, audit_sequence: audit,
        changes: [{ lease_id: authority().lease_id, before_generation: beforeGeneration, after_generation: generation,
          before_resources: beforeResources, after_resources: [...resources], before_phase: beforePhase, after_phase: phase }],
      },
    } };
    replay.set(request.request_id, response);
    if (options.loseFirstReply && routes[action.kind] && !lost) { lost = true; throw new Error("reply lost"); }
    return response;
  };
  return { invoke, calls, authority, committed: () => committed };
}

for (const intent of intents) {
  for (const resources of [["video"], ["lighting", "video"]]) {
    const backend = machine({ resources });
    const receipt = await controller.executeVideoOutputAuthoringControl(backend.invoke, intent);
    assert.equal(receipt.operation_id, routes[intent.kind][1]);
    assert.equal(receipt.fence_after.project_revision, 4);
    assert.equal(receipt.fence_after.output_generation, 1);
    assert.equal(backend.committed(), 1);
    assert.equal(backend.calls.filter((call) => call.command === routes[intent.kind][0]).length, 1);
  }
  const noop = machine({ noOp: true });
  assert.equal((await controller.executeVideoOutputAuthoringControl(noop.invoke, intent)).outcome, "no_op");
  assert.equal(noop.committed(), 0);
  for (const reject of ["stale_fence", "invalid_request", "forbidden", "busy"]) {
    const backend = machine({ reject });
    await assert.rejects(controller.executeVideoOutputAuthoringControl(backend.invoke, intent), new RegExp(reject));
    assert.equal(backend.committed(), 0);
  }
  const corrupt = machine({ badFence: true });
  await assert.rejects(controller.executeVideoOutputAuthoringControl(corrupt.invoke, intent), /inconsistent/);
  const stale = machine();
  await assert.rejects(controller.executeVideoOutputAuthoringControl(stale.invoke, intent, {
    project_epoch: 99, project_revision: 3, checkpoint_hash: "a".repeat(64),
  }), /authority changed/);
  assert.equal(stale.committed(), 0);
  assert.equal(stale.calls.some((call) => call.command === routes[intent.kind][0]), false);
}
const preparedAuthoringProject = {
  project_epoch: initialFence().project_epoch,
  project_revision: initialFence().project_revision,
  checkpoint_hash: initialFence().project_checkpoint_hash,
};
for (const phase of ["unavailable", "held_orphaned"]) {
  const backend = machine({ phase });
  await controller.executeVideoOutputAuthoringControl(backend.invoke, intents[0], preparedAuthoringProject);
  assert.ok(backend.calls.some((call) => call.command === (phase === "unavailable" ? "acquire_output_lease_v2" : "recover_output_lease_v2")));
  assert.equal(backend.calls.some((call) => /arm_output|enable_output|force_transfer|relinquish/.test(call.command)), false,
    "authoring does not alter the current output role or request an extra confirmation");
  for (const driftOnFirstLeaseQuery of [
    { project_epoch: initialFence().project_epoch + 1 },
    { project_revision: initialFence().project_revision + 1 },
    { project_checkpoint_hash: "f".repeat(64) },
  ]) {
    const staleSetup = machine({ phase, driftOnFirstLeaseQuery });
    await assert.rejects(
      controller.executeVideoOutputAuthoringControl(staleSetup.invoke, intents[0], preparedAuthoringProject),
      /project authority changed before dispatch/,
    );
    assert.deepEqual(staleSetup.calls.map((call) => call.command), [
      "query_output_lease_authority_v1", "query_output_control_authority_v1",
    ], `${phase}: a project change after intent preparation must stop before lease mutation or request allocation`);
    assert.equal(staleSetup.committed(), 0);
  }
}
const retry = machine({ loseFirstReply: true });
await controller.executeVideoOutputAuthoringControl(retry.invoke, intents[0]);
assert.equal(retry.committed(), 1);
const dispatches = retry.calls.filter((call) => call.command === "add_external_output_v2");
assert.equal(dispatches.length, 2);
assert.equal(dispatches[0].args, dispatches[1].args, "a lost reply retries one immutable request identity");
for (const invalid of [
  { ...intents[0], spec: { ...spec, enabled: true } },
  { ...intents[0], spec: { ...spec, kind: "Display" } },
  { ...intents[0], spec: { ...spec, width: 0 } },
  { ...intents[1], output_id: 0 },
  { ...intents[2], output_id: -1 },
]) {
  const backend = machine();
  await assert.rejects(controller.executeOutputControl(backend.invoke, { ...invalid, lease: backend.authority() }));
  assert.equal(backend.calls.length, 0, "malformed actions fail before authority or transport calls");
}
const restartRecoveryMessage = "The output change could not be verified. Restart Syndocal and use Recovery before editing outputs again.";
for (const uncertain of [
  new Error("OutputControl rejected (publication_failed); physical output state is unknown."),
  new Error("OutputControl rejected (publication_failed); lease authority expired before publication."),
  new Error("Video output authoring is in doubt; pending receipt retained: lease authority expired during sender startup"),
  "Video output authoring is in doubt; pending receipt retained: project authority changed while ownership was busy",
]) {
  assert.equal(policy.videoOutputAuthoringErrorMessage(uncertain), restartRecoveryMessage,
    "retained publication failures must request restart/Recovery before nested lease or authority fallbacks");
}
assert.equal(policy.videoOutputAuthoringErrorMessage(new Error("forbidden: lease authority is unavailable")),
  "Video output ownership is unavailable. Refresh output ownership and try again.",
  "ordinary ownership refusal keeps its existing recovery guidance");
assert.match(translateUiText(restartRecoveryMessage, "ja"), /再起動/);
assert.match(translateUiText(restartRecoveryMessage, "ja"), /Recovery（復旧）/);
for (const value of [
  restartRecoveryMessage,
  "Video output added.", "Video output updated.", "Video output removed. Undo is available.",
  "Remove output", "Removing an output can be undone.", "NDI output is unavailable in this build.",
  "Spout output is available on Windows only.", "New outputs follow the current output state and may start sending immediately.",
  ...["stale_fence", "invalid_request", "forbidden", "busy", "reply lost"].map((value) => policy.videoOutputAuthoringErrorMessage(new Error(value))),
]) {
  assert.equal(translateUiText(value, "en"), value);
  assert.match(translateUiText(value, "ja"), /[\p{Script=Hiragana}\p{Script=Katakana}\p{Script=Han}]/u, value);
}
await import("./check-syphon-output-ui.mjs");
console.log("Output authoring UI: PASS (three canonical flows, capabilities, exact DTOs/fences, stale rejection, ownership, replay, JA/EN; no native output started)");
