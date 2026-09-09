import { createHash } from "node:crypto";
import { execFileSync, spawn } from "node:child_process";
import { once } from "node:events";
import { closeSync, existsSync, openSync, readFileSync, readSync, statSync } from "node:fs";
import path from "node:path";
import readline from "node:readline";
import Ajv2020 from "ajv/dist/2020.js";
import { parse as parseToml } from "smol-toml";

export const INVENTORY_SCHEMA_VERSION = 2;
export const REQUIRED_CONFIGURATION_IDS = [
  "windows-default-all-targets",
  "windows-default-release",
  "windows-workspace-tests",
  "windows-syndocal-asio",
  "windows-syndocal-ndi",
  "windows-syndocal-spout",
  "windows-asio-bridge",
  "macos-default-all-targets",
  "macos-default-release",
  "linux-default-all-targets",
  "linux-default-release",
  "frontend-typescript-vite-windows",
  "windows-native-release",
];

const ENCODED_RUSTFLAGS_NAME = ["CARGO", "ENCODED", "RUSTFLAGS"].join("_");
const ENCODED_RUSTDOCFLAGS_NAME = ["CARGO", "ENCODED", "RUSTDOCFLAGS"].join("_");
const CARGO_COMPILER_SELECTION_KEYS = ["rustc", "rustc-wrapper", "rustc-workspace-wrapper"];
const CARGO_BUILD_CONTROL_KEYS = [...CARGO_COMPILER_SELECTION_KEYS, "rustdoc", "rustflags", "rustdocflags", "target"];
// Diff inspection is intentionally bounded so a pathological worktree cannot exhaust the checker.
// The current repository diff is well below this limit; overflow is a hard error rather than a partial audit.
export const GIT_OUTPUT_MAX_BUFFER = 16 * 1024 * 1024;
export const GIT_OUTPUT_LIMIT_ERROR_CODE = "WARNING_RATCHET_GIT_OUTPUT_LIMIT";
// Current-file inspection has its own byte ceiling. Untracked files share this as an
// aggregate budget; modified Cargo configs share a separate aggregate budget.
export const CURRENT_FILE_CONTENT_MAX_BYTES = 16 * 1024 * 1024;
const FILE_CONTENT_LIMIT_ERROR_CODE = "WARNING_RATCHET_FILE_CONTENT_LIMIT";
const FILE_READ_CHUNK_BYTES = 64 * 1024;
const CARGO_WARNING_FLAG_KEYS = ["rustflags", "rustdocflags"];
const CARGO_CONFIG_ENV_KEYS = new Set([
  "RUSTFLAGS",
  "RUSTDOCFLAGS",
  ENCODED_RUSTFLAGS_NAME,
  ENCODED_RUSTDOCFLAGS_NAME,
  "RUSTC",
  "RUSTC_WRAPPER",
  "RUSTC_WORKSPACE_WRAPPER",
  "RUSTC_BOOTSTRAP",
  "RUSTDOC",
  "CARGO_HOME",
]);

// Every Windows Cargo invocation must carry the exact MSVC link.exe pin from
// the native build completion gate, so the ratchet accepts precisely this one
// target-linker environment pair and preserves it into controlled child
// environments. The only second value is the exact Enterprise edition-root
// used by GitHub's official hosted windows-2022 image, and it is accepted only
// in that wrapper-marked hosted context. Every other target linker key/value
// stays fail-closed forbidden (Git usr/bin/link.exe, stale toolsets, bare names,
// empty values, other triples), and Cargo config linker overrides stay forbidden.
export const PINNED_WINDOWS_MSVC_TARGET_LINKER_KEY = "CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER";
export const PINNED_WINDOWS_MSVC_TARGET_LINKER_VALUE = String.raw`C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe`;
export const PINNED_BUILD_TOOLS_WINDOWS_MSVC_TARGET_LINKER_VALUE = String.raw`C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe`;
export const PINNED_GITHUB_HOSTED_WINDOWS_MSVC_TARGET_LINKER_VALUE =
  String.raw`C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe`;
export const GITHUB_HOSTED_WINDOWS_TOOLCHAIN_MARKER =
  "SYNDOCAL_GITHUB_HOSTED_WINDOWS_MSVC";
export const GITHUB_HOSTED_WINDOWS_TOOLCHAIN_MARKER_VALUE =
  "windows-2022-enterprise";
const REQUIRED_LOCAL_VCVARS_BATCH =
  String.raw`C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat`;
const REQUIRED_BUILD_TOOLS_VCVARS_BATCH =
  String.raw`C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Auxiliary\Build\vcvars64.bat`;
const REQUIRED_GITHUB_HOSTED_VCVARS_BATCH =
  String.raw`C:\Program Files\Microsoft Visual Studio\2022\Enterprise\VC\Auxiliary\Build\vcvars64.bat`;

function isGitHubHostedWindowsToolchain(environment) {
  return environment?.GITHUB_ACTIONS === "true"
    && environment?.RUNNER_OS === "Windows"
    && environment?.RUNNER_ENVIRONMENT === "github-hosted"
    && environment?.[GITHUB_HOSTED_WINDOWS_TOOLCHAIN_MARKER]
      === GITHUB_HOSTED_WINDOWS_TOOLCHAIN_MARKER_VALUE;
}

function isGitHubActionsWindows(environment) {
  return environment?.GITHUB_ACTIONS === "true"
    && environment?.RUNNER_OS === "Windows";
}

function isPinnedWindowsMsvcTargetLinkerEntry(key, value, environment) {
  if (key !== PINNED_WINDOWS_MSVC_TARGET_LINKER_KEY) return false;
  if (value === PINNED_WINDOWS_MSVC_TARGET_LINKER_VALUE
      || value === PINNED_BUILD_TOOLS_WINDOWS_MSVC_TARGET_LINKER_VALUE) {
    return !isGitHubActionsWindows(environment);
  }
  return value === PINNED_GITHUB_HOSTED_WINDOWS_MSVC_TARGET_LINKER_VALUE
    && isGitHubHostedWindowsToolchain(environment);
}

function isTargetLinkerEnvironmentKey(key) {
  return /^CARGO_TARGET_.+_LINKER$/i.test(key);
}

const normalizeWindowsPathKey = (candidate) =>
  path.win32.normalize(String(candidate)).trim().replace(/[\\/]+$/, "").toLowerCase();

const toolsetDirectoryForLinker = (linker) => {
  let directory = path.win32.dirname(linker);
  for (let index = 0; index < 3; index += 1) directory = path.win32.dirname(directory);
  return directory;
};

function defaultFileIsRegular(candidate) {
  try {
    return statSync(candidate).isFile();
  } catch {
    return false;
  }
}

function defaultLocateWindowsLinkers(environment) {
  const output = execFileSync("where.exe", ["link.exe"], {
    env: environment,
    encoding: "utf8",
    windowsHide: true,
    stdio: ["ignore", "pipe", "pipe"],
  });
  return String(output)
    .split(/\r?\n/)
    .map((candidate) => candidate.trim())
    .filter(Boolean);
}

function parseCommandLineSetOutput(output) {
  const parsedEnvironment = {};
  for (const line of String(output).split(/\r?\n/)) {
    const match = /^([A-Za-z_][A-Za-z0-9_]*)=(.*)$/.exec(line);
    if (match) parsedEnvironment[match[1]] = match[2];
  }
  return Object.keys(parsedEnvironment).length > 0 ? parsedEnvironment : null;
}

function defaultInitializeWindowsMsvcEnvironment(environment) {
  const vcvarsBatch = isGitHubHostedWindowsToolchain(environment)
    ? REQUIRED_GITHUB_HOSTED_VCVARS_BATCH
    : defaultFileIsRegular(PINNED_WINDOWS_MSVC_TARGET_LINKER_VALUE)
      ? REQUIRED_LOCAL_VCVARS_BATCH
      : REQUIRED_BUILD_TOOLS_VCVARS_BATCH;
  try {
    const output = execFileSync(
      "cmd.exe",
      ["/d", "/s", "/c", `""${vcvarsBatch}" -vcvars_ver=14.44 && set"`],
      {
        env: environment,
        encoding: "utf8",
        windowsHide: true,
        windowsVerbatimArguments: true,
        maxBuffer: 16 * 1024 * 1024,
        stdio: ["ignore", "pipe", "pipe"],
      },
    );
    const initializedEnvironment = parseCommandLineSetOutput(output);
    if (!initializedEnvironment?.VCToolsInstallDir) return initializedEnvironment;
    initializedEnvironment[PINNED_WINDOWS_MSVC_TARGET_LINKER_KEY] = path.win32.resolve(
      initializedEnvironment.VCToolsInstallDir.trim(),
      "bin",
      "Hostx64",
      "x64",
      "link.exe",
    );
    return initializedEnvironment;
  } catch {
    return null;
  }
}

/**
 * Fail closed before any warning-ratchet Cargo metadata/build/test process.
 * Tests may inject platform/file/where seams, but production callers use the
 * real process platform, filesystem, and where.exe resolution order.
 */
export function verifyExactWindowsMsvcCargoEnvironment(
  environment = process.env,
  {
    platform = process.platform,
    fileIsRegular = defaultFileIsRegular,
    locateLinkers = defaultLocateWindowsLinkers,
    initializeEnvironment = defaultInitializeWindowsMsvcEnvironment,
    log = (message) => console.error(message),
  } = {},
) {
  if (platform !== "win32") return environment;

  const initializedEnvironment = initializeEnvironment(environment);
  if (!initializedEnvironment) {
    throw new Error(
      "Windows warning-ratchet Cargo could not initialize vcvars64.bat -vcvars_ver=14.44",
    );
  }
  const hosted = isGitHubHostedWindowsToolchain(initializedEnvironment);
  const expectedLinker = hosted
    ? PINNED_GITHUB_HOSTED_WINDOWS_MSVC_TARGET_LINKER_VALUE
    : normalizeWindowsPathKey(initializedEnvironment?.VCToolsInstallDir)
      === normalizeWindowsPathKey(toolsetDirectoryForLinker(PINNED_BUILD_TOOLS_WINDOWS_MSVC_TARGET_LINKER_VALUE))
      ? PINNED_BUILD_TOOLS_WINDOWS_MSVC_TARGET_LINKER_VALUE
      : PINNED_WINDOWS_MSVC_TARGET_LINKER_VALUE;
  const expectedToolsetDirectory = toolsetDirectoryForLinker(expectedLinker);
  const actualToolsetDirectory = initializedEnvironment?.VCToolsInstallDir?.trim();
  if (!actualToolsetDirectory) {
    throw new Error("Windows warning-ratchet Cargo requires VCToolsInstallDir from the exact MSVC 14.44.35207 x64 environment");
  }
  if (normalizeWindowsPathKey(actualToolsetDirectory)
      !== normalizeWindowsPathKey(expectedToolsetDirectory)) {
    throw new Error(
      `Windows warning-ratchet Cargo VCToolsInstallDir does not match the required exact toolset: ${actualToolsetDirectory}; required ${expectedToolsetDirectory}`,
    );
  }
  if (initializedEnvironment?.[PINNED_WINDOWS_MSVC_TARGET_LINKER_KEY] !== expectedLinker) {
    throw new Error(
      `Windows warning-ratchet Cargo requires exact ${PINNED_WINDOWS_MSVC_TARGET_LINKER_KEY}=${expectedLinker}`,
    );
  }
  if (!fileIsRegular(expectedLinker)) {
    throw new Error(`Windows warning-ratchet Cargo required linker is missing: ${expectedLinker}`);
  }

  let locatedLinkers;
  try {
    locatedLinkers = locateLinkers(initializedEnvironment);
  } catch (error) {
    throw new Error(
      `Windows warning-ratchet Cargo where.exe link.exe failed: ${error instanceof Error ? error.message : String(error)}`,
    );
  }
  const firstLinker = locatedLinkers?.[0];
  if (!firstLinker) {
    throw new Error("Windows warning-ratchet Cargo where.exe link.exe returned no linker");
  }
  if (normalizeWindowsPathKey(firstLinker) !== normalizeWindowsPathKey(expectedLinker)) {
    const pathKeyEvidence = Object.entries(initializedEnvironment)
      .filter(([key]) => key.toLowerCase() === "path")
      .map(([key, value]) => {
        const expectedBin = normalizeWindowsPathKey(path.win32.dirname(expectedLinker));
        const expectedBinIndex = String(value)
          .split(";")
          .findIndex((candidate) => normalizeWindowsPathKey(candidate) === expectedBin);
        return `${key}:expected-bin-index=${expectedBinIndex}`;
      })
      .join(", ");
    throw new Error(
      `Windows warning-ratchet Cargo resolves ${firstLinker} first; required first linker: ${expectedLinker}; PATH evidence: ${pathKeyEvidence || "missing"}`,
    );
  }

  log("[warning-ratchet] vcvars64.bat -vcvars_ver=14.44 initialized");
  log(`[warning-ratchet] pinned ${PINNED_WINDOWS_MSVC_TARGET_LINKER_KEY}=${expectedLinker}`);
  log(`[warning-ratchet] where.exe link.exe:\n${locatedLinkers.join("\n")}`);
  return initializedEnvironment;
}

