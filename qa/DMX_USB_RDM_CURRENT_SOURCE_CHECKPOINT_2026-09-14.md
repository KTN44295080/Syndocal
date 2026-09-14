# DMX USB/RDM Current-Source Checkpoint — 2026-09-14

## Scope and authority

- Flow marker: `DMX-USB-RDM-001` (section 8, Open)
- Q1 row: `COV-OUTPUT-LOCAL-001`
- Branch: `codex/showclock-review-20260912`
- Base: `fb6c169185f6fc34b6a0ea818ad1efd2605cdefa`
- Authority: `qa/M4_IO_VALIDATION.md`

This checkpoint covers the current USB-DMX/RDM control and fail-closed source
contracts. It does not claim RDM hardware acceptance or analyzer capture.

## Verification

```text
pnpm.cmd --dir app run check:output-control-runtime
pnpm.cmd --dir app run check:output-ownership
pnpm.cmd --dir app run check:safety-blackout-runtime
```

Result: exit code 0.

- Output-control runtime contract: PASS, including revision-fenced USB-DMX
  status, strict receipts, S0/zero-first behavior, bounded shutdown, stale
  status invalidation, exact USB-DMX identity, and no bypass route.
- Standby Sync output-lease UI contract: PASS.
- Output ownership static contract: PASS.
- Safety blackout runtime contract: PASS.
- First-party warning count observed in this focused source run: `0`.

## Takeover rerun and host inventory — 2026-09-14

The output-control, output-ownership, and safety-blackout checks were rerun
after takeover and passed. The current PnP inventory exposed a generic
`USB Serial Port (COM5)` / FTDI device, but no present device named Enttec,
DMXKing, or RDM. This inventory is not a protocol or fixture proof; the
generic serial device was not opened and no USB-DMX/RDM bytes or analyzer
capture were produced.

## Unresolved acceptance

`DMX-USB-RDM-001` stays Open. The required Enttec/DMXKing hardware path needs
E1.20, ArtRdm/TOD, USB Pro labels, ACK overflow/timer, queued messages,
discovery splitting, timeout/collision-safe parser behavior on a gateway or USB
interface with at least two RDM fixtures, plus inventory churn and analyzer
capture. Static contracts and the historical serial-DMX slice cannot replace
that external evidence.

Next action is the two-fixture RDM discovery/request matrix with firmware,
fixture UIDs, raw packets, ACK/NACK/timeout logs, and inventory timing.

## Takeover continuation — current-source USB-DMX/RDM recheck — 2026-09-14

At HEAD `94cbe220`, output-control/Standby Sync, output ownership, and
safety-blackout runtime contracts all passed. The read-only current PnP Ports
inventory exposed only a generic `USB Serial Port (COM5)` FTDI device; no
Enttec, DMXKing, or named RDM interface was present. The generic device was
not opened and no USB-DMX/RDM bytes or analyzer capture were produced.
`DMX-USB-RDM-001` remains `Open` for the required two-fixture external matrix.
