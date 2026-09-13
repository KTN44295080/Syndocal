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
