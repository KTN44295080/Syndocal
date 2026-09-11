# Media derived authority audit — 2026-09-08

## Scope

This checkpoint audits the existing media-derived UI authority and thumbnail
controller contracts on current `main`. It repairs only stale source markers
in an existing checker; product/runtime source is unchanged. The previously
completed real-file-missing → UI Retry → recovery test is not rerun.

- Main base before this checkpoint: `e985e57e7e35275583e59283e0de9bb48d8764b1`
- Product source changes: none
- Changed file: `app/scripts/check-vj-media-import-access.mjs`
- Physical media, browser rendering, cache/proxy/waveform performance, and
  native real-file recovery were not exercised

## Checker repair

The checker searched for the pre-cancellation call shape
`loadVideoLayerThumbnail(source.id)` / `loadMediaAssetThumbnail(source.id)` and
for a callback without its `AbortSignal`. Current production code correctly
passes `signal` to both reads and rejects retired batches through
`signal.aborted`. The checker now matches that current contract, including the
multi-line `readThumbnailWithRetry` call and `(isCurrent, signal)` batch
callback. No assertion was removed or made less strict.

## Verification

| Command | Result |
| --- | --- |
| `node app/scripts/check-media-asset-authority.mjs` | PASS — E/R/H authority, cancellation CAS, reply-loss/query, reset, catalog normalization, and UI-fence contracts |
| `node app/scripts/check-vj-media-import-access.mjs` | PASS — includes the production thumbnail controller regression and guarded media-import/VJ source contract |
| nested thumbnail controller regression | PASS — 100 unchanged-array/equivalent-authority publications, 50-change burst bounded to one read per lane, 13-asset successor, bounded retry, terminal errors, stale retry retirement, explicit retry/cache reuse, busy ownership, and reset/dispose |
| `git diff --check` | PASS |

The VJ checker explicitly reports that it uses source/policy fixtures and does
not establish browser or native rendered reachability. Its explicit recovery
fixture is a deterministic transient-retry contract, not the separately
reserved real-file-missing/native UI Retry acceptance.

## Current-main media authority and checker revalidation — 2026-09-12

The bounded MEDIA-DERIVED software slice was rerun against current `main` at
source HEAD `612c329bdee3600e71d21a0c06634ad8cb4d8432`; `origin/main` matched
before this checker repair. Product source was unchanged.

| Check | Result |
| --- | --- |
| `pnpm.cmd --dir app run check:media-thumbnails` | PASS — bounded retry/terminal rejection, stale retirement, explicit recovery/cache, lanes, reset/disposal, and view bindings |
| `pnpm.cmd --dir app run check:media-asset-operations` | PASS — phase, exact cancellation, idempotent release, owner cleanup |
| `pnpm.cmd --dir app run check:media-asset-authority` | PASS — phase order, reply-loss/query, cancellation CAS, authority continuity, operator classification, paired application, empty-catalog normalization |
| `pnpm.cmd --dir app run check:native-thumbnail-request` | PASS — normal success, exact abort, stale-result, cancellation failure, malformed/foreign ticket rejection |

The first direct run of `node app/scripts/check-vj-media-import-access.mjs`
failed because its checker expected the old implementation shape with
`controller.abort()` and the phase guard inline in `App.tsx`. The current
product keeps those responsibilities in
`mediaAssetOperationController.ts`; `App.tsx` owns the controller and wires
its `cancel`. The checker was repaired to assert that ownership boundary,
including the real `AbortController.abort()` and the aborted phase-update
guard, without removing or weakening the contract. After the repair the same
checker passed, including its nested thumbnail controller regression and the
full source/policy fixture set. The initial failure was checker drift, not a
product behavior failure.

`MEDIA-DERIVED-001` remains Open for cache capacity/eviction,
waveform/proxy/analysis pipelines, cold-disk and real-show performance,
browser/native rendered reachability, the still-unproven MP4 missing-file
recovery case, physical/external acceptance, and the broader product gate.

## Boundary

This checkpoint does not close `MEDIA-DERIVED-001`. Cache capacity/eviction,
waveform/proxy/analysis pipelines, cold-disk and real-show performance, native
browser rendering, and physical/external acceptance remain open.
