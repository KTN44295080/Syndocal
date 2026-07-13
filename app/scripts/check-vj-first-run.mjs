import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const app = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
const backend = await readFile(new URL("../src-tauri/src/main.rs", import.meta.url), "utf8");
const engine = await readFile(new URL("../../crates/engine/src/lib.rs", import.meta.url), "utf8");
const clipGrid = await readFile(new URL("../src/components/VideoClipGridPanel.tsx", import.meta.url), "utf8");
const liveMonitors = await readFile(new URL("../src/components/LiveVideoMonitorPanel.tsx", import.meta.url), "utf8");

const workflowStart = app.indexOf("const createFirstRunVjShow = async () => {");
const workflowEnd = app.indexOf("const videoThumbnailSourceSignature", workflowStart);
assert.ok(workflowStart >= 0 && workflowEnd > workflowStart, "first-run VJ workflow is missing");
const workflow = app.slice(workflowStart, workflowEnd);
assert.ok(
  workflow.indexOf('"select_video_source_files"') < workflow.indexOf('"bootstrap_vj_show"'),
  "file selection must happen before the first project mutation",
);
assert.ok(workflow.includes("if (vjFirstRunBusy()) return;"), "double activation must be ignored");
assert.ok(workflow.includes("if (paths.length === 0)"), "cancel must return before setup mutation");
assert.ok(workflow.includes("if (!refreshed)"), "post-commit snapshot refresh failure must be reconciled separately");
assert.ok(app.includes('"bootstrap_vj_show",'), "bootstrap_vj_show must be one project transaction");
for (const forbidden of [
  "openVideoOutputWindow",
  "syncExternalVideoTransports",
  "takeVideoClip",
  "launchVideoClip",
  "setVideoOutputEnabled",
  "setVideoOutputBlackout",
]) {
  assert.ok(!workflow.includes(forbidden), `first-run VJ workflow must not call ${forbidden}`);
}
assert.ok(workflow.includes("setSelectedVideoOutputId(result.output_id)"));
assert.ok(workflow.includes("await stageVjPreviewLayer(firstLayerId)"));
assert.ok(!workflow.includes("setVideoPreviewLayerId("), "first-run Preview must be staged by the runtime backend");
assert.ok(backend.includes("enabled: false"));
assert.ok(backend.includes("blackout: true"));
assert.ok(backend.includes("fullscreen: false"));
assert.ok(engine.includes("EngineCommand::BootstrapVjShow"));
assert.ok(engine.includes("let publication_barrier = matches!("));
assert.ok(engine.includes("self.publish_pending_command_acks(queue.len(), &snapshot)"));
assert.ok(engine.includes("recv_timeout(Duration::from_secs(3))"));
assert.ok(engine.includes("First-run VJ setup expired before engine execution"));
assert.ok(backend.includes("async fn bootstrap_vj_show("));
assert.ok(backend.includes("tauri::async_runtime::spawn_blocking(move ||"));
assert.ok(clipGrid.includes('aria-busy={props.firstRunBusy}'));
assert.ok(clipGrid.includes('querySelector<HTMLButtonElement>(".videoClipLaunch")?.focus()'));

const mutationCommandsStart = app.indexOf("const projectMutationCommands = new Set([");
const mutationCommandsEnd = app.indexOf("]);", mutationCommandsStart);
assert.ok(mutationCommandsStart >= 0 && mutationCommandsEnd > mutationCommandsStart);
const mutationCommands = app.slice(mutationCommandsStart, mutationCommandsEnd);
for (const command of [
  "get_vj_preview_transport",
  "stage_vj_preview_layer",
  "set_vj_preview_playing",
  "seek_vj_preview",
  "set_vj_preview_speed",
  "clear_vj_preview",
]) {
  assert.ok(app.includes(`"${command}"`), `${command} frontend command is missing`);
  assert.ok(backend.includes(command), `${command} backend command is missing`);
  assert.ok(!mutationCommands.includes(`"${command}"`), `${command} must not create project history`);
}
assert.ok(app.includes("vjPreviewPollInFlight"), "Preview polling must remain single-flight");
assert.ok(clipGrid.includes("onStagePreview(layer.id)"), "P buttons must stage the runtime Preview");
assert.ok(clipGrid.includes("!props.previewBackendAvailable"), "browser P buttons must stay disabled");
assert.ok(liveMonitors.includes("PREVIEW TRANSPORT"));
assert.ok(liveMonitors.includes("props.onSetPreviewPlaying"));
assert.ok(liveMonitors.includes("props.onSeekPreview"));
assert.ok(liveMonitors.includes("props.onSetPreviewSpeed"));
assert.ok(liveMonitors.includes("props.onClearPreview"));

console.log("safe first-run VJ workflow and independent Preview transport ok");
