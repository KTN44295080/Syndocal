# Live video sources checkpoint — 2026-09-13

This checkpoint closes `VIDEO-LIVE-SOURCES-001` for the supported current-source
software slice. Live input routes now have a versioned, stable identity and an
observable runtime state/generation. Source faults are surfaced through the
transport status contract and the UI does not continue to present a faulty
input as Active.

## Implemented contract verified

- Camera, screen-capture, NDI, Spout, and Syphon input routes use the same
  length-delimited `v1` identity key derived from the canonical backend and the
  trimmed endpoint name. Empty or control-character identities fail closed.
- Input route state is explicit: `Disabled`, `Unavailable`, `Ready`,
  `Starting`, `Live`, `Fault`, or `Retiring`. Source generations are retained
  by stable identity so replacement and stale-worker retirement are observable
  rather than silently reusing a prior worker.
- Start/admission/teardown failures are retained in the runtime status. Camera
  capture faults, NDI receive faults, and Spout receive faults are reported
  without draining the diagnostic state. A source fault removes the route from
  the UI's Active/Started/Kept rows while preserving a visible fault row.
- The existing output-ownership fence remains the lifecycle boundary: a new
  route is admitted only after the prior route is stopped, and a failed stop is
  represented as `Retiring`/stop-failed rather than replaced silently.
- The frontend types, status panels, controller message, and Tauri status
  response carry live-source states and source faults additively. Legacy
  capture-fault data remains visible through its existing diagnostic surface.

## Verification

The browser plugin was not available. The existing focused frontend contracts
and the production build were run; no browser-plugin or physical-device claim
is made. The Windows native gate used the repository procedure with
`vcvars64.bat -vcvars_ver=14.44`, the pinned Build Tools `14.44.35207` x64
linker, and that exact linker first in `where.exe link.exe`.

| Check | Result |
| --- | --- |
| `cargo test -p video --release --locked -j 1 live_source -- --nocapture --test-threads=1` | PASS — 2 passed, 0 failed |
| `cargo test -p video --release --locked -j 1 external_video -- --nocapture --test-threads=1` | PASS — 5 passed, 0 failed |
| `cargo check --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1` | PASS |
| `cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 "external_video" -- --nocapture --test-threads=1` | PASS — 8 passed, 0 failed |
| `pnpm.cmd --dir app run check:camera-input` | PASS |
| `pnpm.cmd --dir app run check:video-output-window-runtime` | PASS |
| `pnpm.cmd --dir app run check:video-output-window-observation` | PASS |
| `node app/scripts/check-timeline-source-shelf-contract.mjs` | PASS |
| `pnpm.cmd --dir app build` | PASS — TypeScript/Vite build |
| `pnpm.cmd --dir app tauri build --no-bundle` | PASS — Windows `target/release/syndocal.exe` |
| Exact release executable process smoke | PASS — one exact-path `Syndocal` process, nonzero main-window handle, title `Syndocal`, responsive; verification process then exited |
| `git diff --check` | PASS before commit |

Native artifact identity for this checkpoint:

`target/release/syndocal.exe` SHA-256
`9C8E5B0635101FFF2FEA25EDC51C04004EF1A6CBBE5C85A58DECFBE1DC52DD75`

## Boundary

This closes the current-source identity, availability-state, permission/fault
reporting seam, receive-fault retention, and old-worker retirement software
contract for the source kinds currently modelled by the product. There is no
standalone persisted `Generator` `VideoSourceKind` in this checkout; generator
effects are not silently relabelled as live inputs. This checkpoint therefore
does not claim a new generator-input implementation.

It also does not claim real camera or capture-device availability, OS permission
dialog behavior, live NDI/Spout peers, Syphon interoperability, generator
input output, GPU interop, native UI interaction/maximized-window acceptance,
reconnect timing on physical devices, one-hour operation, ASIO, recording
integration, signing, publication, or product-wide acceptance. Those boundaries
remain represented by the `VIDEO-PHYSICAL-001` and related open/deferred
coverage rows.
