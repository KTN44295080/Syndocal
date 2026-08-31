import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const appRoot = new URL("..", import.meta.url);
const read = (path) => readFile(new URL(path, appRoot), "utf8");
const controllerSource = await read("src/timelineCueAudioStatusSync.ts");
const app = await read("src/App.tsx");
const editor = await read("src/components/TimelinePerformanceEditor.tsx");
const routingPanel = await read("src/components/TimelineCueAudioRoutingPanel.tsx");
const audioPanel = await read("src/components/AudioOutputPanel.tsx");
const routing = await read("src/timelineCueAudioRouting.ts");
const operator = await read("src/components/TimelineOperatorBar.tsx");
const styles = await read("src/styles.css");
const types = await read("src/types.ts");
const invokes = await read("src/tauriInvokeCommands.ts");
const manifest = await read("src/tauri-invoke-manifest.json");
const localization = await read("src/uiLocalization.ts");

const output = ts.transpileModule(controllerSource, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
  fileName: "timelineCueAudioStatusSync.ts",
}).outputText;
const sync = await import(`data:text/javascript;base64,${Buffer.from(output).toString("base64")}`);
const routingOutput = ts.transpileModule(routing, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
  fileName: "timelineCueAudioRouting.ts",
}).outputText;
const endpointPolicy = await import(`data:text/javascript;base64,${Buffer.from(routingOutput).toString("base64")}`);

const fence = (overrides = {}) => ({
  runtimeIncarnation: 11,
  statusRevision: 1,
  ...overrides,
});

assert.equal(sync.timelineCueAudioStatusCanApply(null, fence()), true);
assert.equal(sync.timelineCueAudioStatusCanApply(fence(), fence({ statusRevision: 0 })), false, "older checked status revision is rejected");
assert.equal(sync.timelineCueAudioStatusCanApply(fence(), fence({ runtimeIncarnation: 12, statusRevision: 0 })), true, "a new runtime incarnation owns a fresh status revision domain");
assert.equal(sync.timelineCueAudioStatusCanApply(null, fence({ runtimeIncarnation: Number.NaN })), false, "NaN incarnation reply is rejected");
assert.equal(sync.timelineCueAudioStatusCanApply(null, fence({ runtimeIncarnation: 11.5 })), false, "fractional incarnation reply is rejected");
assert.equal(sync.timelineCueAudioStatusCanApply(null, fence({ statusRevision: -1 })), false, "negative status revision reply is rejected");
assert.equal(sync.timelineCueAudioStatusCanApply(null, fence({ statusRevision: Number.MAX_SAFE_INTEGER + 1 })), false, "unsafe status-revision overflow reply is rejected");

const gate = sync.createTimelineCueAudioStatusRequestGate();
const oldPoll = gate.beginPoll();
assert.equal(oldPoll, 0);
assert.equal(gate.beginPoll(), null, "status poll is singleflight");
const mutation = gate.beginMutation();
assert.equal(gate.acceptsRequest(oldPoll), false, "pre-mutation poll cannot publish after a settings request");
gate.endPoll();

const deferred = () => {
  let resolve;
  const promise = new Promise((resolvePromise) => { resolve = resolvePromise; });
  return { promise, resolve };
};
const topologyGate = sync.createTimelineCueAudioStatusRequestGate();
const topologyRefresh = sync.createTimelineCueAudioRefreshQueue({
  canRun: () => topologyGate.canBeginPoll(),
  run: async () => {
    const epoch = topologyGate.beginPoll();
    assert.notEqual(epoch, null, "queued topology refresh starts only after the active poll settles");
    await topologyRefreshDeferred.promise;
    topologyGate.endPoll();
    return { epoch };
  },
});
const topologyRefreshDeferred = deferred();
const activeTopologyPoll = topologyGate.beginPoll();
const queuedTopologyRefresh = topologyRefresh.request();
await Promise.resolve();
assert.equal(topologyGate.canBeginPoll(), false, "a mounted Setup refresh observes the active status poll");
topologyGate.endPoll();
topologyRefresh.notifyAvailable();
await Promise.resolve();
await Promise.resolve();
assert.equal(topologyGate.canBeginPoll(), false, "queued refresh owns the single poll slot after collision");
topologyRefreshDeferred.resolve();
const queuedTopologyResult = await queuedTopologyRefresh;
assert.equal(queuedTopologyResult.epoch, activeTopologyPoll, "queued mount refresh retains the current request epoch");
assert.equal(topologyGate.canBeginPoll(), true, "queued topology refresh releases the poll slot after status completion");
assert.equal(gate.beginPoll(), null, "polling is blocked while a settings request is active");
gate.endMutation(mutation);
assert.equal(gate.beginPoll(), 1, "status polling resumes after mutation settlement");
gate.endPoll();

