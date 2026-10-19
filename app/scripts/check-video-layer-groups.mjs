import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const source = await readFile(new URL("../src/videoLayerGroups.ts", import.meta.url), "utf8");
const compiled = ts.transpileModule(source, { compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ES2022 } });
const groups = await import(`data:text/javascript;base64,${Buffer.from(compiled.outputText).toString("base64")}`);
const group = (id, layer_ids) => ({ ...groups.defaultVideoGroupMix, id, label: `Group ${id}`, composition_id: 1, layer_ids });
const composition = { id: 1, label: "Composition", layer_ids: [1, 2, 3, 4, 5, 6] };
const catalog = { effect_chains: [], effect_presets: [], transition_buses: [], layer_groups: [group(10, [2, 3, 4]), group(20, [5, 6])] };
const original = structuredClone(catalog);
const orderOf = update => update.composition_layer_orders[0].layer_ids;
const contiguous = update => {
  const order = orderOf(update);
  assert.deepEqual([...order].sort(), [...composition.layer_ids].sort(), "membership edits preserve exactly the existing layers");
  for (const entry of update.layer_groups) {
    const indices = entry.layer_ids.map(id => order.indexOf(id));
    assert.ok(indices.length > 0 && indices.every((value, index) => index === 0 || value === indices[index - 1] + 1), "every group remains ordered and contiguous");
  }
};
const ungroup = groups.assignVideoGroupLayer(catalog, composition, 3, null);
assert.deepEqual(orderOf(ungroup), [1, 2, 4, 3, 5, 6]);
assert.deepEqual(ungroup.layer_groups[0].layer_ids, [2, 4]);
contiguous(ungroup);
const transfer = groups.assignVideoGroupLayer(catalog, composition, 3, 20);
assert.deepEqual(orderOf(transfer), [1, 2, 4, 5, 6, 3]);
assert.deepEqual(transfer.layer_groups[1].layer_ids, [5, 6, 3]);
contiguous(transfer);
const moveUp = groups.moveVideoGroup(catalog, composition, 20, -1);
assert.deepEqual(orderOf(moveUp), [1, 5, 6, 2, 3, 4]);
contiguous(moveUp);
const moveDown = groups.moveVideoGroup(catalog, composition, 10, 1);
assert.deepEqual(orderOf(moveDown), [1, 5, 6, 2, 3, 4]);
assert.equal(groups.moveVideoGroup(catalog, composition, 20, 1), null);
const chains = [{ id: 1, scope: { scope: "group", group_id: 10 } }, { id: 2, scope: { scope: "transition", owner: { kind: "layer_bus", bus_id: 30 } } }, { id: 3, scope: { scope: "group", group_id: 20 } }];
const withReferences = { ...catalog, effect_chains: chains, transition_buses: [{ id: 30, members: [{kind: "group", group_id: 10}], default_from: {kind: "group", group_id: 10}, default_to: {kind: "group", group_id: 10} }] };
const removed = groups.removeVideoGroup(withReferences, 10);
assert.deepEqual(removed.layer_groups.map(entry => entry.id), [20]);
assert.deepEqual(removed.effect_chains.map(entry => entry.id), [3]);
assert.deepEqual(removed.transition_buses, []);
const patched = groups.patchVideoGroup(catalog, 10, { opacity: 0.25, solo: true, bypassed: true });
assert.equal(patched.layer_groups[0].opacity, 0.25);
assert.equal(patched.layer_groups[1], catalog.layer_groups[1]);
assert.deepEqual(catalog, original, "helpers never mutate the authoritative catalog");
console.log("PASS layer groups: membership transfer/ungroup, contiguous ordering, group block reorder, owned-reference deletion, immutable edits");
