import { createHash, randomUUID } from "node:crypto";
import { spawnSync } from "node:child_process";
import {
  chmodSync,
  existsSync,
  lstatSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  realpathSync,
  renameSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { basename, dirname, extname, isAbsolute, join, relative, resolve } from "node:path";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";
import {
  assertDirectoryTreeHasNoReparsePoints,
  assertSafeExternalDirectory,
  readVerifiedRegularFile,
} from "./windows-runtime-inventory.mjs";
import { parseStrictJson } from "./strict-json.mjs";

export const WINDOWS_CANDIDATE_EXTRACTION_METHOD = "repository-owned-deterministic-extractor-v1";
export const WINDOWS_CANDIDATE_EXTRACTION_ROOT = "target/qa/windows-release-candidate";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const defaultWorkspaceRoot = resolve(scriptDir, "../..");

const RC_PRODUCT_VERSION_PATTERN = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)-rc\.(?:[1-9]\d*)$/u;
const MAX_ARCHIVE_ENTRIES = 100_000;
const MAX_ARCHIVE_ENTRY_PATH_LENGTH = 1024;
const MAX_PROCESS_OUTPUT_BYTES = 16 * 1024 * 1024;
const PROCESS_TIMEOUT_MS = 5 * 60 * 1000;

export class WindowsCandidateExtractorUnavailableError extends Error {
  constructor(message) {
    super("[SYNDOCAL_WINDOWS_CANDIDATE_EXTRACTOR_UNAVAILABLE] " + message);
    this.name = "WindowsCandidateExtractorUnavailableError";
    this.code = "SYNDOCAL_WINDOWS_CANDIDATE_EXTRACTOR_UNAVAILABLE";
  }
}

export class WindowsCandidateExtractionError extends Error {
  constructor(message) {
    super("[SYNDOCAL_WINDOWS_CANDIDATE_EXTRACTION_FAILED] " + message);
    this.name = "WindowsCandidateExtractionError";
    this.code = "SYNDOCAL_WINDOWS_CANDIDATE_EXTRACTION_FAILED";
  }
}

function canonicalPath(path) {
  const normalized = resolve(path).replaceAll("\\", "/");
  return process.platform === "win32" ? normalized.toLocaleLowerCase("en-US") : normalized;
}

function pathExists(path) {
  try {
    lstatSync(path);
    return true;
  } catch (error) {
    if (error?.code === "ENOENT") return false;
    throw error;
  }
}

function sha256(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

function assertCandidateProductVersion(value, label = "candidate product_version") {
  if (typeof value !== "string" || !RC_PRODUCT_VERSION_PATTERN.test(value)) {
    throw new Error(label + " must be an explicit release-candidate SemVer (MAJOR.MINOR.PATCH-rc.N); alpha and non-RC versions are rejected.");
  }
  return value;
}

function assertNoAmbiguousPathText(value, label) {
  if (
    typeof value !== "string"
    || value.length === 0
    || value !== value.trim()
    || value.includes("\0")
    || /[\r\n]/u.test(value)
  ) {
    throw new Error(label + " must be a non-empty single-line path without NUL or whitespace ambiguity.");
  }
}

export function assertStrictCandidateRelativePath(value, label) {
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
    throw new Error(label + " must be an exact slash-separated relative path with no traversal or alternate data stream.");
  }
}

function workspaceRelativePath(path, workspace, label) {
  const realWorkspace = assertSafeExternalDirectory(workspace, label + " workspace");
  const candidate = resolve(path);
  const child = relative(realWorkspace, candidate).replaceAll("\\", "/");
  assertStrictCandidateRelativePath(child, label);
  if (canonicalPath(resolve(realWorkspace, child)) !== canonicalPath(candidate)) {
    throw new Error(label + " does not resolve inside the workspace: " + path);
  }
  return child;
}

function assertExistingWorkspaceFile(pathText, workspace, label) {
  assertNoAmbiguousPathText(pathText, label);
  if (!isAbsolute(pathText)) throw new Error(label + " must be an absolute path.");
  const resolved = resolve(pathText);
  const relativePath = workspaceRelativePath(resolved, workspace, label + " workspace-relative path");
  const record = readVerifiedRegularFile(resolved, label, { allowedRoots: [workspace] });
  return Object.freeze({
    path: record.realPath,
    relativePath,
    bytes: record.bytes,
    identity: record.identity,
    sha256: sha256(record.bytes),
  });
}

function expectedInstallerNames(productVersion) {
  return Object.freeze({
    nsis: "Syndocal_" + productVersion + "_x64-setup.exe",
    msi: "Syndocal_" + productVersion + "_x64_ja-JP.msi",
  });
}

function readReleaseEvidenceIdentity({ releaseEvidencePath, productVersion, workspace }) {
  const pathText = releaseEvidencePath ?? join(workspace, "qa", "release", "release-evidence.json");
  const record = assertExistingWorkspaceFile(pathText, workspace, "release evidence manifest");
  const text = record.bytes.toString("utf8");
  if (!Buffer.from(text, "utf8").equals(record.bytes)) throw new Error("release evidence manifest is not UTF-8.");
  const evidence = parseStrictJson(text, "release evidence manifest");
  if (
    typeof evidence !== "object"
    || evidence === null
    || Array.isArray(evidence)
    || evidence.schemaVersion !== 1
    || evidence.productVersion !== productVersion
    || evidence.tag !== "v" + productVersion
    || typeof evidence.commit !== "string"
    || !/^[0-9a-f]{40}$/u.test(evidence.commit)
  ) {
    throw new Error("release evidence manifest must identify the exact RC product, tag, and 40-character commit.");
  }
  return Object.freeze({
    path: record.relativePath,
    sha256: createHash("sha256").update(record.bytes).digest("hex"),
    commit: evidence.commit,
    tag: evidence.tag,
    identity: record.identity,
  });
}

function updaterArtifactMode({ updaterConfig, workspace }) {
  let config = updaterConfig;
  if (config === undefined) {
    const configPath = join(workspace, "app", "src-tauri", "tauri.updater.conf.json");
    const record = readVerifiedRegularFile(configPath, "Tauri updater configuration", { allowedRoots: [workspace] });
    const text = record.bytes.toString("utf8");
    if (!Buffer.from(text, "utf8").equals(record.bytes)) throw new Error("Tauri updater configuration is not UTF-8.");
    config = parseStrictJson(text, "Tauri updater configuration");
  }
  const mode = config?.bundle?.createUpdaterArtifacts;
  if (mode !== true && mode !== "v1Compatible") {
    throw new Error("Tauri v2 updater configuration must set bundle.createUpdaterArtifacts to true or v1Compatible.");
  }
  return mode;
}

export function expectedWindowsCandidateArtifactRelativePaths(productVersion, role, updaterMode = undefined) {
  assertCandidateProductVersion(productVersion);
  const names = expectedInstallerNames(productVersion);
  const nsisUpdaterZip = names.nsis.replace(/\.exe$/u, ".nsis.zip");
  const expected = role === "nsis"
    ? [join("target", "release", "bundle", "nsis", names.nsis)]
    : role === "msi"
      ? [join("target", "release", "bundle", "msi", names.msi)]
      : role === "updater"
        ? [
            join("target", "release", "bundle", "nsis", names.nsis),
            join("target", "release", "bundle", "msi", names.msi),
            join("target", "release", "bundle", "nsis", nsisUpdaterZip),
            join("target", "release", "bundle", "msi", names.msi + ".zip"),
          ]
        : null;
  if (expected === null) throw new Error("unknown Windows candidate artifact role: " + String(role));
  const filtered = role !== "updater" || updaterMode === undefined
    ? expected
    : expected.filter((candidate) => {
      const isZip = candidate.toLocaleLowerCase("en-US").endsWith(".zip");
      return updaterMode === "v1Compatible" ? isZip : !isZip;
    });
  return Object.freeze(filtered);
}

function samePath(left, right) {
  return canonicalPath(left) === canonicalPath(right);
}

function stableDirectoryIdentity(stats) {
  return String(stats.dev) + ":" + String(stats.ino);
}

function assertExpectedArtifactPath(role, pathText, productVersion, workspace, updaterMode) {
  const record = assertExistingWorkspaceFile(pathText, workspace, role + " candidate artifact");
  const expected = expectedWindowsCandidateArtifactRelativePaths(productVersion, role, updaterMode);
  if (!expected.some((candidate) => samePath(resolve(workspace, candidate), resolve(workspace, record.relativePath)))) {
    throw new Error(
      role
        + " candidate artifact must be the exact versioned Tauri output under target/release/bundle (accepted names: "
        + expected.join(", ")
        + "); got "
        + record.relativePath,
    );
  }
  return record;
}

