import { For, Show, createMemo, createSignal } from "solid-js";
import type {
  CompositionSummary,
  VideoClipTakeDurationUnit,
  VideoClipTakeKind,
  VideoEffectCatalog,
  VideoLayerTransitionBusRuntimeSummary,
  VideoLayerTransitionCurve,
  VideoLayerTransitionRuntimeSnapshot,
  VideoLayerTransitionTarget,
} from "../types";

interface VideoTransitionBusPanelProps {
  catalog: VideoEffectCatalog;
  compositions: CompositionSummary[];
  runtime: VideoLayerTransitionRuntimeSnapshot;
  busy?: boolean;
  compact?: boolean;
  onApplyCatalog: (catalog: VideoEffectCatalog) => void | Promise<unknown>;
  onLaunch: (request: {
    bus_id: number;
    from: VideoLayerTransitionTarget;
    to: VideoLayerTransitionTarget;
    kind: VideoClipTakeKind;
    duration: { unit: VideoClipTakeDurationUnit; value_milliunits: number };
    curve: VideoLayerTransitionCurve;
  }) => void | Promise<unknown>;
  onRelease: (busId: number) => void | Promise<unknown>;
}

const targetKey = (target: VideoLayerTransitionTarget) =>
  target.kind === "layer" ? `layer:${target.layer_id}` : `group:${target.group_id}`;

const targetLabel = (target: VideoLayerTransitionTarget, catalog: VideoEffectCatalog) => {
  if (target.kind === "layer") return `Layer ${target.layer_id}`;
  return catalog.layer_groups.find((group) => group.id === target.group_id)?.label ?? `Group ${target.group_id}`;
};

