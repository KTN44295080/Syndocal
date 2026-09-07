import { spawnSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, readdirSync, readFileSync } from "node:fs";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { validateAsioPackagingBoundary } from "./check-release-metadata.mjs";
import {
  assertDirectoryTreeHasNoReparsePoints,
  assertExactWindowsTargetTriple,
  assertNoBlockedAsioPayload,
  assertSafeExternalDirectory,
  canonicalWindowsFfmpegRuntimeDllNames,
  loadWindowsRuntimeInventory,
  readVerifiedRegularFile,
  verifyPinnedRuntimeFile,
} from "./windows-runtime-inventory.mjs";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const workspaceRoot = resolve(scriptDir, "../..");

function isDescendant(root, candidate) {
  const child = relative(root, candidate);
  return child !== "" && !child.startsWith("..") && !child.includes(":");
}

export function validateFfmpegRuntimeDllNames(names, label = "FFmpeg runtime directory", inventory = loadWindowsRuntimeInventory()) {
  if (!Array.isArray(names)) throw new Error(label + " DLL inventory must be an array.");
  const expected = new Set(inventory.runtime_dlls.map((runtime) => runtime.filename.toLocaleLowerCase("en-US")));
  const seen = new Set();
  for (const rawName of names) {
    const name = String(rawName);
    const normalized = name.toLocaleLowerCase("en-US");
    assertNoBlockedAsioPayload(name, Buffer.alloc(0), label, inventory);
    if (seen.has(normalized)) throw new Error(label + " contains a duplicate runtime DLL: " + name);
    seen.add(normalized);
  }
  const unexpected = [...seen].filter((name) => !expected.has(name));
  const missing = [...expected].filter((name) => !seen.has(name));
  if (unexpected.length > 0 || missing.length > 0) {
    throw new Error(
      label
        + " DLL inventory is not the exact approved FFmpeg set (unexpected: "
        + (unexpected.join(", ") || "none")
        + "; missing: "
        + (missing.join(", ") || "none")
        + ").",
    );
  }
  return canonicalWindowsFfmpegRuntimeDllNames;
}

export function assertNoAsioRuntimeDllInDirectory(directory, label, inventory = loadWindowsRuntimeInventory()) {
  if (!existsSync(directory)) return;
  const releaseDirectory = assertSafeExternalDirectory(directory, label);
  for (const entry of readdirSync(releaseDirectory, { withFileTypes: true })) {
    if (!entry.name.toLocaleLowerCase("en-US").endsWith(".dll")) continue;
    if (!entry.isFile() || entry.isSymbolicLink()) {
      throw new Error(label + " contains a DLL directory, symbolic link, or reparse point: " + entry.name);
    }
    const record = readVerifiedRegularFile(join(releaseDirectory, entry.name), label + " DLL", {
      allowedRoots: [releaseDirectory],
    });
    assertNoBlockedAsioPayload(entry.name, record.bytes, label, inventory);
  }
}

export function validatePinnedFfmpegRuntimeDirectory(directory, label, inventory = loadWindowsRuntimeInventory()) {
  if (!existsSync(directory)) throw new Error(label + " is missing: " + directory);
  const releaseDirectory = assertSafeExternalDirectory(directory, label);
  const names = readdirSync(releaseDirectory, { withFileTypes: true })
    .filter((entry) => entry.name.toLocaleLowerCase("en-US").endsWith(".dll"))
    .map((entry) => {
      if (!entry.isFile() || entry.isSymbolicLink()) {
        throw new Error(label + " contains a DLL directory, symbolic link, or reparse point: " + entry.name);
      }
      return entry.name;
    });
  validateFfmpegRuntimeDllNames(names, label, inventory);
  for (const runtime of inventory.runtime_dlls) {
    verifyPinnedRuntimeFile(
      join(releaseDirectory, runtime.filename),
      runtime,
      label + " " + runtime.filename,
      { allowedRoots: [releaseDirectory] },
    );
  }
}

export function releaseDirectories(
  workspace = workspaceRoot,
  targetTriple = process.env.TAURI_ENV_TARGET_TRIPLE,
) {
  const resolvedWorkspace = assertSafeExternalDirectory(workspace, "Syndocal workspace");
  const targetRoot = resolve(resolvedWorkspace, "target");
  if (!isDescendant(resolvedWorkspace, targetRoot)) {
    throw new Error("Resolved Cargo target root escapes the workspace: " + targetRoot);
  }
  const triple = assertExactWindowsTargetTriple(targetTriple);
  const directories = [resolve(targetRoot, "release")];
  if (triple !== null) {
    const tripleDirectory = resolve(targetRoot, triple, "release");
    if (!isDescendant(targetRoot, tripleDirectory)) {
      throw new Error("Resolved target-triple release directory escapes workspace target: " + tripleDirectory);
    }
    directories.push(tripleDirectory);
  }
  return [...new Set(directories)];
}

