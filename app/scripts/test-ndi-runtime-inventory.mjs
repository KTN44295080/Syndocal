import assert from "node:assert/strict";
import test from "node:test";
import { createHash } from "node:crypto";
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { canonicalPinnedNdiRuntime, resolvePinnedNdiRuntimeDll } from "./windows-runtime-inventory.mjs";
import { inspectAuthenticodeBytes, ndiVendorCertificateThumbprint, validateWindowsSigningState, verifyWindowsSigningComponent } from "./sign-windows-release-state.mjs";

test("approved NDI redistributable retains its vendor signature in either release signing state", {
  skip: process.platform !== "win32" || !process.env.SYNDOCAL_NDI_TEST_DLL,
}, () => {
  const bytes = readFileSync(process.env.SYNDOCAL_NDI_TEST_DLL);
  assert.equal(createHash("sha256").update(bytes).digest("hex"), canonicalPinnedNdiRuntime.dll.sha256);
  const vendor = { path: "payload/Processing.NDI.Lib.x64.dll", sha256: canonicalPinnedNdiRuntime.dll.sha256,
    ownership: "upstream", authenticode: "valid", certificateThumbprint: ndiVendorCertificateThumbprint };
  const actual = inspectAuthenticodeBytes(bytes);
  for (const state of ["declared-unsigned", "signpath-signed"]) {
    const projectThumbprint = "A".repeat(40);
    const projectSigned = state === "signpath-signed";
    const declaration = { schemaVersion: 1, state, unsignedReason: projectSigned ? null : "Signing approval pending", signpath: projectSigned
      ? { organizationId: "org", projectSlug: "project", signingPolicySlug: "release", certificateThumbprint: projectThumbprint,
        requestIds: ["11111111-1111-4111-8111-111111111111", "22222222-2222-4222-8222-222222222222"] } : null,
      components: ["installer/setup.exe", "payload/syndocal.exe", "payload/uninstall.exe"].map(path => ({ path,
        ownership: "project", sha256: "a".repeat(64), authenticode: projectSigned ? "valid" : "not-signed",
        certificateThumbprint: projectSigned ? projectThumbprint : null })).concat([vendor]) };
    validateWindowsSigningState(declaration);
    verifyWindowsSigningComponent(declaration, vendor.path, bytes, () => actual);
    for (const patch of [{ status: "HashMismatch" }, { status: "NotSigned", thumbprint: null }, { timestamped: false },
      { thumbprint: projectThumbprint }, { subject: "CN=Evil Vizrt AG" }]) {
      assert.throws(() => verifyWindowsSigningComponent(declaration, vendor.path, bytes, () => ({ ...actual, ...patch })), /Vizrt AG Authenticode/);
    }
  }
});

test("NDI build source selection verifies bytes and never falls back from an explicit override", () => {
  const root = mkdtempSync(join(tmpdir(), "syndocal-ndi-source-"));
  try {
    // Only the file identity is synthetic; the production source resolver runs unchanged.
    const bytes = Buffer.alloc(128);
    bytes.write("MZ"); bytes.writeUInt32LE(0x40, 0x3c);
    bytes.write("PE\0\0", 0x40); bytes.writeUInt16LE(0x8664, 0x44);
    bytes.writeUInt16LE(0x2022, 0x56); bytes.writeUInt16LE(0x20b, 0x58);
    const runtime = { ...canonicalPinnedNdiRuntime.dll, byte_size: bytes.length,
      sha256: createHash("sha256").update(bytes).digest("hex") };
    const inventory = { ndi_runtime: { dll: runtime } };
    const standard = join(root, "NDI", "NDI 6 Runtime", "v6");
    mkdirSync(standard, { recursive: true });
    writeFileSync(join(standard, runtime.filename), bytes);
    const environment = { ProgramFiles: root };
    assert.equal(resolvePinnedNdiRuntimeDll({ environment, inventory }).directory, standard);
    for (const key of ["SYNDOCAL_NDI_RUNTIME_DIR", "NDI_RUNTIME_DIR_V6", "NDI_SDK_DIR"]) {
      for (const value of ["", "relative", " " + standard]) {
        assert.throws(() => resolvePinnedNdiRuntimeDll({ environment: { ...environment, [key]: value }, inventory }), /must select one absolute/);
      }
      assert.throws(() => resolvePinnedNdiRuntimeDll({ environment: { ...environment, [key]: join(root, "missing") }, inventory }), /No fallback is permitted/);
    }
    assert.equal(resolvePinnedNdiRuntimeDll({ environment: { SYNDOCAL_NDI_RUNTIME_DIR: standard, NDI_RUNTIME_DIR_V6: "invalid" }, inventory }).directory, standard);
    const invalid = join(root, "invalid"); mkdirSync(invalid);
    writeFileSync(join(invalid, runtime.filename), Buffer.alloc(128));
    assert.throws(() => resolvePinnedNdiRuntimeDll({ environment: { ...environment, NDI_RUNTIME_DIR_V6: invalid }, inventory }), /SHA-256|PE/);
  } finally { rmSync(root, { recursive: true, force: true }); }
});
