import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { execFileSync } from "node:child_process";
import ts from "typescript";

const sourceUrl = new URL("../src/createVideoRuntimeController.ts", import.meta.url);
const moduleUrls = new Map();
async function compile(url, override) {
  if (override === undefined && moduleUrls.has(url.href)) return moduleUrls.get(url.href);
  let output = ts.transpileModule(override ?? await readFile(url, "utf8"), {
    compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
    fileName: url.pathname,
  }).outputText;
  for (const match of [...output.matchAll(/from\s+"(\.[^"]+)"/g)]) {
    const dependency = await compile(new URL(`${match[1]}.ts`, url));
    output = output.replace(match[0], `from "${dependency}"`);
  }
  const result = `data:text/javascript;base64,${Buffer.from(output).toString("base64")}`;
  if (override === undefined) moduleUrls.set(url.href, result);
  return result;
}
const baselineIndex = process.argv.indexOf("--baseline-ref");
const baselineSource = baselineIndex < 0 ? undefined : execFileSync("git", [
  "show", `${process.argv[baselineIndex + 1]}:app/src/createVideoRuntimeController.ts`,
], { encoding: "utf8" });
const { createVideoRuntimeController } = await import(await compile(sourceUrl, baselineSource));
const token = (epoch = 1) => ({ project_epoch: epoch, project_revision: 1, checkpoint_hash: "a" });
const lanes = [
  ["clip", "refreshVideoClipSlotRuntime", "resetVideoClipSlotRuntimeFence", "get_video_clip_slot_runtime", "setVideoClipRuntime", "layers"],
  ["transition", "refreshVideoTransitionRuntime", "resetVideoTransitionRuntimeFence", "get_video_layer_transition_runtime", "setVideoTransitionRuntime", "buses"],
];
function fixture() {
  let authority = token();
  let windowCurrent = true;
  let outstanding = 0;
  let maxConcurrency = 0;
  const pending = [], applied = [], messages = [];
  const controller = createVideoRuntimeController({
    invoke(command, args) {
      outstanding += 1;
      maxConcurrency = Math.max(maxConcurrency, outstanding);
      return new Promise((resolve, reject) => pending.push({ command, args,
        resolve: (value) => { outstanding -= 1; resolve(value); },
        reject: (error) => { outstanding -= 1; reject(error); },
      }));
    },
    getCurrentProjectAuthority: () => authority,
    isProjectAuthorityCurrent: (value) => windowCurrent && JSON.stringify(value) === JSON.stringify(authority),
    projectTransactionOwnerId: "poll-test-window",
    setVideoClipRuntime: (value) => applied.push(value),
    setVideoTransitionRuntime: (value) => applied.push(value),
    setMessage: (value) => messages.push(value),
  });
  return { controller, pending, applied, messages,
    authority: () => authority,
    replace: (value) => { authority = value; },
    retireWindow: () => { windowCurrent = false; },
    metrics: () => ({ invokes: pending.length, maxConcurrency }),
  };
}
const report = (authority, key, generation = 1) => ({ ...authority, runtime_generation: generation, runtime: { [key]: [] } });
for (const [name, refresh, reset, command, , key] of lanes) {
  const f = fixture();
  const burst = Array.from({ length: 20 }, () => f.controller[refresh]());
  const measured = f.metrics();
  console.log(`${baselineIndex < 0 ? "current" : "baseline"} ${name}: ${JSON.stringify(measured)}`);
  for (const request of f.pending) request.resolve(report(token(), key));
  await Promise.all(burst);
  if (baselineIndex >= 0) continue;
  assert.deepEqual(measured, { invokes: 1, maxConcurrency: 1 }, `${name}: slow scheduled burst stays single-flight`);
  assert.equal(f.pending[0].command, command);
  assert.deepEqual(f.pending[0].args, { expectedEpoch: 1, expectedRevision: 1, expectedCheckpointHash: "a", ownerId: "poll-test-window" });
  assert.equal(f.applied.length, 1);
  const failed = f.controller[refresh](true);
  f.pending[1].reject(new Error("read failed"));
  assert.equal(await failed, null);
  assert.deepEqual(f.messages, ["Error: read failed"]);
  const recovered = f.controller[refresh]();
  f.pending[2].resolve(report(token(), key, 2));
  assert.notEqual(await recovered, null, `${name}: query resumes after failure`);
  const applicationsBeforeOlderGeneration = f.applied.length;
  const older = f.controller[refresh]();
  f.pending[3].resolve(report(token(), key, 1));
  assert.equal(await older, null, `${name}: generation fence still rejects older runtime`);
  assert.equal(f.applied.length, applicationsBeforeOlderGeneration);

  // A replacement invalidates old success and error, but cannot cancel IPC.
  for (const reject of [false, true]) {
    const g = fixture();
    const old = g.controller[refresh](true);
    g.replace(token(2));
    g.controller[reset]();
    const applicationsAfterReset = g.applied.length;
    assert.equal(await g.controller[refresh](), null);
    assert.equal(g.pending.length, 1, `${name}: replacement does not overlap old native read`);
    if (reject) g.pending[0].reject(new Error("old project failed"));
    else g.pending[0].resolve(report(token(), key, 99));
    assert.equal(await old, null);
    assert.equal(g.applied.length, applicationsAfterReset);
    assert.deepEqual(g.messages, []);
    const fresh = g.controller[refresh]();
    assert.equal(g.pending[1].args.expectedEpoch, 2);
    g.pending[1].resolve(report(token(2), key));
    assert.notEqual(await fresh, null);
  }
  // Reset also fences an old response even if the authority token is identical.
  const aba = fixture();
  const stale = aba.controller[refresh](true);
  aba.controller[reset]();
  aba.pending[0].resolve(report(token(), key, 99));
  assert.equal(await stale, null);
  assert.equal(aba.applied.length, 1, `${name}: only reset applied`);
  const retired = fixture();
  const retiring = retired.controller[refresh](true);
  retired.retireWindow();
  retired.pending[0].resolve(report(token(), key));
  assert.equal(await retiring, null);
  assert.equal(retired.applied.length, 0);
  const retiredError = fixture();
  const failingRetirement = retiredError.controller[refresh](true);
  retiredError.retireWindow();
  retiredError.pending[0].reject(new Error("retired window failed"));
  assert.equal(await failingRetirement, null);
  assert.deepEqual(retiredError.messages, []);
}
if (baselineIndex < 0) {
  const f = fixture();
  const reads = lanes.map(([, refresh]) => f.controller[refresh]());
  assert.equal(f.pending.length, 2, "clip and transition have independent read lanes");
  f.pending.forEach((request, index) => request.resolve(report(token(), lanes[index][5])));
  await Promise.all(reads);
  console.log("video runtime polling deferred-response checks passed");
}

