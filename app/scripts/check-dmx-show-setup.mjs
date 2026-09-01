import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const appRoot = path.resolve(scriptDirectory, "..");
const read = (relativePath) => fs.readFileSync(path.join(appRoot, relativePath), "utf8");

const panel = read("src/components/DmxOutputConfigPanel.tsx");
const controller = read("src/createOutputDiagnosticsController.ts");
const app = read("src/App.tsx");
const nativeMain = read("src-tauri/src/main.rs");
const ownershipStart = controller.indexOf("export const isBothOutputOwnershipReady =");
const decisionStart = controller.indexOf("export const decideBothOutputLeasePreparation =");
assert.ok(ownershipStart >= 0 && decisionStart > ownershipStart,
  "stage-1 ownership readiness helper must precede the lease decision");
const ownershipSource = controller
  .slice(ownershipStart, decisionStart)
  .trim()
  .replace(/^export const /, "const ")
  .replace(/status: OutputOwnershipStatus/g, "status")
  .replace(/: boolean =>/g, " =>");
const isBothOutputOwnershipReady = new Function(
  `${ownershipSource}; return isBothOutputOwnershipReady;`,
)();
const decisionEnd = controller.indexOf("\n\nexport const outputProtocolLabel", decisionStart);
assert.ok(decisionStart >= 0 && decisionEnd > decisionStart,
  "stage-1 lease decision helper must have a bounded source body");
const decisionSource = controller
  .slice(decisionStart, decisionEnd)
  .replace(/^export const /, "const ")
  .replace(/query: OutputLeaseAuthorityQuery/g, "query")
  .replace(/: BothOutputLeasePreparationDecision =>/g, " =>");
const decideBothOutputLeasePreparation = new Function(
  "selectOnlyActiveOutputLease",
  `${decisionSource}; return decideBothOutputLeasePreparation;`,
)(
  (query, expectedResources) => {
    const match = query.statuses.find((status) =>
      status.status === "held_active"
      && status.resources.length === expectedResources.length
      && status.resources.every((resource, index) => resource === expectedResources[index]),
    );
    assert.ok(match, "the deterministic lease fixture must contain the requested active lease");
    return { ...match.authority };
  },
);
const preparationStart = controller.indexOf("const prepareShowDmx =");
const preparationEnd = controller.indexOf("const stopShowSerialDmxSafetyBlackoutRoute =", preparationStart);
assert.ok(preparationStart >= 0 && preparationEnd > preparationStart, "quick setup controller must have a bounded preparation function");
const preparation = controller.slice(preparationStart, preparationEnd);

assert.match(panel, /data-io-show-dmx-setup/);
assert.match(panel, /data-io-control="dmx-prepare-show-dmx"/);
assert.match(panel, /onPrepareShowDmx\(selected\)/);
assert.match(panel, /showDmxPreparationBusy/);
assert.match(panel, /showDmxPreparationStage/);
assert.match(panel, /hasExactMachineLocalOpenDmxIdentity\(selectedSerialPort\(\)\)/);

const diagnosticsDisclosure = panel.indexOf('data-io-disclosure="dmx-individual-diagnostics"');
const oldLoopbackControl = panel.indexOf('data-io-control="dmx-enable-staged-show-artnet-loopback-route"');
const oldUsbControl = panel.indexOf('data-io-control="dmx-enable-show-serial-dmx-safety-blackout-route"');
assert.ok(diagnosticsDisclosure >= 0 && diagnosticsDisclosure < oldLoopbackControl, "individual loopback diagnostics must remain behind the disclosure");
assert.ok(oldLoopbackControl < oldUsbControl, "individual Art-Net diagnostics must remain before USB diagnostics");
assert.match(panel, /<details class="ioDisclosure dmxIndividualDiagnostics"[^>]*>/);
assert.doesNotMatch(panel, /<details class="ioDisclosure dmxIndividualDiagnostics"[^>]*\bopen\b/,
  "individual diagnostics must default collapsed behind the quick setup");
assert.match(panel, /disabled=\{!showDmxPreparationCanStart\(\)\}/);
assert.match(panel, /&& !serialWorkerFaulted\(\)[\s\S]*&& serialWorkerArmAdmissible\(\)/,
  "quick setup preflight must reject faulted, active, live-frame, incomplete-shutdown, or pending-zero workers");
assert.doesNotMatch(panel, /Prepare show DMX \(S0-safe\)/,
  "the complete sequence must not claim S0 safety before loopback staging");

assert.match(controller, /createSafetyBlackoutRuntimeController/);
assert.match(controller, /import[\s\S]*enableOutput/,
  "quick setup must use the canonical normal output enable path");
assert.match(controller, /const ensureBothOutputLease = async \(\) =>/);
assert.match(controller, /export const decideBothOutputLeasePreparation =/,
  "stage-1 lease handling must expose a deterministic active-vs-enable decision");
assert.match(controller, /const decision = decideBothOutputLeasePreparation\(query\)/,
  "stage-1 must use the authoritative active-vs-enable decision");
