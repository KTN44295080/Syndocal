import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const transpile = (source, fileName) => ts.transpileModule(source, {
  compilerOptions: {
    module: ts.ModuleKind.ESNext,
    target: ts.ScriptTarget.ES2022,
    importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
  },
  fileName,
});
const loadModule = async (name) => {
  const source = await readFile(new URL(`../src/${name}`, import.meta.url), "utf8");
  const transpiled = transpile(source, name);
  return import(`data:text/javascript;base64,${Buffer.from(transpiled.outputText).toString("base64")}`);
};
const dataModule = (source, fileName) =>
  `data:text/javascript;base64,${Buffer.from(transpile(source, fileName).outputText).toString("base64")}`;
const runtime = await loadModule("timelineExternalDrag.ts");
const dropRuntime = await loadModule("timelineExternalDropRuntime.ts");
const localization = await loadModule("uiLocalization.ts");
const bankAuthority = await loadModule("bankAuthority.ts");
const gestureSource = await readFile(new URL("../src/timelineBlockGestures.ts", import.meta.url), "utf8");
const canonicalKindSource = await readFile(new URL("../src/sceneCueKind.ts", import.meta.url), "utf8");
const canonicalKindModuleUrl = dataModule(canonicalKindSource, "sceneCueKind.ts");
const timelineSceneBlocksSource = await readFile(new URL("../src/timelineSceneBlocks.ts", import.meta.url), "utf8");
const timelineSceneBlockHelpers = await import(dataModule(
  timelineSceneBlocksSource
    .replace("./timelineBlockGestures", dataModule(gestureSource, "timelineBlockGestures.ts"))
    .replace("./sceneCueKind", canonicalKindModuleUrl),
  "timelineSceneBlocks.ts",
));
const overviewSource = await readFile(new URL("../src/components/TimelineOverview.tsx", import.meta.url), "utf8");
const overviewPreflightMatch = overviewSource.match(/export const timelineOverviewExternalDropPreflight = [\s\S]*?\n};/);
assert.ok(overviewPreflightMatch, "TimelineOverview must export the pre-callback external-drop seam");
const overviewPreflight = await import(`data:text/javascript;base64,${Buffer.from(
  transpile(overviewPreflightMatch[0], "TimelineOverview.preflight.ts").outputText,
).toString("base64")}`);
const appSource = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
assert.match(
  appSource,
  /onTimelineStatus=\{\(source\) => setMessage\(translateUiText\(source, uiLocale\(\)\)\)\}/,
  "TimelineOverview's production status callback must localize at the App edge",
);
assert.match(
  appSource,
  /setMessage\(translateUiText\(`Scene \$\{cueId\} is no longer available\.`, uiLocale\(\)\)\)/,
  "the App scene-conflict fallback must use the same localized status edge",
);

const { TIMELINE_EXTERNAL_DRAG_MIME } = runtime;
const mediaVideo = {
  schema: 1,
  kind: "media_asset",
  media_asset_id: 7,
  lane_kind: "Video",
  video_layer_id: 11,
  audio_layer_id: 12,
};
const mediaAudio = { ...mediaVideo, lane_kind: "Audio" };
const scene = { schema: 1, kind: "scene", cue_id: 12, lane_kind: "Lighting" };

assert.deepEqual(runtime.parseTimelineExternalDragPayload(JSON.stringify(mediaVideo)), mediaVideo);
assert.deepEqual(runtime.parseTimelineExternalDragPayload(JSON.stringify(mediaAudio)), mediaAudio);
assert.deepEqual(runtime.parseTimelineExternalDragPayload(JSON.stringify(scene)), scene);
for (const invalid of [
  "",
  "not-json",
  JSON.stringify({ ...mediaVideo, schema: 2 }),
  JSON.stringify({ ...mediaVideo, media_asset_id: 0 }),
  JSON.stringify({ ...mediaVideo, media_asset_id: "7" }),
  JSON.stringify({ ...mediaVideo, lane_kind: "Lighting" }),
  JSON.stringify({ ...scene, lane_kind: "Video" }),
  JSON.stringify({ ...scene, extra: true }),
  JSON.stringify({ ...mediaVideo, extra: true }),
]) {
  assert.equal(runtime.parseTimelineExternalDragPayload(invalid), null, `invalid payload accepted: ${invalid}`);
}

// A protected dragover advertises the MIME but returns no data. Production
// code must still accept the browser drop, then parse strictly on drop.
const protectedTransfer = {
  types: [TIMELINE_EXTERNAL_DRAG_MIME],
  getData: () => "",
};
assert.equal(runtime.timelineExternalDragDataIsAdvertised(protectedTransfer), true);
assert.equal(runtime.parseTimelineExternalDragDataTransfer(protectedTransfer), null);
assert.equal(runtime.timelineExternalDragDataIsAdvertised({ types: ["text/plain"], getData: () => "" }), false);

