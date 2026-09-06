# Recording worker ownership and Video BO recovery regression

Base `56d4d41fdc7eec7b4e295a8c7ad23bc3c3d238e3`, branch `codex/syndocal-v1.2`,
internal product `1.2.0-alpha.69`. Continued authorized remaining-work cleanup;
bounded implementation delegated to Luna max, independently reviewed and
integrated by root.

## Recording behavior

Explicit Stop previously removed the worker from shared runtime state and then
joined it without a deadline. A blocked renderer/encoder could hold that call
indefinitely, while another Start could observe no retained worker. Start also
used `status.active` even though a worker can publish inactive just before its
thread actually exits.

The lifecycle module now owns the stop flag and join handle together. Explicit
Stop requests termination and waits with a 250ms worker-wait budget, checking
`JoinHandle::is_finished` at intervals of at most 5ms. On timeout it returns a
specific still-stopping error and keeps the original worker owned. Start and
Stop remain serialized by the recording runtime mutex. Start rejects a live
worker even with an inactive status; a finished worker can be joined and
replaced. Worker panic remains an explicit error, and a poisoned status no
longer looks inactive during Start admission.

No per-frame/background polling was added. This is a bound on the explicit
worker wait, not a hard wall-clock bound on mutex acquisition or the entire IPC
call. Renderer locks, FFmpeg stdin writes/process waits and runtime Drop remain
unbounded. Drop still requests stop and joins rather than abandoning the
worker. No encoder termination, partial-file publication/cleanup, project
reset, protocol or physical-output semantics changed.

An initial completion-notification design was rejected during review because
notifications can precede actual thread termination. The final implementation
uses only thread termination truth. Adapter tests release controlled workers
on assertion unwinding before the runtime's blocking Drop.

## Video BO recovery

Existing tests proved hard black and lighting isolation but did not test
ordinary Video ON -> OFF followed by a nonblack artistic Spout frame. The new
test uses a deterministic frame provider with the production artistic renderer,
materializer and send-authorization boundary, ending in a recording fake Spout
sender. The old authority must remain rejected after both transitions; a fresh
authority must render and send the expected artistic pixels.

Further investigation found a concrete failure path: an ON -> OFF pair restores
the blackout bit but advances the presentation token. Render-time revalidation
classified that token-only mismatch as a fatal error without Follow. Paused
keepalive could also fault. This is a reproducible code path, not proof that it
caused the user's particular incident.

The shared fence now distinguishes `PresentationChanged` only after ownership,
the full output descriptor and safety checks. Spout discards that stale frame
and reacquires at the next tick. Owner/route mismatch and SDK failures retain
their existing handling. NDI's string-wrapper behavior is unchanged.

An injected production worker-loop test also verifies stale-frame rejection,
fresh artistic-frame delivery, worker join and lease retirement. It exercises
the send-time retry boundary. The production render-error decision is extracted
into a small function: a real classified Video ON/OFF result must yield Retry
without Follow or old authority; Other remains an error without Follow. The
paused keepalive classifier is executed against the same Video round trip.
DirectX/Spout receiver behavior and Unity display remain separate
from software/fake-transport evidence. No real Spout or DMX output is started
by these tests.

## Verification

Final focused native tests passed using the exact MSVC 14.44.35207 linker,
absolute Cargo pin and PATH-first verification required by WINDOWS_NATIVE_BUILD:

- `cargo test -p syndocal --locked spout_ -- --nocapture --test-threads=1`:
  91 passed, 5 ignored, 1642 filtered; 3.57s. This includes the three new recovery
  tests and existing route/SDK-error rejection and teardown coverage.
- `cargo test -p syndocal --locked recording_ -- --nocapture --test-threads=1`:
  29 passed, 2 ignored, 1707 filtered; 0.60s. Includes actual Stop timeout,
  retained ownership, Start rejection and late completion/reap.
- `cargo test -p syndocal --locked output_blackout_target_video_fence_ignores_lighting_and_preserves_s0 -- --nocapture --test-threads=1`:
  2 passed, 1736 filtered; 0.03s, through Spout and NDI's shared fence modules.

Logs are under `target/qa/recording-retention-20260906/`:
`spout-tests-final.log`, `recording-tests-final.log`, `target-fence-tests-final.log`.
Ignored tests were not executed and are not claimed as acceptance.

