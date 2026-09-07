# Recording encoder shutdown

Base `f7173c9bb62ff5959cd14dd4f47b9ac016fff9d5`, branch
`codex/syndocal-v1.2`, internal product `1.2.0-alpha.69`.

## Problem and boundary

The existing Stop operation bounds its caller's worker wait to 250ms and
retains the recording worker, but cannot reach FFmpeg while the worker is
blocked in stdin.write_all or child.wait. The diagnostic reader is joined on
error, so it can also keep that worker alive. This tranche addresses the
encoder process boundary; it does not claim cancellation of synchronous
renderer work or a hard deadline for the entire runtime Drop.

Ordinary EOF must allow FFmpeg to finalize MP4 before publication. A forced
termination, unconfirmed teardown, or encoder error must not publish the
partial recording. The existing target remains untouched on these failures.
The outer runtime must retain ownership until its child process and pipe work
are reclaimed, so a new recording cannot overlap an unfinished one.

## Implemented behavior and ownership

The direct child has a dedicated supervisor, polling every 25ms only while
recording. It observes Stop independently of renderer locks and blocked stdin
writes. The first observed Stop or EOF-finalization request starts one 5-second
grace period. Abort or expiry is latched before checking process exit; a zero
exit code cannot turn an aborted recording into a successful publication.
Termination is requested on expiry, and `try_wait` must confirm exit. Kill or
poll failures retain the process owner and retry termination at 250ms intervals;
they do not return a false completion. Scheduling and OS acknowledgement are
not hard wall-clock guarantees.

A guarded child handoff also covers thread-spawn failure and panic unwinding.
The supervisor and diagnostic reader are joined before the outer worker returns.
The existing runtime consequently continues to block another Start when cleanup
is incomplete. Inherited pipe handles can still hold the reader or writer open;
no process-tree termination or pipe-thread cancellation is claimed. The renderer
stop check after rendering avoids starting another write once Stop is observed.

`video_recording.rs` keeps capture/encode/publication orchestration. Process
supervision lives in two small dedicated modules. The existing recording test
module moves out of main with its module identity unchanged; all helper/test
tokens outside the strengthened graceful recording test were checked unchanged
apart from formatting. No extra frame copy, pixel conversion or frame queue is
introduced. One supervisor thread is added during recording; no FPS improvement
or complete performance acceptance is claimed.

## Verification record

Evidence is under `target/qa/recording-encoder-stop-20260906/`.
`recording-tests-owned-final.log`: pinned MSVC recording filter passed, 38 passed,
3 ignored, 1724 filtered, 1.52s. The ignored cases are a subprocess helper,
explicit FFmpeg acceptance and long A/V acceptance; they are not silently counted
as run. The first run exposed one expected-message mismatch: the new cleanup
preserves the primary error and appends the diagnostic tail. The assertion now
requires both, rather than discarding the tail to preserve the old expectation.
The retired artifact `retain` method and its dedicated old-path test were removed:
unknown child termination now retains the live process/worker/artifact owner,
instead of releasing process ownership and returning with only a partial path.
The earlier native build exposed the unused method warning; it is not allowed
in the final checkpoint and is not suppressed.

Independent review accepted the process/pipe ownership and publication boundary.
Its stale source-seam delimiter finding was corrected to the new recording
function boundary, preserving the effect-aware preview assertion. Build-wrapper
checks passed, 231 assertions and 27 hostile mutation fixtures.
`ffmpeg-smoke-final.log`: the explicitly selected ignored A/V test passed,
1 passed, 1764 filtered, 0.33s. It uses the production supervisor, retains the
old target until success, confirms video and audio streams and decodes all 30
video frames. `source-seam-final.log`: 1 passed, 1764 filtered, 0.01s.
Final test compiler warnings are zero.

`pnpm --dir app tauri build --no-bundle` passed in `native-build-final.log`:
native release 2m53s, Vite 9.87s, App chunk 499.92kB unchanged. First-party
compiler/Vite warning baseline zero, final zero, delta zero. The earlier build
had one unused-method warning, removed with the retired path as described above.
Exact Community MSVC 14.44.35207 linker pin and PATH-first checks passed.

## Native and MCP checkpoint

The build preflight stopped only this checkout's previous executable, PID 76980.
The final build preflight verified that same executable was not running.
Launched `C:/Users/kouty/Documents/KDMX/target/release/syndocal.exe`, PID 101036,
SHA256 `33B8FD7B7DBF67371EFED5A0C7D84C9D813D349E3B193D3F29737CDA97709B2D`.
`native-launch.json` proves exactly one responsive maximized Syndocal main
window; auxiliary windows are recorded separately.

A fresh MCP connection completed fixture/runtime reads in `startup-runtime.json`.
The official single-instance file-open path then restored the already preserved
copy `target/qa/spout-output-snapshot-20260906/DSF2026-before-output-read.sdc`.
`before-runtime.json` and `after-runtime.json` match project checkpoint
`6bb71f6c4218ce00e17ad0e63dcc5084e5d85121205c498c545e7a2fffa30229`, all 46 fixture
transforms, both output descriptors (3840x2160 and 1920x1080), and Timeline 1
paused at 0ms with duration 307274ms. Ownership is Ready/Standby,
ProjectSwapDisarmed, error null. MCP mutations: zero. No real output was armed.

This proves native launch/readiness and software encoder behavior, not real
in-app recording operations, long-duration recording or new hardware acceptance.

Preserve the unrelated viewport checker and original Unity test show. Existing
target compare-and-replace concurrency, synchronous renderer cancellation,
recording clock/crash/import acceptance and the broader recording ledger remain
separate unresolved work. Spout, Video BO isolation/restoration and Unity
smoothness already have user acceptance and are not reopened by this tranche.

## 2026-09-07 process-tree and encoder deadline completion

The current encoder process owner creates a Windows Job Object for every direct
FFmpeg child and sets `KILL_ON_JOB_CLOSE`. The stdin and stderr reader controls
retain an erased Job Object guard until their respective handles are closed.
This closes the inherited-pipe descendant ownership gap: a descendant cannot
outlive the last owned cleanup handle. `recording_encoder_failure_reaps_the_descendant_process_tree`
proves the direct-child/descendant case; the existing inherited stdin/stderr
tests continue to prove blocked pipe cancellation.

The supervisor continues to poll and confirm process exit. At the shared
ten-second total-stop deadline it transfers the complete child owner to a
named process reaper, retaining a retry/quarantine path if reaper creation
fails. No false-success publication is possible after abort or termination
failure. The separate renderer worker lifecycle applies the same bounded
ownership transfer, while arbitrary in-process renderer operations remain
cooperative because safe Rust cannot force-kill their thread.