const gate = runtime.createTimelineExternalDropGate();
const dropEvent = {};
assert.equal(gate(dropEvent), true);
assert.equal(gate(dropEvent), false, "duplicate drop event must be single-flight");
assert.equal(gate({}), true, "a distinct drop event remains eligible");

const serialized = runtime.serializeTimelineExternalDragPayload(mediaVideo);
assert.deepEqual(JSON.parse(serialized), mediaVideo);

// Drive the production drop orchestrator with a fake backend adapter. This
// proves exact target/companion IDs and fail-closed wrong-kind/no-companion
// paths without duplicating the App's insert-media model in this checker.
const layers = [
  { id: 11, label: "Video A", order: 0, muted: false, locked: false, solo: false, kind: "Video" },
  { id: 12, label: "Audio A", order: 0, muted: false, locked: false, solo: false, kind: "Audio" },
  { id: 13, label: "Lighting A", order: 0, muted: false, locked: false, solo: false, kind: "Lighting" },
];

// Clicks resolve a sole unlocked lane, require an explicit choice when the
// target is ambiguous, and expose a visible no-candidate state to the Shelf.
assert.equal(dropRuntime.resolveTimelineExternalLayer(layers, "Video", null).mode, "unique");
assert.equal(dropRuntime.resolveTimelineExternalLayer(layers, "Lighting", null).mode, "unique");
assert.equal(
  dropRuntime.resolveTimelineExternalLayer(
    [
      { id: 11, label: "Video A", order: 0, muted: false, locked: false, solo: false, kind: "Video" },
      { id: 14, label: "Video B", order: 1, muted: false, locked: false, solo: false, kind: "Video" },
    ],
    "Video",
    null,
  ).mode,
  "ambiguous",
);
assert.equal(
  dropRuntime.resolveTimelineExternalLayer(
    [
      { id: 11, label: "Video A", order: 0, muted: false, locked: false, solo: false, kind: "Video" },
      { id: 14, label: "Video B", order: 1, muted: false, locked: false, solo: false, kind: "Video" },
    ],
    "Video",
    14,
  ).selected?.id,
  14,
);
assert.equal(
  dropRuntime.resolveTimelineExternalLayer(
    [{ id: 11, label: "Video A", order: 0, muted: false, locked: true, solo: false, kind: "Video" }],
    "Video",
    null,
  ).mode,
  "none",
);
for (const staleLayers of [
  [{ id: 11, label: "Video A", order: 0, muted: false, locked: false, solo: false, kind: "Video" }],
  [
    { id: 11, label: "Video A", order: 0, muted: false, locked: false, solo: false, kind: "Video" },
    { id: 14, label: "Video B", order: 1, muted: false, locked: true, solo: false, kind: "Video" },
  ],
  [
    { id: 11, label: "Video A", order: 0, muted: false, locked: false, solo: false, kind: "Video" },
    { id: 14, label: "Former Video B", order: 1, muted: false, locked: false, solo: false, kind: "Audio" },
  ],
]) {
  const resolution = dropRuntime.resolveTimelineExternalLayer(staleLayers, "Video", 14);
  assert.equal(resolution.mode, "stale", "a stale explicit click target must not auto-retarget to the sole remaining lane");
  assert.equal(resolution.selected, null);
}
const avAsset = {
  id: 7,
  label: "Show MP4",
  source: { kind: "File", metadata: { duration_ms: 1000, width: 1920, height: 1080, has_audio: true } },
};
const availableVerified = { 7: { kind: "available_verified", asset_id: 7 } };
const sceneIds = new Set([12]);
let insertCalls = 0;
let insertRequest = null;
const placed = await dropRuntime.executeTimelineExternalDrop(
  mediaVideo,
  layers[0],
  layers,
  [avAsset],
  sceneIds,
  1234.4,
  {
    insertMedia: async (assetId, placement) => {
      insertCalls += 1;
      insertRequest = { assetId, placement };
      return true;
    },
    placeScene: () => { throw new Error("scene callback must not run for media"); },
  },
  availableVerified,
);
assert.equal(placed, true);
assert.equal(insertCalls, 1);
assert.deepEqual(insertRequest, {
  assetId: 7,
  placement: {
    startMs: 1234,
    videoLayerId: 11,
    audioLayerId: 12,
    linkedAudio: true,
    linkedVideo: true,
  },
});

