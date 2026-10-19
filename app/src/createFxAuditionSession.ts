import { batch, createEffect, createSignal, type Accessor } from "solid-js";
import type { VideoIsfEffectStageSummary, VideoIsfEffectSummary, VideoLayerSummary, VjPreviewTransportSummary } from "./types";
import {
  auditionAppendStage,
  auditionMoveStage,
  auditionRemoveStage,
  auditionReplaceChain,
  auditionResetStage,
  auditionSetControl,
  auditionSetEnabled,
  cloneFxChain,
  fxChainsEqual,
  fxChainStages,
  FX_AUDITION_MAX_STAGES,
  type FxChain,
} from "./fxAuditionModel";

/** Why an audition can no longer be taken. The draft is kept so the operator
 * can read what happened, but Take is refused until a new audition starts. */
export type FxAuditionStale = "layer-missing" | "preview-changed" | "live-changed";
/** Failures reported to the operator; the UI localises these. */
export type FxAuditionError = "preview-unavailable" | "audition-rejected" | "take-failed" | "stage-limit" | "builtin-unavailable" | "preview-busy";

export interface FxAuditionState {
  layerId: number;
  /** The authored chain when the audition began; Take refuses if it moved. */
  base: FxChain;
  draft: FxChain;
  /** B shows the draft in Preview, A shows the authored chain. */
  side: "A" | "B";
  stale: FxAuditionStale | null;
}

const HISTORY_LIMIT = 20;

export interface FxAuditionOptions {
  layers: Accessor<VideoLayerSummary[]>;
  transport: Accessor<VjPreviewTransportSummary>;
  /** Stages the layer in Preview; resolves false with the backend reason when refused. */
  stagePreview: (layerId: number, onFailure: (reason: string) => void) => Promise<boolean>;
  /** Preview-only chain: active=false returns Preview to the authored chain. */
  setPreviewEffect: (layerId: number, active: boolean, effect: FxChain) => Promise<VjPreviewTransportSummary | null>;
  describeBuiltin: (presetId: string) => Promise<VideoIsfEffectSummary | null>;
  /** Writes the draft through the existing ticketed layer-chain command. */
  commit: (layerId: number, effect: FxChain) => Promise<void>;
}

/** Holds an effect chain that only the Preview monitor renders. Nothing here
 * writes the project or Program until `take`, which uses the ordinary chain
 * command, so Undo, ticketing and ownership rules stay with that command. */
