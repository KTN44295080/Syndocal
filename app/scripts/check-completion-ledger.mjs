import { execFileSync } from "node:child_process";
import { existsSync, lstatSync, readFileSync } from "node:fs";
import { dirname, isAbsolute, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const scriptDirectory = dirname(fileURLToPath(import.meta.url));
export const workspaceRoot = resolve(scriptDirectory, "..", "..");
export const ledgerPath = "qa/SYNDOCAL_COMPLETION_LEDGER.json";
export const sourceDocumentPath = "qa/SYNDOCAL_COMPLETION_FLOW_2026-08-19.md";
export const requiredSections = Object.freeze(["6", "7", "8", "9"]);
export const allowedClassifications = Object.freeze([
  "Supported",
  "Deferred",
  "External acceptance",
  "Out of scope",
]);
export const allowedSourceKinds = Object.freeze(["Open", "Deferred", "Complete"]);
export const expectedStatusCounts = Object.freeze({ Open: 40, Deferred: 8, Complete: 10 });
export const expectedSectionCounts = Object.freeze({ "6": 37, "7": 6, "8": 9, "9": 6 });

const markerPattern = /<!--\s*completion-ledger:\s*(Open|Deferred|Complete):\s*([A-Z0-9-]+)\s*-->/gu;
const stableIdPattern = /^[A-Z][A-Z0-9]*(?:-[A-Z0-9]+)*-\d{3}$/u;

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function nonemptyString(value) {
  return typeof value === "string" && value.trim().length > 0;
}

function safeRepositoryPath(path, label) {
  assert(nonemptyString(path), `${label} must be a non-empty repository-relative path.`);
  assert(!isAbsolute(path), `${label} must not be absolute: ${path}`);
  assert(!path.includes("\\"), `${label} must use forward slashes: ${path}`);
  const normalized = path.split("/");
  assert(!normalized.includes("..") && !normalized.includes("."), `${label} must not traverse the repository: ${path}`);
  return path;
}

function defaultTrackedPath(repoRoot, path) {
  try {
    execFileSync("git", ["ls-files", "--error-unmatch", "--", path], {
      cwd: repoRoot,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
    });
    return true;
  } catch {
    return false;
  }
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

export function collectAuthoritativeMarkers(documentText) {
  const markers = [];
  const sectionsWithMarkers = new Set();
  let currentSection = null;
  for (const [index, line] of String(documentText).split(/\r?\n/u).entries()) {
    const heading = /^##\s+(\d+)\./u.exec(line);
    if (heading) currentSection = heading[1];
    if (!requiredSections.includes(currentSection)) continue;

    const markedRows = [...line.matchAll(markerPattern)].map((match) => ({ kind: match[1], id: match[2] }));
    const hasLedgerMarker = /<!--\s*completion-ledger:/u.test(line);
    const openCheckbox = /^- \[ \]/u.test(line);
    const completeCheckbox = /^- \[x\]/iu.test(line);
    const explicitDeferred = /^- \*\*Deferred outside the Windows target:/u.test(line);
    const frozenDeferred = /^- \*\*Frozen \/ out of scope for the current Windows-local completion target:/u.test(line);
    const deferredRow = explicitDeferred || frozenDeferred;
    if (hasLedgerMarker && markedRows.length === 0) {
      throw new Error(`completion-ledger marker at ${sourceDocumentPath}:${index + 1} has an invalid kind or format.`);
    }
    if (openCheckbox && (markedRows.length !== 1 || markedRows[0].kind !== "Open")) {
      throw new Error(`unmapped authoritative open row at ${sourceDocumentPath}:${index + 1}: exactly one Open marker is required.`);
    }
    if (completeCheckbox && markedRows.length > 0 && (markedRows.length !== 1 || markedRows[0].kind !== "Complete")) {
      throw new Error(`unmapped authoritative completed row at ${sourceDocumentPath}:${index + 1}: exactly one Complete marker is required.`);
    }
    if (!openCheckbox && !completeCheckbox && markedRows.length > 0 && !deferredRow) {
      throw new Error(`arbitrary completion-ledger marker at ${sourceDocumentPath}:${index + 1} is not an authority row.`);
    }
    if (deferredRow && (markedRows.length !== 1 || markedRows[0].kind !== "Deferred")) {
      throw new Error(`unmapped authoritative deferred row at ${sourceDocumentPath}:${index + 1}: exactly one Deferred marker is required.`);
    }
    for (const marker of markedRows) {
      markers.push({ ...marker, section: currentSection, line: index + 1 });
      sectionsWithMarkers.add(currentSection);
    }
  }
  for (const section of requiredSections) {
    assert(sectionsWithMarkers.has(section), `authoritative section ${section} has no ledger markers.`);
  }
  return markers;
}

function parseLedger(ledgerText) {
  try {
    return JSON.parse(ledgerText);
  } catch (error) {
    throw new Error(`completion ledger is not valid JSON: ${error.message}`);
  }
}

export function validateCompletionLedger({
  ledger,
  sourceDocument,
  repoRoot = workspaceRoot,
  pathExists = (path) => defaultPathExists(repoRoot, path),
  isTracked = (path) => defaultTrackedPath(repoRoot, path),
} = {}) {
  assert(ledger && typeof ledger === "object" && !Array.isArray(ledger), "completion ledger must be an object.");
  assert(ledger.schema_version === 1, "completion ledger schema_version must be 1.");
  assert(ledger.source_document === sourceDocumentPath, `completion ledger source_document must be ${sourceDocumentPath}.`);
  assert(
    Array.isArray(ledger.authoritative_sections)
      && ledger.authoritative_sections.length === requiredSections.length
      && requiredSections.every((section) => ledger.authoritative_sections.includes(section)),
    "completion ledger authoritative_sections must be exactly 6, 7, 8, and 9.",
  );
  assert(
    Array.isArray(ledger.classifications)
      && ledger.classifications.length === allowedClassifications.length
      && allowedClassifications.every((classification) => ledger.classifications.includes(classification)),
    "completion ledger classifications must be the exact supported/deferred/external/out-of-scope set.",
  );
  assert(
    ledger.expected_status_counts
      && typeof ledger.expected_status_counts === "object"
      && JSON.stringify(ledger.expected_status_counts) === JSON.stringify(expectedStatusCounts),
    "completion ledger expected_status_counts must match the current Open/Deferred/Complete counts.",
  );
  assert(
    ledger.expected_section_counts
      && typeof ledger.expected_section_counts === "object"
      && requiredSections.every((section) => ledger.expected_section_counts[section] === expectedSectionCounts[section])
      && Object.keys(ledger.expected_section_counts).length === Object.keys(expectedSectionCounts).length,
    "completion ledger expected_section_counts must be exactly 6=37, 7=6, 8=9, and 9=6.",
  );
  assert(Array.isArray(ledger.items) && ledger.items.length > 0, "completion ledger must contain authority rows.");

  const markers = collectAuthoritativeMarkers(sourceDocument);
  const markerById = new Map();
  for (const marker of markers) {
    assert(stableIdPattern.test(marker.id), `authoritative marker ${marker.id} at ${sourceDocumentPath}:${marker.line} is not a stable ID.`);
    assert(!markerById.has(marker.id), `duplicate authoritative marker ID ${marker.id} at ${sourceDocumentPath}:${marker.line}.`);
    markerById.set(marker.id, marker);
  }

  const ledgerById = new Map();
  const statusCounts = { Open: 0, Deferred: 0, Complete: 0 };
  for (const [index, item] of ledger.items.entries()) {
    const label = `ledger item ${index + 1}`;
    assert(item && typeof item === "object" && !Array.isArray(item), `${label} must be an object.`);
    assert(nonemptyString(item.id) && stableIdPattern.test(item.id), `${label} has an invalid stable id.`);
    assert(!ledgerById.has(item.id), `duplicate ledger ID ${item.id}.`);
    assert(item.source && typeof item.source === "object" && !Array.isArray(item.source), `${item.id} is missing source.`);
    assert(requiredSections.includes(item.source.section), `${item.id} has an unsupported authoritative section.`);
    assert(allowedSourceKinds.includes(item.source.kind), `${item.id} is missing a valid source kind.`);
    assert(allowedClassifications.includes(item.classification), `${item.id} is missing a valid classification.`);
    assert(nonemptyString(item.owner), `${item.id} is missing owner.`);
    assert(nonemptyString(item.dependency), `${item.id} is missing dependency.`);
    assert(nonemptyString(item.nonclaim) && /does not claim/iu.test(item.nonclaim), `${item.id} is missing an explicit nonclaim.`);
    if (item.classification === "External acceptance") {
      assert(
        item.source.kind === "Open" && item.status === "Open",
        `external acceptance ${item.id} must remain Open and may never be Deferred or Complete.`,
      );
    }
    assert(item.status === "Open" || item.status === "Deferred" || item.status === "Complete", `${item.id} has an invalid status.`);
    if (item.source.kind === "Open") {
      assert(item.status === "Open", `open source row ${item.id} must remain status Open.`);
      assert(
        item.classification === "Supported" || item.classification === "External acceptance",
        `open source row ${item.id} must be Supported or External acceptance.`,
      );
    } else if (item.source.kind === "Deferred") {
      assert(item.status === "Deferred", `deferred source row ${item.id} must remain status Deferred.`);
      assert(
        item.classification === "Deferred" || item.classification === "Out of scope",
        `deferred source row ${item.id} must be Deferred or Out of scope.`,
      );
    } else {
      assert(item.status === "Complete", `completed source row ${item.id} must remain status Complete.`);
      assert(item.classification === "Supported", `completed source row ${item.id} must remain Supported.`);
    }
    statusCounts[item.status] += 1;
    assert(Array.isArray(item.evidence_paths) && item.evidence_paths.length > 0, `${item.id} is missing evidence paths.`);
    const uniqueEvidence = new Set();
    for (const [evidenceIndex, evidencePath] of item.evidence_paths.entries()) {
      const path = safeRepositoryPath(evidencePath, `${item.id} evidence path ${evidenceIndex + 1}`);
      assert(!uniqueEvidence.has(path), `${item.id} repeats evidence path ${path}.`);
      uniqueEvidence.add(path);
      assert(pathExists(path), `${item.id} evidence path does not exist as a tracked regular file: ${path}`);
      assert(isTracked(path), `${item.id} evidence path is not tracked: ${path}`);
    }
    ledgerById.set(item.id, item);
  }

  for (const [id, marker] of markerById) {
    const item = ledgerById.get(id);
    assert(item, `unmapped authoritative item ${id} at ${sourceDocumentPath}:${marker.line}.`);
    assert(item.source.section === marker.section, `stale ledger ID ${id}: source section ${item.source.section} does not match ${marker.section}.`);
    assert(item.source.kind === marker.kind, `stale ledger ID ${id}: source kind ${item.source.kind} does not match ${marker.kind}.`);
  }
  for (const id of ledgerById.keys()) {
    assert(markerById.has(id), `stale ledger ID ${id}: no authoritative marker remains in ${sourceDocumentPath}.`);
  }

  assert(
    JSON.stringify(statusCounts) === JSON.stringify(expectedStatusCounts),
    `completion ledger must retain ${expectedStatusCounts.Open} Open + ${expectedStatusCounts.Deferred} Deferred + ${expectedStatusCounts.Complete} Complete rows.`,
  );
  const sectionCounts = Object.fromEntries(
    requiredSections.map((section) => [section, markers.filter((marker) => marker.section === section).length]),
  );
  for (const section of requiredSections) {
    assert(
      sectionCounts[section] === expectedSectionCounts[section],
      `authoritative section ${section} must retain ${expectedSectionCounts[section]} ledger rows.`,
    );
  }

  return {
    itemCount: ledgerById.size,
    sections: [...new Set(markers.map((marker) => marker.section))],
    sectionCounts,
    statusCounts,
  };
}

export function runCompletionLedgerCheck({ repoRoot = workspaceRoot } = {}) {
  const ledger = parseLedger(readFileSync(resolve(repoRoot, ledgerPath), "utf8"));
  const sourceDocument = readFileSync(resolve(repoRoot, sourceDocumentPath), "utf8");
  return validateCompletionLedger({ ledger, sourceDocument, repoRoot });
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const result = runCompletionLedgerCheck();
  console.log(
    `completion ledger ok; ${result.statusCounts.Open} Open + ${result.statusCounts.Deferred} Deferred + ${result.statusCounts.Complete} Complete authority rows `
      + `(${result.itemCount} total; sections 6=${result.sectionCounts["6"]}, 7=${result.sectionCounts["7"]}, `
      + `8=${result.sectionCounts["8"]}, 9=${result.sectionCounts["9"]})`,
  );
}
