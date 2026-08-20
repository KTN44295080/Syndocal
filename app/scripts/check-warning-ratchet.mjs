import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  collectModifiedFiles,
  compareArtifactCoverage,
  compareDiagnostics,
  detectHostPlatform,
  detectToolchain,
  findAddedSuppressions,
  findBaselineLaundering,
  getConfiguration,
  loadInventory,
  loadInventoryAtRef,
  resolveTrustedComparison,
  runCargoConfiguration,
  validateInventory,
} from "./warning-ratchet-lib.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(scriptDir, "../..");
const inventoryPath = path.join(repoRoot, "qa/warnings/warning-inventory.json");
const schemaPath = path.join(repoRoot, "qa/warnings/warning-inventory.schema.json");

function parseArgs(values) {
  const result = {
    configuration: null,
    baseRef: process.env.WARNING_RATCHET_BASE_REF || null,
    headRef: process.env.WARNING_RATCHET_HEAD_REF || "HEAD",
    bootstrap: false,
  };
  for (let index = 0; index < values.length; index += 1) {
    const value = values[index];
    if (value === "--") continue;
    if (value === "--configuration") result.configuration = values[++index];
    else if (value === "--base-ref") result.baseRef = values[++index];
    else if (value === "--head-ref") result.headRef = values[++index];
    else if (value === "--bootstrap-baseline") result.bootstrap = true;
    else throw new Error(`unknown argument: ${value}`);
  }
  if (!result.configuration) throw new Error("--configuration is required");
  return result;
}

const args = parseArgs(process.argv.slice(2));
const inventory = loadInventory(inventoryPath);
const schema = loadInventory(schemaPath);
const inventoryErrors = validateInventory(inventory, schema);
if (inventoryErrors.length > 0) {
  throw new Error(`warning inventory validation failed:\n${inventoryErrors.map((error) => `- ${error}`).join("\n")}`);
}

const configuration = getConfiguration(inventory, args.configuration);
if (configuration.status !== "enforced") {
  throw new Error(`configuration ${configuration.id} is pending: ${configuration.blockingReason}; next: ${configuration.nextAction}`);
}
const hostPlatform = detectHostPlatform();
if (configuration.platform !== hostPlatform) {
  throw new Error(`configuration ${configuration.id} targets ${configuration.platform}, but this host is ${hostPlatform}`);
}
const currentToolchain = detectToolchain();
for (const [tool, expected] of Object.entries(configuration.evidence.toolchain)) {
  if (currentToolchain[tool] !== expected) {
    throw new Error(`toolchain drift for ${tool}: inventory=${expected}; current=${currentToolchain[tool]}`);
  }
}

const comparison = resolveTrustedComparison(repoRoot, args.baseRef, args.headRef);
const previousInventory = loadInventoryAtRef(
  repoRoot,
  comparison.base,
  "qa/warnings/warning-inventory.json",
  args.bootstrap,
);
if (!args.bootstrap && !previousInventory) throw new Error("normal warning gate requires a trusted prior inventory");
if (args.bootstrap && previousInventory) throw new Error("bootstrap mode is forbidden when a trusted prior inventory exists");
if (args.bootstrap && configuration.evidence.commit !== comparison.head) {
  throw new Error(`bootstrap evidence commit must equal trusted head ${comparison.head}`);
}
if (previousInventory) {
  const previousConfiguration = previousInventory.configurations.find((candidate) => candidate.id === configuration.id);
  const laundering = findBaselineLaundering(previousConfiguration, configuration);
  if (laundering.length > 0) {
    throw new Error(`warning baseline laundering detected:\n${laundering.map((failure) => `- ${failure}`).join("\n")}`);
  }
}

const modifiedFiles = collectModifiedFiles(repoRoot, comparison);
const suppressions = findAddedSuppressions(repoRoot, comparison);
if (suppressions.length > 0) throw new Error(`warning suppression loophole detected: ${suppressions.join(", ")}`);

console.log(`warning ratchet: ${configuration.id}`);
console.log(`trusted comparison: ${comparison.base}...${comparison.head}${args.bootstrap ? " (explicit bootstrap)" : ""}`);
console.log(`command: cargo ${configuration.command.args.join(" ")}`);
console.log(`modified files considered: ${modifiedFiles.size}`);
const result = await runCargoConfiguration(configuration, repoRoot);
if (result.timedOut) throw new Error(`Cargo warning command timed out after ${configuration.timeoutMs}ms`);
if (result.exitCode !== 0) throw new Error(`Cargo warning command exited with ${result.exitCode}`);
if (result.invalidJsonLines.length > 0) throw new Error(`Cargo emitted ${result.invalidJsonLines.length} malformed JSON stdout line(s)`);
if (result.buildFinished.length !== 1 || result.buildFinished[0] !== true) {
  throw new Error(`Cargo build-finished coverage invalid: ${JSON.stringify(result.buildFinished)}`);
}
if (result.stderrWarning) throw new Error("Cargo emitted warning-shaped stderr outside structured JSON");

const coverage = compareArtifactCoverage(configuration.expectedArtifacts, result.artifacts);
if (!coverage.ok) {
  throw new Error(`Cargo artifact coverage mismatch: missing=${JSON.stringify(coverage.missing)} unexpected=${JSON.stringify(coverage.unexpected)}`);
}
const diagnosticComparison = compareDiagnostics(configuration, result.diagnostics, modifiedFiles);
console.log(`artifact coverage: ${result.artifacts.length}/${configuration.expectedArtifacts.length}`);
console.log(`baseline warnings: ${JSON.stringify(diagnosticComparison.baselineCounts)}`);
console.log(`current warnings: ${JSON.stringify(diagnosticComparison.currentCounts)}`);
console.log(`identity removals: ${diagnosticComparison.removed.length}`);
if (!diagnosticComparison.ok) {
  throw new Error(`warning ratchet failed:\n${diagnosticComparison.failures.map((failure) => `- ${failure}`).join("\n")}`);
}
console.log("warning ratchet ok");
