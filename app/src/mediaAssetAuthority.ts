import type {
  MediaAssetImportReport,
  MediaAssetId,
  MediaAssetRelinkOutcome,
  MediaAssetRelinkPolicy,
  MediaAssetRelinkPrepareReport,
  MediaAssetRelinkReport,
  VideoSourceKind,
} from "./types";

/** The raw IPC surface used before a project history ticket exists. */
export type MediaAssetInvoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

/**
 * A renderer-local ID has to be globally unlikely to collide because the
 * backend operation registry is deliberately keyed before it knows whether a
 * later project transaction will be opened. A random 21-bit renderer prefix
 * plus a monotonic 32-bit suffix stays exact in JavaScript and valid in Rust's
 * `u64`, while avoiding same-renderer collisions entirely.
 */
const mediaAssetRequestPrefix = Math.floor(Math.random() * ((2 ** 21) - 1)) + 1;
const mediaAssetRequestSequenceLimit = (2 ** 32) - 1;
let nextMediaAssetRequestSequence = 0;

export const allocateMediaAssetRequestId = (): number => {
  if (nextMediaAssetRequestSequence >= mediaAssetRequestSequenceLimit) {
    throw new Error("Media operation request IDs are exhausted; restart Syndocal before importing again.");
  }
  nextMediaAssetRequestSequence += 1;
  return (mediaAssetRequestPrefix * (2 ** 32)) + nextMediaAssetRequestSequence;
};

export type MediaAssetOperationIdentity = {
  requestId: number;
  operationGeneration: number;
};

export type MediaAssetImportEntryStatus = "prepared" | "imported" | "reused" | "skipped" | "failed";

type MediaAssetOperationOptions = {
  invoke: MediaAssetInvoke;
  expectedEpoch: number;
  ownerId: string;
  requestId?: number;
  signal?: AbortSignal;
};

export type PreparedMediaAssetCommitOptions<T> = MediaAssetOperationOptions & {
  kind: Extract<VideoSourceKind, "File" | "StillImage">;
  paths: string[];
  /** Must be a project-mutation command handled by App's generic ticket wrapper. */
  commitCommand: "commit_prepared_media_assets"
    | "commit_prepared_video_file_layer"
    | "commit_prepared_still_image_layer"
    | "commit_prepared_local_media_layers"
    | "commit_prepared_bootstrap_vj_show";
  commitArgs?: Record<string, unknown>;
  /**
   * First-run show creation is atomic at the operator level: a mixed batch
   * must not create a partial show. Ordinary Media Library import intentionally
   * leaves this false so valid assets can land with per-input failure truth.
   */
  requireAllPrepared?: boolean;
};

export type PreparedMediaAssetCommitResult<T> = {
  report: MediaAssetImportReport;
  committed: T | null;
};

export type PreparedMediaAssetRelinkOptions = MediaAssetOperationOptions & {
  assetId: MediaAssetId;
  replacementPath: string;
  policy: MediaAssetRelinkPolicy;
};

export type PreparedMediaAssetRelinkResult = {
  report: MediaAssetRelinkPrepareReport | MediaAssetRelinkReport;
  outcome: MediaAssetRelinkOutcome | null;
  committed: boolean;
};

const abortError = () => new DOMException("Media operation was cancelled.", "AbortError");

const assertOperationNotAborted = (signal: AbortSignal | undefined) => {
  if (signal?.aborted) throw abortError();
};

const knownOperationFromReport = (report: {
  request_id: number;
  operation_generation: number;
}): MediaAssetOperationIdentity => ({
  requestId: report.request_id,
  operationGeneration: report.operation_generation,
});

/**
 * The current backend exposes `operation_generation` only in Prepare's reply.
 * Consequently an AbortSignal cannot cancel an in-flight initial hash yet.
 * Once that reply is known, the helper can submit a best-effort cancellation
 * request for that request/generation/owner identity. This does not establish
 * that cancellation wins against a concurrent staged commit: a backend
 * Cancelable→Admitted linearization/receipt is still required for that terminal
 * guarantee. A future start-operation IPC can plug into this seam without
 * changing callers.
 */
