import { createSignal, For, Show } from "solid-js";
import type { PaletteKind, ReferencePaletteSummary } from "../types";

interface ReferencePalettePanelProps {
  palettes: ReferencePaletteSummary[];
  captureFixtureLabel: string | null;
  targetLabel: string;
  targetFixtureCount: number;
  onCreate: (label: string, kind: PaletteKind) => void | Promise<void>;
  onUpdate: (palette: ReferencePaletteSummary) => void | Promise<void>;
  onApply: (palette: ReferencePaletteSummary) => void | Promise<void>;
  onRemove: (palette: ReferencePaletteSummary) => void | Promise<void>;
}

const paletteKinds: PaletteKind[] = ["Intensity", "Position", "Color", "Beam", "All"];

export function ReferencePalettePanel(props: ReferencePalettePanelProps) {
  const [label, setLabel] = createSignal("New Palette");
  const [kind, setKind] = createSignal<PaletteKind>("Color");

  return (
    <section class="referencePalettePanel" aria-label="Reference palettes">
      <div class="programmerSummary">
        <strong>Palettes</strong>
        <span>{props.palettes.length} reference palette(s)</span>
        <span class="status">LIVE LINK</span>
      </div>
      <div class="referencePaletteCreate">
        <label>
          Label
          <input
            maxlength="64"
            value={label()}
            onInput={(event) => setLabel(event.currentTarget.value)}
          />
        </label>
        <label>
          Kind
          <select value={kind()} onInput={(event) => setKind(event.currentTarget.value as PaletteKind)}>
            <For each={paletteKinds}>{(paletteKind) => <option value={paletteKind}>{paletteKind}</option>}</For>
          </select>
        </label>
        <button
          class="primary"
          disabled={!props.captureFixtureLabel || !label().trim()}
          onClick={() => void props.onCreate(label(), kind())}
        >
          Capture
        </button>
      </div>
      <p class="hint">
        Capture reads the current values from {props.captureFixtureLabel ?? "the selected fixture"}. Apply targets {props.targetLabel} ({props.targetFixtureCount} fixture(s)). Cue references resolve the latest palette value on every GO.
      </p>
      <div class="referencePaletteList">
        <Show when={props.palettes.length === 0}>
          <p class="empty">No palettes. Select a fixture, choose a kind, then Capture.</p>
        </Show>
        <For each={props.palettes}>
          {(palette) => (
            <article class="referencePaletteItem">
              <div>
                <strong data-no-localize>{palette.label}</strong>
                <span>{palette.kind} · {palette.values.length} value(s)</span>
              </div>
              <div class="buttonRow">
                <button
                  class="primary"
                  disabled={props.targetFixtureCount === 0}
                  onClick={() => void props.onApply(palette)}
                >
                  Apply
                </button>
                <button
                  disabled={!props.captureFixtureLabel}
                  onClick={() => void props.onUpdate(palette)}
                >
                  Recapture
                </button>
                <button class="danger" onClick={() => void props.onRemove(palette)}>Remove</button>
              </div>
            </article>
          )}
        </For>
      </div>
    </section>
  );
}
