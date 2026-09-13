# Authored audio checkpoint — 2026-09-13

This checkpoint closes `AUDIO-AUTHORED-001` for the supported current-source
software slice. It adds an additive authored Timeline audio policy with an
explicit ShowClock master, deterministic PTS/resync rules, and fail-closed
underrun/device-fault behavior. Existing history, command rollback, and
Timeline audio contracts remain intact.

## Implemented contract verified

- `TimelineAudioPolicy` is versioned, serialized additively, and defaults when
  loading legacy Timeline JSON that has no policy field. Unsupported versions,
  unknown enum values, and out-of-bound numeric settings fail closed.
- ShowClock is the sole authored-audio clock master. Source PTS is derived with
  checked arithmetic, device correction is bounded by an explicit slew limit,
  and resync is deterministic from the declared drift threshold and cooldown.
- Seek reanchors to ShowClock, loops wrap to the authored range, and the same
  policy is carried through the protocol snapshot, engine runtime projection,
  and authored-bank projection fence.
- Timeline audio synchronization consumes the policy rather than a hidden
  local threshold. An active sink that ends before its authored duration is an
  explicit underrun: it is retired and held through the existing failure
  receipt/retry fence instead of being respawned indefinitely. Device-fault
  handling uses the same Retire-and-Hold policy boundary.

## Verification

The browser plugin was not available. No new frontend layout claim is made;
the existing source contracts were run. The Windows native gate used the
repository procedure with `vcvars64.bat -vcvars_ver=14.44`, the pinned
Build Tools `14.44.35207` x64 linker, and that exact linker first in
`where.exe link.exe`.

| Check | Result |
| --- | --- |
| `pnpm.cmd --dir app run check:timeline-cue-audio` | PASS |
| `pnpm.cmd --dir app run check:timeline-audio-output-bus` | PASS — 11 assertions |
| `pnpm.cmd --dir app run check:timeline-loop-runtime` | PASS |
| `pnpm.cmd --dir app run check:timeline-transport-runtime` | PASS |
| `cargo test -p protocol --release --locked -j 1 timeline_audio_policy -- --nocapture --test-threads=1` | PASS — 3 passed, 0 failed |
| `cargo test -p engine --release --locked -j 1 timeline_audio -- --nocapture --test-threads=1` | PASS — 34 passed, 0 failed |
| `cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 timeline_audio -- --nocapture --test-threads=1` | PASS — 34 passed, 0 failed |
| `cargo test -p engine --release --locked -j 1 timeline_fractional_ticks -- --nocapture --test-threads=1` | PASS — 6 passed, 0 failed |
| `pnpm.cmd --dir app tauri build --no-bundle` | PASS — TypeScript/Vite build and Windows `target/release/syndocal.exe` build |
| Exact release executable process smoke | PASS — one exact-path `Syndocal` process, nonzero main-window handle, title `Syndocal`, responsive; verification process then exited |
| `git diff --check` | PASS before commit |

## Boundary

This closes the current-source authored Timeline audio schema/migration,
ShowClock/PTS policy, deterministic seek/loop/resync, and fail-closed
underrun/device-fault software slice. It does not claim real audio-device or
ASIO driver behavior, physical audible output, long-duration clock drift,
device swap in the field, recording ownership integration, external clocks,
venue/soak operation, native UI interaction/maximized-window acceptance,
cross-platform execution, signing, publication, or product-wide acceptance.
Those boundaries remain represented by the appropriate open or deferred
markers.
