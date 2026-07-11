import { readFile } from "node:fs/promises";
import assert from "node:assert/strict";
import ts from "typescript";

async function importTsModule(path) {
  const source = await readFile(new URL(path, import.meta.url), "utf8");
  const transpiled = ts.transpileModule(source, {
    compilerOptions: {
      module: ts.ModuleKind.ESNext,
      target: ts.ScriptTarget.ES2022,
      importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
    },
    fileName: path,
  });
  return import(`data:text/javascript;base64,${Buffer.from(transpiled.outputText).toString("base64")}`);
}

const recent = await importTsModule("../src/projectRecentStorage.ts");
const recovery = await importTsModule("../src/projectRecoveryStorage.ts");

assert.equal(recent.recentProjectFileName("C:\\shows\\main.sdc"), "main.sdc");
assert.equal(recent.recentProjectFileName("/shows/main.sdc"), "main.sdc");

assert.deepEqual(
  recent.recentProjectPathsFromUnknown([
    " C:/shows/main.sdc ",
    "C:/shows/MAIN.SDC",
    "C:/shows/notes.txt",
    42,
    "C:/shows/backup.sdc",
  ]),
  ["C:/shows/main.sdc", "C:/shows/backup.sdc"],
);
const legacyProjectPath = `C:/shows/legacy.${["r", "y"].join("")}`;
assert.deepEqual(recent.recentProjectPathsFromUnknown([legacyProjectPath]), []);

const manyProjects = Array.from({ length: 12 }, (_, index) => `C:/shows/show-${index}.sdc`);
assert.equal(recent.recentProjectPathsFromUnknown(manyProjects).length, 8);
assert.deepEqual(
  recent.touchRecentProjectPath(["C:/shows/a.sdc", "C:/shows/b.sdc"], "C:/shows/B.SDC"),
  ["C:/shows/B.SDC", "C:/shows/a.sdc"],
);
assert.deepEqual(recent.touchRecentProjectPath(["C:/shows/a.sdc"], "C:/shows/b.txt"), ["C:/shows/a.sdc"]);

const project = {
  version: 1,
  app: "Syndocal",
  custom_profiles: [],
  snapshot: { fixtures: [], cues: [] },
};
const checkpoint = recovery.createProjectRecoveryCheckpoint(project, "C:/shows/main.sdc", "sig-1");
assert.equal(checkpoint.version, 1);
assert.equal(checkpoint.app, "Syndocal");
assert.equal(checkpoint.project, project);
assert.equal(checkpoint.signature, "sig-1");
assert.equal(checkpoint.source_path, "C:/shows/main.sdc");
assert.equal(recovery.projectRecoverySourceLabel(checkpoint), "main.sdc");

assert.deepEqual(recovery.recoveryCheckpointFromUnknown(checkpoint), checkpoint);
assert.equal(
  recovery.recoveryCheckpointFromUnknown({ ...checkpoint, source_path: " C:/shows/spaced.sdc " }).source_path,
  "C:/shows/spaced.sdc",
);
assert.equal(recovery.recoveryCheckpointFromUnknown({ ...checkpoint, app: "Other" }), null);
assert.equal(recovery.recoveryCheckpointFromUnknown({ ...checkpoint, project: { ...project, app: "Other" } }), null);
assert.equal(recovery.recoveryCheckpointFromUnknown({ ...checkpoint, project: { ...project, snapshot: null } }), null);
assert.equal(recovery.recoveryCheckpointFromUnknown({ ...checkpoint, project: { ...project, snapshot: {} } }), null);
assert.equal(recovery.projectRecoverySourceLabel({ ...checkpoint, source_path: null }), "Untitled.sdc");
assert.equal(recovery.projectRecoveryTimeLabel({ ...checkpoint, saved_at: "bad-date" }), "Recovery");

console.log("project storage helpers ok");