const settleMicrotasks = async (count = 4) => {
  for (let index = 0; index < count; index += 1) await Promise.resolve();
};
const waitForEvent = async (events, expected, message) => {
  for (let index = 0; index < 32; index += 1) {
    if (events.includes(expected)) return;
    await Promise.resolve();
  }
  assert.fail(message);
};
const createMutationTerminal = () => {
  let busy = false;
  const idleWaiters = new Set();
  return {
    begin: () => { busy = true; },
    settle: () => {
      busy = false;
      const waiters = [...idleWaiters];
      idleWaiters.clear();
      waiters.forEach((resolve) => resolve());
    },
    wait: () => busy
      ? new Promise((resolve) => idleWaiters.add(resolve))
      : Promise.resolve(),
  };
};

// Opening Setup starts list -> status. A selection that is received at the
// same time is deferred until the topology receipt is terminal, rather than
// being allowed to race a native list/status call.
const mountEvents = [];
const mountBusy = [];
const mountList = deferred();
const mountStatus = deferred();
const mountMutation = createMutationTerminal();
const mountOperationGate = sync.createTimelineCueAudioSetupOperationGate({
  waitForMutationIdle: () => mountMutation.wait(),
  waitForMutationTerminal: () => mountMutation.wait(),
  refresh: async () => {
    mountEvents.push("list-start");
    await mountList.promise;
    mountEvents.push("list-terminal");
    mountEvents.push("status-start");
    await mountStatus.promise;
    mountEvents.push("status-terminal");
  },
  onBusy: (busy) => mountBusy.push(busy),
});
const mountedRefresh = mountOperationGate.refresh();
const immediateSelection = mountOperationGate.configure(() => {
  mountEvents.push("settings-start");
  mountMutation.begin();
});
await settleMicrotasks();
assert.deepEqual(mountEvents, ["list-start"], "mount refresh owns Setup Audio before an immediate selection");
mountList.resolve();
await settleMicrotasks();
assert.deepEqual(
  mountEvents,
  ["list-start", "list-terminal", "status-start"],
  "selection waits for list completion and the authoritative status request",
);
mountStatus.resolve();
await waitForEvent(mountEvents, "settings-start", "selection should run after the status terminal receipt");
assert.ok(
  mountEvents.indexOf("status-terminal") < mountEvents.indexOf("settings-start"),
  "settings starts only after list/status is terminal",
);
mountEvents.push("settings-terminal");
mountMutation.settle();
await Promise.all([mountedRefresh, immediateSelection]);
assert.deepEqual(mountBusy, [true, false], "both Setup operations share one busy interval");

// The inverse arrival order has the same single transaction boundary: a
// refresh cannot enumerate or fetch status while an accepted selection waits
// for the native settings terminal state.
const selectionEvents = [];
const selectionBusy = [];
const selectionList = deferred();
const selectionStatus = deferred();
const selectionMutation = createMutationTerminal();
const selectionOperationGate = sync.createTimelineCueAudioSetupOperationGate({
  waitForMutationIdle: () => selectionMutation.wait(),
  waitForMutationTerminal: () => selectionMutation.wait(),
  refresh: async () => {
    selectionEvents.push("list-start");
    await selectionList.promise;
    selectionEvents.push("list-terminal");
    selectionEvents.push("status-start");
    await selectionStatus.promise;
    selectionEvents.push("status-terminal");
  },
  onBusy: (busy) => selectionBusy.push(busy),
});
const firstSelection = selectionOperationGate.configure(() => {
  selectionEvents.push("settings-start");
  selectionMutation.begin();
});
const deferredRefresh = selectionOperationGate.refresh();
await settleMicrotasks();
assert.deepEqual(selectionEvents, ["settings-start"], "refresh is deferred while settings are active");
selectionEvents.push("settings-terminal");
selectionMutation.settle();
await waitForEvent(selectionEvents, "list-start", "refresh should run after the settings terminal receipt");
assert.ok(
  selectionEvents.indexOf("settings-terminal") < selectionEvents.indexOf("list-start"),
  "refresh begins only after the settings terminal state",
);
selectionList.resolve();
await settleMicrotasks();
selectionStatus.resolve();
await Promise.all([firstSelection, deferredRefresh]);
assert.deepEqual(selectionBusy, [true, false], "reverse arrival order retains one busy interval");

