import { execFileSync } from "node:child_process";
import { existsSync, lstatSync, readFileSync } from "node:fs";
import { dirname, isAbsolute, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { collectAuthoritativeMarkers, workspaceRoot as completionWorkspaceRoot } from "./check-completion-ledger.mjs";

export const workspaceRoot = completionWorkspaceRoot;

export const q1Q4LedgerPath = "qa/SYNDOCAL_Q1_Q4_LEDGER.json";
export const masterRoadmapPath = "qa/CODEX_COMPLETION_ROADMAP_2026-08-13.md";
export const flowDocumentPath = "qa/SYNDOCAL_COMPLETION_FLOW_2026-08-19.md";
export const completionLedgerPath = "qa/SYNDOCAL_COMPLETION_LEDGER.json";
export const mirrorFenceTag = "syndocal-q1-q4-ledger";

export const q1Scopes = Object.freeze(["Supported", "Deferred", "External acceptance", "Out of scope"]);
export const q1Statuses = Object.freeze(["Not started", "In progress", "Implemented", "Reviewed", "Accepted", "Blocked"]);
export const proofStatuses = Object.freeze(["not-run", "recorded", "accepted", "not-applicable"]);
export const residualClasses = Object.freeze(["P0", "P0-Code", "P0-Release", "P1", "P2", "P2 release-blocking", "non-blocking"]);
export const q2States = Object.freeze(["Accepted", "Open", "Superseded", "Out of scope"]);
export const q3Severities = Object.freeze(["P0-Code", "P0-Release", "P1", "P2", "P2 release-blocking"]);
export const q3Likelihoods = Object.freeze(["rare", "unlikely", "possible", "likely", "near-certain", "undetermined"]);
export const q3Statuses = Object.freeze(["Open", "Closed"]);
export const q4Kinds = Object.freeze(["historical-evidence", "current-source-automated", "required-not-yet-produced"]);
export const q4Statuses = Object.freeze(["accepted-historical", "accepted-current", "open", "planned"]);

export const aiRoadmapSourceContract = "qa/SYNDOCAL_AI_CONTROL_PLANE_ROADMAP.md";
export const aiControlPlaneFlowIds = Object.freeze([
  "AI0-COVERAGE-001",
  "AI1-SCHEMAS-001",
  "AI2-COMMAND-BRIDGE-001",
  "AI3-DURABLE-RECOVERY-001",
  "AI3-NATIVE-INGRESS-001",
  "AI3-PHYSICAL-REARM-001",
  "AI3-DURABLE-ACCEPTANCE-001",
  "AI4-CONSENT-001",
  "AI5-SIDECAR-001",
  "AI6-ADMIN-UI-001",
  "AI7-ADVERSARIAL-PROOF-001",
  "AI8-EXTERNAL-ACCEPTANCE-001",
]);

const stableIdPattern = /^[A-Z][A-Z0-9]*(?:-[A-Z0-9]+)*-\d{3}$/u;
const secondaryIdPattern = /^[A-Z][A-Z0-9]*(?:-[A-Z0-9]+)*$/u;
const shaPattern = /^[0-9a-f]{7,40}$/iu;
const artifactHashPattern = /^[0-9a-f]{64}$/i;
const datePattern = /^\d{4}-\d{2}-\d{2}$/u;

function nonemptyString(value) {
  return typeof value === "string" && value.trim().length > 0;
}

function isPlainObject(value) {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

export function deepEquals(a, b) {
  if (a === b) return true;
  if (Array.isArray(a) && Array.isArray(b)) {
    return a.length === b.length && a.every((entry, index) => deepEquals(entry, b[index]));
  }
  if (isPlainObject(a) && isPlainObject(b)) {
    const aKeys = Object.keys(a);
    const bKeys = Object.keys(b);
    return aKeys.length === bKeys.length
      && aKeys.every((key) => Object.hasOwn(b, key) && deepEquals(a[key], b[key]));
  }
  return false;
}

class Findings {
  constructor() {
    this.entries = [];
  }

  add(code, message) {
    this.entries.push(`${code}: ${message}`);
  }

  throwIfAny() {
    if (this.entries.length > 0) {
      throw new Error(`q1-q4 ledger validation failed (${this.entries.length} finding(s)):\n- ${this.entries.join("\n- ")}`);
    }
  }
}

export function extractMasterMirror(masterText) {
  const marker = `\`\`\`json ${mirrorFenceTag}`;
  const first = String(masterText).indexOf(marker);
  if (first < 0) throw new Error(`MASTER_FENCE_MISSING: master roadmap has no fenced ${mirrorFenceTag} mirror block.`);
  if (String(masterText).indexOf(marker, first + 1) >= 0) {
    throw new Error(`MASTER_FENCE_MULTIPLE: master roadmap must contain exactly one fenced ${mirrorFenceTag} block.`);
  }
  const bodyStart = String(masterText).indexOf("\n", first) + 1;
  const endOffset = String(masterText).indexOf("\n```", bodyStart);
  if (endOffset < 0) throw new Error(`MASTER_FENCE_MISSING: fenced ${mirrorFenceTag} block is never terminated.`);
  return { jsonText: String(masterText).slice(bodyStart, endOffset), startOffset: first, endOffset };
}

export function parseQ0RegistryTables(masterText) {
  const text = String(masterText);
  const q0Start = text.indexOf("### Q0.");
  const coverageMarker = text.indexOf("Source-to-roadmap coverage:");
  const q1Start = text.indexOf("### Q1.");
  if (q0Start < 0 || coverageMarker < 0 || q1Start < 0 || !(q0Start < coverageMarker && coverageMarker < q1Start)) {
    throw new Error("Q0_REGISTRY_PARSE_FAILED: master Q0 section anchors are missing or out of order.");
  }
  const domainSlice = text.slice(q0Start, coverageMarker);
  const sourceSlice = text.slice(coverageMarker, q1Start);
  const rowPattern = /^\|[^\S\r\n]*`([^`]+)`[^|\r\n]*\|/gmu;
  const domainIds = [...domainSlice.matchAll(rowPattern)].map((match) => match[1]);
  const sourcePaths = [...sourceSlice.matchAll(rowPattern)].map((match) => match[1]);
  if (domainIds.length === 0 || sourcePaths.length === 0) {
    throw new Error("Q0_REGISTRY_PARSE_FAILED: master Q0 tables produced no parseable rows.");
  }
  return { domainIds, sourcePaths };
}

function defaultTrackedPathFactory(repoRoot) {
  let cache = null;
  return (path) => {
    if (cache === null) {
      try {
        const listing = execFileSync("git", ["ls-files", "-z"], {
          cwd: repoRoot,
          encoding: "buffer",
          stdio: ["ignore", "pipe", "pipe"],
          maxBuffer: 256 * 1024 * 1024,
        });
        cache = new Set(listing.toString("utf8").split("\0").filter((entry) => entry.length > 0));
      } catch {
        cache = new Set();
      }
    }
    return cache.has(path.split("/").join("/"));
  };
}

function defaultPathExists(repoRoot, path) {
  const candidate = resolve(repoRoot, path);
  const outside = relative(repoRoot, candidate);
  if (outside === "" || outside.startsWith("..") || isAbsolute(outside)) return false;
  try {
    return existsSync(candidate) && lstatSync(candidate).isFile();
  } catch {
    return false;
  }
}

function validateRepositoryPath(findings, context, path, { exists, tracked }, providers) {
  if (!nonemptyString(path)) {
    findings.add("EVIDENCE_PATH_INVALID", `${context} path must be a non-empty repository-relative path.`);
    return false;
  }
  if (isAbsolute(path) || path.includes("\\")) {
    findings.add("EVIDENCE_PATH_INVALID", `${context} path must be repository-relative with forward slashes: ${path}`);
    return false;
  }
  const segments = path.split("/");
  if (segments.includes("..") || segments.includes(".")) {
    findings.add("EVIDENCE_PATH_INVALID", `${context} path must not traverse the repository: ${path}`);
    return false;
  }
  let valid = true;
  if (exists && !providers.pathExists(path)) {
    findings.add("EVIDENCE_PATH_NOT_FOUND", `${context} path does not exist as a regular repository file: ${path}`);
    valid = false;
  }
  if (tracked && !providers.isTracked(path)) {
    findings.add("EVIDENCE_PATH_NOT_TRACKED", `${context} path is not tracked: ${path}`);
    valid = false;
  }
  return valid;
}

function requireKeys(findings, context, container, keys) {
  for (const key of keys) {
    if (!isPlainObject(container) || !Object.hasOwn(container, key)) {
      findings.add("REQUIRED_FIELD_MISSING", `${context} is missing required field "${key}".`);
    }
  }
}

function checkEnum(findings, context, value, allowed, label) {
  if (!allowed.includes(value)) {
    findings.add("INVALID_ENUM_VALUE", `${context} has invalid ${label}: ${JSON.stringify(value)}; allowed: ${allowed.join(", ")}.`);
    return false;
  }
  return true;
}

function checkNonEmpty(findings, context, value, label) {
  if (!nonemptyString(value)) {
    findings.add("REQUIRED_FIELD_MISSING", `${context} is missing a non-empty ${label}.`);
    return false;
  }
  return true;
}

function checkDate(findings, context, value, label) {
  if (!nonemptyString(value) || !datePattern.test(value)) {
    findings.add("DATE_FORMAT_INVALID", `${context} ${label} must be an ISO date (YYYY-MM-DD): ${JSON.stringify(value)}.`);
    return false;
  }
  return true;
}

function validateProofBlock(findings, context, proof, { requiresTopology }, providers) {
  if (!isPlainObject(proof) || !checkEnum(findings, context, proof.status, proofStatuses, "proof status")) {
    return;
  }
  if (!Object.hasOwn(proof, "raw_evidence_paths")) {
    findings.add("REQUIRED_FIELD_MISSING", `${context} is missing required field "raw_evidence_paths".`);
  } else if (!Array.isArray(proof.raw_evidence_paths)) {
    findings.add("REQUIRED_FIELD_MISSING", `${context} raw_evidence_paths must be an array.`);
  }
  if (proof.status === "accepted") {
    if (!nonemptyString(proof.artifact_hash) || !artifactHashPattern.test(proof.artifact_hash)) {
      findings.add("ACCEPTED_PROOF_WITHOUT_ARTIFACT_EVIDENCE", `${context} cannot be Accepted without an exact 64-hex artifact/hash identity.`);
    }
    if (!Array.isArray(proof.raw_evidence_paths) || proof.raw_evidence_paths.length === 0) {
      findings.add("ACCEPTED_PROOF_WITHOUT_ARTIFACT_EVIDENCE", `${context} cannot be Accepted without raw evidence paths.`);
    }
    if (requiresTopology && !nonemptyString(proof.device_topology_duration)) {
      findings.add("ACCEPTED_PROOF_WITHOUT_DEVICE_TOPOLOGY", `${context} hardware acceptance requires device/topology/duration detail.`);
    }
  }
  if (proof.status === "recorded") {
    if (!Array.isArray(proof.raw_evidence_paths) || proof.raw_evidence_paths.length === 0) {
      findings.add("ACCEPTED_PROOF_WITHOUT_ARTIFACT_EVIDENCE", `${context} recorded proof requires at least one raw evidence path.`);
    }
  }
  if (proof.status === "not-applicable" && !nonemptyString(proof.na_reason)) {
    findings.add("PROOF_NOT_APPLICABLE_WITHOUT_REASON", `${context} N/A proof requires an explicit reason.`);
  }
  if (proof.artifact_hash !== null && proof.artifact_hash !== undefined && proof.artifact_hash !== ""
      && !artifactHashPattern.test(String(proof.artifact_hash))) {
    findings.add("ARTIFACT_HASH_FORMAT", `${context} artifact_hash must be null/empty or a 64-hex SHA-256.`);
  }
  if (Array.isArray(proof.raw_evidence_paths)) {
    for (const [index, path] of proof.raw_evidence_paths.entries()) {
      validateRepositoryPath(findings, `${context} raw evidence path ${index + 1}`, path, { exists: true, tracked: true }, providers);
    }
  }
}

export function validateQ1Q4Ledger({
  ledger,
  masterDocument,
  flowDocument,
  completionLedger,
  repoRoot = workspaceRoot,
  pathExists,
  isTracked,
} = {}) {
  const findings = new Findings();
  const providers = {
    pathExists: pathExists ?? ((path) => defaultPathExists(repoRoot, path)),
    isTracked: isTracked ?? defaultTrackedPathFactory(repoRoot),
  };

  if (!isPlainObject(ledger) || ledger.schema_version !== 1 || ledger.ledger_kind !== "SYNDOCAL_Q1_Q4_COVERAGE_MIRROR") {
    findings.add("LEDGER_SHAPE_INVALID", "ledger must be an object with schema_version 1 and ledger_kind SYNDOCAL_Q1_Q4_COVERAGE_MIRROR.");
    findings.throwIfAny();
  }

  const nonclaims = ledger.nonclaims ?? {};
  const overallClaim = String(nonclaims.overall ?? "");
  if (!(overallClaim.includes("does not claim") && overallClaim.includes("COMP-Q1-Q4-001"))) {
    findings.add("NONCLAIM_CONTRACT_VIOLATION", "nonclaims.overall must deny claims and name COMP-Q1-Q4-001.");
  }
  const flowClaim = String(nonclaims.flow_reference ?? "");
  if (!(/referenc/i.test(flowClaim) && /\b(not|never|cannot)\b/i.test(flowClaim) && /(complet|clos|accept)/i.test(flowClaim))) {
    findings.add("NONCLAIM_CONTRACT_VIOLATION", "nonclaims.flow_reference must state that Flow references are traceability only and never closure/completion/acceptance.");
  }
  const historicalClaim = String(nonclaims.historical_evidence ?? "");
  if (!(/historical/i.test(historicalClaim) && /current-source/i.test(historicalClaim) && /(cannot|not|never)/i.test(historicalClaim))) {
    findings.add("NONCLAIM_CONTRACT_VIOLATION", "nonclaims.historical_evidence must state that historical evidence cannot become current-source acceptance.");
  }

  let mirrorJsonText = null;
  try {
    const mirror = extractMasterMirror(masterDocument);
    const q1Heading = masterDocument.indexOf("### Q1.");
    const q5Heading = masterDocument.indexOf("### Q5.");
    if (q1Heading < 0 || q5Heading < 0 || !(mirror.startOffset > q1Heading && mirror.endOffset < q5Heading)) {
      findings.add("MASTER_FENCE_OUTSIDE_Q1_Q4_REGION", "the canonical mirror fence must sit inside the master Q1-Q4 coverage sections.");
    }
    mirrorJsonText = mirror.jsonText;
  } catch (error) {
    findings.add("MASTER_FENCE_EXTRACTION_FAILED", String(error.message ?? error));
  }
  if (mirrorJsonText !== null) {
    try {
      const mirrored = JSON.parse(mirrorJsonText);
      if (!deepEquals(mirrored, ledger)) {
        findings.add("MASTER_JSON_PARITY_MISMATCH", "the master fenced mirror does not exactly equal qa/SYNDOCAL_Q1_Q4_LEDGER.json.");
      }
    } catch {
      findings.add("MASTER_JSON_PARITY_MISMATCH", "the master fenced mirror is not parseable JSON equal to the ledger file.");
    }
  }

  const registry = ledger.q0_registry ?? {};
  const domainRegistryIds = Array.isArray(registry.domains) ? registry.domains.map((entry) => entry?.domain_id) : [];
  const sourceContractPaths = Array.isArray(registry.source_contracts) ? registry.source_contracts.map((entry) => entry?.path) : [];
  let masterDomainIds = [];
  let masterSourcePaths = [];
  try {
    const parsed = parseQ0RegistryTables(masterDocument);
    masterDomainIds = parsed.domainIds;
    masterSourcePaths = parsed.sourcePaths;
  } catch (error) {
    findings.add("Q0_REGISTRY_PARSE_FAILED", String(error.message ?? error));
  }
  if (masterDomainIds.length > 0) {
    if (!deepEquals([...domainRegistryIds].sort(), [...masterDomainIds].sort())) {
      findings.add("Q0_DOMAIN_REGISTRY_MISMATCH", "ledger q0_registry.domains must mirror the master Q0 domain table IDs exactly.");
    }
  }
  if (masterSourcePaths.length > 0) {
    if (!deepEquals([...sourceContractPaths].sort(), [...masterSourcePaths].sort())) {
      findings.add("Q0_SOURCE_REGISTRY_MISMATCH", "ledger q0_registry.source_contracts must mirror the master Q0 source-contract table exactly.");
    }
  }
  if (new Set(domainRegistryIds.filter(Boolean)).size !== domainRegistryIds.length) {
    findings.add("DUPLICATE_ID", "q0_registry.domains contains duplicate domain_id entries.");
  }
  for (const [index, entry] of (Array.isArray(registry.domains) ? registry.domains : []).entries()) {
    const context = `q0_registry.domains[${index}]`;
    if (!isPlainObject(entry)) {
      findings.add("REQUIRED_FIELD_MISSING", `${context} must be an object.`);
      continue;
    }
    if (!nonemptyString(entry.domain_id)) {
      findings.add("REQUIRED_FIELD_MISSING", `${context} domain_id is missing.`);
    }
    checkNonEmpty(findings, context, entry.scope_summary, "scope_summary");
    checkNonEmpty(findings, context, entry.frozen_status_summary, "frozen_status_summary");
  }
  const seenSources = new Set();
  for (const [index, entry] of (Array.isArray(registry.source_contracts) ? registry.source_contracts : []).entries()) {
    const context = `q0_registry.source_contracts[${index}]`;
    if (!isPlainObject(entry)) {
      findings.add("REQUIRED_FIELD_MISSING", `${context} must be an object.`);
      continue;
    }
    if (seenSources.has(entry.path)) findings.add("DUPLICATE_ID", `${context} repeats source contract path ${entry.path}.`);
    seenSources.add(entry.path);
    if (validateRepositoryPath(findings, `${context} path`, entry.path, { exists: true, tracked: true }, providers)) {
      if (!Array.isArray(entry.carried_by_sections) || entry.carried_by_sections.length === 0) {
        findings.add("REQUIRED_FIELD_MISSING", `${context} carried_by_sections must list the master sections carrying the contract.`);
      }
    }
  }

  const flowMarkers = collectAuthoritativeMarkers(flowDocument);
  const markerById = new Map(flowMarkers.map((marker) => [marker.id, marker]));
  const completionItems = Array.isArray(completionLedger?.items) ? completionLedger.items : [];
  const completionById = new Map(completionItems.map((item) => [item.id, item]));
  const statusCountsFromLedger = { Open: 0, Deferred: 0 };
  for (const item of completionItems) {
    if (item.status in statusCountsFromLedger) statusCountsFromLedger[item.status] += 1;
  }
  const markerIdSet = new Set(markerById.keys());
  const completionIdSet = new Set(completionById.keys());
  if (markerIdSet.size !== completionIdSet.size || [...markerIdSet].some((id) => !completionIdSet.has(id))) {
    findings.add("COMPLETION_LEDGER_MARKER_DRIFT", "completion ledger items and Flow authoritative markers diverge.");
  }
  for (const [id, marker] of markerById) {
    const item = completionById.get(id);
    if (item && item.status !== marker.kind) {
      findings.add("COMPLETION_LEDGER_MARKER_DRIFT", `completion ledger status for ${id} (${item.status}) diverges from Flow marker kind (${marker.kind}).`);
    }
  }
  const declaredFlowCounts = ledger.expected_counts?.flow_markers ?? {};
  if (declaredFlowCounts.Open !== statusCountsFromLedger.Open || declaredFlowCounts.Deferred !== statusCountsFromLedger.Deferred) {
    findings.add("STALE_EXPECTED_COUNTS", "expected_counts.flow_markers must equal the live completion-ledger Open/Deferred counts.");
  }
  const infraItem = completionById.get("COMP-Q1-Q4-001");
  if (!infraItem || infraItem.status !== "Open") {
    findings.add("COMPLETION_INFRA_ROW_CLOSED", "COMP-Q1-Q4-001 must remain Open while this coverage infrastructure tranche stands.");
  }

  const q1Rows = Array.isArray(ledger.q1_requirements) ? ledger.q1_requirements : [];
  const q2Rows = Array.isArray(ledger.q2_decisions) ? ledger.q2_decisions : [];
  const q3Rows = Array.isArray(ledger.q3_risks) ? ledger.q3_risks : [];
  const q4Rows = Array.isArray(ledger.q4_evidence) ? ledger.q4_evidence : [];

  const domainSet = new Set(domainRegistryIds.filter(Boolean));
  const sourceSet = new Set(sourceContractPaths.filter(Boolean));
  const coveredDomains = new Set();
  const coveredSources = new Set();
  const coveredFlowMarkers = new Set();
  const seenQ1Ids = new Set();
  const q4RowById = new Map(q4Rows.filter((row) => isPlainObject(row)).map((row) => [row.id, row]));
  const referencedDecisionIds = new Set();
  const referencedRiskIds = new Set();
  const referencedEvidenceIds = new Set();

  for (const [index, row] of q1Rows.entries()) {
    const context = `q1_requirements[${index}] ${typeof row?.id === "string" ? row.id : "(unidentified)"}`;
    if (!isPlainObject(row)) {
      findings.add("REQUIRED_FIELD_MISSING", `${context} must be an object.`);
      continue;
    }
    requireKeys(findings, context, row, [
      "id", "requirement", "failure_behavior", "source", "source_contracts", "domains", "flow_refs", "scope",
      "owner_files", "dependencies", "automated_proof", "native_proof", "hardware_external_proof", "status",
      "commit", "residual_risk", "nonclaim", "decision_ids", "risk_ids", "evidence_ids",
    ]);
    if (!nonemptyString(row.id) || !stableIdPattern.test(row.id)) {
      findings.add("INVALID_ID_FORMAT", `${context} id must be a stable domain-prefixed ID (e.g. MEDIA-AUTH-001).`);
    }
    if (nonemptyString(row.id)) {
      if (seenQ1Ids.has(row.id)) findings.add("DUPLICATE_ID", `q1_requirements repeats ID ${row.id}.`);
      seenQ1Ids.add(row.id);
    }
    checkEnum(findings, context, row.scope, q1Scopes, "scope");
    checkEnum(findings, context, row.status, q1Statuses, "requirement status");
    checkNonEmpty(findings, context, row.requirement, "requirement");
    checkNonEmpty(findings, context, row.failure_behavior, "failure_behavior");
    checkNonEmpty(findings, context, row.source, "source");
    checkNonEmpty(findings, context, row.owner_files, "owner_files");
    if (!nonemptyString(row.nonclaim) || !/does not/i.test(row.nonclaim)) {
      findings.add("NONCLAIM_CONTRACT_VIOLATION", `${context} nonclaim must explicitly deny overclaim ("does not ...").`);
    }
    if (!Array.isArray(row.dependencies) || row.dependencies.length === 0 || !row.dependencies.every(nonemptyString)) {
      findings.add("REQUIRED_FIELD_MISSING", `${context} dependencies must be a non-empty string array.`);
    }

    for (const domain of Array.isArray(row.domains) ? row.domains : []) {
      if (!domainSet.has(domain)) {
        findings.add("UNKNOWN_DOMAIN_REFERENCE", `${context} references unknown Q0 domain ${JSON.stringify(domain)}.`);
      }
      coveredDomains.add(domain);
    }
    for (const source of Array.isArray(row.source_contracts) ? row.source_contracts : []) {
      if (!sourceSet.has(source)) {
        findings.add("UNKNOWN_SOURCE_CONTRACT_REFERENCE", `${context} references unknown Q0 source contract ${JSON.stringify(source)}.`);
      }
      coveredSources.add(source);
    }
    for (const flowRef of Array.isArray(row.flow_refs) ? row.flow_refs : []) {
      if (!markerById.has(flowRef)) {
        findings.add("UNKNOWN_FLOW_MARKER_REFERENCE", `${context} references unknown Flow marker ${JSON.stringify(flowRef)}.`);
        continue;
      }
      coveredFlowMarkers.add(flowRef);
      if (row.status === "Accepted") {
        const markerItem = completionById.get(flowRef);
        if (markerItem && (markerItem.status === "Open" || markerItem.status === "Deferred")) {
          findings.add("ACCEPTED_ROW_REFERENCES_OPEN_FLOW_MARKER", `${context} is Accepted yet still cites ${flowRef}, whose completion-ledger status is ${markerItem.status}.`);
        }
      }
    }

    const automated = row.automated_proof ?? {};
    if (checkEnum(findings, context, automated.status, ["passing", "not-passing"], "automated_proof.status")) {
      if (automated.status === "passing") {
        if (!nonemptyString(automated.command)) {
          findings.add("AUTOMATED_PROOF_COMMAND_REQUIRED", `${context} claims passing automated proof without an exact command.`);
        }
        if (!Number.isInteger(automated.expected_count) || automated.expected_count <= 0) {
          findings.add("AUTOMATED_PROOF_ZERO_COUNT", `${context} claims passing automated proof without a nonzero expected assertion/count.`);
        }
      }
    } else if (Object.keys(automated).length === 0) {
      findings.add("REQUIRED_FIELD_MISSING", `${context} is missing automated_proof.`);
    }

    validateProofBlock(findings, `${context} native_proof`, row.native_proof, { requiresTopology: false }, providers);
    validateProofBlock(findings, `${context} hardware_external_proof`, row.hardware_external_proof, { requiresTopology: true }, providers);

    if (row.status === "Accepted") {
      const nativeAccepted = row.native_proof?.status === "accepted";
      const hardwareOk = row.hardware_external_proof?.status === "accepted" || row.hardware_external_proof?.status === "not-applicable";
      if (!nativeAccepted || !hardwareOk) {
        findings.add("ROW_STATUS_ACCEPTED_WITHOUT_ACCEPTED_PROOFS", `${context} is Accepted without accepted native proof and accepted/not-applicable hardware proof.`);
      }
    }
    if (row.native_proof?.status === "accepted" || row.hardware_external_proof?.status === "accepted") {
      if (row.status !== "Accepted") {
        findings.add("ACCEPTED_PROOF_ROW_NOT_ACCEPTED", `${context} carries accepted native/hardware proof while the row itself is ${JSON.stringify(row.status)}.`);
      }
      for (const evidenceId of Array.isArray(row.evidence_ids) ? row.evidence_ids : []) {
        const cited = q4RowById.get(evidenceId);
        if (cited && cited.kind === "historical-evidence") {
          findings.add("HISTORICAL_EVIDENCE_PROMOTED_TO_CURRENT_ACCEPTANCE", `${context} cites historical evidence ${evidenceId} alongside an accepted claim; historical evidence cannot become current-source acceptance.`);
        }
      }
    }

    const residual = row.residual_risk ?? {};
    checkEnum(findings, context, residual.classification, residualClasses, "residual_risk.classification");
    checkNonEmpty(findings, context, residual.disposition, "residual_risk.disposition");

    if (["Implemented", "Reviewed", "Accepted"].includes(row.status)) {
      if (typeof row.commit !== "string" || row.commit === "" || /^head/i.test(row.commit) || !shaPattern.test(row.commit)) {
        findings.add("COMMIT_MUST_BE_SHA", `${context} with status ${row.status} requires a focused commit SHA, never HEAD or an empty value.`);
      }
    } else if (typeof row.commit === "string" && row.commit !== "" && !shaPattern.test(row.commit)) {
      findings.add("COMMIT_MUST_BE_SHA", `${context} commit must be empty or a focused commit SHA.`);
    }

    for (const target of Array.isArray(row.decision_ids) ? row.decision_ids : []) referencedDecisionIds.add(target);
    for (const target of Array.isArray(row.risk_ids) ? row.risk_ids : []) referencedRiskIds.add(target);
    for (const target of Array.isArray(row.evidence_ids) ? row.evidence_ids : []) referencedEvidenceIds.add(target);
  }

  for (const [collection, label] of [[q2Rows, "q2_decisions"], [q3Rows, "q3_risks"], [q4Rows, "q4_evidence"]]) {
    const seen = new Set();
    for (const [index, row] of collection.entries()) {
      const context = `${label}[${index}] ${typeof row?.id === "string" ? row.id : "(unidentified)"}`;
      if (!isPlainObject(row) || !nonemptyString(row.id) || !secondaryIdPattern.test(row.id)) {
        findings.add("INVALID_ID_FORMAT", `${context} requires a stable ID.`);
        continue;
      }
      if (seen.has(row.id)) findings.add("DUPLICATE_ID", `${label} repeats ID ${row.id}.`);
      seen.add(row.id);
    }
  }

  for (const [index, row] of q2Rows.entries()) {
    const context = `q2_decisions[${index}] ${row?.id ?? ""}`;
    if (!isPlainObject(row)) continue;
    requireKeys(findings, context, row, ["id", "decision", "state", "date", "approver", "alternatives", "consequences", "linked_q1_ids"]);
    checkNonEmpty(findings, context, row.decision, "decision");
    checkEnum(findings, context, row.state, q2States, "decision state");
    checkDate(findings, context, row.date, "date");
    checkNonEmpty(findings, context, row.approver, "approver");
    if (!Array.isArray(row.alternatives) || row.alternatives.length === 0 || !row.alternatives.every(nonemptyString)) {
      findings.add("REQUIRED_FIELD_MISSING", `${context} alternatives must be a non-empty string array.`);
    }
    checkNonEmpty(findings, context, row.consequences, "consequences");
    if (!Array.isArray(row.linked_q1_ids) || row.linked_q1_ids.length === 0 || !referencedDecisionIds.has(row.id)) {
      findings.add("ORPHAN_DECISION", `${context} is not referenced by any Q1 requirement row.`);
    }
  }

  for (const [index, row] of q3Rows.entries()) {
    const context = `q3_risks[${index}] ${row?.id ?? ""}`;
    if (!isPlainObject(row)) continue;
    requireKeys(findings, context, row, [
      "id", "severity", "reproduction", "affected_scope", "likelihood", "owner", "mitigation",
      "blocking_milestone", "proof_needed", "status", "last_reviewed", "linked_q1_ids",
    ]);
    checkEnum(findings, context, row.severity, q3Severities, "severity");
    checkEnum(findings, context, row.likelihood, q3Likelihoods, "likelihood");
    checkEnum(findings, context, row.status, q3Statuses, "risk status");
    checkNonEmpty(findings, context, row.reproduction, "reproduction");
    checkNonEmpty(findings, context, row.affected_scope, "affected_scope");
    checkNonEmpty(findings, context, row.owner, "owner");
    checkNonEmpty(findings, context, row.mitigation, "mitigation");
    checkNonEmpty(findings, context, row.blocking_milestone, "blocking_milestone");
    checkNonEmpty(findings, context, row.proof_needed, "proof_needed");
    checkDate(findings, context, row.last_reviewed, "last_reviewed");
    if (row.escalation !== null && row.escalation !== undefined && !nonemptyString(row.escalation)) {
      findings.add("REQUIRED_FIELD_MISSING", `${context} escalation must be null or a non-empty string.`);
    }
    if (row.status === "Closed" && !nonemptyString(row.closing_commit) && !nonemptyString(row.closing_note)) {
      findings.add("REQUIRED_FIELD_MISSING", `${context} is Closed and requires closing_commit or closing_note evidence.`);
    }
    if (nonemptyString(row.closing_commit) && !shaPattern.test(row.closing_commit)) {
      findings.add("COMMIT_MUST_BE_SHA", `${context} closing_commit must be a focused commit SHA.`);
    }
    if (!Array.isArray(row.linked_q1_ids) || row.linked_q1_ids.length === 0 || !referencedRiskIds.has(row.id)) {
      findings.add("ORPHAN_RISK", `${context} is not referenced by any Q1 requirement row.`);
    }
  }

  for (const [index, row] of q4Rows.entries()) {
    const context = `q4_evidence[${index}] ${row?.id ?? ""}`;
    if (!isPlainObject(row)) continue;
    requireKeys(findings, context, row, [
      "id", "scope", "kind", "status", "date", "command", "exit_code", "assertion_count", "ignored_count",
      "warning_count", "artifact_hash", "artifact_na_reason", "raw_evidence_paths", "currency_nonclaim", "linked_q1_ids",
    ]);
    checkNonEmpty(findings, context, row.scope, "scope");
    checkDate(findings, context, row.date, "date");
    const kindOk = checkEnum(findings, context, row.kind, q4Kinds, "evidence kind");
    const statusOk = checkEnum(findings, context, row.status, q4Statuses, "evidence status");
    if (kindOk && statusOk) {
      if (row.kind === "historical-evidence" && !["accepted-historical", "open"].includes(row.status)) {
        findings.add("INVALID_ENUM_VALUE", `${context} historical evidence cannot carry status ${row.status}.`);
      }
      if (row.kind === "current-source-automated" && !["accepted-current", "open"].includes(row.status)) {
        findings.add("INVALID_ENUM_VALUE", `${context} current-source evidence cannot carry status ${row.status}.`);
      }
      if (row.kind === "required-not-yet-produced" && row.status !== "planned") {
        findings.add("INVALID_ENUM_VALUE", `${context} required-not-yet-produced evidence must stay planned.`);
      }
    }
    if (!nonemptyString(row.currency_nonclaim) || !/current-source/i.test(row.currency_nonclaim) || !/(cannot|not|never)/i.test(row.currency_nonclaim)) {
      findings.add("NONCLAIM_CONTRACT_VIOLATION", `${context} currency_nonclaim must deny current-source acceptance explicitly.`);
    }
    if (row.status === "accepted-current") {
      if (!nonemptyString(row.command) || row.exit_code !== 0 || !Number.isInteger(row.assertion_count) || row.assertion_count <= 0) {
        findings.add("Q4_ACCEPTED_CURRENT_INCOMPLETE", `${context} accepted-current evidence requires command, exit_code 0, and a nonzero assertion count.`);
      }
      if (!Number.isInteger(row.ignored_count) || !Number.isInteger(row.warning_count)) {
        findings.add("Q4_ACCEPTED_CURRENT_INCOMPLETE", `${context} accepted-current evidence requires integer ignored/warning counts.`);
      }
      if (!nonemptyString(row.artifact_hash) && !nonemptyString(row.artifact_na_reason)) {
        findings.add("Q4_ACCEPTED_CURRENT_INCOMPLETE", `${context} accepted-current evidence requires an artifact hash or an explicit artifact_na_reason.`);
      }
      if (!Array.isArray(row.raw_evidence_paths) || row.raw_evidence_paths.length === 0) {
        findings.add("Q4_ACCEPTED_CURRENT_INCOMPLETE", `${context} accepted-current evidence requires raw evidence paths.`);
      }
    }
    if (row.status === "planned" && !nonemptyString(row.required_for)) {
      findings.add("Q4_PLANNED_WITHOUT_REQUIRED_SCOPE", `${context} planned evidence requires required_for naming the owning milestone.`);
    }
    if (Array.isArray(row.raw_evidence_paths)) {
      for (const [pathIndex, path] of row.raw_evidence_paths.entries()) {
        validateRepositoryPath(findings, `${context} raw evidence path ${pathIndex + 1}`, path, { exists: true, tracked: true }, providers);
      }
    }
    if (!Array.isArray(row.linked_q1_ids) || row.linked_q1_ids.length === 0 || !referencedEvidenceIds.has(row.id)) {
      findings.add("ORPHAN_EVIDENCE", `${context} is not referenced by any Q1 requirement row.`);
    }
  }

  const q1ById = new Map(q1Rows.filter((row) => isPlainObject(row) && nonemptyString(row.id)).map((row) => [row.id, row]));
  const linkChecks = [
    ["decision_ids", q2Rows, "DECISION"],
    ["risk_ids", q3Rows, "RISK"],
    ["evidence_ids", q4Rows, "EVIDENCE"],
  ];
  for (const [field, collection, label] of linkChecks) {
    const collectionIds = new Set(collection.filter((row) => isPlainObject(row)).map((row) => row.id));
    for (const row of q1Rows) {
      if (!isPlainObject(row)) continue;
      for (const target of Array.isArray(row[field]) ? row[field] : []) {
        if (!collectionIds.has(target)) {
          findings.add("UNKNOWN_LINK_TARGET", `Q1 row ${row.id} ${field} references unknown ${label} ${JSON.stringify(target)}.`);
          continue;
        }
        const counterpart = collection.find((entry) => isPlainObject(entry) && entry.id === target);
        if (counterpart && Array.isArray(counterpart.linked_q1_ids) && !counterpart.linked_q1_ids.includes(row.id)) {
          findings.add("LINK_ASYMMETRY", `${label} ${target}.linked_q1_ids does not include citing Q1 row ${row.id}.`);
        }
      }
    }
    for (const entry of collection) {
      if (!isPlainObject(entry)) continue;
      for (const backref of Array.isArray(entry.linked_q1_ids) ? entry.linked_q1_ids : []) {
        if (!q1ById.has(backref)) {
          findings.add("UNKNOWN_LINK_TARGET", `${label} ${entry.id} linked_q1_ids references unknown Q1 row ${JSON.stringify(backref)}.`);
        }
      }
    }
  }

  for (const domain of domainSet) {
    if (!coveredDomains.has(domain)) {
      findings.add("MISSING_DOMAIN_COVERAGE", `no Q1 requirement row represents Q0 domain ${domain}.`);
    }
  }
  for (const source of sourceSet) {
    if (!coveredSources.has(source)) {
      findings.add("MISSING_SOURCE_COVERAGE", `no Q1 requirement row represents Q0 source contract ${source}.`);
    }
  }
  for (const markerId of markerIdSet) {
    if (!coveredFlowMarkers.has(markerId)) {
      findings.add("FLOW_MARKER_UNREFERENCED", `Flow marker ${markerId} is not referenced by any Q1 requirement row.`);
    }
  }

  const expectedCounts = ledger.expected_counts ?? {};
  const actualCounts = {
    q0_domains: domainRegistryIds.length,
    q0_source_contracts: sourceContractPaths.length,
    flow_markers: { Open: statusCountsFromLedger.Open, Deferred: statusCountsFromLedger.Deferred },
    q1_rows: q1Rows.length,
    q2_decisions: q2Rows.length,
    q3_risks: q3Rows.length,
    q4_evidence: q4Rows.length,
  };
  for (const key of ["q0_domains", "q0_source_contracts", "q1_rows", "q2_decisions", "q3_risks", "q4_evidence"]) {
    if (expectedCounts[key] !== actualCounts[key]) {
      findings.add("STALE_EXPECTED_COUNTS", `expected_counts.${key} is ${JSON.stringify(expectedCounts[key])} but the ledger carries ${actualCounts[key]}.`);
    }
  }

  const aiRows = q1Rows.filter((row) => isPlainObject(row) && Array.isArray(row.domains) && row.domains.includes("AI-CONTROL"));
  if (aiRows.length === 0) {
    findings.add("AI_CONTROL_LINKAGE_FLATTENED", "no Q1 row owns the AI-CONTROL domain.");
  } else {
    const aiSources = new Set(aiRows.flatMap((row) => Array.isArray(row.source_contracts) ? row.source_contracts : []));
    if (!aiSources.has(aiRoadmapSourceContract)) {
      findings.add("AI_CONTROL_LINKAGE_FLATTENED", "AI-CONTROL rows must keep the dedicated AI control-plane roadmap source contract linked.");
    }
    const aiDecisions = new Set(aiRows.flatMap((row) => Array.isArray(row.decision_ids) ? row.decision_ids : []));
    for (const requiredDecision of ["DEC-AI-ARCH-001", "DEC-AI-CONSENT-001"]) {
      if (!aiDecisions.has(requiredDecision)) {
        findings.add("AI_CONTROL_LINKAGE_FLATTENED", `AI-CONTROL rows must keep ${requiredDecision} linked.`);
      }
    }
    const aiRisks = new Set(aiRows.flatMap((row) => Array.isArray(row.risk_ids) ? row.risk_ids : []));
    for (const requiredRisk of ["R-AI-BYPASS-001", "R-AI-SAFETY-001"]) {
      if (!aiRisks.has(requiredRisk)) {
        findings.add("AI_CONTROL_LINKAGE_FLATTENED", `AI-CONTROL rows must keep ${requiredRisk} linked.`);
      }
    }
    const aiReferenced = new Set(aiRows.flatMap((row) => Array.isArray(row.flow_refs) ? row.flow_refs : []));
    for (const aiFlowId of aiControlPlaneFlowIds) {
      if (!aiReferenced.has(aiFlowId)) {
        findings.add("AI_CONTROL_LINKAGE_FLATTENED", `AI control-plane marker ${aiFlowId} must be referenced by a dedicated AI-CONTROL row, not flattened elsewhere.`);
      }
    }
  }

  findings.throwIfAny();
  return {
    q1Rows: q1Rows.length,
    q2Decisions: q2Rows.length,
    q3Risks: q3Rows.length,
    q4Evidence: q4Rows.length,
    domainsCovered: coveredDomains.size,
    sourceContractsCovered: coveredSources.size,
    flowMarkersReferenced: coveredFlowMarkers.size,
    flowMarkerCounts: { Open: statusCountsFromLedger.Open, Deferred: statusCountsFromLedger.Deferred },
  };
}

export function runQ1Q4LedgerCheck({ repoRoot = workspaceRoot } = {}) {
  const ledger = JSON.parse(readFileSync(resolve(repoRoot, q1Q4LedgerPath), "utf8"));
  const masterDocument = readFileSync(resolve(repoRoot, masterRoadmapPath), "utf8");
  const flowDocument = readFileSync(resolve(repoRoot, flowDocumentPath), "utf8");
  const completionLedger = JSON.parse(readFileSync(resolve(repoRoot, completionLedgerPath), "utf8"));
  return validateQ1Q4Ledger({ ledger, masterDocument, flowDocument, completionLedger, repoRoot });
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const result = runQ1Q4LedgerCheck();
  console.log(
    `q1-q4 coverage ledger ok; ${result.q1Rows} Q1 rows cover ${result.domainsCovered}/29 Q0 domains and `
      + `${result.sourceContractsCovered}/10 source contracts; ${result.flowMarkersReferenced}/58 Flow markers referenced `
      + `(${result.flowMarkerCounts.Open} Open + ${result.flowMarkerCounts.Deferred} Deferred preserved); `
      + `${result.q2Decisions} decisions, ${result.q3Risks} risks, ${result.q4Evidence} evidence records linked; `
      + "master mirror parity verified",
  );
}
