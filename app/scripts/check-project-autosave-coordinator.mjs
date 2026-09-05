import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const source = await readFile(new URL("../src/projectAutosaveCoordinator.ts", import.meta.url), "utf8");
const output = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
}).outputText;
const { createProjectAutosaveCoordinator } = await import(`data:text/javascript;base64,${Buffer.from(output).toString("base64")}`);
let now = 0;
const main = createProjectAutosaveCoordinator(true, () => now);
assert.equal(main.beginRecovery(), true);
assert.equal(main.beginRecovery(), false, "overlapping timer cannot start another capture");
try {
  assert.equal(main.beginDesktopBackup(true), true, "first changed image attempts immediately");
  throw new Error("injected IPC failure");
} catch (error) {
  assert.equal(main.shouldReportError(String(error)), true);
} finally {
  main.finishRecovery();
}
assert.equal(main.beginRecovery(), true, "failure releases the capture guard");
main.finishRecovery();
for (now = 10_000; now < 60_000; now += 10_000) {
  assert.equal(main.beginDesktopBackup(true), false, "failed attempt retains its cooldown");
}
assert.equal(main.beginDesktopBackup(true), true, "retry is allowed at the exact 60 second boundary");
assert.equal(main.shouldReportError("Error: injected IPC failure"), false, "identical errors cannot overwrite a newer footer message");
assert.equal(main.shouldReportError("a different failure"), true);
main.recoverySucceeded();
assert.equal(main.shouldReportError("a different failure"), true, "a failure after actual success is reportable again");
now = 120_000;
assert.equal(main.beginDesktopBackup(false), false, "unchanged successful signature needs no backup");
assert.equal(main.beginDesktopBackup(true), true, "skipping an unchanged image does not reserve an attempt");

const pane = createProjectAutosaveCoordinator(false, () => now);
assert.equal(pane.beginRecovery(), true, "pane browser checkpoint capture remains available");
assert.equal(pane.beginDesktopBackup(true), false, "pane cannot publish a desktop backup");
pane.finishRecovery();

const app = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
assert.match(app, /createProjectAutosaveCoordinator\(!paneWindow\)/);
assert.match(app, /if \(!projectAutosaveCoordinator.beginRecovery\(\)\) return;[\s\S]*?finally \{\s*projectAutosaveCoordinator.finishRecovery\(\);/);
assert.match(app, /beginDesktopBackup\(signature !== lastDesktopBackupSignature\)/);
assert.match(app, /backup.state !== "succeeded"[\s\S]*?throw new Error[\s\S]*?lastDesktopBackupSignature = signature;\s*projectAutosaveCoordinator.recoverySucceeded\(\)/);
assert.match(app, /shouldReportError\(message\)\) setMessage\(message\)/);
assert.match(app, /if \(!isTauriRuntime\(\) \|\| paneWindow \|\| projectPublicationStartupRecoveryStarted\) return;/);
assert.doesNotMatch(app, /lastDesktopBackupAt/);
console.log("Project autosave coordinator: PASS (single flight, failure cooldown, exact errors, success reset, main/pane ownership and App wiring)");
