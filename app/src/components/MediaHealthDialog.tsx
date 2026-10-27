import { For, Show, createEffect, createMemo, createSignal } from "solid-js";
import type { MediaAssetAvailability, MediaAssetId, MediaAssetSummary } from "../types";
import { ndiInputHealthTitle, type NdiInputHealthRow } from "../ndiInputHealth";
import {
  MEDIA_HEALTH_ATTENTION_STATES,
  buildMediaHealthRows,
  formatMediaByteSize,
  groupMediaHealthRows,
  summarizeMediaHealth,
  type MediaHealthRow,
  type MediaHealthState,
} from "../mediaHealthModel";
import {
  mediaRelinkApplySummary,
  planFolderRelink,
  type MediaRelinkApplyRequest,
  type MediaRelinkApplyResult,
  type MediaRelinkFolderScan,
  type MediaRelinkPlan,
} from "../mediaRelinkPlan";
import "./MediaHealthDialog.css";

export interface MediaHealthDialogProps {
  assets: MediaAssetSummary[];
  ndiInputs?: readonly NdiInputHealthRow[];
  availabilityById: Record<number, MediaAssetAvailability>;
  /** Epoch milliseconds of each asset's last verification in this session. */
  checkedAtById?: Record<number, number>;
  backendAvailable: boolean;
  /** True while any Media Library operation is running: new ones wait. */
  busy: boolean;
  onVerify: (assetIds: MediaAssetId[]) => void | Promise<void>;
  /** Resolves true when the asset was relinked. */
  onRelink: (assetId: MediaAssetId) => boolean | void | Promise<boolean | void>;
  /** Folder relink is offered only when both folder callbacks are provided. */
  onScanRelinkFolder?: () => Promise<MediaRelinkFolderScan | null>;
  onApplyRelinkRequests?: (
    requests: MediaRelinkApplyRequest[],
    onProgress?: (completed: number, total: number) => void,
  ) => Promise<MediaRelinkApplyResult | null>;
  ref?: (element: HTMLDialogElement) => void;
  onClose?: () => void;
}

const stateTitle: Record<MediaHealthState, string> = {
  missing: "Missing",
  unreadable: "Unreadable",
  hash_mismatch: "Hash mismatch",
  unverified: "Not verified",
  available: "Available",
};

const stateExplanation: Record<MediaHealthState, string> = {
  missing: "The file is not at the path the project recorded.",
  unreadable: "The file is there but could not be read.",
  hash_mismatch: "The file at this path has different content than the project recorded. Relink the right file, or adopt a different one after confirming.",
  unverified: "Not hash-checked on this machine yet. Use Verify All.",
  available: "Present and matching the recorded content.",
};

const checkedTime = (ms: number) => new Date(ms).toLocaleTimeString([], { hour12: false });

/**
 * One place to see which media files are broken and to fix them. It reuses the
 * existing Verify and Relink operations (and their fences); nothing here
 * mutates the project directly.
 */