if (baselineIndex < 0) {
  for (const [name, refresh, , , , key] of lanes) {
    const f = fixture();
    const omitted = f.controller[refresh]();
    f.pending[0].resolve({ ...token(), runtime_generation: 1, runtime: {} });
    assert.notEqual(await omitted, null);
    assert.deepEqual(f.applied[0], { [key]: [] }, `${name}: native omitted empty collection becomes a usable UI array`);
    for (const invalid of [null, [], { [key]: null }, { [key]: 'invalid' }, { [key]: {} }, { [key]: undefined }]) {
      const count = f.applied.length, i = f.pending.length;
      const rejected = f.controller[refresh]();
      f.pending[i].resolve({ ...token(), runtime_generation: 100, runtime: invalid });
      assert.equal(await rejected, null, `${name}: malformed explicit collection must fail closed`);
      assert.equal(f.applied.length, count);
    }
    const i = f.pending.length, array = [{ fixtureId: 9 }], runtime = { [key]: array };
    const valid = f.controller[refresh]();
    f.pending[i].resolve({ ...token(), runtime_generation: 2, runtime });
    assert.notEqual(await valid, null, `${name}: rejected data must not advance accepted generation`);
    assert.equal(f.applied.at(-1), runtime, `${name}: valid publication is not copied`);
    assert.equal(f.applied.at(-1)[key], array);
  }
  console.log('PASS omitted native runtime arrays, explicit-malformed rejection, generation retention and zero-copy valid arrays');
}
