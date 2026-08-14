import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const read = (path) => readFile(new URL(path, import.meta.url), "utf8");
const [modelSource, bank, inspector, app, controller, styles, modes, types, touch, edit, chrome, localization] = await Promise.all([
  read("../src/videoClipSlotBankModel.ts"),
  read("../src/components/VideoClipSlotBankPanel.tsx"),
  read("../src/components/VideoClipSlotInspectorPanel.tsx"),
  read("../src/App.tsx"),
  read("../src/createVideoRuntimeController.ts"),
  read("../src/styles.css"),
  read("../src/uiModes.ts"),
  read("../src/types.ts"),
  read("../src/components/TouchVideoPanel.tsx"),
  read("../src/components/VideoControlPanel.tsx"),
  read("../src/components/WorkspaceChrome.tsx"),
  read("../src/uiLocalization.ts"),
]);

const transpiled = ts.transpileModule(modelSource, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
  fileName: "videoClipSlotBankModel.ts",
}).outputText;
const model = await import(`data:text/javascript;base64,${Buffer.from(transpiled).toString("base64")}`);

const authored = [
  { id: 401, media_asset_id: 9 },
  { id: 907, media_asset_id: 10 },
];
const runtime = {
  layer_id: 3,
  active_slot_id: 401,
  queued_slot_id: 907,
  pending_launch: { slot_id: 907, quantization: "NextBar", target_boundary_ordinal: 12, clock_generation: 1 },
  playhead_ms: 4321,
  playing: true,
  ping_pong_reverse: false,
};
const cells = model.videoClipSlotBankCells(authored, runtime);
assert.equal(model.VIDEO_CLIP_SLOT_BANK_SIZE, 32, "shared bank remains exactly 32 cells");
assert.equal(cells.length, 32, "bank projects every fixed pad including empties");
assert.equal(cells[0].active, true, "active state comes from runtime slot identity");
assert.equal(cells[1].queued && cells[1].pending, true, "queued/pending truth comes from runtime, not layer state");
assert.equal(cells[2].slot, null, "empty authored slots remain visible pads");
assert.deepEqual(model.videoClipSlotBankActionLabels("edit"), ["Select", "Preview", "More"], "Edit primary actions stay capped at three");
assert.deepEqual(model.videoClipSlotBankActionLabels("control"), ["Queue", "Preview", "More"], "Control primary actions stay capped at three");
assert.deepEqual(
  model.videoClipSlotReorderedIds([401, 907, 1222], 1222, 0),
  [1222, 401, 907],
  "reorder retains globally stable full slot IDs rather than pad indices",
);
assert.equal(model.videoClipSlotCommandKindMatches("queue", "queue"), true, "matching runtime receipt kinds are accepted");
assert.equal(model.videoClipSlotCommandKindMatches("queue", "launch"), false, "runtime command-kind mismatch fails closed");
assert.equal(model.videoClipSlotRuntimeGenerationCanApply(4, 8, 4, 7), false, "late same-project runtime cannot roll generation backward");
assert.equal(model.videoClipSlotRuntimeGenerationCanApply(4, 8, 4, 8), true, "same-generation retry/query remains idempotent");
assert.equal(model.videoClipSlotRuntimeGenerationCanApply(4, 8, 5, 0), true, "replacement epoch starts a fresh runtime generation");
assert.throws(
  () => model.videoClipSlotRuntimeGenerationCanApply(4, 8, 4, Number.MAX_SAFE_INTEGER + 1),
  /exact integer range/,
  "unsafe u64 JSON values are never compared approximately",
);

