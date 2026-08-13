import type {
  MediaAssetAuthoritativeBootstrapResult,
  MediaAssetAuthoritativeCommitKind,
  MediaAssetAuthoritativeImportResult,
  MediaAssetAuthoritativeLayerResult,
  MediaAssetAuthoritativeRelinkResult,
  MediaAssetAuthoritativeTerminalEnvelope,
  MediaAssetImportReport,
  MediaAssetId,
  MediaAssetOperationStartReport,
  MediaAssetRelinkOutcome,
  MediaAssetRelinkPolicy,
  MediaAssetRelinkPrepareReport,
  MediaAssetRelinkReport,
  VjFirstRunSetupResult,
  VideoSourceKind,
} from "./types";
import type { ProjectAuthorityToken } from "./projectAuthority";

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

type MediaAssetOperationAuthority = {
  projectEpoch: number;
  projectRevision: number;
  checkpointHash: string;
};

export type MediaAssetImportEntryStatus = "prepared" | "imported" | "reused" | "skipped" | "failed";

type MediaAssetOperationOptions = {
  invoke: MediaAssetInvoke;
  expectedEpoch: number;
  ownerId: string;
  requestId?: number;
  signal?: AbortSignal;
};

export type MediaAssetAuthoritativeCommitCommand =
  | "commit_prepared_media_assets_authoritative"
  | "commit_prepared_media_asset_relink_authoritative"
  | "commit_prepared_video_file_layer_authoritative"
  | "commit_prepared_still_image_layer_authoritative"
  | "commit_prepared_local_media_layers_authoritative"
  | "commit_prepared_bootstrap_vj_show_authoritative";

type MediaAssetAuthoritativeCommitResponseMap = {
  commit_prepared_media_assets_authoritative: MediaAssetAuthoritativeImportResult;
  commit_prepared_media_asset_relink_authoritative: MediaAssetAuthoritativeRelinkResult;
  commit_prepared_video_file_layer_authoritative: MediaAssetAuthoritativeLayerResult;
  commit_prepared_still_image_layer_authoritative: MediaAssetAuthoritativeLayerResult;
  commit_prepared_local_media_layers_authoritative: MediaAssetAuthoritativeLayerResult;
  commit_prepared_bootstrap_vj_show_authoritative: MediaAssetAuthoritativeBootstrapResult;
};

type MediaAssetAuthoritativeCommittedValueMap = {
  commit_prepared_media_assets_authoritative: MediaAssetImportReport;
  commit_prepared_media_asset_relink_authoritative: MediaAssetRelinkReport;
  commit_prepared_video_file_layer_authoritative: number;
  commit_prepared_still_image_layer_authoritative: number;
  commit_prepared_local_media_layers_authoritative: number[];
  commit_prepared_bootstrap_vj_show_authoritative: VjFirstRunSetupResult;
};

export type PreparedMediaAssetCommitOptions<C extends MediaAssetAuthoritativeCommitCommand> =
  MediaAssetOperationOptions & {
  kind: Extract<VideoSourceKind, "File" | "StillImage">;
  paths: string[];
  /** Backend-owned mutation invoked through App's direct authoritative facade. */
  commitCommand: C;
  commitArgs?: Record<string, unknown>;
  /**
   * First-run show creation is atomic at the operator level: a mixed batch
   * must not create a partial show. Ordinary Media Library import intentionally
   * leaves this false so valid assets can land with per-input failure truth.
   */
  requireAllPrepared?: boolean;
  };

export type PreparedMediaAssetCommitResult<C extends MediaAssetAuthoritativeCommitCommand> = {
  report: MediaAssetImportReport;
  committed: MediaAssetAuthoritativeCommittedValueMap[C] | null;
  /** False when a later project C already superseded this terminal B result. */
  applicationCurrent: boolean;
  /** Exact content authority published by the terminal result (or Start for a no-op). */
  terminalAuthority: ProjectAuthorityToken;
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
  applicationCurrent: boolean;
  terminalAuthority: ProjectAuthorityToken;
};

