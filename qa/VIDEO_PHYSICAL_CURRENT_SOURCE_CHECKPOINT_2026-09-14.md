# Video Physical Current-Source Checkpoint — 2026-09-14

## Scope and authority

- Flow marker: `VIDEO-PHYSICAL-001` (section 8, Open)
- Q1 row: `COV-OUTPUT-LOCAL-001`
- Branch: `codex/showclock-review-20260912`
- Base: `a44f43de4293666ddfe3506c5dee15df2c32bffe`
- Authority: `qa/M5_RELIABILITY_VALIDATION.md` and the output-control contracts

This checkpoint covers current-source video output routing, managed-window
lifecycle, bounded polling, and a current-host DirectShow camera
capture/restart slice. It does not claim physical display, HDMI output,
NDI/Spout receiver, or full video physical acceptance.

## Verification

```text
pnpm.cmd --dir app run check:video-output-routing-runtime
pnpm.cmd --dir app run check:video-output-window-runtime
pnpm.cmd --dir app run check:video-output-window-observation
pnpm.cmd --dir app run check:video-runtime-polling
```

Result: exit code 0.

- Video output routing R4 contract: PASS.
- Managed video-output window runtime: PASS, including exact-Both recovery,
  receipt rejection, singleflight, incarnation reducer, and zero legacy invokes.
- Video-output window observation contract: PASS.
- Video runtime polling: PASS for bounded current/transition requests,
  malformed-response rejection, generation retention, and zero-copy valid arrays.
- First-party warning count observed in this focused source run: `0`.

## Takeover rerun — 2026-09-14

The routing, managed-window runtime, window observation, and video polling
checks were rerun after takeover and passed. They again verified exact-Both
recovery, receipt rejection, singleflight, incarnation fencing, bounded
polling, malformed-response rejection, generation retention, and zero-copy
valid arrays. No display, HDMI output, NDI/Spout receiver, or physical output
was used in this contract rerun.

## Current-host physical camera slice — 2026-09-14

The production capture worker was exercised against the currently enumerated
`ASUS 5M webcam` through the canonical DirectShow profile endpoint. The
selected profile was the exact device alternative identity
`@device_pnp_\\?\usb#vid_636e&pid_0bda&mi_00#7&154dae8b&0&0000#{65e8773d-8f56-11d0-a3b9-00a0c9223196}\global`,
`nv12`, `1280x720`, `30/1`. FFmpeg was the explicit local
`C:\Users\janua\AppData\Local\SyndocalDev\ffmpeg-n8.1-latest-win64-lgpl-shared-8.1\bin\ffmpeg.exe`.

With the pinned MSVC `14.44.35207` x64 linker, the ignored test
`camera_worker_captures_a_real_frame_and_restarts_cleanly` passed
`1 passed; 0 failed; 0 ignored`. It completed two worker start/stop attempts,
received the expected `1280x720` RGBA frame on each attempt, and rejected a
fully transparent frame. This is a partial physical capture/restart result;
it does not prove unplug/replug, HDMI/display pixels, NDI/Spout, frame drops,
one-hour, venue, or product completion.

## Unresolved acceptance

`VIDEO-PHYSICAL-001` stays Open. The required real display topology, HDMI/video
output, NDI/Spout receiver, camera fault/unplug-replug, reconnect, frame-drop,
and one-hour acceptance must be exercised with named hardware and raw
observations. The camera slice above proves only two clean current-source
capture/restart cycles; it cannot prove pixels reached a real display or
receiver.

Next action is the physical display/capture matrix with exact resolution,
receiver, source, frame-rate, reconnect, and dropped-frame records.

## Takeover continuation — current-source video-output recheck — 2026-09-14

At HEAD `d9908a8f`, video output routing, managed output-window runtime,
output-window observation, and video runtime polling all passed. The runtime
again confirmed exact-Both recovery, receipt rejection, singleflight,
incarnation fencing, malformed-response rejection, generation retention, and
zero-copy valid arrays. No display/HDMI output, NDI/Spout receiver, or new
camera capture was opened. The existing ASUS webcam two-cycle slice remains
partial evidence; `VIDEO-PHYSICAL-001` stays `Open`.