export function MediaHealthDialog(props: MediaHealthDialogProps) {
  const rows = createMemo(() =>
    buildMediaHealthRows(props.assets, props.availabilityById, props.checkedAtById ?? {}));
  const groups = createMemo(() => groupMediaHealthRows(rows()));
  const summary = createMemo(() => summarizeMediaHealth(props.assets, props.availabilityById));
  const missingAssets = createMemo(() => {
    const ids = new Set(rows().filter((row) => row.state === "missing").map((row) => row.assetId));
    return props.assets.filter((asset) => ids.has(asset.id));
  });
  const [folderBusy, setFolderBusy] = createSignal(false);
  const [plan, setPlan] = createSignal<MediaRelinkPlan | null>(null);
  const [selected, setSelected] = createSignal<ReadonlySet<MediaAssetId>>(new Set());
  const [progress, setProgress] = createSignal<{ done: number; total: number } | null>(null);
  const [report, setReport] = createSignal<MediaRelinkApplyResult | null>(null);
  const [rowBusyId, setRowBusyId] = createSignal<MediaAssetId | null>(null);
  const [stateFilter, setStateFilter] = createSignal<MediaHealthState | "all" | "attention">("all");
  const filteredGroups = createMemo(() => groups().map((group) => ({
    ...group,
    rows: stateFilter() === "all"
      ? group.rows
      : stateFilter() === "attention"
        ? (MEDIA_HEALTH_ATTENTION_STATES.includes(group.state) ? group.rows : [])
        : group.state === stateFilter() ? group.rows : [],
  })).filter((group) => group.rows.length > 0));
  // Bumped whenever the dialog closes so a slow Apply cannot repopulate a closed view.
  let session = 0;
  const disabled = () => !props.backendAvailable || props.busy || folderBusy() || rowBusyId() !== null;

  const chooseFolder = async () => {
    if (disabled() || missingAssets().length === 0) return;
    setFolderBusy(true);
    setReport(null);
    try {
      const scan = await props.onScanRelinkFolder?.() ?? null;
      if (!scan) return;
      const next = planFolderRelink(missingAssets(), scan);
      setPlan(next);
      // Only same-name-and-size matches start selected: a size difference almost
      // always means different content, and applying one hashes the whole file.
      setSelected(new Set(next.proposals.filter((proposal) => proposal.confidence === "exact").map((proposal) => proposal.assetId)));
    } finally {
      setFolderBusy(false);
    }
  };
  const toggleProposal = (assetId: MediaAssetId, checked: boolean) => {
    const next = new Set(selected());
    if (checked) next.add(assetId);
    else next.delete(assetId);
    setSelected(next);
  };
  const cancelPlan = () => {
    setPlan(null);
    setSelected(new Set<MediaAssetId>());
  };
  const applyPlan = async () => {
    const current = plan();
    if (!current || disabled()) return;
    const requests = current.proposals
      .filter((proposal) => selected().has(proposal.assetId))
      .map((proposal): MediaRelinkApplyRequest => ({
        assetId: proposal.assetId,
        label: proposal.label,
        previousPath: proposal.previousPath,
        replacementPath: proposal.replacementPath,
      }));
    if (requests.length === 0) return;
    const applySession = session;
    setFolderBusy(true);
    setProgress({ done: 0, total: requests.length });
    try {
      const result = await props.onApplyRelinkRequests?.(requests, (done, total) => setProgress({ done, total }));
      if (result && applySession === session) {
        setReport(result);
        setPlan(null);
        setSelected(new Set<MediaAssetId>());
        // A relink is verified by construction (the content hash was checked),
        // but availability is cleared by the relink; re-verify so the view
        // shows the real, current state.
        if (result.relinked.length > 0) void props.onVerify(result.relinked);
      } else if (result) {
        if (result.relinked.length > 0) void props.onVerify(result.relinked);
      }
    } finally {
      setProgress(null);
      setFolderBusy(false);
    }
  };
  const relinkOne = async (row: MediaHealthRow) => {
    if (disabled()) return;
    setRowBusyId(row.assetId);
    try {
      const relinked = await props.onRelink(row.assetId);
      if (relinked) void props.onVerify([row.assetId]);
    } finally {
      setRowBusyId(null);
    }
  };
  const chip = (state: MediaHealthState) => summary().counts[state];
  // A reviewed plan is only valid for the exact assets and recorded paths it was
  // made from. If the library changes under an open review (project replaced,
  // an asset relinked elsewhere), drop it rather than apply stale matches.
  createEffect(() => {
    const current = plan();
    if (!current) return;
    const assets = props.assets;
    const stillValid = current.proposals.every((proposal) =>
      assets.some((asset) => asset.id === proposal.assetId && (asset.source.path ?? "") === proposal.previousPath));
    if (!stillValid) {
      setPlan(null);
      setSelected(new Set<MediaAssetId>());
    }
  });

  return (
    <dialog
      ref={props.ref}
      class="mediaHealthDialog"
      data-media-health-dialog
      aria-labelledby="media-health-title"
      onClose={() => { session += 1; setPlan(null); setSelected(new Set<MediaAssetId>()); setReport(null); props.onClose?.(); }}
    >
      <header class="mediaHealthHeader">
        <h2 id="media-health-title">Media health</h2>
        <button type="button" data-media-health-close onClick={(event) => event.currentTarget.closest("dialog")?.close()}>Close</button>
      </header>
      <div class="mediaHealthSummary" data-media-health-summary role="group" aria-label="Media health summary">
        <For each={["missing", "unreadable", "hash_mismatch", "unverified", "available"] as MediaHealthState[]}>
          {(state) => (
            <span
              class={`mediaHealthChip mediaHealthChip-${state}`}
              data-media-health-chip={state}
              data-attention={MEDIA_HEALTH_ATTENTION_STATES.includes(state) && chip(state) > 0 ? "true" : undefined}
            >
              <span>{stateTitle[state]}</span>
              <strong data-no-localize>{chip(state)}</strong>
            </span>
          )}
        </For>
        <Show when={summary().live > 0}>
          <span class="mediaHealthChip" data-media-health-chip="live">
            <span>Live source</span>
            <strong data-no-localize>{summary().live}</strong>
          </span>
        </Show>
      </div>
      <div class="mediaHealthActions">
        <button
          type="button"
          data-media-health-verify-all
          disabled={disabled() || rows().length === 0}
          onClick={() => void props.onVerify(rows().map((row) => row.assetId))}
        >Verify All</button>
        <label class="mediaHealthFilter">
          <span>Show</span>
          <select value={stateFilter()} onChange={(event) => setStateFilter(event.currentTarget.value as MediaHealthState | "all" | "attention")}>
            <option value="all">All</option>
            <option value="attention">Needs attention</option>
            <option value="unverified">Not checked</option>
            <option value="missing">Missing</option>
            <option value="unreadable">Unreadable</option>
            <option value="hash_mismatch">Changed</option>
            <option value="available">Available</option>
          </select>
        </label>
        <Show when={props.onScanRelinkFolder && props.onApplyRelinkRequests}>
          <button
            type="button"
            data-media-health-relink-folder
            disabled={disabled() || missingAssets().length === 0}
            title={missingAssets().length > 0
              ? "Choose a folder; missing files are matched by name and size"
              : summary().counts.unverified > 0 ? "Run Verify All first to find the missing files" : "No file is missing"}
            onClick={() => void chooseFolder()}
          >Relink missing from folder…</button>
        </Show>
        <Show when={progress()}>
          {(current) => (
            <span class="mediaHealthProgress" role="status" data-media-health-progress>
              <span>Relinking</span> <span data-no-localize>{current().done}/{current().total}</span>
            </span>
          )}
        </Show>
      </div>
      <Show when={props.busy}>
        <p class="mediaHealthNote" data-media-health-busy>Another media operation is running. Wait for it or cancel it first.</p>
      </Show>
      <Show when={report()}>
        {(done) => (
          <section class="mediaHealthReport" data-media-health-report role="status">
            <p>{mediaRelinkApplySummary(done())}</p>
            <Show when={done().differentContent.length > 0}>
              <p class="mediaHealthNote">
                These files have different content than the project recorded, so they were not applied. Relink them one at a time to review and confirm adopting the new file:
              </p>
              <ul>
                <For each={done().differentContent}>{(entry) => <li data-no-localize>{entry.label}</li>}</For>
              </ul>
            </Show>
            <Show when={done().failed.length > 0}>
              <ul>
                <For each={done().failed}>{(entry) => <li><strong data-no-localize>{entry.label}</strong>: <span>{entry.message}</span></li>}</For>
              </ul>
            </Show>
          </section>
        )}
      </Show>
      <Show when={plan()}>
        {(current) => (
          <section class="mediaHealthPlan" data-media-relink-plan aria-labelledby="media-relink-plan-title">
            <h3 id="media-relink-plan-title">Proposed matches</h3>
            <p class="mediaHealthPath" data-no-localize>{current().directory}</p>
            <Show when={current().truncated}>
              <p class="mediaHealthNote" role="alert" data-media-relink-truncated>
                Not every file in this folder could be scanned (too many files, too deep, or an unreadable folder), so some matches may be missing. Choose a narrower folder to be sure.
              </p>
            </Show>
            <Show when={current().proposals.length > 0} fallback={<p class="mediaHealthNote" data-media-relink-no-matches>No missing file was found by name in this folder.</p>}>
              <ul class="mediaHealthProposals">
                <For each={current().proposals}>
                  {(proposal) => (
                    <li data-media-relink-proposal={proposal.assetId} data-confidence={proposal.confidence}>
                      <label>
                        <input
                          type="checkbox"
                          checked={selected().has(proposal.assetId)}
                          disabled={folderBusy()}
                          onChange={(event) => toggleProposal(proposal.assetId, event.currentTarget.checked)}
                        />
                        <span class="mediaHealthProposalText">
                          <strong data-no-localize>{proposal.label}</strong>
                          <span class="mediaHealthPath" data-no-localize title={proposal.previousPath}>{proposal.previousPath}</span>
                          <span class="mediaHealthPath" data-no-localize title={proposal.replacementPath}>{proposal.replacementPath}</span>
                        </span>
                      </label>
                      <span class="mediaHealthBadge" data-confidence-badge={proposal.confidence}>
                        {proposal.confidence === "exact" ? "Same name and size" : "Same name, size differs"}
                      </span>
                    </li>
                  )}
                </For>
              </ul>
            </Show>
            <Show when={current().ambiguous.length > 0}>
              <details data-media-relink-ambiguous>
                <summary>
                  <span data-no-localize>{current().ambiguous.length}</span> <span>left out: several files have the same name</span>
                </summary>
                <ul>
                  <For each={current().ambiguous}>
                    {(entry) => <li><strong data-no-localize>{entry.label}</strong> <span data-no-localize>{entry.candidatePaths.length}</span> <span>candidates</span></li>}
                  </For>
                </ul>
              </details>
            </Show>
            <Show when={current().unmatched.length > 0}>
              <details data-media-relink-unmatched>
                <summary>
                  <span data-no-localize>{current().unmatched.length}</span> <span>not found in this folder</span>
                </summary>
                <ul>
                  <For each={current().unmatched}>{(entry) => <li data-no-localize>{entry.label}</li>}</For>
                </ul>
              </details>
            </Show>
            <p class="mediaHealthNote">
              Each file is checked against the project's recorded content before it is linked. Files with different content are skipped; relink those one at a time.
            </p>
            <div class="mediaHealthActions">
              <button
                type="button"
                class="primary"
                data-media-relink-apply
                disabled={disabled() || selected().size === 0}
                onClick={() => void applyPlan()}
              >Apply selected <span data-no-localize>({selected().size})</span></button>
              <button type="button" data-media-relink-cancel disabled={folderBusy()} onClick={cancelPlan}>Cancel</button>
            </div>
          </section>
        )}
      </Show>
      <Show when={(props.ndiInputs?.length ?? 0) > 0}>
        <section class="mediaHealthGroup" data-media-health-ndi aria-labelledby="media-health-ndi-title">
          <h3 id="media-health-ndi-title">NDI inputs</h3>
          <ul class="mediaHealthRows">
            <For each={props.ndiInputs?.map((input) => input.layerId) ?? []}>
              {(layerId) => (
                <Show when={props.ndiInputs?.find((input) => input.layerId === layerId)}>
                  {(input) => (
                    <li class="mediaHealthRow mediaHealthNdiRow" data-media-health-ndi-row={input().layerId} data-ndi-state={input().state}>
                      <div class="mediaHealthRowMain">
                        <strong data-no-localize>{input().label}</strong>
                        <span class="mediaHealthPath" data-no-localize title={input().sourceName}>{input().sourceName}</span>
                        <span class="mediaHealthNote">{input().detail}</span>
                        <Show when={input().diagnostic}>
                          <details>
                            <summary>NDI diagnostic details</summary>
                            <p class="mediaHealthNote" data-no-localize>{input().diagnostic}</p>
                          </details>
                        </Show>
                      </div>
                      <span class="mediaHealthBadge" data-ndi-health-label>{ndiInputHealthTitle[input().state]}</span>
                    </li>
                  )}
                </Show>
              )}
            </For>
          </ul>
        </section>
      </Show>
      <Show when={rows().length > 0} fallback={<p class="mediaHealthEmpty" data-media-health-empty>No local media files in this project.</p>}>
        <For each={filteredGroups()}>
          {(group) => (
            <section class="mediaHealthGroup" data-media-health-group={group.state} aria-labelledby={`media-health-group-${group.state}`}>
              <h3 id={`media-health-group-${group.state}`}>
                <span>{stateTitle[group.state]}</span> <span data-no-localize>({group.rows.length})</span>
              </h3>
              <p class="mediaHealthNote">{stateExplanation[group.state]}</p>
              <ul class="mediaHealthRows">
                <For each={group.rows}>
                  {(row) => (
                    <li class="mediaHealthRow" data-media-health-row={row.assetId} data-state={row.state}>
                      <div class="mediaHealthRowMain">
                        <strong data-no-localize>{row.label}</strong>
                        <span class="mediaHealthPath" data-no-localize title={row.path}>{row.path}</span>
                        <Show when={row.detail}><small class="mediaHealthDetail" data-no-localize>{row.detail}</small></Show>
                      </div>
                      <div class="mediaHealthRowMeta">
                        <span data-media-health-size data-no-localize>{formatMediaByteSize(row.byteSize)}</span>
                        <span data-media-health-checked>
                          <Show when={row.checkedAtMs !== null} fallback={<span>Not checked</span>}>
                            <span>Checked</span> <span data-no-localize>{checkedTime(row.checkedAtMs ?? 0)}</span>
                          </Show>
                        </span>
                      </div>
                      <div class="mediaHealthRowActions">
                        <button
                          type="button"
                          data-media-health-verify={row.assetId}
                          disabled={disabled()}
                          aria-label={`Verify ${row.label} on this machine`}
                          onClick={() => void props.onVerify([row.assetId])}
                        >Verify</button>
                        <Show when={row.state === "missing" || row.state === "unreadable" || row.state === "hash_mismatch"}>
                          <button
                            type="button"
                            data-media-health-relink={row.assetId}
                            disabled={disabled()}
                            aria-busy={rowBusyId() === row.assetId ? "true" : undefined}
                            aria-label={`Relink ${row.label}`}
                            title="Choose a replacement file"
                            onClick={() => void relinkOne(row)}
                          >Relink…</button>
                        </Show>
                      </div>
                    </li>
                  )}
                </For>
              </ul>
            </section>
          )}
        </For>
      </Show>
    </dialog>
  );
}