// A rejected list/status receipt must not poison the FIFO tail: the next
// accepted selection and its following refresh still run in that order.
const rejectedRefreshEvents = [];
const rejectedRefreshBusy = [];
let rejectedRefreshAttempts = 0;
const rejectedRefreshGate = sync.createTimelineCueAudioSetupOperationGate({
  waitForMutationIdle: () => Promise.resolve(),
  waitForMutationTerminal: () => Promise.resolve(),
  refresh: async () => {
    rejectedRefreshAttempts += 1;
    rejectedRefreshEvents.push(`refresh-${rejectedRefreshAttempts}`);
    if (rejectedRefreshAttempts === 1) throw new Error("list/status rejected");
  },
  onBusy: (busy) => rejectedRefreshBusy.push(busy),
});
await assert.rejects(
  rejectedRefreshGate.refresh(),
  /list\/status rejected/,
  "refresh rejection is observable to its initiating operation",
);
await rejectedRefreshGate.configure(() => rejectedRefreshEvents.push("settings-after-refresh-rejection"));
await rejectedRefreshGate.refresh();
assert.deepEqual(
  rejectedRefreshEvents,
  ["refresh-1", "settings-after-refresh-rejection", "refresh-2"],
  "refresh rejection -> configure -> refresh recovers without reordering",
);
assert.deepEqual(rejectedRefreshBusy, [true, false, true, false, true, false], "each recovered operation returns the busy state to idle");

// The same tail recovery is required when the settings write itself throws;
// the later refresh remains reachable and never inherits the rejected write.
const rejectedConfigureEvents = [];
const rejectedConfigureBusy = [];
const rejectedConfigureGate = sync.createTimelineCueAudioSetupOperationGate({
  waitForMutationIdle: () => Promise.resolve(),
  waitForMutationTerminal: () => Promise.resolve(),
  refresh: () => rejectedConfigureEvents.push("refresh-after-settings-rejection"),
  onBusy: (busy) => rejectedConfigureBusy.push(busy),
});
await assert.rejects(
  rejectedConfigureGate.configure(() => {
    rejectedConfigureEvents.push("settings-rejected");
    throw new Error("settings rejected");
  }),
  /settings rejected/,
  "settings rejection is observable to its initiating operation",
);
await rejectedConfigureGate.refresh();
assert.deepEqual(
  rejectedConfigureEvents,
  ["settings-rejected", "refresh-after-settings-rejection"],
  "configure rejection -> refresh recovers without a stale write",
);
assert.deepEqual(rejectedConfigureBusy, [true, false, true, false], "settings rejection releases the busy state before refresh recovery");

