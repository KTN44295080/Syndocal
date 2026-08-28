import assert from "node:assert/strict";
import { execFile as execFileCallback } from "node:child_process";
import { readFile } from "node:fs/promises";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { promisify } from "node:util";

const execFile = promisify(execFileCallback);

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const read = (relativePath) =>
  readFile(resolve(scriptDirectory, relativePath), "utf8").then((source) =>
    source.replace(/\r\n?/gu, "\n"),
  );

const [app, deck, control, panel, commands, manifest, nativeMain] = await Promise.all([
  read("../src/App.tsx"),
  read("../src/components/IoConnectionDeck.tsx"),
  read("../src/audioOutputControl.ts"),
  read("../src/components/AudioOutputPanel.tsx"),
  read("../src/tauriInvokeCommands.ts"),
  read("../src/tauri-invoke-manifest.json"),
  read("../src-tauri/src/main.rs"),
]);

const commandBody = commands.slice(commands.indexOf("[") + 1, commands.indexOf("] as const"));
const frontendCommands = [...commandBody.matchAll(/"([^"]+)"/gu)].map((match) => match[1]);
const manifestCommands = JSON.parse(manifest);
const functionSlice = (startMarker, endMarker) => {
  const start = control.indexOf(startMarker);
  const end = control.indexOf(endMarker, start + startMarker.length);
  if (start < 0 || end <= start) {
    throw new Error(`controller function boundary is missing: ${startMarker}`);
  }
  return control.slice(start, end);
};
const initialReselectBody = functionSlice(
  "const reselectConfiguredProfile =",
  "const loadCapabilities =",
);
const refreshBody = functionSlice("const refresh =", "const setBackend =");
const revalidateBody = functionSlice("const revalidate =", "const start =");
const returnToNormalBody = functionSlice("const returnToNormal =", "const preflightActionIsAllowed =");
const setDriverBody = functionSlice("const setDriver =", "const setSampleRate =");
const requiredCommands = [
  "get_asio_output_status",
  "list_asio_output_drivers",
  "get_asio_output_capabilities",
  "reselect_asio_output_profile",
  "revalidate_asio_program_cue_output",
  "start_asio_program_cue_output",
  "stop_close_asio_program_cue_output",
  "select_normal_audio_output",
  "set_asio_output_solo",
  "set_asio_output_test",
];

