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

## Boundary

This checkpoint does not close `MEDIA-DERIVED-001`. Cache capacity/eviction,
waveform/proxy/analysis pipelines, cold-disk and real-show performance, native
browser rendering, and physical/external acceptance remain open.