function assertOutputRootPath(outputPath, workspace) {
  assertNoAmbiguousPathText(outputPath, "candidate extraction output root");
  if (!isAbsolute(outputPath)) throw new Error("candidate extraction output root must be an absolute path.");
  const resolved = resolve(outputPath);
  const relativePath = workspaceRelativePath(resolved, workspace, "candidate extraction output root workspace-relative path");
  const expectedPrefix = WINDOWS_CANDIDATE_EXTRACTION_ROOT + "/";
  if (!relativePath.startsWith(expectedPrefix)) {
    throw new Error("candidate extraction output root must remain below " + WINDOWS_CANDIDATE_EXTRACTION_ROOT + ": " + relativePath);
  }
  if (pathExists(resolved)) throw new Error("candidate extraction output root already exists and cannot be overwritten: " + resolved);
  const targetRoot = assertSafeExternalDirectory(join(workspace, "target"), "candidate extraction target root");
  let current = targetRoot;
  const relativeToTarget = relative(targetRoot, resolved).replaceAll("\\", "/");
  assertStrictCandidateRelativePath(relativeToTarget, "candidate extraction output root below target");
  const segments = relativeToTarget.split("/");
  for (const segment of segments.slice(0, -1)) {
    current = join(current, segment);
    if (existsSync(current)) {
      assertSafeExternalDirectory(current, "candidate extraction output parent");
    } else {
      mkdirSync(current);
      assertSafeExternalDirectory(current, "candidate extraction output parent");
    }
  }
  if (pathExists(resolved)) throw new Error("candidate extraction output root already exists and cannot be overwritten: " + resolved);
  return resolved;
}

function assertToolPath(pathText, label) {
  assertNoAmbiguousPathText(pathText, label);
  if (!isAbsolute(pathText)) throw new WindowsCandidateExtractorUnavailableError(label + " must be an absolute executable path.");
  const resolved = resolve(pathText);
  const fileName = basename(resolved).toLocaleLowerCase("en-US");
  if (fileName !== "7z.exe" && fileName !== "7zz.exe" && fileName !== "msiexec.exe") {
    throw new WindowsCandidateExtractorUnavailableError(label + " has an unsupported executable name: " + basename(resolved));
  }
  try {
    readVerifiedRegularFile(resolved, label, { allowedRoots: [dirname(resolved)] });
  } catch (error) {
    throw new WindowsCandidateExtractorUnavailableError(
      label + " is missing or unsafe: " + String(error instanceof Error ? error.message : error),
    );
  }
  return resolved;
}

export function resolveSevenZipTool({
  environment = process.env,
  platform = process.platform,
  explicitPath,
  knownPaths,
} = {}) {
  if (platform !== "win32") {
    throw new WindowsCandidateExtractorUnavailableError("safe NSIS/ZIP extraction requires Windows x64 and a pinned 7-Zip executable.");
  }
  const candidates = [];
  if (explicitPath !== undefined) candidates.push(explicitPath);
  else if (typeof environment.SYNDOCAL_WINDOWS_7ZIP_PATH === "string" && environment.SYNDOCAL_WINDOWS_7ZIP_PATH.length > 0) {
    candidates.push(environment.SYNDOCAL_WINDOWS_7ZIP_PATH);
  }
  if (Array.isArray(knownPaths)) candidates.push(...knownPaths);
  if (candidates.length === 0) {
    for (const root of [environment.ProgramW6432, environment.ProgramFiles, environment["ProgramFiles(x86)"]]) {
      if (typeof root === "string" && root.length > 0) candidates.push(join(root, "7-Zip", "7z.exe"));
    }
  }
  const seen = new Set();
  for (const candidate of candidates) {
    if (typeof candidate !== "string" || candidate.length === 0) continue;
    const key = canonicalPath(candidate);
    if (seen.has(key)) continue;
    seen.add(key);
    if (!existsSync(resolve(candidate))) continue;
    try {
      return assertToolPath(candidate, "7-Zip extractor");
    } catch {
      // A present but unsafe candidate is not an acceptable fallback. Continue
      // only to another explicitly known candidate, then fail closed below.
    }
  }
  throw new WindowsCandidateExtractorUnavailableError(
    "7z.exe/7zz.exe was not found at an explicit trusted path. Install or provision 7-Zip and set SYNDOCAL_WINDOWS_7ZIP_PATH to its absolute path; PATH lookup and Expand-Archive are not accepted.",
  );
}

export function resolveWindowsMsiExec({ environment = process.env, platform = process.platform } = {}) {
  if (platform !== "win32") {
    throw new WindowsCandidateExtractorUnavailableError("safe MSI extraction requires Windows Installer on Windows x64.");
  }
  const systemRoot = typeof environment.SystemRoot === "string" && environment.SystemRoot.length > 0
    ? environment.SystemRoot
    : typeof environment.WINDIR === "string" && environment.WINDIR.length > 0
      ? environment.WINDIR
      : null;
  if (systemRoot === null) {
    throw new WindowsCandidateExtractorUnavailableError("SystemRoot/WINDIR is unavailable; refusing a PATH-resolved msiexec.exe.");
  }
  return assertToolPath(join(systemRoot, "System32", "msiexec.exe"), "Windows Installer msiexec.exe");
}

function runCheckedProcess(executable, args, label, {
  spawn = spawnSync,
  environment = process.env,
  timeout = PROCESS_TIMEOUT_MS,
} = {}) {
  let result;
  try {
    result = spawn(executable, args, {
      encoding: "utf8",
      windowsHide: true,
      timeout,
      maxBuffer: MAX_PROCESS_OUTPUT_BYTES,
      env: environment,
      stdio: ["ignore", "pipe", "pipe"],
    });
  } catch (error) {
    throw new WindowsCandidateExtractionError(label + " could not start: " + String(error instanceof Error ? error.message : error));
  }
  if (result.error) {
    throw new WindowsCandidateExtractionError(label + " could not start: " + String(result.error));
  }
  if (result.status !== 0) {
    const detail = String(result.stderr ?? result.stdout ?? "").trim().slice(0, 2000);
    throw new WindowsCandidateExtractionError(label + " failed with exit code " + String(result.status) + (detail.length > 0 ? ": " + detail : "."));
  }
  return result;
}

function assertArchiveEntryPath(value, label) {
  if (
    typeof value !== "string"
    || value.length === 0
    || value.length > MAX_ARCHIVE_ENTRY_PATH_LENGTH
    || value !== value.trim()
    || value.includes("\0")
    || value.includes("\\")
    || value.includes(":")
    || value.startsWith("/")
    || /^[A-Za-z]:/u.test(value)
    || value.split("/").some((part) => part === "" || part === "." || part === "..")
  ) {
    throw new WindowsCandidateExtractionError(label + " contains an unsafe archive path: " + JSON.stringify(value));
  }
}

function parseSevenZipListing(output, label) {
  const records = [];
  let afterSeparator = false;
  let current = null;
  for (const line of String(output).split(/\r?\n/u)) {
    if (line.trim() === "----------") {
      if (current !== null) records.push(current);
      current = null;
      afterSeparator = true;
      continue;
    }
    if (!afterSeparator) continue;
    const match = /^([^=]+?)\s=\s(.*)$/u.exec(line);
    if (!match) continue;
    const key = match[1].trim();
    const value = match[2];
    if (key === "Path") {
      if (current !== null) records.push(current);
      current = { path: value, folder: false, attributes: "" };
    } else if (current !== null && key === "Folder") {
      current.folder = value === "+";
    } else if (current !== null && key === "Attributes") {
      current.attributes = value;
    }
  }
  if (current !== null) records.push(current);
  if (records.length === 0) throw new WindowsCandidateExtractionError(label + " listing contained no archive entries.");
  if (records.length > MAX_ARCHIVE_ENTRIES) throw new WindowsCandidateExtractionError(label + " contains too many archive entries.");
  const seen = new Set();
  for (const record of records) {
    assertArchiveEntryPath(record.path, label);
    const key = process.platform === "win32" ? record.path.toLocaleLowerCase("en-US") : record.path;
    if (seen.has(key)) throw new WindowsCandidateExtractionError(label + " contains duplicate archive path: " + record.path);
    seen.add(key);
    if (/[lL]/u.test(record.attributes)) {
      throw new WindowsCandidateExtractionError(label + " contains a symbolic-link/reparse archive entry: " + record.path);
    }
  }
  return Object.freeze(records.map((record) => Object.freeze({ ...record })));
}

