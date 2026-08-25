import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import {
  closeSync,
  constants,
  existsSync,
  fstatSync,
  lstatSync,
  openSync,
  readFileSync,
  readdirSync,
  realpathSync,
} from "node:fs";
import { basename, dirname, isAbsolute, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseStrictJson } from "./strict-json.mjs";

const scriptDir = dirname(fileURLToPath(import.meta.url));
export const workspaceRoot = resolve(scriptDir, "../..");
export const runtimeInventoryRelativePath = "qa/FFMPEG_WINDOWS_RUNTIME_INVENTORY.json";
export const canonicalWindowsFfmpegRuntimeDllNames = Object.freeze([
  "avcodec-62.dll",
  "avdevice-62.dll",
  "avfilter-11.dll",
  "avformat-62.dll",
  "avutil-60.dll",
  "swresample-6.dll",
  "swscale-9.dll",
]);
export const canonicalCommonBundleResources = Object.freeze([
  Object.freeze({ source: "../../THIRD_PARTY_NOTICES.md", destination: "THIRD_PARTY_NOTICES.md" }),
  Object.freeze({ source: "../../licenses/FFmpeg-LGPL-3.0.txt", destination: "licenses/FFmpeg-LGPL-3.0.txt" }),
  Object.freeze({ source: "../../licenses/Spout2-BSD-2-Clause.txt", destination: "licenses/Spout2-BSD-2-Clause.txt" }),
  Object.freeze({ source: "../../licenses/bcdec_rs-MIT.txt", destination: "licenses/bcdec_rs-MIT.txt" }),
]);

export const anchoredAmd64PeIdentity = Object.freeze({
  signature: "PE\0\0",
  machine: 0x8664,
  characteristics: 0x222e,
  optional_magic: 0x20b,
});

export const canonicalPinnedRuntimeIdentity = Object.freeze(new Map([
  ["avcodec-62.dll", Object.freeze({ byte_size: 97454080, sha256: "34f5b1baac01c4be3edf464309c79db05ffbd4a9c905c94b4a4651cd15370296" })],
  ["avdevice-62.dll", Object.freeze({ byte_size: 6323200, sha256: "d213d6cad9f3a526f7664ebb3f93d6882669540db7164daecd03f52e0f5288cc" })],
  ["avfilter-11.dll", Object.freeze({ byte_size: 124344320, sha256: "e318cac83d648869180d0b57c45f21aad1e4db34a467000c157da2938ff7f63f" })],
  ["avformat-62.dll", Object.freeze({ byte_size: 20179968, sha256: "c04e6ed2f9f36d42325d4f4df5babb5d6ce7c55dbffeb7ef1007e25e97bcb716" })],
  ["avutil-60.dll", Object.freeze({ byte_size: 3148288, sha256: "6f172b5d10224fcc3f729c8baa6fd36a97bb58042bb3b1417078d77d2da59b87" })],
  ["swresample-6.dll", Object.freeze({ byte_size: 486912, sha256: "72e2721672c11fd37d983b05cc2370f612784e4e3218362a0cb4315d08c917fb" })],
  ["swscale-9.dll", Object.freeze({ byte_size: 12748288, sha256: "3d07972cada6ba38c492e92b0f6c025a6835607fe00cdf83afc604c2fbdfe550" })],
]));

export const canonicalPinnedCommonResourceIdentity = Object.freeze(new Map([
  ["../../THIRD_PARTY_NOTICES.md\0THIRD_PARTY_NOTICES.md", Object.freeze({ byte_size: 3318, sha256: "9c194c6b24445d8e9dbe39898b7aa20ee7b6e2a75972dd458c6011f79b47f175" })],
  ["../../licenses/FFmpeg-LGPL-3.0.txt\0licenses/FFmpeg-LGPL-3.0.txt", Object.freeze({ byte_size: 7652, sha256: "97628afebc60f026f5c2b25d7491c46a5c4ee61f693e7cfa07fbd2c03605979b" })],
  ["../../licenses/Spout2-BSD-2-Clause.txt\0licenses/Spout2-BSD-2-Clause.txt", Object.freeze({ byte_size: 2726, sha256: "04514504b4636462450722a84452b6fe5f671d1f74f22d68867256267f3030f6" })],
  ["../../licenses/bcdec_rs-MIT.txt\0licenses/bcdec_rs-MIT.txt", Object.freeze({ byte_size: 1060, sha256: "0194fa871df6d82a26d58f3b6e035bed033afdc413e2a6ac0ae2f15b7becbb20" })],
]));

