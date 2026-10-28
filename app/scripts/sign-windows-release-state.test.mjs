import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { collectWindowsSigningComponents, validateWindowsSigningState, verifyWindowsSigningComponent, verifyWindowsSigningTree } from "./sign-windows-release-state.mjs";

const hash = (bytes) => createHash("sha256").update(bytes).digest("hex");
const root = mkdtempSync(join(tmpdir(), "syndocal-signing-contract-"));
let assertions = 0;
const rejects = (fn, pattern) => { assert.throws(fn, pattern); assertions += 1; };
const pass = (fn) => { fn(); assertions += 1; };
try {
  const payloadRoot = join(root, "payload");
  mkdirSync(payloadRoot);
  const installer = join(root, "Syndocal_1.2.0-rc.1_x64-setup.exe");
  const bytes = Buffer.from("MZ executable fixture");
  const runtime = Buffer.from("MZ pinned upstream fixture");
  writeFileSync(installer, bytes);
  writeFileSync(join(payloadRoot, "syndocal.exe"), bytes);
  writeFileSync(join(payloadRoot, "uninstall.exe"), bytes);
  writeFileSync(join(payloadRoot, "runtime.dll"), runtime);
  writeFileSync(join(payloadRoot, "ffmpeg.exe"), runtime);
  const inventory = { runtime_dlls: [{ filename: "runtime.dll", sha256: hash(runtime) }], recording_encoder: { filename: "ffmpeg.exe", sha256: hash(runtime) } };
  const records = collectWindowsSigningComponents({ installer, payloadRoot, inventory });
  const unsigned = { schemaVersion: 1, state: "declared-unsigned", unsignedReason: "SignPath approval not yet available", signpath: null,
    components: records.map(({ path, sha256, ownership }) => ({ path, sha256, ownership, authenticode: "not-signed", certificateThumbprint: null })) };
  const options = { installer, payloadRoot, inventory, inspect: () => ({ status: "NotSigned", thumbprint: null }) };
  pass(() => verifyWindowsSigningTree(unsigned, options));
  const thumbprint = "A".repeat(40);
  const signed = structuredClone(unsigned);
  signed.state = "signpath-signed";
  signed.unsignedReason = null;
  signed.signpath = { organizationId: "org", projectSlug: "Syndocal.Project", signingPolicySlug: "release.policy", requestIds: ["11111111-1111-4111-8111-111111111111", "22222222-2222-4222-8222-222222222222"], certificateThumbprint: thumbprint };
  for (const c of signed.components.filter((c) => c.ownership === "project")) { c.authenticode = "valid"; c.certificateThumbprint = thumbprint; }
  const signedInspect = (value) => value.equals(runtime) ? { status: "NotSigned", thumbprint: null }
    : { status: "Valid", thumbprint, subject: "CN=SignPath Foundation, O=SignPath Foundation", timestamped: true };
  pass(() => verifyWindowsSigningTree(signed, { ...options, inspect: signedInspect }));
  const mutateReject = (base, mutate, pattern) => { const value = structuredClone(base); mutate(value); rejects(() => verifyWindowsSigningTree(value, { ...options, inspect: base === signed ? signedInspect : options.inspect }), pattern); };
  rejects(() => validateWindowsSigningState(undefined), /must be an object/);
  mutateReject(unsigned, (s) => { s.unsignedReason = null; }, /explicit reason/);
  mutateReject(unsigned, (s) => { s.state = "unsigned"; }, /supported state/);
  mutateReject(unsigned, (s) => { s.schemaVersion = 2; }, /schemaVersion/);
  mutateReject(unsigned, (s) => { s.signpath = {}; }, /no SignPath claim/);
  mutateReject(unsigned, (s) => { s.components[0].authenticode = "valid"; }, /Mixed/);
  mutateReject(unsigned, (s) => { s.components[0].sha256 = "0".repeat(64); }, /SHA-256 mismatch/);
  mutateReject(unsigned, (s) => { s.components[0].certificateThumbprint = thumbprint; }, /Mixed/);
  mutateReject(signed, (s) => { s.signpath = null; }, /must be an object/);
  mutateReject(signed, (s) => { s.signpath.requestIds = []; }, /request IDs/);
  mutateReject(signed, (s) => { s.signpath.requestIds[0] = "-".repeat(36); }, /request IDs/);
  mutateReject(signed, (s) => { s.signpath.requestIds[1] = s.signpath.requestIds[0]; }, /request IDs/);
  mutateReject(signed, (s) => { s.components.find((c) => c.path === "payload/syndocal.exe").authenticode = "not-signed"; }, /Mixed/);
  mutateReject(signed, (s) => { s.components = s.components.filter((c) => c.path !== "payload/uninstall.exe"); }, /generated uninstaller/);
  mutateReject(signed, (s) => { s.components.find((c) => c.ownership === "upstream").authenticode = "valid"; }, /Mixed/);
  mutateReject(unsigned, (s) => { s.components.push(structuredClone(s.components[0])); }, /duplicated/);
  mutateReject(unsigned, (s) => { s.components[0].path = "payload/../bad.exe"; }, /invalid/);
  mutateReject(unsigned, (s) => { s.components[0].extra = true; }, /keys are not exact/);
  mutateReject(unsigned, (s) => { s.components.find((c) => c.ownership === "upstream").ownership = "project"; }, /ownership does not match/);
  for (const result of [
    { status: "UnknownError", thumbprint: null },
    { status: "HashMismatch", thumbprint },
    { status: "Valid", thumbprint: "B".repeat(40), subject: "CN=SignPath Foundation", timestamped: true },
    { status: "Valid", thumbprint, subject: "Local PFX", timestamped: true },
    { status: "Valid", thumbprint, subject: "CN=Evil SignPath Foundation, O=SignPath Foundation", timestamped: true },
    { status: "Valid", thumbprint, subject: "CN=SignPath Foundation", timestamped: false },
  ]) rejects(() => verifyWindowsSigningComponent(signed, "payload/syndocal.exe", bytes, () => result), /SignPath Foundation/);
  rejects(() => verifyWindowsSigningComponent(unsigned, "payload/syndocal.exe", bytes, signedInspect), /explicitly unsigned/);
  writeFileSync(join(payloadRoot, "surprise.dll"), bytes);
  rejects(() => verifyWindowsSigningTree(unsigned, options), /Unapproved PE component/);
  rmSync(join(payloadRoot, "surprise.dll"));
  for (const missing of ["runtime.dll", "ffmpeg.exe", "uninstall.exe"]) {
    rmSync(join(payloadRoot, missing));
    rejects(() => verifyWindowsSigningTree(unsigned, options), /missing required PE component/);
    writeFileSync(join(payloadRoot, missing), missing === "uninstall.exe" ? bytes : runtime);
  }
  writeFileSync(join(payloadRoot, "runtime.dll"), bytes);
  rejects(() => verifyWindowsSigningTree(unsigned, options), /Pinned upstream runtime bytes changed/);
  writeFileSync(join(payloadRoot, "runtime.dll"), runtime);
  writeFileSync(installer, Buffer.from("MZ changed after updater signature"));
  rejects(() => verifyWindowsSigningTree(unsigned, options), /SHA-256 mismatch/);
  console.log("Windows release signing state tests passed: " + assertions + " assertions");
} finally {
  rmSync(root, { recursive: true, force: true });
}
