import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  GITHUB_HOSTED_WINDOWS_TOOLCHAIN_MARKER,
  GITHUB_HOSTED_WINDOWS_TOOLCHAIN_MARKER_VALUE,
  REQUIRED_GITHUB_HOSTED_MSVS_LINKER,
  REQUIRED_GITHUB_HOSTED_VCTOOLS_INSTALL_DIR,
  REQUIRED_MSVS_LINKER,
  REQUIRED_VCVARS_ARGUMENTS,
  REQUIRED_VCVARS_BATCH,
  REQUIRED_VCTOOLS_INSTALL_DIR,
  REQUIRED_VCTOOLS_VERSION,
  captureRequiredVcvarsEnvironment,
  isGitHubHostedWindowsToolchain,
  isNativeReleaseBuild,
  isWindowsNativeCargoCommand,
  locateLinkersWithWhere,
  parseCommandLineSetOutput,
  releaseExecutablePath,
  requireExactMsvcLinkerFirst,
  requireExactMsvcToolset,
  requiredMsvcToolchain,
  tauriCommandEnvironment,
  verifiedNativeBuildEnvironment,
} from "./run-tauri.mjs";

let assertionCount = 0;
const equal = (...arguments_) => {
  assertionCount += 1;
  return assert.equal(...arguments_);
};
const deepEqual = (...arguments_) => {
  assertionCount += 1;
  return assert.deepStrictEqual(...arguments_);
};
const ok = (...arguments_) => {
  assertionCount += 1;
  return assert.ok(...arguments_);
};
const throws = (...arguments_) => {
  assertionCount += 1;
  return assert.throws(...arguments_);
};
const match = (...arguments_) => {
  assertionCount += 1;
  return assert.match(...arguments_);
};

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const appDir = path.resolve(scriptDir, "..");
const repoDir = path.resolve(appDir, "..");
const packageJson = JSON.parse(
  await readFile(path.join(appDir, "package.json"), "utf8"),
);
const wrapperSource = await readFile(path.join(scriptDir, "run-tauri.mjs"), "utf8");
const warningRatchetSource = await readFile(
  path.join(scriptDir, "warning-ratchet-lib.mjs"),
  "utf8",
);
const workflowSource = await readFile(
  path.join(repoDir, ".github", "workflows", "cross-platform.yml"),
  "utf8",
);
const asioBuildHarnessSource = await readFile(
  path.join(repoDir, "qa", "harnesses", "check-asio-build.ps1"),
  "utf8",
);
const soakHarnessSource = await readFile(
  path.join(repoDir, "qa", "run-soak.ps1"),
  "utf8",
);
const soakHarnessSelfTestSource = await readFile(
  path.join(repoDir, "qa", "test-run-soak-linker-gate.ps1"),
  "utf8",
);

equal(
  packageJson.scripts.tauri,
  "node scripts/run-tauri.mjs",
  "pnpm tauri must pass through the guarded wrapper",
);
equal(isNativeReleaseBuild(["build", "--no-bundle"]), true);
equal(isNativeReleaseBuild(["--verbose", "build", "--no-bundle"]), true);
equal(isNativeReleaseBuild(["dev"]), false);
equal(isWindowsNativeCargoCommand(["build", "--no-bundle"]), true);
equal(isWindowsNativeCargoCommand(["dev"]), true);
equal(isWindowsNativeCargoCommand(["--verbose", "dev"]), true);
equal(isWindowsNativeCargoCommand(["info"]), false);
equal(
  releaseExecutablePath(appDir),
  path.resolve(appDir, "..", "target", "release", "syndocal.exe"),
);

equal(REQUIRED_VCTOOLS_VERSION, "14.44.35207");
equal(
  REQUIRED_VCTOOLS_INSTALL_DIR,
  "C:\\Program Files\\Microsoft Visual Studio\\2022\\Community\\VC\\Tools\\MSVC\\14.44.35207",
);
equal(
  REQUIRED_MSVS_LINKER,
  "C:\\Program Files\\Microsoft Visual Studio\\2022\\Community\\VC\\Tools\\MSVC\\14.44.35207\\bin\\Hostx64\\x64\\link.exe",
);
equal(
  REQUIRED_VCVARS_BATCH,
  "C:\\Program Files\\Microsoft Visual Studio\\2022\\Community\\VC\\Auxiliary\\Build\\vcvars64.bat",
);
equal(REQUIRED_VCVARS_ARGUMENTS, "-vcvars_ver=14.44");
equal(GITHUB_HOSTED_WINDOWS_TOOLCHAIN_MARKER, "SYNDOCAL_GITHUB_HOSTED_WINDOWS_MSVC");
equal(GITHUB_HOSTED_WINDOWS_TOOLCHAIN_MARKER_VALUE, "windows-2022-enterprise");
equal(
  REQUIRED_GITHUB_HOSTED_VCTOOLS_INSTALL_DIR,
  "C:\\Program Files\\Microsoft Visual Studio\\2022\\Enterprise\\VC\\Tools\\MSVC\\14.44.35207",
);
equal(
  REQUIRED_GITHUB_HOSTED_MSVS_LINKER,
  "C:\\Program Files\\Microsoft Visual Studio\\2022\\Enterprise\\VC\\Tools\\MSVC\\14.44.35207\\bin\\Hostx64\\x64\\link.exe",
);