export const canonicalKnownAsioBridgeSha256 = Object.freeze([
  "73ede85d5a5568f5dcad33990158c3bd755590c6d06c797166ac7041f6d786f0",
  "14057e266192ee5ea2be66d64bf802ba79f6fe686a338002ae85ffcc38a2e853",
  "2008c70b63272e66d1df1406498a1797114c67e96ea118c2b804adb8d80734d8",
  "3d0d52721fd52b6769d95ffe07a99114444423f5be3e566ed16d1f2edb71a516",
]);

const runtimeNames = new Set(canonicalWindowsFfmpegRuntimeDllNames);
const commonResourceIdentities = new Set(
  canonicalCommonBundleResources.map((resource) => resource.source + "\0" + resource.destination),
);
const sha256Pattern = /^[0-9a-f]{64}$/u;

function canonicalPath(path) {
  const normalized = resolve(path).replaceAll("\\", "/");
  return process.platform === "win32" ? normalized.toLocaleLowerCase("en-US") : normalized;
}

function assertExactKeys(value, keys, label) {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error(label + " must be an object.");
  }
  const actual = Object.keys(value).sort();
  const expected = [...keys].sort();
  if (JSON.stringify(actual) !== JSON.stringify(expected)) {
    throw new Error(label + " keys are not exact (expected: " + expected.join(", ") + "; actual: " + actual.join(", ") + ").");
  }
}

function assertPlainFilename(value, label) {
  if (
    typeof value !== "string"
    || value.length === 0
    || value !== value.trim()
    || value.includes("\0")
    || /[\\/:]/u.test(value)
  ) {
    throw new Error(label + " must be an exact plain filename.");
  }
}

function assertSha256(value, label) {
  if (typeof value !== "string" || !sha256Pattern.test(value)) {
    throw new Error(label + " must be an exact lowercase SHA-256 digest.");
  }
}

function assertPositiveInteger(value, label) {
  if (!Number.isSafeInteger(value) || value <= 0) {
    throw new Error(label + " must be a positive safe integer.");
  }
}

function assertContained(root, candidate, label) {
  const realRoot = realpathSync(root);
  const realCandidate = realpathSync(candidate);
  const child = relative(realRoot, realCandidate);
  if (child === "" || child.startsWith("..") || isAbsolute(child)) {
    throw new Error(label + " escapes its intended root: " + candidate);
  }
  return realCandidate;
}

function fileIdentity(stats) {
  return String(stats.dev) + ":" + String(stats.ino) + ":" + String(stats.size) + ":" + String(stats.mtimeNs);
}

const reparseClearanceCache = new Map();

function runWindowsPowerShellInventory(script, environmentName, environmentValue, label) {
  if (process.platform !== "win32") return "";
  const result = spawnSync(
    "powershell.exe",
    ["-NoLogo", "-NoProfile", "-NonInteractive", "-Command", script],
    {
      encoding: "utf8",
      windowsHide: true,
      timeout: 60_000,
      maxBuffer: 16 * 1024 * 1024,
      env: { ...process.env, [environmentName]: environmentValue },
    },
  );
  if (result.error) {
    throw new Error(label + " could not be audited for Windows reparse point attributes: " + String(result.error));
  }
  if (result.status !== 0) {
    throw new Error(label + " reparse point attribute audit failed closed with exit code " + String(result.status) + ".");
  }
  return result.stdout ?? "";
}

export function findWindowsReparsePointPaths(paths, label = "Path list") {
  if (!Array.isArray(paths) || paths.some((path) => typeof path !== "string" || path.length === 0)) {
    throw new Error(label + " must be a non-empty array of absolute path strings.");
  }
  if (process.platform !== "win32") return [];
  const output = runWindowsPowerShellInventory(
    "$ErrorActionPreference='Stop'\n"
      + "$paths=@($env:SYNDOCAL_REPARSE_PATHS | ConvertFrom-Json)\n"
      + "$flagged=@()\n"
      + "foreach($p in $paths){ if(((Get-Item -LiteralPath $p -Force).Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0){ $flagged+=$p } }\n"
      + "[Console]::Out.Write(($flagged -join \"`n\"))\n",
    "SYNDOCAL_REPARSE_PATHS",
    JSON.stringify(paths),
    label,
  );
  return output.split(/\r?\n/u).map((line) => line.trim()).filter(Boolean);
}