const assertions = [
  [
    /import \{ createAudioOutputController \} from "\.\/audioOutputControl";/u,
    app,
    "App must use the extracted audio-output controller",
  ],
  [
    /import \{ AudioOutputPanel \} from "\.\/components\/AudioOutputPanel";/u,
    app,
    "App must render the existing audio-output panel",
  ],
  [
    /const audioOutputController = createAudioOutputController\(\{[\s\S]*?invoke,[\s\S]*?backendAvailable: isTauriRuntime\(\),[\s\S]*?\}\);/u,
    app,
    "App must instantiate the controller with the typed invoke facade and runtime availability",
  ],
  [
    /const audioOutputController = createAudioOutputController\(\{[\s\S]*?readLivePlaybackActive: \(\) => snapshot\(\)\.timeline\.playing,[\s\S]*?\}\);/u,
    app,
    "App must pass the authoritative Timeline playing state to audio output controls",
  ],
  [
    /audioOutputSummary=\{audioOutputController\.summary\(\)\}[\s\S]*?audioOutputState=\{audioOutputController\.view\(\)\.state\}[\s\S]*?audioOutputStateTone=\{audioOutputController\.stateTone\(\)\}/u,
    app,
    "Setup I/O deck must receive the live Audio summary and state",
  ],
  [
    /connection === "audio" \? \([\s\S]*?<AudioOutputPanel[\s\S]*?onRevalidate=\{audioOutputController\.revalidate\}[\s\S]*?onStart=\{audioOutputController\.start\}[\s\S]*?onStop=\{audioOutputController\.stop\}/u,
    app,
    "Audio must be an in-flow Setup I/O workbench, not a floating surface",
  ],
  [
    /export type IoConnectionId = "dmx" \| "audio" \| "midi" \| "osc" \| "web" \| "dj";/u,
    deck,
    "Setup I/O connection ids must include Audio",
  ],
  [
    /id: "audio",[\s\S]*?summary: props\.audioOutputSummary,[\s\S]*?state: props\.audioOutputState,/u,
    deck,
    "Audio must have a same-level connection card",
  ],
  [
    /const ASIO_SCHEMA_VERSION = 3;[\s\S]*?const ASIO_ABI_VERSION = 3;[\s\S]*?const ASIO_BACKEND = "asio-sdk-v3-rt";/u,
    control,
    "ASIO parser must pin schema 3, ABI 3, and the v3 realtime backend",
  ],
  [
    /requireAsioHeader\([\s\S]*?"drivers"[\s\S]*?\["schemaVersion", "kind", "abiVersion", "backend", "built", "drivers"\]/u,
    control,
    "Driver catalog parsing must enforce the exact v3 header",
  ],
  [
    /requireAsioHeader\([\s\S]*?"capabilities"[\s\S]*?\["schemaVersion", "kind", "abiVersion", "backend", "built", "driver", "output", "input"\]/u,
    control,
    "Capability parsing must enforce the exact v3 header",
  ],
  [
    /requireAsioHeader\([\s\S]*?"status"[\s\S]*?"terminalFault",/u,
    control,
    "Status parsing must enforce the exact v3 header",
  ],
  [
    /requireAsioHeader\(\s*value,\s*"status",[\s\S]*?"routerState",/u,
    control,
    "Status parsing must require routerState as an exact field",
  ],
  [
    /const routerState = String\(value\.routerState\);[\s\S]*?\["Normal", "Quiescing", "AsioReady", "AsioStarting", "AsioActive", "Fault", "Locked"\]/u,
    control,
    "routerState must use the exact native state union",
  ],
  [
    /routerState: routerState as AsioOutputRouterState,/u,
    control,
    "Parsed status must return the strict routerState",
  ],
  [
    /const granularity = requireInteger\(record\.granularity, `\$\{label\}\.granularity`, -1\);/u,
    control,
    "ASIO granularity must preserve the native -1 sentinel",
  ],
  [
    /programLeftOutput = channelToOneBased\(draft\.programLeft,[\s\S]*?programRightOutput = channelToOneBased\(draft\.programRight,[\s\S]*?cueOutput = channelToOneBased\(draft\.cue,/u,
    control,
    "Machine profile conversion must be the single zero-based to one-based boundary",
  ],
  [
    /schemaVersion: MACHINE_PROFILE_SCHEMA_VERSION,[\s\S]*?catalogGeneration,[\s\S]*?nativeFormat,[\s\S]*?fixedBufferFrames,[\s\S]*?deviceOutputChannels,/u,
    control,
    "Machine profile JSON must carry schema, generation, native format, buffer, and device width",
  ],
  [
    /const normalView = \(\): AudioOutputView => \(\{[\s\S]*?backend: "normal-wasapi",[\s\S]*?state: "Ready",/u,
    control,
    "Normal WASAPI must be visibly Ready by default",
  ],
  [
    /Show ASIO command is unavailable in this build; output remains Locked\./u,
    control,
    "Unavailable Show ASIO must remain visibly Locked",
  ],
  [
    /const canStart = (?:createMemo\(\(\) => \{|\(\): boolean => \{)[\s\S]*?view\(\)\.backend !== "show-asio" \|\| busy\(\) \|\| view\(\)\.state !== "Ready"/u,
    control,
    "Start must be fail-closed unless Show ASIO is Ready",
  ],
  [
    /const canStart = (?:createMemo\(\(\) => \{|\(\): boolean => \{)[\s\S]*?nativeStatus[\s\S]*?isSafeForEnumeration\(nativeStatus\)[\s\S]*?nativeStatus\.routerState !== "Normal" && nativeStatus\.routerState !== "AsioReady"[\s\S]*?nativeStatus\.profileReady/u,
    control,
    "Start availability must require native status, safe lifecycle, profile, and Normal/AsioReady router state",
  ],
  [
    /const asioReady = status\.state === "ready"[\s\S]*?status\.routerState === "Normal" \|\| status\.routerState === "AsioReady"[\s\S]*?status\.lifecycle === "Locked" \|\| status\.lifecycle === "Closed"/u,
    control,
    "Ready rendering must require coherent ready/profile/router/lifecycle state",
  ],
  [
    /const asioReady = status\.state === "ready"[\s\S]*?status\.routerState === "Normal" \|\| status\.routerState === "AsioReady"/u,
    control,
    "ready plus router Locked must not render as Ready",
  ],
  [
    /const isActiveAsioOutputStatus = \(status: AsioOutputStatusRecord \| null\): boolean =>[\s\S]*?status\.state === "active"[\s\S]*?status\.routerState === "AsioActive"/u,
    control,
    "Preflight actions must require the exact native Active/AsioActive status pair",
  ],
  [
    /invoke<unknown>\("set_asio_output_test",[\s\S]*?request: \{ test: test [?][?] "off"[\s\S]*?\}\)/u,
    control,
    "Test actions must use the exact nested test request",
  ],
  [
    /invoke<unknown>\("set_asio_output_solo",[\s\S]*?request: \{ mode \}[\s\S]*?\}\)/u,
    control,
    "Solo actions must use the exact nested mode request",
  ],
  [
    /const parseActivePreflightStatus =[\s\S]*?parseAsioOutputStatus\(raw\)[\s\S]*?isActiveAsioOutputStatus\(status\)[\s\S]*?throw new Error/u,
    control,
    "Test and solo replies must be parsed strictly and rechecked as Active/AsioActive before local updates",
  ],
  [
    /const canTest = \(\): boolean =>[\s\S]*?!busy\(\)[\s\S]*?view\(\)\.state === "Active"[\s\S]*?isActiveAsioOutputStatus\(nativeStatus\);[\s\S]*?const canSolo = \(\): boolean =>[\s\S]*?!busy\(\)[\s\S]*?view\(\)\.state === "Active"[\s\S]*?isActiveAsioOutputStatus\(nativeStatus\);/u,
    control,
    "Native test and solo controls must require available, idle, Active/AsioActive output",
  ],
  [
    /if \(test !== null\) setSoloModeState\("none"\);[\s\S]*?setTestMode\(test\);/u,
    control,
    "A successful non-off test must clear the local solo signal before publishing the test",
  ],
  [
    /if \(mode !== "none"\) setTestMode\(null\);[\s\S]*?setSoloModeState\(mode\);/u,
    control,
    "A successful non-none solo must clear the local test signal before publishing solo",
  ],
  [
    /isSafeForEnumeration\([\s\S]*?routerState[\s\S]*?lifecycle[\s\S]*?state/u,
    control,
    "Enumeration and capabilities must be gated by router state, lifecycle, and native status",
  ],
  [
    /const accepted = setConfigurationValue\([\s\S]*?if \(!accepted \|\| !dependencies\.backendAvailable\) return;[\s\S]*?loadCapabilities/u,
    setDriverBody,
    "setDriver must stop before capabilities when configuration mutation is rejected",
  ],
  [
    /invoke<unknown>\("revalidate_asio_program_cue_output"\)/u,
    revalidateBody,
    "Explicit Revalidate must invoke the native revalidate command",
  ],
  [
    /invoke<unknown>\("reselect_asio_output_profile"/u,
    initialReselectBody,
    "Initial selection path must persist through the reselect command",
  ],
  [
    /revalidate_asio_program_cue_output/u,
    refreshBody,
    "Initial Refresh/reselect path must not invoke Locked-only revalidation",
    true,
  ],
  [
    /revalidate_asio_program_cue_output/u,
    initialReselectBody,
    "Initial reselection path must not invoke Locked-only revalidation",
    true,
  ],
  [
    /reselect_asio_output_profile/u,
    revalidateBody,
    "Explicit Revalidate must not substitute profile reselection",
    true,
  ],
  [
    /nativeStatus\.routerState !== "Locked" && nativeStatus\.routerState !== "AsioReady"/u,
    returnToNormalBody,
    "Return to normal must admit only exact Locked or AsioReady router state",
  ],
  [
    /const canReturnToNormal = createMemo\([\s\S]*?nativeStatus\.routerState === "Locked" \|\| nativeStatus\.routerState === "AsioReady"/u,
    control,
    "Return-to-Normal availability must include revalidated AsioReady without admitting other states",
  ],
  [
    /fn select_normal_audio_output\(/u,
    nativeMain,
    "Native Normal selection must use the canonical command function",
  ],
  [
    /fn select_normal_audio_output_router\([\s\S]*?State::AsioReady[\s\S]*?slot\.cancel_ready\(\)[\s\S]*?slot\.select_normal\(\)/u,
    nativeMain,
    "Native AsioReady return must cancel the exact Ready ticket before selecting Normal",
  ],
  [
    /fn reselect_asio_output_profile\([\s\S]*?asio_output_lifecycle[\s\S]*?prepare_asio_router_for_profile_reselection\(slot\)\?/u,
    nativeMain,
    "Profile reselection must serialize lifecycle mutation and invalidate an AsioReady ticket",
  ],
  [
    /select_normal_audio_output,/u,
    nativeMain,
    "Native command registration must expose the canonical Normal selection command",
  ],
  [
    /return_to_normal_audio_output/u,
    nativeMain,
    "Native source must not retain the stale Normal selection alias",
    true,
  ],
  [
    /position\s*:\s*(?:fixed|sticky)|<Portal|<dialog/u,
    `${panel}\n${control}`,
    "Audio output must stay in the Setup I/O flow",
    true,
  ],
];

for (const assertion of assertions) {
  const [pattern, source, message, negative = false] = assertion;
  if (negative) assert.doesNotMatch(source, pattern, message);
  else assert.match(source, pattern, message);
}

assert.deepEqual(frontendCommands, manifestCommands, "Frontend invoke union and JSON manifest must be identical");
assert.deepEqual(frontendCommands, [...frontendCommands].sort(), "Frontend invoke union and JSON manifest must remain sorted");
for (const command of requiredCommands) {
  assert.ok(frontendCommands.includes(command), `Frontend invoke inventory is missing ${command}`);
}

const runtimeRegression = String.raw`
import assert from "node:assert/strict";
import { createSignal } from "solid-js";
import { createAudioOutputController } from "./src/audioOutputControl.ts";

const checks = [];
const check = (value, message) => {
  assert.equal(value, true, message);
  checks.push(message);
};
const settle = async () => {
  for (let index = 0; index < 8; index += 1) {
    await new Promise((resolve) => setTimeout(resolve, 0));
  }
};
const status = (state, routerState, lifecycle, profileReady, catalogGeneration = 1) => ({
  schemaVersion: 3,
  kind: "status",
  abiVersion: 3,
  backend: "asio-sdk-v3-rt",
  built: true,
  state,
  routerState,
  catalogGeneration,
  profileReady,
  lastError: null,
  lifecycle,
  callbacks: null,
  xruns: null,
  terminalFault: null,
});
const catalog = JSON.stringify({
  schemaVersion: 3,
  kind: "drivers",
  abiVersion: 3,
  backend: "asio-sdk-v3-rt",
  built: true,
  drivers: [{ id: "asio:mock", name: "Mock ASIO" }],
});
const capabilities = JSON.stringify({
  schemaVersion: 3,
  kind: "capabilities",
  abiVersion: 3,
  backend: "asio-sdk-v3-rt",
  built: true,
  driver: { id: "asio:mock", name: "Mock ASIO" },
  output: {
    channels: 4,
    nativeFormat: "f32",
    sampleRatesHz: [48000],
    fixedBufferFrames: { min: 64, max: 512, preferred: 128, granularity: -1 },
  },
  input: null,
});
let routerState = "Normal";
let nativeState = "locked";
let profileReady = false;
let testMode = "off";
let soloMode = "none";
let nextPreflightReply = null;
const calls = [];
const callRecords = [];
const invoke = async (command, args) => {
  calls.push(command);
  callRecords.push({ command, args });
  switch (command) {
    case "get_asio_output_status":
      return status(nativeState, routerState, nativeState === "active" ? "Active" : "Locked", profileReady);
    case "list_asio_output_drivers":
      return catalog;
    case "get_asio_output_capabilities":
      return capabilities;
    case "reselect_asio_output_profile":
      nativeState = "ready";
      profileReady = true;
      return status("ready", routerState, "Closed", true);
    case "start_asio_program_cue_output":
      routerState = "AsioActive";
      nativeState = "active";
      return status("active", routerState, "Active", true);
    case "stop_close_asio_program_cue_output":
      routerState = "Locked";
      nativeState = "locked";
      return status("locked", routerState, "Closed", true);
    case "revalidate_asio_program_cue_output":
      routerState = "AsioReady";
      nativeState = "ready";
      profileReady = true;
      return status("ready", routerState, "Locked", true);
    case "select_normal_audio_output":
      routerState = "Normal";
      nativeState = "normal";
      return status("normal", routerState, "Closed", profileReady);
    case "set_asio_output_test": {
      const nextTest = args?.request?.test;
      if (!["off", "program-left", "program-right", "program-stereo", "cue", "spare"].includes(nextTest)) {
        throw new Error("unexpected ASIO test request");
      }
      testMode = nextTest;
      if (nextTest !== "off") soloMode = "none";
      if (nextPreflightReply !== null) {
        const reply = nextPreflightReply;
        nextPreflightReply = null;
        return reply;
      }
      return status("active", "AsioActive", "Active", true);
    }
    case "set_asio_output_solo": {
      const nextSolo = args?.request?.mode;
      if (!["none", "program-only", "cue-only"].includes(nextSolo)) {
        throw new Error("unexpected ASIO solo request");
      }
      soloMode = nextSolo;
      if (nextSolo !== "none") testMode = "off";
      if (nextPreflightReply !== null) {
        const reply = nextPreflightReply;
        nextPreflightReply = null;
        return reply;
      }
      return status("active", "AsioActive", "Active", true);
    }
    default:
      throw new Error("unexpected command " + command);
  }
};
const [timelinePlaying, setTimelinePlaying] = createSignal(false);
const controller = createAudioOutputController({
  invoke,
  backendAvailable: true,
  readLivePlaybackActive: timelinePlaying,
});
controller.setBackend("show-asio");
await settle();
controller.setDriver("asio:mock");
await settle();
controller.setSampleRate(48000);
controller.setBufferFrames(128);
controller.setProgramLeft(0);
controller.setProgramRight(1);
controller.setCue(2);
await settle();
check(controller.view().state === "Ready", "initial Normal reselection reaches Ready");
check(controller.canStart(), "initial Normal reselection enables Start");
await controller.start();
check(controller.canTest() && controller.canSolo() && !controller.livePlaybackActive(), "Active/AsioActive output with stopped Timeline enables native test and solo");
setTimelinePlaying(true);
await settle();
check(controller.livePlaybackActive() && controller.canTest() && controller.canSolo(), "Timeline playing is exposed separately while native preflight remains Active/AsioActive");
setTimelinePlaying(false);
await settle();
check(!controller.livePlaybackActive() && controller.canTest() && controller.canSolo(), "stopping the Timeline re-enables the preflight controls");
await controller.setTest("program-left");
const testCall = callRecords.at(-1);
check(controller.testMode() === "program-left" && controller.soloMode() === "none", "successful test publishes test and clears solo");
check(testCall?.command === "set_asio_output_test"
  && JSON.stringify(testCall.args) === JSON.stringify({ request: { test: "program-left" } }), "test sends the exact nested request");
await controller.setSoloMode("cue-only");
const soloCall = callRecords.at(-1);
check(controller.soloMode() === "cue-only" && controller.testMode() === null, "successful non-none solo publishes solo and clears test");
check(soloCall?.command === "set_asio_output_solo"
  && JSON.stringify(soloCall.args) === JSON.stringify({ request: { mode: "cue-only" } }), "solo sends the exact nested request");
await controller.setTest(null);
check(controller.testMode() === null && controller.soloMode() === "cue-only", "test off clears only the test signal");
const testOffCall = callRecords.at(-1);
check(testOffCall?.command === "set_asio_output_test"
  && JSON.stringify(testOffCall.args) === JSON.stringify({ request: { test: "off" } }), "test off sends the exact off request");
await controller.setSoloMode("none");
check(controller.soloMode() === "none", "solo none clears only the solo signal");
await controller.stop();
check(controller.view().state === "Locked", "Stop/Close leaves the controller Locked");
check(controller.testMode() === null && controller.soloMode() === "none"
  && !controller.canTest() && !controller.canSolo(), "Stop/Close clears stale preflight state and gates");
controller.setProgramLeft(3);
await settle();
check(calls.includes("reselect_asio_output_profile"), "post-stop configuration uses reselection");
check(controller.view().state === "Locked", "ready plus Locked router state stays Locked");
check(!controller.canStart(), "Start stays disabled while router is Locked");
const explicitCallStart = calls.length;
await controller.revalidate();
check(calls.slice(explicitCallStart).includes("revalidate_asio_program_cue_output"), "explicit Revalidate uses native revalidation");
check(!calls.slice(explicitCallStart).includes("reselect_asio_output_profile"), "explicit Revalidate does not substitute reselection");
check(controller.view().state === "Ready", "native revalidation reaches Ready");
check(controller.canStart(), "Start unlocks only after AsioReady revalidation");
check(controller.canReturnToNormal(), "revalidated AsioReady permits an explicit return to Normal");
const bridgeLifecycleCallsBeforeReturn = calls.filter((command) =>
  command === "start_asio_program_cue_output" || command === "stop_close_asio_program_cue_output").length;
await controller.returnToNormal();
check(controller.view().backend === "normal-wasapi" && controller.view().state === "Ready",
  "Revalidate followed by Return to normal publishes the exact Normal view");
check(calls.filter((command) =>
  command === "start_asio_program_cue_output" || command === "stop_close_asio_program_cue_output").length
    === bridgeLifecycleCallsBeforeReturn,
"AsioReady cancellation and Normal selection do not invoke bridge Start or Stop");
controller.setBackend("show-asio");
await settle();
controller.setDriver("asio:mock");
await settle();
controller.setSampleRate(48000);
controller.setBufferFrames(128);
controller.setProgramLeft(3);
controller.setProgramRight(1);
controller.setCue(2);
await settle();
await controller.start();
await controller.setTest("spare");
check(controller.testMode() === "spare" && controller.canTest(), "native test remains available after a second Active transition");
nextPreflightReply = status("active", "Locked", "Active", true);
await controller.setSoloMode("program-only");
check(controller.testMode() === "spare" && controller.soloMode() === "none", "non-Active/AsioActive reply leaves local preflight signals unchanged");
check(controller.view().state === "Locked" && controller.view().reason.includes("did not publish"), "non-Active/AsioActive reply fails closed with an actionable reason");
controller.dispose();
check(controller.testMode() === null && controller.soloMode() === "none", "dispose clears stale preflight state");
console.log("audio output controller runtime regression passed (" + checks.length + " assertions)");
`;
const runtimeResult = await execFile(
  process.execPath,
  ["--no-warnings", "--conditions=browser", "--experimental-strip-types", "--input-type=module", "-e", runtimeRegression],
  { cwd: resolve(scriptDirectory, "..") },
);
assert.match(runtimeResult.stdout, /audio output controller runtime regression passed \(28 assertions\)/u);

console.log(`audio output control checks passed (${assertions.length + requiredCommands.length + 3} assertions; runtime regression 28 assertions)`);
