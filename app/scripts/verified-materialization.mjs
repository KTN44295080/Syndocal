import { createHash } from "node:crypto";
import {
  chmodSync,
  lstatSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { basename, join } from "node:path";

export function sha256Bytes(bytes) {
  if (!Buffer.isBuffer(bytes)) throw new Error("verified materialization requires Buffer bytes.");
  return createHash("sha256").update(bytes).digest("hex");
}

function assertSafeFileName(value, label) {
  if (typeof value !== "string" || value.length === 0 || value !== basename(value) || /[\\/:\0\r\n]/u.test(value)) {
    throw new Error(label + " must be a single safe file name.");
  }
}

function assertFreshPrivateDirectory(directory, label) {
  const stats = lstatSync(directory, { bigint: true });
  if (!stats.isDirectory() || stats.isSymbolicLink()) {
    throw new Error(label + " materialization directory is not a fresh ordinary directory: " + directory);
  }
  try {
    chmodSync(directory, 0o700);
  } catch {
    // Windows ACLs govern the per-user temp directory. Node cannot prove its
    // owner/DACL, so callers must treat this as a bounded per-user temp copy,
    // not a handle-atomic ACL proof.
  }
  return `${stats.dev}:${stats.ino}`;
}

function cleanupMaterializationDirectory(directory, directoryIdentity, label, removeDirectory) {
  const stats = lstatSync(directory, { bigint: true });
  if (!stats.isDirectory() || stats.isSymbolicLink() || `${stats.dev}:${stats.ino}` !== directoryIdentity) {
    throw new Error(label + " cleanup refused a swapped materialization directory; it remains visible at " + directory + ".");
  }
  try {
    removeDirectory(directory);
  } catch (error) {
    throw new Error(
      label
        + " cleanup failed; the materialization directory remains visible at "
        + directory
        + ": "
        + String(error instanceof Error ? error.message : error),
    );
  }
}

/**
 * Supplies an inspector only a fresh wx-written copy of verified bytes. The
 * copy is SHA-256 checked before and after the inspector, then removed only
 * when its original directory identity remains intact.
 */
export function withMaterializedVerifiedExecutable(bytes, inspect, {
  label = "Windows executable evidence",
  filename = "syndocal-inspect.exe",
  removeDirectory = (directory) => rmSync(directory, { recursive: true, force: false }),
} = {}) {
  if (!Buffer.isBuffer(bytes) || bytes.length === 0) throw new Error(label + " bytes are empty.");
  if (typeof inspect !== "function") throw new Error(label + " inspector must be a function.");
  assertSafeFileName(filename, label + " filename");
  const expectedSha256 = sha256Bytes(bytes);
  const directory = mkdtempSync(join(tmpdir(), "syndocal-verified-inspect-"));
  const directoryIdentity = assertFreshPrivateDirectory(directory, label);
  const executablePath = join(directory, filename);
  let result;
  let primaryError;
  try {
    writeFileSync(executablePath, bytes, { flag: "wx", mode: 0o700 });
    try { chmodSync(executablePath, 0o700); } catch { /* Windows ACLs apply. */ }
    const before = readFileSync(executablePath);
    if (sha256Bytes(before) !== expectedSha256) {
      throw new Error(label + " materialized copy SHA-256 differs before inspection.");
    }
    result = inspect(executablePath);
    const afterLinkStats = lstatSync(executablePath, { bigint: true });
    if (!afterLinkStats.isFile() || afterLinkStats.isSymbolicLink()) {
      throw new Error(label + " materialized copy is no longer an ordinary regular file after inspection.");
    }
    const after = readFileSync(executablePath);
    if (sha256Bytes(after) !== expectedSha256) {
      throw new Error(label + " materialized copy SHA-256 changed during inspection.");
    }
    const fileStats = statSync(executablePath, { bigint: true });
    if (!fileStats.isFile()) throw new Error(label + " materialized copy is no longer a regular file after inspection.");
  } catch (error) {
    primaryError = error;
  }
  let cleanupError;
  try {
    cleanupMaterializationDirectory(directory, directoryIdentity, label, removeDirectory);
  } catch (error) {
    cleanupError = error;
  }
  if (cleanupError !== undefined) {
    if (primaryError !== undefined) {
      throw new Error(
        String(primaryError instanceof Error ? primaryError.message : primaryError)
          + " Cleanup also failed: "
          + String(cleanupError instanceof Error ? cleanupError.message : cleanupError),
      );
    }
    throw cleanupError;
  }
  if (primaryError !== undefined) throw primaryError;
  return result;
}

export function runVerifiedMaterializationSelfTest() {
  let assertions = 0;
  const pass = (condition, label) => {
    if (!condition) throw new Error(label);
    assertions += 1;
  };
  const rejects = (action, pattern) => {
    let thrown;
    try { action(); } catch (error) { thrown = error; }
    if (!(thrown instanceof Error) || !pattern.test(thrown.message)) {
      throw new Error("materialization self-test expected rejection " + pattern + "; got " + String(thrown));
    }
    assertions += 1;
  };
  const bytes = Buffer.from("verified executable fixture", "utf8");
  let observedPath = null;
  const result = withMaterializedVerifiedExecutable(bytes, (path) => {
    observedPath = path;
    return "inspected";
  }, { label: "materialization self-test" });
  pass(result === "inspected" && typeof observedPath === "string" && observedPath.includes("syndocal-verified-inspect-"), "inspector receives only a fresh materialized executable path");
  rejects(
    () => withMaterializedVerifiedExecutable(bytes, (path) => writeFileSync(path, Buffer.from("swapped", "utf8")), { label: "materialization swap self-test" }),
    /SHA-256 changed during inspection/,
  );
  let retainedDirectory = null;
  rejects(
    () => withMaterializedVerifiedExecutable(bytes, () => undefined, {
      label: "materialization cleanup self-test",
      removeDirectory: (directory) => {
        retainedDirectory = directory;
        throw new Error("simulated cleanup refusal");
      },
    }),
    /cleanup failed; the materialization directory remains visible at/,
  );
  pass(retainedDirectory !== null && lstatSync(retainedDirectory, { bigint: true }).isDirectory(), "cleanup refusal leaves the materialization directory visible");
  rmSync(retainedDirectory, { recursive: true, force: false });
  console.log("Verified materialization self-test passed: " + assertions + " assertions");
}
