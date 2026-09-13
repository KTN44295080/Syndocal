# Physical Input Current-Source Checkpoint — 2026-09-14

## Scope and authority

- Flow marker: `INPUT-PHYSICAL-001` (section 8, Open)
- Q1 row: `COV-INPUT-001`
- Branch: `codex/showclock-review-20260912`
- Base: `12090100ca713bfbc1c5bb0f04c59e937c320891`
- Authority: `qa/M4_IO_VALIDATION.md` and the F1 input-generation contracts

This checkpoint covers current-source input routing and generation guards. It
does not claim the physical MIDI/OSC/Remote client matrix.

## Verification

```text
pnpm.cmd --dir app run check:dvc-midi-shortcuts
node app/scripts/check-dvc-dmx-shortcuts.mjs
pnpm.cmd --dir app run check:frontend-command-routing
```

Result: exit code 0.

- DVC MIDI shortcuts: `39 assertions`, PASS.
- DVC DMX shortcuts: `41 assertions`, PASS.
- Frontend command routing: `133 renderer mutations`, `31 server-authoritative
  mutations`, `28 raw dispatches`, `479 facade dispatches`.
- Current source preserves generation/authority routing and the narrow input
  reader boundary.
- First-party warning count observed in this focused source run: `0`.

## Unresolved acceptance

`INPUT-PHYSICAL-001` stays Open. The required physical MIDI, OSC, Remote,
reconnect, latency, feedback/clock, and native device acceptance must be run
with named clients and devices. Static route inventories cannot establish
controller movement, LED/clock/MTC behavior, or physical recovery.

Next action is the named physical input matrix with device/client versions,
generation transitions, reconnect timing, raw logs, and operator observations.
