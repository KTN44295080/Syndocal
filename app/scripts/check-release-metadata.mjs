import { createHash, createPublicKey, verify as verifySignature } from "node:crypto";
import { execFileSync } from "node:child_process";
import {
  closeSync,
  constants,
  existsSync,
  fstatSync,
  lstatSync,
  mkdtempSync,
  openSync,
  readFileSync,
  realpathSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { basename, dirname, isAbsolute, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import Ajv from "ajv";
import {
  canonicalWindowsFfmpegRuntimeDllNames,
  commonBundleResourceMap,
  loadWindowsRuntimeInventory,
  verifyPinnedCommonResource,
  verifyPinnedRuntimeFile,
  windowsFfmpegBundleResourceMap,
} from "./windows-runtime-inventory.mjs";
import { StrictJsonError, parseStrictJson } from "./strict-json.mjs";

export const expectedVersion = "1.2.0-alpha.17";
const scriptDir = dirname(fileURLToPath(import.meta.url));
const appRoot = resolve(scriptDir, "..");
const workspaceRoot = resolve(appRoot, "..");
const read = (path) => readFileSync(resolve(workspaceRoot, path), "utf8");

export const windowsFfmpegRuntimeDlls = canonicalWindowsFfmpegRuntimeDllNames;

export const blockedAsioRuntimeDlls = Object.freeze([
  "syndocal_asio_bridge.dll",
  "syndocal-asio-bridge.dll",
]);

const asioDllName = /asio.*\.dll$/iu;

function parseRequiredJson(text, label) {
  try {
    return parseStrictJson(text, label);
  } catch (error) {
    throw new Error(`${label} is not valid JSON: ${error instanceof StrictJsonError ? error.detail : error instanceof Error ? error.message : String(error)}`);
  }
}

export function isBlockedAsioRuntimeDllName(name) {
  const normalized = String(name).replaceAll("\\", "/").split("/").at(-1)?.toLocaleLowerCase("en-US") ?? "";
  return blockedAsioRuntimeDlls.includes(normalized) || asioDllName.test(normalized);
}

function assertExactResourceMap(actual, expected, label) {
  if (typeof actual !== "object" || actual === null || Array.isArray(actual)) {
    throw new Error(`${label} must be a JSON object with explicitly mapped resources.`);
  }
  const actualEntries = Object.entries(actual).sort(([left], [right]) => left.localeCompare(right));
  const expectedEntries = Object.entries(expected).sort(([left], [right]) => left.localeCompare(right));
  if (JSON.stringify(actualEntries) !== JSON.stringify(expectedEntries)) {
    throw new Error(`${label} is not the exact approved explicit runtime-resource map.`);
  }
}

const exactAsioSdkPin = Object.freeze({
  schema_version: 2,
  bridge_abi_version: 2,
  canonical_dll_filename: "syndocal_asio_bridge.dll",
  sdk_name: "Steinberg ASIO SDK",
  version: "2.3.4",
  archive_filename: "ASIO-SDK_2.3.4_2025-10-15.zip",
  sha256: "D5EBF0C20DD2C5F43771FD0C1418F4B361BF52434EE670097CFA6B3A335E2ECA",
  archive_root: "ASIOSDK",
  source_page: "https://www.steinberg.net/developers/asiosdk-open/",
  acquisition: "manual",
  distribution_approved: false,
  distribution_gate: "Select and record either a separate GPLv3 distribution path or a signed Steinberg proprietary ASIO SDK agreement before packaging the ASIO artifact.",
});

export function validateAsioSdkPin(sdkPin) {
  if (typeof sdkPin !== "object" || sdkPin === null || Array.isArray(sdkPin)) {
    throw new Error("qa/ASIO_SDK_PIN.json must be an object.");
  }
  const actualKeys = Object.keys(sdkPin).sort();
  const expectedKeys = Object.keys(exactAsioSdkPin).sort();
  if (JSON.stringify(actualKeys) !== JSON.stringify(expectedKeys)) {
    throw new Error("qa/ASIO_SDK_PIN.json keys are not exact; future or retired schema fields are rejected.");
  }
  for (const [key, expected] of Object.entries(exactAsioSdkPin)) {
    if (sdkPin[key] !== expected) {
      throw new Error("qa/ASIO_SDK_PIN.json has unexpected " + key + "; schema, ABI, canonical filename, and distribution boundary are pinned.");
    }
  }
  return Object.freeze(structuredClone(sdkPin));
}

export const requiredPrepareRuntimeLibsCommand = "pnpm --dir app run prepare:runtime-libs";

export class MissingStagedWindowsRuntimeError extends Error {
  constructor(missingPaths, requiredCommand = requiredPrepareRuntimeLibsCommand) {
    super(
      "Windows release metadata verification requires the pinned FFmpeg runtime DLLs staged under target/release before this gate runs, and they are absent from this checkout."
        + ` Run the required prior command exactly as printed, then rerun this check: ${requiredCommand}.`
        + ` Missing exact staged runtime path${missingPaths.length === 1 ? "" : "s"}: ${missingPaths.join("; ")}`,
    );
    this.name = "MissingStagedWindowsRuntimeError";
    this.missingPaths = Object.freeze([...missingPaths]);
    this.requiredCommand = requiredCommand;
  }
}

function assertDefaultBundleDoesNotEnableNdi(readManifest) {
  const cargoManifest = readManifest("app/src-tauri/Cargo.toml");
  if (!/^\s*default\s*=\s*\[\s*"libav"\s*,\s*"spout"\s*\]\s*$/mu.test(cargoManifest)) {
    throw new Error("Default Tauri feature selection must remain exactly libav + spout; NDI-enabled bundling needs a separately licensed overlay.");
  }
  const packageManifest = readManifest("app/package.json");
  const workflow = readManifest(".github/workflows/cross-platform.yml");
  const bundleCommandEnablesNdi = /tauri\s+build[^\r\n]*--features[^\r\n]*\bndi\b/iu;
  if (bundleCommandEnablesNdi.test(packageManifest) || bundleCommandEnablesNdi.test(workflow)) {
    throw new Error("NDI-enabled bundling is forbidden until a separate licensed runtime overlay is proven.");
  }
}

export function validateAsioPackagingBoundary(
  readManifest = read,
  { workspace = workspaceRoot, verifyWindowsRuntimeSources = process.platform === "win32" } = {},
) {
  const inventory = loadWindowsRuntimeInventory({ workspace });
  const sdkPin = validateAsioSdkPin(
    parseRequiredJson(readManifest("qa/ASIO_SDK_PIN.json"), "qa/ASIO_SDK_PIN.json"),
  );
  if (sdkPin.distribution_approved !== false) {
    throw new Error("ASIO distribution_approved must remain exactly false until a separately reviewed artifact workflow is approved.");
  }
  const windowsConfig = parseRequiredJson(
    readManifest("app/src-tauri/tauri.windows.conf.json"),
    "app/src-tauri/tauri.windows.conf.json",
  );
  assertExactResourceMap(
    windowsConfig.bundle?.resources,
    windowsFfmpegBundleResourceMap(inventory),
    "Windows bundle resources",
  );
  const tauri = parseRequiredJson(readManifest("app/src-tauri/tauri.conf.json"), "app/src-tauri/tauri.conf.json");
  assertExactResourceMap(
    tauri.bundle?.resources,
    commonBundleResourceMap(inventory),
    "Main Tauri bundle resources",
  );
  const updater = parseRequiredJson(
    readManifest("app/src-tauri/tauri.updater.conf.json"),
    "app/src-tauri/tauri.updater.conf.json",
  );
  if (Object.hasOwn(updater.bundle ?? {}, "resources")) {
    throw new Error("Updater overlay must not add or remap normal bundle resources.");
  }
  assertDefaultBundleDoesNotEnableNdi(readManifest);

  const tauriRoot = resolve(workspace, "app", "src-tauri");
  for (const resource of inventory.common_resources) {
    verifyPinnedCommonResource(
      resolve(tauriRoot, resource.source),
      resource,
      "Normal bundle resource " + resource.destination,
      { allowedRoots: [workspace] },
    );
  }
  if (verifyWindowsRuntimeSources) {
    if (process.platform === "win32") {
      const missingStagedRuntimePaths = inventory.runtime_dlls
        .map((runtime) => resolve(tauriRoot, "../../target/release/" + runtime.filename))
        .filter((stagedPath) => !existsSync(stagedPath));
      if (missingStagedRuntimePaths.length > 0) {
        throw new MissingStagedWindowsRuntimeError(missingStagedRuntimePaths);
      }
    }
    for (const runtime of inventory.runtime_dlls) {
      verifyPinnedRuntimeFile(
        resolve(tauriRoot, "../../target/release/" + runtime.filename),
        runtime,
        "Windows FFmpeg bundle resource " + runtime.filename,
        { allowedRoots: [resolve(workspace, "target")] },
      );
    }
  }
}

export const requiredWorkspaceMembers = Object.freeze([
  "crates/audio",
  "crates/protocol",
  "crates/gdtf",
  "crates/io",
  "crates/engine",
  "crates/video",
  "crates/visualizer",
  "app/src-tauri",
]);

export const authoritativeMemberPackageNames = Object.freeze({
  "crates/audio": "audio",
  "crates/protocol": "protocol",
  "crates/gdtf": "gdtf",
  "crates/io": "io",
  "crates/engine": "engine",
  "crates/video": "video",
  "crates/visualizer": "visualizer",
  "app/src-tauri": "syndocal",
});

export function windowsInstallerNames(productVersion) {
  return {
    nsis: `Syndocal_${productVersion}_x64-setup.exe`,
    msi: `Syndocal_${productVersion}_x64_ja-JP.msi`,
  };
}

export function readmeProductLine(productVersion) {
  return `- 製品名: **Syndocal ${productVersion}**`;
}

export function readmeWindowsInstallerLine(productVersion) {
  const { nsis, msi } = windowsInstallerNames(productVersion);
  return `- Windows: \`${nsis}\` (NSIS)、\`${msi}\``;
}

export function stripTomlComment(line) {
  let stripped = "";
  let inBasicString = false;
  for (let index = 0; index < line.length; index += 1) {
    const character = line[index];
    if (inBasicString) {
      stripped += character;
      if (character === "\\") {
        index += 1;
        if (index < line.length) stripped += line[index];
      } else if (character === '"') {
        inBasicString = false;
      }
      continue;
    }
    if (character === '"') {
      inBasicString = true;
      stripped += character;
      continue;
    }
    if (character === "#") break;
    stripped += character;
  }
  return stripped;
}

export function parseTomlSections(text) {
  const sections = [{ header: "", lines: [] }];
  for (const rawLine of String(text).split(/\r?\n/u)) {
    const line = stripTomlComment(rawLine);
    const header = /^\s*\[{1,2}\s*([^\]]+?)\s*\]{1,2}\s*$/u.exec(line);
    if (header) {
      sections.push({ header: header[1], lines: [] });
      continue;
    }
    sections.at(-1).lines.push(line);
  }
  return sections;
}

function uniqueTomlSection(sections, header, label) {
  const matches = sections.filter((section) => section.header === header);
  if (matches.length !== 1) {
    throw new Error(`${label} declares ${matches.length} active [${header}] sections; exactly one is required.`);
  }
  return matches[0];
}

export function parseWorkspaceMembers(manifestText) {
  const workspace = uniqueTomlSection(parseTomlSections(manifestText), "workspace", "Root Cargo.toml");
  const assignmentCount = workspace.lines.filter((line) => /^\s*members\s*=/u.test(line)).length;
  if (assignmentCount !== 1) {
    throw new Error(
      `Root Cargo.toml [workspace] declares ${assignmentCount} active members assignments; exactly one is required.`,
    );
  }
  const membersBlock = /^\s*members\s*=\s*\[([\s\S]*?)\]/mu.exec(workspace.lines.join("\n"));
  if (!membersBlock) throw new Error("Root Cargo.toml [workspace] lacks an active members array.");
  return [...membersBlock[1].matchAll(/"([^"]+)"/gu)].map((entry) => entry[1]);
}

export function parseLockPackages(lockText) {
  const packages = [];
  let current = null;
  for (const rawLine of String(lockText).split(/\r?\n/u)) {
    const line = stripTomlComment(rawLine);
    if (/^\s*\[\[\s*package\s*\]\]\s*$/u.test(line)) {
      current = {};
      packages.push(current);
      continue;
    }
    if (!current) continue;
    if (/^\s*\[/u.test(line)) {
      current = null;
      continue;
    }
    const field = /^\s*(name|version)\s*=\s*"([^"]*)"\s*$/u.exec(line);
    if (field) {
      if (current[field[1]] !== undefined) {
        throw new Error(
          `Cargo.lock [[package]] entry declares a duplicate ${field[1]} assignment (${current[field[1]]} then ${field[2]}).`,
        );
      }
      current[field[1]] = field[2];
    }
  }
  return packages;
}

