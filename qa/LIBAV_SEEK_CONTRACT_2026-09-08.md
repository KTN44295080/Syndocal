# Libav seek/catch-up acceptance — 2026-09-08

Base: `ab8e670be7075561c00ba8341dbf3755aafca652`.
Branch: `chatgpt/macos-artifact-validation`.
This checkpoint changes two existing tests, not the production decoder.
ChatGPT performed implementation and self-review; no Codex/Works or delegated
reviewer was used. Independent-review acceptance is not claimed.

## Reproduction and corrected contract

Both previously recorded failures were reproduced before editing. The B-frame
fixture's 592 -> 1500ms gap was 908ms; the offset-timestamp fixture's 500 ->
1500ms gap was 1000ms. The existing 1000ms-inclusive catch-up policy correctly
continued, while the old tests expected resets. The policy is unchanged.

The B-frame test now checks 592 -> 1592ms continuation, 1592 -> 2593ms reopen,
backward seek, reference pixels, session counts, EOF drain and late-frame reuse.
The positive/negative start-time fixtures check 500 -> 1500ms continuation,
1500 -> 100ms reversal and 100 -> 1101ms reopen, with exact normalized PTS,
reference pixels and reset counts at every step. No original pixel tolerance,
EOF assertion or final reset assertion was weakened or removed.
Session opens can include the existing bounded seek-error recovery; they are not
universally equal to reset count plus one. Catch-up still requires one open.

## Investigated but rejected runtime change

A direct primary-seek diagnostic failed for the negative-start MPEG-TS fixture
at 100ms with `decoder produced no video frame`. The existing higher-level
recovery returned correct pixels; this is not proof of a user-visible frame error.

An origin-adjusted, at-or-before timestamp experiment instead returned 1200ms
for the positive fixture's 100ms primary request. It was rejected and completely
removed from production code. Its source and failing logs remain only in the
ignored evidence directory. Direct demux seeking, timestamp origin and decoder
preroll require a separate design before optimizing that recovery path.
The final tests verify the existing public decoder result, not an unsupported
assumption that every demuxer primary seek succeeds without recovery.

The attempted extra numerical edge-case test write was denied by the service.
It was not retried; the unused experimental helper was removed with the rejected
runtime patch. No numerical-range acceptance is inferred from that attempt.
Reference inspected: FFmpeg Demuxing avformat_seek_file and AVStream.start_time
API documentation, plus the installed ffmpeg-next 8.1.0 input seek wrapper.

## Evidence scope

Evidence is retained under `target/qa/libav-seek-contract-20260908/`.
`bframes-before.log`, `timestamps-before.log`, `libav-full-after.log`,
`primary-seek-before.log` and `libav-full-fixed.log` are preserved failure records,
not renamed or overwritten as successful runs. Experimental source is retained
in `rejected-origin-projection.rs.txt` and `rejected-seek-time-helper.rs.txt`.
The checked production prefix of libav_decoder.rs is byte-identical to the base
(after normalizing checkout line endings); only its cfg(test) module changes.
UI, command schemas, ASIO/NDI, rendering hot paths and product versions are unchanged.
No new native application build, native-window launch, Mac or device acceptance
is claimed for this tests-only checkpoint. MEDIA-DERIVED-001 remains open.

## Final executed results

All Cargo runs used the maintained wrapper's exact Build Tools MSVC 14.44.35207
absolute linker pin and PATH-first verification. FFmpeg/FFprobe were explicitly
selected from the existing pinned 8.1.2 shared installation.

- `cargo test -p video --release --locked --features libav -- --test-threads=1`:
  193 passed / 0 failed / 8 ignored; the final formatted source was rerun successfully.
- `cargo test -p video --release --locked -- --test-threads=1`:
  180 passed / 0 failed / 3 ignored. The later formatting touched only libav-gated tests.
- `pnpm --dir app run check:release`: exit 0 on the final source.
- `check:tauri-build-wrapper`: 243 assertions / 27 hostile fixtures passed.
- Completion and Q1-Q4 ledger validators passed; 50 Open + 8 Deferred unchanged.
- Targeted baseline logs and final video logs contain zero Rust warning diagnostics.
  The full ordinary-video pre-change warning baseline was not rerun or invented.

Final logs: `libav-full-format-final.log`, `video-default-final.log`,
`release-final.log`, `wrapper.log`. Ignored tests are not counted as run.
The seven protected dirty macOS files match their initial SHA-256 hashes.
This closes the two stale libav test failures, not all video/media acceptance,
seek-performance work, native cancellation, main integration or release readiness.