export function VideoTransitionBusPanel(props: VideoTransitionBusPanelProps) {
  const [label, setLabel] = createSignal("");
  const [compositionId, setCompositionId] = createSignal(props.compositions[0]?.id ?? 0);
  const [fromKey, setFromKey] = createSignal("");
  const [toKey, setToKey] = createSignal("");
  const [kind, setKind] = createSignal<VideoClipTakeKind>("Crossfade");
  const [durationUnit, setDurationUnit] = createSignal<VideoClipTakeDurationUnit>("Milliseconds");
  const [durationValue, setDurationValue] = createSignal(1_000);
  const [curve, setCurve] = createSignal<VideoLayerTransitionCurve>("ease_in_out");
  const composition = createMemo(() => props.compositions.find((entry) => entry.id === compositionId()) ?? props.compositions[0] ?? null);
  const targets = createMemo(() => {
    const selected = composition();
    if (!selected) return [] as VideoLayerTransitionTarget[];
    const groupByLayer = new Map<number, number>();
    for (const group of props.catalog.layer_groups) {
      if (group.composition_id !== selected.id) continue;
      for (const layerId of group.layer_ids) groupByLayer.set(layerId, group.id);
    }
    const seenGroups = new Set<number>();
    const result: VideoLayerTransitionTarget[] = [];
    for (const layerId of selected.layer_ids) {
      const groupId = groupByLayer.get(layerId);
      if (groupId !== undefined) {
        if (!seenGroups.has(groupId)) {
          seenGroups.add(groupId);
          result.push({ kind: "group", group_id: groupId });
        }
      } else {
        result.push({ kind: "layer", layer_id: layerId });
      }
    }
    return result;
  });
  const selectedMemberRange = createMemo(() => {
    const entries = targets();
    const first = entries.findIndex((target) => targetKey(target) === fromKey());
    const last = entries.findIndex((target) => targetKey(target) === toKey());
    if (first < 0 || last <= first) return [] as VideoLayerTransitionTarget[];
    return entries.slice(first, last + 1);
  });
  const createBus = () => {
    const selected = composition();
    const members = selectedMemberRange();
    const busLabel = label().trim();
    if (!selected || !busLabel || members.length < 2) return;
    return props.onApplyCatalog({
      ...props.catalog,
      transition_buses: [...props.catalog.transition_buses, {
        id: 0,
        label: busLabel,
        composition_id: selected.id,
        enabled: true,
        members,
        default_from: members[0],
        default_to: members[members.length - 1],
        default_kind: kind(),
        default_duration: {
          unit: durationUnit(),
          value_milliunits: Math.max(1, Math.round(
            durationUnit() === "Milliseconds" ? durationValue() : durationValue() * 1_000,
          )),
        },
        default_curve: curve(),
        matte_source: null,
      }],
    });
  };
  const activeFor = (busId: number): VideoLayerTransitionBusRuntimeSummary | null =>
    props.runtime.buses.find((entry) => entry.bus_id === busId) ?? null;
  const launch = (busId: number) => {
    const bus = props.catalog.transition_buses.find((entry) => entry.id === busId);
    if (!bus) return;
    const active = activeFor(busId);
    return props.onLaunch(active ? {
      bus_id: bus.id,
      from: active.to,
      to: active.from,
      kind: active.kind,
      duration: active.duration,
      curve: active.curve,
    } : {
      bus_id: bus.id,
      from: bus.default_from,
      to: bus.default_to,
      kind: bus.default_kind,
      duration: bus.default_duration,
      curve: bus.default_curve,
    });
  };
  const removeBus = (busId: number, busLabel: string) => {
    if (activeFor(busId)) return;
    if (!globalThis.confirm(`Remove transition bus “${busLabel}” and its Bus FX chain?`)) return;
    return props.onApplyCatalog({
      ...props.catalog,
      transition_buses: props.catalog.transition_buses.filter((entry) => entry.id !== busId),
      effect_chains: props.catalog.effect_chains.filter((entry) => !(
        entry.scope.scope === "transition"
        && entry.scope.owner.kind === "layer_bus"
        && entry.scope.owner.bus_id === busId
      )),
    });
  };

  return (
    <section class={props.compact ? "videoTransitionBusPanel compact" : "videoTransitionBusPanel"} aria-label="Layer transition buses">
      <header><div><strong>Layer Transition Buses</strong><span>Opt-in layers only · reversible</span></div></header>
      <Show when={!props.compact}>
        <fieldset class="videoTransitionBusCreate" disabled={props.busy || props.compositions.length === 0}>
          <legend>Create bus</legend>
          <label>Name<input value={label()} onInput={(event) => setLabel(event.currentTarget.value)} /></label>
          <label>Composition<select value={compositionId()} onChange={(event) => { setCompositionId(Number(event.currentTarget.value)); setFromKey(""); setToKey(""); }}><For each={props.compositions}>{(entry) => <option value={entry.id}>{entry.label}</option>}</For></select></label>
          <label>From<select value={fromKey()} onChange={(event) => setFromKey(event.currentTarget.value)}><option value="">Choose</option><For each={targets()}>{(target) => <option value={targetKey(target)}>{targetLabel(target, props.catalog)}</option>}</For></select></label>
          <label>To<select value={toKey()} onChange={(event) => setToKey(event.currentTarget.value)}><option value="">Choose</option><For each={targets()}>{(target) => <option value={targetKey(target)}>{targetLabel(target, props.catalog)}</option>}</For></select></label>
          <label>Mode<select value={kind()} onChange={(event) => setKind(event.currentTarget.value as VideoClipTakeKind)}><For each={["Cut", "Crossfade", "Dip", "Wipe", "Luma", "Displacement", "Blur", "Glitch", "Custom"] as VideoClipTakeKind[]}>{(entry) => <option value={entry}>{entry}</option>}</For></select></label>
          <label>Duration<input type="number" min={durationUnit() === "Milliseconds" ? "1" : "0.001"} max={durationUnit() === "Milliseconds" ? "600000" : "1000"} step={durationUnit() === "Milliseconds" ? "1" : "0.001"} value={durationValue()} onInput={(event) => setDurationValue(Number(event.currentTarget.value))} /></label>
          <label>Unit<select value={durationUnit()} onChange={(event) => { const next = event.currentTarget.value as VideoClipTakeDurationUnit; setDurationUnit(next); setDurationValue(next === "Milliseconds" ? 1_000 : 1); }}><option value="Milliseconds">ms</option><option value="Beats">Beats</option><option value="Bars">Bars</option></select></label>
          <label>Curve<select value={curve()} onChange={(event) => setCurve(event.currentTarget.value as VideoLayerTransitionCurve)}><option value="linear">Linear</option><option value="ease_in">Ease In</option><option value="ease_out">Ease Out</option><option value="ease_in_out">Ease In/Out</option></select></label>
          <button class="primary" disabled={!label().trim() || selectedMemberRange().length < 2} onClick={() => void createBus()}>Create Bus</button>
        </fieldset>
      </Show>
      <div class="videoTransitionBusList">
        <For each={props.catalog.transition_buses} fallback={<p class="empty">No Layer Transition Buses.</p>}>
          {(bus) => {
            const active = () => activeFor(bus.id);
            return (
              <article class={active() ? "videoTransitionBus active" : "videoTransitionBus"}>
                <div><strong data-no-localize>{bus.label}</strong><small>{bus.members.length} targets · {active() ? `${active()!.progress_millis / 10}%` : bus.default_kind}</small></div>
                <progress max="1000" value={active()?.progress_millis ?? 0} aria-label={`${bus.label} transition progress`} />
                <div class="videoTransitionBusActions">
                  <button class="primary" disabled={props.busy || !bus.enabled} onClick={() => void launch(bus.id)}>{active() ? "Reverse" : "Take"}</button>
                  <button disabled={props.busy || !active()} onClick={() => void props.onRelease(bus.id)}>Release</button>
                  <Show when={!props.compact}><button class="danger" disabled={props.busy || Boolean(active())} onClick={() => void removeBus(bus.id, bus.label)}>Remove</button></Show>
                </div>
              </article>
            );
          }}
        </For>
      </div>
    </section>
  );
}