const FORBIDDEN_WARNING_ENV = [
  /^RUSTFLAGS$/i,
  /^RUSTDOCFLAGS$/i,
  new RegExp(`^${ENCODED_RUSTFLAGS_NAME}$`, "i"),
  new RegExp(`^${ENCODED_RUSTDOCFLAGS_NAME}$`, "i"),
  /^CARGO_HOME$/i,
  /^CARGO_BUILD_.+$/i,
  /^CARGO_TARGET_.+_(?:RUSTFLAGS|RUSTDOCFLAGS|RUSTC|RUSTC_WRAPPER|RUSTC_WORKSPACE_WRAPPER|RUSTDOC|LINKER)$/i,
  /^CARGO_PROFILE_.+$/i,
  /^RUSTDOC$/i,
  /^RUSTC$/i,
  /^RUSTC_(?:WRAPPER|WORKSPACE_WRAPPER|BOOTSTRAP)$/i,
];

const FORBIDDEN_GENERIC_COMMAND_ENV = [
  /^NODE_OPTIONS$/i,
];
// `pnpm run` injects these package-manager variables into the checker itself.
// They have not affected the already-running Node process, so remove them from
// the controlled child environment instead of making the official package
// script impossible to execute. NODE_OPTIONS remains fail-closed because it
// has already changed the checker process before this code runs.
const STRIPPED_GENERIC_COMMAND_ENV = [
  /^PNPM_.+/i,
  /^NPM_CONFIG_/i,
  /^NPM_EXEC_PATH$/i,
  /^NPM_NODE_EXEC_PATH$/i,
];
const SUPPORTED_COMMAND_EXECUTABLES = new Set(["cargo", "pnpm"]);
export const GENERIC_COMMAND_OUTPUT_LIMIT_BYTES = 8 * 1024 * 1024;

function resolvePnpmInvocation(args) {
  if (process.platform !== "win32") return { executable: "pnpm", args, shell: false };
  const nodeDirectory = path.dirname(process.execPath);
  const pnpmHome = process.env.PNPM_HOME;
  const directPnpmScriptCandidates = [
    ...(pnpmHome
      ? [path.resolve(pnpmHome, "..", "pnpm", "bin", "pnpm.cjs")]
      : []),
    path.join(nodeDirectory, "node_modules", "corepack", "dist", "pnpm.js"),
    path.join(nodeDirectory, "node_modules", "pnpm", "bin", "pnpm.cjs"),
  ];
  const directPnpmScript = directPnpmScriptCandidates.find((candidate) => existsSync(candidate));
  if (directPnpmScript) {
    return { executable: process.execPath, args: [directPnpmScript, ...args], shell: false };
  }
  return { executable: "pnpm.cmd", args, shell: true };
}

