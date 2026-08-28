import assert from "node:assert/strict";
import { existsSync, readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { inspectBridgeExports } from "./check-show-asio-artifact.mjs";

const scriptDir = dirname(fileURLToPath(import.meta.url));
export const workspaceRoot = resolve(scriptDir, "../..");

// v2 remains an exact compatibility surface; v3 must be its own exact nine.
export const asioV2Exports = Object.freeze([
  "syndocal_asio_v2_abi_version",
  "syndocal_asio_v2_build_flags",
  "syndocal_asio_v2_drivers_json",
  "syndocal_asio_v2_capabilities_json",
  "syndocal_asio_v2_string_free",
  "syndocal_asio_v2_start",
  "syndocal_asio_v2_stop",
  "syndocal_asio_v2_close",
  "syndocal_asio_v2_telemetry_json",
]);
export const asioV3Exports = Object.freeze([
  "syndocal_asio_v3_abi_version",
  "syndocal_asio_v3_build_flags",
  "syndocal_asio_v3_drivers_json",
  "syndocal_asio_v3_capabilities_json",
  "syndocal_asio_v3_string_free",
  "syndocal_asio_v3_start",
  "syndocal_asio_v3_stop",
  "syndocal_asio_v3_close",
  "syndocal_asio_v3_telemetry_json",
]);
export const exactBridgeExports = Object.freeze([...asioV2Exports, ...asioV3Exports].sort());

function assertExactList(actual, expected, label) {
  assert.ok(Array.isArray(actual), label + " must be an array");
  assert.equal(new Set(actual).size, actual.length, label + " must not contain duplicate symbols");
  assert.deepEqual([...actual].sort(), [...expected].sort(), label + " must be exact");
}

export function assertExactAsioBridgeExports(exportsList) {
  const owned = exportsList.filter((name) => name.startsWith("syndocal_asio_"));
  assertExactList(owned, exactBridgeExports, "ASIO v2/v3 bridge export set");
  return owned.sort();
}

function normalizedC(text) {
  return String(text).replace(/\s+/gu, " ").trim();
}

function enumValues(headerText, enumName) {
  const match = new RegExp("enum\\s+" + enumName + "\\s*\\{([\\s\\S]*?)\\};", "u").exec(headerText);
  assert.ok(match, "header is missing enum " + enumName);
  return Object.fromEntries(
    [...match[1].matchAll(/\b(SYNDOCAL_ASIO_V3_[A-Z_]+)\s*=\s*(\d+)/gu)].map((entry) => [entry[1], Number(entry[2])]),
  );
}

function assertExactEnum(headerText, enumName, expected) {
  assert.deepEqual(enumValues(headerText, enumName), expected, "header " + enumName + " values must be exact");
}

const requiredV3LoaderSignatures = Object.freeze([
  ["OutputCallbackV3", 'pub(crate) type OutputCallbackV3 = unsafe extern "C" fn(*mut c_void, *mut f32, u32, u32, u64, u64, u64) -> u32;'],
  ["DuplexCallbackV3", 'pub(crate) type DuplexCallbackV3 = unsafe extern "C" fn(*mut c_void, *const f32, u32, *mut f32, u32, u32, u64, u64, u64) -> u32;'],
  ["EventCallbackV3", 'pub(crate) type EventCallbackV3 = unsafe extern "C" fn(*mut c_void, u32, u32, *const u8, usize);'],
  ["AbiVersionFn", 'type AbiVersionFn = unsafe extern "C" fn() -> u32;'],
  ["BuildFlagsFn", 'type BuildFlagsFn = unsafe extern "C" fn() -> u32;'],
  ["DriversJsonFn", 'type DriversJsonFn = unsafe extern "C" fn(*mut BridgeStringV3, *mut BridgeStringV3) -> u32;'],
  ["CapabilitiesJsonFn", 'type CapabilitiesJsonFn = unsafe extern "C" fn(*const u8, usize, *mut BridgeStringV3, *mut BridgeStringV3) -> u32;'],
  ["StringFreeFn", 'type StringFreeFn = unsafe extern "C" fn(BridgeStringV3);'],
  ["StartFn", 'type StartFn = unsafe extern "C" fn( *const u8, usize, Option<OutputCallbackV3>, Option<DuplexCallbackV3>, Option<EventCallbackV3>, *mut c_void, *mut *mut c_void, *mut BridgeStringV3, *mut BridgeStringV3, ) -> u32;'],
  ["StopFn", 'type StopFn = unsafe extern "C" fn(*mut c_void, *mut BridgeStringV3, *mut BridgeStringV3) -> u32;'],
  ["CloseFn", 'type CloseFn = unsafe extern "C" fn(*mut *mut c_void, *mut BridgeStringV3, *mut BridgeStringV3) -> u32;'],
  ["TelemetryJsonFn", 'type TelemetryJsonFn = unsafe extern "C" fn(*const c_void, *mut BridgeStringV3, *mut BridgeStringV3) -> u32;'],
]);

const loaderSignatureMutations = Object.freeze([
  ["OutputCallbackV3", "*mut f32, u32, u32, u64, u64, u64) -> u32;", "*mut f32, u32, u32, u64, u64) -> u32;"],
  ["DuplexCallbackV3", "*const f32, u32, *mut f32, u32, u32, u64, u64, u64) -> u32;", "*const f32, u32, *mut f32, u32, u32, u64, u64) -> u32;"],
  ["EventCallbackV3", "fn(*mut c_void, u32, u32, *const u8, usize);", "fn(*mut c_void, u32, u64, *const u8, usize);"],
  ["AbiVersionFn", 'type AbiVersionFn = unsafe extern "C" fn() -> u32;', 'type AbiVersionFn = unsafe extern "C" fn() -> u64;'],
  ["BuildFlagsFn", 'type BuildFlagsFn = unsafe extern "C" fn() -> u32;', 'type BuildFlagsFn = unsafe extern "C" fn() -> u64;'],
  ["DriversJsonFn", 'type DriversJsonFn = unsafe extern "C" fn(*mut BridgeStringV3, *mut BridgeStringV3) -> u32;', 'type DriversJsonFn = unsafe extern "C" fn(*const BridgeStringV3, *mut BridgeStringV3) -> u32;'],
  ["CapabilitiesJsonFn", "*const u8, usize, *mut BridgeStringV3, *mut BridgeStringV3) -> u32;", "*const u8, u32, *mut BridgeStringV3, *mut BridgeStringV3) -> u32;"],
  ["StringFreeFn", 'type StringFreeFn = unsafe extern "C" fn(BridgeStringV3);', 'type StringFreeFn = unsafe extern "C" fn(*mut BridgeStringV3);'],
  ["StartFn", "Option<EventCallbackV3>,", "Option<OutputCallbackV3>,"],
  ["StopFn", 'type StopFn = unsafe extern "C" fn(*mut c_void, *mut BridgeStringV3, *mut BridgeStringV3) -> u32;', 'type StopFn = unsafe extern "C" fn(*const c_void, *mut BridgeStringV3, *mut BridgeStringV3) -> u32;'],
  ["CloseFn", "type CloseFn =\n    unsafe extern \"C\" fn(*mut *mut c_void, *mut BridgeStringV3, *mut BridgeStringV3) -> u32;", "type CloseFn =\n    unsafe extern \"C\" fn(*mut c_void, *mut BridgeStringV3, *mut BridgeStringV3) -> u32;"],
  ["TelemetryJsonFn", "type TelemetryJsonFn =\n    unsafe extern \"C\" fn(*const c_void, *mut BridgeStringV3, *mut BridgeStringV3) -> u32;", "type TelemetryJsonFn =\n    unsafe extern \"C\" fn(*mut c_void, *mut BridgeStringV3, *mut BridgeStringV3) -> u32;"],
]);

export function assertHeaderV3Contract(headerText) {
  const declared = [...headerText.matchAll(/\b(syndocal_asio_[a-z0-9_]+)\s*\(/gu)].map((match) => match[1]);
  assertExactList(declared, exactBridgeExports, "public ASIO v2/v3 header exports");
  assert.match(headerText, /typedef struct SyndocalAsioV3Handle SyndocalAsioV3Handle;/u, "header must keep the opaque v3 handle typedef");
  assert.match(headerText, /typedef struct SyndocalAsioStringV3\s*\{\s*uint8_t \*ptr;\s*size_t len;\s*\} SyndocalAsioStringV3;/su, "header must keep the exact v3 bridge string layout");
  const header = normalizedC(headerText);
  for (const signature of [
    "typedef uint32_t(SYNDOCAL_ASIO_CALL *SyndocalAsioOutputCallbackV3)( void *context, float *interleaved_output, uint32_t output_channels, uint32_t frames, uint64_t first_output_frame, uint64_t session_generation, uint64_t render_generation);",
    "typedef uint32_t(SYNDOCAL_ASIO_CALL *SyndocalAsioDuplexCallbackV3)( void *context, const float *interleaved_input, uint32_t input_channels, float *interleaved_output, uint32_t output_channels, uint32_t frames, uint64_t first_output_frame, uint64_t session_generation, uint64_t render_generation);",
    "typedef void(SYNDOCAL_ASIO_CALL *SyndocalAsioEventCallbackV3)( void *context, uint32_t severity, uint32_t kind, const uint8_t *message, size_t message_len);",
    "uint32_t SYNDOCAL_ASIO_CALL syndocal_asio_v3_abi_version(void);",
    "uint32_t SYNDOCAL_ASIO_CALL syndocal_asio_v3_build_flags(void);",
    "uint32_t SYNDOCAL_ASIO_CALL syndocal_asio_v3_drivers_json( SyndocalAsioStringV3 *out_json, SyndocalAsioStringV3 *out_error_json);",
    "uint32_t SYNDOCAL_ASIO_CALL syndocal_asio_v3_capabilities_json( const uint8_t *request_json, size_t request_json_len, SyndocalAsioStringV3 *out_json, SyndocalAsioStringV3 *out_error_json);",
    "void SYNDOCAL_ASIO_CALL syndocal_asio_v3_string_free(SyndocalAsioStringV3 string);",
    "uint32_t SYNDOCAL_ASIO_CALL syndocal_asio_v3_start( const uint8_t *request_json, size_t request_json_len, SyndocalAsioOutputCallbackV3 output_callback, SyndocalAsioDuplexCallbackV3 duplex_callback, SyndocalAsioEventCallbackV3 event_callback, void *context, SyndocalAsioV3Handle **out_handle, SyndocalAsioStringV3 *out_result_json, SyndocalAsioStringV3 *out_error_json);",
    "uint32_t SYNDOCAL_ASIO_CALL syndocal_asio_v3_stop( SyndocalAsioV3Handle *handle, SyndocalAsioStringV3 *out_result_json, SyndocalAsioStringV3 *out_error_json);",
    "uint32_t SYNDOCAL_ASIO_CALL syndocal_asio_v3_close( SyndocalAsioV3Handle **handle, SyndocalAsioStringV3 *out_result_json, SyndocalAsioStringV3 *out_error_json);",
    "uint32_t SYNDOCAL_ASIO_CALL syndocal_asio_v3_telemetry_json( const SyndocalAsioV3Handle *handle, SyndocalAsioStringV3 *out_json, SyndocalAsioStringV3 *out_error_json);",
  ]) {
    assert.ok(header.includes(signature), "header has drifted v3 typedef/signature: " + signature);
  }
  assertExactEnum(headerText, "SyndocalAsioStatusV3", {
    SYNDOCAL_ASIO_V3_OK: 0,
    SYNDOCAL_ASIO_V3_INVALID_ARGUMENT: 1,
    SYNDOCAL_ASIO_V3_UNSUPPORTED: 2,
    SYNDOCAL_ASIO_V3_BACKEND_ERROR: 5,
    SYNDOCAL_ASIO_V3_TERMINAL: 6,
    SYNDOCAL_ASIO_V3_PANIC: 255,
  });
  assertExactEnum(headerText, "SyndocalAsioCallbackResultV3", {
    SYNDOCAL_ASIO_V3_CALLBACK_ACCEPTED: 0,
    SYNDOCAL_ASIO_V3_CALLBACK_QUEUE_UNDERFLOW: 1,
    SYNDOCAL_ASIO_V3_CALLBACK_QUEUE_FULL: 2,
    SYNDOCAL_ASIO_V3_CALLBACK_TERMINAL: 3,
    SYNDOCAL_ASIO_V3_CALLBACK_INVALID_BLOCK: 4,
    SYNDOCAL_ASIO_V3_CALLBACK_PANIC: 5,
  });
  assertExactEnum(headerText, "SyndocalAsioEventSeverityV3", {
    SYNDOCAL_ASIO_V3_EVENT_WARNING: 1,
    SYNDOCAL_ASIO_V3_EVENT_TERMINAL: 2,
  });
  assertExactEnum(headerText, "SyndocalAsioEventKindV3", {
    SYNDOCAL_ASIO_V3_EVENT_XRUN: 1,
    SYNDOCAL_ASIO_V3_EVENT_RESET: 2,
    SYNDOCAL_ASIO_V3_EVENT_RESYNC: 3,
    SYNDOCAL_ASIO_V3_EVENT_SAMPLE_RATE_CHANGED: 4,
    SYNDOCAL_ASIO_V3_EVENT_DEVICE_LOST: 5,
    SYNDOCAL_ASIO_V3_EVENT_CALLBACK_GAP: 6,
    SYNDOCAL_ASIO_V3_EVENT_REALTIME_DENIED: 7,
    SYNDOCAL_ASIO_V3_EVENT_BACKEND: 8,
    SYNDOCAL_ASIO_V3_EVENT_MALFORMED_CALLBACK: 9,
    SYNDOCAL_ASIO_V3_EVENT_BUFFER_SIZE_CHANGED: 10,
    SYNDOCAL_ASIO_V3_EVENT_OUTPUT_QUEUE_UNDERFLOW: 11,
    SYNDOCAL_ASIO_V3_EVENT_OUTPUT_QUEUE_FULL: 12,
    SYNDOCAL_ASIO_V3_EVENT_CALLBACK_PANIC: 13,
    SYNDOCAL_ASIO_V3_EVENT_INVALID_OUTPUT_BLOCK: 14,
  });
  for (const required of [
    "SYNDOCAL_ASIO_V3_CALLBACK_ACCEPTED = 0",
    "SYNDOCAL_ASIO_V3_CALLBACK_QUEUE_UNDERFLOW = 1",
    "SYNDOCAL_ASIO_V3_CALLBACK_QUEUE_FULL = 2",
    "SYNDOCAL_ASIO_V3_CALLBACK_TERMINAL = 3",
    "SYNDOCAL_ASIO_V3_CALLBACK_INVALID_BLOCK = 4",
    "SYNDOCAL_ASIO_V3_CALLBACK_PANIC = 5",
    "SYNDOCAL_ASIO_V3_EVENT_OUTPUT_QUEUE_UNDERFLOW = 11",
    "SYNDOCAL_ASIO_V3_EVENT_OUTPUT_QUEUE_FULL = 12",
    "SYNDOCAL_ASIO_V3_EVENT_CALLBACK_PANIC = 13",
    "SYNDOCAL_ASIO_V3_EVENT_INVALID_OUTPUT_BLOCK = 14",
    "native ASIO format",
    "successful Stop/Close",
  ]) {
    assert.ok(headerText.includes(required), "header lacks frozen v3 contract text: " + required);
  }
}

export function assertV3LoaderSource(loaderText) {
  const names = [...loaderText.matchAll(/"(syndocal_asio_v3_[a-z_]+)"/gu)].map((match) => match[1]);
  assertExactList(names, asioV3Exports, "v3 dynamic loader required symbols");
  assert.match(loaderText, /REQUIRED_V3_ABI_VERSION:\s*u32\s*=\s*3;/u, "loader must require ABI v3 exactly");
  const loader = normalizedC(loaderText);
  for (const [name, signature] of requiredV3LoaderSignatures) {
    assert.ok(loader.includes(signature), "loader FFI signature drifted: " + name);
  }
}

export function checkAsioV3Contract({
  workspace = workspaceRoot,
  bridgePath,
  packagedBridgePath,
  inspect = inspectBridgeExports,
} = {}) {
  const headerPath = resolve(workspace, "tools/asio-bridge/include/syndocal_asio_bridge.h");
  const loaderPath = resolve(workspace, "app/src-tauri/src/asio_bridge_v3.rs");
  assertHeaderV3Contract(readFileSync(headerPath, "utf8"));
  assertV3LoaderSource(readFileSync(loaderPath, "utf8"));
  const checked = { headerPath, loaderPath, binaries: [] };
  for (const [label, candidate] of [["bridge", bridgePath], ["packaged bridge", packagedBridgePath]]) {
    if (candidate === undefined) continue;
    const exact = resolve(candidate);
    assert.ok(existsSync(exact), label + " path does not exist: " + exact);
    assertExactAsioBridgeExports(inspect(exact));
    checked.binaries.push(exact);
  }
  return checked;
}

export function runSelfTest() {
  assertExactAsioBridgeExports(exactBridgeExports);
  assert.throws(
    () => assertExactAsioBridgeExports(asioV2Exports),
    /must be exact/u,
    "missing v3 symbol rejects",
  );
  assert.throws(
    () => assertExactAsioBridgeExports([...exactBridgeExports, "syndocal_asio_v3_retired"]),
    /must be exact/u,
    "unknown/retired v3 symbol rejects",
  );
  const header = readFileSync(resolve(workspaceRoot, "tools/asio-bridge/include/syndocal_asio_bridge.h"), "utf8");
  const loader = readFileSync(resolve(workspaceRoot, "app/src-tauri/src/asio_bridge_v3.rs"), "utf8");
  assertHeaderV3Contract(header);
  assertV3LoaderSource(loader);
  assert.throws(
    () => assertHeaderV3Contract(header.replace("SYNDOCAL_ASIO_V3_CALLBACK_QUEUE_FULL = 2", "SYNDOCAL_ASIO_V3_CALLBACK_QUEUE_FULL = 7")),
    /must be exact/u,
    "callback-code mutation rejects",
  );
  assert.throws(
    () => assertHeaderV3Contract(header + "\nuint32_t syndocal_asio_v4_retired(void);\n"),
    /must be exact/u,
    "future/retired header export rejects",
  );
  assert.throws(
    () => assertV3LoaderSource(loader.replace("REQUIRED_V3_ABI_VERSION: u32 = 3", "REQUIRED_V3_ABI_VERSION: u32 = 2")),
    /require ABI v3 exactly/u,
    "loader ABI mutation rejects",
  );
  assert.throws(
    () => assertV3LoaderSource(loader.replace("syndocal_asio_v3_close", "syndocal_asio_v3_retired")),
    /must be exact/u,
    "loader symbol mutation rejects",
  );
  for (const [name, original, replacement] of loaderSignatureMutations) {
    assert.notEqual(original, replacement, "mutation must alter " + name);
    assert.ok(loader.includes(original), "loader mutation source missing: " + name);
    assert.throws(
      () => assertV3LoaderSource(loader.replace(original, replacement)),
      new RegExp("loader FFI signature drifted: " + name, "u"),
      "loader signature mutation rejects: " + name,
    );
  }
  return 22;
}

function parseCli(argv) {
  if (argv.length === 1 && argv[0] === "--self-test") return { selfTest: true };
  if (argv.length !== 4) {
    throw new Error("Usage: check-asio-v3-contract.mjs [--self-test|--bridge <dll> --packaged-bridge <dll>]");
  }
  const options = {};
  for (let index = 0; index < argv.length; index += 2) {
    const flag = argv[index];
    const value = argv[index + 1];
    if (![["--bridge", "bridgePath"], ["--packaged-bridge", "packagedBridgePath"]].some(([known]) => known === flag) || !value) {
      throw new Error("Usage: check-asio-v3-contract.mjs [--self-test|--bridge <dll> [--packaged-bridge <dll>]]");
    }
    options[flag === "--bridge" ? "bridgePath" : "packagedBridgePath"] = value;
  }
  if (!options.bridgePath || !options.packagedBridgePath) {
    throw new Error("ASIO v3 normal contract gate requires both --bridge and --packaged-bridge.");
  }
  return options;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const options = parseCli(process.argv.slice(2));
  if (options.selfTest) {
    const assertions = runSelfTest();
    process.stdout.write("ASIO v3 contract self-test passed: " + assertions + " assertions.\n");
  } else {
    const result = checkAsioV3Contract(options);
    process.stdout.write("ASIO v3 header/loader/packaging contract passed; binaries=" + result.binaries.length + ".\n");
  }
}