// Destroying Setup while an external mutation is still settling retires every
// queued operation. In particular, the queued configure callback must not
// start a new native settings write after its owner has gone away.
const disposeEvents = [];
const disposeBusy = [];
const disposeMutationWait = deferred();
const disposedOperationGate = sync.createTimelineCueAudioSetupOperationGate({
  waitForMutationIdle: () => disposeMutationWait.promise,
  waitForMutationTerminal: () => disposeMutationWait.promise,
  refresh: () => disposeEvents.push("refresh-started"),
  onBusy: (busy) => disposeBusy.push(busy),
});
const disposeWaitingRefresh = disposedOperationGate.refresh();
const disposeQueuedConfigure = disposedOperationGate.configure(() => disposeEvents.push("settings-started"));
const disposeQueuedRefresh = disposedOperationGate.refresh();
await settleMicrotasks();
assert.deepEqual(disposeEvents, [], "no native operation starts while the external mutation is unsettled");
disposedOperationGate.dispose();
disposeMutationWait.resolve();
assert.deepEqual(
  await Promise.all([disposeWaitingRefresh, disposeQueuedConfigure, disposeQueuedRefresh]),
  [false, false, false],
  "disposed waiting and queued operations report retirement",
);
assert.deepEqual(disposeEvents, [], "dispose prevents a waiting configure from starting a new mutation");
assert.deepEqual(disposeBusy, [true, false], "dispose clears the panel busy state exactly once");

const first = deferred();
const second = deferred();
const sent = [];
const received = [];
const queueGate = sync.createTimelineCueAudioStatusRequestGate();
const queue = sync.createTimelineCueAudioSettingsQueue({
  gate: queueGate,
  send: (settings) => {
    sent.push(settings);
    return sent.length === 1 ? first.promise : second.promise;
  },
  fenceOf: (status) => status.fence,
  onStatus: (status) => received.push(status.value),
  onError: (error) => { throw error; },
  onBusy: () => {},
});
queue.submit("A");
queue.submit("B");
await Promise.resolve();
assert.deepEqual(sent, ["A"], "first request begins immediately");
first.resolve({ value: "A", fence: fence({ settingsRevision: 1 }) });
await Promise.resolve();
await Promise.resolve();
assert.deepEqual(sent, ["A", "B"], "latest desired settings are serialized after the active request");
second.resolve({ value: "B", fence: fence({ statusRevision: 2 }) });
await Promise.resolve();
await Promise.resolve();
assert.deepEqual(received, ["B"], "superseded mutation reply never becomes applied frontend truth");

const restartGate = sync.createTimelineCueAudioStatusRequestGate();
assert.equal(restartGate.commitStatus(fence({ runtimeIncarnation: 20, statusRevision: 8 })), true);
assert.equal(restartGate.commitStatus(fence({ runtimeIncarnation: 20, statusRevision: 7 })), false, "same-runtime stale poll is rejected");
assert.equal(restartGate.commitStatus(fence({ runtimeIncarnation: 21, statusRevision: 1 })), true, "restart status is accepted as a fresh checked domain");
assert.equal(restartGate.commitStatus(fence({ runtimeIncarnation: 20, statusRevision: 9 })), false, "a reply from the retired runtime cannot overwrite restart truth");

const liveEndpoint = (name, overrides = {}) => ({ name, occurrences: 1, selectable: true, ...overrides });
const liveEndpoints = [liveEndpoint("Headphones (Wave Link)"), liveEndpoint("Speakers")];
assert.equal(
  endpointPolicy.timelineCueAudioMissingSavedName(liveEndpoints, "Music (Elgato Virtual Audio)"),
  true,
  "a persisted missing device remains identifiable without hiding current endpoints",
);
assert.equal(
  endpointPolicy.timelineCueAudioEndpointByExactName(liveEndpoints, "Headphones (Wave Link)")?.name,
  "Headphones (Wave Link)",
  "a current unique endpoint remains selectable alongside a missing saved name",
);
assert.equal(
  endpointPolicy.timelineCueAudioEndpointIsSelectable(
    [liveEndpoint("Duplicate", { occurrences: 2, selectable: true })],
    liveEndpoint("Duplicate", { occurrences: 2, selectable: true }),
  ),
  false,
  "duplicate exact names remain fail-closed even if native status is overly permissive",
);
assert.equal(
  endpointPolicy.timelineCueAudioEndpointByExactName(
    [liveEndpoint("Duplicate"), liveEndpoint("Duplicate")],
    "Duplicate",
  ),
  null,
  "duplicate endpoint records cannot be selected by name",
);

