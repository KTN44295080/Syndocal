import { readFile } from "node:fs/promises";
import assert from "node:assert/strict";
import ts from "typescript";

async function importTsModule(path) {
  const source = await readFile(new URL(path, import.meta.url), "utf8");
  const transpiled = ts.transpileModule(source, {
    compilerOptions: {
      module: ts.ModuleKind.ESNext,
      target: ts.ScriptTarget.ES2022,
      importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
    },
    fileName: path,
  });
  return import(`data:text/javascript;base64,${Buffer.from(transpiled.outputText).toString("base64")}`);
}

const recent = await importTsModule("../src/projectRecentStorage.ts");
const recovery = await importTsModule("../src/projectRecoveryStorage.ts");
const projectSnapshot = await importTsModule("../src/projectSnapshot.ts");
const workspaceLayout = await importTsModule("../src/workspaceLayoutStorage.ts");

assert.equal(recent.recentProjectFileName("C:\\shows\\main.sdc"), "main.sdc");
assert.equal(recent.recentProjectFileName("/shows/main.sdc"), "main.sdc");

assert.deepEqual(
  recent.recentProjectPathsFromUnknown([
    " C:/shows/main.sdc ",
    "C:/shows/MAIN.SDC",
    "C:/shows/notes.txt",
    42,
    "C:/shows/backup.sdc",
  ]),
  ["C:/shows/main.sdc", "C:/shows/backup.sdc"],
);
const legacyProjectPath = `C:/shows/legacy.${["r", "y"].join("")}`;
assert.deepEqual(recent.recentProjectPathsFromUnknown([legacyProjectPath]), []);

const manyProjects = Array.from({ length: 12 }, (_, index) => `C:/shows/show-${index}.sdc`);
assert.equal(recent.recentProjectPathsFromUnknown(manyProjects).length, 8);
assert.deepEqual(
  recent.touchRecentProjectPath(["C:/shows/a.sdc", "C:/shows/b.sdc"], "C:/shows/B.SDC"),
  ["C:/shows/B.SDC", "C:/shows/a.sdc"],
);
assert.deepEqual(recent.touchRecentProjectPath(["C:/shows/a.sdc"], "C:/shows/b.txt"), ["C:/shows/a.sdc"]);

const project = {
  version: 1,
  app: "Syndocal",
  custom_profiles: [],
  midi_mappings: [{ channel: 0, message: "ControlChange", number: 74, action: "LightingMaster" }],
  osc_mappings: [{ address: "/show/go", action: "TriggerNextCue" }],
  snapshot: { fixtures: [], cues: [] },
};
const checkpoint = recovery.createProjectRecoveryCheckpoint(project, "C:/shows/main.sdc", "sig-1");
assert.equal(checkpoint.version, 1);
assert.equal(checkpoint.app, "Syndocal");
assert.equal(checkpoint.project, project);
assert.equal(checkpoint.project.midi_mappings[0].number, 74);
assert.equal(checkpoint.project.osc_mappings[0].address, "/show/go");
assert.equal(checkpoint.signature, "sig-1");
assert.equal(checkpoint.source_path, "C:/shows/main.sdc");
assert.equal(recovery.projectRecoverySourceLabel(checkpoint), "main.sdc");

assert.deepEqual(recovery.recoveryCheckpointFromUnknown(checkpoint), checkpoint);
assert.equal(
  recovery.recoveryCheckpointFromUnknown({ ...checkpoint, source_path: " C:/shows/spaced.sdc " }).source_path,
  "C:/shows/spaced.sdc",
);
assert.equal(recovery.recoveryCheckpointFromUnknown({ ...checkpoint, app: "Other" }), null);
assert.equal(recovery.recoveryCheckpointFromUnknown({ ...checkpoint, project: { ...project, app: "Other" } }), null);
assert.equal(recovery.recoveryCheckpointFromUnknown({ ...checkpoint, project: { ...project, snapshot: null } }), null);
assert.equal(recovery.recoveryCheckpointFromUnknown({ ...checkpoint, project: { ...project, snapshot: {} } }), null);
assert.equal(recovery.projectRecoverySourceLabel({ ...checkpoint, source_path: null }), "Untitled.sdc");
assert.equal(recovery.projectRecoveryTimeLabel({ ...checkpoint, saved_at: "bad-date" }), "Recovery");

