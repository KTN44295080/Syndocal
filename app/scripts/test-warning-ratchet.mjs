import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdtempSync, mkdirSync, readFileSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  aggregateDiagnostics,
  auditZeroWarningPromotion,
  auditOutputMarkerRebaseline,
  artifactIdentity,
  compareArtifactCoverage,
  compareOutputMarkerCoverage,
  compareDiagnostics,
  collectModifiedFiles,
  controlledChildEnvironment,
  controlledGenericCommandEnvironment,
  CURRENT_FILE_CONTENT_MAX_BYTES,
  detectToolchain,
  detectSuppressionText,
  diagnosticIdentityHash,
  findAddedSuppressions,
  findBaselineLaundering,
  forbiddenGenericCommandEnvironment,
  forbiddenWarningEnvironment,
  GIT_OUTPUT_LIMIT_ERROR_CODE,
  GIT_OUTPUT_MAX_BUFFER,
  inspectCargoManifestLintText,
  loadInventory,
  loadInventoryAtRef,
  normalizeRepoPath,
  parseCargoJsonLines,
  resolveTrustedComparison,
  resolveExplicitAncestorComparison,
  runCargoConfiguration,
  runGenericConfiguration,
  runProcessWithTimeout,
  runWarningConfiguration,
  validateInventory,
  validateExpectedOutputMarkers,
  validateInventorySchema,
  warningAffectingCargoConfigs,
  warningShapedStderr,
} from "./warning-ratchet-lib.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "../..");
const fixtureRoot = path.join(scriptDir, "fixtures/warning-ratchet");
const lines = readFileSync(path.join(fixtureRoot, "cargo-warning.jsonl"), "utf8").trim().split(/\r?\n/);
const metadata = JSON.parse(readFileSync(path.join(fixtureRoot, "cargo-metadata.json"), "utf8"));
const context = {
  repoRoot,
  packageNames: new Map(metadata.packages.map((pkg) => [pkg.id, pkg.name])),
  firstPartyPackageIds: new Set(metadata.workspaceMembers),
};
const configuration = {
  platform: "windows-x86_64-msvc",
  profile: "dev-all-targets",
  features: ["workspace-default"],
};

const parsed = parseCargoJsonLines(lines, configuration, context);
assert.equal(parsed.invalidJsonLines.length, 0);
assert.deepEqual(parsed.buildFinished, [true]);
assert.equal(parsed.artifacts.length, 1);
assert.equal(parsed.diagnostics.length, 2);
assert.deepEqual(parsed.diagnostics.map((item) => item.classification).sort(), ["first-party", "third-party"]);

const firstParty = parsed.diagnostics.find((item) => item.classification === "first-party");
const thirdParty = parsed.diagnostics.find((item) => item.classification === "third-party");
assert.ok(firstParty && thirdParty);
const annotated = { ...firstParty, owner: "engine", reason: "pre-W0", removalCheckpoint: "W2-engine" };
const moved = { ...annotated, line: 900, column: 9 };
assert.equal(diagnosticIdentityHash(annotated), diagnosticIdentityHash(moved));

const baseConfiguration = {
  ...configuration,
  diagnostics: [annotated],
  externalWarningAllows: [],
};
assert.equal(compareDiagnostics(baseConfiguration, [moved]).ok, true);
assert.equal(compareDiagnostics(baseConfiguration, [annotated], new Set([annotated.file])).ok, false);
assert.equal(compareDiagnostics(baseConfiguration, [thirdParty]).ok, false);
const allowedExternal = {
  ...baseConfiguration,
  diagnostics: [annotated, thirdParty],
  externalWarningAllows: [{
    identity: thirdParty.identity,
    owner: "dependency-owner",
    reason: "upstream warning",
    upstream: "https://example.invalid/upstream/1",
    expiry: "2099-01-01T00:00:00Z",
    maxOccurrences: 1,
  }],
};
assert.equal(compareDiagnostics(allowedExternal, [thirdParty]).ok, true);

const downgraded = { ...annotated, classification: "third-party" };
downgraded.identity = diagnosticIdentityHash(downgraded);
assert.ok(compareDiagnostics(baseConfiguration, [downgraded]).failures.some((failure) => failure.includes("classification downgrade")));

const customBuildMessage = JSON.stringify({
  reason: "compiler-message",
  package_id: metadata.workspaceMembers[0],
  target: { name: "build-script-build", kind: ["custom-build"] },
  message: { level: "warning", message: "generated warning", code: null, spans: [] },
});
const customBuild = parseCargoJsonLines([customBuildMessage], configuration, context).diagnostics[0];
assert.equal(customBuild.classification, "first-party");
assert.equal(customBuild.origin, "custom-build");

const excludedId = "path+file:///C:/fixture/KDMX/tools/asio-bridge#syndocal-asio-bridge@1.0.0";
const excludedContext = {
  ...context,
  packageNames: new Map([...context.packageNames, [excludedId, "syndocal-asio-bridge"]]),
  firstPartyPackageIds: new Set([...context.firstPartyPackageIds, excludedId]),
};
const excludedWarning = parseCargoJsonLines([JSON.stringify({
  reason: "compiler-message",
  package_id: excludedId,
  target: { name: "syndocal_asio_bridge", kind: ["lib"] },
  message: { level: "warning", message: "bridge warning", code: { code: "dead_code" }, spans: [] },
})], configuration, excludedContext).diagnostics[0];
assert.equal(excludedWarning.classification, "first-party");

const grown = { ...annotated, occurrences: 2 };
assert.equal(compareDiagnostics(baseConfiguration, [grown]).ok, false);
assert.equal(aggregateDiagnostics([annotated, annotated])[0].occurrences, 2);

const artifact = parsed.artifacts[0];
assert.equal(compareArtifactCoverage([artifact], [artifact]).ok, true);
assert.equal(compareArtifactCoverage([], []).ok, false);
assert.equal(compareArtifactCoverage([artifact], []).missing.length, 1);
assert.equal(artifactIdentity(artifact), artifactIdentity({ ...artifact, targetKinds: [...artifact.targetKinds].reverse() }));

const encodedName = ["CARGO", "ENCODED", "RUSTFLAGS"].join("_");
const encodedRustdocName = ["CARGO", "ENCODED", "RUSTDOCFLAGS"].join("_");
for (const environment of [
  { RUSTFLAGS: "-A" + "warnings" },
  { RUSTDOCFLAGS: "-A" + "dead_code" },
  { [encodedName]: "-A\u001fdead_code" },
  { [encodedRustdocName]: "-A\u001fdead_code" },
  { CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTFLAGS: "-A" + "warnings" },
  { CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTDOC: "rustdoc-proxy" },
  { RUSTDOC: "rustdoc-proxy" },
  { RUSTC: "rustc-proxy" },
  { RUSTC_WRAPPER: "sccache" },
  { RUSTC_WORKSPACE_WRAPPER: "workspace-rustc-proxy" },
  { CARGO_BUILD_RUSTC: "rustc-proxy" },
  { CARGO_BUILD_RUSTC_WRAPPER: "sccache" },
  { CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER: "workspace-rustc-proxy" },
  { CARGO_BUILD_RUSTDOC: "rustdoc-proxy" },
  { CARGO_BUILD_JOBS: "1" },
  { CARGO_HOME: "C:/tainted" },
]) {
  assert.ok(forbiddenWarningEnvironment(environment).length > 0);
  assert.throws(() => controlledChildEnvironment(environment), /forbidden/);
}
assert.deepEqual(forbiddenWarningEnvironment({ RUST_LOG: "warn", PATH: "x" }), []);

const suppressionSamples = [
  "#[" + "allow(\n dead_code,\n unused_imports\n)]",
  "#[" + "expect(unused_variables, reason = \"temporary\")]",
  ["RUST", "FLAGS"].join("") + "='-A " + "unused_imports'",
  "--cap-lints " + "allow",
  "rust" + "flags = [\"-A\", \"dead_code\"]",
  encodedName + "=-A\u001fwarnings",
  "chunkSize" + "WarningLimit: 9999",
];
suppressionSamples.push(
  ["#[", "/* hidden */", "allow", "(dead_code)]"].join(""),
  ["#[", "allow", "/* hidden */", "(dead_code)]"].join(""),
  ["#[r#", "allow", "(dead_code)]"].join(""),
  [
    "#[cfg_attr(all(",
    Array.from({ length: 40 }, (_, index) => `feature = \"feature_${index}\"`).join(", "),
    "), ",
    "allow",
    "(dead_code))]",
  ].join(""),
  [
    "macro_rules! with_lint { ($level:ident, $item:item) => { #[$level(dead_code)] $item }; }\nwith_lint!(",
    "allow",
    ", fn hidden() {})",
  ].join(""),
  ["with_lint!(r#", "expect", ", fn hidden() {})"].join(""),
);
const compilerSelectionKeys = [
  ["rust", "c"].join(""),
  ["rustc", "-wrapper"].join(""),
  ["rustc", "-workspace-wrapper"].join(""),
];
const compilerSelectionConfigForms = [
  (key) => ["[bui", "ld]\n\"", key, "\" = \"compiler-proxy\"\n"].join(""),
  (key) => ["build.", key, " = \"compiler-proxy\"\n"].join(""),
  (key) => ["build = { \"", key, "\" = \"compiler-proxy\" }\n"].join(""),
];
const warningFlagKeys = [
  ["rust", "flags"].join(""),
  ["rustdoc", "flags"].join(""),
];
const semanticWarningFlagConfigs = [];
for (const key of warningFlagKeys) {
  semanticWarningFlagConfigs.push(["[bui", "ld]\n\"", key, "\" = []\n"].join(""));
  semanticWarningFlagConfigs.push(["build.", key, " = []\n"].join(""));
  semanticWarningFlagConfigs.push(["build = { \"", key, "\" = [] }\n"].join(""));
  semanticWarningFlagConfigs.push(["[target.'cfg(windows)']\n\"", key, "\" = [\"--allow\", \"warnings\"]\n"].join(""));
  semanticWarningFlagConfigs.push(["target.\"x86_64-pc-windows-msvc\".", key, " = [\"--allow\", \"warnings\"]\n"].join(""));
  semanticWarningFlagConfigs.push(["target = { \"cfg(unix)\" = { \"", key, "\" = [\"--allow\", \"warnings\"] } }\n"].join(""));
}
const semanticBuildControlConfigs = [];
for (const key of [["rust", "doc"].join(""), "target"]) {
  semanticBuildControlConfigs.push(["[bui", "ld]\n\"", key, "\" = \"controlled-value\"\n"].join(""));
  semanticBuildControlConfigs.push(["build.", key, " = \"controlled-value\"\n"].join(""));
  semanticBuildControlConfigs.push(["build = { \"", key, "\" = \"controlled-value\" }\n"].join(""));
}
const semanticProfileConfigs = [
  ["[pro", "file.dev]\nopt-level = 0\n"].join(""),
  ["profile.release", " = { lto = false }\n"].join(""),
];
const cargoConfigEnvironmentKeys = [
  ["RUST", "FLAGS"].join(""),
  ["RUSTDOC", "FLAGS"].join(""),
  encodedName,
  encodedRustdocName,
  "RUSTDOC",
  "RUSTC",
  "RUSTC_WRAPPER",
  "RUSTC_WORKSPACE_WRAPPER",
  "CARGO_BUILD_RUSTC",
  "CARGO_BUILD_RUSTC_WRAPPER",
  "CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER",
  "CARGO_BUILD_RUSTDOC",
  "CARGO_BUILD_JOBS",
  "CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTDOCFLAGS",
  "CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_RUSTDOC",
  "CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER",
  "CARGO_PROFILE_RELEASE_LTO",
  "RUSTC_BOOTSTRAP",
  "CARGO_HOME",
];
const semanticEnvironmentConfigs = cargoConfigEnvironmentKeys.map((key, index) => [
  "[env]\n\"",
  index % 2 === 0 ? key.toLocaleLowerCase("en-US") : key,
  index % 2 === 0 ? "\" = \"injected\"\n" : "\" = { value = \"injected\", force = true }\n",
].join(""));
for (const key of compilerSelectionKeys) {
  suppressionSamples.push(["[bui", "ld]\n", key, " = 'compiler-proxy'"].join(""));
  suppressionSamples.push(["--con", "fig build.", key, "='compiler-proxy'"].join(""));
}
for (const sample of suppressionSamples) assert.ok(detectSuppressionText(sample).length > 0, sample);
assert.ok(detectSuppressionText("rustc -A " + "warnings source.rs").includes("rust-command-line-allow"));
assert.ok(detectSuppressionText("rustc -A " + "dead_code source.rs").includes("rust-command-line-allow"));
assert.deepEqual(detectSuppressionText("git add -A\ngit commit -m seed\n"), []);
assert.deepEqual(detectSuppressionText("const rustc = selectCompiler();\nconst wrapperName = 'rustc-wrapper';"), []);
assert.deepEqual(detectSuppressionText(["const text = \"", "#[", "allow", "(dead_code)]\";"].join("")), []);

