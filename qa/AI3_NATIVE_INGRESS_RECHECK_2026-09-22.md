# AI3 native ingress current-host follow-up — 2026-09-22

- Marker: `AI3-NATIVE-INGRESS-001` — remains **Open**.
- Branch: `codex/showclock-review-20260912`.
- Source HEAD: `2cc570a221720e8453f8d8fb9af0e64e7dbf3681`.
- Scope: current SMC-Mixer device/endpoint inventory and one 60-second,
  receive-only production `midir` capture. No product code or device settings
  were changed.

## Current device state

Windows PnP showed the SMC-Mixer USB device (`USB\\VID_4353&PID_4B4D`) and its
MIDI endpoints present. `SMC-Mixer` and `MIDIIN2 (SMC-Mixer)` share that USB
parent/container. `SMC-Mixer-bt` is a separate Bome virtual endpoint. The live
BTMidiConnector accessibility tree showed `SMC-Mixer Connected.`

The exact-checkout Syndocal window showed Setup > I/O with the MIDI summary
`Stopped`; its Audio detail panel remained selected. Native navigation to the
MIDI details did not succeed, so the selected input and app-side mapping state
were not verified. No input setting or mapping was changed.

## Production WinMM endpoint inventory

With the repository-required Build Tools MSVC `14.44.35207` x64 linker pinned
and first in `where.exe link.exe`, a deliberate nonmatching-selector run of
`physical_midi_ports_enumerate_open_and_send_feedback` printed:

```text
MIDI inputs: 0 CustomMIDI1, 1 SMC-Mixer, 2 MIDIIN2 (SMC-Mixer), 3 SMC-Mixer-bt
MIDI outputs: 0 Microsoft GS Wavetable Synth, 1 CustomMIDI1, 2 SMC-Mixer,
              3 MIDIOUT2 (SMC-Mixer), 4 SMC-Mixer-bt
no MIDI input matched '__CODEX_ENUM_ONLY_NO_MATCH_INPUT__'
test exit_code=101
```

The intentional no-match stopped before opening an endpoint or sending MIDI.
It is an inventory observation, not a successful ingress test.

## Receive-only operator ingress attempt

```text
SYNDOCAL_TEST_MIDI_INPUT=SMC-Mixer
SYNDOCAL_TEST_MIDI_CAPTURE_SECONDS=60
cargo test -p io --release --locked -j 1 physical_midi_input_captures_operator_ingress -- --ignored --nocapture --test-threads=1
capturing physical MIDI input 'SMC-Mixer' for 60s; move one knob or press one button
physical MIDI input 'SMC-Mixer' produced no operator ingress during 60s
test result: 0 passed; 1 failed; 0 ignored; exit_code=101
```

The production input opened, but no MIDI event arrived. Operator movement during
the capture was not independently confirmed; this does not establish device
fault, controller ingress, or app-selected/mapped ingress. The test sent no MIDI
or DMX output, and no raw-event artifact exists.

## Remaining proof

The marker remains open for an operator-confirmed control movement captured as
raw MIDI, the intended input selected in Syndocal, a non-output mapping such as
Tap BPM observed through the native app route, and the remaining native
client/hardware ingress matrix (feedback, Clock/MTC, reconnect, latency,
OSC/Remote, and DMX/Art-Net). Do not promote endpoint presence or a connected
bridge to ingress acceptance.
