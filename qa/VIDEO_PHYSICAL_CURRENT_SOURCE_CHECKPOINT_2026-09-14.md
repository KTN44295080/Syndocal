# Video Physical Current-Source Checkpoint — 2026-09-14

## Scope and authority

- Flow marker: `VIDEO-PHYSICAL-001` (section 8, Open)
- Q1 row: `COV-OUTPUT-LOCAL-001`
- Branch: `codex/showclock-review-20260912`
- Base: `a44f43de4293666ddfe3506c5dee15df2c32bffe`
- Authority: `qa/M5_RELIABILITY_VALIDATION.md` and the output-control contracts

This checkpoint covers current-source video output routing, managed-window
lifecycle, and bounded polling. It does not claim physical display, capture,
NDI, Spout, or camera acceptance.

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
valid arrays. No display, capture device, NDI/Spout receiver, or physical
output was used.

## Unresolved acceptance

`VIDEO-PHYSICAL-001` stays Open. The required real display topology, HDMI/video
output, NDI/Spout receiver, camera/capture-fault, reconnect, frame-drop, and
one-hour acceptance must be exercised with named hardware and raw observations.
Current source and window lifecycle checks cannot prove pixels reached a real
display or receiver.

Next action is the physical display/capture matrix with exact resolution,
receiver, source, frame-rate, reconnect, and dropped-frame records.
