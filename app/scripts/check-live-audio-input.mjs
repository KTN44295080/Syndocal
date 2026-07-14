import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const source = await readFile(new URL("../src/liveAudioInputPresentation.ts", import.meta.url), "utf8");
const syncSource = await readFile(new URL("../src/liveAudioInputStatusSync.ts", import.meta.url), "utf8");
const app = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
const backend = await readFile(new URL("../src-tauri/src/main.rs", import.meta.url), "utf8");
const engine = await readFile(new URL("../../crates/engine/src/lib.rs", import.meta.url), "utf8");
const clipGrid = await readFile(
  new URL("../src/components/VideoClipGridPanel.tsx", import.meta.url),
  "utf8",
);
const inputRail = await readFile(
  new URL("../src/components/LiveAudioInputRail.tsx", import.meta.url),
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
  device_id: null,
  device_name: null,
  backend: null,
  sample_format: null,
  sample_rate: 0,
  channels: 0,
  configured_buffer_frames: null,
  channel_mix: { mode: "average_all" },
  bass: 0,
  mid: 0,
  high: 0,
  analyzed_windows: 0,
  dropped_chunks: 0,
  dropped_frames: 0,
  backend_xruns: 0,
  callback_count: 0,
  last_callback_frames: 0,
  min_callback_frames: 0,
  max_callback_frames: 0,
  capture_to_worker_us: 0,
  max_capture_to_worker_us: 0,
  queue_depth: 0,
  queue_capacity: 0,
  queue_depth_high_water: 0,
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
    status({
      running: true,
      backend: "WASAPI",
      sample_format: "f32",
      sample_rate: 48_000,
      channels: 2,
      analyzed_windows: 42,
    }),
  ),
  "OVR 0/0f · XRUN 0 · WASAPI shared · f32 48.0kHz · 2→M · REQ BUF default · APPLIED pending · CB 0/0/0f · FFT 42 · C→W EST 0.0/0.0ms · Q 0/0/0",
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

