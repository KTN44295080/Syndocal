import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { spawnSync } from "node:child_process";
import {
  existsSync,
  linkSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  unlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { parseStrictJson } from "./strict-json.mjs";
import { validateAsioPackagingBoundary } from "./check-release-metadata.mjs";
import {
  assertExactWindowsTargetTriple,
  assertSafeExternalDirectory,
  loadWindowsRuntimeInventory,
  readVerifiedRegularFile,
} from "./windows-runtime-inventory.mjs";
import { assertNoNdiEnabledBundling, validatePinnedFfmpegRuntimeDirectory } from "./prepare-release-runtime.mjs";
import {
  requiredMsvcToolchain,
  requireExactMsvcLinkerFirst,
  stopCheckoutReleaseExecutable,
  tauriCommandEnvironment,
} from "./run-tauri.mjs";
import {
  collectShowAsioSourceIdentity,
  computeShowAsioHostBinding,
  expectedShowAsioArtifactRelativeDirectory,
  inspectBridgeExports,
  readWindowsMachineGuid,
  showAsioBridgeExports,
  showAsioFeatures,
} from "./check-show-asio-artifact.mjs";
import {
  prepareShowAsioRuntime,
  readVerifiedCargoBridgeBuildOutput,
} from "./prepare-show-asio-runtime.mjs";

const scriptDir = dirname(fileURLToPath(import.meta.url));
export const workspaceRoot = resolve(scriptDir, "../..");
const appTargetRelative = "target/show-asio-build/app";
const bridgeTargetRelative = "target/show-asio-build/bridge";
const versionPattern = /^[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$/u;
const commitPattern = /^[0-9a-f]{40}$/u;

export class ShowAsioBlockedError extends Error {
  constructor(kind, message) {
    super(message);
    this.name = "ShowAsioBlockedError";
    this.kind = kind;
  }
}

function assertExactKeys(value, expectedKeys, label) {
  if (typeof value !== "object" || value === null || Array.isArray(value)) throw new Error(label + " must be an object.");
  const actual = Object.keys(value).sort();
  const expected = [...expectedKeys].sort();
  if (JSON.stringify(actual) !== JSON.stringify(expected)) {
    throw new Error(label + " keys are not exact (expected: " + expected.join(", ") + "; actual: " + actual.join(", ") + ").");
  }
}

export function parseBuildMode(argv) {
  if (!Array.isArray(argv)) throw new Error("Show-ASIO arguments must be an array.");
  if (argv.length === 0) return "build";
  if (argv.length === 1 && argv[0] === "--plan") return "plan";
  if (argv.length === 1 && argv[0] === "--self-test") return "self-test";
  throw new Error("Usage: build-windows-show-asio.mjs [--plan|--self-test]; feature, target, bundle, signing, and output overrides are forbidden.");
}

function parseWorkspaceVersion(cargoText) {
  const sections = String(cargoText).split(/^\s*(?=\[)/mu);
  const matches = sections.filter((section) => /^\[workspace\.package\]\s*$/mu.test(section));
  if (matches.length !== 1) throw new Error("Cargo.toml must contain one exact [workspace.package] section.");
  const versions = [...matches[0].matchAll(/^version\s*=\s*"([^"]+)"\s*$/gmu)];
  if (versions.length !== 1 || !versionPattern.test(versions[0][1])) throw new Error("Cargo workspace product version is missing, duplicated, or invalid.");
  return versions[0][1];
}

function parseCargoLockSyndocalVersion(lockText) {
  const packages = String(lockText).split(/^\[\[package\]\]\s*$/mu).slice(1);
  const matches = packages.filter((block) => /^name\s*=\s*"syndocal"\s*$/mu.test(block));
  if (matches.length !== 1) throw new Error("Cargo.lock must contain one exact syndocal package entry.");
  const versions = [...matches[0].matchAll(/^version\s*=\s*"([^"]+)"\s*$/gmu)];
  if (versions.length !== 1 || !versionPattern.test(versions[0][1])) throw new Error("Cargo.lock syndocal version is missing, duplicated, or invalid.");
  return versions[0][1];
}

export function validateVersionSet({ workspaceVersion, lockVersion, packageVersion, tauriVersion }) {
  const values = [workspaceVersion, lockVersion, packageVersion, tauriVersion];
  if (values.some((value) => typeof value !== "string" || !versionPattern.test(value))) {
    throw new Error("Show-ASIO product version set contains an invalid SemVer value.");
  }
  if (new Set(values).size !== 1) throw new Error("Show-ASIO product version drift exists across Cargo, lockfile, package, or Tauri metadata.");
  return workspaceVersion;
}

export function readShowAsioProductVersion(workspace = workspaceRoot) {
  const read = (path) => readVerifiedRegularFile(resolve(workspace, path), "Show-ASIO version source " + path, { allowedRoots: [workspace] }).bytes.toString("utf8");
  const packageJson = parseStrictJson(read("app/package.json"), "app/package.json");
  const tauriJson = parseStrictJson(read("app/src-tauri/tauri.conf.json"), "tauri.conf.json");
  return validateVersionSet({
    workspaceVersion: parseWorkspaceVersion(read("Cargo.toml")),
    lockVersion: parseCargoLockSyndocalVersion(read("Cargo.lock")),
    packageVersion: packageJson.version,
    tauriVersion: tauriJson.version,
  });
}

function parseFeatureArray(text, key, label) {
  const escaped = key.replace(/[.*+?^${}()|[\]\\]/gu, "\\$&");
  const matches = [...String(text).matchAll(new RegExp("^" + escaped + "\\s*=\\s*\\[([^\\]]*)\\]\\s*$", "gmu"))];
  if (matches.length !== 1) throw new Error(label + " must define " + key + " exactly once as a literal array.");
  try {
    const parsed = JSON.parse("[" + matches[0][1] + "]");
    if (!Array.isArray(parsed) || parsed.some((value) => typeof value !== "string")) throw new Error("not a string array");
    return parsed;
  } catch (error) {
    throw new Error(label + " " + key + " feature array is not an exact string list: " + String(error));
  }
}

function featureSection(cargoText) {
  const lines = String(cargoText).split(/\r?\n/u);
  const starts = lines.map((line, index) => (line.trim() === "[features]" ? index : -1)).filter((index) => index >= 0);
  if (starts.length !== 1) throw new Error("app/src-tauri/Cargo.toml must contain one [features] section.");
  let end = lines.length;
  for (let index = starts[0] + 1; index < lines.length; index += 1) {
    if (/^\[[^\r\n]+\]$/u.test(lines[index].trim())) {
      end = index;
      break;
    }
  }
  return lines.slice(starts[0] + 1, end).join("\n");
}

export function validateShowAsioFeatureContract(cargoText, requestedFeatures = showAsioFeatures) {
  if (JSON.stringify(requestedFeatures) !== JSON.stringify(showAsioFeatures)) {
    throw new Error("Show-ASIO requested feature union must be exactly libav, spout, show-asio.");
  }
  const section = featureSection(cargoText);
  const defaults = parseFeatureArray(section, "default", "Tauri features");
  if (JSON.stringify(defaults) !== JSON.stringify(["libav", "spout"])) {
    throw new Error("Normal MIT/WASAPI default features must remain exactly libav + spout.");
  }
  if (!/^show-asio\s*=/mu.test(section)) {
    throw new ShowAsioBlockedError(
      "SHOW_ASIO_FEATURE_MISSING",
      "app/src-tauri/Cargo.toml does not yet define the clean-break show-asio feature; Cargo/native execution is blocked before it starts.",
    );
  }
  const showFeature = parseFeatureArray(section, "show-asio", "Tauri features");
  const asioFeature = parseFeatureArray(section, "asio", "Tauri features");
  if (JSON.stringify(showFeature) !== JSON.stringify(["asio"]) || JSON.stringify(asioFeature) !== JSON.stringify(["dep:libloading"])) {
    throw new Error("show-asio must map exactly to asio, and asio exactly to dep:libloading; fallback aliases are rejected.");
  }
  return [...showAsioFeatures];
}

export function validateShowAsioTauriConfig(config) {
  assertExactKeys(config, ["$schema", "productName", "identifier", "plugins", "bundle"], "Show-ASIO Tauri config");
  if (config.$schema !== "https://schema.tauri.app/config/2") throw new Error("Show-ASIO Tauri config schema is not exact.");
  if (config.productName !== "Syndocal Show ASIO Local" || config.identifier !== "jp.seraf.ktn.syndocal.show-asio-local") {
    throw new Error("Show-ASIO Tauri product identity is not exact.");
  }
  assertExactKeys(config.plugins, ["updater"], "Show-ASIO Tauri plugins");
  assertExactKeys(config.plugins.updater, ["pubkey", "endpoints"], "Show-ASIO updater config");
  if (config.plugins.updater.pubkey !== "" || !Array.isArray(config.plugins.updater.endpoints) || config.plugins.updater.endpoints.length !== 0) {
    throw new Error("Show-ASIO updater must remain empty and disabled.");
  }
  assertExactKeys(config.bundle, ["active", "createUpdaterArtifacts"], "Show-ASIO bundle config");
  if (config.bundle.active !== false || config.bundle.createUpdaterArtifacts !== false) {
    throw new Error("Show-ASIO Tauri bundling and updater artifacts must remain disabled.");
  }
  return config;
}

export function validateGitState({ branch, head, upstream, status }) {
  if (typeof branch !== "string" || branch.length === 0 || branch === "HEAD") throw new Error("Show-ASIO build requires one named Git branch, not a detached HEAD.");
  if (!commitPattern.test(head ?? "") || !commitPattern.test(upstream ?? "")) throw new Error("Show-ASIO build requires exact lowercase HEAD and upstream object IDs.");
  if (head !== upstream) throw new Error("Show-ASIO HEAD must equal its upstream before any build starts.");
  if (typeof status !== "string" || status.length !== 0) throw new Error("Show-ASIO build requires a completely clean tracked and untracked worktree.");
  return { branch, head, upstream, status };
}

function runGit(workspace, args, spawn = spawnSync) {
  const result = spawn("git.exe", args, { cwd: workspace, encoding: "utf8", windowsHide: true, maxBuffer: 16 * 1024 * 1024 });
  if (result.error || result.status !== 0) throw new Error("Git preflight failed for " + args.join(" ") + ".");
  return String(result.stdout ?? "").trimEnd();
}

export function captureGitState(workspace = workspaceRoot, spawn = spawnSync) {
  return validateGitState({
    branch: runGit(workspace, ["symbolic-ref", "--quiet", "--short", "HEAD"], spawn).trim(),
    head: runGit(workspace, ["rev-parse", "HEAD"], spawn).trim(),
    upstream: runGit(workspace, ["rev-parse", "@{upstream}"], spawn).trim(),
    status: runGit(workspace, ["status", "--porcelain=v1", "--untracked-files=all"], spawn),
  });
}

export function assertSourceAndGitStable(before, after, sourceBefore, sourceAfter) {
  if (
    before.branch !== after.branch
    || before.head !== after.head
    || before.upstream !== after.upstream
    || after.status !== ""
  ) {
    throw new Error("Show-ASIO Git identity or cleanliness changed during the build.");
  }
  if (JSON.stringify(sourceBefore) !== JSON.stringify(sourceAfter)) {
    throw new Error("Show-ASIO authoritative source bytes changed during the build.");
  }
}

export function assertNoShowAsioEnvironmentEscape(environment) {
  assertNoNdiEnabledBundling(environment);
  assertExactWindowsTargetTriple(environment.TAURI_ENV_TARGET_TRIPLE);
  const forbiddenExact = [
    "CARGO_TARGET_DIR",
    "CARGO_BUILD_TARGET",
    ["CARGO", "ENCODED", "RUSTFLAGS"].join("_"),
    "RUSTFLAGS",
    "TAURI_CONFIG",
    "TAURI_BUNDLE_TARGETS",
    "TAURI_SIGNING_PRIVATE_KEY",
    "TAURI_SIGNING_PRIVATE_KEY_PASSWORD",
    "TAURI_BUNDLE_FEATURES",
    "SYNDOCAL_TAURI_FEATURES",
    "SYNDOCAL_NDI_ENABLED",
  ];
  for (const name of forbiddenExact) {
    if (Object.hasOwn(environment ?? {}, name)) {
      throw new Error("Show-ASIO inherited environment override is forbidden: " + name);
    }
  }
  const allowedNdiDiscoveryVariables = new Set([
    "WINDIR",
    "NDI_SDK_DIR",
    "NDI_RUNTIME_DIR_V2",
    "NDI_RUNTIME_DIR_V3",
    "NDI_RUNTIME_DIR_V4",
    "NDI_RUNTIME_DIR_V5",
    "NDI_RUNTIME_DIR_V6",
  ]);
  for (const [name, value] of Object.entries(environment ?? {})) {
    const upper = name.toUpperCase();
    if (upper.includes("NDI") && !allowedNdiDiscoveryVariables.has(upper)) {
      throw new Error("Show-ASIO inherited NDI signal is forbidden: " + name);
    }
    if (typeof value !== "string" || value.length === 0) continue;
    if (upper.startsWith("CARGO_FEATURE_")) throw new Error("Show-ASIO inherited Cargo feature escape is forbidden: " + name);
    if (upper.includes("TAURI_SIGNING")) throw new Error("Show-ASIO inherited signing escape is forbidden: " + name);
  }
  const required = ["FFMPEG_DIR", "CPAL_ASIO_DIR", "SYNDOCAL_ASIO_SDK_ARCHIVE_PATH", "LIBCLANG_PATH"];
  for (const name of required) {
    if (typeof environment?.[name] !== "string" || environment[name].length === 0 || environment[name] !== environment[name].trim()) {
      throw new Error("Show-ASIO requires one explicit nonblank environment value: " + name);
    }
  }
}

export function assertPreparedLinkerEnvironment(environment, locatedLinkers) {
  const required = requiredMsvcToolchain(environment).linker;
  const pinned = resolve(String(environment.CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER ?? ""));
  if (pinned.toLocaleLowerCase("en-US") !== resolve(required).toLocaleLowerCase("en-US")) {
    throw new Error("Show-ASIO Cargo linker pin is not the exact required MSVC linker.");
  }
  if (!Array.isArray(locatedLinkers) || locatedLinkers.length === 0 || resolve(locatedLinkers[0]).toLocaleLowerCase("en-US") !== resolve(required).toLocaleLowerCase("en-US")) {
    throw new Error("Show-ASIO where.exe linker order does not place the exact required MSVC linker first.");
  }
}

function readStaticContract(workspace) {
  const cargoText = readVerifiedRegularFile(resolve(workspace, "app/src-tauri/Cargo.toml"), "Tauri Cargo.toml", { allowedRoots: [workspace] }).bytes.toString("utf8");
  const showConfigText = readVerifiedRegularFile(resolve(workspace, "app/src-tauri/tauri.show-asio.conf.json"), "Show-ASIO Tauri config", { allowedRoots: [workspace] }).bytes.toString("utf8");
  validateShowAsioTauriConfig(parseStrictJson(showConfigText, "Show-ASIO Tauri config"));
  validateAsioPackagingBoundary(undefined, { workspace, verifyWindowsRuntimeSources: false });
  const version = readShowAsioProductVersion(workspace);
  const features = validateShowAsioFeatureContract(cargoText);
  return { version, features };
}

function runChecked(label, command, args, { cwd, environment, spawn = spawnSync } = {}) {
  const result = spawn(command, args, { cwd, env: environment, stdio: "inherit", windowsHide: true });
  if (result.error) throw new Error(label + " could not start: " + String(result.error));
  if (result.status !== 0) throw new Error(label + " failed with exit code " + String(result.status) + ".");
}

function assertExistingBuildTargetParents(workspace) {
  const candidates = [
    resolve(workspace, "target"),
    resolve(workspace, "target/show-asio-build"),
    resolve(workspace, appTargetRelative),
    resolve(workspace, bridgeTargetRelative),
  ];
  for (const candidate of candidates) {
    if (existsSync(candidate)) assertSafeExternalDirectory(candidate, "Show-ASIO isolated build target");
  }
}

function printAndVerifyLinker(environment) {
  const located = requireExactMsvcLinkerFirst(environment);
  assertPreparedLinkerEnvironment(environment, located);
  console.error("[show-asio-build] pinned CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER=" + environment.CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER);
  console.error("[show-asio-build] where.exe link.exe:\n" + located.join("\n"));
}

export function buildPlan(workspace = workspaceRoot, environment = process.env) {
  const blockers = [];
  let version = null;
  let commit = null;
  try {
    version = readShowAsioProductVersion(workspace);
  } catch (error) {
    blockers.push({ kind: "VERSION_CONTRACT", message: error.message });
  }
  try {
    const cargoText = readFileSync(resolve(workspace, "app/src-tauri/Cargo.toml"), "utf8");
    validateShowAsioFeatureContract(cargoText);
  } catch (error) {
    blockers.push({ kind: error instanceof ShowAsioBlockedError ? error.kind : "FEATURE_CONTRACT", message: error.message });
  }
  try {
    const configText = readFileSync(resolve(workspace, "app/src-tauri/tauri.show-asio.conf.json"), "utf8");
    validateShowAsioTauriConfig(parseStrictJson(configText, "Show-ASIO Tauri config"));
    validateAsioPackagingBoundary(undefined, { workspace, verifyWindowsRuntimeSources: false });
  } catch (error) {
    blockers.push({ kind: "PACKAGING_CONTRACT", message: error.message });
  }
  try {
    assertNoShowAsioEnvironmentEscape(environment);
  } catch (error) {
    blockers.push({ kind: "ENVIRONMENT_CONTRACT", message: error.message });
  }
  try {
    const state = captureGitState(workspace);
    commit = state.head;
  } catch (error) {
    blockers.push({ kind: "GIT_CONTRACT", message: error.message });
    const head = spawnSync("git.exe", ["rev-parse", "HEAD"], { cwd: workspace, encoding: "utf8", windowsHide: true });
    const candidate = String(head.stdout ?? "").trim();
    if (commitPattern.test(candidate)) commit = candidate;
  }
  const finalArtifact = version && commit ? expectedShowAsioArtifactRelativeDirectory(version, commit) : null;
  if (finalArtifact && existsSync(resolve(workspace, finalArtifact))) {
    blockers.push({ kind: "FINAL_ARTIFACT_EXISTS", message: "The exact final artifact directory already exists and cannot be overwritten." });
  }
  return {
    mode: "plan",
    status: blockers.length === 0 ? "READY" : "BLOCKED",
    cargoInvoked: false,
    nativeInvoked: false,
    processStopInvoked: false,
    version,
    commit,
    features: [...showAsioFeatures],
    applicationTarget: appTargetRelative,
    bridgeTarget: bridgeTargetRelative,
    finalArtifact,
    blockers,
  };
}

export function buildShowAsio({ workspace = workspaceRoot, environment = process.env, platform = process.platform } = {}) {
  if (platform !== "win32") throw new ShowAsioBlockedError("WINDOWS_X64_REQUIRED", "Show-ASIO local artifact build is Windows x64 only.");
  const staticContract = readStaticContract(workspace);
  assertNoShowAsioEnvironmentEscape(environment);
  const inventory = loadWindowsRuntimeInventory({ workspace });
  const ffmpegRoot = assertSafeExternalDirectory(environment.FFMPEG_DIR, "Show-ASIO FFMPEG_DIR");
  validatePinnedFfmpegRuntimeDirectory(resolve(ffmpegRoot, "bin"), "Show-ASIO FFMPEG_DIR/bin", inventory);
  assertExistingBuildTargetParents(workspace);
  const gitBefore = captureGitState(workspace);
  const sourceBefore = collectShowAsioSourceIdentity(workspace);
  const finalArtifact = resolve(workspace, expectedShowAsioArtifactRelativeDirectory(staticContract.version, gitBefore.head));
  if (existsSync(finalArtifact)) throw new Error("Show-ASIO final artifact directory already exists and cannot be overwritten: " + finalArtifact);

  const nativeEnvironment = tauriCommandEnvironment(["build"], environment, platform);
  const appTarget = resolve(workspace, appTargetRelative);
  const bridgeTarget = resolve(workspace, bridgeTargetRelative);
  const applicationEnvironment = {
    ...nativeEnvironment,
    CARGO_TARGET_DIR: appTarget,
    SYNDOCAL_TAURI_FEATURES: showAsioFeatures.join(","),
  };
  const bridgeEnvironment = { ...nativeEnvironment, CARGO_TARGET_DIR: bridgeTarget };

  runChecked("release metadata gate", "pnpm.exe", ["--dir", "app", "run", "check:release"], {
    cwd: workspace,
    environment: nativeEnvironment,
  });
  printAndVerifyLinker(bridgeEnvironment);
  runChecked(
    "ASIO SDK/linker preflight",
    "powershell.exe",
    ["-NoLogo", "-NoProfile", "-NonInteractive", "-File", resolve(workspace, "qa/harnesses/check-asio-build.ps1"), "-PreflightOnly"],
    { cwd: workspace, environment: bridgeEnvironment },
  );
  printAndVerifyLinker(bridgeEnvironment);
  stopCheckoutReleaseExecutable(resolve(workspace, "app"));
  runChecked(
    "isolated ASIO bridge release build",
    "cargo.exe",
    [
      "build",
      "--manifest-path", resolve(workspace, "tools/asio-bridge/Cargo.toml"),
      "--no-default-features",
      "--features", "asio",
      "--locked",
      "--release",
      "--lib",
      "--target-dir", bridgeTarget,
    ],
    { cwd: workspace, environment: bridgeEnvironment },
  );
  const bridgePath = resolve(bridgeTarget, "release/syndocal_asio_bridge.dll");
  const bridgeRecord = readVerifiedCargoBridgeBuildOutput(bridgePath, bridgeTarget);
  const bridgeSha256 = createHash("sha256").update(bridgeRecord.bytes).digest("hex");
  const exports = inspectBridgeExports(bridgePath, { environment: bridgeEnvironment });
  if (JSON.stringify(exports) !== JSON.stringify(showAsioBridgeExports)) {
    throw new Error("Just-built Show-ASIO bridge exports are not the exact ABI v2 set.");
  }

  printAndVerifyLinker(applicationEnvironment);
  stopCheckoutReleaseExecutable(resolve(workspace, "app"));
  const tauriCli = resolve(workspace, "app/node_modules/@tauri-apps/cli/tauri.js");
  runChecked(
    "isolated unbundled Show-ASIO Tauri build",
    process.execPath,
    [
      tauriCli,
      "build",
      "--no-bundle",
      "--config", "src-tauri/tauri.show-asio.conf.json",
      "--features", showAsioFeatures.join(","),
      "--",
      "--locked",
    ],
    { cwd: resolve(workspace, "app"), environment: applicationEnvironment },
  );

  const gitAfter = captureGitState(workspace);
  const sourceAfter = collectShowAsioSourceIdentity(workspace);
  assertSourceAndGitStable(gitBefore, gitAfter, sourceBefore, sourceAfter);
  const hostBinding = computeShowAsioHostBinding(workspace, readWindowsMachineGuid());
  return prepareShowAsioRuntime({
    workspace,
    version: staticContract.version,
    commit: gitBefore.head,
    hostBindingSha256: hostBinding,
    sourceFiles: sourceBefore,
    ffmpegDir: environment.FFMPEG_DIR,
    expectedBridgeSha256: bridgeSha256,
    applicationPath: resolve(appTarget, "release/syndocal.exe"),
    bridgePath,
    allowCargoRootDepsAlias: true,
    artifactDir: finalArtifact,
    inventory,
    exportInspector: (path) => inspectBridgeExports(path, { environment: applicationEnvironment }),
  });
}

async function runSelfTest() {
  let assertions = 0;
  const pass = (condition, label) => { assert.ok(condition, label); assertions += 1; };
  const rejects = (action, pattern, label) => { assert.throws(action, pattern, label); assertions += 1; };
  pass(parseBuildMode([]) === "build", "no arguments select the fixed real build route");
  pass(parseBuildMode(["--plan"]) === "plan", "one exact plan argument is accepted");
  pass(parseBuildMode(["--self-test"]) === "self-test", "one exact self-test argument is accepted");
  for (const args of [["--features", "ndi"], ["--plan", "extra"], ["--bundle"], ["--target-dir", "elsewhere"]]) {
    rejects(() => parseBuildMode(args), /overrides are forbidden|Usage/, "extra/override arguments fail closed: " + args.join(" "));
  }
  const baseCargo = '[features]\ndefault = ["libav", "spout"]\nasio = ["dep:libloading"]\nshow-asio = ["asio"]\nlibav = []\nspout = []\nndi = []\n\n[dependencies]\n';
  pass(JSON.stringify(validateShowAsioFeatureContract(baseCargo)) === JSON.stringify(showAsioFeatures), "exact feature contract is accepted");
  rejects(
    () => validateShowAsioFeatureContract(baseCargo.replace('show-asio = ["asio"]\n', "")),
    (error) => error instanceof ShowAsioBlockedError && error.kind === "SHOW_ASIO_FEATURE_MISSING",
    "missing app integration produces typed BLOCKED before Cargo",
  );
  rejects(() => validateShowAsioFeatureContract(baseCargo, [...showAsioFeatures, "ndi"]), /feature union/, "extra requested feature is rejected");
  rejects(() => validateShowAsioFeatureContract(baseCargo.replace('default = ["libav", "spout"]', 'default = ["libav", "spout", "show-asio"]')), /Normal MIT\/WASAPI/, "normal default feature contamination is rejected");
  rejects(() => validateShowAsioFeatureContract(baseCargo.replace('show-asio = ["asio"]', 'show-asio = ["asio", "legacy"]')), /fallback aliases/, "legacy/fallback show feature is rejected");
  const cleanGit = { branch: "beta", head: "a".repeat(40), upstream: "a".repeat(40), status: "" };
  pass(validateGitState(cleanGit).head === cleanGit.head, "clean pushed Git state is accepted");
  rejects(() => validateGitState({ ...cleanGit, status: " M tracked.txt" }), /completely clean/, "tracked dirt is rejected");
  rejects(() => validateGitState({ ...cleanGit, status: "?? untracked.txt" }), /completely clean/, "untracked files are rejected");
  rejects(() => validateGitState({ ...cleanGit, upstream: "b".repeat(40) }), /must equal its upstream/, "HEAD/upstream drift is rejected");
  pass(validateVersionSet({ workspaceVersion: "1.2.0-alpha.12", lockVersion: "1.2.0-alpha.12", packageVersion: "1.2.0-alpha.12", tauriVersion: "1.2.0-alpha.12" }) === "1.2.0-alpha.12", "synchronized version set is accepted");
  rejects(() => validateVersionSet({ workspaceVersion: "1.2.0-alpha.12", lockVersion: "1.2.0-alpha.11", packageVersion: "1.2.0-alpha.12", tauriVersion: "1.2.0-alpha.12" }), /version drift/, "version drift is rejected");
  rejects(() => assertSourceAndGitStable(cleanGit, { ...cleanGit, head: "b".repeat(40) }, [], []), /Git identity/, "commit drift during build is rejected");
  rejects(() => assertSourceAndGitStable(cleanGit, cleanGit, [{ path: "x", sha256: "a" }], [{ path: "x", sha256: "b" }]), /source bytes changed/, "source mutation during build is rejected");
  const baseEnvironment = {
    FFMPEG_DIR: "C:\\ffmpeg",
    CPAL_ASIO_DIR: "C:\\asio",
    SYNDOCAL_ASIO_SDK_ARCHIVE_PATH: "C:\\asio.zip",
    LIBCLANG_PATH: "C:\\llvm",
  };
  assertNoShowAsioEnvironmentEscape(baseEnvironment);
  assertions += 1;
  assertNoShowAsioEnvironmentEscape({ ...baseEnvironment, NDI_SDK_DIR: "C:\\discovery-only" });
  assertions += 1;
  for (const [name, value] of [
    ["CARGO_TARGET_DIR", "escape"],
    ["TAURI_CONFIG", ""],
    ["CARGO_FEATURE_NDI", "1"],
    ["CARGO_FEATURE_LEGACY", "1"],
    ["TAURI_SIGNING_PRIVATE_KEY", "secret"],
    ["TAURI_BUNDLE_TARGETS", "nsis"],
    ["SYNDOCAL_TAURI_FEATURES", "libav,spout,ndi"],
    ["SYNDOCAL_NDI_ESCAPE", "1"],
  ]) {
    rejects(() => assertNoShowAsioEnvironmentEscape({ ...baseEnvironment, [name]: value }), /forbidden|NDI-enabled|NDI environment signal|fail-closed/, "environment escape is rejected: " + name);
  }
  const requiredLinker = requiredMsvcToolchain({}).linker;
  rejects(
    () => assertPreparedLinkerEnvironment({ CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER: requiredLinker }, ["C:\\Program Files\\Git\\usr\\bin\\link.exe", requiredLinker]),
    /does not place|order/,
    "Git link.exe resolving first is rejected",
  );
  rejects(
    () => assertPreparedLinkerEnvironment({ CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER: "C:\\wrong\\link.exe" }, [requiredLinker]),
    /pin is not the exact/,
    "wrong absolute Cargo linker pin is rejected",
  );
  const cargoLinkFixture = mkdtempSync(join(tmpdir(), "syndocal-show-asio-cargo-link-"));
  try {
    const releaseDirectory = join(cargoLinkFixture, "release");
    const depsDirectory = join(releaseDirectory, "deps");
    const rootOutput = join(releaseDirectory, "syndocal_asio_bridge.dll");
    const depsOutput = join(depsDirectory, "syndocal_asio_bridge.dll");
    mkdirSync(depsDirectory, { recursive: true });
    writeFileSync(rootOutput, "cargo bridge bytes", { flag: "wx" });
    linkSync(rootOutput, depsOutput);
    const accepted = readVerifiedCargoBridgeBuildOutput(rootOutput, cargoLinkFixture);
    pass(accepted.bytes.toString("utf8") === "cargo bridge bytes", "exact Cargo root/deps two-name bridge topology is accepted");

    const thirdLink = join(releaseDirectory, "unexpected-bridge-alias.dll");
    linkSync(rootOutput, thirdLink);
    rejects(
      () => readVerifiedCargoBridgeBuildOutput(rootOutput, cargoLinkFixture),
      /exactly the canonical root\/deps two-name link topology/,
      "an additional in-target bridge hard-link alias is rejected",
    );
    unlinkSync(thirdLink);

    rejects(
      () => readVerifiedCargoBridgeBuildOutput(join(releaseDirectory, "wrong.dll"), cargoLinkFixture),
      /not the exact isolated target path/,
      "a noncanonical Cargo bridge root path is rejected",
    );

    unlinkSync(depsOutput);
    const unexpectedAlias = join(releaseDirectory, "unexpected-root-alias.dll");
    linkSync(rootOutput, unexpectedAlias);
    rejects(
      () => readVerifiedCargoBridgeBuildOutput(rootOutput, cargoLinkFixture),
      /deps output is missing/,
      "a two-link root with the canonical deps name missing is rejected",
    );
    unlinkSync(unexpectedAlias);

    const rootAlias = join(releaseDirectory, "root-pair-alias.dll");
    linkSync(rootOutput, rootAlias);
    writeFileSync(depsOutput, "cargo bridge bytes", { flag: "wx" });
    const depsAlias = join(depsDirectory, "deps-pair-alias.dll");
    linkSync(depsOutput, depsAlias);
    rejects(
      () => readVerifiedCargoBridgeBuildOutput(rootOutput, cargoLinkFixture),
      /not exactly two names for one unchanged file identity/,
      "two separate two-link same-byte pairs are rejected instead of being mistaken for one Cargo identity",
    );
  } finally {
    rmSync(cargoLinkFixture, { recursive: true, force: true });
  }
  pass(!existsSync(resolve(workspaceRoot, "target/show-asio-build/self-test-probe")), "self-test creates no Cargo/native output");
  console.log("Show-ASIO build orchestrator self-test passed: " + assertions + " assertions; Cargo/native/process-stop=NOT_RUN");
}

async function main(argv = process.argv.slice(2)) {
  const mode = parseBuildMode(argv);
  if (mode === "self-test") {
    await runSelfTest();
    return;
  }
  if (mode === "plan") {
    console.log(JSON.stringify(buildPlan(workspaceRoot), null, 2));
    return;
  }
  const result = await buildShowAsio();
  console.log("Show-ASIO local artifact created and verified: " + result.artifactDir + " files=" + String(result.filesVerified) + " distributionApproved=false");
}

if (resolve(process.argv[1] ?? "") === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    if (error instanceof ShowAsioBlockedError) {
      console.error("[show-asio-build] BLOCKED " + error.kind + ": " + error.message);
      process.exitCode = 2;
    } else {
      console.error("[show-asio-build] " + (error instanceof Error ? error.message : String(error)));
      process.exitCode = 1;
    }
  });
}