`pnpm --dir app tauri build --no-bundle` passed (native release 2m14s,
Vite 7.56s). First-party warning baseline from the prior accepted native
checkpoint is zero; final test/build compiler warnings and Vite advisories are
zero, delta zero. App chunk remains 499.96kB. The first test
compile caught private-field access in the new Spout test; it was corrected to
the existing getter without weakening its assertion. Only final successful
runs count as acceptance.

Independent review accepted the final lifecycle, fence, Spout production and
test diffs. Its test-unwind cleanup and render-branch coverage findings were
fixed before final validation. The first worker-loop run also caught a missing
all-deny teardown transition in the test; the test now follows the production
retirement protocol after the fake SDK send has returned. Keep
the unrelated `app/scripts/check-viewport-containment.mjs`, user show files and
Unity assets unchanged.

## Native and MCP acceptance boundary

The build wrapper resolved and stopped only this checkout's executable (old
PID 95484). Launched `C:/Users/kouty/Documents/KDMX/target/release/syndocal.exe`,
PID 98136, SHA256
`83EC6C76BAA85A49587A5A25658A6921C984C76318BDAFD3623DFA1A700D1D9C`.
Exactly one responsive maximized Syndocal main window was verified; auxiliary
windows were recorded separately. A fresh MCP stdio connection negotiated five
tools and successfully completed fixture-list and runtime-status reads, zero
mutations. Logs: `native-build.log`, `native-launch.json`,
`native-mcp-probe.json` in the evidence directory above.

Startup project remains empty at E0/R1, no fixtures or outputs, Follow idle,
effective lighting/video/S0 blackout false. Output ownership is the existing
Standby/StartupDenied state. This does not exercise a user's show, Video BO
restoration in Unity, fixture mutation or UI Undo/Redo. Those acceptance items
remain open. No physical output was activated, user show was opened or Unity
asset changed. The recording encoder/renderer/Drop stop bound also remains open
as described above. Changes add no per-frame polling, capture or pixel copies;
no measured FPS improvement is claimed.

## Follow-on: runtime separation and diagnostic-reader ownership

Base `4d636c954e2b7f1e68f9e7ab87ba0e4155b9ddee`, same branch/product version.
The recording status, runtime lifecycle and Stop application operation move
out of main into `video_recording_runtime.rs`. The Tauri commands remain thin
adapters; the thread termination mechanism remains in its separate lifecycle
module. Status serialization and Start/Stop behavior are preserved.

Inspection also found an ownership gap on encoder pipe/wait errors. Those
paths returned after spawning the stderr reader but without joining it, so
the outer worker could finish and admit a replacement recording while the old
reader remained alive. Both error paths now retain the worker until the
diagnostic reader joins, preserving the primary encoder error and any reader
failure. This is not a full shutdown deadline: inherited pipe handles can keep
the reader blocked even after the direct encoder child exits. The retained
outer worker continues to gate Start and the explicit Stop wait remains bounded
as documented above. No immediate kill on ordinary Stop is introduced.

Independent review accepted both the runtime extraction and error-path reader
ownership fix. Main is reduced by 144 lines; no per-frame work is added.
`cargo test -p syndocal --locked recording_ -- --nocapture --test-threads=1`
passed: 33 passed, 2 ignored, 1707 filtered, 0.65s. The four new tests exercise
a reader blocked on a controlled Read, primary-error preservation, diagnostic
read failure and reader panic. Existing timeout/Start/reap tests run inside the
new runtime module with unchanged assertions. Exact MSVC pin and PATH-first
checks passed; test compiler warnings zero. Evidence:
`target/qa/recording-runtime-20260906/recording-tests.log`.

`pnpm --dir app tauri build --no-bundle` passed: release 2m53s, Vite 10.98s,
App 499.96kB unchanged. First-party compiler warnings and Vite advisories remain
zero (baseline zero, delta zero). The exact-path build preflight stopped only
the prior checkout process, PID 98136.

Launched the new exact checkout executable, PID 108972, SHA256
`1ACE2D233024EF7F616F97C7CFFFD0716B63CFDB85EF0C8A9E931BF5DE9FD743`.
One responsive maximized Syndocal main window was verified. MCP negotiated five
tools and completed fixture-list/runtime-status reads, zero mutations. The
empty E0/R1 project, no outputs/fixtures, idle Follow and StartupDenied output
ownership remain unchanged. Native logs, launch identity and read-only MCP
proof are `native-build.log`, `native-launch.json`, `native-mcp-probe.json` in
`target/qa/recording-runtime-20260906/`. User-show/Unity acceptance and the full
recording cancellation/deadline boundary remain open. The unrelated viewport
checker is preserved unchanged; only owned files belong to this checkpoint.
