import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { createHash, generateKeyPairSync, sign as signMessage } from "node:crypto";
import { cpSync, lstatSync, mkdtempSync, mkdirSync, readdirSync, realpathSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { basename, dirname, isAbsolute, join, relative, resolve } from "node:path";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";
import {
  assertNoBlockedAsioPayload,
  assertSafeExternalDirectory,
  loadWindowsRuntimeInventory,
  readVerifiedRegularFile,
  validateWindowsRuntimeInventory,
  verifyPinnedCommonResource,
  verifyPinnedRuntimeFile,
  walkVerifiedFiles,
} from "./windows-runtime-inventory.mjs";
import { parseStrictJson } from "./strict-json.mjs";
import {
  computeVerifiedCandidateTreeSha256,
  expectedWindowsCandidateArtifactRelativePaths,
  runWindowsCandidateExtractorSelfTest,
  WINDOWS_CANDIDATE_EXTRACTION_METHOD,
  WINDOWS_CANDIDATE_EXTRACTION_ROOT,
} from "./windows-candidate-extractor.mjs";
import { parseRuntimeUpdaterIdentity } from "./check-release-metadata.mjs";
import {
  canonicalDecodedUpdaterUrlPath,
  parseTauriMinisignPublicKey,
  verifyTauriUpdaterPayload,
} from "./updater-evidence-crypto.mjs";
import {
  runVerifiedMaterializationSelfTest,
  withMaterializedVerifiedExecutable,
} from "./verified-materialization.mjs";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const workspaceRoot = resolve(scriptDir, "../..");

export class CandidateArtifactExtractionRequiredError extends Error {
  constructor(message) {
    super("[SYNDOCAL_WINDOWS_ARTIFACT_EXTRACTION_REQUIRED] " + message);
    this.name = "CandidateArtifactExtractionRequiredError";
    this.code = "SYNDOCAL_WINDOWS_ARTIFACT_EXTRACTION_REQUIRED";
  }
}

export class WindowsExeVersionInspectorUnavailableError extends Error {
  constructor(message) {
    super("[SYNDOCAL_WINDOWS_EXE_VERSION_INSPECTOR_UNAVAILABLE] " + message);
    this.name = "WindowsExeVersionInspectorUnavailableError";
    this.code = "SYNDOCAL_WINDOWS_EXE_VERSION_INSPECTOR_UNAVAILABLE";
  }
}

function assertStrictRelativePath(value, label) {
  if (
    typeof value !== "string"
    || value.length === 0
    || value !== value.trim()
    || value.includes("\0")
    || value.includes("\\")
    || value.includes(":")
    || value.split("/").some((part) => part === "" || part === "." || part === "..")
  ) {
    throw new Error(label + " must be an exact slash-separated relative path.");
  }
}

function slashRelativePath(root, path, label) {
  const pathFromRoot = relative(root, path).replaceAll("\\", "/");
  assertStrictRelativePath(pathFromRoot, label);
  return pathFromRoot;
}

function approvedArtifactFiles(inventory) {
  const approved = new Map([["syndocal.exe", Object.freeze({ kind: "executable" })]]);
  for (const runtime of inventory.runtime_dlls) {
    approved.set(runtime.filename, Object.freeze({ kind: "runtime", runtime }));
  }
  for (const resource of inventory.common_resources) {
    approved.set(resource.destination, Object.freeze({ kind: "common-resource", resource }));
  }
  return approved;
}

function approvedArtifactDirectories(inventory) {
  const directories = new Set();
  for (const resource of inventory.common_resources) {
    const segments = resource.destination.split("/");
    for (let index = 1; index < segments.length; index += 1) {
      directories.add(segments.slice(0, index).join("/"));
    }
  }
  return directories;
}

function extractedArtifactDirectories(root, label) {
  const directories = new Set();
  const visit = (directory) => {
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
      const path = join(directory, entry.name);
      const stats = lstatSync(path, { bigint: true });
      if (stats.isSymbolicLink()) {
        throw new Error(label + " contains a symbolic-link or reparse-point directory: " + path);
      }
      if (!stats.isDirectory()) continue;
      const relativeDirectory = slashRelativePath(root, path, label + " directory path");
      directories.add(relativeDirectory);
      visit(path);
    }
  };
  visit(root);
  return directories;
}

function assertSyndocalExecutable(path, label, allowedRoots) {
  const record = readVerifiedRegularFile(path, label, { allowedRoots });
  const bytes = record.bytes;
  if (bytes.length < 0x40 || bytes.subarray(0, 2).toString("latin1") !== "MZ") {
    throw new Error(label + " is not a Windows PE executable.");
  }
  const offset = bytes.readUInt32LE(0x3c);
  if (offset > bytes.length - 26) throw new Error(label + " has an out-of-range PE header.");
  if (
    bytes.subarray(offset, offset + 4).toString("latin1") !== "PE\0\0"
    || bytes.readUInt16LE(offset + 4) !== 0x8664
    || bytes.readUInt16LE(offset + 24) !== 0x20b
    || (bytes.readUInt16LE(offset + 22) & 0x0002) !== 0x0002
  ) {
    throw new Error(label + " must be an AMD64 PE32+ executable image.");
  }
  return record;
}

const CANONICAL_PRODUCT_SEMVER_PATTERN =
  /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-(?:(?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*)(?:\.(?:0|[1-9]\d*|\d*[A-Za-z-][0-9A-Za-z-]*))*))?$/u;

function parseCanonicalProductSemVer(value, label) {
  if (typeof value !== "string" || !CANONICAL_PRODUCT_SEMVER_PATTERN.test(value)) {
    throw new Error(
      label
        + " must be an explicit canonical SemVer (MAJOR.MINOR.PATCH[-PRERELEASE], no leading zeros, no build metadata); syndocal.exe is never accepted on PE shape alone.",
    );
  }
  return value;
}

function parseExecutableSha256Authority(value, label) {
  if (typeof value !== "string" || !/^[0-9a-f]{64}$/u.test(value)) {
    throw new Error(label + " must be an exact lowercase 64-character hex SHA-256 digest.");
  }
  return value;
}

export function parseWindowsReleaseExeAuthority(value, label = "manual three-root Windows release executable authority") {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error(label + " must be an object with exactly product_version and executable_sha256.");
  }
  exactKeys(value, ["executable_sha256", "product_version"], label);
  return Object.freeze({
    productVersion: parseCanonicalProductSemVer(value.product_version, label + " product_version"),
    executableSha256: parseExecutableSha256Authority(value.executable_sha256, label + " executable_sha256"),
  });
}

const WINDOWS_VERSION_INFO_FIELD_PATTERN = /^[\x20-\x7E]+$/u;

export function validateInspectedWindowsVersionInfo(value, label = "inspected Windows VersionInfo") {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error(label + " must be an object with productVersion and fileVersion strings.");
  }
  exactKeys(value, ["fileVersion", "productVersion"], label);
  for (const field of ["productVersion", "fileVersion"]) {
    const text = value[field];
    if (
      typeof text !== "string"
      || text.length === 0
      || text !== text.trim()
      || !WINDOWS_VERSION_INFO_FIELD_PATTERN.test(text)
    ) {
      throw new Error(label + " " + field + " must be a non-empty single-line printable Windows VersionInfo string.");
    }
  }
  return Object.freeze({ productVersion: value.productVersion, fileVersion: value.fileVersion });
}

function parseWindowsVersionInfoDiagnosticOutput(output) {
  const lines = String(output).trim().split(/\r?\n/u);
  if (lines.length !== 2) {
    throw new Error("Windows VersionInfo diagnostic must emit exactly the ProductVersion and FileVersion lines.");
  }
  return validateInspectedWindowsVersionInfo({ productVersion: lines[0], fileVersion: lines[1] }, "Windows VersionInfo diagnostic");
}

export function createWindowsFileVersionInfoInspector() {
  if (process.platform !== "win32") {
    throw new WindowsExeVersionInspectorUnavailableError(
      "trustworthy Windows ProductVersion/FileVersion inspection requires win32; inject a hermetic inspector for deterministic self-tests instead of bypassing the gate.",
    );
  }
  return Object.freeze({
    kind: "windows-release-diagnostics-powershell-v1",
    inspect(executablePath) {
      if (typeof executablePath !== "string" || executablePath.length === 0) {
        throw new Error("Windows VersionInfo inspection requires an executable path.");
      }
      let output;
      try {
        output = execFileSync(
          "powershell.exe",
          [
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "$i=(Get-Item -LiteralPath $env:SYNDOCAL_RELEASE_INSPECT_PATH).VersionInfo;[Console]::Out.Write($i.ProductVersion+\"`n\"+$i.FileVersion)",
          ],
          {
            encoding: "utf8",
            stdio: ["ignore", "pipe", "pipe"],
            env: { ...process.env, SYNDOCAL_RELEASE_INSPECT_PATH: executablePath },
            timeout: 30_000,
            windowsHide: true,
          },
        );
      } catch (error) {
        throw new Error(
          "[SYNDOCAL_WINDOWS_EXE_VERSION_INSPECTION_FAILED] Windows VersionInfo diagnostic failed for "
            + executablePath
            + ": "
            + String(error instanceof Error ? error.message : error),
        );
      }
      return parseWindowsVersionInfoDiagnosticOutput(output);
    },
  });
}

export function normalizeInjectedWindowsVersionInfoInspector(inspector) {
  if (inspector === null || typeof inspector !== "object" || Array.isArray(inspector) || typeof inspector.inspect !== "function") {
    throw new WindowsExeVersionInspectorUnavailableError(
      "an injected Windows VersionInfo inspector was required but is missing or malformed; refusing to evaluate syndocal.exe on PE shape alone.",
    );
  }
  return Object.freeze({
    kind: typeof inspector.kind === "string" && inspector.kind.length > 0 ? inspector.kind : "injected-windows-version-info",
    inspect(executablePath) {
      let inspected;
      try {
        inspected = inspector.inspect(executablePath);
      } catch (error) {
        throw new Error(
          "[SYNDOCAL_WINDOWS_EXE_VERSION_INSPECTION_FAILED] injected Windows VersionInfo inspector failed for "
            + executablePath
            + ": "
            + String(error instanceof Error ? error.message : error),
        );
      }
      return validateInspectedWindowsVersionInfo(inspected, "injected Windows VersionInfo inspector");
    },
  });
}

function assertCompatibleWindowsFileVersion(fileVersion, productVersion, label) {
  const core = productVersion.split("-", 1)[0];
  const stampable = core.split(".").every((segment) => Number(segment) <= 65535);
  const exactCompanion =
    fileVersion === core
    || (fileVersion.startsWith(core + ".") && fileVersion.slice(core.length + 1) === "0");
  if (!stampable || !exactCompanion) {
    throw new Error(
      label
        + " syndocal.exe FileVersion "
        + JSON.stringify(fileVersion)
        + " is not the exact Windows-stampable companion ("
        + core
        + " or "
        + core
        + ".0, each component 0..65535, canonical digits, no fifth segment) of authoritative product SemVer "
        + productVersion
        + "; refusing ambiguous version metadata.",
    );
  }
}

function assertSyndocalExecutableReleaseIdentity(
  root,
  label,
  authority,
  inspector,
  { allowExactProductVersionFileVersion = false } = {},
) {
  const executablePath = join(root, "syndocal.exe");
  const record = assertSyndocalExecutable(executablePath, label + " syndocal.exe", [root]);
  const actualSha256 = createHash("sha256").update(record.bytes).digest("hex");
  if (actualSha256 !== authority.executableSha256) {
    throw new Error(
      label
        + " syndocal.exe SHA-256 "
        + actualSha256
        + " does not match the authoritative release hash "
        + authority.executableSha256
        + "; refusing a stale or substituted executable that merely has valid AMD64 PE shape.",
    );
  }
  const identity = withMaterializedVerifiedExecutable(
    record.bytes,
    (materializedPath) => inspector.inspect(materializedPath),
    { label: label + " syndocal.exe" },
  );
  if (identity.productVersion !== authority.productVersion) {
    throw new Error(
      label
        + " syndocal.exe ProductVersion "
        + JSON.stringify(identity.productVersion)
        + " does not match the authoritative product SemVer "
        + authority.productVersion
        + ".",
    );
  }
  if (!(allowExactProductVersionFileVersion && identity.fileVersion === authority.productVersion)) {
    assertCompatibleWindowsFileVersion(identity.fileVersion, authority.productVersion, label);
  }
  return Object.freeze({ sha256: actualSha256, productVersion: identity.productVersion, fileVersion: identity.fileVersion });
}

