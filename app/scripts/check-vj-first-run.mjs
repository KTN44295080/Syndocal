import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const app = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
const backend = await readFile(new URL("../src-tauri/src/main.rs", import.meta.url), "utf8");
const engine = await readFile(new URL("../../crates/engine/src/lib.rs", import.meta.url), "utf8");
const mediaAuthority = await readFile(new URL("../src/mediaAssetAuthority.ts", import.meta.url), "utf8");
const clipGrid = await readFile(new URL("../src/components/VideoClipGridPanel.tsx", import.meta.url), "utf8");
const liveMonitors = await readFile(new URL("../src/components/LiveVideoMonitorPanel.tsx", import.meta.url), "utf8");

const workflowStart = app.indexOf("const createFirstRunVjShow = async () => {");
const workflowEnd = app.indexOf("const videoThumbnailSourceSignature", workflowStart);
assert.ok(workflowStart >= 0 && workflowEnd > workflowStart, "first-run VJ workflow is missing");
const workflow = app.slice(workflowStart, workflowEnd);
assert.ok(
  workflow.indexOf("preflightAndBeginMediaAssetOperation") < workflow.indexOf('"select_video_source_files"'),
  "first-run must flush mappings and only then register its AbortController before the picker",
);
assert.ok(
  workflow.indexOf('"select_video_source_files"') < workflow.indexOf("prepareFinalizeAndCommitMediaAssets"),
  "first-run selection must happen before handing the batch to staged preparation",
);
const startMediaAssetOperation = mediaAuthority.indexOf('"start_media_asset_operation"');
const prepareReservedMediaAssets = mediaAuthority.indexOf('"prepare_reserved_media_assets"');
const finalizePreparedMediaAssets = mediaAuthority.indexOf('"finalize_prepared_media_assets"');
const authoritativeMediaCommit = mediaAuthority.indexOf("const response = await invokeAuthoritativeCommit(");
assert.ok(
  startMediaAssetOperation >= 0
    && prepareReservedMediaAssets > startMediaAssetOperation
    && finalizePreparedMediaAssets > prepareReservedMediaAssets
    && authoritativeMediaCommit > finalizePreparedMediaAssets,
  "first-run reserves, prepares, and finalizes before its one authoritative project mutation",
);
assert.ok(workflow.includes("if (vjFirstRunBusy()) return;"), "double activation must be ignored");
assert.ok(workflow.includes("if (paths.length === 0)"), "cancel must return before setup mutation");
assert.ok(workflow.includes("if (!refreshed)"), "post-commit snapshot refresh failure must be reconciled separately");
assert.ok(
  (workflow.match(/if \(!terminalIsCurrent\(\)\)/g) ?? []).length >= 3,
  "project C must stop the Bootstrap continuation after snapshot/Preview awaits and before B side effects",
);
assert.ok(workflow.includes("requireAllPrepared: true"), "first-run mixed Prepare result must reject before the project mutation");
assert.ok(
  workflow.includes('commitCommand: "commit_prepared_bootstrap_vj_show_authoritative"'),
  "first-run must use one backend-owned authoritative Bootstrap transaction",
);
const authoritativeCommandsStart = app.indexOf("const serverAuthoritativeProjectMutationCommands = new Set([");
const authoritativeCommandsEnd = app.indexOf("]);", authoritativeCommandsStart);
assert.ok(authoritativeCommandsStart >= 0 && authoritativeCommandsEnd > authoritativeCommandsStart);
assert.ok(
  app.slice(authoritativeCommandsStart, authoritativeCommandsEnd).includes('"commit_prepared_bootstrap_vj_show_authoritative"'),
  "authoritative Bootstrap remains explicitly classified as a mutation",
);
const firstRunAvailabilityStart = app.indexOf("const vjFirstRunAvailable = createMemo(() =>");
const firstRunAvailabilityEnd = app.indexOf("  createEffect(() =>", firstRunAvailabilityStart);
assert.ok(firstRunAvailabilityStart >= 0 && firstRunAvailabilityEnd > firstRunAvailabilityStart, "first-run availability predicate is missing");
const firstRunAvailability = app.slice(firstRunAvailabilityStart, firstRunAvailabilityEnd);
assert.ok(
  firstRunAvailability.includes("snapshot().video.media_assets.length === 0"),
  "catalog-only assets make the show nonempty, so first-run CTA must be unavailable",
);
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
assert.ok(
  workflow.includes("await stageVjPreviewLayer(firstLayerId, staged.terminalAuthority)"),
  "automatic Preview staging must carry the exact committed project authority",
);
assert.ok(!workflow.includes("setVideoPreviewLayerId("), "first-run Preview must be staged by the runtime backend");
assert.ok(backend.includes("enabled: false"));
assert.ok(backend.includes("blackout: true"));
assert.ok(backend.includes("fullscreen: false"));
assert.ok(engine.includes("EngineCommand::BootstrapVjShow"));
assert.ok(engine.includes("let publication_barrier = matches!("));
assert.ok(engine.includes("self.publish_pending_command_acks(queue.len(), &snapshot)"));
assert.ok(engine.includes("recv_timeout(Duration::from_secs(3))"));
assert.ok(engine.includes("First-run VJ setup expired before engine execution"));
// The old bridge stays registered for older renderer binaries, but current
// production App routing uses the staged name checked above.
assert.ok(backend.includes("fn bootstrap_vj_show("), "legacy bootstrap compatibility command remains available");
assert.ok(backend.includes("fn commit_prepared_bootstrap_vj_show("), "staged bootstrap backend command is available");
const previewStageCommand = backend.slice(
  backend.indexOf("fn stage_vj_preview_layer("),
  backend.indexOf("fn set_vj_preview_playing(", backend.indexOf("fn stage_vj_preview_layer(")),
);
assert.match(
  previewStageCommand,
  /expected_epoch[\s\S]*?expected_revision[\s\S]*?expected_checkpoint_hash/,
  "Preview stage backend accepts the exact E\/R\/H fence",
);
assert.match(
  previewStageCommand,
  /lock_project_external_command_admission[\s\S]*?lock_project_coordinator[\s\S]*?with_exact_vj_preview_project_authority/,
  "Preview stage validates authority under the canonical admission boundary",
);
assert.ok(backend.includes("fn commit_prepared_bootstrap_vj_show_authoritative("), "authoritative Bootstrap backend command is available");
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