export function assertNoWindowsReparsePoints(paths, label) {
  const flagged = findWindowsReparsePointPaths(paths, label);
  if (flagged.length > 0) {
    throw new Error(
      label
        + " carries a Windows reparse point attribute; symlinks, junctions, mount points, and every other reparse tag are rejected: "
        + flagged.join(", "),
    );
  }
}

export function assertDirectoryTreeHasNoReparsePoints(root, label) {
  if (process.platform !== "win32") return;
  const realRoot = realpathSync(assertSafeExternalDirectory(root, label));
  const output = runWindowsPowerShellInventory(
    "$ErrorActionPreference='Stop'\n"
      + "$hits=@(Get-ChildItem -LiteralPath $env:SYNDOCAL_REPARSE_ROOT -Recurse -Force -Attributes ReparsePoint | ForEach-Object { $_.FullName })\n"
      + "[Console]::Out.Write(($hits -join \"`n\"))\n",
    "SYNDOCAL_REPARSE_ROOT",
    realRoot,
    label,
  );
  const hits = output.split(/\r?\n/u).map((line) => line.trim()).filter(Boolean);
  if (hits.length > 0) {
    throw new Error(
      label
        + " contains Windows reparse point entries (symlinks, junctions, mount points, or any other reparse tag are all rejected): "
        + hits.join(", "),
    );
  }
}

function assertEntryHasNoReparseAttribute(candidate, statsIdentity, label) {
  if (process.platform !== "win32") return;
  const key = canonicalPath(candidate);
  if (reparseClearanceCache.get(key) === statsIdentity) return;
  assertNoWindowsReparsePoints([candidate], label);
  reparseClearanceCache.set(key, statsIdentity);
}

export function readVerifiedRegularFile(path, label, { allowedRoots = [], rejectHardLinks = true } = {}) {
  const candidate = resolve(path);
  if (!existsSync(candidate)) throw new Error(label + " is missing: " + candidate);
  const linkStats = lstatSync(candidate, { bigint: true });
  if (!linkStats.isFile() || linkStats.isSymbolicLink()) {
    throw new Error(label + " must be an ordinary regular file, not a symbolic link, reparse point, or directory: " + candidate);
  }
  if (rejectHardLinks && linkStats.nlink !== 1n) {
    throw new Error(label + " must not be a hard-link alias: " + candidate);
  }
  assertEntryHasNoReparseAttribute(candidate, fileIdentity(linkStats), label);
  const realCandidate = realpathSync(candidate);
  if (canonicalPath(candidate) !== canonicalPath(realCandidate)) {
    throw new Error(label + " must not traverse a symbolic-link or reparse-point path: " + candidate);
  }
  if (allowedRoots.length > 0) {
    let contained = false;
    for (const root of allowedRoots) {
      if (!existsSync(root)) continue;
      try {
        assertContained(root, realCandidate, label);
        contained = true;
        break;
      } catch {
        // The final explicit containment error is emitted after every allowed root was tried.
      }
    }
    if (!contained) throw new Error(label + " is outside every intended root: " + candidate);
  }
  const descriptor = openSync(realCandidate, constants.O_RDONLY | (constants.O_NOFOLLOW ?? 0));
  try {
    const before = fstatSync(descriptor, { bigint: true });
    if (!before.isFile() || (rejectHardLinks && before.nlink !== 1n)) {
      throw new Error(label + " changed before it could be read: " + candidate);
    }
    const bytes = readFileSync(descriptor);
    const after = fstatSync(descriptor, { bigint: true });
    if (fileIdentity(before) !== fileIdentity(after)) {
      throw new Error(label + " changed while it was being read: " + candidate);
    }
    const finalLinkStats = lstatSync(candidate, { bigint: true });
    if (
      !finalLinkStats.isFile()
      || finalLinkStats.isSymbolicLink()
      || (rejectHardLinks && finalLinkStats.nlink !== 1n)
      || canonicalPath(realpathSync(candidate)) !== canonicalPath(realCandidate)
    ) {
      throw new Error(label + " path changed while it was being read: " + candidate);
    }
    return { bytes, realPath: realCandidate, identity: fileIdentity(before) };
  } finally {
    closeSync(descriptor);
  }
}

function assertResourcePath(value, label) {
  const resourcePath = typeof value === "string" && value.startsWith("../../") ? value.slice(6) : value;
  if (
    typeof value !== "string"
    || value.length === 0
    || value !== value.trim()
    || value.includes("\0")
    || value.includes("\\")
    || value.includes(":")
    || resourcePath.split("/").some((part) => part === "" || part === "." || part === "..")
  ) {
    throw new Error(label + " must be an exact slash-separated relative path.");
  }
}

