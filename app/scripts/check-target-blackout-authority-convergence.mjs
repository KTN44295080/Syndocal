import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const source = await readFile(
  new URL("../src/projectAuthorityReceiptConvergence.ts", import.meta.url),
  "utf8",
);
const appSource = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
const compiled = new Map();

const compile = async (url) => {
  if (compiled.has(url.href)) return compiled.get(url.href);
  let code = ts.transpileModule(await readFile(url, "utf8"), {
    compilerOptions: {
      module: ts.ModuleKind.ESNext,
      target: ts.ScriptTarget.ES2022,
      importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
    },
    fileName: url.pathname,
  }).outputText;
  const imports = [...code.matchAll(/from\s+"(\.[^"]+)"/g)];
  for (const match of imports) {
    const specifier = match[1];
    const child = new URL(specifier.endsWith(".ts") ? specifier : `${specifier}.ts`, url);
    code = code.replace(`from "${specifier}"`, `from "${await compile(child)}"`);
  }
  const dataUrl = `data:text/javascript;base64,${Buffer.from(code).toString("base64")}`;
  compiled.set(url.href, dataUrl);
  return dataUrl;
};

const { createProjectAuthorityReceiptConvergence } = await import(
  await compile(new URL("../src/projectAuthorityReceiptConvergence.ts", import.meta.url)),
);

const deferred = () => {
  let resolve;
  const promise = new Promise((resolvePromise) => { resolve = resolvePromise; });
  return { promise, resolve };
};
const receipt = {
  fence_after: {
    project_epoch: 7,
    project_revision: 8,
    project_checkpoint_hash: "b".repeat(64),
  },
};
const expected = {
  project_epoch: 7,
  project_revision: 8,
  checkpoint_hash: "b".repeat(64),
};
const oldAuthority = {
  project_epoch: 7,
  project_revision: 7,
  checkpoint_hash: "a".repeat(64),
};

let currentAuthority = expected;
let pollCalls = 0;
const alreadyConverged = createProjectAuthorityReceiptConvergence({
  currentAuthority: () => currentAuthority,
  inFlightAuthorityPoll: () => null,
  pollProjectAuthorityBundle: async () => { pollCalls += 1; },
});
await alreadyConverged(receipt);
assert.equal(pollCalls, 0, "an already converged receipt must not start another authority poll");

currentAuthority = oldAuthority;
const preReceiptPoll = deferred();
const canonicalPoll = deferred();
let freshPollCalls = 0;
const convergePendingReceipt = createProjectAuthorityReceiptConvergence({
  currentAuthority: () => currentAuthority,
  inFlightAuthorityPoll: () => preReceiptPoll.promise,
  pollProjectAuthorityBundle: async () => {
    freshPollCalls += 1;
    await canonicalPoll.promise;
    currentAuthority = expected;
  },
});
const pendingConvergence = convergePendingReceipt(receipt);
assert.equal(freshPollCalls, 0, "a pre-receipt poll must settle before the fresh canonical poll starts");
preReceiptPoll.resolve();
await Promise.resolve();
assert.equal(freshPollCalls, 1, "the fresh canonical poll must start after the pre-receipt poll settles");
canonicalPoll.resolve();
await pendingConvergence;
assert.deepEqual(currentAuthority, expected, "canonical polling must converge the receipt fence");

currentAuthority = oldAuthority;
let advancedPollCalls = 0;
const advancedAuthority = {
  project_epoch: 7,
  project_revision: 9,
  checkpoint_hash: "c".repeat(64),
};
await createProjectAuthorityReceiptConvergence({
  currentAuthority: () => currentAuthority,
  inFlightAuthorityPoll: () => null,
  pollProjectAuthorityBundle: async () => {
    advancedPollCalls += 1;
    currentAuthority = advancedAuthority;
  },
})(receipt);
assert.equal(advancedPollCalls, 1, "a later authority revision is accepted after the receipt");

currentAuthority = oldAuthority;
await assert.rejects(
  createProjectAuthorityReceiptConvergence({
    currentAuthority: () => currentAuthority,
    inFlightAuthorityPoll: () => null,
    pollProjectAuthorityBundle: async () => {},
  })(receipt),
  /Target blackout applied, but canonical project authority did not converge; snapshot was not refreshed\./,
  "an unchanged canonical authority must fail closed",
);

await assert.rejects(
  createProjectAuthorityReceiptConvergence({
    currentAuthority: () => currentAuthority,
    inFlightAuthorityPoll: () => null,
    pollProjectAuthorityBundle: async () => {},
  })(receipt, "Show Spout outputs"),
  /Show Spout outputs applied, but canonical project authority did not converge; snapshot was not refreshed\./,
  "a Spout convergence failure must identify the operation that committed",
);

assert.match(
  appSource,
  /const refreshProjectAuthorityAfterTargetBlackout = createProjectAuthorityReceiptConvergence\(\{[\s\S]*?currentAuthority: projectMappingsAuthority,[\s\S]*?inFlightAuthorityPoll: \(\) => projectAuthorityPollInFlight,[\s\S]*?pollProjectAuthorityBundle,[\s\S]*?\}\);/,
  "App must inject the live authority poll, in-flight poll and current authority into the extracted convergence seam",
);
assert.match(source, /if \(inFlight\) await inFlight;/);
assert.match(source, /await options\.pollProjectAuthorityBundle\(\);/);
assert.match(
  source,
  /operationLabel = options\.operationLabel \?\? "Target blackout"[\s\S]*\$\{operationLabel\} applied, but canonical project authority did not converge/,
  "convergence errors must use the injected operation label",
);

console.log("target blackout authority convergence: PASS (pre-receipt race, exact convergence, later revision acceptance, and fail-closed mismatch)");
