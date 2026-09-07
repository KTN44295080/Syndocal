# Thumbnail render cancellation — 2026-09-08

Base: `269c23204c4b3f16cc3da1024cec10a204e1790e`.
Branch: `chatgpt/macos-artifact-validation`.
Scope: forward the existing native thumbnail cancellation signal into rendering
and decoding; another bounded subset of `MEDIA-DERIVED-001`.
Implementation and author self-review were performed directly in ChatGPT.
No Codex/Works or separate-model delegation was used; independent review is unverified.

## Observed gap

The previous native job gate checked cancellation before/after its operation and
while waiting for the renderer. Both thumbnail operations still used the ordinary
`render_layer_preview` entry point, so that flag did not reach the renderer's
existing cancellable input/effect/decode chain. The old layer method is retained
as source evidence in `target/qa/thumbnail-render-cancel-20260908/baseline-layer-preview.txt`.
This is a source-confirmed missing propagation path, not a measured hardware stall.

## Change and responsibility boundaries

`crates/video/src/layer_thumbnail_render.rs` owns one canonical thumbnail rendering
implementation. The original API delegates with no cancellation, retaining normal
provider calls. The new cancellable entry delegates with the supplied callback.
Preparation uses the existing cancellable input/effect pipeline; boundary checks
before preparation and before/after composition reject cancelled late results.
The native layer and asset jobs forward their existing AtomicBool through that
callback. The asset private-copy helper retains its ordinary preview entry and
routes both modes into the same renderer, not a duplicated compositor or decoder.

## Compatibility, cost and non-claims

Command names, arguments, frame shapes, source fingerprints, post-render asset
E/R/H checks and native one-worker-per-lane admission are unchanged. The fixed
three-screen UI and Windows ASIO/NDI fail-closed paths are not edited.
No new permanent worker, queue or frame copy is added by the forwarding seam.
The opted-in FFmpeg CLI branch reuses the existing cancellable process helper,
including its temporary pipe-reader threads; ordinary APIs retain the non-cancellable
provider path. The change is not claimed to reduce CPU use, latency or frame drops.

The callback is cooperative. Synchronous HAP/libav, image decode, composition,
GPU/effect and OS I/O cannot be interrupted inside a single operation by this seam.
The existing subprocess helper's descendant/pipe and OS termination-failure limits
are not resolved here. A render must return before the existing worker permit is
released; no total hard-stop deadline, forced reaping or detached-worker shortcut
is introduced. Frontend reset/unmount still does not send an explicit native cancel
request. This work connects actual async-waiter retirement to supported decode
boundaries; it does not claim complete user-facing cancellation acceptance.

## Focused automated evidence

On the exact pinned Build Tools MSVC 14.44.35207 environment:
`cargo test -p video --release --locked layer_thumbnail_render -- --test-threads=1`
passed 6 / failed 0 / ignored 0. Tests exercise the real renderer with a controlled
decoder: pre-cancel rejection, normal/cancellable pixel equality and distinct decoder
paths, late-success rejection, renderer reuse, no placeholder/sync retry after decoder
cancellation, and unchanged size/missing-layer errors. No physical input was opened.
`pnpm --dir app run check:release` exited 0 on the changed source.

## Full video gate and unresolved libav fixtures

`cargo test -p video --release --locked -- --test-threads=1` passed:
180 passed / 0 failed / 3 ignored. The ignored cases were not counted as run.
The same suite with `--features libav` ran and FAILED:
191 passed / 2 failed / 8 ignored. Its first failure log is preserved.
The failures are:
- `libav_decoder::tests::sequential_b_frame_session_matches_reference_and_drains_eof`
  at the exact session-open count after its forward jump (observed 1, expected 2).
- `libav_decoder::tests::normalizes_positive_and_negative_stream_start_times_for_decode_and_seek`
  at the reset count (observed 1, expected 2 for the positive fixture).

Source inspection identifies stale seek inputs: the existing runtime and catch-up
regression accept forward gaps up to 1000ms. The first test's 592 -> 1500ms jump
is 908ms; the second's 500 -> 1500ms jump is exactly 1000ms. Both are inside that
continuation budget, despite the old tests expecting a reopened session. Pixel
comparisons before the failing count assertions passed. Neither the decoder nor
these fixture files was changed by this checkpoint; a clean-base rerun was not made.

The proposed repair was to move the test targets beyond the existing catch-up
budget, within each fixture's duration, retaining all pixel, timestamp, seek/reset
count and EOF assertions. That edit/test tool call was rejected by the service
and was not reissued. No runtime threshold or test assertion was weakened.
The libav full gate remains failed, and full libav acceptance is NOT claimed.
A separate combined inspection command was also rejected; neither denied call
is counted as successful work. Native focused checks below are a separate scope.

## Native focused acceptance

`cargo test -p syndocal --release --locked thumbnail -- --test-threads=1`
exited 0: 17 passed / 0 failed / 0 ignored. This includes the new real-PNG test
comparing normal and cancellable renderer output, rejecting pre-cancelled render,
retaining the private copy while owned and deleting it when its owner drops.
It also reruns the previous native admission, async-waiter retirement, renderer-wait
and asset-authority tests. These tests do not launch cameras, DMX or real-show output.
The optimized test build ran through the exact Build Tools MSVC 14.44.35207 wrapper.
`check:tauri-build-wrapper` passed 243 assertions / 27 hostile fixtures; completion
and Q1-Q4 ledgers passed with their acceptance states unchanged.

The cancellation subset therefore has passing focused tests and ordinary-video
regressions. This does not override the explicitly failing full libav matrix above.

## Native build, launch and final checkpoint boundary

`pnpm --dir app tauri build --no-bundle` exited 0; the optimized Rust stage
finished in 4m35s. The four test logs and native-build log contain zero Rust warning
diagnostics; a pre-change warning baseline for these configurations was not measured.
The failed libav result is retained despite having no compiler warnings.

Fresh artifact: `target/release/syndocal.exe`, 64,528,896 bytes,
version `1.2.0-alpha.69`, SHA-256:
`75DB03CC03B24B7A8360712DB45CD8ECF3D5D405D8127446B6CFDF2313975058`.
Built from the stated base plus this checkpoint's source changes, not a frozen
release tag. At `2026-09-07T19:00:31.1023068Z`, its PID 41056 had one visible,
responsive and maximized Syndocal window. Launch verification exited 0, issued
zero physical-output commands and stopped only its own exact-path application.
The post-check found zero remaining processes for that executable.
This is native-window acceptance, not end-user cancellation, device or venue QA.

Evidence remains in `target/qa/thumbnail-render-cancel-20260908/`: the source
baseline, protected hashes, video-targeted/full/libav and native-thumbnail logs,
release/wrapper logs, native-build log and native-launch JSON/log. The seven
pre-existing macOS files retain their starting SHA-256 hashes and remain outside
this checkpoint. No independent review, complete libav acceptance, main-branch
integration, installer update or public release is claimed. MEDIA-DERIVED-001
and the overall completion ledger remain open beyond the tested subset.
