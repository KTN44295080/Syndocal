import { copyFileSync, existsSync, mkdirSync, readdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

if (process.platform !== "win32") {
  console.log("No staged runtime libraries are required on this platform.");
  process.exit(0);
}

const ffmpegDir = process.env.FFMPEG_DIR?.trim();
if (!ffmpegDir) {
  console.log("FFMPEG_DIR is not set; producing the SDK-independent bundle.");
  process.exit(0);
}

const sourceDir = join(ffmpegDir, "bin");
if (!existsSync(sourceDir)) {
  throw new Error(`FFmpeg runtime directory does not exist: ${sourceDir}`);
}

const scriptDir = dirname(fileURLToPath(import.meta.url));
const workspaceRoot = resolve(scriptDir, "../..");
const releaseDirs = [join(workspaceRoot, "target", "release")];
const targetTriple = process.env.TAURI_ENV_TARGET_TRIPLE?.trim();
if (targetTriple) {
  releaseDirs.push(join(workspaceRoot, "target", targetTriple, "release"));
}
const runtimeLibraries = readdirSync(sourceDir).filter((name) => name.toLowerCase().endsWith(".dll"));
if (runtimeLibraries.length === 0) {
  throw new Error(`No FFmpeg runtime DLLs were found in ${sourceDir}`);
}

for (const releaseDir of new Set(releaseDirs)) {
  if (!existsSync(join(releaseDir, "syndocal.exe"))) {
    continue;
  }
  mkdirSync(releaseDir, { recursive: true });
  for (const library of runtimeLibraries) {
    copyFileSync(join(sourceDir, library), join(releaseDir, library));
  }
  console.log(`Staged ${runtimeLibraries.length} FFmpeg DLLs in ${releaseDir}`);
}
