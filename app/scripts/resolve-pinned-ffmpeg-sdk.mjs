import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import {
  anchoredAmd64PeIdentity,
  assertSafeExternalDirectory,
  canonicalPinnedFfmpegBuild,
  canonicalPinnedRuntimeIdentity,
  canonicalPinnedRecordingEncoder,
  canonicalWindowsFfmpegRuntimeDllNames,
  readVerifiedRegularFile,
  verifyPinnedRuntimeFile,
  workspaceRoot,
} from "./windows-runtime-inventory.mjs";

export const pinnedFfmpegSdkRelativeRoot = "target/qa/ffmpeg-selfbuild-20261027/sdk/"
  + "ffmpeg-n8.1.3-9-g29e619e767-win64-lgpl-shared-8.1";

// No executable or DLL is loaded: the same independently anchored bytes and PE
// identities used for distribution also gate native harness SDK selection.
export function resolvePinnedFfmpegSdk({ workspace = workspaceRoot, explicitRoot = process.env.FFMPEG_DIR } = {}) {
  const overridden = explicitRoot !== undefined && explicitRoot.trim() !== "";
  const candidate = overridden ? explicitRoot : resolve(workspace, pinnedFfmpegSdkRelativeRoot);
  const label = overridden ? "FFMPEG_DIR" : "Pinned LGPL FFmpeg SDK";
  try {
    const root = assertSafeExternalDirectory(candidate, label);
    for (const directory of ["include", "lib", "bin"]) {
      assertSafeExternalDirectory(join(root, directory), `${label} ${directory}`);
    }
    for (const filename of canonicalWindowsFfmpegRuntimeDllNames) {
      const library = filename.slice(0, filename.lastIndexOf("-"));
      for (const required of [`include/lib${library}/${library}.h`, `lib/${library}.lib`]) {
        const record = readVerifiedRegularFile(join(root, required), `${label} ${required}`, { allowedRoots: [root] });
        if (record.bytes.length === 0) throw new Error(`${label} ${required} is empty.`);
      }
      verifyPinnedRuntimeFile(join(root, "bin", filename), {
        filename,
        ...canonicalPinnedRuntimeIdentity.get(filename),
        pe_identity: anchoredAmd64PeIdentity,
      }, `${label} runtime ${filename}`, { allowedRoots: [root] });
    }
    verifyPinnedRuntimeFile(join(root, "bin", "ffmpeg.exe"), canonicalPinnedRecordingEncoder, `${label} recording encoder`, { allowedRoots: [root] });
    return root;
  } catch (error) {
    throw new Error(`${label} '${candidate}' is invalid: ${error.message} `
      + `Provision the exact ${canonicalPinnedFfmpegBuild.version} LGPL shared SDK described in qa/WINDOWS_NATIVE_BUILD.md. `
      + "FFMPEG_DIR overrides must pass the same seven pinned DLL and recording ffmpeg.exe size/SHA-256/PE checks; no WinGet, GPL or PATH fallback is permitted.",
    { cause: error });
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  let result;
  try {
    if (process.argv.length !== 2) throw new Error("This resolver accepts no command-line arguments; use FFMPEG_DIR for an explicit SDK root.");
    result = { root: resolvePinnedFfmpegSdk() };
  } catch (error) {
    result = { error: error.message };
    process.exitCode = 1;
  }
  // ASCII JSON preserves Japanese paths through Windows PowerShell's native
  // command decoding regardless of the active console code page.
  process.stdout.write(JSON.stringify(result).replace(/[\u007f-\uffff]/gu,
    (character) => "\\u" + character.charCodeAt(0).toString(16).padStart(4, "0")) + "\n");
}
