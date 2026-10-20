// Exercise the actual App/IPC functions: Preview identity must survive selection,
// queue changes and asynchronous delivery, and a refusal cannot launch a fallback.
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const parsed = new Map();
async function extract(file, name, dependencies) {
  let tree = parsed.get(file);
  if (!tree) {
    tree = ts.createSourceFile(file, await readFile(new URL(file, import.meta.url), "utf8"), ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
    parsed.set(file, tree);
  }
  let initializer;
  const visit = (node) => {
    if (ts.isVariableDeclaration(node) && node.name.getText(tree) === name) initializer = node.initializer.getText(tree);
    ts.forEachChild(node, visit);
  };
  visit(tree);
  assert.ok(initializer, name);
  const js = ts.transpileModule(`const ${name} = ${initializer};`, { compilerOptions: { target: ts.ScriptTarget.ES2022 } }).outputText;
  return new Function(...Object.keys(dependencies), `${js}; return ${name};`)(...Object.values(dependencies));
}

const app = "../src/App.tsx";
const layers = [{ id: 2, clip_slots: [{ id: 201 }, { id: 202 }] }, { id: 3, clip_slots: [{ id: 301 }] }];
let selected = layers[0], selectedSlot = null, authority = { project_epoch: 4, project_revision: 9, checkpoint_hash: "head" };
const staged = [], errors = [];
let thumbnails = 0;
const previewSlot = await extract(app, "previewVideoClipSlot", {
  selectedVideoClipSlotLayer: () => selected,
  authorizeVideoThumbnailAccess: () => { thumbnails++; },
  setSelectedVideoClipSlotId: id => { selectedSlot = id; },
  captureProjectAuthorityIdentity: () => ({ ...authority }),
  stageVjPreviewLayer: async (...args) => { staged.push(args); return true; },
});
assert.equal(await previewSlot(202), true);
assert.deepEqual(staged, [[2, authority, undefined, 202]]);
assert.equal(selectedSlot, 202);
assert.equal(thumbnails, 1);
assert.equal(await previewSlot(301), false, "another layer's slot cannot be staged by the selected-row callback");
assert.equal(staged.length, 1);

let response = { layer_id: 2, slot_id: 202 };
const stageCalls = [];
const stageLayer = await extract(app, "stageVjPreviewLayer", {
  isProjectAuthorityIdentityCurrent: expected => JSON.stringify(expected) === JSON.stringify(authority),
  runVjPreviewTransportCommand: async (command, args, options) => { stageCalls.push({ command, args, options }); return response; },
  clearVjFirstRunPreviewFailure: () => {},
  setVjPreviewTransportError: error => errors.push(error),
  translateUiText: text => text, uiLocale: () => "en",
});
assert.equal(await stageLayer(2, { ...authority }, undefined, 202), true);
assert.equal(stageCalls[0].command, "stage_vj_preview_layer");
assert.deepEqual(stageCalls[0].args, { layerId: 2, expectedEpoch: 4, expectedRevision: 9, expectedCheckpointHash: "head", slotId: 202 });
assert.equal(stageCalls[0].options.showError, true, "manual audition failures are visible");
response = { layer_id: 2, slot_id: 201 };
assert.equal(await stageLayer(2, { ...authority }, undefined, 202), false, "a mismatched accepted identity is refused");
assert.match(errors.at(-1), /Preview changed/);
response = null;
assert.equal(await stageLayer(2, { ...authority }, undefined, 202), false, "busy/refused staging never invents Preview state");
response = { layer_id: 2, slot_id: null };
assert.equal(await stageLayer(2), true, "P remains the current-source route");
assert.deepEqual(stageCalls.at(-1).args, { layerId: 2 });
response = { layer_id: 2, slot_id: 202 };
assert.equal(await stageLayer(2, { ...authority, project_epoch: 3 }, undefined, 202), false, "old project reply is refused");

let preview = { layer_id: 2, slot_id: 202, generation: 8 }, busy = false;
let runtimes = [{ layer_id: 2, active_slot_id: 201, queued_slot_id: 201, transition: null }, { layer_id: 3, queued_slot_id: 301, transition: null }];
selected = layers[1];
const takeEnabled = await extract(app, "videoClipSlotTakeEnabled", {
  vjPreviewTransportBusy: () => busy, vjPreviewTransport: () => preview,
  snapshot: () => ({ video: { layers } }), videoClipRuntime: () => ({ layers: runtimes }),
  selectedVideoClipSlotRuntime: () => runtimes.find(item => item.layer_id === selected.id),
});
assert.equal(takeEnabled(), true, "audition Take does not require the previewed slot to be queued or selected");
busy = true; assert.equal(takeEnabled(), false); busy = false;
runtimes[0].transition = { outgoing_slot_id: 201 };
assert.equal(takeEnabled(), false, "audition cannot reverse an unrelated transition");
runtimes[0].transition = null;
preview = { ...preview, slot_id: 999 };
assert.equal(takeEnabled(), false, "removed slot is not takeable");
preview = { layer_id: 2, slot_id: 202, generation: 8 };

const launches = [];
let transportReads = 0, releaseLaunch;
const take = await extract(app, "takeSelectedVideoClipSlot", {
  videoClipSlotTakeEnabled: takeEnabled, vjPreviewTransport: () => preview,
  snapshot: () => ({ video: { layers } }), selectedVideoClipSlotLayer: () => selected,
  videoClipRuntime: () => ({ layers: runtimes }), videoClipTakeKind: () => "Crossfade",
  videoClipTakeDurationUnit: () => "Milliseconds", videoClipTakeDurationMilliunits: () => 750,
  launchVideoClipSlot: (...args) => { launches.push(args); return new Promise(done => { releaseLaunch = done; }); },
  readVjPreviewTransport: async () => { transportReads++; },
  videoProgramAudioEnabled: () => false,
});
const taking = take();
assert.deepEqual(launches[0], [2, 202, "Crossfade", { unit: "Milliseconds", value_milliunits: 750 }, 8]);
selected = layers[0];
preview = { layer_id: 3, slot_id: 301, generation: 9 };
runtimes[0].queued_slot_id = 201;
releaseLaunch(null);
await taking;
assert.equal(launches.length, 1, "a rejected or superseded audition never retries queued/default");
assert.equal(transportReads, 1);
assert.deepEqual(launches[0].slice(0, 2), [2, 202], "selection and Preview changes during delivery cannot retarget Take");
preview = { layer_id: null, slot_id: null, generation: 10 };
runtimes[0].transition = { outgoing_slot_id: 201, kind: "Dip", duration: { unit: "Beats", value_milliunits: 1000 } };
const reversing = take();
assert.deepEqual(launches[1], [2, 201, "Dip", { unit: "Beats", value_milliunits: 1000 }, undefined], "ordinary Reverse remains unchanged");
releaseLaunch({});
await reversing;

const ipc = [], notices = [];
let refuse = false;
const launch = await extract("../src/createVideoRuntimeController.ts", "launchVideoClipSlot", {
  runVideoClipSlotRuntime: async (command, request) => {
    ipc.push({ command, request });
    if (refuse) throw Error("Preview session changed");
    return {};
  },
  options: { setMessage: message => notices.push(message) },
});
await launch(2, 202, "Crossfade", { unit: "Beats", value_milliunits: 2000 }, 8);
assert.deepEqual(ipc[0], { command: "launch_video_clip_slot_authoritative", request: {
  layer_id: 2, slot_id: 202, preview_generation: 8, transition_kind: "Crossfade",
  transition_duration_ms: 0, transition_duration: { unit: "Beats", value_milliunits: 2000 },
} });
refuse = true;
assert.equal(await launch(2, 202, "Cut", undefined, 8), null);
assert.equal(ipc.length, 2, "native refusal produces exactly one ticketed request");
assert.match(notices.at(-1), /Preview session changed/);
refuse = false;
await launch(3);
assert.equal(Object.hasOwn(ipc.at(-1).request, "preview_generation"), false, "non-audition request shape stays compatible");
console.log("PASS clip audition: exact staging/authority, mismatch refusal, busy/removal/transition guards, frozen Take identity, ticketed generation and no fallback; ordinary P/Reverse preserved");
