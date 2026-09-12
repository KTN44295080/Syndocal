import assert from "node:assert/strict";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import {
  aiRoadmapSourceContract,
  completionLedgerPath,
  flowDocumentPath,
  masterRoadmapPath,
  q1Q4LedgerPath,
  validateQ1Q4Ledger,
  workspaceRoot,
} from "./check-q1-q4-ledger.mjs";

const realMaster = readFileSync(resolve(workspaceRoot, masterRoadmapPath), "utf8");
const realFlow = readFileSync(resolve(workspaceRoot, flowDocumentPath), "utf8");
const baseCompletion = JSON.parse(readFileSync(resolve(workspaceRoot, completionLedgerPath), "utf8"));
const baseLedger = JSON.parse(readFileSync(resolve(workspaceRoot, q1Q4LedgerPath), "utf8"));
const fenceMarker = "```json syndocal-q1-q4-ledger";

const realResult = validateQ1Q4Ledger({
  ledger: baseLedger,
  masterDocument: realMaster,
  flowDocument: realFlow,
  completionLedger: baseCompletion,
});
assert.equal(realResult.q1Rows, 32);
assert.equal(realResult.domainsCovered, 29);
assert.equal(realResult.sourceContractsCovered, 10);
assert.equal(realResult.flowMarkersReferenced, 58);
assert.deepEqual(realResult.flowMarkerCounts, { Open: 48, Deferred: 8, Complete: 2 });
console.log("ok - real-repository baseline passes end to end");

function byId(collection, id) {
  const entry = collection.find((row) => row.id === id);
  assert.ok(entry, `fixture setup: ${id} must exist`);
  return entry;
}

function buildInputs({
  ledger = baseLedger,
  master = realMaster,
  flow = realFlow,
  completion = baseCompletion,
  missingPaths = [],
  untrackedPaths = [],
} = {}) {
  return {
    ledger,
    masterText: master,
    flowText: flow,
    completion,
    missingPaths,
    untrackedPaths,
  };
}

function runValidation(inputs) {
  const fixtureDir = mkdtempSync(join(tmpdir(), "syndocal-q1q4-fixture-"));
  try {
    writeFileSync(join(fixtureDir, "q1q4.fixture.json"), `${JSON.stringify(inputs.ledger, null, 2)}\n`);
    writeFileSync(join(fixtureDir, "master.fixture.md"), inputs.masterText);
    writeFileSync(join(fixtureDir, "flow.fixture.md"), inputs.flowText);
    writeFileSync(join(fixtureDir, "completion.fixture.json"), `${JSON.stringify(inputs.completion, null, 2)}\n`);
    return validateQ1Q4Ledger({
      ledger: JSON.parse(readFileSync(join(fixtureDir, "q1q4.fixture.json"), "utf8")),
      masterDocument: readFileSync(join(fixtureDir, "master.fixture.md"), "utf8"),
      flowDocument: readFileSync(join(fixtureDir, "flow.fixture.md"), "utf8"),
      completionLedger: JSON.parse(readFileSync(join(fixtureDir, "completion.fixture.json"), "utf8")),
      repoRoot: fixtureDir,
      pathExists: (path) => !inputs.missingPaths.includes(path),
      isTracked: (path) => !inputs.untrackedPaths.includes(path),
    });
  } finally {
    rmSync(fixtureDir, { recursive: true, force: true });
  }
}

let executedCases = 0;

expectPass({});

function expectPass(overrides) {
  runValidation(buildInputs(overrides));
  executedCases += 1;
  console.log(`ok - positive fixture baseline (${executedCases})`);
}

function expectFailure(code, overrides) {
  let thrown = null;
  try {
    runValidation(buildInputs(overrides));
  } catch (error) {
    thrown = error;
  }
  const message = thrown === null ? "(validation unexpectedly passed)" : String(thrown?.message ?? thrown);
  assert.ok(thrown !== null, `expected validation failure with ${code}`);
  assert.ok(message.includes(code), `expected finding ${code} but got:\n${message}`);
  executedCases += 1;
  console.log(`ok - negative case ${executedCases}: ${code}`);
}

function mutateLedger(mutate) {
  const copy = structuredClone(baseLedger);
  mutate(copy);
  return { ledger: copy };
}

