import { createEffect, createMemo, createSignal, For, on, Show } from "solid-js";
import type { VideoEffectCatalog, VideoIsfEffectStageSummary, VideoIsfImageSource, VideoLayerSummary } from "../types";
import { fxAdditionalImageInputs, fxImageInputs, fxImageLayerWouldCycle, fxImageSource, fxImportedImagePathError, MAX_ISF_IMAGE_INPUTS, withFxImageSource, type FxImageInput } from "../fxImageInputModel";
import { explainFxRejection } from "../fxRejectionExplanation";
import { translateUiText } from "../uiLocalization";

interface VideoFxImageInputsProps {
  identity: string;
  layerId: number;
  stage: VideoIsfEffectStageSummary;
  layers?: readonly VideoLayerSummary[];
  catalog?: VideoEffectCatalog;
  locale?: string;
  disabled: boolean;
  onSetStage: (stage: VideoIsfEffectStageSummary) => unknown | Promise<unknown>;
}

/** Image choices use the same stage mutation/undo boundary as scalar controls. */
export function VideoFxImageInputs(props: VideoFxImageInputsProps) {
  const inputs = createMemo(() => fxImageInputs(props.stage.source));
  const additional = createMemo(() => fxAdditionalImageInputs(props.stage.source));
  const [problem, setProblem] = createSignal<string | null>(null);
  const [pending, setPending] = createSignal(false);
  const t = (text: string) => translateUiText(text, props.locale === "ja" ? "ja" : "en");
  const disabled = () => props.disabled || pending();
  createEffect(on(() => props.identity, () => setProblem(null)));
  const explanation = createMemo(() => problem() ? explainFxRejection(problem()!, props.locale) : null);
  const commit = async (name: string, source: VideoIsfImageSource) => {
    if (disabled()) return false;
    const actionIdentity = props.identity;
    setProblem(null); setPending(true);
    try {
      if (source.kind === "layer" && fxImageLayerWouldCycle(props.layerId, source.layer_id, props.layers ?? [], props.catalog)) {
        throw new Error("ISF image input dependency cycle");
      }
      const result = await props.onSetStage(withFxImageSource(props.stage, name, source));
      if (result === null) throw new Error("ISF image input binding update rejected");
      if (props.identity === actionIdentity) {
        const confirmed = fxImageSource(props.stage, name);
        const matches = source.kind === "none" ? confirmed.kind === "none"
          : source.kind === "layer" ? confirmed.kind === "layer" && confirmed.layer_id === source.layer_id
          : confirmed.kind === "imported" && confirmed.path === source.path;
        if (!matches) throw new Error("ISF image input binding update was not confirmed");
      }
      return true;
    } catch (error) {
      if (props.identity === actionIdentity) setProblem(error instanceof Error ? error.message : String(error));
      return false;
    } finally { setPending(false); }
  };
  const ImageInputRow = (row: { input: FxImageInput }) => {
    const source = () => fxImageSource(props.stage, row.input.name);
    const [choice, setChoice] = createSignal("none");
    const [path, setPath] = createSignal("");
    createEffect(on(() => props.identity + ":" + JSON.stringify(source()), () => {
      const current = source();
      setChoice(current.kind === "layer" ? "layer:" + current.layer_id : current.kind);
      setPath(current.kind === "imported" ? current.path : "");
    }));
    const currentLayer = () => { const current = source(); return current.kind === "layer" ? current.layer_id : null; };
    const missing = () => currentLayer() !== null && !(props.layers ?? []).some((layer) => layer.id === currentLayer());
    const wouldCycle = (id: number) => fxImageLayerWouldCycle(props.layerId, id, props.layers ?? [], props.catalog);
    const pathError = () => fxImportedImagePathError(path(), props.stage.source_path);
    return <div class="videoFxImageInput" data-fx-image-input={row.input.name}>
      <label><span data-no-localize>{row.input.label}</span>
        <select data-fx-image-source disabled={disabled()} aria-label={row.input.label} value={choice()} onChange={(event) => {
          const value = event.currentTarget.value;
          setChoice(value);
          const next: VideoIsfImageSource | null = value === "none" ? { kind: "none" }
            : value.startsWith("layer:") ? { kind: "layer", layer_id: Number(value.slice(6)) } : null;
          const selectionIdentity = props.identity;
          if (next) void commit(row.input.name, next).then((ok) => {
            if (!ok && props.identity === selectionIdentity) { const saved = source(); setChoice(saved.kind === "layer" ? "layer:" + saved.layer_id : saved.kind); }
          });
        }}>
          <option value="none">{t("None (transparent)")}</option>
          <option value="imported" disabled={!props.stage.source_path}>{t("Imported still")}</option>
          <Show when={missing()}><option value={"layer:" + currentLayer()} disabled>{t("Missing layer")} · {currentLayer()}</option></Show>
          <For each={props.layers ?? []}>{(layer) => <option data-no-localize value={"layer:" + layer.id} disabled={wouldCycle(layer.id)}>
            {layer.label} · #{layer.id}{wouldCycle(layer.id) ? " · " + t("Would create a cycle") : ""}
          </option>}</For>
        </select>
      </label>
      <Show when={choice() === "imported"}>
        <label><span>{t("Image path relative to the shader")}</span>
          <input data-fx-image-path data-no-localize type="text" value={path()} disabled={disabled()} maxLength={4096}
            aria-label={t("Image path relative to the shader")} onInput={(event) => setPath(event.currentTarget.value)} />
        </label>
        <p class="videoFxImageHint">{t("Use a PNG or JPEG inside the shader folder. Subfolders are allowed.")}</p>
        <button type="button" data-fx-image-apply disabled={disabled() || Boolean(pathError())}
          onClick={() => void commit(row.input.name, { kind: "imported", path: path() })}>{t("Apply image")}</button>
        <Show when={path() && pathError()}><p class="videoFxImageHint">{t("Choose a relative PNG or JPEG path without parent folders.")}</p></Show>
      </Show>
      <Show when={!props.stage.source_path}><p class="videoFxImageHint">{t("Import the shader from a file to bind a still image.")}</p></Show>
    </div>;
  };
  return <Show when={inputs().length === 0 || additional().length > 0}>
    <section class="videoFxImageInputs" data-fx-image-inputs aria-label={t("Image inputs")}>
      <strong>{t("Image inputs")}</strong>
      <Show when={inputs().length === 0}><p class="videoFxImageHint" data-fx-generator>{t("This generator ignores the layer image and uses the layer render size.")}</p></Show>
      <Show when={inputs().length <= MAX_ISF_IMAGE_INPUTS} fallback={<p class="inlineError" role="alert">{t("At most four image inputs are supported.")}</p>}>
        <For each={additional()}>{(input) => <ImageInputRow input={input} />}</For>
      </Show>
      <Show when={additional().length > 0}><p class="videoFxImageHint">{t("Layer inputs use the output after Clip and Layer FX, before group or composition processing.")}</p></Show>
      <Show when={explanation()}>{(error) => <div class="inlineError" role="alert" data-fx-image-error>
        <strong data-no-localize>{error().title}</strong>
        <p data-no-localize>{error().detail} {error().action}</p>
        <details><summary>{t("Technical details")}</summary><pre data-no-localize>{error().raw}</pre></details>
      </div>}</Show>
    </section>
  </Show>;
}
