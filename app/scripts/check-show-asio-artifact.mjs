import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import {
  existsSync,
  lstatSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, isAbsolute, join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";
import Ajv2020 from "ajv/dist/2020.js";
import { parseStrictJson } from "./strict-json.mjs";
import { requiredMsvcToolchain } from "./run-tauri.mjs";
import {
  assertDirectoryTreeHasNoReparsePoints,
  assertSafeExternalDirectory,
  loadWindowsRuntimeInventory,
  readVerifiedRegularFile,
  verifyPinnedRuntimeFile,
} from "./windows-runtime-inventory.mjs";

const scriptDir = dirname(fileURLToPath(import.meta.url));
export const workspaceRoot = resolve(scriptDir, "../..");
export const showAsioManifestFilename = "show-asio-local-manifest.json";
export const showAsioArtifactFlavor = "windows-show-asio-local-only";
export const showAsioPlatform = "windows-x86_64";
export const showAsioFeatures = Object.freeze(["libav", "spout", "show-asio"]);
export const showAsioBridgeExports = Object.freeze([
  "syndocal_asio_v2_abi_version",
  "syndocal_asio_v2_build_flags",
  "syndocal_asio_v2_capabilities_json",
  "syndocal_asio_v2_close",
  "syndocal_asio_v2_drivers_json",
  "syndocal_asio_v2_start",
  "syndocal_asio_v2_stop",
  "syndocal_asio_v2_string_free",
  "syndocal_asio_v2_telemetry_json",
  "syndocal_asio_v3_abi_version",
  "syndocal_asio_v3_build_flags",
  "syndocal_asio_v3_capabilities_json",
  "syndocal_asio_v3_close",
  "syndocal_asio_v3_drivers_json",
  "syndocal_asio_v3_start",
  "syndocal_asio_v3_stop",
  "syndocal_asio_v3_string_free",
  "syndocal_asio_v3_telemetry_json",
]);
export const showAsioTrustedHelperIdentityPaths = Object.freeze([
  "app/scripts/check-release-metadata.mjs",
  "app/scripts/prepare-release-runtime.mjs",
  "app/scripts/run-tauri.mjs",
  "app/scripts/strict-json.mjs",
  "app/scripts/windows-runtime-inventory.mjs",
]);
export const showAsioSourceIdentityPaths = Object.freeze([
  "Cargo.lock",
  "Cargo.toml",
  "app/package.json",
  "app/src-tauri/Cargo.toml",
  "crates/engine/Cargo.toml",
  "crates/protocol/Cargo.toml",
  "app/src-tauri/tauri.conf.json",
  "app/src-tauri/tauri.windows.conf.json",
  "app/src-tauri/tauri.show-asio.conf.json",
  "app/scripts/build-windows-show-asio.mjs",
  "app/scripts/check-asio-v3-contract.mjs",
  "app/scripts/check-audio-output-control.mjs",
  "app/scripts/check-audio-output-panel.mjs",
  "app/scripts/prepare-show-asio-runtime.mjs",
  "app/scripts/check-show-asio-artifact.mjs",
  "app/scripts/test-asio-v3-contract.mjs",
  "app/src-tauri/src/main.rs",
  "app/src-tauri/src/control_plane.rs",
  "app/src-tauri/src/dvc_import.rs",
  "app/src-tauri/src/asio_bridge_v2.rs",
  "app/src-tauri/src/asio_bridge_v3.rs",
  "app/src-tauri/src/asio_output_preflight_command.rs",
  "app/src-tauri/src/asio_program_cue.rs",
  "app/src-tauri/src/asio_program_cue_render.rs",
  "app/src-tauri/src/asio_output_runtime.rs",
  "app/src-tauri/src/asio_timeline_output.rs",
  "app/src-tauri/src/asio_timeline_transport.rs",
  "app/src-tauri/src/audio_output_router.rs",
  "app/src-tauri/src/audio_output_router_tests.rs",
  "app/src-tauri/src/normal_audio_output.rs",
  "app/src-tauri/src/timeline_cue_audio.rs",
  "app/src/App.tsx",
  "app/src/audioOutputControl.ts",
  "app/src/uiLocalization.ts",
  "app/src/components/AudioOutputPanel.css",
  "app/src/components/AudioOutputPanel.tsx",
  "app/src/components/IoConnectionDeck.tsx",
  "app/src/components/TimelineCueEventsPanel.tsx",
  "app/src/components/TimelineOverview.tsx",
  "app/src/projectOpenBootstrap.ts",
  "app/src/types.ts",
  "app/src/tauri-invoke-manifest.json",
  "app/src/tauriInvokeCommands.ts",
  ...showAsioTrustedHelperIdentityPaths,
  "qa/ASIO_SDK_PIN.json",
  "qa/FFMPEG_WINDOWS_RUNTIME_INVENTORY.json",
  "qa/release/windows-show-asio-local-manifest.schema.json",
  "qa/ASIO_SHOW_LOCAL_ONLY.md",
  "tools/asio-bridge/Cargo.lock",
  "tools/asio-bridge/Cargo.toml",
  "tools/asio-bridge/README.md",
  "tools/asio-bridge/build.rs",
  "tools/asio-bridge/include/syndocal_asio_bridge.h",
  "tools/asio-bridge/src/lib.rs",
  "tools/asio-bridge/src/asio_host_lease.rs",
  "tools/asio-bridge/src/v3_abi.rs",
  "tools/asio-bridge/src/v3_abi_tests.rs",
  "tools/asio-bridge/src/v3_native.rs",
  "tools/asio-bridge/src/v3_rt_backend.rs",
  "tools/asio-bridge/src/v3_rt_backend_tests.rs",
  "tools/asio-bridge/src/v3_sdk_ffi.rs",
  "tools/asio-bridge/src/v3_sdk_backend.cpp",
  "tools/asio-bridge/src/v3_sdk_backend.h",
  "crates/engine/src/timeline_audio_live_fence.rs",
  "crates/engine/src/lib.rs",
  "crates/protocol/src/lib.rs",
]);
// Evidence-only commits may be appended after the artifact source commit, but
// they must never alter the source identity above. Keep this list deliberately
// finite: a broad `qa/**` rule would allow source or policy changes to hide in
// an evidence descendant.
export const showAsioEvidenceOnlyPathAllowlist = Object.freeze([
  "qa/ASIO_INPUT_ACCEPTANCE.md",
  "qa/ASIO_PROGRAM_CUE_OUTPUT_ACCEPTANCE.md",
  "qa/SYNDOCAL_COMPLETION_FLOW_2026-08-19.md",
  "qa/SYNDOCAL_SHOW_COMPLETION_HANDOFF_2026-08-28.md",
  "qa/artifacts/three-display/2026-08-28-display-inventory.json",
]);
const showAsioCriticalRuntimeIdentityPaths = Object.freeze([
  "crates/engine/Cargo.toml",
  "crates/protocol/Cargo.toml",
  "app/src-tauri/src/main.rs",
  "app/src-tauri/src/control_plane.rs",
  "app/src-tauri/src/dvc_import.rs",
  "app/src-tauri/src/asio_bridge_v2.rs",
  "app/src-tauri/src/asio_bridge_v3.rs",
  "app/src-tauri/src/asio_output_preflight_command.rs",
  "app/src-tauri/src/asio_program_cue.rs",
  "app/src-tauri/src/asio_program_cue_render.rs",
  "app/src-tauri/src/asio_output_runtime.rs",
  "app/src-tauri/src/asio_timeline_output.rs",
  "app/src-tauri/src/asio_timeline_transport.rs",
  "app/src-tauri/src/audio_output_router.rs",
  "app/src-tauri/src/normal_audio_output.rs",
  "app/src-tauri/src/timeline_cue_audio.rs",
  "app/src/App.tsx",
  "app/src/audioOutputControl.ts",
  "app/src/components/AudioOutputPanel.tsx",
  "app/src/components/IoConnectionDeck.tsx",
  "app/src/components/TimelineCueEventsPanel.tsx",
  "app/src/components/TimelineOverview.tsx",
  "app/src/projectOpenBootstrap.ts",
  "app/src/types.ts",
  "app/src/tauri-invoke-manifest.json",
  "app/src/tauriInvokeCommands.ts",
  "tools/asio-bridge/build.rs",
  "tools/asio-bridge/include/syndocal_asio_bridge.h",
  "tools/asio-bridge/src/lib.rs",
  "tools/asio-bridge/src/asio_host_lease.rs",
  "tools/asio-bridge/src/v3_abi.rs",
  "tools/asio-bridge/src/v3_native.rs",
  "tools/asio-bridge/src/v3_rt_backend.rs",
  "tools/asio-bridge/src/v3_sdk_ffi.rs",
  "tools/asio-bridge/src/v3_sdk_backend.cpp",
  "tools/asio-bridge/src/v3_sdk_backend.h",
  "crates/engine/src/timeline_audio_live_fence.rs",
  "crates/engine/src/lib.rs",
  "crates/protocol/src/lib.rs",
]);
const showAsioP0IdentityPaths = Object.freeze([
  "app/src-tauri/src/control_plane.rs",
  "app/src-tauri/src/dvc_import.rs",
  "app/src/components/TimelineCueEventsPanel.tsx",
  "app/src/components/TimelineOverview.tsx",
  "app/src/projectOpenBootstrap.ts",
  "app/src/types.ts",
  "crates/engine/src/lib.rs",
]);
const showAsioP1IdentityPaths = Object.freeze([
  "crates/engine/Cargo.toml",
  "crates/protocol/Cargo.toml",
  "app/src-tauri/src/asio_output_preflight_command.rs",
  "app/src-tauri/src/asio_timeline_output.rs",
  "app/src-tauri/src/asio_timeline_transport.rs",
  "crates/engine/src/timeline_audio_live_fence.rs",
  "crates/protocol/src/lib.rs",
]);

const sha256Pattern = /^[0-9a-f]{64}$/u;
const commitPattern = /^[0-9a-f]{40}$/u;
const branchPattern = /^[A-Za-z0-9][A-Za-z0-9._/-]*$/u;
const versionPattern = /^[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$/u;
const manifestSchemaRelativePath = "qa/release/windows-show-asio-local-manifest.schema.json";
const compiledSchemaCache = new Map();

function hash(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

function canonical(candidate) {
  const normalized = resolve(candidate).replaceAll("\\", "/");
  return process.platform === "win32" ? normalized.toLocaleLowerCase("en-US") : normalized;
}

function canonicalRelativePath(value) {
  return value.split("/").join("/").toLocaleLowerCase("en-US");
}

function assertExactKeys(value, expectedKeys, label) {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error(label + " must be an object.");
  }
  const actual = Object.keys(value).sort();
  const expected = [...expectedKeys].sort();
  if (JSON.stringify(actual) !== JSON.stringify(expected)) {
    throw new Error(label + " keys are not exact (expected: " + expected.join(", ") + "; actual: " + actual.join(", ") + ").");
  }
}

function assertLowerSha256(value, label) {
  if (typeof value !== "string" || !sha256Pattern.test(value)) {
    throw new Error(label + " must be an exact lowercase SHA-256 digest.");
  }
}

function assertPositiveInteger(value, label) {
  if (!Number.isSafeInteger(value) || value <= 0) throw new Error(label + " must be a positive safe integer.");
}

function assertPlainRelativePath(value, label) {
  if (
    typeof value !== "string"
    || value.length === 0
    || value !== value.trim()
    || value.includes("\0")
    || /[\r\n\u0001-\u001f\u007f]/u.test(value)
    || value.includes("\\")
    || value.includes(":")
    || value.startsWith("/")
    || value.split("/").some((part) => part === "" || part === "." || part === "..")
  ) {
    throw new Error(label + " must be an exact slash-separated relative path.");
  }
}

export function assertUniqueCaseFoldedRelativePaths(paths, label) {
  if (!Array.isArray(paths)) throw new Error(label + " must be an array of exact slash-separated relative paths.");
  const exact = new Set();
  const folded = new Map();
  for (const path of paths) {
    assertPlainRelativePath(path, label + " path");
    if (exact.has(path)) throw new Error(label + " contains an exact duplicate path: " + path);
    exact.add(path);
    const key = canonicalRelativePath(path);
    const previous = folded.get(key);
    if (previous !== undefined) {
      throw new Error(label + " contains a case-fold duplicate path: " + previous + " / " + path);
    }
    folded.set(key, path);
  }
  return paths;
}

export function assertShowAsioCommit(value, label = "Git commit") {
  if (typeof value !== "string" || !commitPattern.test(value)) {
    throw new Error(label + " must be one exact lowercase 40-character Git object ID.");
  }
  return value;
}

export function assertShowAsioSourceBranch(value, label = "Show-ASIO source branch") {
  if (typeof value !== "string" || !branchPattern.test(value)) {
    throw new Error(label + " must be one exact named Git branch.");
  }
  return value;
}

export function resolveContainedShowAsioPath(workspace, relativePath, label) {
  assertPlainRelativePath(relativePath, label);
  const workspaceAbsolute = resolve(workspace);
  const candidate = resolve(workspaceAbsolute, ...relativePath.split("/"));
  const distance = relative(workspaceAbsolute, candidate).replaceAll("\\", "/");
  if (distance.length === 0 || isAbsolute(distance) || distance === ".." || distance.startsWith("../")) {
    throw new Error(label + " escapes the exact workspace root.");
  }
  return candidate;
}

export function assertShowAsioManifestSchemaSourceIdentityBounds(schema, label = "Show-ASIO manifest schema") {
  const bounds = schema?.properties?.sourceFiles;
  const authorityLength = showAsioSourceIdentityPaths.length;
  if (
    !bounds
    || bounds.minItems !== authorityLength
    || bounds.maxItems !== authorityLength
  ) {
    throw new Error(label + " sourceFiles minItems/maxItems must equal the exact authority length " + String(authorityLength) + ".");
  }
}

export function expectedShowAsioArtifactRelativeDirectory(version, commit) {
  if (typeof version !== "string" || !versionPattern.test(version)) {
    throw new Error("Show-ASIO product version is not an exact SemVer value safe for the local artifact directory.");
  }
  if (typeof commit !== "string" || !commitPattern.test(commit)) {
    throw new Error("Show-ASIO commit must be an exact lowercase 40-character Git object ID.");
  }
  return "target/show-asio-local/Syndocal_Show_ASIO_" + version + "_" + commit.slice(0, 12) + "_x64";
}

export function assertExactShowAsioArtifactDirectory(workspace, candidate, version, commit, { mustExist = true } = {}) {
  if (
    typeof candidate !== "string"
    || candidate.length === 0
    || candidate !== candidate.trim()
    || candidate.includes("\0")
    || /[\r\n]/u.test(candidate)
    || /^[\\/]{2}/u.test(candidate)
    || /^\\\\[?.]/u.test(candidate)
    || candidate.split(/[\\/]+/u).some((part) => part === ".." || part === ".")
    || !isAbsolute(candidate)
  ) {
    throw new Error("Show-ASIO artifact directory must be the exact absolute local non-UNC, non-traversing path.");
  }
  const expectedRelative = expectedShowAsioArtifactRelativeDirectory(version, commit);
  const expected = resolveContainedShowAsioPath(workspace, expectedRelative, "Show-ASIO artifact directory authority");
  const workspaceAbsolute = resolve(workspace);
  const candidateDistance = relative(workspaceAbsolute, resolve(candidate)).replaceAll("\\", "/");
  if (candidateDistance.length === 0 || isAbsolute(candidateDistance) || candidateDistance === ".." || candidateDistance.startsWith("../")) {
    throw new Error("Show-ASIO artifact directory must remain contained by the exact workspace root.");
  }
  if (canonical(candidate) !== canonical(expected)) {
    throw new Error("Show-ASIO artifact directory is not the exact authority path: " + expected);
  }
  if (mustExist) {
    const actual = assertSafeExternalDirectory(candidate, "Show-ASIO artifact directory");
    if (canonical(actual) !== canonical(expected)) throw new Error("Show-ASIO artifact directory resolves outside its exact authority path.");
  }
  return expected;
}

export function parsePeIdentity(bytes, label) {
  if (!Buffer.isBuffer(bytes) || bytes.length < 0x40 || bytes.subarray(0, 2).toString("latin1") !== "MZ") {
    throw new Error(label + " is not a DOS/PE image.");
  }
  const offset = bytes.readUInt32LE(0x3c);
  if (offset > bytes.length - 26) throw new Error(label + " has an out-of-range PE header.");
  const identity = {
    signature: bytes.subarray(offset, offset + 4).toString("latin1"),
    machine: bytes.readUInt16LE(offset + 4),
    characteristics: bytes.readUInt16LE(offset + 22),
    optionalMagic: bytes.readUInt16LE(offset + 24),
  };
  if (identity.signature !== "PE\0\0" || identity.machine !== 0x8664 || identity.optionalMagic !== 0x20b) {
    throw new Error(label + " must be an AMD64 PE32+ image.");
  }
  return identity;
}

function assertPeRole(identity, role, label) {
  assertExactKeys(identity, ["signature", "machine", "characteristics", "optionalMagic"], label + " PE identity");
  if (identity.signature !== "PE\0\0" || identity.machine !== 0x8664 || identity.optionalMagic !== 0x20b) {
    throw new Error(label + " manifest PE identity must be AMD64 PE32+.");
  }
  if ((identity.characteristics & 0x0002) === 0) throw new Error(label + " PE image is not executable.");
  const isDll = (identity.characteristics & 0x2000) !== 0;
  if (role === "application" && isDll) throw new Error(label + " must be an executable, not a DLL.");
  if ((role === "asio-bridge" || role === "ffmpeg-runtime") && !isDll) throw new Error(label + " must carry the PE DLL characteristic.");
}

export function parseDumpbinExports(stdout) {
  const lines = String(stdout).split(/\r?\n/u);
  const header = lines.findIndex((line) => /ordinal\s+hint\s+RVA\s+name/iu.test(line));
  if (header < 0) throw new Error("dumpbin output lacks the exports table header.");
  const exports = [];
  for (const line of lines.slice(header + 1)) {
    if (/^\s*Summary\s*$/iu.test(line)) break;
    const match = /^\s*\d+\s+[0-9A-F]+\s+[0-9A-F]+\s+(\S+)/iu.exec(line);
    if (match) exports.push(match[1]);
  }
  if (exports.length === 0) throw new Error("dumpbin output contains no named exports.");
  if (new Set(exports).size !== exports.length) throw new Error("dumpbin output contains duplicate export names.");
  return exports.sort();
}

export function inspectBridgeExports(dllPath, { environment = process.env, spawn = spawnSync } = {}) {
  if (process.platform !== "win32") throw new Error("ASIO bridge export inspection is Windows-only.");
  const toolchain = requiredMsvcToolchain(environment);
  const dumpbin = resolve(toolchain.installDir, "bin", "Hostx64", "x64", "dumpbin.exe");
  readVerifiedRegularFile(dumpbin, "Pinned MSVC dumpbin.exe");
  const result = spawn(dumpbin, ["/nologo", "/exports", dllPath], {
    encoding: "utf8",
    windowsHide: true,
    env: environment,
    maxBuffer: 16 * 1024 * 1024,
  });
  if (result.error) throw new Error("ASIO bridge export inspection failed: " + String(result.error));
  if (result.status !== 0) throw new Error("ASIO bridge export inspection exited with " + String(result.status) + ".");
  return parseDumpbinExports(result.stdout);
}

export function computeShowAsioHostBinding(workspace, machineGuid) {
  if (
    typeof machineGuid !== "string"
    || !/^(?:[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}|\{[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}\})$/u.test(machineGuid.trim())
  ) {
    throw new Error("Windows MachineGuid is missing or malformed; same-host Show-ASIO binding cannot be established.");
  }
  return hash(Buffer.from(machineGuid.trim().toLocaleLowerCase("en-US") + "\0" + canonical(workspace), "utf8"));
}

export function readWindowsMachineGuid({ spawn = spawnSync } = {}) {
  if (process.platform !== "win32") throw new Error("Show-ASIO local artifacts are Windows-only.");
  const result = spawn(
    "reg.exe",
    ["QUERY", "HKLM\\SOFTWARE\\Microsoft\\Cryptography", "/v", "MachineGuid"],
    { encoding: "utf8", windowsHide: true, maxBuffer: 1024 * 1024 },
  );
  if (result.error || result.status !== 0) {
    throw new Error("Windows MachineGuid query failed closed; the same-host artifact cannot be bound.");
  }
  const matches = [...String(result.stdout).matchAll(/^\s*MachineGuid\s+REG_SZ\s+(\S+)\s*$/gimu)];
  if (matches.length !== 1) throw new Error("Windows MachineGuid query did not return one exact REG_SZ value.");
  return matches[0][1];
}

export function collectShowAsioSourceIdentity(workspace) {
  assertUniqueCaseFoldedRelativePaths(showAsioSourceIdentityPaths, "Show-ASIO source identity authority");
  return showAsioSourceIdentityPaths.map((path) => {
    const sourcePath = resolveContainedShowAsioPath(workspace, path, "Show-ASIO source identity " + path);
    const record = readVerifiedRegularFile(sourcePath, "Show-ASIO source identity " + path, {
      allowedRoots: [workspace],
    });
    return { path, sha256: hash(record.bytes) };
  });
}

function trimGitIdentityOutput(stdout) {
  return String(stdout ?? "").replace(/(?:\r\n|\n|\r)+$/u, "");
}

export const showAsioGitAuthorityEnvironmentNames = Object.freeze([
  "GIT_DIR",
  "GIT_WORK_TREE",
  "GIT_INDEX_FILE",
  "GIT_OBJECT_DIRECTORY",
  "GIT_OBJECT_DIRECTORY_RELATIVE",
  "GIT_ALTERNATE_OBJECT_DIRECTORIES",
  "GIT_COMMON_DIR",
  "GIT_NAMESPACE",
  "GIT_CEILING_DIRECTORIES",
  "GIT_DISCOVERY_ACROSS_FILESYSTEM",
  "GIT_GRAFT_FILE",
  "GIT_SHALLOW_FILE",
  "GIT_REPLACE_REF_BASE",
  "GIT_NO_REPLACE_OBJECTS",
  "GIT_CONFIG",
  "GIT_CONFIG_GLOBAL",
  "GIT_CONFIG_SYSTEM",
  "GIT_CONFIG_NOSYSTEM",
  "GIT_CONFIG_COUNT",
  "GIT_CONFIG_PARAMETERS",
]);
const showAsioGitNoReplaceObjectsArguments = Object.freeze(["--no-replace-objects"]);

function showAsioGitEnvironmentOverrideNames(environment) {
  const exactNames = new Set(showAsioGitAuthorityEnvironmentNames);
  return Object.keys(environment ?? {})
    .filter((name) => {
      const upperName = name.toLocaleUpperCase("en-US");
      return exactNames.has(upperName) || upperName.startsWith("GIT_CONFIG_");
    })
    .sort();
}

export function assertShowAsioGitEnvironmentSafe(environment = process.env) {
  if (typeof environment !== "object" || environment === null || Array.isArray(environment)) {
    throw new Error("Show-ASIO Git authority environment must be an object.");
  }
  const overrides = showAsioGitEnvironmentOverrideNames(environment);
  if (overrides.length !== 0) {
    throw new Error("Show-ASIO Git commands reject repository/index/object/config authority environment overrides: " + overrides.join(", ") + ".");
  }
  return environment;
}

function showAsioGitSpawnEnvironment() {
  const environment = { ...process.env };
  assertShowAsioGitEnvironmentSafe(environment);
  return environment;
}

function showAsioGitArguments(args) {
  return [...showAsioGitNoReplaceObjectsArguments, ...args];
}

function runShowAsioGit(workspace, args, { spawn = spawnSync, encoding = "utf8" } = {}) {
  const result = spawn("git.exe", showAsioGitArguments(args), {
    cwd: workspace,
    encoding,
    windowsHide: true,
    env: showAsioGitSpawnEnvironment(),
    maxBuffer: 32 * 1024 * 1024,
  });
  if (result.error || result.status !== 0) {
    const detail = trimGitIdentityOutput(result.stderr);
    throw new Error("Show-ASIO Git provenance command failed: git " + args.join(" ") + (detail ? " (" + detail + ")" : "."));
  }
  return result.stdout;
}

function readShowAsioGitIdentity(workspace, args, label, { spawn = spawnSync } = {}) {
  const value = trimGitIdentityOutput(runShowAsioGit(workspace, args, { spawn }));
  if (value.length === 0 || /[\r\n]/u.test(value)) throw new Error(label + " is unavailable or contains multiple lines.");
  return value;
}

export function parseShowAsioGitNameStatus(output) {
  const fields = String(output ?? "").split("\0");
  if (fields.length > 0 && fields[fields.length - 1] === "") fields.pop();
  const records = [];
  for (let index = 0; index < fields.length;) {
    const status = fields[index++];
    if (typeof status !== "string" || status.length === 0) throw new Error("Show-ASIO Git diff contains an empty status record.");
    const kind = status[0];
    if (kind === "R" || kind === "C") {
      if (index + 1 >= fields.length) throw new Error("Show-ASIO Git diff contains an incomplete rename/copy record.");
      records.push({ status, paths: [fields[index++], fields[index++]] });
    } else {
      if (index >= fields.length) throw new Error("Show-ASIO Git diff contains an incomplete path record.");
      records.push({ status, paths: [fields[index++]] });
    }
  }
  return records;
}

function assertShowAsioEvidenceOnlyAllowlist(paths) {
  assertUniqueCaseFoldedRelativePaths(paths, "Show-ASIO evidence-only path allowlist");
  const sourcePaths = new Set(showAsioSourceIdentityPaths.map((path) => canonicalRelativePath(path)));
  for (const path of paths) {
    if (sourcePaths.has(canonicalRelativePath(path))) {
      throw new Error("Show-ASIO evidence-only allowlist overlaps source identity path: " + path);
    }
  }
  return new Set(paths);
}

function runShowAsioGitStatus(workspace, args, { spawn = spawnSync } = {}) {
  const result = spawn("git.exe", showAsioGitArguments(args), {
    cwd: workspace,
    encoding: "utf8",
    windowsHide: true,
    env: showAsioGitSpawnEnvironment(),
    maxBuffer: 32 * 1024 * 1024,
  });
  if (result.error || result.status !== 0) {
    throw new Error("Show-ASIO Git provenance command failed: git " + args.join(" ") + ".");
  }
  return String(result.stdout ?? "");
}

function assertShowAsioAncestor(sourceHead, evidenceHead, workspace, { spawn = spawnSync } = {}) {
  const result = spawn("git.exe", showAsioGitArguments(["merge-base", "--is-ancestor", sourceHead, evidenceHead]), {
    cwd: workspace,
    encoding: "utf8",
    windowsHide: true,
    env: showAsioGitSpawnEnvironment(),
    maxBuffer: 1024 * 1024,
  });
  if (result.error) throw new Error("Show-ASIO Git ancestry query failed: " + String(result.error));
  if (result.status === 0) return;
  if (result.status === 1) {
    throw new Error("Show-ASIO artifact source commit is not an ancestor of the evidence HEAD.");
  }
  throw new Error("Show-ASIO Git ancestry query failed with exit code " + String(result.status) + ".");
}

function assertShowAsioEvidenceCommitDiff(commit, expectedParent, workspace, allowlist, { spawn = spawnSync } = {}) {
  const diff = runShowAsioGitStatus(workspace, [
    "diff-tree",
    "--no-commit-id",
    "--no-ext-diff",
    "--no-textconv",
    "--name-status",
    "-r",
    "-z",
    "--find-renames=50%",
    "--find-copies=50%",
    expectedParent,
    commit,
  ], { spawn });
  const records = parseShowAsioGitNameStatus(diff);
  if (records.length === 0) {
    throw new Error("Show-ASIO evidence-only descendant contains an empty commit; every descendant commit must record allowlisted evidence.");
  }
  for (const record of records) {
    const kind = record.status[0];
    if (kind === "D") throw new Error("Show-ASIO evidence-only commit contains a deletion (D): " + record.paths.join(" -> "));
    if (kind === "R") throw new Error("Show-ASIO evidence-only commit contains a rename (R): " + record.paths.join(" -> "));
    if (kind === "T") throw new Error("Show-ASIO evidence-only commit contains a type change (T): " + record.paths.join(" -> "));
    if (record.status !== "A" && record.status !== "M") {
      throw new Error("Show-ASIO evidence-only commit contains unsupported Git status " + record.status + ".");
    }
    for (const path of record.paths) {
      assertPlainRelativePath(path, "Show-ASIO evidence-only commit path");
      if (!allowlist.has(path)) {
        if (showAsioSourceIdentityPaths.includes(path)) {
          throw new Error("Show-ASIO evidence-only commit mutates source identity path: " + path);
        }
        throw new Error("Show-ASIO evidence-only commit path is outside the exact docs/evidence allowlist: " + path);
      }
    }
  }
  return records;
}

export function verifyShowAsioEvidenceOnlyDescendant({
  workspace = workspaceRoot,
  artifactSourceHead,
  evidenceHead,
  sourceBranch,
  spawn = spawnSync,
} = {}) {
  assertShowAsioCommit(artifactSourceHead, "Show-ASIO artifact source commit");
  assertShowAsioCommit(evidenceHead, "Show-ASIO evidence HEAD");
  assertShowAsioSourceBranch(sourceBranch);
  const exactAllowlist = assertShowAsioEvidenceOnlyAllowlist(showAsioEvidenceOnlyPathAllowlist);
  const currentBranch = readShowAsioGitIdentity(workspace, ["symbolic-ref", "--quiet", "--short", "HEAD"], "Show-ASIO current branch", { spawn });
  if (currentBranch !== sourceBranch) {
    throw new Error("Show-ASIO evidence HEAD is on branch '" + currentBranch + "', expected the exact source branch '" + sourceBranch + "'.");
  }
  const currentHead = assertShowAsioCommit(
    readShowAsioGitIdentity(workspace, ["rev-parse", "HEAD"], "Show-ASIO current HEAD", { spawn }),
    "Show-ASIO current HEAD",
  );
  if (currentHead !== evidenceHead) throw new Error("Show-ASIO current HEAD does not equal the supplied evidence HEAD.");
  const upstream = assertShowAsioCommit(
    readShowAsioGitIdentity(workspace, ["rev-parse", "@{upstream}"], "Show-ASIO upstream", { spawn }),
    "Show-ASIO upstream",
  );
  if (upstream !== evidenceHead) throw new Error("Show-ASIO upstream does not equal the supplied evidence HEAD.");
  const status = runShowAsioGitStatus(workspace, ["status", "--porcelain=v1", "--untracked-files=all"], { spawn });
  if (status.length !== 0) throw new Error("Show-ASIO evidence-only verification requires a clean checkout.");
  assertShowAsioAncestor(artifactSourceHead, evidenceHead, workspace, { spawn });

  const commitsOutput = runShowAsioGitStatus(workspace, ["rev-list", "--reverse", "--topo-order", artifactSourceHead + ".." + evidenceHead], { spawn });
  const commits = trimGitIdentityOutput(commitsOutput).split(/\r?\n/u).filter(Boolean);
  let expectedParent = artifactSourceHead;
  const commitRecords = [];
  for (const commit of commits) {
    assertShowAsioCommit(commit, "Show-ASIO evidence descendant commit");
    const parentLine = readShowAsioGitIdentity(workspace, ["rev-list", "--parents", "-n", "1", commit], "Show-ASIO evidence commit parents", { spawn });
    const parentTokens = parentLine.split(/\s+/u);
    if (parentTokens.length !== 2) {
      throw new Error("Show-ASIO evidence-only descendant contains a merge commit; merges are rejected.");
    }
    if (parentTokens[0] !== commit || parentTokens[1] !== expectedParent) {
      throw new Error("Show-ASIO evidence-only commits are not one sequential first-parent chain.");
    }
    const changed = assertShowAsioEvidenceCommitDiff(commit, expectedParent, workspace, exactAllowlist, { spawn });
    commitRecords.push({ commit, parent: expectedParent, changed });
    expectedParent = commit;
  }
  if (expectedParent !== evidenceHead) {
    throw new Error("Show-ASIO evidence-only descendant history did not terminate at the supplied evidence HEAD.");
  }
  return {
    artifactSourceHead,
    evidenceHead,
    sourceBranch,
    currentHead,
    upstream,
    currentBranch,
    commits: commitRecords,
  };
}

function validateManifestAgainstTrackedSchema(workspace, manifest) {
  const schemaRecord = readVerifiedRegularFile(
    resolve(workspace, manifestSchemaRelativePath),
    "Show-ASIO manifest schema",
    { allowedRoots: [workspace] },
  );
  const schemaHash = hash(schemaRecord.bytes);
  let validate = compiledSchemaCache.get(schemaHash);
  if (!validate) {
    const text = schemaRecord.bytes.toString("utf8");
    if (!Buffer.from(text, "utf8").equals(schemaRecord.bytes)) throw new Error("Show-ASIO manifest schema is not valid UTF-8.");
    const schema = parseStrictJson(text, "Show-ASIO manifest schema");
    if (
      schema?.$schema !== "https://json-schema.org/draft/2020-12/schema"
      || schema?.$id !== "https://syndocal.invalid/schemas/windows-show-asio-local-manifest-v3.json"
    ) {
      throw new Error("Show-ASIO manifest schema identity is not exact.");
    }
    assertShowAsioManifestSchemaSourceIdentityBounds(schema);
    validate = new Ajv2020({ allErrors: true, strict: true }).compile(schema);
    compiledSchemaCache.set(schemaHash, validate);
  }
  if (!validate(manifest)) {
    const details = (validate.errors ?? []).map((error) => error.instancePath + " " + error.message).join("; ");
    throw new Error("Show-ASIO manifest violates its tracked schema: " + details);
  }
}

function assertSourceIdentity(sourceFiles, label) {
  if (!Array.isArray(sourceFiles) || sourceFiles.length !== showAsioSourceIdentityPaths.length) {
    throw new Error(label + " must contain the exact authoritative source-file set.");
  }
  assertUniqueCaseFoldedRelativePaths(showAsioSourceIdentityPaths, "Show-ASIO source identity authority");
  assertUniqueCaseFoldedRelativePaths(sourceFiles.map((entry) => entry?.path), label);
  for (let index = 0; index < sourceFiles.length; index += 1) {
    const entry = sourceFiles[index];
    assertExactKeys(entry, ["path", "sha256"], label + " entry");
    if (entry.path !== showAsioSourceIdentityPaths[index]) throw new Error(label + " order/path differs from the exact authority set.");
    assertLowerSha256(entry.sha256, label + " " + entry.path);
  }
}

function expectedArtifactFiles(inventory) {
  return [
    ["application", "syndocal-show-asio.exe"],
    ["asio-bridge", "syndocal_asio_bridge.dll"],
    ...inventory.runtime_dlls.map((entry) => ["ffmpeg-runtime", entry.filename]),
    ...inventory.common_resources.map((entry) => ["notice", entry.destination]),
    ["notice", "ASIO_SHOW_LOCAL_ONLY.md"],
  ];
}

export function validateShowAsioManifest(manifest, {
  expectedVersion,
  expectedCommit,
  expectedSourceBranch,
  expectedHostBindingSha256,
  inventory,
} = {}) {
  assertExactKeys(manifest, [
    "schemaVersion",
    "artifactFlavor",
    "platform",
    "productVersion",
    "commit",
    "commitShort",
    "sourceBranch",
    "distributionApproved",
    "sameHostOnly",
    "unbundled",
    "featureSet",
    "hostBindingSha256",
    "artifactDirectory",
    "buildTargets",
    "sourceFiles",
    "restrictions",
    "verification",
    "files",
  ], "Show-ASIO manifest");
  if (manifest.schemaVersion !== 3) throw new Error("Show-ASIO manifest schemaVersion must be exactly 3; future and legacy schemas are rejected.");
  if (manifest.artifactFlavor !== showAsioArtifactFlavor) throw new Error("Show-ASIO manifest artifactFlavor is wrong.");
  if (manifest.platform !== showAsioPlatform) throw new Error("Show-ASIO manifest platform must be windows-x86_64.");
  if (manifest.productVersion !== expectedVersion) throw new Error("Show-ASIO manifest product version drifted from the authoritative build version.");
  if (manifest.commit !== expectedCommit || manifest.commitShort !== expectedCommit.slice(0, 12)) {
    throw new Error("Show-ASIO manifest commit identity drifted from the authoritative build commit.");
  }
  assertShowAsioSourceBranch(manifest.sourceBranch, "Show-ASIO manifest sourceBranch");
  if (typeof expectedSourceBranch !== "string" || manifest.sourceBranch !== expectedSourceBranch) {
    throw new Error("Show-ASIO manifest sourceBranch drifted from the authoritative source branch.");
  }
  if (manifest.distributionApproved !== false) throw new Error("Show-ASIO distributionApproved must remain exactly false.");
  if (manifest.sameHostOnly !== true || manifest.unbundled !== true) throw new Error("Show-ASIO artifact must remain same-host-only and unbundled.");
  if (JSON.stringify(manifest.featureSet) !== JSON.stringify(showAsioFeatures)) {
    throw new Error("Show-ASIO feature set must be exactly libav, spout, show-asio in canonical order.");
  }
  assertLowerSha256(manifest.hostBindingSha256, "Show-ASIO host binding");
  if (manifest.hostBindingSha256 !== expectedHostBindingSha256) throw new Error("Show-ASIO host binding does not match this exact host and checkout path.");
  const expectedRelativeDirectory = expectedShowAsioArtifactRelativeDirectory(expectedVersion, expectedCommit);
  if (manifest.artifactDirectory !== expectedRelativeDirectory) throw new Error("Show-ASIO manifest artifact directory is not exact.");
  assertExactKeys(manifest.buildTargets, ["application", "bridge"], "Show-ASIO buildTargets");
  if (
    manifest.buildTargets.application !== "target/show-asio-build/app"
    || manifest.buildTargets.bridge !== "target/show-asio-build/bridge"
  ) {
    throw new Error("Show-ASIO build target directories are not the exact isolated paths.");
  }
  assertSourceIdentity(manifest.sourceFiles, "Show-ASIO sourceFiles");
  assertExactKeys(manifest.restrictions, ["installer", "updater", "archive", "copy", "publish", "signing", "ndi"], "Show-ASIO restrictions");
  for (const [key, value] of Object.entries(manifest.restrictions)) {
    if (value !== false) throw new Error("Show-ASIO restriction " + key + " must remain false.");
  }
  assertExactKeys(manifest.verification, ["checker", "requiredBeforeEveryUse", "manifestWrittenLast"], "Show-ASIO verification");
  if (
    manifest.verification.checker !== "app/scripts/check-show-asio-artifact.mjs"
    || manifest.verification.requiredBeforeEveryUse !== true
    || manifest.verification.manifestWrittenLast !== true
  ) {
    throw new Error("Show-ASIO verification policy is not exact.");
  }
  const expected = expectedArtifactFiles(inventory);
  if (!Array.isArray(manifest.files) || manifest.files.length !== expected.length) {
    throw new Error("Show-ASIO manifest files must be the exact local runtime set.");
  }
  assertUniqueCaseFoldedRelativePaths(manifest.files.map((file) => file?.path), "Show-ASIO manifest files");
  for (let index = 0; index < manifest.files.length; index += 1) {
    const file = manifest.files[index];
    const [expectedRole, expectedPath] = expected[index];
    const baseKeys = ["role", "path", "byteSize", "sha256"];
    const keys = expectedRole === "asio-bridge"
      ? [...baseKeys, "pe", "bridgeAbiVersion", "exports"]
      : (expectedRole === "application" || expectedRole === "ffmpeg-runtime" ? [...baseKeys, "pe"] : baseKeys);
    assertExactKeys(file, keys, "Show-ASIO file entry " + String(index));
    assertPlainRelativePath(file.path, "Show-ASIO file path");
    if (file.role !== expectedRole || file.path !== expectedPath) throw new Error("Show-ASIO file role/path order is not exact.");
    assertPositiveInteger(file.byteSize, "Show-ASIO " + file.path + " byteSize");
    assertLowerSha256(file.sha256, "Show-ASIO " + file.path + " SHA-256");
    if (file.pe) assertPeRole(file.pe, file.role, "Show-ASIO " + file.path);
    if (file.role === "asio-bridge") {
      if (file.bridgeAbiVersion !== 3) throw new Error("Show-ASIO bridge maximum ABI must be exactly v3 while retaining v2.");
      if (JSON.stringify(file.exports) !== JSON.stringify(showAsioBridgeExports)) {
        throw new Error("Show-ASIO bridge export list is not the exact ABI v2 plus v3 set.");
      }
    }
  }
  return manifest;
}

function enumerateArtifactTree(root) {
  const files = [];
  const directories = [];
  const visit = (directory) => {
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
      const candidate = join(directory, entry.name);
      const stats = lstatSync(candidate, { bigint: true });
      if (entry.isSymbolicLink() || stats.isSymbolicLink()) throw new Error("Show-ASIO artifact contains a symbolic-link/reparse entry: " + candidate);
      if (entry.isDirectory()) {
        directories.push(relative(root, candidate).split(sep).join("/"));
        visit(candidate);
      }
      else if (entry.isFile()) files.push(relative(root, candidate).split(sep).join("/"));
      else throw new Error("Show-ASIO artifact contains an unsupported filesystem entry: " + candidate);
    }
  };
  visit(root);
  return { files: files.sort(), directories: directories.sort() };
}

export function checkShowAsioArtifact({
  workspace = workspaceRoot,
  artifactDir,
  expectedVersion,
  expectedCommit,
  expectedSourceBranch,
  expectedEvidenceHead,
  expectedHostBindingSha256,
  inventory = loadWindowsRuntimeInventory({ workspace }),
  exportInspector = inspectBridgeExports,
  verifySourceIdentity = true,
} = {}) {
  const exactDirectory = assertExactShowAsioArtifactDirectory(
    workspace,
    artifactDir,
    expectedVersion,
    expectedCommit,
  );
  assertDirectoryTreeHasNoReparsePoints(exactDirectory, "Show-ASIO artifact directory");
  const manifestRecord = readVerifiedRegularFile(
    join(exactDirectory, showAsioManifestFilename),
    "Show-ASIO manifest",
    { allowedRoots: [exactDirectory] },
  );
  const text = manifestRecord.bytes.toString("utf8");
  if (!Buffer.from(text, "utf8").equals(manifestRecord.bytes)) throw new Error("Show-ASIO manifest is not valid UTF-8.");
  const parsedManifest = parseStrictJson(text, "Show-ASIO manifest");
  validateManifestAgainstTrackedSchema(workspace, parsedManifest);
  const manifest = validateShowAsioManifest(parsedManifest, {
    expectedVersion,
    expectedCommit,
    expectedSourceBranch,
    expectedHostBindingSha256,
    inventory,
  });
  let provenance = null;
  if (expectedEvidenceHead !== undefined) {
    provenance = verifyShowAsioEvidenceOnlyDescendant({
      workspace,
      artifactSourceHead: expectedCommit,
      evidenceHead: expectedEvidenceHead,
      sourceBranch: expectedSourceBranch,
    });
  }
  if (verifySourceIdentity) {
    const current = collectShowAsioSourceIdentity(workspace);
    if (JSON.stringify(current) !== JSON.stringify(manifest.sourceFiles)) {
      throw new Error("Show-ASIO source identity changed after the manifest was created.");
    }
  }
  const expectedNames = [showAsioManifestFilename, ...manifest.files.map((file) => file.path)].sort();
  const expectedDirectories = [...new Set(
    manifest.files
      .map((file) => dirname(file.path).replaceAll("\\", "/"))
      .filter((directory) => directory !== "."),
  )].sort();
  const actualTree = enumerateArtifactTree(exactDirectory);
  if (
    JSON.stringify(actualTree.files) !== JSON.stringify(expectedNames)
    || JSON.stringify(actualTree.directories) !== JSON.stringify(expectedDirectories)
  ) {
    throw new Error("Show-ASIO artifact contains missing or extra filesystem entries.");
  }
  for (const file of manifest.files) {
    const path = resolveContainedShowAsioPath(exactDirectory, file.path, "Show-ASIO artifact " + file.path);
    const record = readVerifiedRegularFile(path, "Show-ASIO artifact " + file.path, { allowedRoots: [exactDirectory] });
    if (record.bytes.length !== file.byteSize || hash(record.bytes) !== file.sha256) {
      throw new Error("Show-ASIO artifact file changed after manifest creation: " + file.path);
    }
    if (file.pe) {
      const actualPe = parsePeIdentity(record.bytes, "Show-ASIO artifact " + file.path);
      if (JSON.stringify(actualPe) !== JSON.stringify(file.pe)) throw new Error("Show-ASIO PE identity changed after manifest creation: " + file.path);
      assertPeRole(actualPe, file.role, "Show-ASIO artifact " + file.path);
    }
    if (file.role === "ffmpeg-runtime") {
      const pinned = inventory.runtime_dlls.find((entry) => entry.filename === file.path);
      verifyPinnedRuntimeFile(path, pinned, "Show-ASIO FFmpeg runtime " + file.path, { allowedRoots: [exactDirectory] });
    }
    if (file.role === "asio-bridge") {
      const exports = exportInspector(path);
      if (JSON.stringify(exports) !== JSON.stringify(showAsioBridgeExports)) {
        throw new Error("Show-ASIO bridge on-disk exports differ from the exact ABI v2 plus v3 set.");
      }
    }
  }
  return { manifest, artifactDir: exactDirectory, filesVerified: manifest.files.length, provenance };
}

function readCurrentProductVersion(workspace) {
  const read = (path, label = path) => readVerifiedRegularFile(resolve(workspace, path), label, { allowedRoots: [workspace] }).bytes.toString("utf8");
  const packageText = read("app/package.json");
  const tauriText = read("app/src-tauri/tauri.conf.json", "tauri.conf.json");
  const packageVersion = parseStrictJson(packageText, "app/package.json").version;
  const tauriVersion = parseStrictJson(tauriText, "tauri.conf.json").version;
  const cargoSections = read("Cargo.toml").split(/^\s*(?=\[)/mu).filter((section) => /^\[workspace\.package\]\s*$/mu.test(section));
  if (cargoSections.length !== 1) throw new Error("Current Cargo.toml must contain one exact [workspace.package] section.");
  const cargoVersions = [...cargoSections[0].matchAll(/^version\s*=\s*"([^"]+)"\s*$/gmu)];
  const lockPackages = read("Cargo.lock").split(/^\[\[package\]\]\s*$/mu).slice(1)
    .filter((block) => /^name\s*=\s*"syndocal"\s*$/mu.test(block));
  if (cargoVersions.length !== 1 || lockPackages.length !== 1) {
    throw new Error("Current Cargo/Cargo.lock Syndocal version authority is missing or duplicated.");
  }
  const lockVersions = [...lockPackages[0].matchAll(/^version\s*=\s*"([^"]+)"\s*$/gmu)];
  const cargoVersion = cargoVersions[0][1];
  const lockVersion = lockVersions.length === 1 ? lockVersions[0][1] : null;
  const versions = [packageVersion, tauriVersion, cargoVersion, lockVersion];
  if (versions.some((version) => typeof version !== "string" || !versionPattern.test(version)) || new Set(versions).size !== 1) {
    throw new Error("Current Cargo, Cargo.lock, package, and Tauri product versions are not one exact supported SemVer value.");
  }
  return packageVersion;
}

function syntheticPe({ dll, marker }) {
  const bytes = Buffer.alloc(0x120, marker);
  bytes.write("MZ", 0, "latin1");
  bytes.writeUInt32LE(0x80, 0x3c);
  bytes.write("PE\0\0", 0x80, "latin1");
  bytes.writeUInt16LE(0x8664, 0x84);
  bytes.writeUInt16LE(0x0002 | (dll ? 0x2000 : 0), 0x80 + 22);
  bytes.writeUInt16LE(0x20b, 0x80 + 24);
  return bytes;
}

function makeSyntheticManifestFixture(root) {
  const workspace = join(root, "workspace");
  mkdirSync(workspace, { recursive: true });
  for (const path of showAsioSourceIdentityPaths) {
    const destination = resolve(workspace, ...path.split("/"));
    mkdirSync(dirname(destination), { recursive: true });
    const bytes = path === manifestSchemaRelativePath
      ? readFileSync(resolve(workspaceRoot, manifestSchemaRelativePath))
      : "source:" + path;
    writeFileSync(destination, bytes, { flag: "wx" });
  }
  const version = "1.2.0-alpha.99";
  const commit = "a".repeat(40);
  const sourceBranch = "codex/syndocal-v1.2";
  const hostBinding = "b".repeat(64);
  const artifactDir = resolve(workspace, expectedShowAsioArtifactRelativeDirectory(version, commit));
  mkdirSync(artifactDir, { recursive: true });
  const runtimeDlls = [
    "avcodec-62.dll", "avdevice-62.dll", "avfilter-11.dll", "avformat-62.dll",
    "avutil-60.dll", "swresample-6.dll", "swscale-9.dll",
  ].map((filename, index) => {
    const bytes = syntheticPe({ dll: true, marker: 0x20 + index });
    return {
      filename,
      byte_size: bytes.length,
      sha256: hash(bytes),
      pe_identity: { signature: "PE\0\0", machine: 0x8664, characteristics: 0x2002, optional_magic: 0x20b },
      bytes,
    };
  });
  const common = [
    ["../../THIRD_PARTY_NOTICES.md", "THIRD_PARTY_NOTICES.md"],
    ["../../licenses/FFmpeg-LGPL-3.0.txt", "licenses/FFmpeg-LGPL-3.0.txt"],
    ["../../licenses/Spout2-BSD-2-Clause.txt", "licenses/Spout2-BSD-2-Clause.txt"],
    ["../../licenses/bcdec_rs-MIT.txt", "licenses/bcdec_rs-MIT.txt"],
  ].map(([source, destination]) => {
    const bytes = Buffer.from("notice:" + destination, "utf8");
    return { source, destination, byte_size: bytes.length, sha256: hash(bytes), bytes };
  });
  const inventory = { runtime_dlls: runtimeDlls, common_resources: common };
  const appBytes = syntheticPe({ dll: false, marker: 0x51 });
  const bridgeBytes = syntheticPe({ dll: true, marker: 0x52 });
  const localNotice = Buffer.from("LOCAL ONLY", "utf8");
  const rawFiles = [
    ["application", "syndocal-show-asio.exe", appBytes],
    ["asio-bridge", "syndocal_asio_bridge.dll", bridgeBytes],
    ...runtimeDlls.map((entry) => ["ffmpeg-runtime", entry.filename, entry.bytes]),
    ...common.map((entry) => ["notice", entry.destination, entry.bytes]),
    ["notice", "ASIO_SHOW_LOCAL_ONLY.md", localNotice],
  ];
  const files = rawFiles.map(([role, path, bytes]) => {
    const destination = resolve(artifactDir, ...path.split("/"));
    mkdirSync(dirname(destination), { recursive: true });
    writeFileSync(destination, bytes, { flag: "wx" });
    const entry = { role, path, byteSize: bytes.length, sha256: hash(bytes) };
    if (role !== "notice") entry.pe = parsePeIdentity(bytes, path);
    if (role === "asio-bridge") {
      entry.bridgeAbiVersion = 3;
      entry.exports = [...showAsioBridgeExports];
    }
    return entry;
  });
  const manifest = {
    schemaVersion: 3,
    artifactFlavor: showAsioArtifactFlavor,
    platform: showAsioPlatform,
    productVersion: version,
    commit,
    commitShort: commit.slice(0, 12),
    sourceBranch,
    distributionApproved: false,
    sameHostOnly: true,
    unbundled: true,
    featureSet: [...showAsioFeatures],
    hostBindingSha256: hostBinding,
    artifactDirectory: expectedShowAsioArtifactRelativeDirectory(version, commit),
    buildTargets: { application: "target/show-asio-build/app", bridge: "target/show-asio-build/bridge" },
    sourceFiles: collectShowAsioSourceIdentity(workspace),
    restrictions: { installer: false, updater: false, archive: false, copy: false, publish: false, signing: false, ndi: false },
    verification: { checker: "app/scripts/check-show-asio-artifact.mjs", requiredBeforeEveryUse: true, manifestWrittenLast: true },
    files,
  };
  writeFileSync(join(artifactDir, showAsioManifestFilename), JSON.stringify(manifest, null, 2) + "\n", { flag: "wx" });
  return { workspace, artifactDir, version, commit, sourceBranch, hostBinding, inventory, manifest };
}

function makeSyntheticGitProvenanceSpawn({
  branch = "codex/syndocal-v1.2",
  head,
  upstream = head,
  status = "",
  ancestorStatus = 0,
  commits = [],
  parents = new Map(),
  diffs = new Map(),
} = {}) {
  return (_command, args) => {
    const gitArgs = args[0] === "--no-replace-objects" ? args.slice(1) : args;
    const result = { status: 0, stdout: "", stderr: "" };
    if (gitArgs[0] === "symbolic-ref") result.stdout = branch + "\n";
    else if (gitArgs[0] === "rev-parse" && gitArgs[1] === "HEAD") result.stdout = head + "\n";
    else if (gitArgs[0] === "rev-parse" && gitArgs[1] === "@{upstream}") result.stdout = upstream + "\n";
    else if (gitArgs[0] === "status") result.stdout = status;
    else if (gitArgs[0] === "merge-base") result.status = ancestorStatus;
    else if (gitArgs[0] === "rev-list" && gitArgs[1] === "--parents") {
      const commit = args[args.length - 1];
      result.stdout = [commit, ...(parents.get(commit) ?? [])].join(" ") + "\n";
    } else if (gitArgs[0] === "rev-list") result.stdout = commits.join("\n") + (commits.length ? "\n" : "");
    else if (gitArgs[0] === "diff-tree") result.stdout = diffs.get(args[args.length - 1]) ?? "";
    else {
      result.status = 2;
      result.stderr = "unexpected synthetic git command";
    }
    return result;
  };
}

async function runSelfTest() {
  let assertions = 0;
  const pass = (condition, label) => { assert.ok(condition, label); assertions += 1; };
  const rejects = (action, pattern, label) => { assert.throws(action, pattern, label); assertions += 1; };
  pass(showAsioSourceIdentityPaths.length === 70, "Show-ASIO source identity authority remains exactly 70 paths");
  pass(showAsioSourceIdentityPaths.includes("app/src/uiLocalization.ts"), "Show-ASIO source identity includes the AudioOutput UI localization runtime source");
  pass(showAsioCriticalRuntimeIdentityPaths.length === 39, "Show-ASIO critical runtime mutation authority remains exactly 39 paths");
  pass(showAsioEvidenceOnlyPathAllowlist.length === 5, "Show-ASIO evidence-only path allowlist remains narrow and exact");
  pass(
    JSON.stringify(parseShowAsioCheckerArgs([
      "--artifact-source", "a".repeat(40), "--evidence-head", "b".repeat(40), "--source-branch", "codex/syndocal-v1.2",
    ])) === JSON.stringify({ mode: "check", artifactSourceHead: "a".repeat(40), evidenceHead: "b".repeat(40), sourceBranch: "codex/syndocal-v1.2" }),
    "checker accepts only the exact artifact-source/evidence-head/source-branch CLI shape",
  );
  pass(parseShowAsioCheckerArgs(["--self-test"]).mode === "self-test", "checker accepts the standalone self-test CLI shape");
  for (const argv of [
    [],
    ["--artifact-source", "a".repeat(40), "--evidence-head", "b".repeat(40)],
    ["--artifact-source", "A".repeat(40), "--evidence-head", "b".repeat(40), "--source-branch", "codex/syndocal-v1.2"],
    ["--artifact-source", "a".repeat(40), "--evidence-head", "b".repeat(40), "--source-branch", ""],
  ]) {
    rejects(() => parseShowAsioCheckerArgs(argv), /Usage|exact lowercase|exact named/, "malformed checker argv fails closed: " + argv.join(" "));
  }
  pass(
    assertShowAsioGitEnvironmentSafe({ PATH: "C:\\safe\\bin" }).PATH === "C:\\safe\\bin",
    "Git authority environment accepts an ordinary non-authority variable",
  );
  for (const environmentName of [
    ...showAsioGitAuthorityEnvironmentNames,
    "GIT_CONFIG_KEY_0",
    "GIT_CONFIG_VALUE_0",
  ]) {
    rejects(
      () => assertShowAsioGitEnvironmentSafe({ [environmentName]: "override" }),
      /reject repository\/index\/object\/config authority environment overrides/,
      "Git authority environment override fails closed: " + environmentName,
    );
  }
  const sourceHead = "a".repeat(40);
  const evidenceCommit = "b".repeat(40);
  const evidenceHead = evidenceCommit;
  const validParents = new Map([[evidenceCommit, [sourceHead]]]);
  const validDiffs = new Map([[evidenceCommit, "M\0qa/ASIO_PROGRAM_CUE_OUTPUT_ACCEPTANCE.md\0"]]);
  const validSpawn = makeSyntheticGitProvenanceSpawn({
    head: evidenceHead,
    commits: [evidenceCommit],
    parents: validParents,
    diffs: validDiffs,
  });
  const capturedGitEnvironments = [];
  const capturedGitArguments = [];
  const validProvenance = verifyShowAsioEvidenceOnlyDescendant({
    workspace: workspaceRoot,
    artifactSourceHead: sourceHead,
    evidenceHead,
    sourceBranch: "codex/syndocal-v1.2",
    spawn: (file, args, options) => {
      capturedGitEnvironments.push(options?.env);
      capturedGitArguments.push(args);
      return validSpawn(file, args, options);
    },
  });
  pass(validProvenance.currentHead === evidenceHead && validProvenance.commits.length === 1, "one sequential allowlisted evidence descendant is accepted");
  pass(
    capturedGitEnvironments.length > 0 && capturedGitEnvironments.every((environment) =>
      Object.keys(environment ?? {}).every((name) => {
        const upperName = name.toLocaleUpperCase("en-US");
        return !showAsioGitAuthorityEnvironmentNames.includes(upperName) && !upperName.startsWith("GIT_CONFIG_");
      })),
    "every checker Git subprocess receives an environment without repository/index/object/config authority overrides",
  );
  pass(
    capturedGitArguments.length > 0 && capturedGitArguments.every((args) => args[0] === "--no-replace-objects"),
    "every checker Git subprocess disables refs/replace object substitution",
  );
  const refsReplacePoisonedSpawn = (file, args, options) => {
    if (args[0] !== "--no-replace-objects") {
      return makeSyntheticGitProvenanceSpawn({
        branch: "refs/replace/poisoned",
        head: "c".repeat(40),
      })(file, args, options);
    }
    return validSpawn(file, args, options);
  };
  const replaceProtectedProvenance = verifyShowAsioEvidenceOnlyDescendant({
    workspace: workspaceRoot,
    artifactSourceHead: sourceHead,
    evidenceHead,
    sourceBranch: "codex/syndocal-v1.2",
    spawn: refsReplacePoisonedSpawn,
  });
  pass(
    replaceProtectedProvenance.currentHead === evidenceHead,
    "refs/replace-poisoned Git history cannot alter checker provenance because replacement objects are disabled",
  );
  const sameHead = "d".repeat(40);
  const sameHeadResult = verifyShowAsioEvidenceOnlyDescendant({
    workspace: workspaceRoot,
    artifactSourceHead: sameHead,
    evidenceHead: sameHead,
    sourceBranch: "codex/syndocal-v1.2",
    spawn: makeSyntheticGitProvenanceSpawn({ head: sameHead }),
  });
  pass(sameHeadResult.commits.length === 0, "S=E is explicitly permitted without a descendant commit");
  rejects(
    () => verifyShowAsioEvidenceOnlyDescendant({
      workspace: workspaceRoot, artifactSourceHead: sourceHead, evidenceHead, sourceBranch: "codex/wrong",
      spawn: makeSyntheticGitProvenanceSpawn({ head: evidenceHead }),
    }),
    /exact source branch/,
    "wrong source branch is rejected",
  );
  rejects(
    () => verifyShowAsioEvidenceOnlyDescendant({
      workspace: workspaceRoot, artifactSourceHead: sourceHead, evidenceHead, sourceBranch: "codex/syndocal-v1.2",
      spawn: makeSyntheticGitProvenanceSpawn({ head: evidenceHead, upstream: "e".repeat(40) }),
    }),
    /upstream does not equal/,
    "unpushed evidence HEAD is rejected",
  );
  rejects(
    () => verifyShowAsioEvidenceOnlyDescendant({
      workspace: workspaceRoot, artifactSourceHead: sourceHead, evidenceHead, sourceBranch: "codex/syndocal-v1.2",
      spawn: makeSyntheticGitProvenanceSpawn({ head: evidenceHead, status: " M qa/ASIO_PROGRAM_CUE_OUTPUT_ACCEPTANCE.md" }),
    }),
    /clean checkout/,
    "dirty evidence checkout is rejected",
  );
  rejects(
    () => verifyShowAsioEvidenceOnlyDescendant({
      workspace: workspaceRoot, artifactSourceHead: sourceHead, evidenceHead, sourceBranch: "codex/syndocal-v1.2",
      spawn: makeSyntheticGitProvenanceSpawn({ head: evidenceHead, ancestorStatus: 1 }),
    }),
    /not an ancestor/,
    "non-ancestor artifact source is rejected",
  );
  for (const [status, expected] of [
    ["D\0qa/ASIO_PROGRAM_CUE_OUTPUT_ACCEPTANCE.md\0", /deletion/],
    ["R100\0qa/ASIO_PROGRAM_CUE_OUTPUT_ACCEPTANCE.md\0qa/SYNDOCAL_COMPLETION_FLOW_2026-08-19.md\0", /rename/],
    ["T\0qa/ASIO_PROGRAM_CUE_OUTPUT_ACCEPTANCE.md\0", /type change/],
    ["A\0README.md\0", /outside the exact docs\/evidence allowlist/],
  ]) {
    rejects(
      () => verifyShowAsioEvidenceOnlyDescendant({
        workspace: workspaceRoot, artifactSourceHead: sourceHead, evidenceHead, sourceBranch: "codex/syndocal-v1.2",
        spawn: makeSyntheticGitProvenanceSpawn({
          head: evidenceHead, commits: [evidenceCommit], parents: validParents,
          diffs: new Map([[evidenceCommit, status]]),
        }),
      }),
      expected,
      "forbidden evidence descendant status fails closed: " + status.split("\0")[0],
    );
  }
  rejects(
    () => verifyShowAsioEvidenceOnlyDescendant({
      workspace: workspaceRoot, artifactSourceHead: sourceHead, evidenceHead, sourceBranch: "codex/syndocal-v1.2",
      spawn: makeSyntheticGitProvenanceSpawn({
        head: evidenceHead, commits: [evidenceCommit], parents: new Map([[evidenceCommit, [sourceHead, "e".repeat(40)]]]), diffs: validDiffs,
      }),
    }),
    /merge commit/,
    "merge evidence descendant is rejected",
  );
  rejects(
    () => verifyShowAsioEvidenceOnlyDescendant({
      workspace: workspaceRoot, artifactSourceHead: sourceHead, evidenceHead, sourceBranch: "codex/syndocal-v1.2",
      spawn: makeSyntheticGitProvenanceSpawn({
        head: evidenceHead, commits: [evidenceCommit], parents: validParents,
        diffs: new Map([[evidenceCommit, "M\0Cargo.toml\0"]]),
      }),
    }),
    /mutates source identity path/,
    "source mutation is rejected even when the descendant is otherwise linear",
  );
  const revertedEvidenceHead = "c".repeat(40);
  rejects(
    () => verifyShowAsioEvidenceOnlyDescendant({
      workspace: workspaceRoot,
      artifactSourceHead: sourceHead,
      evidenceHead: revertedEvidenceHead,
      sourceBranch: "codex/syndocal-v1.2",
      spawn: makeSyntheticGitProvenanceSpawn({
        head: revertedEvidenceHead,
        commits: [evidenceCommit, revertedEvidenceHead],
        parents: new Map([[evidenceCommit, [sourceHead]], [revertedEvidenceHead, [evidenceCommit]]]),
        diffs: new Map([
          [evidenceCommit, "M\0Cargo.toml\0"],
          [revertedEvidenceHead, "M\0Cargo.toml\0"],
        ]),
      }),
    }),
    /mutates source identity path/,
    "source mutation followed by a revert is rejected at the intermediate commit",
  );
  const uiLocalizationRevertedEvidenceHead = "f".repeat(40);
  rejects(
    () => verifyShowAsioEvidenceOnlyDescendant({
      workspace: workspaceRoot,
      artifactSourceHead: sourceHead,
      evidenceHead: uiLocalizationRevertedEvidenceHead,
      sourceBranch: "codex/syndocal-v1.2",
      spawn: makeSyntheticGitProvenanceSpawn({
        head: uiLocalizationRevertedEvidenceHead,
        commits: [evidenceCommit, uiLocalizationRevertedEvidenceHead],
        parents: new Map([[evidenceCommit, [sourceHead]], [uiLocalizationRevertedEvidenceHead, [evidenceCommit]]]),
        diffs: new Map([
          [evidenceCommit, "M\0app/src/uiLocalization.ts\0"],
          [uiLocalizationRevertedEvidenceHead, "M\0app/src/uiLocalization.ts\0"],
        ]),
      }),
    }),
    /mutates source identity path/,
    "UI localization source mutation followed by a revert is rejected at the intermediate commit",
  );
  rejects(
    () => verifyShowAsioEvidenceOnlyDescendant({
      workspace: workspaceRoot, artifactSourceHead: sourceHead, evidenceHead, sourceBranch: "codex/syndocal-v1.2",
      spawn: makeSyntheticGitProvenanceSpawn({
        head: evidenceHead, commits: [evidenceCommit], parents: validParents,
        diffs: new Map([[evidenceCommit, "M\0qa/ASIO_SHOW_LOCAL_ONLY.md\0"]]),
      }),
    }),
    /mutates source identity path/,
    "ASIO_SHOW_LOCAL_ONLY.md is not an evidence-only allowlist escape",
  );
  rejects(
    () => assertShowAsioEvidenceOnlyAllowlist(["qa/asio_show_local_only.md"]),
    /overlaps source identity path/,
    "case-fold source identity cannot be added to the evidence allowlist",
  );
  pass(
    showAsioCriticalRuntimeIdentityPaths.every((path) => showAsioSourceIdentityPaths.includes(path)),
    "Show-ASIO critical runtime paths are all present in the exact source identity set",
  );
  pass(
    showAsioP0IdentityPaths.every((path) => showAsioSourceIdentityPaths.includes(path)),
    "Show-ASIO P0 semantic closure paths are all present in the exact source identity set",
  );
  pass(
    showAsioP0IdentityPaths.every((path) => showAsioCriticalRuntimeIdentityPaths.includes(path)),
    "Show-ASIO P0 semantic closure paths are all covered by critical mutation proof",
  );
  pass(
    showAsioP1IdentityPaths.every((path) => showAsioSourceIdentityPaths.includes(path)),
    "Show-ASIO P1 closure paths are all present in the exact source identity set",
  );
  pass(
    showAsioP1IdentityPaths.every((path) => showAsioCriticalRuntimeIdentityPaths.includes(path)),
    "Show-ASIO P1 closure paths are all covered by critical mutation proof",
  );
  const authoritySchema = parseStrictJson(
    readFileSync(resolve(workspaceRoot, manifestSchemaRelativePath), "utf8"),
    "Show-ASIO manifest schema self-test",
  );
  assertShowAsioManifestSchemaSourceIdentityBounds(authoritySchema);
  pass(
    authoritySchema.properties.sourceFiles.minItems === showAsioSourceIdentityPaths.length
      && authoritySchema.properties.sourceFiles.maxItems === showAsioSourceIdentityPaths.length,
    "manifest schema sourceFiles bounds match the exact source identity authority length",
  );
  const staleSchema = structuredClone(authoritySchema);
  staleSchema.properties.sourceFiles.maxItems -= 1;
  rejects(
    () => assertShowAsioManifestSchemaSourceIdentityBounds(staleSchema),
    /minItems\/maxItems must equal the exact authority length/,
    "stale manifest schema sourceFiles bounds fail closed",
  );
  rejects(
    () => assertUniqueCaseFoldedRelativePaths(["Cargo.lock", "Cargo.lock"], "exact duplicate fixture"),
    /exact duplicate path/,
    "exact duplicate slash-separated paths fail closed",
  );
  rejects(
    () => assertUniqueCaseFoldedRelativePaths(["Cargo.lock", "cargo.lock"], "case duplicate fixture"),
    /case-fold duplicate path/,
    "case-fold duplicate slash-separated paths fail closed",
  );
  for (const [candidate, label] of [
    ["./Cargo.lock", "dot-slash path"],
    ["../Cargo.lock", "parent path"],
    ["app\\src\\types.ts", "backslash path"],
    ["app/src/types.ts::$DATA", "ADS alias path"],
  ]) {
    rejects(
      () => assertPlainRelativePath(candidate, "hostile " + label),
      /exact slash-separated relative path/,
      "hostile path form is rejected: " + label,
    );
  }
  pass(
    relative(workspaceRoot, resolveContainedShowAsioPath(workspaceRoot, "target/show-asio-local", "contained path")).replaceAll("\\", "/") === "target/show-asio-local",
    "workspace-contained slash-separated path resolves under the exact root",
  );
  rejects(
    () => resolveContainedShowAsioPath(workspaceRoot, "../outside", "escaping path"),
    /exact slash-separated relative path/,
    "workspace escape path fails closed before filesystem access",
  );
  pass(
    JSON.stringify(parseDumpbinExports("ordinal hint RVA      name\r\n      1    0 00001000 syndocal_asio_v2_stop\r\n      2    1 00002000 syndocal_asio_v2_start\r\n  Summary"))
      === JSON.stringify(["syndocal_asio_v2_start", "syndocal_asio_v2_stop"]),
    "dumpbin exports parse and sort deterministically",
  );
  rejects(() => parseDumpbinExports("no table"), /lacks the exports table/, "dumpbin output without an export table fails closed");
  pass(
    computeShowAsioHostBinding("C:\\show", "00112233-4455-6677-8899-aabbccddeeff").length === 64,
    "one exact canonical MachineGuid produces a host binding",
  );
  rejects(
    () => computeShowAsioHostBinding("C:\\show", "00112233445566778899aabbccddeeff"),
    /missing or malformed/,
    "noncanonical MachineGuid spelling is rejected",
  );
  const root = mkdtempSync(join(tmpdir(), "syndocal-show-asio-check-"));
  try {
    const fixture = makeSyntheticManifestFixture(root);
    const inspect = () => [...showAsioBridgeExports];
    const verified = checkShowAsioArtifact({
      workspace: fixture.workspace,
      artifactDir: fixture.artifactDir,
    expectedVersion: fixture.version,
    expectedCommit: fixture.commit,
    expectedSourceBranch: fixture.sourceBranch,
    expectedHostBindingSha256: fixture.hostBinding,
      inventory: fixture.inventory,
      exportInspector: inspect,
    });
    pass(verified.filesVerified === fixture.manifest.files.length, "exact synthetic local artifact verifies");
    let postBuildMutationExportInspections = 0;
    for (const helperPath of showAsioTrustedHelperIdentityPaths) {
      const absoluteHelperPath = resolve(fixture.workspace, ...helperPath.split("/"));
      const originalHelper = readFileSync(absoluteHelperPath);
      writeFileSync(
        absoluteHelperPath,
        Buffer.concat([originalHelper, Buffer.from("\npost-build mutation", "utf8")]),
      );
      rejects(
        () => checkShowAsioArtifact({
          workspace: fixture.workspace,
          artifactDir: fixture.artifactDir,
          expectedVersion: fixture.version,
          expectedCommit: fixture.commit,
          expectedSourceBranch: fixture.sourceBranch,
          expectedHostBindingSha256: fixture.hostBinding,
          inventory: fixture.inventory,
          exportInspector: () => {
            postBuildMutationExportInspections += 1;
            return [...showAsioBridgeExports];
          },
        }),
        /source identity changed after the manifest was created/,
        "post-build trusted-helper mutation is rejected before artifact acceptance: " + helperPath,
      );
      writeFileSync(absoluteHelperPath, originalHelper);
    }
    pass(
      postBuildMutationExportInspections === 0,
      "trusted-helper mutations fail before bridge SDK inspection or artifact acceptance",
    );
    let runtimeMutationExportInspections = 0;
    for (const sourcePath of showAsioCriticalRuntimeIdentityPaths) {
      const absoluteSourcePath = resolve(fixture.workspace, ...sourcePath.split("/"));
      const originalSource = readFileSync(absoluteSourcePath);
      writeFileSync(
        absoluteSourcePath,
        Buffer.concat([originalSource, Buffer.from("\npost-build runtime mutation", "utf8")]),
      );
      rejects(
        () => checkShowAsioArtifact({
          workspace: fixture.workspace,
          artifactDir: fixture.artifactDir,
          expectedVersion: fixture.version,
          expectedCommit: fixture.commit,
          expectedSourceBranch: fixture.sourceBranch,
          expectedHostBindingSha256: fixture.hostBinding,
          inventory: fixture.inventory,
          exportInspector: () => {
            runtimeMutationExportInspections += 1;
            return [...showAsioBridgeExports];
          },
        }),
        /source identity changed after the manifest was created/,
        "post-build ASIO runtime mutation is rejected before artifact acceptance: " + sourcePath,
      );
      writeFileSync(absoluteSourcePath, originalSource);
    }
    pass(
      runtimeMutationExportInspections === 0,
      "ASIO runtime mutations fail before bridge SDK inspection or artifact acceptance",
    );
    let uiLocalizationMutationExportInspections = 0;
    const uiLocalizationPath = resolve(fixture.workspace, "app/src/uiLocalization.ts");
    const originalUiLocalization = readFileSync(uiLocalizationPath);
    writeFileSync(
      uiLocalizationPath,
      Buffer.concat([originalUiLocalization, Buffer.from("\npost-build UI localization mutation", "utf8")]),
    );
    rejects(
      () => checkShowAsioArtifact({
        workspace: fixture.workspace,
        artifactDir: fixture.artifactDir,
        expectedVersion: fixture.version,
        expectedCommit: fixture.commit,
        expectedSourceBranch: fixture.sourceBranch,
        expectedHostBindingSha256: fixture.hostBinding,
        inventory: fixture.inventory,
        exportInspector: () => {
          uiLocalizationMutationExportInspections += 1;
          return [...showAsioBridgeExports];
        },
      }),
      /source identity changed after the manifest was created/,
      "post-build UI localization mutation is rejected before artifact acceptance",
    );
    writeFileSync(uiLocalizationPath, originalUiLocalization);
    pass(
      uiLocalizationMutationExportInspections === 0,
      "UI localization mutation fails before bridge SDK inspection or artifact acceptance",
    );
    let missingUiLocalizationExportInspections = 0;
    rmSync(uiLocalizationPath);
    rejects(
      () => checkShowAsioArtifact({
        workspace: fixture.workspace,
        artifactDir: fixture.artifactDir,
        expectedVersion: fixture.version,
        expectedCommit: fixture.commit,
        expectedSourceBranch: fixture.sourceBranch,
        expectedHostBindingSha256: fixture.hostBinding,
        inventory: fixture.inventory,
        exportInspector: () => {
          missingUiLocalizationExportInspections += 1;
          return [...showAsioBridgeExports];
        },
      }),
      /Show-ASIO source identity .* is missing/,
      "missing UI localization source is rejected before artifact acceptance",
    );
    writeFileSync(uiLocalizationPath, originalUiLocalization, { flag: "wx" });
    pass(
      missingUiLocalizationExportInspections === 0,
      "missing UI localization source fails before bridge SDK inspection or artifact acceptance",
    );
    let missingP0SourceExportInspections = 0;
    for (const sourcePath of showAsioP0IdentityPaths) {
      const absoluteSourcePath = resolve(fixture.workspace, ...sourcePath.split("/"));
      const originalSource = readFileSync(absoluteSourcePath);
      rmSync(absoluteSourcePath);
      rejects(
        () => checkShowAsioArtifact({
          workspace: fixture.workspace,
          artifactDir: fixture.artifactDir,
          expectedVersion: fixture.version,
          expectedCommit: fixture.commit,
          expectedSourceBranch: fixture.sourceBranch,
          expectedHostBindingSha256: fixture.hostBinding,
          inventory: fixture.inventory,
          exportInspector: () => {
            missingP0SourceExportInspections += 1;
            return [...showAsioBridgeExports];
          },
        }),
        /Show-ASIO source identity .* is missing/,
        "missing Show-ASIO P0 semantic closure source is rejected before artifact acceptance: " + sourcePath,
      );
      writeFileSync(absoluteSourcePath, originalSource, { flag: "wx" });
    }
    pass(
      missingP0SourceExportInspections === 0,
      "missing Show-ASIO P0 semantic closure sources fail before bridge SDK inspection or artifact acceptance",
    );
    let missingSourceExportInspections = 0;
    for (const sourcePath of showAsioP1IdentityPaths) {
      const absoluteSourcePath = resolve(fixture.workspace, ...sourcePath.split("/"));
      const originalSource = readFileSync(absoluteSourcePath);
      rmSync(absoluteSourcePath);
      rejects(
        () => checkShowAsioArtifact({
          workspace: fixture.workspace,
          artifactDir: fixture.artifactDir,
          expectedVersion: fixture.version,
          expectedCommit: fixture.commit,
          expectedSourceBranch: fixture.sourceBranch,
          expectedHostBindingSha256: fixture.hostBinding,
          inventory: fixture.inventory,
          exportInspector: () => {
            missingSourceExportInspections += 1;
            return [...showAsioBridgeExports];
          },
        }),
        /Show-ASIO source identity .* is missing/,
        "missing Show-ASIO P1 closure source is rejected before artifact acceptance: " + sourcePath,
      );
      writeFileSync(absoluteSourcePath, originalSource, { flag: "wx" });
    }
    pass(
      missingSourceExportInspections === 0,
      "missing Show-ASIO P1 closure sources fail before bridge SDK inspection or artifact acceptance",
    );
    const mutations = [
      ["schemaVersion", 2, /schemaVersion/],
      ["schemaVersion", 4, /schemaVersion/],
      ["artifactFlavor", "future-show", /artifactFlavor/],
      ["productVersion", "1.2.0-alpha.100", /product version drifted/],
      ["commit", "c".repeat(40), /commit identity drifted/],
      ["sourceBranch", "codex/wrong", /sourceBranch/],
      ["distributionApproved", true, /distributionApproved/],
    ];
    for (const [key, value, pattern] of mutations) {
      const candidate = structuredClone(fixture.manifest);
      candidate[key] = value;
      rejects(
        () => validateShowAsioManifest(candidate, {
          expectedVersion: fixture.version,
          expectedCommit: fixture.commit,
          expectedSourceBranch: fixture.sourceBranch,
          expectedHostBindingSha256: fixture.hostBinding,
          inventory: fixture.inventory,
        }),
        pattern,
        "manifest mutation is rejected: " + key,
      );
    }
    const unknown = structuredClone(fixture.manifest);
    unknown.future = true;
    rejects(
      () => validateShowAsioManifest(unknown, {
        expectedVersion: fixture.version,
        expectedCommit: fixture.commit,
        expectedSourceBranch: fixture.sourceBranch,
        expectedHostBindingSha256: fixture.hostBinding,
        inventory: fixture.inventory,
      }),
      /keys are not exact/,
      "unknown manifest fields are rejected",
    );
    const featureDrift = structuredClone(fixture.manifest);
    featureDrift.featureSet = ["libav", "spout", "show-asio", "ndi"];
    rejects(
      () => validateShowAsioManifest(featureDrift, {
        expectedVersion: fixture.version,
        expectedCommit: fixture.commit,
        expectedSourceBranch: fixture.sourceBranch,
        expectedHostBindingSha256: fixture.hostBinding,
        inventory: fixture.inventory,
      }),
      /feature set/,
      "feature union drift including NDI is rejected",
    );
    const abiDrift = structuredClone(fixture.manifest);
    abiDrift.files[1].bridgeAbiVersion = 2;
    rejects(
      () => validateShowAsioManifest(abiDrift, {
        expectedVersion: fixture.version,
        expectedCommit: fixture.commit,
        expectedSourceBranch: fixture.sourceBranch,
        expectedHostBindingSha256: fixture.hostBinding,
        inventory: fixture.inventory,
      }),
      /maximum ABI must be exactly v3/,
      "bridge ABI drift is rejected",
    );
    const exportDrift = structuredClone(fixture.manifest);
    exportDrift.files[1].exports = showAsioBridgeExports.slice(1);
    rejects(
      () => validateShowAsioManifest(exportDrift, {
        expectedVersion: fixture.version,
        expectedCommit: fixture.commit,
        expectedSourceBranch: fixture.sourceBranch,
        expectedHostBindingSha256: fixture.hostBinding,
        inventory: fixture.inventory,
      }),
      /export list/,
      "bridge export drift is rejected",
    );
    rejects(
      () => checkShowAsioArtifact({
        workspace: fixture.workspace,
        artifactDir: fixture.artifactDir,
        expectedVersion: fixture.version,
        expectedCommit: fixture.commit,
        expectedSourceBranch: fixture.sourceBranch,
        expectedHostBindingSha256: fixture.hostBinding,
        inventory: fixture.inventory,
        exportInspector: () => showAsioBridgeExports.slice(1),
      }),
      /on-disk exports differ/,
      "wrong inspected bridge exports are rejected even when the manifest claims the exact list",
    );
    const duplicateText = JSON.stringify(fixture.manifest).replace("{", '{"schemaVersion":9,');
    rejects(() => parseStrictJson(duplicateText, "hostile manifest"), /duplicate object key/, "duplicate manifest keys are rejected before trust");
    const manifestPath = join(fixture.artifactDir, showAsioManifestFilename);
    const originalManifest = readFileSync(manifestPath);
    const wrongHashManifest = structuredClone(fixture.manifest);
    wrongHashManifest.files[1].sha256 = "f".repeat(64);
    writeFileSync(manifestPath, JSON.stringify(wrongHashManifest));
    rejects(
      () => checkShowAsioArtifact({
        workspace: fixture.workspace,
        artifactDir: fixture.artifactDir,
        expectedVersion: fixture.version,
        expectedCommit: fixture.commit,
        expectedSourceBranch: fixture.sourceBranch,
        expectedHostBindingSha256: fixture.hostBinding,
        inventory: fixture.inventory,
        exportInspector: inspect,
      }),
      /changed after manifest creation/,
      "manifest bridge hash that does not match on-disk bytes is rejected",
    );
    writeFileSync(manifestPath, originalManifest);
    const appPath = join(fixture.artifactDir, "syndocal-show-asio.exe");
    const originalApp = readFileSync(appPath);
    const tampered = Buffer.from(originalApp);
    tampered[tampered.length - 1] ^= 1;
    writeFileSync(appPath, tampered);
    rejects(
      () => checkShowAsioArtifact({
        workspace: fixture.workspace,
        artifactDir: fixture.artifactDir,
        expectedVersion: fixture.version,
        expectedCommit: fixture.commit,
        expectedSourceBranch: fixture.sourceBranch,
        expectedHostBindingSha256: fixture.hostBinding,
        inventory: fixture.inventory,
        exportInspector: inspect,
      }),
      /changed after manifest creation/,
      "one-byte post-manifest mutation is rejected",
    );
    writeFileSync(appPath, originalApp);
    const extra = join(fixture.artifactDir, "installer.msi");
    writeFileSync(extra, "forbidden", { flag: "wx" });
    mkdirSync(join(fixture.artifactDir, "empty-installer-dir"));
    rejects(
      () => checkShowAsioArtifact({
        workspace: fixture.workspace,
        artifactDir: fixture.artifactDir,
        expectedVersion: fixture.version,
        expectedCommit: fixture.commit,
        expectedSourceBranch: fixture.sourceBranch,
        expectedHostBindingSha256: fixture.hostBinding,
        inventory: fixture.inventory,
        exportInspector: inspect,
      }),
      /missing or extra filesystem entries/,
      "installer/extra payload and empty directory are rejected",
    );
    pass(!existsSync(join(fixture.workspace, "target", "release")), "self-test never creates or touches normal target/release");
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
  console.log("Show-ASIO artifact checker self-test passed: " + assertions + " assertions; Cargo/native/process-stop=NOT_RUN");
}

export function parseShowAsioCheckerArgs(argv) {
  if (!Array.isArray(argv)) throw new Error("Show-ASIO checker arguments must be an array.");
  if (argv.length === 1 && argv[0] === "--self-test") return { mode: "self-test" };
  if (
    argv.length !== 6
    || argv[0] !== "--artifact-source"
    || argv[2] !== "--evidence-head"
    || argv[4] !== "--source-branch"
  ) {
    throw new Error("Usage: check-show-asio-artifact.mjs --artifact-source S --evidence-head E --source-branch B | --self-test");
  }
  const artifactSourceHead = assertShowAsioCommit(argv[1], "--artifact-source");
  const evidenceHead = assertShowAsioCommit(argv[3], "--evidence-head");
  const sourceBranch = assertShowAsioSourceBranch(argv[5], "--source-branch");
  return { mode: "check", artifactSourceHead, evidenceHead, sourceBranch };
}

async function main(argv = process.argv.slice(2)) {
  const parsedArgs = parseShowAsioCheckerArgs(argv);
  if (parsedArgs.mode === "self-test") {
    await runSelfTest();
    return;
  }
  const version = readCurrentProductVersion(workspaceRoot);
  const hostBinding = computeShowAsioHostBinding(workspaceRoot, readWindowsMachineGuid());
  const artifactDir = resolve(workspaceRoot, expectedShowAsioArtifactRelativeDirectory(version, parsedArgs.artifactSourceHead));
  const result = checkShowAsioArtifact({
    workspace: workspaceRoot,
    artifactDir,
    expectedVersion: version,
    expectedCommit: parsedArgs.artifactSourceHead,
    expectedSourceBranch: parsedArgs.sourceBranch,
    expectedEvidenceHead: parsedArgs.evidenceHead,
    expectedHostBindingSha256: hostBinding,
  });
  console.log("Show-ASIO local artifact PASS: " + result.artifactDir + " files=" + String(result.filesVerified) + " distributionApproved=false");
}

if (resolve(process.argv[1] ?? "") === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error("[show-asio-check] " + (error instanceof Error ? error.message : String(error)));
    process.exitCode = 1;
  });
}