/** The part of a video control panel's media library the health view needs. */
export interface MediaHealthLibrary {
  assets: MediaAssetSummary[];
  ndiInputs?: readonly NdiInputHealthRow[];
  availabilityById: Record<number, MediaAssetAvailability>;
  checkedAtById?: Record<number, number>;
  backendAvailable: boolean;
  activeOperations: readonly unknown[];
  onVerify: MediaHealthDialogProps["onVerify"];
  onRelink: MediaHealthDialogProps["onRelink"];
  onScanRelinkFolder?: MediaHealthDialogProps["onScanRelinkFolder"];
  onApplyRelinkRequests?: MediaHealthDialogProps["onApplyRelinkRequests"];
}

/**
 * Binds the health view to a panel's media library. Both the Perform header and
 * the Edit Video library mount this, so they show the same state and use the
 * same Verify and Relink operations.
 */
export function MediaLibraryHealthDialog(props: {
  library: MediaHealthLibrary;
  ref?: (element: HTMLDialogElement) => void;
  onClose?: () => void;
}) {
  return (
    <MediaHealthDialog
      ref={props.ref}
      onClose={props.onClose}
      assets={props.library.assets}
      ndiInputs={props.library.ndiInputs}
      availabilityById={props.library.availabilityById}
      checkedAtById={props.library.checkedAtById}
      backendAvailable={props.library.backendAvailable}
      busy={props.library.activeOperations.length > 0}
      onVerify={props.library.onVerify}
      onRelink={props.library.onRelink}
      onScanRelinkFolder={props.library.onScanRelinkFolder}
      onApplyRelinkRequests={props.library.onApplyRelinkRequests}
    />
  );
}