assert.match(controller, /if \(decision\.action === "reuse"\) \{[\s\S]*?return decision;/,
  "an active Both lease must be returned for reuse without another ownership action");
assert.match(controller, /invoke<OutputOwnershipStatus>\("get_output_ownership_status"\)/,
  "stage 1 must correlate the active Both lease with current output ownership");
assert.match(controller, /if \(!isBothOutputOwnershipReady\(ownership\)\) \{[\s\S]*?throw new Error/,
  "active-but-unarmed ownership must stop before device or route mutation");
assert.match(controller, /await enableOutput\(options\.invoke\)/,
  "fresh no-lease setup must acquire Both through enable_output");
assert.match(controller, /lease: await selectBothOutputLease\(\)/,
  "fresh enable must be followed by an exact Both re-query");
assert.match(controller, /const selectFreshBothOutputLease = async \(\) =>/,
  "physical route actions must have a dedicated fresh authority/lease read");
assert.match(controller, /await options\.invoke<unknown>\("query_output_control_authority_v1"\)/,
  "fresh route lease selection must refresh output-control authority first");
for (const marker of [
  "showDmxPreparationPromise",
  "setShowDmxPreparationBusy(true)",
  'runStage("1/4 output role Both"',
  'runStage("2/4 machine-local binding"',
  "confirmSerialDmxMachineBindingInternal",
  'runStage("3/4 Art-Net loopback"',
  "enableStagedShowArtNetLoopbackRouteInternal",
  'runStage("4/4 S0 + Open DMX arm"',
  "safetyBlackoutRuntime.engage()",
  "enableShowSerialDmxSafetyBlackoutRouteInternal",
  "setShowDmxPreparationBusy(false)",
]) {
  assert.ok(preparation.includes(marker), `quick setup must retain ${marker}`);
}
assert.match(preparation, /await ensureBothOutputLease\(\);/,
  "stage 1 must complete through the lease preparation helper");
const stage1Start = preparation.indexOf('runStage("1/4 output role Both"');
const stage2Start = preparation.indexOf('runStage("2/4 machine-local binding"', stage1Start);
assert.ok(stage1Start >= 0 && stage2Start > stage1Start, "stage 1 and stage 2 boundaries must remain ordered");
const stage1Block = preparation.slice(stage1Start, stage2Start);
assert.doesNotMatch(stage1Block, /kind:\s*"arm"/,
  "an active Both lease must satisfy stage 1 without a redundant Arm mutation");
assert.doesNotMatch(stage1Block, /selectFreshBothOutputLease\(\)/,
  "stage 1 must not rotate/read a second lease merely to issue a redundant Arm");
assert.doesNotMatch(preparation, /prepared\.enabled/,
  "stage 1 must not interpret an existing lease as a request to issue Arm");

const orderedStages = [
  'runStage("1/4 output role Both"',
  'runStage("2/4 machine-local binding"',
  'runStage("3/4 Art-Net loopback"',
  'runStage("4/4 S0 + Open DMX arm"',
].map((marker) => preparation.indexOf(marker));
assert.ok(orderedStages.every((index) => index >= 0));
for (let index = 1; index < orderedStages.length; index += 1) {
  assert.ok(orderedStages[index - 1] < orderedStages[index], "quick setup stages must be ordered");
}
const stage3Block = preparation.slice(orderedStages[2], orderedStages[3]);
assert.match(stage3Block, /const lease = await selectFreshBothOutputLease\(\)/,
  "stage 3 must requery the output-control authority and exact Both generation after stages 1/2");
assert.match(stage3Block, /await enableStagedShowArtNetLoopbackRouteInternal\(lease\)/,
  "stage 3 must execute with the freshly selected Both lease, not a pre-stage lease");
const serialEnableStart = controller.indexOf("const enableShowSerialDmxSafetyBlackoutRouteInternal = async () =>");
const serialEnableEnd = controller.indexOf("const enableShowSerialDmxSafetyBlackoutRoute = async () =>", serialEnableStart);
assert.ok(serialEnableStart >= 0 && serialEnableEnd > serialEnableStart,
  "USB arm helper must remain bounded for fresh authority auditing");
assert.match(controller.slice(serialEnableStart, serialEnableEnd), /selectFreshBothOutputLease\(\)/,
  "stage 4 USB arm must also select the current Both generation after S0");
const artNetCommitStart = nativeMain.indexOf("fn enable_show_artnet_loopback_route_with_output_control_fence(");
const artNetCommitEnd = nativeMain.indexOf("fn enable_show_spout_outputs_with_output_control_fence(", artNetCommitStart);
assert.ok(artNetCommitStart >= 0 && artNetCommitEnd > artNetCommitStart,
  "the native Art-Net project mutation must remain a bounded output-control function");
const artNetCommit = nativeMain.slice(artNetCommitStart, artNetCommitEnd);
assert.match(artNetCommit, /reconcile_project_checkpoint_for_coordinator/,
  "stage 3 must reconcile the project checkpoint after enabling the authored Art-Net route");
assert.match(artNetCommit, /committed_show_artnet_loopback_route_fence/,
  "stage 3 must return the committed post-route project fence to stage 4");
assert.match(nativeMain, /fn committed_show_artnet_loopback_route_fence\([\s\S]*committed_output_control_fence/,
  "the Stage 3 seam must preserve the exact committed fence constructor");
assert.doesNotMatch(artNetCommit, /physical activation only:[\s\S]*expected_fence\.clone\(\)/,
  "stage 3 must not conceal its persisted route mutation behind the admitted fence");
assert.doesNotMatch(preparation, /release_blackout|releaseBlackout/,
  "quick setup must never clear S0");
assert.doesNotMatch(preparation, /enableShowSpoutOutputs|resetShowSpoutOutputs/,
  "quick setup must not alter the independent Spout route");

assert.match(app, /showDmxPreparationBusy=\{showDmxPreparationBusy\(\)\}/);
assert.match(app, /showDmxPreparationStage=\{showDmxPreparationStage\(\)\}/);
assert.match(app, /onPrepareShowDmx=\{prepareShowDmx\}/);

assert.match(panel, /const explicitKey = selectedSerialPortKey\(\)/);
assert.match(panel, /persistedBindingKey/);
assert.match(panel, /const eligible = props\.serialPorts\.filter\(\(port\) => hasExactMachineLocalOpenDmxIdentity\(port\)\)/);
assert.match(panel, /eligible\.length === 1 \? eligible\[0\] : undefined/);
assert.match(panel, /data-io-usb-dmx-selection-state/);
assert.match(panel, /value=\{selectedSerialPort\(\) \? serialPortKey\(selectedSerialPort\(\)!\) : ""\}/);

const activeBothAuthority = { lease_id: "lease-0123456789abcdef", generation: 31 };
assert.deepEqual(
  decideBothOutputLeasePreparation({
    operation_id: "syndocal.output.lease.authority.query.v1",
    statuses: [{
      status: "held_active",
      authority: activeBothAuthority,
      resources: ["lighting", "video"],
    }],
  }),
  { action: "reuse", lease: activeBothAuthority },
  "an exact active Both lease must be a deterministic no-Arm reuse decision",
);
assert.deepEqual(
  decideBothOutputLeasePreparation({
    operation_id: "syndocal.output.lease.authority.query.v1",
    statuses: [{ status: "unavailable" }],
  }),
  { action: "enable" },
  "no active lease must take the canonical atomic enable_output path",
);
assert.deepEqual(
  decideBothOutputLeasePreparation({
    operation_id: "syndocal.output.lease.authority.query.v1",
    statuses: [{
      status: "held_orphaned",
      authority: activeBothAuthority,
      resources: ["lighting", "video"],
    }],
  }),
  { action: "enable" },
  "an orphaned lease must not be treated as an active stage-1 completion",
);
assert.throws(
  () => decideBothOutputLeasePreparation({
    operation_id: "syndocal.output.lease.authority.query.v1",
    statuses: [
      { status: "held_active", authority: activeBothAuthority, resources: ["lighting", "video"] },
      { status: "held_active", authority: { lease_id: "lease-fedcba9876543210", generation: 4 }, resources: ["lighting", "video"] },
    ],
  }),
  /ambiguous or has the wrong resources/,
  "multiple active Both leases must fail closed",
);
assert.throws(
  () => decideBothOutputLeasePreparation({
    operation_id: "syndocal.output.lease.authority.query.v1",
    statuses: [
      { status: "held_active", authority: activeBothAuthority, resources: ["lighting", "video"] },
      { status: "held_orphaned", authority: { lease_id: "lease-fedcba9876543210", generation: 4 }, resources: ["lighting", "video"] },
    ],
  }),
  /ambiguous or has the wrong resources/,
  "an active-plus-orphaned query must fail closed",
);
assert.throws(
  () => decideBothOutputLeasePreparation({
    operation_id: "syndocal.output.lease.authority.query.v1",
    statuses: [{ status: "held_active", authority: activeBothAuthority, resources: ["lighting"] }],
  }),
  /ambiguous or has the wrong resources/,
  "wrong-resource authority must fail closed",
);

const readyBothOwnership = {
  state: "Ready",
  effective_role: "Both",
  desired_role: "Both",
  lighting_allowed: true,
  video_allowed: true,
};
assert.equal(isBothOutputOwnershipReady(readyBothOwnership), true,
  "only Ready/Both ownership may reuse an active Both lease");
for (const ownership of [
  { ...readyBothOwnership, state: "Failed", lighting_reason: "StartupDenied" },
  { ...readyBothOwnership, effective_role: "Standby", lighting_reason: "ProjectSwapDisarmed" },
  { ...readyBothOwnership, desired_role: "Standby" },
  { ...readyBothOwnership, lighting_allowed: false },
  { ...readyBothOwnership, video_allowed: false },
]) {
  assert.equal(isBothOutputOwnershipReady(ownership), false,
    "non-Ready/non-Both/blocked ownership must stop before later show-DMX stages");
}

console.log("DMX show setup UI contract: PASS (loopback-before-S0 boundary, exact device selection, singleflight, individual diagnostics disclosure)");
