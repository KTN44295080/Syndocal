import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

import {
  isNativeReleaseBuild,
  releaseExecutablePath,
} from "./run-tauri.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const appDir = path.resolve(scriptDir, "..");
const packageJson = JSON.parse(
  await readFile(path.join(appDir, "package.json"), "utf8"),
);
const wrapperSource = await readFile(path.join(scriptDir, "run-tauri.mjs"), "utf8");

assert.equal(
  packageJson.scripts.tauri,
  "node scripts/run-tauri.mjs",
  "pnpm tauri must pass through the guarded wrapper",
);
assert.equal(isNativeReleaseBuild(["build", "--no-bundle"]), true);
assert.equal(isNativeReleaseBuild(["dev"]), false);
assert.equal(
  releaseExecutablePath(appDir),
  path.resolve(appDir, "..", "target", "release", "syndocal.exe"),
);
assert.match(wrapperSource, /StringComparer\]::OrdinalIgnoreCase\.Equals\(\$actual, \$expected\)/);
assert.match(wrapperSource, /Stop-Process -Id \$pidToStop -Force/);
assert.match(wrapperSource, /Get-ExactSyndocalProcess/);
assert.match(wrapperSource, /Refusing to build while the checkout release executable cannot be stopped/);

console.log("tauri build wrapper checks passed (8 assertions)");
