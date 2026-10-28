import assert from "node:assert/strict";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { createRequire } from "node:module";
import ts from "typescript";
import { typescriptFixtureModuleUrl } from "./typescript-fixture-module.mjs";
const networkPolicy = await import(await typescriptFixtureModuleUrl(new URL("../src/networkDmxRoutes.ts", import.meta.url)));

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const appRoot = path.resolve(scriptDirectory, "..");
const read = (relativePath) => fs.readFileSync(path.join(appRoot, relativePath), "utf8");

const panel = read("src/components/DmxOutputConfigPanel.tsx");
const outputDiagnosticsPanel = read("src/components/OutputDiagnosticsPanel.tsx");
const styles = read("src/styles.css");
const controller = read("src/createOutputDiagnosticsController.ts");
const setupIoView = read("src/components/SetupIoWorkspaceView.tsx");
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
const decisionEnd = controller.indexOf("export const outputProtocolLabel", decisionStart);
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

// Task 74 replaces fixed U0 + Prepare/manual controls with independent drafts
// committed by one Apply. Legacy diagnostics/controller authority tests below
// remain required; their retained paths are not the everyday operator surface.
assert.match(panel, /data-dmx-universe-table/);
assert.match(panel, /data-usb-dmx-cables/);
assert.match(panel, /data-network-dmx-routes/);
assert.match(panel, /data-dmx-universe-apply/);
assert.match(panel, /onApplyUniverseTable\(plan\(\)\)/);
assert.match(panel, /validateUniverseTablePlan/);
assert.match(panel, /conflict\(\)/);
assert.doesNotMatch(panel, /onPrepareShowDmx|dmx-prepare-show-dmx|dmxShowSetup|Manual controls|S0 engaged/);
assert.match(setupIoView, /onApplyUniverseTable=\{applyDmxOutputRoutes\}/);
assert.doesNotMatch(setupIoView, /onPrepareShowDmx=/);
assert.match(outputDiagnosticsPanel, /data-io-dmx-protocol-diagnostics[\s\S]*data-io-control="dmx-send-dsf2026-artnet-acceptance-probe"[\s\S]*data-io-control="dmx-acknowledge-dsf2026-artnet-acceptance-probe-in-doubt"/);
assert.doesNotMatch(controller, /safetyBlackoutRuntime\.engage|executeBlackoutRelease/,
  "ordinary USB preparation must preserve the operator's global safety state");
assert.match(controller, /import[\s\S]*enableOutput/,
  "quick setup must use the canonical normal output enable path");
assert.match(controller, /const ensureBothOutputLease = async \(expectedProject = options\.captureProjectAuthorityIdentity\(\)\) =>/);
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
assert.match(controller, /await enableOutput\(options\.invoke, expectedProject\)/,
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
  'runStage("2/4 optional machine-local binding"',
  "confirmSerialDmxMachineBindingInternal",
  'runStage("3/4 Art-Net loopback"',
  "enableStagedShowArtNetLoopbackRouteInternal",
  'runStage("4/4 optional Open DMX arm"',
  "No USB-DMX device was selected",
  "enableShowSerialDmxSafetyBlackoutRouteInternal",
  "waitForShowSerialDmxLive",
  "setShowDmxPreparationBusy(false)",
]) {
  assert.ok(preparation.includes(marker), `quick setup must retain ${marker}`);
}
assert.match(preparation, /await ensureBothOutputLease\(\);/,
  "stage 1 must complete through the lease preparation helper");
const stage1Start = preparation.indexOf('runStage("1/4 output role Both"');
const stage2Start = preparation.indexOf('runStage("2/4 optional machine-local binding"', stage1Start);
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
  'runStage("2/4 optional machine-local binding"',
  'runStage("3/4 Art-Net loopback"',
  'runStage("4/4 optional Open DMX arm"',
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
const serialEnableStart = controller.indexOf("const enableShowSerialDmxSafetyBlackoutRouteInternal = async (");
const serialEnableEnd = controller.indexOf("const enableShowSerialDmxSafetyBlackoutRoute = async () =>", serialEnableStart);
assert.ok(serialEnableStart >= 0 && serialEnableEnd > serialEnableStart,
  "USB arm helper must remain bounded for fresh authority auditing");
