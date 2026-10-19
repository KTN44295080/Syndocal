// FX audition: Preview-only chain editing. Proves that nothing reaches the live
// layer (Program) until the explicit Take, that stale auditions refuse to take,
// and that draft edits obey the same parameter bounds as the live editor.
import assert from "node:assert/strict";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";

const appRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const server = await createServer({
  root: appRoot, configFile: false, logLevel: "error", appType: "custom", optimizeDeps: { noDiscovery: true },
  resolve: { conditions: ["browser", "development"] },
  ssr: { noExternal: ["solid-js"], resolve: { conditions: ["browser", "development"] } },
  server: { middlewareMode: true },
});
try {
  const model = await server.ssrLoadModule("/src/fxAuditionModel.ts");
  const { createFxAuditionSession } = await server.ssrLoadModule("/src/createFxAuditionSession.ts");
  const solid = await server.ssrLoadModule("solid-js");

  const control = (name, kind, minimum, maximum, value) => ({ name, kind, minimum, maximum, value, default: [...value], values: [], labels: [] });
  const stage = (label, level = 0.5) => ({ enabled: true, label, source: `source ${label}`, categories: [], controls: [control("level", "Float", [0, 0, 0, 0], [1, 0, 0, 0], [level, 0, 0, 0])] });
  const chain = (...labels) => model.fxChainFromStages(labels.map((label) => stage(label)));

  // ---- Pure chain edits ----
  const base = chain("Invert", "Mirror");
  assert.deepEqual(model.fxChainStages(base).map((entry) => entry.label), ["Invert", "Mirror"]);
  assert.equal(model.fxChainFromStages([]), null);
  assert.equal(model.fxChainsEqual(base, model.cloneFxChain(base)), true);
  assert.deepEqual(model.fxChainStages(model.auditionMoveStage(base, 0, 1)).map((entry) => entry.label), ["Mirror", "Invert"]);
  assert.equal(model.auditionMoveStage(base, 0, -1), base, "moving past the start changes nothing");
  assert.deepEqual(model.fxChainStages(model.auditionRemoveStage(base, 0)).map((entry) => entry.label), ["Mirror"]);
  assert.equal(model.auditionRemoveStage(chain("Only"), 0), null, "removing the last stage empties the chain");
  assert.equal(model.fxChainStages(model.auditionSetEnabled(base, 1, false))[1].enabled, false);
  const bounded = model.fxChainStages(model.auditionSetControl(base, 0, "level", [9, 0, 0, 0]))[0].controls[0].value;
  assert.deepEqual(bounded, [1, 0, 0, 0], "draft values are bounded by the declared range");
  assert.equal(model.auditionSetControl(base, 0, "missing", [1, 0, 0, 0]), base, "unknown control names change nothing");
  assert.deepEqual(model.fxChainStages(model.auditionResetStage(model.auditionSetControl(base, 0, "level", [0.9, 0, 0, 0]), 0))[0].controls[0].value, [0.5, 0, 0, 0]);
  let full = null;
  for (let index = 0; index < 12; index++) full = model.auditionAppendStage(full, stage(`S${index}`));
  assert.equal(model.fxChainStages(full).length, model.FX_AUDITION_MAX_STAGES, "the native stage limit is respected");
  assert.equal(model.fxChainStages(base).length, 2, "edits never mutate their input");

  // ---- Session ----
  const calls = { stage: [], preview: [], commit: [], builtin: [] };
  // Reactive like the application's snapshot and Preview transport signals.
  const [layersSignal, setLayersSignal] = solid.createSignal([{ id: 2, label: "Strobe Wash", isf_effect: base }, { id: 3, label: "Logo", isf_effect: null }]);
  const [transportSignal, setTransportSignal] = solid.createSignal({ layer_id: null, effect_audition: false });
  const readLayers = () => layersSignal();
  const setLayers = (next) => setLayersSignal(next);
  const setTransport = (next) => setTransportSignal(next);
  let refuseStage = false, refusePreview = false, failCommit = false, previewDelay = 0, refuseEnd = 0;
  // The engine stores f32 and a snapshot returns its shortest decimal; emulate that round trip.
  const f32Shortest = (value) => { for (let digits = 1; digits <= 9; digits++) { const candidate = Number(value.toPrecision(digits)); if (Math.fround(candidate) === Math.fround(value)) return candidate; } return value; };
  const throughEngine = (effect) => effect && JSON.parse(JSON.stringify(effect, (_key, value) => typeof value === "number" ? f32Shortest(value) : value));
  const options = {
    layers: readLayers,
    transport: transportSignal,
    stagePreview: async (layerId, onFailure) => {
      calls.stage.push(layerId);
      if (refuseStage) { onFailure("VJ Preview supports File, Still Image and Generator layers"); return false; }
      setTransport({ layer_id: layerId, effect_audition: false });
      return true;
    },
    setPreviewEffect: async (layerId, active, effect) => {
      if (previewDelay) await new Promise((done) => setTimeout(done, previewDelay));
      calls.preview.push({ layerId, active, effect: effect ? model.fxChainStages(effect).map((entry) => entry.label) : effect });
      if (refusePreview) return null;
      if (!active && refuseEnd > 0) { refuseEnd--; return null; }
      setTransport({ layer_id: layerId, effect_audition: active });
      return transportSignal();
    },
    describeBuiltin: async (id) => { calls.builtin.push(id); return id === "invert" ? stage("Invert") : null; },
    commit: async (layerId, effect) => { calls.commit.push({ layerId, effect }); if (!failCommit) setLayers(readLayers().map((layer) => layer.id === layerId ? { ...layer, isf_effect: throughEngine(effect) } : layer)); },
  };
  await solid.createRoot(async (dispose) => {
    const session = createFxAuditionSession(options);
    const tick = () => new Promise((done) => setTimeout(done, 0));

    // Begin stages the layer and starts a Preview-only copy of the live chain.
    assert.equal(await session.begin(2), true);
    assert.deepEqual(calls.stage, [2]);
    assert.deepEqual(calls.preview.at(-1), { layerId: 2, active: true, effect: ["Invert", "Mirror"] });
    assert.equal(session.active(2).side, "B");
    assert.equal(session.active(3), null, "an audition is scoped to its layer");
    assert.equal(calls.commit.length, 0, "beginning an audition never writes the live layer");

    // Edits land in the draft and Preview, never in the layer.
    session.setControl(2, 0, "level", [0.9, 0, 0, 0]);
    session.setControl(2, 0, "level", [0.95, 0, 0, 0]);
    await tick(); await tick();
    assert.deepEqual(model.fxChainStages(session.active(2).draft)[0].controls[0].value, [0.95, 0, 0, 0]);
    assert.deepEqual(model.fxChainStages(readLayers()[0].isf_effect)[0].controls[0].value, [0.5, 0, 0, 0], "the live chain is untouched");
    assert.equal(calls.preview.at(-1).active, true);
    assert.equal(calls.commit.length, 0);

    // A/B: A returns Preview to the live chain, B returns to the draft.
    await session.show("A"); assert.deepEqual(calls.preview.at(-1), { layerId: 2, active: false, effect: null });
    await session.show("B"); assert.equal(calls.preview.at(-1).active, true);
    assert.equal(calls.commit.length, 0, "comparing never commits");

    // Editing while A is shown switches back to B so the operator sees the edit. Even with a slow
    // Preview command, switching sides never reports a false stale state or drops the next edit.
    previewDelay = 20;
    await session.show("A");
    session.setEnabled(2, 1, false);
    assert.equal(session.active(2).stale, null, "A to B through an edit is not stale while Preview is being told");
    session.setControl(2, 0, "level", [0.3, 0, 0, 0]);
    assert.equal(session.active(2).stale, null);
    assert.deepEqual(model.fxChainStages(session.active(2).draft)[0].controls[0].value, [0.3, 0, 0, 0], "an edit during publication is kept");
    await new Promise((done) => setTimeout(done, 90));
    assert.equal(session.active(2).side, "B");
    assert.equal(session.active(2).stale, null);
    assert.equal(calls.preview.at(-1).active, true);
    assert.deepEqual(calls.preview.at(-1).effect.length, 2);
    await session.show("A");
    await session.show("B");
    assert.equal(session.active(2).stale, null, "show(B) is not stale while Preview is being told");
    previewDelay = 0; await tick(); await tick();

    // Undo restores earlier drafts; the stack is bounded.
    assert.equal(session.canUndo(), true);
    await session.undo(); await session.undo();
    assert.equal(model.fxChainStages(session.active(2).draft)[1].enabled, true);
    for (let index = 0; index < 40; index++) session.setControl(2, 0, "level", [index / 100, 0, 0, 0]);
    await tick(); await tick();
    let undos = 0; while (session.canUndo()) { await session.undo(); undos++; if (undos > 100) break; }
    assert.ok(undos <= 20, `undo history is bounded (${undos})`);

    // Adding built-in and imported stages, within the limit.
    assert.equal(await session.addToAudition(2, session.describeBuiltinStage("invert")), true);
    assert.deepEqual(calls.builtin, ["invert"]);
    assert.equal(model.fxChainStages(session.active(2).draft).length, 3);
    assert.equal(await session.addToAudition(2, session.describeBuiltinStage("missing")), false);
    assert.equal(session.error(), "builtin-unavailable");

    // Take refuses once the live chain has moved, and keeps the draft readable.
    const liveBefore = readLayers()[0].isf_effect;
    setLayers(readLayers().map((layer) => layer.id === 2 ? { ...layer, isf_effect: chain("Other") } : layer));
    await tick();
    assert.equal(session.active(2).stale, "live-changed");
    assert.equal(await session.take(), false);
    assert.equal(calls.commit.length, 0, "a stale audition cannot overwrite the live chain");
    session.setControl(2, 0, "level", [0.1, 0, 0, 0]);
    assert.equal(calls.preview.filter((entry) => entry.active).length > 0, true);
    setLayers(readLayers().map((layer) => layer.id === 2 ? { ...layer, isf_effect: liveBefore } : layer));
    await tick();
    assert.equal(session.active(2).stale, null, "restoring the live chain lifts the stale state");

    // Preview moving to another layer expires the audition.
    setTransport({ layer_id: 3, effect_audition: false }); await tick();
    assert.equal(session.active(2).stale, "preview-changed");
    setTransport({ layer_id: 2, effect_audition: true }); await tick();
    assert.equal(session.active(2).stale, null);

    // Take writes exactly the draft through the live command, then ends the audition. A browser
    // double such as 0.4837209302325581 must still be recognised after the engine's f32 round trip.
    session.setControl(2, 0, "level", [0.4837209302325581, 0, 0, 0]); await tick(); await tick();
    const draft = session.active(2).draft;
    assert.equal(await session.take(), true);
    assert.equal(calls.commit.length, 1);
    assert.equal(calls.commit[0].layerId, 2);
    assert.equal(model.fxChainsEqual(calls.commit[0].effect, draft), true);
    assert.equal(session.state(), null);
    assert.equal(calls.preview.at(-1).active, false, "Take ends the Preview audition");

    // Discard ends the audition without a commit.
    assert.equal(await session.begin(2), true);
    session.setEnabled(2, 0, false);
    await tick(); await tick();
    const commitsBefore = calls.commit.length;
    await session.discard();
    assert.equal(session.state(), null);
    assert.equal(calls.commit.length, commitsBefore, "Discard never commits");
    assert.equal(calls.preview.at(-1).active, false);

    // Preview refusals are explained and leave no audition behind.
    refuseStage = true; setTransport({ layer_id: null, effect_audition: false });
    assert.equal(await session.begin(3), false);
    assert.equal(session.error(), "preview-unavailable");
    assert.match(session.reason(), /File, Still Image/);
    assert.equal(session.state(), null);
    refuseStage = false; refusePreview = true;
    assert.equal(await session.begin(3), false);
    assert.equal(session.error(), "audition-rejected");
    assert.equal(session.state(), null);
    refusePreview = false;

    // Ending Preview retries a busy transport slot; a persistent refusal keeps the audition open.
    assert.equal(await session.begin(2), true);
    refuseEnd = 2;
    await session.discard();
    assert.equal(session.state(), null, "Discard retries a busy Preview slot");
    assert.equal(await session.begin(2), true);
    refuseEnd = 99;
    await session.discard();
    assert.ok(session.state(), "an unconfirmed end keeps the audition so Discard can be retried");
    assert.equal(session.error(), "preview-busy");
    refuseEnd = 0;
    await session.discard();
    assert.equal(session.state(), null);

    // A stale audition restarts from the live chain when the operator tries again.
    assert.equal(await session.begin(2), true);
    setTransport({ layer_id: 3, effect_audition: false }); await tick();
    assert.equal(session.active(2).stale, "preview-changed");
    const liveLength = model.fxChainStages(readLayers()[0].isf_effect).length;
    assert.equal(await session.addToAudition(2, session.describeBuiltinStage("invert")), true);
    assert.equal(session.active(2).stale, null, "a fresh audition replaced the stale one");
    assert.equal(model.fxChainStages(session.active(2).draft).length, liveLength + 1, "the new effect was added to a fresh copy of the live chain");
    await session.discard();

    // A failed Take leaves the audition open.
    assert.equal(await session.begin(2), true);
    failCommit = true;
    session.setEnabled(2, 0, false);
    await tick(); await tick();
    assert.equal(await session.take(), false);
    assert.equal(session.error(), "take-failed");
    assert.ok(session.active(2), "the draft survives a failed Take");
    await session.discard();
    dispose();
  });
  console.log("PASS FX audition: chain edits, Preview-only publication, A/B, bounded undo, stale refusal, Take/Discard, refusals");
} finally {
  await server.close();
}
