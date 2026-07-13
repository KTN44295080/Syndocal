import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const source = await readFile(new URL("../src/liveAudioInputPresentation.ts", import.meta.url), "utf8");
const syncSource = await readFile(new URL("../src/liveAudioInputStatusSync.ts", import.meta.url), "utf8");
const app = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
const backend = await readFile(new URL("../src-tauri/src/main.rs", import.meta.url), "utf8");
const clipGrid = await readFile(
  new URL("../src/components/VideoClipGridPanel.tsx", import.meta.url),
  "utf8",
);
const styles = await readFile(new URL("../src/styles.css", import.meta.url), "utf8");

const transpiled = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
  fileName: "liveAudioInputPresentation.ts",
});
const presentation = await import(
  `data:text/javascript;base64,${Buffer.from(transpiled.outputText).toString("base64")}`
);
const transpiledSync = ts.transpileModule(syncSource, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
  fileName: "liveAudioInputStatusSync.ts",
});
const statusSync = await import(
  `data:text/javascript;base64,${Buffer.from(transpiledSync.outputText).toString("base64")}`
);

const status = (overrides = {}) => ({
  running: false,
  stale: false,
  safety_clear_pending: false,
  device_name: null,
  sample_rate: 0,
  channels: 0,
  bass: 0,
  mid: 0,
  high: 0,
  analyzed_windows: 0,
  dropped_chunks: 0,
  last_error: null,
  ...overrides,
});

assert.equal(presentation.liveAudioInputHealth(status()), "stopped");
assert.equal(presentation.liveAudioInputHealth(status(), false), "unknown");
assert.equal(
  presentation.liveAudioInputDetail(status()),
  "Live bands can drive Node Graph Audio sources.",
);
assert.equal(
  presentation.liveAudioInputDetail(
    status({ running: true, sample_rate: 48_000, channels: 2, analyzed_windows: 42 }),
  ),
  "48000Hz / 2ch / 42 FFT",
);
const stale = status({ running: true, stale: true, last_error: "Input timed out." });
assert.equal(presentation.liveAudioInputHealth(stale), "stale");
assert.match(presentation.liveAudioInputDetail(stale), /^SAFETY CLEAR ACCEPTED/);
assert.match(presentation.liveAudioInputDetail(stale), /Stop then Start/);
assert.equal(presentation.liveAudioNodeAvailability(stale), "Live stale · clear accepted");
const clearing = status({
  running: true,
  stale: true,
  safety_clear_pending: true,
  last_error: "Engine command queue is full.",
});
assert.equal(presentation.liveAudioInputHealth(clearing), "clearing");
assert.match(presentation.liveAudioInputDetail(clearing), /^SAFETY CLEAR PENDING/);
assert.doesNotMatch(presentation.liveAudioInputDetail(clearing), /SAFETY ZERO/);
assert.doesNotMatch(presentation.liveAudioNodeAvailability(clearing), /output zero/);
const stoppedClearing = status({ safety_clear_pending: true });
assert.equal(presentation.liveAudioInputHealth(stoppedClearing), "clearing");
assert.match(presentation.liveAudioInputDetail(stoppedClearing), /Start stays locked/);
assert.match(
  presentation.liveAudioInputAnnouncement(status({ running: true }), false),
  /Stop remains available/,
);
assert.equal(
  presentation.liveAudioInputAnnouncement(status({ running: true, analyzed_windows: 1 })),
  presentation.liveAudioInputAnnouncement(status({ running: true, analyzed_windows: 999 })),
  "healthy meter polling must not change the live-region announcement",
);

const requestGate = statusSync.createLiveAudioInputStatusRequestGate();
const oldPoll = requestGate.beginPoll();
assert.equal(oldPoll, 0);
assert.equal(requestGate.beginPoll(), null, "polling must remain single-flight");
const stopCommand = requestGate.beginCommand();
assert.equal(stopCommand, 1);
assert.equal(requestGate.accepts(oldPoll), false, "a pre-Stop poll must not overwrite Stop status");
assert.equal(requestGate.accepts(stopCommand), true);
requestGate.endPoll();
assert.equal(requestGate.beginPoll(), null, "polling must stay blocked while Stop is in flight");
requestGate.endCommand(stopCommand);
assert.equal(requestGate.beginPoll(), 1, "polling must resume after the older request settles");

assert.ok(app.includes("stale: false"), "the frontend status default must be healthy/stopped");
assert.ok(app.includes("safety_clear_pending: false"));
assert.ok(app.includes("liveAudioNodeAvailability(liveAudioInputStatus(), liveAudioInputStatusKnown())"));
assert.ok(app.includes("const requestEpoch = liveAudioStatusRequests.beginPoll()"));
assert.ok(app.includes("void refreshLiveAudioInputStatus();"), "status discovery must not depend on the stale frontend default");
assert.ok(
  app.includes("nextStatus.safety_clear_pending || nextStatus.running"),
  "Stop feedback must not claim completion while the backend retains the runtime",
);
assert.ok(clipGrid.includes("data-health={liveAudioHealth()}"));
assert.ok(clipGrid.includes('class="liveAudioHealthAnnouncement" aria-live="polite"'));
assert.ok(!clipGrid.includes('<small aria-live="polite">'));
assert.ok(styles.includes(".liveAudioInputBar.stale"));
assert.ok(styles.includes(".liveAudioInputBar.clearing"));
assert.ok(!styles.includes(".videoClipGridPanel.empty > .liveAudioInputBar"));
assert.ok(
  !/\.liveAudioInputBar small\s*\{\s*display:\s*none/.test(styles),
  "compact layouts must keep the non-color safety message visible",
);
assert.ok(backend.includes("safety_clear_pending: bool"));
assert.ok(backend.includes("status.safety_clear_pending = true"));
assert.ok(backend.includes("status.safety_clear_pending = false"));
assert.ok(backend.includes("live_audio_input_lifecycle: Mutex<()>"));
assert.ok(backend.includes("if pending_status.safety_clear_pending {\n        return Ok(pending_status);"));
assert.ok(backend.includes("Live audio input is already active or stopping"));

console.log("live audio fail-closed lifecycle, presentation, and request ordering ok");