// The exact lane hit by a drag is authoritative for its primary media kind;
// a stale or omitted Shelf primary selection cannot redirect or block it.
let dropPrimaryRequest = null;
assert.equal(await dropRuntime.executeTimelineExternalDrop(
  { ...mediaVideo, video_layer_id: 999, audio_layer_id: null },
  { ...layers[0], id: 14, label: "Video B" },
  [...layers, { id: 14, label: "Video B", order: 1, muted: false, locked: false, solo: false, kind: "Video" }],
  [avAsset],
  sceneIds,
  2.2,
  {
    insertMedia: (assetId, placement) => { dropPrimaryRequest = { assetId, placement }; return true; },
    placeScene: () => { throw new Error("scene callback must not run for media"); },
    reject: (message) => { throw new Error(`primary drop unexpectedly rejected: ${message}`); },
  },
  availableVerified,
), true, "a drag uses the exact unlocked drop lane as its primary target");
assert.equal(dropPrimaryRequest?.placement.videoLayerId, 14);
assert.equal(dropPrimaryRequest?.placement.audioLayerId, 12, "a sole linked companion lane is auto-resolved");

// A linked companion still requires explicit selection when more than one
// unlocked lane exists; it must never silently select the first match.
const twoAudioLayers = [
  ...layers,
  { id: 15, label: "Audio B", order: 1, muted: false, locked: false, solo: false, kind: "Audio" },
];
let ambiguousCompanionCalls = 0;
let ambiguousCompanionStatus = null;
assert.equal(await dropRuntime.executeTimelineExternalDrop(
  { ...mediaVideo, video_layer_id: null, audio_layer_id: null },
  twoAudioLayers[0],
  twoAudioLayers,
  [avAsset],
  sceneIds,
  0,
  {
    insertMedia: () => { ambiguousCompanionCalls += 1; return true; },
    placeScene: () => {},
    reject: (message) => { ambiguousCompanionStatus = message; },
  },
  availableVerified,
), false, "ambiguous linked companion placement must fail closed");
assert.equal(ambiguousCompanionCalls, 0);
assert.equal(ambiguousCompanionStatus, "Select an unlocked Audio Timeline lane before placing media.");

let staleCompanionStatus = null;
assert.equal(await dropRuntime.executeTimelineExternalDrop(
  { ...mediaVideo, video_layer_id: null, audio_layer_id: 999 },
  twoAudioLayers[0],
  twoAudioLayers,
  [avAsset],
  sceneIds,
  0,
  {
    insertMedia: () => { throw new Error("stale companion must not invoke insert"); },
    placeScene: () => {},
    reject: (message) => { staleCompanionStatus = message; },
  },
  availableVerified,
), false, "a stale explicit linked companion must fail closed");
assert.equal(staleCompanionStatus, "Selected Audio Timeline lane is no longer available.");

// An explicit companion selection is accepted even when the primary drag
// selection is absent; the selected companion remains exact and revalidated.
let explicitCompanionRequest = null;
assert.equal(await dropRuntime.executeTimelineExternalDrop(
  { ...mediaVideo, video_layer_id: null, audio_layer_id: 15 },
  twoAudioLayers[0],
  twoAudioLayers,
  [avAsset],
  sceneIds,
  0,
  {
    insertMedia: (assetId, placement) => { explicitCompanionRequest = { assetId, placement }; return true; },
    placeScene: () => {},
  },
  availableVerified,
), true);
assert.equal(explicitCompanionRequest?.placement.videoLayerId, 11);
assert.equal(explicitCompanionRequest?.placement.audioLayerId, 15);

let audioOriginInsert = null;
assert.equal(await dropRuntime.executeTimelineExternalDrop(
  mediaAudio,
  layers[1],
  layers,
  [avAsset],
  sceneIds,
  1234.4,
  {
    insertMedia: (assetId, placement) => { audioOriginInsert = { assetId, placement }; return true; },
    placeScene: () => { throw new Error("scene callback must not run for media"); },
  },
  availableVerified,
), true, "an Audio-origin AV placement must retain both explicitly selected lanes");
assert.deepEqual(audioOriginInsert, {
  assetId: 7,
  placement: {
    startMs: 1234,
    videoLayerId: 11,
    audioLayerId: 12,
    linkedAudio: true,
    linkedVideo: true,
  },
});