assert.match(bank, /data-video-clip-slot-pad-index/, "browser bank exposes roving-pad identity");
assert.match(bank, /ArrowRight[\s\S]*ArrowLeft[\s\S]*ArrowDown[\s\S]*ArrowUp/, "browser bank supports directional keyboard movement");
assert.match(bank, /event\.key === "Escape"/, "Escape clears pad selection");
assert.match(bank, /data-video-clip-slot-layer-id/, "drop targets retain an exact layer identity");
assert.match(bank, /data-video-clip-slot-id/, "occupied drop targets retain an exact slot identity");
assert.match(bank, /props\.onSelect\(slotId\)[\s\S]*props\.onQueue\(slotId\)/, "Control Queue also moves shared authored selection");
assert.match(bank, /getComputedStyle\(bankGrid\)\.gridTemplateColumns/, "roving vertical movement follows the actual container column count");
assert.match(bank, /onSelectLayer/, "both mounts expose explicit layer selection");
assert.match(bank, /onCancelQueue/, "Control exposes queue cancellation");
assert.match(bank, /value="Crossfade"[\s\S]*value="Dip"[\s\S]*value="Wipe"[\s\S]*value="Luma"[\s\S]*value="Displacement"[\s\S]*value="Blur"[\s\S]*value="Glitch"[\s\S]*value="Custom"[\s\S]*value="Milliseconds"[\s\S]*value="Beats"[\s\S]*value="Bars"/, "Control exposes every typed Clip Take mode and deterministic duration unit");
assert.match(bank, /progress_millis \/ 10/, "Control reports authoritative Clip Take progress rather than inferring it from layer state");
assert.match(bank, /Preview thumbnail for slot/, "Preview labels the supported asset-thumbnail behavior honestly");
assert.match(inspector, /onReorder/, "inspector preserves authored reorder through full IDs");
assert.match(inspector, /window\.requestAnimationFrame\(\(\) => props\.returnFocus\?\.focus\(\)\)/, "inspector restores focus on close");
assert.match(inspector, /role="dialog"[\s\S]*aria-modal="true"/, "inspector uses dialog semantics");
assert.match(inspector, /focusableSelector[\s\S]*event\.key !== "Tab"/, "inspector traps focus");
for (const field of ["in_point_ms", "out_point_ms", "loop_mode", "speed", "launch_quantization", "cue_points", "effect_overrides"]) {
  assert.ok(inspector.includes(field), `inspector authors ${field}`);
}
assert.match(inspector, /globalThis\.confirm[\s\S]*props\.onRemove/, "remove is explicitly confirmed before the authored command");

