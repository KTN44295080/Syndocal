import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import ts from "typescript";

const appRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const read = (relativePath) => readFile(path.join(appRoot, relativePath), "utf8");

const helperSource = await read("src/videoCameraProfiles.ts");
const helperOutput = ts.transpileModule(helperSource, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
  fileName: "videoCameraProfiles.ts",
  reportDiagnostics: true,
});
const diagnostics = helperOutput.diagnostics ?? [];
assert.equal(
  diagnostics.filter((diagnostic) => diagnostic.category === ts.DiagnosticCategory.Error).length,
  0,
  diagnostics.map((diagnostic) => ts.flattenDiagnosticMessageText(diagnostic.messageText, "\n")).join("\n"),
);
const camera = await import(
  `data:text/javascript;base64,${Buffer.from(helperOutput.outputText, "utf8").toString("base64")}`,
);

const profile = (overrides = {}) => ({
  device_identity: "device:camera-a",
  device_name: "Camera A",
  profile_identity: "profile:dci4k30",
  width: 4096,
  height: 2160,
  frame_rate_numerator: 30,
  frame_rate_denominator: 1,
  frame_rate_label: "30fps",
  pixel_format: "yuv420p",
  codec: "mjpeg",
  endpoint_name: "syndocal-camera-v1:dW5pcXVlLWVuZHBvaW50",
  ...overrides,
});

const dci4k = profile();
const uhd120 = profile({
  profile_identity: "profile:uhd120",
  width: 1280,
  height: 720,
  frame_rate_numerator: 120,
  frame_rate_label: "120fps",
  endpoint_name: "syndocal-camera-v1:dW5pcXVlLXVuaXF1ZQ",
});
assert.deepEqual(camera.parseVideoCameraProfiles([dci4k, uhd120]), [dci4k, uhd120]);
assert.equal(camera.cameraProfileDisplayLabel(dci4k), "4096x2160 · 30fps · yuv420p · mjpeg");
assert.match(camera.cameraProfileDisplayLabel(uhd120), /1280x720 · 120fps/);
assert.deepEqual(camera.cameraDeviceOptions([dci4k, uhd120]), [{
  device_identity: "device:camera-a",
  device_name: "Camera A",
  profile_count: 2,
}]);

// A catalog is backend-authoritative: malformed entries, missing required
// fields, and duplicate opaque endpoints are all rejected instead of being
// replaced by a guessed/default camera.
assert.throws(() => camera.parseVideoCameraProfiles({ profiles: [dci4k] }), /not an array/);
assert.throws(() => camera.parseVideoCameraProfiles([{ ...dci4k, codec: undefined }]), /entry 1 is invalid/);
assert.throws(
  () => camera.parseVideoCameraProfiles([dci4k, { ...uhd120, endpoint_name: dci4k.endpoint_name }]),
  /duplicate endpoint/,
);

const probe = {
  success: true,
  endpoint_name: dci4k.endpoint_name,
  actual_width: 4096,
  actual_height: 2160,
  frame_rate_numerator: 30,
  frame_rate_denominator: 1,
  frame_rate_label: "30fps",
  pixel_format: "yuv420p",
  codec: "mjpeg",
};
assert.deepEqual(camera.parseVideoCameraProbe(probe, dci4k.endpoint_name), probe);
assert.throws(() => camera.parseVideoCameraProbe({ ...probe, endpoint_name: "other" }, dci4k.endpoint_name), /different endpoint/);
assert.throws(() => camera.parseVideoCameraProbe({ ...probe, success: false }, dci4k.endpoint_name), /did not succeed/);

const successfulState = {
  status: "success",
  endpoint_name: dci4k.endpoint_name,
  profile_identity: dci4k.profile_identity,
  result: probe,
  message: "ok",
};
assert.equal(camera.cameraProbeAllowsAdd("Camera", dci4k.endpoint_name, dci4k, successfulState), true);
assert.equal(camera.cameraProbeAllowsAdd("Camera", "different", dci4k, successfulState), false);
assert.equal(camera.cameraProbeAllowsAdd("Camera", dci4k.endpoint_name, uhd120, successfulState), false);
const staleState = camera.invalidatedCameraProbeState("Camera profile changed; test again.", dci4k);
assert.equal(staleState.status, "stale", "profile/device/catalog changes explicitly invalidate the probe");
assert.equal(camera.cameraProbeAllowsAdd("Camera", dci4k.endpoint_name, dci4k, staleState), false);
assert.equal(camera.cameraProbeAllowsAdd("File", "", undefined, staleState), true, "non-camera paths remain unchanged");

const panel = await read("src/components/VideoSourceCreatePanel.tsx");
const picker = await read("src/components/VideoCameraProfilePicker.tsx");
const nativeCatalog = await read("src-tauri/src/capture_catalog.rs");
const control = await read("src/components/VideoControlPanel.tsx");
const invokeTuple = await read("src/tauriInvokeCommands.ts");
const localization = await read("src/uiLocalization.ts");
const manifest = JSON.parse(await read("src/tauri-invoke-manifest.json"));