export type MediaAssetOperationLease = {
  signal: AbortSignal;
  release: () => void;
};

/**
 * Flush and validate the picker-era project fence before registering the new
 * operation's AbortController. Mapping publication may abort older operations;
 * the operation being started must not exist yet or it would cancel itself.
 */
export const preflightAndBeginMediaAssetOperation = async (
  expectedEpoch: number,
  preflight: (expectedEpoch: number) => Promise<number>,
  begin: () => MediaAssetOperationLease,
): Promise<{ expectedEpoch: number; operation: MediaAssetOperationLease }> => {
  const currentEpoch = await preflight(expectedEpoch);
  if (currentEpoch !== expectedEpoch) {
    throw new Error("Project changed before the media operation started; nothing was applied.");
  }
  return { expectedEpoch: currentEpoch, operation: begin() };
};

const authoritativeApplicationCurrentProperty = "__syndocalAuthoritativeApplicationCurrent";

const authoritativeApplicationCurrent = (value: unknown): boolean => {
  if (!value || typeof value !== "object") return true;
  return (value as Record<string, unknown>)[authoritativeApplicationCurrentProperty] !== false;
};

const copyAuthoritativeApplicationCurrent = (source: unknown, target: unknown) => {
  if (!target || typeof target !== "object") return;
  if (authoritativeApplicationCurrent(source)) return;
  Object.defineProperty(target, authoritativeApplicationCurrentProperty, {
    configurable: true,
    enumerable: false,
    value: false,
  });
};

const abortError = () => new DOMException("Media operation was cancelled.", "AbortError");

const assertOperationNotAborted = (signal: AbortSignal | undefined) => {
  if (signal?.aborted) throw abortError();
};

const exactNonNegativeInteger = (value: unknown, label: string): number => {
  if (!Number.isSafeInteger(value) || (value as number) < 0) {
    throw new Error(`${label} must be a non-negative exact integer.`);
  }
  return value as number;
};

const exactPositiveInteger = (value: unknown, label: string): number => {
  const normalized = exactNonNegativeInteger(value, label);
  if (normalized === 0) throw new Error(`${label} must be non-zero.`);
  return normalized;
};

const knownOperationFromReport = (report: {
  request_id: unknown;
  operation_generation: unknown;
}, phase: string): MediaAssetOperationIdentity => ({
  requestId: exactPositiveInteger(report.request_id, `${phase} request_id`),
  operationGeneration: exactPositiveInteger(report.operation_generation, `${phase} operation_generation`),
});

const authorityFromReport = (report: {
  project_epoch: unknown;
  project_revision: unknown;
  checkpoint_hash: unknown;
}, phase: string): MediaAssetOperationAuthority => {
  if (typeof report.checkpoint_hash !== "string") {
    throw new Error(`${phase} checkpoint_hash must be a string.`);
  }
  return {
    projectEpoch: exactNonNegativeInteger(report.project_epoch, `${phase} project_epoch`),
    projectRevision: exactNonNegativeInteger(report.project_revision, `${phase} project_revision`),
    checkpointHash: report.checkpoint_hash,
  };
};

const projectAuthorityFromOperation = (authority: MediaAssetOperationAuthority): ProjectAuthorityToken => ({
  project_epoch: authority.projectEpoch,
  project_revision: authority.projectRevision,
  checkpoint_hash: authority.checkpointHash,
});

const projectAuthorityFromResponse = <C extends MediaAssetAuthoritativeCommitCommand>(
  response: MediaAssetAuthoritativeCommitResponseMap[C],
): ProjectAuthorityToken => {
  const authority = response.mutation.authority;
  return {
    project_epoch: authority.project_epoch,
    project_revision: authority.project_revision,
    checkpoint_hash: authority.checkpoint_hash,
  };
};

