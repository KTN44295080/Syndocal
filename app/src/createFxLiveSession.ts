import { createSignal, type Accessor } from "solid-js";
import type { ProjectAuthorityToken } from "./projectAuthority";
import type { VideoIsfEffectSummary, VideoLayerSummary } from "./types";

const clone = <T,>(value: T): T => JSON.parse(JSON.stringify(value));
const sameAuthority = (a: ProjectAuthorityToken, b: ProjectAuthorityToken) =>
  a.project_epoch === b.project_epoch && a.project_revision === b.project_revision && a.checkpoint_hash === b.checkpoint_hash;
interface FxComparison {
  layerId: number;
  a: VideoIsfEffectSummary | null;
  b: VideoIsfEffectSummary | null;
  side: "A" | "B";
  authority: ProjectAuthorityToken;
}

/** Observes the existing ticketed FX commands. It owns no transport or output
 * path. A/B is explicitly LIVE, and expires on any unrelated authored edit. */
export function createFxLiveSession(options: {
  authority: Accessor<ProjectAuthorityToken>;
  layers: Accessor<VideoLayerSummary[]>;
  setEffect: (layerId: number, effect: VideoIsfEffectSummary | null, expected: ProjectAuthorityToken) => Promise<void>;
  undo: (expected: ProjectAuthorityToken) => void | Promise<void>;
}) {
  const [busy, setBusy] = createSignal(false);
  const [comparison, setComparison] = createSignal<FxComparison | null>(null);
  const [undoAuthority, setUndoAuthority] = createSignal<ProjectAuthorityToken | null>(null);
  const current = () => {
    const value = comparison();
    return value && sameAuthority(value.authority, options.authority()) ? value : null;
  };
  const run = async (layerId: number, action: () => void | Promise<void>) => {
    if (busy()) return;
    const layer = options.layers().find(entry => entry.id === layerId);
    if (!layer) return;
    const before = clone(layer.isf_effect ?? null);
    const authority = { ...options.authority() };
    const previous = current();
    setBusy(true);
    try {
      await action();
      const afterAuthority = options.authority();
      const afterLayer = options.layers().find(entry => entry.id === layerId);
      // A second authored publication may be an unrelated edit. Never offer
      // project Undo or a comparison under an ambiguous history head.
      if (afterAuthority.project_epoch !== authority.project_epoch || !afterLayer || afterAuthority.project_revision !== authority.project_revision + 1) return;
      const after = clone(afterLayer.isf_effect ?? null);
      if (JSON.stringify(before) === JSON.stringify(after)) return;
      setUndoAuthority({ ...afterAuthority });
      setComparison({ layerId, a: previous?.layerId === layerId ? previous.a : before,
        b: after, side: "B", authority: { ...afterAuthority } });
    } finally { setBusy(false); }
  };
  const compare = async (side: "A" | "B") => {
    const state = current();
    if (!state || state.side === side || busy()) return;
    setBusy(true);
    try {
      await options.setEffect(state.layerId, clone(side === "A" ? state.a : state.b), state.authority);
      const authority = options.authority();
      const actual = options.layers().find(entry => entry.id === state.layerId)?.isf_effect ?? null;
      if (authority.project_epoch === state.authority.project_epoch && authority.project_revision === state.authority.project_revision + 1 && JSON.stringify(actual) === JSON.stringify(side === "A" ? state.a : state.b)) {
        setComparison({ ...state, side, authority: { ...authority } });
        setUndoAuthority({ ...authority });
      }
    } finally { setBusy(false); }
  };
  const canUndo = () => !busy() && undoAuthority() !== null && sameAuthority(undoAuthority()!, options.authority());
  const undo = async () => {
    if (!canUndo()) return;
    setBusy(true);
    try { await options.undo({ ...undoAuthority()! }); setComparison(null); setUndoAuthority(null); }
    finally { setBusy(false); }
  };
  return { busy, run, current, compare, canUndo, undo };
}
export type FxLiveSession = ReturnType<typeof createFxLiveSession>;
