import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { existsSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { basename, dirname, isAbsolute, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { assertSafeExternalDirectory, loadWindowsRuntimeInventory, readVerifiedRegularFile } from "./windows-runtime-inventory.mjs";
import { parseStrictJson } from "./strict-json.mjs";
import { withMaterializedVerifiedExecutable } from "./verified-materialization.mjs";

const hash = (bytes) => createHash("sha256").update(bytes).digest("hex");
const exact = (value, keys, label) => {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error(label + " must be an object.");
  assert.deepEqual(Object.keys(value).sort(), [...keys].sort(), label + " keys are not exact.");
};
const safePath = (value) => typeof value === "string" && /^(?:installer|payload)\//u.test(value)
  && !/[\\:\u0000\r\n]/u.test(value) && value.split("/").every((part) => part && part !== "." && part !== "..");

export function validateWindowsSigningState(value) {
  exact(value, ["schemaVersion", "state", "unsignedReason", "signpath", "components"], "Windows signing declaration");
  if (value.schemaVersion !== 1 || !["signpath-signed", "declared-unsigned"].includes(value.state)) throw new Error("Windows signing declaration requires schemaVersion 1 and an exact supported state.");
  if (value.state === "declared-unsigned") {
    if (typeof value.unsignedReason !== "string" || value.unsignedReason.trim().length < 8 || value.signpath !== null) throw new Error("Unsigned release requires an explicit reason and no SignPath claim.");
  } else {
    if (value.unsignedReason !== null) throw new Error("Signed release cannot declare an unsigned reason.");
    exact(value.signpath, ["organizationId", "projectSlug", "signingPolicySlug", "requestIds", "certificateThumbprint"], "SignPath provenance");
    for (const field of ["organizationId", "projectSlug", "signingPolicySlug"]) {
      if (typeof value.signpath[field] !== "string" || !/^[A-Za-z0-9_.-]+$/u.test(value.signpath[field])) throw new Error("SignPath provenance " + field + " is missing or invalid.");
    }
    if (!/^[0-9A-F]{40}$/u.test(value.signpath.certificateThumbprint)
      || !Array.isArray(value.signpath.requestIds) || value.signpath.requestIds.length < 2
      || new Set(value.signpath.requestIds).size !== value.signpath.requestIds.length
      || value.signpath.requestIds.some((id) => typeof id !== "string" || !/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/iu.test(id))) throw new Error("SignPath requires a certificate thumbprint and distinct payload and installer request IDs.");
  }
  if (!Array.isArray(value.components) || value.components.length < 2) throw new Error("Windows signing component inventory is missing.");
  const paths = new Set();
  for (const component of value.components) {
    exact(component, ["path", "sha256", "ownership", "authenticode", "certificateThumbprint"], "Windows signing component");
    if (!safePath(component.path) || paths.has(component.path.toLowerCase()) || !/\.(exe|dll)$/iu.test(component.path)
      || !/^[0-9a-f]{64}$/u.test(component.sha256) || !["project", "upstream"].includes(component.ownership)) throw new Error("Windows signing component path, hash, or ownership is invalid or duplicated.");
    paths.add(component.path.toLowerCase());
    const signed = component.ownership === "project" && value.state === "signpath-signed";
    if (component.authenticode !== (signed ? "valid" : "not-signed")
      || component.certificateThumbprint !== (signed ? value.signpath.certificateThumbprint : null)) throw new Error("Mixed or invalid component signature state is rejected: " + component.path);
  }
  if (!paths.has("payload/syndocal.exe") || value.components.filter((c) => c.path.startsWith("installer/")).length !== 1) throw new Error("Signing declaration requires exactly one installer and payload/syndocal.exe.");
  if (value.components.some((c) => (c.path.startsWith("installer/") || c.path === "payload/syndocal.exe") && c.ownership !== "project")) throw new Error("Installer and application executable must be project owned.");
  if (!value.components.some((c) => c.ownership === "project" && c.path === "payload/uninstall.exe")) throw new Error("Final NSIS release requires its generated uninstaller in the component inventory.");
  return value;
}

function signToolPath() {
  const sdkRoot = "C:/Program Files (x86)/Windows Kits/10/bin";
  const candidates = process.env.SYNDOCAL_SIGNTOOL ? [process.env.SYNDOCAL_SIGNTOOL] :
    existsSync(sdkRoot) ? readdirSync(sdkRoot).filter((name) => /^10\.0\.\d+\.0$/u.test(name)).sort().reverse().map((name) => join(sdkRoot, name, "x64", "signtool.exe")) : [];
  for (const candidate of candidates) {
    if (!isAbsolute(candidate)) continue;
    if (!existsSync(candidate) || basename(candidate).toLowerCase() !== "signtool.exe") continue;
    return readVerifiedRegularFile(candidate, "Windows SDK SignTool", { allowedRoots: [dirname(candidate)] }).realPath;
  }
  throw new Error("Signed release verification requires trusted x64 Windows SDK signtool.exe; set absolute SYNDOCAL_SIGNTOOL if needed.");
}

export function inspectAuthenticodeBytes(bytes) {
  if (process.platform !== "win32") throw new Error("Authenticode verification requires Windows.");
  return withMaterializedVerifiedExecutable(bytes, (file) => {
    const result = execFileSync("powershell.exe", ["-NoLogo", "-NoProfile", "-NonInteractive", "-Command",
      "$s=Get-AuthenticodeSignature -LiteralPath $env:SYNDOCAL_AUTHENTICODE_PATH;[pscustomobject]@{status=[string]$s.Status;thumbprint=if($s.SignerCertificate){$s.SignerCertificate.Thumbprint}else{$null};subject=if($s.SignerCertificate){$s.SignerCertificate.Subject}else{$null};timestamped=($null -ne $s.TimeStamperCertificate)}|ConvertTo-Json -Compress"],
    { encoding: "utf8", windowsHide: true, timeout: 30000, env: { ...process.env, SYNDOCAL_AUTHENTICODE_PATH: file } });
    const signature = parseStrictJson(result.trim(), "Authenticode inspection");
    if (signature.status === "Valid") {
      execFileSync(signToolPath(), ["verify", "/pa", "/all", "/tw", file], { encoding: "utf8", windowsHide: true, timeout: 30000, stdio: ["ignore", "pipe", "pipe"] });
    }
    return signature;
  });
}

export function verifyWindowsSigningComponent(declaration, path, bytes, inspect = inspectAuthenticodeBytes) {
  const component = declaration.components.find((c) => c.path === path);
  if (!component || hash(bytes) !== component.sha256) throw new Error("Windows signing final component SHA-256 mismatch: " + path);
  const result = inspect(bytes);
  if (component.authenticode === "not-signed") {
    if (result.status !== "NotSigned" || result.thumbprint !== null) throw new Error("Expected explicitly unsigned component: " + path);
  } else if (result.status !== "Valid" || result.thumbprint?.toUpperCase() !== component.certificateThumbprint
    || result.timestamped !== true || !/(?:^|,\s*)CN=SignPath Foundation(?:,|$)/u.test(result.subject ?? "")) {
    throw new Error("Component lacks valid timestamped SignPath Foundation Authenticode: " + path);
  }
}

export function collectWindowsSigningComponents({ installer, payloadRoot, inventory = loadWindowsRuntimeInventory() }) {
  const root = assertSafeExternalDirectory(resolve(payloadRoot), "signing payload root");
  const pinned = new Map([...inventory.runtime_dlls, inventory.recording_encoder].map((entry) => [entry.filename, entry.sha256]));
  const records = [];
  const requiredPayload = new Set(["syndocal.exe", "uninstall.exe", ...pinned.keys()]);
  const add = (file, path) => {
    const record = readVerifiedRegularFile(file, "signing component " + path, { allowedRoots: [path.startsWith("payload/") ? root : dirname(resolve(installer))] });
    if (path.startsWith("payload/") && !requiredPayload.has(path.slice(8))) throw new Error("Unapproved PE component in final NSIS payload: " + path);
    const upstreamHash = path.startsWith("payload/") ? pinned.get(path.slice(8)) : undefined;
    if (upstreamHash !== undefined && hash(record.bytes) !== upstreamHash) throw new Error("Pinned upstream runtime bytes changed; signing is not an exemption: " + path);
    records.push({ path, bytes: record.bytes, sha256: hash(record.bytes), ownership: upstreamHash === undefined ? "project" : "upstream" });
  };
  const walk = (directory, prefix = "") => {
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
      if (entry.isSymbolicLink()) throw new Error("Signing inventory rejects reparse points.");
      const relative = prefix + entry.name;
      if (entry.isDirectory()) walk(join(directory, entry.name), relative + "/");
      else if (/\.(exe|dll)$/iu.test(entry.name)) add(join(directory, entry.name), "payload/" + relative);
    }
  };
  if (!/\.exe$/iu.test(installer)) throw new Error("NSIS signing requires an .exe installer.");
  add(resolve(installer), "installer/" + basename(installer));
  walk(root);
  for (const required of requiredPayload) if (!records.some((record) => record.path === "payload/" + required)) throw new Error("Final NSIS payload is missing required PE component: " + required);
  return records.sort((a, b) => a.path.localeCompare(b.path));
}

export function verifyWindowsSigningTree(declaration, options) {
  validateWindowsSigningState(declaration);
  const records = collectWindowsSigningComponents(options);
  assert.deepEqual(records.map((r) => r.path).sort(), declaration.components.map((r) => r.path).sort(), "Signing inventory must cover the exact recursive PE component set.");
  for (const record of records) {
    if (declaration.components.find((c) => c.path === record.path).ownership !== record.ownership) throw new Error("Component ownership does not match pinned upstream inventory: " + record.path);
    verifyWindowsSigningComponent(declaration, record.path, record.bytes, options.inspect);
  }
  return declaration;
}

export function runWindowsSigningStateCli(argv) {
  const flags = new Map();
  for (let index = 0; index < argv.length; index += 2) {
    if (!argv[index]?.startsWith("--") || !argv[index + 1] || flags.has(argv[index])) throw new Error("Expected unique --flag value arguments.");
    flags.set(argv[index], argv[index + 1]);
  }
  const allowed = ["--installer", "--payload-root", "--output", "--state", "--unsigned-reason", "--signpath", "--verify"];
  if ([...flags.keys()].some((key) => !allowed.includes(key)) || !flags.has("--installer") || !flags.has("--payload-root")) throw new Error("Usage: --installer <NSIS exe> --payload-root <installed tree> (--verify <state.json> | --output <new state.json> --state <declared-unsigned|signpath-signed> [--unsigned-reason <reason>] [--signpath <provenance.json>])");
  const options = { installer: flags.get("--installer"), payloadRoot: flags.get("--payload-root") };
  if (flags.has("--verify")) {
    if (flags.has("--output") || flags.has("--state") || flags.has("--unsigned-reason") || flags.has("--signpath")) throw new Error("Verification takes only --verify, --installer and --payload-root.");
    verifyWindowsSigningTree(parseStrictJson(readFileSync(flags.get("--verify"), "utf8"), "Windows signing state"), options);
  } else {
    if (!flags.has("--output") || !flags.has("--state")) throw new Error("Generation requires --output and --state.");
    const declaration = { schemaVersion: 1, state: flags.get("--state"), unsignedReason: flags.get("--unsigned-reason") ?? null,
      signpath: flags.has("--signpath") ? parseStrictJson(readFileSync(flags.get("--signpath"), "utf8"), "SignPath provenance") : null,
      components: collectWindowsSigningComponents(options).map((record) => {
        const signature = inspectAuthenticodeBytes(record.bytes);
        return { path: record.path, sha256: record.sha256, ownership: record.ownership, authenticode: signature.status === "Valid" ? "valid" : "not-signed", certificateThumbprint: signature.thumbprint?.toUpperCase() ?? null };
      }) };
    verifyWindowsSigningTree(declaration, options);
    writeFileSync(flags.get("--output"), JSON.stringify(declaration, null, 2) + "\n", { flag: "wx" });
  }
  console.log("Windows release signing state verified against final bytes and exact PE inventory.");
}
if (resolve(process.argv[1] ?? "") === fileURLToPath(import.meta.url)) runWindowsSigningStateCli(process.argv.slice(2));
