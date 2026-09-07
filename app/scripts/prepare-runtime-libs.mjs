import { spawnSync } from "node:child_process";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { main as prepareReleaseRuntime } from "./prepare-release-runtime.mjs";

const scriptDir = dirname(fileURLToPath(import.meta.url));

export function run(platform = process.platform) {
  if (platform === "win32") {
    const check = spawnSync(process.execPath, [join(scriptDir, "check-asio-packaging.mjs")], {
      cwd: join(scriptDir, ".."),
      stdio: "inherit",
      windowsHide: true,
    });
    if (check.error) throw check.error;
    if (check.status !== 0) return check.status ?? 1;
  }
  prepareReleaseRuntime({ platform });
  return 0;
}

const invokedDirectly = process.argv[1]
  && fileURLToPath(import.meta.url) === process.argv[1];

if (invokedDirectly) {
  try {
    process.exitCode = run();
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
