# Physical Input Current-Source Checkpoint — 2026-09-14

## Scope and authority

- Flow marker: `INPUT-PHYSICAL-001` (section 8, Open)
- Q1 row: `COV-INPUT-001`
- Branch: `codex/showclock-review-20260912`
- Base: `a192125b8da4378cc952ef69d70baf87130fd6fb`
- Authority: `qa/M4_IO_VALIDATION.md` and the F1 input-generation contracts

This checkpoint covers current-source input routing and generation guards plus
one safe current-host physical MIDI enumerate/open/feedback slice. It does not
claim the complete physical MIDI/OSC/Remote client matrix.

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

## Takeover source rerun — 2026-09-14

The DVC MIDI shortcut check passed 39 assertions, the DVC DMX shortcut check
passed 41 assertions, and frontend command routing passed with 133 renderer,
31 server-authoritative, 28 raw, and 479 facade dispatches. This rerun did not
repeat the physical device operation; the bounded SMC-Mixer enumerate/open and
safe All Notes Off result above remains the only current-host hardware slice.

## Current-host physical MIDI recheck — 2026-09-14

With the pinned MSVC `14.44.35207` x64 linker and
`SYNDOCAL_TEST_MIDI_INPUT=SMC-Mixer` / `SYNDOCAL_TEST_MIDI_OUTPUT=SMC-Mixer`,
the release-mode ignored test
`physical_midi_ports_enumerate_open_and_send_feedback` passed:
`1 passed / 0 failed / 0 ignored` in `0.37s` after the release build.
Production `midir` enumerated input `SMC-Mixer` at index `1` and output
`SMC-Mixer` at index `2`, opened the clock input and feedback output, and sent
one safe channel-1 All Notes Off message `B0 7B 00`. The full observed lists
were inputs `CustomMIDI1`, `SMC-Mixer`, `MIDIIN2 (SMC-Mixer)` and outputs
`Microsoft GS Wavetable Synth`, `CustomMIDI1`, `SMC-Mixer`,
`MIDIOUT2 (SMC-Mixer)`.

This is a current-source physical transport slice only. It does not prove
controller movement, LED/clock/MTC behavior, latency, reconnect/replacement,
OSC/TouchOSC, Web Remote, native UI routing, DMX, or venue acceptance.

## Unresolved acceptance

`INPUT-PHYSICAL-001` stays Open. The required physical MIDI, OSC, Remote,
reconnect, latency, feedback/clock, and native device acceptance must be run
with named clients and devices. Static route inventories cannot establish
controller movement, LED/clock/MTC behavior, or physical recovery.

Next action is the named physical input matrix with device/client versions,
generation transitions, reconnect timing, raw logs, and operator observations.