function mutateCompletion(mutate) {
  const copy = structuredClone(baseCompletion);
  mutate(copy);
  return { completion: copy };
}

function mutateMaster(text) {
  return { master: text };
}

expectFailure("AUTOMATED_PROOF_ZERO_COUNT", mutateLedger((copy) => {
  byId(copy.q1_requirements, "COV-Q1Q4-INFRA-001").automated_proof.expected_count = 0;
}));

expectFailure("AUTOMATED_PROOF_COMMAND_REQUIRED", mutateLedger((copy) => {
  byId(copy.q1_requirements, "COV-Q1Q4-INFRA-001").automated_proof.command = "";
}));

expectFailure("PROOF_NOT_APPLICABLE_WITHOUT_REASON", mutateLedger((copy) => {
  byId(copy.q1_requirements, "COV-Q1Q4-INFRA-001").native_proof.na_reason = "";
}));

expectFailure("PROOF_NOT_APPLICABLE_WITHOUT_REASON", mutateLedger((copy) => {
  byId(copy.q1_requirements, "COV-Q1Q4-INFRA-001").hardware_external_proof.na_reason = "";
}));

expectFailure("DUPLICATE_ID", mutateLedger((copy) => {
  copy.q1_requirements.push(structuredClone(copy.q1_requirements[0]));
}));

expectFailure("INVALID_ENUM_VALUE", mutateLedger((copy) => {
  byId(copy.q1_requirements, "COV-MEDIA-DERIVED-001").scope = "Bogus";
}));

expectFailure("INVALID_ENUM_VALUE", mutateLedger((copy) => {
  byId(copy.q1_requirements, "COV-MEDIA-DERIVED-001").status = "Paused";
}));

expectFailure("REQUIRED_FIELD_MISSING", mutateLedger((copy) => {
  delete byId(copy.q1_requirements, "COV-MEDIA-DERIVED-001").requirement;
}));

expectFailure("COMMIT_MUST_BE_SHA", mutateLedger((copy) => {
  byId(copy.q1_requirements, "COV-Q1Q4-INFRA-001").status = "Implemented";
}));

expectFailure("UNKNOWN_DOMAIN_REFERENCE", mutateLedger((copy) => {
  byId(copy.q1_requirements, "COV-MEDIA-DERIVED-001").domains.push("GHOST-DOMAIN");
}));

expectFailure("UNKNOWN_SOURCE_CONTRACT_REFERENCE", mutateLedger((copy) => {
  byId(copy.q1_requirements, "COV-MEDIA-DERIVED-001").source_contracts.push("qa/GHOST_CONTRACT.md");
}));

expectFailure("EVIDENCE_PATH_NOT_TRACKED", { untrackedPaths: ["AGENTS.md"] });

expectFailure("EVIDENCE_PATH_NOT_FOUND", { missingPaths: ["AGENTS.md"] });

expectFailure("EVIDENCE_PATH_NOT_TRACKED", { untrackedPaths: ["qa/CODEX_COMPLETION_ROADMAP_2026-08-13.md"] });

expectFailure("EVIDENCE_PATH_NOT_FOUND", { missingPaths: ["qa/CODEX_COMPLETION_ROADMAP_2026-08-13.md"] });

expectFailure("UNKNOWN_FLOW_MARKER_REFERENCE", mutateLedger((copy) => {
  byId(copy.q1_requirements, "COV-MEDIA-T1-001").flow_refs.push("NOT-A-MARKER-001");
}));

expectFailure("FLOW_MARKER_UNREFERENCED", mutateLedger((copy) => {
  byId(copy.q1_requirements, "COV-Q1Q4-INFRA-001").flow_refs = [];
}));

expectFailure("COMPLETION_INFRA_ROW_CLOSED", mutateCompletion((copy) => {
  byId(copy.items, "COMP-Q1-Q4-001").status = "Accepted";
}));

expectFailure("COMPLETION_LEDGER_MARKER_DRIFT", mutateCompletion((copy) => {
  copy.items.pop();
}));

expectFailure("ACCEPTED_ROW_REFERENCES_OPEN_FLOW_MARKER", mutateLedger((copy) => {
  byId(copy.q1_requirements, "COV-Q1Q4-INFRA-001").status = "Accepted";
}));

