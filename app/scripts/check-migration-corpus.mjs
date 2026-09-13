import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { requireExactMsvcLinkerFirst, tauriCommandEnvironment } from "./run-tauri.mjs";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const expectedTests = 11;
assert.equal(process.argv.length, 2, "Usage: node app/scripts/check-migration-corpus.mjs");
// Reuse the maintained exact vcvars/toolset/PATH-first contract. This prepares
// Cargo's environment only; it does not invoke Tauri, launch or stop an app.
const env = tauriCommandEnvironment(["build"]);
if (process.platform === "win32") {
  console.log(`CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER=${env.CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER}`);
  console.log(`where.exe link.exe:\n${requireExactMsvcLinkerFirst(env).join("\n")}`);
}
const args = ["test", "--manifest-path", "app/src-tauri/Cargo.toml", "--release",
  "--locked", "-j", "1", "migration_corpus_", "--", "--nocapture", "--test-threads=1"];
console.log(`cargo ${args.join(" ")}`);
const result = spawnSync("cargo", args, {
  cwd: root, env, encoding: "utf8", maxBuffer: 16 * 1024 * 1024,
  stdio: ["ignore", "pipe", "pipe"],
});
process.stdout.write(result.stdout ?? "");
process.stderr.write(result.stderr ?? "");
if (result.error) throw result.error;
assert.equal(result.status, 0, `Migration corpus Cargo exit=${result.status}, signal=${result.signal}`);
const summaries = [...(result.stdout ?? "").matchAll(
  /test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored;/gu,
)];
assert.equal(summaries.length, 1, "Expected one actual Rust test-binary result, not an empty selection");
assert.equal(Number(summaries[0][1]), expectedTests, "Migration corpus test selection changed");
assert.equal(Number(summaries[0][2]), 0);
assert.equal(Number(summaries[0][3]), 0, "Migration acceptance tests must not be ignored");
console.log(`migration corpus gate passed: ${expectedTests} Rust tests, none failed or ignored`);
