import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import {
  ledgerPath,
  sourceDocumentPath,
  validateCompletionLedger,
  workspaceRoot,
} from "./check-completion-ledger.mjs";

const ledger = JSON.parse(readFileSync(resolve(workspaceRoot, ledgerPath), "utf8"));
const sourceDocument = readFileSync(resolve(workspaceRoot, sourceDocumentPath), "utf8");
const clone = (value) => structuredClone(value);
const fixtureOptions = {
  sourceDocument,
  pathExists: () => true,
  isTracked: () => true,
};

const result = validateCompletionLedger({ ledger, sourceDocument, repoRoot: workspaceRoot });
assert.equal(result.itemCount, 58);
assert.deepEqual(result.sections, ["6", "7", "8", "9"]);
assert.deepEqual(result.statusCounts, { Open: 41, Deferred: 8, Complete: 9 });
assert.deepEqual(result.sectionCounts, { "6": 37, "7": 6, "8": 9, "9": 6 });

function expectFailure(name, mutate, pattern, options = {}) {
  const candidate = clone(ledger);
  mutate(candidate);
  assert.throws(
    () => validateCompletionLedger({ ledger: candidate, ...fixtureOptions, ...options }),
    pattern,
    name,
  );
}

expectFailure(
  "duplicate stable IDs fail closed",
  (candidate) => candidate.items.push(clone(candidate.items[0])),
  /duplicate ledger ID COMP-Q1-Q4-001/u,
);
expectFailure(
  "missing classification fails closed",
  (candidate) => delete candidate.items[0].classification,
  /missing a valid classification/u,
);
expectFailure(
  "missing owner fails closed",
  (candidate) => delete candidate.items[0].owner,
  /missing owner/u,
);
expectFailure(
  "missing dependency fails closed",
  (candidate) => delete candidate.items[0].dependency,
  /missing dependency/u,
);
expectFailure(
  "missing nonclaim fails closed",
  (candidate) => delete candidate.items[0].nonclaim,
  /missing an explicit nonclaim/u,
);
expectFailure(
  "missing evidence fails closed",
  (candidate) => delete candidate.items[0].evidence_paths,
  /missing evidence paths/u,
);
expectFailure(
  "unmapped authoritative open item fails closed",
  (candidate) => candidate.items.shift(),
  /unmapped authoritative item COMP-Q1-Q4-001/u,
);
expectFailure(
  "unmapped authoritative open section row fails closed",
  () => {},
  /unmapped authoritative open row/u,
  { sourceDocument: sourceDocument.replace("- [x] Build Q1-Q4 coverage from every phase below; assign Supported/External/etc. <!-- completion-ledger: Complete: COMP-Q1-Q4-001 -->", "- [ ] Build Q1-Q4 coverage from every phase below; assign Supported/External/etc. <!-- completion-ledger: Complete: COMP-Q1-Q4-001 -->") },
);
expectFailure(
  "stale source section fails closed",
  (candidate) => { candidate.items[0].source.section = "7"; },
  /stale ledger ID COMP-Q1-Q4-001/u,
);
expectFailure(
  "nonexistent evidence path fails closed",
  (candidate) => { candidate.items[0].evidence_paths = ["qa/not-present-evidence.md"]; },
  /evidence path does not exist/u,
  { pathExists: () => false, isTracked: () => false },
);
expectFailure(
  "untracked evidence path fails closed",
  (candidate) => { candidate.items[0].evidence_paths = ["qa/not-tracked-evidence.md"]; },
  /evidence path is not tracked/u,
  { pathExists: () => true, isTracked: () => false },
);
expectFailure(
  "external acceptance cannot be marked complete",
  (candidate) => { candidate.items.find((item) => item.classification === "External acceptance").status = "Complete"; },
  /external acceptance .*may never be Deferred or Complete/u,
);
expectFailure(
  "open source row cannot be marked Complete",
  (candidate) => { candidate.items.find((item) => item.id === "AI5-SIDECAR-001").status = "Complete"; },
  /open source row AI5-SIDECAR-001 must remain status Open/u,
);

expectFailure(
  "completed source row requires Complete status",
  (candidate) => { candidate.items.find((item) => item.id === "RELEASE-METADATA-GATE-001").status = "Open"; },
  /completed source row RELEASE-METADATA-GATE-001 must remain status Complete/u,
);

expectFailure(
  "reopening a frozen distribution row fails closed",
  () => {},
  /unmapped authoritative open row/u,
  {
    sourceDocument: sourceDocument.replace(
      "- **Frozen / out of scope for the current Windows-local completion target:** Platform/package matrix.",
      "- [ ] Platform/package matrix.",
    ),
  },
);
expectFailure(
  "treating a deferred row as open fails closed",
  (candidate) => { candidate.items.find((item) => item.id === "WARN-MACOS-001").status = "Open"; },
  /deferred source row WARN-MACOS-001 must remain status Deferred/u,
);
expectFailure(
  "wrong classification for an open source row fails closed",
  (candidate) => { candidate.items.find((item) => item.id === "AI5-SIDECAR-001").classification = "Deferred"; },
  /open source row AI5-SIDECAR-001 must be Supported or External acceptance/u,
);
expectFailure(
  "wrong status for an open source row fails closed",
  (candidate) => { candidate.items.find((item) => item.id === "AI5-SIDECAR-001").status = "Deferred"; },
  /open source row AI5-SIDECAR-001 must remain status Open/u,
);
expectFailure(
  "wrong source kind fails closed",
  (candidate) => {
    const item = candidate.items.find((entry) => entry.id === "F1-INPUT-GENERATIONS-001");
    item.source.kind = "Deferred";
    item.classification = "Deferred";
    item.status = "Deferred";
  },
  /stale ledger ID F1-INPUT-GENERATIONS-001: source kind Deferred does not match Open/u,
);
expectFailure(
  "arbitrary marker line fails closed",
  () => {},
  /arbitrary completion-ledger marker/u,
  {
    sourceDocument: sourceDocument.replace(
      "Exit: synchronized version metadata, warning baseline, current traceability/risk/",
      "Exit: synchronized version metadata, warning baseline, current traceability/risk/ <!-- completion-ledger: Open: ARBITRARY-MARKER-001 -->",
    ),
  },
);

console.log("completion ledger self-tests ok; 19 negative cases");