function safeTreeEntries(root, label) {
  const realRoot = assertSafeExternalDirectory(root, label);
  assertDirectoryTreeHasNoReparsePoints(realRoot, label);
  const files = [];
  const directories = [];
  const visit = (directory) => {
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
      const path = join(directory, entry.name);
      const stats = lstatSync(path, { bigint: true });
      if (stats.isSymbolicLink()) throw new WindowsCandidateExtractionError(label + " contains a symbolic link: " + path);
      if (stats.isDirectory()) {
        assertSafeExternalDirectory(path, label + " directory");
        directories.push(path);
        visit(path);
      } else if (stats.isFile()) {
        const record = readVerifiedRegularFile(path, label + " file", { allowedRoots: [realRoot] });
        files.push({ path, record });
      } else {
        throw new WindowsCandidateExtractionError(label + " contains an unsupported entry: " + path);
      }
    }
  };
  visit(realRoot);
  return { root: realRoot, files, directories };
}

function assertArchiveListingMatchesTree(listed, tree, label) {
  const listedFiles = new Set(listed.filter((entry) => !entry.folder).map((entry) => entry.path));
  const extractedFiles = new Set(tree.files.map(({ path }) => relative(tree.root, path).replaceAll("\\", "/")));
  if (listedFiles.size !== extractedFiles.size || [...listedFiles].some((path) => !extractedFiles.has(path))) {
    throw new WindowsCandidateExtractionError(
      label + " listing does not correspond to the extracted tree (listed files: " + [...listedFiles].sort().join(", ")
        + "; extracted files: " + [...extractedFiles].sort().join(", ") + ").",
    );
  }
}

function findApplicationRoot(root, label) {
  const tree = safeTreeEntries(root, label);
  const candidates = tree.files.filter(({ path }) => basename(path).toLocaleLowerCase("en-US") === "syndocal.exe");
  if (candidates.length !== 1) {
    throw new WindowsCandidateExtractionError(label + " must contain exactly one syndocal.exe; found " + String(candidates.length) + ".");
  }
  const applicationRoot = assertSafeExternalDirectory(dirname(candidates[0].path), label + " application root");
  for (const { path } of tree.files) {
    const child = relative(applicationRoot, path);
    if (child === "" || child.startsWith("..") || isAbsolute(child)) {
      throw new WindowsCandidateExtractionError(label + " contains payload outside the syndocal.exe directory: " + path);
    }
  }
  return applicationRoot;
}

function createFreshExtractionDirectory(destination, label) {
  if (existsSync(destination)) {
    throw new WindowsCandidateExtractionError(
      label + " must not already exist; candidate extraction never reuses even an empty directory.",
    );
  }
  const parent = assertSafeExternalDirectory(dirname(destination), label + " parent");
  if (!samePath(parent, dirname(destination))) {
    throw new WindowsCandidateExtractionError(label + " parent changed before fresh directory creation.");
  }
  mkdirSync(destination, { recursive: false });
  assertSafeExternalDirectory(destination, label);
}

function materializeVerifiedToolInput(sourcePath, sourceRecord, destination, label, removeDirectory) {
  const expectedSha256 = sha256(sourceRecord.bytes);
  const parent = assertSafeExternalDirectory(dirname(destination), label + " tool-input parent");
  const parentIdentity = stableDirectoryIdentity(lstatSync(parent, { bigint: true }));
  const inputDirectory = mkdtempSync(join(parent, ".candidate-tool-input-"));
  assertSafeExternalDirectory(inputDirectory, label + " tool-input directory");
  const inputIdentity = stableDirectoryIdentity(lstatSync(inputDirectory, { bigint: true }));
  const inputName = basename(sourcePath);
  if (inputName.length === 0 || /[\\/:\0\r\n]/u.test(inputName)) {
    throw new WindowsCandidateExtractionError(label + " source file name is unsafe for materialization.");
  }
  const inputPath = join(inputDirectory, inputName);
  writeFileSync(inputPath, sourceRecord.bytes, { flag: "wx", mode: 0o600 });
  const assertExactCopy = (phase) => {
    const record = readVerifiedRegularFile(inputPath, label + " materialized tool input " + phase, { allowedRoots: [inputDirectory] });
    if (sha256(record.bytes) !== expectedSha256) {
      throw new WindowsCandidateExtractionError(label + " materialized tool input SHA-256 changed " + phase + ".");
    }
    return record;
  };
  assertExactCopy("before tool use");
  const cleanup = () => removeOwnedTemporaryDirectory(
    inputDirectory,
    inputIdentity,
    parentIdentity,
    label + " materialized tool input",
    removeDirectory,
  );
  return Object.freeze({ path: inputPath, directory: inputDirectory, sha256: expectedSha256, assertExactCopy, cleanup });
}

function closeMaterializedToolInput(toolInput, label, primaryError) {
  try {
    toolInput.cleanup();
  } catch (cleanupError) {
    const error = new WindowsCandidateExtractionError(
      label
        + " materialized tool input cleanup failed; its revalidated tombstone or stage remains visible: "
        + String(cleanupError instanceof Error ? cleanupError.message : cleanupError),
    );
    error.preserveStaging = true;
    if (primaryError !== undefined) {
      error.cause = primaryError;
      error.message = String(primaryError instanceof Error ? primaryError.message : primaryError) + " Cleanup also failed: " + error.message;
    }
    throw error;
  }
  if (primaryError !== undefined) throw primaryError;
}

function extractArchive({ archivePath, destination, label, sevenZipPath, listArchiveEntries, extractArchive, removeToolInputDirectory }) {
  createFreshExtractionDirectory(destination, label + " destination");
  const sourceRecord = readVerifiedRegularFile(archivePath, label + " source archive", { allowedRoots: [dirname(archivePath)] });
  const sourceSha256 = sha256(sourceRecord.bytes);
  const toolInput = materializeVerifiedToolInput(archivePath, sourceRecord, destination, label + " archive", removeToolInputDirectory);
  let listed;
  let primaryError;
  try {
    listed = listArchiveEntries === undefined
      ? parseSevenZipListing(
          runCheckedProcess(sevenZipPath, ["l", "-slt", "-sccUTF-8", toolInput.path], label + " archive listing").stdout,
          label,
        )
      : listArchiveEntries(toolInput.path, label);
    toolInput.assertExactCopy("after archive listing");
    for (const record of listed) {
      assertArchiveEntryPath(record.path, label + " archive listing");
    }
    if (extractArchive === undefined) {
      runCheckedProcess(
        sevenZipPath,
        ["x", "-y", "-bd", "-aoa", "-sccUTF-8", "-o" + destination, toolInput.path],
        label + " archive extraction",
      );
    } else {
      extractArchive(toolInput.path, destination, label, listed);
    }
    toolInput.assertExactCopy("after archive extraction");
  } catch (error) {
    primaryError = error;
  }
  closeMaterializedToolInput(toolInput, label, primaryError);
  const finalSourceRecord = readVerifiedRegularFile(archivePath, label + " source archive after extraction", {
    allowedRoots: [dirname(archivePath)],
  });
  if (finalSourceRecord.identity !== sourceRecord.identity || sha256(finalSourceRecord.bytes) !== sourceSha256) {
    throw new WindowsCandidateExtractionError(label + " source archive SHA-256 changed during listing/extraction.");
  }
  const tree = safeTreeEntries(destination, label + " extracted tree");
  assertArchiveListingMatchesTree(listed, tree, label);
  return Object.freeze({ listed, tree });
}

function extractMsi({ artifactPath, destination, label, msiexecPath, runMsi, removeToolInputDirectory }) {
  createFreshExtractionDirectory(destination, label + " destination");
  const sourceRecord = readVerifiedRegularFile(artifactPath, label + " source MSI", { allowedRoots: [dirname(artifactPath)] });
  const sourceSha256 = sha256(sourceRecord.bytes);
  const toolInput = materializeVerifiedToolInput(artifactPath, sourceRecord, destination, label + " MSI", removeToolInputDirectory);
  let primaryError;
  try {
    if (runMsi === undefined) {
      runCheckedProcess(
        msiexecPath,
        ["/a", toolInput.path, "/qn", "/norestart", "TARGETDIR=" + destination],
        label + " Windows Installer administrative extraction",
      );
    } else {
      runMsi(toolInput.path, destination, label);
    }
    toolInput.assertExactCopy("after MSI extraction");
  } catch (error) {
    primaryError = error;
  }
  closeMaterializedToolInput(toolInput, label, primaryError);
  const finalSourceRecord = readVerifiedRegularFile(artifactPath, label + " source MSI after extraction", {
    allowedRoots: [dirname(artifactPath)],
  });
  if (finalSourceRecord.identity !== sourceRecord.identity || sha256(finalSourceRecord.bytes) !== sourceSha256) {
    throw new WindowsCandidateExtractionError(label + " source MSI SHA-256 changed during extraction.");
  }
  return safeTreeEntries(destination, label + " extracted tree");
}