for (const lockedLayerId of [11, 12]) {
  const lockedLayers = layers.map((layer) => layer.id === lockedLayerId ? { ...layer, locked: true } : layer);
  const target = lockedLayers.find((layer) => layer.id === lockedLayerId);
  let lockedInsertCalls = 0;
  assert.equal(await dropRuntime.executeTimelineExternalDrop(
    lockedLayerId === 11 ? mediaVideo : mediaAudio,
    target,
    lockedLayers,
    [avAsset],
    sceneIds,
    0,
    { insertMedia: () => { lockedInsertCalls += 1; return true; }, placeScene: () => {}, reject: () => {} },
    availableVerified,
  ), false, `locked ${target.kind} lane must fail closed`);
  assert.equal(lockedInsertCalls, 0, `locked ${target.kind} lane must not reach insert callback`);
}

let rejected = 0;
const reject = () => { rejected += 1; };
assert.equal(await dropRuntime.executeTimelineExternalDrop(
  mediaAudio,
  layers[0],
  layers,
  [avAsset],
  sceneIds,
  0,
  { insertMedia: () => { throw new Error("wrong-kind must not invoke"); }, placeScene: () => {}, reject },
), false);
assert.equal(rejected, 1);
const noAudioLayers = layers.filter((layer) => layer.kind !== "Audio");
assert.equal(await dropRuntime.executeTimelineExternalDrop(
  mediaVideo,
  noAudioLayers[0],
  noAudioLayers,
  [avAsset],
  sceneIds,
  0,
  { insertMedia: () => { throw new Error("missing companion must not invoke"); }, placeScene: () => {}, reject },
), false);
assert.equal(rejected, 2);

let unavailableInsertCalls = 0;
for (const availability of [
  { kind: "missing", asset_id: 7 },
  { kind: "hash_mismatch", asset_id: 7, expected: { algorithm: "Sha256", hex: "a" }, actual: { algorithm: "Sha256", hex: "b" } },
  { kind: "unreadable", asset_id: 7, error: "unreadable" },
  { kind: "live_source", asset_id: 7 },
]) {
  assert.equal(await dropRuntime.executeTimelineExternalDrop(
    mediaVideo,
    layers[0],
    layers,
    [avAsset],
    sceneIds,
    0,
    {
      insertMedia: () => { unavailableInsertCalls += 1; return true; },
      placeScene: () => {},
      reject,
    },
    { 7: availability },
  ), false);
}
assert.equal(unavailableInsertCalls, 0, "unavailable media must not reach the production insert adapter");
assert.equal(rejected, 6);

let unverifiedInsertCalls = 0;
assert.equal(await dropRuntime.executeTimelineExternalDrop(
  mediaVideo,
  layers[0],
  layers,
  [avAsset],
  sceneIds,
  0,
  { insertMedia: () => { unverifiedInsertCalls += 1; return true; }, placeScene: () => {}, reject },
  { 7: { kind: "available_unverified", asset_id: 7 } },
), true);
assert.equal(unverifiedInsertCalls, 1);

let notCheckedInsertCalls = 0;
assert.equal(await dropRuntime.executeTimelineExternalDrop(
  mediaVideo,
  layers[0],
  layers,
  [avAsset],
  sceneIds,
  0,
  { insertMedia: () => { notCheckedInsertCalls += 1; return true; }, placeScene: () => {}, reject },
), true);
assert.equal(notCheckedInsertCalls, 1, "not-yet-inspected media remains placeable but is labeled not verified in the shelf");

let noOpInsertCalls = 0;
assert.equal(await dropRuntime.executeTimelineExternalDrop(
  mediaVideo,
  layers[0],
  layers,
  [avAsset],
  sceneIds,
  0,
  { insertMedia: () => { noOpInsertCalls += 1; return false; }, placeScene: () => {}, reject },
  availableVerified,
), false, "a backend no-op must not be reported as a placed source");
assert.equal(noOpInsertCalls, 1);

let scenePlacement = null;
assert.equal(await dropRuntime.executeTimelineExternalDrop(
  scene,
  layers[2],
  layers,
  [],
  sceneIds,
  55.9,
  {
    insertMedia: () => { throw new Error("media callback must not run for scene"); },
    placeScene: (cueId, layerId, startMs) => { scenePlacement = { cueId, layerId, startMs }; },
  },
), true);
assert.deepEqual(scenePlacement, { cueId: 12, layerId: 13, startMs: 56 });

let staleSceneCalls = 0;
assert.equal(await dropRuntime.executeTimelineExternalDrop(
  scene,
  layers[2],
  layers,
  [],
  new Set(),
  55.9,
  {
    insertMedia: () => { throw new Error("media callback must not run for stale scene"); },
    placeScene: () => { staleSceneCalls += 1; return false; },
  },
), false);
assert.equal(staleSceneCalls, 0, "stale Scene must be rejected before its callback");

