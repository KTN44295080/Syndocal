import { createMemo, createSignal, For, Show } from "solid-js";
import type { VideoEffectCatalogUpdate, VideoLayerSummary } from "../types";
import type { VideoControlPanelProps } from "./VideoControlPanel";
import { assignVideoGroupLayer, defaultVideoGroupMix, moveVideoGroup, patchVideoGroup, removeVideoGroup } from "../videoLayerGroups";
import { VideoGroupInspector } from "./VideoGroupInspector";
import "./VideoLayerGroups.css";

export function VideoLayerGroupsPanel(props: { effects: VideoControlPanelProps["effectScopes"]; layers: VideoLayerSummary[] }) {
  const [compositionId, setCompositionId] = createSignal<number | null>(null);
  const composition = createMemo(() => props.effects.compositions.find(item => item.id === compositionId()) ?? props.effects.compositions[0]);
  const groups = createMemo(() => props.effects.catalog.layer_groups.filter(group => group.composition_id === composition()?.id));
  const [name, setName] = createSignal("");
  const [first, setFirst] = createSignal<number | null>(null);
  const [last, setLast] = createSignal<number | null>(null);
  const [selectedId, setSelectedId] = createSignal<number | null>(null);
  const selected = () => groups().find(group => group.id === selectedId());
  const selectedRange = () => {
    const ids = composition()?.layer_ids ?? [];
    const start = ids.indexOf(first() ?? -1), end = ids.indexOf(last() ?? first() ?? -1);
    return start >= 0 && end >= start ? ids.slice(start, end + 1) : [];
  };
  const availableRange = () => selectedRange().length > 0 && !selectedRange().some(id => groups().some(group => group.layer_ids.includes(id)));
  const apply = (catalog: VideoEffectCatalogUpdate) => props.effects.onApplyCatalog(catalog);
  const label = (id: number) => props.layers.find(layer => layer.id === id)?.label ?? `#${id}`;
  const remove = async (id: number) => {
    const group = groups().find(item => item.id === id);
    if (!group) return;
    if (!props.effects.catalog.layer_groups.some(item => item.id === id)) return;
    await apply(removeVideoGroup(props.effects.catalog, id));
  };
  const move = (id: number, delta: -1 | 1) => {
    const current = composition(); if (!current) return;
    const update = moveVideoGroup(props.effects.catalog, current, id, delta); if (update) void apply(update);
  };
  const assign = async (layerId: number, groupId: number | null) => {
    const source = groups().find(group => group.layer_ids.includes(layerId));
    const compositionBefore = composition();
    if (!compositionBefore || source?.id === groupId) return;
    // A modal may overlap an authority refresh; never apply a stale composition.
    const current = composition();
    if (!current || current.id !== compositionBefore.id) return;
    void apply(assignVideoGroupLayer(props.effects.catalog, current, layerId, groupId));
  };
  return <section class="videoLayerGroupsPanel" data-video-layer-groups>
    <h3>Layer groups</h3>
    <Show when={composition()} fallback={<p>Add a composition to manage layer groups.</p>}>
      <label>Composition<select data-video-group-composition value={composition()?.id} disabled={props.effects.busy} onChange={event => { setCompositionId(Number(event.currentTarget.value)); setFirst(null); setLast(null); setSelectedId(null); }}><For each={props.effects.compositions}>{item => <option value={item.id} data-no-localize>{item.label}</option>}</For></select></label>
      <fieldset disabled={props.effects.busy}><legend>Create contiguous layer group</legend>
        <label>Group name<input data-video-group-create-name value={name()} onInput={event => setName(event.currentTarget.value)} /></label>
        <div class="videoGroupRange"><For each={[["First layer", first, setFirst], ["Last layer", last, setLast]] as const}>{([title, value, setter]) => <label><span>{title}</span><select aria-label={title} value={value() ?? ""} onChange={event => setter(Number(event.currentTarget.value))}><option value="">Choose layer</option><For each={composition()?.layer_ids ?? []}>{id => <option value={id} data-no-localize>{label(id)}</option>}</For></select></label>}</For></div>
        <button data-video-group-create disabled={!name().trim() || !availableRange()} onClick={() => { const current = composition(); if (current) void apply({ ...props.effects.catalog, layer_groups: [...props.effects.catalog.layer_groups, { ...defaultVideoGroupMix, id: 0, label: name().trim(), composition_id: current.id, layer_ids: selectedRange() }] }); }}>Create group</button>
      </fieldset>
      <For each={groups()}>{group => <div class="videoGroupManagementRow" data-video-group-manage={group.id}>
        <input aria-label="Group name" value={group.label} disabled={props.effects.busy} onChange={event => { const value = event.currentTarget.value.trim(); if (value) void apply(patchVideoGroup(props.effects.catalog, group.id, { label: value })); }} />
        <div class="buttonRow"><button data-video-group-edit={group.id} aria-pressed={selectedId() === group.id} onClick={() => setSelectedId(selectedId() === group.id ? null : group.id)}>Group FX and transform</button>
          <button aria-label="Move group up" disabled={props.effects.busy || composition()?.layer_ids[0] === group.layer_ids[0]} onClick={() => move(group.id, -1)}>&#8593;</button>
          <button aria-label="Move group down" disabled={props.effects.busy || composition()?.layer_ids.at(-1) === group.layer_ids.at(-1)} onClick={() => move(group.id, 1)}>&#8595;</button>
          <button class="danger" data-video-group-remove={group.id} disabled={props.effects.busy} onClick={() => void remove(group.id)}>Remove group</button></div>
      </div>}</For>
      <fieldset disabled={props.effects.busy}><legend>Layer membership</legend><For each={composition()?.layer_ids ?? []}>{id => <label><span data-no-localize>{label(id)}</span><select data-video-group-membership={id} aria-label="Layer group" value={groups().find(group => group.layer_ids.includes(id))?.id ?? ""} onChange={event => void assign(id, event.currentTarget.value ? Number(event.currentTarget.value) : null)}><option value="">Ungrouped</option><For each={groups()}>{group => <option value={group.id} data-no-localize>{group.label}</option>}</For></select></label>}</For></fieldset>
      <Show when={selected()?.id} keyed>{id => <VideoGroupInspector group={groups().find(group => group.id === id)!} effects={props.effects} />}</Show>
    </Show>
  </section>;
}
