import type { VideoEffectCatalog, VideoIsfEffectStageSummary, VideoIsfEffectSummary, VideoIsfImageSource, VideoLayerSummary } from "./types";
import { readIsfMetadata } from "./fxCatalogModel";
import { cloneFxStage } from "./fxParameterModel";

export const MAX_ISF_IMAGE_INPUTS = 4;
export interface FxImageInput { name: string; label: string }

export function fxImageInputs(source: string): FxImageInput[] {
  const inputs = readIsfMetadata(source).INPUTS;
  if (!Array.isArray(inputs)) return [];
  return inputs.filter((input) => input && input.TYPE === "image" && typeof input.NAME === "string")
    .map((input) => ({ name: input.NAME as string, label: typeof input.LABEL === "string" && input.LABEL.trim() ? input.LABEL.trim() : input.NAME as string }));
}

/** The host image is inputImage when declared, otherwise the first image input. */
export function fxAdditionalImageInputs(source: string): FxImageInput[] {
  const images = fxImageInputs(source);
  const primary = images.find((input) => input.name === "inputImage")?.name ?? images[0]?.name;
  return images.filter((input) => input.name !== primary);
}

export const fxImageSource = (stage: VideoIsfEffectStageSummary, name: string): VideoIsfImageSource =>
  stage.image_bindings?.find((binding) => binding.name === name)?.source ?? { kind: "none" };

export function fxImportedImagePathError(path: string, sourcePath?: string | null): string | null {
  if (!sourcePath) return "ISF imported image requires a shader source path";
  const parts = path.replace(/\\/g, "/").split("/");
  if (!path || path !== path.trim() || path.length > 4096 || /[:\u0000-\u001f]/.test(path)
    || /^[\\/]/.test(path) || parts.some((part) => !part || part === "." || part === "..")
    || !/\.(png|jpe?g)$/i.test(path)) return "ISF imported image path is invalid";
  return null;
}

/** Explicit edit; preserve unedited bindings (including externally authored primary bindings). */
export function withFxImageSource(stage: VideoIsfEffectStageSummary, name: string, source: VideoIsfImageSource): VideoIsfEffectStageSummary {
  const images = fxImageInputs(stage.source);
  if (images.length > MAX_ISF_IMAGE_INPUTS) throw new Error("ISF declares too many image inputs; the safety limit is 4");
  if (!fxAdditionalImageInputs(stage.source).some((input) => input.name === name)) throw new Error("ISF image binding '" + name + "' has no declared additional image input");
  if (source.kind === "layer" && (!Number.isSafeInteger(source.layer_id) || source.layer_id < 1)) throw new Error("ISF image input layer is missing");
  if (source.kind === "imported") {
    const error = fxImportedImagePathError(source.path, stage.source_path);
    if (error) throw new Error(error);
  }
  const next = cloneFxStage(stage);
  next.image_bindings = [...(next.image_bindings ?? []).filter((binding) => binding.name !== name), { name, source: { ...source } }];
  return next;
}

const stagesOf = (effect?: VideoIsfEffectSummary | null) => effect ? [effect, ...(effect.stack ?? [])] : [];

/** Editor preflight over authored Clip/Layer edges, including disabled stages. Native validation is authoritative. */
export function fxImageLayerWouldCycle(ownerId: number, candidateId: number, layers: readonly VideoLayerSummary[], catalog?: VideoEffectCatalog): boolean {
  if (ownerId === candidateId) return true;
  const edges = new Map<number, Set<number>>();
  const collect = (owner: number, stages: readonly VideoIsfEffectStageSummary[]) => {
    let dependencies = edges.get(owner);
    if (!dependencies) { dependencies = new Set(); edges.set(owner, dependencies); }
    for (const stage of stages) for (const binding of stage.image_bindings ?? []) {
      if (binding.source.kind === "layer") dependencies.add(binding.source.layer_id);
    }
  };
  for (const layer of layers) collect(layer.id, stagesOf(layer.isf_effect));
  for (const chain of catalog?.effect_chains ?? []) {
    if (chain.scope.scope !== "layer" && chain.scope.scope !== "clip") continue;
    for (const stage of chain.stages) collect(chain.scope.layer_id, stagesOf(stage.effect.kind.effect));
  }
  const pending = [candidateId];
  const visited = new Set<number>();
  while (pending.length) {
    const current = pending.pop()!;
    if (current === ownerId) return true;
    if (visited.has(current)) continue;
    visited.add(current);
    pending.push(...(edges.get(current) ?? []));
  }
  return false;
}
