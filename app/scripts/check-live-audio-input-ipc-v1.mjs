import assert from "node:assert/strict";

const ipc = await import("../src/liveAudioInputIpcV1.ts");

const {
  buildLiveAudioInputBackendArgsV1,
  buildLiveAudioInputCapabilitiesArgsV1,
  buildLiveAudioInputStartArgsV1,
  mapLiveAudioChannelMixToWireV1,
} = ipc;

const assertRejected = (callback, label) => {
  assert.throws(callback, /Live audio IPC v1/, label);
};

const assertExactKeys = (value, keys, label) => {
  assert.deepEqual(Object.keys(value), keys, `${label} keys must be exact and ordered`);
  assert.equal(Object.keys(value).length, keys.length, `${label} must not duplicate keys`);
};

const noSnakeCaseWireKeys = (value, label) => {
  const serialized = JSON.stringify(value);
  assert.doesNotMatch(serialized, /(?:channel_mix|channel_index|left_channel_index|right_channel_index|schema_version|device_id|sample_rate|stream_channels|sample_format|buffer_frames|safety_clear_pending|configured_buffer_frames|applied_buffer_frames)/, `${label} must not emit internal snake_case keys`);
};

const backendArgs = buildLiveAudioInputBackendArgsV1("asio");
assertExactKeys(backendArgs, ["request"], "backend args");
assertExactKeys(backendArgs.request, ["schemaVersion", "backend"], "backend request");
assert.deepEqual(backendArgs, { request: { schemaVersion: 1, backend: "asio" } });
noSnakeCaseWireKeys(backendArgs, "backend args");
assert.deepEqual(
  buildLiveAudioInputBackendArgsV1("wasapi_shared"),
  { request: { schemaVersion: 1, backend: "wasapiShared" } },
  "WASAPI's internal snake_case id must map to Rust's camelCase enum spelling",
);

const capabilitiesArgs = buildLiveAudioInputCapabilitiesArgsV1({
  backend: "asio",
  deviceId: null,
  sampleRate: null,
});
assertExactKeys(capabilitiesArgs, ["request"], "capabilities args");
assertExactKeys(capabilitiesArgs.request, ["schemaVersion", "backend", "deviceId", "sampleRate"], "capabilities request");
assert.deepEqual(capabilitiesArgs, {
  request: { schemaVersion: 1, backend: "asio", deviceId: null, sampleRate: null },
});
noSnakeCaseWireKeys(capabilitiesArgs, "capabilities args");

const averageStart = buildLiveAudioInputStartArgsV1({
  backend: "wasapi_shared",
  device_id: null,
  sample_rate: null,
  stream_channels: null,
  sample_format: null,
  buffer_frames: null,
  channel_mix: { mode: "average_all" },
});
assertExactKeys(averageStart, ["request"], "average start args");
assertExactKeys(averageStart.request, ["schemaVersion", "backend", "deviceId", "sampleRate", "streamChannels", "sampleFormat", "bufferFrames", "channelMix"], "start request");
assertExactKeys(averageStart.request.channelMix, ["mode"], "average channel mix");
assert.deepEqual(averageStart, {
  request: {
    schemaVersion: 1,
    backend: "wasapiShared",
    deviceId: null,
    sampleRate: null,
    streamChannels: null,
    sampleFormat: null,
    bufferFrames: null,
    channelMix: { mode: "averageAll" },
  },
});
noSnakeCaseWireKeys(averageStart, "average start args");

const singleStart = buildLiveAudioInputStartArgsV1({
  backend: "asio",
  device_id: "asio:driver-1",
  sample_rate: 48_000,
  stream_channels: 2,
  sample_format: "i32",
  buffer_frames: 128,
  channel_mix: { mode: "single", channel_index: 1 },
});
assert.deepEqual(singleStart.request.channelMix, { mode: "single", channelIndex: 1 });
assertExactKeys(singleStart.request.channelMix, ["mode", "channelIndex"], "single channel mix");
noSnakeCaseWireKeys(singleStart, "single start args");

const stereoStart = buildLiveAudioInputStartArgsV1({
  backend: "asio",
  device_id: "asio:driver-1",
  sample_rate: 48_000,
  stream_channels: 2,
  sample_format: "f32",
  buffer_frames: 128,
  channel_mix: { mode: "stereo_pair", left_channel_index: 0, right_channel_index: 1 },
});
assert.deepEqual(stereoStart.request.channelMix, {
  mode: "stereoPair",
  leftChannelIndex: 0,
  rightChannelIndex: 1,
});
assertExactKeys(stereoStart.request.channelMix, ["mode", "leftChannelIndex", "rightChannelIndex"], "stereo pair channel mix");
noSnakeCaseWireKeys(stereoStart, "stereo start args");

