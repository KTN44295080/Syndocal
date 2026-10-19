import builtinSource from "../../crates/video/src/builtin_isf.rs?raw";
import type { VideoIsfEffectStageSummary, VideoIsfStageError, VideoLayerSummary } from "./types";

export type FxCategory = "Colour" | "Blur/Sharpen" | "Distort" | "Stylize" | "Generator" | "Transition" | "Key/Mask" | "Other";
export interface FxCatalogEntry {
  id: string;
  builtinId?: string;
  label: string;
  source: string;
  category: FxCategory;
  categories: string[];
  description: { en: string; ja: string };
  rejection?: string;
}
export interface FxImportRejection { label: string; source?: string; message: string }
export interface FxCatalogOptions {
  imported?: VideoIsfEffectStageSummary[];
  rejections?: FxImportRejection[];
  runtimeErrors?: VideoIsfStageError[];
  layers?: VideoLayerSummary[];
}

const descriptions: Record<string, [FxCategory, string, string]> = {
  invert: ["Colour", "Reverse the colours while preserving transparency.", "明暗と色を反転し、ネガフィルムのようにします。"],
  monochrome: ["Colour", "Fade from the original colours to black and white.", "元の色を残しながら、白黒の映像に近づけます。"],
  threshold: ["Key/Mask", "Separate light and dark areas into a soft black-and-white mask.", "明るさを境に白と黒へ分け、境目の柔らかさを調整します。"],
  posterize: ["Stylize", "Reduce colour steps for a bold, graphic look.", "色の段階を減らして、ポスターのような質感にします。"],
  "rgb-split": ["Distort", "Offset red and blue to create colour fringes.", "赤と青を左右にずらして、色のにじみを作ります。"],
  mirror: ["Distort", "Reflect the image horizontally, vertically or both.", "映像を左右・上下に折り返して、対称の模様を作ります。"],
  kaleidoscope: ["Distort", "Repeat the image around the centre like a kaleidoscope.", "中心のまわりに映像を繰り返し、万華鏡の模様を作ります。"],
  zoom: ["Distort", "Magnify or shrink the image around a chosen point.", "指定した位置を中心に、映像を拡大・縮小します。"],
  rotate: ["Distort", "Turn the image around a chosen centre point.", "指定した位置を中心に、映像を回転させます。"],
  strobe: ["Stylize", "Alternate the image with black frames at a chosen rate.", "映像と黒画面を交互に表示し、点滅の速さを調整します。"],
  scanlines: ["Stylize", "Overlay dark horizontal lines for a monitor-like texture.", "暗い横線を重ねて、ブラウン管のような質感にします。"],
  colorize: ["Colour", "Tint the image with a chosen colour while keeping its light and shade.", "明暗を生かしながら、選んだ色で映像を染めます。"],
  vignette: ["Stylize", "Darken the frame edges to draw attention to the centre.", "画面の周辺を暗くして、中央に視線を集めます。"],
  "glitch-shift": ["Distort", "Shift animated horizontal strips for a broken-signal effect.", "横方向の帯を不規則にずらして、信号が乱れたようにします。"],
};

export const normalizeFxSource = (source: string): string => source.replace(/\r\n?/g, "\n");
const sourceKeyOwners = new Map<string, string[]>();
/** Presentation IDs stay stable for this session. Full source identity resolves
 * hash collisions before an ID may be used by drag/add command adapters. */
export function fxSourceKey(source: string): string {
  source = normalizeFxSource(source);
  let hash = 2166136261;
  for (let index = 0; index < source.length; index++) hash = Math.imul(hash ^ source.charCodeAt(index), 16777619);
  const prefix = (hash >>> 0).toString(36);
  const owners = sourceKeyOwners.get(prefix) ?? [];
  let index = owners.indexOf(source);
  if (index < 0) { index = owners.length; owners.push(source); sourceKeyOwners.set(prefix, owners); }
  return `${prefix}:${index}`;
}

