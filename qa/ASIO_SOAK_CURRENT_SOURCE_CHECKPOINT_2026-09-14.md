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