export function parseCargoPackageIdentity(manifestText, label) {
  const pkg = uniqueTomlSection(parseTomlSections(manifestText), "package", label);
  const assignments = pkg.lines.map((line) => line.trim()).filter((line) => line !== "");
  const nameMatches = assignments.map((line) => /^name\s*=\s*"([^"]+)"$/u.exec(line)).filter(Boolean);
  if (nameMatches.length === 0) throw new Error(`${label} [package] lacks an exact active name = "..." assignment.`);
  if (nameMatches.length > 1) {
    throw new Error(
      `${label} [package] declares ${nameMatches.length} active name assignments; exactly one is required.`,
    );
  }
  const name = nameMatches[0][1];
  const versionAssignments = assignments.filter((line) => /^version(?:\.workspace)?\s*=/u.test(line));
  if (versionAssignments.length !== 1 || versionAssignments[0] !== "version.workspace = true") {
    throw new Error(`${label} does not inherit the workspace version through exactly one active version.workspace = true assignment.`);
  }
  return name;
}

function assertExactWorkspaceMembers(manifestText) {
  const members = parseWorkspaceMembers(manifestText);
  const counts = new Map();
  for (const member of members) counts.set(member, (counts.get(member) ?? 0) + 1);
  const duplicates = [...counts.entries()].filter(([, count]) => count > 1).map(([member]) => member);
  if (duplicates.length > 0) {
    throw new Error(`Root Cargo.toml declares duplicate workspace members: ${duplicates.join(", ")}`);
  }
  const missing = requiredWorkspaceMembers.filter((member) => !counts.has(member));
  const extra = [...counts.keys()].filter((member) => !requiredWorkspaceMembers.includes(member));
  if (missing.length > 0 || extra.length > 0) {
    throw new Error(
      `Root Cargo.toml workspace members are not the exact authoritative set `
        + `(missing: ${missing.join(", ") || "none"}; unexpected: ${extra.join(", ") || "none"}).`,
    );
  }
}

