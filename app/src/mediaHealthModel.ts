import type { MediaAssetAvailability, MediaAssetId, MediaAssetSummary } from "./types";

/**
 * Operator-facing health states of a Media Library asset, ordered from the one
 * that needs action first to the one that needs none. Live sources have no
 * local file to verify, so they are counted separately and never listed.
 */
export type MediaHealthState = "missing" | "unreadable" | "hash_mismatch" | "unverified" | "available";

export const MEDIA_HEALTH_STATE_ORDER: readonly MediaHealthState[] = [
  "missing",
  "unreadable",
  "hash_mismatch",
  "unverified",
  "available",
];

/** States where the file on disk cannot be used as recorded and a relink may fix it. */
export const MEDIA_HEALTH_ATTENTION_STATES: readonly MediaHealthState[] = [
  "missing",
  "unreadable",
  "hash_mismatch",
];

export interface MediaHealthRow {
  assetId: MediaAssetId;
  label: string;
  /** Recorded path of the local file, or an empty string when the source has none. */
  path: string;
  state: MediaHealthState;
  byteSize: number | null;
  /** Epoch milliseconds of the last verification in this session, if any. */
  checkedAtMs: number | null;
  /** Engine detail for unreadable files and the two hashes of a mismatch. */
  detail: string;
}

export interface MediaHealthSummary {
  counts: Record<MediaHealthState, number>;
  live: number;
  total: number;
  attention: number;
}

export const isLocalMediaAsset = (asset: MediaAssetSummary): boolean =>
  asset.source.kind === "File" || asset.source.kind === "StillImage";

export const mediaHealthStateOf = (availability: MediaAssetAvailability | undefined): MediaHealthState | "live" => {
  switch (availability?.kind) {
    case "missing": return "missing";
    case "unreadable": return "unreadable";
    case "hash_mismatch": return "hash_mismatch";
    case "available_verified": return "available";
    case "live_source": return "live";
    case "available_unverified":
    default: return "unverified";
  }
};

const mediaHealthDetail = (availability: MediaAssetAvailability | undefined): string => {
  if (availability?.kind === "hash_mismatch") {
    return `Expected ${availability.expected.hex}; found ${availability.actual.hex}`;
  }
  if (availability?.kind === "unreadable") return availability.error;
  return "";
};

/**
 * Rows for every local-file asset. Assets whose source is a live input are
 * omitted: they cannot be missing, unreadable or hash-checked.
 */
export function buildMediaHealthRows(
  assets: readonly MediaAssetSummary[],
  availabilityById: Readonly<Record<number, MediaAssetAvailability>>,
  checkedAtById: Readonly<Record<number, number>> = {},
): MediaHealthRow[] {
  const rows: MediaHealthRow[] = [];
  for (const asset of assets) {
    if (!isLocalMediaAsset(asset)) continue;
    const availability = availabilityById[asset.id];
    const state = mediaHealthStateOf(availability);
    if (state === "live") continue;
    rows.push({
      assetId: asset.id,
      label: asset.label,
      path: asset.source.path ?? "",
      state,
      byteSize: asset.byte_size ?? null,
      checkedAtMs: checkedAtById[asset.id] ?? null,
      detail: mediaHealthDetail(availability),
    });
  }
  return rows;
}

export function summarizeMediaHealth(
  assets: readonly MediaAssetSummary[],
  availabilityById: Readonly<Record<number, MediaAssetAvailability>>,
): MediaHealthSummary {
  const counts: Record<MediaHealthState, number> = {
    missing: 0, unreadable: 0, hash_mismatch: 0, unverified: 0, available: 0,
  };
  let live = 0;
  for (const asset of assets) {
    if (!isLocalMediaAsset(asset)) { live += 1; continue; }
    const state = mediaHealthStateOf(availabilityById[asset.id]);
    if (state === "live") live += 1;
    else counts[state] += 1;
  }
  const attention = counts.missing + counts.unreadable + counts.hash_mismatch;
  return { counts, live, total: assets.length, attention };
}

export interface MediaHealthGroup {
  state: MediaHealthState;
  rows: MediaHealthRow[];
}

/** Non-empty groups in operator priority order; rows keep library order. */
export function groupMediaHealthRows(rows: readonly MediaHealthRow[]): MediaHealthGroup[] {
  return MEDIA_HEALTH_STATE_ORDER
    .map((state) => ({ state, rows: rows.filter((row) => row.state === state) }))
    .filter((group) => group.rows.length > 0);
}

/** Binary-prefixed size (KiB, MiB, GiB) with one decimal; empty when unknown. */
export function formatMediaByteSize(bytes: number | null | undefined): string {
  if (bytes === null || bytes === undefined || !Number.isFinite(bytes) || bytes < 0) return "";
  if (bytes < 1024) return `${Math.round(bytes)} B`;
  const units = ["KiB", "MiB", "GiB", "TiB"];
  let value = bytes / 1024;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) { value /= 1024; unit += 1; }
  return `${value.toFixed(value >= 100 ? 0 : 1)} ${units[unit]}`;
}

/** Final path segment for either separator style. */
export function mediaPathBaseName(path: string): string {
  const trimmed = path.replace(/[\\/]+$/, "");
  const index = Math.max(trimmed.lastIndexOf("/"), trimmed.lastIndexOf("\\"));
  return index >= 0 ? trimmed.slice(index + 1) : trimmed;
}
