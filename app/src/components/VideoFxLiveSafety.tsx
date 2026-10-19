import { Show } from "solid-js";
import type { FxLiveSession } from "../createFxLiveSession";
import "./VideoFxLiveSafety.css";

export function VideoFxLiveSafety(props: { session?: FxLiveSession; layerId: number | null }) {
  const state = () => {
    const value = props.session?.current();
    return value?.layerId === props.layerId ? value : null;
  };
  return <aside class="videoFxLiveSafety" data-fx-live-safety aria-label="FX output safety">
    <details>
      <summary><strong>LIVE — affects Program</strong></summary>
      <p>Layer FX are shared by Preview and Program. P does not isolate effects. Changes apply immediately; use Undo FX to reverse the last edit.</p>
      <p>A/B switches the live layer between the original A and edited B. It also affects Program. Comparison expires after another project edit.</p>
    </details>
    <Show when={props.session && state()}>
      <div class="videoFxLiveActions">
        <button type="button" data-fx-live-undo disabled={!state() || !props.session!.canUndo()} onClick={() => void props.session!.undo()}>Undo FX</button>
        <button type="button" data-fx-compare="A" disabled={!state() || props.session!.busy()} aria-pressed={state()?.side === "A"}
          onClick={() => void props.session!.compare("A")}>A · Original (LIVE)</button>
        <button type="button" data-fx-compare="B" disabled={!state() || props.session!.busy()} aria-pressed={state()?.side === "B"}
          onClick={() => void props.session!.compare("B")}>B · Edited (LIVE)</button>
      </div>
    </Show>
  </aside>;
}
