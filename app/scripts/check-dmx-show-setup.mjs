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
assert.match(controller, /hasOnlyActiveOutputLease\(query, \["lighting", "video"\]\)/);
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
assert.match(preparation, /const prepared = await ensureBothOutputLease\(\)/);
assert.match(preparation, /if \(!prepared\.enabled\)[\s\S]*kind: "arm"/,
  "an existing Both lease may be armed, but enable_output must not be redundantly re-armed");

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

console.log("DMX show setup UI contract: PASS (loopback-before-S0 boundary, exact device selection, singleflight, individual diagnostics disclosure)");
