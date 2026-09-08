# Control-plane source inventory count repair — 2026-09-09

## Scope

This checkpoint repairs stale test expectations in the existing control-plane
inventory. It does not add a product route or change runtime admission behavior.

The current source already contains the cancellable native-thumbnail route
`cancel_native_thumbnail_request_v1`, and the frontend manifest already contains
457 invoke entries. The Rust inventory tests still expected the pre-cancellation
456-entry frontend manifest and its derived totals.

Source base before this checkpoint: `46d6f25f1913189777f38122211e80216a85a628`.

## Reproduced baseline

With the exact Windows native test environment (MSVC 14.44.35207 Build Tools
linker pinned and first in `where.exe link.exe`),

```text
cargo test -p syndocal --release --locked control_plane -- --test-threads=1
```

initially ran 76 tests with 73 passing and 3 failures. The failures were fixed
count mismatches in `control_plane.rs`; the runtime registry contained 1,575
operations while the stale expectations still derived 1,573.

## Change

Only `app/src-tauri/src/control_plane.rs` test expectations were synchronized
with the existing source-of-truth manifests and registry:

- frontend invoke source count: `456` → `457`;
- legacy operation/source inventory: `1,573` → `1,575`;
- canonical source inventory including 30 app and 3 project-file keyboard
  sources: `1,606` → `1,608`;
- canonical unclassified-source derived count: `1,098` → `1,099`.

The exact-set, route-admission SHA/count, frontend manifest ordering, keyboard
manifest, and negative rejection assertions remain unchanged.

## Validation

The repaired release control-plane suite passed:

```text
76 passed; 0 failed; 0 ignored; 1739 filtered out
```

Evidence log: `target/qa/control-plane-current-20260909-04/control-plane-tests.log`.

The release static/metadata gate also passed:

```text
pnpm.cmd --dir app run check:release
```

This included the exact 516-command native admission inventory with 18 negative
fixtures, media-thumbnail retry/controller checks, cancellable native-thumbnail
request checks, output/safety/ASIO/timeline/snapshot/runtime checks, and release
metadata validation for `Syndocal 1.2.0-alpha.69`.

## Boundary

The real file-missing → UI Retry → restore → recovery flow was not executed in
this checkpoint. No helper moved or renamed a test PNG, so that acceptance
remains explicitly unclaimed. No physical output, release publication, signing,
notarization, Mac acceptance, or incomplete Channel API work is included.