const SUPPRESSION_PATTERNS = [
  // A separated rustc lint level is a same-command-line token pair. Restrict
  // the separator to horizontal whitespace so an unrelated command ending in
  // `-A` cannot consume the first word of the next line (for example the
  // common `git add -A` followed by `git commit`). Glued rustc forms retain
  // their existing coverage in the second alternative.
  { id: "rust-command-line-allow", pattern: /(?:^|[\s'",\[])(?:-A[ \t]+(?:warnings|[a-zA-Z_][\w-]*)|-A(?:warnings|unused|dead_code|[a-zA-Z][\w-]*_[\w-]+))(?:$|[\s'",\]])/m },
  { id: "rust-cap-lints-allow", pattern: /--cap-lints(?:=|\s+)allow\b/i },
  { id: "cargo-rustflags-config", pattern: /\brustdocflags\s*=|\brustflags\s*=/i },
  {
    id: "cargo-rustc-selection-config",
    pattern: /(?:\[\s*build\s*\][\s\S]{0,500}?(?:^|\r?\n)\s*(?:rustc|rustc-wrapper|rustc-workspace-wrapper)\s*=|--config(?:=|\s+)["']?build\.(?:rustc|rustc-wrapper|rustc-workspace-wrapper)\s*=)/im,
  },
  { id: "cargo-encoded-rustflags", pattern: new RegExp(["CARGO", "ENCODED", "RUSTFLAGS"].join("_"), "i") },
  { id: "vite-chunk-limit-increase", pattern: /chunkSizeWarningLimit\s*:/ },
];

function slash(value) {
  return value.replaceAll("\\", "/");
}

function stripWindowsDevicePrefix(value) {
  if (/^\\\\\?\\UNC\\/i.test(value)) return `\\\\${value.slice(8)}`;
  if (/^\\\\\?\\/.test(value)) return value.slice(4);
  return value;
}

function looksWindowsPath(value) {
  return /^[a-zA-Z]:[\\/]/.test(value) || /^\\\\/.test(value) || /^\\\\\?\\/.test(value);
}

export function normalizeRepoPath(fileName, repoRoot) {
  if (!fileName) return "<no-source>";
  const useWindows = looksWindowsPath(fileName) || looksWindowsPath(repoRoot) || process.platform === "win32";
  const pathApi = useWindows ? path.win32 : path;
  const cleanRoot = useWindows ? stripWindowsDevicePrefix(repoRoot) : repoRoot;
  const cleanFile = useWindows ? stripWindowsDevicePrefix(fileName) : fileName;
  const root = pathApi.resolve(cleanRoot);
  const absolute = pathApi.isAbsolute(cleanFile) ? pathApi.resolve(cleanFile) : pathApi.resolve(root, cleanFile);
  const relative = pathApi.relative(root, absolute);
  const inside = relative === "" || (!relative.startsWith("..") && !pathApi.isAbsolute(relative));
  const canonical = inside ? (relative || ".") : `<external>/${pathApi.basename(absolute)}`;
  const normalized = slash(canonical);
  return useWindows ? normalized.toLocaleLowerCase("en-US") : normalized;
}

export function normalizeComparisonPath(value) {
  const normalized = slash(value);
  return process.platform === "win32" || looksWindowsPath(value)
    ? normalized.toLocaleLowerCase("en-US")
    : normalized;
}

function canonicalStrings(values) {
  return [...new Set(values ?? [])].sort();
}

function primarySpan(diagnostic) {
  return diagnostic.spans?.find((span) => span.is_primary) ?? diagnostic.spans?.[0] ?? null;
}

function packageNameFromId(packageId) {
  const hashPart = String(packageId ?? "unknown").split("#").at(-1);
  return hashPart?.split("@")[0] || "unknown";
}

export function diagnosticIdentity(record) {
  return JSON.stringify({
    platform: record.platform,
    profile: record.profile,
    features: canonicalStrings(record.features),
    classification: record.classification,
    origin: record.origin,
    package: record.package,
    target: record.target,
    targetKinds: canonicalStrings(record.targetKinds),
    code: record.code ?? null,
    file: normalizeComparisonPath(record.file),
    message: record.message.trim(),
  });
}

export function diagnosticIdentityHash(record) {
  return createHash("sha256").update(diagnosticIdentity(record)).digest("hex");
}

function classificationInvariantKey(record) {
  return JSON.stringify({
    platform: record.platform,
    profile: record.profile,
    features: canonicalStrings(record.features),
    package: record.package,
    target: record.target,
    targetKinds: canonicalStrings(record.targetKinds),
    code: record.code ?? null,
    file: normalizeComparisonPath(record.file),
    message: record.message.trim(),
  });
}

export function artifactIdentity(artifact) {
  return JSON.stringify({
    package: artifact.package,
    target: artifact.target,
    targetKinds: canonicalStrings(artifact.targetKinds),
    crateTypes: canonicalStrings(artifact.crateTypes),
  });
}

export function parseCargoMessage(message, configuration, context) {
  if (message?.reason === "compiler-artifact") {
    if (!context.firstPartyPackageIds.has(message.package_id)) return { artifact: null };
    return {
      artifact: {
        package: context.packageNames.get(message.package_id) ?? packageNameFromId(message.package_id),
        target: message.target?.name ?? "unknown",
        targetKinds: canonicalStrings(message.target?.kind),
        crateTypes: canonicalStrings(message.target?.crate_types),
      },
    };
  }
  if (message?.reason === "build-finished") return { buildFinished: message.success === true };
  if (message?.reason !== "compiler-message" || message.message?.level !== "warning") return {};

  const span = primarySpan(message.message);
  const packageName = context.packageNames.get(message.package_id) ?? packageNameFromId(message.package_id);
  const isFirstParty = context.firstPartyPackageIds.has(message.package_id);
  const file = normalizeRepoPath(span?.file_name, context.repoRoot);
  const targetKinds = canonicalStrings(message.target?.kind);
  const origin = targetKinds.includes("custom-build")
    ? "custom-build"
    : !span
      ? "no-source"
      : file.startsWith("<external>/")
        ? "external-source"
        : "source";
  const record = {
    platform: configuration.platform,
    profile: configuration.profile,
    features: canonicalStrings(configuration.features),
    classification: isFirstParty ? "first-party" : "third-party",
    origin,
    package: packageName,
    target: message.target?.name ?? "unknown",
    targetKinds,
    code: message.message.code?.code ?? null,
    file,
    line: span?.line_start ?? null,
    column: span?.column_start ?? null,
    message: String(message.message.message ?? "").trim(),
    occurrences: 1,
  };
  record.identity = diagnosticIdentityHash(record);
  return { diagnostic: record };
}

export function aggregateDiagnostics(records) {
  const aggregate = new Map();
  for (const record of records) {
    const identity = record.identity ?? diagnosticIdentityHash(record);
    const existing = aggregate.get(identity);
    if (existing) {
      existing.occurrences += record.occurrences ?? 1;
      if ((record.line ?? Number.MAX_SAFE_INTEGER) < (existing.line ?? Number.MAX_SAFE_INTEGER)) {
        existing.line = record.line;
        existing.column = record.column;
      }
    } else {
      aggregate.set(identity, { ...record, identity, occurrences: record.occurrences ?? 1 });
    }
  }
  return [...aggregate.values()].sort((a, b) => a.identity.localeCompare(b.identity));
}

function aggregateArtifacts(records) {
  const map = new Map(records.filter(Boolean).map((record) => [artifactIdentity(record), record]));
  return [...map.values()].sort((a, b) => artifactIdentity(a).localeCompare(artifactIdentity(b)));
}

export function parseCargoJsonLines(lines, configuration, context) {
  const diagnostics = [];
  const artifacts = [];
  const invalidJsonLines = [];
  const buildFinished = [];
  for (const rawLine of lines) {
    const line = rawLine.trim();
    if (!line) continue;
    let message;
    try {
      message = JSON.parse(line);
    } catch {
      invalidJsonLines.push(line);
      continue;
    }
    const parsed = parseCargoMessage(message, configuration, context);
    if (parsed.diagnostic) diagnostics.push(parsed.diagnostic);
    if (parsed.artifact) artifacts.push(parsed.artifact);
    if (Object.hasOwn(parsed, "buildFinished")) buildFinished.push(parsed.buildFinished);
  }
  return {
    diagnostics: aggregateDiagnostics(diagnostics),
    artifacts: aggregateArtifacts(artifacts),
    invalidJsonLines,
    buildFinished,
  };
}

export function forbiddenWarningEnvironment(environment = process.env) {
  return Object.entries(environment)
    .filter(([key, value]) => (value !== undefined && value !== "" || isTargetLinkerEnvironmentKey(key))
      && !isPinnedWindowsMsvcTargetLinkerEntry(key, value, environment)
      && FORBIDDEN_WARNING_ENV.some((pattern) => pattern.test(key)))
    .map(([key]) => key)
    .sort();
}

export function controlledChildEnvironment(environment = process.env) {
  const rejected = forbiddenWarningEnvironment(environment);
  if (rejected.length > 0) throw new Error(`warning-affecting environment is forbidden: ${rejected.join(", ")}`);
  return Object.fromEntries(Object.entries(environment).filter(([key, value]) =>
    isPinnedWindowsMsvcTargetLinkerEntry(key, value, environment)
    || !FORBIDDEN_WARNING_ENV.some((pattern) => pattern.test(key))));
}

export function forbiddenGenericCommandEnvironment(environment = process.env) {
  return Object.entries(environment)
    .filter(([key, value]) => value !== undefined && value !== ""
      && !STRIPPED_GENERIC_COMMAND_ENV.some((pattern) => pattern.test(key))
      && FORBIDDEN_GENERIC_COMMAND_ENV.some((pattern) => pattern.test(key)))
    .map(([key]) => key)
    .sort();
}

export function controlledGenericCommandEnvironment(environment = process.env) {
  const rejected = forbiddenGenericCommandEnvironment(environment);
  if (rejected.length > 0) {
    throw new Error(`generic command environment is forbidden: ${rejected.join(", ")}`);
  }
  const controlled = controlledChildEnvironment(environment);
  return Object.fromEntries(Object.entries(controlled)
    .filter(([key]) => !STRIPPED_GENERIC_COMMAND_ENV.some((pattern) => pattern.test(key))));
}

function cargoConfigCandidates(repoRoot, environment) {
  const result = new Set();
  let current = path.resolve(repoRoot);
  while (true) {
    result.add(path.join(current, ".cargo", "config.toml"));
    result.add(path.join(current, ".cargo", "config"));
    const parent = path.dirname(current);
    if (parent === current) break;
    current = parent;
  }
  const home = environment.USERPROFILE || environment.HOME;
  if (home) {
    result.add(path.join(home, ".cargo", "config.toml"));
    result.add(path.join(home, ".cargo", "config"));
  }
  return [...result];
}

export function inspectCargoConfigText(text) {
  let config;
  try {
    config = parseToml(text);
  } catch {
    return ["cargo-config-malformed"];
  }
  const findings = [];
  const build = config?.build;
  if (build && typeof build === "object" && !Array.isArray(build)) {
    for (const key of CARGO_BUILD_CONTROL_KEYS) {
      if (Object.hasOwn(build, key)) findings.push(`cargo-build-${key}`);
    }
  }
  if (Object.hasOwn(config, "profile")) findings.push("cargo-profile-config");
  const targets = config?.target;
  if (targets && typeof targets === "object" && !Array.isArray(targets)) {
    for (const target of Object.values(targets)) {
      if (!target || typeof target !== "object" || Array.isArray(target)) continue;
      if (Object.hasOwn(target, "linker")) findings.push("cargo-target-linker");
      for (const key of CARGO_WARNING_FLAG_KEYS) {
        if (Object.hasOwn(target, key)) findings.push(`cargo-target-${key}`);
      }
    }
  }
  const cargoEnvironment = config?.env;
  if (cargoEnvironment && typeof cargoEnvironment === "object" && !Array.isArray(cargoEnvironment)) {
    for (const key of Object.keys(cargoEnvironment)) {
      const canonicalKey = key.toLocaleUpperCase("en-US");
      const isDangerous = CARGO_CONFIG_ENV_KEYS.has(canonicalKey)
        || /^CARGO_BUILD_.+$/.test(canonicalKey)
        || /^CARGO_PROFILE_.+$/.test(canonicalKey)
        || /^CARGO_TARGET_.+_(?:RUSTFLAGS|RUSTDOCFLAGS|RUSTC|RUSTC_WRAPPER|RUSTC_WORKSPACE_WRAPPER|RUSTDOC|LINKER)$/.test(canonicalKey);
      if (isDangerous) findings.push(`cargo-env-${canonicalKey.toLocaleLowerCase("en-US")}`);
    }
  }
  return [...new Set(findings)];
}

function cargoLintTreeContainsAllow(value) {
  if (typeof value === "string") return value.toLocaleLowerCase("en-US") === "allow";
  if (!value || typeof value !== "object" || Array.isArray(value)) return false;
  if (typeof value.level === "string" && value.level.toLocaleLowerCase("en-US") === "allow") return true;
  return Object.entries(value).some(([key, child]) => key !== "workspace" && cargoLintTreeContainsAllow(child));
}

export function inspectCargoManifestLintText(text) {
  let manifest;
  try {
    manifest = parseToml(text);
  } catch {
    return [];
  }
  const findings = [];
  if (manifest?.lints?.workspace === true) findings.push("cargo-lints-workspace-inheritance");
  if (cargoLintTreeContainsAllow(manifest?.lints) || cargoLintTreeContainsAllow(manifest?.workspace?.lints)) {
    findings.push("cargo-lint-level-allow");
  }
  return findings;
}

export function warningAffectingCargoConfigs(repoRoot, environment = process.env) {
  const findings = [];
  for (const candidate of cargoConfigCandidates(repoRoot, environment)) {
    try {
      const { text } = readBoundedFileText(
        candidate,
        candidate,
        CURRENT_FILE_CONTENT_MAX_BYTES,
        "Cargo config content",
      );
      const semanticFindings = inspectCargoConfigText(text);
      if (semanticFindings.length > 0 || /\brust(?:doc)?flags\s*=|\bcap-lints\b|(?:^|\s)-A(?:\s*|=)[\w-]+/im.test(text)) findings.push(candidate);
    } catch (error) {
      if (isMissingFileError(error)) continue;
      throw error;
    }
  }
  return findings;
}

function assertControlledCargoInputs(configuration, repoRoot, environment) {
  controlledChildEnvironment(environment);
  const missingBuildEnvironment = (configuration.requiredEnvironment ?? []).filter((key) => !environment[key]);
  if (missingBuildEnvironment.length > 0) {
    throw new Error(`required build environment is missing: ${missingBuildEnvironment.join(", ")}`);
  }
  if (configuration.command.args.some((arg) => arg === "--config" || arg.startsWith("--config="))) {
    throw new Error("Cargo --config is forbidden for warning inventory commands");
  }
  const configs = warningAffectingCargoConfigs(repoRoot, environment);
  if (configs.length > 0) throw new Error(`warning-affecting Cargo config is forbidden: ${configs.join(", ")}`);
}

function metadataFor(repoRoot, args, environment, timeoutMs) {
  return JSON.parse(execFileSync("cargo", args, {
    cwd: repoRoot,
    env: controlledChildEnvironment(environment),
    encoding: "utf8",
    timeout: timeoutMs,
    maxBuffer: 64 * 1024 * 1024,
    stdio: ["ignore", "pipe", "pipe"],
  }));
}

function readCargoMetadataFromVerifiedEnvironment(repoRoot, configuration, environment) {
  assertControlledCargoInputs(configuration, repoRoot, environment);
  const metadataSets = [metadataFor(repoRoot, ["metadata", "--format-version", "1", "--no-deps", "--locked"], environment, configuration.timeoutMs)];
  for (const manifest of configuration.firstPartyManifests ?? []) {
    metadataSets.push(metadataFor(repoRoot, ["metadata", "--format-version", "1", "--no-deps", "--locked", "--manifest-path", manifest], environment, configuration.timeoutMs));
  }
  const packageNames = new Map();
  const firstPartyPackageIds = new Set();
  for (const metadata of metadataSets) {
    for (const pkg of metadata.packages) packageNames.set(pkg.id, pkg.name);
    for (const id of metadata.workspace_members) firstPartyPackageIds.add(id);
  }
  return { packageNames, firstPartyPackageIds };
}

export function readCargoMetadata(
  repoRoot,
  configuration,
  environment = process.env,
  nativeToolchainOptions = {},
) {
  assertControlledCargoInputs(configuration, repoRoot, environment);
  const verifiedEnvironment = verifyExactWindowsMsvcCargoEnvironment(
    environment,
    nativeToolchainOptions,
  );
  return readCargoMetadataFromVerifiedEnvironment(
    repoRoot,
    configuration,
    verifiedEnvironment,
  );
}

export async function runProcessWithTimeout(executable, args, options) {
  const maxOutputBytes = options.maxOutputBytes === undefined ? null : options.maxOutputBytes;
  if (maxOutputBytes !== null && (!Number.isSafeInteger(maxOutputBytes) || maxOutputBytes < 1)) {
    throw new Error("process output limit is invalid");
  }
  const pnpmInvocation = executable === "pnpm" ? resolvePnpmInvocation(args) : null;
  const resolvedExecutable = pnpmInvocation?.executable ?? executable;
  const resolvedArgs = pnpmInvocation?.args ?? args;
  const child = spawn(resolvedExecutable, resolvedArgs, {
    cwd: options.cwd,
    env: options.env,
    // The fallback .cmd adapter is used only when no direct pnpm script is
    // available. Direct Node invocation keeps generic-command output free of
    // cmd.exe command-echo noise on Windows.
    shell: pnpmInvocation?.shell ?? false,
    stdio: ["ignore", "pipe", "pipe"],
    windowsHide: true,
  });
  let stderr = "";
  let stdout = "";
  let outputBytes = 0;
  let outputLimitExceeded = false;
  const captureChunk = (stream, chunk) => {
    const text = Buffer.isBuffer(chunk) ? chunk.toString("utf8") : String(chunk);
    if (outputLimitExceeded) return;
    const bytes = Buffer.byteLength(text, "utf8");
    if (maxOutputBytes !== null && outputBytes + bytes > maxOutputBytes) {
      outputLimitExceeded = true;
      child.kill("SIGKILL");
      return;
    }
    if (maxOutputBytes !== null) outputBytes += bytes;
    if (stream === "stdout") stdout += text;
    else stderr += text;
  };
  child.stderr.on("data", (chunk) => {
    captureChunk("stderr", chunk);
    if (options.forwardStderr) process.stderr.write(chunk);
  });
  if (maxOutputBytes !== null) child.stdout.on("data", (chunk) => captureChunk("stdout", chunk));
  let stdoutClosed;
  if (options.onStdoutLine) {
    const stdoutReader = readline.createInterface({ input: child.stdout, crlfDelay: Infinity });
    stdoutClosed = once(stdoutReader, "close");
    stdoutReader.on("line", (line) => options.onStdoutLine(line));
  } else {
    stdoutClosed = once(child.stdout, "end");
  }
  let timedOut = false;
  const timeout = setTimeout(() => {
    timedOut = true;
    child.kill("SIGKILL");
  }, options.timeoutMs);
  const exitCode = await new Promise((resolve, reject) => {
    child.once("error", reject);
    child.once("close", resolve);
  });
  clearTimeout(timeout);
  await stdoutClosed;
  return { exitCode, timedOut, stdout, stderr, outputBytes, outputLimitExceeded };
}

export function warningShapedStderr(stderr) {
  return /(?:^|\r?\n)\s*warning(?:\[[^\]]+\])?\s*:/i.test(stderr);
}

export function warningShapedOutput(stdout = "", stderr = "") {
  const output = `${stdout}\n${stderr}`;
  return /(?:^|\r?\n)\s*warning(?:\[[^\]]+\])?\s*:/i.test(output)
    || /\bWARN\b/i.test(output)
    || /\(!\)/.test(output);
}

export function validateExpectedOutputMarkers(markers) {
  if (!Array.isArray(markers) || markers.length === 0) {
    throw new Error("expectedOutputMarkers must be a non-empty array");
  }
  if (markers.some((marker) => typeof marker !== "string" || marker.trim().length === 0)) {
    throw new Error("expectedOutputMarkers must contain non-empty literal strings");
  }
  if (new Set(markers).size !== markers.length) {
    throw new Error("expectedOutputMarkers must contain unique literal strings");
  }
  return [...markers];
}

export function compareOutputMarkerCoverage(expected, output) {
  const markers = validateExpectedOutputMarkers(expected);
  const missing = markers.filter((marker) => !output.includes(marker));
  return { ok: missing.length === 0, missing, expected: markers };
}

export async function runCargoConfiguration(
  configuration,
  repoRoot,
  environment = process.env,
  nativeToolchainOptions = {},
) {
  if (configuration.command?.executable !== "cargo") throw new Error(`configuration ${configuration.id} must use cargo`);
  if (!Number.isInteger(configuration.timeoutMs) || configuration.timeoutMs < 1) throw new Error(`configuration ${configuration.id} timeoutMs is invalid`);
  const args = [...configuration.command.args];
  if (!args.some((arg) => arg === "--message-format=json" || arg.startsWith("--message-format=json"))) {
    throw new Error(`configuration ${configuration.id} must request Cargo JSON diagnostics`);
  }
  assertControlledCargoInputs(configuration, repoRoot, environment);
  const verifiedEnvironment = verifyExactWindowsMsvcCargoEnvironment(
    environment,
    nativeToolchainOptions,
  );
  const metadata = readCargoMetadataFromVerifiedEnvironment(
    repoRoot,
    configuration,
    verifiedEnvironment,
  );
  const lines = [];
  const processResult = await runProcessWithTimeout("cargo", args, {
    cwd: repoRoot,
    env: controlledChildEnvironment(verifiedEnvironment),
    timeoutMs: configuration.timeoutMs,
    forwardStderr: true,
    onStdoutLine: (line) => lines.push(line),
  });
  const parsed = parseCargoJsonLines(lines, configuration, { ...metadata, repoRoot });
  return { ...processResult, ...parsed, stderrWarning: warningShapedStderr(processResult.stderr) };
}

function assertGenericCommandConfiguration(configuration) {
  const executable = configuration?.command?.executable;
  if (executable === "cargo") throw new Error(`configuration ${configuration.id} must use the Cargo dispatch`);
  if (!SUPPORTED_COMMAND_EXECUTABLES.has(executable)) {
    throw new Error(`configuration ${configuration?.id ?? "<unknown>"} uses unsupported command executable: ${executable ?? "<missing>"}`);
  }
  if (!Number.isInteger(configuration.timeoutMs) || configuration.timeoutMs < 1) {
    throw new Error(`configuration ${configuration.id} timeoutMs is invalid`);
  }
  if (!Array.isArray(configuration.command.args) || configuration.command.args.length === 0
    || configuration.command.args.some((arg) => typeof arg !== "string")) {
    throw new Error(`configuration ${configuration.id} command args are invalid`);
  }
  validateExpectedOutputMarkers(configuration.expectedOutputMarkers);
}

function assertRequiredGenericEnvironment(configuration, environment) {
  const required = configuration?.requiredEnvironment;
  // Existing frontend entries intentionally have no external SDK dependency.
  // When the field is present, however, it is strict and cannot be empty.
  if (required === undefined) return;
  if (!Array.isArray(required) || required.length === 0) {
    throw new Error(`configuration ${configuration?.id ?? "<unknown>"} requiredEnvironment must be a non-empty array`);
  }
  if (required.some((key) => typeof key !== "string" || key.trim().length === 0)) {
    throw new Error(`configuration ${configuration.id} requiredEnvironment must contain non-empty names`);
  }
  if (new Set(required).size !== required.length) {
    throw new Error(`configuration ${configuration.id} requiredEnvironment must contain unique names`);
  }
  const missing = required.filter((key) => typeof environment[key] !== "string" || environment[key].trim().length === 0);
  if (missing.length > 0) {
    throw new Error(`required generic environment is missing: ${missing.join(", ")}`);
  }
}

export async function runGenericConfiguration(configuration, repoRoot, environment = process.env) {
  assertGenericCommandConfiguration(configuration);
  const controlledEnvironment = controlledGenericCommandEnvironment(environment);
  assertRequiredGenericEnvironment(configuration, controlledEnvironment);
  const args = [...configuration.command.args];
  const processResult = await runProcessWithTimeout(configuration.command.executable, args, {
    cwd: repoRoot,
    env: controlledEnvironment,
    timeoutMs: configuration.timeoutMs,
    maxOutputBytes: GENERIC_COMMAND_OUTPUT_LIMIT_BYTES,
    forwardStderr: false,
  });
  const output = `${processResult.stdout}\n${processResult.stderr}`;
  return {
    ...processResult,
    output,
    warningShaped: warningShapedOutput(processResult.stdout, processResult.stderr),
    markerCoverage: compareOutputMarkerCoverage(configuration.expectedOutputMarkers, output),
  };
}

export async function runWarningConfiguration(configuration, repoRoot, environment = process.env) {
  if (configuration?.command?.executable === "cargo") {
    return runCargoConfiguration(configuration, repoRoot, environment);
  }
  return runGenericConfiguration(configuration, repoRoot, environment);
}

export function compareArtifactCoverage(expected, current) {
  const expectedKeys = new Set(expected.map(artifactIdentity));
  const currentKeys = new Set(current.map(artifactIdentity));
  const missing = [...expectedKeys].filter((key) => !currentKeys.has(key));
  const unexpected = [...currentKeys].filter((key) => !expectedKeys.has(key));
  return { ok: expectedKeys.size > 0 && missing.length === 0 && unexpected.length === 0, missing, unexpected };
}

export function validateInventorySchema(inventory, schema) {
  const ajv = new Ajv2020({ allErrors: true, strict: true, strictRequired: false, validateFormats: false });
  const validate = ajv.compile(schema);
  return validate(inventory) ? [] : validate.errors.map((error) => `${error.instancePath || "/"} ${error.message}`);
}

export function validateInventory(inventory, schema = null) {
  const errors = schema ? validateInventorySchema(inventory, schema) : [];
  if (inventory?.schemaVersion !== INVENTORY_SCHEMA_VERSION) errors.push(`schemaVersion must be ${INVENTORY_SCHEMA_VERSION}`);
  if (!Array.isArray(inventory?.configurations)) return [...errors, "configurations must be an array"];
  const ids = new Set();
  for (const configuration of inventory.configurations) {
    if (ids.has(configuration.id)) errors.push(`duplicate configuration id: ${configuration.id}`);
    ids.add(configuration.id);
    const executable = configuration.command?.executable;
    if (!SUPPORTED_COMMAND_EXECUTABLES.has(executable)) {
      errors.push(`configuration ${configuration.id} uses unsupported command executable: ${executable ?? "<missing>"}`);
    }
    if (Object.hasOwn(configuration, "expectedOutputMarkers")) {
      try {
        validateExpectedOutputMarkers(configuration.expectedOutputMarkers);
      } catch (error) {
        errors.push(`configuration ${configuration.id} has invalid expectedOutputMarkers: ${error.message}`);
      }
    }
    if (configuration.status === "enforced") {
      if (executable === "cargo"
        && (!Array.isArray(configuration.expectedArtifacts) || configuration.expectedArtifacts.length === 0)) {
        errors.push(`enforced Cargo configuration ${configuration.id} has zero expected artifact coverage`);
      }
      if (executable !== "cargo") {
        if (!Object.hasOwn(configuration, "expectedOutputMarkers")) {
          errors.push(`enforced generic configuration ${configuration.id} has no expected output markers`);
        }
        if ((configuration.expectedArtifacts?.length ?? 0) > 0
          || (configuration.diagnostics?.length ?? 0) > 0
          || (configuration.externalWarningAllows?.length ?? 0) > 0) {
          errors.push(`enforced generic configuration ${configuration.id} must not carry Cargo artifacts or diagnostics`);
        }
      }
      if (!/^[0-9a-f]{40}$/.test(configuration.evidence?.commit ?? "")) errors.push(`configuration ${configuration.id} evidence commit is invalid`);
      const diagnosticIds = new Set();
      for (const diagnostic of configuration.diagnostics ?? []) {
        if (diagnosticIds.has(diagnostic.identity)) errors.push(`duplicate diagnostic identity in ${configuration.id}: ${diagnostic.identity}`);
        diagnosticIds.add(diagnostic.identity);
        if (diagnostic.identity !== diagnosticIdentityHash(diagnostic)) errors.push(`stale diagnostic identity in ${configuration.id}: ${diagnostic.identity}`);
        if (diagnostic.platform !== configuration.platform || diagnostic.profile !== configuration.profile
          || JSON.stringify(canonicalStrings(diagnostic.features)) !== JSON.stringify(canonicalStrings(configuration.features))) {
          errors.push(`diagnostic scope mismatch in ${configuration.id}: ${diagnostic.identity}`);
        }
      }
      const artifactIds = configuration.expectedArtifacts.map(artifactIdentity);
      if (new Set(artifactIds).size !== artifactIds.length) errors.push(`duplicate expected artifact in ${configuration.id}`);
      const allowIds = (configuration.externalWarningAllows ?? []).map((allow) => allow.identity);
      if (new Set(allowIds).size !== allowIds.length) errors.push(`duplicate external warning allow in ${configuration.id}`);
      const diagnosticById = new Map((configuration.diagnostics ?? []).map((diagnostic) => [diagnostic.identity, diagnostic]));
      for (const allow of configuration.externalWarningAllows ?? []) {
        if (!Number.isFinite(Date.parse(allow.expiry))) errors.push(`external warning allow has invalid expiry in ${configuration.id}: ${allow.identity}`);
        if (diagnosticById.get(allow.identity)?.classification !== "third-party") {
          errors.push(`external warning allow does not match a third-party diagnostic in ${configuration.id}: ${allow.identity}`);
        }
      }
      for (const diagnostic of configuration.diagnostics ?? []) {
        if (diagnostic.classification === "third-party" && !allowIds.includes(diagnostic.identity)) {
          errors.push(`third-party diagnostic lacks an owned allow in ${configuration.id}: ${diagnostic.identity}`);
        }
      }
    } else if ((configuration.diagnostics?.length ?? 0) > 0 || (configuration.expectedArtifacts?.length ?? 0) > 0) {
      errors.push(`pending configuration ${configuration.id} carries unmeasured baseline or coverage`);
    }
  }
  for (const id of REQUIRED_CONFIGURATION_IDS) if (!ids.has(id)) errors.push(`required warning configuration is missing: ${id}`);
  if (inventory.policy?.requiredMatrixComplete === true && inventory.configurations.some((c) => c.status !== "enforced")) {
    errors.push("requiredMatrixComplete is true while configurations remain pending");
  }
  return errors;
}

function uniqueMap(records, identity) {
  const map = new Map();
  for (const record of records) {
    const key = identity(record);
    if (map.has(key)) throw new Error(`duplicate identity encountered: ${key}`);
    map.set(key, record);
  }
  return map;
}

export function compareDiagnostics(configuration, current, modifiedFiles = new Set(), now = new Date()) {
  const baselineMap = uniqueMap(configuration.diagnostics, (record) => record.identity);
  const currentMap = uniqueMap(current, (record) => record.identity);
  const baselineByInvariant = new Map(configuration.diagnostics.map((record) => [classificationInvariantKey(record), record]));
  const allowMap = uniqueMap(configuration.externalWarningAllows ?? [], (allow) => allow.identity);
  const failures = [];
  const removed = [];
  for (const record of currentMap.values()) {
    const previous = baselineMap.get(record.identity);
    const invariantPrevious = baselineByInvariant.get(classificationInvariantKey(record));
    if (invariantPrevious?.classification === "first-party" && record.classification !== "first-party") {
      failures.push(`classification downgrade for ${record.file}: first-party -> ${record.classification}`);
    }
    if (record.classification === "first-party") {
      if (!previous) failures.push(`new first-party warning ${record.code ?? "<no-code>"} in ${record.file}: ${record.message}`);
      else if (record.occurrences > previous.occurrences) failures.push(`first-party warning occurrence growth ${record.identity}: ${previous.occurrences} -> ${record.occurrences}`);
      if (modifiedFiles.has(normalizeComparisonPath(record.file))) failures.push(`first-party warning remains in modified file ${record.file}: ${record.message}`);
    } else {
      const allow = allowMap.get(record.identity);
      if (!allow) failures.push(`unowned ${record.origin} warning ${record.identity}`);
      else {
        if (record.occurrences > allow.maxOccurrences) failures.push(`external warning occurrence growth ${record.identity}`);
        if (Date.parse(allow.expiry) <= now.getTime()) failures.push(`external warning allow expired ${record.identity}`);
      }
    }
  }
  for (const record of baselineMap.values()) if (!currentMap.has(record.identity)) removed.push(record);
  const counts = (records) => records.reduce((result, record) => {
    result.total += record.occurrences;
    result[record.classification] = (result[record.classification] ?? 0) + record.occurrences;
    result.origins[record.origin] = (result.origins[record.origin] ?? 0) + record.occurrences;
    return result;
  }, { total: 0, "first-party": 0, "third-party": 0, origins: {} });
  return { ok: failures.length === 0, failures, removed, baselineCounts: counts(configuration.diagnostics), currentCounts: counts(current) };
}

function git(repoRoot, args) {
  try {
    return execFileSync("git", args, {
      cwd: repoRoot,
      encoding: "utf8",
      maxBuffer: GIT_OUTPUT_MAX_BUFFER,
      stdio: ["ignore", "pipe", "pipe"],
    }).trimEnd();
  } catch (error) {
    if (error?.code === "ENOBUFS") {
      const command = ["git", ...args].join(" ");
      const limitError = new Error(
        `Git output exceeded the warning-ratchet limit of ${GIT_OUTPUT_MAX_BUFFER} bytes while running: ${command}`,
        { cause: error },
      );
      limitError.code = GIT_OUTPUT_LIMIT_ERROR_CODE;
      throw limitError;
    }
    throw error;
  }
}

function isGitOutputLimitError(error) {
  return error?.code === GIT_OUTPUT_LIMIT_ERROR_CODE;
}

function resolveCommit(repoRoot, ref) {
  if (!ref || /^0+$/.test(ref)) throw new Error(`invalid comparison ref: ${ref || "<missing>"}`);
  try {
    return git(repoRoot, ["rev-parse", "--verify", `${ref}^{commit}`]);
  } catch (error) {
    if (isGitOutputLimitError(error)) throw error;
    throw new Error(`unresolvable comparison ref: ${ref}`);
  }
}

export function resolveTrustedComparison(repoRoot, baseRef, headRef = "HEAD") {
  const head = resolveCommit(repoRoot, headRef);
  let candidate = baseRef;
  if (!candidate) candidate = `${head}^`;
  const base = resolveCommit(repoRoot, candidate);
  if (base === head) throw new Error("comparison base and head must differ");
  let mergeBase;
  try {
    mergeBase = git(repoRoot, ["merge-base", base, head]);
  } catch (error) {
    if (isGitOutputLimitError(error)) throw error;
    throw new Error(`comparison refs have no merge base: ${base} ${head}`);
  }
  if (!mergeBase || mergeBase === head) throw new Error("trusted comparison resolved to self");
  return { base: mergeBase, head };
}

/**
 * Rebaseline audits must bind their evidence and inventory diff to the exact
 * caller-provided commits. Unlike normal ratchets, never substitute merge-base
 * for the requested base: that would let a sibling branch rebaseline against a
 * different inventory than the reviewer named.
 */
export function resolveExplicitAncestorComparison(repoRoot, baseRef, headRef) {
  if (typeof baseRef !== "string" || baseRef.trim().length === 0
    || typeof headRef !== "string" || headRef.trim().length === 0) {
    throw new Error("explicit comparison requires base and head refs");
  }
  const base = resolveCommit(repoRoot, baseRef);
  const head = resolveCommit(repoRoot, headRef);
  if (base === head) throw new Error("explicit comparison base and head must differ");
  try {
    git(repoRoot, ["merge-base", "--is-ancestor", base, head]);
  } catch (error) {
    if (isGitOutputLimitError(error)) throw error;
    throw new Error(`explicit comparison base is not an ancestor of head: ${base} ${head}`);
  }
  return { base, head };
}

function parseNameOnlyZ(output) {
  return output.split("\0").filter(Boolean).map(normalizeComparisonPath);
}

function parseNameOnlyZPreservingCase(output) {
  return output.split("\0").filter(Boolean).map(slash);
}

export function collectModifiedFiles(repoRoot, comparison) {
  const files = new Set(parseNameOnlyZ(git(repoRoot, ["diff", "--name-only", "-z", `${comparison.base}...${comparison.head}`])));
  for (const args of [["diff", "--name-only", "-z"], ["diff", "--cached", "--name-only", "-z"], ["ls-files", "--others", "--exclude-standard", "-z"]]) {
    for (const file of parseNameOnlyZ(git(repoRoot, args))) files.add(file);
  }
  return files;
}

function isRustWhitespaceCode(code) {
  return (code >= 0x09 && code <= 0x0d)
    || code === 0x20
    || code === 0x85
    || code === 0x200e
    || code === 0x200f
    || code === 0x2028
    || code === 0x2029;
}

function rustCommentEnd(text, index) {
  if (text[index] !== "/") return null;
  if (text[index + 1] === "/") {
    const newline = text.indexOf("\n", index + 2);
    return newline === -1 ? text.length : newline + 1;
  }
  if (text[index + 1] !== "*") return null;
  let depth = 1;
  let cursor = index + 2;
  while (cursor < text.length && depth > 0) {
    if (text[cursor] === "/" && text[cursor + 1] === "*") {
      depth += 1;
      cursor += 2;
    } else if (text[cursor] === "*" && text[cursor + 1] === "/") {
      depth -= 1;
      cursor += 2;
    } else {
      cursor += 1;
    }
  }
  return cursor;
}

function skipRustTrivia(text, index) {
  let cursor = index;
  while (cursor < text.length) {
    if (isRustWhitespaceCode(text.charCodeAt(cursor))) {
      cursor += 1;
      continue;
    }
    const commentEnd = rustCommentEnd(text, cursor);
    if (commentEnd === null) break;
    cursor = commentEnd;
  }
  return cursor;
}

function rustRawStringEnd(text, index) {
  let cursor;
  if (text[index] === "r") cursor = index + 1;
  else if (text[index] === "b" && text[index + 1] === "r") cursor = index + 2;
  else return null;
  let hashes = 0;
  while (text[cursor] === "#" && hashes <= 255) {
    hashes += 1;
    cursor += 1;
  }
  if (hashes > 255 || text[cursor] !== "\"") return null;
  const closing = `\"${"#".repeat(hashes)}`;
  const closingIndex = text.indexOf(closing, cursor + 1);
  return closingIndex === -1 ? text.length : closingIndex + closing.length;
}

function rustQuotedStringEnd(text, index) {
  let quoteIndex = index;
  if ((text[index] === "b" || text[index] === "c") && text[index + 1] === "\"") quoteIndex += 1;
  else if (text[index] !== "\"") return null;
  let cursor = quoteIndex + 1;
  while (cursor < text.length) {
    if (text[cursor] === "\\") cursor += 2;
    else if (text[cursor] === "\"") return cursor + 1;
    else cursor += 1;
  }
  return text.length;
}

function isRustIdentifierStart(character) {
  return character === "_" || /[a-zA-Z]/.test(character ?? "");
}

function isRustIdentifierContinue(character) {
  return isRustIdentifierStart(character) || /[0-9]/.test(character ?? "");
}

function rustSuppressionIdentifier(text, index) {
  let identifierStart = index;
  let cursor = index;
  if (text[cursor] === "r" && text[cursor + 1] === "#" && isRustIdentifierStart(text[cursor + 2])) {
    identifierStart = cursor + 2;
    cursor = identifierStart;
  }
  if (!isRustIdentifierStart(text[cursor])) return null;
  cursor += 1;
  while (isRustIdentifierContinue(text[cursor])) cursor += 1;
  const identifierLength = cursor - identifierStart;
  if (identifierLength === 5 && text.startsWith("allow", identifierStart)) return { end: cursor, kind: "allow" };
  if (identifierLength === 6 && text.startsWith("expect", identifierStart)) return { end: cursor, kind: "expect" };
  return null;
}

function rustNormalizedTokenHash(text, start, end) {
  const hash = createHash("sha256");
  let buffer = "";
  const append = (type, value) => {
    buffer += `${type}${value.length}:${value};`;
    if (buffer.length >= 64 * 1024) {
      hash.update(buffer);
      buffer = "";
    }
  };
  let cursor = start;
  while (cursor < end) {
    if (isRustWhitespaceCode(text.charCodeAt(cursor))) {
      cursor += 1;
      continue;
    }
    const commentEnd = rustCommentEnd(text, cursor);
    if (commentEnd !== null) {
      cursor = Math.min(commentEnd, end);
      continue;
    }
    const literalEnd = rustRawStringEnd(text, cursor) ?? rustQuotedStringEnd(text, cursor);
    if (literalEnd !== null) {
      const boundedLiteralEnd = Math.min(literalEnd, end);
      append("l", text.slice(cursor, boundedLiteralEnd));
      cursor = boundedLiteralEnd;
      continue;
    }
    let identifierStart = cursor;
    if (text[cursor] === "r" && text[cursor + 1] === "#" && isRustIdentifierStart(text[cursor + 2])) {
      identifierStart = cursor + 2;
      cursor = identifierStart;
    }
    if (isRustIdentifierStart(text[cursor])) {
      cursor += 1;
      while (cursor < end && isRustIdentifierContinue(text[cursor])) cursor += 1;
      append("i", text.slice(identifierStart, cursor));
      continue;
    }
    if (/[0-9]/.test(text[cursor])) {
      const numberStart = cursor;
      cursor += 1;
      while (cursor < end && /[0-9a-zA-Z_.]/.test(text[cursor])) cursor += 1;
      append("n", text.slice(numberStart, cursor));
      continue;
    }
    append("p", text[cursor]);
    cursor += 1;
  }
  if (buffer.length > 0) hash.update(buffer);
  return hash.digest("hex");
}

function rustAttributeScan(text, openBracketIndex) {
  let squareDepth = 1;
  let cursor = openBracketIndex + 1;
  const suppressionKinds = [];
  const finish = (end) => {
    if (suppressionKinds.length === 0) return { end, fingerprints: [] };
    const normalizedHash = rustNormalizedTokenHash(text, openBracketIndex, end);
    return {
      end,
      fingerprints: suppressionKinds.map((kind) => `attribute:${kind}:${normalizedHash}`),
    };
  };
  while (cursor < text.length) {
    const commentEnd = rustCommentEnd(text, cursor);
    if (commentEnd !== null) {
      cursor = commentEnd;
      continue;
    }
    const literalEnd = rustRawStringEnd(text, cursor) ?? rustQuotedStringEnd(text, cursor);
    if (literalEnd !== null) {
      cursor = literalEnd;
      continue;
    }
    if (text[cursor] === "[") {
      squareDepth += 1;
      cursor += 1;
      continue;
    }
    if (text[cursor] === "]") {
      squareDepth -= 1;
      cursor += 1;
      if (squareDepth === 0) return finish(cursor);
      continue;
    }
    const suppressionIdentifier = rustSuppressionIdentifier(text, cursor);
    if (suppressionIdentifier !== null
      && text[skipRustTrivia(text, suppressionIdentifier.end)] === "(") {
      suppressionKinds.push(suppressionIdentifier.kind);
    }
    if (text[cursor] === "r" && text[cursor + 1] === "#" && isRustIdentifierStart(text[cursor + 2])) cursor += 2;
    if (isRustIdentifierStart(text[cursor])) {
      cursor += 1;
      while (isRustIdentifierContinue(text[cursor])) cursor += 1;
      continue;
    }
    cursor += 1;
  }
  return finish(text.length);
}

function rustMacroInvocationSuppression(text, bangIndex) {
  const openIndex = skipRustTrivia(text, bangIndex + 1);
  if (text[openIndex] !== "(" && text[openIndex] !== "[" && text[openIndex] !== "{") return null;
  const firstToken = skipRustTrivia(text, openIndex + 1);
  const identifier = rustSuppressionIdentifier(text, firstToken);
  if (identifier === null) return null;
  const tokenEnd = skipRustTrivia(text, identifier.end);
  const expectedClose = text[openIndex] === "(" ? ")" : text[openIndex] === "[" ? "]" : "}";
  return text[tokenEnd] === "," || text[tokenEnd] === expectedClose ? identifier.kind : null;
}

function rustSuppressionFingerprints(text) {
  const fingerprints = [];
  let cursor = 0;
  while (cursor < text.length) {
    const commentEnd = rustCommentEnd(text, cursor);
    if (commentEnd !== null) {
      cursor = commentEnd;
      continue;
    }
    const literalEnd = rustRawStringEnd(text, cursor) ?? rustQuotedStringEnd(text, cursor);
    if (literalEnd !== null) {
      cursor = literalEnd;
      continue;
    }
    if (text[cursor] === "!") {
      const macroSuppression = rustMacroInvocationSuppression(text, cursor);
      if (macroSuppression !== null) fingerprints.push(`macro:${macroSuppression}`);
    }
    if (text[cursor] !== "#") {
      cursor += 1;
      continue;
    }
    let attributeStart = skipRustTrivia(text, cursor + 1);
    if (text[attributeStart] === "!") attributeStart = skipRustTrivia(text, attributeStart + 1);
    if (text[attributeStart] !== "[") {
      cursor += 1;
      continue;
    }
    const result = rustAttributeScan(text, attributeStart);
    fingerprints.push(...result.fingerprints);
    cursor = result.end;
  }
  return fingerprints;
}

export function detectSuppressionText(text) {
  const findings = SUPPRESSION_PATTERNS.filter(({ pattern }) => pattern.test(text)).map(({ id }) => id);
  if (rustSuppressionFingerprints(text).length > 0) findings.push("rust-allow-or-expect-attribute");
  return findings;
}

function addedDiffText(repoRoot, tailArgs) {
  return git(repoRoot, ["diff", "--unified=0", "--no-ext-diff", "--no-textconv", "--no-renames", "--text", ...tailArgs])
    .split(/\r?\n/)
    .filter((line) => line.startsWith("+") && !line.startsWith("+++" ))
    .map((line) => line.slice(1))
    .join("\n");
}

function boundedFileContentError(description, relative, reportedLimit) {
  const error = new Error(
    `${description} exceeded the warning-ratchet limit of ${reportedLimit} bytes while reading: ${slash(relative)}`,
  );
  error.code = FILE_CONTENT_LIMIT_ERROR_CODE;
  return error;
}

function isMissingFileError(error) {
  return error?.code === "ENOENT" || error?.code === "ENOTDIR";
}

function readBoundedFileText(file, displayPath, maxBytes, description, reportedLimit = maxBytes) {
  const buffers = [];
  let bytesRead = 0;
  let descriptor;
  let operationError;
  let overflow = false;
  try {
    descriptor = openSync(file, "r");
    const scratch = Buffer.allocUnsafe(Math.min(FILE_READ_CHUNK_BYTES, maxBytes + 1));
    while (bytesRead <= maxBytes) {
      // Probe no further than one byte beyond the remaining budget. This stays bounded
      // even if the file grows after it is opened; no prior stat result is trusted.
      const requestedBytes = Math.min(scratch.length, maxBytes + 1 - bytesRead);
      const currentRead = readSync(descriptor, scratch, 0, requestedBytes, null);
      if (currentRead === 0) break;
      bytesRead += currentRead;
      if (bytesRead > maxBytes) {
        overflow = true;
        break;
      }
      buffers.push(Buffer.from(scratch.subarray(0, currentRead)));
    }
  } catch (error) {
    operationError = error;
  } finally {
    if (descriptor !== undefined) {
      try {
        closeSync(descriptor);
      } catch (error) {
        operationError ??= error;
      }
    }
  }
  if (overflow) throw boundedFileContentError(description, displayPath, reportedLimit);
  if (operationError) throw operationError;
  return { bytesRead, text: Buffer.concat(buffers, bytesRead).toString("utf8") };
}

function gitErrorDetail(error) {
  const stderr = Buffer.isBuffer(error?.stderr) ? error.stderr.toString("utf8") : String(error?.stderr ?? "");
  return `${stderr}\n${String(error?.message ?? "")}`;
}

function isMissingGitSnapshotError(error) {
  const detail = gitErrorDetail(error);
  return /does not exist in\b|exists on disk, but not in (?:the index|['"])|does not exist \(neither on disk nor in the index\)/i.test(detail);
}

function readGitSnapshotText(repoRoot, snapshot, relative) {
  const spec = snapshot === null ? `:${slash(relative)}` : `${snapshot}:${slash(relative)}`;
  try {
    return git(repoRoot, ["show", spec]);
  } catch (error) {
    if (isGitOutputLimitError(error)) throw error;
    if (isMissingGitSnapshotError(error)) return null;
    throw new Error(`unable to read Git snapshot blob: ${spec}`, { cause: error });
  }
}

function modifiedPathsFromDiff(repoRoot, args) {
  return parseNameOnlyZPreservingCase(git(repoRoot, args));
}

function inspectModifiedSemanticSnapshots({
  repoRoot,
  comparison,
  headPaths,
  stagedPaths,
  worktreePaths,
  untracked,
  untrackedTextByPath,
  matches,
  inspectText,
  worktreeDescription,
}) {
  const findings = [];
  const inspectSnapshot = (text) => {
    if (text !== null) findings.push(...inspectText(text));
  };
  for (const relative of headPaths.filter(matches)) {
    inspectSnapshot(readGitSnapshotText(repoRoot, comparison.head, relative));
  }
  for (const relative of stagedPaths.filter(matches)) {
    inspectSnapshot(readGitSnapshotText(repoRoot, null, relative));
  }
  let worktreeBytesRead = 0;
  for (const relative of worktreePaths.filter(matches)) {
    try {
      const result = readBoundedFileText(
        path.join(repoRoot, relative),
        relative,
        CURRENT_FILE_CONTENT_MAX_BYTES - worktreeBytesRead,
        worktreeDescription,
        CURRENT_FILE_CONTENT_MAX_BYTES,
      );
      worktreeBytesRead += result.bytesRead;
      inspectSnapshot(result.text);
    } catch (error) {
      if (isMissingFileError(error)) continue;
      throw error;
    }
  }
  for (const relative of untracked.filter(matches)) {
    const normalizedRelative = slash(relative);
    // Reuse the bounded snapshot. Retrying an unreadable file here could race into
    // an unbounded read after it changes size or permissions.
    if (untrackedTextByPath.has(normalizedRelative)) inspectSnapshot(untrackedTextByPath.get(normalizedRelative));
  }
  return findings;
}

function introducesRustSuppression(sourceText, targetText) {
  const available = new Map();
  for (const fingerprint of rustSuppressionFingerprints(sourceText)) {
    available.set(fingerprint, (available.get(fingerprint) ?? 0) + 1);
  }
  for (const fingerprint of rustSuppressionFingerprints(targetText)) {
    const remaining = available.get(fingerprint) ?? 0;
    if (remaining === 0) return true;
    available.set(fingerprint, remaining - 1);
  }
  return false;
}

function inspectRustSuppressionTransitions({
  repoRoot,
  comparison,
  headPaths,
  stagedPaths,
  worktreePaths,
  untracked,
  untrackedTextByPath,
}) {
  const rustPaths = (paths) => paths.filter((relative) => /\.rs$/i.test(slash(relative)));
  for (const relative of rustPaths(headPaths)) {
    const source = readGitSnapshotText(repoRoot, comparison.base, relative) ?? "";
    const target = readGitSnapshotText(repoRoot, comparison.head, relative) ?? "";
    if (introducesRustSuppression(source, target)) return ["rust-allow-or-expect-attribute"];
  }
  for (const relative of rustPaths(stagedPaths)) {
    const source = readGitSnapshotText(repoRoot, "HEAD", relative) ?? "";
    const target = readGitSnapshotText(repoRoot, null, relative) ?? "";
    if (introducesRustSuppression(source, target)) return ["rust-allow-or-expect-attribute"];
  }
  let worktreeBytesRead = 0;
  for (const relative of rustPaths(worktreePaths)) {
    const source = readGitSnapshotText(repoRoot, null, relative) ?? "";
    let target = "";
    try {
      const result = readBoundedFileText(
        path.join(repoRoot, relative),
        relative,
        CURRENT_FILE_CONTENT_MAX_BYTES - worktreeBytesRead,
        "Modified worktree Rust source content aggregate",
        CURRENT_FILE_CONTENT_MAX_BYTES,
      );
      worktreeBytesRead += result.bytesRead;
      target = result.text;
    } catch (error) {
      if (!isMissingFileError(error)) throw error;
    }
    if (introducesRustSuppression(source, target)) return ["rust-allow-or-expect-attribute"];
  }
  for (const relative of rustPaths(untracked)) {
    const target = untrackedTextByPath.get(slash(relative));
    if (target !== undefined && introducesRustSuppression("", target)) return ["rust-allow-or-expect-attribute"];
  }
  return [];
}

export function findAddedSuppressions(repoRoot, comparison) {
  const chunks = [
    addedDiffText(repoRoot, [`${comparison.base}...${comparison.head}`]),
    addedDiffText(repoRoot, []),
    addedDiffText(repoRoot, ["--cached"]),
  ];
  const untracked = parseNameOnlyZPreservingCase(git(repoRoot, ["ls-files", "--others", "--exclude-standard", "-z"]));
  const untrackedTextByPath = new Map();
  let untrackedBytesRead = 0;
  for (const relative of untracked) {
    try {
      const result = readBoundedFileText(
        path.join(repoRoot, relative),
        relative,
        CURRENT_FILE_CONTENT_MAX_BYTES - untrackedBytesRead,
        "Untracked content aggregate",
        CURRENT_FILE_CONTENT_MAX_BYTES,
      );
      untrackedBytesRead += result.bytesRead;
      untrackedTextByPath.set(slash(relative), result.text);
      chunks.push(result.text);
    } catch (error) {
      if (error?.code === FILE_CONTENT_LIMIT_ERROR_CODE) throw error;
      // Preserve the existing boundary for an untracked file that disappears or
      // becomes unreadable during inspection. Limit overflow is never hidden here.
    }
  }
  const headPaths = modifiedPathsFromDiff(repoRoot, ["diff", "--no-renames", "--name-only", "-z", `${comparison.base}...${comparison.head}`]);
  const stagedPaths = modifiedPathsFromDiff(repoRoot, ["diff", "--no-renames", "--cached", "--name-only", "-z"]);
  const worktreePaths = modifiedPathsFromDiff(repoRoot, ["diff", "--no-renames", "--name-only", "-z"]);
  const semanticFindings = [
    ...inspectRustSuppressionTransitions({
      repoRoot,
      comparison,
      headPaths,
      stagedPaths,
      worktreePaths,
      untracked,
      untrackedTextByPath,
    }),
    ...inspectModifiedSemanticSnapshots({
      repoRoot,
      comparison,
      headPaths,
      stagedPaths,
      worktreePaths,
      untracked,
      untrackedTextByPath,
      matches: (relative) => /(?:^|\/)\.cargo\/config(?:\.toml)?$/i.test(slash(relative)),
      inspectText: inspectCargoConfigText,
      worktreeDescription: "Modified worktree Cargo config content aggregate",
    }),
    ...inspectModifiedSemanticSnapshots({
      repoRoot,
      comparison,
      headPaths,
      stagedPaths,
      worktreePaths,
      untracked,
      untrackedTextByPath,
      matches: (relative) => /(?:^|\/)Cargo\.toml$/i.test(slash(relative)),
      inspectText: inspectCargoManifestLintText,
      worktreeDescription: "Modified worktree Cargo manifest content aggregate",
    }),
  ];
  return [...new Set([...detectSuppressionText(chunks.join("\n")), ...semanticFindings])];
}

export function loadInventoryAtRef(repoRoot, ref, relativePath, allowMissing = false) {
  const text = readGitSnapshotText(repoRoot, ref, relativePath);
  if (text === null) {
    if (allowMissing) return null;
    throw new Error(`trusted prior inventory is missing at ${ref}:${relativePath}`);
  }
  try {
    return JSON.parse(text);
  } catch (error) {
    throw new Error(`trusted prior inventory is invalid at ${ref}:${relativePath}`, { cause: error });
  }
}

export function findBaselineLaundering(previousConfiguration, currentConfiguration) {
  if (!previousConfiguration) return ["trusted prior configuration is missing"];
  return JSON.stringify(previousConfiguration) === JSON.stringify(currentConfiguration)
    ? []
    : ["warning baseline/configuration metadata is immutable; use a separately reviewed rebaseline checkpoint"];
}

const ZERO_WARNING_PROMOTION_FIELDS = [
  "id",
  "command",
  "platform",
  "profile",
  "features",
  "timeoutMs",
  "firstPartyManifests",
];

export const ZERO_WARNING_PROMOTION_ALLOWED_FILES = Object.freeze([
  "qa/warnings/warning-inventory.json",
]);

export const OUTPUT_MARKER_REBASELINE_ALLOWED_FILES = Object.freeze([
  "qa/warnings/warning-inventory.json",
]);

export const ARTIFACT_REBASELINE_ALLOWED_FILES = Object.freeze([
  "qa/warnings/warning-inventory.json",
]);

const VARIABLE_MODULE_COUNT_MARKER = /^\d+ modules transformed\.$/;
const STABLE_MODULE_TRANSFORMED_MARKER = "modules transformed.";

function selectedConfigurationShape(configuration) {
  return Object.fromEntries(ZERO_WARNING_PROMOTION_FIELDS.map((field) => [field, configuration?.[field]]));
}

function sameJson(left, right) {
  return JSON.stringify(left) === JSON.stringify(right);
}

function configurationsById(inventory, label) {
  if (!Array.isArray(inventory?.configurations)) throw new Error(`${label} inventory configurations are invalid`);
  const byId = new Map();
  for (const configuration of inventory.configurations) {
    if (byId.has(configuration.id)) throw new Error(`${label} inventory has duplicate configuration id: ${configuration.id}`);
    byId.set(configuration.id, configuration);
  }
  return byId;
}

function assertPromotionRunnerResult(configuration, result) {
  if (result.timedOut) throw new Error(`${configuration.id} warning command timed out during promotion audit`);
  if (result.outputLimitExceeded) throw new Error(`${configuration.id} warning command exceeded bounded output during promotion audit`);
  if (result.exitCode !== 0) throw new Error(`${configuration.id} warning command exited with ${result.exitCode} during promotion audit`);
  if (configuration.command.executable === "cargo") {
    if ((result.invalidJsonLines?.length ?? 0) > 0) throw new Error(`${configuration.id} emitted malformed Cargo JSON during promotion audit`);
    if (JSON.stringify(result.buildFinished ?? []) !== JSON.stringify([true])) {
      throw new Error(`${configuration.id} Cargo build-finished coverage is not exactly one successful build`);
    }
    if (result.stderrWarning) throw new Error(`${configuration.id} emitted warning-shaped Cargo stderr during promotion audit`);
    const coverage = compareArtifactCoverage(configuration.expectedArtifacts, result.artifacts ?? []);
    if (!coverage.ok) throw new Error(`${configuration.id} artifact coverage mismatch during promotion audit`);
  } else {
    if (result.warningShaped) throw new Error(`${configuration.id} emitted warning-shaped output during promotion audit`);
    if (!result.markerCoverage?.ok) {
      throw new Error(`${configuration.id} output marker coverage mismatch during promotion audit`);
    }
    if ((configuration.expectedArtifacts?.length ?? 0) !== 0) {
      throw new Error(`${configuration.id} generic promotion must have zero artifact expectations`);
    }
  }
  if ((result.diagnostics?.length ?? 0) !== 0) {
    throw new Error(`${configuration.id} produced diagnostics during zero-warning promotion`);
  }
}

/**
 * Audits an already-authored pending->enforced inventory transition. This is
 * intentionally opt-in: it never writes the inventory and is not used by the
 * normal warning gate. The caller must provide both trusted refs explicitly.
 */
export async function auditZeroWarningPromotion({
  repoRoot,
  baseRef,
  headRef,
  configurationId,
  inventoryPath = "qa/warnings/warning-inventory.json",
  schema = null,
  environment = process.env,
  runConfiguration = runWarningConfiguration,
  allowedFiles = ZERO_WARNING_PROMOTION_ALLOWED_FILES,
}) {
  if (typeof baseRef !== "string" || baseRef.trim().length === 0
    || typeof headRef !== "string" || headRef.trim().length === 0) {
    throw new Error("zero-warning promotion requires explicit base and head refs");
  }
  const comparison = resolveTrustedComparison(repoRoot, baseRef, headRef);
  const priorInventory = loadInventoryAtRef(repoRoot, comparison.base, inventoryPath);
  const currentInventory = loadInventoryAtRef(repoRoot, comparison.head, inventoryPath);
  const priorErrors = validateInventory(priorInventory, schema);
  if (priorErrors.length > 0) throw new Error(`trusted prior inventory validation failed: ${priorErrors.join("; ")}`);
  const currentErrors = validateInventory(currentInventory, schema);
  if (currentErrors.length > 0) throw new Error(`promotion head inventory validation failed: ${currentErrors.join("; ")}`);
  if (!sameJson(priorInventory.policy, currentInventory.policy)) {
    throw new Error("zero-warning promotion cannot change inventory policy");
  }

  const priorById = configurationsById(priorInventory, "trusted prior");
  const currentById = configurationsById(currentInventory, "promotion head");
  const priorIds = [...priorById.keys()].sort();
  const currentIds = [...currentById.keys()].sort();
  if (!sameJson(priorIds, currentIds)) {
    throw new Error("zero-warning promotion rejects configuration add/remove, including pending entries");
  }

  const promoted = [];
  for (const id of priorIds) {
    const previous = priorById.get(id);
    const current = currentById.get(id);
    if (previous.status === "enforced") {
      if (!sameJson(previous, current)) {
        throw new Error(`zero-warning promotion rejects enforced configuration change: ${id}`);
      }
      continue;
    }
    if (previous.status !== "pending") throw new Error(`trusted prior configuration ${id} has invalid status`);
    if (current.status === "pending") {
      if (!sameJson(previous, current)) {
        throw new Error(`zero-warning promotion rejects pending configuration change: ${id}`);
      }
      continue;
    }
    if (current.status !== "enforced") throw new Error(`promotion status is invalid for ${id}`);
    if (!sameJson(selectedConfigurationShape(previous), selectedConfigurationShape(current))) {
      throw new Error(`zero-warning promotion changed immutable configuration fields: ${id}`);
    }
    if (previous.requiredEnvironment !== undefined
      && !sameJson(previous.requiredEnvironment, current.requiredEnvironment)) {
      throw new Error(`zero-warning promotion changed an existing requiredEnvironment: ${id}`);
    }
    if ((current.diagnostics?.length ?? 0) !== 0 || (current.externalWarningAllows?.length ?? 0) !== 0) {
      throw new Error(`zero-warning promotion requires empty diagnostics and external allows: ${id}`);
    }
    if (current.evidence?.commit !== comparison.base) {
      throw new Error(`zero-warning promotion evidence.commit must equal trusted base ${comparison.base}: ${id}`);
    }
    if (current.command.executable === "pnpm") {
      validateExpectedOutputMarkers(current.expectedOutputMarkers);
      if ((current.expectedArtifacts?.length ?? 0) !== 0) {
        throw new Error(`zero-warning promotion generic artifact coverage must be empty: ${id}`);
      }
    } else if (!Array.isArray(current.expectedArtifacts) || current.expectedArtifacts.length === 0) {
      throw new Error(`zero-warning promotion Cargo artifact coverage must be non-empty: ${id}`);
    }
    promoted.push(current);
  }
  if (promoted.length === 0) throw new Error("zero-warning promotion requires at least one pending->enforced configuration");

  const modifiedFiles = collectModifiedFiles(repoRoot, comparison);
  const allowed = new Set(allowedFiles.map((file) => normalizeComparisonPath(file)));
  const disallowed = [...modifiedFiles].filter((file) => !allowed.has(normalizeComparisonPath(file))).sort();
  if (disallowed.length > 0) {
    throw new Error(`zero-warning promotion changed files outside inventory/warning gate scope: ${disallowed.join(", ")}`);
  }
  const suppressions = findAddedSuppressions(repoRoot, comparison);
  if (suppressions.length > 0) throw new Error(`zero-warning promotion found suppression loopholes: ${suppressions.join(", ")}`);

  if (typeof configurationId !== "string" || configurationId.trim().length === 0) {
    throw new Error("zero-warning promotion requires an explicit configuration id");
  }
  const configuration = promoted.find((candidate) => candidate.id === configurationId);
  if (!configuration) {
    throw new Error(`zero-warning promotion configuration is not a pending->enforced transition: ${configurationId}`);
  }
  const hostPlatform = detectHostPlatform();
  if (configuration.platform !== hostPlatform) {
    throw new Error(`configuration ${configuration.id} targets ${configuration.platform}, but this host is ${hostPlatform}`);
  }
  const currentToolchain = detectToolchain();
  for (const [tool, expected] of Object.entries(configuration.evidence?.toolchain ?? {})) {
    if (currentToolchain[tool] !== expected) {
      throw new Error(`toolchain drift for ${tool}: inventory=${expected}; current=${currentToolchain[tool]}`);
    }
  }
  const result = await runConfiguration(configuration, repoRoot, environment);
  assertPromotionRunnerResult(configuration, result);
  return {
    comparison,
    configurationId,
    promotedIds: promoted.map((candidate) => candidate.id),
    modifiedFiles,
    results: [{ id: configuration.id, result }],
  };
}

function expectedEvidenceCommand(configuration) {
  return `${configuration.command.executable} ${configuration.command.args.join(" ")}`;
}

function assertOutputMarkerRebaselineConfiguration(previous, current, comparison) {
  if (previous.status !== "enforced" || current.status !== "enforced") {
    throw new Error(`output-marker rebaseline only supports an enforced configuration: ${current.id}`);
  }
  if (previous.command?.executable === "cargo" || current.command?.executable === "cargo") {
    throw new Error(`output-marker rebaseline only supports an enforced generic configuration: ${current.id}`);
  }
  if ((previous.expectedArtifacts?.length ?? 0) !== 0
    || (current.expectedArtifacts?.length ?? 0) !== 0
    || (previous.diagnostics?.length ?? 0) !== 0
    || (current.diagnostics?.length ?? 0) !== 0
    || (previous.externalWarningAllows?.length ?? 0) !== 0
    || (current.externalWarningAllows?.length ?? 0) !== 0) {
    throw new Error(`output-marker rebaseline requires a generic zero-warning configuration: ${current.id}`);
  }

  const previousShape = { ...previous };
  const currentShape = { ...current };
  delete previousShape.expectedOutputMarkers;
  delete previousShape.evidence;
  delete currentShape.expectedOutputMarkers;
  delete currentShape.evidence;
  if (!sameJson(previousShape, currentShape)) {
    throw new Error(`output-marker rebaseline changed immutable configuration fields: ${current.id}`);
  }

  validateExpectedOutputMarkers(previous.expectedOutputMarkers);
  validateExpectedOutputMarkers(current.expectedOutputMarkers);
  const changedMarkers = previous.expectedOutputMarkers
    .map((marker, index) => ({ marker, replacement: current.expectedOutputMarkers[index] }))
    .filter(({ marker, replacement }) => marker !== replacement);
  if (previous.expectedOutputMarkers.length !== current.expectedOutputMarkers.length
    || changedMarkers.length !== 1
    || !VARIABLE_MODULE_COUNT_MARKER.test(changedMarkers[0]?.marker ?? "")
    || changedMarkers[0]?.replacement !== STABLE_MODULE_TRANSFORMED_MARKER) {
    throw new Error(
      `output-marker rebaseline only permits one literal module-count marker to become '${STABLE_MODULE_TRANSFORMED_MARKER}': ${current.id}`,
    );
  }

  if (current.evidence?.commit !== comparison.base) {
    throw new Error(`output-marker rebaseline evidence.commit must equal trusted base ${comparison.base}: ${current.id}`);
  }
  if (current.evidence?.command !== expectedEvidenceCommand(current)) {
    throw new Error(`output-marker rebaseline evidence.command must exactly match the generic command: ${current.id}`);
  }
}

function assertOutputMarkerRebaselineRunnerResult(configuration, result) {
  if (result.timedOut) throw new Error(`${configuration.id} warning command timed out during output-marker rebaseline`);
  if (result.outputLimitExceeded) throw new Error(`${configuration.id} warning command exceeded output limit during output-marker rebaseline`);
  if (result.exitCode !== 0) throw new Error(`${configuration.id} warning command exited with ${result.exitCode} during output-marker rebaseline`);
  if (result.warningShaped) throw new Error(`${configuration.id} emitted warning-shaped output during output-marker rebaseline`);
  if (!result.markerCoverage?.ok) {
    throw new Error(`${configuration.id} output marker coverage mismatch during output-marker rebaseline`);
  }
  if ((result.diagnostics?.length ?? 0) !== 0) {
    throw new Error(`${configuration.id} produced diagnostics during output-marker rebaseline`);
  }
}

/**
 * Audits one reviewed generic-output marker rebaseline. This deliberately does
 * not relax normal inventory immutability or write the inventory itself.
 */
export async function auditOutputMarkerRebaseline({
  repoRoot,
  baseRef,
  headRef,
  configurationId,
  inventoryPath = "qa/warnings/warning-inventory.json",
  schema = null,
  environment = process.env,
  runConfiguration = runWarningConfiguration,
  allowedFiles = OUTPUT_MARKER_REBASELINE_ALLOWED_FILES,
}) {
  if (typeof baseRef !== "string" || baseRef.trim().length === 0
    || typeof headRef !== "string" || headRef.trim().length === 0) {
    throw new Error("output-marker rebaseline requires explicit base and head refs");
  }
  if (typeof configurationId !== "string" || configurationId.trim().length === 0) {
    throw new Error("output-marker rebaseline requires an explicit configuration id");
  }
  const comparison = resolveExplicitAncestorComparison(repoRoot, baseRef, headRef);
  const priorInventory = loadInventoryAtRef(repoRoot, comparison.base, inventoryPath);
  const currentInventory = loadInventoryAtRef(repoRoot, comparison.head, inventoryPath);
  const priorErrors = validateInventory(priorInventory, schema);
  if (priorErrors.length > 0) throw new Error(`trusted prior inventory validation failed: ${priorErrors.join("; ")}`);
  const currentErrors = validateInventory(currentInventory, schema);
  if (currentErrors.length > 0) throw new Error(`output-marker rebaseline head inventory validation failed: ${currentErrors.join("; ")}`);
  if (!sameJson(priorInventory.policy, currentInventory.policy)) {
    throw new Error("output-marker rebaseline cannot change inventory policy");
  }

  const priorById = configurationsById(priorInventory, "trusted prior");
  const currentById = configurationsById(currentInventory, "rebaseline head");
  const priorIds = [...priorById.keys()].sort();
  const currentIds = [...currentById.keys()].sort();
  if (!sameJson(priorIds, currentIds)) {
    throw new Error("output-marker rebaseline rejects configuration add/remove");
  }
  const previous = priorById.get(configurationId);
  const configuration = currentById.get(configurationId);
  if (!previous || !configuration) throw new Error(`warning inventory has no configuration named ${configurationId}`);
  for (const id of priorIds) {
    if (id === configurationId) continue;
    if (!sameJson(priorById.get(id), currentById.get(id))) {
      throw new Error(`output-marker rebaseline changed another configuration: ${id}`);
    }
  }
  assertOutputMarkerRebaselineConfiguration(previous, configuration, comparison);

  const modifiedFiles = collectModifiedFiles(repoRoot, comparison);
  const allowed = new Set(allowedFiles.map((file) => normalizeComparisonPath(file)));
  const disallowed = [...modifiedFiles].filter((file) => !allowed.has(normalizeComparisonPath(file))).sort();
  if (disallowed.length > 0) {
    throw new Error(`output-marker rebaseline changed files outside inventory/warning gate scope: ${disallowed.join(", ")}`);
  }
  const suppressions = findAddedSuppressions(repoRoot, comparison);
  if (suppressions.length > 0) throw new Error(`output-marker rebaseline found suppression loopholes: ${suppressions.join(", ")}`);

  const hostPlatform = detectHostPlatform();
  if (configuration.platform !== hostPlatform) {
    throw new Error(`configuration ${configuration.id} targets ${configuration.platform}, but this host is ${hostPlatform}`);
  }
  const currentToolchain = detectToolchain();
  for (const [tool, expected] of Object.entries(configuration.evidence?.toolchain ?? {})) {
    if (currentToolchain[tool] !== expected) {
      throw new Error(`toolchain drift for ${tool}: inventory=${expected}; current=${currentToolchain[tool]}`);
    }
  }
  const result = await runConfiguration(configuration, repoRoot, environment);
  assertOutputMarkerRebaselineRunnerResult(configuration, result);
  return { comparison, configurationId, modifiedFiles, result };
}

function assertArtifactRebaselineConfiguration(previous, current, comparison) {
  if (previous.status !== "enforced" || current.status !== "enforced") {
    throw new Error(`artifact rebaseline only supports an enforced configuration: ${current.id}`);
  }
  if (previous.command?.executable !== "cargo" || current.command?.executable !== "cargo") {
    throw new Error(`artifact rebaseline only supports an enforced Cargo configuration: ${current.id}`);
  }
  if (!sameJson(previous.diagnostics ?? [], current.diagnostics ?? [])
    || !sameJson(previous.externalWarningAllows ?? [], current.externalWarningAllows ?? [])) {
    throw new Error(`artifact rebaseline cannot change diagnostics or external warning allows: ${current.id}`);
  }

  const previousShape = { ...previous };
  const currentShape = { ...current };
  delete previousShape.expectedArtifacts;
  delete previousShape.evidence;
  delete currentShape.expectedArtifacts;
  delete currentShape.evidence;
  if (!sameJson(previousShape, currentShape)) {
    throw new Error(`artifact rebaseline changed immutable configuration fields: ${current.id}`);
  }
  if (!sameJson(previous.evidence?.toolchain, current.evidence?.toolchain)) {
    throw new Error(`artifact rebaseline changed the recorded toolchain: ${current.id}`);
  }

  const previousKeys = new Set((previous.expectedArtifacts ?? []).map(artifactIdentity));
  const currentKeys = new Set((current.expectedArtifacts ?? []).map(artifactIdentity));
  const removed = [...previousKeys].filter((key) => !currentKeys.has(key));
  const added = [...currentKeys].filter((key) => !previousKeys.has(key));
  if (removed.length > 0) {
    throw new Error(`artifact rebaseline cannot remove existing artifacts: ${current.id}`);
  }
  if (added.length === 0) {
    throw new Error(`artifact rebaseline requires at least one added artifact: ${current.id}`);
  }
  if (current.evidence?.commit !== comparison.base) {
    throw new Error(`artifact rebaseline evidence.commit must equal trusted base ${comparison.base}: ${current.id}`);
  }
  if (current.evidence?.command !== expectedEvidenceCommand(current)) {
    throw new Error(`artifact rebaseline evidence.command must exactly match the Cargo command: ${current.id}`);
  }
}

function assertArtifactRebaselineRunnerResult(configuration, result) {
  if (result.timedOut) throw new Error(`${configuration.id} warning command timed out during artifact rebaseline`);
  if (result.outputLimitExceeded) throw new Error(`${configuration.id} warning command exceeded output limit during artifact rebaseline`);
  if (result.exitCode !== 0) throw new Error(`${configuration.id} warning command exited with ${result.exitCode} during artifact rebaseline`);
  if ((result.invalidJsonLines?.length ?? 0) > 0) {
    throw new Error(`${configuration.id} emitted malformed Cargo JSON during artifact rebaseline`);
  }
  if (JSON.stringify(result.buildFinished ?? []) !== JSON.stringify([true])) {
    throw new Error(`${configuration.id} Cargo build-finished coverage is not exactly one successful build during artifact rebaseline`);
  }
  if (result.stderrWarning) throw new Error(`${configuration.id} emitted warning-shaped Cargo stderr during artifact rebaseline`);
  const coverage = compareArtifactCoverage(configuration.expectedArtifacts, result.artifacts ?? []);
  if (!coverage.ok) throw new Error(`${configuration.id} artifact coverage mismatch during artifact rebaseline`);
  const diagnostics = compareDiagnostics(configuration, result.diagnostics ?? []);
  if (!diagnostics.ok) {
    throw new Error(`${configuration.id} diagnostic ratchet failed during artifact rebaseline: ${diagnostics.failures.join("; ")}`);
  }
  return { coverage, diagnostics };
}

/**
 * Audits a narrowly scoped Cargo artifact-coverage rebaseline. This is an
 * explicit, inventory-only checkpoint for a real compiler target becoming
 * observable in an existing command. It never writes the inventory and never
 * relaxes the normal warning gate's immutable-baseline rule.
 */
export async function auditArtifactRebaseline({
  repoRoot,
  baseRef,
  headRef,
  configurationId,
  inventoryPath = "qa/warnings/warning-inventory.json",
  schema = null,
  environment = process.env,
  runConfiguration = runWarningConfiguration,
  allowedFiles = ARTIFACT_REBASELINE_ALLOWED_FILES,
}) {
  if (typeof baseRef !== "string" || baseRef.trim().length === 0
    || typeof headRef !== "string" || headRef.trim().length === 0) {
    throw new Error("artifact rebaseline requires explicit base and head refs");
  }
  if (typeof configurationId !== "string" || configurationId.trim().length === 0) {
    throw new Error("artifact rebaseline requires an explicit configuration id");
  }
  const comparison = resolveExplicitAncestorComparison(repoRoot, baseRef, headRef);
  const priorInventory = loadInventoryAtRef(repoRoot, comparison.base, inventoryPath);
  const currentInventory = loadInventoryAtRef(repoRoot, comparison.head, inventoryPath);
  const priorErrors = validateInventory(priorInventory, schema);
  if (priorErrors.length > 0) throw new Error(`trusted prior inventory validation failed: ${priorErrors.join("; ")}`);
  const currentErrors = validateInventory(currentInventory, schema);
  if (currentErrors.length > 0) throw new Error(`artifact rebaseline head inventory validation failed: ${currentErrors.join("; ")}`);
  if (!sameJson(priorInventory.policy, currentInventory.policy)) {
    throw new Error("artifact rebaseline cannot change inventory policy");
  }

  const priorById = configurationsById(priorInventory, "trusted prior");
  const currentById = configurationsById(currentInventory, "artifact rebaseline head");
  const priorIds = [...priorById.keys()].sort();
  const currentIds = [...currentById.keys()].sort();
  if (!sameJson(priorIds, currentIds)) {
    throw new Error("artifact rebaseline rejects configuration add/remove");
  }
  const previous = priorById.get(configurationId);
  const configuration = currentById.get(configurationId);
  if (!previous || !configuration) throw new Error(`warning inventory has no configuration named ${configurationId}`);
  for (const id of priorIds) {
    if (id === configurationId) continue;
    if (!sameJson(priorById.get(id), currentById.get(id))) {
      throw new Error(`artifact rebaseline changed another configuration: ${id}`);
    }
  }
  assertArtifactRebaselineConfiguration(previous, configuration, comparison);

  const modifiedFiles = collectModifiedFiles(repoRoot, comparison);
  const allowed = new Set(allowedFiles.map((file) => normalizeComparisonPath(file)));
  const disallowed = [...modifiedFiles].filter((file) => !allowed.has(normalizeComparisonPath(file))).sort();
  if (disallowed.length > 0) {
    throw new Error(`artifact rebaseline changed files outside inventory/warning gate scope: ${disallowed.join(", ")}`);
  }
  const suppressions = findAddedSuppressions(repoRoot, comparison);
  if (suppressions.length > 0) throw new Error(`artifact rebaseline found suppression loopholes: ${suppressions.join(", ")}`);

  const hostPlatform = detectHostPlatform();
  if (configuration.platform !== hostPlatform) {
    throw new Error(`configuration ${configuration.id} targets ${configuration.platform}, but this host is ${hostPlatform}`);
  }
  const currentToolchain = detectToolchain();
  for (const [tool, expected] of Object.entries(configuration.evidence?.toolchain ?? {})) {
    if (currentToolchain[tool] !== expected) {
      throw new Error(`toolchain drift for ${tool}: inventory=${expected}; current=${currentToolchain[tool]}`);
    }
  }
  const result = await runConfiguration(configuration, repoRoot, environment);
  const checks = assertArtifactRebaselineRunnerResult(configuration, result);
  return { comparison, configurationId, modifiedFiles, result, ...checks };
}

export function loadInventory(file) {
  return JSON.parse(readFileSync(file, "utf8"));
}

export function getConfiguration(inventory, id) {
  const configuration = inventory.configurations.find((candidate) => candidate.id === id);
  if (!configuration) throw new Error(`warning inventory has no configuration named ${id}`);
  return configuration;
}

export function detectHostPlatform() {
  const os = process.platform === "win32" ? "windows" : process.platform === "darwin" ? "macos" : process.platform;
  const arch = process.arch === "x64" ? "x86_64" : process.arch === "arm64" ? "aarch64" : process.arch;
  return process.platform === "win32" ? `${os}-${arch}-msvc` : `${os}-${arch}`;
}

function versionLine(executable, args) {
  const pnpmInvocation = executable === "pnpm" ? resolvePnpmInvocation(args) : null;
  const resolvedExecutable = pnpmInvocation?.executable ?? executable;
  const resolvedArgs = pnpmInvocation?.args ?? args;
  return execFileSync(resolvedExecutable, resolvedArgs, {
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
    shell: pnpmInvocation?.shell ?? false,
  }).trim().replace(/^v/, "");
}

export function detectToolchain() {
  return {
    cargo: versionLine("cargo", ["--version"]).replace(/^cargo\s+/, ""),
    rustc: versionLine("rustc", ["--version"]).replace(/^rustc\s+/, ""),
    node: versionLine("node", ["--version"]),
    pnpm: versionLine("pnpm", ["--version"]),
  };
}

export function annotateInventoryDiagnostics(diagnostics) {
  return diagnostics.map((diagnostic) => ({
    ...diagnostic,
    owner: diagnostic.package === "syndocal" ? "app-runtime" : diagnostic.package,
    reason: `Tracked pre-W0 ${diagnostic.code ?? "compiler"} diagnostic: ${diagnostic.message}`,
    removalCheckpoint: `W2-${diagnostic.package}-warning-cleanup`,
  }));
}
