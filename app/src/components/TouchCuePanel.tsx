import { For, Show } from "solid-js";
import type { CueSummary, EngineSnapshot } from "../types";
import { cueIdentityHue, identityCssColor } from "../identityColor";

type MaybePromise = void | Promise<void>;

export interface TouchCuePad {
  slot: string;
  index: number;
  cue: CueSummary | null;
}

interface TouchCuePanelProps {
  snapshot: EngineSnapshot;
  activeCue: CueSummary | null;
  nextCue: CueSummary | null;
  cuePads: TouchCuePad[];
  cuePadRangeLabel: string;
  cuePadBank: number;
  cuePadBankCount: number;
  onPreviousBank: () => void;
  onNextBank: () => void;
  onTriggerCue: (cueId: number) => MaybePromise;
}

export function TouchCuePanel(props: TouchCuePanelProps) {
  return (
    <section class="panel touchPanel touchCuePanel">
      <div class="panelHeader">
        <h2>Touch Cues</h2>
        <Show when={props.activeCue} fallback={<span>Standby</span>}>
          {(cue) => <span data-no-localize>{cue().label}</span>}
        </Show>
      </div>
      <div class="touchCueStatus">
        <div>
          <span>Active</span>
          <Show when={props.activeCue} fallback={<strong>None</strong>}>
            {(cue) => <strong data-no-localize>{cue().label}</strong>}
          </Show>
        </div>
        <div>
          <span>Next</span>
          <Show when={props.nextCue} fallback={<strong>None</strong>}>
            {(cue) => <strong data-no-localize>{cue().label}</strong>}
          </Show>
        </div>
      </div>
      <div class="liveCuePadHeader">
        <h3>Cue Pads</h3>
        <span>{props.cuePadRangeLabel}</span>
        <button onClick={props.onPreviousBank} disabled={props.cuePadBank === 0}>Prev</button>
        <button onClick={props.onNextBank} disabled={props.cuePadBank >= props.cuePadBankCount - 1}>Next</button>
      </div>
      <div class="touchCuePadGrid">
        <For each={props.cuePads}>
          {(pad) => (
            <button
              class={`liveCuePad ${pad.cue?.id === props.snapshot.active_cue_id ? "active" : ""} ${
                pad.cue?.id === props.nextCue?.id ? "next" : ""
              }`}
              style={pad.cue ? { "--identity": identityCssColor(cueIdentityHue(pad.cue.id), "text") } : undefined}
              disabled={!pad.cue}
              onClick={() => {
                if (pad.cue) void props.onTriggerCue(pad.cue.id);
              }}
            >
              <span>{pad.slot}</span>
              <Show when={pad.cue} fallback={<strong>Empty</strong>}>
                {(cue) => <strong data-no-localize>{cue().label}</strong>}
              </Show>
              <small>{pad.cue ? `${pad.cue.fade_ms} ms` : "-"}</small>
            </button>
          )}
        </For>
      </div>
      <Show when={props.snapshot.cues.length === 0}>
        <p class="empty">No cues. Create one in Control &gt; Live with Store Cue.</p>
      </Show>
      <Show when={props.snapshot.active_fade}>
        {(fade) => (
          <div class="liveFadeMeter touchFadeMeter">
            <span>{Math.round(fade().progress * 100)}%</span>
            <progress max="1" value={fade().progress} />
          </div>
        )}
      </Show>
    </section>
  );
}