const asioBackend = {
  id: "asio",
  label: "ASIO",
  built: true,
  requires_explicit_device: true,
  distribution: "GPL-3.0-or-proprietary feature build",
};
const backendState = (overrides = {}) => presentation.liveAudioInputBackendState({
  backend: asioBackend,
  backendKnown: true,
  backendBusy: false,
  backendError: null,
  devicesAvailable: 1,
  selectedDeviceId: "asio-driver-1",
  sampleRate: 48_000,
  bufferFrames: 128,
  capabilities: {},
  status: status(),
  statusKnown: true,
  ...overrides,
});
assert.equal(backendState({ backend: { ...asioBackend, built: false } }), "not_built");
assert.equal(backendState({ devicesAvailable: 0 }), "empty");
assert.equal(backendState({ selectedDeviceId: "" }), "select_device");
assert.equal(backendState({ sampleRate: null }), "configure");
assert.equal(backendState({ bufferFrames: null }), "configure");
assert.equal(backendState(), "ready");
assert.equal(backendState({ status: status({ running: true, backend: "ASIO" }) }), "open");
assert.equal(
  backendState({ status: status({ running: true, backend: "ASIO", callback_count: 1 }) }),
  "active",
);
assert.equal(
  backendState({ status: status({ running: true, stale: true, backend: "ASIO" }) }),
  "fault",
);
assert.equal(presentation.liveAudioInputBackendStateLabel("not_built"), "NOT BUILT");
assert.match(
  presentation.liveAudioInputDetail(status({
    running: true,
    backend: "ASIO",
    sample_format: "f32",
    sample_rate: 48_000,
    channels: 2,
    configured_buffer_frames: 128,
    applied_buffer_frames: 64,
  })),
  /BUF 64f · REQ 128f/,
);
assert.doesNotMatch(
  presentation.liveAudioInputDetail(status({
    running: true,
    backend: "ASIO",
    sample_format: "f32",
    sample_rate: 48_000,
    channels: 2,
  })),
  /ASIO shared/,
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
assert.ok(app.includes("{ backend: backendId, deviceId: deviceId || null, sampleRate }"));
assert.ok(app.includes('"live_audio_input_backends"'));
assert.ok(app.includes('{\n          backend: backendId,\n        }'));
assert.ok(app.includes('backendId === "wasapi_shared"'));
assert.ok(app.includes('backend: selectedLiveAudioInputBackend()'));
assert.ok(app.includes('liveAudioInputSampleRate() === null || liveAudioInputBufferFrames() === null'));
assert.ok(app.includes("const capabilitiesReady = await refreshLiveAudioInputCapabilities("));
assert.ok(
  app.includes("capabilitiesReady && announce"),
  "device discovery success must not overwrite a capability/config resolution error",
);
assert.ok(app.includes("stream_channels: resolvedConfig.channels"));
assert.ok(app.includes("sample_format: resolvedConfig.sample_format"));
assert.ok(
  app.includes("nextStatus.safety_clear_pending || nextStatus.running"),
  "Stop feedback must not claim completion while the backend retains the runtime",
);
assert.ok(inputRail.includes("data-health={health()}"));
assert.ok(inputRail.includes('class="liveAudioHealthAnnouncement"'));
assert.ok(inputRail.includes('role="status"'));
assert.ok(inputRail.includes('aria-live="polite"'));
assert.ok(inputRail.includes('role="meter"'));
assert.ok(inputRail.includes('aria-valuenow={percent()}'));
assert.ok(inputRail.includes("capabilities.resolved_config"));
assert.ok(inputRail.includes("resolved.buffer_size"));
assert.ok(inputRail.includes("rates.add(config.min_sample_rate)"));
assert.ok(inputRail.includes("rates.add(config.max_sample_rate)"));
assert.ok(inputRail.includes("!props.liveAudioInputCapabilities"));
assert.ok(inputRail.includes('data-live-audio-control="backend"'));
assert.ok(inputRail.includes('data-live-audio-control="device"'));
assert.ok(inputRail.includes('data-live-audio-backend-state={backendState()}'));
assert.ok(inputRail.includes('fallback={<option value="">Select ASIO driver</option>}'));
assert.ok(inputRail.includes('fallback={<option value="">Select fixed buffer</option>}'));
assert.ok(!inputRail.includes('<small aria-live="polite">'));
assert.ok(clipGrid.includes("<Show when={!props.compact}>") && clipGrid.includes("<LiveAudioInputRail"));
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
assert.ok(backend.includes("fn resolve_live_audio_stream_config("));
assert.ok(backend.includes("resolved_config: LiveAudioInputResolvedConfig"));
assert.ok(backend.includes("sample_format: Option<String>"));
assert.ok(backend.includes("if pending_status.safety_clear_pending {\n        return Ok(pending_status);"));
assert.ok(backend.includes("Live audio input is already active or stopping"));
assert.ok(backend.includes("crossbeam_queue::ArrayQueue"));
assert.ok(backend.includes("std::panic::catch_unwind"));
assert.ok(backend.includes("worker.is_finished()"));
assert.ok(backend.includes("worker_heartbeat_is_stale()"));
assert.ok(backend.includes("callback_frames.saturating_sub(capture_capacity_frames)"));
assert.ok(backend.includes("capture_to_worker < LIVE_AUDIO_STALE_AFTER"));
assert.ok(backend.includes("if converted.is_finite()"));
assert.ok(engine.includes("LIVE_AUDIO_SPECTRUM_TTL: Duration = Duration::from_millis(250)"));
assert.ok(engine.includes("self.expire_live_audio_spectrum(now)"));
const callbackStart = backend.indexOf("fn queue_live_audio_samples");
const callbackEnd = backend.indexOf("fn sync_live_audio_capture_telemetry", callbackStart);
assert.ok(callbackStart >= 0 && callbackEnd > callbackStart);
const callbackSource = backend.slice(callbackStart, callbackEnd);
assert.doesNotMatch(callbackSource, /Vec|collect\s*\(|status\.lock|Mutex/);
assert.match(callbackSource, /free_capture_slots\.pop/);
assert.match(callbackSource, /ready_capture_chunks\.push/);
assert.doesNotMatch(callbackSource, /Box::new|LiveAudioSampleChunk::new/);

console.log("live audio fail-closed lifecycle, presentation, and request ordering ok");