export function createFxAuditionSession(options: FxAuditionOptions) {
  const [state, setState] = createSignal<FxAuditionState | null>(null);
  const [busy, setBusy] = createSignal(false);
  const [error, setError] = createSignal<FxAuditionError | null>(null);
  const [reason, setReason] = createSignal("");
  const [history, setHistory] = createSignal<FxChain[]>([]);
  // True while a chain is being published to Preview: the monitor's own flag
  // legitimately lags behind the draft until the command returns.
  const [publishing, setPublishing] = createSignal(false);
  let pushing: Promise<void> | null = null;
  let pushAgain = false;

  const layerOf = (layerId: number) => options.layers().find((layer) => layer.id === layerId);
  const active = (layerId: number | null | undefined) => {
    const value = state();
    return value !== null && layerId != null && value.layerId === layerId ? value : null;
  };

  const staleReason = (value: FxAuditionState): FxAuditionStale | null => {
    const layer = layerOf(value.layerId);
    const transport = options.transport();
    if (!layer) return "layer-missing";
    if (transport.layer_id !== value.layerId || (value.side === "B" && transport.effect_audition === false)) return "preview-changed";
    if (!fxChainsEqual(layer.isf_effect, value.base)) return "live-changed";
    return null;
  };

  // Detect an audition that can no longer be committed. Skipped while a command
  // is in flight because its own publication legitimately changes these inputs.
  createEffect(() => {
    const value = state();
    if (!value || busy() || publishing()) return;
    const stale = staleReason(value);
    if (stale !== value.stale) setState({ ...value, stale });
  });

  const pushToPreview = async () => {
    const value = state();
    if (!value) return true;
    const summary = await options.setPreviewEffect(value.layerId, value.side === "B", value.side === "B" ? value.draft : null);
    if (!summary || summary.effect_audition !== (value.side === "B")) {
      setError("audition-rejected");
      return false;
    }
    return true;
  };
  // Coalesce a burst of edits (sliders, Randomize) into the newest draft. The
  // caller marks `publishing` in the same batch as its state change, so the
  // stale check never sees a draft the monitor has not been told about yet.
  const publish = async () => {
    pushAgain = true;
    if (pushing) return pushing;
    pushing = (async () => {
      try {
        while (pushAgain) { pushAgain = false; await pushToPreview(); }
      } finally { pushing = null; setPublishing(false); }
    })();
    return pushing;
  };
  /** Returns Preview to the authored chain. True once Preview confirms it, or when
   * nothing of this layer is staged any more; a busy transport slot is retried. */
  const endPreview = async (layerId: number): Promise<boolean> => {
    for (let attempt = 0; attempt < 4; attempt++) {
      if (options.transport().layer_id !== layerId) return true;
      try {
        const summary = await options.setPreviewEffect(layerId, false, null);
        if (summary?.effect_audition === false) return true;
      } catch { /* the staged session may already be gone */ }
      await new Promise((done) => setTimeout(done, 150));
    }
    return options.transport().layer_id !== layerId;
  };

  const begin = async (layerId: number): Promise<boolean> => {
    if (busy()) return false;
    const layer = layerOf(layerId);
    if (!layer) return false;
    const previous = state();
    if (previous?.layerId === layerId && !previous.stale) return true;
    setBusy(true); setError(null); setReason("");
    try {
      if (previous && !(await endPreview(previous.layerId))) { setError("preview-busy"); return false; }
      setState(null); setHistory([]);
      if (options.transport().layer_id !== layerId) {
        const staged = await options.stagePreview(layerId, setReason);
        if (!staged) { setError("preview-unavailable"); return false; }
      }
      const base = cloneFxChain(layer.isf_effect);
      const summary = await options.setPreviewEffect(layerId, true, base);
      if (!summary?.effect_audition) { setError("audition-rejected"); return false; }
      setState({ layerId, base, draft: cloneFxChain(base), side: "B", stale: null });
      return true;
    } finally { setBusy(false); }
  };

  const edit = (layerId: number, change: (draft: FxChain) => FxChain) => {
    const value = active(layerId);
    if (!value || value.stale) return;
    const next = change(value.draft);
    if (fxChainsEqual(next, value.draft)) return;
    batch(() => {
      setHistory((entries) => [...entries.slice(-(HISTORY_LIMIT - 1)), cloneFxChain(value.draft)]);
      setState({ ...value, draft: cloneFxChain(next), side: "B" });
      setPublishing(true);
    });
    void publish();
  };

  const appendStage = (layerId: number, stage: VideoIsfEffectStageSummary) => {
    const value = active(layerId);
    if (value && fxChainStages(value.draft).length >= FX_AUDITION_MAX_STAGES) { setError("stage-limit"); return; }
    setError(null);
    edit(layerId, (draft) => auditionAppendStage(draft, stage));
  };

  /** Starts (or continues) an audition and appends a built-in or already known effect. */
  const addToAudition = async (layerId: number, resolve: () => Promise<VideoIsfEffectStageSummary | null>): Promise<boolean> => {
    const current = active(layerId);
    // A stale audition cannot be edited; start a fresh one from the live chain.
    if ((!current || current.stale) && !(await begin(layerId))) return false;
    const stage = await resolve();
    if (!stage) { setError("builtin-unavailable"); return false; }
    appendStage(layerId, stage);
    return true;
  };

  const show = async (side: "A" | "B") => {
    const value = state();
    if (!value || value.stale || value.side === side || busy()) return;
    batch(() => { setState({ ...value, side }); setPublishing(true); });
    await publish();
  };

  const undo = async () => {
    const value = state();
    const entries = history();
    if (!value || value.stale || entries.length === 0) return;
    batch(() => {
      setHistory(entries.slice(0, -1));
      setState({ ...value, draft: cloneFxChain(entries[entries.length - 1]), side: "B" });
      setPublishing(true);
    });
    await publish();
  };

  const discard = async () => {
    const value = state();
    if (!value || busy()) return;
    setBusy(true);
    try {
      if (await endPreview(value.layerId)) { setState(null); setHistory([]); setError(null); }
      else setError("preview-busy");
    } finally { setBusy(false); }
  };

  const take = async (): Promise<boolean> => {
    const value = state();
    // Re-check at the moment of Take; the reactive flag skips its own in-flight commands.
    if (!value || value.stale || staleReason(value) || busy()) return false;
    setBusy(true); setError(null);
    try {
      await options.commit(value.layerId, value.draft);
      if (!fxChainsEqual(layerOf(value.layerId)?.isf_effect, value.draft)) { setError("take-failed"); return false; }
      if (await endPreview(value.layerId)) { setState(null); setHistory([]); }
      else {
        // Program has the chain. Keep the audition open on the new baseline so Discard can retry ending Preview.
        setState({ ...value, base: cloneFxChain(value.draft), stale: null });
        setError("preview-busy");
      }
      return true;
    } finally { setBusy(false); }
  };

  const describeBuiltinStage = (presetId: string) => async () => {
    const effect = await options.describeBuiltin(presetId);
    return effect ? fxChainStages(effect)[0] ?? null : null;
  };

  return {
    state, busy, error, reason, active, begin, edit, appendStage, addToAudition, describeBuiltinStage, show, undo, discard, take,
    canUndo: () => history().length > 0 && !busy() && state()?.stale === null,
    /** Draft-editing adapters with the same shape as the live layer commands. */
    setControl: (layerId: number, index: number, name: string, value: readonly number[]) => edit(layerId, (draft) => auditionSetControl(draft, index, name, value)),
    setEnabled: (layerId: number, index: number, enabled: boolean) => edit(layerId, (draft) => auditionSetEnabled(draft, index, enabled)),
    moveStage: (layerId: number, index: number, delta: number) => edit(layerId, (draft) => auditionMoveStage(draft, index, delta)),
    removeStage: (layerId: number, index: number) => edit(layerId, (draft) => auditionRemoveStage(draft, index)),
    resetStage: (layerId: number, index: number) => edit(layerId, (draft) => auditionResetStage(draft, index)),
    replaceChain: (layerId: number, chain: FxChain) => edit(layerId, () => auditionReplaceChain(chain)),
  };
}
export type FxAuditionSession = ReturnType<typeof createFxAuditionSession>;
