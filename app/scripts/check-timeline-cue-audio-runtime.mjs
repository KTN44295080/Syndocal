import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const appRoot = new URL("..", import.meta.url);
const read = (path) => readFile(new URL(path, appRoot), "utf8");
const controllerSource = await read("src/timelineCueAudioStatusSync.ts");
const app = await read("src/App.tsx");
const editor = await read("src/components/TimelinePerformanceEditor.tsx");
const operator = await read("src/components/TimelineOperatorBar.tsx");
const styles = await read("src/styles.css");
const types = await read("src/types.ts");
const invokes = await read("src/tauriInvokeCommands.ts");
const manifest = await read("src/tauri-invoke-manifest.json");

const output = ts.transpileModule(controllerSource, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
  fileName: "timelineCueAudioStatusSync.ts",
}).outputText;
const sync = await import(`data:text/javascript;base64,${Buffer.from(output).toString("base64")}`);

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
assert.equal(gate.beginPoll(), null, "polling is blocked while a settings request is active");
gate.endMutation(mutation);
assert.equal(gate.beginPoll(), 1, "status polling resumes after mutation settlement");
gate.endPoll();

const deferred = () => {
  let resolve;
  const promise = new Promise((resolvePromise) => { resolve = resolvePromise; });
  return { promise, resolve };
};
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
assert.match(editor, /matching outputs; ambiguous/, "duplicate endpoint names remain visibly ambiguous instead of deduped");
assert.doesNotMatch(editor, /Guide monitor bus|type="checkbox"/, "no local Guide monitor enable survives");
assert.match(editor, /missing_device|topology_changed|fault/, "missing-device, topology mismatch, and fault states stay visible in Cue Audio");
assert.match(editor, /Restart required[\s\S]*?Fault/, "restart-required lifecycle is rendered before generic fault");
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

console.log("timeline cue audio frontend contract: PASS (fences, singleflight, latest mutation, duplicate endpoints, canonical invokes)");
