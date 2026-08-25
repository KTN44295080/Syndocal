import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { cpSync, lstatSync, mkdtempSync, mkdirSync, readdirSync, realpathSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { dirname, isAbsolute, join, relative, resolve } from "node:path";
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

function assertSyndocalExecutableReleaseIdentity(root, label, authority, inspector) {
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
  const identity = inspector.inspect(record.realPath);
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
  assertCompatibleWindowsFileVersion(identity.fileVersion, authority.productVersion, label);
  return Object.freeze({ sha256: actualSha256, productVersion: identity.productVersion, fileVersion: identity.fileVersion });
}

export function assertExtractedWindowsArtifactContents(root, label, inventory = loadWindowsRuntimeInventory()) {
  const realRoot = assertSafeExternalDirectory(root, label);
  const verifiedInventory = validateWindowsRuntimeInventory(inventory, label + " runtime inventory");
  const approvedFiles = approvedArtifactFiles(verifiedInventory);
  const approvedDirectories = approvedArtifactDirectories(verifiedInventory);
  const filesByRelativePath = new Map();
  for (const file of walkVerifiedFiles(realRoot, label, { inventory: verifiedInventory })) {
    const relativeFile = slashRelativePath(realRoot, file.path, label + " file path");
    if (filesByRelativePath.has(relativeFile)) {
      throw new Error(label + " contains duplicate canonical artifact file path: " + relativeFile);
    }
    filesByRelativePath.set(relativeFile, file);
    if (!approvedFiles.has(relativeFile)) {
      throw new Error(label + " contains an unapproved artifact file: " + relativeFile);
    }
  }
  for (const [relativeFile] of approvedFiles) {
    if (!filesByRelativePath.has(relativeFile)) {
      throw new Error(label + " is missing approved artifact file: " + relativeFile);
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
  exactKeys(value, ["schema_version", "product_version", "extractions"], label);
  if (value.schema_version !== 1 || typeof value.product_version !== "string" || !/^\d+\.\d+\.\d+-rc\.[1-9]\d*$/u.test(value.product_version)) {
    throw new Error(label + " must use schema_version 1 and an RC SemVer product_version.");
  }
  if (!Array.isArray(value.extractions) || value.extractions.length !== 3) {
    throw new Error(label + " must contain exactly NSIS, MSI, and updater extractions.");
  }
  const requiredRoles = new Set(["nsis", "msi", "updater"]);
  const seenRoles = new Set();
  for (const extraction of value.extractions) {
    exactKeys(
      extraction,
      ["role", "artifact_path", "artifact_sha256", "extracted_root", "extracted_tree_sha256", "method"],
      label + " extraction",
    );
    if (!requiredRoles.has(extraction.role) || seenRoles.has(extraction.role)) {
      throw new Error(label + " roles must be the exact NSIS, MSI, updater set.");
    }
    seenRoles.add(extraction.role);
    assertStrictRelativePath(extraction.artifact_path, label + " artifact_path");
    assertStrictRelativePath(extraction.extracted_root, label + " extracted_root");
    if (!/^[0-9a-f]{64}$/u.test(extraction.artifact_sha256) || !/^[0-9a-f]{64}$/u.test(extraction.extracted_tree_sha256)) {
      throw new Error(label + " extraction hashes must be exact lowercase SHA-256 values.");
    }
    if (extraction.method !== "repository-owned-deterministic-extractor-v1") {
      throw new Error(
        label
          + " has no approved repository-owned deterministic installer extractor; do not accept a candidate until method repository-owned-deterministic-extractor-v1 exists and is independently reviewed.",
      );
    }
  }
  return Object.freeze(structuredClone(value));
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
    validateWindowsCandidateExtractionInventory(parseStrictJson(text, "Windows candidate extraction inventory"));
  } catch (error) {
    throw new CandidateArtifactExtractionRequiredError(
      "Release-candidate Windows packaging is fail-closed: extraction inventory path, file identity, or content is unavailable, stale, unsafe, or invalid: "
        + String(error instanceof Error ? error.message : error),
    );
  }
  throw new CandidateArtifactExtractionRequiredError(
    "Release-candidate Windows packaging is fail-closed: no repository-owned deterministic NSIS/MSI/updater extractor is implemented yet. Installing or unpacking arbitrary candidate installers is not accepted as equivalent proof.",
  );
}

function parseCli(argv) {
  if (argv.length === 1 && argv[0] === "--self-test") return { selfTest: true };
  if (argv.length === 1 && argv[0] === "--require-candidate-extraction") return { requireCandidate: true };
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
      + " | --nsis-root <dir> --msi-root <dir> --updater-root <dir> --product-version <canonical-SemVer> --exe-sha256 <64-hex>"
      + " (manual three-root mode requires the explicit canonical product SemVer and executable SHA-256 authority; PE shape alone never accepts syndocal.exe)",
  );
}

function selfTest() {
  const inventory = loadWindowsRuntimeInventory();
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
  const usageAttack = (argv) => rejects(() => parseCli(argv), /Usage: check-windows-release-artifacts\.mjs/u);
  usageAttack([]);
  usageAttack(["--self-test", "--extra"]);
  usageAttack(["--extra", "--self-test"]);
  usageAttack(["--require-candidate-extraction", "--extra"]);
  usageAttack(["--self-test", "--require-candidate-extraction"]);
  usageAttack(happyArgs.slice(0, 8));
  usageAttack([...happyArgs.slice(0, 4), ...happyArgs.slice(6)]);
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
      /exactly NSIS, MSI, and updater/,
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
      /relative path/,
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
      /no repository-owned deterministic NSIS\/MSI\/updater extractor is implemented yet/,
    );
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
      stagedReports.set(canonicalSelfTestPathKey(join(directory, "syndocal.exe")), report);
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
        const report = stagedReports.get(canonicalSelfTestPathKey(executablePath));
        if (report === undefined) throw new Error("synthetic self-test inspector has no staged VersionInfo for " + executablePath);
        return { productVersion: report.productVersion, fileVersion: report.fileVersion };
      },
    };
    const runThreeRoots = (overrides = {}) =>
      assertManualThreeRootWindowsReleaseArtifacts({
        nsisRoot: roleDirs.NSIS,
        msiRoot: roleDirs.MSI,
        updaterRoot: roleDirs.Updater,
        productVersion: authorityProductVersion,
        executableSha256: canonicalExeSha256,
        inspector: injectedInspector,
        inventory: cleanInventory,
        ...overrides,
      });
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
        && authoritySummary.productVersion === authorityProductVersion,
      "exact positive: all three root exes match the authoritative version, hash, and AMD64 PE identity through the injected inspector",
    );
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
    assertCandidateArtifactExtractionRequirement();
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
