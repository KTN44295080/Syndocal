# Live audio software revalidation — 2026-09-12

This checkpoint advances only the current deterministic software slice of
`COV-AUDIO-LIVE-001` on `main` at
`917084057f14e54d85911ead62c0fab41cb563c1`. No product source was changed.

| Check | Result |
| --- | --- |
| `pnpm --dir app run check:live-audio` | PASS — fail-closed lifecycle, availability, exact selection persistence and request ordering |
| `pnpm --dir app run check:live-audio-ipc-v1` | PASS — exact IPC mapping and rejection cases |
| `pnpm --dir app run check:asio-packaging` | PASS — 169 packaging-boundary assertions; SDK-independent bundle mode explicitly stayed unbundled |
| `pnpm --dir app run check:asio-v3-contract` | PASS — 22 contract assertions |
| `cargo test ... live_audio_input_tests` | PASS — 95 passed, 0 failed, 0 ignored, 1771 filtered; exact MSVC 14.44.35207 linker |
| Engine TTL expiry test | PASS — 1 passed, 0 failed |
| Engine unverified-legacy rejection test | PASS — 1 passed, 0 failed |

The app tests cover generation/selection fencing, stale catalog and device
ABA rejection, callback bounds/format sanitation, stop/fault ownership,
ASIO separation, stale watchdog recovery, and reactive-feature clearing.
The Engine tests cover TTL expiry and rejection of unverified legacy audio.

This does not close the ASIO row: current final DLL/native operator loading,
license/artifact decision, full advertised rate/buffer/channel matrix,
occupied/reset/unplug/XRUN faults, matched one-hour soak, physical
input-to-pixel latency, or TouchDesigner parity remain open. A browser restore
gate was not run because this PC has no Chrome/Edge executable at the checker
paths. `COV-AUDIO-LIVE-001` remains `In progress`.

## Current-main rerun — 2026-09-12

The live-audio software gate was rerun against current `main` at
`b5af9ea1c43041a3a958f09a94b17cdccbd4e4c6`, with the documented MSVC
14.44.35207 x64 absolute linker pin confirmed first by `where.exe link.exe`.

All seven command groups passed: live-audio lifecycle, IPC v1, ASIO packaging
(169 assertions), ASIO v3 contract (22 assertions), Tauri live-audio input
(95 passed, 0 failed, 0 ignored), engine TTL expiry (1/1), and unverified
legacy rejection (1/1). No ASIO/WASAPI device, one-hour soak, physical input,
browser, or venue acceptance was used.