const assertReportContinuity = (
  phase: string,
  report: {
    request_id: unknown;
    operation_generation: unknown;
    project_epoch: unknown;
    project_revision: unknown;
    checkpoint_hash: unknown;
  },
  operation: MediaAssetOperationIdentity,
  authority: MediaAssetOperationAuthority,
) => {
  const reportedOperation = knownOperationFromReport(report, phase);
  if (reportedOperation.requestId !== operation.requestId
    || reportedOperation.operationGeneration !== operation.operationGeneration) {
    throw new Error(`${phase} operation identity changed; no project changes were made.`);
  }
  const reportedAuthority = authorityFromReport(report, phase);
  if (reportedAuthority.projectEpoch !== authority.projectEpoch
    || reportedAuthority.projectRevision !== authority.projectRevision
    || reportedAuthority.checkpointHash !== authority.checkpointHash) {
    throw new Error(`${phase} project authority changed; no project changes were made.`);
  }
};

type MediaAssetAuthoritativeCommitDescriptor = {
  commandKind: MediaAssetAuthoritativeCommitKind;
  terminalKind: "import" | "relink" | "layers" | "bootstrap";
  shapeFields: string[];
};

const videoSourceKindShapeName = (kind: unknown): string => {
  switch (kind) {
    case "File": return "file";
    case "StillImage": return "still_image";
    default: throw new Error("Authoritative local media commit requires File or StillImage kind.");
  }
};

// Rust `str::trim()` follows Unicode White_Space, while ECMAScript `trim()`
// additionally strips U+FEFF. Keep receipt fingerprints byte-identical to
// `normalize_video_layer_label` without depending on the WebView Unicode table.
const rustWhitespaceCodePoint = (codePoint: number): boolean =>
  (codePoint >= 0x0009 && codePoint <= 0x000d)
  || codePoint === 0x0020
  || codePoint === 0x0085
  || codePoint === 0x00a0
  || codePoint === 0x1680
  || (codePoint >= 0x2000 && codePoint <= 0x200a)
  || codePoint === 0x2028
  || codePoint === 0x2029
  || codePoint === 0x202f
  || codePoint === 0x205f
  || codePoint === 0x3000;

export const trimLikeRust = (value: string): string => {
  let start = 0;
  let end = value.length;
  while (start < end) {
    const codePoint = value.codePointAt(start);
    if (codePoint === undefined || !rustWhitespaceCodePoint(codePoint)) break;
    start += codePoint > 0xffff ? 2 : 1;
  }
  while (end > start) {
    const trailing = value.charCodeAt(end - 1);
    const codePoint = trailing >= 0xdc00 && trailing <= 0xdfff && end >= 2
      ? value.codePointAt(end - 2)
      : trailing;
    if (codePoint === undefined || !rustWhitespaceCodePoint(codePoint)) break;
    end -= codePoint > 0xffff ? 2 : 1;
  }
  return value.slice(start, end);
};

const authoritativeCommitDescriptor = (
  command: MediaAssetAuthoritativeCommitCommand,
  args: Record<string, unknown> | undefined,
): MediaAssetAuthoritativeCommitDescriptor => {
  switch (command) {
    case "commit_prepared_media_assets_authoritative":
      return { commandKind: "import", terminalKind: "import", shapeFields: [] };
    case "commit_prepared_media_asset_relink_authoritative":
      return { commandKind: "relink", terminalKind: "relink", shapeFields: [] };
    case "commit_prepared_video_file_layer_authoritative":
      return {
        commandKind: "video_file_layer",
        terminalKind: "layers",
        shapeFields: [trimLikeRust(String(args?.label ?? ""))],
      };
    case "commit_prepared_still_image_layer_authoritative":
      return {
        commandKind: "still_image_layer",
        terminalKind: "layers",
        shapeFields: [trimLikeRust(String(args?.label ?? ""))],
      };
    case "commit_prepared_local_media_layers_authoritative":
      return {
        commandKind: "local_media_layers",
        terminalKind: "layers",
        shapeFields: [videoSourceKindShapeName(args?.kind)],
      };
    case "commit_prepared_bootstrap_vj_show_authoritative":
      return {
        commandKind: "bootstrap_vj_show",
        terminalKind: "bootstrap",
        shapeFields: [videoSourceKindShapeName(args?.kind), "safe_output_v1"],
      };
  }
};