const requiredFileIsRegular = (candidate) => candidate === REQUIRED_MSVS_LINKER;
const gitLinkerPath = "C:/Program Files/Git/usr/bin/link.exe";
const staleVcToolsDir = "C:\\Program Files\\Microsoft Visual Studio\\2022\\Community\\VC\\Tools\\MSVC\\14.43.34808";
const staleLinkerPath = path.win32.resolve(staleVcToolsDir, "bin", "Hostx64", "x64", "link.exe");
const properVcvarsEnvironment = { VCToolsInstallDir: REQUIRED_VCTOOLS_INSTALL_DIR };
const requiredLinkers = () => [REQUIRED_MSVS_LINKER, gitLinkerPath];
const githubHostedEnvironment = {
  GITHUB_ACTIONS: "true",
  RUNNER_OS: "Windows",
  RUNNER_ENVIRONMENT: "github-hosted",
  [GITHUB_HOSTED_WINDOWS_TOOLCHAIN_MARKER]: GITHUB_HOSTED_WINDOWS_TOOLCHAIN_MARKER_VALUE,
  VCToolsInstallDir: REQUIRED_GITHUB_HOSTED_VCTOOLS_INSTALL_DIR,
};
const githubHostedLinkers = () => [REQUIRED_GITHUB_HOSTED_MSVS_LINKER, gitLinkerPath];

equal(isGitHubHostedWindowsToolchain(githubHostedEnvironment), true);
equal(isGitHubHostedWindowsToolchain({ ...githubHostedEnvironment, RUNNER_OS: "Linux" }), false);
equal(
  isGitHubHostedWindowsToolchain({ ...githubHostedEnvironment, RUNNER_ENVIRONMENT: "self-hosted" }),
  false,
  "a self-hosted runner cannot opt into the Enterprise edition-root exception",
);
equal(
  isGitHubHostedWindowsToolchain({
    ...githubHostedEnvironment,
    [GITHUB_HOSTED_WINDOWS_TOOLCHAIN_MARKER]: undefined,
  }),
  false,
  "Enterprise is never accepted without the workflow's explicit hosted-runner marker",
);
equal(requiredMsvcToolchain(properVcvarsEnvironment).linker, REQUIRED_MSVS_LINKER);
equal(
  requiredMsvcToolchain(githubHostedEnvironment).linker,
  REQUIRED_GITHUB_HOSTED_MSVS_LINKER,
);
equal(
  requireExactMsvcToolset(
    githubHostedEnvironment,
    (candidate) => candidate === REQUIRED_GITHUB_HOSTED_MSVS_LINKER,
  ),
  REQUIRED_GITHUB_HOSTED_MSVS_LINKER,
  "the official GitHub-hosted windows-2022 Enterprise root is accepted only with the exact 14.44.35207 linker",
);
deepEqual(
  requireExactMsvcLinkerFirst(githubHostedEnvironment, githubHostedLinkers),
  [REQUIRED_GITHUB_HOSTED_MSVS_LINKER, gitLinkerPath],
);
throws(
  () => requireExactMsvcToolset(
    { ...githubHostedEnvironment, [GITHUB_HOSTED_WINDOWS_TOOLCHAIN_MARKER]: undefined },
    () => true,
  ),
  /required Visual Studio 2022 Community/,
  "an Enterprise root outside the narrow hosted-runner context must fail closed",
);
let hostedFallbackAttempted = false;
throws(
  () => tauriCommandEnvironment(
    ["build"],
    { ...githubHostedEnvironment, VCToolsInstallDir: "" },
    "win32",
    () => false,
    () => {
      hostedFallbackAttempted = true;
      return properVcvarsEnvironment;
    },
    githubHostedLinkers,
  ),
  /hosted windows-2022 MSVC preflight must initialize and verify/,
  "hosted CI must fail closed instead of trying the unavailable local Community vcvars batch",
);
equal(hostedFallbackAttempted, false);