function adjacentSignatureRecord(artifactPath, workspace, label) {
  const signaturePath = join(dirname(artifactPath), basename(artifactPath) + ".sig");
  return assertExistingWorkspaceFile(signaturePath, workspace, label + " adjacent signature");
}

export function computeVerifiedCandidateTreeSha256(root, label = "candidate extracted tree") {
  const tree = safeTreeEntries(root, label);
  const rootPath = tree.root;
  const entries = [];
  for (const directory of tree.directories) {
    entries.push({ kind: "D", path: relative(rootPath, directory).replaceAll("\\", "/") });
  }
  for (const { path, record } of tree.files) {
    entries.push({
      kind: "F",
      path: relative(rootPath, path).replaceAll("\\", "/"),
      size: record.bytes.length,
      sha256: createHash("sha256").update(record.bytes).digest("hex"),
    });
  }
  entries.sort((left, right) => left.path.localeCompare(right.path, "en-US") || left.kind.localeCompare(right.kind, "en-US"));
  const digest = createHash("sha256");
  for (const entry of entries) {
    digest.update(entry.kind + "\0" + entry.path + "\0");
    if (entry.kind === "F") digest.update(String(entry.size) + "\0" + entry.sha256 + "\0");
    digest.update("\n");
  }
  return digest.digest("hex");
}

function assertTreeHash(root, expected, label) {
  if (!/^[0-9a-f]{64}$/u.test(expected)) throw new Error(label + " must be a lowercase SHA-256 digest.");
  const actual = computeVerifiedCandidateTreeSha256(root, label);
  if (actual !== expected) throw new Error(label + " hash mismatch: expected " + expected + ", actual " + actual + ".");
  return actual;
}

function findSingleUpdaterInstaller(root, label) {
  const tree = safeTreeEntries(root, label);
  if (tree.files.length !== 1) {
    throw new WindowsCandidateExtractionError(label + " updater ZIP must contain exactly one installer file; found " + String(tree.files.length) + ".");
  }
  const installer = tree.files[0].path;
  const extension = extname(installer).toLocaleLowerCase("en-US");
  if (extension !== ".exe" && extension !== ".msi") {
    throw new WindowsCandidateExtractionError(label + " updater ZIP must contain one .exe or .msi installer.");
  }
  return installer;
}

function relativeArtifactPath(path, workspace, label) {
  return workspaceRelativePath(path, workspace, label);
}

function copyApplicationRoot(applicationRoot, finalRoot, label) {
  if (existsSync(finalRoot)) throw new WindowsCandidateExtractionError(label + " final extraction root already exists: " + finalRoot);
  const finalParent = dirname(finalRoot);
  if (existsSync(finalParent)) assertSafeExternalDirectory(finalParent, label + " final extraction parent");
  else mkdirSync(finalParent, { recursive: false });
  const parentStats = lstatSync(finalParent, { bigint: true });
  const parentIdentity = stableDirectoryIdentity(parentStats);
  if (!parentStats.isDirectory() || parentStats.isSymbolicLink() || !samePath(realpathSync(finalParent), finalParent)) {
    throw new WindowsCandidateExtractionError(label + " final extraction parent is unsafe before publication.");
  }
  if (existsSync(finalRoot)) throw new WindowsCandidateExtractionError(label + " final extraction root appeared before publication: " + finalRoot);
  const currentParentStats = lstatSync(finalParent, { bigint: true });
  if (
    !currentParentStats.isDirectory()
    || currentParentStats.isSymbolicLink()
    || stableDirectoryIdentity(currentParentStats) !== parentIdentity
    || !samePath(realpathSync(finalParent), finalParent)
  ) {
    throw new WindowsCandidateExtractionError(label + " final extraction parent changed before publication.");
  }
  renameSync(applicationRoot, finalRoot);
  assertSafeExternalDirectory(finalRoot, label + " final extraction root");
  return finalRoot;
}

export function removeOwnedTemporaryDirectory(path, identity, parentIdentity, label, removeDirectory = (target) => rmSync(target, { recursive: true, force: true })) {
  if (path === null || !pathExists(path)) return;
  const parent = dirname(path);
  const assertCurrentParent = () => {
    let parentStats;
    try {
      parentStats = lstatSync(parent, { bigint: true });
    } catch (error) {
      throw new WindowsCandidateExtractionError(label + " cleanup could not revalidate its parent: " + String(error instanceof Error ? error.message : error));
    }
    if (
      !parentStats.isDirectory()
      || parentStats.isSymbolicLink()
      || stableDirectoryIdentity(parentStats) !== parentIdentity
      || !samePath(realpathSync(parent), parent)
    ) {
      throw new WindowsCandidateExtractionError(label + " cleanup refused a changed temporary-directory parent: " + parent);
    }
  };
  assertCurrentParent();
  let stats;
  try {
    stats = lstatSync(path, { bigint: true });
  } catch (error) {
    throw new WindowsCandidateExtractionError(label + " cleanup could not revalidate its owner: " + String(error instanceof Error ? error.message : error));
  }
  if (!stats.isDirectory() || stats.isSymbolicLink()) {
    throw new WindowsCandidateExtractionError(label + " cleanup refused a swapped or reparse path: " + path);
  }
  const actualIdentity = stableDirectoryIdentity(stats);
  if (actualIdentity !== identity || !samePath(realpathSync(path), path)) {
    throw new WindowsCandidateExtractionError(label + " cleanup refused a swapped temporary directory: " + path);
  }
  const tombstone = join(parent, "." + basename(path) + ".tombstone-" + randomUUID());
  if (pathExists(tombstone)) throw new WindowsCandidateExtractionError(label + " cleanup tombstone path already exists: " + tombstone);
  // Rename first: after this same-volume sibling move, only the inode we
  // revalidated can be recursively removed. A swapped path is left intact.
  assertCurrentParent();
  const finalStats = lstatSync(path, { bigint: true });
  if (
    !finalStats.isDirectory()
    || finalStats.isSymbolicLink()
    || stableDirectoryIdentity(finalStats) !== identity
    || !samePath(realpathSync(path), path)
  ) {
    throw new WindowsCandidateExtractionError(label + " cleanup refused a swapped temporary directory before tombstone rename: " + path);
  }
  try {
    renameSync(path, tombstone);
  } catch (error) {
    throw new WindowsCandidateExtractionError(label + " cleanup tombstone rename failed: " + String(error instanceof Error ? error.message : error));
  }
  const tombstoneStats = lstatSync(tombstone, { bigint: true });
  if (
    !tombstoneStats.isDirectory()
    || tombstoneStats.isSymbolicLink()
    || stableDirectoryIdentity(tombstoneStats) !== identity
    || !samePath(realpathSync(tombstone), tombstone)
  ) {
    throw new WindowsCandidateExtractionError(label + " cleanup refused a swapped tombstone directory: " + tombstone);
  }
  try {
    removeDirectory(tombstone);
  } catch (error) {
    throw new WindowsCandidateExtractionError(
      label
        + " cleanup failed; the tombstone remains visible at "
        + tombstone
        + ": "
        + String(error instanceof Error ? error.message : error),
    );
  }
}

