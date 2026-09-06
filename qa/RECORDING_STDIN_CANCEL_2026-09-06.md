# Windows recording stdin cancellation

Base `6a65ee2b98906c3b939ec1a7fa2c73fd39b12f2b`, branch
`codex/syndocal-v1.2`, internal product `1.2.0-alpha.69` unchanged.

## Change and boundary

An encoder descendant can retain stdin's read handle after the direct encoder
exits. A recording worker blocked writing a frame then cannot reach EOF cleanup,
even though the encoder supervisor has reaped its direct child.

`video_recording_encoder_stdin.rs` wraps the owned `ChildStdin`. The existing
supervisor remains responsible for input cancellation after direct-child reap;
there is no new thread, frame copy or frame queue. On Windows, explicit Stop
allows a 250ms post-reap grace; abort or a failed encoder result cancels immediately.
This is separate from the existing direct-child 5-second shutdown grace.

Cancellation is latched before `CancelIoEx`, retried across pending-I/O races,
and serialized with pipe-handle destruction to prevent cancellation against a
reused handle. A cancelled write and the encoder result both report failure,
including a racing successful write or a caller ignoring its write error.
Existing publication only proceeds after encoder success, preserving the prior
destination on cancellation. Complete-frame EOF continues to finalize normally.

The supervisor stays owned until the input wrapper closes; OS cancellation or
termination failure is not reported as successful shutdown. Running renderer
work and total OS-level shutdown deadlines remain separate unresolved boundaries.
Non-Windows retains its previous blocking pipe behavior and is not compiled here.

## Local toolchain

The user approved support for this PC's Visual Studio Build Tools 2022 edition.
The wrapper and native procedure now accept its exact MSVC `14.44.35207`
Hostx64/x64 path as well as the existing Community path. Absolute linker pinning,
PATH-first checks and the restricted hosted Enterprise exception are preserved.
Unknown roots and versions remain rejected. Existing ASIO/soak harnesses retain
their narrower Community preflight.

The focused wrapper checker passed 243 assertions and 27 hostile fixtures.
Actual environment capture and Cargo invocation verified the Build Tools linker
as the first `where.exe link.exe` result. Independent source review accepted the
wrapper and recording changes without blocking findings.

## Verification

Evidence directory: `target/qa/recording-stdin-cancel-20260906/`.
The initial pause failures and the continuation evidence are tracked in
[the Luna handoff](LUNA_RESUME_HANDOFF_2026-09-07.md).
The bounded stdin candidate has now passed its focused recording and native
gates; this does not claim completion of the broader recording acceptance.

New real-pipe tests wait for the direct child to exit, retain the descendant's
process handle, establish a successful frame-prefix write, and require Stop or
encoder Drop completion while the inherited reader remains alive. They then
release and wait for that owned helper. A separate complete-frame Stop regression
checks normal EOF success. Existing diagnostics and recording-publication tests
remain applicable.

## 2026-09-07 continuation evidence

- The diagnostic publication assertion passed: `1 passed, 0 failed`, with
  `recording_publication_recovers_after_process_exit_at_each_boundary` selected.
- The real MP4 test passed with the exact Gyan FFmpeg 8.1.2 full shared build:
  `recording_command_writes_a_real_video_and_audio_mp4` passed with
  `--ignored`; the pair exposes `libx264` and AAC. The original LGPL
  `Unknown encoder 'libx264'` failure remains retained as prior evidence.
- The recording regression filter passed `54 passed, 6 ignored, 0 failed`.
  This includes inherited-stdin Stop/Drop, complete-frame graceful EOF,
  publication boundaries, and inherited-stderr cancellation. Ignored helper
  tests remain subprocess-owned and are not counted as standalone acceptance.
- `pnpm.cmd --dir app tauri build --no-bundle` passed under the exact Build
  Tools MSVC 14.44.35207 linker. The fresh artifact is
  `target/release/syndocal.exe`, version `1.2.0-alpha.69`, SHA-256
  `7B99B6B7B518DD6D18EDC6C3FC6270FA63696B4BB3D5BD30E4536B5488A3BF28`.
  `native-launch.json` records one responsive, maximized `Syndocal` window and
  `physicalOutputOperations=0`; the exact checkout process was stopped after
  verification.
- Current metadata now uses author `Seraf()` and publisher/copyright
  `Seraf() / KTN`. This is metadata branding only; the alpha artifact remains
  unsigned until real Authenticode credentials are supplied.

The full `check:release` now passes, including the ASIO v3 contract. The prior
`loader mutation source missing: CloseFn` result was a checker self-test
normalization defect: the mutation source was normalized for contract
inspection but not for the fixture lookup. The checker now normalizes both
mutation sides and its 22 assertions pass.
