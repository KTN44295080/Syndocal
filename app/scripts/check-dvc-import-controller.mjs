import assert from "node:assert/strict";

const { createDvcImportController } = await import("../src/dvcImportController.ts");

const authority = {
  project_epoch: 17,
  project_revision: 23,
  checkpoint_hash: "checkpoint-test",
};
const report = {
  summary: { fixtures: 46, cues: 2 },
  midi_mappings: [{ id: "midi-1" }],
  dmx_mappings: [{ id: "dmx-1" }],
};
const load = { path: null };
const imported = { report, load };

function makeController({
  discard = true,
  response = imported,
  applyResult = { verdict: "apply" },
  current = true,
  invoke = null,
} = {}) {
  let busy = false;
  const invocations = [];
  const messages = [];
  const workspaceTabs = [];
  const setupSubTabs = [];
  const reports = [];
  const applied = [];
  const capturedAuthorities = [];
  const currentChecks = [];
  const setBusy = (next) => { busy = next; };
  const defaultInvoke = async (command, args) => {
    invocations.push({ command, args });
    return response;
  };
  const controller = createDvcImportController({
    invoke: invoke ?? defaultInvoke,
    projectTransactionOwnerId: "renderer:test-owner",
    daslightProjectImportBusy: () => busy,
    captureProjectAuthorityIdentity: () => {
      capturedAuthorities.push(true);
      return authority;
    },
    isProjectAuthorityIdentityCurrent: () => current,
    confirmDiscardProjectChanges: async () => discard,
    applyLoadedProjectResult: async (nextLoad, legacyCurrentPath) => {
      applied.push({ nextLoad, legacyCurrentPath });
      return applyResult;
    },
    projectAuthorityApplicationResultIsCurrent: (result) => {
      currentChecks.push(result);
      return current;
    },
    setDaslightProjectImportBusy: setBusy,
    setMessage: (text, key) => { messages.push({ text, key }); },
    setWorkspaceTab: (tab) => { workspaceTabs.push(tab); },
    setSetupSubTab: (tab) => { setupSubTabs.push(tab); },
    setDvcImportReport: (nextReport) => { reports.push(nextReport); },
  });
  return {
    ...controller,
    get busy() { return busy; },
    invocations,
    messages,
    workspaceTabs,
    setupSubTabs,
    reports,
    applied,
    capturedAuthorities,
    currentChecks,
  };
}

async function testBusyDoubleStartSuppression() {
  let resolveInvoke;
  const invocations = [];
  const pendingInvoke = (command, args) => new Promise((resolve) => {
    invocations.push({ command, args });
    resolveInvoke = resolve;
  });
  const test = makeController({ invoke: pendingInvoke });
  const first = test.importDaslightProject();
  await Promise.resolve();
  assert.equal(test.busy, true);
  const second = test.importDaslightProject();
  await second;
  assert.equal(invocations.length, 1);
  assert.equal(test.busy, true);
  resolveInvoke(imported);
  await first;
  assert.equal(test.busy, false);
}

async function testDiscardRejectionSkipsInvoke() {
  const test = makeController({ discard: false });
  await test.importDaslightProject();
  assert.equal(test.invocations.length, 0);
  assert.equal(test.applied.length, 0);
  assert.equal(test.busy, false);
  assert.equal(test.messages.at(-1)?.text, "Daslight Project import canceled.");
}

async function testForwarding() {
  const test = makeController();
  await test.importDaslightProject();
  assert.deepEqual(test.capturedAuthorities, [true]);
  assert.equal(test.invocations.length, 1);
  assert.deepEqual(test.invocations[0], {
    command: "import_daslight_project_with_result",
    args: {
      path: null,
      ownerId: "renderer:test-owner",
      expectedEpoch: 17,
      expectedRevision: 23,
      expectedCheckpointHash: "checkpoint-test",
    },
  });
  assert.deepEqual(test.applied, [{ nextLoad: load, legacyCurrentPath: null }]);
}

async function testNullResponseIsCancel() {
  const test = makeController({ response: null });
  await test.importDaslightProject();
  assert.equal(test.applied.length, 0);
  assert.deepEqual(test.workspaceTabs, []);
  assert.deepEqual(test.setupSubTabs, []);
  assert.deepEqual(test.reports, []);
  assert.equal(test.messages.at(-1)?.text, "Daslight Project import canceled.");
  assert.equal(test.busy, false);
}

async function testStaleResultDoesNotPublish() {
  const test = makeController({ current: false });
  await test.importDaslightProject();
  assert.equal(test.currentChecks.length, 1);
  assert.deepEqual(test.workspaceTabs, []);
  assert.deepEqual(test.setupSubTabs, []);
  assert.deepEqual(test.reports, []);
  assert.equal(test.messages.at(-1)?.text, "Importing Daslight Project...");
  assert.equal(test.busy, false);
}

async function testCurrentResultPublishes() {
  const test = makeController();
  await test.importDaslightProject();
  assert.deepEqual(test.workspaceTabs, ["setup"]);
  assert.deepEqual(test.setupSubTabs, ["patch"]);
  assert.deepEqual(test.reports, [report]);
  assert.equal(
    test.messages.at(-1)?.text,
    "Imported Daslight Project (.dvc): 46 fixtures, 2 cues, 1 MIDI and 1 DMX mappings. Save As to create a Syndocal Project (.sdc).",
  );
  assert.equal(test.busy, false);
}

async function testFinallyClearsBusyForCancelAndFailure() {
  const cancel = makeController({ response: null });
  await cancel.importDaslightProject();
  assert.equal(cancel.busy, false);

  const failure = makeController({
    invoke: async () => { throw new Error("broken"); },
  });
  await failure.importDaslightProject();
  assert.equal(failure.busy, false);
  assert.equal(failure.messages.at(-1)?.text, "Daslight Project import failed: Error: broken");
}

async function testStaleFailureDoesNotPublishError() {
  const test = makeController({
    current: false,
    invoke: async () => { throw new Error("stale project"); },
  });
  await test.importDaslightProject();
  assert.equal(test.busy, false);
  assert.equal(test.messages.at(-1)?.text, "Importing Daslight Project...");
}

const tests = [
  testBusyDoubleStartSuppression,
  testDiscardRejectionSkipsInvoke,
  testForwarding,
  testNullResponseIsCancel,
  testStaleResultDoesNotPublish,
  testCurrentResultPublishes,
  testFinallyClearsBusyForCancelAndFailure,
  testStaleFailureDoesNotPublishError,
];

for (const test of tests) {
  await test();
}

console.log(`DVC import controller focused proof: ${tests.length} cases passed`);