// A syntactically valid external MIME payload must still fail closed when its
// Scene is the edited child owner or a nested child Timeline whose reference
// graph returns to that owner. A separate acyclic nested Super Scene remains
// admitted. The production runtime only receives this exact admission set;
// both rejected payloads therefore prove zero Scene callback/IPC/event mutation.
const childTimelineAuthority = bankAuthority.inspectBankAuthority(
  [{ id: 1, label: "Bank 1", active_cue_id: null }],
  [
    { id: 40, cue_list_id: 1, child_timeline: { layers: [], events: [{ id: 1, cue_id: 43 }] } },
    { id: 41, cue_list_id: 1, child_timeline: null },
    { id: 42, cue_list_id: 1, child_timeline: { layers: [], events: [{ id: 1, cue_id: 40 }] } },
    { id: 43, cue_list_id: 1, child_timeline: { layers: [], events: [{ id: 1, cue_id: 41 }] } },
  ],
  [],
);
assert.equal(childTimelineAuthority.issue, null);
const childSafeSceneIds = timelineSceneBlockHelpers.timelineSceneBlockAllowedCueIds(childTimelineAuthority, 40);
assert.deepEqual([... (childSafeSceneIds ?? [])], [41, 43], "child source admission retains only leaf and acyclic nested Super Scene sources");
assert.equal(
  timelineSceneBlockHelpers.timelineSceneBlockCueAllowedByAuthority(40, childTimelineAuthority, 40),
  false,
  "the child owner is not an admitted Scene source",
);
assert.equal(
  timelineSceneBlockHelpers.timelineSceneBlockCueAllowedByAuthority(42, childTimelineAuthority, 40),
  false,
  "a nested Super Scene whose graph returns to the owner is not an admitted Scene source",
);
assert.equal(
  timelineSceneBlockHelpers.timelineSceneBlockCueAllowedByAuthority(43, childTimelineAuthority, 40),
  true,
  "an acyclic nested Super Scene remains an admitted child Timeline source",
);
let forgedChildSceneMutationCalls = 0;
let forgedChildSceneRejects = 0;
for (const cueId of [40, 42]) {
  const forgedChildScene = runtime.parseTimelineExternalDragPayload(JSON.stringify({
    schema: 1,
    kind: "scene",
    cue_id: cueId,
    lane_kind: "Lighting",
  }));
  assert.deepEqual(forgedChildScene, { schema: 1, kind: "scene", cue_id: cueId, lane_kind: "Lighting" });
  assert.equal(await dropRuntime.executeTimelineExternalDrop(
    forgedChildScene,
    layers[2],
    layers,
    [],
    childSafeSceneIds ?? new Set(),
    0,
    {
      insertMedia: () => { throw new Error("forged child Scene must not call media placement"); },
      placeScene: () => { forgedChildSceneMutationCalls += 1; return true; },
      reject: () => { forgedChildSceneRejects += 1; },
    },
  ), false, `forged child Timeline Scene ${cueId} must be rejected before mutation`);
}
assert.equal(forgedChildSceneMutationCalls, 0, "forged child Timeline Scenes cause zero production placement callbacks/IPC/events");
assert.equal(forgedChildSceneRejects, 2, "each forged child Timeline Scene reports exactly one rejection");
let acyclicNestedSceneMutationCalls = 0;
assert.equal(await dropRuntime.executeTimelineExternalDrop(
  runtime.parseTimelineExternalDragPayload(JSON.stringify({ schema: 1, kind: "scene", cue_id: 43, lane_kind: "Lighting" })),
  layers[2],
  layers,
  [],
  childSafeSceneIds ?? new Set(),
  0,
  {
    insertMedia: () => { throw new Error("acyclic nested Scene must not call media placement"); },
    placeScene: () => { acyclicNestedSceneMutationCalls += 1; return true; },
  },
), true, "an acyclic nested Super Scene must reach the same production placement runtime");
assert.equal(acyclicNestedSceneMutationCalls, 1, "an acyclic nested Super Scene gets exactly one placement callback");