const autoVjConfig = {
  eligible_layer_ids: [5, 7],
  seed: 42,
  beats_per_change: 4,
  transition_ms: 500,
  avoid_immediate_repeat: true,
  rhythm_source: "LiveAudio",
};
const runtimeAutoVjAction = {
  sequence: 3,
  boundary_index: 2,
  beat: 8,
  layer_id: 7,
  transition_ms: 500,
  selection_token: 99,
  seed: 42,
  show_revision: 4,
  trigger: "LiveAudioOnset",
  live_audio_feature_sequence: 17,
};
const runtimeAutoVjStatus = {
  mode: "Running",
  armed: true,
  hold: false,
  show_revision: 4,
  action_sequence: 3,
  last_consumed_boundary: 2,
  next_boundary_beat: 12,
  last_action: runtimeAutoVjAction,
  action_log: [runtimeAutoVjAction],
  fault: null,
  live_audio_beat_counter: 8,
  last_live_audio_feature_sequence: 17,
};
const autoVjProjectSnapshot = {
  fixtures: [],
  cues: [],
  active_cue_id: null,
  active_fade: null,
  timeline: { playing: false, position_ms: 0 },
  video: {
    layers: [],
    auto_vj: { config: autoVjConfig, status: runtimeAutoVjStatus },
  },
  clock: { beat_phase: 0, beat_counter: 0, tap_count: 0 },
  dmx_preview: [],
  dmx_previews: [],
  telemetry: {},
};
const runtimeOnlyAutoVjChange = JSON.parse(JSON.stringify(autoVjProjectSnapshot));
runtimeOnlyAutoVjChange.video.auto_vj.status = {
  ...runtimeOnlyAutoVjChange.video.auto_vj.status,
  mode: "Hold",
  hold: true,
  action_sequence: 99,
  action_log: [
    ...runtimeOnlyAutoVjChange.video.auto_vj.status.action_log,
    { ...runtimeAutoVjAction, sequence: 99, boundary_index: 98 },
  ],
  fault: "runtime-only fault",
  live_audio_beat_counter: 200,
};
assert.equal(
  projectSnapshot.projectSnapshotSignature(autoVjProjectSnapshot),
  projectSnapshot.projectSnapshotSignature(runtimeOnlyAutoVjChange),
  "Auto VJ runtime status must not dirty a project or rotate recovery/autosave signatures",
);
const changedAutoVjConfig = JSON.parse(JSON.stringify(autoVjProjectSnapshot));
changedAutoVjConfig.video.auto_vj.config.seed = 43;
assert.notEqual(
  projectSnapshot.projectSnapshotSignature(autoVjProjectSnapshot),
  projectSnapshot.projectSnapshotSignature(changedAutoVjConfig),
  "Auto VJ deterministic configuration remains project data",
);
const persistedAutoVjSnapshot = projectSnapshot.normalizeProjectAutoVjForPersistence(autoVjProjectSnapshot);
assert.deepEqual(persistedAutoVjSnapshot.video.auto_vj.config, autoVjConfig);
assert.deepEqual(
  persistedAutoVjSnapshot.video.auto_vj.status,
  projectSnapshot.defaultPersistedAutoVjStatus(),
  "frontend persistence normalization matches backend project_snapshot_for_save",
);
assert.equal(autoVjProjectSnapshot.video.auto_vj.status.mode, "Running", "normalization must not mutate live state");
assert.deepEqual(
  projectSnapshot.normalizeProjectAutoVjForPersistence(persistedAutoVjSnapshot),
  persistedAutoVjSnapshot,
  "Auto VJ persistence normalization is idempotent",
);

const audioRuntimeSnapshot = JSON.parse(JSON.stringify(autoVjProjectSnapshot));
audioRuntimeSnapshot.video.layers = [{ id: 1, opacity: 0.92 }];
audioRuntimeSnapshot.authored_video = {
  ...audioRuntimeSnapshot.video,
  layers: [{ id: 1, opacity: 0.4 }],
};
audioRuntimeSnapshot.node_graphs = [{
  id: 7,
  label: "Kick opacity",
  enabled: true,
  nodes: [],
  edges: [],
  audio_runtime: [{
    node_id: 1,
    input_value: 0.8,
    output_value: 0.6,
    source_available: true,
    safety_zeroed: false,
    held: false,
    feature_sequence: 42,
  }],
}];
const storedAudioRuntimeSnapshot = projectSnapshot.normalizeProjectSnapshotForStorage(audioRuntimeSnapshot);
assert.equal(storedAudioRuntimeSnapshot.video.layers[0].opacity, 0.4, "authored video replaces rendered modulation at the save boundary");
assert.equal("authored_video" in storedAudioRuntimeSnapshot, false, "authored transport copy is not nested into project data");
assert.equal("audio_runtime" in storedAudioRuntimeSnapshot.node_graphs[0], false, "audio meters are not project data");
const changedAudioRuntimeSnapshot = JSON.parse(JSON.stringify(audioRuntimeSnapshot));
changedAudioRuntimeSnapshot.video.layers[0].opacity = 0.1;
changedAudioRuntimeSnapshot.node_graphs[0].audio_runtime[0].output_value = 0.05;
assert.equal(
  projectSnapshot.projectSnapshotSignature(audioRuntimeSnapshot),
  projectSnapshot.projectSnapshotSignature(changedAudioRuntimeSnapshot),
  "rendered audio modulation and meters must not dirty the project",
);

