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
// Exact composition, not substring: the pinned linkers must be exactly the
// Hostx64/x64 link.exe under their exact edition toolset roots.
equal(
  REQUIRED_MSVS_LINKER,
  `${REQUIRED_VCTOOLS_INSTALL_DIR}\\bin\\Hostx64\\x64\\link.exe`,
  "the local pin must be exactly the Hostx64/x64 link.exe under the exact Community 14.44.35207 root",
);
equal(
  REQUIRED_GITHUB_HOSTED_MSVS_LINKER,
  `${REQUIRED_GITHUB_HOSTED_VCTOOLS_INSTALL_DIR}\\bin\\Hostx64\\x64\\link.exe`,
  "the hosted pin must be exactly the Hostx64/x64 link.exe under the exact Enterprise 14.44.35207 root",
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
  ["CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER", "require the explicit Cargo linker pin"],
  ["where.exe link.exe", "prove the real linker resolution order"],
  ["Test-MsvcLinkerPinContract", "fail closed through the shared harness preflight contract"],
  ["[switch]$PreflightOnly", "provide a no-Cargo live preflight mode"],
  ["[switch]$SelfTest", "provide a hermetic no-Cargo negative-test mode"],
  [
    "resolves first instead of",
    "fail closed when Git usr/bin/link.exe resolves ahead of the pinned MSVC linker",
  ],
  [
    '@{ CargoLinkerPin = "C:\\Program Files\\Git\\usr\\bin\\link.exe" }',
    "keep the Git-linker-pin negative fixture",
  ],
  [
    '@{ LinkResolutionOrder = @("C:\\Program Files\\Git\\usr\\bin\\link.exe", $script:RequiredHostX64Linker) }',
    "keep the Git-first resolution negative fixture",
  ],
]) {
  ok(
    asioBuildHarnessSource.includes(needle),
    `ASIO direct-Cargo harness must ${reason}`,
  );
}
// ---- Normalized PowerShell static seam -------------------------------------
// Byte-offset-preserving normalization: comments and the interiors of strings
// and here-strings become spaces (double-quoted strings honor $( )
// subexpression nesting so nested quotes cannot shift the true terminator),
// so every structural and Cargo-launch scan below sees real code positions
// and cannot be evaded by hiding tokens inside strings, comments, or
// here-strings. Detection is pattern-based (bare cargo, quoted call operator,
// Start-Process, cmd/cmd.exe /c, Invoke-Expression/iex, non-Start-Process
// spawners such as Invoke-Item/Start-Job/Start-ThreadJob/WMI/CIM/.NET
// Process.Start/pwsh -Command, statically bound variable indirection
// including env:/script:/global:/braced targets and "car" + "go" literal
// concatenation) rather than a finite allowlist.
const analyzePowerShellSource = (source) => {
  const maskedCharacters = source.split("");
  const literals = [];
  const length = source.length;
  const blankRange = (start, end) => {
    for (let index = start; index < end; index += 1) {
      if (maskedCharacters[index] !== "\r" && maskedCharacters[index] !== "\n") {
        maskedCharacters[index] = " ";
      }
    }
  };
  const consumeSingleQuoted = (start) => {
    let cursor = start + 1;
    while (cursor < length) {
      if (source[cursor] === "'") {
        if (source[cursor + 1] === "'") {
          cursor += 2;
          continue;
        }
        break;
      }
      cursor += 1;
    }
    const end = Math.min(cursor, length);
    const closeEnd = Math.min(cursor + 1, length);
    literals.push({ end: closeEnd, index: start, value: source.slice(start + 1, end).replace(/''/g, "'") });
    blankRange(start + 1, end);
    return closeEnd;
  };
  // Double-quoted strings are expandable: a $( ) subexpression may contain
  // nested quoted strings whose quotes must not terminate the outer string.
  // The cursor therefore tracks subexpression depth plus any nested
  // single/double-quoted string opened at depth > 0. The terminator search can
  // only diverge toward exposing code early (never toward masking real code),
  // which is the fail-closed direction for this audit. Offsets are stable:
  // masking still blanks exactly [start + 1, end) of the true outer string.
  const consumeDoubleQuoted = (start) => {
    let cursor = start + 1;
    let subexpressionDepth = 0;
    let nestedQuote = null;
    while (cursor < length) {
      const character = source[cursor];
      if (nestedQuote !== null) {
        if (nestedQuote === "'") {
          if (character === "'" && source[cursor + 1] === "'") {
            cursor += 2;
            continue;
          }
          if (character === "'") {
            nestedQuote = null;
            cursor += 1;
            continue;
          }
        } else {
          if (character === "`") {
            cursor += 2;
            continue;
          }
          if (character === '"' && source[cursor + 1] === '"') {
            cursor += 2;
            continue;
          }
          if (character === '"') {
            nestedQuote = null;
            cursor += 1;
            continue;
          }
        }
        cursor += 1;
        continue;
      }
      if (character === "`") {
        cursor += 2;
        continue;
      }
      if (subexpressionDepth > 0) {
        if (character === "'") {
          nestedQuote = "'";
          cursor += 1;
          continue;
        }
        if (character === '"') {
          nestedQuote = '"';
          cursor += 1;
          continue;
        }
        if (character === "(") {
          subexpressionDepth += 1;
          cursor += 1;
          continue;
        }
        if (character === ")") {
          subexpressionDepth -= 1;
          cursor += 1;
          continue;
        }
        cursor += 1;
        continue;
      }
      if (character === "$" && source[cursor + 1] === "(") {
        subexpressionDepth = 1;
        cursor += 2;
        continue;
      }
      if (character === '"') {
        if (source[cursor + 1] === '"') {
          cursor += 2;
          continue;
        }
        break;
      }
      cursor += 1;
    }
    const end = Math.min(cursor, length);
    const closeEnd = Math.min(cursor + 1, length);
    const value = source.slice(start + 1, end).replace(/`(.)/gs, "$1").replace(/""/g, '"');
    literals.push({ end: closeEnd, index: start, value });
    blankRange(start + 1, end);
    return closeEnd;
  };
  let index = 0;
  while (index < length) {
    const character = source[index];
    const next = source[index + 1];
    if (character === "<" && next === "#") {
      const end = source.indexOf("#>", index + 2);
      const stop = end === -1 ? length : end + 2;
      blankRange(index, stop);
      index = stop;
    } else if (character === "#") {
      const relativeNewline = /\r?\n/.exec(source.slice(index));
      const stop = relativeNewline ? index + relativeNewline.index : length;
      blankRange(index, stop);
      index = stop;
    } else if (
      character === "@"
      && (next === "'" || next === '"')
      && (source[index + 2] === "\r" || source[index + 2] === "\n")
    ) {
      const terminator = next === "'" ? "'@" : '"@';
      let terminatorIndex = -1;
      for (let cursor = index + 2; cursor < length; cursor += 1) {
        if (source[cursor] === "\n" && source.startsWith(terminator, cursor + 1)) {
          terminatorIndex = cursor + 1;
          break;
        }
      }
      const stop = terminatorIndex === -1 ? length : terminatorIndex + 2;
      blankRange(index, stop);
      index = stop;
    } else if (character === "'") {
      index = consumeSingleQuoted(index);
    } else if (character === '"') {
      index = consumeDoubleQuoted(index);
    } else {
      index += character === "`" ? 2 : 1;
    }
  }
  return { literals, masked: maskedCharacters.join("") };
};