export function assertExtractedWindowsArtifactContents(
  root,
  label,
  inventory = loadWindowsRuntimeInventory(),
  { allowedExtraFiles = [] } = {},
) {
  const realRoot = assertSafeExternalDirectory(root, label);
  const verifiedInventory = validateWindowsRuntimeInventory(inventory, label + " runtime inventory");
  const approvedFiles = approvedArtifactFiles(verifiedInventory);
  const extraFiles = new Set(allowedExtraFiles);
  const approvedDirectories = approvedArtifactDirectories(verifiedInventory);
  const filesByRelativePath = new Map();
  for (const file of walkVerifiedFiles(realRoot, label, { inventory: verifiedInventory })) {
    const relativeFile = slashRelativePath(realRoot, file.path, label + " file path");
    if (filesByRelativePath.has(relativeFile)) {
      throw new Error(label + " contains duplicate canonical artifact file path: " + relativeFile);
    }
    filesByRelativePath.set(relativeFile, file);
    if (!approvedFiles.has(relativeFile) && !extraFiles.has(relativeFile)) {
      throw new Error(label + " contains an unapproved artifact file: " + relativeFile);
    }
  }
  for (const [relativeFile] of approvedFiles) {
    if (!filesByRelativePath.has(relativeFile)) {
      throw new Error(label + " is missing approved artifact file: " + relativeFile);
    }
  }
  for (const relativeFile of extraFiles) {
    if (!filesByRelativePath.has(relativeFile)) {
      throw new Error(label + " is missing allowed extra artifact file: " + relativeFile);
    }
  }
  const extractedDirectories = extractedArtifactDirectories(realRoot, label);
  for (const relativeDirectory of extractedDirectories) {
    if (!approvedDirectories.has(relativeDirectory)) {
      throw new Error(label + " contains an unapproved artifact directory: " + relativeDirectory);
    }
  }
  for (const relativeDirectory of approvedDirectories) {
    if (!extractedDirectories.has(relativeDirectory)) {
      throw new Error(label + " is missing approved artifact directory: " + relativeDirectory);
    }
  }

  assertSyndocalExecutable(join(realRoot, "syndocal.exe"), label + " syndocal.exe", [realRoot]);
  for (const runtime of verifiedInventory.runtime_dlls) {
    verifyPinnedRuntimeFile(
      join(realRoot, runtime.filename),
      runtime,
      label + " runtime " + runtime.filename,
      { allowedRoots: [realRoot] },
    );
  }
  for (const resource of verifiedInventory.common_resources) {
    verifyPinnedCommonResource(
      join(realRoot, resource.destination),
      resource,
      label + " common resource " + resource.destination,
      { allowedRoots: [realRoot] },
    );
  }
  return Object.freeze({
    root: realRoot,
    applicationDirectory: realRoot,
    fileCount: filesByRelativePath.size,
    ffmpegRuntimeCount: verifiedInventory.runtime_dlls.length,
    commonResourceCount: verifiedInventory.common_resources.length,
  });
}

export function assertManualThreeRootWindowsReleaseArtifacts({
  nsisRoot,
  msiRoot,
  updaterRoot,
  productVersion,
  executableSha256,
  inspector,
  inventory = loadWindowsRuntimeInventory(),
}) {
  if (typeof nsisRoot !== "string" || typeof msiRoot !== "string" || typeof updaterRoot !== "string") {
    throw new Error("Manual three-root mode requires explicit NSIS, MSI, and updater extracted-root directories.");
  }
  const authority = parseWindowsReleaseExeAuthority({ product_version: productVersion, executable_sha256: executableSha256 });
  const activeInspector = inspector === undefined
    ? createWindowsFileVersionInfoInspector()
    : normalizeInjectedWindowsVersionInfoInspector(inspector);
  const verifiedInventory = validateWindowsRuntimeInventory(inventory, "manual three-root Windows release runtime inventory");
  const roots = [["NSIS", nsisRoot], ["MSI", msiRoot], ["Updater", updaterRoot]];
  const identities = [];
  for (const [roleLabel, root] of roots) {
    assertExtractedWindowsArtifactContents(root, roleLabel + " extracted artifact", verifiedInventory);
    identities.push([
      roleLabel,
      assertSyndocalExecutableReleaseIdentity(root, roleLabel + " extracted artifact", authority, activeInspector),
    ]);
  }
  for (let index = 1; index < identities.length; index += 1) {
    const [previousRole, previous] = identities[index - 1];
    const [role, current] = identities[index];
    for (const field of ["sha256", "productVersion", "fileVersion"]) {
      if (previous[field] !== current[field]) {
        throw new Error(
          "Manual three-root Windows release roots disagree: "
            + previousRole
            + " and "
            + role
            + " syndocal.exe "
            + field
            + " values differ ("
            + JSON.stringify(previous[field])
            + " vs "
            + JSON.stringify(current[field])
            + ").",
        );
      }
    }
  }
  return Object.freeze({
    productVersion: authority.productVersion,
    executableSha256: authority.executableSha256,
    inspectorKind: activeInspector.kind,
    roots: Object.freeze(Object.fromEntries(identities)),
  });
}

export function assertManualTwoRootWindowsInstallerArtifacts({
  nsisRoot,
  msiRoot,
  productVersion,
  executableSha256,
  inspector,
  inventory = loadWindowsRuntimeInventory(),
}) {
  if (typeof nsisRoot !== "string" || typeof msiRoot !== "string") {
    throw new Error("Manual two-root Windows installer mode requires explicit NSIS and MSI application directories.");
  }
  const authority = parseWindowsReleaseExeAuthority({ product_version: productVersion, executable_sha256: executableSha256 }, "manual two-root Windows installer executable authority");
  const activeInspector = inspector === undefined
    ? createWindowsFileVersionInfoInspector()
    : normalizeInjectedWindowsVersionInfoInspector(inspector);
  const verifiedInventory = validateWindowsRuntimeInventory(inventory, "manual two-root Windows installer runtime inventory");
  const roots = [
    ["NSIS", nsisRoot, { allowedExtraFiles: ["uninstall.exe"] }],
    ["MSI", msiRoot, {}],
  ];
  const identities = [];
  for (const [roleLabel, root, options] of roots) {
    assertExtractedWindowsArtifactContents(root, roleLabel + " installed artifact", verifiedInventory, options);
    identities.push([
      roleLabel,
      assertSyndocalExecutableReleaseIdentity(
        root,
        roleLabel + " installed artifact",
        authority,
        activeInspector,
        { allowExactProductVersionFileVersion: true },
      ),
    ]);
  }
  for (const field of ["sha256", "productVersion", "fileVersion"]) {
    if (identities[0][1][field] !== identities[1][1][field]) {
      throw new Error(
        "Manual two-root Windows installer roots disagree on syndocal.exe "
          + field
          + " ("
          + JSON.stringify(identities[0][1][field])
          + " vs "
          + JSON.stringify(identities[1][1][field])
          + ").",
      );
    }
  }
  return Object.freeze({
    productVersion: authority.productVersion,
    executableSha256: authority.executableSha256,
    inspectorKind: activeInspector.kind,
    roots: Object.freeze(Object.fromEntries(identities)),
  });
}

function exactKeys(value, keys, label) {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error(label + " must be an object.");
  }
  const actual = Object.keys(value).sort();
  const expected = [...keys].sort();
  if (JSON.stringify(actual) !== JSON.stringify(expected)) {
    throw new Error(label + " keys are not exact.");
  }
}

export function validateWindowsCandidateExtractionInventory(value, label = "Windows candidate extraction inventory") {
  exactKeys(value, ["schema_version", "product_version", "release_evidence", "extractions"], label);
  if (value.schema_version !== 2 || typeof value.product_version !== "string" || !/^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)-rc\.[1-9]\d*$/u.test(value.product_version)) {
    throw new Error(label + " must use schema_version 2 and an RC SemVer product_version.");
  }
  exactKeys(value.release_evidence, ["path", "sha256", "commit", "tag"], label + " release_evidence");
  assertStrictRelativePath(value.release_evidence.path, label + " release_evidence.path");
  if (!/^[0-9a-f]{64}$/u.test(value.release_evidence.sha256)
    || !/^[0-9a-f]{40}$/u.test(value.release_evidence.commit)
    || value.release_evidence.tag !== "v" + value.product_version) {
    throw new Error(label + " release_evidence must bind an exact manifest hash, commit, and tag.");
  }
  if (!Array.isArray(value.extractions) || value.extractions.length !== 3) {
    throw new Error(label + " must contain exactly NSIS, MSI, and updater extractions.");
  }
  const requiredRoles = new Set(["nsis", "msi", "updater"]);
  const seenRoles = new Set();
  for (const extraction of value.extractions) {
    exactKeys(
      extraction,
      ["role", "artifact_path", "artifact_sha256", "signature_path", "signature_sha256", "extracted_root", "extracted_tree_sha256", "method"],
      label + " extraction",
    );
    if (!requiredRoles.has(extraction.role) || seenRoles.has(extraction.role)) {
      throw new Error(label + " roles must be the exact NSIS, MSI, updater set.");
    }
    seenRoles.add(extraction.role);
    assertStrictRelativePath(extraction.artifact_path, label + " artifact_path");
    if (extraction.signature_path !== null) assertStrictRelativePath(extraction.signature_path, label + " signature_path");
    assertStrictRelativePath(extraction.extracted_root, label + " extracted_root");
    if ((extraction.signature_path !== null && typeof extraction.signature_path !== "string")
      || (extraction.signature_path !== null && extraction.signature_path.length === 0)
      || (extraction.signature_path !== null && !/^[0-9a-f]{64}$/u.test(extraction.signature_sha256))
      || (extraction.signature_path === null && extraction.signature_sha256 !== null)
      || !/^[0-9a-f]{64}$/u.test(extraction.artifact_sha256) || !/^[0-9a-f]{64}$/u.test(extraction.extracted_tree_sha256)) {
      throw new Error(label + " extraction hashes must be exact lowercase SHA-256 values.");
    }
    if (extraction.role === "updater" && extraction.signature_path === null) throw new Error(label + " updater extraction requires an adjacent signature.");
    if (extraction.role !== "updater" && extraction.signature_path !== null) throw new Error(label + " only the updater extraction may carry a signature.");
    if (extraction.method !== "repository-owned-deterministic-extractor-v1") {
      throw new Error(
        label
          + " has no approved repository-owned deterministic installer extractor.",
      );
    }
  }
  return Object.freeze(structuredClone(value));
}

function canonicalCandidatePath(path) {
  const normalized = resolve(path).replaceAll("\\", "/");
  return process.platform === "win32" ? normalized.toLocaleLowerCase("en-US") : normalized;
}

function assertCandidateExtractionOutputShape(outputRoot, label) {
  const entries = readdirSync(outputRoot, { withFileTypes: true });
  const names = entries.map((entry) => entry.name).sort();
  assert.deepStrictEqual(names, ["extracted", "inventory.json"], label + " must publish only extracted/ and inventory.json.");
  const extracted = entries.find((entry) => entry.name === "extracted");
  if (extracted === undefined || !extracted.isDirectory()) {
    throw new Error(label + " extracted entry must be an ordinary directory.");
  }
  const inventory = entries.find((entry) => entry.name === "inventory.json");
  if (inventory === undefined || !inventory.isFile() || inventory.isSymbolicLink()) {
    throw new Error(label + " inventory.json must be an ordinary regular file.");
  }
}

