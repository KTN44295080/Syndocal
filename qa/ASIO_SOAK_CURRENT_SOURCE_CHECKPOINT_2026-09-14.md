# ASIO Soak Current-Source Checkpoint — 2026-09-14

## Scope and authority

- Flow marker: `ASIO-SOAK-001` (section 7, Open)
- Q1 row: `COV-AUDIO-LIVE-001`
- Branch: `codex/showclock-review-20260912`
- Base: `10b1273a230c5214963256d232868af71a135fa6`
- Authority: `qa/ASIO_INPUT_ACCEPTANCE.md`

This checkpoint records the current software lifecycle proof and the explicit
absence of a long-duration device result. No one-hour hardware claim is made.

## Verification

```text
pnpm.cmd --dir app run check:live-audio
```

Result: exit code 0. The current-source fail-closed lifecycle, availability,
selection persistence, presentation, and request-ordering checks passed.

## Takeover rerun — 2026-09-14

The live-audio lifecycle check was rerun after takeover and passed. It again
covered fail-closed availability, explicit selection persistence, presentation,
and request ordering only; no ASIO/WASAPI device was opened and no
long-duration soak or latency artifact was produced.

The SDK-free bridge fault suite and ABI contract evidence are retained in
`qa/ASIO_FAULT_CURRENT_SOURCE_CHECKPOINT_2026-09-14.md`; they are deterministic
contract evidence, not a soak substitute. A parallel Cargo retry was stopped
after it became a duplicate build-lock wait and is not counted as evidence.

## Unresolved acceptance

`ASIO-SOAK-001` stays Open. The required matched one-hour ASIO and WASAPI run
still needs real devices, overrun/XRUN telemetry, callback p99 and maximum
budgets, capture-to-engine p95, and loss-to-zero timing bound to the exact
artifact and operator. Source tests cannot establish thermal, clock-drift,
driver, or long-duration behavior.

Next action is a matched one-hour ASIO/WASAPI run on the approved artifact with
raw time-series logs and the stated thresholds recorded.

## Takeover continuation — current-source soak preflight — 2026-09-14

At HEAD `767f1318`, `pnpm.cmd --dir app run check:live-audio` passed the
fail-closed lifecycle, availability, explicit selection persistence,
presentation, and request-ordering contract. No ASIO or WASAPI device was
opened and no long-duration stream, thermal, clock-drift, callback, or latency
artifact was produced. `ASIO-SOAK-001` remains `Open` pending the matched
one-hour physical ASIO/WASAPI run.
