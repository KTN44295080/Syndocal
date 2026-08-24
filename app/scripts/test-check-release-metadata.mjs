import assert from "node:assert/strict";
import { createHash, generateKeyPairSync, sign as signMessage } from "node:crypto";
import { execFileSync } from "node:child_process";
import {
  existsSync,
  mkdtempSync,
  mkdirSync,
  linkSync,
  readFileSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import Ajv from "ajv";
import {
  authoritativeMemberPackageNames,
  collectGitCandidateState,
  compareSemver,
  expectedVersion,
  main,
  parseCli,
  parseCargoPackageIdentity,
  parseLockPackages,
  parseRuntimeUpdaterIdentity,
  parseSemver,
  parseWorkspaceMembers,
  readVerifiedEvidenceFile,
  readmeProductLine,
  readmeWindowsInstallerLine,
  requiredWorkspaceMembers,
  validateCandidateEvidence,
  validateStaticReleaseMetadata,
  windowsInstallerNames,
  withMaterializedVerifiedExecutable,
} from "./check-release-metadata.mjs";

const scriptDir = dirname(fileURLToPath(import.meta.url));
const schema = JSON.parse(readFileSync(resolve(scriptDir, "../../qa/release/release-evidence-manifest.schema.json"), "utf8"));
const validateSchema = new Ajv({ allErrors: true, strict: true }).compile(schema);
const tempRoot = mkdtempSync(join(tmpdir(), "syndocal-release-evidence-"));
const evidenceRoot = join(tempRoot, "evidence");
mkdirSync(evidenceRoot);

const version = "1.2.0-rc.1";
const head = "a".repeat(40);
const previousCommit = "c".repeat(40);
const target = "windows-x86_64";
const endpoint = "https://releases.example.invalid/syndocal/beta/latest.json";
const minisignKeyIdBytes = Buffer.from("bc11804b6c0d8c26", "hex");
const fixtureKeyPair = generateKeyPairSync("ed25519");
const fixturePublicDer = fixtureKeyPair.publicKey.export({ format: "der", type: "spki" });
const minisignPublicPacket = Buffer.concat([Buffer.from("Ed", "ascii"), minisignKeyIdBytes, fixturePublicDer.subarray(-32)]);
const minisignKeyId = Buffer.from(minisignKeyIdBytes).reverse().toString("hex").toUpperCase();
const publicKeyText = `untrusted comment: minisign public key: ${minisignKeyId}\n${minisignPublicPacket.toString("base64")}\n`;
const publicKey = Buffer.from(publicKeyText, "utf8").toString("base64");
const executableName = "syndocal.exe";
const payloadName = `Syndocal_${version}_x64-setup.exe`;
const executableBytes = Buffer.from(`fixture\0${endpoint}\0${publicKey}`, "utf8");
const payloadBytes = Buffer.from("signed release updater payload fixture", "utf8");
const makeTauriSignature = (bytes, privateKey, keyIdBytes, filename) => {
  const payloadDigest = createHash("blake2b512").update(bytes).digest();
  const payloadSignature = signMessage(null, payloadDigest, privateKey);
  const signaturePacket = Buffer.concat([Buffer.from("ED", "ascii"), keyIdBytes, payloadSignature]);
  const trustedComment = `timestamp:1787284788\tfile:${filename}`;
  const trustedCommentSignature = signMessage(
    null,
    Buffer.concat([payloadSignature, Buffer.from(trustedComment, "utf8")]),
    privateKey,
  );
  const signatureText = [
    "untrusted comment: signature from tauri secret key",
    signaturePacket.toString("base64"),
    `trusted comment: ${trustedComment}`,
    trustedCommentSignature.toString("base64"),
    "",
  ].join("\n");
  return Buffer.from(signatureText, "utf8").toString("base64");
};
const signature = makeTauriSignature(payloadBytes, fixtureKeyPair.privateKey, minisignKeyIdBytes, payloadName);
const signatureBytes = Buffer.from(`${signature}\n`, "utf8");
const tauriSignerFixtureName = `Syndocal_${version}_tauri-signer-fixture.bin`;
const tauriSignerFixtureRoot = resolve(scriptDir, "fixtures/release");
const tauriSignerFixtureBytes = readFileSync(join(tauriSignerFixtureRoot, tauriSignerFixtureName));
const tauriSignerFixtureSignatureBytes = readFileSync(join(tauriSignerFixtureRoot, `${tauriSignerFixtureName}.sig`));
const tauriSignerFixtureSignature = tauriSignerFixtureSignatureBytes.toString("utf8").trim();
const tauriSignerFixturePublicKeyBytes = readFileSync(join(tauriSignerFixtureRoot, "tauri-signer-public.key"));
const tauriSignerFixturePublicKey = tauriSignerFixturePublicKeyBytes.toString("utf8").trim();
const updaterManifest = {
  version,
  platforms: {
    [target]: {
      signature,
      url: `https://releases.example.invalid/syndocal/beta/${payloadName}`,
    },
  },
};
const updaterManifestBytes = Buffer.from(JSON.stringify(updaterManifest, null, 2), "utf8");
const hash = (bytes) => createHash("sha256").update(bytes).digest("hex");

const decodedSignatureLines = Buffer.from(signature, "base64").toString("utf8").trimEnd().split("\n");
const tamperedSignaturePacket = Buffer.from(decodedSignatureLines[1], "base64");
tamperedSignaturePacket[tamperedSignaturePacket.length - 1] ^= 0x01;
decodedSignatureLines[1] = tamperedSignaturePacket.toString("base64");
const tamperedSignature = Buffer.from(`${decodedSignatureLines.join("\n")}\n`, "utf8").toString("base64");
const tamperedSignatureBytes = Buffer.from(`${tamperedSignature}\n`, "utf8");
const otherKeyPair = generateKeyPairSync("ed25519");
const otherKeyIdBytes = Buffer.from("0123456789abcdef", "hex");
const otherKeySignature = makeTauriSignature(payloadBytes, otherKeyPair.privateKey, otherKeyIdBytes, payloadName);
const otherKeySignatureBytes = Buffer.from(`${otherKeySignature}\n`, "utf8");

writeFileSync(join(evidenceRoot, executableName), executableBytes);
writeFileSync(join(evidenceRoot, payloadName), payloadBytes);
writeFileSync(join(evidenceRoot, `${payloadName}.sig`), signatureBytes);
writeFileSync(join(evidenceRoot, "tampered.sig"), tamperedSignatureBytes);
writeFileSync(join(evidenceRoot, "other-key.sig"), otherKeySignatureBytes);
const wrongSignatureBytes = Buffer.from("different-untrusted-signature-0123456789-ABCDEFGHIJKLMNOPQRSTUVWXYZ\n", "utf8");
writeFileSync(join(evidenceRoot, "wrong.sig"), wrongSignatureBytes);
const privateKeyText = "-----BEGIN PRIVATE KEY-----\nfixture-only\n-----END PRIVATE KEY-----";
writeFileSync(join(evidenceRoot, "private.pub"), privateKeyText);
const tauriSecretText = "untrusted comment: minisign encrypted secret key\nRWRTQkFEX0ZJWFRVUkVfU0VDUkVUX0tFWQ==\n";
writeFileSync(join(evidenceRoot, "tauri-secret.pub"), tauriSecretText);
writeFileSync(join(evidenceRoot, "tauri-secret-base64.pub"), Buffer.from(tauriSecretText, "utf8").toString("base64"));
writeFileSync(join(evidenceRoot, "updater.json"), updaterManifestBytes);
const tamperedSignatureManifest = structuredClone(updaterManifest);
tamperedSignatureManifest.platforms[target].signature = tamperedSignature;
const tamperedSignatureManifestBytes = Buffer.from(JSON.stringify(tamperedSignatureManifest, null, 2), "utf8");
writeFileSync(join(evidenceRoot, "updater-tampered-signature.json"), tamperedSignatureManifestBytes);
const otherKeySignatureManifest = structuredClone(updaterManifest);
otherKeySignatureManifest.platforms[target].signature = otherKeySignature;
const otherKeySignatureManifestBytes = Buffer.from(JSON.stringify(otherKeySignatureManifest, null, 2), "utf8");
writeFileSync(join(evidenceRoot, "updater-other-key.json"), otherKeySignatureManifestBytes);
writeFileSync(join(evidenceRoot, "syndocal.pub"), `${publicKey}\n`);
writeFileSync(join(evidenceRoot, tauriSignerFixtureName), tauriSignerFixtureBytes);
writeFileSync(join(evidenceRoot, `${tauriSignerFixtureName}.sig`), tauriSignerFixtureSignatureBytes);
writeFileSync(join(evidenceRoot, "tauri-signer-public.key"), tauriSignerFixturePublicKeyBytes);

const tauriSignerUpdaterManifest = {
  version,
  platforms: {
    [target]: {
      signature: tauriSignerFixtureSignature,
      url: `https://releases.example.invalid/syndocal/beta/${tauriSignerFixtureName}`,
    },
  },
};
const tauriSignerUpdaterManifestBytes = Buffer.from(JSON.stringify(tauriSignerUpdaterManifest, null, 2), "utf8");
writeFileSync(join(evidenceRoot, "updater-tauri-signer.json"), tauriSignerUpdaterManifestBytes);

const manifestPath = join(evidenceRoot, "release-evidence.json");
const baseManifest = {
  schemaVersion: 1,
  productVersion: version,
  tag: `v${version}`,
  commit: head,
  previousVersion: "1.2.0-alpha.1",
  previousTag: "v1.2.0-alpha.1",
  updater: {
    channel: "beta",
    endpoint,
    manifestPath: "updater.json",
    manifestSha256: hash(updaterManifestBytes),
    publicKeyPath: "syndocal.pub",
    publicKeyFingerprint: hash(Buffer.from(publicKey, "utf8")),
  },
  artifacts: [
    {
      role: "windows-executable",
      target,
      filename: executableName,
      path: executableName,
      sha256: hash(executableBytes),
    },
    {
      role: "updater-payload",
      target,
      filename: payloadName,
      path: payloadName,
      sha256: hash(payloadBytes),
      signaturePath: `${payloadName}.sig`,
      signatureSha256: hash(signatureBytes),
    },
  ],
};
writeFileSync(manifestPath, JSON.stringify(baseManifest, null, 2));

let inspectedUpdater = null;
const options = {
  manifestPath,
  manifestRecord: readVerifiedEvidenceFile(evidenceRoot, "release-evidence.json", "release evidence manifest"),
  productVersion: version,
  headCommit: head,
  clean: true,
  currentTagCommit: head,
  previousTagCommit: previousCommit,
  resolveTag: (tag) => {
    if (tag === `v${version}`) return head;
    if (tag === "v1.2.0-alpha.1") return previousCommit;
    throw new Error(`unknown tag ${tag}`);
  },
  productTags: ["v1.1.0", "v1.2.0-alpha.1", `v${version}`],
  inspectExecutable: (_path, bytes, updater) => {
    inspectedUpdater = updater;
    assert.deepEqual(bytes, executableBytes);
    return { productVersion: version };
  },
};

let assertions = 0;
const pass = (condition, message) => {
  assert.ok(condition, message);
  assertions += 1;
};
const rejects = (mutate, pattern, optionMutate) => {
  const manifest = structuredClone(baseManifest);
  mutate(manifest);
  const candidateOptions = { ...options };
  optionMutate?.(candidateOptions);
  assert.throws(() => validateCandidateEvidence(manifest, candidateOptions), pattern);
  assertions += 1;
};
const git = (args, cwd) => execFileSync("git", args, {
  cwd,
  encoding: "utf8",
  stdio: ["ignore", "pipe", "pipe"],
}).trim();

try {
  pass(parseSemver(version)?.prerelease.join(".") === "rc.1", "RC SemVer parses");
  assert.throws(() => parseCli([], version), /require --release-candidate --manifest evidence/);
  assertions += 1;
  pass(
    parseCli(["--release-candidate", "--manifest", manifestPath], version).candidate,
    "RC command accepts only explicit candidate evidence mode",
  );
  pass(!parseCli([], "1.2.0-alpha.1").candidate, "non-RC development version retains the static metadata gate");
  pass(
    parseRuntimeUpdaterIdentity(JSON.stringify({
      endpoint,
      channel: "beta",
      publicKeyFingerprint: hash(Buffer.from(publicKey, "utf8")),
    })).channel === "beta",
    "runtime updater diagnostic parser accepts the exact one-line identity",
  );
  assert.throws(
    () => parseRuntimeUpdaterIdentity(`${JSON.stringify({ endpoint, channel: "beta", publicKeyFingerprint: "0".repeat(64) })}\nextra`),
    /exactly one JSON line/,
  );
  assertions += 1;
  assert.throws(
    () => parseRuntimeUpdaterIdentity(JSON.stringify({ endpoint, channel: "beta", publicKeyFingerprint: "0".repeat(64), extra: true })),
    /fields are not exact/,
  );
  assertions += 1;
  let materializedPath = null;
  const materializedBytes = Buffer.from("verified executable bytes", "utf8");
  const materializedResult = withMaterializedVerifiedExecutable(materializedBytes, (path) => {
    materializedPath = path;
    assert.deepEqual(readFileSync(path), materializedBytes);
    return "inspected";
  });
  pass(materializedResult === "inspected", "runtime inspection uses a materialized copy of verified bytes");
  pass(materializedPath !== null && !existsSync(materializedPath), "materialized runtime inspection copy is removed");
  let failedMaterializedPath = null;
  assert.throws(
    () => withMaterializedVerifiedExecutable(materializedBytes, (path) => {
      failedMaterializedPath = path;
      throw new Error("fixture inspection failure");
    }),
    /fixture inspection failure/,
  );
  pass(failedMaterializedPath !== null && !existsSync(failedMaterializedPath), "failed runtime inspection also removes its fixed copy");
  pass(parseSemver("1.2.3-alpha.01") === null, "numeric prerelease leading zero is rejected");
  pass(compareSemver("999999999999999999999.0.0", "1000000000000000000000.0.0") < 0, "large SemVer cores compare exactly");
  pass(compareSemver("1.2.3-rc.9007199254740992", "1.2.3-rc.9007199254740993") < 0, "large prerelease numbers compare exactly");
  pass(compareSemver("1.2.3+build.9", "1.2.3+build.10") === 0, "build metadata does not change precedence");
  pass(compareSemver("1.2.0-alpha.1", version) < 0, "RC is newer than alpha");
  pass(compareSemver("1.2.0-rc.1", "1.2.0") < 0, "release is newer than RC");
  pass(validateSchema(baseManifest), "canonical manifest satisfies JSON Schema");
  validateCandidateEvidence(baseManifest, options);
  assertions += 1;
  pass(inspectedUpdater?.endpoint === endpoint && inspectedUpdater?.publicKey === publicKey, "executable inspector receives exact updater identity");
  const tauriSignerManifest = structuredClone(baseManifest);
  tauriSignerManifest.updater.manifestPath = "updater-tauri-signer.json";
  tauriSignerManifest.updater.manifestSha256 = hash(tauriSignerUpdaterManifestBytes);
  tauriSignerManifest.updater.publicKeyPath = "tauri-signer-public.key";
  tauriSignerManifest.updater.publicKeyFingerprint = hash(Buffer.from(tauriSignerFixturePublicKey, "utf8"));
  tauriSignerManifest.artifacts[1].filename = tauriSignerFixtureName;
  tauriSignerManifest.artifacts[1].path = tauriSignerFixtureName;
  tauriSignerManifest.artifacts[1].sha256 = hash(tauriSignerFixtureBytes);
  tauriSignerManifest.artifacts[1].signaturePath = `${tauriSignerFixtureName}.sig`;
  tauriSignerManifest.artifacts[1].signatureSha256 = hash(tauriSignerFixtureSignatureBytes);
  validateCandidateEvidence(tauriSignerManifest, options);
  assertions += 1;
  assert.throws(
    () => main(["--release-candidate", "--manifest", manifestPath]),
    /requires a product version ending in -rc\.N/,
  );
  assertions += 1;

  rejects((m) => { m.commit = "d".repeat(40); }, /commit does not match HEAD/);
  rejects((m) => { m.tag = "v1.2.0-rc.2"; }, /version\/tag/);
  rejects(() => {}, /clean worktree/, (o) => { o.clean = false; });
  rejects((m) => { m.previousVersion = "1.1.0"; m.previousTag = "v1.1.0"; }, /latest previous/);
  rejects((m) => { m.previousVersion = version; m.previousTag = `v${version}`; }, /increase monotonically/);
  rejects(() => {}, /different commit/, (o) => { o.previousTagCommit = head; });
  rejects((m) => { m.updater.endpoint = "http://releases.example.invalid/syndocal/beta/latest.json"; }, /credential-free HTTPS/);
  rejects((m) => { m.updater.endpoint = "https://user:secret@releases.example.invalid/syndocal/beta/latest.json"; }, /credential-free HTTPS/);
  rejects((m) => { m.updater.endpoint = `${endpoint}#fragment`; }, /credential-free HTTPS/);
  rejects((m) => { m.updater.endpoint = `${endpoint}?token=secret`; }, /credential-free HTTPS/);
  rejects((m) => { m.updater.channel = "stable"; }, /channel segment/);
  rejects((m) => { m.privateNote = "-----BEGIN PRIVATE KEY-----"; }, /private signing key/);
  rejects((m) => { m.updater.manifestSha256 = "0".repeat(64); }, /manifest SHA-256 mismatch/);
  rejects((m) => { m.updater.publicKeyFingerprint = "0".repeat(64); }, /public-key fingerprint mismatch/);
  rejects((m) => { m.artifacts[1].path = `../${payloadName}`; }, /stay below/);
  rejects((m) => { m.artifacts[1].filename = `Other_${version}.exe`; }, /filename\/path mismatch/);
  rejects((m) => {
    m.artifacts[1].filename = payloadName.replace(version, "1.1.0");
    m.artifacts[1].path = m.artifacts[1].filename;
  }, /does not contain product version|missing/);
  rejects((m) => { m.artifacts[1].sha256 = "0".repeat(64); }, /Artifact SHA-256 mismatch/);
  rejects((m) => { m.artifacts.push(structuredClone(m.artifacts[1])); }, /Duplicate release artifact identity/);
  rejects((m) => {
    const alias = structuredClone(m.artifacts[1]);
    alias.role = "installer";
    alias.target = "linux-x86_64";
    alias.path = `.\\${payloadName}`;
    delete alias.signaturePath;
    delete alias.signatureSha256;
    m.artifacts.push(alias);
  }, /duplicates another evidence file/);
  if (process.platform === "win32") {
    rejects((m) => {
      const alias = structuredClone(m.artifacts[1]);
      alias.role = "installer";
      alias.target = "linux-x86_64";
      alias.filename = `SYNDOCAL_${version}_X64-SETUP.EXE`;
      alias.path = alias.filename;
      delete alias.signaturePath;
      delete alias.signatureSha256;
      m.artifacts.push(alias);
    }, /duplicates another evidence file/);
  }
  rejects((m) => { m.artifacts[1].signatureSha256 = "0".repeat(64); }, /signature SHA-256 mismatch/);
  rejects((m) => {
    m.artifacts[1].signaturePath = "wrong.sig";
    m.artifacts[1].signatureSha256 = hash(wrongSignatureBytes);
  }, /signature does not match the exact signature file/);
  const tamperedPayloadBytes = Buffer.from("tampered release updater payload fixture", "utf8");
  writeFileSync(join(evidenceRoot, payloadName), tamperedPayloadBytes);
  try {
    rejects((m) => {
      m.artifacts[1].sha256 = hash(tamperedPayloadBytes);
    }, /cryptographic signature verification failed/);
  } finally {
    writeFileSync(join(evidenceRoot, payloadName), payloadBytes);
  }
  rejects((m) => {
    m.updater.manifestPath = "updater-tampered-signature.json";
    m.updater.manifestSha256 = hash(tamperedSignatureManifestBytes);
    m.artifacts[1].signaturePath = "tampered.sig";
    m.artifacts[1].signatureSha256 = hash(tamperedSignatureBytes);
  }, /cryptographic signature verification failed/);
  rejects((m) => {
    m.updater.manifestPath = "updater-other-key.json";
    m.updater.manifestSha256 = hash(otherKeySignatureManifestBytes);
    m.artifacts[1].signaturePath = "other-key.sig";
    m.artifacts[1].signatureSha256 = hash(otherKeySignatureBytes);
  }, /signature key identifier\/packet is invalid/);
  rejects((m) => { m.artifacts[1].signaturePath = "syndocal.pub"; m.artifacts[1].signatureSha256 = hash(Buffer.from(`${publicKey}\n`)); }, /duplicates another evidence file/);
  rejects((m) => {
    m.updater.publicKeyPath = "private.pub";
    m.updater.publicKeyFingerprint = hash(Buffer.from(privateKeyText, "utf8"));
  }, /private-key material/);
  rejects((m) => {
    m.updater.publicKeyPath = "tauri-secret.pub";
    m.updater.publicKeyFingerprint = hash(Buffer.from(tauriSecretText, "utf8"));
  }, /private-key material/);
  rejects((m) => {
    const encodedSecret = Buffer.from(tauriSecretText, "utf8").toString("base64");
    m.updater.publicKeyPath = "tauri-secret-base64.pub";
    m.updater.publicKeyFingerprint = hash(Buffer.from(encodedSecret, "utf8"));
  }, /private-key material/);
  rejects(() => {}, /Ambiguous equivalent previous product tags/, (o) => {
    o.productTags = ["v1.1.0", "v1.2.0-alpha.1+build.1", "v1.2.0-alpha.1+build.2", `v${version}`];
  });
  rejects(() => {}, /Ambiguous equivalent previous product tags/, (o) => {
    o.productTags = ["v1.1.0", "v1.1.0+build.1", "v1.2.0-alpha.1", `v${version}`];
  });
  rejects((m) => { m.artifacts[1].target = "windows-arm64"; }, /manifest signature|platforms and updater-payload evidence|lacks an inspected/);
  rejects(() => {}, /ProductVersion mismatch/, (o) => {
    o.inspectExecutable = () => ({ productVersion: "1.2.0-alpha.1" });
  });

  const schemaInvalid = structuredClone(baseManifest);
  schemaInvalid.artifacts[1].signaturePath = "";
  pass(!validateSchema(schemaInvalid), "empty updater signature path is schema-invalid");
  const missingSignature = structuredClone(baseManifest);
  delete missingSignature.artifacts[1].signaturePath;
  delete missingSignature.artifacts[1].signatureSha256;
  pass(!validateSchema(missingSignature), "updater payload signature evidence is schema-required");
  const extraProperty = structuredClone(baseManifest);
  extraProperty.unreviewed = true;
  pass(!validateSchema(extraProperty), "unknown manifest fields fail closed");

  const queriedUpdaterManifest = structuredClone(updaterManifest);
  queriedUpdaterManifest.platforms[target].url = `${queriedUpdaterManifest.platforms[target].url}?token=secret`;
  const queriedUpdaterManifestBytes = Buffer.from(JSON.stringify(queriedUpdaterManifest, null, 2), "utf8");
  writeFileSync(join(evidenceRoot, "updater-query.json"), queriedUpdaterManifestBytes);
  rejects((m) => {
    m.updater.manifestPath = "updater-query.json";
    m.updater.manifestSha256 = hash(queriedUpdaterManifestBytes);
  }, /credential-free HTTPS/);

  const hardLinkName = `HardLink_${version}_x64-setup.exe`;
  linkSync(join(evidenceRoot, payloadName), join(evidenceRoot, hardLinkName));
  rejects((m) => {
    const alias = structuredClone(m.artifacts[1]);
    alias.role = "installer";
    alias.target = "linux-x86_64";
    alias.filename = hardLinkName;
    alias.path = hardLinkName;
    delete alias.signaturePath;
    delete alias.signatureSha256;
    m.artifacts.push(alias);
  }, /duplicates another evidence file/);

  const outsideRoot = join(tempRoot, "outside");
  mkdirSync(outsideRoot);
  writeFileSync(join(outsideRoot, payloadName), payloadBytes);
  const junction = join(evidenceRoot, "outside-link");
  try {
    symlinkSync(outsideRoot, junction, "junction");
    rejects((m) => {
      const escaped = structuredClone(m.artifacts[1]);
      escaped.role = "installer";
      escaped.target = "linux-x86_64";
      escaped.path = `outside-link/${payloadName}`;
      delete escaped.signaturePath;
      delete escaped.signatureSha256;
      m.artifacts.push(escaped);
    }, /escapes the evidence directory/);
  } catch (error) {
    if (!String(error).includes("privilege")) throw error;
  }

  const gitRoot = join(tempRoot, "git-fixture");
  mkdirSync(gitRoot);
  git(["init", "--quiet"], gitRoot);
  git(["config", "user.email", "release-fixture@syndocal.invalid"], gitRoot);
  git(["config", "user.name", "Syndocal Release Fixture"], gitRoot);
  writeFileSync(join(gitRoot, "version.txt"), "1.2.0-alpha.1\n");
  git(["add", "version.txt"], gitRoot);
  git(["commit", "--quiet", "-m", "alpha"], gitRoot);
  git(["tag", "v1.2.0-alpha.1"], gitRoot);
  const actualPrevious = git(["rev-parse", "HEAD"], gitRoot);
  writeFileSync(join(gitRoot, "version.txt"), `${version}\n`);
  git(["add", "version.txt"], gitRoot);
  git(["commit", "--quiet", "-m", "candidate"], gitRoot);
  git(["tag", `v${version}`], gitRoot);
  const actualGitState = collectGitCandidateState(gitRoot, `v${version}`, "v1.2.0-alpha.1");
  pass(actualGitState.clean, "real Git fixture is clean after the candidate commit");
  pass(actualGitState.currentTagCommit === actualGitState.headCommit, "real candidate tag resolves to HEAD");
  pass(actualGitState.previousTagCommit === actualPrevious && actualPrevious !== actualGitState.headCommit, "real previous tag resolves to a distinct commit");
  writeFileSync(join(gitRoot, "dirty.txt"), "dirty\n");
  pass(!collectGitCandidateState(gitRoot, `v${version}`, "v1.2.0-alpha.1").clean, "real Git fixture detects an untracked dirty file");

  const workspaceRoot = resolve(scriptDir, "..", "..");
  const readRepoFile = (relativePath) => readFileSync(resolve(workspaceRoot, relativePath), "utf8");
  const staticFixturePaths = [
    "Cargo.toml",
    "Cargo.lock",
    "README.md",
    ...requiredWorkspaceMembers.map((member) => `${member}/Cargo.toml`),
  ];
  let staticFixtureSnapshot = null;
  const snapshotStaticFixtures = () => {
    staticFixtureSnapshot ??= new Map(staticFixturePaths.map((path) => [path, readRepoFile(path)]));
    return staticFixtureSnapshot;
  };
  const staticRejects = (mutate, pattern, label) => {
    const files = new Map(snapshotStaticFixtures());
    mutate?.(files);
    assert.throws(
      () =>
        validateStaticReleaseMetadata((path) =>
          files.has(path) ? files.get(path) : readRepoFile(path),
        ),
      pattern,
      label,
    );
    assertions += 1;
  };
  const replaceOnce = (text, from, to) => {
    if (!text.includes(from)) throw new Error(`fixture bug: pattern not found: ${from}`);
    return text.replace(from, to);
  };
  const lockBlock = (lockText, packageName) => {
    const block = lockText
      .split(/(?=\[\[package\]\])/u)
      .find((candidate) => /^\s*name\s*=\s*"([^"]+)"/mu.exec(candidate)?.[1] === packageName);
    if (!block) throw new Error(`fixture bug: no Cargo.lock block for ${packageName}`);
    return block;
  };

  validateStaticReleaseMetadata(readRepoFile);
  assertions += 1;
  const manifestPackageNames = requiredWorkspaceMembers.map((member) =>
    parseCargoPackageIdentity(readRepoFile(`${member}/Cargo.toml`), `${member}/Cargo.toml`),
  );
  pass(
    JSON.stringify([...parseWorkspaceMembers(snapshotStaticFixtures().get("Cargo.toml"))].sort())
      === JSON.stringify([...requiredWorkspaceMembers].sort()),
    `live workspace member set matches the authoritative set order-independently (${manifestPackageNames.join(", ")})`,
  );
  pass(
    JSON.stringify(parseWorkspaceMembers('[workspace]\nmembers = [\n  # "crates/audio",\n  "crates/audio", # live\n]'))
      === JSON.stringify(["crates/audio"]),
    "commented workspace members stay inert",
  );
  pass(
    JSON.stringify(parseLockPackages('[[package]]\nname = "audio"\nversion = "1.0.0"\n\n[dependencies]\nname = "shadow"\nversion = "9.9.9"\n'))
      === JSON.stringify([{ name: "audio", version: "1.0.0" }]),
    "lock parser ignores nested dependency tables and comments",
  );

  staticRejects(
    (files) => {
      files.set("Cargo.toml", replaceOnce(files.get("Cargo.toml"), '"app/src-tauri",', '"app/src-tauri",\n    "crates/ghost",'));
    },
    /unexpected: crates\/ghost/,
    "extra workspace member is rejected",
  );
  staticRejects(
    (files) => {
      files.set("Cargo.toml", replaceOnce(files.get("Cargo.toml"), '    "crates/video",', ""));
    },
    /missing: crates\/video/,
    "missing workspace member is rejected",
  );
  staticRejects(
    (files) => {
      files.set("Cargo.toml", replaceOnce(files.get("Cargo.toml"), '"crates/audio",', '"crates/audio",\n    "crates/audio",'));
    },
    /duplicate workspace members: crates\/audio/,
    "duplicate workspace member is rejected",
  );
  for (const packageName of manifestPackageNames) {
    staticRejects(
      (files) => {
        const lockText = files.get("Cargo.lock");
        const block = lockBlock(lockText, packageName);
        files.set("Cargo.lock", lockText.replace(block, block.replace(/^(\s*version\s*=\s*)"[^"]*"/mu, '$1"9.8.7-fixture"')));
      },
      new RegExp(`Cargo\\.lock pins ${packageName} at 9\\.8\\.7-fixture`),
      `${packageName} lock version mismatch is rejected`,
    );
  }
  staticRejects(
    (files) => {
      const lockText = files.get("Cargo.lock");
      files.set("Cargo.lock", lockText.replace(lockBlock(lockText, "engine"), ""));
    },
    /Cargo\.lock is missing a package entry for engine\./,
    "missing lock entry is rejected",
  );
  staticRejects(
    (files) => {
      const lockText = files.get("Cargo.lock");
      const block = lockBlock(lockText, "io");
      files.set("Cargo.lock", lockText.replace(block, `${block}${block}`));
    },
    /package entries exist for io\./,
    "duplicate lock entry is rejected",
  );
  staticRejects(
    (files) => {
      files.set("crates/audio/Cargo.toml", replaceOnce(files.get("crates/audio/Cargo.toml"), "version.workspace = true", "# version.workspace = true"));
    },
    /does not inherit the workspace version/,
    "commented workspace inheritance is rejected",
  );
  staticRejects(
    (files) => {
      files.set("crates/audio/Cargo.toml", replaceOnce(files.get("crates/audio/Cargo.toml"), "version.workspace = true", 'version = "1.2.0-alpha.10"'));
    },
    /does not inherit the workspace version/,
    "literal version override is rejected instead of substring-matching",
  );
  staticRejects(
    (files) => {
      files.set("README.md", replaceOnce(files.get("README.md"), readmeProductLine(expectedVersion), readmeProductLine("1.2.0-alpha.9")));
    },
    /exact product line/,
    "wrong README product version is rejected",
  );
  staticRejects(
    (files) => {
      const { nsis } = windowsInstallerNames(expectedVersion);
      files.set("README.md", replaceOnce(files.get("README.md"), nsis, nsis.replace("_x64-setup.exe", "_x64-install.exe")));
    },
    /versioned NSIS name/,
    "wrong README NSIS name is rejected",
  );
  staticRejects(
    (files) => {
      const { msi } = windowsInstallerNames(expectedVersion);
      files.set("README.md", replaceOnce(files.get("README.md"), msi, msi.replace("_ja-JP.msi", ".msi")));
    },
    /versioned MSI name/,
    "wrong README MSI name is rejected",
  );
  staticRejects(
    (files) => {
      files.set("README.md", replaceOnce(files.get("README.md"), readmeProductLine(expectedVersion), `${readmeProductLine(expectedVersion)}\n${readmeProductLine(expectedVersion)}`));
    },
    /exactly one canonical product line/,
    "duplicate canonical README product line is rejected",
  );
  staticRejects(
    (files) => {
      files.set("README.md", replaceOnce(files.get("README.md"), readmeProductLine(expectedVersion), `${readmeProductLine(expectedVersion)}\n- 製品名: **Syndocal 1.2.0-alpha.9**`));
    },
    /exactly one canonical product line/,
    "canonical plus stale README product line is rejected",
  );
  staticRejects(
    (files) => {
      files.set("README.md", replaceOnce(files.get("README.md"), readmeWindowsInstallerLine(expectedVersion), `${readmeWindowsInstallerLine(expectedVersion)}\n${readmeWindowsInstallerLine(expectedVersion)}`));
    },
    /Windows installer metadata lines .*exactly one is required/,
    "duplicate canonical README installer line is rejected",
  );
  staticRejects(
    (files) => {
      files.set(
        "README.md",
        replaceOnce(
          files.get("README.md"),
          readmeWindowsInstallerLine(expectedVersion),
          `${readmeWindowsInstallerLine(expectedVersion)}\n- Windows: \`Syndocal_1.2.0-alpha.9_x64-setup.exe\` (NSIS)、\`Syndocal_1.2.0-alpha.9_x64_ja-JP.msi\``,
        ),
      );
    },
    /Windows installer metadata lines .*exactly one is required/,
    "canonical plus stale README installer line is rejected",
  );
  {
    const files = new Map(snapshotStaticFixtures());
    const proAudioBullet = "- Windows: Pro Audio MMCSS Critical + 1ms timer、macOS: USER_INTERACTIVE QoS";
    files.set(
      "README.md",
      replaceOnce(
        files.get("README.md"),
        readmeWindowsInstallerLine(expectedVersion),
        `${proAudioBullet}\n${readmeWindowsInstallerLine(expectedVersion)}`,
      ),
    );
    validateStaticReleaseMetadata((path) => (files.has(path) ? files.get(path) : readRepoFile(path)));
    assertions += 1;
  }
  pass(
    Object.isFrozen(requiredWorkspaceMembers)
      && Object.isFrozen(authoritativeMemberPackageNames)
      && requiredWorkspaceMembers.length === 8
      && requiredWorkspaceMembers.every((member) => typeof member === "string")
      && Object.keys(authoritativeMemberPackageNames).length === 8
      && Object.values(authoritativeMemberPackageNames).every((name) => typeof name === "string")
      && new Set(Object.values(authoritativeMemberPackageNames)).size === 8
      && requiredWorkspaceMembers.every((member) => Object.hasOwn(authoritativeMemberPackageNames, member)),
    "frozen authoritative member-to-package mapping covers every member with unique package names",
  );
  staticRejects(
    (files) => {
      files.set("crates/audio/Cargo.toml", replaceOnce(files.get("crates/audio/Cargo.toml"), 'name = "audio"', 'name = "audio-drift"'));
    },
    /instead of the frozen authoritative name 'audio'/,
    "member manifest package-name drift from the frozen mapping is rejected",
  );
  staticRejects(
    (files) => {
      files.set("Cargo.toml", replaceOnce(files.get("Cargo.toml"), "[workspace]", "[workspace]\n[workspace]"));
    },
    /2 active \[workspace\] sections/,
    "duplicate active [workspace] section is rejected",
  );
  staticRejects(
    (files) => {
      files.set("Cargo.toml", replaceOnce(files.get("Cargo.toml"), "[workspace.package]", '[workspace.package]\nversion = "9.9.9"\n[workspace.package]'));
    },
    /2 active \[workspace\.package\] sections/,
    "duplicate active [workspace.package] section is rejected",
  );
  staticRejects(
    (files) => {
      files.set(
        "Cargo.toml",
        replaceOnce(
          files.get("Cargo.toml"),
          '"app/src-tauri",',
          '"app/src-tauri",\n]\nmembers = [\n    "crates/audio",\n    "crates/ghost",',
        ),
      );
    },
    /2 active members assignments/,
    "second active members assignment is rejected even when the first list is exact",
  );
  staticRejects(
    (files) => {
      files.set("crates/audio/Cargo.toml", replaceOnce(files.get("crates/audio/Cargo.toml"), "version.workspace = true", 'version.workspace = true\nname = "audio-alias"'));
    },
    /2 active name assignments/,
    "duplicate active member name assignment is rejected",
  );
  staticRejects(
    (files) => {
      files.set("crates/audio/Cargo.toml", replaceOnce(files.get("crates/audio/Cargo.toml"), "[package]", '[package]\nname = "shadow-audio"\n[package]'));
    },
    /2 active \[package\] sections/,
    "duplicate active member [package] section is rejected",
  );
  assert.throws(() => parseLockPackages('[[package]]\nname = "engine"\nname = "engine-alias"\nversion = "1.0.0"'), /duplicate name assignment/);
  assertions += 1;
  assert.throws(() => parseLockPackages('[[package]]\nname = "engine"\nversion = "1.0.0"\nversion = "9.9.9"'), /duplicate version assignment/);
  assertions += 1;
  pass(
    readRepoFile("README.md").split(/\r?\n/u).includes(readmeWindowsInstallerLine(expectedVersion)),
    "canonical installer line matches the live README byte-for-byte",
  );

  console.log(`release metadata self-tests ok: ${assertions} assertion groups`);
} finally {
  rmSync(tempRoot, { recursive: true, force: true });
}