for (const manifestText of [
  "[lints.rust]\ndead_code = \"allow\"\n",
  "lints.rust.dead_code = \"allow\"\n",
  "lints = { rust = { dead_code = { level = \"allow\", priority = -1 } } }\n",
  "[workspace.lints.clippy]\nall = { level = \"allow\", priority = -1 }\n",
]) {
  assert.ok(inspectCargoManifestLintText(manifestText).includes("cargo-lint-level-allow"));
}
assert.ok(inspectCargoManifestLintText("[lints]\nworkspace = true\n").includes("cargo-lints-workspace-inheritance"));
assert.deepEqual(inspectCargoManifestLintText("[lints]\nworkspace = false\n"), []);

assert.equal(normalizeRepoPath("\\\\?\\C:\\USERS\\KOUTY\\REPO\\Src\\LIB.rs", "C:\\Users\\kouty\\repo"), "src/lib.rs");
assert.equal(normalizeRepoPath("\\\\?\\UNC\\server\\share\\Repo\\SRC\\lib.rs", "\\\\server\\share\\repo"), "src/lib.rs");

assert.equal(warningShapedStderr("warning: hidden warning\n"), true);
assert.equal(warningShapedStderr("Finished dev profile"), false);
const timeoutResult = await runProcessWithTimeout(process.execPath, ["-e", "setTimeout(() => {}, 5000)"], {
  cwd: repoRoot,
  env: process.env,
  timeoutMs: 25,
  forwardStderr: false,
});
assert.equal(timeoutResult.timedOut, true);

const genericConfiguration = (code, markers = ["generic-marker"], timeoutMs = 5_000) => ({
  id: "generic-warning-fixture",
  command: { executable: "pnpm", args: ["exec", "node", "-e", code] },
  timeoutMs,
  expectedOutputMarkers: markers,
});
const genericSuccess = await runWarningConfiguration(
  genericConfiguration("process.stdout.write('generic-marker')"),
  repoRoot,
);
assert.equal(genericSuccess.exitCode, 0);
assert.equal(genericSuccess.timedOut, false);
assert.equal(genericSuccess.warningShaped, false);
assert.equal(genericSuccess.markerCoverage.ok, true);
assert.deepEqual(compareOutputMarkerCoverage(["literal"], "literal output"), {
  ok: true,
  missing: [],
  expected: ["literal"],
});
const genericStdoutWarning = await runGenericConfiguration(
  genericConfiguration("process.stdout.write('generic-marker\\nwarning: synthetic')"),
  repoRoot,
);
assert.equal(genericStdoutWarning.warningShaped, true);
const genericStderrWarning = await runGenericConfiguration(
  genericConfiguration("process.stdout.write('generic-marker'); process.stderr.write('WARN synthetic')"),
  repoRoot,
);
assert.equal(genericStderrWarning.warningShaped, true);
const genericViteWarning = await runGenericConfiguration(
  genericConfiguration("process.stdout.write('generic-marker\\n(!) Vite synthetic warning')"),
  repoRoot,
);
assert.equal(genericViteWarning.warningShaped, true);
const genericNonzero = await runGenericConfiguration(
  genericConfiguration("process.stdout.write('generic-marker'); process.exitCode = 7"),
  repoRoot,
);
assert.equal(genericNonzero.exitCode, 7);
const genericTimeout = await runGenericConfiguration(
  genericConfiguration("setTimeout(() => process.stdout.write('generic-marker'), 10_000)", ["generic-marker"], 1_000),
  repoRoot,
);
assert.equal(genericTimeout.timedOut, true);
const genericMissingMarker = await runGenericConfiguration(
  genericConfiguration("process.stdout.write('generic-marker')", ["missing-marker"]),
  repoRoot,
);
assert.equal(genericMissingMarker.markerCoverage.ok, false);
assert.deepEqual(genericMissingMarker.markerCoverage.missing, ["missing-marker"]);
const forwardedEnvironment = { ...process.env, WARNING_RATCHET_TEST_ENV: "forwarded" };
const forwardedEnvironmentResult = await runGenericConfiguration({
  ...genericConfiguration("process.stdout.write(process.env.WARNING_RATCHET_TEST_ENV)"),
  requiredEnvironment: ["WARNING_RATCHET_TEST_ENV"],
}, repoRoot, forwardedEnvironment);
assert.equal(forwardedEnvironmentResult.stdout, "forwarded");
await assert.rejects(
  runGenericConfiguration({
    ...genericConfiguration("process.stdout.write('generic-marker')"),
    requiredEnvironment: [],
  }, repoRoot),
  /requiredEnvironment must be a non-empty array/,
);
const missingNativeEnvironment = { ...process.env };
delete missingNativeEnvironment.FFMPEG_DIR;
delete missingNativeEnvironment.LIBCLANG_PATH;
await assert.rejects(
  runGenericConfiguration({
    ...genericConfiguration("process.stdout.write('generic-marker')"),
    requiredEnvironment: ["FFMPEG_DIR", "LIBCLANG_PATH"],
  }, repoRoot, missingNativeEnvironment),
  /required generic environment is missing: FFMPEG_DIR, LIBCLANG_PATH/,
);
const outputCap = await runProcessWithTimeout(
  process.execPath,
  ["-e", "process.stdout.write('x'.repeat(4096))"],
  { cwd: repoRoot, env: process.env, timeoutMs: 1_000, maxOutputBytes: 64, forwardStderr: false },
);
assert.equal(outputCap.outputLimitExceeded, true);
const stderrOutputCap = await runProcessWithTimeout(
  process.execPath,
  ["-e", "process.stderr.write('x'.repeat(4096))"],
  { cwd: repoRoot, env: process.env, timeoutMs: 1_000, maxOutputBytes: 64, forwardStderr: false },
);
assert.equal(stderrOutputCap.outputLimitExceeded, true);
await assert.rejects(
  runGenericConfiguration({ ...genericConfiguration("process.stdout.write('generic-marker')"), command: { executable: "node", args: [] } }, repoRoot),
  /unsupported command executable/,
);
await assert.rejects(
  runGenericConfiguration({ ...genericConfiguration("process.stdout.write('generic-marker')"), expectedOutputMarkers: [] }, repoRoot),
  /expectedOutputMarkers must be a non-empty array/,
);
assert.ok(forbiddenGenericCommandEnvironment({ NODE_OPTIONS: "--require=taint" }).includes("NODE_OPTIONS"));
assert.deepEqual(forbiddenGenericCommandEnvironment({ PNPM_CONFIG_FOO: "taint" }), []);
assert.throws(() => controlledGenericCommandEnvironment({ NODE_OPTIONS: "--require=taint" }), /generic command environment is forbidden/);
assert.equal(Object.hasOwn(controlledGenericCommandEnvironment({ PNPM_HOME: "C:/tainted" }), "PNPM_HOME"), false);
assert.equal(Object.hasOwn(controlledGenericCommandEnvironment({ PNPM_CONFIG_FOO: "taint" }), "PNPM_CONFIG_FOO"), false);
assert.equal(Object.hasOwn(controlledGenericCommandEnvironment({ npm_config_script_shell: "taint" }), "npm_config_script_shell"), false);
assert.throws(() => validateExpectedOutputMarkers([]), /non-empty array/);
assert.throws(() => validateExpectedOutputMarkers(["duplicate", "duplicate"]), /unique literal strings/);

