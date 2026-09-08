# Native thumbnail cooperative cancellation — 2026-09-08

Base: `df289fb0a0ca6df10898f470b5ad29435ddf288c`.
Working branch: `codex/native-thumbnail-cancel-20260908`.
Scope: one bounded continuation of `MEDIA-DERIVED-001`: exact, cooperative
cancellation for the existing layer and media-asset thumbnail commands. This
does not close whole-media acceptance, retry/recovery acceptance, or the
completion-ledger parent row.

## Change and invariants

`native_thumbnail_ticket.rs` defines the versioned `{ schemaVersion, lane,
requestId }` start ticket. The native gate binds each request to its WebView
owner, lane and random 32-character lowercase request ID. Cancellation routes
only to the ticket's lane and exact owner/request ID. A cancelled worker keeps
its slot until the native operation returns; a late success is rejected. A
failed Channel announcement drops the unstarted job without leaving the gate
occupied.

`nativeThumbnailRequest.ts` connects the existing AbortSignal to the announced
ticket. It accepts cancellation only when the native acknowledgement is exactly
`true`; `false`, malformed tickets and IPC errors are failures. The settled
native read is awaited before the Channel handler is detached, and an aborted
request cannot publish its result. `createLatestThumbnailBatch` aborts the
active request on replacement, clear, reset and dispose while retaining the
worker ownership barrier. Layer and asset lanes remain independent.

The new command is registered in the native admission inventory, frontend
manifest and routing checker. It is classified as recovery maintenance so the
existing terminal-recovery permission path can issue cancellation while an
ordinary thumbnail read remains read-only. No output, ASIO, NDI, project schema,
fixed-three-screen layout, or physical-device path was changed.

## Automated evidence

Commands were run from the repository root on Windows with the maintained
wrapper and exact Build Tools MSVC 14.44.35207 linker pin. All listed commands
exited 0 unless noted; no `--ignored` tests were used in the focused Rust run.

- `pnpm.cmd --dir app run check:native-thumbnail-request`: normal success,
  exact abort cancellation, stale-result rejection, cancellation `false` and
  IPC-error rejection, malformed/foreign ticket rejection, and callback cleanup
  after pre-dispatch owner-barrier rejection.
- `pnpm.cmd --dir app run check:media-thumbnails`: bounded retry, reset,
  authority, cache, disposal and active AbortSignal regression.
- `pnpm.cmd --dir app run check:frontend-invokes`: exact 457 commands.
- `pnpm.cmd --dir app run check:frontend-command-routing`: 133 renderer
  mutations, 31 server-authoritative mutations, 28 raw dispatches, 464 facade
  dispatches.
- `pnpm.cmd --dir app build`: TypeScript and Vite production build.
- `pnpm.cmd --dir app run check:release`: native inventory exact 516 commands,
  inventory SHA-256
  `5120894f36feb82ac58fffd4db20739d80ae1b1a1c556fc95838a1708cbb8eea`, 18
  negative fixtures rejected, plus the existing release/runtime contracts.
- `pnpm.cmd --dir app run check:tauri-build-wrapper`: 243 assertions and 27
  hostile mutation fixtures.
- `cargo test --manifest-path app/src-tauri/Cargo.toml --locked -j 1
  native_thumbnail -- --test-threads=1`: 23 passed, 0 failed, 0 ignored, 1791
  filtered. This includes real PNG/MP4 thumbnail behavior, cancellation,
  independent lanes, Channel serialization, slot retention and poisoned-lock
  rejection.

## Native WebView proof

`pnpm.cmd --dir app tauri build --no-bundle` was run after the final source
change with `vcvars64.bat -vcvars_ver=14.44`, the absolute Build Tools linker
pin, and `where.exe link.exe` first resolving to that exact linker. The exact
artifact was:

```text
path:    target/release/syndocal.exe
version: 1.2.0-alpha.69
bytes:   64,700,928
SHA-256: 40FE423420DD8A0F54A09A5B4A00CF1F2B8E9549D8FC71E60412B264082E2B84
```

The fresh process probe in
`target/qa/native-thumbnail-cancel-20260908-01/native-thumbnail-cancel.json`
used that exact hash and a real Tauri `Channel` in the WebView. It confirmed:

- exactly one responsive, maximized `Syndocal` window;
- `get_snapshot` succeeded and the unknown route was rejected;
- actual authoritative video-file import produced a real layer and media-asset
  ID in the isolated process;
- layer lane: Channel ticket → exact cancellation → native cancelled result;
- asset lane: Channel ticket → exact cancellation → native cancelled result;
- startup remained Standby with lighting/video permissions false and no physical
  output operation (0); and
- the owned EXE exited with zero remaining debug listener.

An earlier probe using an invalid layer/asset ID is retained as a failed
diagnostic and is not counted as acceptance. The final probe used a real
prepared MP4 and passed on the rebuilt artifact. No user PNG was moved or
renamed.

## Review and limits

The stable diff was independently reviewed before integration by Astra low.
The review scope included the native ticket/gate, frontend AbortSignal
protocol, command inventories and fail-closed behavior. An initial P2 found
that pre-dispatch owner-barrier rejection could retain a Tauri callback. The
explicit runtime Channel cleanup seam and its regression were added, then the
review rechecked ten pre-dispatch failures and reported ACCEPT with zero
callbacks retained. No assertion was weakened to close the finding.

This unit does not claim a hard cancellation deadline for synchronous OS I/O,
decoder, GPU/effect work, or snapshot locks. It does not claim decoder process
termination, waveform/proxy completion, physical output, ASIO/NDI/DMX device
acceptance, Mac execution, signing, publication, venue acceptance, or product-
wide completion. The previously completed missing-file → UI Retry → recovery
test was not rerun.

Evidence is kept under
`target/qa/native-thumbnail-cancel-20260908-01/`; generated `target/qa` files
are ignored and are not part of the source commit.
