# ASIO Latency Current-Source Checkpoint — 2026-09-14

## Scope and authority

- Flow marker: `ASIO-LATENCY-001` (section 7, Open)
- Q1 row: `COV-AUDIO-LIVE-001`
- Branch: `codex/showclock-review-20260912`
- Base: `07fe7caa7a98be597b70a30a032e9a4b5ab0056b`
- Authority: `qa/ASIO_INPUT_ACCEPTANCE.md`

This checkpoint records the current source lifecycle contract only. It contains
no physical latency measurement and no TouchDesigner acceptance claim.

## Verification

```text
pnpm.cmd --dir app run check:live-audio
```

Result: exit code 0. The fail-closed live-audio lifecycle, availability,
selection persistence, presentation, and request-ordering source checks passed.

## Takeover rerun — 2026-09-14

The current live-audio lifecycle check was rerun after takeover and passed.
This confirms source-side availability, selection persistence, presentation,
and request ordering only; no ASIO stream, TouchDesigner session, physical
input, or input-to-engine/input-to-pixel measurement was performed.

The implementation exposes capture/worker telemetry fields and preserves the
ShowClock/output ownership boundaries, but source presence is not a measured
input-to-pixel result.

## Unresolved acceptance

`ASIO-LATENCY-001` stays Open. The required physical input-to-pixel measurement
and five matched TouchDesigner trials need the approved ASIO artifact, named
driver/device, marker or equivalent measurement source, exact configuration,
and raw timing logs. No browser, source, or short native smoke can substitute
for that external measurement.

Next action is five matched physical trials with capture-to-engine and
input-to-pixel percentiles recorded against the acceptance thresholds.

## Takeover continuation — current-source latency preflight — 2026-09-14

At HEAD `5a612989`, `pnpm.cmd --dir app run check:live-audio` passed the
fail-closed lifecycle, availability, explicit selection persistence,
presentation, and request-ordering contract. The source exposes typed capture
and engine telemetry fields, but no physical stream, marker, TouchDesigner
session, or input-to-pixel measurement was performed. `ASIO-LATENCY-001`
remains `Open` pending five matched physical trials with raw timing logs.

## Takeover continuation — current-source latency-contract recheck — 2026-09-14

At current source HEAD `0d216dc2`, the source-side contracts were rerun:

```text
pnpm.cmd --dir app run check:live-audio
live audio fail-closed lifecycle, availability contract, selection persistence,
presentation, and request ordering ok

pnpm.cmd --dir app run check:live-audio-ipc-v1
live audio IPC v1 exact request mapping and fail-closed checks ok

pnpm.cmd --dir app run check:asio-v3-contract
ASIO v3 contract tests passed: 22 assertions.
```

These checks confirm the typed capture/engine telemetry surface, exact IPC
request mapping, generation and fail-closed rules, and ASIO v3 event contract.
They did not open a physical ASIO/WASAPI stream, route a marker through
TouchDesigner, render a measured output pixel, or create raw timing logs.

`ASIO-LATENCY-001` remains `Open`. Completion still requires five matched
physical trials on the approved artifact and named driver/device, with exact
sample rate/channels/format/buffer configuration and raw capture-to-engine and
input-to-pixel percentiles against the acceptance thresholds. Source or
browser checks cannot substitute for that external measurement.

## Takeover continuation — current-source latency-contract recheck after Video repair — 2026-09-14

At current source HEAD `0a114672`, the live-audio lifecycle check was rerun:

```text
pnpm.cmd run check:live-audio
live audio fail-closed lifecycle, availability contract, selection persistence,
presentation, and request ordering ok
```

The command exited `0`. It reconfirms the typed capture/engine-adjacent
fail-closed lifecycle, explicit selection persistence, presentation, and
request-ordering contracts only. No physical ASIO/WASAPI stream, marker,
TouchDesigner session, measured output pixel, or raw timing log was created.
`ASIO-LATENCY-001` remains `Open`; completion still requires five matched
physical trials on the approved artifact and named driver/device with exact
configuration and capture-to-engine/input-to-pixel percentiles against the
acceptance thresholds.