assert.throws(() => resolveTrustedComparison(repoRoot, "0".repeat(40), "HEAD"), /invalid comparison ref/);
assert.throws(() => resolveTrustedComparison(repoRoot, "definitely-missing", "HEAD"), /unresolvable/);
assert.throws(() => resolveTrustedComparison(repoRoot, "HEAD", "HEAD"), /must differ/);
const trusted = resolveTrustedComparison(repoRoot, null, "HEAD");
assert.notEqual(trusted.base, trusted.head);
assert.throws(() => loadInventoryAtRef(repoRoot, trusted.base, "qa/warnings/does-not-exist.json"), /trusted prior inventory is missing/);

const gitFixtureRoot = mkdtempSync(path.join(tmpdir(), "syndocal-warning-git-"));
try {
  const fixtureGit = (args) => execFileSync("git", args, { cwd: gitFixtureRoot, encoding: "utf8" }).trim();
  fixtureGit(["init", "--initial-branch=main"]);
  fixtureGit(["config", "core.autocrlf", "false"]);
  fixtureGit(["config", "user.name", "Warning Ratchet Test"]);
  fixtureGit(["config", "user.email", "warning-ratchet@example.invalid"]);
  writeFileSync(path.join(gitFixtureRoot, "unstaged.txt"), "baseline\n");
  fixtureGit(["add", "unstaged.txt"]);
  fixtureGit(["commit", "-m", "baseline"]);
  const fixtureBase = fixtureGit(["rev-parse", "HEAD"]);
  writeFileSync(path.join(gitFixtureRoot, "committed.txt"), "committed\n");
  fixtureGit(["add", "committed.txt"]);
  fixtureGit(["commit", "-m", "head"]);
  const fixtureHead = fixtureGit(["rev-parse", "HEAD"]);
  writeFileSync(path.join(gitFixtureRoot, "staged.txt"), "staged\n");
  fixtureGit(["add", "staged.txt"]);
  writeFileSync(path.join(gitFixtureRoot, "unstaged.txt"), "changed\n");
  writeFileSync(path.join(gitFixtureRoot, "untracked.txt"), "untracked\n");
  assert.deepEqual(
    [...collectModifiedFiles(gitFixtureRoot, { base: fixtureBase, head: fixtureHead })].sort(),
    ["committed.txt", "staged.txt", "unstaged.txt", "untracked.txt"],
  );
  mkdirSync(path.join(gitFixtureRoot, ".cargo"));
  const changedConfigPath = path.join(gitFixtureRoot, ".cargo", "config.toml");
  writeFileSync(changedConfigPath, compilerSelectionConfigForms[2](compilerSelectionKeys[1]));
  assert.ok(findAddedSuppressions(gitFixtureRoot, { base: fixtureBase, head: fixtureHead }).includes("cargo-build-rustc-wrapper"));
  writeFileSync(changedConfigPath, semanticWarningFlagConfigs.at(-1));
  assert.ok(findAddedSuppressions(gitFixtureRoot, { base: fixtureBase, head: fixtureHead }).includes("cargo-target-rustdocflags"));
  writeFileSync(changedConfigPath, semanticEnvironmentConfigs[0]);
  assert.ok(findAddedSuppressions(gitFixtureRoot, { base: fixtureBase, head: fixtureHead }).some((finding) => finding.startsWith("cargo-env-")));
  writeFileSync(changedConfigPath, semanticProfileConfigs[0]);
  assert.ok(findAddedSuppressions(gitFixtureRoot, { base: fixtureBase, head: fixtureHead }).includes("cargo-profile-config"));
  writeFileSync(changedConfigPath, "[build\ninvalid = true\n");
  assert.ok(findAddedSuppressions(gitFixtureRoot, { base: fixtureBase, head: fixtureHead }).includes("cargo-config-malformed"));
  writeFileSync(changedConfigPath, "[build]\njobs = 2\n[env]\nrustc_note = \"documentation only\"\n");
  assert.deepEqual(findAddedSuppressions(gitFixtureRoot, { base: fixtureBase, head: fixtureHead }), []);
} finally {
  rmSync(gitFixtureRoot, { recursive: true, force: true });
}

const cargoConfigSnapshotFixtureRoot = mkdtempSync(path.join(tmpdir(), "syndocal-warning-config-snapshots-"));
try {
  const fixtureGit = (args) => execFileSync("git", args, { cwd: cargoConfigSnapshotFixtureRoot, encoding: "utf8" }).trim();
  fixtureGit(["init", "--initial-branch=main"]);
  fixtureGit(["config", "core.autocrlf", "false"]);
  fixtureGit(["config", "user.name", "Warning Ratchet Cargo Config Snapshot Test"]);
  fixtureGit(["config", "user.email", "warning-ratchet-cargo-config-snapshot@example.invalid"]);
  const configPath = path.join(cargoConfigSnapshotFixtureRoot, ".cargo", "config.toml");
  const benignConfig = "[build]\njobs = 2\n";
  const dangerousInlineConfig = "build = { \"rustc-wrapper\" = \"proxy\" }\n";
  assert.deepEqual(detectSuppressionText(dangerousInlineConfig), []);
  mkdirSync(path.dirname(configPath));
  writeFileSync(configPath, benignConfig);
  fixtureGit(["add", ".cargo/config.toml"]);
  fixtureGit(["commit", "-m", "benign base"]);
  const benignBase = fixtureGit(["rev-parse", "HEAD"]);

  writeFileSync(configPath, dangerousInlineConfig);
  fixtureGit(["add", ".cargo/config.toml"]);
  fixtureGit(["commit", "-m", "dangerous committed config"]);
  const dangerousHead = fixtureGit(["rev-parse", "HEAD"]);
  writeFileSync(configPath, benignConfig);
  assert.ok(findAddedSuppressions(cargoConfigSnapshotFixtureRoot, { base: benignBase, head: dangerousHead }).includes("cargo-build-rustc-wrapper"));
  rmSync(configPath);
  assert.ok(findAddedSuppressions(cargoConfigSnapshotFixtureRoot, { base: benignBase, head: dangerousHead }).includes("cargo-build-rustc-wrapper"));

  writeFileSync(configPath, benignConfig);
  fixtureGit(["add", ".cargo/config.toml"]);
  fixtureGit(["commit", "-m", "benign staged base"]);
  const benignHead = fixtureGit(["rev-parse", "HEAD"]);
  writeFileSync(configPath, dangerousInlineConfig);
  fixtureGit(["add", ".cargo/config.toml"]);
  writeFileSync(configPath, benignConfig);
  assert.ok(findAddedSuppressions(cargoConfigSnapshotFixtureRoot, { base: benignHead, head: benignHead }).includes("cargo-build-rustc-wrapper"));

  fixtureGit(["add", ".cargo/config.toml"]);
  writeFileSync(configPath, dangerousInlineConfig);
  assert.ok(findAddedSuppressions(cargoConfigSnapshotFixtureRoot, { base: benignHead, head: benignHead }).includes("cargo-build-rustc-wrapper"));
  writeFileSync(configPath, benignConfig);

  const untrackedConfigPath = path.join(cargoConfigSnapshotFixtureRoot, "nested", ".cargo", "config.toml");
  mkdirSync(path.dirname(untrackedConfigPath), { recursive: true });
  writeFileSync(untrackedConfigPath, dangerousInlineConfig);
  assert.ok(findAddedSuppressions(cargoConfigSnapshotFixtureRoot, { base: benignHead, head: benignHead }).includes("cargo-build-rustc-wrapper"));
} finally {
  rmSync(cargoConfigSnapshotFixtureRoot, { recursive: true, force: true });
}

const rustSnapshotFixtureRoot = mkdtempSync(path.join(tmpdir(), "syndocal-warning-rust-snapshots-"));
try {
  const fixtureGit = (args) => execFileSync("git", args, { cwd: rustSnapshotFixtureRoot, encoding: "utf8" }).trim();
  fixtureGit(["init", "--initial-branch=main"]);
  fixtureGit(["config", "core.autocrlf", "false"]);
  fixtureGit(["config", "user.name", "Warning Ratchet Rust Snapshot Test"]);
  fixtureGit(["config", "user.email", "warning-ratchet-rust-snapshot@example.invalid"]);
  const sourcePath = path.join(rustSnapshotFixtureRoot, "source.rs");
  const rustSource = (level, body = "fn hidden() {}") => `#[\n${level}(dead_code)\n]\n${body}\n`;
  writeFileSync(sourcePath, rustSource("warn"));
  fixtureGit(["add", "source.rs"]);
  fixtureGit(["commit", "-m", "warn attribute base"]);
  const warnBase = fixtureGit(["rev-parse", "HEAD"]);
  writeFileSync(sourcePath, rustSource("allow"));
  fixtureGit(["add", "source.rs"]);
  fixtureGit(["commit", "-m", "central line becomes allow"]);
  const allowHead = fixtureGit(["rev-parse", "HEAD"]);
  assert.ok(findAddedSuppressions(rustSnapshotFixtureRoot, { base: warnBase, head: allowHead }).includes("rust-allow-or-expect-attribute"));

  writeFileSync(sourcePath, rustSource("warn"));
  fixtureGit(["add", "source.rs"]);
  fixtureGit(["commit", "-m", "warn staged base"]);
  const warnHead = fixtureGit(["rev-parse", "HEAD"]);
  writeFileSync(sourcePath, rustSource("allow/* hidden */"));
  fixtureGit(["add", "source.rs"]);
  writeFileSync(sourcePath, rustSource("warn"));
  assert.ok(findAddedSuppressions(rustSnapshotFixtureRoot, { base: warnHead, head: warnHead }).includes("rust-allow-or-expect-attribute"));

  fixtureGit(["add", "source.rs"]);
  writeFileSync(sourcePath, rustSource("allow"));
  assert.ok(findAddedSuppressions(rustSnapshotFixtureRoot, { base: warnHead, head: warnHead }).includes("rust-allow-or-expect-attribute"));
  writeFileSync(sourcePath, rustSource("warn"));

  writeFileSync(path.join(rustSnapshotFixtureRoot, "untracked.rs"), rustSource("allow"));
  assert.ok(findAddedSuppressions(rustSnapshotFixtureRoot, { base: warnHead, head: warnHead }).includes("rust-allow-or-expect-attribute"));
  rmSync(path.join(rustSnapshotFixtureRoot, "untracked.rs"));

  writeFileSync(sourcePath, rustSource("allow"));
  fixtureGit(["add", "source.rs"]);
  fixtureGit(["commit", "-m", "existing allow attribute"]);
  const existingAllowHead = fixtureGit(["rev-parse", "HEAD"]);
  writeFileSync(sourcePath, rustSource("allow", "fn hidden() { let _unchanged = true; }"));
  assert.ok(!findAddedSuppressions(rustSnapshotFixtureRoot, { base: existingAllowHead, head: existingAllowHead }).includes("rust-allow-or-expect-attribute"));
} finally {
  rmSync(rustSnapshotFixtureRoot, { recursive: true, force: true });
}