// Negative fixture: an inherited Git usr/bin/link.exe must be replaced by the
// exact MSVC 14.44.35207 linker, never trusted or passed through.
const gitPoisonedEnvironment = {
  ...properVcvarsEnvironment,
  CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER: gitLinkerPath,
};
const verifiedEnvironment = verifiedNativeBuildEnvironment(
  gitPoisonedEnvironment,
  "win32",
  requiredFileIsRegular,
  requiredLinkers,
);
equal(
  verifiedEnvironment.CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER,
  REQUIRED_MSVS_LINKER,
  "Windows native builds must replace an inherited Git linker with the exact MSVC 14.44.35207 linker",
);
equal(
  gitPoisonedEnvironment.CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER,
  gitLinkerPath,
  "linker verification must not mutate the caller's environment",
);
equal(
  tauriCommandEnvironment(
    ["build"],
    gitPoisonedEnvironment,
    "win32",
    requiredFileIsRegular,
    undefined,
    requiredLinkers,
  ).CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER,
  REQUIRED_MSVS_LINKER,
  "Windows Tauri build must pin the exact MSVC 14.44.35207 linker",
);
equal(
  tauriCommandEnvironment(
    ["dev"],
    gitPoisonedEnvironment,
    "win32",
    requiredFileIsRegular,
    undefined,
    requiredLinkers,
  ).CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER,
  REQUIRED_MSVS_LINKER,
  "Windows Tauri dev must use the same exact MSVC 14.44.35207 linker as release builds",
);

// Negative fixture: a resolved stale 14.43 toolset fails closed even when its
// link.exe exists.
throws(
  () =>
    requireExactMsvcToolset(
      { VCToolsInstallDir: staleVcToolsDir },
      () => true,
    ),
  (error) => error.message.includes("does not match") && error.message.includes("14.44.35207"),
  "a stale 14.43 toolset must be rejected even when its linker exists",
);
throws(
  () => tauriCommandEnvironment(["dev"], { VCToolsInstallDir: staleVcToolsDir }, "win32", () => true, () => ({ VCToolsInstallDir: staleVcToolsDir })),
  /does not match/,
  "an initializer that captures another stale toolset must fail closed",
);
equal(
  requireExactMsvcToolset(properVcvarsEnvironment, requiredFileIsRegular),
  REQUIRED_MSVS_LINKER,
  "the exact 14.44.35207 toolset must resolve to the pinned Hostx64/x64 linker",
);

// The wrapper self-heals an uninitialized or stale shell by initializing
// vcvars64.bat -vcvars_ver=14.44 itself.
let initializedFrom = null;
const healedEnvironment = tauriCommandEnvironment(
  ["build"],
  { VCToolsInstallDir: staleVcToolsDir },
  "win32",
  requiredFileIsRegular,
  (initial) => {
    initializedFrom = initial;
    return properVcvarsEnvironment;
  },
  requiredLinkers,
);
equal(initializedFrom?.VCToolsInstallDir, staleVcToolsDir, "vcvars initialization must seed from the caller's environment");
equal(
  healedEnvironment.CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER,
  REQUIRED_MSVS_LINKER,
  "automatic vcvars 14.44 initialization must heal an uninitialized or stale shell",
);
throws(
  () =>
    tauriCommandEnvironment(
      ["build"],
      {},
      "win32",
      requiredFileIsRegular,
      () => null,
    ),
  /Automatic vcvars64\.bat -vcvars_ver=14\.44 initialization failed/,
  "a failed automatic vcvars initialization must fail closed",
);

equal(
  tauriCommandEnvironment(["info"], gitPoisonedEnvironment, "win32", () => false),
  gitPoisonedEnvironment,
  "non-Cargo Tauri commands must preserve their environment",
);
equal(
  verifiedNativeBuildEnvironment(gitPoisonedEnvironment, "linux", () => false),
  gitPoisonedEnvironment,
  "non-Windows builds must preserve their environment",
);
equal(
  tauriCommandEnvironment(
    ["build"],
    gitPoisonedEnvironment,
    "linux",
    () => false,
    () => {
      throw new Error("vcvars must not initialize on non-Windows platforms");
    },
  ),
  gitPoisonedEnvironment,
  "non-Windows native commands must never trigger vcvars initialization",
);

throws(
  () => verifiedNativeBuildEnvironment({}, "win32", () => true),
  /without VCToolsInstallDir/,
);
throws(
  () => verifiedNativeBuildEnvironment({ VCToolsInstallDir: REQUIRED_VCTOOLS_INSTALL_DIR }, "win32", () => false),
  /asserted MSVC linker is missing/,
);

