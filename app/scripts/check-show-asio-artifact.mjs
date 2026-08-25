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
  "app/src-tauri/tauri.conf.json",
  "app/src-tauri/tauri.windows.conf.json",
  "app/src-tauri/tauri.show-asio.conf.json",
  "app/scripts/build-windows-show-asio.mjs",
  "app/scripts/prepare-show-asio-runtime.mjs",
  "app/scripts/check-show-asio-artifact.mjs",
  ...showAsioTrustedHelperIdentityPaths,
  "qa/ASIO_SDK_PIN.json",
  "qa/FFMPEG_WINDOWS_RUNTIME_INVENTORY.json",
  "qa/release/windows-show-asio-local-manifest.schema.json",
  "qa/ASIO_SHOW_LOCAL_ONLY.md",
  "tools/asio-bridge/Cargo.lock",
  "tools/asio-bridge/Cargo.toml",
  "tools/asio-bridge/include/syndocal_asio_bridge.h",
]);

const sha256Pattern = /^[0-9a-f]{64}$/u;
const commitPattern = /^[0-9a-f]{40}$/u;
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
    || value.includes("\\")
    || value.includes(":")
    || value.startsWith("/")
    || value.split("/").some((part) => part === "" || part === "." || part === "..")
  ) {
    throw new Error(label + " must be an exact slash-separated relative path.");
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
  const expected = resolve(workspace, expectedShowAsioArtifactRelativeDirectory(version, commit));
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
  return showAsioSourceIdentityPaths.map((path) => {
    const record = readVerifiedRegularFile(resolve(workspace, path), "Show-ASIO source identity " + path, {
      allowedRoots: [workspace],
    });
    return { path, sha256: hash(record.bytes) };
  });
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
      || schema?.$id !== "https://syndocal.invalid/schemas/windows-show-asio-local-manifest-v1.json"
    ) {
      throw new Error("Show-ASIO manifest schema identity is not exact.");
    }
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
  if (manifest.schemaVersion !== 1) throw new Error("Show-ASIO manifest schemaVersion must be exactly 1; future and legacy schemas are rejected.");
  if (manifest.artifactFlavor !== showAsioArtifactFlavor) throw new Error("Show-ASIO manifest artifactFlavor is wrong.");
  if (manifest.platform !== showAsioPlatform) throw new Error("Show-ASIO manifest platform must be windows-x86_64.");
  if (manifest.productVersion !== expectedVersion) throw new Error("Show-ASIO manifest product version drifted from the authoritative build version.");
  if (manifest.commit !== expectedCommit || manifest.commitShort !== expectedCommit.slice(0, 12)) {
    throw new Error("Show-ASIO manifest commit identity drifted from the authoritative build commit.");
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
  const seen = new Set();
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
    if (seen.has(file.path.toLocaleLowerCase("en-US"))) throw new Error("Show-ASIO manifest contains a duplicate file path.");
    seen.add(file.path.toLocaleLowerCase("en-US"));
    assertPositiveInteger(file.byteSize, "Show-ASIO " + file.path + " byteSize");
    assertLowerSha256(file.sha256, "Show-ASIO " + file.path + " SHA-256");
    if (file.pe) assertPeRole(file.pe, file.role, "Show-ASIO " + file.path);
    if (file.role === "asio-bridge") {
      if (file.bridgeAbiVersion !== 2) throw new Error("Show-ASIO bridge ABI must be exactly v2.");
      if (JSON.stringify(file.exports) !== JSON.stringify(showAsioBridgeExports)) {
        throw new Error("Show-ASIO bridge export list is not the exact ABI v2 set.");
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
    expectedHostBindingSha256,
    inventory,
  });
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
    const path = resolve(exactDirectory, ...file.path.split("/"));
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
        throw new Error("Show-ASIO bridge on-disk exports differ from the exact ABI v2 set.");
      }
    }
  }
  return { manifest, artifactDir: exactDirectory, filesVerified: manifest.files.length };
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

function readCurrentCommit(workspace) {
  const result = spawnSync("git.exe", ["rev-parse", "HEAD"], { cwd: workspace, encoding: "utf8", windowsHide: true });
  const commit = String(result.stdout ?? "").trim();
  if (result.error || result.status !== 0 || !commitPattern.test(commit)) throw new Error("Could not resolve one exact current Git commit.");
  return commit;
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
      entry.bridgeAbiVersion = 2;
      entry.exports = [...showAsioBridgeExports];
    }
    return entry;
  });
  const manifest = {
    schemaVersion: 1,
    artifactFlavor: showAsioArtifactFlavor,
    platform: showAsioPlatform,
    productVersion: version,
    commit,
    commitShort: commit.slice(0, 12),
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
  return { workspace, artifactDir, version, commit, hostBinding, inventory, manifest };
}

async function runSelfTest() {
  let assertions = 0;
  const pass = (condition, label) => { assert.ok(condition, label); assertions += 1; };
  const rejects = (action, pattern, label) => { assert.throws(action, pattern, label); assertions += 1; };
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
    const mutations = [
      ["schemaVersion", 2, /schemaVersion/],
      ["artifactFlavor", "future-show", /artifactFlavor/],
      ["productVersion", "1.2.0-alpha.100", /product version drifted/],
      ["commit", "c".repeat(40), /commit identity drifted/],
      ["distributionApproved", true, /distributionApproved/],
    ];
    for (const [key, value, pattern] of mutations) {
      const candidate = structuredClone(fixture.manifest);
      candidate[key] = value;
      rejects(
        () => validateShowAsioManifest(candidate, {
          expectedVersion: fixture.version,
          expectedCommit: fixture.commit,
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
        expectedHostBindingSha256: fixture.hostBinding,
        inventory: fixture.inventory,
      }),
      /feature set/,
      "feature union drift including NDI is rejected",
    );
    const abiDrift = structuredClone(fixture.manifest);
    abiDrift.files[1].bridgeAbiVersion = 3;
    rejects(
      () => validateShowAsioManifest(abiDrift, {
        expectedVersion: fixture.version,
        expectedCommit: fixture.commit,
        expectedHostBindingSha256: fixture.hostBinding,
        inventory: fixture.inventory,
      }),
      /ABI must be exactly v2/,
      "bridge ABI drift is rejected",
    );
    const exportDrift = structuredClone(fixture.manifest);
    exportDrift.files[1].exports = showAsioBridgeExports.slice(1);
    rejects(
      () => validateShowAsioManifest(exportDrift, {
        expectedVersion: fixture.version,
        expectedCommit: fixture.commit,
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

async function main(argv = process.argv.slice(2)) {
  if (argv.length === 1 && argv[0] === "--self-test") {
    await runSelfTest();
    return;
  }
  if (argv.length !== 0) throw new Error("Usage: check-show-asio-artifact.mjs [--self-test]");
  const version = readCurrentProductVersion(workspaceRoot);
  const commit = readCurrentCommit(workspaceRoot);
  const hostBinding = computeShowAsioHostBinding(workspaceRoot, readWindowsMachineGuid());
  const artifactDir = resolve(workspaceRoot, expectedShowAsioArtifactRelativeDirectory(version, commit));
  const result = checkShowAsioArtifact({
    workspace: workspaceRoot,
    artifactDir,
    expectedVersion: version,
    expectedCommit: commit,
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
