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

// The Windows native gate is pinned to the exact VS2022 Community / Build Tools MSVC
// 14.44.35207 toolset. Cargo otherwise resolves a bare `link.exe` from PATH,
// and Git for Windows ships usr/bin/link.exe, which accepts Unix arguments and
// fails MSVC links, so the verified linker is always pinned explicitly and any
// resolved VCToolsInstallDir/linker mismatch fails closed.
export const REQUIRED_VCTOOLS_VERSION = "14.44.35207";
export const REQUIRED_VCTOOLS_INSTALL_DIR =
  "C:\\Program Files\\Microsoft Visual Studio\\2022\\Community\\VC\\Tools\\MSVC\\14.44.35207";
export const REQUIRED_MSVS_LINKER = path.win32.resolve(
  REQUIRED_VCTOOLS_INSTALL_DIR,
  "bin",
  "Hostx64",
  "x64",
  "link.exe",
);
export const REQUIRED_VCVARS_BATCH =
  "C:\\Program Files\\Microsoft Visual Studio\\2022\\Community\\VC\\Auxiliary\\Build\\vcvars64.bat";
export const REQUIRED_VCVARS_ARGUMENTS = "-vcvars_ver=14.44";
export const REQUIRED_BUILD_TOOLS_VCTOOLS_INSTALL_DIR =
  "C:\\Program Files (x86)\\Microsoft Visual Studio\\2022\\BuildTools\\VC\\Tools\\MSVC\\14.44.35207";
export const REQUIRED_BUILD_TOOLS_MSVS_LINKER = path.win32.resolve(
  REQUIRED_BUILD_TOOLS_VCTOOLS_INSTALL_DIR, "bin", "Hostx64", "x64", "link.exe",
);
export const REQUIRED_BUILD_TOOLS_VCVARS_BATCH =
  "C:\\Program Files (x86)\\Microsoft Visual Studio\\2022\\BuildTools\\VC\\Auxiliary\\Build\\vcvars64.bat";

// GitHub's hosted windows-2022 image ships Visual Studio Enterprise rather
// than Community.  Keep that edition-root difference explicit and narrowly
// gated; the selected MSVC toolset, host/target architecture, absolute linker
// pin, and where.exe-first requirement remain identical to the local gate.
export const GITHUB_HOSTED_WINDOWS_TOOLCHAIN_MARKER =
  "SYNDOCAL_GITHUB_HOSTED_WINDOWS_MSVC";
export const GITHUB_HOSTED_WINDOWS_TOOLCHAIN_MARKER_VALUE =
  "windows-2022-enterprise";
export const REQUIRED_GITHUB_HOSTED_VCTOOLS_INSTALL_DIR =
  "C:\\Program Files\\Microsoft Visual Studio\\2022\\Enterprise\\VC\\Tools\\MSVC\\14.44.35207";
export const REQUIRED_GITHUB_HOSTED_MSVS_LINKER = path.win32.resolve(
  REQUIRED_GITHUB_HOSTED_VCTOOLS_INSTALL_DIR,
  "bin",
  "Hostx64",
  "x64",
  "link.exe",
);

export function isGitHubHostedWindowsToolchain(environment = process.env) {
  return environment?.GITHUB_ACTIONS === "true"
    && environment?.RUNNER_OS === "Windows"
    && environment?.RUNNER_ENVIRONMENT === "github-hosted"
    && environment?.[GITHUB_HOSTED_WINDOWS_TOOLCHAIN_MARKER]
      === GITHUB_HOSTED_WINDOWS_TOOLCHAIN_MARKER_VALUE;
}