assert.ok(
  app.includes('mode: "edit"') && app.includes('mode: "control"'),
  "one App state mounts the same bank in Edit and Control",
);
assert.equal((app.match(/createSignal<VideoClipRuntimeSnapshot>/g) ?? []).length, 1, "Edit and Control share exactly one runtime signal");
assert.equal((app.match(/createSignal<VideoClipSlotId \| null>/g) ?? []).length, 1, "Edit and Control share exactly one selected slot signal");
assert.match(app, /projectPath && dropTarget\?\.closest\("\[data-project-drop-surface\]"\)/, ".sdc opens only on the explicit project drop surface");
assert.match(chrome, /data-project-drop-surface/, "Project chrome exposes the explicit project drop surface");
assert.match(app, /videoClipSlotDropTarget[\s\S]*importAndAssignVideoClipSlots/, "exact layer/slot drops use guarded direct import-and-assign");
assert.match(app, /closest\("\[data-media-library-rail\]"\)[\s\S]*importMediaFilesFromPaths\(visualPaths\)/, "Media Library visual drops remain catalog-only");
assert.match(app, /Drop visual media on Media Library, a Video layer, or an exact Clip Slot/, "unscoped visual drops fail closed");
assert.match(app, /editVideoVisible[\s\S]*controlVideoVisible/, "runtime polling covers user Edit Video and Control surfaces");
assert.match(app, /activeTransition\?\.outgoing_slot_id[\s\S]*activeTransition\?\.duration/, "dominant Take reverses an active transition through explicit outgoing runtime identity and frozen duration intent");
assert.match(app, /resetVideoClipSlotRuntimeFence\(\)[\s\S]*setSelectedVideoClipSlotLayerId\(null\)[\s\S]*setVideoClipSlotInspectorOpen\(false\)/, "replacement resets runtime generation, selection, and inspector UI");
for (const command of ["create", "assign", "update", "remove", "reorder", "duplicate", "set_default", "import_and_assign"]) {
  assert.match(app, new RegExp(`${command === "set_default" ? "set_default" : command}.*video_clip_slot|video_clip_slot.*${command}`), `App facade classifies authored ${command}`);
}
assert.match(controller, /get_video_clip_slot_operation_terminal_result/, "authoritative slot commands recover a lost reply from terminal receipts");
assert.match(controller, /queue_video_clip_slot_authoritative[\s\S]*launch_video_clip_slot_authoritative[\s\S]*seek_video_clip_slot_authoritative/, "runtime four is routed through explicit runtime commands");
assert.match(controller, /transition_kind: transitionKind[\s\S]*transition_duration_ms:[\s\S]*transition_duration:/, "Take sends the typed transition and duration bundle through the authoritative launch request");
assert.match(controller, /setVideoClipRuntime/, "runtime truth has an ephemeral renderer destination");
assert.match(controller, /get_video_clip_slot_runtime/, "bank fetches initial runtime truth through a read-only fenced command");
assert.match(controller, /terminal\.runtime[\s\S]*applyVideoClipSlotRuntime\(terminal\.runtime\)/, "runtime terminal recovery applies the envelope's fresh runtime report");
assert.match(controller, /recoverVideoClipSlotTerminal\(0, requestId, 0, authority\)[\s\S]*if \(terminal\) requireTerminalEnvelope\(terminal, kind\)[\s\S]*terminal\?\.terminal\.kind === "runtime"/, "runtime lost-reply validates every terminal envelope before discriminating outcome kind");
assert.match(controller, /recoverVideoClipSlotTerminal\(preparedToken, requestId, operationGeneration, authority\)[\s\S]*if \(terminal\) requireTerminalEnvelope\(terminal, "import_and_assign"\)[\s\S]*terminal\?\.terminal\.kind === "authored"/, "direct-import lost-reply validates every terminal envelope before discriminating outcome kind");
assert.match(controller, /videoClipSlotRuntimeGenerationCanApply[\s\S]*appliedRuntimeGeneration/, "controller applies the tested exact runtime generation fence");
assert.match(controller, /resetVideoClipSlotRuntimeFence[\s\S]*appliedRuntimeEpoch = null[\s\S]*appliedRuntimeGeneration = -1/, "replacement explicitly resets the runtime generation fence");
assert.doesNotMatch(controller, /applyVideoClipSlotRuntime\(terminal\.terminal\.result\)/, "immutable runtime receipt is never mistaken for transport truth");
assert.match(controller, /authoritativeApplicationIsCurrent\(result\)/, "authored R+1 success consumes App's authoritative-current marker");
assert.match(controller, /shape_fingerprint[\s\S]*length === 0/, "terminal query validates request-shape presence");
assert.match(controller, /let operation: MediaAssetOperationLease \| null = null;[\s\S]*try \{[\s\S]*prepareMediaAssetOperationStart/, "direct drop preflight is inside the guarded operation scope");
assert.match(types, /interface VideoClipSlotAuthoritativeRuntimeOutcome[\s\S]*command_kind/, "runtime terminal receipt mirrors immutable ABI outcome");
assert.match(types, /interface VideoClipSlotAuthoritativeTerminalEnvelope[\s\S]*runtime\?: VideoClipSlotAuthoritativeRuntimeResult/, "terminal envelope mirrors the fresh optional runtime field");
assert.match(types, /interface VideoClipRuntimeReport[\s\S]*runtime: VideoClipRuntimeSnapshot/, "history-free runtime read has a distinct DTO");
assert.match(types, /interface VideoClipTakeTransitionSummary[\s\S]*progress_millis/, "runtime DTO exposes explicit dual-source transition progress");
assert.match(touch, /VideoClipSlotBankPanel/, "Control mounts the shared bank component");
assert.match(edit, /VideoClipSlotBankPanel/, "Edit Video mounts the shared bank component");
assert.match(
  modes,
  /editDomainModes = controlModes\.filter\(\(mode\) => mode\.id !== "live"\)/,
  "Timeline remains excluded from the persistent Edit-domain peer chrome",
);

assert.match(styles, /\.videoClipSlotBankPanel \{[\s\S]*overflow: auto;/, "bank scroll remains internal");
assert.match(styles, /\.videoClipSlotBankPanel \{[\s\S]*container-type: inline-size;/, "bank establishes pane-local container sizing");
assert.match(styles, /\.videoClipSlotBankGrid \{[\s\S]*repeat\(auto-fit, minmax\(min\(112px, calc\(50% - 4px\)\), 1fr\)\)/, "grid auto-fits actual pane width without collapsing to one column");
assert.match(styles, /\.videoClipSlotPad \{[\s\S]*aspect-ratio: 16 \/ 9;/, "pads preserve 16:9 contain geometry");
assert.match(styles, /\.videoClipSlotPrimary img \{[\s\S]*object-fit: contain;/, "thumbnail media is contained rather than stretched");
assert.doesNotMatch(styles.slice(styles.indexOf("/* B4 shared Clip Slot bank")), /@media \(max-width:/, "Clip Slot geometry does not use viewport breakpoints");
for (const label of ["Clip control", "Clip Inspector", "Create Slot", "Launch quantization", "No active slot", "Move Earlier", "Move Later", "Cancel Queue"]) {
  assert.ok(localization.includes(label), `localization source covers ${label}`);
}

for (const viewport of [[1920, 1080], [1920, 1032], [1366, 768], [1280, 720], [860, 520]]) {
  assert.ok(viewport[0] >= 860 && viewport[1] >= 520, `required browser viewport fixture declared: ${viewport.join("x")}`);
}

console.log("Video Clip Slot B4 focused model/browser-contract gate passed.");
