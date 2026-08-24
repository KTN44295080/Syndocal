import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  isNativeReleaseBuild,
  isWindowsNativeCargoCommand,
  releaseExecutablePath,
  tauriCommandEnvironment,
  verifiedNativeBuildEnvironment,
} from "./run-tauri.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const appDir = path.resolve(scriptDir, "..");
const packageJson = JSON.parse(
  await readFile(path.join(appDir, "package.json"), "utf8"),
);
const wrapperSource = await readFile(path.join(scriptDir, "run-tauri.mjs"), "utf8");
const workflowSource = await readFile(
  path.resolve(appDir, "..", ".github", "workflows", "cross-platform.yml"),
  "utf8",
);

assert.equal(
  packageJson.scripts.tauri,
  "node scripts/run-tauri.mjs",
  "pnpm tauri must pass through the guarded wrapper",
);
assert.equal(isNativeReleaseBuild(["build", "--no-bundle"]), true);
assert.equal(isNativeReleaseBuild(["--verbose", "build", "--no-bundle"]), true);
assert.equal(isNativeReleaseBuild(["dev"]), false);
assert.equal(isWindowsNativeCargoCommand(["build", "--no-bundle"]), true);
assert.equal(isWindowsNativeCargoCommand(["dev"]), true);
assert.equal(isWindowsNativeCargoCommand(["--verbose", "dev"]), true);
assert.equal(isWindowsNativeCargoCommand(["info"]), false);
assert.equal(
  releaseExecutablePath(appDir),
  path.resolve(appDir, "..", "target", "release", "syndocal.exe"),
);
const fakeVcToolsDir = path.resolve("C:/VisualStudio/VC/Tools/MSVC/14.43.34808");
const expectedMsvcLinker = path.resolve(fakeVcToolsDir, "bin", "Hostx64", "x64", "link.exe");
const originalEnvironment = {
  VCToolsInstallDir: fakeVcToolsDir,
  CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER: "C:/Program Files/Git/usr/bin/link.exe",
};
const verifiedEnvironment = verifiedNativeBuildEnvironment(
  originalEnvironment,
  "win32",
  (candidate) => candidate === expectedMsvcLinker,
);
assert.equal(
  verifiedEnvironment.CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER,
  expectedMsvcLinker,
  "Windows native builds must replace an inherited Git linker with the asserted MSVC linker",
);
assert.equal(
  tauriCommandEnvironment(
    ["dev"],
    originalEnvironment,
    "win32",
    (candidate) => candidate === expectedMsvcLinker,
  ).CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER,
  expectedMsvcLinker,
  "Windows Tauri dev must use the same asserted MSVC linker as release builds",
);
assert.equal(
  tauriCommandEnvironment(["info"], originalEnvironment, "win32", () => false),
  originalEnvironment,
  "non-Cargo Tauri commands must preserve their environment",
);
assert.equal(
  originalEnvironment.CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER,
  "C:/Program Files/Git/usr/bin/link.exe",
  "linker verification must not mutate the caller's environment",
);
assert.throws(
  () => verifiedNativeBuildEnvironment({}, "win32", () => true),
  /without VCToolsInstallDir/,
);
assert.throws(
  () => verifiedNativeBuildEnvironment({ VCToolsInstallDir: fakeVcToolsDir }, "win32", () => false),
  /asserted MSVC linker is missing/,
);
assert.equal(
  verifiedNativeBuildEnvironment(originalEnvironment, "linux", () => false),
  originalEnvironment,
  "non-Windows builds must preserve their environment",
);
assert.match(wrapperSource, /StringComparer\]::OrdinalIgnoreCase\.Equals\(\$actual, \$expected\)/);
assert.match(wrapperSource, /Stop-Process -Id \$pidToStop -Force/);
assert.match(wrapperSource, /Get-ExactSyndocalProcess/);
assert.match(wrapperSource, /Refusing to build while the checkout release executable cannot be stopped/);
assert.match(wrapperSource, /CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER: linker/);
assert.match(wrapperSource, /const environment = tauriCommandEnvironment\(args\)/);
assert.match(
  wrapperSource,
  /spawnSync\(process\.execPath, \[tauriCli, \.\.\.args\],[\s\S]*?env: environment/,
);
assert.match(wrapperSource, /statSync\(candidate\)\.isFile\(\)/);
assert.match(workflowSource, /uses: ilammy\/msvc-dev-cmd@v1[\s\S]*?arch: x64/);
const msvcEnvironmentIndex = workflowSource.indexOf("uses: ilammy/msvc-dev-cmd@v1");
const directCargoIndexes = [...workflowSource.matchAll(/\bcargo (?:build|check|clippy|fmt|test)\b/g)]
  .map((match) => match.index);
assert.ok(
  directCargoIndexes.length > 0
    && directCargoIndexes.every((commandIndex) => msvcEnvironmentIndex < commandIndex),
  "Windows MSVC environment initialization must precede every direct Cargo command",
);

console.log("tauri build wrapper checks passed (26 assertions)");
