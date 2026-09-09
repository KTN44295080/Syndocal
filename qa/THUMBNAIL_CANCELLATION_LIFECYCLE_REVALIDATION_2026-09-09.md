# Thumbnail cancellation and lifecycle revalidation — 2026-09-09

This is a focused revalidation checkpoint for the current `main` tree. It
does not add product behavior, re-run the completed missing-file recovery
acceptance, or merge the older thumbnail/channel candidates. The purpose is
to verify the existing cancellation and worker-lifecycle contract before
moving to the next handoff unit.

## Source and scope

- Source: `main` at `b571ff0d4183dfa6a1ff6b6cdd0a3af0a8fcb573`.
- Remote comparison: `origin/main` matched before validation.
- Product source changes: none.
- Native/hardware scope: deterministic source checks and Windows native unit
  tests only; no physical output or external client was enabled.
- The previously completed real-file missing → Retry → recovery acceptance was
  not repeated.

## Contract review

The current implementation preserves the required ownership boundaries:

- `nativeThumbnailRequest.ts` announces and validates the exact native ticket,
  requests cancellation only for the matching ticket, and rejects an
  unacknowledged cancellation instead of treating it as success.
- `native_thumbnail_dispatch.rs` keeps the worker admission guard until the
  blocking operation has returned, so dropping an async waiter cannot free a
  slot while the worker is still running.
- `native_thumbnail_work.rs` scopes cancellation by window, lane, and request
  ID; stale, malformed, foreign, and late results are rejected.
- `createLatestThumbnailBatch.ts` and
  `createMediaThumbnailController.ts` keep one active read per lane, retain at
  most the latest successor, and retire results across authority changes,
  reset, and disposal.
- The central Tauri admission inventory includes
  `cancel_native_thumbnail_request_v1` as a `RecoveryMaintenance` route.

No source change was required by this review.

## Evidence

All commands were run from the repository root on 2026-09-09:

| Check | Result | Boundary covered |
| --- | --- | --- |
| `pnpm.cmd --dir app run check:native-thumbnail-request` | PASS | normal completion, exact abort cancellation, stale-result rejection, cancellation failure, malformed/foreign ticket rejection |
| `pnpm.cmd --dir app run check:media-thumbnails` | PASS | 100 unchanged-array and 100 equivalent-authority publications, independent lanes, authority/reset/disposal, 50-change burst, one active worker per lane, bounded retry, cache reuse and visible retry eligibility |
| `node app/scripts/check-tauri-admission-inventory.mjs` | PASS — 516 commands, SHA-256 `5120894f36feb82ac58fffd4db20739d80ae1b1a1c556fc95838a1708cbb8eea`, 18 negative fixtures rejected | exact route inventory and fail-closed admission |
| `node app/scripts/check-frontend-command-routing.mjs` | PASS — 133 renderer, 31 server-authoritative, 28 raw, 464 facade dispatches | frontend routing ownership and raw-dispatch classification |
| `cargo test -p syndocal --release --locked native_thumbnail -- --test-threads=1` after exact MSVC initialization and linker pin | PASS — 23 passed, 0 failed, 0 ignored, 1795 filtered out | dispatch admission/reaping, real PNG/MP4 pixels, private-copy isolation, cancellation, lane/request scoping, malformed tickets and late-result rejection |
| `git diff --check` | PASS | whitespace integrity |

The native command used `vcvars64.bat -vcvars_ver=14.44` and the exact
Build Tools 14.44.35207 linker; `where.exe link.exe` resolved that linker as
the first result.

## Disposition and boundaries

This closes the current-main cancellation/lifecycle software revalidation
unit only. It does not close `MEDIA-DERIVED-001`, and does not prove GUI
stop-latency, hard cancellation deadlines, application restart, saved-project
reload, cold-disk performance, ASIO/NDI, physical output, Mac, signing,
publication, venue, or product-wide completion. The broader GUI/native
acceptance remains open under the handoff.