assert.match(types, /topology_fingerprint: string \| null/, "machine settings keep the exact topology fence");
assert.match(types, /runtimeIncarnation: number/, "status exposes the native restart incarnation");
assert.match(types, /statusRevision: number/, "status exposes the checked native status revision");
assert.match(types, /occurrences: number/, "endpoint summaries retain duplicate-name occurrence truth");
assert.match(types, /\| "interlude"/, "Timeline phase role retains protocol interlude");
assert.match(types, /\| \{ kind: "complete" \}/, "Timeline Guide retains protocol complete cue");
for (const field of ["asset", "playback_rate_milli", "sample_frame", "epoch", "transport_generation", "schedule_generation", "source"]) {
  assert.match(types, new RegExp(`\\b${field}\\b`), `Timeline Guide retains protocol ${field}`);
}
assert.match(editor, /data-timeline-cue-audio-editor/, "Cue Audio has a stable production selector");
assert.match(editor, /data-timeline-cue-audio-status/, "Timeline Cue Audio status has a stable production selector");
assert.doesNotMatch(editor, /data-timeline-authoring-monitor/, "retired Timeline authoring-monitor selector is removed");
assert.match(editor, /TimelineCueAudioStatusSummary/, "Timeline tools keep only the shared read-only Cue Audio status summary");
assert.doesNotMatch(editor, /data-timeline-cue-audio-output/, "Timeline tools no longer own a second editable device selector");
assert.doesNotMatch(editor, /onConfigureCueAudio|onRefreshCueAudio|cueAudioMutationBusy|cueAudioLocalError/, "retired Timeline configuration props are removed instead of threaded through a dead path");
assert.match(editor, /timelineCueAudioLifecycleLabel\(props\.cueAudioStatus\)/, "Timeline summary renders a human-readable lifecycle label instead of a raw enum");
assert.match(controllerSource, /createTimelineCueAudioSetupOperationGate/, "Setup Audio has one shared refresh/settings operation gate");
assert.match(routingPanel, /const \[setupOperationBusy, setSetupOperationBusy\] = createSignal\(true\)/, "Setup controls begin disabled until the mount receipt is terminal");
assert.match(routingPanel, /onMount\(\(\) => \{[\s\S]*?refreshOutputs\(\)/, "opening Setup Audio enumerates current outputs through the shared operation gate");
assert.match(routingPanel, /setupOperationGate\.refresh\(\)/, "manual refresh uses the shared operation gate");
assert.match(routingPanel, /setupOperationGate\.configure\(/, "route and device writes use the shared operation gate");
assert.match(routingPanel, /disabled=\{controlsBusy\(\)\}/, "all Setup selection controls are disabled while either direction is active");
assert.doesNotMatch(routingPanel, /disabled=\{props\.mutationBusy\}/, "controls do not expose an input window during mount list/status refresh");
assert.match(routingPanel, /data-timeline-cue-audio-open-setup/, "Timeline summary exposes an accessible Setup navigation action");
assert.doesNotMatch(routingPanel, /data-no-localize[^>]*data-timeline-cue-audio-selected-device|data-timeline-cue-audio-selected-device[^>]*data-no-localize/, "device fallback status remains dynamically localizable");
assert.match(routingPanel, /timelineCueAudioOpenSetupEvent/, "Timeline navigation uses the shared Setup event contract");
assert.match(routingPanel, /data-timeline-cue-audio-missing/, "a saved missing endpoint remains a visible disabled warning");
assert.match(routingPanel, /matching outputs; ambiguous/, "duplicate endpoint names remain visibly ambiguous instead of deduped");
assert.match(routingPanel, /observedTopologyFingerprint/, "reselecting after a topology change uses the current exact topology fence");
assert.match(audioPanel, /TimelineCueAudioRoutingPanel/, "Setup Audio renders the shared Timeline Cue Audio route");
assert.match(audioPanel, /timelineCueAudioStatus\?: TimelineCueAudioStatus/, "Setup Audio accepts canonical Cue Audio status");
assert.match(app, /timelineCueAudioStatus=\{timelineCueAudioStatus\(\)\}/, "App wires canonical Cue Audio status into Setup Audio");
assert.match(app, /onRefreshTimelineCueAudio=\{async \(\) => \{ await refreshTimelineCueAudioStatus\(true, true\); \}\}/, "Setup Audio refresh uses the canonical list-then-status handler");
assert.match(app, /setWorkspaceTab\("setup"\);[\s\S]*?setSetupSubTab\("io"\);[\s\S]*?setActiveIoConnection\("audio"\)/, "Timeline summary navigation selects Setup I\/O Audio");
assert.match(app, /createTimelineCueAudioRefreshQueue/, "explicit topology refreshes use the serialized queue");
assert.match(app, /canRun: \(\) => timelineCueAudioStatusRequests\.canBeginPoll\(\)/, "queued refresh waits for current poll or mutation settlement");
assert.doesNotMatch(editor, /Guide monitor bus|type="checkbox"/, "no local Guide monitor enable survives");
assert.match(routingPanel, /missing_device|topology_changed|fault/, "missing-device, topology mismatch, and fault states stay visible in Cue Audio");
assert.match(routingPanel, /No current Windows output was enumerated/, "empty enumeration has a clear actionable state");
assert.match(operator, /data-timeline-metronome[\s\S]*?onSetMetronome/, "project Click stays the top-bar authority");
assert.match(operator, /data-timeline-guide[\s\S]*?onSetGuideEnabled/, "project Guide stays the top-bar authority");
assert.match(app, /get_timeline_cue_audio_status/, "status polling uses the canonical Cue Audio command");
assert.match(app, /set_machine_timeline_cue_audio_settings/, "settings mutation uses the canonical Cue Audio command");
const cueAudioAppSource = app.slice(app.indexOf("const applyTimelineCueAudioStatus"), app.indexOf("const applyTimelineFollowRuntimeReport"));
assert.doesNotMatch(cueAudioAppSource, /get_timeline_guide_audio_status|set_timeline_guide_audio_config/, "Cue Audio never invokes legacy status/settings commands");
assert.match(cueAudioAppSource, /if \(refreshOutputs\) await invoke<string\[\]>\("list_audio_output_devices"\);[\s\S]*?get_timeline_cue_audio_status/, "manual Refresh enumerates once before canonical status");
assert.match(app, /setInterval\(refresh, 1_000\)/, "status timer remains present");
assert.doesNotMatch(app.slice(app.indexOf("const refresh = async () =>"), app.indexOf("const applyTimelineFollowRuntimeReport")), /refreshTimelineCueAudioStatus\(true, true\)/, "timer never enumerates outputs");
assert.match(styles, /\.timelineCueAudioEditor > summary[\s\S]*?min-height: 44px/, "Cue Audio disclosure keeps a 44px summary target");
assert.match(styles, /\.timelineToolsDisclosurePanel:has\(> \.timelinePerformanceEditor\)[\s\S]*?overflow: auto/, "Cue Audio remains inside the existing internal-scroll disclosure");

const commands = JSON.parse(manifest);
assert.deepEqual(commands, [...commands].sort(), "Cue Audio manifest remains bytewise sorted");
assert.ok(commands.includes("get_timeline_cue_audio_status"));
assert.ok(commands.includes("set_machine_timeline_cue_audio_settings"));
assert.ok(!commands.includes("get_timeline_guide_audio_status"));
assert.ok(!commands.includes("set_timeline_guide_audio_config"));
assert.match(invokes, /"get_timeline_cue_audio_status"/);
assert.match(invokes, /"set_machine_timeline_cue_audio_settings"/);
for (const sourceText of [
  "Timeline authoring audio",
  "Choose the Windows output for Timeline media, Guide, and Click.",
  "Refresh Windows outputs",
  "No current Windows output was enumerated",
  "Select an exact Windows output…",
  "Configure in Setup → I/O → Audio",
  "Ambiguous device",
]) {
  assert.match(localization, new RegExp(`\\"${sourceText.replace(/[.*+?^${}()|[\\]\\\\]/g, "\\\\$&")}\\"`), `Japanese localization exists for ${sourceText}`);
}
assert.match(localization, /\bRunning:\s*"実行中"/, "Japanese localization exists for Running");
assert.match(localization, /\bFault:\s*"異常"/, "Japanese localization exists for Fault");

console.log("timeline cue audio frontend contract: PASS (fences, singleflight, latest mutation, duplicate endpoints, canonical invokes)");