deepEqual(
  locateLinkersWithWhere(properVcvarsEnvironment, (file, arguments_, options) => {
    equal(file, "where.exe");
    deepEqual(arguments_, ["link.exe"]);
    equal(options.env, properVcvarsEnvironment);
    return { status: 0, stdout: `${REQUIRED_MSVS_LINKER}\r\n${gitLinkerPath}\r\n` };
  }),
  [REQUIRED_MSVS_LINKER, gitLinkerPath],
  "where.exe output must preserve linker resolution order",
);
deepEqual(
  requireExactMsvcLinkerFirst(properVcvarsEnvironment, requiredLinkers),
  [REQUIRED_MSVS_LINKER, gitLinkerPath],
);
throws(
  () => requireExactMsvcLinkerFirst(properVcvarsEnvironment, () => [gitLinkerPath, REQUIRED_MSVS_LINKER]),
  /Git[\\/]usr[\\/]bin[\\/]link\.exe first/,
  "Git link.exe resolving first must fail closed even when the exact MSVC linker is also present",
);
throws(
  () => requireExactMsvcLinkerFirst(properVcvarsEnvironment, () => []),
  /returned no linker/,
);
throws(
  () => locateLinkersWithWhere(properVcvarsEnvironment, () => ({ status: 1, stdout: "" })),
  /where\.exe link\.exe failed/,
);
throws(
  () =>
    verifiedNativeBuildEnvironment(
      properVcvarsEnvironment,
      "win32",
      requiredFileIsRegular,
      () => [gitLinkerPath, REQUIRED_MSVS_LINKER],
    ),
  /Git[\\/]usr[\\/]bin[\\/]link\.exe first/,
  "an otherwise exact toolset must still reject Git link.exe resolving first",
);
let wherePoisonedInitializationCount = 0;
const healedWhereEnvironment = tauriCommandEnvironment(
  ["build"],
  { ...properVcvarsEnvironment, PATH: "C:\\Program Files\\Git\\usr\\bin" },
  "win32",
  requiredFileIsRegular,
  () => {
    wherePoisonedInitializationCount += 1;
    return properVcvarsEnvironment;
  },
  (environment) => environment.PATH ? [gitLinkerPath, REQUIRED_MSVS_LINKER] : requiredLinkers(),
);
equal(wherePoisonedInitializationCount, 1, "a Git-first PATH must be healed through a fresh vcvars environment");
equal(
  healedWhereEnvironment.CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER,
  REQUIRED_MSVS_LINKER,
);

deepEqual(
  parseCommandLineSetOutput(
    [
      "********** some vcvars banner **********",
      "[vcvarsall.bat Environment initialized]",
      "PATH=C:\\Program Files\\Microsoft Visual Studio\\2022\\Community\\VC\\Tools\\MSVC\\14.44.35207\\bin\\Hostx64\\x64;C:\\Windows",
      "VCToolsInstallDir=C:\\Program Files\\Microsoft Visual Studio\\2022\\Community\\VC\\Tools\\MSVC\\14.44.35207\\",
      "",
    ].join("\r\n"),
  ),
  {
    PATH: "C:\\Program Files\\Microsoft Visual Studio\\2022\\Community\\VC\\Tools\\MSVC\\14.44.35207\\bin\\Hostx64\\x64;C:\\Windows",
    VCToolsInstallDir: "C:\\Program Files\\Microsoft Visual Studio\\2022\\Community\\VC\\Tools\\MSVC\\14.44.35207\\",
  },
  "`set` output parsing must skip banner lines and keep NAME=VALUE pairs",
);
equal(parseCommandLineSetOutput("no environment assignments here\r\n"), null);

