import { For, Show, createEffect, createMemo, createSignal, on, onMount } from "solid-js";
import type {
  MediaAssetSummary,
  VideoClipCuePointSummary,
  VideoClipEffectOverrideSummary,
  VideoClipSlotId,
  VideoClipSlotSummary,
  VideoEffectCatalog,
  VideoLayerSummary,
} from "../types";
import { VideoEffectScopePanel } from "./VideoEffectScopePanel";

export interface VideoClipSlotInspectorPanelProps {
  /** Rendered inside a workspace pane (the Perform Inspector's Clip tab): an
   * ordinary in-flow region with no modal role, focus trap or Escape handling,
   * so the performance shortcuts keep working. */
  embedded?: boolean;
  layer: VideoLayerSummary | null;
  slotId: VideoClipSlotId | null;
  assets: MediaAssetSummary[];
  returnFocus: HTMLElement | null;
  removeDisabledReason: string | null;
  effectCatalog: VideoEffectCatalog;
  onClose: () => void;
  onCreate: (assetId: number, beforeSlotId: VideoClipSlotId | null) => void | Promise<unknown>;
  onAssign: (slotId: VideoClipSlotId, assetId: number) => void | Promise<unknown>;
  onUpdate: (slot: VideoClipSlotSummary) => void | Promise<unknown>;
  onRemove: (slotId: VideoClipSlotId) => void | Promise<unknown>;
  onDuplicate: (slotId: VideoClipSlotId) => void | Promise<unknown>;
  onSetDefault: (slotId: VideoClipSlotId) => void | Promise<unknown>;
  onReorder: (slotId: VideoClipSlotId, direction: -1 | 1) => void | Promise<unknown>;
  onApplyEffectCatalog: (catalog: VideoEffectCatalog) => void | Promise<unknown>;
  onImportScopeIsf: Parameters<typeof VideoEffectScopePanel>[0]["onImportIsf"];
  onApplyEffectPreset: Parameters<typeof VideoEffectScopePanel>[0]["onApplyPreset"];
  onRemoveEffectChain: Parameters<typeof VideoEffectScopePanel>[0]["onRemoveChain"];
}

const focusableSelector = "button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), [tabindex]:not([tabindex='-1'])";

