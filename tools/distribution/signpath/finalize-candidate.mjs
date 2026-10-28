import assert from "node:assert/strict";
import { createHash, generateKeyPairSync, sign } from "node:crypto";
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { verifyWindowsSigningTree } from "../../../app/scripts/sign-windows-release-state.mjs";
import { parseStrictJson } from "../../../app/scripts/strict-json.mjs";
import { canonicalDecodedUpdaterUrlPath, parseTauriMinisignPublicKey, verifyTauriUpdaterPayload } from "../../../app/scripts/updater-evidence-crypto.mjs";

const sha256 = (bytes) => createHash("sha256").update(bytes).digest("hex");
export function updaterConfiguration(env = process.env) {
  const names = ["TAURI_PRIVATE_KEY", "TAURI_PRIVATE_KEY_PASSWORD", "SYNDOCAL_UPDATE_PUBKEY", "SYNDOCAL_UPDATE_ENDPOINT", "SYNDOCAL_UPDATE_CHANNEL", "SYNDOCAL_UPDATE_PUBLIC_KEY_FINGERPRINT", "SYNDOCAL_UPDATE_DOWNLOAD_BASE"];
  if (names.some((name) => !env[name])) return null;
  const key = parseTauriMinisignPublicKey(Buffer.from(env.SYNDOCAL_UPDATE_PUBKEY), { expectedFingerprint: env.SYNDOCAL_UPDATE_PUBLIC_KEY_FINGERPRINT });
  if (!["stable", "beta", "nightly"].includes(env.SYNDOCAL_UPDATE_CHANNEL)) throw new Error("Invalid production updater channel.");
  canonicalDecodedUpdaterUrlPath(env.SYNDOCAL_UPDATE_ENDPOINT, "Production updater endpoint");
  const base = env.SYNDOCAL_UPDATE_DOWNLOAD_BASE;
  canonicalDecodedUpdaterUrlPath(base + "/probe", "Production updater download base");
  if (new URL(base).pathname.split("/").at(-1) !== env.SYNDOCAL_UPDATE_CHANNEL) throw new Error("Updater download base must end in the exact channel.");
  return { key, endpoint: env.SYNDOCAL_UPDATE_ENDPOINT, channel: env.SYNDOCAL_UPDATE_CHANNEL, base };
}

export function verifyFinalUpdater({ payloadBytes, filename, signatureBytes, configuration, version }) {
  const manifest = { version, platforms: { "windows-x86_64": { signature: signatureBytes.toString("utf8").trim(), url: configuration.base + "/" + encodeURIComponent(filename) } } };
  verifyTauriUpdaterPayload({ payloadBytes, artifactFilename: filename, signatureEvidenceBytes: signatureBytes, publicKey: configuration.key, updaterManifest: manifest, target: "windows-x86_64", channel: configuration.channel });
  return manifest;
}

async function finalize(directory) {
  if (process.env.GITHUB_ACTIONS !== "true" || process.env.RUNNER_ENVIRONMENT !== "github-hosted") throw new Error("Final CI provenance requires a GitHub-hosted Actions runner.");
  const root = resolve(directory);
  const installers = readdirSync(root).filter((file) => /^Syndocal_.+_x64-setup\.exe$/u.test(file));
  if (installers.length !== 1) throw new Error("Expected exactly one final NSIS installer.");
  const filename = installers[0];
  const installer = join(root, filename);
  const state = parseStrictJson(readFileSync(join(root, "windows-signing.json"), "utf8"), "Final signing declaration");
  verifyWindowsSigningTree(state, { installer, payloadRoot: join(root, "installed") });
  const payload = readFileSync(installer);
  const config = updaterConfiguration();
  let updater;
  if (config) {
    // The inspected application never receives updater signing secrets.
    const inspectionEnv = Object.fromEntries(Object.entries(process.env).filter(([name]) => !/^TAURI_(?:SIGNING_)?PRIVATE_KEY/iu.test(name)));
    const embedded = parseStrictJson(execFileSync(join(root, "installed", "syndocal.exe"), ["--print-updater-release-identity"], { encoding: "utf8", windowsHide: true, timeout: 30000, env: inspectionEnv }).trim(), "Embedded production updater identity");
    assert.deepEqual(embedded, { endpoint: config.endpoint, channel: config.channel, publicKeyFingerprint: config.key.fingerprint }, "Built application must embed the exact production updater identity.");
    const version = JSON.parse(readFileSync("app/package.json", "utf8")).version;
    const manifest = verifyFinalUpdater({ payloadBytes: payload, filename, signatureBytes: readFileSync(installer + ".sig"), configuration: config, version });
    writeFileSync(join(root, "updater.json"), JSON.stringify(manifest, null, 2) + "\n", { flag: "wx" });
    writeFileSync(join(root, "updater.pub"), config.key.canonicalText + "\n", { flag: "wx" });
    updater = { state: "production-signature-verified", endpoint: config.endpoint, channel: config.channel, publicKeyFingerprint: config.key.fingerprint, payloadSha256: sha256(payload), signatureSha256: sha256(readFileSync(installer + ".sig")) };
  } else {
    if (existsSync(installer + ".sig")) throw new Error("Updater signature without complete production configuration is rejected.");
    updater = { state: "missing-production-updater-configuration", publishable: false };
  }
  const commit = execFileSync("git", ["rev-parse", "HEAD"], { encoding: "utf8" }).trim();
  if (commit !== process.env.GITHUB_SHA) throw new Error("Source commit differs from the workflow origin.");
  const candidate = { schemaVersion: 1, productVersion: JSON.parse(readFileSync("app/package.json", "utf8")).version, commit, ref: process.env.GITHUB_REF, repository: process.env.GITHUB_REPOSITORY, workflow: ".github/workflows/windows-release-signpath.yml", runId: process.env.GITHUB_RUN_ID, runAttempt: process.env.GITHUB_RUN_ATTEMPT, authenticode: state.state, installer: { filename, sha256: sha256(payload) }, updater, releaseAcceptance: "pending-native-clean-machine-update-license-and-publication-gates" };
  writeFileSync(join(root, "candidate.json"), JSON.stringify(candidate, null, 2) + "\n", { flag: "wx" });
  writeFileSync(join(root, "SIGNING-STATE.txt"), state.state + "\n" + (state.unsignedReason ?? "SignPath payload and installer requests verified against actual component signatures.") + "\nUpdater: " + updater.state + "\nCandidate only; publication is not authorized by this workflow.\n", { flag: "wx" });
  const files = readdirSync(root, { withFileTypes: true }).filter((entry) => entry.isFile() && entry.name !== "SHA256SUMS").map((entry) => entry.name).sort();
  writeFileSync(join(root, "SHA256SUMS"), files.map((file) => sha256(readFileSync(join(root, file))) + "  " + file).join("\n") + "\n", { flag: "wx" });
  console.log(JSON.stringify(candidate));
}

