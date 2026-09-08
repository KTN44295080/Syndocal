# Snapshot live publication deduplication — 2026-09-08

## Scope

Base source: `1f2e78404254db80504c2e4e24068727150b1982` (`main`).

This checkpoint removes one duplicate frontend live publication from
`applyEngineSnapshotSyncResponse`. Full responses and UI-synchronized deltas
now pass through `applyAcceptedEngineSnapshot`, which already publishes the
live DMX and fixture projections. Live-only deltas retain their direct live
publication path. The snapshot ingress guard, project read identity,
watermark validation, revision, editor synchronization and rejection policy
are unchanged.

No native command, IPC schema, persistence schema, output ownership, or
physical-output behavior was changed.

## Verification

- `pnpm --dir app run check:snapshot-live-publication`: PASS. The checker
  executes the actual App sync arrow and the actual live fixture projection
  with explicit harness ports. It covers full, UI-synchronized delta,
  live-only delta, rejected ingress, and two successive responses in one
  session. Full/UI paths each publish live DMX and fixtures once; live-only
  publishes once without editor sync; rejection publishes neither nor advances
  revision.
- `pnpm --dir app run check:snapshot-runtime-watermark`: PASS.
- `node app/scripts/check-media-asset-authority.mjs`: PASS. Existing media
  authority and empty-catalog normalization checks remained intact.
- `pnpm --dir app exec tsc --noEmit`: PASS.
- `pnpm --dir app build`: PASS. Vite production build completed without
  compiler or bundler warnings.
- `pnpm --dir app run check:release`: PASS. Native admission inventory was
  516 commands with the existing frozen hash; existing thumbnail, authority,
  output, ASIO, timeline, watermark, bootstrap, routing, observation and
  camera checks passed.
- `git diff --check`: PASS.

Independent review by GPT-6 Astra (low), review-only: ACCEPT. The review
confirmed exactly-once live publication, live-only editor bypass, rejection
side-effect freedom, revision ordering and unchanged authority ingress. The
review also caught and required correction of an initially misleading
successive-response checker; the final checker uses one persistent harness
session.

## Windows native gate

The exact MSVC 14.44.35207 linker pin and `where.exe link.exe` check were used
before:

```text
pnpm --dir app tauri build --no-bundle
```

The build exited 0. The exact artifact was launched from its `target/release`
working directory and verified as one responsive, maximized `Syndocal` window.

| Artifact | Value |
| --- | --- |
| Path | `target/release/syndocal.exe` |
| Version | `1.2.0-alpha.69` |
| Bytes | `64,700,928` |
| SHA-256 | `CC984A75AB399E80C373C6B9133C5FFA6661ABA6878B4D17EF606A063F6523B3` |
| Window | 1 titled `Syndocal`, responsive, maximized |
| Physical output operations | 0 |

Evidence: `target/qa/snapshot-live-publication-20260908-01/`.

The first ad-hoc launch observer attempt did not establish evidence because
its wrapper used the wrong working-directory/observer setup; it was not
counted as a product result. The documented exact-artifact launch contract
was then rerun successfully. The final process was terminated by exact
executable path; no Syndocal process or debug listener was retained.

## Boundaries

This closes only the duplicate live-publication sub-unit. It does not claim
whole-show snapshot CPU/FPS, native lock-wait, deep-delta or serialization
performance, physical output, ASIO/NDI/DMX hardware acceptance, Mac
execution, signing, publication, venue acceptance, or product-wide
completion. The previously completed real-file-missing → UI Retry → recovery
test was not rerun.
