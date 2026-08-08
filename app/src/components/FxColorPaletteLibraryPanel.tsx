import { createMemo, createSignal, For, Show } from "solid-js";
import {
  builtinFxColorPalettes,
  cloneFxPaletteStops,
  customFxColorPalettes,
  fxPaletteColorToHex,
  normalizeFxPaletteStops,
  type FxColorPaletteDefinition,
} from "../fxColorPalettes";
import type { ColorEffectStop, EffectKind, ReferencePaletteSummary } from "../types";

export interface FxColorPaletteLibraryPanelProps {
  effectType: EffectKind;
  palettes: ReferencePaletteSummary[];
  onApply: (palette: FxColorPaletteDefinition) => void | Promise<void>;
  onCreate: (label: string, stops: ColorEffectStop[]) => Promise<number | null>;
  onUpdate: (paletteId: number, label: string, stops: ColorEffectStop[]) => void | Promise<void>;
  onRemove: (paletteId: number, label: string) => boolean | Promise<boolean>;
}

const initialPalette = builtinFxColorPalettes[0];

export function FxColorPaletteLibraryPanel(props: FxColorPaletteLibraryPanelProps) {
  const [selectedId, setSelectedId] = createSignal(initialPalette.id);
  const [draftLabel, setDraftLabel] = createSignal(`${initialPalette.label} Custom`);
  const [draftStops, setDraftStops] = createSignal(cloneFxPaletteStops(initialPalette.stops));
  const [status, setStatus] = createSignal("");
  const [busy, setBusy] = createSignal(false);

  const palettes = createMemo(() => [
    ...builtinFxColorPalettes,
    ...customFxColorPalettes(props.palettes),
  ]);
  const selectedPalette = createMemo(() =>
    palettes().find((palette) => palette.id === selectedId()) ?? palettes()[0]);
  const selectedCustomId = createMemo(() => selectedPalette()?.customPaletteId ?? null);
  const orderedDraftStops = createMemo(() => normalizeFxPaletteStops(draftStops()));
  const draftError = createMemo(() => {
    const stops = orderedDraftStops();
    if (stops.length < 2 || stops.length > 16) return "FX palettes require 2 to 16 color stops.";
    if (stops.some((stop, index) => index > 0 && stop.position <= stops[index - 1].position)) {
      return "Each color stop needs a unique position.";
    }
    if (!draftLabel().trim()) return "Palette label is required.";
    return "";
  });
  const applyLabel = createMemo(() =>
    props.effectType === "Color"
      ? "Load into Colour FX"
      : props.effectType === "ColorMapping"
        ? "Load into Colour Mapping"
        : "Add Colour layer");

  const loadPalette = (palette: FxColorPaletteDefinition) => {
    setSelectedId(palette.id);
    setDraftLabel(palette.customPaletteId ? palette.label : `${palette.label} Custom`);
    setDraftStops(cloneFxPaletteStops(palette.stops));
    setStatus(`${palette.label}: ${palette.stops.length} colors loaded.`);
  };

  const updateStop = (index: number, patch: Partial<ColorEffectStop>) => {
    setDraftStops(orderedDraftStops().map((stop, candidateIndex) =>
      candidateIndex === index
        ? {
            ...stop,
            ...patch,
            color: patch.color ? { ...patch.color } : { ...stop.color },
          }
        : { ...stop, color: { ...stop.color } }));
  };

  const addStop = () => {
    const stops = orderedDraftStops();
    if (stops.length >= 16) return;
    let left = stops[0];
    let right = stops[1];
    let largestGap = (right?.position ?? 1) - (left?.position ?? 0);
    for (let index = 1; index < stops.length - 1; index += 1) {
      const gap = stops[index + 1].position - stops[index].position;
      if (gap > largestGap) {
        largestGap = gap;
        left = stops[index];
        right = stops[index + 1];
      }
    }
    if (!left || !right) return;
    setDraftStops([...stops, {
      position: (left.position + right.position) / 2,
      color: {
        red: Math.round((left.color.red + right.color.red) / 2),
        green: Math.round((left.color.green + right.color.green) / 2),
        blue: Math.round((left.color.blue + right.color.blue) / 2),
      },
    }]);
  };

  const runBusy = async (operation: () => Promise<void>) => {
    if (busy()) return;
    setBusy(true);
    try {
      await operation();
    } finally {
      setBusy(false);
    }
  };

  const createPalette = () => runBusy(async () => {
    if (draftError()) return;
    const paletteId = await props.onCreate(draftLabel().trim(), orderedDraftStops());
    if (paletteId === null) return;
    setSelectedId(`custom:${paletteId}`);
    setStatus(`Created ${draftLabel().trim()} with ${orderedDraftStops().length} colors.`);
  });

  const updatePalette = () => runBusy(async () => {
    const paletteId = selectedCustomId();
    if (paletteId === null || draftError()) return;
    await props.onUpdate(paletteId, draftLabel().trim(), orderedDraftStops());
    setStatus(`Updated ${draftLabel().trim()}.`);
  });

  const applyPalette = () => runBusy(async () => {
    const palette = selectedPalette();
    if (!palette) return;
    await props.onApply({ ...palette, stops: cloneFxPaletteStops(palette.stops) });
    setStatus(`${palette.label} applied to ${props.effectType}.`);
  });

  const removePalette = () => runBusy(async () => {
    const paletteId = selectedCustomId();
    if (paletteId === null) return;
    const removed = await props.onRemove(paletteId, selectedPalette()?.label ?? draftLabel());
    if (!removed) return;
    loadPalette(initialPalette);
    setStatus(`Removed custom palette ${paletteId}.`);
  });

  return (
    <section
      class="fxColorPaletteLibrary"
      aria-label={`Color palettes for ${props.effectType} FX`}
      data-fx-color-palette-library={props.effectType}
      data-built-in-palette-count={builtinFxColorPalettes.length}
    >
      <header>
        <div>
          <strong class="uiMicroLabel">Colour Palette</strong>
          <span>{builtinFxColorPalettes.length} built-in · {customFxColorPalettes(props.palettes).length} custom</span>
        </div>
        <button type="button" class="primary" disabled={busy()} onClick={() => void applyPalette()}>
          {applyLabel()}
        </button>
      </header>
      <div class="fxColorPalettePickerRow">
        <label>
          Palette
          <select
            value={selectedPalette()?.id ?? initialPalette.id}
            aria-label={`Palette for ${props.effectType} FX`}
            onInput={(event) => {
              const palette = palettes().find((candidate) => candidate.id === event.currentTarget.value);
              if (palette) loadPalette(palette);
            }}
          >
            <optgroup label="Built-in">
              <For each={builtinFxColorPalettes}>
                {(palette) => <option value={palette.id} data-no-localize>{palette.label}</option>}
              </For>
            </optgroup>
            <Show when={customFxColorPalettes(props.palettes).length > 0}>
              <optgroup label="Project custom">
                <For each={customFxColorPalettes(props.palettes)}>
                  {(palette) => <option value={palette.id} data-no-localize>{palette.label}</option>}
                </For>
              </optgroup>
            </Show>
          </select>
        </label>
        <div class="fxColorPaletteSwatches" aria-label={`${selectedPalette()?.label ?? "Palette"} colors`}>
          <For each={selectedPalette()?.stops ?? []}>
            {(stop, index) => (
              <span
                style={{ background: fxPaletteColorToHex(stop.color) }}
                title={`${Math.round(stop.position * 100)}% ${fxPaletteColorToHex(stop.color)}`}
                aria-label={`Color ${index() + 1}: ${fxPaletteColorToHex(stop.color)}`}
              />
            )}
          </For>
        </div>
      </div>
      <details class="fxColorPaletteEditorDisclosure">
        <summary>Edit / create palette</summary>
        <div class="fxColorPaletteEditor" data-fx-color-palette-editor>
          <label class="fxColorPaletteLabel">
            Palette label
            <input
              maxlength="64"
              value={draftLabel()}
              onInput={(event) => setDraftLabel(event.currentTarget.value)}
            />
          </label>
          <div class="fxColorPaletteStopList" aria-label="Editable FX palette colors">
            <For each={orderedDraftStops()}>
              {(stop, index) => (
                <div class="fxColorPaletteStopRow">
                  <span class="tabularNums">{index() + 1}</span>
                  <input
                    type="color"
                    value={fxPaletteColorToHex(stop.color)}
                    aria-label={`FX palette color ${index() + 1}`}
                    onInput={(event) => {
                      const hex = event.currentTarget.value.slice(1);
                      updateStop(index(), { color: {
                        red: Number.parseInt(hex.slice(0, 2), 16) * 257,
                        green: Number.parseInt(hex.slice(2, 4), 16) * 257,
                        blue: Number.parseInt(hex.slice(4, 6), 16) * 257,
                      } });
                    }}
                  />
                  <output class="tabularNums">{fxPaletteColorToHex(stop.color)}</output>
                  <label>
                    Position
                    <input
                      class="tabularNums"
                      type="number"
                      min="0"
                      max="1"
                      step="0.01"
                      value={stop.position.toFixed(2)}
                      onChange={(event) => updateStop(index(), { position: Number(event.currentTarget.value) })}
                    />
                  </label>
                  <button
                    type="button"
                    aria-label={`Remove FX palette color ${index() + 1}`}
                    title="Remove color"
                    disabled={orderedDraftStops().length <= 2}
                    onClick={() => setDraftStops(orderedDraftStops()
                      .filter((_, candidate) => candidate !== index()))}
                  >
                    <span aria-hidden="true">×</span>
                  </button>
                </div>
              )}
            </For>
          </div>
          <div class="fxColorPaletteEditorActions">
            <button type="button" disabled={orderedDraftStops().length >= 16} onClick={addStop}>Add color</button>
            <button type="button" class="primary" disabled={busy() || Boolean(draftError())} onClick={() => void createPalette()}>
              Save as new
            </button>
            <button type="button" disabled={busy() || selectedCustomId() === null || Boolean(draftError())} onClick={() => void updatePalette()}>
              Update custom
            </button>
            <button
              type="button"
              class="danger"
              disabled={busy() || selectedCustomId() === null}
              onClick={() => void removePalette()}
            >
              Remove custom
            </button>
          </div>
          <Show when={draftError()}>
            {(error) => <p class="fieldError textPretty" role="alert">{error()}</p>}
          </Show>
          <Show when={status()}>
            <p class="hint textPretty" role="status">{status()}</p>
          </Show>
        </div>
      </details>
    </section>
  );
}