export function requiredMsvcToolchain(environment = process.env) {
  if (isGitHubHostedWindowsToolchain(environment)) {
    return {
      edition: "Enterprise (GitHub-hosted windows-2022)",
      installDir: REQUIRED_GITHUB_HOSTED_VCTOOLS_INSTALL_DIR,
      linker: REQUIRED_GITHUB_HOSTED_MSVS_LINKER,
    };
  }
  if (environment?.VCToolsInstallDir
      && normalizeWindowsPathKey(resolveMsvcHostLinker(environment.VCToolsInstallDir.trim()))
        === normalizeWindowsPathKey(REQUIRED_BUILD_TOOLS_MSVS_LINKER)) {
    return {
      edition: "Build Tools",
      installDir: REQUIRED_BUILD_TOOLS_VCTOOLS_INSTALL_DIR,
      linker: REQUIRED_BUILD_TOOLS_MSVS_LINKER,
    };
  }
  return {
    edition: "Community",
    installDir: REQUIRED_VCTOOLS_INSTALL_DIR,
    linker: REQUIRED_MSVS_LINKER,
  };
}

const normalizeWindowsPathKey = (candidate) =>
  path.win32.normalize(String(candidate)).trim().toLowerCase();

export function resolveMsvcHostLinker(vcToolsInstallDir) {
  return path.win32.resolve(String(vcToolsInstallDir), "bin", "Hostx64", "x64", "link.exe");
}

export function requireExactMsvcToolset(environment, fileIsRegular = isRegularFile) {
  const vcToolsInstallDir = environment?.VCToolsInstallDir?.trim();
  const requiredToolchain = requiredMsvcToolchain(environment);
  if (!vcToolsInstallDir) {
    throw new Error(
      `Refusing the Windows native build without VCToolsInstallDir. Initialize the exact x64 Visual Studio 2022 ${requiredToolchain.edition} environment for MSVC ${REQUIRED_VCTOOLS_VERSION}.`,
    );
  }
  const resolvedLinker = resolveMsvcHostLinker(vcToolsInstallDir);
  if (normalizeWindowsPathKey(resolvedLinker) !== normalizeWindowsPathKey(requiredToolchain.linker)) {
    throw new Error(
      `Refusing the Windows native build because the resolved MSVC toolset does not match the required Visual Studio 2022 ${requiredToolchain.edition} ${REQUIRED_VCTOOLS_VERSION} toolset. Resolved linker: ${resolvedLinker}. Required linker: ${requiredToolchain.linker}.`,
    );
  }
  if (!fileIsRegular(resolvedLinker)) {
    throw new Error(`Refusing the Windows native build because the asserted MSVC linker is missing: ${resolvedLinker}`);
  }
  return resolvedLinker;
}

export function parseCommandLineSetOutput(output) {
  const parsedEnvironment = {};
  for (const line of String(output).split(/\r?\n/)) {
    const match = /^([A-Za-z_][A-Za-z0-9_]*)=(.*)$/.exec(line);
    if (!match) continue;
    parsedEnvironment[match[1]] = match[2];
  }
  return Object.keys(parsedEnvironment).length > 0 ? parsedEnvironment : null;
}

export function captureRequiredVcvarsEnvironment(
  initialEnvironment = process.env,
  spawnCommandLine = spawnSync,
  fileIsRegular = isRegularFile,
) {
  // Preserve an explicitly initialized supported edition. Otherwise prefer
  // Community when both exact installations exist. Never retry a failed batch
  // against another edition or discover arbitrary Visual Studio installations.
  const explicitlyBuildTools = requiredMsvcToolchain(initialEnvironment).edition === "Build Tools";
  const vcvarsBatch = explicitlyBuildTools || (!fileIsRegular(REQUIRED_MSVS_LINKER)
      && fileIsRegular(REQUIRED_BUILD_TOOLS_MSVS_LINKER))
    ? REQUIRED_BUILD_TOOLS_VCVARS_BATCH : REQUIRED_VCVARS_BATCH;
  const result = spawnCommandLine(
    "cmd.exe",
    ["/d", "/s", "/c", `""${vcvarsBatch}" ${REQUIRED_VCVARS_ARGUMENTS} && set"`],
    {
      encoding: "utf8",
      env: initialEnvironment,
      windowsHide: true,
      windowsVerbatimArguments: true,
    },
  );
  if (result?.error || result?.status !== 0 || typeof result?.stdout !== "string") return null;
  return parseCommandLineSetOutput(result.stdout);
}