let capturedArguments = null;
const capturedEnvironment = { VCToolsInstallDir: "" };
const fakeCapture = captureRequiredVcvarsEnvironment(
  capturedEnvironment,
  (file, arguments_, options) => {
    capturedArguments = { file, arguments_, options };
    return { status: 0, stdout: "VCToolsInstallDir=C:\\Program Files\\Microsoft Visual Studio\\2022\\Community\\VC\\Tools\\MSVC\\14.44.35207\\\r\n" };
  },
);
ok(capturedArguments !== null, "vcvars capture must spawn cmd.exe");
equal(capturedArguments.file, "cmd.exe");
equal(capturedArguments.arguments_.slice(0, 3).join(" "), "/d /s /c");
match(capturedArguments.arguments_[3], /^""[^"]+vcvars64\.bat" -vcvars_ver=14\.44 && set"$/);
equal(capturedArguments.options.env, capturedEnvironment, "vcvars must inherit the caller's environment");
equal(capturedArguments.options.windowsVerbatimArguments, true, "cmd argument quoting must stay verbatim");
equal(
  fakeCapture.VCToolsInstallDir,
  "C:\\Program Files\\Microsoft Visual Studio\\2022\\Community\\VC\\Tools\\MSVC\\14.44.35207\\",
);
equal(
  captureRequiredVcvarsEnvironment({}, () => ({ status: 1, stdout: "boom" })),
  null,
  "a failing vcvars invocation must report no captured environment",
);
equal(
  captureRequiredVcvarsEnvironment({}, () => ({ error: new Error("spawn failed"), status: null })),
  null,
);

match(wrapperSource, /StringComparer\]::OrdinalIgnoreCase\.Equals\(\$actual, \$expected\)/);
match(wrapperSource, /Stop-Process -Id \$pidToStop -Force/);
match(wrapperSource, /Get-ExactSyndocalProcess/);
match(wrapperSource, /Refusing to build while the checkout release executable cannot be stopped/);
match(wrapperSource, /CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER: linker/);
match(wrapperSource, /14\.44\.35207/, "the exact MSVC toolset version must appear in the wrapper source");
match(wrapperSource, /-vcvars_ver=14\.44/, "the wrapper must initialize vcvars with -vcvars_ver=14.44");
match(wrapperSource, /where\.exe link\.exe resolves/);
match(wrapperSource, /where\.exe link\.exe:\\n/);
match(
  wrapperSource,
  /process\.platform === "win32"[\s\S]*?CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER/,
  "post-verification where.exe logging must remain Windows-only",
);
match(
  wrapperSource,
  /initializeVcvarsEnvironment = captureRequiredVcvarsEnvironment/,
  "native Cargo commands must default to automatic vcvars 14.44 initialization",
);
match(wrapperSource, /const environment = tauriCommandEnvironment\(args\)/);
const pinAssignmentIndex = wrapperSource.indexOf("CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER: linker");
const environmentDefinitionIndex = wrapperSource.indexOf("const environment = tauriCommandEnvironment(args)");
const childSpawnIndex = wrapperSource.indexOf("spawnSync(process.execPath");
ok(
  pinAssignmentIndex > -1 && environmentDefinitionIndex > -1 && childSpawnIndex > -1
    && pinAssignmentIndex < environmentDefinitionIndex && environmentDefinitionIndex < childSpawnIndex,
  "the linker pin must be established before the guarded environment feeds the child Tauri/Cargo spawn",
);
match(
  wrapperSource,
  /spawnSync\(process\.execPath, \[tauriCli, \.\.\.args\],[\s\S]*?env: environment/,
);
match(wrapperSource, /statSync\(candidate\)\.isFile\(\)/);
match(workflowSource, /uses: ilammy\/msvc-dev-cmd@v1[\s\S]*?arch: x64/);
const requiredToolsetInput = REQUIRED_VCVARS_ARGUMENTS.replace("-vcvars_ver=", "");
equal(requiredToolsetInput, "14.44");
match(
  workflowSource,
  new RegExp(`uses: ilammy/msvc-dev-cmd@v1[\\s\\S]*?arch: x64\\s*\\n\\s*toolset: ${requiredToolsetInput}`),
  "the Windows MSVC initializer must request the exact toolset through the action's supported input",
);
const escapeRegExp = (value) => value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
const msvcEnvironmentIndex = workflowSource.indexOf("uses: ilammy/msvc-dev-cmd@v1");
const preflightStepMarker = "- name: Pin exact Windows MSVC 14.44.35207 linker";
const preflightIndex = workflowSource.indexOf(preflightStepMarker);
ok(
  preflightIndex > msvcEnvironmentIndex && preflightIndex > -1,
  "an exact-linker preflight step must exist after MSVC initialization",
);
const preflightBlock = workflowSource.slice(
  preflightIndex,
  workflowSource.indexOf("\n      - name:", preflightIndex + preflightStepMarker.length),
);
match(
  preflightBlock,
  new RegExp(escapeRegExp(`${REQUIRED_GITHUB_HOSTED_VCTOOLS_INSTALL_DIR}\\`)),
  "the hosted workflow must require VCToolsInstallDir to resolve exactly the official Enterprise toolset directory",
);
match(
  preflightBlock,
  /Join-Path \$canonicalActual 'bin\\Hostx64\\x64\\link\.exe'/,
  "the hosted workflow must derive the Hostx64/x64 linker from the already exact VCToolsInstallDir",
);
match(
  preflightBlock,
  /Test-Path -LiteralPath \$expectedLinker -PathType Leaf/,
  "the preflight must verify the exact linker exists as a regular file",
);
match(
  preflightBlock,
  /"CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER=\$expectedLinker" >> \$env:GITHUB_ENV/,
  "the preflight must pin CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER for following steps through GITHUB_ENV",
);
match(
  preflightBlock,
  new RegExp(
    `"${GITHUB_HOSTED_WINDOWS_TOOLCHAIN_MARKER}=${GITHUB_HOSTED_WINDOWS_TOOLCHAIN_MARKER_VALUE}" >> \\$env:GITHUB_ENV`,
  ),
  "the workflow must explicitly mark the narrow GitHub-hosted Enterprise exception for the wrapper and warning ratchet",
);
match(
  preflightBlock,
  /SYNDOCAL_RUNNER_ENVIRONMENT -ne 'github-hosted'/,
  "the Enterprise edition-root exception must fail closed on self-hosted runners",
);
match(
  preflightBlock,
  /SYNDOCAL_RUNNER_ENVIRONMENT: \$\{\{ runner\.environment \}\}/,
  "the hosted/self-hosted decision must come from the immutable runner context",
);
ok(
  !preflightBlock.includes(REQUIRED_VCTOOLS_INSTALL_DIR),
  "the GitHub-hosted preflight must not require the unavailable local Community edition root",
);
match(preflightBlock, /where\.exe link\.exe/, "the preflight must prove linker resolution with where.exe link.exe");
ok(
  preflightBlock.includes("does not match") &&
    preflightBlock.includes("is missing") &&
    preflightBlock.includes("returned no linker") &&
    preflightBlock.includes("resolves '"),
  "the preflight must fail closed on toolset mismatch, missing linker, empty resolution, and non-first resolution",
);
const directCargoIndexes = [...workflowSource.matchAll(/\b(?:cargo|rustc)(?:\.exe)?\s+[A-Za-z][\w-]*/g)]
  .map((match_) => match_.index);
ok(
  directCargoIndexes.length > 0
    && directCargoIndexes.every((commandIndex) => msvcEnvironmentIndex < commandIndex),
  "Windows MSVC environment initialization must precede every direct Cargo/rustc command",
);
ok(
  directCargoIndexes.every((commandIndex) => preflightIndex < commandIndex),
  "the exact-toolset/linker preflight must precede every direct Cargo/rustc command",
);
const workflowToolsetVersions = [...workflowSource.matchAll(/\b14\.\d+\.\d+\b/g)].map((match_) => match_[0]);
ok(
  workflowToolsetVersions.length > 0
    && workflowToolsetVersions.every((version) => version === REQUIRED_VCTOOLS_VERSION),
  "every toolset version literal in the workflow must equal the required 14.44.35207 constant (drift detection)",
);

for (const [needle, reason] of [
  ["verifyExactWindowsMsvcCargoEnvironment", "define an exact Windows toolchain preflight"],
  ["VCToolsInstallDir", "validate the initialized exact toolset"],
  ["where.exe", "resolve the real linker search order"],
  ["required linker is missing", "fail closed when the exact linker file is absent"],
  ["returned no linker", "fail closed when linker resolution is empty"],
  ["required first linker", "fail closed when Git or another linker resolves first"],
]) {
  ok(
    warningRatchetSource.includes(needle),
    `warning-ratchet direct Cargo dispatch must ${reason}`,
  );
}
const warningCargoDispatchIndex = warningRatchetSource.indexOf(
  "export async function runCargoConfiguration(",
);
const warningCargoPreflightIndex = warningRatchetSource.indexOf(
  "const verifiedEnvironment = verifyExactWindowsMsvcCargoEnvironment(",
  warningCargoDispatchIndex,
);
const warningCargoMetadataIndex = warningRatchetSource.indexOf(
  "readCargoMetadataFromVerifiedEnvironment(",
  warningCargoDispatchIndex,
);
const warningCargoSpawnIndex = warningRatchetSource.indexOf(
  'runProcessWithTimeout("cargo"',
  warningCargoDispatchIndex,
);
ok(
  warningCargoDispatchIndex > -1
    && warningCargoPreflightIndex > warningCargoDispatchIndex
    && warningCargoMetadataIndex > warningCargoPreflightIndex
    && warningCargoSpawnIndex > warningCargoMetadataIndex,
  "warning-ratchet must verify the exact toolchain before Cargo metadata and the real Cargo command",
);
match(
  warningRatchetSource,
  /\[warning-ratchet\] pinned[\s\S]*?\[warning-ratchet\] where\.exe link\.exe:/,
  "warning-ratchet must print the exact pin and where.exe order before Cargo",
);

for (const [needle, reason] of [
  ["14.44.35207", "pin the exact supported local MSVC toolset"],
  ["CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER", "require the explicit Cargo linker pin"],
  ["where.exe link.exe", "prove the real linker resolution order"],
  ["Test-MsvcLinkerPinContract", "fail closed through the shared harness preflight contract"],
  ["[switch]$PreflightOnly", "provide a no-Cargo live preflight mode"],
  ["[switch]$SelfTest", "provide a hermetic no-Cargo negative-test mode"],
]) {
  ok(
    asioBuildHarnessSource.includes(needle),
    `ASIO direct-Cargo harness must ${reason}`,
  );
}
const asioLivePreflightIndex = asioBuildHarnessSource.indexOf(
  "$linkerPinContract = Test-MsvcLinkerPinContract",
);
const asioPreflightOnlyIndex = asioBuildHarnessSource.indexOf("if ($PreflightOnly)");
const asioCargoIndex = asioBuildHarnessSource.indexOf("& cargo check");
ok(
  asioLivePreflightIndex > -1
    && asioPreflightOnlyIndex > asioLivePreflightIndex
    && asioCargoIndex > asioPreflightOnlyIndex,
  "ASIO harness must validate the exact linker and allow a no-Cargo exit before its sole Cargo check",
);
equal(
  [...asioBuildHarnessSource.matchAll(/&\s+cargo\b/g)].length,
  1,
  "ASIO harness must keep every direct Cargo launch behind the verified preflight",
);
match(
  asioBuildHarnessSource,
  /where\.exe link\.exe resolution order[\s\S]*?Pinned host x64 linker/,
  "ASIO harness must log the real linker order and accepted pin before Cargo",
);

for (const [needle, reason] of [
  ["14.44.35207", "pin the exact supported local MSVC toolset"],
  ["-vcvars_ver=14.44", "initialize the exact supported vcvars toolset"],
  ["CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER", "pin Cargo to the exact Hostx64/x64 linker"],
  ["where.exe link.exe", "prove and print the real linker search order"],
  ["[switch]$PreflightOnly", "provide a live no-Cargo preflight mode"],
  ["must never be present on a build or soak-harness run", "reject hermetic test overrides on production execution"],
]) {
  ok(soakHarnessSource.includes(needle), `soak direct-Cargo harness must ${reason}`);
}
const soakOverrideGuardIndex = soakHarnessSource.indexOf(
  "if ($null -ne $global:SYNDOCAL_RUN_SOAK_TEST_DEPENDENCIES -and -not $PreflightOnly)",
);
const soakLivePreflightIndex = soakHarnessSource.indexOf(
  "$verifiedEnvironment = Invoke-SoakMsvcPreflight",
);
const soakPreflightOnlyIndex = soakHarnessSource.indexOf("if ($PreflightOnly)");
const soakCargoIndex = soakHarnessSource.indexOf("& cargo @cargoArguments");
ok(
  soakOverrideGuardIndex > -1
    && soakLivePreflightIndex > soakOverrideGuardIndex
    && soakPreflightOnlyIndex > soakLivePreflightIndex
    && soakCargoIndex > soakPreflightOnlyIndex,
  "soak harness must reject test-only overrides, verify the exact linker, and allow a no-Cargo exit before its sole Cargo build",
);
equal(
  [...soakHarnessSource.matchAll(/&\s+cargo\b/g)].length,
  1,
  "soak harness must keep every direct Cargo launch behind the verified preflight",
);
for (const proof of [
  "Git usr/bin/link.exe resolving first must be rejected",
  "failed vcvars initialization fails closed",
  "test dependency overrides are rejected outside -PreflightOnly",
  "must never launch Cargo",
]) {
  ok(
    soakHarnessSelfTestSource.includes(proof),
    `soak linker self-test must prove: ${proof}`,
  );
}

const nativeQaLaunchers = [
  {
    name: "check-native-window-acceptance.ps1",
    config: "src-tauri/tauri.native-acceptance.conf.json",
  },
  {
    name: "check-native-workspace-operator.ps1",
    config: "src-tauri/tauri.workspace-operator-acceptance.conf.json",
  },
];
for (const launcher of nativeQaLaunchers) {
  const launcherSource = await readFile(path.join(scriptDir, launcher.name), "utf8");
  ok(
    !/["'](?:exec|dlx)["']\s*,\s*["']tauri["']/.test(launcherSource),
    `${launcher.name} must not launch the Tauri CLI through pnpm exec/dlx argument lists`,
  );
  ok(
    !/pnpm\s+(?:exec|dlx)\s+tauri/i.test(launcherSource),
    `${launcher.name} must not bypass the guarded package script with pnpm exec/dlx`,
  );
  ok(
    !launcherSource.includes("run-tauri.mjs"),
    `${launcher.name} must route through the guarded package script, not invoke the wrapper file directly`,
  );
  ok(
    !launcherSource.includes("@tauri-apps"),
    `${launcher.name} must not reference the Tauri CLI package directly`,
  );
  match(
    launcherSource,
    /Start-Process -FilePath \$pnpm\.Source/,
    `${launcher.name} must keep launching Tauri through pnpm`,
  );
  match(
    launcherSource,
    new RegExp(
      `-ArgumentList @\\("tauri", "dev", "--config", "${escapeRegExp(launcher.config)}"`,
    ),
    `${launcher.name} must start dev through the guarded pnpm tauri package script with its acceptance config`,
  );
}

// Behavioral seam over runTauri orchestration: evaluate the extracted function
// body with process-leaving operations stubbed so ordering can be proven
// without launching any real process.
const runTauriBodyMatch =
  /export function runTauri\(args, baseAppDir = appDir\) \{\n([\s\S]*?)\n\}\n\nconst invokedDirectly/.exec(
    wrapperSource,
  );
ok(runTauriBodyMatch !== null, "runTauri orchestration must be extractable for behavioral verification");
const seamRunTauri = new Function(
  "isNativeReleaseBuild",
  "stopCheckoutReleaseExecutable",
  "tauriCommandEnvironment",
  "requireExactMsvcLinkerFirst",
  "path",
  "spawnSync",
  "process",
  "console",
  "appDir",
  `function runTauri(args, baseAppDir = appDir) {\n${runTauriBodyMatch[1]}\n}\nreturn runTauri;`,
);
const seamConsole = { error: () => {} };
const createSeamHarness = ({
  pinnedLinker = "",
  spawnResult = { status: 0 },
  platform = "win32",
} = {}) => {
  const events = [];
  const seamState = { lastSpawn: null };
  const seamEnvironment = pinnedLinker ? { CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER: pinnedLinker } : {};
  const runTauri = seamRunTauri(
    isNativeReleaseBuild,
    (baseAppDir) => events.push(["stop", baseAppDir]),
    (arguments_) => {
      events.push(["environment", arguments_.join(" ")]);
      return seamEnvironment;
    },
    (environment) => {
      events.push(["where", environment.CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER]);
      return [environment.CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER];
    },
    path,
    (file, arguments_, options) => {
      events.push(["spawn", file]);
      seamState.lastSpawn = { arguments_, options };
      return spawnResult;
    },
    { execPath: "seam-node", platform },
    seamConsole,
    "<seam-app-dir>",
  );
  return { events, seamState, seamEnvironment, runTauri };
};

const releaseSeam = createSeamHarness();
equal(releaseSeam.runTauri(["build", "--no-bundle"]), 0);
deepEqual(
  releaseSeam.events.map(([kind]) => kind),
  ["stop", "environment", "spawn"],
  "release builds must stop the checkout executable before verifying the environment and spawning the child",
);
deepEqual(releaseSeam.events[0], ["stop", "<seam-app-dir>"]);
deepEqual(
  releaseSeam.events[2],
  ["spawn", "seam-node"],
  "the child must be spawned through process.execPath",
);
equal(
  releaseSeam.seamState.lastSpawn.arguments_[0],
  path.join("<seam-app-dir>", "node_modules", "@tauri-apps", "cli", "tauri.js"),
);
equal(releaseSeam.seamState.lastSpawn.options.cwd, "<seam-app-dir>");
equal(releaseSeam.seamState.lastSpawn.options.env, releaseSeam.seamEnvironment);

const devSeam = createSeamHarness({ pinnedLinker: REQUIRED_MSVS_LINKER });
equal(devSeam.runTauri(["dev"]), 0);
ok(
  !devSeam.events.some(([kind]) => kind === "stop"),
  "dev must never attempt to stop the checkout release executable",
);
deepEqual(
  devSeam.events.map(([kind]) => kind),
  ["environment", "where", "spawn"],
  "dev must verify the pinned environment and re-resolve linkers before spawning the child",
);

const infoSeam = createSeamHarness();
equal(infoSeam.runTauri(["info"]), 0);
ok(!infoSeam.events.some(([kind]) => kind === "stop"), "non-native commands must never stop the release executable");

const nonWindowsPinnedSeam = createSeamHarness({
  pinnedLinker: REQUIRED_MSVS_LINKER,
  platform: "linux",
});
equal(nonWindowsPinnedSeam.runTauri(["dev"]), 0);
ok(
  !nonWindowsPinnedSeam.events.some(([kind]) => kind === "where"),
  "a stray Windows linker variable must never invoke where.exe on a non-Windows platform",
);

const failingSpawnSeam = createSeamHarness({
  spawnResult: { error: new Error("seam spawn failure") },
});
throws(() => failingSpawnSeam.runTauri(["build"]), /seam spawn failure/);
equal(failingSpawnSeam.events[0][0], "stop", "even a failing spawn must have stopped the checkout executable first");

console.log(`tauri build wrapper checks passed (${assertionCount} assertions)`);