const appendLengthPrefixedUtf8 = (chunks: Uint8Array[], value: string) => {
  const bytes = new TextEncoder().encode(value);
  const length = new Uint8Array(8);
  new DataView(length.buffer).setBigUint64(0, BigInt(bytes.byteLength), true);
  chunks.push(length, bytes);
};

/** Mirrors `media_asset_authoritative_shape` exactly, including little-endian lengths. */
const authoritativeShapeFingerprint = async (
  descriptor: MediaAssetAuthoritativeCommitDescriptor,
): Promise<string> => {
  const chunks: Uint8Array[] = [new TextEncoder().encode("syndocal-media-authoritative-shape-v1")];
  appendLengthPrefixedUtf8(chunks, descriptor.commandKind);
  for (const field of descriptor.shapeFields) appendLengthPrefixedUtf8(chunks, field);
  const totalLength = chunks.reduce((total, chunk) => total + chunk.byteLength, 0);
  const input = new Uint8Array(totalLength);
  let offset = 0;
  for (const chunk of chunks) {
    input.set(chunk, offset);
    offset += chunk.byteLength;
  }
  if (!globalThis.crypto?.subtle) {
    throw new Error("Secure media operation fingerprints are unavailable in this WebView.");
  }
  const digest = new Uint8Array(await globalThis.crypto.subtle.digest("SHA-256", input));
  return Array.from(digest, (byte) => byte.toString(16).padStart(2, "0")).join("");
};

/**
 * `start_media_asset_operation` reserves the exact identity before the first
 * local-media hash begins. Cancellation is submitted at every phase, including
 * an in-flight authoritative commit. The backend's Cancelable -> Admitted CAS
 * decides whether cancel wins; once admission wins, the UI resolves the exact
 * terminal receipt instead of reporting a false cancellation.
 */
const installKnownOperationCancellation = (
  invoke: MediaAssetInvoke,
  ownerId: string,
  signal: AbortSignal | undefined,
  operation: () => MediaAssetOperationIdentity | null,
) => {
  let cancellation: Promise<boolean | null> | null = null;
  const cancelKnown = () => {
    const known = operation();
    if (!known) return Promise.resolve(null);
    if (cancellation) return cancellation;
    cancellation = invoke<boolean>("cancel_media_asset_operation", {
      requestId: known.requestId,
      operationGeneration: known.operationGeneration,
      ownerId,
    }).catch(() => null);
    return cancellation;
  };
  const onAbort = () => { void cancelKnown(); };
  signal?.addEventListener("abort", onAbort, { once: true });
  return {
    cancelKnown,
    cancelKnownAfterError: cancelKnown,
    dispose: () => signal?.removeEventListener("abort", onAbort),
  };
};

const authoritativeTerminalResponse = <C extends MediaAssetAuthoritativeCommitCommand>(
  command: C,
  descriptor: MediaAssetAuthoritativeCommitDescriptor,
  expectedShapeFingerprint: string,
  envelope: MediaAssetAuthoritativeTerminalEnvelope,
): MediaAssetAuthoritativeCommitResponseMap[C] => {
  if (envelope.command_kind !== descriptor.commandKind
    || envelope.shape_fingerprint !== expectedShapeFingerprint) {
    throw new Error("The terminal media receipt belongs to a different operation shape.");
  }
  if (envelope.terminal.kind === "failure") throw new Error(envelope.terminal.result.message);
  if (envelope.terminal.kind !== descriptor.terminalKind) {
    throw new Error(`The terminal media receipt has unexpected kind ${envelope.terminal.kind}.`);
  }
  const response = envelope.terminal.result as MediaAssetAuthoritativeCommitResponseMap[C];
  copyAuthoritativeApplicationCurrent(envelope, response);
  return response;
};