function assertWorkspacePackageVersion(rootManifest, productVersion) {
  const section = uniqueTomlSection(parseTomlSections(rootManifest), "workspace.package", "Root Cargo.toml");
  const assignments = (section?.lines ?? []).map((line) => line.trim()).filter((line) => line !== "");
  const versionAssignments = assignments.filter((line) => /^version\s*=/u.test(line));
  if (versionAssignments.length !== 1 || versionAssignments[0] !== `version = "${productVersion}"`) {
    throw new Error(`Cargo workspace version is not ${productVersion}.`);
  }
}

function assertLockFirstPartyVersions(lockText, packageNames, productVersion) {
  const byName = new Map();
  for (const record of parseLockPackages(lockText)) {
    if (!record.name || record.version === undefined) {
      throw new Error("Cargo.lock contains a [[package]] entry lacking an exact name/version pair.");
    }
    const entries = byName.get(record.name) ?? [];
    entries.push(record.version);
    byName.set(record.name, entries);
  }
  for (const name of packageNames) {
    const entries = byName.get(name) ?? [];
    if (entries.length === 0) throw new Error(`Cargo.lock is missing a package entry for ${name}.`);
    if (entries.length > 1) {
      throw new Error(`Cargo.lock is ambiguous: ${entries.length} package entries exist for ${name}.`);
    }
    if (entries[0] !== productVersion) {
      throw new Error(`Cargo.lock pins ${name} at ${entries[0]} instead of ${productVersion}.`);
    }
  }
}

function readmeWindowsInstallerCandidates(lines) {
  return lines.filter(
    (line) => line.startsWith("- Windows:") && /Syndocal_|\.(?:exe|msi)\b|\(NSIS\)/u.test(line),
  );
}

function assertReadmeVersionLines(markdown, productVersion) {
  const lines = markdown.split(/\r?\n/u);
  const canonicalProductLine = readmeProductLine(productVersion);
  const productLines = lines.filter((line) => line.startsWith("- 製品名:"));
  if (!productLines.includes(canonicalProductLine)) {
    throw new Error(`README lacks the exact product line '${canonicalProductLine}'.`);
  }
  if (productLines.length !== 1) {
    throw new Error(
      `README declares ${productLines.length} '- 製品名:' product metadata lines; exactly one canonical product line is required.`,
    );
  }
  const { nsis, msi } = windowsInstallerNames(productVersion);
  const installerLines = readmeWindowsInstallerCandidates(lines);
  if (installerLines.length !== 1) {
    throw new Error(
      `README declares ${installerLines.length} Windows installer metadata lines (Syndocal_/.exe/.msi/(NSIS)); exactly one is required.`,
    );
  }
  const windowsLine = installerLines[0];
  if (!windowsLine.includes("(NSIS)")) {
    throw new Error("README lacks the exact Windows installer line for NSIS and MSI artifacts.");
  }
  if (!windowsLine.includes(nsis)) {
    throw new Error(`README Windows installer line does not carry the versioned NSIS name ${nsis}.`);
  }
  if (!windowsLine.includes(msi)) {
    throw new Error(`README Windows installer line does not carry the versioned MSI name ${msi}.`);
  }
  if (windowsLine !== readmeWindowsInstallerLine(productVersion)) {
    throw new Error("README Windows installer line is not the exact canonical versioned line.");
  }
}