assert.deepEqual(mapLiveAudioChannelMixToWireV1({ mode: "average_all" }, null), { mode: "averageAll" });
assertRejected(() => mapLiveAudioChannelMixToWireV1({ mode: "average_all", channel_index: 0 }, 2), "average-all unknown fields must reject");
assertRejected(() => mapLiveAudioChannelMixToWireV1({ mode: "single", channel_index: 0 }, null), "single mix without a channel count must reject");
assertRejected(() => mapLiveAudioChannelMixToWireV1({ mode: "single", channel_index: 2 }, 2), "single out-of-range index must reject");
assertRejected(() => mapLiveAudioChannelMixToWireV1({ mode: "stereo_pair", left_channel_index: 0, right_channel_index: 0 }, 2), "stereo duplicate index must reject");
assertRejected(() => mapLiveAudioChannelMixToWireV1({ mode: "averageAll" }, 2), "wire mode must not be accepted as internal input");

assertRejected(() => buildLiveAudioInputBackendArgsV1("legacy"), "unknown backend must reject");
assertRejected(() => buildLiveAudioInputCapabilitiesArgsV1({ backend: "asio", deviceId: null }), "missing capabilities sampleRate must reject");
assertRejected(() => buildLiveAudioInputCapabilitiesArgsV1({ backend: "asio", device_id: null, deviceId: null, sampleRate: null }), "capabilities snake_case alias must reject");
assertRejected(() => buildLiveAudioInputCapabilitiesArgsV1({ backend: "asio", deviceId: "", sampleRate: null }), "empty device id must reject");
for (const value of [NaN, Infinity, 7_999, 768_001, 48_000.5]) {
  assertRejected(() => buildLiveAudioInputCapabilitiesArgsV1({ backend: "asio", deviceId: null, sampleRate: value }), `invalid sample rate ${value} must reject`);
}
const completeStartInput = {
  backend: "asio",
  device_id: "asio:driver-1",
  sample_rate: 48_000,
  stream_channels: 2,
  sample_format: "i32",
  buffer_frames: 128,
  channel_mix: { mode: "average_all" },
};
for (const field of Object.keys(completeStartInput)) {
  const omitted = { ...completeStartInput };
  delete omitted[field];
  assertRejected(() => buildLiveAudioInputStartArgsV1(omitted), `omitted start field ${field} must reject`);
}
assertRejected(() => buildLiveAudioInputStartArgsV1({
  backend: "asio",
  device_id: "asio:driver-1",
  sample_rate: 48_000,
  stream_channels: 2,
  sample_format: "i32",
  buffer_frames: 128,
  channel_mix: { mode: "single", channelIndex: 0 },
}), "start must reject camelCase internal mix aliases");
assertRejected(() => buildLiveAudioInputStartArgsV1({
  backend: "asio",
  device_id: "asio:driver-1",
  sample_rate: 48_000,
  stream_channels: 2,
  sample_format: "u16",
  buffer_frames: 128,
  channel_mix: { mode: "average_all" },
}), "unsupported ASIO sample format must reject");
assertRejected(() => buildLiveAudioInputStartArgsV1({
  backend: "asio",
  device_id: "asio:driver-1",
  sample_rate: 48_000,
  stream_channels: 2,
  sample_format: "i32",
  buffer_frames: 8_193,
  channel_mix: { mode: "average_all" },
}), "oversized buffer must reject");
assertRejected(() => buildLiveAudioInputStartArgsV1({
  backend: "asio",
  device_id: "asio:driver-1",
  sample_rate: 8_000,
  stream_channels: 2,
  sample_format: "i32",
  buffer_frames: 1_601,
  channel_mix: { mode: "average_all" },
}), "buffer age above 200 ms must reject");
assertRejected(() => buildLiveAudioInputStartArgsV1({
  backend: "asio",
  device_id: "asio:driver-1",
  sample_rate: 48_000,
  stream_channels: 0,
  sample_format: "i32",
  buffer_frames: 128,
  channel_mix: { mode: "average_all" },
}), "zero stream channels must reject");
assertRejected(() => buildLiveAudioInputStartArgsV1({
  backend: "asio",
  device_id: "asio:driver-1",
  sample_rate: 48_000,
  stream_channels: 2,
  sample_format: "i32",
  buffer_frames: 128,
  channel_mix: { mode: "average_all" },
  unexpected: true,
}), "unknown start field must reject");

console.log("live audio IPC v1 exact request mapping and fail-closed checks ok");
