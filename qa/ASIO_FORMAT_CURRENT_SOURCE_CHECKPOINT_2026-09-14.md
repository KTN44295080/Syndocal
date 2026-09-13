# ASIO Format Matrix Current-Source Checkpoint — 2026-09-14

## Scope and authority

- Flow marker: `ASIO-FORMAT-MATRIX-001` (section 7, Open)
- Q1 row: `COV-AUDIO-LIVE-001`
- Branch: `codex/showclock-review-20260912`
- Base: `b22a950bd937cefa811612837fad82cdb57848b8`
- Authority: `qa/ASIO_INPUT_ACCEPTANCE.md`

This checkpoint covers current-source admission, request mapping, persistence,
and ABI contract behavior. It does not claim the required real-driver matrix.

## Current-source result

The source requires an explicit ASIO device and revalidates the requested sample
rate, stream channels, native sample format, and fixed buffer before start. It
rejects malformed, unsupported, stale, or mismatched values and does not
substitute another driver or WASAPI. Selection and IPC contracts keep the
fields explicit rather than silently applying a default.

## Verification

```text
pnpm.cmd --dir app run check:live-audio
pnpm.cmd --dir app run check:live-audio-ipc-v1
pnpm.cmd --dir app run check:asio-v3-contract
```

Result: exit code 0.

- Live-audio fail-closed lifecycle, availability, selection persistence,
  presentation, and request ordering: PASS.
- Live-audio IPC v1 exact request mapping and fail-closed checks: PASS.
- ASIO v3 contract: `22 assertions`, PASS.
- The checks cover explicit configuration and invalid-input rejection in the
  current source; they do not enumerate real drivers or open every advertised
  rate/buffer/format/channel combination.
- First-party warning count observed in this focused source run: `0`.

## Takeover rerun — 2026-09-14

The live-audio lifecycle, live-audio IPC v1, and ASIO ABI-v3 checks were rerun
after takeover and all passed. The source still requires explicit device,
sample-rate, channel, native-format, and fixed-buffer values; no fallback was
introduced. No real ASIO driver was opened and no format-matrix acceptance
artifact was produced.

## Unresolved acceptance

`ASIO-FORMAT-MATRIX-001` stays Open. The required 44.1/48/96 kHz,
64/128/256-frame, native-format, mono/stereo/channel matrix must be exercised
with the licensed artifact and real ASIO drivers, recording applied values and
proving no fallback. Current source checks cannot replace that native external
acceptance.

Next action is the named-device matrix on the approved artifact, followed by
raw logs bound to the exact executable, bridge, driver, operator, and date.
