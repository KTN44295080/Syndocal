import assert from "node:assert/strict";

const { createMediaAssetOperationController } = await import(
  "../src/mediaAssetOperationController.ts",
);

let operations = [];
const cleanups = [];
const controller = createMediaAssetOperationController({
  publish: (update) => {
    operations = typeof update === "function" ? update(operations) : update;
  },
  registerCleanup: (cleanup) => cleanups.push(cleanup),
});

assert.equal(cleanups.length, 1, "controller registers exactly one owner cleanup");
const first = controller.begin("Import media", "picker");
assert.equal(operations.length, 1, "begin publishes one active operation");
assert.equal(operations[0].label, "Import media");
assert.equal(operations[0].phase, "picker");

first.setPhase("hashing");
assert.equal(operations[0].phase, "hashing", "active operation phase is published");
const firstId = operations[0].id;
controller.cancel(firstId);
assert.equal(first.signal.aborted, true, "cancel aborts the exact operation signal");
assert.equal(operations[0].phase, "cancelling", "cancel publishes the cancelling phase");
first.setPhase("committing");
assert.equal(operations[0].phase, "cancelling", "aborted operation cannot publish a later phase");

first.release();
first.release();
assert.equal(operations.length, 0, "release is idempotent and removes the operation");

const second = controller.begin("Verify media", "preparing");
const third = controller.begin("Relink media", "finalizing");
assert.equal(operations.length, 2, "multiple operation lanes remain independently visible");
cleanups[0]();
assert.equal(second.signal.aborted, true, "owner cleanup aborts the first remaining operation");
assert.equal(third.signal.aborted, true, "owner cleanup aborts the second remaining operation");
assert.deepEqual(
  operations.map(({ phase }) => phase),
  ["cancelling", "cancelling"],
  "owner cleanup publishes cancellation for every remaining operation",
);

console.log("media asset operation controller: PASS (phase, exact cancellation, idempotent release, owner cleanup)");