const direct = process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url);
if (direct && process.argv[2] === "--self-test") {
  let count = 0;
  assert.equal(updaterConfiguration({}), null); count++;
  assert.equal(updaterConfiguration({ TAURI_PRIVATE_KEY: "partial" }), null); count++;
  assert.throws(() => updaterConfiguration(Object.fromEntries(["TAURI_PRIVATE_KEY", "TAURI_PRIVATE_KEY_PASSWORD", "SYNDOCAL_UPDATE_PUBKEY", "SYNDOCAL_UPDATE_ENDPOINT", "SYNDOCAL_UPDATE_CHANNEL", "SYNDOCAL_UPDATE_PUBLIC_KEY_FINGERPRINT", "SYNDOCAL_UPDATE_DOWNLOAD_BASE"].map((name) => [name, "invalid"])))); count++;
  assert.throws(() => verifyFinalUpdater({ payloadBytes: Buffer.from("changed bytes"), filename: "Syndocal_1.2.0_x64-setup.exe", signatureBytes: Buffer.from("unverified"), configuration: { key: {}, base: "https://example.invalid/stable", channel: "stable" }, version: "1.2.0" })); count++;
  // Fresh ephemeral keys stay in memory; no fixture or production secret is read.
  const pair = generateKeyPairSync("ed25519");
  const id = Buffer.from("0102030405060708", "hex");
  const packet = Buffer.concat([Buffer.from("Ed"), id, pair.publicKey.export({ format: "der", type: "spki" }).subarray(-32)]);
  const publicText = Buffer.from("untrusted comment: minisign public key: " + Buffer.from(id).reverse().toString("hex").toUpperCase() + "\n" + packet.toString("base64") + "\n").toString("base64");
  const key = parseTauriMinisignPublicKey(Buffer.from(publicText));
  const filename = "Syndocal_1.2.0_x64-setup.exe";
  const payloadBytes = Buffer.from("exact final Authenticode installer bytes");
  const signed = sign(null, payloadBytes, pair.privateKey);
  const trusted = "timestamp:1793142000\tfile:" + filename;
  const signatureBytes = Buffer.from(Buffer.from("untrusted comment: signature from tauri secret key\n" + Buffer.concat([Buffer.from("Ed"), id, signed]).toString("base64") + "\ntrusted comment: " + trusted + "\n" + sign(null, Buffer.concat([signed, Buffer.from(trusted)]), pair.privateKey).toString("base64") + "\n").toString("base64"));
  const args = { payloadBytes, filename, signatureBytes, configuration: { key, base: "https://example.invalid/stable", channel: "stable" }, version: "1.2.0" };
  assert.equal(verifyFinalUpdater(args).version, "1.2.0"); count++;
  assert.throws(() => verifyFinalUpdater({ ...args, payloadBytes: Buffer.concat([payloadBytes, Buffer.from("changed after signing")]) }), /cryptographic/); count++;
  assert.throws(() => verifyFinalUpdater({ ...args, filename: "Syndocal_1.2.1_x64-setup.exe" }), /filename/); count++;
  assert.throws(() => verifyFinalUpdater({ ...args, configuration: { ...args.configuration, channel: "beta" } }), /channel/); count++;
  console.log(`Candidate finalizer self-test passed: ${count} assertions`);
} else if (direct) {
  if (process.argv.length !== 3) throw new Error("Usage: node tools/distribution/signpath/finalize-candidate.mjs <candidate-directory>");
  await finalize(process.argv[2]);
}
