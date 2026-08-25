import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import {
  existsSync,
  linkSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { platform, pid as currentProcessId } from "node:process";
import { fileURLToPath } from "node:url";
import {
  isBlockedAsioRuntimeDllName,
  validateAsioPackagingBoundary,
  validateAsioSdkPin,
  windowsFfmpegRuntimeDlls,
} from "./check-release-metadata.mjs";
import {
  anchoredAmd64PeIdentity,
  assertDirectoryTreeHasNoReparsePoints,
  assertExactWindowsTargetTriple,
  assertNoBlockedAsioPayload,
  assertSafeExternalDirectory,
  canonicalKnownAsioBridgeSha256,
  canonicalPinnedCommonResourceIdentity,
  canonicalPinnedRuntimeIdentity,
  enforceWindowsRuntimeInventoryAnchors,
  findWindowsReparsePointPaths,
  loadWindowsRuntimeInventory,
  readVerifiedRegularFile,
  validateWindowsRuntimeInventory,
  verifyPinnedRuntimeFile,
} from "./windows-runtime-inventory.mjs";
import {
  assertNoAsioRuntimeDllInDirectory,
  assertNoNdiCliBypass,
  assertNoNdiEnabledBundling,
  assertNoNdiFeatureEscape,
  main as prepareReleaseRuntime,
  parseProcessChainSnapshot,
  releaseDirectories,
  validateFfmpegRuntimeDllNames,
} from "./prepare-release-runtime.mjs";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const workspaceRoot = resolve(scriptDir, "../..");
const runtimeInventoryFileName = "FFMPEG_WINDOWS_RUNTIME_INVENTORY.json";
const read = (path) => readFileSync(resolve(workspaceRoot, path), "utf8");
const sourceMap = new Map([
  ["app/src-tauri/tauri.conf.json", read("app/src-tauri/tauri.conf.json")],
  ["app/src-tauri/tauri.windows.conf.json", read("app/src-tauri/tauri.windows.conf.json")],
  ["app/src-tauri/tauri.updater.conf.json", read("app/src-tauri/tauri.updater.conf.json")],
  ["qa/ASIO_SDK_PIN.json", read("qa/ASIO_SDK_PIN.json")],
  ["app/src-tauri/Cargo.toml", read("app/src-tauri/Cargo.toml")],
  ["app/package.json", read("app/package.json")],
  [".github/workflows/cross-platform.yml", read(".github/workflows/cross-platform.yml")],
]);
const mutationRead = (mutations) => (path) => mutations.get(path) ?? sourceMap.get(path) ?? read(path);
const inventory = loadWindowsRuntimeInventory();

let assertions = 0;
const skips = [];
const pass = (condition, label) => {
  assert.ok(condition, label);
  assertions += 1;
};
const rejects = (action, pattern, label) => {
  assert.throws(action, pattern, label);
  assertions += 1;
};
const skipVisibility = (label, reason) => {
  skips.push(label + ": " + reason);
};
const rejectsBoundary = (mutate, pattern, label) => {
  const mutations = new Map();
  mutate(mutations);
  rejects(
    () => validateAsioPackagingBoundary(mutationRead(mutations), { verifyWindowsRuntimeSources: false }),
    pattern,
    label,
  );
};
const mutateJson = (mutations, path, mutate) => {
  const json = JSON.parse(sourceMap.get(path));
  mutate(json);
  mutations.set(path, JSON.stringify(json));
};

validateAsioPackagingBoundary(mutationRead(new Map()), { verifyWindowsRuntimeSources: false });
assertions += 1;
pass(
  JSON.stringify(validateFfmpegRuntimeDllNames([...windowsFfmpegRuntimeDlls], "approved names", inventory))
    === JSON.stringify(windowsFfmpegRuntimeDlls),
  "the exact seven-name FFmpeg allowlist is accepted",
);
pass(
  inventory.common_resources.some((resource) => resource.destination === "THIRD_PARTY_NOTICES.md"),
  "the normal common-resource allowlist includes THIRD_PARTY_NOTICES",
);

for (const blocked of ["syndocal_asio_bridge.dll", "syndocal-asio-bridge.dll", "third-party-asio-helper.dll"]) {
  pass(isBlockedAsioRuntimeDllName(blocked), "ASIO runtime name is blocked: " + blocked);
  rejects(
    () => validateFfmpegRuntimeDllNames([...windowsFfmpegRuntimeDlls, blocked], "mutated names", inventory),
    /forbidden ASIO runtime DLL name|exact approved FFmpeg set/,
    "ASIO runtime name cannot enter the FFmpeg list",
  );
}
rejects(
  () => validateFfmpegRuntimeDllNames(windowsFfmpegRuntimeDlls.slice(1), "missing names", inventory),
  /exact approved FFmpeg set/,
  "missing FFmpeg DLL is rejected",
);
rejects(
  () => validateFfmpegRuntimeDllNames([...windowsFfmpegRuntimeDlls, "other-runtime.dll"], "extra names", inventory),
  /exact approved FFmpeg set/,
  "extra DLL is rejected",
);

for (const [path, mutate, pattern, label] of [
  [
    "app/src-tauri/tauri.windows.conf.json",
    (config) => { config.bundle.resources["../../target/release/syndocal_asio_bridge.dll"] = "syndocal_asio_bridge.dll"; },
    /exact approved explicit runtime-resource map/,
    "canonical ASIO resource is rejected",
  ],
  [
    "app/src-tauri/tauri.windows.conf.json",
    (config) => { config.bundle.resources = { "../../target/release/*.dll": "" }; },
    /exact approved explicit runtime-resource map/,
    "DLL glob is rejected",
  ],
  [
    "app/src-tauri/tauri.windows.conf.json",
    (config) => { config.bundle.resources["../../target/release/avcodec-62.dll"] = "nested/avcodec-62.dll"; },
    /exact approved explicit runtime-resource map/,
    "runtime destination remap is rejected",
  ],
  [
    "app/src-tauri/tauri.windows.conf.json",
    (config) => { delete config.bundle.resources["../../target/release/avcodec-62.dll"]; },
    /exact approved explicit runtime-resource map/,
    "runtime resource removal is rejected",
  ],
  [
    "app/src-tauri/tauri.conf.json",
    (config) => { delete config.bundle.resources["../../THIRD_PARTY_NOTICES.md"]; },
    /exact approved explicit runtime-resource map/,
    "THIRD_PARTY_NOTICES removal is rejected",
  ],
  [
    "app/src-tauri/tauri.conf.json",
    (config) => { config.bundle.resources["../../licenses/FFmpeg-LGPL-3.0.txt"] = "licenses/renamed.txt"; },
    /exact approved explicit runtime-resource map/,
    "common resource remap is rejected",
  ],
  [
    "app/src-tauri/tauri.conf.json",
    (config) => { config.bundle.resources["../../licenses"] = "licenses"; },
    /exact approved explicit runtime-resource map/,
    "directory resource is rejected",
  ],
  [
    "app/src-tauri/tauri.updater.conf.json",
    (config) => { config.bundle.resources = { "../../target/release/syndocal_asio_bridge.dll": "syndocal_asio_bridge.dll" }; },
    /must not add or remap/,
    "updater resource injection is rejected",
  ],
]) {
  rejectsBoundary((mutations) => mutateJson(mutations, path, mutate), pattern, label);
}

const canonicalPin = JSON.parse(sourceMap.get("qa/ASIO_SDK_PIN.json"));
for (const key of Object.keys(canonicalPin)) {
  const mutated = structuredClone(canonicalPin);
  if (typeof mutated[key] === "number") mutated[key] += 1;
  else if (typeof mutated[key] === "boolean") mutated[key] = !mutated[key];
  else mutated[key] += "-drift";
  rejects(
    () => validateAsioSdkPin(mutated),
    /unexpected|distribution boundary/,
    "ASIO SDK pin drift is rejected: " + key,
  );
}
const pinWithFutureField = structuredClone(canonicalPin);
pinWithFutureField.future_schema_field = true;
rejects(
  () => validateAsioSdkPin(pinWithFutureField),
  /keys are not exact/,
  "future ASIO SDK pin schema field is rejected",
);
rejectsBoundary(
  (mutations) => {
    const pin = structuredClone(canonicalPin);
    pin.distribution_approved = true;
    mutations.set("qa/ASIO_SDK_PIN.json", JSON.stringify(pin));
  },
  /unexpected distribution_approved/,
  "ASIO distribution approval drift is rejected",
);

for (const runtime of inventory.runtime_dlls) {
  for (const [field, mutation, description] of [
    ["byte_size", (value) => value + 1, "byte_size increment"],
    ["sha256", (value) => (value.startsWith("0") ? "1" : "0") + value.slice(1), "SHA-256 digit swap"],
  ]) {
    const mutatedInventory = structuredClone(inventory);
    const entry = mutatedInventory.runtime_dlls.find((candidate) => candidate.filename === runtime.filename);
    entry[field] = mutation(entry[field]);
    rejects(
      () => enforceWindowsRuntimeInventoryAnchors(validateWindowsRuntimeInventory(mutatedInventory)),
      /independently anchored approved byte_size\/SHA-256 authority/,
      "inventory cannot redefine anchored authority (" + runtime.filename + " " + description + ")",
    );
  }
  for (const [field, mutatedValue, description] of [
    ["machine", 0xaa64, "ARM64 PE machine substitution"],
    ["machine", 0x8664 + 1, "PE machine drift"],
    ["characteristics", 0x222e ^ 1, "PE characteristics drift"],
    ["optional_magic", 0x20b ^ 0xff, "PE optional-header magic drift"],
    ["signature", "MZ", "PE signature drift"],
  ]) {
    const mutatedInventory = structuredClone(inventory);
    mutatedInventory.runtime_dlls.find((candidate) => candidate.filename === runtime.filename).pe_identity[field]
      = mutatedValue;
    rejects(
      () => validateWindowsRuntimeInventory(mutatedInventory),
      /pinned PE32\+ AMD64 DLL identity/,
      "inventory cannot redefine anchored PE identity (" + runtime.filename + " " + description + ")",
    );
  }
}
pass(
  [...canonicalPinnedRuntimeIdentity.keys()].join("|") === windowsFfmpegRuntimeDlls.join("|"),
  "the independent code anchor covers exactly the canonical seven-DLL set",
);
for (const resource of inventory.common_resources) {
  const identityKey = resource.source + "\0" + resource.destination;
  for (const [field, mutation, description] of [
    ["byte_size", (value) => value + 1, "byte_size increment"],
    ["sha256", (value) => value.slice(0, 8) + (value[8] === "a" ? "b" : "a") + value.slice(9), "SHA-256 digit swap"],
  ]) {
    const mutatedInventory = structuredClone(inventory);
    const entry = mutatedInventory.common_resources.find((candidate) => candidate.source === resource.source);
    entry[field] = mutation(entry[field]);
    rejects(
      () => enforceWindowsRuntimeInventoryAnchors(validateWindowsRuntimeInventory(mutatedInventory)),
      /independently anchored approved byte_size\/SHA-256 authority/,
      "inventory cannot redefine common-resource authority (" + resource.destination + " " + description + ")",
    );
  }
  pass(canonicalPinnedCommonResourceIdentity.has(identityKey), "common resource is independently anchored: " + resource.destination);
}
const bridgeSetMutations = [
  [(hashes) => hashes.map((hash, index) => (index === 0 ? "0".repeat(64) : hash)), "substituted hash"],
  [(hashes) => hashes.slice(1), "removed hash"],
  [(hashes) => [...hashes, "f".repeat(64)], "appended hash"],
];
for (const [mutation, description] of bridgeSetMutations) {
  const mutatedInventory = structuredClone(inventory);
  mutatedInventory.known_asio_bridge_sha256 = mutation(mutatedInventory.known_asio_bridge_sha256);
  rejects(
    () => enforceWindowsRuntimeInventoryAnchors(validateWindowsRuntimeInventory(mutatedInventory)),
    /not exactly the independently anchored known-build set/,
    "inventory cannot redefine the known ASIO bridge hash set (" + description + ")",
  );
}
pass(
  JSON.stringify([...inventory.known_asio_bridge_sha256].sort())
    === JSON.stringify([...canonicalKnownAsioBridgeSha256].sort()),
  "the shipped known ASIO bridge hash set equals the independent code anchor",
);
const reorderedBridgeHashes = structuredClone(inventory);
reorderedBridgeHashes.known_asio_bridge_sha256 = [...reorderedBridgeHashes.known_asio_bridge_sha256].reverse();
enforceWindowsRuntimeInventoryAnchors(validateWindowsRuntimeInventory(reorderedBridgeHashes));
assertions += 1;

rejectsBoundary(
  (mutations) => {
    const cargo = sourceMap.get("app/src-tauri/Cargo.toml").replace('default = ["libav", "spout"]', 'default = ["libav", "spout", "ndi"]');
    mutations.set("app/src-tauri/Cargo.toml", cargo);
  },
  /Default Tauri feature selection/,
  "NDI default-feature bundling is rejected",
);
rejectsBoundary(
  (mutations) => {
    const packageManifest = sourceMap.get("app/package.json") + "\n\"tauri build --features ndi\"";
    mutations.set("app/package.json", packageManifest);
  },
  /NDI-enabled bundling/,
  "NDI feature bundle command is rejected",
);

const benignManifestText = sourceMap.get("app/package.json");
pass(assertNoNdiFeatureEscape(() => benignManifestText) === undefined, "benign package manifest passes the NDI feature-escape scan");
pass(
  (() => {
    assertNoNdiFeatureEscape((path) => sourceMap.get(path) ?? read(path));
    return true;
  })(),
  "all repository-owned packaging surfaces pass the NDI feature-escape scan",
);
for (const [path, hostileText, label] of [
  ["app/package.json", "{\n  \"scripts\": {\n    \"bundle\": \"pnpm tauri build --features ndi\"\n  }\n}", "package script --features ndi"],
  ["app/package.json", "{\n  \"scripts\": {\n    \"bundle\": \"pnpm tauri build --features=ndi\"\n  }\n}", "package script --features=ndi"],
  ["app/package.json", "{\n  \"scripts\": {\n    \"bundle\": \"pnpm tauri build -F ndi\"\n  }\n}", "package script short -F ndi"],
  [".github/workflows/cross-platform.yml", "run: cargo build --features ndi -p syndocal\n", "workflow cargo --features ndi"],
  ["app/src-tauri/tauri.conf.json", "{\n  \"build\": {\n    \"beforeBuildCommand\": \"pnpm build && cargo build --features ndi\"\n  }\n}", "tauri beforeBuildCommand ndi escape"],
  ["app/src-tauri/tauri.windows.conf.json", "{}\nrun: tauri build --features ndi\n", "windows overlay ndi escape"],
  ["app/src-tauri/Cargo.toml", "[package]\nname = \"x\"\n[features]\nndi-bundle = []\ndefault = [\"ndi-bundle\"]\nrun: cargo build --features ndi-bundle\n", "cargo ndi-bundle alias escape"],
]) {
  rejects(
    () => assertNoNdiFeatureEscape((probedPath) => (probedPath === path ? hostileText : sourceMap.get(probedPath) ?? read(probedPath))),
    new RegExp("NDI-enabled feature escape is fail-closed in the .*\\(" + path.replace(/[.*+?^${}()|[\]\\]/gu, "\\$&") + "\\)", "u"),
    "NDI feature-escape surface rejection: " + label,
  );
}
{
  const landingText = "run: echo $NDI_SDK_DIR\nrun: pnpm tauri build --features libav spout\n# NDIS adapters are unrelated; the word landing contains no standalone token\n";
  pass(
    assertNoNdiFeatureEscape((path) => (path === "app/package.json" ? landingText : sourceMap.get(path) ?? read(path))) === undefined,
    "NDI_SDK_DIR references, benign feature lists, and embedded words do not false-positive the escape scan",
  );
}

rejects(
  () => assertNoNdiEnabledBundling({ CARGO_FEATURE_NDI: "1" }),
  /NDI-enabled bundling is fail-closed/,
  "runtime NDI Cargo feature is rejected",
);
rejects(
  () => assertNoNdiEnabledBundling({ TAURI_BUNDLE_FEATURES: "libav ndi" }),
  /NDI-enabled bundling is fail-closed/,
  "runtime NDI feature list is rejected",
);
rejects(
  () => assertNoNdiEnabledBundling({ SYNDOCAL_NDI_EXPERIMENT: "1" }),
  /Unknown NDI environment signal is fail-closed/,
  "unknown NDI-named environment signal is rejected",
);
rejects(
  () => assertNoNdiEnabledBundling({ SYNDOCAL_NDI_OVERLAY_INTENT: "" }),
  /Unknown NDI environment signal is fail-closed/,
  "empty unknown NDI environment signal is still rejected",
);
pass(
  assertNoNdiEnabledBundling({
    windir: "C:\\Windows",
    NDI_SDK_DIR: "C:\\Program Files\\NDI",
    NDI_RUNTIME_DIR_V6: "C:\\Program Files\\NDI\\Runtime\\v6",
    CPAL_ASIO_DIR: "C:\\asio",
  }) === undefined,
  "documented SDK-path and platform variables are not treated as NDI enablement signals",
);

const chainSnapshot = [
  { ProcessId: 4, ParentProcessId: 0, CommandLine: null },
  { ProcessId: 100, ParentProcessId: 4, CommandLine: "Runner.Worker.exe --job ... normal CI text" },
  { ProcessId: 200, ParentProcessId: 100, CommandLine: "pwsh -Command pnpm --dir app run prepare:runtime-libs" },
  { ProcessId: 300, ParentProcessId: 200, CommandLine: "node scripts/prepare-release-runtime.mjs" },
];
pass(Array.isArray(parseProcessChainSnapshot(JSON.stringify(chainSnapshot))), "process-chain snapshot parses a plain array");
pass(parseProcessChainSnapshot("[{\"ProcessId\":1}]").length === 1, "process-chain snapshot parses a single-entry array");
pass(parseProcessChainSnapshot("not-json") === null, "unparseable process-chain snapshot fails closed to null");
pass(parseProcessChainSnapshot("{\"ProcessId\":true}") === null, "non-integer process identifiers fail closed");
for (const [hostileLine, label] of [
  ["node tauri.js build --ci --features ndi", "ancestor tauri CLI --features ndi"],
  ["node tauri.js build --ci --features=ndi", "ancestor tauri CLI --features=ndi"],
  ["node tauri.js build -F ndi --ci", "ancestor tauri CLI short -F ndi"],
  ["cmd /c pnpm tauri build --features libav,ndi", "ancestor combined feature list"],
  ["powershell -Command cargo build --features syndocal/ndi", "ancestor cargo feature-path escape"],
]) {
  const processes = parseProcessChainSnapshot(JSON.stringify([
    { ProcessId: currentProcessId, ParentProcessId: 500, CommandLine: "node scripts/prepare-release-runtime.mjs" },
    { ProcessId: 500, ParentProcessId: 600, CommandLine: hostileLine },
    { ProcessId: 600, ParentProcessId: 0, CommandLine: "explorer.exe" },
  ]));
  rejects(
    () => assertNoNdiCliBypass(process.env, { platform, currentProcessId, snapshot: processes }),
    /ancestor packaging process was invoked with an NDI feature flag/,
    "NDI CLI bypass chain rejection: " + label,
  );
}
{
  const benignProcesses = parseProcessChainSnapshot(JSON.stringify([
    { ProcessId: currentProcessId, ParentProcessId: 500, CommandLine: "node scripts/prepare-release-runtime.mjs" },
    { ProcessId: 500, ParentProcessId: 600, CommandLine: "pwsh -Command Add-Content $env:GITHUB_ENV 'NDI_SDK_DIR=C:\\Program Files\\NDI'" },
    { ProcessId: 600, ParentProcessId: 0, CommandLine: "Runner.Worker.exe" },
  ]));
  pass(
    assertNoNdiCliBypass({ NDI_SDK_DIR: "C:\\Program Files\\NDI" }, { platform, currentProcessId, snapshot: benignProcesses }) === undefined,
    "a benign ancestor chain with documented NDI_SDK_DIR usage passes the CLI bypass audit",
  );
}
rejects(
  () => assertNoNdiCliBypass({}, { platform, currentProcessId: 999999, snapshot: [{ processId: 1, parentProcessId: 0, commandLine: "" }] }),
  /could not locate the packaging process itself/,
  "an incomplete process snapshot fails closed",
);

for (const targetTriple of [
  "",
  " x86_64-pc-windows-msvc",
  "x86_64-pc-windows-msvc ",
  "x86_64-pc-windows-gnu",
  "aarch64-pc-windows-msvc",
  "../x86_64-pc-windows-msvc",
  "\\\\server\\share",
  "x86_64-pc-windows-msvc:ads",
]) {
  rejects(
    () => assertExactWindowsTargetTriple(targetTriple),
    /TAURI_ENV_TARGET_TRIPLE/,
    "ambiguous or unsupported target triple is rejected: " + JSON.stringify(targetTriple),
  );
}
rejects(
  () => assertExactWindowsTargetTriple("aarch64-pc-windows-msvc"),
  /aarch64\/ARM64 targets and never copies AMD64 DLLs to aarch64/,
  "ARM64 target triple is explicitly refused by the AMD64-only inventory",
);
rejects(
  () => releaseDirectories(workspaceRoot, "aarch64-pc-windows-msvc"),
  /TAURI_ENV_TARGET_TRIPLE/,
  "no aarch64 staging directory may be resolved from the AMD64 inventory",
);
pass(
  assertExactWindowsTargetTriple("x86_64-pc-windows-msvc") === "x86_64-pc-windows-msvc",
  "exact x64 target triple is accepted",
);
pass(
  releaseDirectories(workspaceRoot, "x86_64-pc-windows-msvc").every((path) => path.startsWith(resolve(workspaceRoot, "target"))),
  "resolved target directories remain beneath the workspace target root",
);

const temporaryDirectory = mkdtempSync(join(tmpdir(), "syndocal-asio-packaging-"));
try {
  const plain = join(temporaryDirectory, "plain.bin");
  writeFileSync(plain, "plain", { flag: "wx" });
  pass(readVerifiedRegularFile(plain, "plain fixture", { allowedRoots: [temporaryDirectory] }).bytes.toString("utf8") === "plain", "ordinary regular resource is readable");
  const hardLink = join(temporaryDirectory, "plain-alias.bin");
  linkSync(plain, hardLink);
  rejects(
    () => readVerifiedRegularFile(hardLink, "hard-link fixture", { allowedRoots: [temporaryDirectory] }),
    /hard-link alias/,
    "hard-link resource alias is rejected",
  );
  try {
    const symbolic = join(temporaryDirectory, "plain-link.bin");
    symlinkSync(plain, symbolic, "file");
    rejects(
      () => readVerifiedRegularFile(symbolic, "symbolic-link fixture", { allowedRoots: [temporaryDirectory] }),
      /symbolic link|reparse point|ordinary regular file/,
      "symbolic-link resource alias is rejected",
    );
    if (platform === "win32") {
      pass(findWindowsReparsePointPaths([symbolic], "symlink fixture").length === 1, "reparse attribute audit flags a created symbolic link");
    }
  } catch (error) {
    if (!["EPERM", "EACCES"].includes(error?.code)) throw error;
    skipVisibility("symbolic-link fixture rejection", "alias creation denied with " + error.code + "; coverage skipped visibly, not counted as executed");
  }
  try {
    const decoy = join(temporaryDirectory, "decoy-dir");
    mkdirSync(decoy);
    const junction = join(temporaryDirectory, "evil.dll");
    symlinkSync(decoy, junction, "junction");
    rejects(
      () => readVerifiedRegularFile(junction, "junction fixture", { allowedRoots: [temporaryDirectory] }),
      /ordinary regular file|reparse/,
      "junction posing as a DLL filename is rejected",
    );
    rejects(
      () => assertNoAsioRuntimeDllInDirectory(temporaryDirectory, "directory junction fixture", inventory),
      /DLL directory, symbolic link, or reparse point/,
      "DLL-named junction entry is rejected during directory audit",
    );
    if (platform === "win32") {
      rejects(
        () => assertDirectoryTreeHasNoReparsePoints(temporaryDirectory, "tree reparse audit fixture"),
        /reparse point entries/,
        "arbitrary-named junction is rejected by the recursive tree audit",
      );
      pass(findWindowsReparsePointPaths([junction], "junction fixture").length === 1, "reparse attribute audit flags a created junction");
    } else {
      skipVisibility("recursive reparse tree audit", "Windows-specific attribute audit not applicable on " + platform);
    }
  } catch (error) {
    if (!["EPERM", "EACCES"].includes(error?.code)) throw error;
    skipVisibility("junction fixture rejection", "junction creation denied with " + error.code + "; coverage skipped visibly, not counted as executed");
  }
  mkdirSync(join(temporaryDirectory, "nested.dll"));
  rejects(
    () => assertNoAsioRuntimeDllInDirectory(temporaryDirectory, "directory DLL fixture", inventory),
    /DLL directory/,
    "DLL directory resource is rejected",
  );
  for (const unsafe of [
    " " + temporaryDirectory,
    temporaryDirectory + " ",
    "\\\\server\\share",
    "C:\\\\safe:stream",
    temporaryDirectory + "\\..\\outside",
  ]) {
    rejects(
      () => assertSafeExternalDirectory(unsafe, "unsafe path fixture"),
      /non-UNC|alternate data stream|does not exist/,
      "unsafe path is rejected: " + JSON.stringify(unsafe),
    );
  }
  for (const runtime of inventory.runtime_dlls) {
    const bridgeBytes = Buffer.from("bridge-payload-" + runtime.filename, "utf8");
    const bridgeInventory = { ...inventory, known_asio_bridge_sha256: [createHash("sha256").update(bridgeBytes).digest("hex")] };
    rejects(
      () => assertNoBlockedAsioPayload(runtime.filename, bridgeBytes, "renamed bridge fixture", bridgeInventory),
      /known ASIO bridge build/,
      "known bridge bytes are rejected under allowed FFmpeg name " + runtime.filename,
    );
  }

  const syntheticPeBytes = (machine) => {
    const bytes = Buffer.alloc(0x100, 0x5a);
    bytes.write("MZ", 0, "latin1");
    bytes.writeUInt32LE(0x80, 0x3c);
    bytes.write("PE\0\0", 0x80, "latin1");
    bytes.writeUInt16LE(machine, 0x84);
    bytes.writeUInt16LE(anchoredAmd64PeIdentity.characteristics, 0x80 + 22);
    bytes.writeUInt16LE(anchoredAmd64PeIdentity.optional_magic, 0x80 + 24);
    return bytes;
  };
  const syntheticRecordFor = (bytes) => ({
    filename: "swresample-6.dll",
    byte_size: bytes.length,
    sha256: createHash("sha256").update(bytes).digest("hex"),
    pe_identity: {
      signature: "PE\0\0",
      machine: bytes.readUInt16LE(0x84),
      characteristics: bytes.readUInt16LE(0x80 + 22),
      optional_magic: bytes.readUInt16LE(0x80 + 24),
    },
  });
  const syntheticAmd64 = syntheticPeBytes(0x8664);
  const syntheticAmd64Path = join(temporaryDirectory, "synthetic-amd64.dll");
  writeFileSync(syntheticAmd64Path, syntheticAmd64, { flag: "wx" });
  pass(
    verifyPinnedRuntimeFile(syntheticAmd64Path, syntheticRecordFor(syntheticAmd64), "synthetic AMD64 fixture", { allowedRoots: [temporaryDirectory] }).bytes.length
      === syntheticAmd64.length,
    "self-built synthetic AMD64 PE fixture verifies without any untracked target output",
  );
  const tamperedSynthetic = Buffer.from(syntheticAmd64);
  tamperedSynthetic[tamperedSynthetic.length - 1] ^= 0x01;
  const tamperedSyntheticPath = join(temporaryDirectory, "synthetic-tampered.dll");
  writeFileSync(tamperedSyntheticPath, tamperedSynthetic, { flag: "wx" });
  rejects(
    () => verifyPinnedRuntimeFile(tamperedSyntheticPath, syntheticRecordFor(syntheticAmd64), "modified synthetic fixture", { allowedRoots: [temporaryDirectory] }),
    /SHA-256/,
    "modified synthetic FFmpeg bytes are rejected without depending on target output",
  );
  const arm64Bytes = syntheticPeBytes(0xaa64);
  const arm64Path = join(temporaryDirectory, "synthetic-arm64.dll");
  writeFileSync(arm64Path, arm64Bytes, { flag: "wx" });
  const anchoredSwresample = inventory.runtime_dlls.find((runtime) => runtime.filename === "swresample-6.dll");
  const arm64SelfConsistentRecord = {
    ...structuredClone(anchoredSwresample),
    byte_size: arm64Bytes.length,
    sha256: createHash("sha256").update(arm64Bytes).digest("hex"),
  };
  rejects(
    () => verifyPinnedRuntimeFile(arm64Path, arm64SelfConsistentRecord, "ARM64 payload fixture", { allowedRoots: [temporaryDirectory] }),
    /PE identity does not match pinned inventory/,
    "an ARM64 PE payload is rejected against the anchored AMD64 identity",
  );

  const targetRoot = resolve(workspaceRoot, "target");
  const targetExistedBefore = existsSync(targetRoot);
  const beforeNames = targetExistedBefore ? JSON.stringify(readdirSync(targetRoot).sort()) : null;
  const result = prepareReleaseRuntime({
    platform: "win32",
    ffmpegDir: undefined,
    workspace: workspaceRoot,
    targetTriple: undefined,
    environment: {},
  });
  pass(result.stagedDirectories.length === 0, "unset FFMPEG_DIR creates no release directory");
  if (targetExistedBefore) {
    pass(beforeNames === JSON.stringify(readdirSync(targetRoot).sort()), "unset FFMPEG_DIR copies no DLL into the untracked target tree");
  } else {
    skipVisibility(
      "cold-checkout no-op evidence",
      "untracked target directory absent on this cold checkout; stagedDirectories==0 above is the executed assertion",
    );
  }
} finally {
  rmSync(temporaryDirectory, { recursive: true, force: true });
}

const malformedInventory = structuredClone(inventory);
malformedInventory.runtime_dlls[0].sha256 = malformedInventory.runtime_dlls[0].sha256.toUpperCase();
rejects(
  () => validateWindowsRuntimeInventory(malformedInventory),
  /lowercase SHA-256/,
  "runtime inventory hash spelling drift is rejected",
);
const inventoryWithFutureField = structuredClone(inventory);
inventoryWithFutureField.future = true;
rejects(
  () => validateWindowsRuntimeInventory(inventoryWithFutureField),
  /keys are not exact/,
  "runtime inventory future field is rejected",
);
{
  const tamperedInventoryDirectory = mkdtempSync(join(tmpdir(), "syndocal-inventory-authority-"));
  try {
    const editedInventory = structuredClone(inventory);
    editedInventory.runtime_dlls[0].byte_size += 1;
    const editedInventoryPath = join(tamperedInventoryDirectory, runtimeInventoryFileName);
    writeFileSync(editedInventoryPath, JSON.stringify(editedInventory), { flag: "wx" });
    rejects(
      () => loadWindowsRuntimeInventory({ workspace: tamperedInventoryDirectory, inventoryPath: editedInventoryPath }),
      /independently anchored approved byte_size\/SHA-256 authority/,
      "an edited inventory file cannot redefine anchored authority at load time",
    );
  } finally {
    rmSync(tamperedInventoryDirectory, { recursive: true, force: true });
  }
}

console.log("ASIO packaging boundary self-test passed: " + assertions + " assertions");
if (skips.length > 0) {
  console.warn("SKIPPED (visible, not counted as executed assertions): " + skips.length);
  for (const skip of skips) console.warn("  SKIP " + skip);
}
