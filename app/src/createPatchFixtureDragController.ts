import { createMemo, createSignal, type Accessor, type Setter } from "solid-js";
import { addressRange, fixtureFootprint } from "./dmxAddressing";
import type { FrontendTauriInvoke } from "./tauriInvokeCommands";
import type { PatchedFixtureSummary } from "./types";

export interface PatchFixtureDragPreviewRange {
  index: number;
  start: number;
  range: [number, number];
}

type PatchAddressConflict = (
  ranges: PatchFixtureDragPreviewRange[],
  targetUniverse: number,
  excludedFixtureId?: number | null,
) => string;

interface PatchFixtureDragControllerOptions {
  invoke: FrontendTauriInvoke;
  activeUniverse: Accessor<number>;
  setPatchGridUniverse: (universe: number) => void;
  setDmxPatchViewMode: (mode: "grid" | "list") => void;
  patchAddressConflictFor: PatchAddressConflict;
  refreshSnapshot: () => Promise<unknown>;
  setSelectedFixtureLabelDraft: Setter<string>;
  setSelectedFixtureUniverseDraft: Setter<number>;
  setSelectedFixtureAddressDraft: Setter<number>;
  setMessage: (message: string) => unknown;
}

export function createPatchFixtureDragController(options: PatchFixtureDragControllerOptions) {
  const [source, setSource] = createSignal<PatchedFixtureSummary | null>(null);
  const [address, setAddress] = createSignal<number | null>(null);
  const [dropRejected, setDropRejected] = createSignal(false);

  const footprint = createMemo(() => {
    const fixture = source();
    return fixture ? Math.max(1, fixtureFootprint(fixture)) : 0;
  });
  const ranges = createMemo(() => {
    const fixture = source();
    const start = address();
    if (!fixture || start === null) return [];
    const range = addressRange(start, footprint());
    return range ? [{ index: 0, start, range }] : [];
  });
  const endAddress = createMemo(() => ranges().at(-1)?.range[1] ?? address() ?? 0);
  const conflictText = createMemo(() => {
    const fixture = source();
    if (!fixture || address() === null) return "";
    if (endAddress() > 512) return `Fixture exceeds DMX universe at address ${endAddress()}.`;
    return options.patchAddressConflictFor(ranges(), options.activeUniverse(), fixture.id);
  });
  const invalid = createMemo(() => Boolean(conflictText()));
  const dndStatus = createMemo(() => {
    if (dropRejected()) return "rejected" as const;
    if (!source() || address() === null) return null;
    return invalid() ? "conflict" as const : "valid" as const;
  });

  const begin = (fixture: PatchedFixtureSummary) => {
    setSource(fixture);
    setAddress(null);
    setDropRejected(false);
    options.setPatchGridUniverse(fixture.universe);
    options.setDmxPatchViewMode("grid");
  };

  const clear = () => {
    setSource(null);
    setAddress(null);
    setDropRejected(false);
  };

  const hover = (channel: number) => {
    if (source()) setAddress(channel);
  };

  const leave = () => {
    if (!dropRejected()) setAddress(null);
  };

  const drop = async (channel: number) => {
    const fixture = source();
    if (!fixture) return;
    const fixtureSize = Math.max(1, fixtureFootprint(fixture));
    setAddress(channel);
    const targetUniverse = options.activeUniverse();
    const conflict = conflictText();
    if (conflict) {
      setDropRejected(true);
      options.setMessage("Conflict: move rejected");
      return;
    }
    if (fixture.universe === targetUniverse && fixture.address === channel) {
      options.setMessage(`${fixture.label} is already at U${targetUniverse} A${channel}.`);
      clear();
      return;
    }

    try {
      await options.invoke("set_fixture_patch", {
        fixtureId: fixture.id,
        label: fixture.label,
        universe: targetUniverse,
        address: channel,
      });
      options.setSelectedFixtureLabelDraft(fixture.label);
      options.setSelectedFixtureUniverseDraft(targetUniverse);
      options.setSelectedFixtureAddressDraft(channel);
      options.setMessage(
        `Moved ${fixture.label} to U${targetUniverse} A${channel}-${channel + fixtureSize - 1}.`,
      );
      await options.refreshSnapshot();
      clear();
    } catch (error) {
      setDropRejected(true);
      options.setMessage(String(error));
    }
  };

  return {
    source,
    address,
    dndStatus,
    previewRanges: ranges,
    invalid,
    begin,
    clear,
    hover,
    leave,
    drop,
  };
}
