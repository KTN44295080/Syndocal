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

## Takeover continuation — current-source USB-DMX/RDM recheck after Video repair — 2026-09-14

At current source HEAD `e828b6d2`, the focused output contracts were rerun:

```text
pnpm.cmd run check:output-control-runtime
output control runtime contract: PASS (v12 output commands, lease-bound
Lighting/Video master, Video Take/Clip Launch and group controls, fixed same-PC
Art-Net loopback/DSF2026 probe plus no-send reconciliation/strict Spout V2
receipt fences/reset, revision-fenced USB-DMX status, strict receipts,
fail-closed query)
Standby Sync output-lease UI contract passed.

pnpm.cmd run check:output-ownership
output ownership static contract: PASS

pnpm.cmd run check:safety-blackout-runtime
safety blackout runtime contract: PASS
```

All three commands exited `0`. A read-only Windows PnP inventory exposed only:

```text
OK  Ports  USB Serial Port (COM5)  FTDIBUS\VID_0403+PID_6001+6&1824F623&0&2\0000
```

No Enttec, DMXKing, or named RDM interface was present. The generic serial
device was not opened; no USB-DMX/RDM bytes, E1.20 discovery, ACK/NACK/timeout
log, two-fixture capture, or analyzer artifact was produced.
`DMX-USB-RDM-001` remains `Open` pending the required named gateway/interface
and two-fixture external matrix.

## Continuation — current-source USB-DMX/RDM boundary recheck — 2026-09-15

At current source HEAD `33ad01d4`, with the exact MSVC `14.44.35207` x64 linker
confirmed first by `where.exe link.exe`, the output-control runtime,
ownership, and safety contracts all exited `0`:

```text
pnpm.cmd --dir app run check:output-control-runtime
pnpm.cmd --dir app run check:output-ownership
pnpm.cmd --dir app run check:safety-blackout-runtime
```

No Enttec, DMXKing, or named RDM interface was opened; no USB-DMX/RDM bytes,
E1.20 discovery, ACK/NACK/timeout log, fixture capture, or analyzer artifact
was produced. `DMX-USB-RDM-001` remains **Open**.

## Current native COM5 preparation and restart recovery — 2026-09-15

The current release executable was loaded with
`samples/phase1-mini-show.sdc`. The native I/O surface enumerated and selected
`COM5 · FTDI / USB Serial Port (COM5)`. The user-authorized `Prepare` action was
run through the real UI and its confirmation dialog was accepted. The flow
reached `4/4 optional Open DMX arm` and then stopped with:

```text
Error: OutputControl rejected (publication_failed);
physical output state is unknown.
```

No fixture receipt, wire capture, or analyzer result was obtained. A subsequent
native `All Blackout` action was rejected as `invalid_request` with
`nothing was applied`; the keyboard-only retry was rejected as `forbidden`
with `output was not applied; refresh lease state`. These are fail-closed
observations and are not physical blackout acceptance.

For recovery, only the exact release executable path was closed. The native
unsaved-change dialog was answered `Discard and Close`; no force termination
was needed. The same executable was restarted and was responsive. It returned
to `Ready`; after reloading the sample, COM5 was again visible with `Prepare`
available, while the prior error was not carried into the new process. This
proves application restart recovery from the observed UI failure only; it does
not prove physical serial-DMX recovery or a known electrical output state.

`DMX-USB-RDM-001` remains **Open** pending a named Enttec/DMXKing interface,
physical zero receipt, fixture/wire evidence, Open DMX timing, and the required
two-fixture RDM/TOD matrix.
