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
