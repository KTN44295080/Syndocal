# DJ Link Hardware Current-Source Checkpoint — 2026-09-14

## Scope and authority

- Flow marker: `DJ-LINK-HARDWARE-001` (section 8, Open)
- Q1 row: `COV-REMOTE-TOUCH-001`
- Branch: `codex/showclock-review-20260912`
- Base: `4467750a2c01145e6fc068749e271f4515c5f20d`
- Authority: `qa/REKORDBOX_STREAM_DECK_PEDAL_ACCEPTANCE.md`

This checkpoint covers the current-source DJ Link policy/runtime contracts. It
does not claim the live DJ-PC, rekordbox, Agent, Pedal, or wired-LAN matrix.

## Verification

```text
pnpm.cmd --dir app run check:dj-link
```

Result: exit code 0.

- DJ track mapping policy checks: PASS.
- DJ Link frontend contract checks: PASS.
- The current source keeps peer identity, measured-loop authority, Release,
  reconnect fencing, and mapping behavior in the documented route; the source
  check does not start an external Agent or send pedal/MIDI traffic.
- First-party warning count observed in this focused source run: `0`.

## Unresolved acceptance

`DJ-LINK-HARDWARE-001` stays Open. The required physical Pedal/DJ-PC/Agent and
rekordbox topology must verify HELLO/ACK/STATE_SYNC, track/position/loop,
Release, disconnect/reconnect, restart, and exact mapping behavior with named
versions, tokens, NICs, and raw logs. Existing source checks cannot replace the
0/12 hardware submatrix.

Next action is the live peer and pedal acceptance run, bound to the exact
Syndocal and Agent artifacts and operator observations.