const cargoLintSnapshotFixtureRoot = mkdtempSync(path.join(tmpdir(), "syndocal-warning-cargo-lint-snapshots-"));
try {
  const fixtureGit = (args) => execFileSync("git", args, { cwd: cargoLintSnapshotFixtureRoot, encoding: "utf8" }).trim();
  fixtureGit(["init", "--initial-branch=main"]);
  fixtureGit(["config", "core.autocrlf", "false"]);
  fixtureGit(["config", "user.name", "Warning Ratchet Cargo Lint Snapshot Test"]);
  fixtureGit(["config", "user.email", "warning-ratchet-cargo-lint-snapshot@example.invalid"]);
  const manifestPath = path.join(cargoLintSnapshotFixtureRoot, "Cargo.toml");
  const benignManifest = "[package]\nname = \"lint-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n";
  const dangerousManifest = `${benignManifest}[lints.rust]\ndead_code = \"allow\"\n`;
  writeFileSync(manifestPath, benignManifest);
  fixtureGit(["add", "Cargo.toml"]);
  fixtureGit(["commit", "-m", "benign manifest base"]);
  const benignBase = fixtureGit(["rev-parse", "HEAD"]);

  writeFileSync(manifestPath, dangerousManifest);
  fixtureGit(["add", "Cargo.toml"]);
  fixtureGit(["commit", "-m", "dangerous committed lint level"]);
  const dangerousHead = fixtureGit(["rev-parse", "HEAD"]);
  writeFileSync(manifestPath, benignManifest);
  assert.ok(findAddedSuppressions(cargoLintSnapshotFixtureRoot, { base: benignBase, head: dangerousHead }).includes("cargo-lint-level-allow"));

  fixtureGit(["add", "Cargo.toml"]);
  fixtureGit(["commit", "-m", "benign manifest head"]);
  const benignHead = fixtureGit(["rev-parse", "HEAD"]);
  writeFileSync(manifestPath, dangerousManifest);
  fixtureGit(["add", "Cargo.toml"]);
  writeFileSync(manifestPath, benignManifest);
  assert.ok(findAddedSuppressions(cargoLintSnapshotFixtureRoot, { base: benignHead, head: benignHead }).includes("cargo-lint-level-allow"));

  fixtureGit(["add", "Cargo.toml"]);
  writeFileSync(manifestPath, dangerousManifest);
  assert.ok(findAddedSuppressions(cargoLintSnapshotFixtureRoot, { base: benignHead, head: benignHead }).includes("cargo-lint-level-allow"));
  writeFileSync(manifestPath, benignManifest);

  const untrackedManifestPath = path.join(cargoLintSnapshotFixtureRoot, "untracked", "Cargo.toml");
  mkdirSync(path.dirname(untrackedManifestPath), { recursive: true });
  writeFileSync(untrackedManifestPath, dangerousManifest.replace("lint-fixture", "untracked-lint-fixture"));
  assert.ok(findAddedSuppressions(cargoLintSnapshotFixtureRoot, { base: benignHead, head: benignHead }).includes("cargo-lint-level-allow"));
  rmSync(path.dirname(untrackedManifestPath), { recursive: true, force: true });

  const memberManifestPath = path.join(cargoLintSnapshotFixtureRoot, "member", "Cargo.toml");
  const workspaceManifest = "[workspace]\nmembers = [\"member\"]\nresolver = \"2\"\n[workspace.lints.rust]\ndead_code = \"allow\"\n";
  const memberManifest = "[package]\nname = \"member\"\nversion = \"0.1.0\"\nedition = \"2021\"\n";
  mkdirSync(path.dirname(memberManifestPath), { recursive: true });
  writeFileSync(manifestPath, workspaceManifest);
  writeFileSync(memberManifestPath, memberManifest);
  fixtureGit(["add", "Cargo.toml", "member/Cargo.toml"]);
  fixtureGit(["commit", "-m", "workspace lint base without inheritance"]);
  const workspaceBase = fixtureGit(["rev-parse", "HEAD"]);
  writeFileSync(memberManifestPath, `${memberManifest}[lints]\nworkspace = true\n`);
  fixtureGit(["add", "member/Cargo.toml"]);
  fixtureGit(["commit", "-m", "activate workspace lint inheritance"]);
  const workspaceHead = fixtureGit(["rev-parse", "HEAD"]);
  assert.ok(findAddedSuppressions(cargoLintSnapshotFixtureRoot, { base: workspaceBase, head: workspaceHead }).includes("cargo-lints-workspace-inheritance"));
} finally {
  rmSync(cargoLintSnapshotFixtureRoot, { recursive: true, force: true });
}

const gitAttributeDiffFixtureRoot = mkdtempSync(path.join(tmpdir(), "syndocal-warning-git-attributes-"));
try {
  const fixtureGit = (args) => execFileSync("git", args, { cwd: gitAttributeDiffFixtureRoot, encoding: "utf8" }).trim();
  fixtureGit(["init", "--initial-branch=main"]);
  fixtureGit(["config", "core.autocrlf", "false"]);
  fixtureGit(["config", "user.name", "Warning Ratchet Git Attributes Test"]);
  fixtureGit(["config", "user.email", "warning-ratchet-git-attributes@example.invalid"]);
  fixtureGit(["commit", "--allow-empty", "-m", "base"]);
  const fixtureBase = fixtureGit(["rev-parse", "HEAD"]);
  writeFileSync(path.join(gitAttributeDiffFixtureRoot, ".gitattributes"), "*.rs -diff\n");
  writeFileSync(path.join(gitAttributeDiffFixtureRoot, "source.rs"), "#[" + "allow(dead_code)]\n");
  fixtureGit(["add", ".gitattributes", "source.rs"]);
  fixtureGit(["commit", "-m", "attribute-hidden suppression"]);
  const fixtureHead = fixtureGit(["rev-parse", "HEAD"]);
  assert.ok(findAddedSuppressions(gitAttributeDiffFixtureRoot, { base: fixtureBase, head: fixtureHead }).includes("rust-allow-or-expect-attribute"));

  const macroSource = [
    "macro_rules! with_lint { ($level:ident, $item:item) => { #[$level(dead_code)] $item }; }\nwith_lint!(",
    "allow",
    ", fn hidden() {})\n",
  ].join("");
  writeFileSync(path.join(gitAttributeDiffFixtureRoot, "macro.rs"), macroSource);
  fixtureGit(["add", "macro.rs"]);
  fixtureGit(["commit", "-m", "literal lint macro invocation"]);
  const macroHead = fixtureGit(["rev-parse", "HEAD"]);
  assert.ok(findAddedSuppressions(gitAttributeDiffFixtureRoot, { base: fixtureHead, head: macroHead }).includes("rust-allow-or-expect-attribute"));
} finally {
  rmSync(gitAttributeDiffFixtureRoot, { recursive: true, force: true });
}

const gitRenameDiffFixtureRoot = mkdtempSync(path.join(tmpdir(), "syndocal-warning-git-rename-"));
try {
  const fixtureGit = (args) => execFileSync("git", args, { cwd: gitRenameDiffFixtureRoot, encoding: "utf8" }).trim();
  fixtureGit(["init", "--initial-branch=main"]);
  fixtureGit(["config", "core.autocrlf", "false"]);
  fixtureGit(["config", "user.name", "Warning Ratchet Git Rename Test"]);
  fixtureGit(["config", "user.email", "warning-ratchet-git-rename@example.invalid"]);
  writeFileSync(
    path.join(gitRenameDiffFixtureRoot, "Cargo.toml"),
    "[package]\nname = \"rename-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n[lib]\npath = \"stub.rs\"\n",
  );
  writeFileSync(path.join(gitRenameDiffFixtureRoot, "stub.rs"), "pub fn visible() {}\n");
  writeFileSync(path.join(gitRenameDiffFixtureRoot, "dormant.rs"), "#[" + "allow(dead_code)]\nfn hidden() {}\n");
  fixtureGit(["add", "Cargo.toml", "stub.rs", "dormant.rs"]);
  fixtureGit(["commit", "-m", "dormant suppression"]);
  const fixtureBase = fixtureGit(["rev-parse", "HEAD"]);
  fixtureGit(["mv", "dormant.rs", "active.rs"]);
  writeFileSync(
    path.join(gitRenameDiffFixtureRoot, "Cargo.toml"),
    "[package]\nname = \"rename-fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\n[lib]\npath = \"active.rs\"\n",
  );
  fixtureGit(["add", "Cargo.toml"]);
  fixtureGit(["commit", "-m", "activate renamed suppression"]);
  const fixtureHead = fixtureGit(["rev-parse", "HEAD"]);
  assert.ok(findAddedSuppressions(gitRenameDiffFixtureRoot, { base: fixtureBase, head: fixtureHead }).includes("rust-allow-or-expect-attribute"));
} finally {
  rmSync(gitRenameDiffFixtureRoot, { recursive: true, force: true });
}

