import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const source = await readFile(new URL("../src/createFxLiveSession.ts", import.meta.url), "utf8");
const compiled = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 } }).outputText
  .replace('from "solid-js"', `from ${JSON.stringify(new URL("../node_modules/solid-js/dist/solid.js", import.meta.url).href)}`);
const { createFxLiveSession } = await import(`data:text/javascript;base64,${Buffer.from(compiled).toString("base64")}`);
const a = { enabled: true, label: "A", source: "shader", categories: [], controls: [], stack: [] };
const b = { ...a, label: "B" };
let authority = { project_epoch: 1, project_revision: 10, checkpoint_hash: "10" };
let layers = [{ id: 2, isf_effect: a }];
let undos = 0, sets = 0, forwarded;
const publish = (effect, count = 1) => { layers = [{ id: 2, isf_effect: effect }]; authority = { ...authority, project_revision: authority.project_revision + count, checkpoint_hash: String(authority.project_revision + count) }; };
const session = createFxLiveSession({ authority: () => authority, layers: () => layers,
  setEffect: async (_id, effect, expected) => { assert.deepEqual(expected,authority); sets++; publish(effect); }, undo: async expected => { forwarded=expected; assert.deepEqual(expected,authority); undos++; publish(a); } });
await session.run(2, async () => publish(b));
assert.equal(session.canUndo(), true);
assert.deepEqual(session.current().a, a);
assert.deepEqual(session.current().b, b);
await session.compare("A"); assert.deepEqual(layers[0].isf_effect, a);
await session.compare("B"); assert.deepEqual(layers[0].isf_effect, b);
assert.equal(sets, 2); assert.equal(session.current().side, "B");
await session.undo(); assert.equal(undos, 1); assert.equal(session.canUndo(), false); assert.equal(session.current(), null);
assert.ok(forwarded?.checkpoint_hash,"Undo forwards its exact expected authority through async preflight");
await session.run(2, async () => {}); assert.equal(session.canUndo(), false);
await session.run(2, async () => publish(b, 2)); assert.equal(session.canUndo(), false);
await session.run(2, async () => publish(a)); assert.equal(session.canUndo(), true);
authority = { ...authority, project_revision: authority.project_revision + 1, checkpoint_hash: "unrelated" };
assert.equal(session.current(), null); assert.equal(session.canUndo(), false);
await session.compare("B"); await session.undo(); assert.equal(sets, 2); assert.equal(undos, 1);
await session.run(2, async () => publish(b));
authority = { ...authority, project_epoch: 2 };
assert.equal(session.current(), null); assert.equal(session.canUndo(), false);
let release;
const pending = session.run(2, () => new Promise(resolve => { release = resolve; }));
assert.equal(session.busy(), true);
let overlapping = false;
await session.run(2, async () => { overlapping = true; });
assert.equal(overlapping, false); release(); await pending;
assert.equal(session.busy(), false);
await assert.rejects(session.run(2, async () => { throw Error("refused"); }), /refused/);
assert.equal(session.busy(), false);
console.log("PASS FX live session: exact history undo, LIVE A/B, unchanged/refused/ambiguous publications, stale epoch/revision, overlap guard");

// Exercise the actual shared preflight functions, including their async gap.
// A simplified undo stub alone cannot prove that mapping flush preserves head.
async function extractedFunction(file, name, dependencies) {
  const source = await readFile(new URL(file, import.meta.url), "utf8");
  const tree = ts.createSourceFile(file, source, ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
  let initializer;
  const visit = node => {
    if (ts.isVariableDeclaration(node) && node.name.getText(tree) === name) initializer = node.initializer.getText(tree);
    ts.forEachChild(node, visit);
  };
  visit(tree); assert.ok(initializer, name);
  const js = ts.transpileModule(`const ${name} = ${initializer};`, { compilerOptions: { target: ts.ScriptTarget.ES2022 } }).outputText;
  return new Function(...Object.keys(dependencies), `${js}; return ${name};`)(...Object.values(dependencies));
}
let head = { project_epoch: 4, project_revision: 20, checkpoint_hash: "20" };
let flushChangesHead = false, navigations = 0, messages = [];
const same = (a,b) => JSON.stringify(a) === JSON.stringify(b);
const navigate = await extractedFunction("../src/App.tsx", "navigateProjectHistory", {
  projectHistoryNavigationBusy: () => false, isfEventPulseBusy: () => false,
  confirmDiscardTimelineEditorDrafts: () => true, setProjectHistoryNavigationBusy: () => {},
  isTauriRuntime: () => true, viewportFixture: null,
  flushProjectControlMappingsBeforeMutation: async () => { if (flushChangesHead) head = {...head,project_revision:21,checkpoint_hash:"mapping-edit"}; return head.project_epoch; },
  refreshProjectHistoryStatus: async () => true, captureProjectAuthorityIdentity: () => head,
  isProjectAuthorityIdentityCurrent: expected => same(expected,head),
  setMessage: text => messages.push(text), translateUiText: text => text, uiLocale: () => "en",
});
const expected = {...head};
await navigate(async () => { navigations++; }, expected); assert.equal(navigations,1);
flushChangesHead = true;
await navigate(async () => { navigations++; }, expected); assert.equal(navigations,1,"FX Undo refuses a new mapping history head after preflight");
assert.match(messages.at(-1), /expired/);
let invokes = 0, preflightChangesHead = false;
const invokeEffect = await extractedFunction("../src/createVideoRuntimeController.ts", "invokeLegacyVideoEffectAuthoritative", {
  sameProjectAuthority: same, nextVideoEffectCatalogRequestId: () => 99,
  options: { getCurrentProjectAuthority: () => head,
    prepareMediaAssetOperationStart: async () => { if(preflightChangesHead) head={...head,project_revision:head.project_revision+1,checkpoint_hash:"other"};return {authority:head}; },
    invoke: async () => { invokes++; return {}; }, authoritativeApplicationIsCurrent: () => true },
});
const before = {...head};
await invokeEffect("set_video_layer_isf_effect",{layerId:2,effect:a},before);assert.equal(invokes,1);
preflightChangesHead = true;
await invokeEffect("set_video_layer_isf_effect",{layerId:2,effect:a},before);assert.equal(invokes,1,"LIVE A/B refuses a changed preflight head");
console.log("PASS actual FX Undo / A-B preflight: delayed mapping publication cannot undo or overwrite another edit");