export function VideoClipSlotInspectorPanel(props: VideoClipSlotInspectorPanelProps) {
  const slot = createMemo(() => props.layer?.clip_slots?.find((candidate) => candidate.id === props.slotId) ?? null);
  // The operator's unsaved asset choice. `null` means "untouched": the select then mirrors the
  // authoritative slot asset, so it can never fall out of sync with the slot being inspected.
  const [assetDraft, setAssetDraft] = createSignal<number | null>(null);
  const currentAssetId = createMemo(() => slot()?.media_asset_id ?? null);
  const selectedAssetId = createMemo(() => assetDraft() ?? currentAssetId() ?? props.assets[0]?.id ?? null);
  const assetMissing = createMemo(() => {
    const id = currentAssetId();
    return id !== null && !props.assets.some((asset) => asset.id === id);
  });
  const assignDisabled = createMemo(() => assetDraft() === null || assetDraft() === currentAssetId() || !props.assets.some((asset) => asset.id === assetDraft()));
  const [inPointMs, setInPointMs] = createSignal(0);
  const [outPointMs, setOutPointMs] = createSignal("");
  const [loopMode, setLoopMode] = createSignal<VideoClipSlotSummary["loop_mode"]>("Once");
  const [speed, setSpeed] = createSignal(1);
  const [quantization, setQuantization] = createSignal<VideoClipSlotSummary["launch_quantization"]>("Immediate");
  const [cuePoints, setCuePoints] = createSignal("[]");
  const [effectOverrides, setEffectOverrides] = createSignal("[]");
  const [localError, setLocalError] = createSignal<string | null>(null);
  let panel!: HTMLElement;

  // Drafts are discarded when a different slot is inspected or the authoritative asset changes
  // (an Assign landed, or another surface reassigned it). Snapshot refreshes that merely hand back
  // new objects for the same slot do not retrigger this, so an in-progress choice is preserved.
  createEffect(on([() => props.slotId, currentAssetId], () => setAssetDraft(null)));
  // Other fields re-sync only when the authoritative slot content really changes.
  const slotSignature = createMemo(() => JSON.stringify(slot()));
  createEffect(on([() => props.slotId, slotSignature], () => {
    const current = slot();
    setInPointMs(current?.in_point_ms ?? 0);
    setOutPointMs(current?.out_point_ms == null ? "" : String(current.out_point_ms));
    setLoopMode(current?.loop_mode ?? "Once");
    setSpeed(current?.speed ?? 1);
    setQuantization(current?.launch_quantization ?? "Immediate");
    setCuePoints(JSON.stringify(current?.cue_points ?? [], null, 2));
    setEffectOverrides(JSON.stringify(current?.effect_overrides ?? [], null, 2));
    setLocalError(null);
  }));

  const close = () => {
    props.onClose();
    window.requestAnimationFrame(() => props.returnFocus?.focus());
  };
  const keyDown = (event: KeyboardEvent) => {
    if (props.embedded) return;
    if (event.key === "Escape") {
      event.preventDefault();
      close();
      return;
    }
    if (event.key !== "Tab") return;
    const focusable = Array.from(panel.querySelectorAll<HTMLElement>(focusableSelector));
    if (focusable.length === 0) return;
    const first = focusable[0];
    const last = focusable[focusable.length - 1];
    if (event.shiftKey && document.activeElement === first) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && document.activeElement === last) {
      event.preventDefault();
      first.focus();
    }
  };
  onMount(() => {
    if (!props.embedded) window.requestAnimationFrame(() => panel.querySelector<HTMLElement>(focusableSelector)?.focus());
  });

  const save = () => {
    const current = slot();
    if (!current) return;
    try {
      const parsedCues = JSON.parse(cuePoints()) as VideoClipCuePointSummary[];
      const parsedOverrides = JSON.parse(effectOverrides()) as VideoClipEffectOverrideSummary[];
      if (!Array.isArray(parsedCues) || !Array.isArray(parsedOverrides)) throw new Error("Cue points and effect overrides must be JSON arrays.");
      const parsedOut = outPointMs().trim() === "" ? null : Math.max(0, Math.round(Number(outPointMs())));
      if (parsedOut !== null && !Number.isFinite(parsedOut)) throw new Error("Out point must be empty or a finite millisecond value.");
      setLocalError(null);
      void props.onUpdate({
        ...current,
        in_point_ms: Math.max(0, Math.round(inPointMs())),
        out_point_ms: parsedOut,
        loop_mode: loopMode(),
        speed: Math.max(-4, Math.min(4, speed())),
        launch_quantization: quantization(),
        cue_points: parsedCues,
        effect_overrides: parsedOverrides,
      });
    } catch (error) {
      setLocalError(String(error));
    }
  };
  const remove = (slotId: VideoClipSlotId) => {
    if (props.removeDisabledReason) return;
    if (!globalThis.confirm(`Remove Clip Slot ${slotId}? This authored change can be undone from project history.`)) return;
    void props.onRemove(slotId);
  };

  return (
    <aside
      ref={panel}
      class="videoClipSlotInspectorPanel"
      classList={{ videoClipSlotInspectorEmbedded: props.embedded }}
      role={props.embedded ? undefined : "dialog"}
      aria-modal={props.embedded ? undefined : "true"}
      aria-labelledby="video-clip-slot-inspector-title"
      data-video-clip-slot-inspector
      data-video-clip-slot-inspector-embedded={props.embedded ? "true" : undefined}
      onKeyDown={keyDown}
    >
      <header>
        <div><h3 id="video-clip-slot-inspector-title">Clip Inspector</h3><span>{props.layer?.label ?? "No layer"}</span></div>
        <Show when={!props.embedded}>
          <button type="button" aria-label="Close clip inspector" onClick={close}>Close</button>
        </Show>
      </header>
      <Show when={localError()}>{(error) => <p class="videoClipSlotInspectorError" role="alert" data-no-localize>{error()}</p>}</Show>
      <Show when={props.layer} fallback={<p>Select a Video layer to author a clip bank.</p>}>
        <label>
          Media asset
          {/* Selection is bound per option (`selected`) rather than via select.value: when the asset list is
              replaced the <For> recreates every option and the browser would otherwise fall back to the first one. */}
          <select onInput={(event) => setAssetDraft(Number(event.currentTarget.value))} data-clip-inspector-asset>
            <Show when={assetMissing()}>
              <option value={currentAssetId() ?? ""} selected={assetDraft() === null} disabled data-no-localize>{`Asset ${currentAssetId()} (not in library)`}</option>
            </Show>
            <For each={props.assets}>{(asset) => <option value={asset.id} selected={asset.id === selectedAssetId()} data-no-localize>{asset.label}</option>}</For>
          </select>
        </label>
        <Show when={slot()} fallback={
          <button type="button" class="primary" disabled={selectedAssetId() === null} onClick={() => selectedAssetId() !== null && void props.onCreate(selectedAssetId()!, null)}>
            Create Slot
          </button>
        }>
          {(current) => <>
            <dl><div><dt>Slot ID</dt><dd data-no-localize>{current().id}</dd></div><div><dt>Runtime</dt><dd>See bank badges</dd></div></dl>
            <div class="videoClipSlotInspectorActions">
              <button type="button" disabled={assignDisabled()} onClick={() => !assignDisabled() && void props.onAssign(current().id, assetDraft()!)}>Assign</button>
              <button type="button" onClick={() => void props.onDuplicate(current().id)}>Duplicate</button>
              <button type="button" onClick={() => void props.onSetDefault(current().id)} disabled={props.layer?.default_clip_slot_id === current().id}>Default</button>
              <button type="button" onClick={() => void props.onReorder(current().id, -1)}>Move Earlier</button>
              <button type="button" onClick={() => void props.onReorder(current().id, 1)}>Move Later</button>
              <button type="button" class="danger" disabled={Boolean(props.removeDisabledReason)} title={props.removeDisabledReason ?? undefined} onClick={() => remove(current().id)}>Remove</button>
            </div>
            <div class="videoClipSlotInspectorFields">
              <label>In point (ms)<input type="number" min="0" step="1" value={inPointMs()} onInput={(event) => setInPointMs(Number(event.currentTarget.value))} /></label>
              <label>Out point (ms)<input type="number" min="0" step="1" value={outPointMs()} placeholder="End of media" onInput={(event) => setOutPointMs(event.currentTarget.value)} /></label>
              <label>Loop mode<select value={loopMode()} onInput={(event) => setLoopMode(event.currentTarget.value as VideoClipSlotSummary["loop_mode"])}><option>Once</option><option>Loop</option><option>PingPong</option></select></label>
              <label>Speed<input type="number" min="-4" max="4" step="0.05" value={speed()} onInput={(event) => setSpeed(Number(event.currentTarget.value))} /></label>
              <label>Launch quantization<select value={quantization()} onInput={(event) => setQuantization(event.currentTarget.value as VideoClipSlotSummary["launch_quantization"])}><option value="Immediate">Immediate</option><option value="NextBeat">Next Beat</option><option value="NextBar">Next Bar</option></select></label>
            </div>
            <details>
              <summary>Cue points</summary>
              <label>JSON cue point list<textarea rows="5" value={cuePoints()} onInput={(event) => setCuePoints(event.currentTarget.value)} /></label>
            </details>
            <details>
              <summary>Effect overrides</summary>
              <label>JSON effect override list<textarea rows="5" value={effectOverrides()} onInput={(event) => setEffectOverrides(event.currentTarget.value)} /></label>
            </details>
            <VideoEffectScopePanel
              scopeLabel="Clip FX"
              scope={{ scope: "clip", layer_id: props.layer!.id, slot_id: current().id }}
              catalog={props.effectCatalog}
              onApplyCatalog={props.onApplyEffectCatalog}
              onImportIsf={props.onImportScopeIsf}
              onApplyPreset={props.onApplyEffectPreset}
              onRemoveChain={props.onRemoveEffectChain}
            />
            <button type="button" class="primary" onClick={save}>Save Clip</button>
          </>}
        </Show>
      </Show>
    </aside>
  );
}