export function assertNoNdiEnabledBundling(environment = process.env) {
  const indicators = [
    ["CARGO_FEATURE_NDI", environment.CARGO_FEATURE_NDI],
    ["SYNDOCAL_NDI_ENABLED", environment.SYNDOCAL_NDI_ENABLED],
    ["TAURI_BUNDLE_FEATURES", environment.TAURI_BUNDLE_FEATURES],
    ["SYNDOCAL_TAURI_FEATURES", environment.SYNDOCAL_TAURI_FEATURES],
  ];
  for (const [name, value] of indicators) {
    if (typeof value === "string" && (value === "1" || /\bndi\b/iu.test(value))) {
      throw new Error(
        "NDI-enabled bundling is fail-closed (" + name + "); a separately licensed NDI runtime overlay and its independent artifact proof are required.",
      );
    }
  }
  const allowedNdiNamedKeys = new Set([
    "WINDIR",
    "NDI_SDK_DIR",
    "NDI_RUNTIME_DIR_V2",
    "NDI_RUNTIME_DIR_V3",
    "NDI_RUNTIME_DIR_V4",
    "NDI_RUNTIME_DIR_V5",
    "NDI_RUNTIME_DIR_V6",
  ]);
  for (const key of Object.keys(environment)) {
    if (!/NDI/iu.test(key) || allowedNdiNamedKeys.has(key.toUpperCase())) continue;
    throw new Error(
      "Unknown NDI environment signal is fail-closed ("
        + key
        + "); the only sanctioned repository-owned feature signals are CARGO_FEATURE_NDI, SYNDOCAL_NDI_ENABLED, TAURI_BUNDLE_FEATURES, and SYNDOCAL_TAURI_FEATURES, and every one of them rejects NDI until a separately licensed overlay is proven.",
    );
  }
}

const approvedDefaultFeatureTokens = Object.freeze(new Set(["libav", "spout", "asio"]));
const knownCargoSubcommands = Object.freeze(new Set([
  "add",
  "bench",
  "build",
  "check",
  "clean",
  "clippy",
  "config",
  "doc",
  "fetch",
  "fix",
  "fmt",
  "help",
  "init",
  "install",
  "locate-project",
  "login",
  "logout",
  "metadata",
  "new",
  "owner",
  "package",
  "pkgid",
  "publish",
  "read-manifest",
  "remove",
  "report",
  "run",
  "rustc",
  "rustdoc",
  "search",
  "test",
  "tree",
  "uninstall",
  "update",
  "vendor",
  "verify-project",
  "version",
  "yank",
]));
// Fail-closed NDI feature-escape detection, with explicitly accepted boundaries:
// - Separated short forms ("-F ndi", "-f ndi", "-F=ndi", "-f=ndi") cover cargo's
//   "-F/--features" and the Tauri CLI's lowercase "-f" alias; they are detected
//   without requiring a tool-name context.
// - Attached short forms ("-Fndi", "-fndi") are classified only when the same
//   logical command names a cargo-family tool (cargo/rustc/rustdoc/tauri),
//   because shells own many unrelated attached flags (PowerShell -File,
//   -Filter, -Force, -First). Residual boundary: an attached short form in an
//   invocation that never names such a tool is not classified here; separated
//   forms, long forms, and the cargo-alias analyzer remain independent backstops.
// - A short-form payload containing "." or "/" is treated as a non-feature shell
//   operand (bash "[[ ! -f libavcodec.dylib ]]", PowerShell -Filter values);
//   cargo/tauri feature identifiers cannot contain either character. The raw NDI
//   token check runs before this exemption, so dotted operands naming NDI still
//   fail closed. Prose false positives outside these rules are deliberately
//   accepted over any detection weakening.
// - Wrapper/alias prefix stripping tolerates surrounding quotes and JSON/YAML
//   object-key openers so quoted indirection ("sh -c 'cargo alias'") still
//   reaches the unknown-cargo-subcommand check; the direct feature-option scan
//   over the full logical command remains the authoritative backstop either way.
const featureOptionScanPattern
  = /(?<![\w.,:;/-])--(?:[a-z][a-z-]*)?features?(?:[ \t]*=|[ \t]+|$)|(?<![\w.,:;/-])-[fF](?:[ \t]*=|[ \t]+|$)/giu;