export function validateStaticReleaseMetadata(readManifest = read, productVersion = expectedVersion) {
  const appPackage = parseRequiredJson(readManifest("app/package.json"), "app/package.json");
  const tauri = parseRequiredJson(readManifest("app/src-tauri/tauri.conf.json"), "app/src-tauri/tauri.conf.json");
  const updaterOverlay = parseRequiredJson(readManifest("app/src-tauri/tauri.updater.conf.json"), "app/src-tauri/tauri.updater.conf.json");

  validateAsioPackagingBoundary(readManifest);

  if (appPackage.name !== "syndocal" || appPackage.version !== productVersion) {
    throw new Error(`Frontend package metadata is not Syndocal ${productVersion}.`);
  }
  if (tauri.productName !== "Syndocal" || tauri.version !== productVersion || !tauri.bundle?.active) {
    throw new Error("Tauri product/version/bundle metadata is inconsistent.");
  }
  if (tauri.bundle.publisher !== "Seraf()のKTN") {
    throw new Error("Tauri publisher metadata changed unexpectedly.");
  }
  if (!tauri.bundle.fileAssociations?.some((association) => association.ext?.includes("sdc"))) {
    throw new Error("The .sdc project association is missing.");
  }
  for (const icon of tauri.bundle.icon ?? []) {
    if (!existsSync(resolve(appRoot, "src-tauri", icon))) {
      throw new Error(`Configured bundle icon is missing: ${icon}`);
    }
  }

  const updaterDefaults = tauri.plugins?.updater;
  if (
    typeof updaterDefaults !== "object" ||
    updaterDefaults === null ||
    updaterDefaults.pubkey !== "" ||
    !Array.isArray(updaterDefaults.endpoints) ||
    updaterDefaults.endpoints.length !== 0
  ) {
    throw new Error("Default updater metadata must remain disabled and contain no signing key or endpoint.");
  }
  if (updaterOverlay.bundle?.createUpdaterArtifacts !== true) {
    throw new Error("Updater release overlay must enable signed updater artifacts.");
  }

  const bcdecLicenseSource = "../../licenses/bcdec_rs-MIT.txt";
  if (tauri.bundle.resources?.[bcdecLicenseSource] !== "licenses/bcdec_rs-MIT.txt") {
    throw new Error("The bcdec_rs license is not configured as a bundle resource.");
  }
  if (!existsSync(resolve(appRoot, "src-tauri", bcdecLicenseSource))) {
    throw new Error("The configured bcdec_rs license file is missing.");
  }
  if (!existsSync(resolve(workspaceRoot, "qa", "UPDATE_RELEASE_RUNBOOK.md"))) {
    throw new Error("The signed updater release runbook is missing.");
  }

  const rootManifest = readManifest("Cargo.toml");
  assertExactWorkspaceMembers(rootManifest);
  assertWorkspacePackageVersion(rootManifest, productVersion);
  const firstPartyPackageNames = [];
  const declaredFirstPartyNames = new Map();
  for (const member of requiredWorkspaceMembers) {
    if (!Object.hasOwn(authoritativeMemberPackageNames, member)) {
      throw new Error(`No frozen authoritative package name is mapped for workspace member ${member}.`);
    }
    const expectedName = authoritativeMemberPackageNames[member];
    const declaredName = parseCargoPackageIdentity(readManifest(`${member}/Cargo.toml`), `${member}/Cargo.toml`);
    if (declaredName !== expectedName) {
      throw new Error(
        `${member}/Cargo.toml declares package name '${declaredName}' instead of the frozen authoritative name '${expectedName}'.`,
      );
    }
    if (declaredFirstPartyNames.has(declaredName)) {
      throw new Error(
        `Authoritative workspace members ${declaredFirstPartyNames.get(declaredName)} and ${member} declare the duplicate package name ${declaredName}.`,
      );
    }
    declaredFirstPartyNames.set(declaredName, member);
    firstPartyPackageNames.push(declaredName);
  }
  assertLockFirstPartyVersions(readManifest("Cargo.lock"), firstPartyPackageNames, productVersion);
  assertReadmeVersionLines(readManifest("README.md"), productVersion);

  const macBundleScript = readManifest("app/scripts/bundle-macos-runtime.sh");
  if (!macBundleScript.includes(`Syndocal_${productVersion}_$(uname -m).dmg`)) {
    throw new Error("macOS DMG filename does not match the product version.");
  }
  if (!macBundleScript.includes(`-volname 'Syndocal ${productVersion}'`)) {
    throw new Error("macOS DMG volume name does not match the product version.");
  }

  const crossPlatformWorkflow = readManifest(".github/workflows/cross-platform.yml");
  if (!crossPlatformWorkflow.includes(`name: syndocal-${productVersion}-\${{ matrix.os }}`)) {
    throw new Error("Cross-platform artifact name does not match the product version.");
  }
}

function assertStaticReleaseMetadata() {
  validateStaticReleaseMetadata(read);
}