// TimelineOverview rejects malformed data, no target, kind mismatch, and a
// locked target before invoking the App/backend callback. Exercise its actual
// exported production seam, then run each source string through the exact App
// localization edge used by onTimelineStatus.
const overviewTarget = layers[0];
const overviewRejects = [
  "This Timeline drop is not a recognized source.",
  "Choose a Timeline lane before dropping a source.",
  "Video sources can only be dropped on a Video lane.",
  "Timeline lane Video A is locked. No source was placed.",
];
assert.deepEqual([
  overviewPreflight.timelineOverviewExternalDropPreflight(null, overviewTarget, true),
  overviewPreflight.timelineOverviewExternalDropPreflight(mediaVideo, null, true),
  overviewPreflight.timelineOverviewExternalDropPreflight(mediaVideo, layers[2], true),
  overviewPreflight.timelineOverviewExternalDropPreflight(mediaVideo, { ...overviewTarget, locked: true }, true),
], overviewRejects);
assert.equal(
  overviewPreflight.timelineOverviewExternalDropPreflight(scene, layers[2], false),
  "Scene 12 is unavailable until Bank authority is repaired.",
  "a Bank-authority fault blocks a Scene drop before App/backend callbacks",
);
assert.equal(
  overviewPreflight.timelineOverviewExternalDropPreflight(scene, layers[2], true),
  null,
  "repair restores only a currently authoritative Scene source",
);
assert.equal(
  overviewPreflight.timelineOverviewExternalDropPreflight(
    runtime.parseTimelineExternalDragPayload(JSON.stringify({ schema: 1, kind: "scene", cue_id: 42, lane_kind: "Lighting" })),
    layers[2],
    timelineSceneBlockHelpers.timelineSceneBlockCueAllowedByAuthority(42, childTimelineAuthority, 40),
  ),
  "Scene 42 is unavailable until Bank authority is repaired.",
  "TimelineOverview preflight rejects a forged nested-child Scene before its App callback",
);
assert.match(overviewSource, /bankAuthority: FullBankAuthoritySnapshot;/);
assert.match(overviewSource, /timelineChildCueId: number \| null;/);
assert.match(overviewSource, /sceneSourceAuthoritative: boolean,/);
assert.doesNotMatch(overviewSource, /sceneSourceAuthoritative = true/);
assert.match(overviewSource, /timelineSceneBlockCueAllowedByAuthority\(\s*source\.cue_id,\s*props\.bankAuthority,\s*props\.timelineChildCueId,\s*\)/);
assert.match(overviewSource, /timelineSceneBlockCueAllowedByAuthority\(\s*placement\.cueId,\s*props\.bankAuthority,\s*props\.timelineChildCueId,\s*\)/);

const internalAvailabilityTokens = /\b(?:missing|hash_mismatch|unreadable|live_source)\b/;
const assertLocalized = (source, expectedEn, expectedJa) => {
  assert.equal(localization.translateUiText(source, "en"), expectedEn, `English status changed: ${source}`);
  const japanese = localization.translateUiText(source, "ja");
  assert.equal(japanese, expectedJa, `Japanese status mismatch: ${source}`);
  assert.ok(/[\u3040-\u30ff\u3400-\u9fff]/.test(japanese), `Japanese status has no Japanese text: ${japanese}`);
  assert.doesNotMatch(japanese, internalAvailabilityTokens, `Japanese status leaked an internal availability token: ${japanese}`);
};
for (const source of overviewRejects) {
  assertLocalized(source, source, localization.translateUiText(source, "ja"));
}
for (const source of [
  "Select an unlocked Audio Timeline lane before placing media.",
  "An unlocked Audio Timeline lane is required.",
  "Selected Audio Timeline lane is no longer available.",
]) {
  assertLocalized(source, source, localization.translateUiText(source, "ja"));
}

