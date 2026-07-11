import { existsSync, readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const expectedVersion = "1.0.0";
const scriptDir = dirname(fileURLToPath(import.meta.url));
const appRoot = resolve(scriptDir, "..");
const workspaceRoot = resolve(appRoot, "..");
const read = (path) => readFileSync(resolve(workspaceRoot, path), "utf8");
const appPackage = JSON.parse(read("app/package.json"));
const tauri = JSON.parse(read("app/src-tauri/tauri.conf.json"));

if (appPackage.name !== "syndocal" || appPackage.version !== expectedVersion) {
  throw new Error("Frontend package metadata is not Syndocal 1.0.0.");
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

const rootManifest = read("Cargo.toml");
if (!rootManifest.includes(`version = "${expectedVersion}"`)) {
  throw new Error("Cargo workspace version is not 1.0.0.");
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

console.log("release metadata ok: Syndocal 1.0.0 / .sdc / Seraf()のKTN");