const liveMixerStrobeSnapshot = JSON.parse(JSON.stringify(autoVjProjectSnapshot));
liveMixerStrobeSnapshot.submasters = [{
  group_id: "front",
  label: "Front",
  level: 0.75,
  strobe_hz: 12,
  strobe_fixture_count: 4,
}];
const storedLiveMixerStrobeSnapshot = projectSnapshot.normalizeProjectSnapshotForStorage(liveMixerStrobeSnapshot);
assert.equal("strobe_hz" in storedLiveMixerStrobeSnapshot.submasters[0], false, "Live Mixer strobe rate is not project data");
assert.equal("strobe_fixture_count" in storedLiveMixerStrobeSnapshot.submasters[0], false, "strobe compatibility is derived runtime data");
const changedLiveMixerStrobeSnapshot = JSON.parse(JSON.stringify(liveMixerStrobeSnapshot));
changedLiveMixerStrobeSnapshot.submasters[0].strobe_hz = 24;
changedLiveMixerStrobeSnapshot.submasters[0].strobe_fixture_count = 2;
assert.equal(
  projectSnapshot.projectSnapshotSignature(liveMixerStrobeSnapshot),
  projectSnapshot.projectSnapshotSignature(changedLiveMixerStrobeSnapshot),
  "Live Mixer strobe changes must not dirty the project",
);

const dirtySceneBlockDraft = {
  cue_id: 500,
  time_ms: 123_456,
  time_beats: null,
  track: "Video",
  layer_id: null,
  duration_ms: 2_000,
  duration_beats: null,
  conform_to_tempo: false,
  loop_count: 4,
  loop_fill: false,
  jump_to_event_id: 499,
  fade_in_ms: 0,
  fade_out_ms: 0,
  source_offset_ms: 0,
};
const draftCheckpoint = recovery.createProjectRecoveryCheckpoint(
  project,
  "C:/shows/main.sdc",
  "sig-draft-only",
  { 500: dirtySceneBlockDraft },
);
assert.deepEqual(draftCheckpoint.editor_drafts, {
  version: 1,
  timeline_events: { 500: dirtySceneBlockDraft },
});
assert.deepEqual(recovery.recoveryCheckpointFromUnknown(draftCheckpoint), draftCheckpoint);
assert.deepEqual(
  recovery.recoveryCheckpointFromUnknown({
    ...draftCheckpoint,
    editor_drafts: { version: 99, timeline_events: { 500: dirtySceneBlockDraft } },
  }),
  (({ editor_drafts: _editorDrafts, ...legacyCompatible }) => legacyCompatible)(draftCheckpoint),
  "legacy project recovery stays usable when a future draft payload cannot be read",
);

const storageValues = new Map();
const localStorage = {
  getItem: (key) => storageValues.get(key) ?? null,
  setItem: (key, value) => storageValues.set(key, value),
  removeItem: (key) => storageValues.delete(key),
};
globalThis.window = { localStorage };