const attachedShortFeatureScanPattern = /(?<![\w.\-/\\])-[fF](?=[A-Za-z0-9_])/gu;
const cargoToolchainContextPattern = /\b(?:cargo|rustc|rustdoc|tauri)\b/iu;
const ndiTokenPattern = /\bndi\b/iu;
const unresolvedValuePattern = /\$\{[^}]*\}|\$\(|\$[A-Za-z_(]|%[A-Za-z_][A-Za-z0-9_]*%|`[^\s`]/u;
const upperFeatureAssignmentPattern = /\b([A-Z][A-Za-z0-9_]{1,79})[ \t]*=[ \t]*(?:"([^"]*)"|'([^']*)'|([^\s;&|]+))/u;
const upperFeatureMappingPattern = /^[ \t]*(?:-[ \t]+)*([A-Z][A-Za-z0-9_]{1,79})[ \t]*:[ \t]*(.+?)[ \t]*$/u;
const commandBlockStartPattern = /^([ ]*)(?:[^\s:][^:]*:[ ]*)?([|>])[|>]?[+-]?[ ]*(?:#.*)?$/u;
const lineContinuationPattern = /[\\`^]$/u;

const ndiFeatureEscapeSurfaces = Object.freeze([
  ["app/package.json", "frontend package manifest"],
  [".github/workflows/cross-platform.yml", "cross-platform workflow"],
  ["app/src-tauri/tauri.conf.json", "Tauri base configuration"],
  ["app/src-tauri/tauri.windows.conf.json", "Windows Tauri overlay"],
  ["app/src-tauri/tauri.updater.conf.json", "Tauri updater overlay"],
  ["app/src-tauri/Cargo.toml", "Tauri Cargo manifest"],
  ["Cargo.toml", "workspace Cargo manifest"],
]);

const optionalNdiFeatureEscapeSurfaces = Object.freeze([
  ".cargo/config.toml",
  ".cargo/config",
]);

export function normalizeSurfaceCommands(text) {
  const decoded = String(text)
    .replace(/^\uFEFF/u, "")
    .replace(/\\r\\n|\\n|\\r/gu, "\n")
    .replace(/\r\n?/gu, "\n");
  const entries = [];
  let block = null;
  let scope = 0;
  for (const rawLine of decoded.split("\n")) {
    const line = rawLine.replace(/\t/u, "  ");
    if (block) {
      if (line.trim() === "") {
        if (block.style === "literal") continue;
        block.lines.push("");
        continue;
      }
      const indent = line.length - line.trimStart().length;
      if (indent <= block.parentIndent) {
        flushCommandBlock(entries, block);
        block = null;
      } else {
        block.lines.push(line.trim());
        continue;
      }
    }
    const started = commandBlockStartPattern.exec(line);
    if (started) {
      scope += 1;
      block = { style: started[2], parentIndent: started[1].length, scope, lines: [] };
      continue;
    }
    entries.push({ scope: 0, text: line });
  }
  if (block) flushCommandBlock(entries, block);
  return joinCommandContinuations(entries);
}

function flushCommandBlock(entries, block) {
  if (block.style === ">") {
    let current = [];
    for (const line of [...block.lines, ""]) {
      if (line === "") {
        if (current.length > 0) entries.push({ scope: block.scope, text: current.join(" ") });
        current = [];
      } else {
        current.push(line);
      }
    }
  } else {
    for (const line of block.lines) {
      if (line !== "") entries.push({ scope: block.scope, text: line });
    }
  }
  block.lines.length = 0;
}

function joinCommandContinuations(entries) {
  const joined = [];
  for (const entry of entries) {
    const previous = joined[joined.length - 1];
    if (
      previous
      && previous.scope === entry.scope
      && lineContinuationPattern.test(previous.text.trimEnd())
    ) {
      previous.text = previous.text.trimEnd().replace(lineContinuationPattern, "").trimEnd()
        + " "
        + entry.text.trim();
    } else {
      joined.push({ scope: entry.scope, text: entry.text });
    }
  }
  return joined;
}

function classifyFeatureSelection(optionText, payload, form) {
  const value = payload.trim();
  const snippet = (optionText + value).trim().slice(0, 240);
  if (/all-features/iu.test(optionText)) return { kind: "all-features-forbidden", snippet };
  if (ndiTokenPattern.test(value)) return { kind: "ndi-feature-escape", snippet };
  if (/no-default-features/iu.test(optionText)) return null;
  if (!value) return { kind: "dangling-feature-option", snippet: optionText.trim().slice(0, 240) };
  if (form === "short" && /[./]/u.test(value)) return null;
  if (unresolvedValuePattern.test(value)) return { kind: "unresolved-feature-interpolation", snippet };
  const tokens = value
    .split(/[\s,'"()[\]{}]+/u)
    .map((token) => token.trim())
    .filter((token) => token !== "" && !token.startsWith("-"));
  if (tokens.length === 0) return null;
  const unapproved = tokens.filter(
    (token) => !approvedDefaultFeatureTokens.has(token.toLocaleLowerCase("en-US")),
  );
  if (unapproved.length > 0) {
    return {
      kind: "unapproved-feature-selection",
      snippet,
      detail: unapproved.join(", "),
    };
  }
  return null;
}

function classifyFeatureValueDefinition(name, value) {
  const resolved = value.trim().replace(/^["']+|["']+$/gu, "");
  const snippet = (name + "=" + resolved).slice(0, 240);
  if (ndiTokenPattern.test(resolved)) return { kind: "ndi-feature-escape", snippet };
  if (unresolvedValuePattern.test(resolved)) return { kind: "unresolved-feature-interpolation", snippet };
  if (resolved === "" || /^(?:true|false|0|1|none|empty)$/iu.test(resolved)) return null;
  const tokens = resolved.split(/[\s,'"()[\]{}]+/u).filter((token) => token !== "");
  const unapproved = tokens.filter(
    (token) => !approvedDefaultFeatureTokens.has(token.toLocaleLowerCase("en-US")),
  );
  if (unapproved.length > 0) {
    return { kind: "unapproved-feature-selection", snippet, detail: unapproved.join(", ") };
  }
  return null;
}

const wrapperPrefixPattern
  = /^(?:\{[ \t]*(?:["'][A-Za-z_$][A-Za-z0-9_$. -]*["'][ \t]*:[ \t]*)?|["']{0,2}sudo\s+|["']{0,2}env\s+[A-Za-z_][A-Za-z0-9_]*=\S*\s+|["']{0,2}[A-Za-z0-9._/-]*(?:powershell|pwsh|cmd|sh|bash|zsh|dash)(?:\.exe)?\s+(?:["']?-["']?[A-Za-z]+\s+)*(?:["']?(?:\/c|\/k|-c|-Command)\s+)?|["']{0,2}[A-Za-z][A-Za-z0-9_-]*["']{0,2}:[ \t]*(?!\/))/iu;

function analyzeCargoAliasInvocation(trimmed) {
  let rest = trimmed;
  for (let depth = 0; depth < 6; depth += 1) {
    const stripped = wrapperPrefixPattern.exec(rest);
    if (!stripped) break;
    rest = rest.slice(stripped[0].length);
  }
  if (/^["']{0,2}cargo(?:\.exe)?\s/iu.test(rest)) {
    const words = rest.split(/\s+/u);
    const candidate = (words[1] ?? "").replace(/^["']+|["',]+$/gu, "");
    if (candidate !== "" && !knownCargoSubcommands.has(candidate.toLocaleLowerCase("en-US"))) {
      return {
        kind: "cargo-alias-indirection",
        snippet: trimmed.slice(0, 240),
        detail: candidate,
      };
    }
  }
  return null;
}

function analyzeLogicalCommand(line) {
  const trimmed = line.trim();
  if (trimmed === "") return null;
  const aliasFailure = analyzeCargoAliasInvocation(trimmed);
  if (aliasFailure) return aliasFailure;
  const toolchainContext = cargoToolchainContextPattern.test(trimmed);
  const scanners = [
    [featureOptionScanPattern, (matched) => (matched.startsWith("--") ? "" : "short")],
    ...(toolchainContext ? [[attachedShortFeatureScanPattern, () => "short"]] : []),
  ];
  for (const [pattern, formOf] of scanners) {
    for (const match of trimmed.matchAll(pattern)) {
      const failure = classifyFeatureSelection(
        match[0],
        trimmed.slice(match.index + match[0].length),
        formOf(match[0]),
      );
      if (failure) return failure;
    }
  }
  for (const candidate of String(line).split("\n")) {
    const assignment = upperFeatureAssignmentPattern.exec(candidate);
    if (assignment && /feature/iu.test(assignment[1])) {
      const failure = classifyFeatureValueDefinition(
        assignment[1],
        assignment[2] ?? assignment[3] ?? assignment[4] ?? "",
      );
      if (failure) return failure;
    }
    const mapping = upperFeatureMappingPattern.exec(candidate);
    if (mapping && /feature/iu.test(mapping[1])) {
      const failure = classifyFeatureValueDefinition(mapping[1], mapping[2]);
      if (failure) return failure;
    }
  }
  return null;
}

export function findNdiFeatureEscape(text) {
  for (const entry of normalizeSurfaceCommands(text)) {
    const failure = analyzeLogicalCommand(entry.text);
    if (failure) return failure;
  }
  return null;
}

function ndiFeatureEscapeErrorMessage(label, path, failure) {
  if (failure.kind === "ndi-feature-escape") {
    return "NDI-enabled feature escape is fail-closed in the "
      + label
      + " ("
      + path
      + "): '"
      + failure.snippet
      + "'. Default Tauri features must remain exactly libav + spout + asio; a separately licensed NDI runtime overlay and its independent artifact proof are required.";
  }
  if (failure.kind === "all-features-forbidden") {
    return "--all-features bundling is fail-closed in the "
      + label
      + " ("
      + path
      + "): '"
      + failure.snippet
      + "'. Selecting every feature implicitly enables NDI; default Tauri features must remain exactly libav + spout + asio and only the approved literal set may be selected.";
  }
  return "Unresolved NDI-feature indirection is fail-closed in the "
    + label
    + " ("
    + path
    + "): '"
    + failure.snippet
    + "' ("
    + failure.kind
    + (failure.detail ? ": " + failure.detail : "")
    + "). Default Tauri features must remain exactly libav + spout + asio; every packaging feature selection must resolve to the approved literal set.";
}

function assertSurfaceFreeOfNdiFeatureEscape(readManifest, path, label) {
  const failure = findNdiFeatureEscape(String(readManifest(path)));
  if (!failure) return;
  throw new Error(ndiFeatureEscapeErrorMessage(label, path, failure));
}

export function assertNoNdiFeatureEscape(
  readManifest = ((path) => readFileSync(resolve(workspaceRoot, path), "utf8")),
) {
  for (const [path, label] of ndiFeatureEscapeSurfaces) {
    assertSurfaceFreeOfNdiFeatureEscape(readManifest, path, label);
  }
  for (const path of optionalNdiFeatureEscapeSurfaces) {
    if (!existsSync(resolve(workspaceRoot, path))) continue;
    assertSurfaceFreeOfNdiFeatureEscape(readManifest, path, "Cargo config alias surface");
  }
}

function ancestorCommandLinePatternMatch(commandLine) {
  return typeof commandLine === "string" ? findNdiFeatureEscape(commandLine) : null;
}

export function parseProcessChainSnapshot(stdout) {
  let parsed;
  try {
    parsed = JSON.parse(String(stdout));
  } catch {
    return null;
  }
  const entries = Array.isArray(parsed) ? parsed : [parsed];
  const safeInteger = (value) => (typeof value === "number" && Number.isSafeInteger(value) ? value : Number.NaN);
  const processes = [];
  for (const entry of entries) {
    if (typeof entry !== "object" || entry === null) return null;
    processes.push({
      processId: safeInteger(entry.ProcessId),
      parentProcessId: safeInteger(entry.ParentProcessId),
      commandLine: typeof entry.CommandLine === "string" ? entry.CommandLine : "",
    });
  }
  return processes.every((process) => Number.isSafeInteger(process.processId)) ? processes : null;
}

export function collectAncestorProcessSnapshot(platform = process.platform) {
  if (platform !== "win32") return [];
  const result = spawnSync(
    "powershell.exe",
    [
      "-NoLogo",
      "-NoProfile",
      "-NonInteractive",
      "-Command",
      "$ErrorActionPreference='Stop'\n"
        + "$processes=@(Get-CimInstance Win32_Process -Property ProcessId,ParentProcessId,CommandLine)\n"
        + "[Console]::Out.Write((ConvertTo-Json -InputObject $processes -Compress -Depth 2))\n",
    ],
    { encoding: "utf8", windowsHide: true, timeout: 60_000, maxBuffer: 32 * 1024 * 1024 },
  );
  if (result.error || result.status !== 0 || typeof result.stdout !== "string") {
    throw new Error(
      "The bundling process-chain NDI audit failed closed; refusing to package without an authoritative ancestor command-line snapshot.",
    );
  }
  const snapshot = parseProcessChainSnapshot(result.stdout);
  if (!snapshot) {
    throw new Error("The bundling process-chain NDI audit returned an unparseable process snapshot.");
  }
  return snapshot;
}

export function assertNoNdiCliBypass(
  environment = process.env,
  {
    platform = process.platform,
    currentProcessId = process.pid,
    snapshot,
  } = {},
) {
  assertNoNdiFeatureEscape();
  if (platform !== "win32") return;
  const processes = Array.isArray(snapshot) ? snapshot : collectAncestorProcessSnapshot(platform);
  const byPid = new Map(processes.map((process) => [process.processId, process]));
  if (!byPid.has(currentProcessId)) {
    throw new Error(
      "The bundling process-chain NDI audit could not locate the packaging process itself; refusing to package on an incomplete snapshot.",
    );
  }
  const visited = new Set();
  let cursor = byPid.get(currentProcessId) ?? null;
  while (cursor && !visited.has(cursor.processId)) {
    visited.add(cursor.processId);
    const failure = ancestorCommandLinePatternMatch(cursor.commandLine);
    if (failure) {
      throw new Error(
        failure.kind === "ndi-feature-escape"
          ? "NDI-enabled bundling is fail-closed: an ancestor packaging process was invoked with an NDI feature flag ('"
              + failure.snippet
              + "'); a separately licensed NDI runtime overlay and its independent artifact proof are required."
          : failure.kind === "all-features-forbidden"
              ? "NDI-enabled bundling is fail-closed: an ancestor packaging process invoked '--all-features' ('"
                  + failure.snippet
                  + "'), which implicitly enables NDI; a separately licensed NDI runtime overlay and its independent artifact proof are required."
              : "NDI-enabled bundling is fail-closed: an ancestor packaging process invoked an unresolved cargo/tauri feature selection ('"
              + failure.snippet
              + "' ("
              + failure.kind
              + (failure.detail ? ": " + failure.detail : "")
              + ")); a separately licensed NDI runtime overlay and its independent artifact proof are required.",
      );
    }
    cursor = byPid.get(cursor.parentProcessId) ?? null;
  }
}

export function main(options = {}) {
  const {
    platform = process.platform,
    workspace = workspaceRoot,
    targetTriple = process.env.TAURI_ENV_TARGET_TRIPLE,
    environment = process.env,
    processSnapshot,
  } = options;
  const ffmpegDir = Object.hasOwn(options, "ffmpegDir") ? options.ffmpegDir : process.env.FFMPEG_DIR;
  validateAsioPackagingBoundary(undefined, {
    workspace,
    verifyWindowsRuntimeSources: false,
  });
  if (platform !== "win32") {
    console.log("No staged runtime libraries are required on this platform.");
    return { stagedDirectories: [] };
  }
  assertNoNdiEnabledBundling(environment);
  assertNoNdiCliBypass(environment, { platform, snapshot: processSnapshot });
  const inventory = loadWindowsRuntimeInventory({ workspace });
  const releaseDirs = releaseDirectories(workspace, targetTriple);
  for (const releaseDir of releaseDirs) {
    assertNoAsioRuntimeDllInDirectory(releaseDir, "Windows release directory", inventory);
  }
  if (ffmpegDir === undefined) {
    console.log("FFMPEG_DIR is not set; producing the SDK-independent bundle without creating runtime directories.");
    return { stagedDirectories: [] };
  }
  const sourceRoot = assertSafeExternalDirectory(ffmpegDir, "FFMPEG_DIR");
  const sourceDir = assertSafeExternalDirectory(join(sourceRoot, "bin"), "FFMPEG_DIR/bin");
  assertDirectoryTreeHasNoReparsePoints(sourceDir, "FFMPEG_DIR/bin");
  const runtimeNames = readdirSync(sourceDir, { withFileTypes: true })
    .filter((entry) => entry.name.toLocaleLowerCase("en-US").endsWith(".dll"))
    .map((entry) => {
      if (!entry.isFile() || entry.isSymbolicLink()) {
        throw new Error("FFMPEG_DIR/bin contains a DLL directory, symbolic link, or reparse point: " + entry.name);
      }
      return entry.name;
    });
  validateFfmpegRuntimeDllNames(runtimeNames, "FFMPEG_DIR/bin", inventory);
  for (const runtime of inventory.runtime_dlls) {
    verifyPinnedRuntimeFile(
      join(sourceDir, runtime.filename),
      runtime,
      "FFMPEG_DIR/bin " + runtime.filename,
      { allowedRoots: [sourceDir] },
    );
  }
  for (const releaseDir of releaseDirs) {
    if (existsSync(releaseDir)) {
      assertNoAsioRuntimeDllInDirectory(releaseDir, "Windows release directory", inventory);
    } else {
      mkdirSync(releaseDir, { recursive: true });
    }
    for (const runtime of inventory.runtime_dlls) {
      copyFileSync(join(sourceDir, runtime.filename), join(releaseDir, runtime.filename));
    }
    validatePinnedFfmpegRuntimeDirectory(releaseDir, "Windows staged runtime directory", inventory);
    console.log("Staged " + inventory.runtime_dlls.length + " pinned FFmpeg DLLs in " + releaseDir);
  }
  validateAsioPackagingBoundary(undefined, { workspace, verifyWindowsRuntimeSources: true });
  return { stagedDirectories: releaseDirs };
}

const selfTestHostileFixtures = Object.freeze([
  ["folded-yaml-separate-lines", ".github/workflows/cross-platform.yml", "run: >\n  cargo build\n  --features ndi\n", "ndi-feature-escape"],
  ["literal-yaml-separate-feature-line", ".github/workflows/cross-platform.yml", "run: |\n  pnpm tauri build\n  --features=ndi\n", "ndi-feature-escape"],
  ["literal-yaml-dangling-features", ".github/workflows/cross-platform.yml", "run: |\n  cargo build\n  --features\n  ndi\n", "dangling-feature-option"],
  ["json-embedded-newline-split", "app/package.json", "{\n  \"scripts\": {\n    \"bundle\": \"cargo build --features\\n  ndi\"\n  }\n}", "dangling-feature-option"],
  ["backslash-continuation", ".github/workflows/cross-platform.yml", "run: |\n  cargo build \\\n    --features ndi\n", "ndi-feature-escape"],
  ["powershell-backtick-continuation", ".github/workflows/cross-platform.yml", "run: |\n  cargo build `\n    --features ndi\n", "ndi-feature-escape"],
  ["cmd-caret-continuation", ".github/workflows/cross-platform.yml", "run: |\n  cargo build ^\n    --features ndi\n", "ndi-feature-escape"],
  ["equals-dollar-variable", ".github/workflows/cross-platform.yml", "run: cargo build --features=$SYNDOCAL_FEATURE_LIST -p syndocal\n", "unresolved-feature-interpolation"],
  ["percent-variable-wrapper", "app/package.json", "{\n  \"scripts\": {\n    \"bundle\": \"cmd /c cargo build --features=%NDI_FEATURES%\"\n  }\n}", "unresolved-feature-interpolation"],
  ["powershell-env-variable", "app/package.json", "{\n  \"scripts\": {\n    \"bundle\": \"pwsh -Command cargo build --features $env:TAURI_FEATURES\"\n  }\n}", "unresolved-feature-interpolation"],
  ["backtick-substitution", ".github/workflows/cross-platform.yml", "run: pnpm tauri build `\"--features `echo ndi`\"`\n", "ndi-feature-escape"],
  ["cargo-alias-invocation", ".github/workflows/cross-platform.yml", "run: cargo synbuild --release\n", "cargo-alias-indirection"],
  ["cargo-alias-definition-config", "Cargo.toml", "[alias]\nsynbuild = \"build --features ndi\"\n", "ndi-feature-escape"],
  ["cargo-alias-optional-config-detection", ".cargo/config.toml", "[alias]\nsynbuild = \"build --features ndi\"\n", "ndi-feature-escape"],
  ["mixed-case-upper", "app/package.json", "{\n  \"scripts\": {\n    \"bundle\": \"CARGO BUILD --FEATURES NDI\"\n  }\n}", "ndi-feature-escape"],
  ["mixed-case-equals-list", "app/package.json", "{\n  \"scripts\": {\n    \"bundle\": \"pnpm tauri build --Features=Libav,NDI\"\n  }\n}", "ndi-feature-escape"],
  ["separate-short-form", "app/package.json", "{\n  \"scripts\": {\n    \"bundle\": \"pnpm tauri build --ci -F ndi\"\n  }\n}", "ndi-feature-escape"],
  ["shell-mediated-env-assignment", ".github/workflows/cross-platform.yml", "run: |\n  SYNDOCAL_CARGO_FEATURES=\"libav ndi\"\n  pnpm tauri build $SYNDOCAL_CARGO_FEATURES\n", "ndi-feature-escape"],
  ["github-expression-mapping", ".github/workflows/cross-platform.yml", "env:\n  SYNDOCAL_FEATURES: ${{ inputs.features }}\nrun: cargo build\n", "unresolved-feature-interpolation"],
  ["wrapper-cmd-powershell", ".github/workflows/cross-platform.yml", "run: cmd /c \"powershell -Command cargo build --features ndi\"\n", "ndi-feature-escape"],
  ["unapproved-literal-selection", "app/src-tauri/tauri.conf.json", "{\n  \"build\": {\n    \"beforeBuildCommand\": \"cargo build --features overlay-experimental\"\n  }\n}", "unapproved-feature-selection"],
  ["attached-short-uppercase-ndi", "app/package.json", "{\n  \"scripts\": {\n    \"bundle\": \"pnpm tauri build --ci -Fndi\"\n  }\n}", "ndi-feature-escape"],
  ["attached-short-lowercase-ndi", "app/package.json", "{\n  \"scripts\": {\n    \"bundle\": \"cargo build -fndi --locked\"\n  }\n}", "ndi-feature-escape"],
  ["separated-short-lowercase-ndi", ".github/workflows/cross-platform.yml", "run: pnpm tauri build --ci -f ndi\n", "ndi-feature-escape"],
  ["all-features-with-trailing-flags", ".github/workflows/cross-platform.yml", "run: cargo build --all-features --locked --release\n", "all-features-forbidden"],
  ["all-features-bare-selection", ".github/workflows/cross-platform.yml", "run: cargo build --all-features\n", "all-features-forbidden"],
  ["selection-after-no-default-features", ".github/workflows/cross-platform.yml", "run: cargo build --no-default-features --features ndi\n", "ndi-feature-escape"],
  ["short-selection-after-no-default-features", ".github/workflows/cross-platform.yml", "run: cargo build --no-default-features -F ndi\n", "ndi-feature-escape"],
]);