const waitForTerminalReceipt = async <C extends MediaAssetAuthoritativeCommitCommand>(
  invoke: MediaAssetInvoke,
  command: C,
  descriptor: MediaAssetAuthoritativeCommitDescriptor,
  expectedShapeFingerprint: string,
  token: number,
  operation: MediaAssetOperationIdentity,
  authority: MediaAssetOperationAuthority,
  ownerId: string,
): Promise<MediaAssetAuthoritativeCommitResponseMap[C] | null> => {
  for (let attempt = 0; attempt < 4; attempt += 1) {
    const envelope = await invoke<MediaAssetAuthoritativeTerminalEnvelope | null>(
      "get_media_asset_operation_terminal_result",
      {
        preparedToken: token,
        requestId: operation.requestId,
        operationGeneration: operation.operationGeneration,
        expectedEpoch: authority.projectEpoch,
        expectedRevision: authority.projectRevision,
        expectedCheckpointHash: authority.checkpointHash,
        ownerId,
      },
    ).catch(() => null);
    if (envelope) {
      return authoritativeTerminalResponse(command, descriptor, expectedShapeFingerprint, envelope);
    }
    if (attempt < 3) await new Promise((resolve) => globalThis.setTimeout(resolve, 25));
  }
  return null;
};

const invokeAuthoritativeCommit = async <C extends MediaAssetAuthoritativeCommitCommand>(
  invoke: MediaAssetInvoke,
  command: C,
  commandArgs: Record<string, unknown>,
  token: number,
  operation: MediaAssetOperationIdentity,
  authority: MediaAssetOperationAuthority,
  ownerId: string,
  signal: AbortSignal | undefined,
  cancellation: { cancelKnown: () => Promise<boolean | null> },
): Promise<MediaAssetAuthoritativeCommitResponseMap[C]> => {
  const descriptor = authoritativeCommitDescriptor(command, commandArgs);
  const expectedShapeFingerprint = await authoritativeShapeFingerprint(descriptor);
  const args = {
    ...commandArgs,
    ...(command === "commit_prepared_media_asset_relink_authoritative"
      ? { preparedRelinkToken: token }
      : { preparedImportToken: token }),
    requestId: operation.requestId,
    operationGeneration: operation.operationGeneration,
    expectedEpoch: authority.projectEpoch,
    expectedRevision: authority.projectRevision,
    expectedCheckpointHash: authority.checkpointHash,
    ownerId,
    __expectedProjectEpoch: authority.projectEpoch,
    __shouldAbortProjectMutation: () => signal?.aborted === true,
  };
  let firstError: unknown;
  try {
    return await invoke<MediaAssetAuthoritativeCommitResponseMap[C]>(command, args);
  } catch (error) {
    firstError = error;
  }

  const cancellationWon = signal?.aborted ? await cancellation.cancelKnown() : null;
  if (cancellationWon === true) throw abortError();

  // An exact retry is safe: the backend single-flight/receipt key excludes the
  // old renderer ticket and returns the already-published terminal result.
  try {
    return await invoke<MediaAssetAuthoritativeCommitResponseMap[C]>(command, args);
  } catch {
    const terminal = await waitForTerminalReceipt(
      invoke,
      command,
      descriptor,
      expectedShapeFingerprint,
      token,
      operation,
      authority,
      ownerId,
    );
    if (terminal) return terminal;
  }
  throw firstError;
};

