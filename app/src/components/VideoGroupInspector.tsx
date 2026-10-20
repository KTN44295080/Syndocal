import { controlMappingTargetData, type ControlMappingTarget } from "../controlMappingLearn";
import { createSignal, For, Show } from "solid-js";
import type { VideoLayerGroupSummary, Transform2D } from "../types";
import type { VideoControlPanelProps } from "./VideoControlPanel";
import { defaultTransform } from "../videoLayerDefaults";
import { patchVideoGroup, videoGroupBlends } from "../videoLayerGroups";
import { VideoEffectScopePanel } from "./VideoEffectScopePanel";
import "./VideoLayerGroups.css";
import { VideoGroupCanvas } from "./VideoGroupCanvas";

export function VideoGroupInspector(props: { group: VideoLayerGroupSummary; effects: VideoControlPanelProps["effectScopes"] }) {
  const patch = (value: Partial<VideoLayerGroupSummary>) => props.effects.onApplyCatalog(patchVideoGroup(props.effects.catalog, props.group.id, value));
  const groupMapping = (action: ControlMappingTarget["action"], property: string) => ({
    ...controlMappingTargetData({ action, label: `Video Group ${props.group.id} ${property}`, video_group_id: props.group.id }),
    "data-control-map-group-id": props.group.id, "data-control-map-group-property": property,
  });
  const [draft, setDraft] = createSignal<Transform2D | null>(null);
  const [canvasBusy, setCanvasBusy] = createSignal(false);
  const transform = () => draft() ?? props.group.transform ?? { ...defaultTransform };
  const aspect = () => {
    const composition = props.effects.compositions.find(item => item.id === props.group.composition_id);
    const output = props.effects.outputs.find(item => composition?.output_ids.includes(item.id));
    return output && output.width > 0 && output.height > 0 ? output.width / output.height : 16 / 9;
  };
  const patchTransform = (key: keyof Transform2D, value: number) => {
    if (Number.isFinite(value)) void patch({ transform: { ...transform(), [key]: value } });
  };
  return <div class="videoGroupInspector" data-video-group-inspector={props.group.id}>
    <strong data-no-localize>{props.group.label}</strong>
    <label class="checkbox"><input type="checkbox" checked={props.group.enabled} aria-label="Group enabled" data-video-group-enabled={props.group.id} {...groupMapping("VideoGroupEnabled", "enabled")} disabled={props.effects.busy} onChange={event => void patch({ enabled: event.currentTarget.checked })} />Group enabled</label>
    <div class="videoGroupInspectorMix">
      <button type="button" aria-pressed={props.group.solo} data-video-group-solo={props.group.id} {...groupMapping("VideoGroupSolo", "solo")} disabled={props.effects.busy} onClick={() => void patch({ solo: !props.group.solo })}>Group solo</button>
      <button type="button" aria-pressed={props.group.bypassed} data-video-group-bypass={props.group.id} {...groupMapping("VideoGroupBypass", "bypass")} disabled={props.effects.busy} onClick={() => void patch({ bypassed: !props.group.bypassed })}>Group bypass</button>
    </div>
    <div class="videoGroupInspectorMix">
      <label>Group opacity<input type="range" min="0" max="1" step="0.01" value={props.group.opacity} aria-label="Group opacity" data-video-group-opacity={props.group.id} {...groupMapping("VideoGroupOpacity", "opacity")} disabled={props.effects.busy} onChange={event => void patch({ opacity: Number(event.currentTarget.value) })} /></label>
      <label>Group blend<select value={props.group.blend_mode} aria-label="Group blend" data-video-group-blend={props.group.id} {...groupMapping("VideoGroupBlendMode", "blend")} disabled={props.effects.busy} onChange={event => void patch({ blend_mode: event.currentTarget.value as VideoLayerGroupSummary["blend_mode"] })}><For each={videoGroupBlends}>{blend => <option value={blend}>{blend === "SoftLight" ? "Soft Light" : blend}</option>}</For></select></label>
    </div>
    <fieldset disabled={props.effects.busy || canvasBusy()}>
      <legend>Group transform</legend>
      <label class="checkbox"><input data-video-group-transform-enabled type="checkbox" checked={props.group.transform != null} onChange={event => void patch({ transform: event.currentTarget.checked ? { ...defaultTransform } : null })} />Transform enabled</label>
      <Show when={props.group.transform}>
        <div class="videoGroupTransformFields"><For each={[
          ["x", "Position X", -1, 1, 0.01], ["y", "Position Y", -1, 1, 0.01],
          ["scale_x", "Scale X", 0.05, 3, 0.01], ["scale_y", "Scale Y", 0.05, 3, 0.01],
          ["rotation_deg", "Rotation", -180, 180, 1],
        ] as const}>{([key, label, min, max, step]) => <label><span>{label}</span><input type="number" min={min} max={max} step={step} aria-label={label} value={Number(transform()[key].toFixed(4))} onChange={event => { const value = Number(event.currentTarget.value); if (value >= min && value <= max) patchTransform(key, value); }} /></label>}</For></div>
        <button type="button" onClick={() => void patch({ transform: { ...defaultTransform } })}>Reset transform</button>
      </Show>
    </fieldset>
    <Show when={props.group.transform}>
      <VideoGroupCanvas group={props.group} transform={transform()} aspect={aspect()} disabled={props.effects.busy}
        identity={JSON.stringify([props.group.id, props.effects.canvasAuthority, props.effects.catalog])}
        onDraft={setDraft} onBusy={setCanvasBusy} onLoadPreview={props.effects.onLoadGroupPreview}
        onCommit={value => patch({ transform: value })} />
    </Show>
    <VideoEffectScopePanel scopeLabel="Group FX" title={props.group.label} scope={{ scope: "group", group_id: props.group.id }} catalog={props.effects.catalog} busy={props.effects.busy}
      onApplyCatalog={props.effects.onApplyCatalog} onImportIsf={props.effects.onImportIsf} onApplyPreset={props.effects.onApplyPreset} onRemoveChain={props.effects.onRemoveChain} />
  </div>;
}
