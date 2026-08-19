import { existsSync, readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const expectedVersion = "1.2.0-alpha.1";
const scriptDir = dirname(fileURLToPath(import.meta.url));
const appRoot = resolve(scriptDir, "..");
const workspaceRoot = resolve(appRoot, "..");
const read = (path) => readFileSync(resolve(workspaceRoot, path), "utf8");
const appPackage = JSON.parse(read("app/package.json"));
const tauri = JSON.parse(read("app/src-tauri/tauri.conf.json"));
const updaterOverlay = JSON.parse(read("app/src-tauri/tauri.updater.conf.json"));

if (appPackage.name !== "syndocal" || appPackage.version !== expectedVersion) {
  throw new Error(`Frontend package metadata is not Syndocal ${expectedVersion}.`);
}
if (tauri.productName !== "Syndocal" || tauri.version !== expectedVersion || !tauri.bundle?.active) {
  throw new Error("Tauri product/version/bundle metadata is inconsistent.");
}
if (tauri.bundle.publisher !== "Seraf()のKTN") {
  throw new Error("Tauri publisher metadata changed unexpectedly.");
}
if (!tauri.bundle.fileAssociations?.some((association) => association.ext?.includes("sdc"))) {
  throw new Error("The .sdc project association is missing.");
}
for (const icon of tauri.bundle.icon ?? []) {
  if (!existsSync(resolve(appRoot, "src-tauri", icon))) {
    throw new Error(`Configured bundle icon is missing: ${icon}`);
  }
}

const updaterDefaults = tauri.plugins?.updater;
if (
  typeof updaterDefaults !== "object" ||
  updaterDefaults === null ||
  updaterDefaults.pubkey !== "" ||
  !Array.isArray(updaterDefaults.endpoints) ||
  updaterDefaults.endpoints.length !== 0
) {
  throw new Error("Default updater metadata must remain disabled and contain no signing key or endpoint.");
}
if (updaterOverlay.bundle?.createUpdaterArtifacts !== true) {
  throw new Error("Updater release overlay must enable signed updater artifacts.");
}

const bcdecLicenseSource = "../../licenses/bcdec_rs-MIT.txt";
if (tauri.bundle.resources?.[bcdecLicenseSource] !== "licenses/bcdec_rs-MIT.txt") {
  throw new Error("The bcdec_rs license is not configured as a bundle resource.");
}
if (!existsSync(resolve(appRoot, "src-tauri", bcdecLicenseSource))) {
  throw new Error("The configured bcdec_rs license file is missing.");
}
if (!existsSync(resolve(workspaceRoot, "qa", "UPDATE_RELEASE_RUNBOOK.md"))) {
  throw new Error("The signed updater release runbook is missing.");
}

const rootManifest = read("Cargo.toml");
if (!rootManifest.includes(`version = "${expectedVersion}"`)) {
  throw new Error(`Cargo workspace version is not ${expectedVersion}.`);
}
for (const manifest of [
  "app/src-tauri/Cargo.toml",
  "crates/audio/Cargo.toml",
  "crates/engine/Cargo.toml",
  "crates/gdtf/Cargo.toml",
  "crates/io/Cargo.toml",
  "crates/protocol/Cargo.toml",
  "crates/video/Cargo.toml",
  "crates/visualizer/Cargo.toml",
]) {
  if (!read(manifest).includes("version.workspace = true")) {
    throw new Error(`${manifest} does not inherit the workspace version.`);
  }
}

const macBundleScript = read("app/scripts/bundle-macos-runtime.sh");
if (!macBundleScript.includes(`Syndocal_${expectedVersion}_$(uname -m).dmg`)) {
  throw new Error("macOS DMG filename does not match the product version.");
}
if (!macBundleScript.includes(`-volname 'Syndocal ${expectedVersion}'`)) {
  throw new Error("macOS DMG volume name does not match the product version.");
}

const crossPlatformWorkflow = read(".github/workflows/cross-platform.yml");
if (!crossPlatformWorkflow.includes(`name: syndocal-${expectedVersion}-\${{ matrix.os }}`)) {
  throw new Error("Cross-platform artifact name does not match the product version.");
}

console.log(`release metadata ok: Syndocal ${expectedVersion} / .sdc / signed updater overlay / Seraf()のKTN`);