const selfTestBenignCommands = Object.freeze([
  "run: pnpm --dir app tauri build --ci --bundles nsis,msi\n",
  "run: cargo build --release --locked\n",
  "run: cargo test -p video --features libav --locked -- --test-threads=1\n",
  "run: cargo check -p syndocal --features libav,spout --locked\n",
  "run: cargo fmt --all -- --check\n",
  "run: pnpm tauri build --features libav spout\n",
  "run: echo $NDI_SDK_DIR\n",
  "# NDIS adapters are unrelated; the word landing contains no standalone token\n",
  "run: |\n  bash app/scripts/bundle-macos-runtime.sh \\\n    target/release/bundle/macos/Syndocal.app \\\n    target/release/bundle/dmg\n",
  "run: cargo test --no-default-features -p video --locked\n",
  "env:\n  CARGO_FEATURES: libav,spout\nrun: cargo build\n",
  "run: >\n  sudo apt-get install -y --no-install-recommends\n  build-essential libudev-dev xvfb ffmpeg\n",
  "run: |\n  $sdkRoots = @(Get-ChildItem -LiteralPath $packageRoot -Directory)\n  \"FFMPEG_DIR=$sdk\" >> $env:GITHUB_ENV\n",
  "run: tar -xf \"$archive\" -C \"$RUNNER_TEMP\"\n",
  "{\n  \"scripts\": {\n    \"bundle\": \"pnpm tauri build --ci\\n--bundles app\"\n  }\n}",
  "run: cargo check --no-default-features --features libav --locked\n",
  "run: if [[ ! -f \"$prefix/lib/libavcodec.dylib\" ]]; then echo missing libavcodec; fi\n",
  "run: Stop-Process -Id $process.Id -Force -ErrorAction Stop\n",
  "run: Get-ChildItem . -Filter 'avcodec-*.dll' | Select-Object -First 1\n",
  "run: powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File scripts/check-native-workspace-operator.ps1\n",
]);

