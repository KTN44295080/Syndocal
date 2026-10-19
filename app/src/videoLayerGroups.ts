import type { CompositionSummary, VideoEffectCatalog, VideoEffectCatalogUpdate, VideoLayerGroupSummary } from "./types";

export const defaultVideoGroupMix = { enabled: true, opacity: 1, blend_mode: "Normal" as const, solo: false, bypassed: false, transform: null };
export const videoGroupBlends = ["Normal", "Add", "Multiply", "Screen", "Overlay", "Subtract", "Difference", "Lighten", "Darken", "SoftLight"] as const;
export const patchVideoGroup = (catalog: VideoEffectCatalog, id: number, patch: Partial<VideoLayerGroupSummary>): VideoEffectCatalog => ({
  ...catalog, layer_groups: catalog.layer_groups.map(group => group.id === id ? { ...group, ...patch } : group),
});

/** Deleting a group releases its children and removes all owned FX/bus references. */
export const removeVideoGroup = (catalog: VideoEffectCatalog, id: number): VideoEffectCatalog => {
  const buses = catalog.transition_buses.filter(bus => ![bus.default_from, bus.default_to, bus.matte_source].some(target => target?.kind === "group" && target.group_id === id))
    .map(bus => ({ ...bus, members: bus.members.filter(target => target.kind !== "group" || target.group_id !== id) }));
  const removedBusIds = new Set(catalog.transition_buses.filter(bus => !buses.some(next => next.id === bus.id)).map(bus => bus.id));
  return { ...catalog, layer_groups: catalog.layer_groups.filter(group => group.id !== id), transition_buses: buses,
    effect_chains: catalog.effect_chains.filter(chain => !(chain.scope.scope === "group" && chain.scope.group_id === id)
      && !(chain.scope.scope === "transition" && chain.scope.owner.kind === "layer_bus" && removedBusIds.has(chain.scope.owner.bus_id))),
  };
};

/** Assigning a child moves it beside its new siblings in the same atomic edit. */
export function assignVideoGroupLayer(catalog: VideoEffectCatalog, composition: CompositionSummary, layerId: number, groupId: number | null): VideoEffectCatalogUpdate {
  let next = { ...catalog, layer_groups: catalog.layer_groups.map(group => group.composition_id !== composition.id ? group : {
    ...group, layer_ids: group.layer_ids.filter(id => id !== layerId),
  }) };
  const destination = next.layer_groups.find(group => group.id === groupId && group.composition_id === composition.id);
  let order = composition.layer_ids.filter(id => id !== layerId);
  const previous = catalog.layer_groups.find(group => group.composition_id === composition.id && group.layer_ids.includes(layerId));
  const siblings = destination?.layer_ids ?? previous?.layer_ids.filter(id => id !== layerId) ?? [];
  const anchor = siblings.length ? order.indexOf(siblings[siblings.length - 1]) + 1 : composition.layer_ids.indexOf(layerId);
  order.splice(Math.max(0, anchor), 0, layerId);
  if (destination) destination.layer_ids = order.filter(id => id === layerId || destination.layer_ids.includes(id));
  for (const empty of next.layer_groups.filter(group => group.layer_ids.length === 0)) next = removeVideoGroup(next, empty.id);
  return { ...next, composition_layer_orders: [{ composition_id: composition.id, layer_ids: order }] };
}

/** Move a group past one neighboring group or ungrouped layer as one block. */
export function moveVideoGroup(catalog: VideoEffectCatalog, composition: CompositionSummary, groupId: number, delta: -1 | 1): VideoEffectCatalogUpdate | null {
  const blocks: number[][] = [];
  for (const id of composition.layer_ids) {
    const group = catalog.layer_groups.find(entry => entry.composition_id === composition.id && entry.layer_ids.includes(id));
    if (!group) blocks.push([id]);
    else if (group.layer_ids[0] === id) blocks.push([...group.layer_ids]);
  }
  const group = catalog.layer_groups.find(entry => entry.id === groupId);
  const index = blocks.findIndex(block => block[0] === group?.layer_ids[0]);
  const target = index + delta;
  if (index < 0 || target < 0 || target >= blocks.length) return null;
  [blocks[index], blocks[target]] = [blocks[target], blocks[index]];
  return { ...catalog, composition_layer_orders: [{ composition_id: composition.id, layer_ids: blocks.flat() }] };
}