const CARGO_COMMAND_LITERAL = /^\s*cargo(?:\.exe)?\s*$/i;
const CARGO_COMMAND_STRING = /^\s*cargo(?:\.exe)?(?:\s|$)/i;
const CARGO_TOKEN_PATTERN = /\bcargo(?:\.exe)?(?![\w.\-])/gi;
const DYNAMIC_EXECUTION_PATTERN = /\b(?:Invoke-Expression|iex)\b/gi;

// Alias assignment heads: plain variables plus scope- and environment-
// qualified targets ($env:C, $script:C, $global:C, ...) including braced
// ${...} spellings. Any of these bound to an exact cargo literal makes later
// invocations of that variable auditable.
const ALIAS_ASSIGNMENT_PATTERN =
  /\$(?:\{([^}\r\n]+)\}|(?:(env|script|global|local|private|using):([A-Za-z_][A-Za-z0-9_]*)|([A-Za-z_][A-Za-z0-9_]*)))\s+=\s+$/;

const resolveAliasAssignmentKey = (assignmentMatch) => {
  if (assignmentMatch[1] !== undefined) return assignmentMatch[1];
  if (assignmentMatch[2] !== undefined) return `${assignmentMatch[2]}:${assignmentMatch[3]}`;
  return assignmentMatch[4];
};

const aliasUsePattern = (key) =>
  new RegExp(
    `\\$\\{${escapeRegExp(key)}\\}|\\$${escapeRegExp(key)}(?![A-Za-z0-9_])`,
    "g",
  );

// Adjacent string literals joined only by "+" compose one runtime value;
// "car" + "go" must be audited exactly like the literal "cargo".
const composeLiteralChains = (literals, maskedSource) => {
  const chains = [];
  for (let index = 0; index < literals.length; index += 1) {
    const chain = { end: literals[index].end, index: literals[index].index, value: literals[index].value };
    for (let next = index + 1; next < literals.length; next += 1) {
      const separator = maskedSource.slice(chain.end, literals[next].index);
      if (!/^\s*\+\s*$/.test(separator)) break;
      chain.value += literals[next].value;
      chain.end = literals[next].end;
      index = next;
    }
    chains.push({ index: chain.index, value: chain.value });
  }
  return chains;
};

const statementHeadBefore = (masked, tokenIndex) => {
  const window = masked.slice(Math.max(0, tokenIndex - 200), tokenIndex);
  let boundary = -1;
  for (const character of ["\n", "\r", ";", "{", "}", "("]) {
    const position = window.lastIndexOf(character);
    if (position > boundary) boundary = position;
  }
  return window.slice(boundary + 1).trim();
};