const gitOutputLimitFixtureRoot = mkdtempSync(path.join(tmpdir(), "syndocal-warning-git-output-limit-"));
try {
  const fixtureGit = (args) => execFileSync("git", args, { cwd: gitOutputLimitFixtureRoot, encoding: "utf8" }).trim();
  fixtureGit(["init", "--initial-branch=main"]);
  fixtureGit(["config", "core.autocrlf", "false"]);
  fixtureGit(["config", "user.name", "Warning Ratchet Git Output Limit Test"]);
  fixtureGit(["config", "user.email", "warning-ratchet-git-output-limit@example.invalid"]);
  writeFileSync(path.join(gitOutputLimitFixtureRoot, "tracked.txt"), "baseline\n");
  fixtureGit(["add", "tracked.txt"]);
  fixtureGit(["commit", "-m", "baseline"]);
  writeFileSync(path.join(gitOutputLimitFixtureRoot, "tracked.txt"), Buffer.alloc(GIT_OUTPUT_MAX_BUFFER + 1, "x"));
  assert.throws(
    () => findAddedSuppressions(gitOutputLimitFixtureRoot, { base: fixtureGit(["rev-parse", "HEAD"]), head: fixtureGit(["rev-parse", "HEAD"]) }),
    (error) => {
      assert.equal(error?.code, GIT_OUTPUT_LIMIT_ERROR_CODE);
      assert.match(error?.message ?? "", new RegExp(`Git output exceeded the warning-ratchet limit of ${GIT_OUTPUT_MAX_BUFFER} bytes`));
      return true;
    },
  );

  writeFileSync(path.join(gitOutputLimitFixtureRoot, "tracked.txt"), "baseline\n");
  writeFileSync(
    path.join(gitOutputLimitFixtureRoot, "oversized-untracked.txt"),
    Buffer.alloc(CURRENT_FILE_CONTENT_MAX_BYTES + 1, "u"),
  );
  assert.throws(
    () => findAddedSuppressions(gitOutputLimitFixtureRoot, { base: fixtureGit(["rev-parse", "HEAD"]), head: fixtureGit(["rev-parse", "HEAD"]) }),
    new RegExp(`Untracked content aggregate exceeded the warning-ratchet limit of ${CURRENT_FILE_CONTENT_MAX_BYTES} bytes while reading: oversized-untracked\\.txt`),
  );
  rmSync(path.join(gitOutputLimitFixtureRoot, "oversized-untracked.txt"));

  const firstUntrackedBytes = Math.floor(CURRENT_FILE_CONTENT_MAX_BYTES / 2);
  writeFileSync(path.join(gitOutputLimitFixtureRoot, "aggregate-a.txt"), Buffer.alloc(firstUntrackedBytes, "a"));
  writeFileSync(
    path.join(gitOutputLimitFixtureRoot, "aggregate-b.txt"),
    Buffer.alloc(CURRENT_FILE_CONTENT_MAX_BYTES - firstUntrackedBytes + 1, "b"),
  );
  assert.throws(
    () => findAddedSuppressions(gitOutputLimitFixtureRoot, { base: fixtureGit(["rev-parse", "HEAD"]), head: fixtureGit(["rev-parse", "HEAD"]) }),
    new RegExp(`Untracked content aggregate exceeded the warning-ratchet limit of ${CURRENT_FILE_CONTENT_MAX_BYTES} bytes while reading: aggregate-b\\.txt`),
  );
  rmSync(path.join(gitOutputLimitFixtureRoot, "aggregate-a.txt"));
  rmSync(path.join(gitOutputLimitFixtureRoot, "aggregate-b.txt"));

  const oversizedInventoryPath = path.join(gitOutputLimitFixtureRoot, "oversized-inventory.json");
  writeFileSync(oversizedInventoryPath, JSON.stringify({ padding: "i".repeat(GIT_OUTPUT_MAX_BUFFER) }));
  fixtureGit(["add", "oversized-inventory.json"]);
  fixtureGit(["commit", "-m", "oversized valid inventory"]);
  const oversizedInventoryHead = fixtureGit(["rev-parse", "HEAD"]);
  assert.throws(
    () => loadInventoryAtRef(gitOutputLimitFixtureRoot, oversizedInventoryHead, "oversized-inventory.json", true),
    (error) => {
      assert.equal(error?.code, GIT_OUTPUT_LIMIT_ERROR_CODE);
      assert.match(error?.message ?? "", new RegExp(`Git output exceeded the warning-ratchet limit of ${GIT_OUTPUT_MAX_BUFFER} bytes`));
      return true;
    },
  );
  assert.equal(loadInventoryAtRef(gitOutputLimitFixtureRoot, oversizedInventoryHead, "missing-inventory.json", true), null);
  writeFileSync(path.join(gitOutputLimitFixtureRoot, "invalid-inventory.json"), "{ invalid json\n");
  fixtureGit(["add", "invalid-inventory.json"]);
  fixtureGit(["commit", "-m", "invalid inventory"]);
  const invalidInventoryHead = fixtureGit(["rev-parse", "HEAD"]);
  assert.throws(
    () => loadInventoryAtRef(gitOutputLimitFixtureRoot, invalidInventoryHead, "invalid-inventory.json", true),
    /trusted prior inventory is invalid/,
  );

  const cargoConfigPath = path.join(gitOutputLimitFixtureRoot, ".cargo", "config.toml");
  mkdirSync(path.dirname(cargoConfigPath));
  let cargoConfigPadding = "# filler\n".repeat(Math.ceil(CURRENT_FILE_CONTENT_MAX_BYTES / 9));
  writeFileSync(cargoConfigPath, `# baseline\n${cargoConfigPadding}`);
  fixtureGit(["add", ".cargo/config.toml"]);
  fixtureGit(["commit", "-m", "oversized Cargo config baseline"]);
  const cargoConfigHead = fixtureGit(["rev-parse", "HEAD"]);
  writeFileSync(cargoConfigPath, `# changed!\n${cargoConfigPadding}`);
  cargoConfigPadding = "";
  assert.throws(
    () => findAddedSuppressions(gitOutputLimitFixtureRoot, { base: cargoConfigHead, head: cargoConfigHead }),
    new RegExp(`Modified worktree Cargo config content aggregate exceeded the warning-ratchet limit of ${CURRENT_FILE_CONTENT_MAX_BYTES} bytes while reading: \\.cargo/config\\.toml`),
  );
} finally {
  rmSync(gitOutputLimitFixtureRoot, { recursive: true, force: true });
}

assert.deepEqual(findBaselineLaundering({ id: "x" }, { id: "x" }), []);
assert.ok(findBaselineLaundering({ id: "x", status: "enforced" }, { id: "x", status: "pending" }).length > 0);
const immutableCommandConfiguration = {
  id: "immutable-command",
  command: { executable: "pnpm", args: ["run", "build"] },
  expectedOutputMarkers: ["build complete"],
};
assert.ok(findBaselineLaundering(
  immutableCommandConfiguration,
  { ...immutableCommandConfiguration, command: { executable: "pnpm", args: ["run", "other"] } },
).length > 0);
assert.ok(findBaselineLaundering(
  immutableCommandConfiguration,
  { ...immutableCommandConfiguration, expectedOutputMarkers: ["other marker"] },
).length > 0);

