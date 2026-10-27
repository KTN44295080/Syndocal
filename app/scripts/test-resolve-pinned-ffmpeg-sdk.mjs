import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, realpathSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { pinnedFfmpegSdkRelativeRoot, resolvePinnedFfmpegSdk } from "./resolve-pinned-ffmpeg-sdk.mjs";
import { canonicalWindowsFfmpegRuntimeDllNames, workspaceRoot } from "./windows-runtime-inventory.mjs";

// This local SDK check deliberately requires the real provisioned pin. No
// executable/DLL is loaded and the full native harnesses are never invoked.
const pinnedRoot = resolve(workspaceRoot, pinnedFfmpegSdkRelativeRoot);
const scriptDir = dirname(fileURLToPath(import.meta.url));
const temporaryParent = realpathSync(tmpdir());
const temporaryRoot = mkdtempSync(join(temporaryParent, "syndocal-pinned-sdk-"));
const fixtureRoot = join(temporaryRoot, "SDK-日本語");
let passed = 0;
function check(label, callback) {
  callback();
  passed += 1;
  console.log(`PASS ${label}`);
}
function rejectRoot(root, pattern) {
  assert.throws(() => resolvePinnedFfmpegSdk({ explicitRoot: root }), pattern);
}
function cli(environment, args = []) {
  const result = spawnSync(process.execPath, [join(scriptDir, "resolve-pinned-ffmpeg-sdk.mjs"), ...args], {
    env: { ...process.env, ...environment }, encoding: "utf8", windowsHide: true,
  });
  assert.ifError(result.error);
  assert.equal(result.stderr, "");
  return { status: result.status, record: JSON.parse(result.stdout), stdout: result.stdout };
}
function psLiteral(text) {
  return "'" + text.replaceAll("'", "''") + "'";
}
try {
  check("automatic selection verifies the real pinned seven-DLL and recording-executable SDK", () => {
    assert.equal(resolvePinnedFfmpegSdk({ explicitRoot: "" }), realpathSync(pinnedRoot));
  });
  check("explicit FFMPEG_DIR verifies the same real pinned SDK", () => {
    assert.equal(resolvePinnedFfmpegSdk({ explicitRoot: pinnedRoot }), realpathSync(pinnedRoot));
  });
  check("missing automatic pin fails closed without searching arbitrary SDKs", () => {
    assert.throws(() => resolvePinnedFfmpegSdk({ workspace: temporaryRoot, explicitRoot: "" }), /does not exist.*WINDOWS_NATIVE_BUILD/su);
  });
  check("missing explicit SDK never falls back to the valid automatic pin", () => {
    rejectRoot(join(temporaryRoot, "missing"), /FFMPEG_DIR.*does not exist/su);
  });
  check("relative explicit SDK is rejected", () => rejectRoot("target/sdk", /absolute local directory/u));
  check("whitespace-ambiguous explicit SDK is rejected", () => rejectRoot(` ${pinnedRoot} `, /whitespace ambiguity/u));

  for (const filename of canonicalWindowsFfmpegRuntimeDllNames) {
    const library = filename.slice(0, filename.lastIndexOf("-"));
    for (const relative of [`include/lib${library}/${library}.h`, `lib/${library}.lib`, `bin/${filename}`]) {
      const destination = join(fixtureRoot, relative);
      mkdirSync(dirname(destination), { recursive: true });
      copyFileSync(join(pinnedRoot, relative), destination);
    }
  }
  const encoder = join(fixtureRoot, "bin/ffmpeg.exe");
  copyFileSync(join(pinnedRoot, "bin/ffmpeg.exe"), encoder);
  check("complete copied SDK passes at a Japanese explicit path", () => {
    assert.equal(resolvePinnedFfmpegSdk({ explicitRoot: fixtureRoot }), realpathSync(fixtureRoot));
  });
  rmSync(encoder);
  check("missing pinned recording executable fails closed", () => rejectRoot(fixtureRoot, /recording encoder is missing/u));
  const encoderBytes = readFileSync(join(pinnedRoot, "bin/ffmpeg.exe"));
  const changedEncoderBytes = Buffer.from(encoderBytes);
  changedEncoderBytes[changedEncoderBytes.length - 1] ^= 1;
  writeFileSync(encoder, changedEncoderBytes);
  check("same-size recording executable mismatch fails closed", () => rejectRoot(fixtureRoot, /recording encoder SHA-256 does not match/u));
  writeFileSync(encoder, encoderBytes);
  const header = join(fixtureRoot, "include/libavcodec/avcodec.h");
  rmSync(header);
  check("incomplete headers fail closed", () => rejectRoot(fixtureRoot, /avcodec\.h is missing/u));
  copyFileSync(join(pinnedRoot, "include/libavcodec/avcodec.h"), header);
  const library = join(fixtureRoot, "lib/avcodec.lib");
  writeFileSync(library, "");
  check("empty MSVC import library fails closed", () => rejectRoot(fixtureRoot, /avcodec\.lib is empty/u));
  copyFileSync(join(pinnedRoot, "lib/avcodec.lib"), library);
  const runtime = join(fixtureRoot, "bin/avcodec-62.dll");
  rmSync(runtime);
  check("missing pinned runtime fails closed", () => rejectRoot(fixtureRoot, /avcodec-62\.dll is missing/u));
  copyFileSync(join(pinnedRoot, "bin/avcodec-62.dll"), runtime);
  const originalBytes = readFileSync(runtime);
  const changedBytes = Buffer.from(originalBytes);
  changedBytes[changedBytes.length - 1] ^= 1;
  writeFileSync(runtime, changedBytes);
  check("same-size DLL mismatch cannot bypass FFMPEG_DIR verification", () => rejectRoot(fixtureRoot, /SHA-256 does not match/u));
  const gplBytes = Buffer.from(originalBytes);
  gplBytes.write("--enable-gpl --enable-nonfree", 128, "ascii");
  writeFileSync(runtime, gplBytes);
  check("GPL replacement cannot bypass explicit-root pinning", () => rejectRoot(fixtureRoot, /SHA-256 does not match/u));
  writeFileSync(runtime, originalBytes);

  check("CLI succeeds with an unset FFMPEG_DIR", () => {
    const result = cli({ FFMPEG_DIR: "" });
    assert.equal(result.status, 0);
    assert.equal(result.record.root, realpathSync(pinnedRoot));
  });
  check("CLI preserves Japanese root paths as ASCII JSON", () => {
    const result = cli({ FFMPEG_DIR: fixtureRoot });
    assert.equal(result.status, 0);
    assert.equal(result.record.root, realpathSync(fixtureRoot));
    assert.match(result.stdout, /^[\x00-\x7f]*$/u);
  });
  check("CLI returns a failed status and actionable error for invalid override", () => {
    const result = cli({ FFMPEG_DIR: join(temporaryRoot, "missing") });
    assert.equal(result.status, 1);
    assert.match(result.record.error, /FFMPEG_DIR.*WINDOWS_NATIVE_BUILD/su);
  });
  check("CLI rejects unsupported arguments", () => {
    const result = cli({ FFMPEG_DIR: pinnedRoot }, ["--skip-verification"]);
    assert.equal(result.status, 1);
    assert.match(result.record.error, /accepts no command-line arguments/u);
  });

  for (const harness of ["check-native-workspace-operator.ps1", "check-native-window-acceptance.ps1"]) {
    check(`${harness} delegates exclusively to the shared resolver`, () => {
      const source = readFileSync(join(scriptDir, harness), "utf8");
      assert.match(source, /\. \(Join-Path \$PSScriptRoot "resolve-pinned-ffmpeg-sdk\.ps1"\)/u);
      assert.doesNotMatch(source, /function (?:Test|Resolve)-FfmpegSdkRoot|Gyan\.FFmpeg|full_build-shared|C:\\temp\\ffmpeg/u);
      assert.match(source, /\$ffmpegDir = Resolve-FfmpegSdkRoot/u);
    });
  }
  if (process.platform === "win32") {
    for (const file of ["check-native-workspace-operator.ps1", "check-native-window-acceptance.ps1", "resolve-pinned-ffmpeg-sdk.ps1"]) {
      check(`${file} parses without executing the harness`, () => {
        execFileSync("powershell.exe", ["-NoLogo", "-NoProfile", "-NonInteractive", "-Command",
          "$ErrorActionPreference='Stop'; $tokens=$null; $errors=$null; "
          + `[void][System.Management.Automation.Language.Parser]::ParseFile(${psLiteral(join(scriptDir, file))},[ref]$tokens,[ref]$errors); `
          + "if($errors.Count -gt 0){throw ($errors | Out-String)}"], { windowsHide: true });
      });
    }
    check("PowerShell adapter resolves the Unicode root without running the harness", () => {
      const output = execFileSync("powershell.exe", ["-NoLogo", "-NoProfile", "-NonInteractive", "-Command",
        "$ErrorActionPreference='Stop'; Set-StrictMode -Version Latest; [Console]::OutputEncoding=[Text.UTF8Encoding]::new($false); "
        + `. ${psLiteral(join(scriptDir, "resolve-pinned-ffmpeg-sdk.ps1"))}; Resolve-FfmpegSdkRoot | ConvertTo-Json -Compress`],
      { env: { ...process.env, FFMPEG_DIR: fixtureRoot }, encoding: "utf8", windowsHide: true });
      assert.equal(JSON.parse(output), realpathSync(fixtureRoot));
    });
    check("PowerShell adapter throws on a rejected explicit override", () => {
      const result = spawnSync("powershell.exe", ["-NoLogo", "-NoProfile", "-NonInteractive", "-Command",
        "$ErrorActionPreference='Stop'; Set-StrictMode -Version Latest; "
        + `. ${psLiteral(join(scriptDir, "resolve-pinned-ffmpeg-sdk.ps1"))}; try { Resolve-FfmpegSdkRoot; exit 2 } catch { `
        + "if($_.Exception.Message -notmatch 'FFMPEG_DIR.*WINDOWS_NATIVE_BUILD'){throw}; exit 0 }"],
      { env: { ...process.env, FFMPEG_DIR: join(temporaryRoot, "missing") }, encoding: "utf8", windowsHide: true });
      assert.ifError(result.error);
      assert.equal(result.status, 0, result.stderr);
    });
  }
  console.log(`Pinned FFmpeg SDK resolver: ${passed} checks passed.`);
} finally {
  // Only delete this test's verified absolute, freshly-created temporary root.
  assert.equal(dirname(resolve(temporaryRoot)), temporaryParent);
  assert.match(temporaryRoot.slice(temporaryParent.length + 1), /^syndocal-pinned-sdk-[^\\/]+$/u);
  rmSync(temporaryRoot, { recursive: true, force: true });
}