expectFailure("ROW_STATUS_ACCEPTED_WITHOUT_ACCEPTED_PROOFS", mutateLedger((copy) => {
  const row = byId(copy.q1_requirements, "COV-Q1Q4-INFRA-001");
  row.status = "Accepted";
  row.flow_refs = [];
}));

expectFailure("ACCEPTED_PROOF_WITHOUT_ARTIFACT_EVIDENCE", mutateLedger((copy) => {
  const row = byId(copy.q1_requirements, "COV-Q1Q4-INFRA-001");
  row.status = "Accepted";
  row.flow_refs = [];
  row.native_proof = { status: "accepted", artifact_hash: null, raw_evidence_paths: [], na_reason: null };
}));

expectFailure("ACCEPTED_PROOF_WITHOUT_DEVICE_TOPOLOGY", mutateLedger((copy) => {
  const row = byId(copy.q1_requirements, "COV-Q1Q4-INFRA-001");
  row.status = "Accepted";
  row.flow_refs = [];
  row.native_proof = { status: "accepted", artifact_hash: "a".repeat(64), raw_evidence_paths: ["qa/CODEX_COMPLETION_ROADMAP_2026-08-13.md"], na_reason: null };
  row.hardware_external_proof = { status: "accepted", device_topology_duration: "", artifact_hash: "b".repeat(64), raw_evidence_paths: ["qa/CODEX_COMPLETION_ROADMAP_2026-08-13.md"], na_reason: null };
}));

expectFailure("HISTORICAL_EVIDENCE_PROMOTED_TO_CURRENT_ACCEPTANCE", mutateLedger((copy) => {
  const row = byId(copy.q1_requirements, "COV-MEDIA-T1-001");
  row.status = "Accepted";
  row.native_proof = { status: "accepted", artifact_hash: "c".repeat(64), raw_evidence_paths: ["qa/MEDIA_ASSET_T1_A8_NATIVE_EVIDENCE_2026-08-13.md"], na_reason: null };
}));

expectFailure("ORPHAN_DECISION", mutateLedger((copy) => {
  byId(copy.q1_requirements, "COV-AUDIO-LIVE-001").decision_ids = [];
}));

expectFailure("ORPHAN_RISK", mutateLedger((copy) => {
  byId(copy.q1_requirements, "COV-PATCH-GDTF-001").risk_ids = [];
}));

expectFailure("ORPHAN_EVIDENCE", mutateLedger((copy) => {
  byId(copy.q1_requirements, "COV-STAGE-001").evidence_ids = [];
  byId(copy.q1_requirements, "COV-PROJECT-TX-001").evidence_ids = [];
}));

expectFailure("LINK_ASYMMETRY", mutateLedger((copy) => {
  const decision = byId(copy.q2_decisions, "DEC-ASIO-001");
  decision.linked_q1_ids = decision.linked_q1_ids.filter((id) => id !== "COV-AUDIO-LIVE-001");
}));

expectFailure("LINK_ASYMMETRY", mutateLedger((copy) => {
  const risk = byId(copy.q3_risks, "R-PATCH-ATOMIC-001");
  risk.linked_q1_ids = risk.linked_q1_ids.filter((id) => id !== "COV-PATCH-GDTF-001");
}));

expectFailure("LINK_ASYMMETRY", mutateLedger((copy) => {
  const evidence = byId(copy.q4_evidence, "EV-Q1Q4-COVERAGE-INFRA-2026-08-25");
  evidence.linked_q1_ids = evidence.linked_q1_ids.filter((id) => id !== "COV-Q1Q4-INFRA-001");
}));

expectFailure("UNKNOWN_LINK_TARGET", mutateLedger((copy) => {
  byId(copy.q1_requirements, "COV-Q1Q4-INFRA-001").decision_ids.push("DEC-GHOST-999");
}));

expectFailure("AI_CONTROL_LINKAGE_FLATTENED", mutateLedger((copy) => {
  for (const id of ["COV-Q1Q4-INFRA-001", "COV-AI-CONTROL-001"]) {
    const row = byId(copy.q1_requirements, id);
    row.source_contracts = row.source_contracts.filter((path) => path !== aiRoadmapSourceContract);
  }
}));

