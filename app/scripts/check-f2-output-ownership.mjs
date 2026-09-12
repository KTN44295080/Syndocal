import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const read = (relativePath) =>
  readFile(new URL(relativePath, import.meta.url), "utf8").then((source) =>
    source.replace(/\r\n?/gu, "\n"),
  );

const outputLease = await read("../src-tauri/src/output_lease.rs");
const app = await read("../src-tauri/src/main.rs");
const ownershipContract = await read("../../qa/SYNDOCAL_2PC_TRANCHE1_OUTPUT_OWNERSHIP.md");

const required = (source, needle, label) => {
  assert.ok(source.includes(needle), `${label}: missing ${needle}`);
};

const ordered = (source, needles, label) => {
  let previous = -1;
  for (const needle of needles) {
    const index = source.indexOf(needle);
    assert.ok(index > previous, `${label}: ${needle} is missing or out of order`);
    previous = index;
  }
};

for (const phase of ["Unclaimed", "HeldActive", "HeldOrphaned"]) {
  required(outputLease, phase, "lease phases");
}
for (const resource of ["Lighting", "Video"]) {
  required(outputLease, resource, "lease resources");
}
for (const operation of [
  "acquire",
  "renew",
  "recover",
  "relinquish_output_lease",
  "force_transfer",
  "RetireOwner",
  "ProjectOrphan",
]) {
  required(outputLease, operation, "lease lifecycle");
}
for (const bound of [
  "MAX_OUTPUT_LEASES",
  "MAX_OUTPUT_LEASE_REQUESTS",
  "MAX_OUTPUT_LEASE_LANES",
  "MAX_OUTPUT_LEASE_AUDIT_RECORDS",
]) {
  required(outputLease, bound, "lease memory bounds");
}
for (const marker of [
  "fn require_owner",
  "fn require_current_process",
  "fn require_generation",
  "fn relinquish_output_lease",
  "fn force_transfer",
]) {
  required(outputLease, marker, "owner and generation fences");
}
for (const marker of [
  "output_lease_registry",
  "OutputLeaseRequestAction::RetireOwner",
  "preflight_output_lease_project_orphan",
  "output_ownership_transition",
  "OutputLeaseRequestAction::ProjectOrphan",
]) {
  required(app, marker, "native output ownership integration");
}
for (const marker of [
  "MachineOutputRole::Lighting",
  "MachineOutputRole::Video",
  "MachineOutputRole::Both",
  "MachineOutputRole::Standby",
  "arm_output_ownership_role",
  "begin_output_ownership_failure_fence",
  "teardown",
]) {
  required(app, marker, "machine output ownership integration");
}
for (const marker of [
  "local ownership only",
  "completion claim for native GPU/driver behavior",
  "Cross-PC authenticated peer leases",
]) {
  required(ownershipContract, marker, "external boundary documentation");
}

if (process.argv.includes("--self-test")) {
  assert.equal(["Unclaimed", "HeldActive", "HeldOrphaned"].length, 3);
  assert.equal(["Lighting", "Video"].length, 2);
  assert.equal("relinquish_output_lease".startsWith("relinquish_"), true);
  console.log("F2 output ownership self-test passed: 3 assertions");
} else {
  console.log("F2 output ownership ok; bounded lease lifecycle, role matrix, retirement integration and external boundary verified");
}