export function parseSemver(version) {
  const match = /^(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?$/.exec(version);
  if (!match) return null;
  const prerelease = match[4]?.split(".") ?? [];
  if (prerelease.some((identifier) => /^\d+$/u.test(identifier) && identifier.length > 1 && identifier.startsWith("0"))) {
    return null;
  }
  return {
    major: BigInt(match[1]),
    minor: BigInt(match[2]),
    patch: BigInt(match[3]),
    prerelease,
    build: match[5]?.split(".") ?? [],
  };
}

export function compareSemver(left, right) {
  const a = parseSemver(left);
  const b = parseSemver(right);
  if (!a || !b) throw new Error("Cannot compare invalid SemVer values.");
  for (const key of ["major", "minor", "patch"]) {
    if (a[key] !== b[key]) return a[key] < b[key] ? -1 : 1;
  }
  if (a.prerelease.length === 0 || b.prerelease.length === 0) {
    return a.prerelease.length === b.prerelease.length ? 0 : a.prerelease.length === 0 ? 1 : -1;
  }
  for (let index = 0; index < Math.max(a.prerelease.length, b.prerelease.length); index += 1) {
    const x = a.prerelease[index];
    const y = b.prerelease[index];
    if (x === undefined || y === undefined) return x === undefined ? -1 : 1;
    if (x === y) continue;
    const xNumeric = /^\d+$/.test(x);
    const yNumeric = /^\d+$/.test(y);
    if (xNumeric && yNumeric) return BigInt(x) < BigInt(y) ? -1 : 1;
    if (xNumeric !== yNumeric) return xNumeric ? -1 : 1;
    return x < y ? -1 : 1;
  }
  return 0;
}

function semverPrecedenceKey(version) {
  const parsed = parseSemver(version);
  if (!parsed) throw new Error(`Cannot key invalid SemVer value: ${version}`);
  const core = `${parsed.major}.${parsed.minor}.${parsed.patch}`;
  return parsed.prerelease.length > 0 ? `${core}-${parsed.prerelease.join(".")}` : core;
}

function git(args, cwd = workspaceRoot) {
  return execFileSync("git", args, {
    cwd,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  }).trim();
}

function fileIdentity(stats) {
  return `${stats.dev}:${stats.ino}:${stats.size}:${stats.mtimeNs}`;
}

function canonicalPathKey(path) {
  return process.platform === "win32" ? path.toLocaleLowerCase("en-US") : path;
}

export function readVerifiedEvidenceFile(evidenceRoot, relativePath, label = "evidence file") {
  if (typeof relativePath !== "string" || isAbsolute(relativePath) || relativePath.split(/[\\/]/u).includes("..")) {
    throw new Error(`${label} path must stay below the evidence directory: ${relativePath}`);
  }
  const realRoot = realpathSync(evidenceRoot);
  const candidate = resolve(realRoot, relativePath);
  if (!existsSync(candidate)) throw new Error(`${label} is missing: ${relativePath}`);
  const linkStats = lstatSync(candidate, { bigint: true });
  if (linkStats.isSymbolicLink()) throw new Error(`${label} must not be a symbolic link: ${relativePath}`);
  const realCandidate = realpathSync(candidate);
  const outside = relative(realRoot, realCandidate);
  if (outside === "" || outside.startsWith("..") || isAbsolute(outside)) {
    throw new Error(`${label} path escapes the evidence directory: ${relativePath}`);
  }

  const noFollow = constants.O_NOFOLLOW ?? 0;
  const descriptor = openSync(realCandidate, constants.O_RDONLY | noFollow);
  try {
    const before = fstatSync(descriptor, { bigint: true });
    if (!before.isFile()) throw new Error(`${label} is not a regular file: ${relativePath}`);
    const bytes = readFileSync(descriptor);
    const after = fstatSync(descriptor, { bigint: true });
    if (fileIdentity(before) !== fileIdentity(after)) {
      throw new Error(`${label} changed while it was being read: ${relativePath}`);
    }
    if (realpathSync(candidate) !== realCandidate || lstatSync(candidate).isSymbolicLink()) {
      throw new Error(`${label} path changed while it was being read: ${relativePath}`);
    }
    return {
      bytes,
      realPath: realCandidate,
      identity: canonicalPathKey(realCandidate),
      storageIdentity: `${before.dev}:${before.ino}`,
    };
  } finally {
    closeSync(descriptor);
  }
}

export function collectGitCandidateState(repoRoot, currentTag, previousTag) {
  return {
    headCommit: git(["rev-parse", "HEAD"], repoRoot),
    clean: git(["status", "--porcelain=v1"], repoRoot) === "",
    currentTagCommit: git(["rev-parse", `refs/tags/${currentTag}^{commit}`], repoRoot),
    previousTagCommit: git(["rev-parse", `refs/tags/${previousTag}^{commit}`], repoRoot),
    productTags: git(["tag", "--list", "v*"], repoRoot).split(/\r?\n/u).filter(Boolean),
  };
}

function includesUtf8OrUtf16(bytes, text) {
  return bytes.includes(Buffer.from(text, "utf8")) || bytes.includes(Buffer.from(text, "utf16le"));
}

export function parseRuntimeUpdaterIdentity(output) {
  const trimmed = String(output).trim();
  if (!trimmed || /[\r\n]/u.test(trimmed)) {
    throw new Error("Windows executable updater diagnostic must be exactly one JSON line.");
  }
  let identity;
  try {
    identity = parseStrictJson(trimmed, "Windows executable updater diagnostic");
  } catch (error) {
    throw new Error(
      "Windows executable updater diagnostic is not valid JSON: "
        + (error instanceof StrictJsonError ? error.detail : error instanceof Error ? error.message : String(error)),
    );
  }
  if (typeof identity !== "object" || identity === null || Array.isArray(identity)) {
    throw new Error("Windows executable updater diagnostic is not an object.");
  }
  const keys = Object.keys(identity).sort();
  if (JSON.stringify(keys) !== JSON.stringify(["channel", "endpoint", "publicKeyFingerprint"])) {
    throw new Error("Windows executable updater diagnostic fields are not exact.");
  }
  if (
    typeof identity.endpoint !== "string" ||
    typeof identity.channel !== "string" ||
    typeof identity.publicKeyFingerprint !== "string" ||
    !/^[0-9a-f]{64}$/u.test(identity.publicKeyFingerprint)
  ) {
    throw new Error("Windows executable updater diagnostic field values are invalid.");
  }
  return identity;
}

export function withMaterializedVerifiedExecutable(bytes, inspect) {
  if (!Buffer.isBuffer(bytes) || bytes.length === 0) {
    throw new Error("Windows executable evidence bytes are empty.");
  }
  const directory = mkdtempSync(join(tmpdir(), "syndocal-release-inspect-"));
  const executablePath = join(directory, "syndocal-inspect.exe");
  try {
    writeFileSync(executablePath, bytes, { flag: "wx", mode: 0o700 });
    const materialized = readFileSync(executablePath);
    if (!materialized.equals(bytes)) {
      throw new Error("Materialized Windows executable differs from verified evidence bytes.");
    }
    return inspect(executablePath);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
}

function inspectWindowsExecutable(_path, bytes, updater) {
  if (process.platform !== "win32") {
    throw new Error("Windows executable evidence must be inspected on Windows.");
  }
  if (!includesUtf8OrUtf16(bytes, updater.endpoint)) {
    throw new Error("Windows executable does not contain the exact updater endpoint.");
  }
  if (!includesUtf8OrUtf16(bytes, updater.publicKey)) {
    throw new Error("Windows executable does not contain the exact updater public key.");
  }
  const { productVersion, runtimeIdentity } = withMaterializedVerifiedExecutable(bytes, (executablePath) => {
    const productVersion = execFileSync(
      "powershell.exe",
      [
        "-NoLogo",
        "-NoProfile",
        "-NonInteractive",
        "-Command",
        "$v=(Get-Item -LiteralPath $env:SYNDOCAL_RELEASE_INSPECT_PATH).VersionInfo.ProductVersion;[Console]::Out.Write($v)",
      ],
      {
        encoding: "utf8",
        stdio: ["ignore", "pipe", "pipe"],
        env: { ...process.env, SYNDOCAL_RELEASE_INSPECT_PATH: executablePath },
      },
    ).trim();
    const runtimeIdentity = parseRuntimeUpdaterIdentity(execFileSync(
      executablePath,
      ["--print-updater-release-identity"],
      {
        encoding: "utf8",
        stdio: ["ignore", "pipe", "pipe"],
        timeout: 10_000,
        windowsHide: true,
      },
    ));
    return { productVersion, runtimeIdentity };
  });
  if (
    runtimeIdentity.endpoint !== updater.endpoint ||
    runtimeIdentity.channel !== updater.channel ||
    runtimeIdentity.publicKeyFingerprint !== updater.publicKeyFingerprint
  ) {
    throw new Error("Windows executable runtime updater identity does not match release evidence.");
  }
  return { productVersion, runtimeIdentity };
}

export function validateCandidateEvidence(manifest, options) {
  const {
    manifestPath,
    manifestRecord,
    productVersion,
    headCommit,
    clean,
    resolveTag,
    productTags,
    currentTagCommit = resolveTag(manifest.tag),
    previousTagCommit = resolveTag(manifest.previousTag),
    readEvidenceFile = readVerifiedEvidenceFile,
    inspectExecutable = inspectWindowsExecutable,
  } = options;

  if (!/^\d+\.\d+\.\d+-rc\.[1-9]\d*$/.test(productVersion)) {
    throw new Error("Release-candidate mode requires a product version ending in -rc.N.");
  }
  if (manifest.productVersion !== productVersion || manifest.tag !== `v${productVersion}`) {
    throw new Error("Evidence manifest version/tag does not match product metadata.");
  }
  if (!clean) throw new Error("Release-candidate evidence requires a clean worktree.");
  if (manifest.commit !== headCommit) throw new Error("Evidence manifest commit does not match HEAD.");
  if (currentTagCommit !== headCommit) {
    throw new Error("Release tag does not resolve to the exact HEAD commit.");
  }
  if (manifest.previousTag !== `v${manifest.previousVersion}`) {
    throw new Error("Previous version/tag pair is inconsistent.");
  }
  if (compareSemver(manifest.previousVersion, productVersion) >= 0) {
    throw new Error("Product version must increase monotonically from the previous release.");
  }

  const previousCandidates = productTags
    .filter((tag) => tag.startsWith("v") && parseSemver(tag.slice(1)))
    .map((tag) => tag.slice(1))
    .filter((version) => compareSemver(version, productVersion) < 0);
  const previousGroups = new Map();
  for (const version of previousCandidates) {
    const key = semverPrecedenceKey(version);
    const group = previousGroups.get(key) ?? [];
    group.push(version);
    previousGroups.set(key, group);
  }
  const ambiguousPrevious = [...previousGroups.values()].find((group) => group.length > 1);
  if (ambiguousPrevious) {
    throw new Error(`Ambiguous equivalent previous product tags: ${ambiguousPrevious.join(", ")}`);
  }
  previousCandidates.sort(compareSemver);
  const latestPrevious = previousCandidates.at(-1);
  if (!latestPrevious || latestPrevious !== manifest.previousVersion) {
    throw new Error("Evidence manifest does not name the latest previous product tag.");
  }
  if (manifest.previousTag === manifest.tag || previousTagCommit === headCommit) {
    throw new Error("Previous release tag must resolve to a different commit.");
  }

  const endpoint = new URL(manifest.updater.endpoint);
  if (endpoint.protocol !== "https:" || endpoint.username || endpoint.password || endpoint.search || endpoint.hash) {
    throw new Error("Updater endpoint must be credential-free HTTPS.");
  }
  const endpointSegments = endpoint.pathname.split("/").filter(Boolean).map(decodeURIComponent);
  if (!endpointSegments.includes(manifest.updater.channel)) {
    throw new Error("Updater endpoint path does not contain its exact channel segment.");
  }
  if (/PRIVATE KEY|TAURI_SIGNING_PRIVATE_KEY|BEGIN [A-Z ]*PRIVATE/iu.test(JSON.stringify(manifest))) {
    throw new Error("Release evidence must never contain a private signing key.");
  }

  const evidenceRoot = realpathSync(dirname(manifestPath));
  const occupiedFiles = new Set(
    manifestRecord ? [manifestRecord.identity, manifestRecord.storageIdentity].filter(Boolean) : [],
  );
  const addEvidenceFile = (path, label) => {
    const record = readEvidenceFile(evidenceRoot, path, label);
    const recordIdentities = [record.identity, record.storageIdentity].filter(Boolean);
    if (recordIdentities.some((identity) => occupiedFiles.has(identity))) {
      throw new Error(`${label} duplicates another evidence file: ${path}`);
    }
    for (const identity of recordIdentities) occupiedFiles.add(identity);
    return record;
  };
  const updaterManifestRecord = addEvidenceFile(manifest.updater.manifestPath, "updater manifest");
  const updaterManifestHash = createHash("sha256").update(updaterManifestRecord.bytes).digest("hex");
  if (updaterManifestHash !== manifest.updater.manifestSha256.toLowerCase()) {
    throw new Error("Updater manifest SHA-256 mismatch.");
  }
  let updaterManifest;
  try {
    updaterManifest = parseStrictJson(updaterManifestRecord.bytes.toString("utf8"), "Updater manifest");
  } catch (error) {
    throw new Error(
      "Updater manifest is not valid JSON: "
        + (error instanceof StrictJsonError ? error.detail : error instanceof Error ? error.message : String(error)),
    );
  }
  if (updaterManifest.version !== productVersion || typeof updaterManifest.platforms !== "object" || updaterManifest.platforms === null) {
    throw new Error("Updater manifest version/platforms do not match the release candidate.");
  }

  const publicKeyRecord = addEvidenceFile(manifest.updater.publicKeyPath, "updater public key");
  const publicKey = publicKeyRecord.bytes.toString("utf8").trim();
  const privateKeyPattern = /PRIVATE KEY|SECRET KEY|TAURI_SIGNING_PRIVATE_KEY|BEGIN [A-Z ]*PRIVATE/iu;
  if (publicKey.length < 32 || privateKeyPattern.test(publicKey)) {
    throw new Error("Updater public-key evidence is empty or contains private-key material.");
  }
  const strictBase64 = (text, label) => {
    if (
      text.length === 0 ||
      text.length % 4 !== 0 ||
      !/^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/u.test(text)
    ) {
      throw new Error(`${label} is not strict base64.`);
    }
    const decoded = Buffer.from(text, "base64");
    if (decoded.toString("base64") !== text) throw new Error(`${label} is not canonical base64.`);
    return decoded;
  };
  const decodedPublicKey = strictBase64(publicKey, "Updater public-key evidence");
  const publicKeyText = decodedPublicKey.toString("utf8");
  if (!Buffer.from(publicKeyText, "utf8").equals(decodedPublicKey) || privateKeyPattern.test(publicKeyText)) {
    throw new Error("Updater public-key evidence contains private-key material or invalid UTF-8.");
  }
  const publicKeyLines = publicKeyText.replace(/\r\n/gu, "\n").replace(/\n$/u, "").split("\n");
  const commentMatch = /^untrusted comment: minisign public key: ([0-9A-F]{16})$/u.exec(publicKeyLines[0] ?? "");
  if (!commentMatch || publicKeyLines.length !== 2) {
    throw new Error("Updater public-key evidence is not an exact Tauri minisign public key.");
  }
  const minisignPacket = strictBase64(publicKeyLines[1], "Updater minisign public-key packet");
  if (minisignPacket.length !== 42 || minisignPacket.subarray(0, 2).toString("ascii") !== "Ed") {
    throw new Error("Updater public-key evidence is not an exact Tauri minisign public key.");
  }
  const packetKeyId = Buffer.from(minisignPacket.subarray(2, 10)).reverse().toString("hex").toUpperCase();
  if (packetKeyId !== commentMatch[1]) {
    throw new Error("Updater public-key evidence key identifier does not match its minisign packet.");
  }
  const minisignPublicKey = createPublicKey({
    key: Buffer.concat([
      Buffer.from("302a300506032b6570032100", "hex"),
      minisignPacket.subarray(10),
    ]),
    format: "der",
    type: "spki",
  });
  const publicKeyFingerprint = createHash("sha256").update(publicKey, "utf8").digest("hex");
  if (publicKeyFingerprint !== manifest.updater.publicKeyFingerprint.toLowerCase()) {
    throw new Error("Updater public-key fingerprint mismatch.");
  }

  const identities = new Set();
  const updaterPayloadTargets = new Set();
  const executableTargets = new Set();
  for (const artifact of manifest.artifacts) {
    const identity = `${artifact.role}\0${artifact.target}\0${artifact.filename}`;
    if (identities.has(identity)) throw new Error(`Duplicate release artifact identity: ${artifact.filename}`);
    identities.add(identity);
    if (basename(artifact.path) !== artifact.filename) {
      throw new Error(`Artifact filename/path mismatch: ${artifact.filename}`);
    }
    if (artifact.role !== "windows-executable" && !artifact.filename.includes(productVersion)) {
      throw new Error(`Artifact filename does not contain product version: ${artifact.filename}`);
    }
    const artifactRecord = addEvidenceFile(artifact.path, "release artifact");
    const actualHash = createHash("sha256").update(artifactRecord.bytes).digest("hex");
    if (actualHash !== artifact.sha256.toLowerCase()) {
      throw new Error(`Artifact SHA-256 mismatch: ${artifact.filename}`);
    }

    if (artifact.role === "updater-payload") {
      updaterPayloadTargets.add(artifact.target);
      if (!artifact.signaturePath || !artifact.signatureSha256) {
        throw new Error(`Updater payload lacks signature evidence: ${artifact.filename}`);
      }
      const signatureRecord = addEvidenceFile(artifact.signaturePath, "updater signature");
      const signatureHash = createHash("sha256").update(signatureRecord.bytes).digest("hex");
      if (signatureHash !== artifact.signatureSha256.toLowerCase()) {
        throw new Error(`Updater signature SHA-256 mismatch: ${artifact.filename}`);
      }
      const signature = signatureRecord.bytes.toString("utf8").trim();
      const platform = updaterManifest.platforms[artifact.target];
      if (!platform || platform.signature !== signature || signature.length < 32) {
        throw new Error(`Updater manifest signature does not match the exact signature file: ${artifact.target}`);
      }
      const decodedSignature = strictBase64(signature, "Updater signature evidence");
      const signatureText = decodedSignature.toString("utf8");
      if (!Buffer.from(signatureText, "utf8").equals(decodedSignature)) {
        throw new Error(`Updater signature evidence is not valid UTF-8: ${artifact.target}`);
      }
      const signatureLines = signatureText.replace(/\r\n/gu, "\n").replace(/\n$/u, "").split("\n");
      if (
        signatureLines.length !== 4 ||
        signatureLines[0] !== "untrusted comment: signature from tauri secret key" ||
        !signatureLines[2].startsWith("trusted comment: ")
      ) {
        throw new Error(`Updater signature evidence is not an exact Tauri minisign signature: ${artifact.target}`);
      }
      const signaturePacket = strictBase64(signatureLines[1], "Updater minisign signature packet");
      const signatureAlgorithm = signaturePacket.subarray(0, 2).toString("ascii");
      if (
        signaturePacket.length !== 74 ||
        !["ED", "Ed"].includes(signatureAlgorithm) ||
        !signaturePacket.subarray(2, 10).equals(minisignPacket.subarray(2, 10))
      ) {
        throw new Error(`Updater signature key identifier/packet is invalid: ${artifact.target}`);
      }
      const trustedComment = signatureLines[2].slice("trusted comment: ".length);
      const trustedCommentMatch = /^timestamp:[1-9]\d*\tfile:(.+)$/u.exec(trustedComment);
      if (!trustedCommentMatch || trustedCommentMatch[1] !== artifact.filename) {
        throw new Error(`Updater signature trusted filename does not match the artifact: ${artifact.target}`);
      }
      const trustedCommentSignature = strictBase64(
        signatureLines[3],
        "Updater minisign trusted-comment signature",
      );
      if (trustedCommentSignature.length !== 64) {
        throw new Error(`Updater trusted-comment signature packet is invalid: ${artifact.target}`);
      }
      const payloadMessage = signatureAlgorithm === "ED"
        ? createHash("blake2b512").update(artifactRecord.bytes).digest()
        : artifactRecord.bytes;
      if (
        !verifySignature(null, payloadMessage, minisignPublicKey, signaturePacket.subarray(10)) ||
        !verifySignature(
          null,
          Buffer.concat([signaturePacket.subarray(10), Buffer.from(trustedComment, "utf8")]),
          minisignPublicKey,
          trustedCommentSignature,
        )
      ) {
        throw new Error(`Updater payload cryptographic signature verification failed: ${artifact.target}`);
      }
      const artifactUrl = new URL(platform.url);
      if (artifactUrl.protocol !== "https:" || artifactUrl.username || artifactUrl.password || artifactUrl.search || artifactUrl.hash) {
        throw new Error(`Updater artifact URL must be credential-free HTTPS: ${artifact.target}`);
      }
      const urlSegments = artifactUrl.pathname.split("/").filter(Boolean).map(decodeURIComponent);
      if (!urlSegments.includes(manifest.updater.channel) || urlSegments.at(-1) !== artifact.filename) {
        throw new Error(`Updater artifact URL channel/filename mismatch: ${artifact.target}`);
      }
    } else if (artifact.signaturePath !== undefined || artifact.signatureSha256 !== undefined) {
      throw new Error(`Only updater payloads may declare updater signature evidence: ${artifact.filename}`);
    }

    if (artifact.role === "windows-executable") {
      executableTargets.add(artifact.target);
      const inspected = inspectExecutable(artifactRecord.realPath, artifactRecord.bytes, {
        endpoint: manifest.updater.endpoint,
        publicKey,
        channel: manifest.updater.channel,
        publicKeyFingerprint,
      });
      if (inspected.productVersion !== productVersion) {
        throw new Error(`Windows executable ProductVersion mismatch: ${artifact.filename}`);
      }
    }
  }

  const platformTargets = Object.keys(updaterManifest.platforms).sort();
  const payloadTargets = [...updaterPayloadTargets].sort();
  if (JSON.stringify(platformTargets) !== JSON.stringify(payloadTargets)) {
    throw new Error("Updater manifest platforms and updater-payload evidence are not an exact set.");
  }
  for (const target of updaterPayloadTargets) {
    if (target.startsWith("windows-") && !executableTargets.has(target)) {
      throw new Error(`Windows updater target lacks an inspected application executable: ${target}`);
    }
  }
}

export function parseCli(argv, productVersion = expectedVersion) {
  if (argv.length === 0) {
    if (/^\d+\.\d+\.\d+-rc\.[1-9]\d*$/u.test(productVersion)) {
      throw new Error("Release-candidate product versions require --release-candidate --manifest evidence.");
    }
    return { candidate: false };
  }
  if (argv.length === 3 && argv[0] === "--release-candidate" && argv[1] === "--manifest") {
    return { candidate: true, manifestPath: isAbsolute(argv[2]) ? resolve(argv[2]) : resolve(workspaceRoot, argv[2]) };
  }
  throw new Error("Usage: check-release-metadata.mjs [--release-candidate --manifest <path>]");
}

function validateCandidateFromCli(manifestPath) {
  if (!existsSync(manifestPath)) throw new Error(`Release evidence manifest is missing: ${manifestPath}`);
  const manifestRecord = readVerifiedEvidenceFile(dirname(manifestPath), basename(manifestPath), "release evidence manifest");
  const manifest = parseRequiredJson(manifestRecord.bytes.toString("utf8"), "Release evidence manifest");
  const schema = parseRequiredJson(read("qa/release/release-evidence-manifest.schema.json"), "Release evidence manifest schema");
  const ajv = new Ajv({ allErrors: true, strict: true });
  const validate = ajv.compile(schema);
  if (!validate(manifest)) {
    throw new Error(`Release evidence schema validation failed: ${ajv.errorsText(validate.errors)}`);
  }
  if (!/^\d+\.\d+\.\d+-rc\.[1-9]\d*$/u.test(expectedVersion)) {
    throw new Error("Release-candidate mode requires a product version ending in -rc.N.");
  }
  const gitState = collectGitCandidateState(workspaceRoot, manifest.tag, manifest.previousTag);
  validateCandidateEvidence(manifest, {
    manifestPath,
    manifestRecord,
    productVersion: expectedVersion,
    headCommit: gitState.headCommit,
    clean: gitState.clean,
    currentTagCommit: gitState.currentTagCommit,
    previousTagCommit: gitState.previousTagCommit,
    resolveTag: (tag) => git(["rev-parse", `refs/tags/${tag}^{commit}`]),
    productTags: gitState.productTags,
  });
}

export function main(argv = process.argv.slice(2)) {
  assertStaticReleaseMetadata();
  const cli = parseCli(argv);
  if (cli.candidate) {
    validateCandidateFromCli(cli.manifestPath);
    console.log(`release candidate evidence ok: Syndocal ${expectedVersion}`);
  } else {
    console.log(`release metadata ok: Syndocal ${expectedVersion} / .sdc / signed updater overlay / Seraf()のKTN`);
  }
}

if (resolve(process.argv[1] ?? "") === fileURLToPath(import.meta.url)) main();
