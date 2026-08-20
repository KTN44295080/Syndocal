import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  aggregateDiagnostics,
  artifactIdentity,
  compareArtifactCoverage,
  compareDiagnostics,
  collectModifiedFiles,
  controlledChildEnvironment,
  detectSuppressionText,
  diagnosticIdentityHash,
  findAddedSuppressions,
  findBaselineLaundering,
  forbiddenWarningEnvironment,
  loadInventory,
  loadInventoryAtRef,
  normalizeRepoPath,
  parseCargoJsonLines,
  resolveTrustedComparison,
  runCargoConfiguration,
  runProcessWithTimeout,
  validateInventory,
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
assert.deepEqual(detectSuppressionText("const rustc = selectCompiler();\nconst wrapperName = 'rustc-wrapper';"), []);

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

assert.throws(() => resolveTrustedComparison(repoRoot, "0".repeat(40), "HEAD"), /invalid comparison ref/);
assert.throws(() => resolveTrustedComparison(repoRoot, "definitely-missing", "HEAD"), /unresolvable/);
assert.throws(() => resolveTrustedComparison(repoRoot, "HEAD", "HEAD"), /must differ/);
const trusted = resolveTrustedComparison(repoRoot, null, "HEAD");
assert.notEqual(trusted.base, trusted.head);
assert.throws(() => loadInventoryAtRef(repoRoot, trusted.base, "qa/warnings/does-not-exist.json"), /missing or invalid/);

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

assert.deepEqual(findBaselineLaundering({ id: "x" }, { id: "x" }), []);
assert.ok(findBaselineLaundering({ id: "x", status: "enforced" }, { id: "x", status: "pending" }).length > 0);

const inventory = loadInventory(path.join(repoRoot, "qa/warnings/warning-inventory.json"));
const schema = loadInventory(path.join(repoRoot, "qa/warnings/warning-inventory.schema.json"));
const spoutConfiguration = inventory.configurations.find((candidate) => candidate.id === "windows-syndocal-spout");
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

console.log("warning ratchet self-tests ok: 52 assertion groups");