export function readIsfMetadata(source: string): Record<string, unknown> {
  const header = source.match(/\/\*\s*(\{[\s\S]*?\})\s*\*\//)?.[1];
  if (!header) return {};
  try { const value: unknown = JSON.parse(header); return value && typeof value === "object" && !Array.isArray(value) ? value as Record<string, unknown> : {}; }
  catch { return {}; }
}

export const builtinFxEntries: FxCatalogEntry[] = [...builtinSource.matchAll(/id:\s*"([^"]+)"\s*,\s*label:\s*"([^"]+)"\s*,\s*source:\s*r#"([\s\S]*?)"#/g)].map((match) => {
  const [category, en, ja] = descriptions[match[1]] ?? ["Other", "Built-in effect", "内蔵エフェクト"];
  const metadata = readIsfMetadata(match[3]);
  return { id: `builtin:${match[1]}`, builtinId: match[1], label: match[2], source: normalizeFxSource(match[3]), category, categories: Array.isArray(metadata.CATEGORIES) ? metadata.CATEGORIES.filter((value): value is string => typeof value === "string") : [], description: { en, ja } };
});

function inferCategory(categories: string[]): FxCategory {
  const text = categories.join(" ").toLowerCase();
  if (/colo[u]?r/.test(text)) return "Colour";
  if (/blur|sharpen/.test(text)) return "Blur/Sharpen";
  if (/distort|geometry|glitch/.test(text)) return "Distort";
  if (/generat/.test(text)) return "Generator";
  if (/transition/.test(text)) return "Transition";
  if (/key|mask/.test(text)) return "Key/Mask";
  if (/styl|temporal/.test(text)) return "Stylize";
  return "Other";
}

export function buildFxCatalog(options: FxCatalogOptions = {}): FxCatalogEntry[] {
  const entries = builtinFxEntries.map((entry) => ({ ...entry }));
  const sources = new Map(entries.map((entry) => [normalizeFxSource(entry.source), entry]));
  for (const effect of options.imported ?? []) {
    const normalizedSource = normalizeFxSource(effect.source);
    if (sources.has(normalizedSource)) continue;
    const metadata = readIsfMetadata(effect.source);
    const categories = Array.isArray(metadata.CATEGORIES) ? metadata.CATEGORIES.filter((value): value is string => typeof value === "string") : effect.categories;
    const description = typeof metadata.DESCRIPTION === "string" ? metadata.DESCRIPTION : effect.description;
    const entry: FxCatalogEntry = { id: `import:${fxSourceKey(effect.source)}`, label: effect.label, source: effect.source, category: inferCategory(categories), categories, description: description?.trim() ? { en: description, ja: description } : { en: "Imported shader", ja: "読み込んだシェーダー" } };
    entries.push(entry); sources.set(normalizedSource, entry);
  }
  for (const rejection of options.rejections ?? []) entries.push({ id: `rejected:${fxSourceKey(`${rejection.label}:${rejection.message}`)}`, label: rejection.label, source: rejection.source ?? "", category: "Other", categories: [], description: { en: "Imported shader", ja: "読み込んだシェーダー" }, rejection: rejection.message });
  for (const error of options.runtimeErrors ?? []) {
    const layer = options.layers?.find((item) => item.id === error.layer_id);
    const stage = layer?.isf_effect ? [layer.isf_effect, ...(layer.isf_effect.stack ?? [])][error.stage_index ?? 0] : undefined;
    const entry = stage ? sources.get(normalizeFxSource(stage.source)) : undefined;
    if (entry) entry.rejection = error.message;
  }
  return entries;
}

export function filterFxCatalog(entries: FxCatalogEntry[], query: string, category: FxCategory | "All", locale = "en"): FxCatalogEntry[] {
  const words = query.toLocaleLowerCase().trim().split(/\s+/).filter(Boolean);
  return entries.filter((entry) => (category === "All" || entry.category === category) && words.every((word) => `${entry.label} ${entry.category} ${fxCategoryLabel(entry.category, locale)} ${entry.categories.join(" ")} ${entry.description.en} ${entry.description.ja}`.toLocaleLowerCase().includes(word)));
}

export function fxCategoryLabel(category: FxCategory | "All", locale = "en"): string {
  if (locale !== "ja") return category;
  return ({ All: "すべて", Colour: "色", "Blur/Sharpen": "ぼかし・シャープ", Distort: "変形", Stylize: "質感", Generator: "生成", Transition: "切り替え", "Key/Mask": "キー・マスク", Other: "その他" })[category];
}

export const FX_DRAG_MIME = "application/x-syndocal-fx";
export function writeFxDragPayload(transfer: DataTransfer, entry: FxCatalogEntry): void {
  transfer.setData(FX_DRAG_MIME, entry.id);
  transfer.effectAllowed = "copy";
}
export function readFxDragPayload(transfer: DataTransfer, entries: FxCatalogEntry[]): FxCatalogEntry | undefined {
  const id = transfer.getData(FX_DRAG_MIME);
  return entries.find((entry) => entry.id === id && !entry.rejection);
}
