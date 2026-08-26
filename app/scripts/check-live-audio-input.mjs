import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const source = await readFile(new URL("../src/liveAudioInputPresentation.ts", import.meta.url), "utf8");
const syncSource = await readFile(new URL("../src/liveAudioInputStatusSync.ts", import.meta.url), "utf8");
const selectionStorageSource = await readFile(
  new URL("../src/liveAudioInputSelectionStorage.ts", import.meta.url),
  "utf8",
);
const app = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
const ipcV1Source = await readFile(new URL("../src/liveAudioInputIpcV1.ts", import.meta.url), "utf8");
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
const transpiledSelectionStorage = ts.transpileModule(selectionStorageSource, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
  fileName: "liveAudioInputSelectionStorage.ts",
});
const selectionStorage = await import(
  `data:text/javascript;base64,${Buffer.from(transpiledSelectionStorage.outputText).toString("base64")}`
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
  availability: "ready",
  availability_detail: null,
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
assert.equal(
  backendState({
    backend: {
      ...asioBackend,
      built: false,
      availability: "not_packaged",
      availability_detail: "This application build excludes the ASIO bridge.",
    },
  }),
  "not_packaged",
);
assert.equal(
  backendState({
    backend: {
      ...asioBackend,
      availability: "not_packaged",
      availability_detail: "This application build excludes the ASIO bridge.",
    },
  }),
  "not_packaged",
  "the exact probe classification outranks the redundant built flag",
);
assert.equal(
  backendState({
    backend: {
      ...asioBackend,
      availability: "fault",
      availability_detail: "ASIO driver enumeration fault.",
    },
  }),
  "fault",
);
assert.equal(
  backendState({
    backend: {
      ...asioBackend,
      availability: "unsupported",
      availability_detail: null,
      requires_explicit_device: false,
    },
  }),
  "unsupported",
);
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
assert.equal(presentation.liveAudioInputBackendStateLabel("not_packaged"), "NOT PACKAGED");
assert.equal(presentation.liveAudioInputBackendStateLabel("unsupported"), "UNSUPPORTED");
assert.equal(presentation.liveAudioInputBackendStateLabel("contract_invalid"), "CONTRACT INVALID");
assert.equal(presentation.liveAudioInputBackendStateLabel("fault"), "FAULT");
assert.equal(presentation.liveAudioInputBackendStateLabel("ready"), "READY");
assert.equal(presentation.liveAudioInputBackendAvailabilityLabel("ready"), "READY");
assert.equal(presentation.liveAudioInputBackendAvailabilityLabel("not_packaged"), "NOT PACKAGED");
assert.equal(presentation.liveAudioInputBackendAvailabilityLabel("fault"), "FAULT");
assert.equal(presentation.liveAudioInputBackendAvailabilityLabel("unsupported"), "UNSUPPORTED");

const missingAvailability = structuredClone(asioBackend);
delete missingAvailability.availability;
assert.equal(
  backendState({ backend: missingAvailability }),
  "contract_invalid",
  "missing availability must fail closed instead of mapping to not-built/ready",
);
assert.equal(
  backendState({ backend: { ...asioBackend, availability: null } }),
  "contract_invalid",
  "null availability must fail closed instead of mapping to not-built/ready",
);
assert.equal(
  backendState({ backend: { ...asioBackend, availability: "NotBuilt" } }),
  "contract_invalid",
  "unknown availability values must fail closed instead of mapping to not-built/ready",
);
assert.equal(
  backendState({ backend: { ...asioBackend, built: false } }),
  "contract_invalid",
  "ready availability contradicted by an unbuilt flag must fail closed",
);

// --- FC-11 exact availability contract parser: positives then fail-closed negatives ---
const wasapiReady = {
  id: "wasapi_shared",
  label: "WASAPI Shared",
  built: true,
  requires_explicit_device: false,
  distribution: "Windows shared-mode capture",
  availability: "ready",
  availability_detail: null,
};
const summaryKeys = [
  "availability",
  "availability_detail",
  "built",
  "distribution",
  "id",
  "label",
  "requires_explicit_device",
];
for (const availability of ["ready", "not_packaged", "fault", "unsupported"]) {
  const parsed = presentation.parseLiveAudioInputBackendSummary({
    ...wasapiReady,
    availability,
    availability_detail: `probe detail ${availability}`,
  });
  assert.equal(parsed.ok, true, `${availability} summaries must parse`);
  assert.deepEqual(
    Object.keys(parsed.summary).sort(),
    summaryKeys,
    "parsed summaries must expose exactly the contracted fields",
  );
  assert.equal(parsed.summary.availability, availability);
  assert.equal(parsed.summary.availability_detail, `probe detail ${availability}`);
}
{
  const parsed = presentation.parseLiveAudioInputBackendSummary(wasapiReady);
  assert.equal(parsed.ok, true);
  assert.equal(parsed.summary.availability_detail, null, "a null probe detail is contract-valid");
}
assert.equal(presentation.parseLiveAudioInputBackendSummaries([wasapiReady, asioBackend]).ok, true);
assert.equal(
  presentation.parseLiveAudioInputBackendSummary(42).reason_code,
  "SUMMARY_NOT_OBJECT",
);
assert.equal(
  presentation.parseLiveAudioInputBackendSummary(null).reason_code,
  "SUMMARY_NOT_OBJECT",
);
assert.equal(
  presentation.parseLiveAudioInputBackendSummaries("asio").reason_code,
  "SUMMARY_LIST_INVALID",
);
assert.equal(presentation.parseLiveAudioInputBackendSummaries("asio").index, -1);
const mixedList = presentation.parseLiveAudioInputBackendSummaries([
  structuredClone(asioBackend),
  { ...structuredClone(asioBackend), availability: "NotBuilt" },
]);
assert.equal(mixedList.ok, false, "one invalid summary must fail the entire catalogue");
assert.equal(mixedList.index, 1);
assert.equal(mixedList.reason_code, "AVAILABILITY_UNKNOWN");
const assertInvalidSummary = (mutate, reason, label) => {
  const raw = structuredClone(asioBackend);
  mutate(raw);
  const parsed = presentation.parseLiveAudioInputBackendSummary(raw);
  assert.equal(parsed.ok, false, `${label} must fail closed`);
  assert.equal(parsed.reason_code, reason, label);
  const listParsed = presentation.parseLiveAudioInputBackendSummaries([raw]);
  assert.equal(listParsed.ok, false, label);
  assert.equal(listParsed.index, 0, label);
  assert.equal(listParsed.reason_code, reason, label);
};
assertInvalidSummary((raw) => { delete raw.availability; }, "AVAILABILITY_MISSING", "missing availability");
assertInvalidSummary((raw) => { raw.availability = undefined; }, "AVAILABILITY_MISSING", "undefined availability");
assertInvalidSummary((raw) => { raw.availability = null; }, "AVAILABILITY_MISSING", "null availability");
assertInvalidSummary((raw) => { raw.availability = "Ready"; }, "AVAILABILITY_UNKNOWN", "case-mismatched availability");
assertInvalidSummary((raw) => { raw.availability = "not_built"; }, "AVAILABILITY_UNKNOWN", "retired collapsed availability value");
assertInvalidSummary((raw) => { raw.availability = ""; }, "AVAILABILITY_UNKNOWN", "blank availability string");
assertInvalidSummary((raw) => { raw.id = "coreaudio"; }, "BACKEND_ID_UNKNOWN", "unknown backend id");
assertInvalidSummary((raw) => { raw.label = ""; }, "LABEL_BLANK", "blank label");
assertInvalidSummary((raw) => { raw.built = "true"; }, "BUILT_INVALID", "string built flag");
assertInvalidSummary((raw) => { delete raw.requires_explicit_device; }, "REQUIRES_EXPLICIT_DEVICE_INVALID", "missing device requirement flag");
assertInvalidSummary((raw) => { raw.distribution = "   "; }, "DISTRIBUTION_BLANK", "blank distribution");
assertInvalidSummary((raw) => { raw.availability_detail = ""; }, "AVAILABILITY_DETAIL_BLANK", "empty probe detail");
assertInvalidSummary((raw) => { raw.availability_detail = "   "; }, "AVAILABILITY_DETAIL_BLANK", "blank probe detail");
assertInvalidSummary((raw) => { raw.availability_detail = 7; }, "AVAILABILITY_DETAIL_INVALID", "non-string probe detail");
assertInvalidSummary((raw) => { raw.future_field = true; }, "SUMMARY_UNEXPECTED_FIELD", "unexpected summary field");
assertInvalidSummary((raw) => { delete raw.id; }, "BACKEND_ID_UNKNOWN", "missing backend id");
assertInvalidSummary((raw) => { delete raw.label; }, "LABEL_BLANK", "missing label");
assertInvalidSummary((raw) => { delete raw.built; }, "BUILT_INVALID", "missing built flag");
assertInvalidSummary((raw) => { delete raw.distribution; }, "DISTRIBUTION_BLANK", "missing distribution");
assertInvalidSummary((raw) => { delete raw.availability_detail; }, "AVAILABILITY_DETAIL_INVALID", "missing probe detail");
{
  const listParsed = presentation.parseLiveAudioInputBackendSummaries([
    structuredClone(asioBackend),
    { ...structuredClone(asioBackend), future_field: 1 },
  ]);
  assert.equal(listParsed.ok, false, "an unexpected field must fail the entire catalogue");
  assert.equal(listParsed.index, 1);
  assert.equal(listParsed.reason_code, "SUMMARY_UNEXPECTED_FIELD");
}
{
  const parsed = presentation.parseLiveAudioInputBackendSummary({ ...structuredClone(asioBackend), built: false });
  assert.equal(parsed.ok, false, "ready availability with built:false must be rejected outright");
  assert.equal(parsed.reason_code, "AVAILABILITY_READY_CONTRADICTS_BUILT");
  const listParsed = presentation.parseLiveAudioInputBackendSummaries([
    structuredClone(wasapiReady),
    { ...structuredClone(asioBackend), built: false },
  ]);
  assert.equal(listParsed.ok, false);
  assert.equal(listParsed.index, 1);
  assert.equal(listParsed.reason_code, "AVAILABILITY_READY_CONTRADICTS_BUILT");
}
{
  const duplicateList = presentation.parseLiveAudioInputBackendSummaries([
    structuredClone(wasapiReady),
    structuredClone(wasapiReady),
  ]);
  assert.equal(duplicateList.ok, false, "duplicate backend ids must fail the entire catalogue");
  assert.equal(duplicateList.index, 1);
  assert.equal(duplicateList.reason_code, "BACKEND_ID_DUPLICATE");
  assert.equal("summaries" in duplicateList, false, "a duplicate-id catalogue must not be partially accepted");
  const lateDuplicateList = presentation.parseLiveAudioInputBackendSummaries([
    structuredClone(asioBackend),
    structuredClone(wasapiReady),
    { ...structuredClone(wasapiReady), label: "WASAPI Shared Again" },
  ]);
  assert.equal(lateDuplicateList.ok, false);
  assert.equal(lateDuplicateList.index, 2);
  assert.equal(lateDuplicateList.reason_code, "BACKEND_ID_DUPLICATE");
}
{
  assert.equal(presentation.liveAudioInputBackendCanDispatch(structuredClone(asioBackend)), true);
  assert.equal(presentation.liveAudioInputBackendCanDispatch(structuredClone(wasapiReady)), true);
  assert.equal(presentation.liveAudioInputBackendCanDispatch({ ...structuredClone(asioBackend), built: false }), false);
  assert.equal(
    presentation.liveAudioInputBackendCanDispatch({ ...structuredClone(asioBackend), availability: "fault" }),
    false,
  );
  assert.equal(
    presentation.liveAudioInputBackendCanDispatch({ ...structuredClone(asioBackend), availability: null }),
    false,
  );
  assert.equal(presentation.liveAudioInputBackendCanDispatch(null), false);
  assert.equal(presentation.liveAudioInputBackendCanDispatch("asio"), false);
}
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

const persistedAsioIntent = {
  backend: "asio",
  device: {
    id: "asio:41:7",
    name: "Syndocal ASIO Test Driver",
    label: "Syndocal ASIO Test Driver",
    backend: "ASIO",
  },
  sample_rate: 48_000,
  sample_format: "f32",
  stream_channels: 2,
  buffer_frames: 128,
  channel_mix: { mode: "stereo_pair", left_channel_index: 0, right_channel_index: 1 },
};
const persistedAsio = selectionStorage.serializeLiveAudioInputSelection(persistedAsioIntent);
assert.equal(persistedAsio.ok, true, "a fully explicit ASIO intent must persist");
assert.equal(persistedAsio.selection.schema_version, 1);
assert.doesNotMatch(persistedAsio.serialized, /asio:41:7|token|path|project/i);
const parsedAsio = selectionStorage.parseLiveAudioInputSelectionStorage(persistedAsio.serialized);
assert.equal(parsedAsio.ok, true, "serialized ASIO intent must round-trip exactly");
assert.deepEqual(parsedAsio.selection, persistedAsio.selection);
const restoredAsio = selectionStorage.restoreLiveAudioInputSelection(persistedAsio.serialized);
assert.equal(restoredAsio.state, "stale", "restart must lock persisted input before revalidation");
assert.equal(restoredAsio.start_locked, true);
assert.equal(restoredAsio.storage_action, "preserve");
assert.equal(restoredAsio.reason_code, "REVALIDATION_REQUIRED");

const currentAsioDevice = {
  ...persistedAsioIntent.device,
  id: "asio:42:3",
};
const exactAsioCapabilities = {
  device_id: currentAsioDevice.id,
  device_name: currentAsioDevice.name,
  backend: "ASIO",
  default_config: { channels: 2, sample_rate: 48_000, sample_format: "f32" },
  supported_configs: [{
    channels: 2,
    min_sample_rate: 48_000,
    max_sample_rate: 48_000,
    sample_format: "f32",
    buffer_size: { kind: "range", min_frames: 64, max_frames: 256 },
  }],
  resolved_config: {
    channels: 2,
    sample_rate: 48_000,
    sample_format: "f32",
    buffer_size: { kind: "range", min_frames: 64, max_frames: 256 },
  },
  max_capture_frames: 8_192,
};
const revalidatedAsio = selectionStorage.revalidateLiveAudioInputSelection(restoredAsio, [{
  device: currentAsioDevice,
  capabilities: exactAsioCapabilities,
}]);
assert.equal(revalidatedAsio.state, "ready", "one exact current catalogue match may unlock Start");
assert.equal(revalidatedAsio.start_locked, false);
assert.equal(revalidatedAsio.current_device_id, "asio:42:3", "only fresh ids may be returned");
assert.equal(revalidatedAsio.start_request.device_id, "asio:42:3");
assert.equal(revalidatedAsio.start_request.backend, "asio");
assert.equal(revalidatedAsio.start_request.buffer_frames, 128);

const malformedCatalogue = (mutate) => {
  const entry = structuredClone({ device: currentAsioDevice, capabilities: exactAsioCapabilities });
  mutate(entry);
  return [entry];
};
const assertMalformedCatalogue = (catalogue, label) => {
  const result = selectionStorage.revalidateLiveAudioInputSelection(restoredAsio, catalogue);
  assert.equal(result.state, "stale", label);
  assert.equal(result.start_locked, true, label);
  assert.equal(result.storage_action, "preserve", label);
  assert.equal(result.reason_code, "CATALOGUE_INVALID", label);
  assert.equal("start_request" in result, false, label);
};
assertMalformedCatalogue([{ device: null, capabilities: null }], "null device must not unlock Start");
assertMalformedCatalogue([{ device: "asio-driver", capabilities: null }], "string device must not unlock Start");
assertMalformedCatalogue(
  malformedCatalogue((entry) => { entry.device.id = ""; }),
  "blank catalogue device ids must not unlock Start",
);
assertMalformedCatalogue(
  malformedCatalogue((entry) => { entry.device.id = null; }),
  "null catalogue device ids must not unlock Start",
);
assertMalformedCatalogue(
  malformedCatalogue((entry) => { entry.capabilities.supported_configs[0].buffer_size.min_frames = "64"; }),
  "string buffer bounds must not be coerced",
);
assertMalformedCatalogue(
  malformedCatalogue((entry) => { entry.capabilities.supported_configs[0].buffer_size.max_frames = 256.5; }),
  "noninteger buffer bounds must not unlock Start",
);
assertMalformedCatalogue(
  malformedCatalogue((entry) => { entry.capabilities.resolved_config.buffer_size.kind = "fixed"; }),
  "unsupported buffer range kinds must fail closed",
);
assertMalformedCatalogue(
  malformedCatalogue((entry) => { entry.capabilities.max_capture_frames = "8192"; }),
  "string capture bounds must not be coerced",
);
assertMalformedCatalogue(
  malformedCatalogue((entry) => { entry.capabilities.supported_configs = "not-a-config-list"; }),
  "string configuration lists must not unlock Start",
);

const missingAsio = selectionStorage.revalidateLiveAudioInputSelection(restoredAsio, []);
assert.equal(missingAsio.state, "stale");
assert.equal(missingAsio.reason_code, "DEVICE_MISSING");
assert.deepEqual(missingAsio.selection, persistedAsio.selection);
const ambiguousAsio = selectionStorage.revalidateLiveAudioInputSelection(restoredAsio, [{
  device: currentAsioDevice,
  capabilities: exactAsioCapabilities,
}, {
  device: { ...currentAsioDevice, id: "asio:42:4" },
  capabilities: { ...exactAsioCapabilities, device_id: "asio:42:4" },
}]);
assert.equal(ambiguousAsio.state, "stale");
assert.equal(ambiguousAsio.reason_code, "DEVICE_AMBIGUOUS");
const backendMismatch = selectionStorage.revalidateLiveAudioInputSelection(restoredAsio, [{
  device: { ...currentAsioDevice, id: "wasapi:42:3", backend: "WASAPI" },
  capabilities: { ...exactAsioCapabilities, device_id: "wasapi:42:3", backend: "WASAPI" },
}]);
assert.equal(backendMismatch.state, "stale");
assert.equal(backendMismatch.reason_code, "BACKEND_MISMATCH");
assert.equal(backendMismatch.selection.backend, "asio", "ASIO must never fall back to WASAPI");
assert.equal("start_request" in backendMismatch, false, "locked input must not manufacture a start request");
const capabilityDrift = selectionStorage.revalidateLiveAudioInputSelection(restoredAsio, [{
  device: currentAsioDevice,
  capabilities: {
    ...exactAsioCapabilities,
    resolved_config: { ...exactAsioCapabilities.resolved_config, sample_rate: 44_100 },
  },
}]);
assert.equal(capabilityDrift.state, "stale");
assert.equal(capabilityDrift.reason_code, "CONFIGURATION_DRIFT");
const capabilityBackendMismatch = selectionStorage.revalidateLiveAudioInputSelection(restoredAsio, [{
  device: currentAsioDevice,
  capabilities: { ...exactAsioCapabilities, backend: "WASAPI" },
}]);
assert.equal(capabilityBackendMismatch.state, "stale");
assert.equal(capabilityBackendMismatch.reason_code, "CAPABILITY_BACKEND_MISMATCH");
const unavailableCapabilities = selectionStorage.revalidateLiveAudioInputSelection(restoredAsio, [{
  device: currentAsioDevice,
  capabilities: null,
}]);
assert.equal(unavailableCapabilities.state, "stale");
assert.equal(unavailableCapabilities.reason_code, "CAPABILITIES_UNAVAILABLE");

const persistedWasapiIntent = {
  backend: "wasapi_shared",
  device: {
    id: "wasapi:41:7",
    name: "Syndocal WASAPI Test Device",
    label: "Syndocal WASAPI Test Device",
    backend: "WASAPI",
  },
  sample_rate: 48_000,
  sample_format: "f32",
  stream_channels: 2,
  buffer_frames: 128,
  channel_mix: { mode: "average_all" },
};
const persistedWasapi = selectionStorage.serializeLiveAudioInputSelection(persistedWasapiIntent);
assert.equal(persistedWasapi.ok, true, "a fully explicit WASAPI intent must persist");
const parsedWasapi = selectionStorage.parseLiveAudioInputSelectionStorage(persistedWasapi.serialized);
assert.equal(parsedWasapi.ok, true, "serialized WASAPI intent must round-trip exactly");
assert.deepEqual(parsedWasapi.selection, persistedWasapi.selection);
const restoredWasapi = selectionStorage.restoreLiveAudioInputSelection(persistedWasapi.serialized);
const currentWasapiDevice = { ...persistedWasapiIntent.device, id: "wasapi:42:3" };
const exactWasapiCapabilities = {
  ...exactAsioCapabilities,
  device_id: currentWasapiDevice.id,
  device_name: currentWasapiDevice.name,
  backend: "WASAPI",
};
const revalidatedWasapi = selectionStorage.revalidateLiveAudioInputSelection(restoredWasapi, [{
  device: currentWasapiDevice,
  capabilities: exactWasapiCapabilities,
}]);
assert.equal(revalidatedWasapi.state, "ready", "WASAPI round-trip may unlock only its exact backend");
assert.equal(revalidatedWasapi.start_request.backend, "wasapi_shared");
assert.equal(revalidatedWasapi.start_request.device_id, "wasapi:42:3");
const wasapiNoCrossBackend = selectionStorage.revalidateLiveAudioInputSelection(restoredWasapi, [{
  device: { ...currentWasapiDevice, id: "asio:42:3", backend: "ASIO" },
  capabilities: null,
}]);
assert.equal(wasapiNoCrossBackend.state, "stale");
assert.equal(wasapiNoCrossBackend.reason_code, "BACKEND_MISMATCH");
assert.equal(wasapiNoCrossBackend.selection.backend, "wasapi_shared");

const persistedAsioObject = JSON.parse(persistedAsio.serialized);
const assertStoredFailure = (raw, code) => {
  const before = raw;
  const result = selectionStorage.parseLiveAudioInputSelectionStorage(raw);
  assert.equal(result.ok, false);
  assert.equal(result.reason_code, code);
  assert.equal(raw, before, "invalid machine-local storage must not be silently rewritten");
  const restored = selectionStorage.restoreLiveAudioInputSelection(raw);
  assert.equal(restored.state, "invalid");
  assert.equal(restored.start_locked, true);
  assert.equal(restored.storage_action, "preserve");
  assert.equal(restored.reason_code, code);
};
assertStoredFailure("{", "INVALID_JSON");
assertStoredFailure(JSON.stringify({ ...persistedAsioObject, schema_version: undefined }), "SCHEMA_VERSION_MISSING");
assertStoredFailure(JSON.stringify({ ...persistedAsioObject, schema_version: 2 }), "SCHEMA_VERSION_FUTURE");
assertStoredFailure(JSON.stringify({ ...persistedAsioObject, backend: "legacy" }), "UNKNOWN_BACKEND");
assertStoredFailure(JSON.stringify({ ...persistedAsioObject, sample_rate: Number.NaN }), "INVALID_SAMPLE_RATE");
assertStoredFailure(JSON.stringify({ ...persistedAsioObject, sample_rate: 1_000_000 }), "INVALID_SAMPLE_RATE");
assertStoredFailure(JSON.stringify({ ...persistedAsioObject, sample_format: "pcm24packed" }), "UNKNOWN_SAMPLE_FORMAT");
assertStoredFailure(
  JSON.stringify({ ...persistedAsioObject, channel_mix: { mode: "single", channel_index: "0" } }),
  "INVALID_CHANNEL_MIX",
);
assertStoredFailure(
  JSON.stringify({ ...persistedAsioObject, channel_mix: { mode: "stereo_pair", left_channel_index: null, right_channel_index: 1 } }),
  "INVALID_CHANNEL_MIX",
);
assertStoredFailure(JSON.stringify({ ...persistedAsioObject, project_path: "C:\\show\\project.sdc" }), "UNEXPECTED_FIELD");
assertStoredFailure(
  JSON.stringify({
    ...persistedAsioObject,
    device_identity: { ...persistedAsioObject.device_identity, id: "asio:41:7" },
  }),
  "UNEXPECTED_FIELD",
);
assertStoredFailure(
  persistedAsio.serialized.replace('"backend":"asio"', '"backend":"asio","backend":"asio"'),
  "DUPLICATE_KEY",
);
assert.equal(
  selectionStorage.serializeLiveAudioInputSelection({ ...persistedAsioIntent, buffer_frames: 0 }).reason_code,
  "INVALID_BUFFER_FRAMES",
);
assert.equal(
  selectionStorage.serializeLiveAudioInputSelection({ ...persistedAsioIntent, sample_rate: Number.NaN }).reason_code,
  "INVALID_SAMPLE_RATE",
);

// --- machine-local storage port: exceptions are reported, never claimed as saved ---
const throwingPort = () => ({
  getItem: () => {
    throw new Error("storage denied");
  },
  setItem: () => {
    throw new Error("storage read-only");
  },
});
const portReadFailure = selectionStorage.readLiveAudioInputSelectionStorage(throwingPort);
assert.equal(portReadFailure.ok, false);
assert.match(portReadFailure.error, /storage denied/);
const portWriteFailure = selectionStorage.writeLiveAudioInputSelectionStorage(
  throwingPort,
  persistedAsio.serialized,
);
assert.equal(portWriteFailure.ok, false);
assert.match(portWriteFailure.error, /storage read-only/);

const fixtureKey = selectionStorage.LIVE_AUDIO_INPUT_SELECTION_STORAGE_KEY;
const fixtureStorage = () => {
  const map = new Map();
  let failNextWrite = false;
  return {
    map,
    failNextWrite: () => {
      failNextWrite = true;
    },
    openPort: () => ({
      getItem: (key) => (map.has(key) ? map.get(key) : null),
      setItem: (key, value) => {
        if (failNextWrite) {
          failNextWrite = false;
          throw new Error("quota exceeded");
        }
        map.set(key, value);
      },
    }),
  };
};
const fixture = fixtureStorage();
const fixtureWrite = selectionStorage.writeLiveAudioInputSelectionStorage(
  fixture.openPort,
  persistedAsio.serialized,
);
assert.deepEqual(fixtureWrite, { ok: true });
const storedRaw = fixture.map.get(fixtureKey);
assert.equal(storedRaw, persistedAsio.serialized, "the exact serialized bytes must be machine-local storage");
const storedObject = JSON.parse(storedRaw);
assert.deepEqual(
  Object.keys(storedObject).sort(),
  [
    "backend",
    "buffer_frames",
    "channel_mix",
    "device_identity",
    "sample_format",
    "sample_rate",
    "schema_version",
    "stream_channels",
  ],
  "persisted selections must carry exactly the versioned stable fields",
);
assert.deepEqual(Object.keys(storedObject.device_identity), ["backend", "name", "label"]);
assert.equal(
  storedRaw.includes("asio:41:7"),
  false,
  "generation-scoped device ids must never be stored",
);

// restart on a later generation: fresh read -> stale -> exact revalidation -> newest id
const restartRead = selectionStorage.readLiveAudioInputSelectionStorage(fixture.openPort);
assert.deepEqual(restartRead, { ok: true, raw: storedRaw });
const restartRestored = selectionStorage.restoreLiveAudioInputSelection(restartRead.raw);
assert.equal(restartRestored.state, "stale");
assert.equal(restartRestored.reason_code, "REVALIDATION_REQUIRED");
const thirdGenerationDevice = { ...persistedAsioIntent.device, id: "asio:43:9" };
const thirdGenerationCapabilities = {
  ...exactAsioCapabilities,
  device_id: thirdGenerationDevice.id,
};
const restartRevalidated = selectionStorage.revalidateLiveAudioInputSelection(restartRestored, [
  { device: thirdGenerationDevice, capabilities: thirdGenerationCapabilities },
]);
assert.equal(restartRevalidated.state, "ready");
assert.equal(restartRevalidated.current_device_id, "asio:43:9");
assert.notEqual(restartRevalidated.start_request.device_id, "asio:41:7");

// malformed, future-version, and duplicate-key payloads survive byte-for-byte
const preserveInvalidStorage = (raw, expectedReason, label) => {
  fixture.map.set(fixtureKey, raw);
  const read = selectionStorage.readLiveAudioInputSelectionStorage(fixture.openPort);
  assert.equal(read.ok, true, label);
  const restored = selectionStorage.restoreLiveAudioInputSelection(read.raw);
  assert.equal(restored.state, "invalid", label);
  assert.equal(restored.start_locked, true, label);
  assert.equal(restored.storage_action, "preserve", label);
  assert.equal(restored.reason_code, expectedReason, label);
  assert.equal(fixture.map.get(fixtureKey), raw, `${label} must remain byte-identical`);
};
preserveInvalidStorage('{"schema_version":2,', "INVALID_JSON", "truncated JSON storage");
preserveInvalidStorage(
  JSON.stringify({ ...storedObject, schema_version: 2 }),
  "SCHEMA_VERSION_FUTURE",
  "future-version storage",
);
preserveInvalidStorage(
  persistedAsio.serialized.replace('"backend":"asio"', '"backend":"asio","backend":"asio"'),
  "DUPLICATE_KEY",
  "duplicate-key storage",
);
fixture.map.set(fixtureKey, persistedAsio.serialized);

// failed saves keep the previous payload and report instead of claiming saved
fixture.failNextWrite();
const replacementSerialized = selectionStorage.serializeLiveAudioInputSelection({
  ...persistedAsioIntent,
  buffer_frames: 256,
});
assert.equal(replacementSerialized.ok, true);
const failedReplacement = selectionStorage.writeLiveAudioInputSelectionStorage(
  fixture.openPort,
  replacementSerialized.serialized,
);
assert.equal(failedReplacement.ok, false);
assert.match(failedReplacement.error, /quota exceeded/);
assert.equal(
  fixture.map.get(fixtureKey),
  persistedAsio.serialized,
  "a failed save must leave the previously stored intent untouched",
);

// incomplete operator intents must never serialize over the saved intent
assert.equal(
  selectionStorage.serializeLiveAudioInputSelection({ ...persistedAsioIntent, buffer_frames: null }).ok,
  false,
);
assert.equal(
  selectionStorage.serializeLiveAudioInputSelection({ ...persistedAsioIntent, sample_rate: null }).reason_code,
  "INVALID_SAMPLE_RATE",
);
assert.equal(
  selectionStorage.serializeLiveAudioInputSelection({ ...persistedAsioIntent, stream_channels: null })
    .reason_code,
  "INVALID_STREAM_CHANNELS",
);
assert.equal(
  selectionStorage.serializeLiveAudioInputSelection({
    ...persistedAsioIntent,
    device: { ...persistedAsioIntent.device, label: "" },
  }).reason_code,
  "INVALID_DEVICE_IDENTITY",
);

// every configuration drift axis stays locked and preserves the saved intent
const configurationDriftCase = (mutate, label) => {
  const capabilities = structuredClone(exactAsioCapabilities);
  mutate(capabilities);
  const result = selectionStorage.revalidateLiveAudioInputSelection(restoredAsio, [
    { device: currentAsioDevice, capabilities },
  ]);
  assert.equal(result.state, "stale", label);
  assert.equal(result.reason_code, "CONFIGURATION_DRIFT", label);
  assert.equal(result.storage_action, "preserve", label);
  assert.equal("start_request" in result, false, label);
  assert.deepEqual(result.selection, persistedAsio.selection, label);
};
configurationDriftCase((capabilities) => {
  capabilities.resolved_config.channels = 1;
}, "resolved channel drift must stay locked");
configurationDriftCase((capabilities) => {
  capabilities.resolved_config.sample_format = "i16";
}, "resolved sample format drift must stay locked");
configurationDriftCase((capabilities) => {
  capabilities.supported_configs[0].channels = 8;
}, "unsupported channel configuration must stay locked");
configurationDriftCase((capabilities) => {
  capabilities.supported_configs[0].sample_format = "i16";
}, "unsupported format range must stay locked");
configurationDriftCase((capabilities) => {
  capabilities.supported_configs[0].buffer_size = { kind: "range", min_frames: 512, max_frames: 1_024 };
}, "buffer outside the supported range must stay locked");
configurationDriftCase((capabilities) => {
  capabilities.max_capture_frames = 64;
}, "buffer beyond the capture ceiling must stay locked");

const capabilityIdentityMismatch = selectionStorage.revalidateLiveAudioInputSelection(restoredAsio, [{
  device: currentAsioDevice,
  capabilities: { ...exactAsioCapabilities, device_id: "asio:999:1" },
}]);
assert.equal(capabilityIdentityMismatch.state, "stale");
assert.equal(capabilityIdentityMismatch.reason_code, "CAPABILITY_DEVICE_MISMATCH");
const capabilityNameMismatch = selectionStorage.revalidateLiveAudioInputSelection(restoredAsio, [{
  device: currentAsioDevice,
  capabilities: { ...exactAsioCapabilities, device_name: "Other Driver" },
}]);
assert.equal(capabilityNameMismatch.state, "stale");
assert.equal(capabilityNameMismatch.reason_code, "CAPABILITY_DEVICE_MISMATCH");
assertMalformedCatalogue(
  [
    { device: currentAsioDevice, capabilities: exactAsioCapabilities },
    { device: currentAsioDevice, capabilities: null },
  ],
  "duplicate catalogue ids must not unlock Start even when identities look identical",
);
const labelDrift = selectionStorage.revalidateLiveAudioInputSelection(restoredAsio, [
  { device: { ...currentAsioDevice, label: "Renamed ASIO Driver" }, capabilities: null },
]);
assert.equal(labelDrift.state, "stale");
assert.equal(labelDrift.reason_code, "DEVICE_MISSING");

// --- FC-11/FC-31: the controlled backend select must survive catalogue hydration ---
const railCatalogue = [wasapiReady, structuredClone(asioBackend)];
const backendSelectOptionsFor = (selectedBackend, backends, backendsKnown, savedBackend) =>
  presentation.liveAudioInputBackendSelectOptions({
    selectedBackend,
    backends,
    backendsKnown,
    savedBackend,
  });
const assertUnambiguous = (options, label) =>
  assert.equal(
    new Set(options.map((option) => option.value)).size,
    options.length,
    `${label}: duplicate option values make the visible selection ambiguous`,
  );

const checkingAsio = backendSelectOptionsFor("asio", [], false, "asio");
assert.deepEqual(
  checkingAsio.map((option) => [option.kind, option.value, option.label]),
  [["checking", "asio", "Checking backends"]],
  "delayed hydration must pin a single placeholder option to the restored backend",
);
assert.equal(presentation.liveAudioInputBackendVisibleValue(checkingAsio, "asio"), "asio");
const unknownCatalogueAsio = backendSelectOptionsFor("asio", [], true, "asio");
assert.deepEqual(
  unknownCatalogueAsio.map((option) => option.label),
  ["Backend unavailable"],
);

const hydratedAsio = backendSelectOptionsFor("asio", railCatalogue, true, "asio");
assert.deepEqual(
  hydratedAsio.map((option) => option.value),
  ["wasapi_shared", "asio"],
  "catalogue order must be preserved verbatim",
);
assert.deepEqual(
  hydratedAsio.map((option) => option.label),
  ["WASAPI Shared", "ASIO"],
);
assertUnambiguous(hydratedAsio, "hydrated catalogue");
assert.equal(presentation.liveAudioInputBackendVisibleValue(hydratedAsio, "asio"), "asio");

const missingAsioSaved = backendSelectOptionsFor("asio", [wasapiReady], true, "asio");
assert.deepEqual(
  missingAsioSaved.map((option) => [option.kind, option.value, option.label]),
  [
    ["selection_unavailable", "asio", "ASIO · saved selection missing"],
    ["catalogue", "wasapi_shared", "WASAPI Shared"],
  ],
  "a missing selected backend must stay visibly ASIO/unavailable before any catalogue entry",
);
assertUnambiguous(missingAsioSaved, "missing ASIO with saved lock");
assert.equal(presentation.liveAudioInputBackendVisibleValue(missingAsioSaved, "asio"), "asio");
const missingAsioUnsaved = backendSelectOptionsFor("asio", [wasapiReady], true, null);
assert.deepEqual(
  missingAsioUnsaved.map((option) => [option.kind, option.value, option.label]),
  [
    ["selection_unavailable", "asio", "ASIO · selection unavailable"],
    ["catalogue", "wasapi_shared", "WASAPI Shared"],
  ],
  "missing ASIO without a saved marker must still fail closed instead of first-option WASAPI",
);
assert.doesNotMatch(missingAsioUnsaved[0].label, /ready/i, "the unavailable marker must never look ready");
const missingWasapi = backendSelectOptionsFor("wasapi_shared", [structuredClone(asioBackend)], true, null);
assert.deepEqual(
  missingWasapi.map((option) => [option.kind, option.value, option.label]),
  [
    ["selection_unavailable", "wasapi_shared", "WASAPI · selection unavailable"],
    ["catalogue", "asio", "ASIO"],
  ],
  "the same fail-closed marker must protect the WASAPI selection",
);
assert.equal(presentation.liveAudioInputBackendVisibleValue(missingWasapi, "wasapi_shared"), "wasapi_shared");

const staleMarkerDropped = backendSelectOptionsFor("asio", [structuredClone(asioBackend)], true, "wasapi_shared");
assert.deepEqual(
  staleMarkerDropped.map((option) => [option.kind, option.value]),
  [["catalogue", "asio"]],
  "a stale saved-backend marker must never duplicate a live option's value",
);
assertUnambiguous(staleMarkerDropped, "stale marker against live option");
const duplicatePinned = backendSelectOptionsFor(
  "wasapi_shared",
  [wasapiReady, structuredClone(wasapiReady)],
  true,
  null,
);
assert.deepEqual(
  duplicatePinned.map((option) => [option.kind, option.value]),
  [["catalogue", "wasapi_shared"]],
  "duplicate catalogue ids must pin exactly one option per backend id",
);
assert.match(
  duplicatePinned[0].label,
  /DUPLICATE BACKEND CATALOGUE/,
  "a duplicated catalogue entry must be visibly marked as a duplicate",
);
assert.match(
  duplicatePinned[0].label,
  /CONTRACT INVALID/,
  "a duplicated catalogue entry must be visibly contract-invalid, never silently usable",
);
assert.equal(duplicatePinned[0].duplicate_catalogue_entry, true);
const duplicateSelectedPinned = backendSelectOptionsFor(
  "asio",
  [structuredClone(asioBackend), { ...structuredClone(asioBackend), label: "ASIO Again" }],
  true,
  "asio",
);
assert.deepEqual(
  duplicateSelectedPinned.map((option) => option.label),
  ["ASIO · DUPLICATE BACKEND CATALOGUE · CONTRACT INVALID"],
  "the selected backend itself must stay visibly invalid when its id is duplicated",
);
assertUnambiguous(duplicatePinned, "duplicate catalogue entries");
const tripleDuplicate = backendSelectOptionsFor(
  "wasapi_shared",
  [wasapiReady, structuredClone(wasapiReady), { ...structuredClone(wasapiReady), label: "WASAPI Shared 3" }],
  true,
  null,
);
assert.deepEqual(
  tripleDuplicate.map((option) => [option.kind, option.value]),
  [["catalogue", "wasapi_shared"]],
  "even repeated duplicates must produce exactly one pinned option value",
);
assert.match(tripleDuplicate[0].label, /CONTRACT INVALID/);
assert.equal(presentation.liveAudioInputBackendVisibleValue([{ value: "wasapi_shared" }], "asio"), null,
  "a target without its own option must resolve to fail-closed null, not the first option");

class BackendSelectModel {
  constructor() {
    this.options = [];
    this.selectedIndex = -1;
  }
  get value() {
    const option = this.options[this.selectedIndex];
    return this.selectedIndex >= 0 && option ? option.value : "";
  }
  setOptions(options) {
    const displayed = this.value;
    this.options = [...options];
    if (!displayed || !this.options.some((option) => option.value === displayed)) {
      this.selectedIndex = this.options.length > 0 ? 0 : -1;
    }
  }
  enforce(target) {
    const visible = presentation.liveAudioInputBackendVisibleValue(this.options, target);
    if (visible === null) return false;
    this.selectedIndex = this.options.findIndex((option) => option.value === visible);
    return true;
  }
}

const restoredDom = new BackendSelectModel();
restoredDom.setOptions(backendSelectOptionsFor("asio", [], false, "asio"));
restoredDom.enforce("asio");
assert.equal(restoredDom.value, "asio", "the mount-time placeholder pins the restored ASIO selection");
restoredDom.setOptions(backendSelectOptionsFor("asio", railCatalogue, true, "asio"));
assert.equal(
  restoredDom.value,
  "wasapi_shared",
  "the model must reproduce the browser first-option reset behind the reported restore bug",
);
assert.equal(restoredDom.enforce("asio"), true, "reconciliation must find the ASIO option after hydration");
assert.equal(restoredDom.value, "asio", "post-hydration reconciliation must restore ASIO exactly");

const missingDom = new BackendSelectModel();
missingDom.setOptions(backendSelectOptionsFor("asio", [], false, "asio"));
missingDom.setOptions(backendSelectOptionsFor("asio", [wasapiReady], true, "asio"));
assert.equal(missingDom.enforce("asio"), true);
assert.equal(missingDom.value, "asio", "missing ASIO must stay on its unavailable marker, never WASAPI");

const operatorSwitchDom = new BackendSelectModel();
operatorSwitchDom.setOptions(backendSelectOptionsFor("wasapi_shared", railCatalogue, true, null));
operatorSwitchDom.enforce("wasapi_shared");
assert.equal(operatorSwitchDom.value, "wasapi_shared");
operatorSwitchDom.setOptions(backendSelectOptionsFor("asio", railCatalogue, true, null));
operatorSwitchDom.enforce("asio");
assert.equal(operatorSwitchDom.value, "asio");

// --- FC-12 strict backend-native ASIO selection verdict contract ---
const asioRestoredSelection = {
  state: "restored",
  driver_id: "asio:41:7",
  driver_name: "Syndocal ASIO Test Driver",
  sample_rate_hz: 48_000,
  input_channels: 2,
  sample_format: "f32",
  fixed_buffer_frames: 128,
  reason: null,
  message:
    "Persisted ASIO selection was restored from machine storage and stays locked until Start revalidates it.",
};
const asioRevalidatedSelection = {
  ...asioRestoredSelection,
  state: "revalidated",
  message: "The explicit ASIO selection was revalidated against the current driver catalog.",
};
const asioInvalidNativeSelection = {
  state: "invalid",
  driver_id: null,
  driver_name: null,
  sample_rate_hz: null,
  input_channels: null,
  sample_format: null,
  fixed_buffer_frames: null,
  reason: "ASIO_STORAGE_SCHEMA_FUTURE",
  message: "Persisted ASIO selection bytes were rejected; Start stays locked until an explicit Start succeeds.",
};
const asioSelectionKeys = [
  "driver_id",
  "driver_name",
  "fixed_buffer_frames",
  "input_channels",
  "message",
  "reason",
  "sample_format",
  "sample_rate_hz",
  "state",
];
for (const [fixture, expectedState] of [
  [asioRestoredSelection, "restored"],
  [asioRevalidatedSelection, "revalidated"],
  [asioInvalidNativeSelection, "invalid"],
]) {
  const parsed = presentation.parseLiveAudioInputAsioSelection(structuredClone(fixture));
  assert.equal(parsed.ok, true, `${expectedState} native verdicts must parse`);
  assert.deepEqual(Object.keys(parsed.selection).sort(), asioSelectionKeys);
  assert.equal(parsed.selection.state, expectedState);
}
{
  const parsed = presentation.parseLiveAudioInputAsioSelection(asioRestoredSelection);
  assert.equal(parsed.selection.driver_id, "asio:41:7");
  assert.equal(parsed.selection.sample_rate_hz, 48_000);
  assert.equal(parsed.selection.input_channels, 2);
  assert.equal(parsed.selection.fixed_buffer_frames, 128);
  assert.equal(parsed.selection.reason, null);
  assert.deepEqual(parsed.selection, structuredClone(asioRestoredSelection));
}

const assertMalformedAsioSelection = (mutate, reason, label) => {
  const raw = structuredClone(asioRestoredSelection);
  mutate(raw);
  const parsed = presentation.parseLiveAudioInputAsioSelection(raw);
  assert.equal(parsed.ok, false, `${label} must fail closed`);
  assert.equal(parsed.reason_code, reason, label);
};
assert.equal(presentation.parseLiveAudioInputAsioSelection(42).reason_code, "ASIO_SELECTION_NOT_OBJECT");
assert.equal(presentation.parseLiveAudioInputAsioSelection(null).reason_code, "ASIO_SELECTION_NOT_OBJECT");
assert.equal(presentation.parseLiveAudioInputAsioSelection("restored").reason_code, "ASIO_SELECTION_NOT_OBJECT");
assert.equal(
  presentation.parseLiveAudioInputAsioSelection([structuredClone(asioRestoredSelection)]).reason_code,
  "ASIO_SELECTION_NOT_OBJECT",
);
assertMalformedAsioSelection((raw) => { raw.future_field = 1; }, "ASIO_SELECTION_UNEXPECTED_FIELD", "unexpected native-verdict field");
assertMalformedAsioSelection((raw) => { delete raw.state; }, "ASIO_SELECTION_STATE_INVALID", "missing native-verdict state");
assertMalformedAsioSelection((raw) => { raw.state = "pending"; }, "ASIO_SELECTION_STATE_INVALID", "unknown native-verdict state");
assertMalformedAsioSelection((raw) => { raw.state = null; }, "ASIO_SELECTION_STATE_INVALID", "null native-verdict state");
assertMalformedAsioSelection((raw) => { delete raw.driver_id; }, "ASIO_SELECTION_DRIVER_ID_INVALID", "missing driver id");
assertMalformedAsioSelection((raw) => { raw.driver_id = ""; }, "ASIO_SELECTION_DRIVER_ID_INVALID", "blank driver id");
assertMalformedAsioSelection((raw) => { raw.driver_id = 7; }, "ASIO_SELECTION_DRIVER_ID_INVALID", "non-string driver id");
assertMalformedAsioSelection((raw) => { delete raw.driver_name; }, "ASIO_SELECTION_DRIVER_NAME_INVALID", "missing driver name");
assertMalformedAsioSelection((raw) => { raw.driver_name = ""; }, "ASIO_SELECTION_DRIVER_NAME_INVALID", "blank driver name");
assertMalformedAsioSelection((raw) => { delete raw.sample_rate_hz; }, "ASIO_SELECTION_SAMPLE_RATE_INVALID", "missing sample rate");
assertMalformedAsioSelection((raw) => { raw.sample_rate_hz = 0; }, "ASIO_SELECTION_SAMPLE_RATE_INVALID", "zero sample rate");
assertMalformedAsioSelection((raw) => { raw.sample_rate_hz = -48_000; }, "ASIO_SELECTION_SAMPLE_RATE_INVALID", "negative sample rate");
assertMalformedAsioSelection((raw) => { raw.sample_rate_hz = 48_000.5; }, "ASIO_SELECTION_SAMPLE_RATE_INVALID", "fractional sample rate");
assertMalformedAsioSelection((raw) => { raw.sample_rate_hz = "48000"; }, "ASIO_SELECTION_SAMPLE_RATE_INVALID", "string sample rate");
assertMalformedAsioSelection((raw) => { delete raw.input_channels; }, "ASIO_SELECTION_INPUT_CHANNELS_INVALID", "missing channel count");
assertMalformedAsioSelection((raw) => { raw.input_channels = 0; }, "ASIO_SELECTION_INPUT_CHANNELS_INVALID", "zero channel count");
assertMalformedAsioSelection((raw) => { raw.input_channels = 2.5; }, "ASIO_SELECTION_INPUT_CHANNELS_INVALID", "fractional channel count");
assertMalformedAsioSelection((raw) => { raw.input_channels = Number.NaN; }, "ASIO_SELECTION_INPUT_CHANNELS_INVALID", "NaN channel count");
assertMalformedAsioSelection((raw) => { delete raw.sample_format; }, "ASIO_SELECTION_SAMPLE_FORMAT_INVALID", "missing sample format");
assertMalformedAsioSelection((raw) => { raw.sample_format = "   "; }, "ASIO_SELECTION_SAMPLE_FORMAT_INVALID", "blank sample format");
assertMalformedAsioSelection((raw) => { delete raw.fixed_buffer_frames; }, "ASIO_SELECTION_FIXED_BUFFER_FRAMES_INVALID", "missing buffer frames");
assertMalformedAsioSelection((raw) => { raw.fixed_buffer_frames = 0; }, "ASIO_SELECTION_FIXED_BUFFER_FRAMES_INVALID", "zero buffer frames");
assertMalformedAsioSelection((raw) => { raw.fixed_buffer_frames = 128.5; }, "ASIO_SELECTION_FIXED_BUFFER_FRAMES_INVALID", "fractional buffer frames");
assertMalformedAsioSelection((raw) => { raw.reason = "unexpected while restored"; }, "ASIO_SELECTION_REASON_INVALID", "restored verdict carrying a reason");
assertMalformedAsioSelection((raw) => { delete raw.message; }, "ASIO_SELECTION_MESSAGE_INVALID", "missing verdict message");
assertMalformedAsioSelection((raw) => { raw.message = ""; }, "ASIO_SELECTION_MESSAGE_INVALID", "blank verdict message");
assertMalformedAsioSelection((raw) => { raw.message = 7; }, "ASIO_SELECTION_MESSAGE_INVALID", "non-string verdict message");
{
  const invalidStateFieldReasons = {
    driver_id: "ASIO_SELECTION_DRIVER_ID_INVALID",
    driver_name: "ASIO_SELECTION_DRIVER_NAME_INVALID",
    sample_rate_hz: "ASIO_SELECTION_SAMPLE_RATE_INVALID",
    input_channels: "ASIO_SELECTION_INPUT_CHANNELS_INVALID",
    sample_format: "ASIO_SELECTION_SAMPLE_FORMAT_INVALID",
    fixed_buffer_frames: "ASIO_SELECTION_FIXED_BUFFER_FRAMES_INVALID",
  };
  for (const [field, reason] of Object.entries(invalidStateFieldReasons)) {
    for (const leakedValue of ["leaked-identity", 128]) {
      const raw = { ...asioInvalidNativeSelection, [field]: leakedValue };
      const parsed = presentation.parseLiveAudioInputAsioSelection(raw);
      assert.equal(parsed.ok, false, `an invalid verdict must not carry device/config data (${field})`);
      assert.equal(parsed.reason_code, reason, field);
    }
  }
  const blankReasonInvalid = presentation.parseLiveAudioInputAsioSelection({
    ...asioInvalidNativeSelection,
    reason: "",
  });
  assert.equal(blankReasonInvalid.ok, false);
  assert.equal(blankReasonInvalid.reason_code, "ASIO_SELECTION_REASON_INVALID");
  const missingReasonInvalid = presentation.parseLiveAudioInputAsioSelection({
    ...asioInvalidNativeSelection,
    reason: undefined,
  });
  assert.equal(missingReasonInvalid.ok, false);
  assert.equal(missingReasonInvalid.reason_code, "ASIO_SELECTION_REASON_INVALID");
  const missingMessageInvalid = presentation.parseLiveAudioInputAsioSelection({
    ...asioInvalidNativeSelection,
    message: undefined,
  });
  assert.equal(missingMessageInvalid.ok, false);
  assert.equal(missingMessageInvalid.reason_code, "ASIO_SELECTION_MESSAGE_INVALID");
}

{
  const restoredVerdict = presentation.liveAudioInputAsioSelectionVerdict(structuredClone(asioRestoredSelection));
  assert.equal(restoredVerdict.contract_valid, true);
  assert.equal(restoredVerdict.start_locked, true, "a restored native verdict must keep Start locked");
  assert.equal(restoredVerdict.state, "restored");
  assert.equal(restoredVerdict.reason, null);
  assert.equal(restoredVerdict.message, asioRestoredSelection.message);
  assert.match(restoredVerdict.state_label, /RESTORED/);

  const revalidatedVerdict = presentation.liveAudioInputAsioSelectionVerdict(
    structuredClone(asioRevalidatedSelection),
  );
  assert.equal(revalidatedVerdict.contract_valid, true);
  assert.equal(revalidatedVerdict.start_locked, false, "only a revalidated native verdict may leave Start unlocked");
  assert.equal(revalidatedVerdict.state, "revalidated");

  const invalidVerdict = presentation.liveAudioInputAsioSelectionVerdict(structuredClone(asioInvalidNativeSelection));
  assert.equal(invalidVerdict.contract_valid, true);
  assert.equal(invalidVerdict.start_locked, true, "an invalid native verdict must keep Start locked");
  assert.equal(invalidVerdict.state, "invalid");
  assert.equal(invalidVerdict.reason, "ASIO_STORAGE_SCHEMA_FUTURE");

  const malformedVerdict = presentation.liveAudioInputAsioSelectionVerdict({
    ...structuredClone(asioRestoredSelection),
    future_field: true,
  });
  assert.equal(malformedVerdict.contract_valid, false, "a malformed native verdict must be visibly contract-invalid");
  assert.equal(malformedVerdict.start_locked, true, "a malformed native verdict must lock Start");
  assert.equal(malformedVerdict.state, "contract_invalid");
  assert.match(malformedVerdict.state_label, /CONTRACT INVALID/);
  assert.equal(malformedVerdict.reason, "ASIO_SELECTION_UNEXPECTED_FIELD");
}

assert.ok(
  inputRail.includes("ref={backendSelect}"),
  "the backend select must expose its DOM node for post-render reconciliation",
);
assert.ok(inputRail.includes("createEffect("), "value reconciliation must rerun after every render flush");
assert.ok(
  inputRail.includes("liveAudioInputBackendVisibleValue("),
  "visible-value resolution must be shared with the fail-closed presenter",
);
assert.ok(
  inputRail.includes("<For each={backendSelectOptions()}>"),
  "one deterministic option list must drive the controlled select",
);
assert.ok(
  inputRail.includes("value={props.selectedLiveAudioInputBackend}"),
  "the select stays controlled by the exact selected-backend prop",
);

assert.ok(app.includes("stale: false"), "the frontend status default must be healthy/stopped");
assert.ok(app.includes("safety_clear_pending: false"));
assert.ok(
  app.includes("get graphs() { return snapshot().node_graphs; }"),
  "the compact Audio Reactive status strip must keep reading backend Node Graph state",
);
assert.ok(app.includes("const requestEpoch = liveAudioStatusRequests.beginPoll()"));
assert.ok(app.includes("void refreshLiveAudioInputStatus();"), "status discovery must not depend on the stale frontend default");
assert.ok(app.includes("buildLiveAudioInputCapabilitiesArgsV1({"));
assert.ok(app.includes("deviceId: deviceId || null,"));
assert.ok(app.includes('"live_audio_input_backends"'));
assert.match(app, /list_audio_input_devices[\s\S]*?buildLiveAudioInputBackendArgsV1\(backendId\)/);
assert.match(
  app,
  /start_live_audio_input[\s\S]*?buildLiveAudioInputStartArgsV1\(request\)/,
  "Start must cross the strict V1 mapper instead of sending the internal snake_case request",
);
assert.match(ipcV1Source, /schemaVersion:\s*LIVE_AUDIO_INPUT_IPC_V1_SCHEMA_VERSION/);
assert.doesNotMatch(
  app,
  /"(?:list_audio_input_devices|get_live_audio_input_capabilities)",\s*\{\s*backend:/,
  "live-audio list/capability invokes must not retain the retired flat backend payload",
);
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
assert.ok(
  inputRail.includes("data-live-audio-backend-availability="),
  "the rail must publish the exact availability contract for automation",
);
assert.ok(
  !inputRail.includes("!selectedBackend()?.built"),
  "the collapsed built flag must no longer be the sole Start gate",
);
assert.ok(
  inputRail.includes("!selectedBackendStartable()"),
  "transport Start gating must require an exactly ready backend",
);
assert.ok(
  inputRail.includes("disabled={inputLocked() || !selectedBackendStartable()}"),
  "device selection must stay locked unless the backend is exactly ready",
);
assert.ok(
  inputRail.includes("parseLiveAudioInputBackendSummary("),
  "the rail must validate summaries through the fail-closed contract parser",
);
assert.ok(
  inputRail.includes("liveAudioInputBackendSelectOptions("),
  "backend options must flow through the deterministic fail-closed presenter",
);
assert.ok(
  source.includes("liveAudioInputBackendOptionLabel"),
  "backend options must surface NOT PACKAGED/FAULT/UNSUPPORTED inline",
);
assert.ok(
  inputRail.includes("selectedBackendStartable() ? configFormat() : backendDetail()"),
  "unstartable backends must show their specific failure detail, not only a tooltip",
);
assert.ok(
  !source.includes("not_built"),
  "the collapsed not_built presentation state must be fully retired",
);
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
assert.ok(app.includes("readLiveAudioInputSelectionStorage(() => window.localStorage)"));
const restoreWiringIndex = app.indexOf("readLiveAudioInputSelectionStorage(() => window.localStorage)");
const bootstrapIndex = app.indexOf("void refreshLiveAudioInputBackends(false)");
assert.ok(
  restoreWiringIndex >= 0 && bootstrapIndex > restoreWiringIndex,
  "saved-selection restore must precede backend discovery",
);
assert.equal(
  app.match(/readLiveAudioInputSelectionStorage\(\(\) => window\.localStorage\)/g)?.length,
  1,
  "machine-local live audio selection must be read once per App startup, never on unrelated operator unlock",
);
assert.ok(
  app.indexOf("const restoreSavedLiveAudioInputSelectionAtStartup") <
    app.indexOf("const unlockOperator = async"),
  "startup restore must be outside the operator-unlock flow",
);
assert.ok(
  /const restoreSavedLiveAudioInputSelectionAtStartup[\s\S]*?restoredRuntime\.phase === "stale"[\s\S]*?setSelectedLiveAudioInputBackend\(restoredRuntime\.selection\.backend\)/.test(app),
  "a saved stale selection may pin only its exact backend before catalogue revalidation",
);
assert.ok(
  /const devices = await invoke<LiveAudioInputDeviceSummary\[]>\([\s\S]*?"list_audio_input_devices",[\s\S]*?buildLiveAudioInputBackendArgsV1\(backendId\),[\s\S]*?setLiveAudioInputDevices\(devices\);[\s\S]*?const savedRuntime = liveAudioInputSavedSelection\(\);/.test(app),
  "the exact enumerated device catalogue must publish before saved-selection revalidation; it cannot synthesize a selected option",
);
assert.ok(app.includes("pinnedSavedBackendId"), "backend discovery must honor the saved-selection pin");
assert.ok(
  app.indexOf("pinnedSavedBackendId") <
    app.indexOf('backends.find((backend) => backend.id === "wasapi_shared")'),
  "the WASAPI/first-backend fallback must be guarded by the saved-selection pin",
);
assert.ok(
  app.includes("request = savedRuntime.startRequest;"),
  "Start must send the revalidated request verbatim",
);
assert.ok(
  app.indexOf("request = savedRuntime.startRequest;") < app.indexOf('"start_live_audio_input"'),
  "the revalidated request must be the one sent to start_live_audio_input",
);
assert.ok(app.includes("noteLiveAudioInputOperatorChange()"));
assert.ok(app.includes("maybePersistLiveAudioInputSelection()"));
assert.match(
  app,
  /import\s*\{[\s\S]*?\bliveAudioInputBackendCanDispatch\b[\s\S]*?\}\s*from "\.\/liveAudioInputPresentation";/,
  "App must use the single shared ready+built backend dispatch predicate",
);
assert.doesNotMatch(
  app,
  /const\s+liveAudioInputBackendCanDispatch\s*=/,
  "App must not keep a second local backend dispatch predicate",
);
assert.match(
  app,
  /if \(!serialized\.ok\) \{[\s\S]{0,500}setLiveAudioInputPersistError\(detail\);[\s\S]{0,500}setMessage\(detail\);/,
  "a failed machine-local selection serialization must surface its exact reason instead of returning silently",
);
assert.ok(
  app.includes("applySavedLiveAudioInputSelectionRevalidation(savedRuntime, catalogue, announce, true)"),
);
assert.ok(
  !selectionStorageSource.includes("removeItem") && !selectionStorageSource.includes(".clear()"),
  "selection storage must never delete preserved payloads",
);
assert.ok(inputRail.includes("data-live-audio-saved-state={savedSelection()?.phase"));
assert.ok(inputRail.includes("props.liveAudioInputPersistError"));
assert.ok(inputRail.includes("savedSelectionLockActive()"));
assert.ok(
  inputRail.includes("saved.backend === props.selectedLiveAudioInputBackend"),
  "a stale or invalid saved selection must lock only its named current backend",
);
assert.ok(
  app.includes("const savedRuntimeAppliesToCurrentBackend = ("),
  "App must centralize the saved-selection/current-backend scope boundary",
);
assert.ok(
  app.includes('runtime !== null && "selection" in runtime && runtime.selection.backend === backendId'),
  "only a saved selection with the exact current backend may influence a Start request",
);
assert.ok(
  app.includes("const savedRuntimeApplies = savedRuntimeAppliesToCurrentBackend(savedRuntime, selectedBackendId);"),
  "Start must derive from a saved selection only after exact backend scoping",
);
assert.ok(
  app.includes("if (savedRuntimeAppliesToCurrentBackend(savedRuntime, backendId))"),
  "Refresh revalidation must not apply an ASIO saved intent to another current backend",
);
assert.ok(
  inputRail.includes("savedBackend: savedSelection()?.backend ?? null"),
  "the option presenter must know the saved backend to label its fail-closed marker",
);
assert.ok(
  source.includes("liveAudioInputBackendCanDispatch"),
  "the shared dispatch gate must be exported from the presentation module",
);
assert.ok(
  inputRail.includes("liveAudioInputBackendCanDispatch("),
  "the rail must derive backend startability through the one shared dispatch helper",
);
assert.ok(
  !source.includes("!parsed.summary.built") &&
    !source.includes('availability === "ready" && !'),
  "ready/built dispatch conditions must exist only inside the shared helper and parser",
);
assert.ok(source.includes("parseLiveAudioInputAsioSelection"));
assert.ok(
  inputRail.includes("liveAudioInputAsioSelectionVerdict("),
  "the rail must present the native ASIO verdict only through the shared contract parser",
);
assert.ok(inputRail.includes('data-live-audio-asio-verdict={'));
assert.ok(inputRail.includes('data-live-audio-asio-contract={'));
assert.ok(inputRail.includes('data-live-audio-asio-start-locked={'));
assert.ok(inputRail.includes('data-live-audio-asio-reason={'));
assert.ok(inputRail.includes('data-live-audio-asio-message={'));
assert.ok(
  inputRail.includes('data-live-audio-action="asio-revalidate"') &&
    inputRail.includes("Revalidate current ASIO selection"),
  "an invalid ASIO verdict must expose a clearly labelled explicit revalidation action",
);
assert.ok(
  inputRail.includes("props.liveAudioInputAsioRevalidationEligible") &&
    inputRail.includes("props.liveAudioInputAsioRevalidationArmed"),
  "the rail may unlock invalid ASIO Start only from the App-owned exact arm",
);
// --- ASIO revalidation arm: asserted as control-flow ordering, not substrings ---
const extractBalancedBlock = (source, openBraceAt) => {
  let depth = 0;
  for (let index = openBraceAt; index < source.length; index += 1) {
    if (source[index] === "{") depth += 1;
    else if (source[index] === "}") {
      depth -= 1;
      if (depth === 0) return source.slice(openBraceAt + 1, index);
    }
  }
  throw new Error(`unbalanced block starting at offset ${openBraceAt}`);
};
const appFunctionBody = (signature, label, openAnchor) => {
  const start = app.indexOf(signature);
  assert.ok(start >= 0, `${label}: ${JSON.stringify(signature)} must exist in App.tsx`);
  const openFrom = openAnchor === undefined
    ? start
    : (() => {
      const anchorAt = app.indexOf(openAnchor, start);
      assert.ok(anchorAt > start, `${label}: ${JSON.stringify(openAnchor)} must follow its declaration`);
      // The anchor may end with the opening brace itself, so search from it.
      return anchorAt;
    })();
  const open = app.indexOf("{", openFrom);
  assert.ok(open >= openFrom, `${label}: opening brace must follow its declaration`);
  return { start, body: extractBalancedBlock(app, open) };
};
const codeOnly = (code) => code
  .replace(/\/\*[\s\S]*?\*\//g, " ")
  .replace(/(^|\n)\s*\/\/[^\n]*/g, " ")
  .replace(/\s+/g, "");
const assertOrdered = (body, needles, label) => {
  let cursor = -1;
  for (const needle of needles) {
    const at = body.indexOf(needle, cursor + 1);
    assert.ok(
      at > cursor,
      `${label}: ${JSON.stringify(needle)} must execute after the preceding asserted step, not merely exist`,
    );
    cursor = at;
  }
};
const assertFirstStatementIsArmClear = (body, label) => {
  const clearAt = body.indexOf("setLiveAudioInputAsioRevalidationArm(null);");
  assert.ok(clearAt >= 0, `${label} must clear any pending ASIO revalidation arm`);
  assert.equal(
    codeOnly(body.slice(0, clearAt)),
    "",
    `${label} must clear the pending ASIO revalidation arm as its absolute first statement`,
  );
};

const armHandler = appFunctionBody(
  "const armLiveAudioInputAsioRevalidation = (): void =>",
  "forced Revalidate handler",
);
assertFirstStatementIsArmClear(armHandler.body, "the forced Revalidate handler");
assertOrdered(
  armHandler.body,
  [
    "setLiveAudioInputAsioRevalidationArm(null);",
    "if (!liveAudioInputStatusKnown()) {",
    "ASIO revalidation cannot be armed until a fresh status arrives",
    "return;",
    "if (!liveAudioInputAsioRevalidationEligible()) {",
    "const current = currentLiveAudioInputStartRequest();",
    "const nativeInvalidVerdictFingerprint = liveAudioInputAsioInvalidVerdictFingerprint();",
    "const requestFingerprint = liveAudioInputStartRequestFingerprint(current.request);",
    "setLiveAudioInputAsioRevalidationArm({",
    "setMessage(\"ASIO revalidation is armed for one Start with the exact current driver configuration.\");",
  ],
  "the forced Revalidate handler",
);

const eligibleMemo = appFunctionBody(
  "const liveAudioInputAsioRevalidationEligible = createMemo(() => {",
  "revalidation eligibility",
);
assertOrdered(
  eligibleMemo.body,
  [
    "!liveAudioInputStatusKnown() ||",
    "liveAudioInputBusy() ||",
    "liveAudioInputBackendsBusy() ||",
    "liveAudioInputStatus().running ||",
    'selectedLiveAudioInputBackend() !== "asio" ||',
    "liveAudioInputAsioInvalidVerdictFingerprint() === null",
    "currentLiveAudioInputStartRequest()",
  ],
  "revalidation eligibility",
);

const currentnessFn = appFunctionBody(
  "const liveAudioInputAsioRevalidationArmIsCurrent = (",
  "arm currentness",
);
assertOrdered(
  currentnessFn.body,
  [
    "if (!liveAudioInputStatusKnown()) return false;",
    "const current = currentLiveAudioInputStartRequest();",
    "selectedLiveAudioInputBackend() === \"asio\" &&",
    "currentRequestFingerprint === arm.request_fingerprint &&",
    "liveAudioInputStartRequestFingerprint(arm.request) === arm.request_fingerprint &&",
    "liveAudioInputAsioInvalidVerdictFingerprint() === arm.native_invalid_verdict_fingerprint &&",
    "liveAudioInputSavedSelectionFingerprint() === arm.saved_selection_fingerprint &&",
    "liveAudioInputProjectAuthorityFingerprint() === arm.project_authority_fingerprint;",
  ],
  "arm currentness",
);

const startRoute = appFunctionBody("const startLiveAudioInput = async () =>", "Start route");
const armConsumeReadAt = startRoute.body.indexOf(
  "const asioRevalidationArmAtAttempt = liveAudioInputAsioRevalidationArm();",
);
assert.ok(armConsumeReadAt >= 0, "every Start attempt must read the pending arm exactly once at entry");
assert.equal(
  codeOnly(startRoute.body.slice(0, armConsumeReadAt)),
  "",
  "consuming the revalidation arm must be the absolute entry of every Start attempt, ahead of busy/refresh/validation/return",
);
assertOrdered(
  startRoute.body,
  [
    "const asioRevalidationArmAtAttempt = liveAudioInputAsioRevalidationArm();",
    "setLiveAudioInputAsioRevalidationArm(null);",
    "if (liveAudioInputBusy() || liveAudioInputBackendsBusy()) return;",
    "savedRuntimeAppliesToCurrentBackend(savedRuntime, selectedBackendId)",
    "currentLiveAudioInputStartRequest()",
    "liveAudioInputBackendsKnown() ||",
    "nativeAsioVerdict !== null && nativeAsioVerdict.start_locked",
    '"start_live_audio_input"',
  ],
  "the Start attempt",
);

const stopRoute = appFunctionBody("const stopLiveAudioInput = async () =>", "Stop route");
assertFirstStatementIsArmClear(stopRoute.body, "the Stop route");
assertOrdered(
  stopRoute.body,
  [
    "setLiveAudioInputAsioRevalidationArm(null);",
    "if (liveAudioInputBusy()) return;",
  ],
  "the Stop route",
);

const pollRoute = appFunctionBody(
  "const refreshLiveAudioInputStatus = async () =>",
  "status poll",
);
const pollCatchAt = pollRoute.body.indexOf("} catch (error) {");
assert.ok(pollCatchAt >= 0, "the status poll must own a failure branch");
assertOrdered(
  pollRoute.body.slice(pollCatchAt),
  [
    "if (liveAudioStatusRequests.accepts(requestEpoch)) {",
    "setLiveAudioInputAsioRevalidationArm(null);",
    "invalidateLiveAudioInputLevels();",
    "setLiveAudioInputStatus((current) => ({",
    "clearLiveAudioInputTelemetryFreshness();",
    "setLiveAudioInputStatusKnown(false);",
  ],
  "the accepted status-poll failure branch",
);

for (const [signature, label, openAnchor] of [
  ["const selectLiveAudioInputDevice = (deviceId: string) => {", "device selection", undefined],
  ["const selectLiveAudioInputSampleRate = (sampleRate: number | null) => {", "sample-rate selection", undefined],
  ["const selectLiveAudioInputBufferFrames = (bufferFrames: number | null) => {", "buffer selection", undefined],
  ["const selectLiveAudioInputChannelMix = (channelMix: LiveAudioChannelMix) => {", "channel-mix selection", undefined],
  ["const selectLiveAudioInputBackend = (backendId: LiveAudioInputBackendId) => {", "backend selection", undefined],
  [
    "const applySavedLiveAudioInputSelectionRevalidation = (",
    "saved-selection boundary application",
    "): void => {",
  ],
]) {
  const extracted = appFunctionBody(signature, label, openAnchor);
  assert.ok(
    extracted.body.includes("setLiveAudioInputAsioRevalidationArm(null);"),
    `${label} must clear any pending ASIO revalidation arm within its own body`,
  );
}
for (const [signature, label, publishNeedle] of [
  [
    "const adoptProjectMappingsAuthority = (authority: Partial<ProjectControlMappingsAuthority>) => {",
    "project authority adoption",
    "setProjectMappingsAuthority(next);",
  ],
  [
    "const commitPreparedProjectControlMappings = (prepared: PreparedProjectControlMappings) => {",
    "prepared project authority commit",
    "setProjectMappingsAuthority(prepared.token);",
  ],
]) {
  assertOrdered(
    appFunctionBody(signature, label).body,
    ["setLiveAudioInputAsioRevalidationArm(null);", publishNeedle],
    label,
  );
}
assertOrdered(
  appFunctionBody("commitBundle: (candidate, prepared, options) => {", "authority bundle commit").body,
  [
    "batch(() => {",
    "setLiveAudioInputAsioRevalidationArm(null);",
    "setProjectMappingsAuthority(candidateToken);",
  ],
  "the authority-bundle commit fallback branch",
);

assert.equal(
  app.split("setLiveAudioInputAsioRevalidationArm({").length - 1,
  1,
  "only the explicit Revalidate handler may create an ASIO revalidation arm",
);
assert.ok(
  app.includes("if (arm !== null && !liveAudioInputAsioRevalidationArmed())"),
  "an ASIO arm must also clear reactively on native/storage/project/request/status drift rather than falling back",
);
assert.ok(
  !armHandler.body.includes("isTrusted") && !startRoute.body.includes("isTrusted"),
  "the explicit ASIO revalidation and Start paths must not trust browser event provenance",
);
{
  const startLockedAt = inputRail.indexOf("const backendStartLocked = () =>");
  const visualBandAt = inputRail.indexOf("const visualBandPercent");
  assert.ok(startLockedAt >= 0 && visualBandAt > startLockedAt);
  const gateSource = inputRail.slice(startLockedAt, visualBandAt);
  assert.ok(
    gateSource.includes("asioVerdictStartLocked()"),
    "the Start gate must consult the native ASIO verdict lock",
  );
  assert.ok(
    inputRail.includes('props.selectedLiveAudioInputBackend !== "asio"'),
    "a native ASIO verdict must not lock a current WASAPI Start",
  );
  assert.ok(
    inputRail.includes('verdict.state !== "restored"'),
    "restored must remain visible as a non-autoresume native verdict while a fresh explicit ASIO Start reaches revalidation",
  );
  assert.ok(
    inputRail.includes("`${asioVerdict()?.reason} · ${asioVerdict()?.message}`"),
    "invalid native ASIO verdicts must expose both their exact reason and actionable native message",
  );
}
assert.ok(
  inputRail.includes("data-live-audio-saved-state={savedSelection()?.phase"),
  "the native verdict display must not replace the machine-local saved-selection indicator",
);
assert.ok(
  source.includes("saved selection missing"),
  "the saved-selection lock must keep a visible fail-closed option label",
);
assert.ok(backend.includes("safety_clear_pending: bool"));
assert.ok(backend.includes("status.safety_clear_pending = true"));
assert.ok(backend.includes("status.safety_clear_pending = false"));
assert.ok(backend.includes("live_audio_input_lifecycle: Mutex<()>"));
assert.ok(backend.includes("fn resolve_live_audio_stream_config("));
assert.ok(backend.includes("resolved_config: LiveAudioInputResolvedConfig"));
assert.ok(backend.includes("sample_format: Option<String>"));
{
  const clearPendingGate = backend.indexOf("if pending_status.safety_clear_pending {");
  const inputStateRead = backend.indexOf("let input = state", clearPendingGate);
  assert.ok(clearPendingGate >= 0 && inputStateRead > clearPendingGate,
    "the safety-clear-pending gate must precede the live audio input state read");
  assert.match(
    backend.slice(clearPendingGate, inputStateRead),
    /return [^\n]*pending_status/,
    "the safety-clear-pending short-circuit must return the cloned pending status",
  );
}
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

console.log(
  "live audio fail-closed lifecycle, availability contract, selection persistence, presentation, and request ordering ok",
);