function freezeInventory(inventory) {
  for (const runtime of inventory.runtime_dlls) {
    Object.freeze(runtime.pe_identity);
    Object.freeze(runtime);
  }
  for (const resource of inventory.common_resources) Object.freeze(resource);
  Object.freeze(inventory.runtime_dlls);
  Object.freeze(inventory.common_resources);
  Object.freeze(inventory.known_asio_bridge_sha256);
  return Object.freeze(inventory);
}

export function validateWindowsRuntimeInventory(inventory, label = runtimeInventoryRelativePath) {
  assertExactKeys(inventory, ["schema_version", "platform", "runtime_dlls", "common_resources", "known_asio_bridge_sha256"], label);
  if (inventory.schema_version !== 1 || inventory.platform !== "windows-x86_64") {
    throw new Error(label + " must be schema_version 1 for windows-x86_64.");
  }
  if (!Array.isArray(inventory.runtime_dlls) || inventory.runtime_dlls.length !== canonicalWindowsFfmpegRuntimeDllNames.length) {
    throw new Error(label + " runtime_dlls must be the exact seven-entry approved set.");
  }
  const seenRuntimeNames = new Set();
  for (const runtime of inventory.runtime_dlls) {
    assertExactKeys(runtime, ["filename", "byte_size", "sha256", "pe_identity"], label + " runtime DLL");
    assertPlainFilename(runtime.filename, label + " runtime DLL filename");
    if (!runtimeNames.has(runtime.filename) || seenRuntimeNames.has(runtime.filename)) {
      throw new Error(label + " runtime DLL names are not the exact approved set.");
    }
    seenRuntimeNames.add(runtime.filename);
    assertPositiveInteger(runtime.byte_size, label + " runtime DLL byte_size");
    assertSha256(runtime.sha256, label + " runtime DLL SHA-256");
    assertExactKeys(runtime.pe_identity, ["signature", "machine", "characteristics", "optional_magic"], label + " PE identity");
    if (
      runtime.pe_identity.signature !== "PE\0\0"
      || runtime.pe_identity.machine !== 0x8664
      || runtime.pe_identity.characteristics !== 0x222e
      || runtime.pe_identity.optional_magic !== 0x20b
    ) {
      throw new Error(label + " runtime DLL PE identity must be the pinned PE32+ AMD64 DLL identity.");
    }
  }
  if (seenRuntimeNames.size !== canonicalWindowsFfmpegRuntimeDllNames.length) {
    throw new Error(label + " runtime DLL names are incomplete.");
  }
  if (!Array.isArray(inventory.common_resources) || inventory.common_resources.length !== canonicalCommonBundleResources.length) {
    throw new Error(label + " common_resources must be the exact normal-bundle resource set.");
  }
  const seenCommonResources = new Set();
  for (const resource of inventory.common_resources) {
    assertExactKeys(resource, ["source", "destination", "byte_size", "sha256"], label + " common resource");
    assertResourcePath(resource.source, label + " common resource source");
    assertResourcePath(resource.destination, label + " common resource destination");
    const identity = resource.source + "\0" + resource.destination;
    if (!commonResourceIdentities.has(identity) || seenCommonResources.has(identity)) {
      throw new Error(label + " common resource is unapproved or remapped: " + resource.source + " -> " + resource.destination);
    }
    seenCommonResources.add(identity);
    assertPositiveInteger(resource.byte_size, label + " common resource byte_size");
    assertSha256(resource.sha256, label + " common resource SHA-256");
  }
  if (seenCommonResources.size !== canonicalCommonBundleResources.length) {
    throw new Error(label + " common_resources are incomplete.");
  }
  if (!Array.isArray(inventory.known_asio_bridge_sha256) || inventory.known_asio_bridge_sha256.length === 0) {
    throw new Error(label + " must pin one or more known ASIO bridge SHA-256 values.");
  }
  const bridgeHashes = new Set();
  for (const hash of inventory.known_asio_bridge_sha256) {
    assertSha256(hash, label + " known ASIO bridge SHA-256");
    if (bridgeHashes.has(hash)) throw new Error(label + " duplicates a known ASIO bridge SHA-256.");
    bridgeHashes.add(hash);
  }
  return freezeInventory(structuredClone(inventory));
}

