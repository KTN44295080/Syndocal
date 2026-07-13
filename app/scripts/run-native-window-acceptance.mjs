import { spawn } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const appRoot = resolve(scriptDir, "..");

if (process.platform !== "win32") {
  console.log(
    "SKIP native 1920 acceptance: the automated maximized/F11/Escape gate currently targets Windows; run equivalent native evidence on this host before release sign-off.",
  );
  process.exit(0);
}

const child = spawn(
  "powershell.exe",
  [
    "-NoLogo",
    "-NoProfile",
    "-ExecutionPolicy",
    "Bypass",
    "-File",
    resolve(scriptDir, "check-native-window-acceptance.ps1"),
    ...process.argv.slice(2),
  ],
  {
    cwd: appRoot,
    stdio: "inherit",
    windowsHide: true,
  },
);

child.on("error", (error) => {
  console.error(`native 1920 acceptance could not start: ${error.message}`);
  process.exitCode = 1;
});

child.on("exit", (code, signal) => {
  if (signal) {
    console.error(`native 1920 acceptance ended by ${signal}`);
    process.exitCode = 1;
    return;
  }
  process.exitCode = code ?? 1;
});
