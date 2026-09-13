# ShowClock implementation checkpoint — 2026-09-13

This checkpoint closes `SHOWCLOCK-IMPLEMENTATION-001` for the supported
current-source implementation slice. The ShowClock protocol, estimator,
bounded scheduler, output-generation gate, exact-peer LAN adapter, Tauri
IPC/UI boundary, and two-process loopback are implemented and revalidated on
the current branch.

## Implemented contract verified

- Version-1 ShowClock samples and typed action payloads use explicit canonical
  wire tags, authenticated admission, stable identity validation, replay and
  reorder rejection, and session-local bounded action dedupe that fails closed
  without eviction.
- The estimator establishes its initial offset from an authenticated sample,
  applies bounded correction without stepping show time backward, derives
  expiry-based `STALE`, and requires explicit Manual Hold plus an advanced
  operator-confirmed fencing generation before re-locking.
- The scheduler is fixed-capacity and horizon-bounded. It binds project,
  lease, audio, recording, clock, fencing, owner, and Locked-state generations
  before dispatch; stale or faulted state revokes the local output permit.
- The LAN adapter is manually paired exact-peer UDP unicast with no discovery
  or automatic failover. Wrong-source traffic cannot extend the receive
  deadline, and malformed/oversized paired traffic fails closed.
- Tauri owns process-lifetime Primary/Standby workers with typed action/status
  IPC, explicit output-arm confirmation, clean joined shutdown, session
  incarnation fencing, status-sequence recovery, and Manual Hold/Re-arm UI.
  The two-process loopback harness exercises paired authenticated samples and
  child lifecycle ownership.

## Verification

The browser plugin was not available. The current source was revalidated with
the repository's pinned Windows procedure (`vcvars64.bat -vcvars_ver=14.44` and
the Build Tools `14.44.35207` x64 linker first in `where.exe link.exe`). No
physical output or two-machine venue claim is made.

| Check | Result |
| --- | --- |
| `cargo test -p protocol --release --locked -j 1 show_clock -- --nocapture --test-threads=1` | PASS — 21 passed, 0 failed |
| `cargo test -p io --release --locked -j 1 show_clock_lan -- --nocapture --test-threads=1` | PASS — 3 passed, 0 failed |
| `cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 show_clock_ipc::tests -- --nocapture --test-threads=1` | PASS — 7 passed, 0 failed |
| `cargo test -p io --release --locked -j 1 --test show_clock_two_process -- --nocapture --test-threads=1` | PASS — 2 passed, 0 failed |
| `pnpm.cmd --dir app run check:frontend-invokes` | PASS — 475 commands |
| `pnpm.cmd --dir app run check:frontend-command-routing` | PASS — 133 renderer, 31 server-authoritative, 28 raw, 480 facade dispatches |
| `node app/scripts/check-tauri-admission-inventory.mjs` | PASS — 534 commands; 18 negative fixtures rejected |
| `pnpm.cmd --dir app run check:output-control-runtime` | PASS |
| `pnpm.cmd --dir app build` | PASS — TypeScript/Vite production build |
| `pnpm.cmd --dir app tauri build --no-bundle` | PASS — current Windows release executable with pinned linker |
| Exact release executable process smoke | PASS — one exact-path `Syndocal` process, nonzero main-window handle, title `Syndocal`, responsive; verification process then exited |
| `git diff --check` | PASS before checkpoint commit |

The current-source executable used by the native process smoke was:

`target/release/syndocal.exe` SHA-256
`BEF911C6D3BF83362504AC466B01459314A02F04D5073E3C158302BBE2CD4A21`

## Boundary

This closes the current-source ShowClock implementation, authenticated
estimator/scheduler, generation-bound output gate, exact-peer loopback, Tauri
IPC/UI wiring, and two-process software slice. It does not claim native
button-by-button UI acceptance, physical MIDI/OSC/DMX/Art-Net output, real
wired two-machine partition/rejoin, crash/restart replay-state restoration,
witness or physical interlock, automatic failover, encoder/venue operation,
signing, publication, or product-wide acceptance. Those remain represented by
`COV-SHOWCLOCK-001`'s residual risk and the `SHOWCLOCK-VENUE-001` and related
external Flow markers.
