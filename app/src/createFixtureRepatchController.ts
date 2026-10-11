import { createMemo, createSignal, type Accessor, type Setter } from "solid-js";
import { fixtureMovePlan } from "./fixtureRepatchPolicy";
import type { FrontendTauriInvoke } from "./tauriInvokeCommands";
import type { PatchedFixtureSummary } from "./types";

export interface PatchFixtureDragPreviewRange { index: number; start: number; range: [number, number]; }
interface FixtureRepatchControllerOptions {
  invoke: FrontendTauriInvoke;
  activeUniverse: Accessor<number>;
  fixtures: Accessor<PatchedFixtureSummary[]>;
  projectFence: () => { epoch: number | null; revision: number; hash: string };
  setPatchGridUniverse: (universe: number) => void;
  setDmxPatchViewMode: (mode: "grid" | "list") => void;
  refreshSnapshot: () => Promise<unknown>;
  setSelectedFixtureLabelDraft: Setter<string>;
  setSelectedFixtureUniverseDraft: Setter<number>;
  setSelectedFixtureAddressDraft: Setter<number>;
  setMessage: (message: string) => unknown;
}

export function createFixtureRepatchController(options: FixtureRepatchControllerOptions) {
  const [source, setSource] = createSignal<PatchedFixtureSummary | null>(null);
  const [selection, setSelection] = createSignal<PatchedFixtureSummary[]>([]);
  const [address, setAddress] = createSignal<number | null>(null);
  const [dropRejected, setDropRejected] = createSignal(false);
  const [busy, setBusy] = createSignal(false);
  let fence: ReturnType<typeof options.projectFence> | null = null;
  const plan = createMemo(() => {
    if (!source() || address() === null) return { changes: [], error: "" };
    try { return { changes: fixtureMovePlan(selection(), source()!, options.activeUniverse(), address()!, options.fixtures()), error: "" }; }
    catch (error) { return { changes: [], error: String(error) }; }
  });
  const invalid = createMemo(() => Boolean(plan().error));
  const dndStatus = createMemo(() => dropRejected() ? "rejected" as const
    : !source() || address() === null ? null : invalid() ? "conflict" as const : "valid" as const);
  const begin = (fixture: PatchedFixtureSummary, fixtures = [fixture]) => {
    if (busy()) return;
    fence = options.projectFence();
    setSource(fixture); setSelection([...fixtures]); setAddress(null); setDropRejected(false);
    options.setPatchGridUniverse(fixture.universe); options.setDmxPatchViewMode("grid");
  };
  const clear = () => { setSource(null); setSelection([]); setAddress(null); setDropRejected(false); fence = null; };
  const hover = (channel: number) => { if (source() && !busy()) setAddress(channel); };
  const leave = () => { if (!dropRejected() && !busy()) setAddress(null); };
  const drop = async (channel: number) => {
    const fixture = source(), capturedFence = fence;
    if (!fixture || busy()) return;
    setAddress(channel);
    const preview = plan();
    if (preview.error) { setDropRejected(true); options.setMessage(`Move rejected: ${preview.error}`); return; }
    if (!capturedFence || capturedFence.epoch === null || !capturedFence.hash) {
      setDropRejected(true); options.setMessage("Project authority is unavailable; nothing was moved."); return;
    }
    const changes = preview.changes.map(({ range: _range, ...change }) => change);
    if (changes.every(change => change.universe === change.expectedUniverse && change.address === change.expectedAddress)) { clear(); return; }
    setBusy(true);
    try {
      await options.invoke("set_fixture_patches", { changes,
        __expectedProjectEpoch: capturedFence.epoch, __expectedProjectRevision: capturedFence.revision,
        __expectedCheckpointHash: capturedFence.hash });
      options.setSelectedFixtureLabelDraft(fixture.label);
      options.setSelectedFixtureUniverseDraft(changes[0].universe);
      options.setSelectedFixtureAddressDraft(changes.find(change => change.fixtureId === fixture.id)!.address);
      await options.refreshSnapshot();
      options.setMessage(`Moved ${changes.length} fixture(s), preserving their address spacing.`); clear();
    } catch (error) { setDropRejected(true); options.setMessage(String(error)); }
    finally { setBusy(false); }
  };
  return { source, selection, address, busy, dndStatus,
    previewRanges: createMemo(() => plan().changes.map((change, index) => ({ index, start: change.address, range: change.range }))),
    invalid, begin, clear, hover, leave, drop };
}
