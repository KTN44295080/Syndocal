import { mediaPathBaseName } from "./mediaHealthModel";
import type { MediaAssetId, MediaAssetSummary } from "./types";

/** File listing returned by the read-only `select_media_relink_folder` command. */
export interface MediaRelinkFolderFile {
  path: string;
  file_name: string;
  byte_size: number;
}

export interface MediaRelinkFolderScan {
  directory: string;
  files: MediaRelinkFolderFile[];
  /** True when the folder held more than the scan bound; some files may be absent. */
  truncated: boolean;
}

/**
 * "exact": same file name and same byte size as the recorded asset.
 * "name_only": same file name, but the size differs or the project never
 * recorded one. Both are only proposals: applying either still goes through
 * the ordinary relink operation, which checks the content hash.
 */
export type MediaRelinkConfidence = "exact" | "name_only";

export interface MediaRelinkProposal {
  assetId: MediaAssetId;
  label: string;
  previousPath: string;
  replacementPath: string;
  replacementByteSize: number;
  recordedByteSize: number | null;
  confidence: MediaRelinkConfidence;
}

export interface MediaRelinkAmbiguity {
  assetId: MediaAssetId;
  label: string;
  previousPath: string;
  candidatePaths: string[];
}

export interface MediaRelinkUnmatched {
  assetId: MediaAssetId;
  label: string;
  previousPath: string;
}

export interface MediaRelinkPlan {
  directory: string;
  truncated: boolean;
  proposals: MediaRelinkProposal[];
  ambiguous: MediaRelinkAmbiguity[];
  unmatched: MediaRelinkUnmatched[];
}

/** Case-insensitive on every platform: Windows and macOS volumes usually are. */
const nameKey = (name: string) => name.normalize("NFC").toLowerCase();

/**
 * Proposes replacements for the missing assets from a folder listing. It never
 * guesses: several candidates that cannot be told apart by size are reported as
 * ambiguous instead of being picked, and an asset with no recorded path is
 * unmatched. The plan is pure data for the operator to review.
 */
export function planFolderRelink(
  missingAssets: readonly MediaAssetSummary[],
  scan: MediaRelinkFolderScan,
): MediaRelinkPlan {
  const byName = new Map<string, MediaRelinkFolderFile[]>();
  for (const file of scan.files) {
    const key = nameKey(file.file_name);
    const bucket = byName.get(key);
    if (bucket) bucket.push(file);
    else byName.set(key, [file]);
  }
  const proposals: MediaRelinkProposal[] = [];
  const ambiguous: MediaRelinkAmbiguity[] = [];
  const unmatched: MediaRelinkUnmatched[] = [];
  for (const asset of missingAssets) {
    const previousPath = asset.source.path ?? "";
    const base = mediaPathBaseName(previousPath);
    const candidates = base ? byName.get(nameKey(base)) ?? [] : [];
    if (candidates.length === 0) {
      unmatched.push({ assetId: asset.id, label: asset.label, previousPath });
      continue;
    }
    const recordedByteSize = asset.byte_size ?? null;
    const sameSize = recordedByteSize === null
      ? []
      : candidates.filter((candidate) => candidate.byte_size === recordedByteSize);
    const chosen = sameSize.length === 1
      ? { file: sameSize[0], confidence: "exact" as const }
      : sameSize.length === 0 && candidates.length === 1
        ? { file: candidates[0], confidence: "name_only" as const }
        : null;
    if (!chosen) {
      ambiguous.push({
        assetId: asset.id,
        label: asset.label,
        previousPath,
        candidatePaths: (sameSize.length > 1 ? sameSize : candidates).map((candidate) => candidate.path),
      });
      continue;
    }
    proposals.push({
      assetId: asset.id,
      label: asset.label,
      previousPath,
      replacementPath: chosen.file.path,
      replacementByteSize: chosen.file.byte_size,
      recordedByteSize,
      confidence: chosen.confidence,
    });
  }
  return { directory: scan.directory, truncated: scan.truncated, proposals, ambiguous, unmatched };
}

/** One reviewed proposal handed to the ticketed relink operation. */
export interface MediaRelinkApplyRequest {
  assetId: MediaAssetId;
  label: string;
  /** The path the project recorded when the proposal was made; a changed path voids the request. */
  previousPath: string;
  replacementPath: string;
}

export interface MediaRelinkApplyFailure {
  assetId: MediaAssetId;
  label: string;
  message: string;
}

/**
 * Outcome of applying reviewed proposals. `differentContent` lists files whose
 * hash does not match the recorded asset: a batch never adopts different
 * content, the operator relinks those one at a time with explicit confirmation.
 */
export interface MediaRelinkApplyResult {
  relinked: MediaAssetId[];
  differentContent: Array<{ assetId: MediaAssetId; label: string }>;
  failed: MediaRelinkApplyFailure[];
  /** Requests never attempted because the project changed or an error stopped the batch. */
  skipped: MediaAssetId[];
}

export function mediaRelinkApplySummary(result: MediaRelinkApplyResult): string {
  const parts = [
    result.relinked.length > 0 ? `${result.relinked.length} relinked` : "",
    result.differentContent.length > 0 ? `${result.differentContent.length} have different content` : "",
    result.failed.length > 0 ? `${result.failed.length} failed` : "",
    result.skipped.length > 0 ? `${result.skipped.length} not attempted` : "",
  ].filter(Boolean);
  return parts.length > 0 ? `Folder relink: ${parts.join(", ")}.` : "Folder relink: nothing was applied.";
}
