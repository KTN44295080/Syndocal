import { spawnSync } from "node:child_process";
import { statSync } from "node:fs";
import { fileURLToPath, pathToFileURL } from "node:url";
import path from "node:path";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const appDir = path.resolve(scriptDir, "..");

const isRegularFile = (candidate) => {
  try {
    return statSync(candidate).isFile();
  } catch {
    return false;
  }
};

export function tauriSubcommand(args) {
  return args.find((argument) => typeof argument === "string" && !argument.startsWith("-")) ?? "";
}

export function isNativeReleaseBuild(args) {
  return tauriSubcommand(args) === "build";
}

export function isWindowsNativeCargoCommand(args) {
  const command = tauriSubcommand(args);
  return command === "build" || command === "dev";
}

export function releaseExecutablePath(baseAppDir = appDir) {
  return path.resolve(baseAppDir, "..", "target", "release", "syndocal.exe");
}

export function verifiedNativeBuildEnvironment(
  environment = process.env,
  platform = process.platform,
  fileIsRegular = isRegularFile,
) {
  if (platform !== "win32") return environment;

  const vcToolsInstallDir = environment.VCToolsInstallDir?.trim();
  if (!vcToolsInstallDir) {
    throw new Error(
      "Refusing the Windows native build without VCToolsInstallDir. Run it from an x64 Visual Studio Developer Shell.",
    );
  }
  const linker = path.resolve(vcToolsInstallDir, "bin", "Hostx64", "x64", "link.exe");
  if (!fileIsRegular(linker)) {
    throw new Error(`Refusing the Windows native build because the asserted MSVC linker is missing: ${linker}`);
  }

  // Cargo otherwise resolves a bare `link.exe` from PATH. Git for Windows also
  // ships usr/bin/link.exe, which accepts Unix arguments and fails MSVC links.
  // Pin the target linker to the verified Visual C++ binary for every build.
  return {
    ...environment,
    CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER: linker,
  };
}

export function tauriCommandEnvironment(
  args,
  environment = process.env,
  platform = process.platform,
  fileIsRegular = isRegularFile,
) {
  return isWindowsNativeCargoCommand(args)
    ? verifiedNativeBuildEnvironment(environment, platform, fileIsRegular)
    : environment;
}

export function stopCheckoutReleaseExecutable(baseAppDir = appDir) {
  if (process.platform !== "win32") return;

  const expectedPath = releaseExecutablePath(baseAppDir);
  const stopScript = String.raw`
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$expected = [System.IO.Path]::GetFullPath($env:SYNDOCAL_RELEASE_EXE_PATH)

function Get-ExactSyndocalProcess {
  @(
    Get-Process -Name 'syndocal' -ErrorAction SilentlyContinue | ForEach-Object {
      try {
        $actual = [System.IO.Path]::GetFullPath($_.Path)
        if ([System.StringComparer]::OrdinalIgnoreCase.Equals($actual, $expected)) {
          $_
        }
      } catch {
        # An inaccessible or already-exited process is not this verified target.
      }
    }
  )
}

$matches = @(Get-ExactSyndocalProcess)
foreach ($candidate in $matches) {
  $pidToStop = $candidate.Id
  Write-Output "[syndocal-build] stopping PID $pidToStop at $expected"
  Stop-Process -Id $pidToStop -Force -ErrorAction Stop
  if (-not $candidate.WaitForExit(10000)) {
    throw "Syndocal PID $pidToStop did not exit within 10 seconds: $expected"
  }
}

$remaining = @(Get-ExactSyndocalProcess)
if ($remaining.Count -ne 0) {
  $remainingIds = ($remaining | ForEach-Object { $_.Id }) -join ', '
  throw "Syndocal release executable is still running (PID $remainingIds): $expected"
}

if ($matches.Count -eq 0) {
  Write-Output "[syndocal-build] release executable is not running: $expected"
} else {
  Write-Output "[syndocal-build] verified stopped: $expected"
}
`;
  const encodedScript = Buffer.from(stopScript, "utf16le").toString("base64");
  const result = spawnSync(
    "powershell.exe",
    ["-NoLogo", "-NoProfile", "-NonInteractive", "-EncodedCommand", encodedScript],
    {
      cwd: baseAppDir,
      env: {
        ...process.env,
        SYNDOCAL_RELEASE_EXE_PATH: expectedPath,
      },
      stdio: "inherit",
    },
  );

  if (result.error) throw result.error;
  if (result.status !== 0) {
    throw new Error(
      `Refusing to build while the checkout release executable cannot be stopped (exit ${result.status ?? "unknown"}).`,
    );
  }
}

export function runTauri(args, baseAppDir = appDir) {
  if (isNativeReleaseBuild(args)) {
    stopCheckoutReleaseExecutable(baseAppDir);
  }

  const environment = tauriCommandEnvironment(args);

  const tauriCli = path.join(
    baseAppDir,
    "node_modules",
    "@tauri-apps",
    "cli",
    "tauri.js",
  );
  const result = spawnSync(process.execPath, [tauriCli, ...args], {
    cwd: baseAppDir,
    env: environment,
    stdio: "inherit",
  });

  if (result.error) throw result.error;
  return result.status ?? 1;
}

const invokedDirectly =
  process.argv[1] && pathToFileURL(path.resolve(process.argv[1])).href === import.meta.url;

if (invokedDirectly) {
  try {
    process.exitCode = runTauri(process.argv.slice(2));
  } catch (error) {
    console.error(`[syndocal-build] ${error instanceof Error ? error.message : String(error)}`);
    process.exitCode = 1;
  }
}
