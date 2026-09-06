# Recording diagnostic pipe cancellation

Base `56bdadd75fc93774c4b18c227d4209466bd444d5`, branch
`codex/syndocal-v1.2`, alpha.69 unchanged.

## Problem and scope

The recording supervisor can confirm the direct encoder has exited while a
child of that encoder still holds its inherited stderr writer. The blocking
diagnostic reader then receives no EOF, and its required join retains the
recording worker indefinitely. A join timeout or detached reader would lose
ownership without resolving that condition.

This tranche targets Windows diagnostic reads after the direct encoder has
been reaped. It does not cancel active rendering, inherited stdin writes, or
promise an OS-level deadline for all recording cleanup. The existing worker
ownership and Start gate remain authoritative until cleanup is confirmed.

## API reference and required evidence

Microsoft documents that cancellation requests do not wait for completion:
[CancelIoEx](https://learn.microsoft.com/en-us/windows/win32/api/ioapiset/nf-ioapiset-cancelioex).
Its synchronous-read support is demonstrated in the official
[Windows engineering article](https://devblogs.microsoft.com/oldnewthing/20170928-00/?p=97105).
A successful cancellation request alone therefore cannot authorize publication
or release a thread/handle owner. The read result and thread completion must
both be observed. `ERROR_NOT_FOUND` can race with read issuance; it does not
establish that a reader is finished.

Implementation and focused verification are recorded below. Evidence directory:
`target/qa/recording-diagnostics-cancel-20260906/`.

The first read-only preflight found no running Syndocal executable; no user
process was terminated and the probe did not mutate the project.

## Implemented behavior

`video_recording_encoder_diagnostics.rs` owns one diagnostic reader. The existing
encoder supervisor is joined first. Stop, abort, or a supervisor error then allows
250ms for diagnostics to drain; on Windows, an unfinished reader receives a
cancellation request. Normal EOF keeps the existing bounded diagnostic tail.
The worker still joins the reader before reporting completion.

A mutex protects an optional exact pipe handle. Reader teardown invalidates the
option before closing the handle, including unwind, so cancellation cannot target
a reused handle during the interval before thread completion. ERROR_NOT_FOUND
retries release this mutex and wait 5ms; they continue until the read starts or
the thread finishes. A pre/post-read flag prevents the generic Interrupted retry
loop from issuing another read. Cancellation remains a failure even if a late EOF
wins the OS race, so the existing encoded.and_then(artifact.publish) path refuses
publication. Unknown OS cancellation failure still retains the mandatory join.

No additional frame queue, pixel copy, or worker thread is introduced. The
existing stderr reader adds two atomic checks per diagnostic read, and shutdown
polling sleeps 5ms only while awaiting completion. No FPS improvement is claimed.
Non-Windows retains the existing blocking join behavior; it was source-reviewed,
not compiled in this Windows checkout.

## Focused verification

- `recording-tests-pipe.log`: pinned MSVC, 51 passed, 5 ignored, 1727 filtered.
  The new inherited-pipe case completed while its exact writer process was still
  alive, required the cancellation-specific error, then released and waited for
  that process and joined the finishing thread. Test helpers are ignored entry
  points invoked explicitly by their owning tests; long A/V acceptance remains
  ignored and is not counted as executed.
- `ffmpeg-smoke.log`: explicit real FFmpeg H.264/AAC recording test, 1 passed.
  Graceful Stop finalized a decodable MP4, preserved the previous destination
  until publication, and validated submitted video plus audio.
- Independent production and real-pipe test review accepted ownership and
  cancellation behavior. A test-only readiness publication race was corrected
  with temporary-file rename before the final focused rerun.
- Current compiler warnings: 0, previous accepted baseline 0, delta 0.

The real-pipe test exercises the inherited handle and supervisor integration;
it does not inject every pre-read/ERROR_NOT_FOUND scheduling interleaving.
Those interleavings also received independent source review. This is not a
hardware/in-app recording or long-duration acceptance claim.

Final readiness-marker regression: pipe-accepted.log, exact qualified test
name, 1 passed / 1782 filtered. The earlier pipe-final.log selected zero tests
and is not acceptance evidence. Test helper processes were absent after testing.

## Native checkpoint

`pnpm --dir app tauri build --no-bundle` passed with the pinned MSVC linker:
release 1m 47s, Vite 6.24s, compiler warnings 0 (baseline 0, delta 0).
Executable SHA256:
`414183BEBA9A730D035F5835C8B1BA87AB2D34181FEF9D090008598333D24F0E`.
`native-launch.json` confirms one responsive maximized Syndocal main window,
PID 93544. The existing working copy `DSF2026-before-output-read.sdc` was opened
through official single-instance ingress; `native-readiness.json` confirms 46
fixtures, 2 video outputs, runtime OK and ownership Ready. MCP mutations: 0.
No physical output was armed. The original Downloads show and the unrelated
viewport checker retain their pre-tranche hashes.

Remaining boundaries are active rendering, inherited stdin writers, cancellation
API/OS failure without confirmed completion, and long/in-app recording acceptance.
This checkpoint closes the tested Windows inherited stderr wait, not all recording
shutdown or performance work.
