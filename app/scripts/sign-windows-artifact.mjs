import crypto from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const workspaceDirectory = path.resolve(scriptDirectory, "../..");
const releaseDirectory = path.join(workspaceDirectory, "target", "release");
const defaultArtifact = path.join(releaseDirectory, "syndocal.exe");
const timestampUrl = process.env.SYNDOCAL_SIGNING_TIMESTAMP_URL;

function usage() {
  throw new Error(
    "Usage: node app/scripts/sign-windows-artifact.mjs --artifact <absolute-or-workspace-relative exe> (--thumbprint <40-hex> | --pfx <absolute pfx>) --timestamp <https URL> [--verify]",
  );
}

function parseArguments(argv) {
  const result = { verify: false };
  for (let index = 0; index < argv.length; index += 1) {
    const flag = argv[index];
    if (flag === "--verify") {
      if (result.verify) usage();
      result.verify = true;
      continue;
    }
    const key = {
      "--artifact": "artifact",
      "--thumbprint": "thumbprint",
      "--pfx": "pfx",
      "--timestamp": "timestamp",
    }[flag];
    if (!key || result[key] !== undefined || argv[index + 1] === undefined) usage();
    result[key] = argv[++index];
  }
  if (result.thumbprint && result.pfx) usage();
  if (!result.verify && !result.thumbprint && !result.pfx) usage();
  if (!result.verify && !(result.timestamp ?? timestampUrl)) {
    throw new Error("A trusted Authenticode timestamp URL is required for signing.");
  }
  result.artifact ??= defaultArtifact;
  return result;
}

function canonicalExisting(filePath, label) {
  if (!path.isAbsolute(filePath)) throw new Error(`${label} must be absolute after path resolution.`);
  try {
    return fs.realpathSync(filePath);
  } catch {
    throw new Error(`${label} does not exist.`);
  }
}

function containedBy(directory, filePath) {
  const relative = path.relative(directory, filePath);
  return relative !== ""
    && !relative.startsWith("..")
    && !path.isAbsolute(relative)
    && !relative.includes(path.sep);
}

function artifactPath(value) {
  const candidate = canonicalExisting(path.resolve(workspaceDirectory, value), "Artifact");
  const release = canonicalExisting(releaseDirectory, "Release directory");
  if (!containedBy(release, candidate) || path.extname(candidate).toLowerCase() !== ".exe") {
    throw new Error("Artifact must be an executable directly under this checkout's target/release directory.");
  }
  return candidate;
}

function signtoolPath() {
  const candidates = [
    process.env.SYNDOCAL_SIGNTOOL,
    "C:\\Program Files (x86)\\Windows Kits\\10\\bin\\10.0.26100.0\\x64\\signtool.exe",
    "C:\\Program Files (x86)\\Windows Kits\\10\\bin\\10.0.22621.0\\x64\\signtool.exe",
  ].filter(Boolean);
  for (const candidate of candidates) {
    try {
      const resolved = canonicalExisting(candidate, "SignTool");
      if (path.basename(resolved).toLowerCase() === "signtool.exe") return resolved;
    } catch {
      // Try the next installed Windows SDK candidate.
    }
  }
  throw new Error("A supported x64 signtool.exe was not found. Set SYNDOCAL_SIGNTOOL to an absolute path.");
}

function runSignTool(tool, args) {
  try {
    execFileSync(tool, args, {
      windowsHide: true,
      stdio: ["ignore", "pipe", "pipe"],
      encoding: "utf8",
      maxBuffer: 64 * 1024,
    });
  } catch {
    throw new Error("SignTool failed. No signing result is accepted until verification succeeds.");
  }
}

function verify(tool, artifact) {
  runSignTool(tool, ["verify", "/pa", "/all", "/tw", artifact]);
  const hash = crypto.createHash("sha256").update(fs.readFileSync(artifact)).digest("hex").toUpperCase();
  return { status: "verified", artifact, sha256: hash };
}

const options = parseArguments(process.argv.slice(2));
const artifact = artifactPath(options.artifact);
const tool = signtoolPath();
if (!options.verify) {
  const stamp = options.timestamp ?? timestampUrl;
  if (!/^https:\/\/[^\s/]+(?:\/[^\s]*)?$/u.test(stamp)) {
    throw new Error("The Authenticode timestamp must be an HTTPS URL.");
  }
  if (options.thumbprint && !/^[0-9a-f]{40}$/iu.test(options.thumbprint)) {
    throw new Error("Certificate thumbprint must be exactly 40 hexadecimal characters.");
  }
  const args = ["sign", "/fd", "SHA256", "/tr", stamp, "/td", "SHA256"];
  if (options.thumbprint) {
    args.push("/sha1", options.thumbprint);
  } else {
    const pfx = canonicalExisting(options.pfx, "PFX");
    if (path.extname(pfx).toLowerCase() !== ".pfx") throw new Error("PFX input must use the .pfx extension.");
    const password = process.env.SYNDOCAL_SIGNING_PFX_PASSWORD;
    if (typeof password !== "string" || password.length === 0) {
      throw new Error("SYNDOCAL_SIGNING_PFX_PASSWORD is required for PFX signing and is never logged.");
    }
    args.push("/f", pfx, "/p", password);
  }
  args.push(artifact);
  runSignTool(tool, args);
}
console.log(JSON.stringify(verify(tool, artifact)));