const committedValue = <C extends MediaAssetAuthoritativeCommitCommand>(
  command: C,
  response: MediaAssetAuthoritativeCommitResponseMap[C],
): MediaAssetAuthoritativeCommittedValueMap[C] => {
  switch (command) {
    case "commit_prepared_media_assets_authoritative":
      return (response as MediaAssetAuthoritativeImportResult).report as MediaAssetAuthoritativeCommittedValueMap[C];
    case "commit_prepared_media_asset_relink_authoritative":
      return (response as MediaAssetAuthoritativeRelinkResult).report as MediaAssetAuthoritativeCommittedValueMap[C];
    case "commit_prepared_video_file_layer_authoritative":
    case "commit_prepared_still_image_layer_authoritative": {
      const layerIds = (response as MediaAssetAuthoritativeLayerResult).layer_ids;
      if (layerIds.length !== 1) throw new Error("Authoritative single-layer commit returned an unexpected layer count.");
      return layerIds[0] as MediaAssetAuthoritativeCommittedValueMap[C];
    }
    case "commit_prepared_local_media_layers_authoritative":
      return (response as MediaAssetAuthoritativeLayerResult).layer_ids as MediaAssetAuthoritativeCommittedValueMap[C];
    case "commit_prepared_bootstrap_vj_show_authoritative":
      return (response as MediaAssetAuthoritativeBootstrapResult).setup as MediaAssetAuthoritativeCommittedValueMap[C];
  }
};

/**
 * Reserve an operation before the first local-media hash, then run Reserved
 * Prepare and full-byte Finalize outside history before one backend-owned
 * authoritative Commit. App still owns mapping flush/operator classification,
 * but it does not open a renderer project transaction for this command.
 */
export async function prepareFinalizeAndCommitMediaAssets<C extends MediaAssetAuthoritativeCommitCommand>(
  options: PreparedMediaAssetCommitOptions<C>,
): Promise<PreparedMediaAssetCommitResult<C>> {
  const requestId = options.requestId ?? allocateMediaAssetRequestId();
  let knownOperation: MediaAssetOperationIdentity | null = null;
  const cancellation = installKnownOperationCancellation(
    options.invoke,
    options.ownerId,
    options.signal,
    () => knownOperation,
  );
  try {
    assertOperationNotAborted(options.signal);
    const started = await options.invoke<MediaAssetOperationStartReport>("start_media_asset_operation", {
      requestId,
      expectedEpoch: options.expectedEpoch,
      ownerId: options.ownerId,
    });
    const startedOperation = knownOperationFromReport(started, "Start");
    // If Start's echoed request ID is malformed-but-valid, it is the only
    // identity that can name the backend reservation for cleanup. Reject it
    // after retaining that exact reported pair for the catch-path cancel.
    knownOperation = startedOperation;
    if (startedOperation.requestId !== requestId) {
      throw new Error("Start returned a different media operation request_id; no project changes were made.");
    }
    const startedAuthority = authorityFromReport(started, "Start");
    if (startedAuthority.projectEpoch !== options.expectedEpoch) {
      throw new Error("Start returned a different project epoch; no project changes were made.");
    }
    assertOperationNotAborted(options.signal);

    const prepared = await options.invoke<MediaAssetImportReport>("prepare_reserved_media_assets", {
      requestId: knownOperation.requestId,
      operationGeneration: knownOperation.operationGeneration,
      kind: options.kind,
      paths: options.paths,
      expectedEpoch: options.expectedEpoch,
      ownerId: options.ownerId,
    });
    assertReportContinuity("Reserved Prepare", prepared, knownOperation, startedAuthority);
    assertOperationNotAborted(options.signal);

    if (options.requireAllPrepared
      && (prepared.failed > 0 || prepared.skipped > 0 || prepared.prepared !== options.paths.length)) {
      await cancellation.cancelKnownAfterError();
      throw new Error("First-run VJ setup requires every selected media file to prepare successfully; no project changes were made.");
    }

    const token = prepared.prepared_import_token;
    // A report containing only skipped/failed input is truthful completion and
    // intentionally creates neither a catalog mutation nor an empty history entry.
    if (token === null || token === undefined) {
      return {
        report: prepared,
        committed: null,
        applicationCurrent: true,
        terminalAuthority: projectAuthorityFromOperation(startedAuthority),
      };
    }

    const finalized = await options.invoke<MediaAssetImportReport>("finalize_prepared_media_assets", {
      preparedImportToken: token,
      requestId: knownOperation.requestId,
      operationGeneration: knownOperation.operationGeneration,
      expectedEpoch: options.expectedEpoch,
      ownerId: options.ownerId,
    });
    assertReportContinuity("Finalize", finalized, knownOperation, startedAuthority);
    assertOperationNotAborted(options.signal);

    const response = await invokeAuthoritativeCommit(
      options.invoke,
      options.commitCommand,
      options.commitArgs ?? {},
      token,
      knownOperation,
      startedAuthority,
      options.ownerId,
      options.signal,
      cancellation,
    );
    const report = options.commitCommand === "commit_prepared_media_assets_authoritative"
      ? (response as MediaAssetAuthoritativeImportResult).report
      : finalized;
    return {
      report,
      committed: committedValue(options.commitCommand, response),
      applicationCurrent: authoritativeApplicationCurrent(response),
      terminalAuthority: projectAuthorityFromResponse(response),
    };
  } catch (error) {
    await cancellation.cancelKnownAfterError();
    throw error;
  } finally {
    cancellation.dispose();
  }
}

