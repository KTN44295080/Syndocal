# Recording renderer lock cancellation

Base `c212f08f7f53fc529946b084e96336c77bd2e70a`, branch
`codex/syndocal-v1.2`, alpha.69 unchanged.

The recording loop previously called the shared renderer's blocking `lock()`.
If another consumer held it, Stop could not be observed until that consumer
released it. `video_recording_renderer_access.rs` now uses `try_lock`, checks
Stop before each attempt and after acquisition, and sleeps 5 ms only while
contended. An uncontended frame performs no sleep, allocation or thread spawn.
The guard remains local to the synchronous rendering scope. A poisoned mutex
now enters the existing explicit recording failure/owned encoder cleanup path
instead of silently dropping frames forever.

Pinned MSVC test `recording_renderer_`: 2 passed, 1779 filtered, 0 warnings
(previous accepted compiler warning baseline 0, delta 0). Evidence:
`target/qa/mapping-group-undo-20260906/renderer-access-tests.log`.
The held-lock test synchronizes after an actual WouldBlock and receives stopped
completion while the other guard is still held. Value access, poison rejection,
and stopped access are also covered. Its caught poison-test panic is expected.
Independent source/test review accepted the change.

This improves cancellation while waiting for the renderer mutex. It does not
bound scheduler latency, snapshot capture, an already running render, inherited
pipe I/O or total worker shutdown. Worker and child ownership remain retained.
Native release build and startup passed with the group Undo checkpoint.
See [native evidence](MAPPING_GROUP_UNDO_2026-09-06.md). No hardware output was exercised.

## 2026-09-07 cooperative active-render cancellation

The recording owner now passes a `VideoRenderCancellation` callback through the
existing output-preview pipeline. The cancellation contract is kept in
`crates/video/src/video_render_cancellation.rs`; FFmpeg CLI process polling,
termination and reader joining are kept in
`crates/video/src/ffmpeg_cancellable_process.rs`. The central renderer keeps
one canonical implementation: the existing non-cancellable API delegates with
no callback, so the normal decode path retains synchronous `output()` behavior
and does not create the cancellation reader threads. The cancellable external
fallback creates its two readers only for the operation that needs draining.

Cancellation is checked before and between input/decode, layer, composition,
transition and effect stages. A cancelled direct FFmpeg child is killed,
waited, and its stdout/stderr readers are joined before the cancellation is
returned. No frame queue or detached worker was added. In-process HAP/libav
decode and one individual CPU/GPU/effect operation remain synchronous; they are
checked before and after the operation, not forcibly interrupted. A process
tree whose descendants retain inherited pipes and an OS-level hard total-stop
deadline therefore remain outside this checkpoint.

Verification on this checkout:

- `cargo test -p video --locked`: the two new cancellation contract tests and
  the cancellable process termination/reap test passed; the full run was
  `171 passed, 3 failed, 3 ignored`. The three failures are existing output
  routing/GPU comparison failures outside this diff; the normal legacy path was
  not changed semantically by the cancellation seam.
- `cargo check -p syndocal --locked` passed with no first-party warnings.
- The recording filter passed `54 passed, 6 ignored, 0 failed` under the exact
  Build Tools MSVC 14.44.35207 linker. `pnpm.cmd --dir app run check:release`
  also passed. No physical output, stream or recording was activated.
- `pnpm.cmd --dir app tauri build --no-bundle` passed under the same linker.
  Fresh `target/release/syndocal.exe` is version `1.2.0-alpha.69`, SHA-256
  `C753D6F8CE250D645D5176904F1FA139146B0FC7D680417F7087D0888F1B3886`.
  The exact-checkout launch check observed one responsive, maximized window;
  `physicalOutputOperations=0`.

## 2026-09-07 bounded worker teardown and process-tree completion

This section supersedes the unresolved teardown statements above for the
current source. The recording lifecycle now gives the worker a ten-second
total-stop deadline. If a renderer or encoder worker is still inside an
in-process operation at that deadline, its original `JoinHandle` is transferred
to a named, owned reaper; it is never detached or forgotten. The caller returns
with the lifecycle still occupied, so a new recording cannot overlap the live
worker. Later lifecycle polling reaps the deferred result.

The direct FFmpeg child is placed in a Windows Job Object with
`KILL_ON_JOB_CLOSE`. Its stdin and stderr owners retain the job guard until
their handles are closed, so descendants that inherit either pipe remain part
of the owned cleanup boundary. A descendant regression test now proves that a
failure tears down the child process tree, not only the direct child. FFmpeg
supervision still confirms `try_wait` termination before reporting cleanup.

The deadline bounds caller return and ownership transfer. Safe Rust cannot
forcibly interrupt an arbitrary synchronous HAP/libav/GPU/effect operation, so
those operations remain cooperative at their stage boundaries; claiming a
physical hard-kill of an in-process thread would be unsafe. Real MP4 and the
synthetic 30-minute A/V acceptance both pass, while full in-app/hardware
recording remains a separate acceptance boundary.