export function locateLinkersWithWhere(
  environment = process.env,
  spawnWhere = spawnSync,
) {
  const result = spawnWhere("where.exe", ["link.exe"], {
    encoding: "utf8",
    env: environment,
    windowsHide: true,
  });
  if (result?.error) throw result.error;
  if (result?.status !== 0 || typeof result?.stdout !== "string") {
    throw new Error(
      `Refusing the Windows native build because where.exe link.exe failed (exit ${result?.status ?? "unknown"}).`,
    );
  }
  return result.stdout
    .split(/\r?\n/)
    .map((candidate) => candidate.trim())
    .filter(Boolean);
}

export function requireExactMsvcLinkerFirst(
  environment,
  locateLinkers = locateLinkersWithWhere,
) {
  const locatedLinkers = locateLinkers(environment);
  const firstLinker = locatedLinkers[0];
  const requiredLinker = requiredMsvcToolchain(environment).linker;
  if (!firstLinker) {
    throw new Error(
      "Refusing the Windows native build because where.exe link.exe returned no linker.",
    );
  }
  if (normalizeWindowsPathKey(firstLinker) !== normalizeWindowsPathKey(requiredLinker)) {
    throw new Error(
      `Refusing the Windows native build because where.exe link.exe resolves ${firstLinker} first; required first linker: ${requiredLinker}.`,
    );
  }
  return locatedLinkers;
}

export function releaseExecutablePath(baseAppDir = appDir) {
  return path.resolve(baseAppDir, "..", "target", "release", "syndocal.exe");
}

export function verifiedNativeBuildEnvironment(
  environment = process.env,
  platform = process.platform,
  fileIsRegular = isRegularFile,
  locateLinkers = locateLinkersWithWhere,
) {
  if (platform !== "win32") return environment;

  const linker = requireExactMsvcToolset(environment, fileIsRegular);

  // Pin the target linker to the exact required Visual C++ binary for every
  // build so child Cargo can never fall through to a Git usr/bin/link.exe.
  const verifiedEnvironment = {
    ...environment,
    CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER: linker,
  };
  requireExactMsvcLinkerFirst(verifiedEnvironment, locateLinkers);
  return verifiedEnvironment;
}

export function tauriCommandEnvironment(
  args,
  environment = process.env,
  platform = process.platform,
  fileIsRegular = isRegularFile,
  initializeVcvarsEnvironment = captureRequiredVcvarsEnvironment,
  locateLinkers = locateLinkersWithWhere,
) {
  if (!isWindowsNativeCargoCommand(args)) return environment;
  if (platform !== "win32") return environment;

  try {
    return verifiedNativeBuildEnvironment(environment, platform, fileIsRegular, locateLinkers);
  } catch (ambientError) {
    const ambientReason = ambientError instanceof Error ? ambientError.message : String(ambientError);
    if (isGitHubHostedWindowsToolchain(environment)) {
      throw new Error(
        `${ambientReason} The GitHub-hosted windows-2022 MSVC preflight must initialize and verify the exact Enterprise toolchain before Tauri starts.`,
      );
    }
    const capturedEnvironment = initializeVcvarsEnvironment(environment);
    if (!capturedEnvironment) {
      throw new Error(
        `${ambientReason} Automatic vcvars64.bat -vcvars_ver=14.44 initialization failed; run pnpm from an x64 Visual Studio 2022 Community or Build Tools Developer Command Prompt.`,
      );
    }
    return verifiedNativeBuildEnvironment(capturedEnvironment, platform, fileIsRegular, locateLinkers);
  }
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

  if (process.platform === "win32"
      && environment.CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER) {
    const locatedLinkers = requireExactMsvcLinkerFirst(environment);
    console.error(
      `[syndocal-build] pinned CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER=${environment.CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER}`,
    );
    console.error(`[syndocal-build] where.exe link.exe:\n${locatedLinkers.join("\n")}`);
  }

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
