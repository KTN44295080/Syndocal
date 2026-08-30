import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const read = (relativePath) => readFile(new URL(`../${relativePath}`, import.meta.url), "utf8");

const panel = await read("src/components/StandbySyncPanel.tsx");
const controller = await read("src/outputControlController.ts");

assert.match(
  panel,
  /const requestedLeaseRole = createMemo<OutputControlTargetRole>\([\s\S]*?machineRole\(\) === "Lighting" \? "lighting"[\s\S]*?machineRole\(\) === "Video" \? "video"[\s\S]*?: "both"/,
  "the acquire role must be derived once from the selected machine role",
);
assert.match(
  panel,
  /const activeLeaseOverlapsRequestedRole = createMemo\(\(\) => \{[\s\S]*?candidate\.status === "held_active"[\s\S]*?candidate\.resources\.some\(\(resource\) => requestedResources\.includes\(resource\)\)/,
  "only an active lease with an intersecting selected-role resource may block acquire",
);
assert.match(
  panel,
  /data-io-control="acquire-selected-role-lease"[\s\S]*?disabled=\{!props\.backendAvailable \|\| busy\(\) \|\| activeLeaseOverlapsRequestedRole\(\)\}/,
  "Acquire selected-role lease must be disabled when active authority overlaps its requested role",
);

assert.match(
  controller,
  /const knownPreActionRejections = new Set\(\[[\s\S]*?"invalid_request", "forbidden", "stale_fence", "busy", "overloaded",[\s\S]*?\]\);/,
  "the controller must retain the closed typed terminal rejection inventory",
);
assert.match(
  panel,
  /const terminalNoOutputAppliedRejectionCodes = new Set\(\[[\s\S]*?"invalid_request",[\s\S]*?"forbidden",[\s\S]*?"stale_fence",[\s\S]*?"busy",[\s\S]*?"overloaded",[\s\S]*?\]\);/,
  "the panel must recognise only the controller's closed no-output-applied terminal inventory",
);
assert.ok(
  panel.includes(String.raw`function isTerminalNoOutputAppliedRejection(error: unknown): error is Error {`)
    && panel.includes(String.raw`const match = /^OutputControl rejected \(([^)]+)\); (output was not applied; refresh lease state\.|nothing was applied\.)$/.exec(error.message);`)
    && panel.includes("terminalNoOutputAppliedRejectionCodes.has(match[1])"),
  "the UI may reconcile only a strict, typed terminal no-output-applied rejection",
);
assert.doesNotMatch(
  panel,
  /terminalNoOutputAppliedRejectionCodes[\s\S]{0,300}publication_failed|publication_failed[\s\S]{0,300}terminalNoOutputAppliedRejectionCodes/,
  "publication failures must remain physical-state-unknown and fail closed",
);

const lifecycleCatch = panel.match(/const runLeaseLifecycle[\s\S]*?\} catch \(error\) \{([\s\S]*?)\n    \} finally \{/);
assert.ok(lifecycleCatch, "lease lifecycle must retain a single explicit failure branch");
assert.match(
  lifecycleCatch[1],
  /if \(isTerminalNoOutputAppliedRejection\(error\)\) \{[\s\S]*?await pollStatus\(\);[\s\S]*?setActionError\(error\.message\);[\s\S]*?return;/,
  "a no-output-applied terminal rejection must make one read-only reconciliation before retaining held authority",
);
assert.equal(
  (lifecycleCatch[1].match(/await pollStatus\(\);/g) ?? []).length,
  1,
  "a no-output-applied terminal rejection must issue exactly one reconciliation poll",
);
assert.match(
  lifecycleCatch[1],
  /clearOutputAuthority\(\);[\s\S]*?setActionError\(String\(error\)\);/,
  "all non-typed, malformed, publication-failed, or lost replies must still clear visible authority",
);

console.log("Standby Sync output-lease UI contract passed.");