export function enforceWindowsRuntimeInventoryAnchors(inventory, label = runtimeInventoryRelativePath) {
  if (!Object.isFrozen(inventory?.runtime_dlls ?? null)) {
    throw new Error(label + " must be structurally validated before its anchored authority is enforced.");
  }
  for (const runtime of inventory.runtime_dlls) {
    const anchored = canonicalPinnedRuntimeIdentity.get(runtime.filename);
    if (!anchored || runtime.byte_size !== anchored.byte_size || runtime.sha256 !== anchored.sha256) {
      throw new Error(
        label
          + " runtime DLL "
          + runtime.filename
          + " contradicts the independently anchored approved byte_size/SHA-256 authority; editing this inventory cannot redefine it.",
      );
    }
  }
  for (const resource of inventory.common_resources) {
    const anchored = canonicalPinnedCommonResourceIdentity.get(resource.source + "\0" + resource.destination);
    if (!anchored || resource.byte_size !== anchored.byte_size || resource.sha256 !== anchored.sha256) {
      throw new Error(
        label
          + " common resource "
          + resource.destination
          + " contradicts the independently anchored approved byte_size/SHA-256 authority; editing this inventory cannot redefine it.",
      );
    }
  }
  if (
    JSON.stringify([...inventory.known_asio_bridge_sha256].sort())
      !== JSON.stringify([...canonicalKnownAsioBridgeSha256].sort())
  ) {
    throw new Error(
      label
        + " known ASIO bridge SHA-256 set is not exactly the independently anchored known-build set; editing this inventory cannot redefine it.",
    );
  }
  return inventory;
}

export function loadWindowsRuntimeInventory({
  workspace = workspaceRoot,
  inventoryPath = resolve(workspace, runtimeInventoryRelativePath),
} = {}) {
  const record = readVerifiedRegularFile(inventoryPath, "Windows runtime inventory", { allowedRoots: [resolve(workspace)] });
  const text = record.bytes.toString("utf8");
  if (!Buffer.from(text, "utf8").equals(record.bytes)) {
    throw new Error("Windows runtime inventory is not valid strict JSON: not decodable as UTF-8");
  }
  const parsed = parseStrictJson(text, "Windows runtime inventory");
  const validated = validateWindowsRuntimeInventory(parsed);
  return enforceWindowsRuntimeInventoryAnchors(validated);
}

export function commonBundleResourceMap(inventory = loadWindowsRuntimeInventory()) {
  return Object.freeze(Object.fromEntries(inventory.common_resources.map((resource) => [resource.source, resource.destination])));
}

export function windowsFfmpegBundleResourceMap(inventory = loadWindowsRuntimeInventory()) {
  return Object.freeze(
    Object.fromEntries(inventory.runtime_dlls.map((runtime) => ["../../target/release/" + runtime.filename, runtime.filename])),
  );
}

export function verifyPinnedRuntimeFile(path, runtime, label, { allowedRoots = [] } = {}) {
  const record = readVerifiedRegularFile(path, label, { allowedRoots });
  if (record.bytes.length !== runtime.byte_size) {
    throw new Error(label + " byte size does not match pinned inventory for " + runtime.filename + ".");
  }
  if (createHash("sha256").update(record.bytes).digest("hex") !== runtime.sha256) {
    throw new Error(label + " SHA-256 does not match pinned inventory for " + runtime.filename + ".");
  }
  if (record.bytes.length < 0x40 || record.bytes.subarray(0, 2).toString("latin1") !== "MZ") {
    throw new Error(label + " is not a DOS/PE executable.");
  }
  const offset = record.bytes.readUInt32LE(0x3c);
  if (offset > record.bytes.length - 26) throw new Error(label + " has an out-of-range PE header.");
  const signature = record.bytes.subarray(offset, offset + 4).toString("latin1");
  const machine = record.bytes.readUInt16LE(offset + 4);
  const characteristics = record.bytes.readUInt16LE(offset + 22);
  const optionalMagic = record.bytes.readUInt16LE(offset + 24);
  if (
    signature !== runtime.pe_identity.signature
    || machine !== runtime.pe_identity.machine
    || characteristics !== runtime.pe_identity.characteristics
    || optionalMagic !== runtime.pe_identity.optional_magic
  ) {
    throw new Error(label + " PE identity does not match pinned inventory for " + runtime.filename + ".");
  }
  return record;
}

