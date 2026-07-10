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

assert.equal(recent.recentProjectFileName("C:\\shows\\main.ry"), "main.ry");
assert.equal(recent.recentProjectFileName("/shows/main.ry"), "main.ry");

assert.deepEqual(
  recent.recentProjectPathsFromUnknown([
    " C:/shows/main.ry ",
    "C:/shows/MAIN.RY",
    "C:/shows/notes.txt",
    42,
    "C:/shows/backup.ry",
  ]),
  ["C:/shows/main.ry", "C:/shows/backup.ry"],
);

const manyProjects = Array.from({ length: 12 }, (_, index) => `C:/shows/show-${index}.ry`);
assert.equal(recent.recentProjectPathsFromUnknown(manyProjects).length, 8);
assert.deepEqual(
  recent.touchRecentProjectPath(["C:/shows/a.ry", "C:/shows/b.ry"], "C:/shows/B.RY"),
  ["C:/shows/B.RY", "C:/shows/a.ry"],
);
assert.deepEqual(recent.touchRecentProjectPath(["C:/shows/a.ry"], "C:/shows/b.txt"), ["C:/shows/a.ry"]);

const project = {
  version: 1,
  app: "Rayard",
  custom_profiles: [],
  snapshot: { fixtures: [], cues: [] },
};
const checkpoint = recovery.createProjectRecoveryCheckpoint(project, "C:/shows/main.ry", "sig-1");
assert.equal(checkpoint.version, 1);
assert.equal(checkpoint.app, "Rayard");
assert.equal(checkpoint.project, project);
assert.equal(checkpoint.signature, "sig-1");
assert.equal(checkpoint.source_path, "C:/shows/main.ry");
assert.equal(recovery.projectRecoverySourceLabel(checkpoint), "main.ry");

assert.deepEqual(recovery.recoveryCheckpointFromUnknown(checkpoint), checkpoint);
assert.equal(
  recovery.recoveryCheckpointFromUnknown({ ...checkpoint, source_path: " C:/shows/spaced.ry " }).source_path,
  "C:/shows/spaced.ry",
);
assert.equal(recovery.recoveryCheckpointFromUnknown({ ...checkpoint, app: "Other" }), null);
assert.equal(recovery.recoveryCheckpointFromUnknown({ ...checkpoint, project: { ...project, app: "Other" } }), null);
assert.equal(recovery.recoveryCheckpointFromUnknown({ ...checkpoint, project: { ...project, snapshot: null } }), null);
assert.equal(recovery.recoveryCheckpointFromUnknown({ ...checkpoint, project: { ...project, snapshot: {} } }), null);
assert.equal(recovery.projectRecoverySourceLabel({ ...checkpoint, source_path: null }), "Untitled.ry");
assert.equal(recovery.projectRecoveryTimeLabel({ ...checkpoint, saved_at: "bad-date" }), "Recovery");

console.log("project storage helpers ok");