assert.match(controller.slice(serialEnableStart, serialEnableEnd), /selectFreshBothOutputLease\(\)/,
  "stage 4 USB arm must also select the current Both generation after S0");
assert.match(preparation, /confirmedBinding = await confirmSerialDmxMachineBindingInternal\(port\)/,
  "stage 2 must retain the native binding result for the same Prepare sequence");
assert.match(controller.slice(serialEnableStart, serialEnableEnd), /confirmedBinding\?\.state === "selected_and_present"/,
  "stage 4 must accept the just-confirmed native binding when the status poller is still fenced");
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
assert.match(preparation, /if \(!options\.safetyBlackout\(\)\)/,
  "clear safety admits the retained USB worker without a global blackout mutation");
assert.doesNotMatch(preparation, /enableShowSpoutOutputs|resetShowSpoutOutputs/,
  "quick setup must not alter the independent Spout route");

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

// Exercise the actual controller with deterministic, read-only IPC stand-ins.
// In particular, no delayed UI intent may be queued behind another action.
const require = createRequire(import.meta.url);
const platformIdentityExports = {};
new Function("exports", ts.transpileModule(read("src/serialDmxDeviceInstance.ts"), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
}).outputText)(platformIdentityExports);
const tablePolicy = await import(await typescriptFixtureModuleUrl(new URL("../src/dmxUniverseTable.ts", import.meta.url)));
const applyPolicy = await import(await typescriptFixtureModuleUrl(new URL("../src/applyDmxUniverseTable.ts", import.meta.url)));
const stageLabelExports = {};
new Function("exports", ts.transpileModule(read("src/showDmxStageLabels.ts"), {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
}).outputText)(stageLabelExports);
const controllerJs = ts.transpileModule(controller, {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
}).outputText;
function leaseFixture(initial = "unavailable", config = {}) {
  const events = [], messages = [], actions = [];
  let authorityCurrent = true;
  let generation = 31;
  let statuses = initial === "unavailable" ? [{status:initial}] : [{
    status:initial, resources:["lighting","video"], authority:{lease_id:"fixture-lease",generation},
  }];
  const invoke = async command => {
    events.push(command);
    if (command === "get_output_ownership_status") return config.ownership ?? readyBothOwnership;
    if (command === "query_output_control_authority_v1") {
      generation++;
      if (config.failFresh) throw new Error("fresh authority unavailable");
      statuses[0].authority = {lease_id:"fixture-lease",generation};
      return {};
    }
    if (command === "select_serial_dmx_machine_binding_v1") {
      if (config.serialBinding) return config.serialBinding;
      throw new Error("fixture stops before device mutation");
    }
    throw new Error(`unexpected invoke ${command}`);
  };
  const select = (query, resources) => {
    assert.equal(query.statuses.length,1);
    assert.equal(query.statuses[0].status,"held_active");
    assert.deepEqual(query.statuses[0].resources,resources);
    return {...query.statuses[0].authority};
  };
  const imports = {
    "./networkDmxRoutes": networkPolicy,
    "./applyDmxUniverseTable": applyPolicy,
    "./dmxUniverseTable": tablePolicy,
    "./serialDmxDeviceInstance": platformIdentityExports,
    "./showDmxStageLabels": stageLabelExports,
    "solid-js": require("solid-js"),
    "./outputControlController": {
      queryDsf2026ArtNetAcceptanceProbeStatus: async () => null,
      queryOutputLeaseAuthority: async () => { events.push("lease-query"); if(config.staleDuringLeaseQuery) authorityCurrent=false; return structuredClone({statuses}); },
      selectOnlyActiveOutputLease: select,
      executeBlackoutRelease: async () => { events.push("release-blackout"); return {}; },
      enableOutput: async (_invoke, expectedProject) => {
        assert.deepEqual(expectedProject,{project_epoch:0,project_revision:1,checkpoint_hash:"fixture-checkpoint"},"managed authority enable retains the captured project identity");
        events.push("enable-output");
        await config.enableGate;
        if (config.staleDuringEnable) authorityCurrent = false;
        if (config.failEnable) throw new Error("enable rejected");
        generation++;
        statuses=[{status:"held_active",resources:["lighting","video"],authority:{lease_id:"fixture-lease",generation}}];
      },
      executeOutputControl: async (_invoke, action) => {
        events.push(action.kind);
        assert.equal(action.lease.generation,generation,"route uses authority freshly selected after managed-lease refresh");
        assert.ok(events.includes("query_output_control_authority_v1"));
        actions.push(structuredClone(action));
      },
    },
    "./serialDmxStatusPoller": {createSerialDmxStatusPoller:()=>({refresh:async()=>{},invalidate:()=>{}})},
    "./safetyBlackoutRuntimeController": {createSafetyBlackoutRuntimeController:()=>({engage:async()=>"applied"})},
    "./serialDmxStatusValidation": {},
  };
  const exports = {};
  new Function("require","exports",controllerJs)(name=>{
    assert.ok(name in imports,`unexpected import ${name}`);return imports[name];
  },exports);
  const instance=exports.createOutputDiagnosticsController({invoke,setMessage:message=>messages.push(message),
    refreshSnapshot:async()=>config.snapshot ?? {},refreshProjectAuthority:async()=>{},
    captureProjectAuthorityIdentity:()=>({project_epoch:0,project_revision:1,checkpoint_hash:"fixture-checkpoint"}),
    isProjectAuthorityIdentityCurrent:()=>authorityCurrent,
    safetyBlackout:()=>true,setSerialPorts:()=>{},
  });
  return {instance,events,messages,actions,setStatuses:value=>{statuses=value;}};
}
const routeMethods=["enableShowSpoutOutputs","enableStagedShowArtNetLoopbackRoute"];
for (const method of routeMethods) {
  for (const state of ["unavailable","held_active","held_orphaned"]) {
    const f=leaseFixture(state);
    await f.instance[method]();
    assert.equal(f.events.filter(event=>event==="enable-output").length,state==="held_active"?0:1,`${method}/${state}`);
    assert.equal(f.actions.length,1,`${method}/${state}`);
    assert.equal(f.actions[0].kind,method==="enableShowSpoutOutputs"?"enable_show_spout_outputs":"enable_show_art_net_loopback_route");
    assert.ok(f.actions[0].lease.generation>31);
  }
  for (const config of [{failEnable:true},{ownership:{...readyBothOwnership,state:"Failed"}},{failFresh:true}]) {
    const f=leaseFixture("unavailable",config);
    await f.instance[method]();
    assert.equal(f.actions.length,0,`${method} must stop after any lease preparation failure`);
    assert.ok(f.messages.some(message=>/rejected|without confirmed|unavailable/.test(message)));
  }
  for (const staleConfig of [{ staleDuringEnable: true }, { staleDuringEnable: true, failEnable: true }]) {
    const stale = leaseFixture("unavailable", staleConfig);
    await stale.instance[method]();
    assert.equal(stale.actions.length, 0, `${method} must discard a stale result after managed lease preparation`);
    assert.ok(!stale.messages.some(message => /enable rejected|enabled|rejected/.test(message)),
      `${method} must suppress stale errors and success messages`);
  }
  for (const state of ["unavailable", "held_active"]) {
    const stale = leaseFixture(state, {staleDuringLeaseQuery:true});
    await stale.instance[method]();
    assert.equal(stale.actions.length,0,`${method} must stop when project changes during public lease query`);
    assert.ok(!stale.events.includes("enable-output"),"a stale public query must not enable ownership for a different project");
    assert.ok(!stale.messages.some(message=>/enabled|rejected/.test(message)),"stale actions must not publish success or device errors");
  }
  const unready=leaseFixture("held_active",{ownership:{...readyBothOwnership,video_allowed:false}});
  await unready.instance[method]();
  assert.equal(unready.actions.length,0);
  assert.ok(!unready.events.includes("enable-output"),"active but unready authority must not be stolen or rearmed");
  for (const statuses of [
    [{status:"held_active",resources:["lighting"],authority:activeBothAuthority}],
    [{status:"held_active",resources:["lighting","video"],authority:activeBothAuthority},{status:"unavailable"}],
  ]) {
    const f=leaseFixture();f.setStatuses(statuses);
    await f.instance[method]();
    assert.equal(f.actions.length,0);
    assert.ok(!f.events.includes("enable-output"));
    assert.match(f.messages.at(-1),/ambiguous or has the wrong resources/);
  }
}
{
  const f = leaseFixture("unavailable", {
    serialBinding: { state: "selected_and_present" },
    snapshot: {
      output: { enabled: true, protocol: "ArtNet", target_ip: "127.0.0.1", port: 6454, universe: 0, serial_port: "", serial_baud_rate: 57_600 },
      dmx_outputs: [],
      telemetry: { last_dmx_route_results: [{ index: 0, universe: 0, attempted: true, success: true }] },
    },
  });
  await f.instance.prepareShowDmx({ name: "COM-fixture", windows_device_instance_id: "fixture" });
  assert.equal(f.actions.filter((action) => action.kind === "enable_show_serial_dmx_safety_blackout_route").length, 1,
    "a fixture-less show preparation with a confirmed USB identity must arm Open DMX");
  assert.match(f.messages.at(-1), /Show DMX setup complete/,
    "a fixture-less show preparation must complete after the USB binding succeeds");
}
for (const first of routeMethods) {
  let release;const enableGate=new Promise(resolve=>{release=resolve;});
  const f=leaseFixture("unavailable",{enableGate});
  const pending=f.instance[first]();
  assert.equal(f.instance[first](),pending,"same-action double click shares its existing promise");
  const other=routeMethods.find(method=>method!==first);
  await f.instance[other]();
  await f.instance.prepareShowDmx({name:"COM-fixture",windows_device_instance_id:"fixture"});
  assert.ok(f.messages.some(message=>/No additional action was queued/.test(message)));
  assert.equal(f.actions.length,0);
  release();await pending;
  assert.equal(f.actions.length,1,"busy different actions never run later under a changed project authority");
  assert.equal(f.events.filter(event=>event==="enable-output").length,1);
  await f.instance[other]();
  assert.equal(f.actions.length,2,"explicit retry after completion can reuse fresh authority");
  assert.equal(f.events.filter(event=>event==="enable-output").length,1);
}
{
  const config={failEnable:true};
  const f=leaseFixture("unavailable",config);
  await f.instance.enableShowSpoutOutputs();
  assert.equal(f.actions.length,0);
  config.failEnable=false;
  await f.instance.enableShowSpoutOutputs();
  assert.equal(f.actions.length,1,"failed standalone preparation releases its same-action promise for explicit retry");
}
{
  let release;const enableGate=new Promise(resolve=>{release=resolve;});
  const f=leaseFixture("unavailable",{enableGate});
  const pending=f.instance.prepareShowDmx({name:"COM-fixture",windows_device_instance_id:"fixture"});
  await f.instance.enableShowSpoutOutputs();
  assert.ok(f.messages.some(message=>/No additional action was queued/.test(message)),
    "busy rejection is reported even if the already active DMX action publishes its own later progress");
  release();await pending;
  assert.equal(f.actions.length,0,"failed DMX setup cannot wake a previously rejected Spout intent");
  await f.instance.enableShowSpoutOutputs();
  assert.equal(f.actions.length,1,"the shared lock is released after DMX preparation failure");
  assert.equal(f.events.filter(event=>event==="enable-output").length,1);
}
console.log("DMX single-Apply UI and retained diagnostics controller contract: PASS (canonical standalone acquire/recover/reuse, fresh authority, fail-closed preparation, same-action singleflight, cross-action busy rejection, loopback-before-S0 boundary)");
