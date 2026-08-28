import assert from "node:assert/strict";
import {
  establishProjectOpenBootstrapAuthority,
  isProjectOpenAuthorityMismatch,
  ProjectOpenBootstrapRetryLatch,
} from "../src/projectOpenBootstrap.ts";

const trace = [];
let ready = false;
await establishProjectOpenBootstrapAuthority({
  awaitOwnerRegistration: async () => { trace.push("owner"); },
  refreshAuthority: async () => {
    trace.push("authority");
    ready = true;
    return true;
  },
  authorityReady: () => {
    trace.push("ready");
    return ready;
  },
});
assert.deepEqual(trace, ["owner", "authority", "ready"]);

for (const [label, refreshAuthority, authorityReady] of [
  ["refresh rejected", async () => false, () => true],
  ["authority remained unready", async () => true, () => false],
]) {
  await assert.rejects(
    establishProjectOpenBootstrapAuthority({
      awaitOwnerRegistration: async () => undefined,
      refreshAuthority,
      authorityReady,
    }),
    /startup project loading was not attempted/,
    label,
  );
}

let refreshReached = false;
await assert.rejects(
  establishProjectOpenBootstrapAuthority({
    awaitOwnerRegistration: async () => { throw new Error("owner rejected"); },
    refreshAuthority: async () => {
      refreshReached = true;
      return true;
    },
    authorityReady: () => true,
  }),
  /owner rejected/,
);
assert.equal(refreshReached, false, "authority refresh must not run before owner registration");

let transientAttempts = 0;
let transientReady = false;
const transientGate = {
  awaitOwnerRegistration: async () => undefined,
  refreshAuthority: async () => {
    transientAttempts += 1;
    transientReady = transientAttempts === 2;
    return transientReady;
  },
  authorityReady: () => transientReady,
};
await assert.rejects(establishProjectOpenBootstrapAuthority(transientGate));
await establishProjectOpenBootstrapAuthority(transientGate);
assert.equal(transientAttempts, 2, "a later authority-ready retry may complete the same bootstrap");
assert.equal(
  isProjectOpenAuthorityMismatch("Project authority changed before mutation (expected epoch 0 revision 0)"),
  true,
);
assert.equal(isProjectOpenAuthorityMismatch("Project file is corrupt"), false);

const retryLatch = new ProjectOpenBootstrapRetryLatch();
retryLatch.observe({ project_epoch: 0, project_revision: 0, checkpoint_hash: "A" }, false);
assert.equal(retryLatch.take(), false, "an authority observed before bootstrap is not a retry");
retryLatch.observe({ project_epoch: 1, project_revision: 0, checkpoint_hash: "B" }, true);
retryLatch.observe({ project_epoch: 2, project_revision: 0, checkpoint_hash: "C" }, true);
assert.equal(retryLatch.take(), true, "B/C changes during one in-flight attempt coalesce to one retry");
assert.equal(retryLatch.take(), false, "the retry latch is consumed exactly once");
retryLatch.observe({ project_epoch: 2, project_revision: 0, checkpoint_hash: "C" }, true);
assert.equal(retryLatch.take(), false, "the same authority never rearms a retry loop");

console.log("project open bootstrap authority gate passed");