export function extractWindowsCandidateArtifacts({
  nsisPath,
  msiPath,
  updaterPath,
  productVersion,
  outputRoot,
  workspace = defaultWorkspaceRoot,
  environment = process.env,
  platform = process.platform,
  sevenZipPath,
  msiexecPath,
  listArchiveEntries,
  extractArchive: extractArchiveImpl,
  runMsi,
  removeToolInputDirectory,
  releaseEvidencePath,
  updaterConfig,
  beforePublication,
  now = () => Date.now(),
} = {}) {
  assertCandidateProductVersion(productVersion);
  if (platform !== "win32") {
    throw new WindowsCandidateExtractorUnavailableError("Windows candidate extraction is available only on Windows x64; refusing to substitute a non-Windows archive or MSI extractor.");
  }
  const safeWorkspace = assertSafeExternalDirectory(workspace, "candidate extraction workspace");
  const updaterMode = updaterArtifactMode({ updaterConfig, workspace: safeWorkspace });
  const releaseEvidence = readReleaseEvidenceIdentity({ releaseEvidencePath, productVersion, workspace: safeWorkspace });
  const artifacts = {
    nsis: assertExpectedArtifactPath("nsis", nsisPath, productVersion, safeWorkspace, updaterMode),
    msi: assertExpectedArtifactPath("msi", msiPath, productVersion, safeWorkspace, updaterMode),
    updater: assertExpectedArtifactPath("updater", updaterPath, productVersion, safeWorkspace, updaterMode),
  };
  const updaterSignature = adjacentSignatureRecord(artifacts.updater.path, safeWorkspace, "updater candidate");
  const finalOutputRoot = assertOutputRootPath(outputRoot, safeWorkspace);
  const sevenZip = sevenZipPath ?? (platform === "win32" ? resolveSevenZipTool({ environment, platform }) : null);
  const msiExec = msiexecPath ?? (platform === "win32" ? resolveWindowsMsiExec({ environment, platform }) : null);
  const finalParent = dirname(finalOutputRoot);
  const parentStats = lstatSync(finalParent, { bigint: true });
  const parentIdentity = stableDirectoryIdentity(parentStats);
  const workRoot = mkdtempSync(join(finalParent, ".windows-candidate-stage-"));
  const workStats = lstatSync(workRoot, { bigint: true });
  const workIdentity = stableDirectoryIdentity(workStats);
  const transientRoot = join(workRoot, ".transient");
  mkdirSync(transientRoot);
  const transientIdentity = stableDirectoryIdentity(lstatSync(transientRoot, { bigint: true }));
  const extractedParent = join(workRoot, "extracted");
  mkdirSync(extractedParent);
  const reports = [];
  let published = false;
  try {
    const extractRole = (role, artifactPath, stagedRoleRoot, finalRoleRoot, mode, reportArtifactPath = artifactPath) => {
      const roleWork = join(transientRoot, role);
      mkdirSync(roleWork);
      const rawRoot = join(roleWork, "raw");
      let extraction;
      if (mode === "msi") {
        extraction = extractMsi({ artifactPath, destination: rawRoot, label: role + " candidate", msiexecPath: msiExec, runMsi, removeToolInputDirectory });
      } else {
        extraction = extractArchive({
          archivePath: artifactPath,
          destination: rawRoot,
          label: role + " candidate",
          sevenZipPath: sevenZip,
          listArchiveEntries,
          extractArchive: extractArchiveImpl,
          removeToolInputDirectory,
        });
      }
      const applicationRoot = findApplicationRoot(rawRoot, role + " candidate");
      const stagedRoot = copyApplicationRoot(applicationRoot, stagedRoleRoot, role + " candidate");
      const treeHash = computeVerifiedCandidateTreeSha256(stagedRoot, role + " candidate final tree");
      // Re-read after the move to close the source/destination publication fence.
      assertTreeHash(stagedRoot, treeHash, role + " candidate final tree");
      const currentArtifact = readVerifiedRegularFile(artifacts[role].path, role + " candidate source artifact", {
        allowedRoots: [safeWorkspace],
      });
      if (currentArtifact.identity !== artifacts[role].identity || sha256(currentArtifact.bytes) !== artifacts[role].sha256) {
        throw new WindowsCandidateExtractionError(role + " candidate source artifact SHA-256 changed during extraction.");
      }
      const report = {
        role,
        artifact_path: relativeArtifactPath(reportArtifactPath, safeWorkspace, role + " artifact path"),
        artifact_sha256: artifacts[role].sha256,
        signature_path: role === "updater" ? updaterSignature.relativePath : null,
        signature_sha256: role === "updater" ? updaterSignature.sha256 : null,
        extracted_root: relativeArtifactPath(finalRoleRoot, safeWorkspace, role + " extracted root"),
        extracted_tree_sha256: treeHash,
        method: WINDOWS_CANDIDATE_EXTRACTION_METHOD,
      };
      if (role === "updater") {
        const currentSignature = readVerifiedRegularFile(updaterSignature.path, "updater candidate adjacent signature", {
          allowedRoots: [safeWorkspace],
        });
        if (currentSignature.identity !== updaterSignature.identity || sha256(currentSignature.bytes) !== updaterSignature.sha256) {
          throw new WindowsCandidateExtractionError("updater candidate adjacent signature SHA-256 changed during extraction.");
        }
        report.signature_sha256 = updaterSignature.sha256;
      }
      reports.push(report);
      return extraction;
    };

    extractRole("nsis", artifacts.nsis.path, join(extractedParent, "nsis"), join(finalOutputRoot, "extracted", "nsis"), "archive");
    extractRole("msi", artifacts.msi.path, join(extractedParent, "msi"), join(finalOutputRoot, "extracted", "msi"), "msi");

    const updaterExtension = extname(artifacts.updater.path).toLocaleLowerCase("en-US");
    if (updaterExtension === ".zip") {
      const updaterContainer = join(transientRoot, "updater-container");
      extractArchive({
        archivePath: artifacts.updater.path,
        destination: updaterContainer,
        label: "updater candidate ZIP",
        sevenZipPath: sevenZip,
        listArchiveEntries,
        extractArchive: extractArchiveImpl,
        removeToolInputDirectory,
      });
      const nestedInstaller = findSingleUpdaterInstaller(updaterContainer, "updater candidate ZIP");
      const nestedExtension = extname(nestedInstaller).toLocaleLowerCase("en-US");
      extractRole("updater", nestedInstaller, join(extractedParent, "updater"), join(finalOutputRoot, "extracted", "updater"), nestedExtension === ".msi" ? "msi" : "archive", artifacts.updater.path);
    } else {
      extractRole("updater", artifacts.updater.path, join(extractedParent, "updater"), join(finalOutputRoot, "extracted", "updater"), updaterExtension === ".msi" ? "msi" : "archive");
    }

    const inventory = {
      schema_version: 2,
      product_version: productVersion,
      release_evidence: {
        path: releaseEvidence.path,
        sha256: releaseEvidence.sha256,
        commit: releaseEvidence.commit,
        tag: releaseEvidence.tag,
      },
      extractions: reports,
    };
    const inventoryPath = join(workRoot, "inventory.json");
    const inventoryBytes = Buffer.from(JSON.stringify(inventory, null, 2) + "\n", "utf8");
    writeFileSync(inventoryPath, inventoryBytes, { flag: "wx", mode: 0o444 });
    try {
      chmodSync(inventoryPath, 0o444);
    } catch {
      // Windows ACLs, rather than POSIX mode bits, govern immutability there;
      // the checker re-reads and hashes the inventory and every payload.
    }
    const inventoryRecord = readVerifiedRegularFile(inventoryPath, "generated Windows candidate extraction inventory", {
      allowedRoots: [workRoot],
    });
    if (!inventoryRecord.bytes.equals(inventoryBytes)) throw new WindowsCandidateExtractionError("generated inventory changed before publication.");
    // All extractor/tool copies reside below the private transient stage and
    // must not become part of the published acceptance inventory.
    removeOwnedTemporaryDirectory(transientRoot, transientIdentity, workIdentity, "candidate extraction transient stage");
    if (typeof beforePublication === "function") beforePublication({ stagingRoot: workRoot, outputRoot: finalOutputRoot });
    for (const [role, artifact] of Object.entries(artifacts)) {
      const currentArtifact = readVerifiedRegularFile(artifact.path, role + " candidate source artifact before publication", {
        allowedRoots: [safeWorkspace],
      });
      if (currentArtifact.identity !== artifact.identity || sha256(currentArtifact.bytes) !== artifact.sha256) {
        throw new WindowsCandidateExtractionError(role + " candidate source artifact SHA-256 changed before publication.");
      }
    }
    const currentSignature = readVerifiedRegularFile(updaterSignature.path, "updater candidate adjacent signature before publication", {
      allowedRoots: [safeWorkspace],
    });
    if (currentSignature.identity !== updaterSignature.identity || sha256(currentSignature.bytes) !== updaterSignature.sha256) {
      throw new WindowsCandidateExtractionError("updater candidate adjacent signature SHA-256 changed before publication.");
    }
    const currentEvidence = readVerifiedRegularFile(join(safeWorkspace, releaseEvidence.path), "release evidence manifest before publication", {
      allowedRoots: [safeWorkspace],
    });
    if (currentEvidence.identity !== releaseEvidence.identity || sha256(currentEvidence.bytes) !== releaseEvidence.sha256) {
      throw new WindowsCandidateExtractionError("release evidence manifest changed before publication.");
    }
    const currentParentStats = lstatSync(finalParent, { bigint: true });
    const currentParentIdentity = stableDirectoryIdentity(currentParentStats);
    if (
      !currentParentStats.isDirectory()
      || currentParentStats.isSymbolicLink()
      || currentParentIdentity !== parentIdentity
      || !samePath(realpathSync(finalParent), finalParent)
    ) {
      throw new WindowsCandidateExtractionError("candidate extraction output parent changed before atomic publication.");
    }
    if (pathExists(finalOutputRoot)) throw new WindowsCandidateExtractionError("candidate extraction output root appeared during atomic publication.");
    renameSync(workRoot, finalOutputRoot);
    published = true;
    assertSafeExternalDirectory(finalOutputRoot, "published Windows candidate extraction output root");
    const publishedInventoryPath = join(finalOutputRoot, "inventory.json");
    return Object.freeze({
      outputRoot: finalOutputRoot,
      inventoryPath: publishedInventoryPath,
      inventory,
      generatedAt: now(),
    });
  } catch (error) {
    if (!published && error?.preserveStaging !== true) removeOwnedTemporaryDirectory(workRoot, workIdentity, parentIdentity, "candidate extraction temporary work");
    if (error instanceof WindowsCandidateExtractorUnavailableError || error instanceof WindowsCandidateExtractionError) throw error;
    throw new WindowsCandidateExtractionError(String(error instanceof Error ? error.message : error));
  }
}