// Exercise all runtime rejection families through the production orchestrator.
// Callback and mutation counts remain zero for every rejected source.
const runtimeCases = [
  { source: mediaVideo, target: layers[2], layers, assets: [avAsset], scenes: sceneIds, availability: undefined, expected: "Video sources can only be dropped on a Video lane." },
  { source: mediaVideo, target: { ...layers[0], locked: true }, layers, assets: [avAsset], scenes: sceneIds, availability: undefined, expected: "Timeline lane Video A is locked. No source was placed." },
  { source: scene, target: layers[2], layers, assets: [], scenes: new Set(), availability: undefined, expected: "Scene 12 is no longer available." },
  { source: { ...mediaVideo, media_asset_id: 99 }, target: layers[0], layers, assets: [avAsset], scenes: sceneIds, availability: undefined, expected: "Media Asset 99 is no longer available." },
  { source: mediaVideo, target: layers[0], layers, assets: [avAsset], scenes: sceneIds, availability: { 7: { kind: "missing", asset_id: 7 } }, expected: "Media Asset Show MP4 is unavailable for Timeline placement (missing)." },
  { source: mediaVideo, target: layers[0], layers, assets: [avAsset], scenes: sceneIds, availability: { 7: { kind: "hash_mismatch", asset_id: 7, expected: { algorithm: "Sha256", hex: "a" }, actual: { algorithm: "Sha256", hex: "b" } } }, expected: "Media Asset Show MP4 is unavailable for Timeline placement (hash_mismatch)." },
  { source: mediaVideo, target: layers[0], layers, assets: [avAsset], scenes: sceneIds, availability: { 7: { kind: "unreadable", asset_id: 7, error: "unreadable" } }, expected: "Media Asset Show MP4 is unavailable for Timeline placement (unreadable)." },
  { source: mediaVideo, target: layers[0], layers, assets: [avAsset], scenes: sceneIds, availability: { 7: { kind: "live_source", asset_id: 7 } }, expected: "Media Asset Show MP4 is unavailable for Timeline placement (live_source)." },
  { source: mediaVideo, target: layers[0], layers, assets: [{ ...avAsset, source: { ...avAsset.source, metadata: { duration_ms: 1000, has_audio: true } } }], scenes: sceneIds, availability: undefined, expected: "Video source is unavailable for Show MP4." },
  { source: mediaVideo, target: { id: 14, label: "Video B", order: 1, muted: false, locked: false, solo: false, kind: "Video" }, layers, assets: [avAsset], scenes: sceneIds, availability: undefined, expected: "Selected Video Timeline lane is no longer available." },
];
const expectedJapanese = new Map([
  ["This Timeline drop is not a recognized source.", "このタイムラインドロップは認識されたソースではありません。"],
  ["Choose a Timeline lane before dropping a source.", "ソースをドロップする前にタイムラインレーンを選択してください。"],
  ["Video sources can only be dropped on a Video lane.", "映像ソースは同じ種類のタイムラインレーンにのみドロップできます。"],
  ["Timeline lane Video A is locked. No source was placed.", "タイムラインレーン Video A はロックされています。ソースは配置されませんでした。"],
  ["Scene 12 is no longer available.", "シーン 12 は利用できなくなりました。"],
  ["Media Asset 99 is no longer available.", "メディア素材 99 は利用できなくなりました。"],
  ["Media Asset Show MP4 is unavailable for Timeline placement (missing).", "メディア素材 Show MP4 はタイムラインに配置できません（見つかりません）。"],
  ["Media Asset Show MP4 is unavailable for Timeline placement (hash_mismatch).", "メディア素材 Show MP4 はタイムラインに配置できません（コンテンツハッシュが一致しません）。"],
  ["Media Asset Show MP4 is unavailable for Timeline placement (unreadable).", "メディア素材 Show MP4 はタイムラインに配置できません（読み取れません）。"],
  ["Media Asset Show MP4 is unavailable for Timeline placement (live_source).", "メディア素材 Show MP4 はタイムラインに配置できません（ライブソースです）。"],
  ["Video source is unavailable for Show MP4.", "映像ソースは Show MP4 では利用できません。"],
  ["Selected Video Timeline lane is no longer available.", "選択した映像タイムラインレーンは利用できなくなりました。"],
]);
for (const source of overviewRejects) assertLocalized(source, source, expectedJapanese.get(source));

let rejectedCallbacks = 0;
let rejectedMutations = 0;
for (const testCase of runtimeCases) {
  const statuses = [];
  const placed = await dropRuntime.executeTimelineExternalDrop(
    testCase.source,
    testCase.target,
    testCase.layers,
    testCase.assets,
    testCase.scenes,
    0,
    {
      insertMedia: () => { rejectedMutations += 1; return false; },
      placeScene: () => { rejectedMutations += 1; return false; },
      reject: (message) => { rejectedCallbacks += 1; statuses.push(message); },
      localize: (source) => localization.translateUiText(source, "ja"),
    },
    testCase.availability,
  );
  assert.equal(placed, false, `rejection must fail closed: ${testCase.expected}`);
  assert.deepEqual(statuses, [localization.translateUiText(testCase.expected, "ja")], `localized rejection mismatch: ${testCase.expected}`);
  assertLocalized(testCase.expected, testCase.expected, expectedJapanese.get(testCase.expected));
}
assert.equal(rejectedCallbacks, runtimeCases.length, "each runtime rejection must publish exactly once");
assert.equal(rejectedMutations, 0, "rejected external drops must not invoke a production mutation callback");

