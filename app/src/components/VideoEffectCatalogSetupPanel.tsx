import { For, Show, createMemo, createSignal } from "solid-js";
import type { CompositionSummary, VideoEffectCatalog } from "../types";

interface VideoEffectCatalogSetupPanelProps {
  catalog: VideoEffectCatalog;
  compositions: CompositionSummary[];
  busy?: boolean;
  onApply: (catalog: VideoEffectCatalog) => void | Promise<unknown>;
}

export function VideoEffectCatalogSetupPanel(props: VideoEffectCatalogSetupPanelProps) {
  const [compositionId, setCompositionId] = createSignal(props.compositions[0]?.id ?? 0);
  const [firstLayerId, setFirstLayerId] = createSignal(0);
  const [lastLayerId, setLastLayerId] = createSignal(0);
  const [groupLabel, setGroupLabel] = createSignal("");
  const [presetChainId, setPresetChainId] = createSignal(0);
  const [presetLabel, setPresetLabel] = createSignal("");
  const composition = createMemo(() => props.compositions.find((entry) => entry.id === compositionId()) ?? props.compositions[0] ?? null);
  const selectedLayerRange = createMemo(() => {
    const ids = composition()?.layer_ids ?? [];
    const first = ids.indexOf(firstLayerId());
    const last = ids.indexOf(lastLayerId());
    if (first < 0 || last < first) return [];
    return ids.slice(first, last + 1);
  });
  const createGroup = () => {
    const selectedComposition = composition();
    const layerIds = selectedLayerRange();
    const label = groupLabel().trim();
    if (!selectedComposition || !label || layerIds.length === 0) return;
    return props.onApply({
      ...props.catalog,
      layer_groups: [...props.catalog.layer_groups, {
        id: 0,
        label,
        composition_id: selectedComposition.id,
        layer_ids: layerIds,
      }],
    });
  };
  const savePreset = () => {
    const chain = props.catalog.effect_chains.find((entry) => entry.id === presetChainId());
    const label = presetLabel().trim();
    if (!chain || !label) return;
    return props.onApply({
      ...props.catalog,
      effect_presets: [...props.catalog.effect_presets, {
        id: 0,
        label,
        payload: {
          bypassed: chain.bypassed,
          stages: chain.stages.map((stage) => ({
            enabled: stage.enabled,
            label: stage.label,
            effect: structuredClone(stage.effect.kind),
          })),
        },
      }],
    });
  };
  const removePreset = (presetId: number, label: string) => {
    if (!globalThis.confirm(`Remove preset “${label}”?`)) return;
    return props.onApply({
      ...props.catalog,
      effect_presets: props.catalog.effect_presets.filter((entry) => entry.id !== presetId),
    });
  };
  const removeGroup = (groupId: number, label: string) => {
    if (!globalThis.confirm(`Remove group “${label}” and its Group FX chain?`)) return;
    return props.onApply({
      ...props.catalog,
      layer_groups: props.catalog.layer_groups.filter((entry) => entry.id !== groupId),
      effect_chains: props.catalog.effect_chains.filter((entry) => entry.scope.scope !== "group" || entry.scope.group_id !== groupId),
    });
  };

  return (
    <details class="videoEffectCatalogSetupPanel">
      <summary><strong>FX groups and presets</strong></summary>
      <div class="videoEffectCatalogSetupBody">
        <fieldset disabled={props.busy || props.compositions.length === 0}>
          <legend>Create contiguous layer group</legend>
          <label>Group name<input value={groupLabel()} onInput={(event) => setGroupLabel(event.currentTarget.value)} /></label>
          <label>
            Composition
            <select value={compositionId()} onChange={(event) => { setCompositionId(Number(event.currentTarget.value)); setFirstLayerId(0); setLastLayerId(0); }}>
              <For each={props.compositions}>{(entry) => <option value={entry.id}>{entry.label}</option>}</For>
            </select>
          </label>
          <label>
            First layer
            <select value={firstLayerId()} onChange={(event) => { const id = Number(event.currentTarget.value); setFirstLayerId(id); if (!lastLayerId()) setLastLayerId(id); }}>
              <option value="">Choose layer</option>
              <For each={composition()?.layer_ids ?? []}>{(id) => <option value={id}>Layer {id}</option>}</For>
            </select>
          </label>
          <label>
            Last layer
            <select value={lastLayerId()} onChange={(event) => setLastLayerId(Number(event.currentTarget.value))}>
              <option value="">Choose layer</option>
              <For each={composition()?.layer_ids ?? []}>{(id) => <option value={id}>Layer {id}</option>}</For>
            </select>
          </label>
          <button disabled={!groupLabel().trim() || selectedLayerRange().length === 0} onClick={() => void createGroup()}>Create group</button>
        </fieldset>
        <fieldset disabled={props.busy || props.catalog.effect_chains.length === 0}>
          <legend>Save chain as preset</legend>
          <label>Preset name<input value={presetLabel()} onInput={(event) => setPresetLabel(event.currentTarget.value)} /></label>
          <label>
            Source chain
            <select value={presetChainId()} onChange={(event) => setPresetChainId(Number(event.currentTarget.value))}>
              <option value="">Choose chain</option>
              <For each={props.catalog.effect_chains}>{(entry) => <option value={entry.id} data-no-localize>Chain {entry.id} · {entry.scope.scope}</option>}</For>
            </select>
          </label>
          <button disabled={!presetLabel().trim() || !presetChainId()} onClick={() => void savePreset()}>Save preset</button>
        </fieldset>
        <Show when={props.catalog.effect_presets.length > 0}>
          <ul class="videoEffectPresetList">
            <For each={props.catalog.effect_presets}>{(preset) => (
              <li><span data-no-localize>{preset.label}</span><small>{preset.payload.stages.length} FX</small><button class="danger" disabled={props.busy} onClick={() => void removePreset(preset.id, preset.label)}>Remove</button></li>
            )}</For>
          </ul>
        </Show>
        <Show when={props.catalog.layer_groups.length > 0}>
          <ul class="videoEffectPresetList">
            <For each={props.catalog.layer_groups}>{(group) => (
              <li><span data-no-localize>{group.label}</span><small data-no-localize>{group.layer_ids.length} layers</small><button class="danger" disabled={props.busy} onClick={() => void removeGroup(group.id, group.label)}>Remove group</button></li>
            )}</For>
          </ul>
        </Show>
      </div>
    </details>
  );
}