export function runPrepareReleaseRuntimeSelfTest() {
  let assertions = 0;
  const ok = (condition, label) => {
    if (!condition) throw new Error("prepare-release-runtime self-test failed: " + label);
    assertions += 1;
  };
  for (const [name, path, text, expectedKind] of selfTestHostileFixtures) {
    const failure = findNdiFeatureEscape(text);
    ok(failure !== null, "hostile fixture must be rejected: " + name);
    ok(failure.kind === expectedKind, "hostile fixture kind is " + expectedKind + ": " + name);
    if (expectedKind === "ndi-feature-escape"
      && (ndiFeatureEscapeSurfaces.some(([surfacePath]) => surfacePath === path)
        || existsSync(resolve(workspaceRoot, path)))) {
      let message = null;
      try {
        assertNoNdiFeatureEscape((probedPath) => (probedPath === path ? text : benignSurfaceFixture(probedPath)));
      } catch (error) {
        message = String(error.message);
      }
      ok(
        message !== null
          && new RegExp("NDI-enabled feature escape is fail-closed in the .*\\(" + path.replace(/[.*+?^${}()|[\]\\]/gu, "\\$&") + "\\)", "u").test(message),
        "hostile fixture error names the surface: " + name,
      );
    }
  }
  for (const [index, command] of selfTestBenignCommands.entries()) {
    ok(findNdiFeatureEscape(command) === null, "benign default command passes: #" + index);
  }
  ok(
    JSON.stringify(normalizeSurfaceCommands("run: >\n  cargo build\n  --features libav\n"))
      === JSON.stringify([{ scope: 1, text: "cargo build --features libav" }]),
    "folded YAML block joins into one logical command",
  );
  ok(
    normalizeSurfaceCommands("run: |\n  cargo test -p video --features libav \\\n    --locked\n").length === 1,
    "backslash continuation merges into one logical command",
  );
  ok(
    normalizeSurfaceCommands("a: 1\nb: |\n  x\n  y\nc: 2").some((entry) => entry.scope > 0 && entry.text === "x")
      && normalizeSurfaceCommands("a: 1\nb: |\n  x\n  y\nc: 2").some((entry) => entry.scope === 0 && entry.text === "c: 2"),
    "literal YAML block lines keep a distinct scope from surrounding keys",
  );
  let roundTripMessage = null;
  try {
    assertNoNdiFeatureEscape(benignSurfaceFixture);
  } catch (error) {
    roundTripMessage = String(error.message);
  }
  ok(roundTripMessage === null, "all repository-owned surfaces pass with benign synthetic content");
  let hostilePackageMessage = null;
  try {
    assertNoNdiFeatureEscape((path) => (path === "app/package.json"
      ? "{\n  \"scripts\": {\n    \"bundle\": \"pnpm tauri build --features ndi\"\n  }\n}"
      : benignSurfaceFixture(path)));
  } catch (error) {
    hostilePackageMessage = String(error.message);
  }
  ok(
    hostilePackageMessage !== null
      && /NDI-enabled feature escape is fail-closed in the frontend package manifest \(app\/package\.json\)/u.test(hostilePackageMessage),
    "surface-level rejection reports label and path",
  );
  console.log("prepare-release-runtime NDI feature-escape self-test passed: " + assertions + " assertions");
}

