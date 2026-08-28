import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import {
  existsSync,
  linkSync,
  lstatSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  readdirSync,
  rmSync,
  symlinkSync,
  unlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { basename, dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { validateAsioPackagingBoundary } from "./check-release-metadata.mjs";
import {
  assertDirectoryTreeHasNoReparsePoints,
  assertSafeExternalDirectory,
  loadWindowsRuntimeInventory,
  readVerifiedRegularFile,
  verifyPinnedCommonResource,
  verifyPinnedRuntimeFile,
} from "./windows-runtime-inventory.mjs";
import { validatePinnedFfmpegRuntimeDirectory } from "./prepare-release-runtime.mjs";
import {
  assertExactShowAsioArtifactDirectory,
  checkShowAsioArtifact,
  collectShowAsioSourceIdentity,
  expectedShowAsioArtifactRelativeDirectory,
  parsePeIdentity,
  showAsioArtifactFlavor,
  showAsioBridgeExports,
  showAsioFeatures,
  showAsioManifestFilename,
  showAsioPlatform,
  assertShowAsioSourceBranch,
  showAsioSourceIdentityPaths,
} from "./check-show-asio-artifact.mjs";

const scriptDir = dirname(fileURLToPath(import.meta.url));
export const workspaceRoot = resolve(scriptDir, "../..");
const applicationTargetRelativePath = "target/show-asio-build/app/release/syndocal.exe";
const bridgeTargetRelativePath = "target/show-asio-build/bridge/release/syndocal_asio_bridge.dll";

function hash(bytes) {
  return createHash("sha256").update(bytes).digest("hex");
}

function canonical(candidate) {
  const normalized = resolve(candidate).replaceAll("\\", "/");
  return process.platform === "win32" ? normalized.toLocaleLowerCase("en-US") : normalized;
}

function cargoBuildIdentity(stats) {
  return String(stats.dev) + ":" + String(stats.ino) + ":" + String(stats.size) + ":" + String(stats.mtimeNs);
}

function assertCargoBuildEntry(stats, label) {
  if (!stats.isFile() || stats.isSymbolicLink() || stats.nlink !== 2n) {
    throw new Error(label + " must have exactly the canonical root/deps two-name link topology.");
  }
}

function readVerifiedCargoRootDepsBuildOutput(outputPath, targetPath, filename, label) {
  const target = resolve(targetPath);
  const rootOutput = resolve(outputPath);
  const expectedRootOutput = resolve(target, "release", filename);
  const depsOutput = resolve(target, "release/deps", filename);
  if (canonical(rootOutput) !== canonical(expectedRootOutput)) {
    throw new Error(label + " root output is not the exact isolated target path.");
  }
  if (!existsSync(depsOutput)) throw new Error(label + " canonical deps output is missing.");

  const rootBefore = lstatSync(rootOutput, { bigint: true });
  const depsBefore = lstatSync(depsOutput, { bigint: true });
  assertCargoBuildEntry(rootBefore, label + " root output");
  assertCargoBuildEntry(depsBefore, label + " deps output");
  const rootBeforeIdentity = cargoBuildIdentity(rootBefore);
  const depsBeforeIdentity = cargoBuildIdentity(depsBefore);
  if (rootBeforeIdentity !== depsBeforeIdentity) {
    throw new Error(label + " root/deps outputs are not exactly two names for one unchanged file identity.");
  }

  const rootRecord = readVerifiedRegularFile(rootOutput, label + " root output", {
    allowedRoots: [target],
    rejectHardLinks: false,
  });
  const depsRecord = readVerifiedRegularFile(depsOutput, label + " deps output", {
    allowedRoots: [target],
    rejectHardLinks: false,
  });
  const rootAfter = lstatSync(rootOutput, { bigint: true });
  const depsAfter = lstatSync(depsOutput, { bigint: true });
  assertCargoBuildEntry(rootAfter, label + " root output after read");
  assertCargoBuildEntry(depsAfter, label + " deps output after read");
  const rootAfterIdentity = cargoBuildIdentity(rootAfter);
  const depsAfterIdentity = cargoBuildIdentity(depsAfter);
  if (
    rootBeforeIdentity !== rootAfterIdentity
    || depsBeforeIdentity !== depsAfterIdentity
    || rootRecord.identity !== rootBeforeIdentity
    || depsRecord.identity !== depsBeforeIdentity
    || rootRecord.identity !== depsRecord.identity
    || !rootRecord.bytes.equals(depsRecord.bytes)
  ) {
    throw new Error(label + " root/deps outputs are not exactly two names for one unchanged file identity.");
  }
  return rootRecord;
}

export function readVerifiedCargoBridgeBuildOutput(bridgePath, bridgeTarget) {
  return readVerifiedCargoRootDepsBuildOutput(
    bridgePath,
    bridgeTarget,
    "syndocal_asio_bridge.dll",
    "Show-ASIO Cargo bridge",
  );
}

export function readVerifiedCargoApplicationBuildOutput(applicationPath, applicationTarget) {
  return readVerifiedCargoRootDepsBuildOutput(
    applicationPath,
    applicationTarget,
    "syndocal.exe",
    "Show-ASIO Cargo application",
  );
}

function assertExactInputPath(workspace, candidate, relativePath, label) {
  if (typeof candidate !== "string" || candidate.length === 0 || candidate !== candidate.trim()) {
    throw new Error(label + " must be one exact absolute path.");
  }
  const expected = resolve(workspace, ...relativePath.split("/"));
  if (canonical(candidate) !== canonical(expected)) throw new Error(label + " must be exactly " + expected + ".");
  return expected;
}

function assertExactExports(actual) {
  if (JSON.stringify(actual) !== JSON.stringify(showAsioBridgeExports)) {
    throw new Error("Show-ASIO bridge exports differ from the exact ABI v2 plus v3 set.");
  }
}

function assertApplicationPe(identity) {
  if ((identity.characteristics & 0x0002) === 0 || (identity.characteristics & 0x2000) !== 0) {
    throw new Error("Show-ASIO application must be an executable AMD64 PE32+ image, not a DLL.");
  }
}

function assertDllPe(identity, label) {
  if ((identity.characteristics & 0x0002) === 0 || (identity.characteristics & 0x2000) === 0) {
    throw new Error(label + " must be an executable AMD64 PE32+ DLL image.");
  }
}

function ensureOnlyPinnedFfmpegDlls(sourceDirectory, inventory) {
  const expected = new Set(inventory.runtime_dlls.map((entry) => entry.filename.toLocaleLowerCase("en-US")));
  const actual = readdirSync(sourceDirectory, { withFileTypes: true })
    .filter((entry) => entry.name.toLocaleLowerCase("en-US").endsWith(".dll"))
    .map((entry) => {
      if (!entry.isFile() || entry.isSymbolicLink()) throw new Error("FFMPEG_DIR/bin contains a non-regular DLL entry: " + entry.name);
      return entry.name.toLocaleLowerCase("en-US");
    });
  if (actual.length !== expected.size || actual.some((name) => !expected.has(name))) {
    throw new Error("FFMPEG_DIR/bin must contain the exact seven pinned FFmpeg DLL names and no extra DLL.");
  }
}

export function collectShowAsioInputRecords({
  workspace,
  applicationPath,
  bridgePath,
  expectedBridgeSha256,
  ffmpegDir,
  inventory,
  exportInspector,
  allowCargoRootDepsAliases = false,
} = {}) {
  const resolvedWorkspace = assertSafeExternalDirectory(workspace, "Syndocal workspace");
  const exactApplication = assertExactInputPath(resolvedWorkspace, applicationPath, applicationTargetRelativePath, "Show-ASIO application input");
  const exactBridge = assertExactInputPath(resolvedWorkspace, bridgePath, bridgeTargetRelativePath, "Show-ASIO bridge input");
  if (basename(exactBridge) !== "syndocal_asio_bridge.dll") throw new Error("Show-ASIO bridge must use the one canonical DLL filename.");
  if (typeof expectedBridgeSha256 !== "string" || !/^[0-9a-f]{64}$/u.test(expectedBridgeSha256)) {
    throw new Error("Show-ASIO expected bridge SHA-256 must be one exact lowercase digest captured from this build.");
  }
  const applicationTarget = resolve(resolvedWorkspace, "target/show-asio-build/app");
  const appRecord = allowCargoRootDepsAliases
    ? readVerifiedCargoApplicationBuildOutput(exactApplication, applicationTarget)
    : readVerifiedRegularFile(exactApplication, "Show-ASIO built application", { allowedRoots: [applicationTarget] });
  const appPe = parsePeIdentity(appRecord.bytes, "Show-ASIO built application");
  assertApplicationPe(appPe);
  const bridgeTarget = resolve(resolvedWorkspace, "target/show-asio-build/bridge");
  const bridgeRecord = allowCargoRootDepsAliases
    ? readVerifiedCargoBridgeBuildOutput(exactBridge, bridgeTarget)
    : readVerifiedRegularFile(exactBridge, "Show-ASIO built bridge", { allowedRoots: [bridgeTarget] });
  if (hash(bridgeRecord.bytes) !== expectedBridgeSha256) throw new Error("Show-ASIO bridge hash changed after the bridge build checkpoint.");
  const bridgePe = parsePeIdentity(bridgeRecord.bytes, "Show-ASIO built bridge");
  assertDllPe(bridgePe, "Show-ASIO built bridge");
  const bridgeExports = exportInspector(exactBridge);
  assertExactExports(bridgeExports);

  const ffmpegRoot = assertSafeExternalDirectory(ffmpegDir, "FFMPEG_DIR");
  const ffmpegBin = assertSafeExternalDirectory(join(ffmpegRoot, "bin"), "FFMPEG_DIR/bin");
  assertDirectoryTreeHasNoReparsePoints(ffmpegBin, "FFMPEG_DIR/bin");
  ensureOnlyPinnedFfmpegDlls(ffmpegBin, inventory);
  validatePinnedFfmpegRuntimeDirectory(ffmpegBin, "Show-ASIO FFMPEG_DIR/bin", inventory);
  const runtimeRecords = inventory.runtime_dlls.map((runtime) => ({
    runtime,
    record: verifyPinnedRuntimeFile(
      join(ffmpegBin, runtime.filename),
      runtime,
      "Show-ASIO FFmpeg runtime " + runtime.filename,
      { allowedRoots: [ffmpegBin] },
    ),
  }));
  const resourceRecords = inventory.common_resources.map((resource) => {
    const source = resolve(resolvedWorkspace, "app/src-tauri", ...resource.source.split("/"));
    return {
      resource,
      record: verifyPinnedCommonResource(source, resource, "Show-ASIO notice " + resource.destination, {
        allowedRoots: [resolvedWorkspace],
      }),
    };
  });
  const localNotice = readVerifiedRegularFile(
    resolve(resolvedWorkspace, "qa/ASIO_SHOW_LOCAL_ONLY.md"),
    "Show-ASIO local-only notice",
    { allowedRoots: [resolvedWorkspace] },
  );
  return {
    workspace: resolvedWorkspace,
    application: { record: appRecord, pe: appPe },
    bridge: { record: bridgeRecord, pe: bridgePe, exports: bridgeExports },
    runtimes: runtimeRecords,
    resources: resourceRecords,
    localNotice,
  };
}

function writeRecord(destination, record) {
  mkdirSync(dirname(destination), { recursive: true });
  writeFileSync(destination, record.bytes, { flag: "wx" });
  const staged = readVerifiedRegularFile(destination, "New Show-ASIO staged file", { allowedRoots: [dirname(destination), resolve(dirname(destination), "..")] });
  if (staged.bytes.length !== record.bytes.length || hash(staged.bytes) !== hash(record.bytes)) {
    throw new Error("Show-ASIO staged bytes differ from their verified input: " + destination);
  }
  return staged;
}

function ensureArtifactParent(workspace, artifactDir) {
  const targetRoot = resolve(workspace, "target");
  const localRoot = resolve(targetRoot, "show-asio-local");
  if (canonical(dirname(artifactDir)) !== canonical(localRoot)) {
    throw new Error("Show-ASIO final directory parent is not the exact target/show-asio-local authority.");
  }
  if (!existsSync(targetRoot)) mkdirSync(targetRoot);
  assertSafeExternalDirectory(targetRoot, "Show-ASIO target root");
  if (!existsSync(localRoot)) mkdirSync(localRoot);
  assertSafeExternalDirectory(localRoot, "Show-ASIO local artifact root");
  return localRoot;
}

function manifestFile(role, path, record, extra = {}) {
  return { role, path, byteSize: record.bytes.length, sha256: hash(record.bytes), ...extra };
}

export function prepareShowAsioRuntime({
  workspace = workspaceRoot,
  version,
  commit,
  sourceBranch,
  hostBindingSha256,
  sourceFiles,
  ffmpegDir,
  expectedBridgeSha256,
  applicationPath = resolve(workspace, applicationTargetRelativePath),
  bridgePath = resolve(workspace, bridgeTargetRelativePath),
  artifactDir = resolve(workspace, expectedShowAsioArtifactRelativeDirectory(version, commit)),
  inventory = loadWindowsRuntimeInventory({ workspace }),
  exportInspector,
  allowCargoRootDepsAliases = false,
  validateNormalBoundary = () => validateAsioPackagingBoundary(undefined, { workspace, verifyWindowsRuntimeSources: false }),
} = {}) {
  assertShowAsioSourceBranch(sourceBranch, "Show-ASIO source branch");
  validateNormalBoundary();
  if (existsSync(artifactDir)) throw new Error("Show-ASIO final artifact directory already exists and will never be overwritten: " + artifactDir);
  const exactArtifactDir = assertExactShowAsioArtifactDirectory(workspace, artifactDir, version, commit, { mustExist: false });
  const records = collectShowAsioInputRecords({
    workspace,
    applicationPath,
    bridgePath,
    expectedBridgeSha256,
    ffmpegDir,
    inventory,
    exportInspector,
    allowCargoRootDepsAliases,
  });
  const currentSourceFiles = collectShowAsioSourceIdentity(records.workspace);
  if (JSON.stringify(sourceFiles) !== JSON.stringify(currentSourceFiles)) {
    throw new Error("Show-ASIO source identity drifted before staging; no artifact directory was created.");
  }
  ensureArtifactParent(records.workspace, exactArtifactDir);
  mkdirSync(exactArtifactDir, { recursive: false });
  assertDirectoryTreeHasNoReparsePoints(exactArtifactDir, "New Show-ASIO artifact directory");
  const files = [];
  const appStaged = writeRecord(join(exactArtifactDir, "syndocal-show-asio.exe"), records.application.record);
  files.push(manifestFile("application", "syndocal-show-asio.exe", appStaged, { pe: records.application.pe }));
  const bridgeStaged = writeRecord(join(exactArtifactDir, "syndocal_asio_bridge.dll"), records.bridge.record);
  files.push(manifestFile("asio-bridge", "syndocal_asio_bridge.dll", bridgeStaged, {
    pe: records.bridge.pe,
    bridgeAbiVersion: 3,
    exports: [...showAsioBridgeExports],
  }));
  for (const { runtime, record } of records.runtimes) {
    const staged = writeRecord(join(exactArtifactDir, runtime.filename), record);
    files.push(manifestFile("ffmpeg-runtime", runtime.filename, staged, {
      pe: parsePeIdentity(staged.bytes, "Staged " + runtime.filename),
    }));
  }
  for (const { resource, record } of records.resources) {
    const staged = writeRecord(resolve(exactArtifactDir, ...resource.destination.split("/")), record);
    files.push(manifestFile("notice", resource.destination, staged));
  }
  const localNoticeStaged = writeRecord(join(exactArtifactDir, "ASIO_SHOW_LOCAL_ONLY.md"), records.localNotice);
  files.push(manifestFile("notice", "ASIO_SHOW_LOCAL_ONLY.md", localNoticeStaged));
  const manifest = {
    schemaVersion: 3,
    artifactFlavor: showAsioArtifactFlavor,
    platform: showAsioPlatform,
    productVersion: version,
    commit,
    commitShort: commit.slice(0, 12),
    sourceBranch,
    distributionApproved: false,
    sameHostOnly: true,
    unbundled: true,
    featureSet: [...showAsioFeatures],
    hostBindingSha256,
    artifactDirectory: expectedShowAsioArtifactRelativeDirectory(version, commit),
    buildTargets: { application: "target/show-asio-build/app", bridge: "target/show-asio-build/bridge" },
    sourceFiles,
    restrictions: { installer: false, updater: false, archive: false, copy: false, publish: false, signing: false, ndi: false },
    verification: { checker: "app/scripts/check-show-asio-artifact.mjs", requiredBeforeEveryUse: true, manifestWrittenLast: true },
    files,
  };
  writeFileSync(join(exactArtifactDir, showAsioManifestFilename), JSON.stringify(manifest, null, 2) + "\n", { flag: "wx" });
  const checked = checkShowAsioArtifact({
    workspace: records.workspace,
    artifactDir: exactArtifactDir,
    expectedVersion: version,
    expectedCommit: commit,
    expectedSourceBranch: sourceBranch,
    expectedHostBindingSha256: hostBindingSha256,
    inventory,
    exportInspector,
  });
  return { artifactDir: exactArtifactDir, manifest, filesVerified: checked.filesVerified };
}

function syntheticPe({ dll, marker }) {
  const bytes = Buffer.alloc(0x120, marker);
  bytes.write("MZ", 0, "latin1");
  bytes.writeUInt32LE(0x80, 0x3c);
  bytes.write("PE\0\0", 0x80, "latin1");
  bytes.writeUInt16LE(0x8664, 0x84);
  bytes.writeUInt16LE(0x0002 | (dll ? 0x2000 : 0), 0x80 + 22);
  bytes.writeUInt16LE(0x20b, 0x80 + 24);
  return bytes;
}

function makeFixture(root, label) {
  const workspace = resolve(root, label);
  mkdirSync(workspace, { recursive: true });
  for (const path of showAsioSourceIdentityPaths) {
    const destination = resolve(workspace, ...path.split("/"));
    mkdirSync(dirname(destination), { recursive: true });
    const bytes = path === "qa/release/windows-show-asio-local-manifest.schema.json"
      ? readFileSync(resolve(workspaceRoot, path))
      : (path === "qa/ASIO_SHOW_LOCAL_ONLY.md" ? "LOCAL ONLY" : "source:" + path);
    writeFileSync(destination, bytes, { flag: "wx" });
  }
  const appPath = resolve(workspace, applicationTargetRelativePath);
  const bridgePath = resolve(workspace, bridgeTargetRelativePath);
  mkdirSync(dirname(appPath), { recursive: true });
  mkdirSync(dirname(bridgePath), { recursive: true });
  writeFileSync(appPath, syntheticPe({ dll: false, marker: 0x61 }), { flag: "wx" });
  writeFileSync(bridgePath, syntheticPe({ dll: true, marker: 0x62 }), { flag: "wx" });
  const ffmpegDir = resolve(workspace, "fixture-ffmpeg");
  const ffmpegBin = join(ffmpegDir, "bin");
  mkdirSync(ffmpegBin, { recursive: true });
  const runtimeDlls = [
    "avcodec-62.dll", "avdevice-62.dll", "avfilter-11.dll", "avformat-62.dll",
    "avutil-60.dll", "swresample-6.dll", "swscale-9.dll",
  ].map((filename, index) => {
    const bytes = syntheticPe({ dll: true, marker: 0x30 + index });
    writeFileSync(join(ffmpegBin, filename), bytes, { flag: "wx" });
    return {
      filename,
      byte_size: bytes.length,
      sha256: hash(bytes),
      pe_identity: { signature: "PE\0\0", machine: 0x8664, characteristics: 0x2002, optional_magic: 0x20b },
    };
  });
  const commonResources = [
    ["../../THIRD_PARTY_NOTICES.md", "THIRD_PARTY_NOTICES.md"],
    ["../../licenses/FFmpeg-LGPL-3.0.txt", "licenses/FFmpeg-LGPL-3.0.txt"],
    ["../../licenses/Spout2-BSD-2-Clause.txt", "licenses/Spout2-BSD-2-Clause.txt"],
    ["../../licenses/bcdec_rs-MIT.txt", "licenses/bcdec_rs-MIT.txt"],
  ].map(([source, destination]) => {
    const bytes = Buffer.from("notice:" + destination, "utf8");
    const sourcePath = resolve(workspace, "app/src-tauri", ...source.split("/"));
    mkdirSync(dirname(sourcePath), { recursive: true });
    writeFileSync(sourcePath, bytes, { flag: "wx" });
    return { source, destination, byte_size: bytes.length, sha256: hash(bytes) };
  });
  const version = "1.2.0-alpha.99";
  const commit = "d".repeat(40);
  const sourceBranch = "codex/syndocal-v1.2";
  const inventory = { runtime_dlls: runtimeDlls, common_resources: commonResources, known_asio_bridge_sha256: [] };
  return {
    workspace,
    appPath,
    bridgePath,
    bridgeHash: hash(readFileSync(bridgePath)),
    ffmpegDir,
    inventory,
    version,
    commit,
    sourceBranch,
    artifactDir: resolve(workspace, expectedShowAsioArtifactRelativeDirectory(version, commit)),
    sourceFiles: collectShowAsioSourceIdentity(workspace),
  };
}

async function runSelfTest() {
  let assertions = 0;
  const skips = [];
  const pass = (condition, label) => { assert.ok(condition, label); assertions += 1; };
  const rejects = (action, pattern, label) => { assert.throws(action, pattern, label); assertions += 1; };
  const root = mkdtempSync(join(tmpdir(), "syndocal-show-asio-prepare-"));
  try {
    const valid = makeFixture(root, "valid");
    const inspect = () => [...showAsioBridgeExports];
    pass(
      valid.sourceFiles.length === 70 && valid.sourceFiles.some((entry) => entry.path === "app/src/uiLocalization.ts"),
      "runtime staging carries the exact 70-path source identity including UI localization",
    );
    const result = prepareShowAsioRuntime({
      workspace: valid.workspace,
      version: valid.version,
      commit: valid.commit,
      sourceBranch: valid.sourceBranch,
      hostBindingSha256: "e".repeat(64),
      sourceFiles: valid.sourceFiles,
      ffmpegDir: valid.ffmpegDir,
      expectedBridgeSha256: valid.bridgeHash,
      applicationPath: valid.appPath,
      bridgePath: valid.bridgePath,
      artifactDir: valid.artifactDir,
      inventory: valid.inventory,
      exportInspector: inspect,
      validateNormalBoundary: () => {},
    });
    pass(result.filesVerified === 14, "exact synthetic same-host artifact is staged and post-manifest verified");
    rejects(
      () => prepareShowAsioRuntime({
        workspace: valid.workspace,
        version: valid.version,
        commit: valid.commit,
        sourceBranch: valid.sourceBranch,
        hostBindingSha256: "e".repeat(64),
        sourceFiles: valid.sourceFiles,
        ffmpegDir: valid.ffmpegDir,
        expectedBridgeSha256: valid.bridgeHash,
        applicationPath: valid.appPath,
        bridgePath: valid.bridgePath,
        artifactDir: valid.artifactDir,
        inventory: valid.inventory,
        exportInspector: inspect,
        validateNormalBoundary: () => {},
      }),
      /already exists/,
      "existing final directory is never overwritten",
    );
    rejects(
      () => assertExactShowAsioArtifactDirectory(valid.workspace, "\\\\server\\share\\artifact", valid.version, valid.commit, { mustExist: false }),
      /non-UNC/,
      "UNC targets are rejected",
    );
    rejects(
      () => assertExactShowAsioArtifactDirectory(valid.workspace, resolve(valid.workspace, "target/show-asio-local/../escape"), valid.version, valid.commit, { mustExist: false }),
      /not the exact authority path/,
      "target escapes are rejected",
    );

    const missing = makeFixture(root, "missing-bridge");
    unlinkSync(missing.bridgePath);
    rejects(
      () => collectShowAsioInputRecords({ ...missing, applicationPath: missing.appPath, bridgePath: missing.bridgePath, expectedBridgeSha256: missing.bridgeHash, inventory: missing.inventory, exportInspector: inspect }),
      /missing/,
      "missing bridge is rejected",
    );
    const wrongHash = makeFixture(root, "wrong-hash");
    rejects(
      () => collectShowAsioInputRecords({ ...wrongHash, applicationPath: wrongHash.appPath, bridgePath: wrongHash.bridgePath, expectedBridgeSha256: "f".repeat(64), inventory: wrongHash.inventory, exportInspector: inspect }),
      /hash changed/,
      "wrong bridge hash is rejected",
    );
    const wrongExports = makeFixture(root, "wrong-exports");
    rejects(
      () => collectShowAsioInputRecords({ ...wrongExports, applicationPath: wrongExports.appPath, bridgePath: wrongExports.bridgePath, expectedBridgeSha256: wrongExports.bridgeHash, inventory: wrongExports.inventory, exportInspector: () => showAsioBridgeExports.slice(1) }),
      /exports differ/,
      "wrong bridge export set is rejected",
    );
    const wrongPe = makeFixture(root, "wrong-pe");
    writeFileSync(wrongPe.bridgePath, Buffer.from("not-pe"));
    rejects(
      () => collectShowAsioInputRecords({ ...wrongPe, applicationPath: wrongPe.appPath, bridgePath: wrongPe.bridgePath, expectedBridgeSha256: hash(Buffer.from("not-pe")), inventory: wrongPe.inventory, exportInspector: inspect }),
      /DOS\/PE/,
      "non-PE bridge is rejected",
    );
    const missingFfmpeg = makeFixture(root, "missing-ffmpeg");
    unlinkSync(join(missingFfmpeg.ffmpegDir, "bin", missingFfmpeg.inventory.runtime_dlls[0].filename));
    rejects(
      () => collectShowAsioInputRecords({ ...missingFfmpeg, applicationPath: missingFfmpeg.appPath, bridgePath: missingFfmpeg.bridgePath, expectedBridgeSha256: missingFfmpeg.bridgeHash, inventory: missingFfmpeg.inventory, exportInspector: inspect }),
      /exact seven|exact approved|missing/,
      "missing FFmpeg runtime is rejected",
    );
    const extraFfmpeg = makeFixture(root, "extra-ffmpeg");
    writeFileSync(join(extraFfmpeg.ffmpegDir, "bin", "extra.dll"), syntheticPe({ dll: true, marker: 0x77 }), { flag: "wx" });
    rejects(
      () => collectShowAsioInputRecords({ ...extraFfmpeg, applicationPath: extraFfmpeg.appPath, bridgePath: extraFfmpeg.bridgePath, expectedBridgeSha256: extraFfmpeg.bridgeHash, inventory: extraFfmpeg.inventory, exportInspector: inspect }),
      /exact seven|extra DLL/,
      "extra FFmpeg DLL is rejected",
    );
    const mutatedFfmpeg = makeFixture(root, "mutated-ffmpeg");
    const mutatePath = join(mutatedFfmpeg.ffmpegDir, "bin", mutatedFfmpeg.inventory.runtime_dlls[0].filename);
    const mutatedBytes = readFileSync(mutatePath);
    mutatedBytes[mutatedBytes.length - 1] ^= 1;
    writeFileSync(mutatePath, mutatedBytes);
    rejects(
      () => collectShowAsioInputRecords({ ...mutatedFfmpeg, applicationPath: mutatedFfmpeg.appPath, bridgePath: mutatedFfmpeg.bridgePath, expectedBridgeSha256: mutatedFfmpeg.bridgeHash, inventory: mutatedFfmpeg.inventory, exportInspector: inspect }),
      /SHA-256/,
      "one-byte FFmpeg mutation is rejected",
    );
    const hardlinked = makeFixture(root, "hardlink-bridge");
    const bridgeOriginal = join(dirname(hardlinked.bridgePath), "bridge-original.dll");
    writeFileSync(bridgeOriginal, readFileSync(hardlinked.bridgePath), { flag: "wx" });
    unlinkSync(hardlinked.bridgePath);
    linkSync(bridgeOriginal, hardlinked.bridgePath);
    rejects(
      () => collectShowAsioInputRecords({ ...hardlinked, applicationPath: hardlinked.appPath, bridgePath: hardlinked.bridgePath, expectedBridgeSha256: hardlinked.bridgeHash, inventory: hardlinked.inventory, exportInspector: inspect }),
      /hard-link alias/,
      "hard-linked bridge input is rejected",
    );
    const cargoAliases = makeFixture(root, "cargo-root-deps-aliases");
    const cargoAppDeps = resolve(cargoAliases.workspace, "target/show-asio-build/app/release/deps/syndocal.exe");
    const cargoBridgeDeps = resolve(cargoAliases.workspace, "target/show-asio-build/bridge/release/deps/syndocal_asio_bridge.dll");
    mkdirSync(dirname(cargoAppDeps), { recursive: true });
    mkdirSync(dirname(cargoBridgeDeps), { recursive: true });
    linkSync(cargoAliases.appPath, cargoAppDeps);
    linkSync(cargoAliases.bridgePath, cargoBridgeDeps);
    const cargoAliasRecords = collectShowAsioInputRecords({
      ...cargoAliases,
      applicationPath: cargoAliases.appPath,
      bridgePath: cargoAliases.bridgePath,
      expectedBridgeSha256: cargoAliases.bridgeHash,
      inventory: cargoAliases.inventory,
      exportInspector: inspect,
      allowCargoRootDepsAliases: true,
    });
    pass(
      cargoAliasRecords.application.record.identity !== cargoAliasRecords.bridge.record.identity,
      "explicit Cargo mode accepts only exact app and bridge root/deps two-name outputs and keeps their identities distinct",
    );
    const reparse = makeFixture(root, "reparse-bridge");
    const reparseOriginal = join(dirname(reparse.bridgePath), "bridge-real.dll");
    writeFileSync(reparseOriginal, readFileSync(reparse.bridgePath), { flag: "wx" });
    unlinkSync(reparse.bridgePath);
    try {
      symlinkSync(reparseOriginal, reparse.bridgePath, "file");
      rejects(
        () => collectShowAsioInputRecords({ ...reparse, applicationPath: reparse.appPath, bridgePath: reparse.bridgePath, expectedBridgeSha256: reparse.bridgeHash, inventory: reparse.inventory, exportInspector: inspect }),
        /symbolic link|reparse point|ordinary regular file/,
        "reparse/symlink bridge input is rejected",
      );
    } catch (error) {
      if (!['EPERM', 'EACCES'].includes(error?.code)) throw error;
      skips.push("reparse fixture creation denied with " + error.code);
    }
    pass(!existsSync(resolve(valid.workspace, "target/release")), "self-test never creates or touches normal target/release");
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
  console.log("Show-ASIO runtime staging self-test passed: " + assertions + " assertions; Cargo/native/process-stop=NOT_RUN");
  for (const skip of skips) console.warn("SKIP (visible, not counted): " + skip);
}

async function main(argv = process.argv.slice(2)) {
  if (argv.length === 1 && argv[0] === "--self-test") {
    await runSelfTest();
    return;
  }
  throw new Error("prepare-show-asio-runtime.mjs is import-only; usage: prepare-show-asio-runtime.mjs --self-test");
}

if (resolve(process.argv[1] ?? "") === fileURLToPath(import.meta.url)) {
  main().catch((error) => {
    console.error("[show-asio-prepare] " + (error instanceof Error ? error.message : String(error)));
    process.exitCode = 1;
  });
}
