import { createHash } from "node:crypto";
import { execFileSync, spawn } from "node:child_process";
import { once } from "node:events";
import { readFileSync } from "node:fs";
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

const SUPPRESSION_PATTERNS = [
  { id: "rust-allow-or-expect-attribute", pattern: /#\s*!?\s*\[\s*(?:cfg_attr\s*\([\s\S]{0,300}?,\s*)?(?:allow|expect)\s*\(/i },
  { id: "rust-command-line-allow", pattern: /(?:^|[\s'",\[])(?:-A\s+(?:warnings|[a-zA-Z_][\w-]*)|-A(?:warnings|unused|dead_code|[a-zA-Z][\w-]*_[\w-]+))(?:$|[\s'",\]])/m },
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
    .filter(([key, value]) => value !== undefined && value !== "" && FORBIDDEN_WARNING_ENV.some((pattern) => pattern.test(key)))
    .map(([key]) => key)
    .sort();
}

export function controlledChildEnvironment(environment = process.env) {
  const rejected = forbiddenWarningEnvironment(environment);
  if (rejected.length > 0) throw new Error(`warning-affecting environment is forbidden: ${rejected.join(", ")}`);
  return Object.fromEntries(Object.entries(environment).filter(([key]) => !FORBIDDEN_WARNING_ENV.some((pattern) => pattern.test(key))));
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

export function warningAffectingCargoConfigs(repoRoot, environment = process.env) {
  const findings = [];
  for (const candidate of cargoConfigCandidates(repoRoot, environment)) {
    try {
      const text = readFileSync(candidate, "utf8");
      const semanticFindings = inspectCargoConfigText(text);
      if (semanticFindings.length > 0 || /\brust(?:doc)?flags\s*=|\bcap-lints\b|(?:^|\s)-A(?:\s*|=)[\w-]+/im.test(text)) findings.push(candidate);
    } catch {
      // Missing config is the expected controlled case.
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

export function readCargoMetadata(repoRoot, configuration, environment = process.env) {
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

export async function runProcessWithTimeout(executable, args, options) {
  const maxOutputBytes = options.maxOutputBytes === undefined ? null : options.maxOutputBytes;
  if (maxOutputBytes !== null && (!Number.isSafeInteger(maxOutputBytes) || maxOutputBytes < 1)) {
    throw new Error("process output limit is invalid");
  }
  const child = spawn(executable, args, {
    cwd: options.cwd,
    env: options.env,
    shell: false,
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

export async function runCargoConfiguration(configuration, repoRoot, environment = process.env) {
  if (configuration.command?.executable !== "cargo") throw new Error(`configuration ${configuration.id} must use cargo`);
  if (!Number.isInteger(configuration.timeoutMs) || configuration.timeoutMs < 1) throw new Error(`configuration ${configuration.id} timeoutMs is invalid`);
  const args = [...configuration.command.args];
  if (!args.some((arg) => arg === "--message-format=json" || arg.startsWith("--message-format=json"))) {
    throw new Error(`configuration ${configuration.id} must request Cargo JSON diagnostics`);
  }
  assertControlledCargoInputs(configuration, repoRoot, environment);
  const metadata = readCargoMetadata(repoRoot, configuration, environment);
  const lines = [];
  const processResult = await runProcessWithTimeout("cargo", args, {
    cwd: repoRoot,
    env: controlledChildEnvironment(environment),
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
  return execFileSync("git", args, { cwd: repoRoot, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trimEnd();
}

function resolveCommit(repoRoot, ref) {
  if (!ref || /^0+$/.test(ref)) throw new Error(`invalid comparison ref: ${ref || "<missing>"}`);
  try {
    return git(repoRoot, ["rev-parse", "--verify", `${ref}^{commit}`]);
  } catch {
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
  } catch {
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
    execFileSync("git", ["merge-base", "--is-ancestor", base, head], {
      cwd: repoRoot,
      stdio: "ignore",
    });
  } catch {
    throw new Error(`explicit comparison base is not an ancestor of head: ${base} ${head}`);
  }
  return { base, head };
}

function parseNameOnlyZ(output) {
  return output.split("\0").filter(Boolean).map(normalizeComparisonPath);
}

export function collectModifiedFiles(repoRoot, comparison) {
  const files = new Set(parseNameOnlyZ(execFileSync("git", ["diff", "--name-only", "-z", `${comparison.base}...${comparison.head}`], { cwd: repoRoot, encoding: "utf8" })));
  for (const args of [["diff", "--name-only", "-z"], ["diff", "--cached", "--name-only", "-z"], ["ls-files", "--others", "--exclude-standard", "-z"]]) {
    for (const file of parseNameOnlyZ(execFileSync("git", args, { cwd: repoRoot, encoding: "utf8" }))) files.add(file);
  }
  return files;
}

export function detectSuppressionText(text) {
  return SUPPRESSION_PATTERNS.filter(({ pattern }) => pattern.test(text)).map(({ id }) => id);
}

function addedDiffText(repoRoot, args) {
  return git(repoRoot, args).split(/\r?\n/).filter((line) => line.startsWith("+") && !line.startsWith("+++" )).map((line) => line.slice(1)).join("\n");
}

export function findAddedSuppressions(repoRoot, comparison) {
  const chunks = [
    addedDiffText(repoRoot, ["diff", "--unified=0", "--no-ext-diff", `${comparison.base}...${comparison.head}`]),
    addedDiffText(repoRoot, ["diff", "--unified=0", "--no-ext-diff"]),
    addedDiffText(repoRoot, ["diff", "--cached", "--unified=0", "--no-ext-diff"]),
  ];
  const untracked = parseNameOnlyZ(execFileSync("git", ["ls-files", "--others", "--exclude-standard", "-z"], { cwd: repoRoot, encoding: "utf8" }));
  for (const relative of untracked) {
    try { chunks.push(readFileSync(path.join(repoRoot, relative), "utf8")); } catch { /* binary */ }
  }
  const semanticCargoConfigFindings = [];
  for (const relative of collectModifiedFiles(repoRoot, comparison)) {
    if (!/(?:^|\/)\.cargo\/config(?:\.toml)?$/i.test(slash(relative))) continue;
    try {
      semanticCargoConfigFindings.push(...inspectCargoConfigText(readFileSync(path.join(repoRoot, relative), "utf8")));
    } catch {
      // A deleted config cannot affect the current build.
    }
  }
  return [...new Set([...detectSuppressionText(chunks.join("\n")), ...semanticCargoConfigFindings])];
}

export function loadInventoryAtRef(repoRoot, ref, relativePath, allowMissing = false) {
  try {
    return JSON.parse(git(repoRoot, ["show", `${ref}:${slash(relativePath)}`]));
  } catch (error) {
    if (allowMissing) return null;
    throw new Error(`trusted prior inventory is missing or invalid at ${ref}:${relativePath}`, { cause: error });
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
  return execFileSync(executable, args, { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] }).trim().replace(/^v/, "");
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
