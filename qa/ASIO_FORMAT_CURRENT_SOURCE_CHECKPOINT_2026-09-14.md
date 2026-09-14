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

## Takeover continuation — current-source format-contract recheck — 2026-09-14

At HEAD `8aba7087`, the live-audio fail-closed lifecycle/availability/
persistence/presentation/order check, live-audio IPC v1 exact mapping check,
and ASIO v3 contract check all passed. The ABI check reported `22` assertions.
No ASIO stream or real driver was opened, and no sample-rate/buffer/channel
matrix artifact was produced. `ASIO-FORMAT-MATRIX-001` remains `Open` pending
the licensed artifact and named real-driver matrix.

## Current HEAD format-contract recheck — 2026-09-14

At current HEAD `0e2a4366`, the live-audio fail-closed lifecycle,
availability, selection persistence, presentation, and request-order checks
passed; the live-audio IPC v1 exact request-mapping checks passed; and the
ASIO v3 contract passed 22 assertions.

With the exact MSVC `14.44.35207` Build Tools linker initialized and printed
first in `where.exe link.exe`, the SDK-free `tools/asio-bridge` suite
passed 31 tests with zero failures or ignored tests. It covered explicit
tuple validation, no-fallback behavior, lease ownership, v2/v3 exclusivity,
strict JSON, native-format conversion, callback frame integrity, full
silence on rejected data, and typed Stop/Close teardown.

The current host exposes ASIO registry entries and USB audio devices, but the
pinned SDK archive and reviewed ASIO artifact are not present in this checkout.
No ASIO stream was opened and no 44.1/48/96 kHz, 64/128/256-frame,
native-format, mono/stereo/channel hardware matrix was produced.
`ASIO-FORMAT-MATRIX-001` remains `Open` pending the licensed artifact and
named real-driver evidence.

## Takeover continuation — current-source format-contract recheck after Video repair — 2026-09-14

At current source HEAD `881f637e`, the source-side ASIO format contracts were
rerun:

```text
pnpm.cmd run check:live-audio
live audio fail-closed lifecycle, availability contract, selection persistence,
presentation, and request ordering ok

pnpm.cmd run check:live-audio-ipc-v1
live audio IPC v1 exact request mapping and fail-closed checks ok

pnpm.cmd run check:asio-v3-contract
ASIO v3 contract tests passed: 22 assertions
```

All commands exited `0`. Explicit device, sample-rate, channel, native-format,
and fixed-buffer admission remains fail-closed with exact IPC mapping and the
ABI-v3 contract intact. No ASIO stream or real driver was opened, and no
44.1/48/96 kHz, 64/128/256-frame, native-format, mono/stereo/channel matrix
artifact was produced. `ASIO-FORMAT-MATRIX-001` remains `Open` pending the
licensed artifact and named real-driver matrix with recorded applied values and
no-fallback proof.

## Continuation — current-source format-contract recheck — 2026-09-15

At current source HEAD `bf004b6d`, with the exact MSVC `14.44.35207` x64 linker
confirmed first by `where.exe link.exe`, the following current-source checks
exited `0`:

```text
pnpm.cmd --dir app run check:live-audio
pnpm.cmd --dir app run check:live-audio-ipc-v1
pnpm.cmd --dir app run check:asio-v3-contract
```

The source continues to require explicit device, sample-rate, channel,
native-format, and fixed-buffer values with exact IPC mapping and no fallback;
ABI-v3 reported 22 assertions. No ASIO stream or real driver was opened, and
no advertised rate/buffer/format/channel hardware matrix was produced.
`ASIO-FORMAT-MATRIX-001` remains **Open**.
