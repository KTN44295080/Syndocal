import { spawnSync } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
const checkerPath = resolve(scriptDirectory, "check-control-upper-workspaces-browser.mjs");
const viewports = ["3840x2160", "2560x1440", "1920x1080", "1280x720"];
let allPassed = true;

for (const viewport of viewports) {
  let result;
  try {
    result = spawnSync(process.execPath, [checkerPath], {
      cwd: resolve(scriptDirectory, ".."),
      env: { ...process.env, SYNDOCAL_CONTROL_VIEWPORT: viewport },
      stdio: "inherit",
      windowsHide: true,
    });
  } catch (error) {
    console.error(`control-upper-workspaces ${viewport} failed to spawn: ${error instanceof Error ? error.message : String(error)}`);
    allPassed = false;
    break;
  }

  if (result.error) {
    console.error(`control-upper-workspaces ${viewport} failed to spawn: ${result.error.message}`);
    allPassed = false;
    break;
  }
  if (result.status === null) {
    console.error(`control-upper-workspaces ${viewport} failed closed: child status was null (signal=${result.signal ?? "none"})`);
    allPassed = false;
    break;
  }
  if (result.status !== 0) {
    console.error(`control-upper-workspaces ${viewport} failed with status ${result.status}`);
    allPassed = false;
    break;
  }
}

if (allPassed) {
  console.log(`check:control-upper-workspaces passed: ${viewports.join(", ")}`);
} else {
  process.exitCode = 1;
}
