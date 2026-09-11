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

## Current-main follow-up — `b6889f704a2f5ae4ae9f0df9607d7aee5d421d1c`

The current `main` source has no changes to the thumbnail lifecycle files
listed above since the focused revalidation base. The focused checks were
rerun against the current checkout:

| Check | Result |
| --- | --- |
| `pnpm.cmd --dir app run check:native-thumbnail-request` | PASS — normal success, exact cancellation, stale-result and malformed/foreign ticket rejection |
| `pnpm.cmd --dir app run check:media-thumbnails` | PASS — controller authority/reset/disposal, bounded retry/cache reuse, one active read per lane, and visible retry eligibility |
| `cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 native_thumbnail -- --test-threads=1` | PASS — 23 passed, 0 failed, 0 ignored, 1795 filtered out; Build Tools 14.44.35207 linker was absolute-pinned and first in `where.exe link.exe` |

The current executable was also checked with a fresh, non-overwriting native
WebView probe at
`target/qa/native-thumbnail-cancel-20260910-01/native-thumbnail-cancel.json`:

- executable SHA-256:
  `F9A2DD70959A7C00303EC5D69A0E1CC25E4D5A18C6B98A768CA6A34E46AC385C`;
- one responsive, maximized `Syndocal` window; `Standby`, lighting/video
  denied; zero physical-output operations;
- real WebView `Channel` announcement, exact layer and asset cancellation,
  acknowledged cancellation, and cancelled native result for each lane;
- known `get_snapshot` succeeded and an unknown route was rejected;
- the exact application exited and the debug listener count returned to zero.

This follow-up does not claim a hard GUI stop deadline, interruption of a
synchronous decoder/OS-I/O section, physical output, Mac, signing,
publication, or product-wide completion. The real-file missing → Retry →
recovery evidence was not rerun.

## Current-source signal propagation follow-up — 2026-09-10

The focused controller checker was strengthened without changing product
source. Its deferred loader now records the `AbortSignal` supplied by the
controller and asserts that `reset()` aborts both active lane signals and that
Solid-scope disposal aborts both independent active lane signals. This closes
the evidence gap between the existing batch-level abort contract and the
controller-to-loader seam. The native request checker still covers mapping an
announced exact ticket to the cancellation route and rejecting an unacknowledged
cancellation.

| Check | Result | Boundary covered |
| --- | --- | --- |
| `pnpm.cmd --dir app run check:media-thumbnails` | PASS | controller `reset()`/scope disposal abort the active layer and asset loader signals; existing authority, successor, retry and cache assertions remain active |
| `pnpm.cmd --dir app run check:native-thumbnail-request` | PASS | exact ticket cancellation, cancellation acknowledgement failure, stale-result and malformed/foreign ticket rejection |
| `git diff --check` | PASS | whitespace integrity |

This is still cooperative cancellation evidence. It does not claim hard-stop
latency, interruption of synchronous decoder/OS-I/O/GPU sections, native child
termination, application restart, physical output, Mac, signing, publication,
or product-wide completion. The real-file missing → Retry → recovery evidence
was not rerun.

## Pre-announcement cancellation regression — 2026-09-11

The existing native thumbnail request protocol now has an explicit checker
case for the ordering where the frontend Abort arrives after command dispatch
but before the native start ticket is delivered to JavaScript. The test proves
that no cancellation is sent without a ticket, then the exact announced
lane/request ticket is cancelled once it arrives. The product implementation,
ticket schema, owner binding, worker admission lifetime, and IPC inventory are
unchanged.

`pnpm.cmd --dir app run check:native-thumbnail-request` passed, including the
normal success, exact abort cancellation, pre-announcement abort, stale-result
rejection, cancellation failure, and malformed/foreign ticket cases.
`pnpm.cmd --dir app run check:media-thumbnails` passed, and `git diff --check`
passed. This checker-only regression does not claim hard cancellation latency,
decoder/OS I/O interruption, native hardware, real-file Retry recovery, Mac,
signing, publication, or product-wide completion.

## Current-main lifecycle revalidation — 2026-09-11

The current `main` source was revalidated after the managed output terminal
checkpoint. Product behavior was unchanged by this QA-only update; the
thumbnail lifecycle implementation remains the existing
`mediaAssetOperationController`, `createLatestThumbnailBatch`, and exact native
ticket protocol.

| Check | Result |
| --- | --- |
| `pnpm --dir app run check:media-asset-operations` | PASS — phase, exact cancellation, idempotent release, owner cleanup |
| `pnpm --dir app run check:media-thumbnails` | PASS — bounded lanes, authority/reset/disposal, retry/cache, visible eligibility |
| `pnpm --dir app run check:native-thumbnail-request` | PASS — normal, exact abort, stale, cancellation failure, malformed/foreign tickets |
| `node app/scripts/check-tauri-admission-inventory.mjs` | PASS — 516 commands, 18 negative fixtures rejected |
| `node app/scripts/check-frontend-command-routing.mjs` | PASS — 133 renderer, 31 server, 28 raw, 464 facade |
| `cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 native_thumbnail -- --test-threads=1` | PASS — 23 passed, 0 failed, 0 ignored; 1843 filtered |

The Cargo run initialized the documented MSVC `14.44.35207` environment and
returned the pinned x64 linker first from `where.exe link.exe`. The tests cover
worker admission/reaping, real PNG/MP4 pixels, private-copy isolation,
cancellation and lane/request scoping, malformed tickets, late-result
rejection, renderer wait expiry, poisoned-lock rejection, and worker unwind
cleanup.

This is current-main software/lifecycle evidence only. It does not claim a
hard stop deadline for synchronous decoder/OS-I/O/GPU work, native child
termination, application restart, saved-project reload, physical output,
device acceptance, Mac, signing, publication, venue acceptance, or
product-wide completion. The real-file PNG recovery is recorded separately;
MP4 missing-file recovery remains unclaimed.