const inventory = loadInventory(path.join(repoRoot, "qa/warnings/warning-inventory.json"));
const schema = loadInventory(path.join(repoRoot, "qa/warnings/warning-inventory.schema.json"));
const spoutConfiguration = inventory.configurations.find((candidate) => candidate.id === "windows-syndocal-spout");
const genericInventoryConfiguration = {
  id: "generic-schema-fixture",
  platform: "windows-x86_64-msvc",
  profile: "frontend-fixture",
  features: [],
  command: { executable: "pnpm", args: ["run", "build"] },
  timeoutMs: 1_000,
  firstPartyManifests: [],
  status: "enforced",
  evidence: {
    commit: "a".repeat(40),
    capturedAt: "2026-08-21T00:00:00Z",
    command: "pnpm run build",
    toolchain: { cargo: "1", rustc: "1", node: "1", pnpm: "1" },
  },
  expectedOutputMarkers: ["build complete"],
  expectedArtifacts: [],
  diagnostics: [],
  externalWarningAllows: [],
};
const genericInventory = structuredClone(inventory);
genericInventory.configurations.push(genericInventoryConfiguration);
assert.deepEqual(validateInventorySchema(genericInventory, schema), []);
assert.deepEqual(validateInventory(genericInventory, schema), []);
const genericWithoutMarkers = structuredClone(genericInventory);
delete genericWithoutMarkers.configurations.at(-1).expectedOutputMarkers;
assert.ok(validateInventorySchema(genericWithoutMarkers, schema).some((error) => error.includes("expectedOutputMarkers")));
const invalidMarkerSchema = structuredClone(genericInventory);
invalidMarkerSchema.configurations.at(-1).expectedOutputMarkers = ["duplicate", "duplicate"];
assert.ok(validateInventorySchema(invalidMarkerSchema, schema).some((error) => error.includes("duplicate") || error.includes("uniqueItems")));
const unknownExecutableInventory = structuredClone(genericInventory);
unknownExecutableInventory.configurations.at(-1).command.executable = "node";
assert.ok(validateInventorySchema(unknownExecutableInventory, schema).some((error) => error.includes("allowed values") || error.includes("enum")));
assert.deepEqual(validateInventorySchema(inventory, schema), []);
const promotionFixtureRoot = mkdtempSync(path.join(tmpdir(), "syndocal-warning-promotion-"));
try {
  const fixtureGit = (args) => execFileSync("git", args, { cwd: promotionFixtureRoot, encoding: "utf8" }).trim();
  fixtureGit(["init", "--initial-branch=main"]);
  fixtureGit(["config", "user.name", "Warning Ratchet Promotion Test"]);
  fixtureGit(["config", "user.email", "warning-ratchet-promotion@example.invalid"]);
  mkdirSync(path.join(promotionFixtureRoot, "qa/warnings"), { recursive: true });
  const promotionBaseInventory = structuredClone(inventory);
  const promotionCommand = { executable: "pnpm", args: ["--dir", path.join(repoRoot, "app"), "exec", "node", "-e", "process.stdout.write('promotion-marker')"] };
  promotionBaseInventory.configurations.push({
    id: "generic-promotion-fixture",
    platform: "windows-x86_64-msvc",
    profile: "promotion-fixture",
    features: [],
    command: promotionCommand,
    timeoutMs: 1_000,
    firstPartyManifests: [],
    status: "pending",
    blockingReason: "test fixture",
    nextAction: "test fixture",
    expectedArtifacts: [],
    diagnostics: [],
    externalWarningAllows: [],
  }, {
    id: "generic-linux-promotion-fixture",
    platform: "linux-x86_64",
    profile: "promotion-fixture-linux",
    features: [],
    command: promotionCommand,
    timeoutMs: 1_000,
    firstPartyManifests: [],
    status: "pending",
    blockingReason: "test fixture",
    nextAction: "test fixture",
    expectedArtifacts: [],
    diagnostics: [],
    externalWarningAllows: [],
  });
  const promotionInventoryFile = path.join(promotionFixtureRoot, "qa/warnings/warning-inventory.json");
  writeFileSync(promotionInventoryFile, `${JSON.stringify(promotionBaseInventory, null, 2)}\n`);
  fixtureGit(["add", "qa/warnings/warning-inventory.json"]);
  fixtureGit(["commit", "-m", "trusted pending inventory"]);
  const promotionBase = fixtureGit(["rev-parse", "HEAD"]);
  const promotionHeadInventory = structuredClone(promotionBaseInventory);
  const promotionHeadConfiguration = promotionHeadInventory.configurations.at(-1);
  promotionHeadConfiguration.status = "enforced";
  delete promotionHeadConfiguration.blockingReason;
  delete promotionHeadConfiguration.nextAction;
  promotionHeadConfiguration.expectedOutputMarkers = ["promotion-marker"];
  promotionHeadConfiguration.evidence = {
    commit: promotionBase,
    capturedAt: "2026-08-21T00:00:00Z",
    command: "pnpm exec node -e promotion-marker",
    toolchain: { cargo: "test", rustc: "test", node: "test", pnpm: "test" },
  };
  writeFileSync(promotionInventoryFile, `${JSON.stringify(promotionHeadInventory, null, 2)}\n`);
  fixtureGit(["add", "qa/warnings/warning-inventory.json"]);
  fixtureGit(["commit", "-m", "promote zero warning fixture"]);
  const promotionHead = fixtureGit(["rev-parse", "HEAD"]);
  const promotionToolchain = detectToolchain();
  for (const promotionConfiguration of promotionHeadInventory.configurations.slice(-2)) {
    promotionConfiguration.status = "enforced";
    delete promotionConfiguration.blockingReason;
    delete promotionConfiguration.nextAction;
    promotionConfiguration.expectedOutputMarkers = ["promotion-marker"];
    promotionConfiguration.evidence = {
      commit: promotionBase,
      capturedAt: "2026-08-21T00:00:00Z",
      command: "pnpm --dir app exec node -e promotion-marker",
      toolchain: promotionToolchain,
    };
  }
  promotionHeadInventory.configurations.find(
    (candidate) => candidate.id === "generic-promotion-fixture",
  ).requiredEnvironment = ["PROMOTION_SDK"];
  writeFileSync(promotionInventoryFile, `${JSON.stringify(promotionHeadInventory, null, 2)}\n`);
  fixtureGit(["add", "qa/warnings/warning-inventory.json"]);
  fixtureGit(["commit", "-m", "promote zero warning fixture with platform pair"]);
  const promotionHeadWithPair = fixtureGit(["rev-parse", "HEAD"]);
  const promotionResult = await auditZeroWarningPromotion({
    repoRoot: promotionFixtureRoot,
    baseRef: promotionBase,
    headRef: promotionHeadWithPair,
    configurationId: "generic-promotion-fixture",
    schema,
    environment: { ...process.env, PROMOTION_SDK: "fixture-sdk" },
  });
  assert.deepEqual(promotionResult.promotedIds, ["generic-linux-promotion-fixture", "generic-promotion-fixture"]);
  await assert.rejects(
    auditZeroWarningPromotion({ repoRoot: promotionFixtureRoot, baseRef: null, headRef: promotionHeadWithPair, configurationId: "generic-promotion-fixture", schema }),
    /requires explicit base and head refs/,
  );
  await assert.rejects(
    auditZeroWarningPromotion({ repoRoot: promotionFixtureRoot, baseRef: promotionBase, headRef: promotionHeadWithPair, schema }),
    /requires an explicit configuration id/,
  );
  await assert.rejects(
    auditZeroWarningPromotion({
      repoRoot: promotionFixtureRoot,
      baseRef: promotionBase,
      headRef: promotionHeadWithPair,
      configurationId: "windows-default-all-targets",
      schema,
    }),
    /not a pending->enforced transition/,
  );
  await assert.rejects(
    auditZeroWarningPromotion({
      repoRoot: promotionFixtureRoot,
      baseRef: promotionBase,
      headRef: promotionHeadWithPair,
      configurationId: "generic-linux-promotion-fixture",
      schema,
    }),
    /targets linux-x86_64, but this host is windows-x86_64-msvc/,
  );
  const outsidePromotionFile = path.join(promotionFixtureRoot, "outside.txt");
  writeFileSync(outsidePromotionFile, "not part of an inventory-only promotion\n");
  await assert.rejects(
    auditZeroWarningPromotion({
      repoRoot: promotionFixtureRoot,
      baseRef: promotionBase,
      headRef: promotionHeadWithPair,
      configurationId: "generic-promotion-fixture",
      schema,
    }),
    /outside inventory\/warning gate scope/,
  );
  rmSync(outsidePromotionFile, { force: true });

  const badEvidenceInventory = structuredClone(promotionHeadInventory);
  badEvidenceInventory.configurations.find((candidate) => candidate.id === "generic-promotion-fixture").evidence.commit = "b".repeat(40);
  writeFileSync(promotionInventoryFile, `${JSON.stringify(badEvidenceInventory, null, 2)}\n`);
  fixtureGit(["add", "qa/warnings/warning-inventory.json"]);
  fixtureGit(["commit", "-m", "invalid promotion evidence"]);
  const badEvidenceHead = fixtureGit(["rev-parse", "HEAD"]);
  await assert.rejects(
    auditZeroWarningPromotion({
      repoRoot: promotionFixtureRoot,
      baseRef: promotionBase,
      headRef: badEvidenceHead,
      configurationId: "generic-promotion-fixture",
      schema,
    }),
    /evidence\.commit must equal trusted base/,
  );
  await assert.rejects(
    auditZeroWarningPromotion({
      repoRoot: promotionFixtureRoot,
      baseRef: promotionHeadWithPair,
      headRef: badEvidenceHead,
      configurationId: "generic-promotion-fixture",
      schema,
    }),
    /rejects enforced configuration change/,
  );
  const driftInventory = structuredClone(promotionHeadInventory);
  driftInventory.configurations.find((candidate) => candidate.id === "generic-promotion-fixture").evidence.toolchain = {
    ...promotionToolchain,
    cargo: "toolchain-drift",
  };
  writeFileSync(promotionInventoryFile, `${JSON.stringify(driftInventory, null, 2)}\n`);
  fixtureGit(["add", "qa/warnings/warning-inventory.json"]);
  fixtureGit(["commit", "-m", "invalid promotion toolchain"]);
  const driftHead = fixtureGit(["rev-parse", "HEAD"]);
  await assert.rejects(
    auditZeroWarningPromotion({
      repoRoot: promotionFixtureRoot,
      baseRef: promotionBase,
      headRef: driftHead,
      configurationId: "generic-promotion-fixture",
      schema,
    }),
    /toolchain drift for cargo/,
  );
  const removedConfigurationInventory = structuredClone(promotionBaseInventory);
  removedConfigurationInventory.configurations.pop();
  writeFileSync(promotionInventoryFile, `${JSON.stringify(removedConfigurationInventory, null, 2)}\n`);
  fixtureGit(["add", "qa/warnings/warning-inventory.json"]);
  fixtureGit(["commit", "-m", "invalid pending removal"]);
  const removedConfigurationHead = fixtureGit(["rev-parse", "HEAD"]);
  await assert.rejects(
    auditZeroWarningPromotion({
      repoRoot: promotionFixtureRoot,
      baseRef: promotionBase,
      headRef: removedConfigurationHead,
      configurationId: "generic-promotion-fixture",
      schema,
    }),
    /rejects configuration add\/remove/,
  );
} finally {
  rmSync(promotionFixtureRoot, { recursive: true, force: true });
}
const outputMarkerFixtureRoot = mkdtempSync(path.join(tmpdir(), "syndocal-warning-output-marker-rebaseline-"));
try {
  const fixtureGit = (args) => execFileSync("git", args, { cwd: outputMarkerFixtureRoot, encoding: "utf8" }).trim();
  fixtureGit(["init", "--initial-branch=main"]);
  fixtureGit(["config", "user.name", "Warning Ratchet Output Marker Rebaseline Test"]);
  fixtureGit(["config", "user.email", "warning-ratchet-output-marker@example.invalid"]);
  mkdirSync(path.join(outputMarkerFixtureRoot, "qa/warnings"), { recursive: true });
  mkdirSync(path.join(outputMarkerFixtureRoot, "app/scripts"), { recursive: true });
  writeFileSync(path.join(outputMarkerFixtureRoot, ".gitignore"), "app/node_modules/\n");
  writeFileSync(
    path.join(outputMarkerFixtureRoot, "qa/warnings/warning-inventory.schema.json"),
    readFileSync(path.join(repoRoot, "qa/warnings/warning-inventory.schema.json"), "utf8"),
  );
  for (const script of ["check-warning-ratchet.mjs", "warning-ratchet-lib.mjs"]) {
    writeFileSync(
      path.join(outputMarkerFixtureRoot, "app/scripts", script),
      readFileSync(path.join(repoRoot, "app/scripts", script), "utf8"),
    );
  }
  symlinkSync(path.join(repoRoot, "app/node_modules"), path.join(outputMarkerFixtureRoot, "app/node_modules"), "junction");
  const outputMarkerCommand = {
    executable: "pnpm",
    args: [
      "--dir",
      path.join(repoRoot, "app"),
      "exec",
      "node",
      "-e",
      "process.stdout.write('vite v6.4.2 building for production...\\n268 modules transformed.\\nbuilt in')",
    ],
  };
  const outputMarkerToolchain = detectToolchain();
  const outputMarkerBaseInventory = structuredClone(inventory);
  const outputMarkerBaseConfiguration = outputMarkerBaseInventory.configurations.find(
    (candidate) => candidate.id === "frontend-typescript-vite-windows",
  );
  outputMarkerBaseConfiguration.command = outputMarkerCommand;
  outputMarkerBaseConfiguration.expectedOutputMarkers = [
    "vite v6.4.2 building for production...",
    "265 modules transformed.",
    "built in",
  ];
  outputMarkerBaseConfiguration.evidence = {
    commit: "a".repeat(40),
    capturedAt: "2026-08-22T00:00:00Z",
    command: `pnpm ${outputMarkerCommand.args.join(" ")}`,
    toolchain: outputMarkerToolchain,
  };
  const outputMarkerInventoryFile = path.join(outputMarkerFixtureRoot, "qa/warnings/warning-inventory.json");
  writeFileSync(outputMarkerInventoryFile, `${JSON.stringify(outputMarkerBaseInventory, null, 2)}\n`);
  fixtureGit(["add", ".gitignore", "app/scripts", "qa/warnings/warning-inventory.json", "qa/warnings/warning-inventory.schema.json"]);
  fixtureGit(["commit", "-m", "trusted output marker inventory"]);
  const outputMarkerBase = fixtureGit(["rev-parse", "HEAD"]);
  const outputMarkerHeadInventory = structuredClone(outputMarkerBaseInventory);
  const outputMarkerHeadConfiguration = outputMarkerHeadInventory.configurations.find(
    (candidate) => candidate.id === "frontend-typescript-vite-windows",
  );
  outputMarkerHeadConfiguration.expectedOutputMarkers[1] = "modules transformed.";
  outputMarkerHeadConfiguration.evidence = {
    ...outputMarkerHeadConfiguration.evidence,
    commit: outputMarkerBase,
    capturedAt: "2026-08-22T00:01:00Z",
  };
  writeFileSync(outputMarkerInventoryFile, `${JSON.stringify(outputMarkerHeadInventory, null, 2)}\n`);
  fixtureGit(["add", "qa/warnings/warning-inventory.json"]);
  fixtureGit(["commit", "-m", "rebaseline stable Vite marker"]);
  const outputMarkerHead = fixtureGit(["rev-parse", "HEAD"]);
  const runOutputMarkerCli = (args) => execFileSync(
    "pnpm",
    ["--dir", path.join(repoRoot, "app"), "exec", "node", path.join(outputMarkerFixtureRoot, "app/scripts/check-warning-ratchet.mjs"), ...args],
    {
      cwd: outputMarkerFixtureRoot,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
    },
  );
  const outputMarkerResult = await auditOutputMarkerRebaseline({
    repoRoot: outputMarkerFixtureRoot,
    baseRef: outputMarkerBase,
    headRef: outputMarkerHead,
    configurationId: "frontend-typescript-vite-windows",
    schema,
  });
  assert.equal(outputMarkerResult.configurationId, "frontend-typescript-vite-windows");
  assert.equal(outputMarkerResult.comparison.base, outputMarkerBase);
  assert.equal(outputMarkerResult.comparison.head, outputMarkerHead);
  assert.match(
    runOutputMarkerCli([
      "--rebaseline-output-markers",
      "--base-ref", outputMarkerBase,
      "--head-ref", outputMarkerHead,
      "--configuration", "frontend-typescript-vite-windows",
    ]),
    /output-marker rebaseline audit ok; inventory was not written/,
  );
  fixtureGit(["checkout", "-b", "output-marker-sibling", outputMarkerBase]);
  writeFileSync(outputMarkerInventoryFile, `${JSON.stringify(outputMarkerHeadInventory, null, 2)}\n`);
  fixtureGit(["add", "qa/warnings/warning-inventory.json"]);
  fixtureGit(["commit", "-m", "sibling output marker inventory"]);
  const outputMarkerSibling = fixtureGit(["rev-parse", "HEAD"]);
  assert.throws(
    () => resolveExplicitAncestorComparison(outputMarkerFixtureRoot, outputMarkerSibling, outputMarkerHead),
    /base is not an ancestor of head/,
  );
  assert.throws(
    () => resolveExplicitAncestorComparison(outputMarkerFixtureRoot, outputMarkerHead, outputMarkerSibling),
    /base is not an ancestor of head/,
  );
  await assert.rejects(
    auditOutputMarkerRebaseline({
      repoRoot: outputMarkerFixtureRoot,
      baseRef: outputMarkerSibling,
      headRef: outputMarkerHead,
      configurationId: "frontend-typescript-vite-windows",
      schema,
    }),
    /base is not an ancestor of head/,
  );
  fixtureGit(["checkout", "main"]);
  writeFileSync(path.join(outputMarkerFixtureRoot, "handoff.md"), "inventory unchanged checkpoint\n");
  fixtureGit(["add", "handoff.md"]);
  fixtureGit(["commit", "-m", "normal warning checkpoint"]);
  const outputMarkerNormalHead = fixtureGit(["rev-parse", "HEAD"]);
  assert.match(
    runOutputMarkerCli([
      "--configuration", "frontend-typescript-vite-windows",
      "--base-ref", outputMarkerHead,
      "--head-ref", outputMarkerNormalHead,
    ]),
    /warning ratchet ok/,
  );
  await assert.rejects(
    auditOutputMarkerRebaseline({ repoRoot: outputMarkerFixtureRoot, baseRef: null, headRef: outputMarkerHead, configurationId: "frontend-typescript-vite-windows", schema }),
    /requires explicit base and head refs/,
  );
  await assert.rejects(
    auditOutputMarkerRebaseline({ repoRoot: outputMarkerFixtureRoot, baseRef: outputMarkerBase, headRef: outputMarkerHead, schema }),
    /requires an explicit configuration id/,
  );
  fixtureGit(["checkout", "-b", "cargo-output-marker-rejection", outputMarkerHead]);
  const cargoRejectionInventory = structuredClone(outputMarkerHeadInventory);
  cargoRejectionInventory.configurations.find((candidate) => candidate.id === "windows-default-all-targets")
    .evidence.capturedAt = "2026-08-22T00:02:00Z";
  writeFileSync(outputMarkerInventoryFile, `${JSON.stringify(cargoRejectionInventory, null, 2)}\n`);
  fixtureGit(["add", "qa/warnings/warning-inventory.json"]);
  fixtureGit(["commit", "-m", "reject Cargo output marker rebaseline"]);
  const cargoRejectionHead = fixtureGit(["rev-parse", "HEAD"]);
  await assert.rejects(
    auditOutputMarkerRebaseline({ repoRoot: outputMarkerFixtureRoot, baseRef: outputMarkerHead, headRef: cargoRejectionHead, configurationId: "windows-default-all-targets", schema }),
    /only supports an enforced generic configuration/,
  );
  fixtureGit(["checkout", "main"]);
  fixtureGit(["checkout", "-b", "pending-output-marker-rejection", outputMarkerHead]);
  const pendingRejectionInventory = structuredClone(outputMarkerHeadInventory);
  pendingRejectionInventory.configurations.find((candidate) => candidate.id === "macos-default-all-targets")
    .nextAction = "still unavailable in test fixture";
  writeFileSync(outputMarkerInventoryFile, `${JSON.stringify(pendingRejectionInventory, null, 2)}\n`);
  fixtureGit(["add", "qa/warnings/warning-inventory.json"]);
  fixtureGit(["commit", "-m", "reject pending output marker rebaseline"]);
  const pendingRejectionHead = fixtureGit(["rev-parse", "HEAD"]);
  await assert.rejects(
    auditOutputMarkerRebaseline({ repoRoot: outputMarkerFixtureRoot, baseRef: outputMarkerHead, headRef: pendingRejectionHead, configurationId: "macos-default-all-targets", schema }),
    /only supports an enforced configuration/,
  );
  fixtureGit(["checkout", "main"]);
  fixtureGit(["checkout", "-b", "output-marker-negative-cases", outputMarkerHead]);
  await assert.rejects(
    auditOutputMarkerRebaseline({
      repoRoot: outputMarkerFixtureRoot,
      baseRef: outputMarkerBase,
      headRef: outputMarkerHead,
      configurationId: "frontend-typescript-vite-windows",
      schema,
      runConfiguration: async () => ({
        timedOut: false,
        outputLimitExceeded: false,
        exitCode: 0,
        warningShaped: true,
        markerCoverage: { ok: true },
        diagnostics: [],
      }),
    }),
    /warning-shaped output/,
  );

  const commitOutputMarkerInventory = (next, message) => {
    writeFileSync(outputMarkerInventoryFile, `${JSON.stringify(next, null, 2)}\n`);
    fixtureGit(["add", "qa/warnings/warning-inventory.json"]);
    fixtureGit(["commit", "-m", message]);
    return fixtureGit(["rev-parse", "HEAD"]);
  };
  const broadMarkerInventory = structuredClone(outputMarkerHeadInventory);
  broadMarkerInventory.configurations.find((candidate) => candidate.id === "frontend-typescript-vite-windows")
    .expectedOutputMarkers[1] = "modules";
  const broadMarkerHead = commitOutputMarkerInventory(broadMarkerInventory, "reject broad output marker");
  await assert.rejects(
    auditOutputMarkerRebaseline({ repoRoot: outputMarkerFixtureRoot, baseRef: outputMarkerBase, headRef: broadMarkerHead, configurationId: "frontend-typescript-vite-windows", schema }),
    /only permits one literal module-count marker/,
  );
  const multipleMarkerInventory = structuredClone(outputMarkerHeadInventory);
  const multipleMarkerConfiguration = multipleMarkerInventory.configurations.find(
    (candidate) => candidate.id === "frontend-typescript-vite-windows",
  );
  multipleMarkerConfiguration.expectedOutputMarkers[0] = "vite v6 building for production...";
  multipleMarkerConfiguration.expectedOutputMarkers[1] = "modules transformed.";
  const multipleMarkerHead = commitOutputMarkerInventory(multipleMarkerInventory, "reject multiple output marker changes");
  await assert.rejects(
    auditOutputMarkerRebaseline({ repoRoot: outputMarkerFixtureRoot, baseRef: outputMarkerBase, headRef: multipleMarkerHead, configurationId: "frontend-typescript-vite-windows", schema }),
    /only permits one literal module-count marker/,
  );
  const emptyMarkerInventory = structuredClone(outputMarkerHeadInventory);
  emptyMarkerInventory.configurations.find((candidate) => candidate.id === "frontend-typescript-vite-windows")
    .expectedOutputMarkers = [];
  const emptyMarkerHead = commitOutputMarkerInventory(emptyMarkerInventory, "reject empty output marker");
  await assert.rejects(
    auditOutputMarkerRebaseline({ repoRoot: outputMarkerFixtureRoot, baseRef: outputMarkerBase, headRef: emptyMarkerHead, configurationId: "frontend-typescript-vite-windows", schema }),
    /head inventory validation failed/,
  );
  const immutableFieldInventory = structuredClone(outputMarkerHeadInventory);
  immutableFieldInventory.configurations.find((candidate) => candidate.id === "frontend-typescript-vite-windows")
    .timeoutMs = 1_001;
  const immutableFieldHead = commitOutputMarkerInventory(immutableFieldInventory, "reject immutable output marker field");
  await assert.rejects(
    auditOutputMarkerRebaseline({ repoRoot: outputMarkerFixtureRoot, baseRef: outputMarkerBase, headRef: immutableFieldHead, configurationId: "frontend-typescript-vite-windows", schema }),
    /changed immutable configuration fields/,
  );
  const otherConfigurationInventory = structuredClone(outputMarkerHeadInventory);
  otherConfigurationInventory.configurations.find((candidate) => candidate.id === "windows-native-release")
    .evidence.capturedAt = "2026-08-22T00:02:00Z";
  const otherConfigurationHead = commitOutputMarkerInventory(otherConfigurationInventory, "reject other output marker configuration");
  await assert.rejects(
    auditOutputMarkerRebaseline({ repoRoot: outputMarkerFixtureRoot, baseRef: outputMarkerBase, headRef: otherConfigurationHead, configurationId: "frontend-typescript-vite-windows", schema }),
    /changed another configuration/,
  );
  const badEvidenceInventory = structuredClone(outputMarkerHeadInventory);
  badEvidenceInventory.configurations.find((candidate) => candidate.id === "frontend-typescript-vite-windows")
    .evidence.command = "pnpm unrelated";
  const badEvidenceHead = commitOutputMarkerInventory(badEvidenceInventory, "reject output marker evidence command");
  await assert.rejects(
    auditOutputMarkerRebaseline({ repoRoot: outputMarkerFixtureRoot, baseRef: outputMarkerBase, headRef: badEvidenceHead, configurationId: "frontend-typescript-vite-windows", schema }),
    /evidence\.command must exactly match/,
  );
  const badToolchainInventory = structuredClone(outputMarkerHeadInventory);
  badToolchainInventory.configurations.find((candidate) => candidate.id === "frontend-typescript-vite-windows")
    .evidence.toolchain = { ...outputMarkerToolchain, node: "toolchain-drift" };
  const badToolchainHead = commitOutputMarkerInventory(badToolchainInventory, "reject output marker toolchain drift");
  await assert.rejects(
    auditOutputMarkerRebaseline({ repoRoot: outputMarkerFixtureRoot, baseRef: outputMarkerBase, headRef: badToolchainHead, configurationId: "frontend-typescript-vite-windows", schema }),
    /toolchain drift for node/,
  );
  fixtureGit(["checkout", "-b", "suppression-output-marker-rejection", outputMarkerHead]);
  writeFileSync(path.join(outputMarkerFixtureRoot, "suppression.rs"), "#[allow(dead_code)]\nfn hidden() {}\n");
  fixtureGit(["add", "suppression.rs"]);
  fixtureGit(["commit", "-m", "reject output marker suppression"]);
  const suppressionHead = fixtureGit(["rev-parse", "HEAD"]);
  await assert.rejects(
    auditOutputMarkerRebaseline({
      repoRoot: outputMarkerFixtureRoot,
      baseRef: outputMarkerBase,
      headRef: suppressionHead,
      configurationId: "frontend-typescript-vite-windows",
      schema,
      allowedFiles: ["qa/warnings/warning-inventory.json", "suppression.rs"],
    }),
    /found suppression loopholes: rust-allow-or-expect-attribute/,
  );
  fixtureGit(["checkout", "-b", "outside-output-marker-rejection", outputMarkerHead]);
  writeFileSync(path.join(outputMarkerFixtureRoot, "outside.txt"), "not inventory-only\n");
  fixtureGit(["add", "outside.txt"]);
  fixtureGit(["commit", "-m", "reject non-inventory output marker file"]);
  const outsideFileHead = fixtureGit(["rev-parse", "HEAD"]);
  await assert.rejects(
    auditOutputMarkerRebaseline({ repoRoot: outputMarkerFixtureRoot, baseRef: outputMarkerBase, headRef: outsideFileHead, configurationId: "frontend-typescript-vite-windows", schema }),
    /outside inventory\/warning gate scope/,
  );
} finally {
  rmSync(outputMarkerFixtureRoot, { recursive: true, force: true });
}
await assert.rejects(
  runCargoConfiguration(spoutConfiguration, repoRoot, { ...process.env, RUSTFLAGS: "-A" + "warnings" }),
  /warning-affecting environment is forbidden/,
);
await assert.rejects(
  runCargoConfiguration({
    ...spoutConfiguration,
    command: {
      ...spoutConfiguration.command,
      args: [
        ...spoutConfiguration.command.args,
        "--config",
        ["build.rust", "flags=['-A", "warnings']"].join(""),
      ],
    },
  }, repoRoot),
  /--config is forbidden/,
);
const defaultConfiguration = inventory.configurations.find((candidate) => candidate.id === "windows-default-all-targets");
const environmentWithoutSdk = { ...process.env };
delete environmentWithoutSdk.FFMPEG_DIR;
delete environmentWithoutSdk.LIBCLANG_PATH;
await assert.rejects(
  runCargoConfiguration(defaultConfiguration, repoRoot, environmentWithoutSdk),
  /required build environment is missing/,
);
const cargoConfigRoot = mkdtempSync(path.join(tmpdir(), "syndocal-warning-config-"));
try {
  mkdirSync(path.join(cargoConfigRoot, ".cargo"));
  writeFileSync(path.join(cargoConfigRoot, ".cargo", "config.toml"), "[build]\nrust" + "flags = [\"-A\", \"warnings\"]\n");
  assert.equal(warningAffectingCargoConfigs(cargoConfigRoot, {}).length, 1);
  for (const key of compilerSelectionKeys) {
    for (const form of compilerSelectionConfigForms) {
      writeFileSync(path.join(cargoConfigRoot, ".cargo", "config.toml"), form(key));
      assert.equal(warningAffectingCargoConfigs(cargoConfigRoot, {}).length, 1);
      await assert.rejects(
        runCargoConfiguration(spoutConfiguration, cargoConfigRoot),
        /warning-affecting Cargo config is forbidden/,
      );
    }
  }
  for (const configText of [
    ...semanticWarningFlagConfigs,
    ...semanticBuildControlConfigs,
    ...semanticProfileConfigs,
    ...semanticEnvironmentConfigs,
  ]) {
    writeFileSync(path.join(cargoConfigRoot, ".cargo", "config.toml"), configText);
    assert.equal(warningAffectingCargoConfigs(cargoConfigRoot, {}).length, 1);
    await assert.rejects(
      runCargoConfiguration(spoutConfiguration, cargoConfigRoot),
      /warning-affecting Cargo config is forbidden/,
    );
  }
  writeFileSync(path.join(cargoConfigRoot, ".cargo", "config.toml"), "[build\ninvalid = true\n");
  assert.equal(warningAffectingCargoConfigs(cargoConfigRoot, {}).length, 1);
  await assert.rejects(
    runCargoConfiguration(spoutConfiguration, cargoConfigRoot),
    /warning-affecting Cargo config is forbidden/,
  );
  writeFileSync(
    path.join(cargoConfigRoot, ".cargo", "config.toml"),
    "[build]\njobs = 2\n[target.'cfg(windows)']\nlinker = \"link.exe\"\n[env]\nrustc_note = \"documentation only\"\nmy_rustc = { value = \"documentation only\" }\nCARGO_TARGET_DIR = \"target-alt\"\nMY_CARGO_PROFILE_RELEASE = \"documentation only\"\n",
  );
  assert.equal(warningAffectingCargoConfigs(cargoConfigRoot, {}).length, 0);
  writeFileSync(
    path.join(cargoConfigRoot, ".cargo", "config.toml"),
    Buffer.alloc(CURRENT_FILE_CONTENT_MAX_BYTES + 1, "#"),
  );
  assert.throws(
    () => warningAffectingCargoConfigs(cargoConfigRoot, {}),
    new RegExp(`Cargo config content exceeded the warning-ratchet limit of ${CURRENT_FILE_CONTENT_MAX_BYTES} bytes`),
  );
} finally {
  rmSync(cargoConfigRoot, { recursive: true, force: true });
}
for (const key of compilerSelectionKeys) {
  await assert.rejects(
    runCargoConfiguration({
      ...spoutConfiguration,
      command: {
        ...spoutConfiguration.command,
        args: [...spoutConfiguration.command.args, "--config", ["build.", key, "='compiler-proxy'"].join("")],
      },
    }, repoRoot),
    /--config is forbidden/,
  );
}
assert.ok(!readFileSync(path.join(repoRoot, ".github/workflows/cross-platform.yml"), "utf8").includes("bootstrap-baseline"));
const extraInventory = structuredClone(inventory);
extraInventory.unexpected = true;
assert.ok(validateInventory(extraInventory, schema).some((error) => error.includes("additional properties")));
const duplicateInventory = structuredClone(inventory);
duplicateInventory.configurations.push(structuredClone(duplicateInventory.configurations[0]));
assert.ok(validateInventory(duplicateInventory, schema).some((error) => error.includes("duplicate configuration")));
const duplicateDiagnosticInventory = structuredClone(inventory);
duplicateDiagnosticInventory.configurations[0].diagnostics.push(structuredClone(duplicateDiagnosticInventory.configurations[0].diagnostics[0]));
assert.ok(validateInventory(duplicateDiagnosticInventory, schema).some((error) => error.includes("duplicate diagnostic identity")));
const duplicateArtifactInventory = structuredClone(inventory);
duplicateArtifactInventory.configurations[0].expectedArtifacts.push(structuredClone(duplicateArtifactInventory.configurations[0].expectedArtifacts[0]));
assert.ok(validateInventory(duplicateArtifactInventory, schema).some((error) => error.includes("duplicate expected artifact")));
const orphanAllowInventory = structuredClone(inventory);
orphanAllowInventory.configurations[0].externalWarningAllows.push({
  identity: "0".repeat(64),
  owner: "owner",
  reason: "reason",
  upstream: "https://example.invalid/upstream",
  expiry: "2099-01-01T00:00:00Z",
  maxOccurrences: 1,
});
assert.ok(validateInventory(orphanAllowInventory, schema).some((error) => error.includes("does not match a third-party diagnostic")));

const malformed = parseCargoJsonLines(["not-json"], configuration, context);
assert.equal(malformed.invalidJsonLines.length, 1);
assert.deepEqual(malformed.buildFinished, []);
assert.deepEqual(parseCargoJsonLines([JSON.stringify({ reason: "build-finished", success: false })], configuration, context).buildFinished, [false]);

console.log("warning ratchet self-tests ok");