const sourceShelf = await readFile(new URL("../src/components/TimelineSourceShelf.tsx", import.meta.url), "utf8");
assert.match(sourceShelf, /role="tablist" aria-label="Timeline source or inspector"/);
assert.match(sourceShelf, /id="timeline-source-context-tab-sources" role="tab"/);
assert.match(sourceShelf, /id="timeline-source-context-panel-scenes" role="tabpanel"/);
assert.match(sourceShelf, /id="timeline-source-context-panel-media" role="tabpanel"/);
assert.equal(sourceShelf.match(/id="timeline-source-context-panel-scenes"/g)?.length, 1);
assert.equal(sourceShelf.match(/id="timeline-source-context-panel-media"/g)?.length, 1);
assert.match(sourceShelf, /id="timeline-source-context-panel-inspector" role="tabpanel"/);
assert.match(sourceShelf, /role="group" aria-label="Timeline source categories"/);
assert.match(sourceShelf, /aria-pressed=\{sourceShelfTab\(\) === "Scenes"\}/);
assert.match(sourceShelf, /data-timeline-source-shelf-filter/);
assert.match(
  sourceShelf,
  /const placementPayload = timelineExternalDragPayloadForScene\(cue\.id\)[\s\S]*?onDragStart=\{\(event\) => startSourceShelfDrag\(event, placementPayload\)\}[\s\S]*?onClick=\{\(\) => placeSourceShelfPayload\(placementPayload\)\}/,
  "the mounted Scene source must feed the same strict payload to drag and accessible click",
);
assert.match(
  sourceShelf,
  /const placeSourceShelfPayload = \(payload:[\s\S]*?const resolution = sourceShelfLayerResolution\(payload\.lane_kind\);[\s\S]*?const target = resolution\.selected;[\s\S]*?void props\.onPlace\(payload, target, props\.snapTimeMs\(props\.positionMs\)\);/,
  "the accessible source action must auto-resolve a sole lane and call the shared placement edge",
);
assert.match(
  sourceShelf,
  /const renderTargetSelect = \(kind: TimelineLayerKind, label: string\) => \([\s\S]*?candidates\.length > 1[\s\S]*?mode === "stale"/,
  "the lane selector is rendered only for ambiguity or to repair a stale explicit click target",
);
assert.match(
  sourceShelf,
  /data-timeline-source-no-target-kind=\{kind\}/,
  "zero unlocked lanes have a visible fail-closed state",
);
assert.match(
  (await readFile(new URL("../src/timelineExternalDropRuntime.ts", import.meta.url), "utf8")),
  /const currentTargetLayer = layers\.find\(\(layer\) => layer\.id === targetLayer\.id\);[\s\S]*?source\.lane_kind === "Video"[\s\S]*?currentTargetLayer[\s\S]*?source\.lane_kind === "Audio"[\s\S]*?currentTargetLayer/,
  "the exact Timeline drop lane is the media primary target",
);
assert.match(
  appSource,
  /<TimelineSourceShelf[\s\S]*?onPlace=\{placeTimelineExternalSource\}/,
  "the mounted Timeline Sources shelf must use the App production placement orchestrator",
);
assert.match(
  appSource,
  /const placeTimelineExternalSource = async \([\s\S]*?await executeTimelineExternalDrop\([\s\S]*?placeScene: async \([\s\S]*?await placeArmedTimelineCue\(/,
  "the mounted App placement edge must pass Scene payloads through strict drop validation before mutation",
);
assert.match(
  appSource,
  /const placeArmedTimelineCue = async \([\s\S]*?const cue = requireTimelineSceneBlockCue\(cueId\);[\s\S]*?await timelineSceneBlocks\.addAt\(cue\.id, timeMs, "Lighting", false, \{[\s\S]*?layerId,/,
  "validated Scene placement must retain the exact selected Lighting layer",
);
assert.match(
  appSource,
  /const timelineSceneBlocks = createTimelineSceneBlockController\(\{[\s\S]*?invoke: invokeTimelineSceneBlockCommand,/,
  "Scene placement must use the shared production Timeline Scene Block controller",
);
assert.match(
  appSource,
  /const invokeTimelineSceneBlockCommand = async <T,>\([\s\S]*?return invoke<T>\(command, args\);/,
  "the non-fixture Timeline Scene Block controller must terminate at the registered Tauri invoke edge",
);
assert.match(
  timelineSceneBlocksSource,
  /const eventId = await options\.invoke<number>\("add_timeline_scene_block", \{[\s\S]*?cueId: next\.cue_id,[\s\S]*?timeMs: next\.time_ms,[\s\S]*?track: next\.track,[\s\S]*?layerId: next\.layer_id,/,
  "the shared controller must send cue, snapped time, track, and exact layer through add_timeline_scene_block",
);

console.log("timeline external DnD contract: PASS (strict MIME parser, exact drop-lane primary, sole-click auto-resolution, explicit linked companions, zero-candidate fail-closed, mounted Sources/Inspector, registered production mutation chain)");
