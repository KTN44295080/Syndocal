# Native thumbnail work ownership — 2026-09-08

Base: `3809edca19520202537cf2cfc26f16c242168df3`.
Branch: `chatgpt/macos-artifact-validation`.
Scope: native admission and renderer waiting for the two thumbnail commands;
a bounded continuation of `MEDIA-DERIVED-001`, not whole-media acceptance.
Implementation and self-review are by ChatGPT. No Codex/Works or delegated
model was used. Independent review is not claimed.

## Source finding and bounded change

The old layer-thumbnail command ran synchronously and waited on the shared
preview renderer mutex. The asset command spawned a blocking worker for every
admitted request without an application-wide thumbnail admission bound.
The previous frontend fix capped only each controller's requests; it did not
bound callers of the native commands. These are source findings, not a measured
UI stall, native-process count or FPS regression.

`native_thumbnail_work.rs` owns two independent one-job gates in AppState.
Admission happens before snapshot copying or media catalog capture. The permit
moves with the queued/executing worker and is released only when that job drops,
including error and panic unwinding. Dropping an async waiter requests cooperative
cancellation; it does not free an executing worker's permit.
`native_thumbnail_dispatch.rs` is the thin Tauri blocking-pool adapter.
The layer command is async; snapshot/decode/renderer waiting occurs in that pool.
Renderer acquisition has a two-second checked wait budget and observes cancellation.
No output-loop logic, UI layout, IPC argument schema or ASIO/NDI route is changed.

## Compatibility, performance and stopping limits

The two command names, arguments and frame/error result shapes remain unchanged.
Excess native calls now return a busy error rather than entering an unbounded
blocking queue. There is no automatic retry or extra pending-queue fallback.
The ordinary one-request-per-lane frontend path is retained.
The shared renderer is not replaced; its decoder/input ownership remains canonical.
Only a contended background thumbnail worker polls for that mutex, at up to
10ms intervals within the wait budget. No new permanent threads or per-frame
polling, snapshot copies or transport workers are introduced.

The asset copy path now accepts the worker cancellation flag through its existing
hash/copy checks. Catalog fingerprints, protected private-copy creation and
post-render E/R/H/full-asset equality checks are retained. Existing interactive
preview calls use the same copy implementation with their original non-cancelled
flag; this change does not alter their session lifecycle.

Cancellation is cooperative. An already-running OS read, metadata probe, decoder,
GPU/effect operation or snapshot lock can still block until that operation returns.
The two-second budget covers renderer acquisition only, not total completion or
OS scheduling latency. A frontend reset/unmount is NOT wired to abort a native
Tauri handler by this change; the cancellation guard covers actual async-future
retirement. No real decoder-process termination, hard-stop deadline or UI cancellation
API is claimed. Preview-session frame commands are outside the two gated commands.
Waveform/proxy jobs, cache budgets, full media lifecycle and real-show performance
remain separate requirements. The existing seven dirty Mac files are protected.

## Executed automated evidence

Windows / Node 22.22.1 / pnpm 10.9.0; Cargo commands used the maintained wrapper's
exact Build Tools MSVC 14.44.35207 absolute linker pin and verified PATH-first order.

- `cargo test -p syndocal --release --locked native_thumbnail -- --test-threads=1`:
  12 passed / 0 failed / 0 ignored. Includes 16-caller admission, independent lanes,
  cancelled queued work, retained running ownership, late-success rejection,
  error/unwind release, cancelled/expired/poisoned renderer waits, and actual Tauri
  blocking-pool dispatch including a dropped async waiter.
- `cargo test -p syndocal --release --locked media_asset_thumbnail -- --test-threads=1`:
  4 passed / 0 failed / 0 ignored. Catalog identity, dimensions, stale E/R/H and
  same-ID content replacement rejection remain enforced.
- `cargo test -p syndocal --release --locked media_asset_preview -- --test-threads=1`:
  6 passed / 0 failed / 0 ignored. Includes a real PNG private copy, owner retirement,
  expiry/cleanup and preview-position limits. This is not a real video decode test.
- `pnpm --dir app run check:release`: exit 0, including the existing frontend
  thumbnail, ASIO packaging, blackout/output-authority and video/camera contracts.
- `check:tauri-build-wrapper`: 243 assertions / 27 hostile fixtures passed.
- Completion-ledger and Q1-Q4 validators passed; their 50 Open + 8 Deferred rows
  are unchanged and do not constitute implementation-completion percentages.
- New Rust modules and their test modules were formatted with rustfmt.

Test evidence was produced before an indentation-only alignment of the two
AppState initializer fields. The subsequent native build includes that alignment;
there was no intervening behavior change. One combined source-inspection tool call
was rejected by the service; it was not reissued or treated as successful evidence.

## Native artifact and acceptance limits

`pnpm --dir app tauri build --no-bundle` exited 0; optimized Rust build 3m33s.
The three targeted test logs and the native-build log contain zero Rust warning
diagnostics. A pre-change native warning baseline was not measured; no delta is
invented. This is not a full supported-platform warning-matrix acceptance.

Artifact: this worktree's `target/release/syndocal.exe`, version `1.2.0-alpha.69`,
64,524,800 bytes, SHA-256:
`F67E42A89EEDF74BF9AED4A7783FA0DD2A0309D27C7F17E3F8FE7EF7196A5525`.
It was built from the stated base plus the checkpoint's owned source changes,
not from a previously frozen release tag.
At `2026-09-07T18:33:29.3047040Z`, PID 48676 owned exactly one responsive,
maximized Syndocal window for that exact executable. Launch verification exited 0.
The helper issued zero physical-output commands and stopped only its own process;
the post-check found zero remaining processes for that executable.
This proves native-window launch, not operator thumbnail behavior, long/hung video
decoder cancellation, cameras, ASIO/NDI/DMX devices, macOS or venue acceptance.

Evidence: `target/qa/native-thumbnail-work-20260908/` contains the baseline-source
snapshot, protected file hashes, three Rust test logs, release/wrapper logs,
`native-build-01.log`, `native-launch-01.log` and `native-launch-01.json`.
All seven pre-existing dirty macOS files still match their starting SHA-256 hashes.
No general cleanup, installed-app update, release publication or physical output
activation was performed. MEDIA-DERIVED-001 remains open beyond this subset.