assert.match(panel, /VideoCameraProfilePicker/);
assert.match(
  nativeCatalog,
  /const MAX_HIGH_RESOLUTION_FRAME_RATE: FrameRate = FrameRate \{[\s\S]*?numerator: 60,/,
  "4K/high-resolution camera profiles must admit up to 60fps",
);
assert.match(
  nativeCatalog,
  /width: 4096,[\s\S]*?height: 2160,[\s\S]*?frame_rate_numerator: 60,[\s\S]*?\.is_ok\(\)\);/,
  "native camera bounds must keep a deterministic 4K60 acceptance proof",
);
assert.match(
  nativeCatalog,
  /width: 4096,[\s\S]*?height: 2160,[\s\S]*?frame_rate_numerator: 120,[\s\S]*?\.is_err\(\)\);/,
  "native camera bounds must fail closed for 4K120",
);
assert.match(panel, /disabled=\{props\.sourceKind === "Camera" && !cameraAddAllowed\(\)\}/);
assert.match(picker, /Refresh cameras/);
assert.match(picker, /data-video-camera-device/);
assert.match(picker, /data-video-camera-profile/);
assert.match(picker, /Test selected profile/);
assert.match(picker, /list_video_camera_profiles/);
assert.match(picker, /probe_video_camera_profile/);
assert.match(picker, /\{ endpointName \}/);
assert.match(picker, /props\.onSetPath\(profile\.endpoint_name\)/);
assert.match(picker, /cameraProbeAllowsAdd/);
assert.match(picker, /Camera catalog refresh invalidated the previous probe/);
assert.match(picker, /Camera device changed; test the selected profile/);
assert.match(picker, /Camera profile changed; test the selected profile/);
assert.match(panel, /Screen capture uses a separate truthful 1280x720 \/ 30fps path/);
assert.doesNotMatch(picker, /Camera device name \(Windows\)/, "camera must not expose the old free-text fallback");
assert.doesNotMatch(picker, /value=\{props\.path\}/, "camera surface must not expose a free-text path input");
assert.match(picker, /Select a camera device/);
assert.match(picker, /Select a camera profile/);
assert.match(
  picker,
  /Camera profile refresh failed: \{error\(\)\}/,
  "camera refresh errors must keep the localized prefix and dynamic error detail",
);
assert.match(control, /invokeCommand=\{props\.invokeCommand \?\? props\.sourceCreate\.invokeCommand\}/);

const tupleCommands = [...invokeTuple.matchAll(/^\s+"([a-z0-9_]+)",$/gm)].map((match) => match[1]);
assert.deepEqual(tupleCommands, [...tupleCommands].sort(), "typed invoke list remains sorted");
assert.deepEqual(manifest, [...manifest].sort(), "invoke manifest remains sorted");
for (const command of ["list_video_camera_profiles", "probe_video_camera_profile"]) {
  assert.ok(tupleCommands.includes(command), `typed invoke list admits ${command}`);
  assert.ok(manifest.includes(command), `invoke manifest admits ${command}`);
}

for (const text of [
  "Refreshing cameras…",
  "Refresh cameras",
  "Testing selected camera profile…",
  "Testing selected profile…",
  "Test selected profile",
  "Camera profile discovery is unavailable until the native IPC bridge is connected.",
  "Camera profile testing is unavailable until the native IPC bridge is connected.",
  "Camera profile probe passed for the current endpoint.",
  "Camera catalog refresh invalidated the previous probe; select and test a profile.",
  "Camera device changed; test the selected profile before adding a layer.",
  "Camera profile changed; test the selected profile before adding a layer.",
  "Select a camera device and profile before testing.",
  "Capture rate is above 60fps; output presentation is capped at 60fps.",
]) {
  assert.ok(localization.includes("\"" + text + "\""), "Japanese localization must cover " + text);
}
const escapeRegex = (value) => value.replace(/[\\^$.*+?()[\]{}|]/g, "\\$&");
const assertExactJapaneseMapping = (english, japanese) => {
  const mapping = new RegExp(
    escapeRegex(JSON.stringify(english)) + "\\s*:\\s*" + escapeRegex(JSON.stringify(japanese)),
  );
  assert.match(localization, mapping, "Japanese localization must map " + english + " exactly");
};
for (const [english, japanese] of [
  ["Refreshing cameras…", "カメラを更新中…"],
  ["Refresh cameras", "カメラを更新"],
  ["Testing selected camera profile…", "選択したカメラプロファイルをテスト中…"],
  ["Testing selected profile…", "選択したプロファイルをテスト中…"],
  ["Test selected profile", "選択したプロファイルをテスト"],
  [
    "Camera profile discovery is unavailable until the native IPC bridge is connected.",
    "ネイティブIPCブリッジに接続するまでカメラプロファイル検出は利用できません。",
  ],
  [
    "Camera profile testing is unavailable until the native IPC bridge is connected.",
    "ネイティブIPCブリッジに接続するまでカメラプロファイルのテストは利用できません。",
  ],
  [
    "Camera profile probe passed for the current endpoint.",
    "現在のエンドポイントでカメラプロファイルのプローブに成功しました。",
  ],
  [
    "Camera catalog refresh invalidated the previous probe; select and test a profile.",
    "カメラカタログの更新により前回のプローブは無効になりました。プロファイルを選択してテストしてください。",
  ],
  [
    "Camera device changed; test the selected profile before adding a layer.",
    "カメラデバイスが変更されました。レイヤーを追加する前に選択したプロファイルをテストしてください。",
  ],
  [
    "Camera profile changed; test the selected profile before adding a layer.",
    "カメラプロファイルが変更されました。レイヤーを追加する前に選択したプロファイルをテストしてください。",
  ],
  [
    "Select a camera device and profile before testing.",
    "テストする前にカメラデバイスとプロファイルを選択してください。",
  ],
  [
    "Capture rate is above 60fps; output presentation is capped at 60fps.",
    "キャプチャレートが60fpsを超えています。出力表示は60fpsに制限されます。",
  ],
  ["Camera profile refresh failed:", "カメラプロファイルの更新に失敗:"],
]) {
  assertExactJapaneseMapping(english, japanese);
}

console.log("video camera profile UI contract passed");