function hashBytes(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

function loadCandidateReleaseEvidence({ workspace, productVersion, releaseEvidencePath, headCommit }) {
  const expectedPath = join(workspace, "qa", "release", "release-evidence.json");
  const pathText = releaseEvidencePath ?? expectedPath;
  if (canonicalCandidatePath(pathText) !== canonicalCandidatePath(expectedPath)) {
    throw new Error("candidate release evidence must be the exact qa/release/release-evidence.json manifest.");
  }
  const record = readVerifiedRegularFile(pathText, "candidate release evidence manifest", { allowedRoots: [workspace] });
  const text = record.bytes.toString("utf8");
  if (!Buffer.from(text, "utf8").equals(record.bytes)) throw new Error("candidate release evidence manifest is not UTF-8.");
  const manifest = parseStrictJson(text, "candidate release evidence manifest");
  exactKeys(manifest, ["schemaVersion", "productVersion", "tag", "commit", "previousVersion", "previousTag", "updater", "artifacts"], "candidate release evidence manifest");
  if (manifest.schemaVersion !== 1 || manifest.productVersion !== productVersion || manifest.tag !== "v" + productVersion || !/^[0-9a-f]{40}$/u.test(manifest.commit)) {
    throw new Error("candidate release evidence manifest does not identify the exact product, tag, and commit.");
  }
  const currentHead = headCommit ?? execFileSync("git", ["rev-parse", "HEAD"], { cwd: workspace, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim();
  if (manifest.commit !== currentHead) throw new Error("candidate release evidence commit does not match HEAD.");
  exactKeys(manifest.updater, ["channel", "endpoint", "manifestPath", "manifestSha256", "publicKeyPath", "publicKeyFingerprint"], "candidate release evidence updater");
  if (!/^(stable|beta|nightly)$/u.test(manifest.updater.channel) || typeof manifest.updater.endpoint !== "string" || !/^https:\/\/[^\s]+$/u.test(manifest.updater.endpoint) || !/^[0-9a-f]{64}$/u.test(manifest.updater.publicKeyFingerprint)) {
    throw new Error("candidate release evidence updater identity is invalid.");
  }
  const endpoint = canonicalDecodedUpdaterUrlPath(manifest.updater.endpoint, "candidate release evidence updater endpoint");
  if (!endpoint.pathSegments.includes(manifest.updater.channel)) {
    throw new Error("candidate release evidence updater endpoint is not an exact credential-free channel URL.");
  }
  const evidenceRoot = dirname(record.realPath);
  const readEvidenceChild = (relativePath, label) => {
    try {
      assertStrictRelativePath(relativePath, label + " path");
    } catch (error) {
      throw new Error(label + " path is unsafe: " + String(error instanceof Error ? error.message : error));
    }
    return readVerifiedRegularFile(join(evidenceRoot, relativePath), label, { allowedRoots: [evidenceRoot] });
  };
  const updaterManifestRecord = readEvidenceChild(manifest.updater.manifestPath, "candidate updater manifest");
  if (hashBytes(updaterManifestRecord.bytes) !== String(manifest.updater.manifestSha256).toLowerCase()) throw new Error("candidate updater manifest SHA-256 does not match release evidence.");
  const updaterManifest = parseStrictJson(updaterManifestRecord.bytes.toString("utf8"), "candidate updater manifest");
  if (updaterManifest.version !== productVersion || typeof updaterManifest.platforms !== "object" || updaterManifest.platforms === null) throw new Error("candidate updater manifest version/platforms do not match release evidence.");
  const publicKeyRecord = readEvidenceChild(manifest.updater.publicKeyPath, "candidate updater public key");
  const parsedPublicKey = parseTauriMinisignPublicKey(publicKeyRecord.bytes, {
    expectedFingerprint: manifest.updater.publicKeyFingerprint,
    label: "candidate updater public key",
  });
  if (!Array.isArray(manifest.artifacts)) throw new Error("candidate release evidence artifacts must be an array.");
  const findArtifact = (role) => {
    const matches = manifest.artifacts.filter((artifact) => artifact.role === role && artifact.target === "windows-x86_64");
    if (matches.length !== 1) throw new Error("candidate release evidence must contain exactly one " + role + " windows-x86_64 artifact.");
    return matches[0];
  };
  const executableArtifact = findArtifact("windows-executable");
  const updaterArtifact = findArtifact("updater-payload");
  const installerArtifacts = manifest.artifacts.filter((artifact) => artifact.role === "installer" && artifact.target === "windows-x86_64");
  if (installerArtifacts.length < 2 || installerArtifacts.some((artifact) => !/^[0-9a-fA-F]{64}$/u.test(artifact.sha256))) {
    throw new Error("candidate release evidence must contain NSIS and MSI installer identities for windows-x86_64.");
  }
  if (!/^[0-9a-fA-F]{64}$/u.test(executableArtifact.sha256) || !/^[0-9a-fA-F]{64}$/u.test(updaterArtifact.sha256)
    || typeof updaterArtifact.signaturePath !== "string" || !/^[0-9a-fA-F]{64}$/u.test(updaterArtifact.signatureSha256)) {
    throw new Error("candidate release evidence artifact identities are incomplete.");
  }
  const updaterSignatureRecord = readEvidenceChild(updaterArtifact.signaturePath, "candidate updater signature");
  if (hashBytes(updaterSignatureRecord.bytes) !== updaterArtifact.signatureSha256.toLowerCase()) throw new Error("candidate updater signature hash does not match release evidence.");
  return Object.freeze({
    record,
    manifest,
    executableArtifact,
    updaterArtifact,
    installerArtifacts,
    updaterManifest,
    publicKeyRecord,
    parsedPublicKey,
    updaterSignatureRecord,
    releaseEvidenceHash: hashBytes(record.bytes),
    commit: manifest.commit,
    tag: manifest.tag,
  });
}

function readTauriUpdaterMode(workspace) {
  const path = join(workspace, "app", "src-tauri", "tauri.updater.conf.json");
  const record = readVerifiedRegularFile(path, "Tauri updater configuration", { allowedRoots: [workspace] });
  const config = parseStrictJson(record.bytes.toString("utf8"), "Tauri updater configuration");
  const mode = config?.bundle?.createUpdaterArtifacts;
  if (mode !== true && mode !== "v1Compatible") throw new Error("Tauri v2 updater configuration must explicitly select true or v1Compatible.");
  return mode;
}

function assertCandidateUpdaterCryptographicGate(verifiedCandidate, evidence, safeWorkspace, updaterMode) {
  const extraction = verifiedCandidate.extractions.find((entry) => entry.role === "updater");
  if (extraction === undefined) throw new Error("Windows candidate extraction inventory has no updater role.");
  const expectedArtifactPaths = expectedWindowsCandidateArtifactRelativePaths(verifiedCandidate.product_version, "updater", updaterMode);
  if (!expectedArtifactPaths.some((path) => canonicalCandidatePath(join(safeWorkspace, path)) === canonicalCandidatePath(join(safeWorkspace, extraction.artifact_path)))) {
    throw new Error("Windows candidate updater artifact_path is not an exact versioned Tauri output under target/release/bundle.");
  }
  const artifactPath = resolve(safeWorkspace, extraction.artifact_path);
  const artifactRecord = readVerifiedRegularFile(artifactPath, "Windows candidate updater artifact", { allowedRoots: [safeWorkspace] });
  const artifactSha256 = hashBytes(artifactRecord.bytes);
  if (
    artifactSha256 !== extraction.artifact_sha256
    || artifactSha256 !== String(evidence.updaterArtifact.sha256).toLowerCase()
    || basename(artifactPath) !== evidence.updaterArtifact.filename
  ) {
    throw new Error("Windows candidate updater artifact SHA-256 mismatch or filename does not match release evidence.");
  }
  const expectedSignaturePath = join(dirname(artifactPath), basename(artifactPath) + ".sig");
  if (
    extraction.signature_path !== slashRelativePath(safeWorkspace, expectedSignaturePath, "Windows candidate updater signature path")
    || extraction.signature_path === null
  ) {
    throw new Error("Windows candidate updater signature must be the exact adjacent .sig file.");
  }
  const signatureRecord = readVerifiedRegularFile(expectedSignaturePath, "Windows candidate updater adjacent signature", { allowedRoots: [safeWorkspace] });
  const signatureSha256 = hashBytes(signatureRecord.bytes);
  if (
    signatureSha256 !== extraction.signature_sha256
    || signatureSha256 !== String(evidence.updaterArtifact.signatureSha256).toLowerCase()
  ) {
    throw new Error("Windows candidate updater signature SHA-256 mismatch.");
  }
  verifyTauriUpdaterPayload({
    payloadBytes: artifactRecord.bytes,
    artifactFilename: evidence.updaterArtifact.filename,
    signatureEvidenceBytes: signatureRecord.bytes,
    publicKey: evidence.parsedPublicKey,
    expectedPublicKeyFingerprint: evidence.manifest.updater.publicKeyFingerprint,
    updaterManifest: evidence.updaterManifest,
    target: "windows-x86_64",
    channel: evidence.manifest.updater.channel,
    label: "Windows candidate updater payload",
  });
  return Object.freeze({ artifactPath: artifactRecord.realPath, artifactSha256, signatureSha256 });
}

function inspectCandidateExecutable(bytes, executableInspector, updater) {
  return withMaterializedVerifiedExecutable(bytes, (materializedPath) => {
    if (executableInspector !== undefined) return executableInspector(materializedPath, updater);
    if (process.platform !== "win32") throw new WindowsExeVersionInspectorUnavailableError("candidate syndocal.exe identity requires a Windows VersionInfo/runtime inspector.");
    const versionInfo = createWindowsFileVersionInfoInspector().inspect(materializedPath);
    const runtimeIdentity = parseRuntimeUpdaterIdentity(execFileSync(materializedPath, ["--print-updater-release-identity"], { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"], timeout: 10_000, windowsHide: true }));
    return { ...versionInfo, runtimeIdentity };
  }, { label: "Windows candidate syndocal.exe" });
}

export function assertCandidateArtifactExtractionInventory(
  value,
  {
    workspace = workspaceRoot,
    inventoryPath,
    inventory = loadWindowsRuntimeInventory(),
    releaseEvidencePath,
    headCommit,
    executableInspector,
  } = {},
) {
  const verifiedCandidate = validateWindowsCandidateExtractionInventory(value);
  const safeWorkspace = assertSafeExternalDirectory(workspace, "Windows candidate extraction workspace");
  const updaterMode = readTauriUpdaterMode(safeWorkspace);
  const evidence = loadCandidateReleaseEvidence({ workspace: safeWorkspace, productVersion: verifiedCandidate.product_version, releaseEvidencePath, headCommit });
  // This deliberately precedes every extracted syndocal.exe read/inspector
  // call. A candidate payload, adjacent signature, public key, and updater
  // manifest must be cryptographically coherent before any candidate EXE can
  // execute its runtime-identity diagnostic.
  const cryptographicGate = assertCandidateUpdaterCryptographicGate(verifiedCandidate, evidence, safeWorkspace, updaterMode);
  if (verifiedCandidate.release_evidence.path !== slashRelativePath(safeWorkspace, evidence.record.realPath, "candidate release evidence path")) {
    throw new Error("Windows candidate inventory release_evidence.path does not identify the authoritative manifest.");
  }
  if (verifiedCandidate.release_evidence.sha256 !== evidence.releaseEvidenceHash
    || verifiedCandidate.release_evidence.commit !== evidence.commit
    || verifiedCandidate.release_evidence.tag !== evidence.tag) {
    throw new Error("Windows candidate inventory release_evidence identity does not match the authoritative manifest.");
  }
  if (typeof inventoryPath !== "string" || inventoryPath.length === 0 || !isAbsolute(inventoryPath)) {
    throw new Error("Windows candidate extraction inventory verification requires its absolute inventory path.");
  }
  const resolvedInventoryPath = resolve(inventoryPath);
  const inventoryRecord = readVerifiedRegularFile(resolvedInventoryPath, "Windows candidate extraction inventory", {
    allowedRoots: [safeWorkspace],
  });
  const inventoryText = inventoryRecord.bytes.toString("utf8");
  if (!Buffer.from(inventoryText, "utf8").equals(inventoryRecord.bytes)) throw new Error("Windows candidate extraction inventory is not UTF-8.");
  const parsedInventory = validateWindowsCandidateExtractionInventory(
    parseStrictJson(inventoryText, "Windows candidate extraction inventory"),
  );
  assert.deepStrictEqual(parsedInventory, verifiedCandidate, "Windows candidate extraction inventory value does not match its file.");
  const inventoryRelative = slashRelativePath(safeWorkspace, resolvedInventoryPath, "Windows candidate extraction inventory path");
  const candidatePrefix = WINDOWS_CANDIDATE_EXTRACTION_ROOT + "/";
  if (!inventoryRelative.startsWith(candidatePrefix) || !inventoryRelative.endsWith("/inventory.json")) {
    throw new Error(
      "Windows candidate extraction inventory must be inventory.json below "
        + WINDOWS_CANDIDATE_EXTRACTION_ROOT
        + "; got "
        + inventoryRelative,
    );
  }
  const outputRoot = assertSafeExternalDirectory(dirname(resolvedInventoryPath), "Windows candidate extraction output root");
  if (basename(outputRoot) !== verifiedCandidate.product_version) {
    throw new Error(
      "Windows candidate extraction output root must be named for product_version "
        + verifiedCandidate.product_version
        + "; got "
        + basename(outputRoot),
    );
  }
  assertCandidateExtractionOutputShape(outputRoot, "Windows candidate extraction output");
  const extractedRoot = assertSafeExternalDirectory(join(outputRoot, "extracted"), "Windows candidate extraction extracted root");
  const extractedNames = readdirSync(extractedRoot, { withFileTypes: true }).map((entry) => entry.name).sort();
  assert.deepStrictEqual(extractedNames, ["msi", "nsis", "updater"], "Windows candidate extraction must publish exactly NSIS, MSI, and updater roots.");
  const outputRelative = slashRelativePath(safeWorkspace, outputRoot, "Windows candidate extraction output root");
  const seenExtractedRoots = new Set();
  const identities = [];
  for (const extraction of verifiedCandidate.extractions) {
    const role = extraction.role;
    const expectedArtifactPaths = expectedWindowsCandidateArtifactRelativePaths(verifiedCandidate.product_version, role, updaterMode);
    if (!expectedArtifactPaths.some((path) => canonicalCandidatePath(join(safeWorkspace, path)) === canonicalCandidatePath(join(safeWorkspace, extraction.artifact_path)))) {
      throw new Error(
        "Windows candidate extraction "
          + role
          + " artifact_path is not an exact versioned Tauri output under target/release/bundle: "
          + extraction.artifact_path,
      );
    }
    const artifactPath = resolve(safeWorkspace, extraction.artifact_path);
    const artifactRecord = readVerifiedRegularFile(artifactPath, "Windows candidate " + role + " artifact", {
      allowedRoots: [safeWorkspace],
    });
    const artifactSha256 = createHash("sha256").update(artifactRecord.bytes).digest("hex");
    if (artifactSha256 !== extraction.artifact_sha256) {
      throw new Error(
        "Windows candidate " + role + " artifact SHA-256 mismatch: expected " + extraction.artifact_sha256 + ", actual " + artifactSha256 + ".",
      );
    }
    if (role === "nsis" || role === "msi") {
      const installerEvidence = evidence.installerArtifacts.find((artifact) => artifact.filename === basename(artifactPath));
      if (installerEvidence === undefined || String(installerEvidence.sha256).toLowerCase() !== artifactSha256) {
        throw new Error("Windows candidate " + role + " installer hash does not match release evidence.");
      }
    }
    if (role === "updater") {
      const signaturePath = resolve(safeWorkspace, extraction.signature_path);
      const signatureRecord = readVerifiedRegularFile(signaturePath, "Windows candidate updater signature", { allowedRoots: [safeWorkspace] });
      if (hashBytes(signatureRecord.bytes) !== extraction.signature_sha256) throw new Error("Windows candidate updater signature SHA-256 mismatch.");
      if (extraction.signature_path !== slashRelativePath(safeWorkspace, join(dirname(artifactPath), basename(artifactPath) + ".sig"), "Windows candidate updater signature path")) {
        throw new Error("Windows candidate updater signature must be the exact adjacent .sig file.");
      }
      if (artifactSha256 !== String(evidence.updaterArtifact.sha256).toLowerCase() || extraction.signature_sha256 !== String(evidence.updaterArtifact.signatureSha256).toLowerCase()) {
        throw new Error("Windows candidate updater artifact/signature identity does not match release evidence.");
      }
      const platformEntry = evidence.updaterManifest.platforms["windows-x86_64"];
      if (platformEntry === undefined || typeof platformEntry.signature !== "string" || platformEntry.signature !== signatureRecord.bytes.toString("utf8").trim()) {
        throw new Error("Windows candidate updater URL/signature does not match the release-evidence updater manifest.");
      }
    }
    const expectedExtractedRelative = outputRelative + "/extracted/" + role;
    if (canonicalCandidatePath(join(safeWorkspace, extraction.extracted_root)) !== canonicalCandidatePath(join(safeWorkspace, expectedExtractedRelative))) {
      throw new Error(
        "Windows candidate "
          + role
          + " extracted_root must be the exact published role directory "
          + expectedExtractedRelative
          + "; got "
          + extraction.extracted_root,
      );
    }
    const roleRoot = assertSafeExternalDirectory(resolve(safeWorkspace, extraction.extracted_root), "Windows candidate " + role + " extracted root");
    const roleKey = canonicalCandidatePath(roleRoot);
    if (seenExtractedRoots.has(roleKey)) throw new Error("Windows candidate extraction roots must be unique: " + roleRoot);
    seenExtractedRoots.add(roleKey);
    const extractedSummary = assertExtractedWindowsArtifactContents(roleRoot, "Windows candidate " + role + " extracted artifact", inventory);
    const treeSha256 = computeVerifiedCandidateTreeSha256(roleRoot, "Windows candidate " + role + " extracted tree");
    if (treeSha256 !== extraction.extracted_tree_sha256) {
      throw new Error(
        "Windows candidate " + role + " extracted tree SHA-256 mismatch: expected " + extraction.extracted_tree_sha256 + ", actual " + treeSha256 + ".",
      );
    }
    const executableRecord = readVerifiedRegularFile(join(roleRoot, "syndocal.exe"), "Windows candidate " + role + " syndocal.exe", { allowedRoots: [roleRoot] });
    const executableSha256 = hashBytes(executableRecord.bytes);
    if (executableSha256 !== String(evidence.executableArtifact.sha256).toLowerCase()) {
      throw new Error("Windows candidate " + role + " syndocal.exe SHA-256 does not match the authoritative release-evidence executable.");
    }
    const inspected = inspectCandidateExecutable(executableRecord.bytes, executableInspector, evidence.manifest.updater);
    if (inspected?.productVersion !== verifiedCandidate.product_version) throw new Error("Windows candidate " + role + " syndocal.exe ProductVersion does not match the candidate.");
    const runtimeIdentity = inspected?.runtimeIdentity;
    if (runtimeIdentity === undefined
      || runtimeIdentity.endpoint !== evidence.manifest.updater.endpoint
      || runtimeIdentity.channel !== evidence.manifest.updater.channel
      || runtimeIdentity.publicKeyFingerprint !== evidence.manifest.updater.publicKeyFingerprint) {
      throw new Error("Windows candidate " + role + " syndocal.exe runtime updater identity does not match release evidence.");
    }
    identities.push({ role, artifactPath: artifactRecord.realPath, extractedRoot: roleRoot, fileCount: extractedSummary.fileCount, executableSha256, productVersion: inspected.productVersion, runtimeIdentity });
  }
  if (seenExtractedRoots.size !== 3) throw new Error("Windows candidate extraction must publish exactly three unique role roots.");
  for (let index = 1; index < identities.length; index += 1) {
    for (const field of ["executableSha256", "productVersion"]) {
      if (identities[index][field] !== identities[0][field]) throw new Error("Windows candidate extracted roots disagree on syndocal.exe " + field + ".");
    }
    if (JSON.stringify(identities[index].runtimeIdentity) !== JSON.stringify(identities[0].runtimeIdentity)) throw new Error("Windows candidate extracted roots disagree on runtime updater identity.");
  }
  return Object.freeze({
    productVersion: verifiedCandidate.product_version,
    outputRoot,
    inventoryPath: resolvedInventoryPath,
    method: WINDOWS_CANDIDATE_EXTRACTION_METHOD,
    updaterPayloadSha256: cryptographicGate.artifactSha256,
    roles: Object.freeze(identities),
    extractedRoot,
  });
}

export function assertCandidateArtifactExtractionRequirement(inventoryPath = process.env.SYNDOCAL_WINDOWS_ARTIFACT_INVENTORY) {
  if (
    typeof inventoryPath !== "string"
    || inventoryPath.length === 0
    || inventoryPath !== inventoryPath.trim()
    || inventoryPath.includes("\0")
    || /[\r\n]/u.test(inventoryPath)
    || !isAbsolute(inventoryPath)
  ) {
    throw new CandidateArtifactExtractionRequiredError(
      "Release-candidate Windows packaging is fail-closed: set SYNDOCAL_WINDOWS_ARTIFACT_INVENTORY to one explicit absolute local-file path for a reviewed NSIS/MSI/updater extraction inventory generated by a repository-owned deterministic extractor.",
    );
  }
  try {
    const resolvedInventoryPath = resolve(inventoryPath);
    const inventoryDirectory = assertSafeExternalDirectory(
      dirname(resolvedInventoryPath),
      "Windows candidate extraction inventory directory",
    );
    const record = readVerifiedRegularFile(resolvedInventoryPath, "Windows candidate extraction inventory", {
      allowedRoots: [inventoryDirectory],
    });
    const text = record.bytes.toString("utf8");
    if (!Buffer.from(text, "utf8").equals(record.bytes)) throw new Error("not UTF-8");
    const parsed = parseStrictJson(text, "Windows candidate extraction inventory");
    return assertCandidateArtifactExtractionInventory(parsed, { inventoryPath: resolvedInventoryPath });
  } catch (error) {
    throw new CandidateArtifactExtractionRequiredError(
      "Release-candidate Windows packaging is fail-closed: extraction inventory path, file identity, or content is unavailable, stale, unsafe, or invalid: "
        + String(error instanceof Error ? error.message : error),
    );
  }
}

function parseCli(argv) {
  if (argv.length === 1 && argv[0] === "--self-test") return { selfTest: true };
  if (argv.length === 1 && argv[0] === "--require-candidate-extraction") return { requireCandidate: true };
  if (
    argv.length === 8
    && argv[0] === "--nsis-root"
    && argv[2] === "--msi-root"
    && argv[4] === "--product-version"
    && argv[6] === "--exe-sha256"
  ) {
    return {
      installerRoots: true,
      nsisRoot: argv[1],
      msiRoot: argv[3],
      productVersion: argv[5],
      exeSha256: argv[7],
    };
  }
  if (
    argv.length === 10
    && argv[0] === "--nsis-root"
    && argv[2] === "--msi-root"
    && argv[4] === "--updater-root"
    && argv[6] === "--product-version"
    && argv[8] === "--exe-sha256"
  ) {
    return {
      nsisRoot: argv[1],
      msiRoot: argv[3],
      updaterRoot: argv[5],
      productVersion: argv[7],
      exeSha256: argv[9],
    };
  }
  throw new Error(
    "Usage: check-windows-release-artifacts.mjs --self-test | --require-candidate-extraction"
      + " | --nsis-root <installed-dir> --msi-root <installed-dir> --product-version <canonical-SemVer> --exe-sha256 <64-hex>"
      + " | --nsis-root <dir> --msi-root <dir> --updater-root <dir> --product-version <canonical-SemVer> --exe-sha256 <64-hex>"
      + " (manual three-root mode requires the explicit canonical product SemVer and executable SHA-256 authority; PE shape alone never accepts syndocal.exe)",
  );
}

function selfTest() {
  const inventory = loadWindowsRuntimeInventory();
  runWindowsCandidateExtractorSelfTest();
  runVerifiedMaterializationSelfTest();
  let assertions = 0;
  const pass = (condition, label) => {
    assert.ok(condition, label);
    assertions += 1;
  };
  const rejects = (action, pattern) => {
    assert.throws(action, pattern);
    assertions += 1;
  };
  const rejectsCandidate = (action, pattern) => {
    let thrown;
    try {
      action();
    } catch (error) {
      thrown = error;
    }
    assert.ok(thrown instanceof CandidateArtifactExtractionRequiredError, "candidate extraction failure must use the typed error");
    assert.equal(thrown.code, "SYNDOCAL_WINDOWS_ARTIFACT_EXTRACTION_REQUIRED");
    assert.match(thrown.message, pattern);
    assertions += 1;
  };
  pass(
    JSON.stringify(parseCli(["--self-test"])) === JSON.stringify({ selfTest: true }),
    "parseCli accepts exactly --self-test alone",
  );
  pass(
    JSON.stringify(parseCli(["--require-candidate-extraction"])) === JSON.stringify({ requireCandidate: true }),
    "parseCli accepts exactly --require-candidate-extraction alone",
  );
  assert.deepStrictEqual(
    parseCli([
      "--nsis-root", "nsis-dir",
      "--msi-root", "msi-dir",
      "--product-version", "1.2.3-rc.1",
      "--exe-sha256", "a".repeat(64),
    ]),
    {
      installerRoots: true,
      nsisRoot: "nsis-dir",
      msiRoot: "msi-dir",
      productVersion: "1.2.3-rc.1",
      exeSha256: "a".repeat(64),
    },
  );
  assertions += 1;
  assert.deepStrictEqual(
    parseCli([
      "--nsis-root", "nsis-dir",
      "--msi-root", "msi-dir",
      "--updater-root", "updater-dir",
      "--product-version", "1.2.3-rc.1",
      "--exe-sha256", "a".repeat(64),
    ]),
    {
      nsisRoot: "nsis-dir",
      msiRoot: "msi-dir",
      updaterRoot: "updater-dir",
      productVersion: "1.2.3-rc.1",
      exeSha256: "a".repeat(64),
    },
  );
  assertions += 1;
  const happyArgs = [
    "--nsis-root", "nsis-dir",
    "--msi-root", "msi-dir",
    "--updater-root", "updater-dir",
    "--product-version", "1.2.3-rc.1",
    "--exe-sha256", "a".repeat(64),
  ];
  const installerArgs = [
    "--nsis-root", "nsis-dir",
    "--msi-root", "msi-dir",
    "--product-version", "1.2.3-rc.1",
    "--exe-sha256", "a".repeat(64),
  ];
  const usageAttack = (argv) => rejects(() => parseCli(argv), /Usage: check-windows-release-artifacts\.mjs/u);
  usageAttack([]);
  usageAttack(["--self-test", "--extra"]);
  usageAttack(["--extra", "--self-test"]);
  usageAttack(["--require-candidate-extraction", "--extra"]);
  usageAttack(["--self-test", "--require-candidate-extraction"]);
  usageAttack(happyArgs.slice(0, 8));
  usageAttack(installerArgs.slice(0, 6));
  usageAttack([...happyArgs.slice(0, 4), "--exe-sha256", "a".repeat(64), "--product-version", "1.2.3-rc.1"]);
  usageAttack([
    "--nsis-root", "nsis-dir",
    "--updater-root", "updater-dir",
    "--msi-root", "msi-dir",
    "--product-version", "1.2.3-rc.1",
    "--exe-sha256", "a".repeat(64),
  ]);
  usageAttack([...happyArgs, "surplus"]);
  usageAttack([...happyArgs, "--extra-flag"]);
  usageAttack([...happyArgs.slice(0, 8), "--exe-sha256", "b".repeat(64), "--nsis-root", "again"]);
  usageAttack([
    "--nsis-root", "nsis-dir",
    "--nsis-root", "nsis-dir",
    "--msi-root", "msi-dir",
    "--updater-root", "updater-dir",
    "--product-version", "1.2.3-rc.1",
    "--exe-sha256", "a".repeat(64),
  ]);
  rejects(
    () => {
      const cli = parseCli([
        "--nsis-root", "n",
        "--msi-root", "m",
        "--updater-root", "u",
        "--product-version", "1.2.3-rc.1",
        "--exe-sha256", "--self-test",
      ]);
      return assertManualThreeRootWindowsReleaseArtifacts({
        nsisRoot: cli.nsisRoot,
        msiRoot: cli.msiRoot,
        updaterRoot: cli.updaterRoot,
        productVersion: cli.productVersion,
        executableSha256: cli.exeSha256,
      });
    },
    /lowercase 64-character hex/,
  );
  const strictLoaderRoot = mkdtempSync(join(tmpdir(), "syndocal-strict-json-loader-"));
  try {
    const canonicalInventoryText = JSON.stringify(inventory);
    const spliceAfterMarker = (text, marker, insertedFragment) => {
      const at = text.indexOf(marker);
      if (at < 0) throw new Error("self-test fixture bug: strict JSON marker not found: " + marker);
      return text.slice(0, at + marker.length) + insertedFragment + text.slice(at + marker.length);
    };
    const writeStrictFixture = (name, text) => {
      const fixturePath = join(strictLoaderRoot, name);
      writeFileSync(fixturePath, text, { flag: "wx" });
      return fixturePath;
    };
    rejects(
      () =>
        loadWindowsRuntimeInventory({
          workspace: strictLoaderRoot,
          inventoryPath: writeStrictFixture(
            "duplicate-top-level.json",
            spliceAfterMarker(canonicalInventoryText, "{", '"schema_version":9,'),
          ),
        }),
      /duplicate object key "schema_version"/,
    );
    rejects(
      () =>
        loadWindowsRuntimeInventory({
          workspace: strictLoaderRoot,
          inventoryPath: writeStrictFixture(
            "duplicate-nested-filename.json",
            spliceAfterMarker(canonicalInventoryText, '"filename":"avcodec-62.dll"', ',"filename":"avcodec-62.dll"'),
          ),
        }),
      /duplicate object key "filename"/,
    );
    pass(
      loadWindowsRuntimeInventory({
        workspace: strictLoaderRoot,
        inventoryPath: writeStrictFixture("canonical-copy.json", canonicalInventoryText),
      }).runtime_dlls.length === 7,
      "an exact canonical inventory copy still loads from a neutral location under the strict parser",
    );
    rejectsCandidate(
      () =>
        assertCandidateArtifactExtractionRequirement(
          writeStrictFixture(
            "candidate-duplicate-product-version.json",
            spliceAfterMarker(
              JSON.stringify({
                schema_version: 1,
                product_version: "1.2.0-rc.1",
                extractions: [
                  { role: "nsis", artifact_path: "Syndocal-setup.exe", artifact_sha256: "a".repeat(64), extracted_root: "nsis", extracted_tree_sha256: "b".repeat(64), method: "repository-owned-deterministic-extractor-v1" },
                  { role: "msi", artifact_path: "Syndocal.msi", artifact_sha256: "c".repeat(64), extracted_root: "msi", extracted_tree_sha256: "d".repeat(64), method: "repository-owned-deterministic-extractor-v1" },
                  { role: "updater", artifact_path: "Syndocal-update.exe", artifact_sha256: "e".repeat(64), extracted_root: "updater", extracted_tree_sha256: "f".repeat(64), method: "repository-owned-deterministic-extractor-v1" },
                ],
              }),
              '"schema_version":1',
              ',"product_version":"9.9.9-rc.9"',
            ),
          ),
        ),
      /duplicate object key "product_version"/,
    );
  } finally {
    rmSync(strictLoaderRoot, { recursive: true, force: true });
  }
  const root = mkdtempSync(join(tmpdir(), "syndocal-windows-artifact-self-test-"));
  try {
    const clean = join(root, "clean");
    mkdirSync(clean);
    const fixturePe = (seed, characteristics = 0x222e) => {
      const bytes = Buffer.alloc(128, seed);
      bytes.write("MZ", 0, "latin1");
      bytes.writeUInt32LE(0x40, 0x3c);
      bytes.write("PE\0\0", 0x40, "latin1");
      bytes.writeUInt16LE(0x8664, 0x44);
      bytes.writeUInt16LE(characteristics, 0x56);
      bytes.writeUInt16LE(0x20b, 0x58);
      return bytes;
    };
    writeFileSync(join(clean, "syndocal.exe"), fixturePe(0, 0x0022), { flag: "wx" });
    rejects(
      () => assertExtractedWindowsArtifactContents(clean, "clean extracted artifact", inventory),
      /missing approved artifact file: avcodec-62\.dll/,
    );
    const cleanInventory = structuredClone(inventory);
    cleanInventory.runtime_dlls = cleanInventory.runtime_dlls.map((runtime, index) => {
      const bytes = fixturePe(index + 1);
      writeFileSync(join(clean, runtime.filename), bytes, { flag: "wx" });
      return { ...runtime, byte_size: bytes.length, sha256: createHash("sha256").update(bytes).digest("hex") };
    });
    const writeCommonResource = (resource) => {
      const source = resolve(workspaceRoot, resource.source.slice("../../".length));
      const record = readVerifiedRegularFile(source, "self-test source common resource", { allowedRoots: [workspaceRoot] });
      const destination = join(clean, resource.destination);
      mkdirSync(dirname(destination), { recursive: true });
      writeFileSync(destination, record.bytes, { flag: "wx" });
      return record.bytes;
    };
    const commonResourceBytes = new Map(
      cleanInventory.common_resources.map((resource) => [resource.destination, writeCommonResource(resource)]),
    );
    const cleanResult = assertExtractedWindowsArtifactContents(clean, "clean extracted artifact", cleanInventory);
    pass(
      cleanResult.fileCount === 12 && cleanResult.ffmpegRuntimeCount === 7 && cleanResult.commonResourceCount === 4,
      "clean materialized artifact passes the exact approved twelve-file tree",
    );

    const notices = cleanInventory.common_resources.find((resource) => resource.destination === "THIRD_PARTY_NOTICES.md");
    const ffmpegLicense = cleanInventory.common_resources.find((resource) => resource.destination === "licenses/FFmpeg-LGPL-3.0.txt");
    assert.ok(notices && ffmpegLicense, "self-test inventory must carry the canonical notices and FFmpeg license");
    const noticesPath = join(clean, notices.destination);
    rmSync(noticesPath);
    rejects(
      () => assertExtractedWindowsArtifactContents(clean, "missing common resource artifact", cleanInventory),
      /missing approved artifact file: THIRD_PARTY_NOTICES\.md/,
    );
    writeFileSync(noticesPath, commonResourceBytes.get(notices.destination), { flag: "wx" });

    const ffmpegLicensePath = join(clean, ffmpegLicense.destination);
    const remappedLicensePath = join(clean, "FFmpeg-LGPL-3.0.txt");
    rmSync(ffmpegLicensePath);
    writeFileSync(remappedLicensePath, commonResourceBytes.get(ffmpegLicense.destination), { flag: "wx" });
    rejects(
      () => assertExtractedWindowsArtifactContents(clean, "remapped common resource artifact", cleanInventory),
      /unapproved artifact file: FFmpeg-LGPL-3\.0\.txt/,
    );
    rmSync(remappedLicensePath);
    writeFileSync(ffmpegLicensePath, commonResourceBytes.get(ffmpegLicense.destination), { flag: "wx" });

    const corruptNotices = Buffer.from(commonResourceBytes.get(notices.destination));
    corruptNotices[0] ^= 0x01;
    writeFileSync(noticesPath, corruptNotices);
    rejects(
      () => assertExtractedWindowsArtifactContents(clean, "corrupt common resource artifact", cleanInventory),
      /common resource THIRD_PARTY_NOTICES\.md SHA-256 does not match pinned inventory/,
    );
    writeFileSync(noticesPath, commonResourceBytes.get(notices.destination));

    const extraFile = join(clean, "unapproved", "nested.txt");
    mkdirSync(dirname(extraFile), { recursive: true });
    writeFileSync(extraFile, "injected", { flag: "wx" });
    rejects(
      () => assertExtractedWindowsArtifactContents(clean, "extra artifact file", cleanInventory),
      /unapproved artifact file: unapproved\/nested\.txt/,
    );
    rmSync(join(clean, "unapproved"), { recursive: true, force: true });

    mkdirSync(join(clean, "unapproved-empty"));
    rejects(
      () => assertExtractedWindowsArtifactContents(clean, "extra artifact directory", cleanInventory),
      /unapproved artifact directory: unapproved-empty/,
    );
    rmSync(join(clean, "unapproved-empty"), { recursive: true, force: true });

    const junctionTarget = join(root, "junction-target");
    mkdirSync(junctionTarget);
    const linkedDir = join(clean, "unapproved-linked-dir");
    let linkedDirCreated = false;
    try {
      symlinkSync(junctionTarget, linkedDir, process.platform === "win32" ? "junction" : "dir");
      linkedDirCreated = true;
    } catch {
      linkedDirCreated = false;
    }
    pass(
      !linkedDirCreated || lstatSync(linkedDir, { bigint: true }).isSymbolicLink(),
      "the self-test link fixture is observed through the same lstat boundary the scanner uses",
    );
    if (linkedDirCreated) {
      rejects(
        () => extractedArtifactDirectories(clean, "symlinked extracted artifact directory scan"),
        /symbolic-link or reparse-point directory.*unapproved-linked-dir/,
      );
      rejects(
        () => assertExtractedWindowsArtifactContents(clean, "symlinked extracted artifact", cleanInventory),
        /symbolic-link or reparse-point/,
      );
    }
    rmSync(linkedDir, { recursive: true, force: true });
    rmSync(junctionTarget, { recursive: true, force: true });

    writeFileSync(join(clean, "third-party-asio-helper.dll"), "injected", { flag: "wx" });
    rejects(
      () => assertExtractedWindowsArtifactContents(clean, "injected extracted artifact", cleanInventory),
      /forbidden ASIO runtime DLL name/,
    );
    rmSync(join(clean, "third-party-asio-helper.dll"), { force: true });
    for (const name of ["syndocal_asio_bridge.dll", "syndocal-asio-bridge.dll", "third-party-asio-helper.dll"]) {
      rejects(
        () => assertNoBlockedAsioPayload(name, Buffer.alloc(0), "injected installer payload", inventory),
        /forbidden ASIO runtime DLL name/,
      );
    }
    for (const hash of inventory.known_asio_bridge_sha256) {
      const bytes = Buffer.from(hash, "utf8");
      const customInventory = { ...inventory, known_asio_bridge_sha256: [createHash("sha256").update(bytes).digest("hex")] };
      rejects(
        () => assertNoBlockedAsioPayload("avcodec-62.dll", bytes, "renamed bridge installer payload", customInventory),
        /known ASIO bridge build/,
      );
    }
    rejects(
      () => validateWindowsCandidateExtractionInventory({ schema_version: 1, product_version: "1.2.0-rc.1", extractions: [] }),
      /keys are not exact/,
    );
    rejects(
      () => validateWindowsCandidateExtractionInventory({ schema_version: 2, product_version: "1.2.0-rc.1", extractions: [] }),
      /keys are not exact|release_evidence/,
    );
    rejects(
      () => validateWindowsCandidateExtractionInventory({
        schema_version: 1,
        product_version: "1.2.0-rc.1",
        extractions: [
          { role: "nsis", artifact_path: "../escape.exe", artifact_sha256: "a".repeat(64), extracted_root: "nsis", extracted_tree_sha256: "b".repeat(64), method: "repository-owned-deterministic-extractor-v1" },
          { role: "msi", artifact_path: "setup.msi", artifact_sha256: "a".repeat(64), extracted_root: "msi", extracted_tree_sha256: "b".repeat(64), method: "repository-owned-deterministic-extractor-v1" },
          { role: "updater", artifact_path: "update.exe", artifact_sha256: "a".repeat(64), extracted_root: "updater", extracted_tree_sha256: "b".repeat(64), method: "repository-owned-deterministic-extractor-v1" },
        ],
      }),
      /keys are not exact/,
    );
    rejectsCandidate(
      () => assertCandidateArtifactExtractionRequirement(),
      /fail-closed/,
    );
    rejectsCandidate(
      () => assertCandidateArtifactExtractionRequirement("relative-candidate-inventory.json"),
      /absolute local-file path/,
    );
    rejectsCandidate(
      () => assertCandidateArtifactExtractionRequirement(join(root, "missing-candidate-inventory.json")),
      /path, file identity, or content is unavailable, stale, unsafe, or invalid/,
    );
    const candidateDirectory = join(root, "candidate-directory");
    mkdirSync(candidateDirectory);
    rejectsCandidate(
      () => assertCandidateArtifactExtractionRequirement(candidateDirectory),
      /path, file identity, or content is unavailable, stale, unsafe, or invalid/,
    );
    const candidateInventoryPath = join(root, "candidate-inventory.json");
    writeFileSync(candidateInventoryPath, "{not valid JSON", { flag: "wx" });
    rejectsCandidate(
      () => assertCandidateArtifactExtractionRequirement(candidateInventoryPath),
      /path, file identity, or content is unavailable, stale, unsafe, or invalid/,
    );
    writeFileSync(candidateInventoryPath, JSON.stringify({ schema_version: 1, product_version: "1.2.0-alpha.11", extractions: [] }));
    rejectsCandidate(
      () => assertCandidateArtifactExtractionRequirement(candidateInventoryPath),
      /path, file identity, or content is unavailable, stale, unsafe, or invalid/,
    );
    writeFileSync(candidateInventoryPath, JSON.stringify({
      schema_version: 1,
      product_version: "1.2.0-rc.1",
      extractions: [
        { role: "nsis", artifact_path: "Syndocal-setup.exe", artifact_sha256: "a".repeat(64), extracted_root: "nsis", extracted_tree_sha256: "b".repeat(64), method: "repository-owned-deterministic-extractor-v1" },
        { role: "msi", artifact_path: "Syndocal.msi", artifact_sha256: "c".repeat(64), extracted_root: "msi", extracted_tree_sha256: "d".repeat(64), method: "repository-owned-deterministic-extractor-v1" },
        { role: "updater", artifact_path: "Syndocal-update.exe", artifact_sha256: "e".repeat(64), extracted_root: "updater", extracted_tree_sha256: "f".repeat(64), method: "repository-owned-deterministic-extractor-v1" },
      ],
    }));
    rejectsCandidate(
      () => assertCandidateArtifactExtractionRequirement(candidateInventoryPath),
      /path, file identity, or content is unavailable, stale, unsafe, or invalid/,
    );

    const candidateWorkspace = join(root, "candidate-workspace");
    const candidateVersion = "1.2.3-rc.1";
    const candidateNames = {
      nsis: "Syndocal_" + candidateVersion + "_x64-setup.exe",
      msi: "Syndocal_" + candidateVersion + "_x64_ja-JP.msi",
    };
    const candidateArtifactDirectories = {
      nsis: join(candidateWorkspace, "target", "release", "bundle", "nsis"),
      msi: join(candidateWorkspace, "target", "release", "bundle", "msi"),
    };
    for (const directory of Object.values(candidateArtifactDirectories)) mkdirSync(directory, { recursive: true });
    const candidateArtifactPaths = {
      nsis: join(candidateArtifactDirectories.nsis, candidateNames.nsis),
      msi: join(candidateArtifactDirectories.msi, candidateNames.msi),
      updater: join(candidateArtifactDirectories.nsis, candidateNames.nsis),
    };
    const candidateArtifactBytes = {
      nsis: Buffer.from("candidate NSIS payload", "utf8"),
      msi: Buffer.from("candidate MSI payload", "utf8"),
    };
    for (const role of ["nsis", "msi"]) writeFileSync(candidateArtifactPaths[role], candidateArtifactBytes[role], { flag: "wx" });
    const candidateCommit = "b".repeat(40);
    const candidateEndpoint = "https://releases.example.invalid/syndocal/beta/latest.json";
    const candidateKeyIdBytes = Buffer.from("bc11804b6c0d8c26", "hex");
    const candidateKeyPair = generateKeyPairSync("ed25519");
    const candidatePublicDer = candidateKeyPair.publicKey.export({ format: "der", type: "spki" });
    const candidateMinisignPacket = Buffer.concat([Buffer.from("Ed", "ascii"), candidateKeyIdBytes, candidatePublicDer.subarray(-32)]);
    const candidateKeyId = Buffer.from(candidateKeyIdBytes).reverse().toString("hex").toUpperCase();
    const candidatePublicKey = Buffer.from(
      Buffer.from(`untrusted comment: minisign public key: ${candidateKeyId}\n${candidateMinisignPacket.toString("base64")}\n`, "utf8").toString("base64"),
      "utf8",
    );
    const makeCandidateSignature = (payload, privateKey, filename, keyIdBytes = candidateKeyIdBytes) => {
      const payloadDigest = createHash("blake2b512").update(payload).digest();
      const payloadSignature = signMessage(null, payloadDigest, privateKey);
      const packet = Buffer.concat([Buffer.from("ED", "ascii"), keyIdBytes, payloadSignature]);
      const trustedComment = "timestamp:1700000000\tfile:" + filename;
      const trustedSignature = signMessage(null, Buffer.concat([payloadSignature, Buffer.from(trustedComment, "utf8")]), privateKey);
      return Buffer.from([
        "untrusted comment: signature from tauri secret key",
        packet.toString("base64"),
        "trusted comment: " + trustedComment,
        trustedSignature.toString("base64"),
      ].join("\n"), "utf8").toString("base64");
    };
    const candidateSignature = Buffer.from(makeCandidateSignature(candidateArtifactBytes.nsis, candidateKeyPair.privateKey, candidateNames.nsis), "utf8");
    writeFileSync(join(candidateArtifactDirectories.nsis, candidateNames.nsis + ".sig"), candidateSignature, { flag: "wx" });
    const candidateEvidenceRoot = join(candidateWorkspace, "qa", "release");
    mkdirSync(candidateEvidenceRoot, { recursive: true });
    writeFileSync(join(candidateEvidenceRoot, "syndocal.pub"), candidatePublicKey, { flag: "wx" });
    writeFileSync(join(candidateEvidenceRoot, candidateNames.nsis + ".sig"), candidateSignature, { flag: "wx" });
    const candidateUpdaterManifest = {
      version: candidateVersion,
      platforms: {
        "windows-x86_64": {
          signature: candidateSignature.toString("utf8"),
          url: candidateEndpoint.replace("latest.json", candidateNames.nsis),
        },
      },
    };
    const candidateUpdaterManifestBytes = Buffer.from(JSON.stringify(candidateUpdaterManifest, null, 2), "utf8");
    writeFileSync(join(candidateEvidenceRoot, "updater.json"), candidateUpdaterManifestBytes, { flag: "wx" });
    const candidateExecutableBytes = readVerifiedRegularFile(join(clean, "syndocal.exe"), "candidate fixture executable", { allowedRoots: [clean] }).bytes;
    const candidateExecutableHash = hashBytes(candidateExecutableBytes);
    const candidateManifest = {
      schemaVersion: 1,
      productVersion: candidateVersion,
      tag: "v" + candidateVersion,
      commit: candidateCommit,
      previousVersion: "1.2.0-alpha.1",
      previousTag: "v1.2.0-alpha.1",
      updater: {
        channel: "beta",
        endpoint: candidateEndpoint,
        manifestPath: "updater.json",
        manifestSha256: hashBytes(candidateUpdaterManifestBytes),
        publicKeyPath: "syndocal.pub",
        publicKeyFingerprint: hashBytes(candidatePublicKey),
      },
      artifacts: [
        { role: "windows-executable", target: "windows-x86_64", filename: "syndocal.exe", path: "syndocal.exe", sha256: candidateExecutableHash },
        { role: "installer", target: "windows-x86_64", filename: candidateNames.nsis, path: candidateNames.nsis, sha256: hashBytes(candidateArtifactBytes.nsis) },
        { role: "installer", target: "windows-x86_64", filename: candidateNames.msi, path: candidateNames.msi, sha256: hashBytes(candidateArtifactBytes.msi) },
        { role: "updater-payload", target: "windows-x86_64", filename: candidateNames.nsis, path: candidateNames.nsis, sha256: hashBytes(candidateArtifactBytes.nsis), signaturePath: candidateNames.nsis + ".sig", signatureSha256: hashBytes(candidateSignature) },
      ],
    };
    writeFileSync(join(candidateEvidenceRoot, "release-evidence.json"), JSON.stringify(candidateManifest, null, 2) + "\n", { flag: "wx" });
    mkdirSync(join(candidateWorkspace, "app", "src-tauri"), { recursive: true });
    writeFileSync(join(candidateWorkspace, "app", "src-tauri", "tauri.updater.conf.json"), JSON.stringify({ bundle: { createUpdaterArtifacts: true } }), { flag: "wx" });
    const candidateOutputRoot = join(candidateWorkspace, WINDOWS_CANDIDATE_EXTRACTION_ROOT, candidateVersion);
    const candidateExtractedRoot = join(candidateOutputRoot, "extracted");
    mkdirSync(candidateExtractedRoot, { recursive: true });
    const candidateReports = [];
    for (const role of ["nsis", "msi", "updater"]) {
      const roleRoot = join(candidateExtractedRoot, role);
      cpSync(clean, roleRoot, { recursive: true });
      const artifactBytes = role === "updater" ? candidateArtifactBytes.nsis : candidateArtifactBytes[role];
      candidateReports.push({
        role,
        artifact_path: slashRelativePath(candidateWorkspace, candidateArtifactPaths[role], "candidate self-test artifact path"),
        artifact_sha256: createHash("sha256").update(artifactBytes).digest("hex"),
        extracted_root: slashRelativePath(candidateWorkspace, roleRoot, "candidate self-test extracted root"),
        extracted_tree_sha256: computeVerifiedCandidateTreeSha256(roleRoot, "candidate self-test extracted tree"),
        signature_path: role === "updater" ? slashRelativePath(candidateWorkspace, join(candidateArtifactDirectories.nsis, candidateNames.nsis + ".sig"), "candidate self-test signature path") : null,
        signature_sha256: role === "updater" ? hashBytes(candidateSignature) : null,
        method: WINDOWS_CANDIDATE_EXTRACTION_METHOD,
      });
    }
    const candidateInventory = {
      schema_version: 2,
      product_version: candidateVersion,
      release_evidence: {
        path: "qa/release/release-evidence.json",
        sha256: hashBytes(readVerifiedRegularFile(join(candidateEvidenceRoot, "release-evidence.json"), "candidate fixture release evidence", { allowedRoots: [candidateWorkspace] }).bytes),
        commit: candidateCommit,
        tag: "v" + candidateVersion,
      },
      extractions: candidateReports,
    };
    const verifiedCandidateInventoryPath = join(candidateOutputRoot, "inventory.json");
    writeFileSync(verifiedCandidateInventoryPath, JSON.stringify(candidateInventory, null, 2) + "\n", { flag: "wx" });
    let candidateInspectorCalls = 0;
    const extractedCandidateExecutablePaths = new Set(
      ["nsis", "msi", "updater"].map((role) => join(candidateExtractedRoot, role, "syndocal.exe")),
    );
    const inspectedMaterializedPaths = [];
    const candidateVerificationOptions = {
      workspace: candidateWorkspace,
      inventoryPath: verifiedCandidateInventoryPath,
      inventory: cleanInventory,
      headCommit: candidateCommit,
      executableInspector: (materializedPath) => {
        if (extractedCandidateExecutablePaths.has(materializedPath)) {
          throw new Error("candidate inspector received an extracted executable path instead of a verified materialized copy.");
        }
        candidateInspectorCalls += 1;
        inspectedMaterializedPaths.push(materializedPath);
        return {
          productVersion: candidateVersion,
          runtimeIdentity: { endpoint: candidateEndpoint, channel: "beta", publicKeyFingerprint: hashBytes(candidatePublicKey) },
        };
      },
    };
    const candidateSignaturePath = join(candidateArtifactDirectories.nsis, candidateNames.nsis + ".sig");
    const candidateEvidenceSignaturePath = join(candidateEvidenceRoot, candidateNames.nsis + ".sig");
    const synchronizeCandidateCryptoFixture = ({ payloadBytes, signatureBytes, publicKeyBytes }) => {
      writeFileSync(candidateArtifactPaths.nsis, payloadBytes);
      writeFileSync(candidateSignaturePath, signatureBytes);
      writeFileSync(candidateEvidenceSignaturePath, signatureBytes);
      writeFileSync(join(candidateEvidenceRoot, "syndocal.pub"), publicKeyBytes);
      const updater = structuredClone(candidateUpdaterManifest);
      updater.platforms["windows-x86_64"].signature = signatureBytes.toString("utf8");
      const updaterBytes = Buffer.from(JSON.stringify(updater, null, 2), "utf8");
      writeFileSync(join(candidateEvidenceRoot, "updater.json"), updaterBytes);
      const manifest = structuredClone(candidateManifest);
      manifest.updater.manifestSha256 = hashBytes(updaterBytes);
      manifest.updater.publicKeyFingerprint = hashBytes(publicKeyBytes);
      const nsisInstaller = manifest.artifacts.find((artifact) => artifact.role === "installer" && artifact.filename === candidateNames.nsis);
      const updaterPayload = manifest.artifacts.find((artifact) => artifact.role === "updater-payload");
      nsisInstaller.sha256 = hashBytes(payloadBytes);
      updaterPayload.sha256 = hashBytes(payloadBytes);
      updaterPayload.signatureSha256 = hashBytes(signatureBytes);
      const evidenceBytes = Buffer.from(JSON.stringify(manifest, null, 2) + "\n", "utf8");
      writeFileSync(join(candidateEvidenceRoot, "release-evidence.json"), evidenceBytes);
      candidateInventory.release_evidence.sha256 = hashBytes(evidenceBytes);
      for (const report of candidateInventory.extractions) {
        if (report.role === "nsis" || report.role === "updater") report.artifact_sha256 = hashBytes(payloadBytes);
        if (report.role === "updater") report.signature_sha256 = hashBytes(signatureBytes);
      }
      writeFileSync(verifiedCandidateInventoryPath, JSON.stringify(candidateInventory, null, 2) + "\n");
    };
    const candidateSummary = assertCandidateArtifactExtractionInventory(candidateInventory, candidateVerificationOptions);
    pass(
      candidateSummary.productVersion === candidateVersion
        && candidateSummary.roles.length === 3
        && inspectedMaterializedPaths.length === 3,
      "repository-owned candidate inventory verifies all payload trees and passes its inspector only materialized executable copies",
    );
    for (const unsafeCandidateChildPath of ["", ".", "nested/../payload", "nested\\payload", "payload:alternate-stream"]) {
      const malformedCandidateInventory = structuredClone(candidateInventory);
      malformedCandidateInventory.extractions[0].artifact_path = unsafeCandidateChildPath;
      rejects(
        () => assertCandidateArtifactExtractionInventory(malformedCandidateInventory, candidateVerificationOptions),
        /exact slash-separated relative path/,
      );
    }
    const tamperedPayloadForCrypto = Buffer.from(candidateArtifactBytes.nsis);
    tamperedPayloadForCrypto[0] ^= 0x01;
    synchronizeCandidateCryptoFixture({ payloadBytes: tamperedPayloadForCrypto, signatureBytes: candidateSignature, publicKeyBytes: candidatePublicKey });
    candidateInspectorCalls = 0;
    rejects(
      () => assertCandidateArtifactExtractionInventory(candidateInventory, candidateVerificationOptions),
      /cryptographic signature verification failed/,
    );
    pass(candidateInspectorCalls === 0, "tampered updater payload is rejected by crypto before any candidate syndocal.exe inspector can run");
    const signatureLines = Buffer.from(candidateSignature.toString("utf8"), "base64").toString("utf8").split("\n");
    const tamperedSignaturePacket = Buffer.from(signatureLines[1], "base64");
    tamperedSignaturePacket[10] ^= 0x01;
    signatureLines[1] = tamperedSignaturePacket.toString("base64");
    const cryptographicallyTamperedSignature = Buffer.from(Buffer.from(signatureLines.join("\n"), "utf8").toString("base64"), "utf8");
    synchronizeCandidateCryptoFixture({ payloadBytes: candidateArtifactBytes.nsis, signatureBytes: cryptographicallyTamperedSignature, publicKeyBytes: candidatePublicKey });
    candidateInspectorCalls = 0;
    rejects(
      () => assertCandidateArtifactExtractionInventory(candidateInventory, candidateVerificationOptions),
      /cryptographic signature verification failed/,
    );
    pass(candidateInspectorCalls === 0, "tampered updater signature is rejected by crypto before any candidate syndocal.exe inspector can run");
    const wrongKeyPair = generateKeyPairSync("ed25519");
    const wrongKeyDer = wrongKeyPair.publicKey.export({ format: "der", type: "spki" });
    const wrongKeyPacket = Buffer.concat([Buffer.from("Ed", "ascii"), candidateKeyIdBytes, wrongKeyDer.subarray(-32)]);
    const wrongPublicKey = Buffer.from(
      Buffer.from(`untrusted comment: minisign public key: ${candidateKeyId}\n${wrongKeyPacket.toString("base64")}\n`, "utf8").toString("base64"),
      "utf8",
    );
    synchronizeCandidateCryptoFixture({ payloadBytes: candidateArtifactBytes.nsis, signatureBytes: candidateSignature, publicKeyBytes: wrongPublicKey });
    candidateInspectorCalls = 0;
    rejects(
      () => assertCandidateArtifactExtractionInventory(candidateInventory, candidateVerificationOptions),
      /cryptographic signature verification failed/,
    );
    pass(candidateInspectorCalls === 0, "wrong updater public key is rejected by crypto before any candidate syndocal.exe inspector can run");
    synchronizeCandidateCryptoFixture({ payloadBytes: candidateArtifactBytes.nsis, signatureBytes: candidateSignature, publicKeyBytes: candidatePublicKey });
    const candidateSignatureOriginal = readVerifiedRegularFile(candidateSignaturePath, "candidate fixture signature", { allowedRoots: [candidateWorkspace] }).bytes;
    writeFileSync(candidateSignaturePath, Buffer.from("wrong signature", "utf8"));
    rejects(
      () => assertCandidateArtifactExtractionInventory(candidateInventory, candidateVerificationOptions),
      /updater signature SHA-256 mismatch/,
    );
    writeFileSync(candidateSignaturePath, candidateSignatureOriginal);
    const candidateUpdaterConfigPath = join(candidateWorkspace, "app", "src-tauri", "tauri.updater.conf.json");
    writeFileSync(candidateUpdaterConfigPath, JSON.stringify({ bundle: { createUpdaterArtifacts: "v1Compatible" } }));
    rejects(
      () => assertCandidateArtifactExtractionInventory(candidateInventory, candidateVerificationOptions),
      /exact versioned Tauri output/,
    );
    writeFileSync(candidateUpdaterConfigPath, JSON.stringify({ bundle: { createUpdaterArtifacts: true } }));
    const candidateUpdaterManifestPath = join(candidateEvidenceRoot, "updater.json");
    const candidateUpdaterManifestOriginal = readVerifiedRegularFile(candidateUpdaterManifestPath, "candidate fixture updater manifest", { allowedRoots: [candidateWorkspace] }).bytes;
    writeFileSync(candidateUpdaterManifestPath, Buffer.from("{}", "utf8"));
    rejects(
      () => assertCandidateArtifactExtractionInventory(candidateInventory, candidateVerificationOptions),
      /updater manifest SHA-256/,
    );
    writeFileSync(candidateUpdaterManifestPath, candidateUpdaterManifestOriginal);
    const candidateEvidencePath = join(candidateEvidenceRoot, "release-evidence.json");
    const candidateEvidenceOriginal = readVerifiedRegularFile(candidateEvidencePath, "candidate fixture evidence", { allowedRoots: [candidateWorkspace] }).bytes;
    rmSync(candidateEvidencePath);
    rejects(
      () => assertCandidateArtifactExtractionInventory(candidateInventory, candidateVerificationOptions),
      /candidate release evidence manifest is missing/,
    );
    writeFileSync(candidateEvidencePath, candidateEvidenceOriginal, { flag: "wx" });
    mkdirSync(join(candidateExtractedRoot, "unexpected-role"));
    rejects(
      () => assertCandidateArtifactExtractionInventory(candidateInventory, candidateVerificationOptions),
      /exactly NSIS, MSI, and updater roots/,
    );
    rmSync(join(candidateExtractedRoot, "unexpected-role"), { recursive: true, force: true });
    const tamperedTreeInventory = structuredClone(candidateInventory);
    tamperedTreeInventory.extractions[0].extracted_tree_sha256 = "a".repeat(64);
    writeFileSync(verifiedCandidateInventoryPath, JSON.stringify(tamperedTreeInventory, null, 2) + "\n");
    rejects(
      () => assertCandidateArtifactExtractionInventory(tamperedTreeInventory, candidateVerificationOptions),
      /extracted tree SHA-256 mismatch/,
    );
    writeFileSync(verifiedCandidateInventoryPath, JSON.stringify(candidateInventory, null, 2) + "\n");
    const tamperedArtifact = Buffer.from(candidateArtifactBytes.nsis);
    tamperedArtifact[0] ^= 0x01;
    writeFileSync(candidateArtifactPaths.nsis, tamperedArtifact);
    rejects(
      () => assertCandidateArtifactExtractionInventory(candidateInventory, candidateVerificationOptions),
      /artifact SHA-256 mismatch/,
    );
    writeFileSync(candidateArtifactPaths.nsis, candidateArtifactBytes.nsis);
    pass(inventory.runtime_dlls.length === 7 && inventory.common_resources.length === 4, "inventory carries exact Windows normal-bundle entries");

    // P2 stale-exe closure: manual three-root mode must never accept syndocal.exe on PE shape alone.
    const authorityProductVersion = "1.2.3-rc.4";
    const canonicalExeBytes = readVerifiedRegularFile(join(clean, "syndocal.exe"), "self-test canonical syndocal.exe", {
      allowedRoots: [clean],
    }).bytes;
    const canonicalExeSha256 = createHash("sha256").update(canonicalExeBytes).digest("hex");
    const variantPe = (seed) => {
      const bytes = Buffer.alloc(128, seed);
      bytes.write("MZ", 0, "latin1");
      bytes.writeUInt32LE(0x40, 0x3c);
      bytes.write("PE\0\0", 0x40, "latin1");
      bytes.writeUInt16LE(0x8664, 0x44);
      bytes.writeUInt16LE(0x222e, 0x56);
      bytes.writeUInt16LE(0x20b, 0x58);
      return bytes;
    };
    const roleDirs = {
      NSIS: join(root, "authority-nsis"),
      MSI: join(root, "authority-msi"),
      Updater: join(root, "authority-updater"),
    };
    for (const directory of Object.values(roleDirs)) cpSync(clean, directory, { recursive: true });
    const stagedReports = new Map();
    const authorityRoles = ["NSIS", "MSI", "Updater"];
    const originalAuthorityExecutablePaths = new Set(authorityRoles.map((role) => join(roleDirs[role], "syndocal.exe")));
    const materializedAuthorityInspectorPaths = [];
    let authorityInspectionIndex = 0;
    const canonicalSelfTestPathKey = (value) => {
      const text = String(value);
      let canonical;
      try {
        canonical = realpathSync(text);
      } catch {
        canonical = resolve(text);
      }
      return process.platform === "win32" ? canonical.toLocaleLowerCase("en-US") : canonical;
    };
    const stageReport = (directory, report) => {
      const role = authorityRoles.find((candidate) => directory === roleDirs[candidate]);
      if (role === undefined) throw new Error("synthetic self-test has no authority role for " + directory);
      stagedReports.set(role, report);
    };
    const stageAllGoodReports = (fileVersion = "1.2.3") => {
      stagedReports.clear();
      for (const directory of Object.values(roleDirs)) {
        stageReport(directory, { productVersion: authorityProductVersion, fileVersion });
      }
    };
    const injectedInspector = {
      kind: "synthetic-self-test-version-info",
      inspect(executablePath) {
        if (originalAuthorityExecutablePaths.has(executablePath)) {
          throw new Error("synthetic self-test inspector received an original extracted executable path instead of a verified materialized copy.");
        }
        materializedAuthorityInspectorPaths.push(executablePath);
        const role = authorityRoles[authorityInspectionIndex];
        authorityInspectionIndex += 1;
        const report = stagedReports.get(role);
        if (report === undefined) throw new Error("synthetic self-test inspector has no staged VersionInfo for materialized " + String(role));
        return { productVersion: report.productVersion, fileVersion: report.fileVersion };
      },
    };
    const runThreeRoots = (overrides = {}) => {
      authorityInspectionIndex = 0;
      return assertManualThreeRootWindowsReleaseArtifacts({
        nsisRoot: roleDirs.NSIS,
        msiRoot: roleDirs.MSI,
        updaterRoot: roleDirs.Updater,
        productVersion: authorityProductVersion,
        executableSha256: canonicalExeSha256,
        inspector: injectedInspector,
        inventory: cleanInventory,
        ...overrides,
      });
    };
    const caseProbePath = join(clean, "syndocal.exe");
    pass(
      process.platform !== "win32"
        || canonicalSelfTestPathKey(caseProbePath) === canonicalSelfTestPathKey(caseProbePath.toLowerCase()),
      "the self-test inspector path key collapses Windows case-divergent spellings of the same staged executable",
    );
    stageAllGoodReports();
    const authoritySummary = runThreeRoots();
    pass(
      authoritySummary.roots.NSIS.sha256 === canonicalExeSha256
        && authoritySummary.roots.MSI.sha256 === canonicalExeSha256
        && authoritySummary.roots.Updater.sha256 === canonicalExeSha256
        && authoritySummary.inspectorKind === "synthetic-self-test-version-info"
        && authoritySummary.productVersion === authorityProductVersion
        && materializedAuthorityInspectorPaths.length === 3
        && materializedAuthorityInspectorPaths.every((path) => !originalAuthorityExecutablePaths.has(path)),
      "exact positive: all three root exes match the authoritative version, hash, and AMD64 PE identity through an injected inspector that receives only materialized copies",
    );
    writeFileSync(join(roleDirs.NSIS, "uninstall.exe"), Buffer.from("synthetic NSIS uninstaller", "utf8"), { flag: "wx" });
    authorityInspectionIndex = 0;
    const installerSummary = assertManualTwoRootWindowsInstallerArtifacts({
      nsisRoot: roleDirs.NSIS,
      msiRoot: roleDirs.MSI,
      productVersion: authorityProductVersion,
      executableSha256: canonicalExeSha256,
      inspector: injectedInspector,
      inventory: cleanInventory,
    });
    pass(
      installerSummary.roots.NSIS.sha256 === canonicalExeSha256
        && installerSummary.roots.MSI.sha256 === canonicalExeSha256
        && installerSummary.productVersion === authorityProductVersion,
      "two-root installer smoke mode verifies NSIS/MSI trees against the same executable authority",
    );
    stageAllGoodReports(authorityProductVersion);
    authorityInspectionIndex = 0;
    const prereleaseInstallerSummary = assertManualTwoRootWindowsInstallerArtifacts({
      nsisRoot: roleDirs.NSIS,
      msiRoot: roleDirs.MSI,
      productVersion: authorityProductVersion,
      executableSha256: canonicalExeSha256,
      inspector: injectedInspector,
      inventory: cleanInventory,
    });
    pass(
      prereleaseInstallerSummary.roots.NSIS.fileVersion === authorityProductVersion
        && prereleaseInstallerSummary.roots.MSI.fileVersion === authorityProductVersion,
      "two-root installer smoke mode accepts the exact canonical pre-release FileVersion while retaining product identity checks",
    );
    rmSync(join(roleDirs.NSIS, "uninstall.exe"));
    stageAllGoodReports("1.2.3.0");
    runThreeRoots();
    pass(true, "the exact positive also accepts the stamped FileVersion quad companion of the canonical SemVer");
    stageAllGoodReports();
    for (const [path] of stagedReports) stagedReports.set(path, { productVersion: "1.2.2", fileVersion: "1.2.2" });
    rejects(
      runThreeRoots,
      /NSIS extracted artifact syndocal\.exe ProductVersion "1\.2\.2" does not match the authoritative product SemVer 1\.2\.3-rc\.4/,
    );
    stageAllGoodReports();
    stageReport(roleDirs.Updater, { productVersion: "9.9.9-canary.1", fileVersion: "9.9.9" });
    rejects(runThreeRoots, /Updater extracted artifact syndocal\.exe ProductVersion "9\.9\.9-canary\.1"/);
    stageAllGoodReports();
    writeFileSync(join(roleDirs.NSIS, "syndocal.exe"), variantPe(0x5a));
    rejects(runThreeRoots, /NSIS extracted artifact syndocal\.exe SHA-256 [0-9a-f]{64} does not match the authoritative release hash/);
    writeFileSync(join(roleDirs.NSIS, "syndocal.exe"), canonicalExeBytes);
    stageAllGoodReports();
    writeFileSync(join(roleDirs.Updater, "syndocal.exe"), variantPe(0x33));
    rejects(runThreeRoots, /Updater extracted artifact syndocal\.exe SHA-256 [0-9a-f]{64} does not match the authoritative release hash/);
    writeFileSync(join(roleDirs.Updater, "syndocal.exe"), canonicalExeBytes);
    stageAllGoodReports();
    stageReport(roleDirs.Updater, { productVersion: "1.2.3-rc.3", fileVersion: "1.2.3" });
    rejects(runThreeRoots, /Updater extracted artifact syndocal\.exe ProductVersion "1\.2\.3-rc\.3" does not match the authoritative product SemVer/);
    stageAllGoodReports();
    stageReport(roleDirs.MSI, { productVersion: authorityProductVersion, fileVersion: "0.0.0" });
    rejects(runThreeRoots, /MSI extracted artifact syndocal\.exe FileVersion "0\.0\.0" is not the exact Windows-stampable companion/);
    const fileVersionProbeLabel = "hostile file-version probe";
    for (const good of [
      ["1.2.3", "1.2.3-rc.4"],
      ["1.2.3.0", "1.2.3-rc.4"],
      ["1.2.3", "1.2.3"],
      ["1.2.3.0", "1.2.3"],
      ["0.0.0", "0.0.0"],
      ["0.0.0.0", "0.0.0"],
      ["65535.65535.65535", "65535.65535.65535"],
      ["65535.65535.65535.0", "65535.65535.65535"],
    ]) {
      assertCompatibleWindowsFileVersion(good[0], good[1], fileVersionProbeLabel);
      assertions += 1;
    }
    for (const hostile of [
      "1.2.3-rc.4",
      "1.2.2",
      "1.2.4",
      "1.2.3.1",
      "1.2.3.00",
      "1.2.3.0.0",
      "1.2.3.",
      "01.2.3",
      "1.02.3.0",
      ".1.2.3",
      "1..3",
      "v1.2.3",
      "1.2.3 ",
      "",
      "65536.0.0",
      "70000.0.0",
      "99999999999999999999.0.0",
      "65536.0.0.0",
    ]) {
      rejects(
        () => assertCompatibleWindowsFileVersion(hostile, hostile.includes("rc") ? "1.2.3-rc.4" : "1.2.3", fileVersionProbeLabel),
        /exact Windows-stampable companion/,
      );
    }
    for (const unstampedCore of ["65536.0.0", "70000.0.0", "99999999999999999999.0.0"]) {
      rejects(
        () => assertCompatibleWindowsFileVersion(unstampedCore, unstampedCore, fileVersionProbeLabel),
        /exact Windows-stampable companion/,
      );
      rejects(
        () => assertCompatibleWindowsFileVersion(unstampedCore + ".0", unstampedCore, fileVersionProbeLabel),
        /exact Windows-stampable companion/,
      );
    }
    rejects(() => parseWindowsReleaseExeAuthority({ product_version: "1.2", executable_sha256: canonicalExeSha256 }), /canonical SemVer/);
    rejects(() => parseWindowsReleaseExeAuthority({ product_version: "v1.2.3-rc.4", executable_sha256: canonicalExeSha256 }), /canonical SemVer/);
    rejects(() => parseWindowsReleaseExeAuthority({ product_version: "01.2.3-rc.4", executable_sha256: canonicalExeSha256 }), /canonical SemVer/);
    rejects(() => parseWindowsReleaseExeAuthority({ product_version: "1.2.3-rc.4+build.1", executable_sha256: canonicalExeSha256 }), /canonical SemVer/);
    rejects(() => parseWindowsReleaseExeAuthority({ product_version: 42, executable_sha256: canonicalExeSha256 }), /canonical SemVer/);
    rejects(() => parseWindowsReleaseExeAuthority({ product_version: "1.2.3-00", executable_sha256: canonicalExeSha256 }), /canonical SemVer/);
    rejects(() => parseWindowsReleaseExeAuthority({ product_version: "1.2.3-01", executable_sha256: canonicalExeSha256 }), /canonical SemVer/);
    rejects(() => parseWindowsReleaseExeAuthority({ product_version: "1.2.3-007", executable_sha256: canonicalExeSha256 }), /canonical SemVer/);
    rejects(() => parseWindowsReleaseExeAuthority({ product_version: "1.2.3-alpha.00", executable_sha256: canonicalExeSha256 }), /canonical SemVer/);
    rejects(() => parseWindowsReleaseExeAuthority({ product_version: "1.2.3-rc.04", executable_sha256: canonicalExeSha256 }), /canonical SemVer/);
    rejects(() => parseWindowsReleaseExeAuthority({ product_version: "1.2.3-0.01", executable_sha256: canonicalExeSha256 }), /canonical SemVer/);
    pass(
      parseWindowsReleaseExeAuthority({ product_version: "1.2.3-0", executable_sha256: canonicalExeSha256 }).productVersion === "1.2.3-0"
        && parseWindowsReleaseExeAuthority({ product_version: "1.2.3-00a", executable_sha256: canonicalExeSha256 }).productVersion === "1.2.3-00a"
        && parseWindowsReleaseExeAuthority({ product_version: "1.2.3-0a.01b-x", executable_sha256: canonicalExeSha256 }).productVersion === "1.2.3-0a.01b-x"
        && parseWindowsReleaseExeAuthority({ product_version: "1.2.3-alpha.0", executable_sha256: canonicalExeSha256 }).productVersion === "1.2.3-alpha.0",
      "canonical SemVer still accepts numeric zero and leading-zero alphanumeric prerelease identifiers while rejecting numeric leading zeros",
    );
    rejects(() => parseWindowsReleaseExeAuthority({ product_version: "1.2.3-rc.4", executable_sha256: canonicalExeSha256.toUpperCase() }), /lowercase 64-character hex/);
    rejects(() => parseWindowsReleaseExeAuthority({ product_version: "1.2.3-rc.4", executable_sha256: canonicalExeSha256.slice(1) }), /lowercase 64-character hex/);
    rejects(() => parseWindowsReleaseExeAuthority({ product_version: "1.2.3-rc.4", executable_sha256: null }), /lowercase 64-character hex/);
    rejects(
      () =>
        parseWindowsReleaseExeAuthority({
          ambient_package_version: "1.2.3",
          executable_sha256: canonicalExeSha256,
          product_version: authorityProductVersion,
        }),
      /keys are not exact/,
    );
    rejects(() => runThreeRoots({ productVersion: undefined, executableSha256: undefined }), /canonical SemVer/);
    rejects(
      () => runThreeRoots({ inspector: null }),
      /SYNDOCAL_WINDOWS_EXE_VERSION_INSPECTOR_UNAVAILABLE.*missing or malformed/,
    );
    rejects(
      () => runThreeRoots({ inspector: {} }),
      /SYNDOCAL_WINDOWS_EXE_VERSION_INSPECTOR_UNAVAILABLE.*missing or malformed/,
    );
    rejects(
      () => runThreeRoots({ inspector: { inspect: "not-a-function" } }),
      /SYNDOCAL_WINDOWS_EXE_VERSION_INSPECTOR_UNAVAILABLE.*missing or malformed/,
    );
    rejects(
      () => runThreeRoots({ inspector: { inspect() { throw new Error("boom"); } } }),
      /SYNDOCAL_WINDOWS_EXE_VERSION_INSPECTION_FAILED.*boom/,
    );
    rejects(
      () => runThreeRoots({ inspector: { inspect: () => ({ productVersion: "", fileVersion: "1.2.3" }) } }),
      /non-empty single-line printable Windows VersionInfo string/,
    );
    rejects(
      () => runThreeRoots({ inspector: { inspect: () => ({ productVersion: "1.2.3\r\n9.9.9", fileVersion: "1.2.3" }) } }),
      /non-empty single-line printable Windows VersionInfo string/,
    );
    rejects(() => validateInspectedWindowsVersionInfo(null), /must be an object/);
    rejects(() => validateInspectedWindowsVersionInfo({ productVersion: "1.2.3" }), /keys are not exact/);
    rejects(() => validateInspectedWindowsVersionInfo({ productVersion: "1.2.3", fileVersion: 7 }), /fileVersion must be a non-empty single-line/);
    rejects(() => parseWindowsVersionInfoDiagnosticOutput(""), /exactly the ProductVersion and FileVersion lines/);
    rejects(() => parseWindowsVersionInfoDiagnosticOutput("1\n2\n3"), /exactly the ProductVersion and FileVersion lines/);
    pass(
      parseWindowsVersionInfoDiagnosticOutput("1.2.3\r\n1.2.3\r\n").fileVersion === "1.2.3",
      "the diagnostic parser accepts CRLF-terminated PowerShell output deterministically",
    );
    const originalPlatform = process.platform;
    Object.defineProperty(process, "platform", { value: "linux", configurable: true });
    try {
      rejects(() => createWindowsFileVersionInfoInspector(), /SYNDOCAL_WINDOWS_EXE_VERSION_INSPECTOR_UNAVAILABLE/);
    } finally {
      Object.defineProperty(process, "platform", { value: originalPlatform, configurable: true });
    }
    pass(
      process.platform !== "win32" || createWindowsFileVersionInfoInspector().kind === "windows-release-diagnostics-powershell-v1",
      "production win32 inspector is explicitly typed-required and spawns no diagnostics until inspect runs",
    );
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
  console.log("Windows release artifact self-test passed: " + assertions + " assertions");
}

function main(argv = process.argv.slice(2)) {
  const cli = parseCli(argv);
  if (cli.selfTest) {
    selfTest();
    return;
  }
  if (cli.requireCandidate) {
    const summary = assertCandidateArtifactExtractionRequirement();
    console.log(
      "Windows candidate artifact extraction passed: "
        + summary.productVersion
        + " with exact NSIS/MSI/updater payload roots, SHA-256 identities, and ASIO-free approved normal-bundle trees ("
        + summary.method
        + ").",
    );
    return;
  }
  if (cli.installerRoots) {
    const summary = assertManualTwoRootWindowsInstallerArtifacts({
      nsisRoot: cli.nsisRoot,
      msiRoot: cli.msiRoot,
      productVersion: cli.productVersion,
      executableSha256: cli.exeSha256,
    });
    console.log(
      "Windows installer smoke artifact acceptance passed: NSIS/MSI syndocal.exe both match product "
        + summary.productVersion
        + ", SHA-256 "
        + summary.executableSha256
        + ", and AMD64 PE32+ identity (inspector "
        + summary.inspectorKind
        + "); plus the exact approved file tree with the NSIS uninstaller allowance and 7 pinned FFmpeg DLLs.",
    );
    return;
  }
  const summary = assertManualThreeRootWindowsReleaseArtifacts({
    nsisRoot: cli.nsisRoot,
    msiRoot: cli.msiRoot,
    updaterRoot: cli.updaterRoot,
    productVersion: cli.productVersion,
    executableSha256: cli.exeSha256,
  });
  console.log(
    "Manual three-root Windows release artifact acceptance passed: NSIS/MSI/updater syndocal.exe all match product "
      + summary.productVersion
      + ", SHA-256 "
      + summary.executableSha256
      + ", and AMD64 PE32+ identity (inspector "
      + summary.inspectorKind
      + "); plus the exact approved file tree with 7 pinned FFmpeg DLLs and 4 pinned common resources per artifact.",
  );
}

if (resolve(process.argv[1] ?? "") === fileURLToPath(import.meta.url)) main();