const installKnownOperationCancellation = (
  invoke: MediaAssetInvoke,
  ownerId: string,
  signal: AbortSignal | undefined,
  operation: () => MediaAssetOperationIdentity | null,
  commitAdmissionStarted: () => boolean,
) => {
  let cancellation: Promise<void> | null = null;
  const cancelKnown = () => {
    const known = operation();
    // The callback only classifies whether App has received a Begin ticket; it
    // is not a frontend cancellation barrier. Before that callback this sends
    // a best-effort request; after it, signal-driven cancellation is withheld
    // rather than being presented as a definitive publication outcome.
    if (!known || commitAdmissionStarted()) return Promise.resolve();
    if (cancellation) return cancellation;
    cancellation = invoke<boolean>("cancel_media_asset_operation", {
      requestId: known.requestId,
      operationGeneration: known.operationGeneration,
      ownerId,
    }).then(() => undefined).catch(() => undefined);
    return cancellation;
  };
  const onAbort = () => { void cancelKnown(); };
  signal?.addEventListener("abort", onAbort, { once: true });
  return {
    cancelKnown,
    /** Error cleanup is still best-effort even if ticket admission was attempted. */
    cancelKnownAfterError: () => {
      const known = operation();
      if (!known) return Promise.resolve();
      if (cancellation) return cancellation;
      cancellation = invoke<boolean>("cancel_media_asset_operation", {
        requestId: known.requestId,
        operationGeneration: known.operationGeneration,
        ownerId,
      }).then(() => undefined).catch(() => undefined);
      return cancellation;
    },
    dispose: () => signal?.removeEventListener("abort", onAbort),
  };
};

/**
 * Run local media preparation and full byte finalization outside history, then
 * use exactly one staged commit through App's generic project-ticket wrapper.
 */
export async function prepareFinalizeAndCommitMediaAssets<T>(
  options: PreparedMediaAssetCommitOptions<T>,
): Promise<PreparedMediaAssetCommitResult<T>> {
  const requestId = options.requestId ?? allocateMediaAssetRequestId();
  let knownOperation: MediaAssetOperationIdentity | null = null;
  let commitAdmissionStarted = false;
  const cancellation = installKnownOperationCancellation(
    options.invoke,
    options.ownerId,
    options.signal,
    () => knownOperation,
    () => commitAdmissionStarted,
  );
  try {
    assertOperationNotAborted(options.signal);
    const prepared = await options.invoke<MediaAssetImportReport>("prepare_local_media_assets", {
      requestId,
      kind: options.kind,
      paths: options.paths,
      expectedEpoch: options.expectedEpoch,
      ownerId: options.ownerId,
    });
    knownOperation = knownOperationFromReport(prepared);
    assertOperationNotAborted(options.signal);

    if (options.requireAllPrepared
      && (prepared.failed > 0 || prepared.skipped > 0 || prepared.prepared !== options.paths.length)) {
      await cancellation.cancelKnownAfterError();
      throw new Error("First-run VJ setup requires every selected media file to prepare successfully; no project changes were made.");
    }

    const token = prepared.prepared_import_token;
    // A report containing only skipped/failed input is truthful completion and
    // intentionally creates neither a catalog mutation nor an empty history entry.
    if (token === null || token === undefined) return { report: prepared, committed: null };

    const finalized = await options.invoke<MediaAssetImportReport>("finalize_prepared_media_assets", {
      preparedImportToken: token,
      requestId: knownOperation.requestId,
      operationGeneration: knownOperation.operationGeneration,
      expectedEpoch: options.expectedEpoch,
      ownerId: options.ownerId,
    });
    knownOperation = knownOperationFromReport(finalized);
    assertOperationNotAborted(options.signal);

    // This is the only point at which App's generic wrapper may begin a
    // project ticket. Its private `__expectedProjectEpoch` fences an A picker
    // or file hash from committing into a later B project. The callback records
    // that Begin returned a ticket for cancellation classification only; it does
    // not interrupt App's mapping flush/Begin path or prove a cancellation wins.
    // Backend Cancelable→Admitted linearization/receipt remains required for that.
    const committed = await options.invoke<T>(options.commitCommand, {
      ...(options.commitArgs ?? {}),
      preparedImportToken: token,
      requestId: knownOperation.requestId,
      operationGeneration: knownOperation.operationGeneration,
      __expectedProjectEpoch: options.expectedEpoch,
      __onProjectTransactionOpened: () => { commitAdmissionStarted = true; },
    });
    return { report: finalized, committed };
  } catch (error) {
    await cancellation.cancelKnownAfterError();
    throw error;
  } finally {
    cancellation.dispose();
  }
}