assert.deepEqual(workspaceLayout.workspaceLayoutFromUnknown(null), workspaceLayout.defaultWorkspaceLayout);
assert.deepEqual(
  workspaceLayout.workspaceLayoutFromUnknown({
    workspace_tab: "control",
    setup_sub_tab: "video",
    control_mode: "live",
    timeline_desk_surface: "playback",
    timeline_context_drawer: "cue",
    edit_desk_surface: "faders",
    control_category: "color",
    future_field: true,
  }),
  {
    workspace_tab: "control",
    setup_sub_tab: "video",
    control_mode: "live",
    timeline_desk_surface: "playback",
    timeline_context_drawer: "cue",
    edit_desk_surface: "faders",
    control_category: "color",
    top_split_ratio: workspaceLayout.defaultWorkspaceLayout.top_split_ratio,
    lower_split_ratio: workspaceLayout.defaultWorkspaceLayout.lower_split_ratio,
    selections_drawer_open: workspaceLayout.defaultWorkspaceLayout.selections_drawer_open,
  },
);
assert.deepEqual(
  workspaceLayout.workspaceLayoutFromUnknown({
    workspace_tab: "control",
    control_mode: "live",
    timeline_desk_surface: "cues",
    timeline_context_drawer: "none",
  }),
  {
    ...workspaceLayout.defaultWorkspaceLayout,
    workspace_tab: "control",
    control_mode: "live",
    timeline_desk_surface: "show",
    timeline_context_drawer: "cue",
  },
  "legacy Cues surfaces migrate to the timeline Cue drawer",
);
assert.deepEqual(
  workspaceLayout.workspaceLayoutFromUnknown({
    workspace_tab: "control",
    control_mode: "edit",
    timeline_desk_surface: "cues",
    timeline_context_drawer: "none",
  }),
  {
    ...workspaceLayout.defaultWorkspaceLayout,
    workspace_tab: "control",
    control_mode: "edit",
    timeline_desk_surface: "show",
    timeline_context_drawer: "cue",
  },
  "legacy Cues migration survives an initially hidden Edit workspace",
);
assert.deepEqual(
  workspaceLayout.workspaceLayoutFromUnknown({
    workspace_tab: "control",
    timeline_desk_surface: "show",
    timeline_context_drawer: "block",
  }),
  {
    ...workspaceLayout.defaultWorkspaceLayout,
    workspace_tab: "control",
    timeline_context_drawer: "none",
  },
  "selection-bound Block Properties drawers do not survive a workspace reload",
);
assert.deepEqual(
  workspaceLayout.workspaceLayoutFromUnknown({
    top_split_ratio: -4,
    lower_split_ratio: 3,
    selections_drawer_open: true,
  }),
  {
    ...workspaceLayout.defaultWorkspaceLayout,
    top_split_ratio: 0.15,
    lower_split_ratio: 0.85,
    selections_drawer_open: true,
  },
);
assert.deepEqual(
  workspaceLayout.workspaceLayoutFromUnknown({
    workspace_tab: "invalid",
    setup_sub_tab: "remote",
    control_mode: 4,
    timeline_desk_surface: "future",
    edit_desk_surface: "dmx",
    control_category: "beam",
  }),
  {
    ...workspaceLayout.defaultWorkspaceLayout,
    setup_sub_tab: "io",
    edit_desk_surface: "attributes",
    control_category: "beam",
  },
);
const savedWorkspaceLayout = {
  ...workspaceLayout.defaultWorkspaceLayout,
  workspace_tab: "control",
  control_mode: "live",
  timeline_desk_surface: "playback",
};
assert.equal(workspaceLayout.saveWorkspaceLayout(savedWorkspaceLayout), true);
assert.deepEqual(workspaceLayout.loadWorkspaceLayout(), savedWorkspaceLayout);

const transientBlockDrawerLayout = {
  ...savedWorkspaceLayout,
  timeline_context_drawer: "block",
};
assert.equal(workspaceLayout.saveWorkspaceLayout(transientBlockDrawerLayout), true);
assert.equal(
  JSON.parse(storageValues.get(workspaceLayout.workspaceLayoutStorageKey)).timeline_context_drawer,
  "none",
  "Block Properties drawer state is normalized at the persistence boundary",
);
assert.deepEqual(
  workspaceLayout.loadWorkspaceLayout(),
  { ...transientBlockDrawerLayout, timeline_context_drawer: "none" },
);

storageValues.set(workspaceLayout.workspaceLayoutStorageKey, "{broken-json");
assert.deepEqual(workspaceLayout.loadWorkspaceLayout(), workspaceLayout.defaultWorkspaceLayout);
assert.equal(storageValues.has(workspaceLayout.workspaceLayoutStorageKey), false);

assert.equal(recovery.saveProjectRecoveryCheckpoint(checkpoint), true);
assert.deepEqual(recovery.loadProjectRecoveryCheckpoint(), checkpoint);
assert.equal(recovery.saveProjectRecoveryCheckpoint(draftCheckpoint), true);
assert.deepEqual(
  recovery.loadProjectRecoveryCheckpoint()?.editor_drafts?.timeline_events?.[500],
  dirtySceneBlockDraft,
  "draft-only Scene Block edits survive a storage reload",
);

storageValues.set("syndocal.projectRecovery.v1", "{broken-json");
assert.equal(recovery.loadProjectRecoveryCheckpoint(), null);
assert.equal(storageValues.has("syndocal.projectRecovery.v1"), false);

globalThis.window = {
  localStorage: {
    ...localStorage,
    setItem: () => {
      throw new Error("quota exceeded");
    },
  },
};
assert.equal(workspaceLayout.saveWorkspaceLayout(savedWorkspaceLayout), false);
assert.equal(recovery.saveProjectRecoveryCheckpoint(checkpoint), false);
delete globalThis.window;

console.log("project storage helpers ok");