const classifyLaunchKind = (head) => {
  if (head === "") return "bare-command";
  if (/^&\s*$/.test(head)) return "call-operator";
  if (/^&\s*\(/.test(head)) return "call-operator-expression";
  if (/\bcmd(?:\.exe)?\b[^;{\r\n]*\/(?:c|k)\b/i.test(head)) return "cmd-indirection";
  if (/\bstart-process\b/i.test(head)) return "start-process";
  if (/\binvoke-item\b/i.test(head)) return "item-spawner";
  if (/\bstart-(?:job|threadjob)\b/i.test(head)) return "background-job-spawner";
  if (/\binvoke-(?:wmimethod|cimmethod)\b/i.test(head)) return "wmi-cim-process-spawner";
  if (/\[\s*(?:system\.)?diagnostics\.process\s*\]\s*::\s*start\b/i.test(head)) {
    return "dotnet-process-spawner";
  }
  if (/\bnew-object\b[^;{\r\n]*diagnostics\.process/i.test(head)) return "dotnet-process-spawner";
  if (/\b(?:pwsh|powershell)(?:\.exe)?\b[^;{\r\n]*\s-(?:command|c)\b/i.test(head)) {
    return "shell-indirection";
  }
  if (/\b(?:invoke-expression|iex)\b/i.test(head)) return "invoke-expression";
  if (/[&|]\s*$/.test(head)) return "operator-chained";
  return null;
};

const findStaticCargoLaunchAttempts = ({ literals, masked }) => {
  const attempts = [];
  const aliases = new Map();
  const chains = composeLiteralChains(literals, masked);
  for (const chain of chains) {
    if (!CARGO_COMMAND_LITERAL.test(chain.value)) continue;
    const head = masked.slice(Math.max(0, chain.index - 120), chain.index);
    const assignment = ALIAS_ASSIGNMENT_PATTERN.exec(head);
    if (assignment) aliases.set(resolveAliasAssignmentKey(assignment), chain.index);
  }
  const consider = (tokenIndex, description) => {
    const kind = classifyLaunchKind(statementHeadBefore(masked, tokenIndex));
    if (kind) attempts.push({ description, index: tokenIndex, kind });
  };
  for (const token of masked.matchAll(CARGO_TOKEN_PATTERN)) {
    consider(token.index, "unquoted cargo token");
  }
  for (const chain of chains) {
    if (CARGO_COMMAND_STRING.test(chain.value)) {
      consider(chain.index, `quoted cargo command string '${chain.value.trim()}'`);
    }
  }
  for (const [name] of aliases) {
    for (const use of masked.matchAll(aliasUsePattern(name))) {
      consider(use.index, `static cargo alias $${name}`);
    }
  }
  return attempts;
};

const findDynamicExecutionPrimitives = ({ masked }) =>
  [...masked.matchAll(DYNAMIC_EXECUTION_PATTERN)].map((primitive) => ({
    index: primitive.index,
    kind: "dynamic-execution",
  }));

const findBalancedBraceEnd = (masked, openBraceIndex) => {
  let depth = 0;
  for (let index = openBraceIndex; index < masked.length; index += 1) {
    if (masked[index] === "{") depth += 1;
    else if (masked[index] === "}") {
      depth -= 1;
      if (depth === 0) return index;
    }
  }
  return -1;
};

const findFunctionExtent = (masked, functionName) => {
  const header = new RegExp(`function\\s+${functionName}(?![A-Za-z0-9_-])`).exec(masked);
  if (!header) return null;
  const openBrace = masked.indexOf("{", header.index);
  const closeBrace = openBrace === -1 ? -1 : findBalancedBraceEnd(masked, openBrace);
  return { closeBrace, openBrace, start: header.index };
};

const findIfBlockExtent = (masked, condition) => {
  const conditionMatch = condition.exec(masked);
  if (!conditionMatch) return null;
  const openBrace = masked.indexOf("{", conditionMatch.index);
  const closeBrace = openBrace === -1 ? -1 : findBalancedBraceEnd(masked, openBrace);
  return { closeBrace, openBrace, start: conditionMatch.index };
};

const MSVC_TOOLSET_ROOT_PATTERN =
  /[A-Za-z]:\\Program Files\\Microsoft Visual Studio\\2022\\[^\\"]+\\VC\\Tools\\MSVC\\\d+\.\d+\.\d+/g;

// Exact constant roots, not version substrings: every MSVC toolset literal in a
// gated harness must equal REQUIRED_VCTOOLS_INSTALL_DIR exactly, and sources
// that embed the absolute linker must state the full REQUIRED_MSVS_LINKER value.
const verifyExactLocalCommunityToolsetRoots = (source, linkerLiteralRequired) => {
  const violations = [];
  const roots = [...new Set(source.match(MSVC_TOOLSET_ROOT_PATTERN) ?? [])];
  if (!roots.includes(REQUIRED_VCTOOLS_INSTALL_DIR)) {
    violations.push(
      `source must state the exact required Community VCToolsInstallDir ${REQUIRED_VCTOOLS_INSTALL_DIR}`,
    );
  }
  for (const root of roots) {
    if (root !== REQUIRED_VCTOOLS_INSTALL_DIR) {
      violations.push(
        `MSVC toolset root drifted from the exact Community ${REQUIRED_VCTOOLS_VERSION} pin: ${root}`,
      );
    }
  }
  if (linkerLiteralRequired && !source.includes(REQUIRED_MSVS_LINKER)) {
    violations.push(
      `source must state the exact absolute linker ${REQUIRED_MSVS_LINKER}, not a version substring`,
    );
  }
  return violations;
};

const EXPECTED_ASIO_CARGO_DISPATCH_SITES = 4;
const EXPECTED_ASIO_DISPATCHER_CARGO_LAUNCHES = 2;
// Exact occurrence inventory of Test-MsvcLinkerPinContract in the gated
// harness: one function definition, two Invoke-SelfTest matrix calls, and
// exactly one production gate call. The production call is identified
// structurally (outside the definition name and the Invoke-SelfTest extent),
// never by last-occurrence position.
const EXPECTED_ASIO_LINKER_GATE_OCCURRENCES = 4;
const EXPECTED_ASIO_PRODUCTION_LINKER_GATE_CALLS = 1;

const verifyAsioBuildHarnessStructure = (source) => {
  const violations = [...verifyExactLocalCommunityToolsetRoots(source, false)];
  const { literals, masked } = analyzePowerShellSource(source);
  if (!source.includes('"bin\\Hostx64\\x64\\link.exe"')) {
    violations.push('the harness must compose the pinned linker as Join-Path ... "bin\\Hostx64\\x64\\link.exe"');
  }
  const dispatcher = findFunctionExtent(masked, "Invoke-CargoChecked");
  if (!dispatcher || dispatcher.closeBrace === -1) {
    violations.push("the single Invoke-CargoChecked dispatcher function must remain parseable");
    return violations;
  }

  // The hermetic -SelfTest short-circuit must invoke Invoke-SelfTest and return
  // before the Windows environment gate, the verified-linker gate, and any
  // Cargo gate; deleting or reordering it must fail this checker.
  const windowsGateIndex = masked.indexOf("[Environment]::OSVersion.Platform");
  if (windowsGateIndex === -1) {
    violations.push("the Windows-only environment gate must remain");
  }
  const preflightGuard = findIfBlockExtent(masked, /if\s*\(\s*\$PreflightOnly\s*\)/);
  if (!preflightGuard || preflightGuard.closeBrace === -1) {
    violations.push("the if ($PreflightOnly) no-Cargo exit must remain");
  }
  const whereLogIndex = source.indexOf("where.exe link.exe resolution order:");
  if (whereLogIndex === -1) {
    violations.push("where.exe link.exe resolution-order logging must remain");
  }
  const selfTestBlock = findIfBlockExtent(masked, /if\s*\(\s*\$SelfTest\s*\)/);
  if (!selfTestBlock || selfTestBlock.closeBrace === -1) {
    violations.push("the if ($SelfTest) short-circuit block must remain");
  } else {
    const body = source.slice(selfTestBlock.openBrace + 1, selfTestBlock.closeBrace).trim();
    if (!/^Invoke-SelfTest\s*\r?\n\s*return\b/.test(body)) {
      violations.push("the if ($SelfTest) block must invoke Invoke-SelfTest and then return");
    }
    if (windowsGateIndex !== -1 && selfTestBlock.closeBrace > windowsGateIndex) {
      violations.push("the if ($SelfTest) short-circuit must return before the Windows environment gate");
    }
  }

  // The production verified-linker gate is pinned structurally, not by
  // last-occurrence position: the harness must carry exactly the required
  // occurrence inventory, and the single production call must be uniquely
  // identified outside the definition name and the Invoke-SelfTest extent,
  // ordered after the Windows gate and the -SelfTest short-circuit but before
  // the -PreflightOnly exit. A dead branch or a late decoy can no longer
  // satisfy the gate merely by being the final mention.
  const linkerGateHeader = /function\s+Test-MsvcLinkerPinContract(?![A-Za-z0-9_-])/.exec(masked);
  if (!linkerGateHeader) {
    violations.push("the Test-MsvcLinkerPinContract contract function definition must remain parseable");
  }
  const linkerGateDefinitionNameIndex = linkerGateHeader
    ? linkerGateHeader.index + linkerGateHeader[0].indexOf("Test-MsvcLinkerPinContract")
    : -1;
  const linkerGateOccurrences = [...masked.matchAll(/\bTest-MsvcLinkerPinContract\b/g)]
    .map((occurrence) => occurrence.index);
  if (linkerGateOccurrences.length !== EXPECTED_ASIO_LINKER_GATE_OCCURRENCES) {
    violations.push(
      `expected exactly ${EXPECTED_ASIO_LINKER_GATE_OCCURRENCES} `
      + "Test-MsvcLinkerPinContract occurrences (definition, two self-test calls, one production call), "
      + `found ${linkerGateOccurrences.length}`,
    );
  }
  const selfTestFunctionExtent = findFunctionExtent(masked, "Invoke-SelfTest");
  const insideSelfTestFunction = (candidateIndex) =>
    selfTestFunctionExtent !== null
    && selfTestFunctionExtent.closeBrace !== -1
    && candidateIndex >= selfTestFunctionExtent.start
    && candidateIndex <= selfTestFunctionExtent.closeBrace;
  const productionLinkerGates = linkerGateOccurrences.filter(
    (occurrenceIndex) =>
      occurrenceIndex !== linkerGateDefinitionNameIndex && !insideSelfTestFunction(occurrenceIndex),
  );
  if (productionLinkerGates.length !== EXPECTED_ASIO_PRODUCTION_LINKER_GATE_CALLS) {
    violations.push(
      `expected exactly ${EXPECTED_ASIO_PRODUCTION_LINKER_GATE_CALLS} production `
      + "Test-MsvcLinkerPinContract gate call outside the definition and Invoke-SelfTest, "
      + `found ${productionLinkerGates.length}`,
    );
  }
  const productionLinkerGateIndex =
    productionLinkerGates.length === EXPECTED_ASIO_PRODUCTION_LINKER_GATE_CALLS
      ? productionLinkerGates[0]
      : -1;
  if (productionLinkerGateIndex === -1) {
    violations.push("the production Test-MsvcLinkerPinContract verified-linker gate must remain");
  } else {
    if (
      selfTestBlock
      && selfTestBlock.closeBrace !== -1
      && selfTestBlock.closeBrace > productionLinkerGateIndex
    ) {
      violations.push("the if ($SelfTest) short-circuit must return before the verified-linker gate");
    }
    if (windowsGateIndex !== -1 && productionLinkerGateIndex < windowsGateIndex) {
      violations.push(
        "the production Test-MsvcLinkerPinContract verified-linker gate must follow the Windows environment gate",
      );
    }
    if (preflightGuard && preflightGuard.closeBrace !== -1 && productionLinkerGateIndex > preflightGuard.start) {
      violations.push(
        "the production Test-MsvcLinkerPinContract verified-linker gate must precede the -PreflightOnly no-Cargo exit",
      );
    }
  }

  // Dispatch-site discovery independent of parameter order: every production
  // call site is found by name alone and each must sit after the where.exe
  // log, the verified-linker gate, and the -PreflightOnly no-Cargo exit.
  const definitionHeader = /function\s+Invoke-CargoChecked(?![A-Za-z0-9_-])/.exec(masked);
  const definitionNameIndex = definitionHeader
    ? definitionHeader.index + definitionHeader[0].indexOf("Invoke-CargoChecked")
    : -1;
  const dispatchSites = [...masked.matchAll(/\bInvoke-CargoChecked\b/g)]
    .map((occurrence) => occurrence.index)
    .filter((siteIndex) => siteIndex !== definitionNameIndex);
  if (dispatchSites.length !== EXPECTED_ASIO_CARGO_DISPATCH_SITES) {
    violations.push(
      `expected exactly ${EXPECTED_ASIO_CARGO_DISPATCH_SITES} Invoke-CargoChecked production dispatch sites, found ${dispatchSites.length}`,
    );
  }
  for (const siteIndex of dispatchSites) {
    if (whereLogIndex !== -1 && siteIndex < whereLogIndex) {
      violations.push(`dispatch site at offset ${siteIndex} precedes the where.exe resolution-order log`);
    }
    if (productionLinkerGateIndex !== -1 && siteIndex < productionLinkerGateIndex) {
      violations.push(`dispatch site at offset ${siteIndex} precedes the verified-linker gate`);
    }
    if (preflightGuard && preflightGuard.closeBrace !== -1 && siteIndex < preflightGuard.closeBrace) {
      violations.push(`dispatch site at offset ${siteIndex} precedes the -PreflightOnly no-Cargo exit`);
    }
  }

  // Every Cargo launch attempt must lie inside the dispatcher interior; dynamic
  // execution primitives are forbidden outright because they defeat static
  // auditability.
  const launchAttempts = findStaticCargoLaunchAttempts({ literals, masked });
  if (launchAttempts.length !== EXPECTED_ASIO_DISPATCHER_CARGO_LAUNCHES) {
    violations.push(
      `the dispatcher must own exactly ${EXPECTED_ASIO_DISPATCHER_CARGO_LAUNCHES} static Cargo launches, found ${launchAttempts.length}`,
    );
  }
  for (const attempt of launchAttempts) {
    const insideDispatcher =
      attempt.index >= dispatcher.start && attempt.index <= dispatcher.closeBrace;
    if (!insideDispatcher) {
      violations.push(
        `Cargo launch outside the Invoke-CargoChecked dispatcher (${attempt.kind}: ${attempt.description}) at offset ${attempt.index}`,
      );
    }
  }
  for (const primitive of findDynamicExecutionPrimitives({ masked })) {
    violations.push(`dynamic execution primitive at offset ${primitive.index} must not bypass the static Cargo audit`);
  }
  return violations;
};

deepEqual(
  verifyAsioBuildHarnessStructure(asioBuildHarnessSource),
  [],
  "the real ASIO harness must satisfy the normalized static Cargo-containment contract",
);

const asioSourceAnalysis = analyzePowerShellSource(asioBuildHarnessSource);
equal(
  asioSourceAnalysis.masked.length,
  asioBuildHarnessSource.length,
  "normalization must preserve byte offsets so structural indexes stay valid",
);
ok(
  !asioSourceAnalysis.masked.includes("$($Arguments"),
  "string interiors must be masked so logged cargo text can never fake a launch",
);
equal(
  [...asioSourceAnalysis.masked.matchAll(/\bTest-MsvcLinkerPinContract\b/g)].length,
  EXPECTED_ASIO_LINKER_GATE_OCCURRENCES,
  "the real ASIO harness must carry exactly the pinned Test-MsvcLinkerPinContract occurrence inventory",
);

// Paired quote-scanner probes: $( ) subexpressions inside double-quoted
// strings may contain nested quotes, and the scanner must find the true outer
// terminator both ways — string interiors stay masked (no under-mask) and
// code after the string stays scannable (no over-mask).
{
  const nestedProbeSource = '$message = "count $(Get-Count "items") complete"\nDispatch-Next\n';
  const nestedProbe = analyzePowerShellSource(nestedProbeSource);
  equal(
    nestedProbe.masked.length,
    nestedProbeSource.length,
    "nested-quote normalization must preserve byte offsets",
  );
  ok(
    !nestedProbe.masked.includes("Get-Count"),
    "subexpression interiors inside double-quoted strings must stay masked",
  );
  ok(
    !nestedProbe.masked.includes("items"),
    "quotes nested inside a string subexpression must not leak their text into code positions",
  );
  ok(
    nestedProbe.masked.includes("Dispatch-Next"),
    "code following a nested-quote subexpression string must remain scannable (no over-mask)",
  );
  deepEqual(
    nestedProbe.literals.map((literal) => literal.value),
    ['count $(Get-Count "items") complete'],
    "the outer string literal must span to its true terminator despite the nested quotes",
  );
}
{
  const singleQuoteSubexpressionSource = "$log = \"run $('a' + 'b') done\"\nAfter-Sq\n";
  const singleQuoteSubexpression = analyzePowerShellSource(singleQuoteSubexpressionSource);
  equal(
    singleQuoteSubexpression.masked.length,
    singleQuoteSubexpressionSource.length,
    "single-quote-in-subexpression normalization must preserve byte offsets",
  );
  ok(
    !singleQuoteSubexpression.masked.includes("'a'"),
    "single-quoted fragments inside a string subexpression must stay masked",
  );
  ok(
    singleQuoteSubexpression.masked.includes("After-Sq"),
    "a string containing quoted subexpression content must terminate at its true closing quote",
  );
}
{
  const plainParenStringSource = '$note = "totally (inert) text"\nAfter-Plain\n';
  const plainParenString = analyzePowerShellSource(plainParenStringSource);
  equal(
    plainParenString.masked.length,
    plainParenStringSource.length,
    "plain-paren string normalization must preserve byte offsets",
  );
  ok(!plainParenString.masked.includes("inert"), "plain string interiors must remain fully masked");
  ok(
    plainParenString.masked.includes("After-Plain"),
    "parentheses inside a plain string must never open subexpression tracking or delay its termination",
  );
}

let hostileFixtureCount = 0;
const expectHostileViolation = (label, violations, pattern) => {
  hostileFixtureCount += 1;
  ok(
    violations.length > 0 && violations.some((violation) => pattern.test(violation)),
    `hostile mutation fixture '${label}' must fail closed with a matching violation: ${JSON.stringify(violations)}`,
  );
};
const rejectAsioMutation = (label, mutant, pattern) => {
  ok(mutant !== asioBuildHarnessSource, `hostile mutation fixture '${label}' must alter the source`);
  expectHostileViolation(label, verifyAsioBuildHarnessStructure(mutant), pattern);
};
const insertIntoAsioBeforePreflightGuard = (insertion) => {
  const anchor = asioBuildHarnessSource.indexOf("if ($PreflightOnly)");
  if (anchor === -1) throw new Error("fixture anchor missing: if ($PreflightOnly)");
  return asioBuildHarnessSource.slice(0, anchor) + insertion + asioBuildHarnessSource.slice(anchor);
};
const asioSelfTestBlockExtent = () => {
  const blockStart = asioBuildHarnessSource.indexOf("if ($SelfTest)");
  const blockEnd = asioBuildHarnessSource.indexOf("}\n", blockStart);
  if (blockStart === -1 || blockEnd === -1) throw new Error("fixture anchor missing: if ($SelfTest)");
  return { blockEnd: blockEnd + "}\n".length, blockStart };
};
const asioSelfTestBlockText = asioBuildHarnessSource.slice(
  asioSelfTestBlockExtent().blockStart,
  asioSelfTestBlockExtent().blockEnd,
);
equal(asioSelfTestBlockText, "if ($SelfTest) {\n    Invoke-SelfTest\n    return\n}\n");

rejectAsioMutation(
  "bare cargo launch outside the dispatcher",
  insertIntoAsioBeforePreflightGuard("cargo test --locked\n"),
  /bare-command/,
);
rejectAsioMutation(
  "quoted call-operator cargo launch",
  insertIntoAsioBeforePreflightGuard('& "cargo" --version\n'),
  /call-operator/,
);
rejectAsioMutation(
  "Start-Process cargo launch",
  insertIntoAsioBeforePreflightGuard('Start-Process -FilePath cargo -ArgumentList "build"\n'),
  /start-process/,
);
rejectAsioMutation(
  "cmd.exe /c cargo launch",
  insertIntoAsioBeforePreflightGuard("cmd.exe /c cargo publish --dry-run\n"),
  /cmd-indirection/,
);
rejectAsioMutation(
  "cmd /c cargo launch",
  insertIntoAsioBeforePreflightGuard("cmd /c cargo publish --dry-run\n"),
  /cmd-indirection/,
);
rejectAsioMutation(
  "cmd.exe with a quoted full cargo command string",
  insertIntoAsioBeforePreflightGuard('cmd.exe /c "cargo build --locked"\n'),
  /cmd-indirection/,
);
rejectAsioMutation(
  "Invoke-Expression cargo launch",
  insertIntoAsioBeforePreflightGuard('Invoke-Expression "cargo build"\n'),
  /dynamic execution primitive/,
);
rejectAsioMutation(
  "statically bound cargo variable indirection",
  insertIntoAsioBeforePreflightGuard("$staticallyPinnedCargo = 'cargo'\n& $staticallyPinnedCargo test\n"),
  /static cargo alias \$staticallyPinnedCargo/,
);
rejectAsioMutation(
  "environment-variable cargo alias indirection",
  insertIntoAsioBeforePreflightGuard("$env:CargoTool = 'cargo'\n& $env:CargoTool --version\n"),
  /static cargo alias \$env:CargoTool/,
);
rejectAsioMutation(
  "braced environment-variable cargo alias indirection",
  insertIntoAsioBeforePreflightGuard('${env:CargoToolBraced} = "cargo"\n& ${env:CargoToolBraced} test\n'),
  /static cargo alias \$env:CargoToolBraced/,
);
rejectAsioMutation(
  "script-scope cargo alias indirection",
  insertIntoAsioBeforePreflightGuard("$script:CargoExe = 'cargo'\n& $script:CargoExe test\n"),
  /static cargo alias \$script:CargoExe/,
);
rejectAsioMutation(
  "global-scope cargo alias indirection",
  insertIntoAsioBeforePreflightGuard('$global:CargoExe = "cargo"\n& $global:CargoExe test\n'),
  /static cargo alias \$global:CargoExe/,
);
rejectAsioMutation(
  "concatenated-string cargo alias indirection",
  insertIntoAsioBeforePreflightGuard('$assembledCommand = "car" + "go"\n& $assembledCommand test\n'),
  /static cargo alias \$assembledCommand/,
);
rejectAsioMutation(
  "direct concatenated-string cargo invocation",
  insertIntoAsioBeforePreflightGuard('& ("car" + "go") --version\n'),
  /bare-command/,
);
rejectAsioMutation(
  "Invoke-Item cargo spawner",
  insertIntoAsioBeforePreflightGuard("Invoke-Item cargo.exe\n"),
  /item-spawner/,
);
rejectAsioMutation(
  "Start-ThreadJob cargo spawner",
  insertIntoAsioBeforePreflightGuard("Start-ThreadJob -Name soakgate cargo build\n"),
  /background-job-spawner/,
);
rejectAsioMutation(
  "Invoke-WmiMethod Win32_Process cargo spawner",
  insertIntoAsioBeforePreflightGuard(
    'Invoke-WmiMethod -Class Win32_Process -MethodName Create -ArgumentList "cargo build"\n',
  ),
  /wmi-cim-process-spawner/,
);
rejectAsioMutation(
  ".NET Process.Start cargo spawner",
  insertIntoAsioBeforePreflightGuard('[System.Diagnostics.Process]::Start("cargo", "build")\n'),
  // The "(" call boundary collapses the statement head, so this paren-delimited
  // form fails closed as a bare command outside the dispatcher; the dedicated
  // dotnet-process-spawner classification remains for unparenthesized heads.
  /bare-command|dotnet-process-spawner/,
);
rejectAsioMutation(
  "pwsh -Command cargo re-invocation",
  insertIntoAsioBeforePreflightGuard("pwsh -NoProfile -Command cargo test\n"),
  /shell-indirection/,
);

// Negative controls: launch-shaped text in comments, string interiors, and
// here-strings — plus benign literal concatenation — must never be flagged.
deepEqual(
  verifyAsioBuildHarnessStructure(
    insertIntoAsioBeforePreflightGuard(
      "# Invoke-Item cargo.exe would be rejected if it were code.\n# $env:CargoTool = 'cargo'\n",
    ),
  ),
  [],
  "comment-mentioned launch constructs must never be flagged",
);
deepEqual(
  verifyAsioBuildHarnessStructure(
    insertIntoAsioBeforePreflightGuard(
      '$auditNote = "Invoke-Item cargo.exe; Start-Job { cargo publish }; & `$env:CargoTool test"\n',
    ),
  ),
  [],
  "string-interior launch constructs must never be flagged",
);
deepEqual(
  verifyAsioBuildHarnessStructure(
    insertIntoAsioBeforePreflightGuard("@'\nInvoke-Expression \"cargo build\"\ncargo publish --dry-run\n'@\n"),
  ),
  [],
  "here-string launch constructs must never be flagged",
);
deepEqual(
  verifyAsioBuildHarnessStructure(
    insertIntoAsioBeforePreflightGuard('$salutation = "con" + "cat" + "enate"\nWrite-Host $salutation\n'),
  ),
  [],
  "benign literal concatenation and variable use must never be flagged",
);

const relocateLastAsioDispatchBeforePreflight = () => {
  const callStart = asioBuildHarnessSource.indexOf(
    '    Invoke-CargoChecked -Label "ASIO canonical release DLL"',
  );
  const callEnd = asioBuildHarnessSource.indexOf("\n    )\n", callStart);
  if (callStart === -1 || callEnd === -1) throw new Error("fixture anchor missing: release DLL dispatch");
  const callText = asioBuildHarnessSource.slice(callStart, callEnd + "\n    )\n".length);
  const withoutCall =
    asioBuildHarnessSource.slice(0, callStart) + asioBuildHarnessSource.slice(callEnd + "\n    )\n".length);
  const anchor = withoutCall.indexOf("if ($PreflightOnly)");
  return withoutCall.slice(0, anchor) + callText + withoutCall.slice(anchor);
};
rejectAsioMutation(
  "dispatch site relocated before the preflight exit",
  relocateLastAsioDispatchBeforePreflight(),
  /precedes the -PreflightOnly no-Cargo exit/,
);

rejectAsioMutation(
  "deleted self-test short-circuit",
  asioBuildHarnessSource.replace(asioSelfTestBlockText, ""),
  /if \(\$SelfTest\) short-circuit block must remain/,
);
{
  const { blockEnd, blockStart } = asioSelfTestBlockExtent();
  const withoutBlock =
    asioBuildHarnessSource.slice(0, blockStart) + asioBuildHarnessSource.slice(blockEnd);
  const anchor = withoutBlock.indexOf("if ($PreflightOnly)");
  const delayedMutant =
    withoutBlock.slice(0, anchor) + `${asioSelfTestBlockText}\n` + withoutBlock.slice(anchor);
  rejectAsioMutation(
    "self-test short-circuit reordered past the environment and linker gates",
    delayedMutant,
    /short-circuit must return before the Windows environment gate|short-circuit must return before the verified-linker gate/,
  );
}

{
  const callStart = asioBuildHarnessSource.indexOf(
    'Invoke-CargoChecked -Label "SDK-free ABI and lifecycle tests"',
  );
  const argumentsOpen = asioBuildHarnessSource.indexOf("-Arguments @(", callStart);
  const callClose = asioBuildHarnessSource.indexOf(")\n", argumentsOpen);
  ok(
    callStart > -1 && argumentsOpen > callStart && callClose > argumentsOpen,
    "the first SDK-free dispatch must be extractable for the parameter-order fixture",
  );
  const label = '"SDK-free ABI and lifecycle tests"';
  const argumentsText = asioBuildHarnessSource.slice(argumentsOpen + "-Arguments @(".length, callClose + 1);
  const parameterOrderMutant =
    asioBuildHarnessSource.slice(0, callStart)
    + `Invoke-CargoChecked -Arguments @(${argumentsText}) -Label ${label}`
    + asioBuildHarnessSource.slice(callClose + 1);
  deepEqual(
    verifyAsioBuildHarnessStructure(parameterOrderMutant),
    [],
    "dispatch-site discovery and gating must be independent of named-parameter order",
  );
}

// The former last-occurrence linker-gate bound was gameable: deleting the real
// production gate while parking a decoy occurrence between the -PreflightOnly
// exit and the dispatch sites kept every positional check green. The exact
// occurrence inventory, unique structural production-call identification, and
// authoritative post-selfTest/pre-PreflightOnly position replace it.
const asioProductionGateStatement = () => {
  const statementStart = asioBuildHarnessSource.indexOf(
    "$linkerPinContract = Test-MsvcLinkerPinContract",
  );
  const statementTail = "-LinkResolutionOrder $whereLinkResolution.Entries\n";
  const statementEnd =
    statementStart === -1 ? -1 : asioBuildHarnessSource.indexOf(statementTail, statementStart);
  if (statementStart === -1 || statementEnd === -1) {
    throw new Error("fixture anchor missing: production Test-MsvcLinkerPinContract gate statement");
  }
  return asioBuildHarnessSource.slice(statementStart, statementEnd + statementTail.length);
};
const asioDeadBranchGate =
  "if ($false) {\n    $linkerPinContract = Test-MsvcLinkerPinContract\n}\n";
{
  // Count-neutral gameable shape: the genuine production gate disappears and
  // exactly one decoy lands between the -PreflightOnly exit and the dispatch
  // sites, so the occurrence inventory (4) and uniqueness (1) both still hold
  // while nothing enforces the linker pin before Cargo. Last-occurrence logic
  // accepted this mutant; only the authoritative pre-PreflightOnly position
  // pin rejects it.
  const withoutGate = asioBuildHarnessSource.replace(asioProductionGateStatement(), "");
  const dispatchAnchor = withoutGate.indexOf(
    'Invoke-CargoChecked -Label "SDK-free ABI and lifecycle tests"',
  );
  if (dispatchAnchor === -1) throw new Error("fixture anchor missing: SDK-free dispatch");
  const decoyLineStart = withoutGate.lastIndexOf("\n", dispatchAnchor) + 1;
  const gameableMutant =
    withoutGate.slice(0, decoyLineStart)
    + "$linkerPinContract = Test-MsvcLinkerPinContract | Out-Null\n"
    + withoutGate.slice(decoyLineStart);
  rejectAsioMutation(
    "removed linker gate with a pre-dispatch decoy (former gameable shape)",
    gameableMutant,
    /verified-linker gate must precede/,
  );
}
{
  // Mandated combined hostile: dead-branch gate relocation plus a late decoy
  // plus an early relocated dispatch site must fail closed on the exact
  // occurrence inventory and the unique structural production-call identity.
  const withoutGate = asioBuildHarnessSource.replace(asioProductionGateStatement(), "");
  const selfTestAnchor = withoutGate.indexOf("if ($SelfTest)");
  const withDeadBranch =
    withoutGate.slice(0, selfTestAnchor) + asioDeadBranchGate + withoutGate.slice(selfTestAnchor);
  const callStart = withDeadBranch.indexOf(
    '    Invoke-CargoChecked -Label "SDK-free ABI and lifecycle tests"',
  );
  const callEnd = withDeadBranch.indexOf("\n    )\n", callStart);
  if (callStart === -1 || callEnd === -1) {
    throw new Error("fixture anchor missing: SDK-free dispatch block");
  }
  const callText = withDeadBranch.slice(callStart, callEnd + "\n    )\n".length);
  const hollowed =
    withDeadBranch.slice(0, callStart) + withDeadBranch.slice(callEnd + "\n    )\n".length);
  const guardMarker = 'Write-Host "ASIO preflight-only gate passed; Cargo was not invoked."';
  const guardMarkerIndex = hollowed.indexOf(guardMarker);
  if (guardMarkerIndex === -1) throw new Error("fixture anchor missing: preflight-only marker");
  const guardClose = hollowed.indexOf("}\n", guardMarkerIndex) + "}\n".length;
  const relocatedDispatch = hollowed.slice(0, guardClose) + "\n" + callText + hollowed.slice(guardClose);
  rejectAsioMutation(
    "dead-branch linker gate, late decoy, and early relocated dispatch",
    `${relocatedDispatch}\nTest-MsvcLinkerPinContract | Out-Null\n`,
    /exactly 4 Test-MsvcLinkerPinContract occurrences|exactly 1 production Test-MsvcLinkerPinContract gate call/,
  );
}
{
  // Count-preserving demotion: relocating the genuine gate above the -SelfTest
  // short-circuit must violate the authoritative post-selfTest position pin.
  const withoutGate = asioBuildHarnessSource.replace(asioProductionGateStatement(), "");
  const anchor = withoutGate.indexOf("if ($SelfTest)");
  rejectAsioMutation(
    "production linker gate demoted above the self-test short-circuit",
    withoutGate.slice(0, anchor) + asioProductionGateStatement() + withoutGate.slice(anchor),
    /short-circuit must return before the verified-linker gate|must follow the Windows environment gate/,
  );
}

rejectAsioMutation(
  "Community-to-Enterprise edition-root drift",
  asioBuildHarnessSource.split("2022\\Community\\").join("2022\\Enterprise\\"),
  /drifted from the exact Community 14\.44\.35207 pin|exact required Community VCToolsInstallDir/,
);
match(
  asioBuildHarnessSource,
  /where\.exe link\.exe resolution order[\s\S]*?Pinned host x64 linker/,
  "ASIO harness must log the real linker order and accepted pin before Cargo",
);

for (const [needle, reason] of [
  ["-vcvars_ver=14.44", "initialize the exact supported vcvars toolset"],
  ["CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER", "pin Cargo to the exact Hostx64/x64 linker"],
  ["where.exe link.exe", "prove and print the real linker search order"],
  ["[switch]$PreflightOnly", "provide a live no-Cargo preflight mode"],
  ["must never be present on a build or soak-harness run", "reject hermetic test overrides on production execution"],
]) {
  ok(soakHarnessSource.includes(needle), `soak direct-Cargo harness must ${reason}`);
}
const verifySoakHarnessStructure = (source) => {
  const violations = [...verifyExactLocalCommunityToolsetRoots(source, true)];
  const { literals, masked } = analyzePowerShellSource(source);
  const overrideGuardIndex = masked.indexOf(
    "$null -ne $global:SYNDOCAL_RUN_SOAK_TEST_DEPENDENCIES",
  );
  const livePreflightIndex = masked.indexOf("$verifiedEnvironment = Invoke-SoakMsvcPreflight");
  const preflightGuard = findIfBlockExtent(masked, /if\s*\(\s*\$PreflightOnly\s*\)/);
  if (overrideGuardIndex === -1) {
    violations.push("the hermetic test-dependency override guard must remain");
  }
  if (livePreflightIndex === -1) {
    violations.push("the verified Invoke-SoakMsvcPreflight gate must remain");
  }
  if (!preflightGuard || preflightGuard.closeBrace === -1) {
    violations.push("the if ($PreflightOnly) no-Cargo exit must remain");
  }
  if (overrideGuardIndex > -1 && livePreflightIndex > -1 && livePreflightIndex < overrideGuardIndex) {
    violations.push("the verified MSVC preflight must follow the test-dependency override rejection");
  }
  if (
    preflightGuard
    && preflightGuard.closeBrace !== -1
    && livePreflightIndex > -1
    && preflightGuard.start < livePreflightIndex
  ) {
    violations.push("the -PreflightOnly exit must follow the verified MSVC preflight");
  }
  const launchAttempts = findStaticCargoLaunchAttempts({ literals, masked });
  if (launchAttempts.length !== 1) {
    violations.push(`the soak harness must contain exactly one direct Cargo launch, found ${launchAttempts.length}`);
  }
  for (const attempt of launchAttempts) {
    if (livePreflightIndex > -1 && attempt.index < livePreflightIndex) {
      violations.push(`Cargo launch at offset ${attempt.index} precedes the verified MSVC preflight`);
    }
    if (
      preflightGuard
      && preflightGuard.closeBrace !== -1
      && attempt.index < preflightGuard.closeBrace
    ) {
      violations.push(`Cargo launch at offset ${attempt.index} precedes the -PreflightOnly no-Cargo exit`);
    }
  }
  for (const primitive of findDynamicExecutionPrimitives({ masked })) {
    violations.push(`dynamic execution primitive at offset ${primitive.index} must not bypass the static Cargo audit`);
  }
  return violations;
};

deepEqual(
  verifySoakHarnessStructure(soakHarnessSource),
  [],
  "the real soak harness must satisfy the normalized static Cargo-containment contract",
);
expectHostileViolation(
  "soak Community-to-Enterprise edition-root drift",
  verifySoakHarnessStructure(soakHarnessSource.split("2022\\Community\\").join("2022\\Enterprise\\")),
  /drifted from the exact Community 14\.44\.35207 pin|exact required Community VCToolsInstallDir/,
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

console.log(
  `tauri build wrapper checks passed (${assertionCount} assertions, ${hostileFixtureCount} hostile mutation fixtures)`,
);