expectFailure("MISSING_DOMAIN_COVERAGE", mutateLedger((copy) => {
  for (const id of ["COV-Q1Q4-INFRA-001", "COV-SHOWCLOCK-001"]) {
    const row = byId(copy.q1_requirements, id);
    row.domains = row.domains.filter((domain) => domain !== "SHOWCLOCK");
  }
}));

expectFailure("MISSING_SOURCE_COVERAGE", mutateLedger((copy) => {
  for (const id of ["COV-Q1Q4-INFRA-001", "COV-COMPARE-LIGHTING-001", "COV-PATCH-GDTF-001"]) {
    const row = byId(copy.q1_requirements, id);
    row.source_contracts = row.source_contracts.filter((path) => path !== "qa/DASLIGHT_PARITY_COMPLETION_PLAN.md");
  }
}));

expectFailure("Q0_DOMAIN_REGISTRY_MISMATCH", mutateLedger((copy) => {
  copy.q0_registry.domains.pop();
}));

expectFailure("Q0_SOURCE_REGISTRY_MISMATCH", mutateLedger((copy) => {
  copy.q0_registry.source_contracts.pop();
}));

expectFailure("STALE_EXPECTED_COUNTS", mutateLedger((copy) => {
  copy.expected_counts.q1_rows = 33;
}));

expectFailure("NONCLAIM_CONTRACT_VIOLATION", mutateLedger((copy) => {
  copy.nonclaims.flow_reference = "See the flow document for the authoritative marker list.";
}));

expectFailure("DUPLICATE_ID", mutateLedger((copy) => {
  copy.q2_decisions.push(structuredClone(copy.q2_decisions[0]));
}));

expectFailure("DATE_FORMAT_INVALID", mutateLedger((copy) => {
  copy.q2_decisions[0].date = "2026/08/13";
}));

expectFailure("Q4_ACCEPTED_CURRENT_INCOMPLETE", mutateLedger((copy) => {
  byId(copy.q4_evidence, "EV-Q1Q4-COVERAGE-INFRA-2026-08-25").exit_code = 1;
}));

expectFailure("Q4_PLANNED_WITHOUT_REQUIRED_SCOPE", mutateLedger((copy) => {
  copy.q4_evidence.push({
    id: "EV-PLANNED-GAP-001",
    scope: "Placeholder planned evidence",
    kind: "required-not-yet-produced",
    status: "planned",
    date: "2026-08-25",
    command: null,
    exit_code: null,
    assertion_count: null,
    ignored_count: null,
    warning_count: null,
    artifact_hash: null,
    artifact_na_reason: null,
    raw_evidence_paths: [],
    currency_nonclaim: "Planned placeholder is not current-source acceptance.",
    linked_q1_ids: ["COV-Q1Q4-INFRA-001"],
  });
  byId(copy.q1_requirements, "COV-Q1Q4-INFRA-001").evidence_ids.push("EV-PLANNED-GAP-001");
}));

expectFailure("MASTER_FENCE_MISSING", mutateMaster(realMaster.replace(/```json syndocal-q1-q4-ledger\r?\n/, "")));

expectFailure("MASTER_FENCE_MULTIPLE", mutateMaster(realMaster.replace(fenceMarker, `${fenceMarker}\n${fenceMarker}`)));

expectFailure("MASTER_FENCE_OUTSIDE_Q1_Q4_REGION", mutateMaster((() => {
  const start = realMaster.indexOf(fenceMarker);
  const close = realMaster.indexOf("\n```", start) + 4;
  const block = realMaster.slice(start, close);
  const withoutBlock = realMaster.slice(0, start) + realMaster.slice(close);
  const q1Index = withoutBlock.indexOf("### Q1.");
  return `${withoutBlock.slice(0, q1Index)}${block}\n\n${withoutBlock.slice(q1Index)}`;
})()));

expectFailure("MASTER_JSON_PARITY_MISMATCH", mutateMaster(realMaster.replace('"q1_rows": 32,', '"q1_rows": 31,')));

if (executedCases !== 47) {
  throw new Error(`self-test case drift: executed ${executedCases} cases, expected 47 (1 positive + 46 negative)`);
}

console.log(`q1-q4 ledger self-tests ok; 46 negative cases + 1 positive fixture baseline + 1 real-repository baseline`);