/**
 * Relinking follows the same long-I/O/short-ticket boundary. A typed outcome
 * without a server token is a successful read-only decision, never a mutation.
 * Start reserves the exact identity before Reserved Relink Prepare hashes the
 * replacement, so the existing AbortController can cancel its first chunk.
 */
export async function prepareFinalizeAndCommitMediaAssetRelink(
  options: PreparedMediaAssetRelinkOptions,
): Promise<PreparedMediaAssetRelinkResult> {
  const requestId = options.requestId ?? allocateMediaAssetRequestId();
  let knownOperation: MediaAssetOperationIdentity | null = null;
  const cancellation = installKnownOperationCancellation(
    options.invoke,
    options.ownerId,
    options.signal,
    () => knownOperation,
  );
  try {
    assertOperationNotAborted(options.signal);
    const started = await options.invoke<MediaAssetOperationStartReport>("start_media_asset_operation", {
      requestId,
      expectedEpoch: options.expectedEpoch,
      ownerId: options.ownerId,
    });
    const startedOperation = knownOperationFromReport(started, "Relink Start");
    // A valid-but-unexpected echo is still the only backend operation identity
    // the catch path can name for exact cleanup.
    knownOperation = startedOperation;
    if (startedOperation.requestId !== requestId) {
      throw new Error("Relink Start returned a different media operation request_id; no project changes were made.");
    }
    const startedAuthority = authorityFromReport(started, "Relink Start");
    if (startedAuthority.projectEpoch !== options.expectedEpoch) {
      throw new Error("Relink Start returned a different project epoch; no project changes were made.");
    }
    assertOperationNotAborted(options.signal);

    const prepared = await options.invoke<MediaAssetRelinkPrepareReport>("prepare_reserved_media_asset_relink", {
      requestId: knownOperation.requestId,
      operationGeneration: knownOperation.operationGeneration,
      assetId: options.assetId,
      replacementPath: options.replacementPath,
      policy: options.policy,
      expectedEpoch: options.expectedEpoch,
      ownerId: options.ownerId,
    });
    assertReportContinuity("Reserved Relink Prepare", prepared, knownOperation, startedAuthority);
    assertOperationNotAborted(options.signal);
    const token = prepared.prepared_relink_token;
    if (token === null || token === undefined) {
      return {
        report: prepared,
        outcome: prepared.outcome ?? null,
        committed: false,
        applicationCurrent: true,
        terminalAuthority: projectAuthorityFromOperation(startedAuthority),
      };
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
    assertReportContinuity("Relink Finalize", finalized, knownOperation, startedAuthority);
    assertOperationNotAborted(options.signal);

    const response = await invokeAuthoritativeCommit(
      options.invoke,
      "commit_prepared_media_asset_relink_authoritative",
      {},
      token,
      knownOperation,
      startedAuthority,
      options.ownerId,
      options.signal,
      cancellation,
    );
    const committed = response.report;
    return {
      report: committed,
      outcome: committed.outcome,
      committed: true,
      applicationCurrent: authoritativeApplicationCurrent(response),
      terminalAuthority: projectAuthorityFromResponse(response),
    };
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