export function verifyPinnedCommonResource(path, resource, label, { allowedRoots = [] } = {}) {
  const record = readVerifiedRegularFile(path, label, { allowedRoots });
  if (record.bytes.length !== resource.byte_size) {
    throw new Error(label + " byte size does not match pinned inventory for " + resource.destination + ".");
  }
  if (createHash("sha256").update(record.bytes).digest("hex") !== resource.sha256) {
    throw new Error(label + " SHA-256 does not match pinned inventory for " + resource.destination + ".");
  }
  return record;
}

export function assertSafeExternalDirectory(pathText, label) {
  if (
    typeof pathText !== "string"
    || pathText.length === 0
    || pathText !== pathText.trim()
    || pathText.includes("\0")
    || /[\r\n]/u.test(pathText)
    || /^[\\/]{2}/u.test(pathText)
    || /^\\\\[?.]/u.test(pathText)
    || pathText.split(/[\\/]+/u).some((part) => part === "." || part === "..")
  ) {
    throw new Error(label + " must be an explicit non-UNC, non-traversing local directory path with no whitespace ambiguity.");
  }
  const driveQualified = /^[A-Za-z]:[\\/]/u.test(pathText);
  if (pathText.includes(":") && (!driveQualified || pathText.slice(2).includes(":"))) {
    throw new Error(label + " must not use an alternate data stream or URI-like path.");
  }
  if (!isAbsolute(pathText)) throw new Error(label + " must be an absolute local directory path.");
  const resolved = resolve(pathText);
  if (!existsSync(resolved)) throw new Error(label + " does not exist: " + resolved);
  const stats = lstatSync(resolved, { bigint: true });
  if (!stats.isDirectory() || stats.isSymbolicLink() || canonicalPath(realpathSync(resolved)) !== canonicalPath(resolved)) {
    throw new Error(label + " must be a non-reparse directory.");
  }
  assertEntryHasNoReparseAttribute(resolved, fileIdentity(stats), label);
  return realpathSync(resolved);
}

export function assertExactWindowsTargetTriple(value) {
  if (value === undefined) return null;
  if (
    typeof value !== "string"
    || value.length === 0
    || value !== value.trim()
    || !/^x86_64-pc-windows-msvc$/u.test(value)
  ) {
    throw new Error(
      "TAURI_ENV_TARGET_TRIPLE must be exactly x86_64-pc-windows-msvc; the pinned AMD64 (PE machine 0x8664) runtime inventory rejects aarch64/ARM64 targets and never copies AMD64 DLLs to aarch64.",
    );
  }
  return value;
}

export function isKnownAsioBridgeBytes(bytes, inventory = loadWindowsRuntimeInventory()) {
  return inventory.known_asio_bridge_sha256.includes(createHash("sha256").update(bytes).digest("hex"));
}

export function assertNoBlockedAsioPayload(name, bytes, label, inventory = loadWindowsRuntimeInventory()) {
  const normalized = basename(String(name)).toLocaleLowerCase("en-US");
  if (
    normalized === "syndocal_asio_bridge.dll"
    || normalized === "syndocal-asio-bridge.dll"
    || /asio.*\.dll$/iu.test(normalized)
  ) {
    throw new Error(label + " contains a forbidden ASIO runtime DLL name: " + name);
  }
  if (isKnownAsioBridgeBytes(bytes, inventory)) {
    throw new Error(label + " contains bytes matching a known ASIO bridge build, even though the filename is " + name + ".");
  }
}

export function walkVerifiedFiles(root, label, { inventory = loadWindowsRuntimeInventory() } = {}) {
  const realRoot = assertSafeExternalDirectory(root, label);
  const files = [];
  const visit = (directory) => {
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
      const path = join(directory, entry.name);
      const stats = lstatSync(path, { bigint: true });
      if (stats.isSymbolicLink()) throw new Error(label + " contains a symbolic-link or reparse-point entry: " + path);
      if (stats.isDirectory()) {
        const real = realpathSync(path);
        if (!canonicalPath(real).startsWith(canonicalPath(realRoot) + "/")) {
          throw new Error(label + " directory escapes its extraction root: " + path);
        }
        visit(path);
      } else if (stats.isFile()) {
        const record = readVerifiedRegularFile(path, label + " file", { allowedRoots: [realRoot] });
        assertNoBlockedAsioPayload(entry.name, record.bytes, label + " file", inventory);
        files.push({ path, name: entry.name, record });
      } else {
        throw new Error(label + " contains an unsupported non-file entry: " + path);
      }
    }
  };
  visit(realRoot);
  assertDirectoryTreeHasNoReparsePoints(root, label);
  return files;
}
