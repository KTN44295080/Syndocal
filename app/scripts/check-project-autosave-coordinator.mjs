import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const source = await readFile(new URL("../src/projectAutosaveCoordinator.ts", import.meta.url), "utf8");
const output = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
}).outputText;
const {
  createProjectAutosaveCoordinator,
  isSupersededRecoveryCapture,
  SUPERSEDED_RECOVERY_REPORT_AFTER,
} = await import(`data:text/javascript;base64,${Buffer.from(output).toString("base64")}`);

// A capture the backend refused because a live edit moved the project on, or
// because another capture held the lock, is superseded, not failed.
assert.equal(isSupersededRecoveryCapture("Project changed before recovery capture (expected epoch 1 revision 2)"), true);
assert.equal(isSupersededRecoveryCapture(new Error("Project recovery capture is busy; retry")), true);
assert.equal(isSupersededRecoveryCapture("Project recovery capture admission lock was poisoned"), false);
assert.equal(
  isSupersededRecoveryCapture("Project recovery capture is unavailable: recovery authority is faulted; restart Syndocal"),
  false,
  "a faulted recovery authority needs a restart, so it is reported as a failure",
);
assert.equal(isSupersededRecoveryCapture("browser storage is unavailable or full"), false);
const lagging = createProjectAutosaveCoordinator(true, () => 0);
for (let run = 1; run < SUPERSEDED_RECOVERY_REPORT_AFTER; run += 1) {
  assert.equal(lagging.recoverySuperseded(), false, "a few superseded captures stay silent");
}
assert.equal(lagging.recoverySuperseded(), true, "a run of superseded captures is reported as lagging");
lagging.recoverySucceeded();
assert.equal(lagging.recoverySuperseded(), false, "success restarts the superseded count");
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
assert.match(app, /shouldReportError\(message\)\) setMessage\(message, projectRecoveryStatusKey\)/);
assert.match(app, /if \(isSupersededRecoveryCapture\(error\)\) \{[\s\S]*?retrySupersededProjectRecovery\(\);[\s\S]*?recoverySuperseded\(\)[\s\S]*?return;\s*\}/);
assert.match(app, /setProjectRecoveryCheckpoint\(checkpoint\);\s*projectAutosaveCoordinator.recoverySucceeded\(\);\s*clearProjectRecoveryStatus\(\);/);
assert.match(app, /if \(appStatus\(\)\.key === projectRecoveryStatusKey\) setMessage\(/);
assert.match(app, /window\.clearTimeout\(supersededRecoveryRetry\)/);
// A superseded retry is never retried again or counted: no 1 Hz capture chain.
assert.match(app, /void saveProjectRecovery\(true\);/);
assert.match(app, /if \(isSupersededRecoveryCapture\(error\)\) \{[\s\S]*?if \(!isRetry\) \{\s*retrySupersededProjectRecovery\(\);/);
const backend = await readFile(new URL("../src-tauri/src/main.rs", import.meta.url), "utf8");
assert.match(backend, /recovery authority is faulted; restart Syndocal/);
assert.match(app, /if \(!isTauriRuntime\(\) \|\| paneWindow \|\| projectPublicationStartupRecoveryStarted\) return;/);
assert.doesNotMatch(app, /lastDesktopBackupAt/);
console.log("Project autosave coordinator: PASS (single flight, failure cooldown, exact errors, superseded captures, success reset and status clear, main/pane ownership and App wiring)");