function parseCli(argv) {
  if (argv.length === 1 && argv[0] === "--self-test") return { selfTest: true };
  if (
    argv.length === 10
    && argv[0] === "--nsis"
    && argv[2] === "--msi"
    && argv[4] === "--updater"
    && argv[6] === "--product-version"
    && argv[8] === "--output"
  ) {
    return { nsisPath: argv[1], msiPath: argv[3], updaterPath: argv[5], productVersion: argv[7], outputRoot: argv[9] };
  }
  throw new Error(
    "Usage: windows-candidate-extractor.mjs --self-test | --nsis <absolute-exe> --msi <absolute-msi> --updater <absolute-updater-exe-or-zip> --product-version <X.Y.Z-rc.N> --output <absolute-target/qa/windows-release-candidate/...>",
  );
}

function fixturePe(seed) {
  const bytes = Buffer.alloc(128, seed);
  bytes.write("MZ", 0, "latin1");
  bytes.writeUInt32LE(0x40, 0x3c);
  bytes.write("PE\0\0", 0x40, "latin1");
  bytes.writeUInt16LE(0x8664, 0x44);
  bytes.writeUInt16LE(0x222e, 0x56);
  bytes.writeUInt16LE(0x20b, 0x58);
  return bytes;
}

function selfTest() {
  let assertions = 0;
  const pass = (condition, label) => {
    if (!condition) throw new Error(label);
    assertions += 1;
  };
  const rejects = (action, pattern) => {
    let thrown;
    try { action(); } catch (error) { thrown = error; }
    if (!(thrown instanceof Error) || !pattern.test(thrown.message)) throw new Error("self-test expected rejection: " + pattern + "; got " + String(thrown));
    assertions += 1;
  };

  rejects(() => assertCandidateProductVersion("1.2.3-alpha.1"), /release-candidate/);
  rejects(
    () => extractWindowsCandidateArtifacts({ productVersion: "1.2.3-rc.1", platform: "linux" }),
    /available only on Windows x64/,
  );
  rejects(() => resolveSevenZipTool({ platform: "win32", environment: {}, knownPaths: [] }), /7z\.exe\/7zz\.exe/);
  rejects(() => resolveWindowsMsiExec({ platform: "win32", environment: {} }), /SystemRoot\/WINDIR/);
  rejects(() => assertArchiveEntryPath("../escape", "hostile archive"), /unsafe archive path/);
  rejects(() => assertArchiveEntryPath("C:/escape", "hostile archive"), /unsafe archive path/);
  rejects(() => assertArchiveEntryPath("safe\\escape", "hostile archive"), /unsafe archive path/);

  const root = mkdtempSync(join(tmpdir(), "syndocal-windows-candidate-extractor-self-test-"));
  try {
    const workspace = join(root, "workspace");
    mkdirSync(workspace);
    mkdirSync(join(workspace, "target"));
    mkdirSync(join(workspace, "qa", "release"), { recursive: true });
    const productVersion = "1.2.3-rc.1";
    const names = expectedInstallerNames(productVersion);
    const nsisDir = join(workspace, "target", "release", "bundle", "nsis");
    const msiDir = join(workspace, "target", "release", "bundle", "msi");
    mkdirSync(nsisDir, { recursive: true });
    mkdirSync(msiDir, { recursive: true });
    for (const path of [
      join(nsisDir, names.nsis),
      join(msiDir, names.msi),
      join(nsisDir, names.nsis.replace(/\.exe$/u, ".nsis.zip")),
    ]) writeFileSync(path, Buffer.from("synthetic installer " + basename(path), "utf8"), { flag: "wx" });
    writeFileSync(join(nsisDir, names.nsis.replace(/\.exe$/u, ".nsis.zip") + ".sig"), Buffer.from("synthetic updater signature", "utf8"), { flag: "wx" });
    writeFileSync(
      join(workspace, "qa", "release", "release-evidence.json"),
      JSON.stringify({ schemaVersion: 1, productVersion, tag: "v" + productVersion, commit: "a".repeat(40) }) + "\n",
      { flag: "wx" },
    );
    const createPayload = (destination, seed) => {
      mkdirSync(destination, { recursive: true });
      writeFileSync(join(destination, "syndocal.exe"), fixturePe(seed), { flag: "wx" });
      writeFileSync(join(destination, "payload.dat"), Buffer.from([seed, seed + 1]), { flag: "wx" });
    };
    const safeListing = () => [
      { path: "syndocal.exe", folder: false, attributes: "" },
      { path: "payload.dat", folder: false, attributes: "" },
    ];
    const updaterInstallerName = names.nsis;
    const observedToolInputs = [];
    let nestedInstallerSourcePath = null;
    const fakeTools = {
      sevenZipPath: "C:/synthetic/7z.exe",
      msiexecPath: "C:/synthetic/msiexec.exe",
      listArchiveEntries: (source, label) => {
        observedToolInputs.push({ source, label, phase: "listing" });
        return label === "updater candidate ZIP"
          ? [{ path: updaterInstallerName, folder: false, attributes: "" }]
          : safeListing();
      },
      extractArchive: (source, destination, label) => {
        observedToolInputs.push({ source, label, phase: "extraction" });
        if (label === "updater candidate ZIP") {
          mkdirSync(destination, { recursive: true });
          nestedInstallerSourcePath = join(destination, updaterInstallerName);
          writeFileSync(nestedInstallerSourcePath, Buffer.from("nested updater installer", "utf8"), { flag: "wx" });
          return;
        }
        createPayload(destination, 7);
      },
      runMsi: (source, destination) => {
        observedToolInputs.push({ source, label: "msi candidate", phase: "extraction" });
        createPayload(destination, 8);
      },
    };
    const output = join(workspace, WINDOWS_CANDIDATE_EXTRACTION_ROOT, productVersion);
    const result = extractWindowsCandidateArtifacts({
      nsisPath: join(nsisDir, names.nsis),
      msiPath: join(msiDir, names.msi),
      updaterPath: join(nsisDir, names.nsis.replace(/\.exe$/u, ".nsis.zip")),
      productVersion,
      outputRoot: output,
      workspace,
      platform: "win32",
      updaterConfig: { bundle: { createUpdaterArtifacts: "v1Compatible" } },
      ...fakeTools,
    });
    pass(existsSync(result.inventoryPath), "positive extraction writes an inventory");
    pass(result.inventory.extractions.length === 3, "positive extraction records NSIS/MSI/updater roles");
    pass(result.inventory.extractions.some((entry) => entry.role === "nsis") && result.inventory.extractions.some((entry) => entry.role === "msi"), "representative NSIS listing and MSI administrative-tree fixtures are exercised");
    pass(result.inventory.extractions.every((entry) => entry.method === WINDOWS_CANDIDATE_EXTRACTION_METHOD), "positive extraction records the owned method");
    pass(
      readdirSync(output).every((name) => !name.startsWith(".windows-candidate-stage-")),
      "temporary work is not published",
    );
    const rawToolSources = new Set([
      join(nsisDir, names.nsis),
      join(msiDir, names.msi),
      join(nsisDir, names.nsis.replace(/\.exe$/u, ".nsis.zip")),
    ]);
    pass(
      observedToolInputs.length > 0 && observedToolInputs.every(({ source }) => !rawToolSources.has(source)),
      "NSIS, MSI, and outer updater tools receive only owned wx materialized copies, never source artifact paths",
    );
    pass(
      nestedInstallerSourcePath !== null
        && observedToolInputs.filter(({ label }) => label === "updater candidate").every(({ source }) => source !== nestedInstallerSourcePath),
      "nested updater installer extraction receives a second owned materialized copy, never its outer-extraction path",
    );
    pass(
      observedToolInputs.length > 0 && observedToolInputs.every(({ source }) => !existsSync(dirname(source))),
      "every successful tool input copy is tombstoned and removed before candidate publication",
    );

    const updaterSignaturePath = join(nsisDir, names.nsis.replace(/\.exe$/u, ".nsis.zip") + ".sig");
    const updaterSignatureBytes = readVerifiedRegularFile(updaterSignaturePath, "self-test updater signature", { allowedRoots: [workspace] }).bytes;
    rmSync(updaterSignaturePath);
    rejects(
      () => extractWindowsCandidateArtifacts({
        nsisPath: join(nsisDir, names.nsis),
        msiPath: join(msiDir, names.msi),
        updaterPath: join(nsisDir, names.nsis.replace(/\.exe$/u, ".nsis.zip")),
        productVersion,
        outputRoot: join(workspace, WINDOWS_CANDIDATE_EXTRACTION_ROOT, "missing-signature"),
        workspace,
        platform: "win32",
        updaterConfig: { bundle: { createUpdaterArtifacts: "v1Compatible" } },
        ...fakeTools,
      }),
      /adjacent signature/,
    );
    writeFileSync(updaterSignaturePath, updaterSignatureBytes, { flag: "wx" });
    rejects(
      () => extractWindowsCandidateArtifacts({
        nsisPath: join(nsisDir, names.nsis),
        msiPath: join(msiDir, names.msi),
        updaterPath: join(nsisDir, names.nsis.replace(/\.exe$/u, ".nsis.zip")),
        productVersion,
        outputRoot: join(workspace, WINDOWS_CANDIDATE_EXTRACTION_ROOT, "wrong-config"),
        workspace,
        platform: "win32",
        updaterConfig: { bundle: { createUpdaterArtifacts: true } },
        ...fakeTools,
      }),
      /exact versioned Tauri output/,
    );
    const releaseEvidenceFixturePath = join(workspace, "qa", "release", "release-evidence.json");
    const releaseEvidenceFixtureBytes = readVerifiedRegularFile(releaseEvidenceFixturePath, "self-test release evidence", { allowedRoots: [workspace] }).bytes;
    rmSync(releaseEvidenceFixturePath);
    rejects(
      () => extractWindowsCandidateArtifacts({
        nsisPath: join(nsisDir, names.nsis),
        msiPath: join(msiDir, names.msi),
        updaterPath: join(nsisDir, names.nsis.replace(/\.exe$/u, ".nsis.zip")),
        productVersion,
        outputRoot: join(workspace, WINDOWS_CANDIDATE_EXTRACTION_ROOT, "missing-evidence"),
        workspace,
        platform: "win32",
        updaterConfig: { bundle: { createUpdaterArtifacts: "v1Compatible" } },
        ...fakeTools,
      }),
      /release evidence manifest is missing/,
    );
    writeFileSync(releaseEvidenceFixturePath, releaseEvidenceFixtureBytes, { flag: "wx" });

    const hostileOutput = join(workspace, WINDOWS_CANDIDATE_EXTRACTION_ROOT, "hostile");
    rejects(
      () => extractWindowsCandidateArtifacts({
        nsisPath: join(nsisDir, names.nsis),
        msiPath: join(msiDir, names.msi),
        updaterPath: join(nsisDir, names.nsis.replace(/\.exe$/u, ".nsis.zip")),
        productVersion,
        outputRoot: hostileOutput,
        workspace,
        platform: "win32",
        updaterConfig: { bundle: { createUpdaterArtifacts: "v1Compatible" } },
        ...fakeTools,
        listArchiveEntries: () => [{ path: "../escape", folder: false, attributes: "" }],
      }),
      /unsafe archive path/,
    );
    pass(!existsSync(hostileOutput), "hostile archive rejection leaves no published output root");

    const failingOutput = join(workspace, WINDOWS_CANDIDATE_EXTRACTION_ROOT, "failing");
    rejects(
      () => extractWindowsCandidateArtifacts({
        nsisPath: join(nsisDir, names.nsis),
        msiPath: join(msiDir, names.msi),
        updaterPath: join(nsisDir, names.nsis.replace(/\.exe$/u, ".nsis.zip")),
        productVersion,
        outputRoot: failingOutput,
        workspace,
        platform: "win32",
        updaterConfig: { bundle: { createUpdaterArtifacts: "v1Compatible" } },
        ...fakeTools,
        extractArchive: (_source, destination) => {
          createPayload(destination, 9);
          throw new Error("synthetic extractor failure");
        },
      }),
      /synthetic extractor failure/,
    );
    pass(!existsSync(failingOutput), "extractor failure cleans only its owned staging directory");

    const toolCleanupFailureOutput = join(workspace, WINDOWS_CANDIDATE_EXTRACTION_ROOT, "tool-input-cleanup-failure");
    let retainedToolInputTombstone = null;
    rejects(
      () => extractWindowsCandidateArtifacts({
        nsisPath: join(nsisDir, names.nsis),
        msiPath: join(msiDir, names.msi),
        updaterPath: join(nsisDir, names.nsis.replace(/\.exe$/u, ".nsis.zip")),
        productVersion,
        outputRoot: toolCleanupFailureOutput,
        workspace,
        platform: "win32",
        updaterConfig: { bundle: { createUpdaterArtifacts: "v1Compatible" } },
        ...fakeTools,
        removeToolInputDirectory: (tombstone) => {
          retainedToolInputTombstone = tombstone;
          throw new Error("synthetic tool-input cleanup refusal");
        },
      }),
      /materialized tool input cleanup failed; its revalidated tombstone or stage remains visible/,
    );
    pass(
      retainedToolInputTombstone !== null && existsSync(retainedToolInputTombstone) && !existsSync(toolCleanupFailureOutput),
      "tool-input cleanup refusal leaves its tombstone visible and never publishes a candidate output",
    );

    const listingSwapOutput = join(workspace, WINDOWS_CANDIDATE_EXTRACTION_ROOT, "listing-sha-swap");
    const originalNsisBytes = readVerifiedRegularFile(join(nsisDir, names.nsis), "self-test NSIS source", { allowedRoots: [workspace] }).bytes;
    try {
      rejects(
        () => extractWindowsCandidateArtifacts({
          nsisPath: join(nsisDir, names.nsis),
          msiPath: join(msiDir, names.msi),
          updaterPath: join(nsisDir, names.nsis.replace(/\.exe$/u, ".nsis.zip")),
          productVersion,
          outputRoot: listingSwapOutput,
          workspace,
          platform: "win32",
          updaterConfig: { bundle: { createUpdaterArtifacts: "v1Compatible" } },
          ...fakeTools,
          listArchiveEntries: (source, label) => {
            if (label === "nsis candidate") {
              pass(source !== join(nsisDir, names.nsis), "listing hook receives a wx materialized copy instead of the mutable NSIS source path");
              writeFileSync(join(nsisDir, names.nsis), Buffer.from("attacker-modified-archive", "utf8"));
            }
            return safeListing();
          },
        }),
        /source archive SHA-256 changed during listing\/extraction/,
      );
    } finally {
      writeFileSync(join(nsisDir, names.nsis), originalNsisBytes);
    }
    pass(!existsSync(listingSwapOutput), "listing-time artifact SHA-256 swap never publishes a candidate output");

    const nestedSwapOutput = join(workspace, WINDOWS_CANDIDATE_EXTRACTION_ROOT, "nested-source-swap");
    let nestedOriginalPath = null;
    rejects(
      () => extractWindowsCandidateArtifacts({
        nsisPath: join(nsisDir, names.nsis),
        msiPath: join(msiDir, names.msi),
        updaterPath: join(nsisDir, names.nsis.replace(/\.exe$/u, ".nsis.zip")),
        productVersion,
        outputRoot: nestedSwapOutput,
        workspace,
        platform: "win32",
        updaterConfig: { bundle: { createUpdaterArtifacts: "v1Compatible" } },
        ...fakeTools,
        extractArchive: (source, destination, label) => {
          if (label === "updater candidate ZIP") {
            mkdirSync(destination, { recursive: true });
            nestedOriginalPath = join(destination, updaterInstallerName);
            writeFileSync(nestedOriginalPath, Buffer.from("nested updater installer", "utf8"), { flag: "wx" });
            return;
          }
          if (label === "updater candidate") {
            pass(nestedOriginalPath !== null && source !== nestedOriginalPath, "nested installer hook receives a second wx materialized copy instead of the outer extraction path");
            writeFileSync(nestedOriginalPath, Buffer.from("attacker-replaced-nested-installer", "utf8"));
          }
          createPayload(destination, 7);
        },
      }),
      /source archive SHA-256 changed during listing\/extraction/,
    );
    pass(!existsSync(nestedSwapOutput), "nested installer source swap never publishes a candidate output");

    const publicationSwapOutput = join(workspace, WINDOWS_CANDIDATE_EXTRACTION_ROOT, "publication-sha-swap");
    try {
      rejects(
        () => extractWindowsCandidateArtifacts({
          nsisPath: join(nsisDir, names.nsis),
          msiPath: join(msiDir, names.msi),
          updaterPath: join(nsisDir, names.nsis.replace(/\.exe$/u, ".nsis.zip")),
          productVersion,
          outputRoot: publicationSwapOutput,
          workspace,
          platform: "win32",
          updaterConfig: { bundle: { createUpdaterArtifacts: "v1Compatible" } },
          ...fakeTools,
          beforePublication: () => writeFileSync(join(nsisDir, names.nsis), Buffer.from("attacker-modified-before-publish", "utf8")),
        }),
        /source artifact SHA-256 changed before publication/,
      );
    } finally {
      writeFileSync(join(nsisDir, names.nsis), originalNsisBytes);
    }
    pass(!existsSync(publicationSwapOutput), "publication-time artifact SHA-256 swap never publishes a candidate output");

    const cleanupParent = join(root, "cleanup-parent");
    mkdirSync(cleanupParent);
    const cleanupParentIdentity = stableDirectoryIdentity(lstatSync(cleanupParent, { bigint: true }));
    const cleanupOwned = join(cleanupParent, "owned-stage");
    mkdirSync(cleanupOwned);
    const cleanupOwnedIdentity = stableDirectoryIdentity(lstatSync(cleanupOwned, { bigint: true }));
    removeOwnedTemporaryDirectory(cleanupOwned, cleanupOwnedIdentity, cleanupParentIdentity, "self-test owned cleanup");
    pass(!existsSync(cleanupOwned), "owned temporary cleanup uses a sibling tombstone before recursive removal");
    mkdirSync(cleanupOwned);
    const swappedIdentity = stableDirectoryIdentity(lstatSync(cleanupOwned, { bigint: true }));
    renameSync(cleanupOwned, join(cleanupParent, "attacker-retained-stage"));
    mkdirSync(cleanupOwned);
    writeFileSync(join(cleanupOwned, "operator-sentinel.txt"), "keep", { flag: "wx" });
    rejects(
      () => removeOwnedTemporaryDirectory(cleanupOwned, swappedIdentity, cleanupParentIdentity, "self-test swapped cleanup"),
      /swapped temporary directory/,
    );
    pass(existsSync(join(cleanupOwned, "operator-sentinel.txt")), "swapped cleanup path is preserved instead of recursively deleted");

    const sentinelOutput = join(workspace, WINDOWS_CANDIDATE_EXTRACTION_ROOT, "sentinel");
    mkdirSync(sentinelOutput, { recursive: true });
    writeFileSync(join(sentinelOutput, "operator-sentinel.txt"), "keep", { flag: "wx" });
    rejects(
      () => extractWindowsCandidateArtifacts({
        nsisPath: join(nsisDir, names.nsis),
        msiPath: join(msiDir, names.msi),
        updaterPath: join(nsisDir, names.nsis.replace(/\.exe$/u, ".nsis.zip")),
        productVersion,
        outputRoot: sentinelOutput,
        workspace,
        platform: "win32",
        updaterConfig: { bundle: { createUpdaterArtifacts: "v1Compatible" } },
        ...fakeTools,
      }),
      /already exists/,
    );
    pass(existsSync(join(sentinelOutput, "operator-sentinel.txt")), "regular published output sentinel is never recursively deleted");

    const swapOutput = join(workspace, WINDOWS_CANDIDATE_EXTRACTION_ROOT, "swap");
    rejects(
      () => extractWindowsCandidateArtifacts({
        nsisPath: join(nsisDir, names.nsis),
        msiPath: join(msiDir, names.msi),
        updaterPath: join(nsisDir, names.nsis.replace(/\.exe$/u, ".nsis.zip")),
        productVersion,
        outputRoot: swapOutput,
        workspace,
        platform: "win32",
        updaterConfig: { bundle: { createUpdaterArtifacts: "v1Compatible" } },
        ...fakeTools,
        beforePublication: ({ outputRoot }) => {
          mkdirSync(outputRoot, { recursive: true });
          writeFileSync(join(outputRoot, "operator-sentinel.txt"), "keep", { flag: "wx" });
        },
      }),
      /appeared/,
    );
    pass(existsSync(join(swapOutput, "operator-sentinel.txt")), "publication swap sentinel is preserved after atomic publication refusal");

    const reparseOutput = join(workspace, WINDOWS_CANDIDATE_EXTRACTION_ROOT, "reparse-swap");
    let reparseCreated = false;
    try {
      rejects(
        () => extractWindowsCandidateArtifacts({
          nsisPath: join(nsisDir, names.nsis),
          msiPath: join(msiDir, names.msi),
          updaterPath: join(nsisDir, names.nsis.replace(/\.exe$/u, ".nsis.zip")),
          productVersion,
          outputRoot: reparseOutput,
          workspace,
          platform: "win32",
          updaterConfig: { bundle: { createUpdaterArtifacts: "v1Compatible" } },
          ...fakeTools,
          beforePublication: ({ outputRoot }) => symlinkSync(join(workspace, "target"), outputRoot, "junction"),
        }),
        /appeared/,
      );
      reparseCreated = true;
    } catch (error) {
      if (!/appeared/u.test(String(error?.message ?? error)) && !/privilege|access|operation not permitted|EPERM|EACCES/iu.test(String(error?.message ?? error))) throw error;
    }
    if (existsSync(reparseOutput) || (() => { try { return lstatSync(reparseOutput).isSymbolicLink(); } catch { return false; } })()) {
      pass(lstatSync(reparseOutput).isSymbolicLink(), "reparse output swap is observed and never recursively removed");
      rmSync(reparseOutput, { recursive: true, force: true });
    } else if (!reparseCreated) {
      pass(true, "reparse output fixture unavailable without link privilege; regular swap coverage remains active");
    }

    const symlinkOutput = join(workspace, WINDOWS_CANDIDATE_EXTRACTION_ROOT, "symlink");
    let symlinkCreated = false;
    try {
      symlinkSync(join(workspace, "target"), join(workspace, "target", "symlink-fixture"), "junction");
      symlinkCreated = true;
    } catch {
      // Host privileges may deny junction creation. The deterministic hostile
      // archive path cases above remain executed; no privilege failure counts
      // as a pass for this optional Windows-only alias case.
    }
    if (symlinkCreated) {
      rejects(() => assertSafeExternalDirectory(join(workspace, "target", "symlink-fixture"), "symlink fixture"), /non-reparse/);
      rmSync(join(workspace, "target", "symlink-fixture"), { recursive: true, force: true });
      pass(true, "junction fixture is rejected by the shared safe-directory boundary");
    }
    void symlinkOutput;
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
  console.log("Windows candidate extractor self-test passed: " + assertions + " assertions");
}

export { selfTest as runWindowsCandidateExtractorSelfTest };

function main(argv = process.argv.slice(2)) {
  const cli = parseCli(argv);
  if (cli.selfTest) {
    selfTest();
    return;
  }
  const result = extractWindowsCandidateArtifacts(cli);
  console.log("Windows candidate extraction created: " + result.inventoryPath + " method=" + WINDOWS_CANDIDATE_EXTRACTION_METHOD);
}

if (resolve(process.argv[1] ?? "") === fileURLToPath(import.meta.url)) main();