function benignSurfaceFixture(path) {
  switch (path) {
    case "app/package.json":
      return "{\n  \"scripts\": {\n    \"tauri\": \"node scripts/run-tauri.mjs\",\n    \"prepare:runtime-libs\": \"pnpm run check:asio-packaging && node scripts/prepare-release-runtime.mjs\"\n  }\n}";
    case ".github/workflows/cross-platform.yml":
      return "jobs:\n  test:\n    steps:\n      - name: Bundle Windows installers\n        run: pnpm --dir app tauri build --ci --bundles nsis,msi\n      - name: Test Rust workspace\n        run: cargo test --workspace --locked\n";
    case "app/src-tauri/tauri.conf.json":
      return "{\n  \"$schema\": \"https://schema.tauri.app/config/2\",\n  \"build\": {\n    \"beforeBuildCommand\": \"pnpm build\"\n  }\n}";
    case "app/src-tauri/tauri.windows.conf.json":
      return "{\n  \"bundle\": {\n    \"resources\": {}\n  }\n}";
    case "app/src-tauri/tauri.updater.conf.json":
      return "{\n  \"plugins\": {\n    \"updater\": {\n      \"pubkey\": \"\",\n      \"endpoints\": []\n    }\n  }\n}";
    case "app/src-tauri/Cargo.toml":
      return "[features]\ndefault = [\"libav\", \"spout\", \"asio\"]\n";
    case "Cargo.toml":
      return "[workspace]\nresolver = \"2\"\n[workspace.dependencies]\ntauri = { version = \"=2.5.1\", features = [\"unstable\"] }\ngrafton-ndi = { version = \"=0.11.0\", default-features = false }\n";
    default:
      throw new Error("Unexpected surface requested by the hermetic self-test: " + path);
  }
}

if (resolve(process.argv[1] ?? "") === fileURLToPath(import.meta.url)) {
  if (process.argv.slice(2).includes("--self-test")) {
    runPrepareReleaseRuntimeSelfTest();
  } else {
    main();
  }
}
