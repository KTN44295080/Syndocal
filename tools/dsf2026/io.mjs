import { lstat, open, readFile, realpath } from "node:fs/promises";
import { spawnSync } from "node:child_process";
import { dirname, isAbsolute, join, parse as parsePath, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import {
  decodeUtf8Strict,
  fail,
  parseJsonText,
} from "./common.mjs";

export const MAX_BASE_BYTES = 64 * 1024 * 1024;
export const MAX_MANIFEST_BYTES = 16 * 1024 * 1024;
const WINDOWS_SAFE_WRITE_HELPER = fileURLToPath(new URL("./safe-write-win32.ps1", import.meta.url));
const WINDOWS_POWERSHELL_PATH = "C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe";
const WINDOWS_SYSTEM_ROOT = "C:\\Windows";
const WINDOWS_MAX_SAFE_PATH_LENGTH = 259;

function fileIdentity(info, label) {
  const device = info?.dev;
  const fileIndex = info?.ino;
  const valid = [device, fileIndex].every((value) => (typeof value === "number" && Number.isFinite(value)) || typeof value === "bigint");
  if (!valid) {
    fail(`${label} identity is unavailable; refusing an unguarded filesystem operation`);
  }
  return {
    key: `${device}:${fileIndex}`,
    device: String(device),
    fileIndex: String(fileIndex),
  };
}

export function normalizeIdentityPath(path) {
  const absolute = resolve(path).replace(/[\\/]+/g, "\\");
  return process.platform === "win32" ? absolute.toLowerCase() : absolute;
}

async function resolveWindowsPowerShell() {
  const candidate = WINDOWS_POWERSHELL_PATH;
  if (!isAbsolute(candidate)) fail("the pinned Windows PowerShell path is not absolute");
  let candidateInfo;
  try {
    candidateInfo = await lstat(candidate);
  } catch (error) {
    fail(`Windows PowerShell executable cannot be inspected: ${String(error?.message ?? error)}`);
  }
  if (!candidateInfo.isFile()) fail("Windows PowerShell executable is not a regular file");
  let resolved;
  try {
    resolved = await realpath(candidate);
  } catch (error) {
    fail(`Windows PowerShell executable realpath check failed: ${String(error?.message ?? error)}`);
  }
  if (normalizeIdentityPath(resolved) !== normalizeIdentityPath(candidate)) {
    fail("Windows PowerShell executable resolved through a symlink/reparse point; refusing an unverified executable");
  }
  let resolvedInfo;
  try {
    resolvedInfo = await lstat(resolved);
  } catch (error) {
    fail(`Windows PowerShell executable disappeared during identity verification: ${String(error?.message ?? error)}`);
  }
  if (!resolvedInfo.isFile()) fail("resolved Windows PowerShell executable is not a regular file");
  fileIdentity(resolvedInfo, "Windows PowerShell executable");
  return resolved;
}

function assertWindowsOutputBoundary(path) {
  if (process.platform !== "win32") return;
  const absolute = resolve(path);
  const parsed = parsePath(absolute);
  const leafName = parsed.base;
  if (!/^[A-Za-z]:\\$/u.test(parsed.root) || absolute.startsWith("\\\\")) {
    fail("output supports only a local drive path on NTFS; UNC/device paths are rejected");
  }
  if (absolute.length > WINDOWS_MAX_SAFE_PATH_LENGTH) {
    fail(`output path exceeds the supported ${WINDOWS_MAX_SAFE_PATH_LENGTH}-character local NTFS boundary`);
  }
  if (!leafName || leafName === "." || leafName === "..") fail("output leaf must be a non-device path component");
  if (leafName.includes(":")) fail("output leaf must not contain ':' or an alternate data stream");
  if (/[. ]$/u.test(leafName)) fail("output leaf must not end with a dot or space");
  if (leafName.includes("\\") || leafName.includes("/")) fail("output leaf must be a single path component");
  const deviceName = leafName.split(".", 1)[0].toUpperCase();
  if (["CON", "PRN", "AUX", "NUL", "CLOCK$"].includes(deviceName) || /^(?:COM|LPT)[1-9]$/u.test(deviceName)) {
    fail("output leaf must not be a Windows device name");
  }
}

export async function assertSafePath(path, label, allowMissingLeaf = false) {
  const absolute = resolve(path);
  const parsed = parsePath(absolute);
  const rest = absolute.slice(parsed.root.length).split(/[\\/]+/u).filter(Boolean);
  let current = parsed.root;
  for (const [index, component] of rest.entries()) {
    current = join(current, component);
    let info;
    try {
      info = await lstat(current);
    } catch (error) {
      if (error?.code === "ENOENT" && allowMissingLeaf && index === rest.length - 1) return absolute;
      fail(`${label} path component is missing or unreadable: ${current}`);
    }
    if (info.isSymbolicLink()) fail(`${label} path contains a symlink/reparse point: ${current}`);
    try {
      const resolved = await realpath(current);
      if (normalizeIdentityPath(resolved) !== normalizeIdentityPath(current)) {
        fail(`${label} path contains a symlink/reparse point: ${current}`);
      }
    } catch (error) {
      fail(`${label} realpath check failed: ${String(error?.message ?? error)}`);
    }
  }
  return absolute;
}

export async function requireRegularFile(path, label, maxBytes) {
  const absolute = await assertSafePath(path, label, false);
  let info;
  try {
    info = await lstat(absolute);
  } catch (error) {
    fail(`${label} cannot be read: ${String(error?.message ?? error)}`);
  }
  if (!info.isFile()) fail(`${label} must be a regular file`);
  if (info.size > maxBytes) fail(`${label} exceeds the bounded size of ${maxBytes} bytes`);
  return { absolute, info };
}

export async function readStrictJsonFile(path, label, maxBytes) {
  const file = await requireRegularFile(path, label, maxBytes);
  let bytes;
  try {
    bytes = await readFile(file.absolute);
  } catch (error) {
    fail(`${label} cannot be read: ${String(error?.message ?? error)}`);
  }
  if (bytes.byteLength > maxBytes) fail(`${label} exceeded the bounded size while reading`);
  let after;
  try {
    after = await lstat(file.absolute);
  } catch (error) {
    fail(`${label} changed or disappeared while reading: ${String(error?.message ?? error)}`);
  }
  if (after.size !== file.info.size || after.mtimeMs !== file.info.mtimeMs) fail(`${label} changed while being read; refusing an unstable input`);
  return { ...file, bytes, value: parseJsonText(decodeUtf8Strict(bytes, label), label) };
}

export async function prepareOutputPath(path, options = {}) {
  assertWindowsOutputBoundary(path);
  const absolute = await assertSafePath(path, "output", true);
  let info;
  try {
    info = await lstat(absolute);
  } catch (error) {
    if (error?.code !== "ENOENT") fail(`output cannot be inspected: ${String(error?.message ?? error)}`);
  }
  if (info) fail("output already exists; refusing to overwrite it");
  const parent = dirname(absolute);
  const parentInfo = await lstat(parent).catch((error) => fail(`output parent cannot be inspected: ${String(error?.message ?? error)}`));
  if (!parentInfo.isDirectory()) fail("output parent must be a regular directory");
  let canonicalParent;
  try {
    canonicalParent = await realpath(parent);
  } catch (error) {
    fail(`output parent realpath check failed: ${String(error?.message ?? error)}`);
  }
  if (normalizeIdentityPath(canonicalParent) !== normalizeIdentityPath(parent)) {
    fail("output parent changed to a symlink/reparse point while it was being prepared");
  }
  const canonicalOutput = join(canonicalParent, parsePath(absolute).base);
  let canonicalInfo;
  try {
    canonicalInfo = await lstat(canonicalOutput);
  } catch (error) {
    if (error?.code !== "ENOENT") fail(`output cannot be inspected: ${String(error?.message ?? error)}`);
  }
  if (canonicalInfo) fail("output already exists; refusing to overwrite it");
  const canonicalParentInfo = await lstat(canonicalParent, { bigint: true }).catch((error) => fail(`output parent cannot be rechecked: ${String(error?.message ?? error)}`));
  const guard = options.guard;
  if (guard && typeof guard === "object") {
    const identity = fileIdentity(canonicalParentInfo, "output parent");
    guard.parentPath = canonicalParent;
    guard.parentIdentity = identity.key;
    guard.parentVolumeSerial = identity.device;
    guard.parentFileIndex = identity.fileIndex;
  }
  return canonicalOutput;
}

function assertOutputGuard(guard) {
  if (!guard || typeof guard !== "object" || typeof guard.parentPath !== "string" || typeof guard.parentIdentity !== "string") {
    fail("output write requires the parent identity guard returned by prepareOutputPath");
  }
}

async function assertGuardedExclusivePath(path, guard) {
  assertOutputGuard(guard);
  assertWindowsOutputBoundary(path);
  const absolute = await assertSafePath(path, "output", true);
  if (normalizeIdentityPath(absolute) !== normalizeIdentityPath(path)) fail("output path changed while being prepared");
  const parent = dirname(absolute);
  let canonicalParent;
  try {
    canonicalParent = await realpath(parent);
  } catch (error) {
    fail(`output parent changed or disappeared before write: ${String(error?.message ?? error)}`);
  }
  if (normalizeIdentityPath(canonicalParent) !== normalizeIdentityPath(parent)) {
    fail("output parent changed to a symlink/reparse point before write");
  }
  if (normalizeIdentityPath(parent) !== normalizeIdentityPath(guard.parentPath)) {
    fail("output parent changed before write");
  }
  const currentParentInfo = await lstat(parent, { bigint: true }).catch((error) => fail(`output parent cannot be rechecked before write: ${String(error?.message ?? error)}`));
  if (fileIdentity(currentParentInfo, "output parent").key !== guard.parentIdentity) {
    fail("output parent identity changed before write; refusing a redirected creation");
  }
  return absolute;
}

/**
 * Revalidate a deterministic, existing-parent sibling before inspecting or
 * exclusively creating it.  The caller must use the same guard returned for
 * the candidate .sdc; this deliberately prevents a sidecar from acquiring a
 * second parent authority after the candidate path has been accepted.
 */
export async function prepareGuardedSiblingPath(path, guard) {
  return assertGuardedExclusivePath(path, guard);
}

function assertReadableLeaf(path, label) {
  assertWindowsOutputBoundary(path);
  const absolute = resolve(path);
  const leafName = parsePath(absolute).base;
  if (!leafName) fail(`${label} leaf is missing`);
  return { absolute, leafName };
}

async function prepareReadGuard(path, label, existingGuard = null) {
  const { absolute, leafName } = assertReadableLeaf(path, label);
  if (existingGuard) {
    assertOutputGuard(existingGuard);
    if (normalizeIdentityPath(dirname(absolute)) !== normalizeIdentityPath(existingGuard.parentPath)) {
      fail(`${label} parent differs from its guarded publication`);
    }
    // Existing sidecars intentionally get no path lstat/readFile preflight on
    // Windows.  Their only authoritative observation is the held-parent
    // OBJ_DONT_REPARSE open in the native helper below.
    return { absolute, leafName, guard: existingGuard };
  }

  // This lexical inspection is deliberately only a preflight.  The native
  // helper below opens the parent and leaf relative to a held parent handle,
  // with OBJ_DONT_REPARSE and a second lexical identity comparison, so an
  // attacker cannot turn this preflight into the authority for the read.
  const checked = await assertSafePath(absolute, label, false);
  const parentPath = dirname(checked);
  const parentInfo = await lstat(parentPath, { bigint: true }).catch((error) => fail(`${label} parent cannot be inspected: ${String(error?.message ?? error)}`));
  if (!parentInfo.isDirectory()) fail(`${label} parent must be a regular directory`);
  const canonicalParent = await realpath(parentPath).catch((error) => fail(`${label} parent realpath check failed: ${String(error?.message ?? error)}`));
  if (normalizeIdentityPath(canonicalParent) !== normalizeIdentityPath(parentPath)) {
    fail(`${label} parent contains a symlink/reparse point`);
  }
  const identity = fileIdentity(parentInfo, `${label} parent`);
  return {
    absolute: checked,
    leafName,
    guard: {
      parentPath: canonicalParent,
      parentIdentity: identity.key,
      parentVolumeSerial: identity.device,
      parentFileIndex: identity.fileIndex,
    },
  };
}

function nativeHelperResult(result, label, allowMissing) {
  if (!result.error && result.status === 0) return Buffer.from(result.stdout ?? []);
  if (!result.error && result.status === 3 && allowMissing) return null;
  const detail = result.error?.message ?? result.stderr?.toString("utf8").trim() ?? `exit ${result.status}`;
  fail(`${label} native no-reparse read failed: ${detail}`);
}

/**
 * Read a regular file through a held Windows parent handle.  The helper opens
 * the leaf relative to that parent with OBJ_DONT_REPARSE, holds it with no
 * write/delete sharing, and compares the open-handle identity to the lexical
 * parent/leaf before and after the exact read.  It is consequently safe for
 * both untrusted reference inputs and existing deterministic sidecars.
 */
export async function readRegularFileBytesNoReparse(path, label, maxBytes, options = {}) {
  if (!Number.isSafeInteger(maxBytes) || maxBytes < 0) fail(`${label} maximum byte count is invalid`);
  const prepared = await prepareReadGuard(path, label, options.guard ?? null);
  await options.beforeNativeOpen?.({ absolute: prepared.absolute, guard: { ...prepared.guard } });
  if (process.platform === "win32") {
    const powershellPath = await resolveWindowsPowerShell();
    const helperArgs = [
      "-NoProfile",
      "-NonInteractive",
      "-ExecutionPolicy",
      "Bypass",
      "-File",
      WINDOWS_SAFE_WRITE_HELPER,
      "-Operation",
      "read",
      "-ParentPath",
      prepared.guard.parentPath,
      "-LeafName",
      prepared.leafName,
      "-ExpectedVolumeSerial",
      prepared.guard.parentVolumeSerial,
      "-ExpectedFileIndex",
      prepared.guard.parentFileIndex,
      "-MaximumBytes",
      String(maxBytes),
    ];
    if (options.allowMissing === true) helperArgs.push("-AllowMissing");
    const result = spawnSync(powershellPath, helperArgs, {
      encoding: null,
      maxBuffer: maxBytes + (1024 * 1024),
      env: {
        ...process.env,
        SystemRoot: WINDOWS_SYSTEM_ROOT,
        windir: WINDOWS_SYSTEM_ROOT,
      },
      windowsHide: true,
    });
    const bytes = nativeHelperResult(result, label, options.allowMissing === true);
    if (bytes !== null && bytes.byteLength > maxBytes) fail(`${label} exceeded the bounded size while reading`);
    return bytes === null ? null : { absolute: prepared.absolute, bytes };
  }

  let handle;
  try {
    if (options.guard) await assertGuardedExclusivePath(prepared.absolute, options.guard);
    handle = await open(prepared.absolute, "r");
  } catch (error) {
    if (options.allowMissing === true && error?.code === "ENOENT") return null;
    fail(`${label} cannot be read: ${String(error?.message ?? error)}`);
  }
  try {
    const before = await handle.stat({ bigint: true });
    const lexicalBefore = await lstat(prepared.absolute, { bigint: true });
    if (!before.isFile() || !lexicalBefore.isFile() || before.dev !== lexicalBefore.dev || before.ino !== lexicalBefore.ino || before.size !== lexicalBefore.size) {
      fail(`${label} changed or is not a regular file while being read`);
    }
    if (before.size > BigInt(maxBytes)) fail(`${label} exceeds the bounded size of ${maxBytes} bytes`);
    const bytes = Buffer.alloc(Number(before.size));
    let offset = 0;
    while (offset < bytes.byteLength) {
      const { bytesRead } = await handle.read(bytes, offset, bytes.byteLength - offset, offset);
      if (bytesRead === 0) fail(`${label} changed or became truncated while it was being read`);
      offset += bytesRead;
    }
    const after = await handle.stat({ bigint: true });
    const lexicalAfter = await lstat(prepared.absolute, { bigint: true });
    if (after.dev !== before.dev || after.ino !== before.ino || after.size !== before.size || lexicalAfter.dev !== before.dev || lexicalAfter.ino !== before.ino || lexicalAfter.size !== before.size) {
      fail(`${label} changed while it was being read; refusing an unstable input`);
    }
    return { absolute: prepared.absolute, bytes };
  } finally {
    await handle?.close().catch(() => {});
  }
}

function normalizePublicationFences(fences, parentPath) {
  if (fences == null) return [];
  if (!Array.isArray(fences)) fail("output publication fences must be an array");
  const leaves = new Set();
  return fences.map((fence) => {
    if (!fence || typeof fence !== "object" || typeof fence.path !== "string" || !Number.isSafeInteger(fence.byteSize) || fence.byteSize < 0 || typeof fence.sha256 !== "string" || !/^[a-f0-9]{64}$/iu.test(fence.sha256)) {
      fail("output publication fence is invalid");
    }
    const { absolute, leafName } = assertReadableLeaf(fence.path, "output publication fence");
    if (normalizeIdentityPath(dirname(absolute)) !== normalizeIdentityPath(parentPath)) fail("output publication fence is outside the guarded parent");
    const key = leafName.toLowerCase();
    if (leaves.has(key)) fail("output publication fences contain a duplicate leaf");
    leaves.add(key);
    return { leafName, byteSize: fence.byteSize, sha256: fence.sha256.toLowerCase() };
  });
}

/**
 * Native exclusive publication for arbitrary bytes.  The Windows helper reads
 * stdin as raw bytes and creates the child beneath the held parent handle, so
 * text and binary sidecars share the same reparse/rename boundary.
 */
export async function writeExclusiveBytes(path, bytes, guard, options = {}) {
  if (!(bytes instanceof Uint8Array)) fail("output bytes must be a Uint8Array");
  const payload = Buffer.from(bytes);
  const absolute = await assertGuardedExclusivePath(path, guard);
  const parent = dirname(absolute);
  const fences = normalizePublicationFences(options.fences, parent);
  if (fences.some(({ leafName }) => leafName.toLowerCase() === parsePath(absolute).base.toLowerCase())) {
    fail("output publication fence cannot name the output leaf");
  }
  if (process.platform === "win32") {
    const powershellPath = await resolveWindowsPowerShell();
    const helperArgs = [
      "-NoProfile",
      "-NonInteractive",
      "-ExecutionPolicy",
      "Bypass",
      "-File",
      WINDOWS_SAFE_WRITE_HELPER,
      "-ParentPath",
      parent,
      "-LeafName",
      parsePath(absolute).base,
      "-ExpectedVolumeSerial",
      guard.parentVolumeSerial,
      "-ExpectedFileIndex",
      guard.parentFileIndex,
    ];
    if (fences.length > 0) {
      helperArgs.push(
        "-FenceLeafNames", fences.map(({ leafName }) => leafName).join("|"),
        "-FenceByteLengths", fences.map(({ byteSize }) => String(byteSize)).join("|"),
        "-FenceSha256", fences.map(({ sha256 }) => sha256).join("|"),
      );
    }
    const result = spawnSync(powershellPath, helperArgs, {
      input: payload,
      encoding: "utf8",
      env: {
        ...process.env,
        SystemRoot: WINDOWS_SYSTEM_ROOT,
        windir: WINDOWS_SYSTEM_ROOT,
      },
      windowsHide: true,
    });
    if (result.error || result.status !== 0) {
      const detail = result.error?.message ?? result.stderr?.trim() ?? `exit ${result.status}`;
      fail(`output native exclusive write failed without overwrite permission: ${detail}`);
    }
    return;
  }
  let handle;
  try {
    handle = await open(absolute, "wx");
    await handle.writeFile(payload);
    await handle.sync();
  } catch (error) {
    fail(`output write failed without overwrite permission: ${String(error?.message ?? error)}`);
  } finally {
    await handle?.close().catch(() => {});
  }
}

export async function writeExclusive(path, text, guard, options = {}) {
  if (typeof text !== "string") fail("output text must be a string");
  await writeExclusiveBytes(path, Buffer.from(text, "utf8"), guard, options);
}

export function parseCliArgs(argv) {
  const usage = "usage: node tools/author-dsf2026-show.mjs --base <existing imported .sdc> --manifest <manifest.json> --output <new .sdc> [--reference-audio-jinsei-over <song.mp3> --reference-audio-madow-hoshi <song.mp3>]";
  if (argv.length < 6 || argv.length % 2 !== 0) fail(usage);
  const options = {};
  const allowedFlags = new Set([
    "--base",
    "--manifest",
    "--output",
    "--reference-audio-jinsei-over",
    "--reference-audio-madow-hoshi",
  ]);
  for (let index = 0; index < argv.length; index += 2) {
    const flag = argv[index];
    const value = argv[index + 1];
    if (!allowedFlags.has(flag) || !value || value.startsWith("--") || options[flag]) {
      fail(usage);
    }
    options[flag] = value;
  }
  if (!options["--base"] || !options["--manifest"] || !options["--output"]) fail("base, manifest, and output are all required");
  const jinseiOver = options["--reference-audio-jinsei-over"];
  const madowHoshi = options["--reference-audio-madow-hoshi"];
  if ((jinseiOver && !madowHoshi) || (!jinseiOver && madowHoshi)) fail("both reference audio paths are required when reference audio authoring is enabled");
  return {
    base: options["--base"],
    manifest: options["--manifest"],
    output: options["--output"],
    referenceAudio: jinseiOver && madowHoshi
      ? { jinseiOverPath: jinseiOver, madowHoshiPath: madowHoshi }
      : null,
  };
}