/**
 * Relinking follows the same long-I/O/short-ticket boundary.  A typed outcome
 * without a server token is a successful read-only decision, never a mutation.
 */
export async function prepareFinalizeAndCommitMediaAssetRelink(
  options: PreparedMediaAssetRelinkOptions,
): Promise<PreparedMediaAssetRelinkResult> {
  const requestId = options.requestId ?? allocateMediaAssetRequestId();
  let knownOperation: MediaAssetOperationIdentity | null = null;
  let commitAdmissionStarted = false;
  const cancellation = installKnownOperationCancellation(
    options.invoke,
    options.ownerId,
    options.signal,
    () => knownOperation,
    () => commitAdmissionStarted,
  );
  try {
    assertOperationNotAborted(options.signal);
    const prepared = await options.invoke<MediaAssetRelinkPrepareReport>("prepare_media_asset_relink", {
      requestId,
      assetId: options.assetId,
      replacementPath: options.replacementPath,
      policy: options.policy,
      expectedEpoch: options.expectedEpoch,
      ownerId: options.ownerId,
    });
    knownOperation = knownOperationFromReport(prepared);
    assertOperationNotAborted(options.signal);
    const token = prepared.prepared_relink_token;
    if (token === null || token === undefined) {
      return { report: prepared, outcome: prepared.outcome ?? null, committed: false };
    }

    const finalized = await options.invoke<MediaAssetRelinkPrepareReport>(
      "finalize_prepared_media_asset_relink",
      {
        preparedRelinkToken: token,
        requestId: knownOperation.requestId,
        operationGeneration: knownOperation.operationGeneration,
        expectedEpoch: options.expectedEpoch,
        ownerId: options.ownerId,
      },
    );
    knownOperation = knownOperationFromReport(finalized);
    assertOperationNotAborted(options.signal);

    const committed = await options.invoke<MediaAssetRelinkReport>(
      "commit_prepared_media_asset_relink",
      {
        preparedRelinkToken: token,
        requestId: knownOperation.requestId,
        operationGeneration: knownOperation.operationGeneration,
        __expectedProjectEpoch: options.expectedEpoch,
        // Classification hook only: it records that App received a Begin ticket;
        // it is not a cancellation/publication guarantee.
        __onProjectTransactionOpened: () => { commitAdmissionStarted = true; },
      },
    );
    return { report: committed, outcome: committed.outcome, committed: true };
  } catch (error) {
    await cancellation.cancelKnownAfterError();
    throw error;
  } finally {
    cancellation.dispose();
  }
}

export const mediaAssetImportReportMessage = (report: MediaAssetImportReport): string => {
  const imported = report.imported ?? 0;
  const reused = report.reused ?? 0;
  const failed = report.failed ?? 0;
  const skipped = report.skipped ?? 0;
  const parts = [
    imported > 0 ? `${imported} imported` : "",
    reused > 0 ? `${reused} reused` : "",
    failed > 0 ? `${failed} failed` : "",
    skipped > 0 ? `${skipped} skipped` : "",
  ].filter(Boolean);
  return parts.length > 0 ? `Media library: ${parts.join(", ")}.` : "No media files were imported.";
};

export const mediaAssetRelinkOutcomeMessage = (outcome: MediaAssetRelinkOutcome): string => {
  switch (outcome.kind) {
    case "relinked":
      return outcome.adopted_replacement
        ? `Relinked media asset ${outcome.asset_id} and adopted the replacement.`
        : `Relinked media asset ${outcome.asset_id}.`;
    case "needs_explicit_adoption":
      return `Media asset ${outcome.asset_id} needs explicit replacement approval.`;
    case "hash_mismatch":
      return `Media asset ${outcome.asset_id} does not match the expected content hash.`;
    case "missing_replacement":
      return `Replacement for media asset ${outcome.asset_id} is missing.`;
    case "unreadable_replacement":
      return `Replacement for media asset ${outcome.asset_id} cannot be read: ${outcome.error}`;
    case "live_source":
      return `Live media asset ${outcome.asset_id} cannot be relinked as a local file.`;
  }
};
